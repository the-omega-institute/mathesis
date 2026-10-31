//! Durable CMA progress in the existing referee aggregate, fenced by the job lease.
use super::worker::Worker;
use async_trait::async_trait;
use mathesis_core::{
    app::App,
    model::{AgentStepUpdate, Caller, Submission, SubmissionStatus},
    ports::{JobLease, LeasedJob},
};
use mathesis_review::{ReviewError, ReviewResult, cma::RunStore};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) struct AgentStore {
    factory: Option<Arc<crate::cma_tokens::CmaFactory>>,
    logical_step: String,
    cleanup_ordinal: Option<u32>,
    cleanup: bool,
    ordinal: tokio::sync::Mutex<Option<u32>>,
    source_revision: tokio::sync::Mutex<Option<u64>>,
    app: Arc<App>,
    jobs: Arc<dyn JobLease>,
    job: LeasedJob,
    caller: Caller,
    key: String,
    version: u32,
    claims_revision: u64,
    funded: bool,
}
fn fence() -> ReviewError {
    ReviewError::PersistenceFence
}
impl Worker {
    pub(super) async fn cleanup_agents(
        &self,
        job: &LeasedJob,
    ) -> Result<bool, super::worker::Failure> {
        let Some(client) = self.advisor.as_ref().and_then(|a| a.managed_client()) else {
            return Ok(false);
        };
        let mut pending = false;
        let paper = self
            .app
            .submission(&self.auditor_account, &job.submission)
            .await?;
        for (key, step) in self
            .app
            .agent_steps(&self.auditor_account, &job.submission)
            .await?
        {
            let attempts: Vec<_> = if step.cma_attempts.is_empty() {
                vec![(None, step.progress.clone())]
            } else {
                step.cma_attempts
                    .iter()
                    .map(|a| {
                        (
                            Some(a.ordinal),
                            if Some(a.ordinal) == step.cma_attempts.last().map(|a| a.ordinal) {
                                step.progress.clone()
                            } else {
                                a.progress.clone()
                            },
                        )
                    })
                    .collect()
            };
            for (cleanup_ordinal, progress) in attempts {
                let state: mathesis_review::cma::RunState = match serde_json::from_value(progress) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if state.deleted {
                    continue;
                }
                let mut superseded = paper
                    .current_version()
                    .is_none_or(|v| v.number != step.version)
                    || paper.claims_revision != step.claims_revision
                    || matches!(
                        paper.status,
                        SubmissionStatus::Draft | SubmissionStatus::Withdrawn
                    );
                if let Some(step) = step.cma_attempts.last()
                    && let Some(key) = step
                        .step
                        .strip_prefix("research:")
                        .and_then(|s| s.split(':').next())
                {
                    superseded |= !self
                        .app
                        .research_active(&self.auditor_account, &paper.id, key)
                        .await?;
                }
                if let Some(attempt) = step.cma_attempts.last()
                    && let Some(board) = attempt
                        .step
                        .strip_prefix("binding:")
                        .and_then(|s| s.split(':').next())
                {
                    superseded |= !self
                        .app
                        .binding_active(&self.auditor_account, &paper.id, board)
                        .await?;
                }
                let expired = chrono::Utc::now().timestamp().max(0) as u64 >= state.deadline;
                if !superseded && !expired && state.answer.is_none() && state.failure.is_none() {
                    continue;
                }
                let store = AgentStore {
                    factory: self.cma_factory.clone(),
                    logical_step: String::new(),
                    cleanup_ordinal,
                    cleanup: true,
                    funded: false,
                    ordinal: tokio::sync::Mutex::new(step.cma_attempts.last().map(|a| a.ordinal)),
                    source_revision: tokio::sync::Mutex::new(None),
                    app: self.app.clone(),
                    jobs: self.jobs.clone(),
                    job: job.clone(),
                    caller: self.auditor_account.clone(),
                    key: key.clone(),
                    version: step.version,
                    claims_revision: step.claims_revision,
                };
                if let Err(error) = client.cancel(&store).await {
                    tracing::warn!(%error,"CMA cleanup will be retried on the next paper job");
                }
                if let Some(progress) = store.load().await.map_err(super::worker::model_failure)? {
                    pending |= progress["agent_id"].is_string() && progress["deleted"] != true;
                }
            }
        }
        Ok(pending)
    }

