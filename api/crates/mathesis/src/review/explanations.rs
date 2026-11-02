use super::{
    mapping,
    worker::{Failure, Worker},
};
use mathesis_core::{model::*, ports::LeasedJob};
use mathesis_review::{advisor::AdvisorInput, referee_prompts::RefereeOut};

impl Worker {
    pub(super) async fn paper_explanation_job(&self, job: &LeasedJob) -> Result<bool, Failure> {
        let paper = self.app.submission(&self.reviewer, &job.submission).await?;
        if paper.claims_revision == 0 || !matches!(paper.status, SubmissionStatus::Accepted { .. })
        {
            return Ok(false);
        }
        let Some(version) = paper.current_version() else {
            return Ok(false);
        };
        if version.plain_language.as_ref().is_some_and(|e| {
            e.claims_revision == paper.claims_revision && e.source_sha256 == version.archive.sha256
        }) {
            return Ok(false);
        }
        let Some(advisor) = &self.advisor else {
            return Ok(false);
        };
        // Use source and metadata only. No private audit, advice or letter is
        // supplied to this independently recoverable exposition task.
        let source = self
            .app
            .paper_file(
                Some(&self.referee_account),
                &paper.id,
                Some(version.number),
                false,
            )
            .await?;
        let input = AdvisorInput {
            letter_contract: 0,
            computations: None,
            knowledge: None,
            review_method: None,
            confirmed_claims: None,
            followup: None,
            kind: serde_json::to_value(paper.kind)
                .unwrap()
                .as_str()
                .unwrap()
                .into(),
            title: paper.title.clone(),
            abstract_text: paper.abstract_text.clone(),
            authors: paper
                .authors
                .iter()
                .map(|author| author.name.clone())
                .collect(),
            statements: mapping::confirmed_statements(&paper.claims),
            referee: RefereeOut::default(),
            audit: None,
            decision: None,
            text: Some(
                crate::latex::source_text(&source.bytes, &version.filename)
                    .map_err(Failure::Permanent)?,
            ),
            source_dir: None,
            main_file: None,
        };
        let label = format!("explanation:{}", version.explanation_generation);
        let output = advisor
            .explain_with_store(&input, &self.agent_store(job, &paper, &label))
            .await
            .map_err(super::worker::model_failure)?;
        self.ensure_lease(job).await?;
        self.app
            .record_paper_explanation(
                &self.reviewer,
                &paper.id,
                NewPaperExplanation {
                    version: version.number,
                    claims_revision: paper.claims_revision,
                    source_sha256: version.archive.sha256.clone(),
                    generation: version.explanation_generation,
                    text: PlainLanguageText {
                        en: output.en,
                        zh: output.zh,
                    },
                },
            )
            .await?;
        Ok(false)
    }
}
