//! Direct collection of curated open problems by a trusted operator. Nothing here
//! reviews, audits or admits: an entry records what a curator read and when.
use super::{App, problem_library::collected_problem_key};
use crate::{CoreError, CoreResult, model::*};
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
    /// Publishes one immutable version of a curated problem entry. The operator's
    /// trust covers the curation only; no record, review, target or credit follows.
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
                return Ok(receipt(&saved));
            }
            return Err(error);
        }
        Ok(receipt(&version))
    }
}
