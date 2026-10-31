//! Target-bound public work authorization, saved with the existing solve aggregate.
use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId},
    model::*,
    ports::JobKind,
};
use chrono::Duration;

impl App {
    pub async fn formalization_authorization(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<Option<FormalizationAuthorization>> {
        let target = self.target(record, claim).await?;
        Ok(self
            .solve_file(record, claim)
            .await?
            .authorization
            .filter(|a| a.target_digest == target.digest))
    }

    /// Only the submitting author can grant/extend work. Resuming does not reset consumption.
    pub async fn control_formalization(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        control: FormalizationControl,
    ) -> CoreResult<FormalizationAuthorization> {
        let source = self.conjecture_source(record).await?;
        Self::require_submitter(caller, &source)?;
        if !source.approved_statement(claim) {
            return Err(CoreError::conflict(
                "continuous work requires an author-approved paper formalization",
            ));
        }
        let target = self.target(record, claim).await?;
        if target.digest != control.target_digest {
            return Err(CoreError::conflict("the confirmed target changed"));
        }
        let now = self.ports.clock.now();
        if control.total_generations == 0
            || control.total_generations > 20
            || (!control.paused && control.expires_at <= now)
            || control.expires_at > now + Duration::days(30)
        {
            return Err(CoreError::invalid(
                "authorize 1–20 total generations with an expiry within 30 days",
            ));
        }
        loop {
            let mut file = self.solve_file(record, claim).await?;
            if self.target(record, claim).await?.digest != control.target_digest {
                return Err(CoreError::conflict("the confirmed target changed"));
            }
            let prior = file.authorization.as_ref();
            let changed_target = prior.is_some_and(|a| a.target_digest != control.target_digest);
            if changed_target && !control.change_allowance {
                return Err(CoreError::conflict(
                    "the confirmed target changed; explicitly authorize its saved allowance",
                ));
            }
            if !control.change_allowance
                && prior.is_some_and(|a| {
                    a.total_generations != control.total_generations
                        || a.expires_at != control.expires_at
                })
            {
                return Err(CoreError::conflict(
                    "saved limits changed; changing an allowance requires explicit authorization",
                ));
            }
            let admitted_rounds = prior.map(|a| a.admitted_rounds.clone()).unwrap_or_default();
            if control.total_generations < admitted_rounds.len() as u32 {
                return Err(CoreError::invalid(
                    "total allowance cannot be less than already consumed work",
                ));
            }
            let authorization = FormalizationAuthorization {
                target_digest: control.target_digest.clone(),
                authorized_by: caller.person.clone(),
                total_generations: control.total_generations,
                expires_at: control.expires_at,
                paused: control.paused,
                admitted_rounds,
                contexts: prior.map(|a| a.contexts.clone()).unwrap_or_default(),
            };
            if changed_target {
                // Rebinding is explicit and retains lifetime consumption. Use a
                // fresh round identity so a different target cannot recover an
                // already charged round as free work or reuse its helper context.
                let next = authorization.admitted_rounds.last().map_or(Ok(0), |r| {
                    r.checked_add(1)
                        .ok_or_else(|| CoreError::conflict("formalization round limit reached"))
                })?;
                let round = file
                    .agent_rounds
                    .entry(control.target_digest.clone())
                    .or_default();
                *round = (*round).max(next);
            }
            file.authorization = Some(authorization.clone());
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => result?,
            }
            if !authorization.paused {
                self.ports
                    .queue
                    .enqueue(&file.submission, JobKind::AgentAttempt)
                    .await?;
            }
            return Ok(authorization);
        }
    }

