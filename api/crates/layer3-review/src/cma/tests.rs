use super::*;
use std::collections::VecDeque;
use tokio::sync::Mutex;
#[derive(Default)]
struct Memory {
    state: Mutex<Option<Value>>,
}
#[async_trait]
impl RunStore for Memory {
    fn key(&self) -> &str {
        "wishpool:paper:r1:audit:a0"
    }
    async fn load(&self) -> ReviewResult<Option<Value>> {
        Ok(self.state.lock().await.clone())
    }
    async fn save(&self, state: Value) -> ReviewResult<()> {
        *self.state.lock().await = Some(state);
        Ok(())
    }
}
type Call = (String, String, Option<Value>, Option<String>);
struct Fake {
    calls: Mutex<Vec<Call>>,
    replies: Mutex<VecDeque<ReviewResult<TransportReply>>>,
    agent_observations: Mutex<VecDeque<ReviewResult<TransportReply>>>,
}
#[async_trait]
impl Transport for Fake {
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
        if method == "GET" && path.starts_with("api/v1/agents/") && !path.ends_with("/controls") {
            let deleted = self
                .calls
                .lock()
                .await
                .iter()
                .any(|c| c.0 == "DELETE" && c.1 == path);
            return self
                .agent_observations
                .lock()
                .await
                .pop_front()
                .unwrap_or_else(|| {
                    if deleted {
                        Err(ReviewError::Provider {
                            status: 404,
                            body: "not_found".into(),
                        })
                    } else {
                        reply(json!({"status":"stopped"}))
                    }
                });
        }
        self.replies
            .lock()
            .await
            .pop_front()
            .expect("unexpected CMA request")
    }
}
fn reply(body: Value) -> ReviewResult<TransportReply> {
    Ok(TransportReply {
        status: 200,
        body: body.to_string(),
        response_id: None,
    })
}
fn accepted() -> ReviewResult<TransportReply> {
    Ok(TransportReply {
        status: 202,
        body: String::new(),
        response_id: None,
    })
}

#[tokio::test]
async fn preflight_reads_resources_without_creating_or_charging_an_agent() {
    for status in ["active", "stopped"] {
        let (client, transport) = client(vec![
            reply(json!({"sandbox_id":"sbx_test","status":"ready"})),
            reply(
                json!({"owner":"operator","status":status,"profile":{"id":"sbp_test","revision":1}}),
            ),
            reply(json!({"content":{"model_access":"own_credentials"}})),
        ]);
        assert_eq!(client.preflight().await.is_ok(), status == "active");
        let calls = transport.calls.lock().await;
        assert_eq!(calls.len(), 3);
        assert!(
            calls
                .iter()
                .all(|call| call.0 == "GET" && call.2.is_none() && call.3.is_none())
        );
    }
    let (mut client, transport) = client(vec![]);
    client.profile = None;
    assert!(client.preflight().await.is_err());
    assert!(transport.calls.lock().await.is_empty());
}
fn receipt(id: &str) -> ReviewResult<TransportReply> {
    Ok(TransportReply {
        status: 200,
        body: String::new(),
        response_id: Some(id.into()),
    })
}

#[tokio::test]
async fn workspace_facts_uses_pinned_instance_model_access_without_profile_visibility() {
    for access in ["chronoai_managed", "own_credentials"] {
        let (client, transport) = client(vec![
            reply(json!({"sandbox_id":"sbx_test","status":"ready"})),
            reply(json!({"owner":"operator","status":"active",
                "profile":{"id":"sbp_private","revision":1},"model_access":access})),
        ]);
        let facts = resources::workspace_facts(client.transport.as_ref(), &client.workspace)
            .await
            .unwrap();
        assert_eq!(facts.model_access, access);
        assert_eq!(facts.owner, "operator");
        assert_eq!(facts.sandbox_profile.id, "sbp_private");
        let calls = transport.calls.lock().await;
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].1, "api/v1/sandboxes/sbx_test");
        assert!(calls.iter().all(|call| call.0 == "GET"));
    }
}

