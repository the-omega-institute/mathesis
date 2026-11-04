use super::{MongoStore, from_document, unavailable};
use async_trait::async_trait;
use futures::TryStreamExt;
use mathesis_core::{
    CoreResult,
    model::{CollectedVersion, PaperSearch},
    ports::{CollectedPaperStore, Listing},
};
use mongodb::bson::doc;
pub(super) const COLLECTION: &str = "collected_papers";
#[async_trait]
impl CollectedPaperStore for MongoStore {
    async fn get(&self, work: &str, version: Option<u32>) -> CoreResult<Option<CollectedVersion>> {
        let mut filter = doc! {"delivery.work_id": work};
        if let Some(v) = version {
            filter.insert("delivery.version", i64::from(v));
        }
        self.raw(COLLECTION)
            .find_one(filter)
            .sort(doc! {"delivery.version": -1})
            .await
            .map_err(unavailable)?
            .map(from_document)
            .transpose()
    }
    async fn insert(&self, v: &CollectedVersion) -> CoreResult<()> {
        self.insert(COLLECTION, v).await
    }
    async fn list(
        &self,
        search: &PaperSearch,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<CollectedVersion>> {
        let mut pipeline = vec![
            doc! {"$sort": {"delivery.work_id": 1, "delivery.version": -1}},
            doc! {"$group": {"_id": "$delivery.work_id", "latest": {"$first": "$$ROOT"}}},
            doc! {"$replaceRoot": {"newRoot": "$latest"}},
        ];
        if let Some(before) = before {
            pipeline.push(doc! {"$match": {"id": {"$lt": before}}});
        }
        pipeline.push(doc! {"$sort": {"id": -1}});
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut cursor = self
                .raw(COLLECTION)
                .aggregate(pipeline)
                .max_time(std::time::Duration::from_secs(10))
                .batch_size(25)
                .await
                .map_err(unavailable)?;
            let mut items: Vec<CollectedVersion> = Vec::new();
            while let Some(doc) = cursor.try_next().await.map_err(unavailable)? {
                let v: CollectedVersion = from_document(doc)?;
                if v.matches(search) {
                    items.push(v);
                }
                if items.len() > limit as usize {
                    break;
                }
            }
            let next_before = if items.len() > limit as usize {
                items.pop();
                items.last().map(|v| v.id.clone())
            } else {
                None
            };
            Ok(Listing { items, next_before })
        })
        .await
        .map_err(|_| mathesis_core::CoreError::Unavailable("collection search timed out".into()))?
    }
}
