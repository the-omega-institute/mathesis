use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paper::Author;
use crate::{
    ids::{PersonId, RecordId, SubmissionId},
    policy::AdmissionBasis,
};

/// DOI agency used for Mathesis' own publication identifier. This is kept
/// separate from `Submission.doi`, which is an identifier supplied by the
/// author for an already-published version such as an arXiv or Zenodo item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoiAgency {
    Crossref,
    DataCite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationDoiStatus {
    Pending,
    Registered,
    Failed,
}

/// Durable state for registering the accepted publication with a DOI agency.
/// The metadata hash fences a deposit to the immutable `Record.publication`
/// snapshot, while retry information lets a worker recover from a transient
/// agency failure without changing the public bibliographic identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationDoi {
    pub agency: DoiAgency,
    pub status: PublicationDoiStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_sha256: Option<String>,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl PublicationDoi {
    /// Create an unregistered Crossref record. The deposit worker fills in the
    /// metadata hash and DOI only after it has built the deposit from the
    /// immutable publication snapshot.
    pub fn pending_crossref() -> Self {
        Self {
            agency: DoiAgency::Crossref,
            status: PublicationDoiStatus::Pending,
            doi: None,
            metadata_sha256: None,
            attempts: 0,
            registered_at: None,
            last_error: None,
        }
    }
}

/// The acceptance of a paper. The public page is assembled from the
/// submission at read time, so formalizations added later appear on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    #[serde(default)]
    pub kind: super::SubmissionKind,
    /// `WP-<year>-<4-digit sequence>`.
    pub id: RecordId,
    /// Immutable publication input; legacy inputs are preserved on the submission when revised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication: Option<Box<super::Submission>>,
    /// Mathesis' own DOI registration state, distinct from the author's DOI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_doi: Option<PublicationDoi>,
    pub submission: SubmissionId,
    pub title: String,
    pub authors: Vec<Author>,
    pub basis: AdmissionBasis,
    pub accepted_by: PersonId,
    pub accepted_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_crossref_registration_is_distinct_from_author_doi() {
        let registration = PublicationDoi::pending_crossref();
        assert_eq!(registration.agency, DoiAgency::Crossref);
        assert_eq!(registration.status, PublicationDoiStatus::Pending);
        assert_eq!(registration.doi, None);
        assert_eq!(registration.attempts, 0);
        let json = serde_json::to_value(&registration).unwrap();
        assert_eq!(json["agency"], "crossref");
        assert_eq!(json["status"], "pending");
        assert!(json.get("doi").is_none());
    }
}
