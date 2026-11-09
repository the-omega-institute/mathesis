use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{RecordId, SubmissionId, new_uuid},
    model::*,
    ports::JobKind,
};
use chrono::Duration;

const REPORTS_PER_DAY: usize = 5;
impl App {
    pub async fn file_error_report(
        &self,
        caller: &Caller,
        record: &RecordId,
        new: NewErrorReport,
    ) -> CoreResult<ErrorReportView> {
        let source = self.submission_proof_source(record).await?;
        if new.text.chars().count() > 20_000
            || (new.text.trim().is_empty() && new.solution.is_none())
        {
            return Err(CoreError::invalid(
                "provide evidence text (at most 20,000 characters) or a Lean disproof",
            ));
        }
        match &new.subject {
            ErrorSubject::Statement { claim } => {
                if source.claim(claim).is_none() {
                    return Err(CoreError::not_found("statement", claim.as_str()));
                }
            }
            ErrorSubject::Target { claim, digest } => {
                if self.target(record, claim).await?.digest != *digest {
                    return Err(CoreError::conflict("target digest changed"));
                }
                if new.text.trim().is_empty() {
                    return Err(CoreError::invalid("explain the target mismatch"));
                }
            }
            ErrorSubject::Review => {
                require_text("review error", &new.text, 20_000)?;
            }
        }
        if let Some(solution) = &new.solution {
            if solution.trim().is_empty() || solution.len() > 1_048_576 {
                return Err(CoreError::invalid("Lean evidence must be at most 1 MB"));
            }
            let ErrorSubject::Statement { claim } = &new.subject else {
                return Err(CoreError::invalid(
                    "Lean disproof evidence must name a statement",
                ));
            };
            self.target(record, claim).await?;
        }
        let entrant = self.resolve_entrant(caller, new.as_agent).await?;
        let report = ErrorReport {
            id: new_uuid(),
            record: record.clone(),
            submission: source.id.clone(),
            version: source.current_version().unwrap().number,
            claims_revision: source.claims_revision,
            reporter: entrant.clone(),
            owner: caller.person.clone(),
            subject: new.subject,
            text: new.text,
            solution: new.solution,
            attempt: None,
            receipt: None,
            resolution: None,
            effects_applied: false,
            created_at: self.ports.clock.now(),
        };
        loop {
            let mut file = match self
                .ports
                .solving
                .report_files()
                .await?
                .into_iter()
                .find(|f| f.id == entrant.id)
            {
                Some(f) => f,
                None => {
                    let f = ErrorReportFile {
                        id: entrant.id.clone(),
                        revision: 0,
                        reports: vec![],
                    };
                    match self.ports.solving.insert_report_file(&f).await {
                        Ok(()) | Err(CoreError::Conflict(_)) => continue,
                        Err(e) => return Err(e),
                    }
                }
            };
            if file
                .reports
                .iter()
                .filter(|r| r.created_at > self.ports.clock.now() - Duration::days(1))
                .count()
                >= REPORTS_PER_DAY
            {
                return Err(CoreError::conflict(
                    "daily error report limit reached for this entrant",
                ));
            }
            file.reports.push(report.clone());
            let expected = file.revision;
            file.revision += 1;
            match self
                .ports
                .solving
                .replace_report_file(&file, expected)
                .await
            {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => result?,
            }
            break;
        }
        self.ports
            .queue
            .enqueue(&source.id, JobKind::ErrorReports)
            .await?;
        self.error_report(Some(caller), &report.id).await
    }
    pub(crate) async fn all_error_reports(&self) -> CoreResult<Vec<ErrorReport>> {
        Ok(self
            .ports
            .solving
            .report_files()
            .await?
            .into_iter()
            .flat_map(|f| f.reports)
            .collect())
    }
    async fn stored_report(&self, id: &str) -> CoreResult<ErrorReport> {
        self.all_error_reports()
            .await?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| CoreError::not_found("error report", id))
    }
    async fn change_report(
        &self,
        id: &str,
        change: impl Fn(&mut ErrorReport) -> CoreResult<()>,
    ) -> CoreResult<()> {
        loop {
            let mut file = self
                .ports
                .solving
                .report_files()
                .await?
                .into_iter()
                .find(|f| f.reports.iter().any(|r| r.id == id))
                .ok_or_else(|| CoreError::not_found("error report", id))?;
            change(file.reports.iter_mut().find(|r| r.id == id).unwrap())?;
            let expected = file.revision;
            file.revision += 1;
            match self
                .ports
                .solving
                .replace_report_file(&file, expected)
                .await
            {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => return result,
            }
        }
    }
    pub async fn error_report(
        &self,
        caller: Option<&Caller>,
        id: &str,
    ) -> CoreResult<ErrorReportView> {
        let r = self.stored_report(id).await?;
        let s = self.load(&r.submission).await?;
        let private = caller
            .is_some_and(|c| c.person == r.owner || s.involves(&c.person) || Self::is_staff(c));
        let public = r
            .resolution
            .as_ref()
            .is_some_and(|v| v.verdict == ErrorVerdict::Confirmed)
            && s.analysis_visibility == Visibility::Public
            && self.ports.records.get(&r.record).await?.is_some();
        if !private && !public {
            return Err(CoreError::not_found("error report", id));
        }
        Ok(ErrorReportView {
            id: r.id,
            record: r.record,
            reporter: self.current_entrant(&r.reporter).await?,
            subject: r.subject,
            text: r.text,
            resolution: r.resolution,
            created_at: r.created_at,
            solution: private.then_some(r.solution).flatten(),
            receipt: r.receipt,
        })
    }
    pub async fn error_reports(
        &self,
        caller: Option<&Caller>,
        record: Option<&RecordId>,
    ) -> CoreResult<Vec<ErrorReportView>> {
        let mut out = vec![];
        for r in self.all_error_reports().await? {
            if record.is_some_and(|id| *id != r.record) {
                continue;
            }
            match self.error_report(caller, &r.id).await {
                Ok(v) => out.push(v),
                Err(CoreError::NotFound { .. }) => {}
                Err(e) => return Err(e),
            }
        }
        out.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        Ok(out)
    }
    pub async fn pending_error_reports(
        &self,
        caller: &Caller,
        submission: &SubmissionId,
    ) -> CoreResult<Vec<ErrorReport>> {
        caller.require(Role::Reviewer)?;
        Ok(self
            .all_error_reports()
            .await?
            .into_iter()
            .filter(|r| {
                &r.submission == submission && (r.resolution.is_none() || !r.effects_applied)
            })
            .collect())
    }
    /// Composition-only: no route accepts audit verdicts or verification receipts.
    pub async fn resolve_error_report(
        &self,
        caller: &Caller,
        id: &str,
        verdict: ErrorVerdict,
        reason: String,
        evidence: String,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        require_text("audit reason", &reason, 5000)?;
        if evidence.chars().count() > 5000
            || (verdict != ErrorVerdict::Unclear && evidence.trim().is_empty())
        {
            return Err(CoreError::invalid(
                "confirmed/refuted reports require bounded evidence",
            ));
        }
        let r = self.stored_report(id).await?;
        let s = self.load(&r.submission).await?;
        if r.resolution.is_none()
            && (s.current_version().map(|v| v.number) != Some(r.version)
                || s.claims_revision != r.claims_revision)
        {
            return Err(CoreError::conflict("report inputs changed"));
        }
        let result = ErrorResolution {
            verdict,
            reason,
            evidence,
            at: self.ports.clock.now(),
        };
        self.change_report(id, |r| {
            if r.resolution.is_none() {
                r.resolution = Some(result.clone());
            }
            Ok(())
        })
        .await?;
        self.apply_report_effects(id).await
    }
    pub async fn verify_report_evidence(
        &self,
        caller: &Caller,
        id: &str,
        job: &crate::ports::LeasedJob,
        lease: &dyn crate::ports::JobLease,
    ) -> CoreResult<bool> {
        caller.require(Role::Reviewer)?;
        let r = self.stored_report(id).await?;
        if r.resolution.is_some() {
            self.apply_report_effects(id).await?;
            return Ok(true);
        }
        let source = self.load(&r.submission).await?;
        if source.current_version().map(|v| v.number) != Some(r.version)
            || source.claims_revision != r.claims_revision
        {
            self.change_report(id, |r| {
                r.resolution = Some(ErrorResolution {
                    verdict: ErrorVerdict::Unclear,
                    reason: "The source inputs changed before this report could be checked.".into(),
                    evidence: String::new(),
                    at: self.ports.clock.now(),
                });
                r.effects_applied = true;
                Ok(())
            })
            .await?;
            return Ok(true);
        }
        let Some(solution) = &r.solution else {
            return Ok(false);
        };
        let ErrorSubject::Statement { claim } = &r.subject else {
            return Ok(false);
        };
        // Submit through exactly the same attempt boundary. Reporter owns the resulting attempt.
        let reporter = Caller {
            person: r.owner.clone(),
            roles: Default::default(),
        };
        let attempt = self
            .submit_attempt(
                &reporter,
                &r.record,
                claim,
                solution.clone(),
                (r.reporter.kind == EntrantKind::Agent).then(|| r.reporter.id.clone()),
                String::new(),
            )
            .await?;
        self.verify_attempt_inner(caller, &attempt.id, Some((job, lease)))
            .await?;
        let (_, attempt) = self.find_attempt(&attempt.id).await?;
        self.change_report(id, |r| {
            r.attempt = Some(attempt.id.clone());
            r.receipt = attempt.receipt.clone();
            Ok(())
        })
        .await?;
        if let Some(receipt) = attempt.receipt
            && receipt.verdict == Verdict::Disproved
        {
            self.resolve_error_report(
                caller,
                id,
                ErrorVerdict::Confirmed,
                "The independent verifier disproved the author-confirmed Lean target.".into(),
                format!(
                    "Verified attempt {} against target {}",
                    attempt.id, receipt.target_digest
                ),
            )
            .await?;
            return Ok(true);
        }
        if r.text.trim().is_empty() {
            self.resolve_error_report(
                caller,
                id,
                ErrorVerdict::Unclear,
                "The Lean evidence did not verify a disproof; no text evidence was supplied."
                    .into(),
                String::new(),
            )
            .await?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(crate) async fn apply_report_effects(&self, id: &str) -> CoreResult<()> {
        let r = self.stored_report(id).await?;
        if r.effects_applied || r.resolution.is_none() {
            return Ok(());
        }
        if r.resolution
            .as_ref()
            .is_some_and(|v| v.verdict == ErrorVerdict::Confirmed)
        {
            if let ErrorSubject::Target { claim, digest } = &r.subject {
                loop {
                    let mut s = self.load(&r.submission).await?;
                    if let Some(a) = s
                        .lean_statements
                        .iter_mut()
                        .rev()
                        .find(|a| a.claim == *claim && a.digest == *digest)
                        && !matches!(a.response, LeanStatementResponse::Rejected { .. })
                    {
                        a.response = LeanStatementResponse::Rejected {
                            comment: format!(
                                "Confirmed error report: {}. {}",
                                r.text,
                                r.resolution.as_ref().unwrap().reason
                            ),
                            at: self.ports.clock.now(),
                        };
                        match self.save(&mut s).await {
                            Err(CoreError::StaleRevision { .. }) => continue,
                            result => result?,
                        }
                    }
                    break;
                }
                self.ports
                    .queue
                    .enqueue(&r.submission, JobKind::LeanStatement)
                    .await?;
            }
            self.request_recheck(&r.submission, &format!("error-report:{}", r.id))
                .await?;
        }
        self.change_report(id, |r| {
            r.effects_applied = true;
            Ok(())
        })
        .await
    }
    pub async fn error_notifications(&self, caller: &Caller) -> CoreResult<Vec<ErrorReportView>> {
        let mut out = vec![];
        for r in self.all_error_reports().await? {
            if r.resolution
                .as_ref()
                .is_some_and(|v| v.verdict == ErrorVerdict::Confirmed)
                && self.load(&r.submission).await?.involves(&caller.person)
            {
                out.push(self.error_report(Some(caller), &r.id).await?);
            }
        }
        Ok(out)
    }
}
