//! A small blocking client for the Mathesis contribution API.

use serde_json::{Value, json};

pub struct Client {
    http: reqwest::blocking::Client,
    base: String,
    token: String,
    nyxid_service: Option<String>,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: u16,
    pub detail: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.detail, self.status)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

impl Client {
    /// `base` is the venue origin, e.g. `https://mathesis.example.org`;
    /// `token` a NyxID access token (or `dev:<name>` against a dev server).
    pub fn new(base: &str, token: &str) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("mathesis-contribute/0.1")
            .build()
            .expect("HTTP client");
        Self {
            http,
            base: format!("{}/api/v1", base.trim_end_matches('/')),
            token: token.to_owned(),
            nyxid_service: None,
        }
    }

    pub fn via_nyxid(service: &str) -> Self {
        let mut client = Self::new("https://mathesis.invalid", "");
        client.nyxid_service = Some(service.to_owned());
        client
    }

    fn send(&self, request: reqwest::blocking::RequestBuilder) -> ApiResult<Value> {
        if let Some(service) = &self.nyxid_service {
            use std::{
                io::Write,
                process::{Command, Stdio},
            };
            let request = request.build().map_err(|e| ApiError {
                status: 0,
                detail: e.to_string(),
            })?;
            let url = request.url();
            let path = match url.query() {
                Some(q) => format!("{}?{q}", url.path()),
                None => url.path().to_owned(),
            };
            let mut command = Command::new("nyxid");
            command.args([
                "proxy",
                "request",
                service,
                &path,
                "--method",
                request.method().as_str(),
            ]);
            let body = request.body().and_then(|b| b.as_bytes());
            if body.is_some() {
                let content_type = request
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("application/json");
                command.args([
                    "--data",
                    "-",
                    "--header",
                    &format!("Content-Type:{content_type}"),
                ]);
            }
            // No shell, no token on argv, and request bodies travel through stdin.
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|_| ApiError {
                    status: 0,
                    detail: "Unable to start nyxid. Install the CLI and sign in first.".into(),
                })?;
            if let Some(body) = body {
                let write = child.stdin.take().unwrap().write_all(body);
                if let Err(e) = write {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ApiError {
                        status: 0,
                        detail: e.to_string(),
                    });
                }
            } else {
                drop(child.stdin.take());
            }
            let output = child.wait_with_output().map_err(|e| ApiError {
                status: 0,
                detail: e.to_string(),
            })?;
            if !output.status.success() {
                return Err(ApiError { status: 0, detail: "NyxID proxy request failed. Check your sign-in and connected Mathesis service with nyxid proxy discover.".into() });
            }
            return serde_json::from_slice(&output.stdout).map_err(|_| ApiError {
                status: 0,
                detail: "NyxID returned a non-JSON response".into(),
            });
        }

        let request = if self.token.is_empty() {
            request
        } else {
            request.bearer_auth(&self.token)
        };
        let response = request.send().map_err(|e| ApiError {
            status: 0,
            detail: e.to_string(),
        })?;
        let status = response.status().as_u16();
        let text = response.text().unwrap_or_default();
        let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            let detail = body["detail"].as_str().map(str::to_owned).unwrap_or(text);
            Err(ApiError { status, detail })
        }
    }

    pub fn get(&self, path: &str) -> ApiResult<Value> {
        self.send(self.http.get(format!("{}{path}", self.base)))
    }

    pub fn post(&self, path: &str, body: &Value) -> ApiResult<Value> {
        self.send(self.http.post(format!("{}{path}", self.base)).json(body))
    }

    pub fn delete(&self, path: &str) -> ApiResult<Value> {
        self.send(self.http.delete(format!("{}{path}", self.base)))
    }

    pub fn formalization_opportunities(&self, args: &Value) -> ApiResult<Value> {
        let mut query = vec![];
        for key in ["q", "kind", "status", "before"] {
            if let Some(value) = args[key].as_str() {
                query.push(format!("{key}={}", segment(value)));
            }
        }
        if let Some(limit) = args["limit"].as_u64() {
            query.push(format!("limit={}", limit.clamp(1, 100)));
        }
        self.get(&format!(
            "/formalizations/opportunities?{}",
            query.join("&")
        ))
    }
    pub fn formalization_project(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/formalizations/{}/project", segment(id)))
    }
    pub fn papers(&self, args: &Value) -> ApiResult<Value> {
        let mut query = Vec::new();
        for key in ["q", "author", "msc", "kind", "before"] {
            if let Some(value) = args[key].as_str() {
                query.push((key, value.to_owned()));
            }
        }
        query.push((
            "limit",
            args["limit"]
                .as_u64()
                .unwrap_or(25)
                .clamp(1, 100)
                .to_string(),
        ));
        self.send(self.http.get(format!("{}/papers", self.base)).query(&query))
    }
    pub fn paper(&self, record: &str) -> ApiResult<Value> {
        self.get(&format!("/papers/{}", segment(record)))
    }
    pub fn review(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/submissions/{}/review", segment(id)))
    }

    pub fn prepare_submission(
        &self,
        metadata: &Value,
        filename: &str,
        source: &[u8],
    ) -> ApiResult<Value> {
        use base64::Engine;
        if source.is_empty() || source.len() > 30 * 1024 * 1024 {
            return Err(ApiError {
                status: 422,
                detail: "manuscript must contain 1 byte to 30 MiB".into(),
            });
        }
        self.post("/submissions/prepare", &json!({"metadata":metadata,"filename":filename,"source_base64":base64::engine::general_purpose::STANDARD.encode(source)}))
    }
    pub fn submit_prepared(&self, id: &str, input: &Value) -> ApiResult<Value> {
        self.post(&format!("/submissions/{}/submit", segment(id)), input)
    }
    pub fn submission(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/submissions/{}", segment(id)))
    }

    pub fn deliver_manuscript(
        &self,
        metadata: &Value,
        filename: &str,
        source: &[u8],
    ) -> ApiResult<Value> {
        if self.nyxid_service.is_some() {
            use base64::Engine;
            if source.is_empty() || source.len() > 30 * 1024 * 1024 {
                return Err(ApiError {
                    status: 0,
                    detail: "Manuscript source must contain 1 byte to 30 MiB".into(),
                });
            }
            return self.post("/manuscript-deliveries", &json!({"metadata":metadata,"filename":filename,"source_base64":base64::engine::general_purpose::STANDARD.encode(source)}));
        }
        let metadata = serde_json::to_vec(metadata).map_err(|e| ApiError {
            status: 0,
            detail: e.to_string(),
        })?;
        if metadata.len() > 1024 * 1024
            || source.is_empty()
            || source.len() > 30 * 1024 * 1024
            || filename.is_empty()
            || filename.len() > 200
            || filename
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\\' | '/'))
        {
            return Err(ApiError { status: 0, detail: "Supply a manuscript basename, metadata up to 1 MiB, and a source file up to 30 MiB.".into() });
        }
        let mut serial = 0_u64;
        let boundary = loop {
            let candidate = format!(
                "mathesis-manuscript-{:x}-{serial}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            if !source
                .windows(candidate.len())
                .any(|w| w == candidate.as_bytes())
                && !metadata
                    .windows(candidate.len())
                    .any(|w| w == candidate.as_bytes())
            {
                break candidate;
            }
            serial += 1;
        };
        let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n").into_bytes();
        body.extend(metadata);
        body.extend(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"source\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend(source);
        body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
        self.send(
            self.http
                .post(format!("{}/manuscript-deliveries", self.base))
                .header(
                    reqwest::header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body),
        )
    }

    pub fn manuscript_status(&self, id: &str, version: Option<u64>) -> ApiResult<Value> {
        self.get(&format!(
            "/submissions/{}/delivery{}",
            segment(id),
            version.map(|v| format!("?version={v}")).unwrap_or_default()
        ))
    }

    pub fn find_manuscript(&self, producer: &str, work: &str, purpose: &str) -> ApiResult<Value> {
        self.get(&format!(
            "/manuscript-deliveries?producer={}&work_id={}&purpose={}",
            segment(producer),
            segment(work),
            segment(purpose)
        ))
    }

    pub fn confirm_manuscript(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!("/submissions/{}/delivery-confirmation", segment(id)),
            body,
        )
    }

    pub fn confirm_delivered_manuscript(&self, receipt: Value, claims: &Value) -> ApiResult<Value> {
        if receipt["current"] == false {
            return Ok(receipt);
        }
        let id = receipt["submission"].as_str().ok_or_else(|| ApiError {
            status: 0,
            detail: "Manuscript receipt has no submission identity".into(),
        })?;
        self.confirm_manuscript(id, &json!({"version":receipt["version"],"source_sha256":receipt["source_sha256"],"expected_claims_revision":receipt["expected_claims_revision"],"claims":claims}))
    }
    pub fn knowledge_capabilities(&self) -> ApiResult<Value> {
        self.get("/knowledge/capabilities")
    }
    pub fn feedback_observation(&self, id: &str, key: &str, request: &str) -> ApiResult<Value> {
        self.get(&format!(
            "/submissions/{}/feedback/{}/observations/{}",
            segment(id),
            segment(key),
            segment(request)
        ))
    }
    pub fn feedbacks(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/submissions/{}/feedback", segment(id)))
    }
    pub fn feedback(&self, id: &str, key: &str) -> ApiResult<Value> {
        self.get(&format!(
            "/submissions/{}/feedback/{}",
            segment(id),
            segment(key)
        ))
    }
    pub fn preview_feedback(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!("/submissions/{}/feedback/preview", segment(id)),
            body,
        )
    }
    pub fn prepare_feedback(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(&format!("/submissions/{}/feedback", segment(id)), body)
    }
    pub fn authorize_feedback(&self, id: &str, key: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/feedback/{}/authorize",
                segment(id),
                segment(key)
            ),
            body,
        )
    }
    pub fn revoke_feedback(&self, id: &str, key: &str, reason: &str) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/feedback/{}/revoke",
                segment(id),
                segment(key)
            ),
            &serde_json::json!({"reason":reason}),
        )
    }
    pub fn export_feedback(&self, id: &str, key: &str) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/feedback/{}/export",
                segment(id),
                segment(key)
            ),
            &serde_json::json!({}),
        )
    }
    pub fn feedback_readback(&self, id: &str, key: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/feedback/{}/readback",
                segment(id),
                segment(key)
            ),
            body,
        )
    }
    pub fn bindings(&self, before: Option<&str>) -> ApiResult<Value> {
        self.send(
            self.http.get(format!("{}/bindings", self.base)).query(
                &before
                    .map(|v| ("before", v))
                    .into_iter()
                    .collect::<Vec<_>>(),
            ),
        )
    }
    pub fn binding_profile(&self, person: &str, before: Option<&str>) -> ApiResult<Value> {
        self.send(
            self.http
                .get(format!(
                    "{}/binding-contributors/{}",
                    self.base,
                    segment(person)
                ))
                .query(
                    &before
                        .map(|v| ("before", v))
                        .into_iter()
                        .collect::<Vec<_>>(),
                ),
        )
    }
    pub fn private_bindings(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/submissions/{}/bindings", segment(id)))
    }
    pub fn binding(&self, id: &str, key: &str) -> ApiResult<Value> {
        self.get(&format!("/bindings/{}/{}", segment(id), segment(key)))
    }
    pub fn publish_binding(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(&format!("/submissions/{}/bindings", segment(id)), body)
    }
    pub fn revoke_binding(&self, id: &str, key: &str, reason: &str) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/bindings/{}/revoke",
                segment(id),
                segment(key)
            ),
            &json!({"reason":reason}),
        )
    }
    pub fn contribute_binding(&self, id: &str, key: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!("/bindings/{}/{}/contributions", segment(id), segment(key)),
            body,
        )
    }
    pub fn challenge_binding(&self, id: &str, key: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!("/bindings/{}/{}/challenges", segment(id), segment(key)),
            body,
        )
    }
    pub fn research(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/submissions/{}/research", segment(id)))
    }
    pub fn adopt_research(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(&format!("/submissions/{}/research", segment(id)), body)
    }
    pub fn stop_research(&self, id: &str, key: &str, reason: &str) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/research/{}/stop",
                segment(id),
                segment(key)
            ),
            &json!({"reason":reason}),
        )
    }
    pub fn research_attempt(&self, id: &str, key: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/research/{}/attempts",
                segment(id),
                segment(key)
            ),
            body,
        )
    }
    pub fn research_attempt_status(&self, id: &str, key: &str, attempt: &str) -> ApiResult<Value> {
        self.get(&format!(
            "/submissions/{}/research/{}/attempts/{}",
            segment(id),
            segment(key),
            segment(attempt)
        ))
    }
    pub fn recheck_research(&self, id: &str, key: &str, attempt: &str) -> ApiResult<Value> {
        self.post(
            &format!(
                "/submissions/{}/research/{}/attempts/{}/recheck",
                segment(id),
                segment(key),
                segment(attempt)
            ),
            &json!({}),
        )
    }
    pub fn knowledge_response(&self, id: &str, receipt: &str) -> ApiResult<Value> {
        self.get(&format!(
            "/submissions/{}/referee/knowledge/{}",
            segment(id),
            segment(receipt)
        ))
    }
    pub fn respond_review(&self, id: &str, body: &Value) -> ApiResult<Value> {
        self.post(
            &format!("/submissions/{}/review/responses", segment(id)),
            body,
        )
    }
    pub fn verification_bundle(
        &self,
        record: &str,
        claim: &str,
        attempt: &str,
    ) -> ApiResult<Value> {
        self.get(&format!(
            "/papers/{}/statements/{}/attempts/{}/bundle",
            segment(record),
            segment(claim),
            segment(attempt)
        ))
    }
    pub fn formalization(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/formalizations/{}", segment(id)))
    }
    pub fn verify_formalization(&self, id: &str, record: &str, claim: &str) -> ApiResult<Value> {
        self.verify_formalization_against(id, record, claim, None)
    }
    pub fn verify_formalization_against(
        &self,
        id: &str,
        record: &str,
        claim: &str,
        digest: Option<&str>,
    ) -> ApiResult<Value> {
        self.verify_formalization_project(id, record, claim, digest, None)
    }
    pub fn verify_formalization_project(
        &self,
        id: &str,
        record: &str,
        claim: &str,
        digest: Option<&str>,
        solution: Option<&str>,
    ) -> ApiResult<Value> {
        self.post(
            &format!("/formalizations/{}/attempts", segment(id)),
            &json!({"record":record,"claim":claim,"expected_target":digest,"solution":solution}),
        )
    }
    pub fn conjectures(&self) -> ApiResult<Value> {
        self.get("/conjectures?limit=100")
    }
    pub fn target(&self, record: &str, claim: &str) -> ApiResult<Value> {
        self.get(&format!("/papers/{record}/statements/{claim}"))
    }
    pub fn attempt(
        &self,
        record: &str,
        claim: &str,
        solution: &str,
        agent: Option<&str>,
    ) -> ApiResult<Value> {
        self.post(
            &format!("/papers/{record}/statements/{claim}/attempts"),
            &json!({"solution": solution, "as_agent": agent}),
        )
    }
    pub fn report(&self, record: &str, body: &Value) -> ApiResult<Value> {
        self.post(&format!("/papers/{record}/errors"), body)
    }
    pub fn report_status(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/errors/{id}"))
    }
    pub fn attempt_status(&self, id: &str) -> ApiResult<Value> {
        self.get(&format!("/attempts/{id}"))
    }

    pub fn open_tasks(&self, kind: Option<&str>, limit: u32) -> ApiResult<Value> {
        if kind == Some("formalize") {
            return Err(ApiError {
                status: 422,
                detail: "use target and attempt for statement formalization".into(),
            });
        }
        let mut path = format!("/tasks?status=open&limit={limit}");
        if let Some(kind) = kind {
            path.push_str(&format!("&kind={kind}"));
        }
        self.get(&path)
    }

    pub fn lease(&self, task: &str) -> ApiResult<Value> {
        self.post(&format!("/tasks/{task}/lease"), &json!({}))
    }

    pub fn release(&self, task: &str) -> ApiResult<Value> {
        self.delete(&format!("/tasks/{task}/lease"))
    }

    /// The task plus what an agent needs to do it: the statement, the
    /// statements it depends on, the paper's title and abstract, and the
    /// rules for its kind.
    pub fn context(&self, task: &str) -> ApiResult<Value> {
        let mut context = self.get(&format!("/tasks/{task}"))?;
        let kind = context["task"]["kind"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        context["rules"] = json!(crate::rules::for_kind(&kind));
        Ok(context)
    }

    pub fn submit(&self, task: &str, contribution: &Value) -> ApiResult<Value> {
        self.post(&format!("/tasks/{task}/contributions"), contribution)
    }
}

