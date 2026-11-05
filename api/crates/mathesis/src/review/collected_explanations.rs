//! Public reading guides use the exact collected source, without manuscript admission.
use super::{
    LeasedJob,
    worker::{Failure, Worker},
};
use async_trait::async_trait;
use mathesis_core::{app::App, model::*, ports::JobLease};
use mathesis_review::{
    ReviewError, ReviewResult,
    advisor::AdvisorInput,
    cma::{RunState, RunStore},
    referee_prompts::RefereeOut,
};
use serde_json::Value;
use std::{sync::Arc, time::Duration};

struct CollectionRun {
    app: Arc<App>,
    jobs: Arc<dyn JobLease>,
    job: LeasedJob,
    caller: Caller,
    work: String,
    version: u32,
    key: String,
    revision: tokio::sync::Mutex<Option<u64>>,
}
impl CollectionRun {
    async fn read(&self) -> ReviewResult<CollectedVersion> {
        if !self
            .jobs
            .renew(&self.job)
            .await
            .map_err(|_| ReviewError::PersistenceFence)?
        {
            return Err(ReviewError::PersistenceFence);
        }
        self.app
            .collected_explanation_input(&self.caller, &self.work, self.version)
            .await
            .map_err(|_| ReviewError::PersistenceFence)
    }
}
#[async_trait]
impl RunStore for CollectionRun {
    fn key(&self) -> &str {
        &self.key
    }
    async fn active(&self) -> ReviewResult<bool> {
        Ok(self.read().await?.explanation.text.is_none())
    }
    async fn load(&self) -> ReviewResult<Option<Value>> {
        let value = self.read().await?.explanation;
        *self.revision.lock().await = Some(value.revision);
        Ok((!value.progress.is_null()).then_some(value.progress))
    }
    async fn save(&self, progress: Value) -> ReviewResult<()> {
        let mut value = self.read().await?.explanation;
        let mut expected = self.revision.lock().await;
        if expected.is_some_and(|r| r != value.revision) {
            return Err(ReviewError::PersistenceFence);
        }
        let revision = value.revision;
        value.revision += 1;
        value.progress = progress;
        self.app
            .save_collected_explanation(&self.caller, &self.work, self.version, value, revision)
            .await
            .map_err(|_| ReviewError::PersistenceFence)?;
        *expected = Some(revision + 1);
        Ok(())
    }
}

impl Worker {
    pub(super) async fn collected_explanation_job(&self, job: &LeasedJob) -> Result<bool, Failure> {
        let Some((work, version)) = job
            .submission
            .as_str()
            .strip_prefix("collected:")
            .and_then(|s| s.rsplit_once(':'))
        else {
            return Err(Failure::Permanent("invalid collection job identity".into()));
        };
        let version = version
            .parse::<u32>()
            .map_err(|_| Failure::Permanent("invalid collection version".into()))?;
        let paper = self
            .app
            .collected_explanation_input(&self.reviewer, work, version)
            .await?;
        let advisor = self
            .advisor
            .as_ref()
            .ok_or_else(|| Failure::Transient("explanation service unavailable".into()))?;
        let store = CollectionRun {
            app: self.app.clone(),
            jobs: self.jobs.clone(),
            job: job.clone(),
            caller: self.reviewer.clone(),
            work: work.into(),
            version,
            key: format!("mathesis:collection-explanation:{}", paper.id),
            revision: tokio::sync::Mutex::new(None),
        };
        let result = if paper.explanation.text.is_none() {
            let source = self.app.collected_file(work, version, "source").await?;
            let input = AdvisorInput {
                letter_contract: 0,
                manuscript_origin: None,
                computations: None,
                knowledge: None,
                review_method: None,
                confirmed_claims: None,
                followup: None,
                kind: "note".into(),
                title: paper.delivery.paper.title.clone(),
                abstract_text: paper.delivery.paper.abstract_text.clone(),
                authors: paper
                    .delivery
                    .paper
                    .authors
                    .iter()
                    .map(|a| a.name.clone())
                    .collect(),
                statements: vec![],
                referee: RefereeOut::default(),
                audit: None,
                decision: None,
                text: Some(
                    crate::latex::source_text(&source.bytes, &source.filename)
                        .map_err(Failure::Permanent)?,
                ),
                source_dir: None,
                main_file: None,
            };
            advisor.explain_with_store(&input, &store).await.map(Some)
        } else {
            Ok(None)
        };
        // Lost persistence ownership leaves the original admitted run to its next
        // lease holder. Never cancel that holder's conversation from stale work.
        if matches!(result, Err(ReviewError::PersistenceFence)) {
            return Err(Failure::LeaseLost);
        }
        if let Ok(Some(output)) = &result {
            self.ensure_lease(job).await?;
            let mut explanation = self
                .app
                .collected_explanation_input(&self.reviewer, work, version)
                .await?
                .explanation;
            let expected = explanation.revision;
            explanation.revision += 1;
            explanation.text = Some(PlainLanguageText {
                en: output.en.clone(),
                zh: output.zh.clone(),
            });
            self.app
                .save_collected_explanation(&self.reviewer, work, version, explanation, expected)
                .await?;
        }
        // An already delivered guide can still own a stopping agent. Keep the same
        // job and conversation until actual deletion, including after worker restart.
        if let Some(client) = advisor.managed_client()
            && let Err(error) = client.cancel(&store).await
        {
            tracing::warn!(%error, "collection explanation cleanup remains owned");
        }
        let state = store.load().await.map_err(super::worker::model_failure)?;
        if state
            .and_then(|v| serde_json::from_value::<RunState>(v).ok())
            .is_some_and(|s| !s.deleted && (s.agent_id.is_some() || s.create_intent.is_some()))
        {
            self.jobs.defer(job, Duration::from_secs(30)).await?;
            return Ok(true);
        }
        result.map_err(super::worker::model_failure)?;
        Ok(false)
    }
}
