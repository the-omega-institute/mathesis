//! Lifecycle fixtures verify orchestration, never mathematical truth.
use super::*;
use mathesis_core::{ids::ClaimId, model::*, ports::*};
use mathesis_review::cma::{RunState, Transport, TransportReply};
use serde_json::{Value, json};

type CollectedCall = (String, String, Option<Value>, Option<String>);
#[derive(Default)]
struct CollectedTransport {
    calls: Mutex<Vec<CollectedCall>>,
    answers: Mutex<std::collections::BTreeMap<String, String>>,
    deleted: Mutex<BTreeSet<String>>,
    created: AtomicUsize,
    invalid: AtomicBool,
    empty_first: AtomicBool,
    delay_delete: AtomicBool,
    uncertain: AtomicBool,
    admitted: tokio::sync::Notify,
}
#[async_trait]
impl Transport for CollectedTransport {
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        key: Option<&str>,
    ) -> ReviewResult<TransportReply> {
        self.calls.lock().await.push((
            method.into(),
            path.into(),
            body.clone(),
            key.map(str::to_owned),
        ));
        let mut response_id = None;
        let value = if method == "POST" && path.ends_with("/agents") {
            assert!(path.starts_with("api/v1/workspaces/wks_proofs/"));
            if self.uncertain.swap(false, Ordering::SeqCst) {
                self.admitted.notify_one();
                return std::future::pending().await;
            }
            let n = self.created.fetch_add(1, Ordering::SeqCst) + 1;
            json!({"id":format!("agt_collected{n}")})
        } else if method == "POST" && path.ends_with("/responses") {
            let input = body.as_ref().unwrap()["input"].as_str().unwrap();
            let mut value = if self.invalid.load(Ordering::SeqCst) {
                json!({"proposals":"invalid fixture"})
            } else if input.contains("Prepare exact Lean translations") {
                let proposal = |prop: &str| {
                    json!({
                        "lean":format!("import Mathlib\ndef wishpool_target_prop : Prop := {prop}\n"),
                        "toolchain":"pinned-fixture", "reading":"Exact fixture statement.",
                        "solution":"import Target\ntheorem wishpool_solution : wishpool_target_prop := by simp [wishpool_target_prop]\n",
                        "modules":{},"reused_from":{"url":format!("https://github.com/example/research/blob/{}/Proof.lean", "a".repeat(40)),"revision":"a".repeat(40)},
                        "formalizers":["Existing Formalizer"],"license":"Apache-2.0"
                    })
                };
                json!({"proposals":{"s1":proposal("∀ n : ℕ, n = n"), "s2":proposal("∀ n : ℕ, n + 0 = n")}})
            } else if input.contains("Complete this exact Lean proof") {
                json!({"solution":"","modules":{}})
            } else if input.contains("Independently inspect each") {
                let check = json!({"verdict":"faithful","reason":"Fixture semantic comparison.","mismatch":"","regenerated":false});
                json!({"checks":{"s1":check,"s2":check}})
            } else {
                panic!("unexpected proof model request");
            };
            if self.empty_first.load(Ordering::SeqCst) && value["proposals"]["s1"].is_object() {
                value["proposals"]["s1"]["solution"] = json!("");
            }
            let id = format!("resp_{}", path.split('/').nth(3).unwrap());
            self.answers
                .lock()
                .await
                .insert(id.clone(), value.to_string());
            response_id = Some(id);
            Value::Null
        } else if method == "GET" && path.starts_with("api/v2/responses/") {
            let answer = self
                .answers
                .lock()
                .await
                .get(path.rsplit('/').next().unwrap())
                .unwrap()
                .clone();
            json!({"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":answer}]}]})
        } else if method == "DELETE" {
            self.deleted.lock().await.insert(path.into());
            json!({})
        } else if path.ends_with("/controls") {
            json!({"target_active":false,"target":{}})
        } else if method == "GET" && path.starts_with("api/v1/agents/") {
            if !self.delay_delete.load(Ordering::SeqCst) && self.deleted.lock().await.contains(path)
            {
                return Ok(TransportReply {
                    status: 404,
                    body: json!({"code":"not_found"}).to_string(),
                    response_id: None,
                });
            }
            json!({"status":"stopped"})
        } else {
            json!({})
        };
        Ok(TransportReply {
            status: 200,
            body: value.to_string(),
            response_id,
        })
    }
}

