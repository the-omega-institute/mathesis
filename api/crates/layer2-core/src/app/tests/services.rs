use super::*;
use crate::ports::BlobStore;

#[tokio::test]
async fn staff_can_retry_a_settled_failed_private_review_without_rewriting_history() {
    fn failed<T>() -> Step<T> {
        Step {
            state: StepState::Failed {
                reason: "Terminal provider queue expiry".into(),
                detail: None,
                retryable: false,
                at: chrono::Utc::now(),
            },
            ..Step::pending()
        }
    }
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let editor = w.person("editor", &[Role::Editor]).await;
    let mut draft = w.submit(&author, "t", false).await;
    draft.purpose = SubmissionPurpose::ReviewOnly;
    draft.analysis_visibility = Visibility::Private;
    w.app.save(&mut draft).await.unwrap();
    let paper = w.review_paper(&author, &draft).await;
    w.app
        .begin_referee_round(&reviewer, &paper.id)
        .await
        .unwrap();
    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            1,
            RoundUpdate::Referee(done_step(referee_report(Some(
                Recommendation::MinorRevision,
            )))),
        )
        .await
        .unwrap();
    let audit = audit_for(
        &paper,
        Recommendation::MinorRevision,
        Correctness::Correct,
        ProofShape::BindOnly,
        None,
    );
    let reviewed = audited(&w, &reviewer, &paper, audit.clone()).await;
    assert_eq!(reviewed.status, SubmissionStatus::ReviewComplete);
    assert!(w.app.restart_referee(&editor, &paper.id).await.is_err());
    for update in [
        RoundUpdate::Advice(failed()),
        RoundUpdate::Letter(failed()),
        RoundUpdate::Formal(Step {
            state: StepState::Skipped {
                reason: "Private review".into(),
            },
            ..Step::pending()
        }),
    ] {
        w.app
            .update_referee_round(&reviewer, &paper.id, 1, update)
            .await
            .unwrap();
    }
    let before = w.app.referee_file(&paper.id).await.unwrap().0;
    assert!(before.current().unwrap().is_settled());
    for caller in [&author, &reviewer] {
        assert!(matches!(
            w.app.restart_referee(caller, &paper.id).await,
            Err(CoreError::Forbidden(_))
        ));
    }
    let retried = w.app.restart_referee(&editor, &paper.id).await.unwrap();
    assert_eq!(retried.rounds.len(), 2);
    assert_eq!(retried.rounds[0], before.rounds[0]);
    assert_eq!(retried.letters, before.letters);
    assert_eq!(
        retried.current().unwrap().recheck_requests,
        ["operational-retry:r1"]
    );
    assert_eq!(
        w.app.submission(&author, &paper.id).await.unwrap(),
        reviewed
    );
    assert!(w.app.restart_referee(&editor, &paper.id).await.is_err());
    assert_eq!(
        w.app
            .begin_referee_round(&reviewer, &paper.id)
            .await
            .unwrap(),
        retried
    );

    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            2,
            RoundUpdate::Referee(done_step(referee_report(Some(
                Recommendation::MinorRevision,
            )))),
        )
        .await
        .unwrap();
    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            2,
            RoundUpdate::Audit(done_step(audit)),
        )
        .await
        .unwrap();
    let reapplied = w
        .app
        .apply_referee_audit(&reviewer, &paper.id, 2)
        .await
        .unwrap();
    assert_eq!(reapplied.status, SubmissionStatus::ReviewComplete);
    assert!(reapplied.decision.is_none());
    assert_eq!(reapplied.versions, reviewed.versions);
    assert_eq!(reapplied.analysis_visibility, Visibility::Private);
    assert!(!reapplied.open_to_contributors);
    assert_eq!(
        &reapplied.reports[..reviewed.reports.len()],
        &reviewed.reports
    );
    assert!(
        w.app
            .ports
            .records
            .for_submission(&paper.id)
            .await
            .unwrap()
            .is_none()
    );
    for update in [
        RoundUpdate::Advice(done_step(Advice {
            summary: "Clarify the hypotheses.".into(),
            improvements: vec![],
            formalization: vec![],
        })),
        RoundUpdate::Letter(done_step(LetterDraft {
            subject: "Private review".into(),
            body: "Please clarify the hypotheses.".into(),
            note: String::new(),
        })),
        RoundUpdate::Formal(Step {
            state: StepState::Skipped {
                reason: "Private review".into(),
            },
            ..Step::pending()
        }),
    ] {
        w.app
            .update_referee_round(&reviewer, &paper.id, 2, update)
            .await
            .unwrap();
    }
    let finished = w.app.referee_file(&paper.id).await.unwrap().0;
    assert_eq!(finished.rounds[0], before.rounds[0]);
    assert_eq!(finished.letters.len(), 1);
    assert_eq!(finished.letters[0].round, Some(2));
    assert!(w.app.restart_referee(&editor, &paper.id).await.is_err());
}

