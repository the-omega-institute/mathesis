use async_trait::async_trait;
use chrono::Utc;
use futures::TryStreamExt;
use mathesis_core::{
    CoreError, CoreResult,
    ids::SubmissionId,
    ports::{JobKind, JobLane, JobObservation, ProcessingLane, ProcessingWorker, ReviewQueue},
};
use mongodb::{
    bson::{DateTime as BsonDateTime, doc},
    options::ReturnDocument,
};

use super::{MongoStore, REVIEW_JOBS, is_duplicate_key, unavailable};
use crate::{
    auth,
    review::{JobLease, LeasedJob, MAX_ATTEMPTS, kind_key, parse_kind_key},
};

const LEASE: chrono::Duration = chrono::Duration::minutes(30);

fn job_id(submission: &SubmissionId, kind: JobKind) -> String {
    format!("{submission}:{}", kind_key(kind))
}

#[async_trait]
impl ReviewQueue for MongoStore {
    async fn enqueue(&self, submission: &SubmissionId, kind: JobKind) -> CoreResult<()> {
        let now = BsonDateTime::now();
        // Recovery must not shorten a continuing proof's saved backoff or
        // reset its retry budget. Other queues retain their existing behavior.
        let state = if kind == JobKind::AgentAttempt || kind.lane() == JobLane::Collection {
            doc! { "$nin": ["leased", "queued"] }
        } else {
            doc! { "$ne": "leased" }
        };
        let result = self
            .raw(REVIEW_JOBS)
            .update_one(
                doc! { "_id": job_id(submission, kind), "state": state },
                doc! {
                    "$set": { "state": "queued", "available_at": now, "queued_at": now, "attempts": 0_i32, "last_error": null },
                    "$unset": { "finished_at": "" },
                    "$setOnInsert": { "submission": submission.as_str(), "kind": kind_key(kind), "created_at": now },
                },
            )
            .upsert(true)
            .await;
        match result {
            Ok(_) => Ok(()),
            // The upsert collided with the leased job's _id: already in flight.
            Err(error) if is_duplicate_key(&error) => Ok(()),
            Err(error) => Err(unavailable(error)),
        }
    }

    async fn processing_status(&self) -> CoreResult<Vec<ProcessingLane>> {
        let now = BsonDateTime::now();
        let mut out = Vec::new();
        for lane in [
            JobLane::Compile,
            JobLane::Review,
            JobLane::Collection,
            JobLane::Verify,
        ] {
            let mut filter = lane_filter(lane);
            // Retired work keeps its history without inflating the live queue's retries.
            filter.insert("state", doc! {"$ne":"superseded"});
            let mut status = ProcessingLane {
                lane,
                queued: 0,
                running: 0,
                failed: 0,
                retries: 0,
                oldest_queued_at: None,
                last_success: None,
                last_failure: None,
                workers: vec![],
            };
            let mut groups = self.raw(REVIEW_JOBS).aggregate(vec![
                doc! {"$match": filter.clone()},
                doc! {"$group": {
                    "_id": {"$cond": [
                        {"$and": [{"$eq": ["$state", "leased"]}, {"$lte": ["$lease_until", now]}]},
                        "queued", "$state"
                    ]},
                    "count": {"$sum": 1_i64},
                    "retries": {"$sum": {"$ifNull": ["$retries", 0_i64]}},
                    "oldest": {"$min": {"$ifNull": ["$queued_at", "$created_at"]}},
                }},
            ]).await.map_err(unavailable)?;
            while let Some(group) = groups.try_next().await.map_err(unavailable)? {
                let count = group.get_i64("count").unwrap_or(0).max(0) as u64;
                status.retries += group.get_i64("retries").unwrap_or(0).max(0) as u64;
                match group.get_str("_id").unwrap_or("") {
                    "queued" => {
                        status.queued = count;
                        status.oldest_queued_at =
                            group.get_datetime("oldest").ok().map(|d| d.to_chrono());
                    }
                    "leased" => status.running = count,
                    "failed" => status.failed = count,
                    _ => {}
                }
            }
            for state in ["completed", "failed"] {
                let mut query = filter.clone();
                query.insert("state", state);
                if let Some(row) = self
                    .raw(REVIEW_JOBS)
                    .find_one(query)
                    .sort(doc! {"finished_at": -1})
                    .await
                    .map_err(unavailable)?
                {
                    let observation = observation(&row);
                    if state == "completed" {
                        status.last_success = observation;
                    } else {
                        status.last_failure = observation;
                    }
                }
            }
            // A waiting job can report an actionable blocked stage without being
            // declared terminal or spending a retry. Keep this staff-only.
            let mut waiting = filter;
            waiting.insert("state", "queued");
            waiting.insert("last_error", doc! {"$type": "string"});
            if let Some(row) = self
                .raw(REVIEW_JOBS)
                .find_one(waiting)
                .sort(doc! {"updated_at": -1})
                .await
                .map_err(unavailable)?
                && let Some(observation) = observation(&row)
                && status
                    .last_failure
                    .as_ref()
                    .is_none_or(|last| last.at < observation.at)
            {
                status.last_failure = Some(observation);
            }
            let mut workers = self
                .raw("processing_workers")
                .find(doc! {"lane": lane_key(lane), "expires_at": {"$gt": now}})
                .await
                .map_err(unavailable)?;
            while let Some(row) = workers.try_next().await.map_err(unavailable)? {
                status.workers.push(ProcessingWorker {
                    worker: row.get_str("worker").unwrap_or("").into(),
                    lane,
                    capacity: row.get_i32("capacity").unwrap_or(0).max(0) as u32,
                    workspace: row.get_str("workspace").ok().map(str::to_owned),
                    observed_at: row
                        .get_datetime("observed_at")
                        .map_err(|_| {
                            CoreError::Unavailable("invalid worker observation date".into())
                        })?
                        .to_chrono(),
                    expires_at: row
                        .get_datetime("expires_at")
                        .map_err(|_| CoreError::Unavailable("invalid worker expiry date".into()))?
                        .to_chrono(),
                });
            }
            out.push(status);
        }
        Ok(out)
    }
}

