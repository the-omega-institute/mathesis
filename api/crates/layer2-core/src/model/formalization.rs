//! Formalizing an accepted paper's valuable statements in Lean: editors
//! propose, the author approves, contributors build, editors verify.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::{is_web_url, require_text};
use crate::{CoreError, CoreResult, ids::ClaimId};

/// The user-facing lifecycle of an accepted result's formalization work.
///
/// This is deliberately separate from `ItemState`: the latter records the
/// author's consent and the immutable publication artifact, while this value
/// records the ongoing service work between those two points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormalizationProgressState {
    TargetPreparing,
    AwaitingAuthor,
    ProofQueued,
    InProgress,
    Partial,
    Verified,
    Blocked,
}

impl FormalizationProgressState {
    pub fn name(self) -> &'static str {
        match self {
            Self::TargetPreparing => "target_preparing",
            Self::AwaitingAuthor => "awaiting_author",
            Self::ProofQueued => "proof_queued",
            Self::InProgress => "in_progress",
            Self::Partial => "partial",
            Self::Verified => "verified",
            Self::Blocked => "blocked",
        }
    }
}

/// Durable, bounded progress for the continuous formalization service.
///
/// CMA transcripts and provider details stay in the private run store. This
/// projection contains only what an author or contributor needs to understand
/// what has happened and what can happen next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationProgress {
    pub state: FormalizationProgressState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub completed_lemmas: Vec<String>,
    #[serde(default)]
    pub remaining_obligations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl FormalizationProgress {
    pub fn bounded(mut self) -> Self {
        self.completed_lemmas.truncate(50);
        self.remaining_obligations.truncate(50);
        self.next_step = self
            .next_step
            .take()
            .map(|value| value.chars().take(4_000).collect());
        self.note = self
            .note
            .take()
            .map(|value| value.chars().take(4_000).collect());
        self
    }
}

/// A pinned Lean (or checker) artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalArtifact {
    pub repository: String,
    /// Full commit SHA the result was checked at.
    pub commit: String,
    #[serde(default)]
    pub declarations: Vec<String>,
}

impl FormalArtifact {
    pub fn validate(&self) -> CoreResult<()> {
        if !is_web_url(&self.repository) {
            return Err(CoreError::invalid(
                "artifact repository must be an http(s) URL",
            ));
        }
        if self.commit.len() != 40 || !self.commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CoreError::invalid(
                "artifact commit must be a full 40-hex SHA",
            ));
        }
        if self.declarations.is_empty() {
            return Err(CoreError::invalid("an artifact names its declarations"));
        }
        Ok(())
    }
}

/// The axioms a Lean proof may use to count as kernel-verified.
pub const STANDARD_AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ItemState {
    /// Proposed by an editor; awaiting the author.
    Proposed,
    /// The author wants it formalized.
    Approved,
    Declined {
        reason: String,
    },
    InProgress,
    /// Proved in Lean with standard axioms only.
    Verified {
        artifact: FormalArtifact,
        axioms: Vec<String>,
        contribution: Option<String>,
    },
}

impl ItemState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Declined { .. } => "declined",
            Self::InProgress => "in_progress",
            Self::Verified { .. } => "verified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationItem {
    pub claim: ClaimId,
    /// Why this statement is worth formalizing.
    pub reason: String,
    pub state: ItemState,
    /// Progress is absent on legacy records and is safe to omit from public
    /// publication projections until a target is author-confirmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<FormalizationProgress>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FormalizationPlan {
    /// Where the Lean code for this paper lives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default)]
    pub items: Vec<FormalizationItem>,
}

impl FormalizationPlan {
    pub fn item(&self, claim: &ClaimId) -> Option<&FormalizationItem> {
        self.items.iter().find(|i| &i.claim == claim)
    }

    pub fn verified(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.state, ItemState::Verified { .. }))
            .count()
    }
}

pub fn validate_verification(artifact: &FormalArtifact, axioms: &[String]) -> CoreResult<()> {
    artifact.validate()?;
    if let Some(bad) = axioms
        .iter()
        .find(|a| !STANDARD_AXIOMS.contains(&a.as_str()))
    {
        return Err(CoreError::invalid(format!(
            "axiom {bad} is outside the standard set; the proof is not verified"
        )));
    }
    Ok(())
}

pub fn validate_reason(reason: &str) -> CoreResult<()> {
    require_text("reason", reason, 5_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_rules() {
        let artifact = FormalArtifact {
            repository: "https://github.com/x/y".into(),
            commit: "a".repeat(40),
            declarations: vec!["Main".into()],
        };
        assert!(validate_verification(&artifact, &["propext".into()]).is_ok());
        assert!(validate_verification(&artifact, &["sorryAx".into()]).is_err());
        let no_decls = FormalArtifact {
            declarations: vec![],
            ..artifact
        };
        assert!(validate_verification(&no_decls, &[]).is_err());
    }
}
