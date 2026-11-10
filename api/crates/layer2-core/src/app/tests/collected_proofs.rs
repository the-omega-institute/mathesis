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
            library_use: None,
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

async fn directory(w: &World, status: Option<&str>) -> Vec<super::super::FormalizationOpportunity> {
    w.app
        .formalization_opportunities(super::super::FormalizationQuery {
            status: status.map(str::to_owned),
            ..Default::default()
        })
        .await
        .unwrap()
        .items
}

#[tokio::test]
async fn pending_pool_is_public_before_provider_execution_without_private_metadata_or_credit() {
    let (w, _, reviewer) = world().await;
    w.app.reconcile_collected_proofs().await.unwrap();
    let claim = "s1".into();
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
            serde_json::json!({"private_provider_key": "private-provider-response"}),
            file.revision,
        )
        .await
        .unwrap();
    let pending = directory(&w, Some("unverified")).await;
    assert_eq!(pending.len(), 1);
    let row = &pending[0];
    assert_eq!(row.record.as_str(), "a-note");
    assert_eq!(row.version, 1);
    assert_eq!(row.status, "preparing");
    assert_eq!(row.statement, "Every natural number equals itself.");
    assert_eq!(row.authors[0].name, "Original Author");
    assert!(row.target_digest.is_none() && row.toolchain.is_none());
    assert_eq!(row.verified_count, 0);
    assert!(row.receipt.is_none() && row.contributor.is_none() && row.attribution.is_none());
    let json = serde_json::to_string(&row).unwrap();
    for private in [
        "private_provider_key",
        "private-provider-response",
        "runs",
        "blocked",
        "proposal",
        "requested_by",
    ] {
        assert!(!json.contains(private));
    }
    assert_eq!(directory(&w, Some("preparing")).await.len(), 1);
    for status in [None, Some("open"), Some("verified"), Some("disproved")] {
        assert!(directory(&w, status).await.is_empty());
    }
    let public = w
        .app
        .public_statement(&"a-note".into(), &claim)
        .await
        .unwrap();
    assert_eq!(public.claim.statement, row.statement);
    assert!(public.target.is_none());
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pending_pool_paginates_after_filters_and_rejects_stale_statement_pins() {
    let (w, _, _) = world().await;
    let mut version = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    let index = version.source_index.as_mut().unwrap();
    let expected = index.revision;
    index.revision += 1;
    let results = index.results.as_mut().unwrap();
    results.macros.insert("\\N".into(), "\\mathbb{N}".into());
    results.statements = (0..105)
        .map(|i| CollectedStatement {
            id: format!("s{i:03}"),
            kind: if i == 0 {
                ClaimKind::Conjecture
            } else {
                ClaimKind::Theorem
            },
            name: format!("Theorem {i}"),
            title: None,
            statement: format!(
                "{}: $n \\in \\N$.",
                if i % 2 == 0 { "selected" } else { "other" }
            ),
            section: None,
        })
        .collect();
    w.stores
        .save_source_index("a-note", 1, index, expected)
        .await
        .unwrap();
    w.app.reconcile_collected_proofs().await.unwrap();
    let mut cursor = None;
    let mut claims = BTreeSet::new();
    loop {
        let page = w
            .app
            .formalization_opportunities(super::super::FormalizationQuery {
                status: Some("unverified".into()),
                kind: Some("paper".into()),
                q: Some("selected Original Author".into()),
                basis: None,
                limit: Some(7),
                before: cursor,
            })
            .await
            .unwrap();
        assert!(page.items.len() <= 7);
        for row in page.items {
            assert!(row.statement.starts_with("selected"));
            assert_eq!(row.macros.get("\\N").unwrap(), "\\mathbb{N}");
            assert!(claims.insert(row.claim));
        }
        cursor = page.next_before;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(claims.len(), 52);
    assert_eq!(directory(&w, Some("preparing")).await.len(), 25);
    // Reindexing the same source can change a statement; old proof work must
    // not be attached to its replacement merely because the id still matches.
    let expected = index.revision;
    index.revision += 1;
    for statement in &mut index.results.as_mut().unwrap().statements {
        statement.statement.push_str(" changed");
    }
    w.stores
        .save_source_index("a-note", 1, index, expected)
        .await
        .unwrap();
    assert!(directory(&w, Some("unverified")).await.is_empty());
}

#[tokio::test]
async fn historical_internal_papers_enter_the_pool_without_a_manual_request() {
    let (w, _, reviewer) = world().await;
    assert!(
        w.app
            .collected_proofs("a-note", 1)
            .await
            .unwrap()
            .is_empty()
    );
    w.app.reconcile_collected_proofs().await.unwrap();
    let first = w.app.collected_proofs("a-note", 1).await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].state, "preparing");
    let claim = "s1".into();
    let before = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    w.app.reconcile_collected_proofs().await.unwrap();
    let after = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn admitted_environment_survives_reconciliation_and_rejects_a_different_proposal() {
    let (w, _, reviewer) = world().await;
    w.app.reconcile_collected_proofs().await.unwrap();
    let claim = "s1".into();
    w.app
        .pin_collected_toolchain(&reviewer, "a-note", 1, &claim, "original-pin".into())
        .await
        .unwrap();
    w.app.reconcile_collected_proofs().await.unwrap();
    assert!(
        w.app
            .pin_collected_toolchain(&reviewer, "a-note", 1, &claim, "new-pin".into())
            .await
            .is_err()
    );
    assert!(
        w.app
            .save_collected_proposal(&reviewer, "a-note", &claim, proposal())
            .await
            .is_err()
    );
    let file = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    assert_eq!(
        file.collected.unwrap().toolchain.as_deref(),
        Some("original-pin")
    );
}