    pub(super) fn agent_store(
        &self,
        job: &LeasedJob,
        paper: &Submission,
        step: &str,
    ) -> AgentStore {
        let version = paper.current_version().map_or(0, |v| v.number);
        let logical_step = if step.starts_with("target:") {
            step
        } else {
            step.strip_suffix(":a0").unwrap_or(step)
        };
        // Preserve initial keys of already-admitted conversations; successors use their saved ordinal.
        let initial_step = if step.starts_with("target:")
            || step.starts_with("binding:")
            || step.starts_with("research:")
            || step.starts_with("agent-attempt:")
            || step.starts_with("error-report:")
        {
            step.to_owned()
        } else {
            format!("{logical_step}:a0")
        };
        let identity = format!(
            "{}:v{version}:c{}:{initial_step}",
            paper.id, paper.claims_revision
        );
        AgentStore {
            factory: self.cma_factory.clone(),
            logical_step: logical_step.into(),
            cleanup_ordinal: None,
            cleanup: false,
            ordinal: tokio::sync::Mutex::new(None),
            source_revision: tokio::sync::Mutex::new(None),
            app: self.app.clone(),
            jobs: self.jobs.clone(),
            job: job.clone(),
            caller: self.auditor_account.clone(),
            key: format!("wishpool:{:x}", Sha256::digest(identity.as_bytes())),
            version,
            claims_revision: paper.claims_revision,
            funded: step.starts_with("target:")
                || step.contains("formal")
                || step.starts_with("agent-attempt:"),
        }
    }
}
#[async_trait]
impl RunStore for AgentStore {
    fn key(&self) -> &str {
        &self.key
    }
    async fn active(&self) -> ReviewResult<bool> {
        if !self.jobs.renew(&self.job).await.map_err(|_| fence())? {
            return Err(fence());
        }
        let paper = self
            .app
            .submission(&self.caller, &self.job.submission)
            .await
            .map_err(|_| fence())?;
        if let Some(identity) = self.logical_step.strip_prefix("agent-formalization:") {
            let parts: Vec<_> = identity.split(':').collect();
            let Some(round) = parts
                .get(2)
                .and_then(|r| r.strip_prefix("round"))
                .and_then(|r| r.parse::<u32>().ok())
            else {
                return Ok(false);
            };
            let SubmissionStatus::Accepted { record } = &paper.status else {
                return Ok(false);
            };
            if parts.len() != 3
                || !self
                    .app
                    .formalization_generation_active(
                        &self.caller,
                        record,
                        &parts[0].into(),
                        parts[1],
                        round,
                    )
                    .await
                    .map_err(|_| fence())?
            {
                return Err(fence());
            }
        }
        if let Some(key) = self
            .logical_step
            .strip_prefix("research:")
            .and_then(|s| s.split(':').next())
            && !self
                .app
                .research_active(&self.caller, &paper.id, key)
                .await
                .map_err(|_| fence())?
        {
            return Ok(false);
        }
        if let Some(board) = self
            .logical_step
            .strip_prefix("binding:")
            .and_then(|s| s.split(':').next())
            && !self
                .app
                .binding_active(&self.caller, &paper.id, board)
                .await
                .map_err(|_| fence())?
        {
            return Ok(false);
        }
        Ok(paper
            .current_version()
            .is_some_and(|v| v.number == self.version)
            && paper.claims_revision == self.claims_revision
            && !matches!(
                paper.status,
                SubmissionStatus::Draft | SubmissionStatus::Withdrawn
            ))
    }
    async fn load(&self) -> ReviewResult<Option<Value>> {
        if !self.jobs.renew(&self.job).await.map_err(|_| fence())? {
            return Err(fence());
        }
        let (revision, step) = self
            .app
            .agent_step_snapshot(&self.caller, &self.job.submission, &self.key)
            .await
            .map_err(|_| fence())?;
        *self.source_revision.lock().await = Some(revision);
        Ok(step.and_then(|s| {
            let progress = match self.cleanup_ordinal {
                Some(ordinal) if s.cma_attempts.last().map(|a| a.ordinal) != Some(ordinal) => s
                    .cma_attempts
                    .iter()
                    .find(|a| a.ordinal == ordinal)
                    .map_or(Value::Null, |a| a.progress.clone()),
                _ => s.progress,
            };
            (!progress.is_null()).then_some(progress)
        }))
    }
    async fn save(&self, progress: Value) -> ReviewResult<()> {
        let mut revision = self.source_revision.lock().await;
        let source_revision = match *revision {
            Some(r) => r,
            None => {
                self.app
                    .agent_step_snapshot(&self.caller, &self.job.submission, &self.key)
                    .await
                    .map_err(|_| fence())?
                    .0
            }
        };
        let ordinal = self.cleanup_ordinal.or(*self.ordinal.lock().await);
        // Provider cleanup receipts can follow a just-settled answer on the same store.
        let cleanup = self.cleanup;
        *revision = Some(
            self.app
                .persist_agent_step(
                    &self.caller,
                    &self.job,
                    self.jobs.as_ref(),
                    &self.key,
                    AgentStepUpdate {
                        source_revision,
                        ordinal,
                        version: self.version,
                        claims_revision: self.claims_revision,
                        progress,
                        cleanup,
                    },
                )
                .await
                .map_err(|_| fence())?,
        );
        Ok(())
    }

