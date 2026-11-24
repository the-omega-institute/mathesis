//! Public mathematical reading and shared follow-up work, derived from exact public versions.
//! Private referee advice is deliberately not an input to this projection.
use super::{App, ResearchProblemLink};
use crate::{CoreError, CoreResult, ids::RecordId, model::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub claim: String,
    pub kind: ClaimKind,
    pub role: Option<ClaimRole>,
    pub name: String,
    pub statement: String,
    pub section: Option<String>,
    /// published, verified or disproved; a source/repository link does not change this state.
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperRequest {
    /// paper_link is bound by the paper's published material version. repository
    /// locates related work only; it does not certify a statement/version correspondence.
    pub association: String,
    pub request: SharedFormalizationRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperResearch {
    pub work: String,
    pub version: u32,
    pub problems: Vec<ResearchProblemLink>,
    pub results: Vec<ResearchResult>,
    pub questions: Vec<ResearchResult>,
    pub requests: Vec<PaperRequest>,
    pub lean_source: Option<String>,
    pub macros: std::collections::BTreeMap<String, String>,
    pub requests_observed_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn requests(
    materials: Option<&PaperMaterials>,
    snapshot: &SharedRequestsSnapshot,
) -> Vec<PaperRequest> {
    let Some(materials) = materials else {
        return vec![];
    };
    snapshot
        .requests
        .iter()
        .filter_map(|request| {
            let association = if materials
                .formalization_requests
                .iter()
                .any(|link| link.url == request.url)
            {
                "paper_link"
            } else if request
                .paper_repository
                .as_deref()
                .is_some_and(|repository| {
                    repository.trim_end_matches('/')
                        == materials.repository_url.trim_end_matches('/')
                })
            {
                "repository"
            } else {
                return None;
            };
            Some(PaperRequest {
                association: association.into(),
                request: request.clone(),
            })
        })
        .collect()
}

impl App {
    /// Both people and agents receive the same current public reading. A request
    /// never enters here by title similarity, a guessed author, or a private review.
    pub async fn paper_research(
        &self,
        work: &str,
        version: Option<u32>,
    ) -> CoreResult<PaperResearch> {
        let snapshot = self.shared_formalization_requests().await?;
        let mut view = if work.starts_with("WP-") {
            let paper = self.paper(&RecordId::from(work)).await?;
            if !paper.summary.public {
                return Err(CoreError::not_found("public research", work));
            }
            let published = paper
                .publication
                .as_ref()
                .ok_or_else(|| CoreError::not_found("public research", work))?;
            if version.is_some_and(|v| v != published.version) {
                return Err(CoreError::not_found("public research version", work));
            }
            PaperResearch {
                work: work.into(),
                version: published.version,
                problems: paper.problems,
                results: paper
                    .claims
                    .iter()
                    .map(|claim| ResearchResult {
                        claim: claim.id.to_string(),
                        kind: claim.kind,
                        role: Some(claim.role),
                        name: claim.label.clone(),
                        statement: claim.statement.clone(),
                        section: claim.section.clone(),
                        state: if claim
                            .verified
                            .as_ref()
                            .is_some_and(|a| a.receipt.verdict == Verdict::Disproved)
                        {
                            "disproved"
                        } else if claim.lean.is_some()
                            || claim
                                .verified
                                .as_ref()
                                .is_some_and(|a| a.receipt.verdict == Verdict::Proved)
                        {
                            "verified"
                        } else {
                            "published"
                        }
                        .into(),
                    })
                    .collect(),
                questions: vec![],
                requests: requests(paper.materials.as_ref(), &snapshot),
                lean_source: None,
                macros: paper.macros,
                requests_observed_at: snapshot.observed_at,
            }
        } else {
            let paper = self.collected_paper(work, version).await?;
            let number = paper
                .revision
                .parse::<u32>()
                .map_err(|_| CoreError::invalid("invalid public paper version"))?;
            let knowledge = self.collected_knowledge(work, number).await?;
            PaperResearch {
                work: work.into(),
                version: number,
                problems: paper.problem_links.into_iter().flatten().collect(),
                results: knowledge
                    .results
                    .statements
                    .into_iter()
                    .map(|claim| {
                        let proof = knowledge
                            .formalizations
                            .iter()
                            .find(|proof| proof.claim.as_str() == claim.id);
                        let state = proof
                            .and_then(|proof| {
                                proof.target_digest.as_ref().map(|digest| (proof, digest))
                            })
                            .and_then(|(proof, digest)| {
                                proof.proofs.iter().find(|p| {
                                    p.receipt.target_digest == *digest
                                        && matches!(
                                            p.receipt.verdict,
                                            Verdict::Proved | Verdict::Disproved
                                        )
                                })
                            })
                            .map_or("published", |p| {
                                if p.receipt.verdict == Verdict::Proved {
                                    "verified"
                                } else {
                                    "disproved"
                                }
                            });
                        ResearchResult {
                            claim: claim.id,
                            kind: claim.kind,
                            role: None,
                            name: claim.title.unwrap_or(claim.name),
                            statement: claim.statement,
                            section: claim.section,
                            state: state.into(),
                        }
                    })
                    .collect(),
                questions: vec![],
                requests: requests(paper.materials.as_ref(), &snapshot),
                lean_source: paper.paper.lean.map(|lean| lean.url),
                macros: knowledge.results.macros,
                requests_observed_at: snapshot.observed_at,
            }
        };
        let (questions, results) = view
            .results
            .into_iter()
            .partition(|result| result.kind.is_open());
        view.results = results;
        view.questions = questions;
        Ok(view)
    }
}
