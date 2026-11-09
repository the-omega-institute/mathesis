//! Layer-3 verifier adapter: subprocess locally, isolated HTTP deployment in infra.
use async_trait::async_trait;
use mathesis_core::{
    CoreError, CoreResult,
    model::{VerificationReceipt, VerificationRequest},
    ports::Verifier,
};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub struct ProcessVerifier {
    pub program: PathBuf,
    pub workspace: PathBuf,
    pub scratch: PathBuf,
    pub timeout: Duration,
}
#[async_trait]
impl Verifier for ProcessVerifier {
    async fn toolchain(&self) -> CoreResult<String> {
        mathesis_verifier::Environment::read(&self.workspace)
            .map(|environment| environment.label())
            .map_err(CoreError::Unavailable)
    }
    async fn verify(&self, request: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        let check = async {
            let mut child = tokio::process::Command::new(&self.program)
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", std::env::var_os("HOME").unwrap_or_default())
                .env("TMPDIR", &self.scratch)
                .env("WISHPOOL_LEAN_WORKSPACE", &self.workspace)
                .env(
                    "WISHPOOL_VERIFIER_TIMEOUT_SECS",
                    self.timeout.as_secs().to_string(),
                )
                .current_dir(&self.scratch)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| CoreError::Unavailable(format!("verifier: {e}")))?;
            let input =
                serde_json::to_vec(request).map_err(|e| CoreError::Unavailable(e.to_string()))?;
            let mut stdin = child.stdin.take().unwrap();
            let output = child.stdout.take().unwrap();
            let write = async {
                stdin.write_all(&input).await?;
                stdin.shutdown().await
            };
            let read = async {
                let mut bytes = vec![];
                output.take(65537).read_to_end(&mut bytes).await?;
                Ok::<_, std::io::Error>(bytes)
            };
            let (_, bytes) =
                tokio::try_join!(write, read).map_err(|e| CoreError::Unavailable(e.to_string()))?;
            if bytes.len() > 65536 {
                return Err(CoreError::Unavailable(
                    "verifier output exceeds limit".into(),
                ));
            }
            if !child
                .wait()
                .await
                .map_err(|e| CoreError::Unavailable(e.to_string()))?
                .success()
            {
                return Err(CoreError::Unavailable("verifier process failed".into()));
            }
            serde_json::from_slice(&bytes)
                .map_err(|e| CoreError::Unavailable(format!("verifier receipt: {e}")))
        };
        tokio::time::timeout(self.timeout + Duration::from_secs(10), check)
            .await
            .map_err(|_| CoreError::Unavailable("verifier deadline exceeded".into()))?
    }
}
pub struct HttpVerifier {
    pub url: String,
    pub legacy_url: Option<String>,
    pub timeout: Duration,
}
#[async_trait]
impl Verifier for HttpVerifier {
    async fn toolchain(&self) -> CoreResult<String> {
        let response = reqwest::Client::new()
            .get(format!("{}/environment", self.url.trim_end_matches('/')))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| CoreError::Unavailable(e.to_string()))?
            .error_for_status()
            .map_err(|e| CoreError::Unavailable(e.to_string()))?;
        let mut response = response;
        let mut bytes = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| CoreError::Unavailable(e.to_string()))?
        {
            if bytes.len() + chunk.len() > 4096 {
                return Err(CoreError::Unavailable(
                    "verifier environment exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let environment: mathesis_verifier::Environment =
            serde_json::from_slice(&bytes).map_err(|e| CoreError::Unavailable(e.to_string()))?;
        mathesis_verifier::Environment::parse(&environment.label())
            .map(|environment| environment.label())
            .map_err(CoreError::Unavailable)
    }
    async fn verify(&self, request: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        let mut url = &self.url;
        if let Some(legacy) = &self.legacy_url {
            let current = self.toolchain().await?;
            let environment =
                mathesis_verifier::Environment::parse(&current).map_err(CoreError::Unavailable)?;
            if request.toolchain != current && request.toolchain != environment.lean_toolchain {
                url = legacy;
            }
        }
        let response = reqwest::Client::builder()
            .timeout(self.timeout + Duration::from_secs(10))
            .build()
            .map_err(|e| CoreError::Unavailable(e.to_string()))?
            .post(format!("{}/verify", url.trim_end_matches('/')))
            .json(request)
            .send()
            .await
            .map_err(|e| CoreError::Unavailable(e.to_string()))?
            .error_for_status()
            .map_err(|e| CoreError::Unavailable(e.to_string()))?;
        let mut response = response;
        let mut bytes = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| CoreError::Unavailable(e.to_string()))?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err(CoreError::Unavailable(
                    "verifier receipt exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|e| CoreError::Unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        routing::{get, post},
    };
    use mathesis_core::model::Verdict;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[tokio::test]
    async fn new_work_reads_current_pins_and_historical_requests_reach_the_legacy_verifier() {
        let environment = mathesis_verifier::Environment::parse(
            "leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679",
        )
        .unwrap();
        let current = environment.label();
        let mut servers = vec![];
        let mut urls = vec![];
        let counters = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
        for count in &counters {
            let count = count.clone();
            let environment = environment.clone();
            let router = Router::new()
                .route(
                    "/environment",
                    get(move || async move { Json(environment) }),
                )
                .route(
                    "/verify",
                    post(move |Json(request): Json<VerificationRequest>| async move {
                        count.fetch_add(1, Ordering::SeqCst);
                        Json(VerificationReceipt {
                            verdict: Verdict::Rejected,
                            reason: "routing fixture only".into(),
                            target_digest: request.target_digest,
                            solution_digest: String::new(),
                            toolchain: request.toolchain,
                            axioms: vec![],
                            used_constants: vec![],
                            checked_at: chrono::Utc::now(),
                            duration: 0,
                        })
                    }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            urls.push(format!("http://{}", listener.local_addr().unwrap()));
            servers.push(tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap()
            }));
        }
        let verifier = HttpVerifier {
            url: urls[0].clone(),
            legacy_url: Some(urls[1].clone()),
            timeout: Duration::from_secs(2),
        };
        assert_eq!(verifier.toolchain().await.unwrap(), current);
        for pin in [
            current,
            "leanprover/lean4:v4.33.0, Mathlib db584cd6d46c92f209a44c0f1c829460d327499d".into(),
        ] {
            verifier
                .verify(&VerificationRequest {
                    required_declaration: None,
                    inspect_constants: vec![],
                    modules: Default::default(),
                    target: String::new(),
                    target_digest: "exact-target".into(),
                    toolchain: pin,
                    solution: String::new(),
                })
                .await
                .unwrap();
        }
        assert_eq!(counters[0].load(Ordering::SeqCst), 1);
        assert_eq!(counters[1].load(Ordering::SeqCst), 1);
        for server in servers {
            server.abort();
        }
    }
}
