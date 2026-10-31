use std::{sync::Arc, time::Duration};

use mathesis_core::{
    CoreError,
    app::{App, FiledBy},
    model::{Caller, ReviewerIdentity, Stage, Submission, SubmissionStatus},
    ports::{JobKind, JobLane},
};
use mathesis_review::{Document, ReviewModel};
use tokio::sync::watch;
use tracing::Instrument;

use super::{JobLease, LeasedJob, kind_key, mapping};
use crate::latex::{self, Compile};

const IDLE_POLL: Duration = Duration::from_secs(3);
const RECONCILE_INTERVAL: Duration = Duration::from_secs(60);
const LEASE_HEARTBEAT: Duration = Duration::from_secs(5 * 60);

pub struct Worker {
    pub knowledge: Option<Arc<dyn crate::knowledge::KnowledgeSource>>,
    pub review_concurrency: usize,
    pub cma_factory: Option<Arc<crate::cma_tokens::CmaFactory>>,
    pub app: Arc<App>,
    pub jobs: Arc<dyn JobLease>,
    pub compile: Compile,
    pub model: Option<Arc<dyn ReviewModel>>,
    pub reviewer: Caller,
    pub referee_account: Caller,
    pub auditor_account: Caller,
    pub referee_model: String,
    pub oracle: Option<Arc<dyn mathesis_review::oracle::Oracle>>,
    pub advisor: Option<Arc<dyn mathesis_review::advisor::Advisor>>,
    pub formalizer: Option<Arc<dyn mathesis_review::lean::Formalizer>>,
    pub oracle_poll: Duration,
    pub advisor_work_dir: std::path::PathBuf,
    pub audit_budget: Duration,
    pub formal_budget: Duration,
}

/// Whether a failure is worth retrying.
#[derive(Debug)]
pub(super) enum Failure {
    Transient(String),
    Permanent(String),
    LeaseLost,
}

impl From<CoreError> for Failure {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Unavailable(_) | CoreError::StaleRevision { .. } => {
                Self::Transient(error.to_string())
            }
            other => Self::Permanent(other.to_string()),
        }
    }
}

pub(super) fn model_failure(error: mathesis_review::ReviewError) -> Failure {
    match error {
        mathesis_review::ReviewError::PersistenceFence => Failure::Transient(error.to_string()),
        mathesis_review::ReviewError::Transport(e) => Failure::Transient(e),
        mathesis_review::ReviewError::Provider { status, body }
            if status == 429 || status >= 500 =>
        {
            Failure::Transient(format!("{status}: {body}"))
        }
        other => Failure::Permanent(other.to_string()),
    }
}

impl Worker {
    pub async fn run(self, shutdown: watch::Receiver<bool>) {
        let lanes = std::iter::once(JobLane::Compile)
            .chain(std::iter::once(JobLane::Verify))
            .chain(std::iter::repeat_n(
                JobLane::Review,
                self.review_concurrency,
            ));
        futures::future::join_all(lanes.map(|lane| self.run_lane(lane, shutdown.clone()))).await;
    }

    async fn run_lane(&self, lane: JobLane, mut shutdown: watch::Receiver<bool>) {
        tracing::info!(
            model = self.model.as_ref().map(|m| m.model()),
            "paper worker started"
        );
        loop {
            if *shutdown.borrow() {
                break;
            }
            match self.jobs.claim_in(Some(lane)).await {
                Ok(Some(job)) => self.handle_until_shutdown(job, &mut shutdown).await,
                Ok(None) => {
                    tokio::select! {
                        _ = tokio::time::sleep(IDLE_POLL) => {}
                        _ = shutdown.changed() => {}
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "could not claim a job");
                    tokio::time::sleep(IDLE_POLL).await;
                }
            }
        }
        tracing::info!("paper worker stopped");
    }

    pub(super) async fn handle_until_shutdown(
        &self,
        job: LeasedJob,
        shutdown: &mut watch::Receiver<bool>,
    ) {
        // Drop the local operation and its heartbeat before releasing ownership.
        // Provider work keeps its durable IDs and resumes under the next lease.
        let interrupted = tokio::select! {
            biased;
            _ = shutdown.wait_for(|requested| *requested) => true,
            _ = self.handle(job.clone()) => false,
        };
        if interrupted {
            if let Err(error) = self.jobs.defer(&job, Duration::ZERO).await {
                tracing::warn!(%error, "could not release job on shutdown; lease expiry will recover it");
            } else {
                tracing::info!(submission = %job.submission, "job released on shutdown; admitted work retained");
            }
        }
    }

