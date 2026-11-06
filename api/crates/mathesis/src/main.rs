//! Mathesis server: composes Layer 1–3 with MongoDB, NyxID sign-in, the
//! LaTeX compiler and the machine review worker.

mod archives;
mod auth;
mod cma_connection;
mod cma_tokens;
mod composition;
mod config;
mod donations;
mod health;
mod knowledge;
mod latex;
mod review;
mod store;
mod verifier;

use std::time::Duration;

use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

use crate::{composition::Composition, config::Config};

const HTTP_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if matches!(
        arguments.first().map(String::as_str),
        Some("--knowledge-import" | "--knowledge-query")
    ) {
        let config::Storage::Mongo { uri, database } = &config.storage else {
            anyhow::bail!("knowledge operations require MongoDB");
        };
        let store = store::MongoStore::connect(uri, database).await?;
        let result: anyhow::Result<serde_json::Value> = async {
            match arguments[0].as_str() {
                "--knowledge-import" if arguments.len() == 5 => {
                    let snapshot = knowledge::import(&store, arguments[1].clone().into(), arguments[2].clone().into(), arguments[3].clone().into(), arguments[4].clone().into()).await?;
                    Ok(serde_json::to_value(snapshot)?)
                }
                "--knowledge-query" if (3..=4).contains(&arguments.len()) => {
                    let knowledge = knowledge::Knowledge::new(store.clone(), arguments[1].clone());
                    let index = if arguments.len() == 3 {
                        knowledge.index(&knowledge.configured_revision).await?
                    } else {
                        anyhow::ensure!(arguments[3] == mathesis_review::knowledge::index::IDENTIFIER_PATH_RANKER, "unsupported diagnostic ranker");
                        knowledge.ranked_index(&knowledge.configured_revision, knowledge::SourceRanking::IdentifierPaths).await?
                    };
                    Ok(serde_json::json!({"diagnostic_only":true,"result":index.search(&arguments[2], None, 10)?}))
                }
                _ => anyhow::bail!("use --knowledge-import PROJECTION REPORT SOURCES ADMISSION.json or --knowledge-query INDEX_REVISION QUERY [mathesis.source-ranker.identifier-paths.v1]"),
            }
        }.await;
        store.shutdown().await;
        println!("{}", serde_json::to_string_pretty(&result?)?);
        return Ok(());
    }
    if matches!(
        arguments.first().map(String::as_str),
        Some("--storage-audit" | "--storage-gc")
    ) {
        let config::Storage::Mongo { uri, database } = &config.storage else {
            anyhow::bail!("storage maintenance requires MongoDB");
        };
        let store = store::MongoStore::connect(uri, database).await?;
        let result = if arguments[0] == "--storage-audit" && arguments.len() == 1 {
            store.storage_audit().await
        } else if arguments.as_slice() == ["--storage-gc", "--offline"] {
            store.collect_orphan_files_offline().await
        } else {
            store.shutdown().await;
            anyhow::bail!(
                "use --storage-audit, or stop every API/worker instance and use --storage-gc --offline"
            );
        };
        store.shutdown().await;
        println!("{}", serde_json::to_string_pretty(&result?)?);
        return Ok(());
    }
    anyhow::ensure!(
        arguments.is_empty() || arguments.as_slice() == ["--check"],
        "unknown command; use --check, --storage-audit, --storage-gc --offline, --knowledge-import or --knowledge-query"
    );
    tracing::info!(bind = %config.bind, public_url = %config.public_url, auth = ?config.auth, "starting Mathesis");
    let composition = Composition::build(&config).await?;
    if std::env::args().nth(1).as_deref() == Some("--check") {
        let (ready, status) = composition.readiness.check().await;
        println!("{}", serde_json::to_string_pretty(&status)?);
        if let Some(mongo) = composition.mongo {
            mongo.shutdown().await;
        }
        anyhow::ensure!(
            ready,
            "configured services are unavailable; see the diagnostic status"
        );
        return Ok(());
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mongo = composition.mongo;
    let mut background = Vec::new();
    if config.role.runs_workers() {
        background.push(tokio::spawn(composition.worker.run(shutdown_rx.clone())));
        background.push(tokio::spawn(review::run_reconciler(
            composition.app.clone(),
            shutdown_rx.clone(),
        )));
        if let Some(hosted) = composition.hosted {
            background.push(tokio::spawn(hosted.run(shutdown_rx.clone())));
        }
    } else {
        tracing::info!("api role: background workers run in the worker deployment");
    }

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let mut http_shutdown = shutdown_rx.clone();
    let server = axum::serve(listener, composition.router).with_graceful_shutdown(async move {
        let _ = http_shutdown.changed().await;
    });
    let server = tokio::spawn(async move { server.await });

    shutdown_signal().await;
    tracing::info!("shutdown requested; draining");
    let _ = shutdown_tx.send(true);
    if tokio::time::timeout(HTTP_DRAIN_TIMEOUT, server)
        .await
        .is_err()
    {
        tracing::warn!("HTTP drain timed out");
    }
    for task in background {
        let _ = tokio::time::timeout(HTTP_DRAIN_TIMEOUT, task).await;
    }
    if let Some(mongo) = mongo {
        mongo.shutdown().await;
    }
    tracing::info!("stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
