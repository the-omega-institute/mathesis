//! Composed public authorization, managed storage and worker queue regressions.
//! Provider and verifier results here are fixtures, never mathematical evidence.
use super::*;
use mathesis_core::{ids::RecordId, model::*, ports::Clock};
use mathesis_review::cma::{RunState, Transport, TransportReply};
use serde_json::{Value, json};

struct ProofClock {
    epoch: chrono::DateTime<chrono::Utc>,
    start: tokio::time::Instant,
}
impl Clock for ProofClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.epoch + chrono::Duration::from_std(self.start.elapsed()).unwrap()
    }
}

type ProofCall = (String, String, Option<Value>, Option<String>);
#[derive(Default)]
struct ProofTransport {
    calls: Mutex<Vec<ProofCall>>,
    creation: tokio::sync::Notify,
    polled: tokio::sync::Notify,
    uncertain_creation: AtomicBool,
    answer_ready: AtomicBool,
    stop_visible: AtomicBool,
    delete_visible: AtomicBool,
}
#[async_trait]
impl Transport for ProofTransport {
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        key: Option<&str>,
    ) -> ReviewResult<TransportReply> {
        self.calls
            .lock()
            .await
            .push((method.into(), path.into(), body, key.map(str::to_owned)));
        let mut response_id = None;
        let value = if method == "POST" && path.ends_with("/agents") {
            self.creation.notify_one();
            if self.uncertain_creation.swap(false, Ordering::SeqCst) {
                return std::future::pending().await;
            }
            json!({"id":"agt_proof"})
        } else if method == "POST" && path.ends_with("/responses") {
            response_id = Some("resp_proof".into());
            Value::Null
        } else if method == "GET" && path == "api/v2/responses/resp_proof" {
            self.polled.notify_one();
            if self.answer_ready.load(Ordering::SeqCst) {
                json!({"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":json!({"solution":"import Target\ntheorem wishpool_solution : wishpool_target_prop := True.intro"}).to_string()}]}]})
            } else {
                json!({"status":"in_progress"})
            }
        } else if path.ends_with("/controls") {
            json!({"target_active":true,"runtime_support":{"cancel":true},"target":{"turn_sequence":7},"agent_revision":11})
        } else if method == "GET" && path == "api/v1/agents/agt_proof" {
            if self.delete_visible.load(Ordering::SeqCst) {
                return Ok(TransportReply {
                    status: 404,
                    body: json!({"code":"not_found"}).to_string(),
                    response_id: None,
                });
            }
            json!({"status":if self.stop_visible.load(Ordering::SeqCst) {"stopped"} else {"stopping"}})
        } else {
            json!({})
        };
        Ok(TransportReply {
            status: if method == "DELETE" || path.ends_with("/stop") {
                202
            } else {
                200
            },
            body: value.to_string(),
            response_id,
        })
    }
}

