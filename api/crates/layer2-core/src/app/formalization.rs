//! After acceptance: formalizing the statements worth it, with the author's
//! consent, and following up the paper's conjectures.

use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, SubmissionId},
    model::*,
};

impl App {
    fn require_accepted(submission: &Submission) -> CoreResult<()> {
        if matches!(submission.status, SubmissionStatus::Accepted { .. }) {
            Ok(())
        } else {
            Err(CoreError::conflict("this follows acceptance"))
        }
    }

    /// An editor proposes formalizing a proved statement.
    pub async fn propose_formalization(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        claim: &ClaimId,
        reason: String,
    ) -> CoreResult<Submission> {
        caller.require(Role::Editor)?;
        validate_reason(&reason)?;
        let mut submission = self.submission(caller, id).await?;
        Self::require_accepted(&submission)?;
        let statement = submission
            .claim(claim)
            .ok_or_else(|| CoreError::not_found("statement", claim.as_str()))?;
        if statement.kind.is_open() {
            return Err(CoreError::invalid(
                "conjectures are followed up, not formalized as proved results",
            ));
        }
        if submission.formalization.item(claim).is_some() {
            return Err(CoreError::conflict("already in the formalization plan"));
        }
        let now = self.ports.clock.now();
        submission.formalization.items.push(FormalizationItem {
            claim: claim.clone(),
            reason,
            state: ItemState::Proposed,
            progress: None,
            updated_at: now,
        });
        self.save(&mut submission).await?;
        Ok(submission)
    }

    /// The author approves or declines a proved statement. Approval queues
    /// target generation and audit for the shared attempt/verifier path.
    pub async fn respond_to_formalization(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        claim: &ClaimId,
        approve: bool,
        reason: String,
    ) -> CoreResult<Submission> {
        let mut submission = self.load(id).await?;
        Self::require_submitter(caller, &submission)?;
        Self::require_accepted(&submission)?;
        if submission.analysis_visibility != Visibility::Public {
            return Err(CoreError::conflict(
                "formalization requires a public accepted paper",
            ));
        }
        let statement = submission
            .claim(claim)
            .ok_or_else(|| CoreError::not_found("statement", claim.as_str()))?;
        if statement.kind.is_open() || !statement.has_proof {
            return Err(CoreError::invalid(
                "only proved paper statements may be formalized",
            ));
        }
        if submission.formalization.item(claim).is_none() {
            submission.formalization.items.push(FormalizationItem {
                claim: claim.clone(),
                reason: "Author requested formalization".into(),
                state: ItemState::Proposed,
                progress: None,
                updated_at: self.ports.clock.now(),
            });
        }
        let now = self.ports.clock.now();
        let item = submission
            .formalization
            .items
            .iter_mut()
            .find(|i| &i.claim == claim)
            .ok_or_else(|| CoreError::not_found("plan item", claim.as_str()))?;
        if item.state != ItemState::Proposed {
            return Err(CoreError::conflict(format!(
                "the item is {}",
                item.state.name()
            )));
        }
        item.state = if approve {
            ItemState::Approved
        } else {
            ItemState::Declined {
                reason: reason.chars().take(2_000).collect(),
            }
        };
        if approve {
            item.progress = Some(FormalizationProgress {
                state: FormalizationProgressState::TargetPreparing,
                target_digest: None,
                toolchain: None,
                attempts: 0,
                completed_lemmas: vec![],
                remaining_obligations: vec![],
                next_step: Some("Mathesis is preparing an exact Lean statement.".into()),
                note: None,
                updated_at: now,
            });
        } else {
            item.progress = None;
        }
        item.updated_at = now;
        self.save(&mut submission).await?;
        if approve {
            self.ports
                .queue
                .enqueue(id, crate::ports::JobKind::LeanStatement)
                .await?;
        }
        Ok(submission)
    }

