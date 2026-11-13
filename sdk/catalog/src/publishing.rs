use super::*;

pub(crate) const PAPERS_SITE: &str = "https://papers.trureturing.org";

fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn link(url: &str, label: &str) -> String {
    format!("<a href=\"{}\">{}</a>", html(url), html(label))
}
fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>body{{font:18px/1.6 system-ui,sans-serif;max-width:70rem;margin:auto;padding:2rem;color:#182b3a;background:#fafbf9}}a{{color:#185a82}}table{{border-collapse:collapse;width:100%}}td,th{{text-align:left;border-bottom:1px solid #cdd7dc;padding:.6rem}}li{{margin-bottom:1rem}}small{{color:#465e6e}}@media(max-width:600px){{body{{padding:1rem;font-size:16px}}}}</style><main>{body}</main></html>\n",
        html(title)
    )
}

/// A DOI path is data, never an output-directory traversal. Preserve its slash structure.
pub(crate) fn doi_path(doi: &str) -> Result<String> {
    let (prefix, suffix) = doi
        .split_once('/')
        .ok_or_else(|| msg("DOI has no suffix"))?;
    if !prefix.starts_with("10.")
        || prefix.len() <= 3
        || !prefix[3..].bytes().all(|b| b.is_ascii_digit())
        || suffix.is_empty()
        || doi.len() > 512
        || doi
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\\?#%\"<>".contains(c))
        || doi
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(msg(format!("unsafe DOI path: {doi}")));
    }
    Ok(doi.to_owned())
}

pub(crate) fn archive_index(snapshot: &Snapshot) -> Result<Value> {
    time::OffsetDateTime::parse(
        &snapshot.meta.captured_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| msg("snapshot time must be RFC 3339 for archive-index.json"))?;
    let mut entries = Vec::new();
    for (kind, rows) in [
        ("collected", &snapshot.collected),
        ("record", &snapshot.accepted),
    ] {
        for row in rows {
            // Only exact-version DOIs supplied by the public authority, never concept DOIs.
            let doi = text(row, "doi").or_else(|| row.get("archive").and_then(|a| text(a, "doi")));
            if let Some(doi) = doi {
                doi_path(&doi)?;
                let id = if kind == "collected" {
                    text(row, "id")
                } else {
                    text(row, "record")
                }
                .ok_or_else(|| msg("archive work has no id"))?;
                let version = text(row, "revision")
                    .and_then(|s| s.parse::<i64>().ok())
                    .or_else(|| row.get("version").and_then(Value::as_i64))
                    .filter(|v| *v > 0)
                    .ok_or_else(|| msg("archive DOI has no positive version"))?;
                entries.push(json!({"kind":kind,"id":id,"version":version,"doi":doi}));
            }
        }
    }
    entries.sort_by_key(|v| (text(v, "kind"), text(v, "id"), v["version"].as_i64()));
    Ok(
        json!({"schema":"mathesis.archive-index.v1","generated_at":snapshot.meta.captured_at,"entries":entries}),
    )
}

fn material_url(value: &Value, base: &str) -> Result<()> {
    let url = value
        .as_str()
        .ok_or_else(|| msg("material URL must be a string"))?;
    let parsed = reqwest::Url::parse(url).map_err(|_| msg("invalid material URL"))?;
    if !url.starts_with(base)
        || !parsed.as_str().starts_with(base)
        || parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed
            .path_segments()
            .is_some_and(|mut s| s.any(|p| p == ".." || p == "."))
        || url.contains("%")
        || url.contains("\\")
    {
        return Err(msg(format!("material URL leaves paper site: {url}")));
    }
    Ok(())
}

