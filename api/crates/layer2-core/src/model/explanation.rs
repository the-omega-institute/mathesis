//! Readable exposition of one manuscript version, never a review or proof receipt.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlainLanguageText {
    pub en: String,
    pub zh: String,
}

impl PlainLanguageText {
    pub fn validate(&self) -> crate::CoreResult<()> {
        for (language, text) in [("English", &self.en), ("Chinese", &self.zh)] {
            super::require_text(&format!("{language} explanation"), text, 12_000)?;
        }
        if !self.en.chars().any(|c| c.is_ascii_alphabetic())
            || !self
                .zh
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
        {
            return Err(crate::CoreError::invalid(
                "provide an English and a Chinese explanation",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperExplanation {
    pub version: u32,
    pub claims_revision: u64,
    pub source_sha256: String,
    pub text: PlainLanguageText,
    pub prepared_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPaperExplanation {
    pub version: u32,
    pub claims_revision: u64,
    pub source_sha256: String,
    pub generation: u32,
    pub text: PlainLanguageText,
}
