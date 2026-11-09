use super::*;
use crate::ports::{CollectedPaperStore, Verifier};

struct CheckExact;
#[async_trait::async_trait]
impl Verifier for CheckExact {
    async fn verify(&self, r: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        Ok(VerificationReceipt {
            verdict: Verdict::Proved,
            reason: "test verifier".into(),
            target_digest: r.target_digest.clone(),
            solution_digest: lean_solution_digest(&r.solution, &r.modules),
            toolchain: r.toolchain.clone(),
            axioms: vec![],
            used_constants: vec![],
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}
async fn world() -> (World, Caller, Caller) {
    let mut w = World::new().await;
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.verifier = Arc::new(CheckExact);
    w.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        1,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let (d, p, s) = super::collected::package(1);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    index(&w, 1).await;
    (w, owner, reviewer)
}
async fn index(w: &World, version: u32) {
    let v = w
        .stores
        .get("a-note", Some(version))
        .await
        .unwrap()
        .unwrap();
    let expected = v.source_index.as_ref().unwrap().revision;
    w.stores
        .save_source_index(
            "a-note",
            version,
            &CollectedSourceIndex {
                revision: expected + 1,
                source_sha256: v.source.sha256,
                bibliography: None,
                limitations: vec![],
                results: Some(CollectedResults {
                    statements: vec![CollectedStatement {
                        id: "s1".into(),
                        kind: ClaimKind::Theorem,
                        name: "Theorem 1".into(),
                        title: None,
                        statement: "Every natural number equals itself.".into(),
                        section: None,
                    }],
                    ..Default::default()
                }),
            },
            expected,
        )
        .await
        .unwrap();
}
fn proposal() -> CollectedProofProposal {
    CollectedProofProposal {
        lean: "import Mathlib\ndef wishpool_target_prop : Prop := ∀ n : ℕ, n = n\n".into(),
        toolchain: "pinned-toolchain".into(),
        reading: "Reflexivity for all natural numbers.".into(),
        solution:
            "import Target\ntheorem wishpool_solution : wishpool_target_prop := fun _ => rfl\n"
                .into(),
        modules: Default::default(),
        reused_from: None,
        formalizers: vec![],
        license: "Apache-2.0".into(),
    }
}
fn faithful() -> FaithfulnessCheck {
    FaithfulnessCheck {
        verdict: FaithfulnessVerdict::Faithful,
        reason: "Same domain, quantifier and equality.".into(),
        mismatch: String::new(),
        regenerated: false,
    }
}

#[tokio::test]
async fn collected_result_reaches_shared_verifier_directory_bundle_and_credit_without_referee() {
    let (w, owner, reviewer) = world().await;
    let outsider = w.person("outsider", &[]).await;
    let work = "a-note";
    let claim: ClaimId = "s1".into();
    assert!(
        w.app
            .request_collected_proof(&outsider, work, 1, &claim)
            .await
            .is_err()
    );
    w.app
        .request_collected_proof(&owner, work, 1, &claim)
        .await
        .unwrap();
    w.app
        .request_collected_proof(&owner, work, 1, &claim)
        .await
        .unwrap();
    assert_eq!(
        w.app.collected_proofs(work, 1).await.unwrap()[0].state,
        "preparing"
    );
    assert!(w.app.target(&work.into(), &claim).await.is_err());
    w.app
        .save_collected_proposal(&reviewer, work, &claim, proposal())
        .await
        .unwrap();
    w.app
        .admit_collected_target(&reviewer, work, &claim, faithful())
        .await
        .unwrap();
    // Elaboration by itself is never a proof receipt or leaderboard event.
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        w.app.collected_proofs(work, 1).await.unwrap()[0].state,
        "open"
    );
    let first = w
        .app
        .submit_collected_proposal(&reviewer, work, &claim)
        .await
        .unwrap()
        .unwrap();
    let retry = w
        .app
        .submit_collected_proposal(&reviewer, work, &claim)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.id, retry.id);
    w.app.verify_attempt(&reviewer, &first.id).await.unwrap();
    let view = w.app.collected_proofs(work, 1).await.unwrap();
    assert_eq!(view[0].state, "verified");
    assert_eq!(view[0].proofs.len(), 1);
    let directory = w
        .app
        .formalization_opportunities(Default::default())
        .await
        .unwrap();
    assert_eq!(directory.items.len(), 1);
    assert_eq!(directory.items[0].authors[0].name, "Original Author");
    assert_eq!(directory.items[0].status, "verified");
    let statement = w.app.public_statement(&work.into(), &claim).await.unwrap();
    assert!(statement.claim.verified.is_some());
    let bundle = w
        .app
        .verification_bundle(&work.into(), &claim, &first.id)
        .await
        .unwrap();
    assert_eq!(bundle.statement, "Every natural number equals itself.");
    assert_eq!(
        bundle.source_sha256,
        w.stores
            .get(work, Some(1))
            .await
            .unwrap()
            .unwrap()
            .source
            .sha256
    );
    assert_eq!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        w.stores
            .queued()
            .await
            .iter()
            .all(|(_, k)| !matches!(k, JobKind::Referee | JobKind::OpenProblems))
    );
    assert!(!serde_json::to_string(&view).unwrap().contains("runs"));
}

