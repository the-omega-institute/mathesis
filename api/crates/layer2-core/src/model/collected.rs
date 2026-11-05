//! Trusted collection intake is distinct from independent venue acceptance.
use super::{
    Author, BlobRef, PaperSearch, PlainLanguageText, Source, SubmissionKind, require_text,
    validate_msc,
};
use crate::{CoreError, CoreResult, ids::PersonId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectedProblem {
    pub name: String,
    pub source: Source,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectedLean {
    pub url: String,
    pub revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectedMetadata {
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub authors: Vec<Author>,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plain_language: Option<PlainLanguageText>,
    #[serde(default)]
    pub msc: Vec<String>,
    #[serde(default)]
    pub problems: Vec<CollectedProblem>,
    #[serde(default)]
    pub lean: Option<CollectedLean>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionReview {
    pub verdict: String,
    pub pdf_sha256: String,
    pub source_sha256: String,
    /// Exact producer review evidence; never projected into a reader response.
    pub record: serde_json::Value,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreReviewedDelivery {
    pub schema: String,
    pub producer: String,
    pub work_id: String,
    pub version: u32,
    pub previous_version: Option<u32>,
    pub authorize_publication: bool,
    pub paper: CollectedMetadata,
    pub review: CollectionReview,
}
impl PreReviewedDelivery {
    pub fn validate(&self) -> CoreResult<()> {
        if self.schema != "mathesis.pre-reviewed-delivery.v1" || !self.authorize_publication {
            return Err(CoreError::invalid(
                "require pre-reviewed delivery schema and explicit publication authorization",
            ));
        }
        for value in [&self.producer, &self.work_id] {
            if value.is_empty()
                || value.len() > 160
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err(CoreError::invalid(
                    "producer and work_id require lowercase ASCII slugs",
                ));
            }
        }
        // Ordinary venue records use WP- identifiers, so collection slugs cannot shadow them.
        if self.version == 0
            || self
                .previous_version
                .is_some_and(|v| v == 0 || v >= self.version)
        {
            return Err(CoreError::invalid(
                "version must advance its exact predecessor",
            ));
        }
        require_text("title", &self.paper.title, 1000)?;
        require_text("abstract", &self.paper.abstract_text, 20_000)?;
        super::paper::validate_authors(&self.paper.authors)?;
        if self.paper.authors.is_empty() || self.paper.authors.iter().any(|a| a.person.is_some()) {
            return Err(CoreError::invalid(
                "supply ordered authors without asserting Mathesis account ownership",
            ));
        }
        validate_msc(&self.paper.msc)?;
        if self.paper.license != "CC-BY-4.0" {
            return Err(CoreError::invalid(
                "this collection channel requires CC-BY-4.0",
            ));
        }
        if let Some(text) = &self.paper.plain_language {
            text.validate()?;
        }
        if self.paper.problems.len() > 100 {
            return Err(CoreError::invalid("at most 100 source questions"));
        }
        for problem in &self.paper.problems {
            require_text("source question", &problem.name, 2000)?;
            problem.source.validate()?;
        }
        if let Some(lean) = &self.paper.lean {
            let commit = &lean.revision;
            let prefix = "https://github.com/";
            if commit.len() != 40
                || !commit.bytes().all(|c| c.is_ascii_hexdigit())
                || !lean.url.starts_with(prefix)
                || !lean.url.contains(&format!("/blob/{commit}/"))
                || !lean.url.split('#').next().unwrap_or("").ends_with(".lean")
            {
                return Err(CoreError::invalid(
                    "Lean evidence requires a public GitHub .lean file pinned to a full commit",
                ));
            }
            super::manuscript::public_link(&lean.url)?;
        }
        let r = &self.review;
        if r.verdict != "PASS"
            || r.record.as_object().is_none_or(|v| v.is_empty())
            || [&r.pdf_sha256, &r.source_sha256].iter().any(|s| {
                s.len() != 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
        {
            return Err(CoreError::invalid(
                "require a PASS review, exact lowercase SHA-256 file bindings and a nonempty review record",
            ));
        }
        if serde_json::to_vec(self)
            .map_err(|_| CoreError::invalid("invalid metadata"))?
            .len()
            > 128 * 1024
        {
            return Err(CoreError::invalid("collection metadata exceeds 128 KiB"));
        }
        Ok(())
    }
}
/// Immutable versions; the unique work/predecessor pair prevents concurrent forks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedVersion {
    pub id: String,
    pub owner: PersonId,
    pub delivery: PreReviewedDelivery,
    pub request_digest: String,
    pub pdf: BlobRef,
    pub source: BlobRef,
    pub published_on: DateTime<Utc>,
    /// Independently generated reading guide; never changes the reviewed delivery.
    #[serde(default)]
    pub explanation: CollectedExplanation,
    /// Deterministic source observations, independent of delivery and review.
    #[serde(default)]
    pub source_index: Option<CollectedSourceIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedSourceIndex {
    pub revision: u64,
    pub source_sha256: String,
    pub bibliography: Option<super::Bibliography>,
    pub limitations: Vec<String>,
}

impl CollectedVersion {
    pub fn graph_record(&self) -> crate::ids::RecordId {
        self.delivery.work_id.clone().into()
    }

    pub fn matches_research_paper(&self, search: &super::ResearchSearch) -> bool {
        match search {
            super::ResearchSearch::All => true,
            super::ResearchSearch::Identity(id) => {
                id == &format!("paper:{}", self.delivery.work_id)
            }
            super::ResearchSearch::Text(q) => std::iter::once(self.delivery.work_id.as_str())
                .chain([self.delivery.paper.title.as_str()])
                .chain(self.delivery.paper.authors.iter().map(|a| a.name.as_str()))
                .any(|v| v.to_lowercase().contains(q)),
        }
    }

    pub fn indexed_bibliography(&self) -> Option<&super::Bibliography> {
        self.source_index
            .as_ref()
            .filter(|index| index.source_sha256 == self.source.sha256)
            .and_then(|index| index.bibliography.as_ref())
    }

    pub fn matches_research(&self, search: &super::ResearchSearch) -> bool {
        self.matches_research_paper(search)
            || self.indexed_bibliography().is_some_and(|b| {
                b.references.iter().any(|r| {
                    search.matches_reference(&self.graph_record(), self.delivery.version, r)
                })
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedExplanation {
    pub revision: u64,
    pub text: Option<PlainLanguageText>,
    /// Private, durable CMA conversation and cleanup state, omitted from public projections.
    pub progress: serde_json::Value,
}
impl CollectedVersion {
    pub fn matches(&self, q: &PaperSearch) -> bool {
        let p = &self.delivery.paper;
        if q.kind.is_some_and(|k| k != SubmissionKind::Note)
            || q.msc
                .as_ref()
                .is_some_and(|m| !p.msc.iter().any(|c| c.starts_with(&m.to_uppercase())))
        {
            return false;
        }
        let authors = p
            .authors
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if q.author
            .as_ref()
            .is_some_and(|a| !authors.contains(&a.to_lowercase()))
        {
            return false;
        }
        let text = format!(
            "{} {} {} {} {}",
            self.delivery.work_id,
            p.title,
            p.abstract_text,
            authors,
            p.problems
                .iter()
                .map(|p| format!("{} {}", p.name, p.source.locator))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase();
        q.q.as_ref().is_none_or(|q| {
            q.split_whitespace()
                .all(|w| text.contains(&w.to_lowercase()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delivery() -> PreReviewedDelivery {
        serde_json::from_value(serde_json::json!({
            "schema": "mathesis.pre-reviewed-delivery.v1",
            "producer": "outreach",
            "work_id": "gap-note",
            "version": 1,
            "previous_version": null,
            "authorize_publication": true,
            "paper": {
                "title": "A note on gaps",
                "abstract": "We prove a bound on gaps.",
                "authors": [{"name": "A. Author"}],
                "license": "CC-BY-4.0",
                "msc": ["11B"],
                "problems": [{"name": "The gap question", "source": {"kind": "arxiv", "locator": "2401.12345"}}],
                "lean": {"url": "https://github.com/example/project/blob/0123456789012345678901234567890123456789/Gap.lean", "revision": "0123456789012345678901234567890123456789"}
            },
            "review": {"verdict": "PASS", "pdf_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "source_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "record": {"checked": true}}
        })).unwrap()
    }

    #[test]
    fn trusted_delivery_requires_explicit_review_bindings() {
        let value = delivery();
        value.validate().unwrap();
        let mut changed = value.clone();
        changed.review.verdict = "PENDING".into();
        assert!(changed.validate().is_err());
    }

    #[test]
    fn collection_search_matches_title_author_and_source() {
        let d = delivery();
        let v = CollectedVersion {
            source_index: None,
            explanation: CollectedExplanation::default(),
            id: "id".into(),
            owner: PersonId::from("operator"),
            delivery: d,
            request_digest: "digest".into(),
            pdf: BlobRef {
                id: "pdf".into(),
                bytes: 1,
                sha256: "a".into(),
            },
            source: BlobRef {
                id: "source".into(),
                bytes: 1,
                sha256: "b".into(),
            },
            published_on: Utc::now(),
        };
        let mut q = PaperSearch {
            q: Some("gap-note 2401.12345".into()),
            ..Default::default()
        };
        q.validate().unwrap();
        assert!(v.matches(&q));
        q.q = Some("unrelated".into());
        assert!(!v.matches(&q));
    }
}
