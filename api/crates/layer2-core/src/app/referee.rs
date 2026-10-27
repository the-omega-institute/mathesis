use std::collections::BTreeSet;

use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, SubmissionId},
    model::{
        Caller, Correctness, EscapeAssessment, FeedbackLetter, MAX_LETTER_CHARS, NewLetter,
        Outcome, PriorRelation, PriorWork, ProofShape, RefereeFile, RefereeRound, RefereeView,
        ReviewerIdentity, Role, RoundUpdate, Source, SourceKind, Stage, StagePayload, StageReport,
        Step, StepState, Submission, SubmissionPurpose, SubmissionStatus,
    },
    ports::JobKind,
};

impl App {
    pub(crate) async fn request_recheck(&self, id: &SubmissionId, trigger: &str) -> CoreResult<()> {
        loop {
            let (mut file, stored) = self.referee_file(id).await?;
            if file.recheck_requests.iter().any(|r| r == trigger) {
                if file
                    .current()
                    .is_some_and(|r| r.recheck_requests == file.recheck_requests && r.is_settled())
                {
                    return Ok(());
                }
                break;
            }
            file.recheck_requests.push(trigger.into());
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.ports.queue.enqueue(id, JobKind::Referee).await
    }

    pub async fn recheck_evidence(&self, caller: &Caller, id: &SubmissionId) -> CoreResult<String> {
        caller.require(Role::Reviewer)?;
        let (file, _) = self.referee_file(id).await?;
        let Some(round) = file.current() else {
            return Ok(String::new());
        };
        let mut evidence = vec![];
        for trigger in &round.recheck_requests {
            if let Some(response_id) = trigger.strip_prefix("finding-response:") {
                if let Some(response) = file.finding_responses.iter().find(|r| {
                    r.id == response_id
                        && r.version == round.version
                        && r.claims_revision == round.claims_revision
                }) {
                    let concern = file
                        .rounds
                        .iter()
                        .find(|r| r.number == response.round)
                        .and_then(|r| r.audit.done())
                        .and_then(|a| a.concerns.get(response.concern));
                    evidence.push(
                        serde_json::json!({"author_response":response,"original_finding":concern}),
                    );
                }
            } else if let Some((key, attempt_id)) = trigger
                .strip_prefix("research:")
                .and_then(|s| s.split_once(':'))
            {
                if let Some(task) = file.research_obligations.iter().find(|t| {
                    t.id == key
                        && t.candidate.subject.version == round.version
                        && t.candidate.subject.claims_revision == round.claims_revision
                }) && let Some(attempt) = task
                    .attempts()
                    .into_iter()
                    .find(|a| a.id == attempt_id && a.verified())
                {
                    let target = task.events.iter().find_map(|e| match e {
                        crate::model::ResearchEvent::TargetPrepared { target }
                            if target.digest == attempt.target_digest =>
                        {
                            Some(target)
                        }
                        _ => None,
                    });
                    let checked = self.research_attempt(caller, id, key, attempt_id).await?;
                    evidence.push(serde_json::json!({"kind":"internal_research_evidence", "obligation":task.id,
                        "source_audit":task.candidate, "authorized_goal":task.goal, "target":target,
                        "checked_attempt":checked,
                        "limits":"This is internal evidence for the exact checked target. Independently audit correspondence to the original claim and all remaining premises. It is not external baseline coverage, novelty, publication or contribution credit."}));
                }
            } else if let Some(report_id) = trigger.strip_prefix("error-report:") {
                if let Some(report) = self
                    .all_error_reports()
                    .await?
                    .into_iter()
                    .find(|r| r.id == report_id)
                {
                    evidence.push(serde_json::json!({"subject":report.subject,"text":report.text,"resolution":report.resolution,"receipt":report.receipt,"solution":report.solution}));
                }
            } else if let Some(attempt_id) = trigger.strip_prefix("disproof:") {
                let (solve, attempt) = self.find_attempt(attempt_id).await?;
                let source = self.load(&solve.submission).await?;
                let target = source
                    .lean_statements
                    .iter()
                    .find(|t| t.digest == attempt.target_digest)
                    .map(|t| &t.lean);
                evidence.push(serde_json::json!({"claim":solve.claim,"verified_disproof":attempt.receipt,"solution":attempt.solution,"target":target}));
            }
        }
        serde_json::to_string(&evidence).map_err(|_| CoreError::invalid("recheck evidence"))
    }
    pub async fn agent_steps(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<std::collections::BTreeMap<String, crate::model::AgentStep>> {
        caller.require(Role::Reviewer)?;
        self.submission(caller, id).await?;
        Ok(self.referee_file(id).await?.0.agent_steps)
    }

    pub async fn agent_step(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
    ) -> CoreResult<Option<crate::model::AgentStep>> {
        caller.require(Role::Reviewer)?;
        self.submission(caller, id).await?;
        Ok(self.referee_file(id).await?.0.agent_steps.get(key).cloned())
    }
    pub async fn agent_step_snapshot(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
    ) -> CoreResult<(u64, Option<crate::model::AgentStep>)> {
        caller.require(Role::Reviewer)?;
        self.submission(caller, id).await?;
        let (file, _) = self.referee_file(id).await?;
        Ok((file.revision, file.agent_steps.get(key).cloned()))
    }

    pub async fn persist_agent_step(
        &self,
        caller: &Caller,
        job: &crate::ports::LeasedJob,
        leases: &dyn crate::ports::JobLease,
        key: &str,
        update: crate::model::AgentStepUpdate,
    ) -> CoreResult<u64> {
        use crate::model::{AgentStep, CmaOutcome};
        caller.require(Role::Reviewer)?;
        let submission = self.submission(caller, &job.submission).await?;
        let (mut file, stored) = self.referee_file(&job.submission).await?;
        if file.revision != update.source_revision {
            return Err(CoreError::StaleRevision {
                kind: "referee file",
                id: job.submission.to_string(),
            });
        }
        if key.is_empty()
            || key.len() > 240
            || serde_json::to_vec(&update.progress).map_or(true, |v| v.len() > 4_100_000)
        {
            return Err(CoreError::invalid("agent step exceeds its limit"));
        }
        if !update.cleanup
            && (update.version != Self::version_number(&submission)?
                || update.claims_revision != submission.claims_revision
                || matches!(
                    submission.status,
                    SubmissionStatus::Draft | SubmissionStatus::Withdrawn
                ))
        {
            return Err(CoreError::conflict("agent inputs superseded"));
        }
        if update.cleanup && !file.agent_steps.contains_key(key) {
            return Err(CoreError::conflict("cleanup requires existing CMA work"));
        }
        let step = file.agent_steps.entry(key.into()).or_insert(AgentStep {
            cma_attempts: vec![],
            version: update.version,
            claims_revision: update.claims_revision,
            progress: serde_json::Value::Null,
        });
        if step.version != update.version || step.claims_revision != update.claims_revision {
            return Err(CoreError::conflict("agent step identity cannot change"));
        }
        let current = step.cma_attempts.last().map(|a| a.ordinal);
        if !update.cleanup && current != update.ordinal {
            return Err(CoreError::conflict("CMA selected attempt superseded"));
        }
        let attempt = match update.ordinal {
            Some(ordinal) => Some(
                step.cma_attempts
                    .iter_mut()
                    .find(|a| a.ordinal == ordinal)
                    .ok_or_else(|| CoreError::conflict("CMA attempt not admitted"))?,
            ),
            None if step.cma_attempts.is_empty() => None,
            None => return Err(CoreError::conflict("CMA attempt not selected")),
        };
        let next = update.progress;
        if let Some(attempt) = attempt {
            let progress = if update.ordinal != current {
                &mut attempt.progress
            } else {
                &mut step.progress
            };
            persist_progress(progress, next, update.cleanup)?;
            if !update.cleanup
                && attempt.outcome == CmaOutcome::Running
                && (progress["answer"].is_string() || progress["failure"].is_string())
            {
                attempt.outcome = if progress["answer"].is_string() {
                    CmaOutcome::Succeeded
                } else {
                    CmaOutcome::Failed
                };
                attempt.finished_at = Some(self.ports.clock.now());
                attempt.failure = serde_json::from_value(progress["typed_failure"].clone()).ok();
            }
        } else {
            persist_progress(&mut step.progress, next, update.cleanup)?;
        }
        // Check the live lease immediately before the revision-fenced write.
        if !leases.renew(job).await? {
            return Err(CoreError::conflict("CMA job lease lost"));
        }
        self.store_referee_file(&mut file, stored).await?;
        Ok(file.revision)
    }

    /// Staff see every round; the submitting author and co-authors see the
    /// reports, audits, sent letters and probe outcomes; advice/files stay private.
    pub async fn referee(&self, caller: &Caller, id: &SubmissionId) -> CoreResult<RefereeView> {
        let submission = self.submission(caller, id).await?;
        let file = self
            .ports
            .referees
            .get(id)
            .await?
            .unwrap_or_else(|| RefereeFile::new(submission.id.clone()));
        Ok(file.view(Self::is_staff(caller)))
    }

    /// The round for the paper's current version and statement set, started
    /// if there is none. Reviewer accounts only.
    pub async fn begin_referee_round(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<RefereeFile> {
        caller.require(Role::Reviewer)?;
        let submission = self.submission(caller, id).await?;
        let (mut file, stored) = self.referee_file(id).await?;
        let version = Self::version_number(&submission)?;
        if file.current().is_some_and(|r| {
            r.version == version
                && r.claims_revision == submission.claims_revision
                && (!r.is_settled() || r.recheck_requests == file.recheck_requests)
        }) {
            return Ok(file);
        }
        if !(submission.is_open()
            || matches!(
                submission.status,
                SubmissionStatus::Accepted { .. }
                    | SubmissionStatus::NotAccepted
                    | SubmissionStatus::ReviewComplete
            ) && !file.recheck_requests.is_empty())
        {
            return Err(CoreError::conflict(format!(
                "the paper is {}",
                submission.status.name()
            )));
        }
        self.push_round(&mut file, &submission, version);
        self.store_referee_file(&mut file, stored).await?;
        Ok(file)
    }

    /// Requeue an unfinished round after an operational failure. Persisted
    /// steps and provider identities stay intact; enqueue preserves live leases.
    pub async fn resume_referee(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        number: u32,
    ) -> CoreResult<()> {
        caller.require(Role::Editor)?;
        let submission = self.submission(caller, id).await?;
        if matches!(
            submission.status,
            SubmissionStatus::Draft | SubmissionStatus::Withdrawn
        ) {
            return Err(CoreError::conflict("the paper is not reviewable"));
        }
        let (file, _) = self.referee_file(id).await?;
        let round = file
            .current()
            .filter(|r| r.number == number)
            .ok_or_else(|| CoreError::conflict("the requested round is not current"))?;
        if round.version != Self::version_number(&submission)?
            || round.claims_revision != submission.claims_revision
        {
            return Err(CoreError::conflict("the review inputs have changed"));
        }
        if round.is_settled() {
            return Err(CoreError::conflict("the round is already settled"));
        }
        self.ports.queue.enqueue(id, JobKind::Referee).await
    }

    /// Start a new round on the current statements, e.g. after a failed
    /// step. Editors only.
    pub async fn restart_referee(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<RefereeFile> {
        caller.require(Role::Editor)?;
        let submission = self.submission(caller, id).await?;
        let failed_private_review = submission.purpose == SubmissionPurpose::ReviewOnly
            && submission.status == SubmissionStatus::ReviewComplete;
        if !submission.is_open() && !failed_private_review {
            return Err(CoreError::conflict(format!(
                "the paper is {}",
                submission.status.name()
            )));
        }
        let version = Self::version_number(&submission)?;
        let (mut file, stored) = self.referee_file(id).await?;
        if file.current().is_some_and(|r| !r.is_settled()) {
            return Err(CoreError::conflict("the current round is still running"));
        }
        if failed_private_review {
            let previous = file
                .current()
                .ok_or_else(|| CoreError::conflict("no private review to retry"))?;
            let failed = matches!(previous.referee.state, StepState::Failed { .. })
                || matches!(previous.audit.state, StepState::Failed { .. })
                || matches!(previous.advice.state, StepState::Failed { .. })
                || matches!(previous.letter.state, StepState::Failed { .. });
            if !failed
                || previous.version != version
                || previous.claims_revision != submission.claims_revision
            {
                return Err(CoreError::conflict(
                    "no failed current private review to retry",
                ));
            }
            // Use the existing re-review path for completed private submissions.
            // Persist its intent and the fresh round atomically; previous
            // readings, letters, source and publication state remain intact.
            let trigger = format!("operational-retry:r{}", previous.number);
            if !file.recheck_requests.contains(&trigger) {
                file.recheck_requests.push(trigger);
            }
        }
        self.push_round(&mut file, &submission, version);
        self.store_referee_file(&mut file, stored).await?;
        self.ports.queue.enqueue(id, JobKind::Referee).await?;
        Ok(file)
    }

    /// Record one step of round `number`. Reviewer accounts only. Results
    /// may name only statements of the round's statement set.
    pub async fn update_referee_round(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        number: u32,
        update: RoundUpdate,
    ) -> CoreResult<RefereeFile> {
        caller.require(Role::Reviewer)?;
        let submission = self.submission(caller, id).await?;
        let (mut file, stored) = self.referee_file(id).await?;
        let responses = file.finding_responses.clone();
        let knowledge_receipts = file
            .knowledge_events
            .iter()
            .filter_map(|event| match event {
                crate::model::KnowledgeEvent::QueryCompleted { receipt } => Some(receipt.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let round = file
            .rounds
            .iter_mut()
            .find(|r| r.number == number)
            .ok_or_else(|| CoreError::not_found("referee round", number.to_string()))?;
        if round.version != Self::version_number(&submission)?
            || round.claims_revision != submission.claims_revision
        {
            return Err(CoreError::conflict(
                "the paper version or statements changed since the round started",
            ));
        }
        let knowledge_pending = round.knowledge_contract > 0
            && submission.status == SubmissionStatus::InReview
            && [Stage::Literature, Stage::Escape].iter().all(|stage| {
                submission.latest_report(*stage).is_some_and(|report| {
                    report.evidence.iter().any(|e| {
                        e.kind == "referee_audit"
                            && e.locator
                                == format!(
                                    "referee-audit:{id}:r{number}:v{}:c{}",
                                    round.version, round.claims_revision
                                )
                    })
                })
            })
            && matches!(
                &submission.decision,
                Some(crate::policy::Decision::Pending {
                    awaiting: Stage::Literature,
                    ..
                })
            );
        let known: BTreeSet<&ClaimId> = submission.claims.iter().map(|c| &c.id).collect();
        let check = |claims: Vec<&ClaimId>| -> CoreResult<()> {
            match claims.into_iter().find(|c| !known.contains(c)) {
                Some(unknown) => Err(CoreError::not_found("statement", unknown.as_str())),
                None => Ok(()),
            }
        };
        match update {
            RoundUpdate::Referee(step) => {
                ensure_mutable(&round.referee)?;
                if let Some(report) = step.done() {
                    check(
                        report
                            .claims
                            .iter()
                            .map(|c| &c.claim)
                            .chain(report.concerns.iter().filter_map(|c| c.claim.as_ref()))
                            .collect(),
                    )?;
                }
                round.referee = step;
            }
            RoundUpdate::Audit(step) => {
                ensure_mutable(&round.audit)?;
                if !matches!(step.state, StepState::Pending) && !round.referee.is_settled() {
                    return Err(CoreError::conflict("audit must wait for the referee"));
                }
                if let Some(audit) = step.done() {
                    if round.referee.done().is_none() {
                        return Err(CoreError::conflict(
                            "audit requires a completed referee report",
                        ));
                    }
                    check(audit.claims.iter().map(|c| &c.claim).collect())?;
                    let ids: BTreeSet<_> = audit.claims.iter().map(|c| &c.claim).collect();
                    if ids.len() != submission.claims.len() || ids.len() != audit.claims.len() {
                        return Err(CoreError::invalid(
                            "audit must cover every confirmed statement exactly once",
                        ));
                    }
                    if round.review_contract >= crate::model::REVIEW_CONTRACT {
                        for reading in &audit.claims {
                            if let Some(coverage) = &reading.coverage {
                                if submission.involves(&caller.person) {
                                    return Err(CoreError::forbidden(
                                        "authors cannot file their own knowledge review",
                                    ));
                                }
                                coverage.validate(&knowledge_receipts)?;
                                if coverage.subject.submission != *id
                                    || coverage.subject.round != number
                                    || coverage.subject.version != round.version
                                    || coverage.subject.claims_revision != round.claims_revision
                                    || coverage.subject.claim != reading.claim
                                {
                                    return Err(CoreError::invalid(
                                        "coverage audit belongs to another review input",
                                    ));
                                }
                                if let crate::model::CoverageQualification::SourceAudited {
                                    auditor,
                                    audit_id,
                                } = &coverage.qualification
                                    && (auditor != &caller.person
                                        || audit_id
                                            != &format!(
                                                "referee-audit:{id}:r{number}:v{}:c{}",
                                                round.version, round.claims_revision
                                            ))
                                {
                                    return Err(CoreError::invalid(
                                        "coverage source audit identity mismatch",
                                    ));
                                }
                            }
                            let claim = submission.claim(&reading.claim).expect("validated claim");
                            let method = reading.methodology.as_ref().ok_or_else(|| CoreError::invalid("this review contract requires a methodology assessment for every statement"))?;
                            method.validate(claim.kind.is_open())?;
                            if !claim.kind.is_open()
                                && reading.correctness == Correctness::Correct
                                && (reading.shape != Some(method.escape)
                                    || reading.witnesses
                                        != method
                                            .witnesses
                                            .iter()
                                            .map(|w| w.statement.clone())
                                            .collect::<Vec<_>>())
                            {
                                return Err(CoreError::invalid(
                                    "proof reading must agree with its structured methodology",
                                ));
                            }
                        }
                    }
                    let expected: BTreeSet<_> = responses
                        .iter()
                        .filter(|r| {
                            r.version == round.version
                                && r.claims_revision == round.claims_revision
                                && round
                                    .recheck_requests
                                    .contains(&format!("finding-response:{}", r.id))
                        })
                        .map(|r| r.id.as_str())
                        .collect();
                    let mut assessed = BTreeSet::new();
                    for result in &audit.response_assessments {
                        if !expected.contains(result.response_id.as_str())
                            || !assessed.insert(result.response_id.as_str())
                        {
                            return Err(CoreError::invalid(
                                "unknown or duplicate review response assessment",
                            ));
                        }
                        crate::model::require_text(
                            "response assessment explanation",
                            &result.explanation,
                            8000,
                        )?;
                        let mut paths = BTreeSet::new();
                        for evidence in &result.evidence {
                            if !paths.insert(&evidence.path)
                                || !submission.current_version().is_some_and(|v| {
                                    v.evidence.iter().any(|e| {
                                        e.descriptor.path == evidence.path
                                            && e.sha256 == evidence.sha256
                                    })
                                })
                            {
                                return Err(CoreError::invalid(
                                    "review evidence must match this source version",
                                ));
                            }
                        }
                    }
                    if round.review_contract >= 1 && assessed != expected {
                        return Err(CoreError::invalid(
                            "audit must assess every admitted author response",
                        ));
                    }
                }
                if let Some(audit) = step.done() {
                    for coverage in audit.claims.iter().filter_map(|r| r.coverage.as_ref()) {
                        file.knowledge_events.push(
                            crate::model::KnowledgeEvent::CoverageAssessed {
                                assessment: coverage.clone(),
                                at: self.ports.clock.now(),
                            },
                        );
                    }
                }
                round.audit = step;
            }
            RoundUpdate::Advice(step) => {
                ensure_mutable(&round.advice)?;
                if !matches!(step.state, StepState::Pending) {
                    if !round.audit.is_settled() {
                        return Err(CoreError::conflict("advice must wait for the audit"));
                    }
                    if !matches!(step.state, StepState::Skipped { .. })
                        && (round.audit.done().is_none()
                            || !(knowledge_pending
                                || matches!(
                                    submission.status,
                                    SubmissionStatus::Accepted { .. }
                                        | SubmissionStatus::NotAccepted
                                        | SubmissionStatus::ReviewComplete
                                )))
                    {
                        return Err(CoreError::conflict(
                            "advice must wait for the audited decision",
                        ));
                    }
                }
                if let Some(advice) = step.done() {
                    check(
                        advice
                            .improvements
                            .iter()
                            .filter_map(|i| i.claim.as_ref())
                            .chain(advice.formalization.iter().map(|f| &f.claim))
                            .collect(),
                    )?;
                }
                round.advice = step;
            }
            RoundUpdate::Formal(step) => {
                ensure_mutable(&round.formal)?;
                if !matches!(step.state, StepState::Pending | StepState::Skipped { .. }) {
                    if !matches!(submission.status, SubmissionStatus::Accepted { .. })
                        || round.letter.done().is_none()
                    {
                        return Err(CoreError::conflict(
                            "formalization requires acceptance and a delivered letter",
                        ));
                    }
                    let candidates: BTreeSet<&ClaimId> = round
                        .advice
                        .done()
                        .map(|a| a.formalization.iter().map(|f| &f.claim).collect())
                        .unwrap_or_default();
                    if candidates.is_empty() {
                        return Err(CoreError::conflict(
                            "formalization needs advice with formalization candidates; skip it otherwise",
                        ));
                    }
                    if let Some(other) = step.done().and_then(|probe| {
                        probe
                            .attempts
                            .iter()
                            .find(|a| !candidates.contains(&a.claim))
                    }) {
                        return Err(CoreError::conflict(format!(
                            "{} was not proposed for formalization",
                            other.claim
                        )));
                    }
                }
                if !matches!(step.state, StepState::Pending)
                    && (!round.advice.is_settled() || !round.letter.is_settled())
                {
                    return Err(CoreError::conflict(
                        "formalization must wait for the advice and letter",
                    ));
                }
                round.formal = step;
            }
            RoundUpdate::Letter(mut step) => {
                ensure_mutable(&round.letter)?;
                if !matches!(step.state, StepState::Pending)
                    && (!round.referee.is_settled()
                        || !round.audit.is_settled()
                        || !round.advice.is_settled())
                {
                    return Err(CoreError::conflict(
                        "the letter must wait for the referee, audit and advice",
                    ));
                }
                if let Some(letter) = step.done() {
                    if round.audit.done().is_none()
                        || !(knowledge_pending
                            || matches!(
                                submission.status,
                                SubmissionStatus::Accepted { .. }
                                    | SubmissionStatus::NotAccepted
                                    | SubmissionStatus::ReviewComplete
                            ))
                    {
                        return Err(CoreError::conflict(
                            "the letter requires the audited decision",
                        ));
                    }
                    if round.referee.done().is_none() {
                        return Err(CoreError::conflict(
                            "a letter draft requires a completed referee report",
                        ));
                    }
                    validate_letter(&letter.subject, &letter.body, &letter.note)?;
                }
                if let StepState::Done { result: draft, .. } = &mut step.state {
                    let first = match &submission.status {
                        SubmissionStatus::Accepted { record } => {
                            if submission.kind == crate::model::SubmissionKind::Conjecture {
                                format!("Your conjecture is displayed as {record}.")
                            } else {
                                format!("Your paper is accepted as {record}.")
                            }
                        }
                        SubmissionStatus::ReviewComplete => "Your private Orchestra review is complete. This is research feedback, not a publication decision.".into(),
                        SubmissionStatus::NotAccepted => {
                            let reasons = match &submission.decision {
                                Some(crate::policy::Decision::NotAccepted { reasons }) => reasons
                                    .iter()
                                    .map(|r| r.author_text())
                                    .collect::<Vec<_>>()
                                    .join(" "),
                                _ => String::new(),
                            };
                            if submission.kind == crate::model::SubmissionKind::Conjecture {
                                format!("Your conjecture is not displayed. {reasons}")
                            } else {
                                format!("Your paper is not accepted. {reasons}")
                            }
                        }
                        SubmissionStatus::InReview if knowledge_pending => {
                            let detail = match &submission.decision {
                                Some(crate::policy::Decision::Pending { detail, .. }) => detail.as_str(),
                                _ => unreachable!("knowledge pending is derived from the decision"),
                            };
                            format!("Your publication decision is pending further knowledge evidence. {detail}")
                        }
                        _ => return Err(CoreError::conflict("the decision has not been applied")),
                    };
                    draft.body = with_decision(&draft.body, &first);
                    validate_letter(&draft.subject, &draft.body, &draft.note)?;
                }
                if let Some(draft) = step.done() {
                    file.letters.push(FeedbackLetter {
                        round: Some(number),
                        assessment: round.audit.done().map(|a| a.verdict),
                        subject: draft.subject.clone(),
                        body: draft.body.clone(),
                        note: draft.note.clone(),
                        edited: false,
                        sent_by: caller.person.clone(),
                        sent_at: self.ports.clock.now(),
                    });
                }
                round.letter = step;
            }
        }
        self.store_referee_file(&mut file, stored).await?;
        Ok(file)
    }

    /// Replay-safe settlement of a persisted audit. Both reports are appended
    /// together under the submission revision fence; record insertion can be recovered.
    pub async fn apply_referee_audit(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        number: u32,
    ) -> CoreResult<Submission> {
        caller.require(Role::Reviewer)?;
        let mut submission = self.submission(caller, id).await?;
        if submission.involves(&caller.person) {
            return Err(CoreError::forbidden("authors cannot audit their own paper"));
        }
        let (file, _) = self.referee_file(id).await?;
        let round = file
            .current()
            .filter(|r| r.number == number)
            .ok_or_else(|| CoreError::conflict("only the current audit can decide"))?;
        if round.version != Self::version_number(&submission)?
            || round.claims_revision != submission.claims_revision
        {
            return Err(CoreError::conflict("audit inputs changed"));
        }
        let audit = round
            .audit
            .done()
            .ok_or_else(|| CoreError::conflict("audit is not complete"))?;
        if matches!(
            submission.status,
            SubmissionStatus::Accepted { .. }
                | SubmissionStatus::NotAccepted
                | SubmissionStatus::ReviewComplete
        ) && round.recheck_requests.is_empty()
        {
            return Ok(submission);
        }
        if !submission.is_open() && round.recheck_requests.is_empty() {
            return Err(CoreError::conflict("paper is not in review"));
        }
        let marker = format!(
            "referee-audit:{}:r{number}:v{}:c{}",
            id, round.version, round.claims_revision
        );
        let filed = [Stage::Literature, Stage::Escape].iter().all(|stage| {
            submission.latest_report(*stage).is_some_and(|r| {
                r.evidence
                    .iter()
                    .any(|e| e.kind == "referee_audit" && e.locator == marker)
            })
        });
        if !filed {
            let prior = audit
                .claims
                .iter()
                .filter_map(|c| {
                    c.known.as_ref().map(|known| PriorWork {
                        claim: c.claim.clone(),
                        source: Source {
                            kind: SourceKind::NamedWork,
                            locator: known.clone(),
                            year: None,
                        },
                        relation: PriorRelation::Same,
                        note: c.comment.clone(),
                    })
                })
                .collect();
            let assessments = submission
                .claims
                .iter()
                .filter(|c| {
                    c.is_main_result()
                        || (submission.kind == crate::model::SubmissionKind::Conjecture
                            && c.role == crate::model::ClaimRole::Main
                            && c.kind.is_open())
                })
                .map(|c| {
                    let reading = audit
                        .claims
                        .iter()
                        .find(|a| a.claim == c.id)
                        .expect("validated complete audit");
                    let carries = reading.correctness == Correctness::Correct
                        && reading.shape == Some(ProofShape::Content)
                        && !reading.witnesses.is_empty();
                    let mut methodology = reading.methodology.clone();
                    if round.knowledge_contract > 0 {
                        let method = methodology.get_or_insert_with(|| {
                            crate::model::MethodologyReading::insufficient(
                                "No usable knowledge comparison was returned.",
                            )
                        });
                        method.knowledge =
                            Some(super::knowledge::knowledge_admission(&file, round, reading));
                    }
                    EscapeAssessment {
                        methodology,
                        conjecture: reading.conjecture.clone(),
                        correctness: Some(reading.correctness),
                        claim: c.id.clone(),
                        shape: if let Some(method) = &reading.methodology {
                            if c.kind.is_open() || reading.correctness == Correctness::Correct {
                                method.escape
                            } else {
                                ProofShape::Undetermined
                            }
                        } else if let Some(r) = &reading.conjecture {
                            r.escape
                        } else if carries {
                            ProofShape::Content
                        } else {
                            ProofShape::BindOnly
                        },
                        witnesses: if carries {
                            reading.witnesses.clone()
                        } else {
                            vec![]
                        },
                        rationale: format!("{:?}: {}", reading.correctness, reading.comment),
                        escape_rate: None,
                    }
                })
                .collect();
            let mut searched: Vec<String> = if round.review_contract
                >= crate::model::REVIEW_CONTRACT
            {
                audit
                    .claims
                    .iter()
                    .flat_map(|reading| {
                        reading.methodology.iter().flat_map(|method| {
                            method
                                .searched
                                .iter()
                                .map(|scope| format!("{}: {scope}", reading.claim))
                        })
                    })
                    .collect()
            } else {
                vec!["Networked referee and Codex audit; opened sources are model-reported, not venue-verified.".into()]
            };
            if searched.is_empty() {
                // Preserve an incomplete audit's limitations so it can reach
                // policy and private feedback without inventing a search.
                searched.push("No usable source-search scope was returned. Literature completeness is not established.".into());
            }
            for (stage, payload) in [
                (
                    Stage::Literature,
                    StagePayload::Literature { prior, searched },
                ),
                (Stage::Escape, StagePayload::Escape { assessments }),
            ] {
                let draft = crate::model::ReportDraft {
                    outcome: Outcome::Pass,
                    summary: audit.summary.clone(),
                    payload,
                    evidence: vec![],
                };
                draft.validate(stage, &submission.claims)?;
                submission.push_report(StageReport {
                    stage,
                    outcome: draft.outcome,
                    summary: draft.summary,
                    payload: draft.payload,
                    evidence: vec![crate::model::Evidence {
                        kind: "referee_audit".into(),
                        locator: marker.clone(),
                        note: "Source-based audit; not a Lean verification.".into(),
                    }],
                    reviewer: ReviewerIdentity::Machine {
                        account: caller.person.clone(),
                        engine: round
                            .audit
                            .engine
                            .clone()
                            .unwrap_or_else(|| "codex-cli".into()),
                        model: round.audit.model.clone(),
                    },
                    claims_revision: round.claims_revision,
                    filed_at: self.ports.clock.now(),
                });
            }
            self.save(&mut submission).await?;
        }
        if !round.recheck_requests.is_empty()
            && matches!(submission.status, SubmissionStatus::Accepted { .. })
        {
            // Publication inputs remain immutable; the new audit and assessment belong to this round.
            return Ok(submission);
        }
        if submission.purpose == SubmissionPurpose::ReviewOnly {
            submission.status = SubmissionStatus::ReviewComplete;
            submission.decision = None;
            self.save(&mut submission).await?;
            return Ok(submission);
        }
        if round.knowledge_contract > 0 {
            let endorsements = self.ports.endorsements.for_submission(id).await?;
            let decision = self.policy.decide(&submission, &endorsements);
            if matches!(
                &decision,
                crate::policy::Decision::Pending {
                    awaiting: Stage::Literature,
                    ..
                }
            ) && round.audit.done().is_some_and(|audit| {
                audit.claims.iter().any(|reading| {
                    !super::knowledge::knowledge_admission(&file, round, reading).sufficient()
                })
            }) {
                // Preserve the pending decision so advice and a private letter can
                // explain the missing evidence without publishing or rejecting.
                if submission.decision.as_ref() != Some(&decision) {
                    submission.decision = Some(decision);
                    self.save(&mut submission).await?;
                }
                return Ok(submission);
            }
        }
        self.apply_decision(caller, submission).await
    }

    /// Send feedback to the author. Editors only; the body is what the
    /// author reads, usually the edited draft of the current round.
    pub async fn send_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        new: NewLetter,
    ) -> CoreResult<FeedbackLetter> {
        caller.require(Role::Editor)?;
        let submission = self.submission(caller, id).await?;
        if submission.status == SubmissionStatus::Withdrawn {
            return Err(CoreError::conflict("the paper is withdrawn"));
        }
        validate_letter(&new.subject, &new.body, &new.note)?;
        let (mut file, stored) = self.referee_file(id).await?;
        let letter = FeedbackLetter {
            round: file.current().map(|r| r.number),
            assessment: new.assessment,
            edited: file
                .current()
                .and_then(|r| r.letter.done())
                .is_none_or(|draft| {
                    draft.subject != new.subject || draft.body != new.body || draft.note != new.note
                }),
            subject: new.subject,
            body: new.body,
            note: new.note,
            sent_by: caller.person.clone(),
            sent_at: self.ports.clock.now(),
        };
        file.letters.push(letter.clone());
        self.store_referee_file(&mut file, stored).await?;
        Ok(letter)
    }

    fn version_number(submission: &Submission) -> CoreResult<u32> {
        submission
            .current_version()
            .map(|v| v.number)
            .ok_or_else(|| CoreError::conflict("the paper has no version"))
    }

    fn push_round(&self, file: &mut RefereeFile, submission: &Submission, version: u32) {
        let number = file.rounds.last().map_or(1, |r| r.number + 1);
        file.rounds.push(RefereeRound {
            computation_contract: 1,
            computation_receipts: vec![],
            knowledge_contract: 4,
            review_contract: crate::model::REVIEW_CONTRACT,
            source_sha256: submission
                .current_version()
                .map(|v| v.archive.sha256.clone())
                .unwrap_or_default(),
            recheck_requests: file.recheck_requests.clone(),
            number,
            version,
            claims_revision: submission.claims_revision,
            started_at: self.ports.clock.now(),
            referee: Step::pending(),
            audit: Step::pending(),
            advice: Step::pending(),
            formal: Step::pending(),
            letter: Step::pending(),
        });
    }

    /// The stored file (and whether it exists), or a new one.
    pub(crate) async fn referee_file(&self, id: &SubmissionId) -> CoreResult<(RefereeFile, bool)> {
        Ok(match self.ports.referees.get(id).await? {
            Some(file) => (file, true),
            None => (RefereeFile::new(id.clone()), false),
        })
    }

    pub(crate) async fn store_referee_file(
        &self,
        file: &mut RefereeFile,
        stored: bool,
    ) -> CoreResult<()> {
        if stored {
            let expected = file.revision;
            self.ports.referees.replace(file, expected).await?;
            file.revision = expected + 1;
        } else {
            self.ports.referees.insert(file).await?;
        }
        Ok(())
    }
}

fn ensure_mutable<T>(step: &Step<T>) -> CoreResult<()> {
    if step.is_settled() {
        return Err(CoreError::conflict(
            "a settled step is immutable; restart the referee round",
        ));
    }
    Ok(())
}

/// Puts the decision sentence after the salutation ("Dear ...,"), or first when there is none.
fn with_decision(body: &str, decision: &str) -> String {
    let body = body.trim_start();
    match body.split_once('\n') {
        Some((salutation, rest)) if salutation.starts_with("Dear ") => {
            format!("{salutation}\n\n{decision}\n\n{}", rest.trim_start())
        }
        _ => format!("{decision}\n\n{body}"),
    }
}

fn validate_letter(subject: &str, body: &str, note: &str) -> CoreResult<()> {
    if body.trim().is_empty() {
        return Err(CoreError::invalid("the letter is empty"));
    }
    if subject.chars().count() > 300 {
        return Err(CoreError::invalid("the subject is too long"));
    }
    if body.chars().count() > MAX_LETTER_CHARS || note.chars().count() > MAX_LETTER_CHARS {
        return Err(CoreError::invalid("the letter is too long"));
    }
    Ok(())
}

fn persist_progress(
    old: &mut serde_json::Value,
    next: serde_json::Value,
    cleanup: bool,
) -> CoreResult<()> {
    const CLEANUP_FIELDS: [&str; 5] = [
        "stopped",
        "deleted",
        "cancel_intent",
        "stop_intent",
        "agent_id",
    ];
    let terminal = old["answer"].is_string() || old["failure"].is_string();
    if terminal && !cleanup {
        let mut comparable = next.clone();
        let mut original = old.clone();
        // RunState is owned by Layer 3; normalize its wire defaults here without
        // a reverse layer dependency. New optional fields must not break replay.
        for value in [&mut comparable, &mut original] {
            for field in [
                "create_intent",
                "agent_id",
                "response_id",
                "answer",
                "failure",
                "typed_failure",
                "cancel_intent",
            ] {
                value
                    .as_object_mut()
                    .unwrap()
                    .entry(field)
                    .or_insert(serde_json::Value::Null);
            }
            for field in ["next_chunk", "deadline"] {
                value
                    .as_object_mut()
                    .unwrap()
                    .entry(field)
                    .or_insert(serde_json::json!(0));
            }
            value
                .as_object_mut()
                .unwrap()
                .entry("input")
                .or_insert(serde_json::json!(""));
            for field in ["stopped", "deleted"] {
                value
                    .as_object_mut()
                    .unwrap()
                    .entry(field)
                    .or_insert(serde_json::json!(false));
            }
        }
        for field in CLEANUP_FIELDS {
            comparable[field] = old[field].clone();
            original[field] = old[field].clone();
        }
        if original != comparable {
            return Err(CoreError::conflict("terminal CMA result is immutable"));
        }
    }
    if cleanup || terminal {
        // Cleanup may add receipts and retain its exact control intent, never replace results.
        for field in ["agent_id", "cancel_intent", "stop_intent"] {
            if !next[field].is_null() {
                if !old[field].is_null() && old[field] != next[field] {
                    return Err(CoreError::conflict("CMA cleanup intent is immutable"));
                }
                old[field] = next[field].clone();
            }
        }
        for field in ["stopped", "deleted"] {
            if next[field] == true {
                old[field] = serde_json::Value::Bool(true);
            }
        }
        return Ok(());
    }
    *old = next;
    Ok(())
}

#[cfg(test)]
mod letter_tests {
    use super::with_decision;

    #[test]
    fn decision_follows_the_salutation() {
        assert_eq!(
            with_decision(
                "Dear A. Author,\n\nThank you.",
                "Your paper is accepted as WP-2026-0001."
            ),
            "Dear A. Author,\n\nYour paper is accepted as WP-2026-0001.\n\nThank you."
        );
        assert_eq!(
            with_decision("Thank you.", "Your paper is not accepted."),
            "Your paper is not accepted.\n\nThank you."
        );
    }
}
