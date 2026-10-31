use super::*;
use crate::ports::{RecordStore, Verifier};
use async_trait::async_trait;
use chrono::{Duration, Utc};
struct FakeVerifier;
#[async_trait]
impl Verifier for FakeVerifier {
    async fn verify(&self, r: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        if r.solution.contains("earlier") {
            tokio::task::yield_now().await;
        }
        Ok(VerificationReceipt {
            verdict: if r.solution.contains("reject") {
                Verdict::Rejected
            } else if r.solution.contains("disproof") {
                Verdict::Disproved
            } else {
                Verdict::Proved
            },
            reason: "independent fake Lean boundary".into(),
            target_digest: r.target_digest.clone(),
            solution_digest: lean_solution_digest(&r.solution, &r.modules),
            toolchain: r.toolchain.clone(),
            axioms: vec!["propext".into()],
            checked_at: Utc::now()
                - if r.solution.contains("earlier") {
                    Duration::seconds(10)
                } else {
                    Duration::zero()
                },
            duration: 1,
        })
    }
}
use sha2::Digest;
pub(super) async fn world(limit: usize) -> World {
    let stores = Arc::new(MemoryStores::default());
    let mut ports = stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.verifier = Arc::new(FakeVerifier);
    World {
        app: App::with_attempt_limit(
            ports,
            Policy::default(),
            BTreeSet::from([PersonId::from("admin")]),
            limit,
        ),
        stores,
    }
}
pub(super) async fn target(
    w: &World,
    author: &Caller,
    reviewer: &Caller,
) -> (Submission, RecordId) {
    let paper = displayed_conjecture(w, author, reviewer).await;
    let s = w
        .app
        .record_lean_statement(
            reviewer,
            &paper.id,
            1,
            paper.claims_revision,
            &"C1".into(),
            "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
            "test".into(),
            "True".into(),
            None,
            crate::model::FaithfulnessCheck {
                verdict: crate::model::FaithfulnessVerdict::Faithful,
                reason: "Definitions and quantifiers agree.".into(),
                mismatch: String::new(),
                regenerated: false,
            },
        )
        .await
        .unwrap();
    w.app
        .respond_lean_statement(
            author,
            AuthenticationMethod::CookieSession,
            &paper.id,
            &s.lean_statements[0].digest,
            true,
            String::new(),
        )
        .await
        .unwrap();
    let SubmissionStatus::Accepted { record } = &paper.status else {
        panic!()
    };
    (paper.clone(), record.clone())
}
#[tokio::test]
async fn attempts_are_private_idempotent_rate_limited_and_become_public_only_after_verification() {
    let w = world(2).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (_, record) = target(&w, &a, &r).await;
    let first = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            "PRIVATE_NOTE".into(),
        )
        .await
        .unwrap();
    assert_eq!(first.state, "queued");
    assert!(w.app.attempt(None, &first.id).await.is_err());
    assert!(w.app.attempt(Some(&a), &first.id).await.is_err());
    assert_eq!(
        w.app
            .attempt(Some(&r), &first.id)
            .await
            .unwrap()
            .note
            .as_deref(),
        Some("PRIVATE_NOTE")
    );
    let same = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    assert_eq!(first.id, same.id);
    let bad = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "reject".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    assert!(
        w.app
            .submit_attempt(
                &solver,
                &record,
                &"C1".into(),
                "third".into(),
                None,
                String::new()
            )
            .await
            .is_err()
    );
    assert!(w.app.verify_attempt(&solver, &first.id).await.is_err());
    w.app.verify_attempt(&r, &bad.id).await.unwrap();
    assert!(w.app.attempt(None, &bad.id).await.is_err());
    w.app.verify_attempt(&r, &first.id).await.unwrap();
    w.app.verify_attempt(&r, &first.id).await.unwrap();
    let public = serde_json::to_string(&w.app.attempt(None, &first.id).await.unwrap()).unwrap();
    assert!(!public.contains("PRIVATE_NOTE"));
    assert_eq!(
        w.app
            .my_attempts(&solver, &record, &"C1".into())
            .await
            .unwrap()
            .len(),
        2
    );
    let board = w.app.leaderboard("all", "all").await.unwrap();
    assert_eq!(board[0].score, 1);
    assert_eq!(board[0].solved, 1);
    assert_eq!(w.app.solve_notifications(&a).await.unwrap().len(), 1);
    assert!(w.app.solve_notifications(&solver).await.unwrap().is_empty());
    assert!(
        w.app
            .submit_attempt(
                &solver,
                &record,
                &"C1".into(),
                "x".repeat(1_048_577),
                None,
                String::new()
            )
            .await
            .is_err()
    );
    assert!(
        w.app
            .submit_attempt(
                &solver,
                &record,
                &"C1".into(),
                "x".into(),
                None,
                "x".repeat(2001)
            )
            .await
            .is_err()
    );
}
#[tokio::test]
async fn concurrent_verifications_award_one_point_to_the_earliest_verifier_timestamp() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let s1 = w.person("solver-one", &[]).await;
    let s2 = w.person("solver-two", &[]).await;
    let (_, record) = target(&w, &a, &r).await;
    let one = w
        .app
        .submit_attempt(
            &s1,
            &record,
            &"C1".into(),
            "later proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    let two = w
        .app
        .submit_attempt(
            &s2,
            &record,
            &"C1".into(),
            "earlier proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    let (v1, v2) = tokio::join!(
        w.app.verify_attempt(&r, &one.id),
        w.app.verify_attempt(&r, &two.id)
    );
    v1.unwrap();
    v2.unwrap();
    let board = w.app.leaderboard("all", "all").await.unwrap();
    assert_eq!(board.len(), 1);
    assert_eq!(board[0].entrant.id, s2.person.as_str());
    assert_eq!(board[0].score, 1);
    let public = w.app.public_attempts(&record, &"C1".into()).await.unwrap();
    assert_eq!(public.len(), 2);
    assert_eq!(public.iter().filter(|a| !a.also_verified).count(), 1);
    assert_eq!(public[0].id, two.id);
    assert_eq!(
        w.app
            .entrant_profile(s2.person.as_str())
            .await
            .unwrap()
            .solutions
            .len(),
        1
    );
}
#[tokio::test]
async fn agents_are_owned_unique_renameable_retireable_and_share_one_board() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let owner = w.person("owner", &[]).await;
    let other = w.person("other", &[]).await;
    let (_, record) = target(&w, &a, &r).await;
    let agent = w
        .app
        .create_agent(&owner, "Proof agent".into())
        .await
        .unwrap();
    assert!(
        w.app
            .create_agent(&other, "Proof agent".into())
            .await
            .is_err()
    );
    assert!(w.app.create_agent(&owner, "x".repeat(41)).await.is_err());
    assert!(
        w.app
            .create_agent(&owner, "wishpool-agent".into())
            .await
            .is_err()
    );
    assert!(
        w.app
            .update_agent(&owner, &agent.id, Some("Mathesis-agent".into()), false)
            .await
            .is_err()
    );
    assert!(
        w.app
            .submit_attempt(
                &other,
                &record,
                &"C1".into(),
                "disproof".into(),
                Some(agent.id.clone()),
                String::new()
            )
            .await
            .is_err()
    );
    assert!(
        w.app
            .update_agent(&other, &agent.id, Some("stolen".into()), false)
            .await
            .is_err()
    );
    let agent = w
        .app
        .update_agent(&owner, &agent.id, Some("Named agent".into()), false)
        .await
        .unwrap();
    let attempt = w
        .app
        .submit_attempt(
            &owner,
            &record,
            &"C1".into(),
            "disproof".into(),
            Some(agent.name.clone()),
            String::new(),
        )
        .await
        .unwrap();
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    assert_eq!(w.app.leaderboard("all", "all").await.unwrap().len(), 1);
    assert!(w.app.leaderboard("all", "people").await.unwrap().is_empty());
    let agents = w.app.leaderboard("month", "agents").await.unwrap();
    assert_eq!(agents[0].disproved, 1);
    assert_eq!(agents[0].entrant.owner.as_ref().unwrap().id, owner.person);
    w.app
        .update_agent(&owner, &agent.id, None, true)
        .await
        .unwrap();
    assert!(
        w.app
            .submit_attempt(
                &owner,
                &record,
                &"C1".into(),
                "another".into(),
                Some(agent.id.clone()),
                String::new()
            )
            .await
            .is_err()
    );
    assert_eq!(
        w.app
            .entrant_profile(&agent.id)
            .await
            .unwrap()
            .solutions
            .len(),
        1
    );
    assert!(w.app.leaderboard("week", "all").await.is_err());
    assert!(w.app.leaderboard("all", "unknown").await.is_err());
}
#[tokio::test]
async fn visibility_and_versions_hide_solutions_and_remove_their_credit() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (paper, record) = target(&w, &a, &r).await;
    let attempt = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    w.app
        .set_analysis_visibility(&a, &paper.id, Visibility::Private)
        .await
        .unwrap();
    assert!(
        w.app
            .list_conjectures(None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(w.app.attempt(None, &attempt.id).await.is_err());
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), &attempt.id)
            .await
            .is_err()
    );
    assert!(w.app.leaderboard("all", "all").await.unwrap().is_empty());
    w.app
        .set_analysis_visibility(&a, &paper.id, Visibility::Public)
        .await
        .unwrap();
    assert_eq!(w.app.leaderboard("all", "all").await.unwrap().len(), 1);
    w.app
        .upload_version(
            &a,
            &paper.id,
            Upload {
                filename: "c.tex".into(),
                bytes: b"new".to_vec(),
            },
            String::new(),
            Some("ai-review-2026-10-11".into()),
        )
        .await
        .unwrap();
    assert!(w.app.target(&record, &"C1".into()).await.is_err());
    assert!(w.app.attempt(None, &attempt.id).await.is_err());
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), &attempt.id)
            .await
            .is_err()
    );
    assert!(w.app.leaderboard("all", "all").await.unwrap().is_empty());
}
#[tokio::test]
async fn paper_problem_policy_excludes_private_settled_and_failed_audit_candidates() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let original = displayed_conjecture(&w, &a, &r).await;
    let original_record = w
        .stores
        .for_submission(&original.id)
        .await
        .unwrap()
        .unwrap();
    let mut paper = original.clone();
    paper.id = "paper-problems".into();
    paper.kind = SubmissionKind::Paper;
    let record: RecordId = "WP-2026-0500".into();
    paper.status = SubmissionStatus::Accepted {
        record: record.clone(),
    };
    let mut settled = paper.claims[0].clone();
    settled.id = "C2".into();
    settled.has_proof = true;
    paper.claims.push(settled);
    w.app.ports.submissions.insert(&paper).await.unwrap();
    let mut rec = original_record;
    rec.id = record.clone();
    rec.submission = paper.id.clone();
    rec.kind = SubmissionKind::Paper;
    rec.publication = Some(Box::new(paper.clone()));
    RecordStore::insert(&*w.stores, &rec).await.unwrap();
    let files = w.app.open_problem_files(&r, &paper.id).await.unwrap();
    assert_eq!(files.len(), 1);
    assert!(!files[0].admitted);
    assert!(
        !w.app
            .list_conjectures(None, None)
            .await
            .unwrap()
            .items
            .iter()
            .any(|c| c.record == record)
    );
    let audit = |reading| RefereeAudit {
        response_assessments: vec![],
        verdict: Recommendation::Accept,
        agrees_with_referee: true,
        summary: "PRIVATE_AUDIT".into(),
        claims: vec![AuditedClaim {
            coverage: None,
            methodology: Some(method_for(ProofShape::Content, true)),
            claim: "C1".into(),
            conjecture: Some(reading),
            correctness: Correctness::NotChecked,
            comment: "PRIVATE_COMMENT".into(),
            shape: None,
            witnesses: vec![],
            known: None,
            referee_agreed: true,
        }],
        concerns: vec![],
    };
    let mut failed = files[0].clone();
    let mut reading = open_reading();
    reading.well_posed = false;
    failed.audit = Some(audit(reading));
    w.app.record_problem_review(&r, failed).await.unwrap();
    assert!(!w.app.open_problem_files(&r, &paper.id).await.unwrap()[0].admitted);
    // A separate revision fences a stale old audit, and rechecks fresh inputs.
    let mut current = w.app.load(&paper.id).await.unwrap();
    current.claims_revision += 1;
    w.app.save(&mut current).await.unwrap();
    assert!(
        w.app
            .record_problem_review(&r, files[0].clone())
            .await
            .is_err()
    );
    let mut passed = w
        .app
        .open_problem_files(&r, &paper.id)
        .await
        .unwrap()
        .remove(0);
    passed.audit = Some(audit(open_reading()));
    w.app.record_problem_review(&r, passed).await.unwrap();
    let listed = w.app.list_conjectures(None, None).await.unwrap();
    let c = listed.items.iter().find(|c| c.record == record).unwrap();
    assert_eq!(c.source, "from WP-2026-0500");
    assert_eq!(c.claim.as_str(), "C1");
    let payload = serde_json::to_string(&listed).unwrap();
    assert!(!payload.contains("PRIVATE_"));
    w.app
        .set_analysis_visibility(&a, &paper.id, Visibility::Private)
        .await
        .unwrap();
    assert!(
        w.app
            .open_problem_files(&r, &paper.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !w.app
            .list_conjectures(None, None)
            .await
            .unwrap()
            .items
            .iter()
            .any(|c| c.record == record)
    );
}
#[tokio::test]
async fn unconfirmed_targets_reject_attempts_and_legacy_conversion_requires_new_confirmation() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let paper = displayed_conjecture(&w, &a, &r).await;
    let SubmissionStatus::Accepted { record } = &paper.status else {
        panic!()
    };
    assert!(
        w.app
            .submit_attempt(
                &a,
                record,
                &"C1".into(),
                "proof".into(),
                None,
                String::new()
            )
            .await
            .is_err()
    );
    let mut current = w.app.load(&paper.id).await.unwrap();
    let legacy = "import Mathlib\ntheorem wishpool_target : True := by sorry";
    let old = format!("{:x}", sha2::Sha256::digest(legacy.as_bytes()));
    current.lean_statements.push(LeanStatementAttempt {
        faithfulness: None,
        claim: "C1".into(),
        version: 1,
        claims_revision: current.claims_revision,
        lean: legacy.into(),
        digest: old.clone(),
        toolchain: "old".into(),
        reading: "True".into(),
        response: LeanStatementResponse::Confirmed {
            author: a.person.clone(),
            at: Utc::now(),
        },
        created_at: Utc::now(),
    });
    w.app.save(&mut current).await.unwrap();
    w.app
        .replace_legacy_target(
            &r,
            &paper.id,
            &old,
            Some((
                "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
                "test".into(),
            )),
        )
        .await
        .unwrap();
    assert!(w.app.target(record, &"C1".into()).await.is_err());
    let current = w.app.load(&paper.id).await.unwrap();
    assert_ne!(current.lean_statements[0].digest, old);
    assert_eq!(
        current.lean_statements[0].response,
        LeanStatementResponse::AwaitingAuthor
    );
    assert!(
        w.app
            .respond_lean_statement(
                &a,
                AuthenticationMethod::CookieSession,
                &paper.id,
                &old,
                true,
                String::new()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn superseded_pending_attempts_are_terminal_private_and_not_requeued() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (paper, record) = target(&w, &a, &r).await;
    let attempt = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    while w.stores.take_job().await.is_some() {}
    w.app
        .upload_version(
            &a,
            &paper.id,
            Upload {
                filename: "c.tex".into(),
                bytes: b"new version".to_vec(),
            },
            String::new(),
            Some("ai-review-2026-10-11".into()),
        )
        .await
        .unwrap();
    for _ in 0..2 {
        w.app.reconcile_attempts().await.unwrap();
    }
    assert!(
        !w.stores
            .queued()
            .await
            .iter()
            .any(|(_, kind)| *kind == JobKind::VerifyAttempt)
    );
    let private = w.app.attempt(Some(&solver), &attempt.id).await.unwrap();
    assert_eq!(private.state, "superseded");
    assert!(private.reason.unwrap().contains("current target"));
    assert!(private.receipt.is_none());
    assert!(w.app.attempt(None, &attempt.id).await.is_err());
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), &attempt.id)
            .await
            .is_err()
    );
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    assert!(w.app.leaderboard("all", "all").await.unwrap().is_empty());
}

