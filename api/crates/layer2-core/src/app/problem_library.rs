//! Problems are stable; accepted notes and their versions are evidence about them.
use super::{App, clamp};
use crate::{CoreError, CoreResult, ids::*, model::*, ports::Listing};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchProblemStatus {
    RelatedResearch,
    Settled,
    Partial,
    Conflicting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchProblemLink {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProblemSummary {
    pub id: String,
    pub title: String,
    pub source_id: String,
    pub status: ResearchProblemStatus,
    pub notes: usize,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProblem {
    pub next_before: Option<String>,
    pub summary: ResearchProblemSummary,
    pub descriptions: Vec<ManuscriptProblem>,
    pub notes: Vec<ProblemNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemNote {
    pub record: RecordId,
    pub title: String,
    pub authors: Vec<Author>,
    pub kind: SubmissionKind,
    pub version: u32,
    pub upstream_version: String,
    pub accepted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_url: String,
    pub license: String,
    pub results: Vec<ProblemResult>,
    pub contributions: Vec<SourceContribution>,
    pub evidence: Vec<ManuscriptEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemResult {
    pub claim: ClaimId,
    pub kind: ClaimKind,
    pub label: String,
    pub statement: String,
    /// Present only after the ordinary source audit supports this exact settlement.
    pub outcome: Option<ConjectureOutcome>,
}

fn key(s: &Submission, source_id: &str) -> Option<String> {
    // Delivery identities are owner-scoped. A caller's chosen producer name cannot
    // merge their records into somebody else's problem history.
    let manuscript = s.manuscript.as_ref()?;
    Some(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(&s.submitter, &manuscript.producer, source_id)).ok()?)
    ))
}

fn problem_title<'a>(s: &'a Submission, origin: &'a ManuscriptOrigin, id: &'a str) -> &'a str {
    if let Some(description) = origin.problems.iter().find(|p| p.id == id) {
        return &description.problem.name;
    }
    // Older deliveries supplied source IDs without descriptions. A single
    // association and one consistently named main problem give a usable title;
    // they do not establish its statement, resolution, or verification status.
    if origin.problem_ids.len() == 1 {
        let mut problems = s
            .claims
            .iter()
            .filter(|claim| claim.is_main_result())
            .filter_map(|claim| claim.settles.as_ref());
        if let Some(problem) = problems.next()
            && problems.all(|other| other == problem)
        {
            return &problem.name;
        }
    }
    id
}

pub(crate) fn problem_links(s: &Submission) -> Vec<ResearchProblemLink> {
    let Some(delivery) = s.current_version().and_then(|v| v.delivery.as_ref()) else {
        return vec![];
    };
    delivery
        .origin
        .problem_ids
        .iter()
        .filter_map(|id| {
            Some(ResearchProblemLink {
                id: key(s, id)?,
                title: problem_title(s, &delivery.origin, id).to_owned(),
            })
        })
        .collect()
}

fn claim_matches(claim: &Claim, label: &str) -> bool {
    claim.id.as_str() == label
        || claim.label == label
        || claim.latex_label.as_deref() == Some(label)
}

fn audited_outcome(
    s: &Submission,
    claim: &Claim,
    description: Option<&ManuscriptProblem>,
) -> Option<ConjectureOutcome> {
    description.and_then(|p| {
        let resolution = p
            .resolutions
            .iter()
            .find(|r| claim_matches(claim, &r.claim_label))?;
        if !claim.is_main_result() || claim.settles.as_ref() != Some(&p.problem) {
            return None;
        }
        let StagePayload::Escape { assessments } = &s.latest_report(Stage::Escape)?.payload else {
            return None;
        };
        assessments.iter().find(|a| {
            a.claim == claim.id
                && a.correctness == Some(Correctness::Correct)
                && a.methodology
                    .as_ref()
                    .is_some_and(|m| m.supports_settlement(&p.problem))
        })?;
        Some(resolution.outcome)
    })
}
fn result(s: &Submission, claim: &Claim, description: Option<&ManuscriptProblem>) -> ProblemResult {
    ProblemResult {
        claim: claim.id.clone(),
        kind: claim.kind,
        label: claim.label.clone(),
        statement: claim.statement.clone(),
        outcome: audited_outcome(s, claim, description),
    }
}

fn status(outcomes: &[ConjectureOutcome]) -> ResearchProblemStatus {
    if outcomes.contains(&ConjectureOutcome::Proved)
        && outcomes.contains(&ConjectureOutcome::Disproved)
    {
        ResearchProblemStatus::Conflicting
    } else if outcomes.contains(&ConjectureOutcome::Proved)
        || outcomes.contains(&ConjectureOutcome::Disproved)
    {
        ResearchProblemStatus::Settled
    } else if outcomes.contains(&ConjectureOutcome::Partial) {
        ResearchProblemStatus::Partial
    } else {
        ResearchProblemStatus::RelatedResearch
    }
}

