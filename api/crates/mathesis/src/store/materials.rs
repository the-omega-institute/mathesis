use async_trait::async_trait;
use mathesis_core::{CoreError, CoreResult, model::MaterialsSidecar, ports::MaterialsStore};
use mongodb::bson::doc;

use super::{MongoStore, from_document, is_duplicate_key, to_document, unavailable};

const COLLECTION: &str = "paper_materials";
const ID: &str = "index";

#[async_trait]
impl MaterialsStore for MongoStore {
    async fn load_materials(&self) -> CoreResult<MaterialsSidecar> {
        self.raw(COLLECTION)
            .find_one(doc! {"_id": ID})
            .await
            .map_err(unavailable)?
            .map(from_document)
            .transpose()
            .map(Option::unwrap_or_default)
    }

    async fn replace_materials(&self, sidecar: &MaterialsSidecar, expected: u64) -> CoreResult<()> {
        if sidecar.revision != expected + 1 {
            return Err(CoreError::invalid("invalid materials revision"));
        }
        let stale = || CoreError::StaleRevision {
            kind: "paper materials",
            id: ID.into(),
        };
        let mut document = to_document(sidecar)?;
        document.insert("_id", ID);
        if expected == 0 {
            return self
                .raw(COLLECTION)
                .insert_one(document)
                .await
                .map(|_| ())
                .map_err(|e| {
                    if is_duplicate_key(&e) {
                        stale()
                    } else {
                        unavailable(e)
                    }
                });
        }
        let result = self
            .raw(COLLECTION)
            .replace_one(doc! {"_id":ID,"revision":expected as i64}, document)
            .await
            .map_err(unavailable)?;
        if result.matched_count != 1 {
            return Err(stale());
        }
        Ok(())
    }
}
