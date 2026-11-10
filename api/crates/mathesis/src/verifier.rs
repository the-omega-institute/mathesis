//! Layer-3 verifier adapter: subprocess locally, isolated HTTP deployment in infra.
use async_trait::async_trait;
use mathesis_core::{
    CoreError, CoreResult,
    model::{PreparedLibrary, VerificationReceipt, VerificationRequest},
    ports::Verifier,
};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
/// The verifier bounds its declaration to eight libraries of sixteen roots.
const ENVIRONMENT_CAP: usize = 16 * 1024;
fn prepared(libraries: Vec<mathesis_verifier::Library>) -> Vec<PreparedLibrary> {
    libraries
        .into_iter()
        .map(|library| PreparedLibrary {
            name: library.name,
            revision: library.revision,
            roots: library.roots,
            url: library.url,
        })
        .collect()
}
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
    async fn libraries(&self) -> CoreResult<Vec<PreparedLibrary>> {
        mathesis_verifier::Environment::read(&self.workspace)
            .map(|environment| prepared(environment.libraries))
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
impl HttpVerifier {
    /// The prepared environment the verifier serves for new proof work.
    async fn environment(&self, timeout: Duration) -> CoreResult<mathesis_verifier::Environment> {
        let response = reqwest::Client::new()
            .get(format!("{}/environment", self.url.trim_end_matches('/')))
            .timeout(timeout)
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
            if bytes.len() + chunk.len() > ENVIRONMENT_CAP {
                return Err(CoreError::Unavailable(
                    "verifier environment exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let environment: mathesis_verifier::Environment =
            serde_json::from_slice(&bytes).map_err(|e| CoreError::Unavailable(e.to_string()))?;
        let mut checked = mathesis_verifier::Environment::parse(&environment.label())
            .map_err(CoreError::Unavailable)?;
        checked.libraries = environment.libraries;
        Ok(checked)
    }
}
#[async_trait]
impl Verifier for HttpVerifier {
    async fn toolchain(&self) -> CoreResult<String> {
        Ok(self.environment(Duration::from_secs(15)).await?.label())
    }
    async fn libraries(&self) -> CoreResult<Vec<PreparedLibrary>> {
        // Admission reads this on the request path; an unreachable verifier must not
        // hold a submission for long before the historical import rules apply.
        Ok(prepared(
            self.environment(Duration::from_secs(3)).await?.libraries,
        ))
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
                            library_use: None,
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

    fn library(roots: usize) -> mathesis_verifier::Library {
        mathesis_verifier::Library {
            name: "trureturing".into(),
            revision: "b".repeat(40),
            roots: (0..roots)
                .map(|i| format!("Root{i}{}", "x".repeat(50)))
                .collect(),
            url: Some("https://github.com/example/library".into()),
        }
    }
    async fn serve(body: Vec<u8>) -> (HttpVerifier, tokio::task::JoinHandle<()>) {
        let router = Router::new().route(
            "/environment",
            get(move || async move { ([("content-type", "application/json")], body) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let verifier = HttpVerifier {
            url: format!("http://{}", listener.local_addr().unwrap()),
            legacy_url: None,
            timeout: Duration::from_secs(2),
        };
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (verifier, server)
    }

    #[tokio::test]
    async fn http_environment_reports_prepared_libraries_within_a_small_bound() {
        let mut environment = mathesis_verifier::Environment::parse(
            "leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679",
        )
        .unwrap();
        // A verifier without a library declaration serves the historical body.
        let (verifier, server) = serve(serde_json::to_vec(&environment).unwrap()).await;
        assert_eq!(verifier.toolchain().await.unwrap(), environment.label());
        assert!(verifier.libraries().await.unwrap().is_empty());
        server.abort();

        // Larger than the previous 4096-byte bound, as an eight-library declaration can be.
        environment.libraries = (0..5)
            .map(|i| mathesis_verifier::Library {
                name: format!("library{i}"),
                ..library(16)
            })
            .collect();
        let body = serde_json::to_vec(&environment).unwrap();
        assert!(body.len() > 4096 && body.len() <= ENVIRONMENT_CAP);
        let (verifier, server) = serve(body).await;
        assert_eq!(verifier.toolchain().await.unwrap(), environment.label());
        let libraries = verifier.libraries().await.unwrap();
        assert_eq!(libraries.len(), 5);
        assert_eq!(libraries[0].name, "library0");
        assert_eq!(libraries[0].revision, "b".repeat(40));
        assert_eq!(libraries[0].roots.len(), 16);
        assert_eq!(
            libraries[0].url.as_deref(),
            Some("https://github.com/example/library")
        );
        assert!(libraries.iter().all(PreparedLibrary::valid));
        server.abort();

        let mut oversized = serde_json::to_vec(&environment).unwrap();
        oversized.extend(std::iter::repeat_n(b' ', ENVIRONMENT_CAP));
        let (verifier, server) = serve(oversized).await;
        assert!(matches!(
            verifier.libraries().await,
            Err(CoreError::Unavailable(_))
        ));
        assert!(verifier.toolchain().await.is_err());
        server.abort();
    }

    #[tokio::test]
    async fn process_verifier_reads_the_library_declaration_of_its_workspace() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(
            workspace.path().join("lean-toolchain"),
            "leanprover/lean4:v4.35.0\n",
        )
        .unwrap();
        std::fs::write(
            workspace.path().join("lake-manifest.json"),
            serde_json::json!({"packages":[
                {"name":"mathlib","rev":"56a005855ea17e0b0958f73ec801ae140b0a1679"},
                {"name":"trureturing","rev":"b".repeat(40),"url":"https://github.com/example/library.git"}
            ]})
            .to_string(),
        )
        .unwrap();
        let verifier = ProcessVerifier {
            program: PathBuf::new(),
            workspace: workspace.path().into(),
            scratch: workspace.path().into(),
            timeout: Duration::from_secs(1),
        };
        assert!(verifier.libraries().await.unwrap().is_empty());
        std::fs::write(
            workspace.path().join(mathesis_verifier::LIBRARIES_FILE),
            r#"{"libraries":[{"package":"trureturing","roots":["D5","Trureturing"]}]}"#,
        )
        .unwrap();
        assert_eq!(
            verifier.libraries().await.unwrap(),
            [PreparedLibrary {
                name: "trureturing".into(),
                revision: "b".repeat(40),
                roots: vec!["D5".into(), "Trureturing".into()],
                url: Some("https://github.com/example/library".into()),
            }]
        );
        assert_eq!(
            verifier.toolchain().await.unwrap(),
            "leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679"
        );
    }

    #[test]
    fn verifier_receipts_decode_into_the_domain_receipt_without_loss() {
        let mut receipt = mathesis_verifier::Receipt {
            used_constants: vec![],
            library_use: None,
            verdict: mathesis_verifier::Verdict::Proved,
            reason: "Lean checked the imported target and standard axioms".into(),
            target_digest: "t".into(),
            solution_digest: "s".into(),
            toolchain: "pin".into(),
            axioms: vec!["propext".into()],
            checked_at: chrono::Utc::now(),
            duration: 3,
        };
        for library_use in [
            None,
            Some(mathesis_verifier::LibraryUse {
                libraries: vec![mathesis_verifier::LibraryPin {
                    name: "trureturing".into(),
                    revision: "b".repeat(40),
                    url: Some("https://github.com/example/library".into()),
                }],
                declarations: vec![mathesis_verifier::LibraryDeclaration {
                    library: "trureturing".into(),
                    module: "D5.S3.Result".into(),
                    name: "D5.S3.Result.result".into(),
                }],
                total: 1,
                complete: true,
            }),
        ] {
            receipt.library_use = library_use;
            let wire = serde_json::to_string(&receipt).unwrap();
            let decoded: VerificationReceipt = serde_json::from_str(&wire).unwrap();
            assert_eq!(
                decoded.library_use.as_ref().map(|report| report.valid()),
                receipt.library_use.as_ref().map(|_| true)
            );
            assert_eq!(serde_json::to_string(&decoded).unwrap(), wire);
        }
    }
}
