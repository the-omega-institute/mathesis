use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{App, Upload};
use crate::{CoreError, CoreResult, ids::SubmissionId, model::*, ports::JobKind};

pub(crate) fn validate_manuscript_evidence(
    claims: &[Claim],
    delivery: &ManuscriptVersion,
) -> CoreResult<()> {
    for evidence in &delivery.origin.evidence {
        if !claims.iter().any(|c| {
            c.id.as_str() == evidence.claim_label
                || c.label == evidence.claim_label
                || c.latex_label.as_deref() == Some(evidence.claim_label.as_str())
        }) {
            return Err(CoreError::invalid(format!(
                "evidence refers to an unrecognized manuscript claim: {}",
                evidence.claim_label
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct ManuscriptReceipt {
    pub schema: &'static str,
    pub submission: SubmissionId,
    pub producer: String,
    pub work_id: String,
    pub upstream_version: String,
    pub version: u32,
    pub source_sha256: String,
    pub current_version: u32,
    pub current: bool,
    pub expected_claims_revision: u64,
    pub confirmed_claims_revision: Option<u64>,
    pub status: String,
    pub next_action: &'static str,
    pub pdf_ready: bool,
    pub compile_error: Option<String>,
    pub review: Option<ReviewSummary>,
    pub publication: Option<ManuscriptPublication>,
    /// Exact extraction for the submitting workflow to check and confirm.
    pub extracted: Vec<Claim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManuscriptPublication {
    pub record: crate::ids::RecordId,
    pub version: u32,
    pub public: bool,
}

impl App {
    pub async fn deliver_manuscript(
        &self,
        caller: &Caller,
        mut new: NewManuscriptDelivery,
        upload: Upload,
    ) -> CoreResult<ManuscriptReceipt> {
        if upload.filename.is_empty()
            || upload.filename.len() > 200
            || upload
                .filename
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        {
            return Err(CoreError::invalid(
                "supply a manuscript filename without directories, at most 200 bytes",
            ));
        }
        if upload.bytes.is_empty() || upload.bytes.len() > super::MAX_UPLOAD_BYTES {
            return Err(CoreError::invalid(
                "manuscript source must contain 1 byte to 30 MiB",
            ));
        }
        new.paper.normalise_doi();
        new.validate()?;
        let key = manuscript_key(
            &caller.person,
            &new.producer,
            &new.work_id,
            new.paper.purpose,
        );
        let source_digest = format!("{:x}", Sha256::digest(&upload.bytes));
        let request_digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&new, &upload.filename, source_digest))
                    .map_err(|_| CoreError::invalid("manuscript metadata"))?
            )
        );
        let identity = ManuscriptIdentity {
            key: key.clone(),
            producer: new.producer.clone(),
            work_id: new.work_id.clone(),
        };
        let delivery = ManuscriptVersion {
            version: new.version.clone(),
            request_digest: request_digest.clone(),
            origin: new.origin.clone(),
            confirmation: None,
        };
        let existing = self.ports.submissions.manuscript(&key).await?;
        if let Some(s) = &existing {
            Self::require_submitter(caller, s)?;
            if let Some(v) = s.versions.iter().find(|v| {
                v.delivery
                    .as_ref()
                    .is_some_and(|d| d.version == new.version)
            }) {
                if v.delivery.as_ref().unwrap().request_digest != request_digest {
                    return Err(CoreError::conflict(
                        "this upstream version already has different source or metadata; supply a new version",
                    ));
                }
                self.recover_manuscript_jobs(s, v.number).await?;
                return self.manuscript_receipt(caller, &s.id, Some(v.number)).await;
            }
            if new.previous_version != s.current_version().map(|v| v.number) {
                return Err(CoreError::conflict(
                    "previous_version must match the current manuscript version",
                ));
            }
            if new.paper.kind != s.kind || new.paper.purpose != s.purpose {
                return Err(CoreError::conflict(
                    "a revision cannot change manuscript kind or submission purpose",
                ));
            }
        } else if new.previous_version.is_some() {
            return Err(CoreError::conflict(
                "no manuscript exists for the requested predecessor",
            ));
        }
        let saved = if let Some(s) = existing {
            self.upload_version_with_delivery(
                caller,
                &s.id,
                upload,
                format!("{} version {}", new.producer, new.version),
                new.paper.processing_consent.clone(),
                Some((new.paper, delivery, new.previous_version.unwrap())),
            )
            .await
        } else {
            self.submit_paper_with_delivery(caller, new.paper, upload, Some((identity, delivery)))
                .await
        };
        let saved = match saved {
            Ok(s) => s,
            Err(CoreError::Conflict(_) | CoreError::StaleRevision { .. }) => {
                let s = self
                    .ports
                    .submissions
                    .manuscript(&key)
                    .await?
                    .ok_or_else(|| {
                        CoreError::conflict(
                            "manuscript delivery changed; retry the identical request",
                        )
                    })?;
                if !s.versions.iter().any(|v| {
                    v.delivery.as_ref().is_some_and(|d| {
                        d.version == new.version && d.request_digest == request_digest
                    })
                }) {
                    return Err(CoreError::conflict(
                        "concurrent manuscript delivery differs; read the current receipt before revising",
                    ));
                }
                s
            }
            Err(e) => return Err(e),
        };
        let version = saved
            .versions
            .iter()
            .find(|v| {
                v.delivery
                    .as_ref()
                    .is_some_and(|d| d.version == new.version && d.request_digest == request_digest)
            })
            .ok_or_else(|| CoreError::conflict("manuscript delivery identity changed"))?
            .number;
        self.recover_manuscript_jobs(&saved, version).await?;
        self.manuscript_receipt(caller, &saved.id, Some(version))
            .await
    }

    pub async fn find_manuscript_delivery(
        &self,
        caller: &Caller,
        producer: &str,
        work_id: &str,
        purpose: SubmissionPurpose,
    ) -> CoreResult<ManuscriptReceipt> {
        let key = manuscript_key(&caller.person, producer, work_id, purpose);
        let s = self
            .ports
            .submissions
            .manuscript(&key)
            .await?
            .ok_or_else(|| CoreError::not_found("manuscript", work_id))?;
        self.manuscript_receipt(caller, &s.id, None).await
    }

    pub(crate) async fn recover_manuscript_jobs(
        &self,
        s: &Submission,
        version: u32,
    ) -> CoreResult<()> {
        if let Some(v) = s.current_version().filter(|v| v.number == version) {
            if s.status != SubmissionStatus::Withdrawn
                && v.pdf.is_none()
                && v.compile_error.is_none()
            {
                self.ports.queue.enqueue(&s.id, JobKind::Compile).await?;
            }
            if s.status == SubmissionStatus::InReview {
                self.ports.queue.enqueue(&s.id, JobKind::Referee).await?;
            }
        }
        Ok(())
    }

    pub async fn confirm_manuscript_delivery(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        confirmation: ConfirmManuscriptDelivery,
    ) -> CoreResult<ManuscriptReceipt> {
        self.confirm_claims_bound(caller, id, confirmation.claims.clone(), Some(&confirmation))
            .await?;
        self.manuscript_receipt(caller, id, Some(confirmation.version))
            .await
    }

    pub async fn manuscript_receipt(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        number: Option<u32>,
    ) -> CoreResult<ManuscriptReceipt> {
        let s = self.submission(caller, id).await?;
        let identity = s
            .manuscript
            .as_ref()
            .ok_or_else(|| CoreError::not_found("manuscript delivery", id.as_str()))?;
        let current = s
            .current_version()
            .ok_or_else(|| CoreError::not_found("version", id.as_str()))?;
        let v = s
            .versions
            .iter()
            .find(|v| v.number == number.unwrap_or(current.number))
            .ok_or_else(|| CoreError::not_found("version", id.as_str()))?;
        let delivery = v
            .delivery
            .as_ref()
            .ok_or_else(|| CoreError::not_found("manuscript delivery", v.number.to_string()))?;
        let is_current = v.number == current.number;
        let confirmed = delivery
            .confirmation
            .as_ref()
            .filter(|c| !is_current || c.claims_revision == s.claims_revision);
        let review = match confirmed {
            Some(c) => Some(
                self.ports
                    .referees
                    .summary(id, Some(v.number), c.claims_revision)
                    .await?,
            ),
            None => None,
        };
        let record = self.ports.records.for_submission(id).await?;
        let publication = record.and_then(|r| {
            let inputs = r
                .publication
                .as_deref()
                .or_else(|| s.published_progress.as_ref()?.publication.as_deref())
                .unwrap_or(&s);
            (inputs.current_version().map(|v| v.number) == Some(v.number)).then(|| {
                ManuscriptPublication {
                    record: r.id.clone(),
                    version: v.number,
                    public: s.analysis_visibility == Visibility::Public,
                }
            })
        });
        let status = if is_current {
            s.status.name().to_owned()
        } else {
            "superseded".into()
        };
        let next_action = if !is_current || s.status == SubmissionStatus::Withdrawn {
            "none"
        } else if s.status == SubmissionStatus::Draft {
            "confirm_claims"
        } else if review.as_ref().is_some_and(|r| r.delivered_at.is_some()) {
            "read_report"
        } else if review.as_ref().is_some_and(|r| r.interrupted) {
            "inspect_execution"
        } else {
            "wait"
        };
        Ok(ManuscriptReceipt {
            schema: "mathesis.manuscript-receipt.v1",
            submission: s.id.clone(),
            producer: identity.producer.clone(),
            work_id: identity.work_id.clone(),
            upstream_version: delivery.version.clone(),
            version: v.number,
            source_sha256: v.archive.sha256.clone(),
            current_version: current.number,
            current: is_current,
            expected_claims_revision: s.claims_revision,
            confirmed_claims_revision: confirmed.map(|c| c.claims_revision),
            status,
            next_action,
            pdf_ready: v.pdf.is_some(),
            compile_error: v.compile_error.clone(),
            review,
            publication,
            extracted: if is_current && s.status == SubmissionStatus::Draft {
                s.extracted.clone()
            } else {
                vec![]
            },
        })
    }
}