    pub(super) async fn handle(&self, job: LeasedJob) {
        let span = tracing::info_span!("paper_job", submission = %job.submission, kind = kind_key(job.kind), attempt = job.attempts);
        // The process future is cancelled before it can persist a result when
        // renewal loses ownership (or cannot establish it). No detached
        // heartbeat remains after a job completes or defers.
        let result = async {
            if !self.jobs.renew(&job).await? {
                return Err(Failure::LeaseLost);
            }
            let process = self.process(&job);
            tokio::pin!(process);
            loop {
                tokio::select! {
                    biased;
                    _ = tokio::time::sleep(LEASE_HEARTBEAT) => {
                        if !self.jobs.renew(&job).await? {
                            break Err(Failure::LeaseLost);
                        }
                    }
                    result = &mut process => break result,
                }
            }
        }
        .instrument(span)
        .await;
        // Public proof jobs own admitted provider work through authorization changes,
        // failures and asynchronous cleanup. Deferral retains the original lease job
        // without spending its retry budget or admitting a replacement generation.
        if job.kind == JobKind::AgentAttempt
            && !matches!(result, Ok(true) | Err(Failure::LeaseLost))
        {
            match self.cleanup_agents(&job).await {
                Ok(false) => {}
                pending => {
                    if let Err(error) = pending {
                        tracing::warn!(?error, "proof work cleanup will be resumed");
                    }
                    if let Err(error) = self.jobs.defer(&job, Duration::from_secs(30)).await {
                        tracing::error!(%error, "could not defer proof cleanup; lease expiry will recover it");
                    }
                    return;
                }
            }
        }
        let settled = match result {
            Ok(true) => Ok(()),
            Ok(false) => {
                // A delivered review can still own an asynchronously stopping agent.
                // Keep its original job without consuming retry attempts or rerunning review.
                if matches!(
                    job.kind,
                    JobKind::Referee | JobKind::ResearchTarget | JobKind::BindingAudit
                ) {
                    match self.cleanup_agents(&job).await {
                        Ok(true) => self.jobs.defer(&job, Duration::from_secs(30)).await,
                        Ok(false) => self.jobs.complete(&job).await,
                        Err(Failure::Permanent(reason)) => {
                            tracing::warn!(reason, "review cleanup cannot continue");
                            self.jobs.complete(&job).await
                        }
                        Err(error) => {
                            tracing::warn!(?error, "review cleanup will be resumed");
                            self.jobs.defer(&job, Duration::from_secs(30)).await
                        }
                    }
                } else {
                    self.jobs.complete(&job).await
                }
            }
            Err(Failure::Permanent(reason)) => {
                tracing::warn!(reason, "job abandoned");
                self.jobs.complete(&job).await
            }
            Err(Failure::Transient(reason)) => {
                tracing::warn!(reason, "job will be retried");
                self.jobs.retry(&job, &reason).await
            }
            Err(Failure::LeaseLost) => {
                tracing::warn!("job lease lost; pending result discarded");
                Ok(())
            }
        };
        if let Err(error) = settled {
            tracing::error!(%error, "could not settle job; its lease will expire and it will be retried");
        }
    }

