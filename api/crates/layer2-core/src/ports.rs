//! Ports implemented outside Layer 2. Stores are the only authority for
//! state: a write that replaces an aggregate names the revision it read, and
//! a store refuses it if that revision has moved.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    CoreResult,
    ids::{PersonId, RecordId, SubmissionId},
    judgement::ClaimJudgement,
    model::{
        BlobRef, ClaimKind, Contribution, DonationGrant, Endorsement, Person, Record, RefereeFile,
        Role, Stage, Submission, Task, TaskKind, VerifiedIdentity,
    },
};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A page of results, newest first, with the cursor for the next page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing<T> {
    pub items: Vec<T>,
    pub next_before: Option<String>,
}

#[async_trait]
pub trait PersonStore: Send + Sync {
    async fn upsert_sign_in(
        &self,
        identity: &VerifiedIdentity,
        at: DateTime<Utc>,
    ) -> CoreResult<Person>;
    async fn get(&self, id: &PersonId) -> CoreResult<Option<Person>>;
    async fn list(&self, limit: u32, before: Option<String>) -> CoreResult<Listing<Person>>;
    async fn set_roles(&self, id: &PersonId, roles: &[Role]) -> CoreResult<Person>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubmissionFilter {
    pub involving: Option<PersonId>,
    pub status: Option<String>,
}

#[async_trait]
pub trait SubmissionStore: Send + Sync {
    async fn insert(&self, submission: &Submission) -> CoreResult<()>;
    /// Unique owner/producer/work/purpose identity for resumable manuscript delivery.
    async fn manuscript(&self, key: &str) -> CoreResult<Option<Submission>>;
    async fn get(&self, id: &SubmissionId) -> CoreResult<Option<Submission>>;
    async fn list(
        &self,
        filter: &SubmissionFilter,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Submission>>;
    async fn replace(&self, submission: &Submission, expected: u64) -> CoreResult<()>;
    /// Draft and in-review papers submitted by `person`.
    async fn active_count(&self, person: &PersonId) -> CoreResult<u32>;
}

#[async_trait]
pub trait EndorsementStore: Send + Sync {
    async fn insert(&self, endorsement: &Endorsement) -> CoreResult<()>;
    async fn for_submission(&self, submission: &SubmissionId) -> CoreResult<Vec<Endorsement>>;
}

#[async_trait]
pub trait RecordStore: Send + Sync {
    /// Select matching immutable inputs under live public visibility BEFORE
    /// pagination. No unrelated source prefix may limit discovery. Implementations
    /// bound output and execution time; a deadline is an error, not empty coverage.
    async fn search_research(
        &self,
        search: &crate::model::ResearchSearch,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Record>>;
    async fn search(
        &self,
        search: &crate::model::PaperSearch,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Record>>;
    async fn for_submission(&self, id: &SubmissionId) -> CoreResult<Option<Record>>;
    async fn next_sequence(&self, year: i32) -> CoreResult<u64>;
    async fn insert(&self, record: &Record) -> CoreResult<()>;
    async fn get(&self, id: &RecordId) -> CoreResult<Option<Record>>;
    async fn list(&self, limit: u32, before: Option<String>) -> CoreResult<Listing<Record>>;
}

/// Opaque file storage for source archives and PDFs.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Each write allocates a fresh id, including writes of identical bytes.
    async fn put(&self, bytes: &[u8], content_type: &str) -> CoreResult<BlobRef>;
    async fn get(&self, id: &str) -> CoreResult<Option<Vec<u8>>>;
    /// Discard a write only after the caller establishes that no saved object references it.
    async fn discard(&self, id: &str) -> CoreResult<()>;
}

/// One statement environment read from the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadStatement {
    pub kind: ClaimKind,
    pub display_name: String,
    pub title: Option<String>,
    pub latex_label: Option<String>,
    pub body: String,
    pub has_proof: bool,
    pub section: Option<String>,
}

/// What reading an uploaded LaTeX source yields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPaper {
    pub bibliography: Option<crate::model::Bibliography>,
    pub evidence: Vec<crate::model::EvidenceAttachment>,
    pub main_file: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub statements: Vec<ReadStatement>,
    /// Math macros the preamble defines, name → body.
    pub macros: std::collections::BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

/// Reads LaTeX sources (a `.tex`, `.zip` or `.tar.gz`).
pub trait PaperReader: Send + Sync {
    fn files(
        &self,
        _bytes: &[u8],
        _filename: &str,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
        Err("archive extraction is unavailable".into())
    }
    /// Read the structure; `Err` carries the reason for the author.
    fn read(&self, bytes: &[u8], filename: &str) -> Result<ReadPaper, String>;
}

/// Durable machine work: a review stage, compiling a version's PDF, or
/// advancing the current referee round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Stage(Stage),
    Compile,
    Referee,
    LeanStatement,
    ResearchTarget,
    BindingAudit,
    FeedbackReadback,
    VerifyResearch,
    OpenProblems,
    VerifyAttempt,
    ErrorReports,
    AgentAttempt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobLane {
    Compile,
    Review,
    Verify,
}

impl JobKind {
    pub fn lane(self) -> JobLane {
        match self {
            Self::Compile => JobLane::Compile,
            Self::VerifyAttempt | Self::VerifyResearch => JobLane::Verify,
            _ => JobLane::Review,
        }
    }
}

/// A job together with the opaque lease token that fences its worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeasedJob {
    pub submission: SubmissionId,
    pub kind: JobKind,
    pub attempts: u32,
    /// Fences completion and renewal against a lease that has since been
    /// re-issued to another worker.
    pub lease: String,
}

/// Durable review-job lease operations. Implementations must match the lease
/// token on every mutation; a stale worker may never renew or settle a job.
#[async_trait]
pub trait JobLease: Send + Sync {
    /// Lease the oldest available job.
    async fn claim(&self) -> CoreResult<Option<LeasedJob>> {
        self.claim_in(None).await
    }
    async fn claim_in(&self, lane: Option<JobLane>) -> CoreResult<Option<LeasedJob>>;
    /// Renew the lease for another lease length. `false` means the token no
    /// longer owns the job and the worker must discard any pending result.
    async fn renew(&self, job: &LeasedJob) -> CoreResult<bool>;
    async fn complete(&self, job: &LeasedJob) -> CoreResult<()>;
    /// Release a waiting job without recording a failure or consuming an attempt.
    async fn defer(&self, job: &LeasedJob, delay: std::time::Duration) -> CoreResult<()>;
    /// Return the job for a later retry, or park it as failed after the
    /// implementation's retry limit.
    async fn retry(&self, job: &LeasedJob, error: &str) -> CoreResult<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewJob {
    pub submission: SubmissionId,
    pub kind: JobKind,
    pub attempts: u32,
}

#[async_trait]
pub trait ReviewQueue: Send + Sync {
    /// Enqueue unless an unfinished job of the same kind exists.
    async fn enqueue(&self, submission: &SubmissionId, kind: JobKind) -> CoreResult<()>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilter {
    pub kind: Option<TaskKind>,
    pub status: Option<String>,
    pub submission: Option<SubmissionId>,
    pub holder: Option<PersonId>,
}

#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn insert_if_absent(&self, task: &Task) -> CoreResult<bool>;
    async fn get(&self, id: &str) -> CoreResult<Option<Task>>;
    async fn list(
        &self,
        filter: &TaskFilter,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Task>>;
    async fn replace(&self, task: &Task, expected: u64) -> CoreResult<()>;
    async fn active_leases(&self, holder: &PersonId, now: DateTime<Utc>) -> CoreResult<u32>;
    async fn expired_leases(&self, now: DateTime<Utc>, limit: u32) -> CoreResult<Vec<Task>>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContributionFilter {
    pub task: Option<String>,
    pub contributor: Option<PersonId>,
    pub status: Option<String>,
    pub kind: Option<TaskKind>,
}

#[async_trait]
pub trait ContributionStore: Send + Sync {
    async fn insert(&self, contribution: &Contribution) -> CoreResult<()>;
    async fn get(&self, id: &str) -> CoreResult<Option<Contribution>>;
    async fn list(
        &self,
        filter: &ContributionFilter,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<Contribution>>;
    async fn replace(&self, contribution: &Contribution, expected: u64) -> CoreResult<()>;
    async fn all_by(&self, contributor: Option<&PersonId>) -> CoreResult<Vec<Contribution>>;
}

#[async_trait]
pub trait JudgementStore: Send + Sync {
    async fn insert(&self, judgement: &ClaimJudgement) -> CoreResult<()>;
    async fn for_submission(&self, submission: &SubmissionId) -> CoreResult<Vec<ClaimJudgement>>;
}

#[async_trait]
pub trait RefereeStore: Send + Sync {
    /// Atomic, idempotent admission shared across all binding discussions: five
    /// contribution/challenge requests per person in a rolling 24-hour window.
    async fn admit_binding_submission(
        &self,
        person: &PersonId,
        request: &str,
        at: DateTime<Utc>,
    ) -> CoreResult<()>;
    /// Small directory of histories with binding boards, including revoked boards
    /// for cleanup. The application rechecks current authorization on every read.
    async fn binding_sources(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<SubmissionId>>;
    /// Small private proposal directory for recovering saved observation intents.
    async fn feedback_sources(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<SubmissionId>>;
    /// Small read projection; stores may derive and persist it under the history revision.
    async fn summary(
        &self,
        id: &SubmissionId,
        version: Option<u32>,
        claims_revision: u64,
    ) -> CoreResult<crate::model::ReviewSummary> {
        Ok(self
            .get(id)
            .await?
            .unwrap_or_else(|| RefereeFile::new(id.clone()))
            .summary_for(version, claims_revision))
    }
    async fn get(&self, submission: &SubmissionId) -> CoreResult<Option<RefereeFile>>;
    async fn insert(&self, file: &RefereeFile) -> CoreResult<()>;
    async fn replace(&self, file: &RefereeFile, expected: u64) -> CoreResult<()>;
}

#[async_trait]
pub trait GrantStore: Send + Sync {
    async fn all_grants(&self) -> CoreResult<Vec<DonationGrant>>;
    async fn get(&self, donor: &PersonId) -> CoreResult<Option<DonationGrant>>;
    async fn insert(&self, grant: &DonationGrant) -> CoreResult<()>;
    async fn replace(&self, grant: &DonationGrant, expected: u64) -> CoreResult<()>;
    async fn active(&self) -> CoreResult<Vec<DonationGrant>>;
}

/// Everything Layer 2 needs, bundled for composition.
pub struct Ports {
    pub deposits: Arc<dyn LeanDepositStore>,
    pub cma_bindings: Arc<dyn CmaBindingStore>,
    pub cma_setup: Arc<dyn CmaSetup>,
    pub solving: Arc<dyn SolvingStore>,
    pub verifier: Arc<dyn Verifier>,
    pub clock: Arc<dyn Clock>,
    pub people: Arc<dyn PersonStore>,
    pub submissions: Arc<dyn SubmissionStore>,
    pub endorsements: Arc<dyn EndorsementStore>,
    pub records: Arc<dyn RecordStore>,
    pub blobs: Arc<dyn BlobStore>,
    pub reader: Arc<dyn PaperReader>,
    pub queue: Arc<dyn ReviewQueue>,
    pub tasks: Arc<dyn TaskStore>,
    pub contributions: Arc<dyn ContributionStore>,
    pub judgements: Arc<dyn JudgementStore>,
    pub grants: Arc<dyn GrantStore>,
    pub referees: Arc<dyn RefereeStore>,
}

#[async_trait]
pub trait SolvingStore: Send + Sync {
    async fn list_admitted(
        &self,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<crate::model::SolveFile>>;
    async fn report_files(&self) -> CoreResult<Vec<crate::model::ErrorReportFile>>;
    async fn insert_report_file(&self, file: &crate::model::ErrorReportFile) -> CoreResult<()>;
    async fn replace_report_file(
        &self,
        file: &crate::model::ErrorReportFile,
        expected: u64,
    ) -> CoreResult<()>;

    async fn get(&self, id: &str) -> CoreResult<Option<crate::model::SolveFile>>;
    async fn all(&self) -> CoreResult<Vec<crate::model::SolveFile>>;
    async fn insert(&self, file: &crate::model::SolveFile) -> CoreResult<()>;
    async fn replace(&self, file: &crate::model::SolveFile, expected: u64) -> CoreResult<()>;
    async fn agents(&self) -> CoreResult<Vec<crate::model::Entrant>>;
    async fn insert_agent(&self, agent: &crate::model::Entrant) -> CoreResult<()>;
    async fn replace_agent(&self, agent: &crate::model::Entrant, expected: u64) -> CoreResult<()>;
}
#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(
        &self,
        request: &crate::model::VerificationRequest,
    ) -> CoreResult<crate::model::VerificationReceipt>;
}
pub struct UnavailableVerifier;
#[async_trait]
impl Verifier for UnavailableVerifier {
    async fn verify(
        &self,
        _: &crate::model::VerificationRequest,
    ) -> CoreResult<crate::model::VerificationReceipt> {
        Err(crate::CoreError::Unavailable(
            "verifier is not configured".into(),
        ))
    }
}

#[async_trait]
pub trait CmaBindingStore: Send + Sync {
    async fn get(&self, person: &PersonId) -> CoreResult<Option<crate::model::CmaBinding>>;
    async fn replace(&self, binding: &crate::model::CmaBinding, expected: u64) -> CoreResult<()>;
}
#[async_trait]
pub trait CmaSetup: Send + Sync {
    async fn revoke(&self, _: &PersonId, _: u64) -> CoreResult<()> {
        Ok(())
    }
    fn enabled(&self) -> bool {
        false
    }
    fn setup_url(&self) -> String {
        String::new()
    }
    async fn validate(
        &self,
        binding: &crate::model::CmaBinding,
        workspace: &str,
    ) -> CoreResult<crate::model::CmaWorkspaceCheck>;
}
pub struct UnavailableCmaSetup;
#[async_trait]
impl CmaSetup for UnavailableCmaSetup {
    async fn validate(
        &self,
        _: &crate::model::CmaBinding,
        _: &str,
    ) -> CoreResult<crate::model::CmaWorkspaceCheck> {
        Err(crate::CoreError::Unavailable(
            "CMA connection is disabled".into(),
        ))
    }
}

#[async_trait]
pub trait LeanDepositStore: Send + Sync {
    async fn get(&self, id: &str) -> CoreResult<Option<crate::model::LeanDeposit>>;
    async fn insert(&self, value: &crate::model::LeanDeposit) -> CoreResult<()>;
    async fn replace(&self, value: &crate::model::LeanDeposit, expected: u64) -> CoreResult<()>;
    async fn list(
        &self,
        owner: &PersonId,
        limit: u32,
        before: Option<String>,
    ) -> CoreResult<Listing<crate::model::LeanDeposit>>;
}
