//! Named, independently checked work uses the ordinary attempt/receipt aggregate.
use super::{App, solving::AttemptInput};
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId},
    model::*,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct CheckedContribution {
    pub origin: ProofReuse,
    pub intermediate: IntermediateProposition,
    pub authors: Vec<Author>,
    pub contributor: Entrant,
    pub attribution: Option<FormalizationAttribution>,
    pub source_sha256: String,
    pub receipt: VerificationReceipt,
    pub verification_request: VerificationRequest,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResultLink {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub title: String,
    pub label: String,
    pub authors: Vec<Author>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResultUse {
    pub result: ResultLink,
    pub used_result: ResultLink,
    pub contribution: IntermediateProposition,
    pub contributor: Entrant,
    pub attribution: Option<FormalizationAttribution>,
    pub attempt: String,
    pub source_attempt: String,
    pub basis: &'static str,
    /// All admitted source contributions, including any not reached by the proof.
    pub joint_context: Vec<ProofReuse>,
}

impl App {
    pub async fn submit_intermediate(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        input: NewIntermediate,
    ) -> CoreResult<AttemptView> {
        let spec = &input.intermediate;
        require_text("proposition title", &spec.title, 200)?;
        require_text("proposition", &spec.proposition, 20_000)?;
        require_text("license", &spec.license, 200)?;
        if !valid_module_name(&spec.name) {
            return Err(CoreError::invalid(
                "use a fully qualified Lean declaration name",
            ));
        }
        let target = self.target(record, claim).await?;
        if target.digest != input.expected_target {
            return Err(CoreError::conflict("the confirmed target changed"));
        }
        let (modules, attribution) = if let Some(id) = &input.deposit {
            if !input.modules.is_empty() {
                return Err(CoreError::invalid(
                    "a deposited project retains its original modules",
                ));
            }
            let d = self.lean_deposit(caller, id).await?;
            if d.owner != caller.person {
                return Err(CoreError::forbidden(
                    "only the depositor can submit this source",
                ));
            }
            if d.toolchain != target.toolchain || d.license != spec.license {
                return Err(CoreError::conflict(
                    "retain the deposit's toolchain and original license",
                ));
            }
            (
                self.lean_deposit_modules(caller, id).await?,
                Some(FormalizationAttribution {
                    formalizers: d.formalizers,
                    source_revision: d.source_revision,
                    deposit: id.clone(),
                    authors: d.authors,
                    source_reference: d.source_reference,
                    license: d.license,
                }),
            )
        } else {
            (input.modules, None)
        };
        // Target-free modules have the same meaning in later projects under the same pin.
        if modules.is_empty()
            || modules.values().any(|text| {
                text.split_whitespace()
                    .any(|token| token == "Target" || token.contains("wishpool_target_prop"))
            })
        {
            return Err(CoreError::invalid(
                "reusable propositions need self-contained modules without imports of Target",
            ));
        }
        let imports = modules
            .keys()
            .map(|name| format!("import {name}\n"))
            .collect::<String>();
        let solution = format!(
            "import Target\n{imports}theorem wishpool_solution : wishpool_target_prop := {}\n",
            spec.name
        );
        self.submit_attributed_attempt(
            caller,
            record,
            claim,
            AttemptInput {
                intermediate: Some(input.intermediate),
                reuses: vec![],
                expected_target: Some(input.expected_target),
                modules,
                solution,
                as_agent: input.as_agent,
                note: String::new(),
                attribution,
            },
        )
        .await
    }

    pub async fn checked_contribution(&self, id: &str) -> CoreResult<CheckedContribution> {
        let (file, attempt) = self.find_attempt(id).await?;
        let spec = attempt
            .intermediate
            .clone()
            .ok_or_else(|| CoreError::not_found("checked proposition", id))?;
        let source = self.conjecture_source(&file.record).await?;
        let target = self.target(&file.record, &file.claim).await?;
        if self.solve_file(&file.record, &file.claim).await?.id != file.id
            || attempt.target_digest != target.digest
            || !attempt.reuses.is_empty()
            || attempt.invalidated.is_some()
        {
            return Err(CoreError::not_found("checked proposition", id));
        }
        let receipt = attempt
            .receipt
            .clone()
            .filter(|r| r.verdict == Verdict::Proved)
            .ok_or_else(|| CoreError::not_found("checked proposition", id))?;
        let modules = self.lean_module_sources(&attempt.modules).await?;
        let solution = self.solution_text(&attempt).await?;
        let auxiliary_target = spec.target(&modules);
        if receipt.target_digest != lean_solution_digest(&auxiliary_target, &BTreeMap::new())
            || receipt.solution_digest != lean_solution_digest(&solution, &modules)
            || receipt.solution_digest != attempt.solution_digest
            || receipt.toolchain != target.toolchain
            || !receipt.used_constants.is_empty()
            || receipt
                .axioms
                .iter()
                .any(|a| !["propext", "Classical.choice", "Quot.sound"].contains(&a.as_str()))
        {
            return Err(CoreError::Unavailable(
                "checked proposition receipt scope is invalid".into(),
            ));
        }
        let current_source = self.conjecture_source(&file.record).await?;
        let current_target = self.target(&file.record, &file.claim).await?;
        if current_source.revision != source.revision || current_target.digest != target.digest {
            return Err(CoreError::conflict(
                "contribution source changed; reload the evidence",
            ));
        }
        let origin = ProofReuse {
            record: file.record,
            claim: file.claim,
            version: file.version,
            claims_revision: file.claims_revision,
            attempt: attempt.id.clone(),
            target_digest: attempt.target_digest,
            solution_digest: attempt.solution_digest,
            declaration: spec.name.clone(),
        };
        let source_sha256 = source
            .current_version()
            .ok_or_else(|| CoreError::conflict("source unavailable"))?
            .archive
            .sha256
            .clone();
        Ok(CheckedContribution {
            origin,
            intermediate: spec.clone(),
            authors: source.authors,
            contributor: self.current_entrant(&attempt.entrant).await?,
            attribution: attempt.attribution,
            source_sha256,
            verification_request: VerificationRequest {
                required_declaration: Some(spec.name.clone()),
                modules,
                target: auxiliary_target,
                target_digest: receipt.target_digest.clone(),
                toolchain: receipt.toolchain.clone(),
                solution,
                inspect_constants: vec![],
            },
            receipt,
        })
    }

    pub async fn checked_contributions(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<Vec<CheckedContribution>> {
        self.target(record, claim).await?;
        let file = self.solve_file(record, claim).await?;
        let mut out = vec![];
        for attempt in file
            .attempts
            .iter()
            .filter(|a| a.intermediate.is_some() && a.verified())
        {
            match self.checked_contribution(&attempt.id).await {
                Ok(contribution) => out.push(contribution),
                Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(out)
    }

    /// Provider input uses the original entrant snapshot so a later profile
    /// rename cannot change admitted prompt bytes or invalidate a managed run.
    /// Public displays may still resolve the contributor's current profile.
    pub async fn checked_contribution_input(
        &self,
        caller: &Caller,
        id: &str,
    ) -> CoreResult<CheckedContribution> {
        caller.require(Role::Reviewer)?;
        let mut contribution = self.checked_contribution(id).await?;
        contribution.contributor = self.find_attempt(id).await?.1.entrant;
        Ok(contribution)
    }

    pub async fn submit_reusing_attempt(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        input: ReusingAttempt,
    ) -> CoreResult<AttemptView> {
        if input.contributions.is_empty() || input.contributions.len() > 8 {
            return Err(CoreError::invalid("select 1–8 checked propositions"));
        }
        let target = self.target(record, claim).await?;
        let mut modules = input.modules;
        let mut reuses: Vec<ProofReuse> = vec![];
        for id in input.contributions {
            let contribution = self.checked_contribution(&id).await?;
            if contribution.receipt.toolchain != target.toolchain {
                return Err(CoreError::conflict(
                    "reused propositions require the same pinned toolchain",
                ));
            }
            if reuses.contains(&contribution.origin) {
                return Err(CoreError::invalid("duplicate proposition"));
            }
            if reuses
                .iter()
                .any(|pin| pin.declaration == contribution.origin.declaration)
            {
                return Err(CoreError::conflict(
                    "select one original attribution for each named proposition",
                ));
            }
            for (name, source) in contribution.verification_request.modules {
                if modules
                    .get(&name)
                    .is_some_and(|existing| existing != &source)
                {
                    return Err(CoreError::conflict(
                        "a reused module must retain its exact source",
                    ));
                }
                modules.insert(name, source);
            }
            reuses.push(contribution.origin);
        }
        self.submit_attributed_attempt(
            caller,
            record,
            claim,
            AttemptInput {
                intermediate: None,
                reuses,
                expected_target: Some(input.expected_target),
                modules,
                solution: input.solution,
                as_agent: input.as_agent,
                note: input.note,
                attribution: None,
            },
        )
        .await
    }

    pub(crate) async fn reuse_inputs_public(&self, attempt: &Attempt) -> CoreResult<bool> {
        for pin in &attempt.reuses {
            match self.checked_contribution(&pin.attempt).await {
                Ok(source) if &source.origin == pin => {}
                Ok(_) | Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => {
                    return Ok(false);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }

    pub(crate) async fn public_solve_file(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<SolveFile> {
        let mut file = self.solve_file(record, claim).await?;
        let mut attempts = vec![];
        for attempt in file.attempts {
            if self.reuse_inputs_public(&attempt).await? {
                attempts.push(attempt);
            }
        }
        file.attempts = attempts;
        Ok(file)
    }

    async fn result_link(
        &self,
        record: &RecordId,
        claim: &ClaimId,
        version: u32,
        claims_revision: u64,
    ) -> CoreResult<ResultLink> {
        let current = self.conjecture_source(record).await?;
        let publication = self
            .ports
            .records
            .get(record)
            .await?
            .ok_or_else(|| CoreError::not_found("paper", record.as_str()))?;
        let source = publication
            .publication_input(&current)
            .ok_or_else(|| CoreError::not_found("result", claim.as_str()))?;
        // Proof pins must match both live eligibility and the immutable result
        // page. A later acceptance can reuse the record id, but not its identity.
        if [&current, source].iter().any(|s| {
            s.current_version().map(|v| v.number) != Some(version)
                || s.claims_revision != claims_revision
        }) {
            return Err(CoreError::conflict(
                "result source changed; reload the evidence",
            ));
        }
        let statement = source
            .claim(claim)
            .ok_or_else(|| CoreError::not_found("result", claim.as_str()))?;
        Ok(ResultLink {
            record: record.clone(),
            claim: claim.clone(),
            version,
            claims_revision,
            title: source.title.clone(),
            label: statement.label.clone(),
            authors: source.authors.clone(),
        })
    }

    /// Both directions are derived from actual receipts. Citations and imports never enter here.
    pub async fn result_uses(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<Vec<ResultUse>> {
        self.public_statement(record, claim).await?;
        let mut out = vec![];
        for file in self.ports.solving.all().await? {
            if !(file.record == *record && file.claim == *claim
                || file.attempts.iter().any(|attempt| {
                    attempt
                        .reuses
                        .iter()
                        .any(|pin| pin.record == *record && pin.claim == *claim)
                }))
            {
                continue;
            }
            if !self
                .solve_file(&file.record, &file.claim)
                .await
                .is_ok_and(|current| current.id == file.id)
            {
                continue;
            }
            let proofs = match self.public_attempts(&file.record, &file.claim).await {
                Ok(proofs) => proofs,
                Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => continue,
                Err(error) => return Err(error),
            };
            for proof in proofs
                .into_iter()
                .filter(|a| a.receipt.verdict == Verdict::Proved)
            {
                for pin in &proof.reuses {
                    if (pin.record == file.record && pin.claim == file.claim)
                        || !proof.receipt.used_constants.contains(&pin.declaration)
                        || !((file.record == *record && file.claim == *claim)
                            || (pin.record == *record && pin.claim == *claim))
                    {
                        continue;
                    }
                    let edge = async {
                        let source = self.checked_contribution(&pin.attempt).await?;
                        Ok(ResultUse {
                            result: self
                                .result_link(
                                    &file.record,
                                    &file.claim,
                                    file.version,
                                    file.claims_revision,
                                )
                                .await?,
                            used_result: self
                                .result_link(
                                    &pin.record,
                                    &pin.claim,
                                    pin.version,
                                    pin.claims_revision,
                                )
                                .await?,
                            contribution: source.intermediate,
                            contributor: source.contributor,
                            attribution: source.attribution,
                            attempt: proof.id.clone(),
                            source_attempt: pin.attempt.clone(),
                            basis: "checked_proof_term",
                            joint_context: proof.reuses.clone(),
                        })
                    }
                    .await;
                    match edge {
                        Ok(edge) => out.push(edge),
                        Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => {}
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        Ok(out)
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_venue_project(
        &self,
        caller: &Caller,
        record: &RecordId,
        claim: &ClaimId,
        digest: &str,
        solution: String,
        modules: BTreeMap<String, String>,
        agent: String,
    ) -> CoreResult<AttemptView> {
        self.submit_attributed_attempt(
            caller,
            record,
            claim,
            AttemptInput {
                intermediate: None,
                reuses: vec![],
                expected_target: Some(digest.into()),
                modules,
                solution,
                as_agent: Some(agent),
                note: String::new(),
                attribution: None,
            },
        )
        .await
    }
}

impl App {
    pub(crate) async fn intermediate_credits(
        &self,
        entrant: &str,
    ) -> CoreResult<Vec<IntermediateCredit>> {
        let mut out = vec![];
        for file in self.ports.solving.all().await? {
            for attempt in file
                .attempts
                .iter()
                .filter(|a| a.entrant.id == entrant && a.intermediate.is_some() && a.verified())
            {
                match self.checked_contribution(&attempt.id).await {
                    Ok(c) => out.push(IntermediateCredit {
                        source_title: self.conjecture_source(&c.origin.record).await?.title,
                        source_authors: c.authors,
                        origin: c.origin,
                        intermediate: c.intermediate,
                        attribution: c.attribution,
                        receipt: c.receipt,
                    }),
                    Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        out.sort_by_key(|c| std::cmp::Reverse(c.receipt.checked_at));
        Ok(out)
    }
}