#[tokio::test]
async fn preflight_still_requires_active_sandbox_with_instance_model_access() {
    for status in ["active", "retired"] {
        let (client, _) = client(vec![
            reply(json!({"sandbox_id":"sbx_test","status":"ready"})),
            reply(json!({"owner":"operator","status":status,
                "profile":{"id":"sbp_private","revision":1},
                "model_access":"chronoai_managed"})),
        ]);
        assert_eq!(client.preflight().await.is_ok(), status == "active");
    }
}

#[tokio::test]
async fn legacy_profile_failure_does_not_assume_managed_access() {
    let (client, _) = client(vec![
        reply(json!({"sandbox_id":"sbx_test","status":"ready"})),
        reply(json!({"owner":"operator","status":"active",
            "profile":{"id":"sbp_private","revision":1}})),
        Err(ReviewError::Provider {
            status: 404,
            body: "profile_not_found".into(),
        }),
    ]);
    assert!(matches!(
        resources::workspace_facts(client.transport.as_ref(), &client.workspace).await,
        Err(ReviewError::Provider { status: 404, .. })
    ));
}
fn completed(text: &str) -> ReviewResult<TransportReply> {
    reply(
        json!({"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]}]}),
    )
}
fn client(replies: Vec<ReviewResult<TransportReply>>) -> (Client, Arc<Fake>) {
    let transport = Arc::new(Fake {
        calls: Mutex::new(vec![]),
        replies: Mutex::new(replies.into()),
        agent_observations: Mutex::new(VecDeque::new()),
    });
    (
        Client {
            transport: transport.clone(),
            workspace: "wks_test".into(),
            profile: Some(json!({"id":"agp_test","revision":1})),
            poll: Duration::from_millis(1),
        },
        transport,
    )
}
#[tokio::test]
async fn creates_polls_returns_json_and_cleans_up_once() {
    let (client, fake) = client(vec![
        reply(json!({"id":"agt_test"})),
        receipt("resp_test"),
        reply(json!({"status":"in_progress"})),
        completed("{\"summary\":\"checked\"}"),
        accepted(),
        accepted(),
    ]);
    let store = Memory::default();
    let answer = client
        .answer(&store, "paper source".into(), Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(answer, "{\"summary\":\"checked\"}");
    let state: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert_eq!(state.agent_id.as_deref(), Some("agt_test"));
    assert_eq!(state.response_id.as_deref(), Some("resp_test"));
    assert!(state.stopped && state.deleted);
    assert_eq!(
        client
            .answer(&store, "paper source".into(), Duration::from_secs(60))
            .await
            .unwrap(),
        answer
    );
    let calls = fake.calls.lock().await;
    assert_eq!(calls.len(), 8);
    assert_eq!(calls[0].1, "api/v1/workspaces/wks_test/agents");
    assert_eq!(
        calls[0].2.as_ref().unwrap()["agent_profile"],
        json!({"id":"agp_test","revision":1})
    );
    assert_eq!(calls[1].2.as_ref().unwrap()["stream"], true);
    assert_eq!(calls[4].2, Some(json!({})));
    assert_eq!(calls[5].0, "GET");
    assert_eq!(calls[6].0, "DELETE");
    assert!(calls.iter().filter(|c| c.0 != "GET").all(|c| c.3.is_some()));
}
#[tokio::test]
async fn restart_uses_saved_agent_response_and_cursor_without_submitting() {
    let store = Memory::default();
    let (first, _) = client(vec![reply(json!({"id":"agt_old"})), receipt("resp_old")]);
    let prompt = "source and confirmed claims";
    let mut state = RunState {
        input: format!("{:x}", Sha256::digest(prompt.as_bytes())),
        deadline: now() + 60,
        ..Default::default()
    };
    let chunks = inputs(prompt).unwrap();
    first.advance(&store, &chunks, &mut state).await.unwrap();
    first.advance(&store, &chunks, &mut state).await.unwrap();
    drop(first);
    let (restarted, fake) = client(vec![completed("{\"claims\":[]}"), accepted(), accepted()]);
    assert_eq!(
        restarted
            .answer(&store, prompt.into(), Duration::from_secs(60))
            .await
            .unwrap(),
        "{\"claims\":[]}"
    );
    assert_eq!(fake.calls.lock().await[0].1, "api/v2/responses/resp_old");
    assert!(
        fake.calls
            .lock()
            .await
            .iter()
            .all(|c| !c.1.ends_with("/responses") && !c.1.ends_with("/agents"))
    );
}
#[tokio::test]
async fn uncertain_mutations_reuse_the_exact_key_and_body() {
    let (client, fake) = client(vec![
        Err(ReviewError::Transport("lost reply".into())),
        reply(json!({"id":"agt_test"})),
        Err(ReviewError::Provider {
            status: 429,
            body: "conversation_not_ready".into(),
        }),
        receipt("resp_test"),
        completed("{}"),
        accepted(),
        accepted(),
    ]);
    client
        .answer(&Memory::default(), "data".into(), Duration::from_secs(60))
        .await
        .unwrap();
    let calls = fake.calls.lock().await;
    assert_eq!(calls[0], calls[1]);
    assert_eq!(calls[2], calls[3]);
}
#[tokio::test]
async fn cancelled_active_turn_observes_control_revision_then_stops_and_deletes() {
    let store = Memory::default();
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_test".into()),
                response_id: Some("resp_test".into()),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![
        reply(
            json!({"target_active":true,"target":{"turn_sequence":42},"agent_revision":17,"runtime_support":{"cancel":true}}),
        ),
        accepted(),
        accepted(),
        accepted(),
    ]);
    client.cancel(&store).await.unwrap();
    let calls = fake.calls.lock().await;
    assert_eq!(
        calls[1].2,
        Some(json!({"turn_sequence":42,"expected_agent_revision":17,"command":{"type":"cancel"}}))
    );
    assert!(calls[2].1.ends_with("/stop"));
    assert_eq!(calls[4].0, "DELETE");
}
#[tokio::test]
async fn queued_cancel_uses_the_documented_command_receipt() {
    let store = Memory::default();
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_test".into()),
                response_id: Some("resp_test".into()),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![
        reply(json!({"target":null})),
        reply(
            json!({"cancellable":true,"command_revision":3,"response":{"command_id":"cmd_source"}}),
        ),
        accepted(),
        accepted(),
        accepted(),
    ]);
    client.cancel(&store).await.unwrap();
    assert_eq!(
        fake.calls.lock().await[2].2,
        Some(json!({"source_command_id":"cmd_source","expected_command_revision":3}))
    );
}
#[tokio::test]
async fn malformed_final_and_failed_turns_are_cleaned_up() {
    for final_reply in [
        completed("```json\n{}\n```"),
        completed("[1]"),
        completed(&"x".repeat(MAX_JSON + 1)),
        reply(json!({"status":"incomplete"})),
    ] {
        let (client, fake) = client(vec![
            reply(json!({"id":"agt_test"})),
            receipt("resp_test"),
            final_reply,
            reply(json!({"target_active":false,"target":{}})),
            accepted(),
            accepted(),
        ]);
        assert!(
            client
                .answer(&Memory::default(), "data".into(), Duration::from_secs(60))
                .await
                .is_err()
        );
        let calls = fake.calls.lock().await;
        assert!(calls[calls.len() - 4].1.ends_with("/stop"));
        assert_eq!(calls[calls.len() - 2].0, "DELETE");
    }
}
#[tokio::test]
async fn oversize_input_is_split_across_durable_turns_without_truncation() {
    let prompt = "αβ\\input{lemma}\n".repeat(22000);
    let chunks = inputs(&prompt).unwrap();
    assert!(chunks.len() > 1);
    assert!(chunks.iter().all(|p| p.len() < 256 * 1024));
    let reassembled: String = chunks
        .iter()
        .map(|c| {
            c.split_once("<chunk>\n")
                .unwrap()
                .1
                .strip_suffix("\n</chunk>")
                .unwrap()
        })
        .collect();
    assert_eq!(reassembled, prompt);
    let mut replies = vec![reply(json!({"id":"agt_test"}))];
    for i in 0..chunks.len() {
        replies.push(receipt(&format!("resp_{i}")));
        replies.push(completed(if i + 1 == chunks.len() {
            "{\"done\":true}"
        } else {
            "{}"
        }));
    }
    replies.extend([accepted(), accepted()]);
    let (client, fake) = client(replies);
    assert_eq!(
        client
            .answer(&Memory::default(), prompt, Duration::from_secs(60))
            .await
            .unwrap(),
        "{\"done\":true}"
    );
    let calls = fake.calls.lock().await;
    let keys: std::collections::BTreeSet<_> = calls
        .iter()
        .filter(|c| c.1.ends_with("/responses"))
        .map(|c| c.3.clone())
        .collect();
    assert_eq!(keys.len(), chunks.len());
}
#[test]
fn cli_exit_zero_errors_are_mapped_without_echoing_messages() {
    for status in [401, 403, 404, 409, 429, 500, 502, 503] {
        for body in [
            json!({"error":{"code":"permission_denied","message":"private data"}}),
            json!({"code":"permission_denied","detail":"private data"}),
        ] {
            let error = transport::parse_cli(
                body.to_string().as_bytes(),
                format!("HTTP {status} failure").as_bytes(),
                true,
                None,
            )
            .err()
            .unwrap();
            assert!(matches!(error,ReviewError::Provider{status:s,..} if s==status));
            assert!(!error.to_string().contains("private data"));
        }
    }
    assert!(transport::parse_cli(b"{\"error\":\"denied\"}", b"", true, None).is_err());
    assert!(transport::parse_cli(b"", b"", false, None).is_err());
    let sse = "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_sse\"}}\n\n";
    assert_eq!(
        transport::parse_cli(sse.as_bytes(), b"", true, None)
            .unwrap()
            .response_id()
            .unwrap(),
        "resp_sse"
    );
}
struct Tokens(String);
#[async_trait]
impl TokenSource for Tokens {
    async fn token(&self) -> ReviewResult<String> {
        Ok(self.0.clone())
    }
}
#[tokio::test]
async fn http_transport_maps_statuses_and_uses_the_durable_sse_header() {
    use axum::{Router, http::StatusCode, routing::any};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let token = format!("test-user-{}", now());
    let expected = token.clone();
    let router = Router::new().route(
        "/{status}",
        any(
            move |axum::extract::Path(status): axum::extract::Path<u16>,
                  headers: axum::http::HeaderMap| {
                let expected = expected.clone();
                async move {
                    assert_eq!(headers["authorization"], format!("Bearer {expected}"));
                    (
                        StatusCode::from_u16(status).unwrap(),
                        [("x-cma-response-id", "resp_handshake")],
                        "{\"error\":{\"code\":\"typed_refusal\",\"message\":\"private\"}}",
                    )
                }
            },
        ),
    );
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let http = HttpTransport {
        base_url: format!("http://{addr}"),
        tokens: Arc::new(Tokens(token)),
    };
    for status in [401, 403, 404, 409, 429, 500, 503] {
        let e = http
            .request("GET", &status.to_string(), None, None)
            .await
            .err()
            .unwrap();
        assert!(matches!(e,ReviewError::Provider{status:s,..}if s==status));
        assert!(!e.to_string().contains("private"));
    }
    assert_eq!(
        http.request(
            "POST",
            "200",
            Some(json!({"input":"x","stream":true})),
            Some("same-key")
        )
        .await
        .unwrap()
        .response_id()
        .unwrap(),
        "resp_handshake"
    );
    let agent_key = HttpTransport {
        base_url: format!("http://{addr}"),
        tokens: Arc::new(Tokens(format!("nyxid_ag_{}", now()))),
    };
    assert!(agent_key.request("GET", "200", None, None).await.is_err());
    server.abort();
}

#[tokio::test]
async fn http_transport_disconnects_after_a_stream_receipt_without_a_header() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 8192];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: response.created\r\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_stream\"}}\r\n\r\n").await.unwrap();
        // Keep the stream open until this test aborts its own server.
        std::future::pending::<()>().await;
    });
    let transport = HttpTransport {
        base_url: format!("http://{addr}"),
        tokens: Arc::new(Tokens(format!("test-user-{}", now()))),
    };
    let reply = tokio::time::timeout(
        Duration::from_secs(5),
        transport.request(
            "POST",
            "api/v2/agents/agt_test/responses",
            Some(json!({"input":"x","stream":true})),
            Some("durable-key"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reply.response_id().unwrap(), "resp_stream");
    server.abort();
}

#[tokio::test]
async fn expired_uncertain_creation_is_recovered_with_original_profile_before_cleanup() {
    let store = Memory::default();
    let body = json!({"title":"original title","first_message":"original input","agent_profile":{"id":"agp_original","revision":2}});
    store
        .save(
            serde_json::to_value(RunState {
                create_intent: Some(("api/v1/workspaces/wks_original/agents".into(), body.clone())),
                input: format!("{:x}", Sha256::digest(b"data")),
                deadline: now().saturating_sub(1),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![
        reply(json!({"id":"agt_recovered"})),
        accepted(),
        accepted(),
    ]);
    assert!(
        client
            .answer(&store, "data".into(), Duration::from_secs(60))
            .await
            .is_err()
    );
    let calls = fake.calls.lock().await;
    assert_eq!(calls[0].1, "api/v1/workspaces/wks_original/agents");
    assert_eq!(calls[0].2, Some(body));
    assert_eq!(
        calls[0].3.as_deref(),
        Some("wishpool:paper:r1:audit:a0:create")
    );
    assert!(calls[1].1.ends_with("/stop"));
    assert_eq!(calls[3].0, "DELETE");
    let state: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(state.deleted);
}

#[tokio::test]
async fn changed_inputs_fail_without_resubmission_and_cleanup_the_saved_agent() {
    let store = Memory::default();
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_old".into()),
                input: format!("{:x}", Sha256::digest(b"original data")),
                deadline: now() + 60,
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![accepted(), accepted()]);
    assert!(
        client
            .answer(&store, "changed data".into(), Duration::from_secs(60))
            .await
            .unwrap_err()
            .to_string()
            .contains("inputs changed")
    );
    let calls = fake.calls.lock().await;
    assert_eq!(calls.len(), 4);
    assert!(calls[0].1.ends_with("/stop"));
    assert_eq!(calls[2].0, "DELETE");
}

#[tokio::test]
async fn restart_keeps_the_original_deadline_and_cancels_expired_work() {
    let store = Memory::default();
    let prompt = "unchanged data";
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_expired".into()),
                response_id: Some("resp_expired".into()),
                input: format!("{:x}", Sha256::digest(prompt.as_bytes())),
                deadline: now().saturating_sub(1),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![
        reply(
            json!({"target_active":true,"target":{"turn_sequence":2},"agent_revision":9,"runtime_support":{"cancel":true}}),
        ),
        accepted(),
        accepted(),
        accepted(),
    ]);
    assert!(
        client
            .answer(&store, prompt.into(), Duration::from_secs(3600))
            .await
            .is_err()
    );
    let state: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(state.deleted && state.failure.unwrap().contains("deadline"));
    assert!(
        fake.calls
            .lock()
            .await
            .iter()
            .all(|c| !c.1.ends_with("/responses") && !c.1.ends_with("/agents"))
    );
}
#[cfg(unix)]
#[tokio::test]
async fn cli_stream_receipt_is_saved_without_waiting_for_completion_and_body_file_is_removed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("fake-nyxid");
    let scratch = dir.path().join("scratch");
    std::fs::write(
        &program,
        r#"#!/bin/sh
case "$*" in *'--stream'*) ;; *) exit 3;; esac
while [ "$#" -gt 0 ]; do
  case "$1" in --data) shift; body_path="${1#@}";; esac
  shift
