//! Source archives and compiled PDFs in GridFS, addressed by a UUID.

use async_trait::async_trait;
use futures::{TryStreamExt, io::AsyncWriteExt};
use mathesis_core::{CoreError, CoreResult, ids::new_uuid, model::BlobRef, ports::BlobStore};
use mongodb::bson::{Bson, doc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{MongoStore, unavailable};

fn gridfs(error: impl std::fmt::Display) -> CoreError {
    CoreError::Unavailable(format!("file storage: {error}"))
}

#[derive(Deserialize)]
struct FileHeader {
    length: u64,
    #[serde(rename = "chunkSize")]
    chunk_size: u32,
}

#[derive(Deserialize)]
struct FileChunk {
    n: u64,
    data: mongodb::bson::Binary,
}

impl MongoStore {
    /// Read the existing GridFS format through an ordinary document cursor.
    /// This avoids the driver's separate raw-batch download path. Validate the
    /// complete chunk sequence before returning any bytes to a reader.
    async fn read_blob(&self, id: &str) -> CoreResult<Option<Vec<u8>>> {
        let Some(file) = self
            .db
            .collection::<FileHeader>("papers.files")
            .find_one(doc! { "_id": id })
            .await
            .map_err(gridfs)?
        else {
            return Ok(None);
        };
        if file.chunk_size == 0 {
            return Err(gridfs("invalid chunk size"));
        }
        let mut chunks = self
            .db
            .collection::<FileChunk>("papers.chunks")
            .find(doc! { "files_id": id })
            .sort(doc! { "n": 1 })
            .await
            .map_err(gridfs)?;
        let mut bytes = Vec::new();
        let mut n = 0;
        let mut remaining = file.length;
        while let Some(chunk) = chunks.try_next().await.map_err(gridfs)? {
            let expected = remaining.min(u64::from(file.chunk_size));
            if chunk.n != n || expected == 0 || chunk.data.bytes.len() as u64 != expected {
                return Err(gridfs("missing, extra or incorrectly sized file chunk"));
            }
            bytes.try_reserve(chunk.data.bytes.len()).map_err(gridfs)?;
            bytes.extend_from_slice(&chunk.data.bytes);
            remaining -= expected;
            n += 1;
        }
        if remaining != 0 {
            return Err(gridfs("incomplete file"));
        }
        Ok(Some(bytes))
    }
}

#[async_trait]
impl BlobStore for MongoStore {
    async fn discard(&self, id: &str) -> CoreResult<()> {
        self.bucket()
            .delete(Bson::String(id.to_owned()))
            .await
            .map_err(gridfs)
    }

    async fn put(&self, bytes: &[u8], content_type: &str) -> CoreResult<BlobRef> {
        let id = new_uuid();
        let mut upload = self
            .bucket()
            .open_upload_stream(&id)
            .id(Bson::String(id.clone()))
            .metadata(doc! { "content_type": content_type })
            .await
            .map_err(unavailable)?;
        if let Err(error) = upload.write_all(bytes).await {
            let _ = upload.abort().await;
            return Err(gridfs(error));
        }
        upload.close().await.map_err(gridfs)?;
        Ok(BlobRef {
            id,
            bytes: bytes.len() as u64,
            sha256: Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
    }

    async fn get(&self, id: &str) -> CoreResult<Option<Vec<u8>>> {
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), self.read_blob(id))
            .await
            .map_err(|_| gridfs("read timed out"))?;
        if let Err(error) = &result {
            tracing::warn!(blob_id = id, %error, "file read failed");
        }
        result
    }
}
