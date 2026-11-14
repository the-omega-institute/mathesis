use super::*;
use crate::tests::{mock_server, temp};

fn summary() -> Value {
    json!({"record":"WP-2026-0007","kind":"paper","title":"A paper", "authors":[{"name":"A. Author"}],
        "abstract_text":"A result.","archive_consent":true,"accepted_at":"2026-11-14T00:00:00Z"})
}
fn detail() -> Value {
    let pdf = b"%PDF-1.4 exact original";
    let source = b"\\documentclass{article}\\begin{document}A result.\\end{document}";
    json!({"summary":summary(),"publication":{"version":2,"claims_revision":1},
        "archive":{"marker":"public-archive-2026-11-12","licence":"CC-BY-4.0","consent_time":"2026-11-14T00:00:00Z",
            "doi":"10.1234/exact","concept_doi":"10.1234/all",
            "pdf":{"url":"/f/pdf","sha256":sha256(pdf),"bytes":pdf.len()},
            "source":{"url":"/f/source","sha256":sha256(source),"bytes":source.len()}},
        "plain_language":{"en":"English reading.","zh":"中文解读。"}})
}

#[test]
fn consented_archive_enters_the_same_frozen_projection_without_review() {
    let (base, stop, join) = mock_server(|path| {
        let value = if path.starts_with("/api/v1/papers?") {
            json!({"items":[summary(),{"record":"private","title":"Private"}],"next_before":null})
        } else if path == "/api/v1/papers/WP-2026-0007" {
            detail()
        } else if path == "/f/pdf" {
            return (200, b"%PDF-1.4 exact original".to_vec());
        } else if path == "/f/source" {
            return (
                200,
                b"\\documentclass{article}\\begin{document}A result.\\end{document}".to_vec(),
            );
        } else if path.starts_with("/api/v1/leaderboard") {
            json!([])
        } else {
            assert!(
                !path.contains("private"),
                "unconsented source must not be requested"
            );
            json!({"items":[],"next_before":null})
        };
        (200, serde_json::to_vec(&value).unwrap())
    });
    let root = temp("accepted-archive");
    let snap = root.join("snapshot");
    snapshot(&base, &snap).unwrap();
    fetch_files(&snap).unwrap();
    let out = root.join("out");
    render(&snap, &out, "mathesis-papers", "Mathesis", &base, None).unwrap();
    check(&out).unwrap();
    let paper = out.join("papers/WP-2026-0007");
    assert_eq!(
        fs::read(paper.join("paper.pdf")).unwrap(),
        b"%PDF-1.4 exact original"
    );
    assert_eq!(
        fs::read(paper.join("source.archive")).unwrap(),
        fs::read(paper.join("manuscript/main.tex")).unwrap()
    );
    assert!(!out.join("papers/private").exists());
    let work = read_json(&paper.join("paper.json")).unwrap();
    assert_eq!(work["work"]["current_version"], 2);
    assert_eq!(work["work"]["doi"], "10.1234/exact");
    assert_eq!(work["files"]["source"]["matches_public"], true);
    assert_eq!(work["files"]["source"]["path"], "source.archive");
    let before = fs::read(paper.join(".mathesis/generated.json")).unwrap();
    let mut meta = read_json(&snap.join("snapshot.json")).unwrap();
    meta["captured_at"] = json!("2026-11-15T00:00:00Z");
    write_json(&snap.join("snapshot.json"), &meta).unwrap();
    render(&snap, &out, "mathesis-papers", "Mathesis", &base, None).unwrap();
    assert_eq!(
        before,
        fs::read(paper.join(".mathesis/generated.json")).unwrap()
    );
    let _ = stop.send(());
    join.join().unwrap();
}