    async fn execution(&self) -> ReviewResult<Option<mathesis_review::cma::Execution>> {
        let Some(factory) = &self.factory else {
            return Ok(None);
        };
        if !self.jobs.renew(&self.job).await.map_err(|_| fence())? {
            return Err(fence());
        }
        let attempt = if let Some(ordinal) = self.cleanup_ordinal {
            self.app
                .agent_step(&self.caller, &self.job.submission, &self.key)
                .await
                .map_err(|_| fence())?
                .and_then(|s| s.cma_attempts.into_iter().find(|a| a.ordinal == ordinal))
                .ok_or_else(fence)?
        } else {
            if !self.cleanup && !self.active().await? {
                return Err(ReviewError::Output(
                    "CMA work cancelled or superseded".into(),
                ));
            }
            self.app
                .admit_cma_attempt(
                    &self.caller,
                    &self.job,
                    self.jobs.as_ref(),
                    &self.key,
                    &self.logical_step,
                    &factory.operator.workspace,
                )
                .await
                .map_err(|_| fence())?
        };
        // Only operator-funded target/probe work and venue attempts use phase-D
        // supporter attribution. Replays repair a crash after payer admission.
        if !self.cleanup && self.funded && attempt.payer == mathesis_core::model::CmaPayer::Operator
        {
            self.app
                .attribute_agent_run(&self.key)
                .await
                .map_err(|_| fence())?;
        }
        *self.ordinal.lock().await = Some(attempt.ordinal);
        Ok(Some(mathesis_review::cma::Execution {
            client: factory.client(&attempt)?,
            key: if attempt.ordinal == 0 {
                self.key.clone()
            } else {
                format!("{}:a{}", self.key, attempt.ordinal)
            },
        }))
    }
    async fn fallback(&self, kind: mathesis_review::cma::FailureKind) -> ReviewResult<bool> {
        let Some(factory) = &self.factory else {
            return Ok(false);
        };
        if !self.active().await? {
            return Ok(false);
        }
        let ordinal = (*self.ordinal.lock().await).ok_or_else(fence)?;
        let failure = serde_json::from_value(serde_json::to_value(kind).map_err(|_| fence())?)
            .map_err(|_| fence())?;
        self.app
            .fallback_cma_attempt(
                &self.caller,
                &self.job,
                self.jobs.as_ref(),
                &self.key,
                ordinal,
                failure,
                &factory.operator.workspace,
            )
            .await
            .map_err(|_| fence())
    }
}
