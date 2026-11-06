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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveCitation>,
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
            paper: CollectedMetadata {
                archive_doi: None,
                plain_language: v
                    .explanation
                    .text
                    .clone()
                    .or_else(|| d.paper.plain_language.clone()),
                ..d.paper.clone()
            },
            published_on: v.published_on,
            pdf: file("pdf"),
            source: file("source"),
            archive: v.archive_citation().cloned(),
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

#[derive(Debug, Clone, Serialize)]
pub struct CollectedKnowledge {
    pub work: String,
    pub version: u32,
    #[serde(flatten)]
    pub results: CollectedResults,
    pub limitations: Vec<String>,
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
    fn read_collected_index(
        &self,
        bytes: &[u8],
        sha256: &str,
        revision: u64,
    ) -> CollectedSourceIndex {
        let mut index = CollectedSourceIndex {
            results: Some(CollectedResults::default()),
            revision,
            source_sha256: sha256.into(),
            bibliography: None,
            limitations: vec![],
        };
        match self.ports.reader.read(bytes, "source.zip") {
            Ok(read) => {
                index.limitations = read
                    .warnings
                    .into_iter()
                    .take(10)
                    .map(|s| s.chars().take(1_000).collect())
                    .collect();
                let mut result = CollectedResults::default();
                let mut labels = std::collections::BTreeMap::<String, Vec<String>>::new();
                // Ambiguity anywhere in the source invalidates a label, even if its
                // other occurrence is omitted by the public reading bounds.
                for (position, statement) in read.statements.iter().enumerate() {
                    if let Some(label) = &statement.latex_label {
                        labels
                            .entry(label.clone())
                            .or_default()
                            .push(format!("s{}", position + 1));
                    }
                }
                let mut references = vec![];
                let mut bytes_used = 0;
                for (position, statement) in read.statements.into_iter().enumerate() {
                    // Omit unsupported oversized entries; never truncate a mathematical statement.
                    if position >= 100
                        || statement.body.len() > 30_000
                        || statement.display_name.len() > 1_000
                        || statement.title.as_ref().is_some_and(|s| s.len() > 4_000)
                        || statement.section.as_ref().is_some_and(|s| s.len() > 4_000)
                        || bytes_used + statement.body.len() > 512 * 1024
                    {
                        index.limitations.push(
                            "Some statements exceeded the reading bounds; consult the manuscript."
                                .into(),
                        );
                        continue;
                    }
                    bytes_used += statement.body.len();
                    let id = format!("s{}", position + 1);
                    references.push((id.clone(), statement.references));
                    result.statements.push(CollectedStatement {
                        id,
                        kind: statement.kind,
                        name: statement.display_name,
                        title: statement.title,
                        statement: statement.body,
                        section: statement.section,
                    });
                }
                let admitted: std::collections::BTreeSet<_> =
                    result.statements.iter().map(|s| s.id.clone()).collect();
                for (from, targets) in references {
                    for target in targets {
                        if let Some(ids) = labels.get(&target)
                            && ids.len() == 1
                            && ids[0] != from
                            && admitted.contains(&ids[0])
                        {
                            result.connections.push(CollectedConnection {
                                from: from.clone(),
                                to: ids[0].clone(),
                                relation: CollectedRelation::ReferencesInManuscript,
                            });
                        }
                    }
                }
                if serde_json::to_vec(&read.macros).is_ok_and(|v| v.len() <= 64 * 1024) {
                    result.macros = read.macros;
                }
                index.results = Some(result);
                index.limitations.sort();
                index.limitations.dedup();
                if let Some(bibliography) = read.bibliography {
                    if bibliography.validate().is_ok() {
                        index.bibliography = Some(bibliography);
                    } else {
                        index.limitations.push("Source references exceeded the indexing bounds; consult the original paper.".into());
                    }
                }
            }
            Err(_) => index.limitations.push(
                "This source could not be indexed; consult the original paper's references.".into(),
            ),
        }
        index
    }

    /// Read already public source bytes only. No compile, model, review or proof job.
    pub async fn index_collected_paper(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
    ) -> CoreResult<()> {
        caller.require(Role::Admin)?;
        let saved = self
            .ports
            .collected
            .get(work, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("collected paper", work))?;
        if saved.source_index.as_ref().is_some_and(|index| {
            index.source_sha256 == saved.source.sha256 && index.results.is_some()
        }) {
            return Ok(());
        }
        let bytes = self.collected_file(work, version, "source").await?.bytes;
        let expected = saved.source_index.as_ref().map_or(0, |i| i.revision);
        let index = self.read_collected_index(&bytes, &saved.source.sha256, expected + 1);
        self.ports
            .collected
            .save_source_index(work, version, &index, expected)
            .await
    }

