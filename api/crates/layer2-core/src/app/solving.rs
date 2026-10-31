use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId, SubmissionId, new_uuid},
    model::*,
    ports::JobKind,
};
use chrono::{Datelike, Duration};
use std::collections::BTreeMap;
pub(crate) struct AttemptInput {
    pub intermediate: Option<IntermediateProposition>,
    pub reuses: Vec<ProofReuse>,
    pub modules: BTreeMap<String, String>,
    pub expected_target: Option<String>,
    pub solution: String,
    pub as_agent: Option<String>,
    pub note: String,
    pub attribution: Option<FormalizationAttribution>,
}
impl App {
    pub async fn record_venue_attempt(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        digest: &str,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        if self.target(record, claim).await?.digest != digest {
            return Err(CoreError::conflict("target changed"));
        }
        loop {
            let mut file = self.solve_file(record, claim).await?;
            if !file.agent_attempted.insert(digest.into()) {
                return Ok(());
            }
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => return result,
            }
        }
    }

    /// A terminal result admits at most one continuation, including after a
    /// lost response or worker restart. An empty response uses its exact round;
    /// a rejected proof additionally consumes the verifier attempt identity.
    #[allow(clippy::too_many_arguments)]
    pub async fn advance_formalization_round(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        digest: &str,
        expected_round: u32,
        rejected_attempt: Option<&str>,
    ) -> CoreResult<u32> {
        caller.require(Role::Reviewer)?;
        if self.target(record, claim).await?.digest != digest {
            return Err(CoreError::conflict("target changed"));
        }
        loop {
            let mut file = self.solve_file(record, claim).await?;
            let current = file.agent_rounds.get(digest).copied().unwrap_or(0);
            if let Some(id) = rejected_attempt {
                if !file.attempts.iter().any(|attempt| {
                    attempt.id == id
                        && attempt.intermediate.is_none()
                        && attempt.target_digest == digest
                        && attempt.entrant.id == "wishpool-agent"
                        && attempt
                            .receipt
                            .as_ref()
                            .is_some_and(|r| r.verdict == Verdict::Rejected)
                }) {
                    return Err(CoreError::conflict(
                        "continuation needs a rejected venue attempt",
                    ));
                }
                if file.agent_after_attempt.get(digest).map(String::as_str) == Some(id) {
                    return Ok(current);
                }
            }
            if current != expected_round {
                return Ok(current);
            }
            let next = current
                .checked_add(1)
                .ok_or_else(|| CoreError::conflict("formalization round limit reached"))?;
            file.agent_rounds.insert(digest.into(), next);
            if let Some(id) = rejected_attempt {
                file.agent_after_attempt.insert(digest.into(), id.into());
            }
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    return Ok(next);
                }
            }
        }
    }
    pub async fn venue_agent(&self, caller: &Caller) -> CoreResult<(Caller, Entrant)> {
        caller.require(Role::Reviewer)?;
        let owner = self
            .caller(&VerifiedIdentity {
                subject: "wishpool:venue".into(),
                name: Some("Mathesis".into()),
                email: None,
                picture: None,
            })
            .await?;
        if let Some(agent) = self
            .ports
            .solving
            .agents()
            .await?
            .into_iter()
            .find(|a| a.id == "wishpool-agent")
        {
            return Ok((owner, agent));
        }
        let agent = Entrant {
            id: "wishpool-agent".into(),
            name: "mathesis-agent".into(),
            kind: EntrantKind::Agent,
            owner: Some(Owner {
                id: owner.person.clone(),
                name: "Mathesis".into(),
            }),
            retired: false,
            revision: 0,
        };
        self.ports.solving.insert_agent(&agent).await?;
        Ok((owner, agent))
    }

    pub(crate) async fn conjecture_source(&self, record: &RecordId) -> CoreResult<Submission> {
        let record = self
            .ports
            .records
            .get(record)
            .await?
            .ok_or_else(|| CoreError::not_found("paper", record.as_str()))?;
        let s = self.load(&record.submission).await?;
        if s.analysis_visibility != Visibility::Public
            || !matches!(&s.status, SubmissionStatus::Accepted { record: id } if id == &record.id)
        {
            return Err(CoreError::not_found("conjecture", record.id.as_str()));
        }
        Ok(s)
    }

    pub async fn agents(&self, caller: &Caller) -> CoreResult<Vec<Entrant>> {
        Ok(self
            .ports
            .solving
            .agents()
            .await?
            .into_iter()
            .filter(|a| a.owner.as_ref().is_some_and(|o| o.id == caller.person))
            .collect())
    }
    pub async fn create_agent(&self, caller: &Caller, name: String) -> CoreResult<Entrant> {
        require_text("agent name", &name, 40)?;
        if ["wishpool-agent", "mathesis-agent"]
            .iter()
            .any(|reserved| name.trim().eq_ignore_ascii_case(reserved))
        {
            return Err(CoreError::invalid(
                "this agent name is reserved for the venue",
            ));
        }
        let person = self.me(caller).await?;
        let agent = Entrant {
            id: format!("agent:{}", new_uuid()),
            name: name.trim().into(),
            kind: EntrantKind::Agent,
            owner: Some(Owner {
                id: caller.person.clone(),
                name: person.display_name,
            }),
            retired: false,
            revision: 0,
        };
        self.ports.solving.insert_agent(&agent).await?;
        Ok(agent)
    }
    pub async fn update_agent(
        &self,
        caller: &Caller,
        id: &str,
        name: Option<String>,
        retire: bool,
    ) -> CoreResult<Entrant> {
        let mut agent = self
            .agents(caller)
            .await?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| CoreError::not_found("agent", id))?;
        if let Some(name) = name {
            require_text("agent name", &name, 40)?;
            if ["wishpool-agent", "mathesis-agent"]
                .iter()
                .any(|reserved| name.trim().eq_ignore_ascii_case(reserved))
            {
                return Err(CoreError::invalid(
                    "this agent name is reserved for the venue",
                ));
            }
            agent.name = name.trim().into();
        }
        let expected = agent.revision;
        agent.revision += 1;
        agent.retired |= retire;
        self.ports.solving.replace_agent(&agent, expected).await?;
        Ok(agent)
    }
    pub(crate) async fn solve_file(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<SolveFile> {
        let s = self.conjecture_source(record).await?;
        if !s.approved_statement(claim)
            && !s.claim(claim).is_some_and(|c| {
                c.kind.is_open()
                    && (s.kind == SubmissionKind::Conjecture && c.role == ClaimRole::Main
                        || s.kind != SubmissionKind::Conjecture && !c.has_proof)
            })
        {
            return Err(CoreError::not_found("conjecture", claim.as_str()));
        }
        let version = s.current_version().unwrap().number;
        let id = format!("{record}:{claim}:v{version}:c{}", s.claims_revision);
        if let Some(file) = self.ports.solving.get(&id).await? {
            return Ok(file);
        }
        let file = SolveFile {
            authorization: None,
            agent_attempted: Default::default(),
            agent_rounds: Default::default(),
            agent_after_attempt: Default::default(),
            id: id.clone(),
            record: record.clone(),
            submission: s.id.clone(),
            claim: claim.clone(),
            version,
            claims_revision: s.claims_revision,
            revision: 0,
            admitted: s.approved_statement(claim)
                || s.kind == SubmissionKind::Conjecture
                    && matches!(
                        s.latest_report(Stage::Escape).map(|r| &r.payload),
                        Some(StagePayload::Escape { assessments }) if assessments.iter().any(|a| a.claim == *claim
                            && a.conjecture.as_ref().is_some_and(crate::policy::Policy::admits_open_problem)
                            && a.methodology.as_ref().is_none_or(|method| method.supports_content(true)))
                    ),
            failure: None,
            review_round: 1,
            review_contract: REVIEW_CONTRACT,
            task: None,
            report: None,
            audit: None,
            attempts: vec![],
            dependencies: vec![],
            downstream: 0,
        };
        match self.ports.solving.insert(&file).await {
            Ok(()) => Ok(file),
            Err(error) => self.ports.solving.get(&id).await?.ok_or(error),
        }
    }
    pub(crate) async fn save_solve_file(&self, file: &mut SolveFile) -> CoreResult<()> {
        let expected = file.revision;
        file.revision += 1;
        self.ports.solving.replace(file, expected).await
    }
    pub async fn open_problem_files(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<Vec<SolveFile>> {
        caller.require(Role::Reviewer)?;
        let s = self.submission(caller, id).await?;
        let SubmissionStatus::Accepted { record } = &s.status else {
            return Ok(vec![]);
        };
        if s.analysis_visibility != Visibility::Public {
            return Ok(vec![]);
        }
        let mut files = vec![];
        for claim in s.claims.iter().filter(|c| {
            c.kind.is_open()
                && !c.has_proof
                && (s.kind != SubmissionKind::Conjecture || c.role == ClaimRole::Main)
        }) {
            files.push(self.solve_file(record, &claim.id).await?);
        }
        Ok(files)
    }

    /// Accepted, public paper statements whose authors approved a Lean target.
    /// These use the same durable solve-file and independent verifier path as
    /// community conjectures, but are kept separate from the open-problem
    /// directory so an accepted theorem is never presented as a conjecture.
    pub async fn formalization_files(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<Vec<SolveFile>> {
        caller.require(Role::Reviewer)?;
        let s = self.submission(caller, id).await?;
        let SubmissionStatus::Accepted { record } = &s.status else {
            return Ok(vec![]);
        };
        if s.analysis_visibility != Visibility::Public {
            return Ok(vec![]);
        }
        let mut files = Vec::new();
        for claim in s
            .claims
            .iter()
            .filter(|claim| s.approved_statement(&claim.id))
        {
            // A file exists only after the author confirmed the exact target.
            if let Ok(file) = self.solve_file(record, &claim.id).await {
                files.push(file);
            }
        }
        Ok(files)
    }
    pub async fn queue_open_problems(&self, caller: &Caller, id: &SubmissionId) -> CoreResult<()> {
        caller.require(Role::Admin)?;
        let files = self.open_problem_files(caller, id).await?;
        if !files.is_empty() {
            let mut s = self.submission(caller, id).await?;
            if !s.problem_check_requested {
                s.problem_check_requested = true;
                self.save(&mut s).await?;
            }
        }
        for mut file in files.iter().filter(|f| f.failure.is_some()).cloned() {
            file.failure = None;
            file.task = None;
            file.report = None;
            file.audit = None;
            file.review_round += 1;
            self.save_solve_file(&mut file).await?;
        }
        if !files.is_empty() {
            self.ports.queue.enqueue(id, JobKind::OpenProblems).await?;
        }
        Ok(())
    }
    /// Composition-only, after sanitization; every step is bound to the source inputs.
    pub async fn record_problem_review(
        &self,
        caller: &Caller,
        mut file: SolveFile,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        let s = self.submission(caller, &file.submission).await?;
        let stored = self
            .ports
            .solving
            .get(&file.id)
            .await?
            .ok_or_else(|| CoreError::not_found("problem review", &file.id))?;
        if stored.review_contract != file.review_contract {
            return Err(CoreError::conflict(
                "a problem review retains its admitted method contract",
            ));
        }
        if s.current_version().map(|v| v.number) != Some(file.version)
            || s.claims_revision != file.claims_revision
            || s.analysis_visibility != Visibility::Public
            || !matches!(s.status, SubmissionStatus::Accepted { .. })
        {
            return Err(CoreError::conflict("open problem inputs changed"));
        }
        if let Some(audit) = &file.audit {
            file.admitted = audit
                .claims
                .iter()
                .find(|c| c.claim == file.claim)
                .is_some_and(|reading| {
                    reading
                        .conjecture
                        .as_ref()
                        .is_some_and(crate::policy::Policy::admits_open_problem)
                        && if file.review_contract >= REVIEW_CONTRACT {
                            reading
                                .methodology
                                .as_ref()
                                .is_some_and(|method| method.supports_content(true))
                        } else {
                            true
                        }
                });
        }
        if file.failure.is_some() {
            file.admitted = false;
        }
        self.save_solve_file(&mut file).await?;
        if file.admitted {
            self.ports
                .queue
                .enqueue(&file.submission, JobKind::LeanStatement)
                .await?;
        }
        Ok(())
    }
    pub async fn target(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<super::records::PublicLeanStatement> {
        let file = self.solve_file(record, claim).await?;
        if !file.admitted {
            return Err(CoreError::not_found("conjecture", claim.as_str()));
        }
        self.conjecture_source(record)
            .await?
            .lean_statements
            .into_iter()
            .rev()
            .find(|a| {
                a.claim == *claim
                    && a.version == file.version
                    && a.claims_revision == file.claims_revision
                    && a.faithfulness.is_some()
                    && a.lean.contains("def wishpool_target_prop : Prop :=")
                    && matches!(a.response, LeanStatementResponse::Confirmed { .. })
            })
            .map(|a| super::records::PublicLeanStatement {
                claim: a.claim,
                lean: a.lean,
                digest: a.digest,
                toolchain: a.toolchain,
            })
            .ok_or_else(|| CoreError::conflict("the author has not confirmed a target"))
    }
    pub(crate) async fn resolve_entrant(
        &self,
        caller: &Caller,
        as_agent: Option<String>,
    ) -> CoreResult<Entrant> {
        let entrant = if let Some(agent) = as_agent {
            self.agents(caller)
                .await?
                .into_iter()
                .find(|a| (a.id == agent || a.name == agent) && !a.retired)
                .ok_or_else(|| CoreError::not_found("agent", agent))?
        } else {
            Entrant {
                id: caller.person.to_string(),
                name: self.me(caller).await?.display_name,
                kind: EntrantKind::Person,
                owner: None,
                retired: false,
                revision: 0,
            }
        };
        Ok(entrant)
    }
    pub async fn submit_attempt(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        solution: String,
        as_agent: Option<String>,
        note: String,
    ) -> CoreResult<AttemptView> {
        self.submit_attributed_attempt(
            caller,
            record,
            claim,
            AttemptInput {
                intermediate: None,
                reuses: vec![],
                expected_target: None,
                modules: BTreeMap::new(),
                solution,
                as_agent,
                note,
                attribution: None,
            },
        )
        .await
    }
    pub(crate) async fn submit_attributed_attempt(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        input: AttemptInput,
    ) -> CoreResult<AttemptView> {
        let AttemptInput {
            intermediate,
            reuses,
            modules,
            expected_target,
            solution,
            as_agent,
            note,
            attribution,
        } = input;
        if solution.trim().is_empty() || solution.len() > 1_048_576 {
            return Err(CoreError::invalid(
                "solution must be a Lean file of at most 1 MB",
            ));
        }
        if note.chars().count() > 2000 {
            return Err(CoreError::invalid("note exceeds 2,000 characters"));
        }
        let target = self.target(record, claim).await?;
        if expected_target
            .as_ref()
            .is_some_and(|digest| digest != &target.digest)
        {
            return Err(CoreError::conflict(
                "target changed; review the new target before verification",
            ));
        }
        let entrant = self.resolve_entrant(caller, as_agent).await?;
        validate_lean_modules(&solution, &modules)?;
        let hash = lean_solution_digest(&solution, &modules);
        let mut module_blobs = BTreeMap::new();
        for (name, text) in &modules {
            module_blobs.insert(
                name.clone(),
                self.ports
                    .blobs
                    .put(text.as_bytes(), "text/plain; charset=utf-8")
                    .await?,
            );
        }
        let blob = self
            .ports
            .blobs
            .put(solution.as_bytes(), "text/plain; charset=utf-8")
            .await?;
        loop {
            let mut file = self.solve_file(record, claim).await?;
            if let Some(existing) = file.attempts.iter().find(|a| {
                a.entrant.id == entrant.id
                    && a.target_digest == target.digest
                    && a.solution_digest == hash
                    && a.intermediate == intermediate
                    && a.reuses == reuses
            }) {
                if attribution.is_some() && existing.attribution != attribution {
                    return Err(CoreError::conflict(
                        "this source already has an attempt with different attribution; use its existing receipt",
                    ));
                }
                if existing.receipt.is_none() && existing.invalidated.is_none() {
                    self.ports
                        .queue
                        .enqueue(&SubmissionId(existing.id.clone()), JobKind::VerifyAttempt)
                        .await?;
                }
                return self.attempt(Some(caller), &existing.id).await;
            }
            let now = self.ports.clock.now();
            if file
                .attempts
                .iter()
                .filter(|a| a.entrant.id == entrant.id && a.submitted_at > now - Duration::days(1))
                .count()
                >= self.attempts_per_day
            {
                return Err(CoreError::conflict(
                    "daily attempt limit reached for this entrant and conjecture",
                ));
            }
            let attempt = Attempt {
                intermediate: intermediate.clone(),
                reuses: reuses.clone(),
                modules: module_blobs.clone(),
                attribution: attribution.clone(),
                id: new_uuid(),
                owner: caller.person.clone(),
                entrant: entrant.clone(),
                target_digest: target.digest.clone(),
                solution_digest: hash.clone(),
                solution: blob.clone(),
                note: note.clone(),
                submitted_at: now,
                receipt: None,
                invalidated: None,
            };
            file.attempts.push(attempt.clone());
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => result?,
            }
            self.ports
                .queue
                .enqueue(&SubmissionId(attempt.id.clone()), JobKind::VerifyAttempt)
                .await?;
            return self.attempt(Some(caller), &attempt.id).await;
        }
    }
    pub(crate) async fn current_entrant(&self, snapshot: &Entrant) -> CoreResult<Entrant> {
        let mut entrant = if snapshot.kind == EntrantKind::Agent {
            self.ports
                .solving
                .agents()
                .await?
                .into_iter()
                .find(|a| a.id == snapshot.id)
                .unwrap_or_else(|| snapshot.clone())
        } else {
            let mut entrant = snapshot.clone();
            if let Some(person) = self.ports.people.get(&snapshot.id.clone().into()).await? {
                entrant.name = person.display_name;
            }
            entrant
        };
        if let Some(owner) = &mut entrant.owner
            && let Some(person) = self.ports.people.get(&owner.id).await?
        {
            owner.name = person.display_name;
        }
        Ok(entrant)
    }
    async fn attempt_inputs_current(
        &self,
        file: &SolveFile,
        attempt: &Attempt,
    ) -> CoreResult<bool> {
        match self.solve_file(&file.record, &file.claim).await {
            Ok(current) if current.id == file.id => {}
            Ok(_) | Err(CoreError::NotFound { .. }) | Err(CoreError::Conflict(_)) => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
        match self.target(&file.record, &file.claim).await {
            Ok(target) => Ok(target.digest == attempt.target_digest),
            Err(CoreError::NotFound { .. }) | Err(CoreError::Conflict(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
    async fn invalidate_attempt(&self, id: &str) -> CoreResult<()> {
        loop {
            let (mut file, attempt) = self.find_attempt(id).await?;
            if attempt.receipt.is_some() || attempt.invalidated.is_some() {
                return Ok(());
            }
            file.attempts.iter_mut().find(|a| a.id == id).unwrap().invalidated =
                Some("The source version or confirmed target changed; submit against the current target.".into());
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => return result,
            }
        }
    }
    pub(crate) async fn find_attempt(&self, id: &str) -> CoreResult<(SolveFile, Attempt)> {
        for file in self.ports.solving.all().await? {
            if let Some(a) = file.attempts.iter().find(|a| a.id == id) {
                return Ok((file.clone(), a.clone()));
            }
        }
        Err(CoreError::not_found("attempt", id))
    }
    pub(crate) async fn solution_text(&self, attempt: &Attempt) -> CoreResult<String> {
        let bytes = self
            .ports
            .blobs
            .get(&attempt.solution.id)
            .await?
            .ok_or_else(|| CoreError::not_found("solution", &attempt.id))?;
        use sha2::Digest;
        if bytes.len() as u64 != attempt.solution.bytes
            || format!("{:x}", sha2::Sha256::digest(&bytes)) != attempt.solution.sha256
        {
            return Err(CoreError::Unavailable("solution integrity failure".into()));
        }
        String::from_utf8(bytes).map_err(|_| CoreError::invalid("solution is not UTF-8"))
    }
    pub async fn attempt(&self, caller: Option<&Caller>, id: &str) -> CoreResult<AttemptView> {
        let (file, attempt) = self.find_attempt(id).await?;
        let owner = caller.is_some_and(|c| c.person == attempt.owner || Self::is_staff(c));
        let public = attempt.verified()
            && self.reuse_inputs_public(&attempt).await?
            && self
                .solve_file(&file.record, &file.claim)
                .await
                .is_ok_and(|f| f.id == file.id)
            && self
                .target(&file.record, &file.claim)
                .await
                .is_ok_and(|t| t.digest == attempt.target_digest);
        if !owner && !public {
            return Err(CoreError::not_found("attempt", id));
        }
        Ok(AttemptView {
            intermediate: attempt.intermediate.clone(),
            reuses: attempt.reuses.clone(),
            modules: self.lean_module_sources(&attempt.modules).await?,
            attribution: attempt.attribution.clone(),
            id: attempt.id.clone(),
            record: file.record,
            claim: file.claim,
            entrant: self.current_entrant(&attempt.entrant).await?,
            reason: attempt.invalidated.clone(),
            state: attempt
                .receipt
                .as_ref()
                .map_or(
                    if attempt.invalidated.is_some() {
                        "superseded"
                    } else {
                        "queued"
                    },
                    |r| match r.verdict {
                        Verdict::Proved => "proved",
                        Verdict::Disproved => "disproved",
                        Verdict::Rejected => "rejected",
                    },
                )
                .into(),
            receipt: attempt.receipt.clone(),
            solution: self.solution_text(&attempt).await?,
            note: owner.then_some(attempt.note),
        })
    }
    pub async fn my_attempts(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<Vec<AttemptView>> {
        let file = self.solve_file(record, claim).await?;
        let mut out = vec![];
        for a in file.attempts.iter().filter(|a| a.owner == caller.person) {
            out.push(self.attempt(Some(caller), &a.id).await?);
        }
        Ok(out)
    }
    /// Only the credential-free verifier port can produce an accepted receipt.
    pub async fn verify_attempt(&self, caller: &Caller, id: &str) -> CoreResult<()> {
        self.verify_attempt_inner(caller, id, None).await
    }
    pub async fn verify_attempt_leased(
        &self,
        caller: &Caller,
        job: &crate::ports::LeasedJob,
        lease: &dyn crate::ports::JobLease,
    ) -> CoreResult<()> {
        self.verify_attempt_inner(caller, job.submission.as_str(), Some((job, lease)))
            .await
    }
    pub(crate) async fn verify_attempt_inner(
        &self,
        caller: &Caller,
        id: &str,
        lease: Option<(&crate::ports::LeasedJob, &dyn crate::ports::JobLease)>,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        let (file, attempt) = self.find_attempt(id).await?;
        if attempt.receipt.is_some() || attempt.invalidated.is_some() {
            self.sync_formalization_receipt(caller, &file, &attempt)
                .await?;
            return self.disproof_recheck(&file, &attempt).await;
        }
        let target = match self.target(&file.record, &file.claim).await {
            Ok(target)
                if target.digest == attempt.target_digest
                    && self.solve_file(&file.record, &file.claim).await?.id == file.id =>
            {
                target
            }
            Ok(_) | Err(CoreError::NotFound { .. }) | Err(CoreError::Conflict(_)) => {
                return self.invalidate_attempt(id).await;
            }
            Err(error) => return Err(error),
        };
        if !self.reuse_inputs_public(&attempt).await? {
            return self.invalidate_attempt(id).await;
        }
        let solution = self.solution_text(&attempt).await?;
        let verification_target = attempt
            .intermediate
            .as_ref()
            .map_or(target.lean.clone(), |spec| spec.target(&attempt.modules));
        let verification_digest = lean_solution_digest(&verification_target, &BTreeMap::new());
        let receipt = self
            .ports
            .verifier
            .verify(&VerificationRequest {
                required_declaration: attempt.intermediate.as_ref().map(|s| s.name.clone()),
                inspect_constants: attempt
                    .reuses
                    .iter()
                    .map(|r| r.declaration.clone())
                    .collect(),
                modules: self.lean_module_sources(&attempt.modules).await?,
                target: verification_target,
                target_digest: verification_digest.clone(),
                toolchain: target.toolchain.clone(),
                solution,
            })
            .await?;
        if receipt.target_digest != verification_digest
            || receipt.solution_digest != attempt.solution_digest
            || receipt.toolchain != target.toolchain
            || receipt
                .used_constants
                .iter()
                .any(|name| !attempt.reuses.iter().any(|r| &r.declaration == name))
            || receipt
                .axioms
                .iter()
                .any(|a| !["propext", "Classical.choice", "Quot.sound"].contains(&a.as_str()))
        {
            return Err(CoreError::Unavailable("invalid verifier receipt".into()));
        }
        loop {
            if let Some((job, lease)) = lease
                && !lease.renew(job).await?
            {
                return Err(CoreError::conflict("attempt job lease was lost"));
            }
            // Recheck current source identity and target after the process returns.
            if !self.attempt_inputs_current(&file, &attempt).await?
                || !self.reuse_inputs_public(&attempt).await?
            {
                return self.invalidate_attempt(id).await;
            }
            let mut current = self.solve_file(&file.record, &file.claim).await?;
            let stored = current
                .attempts
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or_else(|| CoreError::not_found("attempt", id))?;
            if stored.receipt.is_some() {
                return Ok(());
            }
            stored.receipt = Some(receipt.clone());
            match self.save_solve_file(&mut current).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    let saved_attempt = current.attempts.iter().find(|a| a.id == id).unwrap();
                    let result = self.disproof_recheck(&current, saved_attempt).await;
                    self.sync_formalization_receipt(caller, &current, saved_attempt)
                        .await?;
                    result?;
                    return Ok(());
                }
            }
        }
    }
    /// Replay-safe projection: a saved receipt survives a worker crash between
    /// verification and updating the author view. No second Lean run is needed.
    async fn sync_formalization_receipt(
        &self,
        caller: &Caller,
        file: &SolveFile,
        attempt: &Attempt,
    ) -> CoreResult<()> {
        if attempt.intermediate.is_some() {
            return Ok(());
        }
        let Some(receipt) = &attempt.receipt else {
            return Ok(());
        };
        let submission = self.load(&file.submission).await?;
        if submission.analysis_visibility != Visibility::Public
            || !submission.approved_statement(&file.claim)
            || !self.attempt_inputs_current(file, attempt).await?
        {
            return Ok(());
        }
        let Some(item) = submission.formalization.item(&file.claim) else {
            return Ok(());
        };
        let state = match receipt.verdict {
            Verdict::Proved => FormalizationProgressState::Verified,
            Verdict::Rejected => FormalizationProgressState::Partial,
            Verdict::Disproved => FormalizationProgressState::Blocked,
        };
        let mut progress = item.progress.clone().unwrap_or(FormalizationProgress {
            state,
            target_digest: None,
            toolchain: None,
            attempts: 0,
            completed_lemmas: vec![],
            remaining_obligations: vec![],
            next_step: None,
            note: None,
            updated_at: self.ports.clock.now(),
        });
        progress.target_digest = Some(attempt.target_digest.clone());
        progress.toolchain = Some(receipt.toolchain.clone());
        progress.attempts = progress.attempts.max(
            file.attempts
                .iter()
                .filter(|a| a.target_digest == attempt.target_digest)
                .count() as u32,
        );
        progress.state = state;
        progress.next_step = Some(
            match state {
                FormalizationProgressState::Verified => {
                    "An independently checked proof is available."
                }
                FormalizationProgressState::Blocked => {
                    "A checked disproof requires review before proof work continues."
                }
                _ => "Saved proof work is unfinished. Further attempts require an active remaining allowance.",
            }
            .into(),
        );
        progress.note = Some(receipt.reason.clone());
        progress.updated_at = self.ports.clock.now();
        self.record_formalization_progress(caller, &file.submission, &file.claim, progress)
            .await?;
        if receipt.verdict == Verdict::Rejected && file.winner(&attempt.target_digest).is_none() {
            self.ports
                .queue
                .enqueue(&file.submission, JobKind::AgentAttempt)
                .await?;
        }
        Ok(())
    }
    pub(crate) async fn disproof_recheck(
        &self,
        file: &SolveFile,
        attempt: &Attempt,
    ) -> CoreResult<()> {
        if attempt.intermediate.is_none()
            && attempt
                .receipt
                .as_ref()
                .is_some_and(|r| r.verdict == Verdict::Disproved)
        {
            let s = self.load(&file.submission).await?;
            if s.kind != SubmissionKind::Conjecture {
                self.request_recheck(&file.submission, &format!("disproof:{}", attempt.id))
                    .await?;
            }
        }
        Ok(())
    }
    pub async fn public_attempts(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<Vec<PublicAttempt>> {
        let target = self.target(record, claim).await?;
        let file = self.public_solve_file(record, claim).await?;
        let winner = file.winner(&target.digest).map(|a| a.id.as_str());
        let mut out = vec![];
        for a in file.attempts.iter().filter(|a| {
            a.intermediate.is_none() && a.verified() && a.target_digest == target.digest
        }) {
            if !self.reuse_inputs_public(a).await? {
                continue;
            }
            out.push(PublicAttempt {
                reuses: a.reuses.clone(),
                modules: self.lean_module_sources(&a.modules).await?,
                attribution: a.attribution.clone(),
                id: a.id.clone(),
                entrant: self.current_entrant(&a.entrant).await?,
                receipt: a.receipt.clone().unwrap(),
                solution: self.solution_text(a).await?,
                also_verified: winner != Some(a.id.as_str()),
            });
        }
        out.sort_by_key(|a| (a.receipt.checked_at, a.id.clone()));
        Ok(out)
    }
    async fn solve_credits(&self) -> CoreResult<Vec<(Entrant, SolveCredit)>> {
        let mut out = vec![];
        for file in self.ports.solving.all().await? {
            if !self
                .solve_file(&file.record, &file.claim)
                .await
                .is_ok_and(|f| f.id == file.id)
            {
                continue;
            }
            let Ok(target) = self.target(&file.record, &file.claim).await else {
                continue;
            };
            let file = self.public_solve_file(&file.record, &file.claim).await?;
            let Some(winner) = file.winner(&target.digest) else {
                continue;
            };
            if !self.reuse_inputs_public(winner).await? {
                continue;
            }
            let source = self.conjecture_source(&file.record).await?;
            let identity = winner
                .entrant
                .owner
                .as_ref()
                .map(|o| o.id.clone())
                .unwrap_or_else(|| winner.entrant.id.clone().into());
            out.push((
                self.current_entrant(&winner.entrant).await?,
                SolveCredit {
                    others: !source.involves(&identity),
                    formalized: !source.claim(&file.claim).is_some_and(|c| c.kind.is_open()),
                    record: file.record.clone(),
                    claim: file.claim.clone(),
                    title: self.conjecture_source(&file.record).await?.title,
                    attempt: winner.id.clone(),
                    receipt: winner.receipt.clone().unwrap(),
                },
            ));
        }
        Ok(out)
    }
    pub async fn leaderboard(
        &self,
        period: &str,
        entrants: &str,
    ) -> CoreResult<Vec<LeaderboardRow>> {
        if !["all", "month"].contains(&period) || !["all", "people", "agents"].contains(&entrants) {
            return Err(CoreError::invalid("invalid leaderboard filter"));
        }
        let now = self.ports.clock.now();
        let mut rows: BTreeMap<String, LeaderboardRow> = BTreeMap::new();
        for (entrant, credit) in self.solve_credits().await? {
            if credit.formalized {
                continue;
            }
            let at = credit.receipt.checked_at;
            if period == "month" && (at.year() != now.year() || at.month() != now.month()) {
                continue;
            }
            if entrants == "people" && entrant.kind != EntrantKind::Person
                || entrants == "agents" && entrant.kind != EntrantKind::Agent
            {
                continue;
            }
            let row = rows.entry(entrant.id.clone()).or_insert(LeaderboardRow {
                rank: 0,
                entrant,
                solved: 0,
                disproved: 0,
                score: 0,
                others: 0,
                total: 0,
                last_solve: at,
            });
            if credit.receipt.verdict == Verdict::Proved {
                row.solved += 1;
            } else {
                row.disproved += 1;
            }
            row.score += 1;
            row.total += 1;
            row.others += usize::from(credit.others);
            row.last_solve = row.last_solve.max(at);
        }
        let mut rows: Vec<_> = rows.into_values().collect();
        rows.sort_by_key(|r| {
            (
                std::cmp::Reverse(r.others),
                std::cmp::Reverse(r.total),
                r.last_solve,
                r.entrant.id.clone(),
            )
        });
        for (i, row) in rows.iter_mut().enumerate() {
            row.rank = i + 1;
        }
        Ok(rows)
    }
    pub async fn entrant_profile(&self, id: &str) -> CoreResult<EntrantProfile> {
        let mut entrant = self
            .ports
            .solving
            .agents()
            .await?
            .into_iter()
            .find(|a| a.id == id);
        if entrant.is_none() {
            entrant = self.ports.people.get(&id.into()).await?.map(|p| Entrant {
                id: id.into(),
                name: p.display_name,
                kind: EntrantKind::Person,
                owner: None,
                retired: false,
                revision: 0,
            });
        }
        let mut profile = EntrantProfile {
            contributions: self.intermediate_credits(id).await?,
            entrant: entrant.ok_or_else(|| CoreError::not_found("entrant", id))?,
            solutions: vec![],
            others: 0,
            total: 0,
            formalized: vec![],
            errors_found: 0,
        };
        for (e, credit) in self.solve_credits().await? {
            if e.id == id {
                if credit.formalized {
                    if credit.receipt.verdict == Verdict::Proved {
                        profile.formalized.push(credit);
                    }
                } else {
                    profile.others += usize::from(credit.others);
                    profile.total += 1;
                    profile.solutions.push(credit);
                }
            }
        }
        profile.errors_found = self
            .error_reports(None, None)
            .await?
            .iter()
            .filter(|r| {
                r.reporter.id == id
                    && r.resolution
                        .as_ref()
                        .is_some_and(|r| r.verdict == ErrorVerdict::Confirmed)
            })
            .count();
        profile
            .solutions
            .sort_by_key(|s| std::cmp::Reverse(s.receipt.checked_at));
        Ok(profile)
    }
    pub async fn solve_notifications(&self, caller: &Caller) -> CoreResult<Vec<SolveNotification>> {
        let mut out = vec![];
        for (_, credit) in self.solve_credits().await? {
            let (file, attempt) = self.find_attempt(&credit.attempt).await?;
            if self.load(&file.submission).await?.submitter == caller.person {
                out.push(SolveNotification {
                    attempt: attempt.id,
                    record: file.record,
                    claim: file.claim,
                    entrant: self.current_entrant(&attempt.entrant).await?,
                    at: credit.receipt.checked_at,
                });
            }
        }
        Ok(out)
    }
    /// Repair the enqueue-after-save crash window without rerunning settled work.
    pub async fn refresh_dependency_counts(&self) -> CoreResult<()> {
        let mut edges = vec![];
        let mut cursor = None;
        loop {
            let page = self.ports.records.list(100, cursor).await?;
            for record in page.items {
                let Ok(s) = self.accepted(&record).await else {
                    continue;
                };
                if s.analysis_visibility != Visibility::Public {
                    continue;
                }
                for edge in &s.conjecture_dependencies {
                    edges.push((
                        (
                            edge.target.clone(),
                            edge.target_version,
                            edge.target_claims_revision,
                        ),
                        DependencyEdge {
                            record: record.id.clone(),
                            claim: edge.from.clone(),
                            version: s.current_version().unwrap().number,
                            claims_revision: s.claims_revision,
                        },
                    ));
                }
            }
            cursor = page.next_before;
            if cursor.is_none() {
                break;
            }
        }
        for mut file in self.ports.solving.all().await? {
            let dependencies: Vec<_> = edges
                .iter()
                .filter(|(target, edge)| {
                    target.0.record == file.record
                        && target.0.claim == file.claim
                        && target.1 == file.version
                        && target.2 == file.claims_revision
                        && edge.record != file.record
                })
                .map(|(_, edge)| edge.clone())
                .collect();
            let downstream = dependencies
                .iter()
                .map(|e| e.record.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            if dependencies != file.dependencies || downstream != file.downstream {
                file.dependencies = dependencies;
                file.downstream = downstream;
                match self.save_solve_file(&mut file).await {
                    Err(CoreError::StaleRevision { .. }) => {}
                    result => result?,
                }
            }
        }
        Ok(())
    }
    pub async fn reconcile_attempts(&self) -> CoreResult<()> {
        self.reconcile_private_rechecks().await?;
        let mut cursor = None;
        loop {
            let page = self
                .ports
                .submissions
                .list(
                    &crate::ports::SubmissionFilter {
                        status: Some("accepted".into()),
                        ..Default::default()
                    },
                    100,
                    cursor,
                )
                .await?;
            for s in page.items {
                // Recover the existing owner even after pause/expiry, a final receipt,
                // or an earlier worker completing before asynchronous deletion settled.
                // This schedules cleanup only; it grants no generation authorization.
                let (review, _) = self.referee_file(&s.id).await?;
                if review.agent_steps.values().any(|step| {
                    step.cma_attempts.iter().any(|attempt| {
                        let progress = if step.cma_attempts.last().map(|a| a.ordinal)
                            == Some(attempt.ordinal)
                        {
                            &step.progress
                        } else {
                            &attempt.progress
                        };
                        attempt.step.starts_with("agent-formalization:")
                            && progress["deleted"] != true
                            && (progress["agent_id"].is_string()
                                || progress["create_intent"].is_array())
                    })
                }) {
                    self.ports
                        .queue
                        .enqueue(&s.id, JobKind::AgentAttempt)
                        .await?;
                }
                if s.analysis_visibility == Visibility::Public
                    && let SubmissionStatus::Accepted { record } = &s.status
                {
                    // Existing approved paper targets also participate in discovery,
                    // including historical confirmations made before this directory.
                    for claim in s.claims.iter().filter(|c| s.approved_statement(&c.id)) {
                        let file = self.solve_file(record, &claim.id).await?;
                        if let Ok(target) = self.target(record, &claim.id).await {
                            let winner = file.winner(&target.digest);
                            // Legacy records, or a crash after storing the receipt,
                            // can lack the corresponding author-facing proof state.
                            if let Some(attempt) = winner {
                                let expected = if attempt
                                    .receipt
                                    .as_ref()
                                    .is_some_and(|r| r.verdict == Verdict::Proved)
                                {
                                    FormalizationProgressState::Verified
                                } else {
                                    FormalizationProgressState::Blocked
                                };
                                if s.formalization
                                    .item(&claim.id)
                                    .and_then(|i| i.progress.as_ref())
                                    .is_none_or(|p| {
                                        p.target_digest.as_deref() != Some(target.digest.as_str())
                                            || p.state != expected
                                    })
                                {
                                    self.ports
                                        .queue
                                        .enqueue(
                                            &SubmissionId::from(attempt.id.as_str()),
                                            JobKind::VerifyAttempt,
                                        )
                                        .await?;
                                }
                            } else if file.authorization.as_ref().is_some_and(|a| {
                                a.can_admit(
                                    &target.digest,
                                    file.agent_rounds.get(&target.digest).copied().unwrap_or(0),
                                    self.ports.clock.now(),
                                )
                            }) && !file.attempts.iter().any(|attempt| {
                                attempt.intermediate.is_none()
                                    && attempt.target_digest == target.digest
                                    && attempt.receipt.is_none()
                                    && attempt.invalidated.is_none()
                            }) {
                                self.ports
                                    .queue
                                    .enqueue(&s.id, JobKind::AgentAttempt)
                                    .await?;
                            }
                        }
                    }
                    let mut needs_review = false;
                    let mut needs_target = false;
                    for claim in s.claims.iter().filter(|c| {
                        c.kind.is_open()
                            && (s.kind == SubmissionKind::Conjecture && c.role == ClaimRole::Main
                                || s.kind != SubmissionKind::Conjecture && !c.has_proof)
                    }) {
                        if s.kind != SubmissionKind::Conjecture && !s.problem_check_requested {
                            continue;
                        }
                        let file = self.solve_file(record, &claim.id).await?;
                        needs_review |= s.kind != SubmissionKind::Conjecture
                            && file.audit.is_none()
                            && file.failure.is_none();
                        let latest = s.lean_statements.iter().rev().find(|a| {
                            a.claim == claim.id
                                && a.version == file.version
                                && a.claims_revision == file.claims_revision
                        });
                        if file.admitted
                            && latest.is_some_and(|a| {
                                a.faithfulness.is_some()
                                    && matches!(a.response, LeanStatementResponse::Confirmed { .. })
                                    && !file.agent_attempted.contains(&a.digest)
                                    && file.winner(&a.digest).is_none()
                            })
                        {
                            self.ports
                                .queue
                                .enqueue(&s.id, JobKind::AgentAttempt)
                                .await?;
                        }
                        needs_target |= file.admitted
                            && latest.is_none_or(|a| {
                                matches!(a.response, LeanStatementResponse::Rejected { .. })
                            });
                    }
                    if needs_review {
                        self.ports
                            .queue
                            .enqueue(&s.id, JobKind::OpenProblems)
                            .await?;
                    }
                    if needs_target {
                        // Standalone targets retain their post-letter gate.
                        let ready = if s.kind == SubmissionKind::Conjecture {
                            self.ports.referees.get(&s.id).await?.is_some_and(|file| {
                                file.current().is_some_and(|r| {
                                    r.version == s.current_version().unwrap().number
                                        && r.claims_revision == s.claims_revision
                                        && r.letter.done().is_some()
                                })
                            })
                        } else {
                            true
                        };
                        if ready {
                            self.ports
                                .queue
                                .enqueue(&s.id, JobKind::LeanStatement)
                                .await?;
                        }
                    }
                }
                if s.claims.iter().any(|c| {
                    s.approved_statement(&c.id)
                        && s.lean_statements
                            .iter()
                            .rev()
                            .find(|a| a.claim == c.id)
                            .is_none_or(|a| {
                                matches!(a.response, LeanStatementResponse::Rejected { .. })
                            })
                }) {
                    self.ports
                        .queue
                        .enqueue(&s.id, JobKind::LeanStatement)
                        .await?;
                }
                if let Some(file) = self.ports.referees.get(&s.id).await?
                    && !file.recheck_requests.is_empty()
                    && file
                        .current()
                        .is_none_or(|r| r.recheck_requests != file.recheck_requests)
                {
                    self.ports.queue.enqueue(&s.id, JobKind::Referee).await?;
                }
                if s.lean_statements.iter().any(|a| {
                    a.version == s.current_version().unwrap().number
                        && a.claims_revision == s.claims_revision
                        && (a.faithfulness.is_none()
                            || !a.lean.contains("def wishpool_target_prop"))
                        && !matches!(a.response, LeanStatementResponse::Rejected { .. })
                }) {
                    self.ports
                        .queue
                        .enqueue(&s.id, JobKind::LeanStatement)
                        .await?;
                }
            }
            cursor = page.next_before;
            if cursor.is_none() {
                break;
            }
        }
        for r in self.all_error_reports().await? {
            if r.resolution.is_none() || !r.effects_applied {
                self.ports
                    .queue
                    .enqueue(&r.submission, JobKind::ErrorReports)
                    .await?;
            }
        }
        for file in self.ports.solving.all().await? {
            for a in &file.attempts {
                self.disproof_recheck(&file, a).await?;
            }
            for a in file
                .attempts
                .iter()
                .filter(|a| a.receipt.is_none() && a.invalidated.is_none())
            {
                if !self.attempt_inputs_current(&file, a).await? {
                    self.invalidate_attempt(&a.id).await?;
                    continue;
                }
                self.ports
                    .queue
                    .enqueue(&SubmissionId(a.id.clone()), JobKind::VerifyAttempt)
                    .await?;
            }
        }
        Ok(())
    }
}
