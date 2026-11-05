//! A shared, bounded public citation graph over immutable publication inputs.
use super::App;
use crate::{CoreError, CoreResult, ids::RecordId, model::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchGraphQuery {
    /// A node identity returned by this API. Use query strings: DOIs contain slashes.
    pub focus: Option<String>,
    pub q: Option<String>,
    pub before: Option<String>,
    /// Matching public source records in this page (1–20, default 10).
    pub limit: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicReference {
    pub work: String,
    pub display_text: String,
    pub reference: BibliographicReference,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicBibliography {
    pub method: String,
    pub references: Vec<PublicReference>,
    pub limitations: Vec<String>,
}
impl PublicBibliography {
    pub(crate) fn from_source(record: &RecordId, version: &PaperVersion) -> Option<Self> {
        let b = version.bibliography.as_ref()?;
        Some(Self {
            method: b.method.clone(),
            limitations: b.limitations.clone(),
            references: b
                .references
                .iter()
                .map(|r| PublicReference {
                    work: reference_identity(record, version.number, r),
                    display_text: r.display_text(),
                    reference: r.clone(),
                })
                .collect(),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchNodeKind {
    Manuscript,
    Literature,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchNode {
    pub id: String,
    pub kind: ResearchNodeKind,
    /// Literature display is one visible source description, never canonical authorship.
    pub title: String,
    pub authors: Vec<String>,
    pub url: Option<String>,
    pub record: Option<RecordId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRelation {
    Cites,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationEdge {
    pub from: String,
    pub to: String,
    pub relation: ResearchRelation,
    pub record: RecordId,
    pub version: u32,
    pub reference_key: String,
    pub display_text: String,
    pub source_location: String,
    /// Keep conflicting descriptions attached to their actual source occurrence.
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<String>,
    /// The bounded citation text exactly as retained from this occurrence.
    pub raw: String,
    pub raw_truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCoverage {
    pub record: RecordId,
    pub version: u32,
    /// None means a legacy accepted version was not indexed.
    pub method: Option<String>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGraph {
    pub nodes: Vec<ResearchNode>,
    pub edges: Vec<CitationEdge>,
    pub coverage: Vec<GraphCoverage>,
    pub next_before: Option<String>,
    pub scope: String,
}
fn manuscript_node(record: &RecordId, s: &Submission) -> ResearchNode {
    ResearchNode {
        id: format!("paper:{record}"),
        kind: ResearchNodeKind::Manuscript,
        title: s.title.clone(),
        authors: s.authors.iter().map(|a| a.name.clone()).collect(),
        url: None,
        record: Some(record.clone()),
        version: s.current_version().map(|v| v.number),
    }
}
fn reference_url(r: &BibliographicReference) -> Option<String> {
    if let Some(doi) = r.doi.as_deref().and_then(canonical_reference_doi) {
        // Escape characters which have special meaning in a URL path/query.
        let encoded: String = doi
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        return Some(format!("https://doi.org/{encoded}"));
    }
    if let Some(arxiv) = r.arxiv.as_deref().and_then(canonical_reference_arxiv) {
        return Some(format!("https://arxiv.org/abs/{arxiv}"));
    }
    r.url
        .as_ref()
        .filter(|v| crate::model::is_web_url(v))
        .cloned()
}
impl App {
    pub async fn research_graph(&self, mut query: ResearchGraphQuery) -> CoreResult<ResearchGraph> {
        for text in [&mut query.focus, &mut query.q, &mut query.before]
            .into_iter()
            .flatten()
        {
            *text = text.trim().into();
            if text.len() > 500 || text.chars().any(char::is_control) {
                return Err(CoreError::invalid("graph query exceeds bounds"));
            }
        }
        let focus = query.focus.as_deref().filter(|v| !v.is_empty());
        if focus.is_some_and(|s| {
            !["paper:", "doi:", "arxiv:", "reference:"]
                .iter()
                .any(|p| s.starts_with(p))
        }) {
            return Err(CoreError::invalid(
                "use a node identity returned by the research graph",
            ));
        }
        let paper_focus = focus
            .and_then(|f| f.strip_prefix("paper:"))
            .map(RecordId::from);
        if paper_focus.is_some()
            && (query.before.is_some() || query.q.as_ref().is_some_and(|q| !q.is_empty()))
        {
            return Err(CoreError::invalid(
                "paper neighborhoods do not take search or pagination",
            ));
        }
        let local_focus = focus
            .and_then(|f| f.strip_prefix("reference:"))
            .and_then(|rest| {
                let mut parts = rest.splitn(3, ':');
                let record = parts.next()?;
                parts.next()?.parse::<u32>().ok()?;
                (!parts.next()?.is_empty()).then(|| RecordId::from(record))
            });
        if local_focus.is_some() && query.before.is_some() {
            return Err(CoreError::invalid(
                "source-local references do not take pagination",
            ));
        }
        if focus.is_some() && query.q.as_ref().is_some_and(|q| !q.is_empty()) {
            return Err(CoreError::invalid(
                "choose either a graph focus or text search",
            ));
        }
        let selection = focus.map_or_else(
            || ResearchSearch::from_text(query.q.as_deref().unwrap_or("")),
            |id| ResearchSearch::Identity(id.into()),
        );
        let limit = query.limit.unwrap_or(10).clamp(1, 20);
        // The cursor first traverses accepted records, then collected versions.
        // Each store filters before pagination; references never consume source slots.
        let (records, collected, next_before) =
            if let Some(id) = paper_focus.as_ref().or(local_focus.as_ref()) {
                let records: Vec<_> = self.ports.records.get(id).await?.into_iter().collect();
                let local_version = local_focus.as_ref().and_then(|_| {
                    focus?
                        .strip_prefix("reference:")?
                        .split(':')
                        .nth(1)?
                        .parse::<u32>()
                        .ok()
                });
                let collected: Vec<_> = if records.is_empty() {
                    self.ports
                        .collected
                        .get(id.as_str(), local_version)
                        .await?
                        .into_iter()
                        .collect()
                } else {
                    vec![]
                };
                if paper_focus.is_some() && records.is_empty() && collected.is_empty() {
                    return Err(CoreError::not_found("paper", id.as_str()));
                }
                (records, collected, None)
            } else if let Some(cursor) = query
                .before
                .as_deref()
                .and_then(|v| v.strip_prefix("collection:"))
            {
                let page = self
                    .ports
                    .collected
                    .search_research(
                        &selection,
                        limit,
                        (!cursor.is_empty()).then(|| cursor.into()),
                    )
                    .await?;
                (
                    vec![],
                    page.items,
                    page.next_before.map(|v| format!("collection:{v}")),
                )
            } else {
                let page = self
                    .ports
                    .records
                    .search_research(&selection, limit, query.before.clone())
                    .await?;
                if page.next_before.is_some() {
                    (page.items, vec![], page.next_before)
                } else if page.items.len() == limit as usize {
                    // Keep the boundary as an explicit cursor so this page stays bounded.
                    let more = self
                        .ports
                        .collected
                        .search_research(&selection, 1, None)
                        .await?;
                    let cursor = (!more.items.is_empty()).then(|| "collection:".to_owned());
                    (page.items, vec![], cursor)
                } else {
                    let collected = self
                        .ports
                        .collected
                        .search_research(&selection, limit - page.items.len() as u32, None)
                        .await?;
                    (
                        page.items,
                        collected.items,
                        collected.next_before.map(|v| format!("collection:{v}")),
                    )
                }
            };
        let mut nodes = BTreeMap::new();
        let mut edges = vec![];
        let mut coverage = vec![];
        let mut sources = vec![];
        for record in records {
            let s = match self.accepted(&record).await {
                Ok(s) => s,
                Err(CoreError::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            };
            if s.purpose != SubmissionPurpose::Publication
                || s.analysis_visibility != Visibility::Public
            {
                if paper_focus.is_some() {
                    return Err(CoreError::not_found("paper", record.id.as_str()));
                }
                continue;
            }
            let Some(version) = s.current_version() else {
                continue;
            };
            sources.push((
                record.id.clone(),
                version.number,
                manuscript_node(&record.id, &s),
                selection.matches_paper(&record.id, &s),
                version.bibliography.clone(),
                vec![],
            ));
        }
        for collected in collected {
            let record = collected.graph_record();
            let version = collected.delivery.version;
            let paper = ResearchNode {
                id: format!("paper:{record}"),
                kind: ResearchNodeKind::Manuscript,
                title: collected.delivery.paper.title.clone(),
                authors: collected
                    .delivery
                    .paper
                    .authors
                    .iter()
                    .map(|a| a.name.clone())
                    .collect(),
                url: None,
                record: Some(record.clone()),
                version: Some(version),
            };
            let limitations = collected
                .source_index
                .as_ref()
                .map_or_else(Vec::new, |i| i.limitations.clone());
            sources.push((
                record,
                version,
                paper,
                collected.matches_research_paper(&selection),
                collected.indexed_bibliography().cloned(),
                limitations,
            ));
        }
        for (record, version, paper, paper_matches, bibliography, mut limitations) in sources {
            if bibliography.is_none() {
                limitations.push(
                    "This source version has not been indexed; consult its original references."
                        .into(),
                );
            }
            if let Some(b) = &bibliography {
                limitations.extend(b.limitations.clone());
            }
            // Coverage applies to this page's public accepted inputs, even with no matches.
            coverage.push(GraphCoverage {
                record: record.clone(),
                version,
                method: bibliography.as_ref().map(|b| b.method.clone()),
                limitations,
            });
            if paper_focus.is_some() || (focus.is_none() && paper_matches) {
                nodes.insert(paper.id.clone(), paper.clone());
            }
            for r in bibliography
                .as_ref()
                .into_iter()
                .flat_map(|b| &b.references)
            {
                let id = reference_identity(&record, version, r);
                let matches = if let Some(focus) = focus {
                    focus == id || focus == paper.id
                } else {
                    paper_matches || selection.matches_reference(&record, version, r)
                };
                if !matches {
                    continue;
                }
                nodes.insert(paper.id.clone(), paper.clone());
                nodes.entry(id.clone()).or_insert_with(|| ResearchNode {
                    id: id.clone(),
                    kind: ResearchNodeKind::Literature,
                    title: r.display_text(),
                    authors: r.authors.clone(),
                    url: reference_url(r),
                    record: None,
                    version: None,
                });
                edges.push(CitationEdge {
                    from: paper.id.clone(),
                    to: id,
                    relation: ResearchRelation::Cites,
                    record: record.clone(),
                    version,
                    reference_key: r.key.clone(),
                    display_text: r.display_text(),
                    source_location: r.source_location.clone(),
                    title: r.title.clone(),
                    authors: r.authors.clone(),
                    year: r.year.clone(),
                    raw: r.raw.clone(),
                    raw_truncated: r.raw_truncated,
                });
            }
        }
        Ok(ResearchGraph {nodes:nodes.into_values().collect(),edges,coverage,next_before,
            scope:"Source-declared citations in a bounded page of matching public Mathesis publications and trusted collected versions. Search and exact-identifier focus select matches before pagination; unrelated publications do not consume result pages. Literature identities are shared by exact supported identifiers. Legacy, PDF and unsupported sources may be unindexed; this is not exhaustive literature coverage or a proof-dependency graph.".into()})
    }
}
