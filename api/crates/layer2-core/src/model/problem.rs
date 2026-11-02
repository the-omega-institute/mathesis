//! Source problem identity and attribution, separate from a manuscript's identity.
use super::*;
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManuscriptProblem {
    pub id: String,
    pub problem: OpenProblemRef,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed_by: Vec<SourceContributor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolutions: Vec<ManuscriptResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManuscriptResolution {
    pub claim_label: String,
    pub outcome: ConjectureOutcome,
}

pub(crate) fn validate_manuscript_problems(origin: &ManuscriptOrigin) -> CoreResult<()> {
    let mut seen = std::collections::BTreeSet::new();
    if origin.problems.len() > 100 {
        return Err(CoreError::invalid("at most 100 problem descriptions"));
    }
    for item in &origin.problems {
        if !origin.problem_ids.contains(&item.id) || !seen.insert(&item.id) {
            return Err(CoreError::invalid(
                "problem descriptions must name unique associated problem ids",
            ));
        }
        super::common::require_text("problem title", &item.problem.name, 300)?;
        item.problem.source.validate()?;
        super::common::require_text("original problem statement", &item.statement, 20_000)?;
        if item.proposed_by.len() > 50 || item.resolutions.len() > 100 {
            return Err(CoreError::invalid(
                "problem attribution or resolutions exceed bounds",
            ));
        }
        for person in &item.proposed_by {
            super::common::require_text("problem contributor", &person.name, 300)?;
            if let Some(url) = &person.profile_url {
                super::manuscript::public_link(url)?;
            }
        }
        let mut labels = std::collections::BTreeSet::new();
        for resolution in &item.resolutions {
            super::common::require_text("resolution claim label", &resolution.claim_label, 200)?;
            if !labels.insert(&resolution.claim_label) {
                return Err(CoreError::invalid("duplicate problem resolution"));
            }
        }
    }
    Ok(())
}