impl App {
    async fn problem_library(
        &self,
        focus: Option<&str>,
        before: Option<&str>,
    ) -> CoreResult<BTreeMap<String, ResearchProblem>> {
        let mut items: BTreeMap<String, (ResearchProblem, Vec<ConjectureOutcome>, bool)> =
            BTreeMap::new();
        let mut cursor = None;
        loop {
            let page = self.ports.records.list(100, cursor).await?;
            for record in page.items {
                let s = match self.accepted(&record).await {
                    Ok(s) => s,
                    Err(CoreError::NotFound { .. }) => continue,
                    Err(e) => return Err(e),
                };
                if s.analysis_visibility != Visibility::Public
                    || s.purpose != SubmissionPurpose::Publication
                {
                    continue;
                }
                let Some(v) = s.current_version() else {
                    continue;
                };
                let Some(delivery) = &v.delivery else {
                    continue;
                };
                for source_id in &delivery.origin.problem_ids {
                    let Some(id) = key(&s, source_id) else {
                        continue;
                    };
                    if focus.is_some_and(|wanted| wanted != id) {
                        continue;
                    }
                    let description = delivery.origin.problems.iter().find(|p| &p.id == source_id);
                    let title = problem_title(&s, &delivery.origin, source_id);
                    let updated = v.uploaded_at;
                    let (item, outcomes, has_description_title) =
                        items.entry(id.clone()).or_insert_with(|| {
                            (
                                ResearchProblem {
                                    next_before: None,
                                    summary: ResearchProblemSummary {
                                        id,
                                        title: title.to_owned(),
                                        source_id: source_id.clone(),
                                        status: ResearchProblemStatus::RelatedResearch,
                                        notes: 0,
                                        updated_at: updated,
                                    },
                                    descriptions: vec![],
                                    notes: vec![],
                                },
                                vec![],
                                description.is_some(),
                            )
                        });
                    if (description.is_some() && !*has_description_title)
                        || item.summary.title == *source_id
                    {
                        item.summary.title = title.to_owned();
                    }
                    *has_description_title |= description.is_some();
                    if let Some(description) = description
                        && focus.is_some()
                        && item.descriptions.len() < 25
                        && !item.descriptions.contains(description)
                    {
                        item.descriptions.push(description.clone());
                    }
                    item.summary.notes += 1;
                    for outcome in s
                        .claims
                        .iter()
                        .filter_map(|c| audited_outcome(&s, c, description))
                    {
                        if !outcomes.contains(&outcome) {
                            outcomes.push(outcome);
                        }
                    }
                    item.summary.updated_at = item.summary.updated_at.max(updated);
                    if focus.is_some()
                        && before.is_none_or(|b| record.id.as_str() < b)
                        && item.notes.len() < 26
                    {
                        item.notes.push(ProblemNote {
                            record: record.id.clone(),
                            title: s.title.clone(),
                            authors: s.authors.clone(),
                            kind: s.kind,
                            version: v.number,
                            upstream_version: delivery.version.clone(),
                            accepted_at: record.accepted_at,
                            updated_at: v.uploaded_at,
                            source_url: delivery.origin.source_url.clone(),
                            license: delivery.origin.license.clone(),
                            results: s
                                .claims
                                .iter()
                                .filter(|c| c.is_main_result())
                                .map(|c| result(&s, c, description))
                                .collect(),
                            contributions: delivery.origin.contributions.clone(),
                            evidence: delivery.origin.evidence.clone(),
                        });
                    }
                }
            }
            cursor = page.next_before;
            if cursor.is_none() {
                break;
            }
        }
        Ok(items
            .into_iter()
            .map(|(id, (mut item, outcomes, _))| {
                item.summary.status = status(&outcomes);
                // Record order is stable across note revisions; it is also the note-page cursor.
                item.notes
                    .sort_by_key(|n| std::cmp::Reverse(n.record.clone()));
                if item.notes.len() > 25 {
                    item.notes.truncate(25);
                    item.next_before = item.notes.last().map(|n| n.record.to_string());
                }
                (id, item)
            })
            .collect())
    }

    pub async fn list_research_problems(
        &self,
        limit: Option<u32>,
        before: Option<String>,
    ) -> CoreResult<Listing<ResearchProblemSummary>> {
        let limit = clamp(limit) as usize;
        let mut items: Vec<_> = self
            .problem_library(None, None)
            .await?
            .into_values()
            .map(|p| p.summary)
            .filter(|p| before.as_ref().is_none_or(|b| &p.id < b))
            .collect();
        items.sort_by_key(|p| std::cmp::Reverse(p.id.clone()));
        let more = items.len() > limit;
        items.truncate(limit);
        Ok(Listing {
            next_before: more.then(|| items.last().unwrap().id.clone()),
            items,
        })
    }

    pub async fn research_problem(&self, id: &str) -> CoreResult<ResearchProblem> {
        self.research_problem_page(id, None).await
    }
    pub async fn research_problem_page(
        &self,
        id: &str,
        before: Option<&str>,
    ) -> CoreResult<ResearchProblem> {
        if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(CoreError::not_found("problem", id));
        }
        self.problem_library(Some(id), before)
            .await?
            .remove(id)
            .ok_or_else(|| CoreError::not_found("problem", id))
    }
}
