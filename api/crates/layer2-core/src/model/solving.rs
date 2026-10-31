//! Solving aggregates and strictly public identity/receipt types.
use super::{BlobRef, RefereeAudit, RefereeReport};
use crate::ids::{ClaimId, PersonId, RecordId, SubmissionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrantKind {
    Person,
    Agent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrant {
    pub id: String,
    pub name: String,
    pub kind: EntrantKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    pub retired: bool,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub id: PersonId,
    pub name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Proved,
    Disproved,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    /// Constants reached through the elaborated proof value, never imports or type names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_constants: Vec<String>,
    pub verdict: Verdict,
    pub reason: String,
    pub target_digest: String,
    pub solution_digest: String,
    pub toolchain: String,
    pub axioms: Vec<String>,
    pub checked_at: DateTime<Utc>,
    pub duration: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_declaration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inspect_constants: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    pub target: String,
    pub target_digest: String,
    pub toolchain: String,
    pub solution: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<IntermediateProposition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, BlobRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub owner: PersonId,
    pub entrant: Entrant,
    pub target_digest: String,
    pub solution_digest: String,
    pub solution: BlobRef,
    pub note: String,
    pub submitted_at: DateTime<Utc>,
    pub receipt: Option<VerificationReceipt>,
    #[serde(default)]
    pub invalidated: Option<String>,
}
impl Attempt {
    pub fn verified(&self) -> bool {
        self.receipt
            .as_ref()
            .is_some_and(|r| r.verdict != Verdict::Rejected)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveFile {
    /// Explicit, target-bound lifetime allowance; never replenished by retry/resume.
    #[serde(default)]
    pub authorization: Option<FormalizationAuthorization>,
    #[serde(default)]
    pub agent_attempted: std::collections::BTreeSet<String>,
    /// Number of durable CMA continuation rounds per confirmed target. This
    /// lets a formalization resume after a failed proof instead of replaying
    /// one frozen response from the first run.
    #[serde(default)]
    pub agent_rounds: std::collections::BTreeMap<String, u32>,
    /// Last rejected attempt already consumed by a continuation.
    #[serde(default)]
    pub agent_after_attempt: std::collections::BTreeMap<String, String>,
    pub id: String,
    pub record: RecordId,
    pub submission: SubmissionId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub revision: u64,
    /// Paper-derived candidates need their own sanitized conjecture audit.
    pub admitted: bool,
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub review_round: u32,
    #[serde(default)]
    pub review_contract: u32,
    pub task: Option<String>,
    pub report: Option<RefereeReport>,
    pub audit: Option<RefereeAudit>,
    pub attempts: Vec<Attempt>,
    /// Confirmed S1 edges only. No model-inferred relationships.
    pub dependencies: Vec<DependencyEdge>,
    pub downstream: usize,
}
impl SolveFile {
    pub fn winner(&self, digest: &str) -> Option<&Attempt> {
        self.attempts
            .iter()
            .filter(|a| a.intermediate.is_none() && a.target_digest == digest && a.verified())
            .min_by_key(|a| (a.receipt.as_ref().unwrap().checked_at, &a.id))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicAttempt {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub entrant: Entrant,
    pub receipt: VerificationReceipt,
    pub solution: String,
    pub also_verified: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct AttemptView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<IntermediateProposition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub record: RecordId,
    pub claim: ClaimId,
    pub entrant: Entrant,
    pub state: String,
    pub reason: Option<String>,
    pub receipt: Option<VerificationReceipt>,
    pub solution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardRow {
    pub rank: usize,
    pub entrant: Entrant,
    pub solved: usize,
    pub disproved: usize,
    pub score: usize,
    pub others: usize,
    pub total: usize,
    pub last_solve: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SolveCredit {
    pub others: bool,
    pub formalized: bool,
    pub record: RecordId,
    pub claim: ClaimId,
    pub title: String,
    pub attempt: String,
    pub receipt: VerificationReceipt,
}
#[derive(Debug, Clone, Serialize)]
pub struct EntrantProfile {
    pub contributions: Vec<IntermediateCredit>,
    pub entrant: Entrant,
    pub solutions: Vec<SolveCredit>,
    pub others: usize,
    pub total: usize,
    pub formalized: Vec<SolveCredit>,
    pub errors_found: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveNotification {
    pub attempt: String,
    pub record: RecordId,
    pub claim: ClaimId,
    pub entrant: Entrant,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConjectureRef {
    pub record: RecordId,
    pub claim: ClaimId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedDependency {
    pub from: ClaimId,
    pub target: ConjectureRef,
    pub target_version: u32,
    pub target_claims_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationAuthorization {
    pub target_digest: String,
    pub authorized_by: PersonId,
    pub total_generations: u32,
    pub expires_at: DateTime<Utc>,
    pub paused: bool,
    /// Round identities are also the durable managed-provider request identities.
    pub admitted_rounds: std::collections::BTreeSet<u32>,
    #[serde(default)]
    pub contexts: std::collections::BTreeMap<String, GenerationContext>,
}
impl FormalizationAuthorization {
    pub fn active(&self, digest: &str, now: DateTime<Utc>) -> bool {
        !self.paused && self.target_digest == digest && now < self.expires_at
    }
    pub fn can_admit(&self, digest: &str, round: u32, now: DateTime<Utc>) -> bool {
        self.active(digest, now)
            && (self.admitted_rounds.contains(&round)
                || self.admitted_rounds.len() < self.total_generations as usize)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalizationControl {
    pub target_digest: String,
    pub total_generations: u32,
    pub expires_at: DateTime<Utc>,
    pub paused: bool,
    /// Explicit authorization to change saved limits or rebind a corrected target.
    /// Pause/resume alone cannot extend limits or authorize a replacement target.
    #[serde(default)]
    pub change_allowance: bool,
}

/// Proposed mathematical content. Only an ordinary independent receipt establishes its proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntermediateProposition {
    pub name: String,
    pub title: String,
    pub proposition: String,
    pub license: String,
}
impl IntermediateProposition {
    pub fn target<T>(&self, modules: &std::collections::BTreeMap<String, T>) -> String {
        let imports = modules
            .keys()
            .map(|name| format!("import {name}\n"))
            .collect::<String>();
        format!(
            "import Mathlib\n{imports}def wishpool_target_prop : Prop := {}\n",
            self.proposition
        )
    }
}
/// Immutable references to original checked work. No new credit event is created by reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofReuse {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub attempt: String,
    pub target_digest: String,
    pub solution_digest: String,
    pub declaration: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewIntermediate {
    pub expected_target: String,
    pub intermediate: IntermediateProposition,
    #[serde(default)]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub deposit: Option<String>,
    #[serde(default)]
    pub as_agent: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReusingAttempt {
    pub expected_target: String,
    pub solution: String,
    pub contributions: Vec<String>,
    #[serde(default)]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub as_agent: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationContext {
    pub contract: u32,
    pub contributions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntermediateCredit {
    pub origin: ProofReuse,
    pub intermediate: IntermediateProposition,
    pub source_title: String,
    pub source_authors: Vec<super::Author>,
    pub attribution: Option<super::FormalizationAttribution>,
    pub receipt: VerificationReceipt,
}
