use super::App;
use crate::{
    CoreError, CoreResult,
    ids::PersonId,
    model::*,
    ports::{JobLease, LeasedJob},
};

impl App {
    pub async fn my_cma(&self, caller: &Caller) -> CoreResult<MyCma> {
        Ok(MyCma {
            enabled: self.ports.cma_setup.enabled(),
            binding: self.cma_binding(&caller.person).await?,
            setup_url: self.ports.cma_setup.setup_url(),
        })
    }
    pub async fn cma_binding(&self, person: &PersonId) -> CoreResult<CmaBinding> {
        Ok(self
            .ports
            .cma_bindings
            .get(person)
            .await?
            .unwrap_or_else(|| CmaBinding::disconnected(person.clone(), self.ports.clock.now())))
    }
    async fn save_cma(&self, mut binding: CmaBinding) -> CoreResult<CmaBinding> {
        let expected = binding.revision;
        binding.revision += 1;
        binding.updated_at = self.ports.clock.now();
        self.ports.cma_bindings.replace(&binding, expected).await?;
        Ok(binding)
    }
    pub async fn begin_cma_connect(&self, caller: &Caller) -> CoreResult<CmaBinding> {
        if !self.ports.cma_setup.enabled() {
            return Err(CoreError::Unavailable("CMA connection is disabled".into()));
        }
        let mut b = self.cma_binding(&caller.person).await?;
        b.generation += 1;
        b.status = CmaStatus::SetupNeeded;
        b.reason = "consent_pending".into();
        b.workspace_id = None;
        b.sandbox_id = None;
        b.sandbox_profile = None;
        let old_generation = b.generation - 1;
        let saved = self.save_cma(b).await?;
        self.ports
            .cma_setup
            .revoke(&caller.person, old_generation)
            .await?;
        Ok(saved)
    }
    pub async fn complete_cma_connect(&self, person: &PersonId, generation: u64) -> CoreResult<()> {
        let mut b = self.cma_binding(person).await?;
        if b.generation != generation || b.status != CmaStatus::SetupNeeded {
            return Err(CoreError::conflict("CMA consent superseded"));
        }
        b.reason = "workspace_needed".into();
        self.save_cma(b).await?;
        Ok(())
    }
    pub async fn disconnect_cma(&self, caller: &Caller) -> CoreResult<CmaBinding> {
        let old = self.cma_binding(&caller.person).await?;
        let mut b = CmaBinding::disconnected(caller.person.clone(), self.ports.clock.now());
        b.revision = old.revision;
        b.generation = old.generation + 1;
        b.created_at = old.created_at;
        let saved = self.save_cma(b).await?;
        self.ports
            .cma_setup
            .revoke(&caller.person, old.generation)
            .await?;
        Ok(saved)
    }
    pub async fn set_cma_workspace(
        &self,
        caller: &Caller,
        input: SetCmaWorkspace,
    ) -> CoreResult<MyCma> {
        let mut b = self.cma_binding(&caller.person).await?;
        if b.revision != input.expected_revision {
            return Err(CoreError::StaleRevision {
                kind: "CMA binding",
                id: caller.person.to_string(),
            });
        }
        if b.status == CmaStatus::Disconnected || b.reason == "consent_pending" {
            return Err(CoreError::conflict("Connect CMA first"));
        }
        if input.workspace_id.is_empty()
            || input.workspace_id.len() > 200
            || !input
                .workspace_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(CoreError::invalid("invalid CMA workspace identifier"));
        }
        let checked = self
            .ports
            .cma_setup
            .validate(&b, &input.workspace_id)
            .await?;
        b.checked_at = Some(self.ports.clock.now());
        b.workspace_id = Some(input.workspace_id);
        b.sandbox_id = None;
        b.sandbox_profile = None;
        b.status = CmaStatus::SetupNeeded;
        b.reason = match checked {
            CmaWorkspaceCheck::AuthorizationRefused => {
                b.status = CmaStatus::Unusable;
                "authorization_refused"
            }
            CmaWorkspaceCheck::SetupMissing => "workspace_setup_needed",
            CmaWorkspaceCheck::Resources(facts) => {
                b.sandbox_id = Some(facts.sandbox_id);
                b.sandbox_profile = Some(facts.sandbox_profile);
                if facts.owner != b.subject.as_str() {
                    "workspace_not_owned"
                } else if facts.workspace_status != "ready" || facts.sandbox_status != "active" {
                    "workspace_not_ready"
                } else if facts.model_access != "own_credentials" {
                    "own_credentials_required"
                } else {
                    b.status = CmaStatus::Ready;
                    "author_connected"
                }
            }
        }
        .into();
        self.save_cma(b).await?;
        self.my_cma(caller).await
    }

    /// Eligible submission steps use only the submitter's binding; venue work stays
    /// operator-funded. Replays retain admission, checked under the current lease.
    pub async fn admit_cma_attempt(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        jobs: &dyn JobLease,
        key: &str,
        logical_step: &str,
        operator_workspace: &str,
    ) -> CoreResult<CmaAttempt> {
        caller.require(Role::Reviewer)?;
        let id = &job.submission;
        if !jobs.renew(job).await? {
            return Err(CoreError::conflict("job lease lost"));
        }
        let paper = self.submission(caller, id).await?;
        self.check_public_generation_step(caller, &paper, logical_step)
            .await?;
        let (mut file, stored) = self.referee_file(id).await?;
        if let Some(key) = logical_step
            .strip_prefix("research:")
            .and_then(|s| s.split(':').next())
            && !self.research_active(caller, id, key).await?
        {
            return Err(CoreError::conflict(
                "research authorization expired or changed",
            ));
        }
        if let Some(board) = logical_step
            .strip_prefix("binding:")
            .and_then(|s| s.split(':').next())
            && !self.binding_active(caller, id, board).await?
        {
            return Err(CoreError::conflict("binding sharing authorization changed"));
        }
        if let Some(attempt) = file
            .agent_steps
            .get(key)
            .and_then(|s| s.cma_attempts.last())
        {
            return Ok(attempt.clone());
        }
        if !file.agent_steps.contains_key(key)
            && matches!(
                paper.status,
                SubmissionStatus::Draft | SubmissionStatus::Withdrawn
            )
        {
            return Err(CoreError::conflict("agent inputs superseded"));
        }
        if key.is_empty() || key.len() > 240 || logical_step.len() > 240 {
            return Err(CoreError::invalid("invalid CMA step identity"));
        }
        let binding = self.cma_binding(&paper.submitter).await?;
        let legacy = file.agent_steps.contains_key(key);
        let identity = CmaStepIdentity::from_label(logical_step);
        let author_eligible = identity.kind.author_eligible();
        let author = author_eligible
            && !legacy
            && self.ports.cma_setup.enabled()
            && binding.status == CmaStatus::Ready
            && binding.workspace_id.is_some();
        let attempt = CmaAttempt {
            identity,
            step: logical_step.into(),
            ordinal: 0,
            payer: if author {
                CmaPayer::Author
            } else {
                CmaPayer::Operator
            },
            owner: author.then(|| paper.submitter.clone()),
            binding_generation: author.then_some(binding.generation),
            workspace: if author {
                binding.workspace_id.unwrap()
            } else {
                operator_workspace.into()
            },
            reason: if legacy {
                "legacy_unknown".into()
            } else if author {
                "author_connected".into()
            } else if !author_eligible {
                "operator_step".into()
            } else if !self.ports.cma_setup.enabled() {
                "byok_disabled".into()
            } else {
                binding.reason
            },
            fallback_from: None,
            admitted_at: self.ports.clock.now(),
            outcome: CmaOutcome::Running,
            finished_at: None,
            failure: None,
            progress: serde_json::Value::Null,
        };
        let step = file.agent_steps.entry(key.into()).or_insert(AgentStep {
            cma_attempts: vec![],
            version: paper.current_version().map_or(0, |v| v.number),
            claims_revision: paper.claims_revision,
            progress: serde_json::Value::Null,
        });
        step.cma_attempts.push(attempt.clone());
        if !jobs.renew(job).await? {
            return Err(CoreError::conflict("job lease lost"));
        }
        self.store_referee_file(&mut file, stored).await?;
        Ok(attempt)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fallback_cma_attempt(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        jobs: &dyn JobLease,
        key: &str,
        ordinal: u32,
        failure: CmaFailure,
        operator_workspace: &str,
    ) -> CoreResult<bool> {
        caller.require(Role::Reviewer)?;
        let id = &job.submission;
        if !jobs.renew(job).await? {
            return Err(CoreError::conflict("job lease lost"));
        }
        self.submission(caller, id).await?;
        let (mut file, stored) = self.referee_file(id).await?;
        let step = file
            .agent_steps
            .get_mut(key)
            .ok_or_else(|| CoreError::conflict("CMA attempt not admitted"))?;
        let Some(current) = step.cma_attempts.last_mut() else {
            return Ok(false);
        };
        if current.ordinal != ordinal {
            return Ok(current.fallback_from == Some(ordinal));
        }
        if current.payer != CmaPayer::Author
            || !failure.permits_fallback()
            || current.outcome == CmaOutcome::Succeeded
        {
            return Ok(false);
        }
        current.progress = std::mem::take(&mut step.progress);
        current.outcome = CmaOutcome::Failed;
        current.failure = Some(failure);
        current.finished_at = Some(self.ports.clock.now());
        let mut next = current.clone();
        next.ordinal += 1;
        next.payer = CmaPayer::Operator;
        next.owner = None;
        next.binding_generation = None;
        next.workspace = operator_workspace.into();
        next.reason = serde_json::to_value(failure)
            .unwrap()
            .as_str()
            .unwrap()
            .into();
        next.fallback_from = Some(ordinal);
        next.admitted_at = self.ports.clock.now();
        next.finished_at = None;
        next.outcome = CmaOutcome::Running;
        next.failure = None;
        next.progress = serde_json::Value::Null;
        let binding_owner = current.owner.clone().zip(current.binding_generation);
        step.cma_attempts.push(next);
        step.progress = serde_json::Value::Null;
        if !jobs.renew(job).await? {
            return Err(CoreError::conflict("job lease lost"));
        }
        self.store_referee_file(&mut file, stored).await?;
        if failure != CmaFailure::CauseUnknown
            && let Some((owner, generation)) = binding_owner
        {
            let mut binding = self.cma_binding(&owner).await?;
            if binding.generation == generation && binding.status == CmaStatus::Ready {
                binding.status = CmaStatus::Unusable;
                binding.reason = match failure {
                    CmaFailure::Auth | CmaFailure::Account => "authorization_refused",
                    CmaFailure::Setup => "workspace_setup_needed",
                    CmaFailure::Credential => "model_credentials_refused",
                    CmaFailure::Quota => "author_quota_refused",
                    _ => "workspace_setup_needed",
                }
                .into();
                // A concurrent reconnect/recheck owns the new revision.
                match self.save_cma(binding).await {
                    Ok(_) | Err(CoreError::StaleRevision { .. }) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(true)
    }
}
