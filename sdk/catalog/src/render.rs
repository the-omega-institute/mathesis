use super::*;

fn value_from<T: Serialize>(x: &T) -> Result<Value> {
    Ok(serde_json::to_value(x)?)
}
pub(crate) fn authors_markdown(authors: &[Value]) -> String {
    authors
        .iter()
        .map(|a| text(a, "name").unwrap_or_default())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
fn node_json(nodes: &[Node]) -> Result<Value> {
    Ok(json!({"nodes": nodes}))
}
fn works_json(works: &[Work]) -> Result<Value> {
    Ok(json!({"works": works}))
}
fn stats_json(stats: &Stats) -> Result<Value> {
    value_from(stats)
}
fn site_link(site: &str, path: &str) -> String {
    format!("{}/{}", normalize_base(site), path.trim_start_matches('/'))
}
pub(crate) fn paper_page(site: &str, work: &Work) -> String {
    paper_page_parts(site, &work.record, work.current_version)
}
fn paper_page_parts(site: &str, record: &str, version: Option<i64>) -> String {
    let mut url = site_link(site, &format!("papers/{record}"));
    if let Some(version) = version {
        url.push_str(&format!("?version={version}"));
    }
    url
}
fn yaml_scalar(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn publication_date(work: &Work) -> String {
    work.published_on
        .as_deref()
        .and_then(|value| {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()
        })
        .map(|value| value.date().to_string())
        .unwrap_or_else(|| "1970-01-01".to_owned())
}
fn publication_year(work: &Work) -> String {
    publication_date(work)[..4].to_owned()
}

fn paper_readme(work: &Work, nodes: &[Node], site: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# {}\n", work.title);
    let _ = writeln!(s, "**Authors:** {}\n", authors_markdown(&work.authors));
    if let Some(a) = &work.abstract_text {
        let _ = writeln!(s, "{}\n", a);
    }
    let _ = writeln!(s, "- PDF: [frozen public file](paper.pdf)");
    if let Some(doi) = &work.doi {
        let _ = writeln!(s, "- DOI: [https://doi.org/{doi}](https://doi.org/{doi})");
    }
    if let Some(doi) = &work.concept_doi {
        let _ = writeln!(
            s,
            "- Concept DOI: [https://doi.org/{doi}](https://doi.org/{doi})"
        );
    }
    let _ = writeln!(
        s,
        "- License: {}",
        work.license.as_deref().unwrap_or("not stated")
    );
    let _ = writeln!(
        s,
        "- Version: {}",
        work.current_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "not stated".to_owned())
    );
    let _ = writeln!(
        s,
        "- Mathesis record: [{}]({})\n",
        work.record,
        paper_page(site, work)
    );
    s.push_str("## How to cite\n\n");
    let _ = writeln!(
        s,
        "{} ({}). {}. Mathesis, version {}. {}\n",
        authors_markdown(&work.authors),
        publication_year(work),
        work.title,
        work.current_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        work.doi
            .as_ref()
            .map(|doi| format!("https://doi.org/{doi}"))
            .unwrap_or_else(|| paper_page(site, work))
    );
    s.push_str("## Statement progress\n\n| Statement | Formalization status | Source | Verification basis |\n|---|---|---|---|\n");
    for n in nodes {
        let source = n
            .lean_source
            .as_ref()
            .map(|l| format!("[Lean source]({})", l.url))
            .unwrap_or_else(|| "No paper-level source link".into());
        let receipt = n
            .receipt
            .as_ref()
            .and_then(|r| text(r, "checked_at"))
            .map(|x| format!("public receipt ({x})"))
            .unwrap_or_else(|| "No public receipt".into());
        let _ = writeln!(
            s,
            "| {} ({}) | {} | {} | {} |",
            n.label, n.claim, n.status, source, receipt
        );
    }
    s.push_str("\n## Contributions\n\nOriginal authors are listed above. Formalization contributors and attribution are retained in `formalization/nodes.json`; repository pull requests do not confer verification or credit. Submit formalization attempts through the Mathesis attempt path described in [CONTRIBUTING.md](CONTRIBUTING.md).\n");
    s
}
fn contributing(site: &str) -> String {
    format!(
        "# Contributing\n\nMathesis records formalization attempts through the public attempt path. Read [Contribute]({}/contribute) and the [public contribution guide](https://github.com/the-omega-institute/mathesis/blob/develop/docs/CONTRIBUTE.md), then use `sdk/contribute`. A repository pull request may explain a proof, point to an erratum, or add reproducibility material; it does not register an attempt, confer verification status, or create credit. Only an independent verifier receipt for the exact target counts. Original paper authors and formalization contributors remain separate.\n\nThe shared Lean library is the first binding. This repository does not copy library sources and does not emit a Lean environment for a paper that has no new Lean. If a paper-specific development is later needed, its pinned environment is generated from the verifier's pinned environment and is never hand-edited.\n",
        normalize_base(site)
    )
}
pub(crate) fn citation(work: &Work, site: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "cff-version: 1.2.0");
    let _ = writeln!(
        s,
        "message: {}",
        yaml_scalar("If you use this work, cite it.")
    );
    let _ = writeln!(s, "title: {}", yaml_scalar(&work.title));
    s.push_str("authors:\n");
    for a in &work.authors {
        let _ = writeln!(
            s,
            "  - name: {}",
            yaml_scalar(&text(a, "name").unwrap_or_default())
        );
    }
    let _ = writeln!(s, "date-released: {}", yaml_scalar(&publication_date(work)));
    let _ = writeln!(
        s,
        "version: {}",
        yaml_scalar(
            &work
                .current_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        )
    );
    let _ = writeln!(s, "url: {}", yaml_scalar(&paper_page(site, work)));
    if let Some(doi) = &work.doi {
        let _ = writeln!(s, "doi: {}", yaml_scalar(doi));
    }
    let _ = writeln!(
        s,
        "license: {}",
        yaml_scalar(work.license.as_deref().unwrap_or("CC-BY-4.0"))
    );
    s
}
fn changelog(work: &Work) -> String {
    format!(
        "# Changelog\n\n- Version {}: public Mathesis snapshot at {}.\n- Formalization events are represented by exact-version nodes in `formalization/nodes.json`.\n",
        work.current_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into()),
        work.published_on.as_deref().unwrap_or("unknown")
    )
}
fn paper_json(work: &Work, snapshot_root: &Path) -> Result<Value> {
    let report = snapshot_root
        .join("files")
        .join(&work.work)
        .join("fetch.json");
    let report_value = if report.exists() {
        Some(read_json(&report)?)
    } else {
        None
    };
    let file_info = |name: &str, path: &str, file: &Path| -> Value {
        let public_key = format!("{name}_public_sha256");
        let public_sha = report_value
            .as_ref()
            .and_then(|r| r.get(&public_key))
            .cloned()
            .unwrap_or(Value::Null);
        let actual = if file.exists() {
            sha256_file(file).ok()
        } else {
            None
        };
        let bytes = file.metadata().ok().map(|m| m.len());
        let matches = public_sha
            .as_str()
            .map(|expected| actual.as_deref() == Some(expected));
        json!({"path": path, "bytes": bytes, "sha256": actual, "public_sha256": public_sha, "matches_public": matches})
    };
    let files = {
        let root = snapshot_root.join("files").join(work_dir_name(&work.work)?);
        json!({
            "pdf": file_info("pdf", "paper.pdf", &root.join("paper.pdf")),
            "source": file_info("source", "manuscript/", &root.join("source.archive"))
        })
    };
    Ok(json!({"work": work, "files": files, "source_identity": "Mathesis public projection"}))
}

