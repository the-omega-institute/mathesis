use sha2::{Digest, Sha256};

use super::{App, Upload};
use crate::{CoreError, CoreResult, ids::SubmissionId, model::*};

impl App {
    /// Save a private, incomplete draft and use the existing durable compile lane.
    /// No review, publication or claimed authorship is admitted by preparation.
    pub async fn prepare_submission(
        &self,
        caller: &Caller,
        preparation: PrepareSubmission,
        upload: Upload,
    ) -> CoreResult<Submission> {
        let new = NewPaper {
            kind: preparation.kind,
            purpose: preparation.purpose,
            processing_consent: preparation.processing_consent,
            title: None,
            abstract_text: None,
            authors: vec![],
            ai_disclosure: AiDisclosure {
                level: AiUse::None,
                statement: "Awaiting the author's disclosure before submission.".into(),
            },
            make_public_after_acceptance: false,
            open_to_contributors: false,
            typed_conjecture: None,
            review_scope: String::new(),
            msc: vec![],
            doi: None,
        };
        self.submit_paper_inner(caller, new, upload, None, true)
            .await
    }

    pub async fn submit_prepared(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        mut request: SubmitPrepared,
    ) -> CoreResult<Submission> {
        request.metadata.normalise_doi();
        self.confirm_claims_inner(caller, id, request.claims.clone(), None, Some(&request))
            .await
    }

    /// Apply metadata and source-bound statement confirmation in the same fenced
    /// aggregate write. Return true for an exact completed-request replay.
    pub(super) fn apply_prepared_metadata(
        &self,
        submission: &mut Submission,
        request: &SubmitPrepared,
    ) -> CoreResult<bool> {
        let intake = submission
            .intake
            .as_ref()
            .ok_or_else(|| CoreError::conflict("this submission is not an upload-first draft"))?;
        let version = submission
            .current_version()
            .filter(|v| v.number == request.version && v.archive.sha256 == request.source_sha256)
            .ok_or_else(|| CoreError::conflict("the manuscript changed; check the current PDF"))?;
        let digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(request)
                    .map_err(|_| CoreError::invalid("submission metadata"))?
            )
        );
        if let Some(saved) = &intake.confirmed_request {
            if saved == &digest && submission.status != SubmissionStatus::Withdrawn {
                return Ok(true);
            }
            return Err(CoreError::conflict("this draft has already been submitted"));
        }
        if submission.status != SubmissionStatus::Draft
            || submission.claims_revision != request.claims_revision
        {
            return Err(CoreError::conflict(
                "the draft changed; reload before submitting",
            ));
        }
        if version.pdf.is_none() || version.compile_error.is_some() {
            return Err(CoreError::conflict(
                "wait for a readable PDF before submitting",
            ));
        }
        let new = &request.metadata;
        new.validate()?;
        if new.kind != submission.kind
            || new.purpose != submission.purpose
            || new.typed_conjecture.is_some()
        {
            return Err(CoreError::invalid(
                "the manuscript type and review purpose must match the upload",
            ));
        }
        let title = new
            .title
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| CoreError::invalid("enter the manuscript title"))?;
        if new.authors.is_empty() {
            return Err(CoreError::invalid("list the manuscript authors"));
        }
        submission.title = title.clone();
        submission.abstract_text = new.abstract_text.clone().unwrap_or_default();
        submission.authors = new.authors.clone();
        submission.ai_disclosure = new.ai_disclosure.clone();
        submission.msc = new.msc.clone();
        submission.doi = new.doi.clone();
        submission.review_scope = new.review_scope.clone();
        submission.open_to_contributors = new.open_to_contributors;
        submission.analysis_visibility =
            if new.purpose == SubmissionPurpose::Publication && new.make_public_after_acceptance {
                Visibility::Public
            } else {
                Visibility::Private
            };
        submission.intake.as_mut().unwrap().confirmed_request = Some(digest);
        Ok(false)
    }
}