#[test]
fn archive_adapter_rejects_changed_identity_or_missing_exact_consent() {
    for mutation in ["record", "marker", "hash", "version"] {
        let (base, stop, join) = mock_server(move |_| {
            let mut d = detail();
            match mutation {
                "record" => d["summary"]["record"] = json!("another"),
                "marker" => d["archive"]["marker"] = json!("legacy"),
                "hash" => d["archive"]["source"]["sha256"] = json!(""),
                "version" => d["publication"]["version"] = json!(0),
                _ => unreachable!(),
            }
            (200, serde_json::to_vec(&d).unwrap())
        });
        assert!(accepted::collect_archive(&http_client().unwrap(), &base, summary()).is_err());
        let _ = stop.send(());
        join.join().unwrap();
    }
    let row = json!({"record":"old","license":"CC-BY-4.0"});
    assert!(!accepted::archive_eligible(&row));
    assert_eq!(
        paper_work(&row, true, &BTreeMap::new())
            .unwrap()
            .publication,
        "requires_author_consent"
    );
}

#[test]
fn accepted_pdf_only_and_tar_uploads_keep_original_bytes_and_paths() {
    let pdf = b"%PDF-1.4 exact original".to_vec();
    let mut archive = tar::Builder::new(Vec::new());
    let tex = b"\\documentclass{article}";
    let mut header = tar::Header::new_gnu();
    header.set_size(tex.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "nested/main.tex", &tex[..])
        .unwrap();
    let tar = archive.into_inner().unwrap();
    for (source, path) in [(pdf.clone(), "manuscript.pdf"), (tar, "nested/main.tex")] {
        let original = source.clone();
        let (base, stop, join) = mock_server(move |path| {
            if path == "/f/source" {
                return (200, source.clone());
            }
            if path == "/f/pdf" {
                return (200, b"%PDF-1.4 exact original".to_vec());
            }
            let value = if path.starts_with("/api/v1/papers?") {
                json!({"items":[summary()], "next_before":null})
            } else if path == "/api/v1/papers/WP-2026-0007" {
                let mut d = detail();
                d["archive"]["source"]["sha256"] = json!(sha256(&source));
                d["archive"]["source"]["bytes"] = json!(source.len());
                d
            } else if path.starts_with("/api/v1/leaderboard") {
                json!([])
            } else {
                json!({"items":[], "next_before":null})
            };
            (200, serde_json::to_vec(&value).unwrap())
        });
        let root = temp("accepted-upload");
        snapshot(&base, &root).unwrap();
        fetch_files(&root).unwrap();
        assert_eq!(
            fs::read(root.join("files/WP-2026-0007/source.archive")).unwrap(),
            original
        );
        assert!(
            root.join("files/WP-2026-0007/manuscript")
                .join(path)
                .is_file()
        );
        let _ = stop.send(());
        join.join().unwrap();
    }
}

#[test]
fn accepted_downloads_reject_size_and_hash_mismatches() {
    for size_error in [true, false] {
        let (base, stop, join) = mock_server(move |path| {
            let value = if path.starts_with("/api/v1/papers?") {
                json!({"items":[summary()], "next_before":null})
            } else if path == "/api/v1/papers/WP-2026-0007" {
                let mut d = detail();
                if size_error {
                    d["archive"]["pdf"]["bytes"] = json!(1);
                } else {
                    d["archive"]["pdf"]["sha256"] = json!("a".repeat(64));
                }
                d
            } else if path == "/f/pdf" {
                return (200, b"%PDF-1.4 exact original".to_vec());
            } else if path.starts_with("/api/v1/leaderboard") {
                json!([])
            } else {
                json!({"items":[], "next_before":null})
            };
            (200, serde_json::to_vec(&value).unwrap())
        });
        let root = temp("accepted-mismatch");
        snapshot(&base, &root).unwrap();
        let error = fetch_files(&root).unwrap_err().to_string();
        assert!(error.contains(if size_error {
            "size mismatch"
        } else {
            "SHA-256 mismatch"
        }));
        let _ = stop.send(());
        join.join().unwrap();
    }
}