#[tokio::test]
async fn new_source_invalidates_targets_and_queued_attempts_while_preserving_history() {
    let (w, owner, reviewer) = world().await;
    let claim = "s1".into();
    w.app
        .request_collected_proof(&owner, "a-note", 1, &claim)
        .await
        .unwrap();
    w.app
        .save_collected_proposal(&reviewer, "a-note", &claim, proposal())
        .await
        .unwrap();
    w.app
        .admit_collected_target(&reviewer, "a-note", &claim, faithful())
        .await
        .unwrap();
    let attempt = w
        .app
        .submit_collected_proposal(&reviewer, "a-note", &claim)
        .await
        .unwrap()
        .unwrap();
    let (d, p, s) = super::collected::package(2);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    index(&w, 2).await;
    assert!(
        !w.app
            .collected_proof_active(&reviewer, "a-note", 1, &claim)
            .await
            .unwrap()
    );
    assert!(w.app.target(&"a-note".into(), &claim).await.is_err());
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    assert!(w.app.attempt(Some(&owner), &attempt.id).await.is_err());
    let file = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    assert!(file.attempts[0].invalidated.is_some());
    assert!(file.attempts[0].receipt.is_none());
    assert!(file.collected.unwrap().target.is_some());
    assert!(
        w.app
            .collected_proofs("a-note", 1)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn unfaithful_target_is_not_admitted_and_stale_run_writes_are_fenced() {
    let (w, owner, reviewer) = world().await;
    let claim = "s1".into();
    w.app
        .request_collected_proof(&owner, "a-note", 1, &claim)
        .await
        .unwrap();
    let file = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    w.app
        .save_collected_proof_run(
            &reviewer,
            "a-note",
            1,
            &claim,
            "prepare",
            serde_json::json!({"input":"exact"}),
            file.revision,
        )
        .await
        .unwrap();
    assert!(matches!(
        w.app
            .save_collected_proof_run(
                &reviewer,
                "a-note",
                1,
                &claim,
                "prepare",
                serde_json::json!({}),
                file.revision
            )
            .await,
        Err(CoreError::StaleRevision { .. })
    ));
    w.app
        .save_collected_proposal(&reviewer, "a-note", &claim, proposal())
        .await
        .unwrap();
    let mut audit = faithful();
    audit.verdict = FaithfulnessVerdict::Weaker;
    audit.mismatch = "Missing hypothesis".into();
    w.app
        .admit_collected_target(&reviewer, "a-note", &claim, audit)
        .await
        .unwrap();
    assert!(w.app.target(&"a-note".into(), &claim).await.is_err());
    assert_eq!(
        w.app.collected_proofs("a-note", 1).await.unwrap()[0].state,
        "needs_attention"
    );
    assert!(
        w.app
            .formalization_opportunities(Default::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn imported_existing_proof_keeps_original_credit_without_new_agent_points() {
    let (w, owner, reviewer) = world().await;
    let (mut d, p, s) = super::collected::package(2);
    let origin = CollectedLean {
        url: format!(
            "https://github.com/example/research/blob/{}/Proof.lean",
            "a".repeat(40)
        ),
        revision: "a".repeat(40),
    };
    d.paper.lean = Some(origin.clone());
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    index(&w, 2).await;
    let claim = "s1".into();
    w.app
        .request_collected_proof(&owner, "a-note", 2, &claim)
        .await
        .unwrap();
    let mut p = proposal();
    p.reused_from = Some(origin.clone());
    p.formalizers = vec!["Original Formalizer".into()];
    w.app
        .save_collected_proposal(&reviewer, "a-note", &claim, p)
        .await
        .unwrap();
    w.app
        .admit_collected_target(&reviewer, "a-note", &claim, faithful())
        .await
        .unwrap();
    let attempt = w
        .app
        .submit_collected_proposal(&reviewer, "a-note", &claim)
        .await
        .unwrap()
        .unwrap();
    w.app.verify_attempt(&reviewer, &attempt.id).await.unwrap();
    let proof = w
        .app
        .collected_proofs("a-note", 2)
        .await
        .unwrap()
        .remove(0)
        .proofs
        .remove(0);
    let attribution = proof.attribution.unwrap();
    assert_eq!(attribution.formalizers, vec!["Original Formalizer"]);
    assert_eq!(attribution.source_reference.locator, origin.url);
    assert_eq!(attribution.authors, vec!["Original Author"]);
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn expired_target_job_cannot_admit_after_verifier_io() {
    let (w, owner, reviewer) = world().await;
    let claim = "s1".into();
    w.app
        .request_collected_proof(&owner, "a-note", 1, &claim)
        .await
        .unwrap();
    w.app
        .save_collected_proposal(&reviewer, "a-note", &claim, proposal())
        .await
        .unwrap();
    let job = crate::ports::LeasedJob {
        submission: "collected:a-note:1".into(),
        kind: JobKind::CollectedFormalization,
        attempts: 1,
        lease: "expired".into(),
    };
    assert!(
        w.app
            .admit_collected_target_leased(
                &reviewer,
                "a-note",
                &claim,
                faithful(),
                &job,
                &TestAgentLease(false)
            )
            .await
            .is_err()
    );
    assert!(w.app.target(&"a-note".into(), &claim).await.is_err());
}

#[tokio::test]
async fn venue_collection_work_is_exempt_from_external_daily_attempt_limits() {
    let (w, owner, reviewer) = world().await;
    let claim = "s1".into();
    w.app
        .request_collected_proof(&owner, "a-note", 1, &claim)
        .await
        .unwrap();
    let p = proposal();
    w.app
        .save_collected_proposal(&reviewer, "a-note", &claim, p.clone())
        .await
        .unwrap();
    w.app
        .admit_collected_target(&reviewer, "a-note", &claim, faithful())
        .await
        .unwrap();
    let first = w
        .app
        .submit_collected_proposal(&reviewer, "a-note", &claim)
        .await
        .unwrap()
        .unwrap();
    w.app
        .continue_collected_proof(
            &reviewer,
            "a-note",
            &claim,
            &lean_solution_digest(&p.solution, &p.modules),
            format!("{}\n-- another checked attempt\n", p.solution),
            p.modules,
        )
        .await
        .unwrap();
    let second = w
        .app
        .submit_collected_proposal(&reviewer, "a-note", &claim)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(first.id, second.id);
}