#[tokio::test]
async fn leaderboard_ranks_by_score_then_earlier_last_solve_and_uses_calendar_month() {
    use chrono::Datelike;
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let ada = w.person("ada", &[]).await;
    let bob = w.person("bob", &[]).await;
    let owner = w.person("owner", &[]).await;
    let agent = w
        .app
        .create_agent(&owner, "Agent before rename".into())
        .await
        .unwrap();
    let start = Utc::now()
        .date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    for (solver, agent_id, at) in [
        (&ada, None, start - Duration::seconds(1)),
        (&ada, None, start),
        (&bob, None, start + Duration::seconds(1)),
        (&bob, None, start + Duration::seconds(2)),
        (&owner, Some(agent.id.clone()), start + Duration::seconds(3)),
    ] {
        let (_, record) = target(&w, &author, &reviewer).await;
        let a = w
            .app
            .submit_attempt(
                solver,
                &record,
                &"C1".into(),
                "proof".into(),
                agent_id,
                String::new(),
            )
            .await
            .unwrap();
        w.app.verify_attempt(&reviewer, &a.id).await.unwrap();
        let mut file = w.app.solve_file(&record, &"C1".into()).await.unwrap();
        file.attempts[0].receipt.as_mut().unwrap().checked_at = at;
        w.app.save_solve_file(&mut file).await.unwrap();
    }
    w.app
        .update_agent(&owner, &agent.id, Some("Renamed agent".into()), true)
        .await
        .unwrap();
    let all = w.app.leaderboard("all", "all").await.unwrap();
    assert_eq!(
        all.iter()
            .map(|r| (r.rank, r.entrant.id.as_str(), r.score))
            .collect::<Vec<_>>(),
        [
            (1, ada.person.as_str(), 2),
            (2, bob.person.as_str(), 2),
            (3, agent.id.as_str(), 1)
        ]
    );
    assert_eq!(all[2].entrant.name, "Renamed agent");
    assert!(all[2].entrant.retired);
    let month = w.app.leaderboard("month", "people").await.unwrap();
    assert_eq!(
        month
            .iter()
            .map(|r| (r.rank, r.entrant.id.as_str(), r.score))
            .collect::<Vec<_>>(),
        [(1, bob.person.as_str(), 2), (2, ada.person.as_str(), 1)]
    );
    let agents = w.app.leaderboard("month", "agents").await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].rank, 1);
    assert_eq!(
        w.app.entrant_profile(&agent.id).await.unwrap().entrant.name,
        "Renamed agent"
    );
}

