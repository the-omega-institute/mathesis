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
