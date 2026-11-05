use super::*;
use crate::ports::CollectedPaperStore;
use sha2::{Digest, Sha256};

fn package(version: u32) -> (PreReviewedDelivery, Upload, Upload) {
    let pdf = Upload {
        filename: "note.pdf".into(),
        bytes: b"%PDF-reviewed".to_vec(),
    };
    let source = Upload {
        filename: "source.zip".into(),
        bytes: b"PK\x03\x04reviewed".to_vec(),
    };
    let delivery = serde_json::from_value(serde_json::json!({
        "schema": "mathesis.pre-reviewed-delivery.v1", "producer":"publisher", "work_id":"a-note",
        "version":version,"previous_version":(version>1).then_some(version-1),"authorize_publication":true,
        "paper":{"title":"A note", "abstract":"A bound on gaps", "authors":[{"name":"Original Author"}],
          "license":"CC-BY-4.0", "plain_language":{"en":"Original explanation.","zh":"原始介绍。"}},
        "review":{"verdict":"PASS", "pdf_sha256":format!("{:x}",Sha256::digest(&pdf.bytes)),
          "source_sha256":format!("{:x}",Sha256::digest(&source.bytes)),"record":{"private_evidence":"must not be exposed"}}
    })).unwrap();
    (delivery, pdf, source)
}

#[tokio::test]
async fn collection_queues_only_exposition_and_keeps_immutable_receipt_on_retry() {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader)),
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        20,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let admin = w.person("admin", &[]).await;
    let deliver = || package(1);
    let (d, p, s) = deliver();
    let receipt = w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert_eq!(
        w.stores.queued().await,
        vec![("collected:a-note:1".into(), JobKind::CollectedExplanation)]
    );
    let before = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    assert!(
        w.app
            .collected_explanation_input(&owner, "a-note", 1)
            .await
            .is_err()
    );
    assert!(
        w.app
            .request_collected_explanation(&owner, "a-note", 1)
            .await
            .is_err()
    );
    let value = CollectedExplanation {
        revision: 1,
        text: Some(PlainLanguageText {
            en: "A complete reading guide.".into(),
            zh: "完整的论文解读。".into(),
        }),
        progress: serde_json::json!({"private_agent":"not public"}),
    };
    assert!(
        w.app
            .save_collected_explanation(&owner, "a-note", 1, value.clone(), 0)
            .await
            .is_err()
    );
    w.app
        .save_collected_explanation(&reviewer, "a-note", 1, value.clone(), 0)
        .await
        .unwrap();
    assert!(matches!(
        w.app
            .save_collected_explanation(&reviewer, "a-note", 1, value.clone(), 0)
            .await,
        Err(CoreError::StaleRevision { .. })
    ));
    let (d, p, s) = deliver();
    let retry = w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::to_value(retry).unwrap()
    );
    let mut after = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    assert_eq!(after.explanation, value);
    after.explanation = CollectedExplanation::default();
    assert_eq!(before, after);
    let public =
        serde_json::to_value(w.app.collected_paper("a-note", Some(1)).await.unwrap()).unwrap();
    assert_eq!(public["plain_language"]["en"], "A complete reading guide.");
    let public = public.to_string();
    assert!(!public.contains("private_evidence") && !public.contains("private_agent"));
    w.app
        .request_collected_explanation(&admin, "a-note", 1)
        .await
        .unwrap();
    let (d, p, s) = package(2);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert_eq!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .paper
            .plain_language
            .unwrap()
            .en,
        "Original explanation."
    );
    assert_eq!(
        w.app
            .collected_paper("a-note", Some(1))
            .await
            .unwrap()
            .paper
            .plain_language
            .unwrap()
            .en,
        "A complete reading guide."
    );
}

#[tokio::test]
async fn collection_retry_repairs_exposition_queue_after_publication_was_saved() {
    use crate::ports::ReviewQueue;
    use std::sync::atomic::{AtomicBool, Ordering};
    struct FailOnce {
        stores: Arc<MemoryStores>,
        fail: AtomicBool,
    }
    #[async_trait::async_trait]
    impl ReviewQueue for FailOnce {
        async fn enqueue(&self, id: &crate::ids::SubmissionId, kind: JobKind) -> CoreResult<()> {
            if self.fail.swap(false, Ordering::SeqCst) {
                return Err(CoreError::Unavailable("queue unavailable".into()));
            }
            self.stores.enqueue(id, kind).await
        }
    }
    let mut w = World::new().await;
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.queue = Arc::new(FailOnce {
        stores: w.stores.clone(),
        fail: AtomicBool::new(true),
    });
    w.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::new(),
        20,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let (d, p, s) = package(1);
    assert!(matches!(
        w.app.collect_pre_reviewed(&owner, d, p, s).await,
        Err(CoreError::Unavailable(_))
    ));
    let saved = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    assert!(w.app.collected_paper("a-note", Some(1)).await.is_ok());
    assert!(w.stores.queued().await.is_empty());
    let (d, p, s) = package(1);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert_eq!(
        w.stores.get("a-note", Some(1)).await.unwrap().unwrap(),
        saved
    );
    assert_eq!(
        w.stores.queued().await,
        vec![("collected:a-note:1".into(), JobKind::CollectedExplanation)]
    );
}

