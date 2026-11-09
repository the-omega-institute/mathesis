//! Collected results share proof attempts and receipts with ordinary publications.
//! They never become synthetic submissions or acquire a venue referee round.
use super::{App, solving::AttemptInput};
use crate::{CoreError, CoreResult, ids::*, model::*, ports::JobKind};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) struct ProofVersion {
    pub number: u32,
    pub archive: BlobRef,
}
pub(crate) struct ProofSource {
    pub title: String,
    pub authors: Vec<Author>,
    pub submitter: PersonId,
    pub revision: u64,
    pub claims_revision: u64,
    pub claims: Vec<Claim>,
    version: ProofVersion,
    approved: Vec<ClaimId>,
}
impl ProofSource {
    pub fn current_version(&self) -> Option<&ProofVersion> {
        Some(&self.version)
    }
    pub fn claim(&self, id: &ClaimId) -> Option<&Claim> {
        self.claims.iter().find(|c| &c.id == id)
    }
    pub fn approved_statement(&self, id: &ClaimId) -> bool {
        self.approved.contains(id)
    }
    pub fn involves(&self, person: &PersonId) -> bool {
        &self.submitter == person
            || self
                .authors
                .iter()
                .any(|a| a.person.as_ref() == Some(person))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectedProofView {
    pub claim: ClaimId,
    pub version: u32,
    pub state: &'static str,
    pub target_digest: Option<String>,
    pub reading: Option<String>,
    pub next_step: String,
    pub proofs: Vec<PublicAttempt>,
}

fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn file_id(work: &str, version: u32, claim: &ClaimId) -> String {
    format!("{work}:{claim}:v{version}:c0")
}

impl App {
    pub(crate) async fn conjecture_source(&self, record: &RecordId) -> CoreResult<ProofSource> {
        if record.as_str().starts_with("WP-") {
            let s = self.submission_proof_source(record).await?;
            let v = s
                .current_version()
                .ok_or_else(|| CoreError::conflict("source unavailable"))?;
            return Ok(ProofSource {
                version: ProofVersion {
                    number: v.number,
                    archive: v.archive.clone(),
                },
                approved: s
                    .claims
                    .iter()
                    .filter(|c| s.approved_statement(&c.id))
                    .map(|c| c.id.clone())
                    .collect(),
                title: s.title,
                authors: s.authors,
                submitter: s.submitter,
                revision: s.revision,
                claims_revision: s.claims_revision,
                claims: s.claims,
            });
        }
        let s = self
            .ports
            .collected
            .get(record.as_str(), None)
            .await?
            .ok_or_else(|| CoreError::not_found("paper", record.as_str()))?;
        let index = s
            .source_index
            .as_ref()
            .filter(|i| i.source_sha256 == s.source.sha256)
            .and_then(|i| i.results.as_ref())
            .ok_or_else(|| CoreError::conflict("source statements have not been indexed"))?;
        let claims: Vec<_> = index
            .statements
            .iter()
            .map(|c| Claim {
                id: c.id.clone().into(),
                kind: c.kind,
                label: c.name.clone(),
                latex_label: None,
                statement: c.statement.clone(),
                role: ClaimRole::Supporting,
                has_proof: false,
                section: c.section.clone(),
                depends_on: vec![],
                settles: None,
            })
            .collect();
        Ok(ProofSource {
            title: s.delivery.paper.title,
            authors: s.delivery.paper.authors,
            submitter: s.owner,
            revision: u64::from(s.delivery.version),
            claims_revision: 0,
            version: ProofVersion {
                number: s.delivery.version,
                archive: s.source,
            },
            approved: claims
                .iter()
                .filter(|c| !c.kind.is_open())
                .map(|c| c.id.clone())
                .collect(),
            claims,
        })
    }

    pub(crate) async fn collected_solve_file(
        &self,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<SolveFile> {
        let source = self.conjecture_source(record).await?;
        let statement = source
            .claim(claim)
            .ok_or_else(|| CoreError::not_found("result", claim.as_str()))?;
        let v = source.current_version().unwrap();
        let file = self
            .ports
            .solving
            .get(&file_id(record.as_str(), v.number, claim))
            .await?
            .ok_or_else(|| CoreError::not_found("formalization", claim.as_str()))?;
        let pin = file
            .collected
            .as_ref()
            .ok_or_else(|| CoreError::conflict("not a collected result"))?;
        if pin.source_sha256 != v.archive.sha256
            || pin.statement_sha256 != hash(&statement.statement)
        {
            return Err(CoreError::conflict("collected result inputs changed"));
        }
        Ok(file)
    }

    /// Explicit operator request: existing evidence first, then fill this exact gap.
    pub async fn request_collected_proof(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        claim: &ClaimId,
    ) -> CoreResult<()> {
        let saved = self
            .ports
            .collected
            .get(work, None)
            .await?
            .ok_or_else(|| CoreError::not_found("paper", work))?;
        if !self.is_internal_account(&caller.person)
            || (saved.owner != caller.person && !caller.has(Role::Admin))
        {
            return Err(CoreError::forbidden(
                "this collection's authorized operator is required",
            ));
        }
        if version != saved.delivery.version {
            return Err(CoreError::conflict(
                "request proof work against the current paper version",
            ));
        }
        self.conjecture_source(&work.into())
            .await?
            .claim(claim)
            .filter(|c| !c.kind.is_open())
            .ok_or_else(|| CoreError::invalid("select an indexed proved result"))?;
        self.enqueue_collected_formalizations(&saved).await
    }

    async fn enqueue_collected_result(
        &self,
        saved: &CollectedVersion,
        claim: &ClaimId,
    ) -> CoreResult<()> {
        let work = saved.delivery.work_id.as_str();
        let version = saved.delivery.version;
        if self
            .ports
            .collected
            .get(work, None)
            .await?
            .is_none_or(|p| p.delivery.version != version)
        {
            return Ok(());
        }
        let source = self.conjecture_source(&work.into()).await?;
        let c = source
            .claim(claim)
            .filter(|c| !c.kind.is_open())
            .ok_or_else(|| CoreError::invalid("select an indexed proved result"))?;
        let id = file_id(work, version, claim);
        if self.ports.solving.get(&id).await?.is_none() {
            let file = SolveFile {
                collected: Some(CollectedProof {
                    source_sha256: saved.source.sha256.clone(),
                    statement_sha256: hash(&c.statement),
                    requested_by: saved.owner.clone(),
                    target: None,
                    proposal: None,
                    faithfulness: None,
                    blocked: None,
                    runs: BTreeMap::new(),
                }),
                id: id.clone(),
                record: work.into(),
                submission: format!("collected:{work}:{version}:{claim}").into(),
                claim: claim.clone(),
                version,
                claims_revision: 0,
                revision: 0,
                admitted: false,
                referee_backend: RefereeBackend::Cma,
                authorization: None,
                agent_attempted: Default::default(),
                agent_rounds: Default::default(),
                agent_after_attempt: Default::default(),
                failure: None,
                review_round: 0,
                review_contract: 0,
                task: None,
                report: None,
                audit: None,
                attempts: vec![],
                dependencies: vec![],
                downstream: 0,
            };
            if let Err(error) = self.ports.solving.insert(&file).await
                && self.ports.solving.get(&id).await?.is_none()
            {
                return Err(error);
            }
        }
        Ok(())
    }

    /// Internal publications are already authorized for venue proof work. No paper referee.
    pub(crate) async fn enqueue_collected_formalizations(
        &self,
        saved: &CollectedVersion,
    ) -> CoreResult<()> {
        if !self.is_internal_account(&saved.owner) {
            return Ok(());
        }
        let Some(results) = saved
            .source_index
            .as_ref()
            .filter(|i| i.source_sha256 == saved.source.sha256)
            .and_then(|i| i.results.as_ref())
        else {
            return Ok(());
        };
        for statement in results.statements.iter().filter(|s| !s.kind.is_open()) {
            self.enqueue_collected_result(saved, &statement.id.clone().into())
                .await?;
        }
        if !results.statements.iter().any(|s| !s.kind.is_open()) {
            return Ok(());
        }
        // One durable paper job shares source inspection and target proposals.
        // Individual statements still receive separate kernel receipts and credits.
        self.ports
            .queue
            .enqueue(
                &format!(
                    "collected:{}:{}",
                    saved.delivery.work_id, saved.delivery.version
                )
                .into(),
                JobKind::CollectedFormalization,
            )
            .await
    }

    pub(crate) async fn reconcile_collected_proofs(&self) -> CoreResult<()> {
        for file in self.ports.solving.all().await? {
            let Some(proof) = &file.collected else {
                continue;
            };
            let cleanup = proof.runs.values().any(|r| {
                r["deleted"] != true && (r["agent_id"].is_string() || r["create_intent"].is_array())
            });
            let current = self
                .collected_solve_file(&file.record, &file.claim)
                .await
                .is_ok_and(|f| f.id == file.id);
            let completed = proof
                .target
                .as_ref()
                .is_some_and(|t| file.winner(&t.digest).is_some());
            if cleanup || (current && !completed && proof.blocked.is_none()) {
                self.ports
                    .queue
                    .enqueue(
                        &format!("collected:{}:{}", file.record, file.version).into(),
                        JobKind::CollectedFormalization,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// Composition-only state, including historical runs so stale-version cleanup can finish.
    pub async fn collected_proof_file(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        claim: &ClaimId,
    ) -> CoreResult<SolveFile> {
        caller.require(Role::Reviewer)?;
        self.ports
            .solving
            .get(&file_id(work, version, claim))
            .await?
            .ok_or_else(|| CoreError::not_found("formalization", claim.as_str()))
    }
    #[allow(clippy::too_many_arguments)] // Exact publication identity plus a CAS-protected run.
    pub async fn save_collected_proof_run(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        claim: &ClaimId,
        step: &str,
        value: serde_json::Value,
        expected: u64,
    ) -> CoreResult<u64> {
        let mut file = self
            .collected_proof_file(caller, work, version, claim)
            .await?;
        if file.revision != expected {
            return Err(CoreError::StaleRevision {
                kind: "collected proof",
                id: file.id,
            });
        }
        file.collected
            .as_mut()
            .ok_or_else(|| CoreError::conflict("not a collected result"))?
            .runs
            .insert(step.into(), value);
        self.save_solve_file(&mut file).await?;
        Ok(file.revision)
    }
    pub async fn collected_proof_active(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        claim: &ClaimId,
    ) -> CoreResult<bool> {
        caller.require(Role::Reviewer)?;
        match self.collected_solve_file(&work.into(), claim).await {
            Ok(f) => Ok(f.version == version),
            Err(CoreError::Conflict(_) | CoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub async fn pause_collected_proof(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        reason: &str,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        require_text("formalization next step", reason, 1000)?;
        let mut file = self.collected_solve_file(&work.into(), claim).await?;
        file.collected.as_mut().unwrap().blocked = Some(reason.into());
        self.save_solve_file(&mut file).await
    }

    pub async fn save_collected_proposal(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        proposal: CollectedProofProposal,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        require_text("Lean target", &proposal.lean, 200_000)?;
        require_text("target reading", &proposal.reading, 20_000)?;
        require_text("toolchain", &proposal.toolchain, 200)?;
        require_text("proof license", &proposal.license, 200)?;
        validate_lean_modules(&proposal.solution, &proposal.modules)?;
        if proposal.formalizers.len() > 50
            || proposal
                .formalizers
                .iter()
                .any(|n| n.len() > 200 || n.trim().is_empty())
        {
            return Err(CoreError::invalid(
                "supply bounded original formalizer names",
            ));
        }
        let mut file = self.collected_solve_file(&work.into(), claim).await?;
        let proof = file.collected.as_mut().unwrap();
        if proof.proposal.as_ref().is_some_and(|p| p != &proposal) {
            return Err(CoreError::conflict(
                "proposal is already fixed for this run",
            ));
        }
        if let Some(origin) = &proposal.reused_from {
            let publication = self
                .ports
                .collected
                .get(work, Some(file.version))
                .await?
                .unwrap();
            if publication.delivery.paper.lean.as_ref() != Some(origin) {
                return Err(CoreError::invalid(
                    "reuse must retain this paper's pinned Lean origin and original formalizers",
                ));
            }
        }
        proof.proposal = Some(proposal);
        self.save_solve_file(&mut file).await
    }

    /// Separate semantic inspection precedes kernel elaboration. The tautology checks
    /// that the exact proposition is well typed; it is never recorded as a proof of it.
    pub async fn admit_collected_target(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        audit: FaithfulnessCheck,
    ) -> CoreResult<()> {
        self.admit_collected_target_inner(caller, work, claim, audit, None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn admit_collected_target_leased(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        audit: FaithfulnessCheck,
        job: &crate::ports::LeasedJob,
        lease: &dyn crate::ports::JobLease,
    ) -> CoreResult<()> {
        self.admit_collected_target_inner(caller, work, claim, audit, Some((job, lease)))
            .await
    }

    async fn admit_collected_target_inner(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        audit: FaithfulnessCheck,
        lease: Option<(&crate::ports::LeasedJob, &dyn crate::ports::JobLease)>,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        let mut file = self.collected_solve_file(&work.into(), claim).await?;
        let proof = file.collected.as_mut().unwrap();
        if proof.target.is_some() {
            return Ok(());
        }
        require_text("faithfulness reason", &audit.reason, 5000)?;
        let proposal = proof
            .proposal
            .clone()
            .ok_or_else(|| CoreError::conflict("no exact target proposal"))?;
        if audit.verdict != FaithfulnessVerdict::Faithful {
            proof.blocked = Some(
                "The proposed Lean statement needs correction before proof work continues.".into(),
            );
            proof.faithfulness = Some(audit);
            return self.save_solve_file(&mut file).await;
        }
        let marker = "def wishpool_target_prop : Prop :=";
        if !proposal.lean.starts_with("import Mathlib\n")
            || proposal.lean.matches(marker).count() != 1
        {
            return Err(CoreError::invalid(
                "supply one complete Target.lean proposition",
            ));
        }
        let (definitions, proposition) = proposal.lean.split_once(marker).unwrap();
        let target =
            format!("{definitions}{marker}\n let p : Prop := (\n{proposition}\n)\n p → p\n");
        let solution =
            "import Target\ntheorem wishpool_solution : wishpool_target_prop := fun h => h\n"
                .to_string();
        let request = VerificationRequest {
            required_declaration: None,
            inspect_constants: vec![],
            modules: BTreeMap::new(),
            target_digest: hash(&target),
            target,
            toolchain: proposal.toolchain.clone(),
            solution,
        };
        let receipt = self.ports.verifier.verify(&request).await?;
        if let Some((job, lease)) = lease
            && !lease.renew(job).await?
        {
            return Err(CoreError::conflict("formalization job lease was lost"));
        }
        if receipt.verdict != Verdict::Proved
            || receipt.target_digest != request.target_digest
            || receipt.solution_digest != hash(&request.solution)
            || receipt.toolchain != request.toolchain
            || !receipt.used_constants.is_empty()
            || receipt
                .axioms
                .iter()
                .any(|a| !["propext", "Classical.choice", "Quot.sound"].contains(&a.as_str()))
        {
            return Err(CoreError::invalid(format!(
                "target elaboration failed: {}",
                receipt.reason
            )));
        }
        // Re-read after I/O: a replacement source/proposal cannot inherit this check.
        let current = self.collected_solve_file(&work.into(), claim).await?;
        if current.id != file.id
            || current.collected.as_ref().and_then(|p| p.proposal.as_ref()) != Some(&proposal)
        {
            return Err(CoreError::conflict("formalization inputs changed"));
        }
        file = current;
        let proof = file.collected.as_mut().unwrap();
        proof.target = Some(CollectedTarget {
            digest: hash(&proposal.lean),
            lean: proposal.lean,
            toolchain: proposal.toolchain,
            reading: proposal.reading,
            confirmed_by: proof.requested_by.clone(),
            confirmed_at: self.ports.clock.now(),
        });
        proof.faithfulness = Some(audit);
        file.admitted = true;
        self.save_solve_file(&mut file).await
    }

    pub async fn submit_collected_proposal(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
    ) -> CoreResult<Option<AttemptView>> {
        caller.require(Role::Reviewer)?;
        let file = self.collected_solve_file(&work.into(), claim).await?;
        let proposal = file
            .collected
            .as_ref()
            .and_then(|p| p.proposal.as_ref())
            .ok_or_else(|| CoreError::conflict("no proof proposal"))?;
        if proposal.solution.trim().is_empty() {
            return Ok(None);
        }
        let (owner, agent) = self.venue_agent(caller).await?;
        let source = self.conjecture_source(&work.into()).await?;
        let target = self.target(&work.into(), claim).await?;
        let attribution = proposal
            .reused_from
            .as_ref()
            .map(|origin| FormalizationAttribution {
                formalizers: proposal.formalizers.clone(),
                source_revision: Some(origin.revision.clone()),
                deposit: String::new(),
                authors: source.authors.iter().map(|a| a.name.clone()).collect(),
                source_reference: Source {
                    kind: SourceKind::Url,
                    locator: origin.url.clone(),
                    year: None,
                },
                license: proposal.license.clone(),
            });
        self.submit_attributed_attempt(
            &owner,
            &work.into(),
            claim,
            AttemptInput {
                intermediate: None,
                reuses: vec![],
                modules: proposal.modules.clone(),
                expected_target: Some(target.digest),
                solution: proposal.solution.clone(),
                as_agent: Some(agent.id),
                note: if attribution.is_some() {
                    "Rechecked existing published formalization; no new proof credit."
                } else {
                    "Mathesis formalization of the published result."
                }
                .into(),
                attribution,
            },
        )
        .await
        .map(Some)
    }

    pub async fn continue_collected_proof(
        &self,
        caller: &Caller,
        work: &str,
        claim: &ClaimId,
        expected_solution: &str,
        solution: String,
        modules: BTreeMap<String, String>,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        validate_lean_modules(&solution, &modules)?;
        let mut file = self.collected_solve_file(&work.into(), claim).await?;
        let proof = file.collected.as_mut().unwrap();
        let proposal = proof
            .proposal
            .as_mut()
            .ok_or_else(|| CoreError::conflict("proof proposal unavailable"))?;
        let digest = lean_solution_digest(&solution, &modules);
        if lean_solution_digest(&proposal.solution, &proposal.modules) != expected_solution {
            return Err(CoreError::conflict("proof continuation was superseded"));
        }
        if solution.trim().is_empty() || file.attempts.iter().any(|a| a.solution_digest == digest) {
            proof.blocked = Some("Automatic work has no new proof to check. The exact target remains open to contributors.".into());
        } else {
            proposal.solution = solution;
            proposal.modules = modules;
        }
        self.save_solve_file(&mut file).await
    }

    pub async fn collected_proofs(
        &self,
        work: &str,
        version: u32,
    ) -> CoreResult<Vec<CollectedProofView>> {
        self.ports
            .collected
            .get(work, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("paper version", version.to_string()))?;
        if self
            .ports
            .collected
            .get(work, None)
            .await?
            .is_none_or(|s| s.delivery.version != version)
        {
            return Ok(vec![]);
        }
        let source = self.conjecture_source(&work.into()).await?;
        if source.current_version().unwrap().number != version {
            return Ok(vec![]);
        }
        let mut out = vec![];
        for claim in &source.claims {
            let file = match self.collected_solve_file(&work.into(), &claim.id).await {
                Err(CoreError::NotFound { .. }) => continue,
                result => result?,
            };
            let p = file.collected.as_ref().unwrap();
            let proofs = if p.target.is_some() {
                self.public_attempts(&work.into(), &claim.id).await?
            } else {
                vec![]
            };
            let state = if proofs.iter().any(|p| p.receipt.verdict == Verdict::Proved) {
                "verified"
            } else if proofs
                .iter()
                .any(|p| p.receipt.verdict == Verdict::Disproved)
            {
                "disproved"
            } else if p.blocked.is_some() {
                "needs_attention"
            } else if p.target.is_some() {
                "open"
            } else {
                "preparing"
            };
            out.push(CollectedProofView {
                claim: claim.id.clone(),
                version,
                state,
                target_digest: p.target.as_ref().map(|t| t.digest.clone()),
                reading: p.target.as_ref().map(|t| t.reading.clone()),
                next_step: match state {
                    "verified" => "Read the checked proof and its credits.",
                    "disproved" => "A checked counterexample is available. The mathematical statement needs correction.",
                    "open" => "Contribute a proof of this exact statement.",
                    "needs_attention" => p
                        .blocked
                        .as_deref()
                        .unwrap_or("Further proof work is needed."),
                    _ => "Existing Lean evidence is being checked before new proof work.",
                }
                .into(),
                proofs,
            });
        }
        Ok(out)
    }
}