    /// Public mathematical syntax from one immutable collected source, not a target approval.
    pub async fn collected_knowledge(
        &self,
        work: &str,
        version: u32,
    ) -> CoreResult<CollectedKnowledge> {
        let saved = self
            .ports
            .collected
            .get(work, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("collected paper", work))?;
        let index = saved
            .source_index
            .as_ref()
            .filter(|i| i.source_sha256 == saved.source.sha256);
        let results = index.and_then(|i| i.results.clone()).unwrap_or_default();
        let mut limitations = index.map_or_else(Vec::new, |i| i.limitations.clone());
        if index.is_none_or(|i| i.results.is_none()) {
            limitations.push(
                "The mathematical statements of this source version have not been indexed.".into(),
            );
        }
        limitations.push("Connections are literal references in statements and attached proofs. They are not independently checked proof dependencies; unlabelled, indirect and deferred references may be missing.".into());
        Ok(CollectedKnowledge {
            work: work.into(),
            version,
            results,
            limitations,
        })
    }

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
                self.enqueue_collected_explanation(&existing).await?;
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
        let archive = if let Some(doi) = &delivery.paper.archive_doi {
            Some(
                self.check_collected_archive(
                    doi,
                    &BlobRef {
                        id: String::new(),
                        bytes: pdf.bytes.len() as u64,
                        sha256: delivery.review.pdf_sha256.clone(),
                    },
                    &BlobRef {
                        id: String::new(),
                        bytes: source.bytes.len() as u64,
                        sha256: delivery.review.source_sha256.clone(),
                    },
                )
                .await?,
            )
        } else {
            None
        };
        let pdf_ref = self.ports.blobs.put(&pdf.bytes, "application/pdf").await?;
        let source_ref = match self.ports.blobs.put(&source.bytes, "application/zip").await {
            Ok(v) => v,
            Err(e) => {
                let _ = self.ports.blobs.discard(&pdf_ref.id).await;
                return Err(e);
            }
        };
        let version = CollectedVersion {
            archive,
            source_index: Some(self.read_collected_index(&source.bytes, &source_ref.sha256, 1)),
            explanation: CollectedExplanation::default(),
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
                    self.enqueue_collected_explanation(&saved).await?;
                    return Ok(receipt(&saved));
                }
            }
            return Err(error);
        }
        self.enqueue_collected_explanation(&version).await?;
        Ok(receipt(&version))
    }

    async fn check_collected_archive(
        &self,
        doi: &str,
        pdf: &BlobRef,
        source: &BlobRef,
    ) -> CoreResult<CollectedArchive> {
        zenodo_record_id(doi)?;
        let citation = self.ports.archives.verify(doi, pdf, source).await?;
        if citation.doi != doi
            || citation
                .concept_doi
                .as_ref()
                .is_some_and(|v| zenodo_record_id(v).is_err() || v == doi)
        {
            return Err(CoreError::invalid(
                "archive returned a different DOI identity",
            ));
        }
        Ok(CollectedArchive {
            citation,
            pdf_sha256: pdf.sha256.clone(),
            source_sha256: source.sha256.clone(),
            checked_at: self.ports.clock.now(),
        })
    }

    /// Enrich one already public version without changing its delivery, receipt or queue.
    pub async fn attach_collected_archive(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        input: CollectedArchiveInput,
    ) -> CoreResult<PublicCollectedPaper> {
        if !caller.has(Role::Admin) && !self.is_internal_account(&caller.person) {
            return Err(CoreError::forbidden(
                "trusted collection operator or administrator required",
            ));
        }
        let saved = self
            .ports
            .collected
            .get(work, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("collected paper", work))?;
        if !caller.has(Role::Admin) && saved.owner != caller.person {
            return Err(CoreError::forbidden(
                "only this collection's owner can attach its archive",
            ));
        }
        if input.pdf_sha256 != saved.pdf.sha256 || input.source_sha256 != saved.source.sha256 {
            return Err(CoreError::conflict(
                "archive must name this exact reviewed PDF and source",
            ));
        }
        zenodo_record_id(&input.doi)?;
        if let Some(existing) = saved.archive_citation() {
            if existing.doi != input.doi {
                return Err(CoreError::conflict(
                    "this version already has a different archive DOI",
                ));
            }
            return Ok((&saved).into());
        }
        let archive = self
            .check_collected_archive(&input.doi, &saved.pdf, &saved.source)
            .await?;
        self.ports
            .collected
            .attach_archive(work, version, &archive)
            .await?;
        self.collected_paper(work, Some(version)).await
    }

    async fn enqueue_collected_explanation(&self, v: &CollectedVersion) -> CoreResult<()> {
        if v.explanation.text.is_none() {
            self.ports
                .queue
                .enqueue(
                    &crate::ids::SubmissionId(format!(
                        "collected:{}:{}",
                        v.delivery.work_id, v.delivery.version
                    )),
                    crate::ports::JobKind::CollectedExplanation,
                )
                .await?;
        }
        Ok(())
    }

    /// Backfill exposition of an already public exact version; never admits or re-reviews it.
    pub async fn request_collected_explanation(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
    ) -> CoreResult<PublicCollectedPaper> {
        caller.require(Role::Admin)?;
        let v = self
            .collected_explanation_input(caller, work, version)
            .await?;
        self.enqueue_collected_explanation(&v).await?;
        Ok((&v).into())
    }

    pub async fn collected_explanation_input(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
    ) -> CoreResult<CollectedVersion> {
        if !caller.has(Role::Admin) {
            caller.require(Role::Reviewer)?;
        }
        self.ports
            .collected
            .get(work, Some(version))
            .await?
            .ok_or_else(|| CoreError::not_found("collected paper", work))
    }

    /// Only the explanation sidecar changes. File bytes, review, authors and receipt stay immutable.
    pub async fn save_collected_explanation(
        &self,
        caller: &Caller,
        work: &str,
        version: u32,
        value: CollectedExplanation,
        expected: u64,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        if let Some(text) = &value.text {
            text.validate()?;
        }
        if serde_json::to_vec(&value)
            .map_err(|_| CoreError::invalid("invalid explanation"))?
            .len()
            > 2_000_000
        {
            return Err(CoreError::invalid("explanation state too large"));
        }
        self.ports
            .collected
            .save_explanation(work, version, &value, expected)
            .await
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
