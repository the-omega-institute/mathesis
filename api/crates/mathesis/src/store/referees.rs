use async_trait::async_trait;
use mathesis_core::{
    CoreError, CoreResult,
    ids::SubmissionId,
    model::{BlobRef, RefereeFile, ReviewSummary},
    ports::{BlobStore, Listing, RefereeStore},
};
use mongodb::bson::{Document, doc};
use sha2::{Digest, Sha256};

use super::{MongoStore, REFEREE_FILES, from_document, to_document, unavailable, write_error};

#[derive(serde::Serialize, serde::Deserialize)]
struct StatusIndex {
    revision: u64,
    items: Vec<ReviewSummary>,
}
impl StatusIndex {
    fn from_file(file: &RefereeFile) -> Self {
        Self {
            revision: file.revision,
            items: file.summary_index(),
        }
    }
    fn summary(
        self,
        id: &SubmissionId,
        version: Option<u32>,
        claims_revision: u64,
    ) -> ReviewSummary {
        self.items
            .into_iter()
            .find(|s| s.version == version && s.claims_revision == claims_revision)
            .unwrap_or_else(|| RefereeFile::new(id.clone()).summary_for(version, claims_revision))
    }
}

const INLINE_BYTES: usize = 512 * 1024;

impl MongoStore {
    async fn referee_document(
        &self,
        file: &RefereeFile,
    ) -> CoreResult<(Document, Option<BlobRef>)> {
        let status_index = to_document(&StatusIndex::from_file(file))?;
        let bytes = serde_json::to_vec(file)
            .map_err(|error| CoreError::Unavailable(format!("encode review history: {error}")))?;
        if bytes.len() <= INLINE_BYTES {
            let mut document = to_document(file)?;
            document.insert("review_status_v1", status_index);
            document.insert("binding_directory", !file.binding_boards.is_empty());
            document.insert("feedback_directory", !file.feedback_proposals.is_empty());
            return Ok((document, None));
        }
        let snapshot =
            BlobStore::put(self, &bytes, "application/vnd.mathesis.review-history+json").await?;
        let document = doc! {
            "id": file.id.as_str(),
            "binding_directory": !file.binding_boards.is_empty(),
            "feedback_directory": !file.feedback_proposals.is_empty(),
            "revision": file.revision as i64,
            "storage_format": "gridfs-json-v1",
            "snapshot": to_document(&snapshot)?,
            "review_status_v1": status_index,
        };
        Ok((document, Some(snapshot)))
    }

    async fn discard_referee_candidate(&self, snapshot: Option<BlobRef>) {
        if let Some(snapshot) = snapshot {
            let _ = BlobStore::discard(self, &snapshot.id).await;
        }
    }

    pub(super) async fn read_referee_document(
        &self,
        id: &SubmissionId,
        document: Document,
    ) -> CoreResult<RefereeFile> {
        if !document.contains_key("storage_format") {
            return from_document(document);
        }
        if document.get_str("storage_format") != Ok("gridfs-json-v1") {
            return Err(CoreError::Unavailable(
                "unknown review history storage format".into(),
            ));
        }
        let snapshot: BlobRef = from_document(
            document
                .get_document("snapshot")
                .map_err(|_| CoreError::Unavailable("review history reference missing".into()))?
                .clone(),
        )?;
        let bytes = BlobStore::get(self, &snapshot.id)
            .await?
            .ok_or_else(|| CoreError::Unavailable("review history snapshot missing".into()))?;
        if bytes.len() as u64 != snapshot.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != snapshot.sha256
        {
            return Err(CoreError::Unavailable(
                "review history snapshot checksum mismatch".into(),
            ));
        }
        let file: RefereeFile = serde_json::from_slice(&bytes)
            .map_err(|_| CoreError::Unavailable("invalid review history snapshot".into()))?;
        if &file.id != id || document.get_i64("revision").ok() != Some(file.revision as i64) {
            return Err(CoreError::Unavailable(
                "review history snapshot identity mismatch".into(),
            ));
        }
        Ok(file)
    }
}

