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
    unavailable: AtomicBool,
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
                        "toolchain":"leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679", "reading":"Exact fixture statement.",
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
        } else if path.starts_with("api/v1/workspaces/") && method == "GET" {
            json!({"sandbox_id":"sbx_proofs","status":"ready"})
        } else if path.starts_with("api/v1/sandboxes/") && method == "GET" {
            let status = if self.unavailable.load(Ordering::SeqCst) {
                "credential_rotating"
            } else {
                "active"
            };
            json!({"owner":"publisher","status":status,"model_access":"managed","profile":{"id":"sbp_fixture","revision":1}})
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

const ENVIRONMENT: &str =
    "leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679";
/// The prepared libraries the verifier currently declares; a deployment may change them.
#[derive(Default)]
struct CollectedVerifier {
    libraries: std::sync::Mutex<Vec<PreparedLibrary>>,
}
fn library(revision: &str) -> PreparedLibrary {
    PreparedLibrary {
        name: "trureturing".into(),
        revision: revision.repeat(40),
        roots: vec!["D5".into(), "Trureturing".into()],
        url: Some("https://github.com/example/library".into()),
    }
}
#[async_trait]
impl Verifier for CollectedVerifier {
    async fn toolchain(&self) -> mathesis_core::CoreResult<String> {
        Ok(ENVIRONMENT.into())
    }
    async fn libraries(&self) -> mathesis_core::CoreResult<Vec<PreparedLibrary>> {
        Ok(self.libraries.lock().unwrap().clone())
    }
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
            library_use: None,
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}
async fn collected_world() -> (World, Arc<CollectedTransport>) {
    let (w, mock, _) = collected_world_with(vec![]).await;
    (w, mock)
}
async fn collected_world_with(
    libraries: Vec<PreparedLibrary>,
) -> (World, Arc<CollectedTransport>, Arc<CollectedVerifier>) {
    use base64::Engine;
    let mut w = World::new(vec![], true).await;
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(LatexReader));
    let verifier = Arc::new(CollectedVerifier {
        libraries: std::sync::Mutex::new(libraries),
    });
    ports.verifier = verifier.clone();
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
          "paper":{"title":"A fixture note (leanprover/lean4:v4.33.0, Mathlib db584cd6d46c92f209a44c0f1c829460d327499d)","abstract":"Fixture statements","authors":[{"name":"Ma, Haobo"}],"license":"CC-BY-4.0",
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
            profile: Some(json!({"id":"agp_fixture","revision":1})),
            poll: Duration::from_millis(1),
        }),
        timeout: Duration::from_secs(60),
        audit_timeout: Duration::from_secs(60),
    }));
    w.worker.cma_formalization_workspace = Some("wks_proofs".into());
    w.worker.formal_budget = Duration::from_secs(60);
    w.worker.audit_budget = Duration::from_secs(60);
    (w, mock, verifier)
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
    let calls = mock.calls.lock().await;
    let preparation = calls
        .iter()
        .filter_map(|(_, _, body, _)| body.as_ref())
        .find_map(|body| {
            body["input"]
                .as_str()
                .filter(|s| s.contains("Prepare exact Lean translations"))
        })
        .unwrap();
    assert!(preparation.contains("v4.35.0"));
    assert!(preparation.contains("56a005855ea17e0b0958f73ec801ae140b0a1679"));
    let (instructions, data) = preparation.split_once("\n{\"").unwrap();
    assert!(!instructions.contains("db584cd6"));
    assert!(data.contains("A fixture note (leanprover/lean4:v4.33.0, Mathlib db584cd6"));
    // Admitted prompt bytes keep the delivered author names; readers see the canonical form.
    assert!(data.contains(r#""authors":[{"name":"Ma, Haobo"}]"#));
    let shown = w
        .worker
        .app
        .collected_paper("a-note", Some(1))
        .await
        .unwrap();
    assert_eq!(shown.paper.authors[0].name, "Haobo Ma");
}

#[tokio::test]
async fn unavailable_sandbox_leaves_the_pool_open_without_creating_an_agent_or_failed_proof() {
    let (w, mock) = collected_world().await;
    mock.unavailable.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    assert!(matches!(
        w.worker.collected_proof_job(&job).await,
        Err(Failure::Transient(_))
    ));
    assert_eq!(mock.created.load(Ordering::SeqCst), 0);
    for claim in ["s1", "s2"] {
        let file = proof_file(&w, claim).await;
        let proof = file.collected.unwrap();
        assert!(proof.blocked.is_none() && proof.runs.is_empty());
        assert!(file.attempts.is_empty());
    }
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

/// Exact provider inputs of the model requests carrying `marker`, in order.
async fn model_inputs(mock: &CollectedTransport, marker: &str) -> Vec<String> {
    mock.calls
        .lock()
        .await
        .iter()
        .filter(|(method, path, _, _)| method == "POST" && path.ends_with("/responses"))
        .filter_map(|(_, _, body, _)| body.as_ref()?["input"].as_str().map(str::to_owned))
        .filter(|input| input.contains(marker))
        .collect()
}
/// The admitted prompt inside a framed provider input, and its durable digest.
fn admitted_digest(framed: &str) -> String {
    use sha2::{Digest, Sha256};
    let (_, rest) = framed.split_once("<chunk>\n").unwrap();
    let (prompt, _) = rest.rsplit_once("\n</chunk>").unwrap();
    format!("{:x}", Sha256::digest(prompt.as_bytes()))
}
async fn run_digest(w: &World, claim: &str, step: &str) -> String {
    let state = proof_file(w, claim).await.collected.unwrap().runs[step].clone();
    serde_json::from_value::<RunState>(state).unwrap().input
}
/// A run admitted earlier: its digest fences every later rendering of the prompt.
async fn admit_run(w: &World, claim: &str, step: &str, framed: &str) {
    let file = proof_file(w, claim).await;
    let state = RunState {
        deadline: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60,
        input: admitted_digest(framed),
        delivery_contract: 1,
        ..Default::default()
    };
    w.worker
        .app
        .save_collected_proof_run(
            &w.worker.reviewer,
            "a-note",
            1,
            &claim.into(),
            step,
            serde_json::to_value(state).unwrap(),
            file.revision,
        )
        .await
        .unwrap();
}
const PREPARE: &str = "Prepare exact Lean translations";
const PROVE: &str = "Complete this exact Lean proof";
// Bytes captured from the renderer that admitted existing work (develop f55b01b). They are
// never regenerated. The fixture delivery's author was "Original Author" at capture; when it
// became "Ma, Haobo" that one name was substituted in the two preparation files, the only
// bytes that differed.
const PREPARE_LEGACY: &str = include_str!("fixtures/collected-prepare-legacy.txt");
const PROOF_LEGACY: &str = include_str!("fixtures/collected-proof-legacy.txt");
const PREPARE_PINNED: &str = include_str!("fixtures/collected-prepare-pinned.txt");
const PROOF_PINNED: &str = include_str!("fixtures/collected-proof-pinned.txt");

#[tokio::test]
async fn new_work_without_a_prepared_library_renders_the_historical_pinned_prompt_bytes() {
    let (w, mock) = collected_world().await;
    mock.empty_first.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    assert!(w.worker.collected_proof_job(&job).await.unwrap());
    assert_eq!(model_inputs(&mock, PREPARE).await, [PREPARE_PINNED]);
    assert_eq!(model_inputs(&mock, PROVE).await, [PROOF_PINNED]);
    assert_eq!(
        run_digest(&w, "s1", "prepare:batch").await,
        admitted_digest(PREPARE_PINNED)
    );
    assert_eq!(
        run_digest(&w, "s1", "proof:0").await,
        admitted_digest(PROOF_PINNED)
    );
}

// The two tests below also run against a verifier that has since begun to declare
// a prepared library: work admitted before that keeps its prompts and gains no pin.
#[tokio::test]
async fn work_admitted_with_an_environment_pin_resumes_with_its_exact_prompt_bytes() {
    for declared in [vec![], vec![library("b")]] {
        let (w, mock, _) = collected_world_with(declared).await;
        mock.empty_first.store(true, Ordering::SeqCst);
        for claim in ["s1", "s2"] {
            w.worker
                .app
                .pin_collected_toolchain(
                    &w.worker.reviewer,
                    "a-note",
                    1,
                    &claim.into(),
                    ENVIRONMENT.into(),
                )
                .await
                .unwrap();
        }
        admit_run(&w, "s1", "prepare:batch", PREPARE_PINNED).await;
        let job = w.jobs.claim().await.unwrap().unwrap();
        // A changed rendering would fail here with "CMA inputs changed after admission".
        assert!(w.worker.collected_proof_job(&job).await.unwrap());
        assert_eq!(model_inputs(&mock, PREPARE).await, [PREPARE_PINNED]);
        assert_eq!(model_inputs(&mock, PROVE).await, [PROOF_PINNED]);
        for claim in ["s1", "s2"] {
            let proof = proof_file(&w, claim).await.collected.unwrap();
            assert_eq!(proof.toolchain.as_deref(), Some(ENVIRONMENT));
            assert!(proof.libraries.is_empty());
        }
    }
}

#[tokio::test]
async fn legacy_work_without_an_environment_pin_resumes_with_its_exact_prompt_bytes() {
    for declared in [vec![], vec![library("b")]] {
        let (w, mock, _) = collected_world_with(declared).await;
        mock.empty_first.store(true, Ordering::SeqCst);
        admit_run(&w, "s1", "prepare:batch", PREPARE_LEGACY).await;
        let job = w.jobs.claim().await.unwrap().unwrap();
        assert!(w.worker.collected_proof_job(&job).await.unwrap());
        assert_eq!(model_inputs(&mock, PREPARE).await, [PREPARE_LEGACY]);
        assert_eq!(model_inputs(&mock, PROVE).await, [PROOF_LEGACY]);
        assert_eq!(
            run_digest(&w, "s1", "proof:0").await,
            admitted_digest(PROOF_LEGACY)
        );
        for claim in ["s1", "s2"] {
            let proof = proof_file(&w, claim).await.collected.unwrap();
            assert!(proof.toolchain.is_none() && proof.libraries.is_empty());
        }
    }
}

/// The admitted prompt inside a framed provider input.
fn admitted_prompt(framed: &str) -> &str {
    let (_, rest) = framed.split_once("<chunk>\n").unwrap();
    rest.rsplit_once("\n</chunk>").unwrap().0
}

#[tokio::test]
async fn new_work_on_a_library_enabled_verifier_pins_the_library_and_proves_library_first() {
    let (w, mock, verifier) = collected_world_with(vec![library("b")]).await;
    mock.empty_first.store(true, Ordering::SeqCst);
    mock.uncertain.store(true, Ordering::SeqCst);
    let job = w.jobs.claim().await.unwrap().unwrap();
    // Admission is interrupted after the durable pin and the first provider write.
    let mut task = Box::pin(w.worker.collected_proof_job(&job));
    tokio::select! { result=&mut task=>panic!("unexpected completion: {result:?}"), _=mock.admitted.notified()=>{} }
    drop(task);
    for claim in ["s1", "s2"] {
        let proof = proof_file(&w, claim).await.collected.unwrap();
        assert_eq!(proof.toolchain.as_deref(), Some(ENVIRONMENT));
        assert_eq!(proof.libraries, [library("b")]);
    }
    let admitted = run_digest(&w, "s1", "prepare:batch").await;
    // The verifier is redeployed with another library revision before the retry.
    *verifier.libraries.lock().unwrap() = vec![library("c")];
    assert!(w.worker.collected_proof_job(&job).await.unwrap());
    assert_eq!(run_digest(&w, "s1", "prepare:batch").await, admitted);
    for claim in ["s1", "s2"] {
        assert_eq!(
            proof_file(&w, claim).await.collected.unwrap().libraries,
            [library("b")]
        );
    }

    let prepare = model_inputs(&mock, PREPARE).await;
    let prove = model_inputs(&mock, PROVE).await;
    assert_eq!((prepare.len(), prove.len()), (1, 1));
    assert_eq!(admitted_digest(&prepare[0]), admitted);
    for (rendered, historical) in [(&prepare[0], PREPARE_PINNED), (&prove[0], PROOF_PINNED)] {
        let (rendered, historical) = (admitted_prompt(rendered), admitted_prompt(historical));
        // Environment preparation, then the library rules, then the historical
        // instructions and data, each on its own line.
        let lines: Vec<_> = rendered.split('\n').collect();
        let expected: Vec<_> = historical.split('\n').collect();
        assert_eq!(lines.len(), expected.len() + 1);
        assert_eq!(lines[0], expected[0]);
        assert_eq!(lines[2..], expected[1..]);
        let rules = lines[1];
        assert!(rules.starts_with("PREPARED LIBRARIES OF THE INDEPENDENT VERIFIER: "));
        for required in [
            format!(
                "trureturing at full Git revision {} (source https://github.com/example/library), module roots D5, Trureturing",
                "b".repeat(40)
            ),
            "LIBRARY FIRST, THEN PROVE".into(),
            "existing_lean".into(),
            "importing that library module and applying its existing declaration".into(),
            "unfolding both definitions".into(),
            "rfl alone may fail".into(),
            "only what the library lacks at that revision".into(),
            "never named below a library root".into(),
        ] {
            assert!(rules.contains(&required), "{required}: {rules}");
        }
        assert!(!rules.contains(&"c".repeat(40)));
        // The report names references; it is never described as an equivalence.
        assert!(!rules.to_lowercase().contains("equivalent"));
    }
    let rules = admitted_prompt(&prepare[0]).split('\n').nth(1).unwrap();
    assert!(rules.contains("Set reused_from to the exact existing_lean object"));
    assert!(rules.contains("A Target never imports a library"));
}