#[tokio::test]
async fn confirmed_external_edges_are_revision_bound_and_count_distinct_public_works() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (_, record) = target(&w, &author, &reviewer).await;
    let target_file = w.app.solve_file(&record, &"C1".into()).await.unwrap();
    let source = w.submit(&author, "tt", false).await;
    let mut confirmations: Vec<_> = source
        .extracted
        .iter()
        .map(|c| ClaimConfirmation {
            id: c.id.clone(),
            kind: c.kind,
            role: c.role,
            depends_on: vec![],
            depends_on_conjectures: vec![],
            settles: None,
            excluded: false,
        })
        .collect();
    let external = ConjectureRef {
        record: record.clone(),
        claim: "C1".into(),
    };
    confirmations[0].depends_on_conjectures = vec![external.clone(), external.clone()];
    confirmations[1].depends_on_conjectures = vec![external];
    let mut source = w
        .app
        .confirm_claims(&author, &source.id, confirmations)
        .await
        .unwrap();
    assert_eq!(source.conjecture_dependencies.len(), 2);
    assert_eq!(
        source.conjecture_dependencies[0].target_version,
        target_file.version
    );
    assert_eq!(
        source.conjecture_dependencies[0].target_claims_revision,
        target_file.claims_revision
    );
    let source_record: RecordId = "WP-2026-0900".into();
    source.status = SubmissionStatus::Accepted {
        record: source_record.clone(),
    };
    source.analysis_visibility = Visibility::Public;
    w.app.save(&mut source).await.unwrap();
    let base = w
        .stores
        .for_submission(&target_file.submission)
        .await
        .unwrap()
        .unwrap();
    let mut rec = base.clone();
    rec.id = source_record;
    rec.submission = source.id.clone();
    rec.publication = Some(Box::new(source.clone()));
    RecordStore::insert(&*w.stores, &rec).await.unwrap();
    w.app.refresh_dependency_counts().await.unwrap();
    let file = w.app.solve_file(&record, &"C1".into()).await.unwrap();
    assert_eq!(file.downstream, 1);
    assert_eq!(file.dependencies.len(), 2);
    assert_eq!(file.dependencies[0].version, 1);
    assert_eq!(file.dependencies[0].claims_revision, source.claims_revision);
    w.app
        .set_analysis_visibility(&author, &source.id, Visibility::Private)
        .await
        .unwrap();
    w.app.refresh_dependency_counts().await.unwrap();
    assert_eq!(
        w.app
            .solve_file(&record, &"C1".into())
            .await
            .unwrap()
            .downstream,
        0
    );
}