async fn proof_world(
    expiry_seconds: i64,
) -> (
    World,
    RecordId,
    String,
    Arc<ProofClock>,
    Arc<ProofTransport>,
) {
    let mut w = World::new(vec![completed("accept")], true).await;
    w.run_round().await;
    let clock = Arc::new(ProofClock {
        epoch: chrono::Utc::now(),
        start: tokio::time::Instant::now(),
    });
    let mut ports = w.stores.ports(clock.clone(), Arc::new(LatexReader));
    ports.verifier = Arc::new(RejectedProofFixture);
    w.worker.app = App::new(ports, Policy::default(), BTreeSet::new());
    let paper = w
        .worker
        .app
        .submission(&w.author, &w.paper.id)
        .await
        .unwrap();
    let SubmissionStatus::Accepted { record } = paper.status else {
        panic!("accepted fixture")
    };
    w.worker
        .app
        .respond_to_formalization(&w.author, &paper.id, &"C1".into(), true, String::new())
        .await
        .unwrap();
    let paper = w
        .worker
        .app
        .record_lean_statement(
            &w.worker.reviewer,
            &paper.id,
            1,
            paper.claims_revision,
            &"C1".into(),
            "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
            "fixture".into(),
            "Fixture target only".into(),
            None,
            FaithfulnessCheck {
                verdict: FaithfulnessVerdict::Faithful,
                reason: "Fixture only".into(),
                mismatch: String::new(),
                regenerated: false,
            },
        )
        .await
        .unwrap();
    let digest = paper.lean_statements.last().unwrap().digest.clone();
    w.worker
        .app
        .respond_lean_statement(
            &w.author,
            AuthenticationMethod::CookieSession,
            &paper.id,
            &digest,
            true,
            String::new(),
        )
        .await
        .unwrap();
    set_allowance(
        &w,
        &record,
        &digest,
        clock.now() + chrono::Duration::seconds(expiry_seconds),
        false,
    )
    .await;
    let transport = Arc::new(ProofTransport::default());
    let client = Arc::new(mathesis_review::cma::Client {
        transport: transport.clone(),
        workspace: "wks_test".into(),
        profile: None,
        poll: Duration::from_millis(1),
    });
    w.worker.cma_factory = Some(Arc::new(crate::cma_tokens::CmaFactory {
        operator: client.clone(),
        connection: None,
        app: w.worker.app.clone(),
    }));
    w.worker.advisor = Some(Arc::new(mathesis_review::cma::CmaAdvisor {
        client,
        timeout: Duration::from_secs(60),
        audit_timeout: Duration::from_secs(60),
    }));
    w.worker.formal_budget = Duration::from_secs(60);
    (w, record, digest, clock, transport)
}

async fn set_allowance(
    w: &World,
    record: &RecordId,
    digest: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
    paused: bool,
) {
    w.worker
        .app
        .control_formalization(
            &w.author,
            record,
            &"C1".into(),
            FormalizationControl {
                target_digest: digest.into(),
                total_generations: 2,
                expires_at,
                paused,
                change_allowance: false,
            },
        )
        .await
        .unwrap();
}

async fn proof_job(w: &World) -> Option<LeasedJob> {
    while let Some(job) = w.jobs.claim().await.unwrap() {
        if job.kind == JobKind::AgentAttempt {
            return Some(job);
        }
        w.jobs.complete(&job).await.unwrap();
    }
    None
}

async fn saved_run(w: &World) -> (String, AgentStep, RunState) {
    let (key, step) = w
        .file()
        .await
        .agent_steps
        .into_iter()
        .find(|(_, step)| {
            step.cma_attempts
                .iter()
                .any(|a| a.step.starts_with("agent-formalization:"))
        })
        .expect("admitted public managed run");
    let run = serde_json::from_value(step.progress.clone()).unwrap();
    (key, step, run)
}

async fn settle_cleanup(w: &World, transport: &ProofTransport, original: &RunState) {
    // Stop acknowledgment alone cannot admit DELETE or settle the job.
    assert!(
        !transport
            .calls
            .lock()
            .await
            .iter()
            .any(|(m, _, _, _)| m == "DELETE")
    );
    transport.stop_visible.store(true, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(30)).await;
    let job = proof_job(w).await.expect("cleanup owner retained");
    assert_eq!(job.attempts, 1, "cleanup does not consume failure retries");
    w.worker.handle(job).await;
    let (_, _, run) = saved_run(w).await;
    assert!(
        !run.deleted,
        "asynchronous DELETE is not an observed deletion"
    );
    assert!(proof_job(w).await.is_none());
    tokio::time::advance(Duration::from_secs(30)).await;
    w.worker
        .handle(
            proof_job(w)
                .await
                .expect("unobserved deletion retains owner"),
        )
        .await;
    let calls = transport.calls.lock().await;
    let deletes: Vec<_> = calls.iter().filter(|(m, _, _, _)| m == "DELETE").collect();
    assert!(deletes.len() >= 2);
    assert!(
        deletes.iter().all(|call| call.3 == deletes[0].3),
        "DELETE retries keep the same key"
    );
    drop(calls);
    transport.delete_visible.store(true, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(30)).await;
    w.worker
        .handle(proof_job(w).await.expect("deletion observation queued"))
        .await;
    let (_, _, run) = saved_run(w).await;
    assert!(run.deleted);
    assert_eq!(run.input, original.input);
    assert_eq!(run.deadline, original.deadline);
    assert_eq!(run.create_intent, original.create_intent);
    assert_eq!(run.response_id, original.response_id);
    assert!(proof_job(w).await.is_none());
}

