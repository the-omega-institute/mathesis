use super::*;
use crate::ports::{CollectedPaperStore, MaterialsStore, RecordStore, SubmissionStore};
use serde_json::{Value, json};

fn entry(kind: &str, id: &str, version: u32) -> Value {
    json!({"kind":kind,"id":id,"version":version,
        "repository_url":"https://github.com/mathesis-papers/paper",
        "site_url":"https://papers.trureturing.org/paper/",
        "reading":{"en":"https://papers.trureturing.org/paper/en/"},
        "slides":[{"id":"seminar","language":"zh","pdf_url":"https://papers.trureturing.org/paper/slides/zh.pdf","notes_url":null}],
        "visualizations":[{"id":"demo","url":"https://papers.trureturing.org/paper/demo/#figure"}]})
}
fn index(entries: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":"mathesis.materials-index.v1",
        "generated_at":"2026-11-13T00:00:00Z","entries":entries}))
    .unwrap()
}

#[test]
fn materials_index_bounds_schema_and_individual_malformed_rows() {
    let valid = entry("collected", "paper", 1);
    let mut bad = valid.clone();
    bad["version"] = json!(0);
    let mut wrong_type = valid.clone();
    wrong_type["reading"] = json!([]);
    let mut too_many = valid.clone();
    too_many["slides"] = json!(vec![valid["slides"][0].clone(); 65]);
    let mut language = valid.clone();
    language["slides"][0]["language"] = Value::Null;
    let mut malformed = valid.clone();
    malformed["visualizations"][0]["url"] = Value::Null;
    let parsed = parse_materials_index(&index(vec![
        Value::Null,
        bad,
        wrong_type,
        too_many,
        language,
        malformed,
        valid,
    ]))
    .unwrap();
    assert_eq!(parsed.len(), 1);
    for bytes in [
        b"not JSON".to_vec(),
        vec![b' '; MATERIALS_INDEX_MAX_BYTES + 1],
        br#"{"schema":"wrong","generated_at":"2026-11-13T00:00:00Z","entries":[]}"#.to_vec(),
        br#"{"schema":"mathesis.materials-index.v1","generated_at":"yesterday","entries":[]}"#
            .to_vec(),
        br#"{"schema":"mathesis.materials-index.v1","generated_at":"2026-11-13T00:00:00Z"}"#
            .to_vec(),
        index(vec![Value::Null; MATERIALS_INDEX_MAX_ENTRIES + 1]),
    ] {
        assert!(parse_materials_index(&bytes).is_err());
    }
}

#[test]
fn materials_url_boundaries_refuse_cross_repo_and_url_normalization_escapes() {
    for raw in [
        "http://papers.trureturing.org/paper/en/",
        "https://papers.trureturing.org.evil.test/paper/",
        "https://papers.trureturing.org/paper-other/en/",
        "https://papers.trureturing.org/other/en/",
        "https://papers.trureturing.org/paper/../other/",
        "https://papers.trureturing.org/paper/%2e%2e/other/",
        "https://papers.trureturing.org/paper/en/?x=1",
        "https://papers.trureturing.org/paper/en/\n",
        "https://papers.trureturing.org/paper/\\other/",
        "https://user:secret@papers.trureturing.org/paper/",
        "https://papers.trureturing.org:444/paper/",
        "https://github.com/mathesis-papers/paper-other",
        "https://github.com/mathesis-papers/other/blob/main/README.md",
        "javascript:alert(1)",
    ] {
        // Refuse an entire row if any URL field escapes, including optional notes.
        for path in [
            "reading",
            "slides",
            "visualizations",
            "repository_url",
            "site_url",
        ] {
            let mut row = entry("collected", "paper", 1);
            match path {
                "reading" => row["reading"]["en"] = json!(raw),
                "slides" => row["slides"][0]["notes_url"] = json!(raw),
                "visualizations" => row["visualizations"][0]["url"] = json!(raw),
                _ => row[path] = json!(raw),
            }
            assert!(
                parse_materials_index(&index(vec![row])).unwrap().is_empty(),
                "{path}: {raw}"
            );
        }
    }
    let mut row = entry("record", "WP-2026-0001", 1);
    row["reading"]["en"] = json!("https://github.com/mathesis-papers/paper/blob/main/README.md");
    assert_eq!(parse_materials_index(&index(vec![row])).unwrap().len(), 1);
}