#[tokio::test]
async fn reconciliation_repairs_requested_problem_jobs_without_backfilling_legacy_papers() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let admin = w.person("admin", &[]).await;
    let (original, _) = target(&w, &author, &reviewer).await;
    let mut paper = original.clone();
    paper.id = "legacy-paper".into();
    paper.kind = SubmissionKind::Paper;
    paper.lean_statements.clear();
    paper.problem_check_requested = false;
    let record: RecordId = "WP-2026-0910".into();
    paper.status = SubmissionStatus::Accepted {
        record: record.clone(),
    };
    w.app.ports.submissions.insert(&paper).await.unwrap();
    let mut rec = w
        .stores
        .for_submission(&original.id)
        .await
        .unwrap()
        .unwrap();
    rec.id = record;
    rec.submission = paper.id.clone();
    rec.publication = Some(Box::new(paper.clone()));
    RecordStore::insert(&*w.stores, &rec).await.unwrap();
    while w.stores.take_job().await.is_some() {}
    w.app.reconcile_attempts().await.unwrap();
    assert!(
        !w.stores
            .queued()
            .await
            .iter()
            .any(|(id, kind)| id == &paper.id && *kind == JobKind::OpenProblems)
    );
    w.app.queue_open_problems(&admin, &paper.id).await.unwrap();
    assert!(w.app.load(&paper.id).await.unwrap().problem_check_requested);
    // Simulate loss of the queued job after the durable request was saved.
    while w.stores.take_job().await.is_some() {}
    w.app.reconcile_attempts().await.unwrap();
    assert!(
        w.stores
            .queued()
            .await
            .iter()
            .any(|(id, kind)| id == &paper.id && *kind == JobKind::OpenProblems)
    );
}

fn faithful() -> FaithfulnessCheck {
    FaithfulnessCheck {
        verdict: FaithfulnessVerdict::Faithful,
        reason: "The domains, assumptions and conclusion agree.".into(),
        mismatch: String::new(),
        regenerated: false,
    }
}
async fn paper_target(w: &World, author: &Caller, reviewer: &Caller) -> (Submission, RecordId) {
    paper_target_with_markup(w, author, reviewer, true).await
}

