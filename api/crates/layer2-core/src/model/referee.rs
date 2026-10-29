use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ProofShape;
use crate::ids::{ClaimId, PersonId, SubmissionId};

/// Characters allowed in a sent letter.
pub const MAX_LETTER_CHARS: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Accept,
    MinorRevision,
    MajorRevision,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Affects correctness or a main result.
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeConcern {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<ClaimId>,
    pub severity: Severity,
    pub issue: String,
}

/// The referee's reading of one statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeClaim {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conjecture: Option<super::ConjectureReading>,
    pub claim: ClaimId,
    pub shape: ProofShape,
    #[serde(default)]
    pub witnesses: Vec<String>,
    /// Where the statement already appears in the literature, if the
    /// referee knows a source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeReport {
    /// Absent when the referee's answer could not be read as a report; the
    /// full answer is still in `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<Recommendation>,
    pub summary: String,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub concerns: Vec<RefereeConcern>,
    #[serde(default)]
    pub claims: Vec<RefereeClaim>,
    #[serde(default)]
    pub limits: Vec<String>,
    /// The referee's complete answer.
    pub text: String,
}

/// Codex's source-based audit of the referee report, never a Lean receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Correctness {
    Correct,
    Gap,
    Error,
    NotChecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcernStatus {
    Confirmed,
    Refuted,
    NotCheckable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditedClaim {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<super::ClaimCoverage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methodology: Option<super::MethodologyReading>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conjecture: Option<super::ConjectureReading>,
    pub claim: ClaimId,
    pub correctness: Correctness,
    pub comment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ProofShape>,
    #[serde(default)]
    pub witnesses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known: Option<String>,
    pub referee_agreed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditedConcern {
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub basis: String,
    /// Reported source location, not independently authenticated.
    #[serde(default)]
    pub location: String,
    pub concern: String,
    pub status: ConcernStatus,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeAudit {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub response_assessments: Vec<ResponseAssessment>,
    pub verdict: Recommendation,
    pub agrees_with_referee: bool,
    pub summary: String,
    pub claims: Vec<AuditedClaim>,
    pub concerns: Vec<AuditedConcern>,
}

/// A source-bound reviewer conclusion, never an independent verification receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseDisposition {
    Addressed,
    Unresolved,
    NotCheckable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewEvidenceReference {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAssessment {
    pub response_id: String,
    pub status: ResponseDisposition,
    pub explanation: String,
    #[serde(default)]
    pub evidence: Vec<ReviewEvidenceReference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementKind {
    /// A step of a proof that is missing or unclear.
    Gap,
    /// A stronger form of a result.
    Strengthen,
    Generalize,
    /// Computation or verification that would support a result.
    Computation,
    Literature,
    Exposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    Checked,
    #[default]
    Proposed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Improvement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<ClaimId>,
    pub kind: ImprovementKind,
    pub suggestion: String,
    /// What contributors could do for the author on this point.
    #[serde(default)]
    pub how_we_help: String,
    #[serde(default)]
    pub status: Evidence,
    #[serde(default)]
    pub evidence: String,
    pub effort: Effort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feasibility {
    /// The definitions and tools needed are in Mathlib.
    Ready,
    /// Needs definitions or lemmas that Mathlib lacks.
    NeedsLibrary,
    /// Needs substantial new formal infrastructure.
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationCandidate {
    pub claim: ClaimId,
    pub feasibility: Feasibility,
    /// Mathlib notions the formal statement would use.
    #[serde(default)]
    pub mathlib: Vec<String>,
    /// Definitions or lemmas that would have to be built.
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(default)]
    pub lean_sketch: String,
    pub plan: String,
    pub effort: Effort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advice {
    pub summary: String,
    #[serde(default)]
    pub improvements: Vec<Improvement>,
    #[serde(default)]
    pub formalization: Vec<FormalizationCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    /// The Lean file compiled against Mathlib without `sorry`, and the
    /// theorem depends only on the standard axioms. Whether the formal
    /// statement says what the paper says is for a human to judge.
    Compiled,
    Failed,
}

/// One statement the formalization probe tried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalAttempt {
    pub claim: ClaimId,
    pub outcome: ProbeOutcome,
    /// The checked declaration, when one was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theorem: Option<String>,
    /// The complete Lean source.
    pub lean: String,
    /// Axioms the theorem depends on, as Lean reported them.
    #[serde(default)]
    pub axioms: Vec<String>,
    /// How the formal statement relates to the paper's, in the prover's words.
    #[serde(default)]
    pub note: String,
    /// Why the check failed, with the end of the compiler output.
    #[serde(default)]
    pub log: String,
}

/// A private formalization probe: never published, never a recorded
/// formalization. Publishing a Lean proof still needs the author's approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalProbe {
    /// Lean toolchain and Mathlib revision the files were checked against.
    pub toolchain: String,
    #[serde(default)]
    pub attempts: Vec<FormalAttempt>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LetterDraft {
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StepState<T> {
    Pending,
    Running {
        /// The external task, when the work runs elsewhere.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        queue_position: Option<u32>,
        since: DateTime<Utc>,
    },
    Done {
        result: T,
        at: DateTime<Utc>,
    },
    Failed {
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(default)]
        retryable: bool,
        at: DateTime<Utc>,
    },
    Skipped {
        reason: String,
    },
}

/// One step of a round and the engine that did (or does) it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step<T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    pub state: StepState<T>,
}

impl<T> Step<T> {
    pub fn pending() -> Self {
        Self {
            engine: None,
            model: None,
            attempts: 0,
            client_ref: None,
            input_digest: None,
            state: StepState::Pending,
        }
    }

    fn not_in_round() -> Self {
        Self {
            state: StepState::Skipped {
                reason: "not part of this round".into(),
            },
            ..Self::pending()
        }
    }

    pub fn done(&self) -> Option<&T> {
        match &self.state {
            StepState::Done { result, .. } => Some(result),
            _ => None,
        }
    }

    pub fn is_settled(&self) -> bool {
        matches!(
            self.state,
            StepState::Done { .. } | StepState::Failed { .. } | StepState::Skipped { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeRound {
    /// Final report style pinned independently; legacy admitted prompts stay exact.
    #[serde(default)]
    pub letter_contract: u32,
    #[serde(default)]
    pub computation_contract: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub computation_receipts: Vec<super::ComputationReceipt>,
    /// Independent knowledge protocol pin; absent historical rounds keep their
    /// original prompt bytes and never receive fabricated search receipts.
    #[serde(default)]
    pub knowledge_contract: u32,
    /// Pin new audit output contracts; old admitted prompts retain their exact bytes.
    #[serde(default)]
    pub review_contract: u32,
    #[serde(default)]
    pub source_sha256: String,
    #[serde(default)]
    pub recheck_requests: Vec<String>,
    /// 1-based, in order of starting.
    pub number: u32,
    /// The paper version and statement set the round reads.
    pub version: u32,
    pub claims_revision: u64,
    pub started_at: DateTime<Utc>,
    pub referee: Step<RefereeReport>,
    #[serde(default = "Step::not_in_round")]
    pub audit: Step<RefereeAudit>,
    pub advice: Step<Advice>,
    /// Rounds stored before the probe existed read as skipped.
    #[serde(default = "Step::not_in_round")]
    pub formal: Step<FormalProbe>,
    pub letter: Step<LetterDraft>,
}

impl RefereeRound {
    pub fn is_settled(&self) -> bool {
        self.referee.is_settled()
            && self.audit.is_settled()
            && self.advice.is_settled()
            && self.formal.is_settled()
            && self.letter.is_settled()
    }
}

/// Feedback the editors sent to the author.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackLetter {
    /// The round the letter answers, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
    /// The sending editor's assessment; never a publication decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<Recommendation>,
    #[serde(default)]
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub edited: bool,
    pub sent_by: PersonId,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStep {
    #[serde(default)]
    pub cma_attempts: Vec<super::CmaAttempt>,
    pub version: u32,
    pub claims_revision: u64,
    /// Provider-specific progress; never part of an author/public projection.
    pub progress: serde_json::Value,
}

/// Immutable author reply: the source it asks to recheck is explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingResponse {
    pub id: String,
    pub round: u32,
    /// Zero-based index into the immutable round audit's concerns.
    pub concern: usize,
    pub version: u32,
    pub claims_revision: u64,
    pub body: String,
    pub author: PersonId,
    pub at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewFindingResponse {
    pub round: u32,
    pub concern: usize,
    pub version: u32,
    pub claims_revision: u64,
    pub body: String,
}

/// Every round and letter of one paper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefereeFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feedback_proposals: Vec<super::FeedbackProposal>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binding_boards: Vec<super::BindingBoard>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research_obligations: Vec<super::ResearchObligation>,
    /// Append-only query admissions, actual receipts and source-audited readings.
    /// Historical files have no fabricated knowledge observations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub knowledge_events: Vec<super::KnowledgeEvent>,
    #[serde(default)]
    pub finding_responses: Vec<FindingResponse>,
    #[serde(default)]
    pub recheck_requests: Vec<String>,
    /// Private durable external-agent steps, including auxiliary problem/target work.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub agent_steps: std::collections::BTreeMap<String, AgentStep>,
    /// The paper's id.
    pub id: SubmissionId,
    #[serde(default)]
    pub rounds: Vec<RefereeRound>,
    #[serde(default)]
    pub letters: Vec<FeedbackLetter>,
    #[serde(default)]
    pub revision: u64,
}

impl RefereeFile {
    pub fn new(id: SubmissionId) -> Self {
        Self {
            id,
            research_obligations: vec![],
            binding_boards: vec![],
            feedback_proposals: vec![],
            knowledge_events: vec![],
            finding_responses: vec![],
            agent_steps: Default::default(),
            recheck_requests: vec![],
            rounds: vec![],
            letters: vec![],
            revision: 0,
        }
    }

    pub fn current(&self) -> Option<&RefereeRound> {
        self.rounds.last()
    }
}

/// Small, author-safe status for lists; no historical report bodies or query payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewSummary {
    pub round: Option<u32>,
    pub version: Option<u32>,
    pub claims_revision: u64,
    pub delivered_at: Option<DateTime<Utc>>,
    pub in_progress: bool,
    pub interrupted: bool,
    pub has_execution_limits: bool,
}

impl RefereeFile {
    /// A small deterministic index over every source/claim revision in this history.
    pub fn summary_index(&self) -> Vec<ReviewSummary> {
        self.rounds
            .iter()
            .map(|r| (r.version, r.claims_revision))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|(version, claims)| self.summary_for(Some(version), claims))
            .collect()
    }

    pub fn summary(&self, submission: &super::Submission) -> ReviewSummary {
        self.summary_for(
            submission.current_version().map(|v| v.number),
            submission.claims_revision,
        )
    }

    pub fn summary_for(&self, version: Option<u32>, claims_revision: u64) -> ReviewSummary {
        let current = self
            .rounds
            .iter()
            .filter(|r| Some(r.version) == version && r.claims_revision == claims_revision)
            .max_by_key(|r| r.number);
        let delivered_at = current.and_then(|r| {
            self.letters
                .iter()
                .filter(|letter| letter.round == Some(r.number))
                .map(|letter| letter.sent_at)
                .max()
        });
        let progress = current.map(|r| {
            [
                &r.referee.state as &dyn SummaryStep,
                &r.audit.state,
                &r.advice.state,
                &r.letter.state,
            ]
        });
        let failed = progress
            .as_ref()
            .is_some_and(|steps| steps.iter().any(|step| step.failed()));
        ReviewSummary {
            round: current.map(|r| r.number),
            version,
            claims_revision,
            delivered_at,
            in_progress: !failed
                && delivered_at.is_none()
                && progress
                    .as_ref()
                    .is_some_and(|steps| steps.iter().any(|step| step.live())),
            interrupted: delivered_at.is_none() && failed,
            has_execution_limits: failed
                || current.is_some_and(|r| {
                    r.referee
                        .done()
                        .is_some_and(|report| report.claims.is_empty())
                }),
        }
    }
}
trait SummaryStep {
    fn failed(&self) -> bool;
    fn live(&self) -> bool;
}
impl<T> SummaryStep for StepState<T> {
    fn failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
    fn live(&self) -> bool {
        matches!(self, Self::Pending | Self::Running { .. })
    }
}

/// GET projection: staff receive the full steps; authors receive no advice,
/// draft, provider metadata, or Lean files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefereeView {
    pub finding_responses: Vec<FindingResponse>,
    pub cma_attempts: Vec<super::CmaAttemptView>,
    pub id: SubmissionId,
    pub rounds: Vec<RefereeRoundView>,
    pub letters: Vec<FeedbackLetter>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefereeRoundView {
    pub letter_contract: u32,
    /// Selected mathematical plans, available only after a letter for this round.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formalization_suggestions: Option<Vec<FormalizationSuggestion>>,
    pub computation_contract: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub computation_receipts: Vec<super::ComputationReceipt>,
    #[serde(default)]
    pub knowledge_contract: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub knowledge_events: Vec<super::KnowledgeEvent>,
    pub review_contract: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_method: Option<String>,
    /// Payload-free lifecycle information, safe even for hidden advice and drafts.
    pub progress: RoundProgress,
    pub response_ids: Vec<String>,
    pub source_sha256: String,
    pub number: u32,
    pub version: u32,
    pub claims_revision: u64,
    pub started_at: DateTime<Utc>,
    pub referee: Step<RefereeReport>,
    pub audit: Step<RefereeAudit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advice: Option<Step<Advice>>,
    pub formal: Step<FormalProbeView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub letter: Option<Step<LetterDraft>>,
}

/// A recommendation is neither author approval nor a verified formalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormalizationSuggestion {
    pub excerpt: bool,
    pub claim: ClaimId,
    pub feasibility: Feasibility,
    pub plan: String,
}

impl RefereeRound {
    fn formalization_suggestions(&self) -> Option<Vec<FormalizationSuggestion>> {
        let advice = self.advice.done()?;
        let audit = self.audit.done()?;
        let mut seen = std::collections::BTreeSet::new();
        Some(
            advice
                .formalization
                .iter()
                .filter(|candidate| {
                    audit.claims.iter().any(|claim| {
                        claim.claim == candidate.claim && claim.correctness == Correctness::Correct
                    }) && !candidate.plan.trim().is_empty()
                        && seen.insert(candidate.claim.clone())
                })
                .take(10)
                .map(|candidate| FormalizationSuggestion {
                    excerpt: candidate.plan.chars().count() > 3_000,
                    claim: candidate.claim.clone(),
                    feasibility: candidate.feasibility,
                    plan: candidate.plan.chars().take(3_000).collect(),
                })
                .collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundProgress {
    pub referee: StageProgress,
    pub audit: StageProgress,
    pub advice: StageProgress,
    pub letter: StageProgress,
    pub formal: StageProgress,
}

/// No provider identifiers, free-form errors, prompts or model results belong here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StageProgress {
    Pending,
    Running { since: DateTime<Utc> },
    Done { at: DateTime<Utc> },
    Failed { at: DateTime<Utc>, retryable: bool },
    Skipped,
}

impl<T> From<&Step<T>> for StageProgress {
    fn from(step: &Step<T>) -> Self {
        match &step.state {
            StepState::Pending => Self::Pending,
            StepState::Running { since, .. } => Self::Running { since: *since },
            StepState::Done { at, .. } => Self::Done { at: *at },
            StepState::Failed { at, retryable, .. } => Self::Failed {
                at: *at,
                retryable: *retryable,
            },
            StepState::Skipped { .. } => Self::Skipped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FormalProbeView {
    Staff(FormalProbe),
    Author { attempts: Vec<ProbeOutcomeView> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProbeOutcomeView {
    pub claim: ClaimId,
    pub outcome: ProbeOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theorem: Option<String>,
}

impl<T: Clone> Step<T> {
    fn project<U>(&self, staff: bool, map: impl FnOnce(&T) -> U) -> Step<U> {
        let state = match &self.state {
            StepState::Pending => StepState::Pending,
            StepState::Running {
                task,
                queue_position,
                since,
            } => StepState::Running {
                task: staff.then(|| task.clone()).flatten(),
                queue_position: staff.then_some(*queue_position).flatten(),
                since: *since,
            },
            StepState::Done { result, at } => StepState::Done {
                result: map(result),
                at: *at,
            },
            StepState::Failed {
                reason,
                detail,
                retryable,
                at,
            } => StepState::Failed {
                reason: if staff {
                    reason.clone()
                } else {
                    "Review could not finish; please try again later.".into()
                },
                detail: staff.then(|| detail.clone()).flatten(),
                retryable: *retryable,
                at: *at,
            },
            StepState::Skipped { reason } => StepState::Skipped {
                reason: if staff {
                    reason.clone()
                } else {
                    "Not required.".into()
                },
            },
        };
        Step {
            engine: staff.then(|| self.engine.clone()).flatten(),
            model: staff.then(|| self.model.clone()).flatten(),
            attempts: if staff { self.attempts } else { 0 },
            client_ref: staff.then(|| self.client_ref.clone()).flatten(),
            input_digest: staff.then(|| self.input_digest.clone()).flatten(),
            state,
        }
    }
}

impl RefereeFile {
    pub fn view(self, staff: bool) -> RefereeView {
        let mut attempts: Vec<super::CmaAttemptView> = self
            .agent_steps
            .values()
            .flat_map(|s| {
                if s.cma_attempts.is_empty() {
                    vec![super::CmaAttemptView::legacy(&s.progress)]
                } else {
                    s.cma_attempts
                        .iter()
                        .map(super::CmaAttemptView::from)
                        .collect()
                }
            })
            .collect();
        let admission: std::collections::BTreeMap<_, _> = attempts
            .iter()
            .filter(|a| a.attempt == 0)
            .map(|a| (a.step.clone(), a.admitted_at))
            .collect();
        attempts.sort_by(|a, b| {
            (admission.get(&a.step), &a.step, a.attempt).cmp(&(
                admission.get(&b.step),
                &b.step,
                b.attempt,
            ))
        });
        let delivered: std::collections::BTreeSet<_> = self
            .letters
            .iter()
            .filter_map(|letter| letter.round)
            .collect();
        RefereeView {
            finding_responses: self.finding_responses,
            cma_attempts: attempts,
            id: self.id,
            letters: self.letters,
            revision: self.revision,
            rounds: self
                .rounds
                .into_iter()
                .map(|r| RefereeRoundView {
                    letter_contract: r.letter_contract,
                    formalization_suggestions: delivered
                        .contains(&r.number)
                        .then(|| r.formalization_suggestions())
                        .flatten(),
                    computation_contract: r.computation_contract,
                    computation_receipts: r.computation_receipts.clone(),
                    knowledge_contract: r.knowledge_contract,
                    knowledge_events: self
                        .knowledge_events
                        .iter()
                        .filter(|event| {
                            let subject = event.subject();
                            subject.round == r.number
                                && subject.version == r.version
                                && subject.claims_revision == r.claims_revision
                        })
                        .cloned()
                        .collect(),
                    review_contract: r.review_contract,
                    review_method: (r.review_contract >= super::REVIEW_CONTRACT)
                        .then(|| super::REVIEW_METHOD.into()),
                    progress: RoundProgress {
                        referee: (&r.referee).into(),
                        audit: (&r.audit).into(),
                        advice: (&r.advice).into(),
                        letter: (&r.letter).into(),
                        formal: (&r.formal).into(),
                    },
                    response_ids: r
                        .recheck_requests
                        .iter()
                        .filter_map(|t| t.strip_prefix("finding-response:").map(str::to_owned))
                        .collect(),
                    source_sha256: r.source_sha256,
                    number: r.number,
                    version: r.version,
                    claims_revision: r.claims_revision,
                    started_at: r.started_at,
                    referee: r.referee.project(staff, Clone::clone),
                    audit: r.audit.project(staff, Clone::clone),
                    advice: staff.then_some(r.advice),
                    letter: staff.then_some(r.letter),
                    formal: r.formal.project(staff, |p| {
                        if staff {
                            FormalProbeView::Staff(p.clone())
                        } else {
                            FormalProbeView::Author {
                                attempts: p
                                    .attempts
                                    .iter()
                                    .map(|a| ProbeOutcomeView {
                                        claim: a.claim.clone(),
                                        outcome: a.outcome,
                                        theorem: a.theorem.clone(),
                                    })
                                    .collect(),
                            }
                        }
                    }),
                })
                .collect(),
        }
    }
}

/// A machine update to one step of a round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoundUpdate {
    Referee(Step<RefereeReport>),
    Audit(Step<RefereeAudit>),
    Advice(Step<Advice>),
    Formal(Step<FormalProbe>),
    Letter(Step<LetterDraft>),
}

/// An editor's in-app letter to the authors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewLetter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<Recommendation>,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub note: String,
}

/// Revision-bound progress or cleanup receipt from a selected durable attempt.
pub struct AgentStepUpdate {
    pub source_revision: u64,
    pub ordinal: Option<u32>,
    pub version: u32,
    pub claims_revision: u64,
    pub progress: serde_json::Value,
    pub cleanup: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_never_exposes_payloads_or_provider_details() {
        let at = "2026-10-15T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        for (state, expected) in [
            (StepState::Pending, serde_json::json!({"state": "pending"})),
            (
                StepState::Running {
                    task: Some("private-task".into()),
                    queue_position: Some(42),
                    since: at,
                },
                serde_json::json!({"state": "running", "since": at}),
            ),
            (
                StepState::Done {
                    result: "private-draft",
                    at,
                },
                serde_json::json!({"state": "done", "at": at}),
            ),
            (
                StepState::Failed {
                    reason: "private-provider-error".into(),
                    detail: Some("private-path".into()),
                    retryable: true,
                    at,
                },
                serde_json::json!({"state": "failed", "at": at, "retryable": true}),
            ),
            (
                StepState::Skipped {
                    reason: "private-reason".into(),
                },
                serde_json::json!({"state": "skipped"}),
            ),
        ] {
            let step = Step {
                engine: Some("private-engine".into()),
                client_ref: Some("private-ref".into()),
                input_digest: Some("private-digest".into()),
                state,
                ..Step::pending()
            };
            assert_eq!(
                serde_json::to_value(StageProgress::from(&step)).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn letter_assessments_round_trip_and_older_letters_remain_valid() {
        let legacy = serde_json::json!({
            "subject": "Feedback", "body": "A useful point.",
            "sent_by": "editor", "sent_at": "2026-10-08T12:00:00Z"
        });
        let letter: FeedbackLetter = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(letter.assessment, None);
        assert!(
            serde_json::to_value(&letter)
                .unwrap()
                .get("assessment")
                .is_none()
        );
        let new: NewLetter = serde_json::from_value(serde_json::json!({
            "subject": "Feedback", "body": "A useful point."
        }))
        .unwrap();
        assert_eq!(new.assessment, None);
        assert!(
            serde_json::to_value(&new)
                .unwrap()
                .get("assessment")
                .is_none()
        );

        for assessment in [
            Recommendation::Accept,
            Recommendation::MinorRevision,
            Recommendation::MajorRevision,
            Recommendation::Reject,
        ] {
            let mut letter = letter.clone();
            letter.assessment = Some(assessment);
            let value = serde_json::to_value(&letter).unwrap();
            assert_eq!(
                serde_json::from_value::<FeedbackLetter>(value).unwrap(),
                letter
            );
            let mut new = new.clone();
            new.assessment = Some(assessment);
            let value = serde_json::to_value(&new).unwrap();
            assert_eq!(serde_json::from_value::<NewLetter>(value).unwrap(), new);
        }
    }
}
