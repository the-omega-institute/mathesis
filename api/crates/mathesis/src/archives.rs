//! Consume published Zenodo records; no credentials, deposits or DOI registration.
use async_trait::async_trait;
use mathesis_core::{
    CoreError, CoreResult,
    model::{ArchiveCitation, BlobRef, zenodo_record_id},
    ports::ArchiveRegistry,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub struct ZenodoArchive;

#[derive(Deserialize)]
struct Record {
    id: u64,
    doi: String,
    conceptdoi: Option<String>,
    submitted: bool,
    state: String,
    files: Vec<File>,
}
#[derive(Deserialize)]
struct File {
    key: String,
    size: u64,
}

impl Record {
    fn citation(&self, doi: &str) -> CoreResult<ArchiveCitation> {
        if self.id != zenodo_record_id(doi)?
            || self.doi != doi
            || !self.submitted
            || self.state != "done"
        {
            return Err(CoreError::invalid(
                "DOI does not identify a published exact-version record",
            ));
        }
        if self
            .conceptdoi
            .as_ref()
            .is_some_and(|v| zenodo_record_id(v).is_err() || v == doi)
        {
            return Err(CoreError::invalid("invalid archive concept DOI"));
        }
        Ok(ArchiveCitation {
            doi: self.doi.clone(),
            concept_doi: self.conceptdoi.clone(),
        })
    }
    fn file(&self, expected: &BlobRef, extension: &str) -> CoreResult<&File> {
        let mut candidates = self.files.iter().filter(|f| {
            f.size == expected.bytes && f.key.to_ascii_lowercase().ends_with(extension)
        });
        let file = candidates.next().ok_or_else(|| {
            CoreError::invalid("archive does not contain the reviewed PDF/source")
        })?;
        if candidates.next().is_some()
            || file.key.is_empty()
            || file.key.len() > 200
            || file.key.contains(['/', '\\'])
            || file.key.chars().any(char::is_control)
        {
            return Err(CoreError::invalid("archive file identity is ambiguous"));
        }
        Ok(file)
    }
}

async fn response(client: &reqwest::Client, url: &str) -> CoreResult<reqwest::Response> {
    let response = client.get(url).send().await.map_err(|_| {
        CoreError::Unavailable("Zenodo could not be reached; retry the same DOI".into())
    })?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(CoreError::invalid(
            "no public published Zenodo record exists for this DOI",
        ));
    }
    if !response.status().is_success() {
        return Err(CoreError::Unavailable(
            "Zenodo did not return a usable public record; retry the same DOI".into(),
        ));
    }
    Ok(response)
}
async fn read_bounded(mut response: reqwest::Response, max: usize) -> CoreResult<Vec<u8>> {
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err(CoreError::invalid(
            "archive response exceeds its file bound",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        CoreError::Unavailable("archive download interrupted; retry the same DOI".into())
    })? {
        if bytes.len().saturating_add(chunk.len()) > max {
            return Err(CoreError::invalid(
                "archive response exceeds its file bound",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[async_trait]
impl ArchiveRegistry for ZenodoArchive {
    async fn verify(
        &self,
        doi: &str,
        pdf: &BlobRef,
        source: &BlobRef,
    ) -> CoreResult<ArchiveCitation> {
        let id = zenodo_record_id(doi)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .user_agent("Mathesis/1.0 (published archive metadata)")
            .build()
            .map_err(|_| CoreError::Unavailable("archive client unavailable".into()))?;
        let url = format!("https://zenodo.org/api/records/{id}");
        let bytes = read_bounded(response(&client, &url).await?, 1024 * 1024).await?;
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|_| CoreError::invalid("invalid published archive metadata"))?;
        let citation = record.citation(doi)?;
        for (expected, extension) in [(pdf, ".pdf"), (source, ".zip")] {
            if expected.bytes > mathesis_core::app::MAX_UPLOAD_BYTES as u64 {
                return Err(CoreError::invalid(
                    "archive file exceeds manuscript size limit",
                ));
            }
            let file = record.file(expected, extension)?;
            // Build a fixed production-host URL rather than following remote links.
            let mut url = url::Url::parse(&format!("https://zenodo.org/api/records/{id}/files/"))
                .expect("fixed URL");
            url.path_segments_mut()
                .expect("fixed hierarchical URL")
                .pop_if_empty()
                .push(&file.key)
                .push("content");
            let bytes = read_bounded(
                response(&client, url.as_str()).await?,
                expected.bytes as usize,
            )
            .await?;
            if bytes.len() as u64 != expected.bytes
                || format!("{:x}", Sha256::digest(&bytes)) != expected.sha256
            {
                return Err(CoreError::invalid(
                    "archive file differs from this reviewed manuscript version",
                ));
            }
        }
        Ok(citation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reserved_concept_and_wrong_version_records() {
        let mut record = Record {
            id: 123,
            doi: "10.5281/zenodo.123".into(),
            conceptdoi: Some("10.5281/zenodo.122".into()),
            submitted: true,
            state: "done".into(),
            files: vec![],
        };
        assert!(record.citation("10.5281/zenodo.123").is_ok());
        assert!(record.citation("10.5281/zenodo.122").is_err());
        record.submitted = false;
        assert!(record.citation("10.5281/zenodo.123").is_err());
        record.submitted = true;
        record.state = "draft".into();
        assert!(record.citation("10.5281/zenodo.123").is_err());
    }

    #[test]
    fn file_selection_requires_unambiguous_matching_size_and_format() {
        let mut record = Record {
            id: 123,
            doi: String::new(),
            conceptdoi: None,
            submitted: true,
            state: "done".into(),
            files: vec![File {
                key: "note.pdf".into(),
                size: 12,
            }],
        };
        let expected = BlobRef {
            id: String::new(),
            bytes: 12,
            sha256: String::new(),
        };
        assert!(record.file(&expected, ".pdf").is_ok());
        assert!(record.file(&expected, ".zip").is_err());
        record.files.push(File {
            key: "other.pdf".into(),
            size: 12,
        });
        assert!(record.file(&expected, ".pdf").is_err());
    }
}