async fn paper_target_with_markup(
    w: &World,
    author: &Caller,
    reviewer: &Caller,
    has_proof: bool,
) -> (Submission, RecordId) {
    let paper = w
        .review_paper(author, &w.submit(author, "t", false).await)
        .await;
    let mut paper = paper;
    paper.claims[0].has_proof = has_proof;
    w.app.save(&mut paper).await.unwrap();
    w.app
        .begin_referee_round(reviewer, &paper.id)
        .await
        .unwrap();
    w.app
        .update_referee_round(
            reviewer,
            &paper.id,
            1,
            RoundUpdate::Referee(done_step(referee_report(Some(Recommendation::Accept)))),
        )
        .await
        .unwrap();
    let paper = audited(
        w,
        reviewer,
        &paper,
        audit_for(
            &paper,
            Recommendation::Accept,
            Correctness::Correct,
            ProofShape::Content,
            None,
        ),
    )
    .await;
    let SubmissionStatus::Accepted { record } = paper.status.clone() else {
        panic!()
    };
    w.app
        .respond_to_formalization(author, &paper.id, &"C1".into(), true, String::new())
        .await
        .unwrap();
    let paper = w
        .app
        .record_lean_statement(
            reviewer,
            &paper.id,
            1,
            paper.claims_revision,
            &"C1".into(),
            "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
            "test".into(),
            "True".into(),
            None,
            faithful(),
        )
        .await
        .unwrap();
    let paper = w
        .app
        .respond_lean_statement(
            author,
            AuthenticationMethod::CookieSession,
            &paper.id,
            &paper.lean_statements[0].digest,
            true,
            String::new(),
        )
        .await
        .unwrap();
    (paper, record)
}
#[tokio::test]
async fn paper_statement_uses_attempt_verifier_and_credits_formalized_only() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (paper, record) = paper_target(&w, &a, &r).await;
    assert!(
        w.app
            .list_tasks(
                TaskFilter {
                    kind: Some(TaskKind::LegacyFormalize),
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
    let attempt = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            "proof".into(),
            None,
            "secret note".into(),
        )
        .await
        .unwrap();
    assert!(
        w.app.paper(&record).await.unwrap().claims[0]
            .verified
            .is_none()
    );
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    let public = w.app.paper(&record).await.unwrap();
    assert_eq!(public.summary.lean_verified, 1);
    assert_eq!(
        w.app.list_papers(None, None).await.unwrap().items[0].lean_verified,
        1
    );
    let query = PaperSearch {
        q: Some(record.to_string()),
        ..Default::default()
    };
    assert_eq!(
        w.app
            .search_papers(query.clone(), None, None)
            .await
            .unwrap()
            .items[0]
            .lean_verified,
        1
    );
    assert_eq!(
        public.claims[0].verified.as_ref().unwrap().entrant.id,
        solver.person.as_str()
    );
    assert!(
        !serde_json::to_string(&public)
            .unwrap()
            .contains("secret note")
    );
    assert!(w.app.leaderboard("all", "all").await.unwrap().is_empty());
    let profile = w.app.entrant_profile(solver.person.as_str()).await.unwrap();
    assert_eq!((profile.formalized.len(), profile.total), (1, 0));
    // Stored history is readable even when new consent/audit fields are absent.
    let mut legacy = serde_json::to_value(&paper).unwrap();
    legacy.as_object_mut().unwrap().remove("processing_consent");
    legacy["lean_statements"][0]
        .as_object_mut()
        .unwrap()
        .remove("faithfulness");
    let legacy: Submission = serde_json::from_value(legacy).unwrap();
    assert!(legacy.processing_consent.is_none());
    assert!(legacy.lean_statements[0].faithfulness.is_none());
    // A previously verified proof must disappear from list counts when its
    // exact target is no longer confirmed, even though the receipt remains.
    let mut current = w.app.load(&paper.id).await.unwrap();
    current.lean_statements.clear();
    w.app.save(&mut current).await.unwrap();
    assert_eq!(
        w.app.list_papers(None, None).await.unwrap().items[0].lean_verified,
        0
    );
    assert_eq!(
        w.app.search_papers(query, None, None).await.unwrap().items[0].lean_verified,
        0
    );
}
#[tokio::test]
async fn leaderboard_ranks_others_before_total_including_coauthors_and_agent_owners() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let coauthor = w.person("coauthor", &[]).await;
    let outsider = w.person("outsider", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let agent = w
        .app
        .create_agent(&coauthor, "coauthor-agent".into())
        .await
        .unwrap();
    for (solver, as_agent, coauthored) in [
        (&author, None, false),
        (&author, None, false),
        (&coauthor, Some(agent.id.clone()), true),
        (&outsider, None, false),
    ] {
        let (mut paper, record) = target(&w, &author, &r).await;
        if coauthored {
            paper = w.app.submission(&author, &paper.id).await.unwrap();
            paper.authors.push(Author {
                name: "Coauthor".into(),
                person: Some(coauthor.person.clone()),
                orcid: None,
                affiliation: None,
                google_scholar: None,
                website: None,
            });
            w.app.save(&mut paper).await.unwrap();
        }
        let attempt = w
            .app
            .submit_attempt(
                solver,
                &record,
                &"C1".into(),
                "proof".into(),
                as_agent,
                String::new(),
            )
            .await
            .unwrap();
        w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    }
    let rows = w.app.leaderboard("all", "all").await.unwrap();
    assert_eq!(rows[0].entrant.id, outsider.person.as_str());
    assert_eq!((rows[0].others, rows[0].total), (1, 1));
    assert_eq!(rows[1].entrant.id, author.person.as_str());
    assert_eq!((rows[1].others, rows[1].total), (0, 2));
    assert_eq!(
        (rows[2].entrant.id.as_str(), rows[2].others, rows[2].total),
        (agent.id.as_str(), 0, 1)
    );
    let p = w.app.entrant_profile(&agent.id).await.unwrap();
    assert_eq!((p.others, p.total), (0, 1));
}
struct ReportLease;
#[async_trait]
impl crate::ports::JobLease for ReportLease {
    async fn claim_in(
        &self,
        _: Option<crate::ports::JobLane>,
    ) -> CoreResult<Option<crate::ports::LeasedJob>> {
        Ok(None)
    }
    async fn renew(&self, _: &crate::ports::LeasedJob) -> CoreResult<bool> {
        Ok(true)
    }
    async fn complete(&self, _: &crate::ports::LeasedJob) -> CoreResult<()> {
        Ok(())
    }
    async fn defer(&self, _: &crate::ports::LeasedJob, _: std::time::Duration) -> CoreResult<()> {
        Ok(())
    }
    async fn retry(&self, _: &crate::ports::LeasedJob, _: &str) -> CoreResult<()> {
        Ok(())
    }
}
#[tokio::test]
async fn error_reports_verify_disproofs_gate_privacy_credit_recheck_and_void_targets() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let reporter = w.person("reporter", &[]).await;
    let stranger = w.person("stranger", &[]).await;
    let (paper, record) = paper_target(&w, &a, &r).await;
    let agent = w
        .app
        .create_agent(&reporter, "reporter-agent".into())
        .await
        .unwrap();
    let report = w
        .app
        .file_error_report(
            &reporter,
            &record,
            NewErrorReport {
                subject: ErrorSubject::Statement { claim: "C1".into() },
                text: "PRIVATE while pending".into(),
                solution: Some("disproof".into()),
                as_agent: Some(agent.id.clone()),
            },
        )
        .await
        .unwrap();
    assert!(w.app.error_report(None, &report.id).await.is_err());
    assert_eq!(
        w.app.paper(&record).await.unwrap().confirmed_error_reports,
        Some(0)
    );
    assert!(
        w.app
            .error_report(Some(&stranger), &report.id)
            .await
            .is_err()
    );
    assert!(w.app.error_report(Some(&a), &report.id).await.is_ok());
    let job = crate::ports::LeasedJob {
        submission: paper.id.clone(),
        kind: JobKind::ErrorReports,
        attempts: 1,
        lease: "fake".into(),
    };
    assert!(
        w.app
            .verify_report_evidence(&r, &report.id, &job, &ReportLease)
            .await
            .unwrap()
    );
    let public = w.app.error_report(None, &report.id).await.unwrap();
    assert_eq!(public.resolution.unwrap().verdict, ErrorVerdict::Confirmed);
    assert_eq!(
        w.app.paper(&record).await.unwrap().confirmed_error_reports,
        Some(1)
    );
    w.app
        .set_analysis_visibility(&a, &paper.id, Visibility::Private)
        .await
        .unwrap();
    let private = serde_json::to_value(w.app.paper(&record).await.unwrap()).unwrap();
    assert!(private.get("confirmed_error_reports").is_none());
    w.app
        .set_analysis_visibility(&a, &paper.id, Visibility::Public)
        .await
        .unwrap();
    let mut report_files = w.app.ports.solving.report_files().await.unwrap();
    let stored = report_files
        .iter_mut()
        .find(|file| file.reports.iter().any(|entry| entry.id == report.id))
        .unwrap();
    let mut future = stored
        .reports
        .iter()
        .find(|entry| entry.id == report.id)
        .unwrap()
        .clone();
    future.id = "different-version-report".into();
    future.version += 1;
    stored.reports.push(future);
    let expected = stored.revision;
    stored.revision += 1;
    w.app
        .ports
        .solving
        .replace_report_file(stored, expected)
        .await
        .unwrap();
    assert_eq!(
        w.app.paper(&record).await.unwrap().confirmed_error_reports,
        Some(1)
    );
    stored.reports.pop();
    let expected = stored.revision;
    stored.revision += 1;
    w.app
        .ports
        .solving
        .replace_report_file(stored, expected)
        .await
        .unwrap();
    assert!(public.solution.is_none());
    assert_eq!(
        w.app.entrant_profile(&agent.id).await.unwrap().errors_found,
        1
    );
    assert_eq!(w.app.error_notifications(&a).await.unwrap().len(), 1);
    let files = w
        .stores
        .ports(Arc::new(SystemClock), Arc::new(FakeReader))
        .referees;
    let file = files.get(&paper.id).await.unwrap().unwrap();
    assert!(
        file.recheck_requests
            .iter()
            .any(|x| x.starts_with("disproof:"))
    );
    assert!(
        file.recheck_requests
            .contains(&format!("error-report:{}", report.id))
    );
    for verdict in [
        ErrorVerdict::Refuted,
        ErrorVerdict::Unclear,
        ErrorVerdict::Confirmed,
    ] {
        let report = w
            .app
            .file_error_report(
                &reporter,
                &record,
                NewErrorReport {
                    subject: ErrorSubject::Review,
                    text: "Check the omitted case n=0.".into(),
                    solution: None,
                    as_agent: None,
                },
            )
            .await
            .unwrap();
        w.app
            .resolve_error_report(
                &r,
                &report.id,
                verdict,
                "Checked n=0 explicitly.".into(),
                "For n=0 the left side equals 1 and the right side equals 0.".into(),
            )
            .await
            .unwrap();
        assert_eq!(
            w.app.error_report(None, &report.id).await.is_ok(),
            verdict == ErrorVerdict::Confirmed
        );
        assert!(w.app.error_report(Some(&a), &report.id).await.is_ok());
    }
    let target_report = w
        .app
        .file_error_report(
            &reporter,
            &record,
            NewErrorReport {
                subject: ErrorSubject::Target {
                    claim: "C1".into(),
                    digest: paper.lean_statements[0].digest.clone(),
                },
                text: "Missing n=0.".into(),
                solution: None,
                as_agent: None,
            },
        )
        .await
        .unwrap();
    w.app
        .resolve_error_report(
            &r,
            &target_report.id,
            ErrorVerdict::Confirmed,
            "Missing a quantified case.".into(),
            "Lean assumes n>0 whereas the source includes n=0.".into(),
        )
        .await
        .unwrap();
    assert!(w.app.target(&record, &"C1".into()).await.is_err());
    assert!(
        files
            .get(&paper.id)
            .await
            .unwrap()
            .unwrap()
            .recheck_requests
            .contains(&format!("error-report:{}", target_report.id))
    );
    assert!(
        w.stores
            .queued()
            .await
            .contains(&(paper.id.clone(), JobKind::LeanStatement))
    );
    assert!(
        w.app
            .paper(&record)
            .await
            .unwrap()
            .lean_statements
            .is_empty()
    );
    assert_eq!(
        w.app
            .entrant_profile(reporter.person.as_str())
            .await
            .unwrap()
            .errors_found,
        2
    );
    // Fifth report is permitted; the sixth is rejected globally for this entrant.
    let draft = || NewErrorReport {
        subject: ErrorSubject::Review,
        text: "Another precise concern".into(),
        solution: None,
        as_agent: None,
    };
    w.app
        .file_error_report(&reporter, &record, draft())
        .await
        .unwrap();
    assert!(matches!(
        w.app.file_error_report(&reporter, &record, draft()).await,
        Err(CoreError::Conflict(_))
    ));
}
#[tokio::test]
async fn supporters_require_authoritative_usage_and_never_rank_by_tokens() {
    let w = world(20).await;
    let z = w.person("Zoe", &[]).await;
    let a = w.person("Ada", &[]).await;
    w.app
        .create_or_renew_donation(&z.person, 10000, "model")
        .await
        .unwrap();
    let donor = w
        .app
        .attribute_agent_run("CMA:probe:1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(donor, z.person);
    assert_eq!(
        w.app.attribute_agent_run("CMA:probe:1").await.unwrap(),
        Some(donor.clone())
    );
    assert_eq!(w.app.supporters().await.unwrap()[0].tokens, 0);
    assert!(
        w.app
            .record_funded_usage(
                &donor,
                "CMA:probe:1",
                TokenUsage {
                    input: 100,
                    output: 10,
                    metered: false
                }
            )
            .await
            .is_err()
    );
    let receipt = TokenUsage {
        input: 100,
        output: 10,
        metered: true,
    };
    w.app
        .record_funded_usage(&donor, "CMA:probe:1", receipt)
        .await
        .unwrap();
    w.app
        .record_funded_usage(&donor, "CMA:probe:1", receipt)
        .await
        .unwrap();
    assert_eq!(w.app.donation(&z).await.unwrap().unwrap().used, 110);
    w.app
        .create_or_renew_donation(&a.person, 10000, "model")
        .await
        .unwrap();
    let rows = w.app.supporters().await.unwrap();
    assert_eq!(
        rows.iter()
            .map(|s| (s.name.as_str(), s.tokens))
            .collect::<Vec<_>>(),
        vec![("Ada", 0), ("Zoe", 110)]
    );
    let public = serde_json::to_string(&rows).unwrap();
    assert!(!public.contains("CMA:"));
    assert!(!public.contains("rank"));
}

#[tokio::test]
async fn deposited_sources_preserve_authorship_and_use_only_independent_verification() {
    let w = world(20).await;
    let a = w.person("author", &[]).await;
    let r = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("depositor", &[]).await;
    let (_, record) = target(&w, &a, &r).await;
    let new = NewLeanDeposit {
        project_files: Default::default(),
        modules: Default::default(),
        formalizers: vec![],
        source_revision: None,
        title: "Existing formalization".into(),
        authors: vec!["Original mathematician".into()],
        statement: "True".into(),
        source_reference: Source {
            kind: SourceKind::Url,
            locator: "https://example.org/proof.lean".into(),
            year: None,
        },
        toolchain: "test".into(),
        license: "Apache-2.0".into(),
        solution: "import Target\ntheorem wishpool_solution : wishpool_target_prop := True.intro"
            .into(),
    };
    let deposit = w
        .app
        .submit_lean_deposit(&solver, new.clone())
        .await
        .unwrap();
    assert_eq!(deposit.authors, new.authors);
    assert!(deposit.attempts.is_empty());
    assert!(
        w.app
            .public_attempts(&record, &"C1".into())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        w.app
            .submit_lean_deposit(&solver, new.clone())
            .await
            .unwrap()
            .id,
        deposit.id
    );
    assert!(matches!(
        w.app.lean_deposit(&a, &deposit.id).await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(
        w.app
            .verify_lean_deposit(&r, &deposit.id, &record, &"C1".into())
            .await
            .is_err()
    );
    let attempt = w
        .app
        .verify_lean_deposit(&solver, &deposit.id, &record, &"C1".into())
        .await
        .unwrap();
    assert!(attempt.receipt.is_none());
    assert_eq!(attempt.state, "queued");
    assert_eq!(
        attempt.attribution.as_ref().unwrap().authors,
        vec!["Original mathematician"]
    );
    assert_eq!(
        w.app
            .verify_lean_deposit(&solver, &deposit.id, &record, &"C1".into())
            .await
            .unwrap()
            .id,
        attempt.id
    );
    assert_eq!(
        w.app
            .lean_deposit(&solver, &deposit.id)
            .await
            .unwrap()
            .attempts,
        vec![attempt.id.clone()]
    );
    w.app.verify_attempt(&r, &attempt.id).await.unwrap();
    assert_eq!(
        w.app
            .attempt(Some(&solver), &attempt.id)
            .await
            .unwrap()
            .receipt
            .unwrap()
            .verdict,
        Verdict::Proved
    );
    assert_eq!(
        w.app
            .public_attempts(&record, &"C1".into())
            .await
            .unwrap()
            .len(),
        1
    );
    let public_proofs = w.app.public_attempts(&record, &"C1".into()).await.unwrap();
    assert_eq!(
        public_proofs[0].attribution.as_ref().unwrap().license,
        "Apache-2.0"
    );
    let mut mismatch = new;
    mismatch.toolchain = "a different toolchain".into();
    let d = w.app.submit_lean_deposit(&solver, mismatch).await.unwrap();
    assert!(
        w.app
            .verify_lean_deposit(&solver, &d.id, &record, &"C1".into())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn project_deposits_bind_all_modules_and_public_directory_checks_current_visibility() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let owner = w.person("formalizer", &[]).await;
    let (paper, record) = target(&w, &author, &reviewer).await;
    let confirmed = w.app.target(&record, &"C1".into()).await.unwrap();
    let body = NewLeanDeposit {
        project_files: Default::default(),
        title: "Formalization project".into(), authors: vec!["Research author".into()], formalizers: vec!["Lean contributor".into()],
        source_revision: Some("a".repeat(40)), statement: "True".into(),
        source_reference: Source { kind: SourceKind::Url, locator: "https://example.org/commit/source".into(), year: None },
        toolchain: confirmed.toolchain.clone(), license: "Apache-2.0".into(),
        solution: "import Target\nimport Helpers\ntheorem wishpool_solution : wishpool_target_prop := helper".into(),
        modules: std::collections::BTreeMap::from([("Helpers".into(), "import Mathlib\ntheorem helper : True := True.intro".into())]),
    };
    let deposit = w
        .app
        .submit_lean_deposit(&owner, body.clone())
        .await
        .unwrap();
    assert_eq!(
        w.app
            .lean_deposit_modules(&owner, &deposit.id)
            .await
            .unwrap(),
        body.modules
    );
    assert!(matches!(
        w.app.lean_deposit_modules(&author, &deposit.id).await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(matches!(
        w.app
            .verify_lean_deposit_against(&owner, &deposit.id, &record, &"C1".into(), Some("stale"))
            .await,
        Err(CoreError::Conflict(_))
    ));
    let attempt = w
        .app
        .verify_lean_deposit_against(
            &owner,
            &deposit.id,
            &record,
            &"C1".into(),
            Some(&confirmed.digest),
        )
        .await
        .unwrap();
    assert_eq!(attempt.modules, body.modules);
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), &attempt.id)
            .await
            .is_err()
    );
    let query = || crate::app::FormalizationQuery {
        status: Some("open".into()),
        ..Default::default()
    };
    let open = w.app.formalization_opportunities(query()).await.unwrap();
    assert_eq!(open.items.len(), 1);
    assert_eq!(open.items[0].verified_count, 0);
    assert!(open.items[0].attribution.is_none());
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    assert!(
        w.app
            .formalization_opportunities(query())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let published = w
        .app
        .formalization_opportunities(crate::app::FormalizationQuery::default())
        .await
        .unwrap();
    assert_eq!(published.items[0].status, "verified");
    assert_eq!(
        published.items[0].attribution.as_ref().unwrap().formalizers,
        body.formalizers
    );
    let proof = w
        .app
        .public_attempts(&record, &"C1".into())
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        proof.receipt.solution_digest,
        lean_solution_digest(&body.solution, &body.modules)
    );
    assert_eq!(proof.modules, body.modules);
    let bundle = w
        .app
        .verification_bundle(&record, &"C1".into(), &attempt.id)
        .await
        .unwrap();
    assert_eq!(bundle.verification_request.modules, body.modules);
    assert_eq!(bundle.verification_request.target, confirmed.lean);
    assert_eq!(
        bundle.attempt.receipt.solution_digest,
        lean_solution_digest(
            &bundle.verification_request.solution,
            &bundle.verification_request.modules
        )
    );
    assert_eq!(
        bundle.source_sha256,
        paper.current_version().unwrap().archive.sha256
    );
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), "missing")
            .await
            .is_err()
    );
    let mut changed = body.clone();
    changed
        .modules
        .get_mut("Helpers")
        .unwrap()
        .push_str("\n-- revised");
    let next = w.app.submit_lean_deposit(&owner, changed).await.unwrap();
    assert_ne!(next.id, deposit.id);
    assert_ne!(
        w.app
            .verify_lean_deposit(&owner, &next.id, &record, &"C1".into())
            .await
            .unwrap()
            .id,
        attempt.id
    );
    w.app
        .set_analysis_visibility(&author, &paper.id, Visibility::Private)
        .await
        .unwrap();
    assert!(
        w.app
            .formalization_opportunities(Default::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(w.app.attempt(None, &attempt.id).await.is_err());
    assert!(
        w.app
            .verification_bundle(&record, &"C1".into(), &attempt.id)
            .await
            .is_err()
    );
    for name in [
        "../Escape",
        "Target",
        "Mathlib",
        "A..B",
        "9Bad",
        "/tmp/File",
        "Helpers/lean.lean",
    ] {
        let mut invalid = body.clone();
        invalid.modules = std::collections::BTreeMap::from([(
            name.into(),
            "theorem ok : True := True.intro".into(),
        )]);
        assert!(
            w.app.submit_lean_deposit(&owner, invalid).await.is_err(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn formalization_directory_pages_and_excludes_unconfirmed_or_stale_targets() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let mut records = vec![];
    for _ in 0..3 {
        records.push(target(&w, &author, &reviewer).await);
    }
    let mut seen = vec![];
    let mut before = None;
    loop {
        let page = w
            .app
            .formalization_opportunities(crate::app::FormalizationQuery {
                limit: Some(1),
                before,
                ..Default::default()
            })
            .await
            .unwrap();
        seen.extend(page.items.iter().map(|i| i.record.clone()));
        before = page.next_before;
        if before.is_none() {
            break;
        }
    }
    assert_eq!(seen.len(), 3);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 3);
    let (source, _) = records.remove(0);
    let mut source = w.app.load(&source.id).await.unwrap();
    source.claims_revision += 1;
    w.app.save(&mut source).await.unwrap();
    assert_eq!(
        w.app
            .formalization_opportunities(Default::default())
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    assert!(
        w.app
            .formalization_opportunities(crate::app::FormalizationQuery {
                kind: Some("paper".into()),
                ..Default::default()
            })
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        w.app
            .formalization_opportunities(crate::app::FormalizationQuery {
                status: Some("queued".into()),
                ..Default::default()
            })
            .await
            .is_err()
    );
}

#[tokio::test]
async fn full_project_deposit_is_private_immutable_and_accepts_a_separate_adapter() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("formalizer", &[]).await;
    let (_, record) = target(&w, &author, &reviewer).await;
    let new: NewLeanDeposit = serde_json::from_value(serde_json::json!({
        "title":"Research project", "authors":["Original author"], "statement":"True",
        "source_reference":{"kind":"url","locator":"https://example.org/research"},
        "toolchain":"test", "license":"Apache-2.0", "solution":"",
        "modules":(0..80).map(|i| (format!("Research.Part{i}"), "namespace Research\ntheorem helper : True := True.intro\nend Research")).collect::<std::collections::BTreeMap<_,_>>(),
        "project_files":{"lean-toolchain":"test", "README.md":"Original research project"}
    })).unwrap();
    let deposit = w
        .app
        .submit_lean_deposit(&solver, new.clone())
        .await
        .unwrap();
    assert_eq!(deposit.modules.len(), 80);
    assert_eq!(deposit.project_files, new.project_files);
    assert_eq!(
        w.app
            .lean_deposit_modules(&solver, &deposit.id)
            .await
            .unwrap(),
        new.modules
    );
    assert!(
        w.app
            .lean_deposit_modules(&author, &deposit.id)
            .await
            .is_err()
    );
    assert!(
        w.app
            .verify_lean_deposit(&solver, &deposit.id, &record, &"C1".into())
            .await
            .is_err()
    );
    let attempt = w.app.verify_lean_project(&solver, &deposit.id, &record, &"C1".into(), None, Some("import Target\nimport Research.Part0\ntheorem wishpool_solution : wishpool_target_prop := Research.helper".into())).await.unwrap();
    assert_eq!(attempt.state, "queued");
    assert!(attempt.receipt.is_none());
    assert_eq!(
        w.app
            .lean_deposit_source(&solver, &deposit.id)
            .await
            .unwrap(),
        ""
    );
}

#[tokio::test]
async fn formalization_continuations_reuse_terminal_events_and_recover_legacy_targets() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (mut paper, record) = paper_target(&w, &author, &reviewer).await;
    let claim = ClaimId::from("C1");
    let digest = paper.lean_statements[0].digest.clone();
    paper.formalization.items[0].progress = None;
    w.app.save(&mut paper).await.unwrap();
    while w.stores.take_job().await.is_some() {}
    w.app.reconcile_attempts().await.unwrap();
    assert!(
        w.stores
            .queued()
            .await
            .iter()
            .any(|(id, kind)| *id == paper.id && *kind == JobKind::AgentAttempt)
    );
    let (owner, agent) = w.app.venue_agent(&reviewer).await.unwrap();
    let attempt = w
        .app
        .submit_attempt(
            &owner,
            &record,
            &claim,
            "reject this proof".into(),
            Some(agent.id),
            String::new(),
        )
        .await
        .unwrap();
    assert!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 0, Some(&attempt.id))
            .await
            .is_err()
    );
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    assert_eq!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 0, Some(&attempt.id))
            .await
            .unwrap(),
        1
    );
    // Replaying the rejected attempt after a restart retains the new run key.
    assert_eq!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 1, Some(&attempt.id))
            .await
            .unwrap(),
        1
    );
    // A subsequent empty answer advances once; the older rejection cannot advance again.
    assert_eq!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 1, None)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 1, None)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, &digest, 2, Some(&attempt.id))
            .await
            .unwrap(),
        2
    );
    assert!(
        w.app
            .advance_formalization_round(&author, &record, &claim, &digest, 2, None)
            .await
            .is_err()
    );
    assert!(
        w.app
            .advance_formalization_round(&reviewer, &record, &claim, "different target", 2, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn formalization_progress_requires_evidence_and_cannot_regress_a_checked_proof() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (paper, record) = paper_target(&w, &author, &reviewer).await;
    let claim = ClaimId::from("C1");
    let mut progress = paper.formalization.items[0].progress.clone().unwrap();
    progress.target_digest = Some(paper.lean_statements[0].digest.clone());
    progress.state = FormalizationProgressState::Verified;
    assert!(
        w.app
            .record_formalization_progress(&reviewer, &paper.id, &claim, progress.clone())
            .await
            .is_err()
    );
    let attempt = w
        .app
        .submit_attempt(
            &author,
            &record,
            &claim,
            "valid proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    progress.state = FormalizationProgressState::InProgress;
    let saved = w
        .app
        .record_formalization_progress(&reviewer, &paper.id, &claim, progress.clone())
        .await
        .unwrap();
    assert_eq!(
        saved.formalization.items[0]
            .progress
            .as_ref()
            .unwrap()
            .state,
        FormalizationProgressState::Verified
    );
    progress.target_digest = Some("stale".into());
    assert!(
        w.app
            .record_formalization_progress(&reviewer, &paper.id, &claim, progress)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn legacy_formalization_receipt_recovers_without_rechecking_the_proof() {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (paper, record) = paper_target(&w, &author, &reviewer).await;
    let claim = ClaimId::from("C1");
    let attempt = w
        .app
        .submit_attempt(
            &author,
            &record,
            &claim,
            "valid proof".into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    let receipt = w
        .app
        .attempt(Some(&author), &attempt.id)
        .await
        .unwrap()
        .receipt;
    let mut current = w.app.submission(&author, &paper.id).await.unwrap();
    current.formalization.items[0].progress = None;
    w.app.save(&mut current).await.unwrap();
    while w.stores.take_job().await.is_some() {}
    w.app.reconcile_attempts().await.unwrap();
    assert!(
        w.stores
            .queued()
            .await
            .iter()
            .any(|(id, kind)| id.as_str() == attempt.id && *kind == JobKind::VerifyAttempt)
    );
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    let recovered = w.app.submission(&author, &paper.id).await.unwrap();
    assert_eq!(
        recovered.formalization.items[0]
            .progress
            .as_ref()
            .unwrap()
            .state,
        FormalizationProgressState::Verified
    );
    assert_eq!(
        w.app
            .attempt(Some(&author), &attempt.id)
            .await
            .unwrap()
            .receipt,
        receipt
    );
}

#[tokio::test]
async fn audited_prose_proof_can_reach_the_shared_formalization_target_without_rewriting_extraction()
 {
    let w = world(20).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (paper, record) = paper_target_with_markup(&w, &author, &reviewer, false).await;
    assert!(!paper.claims[0].has_proof);
    assert!(paper.approved_statement(&"C1".into()));
    assert_eq!(
        w.app
            .formalization_files(&reviewer, &paper.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(w.app.target(&record, &"C1".into()).await.is_ok());
    assert!(matches!(
        paper.formalization.items[0].state,
        ItemState::Approved | ItemState::InProgress
    ));
}