#[cfg(test)]
mod solving_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };

    #[test]
    fn solving_client_downloads_exact_target_and_submits_without_claiming_verification() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for (method, path, body) in [
                (
                    "GET",
                    "/api/v1/papers?q=prime+%26+gaps&author=A.+Author&limit=25",
                    json!({"items":[],"next_before":null}),
                ),
                (
                    "GET",
                    "/api/v1/formalizations/opportunities?q=prime%20%26%20gaps&kind=paper&status=open&limit=1",
                    json!({"items":[], "next_before": "cursor"}),
                ),
                (
                    "GET",
                    "/api/v1/submissions/sub1/review",
                    json!({"schema_version":1,"source_sha256":"pinned-source","service":"Orchestra"}),
                ),
                (
                    "GET",
                    "/api/v1/conjectures?limit=100",
                    json!({"items": [], "next_before": null}),
                ),
                (
                    "GET",
                    "/api/v1/papers/WP-2026-0001/statements/C1",
                    json!({"target": {"lean":"import Mathlib\ndef wishpool_target_prop : Prop := True\n", "digest":"exact-digest"}}),
                ),
                (
                    "POST",
                    "/api/v1/papers/WP-2026-0001/statements/C1/attempts",
                    json!({"id":"a1", "state":"queued"}),
                ),
                (
                    "GET",
                    "/api/v1/attempts/a1",
                    json!({"id":"a1", "state":"rejected", "receipt":{"reason":"Wrong target type"}}),
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let header_end = loop {
                    let mut buf = [0; 4096];
                    let count = stream.read(&mut buf).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buf[..count]);
                    if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
                assert!(header.starts_with(&format!("{method} {path} HTTP/1.1")));
                assert!(
                    header
                        .to_lowercase()
                        .contains("authorization: bearer test-token")
                );
                let length = header
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + length {
                    let mut buf = [0; 4096];
                    let count = stream.read(&mut buf).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buf[..count]);
                }
                if method == "POST" {
                    let submitted: Value = serde_json::from_slice(&bytes[header_end..]).unwrap();
                    assert_eq!(
                        submitted,
                        json!({"solution":"import Target\nproof", "as_agent":"Proof agent"})
                    );
                    assert!(submitted.get("receipt").is_none());
                }
                let text = body.to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len()).unwrap();
            }
        });
        let client = Client::new(&origin, "test-token");
        assert_eq!(
            client
                .papers(&json!({"q":"prime & gaps","author":"A. Author"}))
                .unwrap()["items"],
            json!([])
        );
        assert_eq!(
            client
                .formalization_opportunities(
                    &json!({"q":"prime & gaps", "kind":"paper", "status":"open", "limit":1})
                )
                .unwrap()["next_before"],
            "cursor"
        );
        assert_eq!(
            client.review("sub1").unwrap()["source_sha256"],
            "pinned-source"
        );
        assert_eq!(client.conjectures().unwrap()["items"], json!([]));
        let target = client.target("WP-2026-0001", "C1").unwrap();
        assert_eq!(target["target"]["digest"], "exact-digest");
        assert!(
            target["target"]["lean"]
                .as_str()
                .unwrap()
                .ends_with("True\n")
        );
        assert_eq!(
            client
                .attempt(
                    "WP-2026-0001",
                    "C1",
                    "import Target\nproof",
                    Some("Proof agent")
                )
                .unwrap()["state"],
            "queued"
        );
        assert_eq!(
            client.attempt_status("a1").unwrap()["receipt"]["reason"],
            "Wrong target type"
        );
        server.join().unwrap();
    }
}

