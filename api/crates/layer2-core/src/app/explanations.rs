use super::App;
use crate::{CoreError, CoreResult, ids::SubmissionId, model::*, ports::JobKind};

impl App {
    /// Staff can backfill or retry exposition without repeating mathematical review.
    pub async fn request_paper_explanation(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<Submission> {
        caller.require(Role::Admin)?;
        let mut paper = self.load(id).await?;
        explanation_eligible(&paper)?;
        let version = paper.versions.last_mut().unwrap();
        if version.plain_language.as_ref().is_some_and(|e| {
            e.claims_revision == paper.claims_revision && e.source_sha256 == version.archive.sha256
        }) {
            return Ok(paper);
        }
        version.explanation_generation = version
            .explanation_generation
            .checked_add(1)
            .ok_or_else(|| CoreError::conflict("explanation generation exhausted"))?;
        self.save(&mut paper).await?;
        self.ports
            .queue
            .enqueue(id, JobKind::PaperExplanation)
            .await?;
        Ok(paper)
    }

    /// Only source-bound exposition is saved. This operation changes no review,
    /// admission, visibility, formal proof or contributor credit.
    pub async fn record_paper_explanation(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        input: NewPaperExplanation,
    ) -> CoreResult<Submission> {
        caller.require(Role::Reviewer)?;
        input.text.validate()?;
        let mut paper = self.load(id).await?;
        explanation_eligible(&paper)?;
        let claims_revision = paper.claims_revision;
        let version = paper.versions.last_mut().unwrap();
        if version.number != input.version
            || claims_revision != input.claims_revision
            || version.archive.sha256 != input.source_sha256
            || version.explanation_generation != input.generation
        {
            return Err(CoreError::conflict("the explanation inputs changed"));
        }
        if let Some(existing) = &version.plain_language
            && existing.claims_revision == claims_revision
            && existing.source_sha256 == input.source_sha256
        {
            if existing.text == input.text {
                return Ok(paper);
            }
            if !caller.has(Role::Admin) {
                return Err(CoreError::conflict(
                    "this version already has an explanation",
                ));
            }
        }
        version.plain_language = Some(PaperExplanation {
            version: input.version,
            claims_revision,
            source_sha256: input.source_sha256,
            text: input.text,
            prepared_at: self.ports.clock.now(),
        });
        self.save(&mut paper).await?;
        Ok(paper)
    }
}

fn explanation_eligible(paper: &Submission) -> CoreResult<()> {
    if paper.claims_revision == 0
        || paper.current_version().is_none()
        || matches!(
            paper.status,
            SubmissionStatus::Draft | SubmissionStatus::Withdrawn
        )
    {
        return Err(CoreError::conflict(
            "confirm the manuscript before preparing its explanation",
        ));
    }
    Ok(())
}
