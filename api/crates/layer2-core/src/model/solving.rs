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
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    pub target: String,
    pub target_digest: String,
    pub toolchain: String,
    pub solution: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
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
    #[serde(default)]
    pub agent_attempted: std::collections::BTreeSet<String>,
    /// Number of durable CMA continuation rounds per confirmed target. This
    /// lets a formalization resume after a failed proof instead of replaying
    /// one frozen response from the first run.
    #[serde(default)]
    pub agent_rounds: std::collections::BTreeMap<String, u32>,
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
            .filter(|a| a.target_digest == digest && a.verified())
            .min_by_key(|a| (a.receipt.as_ref().unwrap().checked_at, &a.id))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicAttempt {
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
