//! Read-only projection of the canonical community work queue.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const REQUEST_REPOSITORY: &str = "the-omega-institute/trureturing-requests";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestContributor {
    pub login: String,
    pub url: String,
    pub agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestCompletion {
    pub url: String,
    pub revision: String,
    pub merged_at: DateTime<Utc>,
    pub contributor: RequestContributor,
    /// Unknown identities, platform accounts and requester self-work keep attribution without rank.
    pub ranked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedFormalizationRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    /// open, claimed, merged or closed. Closure alone is never a proof receipt.
    pub state: String,
    pub updated_at: DateTime<Utc>,
    pub requested_by: String,
    pub claimants: Vec<RequestContributor>,
    pub completions: Vec<RequestCompletion>,
    pub paper_repository: Option<String>,
    pub lean_source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRequestsSnapshot {
    pub revision: u64,
    pub observed_at: Option<DateTime<Utc>>,
    pub requests: Vec<SharedFormalizationRequest>,
}

#[derive(Debug, Serialize)]
pub struct SharedRequestRank {
    pub rank: usize,
    pub contributor: RequestContributor,
    pub completed: usize,
    pub requests: Vec<u64>,
}

impl SharedRequestsSnapshot {
    pub fn leaderboard(&self, month: bool) -> Vec<SharedRequestRank> {
        use chrono::Datelike;
        use std::collections::{BTreeMap, BTreeSet};
        let mut rows = BTreeMap::<String, (RequestContributor, BTreeSet<u64>)>::new();
        for request in &self.requests {
            for completed in &request.completions {
                if !completed.ranked
                    || month
                        && self.observed_at.is_none_or(|at| {
                            (at.year(), at.month())
                                != (completed.merged_at.year(), completed.merged_at.month())
                        })
                {
                    continue;
                }
                rows.entry(completed.contributor.login.to_lowercase())
                    .or_insert_with(|| (completed.contributor.clone(), BTreeSet::new()))
                    .1
                    .insert(request.number);
            }
        }
        let mut rows: Vec<_> = rows.into_values().collect();
        rows.sort_by(|a, b| {
            b.1.len()
                .cmp(&a.1.len())
                .then_with(|| a.0.login.to_lowercase().cmp(&b.0.login.to_lowercase()))
        });
        rows.into_iter()
            .enumerate()
            .map(|(i, (contributor, requests))| SharedRequestRank {
                rank: i + 1,
                contributor,
                completed: requests.len(),
                requests: requests.into_iter().collect(),
            })
            .collect()
    }
}
