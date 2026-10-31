//! Source-declared contribution history. Importing these records preserves
//! attribution; it never awards Mathesis proof or correction credit.
use super::common::require_text;
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceContributionRole {
    Problem,
    MathematicalProof,
    Exposition,
    Formalization,
    Correction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContributor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContribution {
    /// The original contribution identity in this source, never a new venue ID.
    pub id: String,
    pub claim_label: String,
    pub role: SourceContributionRole,
    pub contributors: Vec<SourceContributor>,
    pub evidence_url: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    /// Explicit source history links, not formal proof-use assertions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

pub(crate) fn validate_source_contributions(items: &[SourceContribution]) -> CoreResult<()> {
    if items.len() > 100 {
        return Err(CoreError::invalid("at most 100 source contributions"));
    }
    let mut ids = BTreeSet::new();
    for item in items {
        require_text("source contribution id", &item.id, 200)?;
        if item.id.trim() != item.id
            || item.id.chars().any(char::is_control)
            || !ids.insert(item.id.as_str())
        {
            return Err(CoreError::invalid(
                "source contribution ids must be unique stable identifiers",
            ));
        }
        require_text("contribution claim label", &item.claim_label, 200)?;
        require_text("contribution scope", &item.scope, 2000)?;
        super::manuscript::public_link(&item.evidence_url)?;
        if item.contributors.is_empty() || item.contributors.len() > 50 {
            return Err(CoreError::invalid(
                "a source contribution needs 1 to 50 named contributors",
            ));
        }
        for person in &item.contributors {
            require_text("contributor name", &person.name, 300)?;
            if let Some(url) = &person.profile_url {
                super::manuscript::public_link(url)?;
            }
        }
        if let Some(revision) = &item.source_revision
            && (!matches!(revision.len(), 40 | 64)
                || !revision.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err(CoreError::invalid(
                "contribution revision must be a full Git commit",
            ));
        }
        if item.role == SourceContributionRole::Formalization
            && item
                .source_revision
                .as_ref()
                .is_none_or(|r| !item.evidence_url.contains(&format!("/{r}/")))
        {
            return Err(CoreError::invalid(
                "formalization credit needs evidence at its pinned Git revision",
            ));
        }
    }
    for item in items {
        let mut seen = BTreeSet::new();
        for parent in &item.depends_on {
            if parent == &item.id || !ids.contains(parent.as_str()) || !seen.insert(parent) {
                return Err(CoreError::invalid(
                    "contribution history contains a missing, repeated or self dependency",
                ));
            }
        }
    }
    // Preserve the supplied edges; never invent an ordering if they form a cycle.
    let mut pending: BTreeMap<_, _> = items
        .iter()
        .map(|item| (item.id.as_str(), &item.depends_on))
        .collect();
    let mut visited = BTreeSet::new();
    while !pending.is_empty() {
        let ready: Vec<_> = pending
            .iter()
            .filter(|(_, parents)| parents.iter().all(|id| visited.contains(id.as_str())))
            .map(|(id, _)| *id)
            .collect();
        if ready.is_empty() {
            return Err(CoreError::invalid("contribution history contains a cycle"));
        }
        for id in ready {
            pending.remove(id);
            visited.insert(id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, parents: &[&str]) -> SourceContribution {
        SourceContribution {
            id: id.into(),
            claim_label: "thm:main".into(),
            role: SourceContributionRole::MathematicalProof,
            contributors: vec![SourceContributor {
                name: "Original Author".into(),
                profile_url: None,
            }],
            evidence_url: "https://example.org/proof".into(),
            scope: "Theorem 1.".into(),
            source_revision: None,
            depends_on: parents.iter().map(|s| (*s).into()).collect(),
        }
    }
    #[test]
    fn preserves_joint_contribution_history_and_rejects_cycles_and_missing_nodes() {
        let items = vec![
            node("proof", &["problem", "lemma"]),
            node("problem", &[]),
            node("lemma", &["problem"]),
        ];
        assert!(validate_source_contributions(&items).is_ok());
        assert!(validate_source_contributions(&[node("a", &["b"]), node("b", &["a"])]).is_err());
        assert!(validate_source_contributions(&[node("a", &["missing"])]).is_err());
        assert!(validate_source_contributions(&[node("a", &[]), node("a", &[])]).is_err());
    }
    #[test]
    fn formalization_credit_keeps_exact_source_and_never_becomes_a_receipt() {
        let mut item = node("original-proof-id", &[]);
        item.role = SourceContributionRole::Formalization;
        assert!(validate_source_contributions(&[item.clone()]).is_err());
        let revision = "a".repeat(40);
        item.source_revision = Some(revision.clone());
        item.evidence_url =
            format!("https://github.com/example/research/blob/{revision}/Proof.lean");
        assert!(validate_source_contributions(&[item.clone()]).is_ok());
        let encoded = serde_json::to_value(&item).unwrap();
        assert!(encoded.get("receipt").is_none());
        item.contributors.clear();
        assert!(validate_source_contributions(&[item]).is_err());
    }
    #[test]
    fn legacy_origin_encoding_stays_identical_for_delivery_retries() {
        let value = serde_json::json!({"problem_ids":["p"],"source_url":"https://example.org/p","license":"CC-BY-4.0","evidence":[]});
        let origin: crate::model::ManuscriptOrigin = serde_json::from_value(value.clone()).unwrap();
        assert!(origin.contributions.is_empty());
        assert_eq!(serde_json::to_value(origin).unwrap(), value);
    }
}
