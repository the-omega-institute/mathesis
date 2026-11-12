use super::*;
use std::io::Write;
fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "mathesis-catalog-test-{name}-{}",
        std::process::id()
    ));
    if p.exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn repository_fallback_is_deterministic() {
    assert!(valid_repo_name("a-b_1.2"));
    assert!(!valid_repo_name("bad/name"));
    assert_eq!(
        repository_name("bad/name", &BTreeMap::new()),
        repository_name("bad/name", &BTreeMap::new())
    );
}
#[test]
fn archive_rejects_traversal() {
    let p = temp("archive");
    let mut buf = Vec::new();
    {
        let mut z = zip::ZipWriter::new(Cursor::new(&mut buf));
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("../escape", o).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
    }
    assert!(unpack_archive(&buf, &p).is_err());
}
#[test]
fn license_is_official_and_stable() {
    let b = include_bytes!("../template/LICENSES/CC-BY-4.0.txt");
    assert_eq!(
        sha256(b),
        "58230517b7895aa219ff46a6ed63e67057a1582fb9ea054b16728a8c13525650"
    );
}

fn mock_server<F>(
    handler: F,
) -> (
    String,
    std::sync::mpsc::Sender<()>,
    std::thread::JoinHandle<()>,
)
where
    F: Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static,
{
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, mpsc};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (stop_tx, stop_rx) = mpsc::channel();
    let handler = Arc::new(handler);
    let join = std::thread::spawn(move || {
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0u8; 4096];
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let n = stream.read(&mut request).unwrap_or(0);
                    let request_text = String::from_utf8_lossy(&request[..n]).into_owned();
                    let first = request_text.lines().next().unwrap_or("");
                    let path = first.split_whitespace().nth(1).unwrap_or("");
                    let (status, body) = handler(path);
                    let reason = if status == 200 { "OK" } else { "Bad Request" };
                    let head = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(&body);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(_) => break,
            }
        }
    });
    (format!("http://{address}"), stop_tx, join)
}
fn mock_catalog_handler(mismatch: bool) -> impl Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static {
    move |path| {
        let body = |v: Value| serde_json::to_vec(&v).unwrap();
        if path.starts_with("/api/v1/collected-papers?") {
            if path.contains("before=next") {
                return (
                    200,
                    body(
                        json!({"items":[{"id":"w1","revision":"1","title":"Work","abstract":"A","authors":[],"license":"CC-BY-4.0","lean":{"url":"https://github.com/x/y/blob/a/A.lean","revision":"a"},"pdf":{"url":"/files/p.pdf"},"source":{"url":"/files/s.zip"}}],"next_before":null}),
                    ),
                );
            }
            return (200, body(json!({"items":[],"next_before":"next"})));
        }
        if path.starts_with("/api/v1/papers?") {
            return (200, body(json!({"items":[],"next_before":null})));
        }
        if path.starts_with("/api/v1/formalizations/opportunities") {
            if path.contains("status=unverified") && !path.contains("before=next") {
                let next = if mismatch {
                    Value::Null
                } else {
                    Value::String("next".into())
                };
                return (
                    200,
                    body(
                        json!({"items":[{"record":"w1","claim":"c1","version":1,"title":"Work","label":"Theorem","statement":"same","authors":[],"kind":"paper","status":"preparing","target_digest":null,"toolchain":null,"contributor":null,"attribution":null,"receipt":null,"proof_basis":null}],"next_before":next}),
                    ),
                );
            }
            if path.contains("status=unverified") {
                return (
                    200,
                    body(
                        json!({"items":[{"record":"w1","claim":"c2","version":1,"title":"Work","label":"Lemma","statement":"same","authors":[],"kind":"paper","status":"preparing","target_digest":null,"toolchain":null,"contributor":null,"attribution":null,"receipt":null,"proof_basis":null}],"next_before":null}),
                    ),
                );
            }
            return (200, body(json!({"items":[],"next_before":null})));
        }
        if path.starts_with("/api/v1/collected-papers/w1/proofs") {
            let rows = if mismatch {
                json!([])
            } else {
                json!([{"claim":"c1","version":1,"state":"preparing","target_digest":null,"next_step":"x","proofs":[]},{"claim":"c2","version":1,"state":"preparing","target_digest":null,"next_step":"x","proofs":[]}])
            };
            return (200, body(rows));
        }
        (404, b"{}".to_vec())
    }
}
#[test]
fn snapshot_traverses_all_pages_and_cross_checks_proofs() {
    let (base, stop, join) = mock_server(mock_catalog_handler(false));
    let root = temp("pagination").join("snapshot");
    let meta = snapshot(&base, &root).unwrap();
    assert_eq!(meta.collected_works, 1);
    assert_eq!(meta.directory_nodes, 2);
    assert_eq!(meta.listings["formalizations/unverified"].pages, 2);
    let _ = stop.send(());
    let _ = join.join();
}
#[test]
fn archive_entry_limit_is_enforced() {
    let p = temp("archive-count");
    let mut buf = Vec::new();
    {
        let mut z = zip::ZipWriter::new(Cursor::new(&mut buf));
        let o = zip::write::SimpleFileOptions::default();
        for i in 0..=MAX_ARCHIVE_ENTRIES {
            z.start_file(format!("f{i}"), o).unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
    }
    assert!(unpack_archive(&buf, &p).is_err());
}
#[test]
fn archive_unpacked_size_limit_counts_written_bytes() {
    let mut zip_bytes = Vec::new();
    {
        let mut z = zip::ZipWriter::new(Cursor::new(&mut zip_bytes));
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("a.tex", o).unwrap();
        z.write_all(&[b'x'; 600]).unwrap();
        z.start_file("b.tex", o).unwrap();
        z.write_all(&[b'y'; 600]).unwrap();
        z.finish().unwrap();
    }
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        for name in ["a.tex", "b.tex"] {
            let mut header = tar::Header::new_gnu();
            header.set_size(600);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, &[b'z'; 600][..])
                .unwrap();
        }
        builder.finish().unwrap();
    }
    for (name, bytes) in [("zip", &zip_bytes), ("tar", &tar_bytes)] {
        let within = temp(&format!("archive-size-ok-{name}"));
        crate::archive::unpack_archive_with_limits(bytes, &within, 16, 1200).unwrap();
        let over = temp(&format!("archive-size-over-{name}"));
        assert!(crate::archive::unpack_archive_with_limits(bytes, &over, 16, 1199).is_err());
    }
}
#[test]
fn work_id_must_be_a_plain_path_component() {
    for bad in ["", ".", "..", "../evil", "a/b", "a\\b", "name with space"] {
        assert!(crate::snapshot::work_dir_name(bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        crate::snapshot::work_dir_name("oeis-a189733_curtz.v2").unwrap(),
        "oeis-a189733_curtz.v2"
    );
}
#[test]
fn fetch_rejects_sha256_mismatch() {
    let (base, stop, join) = mock_server(|path| {
        if path.ends_with("/pdf") {
            return (200, b"pdf".to_vec());
        }
        if path.ends_with("/source") {
            return (200, b"not-an-archive".to_vec());
        }
        (404, b"{}".to_vec())
    });
    let root = temp("hash-mismatch");
    fs::create_dir_all(root.join("raw/proofs")).unwrap();
    let meta = SnapshotMeta {
        schema_version: "1".into(),
        generator_version: GENERATOR_VERSION.into(),
        api_base: base,
        captured_at: "t".into(),
        listings: BTreeMap::new(),
        collected_works: 1,
        accepted_records: 0,
        directory_nodes: 0,
    };
    write_json(
        &root.join("snapshot.json"),
        &serde_json::to_value(&meta).unwrap(),
    )
    .unwrap();
    write_json(&root.join("raw/collected.json"), &json!([{"id":"w","revision":"1","license":"CC-BY-4.0","authors":[],"pdf":{"url":"/pdf","sha256":"0000000000000000000000000000000000000000000000000000000000000000"},"source":{"url":"/source","sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}])).unwrap();
    write_json(&root.join("raw/papers.json"), &json!([])).unwrap();
    write_json(&root.join("raw/opportunities.json"), &json!([])).unwrap();
    assert!(fetch_files(&root).is_err());
    let _ = stop.send(());
    let _ = join.join();
}
#[test]
fn render_is_byte_deterministic() {
    let root = temp("deterministic-snapshot");
    fs::create_dir_all(root.join("raw/proofs")).unwrap();
    let meta = SnapshotMeta {
        schema_version: "1".into(),
        generator_version: GENERATOR_VERSION.into(),
        api_base: "https://x".into(),
        captured_at: "t".into(),
        listings: BTreeMap::new(),
        collected_works: 0,
        accepted_records: 0,
        directory_nodes: 0,
    };
    write_json(
        &root.join("snapshot.json"),
        &serde_json::to_value(&meta).unwrap(),
    )
    .unwrap();
    write_json(&root.join("raw/collected.json"), &json!([])).unwrap();
    write_json(&root.join("raw/papers.json"), &json!([])).unwrap();
    write_json(&root.join("raw/opportunities.json"), &json!([])).unwrap();
    let a = temp("render-a");
    let b = temp("render-b");
    render(&root, &a, "org", "Mathesis", "https://site", None).unwrap();
    render(&root, &b, "org", "Mathesis", "https://site", None).unwrap();
    let mut ma = BTreeMap::new();
    let mut mb = BTreeMap::new();
    collect_files(&a, &a, &mut ma, false).unwrap();
    collect_files(&b, &b, &mut mb, false).unwrap();
    assert_eq!(ma, mb);
}

#[test]
fn snapshot_mismatch_preserves_previous_directory() {
    let (base, stop, join) = mock_server(mock_catalog_handler(true));
    let root = temp("mismatch").join("snapshot");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("marker"), b"old").unwrap();
    assert!(snapshot(&base, &root).is_err());
    assert_eq!(fs::read(root.join("marker")).unwrap(), b"old");
    let _ = stop.send(());
    let _ = join.join();
}

fn fixture_node(
    work: &str,
    claim: &str,
    status: &str,
    target: Option<&str>,
    receipt: Option<Value>,
    basis: Option<Value>,
) -> Node {
    Node {
        identity: unique_node_key(work, 1, claim),
        lineage: format!("collected:{work}:{claim}"),
        work: work.into(),
        record: work.into(),
        version: 1,
        claim: claim.into(),
        label: claim.into(),
        kind: "paper".into(),
        title: "same text".into(),
        statement: "same text".into(),
        macros: BTreeMap::new(),
        status: status.into(),
        target_digest: target.map(str::to_owned),
        toolchain: Some("lean".into()),
        receipt,
        proof_basis: basis,
        authors: vec![],
        contributor: None,
        attribution: None,
        lean_source: None,
        proof_state: Some(status.into()),
        next_step: None,
        scope: "in_scope".into(),
    }
}

#[test]
fn statistics_keep_similar_statements_separate_and_split_basis() {
    let works = vec![
        Work {
            work: "w1".into(),
            repository_name: Some("w1".into()),
            record: "w1".into(),
            title: "One".into(),
            authors: vec![],
            license: Some("CC-BY-4.0".into()),
            publication: "public".into(),
            doi: None,
            concept_doi: None,
            versions: vec![],
            current_version: Some(1),
            abstract_text: None,
            lean_source: Some(LeanSource {
                url: "https://github.com/x/y/blob/a/A.lean".into(),
                revision: "a".into(),
            }),
            pdf: None,
            source: None,
            msc: vec!["11B".into()],
            published_on: None,
            previous_version: None,
        },
        Work {
            work: "w2".into(),
            repository_name: Some("w2".into()),
            record: "w2".into(),
            title: "Two".into(),
            authors: vec![],
            license: Some("CC-BY-4.0".into()),
            publication: "public".into(),
            doi: None,
            concept_doi: None,
            versions: vec![],
            current_version: Some(1),
            abstract_text: None,
            lean_source: None,
            pdf: None,
            source: None,
            msc: vec!["11B".into()],
            published_on: None,
            previous_version: None,
        },
    ];
    let receipt = |verdict: &str| json!({"verdict":verdict,"target_digest":"d","checked_at":"t"});
    let library = json!({"kind":"library_declarations"});
    let nodes = vec![
        fixture_node(
            "w1",
            "c1",
            "verified",
            Some("d"),
            Some(receipt("proved")),
            Some(library),
        ),
        fixture_node(
            "w1",
            "c2",
            "verified",
            Some("d"),
            Some(receipt("proved")),
            Some(json!({"kind":"new_lean"})),
        ),
        fixture_node(
            "w2",
            "c1",
            "disproved",
            Some("x"),
            Some(receipt("disproved")),
            None,
        ),
        fixture_node("w2", "c2", "open", Some("o"), None, None),
        fixture_node("w2", "c3", "preparing", None, None, None),
    ];
    let snapshot = Snapshot {
        root: PathBuf::new(),
        meta: SnapshotMeta {
            schema_version: "1".into(),
            generator_version: GENERATOR_VERSION.into(),
            api_base: "https://example".into(),
            captured_at: "t".into(),
            listings: BTreeMap::new(),
            collected_works: 2,
            accepted_records: 0,
            directory_nodes: 5,
        },
        collected: vec![],
        accepted: vec![],
        opportunities: vec![],
        proofs: vec![],
    };
    let stats = compute_stats(&snapshot, &works, &nodes);
    assert_eq!(stats.registered_statements, 5);
    assert_eq!(stats.independently_verified_statements, 2);
    assert_eq!(stats.verified_from_existing_library_declarations, 1);
    assert_eq!(stats.verified_with_new_lean, 1);
    assert_eq!(stats.verified_counterexamples_disproofs, 1);
    assert_eq!(stats.open_formalization_targets, 1);
    assert_eq!(stats.statements_awaiting_formal_target, 1);
    assert_eq!(stats.papers_with_lean_source, 1);
    assert_eq!(stats.by_msc["11"].registered, 5);
}

#[test]
fn cff_has_required_version_and_authors() {
    let w = fixture_node("w", "c", "preparing", None, None, None);
    let work = Work {
        work: "w".into(),
        repository_name: Some("w".into()),
        record: "w".into(),
        title: "Title".into(),
        authors: vec![json!({"name":"Author"})],
        license: Some("CC-BY-4.0".into()),
        publication: "public".into(),
        doi: None,
        concept_doi: None,
        versions: vec![],
        current_version: Some(1),
        abstract_text: None,
        lean_source: None,
        pdf: None,
        source: None,
        msc: vec![],
        published_on: None,
        previous_version: None,
    };
    let c = citation(&work, "https://site");
    assert!(c.contains("cff-version: 1.2.0"));
    assert!(c.contains("name: \"Author\""));
    let _ = w;
}

#[test]
fn check_rejects_tampered_stats() {
    let root = temp("check-snapshot");
    fs::create_dir_all(root.join("raw/proofs")).unwrap();
    let meta = SnapshotMeta {
        schema_version: "1".into(),
        generator_version: GENERATOR_VERSION.into(),
        api_base: "https://x".into(),
        captured_at: "t".into(),
        listings: BTreeMap::new(),
        collected_works: 0,
        accepted_records: 0,
        directory_nodes: 0,
    };
    write_json(
        &root.join("snapshot.json"),
        &serde_json::to_value(&meta).unwrap(),
    )
    .unwrap();
    write_json(&root.join("raw/collected.json"), &json!([])).unwrap();
    write_json(&root.join("raw/papers.json"), &json!([])).unwrap();
    write_json(&root.join("raw/opportunities.json"), &json!([])).unwrap();
    let out = temp("check-output");
    render(&root, &out, "org", "Mathesis", "https://site", None).unwrap();
    let stats_path = out.join("catalog/stats.json");
    let mut stats = read_json(&stats_path).unwrap();
    stats["public_papers"] = json!(99);
    write_json(&stats_path, &stats).unwrap();
    assert!(check(&out).is_err());
}

#[test]
fn cff_is_valid_yaml_and_escapes_special_scalars() {
    let work = Work {
        work: "special".into(),
        repository_name: Some("special".into()),
        record: "WP-2026-0099".into(),
        title: "A \"quoted\" title: \u{03bb} # result \\ path".into(),
        authors: vec![json!({"name": "\u{00c9} Author: #1 \\ \""})],
        license: Some("CC-BY-4.0".into()),
        publication: "public".into(),
        doi: Some("10.1234/example".into()),
        concept_doi: Some("10.1234/concept".into()),
        versions: vec![],
        current_version: Some(3),
        abstract_text: None,
        lean_source: None,
        pdf: None,
        source: None,
        msc: vec![],
        published_on: Some("2026-11-12T04:05:06Z".into()),
        previous_version: None,
    };
    let cff = citation(&work, "https://mathesis.example");
    let parsed: serde_yaml::Value = serde_yaml::from_str(&cff).expect("generated CFF is YAML");
    assert_eq!(parsed["cff-version"].as_str(), Some("1.2.0"));
    assert_eq!(parsed["title"].as_str(), Some(work.title.as_str()));
    assert_eq!(
        parsed["authors"][0]["name"].as_str(),
        Some("\u{00c9} Author: #1 \\ \"")
    );
    assert_eq!(parsed["date-released"].as_str(), Some("2026-11-12"));
    assert_eq!(parsed["version"].as_str(), Some("3"));
    assert_eq!(
        parsed["url"].as_str(),
        Some("https://mathesis.example/papers/WP-2026-0099?version=3")
    );
    assert_eq!(parsed["doi"].as_str(), Some("10.1234/example"));
    assert_eq!(parsed["license"].as_str(), Some("CC-BY-4.0"));
}

#[test]
fn accepted_and_conjecture_directory_rows_are_scoped_and_partitioned() {
    let (base, stop, join) = mock_server(|path| {
        let body = |v: Value| serde_json::to_vec(&v).unwrap();
        if path.starts_with("/api/v1/collected-papers?") {
            return (
                200,
                body(
                    json!({"items":[{"id":"collected","revision":"1","title":"Collected","abstract":"A","authors":[],"license":"CC-BY-4.0","lean":{"url":"https://github.com/x/y/blob/a/A.lean","revision":"a"},"pdf":{"url":"/pdf"},"source":{"url":"/source"}}],"next_before":null}),
                ),
            );
        }
        if path.starts_with("/api/v1/papers?") {
            return (
                200,
                body(
                    json!({"items":[{"record":"WP-2026-0001","title":"Accepted","authors":[],"license":"CC-BY-4.0","msc":[],"accepted_at":"2026-11-12T00:00:00Z"}],"next_before":null}),
                ),
            );
        }
        if path.starts_with("/api/v1/formalizations/opportunities") {
            if path.contains("status=unverified") {
                return (
                    200,
                    body(json!({"items":[
                    {"record":"collected","claim":"C1","version":1,"title":"Collected","label":"Theorem","statement":"same","authors":[],"kind":"paper","status":"preparing"},
                    {"record":"WP-2026-0001","claim":"C2","version":1,"title":"Accepted","label":"Conjecture","statement":"same","authors":[],"kind":"conjecture","status":"preparing"}
                ],"next_before":null})),
                );
            }
            return (200, body(json!({"items":[],"next_before":null})));
        }
        if path.starts_with("/api/v1/collected-papers/collected/proofs") {
            return (
                200,
                body(
                    json!([{"claim":"C1","version":1,"state":"preparing","target_digest":null,"next_step":"x","proofs":[]}]),
                ),
            );
        }
        (404, b"{}".to_vec())
    });
    let root = temp("accepted-directory").join("snapshot");
    let meta = snapshot(&base, &root).unwrap();
    assert_eq!(meta.directory_nodes, 2);
    let snap = load_snapshot(&root).unwrap();
    let nodes = make_nodes(&snap).unwrap();
    assert!(
        nodes
            .iter()
            .any(|n| n.identity == "collected:collected:1:C1" && n.scope == "in_scope")
    );
    assert!(
        nodes
            .iter()
            .any(|n| n.identity == "record:WP-2026-0001:1:C2"
                && n.kind == "conjecture"
                && n.scope == "requires_author_consent")
    );
    let mapping = BTreeMap::new();
    let works = vec![
        paper_work(&snap.collected[0], false, &mapping).unwrap(),
        paper_work(&snap.accepted[0], true, &mapping).unwrap(),
    ];
    let stats = compute_stats(&snap, &works, &nodes);
    assert_eq!(stats.public_papers, 1);
    assert_eq!(stats.registered_statements, 1);
    let _ = stop.send(());
    let _ = join.join();
}

#[test]
fn atomic_replace_keeps_unrelated_previous_directory() {
    let root = temp("atomic-backup");
    let output = root.join("output");
    let previous = root.join("output.previous");
    fs::create_dir_all(&output).unwrap();
    fs::create_dir_all(&previous).unwrap();
    fs::write(output.join("old"), b"old").unwrap();
    fs::write(previous.join("keep"), b"keep").unwrap();
    let temp_dir = root.join("temp");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(temp_dir.join("new"), b"new").unwrap();
    atomic_replace_dir(&temp_dir, &output).unwrap();
    assert_eq!(fs::read(output.join("new")).unwrap(), b"new");
    assert_eq!(fs::read(previous.join("keep")).unwrap(), b"keep");
}

#[test]
fn render_records_concept_doi_and_file_schema_without_embedded_nodes() {
    let root = temp("concept-doi");
    fs::create_dir_all(root.join("raw/proofs")).unwrap();
    let meta = SnapshotMeta {
        schema_version: "1".into(),
        generator_version: GENERATOR_VERSION.into(),
        api_base: "https://site".into(),
        captured_at: "2026-11-12T00:00:00Z".into(),
        listings: BTreeMap::new(),
        collected_works: 1,
        accepted_records: 0,
        directory_nodes: 0,
    };
    write_json(
        &root.join("snapshot.json"),
        &serde_json::to_value(&meta).unwrap(),
    )
    .unwrap();
    write_json(
        &root.join("raw/collected.json"),
        &json!([{
            "id":"concept-work", "revision":"2", "title":"Concept", "abstract":"A",
            "authors":[{"name":"Author"}], "license":"CC-BY-4.0",
            "concept_doi":"10.1234/concept", "archive":{"doi":"10.1234/version"},
            "lean":{"url":"https://github.com/x/y/blob/abc/A.lean","revision":"abc"},
            "published_on":"2026-11-11T00:00:00Z", "pdf":{"url":"/pdf"}, "source":{"url":"/source"}
        }]),
    )
    .unwrap();
    write_json(&root.join("raw/papers.json"), &json!([])).unwrap();
    write_json(&root.join("raw/opportunities.json"), &json!([])).unwrap();
    let out = temp("concept-doi-output");
    render(&root, &out, "org", "Mathesis", "https://site", None).unwrap();
    let works = read_json(&out.join("catalog/works.json")).unwrap();
    assert_eq!(works["works"][0]["concept_doi"], "10.1234/concept");
    let paper = read_json(&out.join("papers/concept-work/paper.json")).unwrap();
    assert!(paper.get("nodes").is_none());
    assert!(paper["files"]["pdf"].get("path").is_some());
    assert!(paper["files"]["source"].get("matches_public").is_some());
    let contributing = fs::read_to_string(out.join("papers/concept-work/CONTRIBUTING.md")).unwrap();
    assert!(contributing.contains("https://site/contribute"));
}