fn observation(row: &mongodb::bson::Document) -> Option<JobObservation> {
    Some(JobObservation {
        submission: row.get_str("submission").ok()?.into(),
        stage: row.get_str("kind").ok()?.into(),
        at: row
            .get_datetime("finished_at")
            .or_else(|_| row.get_datetime("updated_at"))
            .ok()?
            .to_chrono(),
        reason: row.get_str("last_error").ok().map(str::to_owned),
    })
}

fn lane_key(lane: JobLane) -> &'static str {
    match lane {
        JobLane::Compile => "compile",
        JobLane::Review => "review",
        JobLane::Collection => "collection",
        JobLane::Verify => "verify",
    }
}

const COLLECTION_KINDS: [&str; 2] = ["collected_explanation", "collected_formalization"];

fn lane_filter(lane: JobLane) -> mongodb::bson::Document {
    match lane {
        JobLane::Compile => doc! {"kind": "compile"},
        JobLane::Verify => doc! {"kind": {"$in": VERIFY_KINDS.to_vec()}},
        JobLane::Collection => doc! {"kind": {"$in": COLLECTION_KINDS.to_vec()}},
        JobLane::Review => {
            let mut other = vec!["compile"];
            other.extend(VERIFY_KINDS);
            other.extend(COLLECTION_KINDS);
            doc! {"kind": {"$nin": other}}
        }
    }
}

/// Stored kinds of `JobLane::Verify`; the review lane takes every other kind
/// except compilation. Keep in step with `JobKind::lane`.
const VERIFY_KINDS: [&str; 3] = [
    "verify_attempt",
    "verify_research",
    "collected_problem_target",
];

#[async_trait]
impl JobLease for MongoStore {
    async fn claim_in(&self, lane: Option<JobLane>) -> CoreResult<Option<LeasedJob>> {
        let now = Utc::now();
        let lease = auth::random_token();
        let mut filter = doc! {
            "$or": [
                { "state": "queued", "available_at": { "$lte": BsonDateTime::from_chrono(now) } },
                { "state": "leased", "lease_until": { "$lte": BsonDateTime::from_chrono(now) } },
            ]
        };
        if let Some(lane) = lane {
            filter.extend(lane_filter(lane));
        }
        let found = self
            .raw(REVIEW_JOBS)
            .find_one_and_update(
                filter,
                doc! {
                    "$set": { "state": "leased", "updated_at": BsonDateTime::from_chrono(now), "lease": &lease, "lease_until": BsonDateTime::from_chrono(now + LEASE) },
                    "$inc": { "attempts": 1_i32 },
                },
            )
            .sort(doc! { "available_at": 1 })
            .return_document(ReturnDocument::After)
            .await
            .map_err(unavailable)?;
        let Some(document) = found else {
            return Ok(None);
        };
        let field = |name: &str| {
            document
                .get_str(name)
                .map(str::to_owned)
                .map_err(|_| CoreError::Unavailable(format!("review job lacks {name}")))
        };
        let key = field("kind")?;
        let kind = parse_kind_key(&key)
            .ok_or_else(|| CoreError::Unavailable(format!("review job kind {key:?}")))?;
        Ok(Some(LeasedJob {
            submission: SubmissionId(field("submission")?),
            kind,
            attempts: document.get_i32("attempts").unwrap_or(1).max(0) as u32,
            lease,
        }))
    }

