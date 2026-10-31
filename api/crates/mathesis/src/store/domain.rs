//! Layer 2 store ports over MongoDB.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mathesis_core::{
    CoreError, CoreResult,
    ids::{PersonId, RecordId, SubmissionId},
    memory::display_name,
    model::{Endorsement, Person, Record, Role, Submission, VerifiedIdentity},
    ports::*,
};
use mongodb::{
    bson::{Document, doc, to_bson},
    options::ReturnDocument,
};

use super::*;

fn bson_value<T: serde::Serialize>(value: &T) -> CoreResult<mongodb::bson::Bson> {
    to_bson(value).map_err(|e| CoreError::Unavailable(format!("encode value: {e}")))
}

#[async_trait]
impl PersonStore for MongoStore {
    async fn upsert_sign_in(
        &self,
        identity: &VerifiedIdentity,
        at: DateTime<Utc>,
    ) -> CoreResult<Person> {
        let at = bson_value(&at)?;
        let mut set = doc! { "display_name": display_name(identity), "last_seen_at": at.clone() };
        let mut unset = Document::new();
        for (field, value) in [("email", &identity.email), ("picture", &identity.picture)] {
            match value {
                Some(value) => set.insert(field, value),
                None => unset.insert(field, ""),
            };
        }
        let mut update = doc! {
            "$set": set,
            "$setOnInsert": { "id": identity.subject.as_str(), "roles": [], "created_at": at },
        };
        if !unset.is_empty() {
            update.insert("$unset", unset);
        }
        let person = self
            .raw(PEOPLE)
            .find_one_and_update(doc! { "id": identity.subject.as_str() }, update)
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await
            .map_err(write_error)?
            .ok_or_else(|| CoreError::Unavailable("upsert returned no person".into()))?;
        from_document(person)
    }

    async fn get(&self, id: &PersonId) -> CoreResult<Option<Person>> {
        self.get_by_id(PEOPLE, id.as_str()).await
    }

    async fn list(&self, limit: u32, before: Option<String>) -> CoreResult<Listing<Person>> {
        self.page(PEOPLE, doc! {}, limit, before).await
    }

    async fn set_roles(&self, id: &PersonId, roles: &[Role]) -> CoreResult<Person> {
        let mut roles: Vec<Role> = roles.to_vec();
        roles.sort();
        roles.dedup();
        let person = self
            .raw(PEOPLE)
            .find_one_and_update(
                doc! { "id": id.as_str() },
                doc! { "$set": { "roles": bson_value(&roles)? } },
            )
            .return_document(ReturnDocument::After)
            .await
            .map_err(unavailable)?
            .ok_or_else(|| CoreError::not_found("person", id.as_str()))?;
        from_document(person)
    }
}

#[async_trait]
impl SubmissionStore for MongoStore {
    async fn insert(&self, submission: &Submission) -> CoreResult<()> {
        MongoStore::insert(self, SUBMISSIONS, submission).await
    }

    async fn manuscript(&self, key: &str) -> CoreResult<Option<Submission>> {
        self.raw(SUBMISSIONS)
            .find_one(doc! {"manuscript.key": key})
            .await
            .map_err(unavailable)?
            .map(from_document)
            .transpose()
    }

    async fn get(&self, id: &SubmissionId) -> CoreResult<Option<Submission>> {
        self.get_by_id(SUBMISSIONS, id.as_str()).await
    }

