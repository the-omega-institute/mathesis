//! Source observations, not authenticated metadata or proof dependencies.
use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult, ids::RecordId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bibliography {
    pub method: String,
    pub references: Vec<BibliographicReference>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibliographicReference {
    pub key: String,
    pub title: Option<String>,
    /// Ordered source-supplied names, independent of accounts and uploaders.
    pub authors: Vec<String>,
    pub year: Option<String>,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub url: Option<String>,
    pub source_location: String,
    pub raw: String,
    pub raw_truncated: bool,
}

impl Bibliography {
    pub fn validate(&self) -> CoreResult<()> {
        let mut keys = std::collections::BTreeSet::new();
        if self.method.len() > 100 || self.references.len() > 100 || self.limitations.len() > 20 {
            return Err(CoreError::invalid("bibliography exceeds extraction bounds"));
        }
        for r in &self.references {
            if r.key.is_empty()
                || r.key.len() > 200
                || r.key.chars().any(char::is_control)
                || !keys.insert(&r.key)
                || r.raw.len() > 4096
                || r.source_location.len() > 1000
                || r.authors.len() > 50
                || r.authors.iter().any(|a| a.len() > 300)
                || [&r.title, &r.year, &r.doi, &r.arxiv, &r.url]
                    .into_iter()
                    .flatten()
                    .any(|v| v.len() > 1000)
            {
                return Err(CoreError::invalid(
                    "bibliographic reference exceeds extraction bounds",
                ));
            }
        }
        if self.limitations.iter().any(|s| s.len() > 1000) {
            return Err(CoreError::invalid("bibliography limitations exceed bounds"));
        }
        Ok(())
    }
}

/// Exact supported identifiers only. DOI case is insignificant; arXiv versions
/// remain distinct. No DOI/arXiv alias, URL, title or person-name inference.
pub fn reference_identity(record: &RecordId, version: u32, r: &BibliographicReference) -> String {
    if let Some(doi) = r.doi.as_deref().and_then(canonical_reference_doi) {
        return format!("doi:{doi}");
    }
    if let Some(arxiv) = r.arxiv.as_deref().and_then(canonical_reference_arxiv) {
        return format!("arxiv:{arxiv}");
    }
    // A single literal resolver URL is also explicit identifier evidence. Other
    // web URLs never define shared work identity.
    if let Some(url) = r.url.as_deref() {
        if [
            "https://doi.org/",
            "http://doi.org/",
            "https://dx.doi.org/",
            "http://dx.doi.org/",
        ]
        .iter()
        .any(|p| url.starts_with(p))
            && let Some(doi) = canonical_reference_doi(url)
        {
            return format!("doi:{doi}");
        }
        if (url.starts_with("https://arxiv.org/abs/") || url.starts_with("http://arxiv.org/abs/"))
            && let Some(arxiv) = canonical_reference_arxiv(url)
        {
            return format!("arxiv:{arxiv}");
        }
    }
    format!("reference:{record}:{version}:{}", r.key)
}

pub fn canonical_reference_doi(value: &str) -> Option<String> {
    let doi = super::normalise_doi(value);
    let (prefix, suffix) = doi.split_once('/')?;
    let registrant = prefix.strip_prefix("10.")?;
    ((4..=9).contains(&registrant.len())
        && registrant.bytes().all(|b| b.is_ascii_digit())
        && !suffix.is_empty()
        && doi.len() <= 256
        && doi
            .bytes()
            .all(|b| b.is_ascii_graphic() && !b"\\{}\"<>".contains(&b)))
    .then(|| doi.to_ascii_lowercase())
}

pub fn canonical_reference_arxiv(value: &str) -> Option<String> {
    let value = value.trim();
    let id = [
        "https://arxiv.org/abs/",
        "http://arxiv.org/abs/",
        "arXiv:",
        "arxiv:",
    ]
    .iter()
    .find_map(|p| value.strip_prefix(p))
    .unwrap_or(value);
    let tail = id.rsplit('/').next().unwrap_or(id);
    let version_at = tail.rfind('v').map(|p| id.len() - tail.len() + p);
    let (base, version) = version_at.map_or((id, None), |p| (&id[..p], Some(&id[p + 1..])));
    if version.is_some_and(|v| {
        v.is_empty() || v.starts_with('0') || !v.bytes().all(|b| b.is_ascii_digit())
    }) {
        return None;
    }
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let modern = base
        .split_once('.')
        .is_some_and(|(a, b)| a.len() == 4 && (4..=5).contains(&b.len()) && digits(a) && digits(b));
    let old = base.split_once('/').is_some_and(|(a, b)| {
        !a.is_empty()
            && a.bytes()
                .all(|c| c.is_ascii_alphabetic() || matches!(c, b'-' | b'.'))
            && b.len() == 7
            && digits(b)
    });
    ((modern || old) && id.len() <= 100).then(|| id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolver_urls_are_explicit_evidence_and_names_or_urls_never_supply_fuzzy_identity() {
        let mut r = BibliographicReference {
            key: "old".into(),
            title: Some("A title".into()),
            authors: vec!["A. Earlier".into()],
            year: None,
            doi: None,
            arxiv: None,
            url: Some("https://doi.org/10.1234/OLD".into()),
            source_location: "main.bbl".into(),
            raw: String::new(),
            raw_truncated: false,
        };
        let record = RecordId::from("WP-2026-0001");
        assert_eq!(reference_identity(&record, 1, &r), "doi:10.1234/old");
        r.url = Some("https://example.org/same-title".into());
        assert_eq!(
            reference_identity(&record, 1, &r),
            "reference:WP-2026-0001:1:old"
        );
        assert_ne!(
            reference_identity(&record, 1, &r),
            reference_identity(&record, 2, &r)
        );
        r.arxiv = Some("math/0301234v2".into());
        assert_eq!(reference_identity(&record, 1, &r), "arxiv:math/0301234v2");
        r.doi = Some("10.1234/old".into());
        assert_eq!(reference_identity(&record, 1, &r), "doi:10.1234/old");
    }
    #[test]
    fn selection_respects_identifier_precedence_and_readable_prose_preserves_evidence() {
        let record = RecordId::from("WP-2026-0001");
        let mut r = BibliographicReference {
            key: "internalKey17".into(), title: None, authors: vec![], year: None,
            doi: Some("10.1234/old".into()), arxiv: Some("math/0301234v2".into()), url: None,
            source_location: "main.bbl".into(),
            raw: r"\bibitem [Earlier 1901] {internalKey17} Émilie Earlier, \emph{Old result} (1901). \doi{10.1234/old}".into(),
            raw_truncated: false,
        };
        let original = serde_json::to_string(&r).unwrap();
        assert_eq!(
            r.display_text(),
            "Émilie Earlier, Old result (1901). 10.1234/old"
        );
        assert_eq!(serde_json::to_string(&r).unwrap(), original);
        assert!(r.title.is_none() && r.authors.is_empty());
        assert!(ResearchSearch::from_text("DOI: 10.1234/OLD").matches_reference(&record, 1, &r));
        assert!(!ResearchSearch::from_text("math/0301234v2").matches_reference(&record, 1, &r));
        assert!(
            !ResearchSearch::Identity("doi:10.1234/old-extra".into())
                .matches_reference(&record, 1, &r)
        );
        r.doi = None;
        assert!(ResearchSearch::from_text("math/0301234v2").matches_reference(&record, 1, &r));
        r.title = Some("Structured title stays intact".into());
        r.authors = (0..28)
            .map(|n| format!("Émilie{n} Givenname Middlename Surname"))
            .collect();
        assert_eq!(r.display_text(), "Structured title stays intact");
        assert_eq!(r.authors.len(), 28);
        assert!(
            ResearchSearch::from_text("Émilie27 Givenname Middlename Surname")
                .matches_reference(&record, 1, &r)
        );
    }
    #[test]
    fn supported_identity_is_exact_and_versioned() {
        assert_eq!(
            canonical_reference_doi("DOI: 10.1234/AbC"),
            Some("10.1234/abc".into())
        );
        assert_eq!(canonical_reference_doi("10.foo/abc"), None);
        assert_eq!(canonical_reference_doi(r"10.1234/\macro"), None);
        assert_ne!(
            canonical_reference_arxiv("2601.12345v1"),
            canonical_reference_arxiv("2601.12345v2")
        );
        assert_ne!(
            canonical_reference_arxiv("2601.12345"),
            canonical_reference_arxiv("2601.12345v1")
        );
        assert_eq!(
            canonical_reference_arxiv("arXiv:math/0301234v2"),
            Some("math/0301234v2".into())
        );
        assert_eq!(canonical_reference_arxiv("2601.12345v0"), None);
        assert_eq!(
            canonical_reference_arxiv("solv-int/9901234"),
            Some("solv-int/9901234".into())
        );
    }
}

/// Shared selection semantics used by store adapters and public projection.
/// Text searches source-supplied fields; exact identifiers use graph identity.
#[derive(Debug, Clone)]
pub enum ResearchSearch {
    All,
    Text(String),
    Identity(String),
}
impl ResearchSearch {
    pub fn from_text(value: &str) -> Self {
        let value = value.trim();
        if value.is_empty() {
            return Self::All;
        }
        if let Some(id) = canonical_reference_doi(value) {
            return Self::Identity(format!("doi:{id}"));
        }
        if let Some(id) = canonical_reference_arxiv(value.strip_prefix("arxiv:").unwrap_or(value)) {
            return Self::Identity(format!("arxiv:{id}"));
        }
        Self::Text(value.to_lowercase())
    }
    pub fn matches_paper(&self, record: &RecordId, source: &super::Submission) -> bool {
        match self {
            Self::All => true,
            Self::Identity(id) => id == &format!("paper:{record}"),
            Self::Text(q) => std::iter::once(record.as_str())
                .chain([source.title.as_str()])
                .chain(source.authors.iter().map(|a| a.name.as_str()))
                .any(|v| v.to_lowercase().contains(q)),
        }
    }
    pub fn matches_reference(
        &self,
        record: &RecordId,
        version: u32,
        r: &BibliographicReference,
    ) -> bool {
        match self {
            Self::All => true,
            Self::Identity(id) => id == &reference_identity(record, version, r),
            Self::Text(q) => [&r.title, &r.doi, &r.arxiv, &r.url]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .chain(r.authors.iter().map(String::as_str))
                .chain([r.raw.as_str()])
                .any(|v| v.to_lowercase().contains(q)),
        }
    }
    pub fn matches_source(&self, record: &RecordId, source: &super::Submission) -> bool {
        self.matches_paper(record, source)
            || source.current_version().is_some_and(|v| {
                v.bibliography.as_ref().is_some_and(|b| {
                    b.references
                        .iter()
                        .any(|r| self.matches_reference(record, v.number, r))
                })
            })
    }
    pub fn matches_visible_record(
        &self,
        record: &super::Record,
        current: &super::Submission,
    ) -> bool {
        current.analysis_visibility == super::Visibility::Public
            && record.publication_input(current).is_some_and(|s| {
                s.purpose == super::SubmissionPurpose::Publication
                    && self.matches_source(&record.id, s)
            })
    }
}

impl BibliographicReference {
    /// Readable citation prose, without guessing person/title boundaries. Raw
    /// syntax remains evidence. Formatting handles a small literal vocabulary,
    /// not TeX evaluation; unresolved database observations receive a neutral label.
    pub fn display_text(&self) -> String {
        if let Some(title) = &self.title
            && !title.trim().is_empty()
        {
            return title.clone();
        }
        fn group(s: &str, open: char, close: char) -> Option<(&str, &str)> {
            let s = s.trim_start();
            let body = s.strip_prefix(open)?;
            let mut depth = 1;
            let mut escaped = false;
            for (i, c) in body.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == open {
                    depth += 1;
                }
                if c == close {
                    depth -= 1;
                }
                if depth == 0 {
                    return Some((&body[..i], &body[i + c.len_utf8()..]));
                }
            }
            None
        }
        let mut prose = self.raw.trim();
        if let Some(tail) = prose.strip_prefix("\\bibitem") {
            let tail = if tail.trim_start().starts_with('[') {
                let Some((_, tail)) = group(tail, '[', ']') else {
                    return "Cited work".into();
                };
                tail
            } else {
                tail
            };
            let Some((_, tail)) = group(tail, '{', '}') else {
                return "Cited work".into();
            };
            prose = tail;
        } else if prose.starts_with('@')
            || prose.starts_with("Candidate in ")
            || prose == "Unresolved literal citation"
        {
            return "Cited work described in the manuscript".into();
        }
        let mut out = String::new();
        while !prose.is_empty() {
            let c = prose.chars().next().unwrap();
            prose = &prose[c.len_utf8()..];
            match c {
                '{' | '}' => {}
                '~' => out.push(' '),
                '\\' => {
                    let end = prose
                        .find(|c: char| !c.is_ascii_alphabetic())
                        .unwrap_or(prose.len());
                    if end == 0 {
                        if let Some(c) = prose.chars().next() {
                            if "%&#_${}".contains(c) {
                                out.push(c);
                            } else {
                                out.push(' ');
                            }
                            prose = &prose[c.len_utf8()..];
                        }
                        continue;
                    }
                    let name = &prose[..end];
                    prose = &prose[end..];
                    // href/bibinfo/bibfield have structural first arguments.
                    if ["href", "bibinfo", "bibfield"].contains(&name) {
                        if let Some((_, tail)) = group(prose, '{', '}') {
                            prose = tail;
                        }
                    } else if ["newblock", "par", "newline", "relax"].contains(&name) {
                        out.push(' ');
                    } else if ![
                        "emph",
                        "textit",
                        "textbf",
                        "textrm",
                        "textnormal",
                        "textsc",
                        "mbox",
                        "url",
                        "doi",
                    ]
                    .contains(&name)
                    {
                        // Unsupported commands/arguments are not display evidence.
                        if let Some((_, tail)) = group(prose, '{', '}') {
                            prose = tail;
                        }
                    }
                }
                _ => out.push(c),
            }
        }
        let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
        if out.is_empty() {
            "Cited work described in the manuscript".into()
        } else {
            out
        }
    }
}