    async fn process(&self, job: &LeasedJob) -> Result<bool, Failure> {
        if job.kind == JobKind::VerifyResearch {
            self.app
                .verify_research_leased(&self.reviewer, job, self.jobs.as_ref())
                .await?;
            return Ok(false);
        }
        if job.kind == JobKind::VerifyAttempt {
            self.ensure_lease(job).await?;
            self.app
                .verify_attempt_leased(&self.reviewer, job, self.jobs.as_ref())
                .await?;
            return Ok(false);
        }
        if job.kind == JobKind::FeedbackReadback {
            return self.feedback_readback_job(job).await;
        }
        if job.kind.lane() == JobLane::Review {
            self.cleanup_agents(job).await?;
        }
        if job.kind == JobKind::ErrorReports {
            return self.error_reports_job(job).await;
        }
        if job.kind == JobKind::AgentAttempt {
            return self.agent_attempt_job(job).await;
        }
        if job.kind == JobKind::OpenProblems {
            return self.open_problems_job(job).await;
        }
        if job.kind == JobKind::BindingAudit {
            return self.binding_audit_job(job).await;
        }
        if job.kind == JobKind::ResearchTarget {
            return self.research_target_job(job).await;
        }
        if job.kind == JobKind::LeanStatement {
            return self.lean_statement_job(job).await;
        }
        if job.kind == JobKind::Referee {
            return self.referee_job(job).await;
        }
        let submission = match self.app.submission(&self.reviewer, &job.submission).await {
            Ok(submission) => submission,
            Err(CoreError::NotFound { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        match job.kind {
            JobKind::ErrorReports
            | JobKind::AgentAttempt
            | JobKind::OpenProblems
            | JobKind::VerifyAttempt
            | JobKind::FeedbackReadback
            | JobKind::BindingAudit
            | JobKind::ResearchTarget
            | JobKind::VerifyResearch
            | JobKind::LeanStatement
            | JobKind::Referee => unreachable!("referee handled above"),
            JobKind::Compile => self.compile(job, &submission).await,
            // Literature is supplied by the networked referee and durable CMA audit.
            JobKind::Stage(Stage::Literature) => Ok(()),
            JobKind::Stage(Stage::Escape) => self.escape(job, &submission).await,
            // The compile job files S0; the author confirms S1.
            JobKind::Stage(Stage::Hygiene | Stage::Claims) => Ok(()),
        }?;
        Ok(false)
    }

    /// Recheck ownership immediately before writing a result, including a
    /// lease lost between periodic heartbeat ticks.
    pub(super) async fn ensure_lease(&self, job: &LeasedJob) -> Result<(), Failure> {
        if self.jobs.renew(job).await? {
            Ok(())
        } else {
            Err(Failure::LeaseLost)
        }
    }

    async fn source(&self, submission: &Submission) -> Result<(Vec<u8>, String), Failure> {
        let version = submission
            .current_version()
            .ok_or_else(|| Failure::Permanent("the paper has no version".into()))?;
        let file = self
            .app
            .paper_file(
                Some(&self.reviewer),
                &submission.id,
                Some(version.number),
                false,
            )
            .await?;
        Ok((file.bytes, version.filename.clone()))
    }

    async fn compile(&self, job: &LeasedJob, submission: &Submission) -> Result<(), Failure> {
        if submission.status == SubmissionStatus::Withdrawn {
            return Ok(());
        }
        let Some(version) = submission.current_version() else {
            return Ok(());
        };
        if version.pdf.is_some() || version.compile_error.is_some() {
            return Ok(());
        }
        let number = version.number;
        let (bytes, filename) = self.source(submission).await?;
        let outcome = self.compile.pdf(bytes, filename).await;
        if let Err(message) = &outcome {
            tracing::info!(version = number, message = %message.lines().last().unwrap_or_default(), "source did not compile");
        }
        self.ensure_lease(job).await?;
        self.app
            .record_compilation(&self.reviewer, &submission.id, number, outcome)
            .await?;
        Ok(())
    }

    fn filed_by(&self, model: &dyn ReviewModel) -> FiledBy {
        FiledBy {
            engine: model.engine().to_owned(),
            model: Some(model.model().to_owned()),
        }
    }

    async fn escape(&self, job: &LeasedJob, submission: &Submission) -> Result<(), Failure> {
        let Some(model) = &self.model else {
            return Ok(());
        };
        if !submission.is_open() {
            return Ok(());
        }
        // Once per claim set: skip when this account already judged it.
        let judged = self
            .app
            .judgements(&self.reviewer, &submission.id)
            .await?
            .iter()
            .any(|j| matches!(&j.reviewer, ReviewerIdentity::Machine { account, .. } if account == &self.reviewer.person));
        if judged {
            return Ok(());
        }
        let (bytes, filename) = self.source(submission).await?;
        let document = Document {
            title: submission.title.clone(),
            abstract_text: submission.abstract_text.clone(),
            text: latex::source_text(&bytes, &filename).ok(),
        };
        let (proposals, _usage) = model
            .assess_escape(&document, &mapping::statements(&submission.claims))
            .await
            .map_err(model_failure)?;
        let (kept, skipped) = mapping::proposals(&proposals, &submission.claims);
        if !skipped.is_empty() {
            tracing::info!(skipped = skipped.join(", "), "escape proposals dropped");
        }
        for (claim, output) in kept {
            self.ensure_lease(job).await?;
            self.app
                .file_machine_judgement(
                    &self.reviewer,
                    &submission.id,
                    &claim,
                    output,
                    self.filed_by(model.as_ref()),
                )
                .await?;
        }
        Ok(())
    }
}

/// Return expired task leases to the pool.
pub async fn run_reconciler(app: Arc<App>, mut shutdown: watch::Receiver<bool>) {
    loop {
        match app.reconcile().await {
            Ok(outcome) if outcome.leases_expired > 0 => {
                tracing::info!(expired = outcome.leases_expired, "task leases expired");
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "reconcile failed"),
        }
        tokio::select! {
            _ = tokio::time::sleep(RECONCILE_INTERVAL) => {}
            _ = shutdown.changed() => break,
        }
    }
}