pub(crate) fn materials(
    work: &Work,
    org: &str,
    mapping: &BTreeMap<String, String>,
    input: Option<&Path>,
) -> Result<Value> {
    let repo = mapping.get(&work.work);
    let repo_url = repo.map(|r| format!("https://github.com/{org}/{r}"));
    let site = repo.map(|r| format!("{PAPERS_SITE}/{r}/"));
    let mut out = json!({"schema":"mathesis.catalog-materials.v1", "work":work.work,"version":work.current_version,
        "repository_url":repo_url,"site_url":site,"manifest_url":site.as_ref().map(|s| format!("{s}mathesis-materials.json")),
        "doi":work.doi,"reading":{},"slides":[],"visualizations":[]});
    if let (Some(root), Some(repo), Some(base)) = (input, repo, site.as_deref()) {
        let path = root.join(format!("{repo}.json"));
        if !path.exists() {
            return Ok(out);
        }
        let m = read_json(&path)?;
        if m["schema"] != "mathesis.materials.v1"
            || m["work"] != work.work
            || m["version"].as_i64() != work.current_version
            || m["repository_url"] != out["repository_url"]
            || m["site_url"] != out["site_url"]
        {
            return Err(msg(format!(
                "materials identity mismatch: {}",
                path.display()
            )));
        }
        let reading = m["reading"]
            .as_object()
            .ok_or_else(|| msg("materials reading must be an object"))?;
        for (lang, url) in reading {
            if lang.is_empty() || !lang.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
                return Err(msg("invalid materials language"));
            }
            material_url(url, base)?;
        }
        for (key, url_keys) in [
            ("slides", vec!["pdf_url", "notes_url"]),
            ("visualizations", vec!["url"]),
        ] {
            let items = m[key]
                .as_array()
                .ok_or_else(|| msg(format!("materials {key} must be an array")))?;
            let mut ids = BTreeSet::new();
            for item in items {
                let id = text(item, "id")
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| msg("material has no id"))?;
                let identity = (id, text(item, "language"));
                if !ids.insert(identity) {
                    return Err(msg("duplicate material identity"));
                }
                for field in &url_keys {
                    let url = item
                        .get(*field)
                        .ok_or_else(|| msg(format!("material missing {field}")))?;
                    if !url.is_null() {
                        material_url(url, base)?;
                    }
                }
            }
        }
        for key in ["reading", "slides", "visualizations"] {
            out[key] = m[key].clone();
        }
        out["manifest_available"] = json!(true);
    }
    Ok(out)
}

/// Only durable repository mappings produce public links. Reuse the validated
/// catalog material rows, retaining repository-only entries before media exists.
pub(crate) fn materials_index(snapshot: &Snapshot, materials: &[Value]) -> Result<Value> {
    time::OffsetDateTime::parse(
        &snapshot.meta.captured_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| msg("snapshot time must be RFC 3339 for materials-index.json"))?;
    let collected: BTreeSet<_> = snapshot
        .collected
        .iter()
        .filter_map(|v| text(v, "id"))
        .collect();
    let mut entries = Vec::new();
    for material in materials {
        if material["repository_url"].is_null() {
            continue;
        }
        let id = text(material, "work").ok_or_else(|| msg("materials work has no id"))?;
        let version = material["version"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= u32::MAX as u64)
            .ok_or_else(|| msg("materials work has no positive version"))?;
        let kind = if collected.contains(&id) {
            "collected"
        } else {
            "record"
        };
        entries.push(json!({"kind":kind,"id":id,"version":version,
            "repository_url":material["repository_url"],"site_url":material["site_url"],
            "reading":material["reading"],"slides":material["slides"],"visualizations":material["visualizations"]}));
    }
    entries.sort_by_key(|v| (text(v, "kind"), text(v, "id"), v["version"].as_u64()));
    Ok(
        json!({"schema":"mathesis.materials-index.v1", "generated_at":snapshot.meta.captured_at,"entries":entries}),
    )
}