#[cfg(test)]
mod manuscript_tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn binary_delivery_and_confirmation_keep_the_same_source_identity() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = vec![];
                let head = loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(p) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break p + 4;
                    }
                };
                let headers = String::from_utf8(bytes[..head].to_vec())
                    .unwrap()
                    .to_lowercase();
                let size = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                while bytes.len() < head + size {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                assert!(headers.contains("authorization: bearer test-token"));
                if index == 0 {
                    assert!(headers.starts_with("post /api/v1/manuscript-deliveries "));
                    assert!(headers.contains("multipart/form-data; boundary="));
                    assert!(bytes[head..].windows(5).any(|w| w == b"PK\x00\xff\x80"));
                    assert!(String::from_utf8_lossy(&bytes[head..]).contains("note.zip"));
                } else {
                    assert!(
                        headers
                            .starts_with("post /api/v1/submissions/sub-one/delivery-confirmation ")
                    );
                    let input: Value = serde_json::from_slice(&bytes[head..]).unwrap();
                    assert_eq!(
                        input,
                        json!({"version":2,"source_sha256":"saved-digest","expected_claims_revision":3,"claims":[{"id":"C1"}]})
                    );
                }
                let body=json!({"submission":"sub-one","current":true,"version":2,"source_sha256":"saved-digest","expected_claims_revision":3}).to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let client = Client::new(&origin, "test-token");
        let receipt = client
            .deliver_manuscript(&json!({"work_id":"note-a"}), "note.zip", b"PK\x00\xff\x80")
            .unwrap();
        client
            .confirm_delivered_manuscript(receipt, &json!([{"id":"C1"}]))
            .unwrap();
        server.join().unwrap();
    }
}