#[tokio::test]
async fn collected_materials_replace_remove_and_project_only_the_current_version() {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader)),
        Policy::default(),
        BTreeSet::from([PersonId::from("admin")]),
        20,
        BTreeSet::from([PersonId::from("admin")]),
    );
    let owner = w.person("admin", &[Role::Admin]).await;
    let (d, p, s) = super::collected::package(1);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    let before = CollectedPaperStore::get(w.stores.as_ref(), "a-note", Some(1))
        .await
        .unwrap()
        .unwrap();
    let queued = w.stores.queued().await;
    let old = entry("collected", "a-note", 1);
    let mut latest = old.clone();
    latest["reading"]["zh"] = json!("https://papers.trureturing.org/paper/zh/");
    assert_eq!(
        w.app
            .reconcile_materials_index(&index(vec![
                old.clone(),
                entry("collected", "unknown", 1),
                entry("collected", "a-note", 2),
                latest.clone(),
                json!({"kind":"collected","id":"a-note"})
            ]))
            .await
            .unwrap(),
        1
    );
    let public = w.app.collected_paper("a-note", None).await.unwrap();
    assert_eq!(public.materials.unwrap().reading.len(), 2);
    assert_eq!(
        CollectedPaperStore::get(w.stores.as_ref(), "a-note", Some(1))
            .await
            .unwrap()
            .unwrap(),
        before
    );
    assert_eq!(w.stores.queued().await, queued);
    assert!(w.app.reconcile_materials_index(b"broken").await.is_err());
    assert!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .materials
            .is_some()
    );
    let sidecar = w.stores.load_materials().await.unwrap();
    w.app
        .reconcile_materials_index(&index(vec![latest]))
        .await
        .unwrap();
    assert_eq!(
        w.stores.load_materials().await.unwrap().revision,
        sidecar.revision
    );
    let (d, p, s) = super::collected::package(2);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .materials
            .is_none()
    );
    assert!(
        w.app
            .collected_paper("a-note", Some(1))
            .await
            .unwrap()
            .materials
            .is_none()
    );
    w.app
        .reconcile_materials_index(&index(vec![entry("collected", "a-note", 2)]))
        .await
        .unwrap();
    assert_eq!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .materials
            .unwrap()
            .version,
        2
    );
    w.app
        .reconcile_materials_index(&index(vec![]))
        .await
        .unwrap();
    assert!(w.stores.load_materials().await.unwrap().entries.is_empty());
    assert!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .materials
            .is_none()
    );
    assert!(matches!(
        w.stores
            .replace_materials(
                &MaterialsSidecar {
                    revision: sidecar.revision + 1,
                    entries: vec![]
                },
                sidecar.revision
            )
            .await,
        Err(CoreError::StaleRevision { .. })
    ));
}

#[tokio::test]
async fn record_materials_require_the_exact_public_accepted_version() {
    let w = World::new().await;
    let author = w.person("materials-author", &[]).await;
    let submitted = w.submit(&author, "t", false).await;
    let record_id = RecordId("WP-2026-0001".into());
    let mut accepted = submitted.clone();
    accepted.status = SubmissionStatus::Accepted {
        record: record_id.clone(),
    };
    accepted.revision += 1;
    SubmissionStore::replace(w.stores.as_ref(), &accepted, submitted.revision)
        .await
        .unwrap();
    let submitted = accepted.clone();
    let record = Record {
        kind: accepted.kind,
        id: record_id.clone(),
        publication: Some(Box::new(accepted.clone())),
        submission: accepted.id.clone(),
        title: accepted.title.clone(),
        authors: accepted.authors.clone(),
        basis: AdmissionBasis::EscapeWitness,
        accepted_by: author.person.clone(),
        accepted_at: chrono::Utc::now(),
    };
    RecordStore::insert(w.stores.as_ref(), &record)
        .await
        .unwrap();
    let original_record = RecordStore::get(w.stores.as_ref(), &record_id)
        .await
        .unwrap()
        .unwrap();
    let old = entry("record", record_id.as_str(), 1);
    assert_eq!(
        w.app
            .reconcile_materials_index(&index(vec![
                old.clone(),
                entry("record", record_id.as_str(), 2),
                entry("record", "WP-2026-9999", 1)
            ]))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        w.app
            .paper(&record_id)
            .await
            .unwrap()
            .materials
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        RecordStore::get(w.stores.as_ref(), &record_id)
            .await
            .unwrap()
            .unwrap(),
        original_record
    );
    assert_eq!(
        SubmissionStore::get(w.stores.as_ref(), &submitted.id)
            .await
            .unwrap()
            .unwrap(),
        submitted
    );
    w.app
        .set_analysis_visibility(&author, &submitted.id, Visibility::Private)
        .await
        .unwrap();
    assert!(w.app.paper(&record_id).await.unwrap().materials.is_none());
    assert!(
        !serde_json::to_value(w.app.paper(&record_id).await.unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("materials")
    );
    assert_eq!(
        w.app
            .reconcile_materials_index(&index(vec![old.clone()]))
            .await
            .unwrap(),
        0
    );
    w.app
        .set_analysis_visibility(&author, &submitted.id, Visibility::Public)
        .await
        .unwrap();
    w.app
        .reconcile_materials_index(&index(vec![old]))
        .await
        .unwrap();
    // An unpublished successor exists, but the immutable accepted input stays v1.
    let mut current = SubmissionStore::get(w.stores.as_ref(), &submitted.id)
        .await
        .unwrap()
        .unwrap();
    let expected = current.revision;
    let mut next = current.versions[0].clone();
    next.number = 2;
    current.versions.push(next);
    current.revision += 1;
    SubmissionStore::replace(w.stores.as_ref(), &current, expected)
        .await
        .unwrap();
    w.app
        .reconcile_materials_index(&index(vec![
            entry("record", record_id.as_str(), 1),
            entry("record", record_id.as_str(), 2),
        ]))
        .await
        .unwrap();
    assert_eq!(
        w.app
            .paper(&record_id)
            .await
            .unwrap()
            .materials
            .unwrap()
            .version,
        1
    );
    w.app
        .reconcile_materials_index(&index(vec![]))
        .await
        .unwrap();
    assert!(w.app.paper(&record_id).await.unwrap().materials.is_none());
}
