use super::*;
use crate::ports::{JobKind, LeasedJob, Verifier};
use sha2::{Digest, Sha256};

struct ResearchVerifier;
#[async_trait::async_trait]
impl Verifier for ResearchVerifier {
    async fn verify(&self, r: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        Ok(VerificationReceipt {
            used_constants: vec![],
            verdict: if r.solution.contains("reject") {
                Verdict::Rejected
            } else {
                Verdict::Proved
            },
            reason: "engineering fixture; no mathematical validation claimed".into(),
            target_digest: if r.solution.contains("forge") {
                "wrong-target".into()
            } else {
                r.target_digest.clone()
            },
            solution_digest: lean_solution_digest(&r.solution, &r.modules),
            toolchain: r.toolchain.clone(),
            axioms: vec!["propext".into()],
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}
pub(super) async fn fixture() -> (World, Caller, Caller, Submission, NewResearchObligation) {
    let stores = Arc::new(MemoryStores::default());
    let mut ports = stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.verifier = Arc::new(ResearchVerifier);
    let w = World {
        app: App::new(
            ports,
            Policy::default(),
            BTreeSet::from([PersonId::from("admin")]),
        ),
        stores,
    };
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let paper = w.submit(&author, "t", false).await;
    let paper = w.review_paper(&author, &paper).await;
    w.app
        .begin_referee_round(&reviewer, &paper.id)
        .await
        .unwrap();
    let mut audit = knowledge::with_comparisons(
        &w,
        &reviewer,
        &paper,
        audit_for(
            &paper,
            Recommendation::Reject,
            Correctness::Gap,
            ProofShape::Undetermined,
            None,
        ),
    )
    .await;
    let (mut file, stored) = w.app.referee_file(&paper.id).await.unwrap();
    let hash = |s: &str| format!("sha256:{:x}", Sha256::digest(s.as_bytes()));
    for reading in &mut audit.claims {
        let c = reading.coverage.as_mut().unwrap();
        let receipt = file
            .knowledge_events
            .iter_mut()
            .find_map(|e| match e {
                KnowledgeEvent::QueryCompleted { receipt } if receipt.id == c.receipt_ids[0] => {
                    Some(receipt)
                }
                _ => None,
            })
            .unwrap();
        let premise = KnowledgeDeclarationRef {
            snapshot: receipt.snapshot.clone().unwrap(),
            module_path: "Fixture/Source.lean".into(),
            declaration_name_key: "premise".into(),
            statement_id: hash("premise"),
            type_digest: hash("type"),
        };
        receipt.returned = vec![premise.clone()];
        c.binding = Some(KnowledgeBinding {
            premises: vec![premise],
            relation: BindingRelation::Implies,
            parameters: vec![ParameterBinding {
                parameter: "x".into(),
                expression: "y".into(),
            }],
            hypotheses: vec![HypothesisDischarge {
                hypothesis: "y is positive".into(),
                argument: String::new(),
            }],
            argument: "The theorem would apply after establishing positivity and the bridge."
                .into(),
            unresolved: vec!["Relate the two definitions of the invariant.".into()],
            source_lineage: vec![],
        });
    }
    file.rounds[0].audit = done_step(audit);
    w.app.store_referee_file(&mut file, stored).await.unwrap();
    let view = w
        .app
        .research_obligations(&author, &paper.id)
        .await
        .unwrap();
    let c = &view.candidates[0];
    let input = NewResearchObligation {
        round: c.subject.round,
        claim: c.subject.claim.clone(),
        gap: c.gap.clone(),
        candidate_digest: c.digest.clone(),
        goal: ResearchGoal {
            definitions: vec!["y is the given invariant".into()],
            hypotheses: vec!["The manuscript assumptions hold".into()],
            quantifiers: "For all admissible inputs".into(),
            conclusion: "y > 0".into(),
            correspondence: "Establish the required positivity premise".into(),
        },
        budget: ResearchBudget {
            max_attempts: 2,
            expires_at: chrono::Utc::now() + chrono::Duration::days(1),
        },
        consent: RESEARCH_CONSENT.into(),
    };
    (w, author, reviewer, paper, input)
}
fn job(paper: &Submission, kind: JobKind) -> LeasedJob {
    LeasedJob {
        submission: paper.id.clone(),
        kind,
        attempts: 1,
        lease: "research-fixture".into(),
    }
}
fn prepared(verdict: FaithfulnessVerdict) -> PreparedResearchTarget {
    PreparedResearchTarget {
        previous_digest: None,
        lean: "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
        toolchain: "fixture-toolchain".into(),
        reading: "Fixture translation only".into(),
        faithfulness: FaithfulnessCheck {
            mismatch: if verdict == FaithfulnessVerdict::Faithful {
                String::new()
            } else {
                "Different proposition".into()
            },
            verdict,
            reason: "Independent inspector fixture".into(),
            regenerated: false,
        },
    }
}
async fn ready(
    w: &World,
    author: &Caller,
    reviewer: &Caller,
    paper: &Submission,
    input: NewResearchObligation,
) -> (String, String) {
    let view = w
        .app
        .adopt_research_obligation(author, &paper.id, input)
        .await
        .unwrap();
    let key = view.obligations[0].obligation.id.clone();
    w.app
        .record_research_target(
            reviewer,
            &job(paper, JobKind::ResearchTarget),
            &TestAgentLease(true),
            &key,
            prepared(FaithfulnessVerdict::Faithful),
        )
        .await
        .unwrap();
    let view = w.app.research_obligations(author, &paper.id).await.unwrap();
    let digest = view.obligations[0].target.as_ref().unwrap().digest.clone();
    w.app
        .respond_research_target(
            author,
            AuthenticationMethod::CookieSession,
            &paper.id,
            &key,
            &digest,
            true,
            String::new(),
        )
        .await
        .unwrap();
    (key, digest)
}
fn attempt(digest: &str, solution: &str) -> ResearchAttemptInput {
    ResearchAttemptInput {
        target_digest: digest.into(),
        solution: solution.into(),
        modules: Default::default(),
        note: "private note".into(),
    }
}

#[tokio::test]
async fn research_adoption_is_private_exact_idempotent_and_bounded() {
    let (w, a, r, p, new) = fixture().await;
    let outsider = w.person("outsider", &[]).await;
    assert!(matches!(
        w.app.research_obligations(&outsider, &p.id).await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(
        w.app
            .adopt_research_obligation(&r, &p.id, new.clone())
            .await
            .is_err()
    );
    for bad in [
        NewResearchObligation {
            consent: String::new(),
            ..new.clone()
        },
        NewResearchObligation {
            candidate_digest: "forged".into(),
            ..new.clone()
        },
        NewResearchObligation {
            budget: ResearchBudget {
                max_attempts: 6,
                ..new.budget.clone()
            },
            ..new.clone()
        },
    ] {
        assert!(
            w.app
                .adopt_research_obligation(&a, &p.id, bad)
                .await
                .is_err()
        );
    }
    let first = w
        .app
        .adopt_research_obligation(&a, &p.id, new.clone())
        .await
        .unwrap();
    let replay = w
        .app
        .adopt_research_obligation(&a, &p.id, new)
        .await
        .unwrap();
    assert_eq!(
        first.obligations[0].obligation,
        replay.obligations[0].obligation
    );
    assert_eq!(replay.obligations.len(), 1);
    assert_eq!(
        first.obligations[0].obligation.candidate.binding.hypotheses[0].argument,
        ""
    );
    let mut changed = w.app.load(&p.id).await.unwrap();
    changed.claims_revision += 1;
    w.app.save(&mut changed).await.unwrap();
    let stale = w.app.research_obligations(&a, &p.id).await.unwrap();
    assert!(stale.candidates.is_empty());
    assert_eq!(stale.obligations[0].state, "stale_source");
    assert!(
        !w.app
            .research_active(&r, &p.id, &first.obligations[0].obligation.id)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn research_targets_require_live_lease_faithfulness_and_cookie_confirmation() {
    let (w, a, r, p, new) = fixture().await;
    let key = w
        .app
        .adopt_research_obligation(&a, &p.id, new)
        .await
        .unwrap()
        .obligations[0]
        .obligation
        .id
        .clone();
    assert!(
        w.app
            .record_research_target(
                &r,
                &job(&p, JobKind::ResearchTarget),
                &TestAgentLease(false),
                &key,
                prepared(FaithfulnessVerdict::Faithful)
            )
            .await
            .is_err()
    );
    w.app
        .record_research_target(
            &r,
            &job(&p, JobKind::ResearchTarget),
            &TestAgentLease(true),
            &key,
            prepared(FaithfulnessVerdict::Weaker),
        )
        .await
        .unwrap();
    let view = w.app.research_obligations(&a, &p.id).await.unwrap();
    let digest = &view.obligations[0].target.as_ref().unwrap().digest;
    for auth in [
        AuthenticationMethod::Bearer,
        AuthenticationMethod::CookieSession,
    ] {
        assert!(
            w.app
                .respond_research_target(&a, auth, &p.id, &key, digest, true, String::new())
                .await
                .is_err()
        );
    }
    assert!(
        w.app
            .submit_research_attempt(&a, &p.id, &key, attempt(digest, "proof"))
            .await
            .is_err()
    );
    w.app
        .respond_research_target(
            &a,
            AuthenticationMethod::CookieSession,
            &p.id,
            &key,
            digest,
            false,
            "Use the complete hypotheses".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        w.app
            .research_obligations(&a, &p.id)
            .await
            .unwrap()
            .obligations[0]
            .obligation
            .events
            .len(),
        2
    );
}

#[tokio::test]
async fn research_verification_keeps_original_decision_and_credit_and_pins_fresh_readback() {
    let (w, a, r, p, new) = fixture().await;
    let (key, digest) = ready(&w, &a, &r, &p, new).await;
    assert!(
        w.app
            .respond_research_target(
                &a,
                AuthenticationMethod::Bearer,
                &p.id,
                &key,
                &digest,
                true,
                String::new()
            )
            .await
            .is_err()
    );
    let view = w
        .app
        .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "proof"))
        .await
        .unwrap();
    let attempt_id = view.obligations[0].attempts[0].id.clone();
    let duplicate = w
        .app
        .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "proof"))
        .await
        .unwrap();
    assert_eq!(duplicate.obligations[0].attempts.len(), 1);
    let verify = job(&p, JobKind::VerifyResearch);
    assert!(
        w.app
            .verify_research_leased(&r, &verify, &TestAgentLease(false))
            .await
            .is_err()
    );
    assert!(
        w.app
            .verify_research_leased(&a, &verify, &TestAgentLease(true))
            .await
            .is_err()
    );
    w.app
        .verify_research_leased(&r, &verify, &TestAgentLease(true))
        .await
        .unwrap();
    w.app
        .verify_research_leased(&r, &verify, &TestAgentLease(true))
        .await
        .unwrap();
    let checked = w.app.research_obligations(&a, &p.id).await.unwrap();
    assert_eq!(checked.obligations[0].state, "verified_target");
    assert_eq!(
        checked.obligations[0]
            .obligation
            .events
            .iter()
            .filter(|e| matches!(e, ResearchEvent::AttemptChecked { .. }))
            .count(),
        1
    );
    assert_eq!(w.app.load(&p.id).await.unwrap().decision, p.decision);
    assert!(w.app.leaderboard("all", "all").await.unwrap().is_empty());
    let before = w.app.recheck_evidence(&r, &p.id).await.unwrap();
    w.app
        .recheck_research(&a, &p.id, &key, &attempt_id)
        .await
        .unwrap();
    assert_eq!(
        before,
        w.app.recheck_evidence(&r, &p.id).await.unwrap(),
        "admitted prompts remain unchanged"
    );
    // A newly admitted round freezes the trigger identities; fixture marks prior round settled.
    let (mut file, stored) = w.app.referee_file(&p.id).await.unwrap();
    let triggers = file.recheck_requests.clone();
    file.rounds[0].recheck_requests = triggers;
    w.app.store_referee_file(&mut file, stored).await.unwrap();
    let evidence = w.app.recheck_evidence(&r, &p.id).await.unwrap();
    assert!(evidence.contains("internal_research_evidence"));
    assert!(evidence.contains("proof"));
    let report = w.app.review_report(&a, &p.id).await.unwrap();
    assert_eq!(report.research.obligations.len(), 1);
    assert!(report.markdown().contains("Private research obligation"));
    let outsider = w.person("outsider", &[]).await;
    assert!(
        w.app
            .research_attempt(&outsider, &p.id, &key, &attempt_id)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn research_budget_pending_state_stop_and_forged_receipts_fail_closed() {
    let (w, a, r, p, mut new) = fixture().await;
    new.budget.max_attempts = 1;
    let (key, digest) = ready(&w, &a, &r, &p, new).await;
    let v = w
        .app
        .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "forge"))
        .await
        .unwrap();
    assert_eq!(v.obligations[0].state, "verifying");
    assert!(
        w.app
            .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "second"))
            .await
            .is_err()
    );
    assert!(
        w.app
            .verify_research_leased(&r, &job(&p, JobKind::VerifyResearch), &TestAgentLease(true))
            .await
            .is_err()
    );
    assert!(
        w.app
            .research_obligations(&a, &p.id)
            .await
            .unwrap()
            .obligations[0]
            .attempts[0]
            .receipt
            .is_none()
    );
    w.app
        .stop_research_obligation(&a, &p.id, &key, "Withdraw processing authorization".into())
        .await
        .unwrap();
    assert!(!w.app.research_active(&r, &p.id, &key).await.unwrap());
    w.app
        .verify_research_leased(&r, &job(&p, JobKind::VerifyResearch), &TestAgentLease(true))
        .await
        .unwrap();
    let stopped = w.app.research_obligations(&a, &p.id).await.unwrap();
    assert!(stopped.obligations[0].attempts[0].invalidated.is_some());
    assert!(stopped.obligations[0].attempts[0].receipt.is_none());
    assert!(
        w.app
            .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "forge"))
            .await
            .is_err()
    );
}

