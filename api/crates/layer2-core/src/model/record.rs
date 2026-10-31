use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paper::Author;
use crate::{
    ids::{PersonId, RecordId, SubmissionId},
    policy::AdmissionBasis,
};

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
    pub submission: SubmissionId,
    pub title: String,
    pub authors: Vec<Author>,
    pub basis: AdmissionBasis,
    pub accepted_by: PersonId,
    pub accepted_at: DateTime<Utc>,
}

impl Record {
    /// Immutable mathematical input, with the same legacy fallback as public
    /// paper reads. Current drafts may supply visibility, never source metadata.
    pub fn publication_input<'a>(
        &'a self,
        current: &'a super::Submission,
    ) -> Option<&'a super::Submission> {
        self.publication.as_deref()
            .or_else(|| current.published_progress.as_ref()?.publication.as_deref())
            .or_else(|| matches!(&current.status, super::SubmissionStatus::Accepted { record } if record == &self.id).then_some(current))
    }
}
