//! Direct collection of curated open problems by a trusted operator. Nothing here
//! reviews, audits or admits: an entry records what a curator read and when.
use super::{App, problem_library::collected_problem_key};
use crate::{
    CoreError, CoreResult,
    ids::SubmissionId,
    model::*,
    ports::{JobKind, JobLease, LeasedJob},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct CollectedProblemReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub problem_id: String,
    pub version: u32,
    /// The lasting problem-library entry.
    pub research_problem: String,
    pub public_path: String,
    pub request_digest: String,
}

/// The public reading of a curated entry at its current version. It names no
/// operator, producer, slug or request digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCuratedProblem {
    pub name: String,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    pub posers: Vec<SourceContributor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posers_unknown: Option<String>,
    pub source: Source,
    /// The curator's reading, never a Mathesis audit result.
    pub status: CuratedStatus,
    /// The most recent day on which the curator checked the listed evidence.
    pub status_as_of: NaiveDate,
    pub status_evidence: Vec<StatusEvidence>,
    pub msc: Vec<String>,
    pub directions: Vec<String>,
    pub source_license: String,
    pub license: String,
    pub version: u32,
    pub published_on: DateTime<Utc>,
    /// The curator's Lean reading of the statement, when one was supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_target: Option<PublicCuratedTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicTargetState {
    /// The independent verifier has not read this exact text yet.
    Checking,
    Elaborated,
    Refused,
}

/// The collection operator's formalization as readers see it. The Lean text is
/// public only after the independent verifier elaborated it; elaboration shows a
/// well-formed proposition, never that it states the problem faithfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCuratedTarget {
    pub state: PublicTargetState,
    /// SHA-256 of the exact target text.
    pub digest: String,
    pub correspondence: String,
    pub vocabulary: TargetVocabulary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The verifier's pinned Lean and Mathlib, once it has read the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<DateTime<Utc>>,
    /// Why the verifier refused the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn public_target(v: &CollectedProblemVersion) -> Option<PublicCuratedTarget> {
    let target = v.delivery.problem.lean_target.as_ref()?;
    let digest = sha256_hex(&target.text);
    // A check of other bytes never speaks for this text.
    let check = v
        .target_check
        .as_ref()
        .filter(|check| check.target_digest == digest);
    let state = match check.map(|check| check.state) {
        None => PublicTargetState::Checking,
        Some(TargetCheckState::Elaborated) => PublicTargetState::Elaborated,
        Some(TargetCheckState::Refused) => PublicTargetState::Refused,
    };
    Some(PublicCuratedTarget {
        state,
        digest,
        correspondence: target.correspondence.clone(),
        vocabulary: target.vocabulary(),
        text: (state == PublicTargetState::Elaborated).then(|| target.text.clone()),
        toolchain: check.map(|check| check.toolchain.clone()),
        checked_at: check.map(|check| check.checked_at),
        reason: check
            .filter(|_| state == PublicTargetState::Refused)
            .map(|check| check.reason.clone()),
    })
}

/// The durable identity of one version's target check.
fn target_job(v: &CollectedProblemVersion) -> SubmissionId {
    SubmissionId(format!(
        "collected-problem:{}:{}",
        v.delivery.problem_id, v.delivery.version
    ))
}

impl From<&CollectedProblemVersion> for PublicCuratedProblem {
    fn from(v: &CollectedProblemVersion) -> Self {
        let p = &v.delivery.problem;
        Self {
            name: p.name.trim().to_owned(),
            statement: p.statement.clone(),
            background: p.background.clone(),
            posers: p.posers.clone(),
            posers_unknown: p.posers_unknown.clone(),
            source: Source {
                locator: p.source.locator.trim().to_owned(),
                ..p.source.clone()
            },
            status: p.status,
            status_as_of: p.status_as_of(),
            status_evidence: p.status_evidence.clone(),
            msc: p.msc.clone(),
            directions: p.directions.clone(),
            source_license: p.source_license.clone(),
            license: p.license.clone(),
            version: v.delivery.version,
            published_on: v.published_on,
            lean_target: public_target(v),
        }
    }
}

fn receipt(v: &CollectedProblemVersion) -> CollectedProblemReceipt {
    CollectedProblemReceipt {
        schema: "mathesis.collected-problem-receipt.v1",
        status: "collected",
        problem_id: v.delivery.problem_id.clone(),
        version: v.delivery.version,
        research_problem: v.key.clone(),
        public_path: format!("/problems/{}", v.key),
        request_digest: v.request_digest.clone(),
    }
}

impl App {
    /// Queues the independent reading of a version's Lean target. The queue keeps
    /// one unfinished job per version, so retries of a delivery do not multiply it.
    async fn queue_target_check(&self, v: &CollectedProblemVersion) -> CoreResult<()> {
        if v.delivery.problem.lean_target.is_some() && v.target_check.is_none() {
            self.ports
                .queue
                .enqueue(&target_job(v), JobKind::CollectedProblemTarget)
                .await?;
        }
        Ok(())
    }

