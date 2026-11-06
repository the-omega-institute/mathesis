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

struct KnowledgeReader;
impl PaperReader for KnowledgeReader {
    fn read(&self, bytes: &[u8], filename: &str) -> Result<ReadPaper, String> {
        let mut read = IndexedReader.read(bytes, filename)?;
        let base = read.statements[0].clone();
        let mut lemma = base.clone();
        lemma.kind = ClaimKind::Lemma;
        lemma.display_name = "Lemma".into();
        lemma.latex_label = Some("lem:base".into());
        let mut theorem = base.clone();
        theorem.title = Some("A named consequence".into());
        theorem.latex_label = Some("thm:main".into());
        theorem.references = vec!["lem:base".into(), "eq:unrelated".into(), "duplicate".into()];
        let mut duplicate = base.clone();
        duplicate.latex_label = Some("duplicate".into());
        read.statements = vec![lemma, theorem, duplicate.clone(), duplicate];
        Ok(read)
    }
}

#[tokio::test]
async fn collected_knowledge_has_exact_result_identity_but_no_invented_verification_or_credit() {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores
            .ports(Arc::new(SystemClock), Arc::new(KnowledgeReader)),
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
    let mut saved = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    let original_queue = w.stores.queued().await;
    let knowledge = w.app.collected_knowledge("a-note", 1).await.unwrap();
    assert_eq!(knowledge.work, "a-note");
    assert_eq!(knowledge.version, 1);
    assert_eq!(knowledge.results.statements.len(), 4);
    assert_eq!(
        knowledge.results.statements[1].title.as_deref(),
        Some("A named consequence")
    );
    assert_eq!(
        knowledge.results.connections,
        [CollectedConnection {
            from: "s2".into(),
            to: "s1".into(),
            relation: CollectedRelation::ReferencesInManuscript
        }]
    );
    let public = serde_json::to_string(&knowledge).unwrap();
    assert!(
        !public.contains("private_evidence")
            && !public.contains("verified")
            && !public.contains("source_sha256")
    );
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(w.app.collected_knowledge("a-note", 2).await.is_err());

    // A legacy bibliography index upgrades once, without touching the public delivery or queue.
    saved.id = "legacy-knowledge".into();
    saved.delivery.work_id = "legacy-knowledge".into();
    saved.source_index.as_mut().unwrap().results = None;
    w.stores.insert(&saved).await.unwrap();
    assert!(
        w.app
            .collected_knowledge("legacy-knowledge", 1)
            .await
            .unwrap()
            .results
            .statements
            .is_empty()
    );
    w.app
        .index_collected_paper(&admin, "legacy-knowledge", 1)
        .await
        .unwrap();
    w.app
        .index_collected_paper(&admin, "legacy-knowledge", 1)
        .await
        .unwrap();
    let mut after = w
        .stores
        .get("legacy-knowledge", Some(1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.source_index.as_ref().unwrap().revision, 2);
    after.source_index = saved.source_index.clone();
    assert_eq!(after, saved);
    assert_eq!(w.stores.queued().await, original_queue);

    // Observations of other bytes cannot supply a public statement or connection.
    let mut stale = saved;
    stale.id = "stale-source".into();
    stale.delivery.work_id = "stale-source".into();
    stale.source_index = w
        .stores
        .get("a-note", Some(1))
        .await
        .unwrap()
        .unwrap()
        .source_index;
    stale.source_index.as_mut().unwrap().source_sha256 = "0".repeat(64);
    w.stores.insert(&stale).await.unwrap();
    assert!(
        w.app
            .collected_knowledge("stale-source", 1)
            .await
            .unwrap()
            .results
            .statements
            .is_empty()
    );
}

struct PublishedArchive(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl crate::ports::ArchiveRegistry for PublishedArchive {
    async fn verify(
        &self,
        doi: &str,
        pdf: &BlobRef,
        source: &BlobRef,
    ) -> CoreResult<ArchiveCitation> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (d, p, s) = package(1);
        assert_eq!(pdf.sha256, d.review.pdf_sha256);
        assert_eq!(source.sha256, d.review.source_sha256);
        assert_eq!(pdf.bytes, p.bytes.len() as u64);
        assert_eq!(source.bytes, s.bytes.len() as u64);
        if doi != "10.5281/zenodo.123" {
            return Err(CoreError::invalid("not this published manuscript"));
        }
        Ok(ArchiveCitation {
            doi: doi.into(),
            concept_doi: Some("10.5281/zenodo.122".into()),
        })
    }
}

#[tokio::test]
async fn late_archive_is_owned_exact_version_set_once_and_preserves_delivery_receipt_and_queue() {
    let mut w = World::new().await;
    let registry = Arc::new(PublishedArchive(std::sync::atomic::AtomicUsize::new(0)));
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.archives = registry.clone();
    w.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        20,
        BTreeSet::from(["publisher".into(), "other-publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let other = w.person("other-publisher", &[]).await;
    let reader = w.person("reader", &[]).await;
    let admin = w.person("admin", &[]).await;
    let (d, p, s) = package(1);
    let receipt = w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    let saved = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    let queue = w.stores.queued().await;
    let input = CollectedArchiveInput {
        doi: "10.5281/zenodo.123".into(),
        pdf_sha256: saved.pdf.sha256.clone(),
        source_sha256: saved.source.sha256.clone(),
    };
    for caller in [&other, &reader] {
        assert!(matches!(
            w.app
                .attach_collected_archive(caller, "a-note", 1, input.clone())
                .await,
            Err(CoreError::Forbidden(_))
        ));
    }
    let mut wrong = input.clone();
    wrong.pdf_sha256 = "0".repeat(64);
    assert!(
        w.app
            .attach_collected_archive(&owner, "a-note", 1, wrong)
            .await
            .is_err()
    );
    assert_eq!(registry.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    let mut wrong = input.clone();
    wrong.doi = "10.5281/zenodo.999".into();
    assert!(
        w.app
            .attach_collected_archive(&owner, "a-note", 1, wrong)
            .await
            .is_err()
    );
    assert_eq!(
        w.stores.get("a-note", Some(1)).await.unwrap().unwrap(),
        saved
    );
    let public = w
        .app
        .attach_collected_archive(&owner, "a-note", 1, input.clone())
        .await
        .unwrap();
    assert_eq!(public.archive.as_ref().unwrap().doi, input.doi);
    assert_eq!(
        public.archive.as_ref().unwrap().concept_doi.as_deref(),
        Some("10.5281/zenodo.122")
    );
    let calls = registry.0.load(std::sync::atomic::Ordering::SeqCst);
    w.app
        .attach_collected_archive(&admin, "a-note", 1, input.clone())
        .await
        .unwrap();
    assert_eq!(registry.0.load(std::sync::atomic::Ordering::SeqCst), calls);
    let mut different = input;
    different.doi = "10.5281/zenodo.124".into();
    assert!(
        w.app
            .attach_collected_archive(&admin, "a-note", 1, different)
            .await
            .is_err()
    );
    let mut after = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    after.archive = None;
    assert_eq!(after, saved);
    let (d, p, s) = package(1);
    assert_eq!(
        serde_json::to_value(w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap()).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(w.stores.queued().await, queue);
    let text = serde_json::to_string(&public).unwrap();
    assert!(
        !text.contains("archive_doi")
            && !text.contains("checked_at")
            && !text.contains("private_evidence")
    );
    let (d, p, s) = package(2);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert!(
        w.app
            .collected_paper("a-note", None)
            .await
            .unwrap()
            .archive
            .is_none()
    );
    assert!(
        w.app
            .collected_paper("a-note", Some(1))
            .await
            .unwrap()
            .archive
            .is_some()
    );
}

#[tokio::test]
async fn initial_published_archive_is_checked_before_storage_and_searchable_without_a_second_review()
 {
    let mut w = World::new().await;
    let registry = Arc::new(PublishedArchive(std::sync::atomic::AtomicUsize::new(0)));
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.archives = registry.clone();
    w.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::new(),
        20,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let (mut d, p, s) = package(1);
    // Absent metadata keeps the original serialized request (and historical retry digest).
    let old = serde_json::to_value(&d).unwrap();
    assert!(old["paper"].get("archive_doi").is_none());
    d.paper.archive_doi = Some("10.5281/zenodo.999".into());
    assert!(
        w.app
            .collect_pre_reviewed(&owner, d.clone(), package(1).1, package(1).2)
            .await
            .is_err()
    );
    assert!(w.stores.get("a-note", None).await.unwrap().is_none());
    assert!(w.stores.queued().await.is_empty());
    d.paper.archive_doi = Some("10.5281/zenodo.123".into());
    w.app
        .collect_pre_reviewed(&owner, d.clone(), package(1).1, package(1).2)
        .await
        .unwrap();
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    assert_eq!(registry.0.load(std::sync::atomic::Ordering::SeqCst), 2);
    for doi in ["10.5281/zenodo.123", "10.5281/zenodo.122"] {
        let found = w
            .app
            .collected_papers(
                PaperSearch {
                    q: Some(doi.into()),
                    ..Default::default()
                },
                Some(10),
                None,
            )
            .await
            .unwrap();
        assert_eq!(found.items.len(), 1);
    }
    assert_eq!(
        w.stores.queued().await,
        vec![("collected:a-note:1".into(), JobKind::CollectedExplanation)]
    );
    // A corrupt/stale sidecar must not leak a DOI onto another source version.
    let mut saved = w.stores.get("a-note", None).await.unwrap().unwrap();
    saved.source.sha256 = "0".repeat(64);
    assert!(PublicCollectedPaper::from(&saved).archive.is_none());
}