struct CollectedVerifier;
#[async_trait]
impl Verifier for CollectedVerifier {
    async fn verify(
        &self,
        request: &VerificationRequest,
    ) -> mathesis_core::CoreResult<VerificationReceipt> {
        Ok(VerificationReceipt {
            verdict: Verdict::Proved,
            reason: "fixture only".into(),
            target_digest: request.target_digest.clone(),
            solution_digest: lean_solution_digest(&request.solution, &request.modules),
            toolchain: request.toolchain.clone(),
            axioms: vec![],
            used_constants: vec![],
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}
async fn collected_world() -> (World, Arc<CollectedTransport>) {
    use base64::Engine;
    let mut w = World::new(vec![], true).await;
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(LatexReader));
    ports.verifier = Arc::new(CollectedVerifier);
    w.worker.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::new(),
        20,
        BTreeSet::from([w.author.person.clone()]),
    );
    let bytes = base64::engine::general_purpose::STANDARD.decode("UEsDBBQAAAAIABBmZV3X65kGPQAAAEkAAAAIAAAAbWFpbi50ZXiLSclPLs1NzStJzkksLq5OLCrJTM5JrY1JSk3PzKuGSda6lqUWVSqkJxYoZBYrpGXmZZak6sWk5qUgVAAAUEsBAhQDFAAAAAgAEGZlXdfrmQY9AAAASQAAAAgAAAAAAAAAAAAAAIABAAAAAG1haW4udGV4UEsFBgAAAAABAAEANgAAAGMAAAAAAA==").unwrap();
    // Use the already validated source archive fixture from the explanation test.
    let source = w.stores.put(&bytes, "application/zip").await.unwrap();
    let pdf = w
        .stores
        .put(b"%PDF-fixture", "application/pdf")
        .await
        .unwrap();
    let paper: CollectedVersion = serde_json::from_value(json!({
        "id":"collected-fixture", "owner":w.author.person,"request_digest":"exact-fixture", "published_on":chrono::Utc::now(),
        "pdf":pdf,"source":source,
        "delivery":{"schema":"mathesis.pre-reviewed-delivery.v1","producer":"publisher","work_id":"a-note","version":1,
          "previous_version":null,"authorize_publication":true,
          "paper":{"title":"A fixture note","abstract":"Fixture statements","authors":[{"name":"Original Author"}],"license":"CC-BY-4.0",
          "lean":{"url":format!("https://github.com/example/research/blob/{}/Proof.lean","a".repeat(40)),"revision":"a".repeat(40)}},
          "review":{"verdict":"PASS","pdf_sha256":pdf.sha256,"source_sha256":source.sha256,"record":{"private_review":"must stay private"}}},
        "source_index":{"revision":1,"source_sha256":source.sha256,"bibliography":null,"limitations":[],"results":{"statements":[
          {"id":"s1","kind":"theorem","name":"Theorem 1","title":null,"statement":"Every natural number equals itself.","section":null},
          {"id":"s2","kind":"lemma","name":"Lemma 2","title":null,"statement":"Adding zero fixes every natural number.","section":null}
        ],"connections":[],"macros":{}}}
    })).unwrap();
    CollectedPaperStore::insert(w.stores.as_ref(), &paper)
        .await
        .unwrap();
    w.worker
        .app
        .request_collected_proof(&w.author, "a-note", 1, &"s1".into())
        .await
        .unwrap();
    let mock = Arc::new(CollectedTransport::default());
    w.worker.advisor = Some(Arc::new(mathesis_review::cma::CmaAdvisor {
        client: Arc::new(mathesis_review::cma::Client {
            transport: mock.clone(),
            workspace: "wks_review".into(),
            profile: None,
            poll: Duration::from_millis(1),
        }),
        timeout: Duration::from_secs(60),
        audit_timeout: Duration::from_secs(60),
    }));
    w.worker.cma_formalization_workspace = Some("wks_proofs".into());
    w.worker.formal_budget = Duration::from_secs(60);
    w.worker.audit_budget = Duration::from_secs(60);
    (w, mock)
}
async fn proof_file(w: &World, claim: &str) -> SolveFile {
    w.worker
        .app
        .collected_proof_file(&w.worker.reviewer, "a-note", 1, &ClaimId::from(claim))
        .await
        .unwrap()
}

#[tokio::test]
async fn collected_batch_runs_through_the_worker_dispatch_and_verification_lanes() {
    let (w, mock) = collected_world().await;
    let job = w
        .jobs
        .claim_in(Some(JobLane::Review))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.kind, JobKind::CollectedFormalization);

    w.worker.handle(job).await;

    assert_eq!(mock.created.load(Ordering::SeqCst), 2);
    for claim in ["s1", "s2"] {
        let file = proof_file(&w, claim).await;
        assert!(file.collected.as_ref().unwrap().target.is_some());
        assert_eq!(file.attempts.len(), 1);
        assert!(file.attempts[0].receipt.is_none());
        let verification = w
            .jobs
            .claim_in(Some(JobLane::Verify))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(verification.kind, JobKind::VerifyAttempt);
        w.worker.handle(verification).await;
    }
    assert!(
        w.worker
            .app
            .collected_proofs("a-note", 1)
            .await
            .unwrap()
            .iter()
            .all(|p| p.state == "verified")
    );
    assert_eq!(w.oracle.submitted.load(Ordering::SeqCst), 0);
    assert!(w.file().await.rounds.is_empty());
}