#[tokio::test]
async fn library_pin_is_set_once_with_the_environment_and_never_added_to_admitted_work() {
    let (w, _, reviewer) = world().await;
    w.app.reconcile_collected_proofs().await.unwrap();
    let claim: ClaimId = "s1".into();
    let library = |revision: &str| PreparedLibrary {
        name: "trureturing".into(),
        revision: revision.repeat(40),
        roots: vec!["D5".into()],
        url: Some("https://github.com/example/library".into()),
    };
    let pinned = |w: &World| {
        let (app, reviewer, claim) = (w.app.clone(), reviewer.clone(), claim.clone());
        async move {
            let file = app
                .collected_proof_file(&reviewer, "a-note", 1, &claim)
                .await
                .unwrap();
            let proof = file.collected.unwrap();
            (proof.toolchain, proof.libraries, file.revision)
        }
    };
    // Nothing is pinned by an invalid declaration.
    let mut invalid = library("b");
    invalid.roots = vec!["D5.S3".into()];
    assert!(matches!(
        w.app
            .pin_collected_environment(&reviewer, "a-note", 1, &claim, "pin".into(), vec![invalid])
            .await,
        Err(CoreError::Invalid(_))
    ));
    assert_eq!(pinned(&w).await.0, None);

    let admitted = w
        .app
        .pin_collected_environment(
            &reviewer,
            "a-note",
            1,
            &claim,
            "pin".into(),
            vec![library("b")],
        )
        .await
        .unwrap();
    assert_eq!(admitted, ("pin".to_string(), vec![library("b")]));
    let saved = pinned(&w).await;
    // A retry with another declaration reads the saved pin and writes nothing.
    for libraries in [vec![library("c")], vec![]] {
        assert_eq!(
            w.app
                .pin_collected_environment(&reviewer, "a-note", 1, &claim, "pin".into(), libraries)
                .await
                .unwrap(),
            admitted
        );
    }
    assert_eq!(
        w.app
            .pin_collected_toolchain(&reviewer, "a-note", 1, &claim, "pin".into())
            .await
            .unwrap(),
        "pin"
    );
    assert_eq!(pinned(&w).await, saved);
    assert!(matches!(
        w.app
            .pin_collected_environment(
                &reviewer,
                "a-note",
                1,
                &claim,
                "other".into(),
                vec![library("b")]
            )
            .await,
        Err(CoreError::Conflict(_))
    ));

    // Work pinned without a library keeps none.
    let (w, _, reviewer) = world().await;
    w.app.reconcile_collected_proofs().await.unwrap();
    w.app
        .pin_collected_toolchain(&reviewer, "a-note", 1, &claim, "pin".into())
        .await
        .unwrap();
    assert_eq!(
        w.app
            .pin_collected_environment(
                &reviewer,
                "a-note",
                1,
                &claim,
                "pin".into(),
                vec![library("b")]
            )
            .await
            .unwrap(),
        ("pin".to_string(), vec![])
    );
    let file = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    // The stored state of library-free work is byte-identical to before.
    let stored = serde_json::to_value(file.collected.as_ref().unwrap()).unwrap();
    assert!(stored.get("libraries").is_none());
    assert_eq!(stored["toolchain"], "pin");

    // Legacy work admitted before any pin accepts neither.
    let (w, _, reviewer) = world().await;
    w.app.reconcile_collected_proofs().await.unwrap();
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
            "prepare:batch",
            serde_json::json!({"input":"admitted"}),
            file.revision,
        )
        .await
        .unwrap();
    assert!(matches!(
        w.app
            .pin_collected_environment(
                &reviewer,
                "a-note",
                1,
                &claim,
                "pin".into(),
                vec![library("b")]
            )
            .await,
        Err(CoreError::Conflict(_))
    ));
    let file = w
        .app
        .collected_proof_file(&reviewer, "a-note", 1, &claim)
        .await
        .unwrap();
    let proof = file.collected.unwrap();
    assert!(proof.toolchain.is_none() && proof.libraries.is_empty());
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
    assert_eq!(
        directory(&w, Some("unverified")).await[0].status,
        "preparing"
    );
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
    assert!(directory(&w, Some("preparing")).await.is_empty());
    let ready = directory(&w, Some("unverified")).await;
    assert_eq!(ready[0].status, "open");
    assert!(ready[0].target_digest.is_some());
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
    assert!(directory(&w, Some("unverified")).await.is_empty());
    assert_eq!(directory(&w, Some("verified")).await.len(), 1);
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
    assert!(directory(&w, Some("unverified")).await.is_empty());
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
