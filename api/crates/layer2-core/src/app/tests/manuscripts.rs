use super::*;
use crate::ports::SubmissionStore;

fn delivery(version: &str, previous: Option<u32>) -> NewManuscriptDelivery {
    serde_json::from_value(serde_json::json!({
        "schema":"mathesis.manuscript-delivery.v1", "producer":"paper", "work_id":"two-related-problems", "version":version, "previous_version":previous,
        "paper":{"kind":"note","purpose":"publication","title":"A short note", "authors":[{"name":"First Author"},{"name":"Second Author"}], "ai_disclosure":{"level":"assisted","statement":"Proofreading assistance."},"processing_consent":"ai-review-2026-10-11"},
        "origin":{"problem_ids":["problem-a","problem-b"],"source_url":"https://example.org/note/v1","license":"CC-BY-4.0","evidence":[{"claim_label":"C1","kind":"written_proof","url":"https://example.org/note/v1.pdf","scope":"Theorem 1 proves the stated special case."}]}
    })).unwrap()
}

fn upload(bytes: &[u8]) -> Upload {
    Upload {
        filename: "t.tex".into(),
        bytes: bytes.to_vec(),
    }
}

fn confirmation(r: &ManuscriptReceipt) -> ConfirmManuscriptDelivery {
    ConfirmManuscriptDelivery {
        version: r.version,
        source_sha256: r.source_sha256.clone(),
        expected_claims_revision: r.expected_claims_revision,
        claims: r
            .extracted
            .iter()
            .map(|c| ClaimConfirmation {
                id: c.id.clone(),
                kind: c.kind,
                role: c.role,
                excluded: false,
                depends_on: vec![],
                depends_on_conjectures: vec![],
                settles: None,
            })
            .collect(),
    }
}

