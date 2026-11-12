use super::*;

fn archive_path(raw: &str) -> Result<PathBuf> {
    let normalized = raw.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute() {
        return Err(msg("archive contains an absolute path"));
    }
    let mut clean = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Normal(x) => clean.push(x),
            Component::CurDir => {}
            Component::ParentDir => return Err(msg(format!("archive path traversal: {raw}"))),
            _ => return Err(msg(format!("invalid archive path: {raw}"))),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(msg("archive contains an empty path"));
    }
    Ok(clean)
}
/// Unpack ZIP or tar(.gz) bytes with path, link, count and total-size limits.
pub fn unpack_archive(bytes: &[u8], destination: &Path) -> Result<()> {
    unpack_archive_with_limits(
        bytes,
        destination,
        MAX_ARCHIVE_ENTRIES,
        MAX_ARCHIVE_UNPACKED_BYTES,
    )
}
/// Copy at most `budget` bytes; more input is an error. Returns the bytes written.
/// The size an archive header declares is untrusted, so the bytes are counted here.
fn copy_bounded(reader: &mut impl Read, writer: &mut impl io::Write, budget: u64) -> Result<u64> {
    let written = io::copy(&mut reader.take(budget.saturating_add(1)), writer)?;
    if written > budget {
        return Err(msg("archive unpacked-size limit exceeded"));
    }
    Ok(written)
}
pub(crate) fn unpack_archive_with_limits(
    bytes: &[u8],
    destination: &Path,
    max_entries: usize,
    max_unpacked: u64,
) -> Result<()> {
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(msg("archive exceeds download limit"));
    }
    fs::create_dir_all(destination)?;
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        let mut z = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| msg(format!("invalid zip archive: {e}")))?;
        if z.len() > max_entries {
            return Err(msg("archive entry limit exceeded"));
        }
        let mut total = 0u64;
        for i in 0..z.len() {
            let mut file = z.by_index(i).map_err(|e| msg(e.to_string()))?;
            let name = file.name().to_owned();
            let rel = archive_path(&name)?;
            if file.is_symlink() {
                return Err(msg(format!("archive symlink rejected: {name}")));
            }
            let out = destination.join(rel);
            if name.ends_with('/') {
                fs::create_dir_all(&out)?;
            } else {
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut f = fs::File::create(out)?;
                total += copy_bounded(&mut file, &mut f, max_unpacked - total)?;
            }
        }
        return Ok(());
    }
    let reader: Box<dyn Read> = if bytes.starts_with(&[0x1f, 0x8b]) {
        Box::new(GzDecoder::new(Cursor::new(bytes)))
    } else {
        Box::new(Cursor::new(bytes))
    };
    let mut archive = Archive::new(reader);
    let mut count = 0usize;
    let mut total = 0u64;
    for item in archive
        .entries()
        .map_err(|e| msg(format!("invalid tar archive: {e}")))?
    {
        let mut entry = item.map_err(|e| msg(e.to_string()))?;
        count += 1;
        if count > max_entries {
            return Err(msg("archive entry limit exceeded"));
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink()
            || entry_type.is_hard_link()
            || (!entry_type.is_dir() && !entry_type.is_file())
        {
            return Err(msg("archive link or special file rejected"));
        }
        let raw = entry
            .path()
            .map_err(|e| msg(e.to_string()))?
            .to_string_lossy()
            .into_owned();
        let rel = archive_path(&raw)?;
        let out = destination.join(rel);
        if entry_type.is_dir() {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            let mut f = fs::File::create(out)?;
            total += copy_bounded(&mut entry, &mut f, max_unpacked - total)?;
        }
    }
    Ok(())
}