fn write_text(path: &Path, value: &str) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, value.as_bytes())?;
    Ok(())
}
fn copy_license(path: &Path) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, include_bytes!("../template/LICENSES/CC-BY-4.0.txt"))?;
    Ok(())
}
fn render_template(root: &Path, site: &str) -> Result<()> {
    write_text(
        &root.join("README.md"),
        "# Mathesis paper repository template\n\nThis tree is generated by `mathesis-catalog`. Replace placeholders only through the publisher. The shared Lean library is the first binding; paper repositories contain mappings and public receipts.\n",
    )?;
    write_text(&root.join("CONTRIBUTING.md"), &contributing(site))?;
    write_text(
        &root.join("CITATION.cff"),
        "cff-version: 1.2.0\nmessage: \"Cite the paper represented by this repository.\"\ntitle: \"Paper title\"\nauthors:\n  - name: \"Paper author\"\nlicense: CC-BY-4.0\n",
    )?;
    write_json(
        &root.join("formalization/nodes.json"),
        &json!({"nodes": []}),
    )?;
    copy_license(&root.join("LICENSES/CC-BY-4.0.txt"))?;
    Ok(())
}
fn manifest_for(root: &Path, snapshot: &Snapshot) -> Result<Value> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files, true)?;
    Ok(
        json!({"generated_paths": files.keys().collect::<Vec<_>>(), "sha256": files, "generator_version": GENERATOR_VERSION, "snapshot_identity": sha256(&fs::read(snapshot.root.join("snapshot.json"))?)}),
    )
}
pub(crate) fn collect_files(
    root: &Path,
    current: &Path,
    out: &mut BTreeMap<String, String>,
    skip_manifest: bool,
) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let p = entry?.path();
        let rel = p
            .strip_prefix(root)
            .map_err(|_| msg("manifest path error"))?;
        if p.is_dir() {
            collect_files(root, &p, out, skip_manifest)?;
        } else if !(skip_manifest && rel == Path::new(".mathesis/generated.json")) {
            out.insert(rel.to_string_lossy().replace('\\', "/"), sha256_file(&p)?);
        }
    }
    Ok(())
}