#[tokio::test(start_paused = true)]
async fn pause_during_public_response_retains_cleanup_owner_and_resume_consumption() {
    let (w, record, digest, clock, transport) = proof_world(3600).await;
    let expires = clock.now() + chrono::Duration::hours(1);
    let job = proof_job(&w).await.unwrap();
    let pause = async {
        transport.polled.notified().await;
        set_allowance(&w, &record, &digest, expires, true).await;
    };
    tokio::join!(w.worker.handle(job), pause);
    let (key, step, run) = saved_run(&w).await;
    assert!(run.failure.is_some());
    assert!(
        run.deadline > clock.now().timestamp() as u64,
        "pause precedes provider deadline"
    );
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Failed);
    assert_eq!(step.cma_attempts[0].failure, Some(CmaFailure::Cancelled));
    assert!(step.cma_attempts[0].finished_at.is_some());
    settle_cleanup(&w, &transport, &run).await;
    let work = w
        .worker
        .app
        .formalization_work(Some(&w.author), &record, &"C1".into())
        .await
        .unwrap();
    assert_eq!(work.used_generations, 1);
    assert_eq!(work.state, "paused");
    assert!(clock.now().timestamp() as u64 > run.deadline);
    // Ordinary resume retains the exact allowance and expiry after cleanup.
    w.worker
        .app
        .control_formalization(
            &w.author,
            &record,
            &"C1".into(),
            FormalizationControl {
                target_digest: digest,
                total_generations: 2,
                expires_at: expires,
                paused: false,
                change_allowance: false,
            },
        )
        .await
        .unwrap();
    w.worker.handle(proof_job(&w).await.unwrap()).await;
    let (resumed_key, _, resumed) = saved_run(&w).await;
    assert_eq!(resumed_key, key);
    assert_eq!(resumed.input, run.input);
    assert_eq!(
        w.worker
            .app
            .formalization_work(Some(&w.author), &record, &"C1".into())
            .await
            .unwrap()
            .used_generations,
        1
    );
    let calls = transport.calls.lock().await;
    assert_eq!(
        calls
            .iter()
            .filter(|(m, p, _, _)| m == "POST" && p.ends_with("/responses"))
            .count(),
        1,
        "pause/resume cannot submit a new turn on a cancelled generation"
    );
}

