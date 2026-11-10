//! Author-supplied formalization sources, distinct from verifier receipts.
use super::{BlobRef, Source, require_text};
use crate::{CoreError, CoreResult, ids::PersonId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewLeanDeposit {
    /// Lean modules keyed by dotted module name (Research.Lemmas → Research/Lemmas.lean).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, String>,
    /// Inert project configuration and documentation, never executed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub project_files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formalizers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub statement: String,
    pub source_reference: Source,
    pub toolchain: String,
    pub license: String,
    #[serde(default)]
    pub solution: String,
}
impl NewLeanDeposit {
    pub fn validate(&self) -> CoreResult<()> {
        require_text("title", &self.title, 1000)?;
        require_text("statement", &self.statement, 20000)?;
        require_text("toolchain", &self.toolchain, 200)?;
        require_text("license", &self.license, 200)?;
        if self.authors.is_empty() || self.authors.len() > 50 {
            return Err(CoreError::invalid("supply 1 to 50 original authors"));
        }
        for author in &self.authors {
            require_text("author", author, 200)?;
        }
        require_text("source locator", &self.source_reference.locator, 2000)?;
        self.source_reference.validate()?;
        if (self.solution.trim().is_empty() && self.modules.is_empty())
            || self.solution.len() > 1_048_576
        {
            return Err(CoreError::invalid("Lean source must be 1 byte to 1 MB"));
        }
        validate_lean_modules(&self.solution, &self.modules)?;
        validate_project_files(&self.project_files)?;
        if self.formalizers.len() > 50 {
            return Err(CoreError::invalid("supply at most 50 formalizers"));
        }
        for name in &self.formalizers {
            require_text("formalizer", name, 200)?;
        }
        if self.source_revision.as_ref().is_some_and(|r| {
            ![40, 64].contains(&r.len()) || !r.bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            return Err(CoreError::invalid(
                "source revision must be a full 40 or 64 digit Git commit hash",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanDeposit {
    /// Lean modules keyed by dotted module name (Research.Lemmas → Research/Lemmas.lean).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, BlobRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub project_files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formalizers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,

    pub id: String,
    pub owner: PersonId,
    pub title: String,
    pub authors: Vec<String>,
    pub statement: String,
    pub source_reference: Source,
    pub toolchain: String,
    pub license: String,
    pub source: BlobRef,
    pub submitted_at: DateTime<Utc>,
    pub attempts: Vec<String>,
    pub revision: u64,
}

/// Reported provenance travels with a verified proof; it is not a verified authorship claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationAttribution {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formalizers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    pub deposit: String,
    pub authors: Vec<String>,
    pub source_reference: Source,
    pub license: String,
}

/// Dotted module names preserve directories without allowing traversal or trusted roots.
pub fn validate_lean_modules(solution: &str, modules: &BTreeMap<String, String>) -> CoreResult<()> {
    validate_lean_modules_with(solution, modules, &[])
}
/// `libraries` are the verifier's prepared libraries. Their modules are imported,
/// never resubmitted: a supporting module cannot be named below one of their
/// roots. Without a library this is exactly `validate_lean_modules`.
pub fn validate_lean_modules_with(
    solution: &str,
    modules: &BTreeMap<String, String>,
    libraries: &[super::PreparedLibrary],
) -> CoreResult<()> {
    if modules.len() > 512
        || solution.len() + modules.values().map(String::len).sum::<usize>() > 8 * 1_048_576
    {
        return Err(CoreError::invalid(
            "at most 512 supporting modules and 8 MB total Lean source",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (name, source) in modules {
        if !valid_module_name(name)
            || !seen.insert(name.to_ascii_lowercase())
            || source.trim().is_empty()
        {
            return Err(CoreError::invalid(
                "supporting modules need distinct dotted Lean module names, nonempty sources, and cannot replace trusted modules",
            ));
        }
        // Case-insensitive, like every reserved root.
        let root = name.split('.').next().unwrap_or_default();
        if let Some(library) = libraries
            .iter()
            .find(|library| library.roots.iter().any(|r| r.eq_ignore_ascii_case(root)))
        {
            return Err(CoreError::invalid(format!(
                "supporting module {name} is named below a root of the prepared library {}; import the library module instead of submitting it",
                library.name
            )));
        }
    }
    Ok(())
}
/// Legacy single-file hashes remain stable. Project hashes bind every module.
pub fn lean_solution_digest(solution: &str, modules: &BTreeMap<String, String>) -> String {
    let bytes = if modules.is_empty() {
        solution.as_bytes().to_vec()
    } else {
        serde_json::to_vec(&("mathesis-project-v1", solution, modules))
            .expect("source serialization")
    };
    format!("{:x}", Sha256::digest(bytes))
}

/// Case-insensitive roots are reserved on every host, including macOS.
pub fn valid_module_name(name: &str) -> bool {
    name.len() <= 240
        && name.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 64
                && part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        && ![
            "target", "solution", "check", "mathlib", "init", "lean", "std", "lake",
        ]
        .contains(
            &name
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
        )
}
fn validate_project_files(files: &BTreeMap<String, String>) -> CoreResult<()> {
    if files.len() > 16 || files.values().map(String::len).sum::<usize>() > 1_048_576 {
        return Err(CoreError::invalid(
            "project configuration exceeds 16 files or 1 MB",
        ));
    }
    for name in files.keys() {
        if ![
            "lean-toolchain",
            "lake-manifest.json",
            "lakefile.lean",
            "lakefile.toml",
            "README.md",
            "LICENSE",
            "LICENSE.md",
            "LICENSE.txt",
        ]
        .contains(&name.as_str())
        {
            return Err(CoreError::invalid(format!(
                "unsupported project configuration: {name}"
            )));
        }
    }
    Ok(())
}
