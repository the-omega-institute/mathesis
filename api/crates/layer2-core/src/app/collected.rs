use super::{App, PaperFile, Upload, clamp};
use crate::{CoreError, CoreResult, model::*, ports::Listing};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct CollectedFileLink {
    pub url: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PublicCollectedPaper {
    pub id: String,
    pub revision: String,
    pub previous_version: Option<String>,
    #[serde(flatten)]
    pub paper: CollectedMetadata,
    pub published_on: chrono::DateTime<chrono::Utc>,
    pub pdf: CollectedFileLink,
    pub source: CollectedFileLink,
}
impl From<&CollectedVersion> for PublicCollectedPaper {
    fn from(v: &CollectedVersion) -> Self {
        let d = &v.delivery;
        let file = |kind: &str| CollectedFileLink {
            url: format!(
                "/api/v1/collected-papers/{}/files/{kind}?version={}",
                d.work_id, d.version
            ),
        };
        Self {
            id: d.work_id.clone(),
            revision: d.version.to_string(),
            previous_version: d.previous_version.map(|v| v.to_string()),
            paper: d.paper.clone(),
            published_on: v.published_on,
            pdf: file("pdf"),
            source: file("source"),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CollectionReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub work_id: String,
    pub version: u32,
    pub public_path: String,
    pub pdf_sha256: String,
    pub source_sha256: String,
}
fn receipt(v: &CollectedVersion) -> CollectionReceipt {
    CollectionReceipt {
        schema: "mathesis.pre-reviewed-receipt.v1",
        status: "collected",
        work_id: v.delivery.work_id.clone(),
        version: v.delivery.version,
        public_path: format!(
            "/papers/{}?version={}",
            v.delivery.work_id, v.delivery.version
        ),
        pdf_sha256: v.pdf.sha256.clone(),
        source_sha256: v.source.sha256.clone(),
    }
}
impl App {
    /// This explicit publication permission does not bypass ordinary submission policy.
    pub async fn collect_pre_reviewed(
        &self,
        caller: &Caller,
        delivery: PreReviewedDelivery,
        pdf: Upload,
        source: Upload,
    ) -> CoreResult<CollectionReceipt> {
        if !self.is_internal_account(&caller.person) {
            return Err(CoreError::forbidden("trusted collection operator required"));
        }
        delivery.validate()?;
        for (upload, expected, extension, magic) in [
            (
                &pdf,
                &delivery.review.pdf_sha256,
                ".pdf",
                b"%PDF-".as_slice(),
            ),
            (
                &source,
                &delivery.review.source_sha256,
                ".zip",
                b"PK\x03\x04".as_slice(),
            ),
        ] {
            if upload.filename.len() > 200
                || upload.filename.contains(['/', '\\'])
                || upload.filename.chars().any(char::is_control)
                || !upload.filename.to_lowercase().ends_with(extension)
                || !upload.bytes.starts_with(magic)
                || upload.bytes.len() > super::MAX_UPLOAD_BYTES
                || format!("{:x}", Sha256::digest(&upload.bytes)) != *expected
            {
                return Err(CoreError::invalid(
                    "reviewed PDF/source file identity, format or size does not match",
                ));
            }
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(&delivery, &pdf.filename, &source.filename))
                    .expect("serializable delivery")
            )
        );
        if let Some(current) = self.ports.collected.get(&delivery.work_id, None).await? {
            if current.owner != caller.person || current.delivery.producer != delivery.producer {
                return Err(CoreError::conflict(
                    "work identity belongs to a different collection operator",
                ));
            }
            if let Some(existing) = self
                .ports
                .collected
                .get(&delivery.work_id, Some(delivery.version))
                .await?
            {
                if existing.request_digest != digest {
                    return Err(CoreError::conflict(
                        "published versions are immutable; deliver a new version",
                    ));
                }
                return Ok(receipt(&existing));
            }
            if delivery.previous_version != Some(current.delivery.version) {
                return Err(CoreError::conflict(
                    "name the current published predecessor",
                ));
            }
        } else if delivery.previous_version.is_some() {
            return Err(CoreError::conflict(
                "first collected version cannot name a predecessor",
            ));
        }
        let pdf_ref = self.ports.blobs.put(&pdf.bytes, "application/pdf").await?;
        let source_ref = match self.ports.blobs.put(&source.bytes, "application/zip").await {
            Ok(v) => v,
            Err(e) => {
                let _ = self.ports.blobs.discard(&pdf_ref.id).await;
                return Err(e);
            }
        };
        let version = CollectedVersion {
            id: uuid::Uuid::now_v7().to_string(),
            owner: caller.person.clone(),
            delivery,
            request_digest: digest,
            pdf: pdf_ref,
            source: source_ref,
            published_on: self.ports.clock.now(),
        };
        if let Err(error) = self.ports.collected.insert(&version).await {
            // Only a definitive uniqueness conflict proves our freshly allocated blobs
            // were not committed. A transport error may follow a successful write.
            if matches!(error, CoreError::Conflict(_)) {
                let _ = self.ports.blobs.discard(&version.pdf.id).await;
                let _ = self.ports.blobs.discard(&version.source.id).await;
                if let Some(saved) = self
                    .ports
                    .collected
                    .get(&version.delivery.work_id, Some(version.delivery.version))
                    .await?
                    && saved.owner == version.owner
                    && saved.request_digest == version.request_digest
                {
                    return Ok(receipt(&saved));
                }
            }
            return Err(error);
        }
        Ok(receipt(&version))
    }
    pub async fn collected_paper(
        &self,
        id: &str,
        version: Option<u32>,
    ) -> CoreResult<PublicCollectedPaper> {
        let v = self
            .ports
            .collected
            .get(id, version)
            .await?
            .ok_or_else(|| CoreError::not_found("paper", id))?;
        Ok((&v).into())
    }
    pub async fn collected_papers(
        &self,
        mut search: PaperSearch,
        limit: Option<u32>,
        before: Option<String>,
    ) -> CoreResult<Listing<PublicCollectedPaper>> {
        search.validate()?;
        let listing = self
            .ports
            .collected
            .list(&search, clamp(limit), before)
            .await?;
        Ok(Listing {
            items: listing.items.iter().map(Into::into).collect(),
            next_before: listing.next_before,
        })
    }
    pub async fn collected_file(
        &self,
        id: &str,
        version: u32,
        kind: &str,
    ) -> CoreResult<PaperFile> {
        let v = self
            .ports
            .collected
            .get(id, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("paper", id))?;
        let (blob, content_type, extension) = match kind {
            "pdf" => (&v.pdf, "application/pdf", "pdf"),
            "source" => (&v.source, "application/zip", "zip"),
            _ => return Err(CoreError::not_found("paper file", kind)),
        };
        let bytes = self
            .ports
            .blobs
            .get(&blob.id)
            .await?
            .ok_or_else(|| CoreError::not_found("paper file", id))?;
        if bytes.len() as u64 != blob.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != blob.sha256
        {
            return Err(CoreError::Unavailable(
                "published file failed its integrity check".into(),
            ));
        }
        Ok(PaperFile {
            bytes,
            content_type,
            filename: format!("{id}-v{version}.{extension}"),
        })
    }
}