/// Render a snapshot without making HTTP requests.
pub fn render(
    snapshot_root: &Path,
    output: &Path,
    organization: &str,
    display_name: &str,
    site_base: &str,
    mapping_path: Option<&Path>,
) -> Result<()> {
    render_with_materials(
        snapshot_root,
        output,
        organization,
        display_name,
        site_base,
        mapping_path,
        None,
    )
}

/// Render using optional locally downloaded repository materials manifests.
pub fn render_with_materials(
    snapshot_root: &Path,
    output: &Path,
    organization: &str,
    display_name: &str,
    site_base: &str,
    mapping_path: Option<&Path>,
    materials_root: Option<&Path>,
) -> Result<()> {
    let snapshot = load_snapshot(snapshot_root)?;
    let mapping = mapping_file(mapping_path)?;
    let mut works = Vec::new();
    for paper in &snapshot.collected {
        works.push(paper_work(paper, false, &mapping)?);
    }
    for paper in &snapshot.accepted {
        works.push(paper_work(paper, true, &mapping)?);
    }
    works.sort_by(|a, b| a.work.cmp(&b.work));
    let nodes = make_nodes(&snapshot)?;
    let stats = compute_stats(&snapshot, &works, &nodes);
    let temp = output.with_extension(format!("tmp-{}", std::process::id()));
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    fs::create_dir_all(&temp)?;
    let result = (|| {
        let catalog = &temp.join("catalog");
        fs::create_dir_all(catalog)?;
        write_json(&catalog.join("works.json"), &works_json(&works)?)?;
        write_json(&catalog.join("nodes.json"), &node_json(&nodes)?)?;
        write_json(&catalog.join("stats.json"), &stats_json(&stats)?)?;
        write_json(&catalog.join("repositories.json"), &json!(mapping))?;
        write_json(
            &catalog.join("archive-index.json"),
            &publishing::archive_index(&snapshot)?,
        )?;
        for work in &works {
            let material = publishing::materials(work, organization, &mapping, materials_root)?;
            write_json(
                &catalog
                    .join("materials")
                    .join(work_dir_name(&work.work)?)
                    .join("materials.json"),
                &material,
            )?;
        }
        publishing::render_site(
            &temp.join("site"),
            &works,
            &stats,
            organization,
            display_name,
            site_base,
            &mapping,
        )?;
        let snapshot_projection = json!({"time": snapshot.meta.captured_at, "api_base": snapshot.meta.api_base, "generator_version": GENERATOR_VERSION, "schema_versions": {"snapshot": SNAPSHOT_SCHEMA, "catalog": CATALOG_SCHEMA}, "listings": snapshot.meta.listings, "rows": {"works": works.len(), "nodes": nodes.len()}});
        write_json(&catalog.join("snapshot.json"), &snapshot_projection)?;
        write_text(
            &catalog.join("README.md"),
            &format!(
                "# Mathesis catalog\n\nThis catalog is generated from public Mathesis records at snapshot time `{}`. It contains {} public papers and {} registered statements. The organization is `{organization}`.\n\nThe source authority is Mathesis; GitHub repositories are projections. See the profile for scope and counting rules.\n",
                snapshot.meta.captured_at, stats.public_papers, stats.registered_statements
            ),
        )?;
        let profile = &temp.join("profile");
        fs::create_dir_all(profile)?;
        write_text(
            &profile.join("README.md"),
            &profile_readme(
                &works,
                &nodes,
                &stats,
                organization,
                display_name,
                site_base,
            )?,
        )?;
        for work in works.iter().filter(|w| w.publication == "public") {
            let repo = work
                .repository_name
                .as_deref()
                .ok_or_else(|| msg("public work has no repository name"))?;
            let dir = temp.join("papers").join(repo);
            fs::create_dir_all(&dir)?;
            let wn: Vec<Node> = nodes
                .iter()
                .filter(|n| n.work == work.work)
                .cloned()
                .collect();
            write_text(&dir.join("README.md"), &paper_readme(work, &wn, site_base))?;
            write_json(&dir.join("paper.json"), &paper_json(work, snapshot_root)?)?;
            write_text(&dir.join("CITATION.cff"), &citation(work, site_base))?;
            write_text(&dir.join("CONTRIBUTING.md"), &contributing(site_base))?;
            write_json(&dir.join("formalization/nodes.json"), &node_json(&wn)?)?;
            write_text(&dir.join("CHANGELOG.md"), &changelog(work))?;
            copy_license(&dir.join("LICENSES/CC-BY-4.0.txt"))?;
            for n in &wn {
                if let Some(r) = &n.receipt {
                    write_json(
                        &dir.join(format!("verification/{}.json", safe_component(&n.identity))),
                        r,
                    )?;
                }
            }
            let frozen = snapshot_root.join("files").join(work_dir_name(&work.work)?);
            if frozen.join("paper.pdf").exists() {
                fs::copy(frozen.join("paper.pdf"), dir.join("paper.pdf"))?;
            }
            if frozen.join("manuscript").exists() {
                copy_tree(&frozen.join("manuscript"), &dir.join("manuscript"))?;
            }
            let manifest = manifest_for(&dir, &snapshot)?;
            write_json(&dir.join(".mathesis/generated.json"), &manifest)?;
        }
        render_template(&temp.join("template"), site_base)?;
        let mut hashes = BTreeMap::new();
        collect_files(&temp, &temp, &mut hashes, false)?;
        write_json(
            &temp.join(".mathesis/rendered.json"),
            &json!({"schema":"mathesis.rendered.v1", "sha256":hashes}),
        )?;
        Ok::<(), Error>(())
    })();
    match result {
        Ok(()) => {
            atomic_replace_dir(&temp, output)?;
            Ok(())
        }
        Err(e) => {
            if temp.exists() {
                fs::remove_dir_all(temp)?;
            }
            Err(e)
        }
    }
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let q = dst.join(e.file_name());
        if p.is_dir() {
            fs::create_dir_all(&q)?;
            copy_tree(&p, &q)?;
        } else {
            fs::create_dir_all(dst)?;
            fs::copy(p, q)?;
        }
    }
    Ok(())
}
fn receipt_time(node: &Node) -> String {
    node.receipt
        .as_ref()
        .and_then(|receipt| text(receipt, "checked_at"))
        .unwrap_or_default()
}