done
[ -f "$body_path" ] || exit 4
printf 'event: response.created\ndata: {"type":"response.created","response":{"id":"resp_cli"}}\n\n'
exec sleep 10
"#,
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let transport = CliTransport {
        program,
        work_dir: scratch.clone(),
    };
    let reply = tokio::time::timeout(
        Duration::from_secs(2),
        transport.request(
            "POST",
            "api/v2/agents/agt_test/responses",
            Some(json!({"input":"paper source","stream":true})),
            Some("durable-key"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reply.response_id().unwrap(), "resp_cli");
    assert!(std::fs::read_dir(scratch).unwrap().next().is_none());
}

#[test]
fn typed_refusals_exclude_transient_and_ambiguous_requests() {
    for status in [429, 500, 502, 503] {
        assert_eq!(
            failure_kind(&ReviewError::Provider {
                status,
                body: "quota_exceeded".into()
            }),
            None
        );
    }
    assert_eq!(
        failure_kind(&ReviewError::Transport("uncertain write".into())),
        None
    );
    assert_eq!(failure_kind(&invalid("invalid mathematical JSON")), None);
    assert_eq!(
        failure_kind(&ReviewError::Provider {
            status: 409,
            body: "conflict".into()
        }),
        None
    );
    for (status, code, kind) in [
        (401, "not_authenticated", FailureKind::Auth),
        (409, "sandbox_credential_setup_required", FailureKind::Setup),
        (403, "credential_revoked", FailureKind::Credential),
        (409, "agent_resource_quota", FailureKind::Quota),
    ] {
        assert_eq!(
            failure_kind(&ReviewError::Provider {
                status,
                body: code.into()
            }),
            Some(kind)
        );
    }
}
#[tokio::test]
async fn terminal_agent_failure_retains_typed_reason_across_restarts() {
    let (client, fake) = client(vec![
        reply(json!({"id":"agt_test"})),
        receipt("resp_test"),
        reply(json!({"status":"failed","error":{"message":"untrusted provider diagnostic"}})),
        reply(json!({"target_active":false,"target":{}})),
        accepted(),
        accepted(),
    ]);
    let store = Memory::default();
    let result = client
        .answer(&store, "input".into(), Duration::from_secs(30))
        .await;
    assert!(matches!(
        result,
        Err(ReviewError::CmaRefusal(FailureKind::CauseUnknown))
    ));
    let state = store.load().await.unwrap().unwrap();
    assert_eq!(state["typed_failure"], "cause_unknown");
    assert!(!state.to_string().contains("untrusted provider diagnostic"));
    let calls = fake.calls.lock().await.len();
    assert!(matches!(
        client
            .answer(&store, "input".into(), Duration::from_secs(30))
            .await,
        Err(ReviewError::CmaRefusal(FailureKind::CauseUnknown))
    ));
    assert_eq!(fake.calls.lock().await.len(), calls);
}

#[tokio::test]
async fn asynchronous_stop_defers_delete_until_observed_and_retains_the_answer() {
    let store = Memory::default();
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_stopping".into()),
                answer: Some("{\"summary\":\"done\"}".into()),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![accepted(), accepted()]);
    fake.agent_observations.lock().await.extend([
        reply(json!({"status":"starting"})),
        reply(json!({"status":"stopped"})),
    ]);
    client.cancel(&store).await.unwrap();
    let first: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(first.stopped && !first.deleted);
    assert!(fake.calls.lock().await.iter().all(|c| c.0 != "DELETE"));
    client.cancel(&store).await.unwrap();
    let second: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(second.deleted);
    assert_eq!(second.answer, first.answer);
    assert!(second.failure.is_none());
    let calls = fake.calls.lock().await;
    assert_eq!(calls.iter().filter(|c| c.1.ends_with("/stop")).count(), 1);
    assert_eq!(calls.iter().filter(|c| c.0 == "DELETE").count(), 1);
}

