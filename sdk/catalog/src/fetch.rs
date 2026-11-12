use super::*;

pub fn fetch_files(snapshot_root: &Path) -> Result<()> {
    let snapshot = load_snapshot(snapshot_root)?;
    let client = http_client()?;
    let files_root = snapshot_root.join("files");
    fs::create_dir_all(&files_root)?;
    for paper in &snapshot.collected {
        let work = text(paper, "id").ok_or_else(|| msg("collected paper has no id"))?;
        let version = text(paper, "revision").unwrap_or_default();
        let dir = files_root.join(work_dir_name(&work)?);
        fs::create_dir_all(&dir)?;
        let mut report = Map::new();
        report.insert("work".into(), Value::String(work.clone()));
        report.insert("version".into(), Value::String(version));
        for (kind, key) in [("pdf", "pdf"), ("source", "source")] {
            let descriptor = paper
                .get(key)
                .ok_or_else(|| msg(format!("{work} has no {key} descriptor")))?;
            let url = descriptor_url(descriptor, &snapshot.meta.api_base)
                .ok_or_else(|| msg(format!("{work} {key} descriptor has no URL")))?;
            let bytes = request_bytes(&client, &url)?;
            let expected = descriptor_hash(descriptor)
                .or_else(|| {
                    paper
                        .get(format!("{key}_sha256"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .or_else(|| {
                    paper
                        .get("archive")
                        .and_then(|a| a.get(format!("{key}_sha256")))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
            let expected = expected.map(|value| value.to_ascii_lowercase());
            let actual = sha256(&bytes);
            if let Some(expected) = &expected
                && expected != &actual
            {
                return Err(msg(format!(
                    "SHA-256 mismatch for {work} {key}: expected {expected}, got {actual}"
                )));
            }
            report.insert(format!("{key}_sha256"), Value::String(actual.clone()));
            report.insert(
                format!("{key}_public_sha256"),
                expected.clone().map(Value::String).unwrap_or(Value::Null),
            );
            report.insert(
                format!("{key}_bytes"),
                Value::Number((bytes.len() as u64).into()),
            );
            report.insert(
                format!("{key}_hash_verified"),
                Value::Bool(expected.is_some()),
            );
            if kind == "pdf" {
                fs::write(dir.join("paper.pdf"), &bytes)?;
            } else {
                let archive = dir.join("source.archive");
                fs::write(&archive, &bytes)?;
                let manuscript = dir.join("manuscript");
                if manuscript.exists() {
                    fs::remove_dir_all(&manuscript)?;
                }
                fs::create_dir_all(&manuscript)?;
                unpack_archive(&bytes, &manuscript)?;
            }
        }
        write_json(&dir.join("fetch.json"), &Value::Object(report))?;
    }
    Ok(())
}
