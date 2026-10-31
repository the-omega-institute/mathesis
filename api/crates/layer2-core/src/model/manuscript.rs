//! Versioned downstream manuscript delivery. These are source associations,
//! never mathematical verification receipts or publication decisions.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::*;
use crate::{CoreError, CoreResult, ids::PersonId};

pub const MANUSCRIPT_DELIVERY_SCHEMA: &str = "mathesis.manuscript-delivery.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManuscriptOrigin {
    pub problem_ids: Vec<String>,
    pub source_url: String,
    pub license: String,
    #[serde(default)]
    pub evidence: Vec<ManuscriptEvidence>,
    /// Existing upstream contributions, retained with their own identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributions: Vec<SourceContribution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManuscriptEvidenceKind {
    WrittenProof,
    FiniteComputation,
    LeanSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManuscriptEvidence {
    pub claim_label: String,
    pub kind: ManuscriptEvidenceKind,
    pub url: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewManuscriptDelivery {
    pub schema: String,
    pub producer: String,
    pub work_id: String,
    pub version: String,
    /// Required for a new revision; never silently supersede a newer upload.
    #[serde(default)]
    pub previous_version: Option<u32>,
    pub paper: NewPaper,
    pub origin: ManuscriptOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManuscriptIdentity {
    pub key: String,
    pub producer: String,
    pub work_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManuscriptVersion {
    pub version: String,
    pub request_digest: String,
    pub origin: ManuscriptOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<ManuscriptConfirmation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManuscriptConfirmation {
    pub digest: String,
    pub claims_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmManuscriptDelivery {
    pub version: u32,
    pub source_sha256: String,
    pub expected_claims_revision: u64,
    pub claims: Vec<ClaimConfirmation>,
}

pub(crate) fn manuscript_key(
    owner: &PersonId,
    producer: &str,
    work_id: &str,
    purpose: SubmissionPurpose,
) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(owner, producer, work_id, purpose))
                .expect("serializable identity")
        )
    )
}

fn identifier(label: &str, text: &str) -> CoreResult<()> {
    require_text(label, text, 200)?;
    if text.trim() != text || text.chars().any(char::is_control) {
        return Err(CoreError::invalid(format!(
            "{label} must be a stable identifier"
        )));
    }
    Ok(())
}

pub(super) fn public_link(value: &str) -> CoreResult<()> {
    let host = value
        .strip_prefix("https://")
        .and_then(|s| s.split('/').next());
    if value.len() > 2000 || !is_web_url(value) || host.is_none_or(|h| h.contains('@')) {
        return Err(CoreError::invalid(
            "manuscript evidence requires a public HTTPS URL without credentials",
        ));
    }
    Ok(())
}

impl NewManuscriptDelivery {
    pub fn validate(&self) -> CoreResult<()> {
        if self.schema != MANUSCRIPT_DELIVERY_SCHEMA {
            return Err(CoreError::invalid("unsupported manuscript delivery schema"));
        }
        identifier("producer", &self.producer)?;
        identifier("work id", &self.work_id)?;
        identifier("manuscript version", &self.version)?;
        self.paper.validate()?;
        if self.paper.kind == SubmissionKind::Conjecture || self.paper.typed_conjecture.is_some() {
            return Err(CoreError::invalid(
                "deliver a paper or short note through this endpoint",
            ));
        }
        if self.paper.title.is_none() || self.paper.authors.is_empty() {
            return Err(CoreError::invalid(
                "a delivered manuscript must supply its title and ordered authors",
            ));
        }
        require_text("manuscript license", &self.origin.license, 200)?;
        public_link(&self.origin.source_url)?;
        if self.origin.problem_ids.is_empty() || self.origin.problem_ids.len() > 100 {
            return Err(CoreError::invalid(
                "associate between 1 and 100 original problem ids",
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for id in &self.origin.problem_ids {
            identifier("problem id", id)?;
            if !ids.insert(id) {
                return Err(CoreError::invalid("duplicate problem id"));
            }
        }
        validate_source_contributions(&self.origin.contributions)?;
        if self.origin.evidence.len() > 100 {
            return Err(CoreError::invalid("at most 100 manuscript evidence links"));
        }
        for evidence in &self.origin.evidence {
            require_text("evidence claim label", &evidence.claim_label, 200)?;
            require_text("evidence scope", &evidence.scope, 2000)?;
            public_link(&evidence.url)?;
            if let Some(revision) = &evidence.source_revision
                && (!matches!(revision.len(), 40 | 64)
                    || !revision.bytes().all(|c| c.is_ascii_hexdigit()))
            {
                return Err(CoreError::invalid(
                    "evidence revision must be a full Git commit",
                ));
            }
            if evidence.kind == ManuscriptEvidenceKind::LeanSource
                && evidence
                    .source_revision
                    .as_ref()
                    .is_none_or(|r| !evidence.url.contains(&format!("/{r}/")))
            {
                return Err(CoreError::invalid(
                    "Lean evidence must link directly to its pinned Git revision",
                ));
            }
        }
        Ok(())
    }
}