    async fn list(
        &self,
        filter: &SubmissionFilter,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Submission>> {
        let mut query = Document::new();
        if let Some(person) = &filter.involving {
            query.insert(
                "$or",
                vec![
                    doc! { "submitter": person.as_str() },
                    doc! { "authors.person": person.as_str() },
                ],
            );
        }
        if let Some(status) = &filter.status {
            query.insert("status.state", status.as_str());
        }
        self.page(SUBMISSIONS, query, limit, before).await
    }

    async fn active_count(&self, person: &PersonId) -> CoreResult<u32> {
        let n = self
            .raw(SUBMISSIONS)
            .count_documents(doc! {
                "submitter": person.as_str(),
                "status.state": { "$in": ["draft", "in_review"] },
            })
            .await
            .map_err(unavailable)?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    async fn replace(&self, submission: &Submission, expected: u64) -> CoreResult<()> {
        self.replace_revision(
            SUBMISSIONS,
            "submission",
            submission.id.as_str(),
            submission,
            expected,
        )
        .await
    }
}

#[async_trait]
impl EndorsementStore for MongoStore {
    async fn insert(&self, endorsement: &Endorsement) -> CoreResult<()> {
        match MongoStore::insert(self, ENDORSEMENTS, endorsement).await {
            Err(CoreError::Conflict(_)) => Err(CoreError::conflict(
                "this endorser has already endorsed the submission",
            )),
            other => other,
        }
    }

    async fn for_submission(&self, submission: &SubmissionId) -> CoreResult<Vec<Endorsement>> {
        let documents: Vec<Document> = self
            .raw(ENDORSEMENTS)
            .find(doc! { "submission": submission.as_str() })
            .sort(doc! { "filed_at": 1 })
            .await
            .map_err(unavailable)?
            .try_collect()
            .await
            .map_err(unavailable)?;
        documents.into_iter().map(from_document).collect()
    }
}

#[async_trait]
impl RecordStore for MongoStore {
    async fn search_research(
        &self,
        search: &mathesis_core::model::ResearchSearch,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Record>> {
        let limit = limit.clamp(1, 20) as usize;
        let mut filter = doc! {};
        if let Some(before) = before {
            filter.insert("id", doc! {"$lt": before});
        }
        // Select only by cursor and live visibility. Mongo regex/lowercasing
        // cannot implement Rust's Unicode lowercase substring semantics, even
        // for ASCII queries against Unicode source text. Keeping every public
        // candidate also covers exact identities and legacy accepted inputs;
        // the shared domain predicate below is the only content/identity test.
        let pipeline = vec![
            doc! {"$match": filter},
            doc! {"$sort": {"id": -1}},
            doc! {"$lookup": {"from": SUBMISSIONS, "localField": "submission", "foreignField": "id", "pipeline": [{"$match": {"analysis_visibility": "public"}}], "as": "research_current"}},
            doc! {"$unwind": "$research_current"},
        ];
        // Bound elapsed work including getMore batches. A timeout is an explicit
        // unavailable result, never an empty/exhaustive graph or reset cursor.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let mut cursor = self
                .raw(RECORDS)
                .aggregate(pipeline)
                .max_time(std::time::Duration::from_secs(10))
                .batch_size(20)
                .await
                .map_err(unavailable)?;
            let mut items = Vec::new();
            while let Some(mut document) = cursor.try_next().await.map_err(unavailable)? {
                let current = document
                    .remove("research_current")
                    .and_then(|v| v.as_document().cloned())
                    .ok_or_else(|| {
                        CoreError::Unavailable("missing research visibility input".into())
                    })?;
                let current: Submission = from_document(current)?;
                let record: Record = from_document(document)?;
                if search.matches_visible_record(&record, &current) {
                    items.push(record);
                    if items.len() > limit {
                        break;
                    }
                }
            }
            let next_before = if items.len() > limit {
                items.pop();
                items.last().map(|r| r.id.to_string())
            } else {
                None
            };
            Ok(Listing { items, next_before })
        })
        .await
        .map_err(|_| {
            CoreError::Unavailable("research graph query exceeded its 10-second deadline".into())
        })?
    }

    async fn search(
        &self,
        search: &mathesis_core::model::PaperSearch,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Record>> {
        let mut filter = doc! {};
        if let Some(q) = &search.q {
            if q.starts_with("WP-") {
                filter.insert("$and", vec![doc! {"id": q}]);
            } else {
                // Accept plain words only, never Mongo text operators or regex syntax.
                let words = q
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                filter.insert("$text", doc! {"$search": words, "$language": "none"});
            }
        }
        if let Some(kind) = search.kind {
            if kind == mathesis_core::model::SubmissionKind::Paper {
                filter.insert(
                    "$or",
                    vec![doc! {"kind": "paper"}, doc! {"kind": {"$exists": false}}],
                );
            } else {
                filter.insert("kind", bson_value(&kind)?);
            }
        }
        let literal = |s: &str| {
            s.chars()
                .map(|c| {
                    if "\\.^$|?*+()[]{}".contains(c) {
                        format!("\\{c}")
                    } else {
                        c.to_string()
                    }
                })
                .collect::<String>()
        };
        if let Some(author) = &search.author {
            filter.insert(
                "authors.name",
                doc! {"$regex": literal(author), "$options": "i"},
            );
        }
        if let Some(msc) = &search.msc {
            filter.insert(
                "publication.msc",
                doc! {"$regex": format!("^{}", literal(msc))},
            );
        }
        if let Some(before) = before {
            filter.insert("id", doc! {"$lt": before});
        }
        // Gate candidates against authoritative live visibility before limiting the page.
        // The application checks it again when producing the public projection.
        let mut documents: Vec<Document> = self.raw(RECORDS).aggregate(vec![
            doc! {"$match": filter},
            doc! {"$sort": {"id": -1}},
            doc! {"$lookup": {"from": SUBMISSIONS, "localField": "submission", "foreignField": "id", "pipeline": [{"$match": {"analysis_visibility": "public"}}, {"$project": {"_id": 1}}], "as": "search_visibility"}},
            doc! {"$match": {"search_visibility.0": {"$exists": true}}},
            doc! {"$unset": "search_visibility"},
            doc! {"$limit": i64::from(limit) + 1},
        ]).max_time(std::time::Duration::from_secs(10)).await.map_err(unavailable)?.try_collect().await.map_err(unavailable)?;
        let next_before = if documents.len() > limit as usize {
            documents.pop();
            documents
                .last()
                .and_then(|d| d.get_str("id").ok())
                .map(str::to_owned)
        } else {
            None
        };
        Ok(Listing {
            items: documents
                .into_iter()
                .map(from_document)
                .collect::<CoreResult<_>>()?,
            next_before,
        })
    }

    async fn for_submission(&self, id: &SubmissionId) -> CoreResult<Option<Record>> {
        self.raw(RECORDS)
            .find_one(doc! { "submission": id.as_str() })
            .await
            .map_err(unavailable)?
            .map(from_document)
            .transpose()
    }
    async fn next_sequence(&self, year: i32) -> CoreResult<u64> {
        let counter = self
            .raw(COUNTERS)
            .find_one_and_update(
                doc! { "_id": format!("records-{year}") },
                doc! { "$inc": { "value": 1_i64 } },
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await
            .map_err(write_error)?
            .ok_or_else(|| CoreError::Unavailable("counter upsert returned nothing".into()))?;
        counter
            .get_i64("value")
            .map(|v| v as u64)
            .map_err(|e| CoreError::Unavailable(format!("counter value: {e}")))
    }

    async fn insert(&self, record: &Record) -> CoreResult<()> {
        MongoStore::insert(self, RECORDS, record).await
    }

    async fn get(&self, id: &RecordId) -> CoreResult<Option<Record>> {
        self.get_by_id(RECORDS, id.as_str()).await
    }

    async fn list(&self, limit: u32, before: Option<String>) -> CoreResult<Listing<Record>> {
        self.page(RECORDS, doc! {}, limit, before).await
    }
}

#[async_trait]
impl LeanDepositStore for MongoStore {
    async fn get(&self, id: &str) -> CoreResult<Option<mathesis_core::model::LeanDeposit>> {
        self.get_by_id("lean_deposits", id).await
    }
    async fn insert(&self, value: &mathesis_core::model::LeanDeposit) -> CoreResult<()> {
        MongoStore::insert(self, "lean_deposits", value).await
    }
    async fn replace(
        &self,
        value: &mathesis_core::model::LeanDeposit,
        expected: u64,
    ) -> CoreResult<()> {
        self.replace_revision("lean_deposits", "deposit", &value.id, value, expected)
            .await
    }
    async fn list(
        &self,
        owner: &PersonId,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<mathesis_core::model::LeanDeposit>> {
        self.page(
            "lean_deposits",
            doc! {"owner": owner.as_str()},
            limit,
            before,
        )
        .await
    }
}
