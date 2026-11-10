use super::*;
use crate::ports::{BlobStore, LeasedJob};
use serde_json::json;
fn authorization(p: &FeedbackProposal) -> FeedbackAuthorization {
    FeedbackAuthorization {
        digest: p.digest.clone(),
        license: FEEDBACK_LICENSE.into(),
        consent: FEEDBACK_EXPORT_CONSENT.into(),
        rights_asserted: true,
    }
}
async fn prepared(w: &World, a: &Caller, p: &Submission, kind: FeedbackKind) -> FeedbackView {
    let preview = w
        .app
        .preview_feedback(a, &p.id, kind.clone())
        .await
        .unwrap();
    w.app
        .prepare_feedback(
            a,
            &p.id,
            NewFeedbackProposal {
                kind,
                preview_digest: preview.digest,
                preparation_consent: FEEDBACK_PREPARATION_CONSENT.into(),
            },
        )
        .await
        .unwrap()
}
fn declaration() -> KnowledgeDeclarationRef {
    KnowledgeDeclarationRef {
        snapshot: format!("sha256:{}", "a".repeat(64)),
        module_path: "Fixture.lean".into(),
        declaration_name_key: "Fixture.theorem".into(),
        statement_id: format!("sha256:{}", "b".repeat(64)),
        type_digest: format!("sha256:{}", "c".repeat(64)),
    }
}
async fn record(w: &World, p: &Submission) {
    let SubmissionStatus::Accepted { record } = &p.status else {
        panic!()
    };
    w.app
        .ports
        .records
        .insert(&Record {
            kind: p.kind,
            id: record.clone(),
            publication: None,
            submission: p.id.clone(),
            title: p.title.clone(),
            authors: p.authors.clone(),
            basis: AdmissionBasis::EscapeWitness,
            accepted_by: "fixture-editor".into(),
            accepted_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn feedback_open_problem_requires_exact_preview_separate_consent_and_current_public_source() {
    let w = World::new().await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let other = w.person("outsider", &[]).await;
    let mut p = displayed_conjecture(&w, &a, &r).await;
    let kind = FeedbackKind::OpenProblem { claim: "C1".into() };
    assert!(
        w.app
            .preview_feedback(&other, &p.id, kind.clone())
            .await
            .is_err()
    );
    let preview = w
        .app
        .preview_feedback(&a, &p.id, kind.clone())
        .await
        .unwrap();
    assert!(!preview.payload.to_string().contains("PRIVATE"));
    for (digest, consent) in [
        (preview.digest.clone(), ""),
        ("wrong".into(), FEEDBACK_PREPARATION_CONSENT),
    ] {
        assert!(
            w.app
                .prepare_feedback(
                    &a,
                    &p.id,
                    NewFeedbackProposal {
                        kind: kind.clone(),
                        preview_digest: digest,
                        preparation_consent: consent.into()
                    }
                )
                .await
                .is_err()
        );
    }
    let v = prepared(&w, &a, &p, kind.clone()).await;
    let key = &v.proposal.id;
    assert!(v.proposal.events.is_empty());
    assert_eq!(v.payload, Some(preview.payload.clone()));
    assert!(w.app.feedback(&other, &p.id, key).await.is_err());
    assert!(w.app.feedback_proposals(&other, &p.id).await.is_err());
    assert!(w.app.export_feedback(&a, &p.id, key).await.is_err());
    let mut input = authorization(&v.proposal);
    input.rights_asserted = false;
    assert!(
        w.app
            .authorize_feedback(&a, &p.id, key, input)
            .await
            .is_err()
    );
    w.app
        .authorize_feedback(&a, &p.id, key, authorization(&v.proposal))
        .await
        .unwrap();
    let export = w.app.export_feedback(&a, &p.id, key).await.unwrap();
    assert!(
        export
            .proposal_markdown
            .contains(&format!("Mathesis-Proposal: {}", v.proposal.digest))
    );
    assert_eq!(export.payload, preview.payload);
    assert_eq!(export.authorizations.len(), 1);
    w.app.export_feedback(&a, &p.id, key).await.unwrap();
    assert_eq!(
        w.app
            .feedback(&a, &p.id, key)
            .await
            .unwrap()
            .proposal
            .events
            .len(),
        2
    );
    p.analysis_visibility = Visibility::Private;
    w.app.save(&mut p).await.unwrap();
    assert!(w.app.export_feedback(&a, &p.id, key).await.is_err());
    let view = w.app.feedback(&a, &p.id, key).await.unwrap();
    assert_eq!(view.state, "source_or_evidence_changed");
    assert!(view.payload.is_none());
}
#[tokio::test]
async fn feedback_verified_solution_requires_both_owners_and_pins_target_receipt_and_slots() {
    let w = solving::world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (p, record) = solving::target(&w, &a, &r).await;
    let attempt = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            "PRIVATE note".into(),
        )
        .await
        .unwrap();
    let kind = FeedbackKind::VerifiedSolution {
        claim: "C1".into(),
        attempt: attempt.id.clone(),
    };
    assert!(
        w.app
            .preview_feedback(&a, &p.id, kind.clone())
            .await
            .is_err()
    );
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    let (mut file, original) = w.app.find_attempt(&attempt.id).await.unwrap();
    let corrupt = BlobStore::put(w.stores.as_ref(), b"changed proof bytes", "text/plain")
        .await
        .unwrap();
    file.attempts
        .iter_mut()
        .find(|a| a.id == attempt.id)
        .unwrap()
        .solution = corrupt;
    w.app.save_solve_file(&mut file).await.unwrap();
    assert!(matches!(
        w.app.preview_feedback(&a, &p.id, kind.clone()).await,
        Err(CoreError::Unavailable(_))
    ));
    file.attempts
        .iter_mut()
        .find(|a| a.id == attempt.id)
        .unwrap()
        .solution = original.solution;
    w.app.save_solve_file(&mut file).await.unwrap();
    let v = prepared(&w, &a, &p, kind).await;
    let key = &v.proposal.id;
    assert_eq!(
        v.proposal.required_signers,
        vec![a.person.clone(), solver.person.clone()]
    );
    assert!(
        v.payload.as_ref().unwrap()["evidence"]["verification_bundle"]["attempt"]
            .get("also_verified")
            .is_none()
    );
    assert!(!v.payload.unwrap().to_string().contains("PRIVATE"));
    let (first, second) = tokio::join!(
        w.app
            .authorize_feedback(&a, &p.id, key, authorization(&v.proposal)),
        w.app
            .authorize_feedback(&solver, &p.id, key, authorization(&v.proposal))
    );
    first.unwrap();
    second.unwrap();
    assert_eq!(
        w.app
            .export_feedback(&solver, &p.id, key)
            .await
            .unwrap()
            .authorizations
            .len(),
        2
    );
    let slots = FourSlotProposal {
        escape_from: "Existing assumptions".into(),
        readout_via: "Exact target observation".into(),
        realization: "Saved verified solution".into(),
        escape_continues: "Proposed subsequent use".into(),
    };
    let dtr = prepared(
        &w,
        &a,
        &p,
        FeedbackKind::DtrRegistration {
            claim: "C1".into(),
            attempt: attempt.id,
            slots,
        },
    )
    .await;
    assert_ne!(dtr.proposal.digest, v.proposal.digest);
    assert!(!dtr.proposal.authorized());
    assert!(
        dtr.payload.unwrap()["verification_scope"]
            .as_str()
            .unwrap()
            .contains("proposed registration")
    );
    w.app
        .revoke_feedback(&solver, &p.id, key, "Revoke future export".into())
        .await
        .unwrap();
    assert!(w.app.export_feedback(&a, &p.id, key).await.is_err());
    assert!(w.app.feedback(&solver, &p.id, key).await.is_err());
    assert!(
        w.app
            .feedback(&a, &p.id, key)
            .await
            .unwrap()
            .payload
            .is_none()
    );
}
#[tokio::test]
async fn feedback_binding_license_does_not_supply_export_consent_and_new_inspection_invalidates_package()
 {
    let (w, a, r, p, b, c) = bindings::fixture().await;
    record(&w, &p).await;
    let contributor = w.person("contributor", &[]).await;
    let b = w
        .app
        .contribute_binding(&contributor, &p.id, &b.id, c)
        .await
        .unwrap();
    let contribution = b.contributions[0].id.clone();
    let kind = FeedbackKind::LibraryBinding {
        board: b.id.clone(),
        contribution: contribution.clone(),
    };
    assert!(
        w.app
            .preview_feedback(&a, &p.id, kind.clone())
            .await
            .is_err()
    );
    bindings::audit(&w, &r, &p, BindingVerdict::Supported).await;
    let v = prepared(&w, &a, &p, kind).await;
    assert!(!v.proposal.authorized());
    let payload = v.payload.unwrap();
    assert_eq!(payload["evidence"]["summary_license"], "CC-BY-4.0");
    assert_eq!(payload["requested_license"], "Apache-2.0");
    assert!(!payload.to_string().contains("PRIVATE"));
    w.app
        .authorize_feedback(&a, &p.id, &v.proposal.id, authorization(&v.proposal))
        .await
        .unwrap();
    assert!(
        w.app
            .export_feedback(&a, &p.id, &v.proposal.id)
            .await
            .is_err()
    );
    w.app
        .authorize_feedback(
            &contributor,
            &p.id,
            &v.proposal.id,
            authorization(&v.proposal),
        )
        .await
        .unwrap();
    w.app
        .export_feedback(&a, &p.id, &v.proposal.id)
        .await
        .unwrap();
    w.app
        .challenge_binding(
            &a,
            &p.id,
            &b.id,
            NewBindingChallenge {
                contribution,
                evidence: "A concrete counterargument".into(),
                license: BINDING_LICENSE.into(),
                consent: BINDING_CONTRIBUTION_CONSENT.into(),
            },
        )
        .await
        .unwrap();
    // A pending accusation alone changes neither support nor issued authorization.
    w.app
        .export_feedback(&a, &p.id, &v.proposal.id)
        .await
        .unwrap();
    bindings::audit(&w, &r, &p, BindingVerdict::Rejected).await;
    assert!(
        w.app
            .export_feedback(&a, &p.id, &v.proposal.id)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn feedback_witness_and_erratum_preserve_public_scope_and_explicit_subjects() {
    let w = World::new().await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let reporter = w.person("reporter", &[]).await;
    let paper = w.submit(&a, "t", false).await;
    let paper = w.review_paper(&a, &paper).await;
    w.app.begin_referee_round(&r, &paper.id).await.unwrap();
    w.app
        .update_referee_round(
            &r,
            &paper.id,
            1,
            RoundUpdate::Referee(done_step(referee_report(Some(Recommendation::Accept)))),
        )
        .await
        .unwrap();
    let audit = audit_for(
        &paper,
        Recommendation::Accept,
        Correctness::Correct,
        ProofShape::Content,
        None,
    );
    let p = audited(&w, &r, &paper, audit).await;
    let kind = FeedbackKind::WitnessFormalization {
        claim: "C1".into(),
        witness: 0,
    };
    let v = prepared(&w, &a, &p, kind).await;
    assert!(
        v.payload.unwrap()["verification_scope"]
            .as_str()
            .unwrap()
            .contains("no proof receipt")
    );
    assert!(
        w.app
            .preview_feedback(
                &a,
                &p.id,
                FeedbackKind::WitnessFormalization {
                    claim: "C1".into(),
                    witness: 100
                }
            )
            .await
            .is_err()
    );
    let SubmissionStatus::Accepted { record } = p.status.clone() else {
        panic!()
    };
    let report = w
        .app
        .file_error_report(
            &reporter,
            &record,
            NewErrorReport {
                subject: ErrorSubject::Statement { claim: "C1".into() },
                text: "Explicit public error evidence".into(),
                solution: None,
                as_agent: None,
            },
        )
        .await
        .unwrap();
    let kind = FeedbackKind::Erratum {
        report: report.id.clone(),
        upstream: vec![declaration()],
    };
    assert!(
        w.app
            .preview_feedback(&a, &p.id, kind.clone())
            .await
            .is_err()
    );
    w.app
        .resolve_error_report(
            &r,
            &report.id,
            ErrorVerdict::Confirmed,
            "Confirmed fixture only".into(),
            "Checked fixture evidence".into(),
        )
        .await
        .unwrap();
    let v = prepared(&w, &a, &p, kind).await;
    assert_eq!(v.proposal.required_signers, vec![a.person, reporter.person]);
    assert!(
        v.payload.unwrap()["evidence"]["confirmed_report"]
            .get("solution")
            .is_none()
    );
}
#[tokio::test]
async fn feedback_concurrent_preparation_is_idempotent_and_payload_corruption_blocks_serving() {
    let w = World::new().await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let p = displayed_conjecture(&w, &a, &r).await;
    let kind = FeedbackKind::OpenProblem { claim: "C1".into() };
    let (v, z) = tokio::join!(
        prepared(&w, &a, &p, kind.clone()),
        prepared(&w, &a, &p, kind)
    );
    assert_eq!(v.proposal.id, z.proposal.id);
    assert_eq!(w.app.feedback_proposals(&a, &p.id).await.unwrap().len(), 1);
    w.app
        .authorize_feedback(&a, &p.id, &v.proposal.id, authorization(&v.proposal))
        .await
        .unwrap();
    BlobStore::discard(w.stores.as_ref(), &v.proposal.payload.id)
        .await
        .unwrap();
    assert!(w.app.feedback(&a, &p.id, &v.proposal.id).await.is_err());
    assert!(
        w.app
            .export_feedback(&a, &p.id, &v.proposal.id)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn feedback_readback_is_durable_fenced_and_never_established_by_the_request() {
    let w = World::new().await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let p = displayed_conjecture(&w, &a, &r).await;
    let v = prepared(&w, &a, &p, FeedbackKind::OpenProblem { claim: "C1".into() }).await;
    let request = FeedbackReadbackRequest {
        pull_request: 9,
        index_revision: format!("sha256:{}", "d".repeat(64)),
        declarations: vec![declaration()],
        observation_key: "first".into(),
    };
    assert!(
        w.app
            .request_feedback_readback(&a, &p.id, &v.proposal.id, request.clone())
            .await
            .is_err()
    );
    w.app
        .authorize_feedback(&a, &p.id, &v.proposal.id, authorization(&v.proposal))
        .await
        .unwrap();
    w.app
        .export_feedback(&a, &p.id, &v.proposal.id)
        .await
        .unwrap();
    w.app
        .request_feedback_readback(&a, &p.id, &v.proposal.id, request.clone())
        .await
        .unwrap();
    w.app
        .request_feedback_readback(&a, &p.id, &v.proposal.id, request.clone())
        .await
        .unwrap();
    let mut changed = request.clone();
    changed.pull_request = 10;
    assert!(
        w.app
            .request_feedback_readback(&a, &p.id, &v.proposal.id, changed)
            .await
            .is_err()
    );
    assert!(
        w.stores
            .queued()
            .await
            .contains(&(p.id.clone(), JobKind::FeedbackReadback))
    );
    let pending = w
        .app
        .pending_feedback_readback(&r, &p.id)
        .await
        .unwrap()
        .unwrap();
    let job = LeasedJob {
        submission: p.id.clone(),
        kind: JobKind::FeedbackReadback,
        attempts: 1,
        lease: "fixture".into(),
    };
    let observation = FeedbackObservation {
        request: pending.request_id.clone(),
        pull_request: 9,
        upstream_repository: "the-omega-institute/trureturing".into(),
        upstream_base: "dev".into(),
        reference_state: "open".into(),
        merge_commit: None,
        merged_at: None,
        consumed_source_commit: "a".repeat(40),
        consumed_source_tree: "b".repeat(40),
        snapshot: declaration().snapshot,
        index_revision: request.index_revision,
        declarations: vec![FeedbackDeclarationObservation {
            identity: declaration(),
            present: false,
            source_sha256: None,
            freeze_status: None,
            frozen_node_id: None,
            package_marker_present: false,
        }],
        merge_in_consumed_source: false,
        evidence: BlobRef {
            id: String::new(),
            bytes: 0,
            sha256: String::new(),
        },
        observed_at: chrono::Utc::now(),
        limits: vec!["Synthetic observation; no acceptance".into()],
    };
    assert_eq!(observation.reference_state, "open");
    assert!(!observation.merge_in_consumed_source);
    let result = || InspectedFeedback {
        observation: observation.clone(),
        evidence: serde_json::to_vec(
            &json!({"fixture":"actual transport adapter tested separately"}),
        )
        .unwrap(),
    };
    assert!(
        w.app
            .record_feedback_observation(&r, &job, &TestAgentLease(false), &pending, result())
            .await
            .is_err()
    );
    w.app
        .record_feedback_observation(&r, &job, &TestAgentLease(true), &pending, result())
        .await
        .unwrap();
    w.app
        .record_feedback_observation(&r, &job, &TestAgentLease(true), &pending, result())
        .await
        .unwrap();
    let evidence = w
        .app
        .feedback_observation_evidence(&a, &p.id, &v.proposal.id, &pending.request_id)
        .await
        .unwrap();
    assert!(evidence.get("fixture").is_some());
    assert!(
        w.app
            .pending_feedback_readback(&r, &p.id)
            .await
            .unwrap()
            .is_none()
    );
    let view = w.app.feedback(&a, &p.id, &v.proposal.id).await.unwrap();
    assert_eq!(
        view.proposal
            .events
            .iter()
            .filter(|e| matches!(e, FeedbackEvent::Observed { .. }))
            .count(),
        1
    );
    let mut request = pending.request.clone();
    request.observation_key = "later".into();
    w.app
        .request_feedback_readback(&a, &p.id, &v.proposal.id, request)
        .await
        .unwrap();
    let pending = w
        .app
        .pending_feedback_readback(&r, &p.id)
        .await
        .unwrap()
        .unwrap();
    w.app
        .revoke_feedback(&a, &p.id, &v.proposal.id, "Stop new work".into())
        .await
        .unwrap();
    assert!(
        w.app
            .pending_feedback_readback(&r, &p.id)
            .await
            .unwrap()
            .is_none()
    );
    let mut late = result();
    late.observation.request = pending.request_id.clone();
    assert!(
        w.app
            .record_feedback_observation(&r, &job, &TestAgentLease(true), &pending, late)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn feedback_payload_keeps_stored_author_names_while_readers_see_the_canonical_form() {
    let stores = Arc::new(MemoryStores::default());
    let w = World {
        app: App::new(
            stores.ports(Arc::new(SystemClock), Arc::new(FamilyFirstReader)),
            Policy::default(),
            BTreeSet::from([PersonId::from("admin")]),
        ),
        stores,
    };
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let p = displayed_conjecture(&w, &a, &r).await;
    let p = w.app.load(&p.id).await.unwrap();
    let SubmissionStatus::Accepted { record } = &p.status else {
        panic!()
    };
    assert_eq!(p.authors[0].name, "Ma, Haobo");
    let shown = w.app.conjecture(record, &"C1".into()).await.unwrap();
    assert_eq!(shown.summary.authors[0].name, "Haobo Ma");
    let listed = w.app.list_conjectures(None, None).await.unwrap();
    assert_eq!(listed.items[0].authors[0].name, "Haobo Ma");
    // The exact payload, and so its digest, carries the attribution as stored.
    let kind = FeedbackKind::OpenProblem { claim: "C1".into() };
    let preview = w
        .app
        .preview_feedback(&a, &p.id, kind.clone())
        .await
        .unwrap();
    assert_eq!(
        preview.payload["attribution"]["authors"][0]["name"],
        "Ma, Haobo"
    );
    assert_eq!(
        preview.payload["evidence"]["problem"]["summary"]["authors"][0]["name"],
        "Ma, Haobo"
    );
    let v = prepared(&w, &a, &p, kind).await;
    assert_eq!(v.proposal.digest, preview.digest);
    assert_eq!(v.payload, Some(preview.payload));
}
