use std::{collections::BTreeSet, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
};
use mathesis_core::{
    app::App,
    memory::MemoryStores,
    model::{Caller, ClaimKind, VerifiedIdentity},
    policy::Policy,
    ports::{PaperReader, ReadPaper, ReadStatement, SystemClock},
};
use tower::ServiceExt;

use crate::AuthenticatedCaller;

/// Test-only authentication: `x-test-subject` names the caller.
async fn test_auth(
    axum::extract::State(app): axum::extract::State<Arc<App>>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(subject) = request
        .headers()
        .get("x-test-subject")
        .and_then(|v| v.to_str().ok())
    {
        let identity = VerifiedIdentity {
            subject: subject.into(),
            name: Some(subject.into()),
            email: None,
            picture: None,
        };
        let caller: Caller = app.caller(&identity).await.unwrap();
        request.extensions_mut().insert(AuthenticatedCaller(caller));
    }
    next.run(request).await
}

/// Reads any upload as a paper with one theorem and one lemma.
struct OneTheorem;

impl PaperReader for OneTheorem {
    fn read(&self, bytes: &[u8], _filename: &str) -> Result<ReadPaper, String> {
        if bytes.starts_with(b"garbage") {
            return Err("no main .tex file".into());
        }
        let statement = |kind, name: &str| ReadStatement {
            kind,
            display_name: name.into(),
            title: None,
            latex_label: None,
            body: "$x$".into(),
            has_proof: true,
            section: None,
        };
        Ok(ReadPaper {
            evidence: vec![],
            main_file: "main.tex".into(),
            title: Some("On gaps".into()),
            authors: vec!["A. Author".into()],
            abstract_text: Some("We bound gaps.".into()),
            statements: vec![
                statement(ClaimKind::Theorem, "Theorem"),
                statement(ClaimKind::Lemma, "Lemma"),
            ],
            macros: Default::default(),
            warnings: vec![],
        })
    }
}

fn router() -> Router {
    let stores = Arc::new(MemoryStores::default());
    let app = App::new(
        stores.ports(Arc::new(SystemClock), Arc::new(OneTheorem)),
        Policy::default(),
        BTreeSet::new(),
    );
    crate::router(app.clone()).layer(middleware::from_fn_with_state(app, test_auth))
}

async fn send(router: &Router, request: Request) -> (StatusCode, serde_json::Value, String) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json, content_type)
}

async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    subject: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, String) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(subject) = subject {
        builder = builder.header("x-test-subject", subject);
    }
    let request = match body {
        Some(json) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string())),
        None => builder.body(Body::empty()),
    }
    .unwrap();
    send(router, request).await
}

const BOUNDARY: &str = "wishpool-test-boundary";

fn multipart(parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, filename, content) in parts {
        body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        match filename {
            Some(f) => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
            ),
            None => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            ),
        }
        body.extend_from_slice(content);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

async fn upload(
    router: &Router,
    subject: &str,
    source: &[u8],
) -> (StatusCode, serde_json::Value, String) {
    let metadata = serde_json::json!({
        "processing_consent":"ai-review-2026-10-11", "ai_disclosure": { "level": "assisted", "statement": "A model proofread section 2." },
        "open_to_contributors": true
    })
    .to_string();
    let body = multipart(&[
        ("metadata", None, metadata.as_bytes()),
        ("source", Some("paper.tex"), source),
    ]);
    let request = Request::builder()
        .method("POST")
        .uri("/submissions")
        .header("x-test-subject", subject)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap();
    send(router, request).await
}