fn profile_readme(
    works: &[Work],
    nodes: &[Node],
    stats: &Stats,
    org: &str,
    display_name: &str,
    site: &str,
) -> Result<String> {
    let mut s = String::new();
    writeln!(s, "# {display_name}\n")?;
    s.push_str("Public papers and formalization results of Mathesis, operated by Chrono AI.\n\n");
    writeln!(
        s,
        "Snapshot time: `{}` UTC. Counts cover current public versions of in-scope collected works. Each scope sentence describes the counted set.\n",
        stats.snapshot_time
    )?;
    writeln!(
        s,
        "| Metric | Count | Scope |\n|---|---:|---|\n| Public papers | {} | in-scope collected works with a current public CC-BY-4.0 version |\n| Registered statements | {} | in-scope nodes of selected versions, deduplicated by identity |\n| Independently verified statements | {} | in-scope nodes with an exact target and matching public receipt |\n| Verified from existing library declarations | {} | in-scope verified nodes whose receipt basis is library declarations |\n| Verified with new Lean | {} | in-scope verified nodes whose receipt basis is new Lean |\n| Verified counterexamples / disproofs | {} | in-scope public disproof receipts, counted separately |\n| Open formalization targets | {} | in-scope confirmed targets without a verified proof |\n| Statements awaiting a formal target | {} | in-scope nodes without a confirmed target; needs-attention subset: {} |\n| Papers with Lean source | {} | in-scope papers with an exact-commit paper-level source link |\n| Papers with a DOI | {} | in-scope papers with a checked exact-version DOI in the public projection |",
        stats.public_papers,
        stats.registered_statements,
        stats.independently_verified_statements,
        stats.verified_from_existing_library_declarations,
        stats.verified_with_new_lean,
        stats.verified_counterexamples_disproofs,
        stats.open_formalization_targets,
        stats.statements_awaiting_formal_target,
        stats.awaiting_target_needs_attention,
        stats.papers_with_lean_source,
        stats.papers_with_doi
    )?;
    let has_msc = stats.by_msc.keys().any(|key| key != "unclassified");
    if has_msc {
        s.push_str("\n\n## By top-level MSC class\n\n| MSC class | Papers | Registered | Verified | Open targets |\n|---|---:|---:|---:|---:|\n");
        let labels = msc_labels();
        for (k, v) in &stats.by_msc {
            if k == "unclassified" {
                continue;
            }
            writeln!(
                s,
                "| {} ({}) | {} | {} | {} | {} |",
                k,
                labels.get(k).cloned().unwrap_or_else(|| k.clone()),
                v.papers,
                v.registered,
                v.verified,
                v.open_targets
            )?;
        }
    } else {
        writeln!(
            s,
            "\n\nNo in-scope work carries an MSC code; {} in-scope papers are unclassified.",
            stats.public_papers
        )?;
    }
    s.push_str("\n## Recent verified statements\n\n");
    let mut verified: Vec<_> = nodes
        .iter()
        .filter(|n| n.scope == "in_scope" && is_verified(n))
        .collect();
    verified.sort_by(|a, b| {
        receipt_time(b)
            .cmp(&receipt_time(a))
            .then_with(|| a.identity.cmp(&b.identity))
    });
    if verified.is_empty() {
        s.push_str("None yet.\n");
    } else {
        for n in verified {
            let src = n
                .lean_source
                .as_ref()
                .map(|x| format!("[exact Lean source]({})", x.url))
                .unwrap_or_else(|| "source link unavailable".into());
            let page = paper_page_parts(site, &n.record, Some(n.version));
            writeln!(s, "- **{}** — [{}]({}) · {}", n.label, n.claim, page, src)?;
        }
    }
    s.push_str("\n## Recent papers\n\n");
    let mut recent: Vec<&Work> = works.iter().filter(|w| w.publication == "public").collect();
    recent.sort_by(|a, b| {
        b.published_on
            .cmp(&a.published_on)
            .then_with(|| a.work.cmp(&b.work))
    });
    for w in recent.into_iter().take(10) {
        writeln!(
            s,
            "- [{}]({}) — {}",
            w.title,
            paper_page(site, w),
            publication_date(w)
        )?;
    }
    writeln!(
        s,
        "\n[Full catalog](https://github.com/{org}/catalog) · [Mathesis site]({}) · [Formalization directory]({}/formalizations)",
        normalize_base(site),
        normalize_base(site)
    )?;
    Ok(s)
}