    /// CAS consumption precedes provider admission. Replays of this round use the same key.
    pub async fn admit_formalization_generation(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        digest: &str,
        round: u32,
    ) -> CoreResult<bool> {
        caller.require(Role::Reviewer)?;
        if self.target(record, claim).await?.digest != digest {
            return Ok(false);
        }
        let current = self.solve_file(record, claim).await?;
        let step = format!(
            "{}:v{}:c{}:agent-formalization:{claim}:{digest}:round{round}:a0",
            current.submission, current.version, current.claims_revision
        );
        use sha2::Digest;
        let key = format!("wishpool:{:x}", sha2::Sha256::digest(step.as_bytes()));
        let legacy = self
            .referee_file(&current.submission)
            .await?
            .0
            .agent_steps
            .contains_key(&key);
        let context = GenerationContext {
            contract: if legacy { 0 } else { 1 },
            contributions: if legacy {
                vec![]
            } else {
                self.checked_contributions(record, claim)
                    .await?
                    .into_iter()
                    .take(8)
                    .map(|c| c.origin.attempt)
                    .collect()
            },
        };
        loop {
            let mut file = self.solve_file(record, claim).await?;
            if file.agent_rounds.get(digest).copied().unwrap_or(0) != round {
                return Ok(false);
            }
            let Some(authorization) = &mut file.authorization else {
                return Ok(false);
            };
            if !authorization.can_admit(digest, round, self.ports.clock.now()) {
                return Ok(false);
            }
            if !authorization.admitted_rounds.insert(round) {
                return Ok(true);
            }
            authorization
                .contexts
                .insert(round.to_string(), context.clone());
            match self.save_solve_file(&mut file).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    return Ok(true);
                }
            }
        }
    }

    /// Rechecked by the managed run store before every provider operation, including polling.
    pub async fn formalization_generation_active(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        digest: &str,
        round: u32,
    ) -> CoreResult<bool> {
        caller.require(Role::Reviewer)?;
        let Ok(target) = self.target(record, claim).await else {
            return Ok(false);
        };
        let file = self.solve_file(record, claim).await?;
        Ok(target.digest == digest
            && file.agent_rounds.get(digest).copied().unwrap_or(0) == round
            && file.authorization.as_ref().is_some_and(|a| {
                a.active(digest, self.ports.clock.now()) && a.admitted_rounds.contains(&round)
            }))
    }
}

impl App {
    pub async fn formalization_generation_context(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        round: u32,
    ) -> CoreResult<Option<GenerationContext>> {
        caller.require(Role::Reviewer)?;
        Ok(self
            .solve_file(record, claim)
            .await?
            .authorization
            .and_then(|a| a.contexts.get(&round.to_string()).cloned()))
    }
    pub(crate) async fn check_public_generation_step(
        &self,
        caller: &Caller,
        submission: &Submission,
        step: &str,
    ) -> CoreResult<()> {
        if let Some(identity) = step.strip_prefix("agent-formalization:") {
            let parts: Vec<_> = identity.split(':').collect();
            let round = parts
                .get(2)
                .and_then(|r| r.strip_prefix("round"))
                .and_then(|r| r.parse().ok());
            let SubmissionStatus::Accepted { record } = &submission.status else {
                return Err(CoreError::conflict("public work inputs changed"));
            };
            if parts.len() != 3
                || round.is_none()
                || !self
                    .formalization_generation_active(
                        caller,
                        record,
                        &parts[0].into(),
                        parts[1],
                        round.unwrap(),
                    )
                    .await?
            {
                return Err(CoreError::conflict(
                    "public work authorization is paused, expired or changed",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct FormalizationWork {
    pub target_digest: String,
    pub total_generations: u32,
    pub used_generations: u32,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub paused: bool,
    pub state: &'static str,
    pub can_control: bool,
    pub managed: bool,
}
impl App {
    pub async fn formalization_work(
        &self,
        caller: Option<&Caller>,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<FormalizationWork> {
        let source = self.conjecture_source(record).await?;
        let target = self.target(record, claim).await?;
        let file = self.public_solve_file(record, claim).await?;
        let saved_authorization = file.authorization.as_ref();
        let authorization = saved_authorization.filter(|a| a.target_digest == target.digest);
        let state = match authorization {
            _ if file.winner(&target.digest).is_some() => "completed",
            None => "not_authorized",
            Some(a) if a.paused => "paused",
            Some(a) if self.ports.clock.now() >= a.expires_at => "expired",
            Some(a)
                if !a.can_admit(
                    &target.digest,
                    file.agent_rounds.get(&target.digest).copied().unwrap_or(0),
                    self.ports.clock.now(),
                ) =>
            {
                "allowance_used"
            }
            Some(_) => "active",
        };
        Ok(FormalizationWork {
            target_digest: target.digest,
            total_generations: saved_authorization.map_or(0, |a| a.total_generations),
            used_generations: saved_authorization.map_or(0, |a| a.admitted_rounds.len() as u32),
            expires_at: saved_authorization.map(|a| a.expires_at),
            paused: saved_authorization.is_some_and(|a| a.paused),
            state,
            can_control: caller.is_some_and(|c| c.person == source.submitter),
            managed: source.approved_statement(claim),
        })
    }
}