pub(crate) fn render_site(
    root: &Path,
    works: &[Work],
    stats: &Stats,
    org: &str,
    name: &str,
    mathesis: &str,
    mapping: &BTreeMap<String, String>,
) -> Result<()> {
    fs::create_dir_all(root)?;
    fs::write(root.join("CNAME"), "papers.trureturing.org\n")?;
    fs::write(root.join(".nojekyll"), "")?;
    let mut body = format!(
        "<h1>{}</h1><p>Public papers from Mathesis.</p><p>Snapshot: <time>{}</time>. Counts cover current public CC-BY-4.0 collected works and their selected-version statement nodes.</p>",
        html(name),
        html(&stats.snapshot_time)
    );
    body.push_str(
        "<table><thead><tr><th>Metric</th><th>Count</th><th>Scope</th></tr></thead><tbody>",
    );
    for (label, count, scope) in [
        (
            "Public papers",
            stats.public_papers,
            "Current in-scope collected versions",
        ),
        (
            "Registered statements",
            stats.registered_statements,
            "In-scope selected-version nodes, deduplicated by identity",
        ),
        (
            "Independently verified statements",
            stats.independently_verified_statements,
            "Exact target with matching public proof receipt",
        ),
        (
            "Verified from existing library declarations",
            stats.verified_from_existing_library_declarations,
            "Verified subset with library-declarations basis",
        ),
        (
            "Verified with new Lean",
            stats.verified_with_new_lean,
            "Verified subset with new-Lean basis",
        ),
        (
            "Verified counterexamples / disproofs",
            stats.verified_counterexamples_disproofs,
            "Public disproof receipts, counted separately",
        ),
        (
            "Open formalization targets",
            stats.open_formalization_targets,
            "Confirmed targets without a verified proof",
        ),
        (
            "Statements awaiting a formal target",
            stats.statements_awaiting_formal_target,
            "Nodes without a confirmed target",
        ),
        (
            "Awaiting target: needs attention",
            stats.awaiting_target_needs_attention,
            "Needs-attention subset",
        ),
        (
            "Papers with Lean source",
            stats.papers_with_lean_source,
            "Paper-level source pinned to an exact commit; does not establish verification",
        ),
        (
            "Papers with a DOI",
            stats.papers_with_doi,
            "Checked exact-version DOI in the public projection",
        ),
    ] {
        write!(
            body,
            "<tr><th>{label}</th><td>{count}</td><td>{scope}</td></tr>"
        )?;
    }
    body.push_str("</tbody></table><h2>Papers</h2><ul>");
    let mut redirects = BTreeMap::new();
    for w in works.iter().filter(|w| w.publication == "public") {
        let mathesis_url = super::render::paper_page(mathesis, w);
        write!(
            body,
            "<li><strong>{}</strong><br><small>{}</small><br>{}",
            html(&w.title),
            html(&super::render::authors_markdown(&w.authors)),
            link(&mathesis_url, "Mathesis paper")
        )?;
        let target = if let Some(repo) = mapping.get(&w.work) {
            let url = format!("{PAPERS_SITE}/{repo}/");
            write!(
                body,
                " · {} · {}",
                link(&url, "Reading and materials"),
                link(&format!("https://github.com/{org}/{repo}"), "Repository")
            )?;
            url
        } else {
            mathesis_url
        };
        if let Some(doi) = &w.doi {
            let path = doi_path(doi)?;
            if redirects
                .insert(path.to_lowercase(), target.clone())
                .is_some()
            {
                return Err(msg(format!("duplicate DOI redirect: {doi}")));
            }
            let dest = root.join("doi").join(&path).join("index.html");
            fs::create_dir_all(dest.parent().unwrap())?;
            fs::write(
                dest,
                format!(
                    "<!doctype html>\n<html lang=\"en\"><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0;url={}\"><link rel=\"canonical\" href=\"{}\"><title>Paper DOI</title><p>{}</p></html>\n",
                    html(&target),
                    html(&target),
                    link(&target, "Open paper")
                ),
            )?;
            write!(body, " · {}", link(&format!("https://doi.org/{doi}"), doi))?;
        }
        body.push_str("</li>");
    }
    write!(
        body,
        "</ul><p>{} · {} · {}</p>",
        link(
            &format!("https://github.com/{org}/formalization"),
            "Formalization repository"
        ),
        link(&format!("https://github.com/{org}/catalog"), "Full catalog"),
        link(mathesis, "Mathesis")
    )?;
    fs::write(root.join("index.html"), page(name, &body))?;
    Ok(())
}
