use super::*;

/// Recompute the statistics in a rendered catalog and reject tampering.
pub fn check(output: &Path) -> Result<()> {
    let manifest = read_json(&output.join(".mathesis/rendered.json"))?;
    if manifest["schema"] != "mathesis.rendered.v1" {
        return Err(msg("unknown render manifest"));
    }
    let expected: BTreeMap<String, String> = serde_json::from_value(manifest["sha256"].clone())?;
    let mut actual = BTreeMap::new();
    super::render::collect_files(output, output, &mut actual, false)?;
    actual.remove(".mathesis/rendered.json");
    if actual != expected {
        return Err(msg("rendered files differ from manifest"));
    }
    if fs::read_to_string(output.join("site/CNAME"))? != "papers.trureturing.org\n" {
        return Err(msg("org site CNAME differs"));
    }
    let catalog = output.join("catalog");
    let works_v = read_json(&catalog.join("works.json"))?;
    let nodes_v = read_json(&catalog.join("nodes.json"))?;
    let stats_v = read_json(&catalog.join("stats.json"))?;
    let catalog_snapshot_time = read_json(&catalog.join("snapshot.json"))?
        .get("time")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let works: Vec<Work> = serde_json::from_value(
        works_v
            .get("works")
            .cloned()
            .ok_or_else(|| msg("works.json has no works"))?,
    )?;
    let nodes: Vec<Node> = serde_json::from_value(
        nodes_v
            .get("nodes")
            .cloned()
            .ok_or_else(|| msg("nodes.json has no nodes"))?,
    )?;
    let expected = compute_stats_for_render(&works, &nodes, &catalog_snapshot_time);
    let actual: Stats = serde_json::from_value(stats_v)?;
    if actual != expected {
        return Err(msg(format!(
            "stats.json does not match catalog: expected {}, got {}",
            serde_json::to_string(&expected)?,
            serde_json::to_string(&actual)?
        )));
    }
    super::formalization::check_output(output)?;
    Ok(())
}
fn compute_stats_for_render(works: &[Work], nodes: &[Node], snapshot_time: &str) -> Stats {
    let fake = Snapshot {
        root: PathBuf::new(),
        meta: SnapshotMeta {
            schema_version: String::new(),
            generator_version: String::new(),
            api_base: String::new(),
            captured_at: snapshot_time.into(),
            listings: BTreeMap::new(),
            collected_works: 0,
            accepted_records: 0,
            directory_nodes: 0,
        },
        collected: vec![],
        accepted: vec![],
        opportunities: vec![],
        proofs: vec![],
    };
    compute_stats(&fake, works, nodes)
}