#[tokio::test]
async fn archive_search_filters_before_pagination_and_never_reads_private_revisions() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let mut ids = vec![];
    for n in 1..=5 {
        let mut s = w.submit(&author, "t", false).await;
        s.title = format!("Prime gaps {n}");
        s.abstract_text = "UniquePublicationWord".into();
        s.analysis_visibility = if n % 2 == 0 {
            Visibility::Private
        } else {
            Visibility::Public
        };
        let id = RecordId::format(2026, n);
        s.status = SubmissionStatus::Accepted { record: id.clone() };
        w.app.save(&mut s).await.unwrap();
        w.app
            .ports
            .records
            .insert(&Record {
                kind: s.kind,
                id: id.clone(),
                submission: s.id.clone(),
                title: s.title.clone(),
                authors: s.authors.clone(),
                publication: Some(Box::new(s.clone())),
                basis: AdmissionBasis::EscapeWitness,
                accepted_by: author.person.clone(),
                accepted_at: w.app.ports.clock.now(),
            })
            .await
            .unwrap();
        ids.push((id, s.id));
    }
    let query = PaperSearch {
        q: Some("UniquePublicationWord".into()),
        author: Some("Author".into()),
        msc: Some("11".into()),
        kind: Some(SubmissionKind::Paper),
    };
    let first = w
        .app
        .search_papers(query.clone(), Some(2), None)
        .await
        .unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(|s| s.record.as_str())
            .collect::<Vec<_>>(),
        vec!["WP-2026-0005", "WP-2026-0003"]
    );
    let second = w
        .app
        .search_papers(query, Some(2), first.next_before)
        .await
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].record.as_str(), "WP-2026-0001");
    assert!(second.next_before.is_none());
    let mut revised = w.app.load(&ids[4].1).await.unwrap();
    revised.title = "PrivateRevisionWord".into();
    revised.abstract_text = "PrivateRevisionWord".into();
    w.app.save(&mut revised).await.unwrap();
    assert!(
        w.app
            .search_papers(
                PaperSearch {
                    q: Some("PrivateRevisionWord".into()),
                    ..Default::default()
                },
                None,
                None
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
    w.app
        .set_analysis_visibility(&author, &ids[4].1, Visibility::Private)
        .await
        .unwrap();
    assert!(
        w.app
            .search_papers(
                PaperSearch {
                    q: Some(ids[4].0.to_string()),
                    ..Default::default()
                },
                None,
                None
            )
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        w.app
            .search_papers(
                PaperSearch {
                    q: Some("*".into()),
                    ..Default::default()
                },
                None,
                None
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn portable_review_preserves_provenance_and_author_only_access() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let stranger = w.person("stranger", &[]).await;
    let staff = w.person("reviewer", &[Role::Reviewer]).await;
    let s = w.submit(&author, "t", false).await;
    assert!(matches!(
        w.app.review_report(&stranger, &s.id).await,
        Err(CoreError::NotFound { .. })
    ));
    let report = w.app.review_report(&author, &s.id).await.unwrap();
    assert_eq!(report.version, 1);
    assert_eq!(report.source_sha256, s.versions[0].archive.sha256);
    assert_eq!(report.claims_revision, 0);
    assert_eq!(report.service, "Orchestra");
    let staff_report = w.app.review_report(&staff, &s.id).await.unwrap();
    assert_eq!(
        serde_json::to_value(report).unwrap(),
        serde_json::to_value(staff_report).unwrap()
    );
}

#[tokio::test]
async fn private_review_has_no_publication_and_retains_evidence_through_revisions() {
    let w = World::new().await;
    let author = w.person("review-author", &[]).await;
    let stranger = w.person("stranger", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer, Role::Editor]).await;
    let new: NewPaper = serde_json::from_value(serde_json::json!({
        "purpose":"review_only", "review_scope":"Check the boundary case and the main argument.",
        "processing_consent":AI_PROCESSING_CONSENT_VERSION,
        "ai_disclosure":{"level":"none","statement":"No AI."},
        "make_public_after_acceptance":true
    }))
    .unwrap();
    let draft = w
        .app
        .submit_paper(
            &author,
            new,
            Upload {
                filename: "t.tex".into(),
                bytes: b"source v1".to_vec(),
            },
        )
        .await
        .unwrap();
    assert_eq!(draft.analysis_visibility, Visibility::Private);
    assert!(
        w.app
            .set_open_to_contributors(&author, &draft.id, true)
            .await
            .is_err()
    );
    let paper = w.review_paper(&author, &draft).await;
    assert!(paper.decision.is_none());
    w.app
        .begin_referee_round(&reviewer, &paper.id)
        .await
        .unwrap();
    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            1,
            RoundUpdate::Referee(done_step(referee_report(Some(Recommendation::Reject)))),
        )
        .await
        .unwrap();
    // A known/bind-only result is still a successful review service.
    let mut audit = audit_for(
        &paper,
        Recommendation::MajorRevision,
        Correctness::Gap,
        ProofShape::BindOnly,
        Some("Known theorem".into()),
    );
    audit.concerns.push(AuditedConcern {
        concern: "Boundary hypothesis is missing".into(),
        status: ConcernStatus::Confirmed,
        note: "Include the zero case.".into(),
        evidence: "At n = 0 the left side is 0 and the claimed lower bound is 1.".into(),
        basis: "offline".into(),
        location: "Theorem 1, final inequality".into(),
    });
    let reviewed = audited(&w, &reviewer, &paper, audit).await;
    assert_eq!(reviewed.status, SubmissionStatus::ReviewComplete);
    assert!(reviewed.decision.is_none());
    assert!(
        w.app
            .ports
            .records
            .for_submission(&paper.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(w.app.decide(&reviewer, &paper.id).await.is_err());
    assert!(
        w.app
            .set_analysis_visibility(&author, &paper.id, Visibility::Public)
            .await
            .is_err()
    );
    assert!(matches!(
        w.app.review_report(&stranger, &paper.id).await,
        Err(CoreError::NotFound { .. })
    ));
    for update in [
        RoundUpdate::Advice(done_step(Advice {
            summary: "Fix the zero case.".into(),
            improvements: vec![],
            formalization: vec![],
        })),
        RoundUpdate::Letter(done_step(LetterDraft {
            subject: "Private review".into(),
            body: "Dear Author,\n\nPlease include the boundary case.".into(),
            note: String::new(),
        })),
        RoundUpdate::Formal(Step {
            state: StepState::Skipped {
                reason: "Review-only service".into(),
            },
            ..Step::pending()
        }),
    ] {
        w.app
            .update_referee_round(&reviewer, &paper.id, 1, update)
            .await
            .unwrap();
    }
    let report = w.app.review_report(&author, &paper.id).await.unwrap();
    assert!(report.markdown().contains("left side is 0"));
    assert!(report.markdown().contains(REVIEW_METHOD));
    assert!(report.decision.is_none());
    assert!(report.markdown().contains("Required knowledge queries"));
    assert!(report.markdown().contains("Knowledge relation: Unknown"));
    assert!(report.markdown().contains("Saved response SHA-256"));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["review"]["rounds"][0]["knowledge_contract"], 4);
    assert!(
        json["review"]["rounds"][0]["knowledge_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event"] == "coverage_assessed")
    );
    assert_eq!(report.review.rounds[0].review_contract, REVIEW_CONTRACT);
    assert_eq!(
        report.review.rounds[0].review_method.as_deref(),
        Some(REVIEW_METHOD)
    );
    assert!(
        report.review.rounds[0]
            .audit
            .done()
            .unwrap()
            .claims
            .iter()
            .all(|claim| claim.methodology.is_some())
    );
    assert_eq!(
        report.review.rounds[0].source_sha256,
        paper.current_version().unwrap().archive.sha256
    );
    assert!(
        report.review.letters[0]
            .body
            .contains("private Orchestra review is complete")
    );
    assert!(!report.review.letters[0].body.contains("not accepted"));
    assert!(report.review.rounds[0].advice.is_none());
    let new = NewFindingResponse {
        round: 1,
        concern: 0,
        version: 1,
        claims_revision: paper.claims_revision,
        body: "The zero case is covered by the displayed convention; please check it.".into(),
    };
    assert!(
        w.app
            .respond_to_finding(&stranger, &paper.id, new.clone())
            .await
            .is_err()
    );
    let responded = w
        .app
        .respond_to_finding(&author, &paper.id, new.clone())
        .await
        .unwrap();
    let replay = w
        .app
        .respond_to_finding(&author, &paper.id, new)
        .await
        .unwrap();
    assert_eq!(responded.finding_responses, replay.finding_responses);
    assert_eq!(replay.finding_responses.len(), 1);
    let round2 = w
        .app
        .begin_referee_round(&reviewer, &paper.id)
        .await
        .unwrap();
    assert_eq!(round2.current().unwrap().number, 2);
    assert!(
        w.app
            .recheck_evidence(&reviewer, &paper.id)
            .await
            .unwrap()
            .contains("displayed convention")
    );
    assert!(round2.current().unwrap().audit.done().is_none());
    assert_eq!(round2.current().unwrap().review_contract, REVIEW_CONTRACT);
    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            2,
            RoundUpdate::Referee(round2.rounds[0].referee.clone()),
        )
        .await
        .unwrap();
    let mut recheck = audit_for(
        &paper,
        Recommendation::MinorRevision,
        Correctness::Correct,
        ProofShape::Content,
        None,
    );
    assert!(
        w.app
            .update_referee_round(
                &reviewer,
                &paper.id,
                2,
                RoundUpdate::Audit(done_step(recheck.clone()))
            )
            .await
            .is_err()
    );
    recheck.response_assessments.push(ResponseAssessment {
        response_id: replay.finding_responses[0].id.clone(),
        status: ResponseDisposition::Addressed,
        explanation: "The explicit convention establishes the zero case.".into(),
        evidence: vec![],
    });
    for bad in [
        ResponseAssessment {
            response_id: "invented".into(),
            ..recheck.response_assessments[0].clone()
        },
        ResponseAssessment {
            evidence: vec![ReviewEvidenceReference {
                path: "missing.json".into(),
                sha256: "invented".into(),
            }],
            ..recheck.response_assessments[0].clone()
        },
    ] {
        let mut invalid = recheck.clone();
        invalid.response_assessments = vec![bad];
        assert!(
            w.app
                .update_referee_round(
                    &reviewer,
                    &paper.id,
                    2,
                    RoundUpdate::Audit(done_step(invalid))
                )
                .await
                .is_err()
        );
    }
    w.app
        .update_referee_round(
            &reviewer,
            &paper.id,
            2,
            RoundUpdate::Audit(done_step(recheck)),
        )
        .await
        .unwrap();
    let export = w.app.review_report(&author, &paper.id).await.unwrap();
    assert!(
        export
            .markdown()
            .contains("explicit convention establishes")
    );
    assert!(
        export
            .markdown()
            .contains("Reviewer conclusion on version 1: Addressed")
    );
    assert_eq!(
        export.review.rounds[0].audit.done().unwrap().concerns[0].status,
        ConcernStatus::Confirmed
    );
    w.app.reconcile_private_rechecks().await.unwrap();
    let revised = w
        .app
        .upload_version(
            &author,
            &paper.id,
            Upload {
                filename: "t.tex".into(),
                bytes: b"source v2".to_vec(),
            },
            "Explain zero case".into(),
            Some(AI_PROCESSING_CONSENT_VERSION.into()),
        )
        .await
        .unwrap();
    assert_eq!(revised.status, SubmissionStatus::Draft);
    assert!(
        w.app
            .respond_to_finding(
                &author,
                &paper.id,
                NewFindingResponse {
                    round: 1,
                    concern: 0,
                    version: 1,
                    claims_revision: paper.claims_revision,
                    body: "stale".into()
                }
            )
            .await
            .is_err()
    );
    let report = w.app.review_report(&author, &paper.id).await.unwrap();
    assert_ne!(report.source_sha256, report.review.rounds[0].source_sha256);
    assert!(report.markdown().contains("Author response on version 1"));
}