#[tokio::test(start_paused = true)]
async fn expired_public_response_recovers_after_shutdown_and_missing_queue_owner() {
    let (mut w, record, _, clock, transport) = proof_world(10).await;
    let job = proof_job(&w).await.unwrap();
    let (shutdown, mut receiver) = tokio::sync::watch::channel(false);
    let interrupt = async {
        transport.polled.notified().await;
        shutdown.send(true).unwrap();
    };
    tokio::join!(
        w.worker.handle_until_shutdown(job.clone(), &mut receiver),
        interrupt
    );
    let (_, _, original) = saved_run(&w).await;
    assert!(original.failure.is_none());
    tokio::time::advance(Duration::from_secs(11)).await;
    assert!((clock.now().timestamp() as u64) < original.deadline);
    let retry = proof_job(&w)
        .await
        .expect("shutdown released admitted work");
    assert_ne!(retry.lease, job.lease);
    let configured = w.worker.advisor.take();
    w.worker.handle(retry).await;
    let (_, step, retained) = saved_run(&w).await;
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Running);
    assert_eq!(retained.response_id, original.response_id);
    assert!(
        proof_job(&w).await.is_none(),
        "unavailable cleanup is deferred"
    );
    w.worker.advisor = configured;
    tokio::time::advance(Duration::from_secs(30)).await;
    w.worker
        .handle(
            proof_job(&w)
                .await
                .expect("configuration loss retained cleanup owner"),
        )
        .await;
    let (_, step, cancelled) = saved_run(&w).await;
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Failed);
    assert_eq!(cancelled.agent_id, original.agent_id);
    // Reproduce an old worker's dropped job; reconciliation must restore cleanup
    // even though the allowance cannot admit any generation and no other job exists.
    tokio::time::advance(Duration::from_secs(30)).await;
    let orphan = proof_job(&w).await.unwrap();
    w.jobs.complete(&orphan).await.unwrap();
    assert!(proof_job(&w).await.is_none());
    w.worker.app.reconcile_attempts().await.unwrap();
    w.worker
        .handle(proof_job(&w).await.expect("expired saved run recovered"))
        .await;
    settle_cleanup(&w, &transport, &original).await;
    let work = w
        .worker
        .app
        .formalization_work(None, &record, &"C1".into())
        .await
        .unwrap();
    assert_eq!(work.state, "expired");
    assert_eq!(work.used_generations, 1);
    assert_eq!(
        transport
            .calls
            .lock()
            .await
            .iter()
            .filter(|(m, p, _, _)| m == "POST" && p.ends_with("/responses"))
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn final_public_answer_retains_job_through_asynchronous_stop_and_delete() {
    let (w, record, _, _, transport) = proof_world(3600).await;
    transport.answer_ready.store(true, Ordering::SeqCst);
    w.worker.handle(proof_job(&w).await.unwrap()).await;
    let (_, step, original) = saved_run(&w).await;
    assert!(original.answer.is_some());
    assert!(original.failure.is_none());
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Succeeded);
    assert_eq!(
        w.worker
            .app
            .formalization_files(&w.worker.reviewer, &w.paper.id)
            .await
            .unwrap()[0]
            .attempts
            .len(),
        1
    );
    settle_cleanup(&w, &transport, &original).await;
    let (_, step, run) = saved_run(&w).await;
    assert_eq!(run.answer, original.answer);
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Succeeded);
    assert_eq!(
        w.worker
            .app
            .formalization_work(None, &record, &"C1".into())
            .await
            .unwrap()
            .used_generations,
        1
    );
}

#[tokio::test(start_paused = true)]
async fn paused_uncertain_creation_reconciles_original_intent_without_new_response() {
    let (w, record, digest, clock, transport) = proof_world(3600).await;
    transport.uncertain_creation.store(true, Ordering::SeqCst);
    let (shutdown, mut receiver) = tokio::sync::watch::channel(false);
    let interrupt = async {
        transport.creation.notified().await;
        shutdown.send(true).unwrap();
    };
    tokio::join!(
        w.worker
            .handle_until_shutdown(proof_job(&w).await.unwrap(), &mut receiver),
        interrupt
    );
    let (_, _, original) = saved_run(&w).await;
    assert!(original.agent_id.is_none());
    set_allowance(
        &w,
        &record,
        &digest,
        clock.now() + chrono::Duration::hours(1),
        true,
    )
    .await;
    w.worker.handle(proof_job(&w).await.unwrap()).await;
    let (_, step, run) = saved_run(&w).await;
    assert_eq!(step.cma_attempts[0].outcome, CmaOutcome::Failed);
    assert_eq!(run.agent_id.as_deref(), Some("agt_proof"));
    settle_cleanup(&w, &transport, &run).await;
    let calls = transport.calls.lock().await;
    let creates: Vec<_> = calls
        .iter()
        .filter(|(m, p, _, _)| m == "POST" && p.ends_with("/agents"))
        .collect();
    assert_eq!(creates.len(), 2);
    assert_eq!(creates[0].2, creates[1].2);
    assert_eq!(creates[0].3, creates[1].3);
    assert!(
        !calls
            .iter()
            .any(|(m, p, _, _)| m == "POST" && p.ends_with("/responses"))
    );
}