#[tokio::test]
async fn policy_is_public() {
    let router = router();
    let (status, json, _) = call(&router, "GET", "/policy", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["policy"]["max_active_per_author"], 3);
    assert_eq!(json["stages"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn anonymous_me_is_a_problem_document() {
    let router = router();
    let (status, json, content_type) = call(&router, "GET", "/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(content_type, "application/problem+json");
    assert_eq!(json["code"], "not_authenticated");
}

#[tokio::test]
async fn upload_and_confirm_round_trip() {
    let router = router();
    let (status, json, _) = upload(&router, "author", b"\\documentclass{article}").await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["status"]["state"], "draft");
    assert_eq!(json["extracted"].as_array().unwrap().len(), 2);
    let id = json["id"].as_str().unwrap().to_owned();

    // Private to the author until accepted.
    let (status, _, _) = call(
        &router,
        "GET",
        &format!("/submissions/{id}"),
        Some("stranger"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(
        &router,
        "GET",
        &format!("/submissions/{id}/files/source"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(
        &router,
        "GET",
        &format!("/submissions/{id}/files/source"),
        Some("author"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let claims = serde_json::json!({ "claims": [
        { "id": "C1", "kind": "theorem", "role": "main" },
        { "id": "C2", "kind": "lemma", "role": "supporting", "depends_on": [] }
    ]});
    let (status, json, _) = call(
        &router,
        "POST",
        &format!("/submissions/{id}/claims"),
        Some("stranger"),
        Some(claims.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{json}");
    let (status, json, _) = call(
        &router,
        "POST",
        &format!("/submissions/{id}/claims"),
        Some("author"),
        Some(claims),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["status"]["state"], "in_review");
    assert_eq!(json["claims"].as_array().unwrap().len(), 2);

    // The author opted in: signed-in contributors see the tasks.
    let (status, _, _) = call(&router, "GET", "/tasks", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, json, _) = call(
        &router,
        "GET",
        &format!("/tasks?submission={id}"),
        Some("helper"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["items"].as_array().unwrap().len(), 3);
    let task = json["items"][0]["id"].as_str().unwrap().to_owned();
    let (status, json, _) = call(
        &router,
        "POST",
        &format!("/tasks/{task}/lease"),
        Some("helper"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    let (status, _, _) = call(
        &router,
        "POST",
        &format!("/tasks/{task}/lease"),
        Some("author"),
        Some(serde_json::json!({"mode": "hosted"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, json, _) = call(
        &router,
        "GET",
        &format!("/submissions/{id}/analysis"),
        Some("author"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["main_results"], 1);
    let (status, json, _) = call(&router, "GET", "/papers", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([]));
}

#[tokio::test]
async fn malformed_input_is_unprocessable() {
    let router = router();
    let (status, json, _) = upload(&router, "author", b"garbage").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json["code"], "invalid");
    assert!(json["detail"].as_str().unwrap().contains("no main .tex"));

    let (status, _, _) = call(
        &router,
        "POST",
        "/submissions/x/stages/nonsense/reports",
        Some("poser"),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = call(&router, "GET", "/submissions/x/files/zip", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn submission_queue_requires_a_review_role() {
    let router = router();
    let (status, _, _) = call(
        &router,
        "GET",
        "/submissions?scope=queue",
        Some("someone"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, json, _) = call(&router, "GET", "/submissions", Some("someone"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([]));
}

#[tokio::test]
async fn referee_routes_project_visibility_roles_and_letter_validation() {
    use mathesis_core::{
        ids::SubmissionId,
        model::{Role, RoundUpdate, Step, StepState},
    };
    let stores = Arc::new(MemoryStores::default());
    let app = App::new(
        stores.ports(Arc::new(SystemClock), Arc::new(OneTheorem)),
        Policy::default(),
        BTreeSet::new(),
    );
    let editor = app
        .ensure_service_account(&"editor".into(), "Editor", Role::Editor)
        .await
        .unwrap();
    let reviewer = app
        .ensure_service_account(&"referee".into(), "Referee", Role::Reviewer)
        .await
        .unwrap();
    let router =
        crate::router(app.clone()).layer(middleware::from_fn_with_state(app.clone(), test_auth));
    let (status, paper, _) = upload(&router, "author", b"paper").await;
    assert_eq!(status, StatusCode::CREATED);
    let id = SubmissionId::from(paper["id"].as_str().unwrap());
    let base = format!("/submissions/{id}/referee");
    let summary_path = format!("/submissions/{id}/review-summary");
    for (identity, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("stranger"), StatusCode::NOT_FOUND),
        (Some("author"), StatusCode::OK),
    ] {
        let (status, body, _) = call(&router, "GET", &summary_path, identity, None).await;
        assert_eq!(status, expected);
        if status == StatusCode::OK {
            assert!(body["round"].is_null());
            assert_eq!(body["interrupted"], false);
            assert!(body.get("rounds").is_none());
        }
    }

    assert_eq!(
        call(&router, "GET", &base, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&router, "GET", &base, Some("stranger"), None).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, view, _) = call(&router, "GET", &base, Some("author"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["rounds"], serde_json::json!([]));
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{base}/restart"),
            Some("author"),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let letters = format!("{base}/letters");
    assert_eq!(
        call(
            &router,
            "POST",
            &letters,
            Some("author"),
            Some(serde_json::json!({"subject":"s","body":"b"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &letters,
            Some("editor"),
            Some(serde_json::json!({"subject":"s","body":" "}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, letter, _) = call(
        &router,
        "POST",
        &letters,
        Some("editor"),
        Some(serde_json::json!({"subject":"Feedback","body":"A useful observation."})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(letter["note"], "");
    assert_eq!(letter["edited"], true);
    app.record_compilation(&reviewer, &id, 1, Ok(b"%PDF".to_vec()))
        .await
        .unwrap();
    let caller = app
        .caller(&VerifiedIdentity {
            subject: "author".into(),
            name: None,
            email: None,
            picture: None,
        })
        .await
        .unwrap();
    let submission = app.submission(&caller, &id).await.unwrap();
    let confirmations = submission
        .extracted
        .iter()
        .map(|c| mathesis_core::model::ClaimConfirmation {
            depends_on_conjectures: vec![],
            id: c.id.clone(),
            kind: c.kind,
            role: c.role,
            depends_on: vec![],
            settles: None,
            excluded: false,
        })
        .collect();
    app.confirm_claims(&caller, &id, confirmations)
        .await
        .unwrap();
    app.begin_referee_round(&reviewer, &id).await.unwrap();
    let resume = format!("{base}/resume");
    for (subject, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("author"), StatusCode::FORBIDDEN),
        (Some("editor"), StatusCode::ACCEPTED),
    ] {
        assert_eq!(
            call(
                &router,
                "POST",
                &resume,
                subject,
                Some(serde_json::json!({"round":1}))
            )
            .await
            .0,
            expected
        );
    }
    assert_eq!(
        call(
            &router,
            "POST",
            &resume,
            Some("editor"),
            Some(serde_json::json!({"round":2}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    {
        use mathesis_core::{model::*, ports::BlobStore};
        let subject = app
            .knowledge_subject(&reviewer, &id, 1, &"C1".into())
            .await
            .unwrap();
        let digest = format!("sha256:{}", "a".repeat(64));
        let mut intent = KnowledgeQueryIntent {
            id: "query-1".into(),
            subject: subject.clone(),
            snapshot: Some(digest.clone()),
            index_revision: Some(digest),
            request_digest: String::new(),
            query: "bounded fixture query".into(),
            scopes: vec!["fixture".into(), KNOWLEDGE_RANKER_SCOPE.into()],
            admitted_at: mathesis_core::ports::Clock::now(&SystemClock),
        };
        intent.request_digest = KnowledgeQueryReceipt::request_digest(
            &subject,
            &intent.snapshot,
            &intent.index_revision,
            &intent.query,
            &intent.scopes,
        );
        app.record_knowledge_event(
            &reviewer,
            &id,
            KnowledgeEvent::QueryAdmitted {
                intent: intent.clone(),
            },
        )
        .await
        .unwrap();
        let response = stores
            .put(br#"{"hits":[],"private_fixture":true}"#, "application/json")
            .await
            .unwrap();
        let receipt = KnowledgeQueryReceipt {
            id: intent.id,
            protocol: KNOWLEDGE_QUERY_PROTOCOL.into(),
            subject,
            snapshot: intent.snapshot,
            index_revision: intent.index_revision,
            request_digest: intent.request_digest,
            query: intent.query,
            scopes: intent.scopes,
            state: KnowledgeQueryState::Complete,
            response: Some(response),
            returned: vec![],
            limits: vec![],
            retryable: false,
            started_at: intent.admitted_at,
            completed_at: mathesis_core::ports::Clock::now(&SystemClock),
        };
        app.record_knowledge_event(&reviewer, &id, KnowledgeEvent::QueryCompleted { receipt })
            .await
            .unwrap();
        let path = format!("{base}/knowledge/query-1");
        let (status, response, headers) = call(&router, "GET", &path, Some("author"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["private_fixture"], true);
        assert_eq!(headers, "application/json");
        let raw = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .header("x-test-subject", "author")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(raw.headers()["cache-control"], "private, no-store");
        assert_eq!(
            call(&router, "GET", &path, Some("editor"), None).await.0,
            StatusCode::OK
        );
        assert_eq!(
            call(&router, "GET", &path, Some("stranger"), None).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            call(&router, "GET", &path, None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(
                &router,
                "GET",
                &format!("{base}/knowledge/missing"),
                Some("author"),
                None
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }

    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{base}/restart"),
            Some("editor"),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut referee = Step::pending();
    referee.state = StepState::Failed {
        reason: "failed".into(),
        detail: None,
        retryable: false,
        at: submission.created_at,
    };
    app.update_referee_round(&reviewer, &id, 1, RoundUpdate::Referee(referee))
        .await
        .unwrap();
    let mut audit = Step::pending();
    audit.state = StepState::Skipped {
        reason: "failed referee".into(),
    };
    app.update_referee_round(&reviewer, &id, 1, RoundUpdate::Audit(audit))
        .await
        .unwrap();
    let mut advice = Step::pending();
    advice.state = StepState::Skipped {
        reason: "failed referee".into(),
    };
    app.update_referee_round(&reviewer, &id, 1, RoundUpdate::Advice(advice))
        .await
        .unwrap();
    let mut letter = Step::pending();
    letter.state = StepState::Skipped {
        reason: "failed referee".into(),
    };
    app.update_referee_round(&reviewer, &id, 1, RoundUpdate::Letter(letter))
        .await
        .unwrap();
    let mut formal = Step::pending();
    formal.state = StepState::Skipped {
        reason: "failed referee".into(),
    };
    app.update_referee_round(&reviewer, &id, 1, RoundUpdate::Formal(formal))
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            "POST",
            &resume,
            Some("editor"),
            Some(serde_json::json!({"round":1}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, file, _) = call(
        &router,
        "POST",
        &format!("{base}/restart"),
        Some("editor"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(file["rounds"].as_array().unwrap().len(), 2);
    let (_, view, _) = call(&router, "GET", &base, Some("author"), None).await;
    assert_eq!(view["rounds"].as_array().unwrap().len(), 2);
    assert!(view["rounds"][0].get("advice").is_none());
    assert!(view["rounds"][0].get("letter").is_none());
    assert_eq!(view["letters"].as_array().unwrap().len(), 1);
    assert_eq!(app.referee(&editor, &id).await.unwrap().rounds.len(), 2);
}

#[tokio::test]
async fn solving_routes_are_public_where_intended_and_agents_require_ownership() {
    use serde_json::json;
    let r = router();
    let (status, body, _) = call(&r, "GET", "/leaderboard", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
    let (status, _, _) = call(&r, "GET", "/leaderboard?period=week", None, None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = call(
        &r,
        "POST",
        "/me/agents",
        None,
        Some(json!({"name":"Agent"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, agent, _) = call(
        &r,
        "POST",
        "/me/agents",
        Some("owner"),
        Some(json!({"name":"Agent"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(agent["owner"]["id"], "owner");
    let id = agent["id"].as_str().unwrap();
    let path = format!("/me/agents/{id}");
    let (status, _, _) = call(
        &r,
        "PATCH",
        &path,
        Some("other"),
        Some(json!({"name":"stolen"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, renamed, _) = call(
        &r,
        "PATCH",
        &path,
        Some("owner"),
        Some(json!({"name":"Renamed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], "Renamed");
    let (status, profile, _) = call(&r, "GET", &format!("/entrants/{id}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(profile.get("email").is_none());
    let (status, retired, _) = call(&r, "DELETE", &path, Some("owner"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(retired["retired"], true);
    let (status, _, _) = call(&r, "GET", "/attempts/unknown", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(
        &r,
        "POST",
        "/conjectures/WP-2026-0001/C1/attempts",
        Some("owner"),
        Some(json!({"solution":"proof","receipt":{"verdict":"proved"}})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = call(
        &r,
        "POST",
        "/submissions/unknown/open-problems/check",
        Some("owner"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn uploads_require_explicit_current_consent_and_retired_proof_routes_are_absent() {
    let router = router();
    for consent in [None, Some("old-version")] {
        let mut metadata =
            serde_json::json!({"ai_disclosure":{"level":"none","statement":"No AI"}});
        if let Some(consent) = consent {
            metadata["processing_consent"] = consent.into();
        }
        let text = metadata.to_string();
        let body = multipart(&[
            ("metadata", None, text.as_bytes()),
            ("source", Some("paper.tex"), b"source"),
        ]);
        let request = Request::builder()
            .method("POST")
            .uri("/submissions")
            .header("x-test-subject", "author")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body))
            .unwrap();
        let (status, json, _) = send(&router, request).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(json["detail"].as_str().unwrap().contains("agreement"));
    }
    for (method, path) in [
        ("PUT", "/submissions/s/formalization/repository"),
        ("POST", "/submissions/s/formalization/items/C1/start"),
        ("POST", "/submissions/s/formalization/items/C1/verification"),
    ] {
        let (status, _, _) = call(
            &router,
            method,
            path,
            Some("editor"),
            Some(serde_json::json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn research_service_contract_and_formalization_permissions() {
    let app = router();
    let (status, spec, _) = call(&app, "GET", "/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(
        spec["paths"]["/api/v1/papers"]["get"]["operationId"],
        "search_papers"
    );
    assert_eq!(
        spec["paths"]["/api/v1/formalizations"]["post"]["security"][0]["nyxid"],
        serde_json::json!([])
    );
    assert_eq!(
        call(&app, "GET", "/papers?q=%2A", None, None).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(
            &app,
            "GET",
            "/papers?msc=11&kind=paper&author=Smith",
            None,
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let input = serde_json::json!({"title":"Lean contribution", "authors":["Original author"],"statement":"True", "source_reference":{"kind":"url","locator":"https://example.org/source.lean"},"toolchain":"leanprover/lean4:v4.24.0", "license":"Apache-2.0", "solution":"import Target\ntheorem wishpool_solution : wishpool_target_prop := True.intro"});
    assert_eq!(
        call(&app, "POST", "/formalizations", None, Some(input.clone()))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, deposit, _) = call(
        &app,
        "POST",
        "/formalizations",
        Some("owner"),
        Some(input.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(deposit["authors"][0], "Original author");
    assert!(deposit["attempts"].as_array().unwrap().is_empty());
    let id = deposit["id"].as_str().unwrap();
    assert_eq!(
        call(&app, "POST", "/formalizations", Some("owner"), Some(input))
            .await
            .1["id"],
        id
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/formalizations/{id}"),
            Some("stranger"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/formalizations/{id}/source"),
            Some("stranger"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let listing = call(&app, "GET", "/formalizations", Some("owner"), None)
        .await
        .1;
    assert_eq!(listing["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        call(
            &app,
            "GET",
            "/submissions/private/review",
            Some("stranger"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn project_export_preserves_sources_and_never_exposes_private_deposits() {
    let app = router();
    let input = serde_json::json!({"title":"Lean project", "authors":["Research author"], "formalizers":["Lean author"], "source_revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "statement":"True", "source_reference":{"kind":"url","locator":"https://example.org/source"},"toolchain":"pinned", "license":"MIT", "solution":"import Target\nimport Helpers", "modules":{"Helpers":"import Mathlib\ntheorem helper : True := True.intro"}});
    let (status, deposit, _) = call(
        &app,
        "POST",
        "/formalizations",
        Some("owner"),
        Some(input.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let path = format!(
        "/formalizations/{}/project",
        deposit["id"].as_str().unwrap()
    );
    assert_eq!(
        call(&app, "GET", &path, Some("stranger"), None).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, project, _) = call(&app, "GET", &path, Some("owner"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(project["modules"], input["modules"]);
    assert_eq!(project["formalizers"], input["formalizers"]);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("x-test-subject", "owner")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/formalizations",
            Some("owner"),
            Some(project)
        )
        .await
        .1["id"],
        deposit["id"]
    );
    let directory = call(&app, "GET", "/formalizations/opportunities", None, None).await;
    assert_eq!(directory.0, StatusCode::OK);
    assert_eq!(directory.1["items"], serde_json::json!([]));
    assert_eq!(
        call(
            &app,
            "GET",
            "/formalizations/opportunities?status=queued",
            None,
            None
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn private_review_reports_are_downloadable_only_by_author_and_responses_are_fenced() {
    let app = router();
    let metadata = serde_json::json!({"purpose":"review_only", "review_scope":"Boundary cases", "make_public_after_acceptance":true, "processing_consent":"ai-review-2026-10-11", "ai_disclosure":{"level":"none","statement":"None."}}).to_string();
    let body = multipart(&[
        ("metadata", None, metadata.as_bytes()),
        ("source", Some("paper.tex"), b"source"),
    ]);
    let request = Request::builder()
        .method("POST")
        .uri("/submissions")
        .header("x-test-subject", "author")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap();
    let (status, paper, _) = send(&app, request).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(paper["purpose"], "review_only");
    assert_eq!(paper["analysis_visibility"], "private");
    let id = paper["id"].as_str().unwrap();
    let report = call(
        &app,
        "GET",
        &format!("/submissions/{id}/review"),
        Some("author"),
        None,
    )
    .await
    .1;
    assert_eq!(report["schema_version"], 3);
    assert_eq!(report["research"]["obligations"], serde_json::json!([]));
    let research = format!("/submissions/{id}/research");
    assert_eq!(
        call(&app, "GET", &research, Some("author"), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", &research, Some("outsider"), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "GET", &research, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &research,
            Some("author"),
            Some(serde_json::json!({"consent":"research-obligation-2026-10-16"}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(report["review_scope"], "Boundary cases");
    let path = format!("/submissions/{id}/review.md");
    let request = Request::builder()
        .uri(&path)
        .header("x-test-subject", "author")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/markdown; charset=utf-8"
    );
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    assert!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .contains("Boundary cases")
    );
    assert_eq!(
        call(&app, "GET", &path, Some("stranger"), None).await.0,
        StatusCode::NOT_FOUND
    );
    let path = format!("/submissions/{id}/review/responses");
    let body = serde_json::json!({"round":1,"concern":0,"version":1,"claims_revision":0,"body":"Please recheck."});
    assert_eq!(
        call(&app, "POST", &path, Some("stranger"), Some(body.clone()))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "POST", &path, Some("author"), Some(body))
            .await
            .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn knowledge_capability_boundary_is_public_and_never_claims_an_unavailable_checker() {
    let (status, body, _) = call(&router(), "GET", "/knowledge/capabilities", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["source_query_protocol"], "mathesis.artifact-query.v1");
    assert_eq!(body["coverage_certificates"]["supported"], false);
    assert_eq!(
        body["coverage_certificates"]["recognized_protocols"],
        serde_json::json!([])
    );
    assert_eq!(
        body["coverage_certificates"]["ordinary_lean_establishes_coverage"],
        false
    );
    assert_eq!(
        body["coverage_certificates"]["reason_code"],
        "no_admitted_coverage_checker"
    );
}

#[tokio::test]
async fn binding_routes_hide_private_inspections_reject_client_verdicts_and_enforce_revocation() {
    use mathesis_core::ports::Clock;
    use mathesis_core::{
        ids::SubmissionId,
        model::*,
        ports::{RefereeStore, SubmissionStore},
    };
    let stores = Arc::new(MemoryStores::default());
    let app = App::new(
        stores.ports(Arc::new(SystemClock), Arc::new(OneTheorem)),
        Policy::default(),
        BTreeSet::new(),
    );
    let router =
        crate::router(app.clone()).layer(middleware::from_fn_with_state(app.clone(), test_auth));
    let (_, uploaded, _) = upload(&router, "author", b"paper").await;
    let id = SubmissionId::from(uploaded["id"].as_str().unwrap());
    let mut paper = SubmissionStore::get(stores.as_ref(), &id)
        .await
        .unwrap()
        .unwrap();
    paper.status = SubmissionStatus::Accepted {
        record: "WP-2026-0001".into(),
    };
    paper.claims = paper.extracted.clone();
    paper.claims_revision = 1;
    paper.analysis_visibility = Visibility::Public;
    paper.open_to_contributors = true;
    SubmissionStore::replace(stores.as_ref(), &paper, paper.revision)
        .await
        .unwrap();
    let hash = |v: &[u8]| {
        format!(
            "sha256:{}",
            lean_solution_digest(std::str::from_utf8(v).unwrap(), &Default::default())
        )
    };
    let claim = &paper.claims[0];
    let target = BindingTarget {
        subject: CoverageSubject {
            submission: id.clone(),
            version: 1,
            claims_revision: paper.claims_revision,
            round: 1,
            claim: claim.id.clone(),
            source_digest: format!("sha256:{}", paper.current_version().unwrap().archive.sha256),
            claim_digest: hash(&serde_json::to_vec(claim).unwrap()),
        },
        statement: claim.statement.clone(),
        sources: vec![KnowledgeDeclarationRef {
            snapshot: hash(b"snapshot"),
            module_path: "Fixture.lean".into(),
            declaration_name_key: "premise".into(),
            statement_id: hash(b"premise"),
            type_digest: hash(b"type"),
        }],
    };
    let b = BindingBoard {
        id: "board".into(),
        target: target.clone(),
        context: "Explicit public fixture".into(),
        license: BINDING_LICENSE.into(),
        consent: BINDING_SHARING_CONSENT.into(),
        authorized_by: "author".into(),
        authorized_at: SystemClock.now(),
        receipt_ids: vec!["PRIVATE query id".into()],
        events: vec![BindingEvent::Audited {
            audit: BindingAudit {
                request: "fixture".into(),
                contribution: "fixture".into(),
                auditor: "auditor".into(),
                verdict: BindingVerdict::Unclear,
                reason: "PRIVATE manuscript audit".into(),
                evidence: "PRIVATE calculations".into(),
                inspected_sources: vec![],
                evidence_complete: false,
                at: SystemClock.now(),
            },
        }],
    };
    let mut file = RefereeFile::new(id.clone());
    file.binding_boards.push(b);
    RefereeStore::insert(stores.as_ref(), &file).await.unwrap();
    let private = format!("/submissions/{id}/bindings");
    let public = format!("/bindings/{id}/board");
    assert_eq!(
        call(&router, "GET", &private, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&router, "GET", &private, Some("outsider"), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (status, body, _) = call(&router, "GET", &private, Some("author"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.to_string().contains("PRIVATE manuscript"));
    let (status, body, _) = call(&router, "GET", &public, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.to_string().contains("PRIVATE"));
    let input = serde_json::json!({"binding":{"premises":target.sources,"relation":"implies","parameters":[],"hypotheses":[],"argument":"Engineering fixture argument","unresolved":[],"source_lineage":[]},"license":BINDING_LICENSE,"consent":BINDING_CONTRIBUTION_CONSENT});
    let mut forged = input.clone();
    forged["verdict"] = serde_json::json!("supported");
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{public}/contributions"),
            Some("outsider"),
            Some(forged)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{public}/contributions"),
            Some("author"),
            Some(input.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{public}/contributions"),
            Some("outsider"),
            Some(input.clone())
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, profile, _) = call(&router, "GET", "/binding-contributors/outsider", None, None).await;
    assert_eq!(profile["items"].as_array().unwrap().len(), 1);
    assert_eq!(profile["items"][0]["credited"], false);
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{private}/board/revoke"),
            Some("outsider"),
            Some(serde_json::json!({"reason":"no authority"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{private}/board/revoke"),
            Some("author"),
            Some(serde_json::json!({"reason":"Author revokes sharing"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&router, "GET", &public, None, None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{public}/contributions"),
            Some("outsider"),
            Some(input)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, profile, _) = call(&router, "GET", "/binding-contributors/outsider", None, None).await;
    assert!(profile["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn feedback_routes_require_exact_rights_and_never_accept_an_observation_verdict() {
    use mathesis_core::{
        ids::SubmissionId,
        model::*,
        ports::{RecordStore, SubmissionStore},
    };
    let stores = Arc::new(MemoryStores::default());
    let app = App::new(
        stores.ports(Arc::new(SystemClock), Arc::new(OneTheorem)),
        Policy::default(),
        BTreeSet::new(),
    );
    let router =
        crate::router(app.clone()).layer(middleware::from_fn_with_state(app.clone(), test_auth));
    let (_, uploaded, _) = upload(&router, "author", b"paper").await;
    let id = SubmissionId::from(uploaded["id"].as_str().unwrap());
    let mut p = SubmissionStore::get(stores.as_ref(), &id)
        .await
        .unwrap()
        .unwrap();
    p.status = SubmissionStatus::Accepted {
        record: "WP-2026-0001".into(),
    };
    p.analysis_visibility = Visibility::Public;
    p.claims = p.extracted.clone();
    p.claims_revision = 1;
    p.reports.push(StageReport {
        stage: Stage::Escape,
        outcome: Outcome::Pass,
        summary: "PRIVATE summary".into(),
        payload: StagePayload::Escape {
            assessments: vec![EscapeAssessment {
                methodology: None,
                conjecture: None,
                correctness: Some(Correctness::Correct),
                claim: p.claims[0].id.clone(),
                shape: ProofShape::Content,
                witnesses: vec!["Public content witness".into()],
                rationale: "PRIVATE rationale".into(),
                escape_rate: None,
            }],
        },
        evidence: vec![],
        reviewer: ReviewerIdentity::Human {
            person: "fixture-editor".into(),
        },
        claims_revision: 1,
        filed_at: mathesis_core::ports::Clock::now(&SystemClock),
    });
    SubmissionStore::replace(stores.as_ref(), &p, p.revision)
        .await
        .unwrap();
    RecordStore::insert(
        stores.as_ref(),
        &Record {
            kind: p.kind,
            id: "WP-2026-0001".into(),
            publication: None,
            submission: id.clone(),
            title: p.title.clone(),
            authors: p.authors.clone(),
            basis: mathesis_core::policy::AdmissionBasis::EscapeWitness,
            accepted_by: "fixture-editor".into(),
            accepted_at: mathesis_core::ports::Clock::now(&SystemClock),
        },
    )
    .await
    .unwrap();
    let path = format!("/submissions/{id}/feedback");
    let kind =
        serde_json::json!({"kind":"witness_formalization","claim":p.claims[0].id,"witness":0});
    assert_eq!(
        call(&router, "GET", &path, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{path}/preview"),
            Some("outsider"),
            Some(kind.clone())
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status, preview, _) = call(
        &router,
        "POST",
        &format!("{path}/preview"),
        Some("author"),
        Some(kind.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!preview.to_string().contains("PRIVATE"));
    let (status,v,_)=call(&router,"POST",&path,Some("author"),Some(serde_json::json!({"kind":kind,"preview_digest":preview["digest"],"preparation_consent":FEEDBACK_PREPARATION_CONSENT}))).await;
    assert_eq!(status, StatusCode::OK);
    let key = v["proposal"]["id"].as_str().unwrap();
    let item = format!("{path}/{key}");
    assert_eq!(
        call(&router, "GET", &item, Some("outsider"), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{item}/export"),
            Some("author"),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let grant = serde_json::json!({"digest":preview["digest"],"license":FEEDBACK_LICENSE,"consent":FEEDBACK_EXPORT_CONSENT,"rights_asserted":true});
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{item}/authorize"),
            Some("author"),
            Some(grant)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, export, _) = call(
        &router,
        "POST",
        &format!("{item}/export"),
        Some("author"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(export["payload"], preview["payload"]);
    assert!(!export.to_string().contains("PRIVATE"));
    assert_eq!(call(&router,"POST",&format!("{item}/readback"),Some("author"),Some(serde_json::json!({"pull_request":9,"index_revision":format!("sha256:{}","a".repeat(64)),"declarations":[],"observation_key":"x","merged":true}))).await.0,StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{item}/observed"),
            Some("author"),
            Some(serde_json::json!({"accepted":true}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{item}/revoke"),
            Some("author"),
            Some(serde_json::json!({"reason":"Stop new exports"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{item}/export"),
            Some("author"),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn manuscript_delivery_http_resumes_and_confirms_only_its_source() {
    let router = router();
    let metadata=serde_json::json!({"schema":"mathesis.manuscript-delivery.v1","producer":"paper","work_id":"note-a","version":"1",
        "paper":{"kind":"note","purpose":"review_only","title":"A note","authors":[{"name":"A. Author"}],"processing_consent":"ai-review-2026-10-11","ai_disclosure":{"level":"assisted","statement":"Proofreading."}},
        "origin":{"problem_ids":["problem-a"],"source_url":"https://example.org/note","license":"CC-BY-4.0"}}).to_string();
    let body = multipart(&[
        ("metadata", None, metadata.as_bytes()),
        ("source", Some("note.tex"), b"source"),
    ]);
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/manuscript-deliveries")
            .header("x-test-subject", "author")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body.clone()))
            .unwrap()
    };
    let (status, receipt, _) = send(&router, request()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(receipt["next_action"], "confirm_claims");
    let (_, again, _) = send(&router, request()).await;
    assert_eq!(again["submission"], receipt["submission"]);
    let id = receipt["submission"].as_str().unwrap();
    assert_eq!(
        call(
            &router,
            "GET",
            &format!("/submissions/{id}/delivery"),
            Some("stranger"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let claims=receipt["extracted"].as_array().unwrap().iter().map(|c|serde_json::json!({"id":c["id"],"kind":c["kind"],"role":c["role"],"excluded":false,"depends_on":[]})).collect::<Vec<_>>();
    let mut confirm = serde_json::json!({"version":1,"source_sha256":"wrong-source","expected_claims_revision":0,"claims":claims});
    let path = format!("/submissions/{id}/delivery-confirmation");
    assert_eq!(
        call(
            &router,
            "POST",
            &path,
            Some("author"),
            Some(confirm.clone())
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    confirm["source_sha256"] = receipt["source_sha256"].clone();
    let (status, confirmed, _) = call(
        &router,
        "POST",
        &path,
        Some("author"),
        Some(confirm.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["status"], "in_review");
    assert!(confirmed["publication"].is_null());
    let (_, again, _) = call(&router, "POST", &path, Some("author"), Some(confirm)).await;
    assert_eq!(
        again["confirmed_claims_revision"],
        confirmed["confirmed_claims_revision"]
    );
    let (_, found, _) = call(
        &router,
        "GET",
        "/manuscript-deliveries?producer=paper&work_id=note-a&purpose=review_only",
        Some("author"),
        None,
    )
    .await;
    assert_eq!(found["submission"], receipt["submission"]);
}

#[tokio::test]
async fn manuscript_json_transport_preserves_binary_source_bytes() {
    use base64::Engine;
    let router = router();
    let source = b"\x00\xffPK\x80binary source\n";
    let mut body = serde_json::json!({"metadata":{"schema":"mathesis.manuscript-delivery.v1","producer":"paper","work_id":"binary-note","version":"1","paper":{"kind":"note","title":"A note","authors":[{"name":"Author"}],"processing_consent":"ai-review-2026-10-11","ai_disclosure":{"level":"assisted","statement":"Proofreading."}},"origin":{"problem_ids":["problem-a"],"source_url":"https://example.org/note","license":"CC-BY-4.0"}},"filename":"note.zip","source_base64":base64::engine::general_purpose::STANDARD.encode(source)});
    let (status, receipt, _) = call(
        &router,
        "POST",
        "/manuscript-deliveries",
        Some("author"),
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = receipt["submission"].as_str().unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/submissions/{id}/files/source?version=1"))
                .header("x-test-subject", "author")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        &axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap()[..],
        source
    );
    body["source_base64"] = serde_json::json!("invalid base64");
    assert_eq!(
        call(
            &router,
            "POST",
            "/manuscript-deliveries",
            Some("author"),
            Some(body)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