    async fn renew(&self, job: &LeasedJob) -> CoreResult<bool> {
        let now = Utc::now();
        let result = self
            .raw(REVIEW_JOBS)
            .update_one(
                doc! {
                    "_id": job_id(&job.submission, job.kind),
                    "state": "leased",
                    "lease": &job.lease,
                    // Once the lease has expired, another worker may claim it;
                    // the old token must not be able to revive it.
                    "lease_until": { "$gt": BsonDateTime::from_chrono(now) },
                },
                doc! {
                    "$set": { "lease_until": BsonDateTime::from_chrono(now + LEASE) },
                },
            )
            .await
            .map_err(unavailable)?;
        Ok(result.matched_count == 1)
    }

    async fn complete(&self, job: &LeasedJob) -> CoreResult<()> {
        self.raw(REVIEW_JOBS)
            .update_one(doc! { "_id": job_id(&job.submission, job.kind), "state": "leased", "lease": &job.lease, "lease_until": { "$gt": BsonDateTime::now() } },
                doc! {"$set": {"state": "completed", "finished_at": BsonDateTime::now(), "last_error": null}, "$unset": {"lease": "", "lease_until": ""}})
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn defer(&self, job: &LeasedJob, delay: std::time::Duration) -> CoreResult<()> {
        let delay = chrono::Duration::from_std(delay)
            .map_err(|_| CoreError::invalid("job deferral is too long"))?;
        self.raw(REVIEW_JOBS).update_one(
            doc! { "_id": job_id(&job.submission, job.kind), "state": "leased", "lease": &job.lease, "lease_until": { "$gt": BsonDateTime::now() } },
            doc! {
                "$set": { "state": "queued", "updated_at": BsonDateTime::now(), "available_at": BsonDateTime::from_chrono(Utc::now() + delay) },
                "$inc": { "attempts": -1_i32 },
                "$unset": { "lease": "", "lease_until": "" },
            },
        ).await.map_err(unavailable)?;
        Ok(())
    }

    async fn retry(&self, job: &LeasedJob, error: &str) -> CoreResult<()> {
        let (state, delay) = if job.attempts >= MAX_ATTEMPTS {
            ("failed", chrono::Duration::zero())
        } else {
            (
                "queued",
                chrono::Duration::seconds(30 * i64::from(job.attempts)),
            )
        };
        self.raw(REVIEW_JOBS)
            .update_one(
                doc! { "_id": job_id(&job.submission, job.kind), "state": "leased", "lease": &job.lease, "lease_until": { "$gt": BsonDateTime::now() } },
                doc! {
                    "$set": {
                        "state": state,
                        "updated_at": BsonDateTime::now(),
                        "finished_at": if state == "failed" { Some(BsonDateTime::now()) } else { None },
                        "available_at": BsonDateTime::from_chrono(Utc::now() + delay),
                        "last_error": error.chars().take(2_000).collect::<String>(),
                    },
                    "$inc": { "retries": 1_i64 },
                    "$unset": { "lease": "", "lease_until": "" },
                },
            )
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn fail(&self, job: &LeasedJob, error: &str) -> CoreResult<()> {
        self.raw(REVIEW_JOBS).update_one(
            doc! {"_id": job_id(&job.submission, job.kind), "state": "leased", "lease": &job.lease, "lease_until": {"$gt": BsonDateTime::now()}},
            doc! {"$set": {"state": "failed", "finished_at": BsonDateTime::now(), "last_error": error.chars().take(2000).collect::<String>()}, "$unset": {"lease": "", "lease_until": ""}},
        ).await.map_err(unavailable)?;
        Ok(())
    }

    async fn wait(
        &self,
        job: &LeasedJob,
        delay: std::time::Duration,
        reason: &str,
    ) -> CoreResult<()> {
        let delay = chrono::Duration::from_std(delay)
            .map_err(|_| CoreError::invalid("job deferral is too long"))?;
        self.raw(REVIEW_JOBS).update_one(
            doc! {"_id": job_id(&job.submission, job.kind), "state": "leased", "lease": &job.lease, "lease_until": {"$gt": BsonDateTime::now()}},
            doc! {"$set": {"state": "queued", "updated_at": BsonDateTime::now(), "last_error": reason.chars().take(2000).collect::<String>(), "available_at": BsonDateTime::from_chrono(Utc::now() + delay)}, "$inc": {"attempts": -1_i32, "retries": 1_i64}, "$unset": {"lease": "", "lease_until": ""}},
        ).await.map_err(unavailable)?;
        Ok(())
    }

    async fn observe_worker(&self, worker: &ProcessingWorker) -> CoreResult<()> {
        self.raw("processing_workers").update_one(
            doc! {"_id": format!("{}:{}", worker.worker, lane_key(worker.lane))},
            doc! {"$set": {"worker": &worker.worker, "lane": lane_key(worker.lane), "capacity": worker.capacity as i32,
                "workspace": &worker.workspace, "observed_at": BsonDateTime::from_chrono(worker.observed_at), "expires_at": BsonDateTime::from_chrono(worker.expires_at)}},
        ).upsert(true).await.map_err(unavailable)?;
        Ok(())
    }
}
