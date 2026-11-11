use super::{MongoStore, from_document, unavailable};
use async_trait::async_trait;
use futures::TryStreamExt;
use mathesis_core::{
    CoreError, CoreResult,
    model::{CollectedProblemVersion, CuratedTargetCheck},
    ports::{CollectedProblemStore, Listing},
};
use mongodb::bson::{Document, doc};

pub(super) const COLLECTION: &str = "collected_problems";

impl MongoStore {
    async fn current_collected_problem(
        &self,
        filter: Document,
    ) -> CoreResult<Option<CollectedProblemVersion>> {
        self.raw(COLLECTION)
            .find_one(filter)
            .sort(doc! {"delivery.version": -1})
            .await
            .map_err(unavailable)?
            .map(from_document)
            .transpose()
    }
}

#[async_trait]
impl CollectedProblemStore for MongoStore {
    async fn get(
        &self,
        problem: &str,
        version: Option<u32>,
    ) -> CoreResult<Option<CollectedProblemVersion>> {
        let mut filter = doc! {"delivery.problem_id": problem};
        if let Some(v) = version {
            filter.insert("delivery.version", i64::from(v));
        }
        self.current_collected_problem(filter).await
    }
    async fn get_by_key(&self, key: &str) -> CoreResult<Option<CollectedProblemVersion>> {
        self.current_collected_problem(doc! {"key": key}).await
    }
    async fn insert(&self, v: &CollectedProblemVersion) -> CoreResult<()> {
        self.insert(COLLECTION, v).await
    }
    async fn attach_target_check(
        &self,
        problem: &str,
        version: u32,
        check: &CuratedTargetCheck,
    ) -> CoreResult<()> {
        let id = doc! {"delivery.problem_id": problem, "delivery.version": i64::from(version)};
        let mut unchecked = id.clone();
        unchecked.insert("target_check", mongodb::bson::Bson::Null);
        let check = super::to_document(check)?;
        // `null` matches a missing field, so the first check wins and later ones
        // leave it in place.
        let updated = self
            .raw(COLLECTION)
            .update_one(unchecked, doc! {"$set": {"target_check": check}})
            .await
            .map_err(unavailable)?;
        if updated.matched_count == 0
            && self
                .raw(COLLECTION)
                .find_one(id)
                .await
                .map_err(unavailable)?
                .is_none()
        {
            return Err(CoreError::not_found(
                "collected problem version",
                format!("{problem}:{version}"),
            ));
        }
        Ok(())
    }
    async fn list(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<CollectedProblemVersion>> {
        let mut pipeline = vec![];
        if let Some(before) = before {
            pipeline.push(doc! {"$match": {"key": {"$lt": before}}});
        }
        // The unique key/version index supplies this order; each group keeps its
        // highest version, which is the entry's current one.
        pipeline.extend([
            doc! {"$sort": {"key": -1, "delivery.version": -1}},
            doc! {"$group": {"_id": "$key", "current": {"$first": "$$ROOT"}}},
            doc! {"$sort": {"_id": -1}},
            doc! {"$limit": i64::from(limit) + 1},
            doc! {"$replaceRoot": {"newRoot": "$current"}},
        ]);
        let mut items: Vec<CollectedProblemVersion> = self
            .raw(COLLECTION)
            .aggregate(pipeline)
            .max_time(std::time::Duration::from_secs(10))
            .await
            .map_err(unavailable)?
            .try_collect::<Vec<Document>>()
            .await
            .map_err(unavailable)?
            .into_iter()
            .map(from_document)
            .collect::<CoreResult<_>>()?;
        let next_before = if items.len() > limit as usize {
            items.pop();
            items.last().map(|v| v.key.clone())
        } else {
            None
        };
        Ok(Listing { items, next_before })
    }
}
