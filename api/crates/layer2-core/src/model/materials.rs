//! Replaceable public reading/media links, separate from manuscript evidence.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

pub const MATERIALS_INDEX_MAX_BYTES: usize = 1024 * 1024;
pub const MATERIALS_INDEX_MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialsKind {
    Collected,
    Record,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperMaterials {
    pub version: u32,
    pub repository_url: String,
    pub site_url: String,
    pub reading: BTreeMap<String, String>,
    pub slides: Vec<MaterialSlides>,
    pub visualizations: Vec<MaterialVisualization>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSlides {
    pub id: String,
    pub language: String,
    pub pdf_url: Option<String>,
    pub notes_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialVisualization {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialsIndexEntry {
    pub kind: MaterialsKind,
    pub id: String,
    #[serde(flatten)]
    pub materials: PaperMaterials,
}

/// One revision-fenced sidecar, replaced atomically after a successful index read.
/// Entries retain their exact version; no manuscript aggregate is rewritten.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialsSidecar {
    pub revision: u64,
    pub entries: Vec<MaterialsIndexEntry>,
}

fn token(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        && s != "."
        && s != ".."
}

impl MaterialsIndexEntry {
    pub fn validate(&self) -> CoreResult<()> {
        let m = &self.materials;
        let repo = m
            .repository_url
            .strip_prefix("https://github.com/mathesis-papers/")
            .filter(|s| token(s, 100))
            .ok_or_else(|| CoreError::invalid("invalid materials repository"))?;
        let site = format!("https://papers.trureturing.org/{repo}/");
        if !token(&self.id, 200)
            || (self.kind == MaterialsKind::Record && !self.id.starts_with("WP-"))
            || m.version == 0
            || m.site_url != site
            || m.reading.len() > 64
            || m.slides.len() > 64
            || m.visualizations.len() > 64
        {
            return Err(CoreError::invalid("invalid materials identity or bounds"));
        }
        let check_url = |raw: &str| -> CoreResult<()> {
            let url =
                url::Url::parse(raw).map_err(|_| CoreError::invalid("invalid materials URL"))?;
            let within = |s: &str| {
                s.starts_with(&site)
                    || s == m.repository_url
                    || s.starts_with(&format!("{}/", m.repository_url))
            };
            if raw.len() > 2048
                || !within(raw)
                || !within(url.as_str())
                || url.scheme() != "https"
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || raw
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control() || c == '%' || c == '\\')
                || raw.split(['/', '#']).any(|s| s == "." || s == "..")
            {
                return Err(CoreError::invalid(
                    "materials URL leaves its paper repository",
                ));
            }
            Ok(())
        };
        for (language, url) in &m.reading {
            if !token(language, 32) {
                return Err(CoreError::invalid("invalid materials language"));
            }
            check_url(url)?;
        }
        let mut slides = BTreeSet::new();
        for item in &m.slides {
            if !token(&item.id, 200)
                || !token(&item.language, 32)
                || !slides.insert((&item.id, &item.language))
                || (item.pdf_url.is_none() && item.notes_url.is_none())
            {
                return Err(CoreError::invalid("invalid materials slides"));
            }
            for url in [&item.pdf_url, &item.notes_url].into_iter().flatten() {
                check_url(url)?;
            }
        }
        let mut visualizations = BTreeSet::new();
        for item in &m.visualizations {
            if !token(&item.id, 200) || !visualizations.insert(&item.id) {
                return Err(CoreError::invalid("invalid materials visualization"));
            }
            check_url(&item.url)?;
        }
        Ok(())
    }
}

/// Envelope failures preserve the previous sidecar. Bad rows are skipped;
/// the last valid occurrence of an exact identity wins during reconciliation.
pub fn parse_materials_index(bytes: &[u8]) -> CoreResult<Vec<MaterialsIndexEntry>> {
    #[derive(Deserialize)]
    struct Document {
        schema: String,
        generated_at: String,
        entries: Vec<serde_json::Value>,
    }
    if bytes.len() > MATERIALS_INDEX_MAX_BYTES {
        return Err(CoreError::invalid("materials index exceeds its size bound"));
    }
    let document: Document = serde_json::from_slice(bytes)
        .map_err(|_| CoreError::invalid("materials index is not valid JSON"))?;
    if document.schema != "mathesis.materials-index.v1"
        || chrono::DateTime::parse_from_rfc3339(&document.generated_at).is_err()
        || document.entries.len() > MATERIALS_INDEX_MAX_ENTRIES
    {
        return Err(CoreError::invalid(
            "materials index schema, time or entry bound is invalid",
        ));
    }
    Ok(document
        .entries
        .into_iter()
        .filter_map(|value| {
            let entry: MaterialsIndexEntry = serde_json::from_value(value).ok()?;
            entry.validate().ok()?;
            Some(entry)
        })
        .collect())
}