#[tokio::test]
async fn concurrent_deposit_retries_discard_only_losing_source_writes() {
    struct RacingBlobs {
        store: Arc<MemoryStores>,
        barrier: tokio::sync::Barrier,
        written: std::sync::Mutex<Vec<String>>,
    }
    #[async_trait::async_trait]
    impl BlobStore for RacingBlobs {
        async fn put(&self, bytes: &[u8], content_type: &str) -> CoreResult<BlobRef> {
            let blob = BlobStore::put(self.store.as_ref(), bytes, content_type).await?;
            self.written.lock().unwrap().push(blob.id.clone());
            // All requests allocate both sources before any can win admission.
            self.barrier.wait().await;
            Ok(blob)
        }
        async fn get(&self, id: &str) -> CoreResult<Option<Vec<u8>>> {
            BlobStore::get(self.store.as_ref(), id).await
        }
        async fn discard(&self, id: &str) -> CoreResult<()> {
            BlobStore::discard(self.store.as_ref(), id).await
        }
    }
    let w = World::new().await;
    let caller = w.person("depositor", &[]).await;
    let blobs = Arc::new(RacingBlobs {
        store: w.stores.clone(),
        barrier: tokio::sync::Barrier::new(8),
        written: Default::default(),
    });
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.blobs = blobs.clone();
    let app = App::new(ports, Policy::default(), BTreeSet::new());
    let body: NewLeanDeposit = serde_json::from_value(serde_json::json!({
        "title":"Concurrent source", "authors":["Original author"],
        "statement":"True", "source_reference":{"kind":"url","locator":"https://example.org/proof"},
        "toolchain":"test", "license":"MIT", "solution":"import Helpers\n",
        "modules":{"Helpers":"theorem helper : True := by trivial\n"}
    }))
    .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let (app, caller, body) = (app.clone(), caller.clone(), body.clone());
        tasks.spawn(async move { app.submit_lean_deposit(&caller, body).await.unwrap() });
    }
    let mut deposits = vec![];
    while let Some(result) = tasks.join_next().await {
        deposits.push(result.unwrap());
    }
    assert!(deposits.iter().all(|d| d.id == deposits[0].id));
    let written = blobs.written.lock().unwrap().clone();
    assert_eq!(written.len(), 16);
    let kept = [&deposits[0].source.id, &deposits[0].modules["Helpers"].id];
    for id in written {
        assert_eq!(
            BlobStore::get(w.stores.as_ref(), &id)
                .await
                .unwrap()
                .is_some(),
            kept.contains(&&id),
            "only the winner's main source and supporting module must remain"
        );
    }
    assert_eq!(
        app.lean_deposit_source(&caller, &deposits[0].id)
            .await
            .unwrap(),
        body.solution
    );
    assert_eq!(
        app.lean_deposit_modules(&caller, &deposits[0].id)
            .await
            .unwrap(),
        body.modules
    );
}

