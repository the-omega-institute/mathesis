//! Author-owned private follow-up. State and evidence retain their original pins;
//! a verifier receipt never updates the mathematical audit automatically.
use super::App;
use crate::{
    CoreError, CoreResult,
    ids::SubmissionId,
    model::*,
    ports::{JobKind, JobLease, LeasedJob, SubmissionFilter},
};
use sha2::{Digest, Sha256};

fn digest(value: &impl serde::Serialize) -> CoreResult<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(
            serde_json::to_vec(value)
                .map_err(|_| CoreError::invalid("research input cannot be encoded"))?
        )
    ))
}

fn candidates(submission: &Submission, file: &RefereeFile) -> CoreResult<Vec<ResearchCandidate>> {
    if matches!(
        submission.status,
        SubmissionStatus::Draft | SubmissionStatus::Withdrawn
    ) {
        return Ok(vec![]);
    }
    let Some(round) = file.rounds.iter().rev().find(|r| {
        Some(r.version) == submission.current_version().map(|v| v.number)
            && r.claims_revision == submission.claims_revision
            && r.audit.done().is_some()
    }) else {
        return Ok(vec![]);
    };
    let receipts = file
        .knowledge_events
        .iter()
        .filter_map(|e| match e {
            KnowledgeEvent::QueryCompleted { receipt } if receipt.subject.round == round.number => {
                Some(receipt.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut result = vec![];
    for reading in &round.audit.done().unwrap().claims {
        let Some(coverage) = &reading.coverage else {
            continue;
        };
        if !matches!(
            coverage.relation,
            CoverageRelation::Candidate | CoverageRelation::Unknown
        ) || !matches!(
            coverage.qualification,
            CoverageQualification::SourceAudited { .. }
        ) || coverage.validate(&receipts).is_err()
        {
            continue;
        }
        if coverage.subject.claim != reading.claim
            || coverage.subject.round != round.number
            || !subject_current(submission, &coverage.subject)
        {
            continue;
        }
        let Some(binding) = &coverage.binding else {
            continue;
        };
        let Some(claim) = submission.claim(&reading.claim) else {
            continue;
        };
        let gaps = binding
            .hypotheses
            .iter()
            .enumerate()
            .filter(|(_, h)| h.argument.trim().is_empty())
            .map(|(index, h)| (ResearchGap::Hypothesis { index }, h.hypothesis.clone()))
            .chain(
                binding
                    .unresolved
                    .iter()
                    .enumerate()
                    .map(|(index, text)| (ResearchGap::Bridge { index }, text.clone())),
            );
        for (gap, text) in gaps {
            let digest = digest(&(
                &coverage.subject,
                &gap,
                &text,
                &claim.statement,
                &coverage.receipt_ids,
                binding,
            ))?;
            result.push(ResearchCandidate {
                digest,
                subject: coverage.subject.clone(),
                gap,
                unresolved_text: text,
                original_statement: claim.statement.clone(),
                receipt_ids: coverage.receipt_ids.clone(),
                binding: binding.clone(),
            });
        }
    }
    Ok(result)
}

pub(super) fn subject_current(submission: &Submission, subject: &CoverageSubject) -> bool {
    !matches!(
        submission.status,
        SubmissionStatus::Draft | SubmissionStatus::Withdrawn
    ) && subject.submission == submission.id
        && submission.current_version().is_some_and(|v| {
            v.number == subject.version
                && format!("sha256:{}", v.archive.sha256) == subject.source_digest
        })
        && submission.claims_revision == subject.claims_revision
        && submission
            .claim(&subject.claim)
            .is_some_and(|c| digest(c).is_ok_and(|d| d == subject.claim_digest))
}

fn resolved(submission: &Submission, file: &RefereeFile, task: &ResearchObligation) -> bool {
    file.rounds
        .iter()
        .rev()
        .find(|r| {
            r.version == task.candidate.subject.version
                && r.claims_revision == submission.claims_revision
                && r.audit.done().is_some()
        })
        .and_then(|r| r.audit.done())
        .is_some_and(|a| {
            a.claims.iter().any(|c| {
                c.claim == task.candidate.subject.claim
                    && c.coverage.as_ref().is_some_and(|coverage| {
                        subject_current(submission, &coverage.subject)
                            && matches!(
                                coverage.relation,
                                CoverageRelation::Covered | CoverageRelation::Refuted
                            )
                    })
            })
        })
}
fn active(
    submission: &Submission,
    file: &RefereeFile,
    task: &ResearchObligation,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    subject_current(submission, &task.candidate.subject)
        && !task.stopped()
        && !resolved(submission, file, task)
        && task.budget.expires_at > now
        && task.consent == RESEARCH_CONSENT
        && submission
            .processing_consent
            .as_ref()
            .is_some_and(|c| c.version == AI_PROCESSING_CONSENT_VERSION)
}
fn confirmed(task: &ResearchObligation) -> CoreResult<LeanStatementAttempt> {
    task.target()
        .filter(|t| {
            matches!(t.response, LeanStatementResponse::Confirmed { .. })
                && t.faithfulness
                    .as_ref()
                    .is_some_and(|f| f.verdict == FaithfulnessVerdict::Faithful)
        })
        .ok_or_else(|| {
            CoreError::conflict("confirm the independently inspected exact target first")
        })
}
fn task<'a>(file: &'a RefereeFile, key: &str) -> CoreResult<&'a ResearchObligation> {
    file.research_obligations
        .iter()
        .find(|t| t.id == key)
        .ok_or_else(|| CoreError::not_found("research obligation", key))
}
impl App {
    pub async fn research_obligations(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<ResearchView> {
        let submission = self.submission(caller, id).await?;
        let (file, _) = self.referee_file(id).await?;
        let now = self.ports.clock.now();
        Ok(ResearchView {
            candidates: candidates(&submission, &file)?,
            obligations: file
                .research_obligations
                .iter()
                .map(|task| {
                    let state = if !subject_current(&submission, &task.candidate.subject) {
                        "stale_source"
                    } else if task.stopped() {
                        "stopped"
                    } else if submission
                        .processing_consent
                        .as_ref()
                        .is_none_or(|c| c.version != AI_PROCESSING_CONSENT_VERSION)
                    {
                        "consent_required"
                    } else if resolved(&submission, &file, task) {
                        "resolved_by_audit"
                    } else if task.attempts().iter().any(Attempt::verified) {
                        "verified_target"
                    } else if task.budget.expires_at <= now {
                        "expired"
                    } else if task
                        .attempts()
                        .iter()
                        .any(|a| a.receipt.is_none() && a.invalidated.is_none())
                    {
                        "verifying"
                    } else if task.attempts().len() >= usize::from(task.budget.max_attempts) {
                        "budget_exhausted"
                    } else if let Some(target) = task.target() {
                        if target
                            .faithfulness
                            .as_ref()
                            .is_none_or(|f| f.verdict != FaithfulnessVerdict::Faithful)
                        {
                            "target_mismatch"
                        } else {
                            match target.response {
                                LeanStatementResponse::AwaitingAuthor => "awaiting_confirmation",
                                LeanStatementResponse::Confirmed { .. } => "ready",
                                LeanStatementResponse::Rejected { .. } => {
                                    if task
                                        .events
                                        .iter()
                                        .filter(|e| {
                                            matches!(e, ResearchEvent::TargetPrepared { .. })
                                        })
                                        .count()
                                        >= 3
                                    {
                                        "target_budget_exhausted"
                                    } else {
                                        "target_rejected"
                                    }
                                }
                            }
                        }
                    } else {
                        "awaiting_target"
                    };
                    ResearchObligationView {
                        target: task.target(),
                        attempts: task.attempts(),
                        obligation: task.clone(),
                        state: state.into(),
                    }
                })
                .collect(),
        })
    }

    pub async fn adopt_research_obligation(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        new: NewResearchObligation,
    ) -> CoreResult<ResearchView> {
        new.goal.validate()?;
        if new.consent != RESEARCH_CONSENT {
            return Err(CoreError::invalid(
                "explicit current research processing authorization is required",
            ));
        }
        let key = digest(&(id, &caller.person, &new))?[7..].to_owned();
        loop {
            let submission = self.submission(caller, id).await?;
            Self::require_submitter(caller, &submission)?;
            if submission
                .processing_consent
                .as_ref()
                .is_none_or(|c| c.version != AI_PROCESSING_CONSENT_VERSION)
            {
                return Err(CoreError::conflict(
                    "current processing consent is required",
                ));
            }
            let (mut file, stored) = self.referee_file(id).await?;
            if file.research_obligations.iter().any(|t| t.id == key) {
                // Queue reconciliation also recovers an uncertain enqueue after admission.
                self.ports
                    .queue
                    .enqueue(id, JobKind::ResearchTarget)
                    .await?;
                return self.research_obligations(caller, id).await;
            }
            let now = self.ports.clock.now();
            new.budget.validate(now)?;
            if file.research_obligations.len() >= 100 {
                return Err(CoreError::conflict("research obligation history is full"));
            }
            let candidate = candidates(&submission, &file)?
                .into_iter()
                .find(|c| {
                    c.subject.round == new.round
                        && c.subject.claim == new.claim
                        && c.gap == new.gap
                        && c.digest == new.candidate_digest
                })
                .ok_or_else(|| {
                    CoreError::conflict("the selected audited obligation changed or is unavailable")
                })?;
            file.research_obligations.push(ResearchObligation {
                id: key.clone(),
                candidate,
                goal: new.goal.clone(),
                budget: new.budget.clone(),
                consent: new.consent.clone(),
                authorized_by: caller.person.clone(),
                authorized_at: now,
                events: vec![],
            });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.ports
            .queue
            .enqueue(id, JobKind::ResearchTarget)
            .await?;
        self.research_obligations(caller, id).await
    }

    pub async fn stop_research_obligation(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        reason: String,
    ) -> CoreResult<ResearchView> {
        require_text("reason for stopping research", &reason, 2_000)?;
        loop {
            let submission = self.submission(caller, id).await?;
            Self::require_submitter(caller, &submission)?;
            let (mut file, stored) = self.referee_file(id).await?;
            let task = file
                .research_obligations
                .iter_mut()
                .find(|t| t.id == key)
                .ok_or_else(|| CoreError::not_found("research obligation", key))?;
            if task.stopped() {
                break;
            }
            task.events.push(ResearchEvent::Stopped {
                by: caller.person.clone(),
                reason: reason.clone(),
                at: self.ports.clock.now(),
            });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.ports
            .queue
            .enqueue(id, JobKind::ResearchTarget)
            .await?;
        self.research_obligations(caller, id).await
    }
}

impl App {
    /// Used by live CMA cancellation as well as new target admission.
    pub async fn research_active(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
    ) -> CoreResult<bool> {
        let submission = self.submission(caller, id).await?;
        let (file, _) = self.referee_file(id).await?;
        Ok(active(
            &submission,
            &file,
            task(&file, key)?,
            self.ports.clock.now(),
        ))
    }

    /// Composition-only, after local elaboration and independent CMA inspection.
    pub async fn record_research_target(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        lease: &dyn JobLease,
        key: &str,
        new: PreparedResearchTarget,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        if job.kind != JobKind::ResearchTarget {
            return Err(CoreError::conflict("wrong research job"));
        }
        require_text("Lean target", &new.lean, 200_000)?;
        require_text("toolchain", &new.toolchain, 2_000)?;
        require_text("target reading", &new.reading, 20_000)?;
        require_text("faithfulness reason", &new.faithfulness.reason, 5_000)?;
        if !new.lean.starts_with("import Mathlib\n")
            || !new.lean.contains("def wishpool_target_prop : Prop :=")
            || new.faithfulness.mismatch.chars().count() > 5_000
            || (new.faithfulness.verdict != FaithfulnessVerdict::Faithful
                && new.faithfulness.mismatch.trim().is_empty())
        {
            return Err(CoreError::invalid("invalid inspected research target"));
        }
        let digest = format!("{:x}", Sha256::digest(new.lean.as_bytes()));
        loop {
            let submission = self.submission(caller, &job.submission).await?;
            let (mut file, stored) = self.referee_file(&job.submission).await?;
            let t = task(&file, key)?;
            if !active(&submission, &file, t, self.ports.clock.now()) {
                return Err(CoreError::conflict(
                    "research authorization expired or changed",
                ));
            }
            let previous = t.target();
            if previous.as_ref().is_some_and(|p| p.digest == digest) {
                return Err(CoreError::conflict("the regenerated target did not change"));
            }
            if previous.as_ref().map(|p| &p.digest) != new.previous_digest.as_ref()
                || previous
                    .as_ref()
                    .is_some_and(|p| !matches!(p.response, LeanStatementResponse::Rejected { .. }))
                || t.events
                    .iter()
                    .filter(|e| matches!(e, ResearchEvent::TargetPrepared { .. }))
                    .count()
                    >= 3
            {
                return Err(CoreError::conflict(
                    "target preparation superseded or budget exhausted",
                ));
            }
            let target = LeanStatementAttempt {
                faithfulness: Some(new.faithfulness.clone()),
                claim: t.candidate.subject.claim.clone(),
                version: t.candidate.subject.version,
                claims_revision: t.candidate.subject.claims_revision,
                lean: new.lean.clone(),
                digest: digest.clone(),
                toolchain: new.toolchain.clone(),
                reading: new.reading.clone(),
                response: LeanStatementResponse::AwaitingAuthor,
                created_at: self.ports.clock.now(),
            };
            file.research_obligations
                .iter_mut()
                .find(|t| t.id == key)
                .unwrap()
                .events
                .push(ResearchEvent::TargetPrepared { target });
            if !lease.renew(job).await? {
                return Err(CoreError::conflict("research job lease lost"));
            }
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => return result,
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn respond_research_target(
        &self,
        caller: &Caller,
        authentication: AuthenticationMethod,
        id: &SubmissionId,
        key: &str,
        digest: &str,
        confirm: bool,
        comment: String,
    ) -> CoreResult<ResearchView> {
        if authentication != AuthenticationMethod::CookieSession {
            return Err(CoreError::forbidden(
                "confirm the research target in a signed-in browser session",
            ));
        }
        if !confirm {
            require_text("target correction", &comment, 10_000)?;
        }
        loop {
            let submission = self.submission(caller, id).await?;
            Self::require_submitter(caller, &submission)?;
            let (mut file, stored) = self.referee_file(id).await?;
            let t = task(&file, key)?;
            if !active(&submission, &file, t, self.ports.clock.now()) {
                return Err(CoreError::conflict(
                    "research authorization expired or changed",
                ));
            }
            let target = t
                .target()
                .ok_or_else(|| CoreError::conflict("no prepared target"))?;
            if target.digest != digest {
                return Err(CoreError::conflict("target digest changed"));
            }
            if matches!(target.response, LeanStatementResponse::Confirmed { .. }) && confirm {
                return self.research_obligations(caller, id).await;
            }
            if !matches!(target.response, LeanStatementResponse::AwaitingAuthor) {
                return Err(CoreError::conflict("target decision already recorded"));
            }
            if confirm
                && target
                    .faithfulness
                    .as_ref()
                    .is_none_or(|f| f.verdict != FaithfulnessVerdict::Faithful)
            {
                return Err(CoreError::conflict(
                    "a faithful independent inspection is required",
                ));
            }
            let response = if confirm {
                LeanStatementResponse::Confirmed {
                    author: caller.person.clone(),
                    at: self.ports.clock.now(),
                }
            } else {
                LeanStatementResponse::Rejected {
                    comment: comment.clone(),
                    at: self.ports.clock.now(),
                }
            };
            file.research_obligations
                .iter_mut()
                .find(|t| t.id == key)
                .unwrap()
                .events
                .push(ResearchEvent::TargetDecided {
                    digest: digest.into(),
                    response,
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        if !confirm {
            self.ports
                .queue
                .enqueue(id, JobKind::ResearchTarget)
                .await?;
        }
        self.research_obligations(caller, id).await
    }

    pub async fn submit_research_attempt(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        input: ResearchAttemptInput,
    ) -> CoreResult<ResearchView> {
        let submission = self.submission(caller, id).await?;
        Self::require_submitter(caller, &submission)?;
        if input.solution.trim().is_empty()
            || input.solution.len() > 1_048_576
            || input.note.chars().count() > 2_000
        {
            return Err(CoreError::invalid(
                "solution must be at most 1 MB and note at most 2,000 characters",
            ));
        }
        validate_lean_modules(&input.solution, &input.modules)?;
        let solution_digest = lean_solution_digest(&input.solution, &input.modules);
        // Authorize before storing large payloads; repeat the checks at admission.
        let (file, _) = self.referee_file(id).await?;
        let t = task(&file, key)?;
        if !active(&submission, &file, t, self.ports.clock.now())
            || confirmed(t)?.digest != input.target_digest
        {
            return Err(CoreError::conflict(
                "research target or authorization changed",
            ));
        }
        let entrant = self.resolve_entrant(caller, None).await?;
        let mut modules = std::collections::BTreeMap::new();
        for (name, source) in &input.modules {
            modules.insert(
                name.clone(),
                self.ports
                    .blobs
                    .put(source.as_bytes(), "text/plain; charset=utf-8")
                    .await?,
            );
        }
        let solution = self
            .ports
            .blobs
            .put(input.solution.as_bytes(), "text/plain; charset=utf-8")
            .await?;
        loop {
            let submission = self.submission(caller, id).await?;
            Self::require_submitter(caller, &submission)?;
            let (mut file, stored) = self.referee_file(id).await?;
            let t = task(&file, key)?;
            if !active(&submission, &file, t, self.ports.clock.now())
                || confirmed(t)?.digest != input.target_digest
            {
                return Err(CoreError::conflict(
                    "research target or authorization changed",
                ));
            }
            let attempts = t.attempts();
            if attempts.iter().any(|a| {
                a.target_digest == input.target_digest && a.solution_digest == solution_digest
            }) {
                self.ports
                    .queue
                    .enqueue(id, JobKind::VerifyResearch)
                    .await?;
                return self.research_obligations(caller, id).await;
            }
            if attempts.len() >= usize::from(t.budget.max_attempts)
                || attempts.iter().any(Attempt::verified)
            {
                return Err(CoreError::conflict(
                    "research attempt budget exhausted or target settled",
                ));
            }
            let attempt = Attempt {
                intermediate: None,
                reuses: vec![],
                id: crate::ids::new_uuid(),
                owner: caller.person.clone(),
                entrant: entrant.clone(),
                target_digest: input.target_digest.clone(),
                solution_digest: solution_digest.clone(),
                solution: solution.clone(),
                modules: modules.clone(),
                note: input.note.clone(),
                attribution: None,
                submitted_at: self.ports.clock.now(),
                receipt: None,
                invalidated: None,
            };
            file.research_obligations
                .iter_mut()
                .find(|t| t.id == key)
                .unwrap()
                .events
                .push(ResearchEvent::AttemptAdmitted {
                    attempt: Box::new(attempt),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.ports
            .queue
            .enqueue(id, JobKind::VerifyResearch)
            .await?;
        self.research_obligations(caller, id).await
    }

    pub async fn research_attempt(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        attempt: &str,
    ) -> CoreResult<ResearchAttemptView> {
        self.submission(caller, id).await?;
        let (file, _) = self.referee_file(id).await?;
        let attempt = task(&file, key)?
            .attempts()
            .into_iter()
            .find(|a| a.id == attempt)
            .ok_or_else(|| CoreError::not_found("research attempt", attempt))?;
        let solution = self.research_solution(&attempt).await?;
        let modules = self.lean_module_sources(&attempt.modules).await?;
        Ok(ResearchAttemptView {
            attempt,
            solution,
            modules,
        })
    }
    async fn research_solution(&self, attempt: &Attempt) -> CoreResult<String> {
        let bytes = self
            .ports
            .blobs
            .get(&attempt.solution.id)
            .await?
            .ok_or_else(|| CoreError::not_found("research solution", &attempt.id))?;
        if format!("{:x}", Sha256::digest(&bytes)) != attempt.solution.sha256 {
            return Err(CoreError::Unavailable(
                "research solution integrity failure".into(),
            ));
        }
        String::from_utf8(bytes).map_err(|_| CoreError::invalid("research solution is not UTF-8"))
    }

    /// Only the credential-free verifier port can supply these immutable receipts.
    pub async fn verify_research_leased(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        lease: &dyn JobLease,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        if job.kind != JobKind::VerifyResearch {
            return Err(CoreError::conflict("wrong verification job"));
        }
        self.submission(caller, &job.submission).await?;
        let (file, _) = self.referee_file(&job.submission).await?;
        for t in &file.research_obligations {
            for attempt in t
                .attempts()
                .iter()
                .filter(|a| a.receipt.is_none() && a.invalidated.is_none())
            {
                self.check_research_attempt(caller, job, lease, &t.id, attempt)
                    .await?;
            }
        }
        Ok(())
    }
    async fn check_research_attempt(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        lease: &dyn JobLease,
        key: &str,
        attempt: &Attempt,
    ) -> CoreResult<()> {
        let mut receipt: Option<VerificationReceipt> = None;
        loop {
            let submission = self.submission(caller, &job.submission).await?;
            let (mut file, stored) = self.referee_file(&job.submission).await?;
            let t = task(&file, key)?;
            let saved = t
                .attempts()
                .into_iter()
                .find(|a| a.id == attempt.id)
                .ok_or_else(|| CoreError::not_found("research attempt", &attempt.id))?;
            if saved.receipt.is_some() || saved.invalidated.is_some() {
                return Ok(());
            }
            let target = confirmed(t).ok();
            let eligible = active(&submission, &file, t, self.ports.clock.now())
                && target
                    .as_ref()
                    .is_some_and(|v| v.digest == attempt.target_digest);
            if !lease.renew(job).await? {
                return Err(CoreError::conflict("research verification lease lost"));
            }
            let event = if !eligible {
                ResearchEvent::AttemptInvalidated {
                    attempt_id: attempt.id.clone(),
                    reason: "The source, target or research authorization changed.".into(),
                }
            } else if let Some(receipt) = &receipt {
                ResearchEvent::AttemptChecked {
                    attempt_id: attempt.id.clone(),
                    receipt: receipt.clone(),
                }
            } else {
                let target = target.unwrap();
                let result = self
                    .ports
                    .verifier
                    .verify(&VerificationRequest {
                        required_declaration: None,
                        inspect_constants: vec![],
                        target: target.lean,
                        target_digest: target.digest,
                        toolchain: target.toolchain.clone(),
                        solution: self.research_solution(attempt).await?,
                        modules: self.lean_module_sources(&attempt.modules).await?,
                    })
                    .await?;
                if result.target_digest != attempt.target_digest
                    || result.solution_digest != attempt.solution_digest
                    || result.toolchain != target.toolchain
                    || result.axioms.iter().any(|a| {
                        !["propext", "Classical.choice", "Quot.sound"].contains(&a.as_str())
                    })
                {
                    return Err(CoreError::Unavailable(
                        "invalid research verifier receipt".into(),
                    ));
                }
                receipt = Some(result);
                continue;
            };
            file.research_obligations
                .iter_mut()
                .find(|t| t.id == key)
                .unwrap()
                .events
                .push(event);
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => return result,
            }
        }
    }

    pub async fn recheck_research(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        attempt: &str,
    ) -> CoreResult<()> {
        let submission = self.submission(caller, id).await?;
        Self::require_submitter(caller, &submission)?;
        let (file, _) = self.referee_file(id).await?;
        let t = task(&file, key)?;
        if !active(&submission, &file, t, self.ports.clock.now())
            || !t.attempts().iter().any(|a| a.id == attempt && a.verified())
        {
            return Err(CoreError::conflict(
                "a checked attempt on the current source is required",
            ));
        }
        self.request_recheck(id, &format!("research:{key}:{attempt}"))
            .await
    }
    pub(crate) async fn reconcile_research(&self) -> CoreResult<()> {
        let mut cursor = None;
        loop {
            let page = self
                .ports
                .submissions
                .list(&SubmissionFilter::default(), 100, cursor)
                .await?;
            for s in page.items {
                let Some(file) = self.ports.referees.get(&s.id).await? else {
                    continue;
                };
                let now = self.ports.clock.now();
                if file.agent_steps.values().any(|step| {
                    step.cma_attempts.iter().any(|attempt| {
                        attempt.step.starts_with("research:")
                            && attempt.progress["agent_id"].is_string()
                            && attempt.progress["deleted"] != true
                    }) || step.cma_attempts.last().is_some_and(|attempt| {
                        attempt.step.starts_with("research:")
                            && step.progress["agent_id"].is_string()
                            && step.progress["deleted"] != true
                    })
                }) {
                    self.ports
                        .queue
                        .enqueue(&s.id, JobKind::ResearchTarget)
                        .await?;
                }
                for t in &file.research_obligations {
                    if active(&s, &file, t, now)
                        && t.target().is_none_or(|target| {
                            matches!(target.response, LeanStatementResponse::Rejected { .. })
                        })
                        && t.events
                            .iter()
                            .filter(|e| matches!(e, ResearchEvent::TargetPrepared { .. }))
                            .count()
                            < 3
                    {
                        self.ports
                            .queue
                            .enqueue(&s.id, JobKind::ResearchTarget)
                            .await?;
                    }
                    if t.attempts()
                        .iter()
                        .any(|a| a.receipt.is_none() && a.invalidated.is_none())
                    {
                        self.ports
                            .queue
                            .enqueue(&s.id, JobKind::VerifyResearch)
                            .await?;
                    }
                }
            }
            cursor = page.next_before;
            if cursor.is_none() {
                break;
            }
        }
        Ok(())
    }
}