#[tokio::test]
async fn paper_batch_reuses_inspection_across_results_and_recovers_cleanup_without_new_model_work()
{
    let (w, mock) = collected_world().await;
    mock.delay_delete.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    assert_eq!(job.kind, JobKind::CollectedFormalization);
    assert!(w.worker.collected_proof_job(&job).await.unwrap());
    assert_eq!(mock.created.load(Ordering::SeqCst), 2); // One preparation and independent inspection for both results.
    for claim in ["s1", "s2"] {
        let file = proof_file(&w, claim).await;
        assert!(file.collected.as_ref().unwrap().target.is_some());
        assert_eq!(file.attempts.len(), 1);
        w.worker
            .app
            .verify_attempt(&w.worker.reviewer, &file.attempts[0].id)
            .await
            .unwrap();
    }
    // Resume the same deferred owner after deletion becomes visible.
    mock.delay_delete.store(false, Ordering::SeqCst);
    ReviewQueue::enqueue(w.stores.as_ref(), &job.submission, job.kind)
        .await
        .unwrap();
    let resumed = w
        .jobs
        .claim_in(Some(JobLane::Review))
        .await
        .unwrap()
        .unwrap();
    assert!(!w.worker.collected_proof_job(&resumed).await.unwrap());
    assert_eq!(mock.created.load(Ordering::SeqCst), 2);
    assert!(
        w.worker
            .app
            .collected_proofs("a-note", 1)
            .await
            .unwrap()
            .iter()
            .all(|p| p.state == "verified")
    );
    assert!(
        w.worker
            .app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty()
    );
    for claim in ["s1", "s2"] {
        let file = proof_file(&w, claim).await;
        assert!(file.collected.unwrap().runs.values().all(|v| {
            serde_json::from_value::<RunState>(v.clone())
                .unwrap()
                .deleted
        }));
    }
    assert_eq!(w.oracle.submitted.load(Ordering::SeqCst), 0);
    assert!(w.file().await.rounds.is_empty());
    let calls = mock.calls.lock().await;
    assert!(
        calls
            .iter()
            .filter_map(|(_, _, body, _)| body.as_ref())
            .all(|v| !v.to_string().contains("private_review"))
    );
}

#[tokio::test]
async fn restart_replays_exact_uncertain_creation_and_never_admits_another_identity() {
    let (w, mock) = collected_world().await;
    mock.uncertain.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    let mut task = Box::pin(w.worker.collected_proof_job(&job));
    tokio::select! { result=&mut task=>panic!("unexpected completion: {result:?}"), _=mock.admitted.notified()=>{} }
    drop(task);
    assert!(w.worker.collected_proof_job(&job).await.unwrap());
    let calls = mock.calls.lock().await;
    let creates: Vec<_> = calls
        .iter()
        .filter(|(m, p, _, _)| m == "POST" && p.ends_with("/agents"))
        .collect();
    assert_eq!(creates.len(), 3); // uncertain preparation, exact replay, independent inspection
    assert_eq!(creates[0], creates[1]);
}

#[tokio::test]
async fn invalid_batch_is_paused_and_does_not_spend_another_model_run_on_retry() {
    let (w, mock) = collected_world().await;
    mock.invalid.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    assert!(matches!(
        w.worker.collected_proof_job(&job).await,
        Err(Failure::Permanent(_))
    ));
    assert!(
        w.worker
            .app
            .collected_proofs("a-note", 1)
            .await
            .unwrap()
            .iter()
            .all(|p| p.state == "needs_attention" && p.proofs.is_empty())
    );
    assert!(!w.worker.collected_proof_job(&job).await.unwrap());
    assert_eq!(mock.created.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn missing_proof_gets_focused_gap_work_then_stays_open_without_endless_retries() {
    let (w, mock) = collected_world().await;
    mock.empty_first.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    assert!(w.worker.collected_proof_job(&job).await.unwrap());
    assert_eq!(mock.created.load(Ordering::SeqCst), 3);
    let gap = proof_file(&w, "s1").await;
    assert!(gap.collected.as_ref().unwrap().target.is_some());
    assert!(gap.collected.as_ref().unwrap().blocked.is_some());
    assert!(gap.attempts.is_empty());
    assert!(
        w.worker
            .app
            .target(&"a-note".into(), &"s1".into())
            .await
            .is_ok()
    );
    let done = proof_file(&w, "s2").await;
    w.worker
        .app
        .verify_attempt(&w.worker.reviewer, &done.attempts[0].id)
        .await
        .unwrap();
    ReviewQueue::enqueue(w.stores.as_ref(), &job.submission, job.kind)
        .await
        .unwrap();
    let resumed = w
        .jobs
        .claim_in(Some(JobLane::Review))
        .await
        .unwrap()
        .unwrap();
    assert!(!w.worker.collected_proof_job(&resumed).await.unwrap());
    assert_eq!(mock.created.load(Ordering::SeqCst), 3);
}