    /// Update the durable author-facing projection of the continuous service.
    /// The worker is the only caller; provider transcripts remain in its
    /// private run store and are never copied into this aggregate.
    pub async fn record_formalization_progress(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        claim: &ClaimId,
        mut progress: FormalizationProgress,
    ) -> CoreResult<Submission> {
        caller.require(Role::Reviewer)?;
        let mut submission = self.submission(caller, id).await?;
        Self::require_accepted(&submission)?;
        if submission.analysis_visibility != Visibility::Public {
            return Err(CoreError::conflict(
                "formalization requires a public accepted paper",
            ));
        }
        if let Some(digest) = &progress.target_digest {
            let SubmissionStatus::Accepted { record } = &submission.status else {
                unreachable!()
            };
            let target = self.target(record, claim).await?;
            if &target.digest != digest {
                return Err(CoreError::conflict("formalization target changed"));
            }
            let file = self.solve_file(record, claim).await?;
            progress.toolchain = Some(target.toolchain);
            let receipt = file
                .winner(digest)
                .and_then(|attempt| attempt.receipt.as_ref());
            if let Some(receipt) = receipt {
                progress.state = if receipt.verdict == Verdict::Proved {
                    FormalizationProgressState::Verified
                } else {
                    FormalizationProgressState::Blocked
                };
                progress.next_step = Some(if receipt.verdict == Verdict::Proved {
                    "An independently checked proof is available.".into()
                } else {
                    "A checked disproof requires review before proof work continues.".into()
                });
                progress.remaining_obligations.clear();
            } else if progress.state == FormalizationProgressState::Verified {
                return Err(CoreError::conflict(
                    "verified progress requires a verifier receipt",
                ));
            }
        } else if progress.state == FormalizationProgressState::Verified {
            return Err(CoreError::conflict(
                "verified progress requires a confirmed target",
            ));
        }
        let item = submission
            .formalization
            .items
            .iter_mut()
            .find(|item| &item.claim == claim)
            .ok_or_else(|| CoreError::not_found("formalization item", claim.as_str()))?;
        if matches!(item.state, ItemState::Proposed | ItemState::Declined { .. }) {
            return Err(CoreError::conflict(
                "formalization needs the author's approval",
            ));
        }
        let progress = progress.bounded();
        if progress.state != FormalizationProgressState::Verified
            && matches!(
                progress.state,
                FormalizationProgressState::ProofQueued
                    | FormalizationProgressState::InProgress
                    | FormalizationProgressState::Partial
                    | FormalizationProgressState::Blocked
            )
            && matches!(item.state, ItemState::Approved)
        {
            item.state = ItemState::InProgress;
        }
        item.progress = Some(progress);
        item.updated_at = self.ports.clock.now();
        self.save(&mut submission).await?;
        Ok(submission)
    }

    /// An editor records what is happening with a conjecture of the paper.
    /// Taking it up opens a probe task when the author lets contributors help.
    pub async fn update_conjecture(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        claim: &ClaimId,
        state: ConjectureState,
    ) -> CoreResult<Submission> {
        caller.require(Role::Editor)?;
        let mut submission = self.submission(caller, id).await?;
        Self::require_accepted(&submission)?;
        if !submission.claim(claim).is_some_and(|c| c.kind.is_open()) {
            return Err(CoreError::invalid(
                "only the paper's conjectures and questions are followed up",
            ));
        }
        if submission.kind == SubmissionKind::Conjecture
            && matches!(
                state,
                ConjectureState::TakenUp | ConjectureState::Settled { .. }
            )
        {
            return Err(CoreError::conflict(
                "conjecture attacks belong to phase B; only an author-confirmed Lean statement may be attacked",
            ));
        }
        match &state {
            ConjectureState::NotPursued { reason } => validate_reason(reason)?,
            ConjectureState::Settled { summary, .. } => validate_reason(summary)?,
            _ => {}
        }
        let now = self.ports.clock.now();
        let taken_up = state == ConjectureState::TakenUp;
        match submission
            .conjectures
            .iter_mut()
            .find(|c| &c.claim == claim)
        {
            Some(follow_up) => {
                follow_up.state = state;
                follow_up.updated_at = now;
            }
            None => submission.conjectures.push(ConjectureFollowUp {
                claim: claim.clone(),
                state,
                updated_at: now,
            }),
        }
        let open = submission.open_to_contributors;
        self.save(&mut submission).await?;
        if taken_up && open {
            self.open_task(caller, id, claim, TaskKind::Probe).await?;
        }
        Ok(submission)
    }
}