struct IndexedReader;
impl PaperReader for IndexedReader {
    fn read(&self, _bytes: &[u8], _filename: &str) -> Result<ReadPaper, String> {
        let mut read = FakeReader.read(b"", "t.tex")?;
        read.bibliography = Some(Bibliography {
            method: "latex_literal_citations_v1".into(),
            references: vec![BibliographicReference {
                key: "Earlier".into(),
                title: Some("An earlier theorem".into()),
                authors: vec!["Earlier Author".into()],
                year: Some("1901".into()),
                doi: Some("10.1234/OLD".into()),
                arxiv: None,
                url: None,
                source_location: "main.tex".into(),
                raw: "Earlier Author, An earlier theorem".into(),
                raw_truncated: false,
            }],
            limitations: vec!["Bounded source extraction".into()],
        });
        Ok(read)
    }
}

#[tokio::test]
async fn collected_graph_indexes_public_sources_and_credits_real_references_without_review() {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores
            .ports(Arc::new(SystemClock), Arc::new(IndexedReader)),
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        20,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let admin = w.person("admin", &[]).await;
    let (delivery, pdf, source) = package(1);
    w.app
        .collect_pre_reviewed(&owner, delivery, pdf, source)
        .await
        .unwrap();
    let saved = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    let graph = w
        .app
        .research_graph(ResearchGraphQuery {
            focus: Some("paper:a-note".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].to, "doi:10.1234/old");
    assert_eq!(graph.edges[0].source_location, "main.tex");
    let paper = graph.nodes.iter().find(|n| n.id == "paper:a-note").unwrap();
    assert_eq!(paper.authors, ["Original Author"]);
    assert_eq!(paper.version, Some(1));
    assert_eq!(paper.record.as_ref().unwrap().as_str(), "a-note");
    assert!(
        !serde_json::to_string(&graph)
            .unwrap()
            .contains("private_evidence")
    );
    let queue = w.stores.queued().await;
    assert_eq!(
        queue,
        vec![("collected:a-note:1".into(), JobKind::CollectedExplanation)]
    );

    // A legacy publication remains a visible node before its references are indexed.
    let mut legacy = saved.clone();
    legacy.id = "legacy-version".into();
    legacy.delivery.work_id = "legacy-note".into();
    legacy.source_index = None;
    w.stores.insert(&legacy).await.unwrap();
    let unindexed = w
        .app
        .research_graph(ResearchGraphQuery {
            focus: Some("paper:legacy-note".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(unindexed.nodes.len(), 1);
    assert!(unindexed.edges.is_empty());
    assert!(unindexed.coverage[0].method.is_none());
    assert!(
        w.app
            .index_collected_paper(&owner, "legacy-note", 1)
            .await
            .is_err()
    );
    w.app
        .index_collected_paper(&admin, "legacy-note", 1)
        .await
        .unwrap();
    w.app
        .index_collected_paper(&admin, "legacy-note", 1)
        .await
        .unwrap();
    let mut after = w.stores.get("legacy-note", Some(1)).await.unwrap().unwrap();
    assert_eq!(after.source_index.as_ref().unwrap().revision, 1);
    after.source_index = None;
    assert_eq!(after, legacy);
    assert_eq!(w.stores.queued().await, queue);

    // Reference search and identifier focus select matches before pagination.
    let first = w
        .app
        .research_graph(ResearchGraphQuery {
            q: Some("Earlier Author".into()),
            limit: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(first.edges.len(), 1);
    assert_eq!(first.coverage.len(), 1);
    let next = w
        .app
        .research_graph(ResearchGraphQuery {
            q: Some("Earlier Author".into()),
            limit: Some(1),
            before: first.next_before,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(next.edges.len(), 1);
    assert_eq!(next.coverage.len(), 1);
    assert_ne!(first.coverage[0].record, next.coverage[0].record);
    assert!(next.next_before.is_none());
    let focused = w
        .app
        .research_graph(ResearchGraphQuery {
            focus: Some("doi:10.1234/old".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(focused.edges.len(), 2);
    assert_eq!(focused.nodes.len(), 3);

    // A full ordinary-record page still offers the first collected page, once.
    let mut submission = w.submit(&owner, "t", false).await;
    let record = RecordId::from("WP-2026-0001");
    submission.claims = submission.extracted.clone();
    submission.status = SubmissionStatus::Accepted {
        record: record.clone(),
    };
    w.app.save(&mut submission).await.unwrap();
    crate::ports::RecordStore::insert(
        w.stores.as_ref(),
        &Record {
            id: record.clone(),
            publication: Some(Box::new(submission.clone())),
            kind: submission.kind,
            submission: submission.id.clone(),
            title: submission.title.clone(),
            authors: submission.authors.clone(),
            basis: AdmissionBasis::EscapeWitness,
            accepted_by: "synthetic-test".into(),
            accepted_at: chrono::Utc::now(),
        },
    )
    .await
    .unwrap();
    let mut before = None;
    let mut seen = BTreeSet::new();
    for expected in [1, 1, 1] {
        let page = w
            .app
            .research_graph(ResearchGraphQuery {
                limit: Some(1),
                before,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.coverage.len(), expected);
        assert!(seen.insert(page.coverage[0].record.clone()));
        before = page.next_before;
    }
    assert!(before.is_none());
    assert_eq!(seen.len(), 3);
    assert!(seen.contains(&record));
}