struct ResearchLeaseLostAfterVerification(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl crate::ports::JobLease for ResearchLeaseLostAfterVerification {
    async fn claim_in(&self, _: Option<crate::ports::JobLane>) -> CoreResult<Option<LeasedJob>> {
        unreachable!()
    }
    async fn renew(&self, _: &LeasedJob) -> CoreResult<bool> {
        Ok(self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0)
    }
    async fn complete(&self, _: &LeasedJob) -> CoreResult<()> {
        unreachable!()
    }
    async fn defer(&self, _: &LeasedJob, _: std::time::Duration) -> CoreResult<()> {
        unreachable!()
    }
    async fn retry(&self, _: &LeasedJob, _: &str) -> CoreResult<()> {
        unreachable!()
    }
}
#[tokio::test]
async fn research_receipt_cannot_land_after_lease_loss_or_expiry() {
    let (w, a, r, p, new) = fixture().await;
    let (key, digest) = ready(&w, &a, &r, &p, new).await;
    w.app
        .submit_research_attempt(&a, &p.id, &key, attempt(&digest, "proof"))
        .await
        .unwrap();
    let lease = ResearchLeaseLostAfterVerification(std::sync::atomic::AtomicUsize::new(0));
    assert!(
        w.app
            .verify_research_leased(&r, &job(&p, JobKind::VerifyResearch), &lease)
            .await
            .is_err()
    );
    assert!(
        w.app
            .research_obligations(&a, &p.id)
            .await
            .unwrap()
            .obligations[0]
            .attempts[0]
            .receipt
            .is_none()
    );
    let (mut file, stored) = w.app.referee_file(&p.id).await.unwrap();
    file.research_obligations[0].budget.expires_at =
        chrono::Utc::now() - chrono::Duration::seconds(1);
    w.app.store_referee_file(&mut file, stored).await.unwrap();
    w.app
        .verify_research_leased(&r, &job(&p, JobKind::VerifyResearch), &TestAgentLease(true))
        .await
        .unwrap();
    let view = w.app.research_obligations(&a, &p.id).await.unwrap();
    assert_eq!(view.obligations[0].state, "expired");
    assert!(view.obligations[0].attempts[0].receipt.is_none());
    assert!(view.obligations[0].attempts[0].invalidated.is_some());
}

#[tokio::test]
async fn research_candidates_reject_cross_claim_subject_and_later_settlement_stops_work() {
    let (w, a, r, p, new) = fixture().await;
    let (key, _) = ready(&w, &a, &r, &p, new).await;
    let (mut file, stored) = w.app.referee_file(&p.id).await.unwrap();
    if let StepState::Done { result, .. } = &mut file.rounds[0].audit.state {
        result.claims[0].coverage.as_mut().unwrap().subject.claim = "different".into();
    }
    w.app.store_referee_file(&mut file, stored).await.unwrap();
    assert!(
        w.app
            .research_obligations(&a, &p.id)
            .await
            .unwrap()
            .candidates
            .is_empty()
    );
    let (mut file, stored) = w.app.referee_file(&p.id).await.unwrap();
    if let StepState::Done { result, .. } = &mut file.rounds[0].audit.state {
        let coverage = result.claims[0].coverage.as_mut().unwrap();
        coverage.subject.claim = p.claims[0].id.clone();
        coverage.relation = CoverageRelation::Covered;
        let binding = coverage.binding.as_mut().unwrap();
        binding.unresolved.clear();
        binding.hypotheses[0].argument = "Checked in later fixture audit".into();
    }
    w.app.store_referee_file(&mut file, stored).await.unwrap();
    assert!(!w.app.research_active(&r, &p.id, &key).await.unwrap());
    assert_eq!(
        w.app
            .research_obligations(&a, &p.id)
            .await
            .unwrap()
            .obligations[0]
            .state,
        "resolved_by_audit"
    );
}
