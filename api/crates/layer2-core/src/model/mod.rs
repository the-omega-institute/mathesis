//! Domain aggregates. Every aggregate with a lifecycle carries a `revision`
//! used for optimistic concurrency; stores must reject a write whose expected
//! revision is not the stored one.

pub(crate) mod claim;
mod cma;
mod common;
mod conjecture;
mod coverage;
mod deposit;
mod donation;
mod endorsement;
mod formalization;
mod manuscript;
mod methodology;
mod paper;
mod person;
mod record;
pub use deposit::*;
pub mod referee;
mod review;
mod search;
mod solving;
mod task;

pub use claim::*;
pub use cma::*;
pub use common::*;
pub use conjecture::*;
pub use coverage::*;
pub use donation::*;
pub use endorsement::*;
pub use formalization::*;
pub use manuscript::*;
pub use methodology::*;
pub use paper::*;
pub use person::*;
pub use record::*;
pub use referee::{
    Advice, AgentStep, AgentStepUpdate, AuditedClaim, AuditedConcern, ConcernStatus, Correctness,
    Effort, Evidence as ImprovementEvidence, Feasibility, FeedbackLetter, FindingResponse,
    FormalAttempt, FormalProbe, FormalProbeView, FormalizationCandidate, Improvement,
    ImprovementKind, LetterDraft, MAX_LETTER_CHARS, NewFindingResponse, NewLetter, ProbeOutcome,
    ProbeOutcomeView, Recommendation, RefereeAudit, RefereeClaim, RefereeConcern, RefereeFile,
    RefereeReport, RefereeRound, RefereeRoundView, RefereeView, ResponseAssessment,
    ResponseDisposition, ReviewEvidenceReference, ReviewSummary, RoundProgress, RoundUpdate,
    Severity, StageProgress, Step, StepState,
};
pub use review::*;
pub use search::*;
pub use solving::*;
pub use task::*;

mod error_report;
pub use error_report::*;

mod project;
pub use project::*;

mod attachment;
pub use attachment::*;

mod computation;
pub use computation::*;

mod research;
pub use research::*;

mod knowledge_capabilities;
pub use knowledge_capabilities::*;

mod binding;
pub use binding::*;

mod feedback;
pub use feedback::*;