#[tokio::test]
async fn delete_acknowledgment_keeps_the_handle_until_absence_is_observed() {
    let store = Memory::default();
    store
        .save(
            serde_json::to_value(RunState {
                agent_id: Some("agt_deleting".into()),
                answer: Some("{}".into()),
                ..Default::default()
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let (client, fake) = client(vec![accepted(), accepted(), accepted()]);
    fake.agent_observations.lock().await.extend([
        reply(json!({"status":"stopped"})),
        reply(json!({"status":"stopped"})),
        reply(json!({"status":"stopped"})),
        Err(ReviewError::Provider {
            status: 404,
            body: "not_found".into(),
        }),
    ]);
    client.cancel(&store).await.unwrap();
    let first: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(!first.deleted);
    assert_eq!(first.agent_id.as_deref(), Some("agt_deleting"));
    client.cancel(&store).await.unwrap();
    let last: RunState = serde_json::from_value(store.load().await.unwrap().unwrap()).unwrap();
    assert!(last.deleted);
    assert_eq!(last.answer, first.answer);
    let calls = fake.calls.lock().await;
    let deletes: Vec<_> = calls.iter().filter(|c| c.0 == "DELETE").collect();
    assert_eq!(deletes.len(), 2);
    assert_eq!(deletes[0], deletes[1]);
}