#[tokio::test]
async fn deposit_cleanup_preserves_sources_after_an_uncertain_committed_insert() {
    use crate::ports::{LeanDepositStore, Listing};
    struct CommittedError {
        store: Arc<MemoryStores>,
        conflict: bool,
    }
    #[async_trait::async_trait]
    impl LeanDepositStore for CommittedError {
        async fn get(&self, id: &str) -> CoreResult<Option<LeanDeposit>> {
            LeanDepositStore::get(self.store.as_ref(), id).await
        }
        async fn insert(&self, value: &LeanDeposit) -> CoreResult<()> {
            LeanDepositStore::insert(self.store.as_ref(), value).await?;
            if self.conflict {
                Err(CoreError::conflict("insert replay after commit"))
            } else {
                Err(CoreError::Unavailable("response lost after commit".into()))
            }
        }
        async fn replace(&self, value: &LeanDeposit, expected: u64) -> CoreResult<()> {
            LeanDepositStore::replace(self.store.as_ref(), value, expected).await
        }
        async fn list(
            &self,
            owner: &PersonId,
            limit: u32,
            before: Option<String>,
        ) -> CoreResult<Listing<LeanDeposit>> {
            LeanDepositStore::list(self.store.as_ref(), owner, limit, before).await
        }
    }
    for conflict in [false, true] {
        let w = World::new().await;
        let caller = w.person("depositor", &[]).await;
        let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
        ports.deposits = Arc::new(CommittedError {
            store: w.stores.clone(),
            conflict,
        });
        let app = App::new(ports, Policy::default(), BTreeSet::new());
        let body: NewLeanDeposit = serde_json::from_value(serde_json::json!({
            "title":"Committed source", "authors":["Original author"], "statement":"True",
            "source_reference":{"kind":"url","locator":"https://example.org/proof"},
            "toolchain":"test", "license":"MIT", "solution":"import Helpers\n",
            "modules":{"Helpers":"theorem helper : True := by trivial\n"}
        }))
        .unwrap();
        let first = app.submit_lean_deposit(&caller, body.clone()).await;
        assert_eq!(first.is_ok(), conflict);
        let saved = app
            .submit_lean_deposit(&caller, body.clone())
            .await
            .unwrap();
        assert_eq!(
            app.lean_deposit_source(&caller, &saved.id).await.unwrap(),
            body.solution
        );
        assert_eq!(
            app.lean_deposit_modules(&caller, &saved.id).await.unwrap(),
            body.modules
        );
    }
}
