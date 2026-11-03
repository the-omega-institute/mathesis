use crate::ids::{ClaimId, PersonId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmaStatus {
    Disconnected,
    SetupNeeded,
    Ready,
    Unusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmaProfile {
    pub id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmaStepKind {
    Referee,
    PaperExplanation,
    ProblemReferee,
    Audit,
    Advice,
    Letter,
    LeanProbe,
    ConjectureTarget,
    ResearchTarget,
    ResearchFaithfulness,
    ProblemAudit,
    TargetFaithfulness,
    ErrorReportAudit,
    BindingAudit,
    VenueAttempt,
    #[default]
    LegacyUnknown,
}
impl CmaStepKind {
    pub fn author_eligible(self) -> bool {
        matches!(
            self,
            Self::PaperExplanation
                | Self::Referee
                | Self::ProblemReferee
                | Self::Audit
                | Self::Advice
                | Self::Letter
                | Self::LeanProbe
                | Self::ConjectureTarget
                | Self::ResearchTarget
                | Self::ResearchFaithfulness
                | Self::ProblemAudit
                | Self::TargetFaithfulness
        )
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmaStepIdentity {
    pub kind: CmaStepKind,
    pub round: Option<u32>,
    pub claim: Option<ClaimId>,
    pub target_generation: Option<u32>,
}
impl CmaStepIdentity {
    pub fn from_label(label: &str) -> Self {
        let parts: Vec<_> = label.split(':').collect();
        match parts.as_slice() {
            ["explanation", _] => Self {
                kind: CmaStepKind::PaperExplanation,
                ..Self::default()
            },
            [round, kind] if round.starts_with('r') => Self {
                kind: match *kind {
                    "referee" => CmaStepKind::Referee,
                    "audit" => CmaStepKind::Audit,
                    "advice" => CmaStepKind::Advice,
                    "letter" => CmaStepKind::Letter,
                    "formal" => CmaStepKind::LeanProbe,
                    _ => CmaStepKind::LegacyUnknown,
                },
                round: round[1..].parse().ok(),
                ..Self::default()
            },
            [round, "knowledge-followup", "audit"] if round.starts_with('r') => Self {
                kind: CmaStepKind::Audit,
                round: round[1..].parse().ok(),
                ..Self::default()
            },
            ["problem", claim, round, kind @ ("audit" | "referee")] => Self {
                kind: if *kind == "referee" {
                    CmaStepKind::ProblemReferee
                } else {
                    CmaStepKind::ProblemAudit
                },
                claim: Some((*claim).into()),
                round: round.strip_prefix('r').and_then(|r| r.parse().ok()),
                target_generation: None,
            },
            ["target", claim, generation] | ["target", claim, generation, "pass0" | "pass1"] => {
                Self {
                    kind: CmaStepKind::ConjectureTarget,
                    claim: Some((*claim).into()),
                    round: None,
                    target_generation: generation.strip_prefix('a').and_then(|g| g.parse().ok()),
                }
            }
            ["target", claim, generation, "faithfulness"]
            | [
                "target",
                claim,
                generation,
                "pass0" | "pass1",
                "faithfulness",
            ] => Self {
                kind: CmaStepKind::TargetFaithfulness,
                claim: Some((*claim).into()),
                target_generation: generation.strip_prefix('a').and_then(|g| g.parse().ok()),
                round: None,
            },
            ["research", _, generation, "pass0" | "pass1"] => Self {
                kind: CmaStepKind::ResearchTarget,
                target_generation: generation.strip_prefix('a').and_then(|g| g.parse().ok()),
                ..Self::default()
            },
            ["research", _, generation, "pass0" | "pass1", "faithfulness"] => Self {
                kind: CmaStepKind::ResearchFaithfulness,
                target_generation: generation.strip_prefix('a').and_then(|g| g.parse().ok()),
                ..Self::default()
            },
            ["binding", _, _] => Self {
                kind: CmaStepKind::BindingAudit,
                ..Self::default()
            },
            ["error-report", _] => Self {
                kind: CmaStepKind::ErrorReportAudit,
                ..Self::default()
            },
            ["agent-attempt", claim, _] => Self {
                kind: CmaStepKind::VenueAttempt,
                claim: Some((*claim).into()),
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

/// Private account binding. Credential custody belongs to the binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmaBinding {
    pub person: PersonId,
    pub revision: u64,
    pub subject: PersonId,
    pub generation: u64,
    pub status: CmaStatus,
    pub reason: String,
    pub workspace_id: Option<String>,
    pub sandbox_id: Option<String>,
    pub sandbox_profile: Option<CmaProfile>,
    pub checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl CmaBinding {
    pub fn disconnected(person: PersonId, at: DateTime<Utc>) -> Self {
        Self {
            subject: person.clone(),
            person,
            revision: 0,
            generation: 0,
            status: CmaStatus::Disconnected,
            reason: "not_connected".into(),
            workspace_id: None,
            sandbox_id: None,
            sandbox_profile: None,
            checked_at: None,
            created_at: at,
            updated_at: at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MyCma {
    pub enabled: bool,
    pub binding: CmaBinding,
    pub setup_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetCmaWorkspace {
    pub workspace_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmaPayer {
    Author,
    Operator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmaFailure {
    Account,
    Auth,
    Setup,
    Credential,
    Quota,
    Billing,
    CauseUnknown,
    Output,
    Cancelled,
}
impl CmaFailure {
    pub fn permits_fallback(self) -> bool {
        !matches!(self, Self::Output | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmaOutcome {
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmaAttempt {
    #[serde(default)]
    pub identity: CmaStepIdentity,
    pub step: String,
    pub ordinal: u32,
    pub payer: CmaPayer,
    pub owner: Option<PersonId>,
    pub binding_generation: Option<u64>,
    pub workspace: String,
    pub reason: String,
    pub fallback_from: Option<u32>,
    pub admitted_at: DateTime<Utc>,
    pub outcome: CmaOutcome,
    pub finished_at: Option<DateTime<Utc>>,
    pub failure: Option<CmaFailure>,
    /// Retained progress of a superseded attempt; the current progress lives on AgentStep.
    #[serde(default)]
    pub progress: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CmaAttemptView {
    pub kind: CmaStepKind,
    pub round: Option<u32>,
    pub claim: Option<ClaimId>,
    pub target_generation: Option<u32>,
    pub step: String,
    pub attempt: u32,
    pub payer: CmaPayer,
    pub reason: String,
    pub status: CmaOutcome,
    pub admitted_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}
impl From<&CmaAttempt> for CmaAttemptView {
    fn from(a: &CmaAttempt) -> Self {
        Self {
            kind: a.identity.kind,
            round: a.identity.round,
            claim: a.identity.claim.clone(),
            target_generation: a.identity.target_generation,
            step: a.step.clone(),
            attempt: a.ordinal,
            payer: a.payer,
            reason: a.reason.clone(),
            status: a.outcome,
            admitted_at: Some(a.admitted_at),
            finished_at: a.finished_at,
        }
    }
}

impl CmaAttemptView {
    pub fn legacy(progress: &serde_json::Value) -> Self {
        Self {
            kind: CmaStepKind::LegacyUnknown,
            round: None,
            claim: None,
            target_generation: None,
            step: "legacy_cma".into(),
            attempt: 0,
            payer: CmaPayer::Operator,
            reason: "legacy_unknown".into(),
            status: if progress["answer"].is_string() {
                CmaOutcome::Succeeded
            } else if progress["failure"].is_string() {
                CmaOutcome::Failed
            } else {
                CmaOutcome::Running
            },
            admitted_at: None,
            finished_at: None,
        }
    }
}

/// Provider observations, without authorization or readiness decisions.
#[derive(Debug, Clone)]
pub enum CmaWorkspaceCheck {
    Resources(CmaWorkspaceFacts),
    AuthorizationRefused,
    SetupMissing,
}
#[derive(Debug, Clone)]
pub struct CmaWorkspaceFacts {
    pub sandbox_id: String,
    pub sandbox_profile: CmaProfile,
    pub owner: String,
    pub workspace_status: String,
    pub sandbox_status: String,
    pub model_access: String,
}