#[tokio::test]
async fn repeated_and_concurrent_deliveries_have_one_identity_and_owner() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (a, b) = tokio::join!(
        w.app
            .deliver_manuscript(&author, delivery("v1", None), upload(b"source")),
        w.app
            .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.submission, b.submission);
    assert_eq!(w.stores.active_count(&author.person).await.unwrap(), 1);
    assert_eq!(a.next_action, "confirm_claims");
    assert!(a.review.is_none() && a.publication.is_none());
    let source = w
        .app
        .paper_file(Some(&author), &a.submission, Some(1), false)
        .await
        .unwrap();
    assert_eq!(source.bytes, b"source");
    let recovered = w
        .app
        .find_manuscript_delivery(
            &author,
            "paper",
            "two-related-problems",
            SubmissionPurpose::Publication,
        )
        .await
        .unwrap();
    assert_eq!(recovered.submission, a.submission);
    let other = w.person("other", &[]).await;
    assert!(matches!(
        w.app.manuscript_receipt(&other, &a.submission, None).await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(matches!(
        w.app
            .find_manuscript_delivery(
                &other,
                "paper",
                "two-related-problems",
                SubmissionPurpose::Publication
            )
            .await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(
        w.app
            .list_papers(None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn reused_version_with_different_content_or_authorship_is_refused() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let a = w
        .app
        .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
        .await
        .unwrap();
    assert!(matches!(
        w.app
            .deliver_manuscript(&author, delivery("v1", None), upload(b"changed"))
            .await,
        Err(CoreError::Conflict(_))
    ));
    let mut changed = delivery("v1", None);
    changed.paper.authors.reverse();
    assert!(matches!(
        w.app
            .deliver_manuscript(&author, changed, upload(b"source"))
            .await,
        Err(CoreError::Conflict(_))
    ));
    let s = w.app.submission(&author, &a.submission).await.unwrap();
    assert_eq!(s.versions.len(), 1);
    assert_eq!(s.authors[0].name, "First Author");
}

#[tokio::test]
async fn confirmed_delivery_starts_once_and_new_version_rejects_stale_confirmations() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let a = w
        .app
        .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
        .await
        .unwrap();
    let input = confirmation(&a);
    let first = w
        .app
        .confirm_manuscript_delivery(&author, &a.submission, input.clone())
        .await
        .unwrap();
    assert_eq!(first.status, "in_review");
    assert!(
        w.stores
            .queued()
            .await
            .contains(&(a.submission.clone(), JobKind::Referee))
    );
    let before = w.app.submission(&author, &a.submission).await.unwrap();
    let again = w
        .app
        .confirm_manuscript_delivery(&author, &a.submission, input.clone())
        .await
        .unwrap();
    assert_eq!(
        again.confirmed_claims_revision,
        first.confirmed_claims_revision
    );
    assert_eq!(
        w.app.submission(&author, &a.submission).await.unwrap(),
        before
    );
    assert!(matches!(
        w.app
            .deliver_manuscript(&author, delivery("v2", None), upload(b"revision"))
            .await,
        Err(CoreError::Conflict(_))
    ));
    let next = w
        .app
        .deliver_manuscript(&author, delivery("v2", Some(1)), upload(b"revision"))
        .await
        .unwrap();
    assert_eq!(next.submission, a.submission);
    assert_eq!(next.version, 2);
    assert!(next.review.is_none());
    assert!(matches!(
        w.app
            .confirm_manuscript_delivery(&author, &a.submission, input)
            .await,
        Err(CoreError::Conflict(_))
    ));
    let old = w
        .app
        .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
        .await
        .unwrap();
    assert!(!old.current);
    assert_eq!(old.version, 1);
    assert_eq!(old.status, "superseded");
    assert_eq!(old.next_action, "none");
    let mut bad = confirmation(&next);
    bad.source_sha256 = a.source_sha256;
    assert!(matches!(
        w.app
            .confirm_manuscript_delivery(&author, &a.submission, bad)
            .await,
        Err(CoreError::Conflict(_))
    ));
}

#[tokio::test]
async fn accepted_origin_is_pinned_and_private_or_revised_inputs_do_not_leak() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let editor = w.person("editor", &[Role::Editor]).await;
    let a = w
        .app
        .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
        .await
        .unwrap();
    w.app
        .confirm_manuscript_delivery(&author, &a.submission, confirmation(&a))
        .await
        .unwrap();
    let p = w.app.submission(&author, &a.submission).await.unwrap();
    let p = w.review_paper(&author, &p).await;
    w.literature(&editor, &p, vec![]).await;
    let (shape, witnesses) = content();
    w.app
        .judge_claim(
            &editor,
            &p.id,
            &"C1".into(),
            shape,
            witnesses,
            "New estimate.".into(),
        )
        .await
        .unwrap();
    w.app.adopt_judgements(&editor, &p.id).await.unwrap();
    let accepted = w.app.decide(&editor, &p.id).await.unwrap();
    let SubmissionStatus::Accepted { record } = accepted.status else {
        panic!("expected acceptance")
    };
    let public = w.app.paper(&record).await.unwrap();
    assert_eq!(public.origin, Some(delivery("v1", None).origin));
    let receipt = w
        .app
        .manuscript_receipt(&author, &p.id, Some(1))
        .await
        .unwrap();
    assert!(receipt.publication.unwrap().public);
    let mut revision = delivery("v2", Some(1));
    revision.origin.source_url = "https://example.org/private-revision".into();
    w.app
        .deliver_manuscript(&author, revision, upload(b"revision"))
        .await
        .unwrap();
    assert_eq!(w.app.paper(&record).await.unwrap().origin, public.origin);
    assert!(
        w.app
            .manuscript_receipt(&author, &p.id, Some(2))
            .await
            .unwrap()
            .publication
            .is_none()
    );
    // Live privacy still protects the pinned public projection.
    let mut s = w.app.submission(&author, &p.id).await.unwrap();
    s.analysis_visibility = Visibility::Private;
    w.app.save(&mut s).await.unwrap();
    assert!(w.app.paper(&record).await.unwrap().origin.is_none());
}

#[tokio::test]
async fn private_review_is_a_distinct_identity_and_requires_real_consent() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let mut private = delivery("v1", None);
    private.paper.purpose = SubmissionPurpose::ReviewOnly;
    let a = w
        .app
        .deliver_manuscript(&author, private.clone(), upload(b"source"))
        .await
        .unwrap();
    let b = w
        .app
        .deliver_manuscript(&author, delivery("v1", None), upload(b"source"))
        .await
        .unwrap();
    assert_ne!(a.submission, b.submission);
    let s = w.app.submission(&author, &a.submission).await.unwrap();
    assert_eq!(s.analysis_visibility, Visibility::Private);
    private.paper.processing_consent = None;
    assert!(matches!(
        w.app
            .deliver_manuscript(&author, private, upload(b"source"))
            .await,
        Err(CoreError::Invalid(_))
    ));
}