#[async_trait]
impl RefereeStore for MongoStore {
    async fn admit_binding_submission(
        &self,
        person: &mathesis_core::ids::PersonId,
        request: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> CoreResult<()> {
        let collection = self.raw("binding_admissions");
        loop {
            let Some(document) = collection
                .find_one(doc! {"_id":person.as_str()})
                .await
                .map_err(unavailable)?
            else {
                match collection
                    .insert_one(doc! {"_id":person.as_str(),"revision":0_i64,"admissions":[]})
                    .await
                {
                    Ok(_) => continue,
                    Err(e) => match write_error(e) {
                        CoreError::Conflict(_) => continue,
                        e => return Err(e),
                    },
                }
            };
            let revision = document
                .get_i64("revision")
                .map_err(|_| CoreError::Unavailable("invalid binding rate revision".into()))?;
            let mut admissions: Vec<mathesis_core::model::BindingAdmission> =
                mongodb::bson::from_bson(document.get("admissions").cloned().unwrap_or_default())
                    .map_err(|_| CoreError::Unavailable("invalid binding rate admissions".into()))?;
            admissions.retain(|e| e.at > at - chrono::Duration::days(1));
            if admissions.iter().any(|e| e.request == request) {
                return Ok(());
            }
            if admissions.len() >= 5 {
                return Err(CoreError::conflict(
                    "daily binding contribution/challenge limit reached",
                ));
            }
            admissions.push(mathesis_core::model::BindingAdmission {
                request: request.into(),
                at,
            });
            let data = mongodb::bson::to_bson(&admissions)
                .map_err(|_| CoreError::Unavailable("cannot encode binding admission".into()))?;
            if collection
                .replace_one(
                    doc! {"_id":person.as_str(),"revision":revision},
                    doc! {"_id":person.as_str(),"revision":revision+1,"admissions":data},
                )
                .await
                .map_err(write_error)?
                .matched_count
                == 1
            {
                return Ok(());
            }
        }
    }

    async fn binding_sources(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<SubmissionId>> {
        use futures::TryStreamExt;
        let mut filter = doc! {"binding_directory":true};
        if let Some(before) = before {
            filter.insert("id", doc! {"$lt":before});
        }
        let docs = self
            .raw(REFEREE_FILES)
            .find(filter)
            .projection(doc! {"id":1,"_id":0})
            .sort(doc! {"id":-1})
            .limit(i64::from(limit) + 1)
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<_>>()
            .await
            .map_err(unavailable)?;
        let mut items = docs
            .into_iter()
            .map(|d| {
                d.get_str("id")
                    .map(|s| SubmissionId(s.into()))
                    .map_err(|_| {
                        CoreError::Unavailable("invalid binding directory identity".into())
                    })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_before = if more {
            items.last().map(ToString::to_string)
        } else {
            None
        };
        Ok(Listing { items, next_before })
    }

    async fn feedback_sources(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<SubmissionId>> {
        use futures::TryStreamExt;
        let mut filter = doc! {"feedback_directory":true};
        if let Some(before) = before {
            filter.insert("id", doc! {"$lt":before});
        }
        let docs = self
            .raw(REFEREE_FILES)
            .find(filter)
            .projection(doc! {"id":1,"_id":0})
            .sort(doc! {"id":-1})
            .limit(i64::from(limit) + 1)
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<_>>()
            .await
            .map_err(unavailable)?;
        let mut items = docs
            .into_iter()
            .map(|d| {
                d.get_str("id")
                    .map(|s| SubmissionId(s.into()))
                    .map_err(|_| {
                        CoreError::Unavailable("invalid feedback directory identity".into())
                    })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_before = if more {
            items.last().map(ToString::to_string)
        } else {
            None
        };
        Ok(Listing { items, next_before })
    }

    async fn summary(
        &self,
        id: &SubmissionId,
        version: Option<u32>,
        claims_revision: u64,
    ) -> CoreResult<ReviewSummary> {
        let header = self
            .raw(REFEREE_FILES)
            .find_one(doc! {"id": id.as_str()})
            .projection(doc! {"revision":1,"review_status_v1":1,"_id":0})
            .await
            .map_err(unavailable)?;
        let Some(header) = header else {
            return Ok(RefereeFile::new(id.clone()).summary_for(version, claims_revision));
        };
        if let Ok(index) = header.get_document("review_status_v1")
            && let Ok(index) = from_document::<StatusIndex>(index.clone())
            && header.get_i64("revision").ok() == Some(index.revision as i64)
        {
            return Ok(index.summary(id, version, claims_revision));
        }
        // Old histories pay the checked snapshot read once. The derived index is
        // separate from the immutable snapshot and fenced against concurrent writes.
        let file = RefereeStore::get(self, id)
            .await?
            .unwrap_or_else(|| RefereeFile::new(id.clone()));
        let index = StatusIndex::from_file(&file);
        let result = file.summary_for(version, claims_revision);
        let _ = self
            .raw(REFEREE_FILES)
            .update_one(
                doc! {"id": id.as_str(), "revision": file.revision as i64},
                doc! {"$set": {"review_status_v1": to_document(&index)?}},
            )
            .await;
        Ok(result)
    }

    async fn get(&self, id: &SubmissionId) -> CoreResult<Option<RefereeFile>> {
        let Some(document) = self
            .raw(REFEREE_FILES)
            .find_one(doc! { "id": id.as_str() })
            .await
            .map_err(unavailable)?
        else {
            return Ok(None);
        };
        self.read_referee_document(id, document).await.map(Some)
    }

    async fn insert(&self, file: &RefereeFile) -> CoreResult<()> {
        let (document, snapshot) = self.referee_document(file).await?;
        match self.raw(REFEREE_FILES).insert_one(document).await {
            Ok(_) => Ok(()),
            Err(error) => {
                let error = write_error(error);
                if matches!(error, CoreError::Conflict(_)) {
                    self.discard_referee_candidate(snapshot).await;
                }
                Err(error)
            }
        }
    }

    async fn replace(&self, file: &RefereeFile, expected: u64) -> CoreResult<()> {
        let mut file = file.clone();
        file.revision = expected + 1;
        let (document, snapshot) = self.referee_document(&file).await?;
        let result = self
            .raw(REFEREE_FILES)
            .replace_one(
                doc! { "id": file.id.as_str(), "revision": expected as i64 },
                document,
            )
            .await
            .map_err(write_error)?;
        if result.matched_count == 1 {
            return Ok(());
        }
        self.discard_referee_candidate(snapshot).await;
        match self
            .raw(REFEREE_FILES)
            .find_one(doc! { "id": file.id.as_str() })
            .await
            .map_err(unavailable)?
        {
            Some(_) => Err(CoreError::StaleRevision {
                kind: "referee file",
                id: file.id.to_string(),
            }),
            None => Err(CoreError::not_found("referee file", file.id.as_str())),
        }
    }
}
