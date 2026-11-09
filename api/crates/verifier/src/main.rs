use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
};
use mathesis_verifier::{Checker, Receipt, Request};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = Arc::new(Checker {
        workspace: std::env::var("WISHPOOL_LEAN_WORKSPACE")?.into(),
        scratch: std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir),
        timeout: Duration::from_secs(
            std::env::var("WISHPOOL_VERIFIER_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1200)
                .clamp(1, 3600),
        ),
    });
    if let Ok(bind) = std::env::var("WISHPOOL_VERIFIER_BIND") {
        async fn verify(
            State((checker, semaphore)): State<(Arc<Checker>, Arc<tokio::sync::Semaphore>)>,
            Json(request): Json<Request>,
        ) -> Json<Receipt> {
            let _permit = semaphore
                .acquire()
                .await
                .expect("verifier semaphore is open");
            Json(checker.verify(&request).await)
        }
        let router = Router::new()
            .route("/healthz", get(|| async { axum::http::StatusCode::OK }))
            .route(
                "/environment",
                get({
                    let checker = checker.clone();
                    move || {
                        let checker = checker.clone();
                        async move {
                            mathesis_verifier::workspace_ready(&checker.workspace)
                                .and_then(|_| {
                                    mathesis_verifier::Environment::read(&checker.workspace)
                                })
                                .map(Json)
                                .map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)
                        }
                    }
                }),
            )
            .route(
                "/readyz",
                get({
                    let checker = checker.clone();
                    move || {
                        let checker = checker.clone();
                        async move {
                            if mathesis_verifier::workspace_ready(&checker.workspace).is_ok() {
                                axum::http::StatusCode::OK
                            } else {
                                axum::http::StatusCode::SERVICE_UNAVAILABLE
                            }
                        }
                    }
                }),
            )
            .route("/verify", post(verify))
            .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
            .with_state((checker, Arc::new(tokio::sync::Semaphore::new(1))));
        axum::serve(tokio::net::TcpListener::bind(bind).await?, router).await?;
    } else {
        let mut bytes = vec![];
        tokio::io::stdin()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("input exceeds 64 MB".into());
        }
        let receipt = checker.verify(&serde_json::from_slice(&bytes)?).await;
        tokio::io::stdout()
            .write_all(&serde_json::to_vec(&receipt)?)
            .await?;
    }
    Ok(())
}
