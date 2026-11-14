//! Adapt the existing consented public-paper archive; never infer consent from a licence.
use super::*;

pub(crate) fn archive_eligible(row: &Value) -> bool {
    row["archive_consent"] == true
        && row["archive"]["marker"] == "public-archive-2026-11-12"
        && row["archive"]["licence"] == "CC-BY-4.0"
        && text(row, "kind").as_deref() != Some("conjecture")
        && row["version"].as_u64().is_some_and(|v| v > 0)
}

pub(crate) fn collect_archive(client: &Client, base: &str, summary: Value) -> Result<Value> {
    if summary["archive_consent"] != true || text(&summary, "kind").as_deref() == Some("conjecture")
    {
        return Ok(summary);
    }
    let record = text(&summary, "record").ok_or_else(|| msg("accepted paper has no record"))?;
    let detail = get_json(client, &format!("{base}/papers/{}", urlencoding(&record)))?;
    if detail["summary"] != summary {
        return Err(msg(format!(
            "accepted paper changed during snapshot: {record}"
        )));
    }
    let version = detail["publication"]["version"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or_else(|| msg("consented paper has no published version"))?;
    let mut row = summary;
    row["version"] = json!(version);
    row["archive"] = detail["archive"].clone();
    if !archive_eligible(&row) {
        return Err(msg(format!("invalid public archive consent: {record}")));
    }
    let time = text(&row["archive"], "consent_time").unwrap_or_default();
    time::OffsetDateTime::parse(&time, &time::format_description::well_known::Rfc3339)
        .map_err(|_| msg("archive consent has no valid time"))?;
    row["license"] = json!("CC-BY-4.0");
    row["published_on"] = row["accepted_at"].clone();
    for name in ["pdf", "source"] {
        let file = &row["archive"][name];
        let hash = text(file, "sha256").unwrap_or_default();
        if hash.len() != 64
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || file["bytes"].as_u64().is_none_or(|n| n == 0)
            || text(file, "url").is_none()
        {
            return Err(msg(format!("invalid accepted archive {name}: {record}")));
        }
        row[name] = file.clone();
    }
    for name in [
        "plain_language",
        "publication",
        "claims",
        "macros",
        "versions",
    ] {
        if let Some(value) = detail.get(name) {
            row[name] = value.clone();
        }
    }
    Ok(row)
}
