use super::*;

pub fn snapshot(api_base: &str, output: &Path) -> Result<SnapshotMeta> {
    let client = http_client()?;
    let public_base = public_root(api_base);
    let api_base = api_root(api_base);
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        output
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("snapshot"),
        std::process::id()
    ));
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    fs::create_dir_all(&temp)?;
    let result = (|| {
        let collected_dir = temp.join("raw/collected-papers");
        let papers_dir = temp.join("raw/papers");
        let formal_dir = temp.join("raw/formalizations");
        fs::create_dir_all(&collected_dir)?;
        fs::create_dir_all(&papers_dir)?;
        fs::create_dir_all(&formal_dir)?;
        let (collected, ccount) =
            collect_pages(&client, &api_base, "/collected-papers", &collected_dir)?;
        let (accepted, pcount) = collect_pages(&client, &api_base, "/papers", &papers_dir)?;
        let accepted = accepted
            .into_iter()
            .map(|row| accepted::collect_archive(&client, &api_base, row))
            .collect::<Result<Vec<_>>>()?;
        let statuses = [
            None,
            Some("unverified"),
            Some("preparing"),
            Some("open"),
            Some("verified"),
            Some("disproved"),
        ];
        let mut listings = BTreeMap::new();
        listings.insert("collected-papers".into(), ccount);
        listings.insert("papers".into(), pcount);
        let mut opportunities = Vec::new();
        for status in statuses {
            let name = status.unwrap_or("confirmed");
            let (rows, count) =
                collect_status_pages(&client, &api_base, status, &formal_dir.join(name))?;
            listings.insert(format!("formalizations/{name}"), count);
            opportunities.extend(rows);
        }
        let mut by_key = BTreeMap::<String, Value>::new();
        for row in opportunities {
            if let Some(k) = directory_row_identity(&row, &collected) {
                by_key.entry(k).or_insert(row);
            }
        }
        // Keep accepted and out-of-scope directory rows in the snapshot. They
        // are projected as nodes, but only collected rows participate in the
        // collected-proofs cross-check.
        let opportunities: Vec<Value> = by_key.values().cloned().collect();
        let mut proof_pairs = Vec::new();
        let proof_dir = temp.join("raw/proofs");
        fs::create_dir_all(&proof_dir)?;
        let mut proof_keys = BTreeSet::new();
        for work in &collected {
            let id = text(work, "id").ok_or_else(|| msg("collected paper has no id"))?;
            let version = text(work, "revision")
                .ok_or_else(|| msg(format!("collected work {id} has no revision")))?;
            let url = format!(
                "{}/collected-papers/{}/proofs?version={}",
                normalize_base(&api_base),
                urlencoding(&id),
                urlencoding(&version)
            );
            let value = get_json(&client, &url)?;
            let rows = value
                .as_array()
                .ok_or_else(|| msg(format!("proofs for {id} are not an array")))?;
            for row in rows {
                if let Some(k) = proof_key(&id, row) {
                    proof_keys.insert(k);
                }
            }
            write_json(
                &proof_dir.join(format!(
                    "{}-v{}.json",
                    safe_component(&id),
                    safe_component(&version)
                )),
                &value,
            )?;
            proof_pairs.push((id, value));
        }
        let directory_keys: BTreeSet<String> = opportunities
            .iter()
            .filter_map(|row| collected_row_key(row, &collected))
            .collect();
        if directory_keys != proof_keys {
            let missing: Vec<_> = proof_keys.difference(&directory_keys).cloned().collect();
            let extra: Vec<_> = directory_keys.difference(&proof_keys).cloned().collect();
            return Err(msg(format!(
                "directory/proofs node mismatch; missing directory={missing:?}, extra directory={extra:?}"
            )));
        }
        super::formalization::collect(&client, &api_base, &temp, &opportunities, &mut listings)?;
        let captured_at = chrono_like_now();
        let meta = SnapshotMeta {
            schema_version: SNAPSHOT_SCHEMA.into(),
            generator_version: GENERATOR_VERSION.into(),
            api_base: public_base,
            captured_at,
            listings,
            collected_works: collected.len(),
            accepted_records: accepted.len(),
            directory_nodes: opportunities.len(),
        };
        let meta_value = serde_json::to_value(&meta)?;
        write_json(&temp.join("snapshot.json"), &meta_value)?;
        write_json(&temp.join("raw/collected.json"), &Value::Array(collected))?;
        write_json(&temp.join("raw/papers.json"), &Value::Array(accepted))?;
        write_json(
            &temp.join("raw/opportunities.json"),
            &Value::Array(opportunities),
        )?;
        Ok(meta)
    })();
    match result {
        Ok(meta) => {
            atomic_replace_dir(&temp, output)?;
            Ok(meta)
        }
        Err(e) => {
            if temp.exists() {
                fs::remove_dir_all(&temp)?;
            }
            Err(e)
        }
    }
}
fn chrono_like_now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
pub(crate) fn atomic_replace_dir(temp: &Path, output: &Path) -> Result<()> {
    let backup = unique_sibling(output, "backup");
    if output.exists() {
        fs::rename(output, &backup)?;
    }
    if let Err(e) = fs::rename(temp, output) {
        if backup.exists() {
            let _ = fs::rename(&backup, output);
        }
        return Err(e.into());
    }
    if backup.exists() {
        fs::remove_dir_all(backup)?;
    }
    Ok(())
}
fn unique_sibling(path: &Path, tag: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let stamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let mut candidate = parent.join(format!(".{name}.{tag}-{}-{stamp}", std::process::id()));
    let mut n = 0u32;
    while candidate.exists() {
        n += 1;
        candidate = parent.join(format!(".{name}.{tag}-{}-{stamp}-{n}", std::process::id()));
    }
    candidate
}
/// A work id names a directory under `files/`. The API base is an input, so the id is
/// accepted only when it is one plain path component.
pub(crate) fn work_dir_name(work: &str) -> Result<&str> {
    let plain = !work.is_empty()
        && work.len() <= 200
        && work != "."
        && work != ".."
        && work
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if plain {
        Ok(work)
    } else {
        Err(msg(format!(
            "work id is not a plain path component: {work:?}"
        )))
    }
}
pub(crate) fn safe_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn load_snapshot(root: &Path) -> Result<Snapshot> {
    let meta: SnapshotMeta = serde_json::from_value(read_json(&root.join("snapshot.json"))?)?;
    let collected = arr(&read_json(&root.join("raw/collected.json"))?, "__never__");
    let collected = if collected.is_empty() {
        read_json(&root.join("raw/collected.json"))?
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        collected
    };
    let accepted = read_json(&root.join("raw/papers.json"))?
        .as_array()
        .cloned()
        .unwrap_or_default();
    let opportunities = read_json(&root.join("raw/opportunities.json"))?
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut proofs = Vec::new();
    let dir = root.join("raw/proofs");
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let p = entry?.path();
            if p.extension().and_then(|x| x.to_str()) == Some("json") {
                let stem = p.file_stem().and_then(|x| x.to_str()).unwrap_or_default();
                let work = collected
                    .iter()
                    .filter_map(|v| text(v, "id"))
                    .find(|id| stem.starts_with(&format!("{}-v", safe_component(id))))
                    .unwrap_or_else(|| stem.split("-v").next().unwrap_or(stem).to_owned());
                proofs.push((work, read_json(&p)?));
            }
        }
    }
    Ok(Snapshot {
        root: root.to_owned(),
        meta,
        collected,
        accepted,
        opportunities,
        proofs,
    })
}

fn collected_row_key(row: &Value, collected: &[Value]) -> Option<String> {
    let record = text(row, "record")?;
    let is_collected = collected
        .iter()
        .any(|work| text(work, "id").as_deref() == Some(record.as_str()));
    if is_collected {
        opportunity_key(row)
    } else {
        None
    }
}

fn directory_row_identity(row: &Value, collected: &[Value]) -> Option<String> {
    let record = text(row, "record")?;
    let version = row.get("version")?.as_i64()?;
    let claim = text(row, "claim")?;
    if collected
        .iter()
        .any(|work| text(work, "id").as_deref() == Some(record.as_str()))
    {
        Some(unique_node_key(&record, version, &claim))
    } else {
        Some(format!("record:{record}:{version}:{claim}"))
    }
}