    /// Worker-only: the independent verifier reads the exact Lean target of one
    /// curated problem version, and the outcome is attached to that version once.
    /// No proof of the statement is involved: the verifier checks `p → p` for the
    /// curator's proposition `p`.
    pub async fn check_collected_problem_target_leased(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        lease: &dyn JobLease,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        let version = job
            .submission
            .0
            .strip_prefix("collected-problem:")
            .and_then(|rest| rest.rsplit_once(':'))
            .and_then(|(problem, version)| Some((problem, version.parse::<u32>().ok()?)))
            .filter(|_| job.kind == JobKind::CollectedProblemTarget);
        let Some((problem, number)) = version else {
            return Err(CoreError::conflict("wrong curated target job"));
        };
        let store = &self.ports.collected_problems;
        let Some(v) = store.get(problem, Some(number)).await? else {
            return Err(CoreError::not_found(
                "collected problem version",
                job.submission.0.clone(),
            ));
        };
        let Some(target) = v.delivery.problem.lean_target.as_ref() else {
            return Ok(());
        };
        if v.target_check.is_some() {
            return Ok(());
        }
        let toolchain = self.ports.verifier.toolchain().await?;
        let (witness, solution) = target.elaboration_witness();
        let request = VerificationRequest {
            required_declaration: None,
            inspect_constants: vec![],
            modules: Default::default(),
            target_digest: sha256_hex(&witness),
            target: witness,
            toolchain,
            solution,
        };
        let receipt = self.ports.verifier.verify(&request).await?;
        if !lease.renew(job).await? {
            return Err(CoreError::conflict("curated target job lease was lost"));
        }
        if receipt.target_digest != request.target_digest
            || receipt.solution_digest != sha256_hex(&request.solution)
            || receipt.toolchain != request.toolchain
        {
            return Err(CoreError::Unavailable(
                "the verifier answered for different inputs".into(),
            ));
        }
        // A deadline says nothing about the text; the job runs again.
        if receipt.verdict != Verdict::Proved && receipt.reason == "verifier deadline exceeded" {
            return Err(CoreError::Unavailable(receipt.reason));
        }
        let elaborated = receipt.verdict == Verdict::Proved
            && receipt.used_constants.is_empty()
            && receipt.library_use.is_none()
            && receipt
                .axioms
                .iter()
                .all(|a| ["propext", "Classical.choice", "Quot.sound"].contains(&a.as_str()));
        let reason: String = if elaborated || receipt.verdict != Verdict::Proved {
            receipt.reason.chars().take(2000).collect()
        } else {
            "the elaboration witness depends on more than the standard axioms".into()
        };
        store
            .attach_target_check(
                problem,
                number,
                &CuratedTargetCheck {
                    state: if elaborated {
                        TargetCheckState::Elaborated
                    } else {
                        TargetCheckState::Refused
                    },
                    target_digest: sha256_hex(&target.text),
                    toolchain: request.toolchain,
                    reason,
                    checked_at: receipt.checked_at,
                },
            )
            .await
    }

    /// Publishes one immutable version of a curated problem entry. The operator's
    /// trust covers the curation only; no record, review, proof job or credit
    /// follows. A supplied Lean target is the curator's own formalization.
    pub async fn collect_problem(
        &self,
        caller: &Caller,
        delivery: CollectedProblemDelivery,
    ) -> CoreResult<CollectedProblemReceipt> {
        if !self.is_internal_account(&caller.person) {
            return Err(CoreError::forbidden("trusted collection operator required"));
        }
        delivery.validate()?;
        // A curator anywhere on Earth is at most one calendar day ahead of UTC.
        let latest = self.ports.clock.now().date_naive() + chrono::Days::new(1);
        if delivery.problem.status_as_of() > latest {
            return Err(CoreError::invalid(
                "status evidence cannot be checked on a future date",
            ));
        }
        let store = &self.ports.collected_problems;
        let key = collected_problem_key(
            &caller.person,
            &delivery.producer,
            &delivery.problem.source,
            &delivery.problem.name,
        );
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&delivery).expect("serializable delivery"))
        );
        if let Some(current) = store.get(&delivery.problem_id, None).await? {
            if current.owner != caller.person || current.delivery.producer != delivery.producer {
                return Err(CoreError::conflict(
                    "problem identity belongs to a different collection operator",
                ));
            }
            if let Some(existing) = store
                .get(&delivery.problem_id, Some(delivery.version))
                .await?
            {
                if existing.request_digest != digest {
                    return Err(CoreError::conflict(
                        "published versions are immutable; deliver a new version",
                    ));
                }
                self.queue_target_check(&existing).await?;
                return Ok(receipt(&existing));
            }
            if delivery.previous_version != Some(current.delivery.version) {
                return Err(CoreError::conflict(
                    "name the current published predecessor",
                ));
            }
            if current.key != key {
                return Err(CoreError::conflict(
                    "a collected problem keeps its name and source; collect a renamed or re-sourced problem under a new problem_id",
                ));
            }
        } else {
            if delivery.previous_version.is_some() {
                return Err(CoreError::conflict(
                    "first collected version cannot name a predecessor",
                ));
            }
            if store.get_by_key(&key).await?.is_some() {
                return Err(CoreError::conflict(
                    "this named source question is already collected under another problem_id",
                ));
            }
        }
        let version = CollectedProblemVersion {
            id: uuid::Uuid::now_v7().to_string(),
            owner: caller.person.clone(),
            key,
            delivery,
            request_digest: digest,
            published_on: self.ports.clock.now(),
            target_check: None,
        };
        if let Err(error) = store.insert(&version).await {
            // A uniqueness conflict means a concurrent delivery won. Only the same
            // bytes from the same operator share its receipt.
            if matches!(error, CoreError::Conflict(_))
                && let Some(saved) = store
                    .get(&version.delivery.problem_id, Some(version.delivery.version))
                    .await?
                && saved.owner == version.owner
                && saved.request_digest == version.request_digest
            {
                self.queue_target_check(&saved).await?;
                return Ok(receipt(&saved));
            }
            return Err(error);
        }
        self.queue_target_check(&version).await?;
        Ok(receipt(&version))
    }
}
