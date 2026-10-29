//! MongoDB integration tests. They run when `WISHPOOL_TEST_MONGODB_URI` names
//! a disposable server and are skipped otherwise; CI provides a MongoDB
//! service. Each test uses its own database and drops it afterwards.

use std::{collections::BTreeSet, sync::Arc};

use chrono::Utc;
use mathesis_core::{
    CoreError,
    app::{App, Upload},
    ids::{ClaimId, PersonId, RecordId, SubmissionId},
    model::*,
    policy::Policy,
    ports::*,
};

use super::*;
use crate::{
    auth::{AuthStore, LoginAttempt, SessionRecord},
    latex::LatexReader,
    review::JobLease,
};

struct TestDb {
    store: MongoStore,
    client: Client,
    name: String,
}

impl TestDb {
    async fn open() -> Option<Self> {
        let Ok(uri) = std::env::var("WISHPOOL_TEST_MONGODB_URI") else {
            eprintln!("WISHPOOL_TEST_MONGODB_URI not set; skipping MongoDB test");
            return None;
        };
        let name = format!("wishpool_test_{}", uuid_like());
        let store = MongoStore::connect(&uri, &name)
            .await
            .expect("connect to test MongoDB");
        let client = Client::with_uri_str(&uri).await.unwrap();
        Some(Self {
            store,
            client,
            name,
        })
    }

    async fn drop(self) {
        self.client.database(&self.name).drop().await.unwrap();
    }
}

fn uuid_like() -> String {
    crate::auth::random_token()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(16)
        .collect()
}

#[tokio::test]
async fn knowledge_snapshots_survive_restart_and_preserve_large_index_and_artifacts() {
    use mathesis_review::knowledge::{
        DeclarationIdentity, PublicationIdentity, PublishedDeclaration, index::SourceIndex, sha256,
    };
    let Some(db) = TestDb::open().await else {
        return;
    };
    let publication = PublicationIdentity {
        bundle_digest: sha256(b"publication"),
        source_commit: "a".repeat(40),
        source_tree: "b".repeat(40),
    };
    let source = "x".repeat(1_800_000);
    let catalog = (0..10)
        .map(|i| PublishedDeclaration {
            identity: DeclarationIdentity {
                snapshot: publication.bundle_digest.clone(),
                module_path: format!("M{i}.lean"),
                name_key: "x".into(),
                statement_id: sha256(format!("decl{i}").as_bytes()),
                type_sha256: sha256(b"type"),
            },
            name: "x".into(),
            kind: "theorem".into(),
            frozen_node_id: sha256(format!("node{i}").as_bytes()),
            freeze_status: "frozen".into(),
            source_sha256: sha256(source.as_bytes()),
            axioms: vec![],
            module_imports: vec![],
            prerequisite_frozen_node_ids: vec![],
        })
        .collect();
    let index = SourceIndex::build(publication.clone(), catalog, |_| {
        Ok(source.as_bytes().to_vec())
    })
    .unwrap();
    let blob = db.store.put_knowledge_index(&index).await.unwrap();
    assert!(blob.bytes > 16 * 1024 * 1024);
    let artifact = db
        .store
        .put(b"original artifact", "application/json")
        .await
        .unwrap();
    let snapshot = super::knowledge::KnowledgeSnapshot {
        id: index.revision.clone(),
        publication,
        index: blob,
        artifacts: std::collections::BTreeMap::from([("original.json".into(), artifact.clone())]),
        candidate_modules: 10,
        indexed_modules: 10,
        omitted_modules: 0,
        admitted_at: Utc::now(),
        admission: serde_json::json!({"test_only":true}),
    };
    db.store.insert_knowledge_snapshot(&snapshot).await.unwrap();
    let reopened = MongoStore::connect(
        &std::env::var("WISHPOOL_TEST_MONGODB_URI").unwrap(),
        &db.name,
    )
    .await
    .unwrap();
    let loaded = reopened
        .load_knowledge_index(&index.revision)
        .await
        .unwrap();
    assert_eq!(loaded.revision, index.revision);
    assert_eq!(loaded.modules[9].source, source);
    assert_eq!(
        reopened.read_checked_blob(&artifact, 100).await.unwrap(),
        b"original artifact"
    );
    assert!(reopened.read_checked_blob(&artifact, 1).await.is_err());
    // Exact re-admission preserves the original immutable snapshot.
    let replay = reopened.insert_knowledge_snapshot(&snapshot).await.unwrap();
    assert_eq!(replay.index.id, snapshot.index.id);
    let mut different = snapshot.clone();
    different.index.sha256 = "0".repeat(64);
    assert!(matches!(
        reopened.insert_knowledge_snapshot(&different).await,
        Err(CoreError::Conflict(_))
    ));
    // Existing offline GC sees index and original artifacts in the Mongo header.
    reopened.raw("papers.files").update_many(mongodb::bson::doc! {}, mongodb::bson::doc! {"$set":{"uploadDate":mongodb::bson::DateTime::from_chrono(Utc::now()-chrono::Duration::days(10))}}).await.unwrap();
    let audit = reopened.collect_orphan_files_offline().await.unwrap();
    assert!(audit.orphan_files.is_empty());
    assert!(audit.missing_files.is_empty());
    // Header corruption must fail before any query can consume the index.
    reopened
        .raw(super::knowledge::KNOWLEDGE_SNAPSHOTS)
        .update_one(
            mongodb::bson::doc! {"id":&index.revision},
            mongodb::bson::doc! {"$set":{"index.sha256":"0".repeat(64)}},
        )
        .await
        .unwrap();
    assert!(
        reopened
            .load_knowledge_index(&index.revision)
            .await
            .unwrap_err()
            .to_string()
            .contains("checksum mismatch")
    );
    reopened.shutdown().await;
    db.drop().await;
}

#[tokio::test]
async fn knowledge_archive_upload_rejects_changed_or_oversized_bytes() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("public.zip");
    std::fs::write(&path, b"bounded public archive").unwrap();
    let hash = mathesis_review::knowledge::sha256(b"bounded public archive");
    let blob = db
        .store
        .put_knowledge_file(&path, &hash, 100)
        .await
        .unwrap();
    assert_eq!(
        db.store.read_checked_blob(&blob, 100).await.unwrap(),
        b"bounded public archive"
    );
    assert!(db.store.put_knowledge_file(&path, &hash, 1).await.is_err());
    assert!(
        db.store
            .put_knowledge_file(
                &path,
                &mathesis_review::knowledge::sha256(b"different"),
                100
            )
            .await
            .is_err()
    );
    assert_eq!(
        db.store
            .raw("papers.files")
            .count_documents(mongodb::bson::doc! {})
            .await
            .unwrap(),
        1
    );
    db.drop().await;
}

fn submission(id: &str, submitter: &str, status: SubmissionStatus) -> Submission {
    Submission {
        intake: None,
        manuscript: None,
        purpose: Default::default(),
        review_scope: Default::default(),
        processing_consent: None,
        problem_check_requested: false,
        conjecture_dependencies: vec![],
        kind: mathesis_core::model::SubmissionKind::Paper,
        lean_statements: vec![],
        id: SubmissionId(id.into()),
        submitter: PersonId(submitter.into()),
        title: "t".into(),
        abstract_text: String::new(),
        authors: vec![],
        ai_disclosure: AiDisclosure {
            level: AiUse::None,
            statement: "none".into(),
        },
        msc: vec![],
        doi: None,
        versions: vec![],
        extracted: vec![],
        claims: vec![],
        claims_revision: 0,
        reports: vec![],
        status,
        decision: None,
        open_to_contributors: false,
        analysis_visibility: Visibility::Undecided,
        formalization: FormalizationPlan::default(),
        conjectures: vec![],
        published_progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        revision: 0,
    }
}

#[tokio::test]
async fn people_submissions_and_revision_fencing() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let identity = VerifiedIdentity {
        subject: "u1".into(),
        name: Some("Ada".into()),
        email: Some("a@x.org".into()),
        picture: None,
    };
    let person = s.upsert_sign_in(&identity, Utc::now()).await.unwrap();
    assert_eq!(person.display_name, "Ada");
    let person = s
        .set_roles(&person.id, &[Role::Editor, Role::Editor, Role::Admin])
        .await
        .unwrap();
    assert_eq!(person.roles, BTreeSet::from([Role::Editor, Role::Admin]));
    // Signing in again refreshes the profile and keeps roles.
    let again = s
        .upsert_sign_in(
            &VerifiedIdentity {
                email: None,
                ..identity
            },
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(again.roles.len(), 2);
    assert_eq!(again.email, None);

    let statuses = [
        SubmissionStatus::Draft,
        SubmissionStatus::InReview,
        SubmissionStatus::NotAccepted,
        SubmissionStatus::Withdrawn,
        SubmissionStatus::InReview,
    ];
    for (i, status) in statuses.into_iter().enumerate() {
        SubmissionStore::insert(s, &submission(&format!("s{i}"), "u1", status))
            .await
            .unwrap();
    }
    assert_eq!(s.active_count(&"u1".into()).await.unwrap(), 3);
    let mine = SubmissionFilter {
        involving: Some("u1".into()),
        status: None,
    };
    let first = SubmissionStore::list(s, &mine, 2, None).await.unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(|w| w.id.as_str())
            .collect::<Vec<_>>(),
        ["s4", "s3"]
    );
    let second = SubmissionStore::list(s, &mine, 2, first.next_before)
        .await
        .unwrap();
    assert_eq!(
        second
            .items
            .iter()
            .map(|w| w.id.as_str())
            .collect::<Vec<_>>(),
        ["s2", "s1"]
    );
    let queue = SubmissionFilter {
        involving: None,
        status: Some("in_review".into()),
    };
    assert_eq!(
        SubmissionStore::list(s, &queue, 10, None)
            .await
            .unwrap()
            .items
            .len(),
        2
    );

    let mut one = SubmissionStore::get(s, &"s1".into())
        .await
        .unwrap()
        .unwrap();
    one.title = "changed".into();
    SubmissionStore::replace(s, &one, 0).await.unwrap();
    assert!(matches!(
        SubmissionStore::replace(s, &one, 0).await,
        Err(CoreError::StaleRevision { .. })
    ));
    assert_eq!(
        SubmissionStore::get(s, &"s1".into())
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    let ghost = submission("nope", "u1", SubmissionStatus::Draft);
    assert!(matches!(
        SubmissionStore::replace(s, &ghost, 0).await,
        Err(CoreError::NotFound { .. })
    ));
    db.drop().await;
}

#[tokio::test]
async fn blobs_beyond_the_document_limit() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let big: Vec<u8> = (0..17 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let stored = BlobStore::put(s, &big, "application/octet-stream")
        .await
        .unwrap();
    assert_eq!(stored.bytes, big.len() as u64);
    assert_eq!(stored.sha256.len(), 64);
    assert_eq!(BlobStore::get(s, &stored.id).await.unwrap(), Some(big));
    assert_eq!(BlobStore::get(s, "missing").await.unwrap(), None);
    BlobStore::discard(s, &stored.id).await.unwrap();
    assert_eq!(BlobStore::get(s, &stored.id).await.unwrap(), None);
    db.drop().await;
}

const PAPER: &str = r"\documentclass{article}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\newtheorem{lemma}[theorem]{Lemma}
\newtheorem{conjecture}{Conjecture}
\title{Gaps between zero runs}
\author{A. Author}
\begin{document}
\maketitle
\begin{abstract}We bound the gaps.\end{abstract}
\begin{lemma}\label{lem:count} The count is finite.\end{lemma}
\begin{proof}By compactness.\end{proof}
\begin{theorem}[Main]\label{thm:main} Every gap is at most $3$.\end{theorem}
\begin{proof}By Lemma~\ref{lem:count}.\end{proof}
\begin{conjecture} Every gap is at most $2$.\end{conjecture}
\end{document}
";

#[tokio::test]
async fn paper_flow_against_mongo() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let shared = Arc::new(db.store.clone());
    let ports = Ports {
        deposits: Arc::new(db.store.clone()),
        cma_bindings: shared.clone(),
        cma_setup: Arc::new(mathesis_core::ports::UnavailableCmaSetup),
        solving: shared.clone(),
        verifier: Arc::new(mathesis_core::ports::UnavailableVerifier),
        clock: Arc::new(SystemClock),
        people: shared.clone(),
        submissions: shared.clone(),
        endorsements: shared.clone(),
        records: shared.clone(),
        blobs: shared.clone(),
        reader: Arc::new(LatexReader),
        queue: shared.clone(),
        tasks: shared.clone(),
        contributions: shared.clone(),
        judgements: shared.clone(),
        grants: shared.clone(),
        referees: shared.clone(),
    };
    let app = App::new(
        ports,
        Policy::default(),
        BTreeSet::from([PersonId::from("admin")]),
    );
    let caller = |sub: &'static str, roles: &'static [Role]| {
        let app = app.clone();
        async move {
            let identity = |s: &str| VerifiedIdentity {
                subject: s.into(),
                name: Some(s.into()),
                email: None,
                picture: None,
            };
            app.sign_in(&identity(sub)).await.unwrap();
            if !roles.is_empty() {
                let admin = app.caller(&identity("admin")).await.unwrap();
                app.set_roles(&admin, &sub.into(), roles).await.unwrap();
            }
            app.caller(&identity(sub)).await.unwrap()
        }
    };
    let author = caller("author", &[]).await;
    let editor = caller("editor", &[Role::Editor]).await;
    let engine = caller("engine", &[Role::Reviewer]).await;
    let helper = caller("helper", &[]).await;

    let new = NewPaper {
        title: None,
        abstract_text: None,
        purpose: Default::default(),
        review_scope: Default::default(),
        processing_consent: Some("ai-review-2026-10-11".into()),
        kind: mathesis_core::model::SubmissionKind::Paper,
        make_public_after_acceptance: true,
        typed_conjecture: None,
        ai_disclosure: AiDisclosure {
            level: AiUse::Assisted,
            statement: "A model proofread the lemma.".into(),
        },
        authors: vec![],
        msc: vec!["11B83".into()],
        doi: Some("  doi:10.48550/arXiv.2609.33421  ".into()),
        open_to_contributors: true,
    };
    let upload = Upload {
        filename: "gaps.tex".into(),
        bytes: PAPER.as_bytes().to_vec(),
    };
    let paper = app.submit_paper(&author, new, upload).await.unwrap();
    assert_eq!(paper.doi.as_deref(), Some("10.48550/arXiv.2609.33421"));
    assert_eq!(
        app.submission(&author, &paper.id).await.unwrap().doi,
        paper.doi
    );
    assert_eq!(paper.title, "Gaps between zero runs");
    assert_eq!(paper.authors[0].name, "A. Author");
    let kinds: Vec<ClaimKind> = paper.extracted.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [ClaimKind::Lemma, ClaimKind::Theorem, ClaimKind::Conjecture]
    );
    let source = app
        .paper_file(Some(&author), &paper.id, None, false)
        .await
        .unwrap();
    assert_eq!(source.bytes, PAPER.as_bytes());

    let job = JobLease::claim(&db.store).await.unwrap().unwrap();
    assert_eq!(job.kind, JobKind::Compile);
    app.record_compilation(&engine, &paper.id, 1, Ok(b"%PDF-1.7".to_vec()))
        .await
        .unwrap();
    JobLease::complete(&db.store, &job).await.unwrap();

    let confirmations = paper
        .extracted
        .iter()
        .map(|c| ClaimConfirmation {
            depends_on_conjectures: vec![],
            id: c.id.clone(),
            kind: c.kind,
            role: c.role,
            depends_on: if c.kind == ClaimKind::Theorem {
                vec!["C1".into()]
            } else {
                vec![]
            },
            settles: None,
            excluded: false,
        })
        .collect();
    let paper = app
        .confirm_claims(&author, &paper.id, confirmations)
        .await
        .unwrap();
    assert_eq!(paper.status, SubmissionStatus::InReview);
    let next = JobLease::claim(&db.store).await.unwrap().unwrap();
    assert_eq!(next.kind, JobKind::Stage(Stage::Literature));

    // Contributors corroborate nothing alone; the editor's report and
    // judgement decide.
    let tasks = app
        .list_tasks(
            TaskFilter {
                submission: Some(paper.id.clone()),
                ..TaskFilter::default()
            },
            None,
            None,
        )
        .await
        .unwrap()
        .items;
    assert_eq!(tasks.len(), 3);
    let judge_main = tasks
        .iter()
        .find(|t| t.kind == TaskKind::JudgeEscape && t.target.claim == ClaimId::from("C2"))
        .unwrap();
    app.lease_task(&helper, &judge_main.id, ContributionMode::OwnAgent)
        .await
        .unwrap();
    app.submit_contribution(
        &helper,
        &judge_main.id,
        NewContribution {
            agent: AgentInfo {
                tool: "codex".into(),
                model: "gpt-5.5".into(),
            },
            output: ContributionOutput::Judgement {
                shape: ProofShape::Content,
                witnesses: vec!["the counting bound".into()],
                rationale: "New estimate.".into(),
            },
            tokens: None,
        },
    )
    .await
    .unwrap();

    let literature = ReportDraft {
        outcome: Outcome::Pass,
        summary: "Inspected reported sources.".into(),
        payload: StagePayload::Literature {
            prior: vec![],
            searched: vec!["Opened source: zero run gaps".into()],
        },
        evidence: vec![],
    };
    app.file_report(&editor, &paper.id, Stage::Literature, literature, None)
        .await
        .unwrap();
    app.judge_claim(
        &editor,
        &paper.id,
        &"C2".into(),
        ProofShape::Content,
        vec!["the counting bound".into()],
        "Agreed.".into(),
    )
    .await
    .unwrap();
    assert_eq!(app.judgements(&editor, &paper.id).await.unwrap().len(), 2);
    let contributions = app
        .list_contributions(
            ContributionFilter {
                contributor: Some("helper".into()),
                ..Default::default()
            },
            None,
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        contributions.items[0].status,
        ContributionStatus::Verified { .. }
    ));
    app.adopt_judgements(&editor, &paper.id).await.unwrap();
    let paper = app.decide(&editor, &paper.id).await.unwrap();
    let SubmissionStatus::Accepted { record } = &paper.status else {
        panic!("expected acceptance, got {:?}", paper.decision)
    };
    assert!(record.as_str().starts_with("WP-"));

    let public = app.paper(record).await.unwrap();
    assert_eq!(
        public.summary.doi.as_deref(),
        Some("10.48550/arXiv.2609.33421")
    );
    assert_eq!(public.claims.len(), 3);
    assert!(
        serde_json::to_value(&public)
            .unwrap()
            .get("analysis")
            .is_none()
    );
    app.set_analysis_visibility(&author, &paper.id, Visibility::Public)
        .await
        .unwrap();
    let public = app.paper(&RecordId(record.0.clone())).await.unwrap();
    assert_eq!(public.claims.len(), 3);
    assert_eq!(
        app.analysis(&author, &paper.id)
            .await
            .unwrap()
            .main_with_content,
        1
    );
    assert_eq!(app.list_papers(None, None).await.unwrap().items.len(), 1);
    let pdf = app.paper_file(None, &paper.id, None, true).await.unwrap();
    assert_eq!(pdf.bytes, b"%PDF-1.7");
    assert_eq!(app.credits(None).await.unwrap()[0].verified, 1);
    db.drop().await;
}

#[tokio::test]
async fn sessions_attempts_and_job_retries() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let live = SessionRecord {
        subject: "u".into(),
        expires_at: Utc::now() + chrono::Duration::hours(1),
    };
    let dead = SessionRecord {
        subject: "u".into(),
        expires_at: Utc::now() - chrono::Duration::hours(1),
    };
    s.create_session("live", &live).await.unwrap();
    s.create_session("dead", &dead).await.unwrap();
    assert_eq!(
        s.session("live").await.unwrap().unwrap().subject,
        PersonId::from("u")
    );
    assert!(
        s.session("dead").await.unwrap().is_none(),
        "expired sessions are not honoured before TTL deletion"
    );
    s.delete_session("live").await.unwrap();
    assert!(s.session("live").await.unwrap().is_none());

    let attempt = LoginAttempt {
        cma_review: None,
        binding: "b".into(),
        verifier: "v".into(),
        nonce: "n".into(),
        return_to: "/".into(),
        // BSON dates hold milliseconds.
        expires_at: chrono::DateTime::from_timestamp_millis(
            Utc::now().timestamp_millis() + 600_000,
        )
        .unwrap(),
        donation: None,
    };
    s.put_attempt("st", &attempt).await.unwrap();
    assert_eq!(s.take_attempt("st").await.unwrap(), Some(attempt));
    assert_eq!(s.take_attempt("st").await.unwrap(), None);

    let sub = SubmissionId("s1".into());
    ReviewQueue::enqueue(s, &sub, JobKind::Compile)
        .await
        .unwrap();
    let job = JobLease::claim(s).await.unwrap().unwrap();
    assert_eq!((job.attempts, job.kind), (1, JobKind::Compile));
    JobLease::retry(s, &job, "boom").await.unwrap();
    assert!(
        JobLease::claim(s).await.unwrap().is_none(),
        "retry is delayed"
    );
    // A new enqueue makes the job available immediately.
    ReviewQueue::enqueue(s, &sub, JobKind::Compile)
        .await
        .unwrap();
    let again = JobLease::claim(s).await.unwrap().unwrap();
    // Completing with a stale lease does nothing.
    JobLease::complete(s, &job).await.unwrap();
    assert_eq!(
        s.raw(REVIEW_JOBS).count_documents(doc! {}).await.unwrap(),
        1
    );
    JobLease::complete(s, &again).await.unwrap();
    assert_eq!(
        s.raw(REVIEW_JOBS).count_documents(doc! {}).await.unwrap(),
        0
    );
    ReviewQueue::enqueue(s, &sub, JobKind::Stage(Stage::Literature))
        .await
        .unwrap();
    let stage = JobLease::claim(s).await.unwrap().unwrap();
    assert_eq!(stage.kind, JobKind::Stage(Stage::Literature));
    db.drop().await;
}

#[tokio::test]
async fn referee_files_are_unique_and_revision_fenced() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let mut file = RefereeFile::new("paper".into());
    file.agent_steps.insert("wishpool:p:r1:audit:a0".into(),AgentStep{cma_attempts: vec![],version:1,claims_revision:2,progress:serde_json::json!({"agent_id":"agt_saved","response_id":"resp_saved","next_chunk":2,"deadline":1800000000,"answer":null})});
    assert!(RefereeStore::get(s, &file.id).await.unwrap().is_none());
    RefereeStore::insert(s, &file).await.unwrap();
    assert!(matches!(
        RefereeStore::insert(s, &file).await,
        Err(CoreError::Conflict(_))
    ));
    file.letters.push(FeedbackLetter {
        round: None,
        assessment: Some(Recommendation::MinorRevision),
        subject: "Feedback".into(),
        body: "Useful point".into(),
        note: String::new(),
        edited: true,
        sent_by: "editor".into(),
        sent_at: Utc::now(),
    });
    RefereeStore::replace(s, &file, 0).await.unwrap();
    let stored = RefereeStore::get(s, &file.id).await.unwrap().unwrap();
    assert_eq!(stored.revision, 1);
    assert_eq!(stored.letters, file.letters);
    assert_eq!(stored.agent_steps, file.agent_steps);
    assert!(matches!(
        RefereeStore::replace(s, &file, 0).await,
        Err(CoreError::StaleRevision { .. })
    ));
    db.drop().await;
}

#[tokio::test]
async fn referee_job_deferral_preserves_attempts_and_lease_fence() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    ReviewQueue::enqueue(s, &"paper".into(), JobKind::Referee)
        .await
        .unwrap();
    let first = JobLease::claim(s).await.unwrap().unwrap();
    assert_eq!(first.attempts, 1);
    JobLease::defer(s, &first, std::time::Duration::ZERO)
        .await
        .unwrap();
    let document = s
        .raw(REVIEW_JOBS)
        .find_one(doc! { "kind": "referee" })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(document.get_i32("attempts").unwrap(), 0);
    assert!(!document.contains_key("lease"));
    assert!(!document.contains_key("lease_until"));
    assert!(document.get("last_error").unwrap().as_null().is_some());
    let second = JobLease::claim(s).await.unwrap().unwrap();
    assert_eq!(second.attempts, 1);
    assert_ne!(second.lease, first.lease);
    JobLease::defer(s, &first, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(JobLease::claim(s).await.unwrap().is_none());
    JobLease::defer(s, &second, std::time::Duration::from_secs(60))
        .await
        .unwrap();
    assert!(JobLease::claim(s).await.unwrap().is_none());
    db.drop().await;
}

#[tokio::test]
async fn oversized_review_history_migrates_without_losing_revision_fences_or_sources() {
    let Some(database) = TestDb::open().await else {
        return;
    };
    let store = &database.store;
    let mut file = RefereeFile::new("large-history".into());
    store
        .raw(REFEREE_FILES)
        .insert_one(to_document(&file).unwrap())
        .await
        .unwrap();
    for step in 0..10 {
        file.agent_steps.insert(
            format!("step-{step}"),
            AgentStep {
                cma_attempts: vec![],
                version: 1,
                claims_revision: 1,
                progress: serde_json::json!({"answer": "x".repeat(1_850_000), "deleted": true}),
            },
        );
    }
    assert!(serde_json::to_vec(&file).unwrap().len() > 16 * 1024 * 1024);
    RefereeStore::replace(store, &file, 0).await.unwrap();
    let stored = RefereeStore::get(store, &file.id).await.unwrap().unwrap();
    assert_eq!(stored.agent_steps, file.agent_steps);
    assert_eq!(stored.revision, 1);
    let header = store
        .raw(REFEREE_FILES)
        .find_one(doc! { "id": file.id.as_str() })
        .await
        .unwrap()
        .unwrap();
    assert!(mongodb::bson::to_vec(&header).unwrap().len() < 4096);
    assert_eq!(header.get_str("storage_format").unwrap(), "gridfs-json-v1");
    let files_before = store
        .raw("papers.files")
        .count_documents(doc! {})
        .await
        .unwrap();
    assert!(matches!(
        RefereeStore::replace(store, &file, 0).await,
        Err(CoreError::StaleRevision { .. })
    ));
    assert_eq!(
        store
            .raw("papers.files")
            .count_documents(doc! {})
            .await
            .unwrap(),
        files_before
    );
    let audit = store.storage_audit().await.unwrap();
    assert!(audit.missing_files.is_empty());
    assert!(audit.orphan_files.is_empty());
    database.drop().await;
}

#[tokio::test]
async fn offline_collection_refuses_unreadable_review_snapshots_before_deleting_files() {
    let Some(database) = TestDb::open().await else {
        return;
    };
    let store = &database.store;
    let mut file = RefereeFile::new("corrupt-history".into());
    file.agent_steps.insert(
        "audit".into(),
        AgentStep {
            cma_attempts: vec![],
            version: 1,
            claims_revision: 1,
            progress: serde_json::json!({"answer": "x".repeat(600_000)}),
        },
    );
    RefereeStore::insert(store, &file).await.unwrap();
    let original = store
        .raw(REFEREE_FILES)
        .find_one(doc! { "id": file.id.as_str() })
        .await
        .unwrap()
        .unwrap();
    let orphan = BlobStore::put(store, b"recovery material", "text/plain")
        .await
        .unwrap();
    store.raw("papers.files").update_one(doc! { "_id": &orphan.id },
        doc! { "$set": { "uploadDate": mongodb::bson::DateTime::from_chrono(Utc::now() - chrono::Duration::days(8)) } }).await.unwrap();

    for mutation in [
        doc! { "snapshot.sha256": "0".repeat(64) },
        doc! { "snapshot.id": "missing-snapshot" },
        doc! { "revision": 99_i64 },
        doc! { "storage_format": 1_i32 },
        doc! { "storage_format": "unknown" },
    ] {
        store
            .raw(REFEREE_FILES)
            .update_one(doc! { "id": file.id.as_str() }, doc! { "$set": mutation })
            .await
            .unwrap();
        assert!(store.storage_audit().await.is_err());
        assert!(store.collect_orphan_files_offline().await.is_err());
        assert_eq!(
            BlobStore::get(store, &orphan.id).await.unwrap().unwrap(),
            b"recovery material"
        );
        store
            .raw(REFEREE_FILES)
            .replace_one(doc! { "id": file.id.as_str() }, original.clone())
            .await
            .unwrap();
    }
    assert_eq!(
        store.storage_audit().await.unwrap().orphan_files,
        vec![orphan.id.clone()]
    );
    store.collect_orphan_files_offline().await.unwrap();
    assert!(BlobStore::get(store, &orphan.id).await.unwrap().is_none());
    assert_eq!(
        RefereeStore::get(store, &file.id).await.unwrap().unwrap(),
        file
    );
    database.drop().await;
}

#[tokio::test]
async fn offline_collection_preserves_references_recent_uploads_and_live_jobs() {
    let Some(database) = TestDb::open().await else {
        return;
    };
    let store = &database.store;
    let kept = BlobStore::put(store, b"saved", "text/plain").await.unwrap();
    let orphan = BlobStore::put(store, b"orphan", "text/plain")
        .await
        .unwrap();
    let recent = BlobStore::put(store, b"recent", "text/plain")
        .await
        .unwrap();
    let old_chunk = mongodb::bson::oid::ObjectId::from_parts(
        (Utc::now() - chrono::Duration::days(8)).timestamp() as u32,
        [0; 5],
        [0; 3],
    );
    for (id, file) in [
        (old_chunk, "interrupted-old"),
        (mongodb::bson::oid::ObjectId::new(), "interrupted-new"),
    ] {
        store.raw("papers.chunks").insert_one(doc! { "_id": id, "files_id": file, "n": 0,
            "data": mongodb::bson::Binary { subtype: mongodb::bson::spec::BinarySubtype::Generic, bytes: vec![1] } })
            .await.unwrap();
    }
    store
        .raw(SUBMISSIONS)
        .insert_one(doc! { "id": "fixture", "source": to_document(&kept).unwrap() })
        .await
        .unwrap();
    store.raw("papers.files").update_many(doc! { "_id": { "$in": [&kept.id, &orphan.id] } },
        doc! { "$set": { "uploadDate": mongodb::bson::DateTime::from_chrono(Utc::now() - chrono::Duration::days(8)) } }).await.unwrap();
    let audit = store.storage_audit().await.unwrap();
    assert_eq!(audit.orphan_files, vec![orphan.id.clone()]);
    assert_eq!(audit.orphan_chunk_files, vec!["interrupted-old"]);
    assert_eq!(audit.orphan_chunks, 1);
    ReviewQueue::enqueue(store, &"fixture".into(), JobKind::Compile)
        .await
        .unwrap();
    let job = JobLease::claim(store).await.unwrap().unwrap();
    assert!(store.collect_orphan_files_offline().await.is_err());
    JobLease::complete(store, &job).await.unwrap();
    store.collect_orphan_files_offline().await.unwrap();
    assert!(
        store
            .raw("papers.chunks")
            .find_one(doc! { "files_id": "interrupted-old" })
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .raw("papers.chunks")
            .find_one(doc! { "files_id": "interrupted-new" })
            .await
            .unwrap()
            .is_some()
    );
    assert!(BlobStore::get(store, &orphan.id).await.unwrap().is_none());
    assert_eq!(
        BlobStore::get(store, &kept.id).await.unwrap().unwrap(),
        b"saved"
    );
    assert_eq!(
        BlobStore::get(store, &recent.id).await.unwrap().unwrap(),
        b"recent"
    );
    database.drop().await;
}

#[tokio::test]
async fn compile_and_verification_jobs_are_available_while_reviews_are_leased() {
    let Some(database) = TestDb::open().await else {
        return;
    };
    let store = &database.store;
    for (paper, kind) in [
        ("review", JobKind::Referee),
        ("compile", JobKind::Compile),
        ("verify", JobKind::VerifyAttempt),
        ("research-verify", JobKind::VerifyResearch),
    ] {
        ReviewQueue::enqueue(store, &paper.into(), kind)
            .await
            .unwrap();
    }
    let review = store
        .claim_in(Some(JobLane::Review))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(review.kind, JobKind::Referee);
    assert!(
        store
            .claim_in(Some(JobLane::Review))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store
            .claim_in(Some(JobLane::Compile))
            .await
            .unwrap()
            .unwrap()
            .kind,
        JobKind::Compile
    );
    assert_eq!(
        store
            .claim_in(Some(JobLane::Verify))
            .await
            .unwrap()
            .unwrap()
            .kind,
        JobKind::VerifyAttempt
    );
    assert_eq!(
        store
            .claim_in(Some(JobLane::Verify))
            .await
            .unwrap()
            .unwrap()
            .kind,
        JobKind::VerifyResearch
    );
    database.drop().await;
}

#[tokio::test]
async fn job_renewal_extends_live_lease_and_never_revives_a_stale_token() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    ReviewQueue::enqueue(s, &"paper".into(), JobKind::Referee)
        .await
        .unwrap();
    let first = JobLease::claim(s).await.unwrap().unwrap();
    s.raw(REVIEW_JOBS).update_one(
        doc! { "lease": &first.lease },
        doc! { "$set": { "lease_until": mongodb::bson::DateTime::from_chrono(Utc::now() + chrono::Duration::minutes(2)) } },
    ).await.unwrap();
    let before = Utc::now();
    assert!(JobLease::renew(s, &first).await.unwrap());
    let renewed = s
        .raw(REVIEW_JOBS)
        .find_one(doc! { "lease": &first.lease })
        .await
        .unwrap()
        .unwrap();
    let until = renewed.get_datetime("lease_until").unwrap().to_chrono();
    assert!(until >= before + chrono::Duration::minutes(30) - chrono::Duration::milliseconds(1));
    assert_eq!(renewed.get_i32("attempts").unwrap(), 1);
    assert!(JobLease::claim(s).await.unwrap().is_none());

    s.raw(REVIEW_JOBS).update_one(
        doc! { "lease": &first.lease },
        doc! { "$set": { "lease_until": mongodb::bson::DateTime::from_chrono(Utc::now() - chrono::Duration::seconds(1)) } },
    ).await.unwrap();
    assert!(!JobLease::renew(s, &first).await.unwrap());
    JobLease::complete(s, &first).await.unwrap();
    let second = JobLease::claim(s)
        .await
        .unwrap()
        .expect("expired lease is reclaimable");
    assert_eq!(second.attempts, 2);
    assert_ne!(second.lease, first.lease);
    assert!(!JobLease::renew(s, &first).await.unwrap());
    JobLease::complete(s, &first).await.unwrap();
    JobLease::defer(s, &first, std::time::Duration::ZERO)
        .await
        .unwrap();
    JobLease::retry(s, &first, "stale worker").await.unwrap();
    assert!(JobLease::renew(s, &second).await.unwrap());
    let current = s
        .raw(REVIEW_JOBS)
        .find_one(doc! { "lease": &second.lease })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.get_i32("attempts").unwrap(), 2);
    assert_eq!(current.get_str("state").unwrap(), "leased");
    assert!(current.get("last_error").unwrap().as_null().is_some());
    JobLease::complete(s, &second).await.unwrap();
    assert!(!JobLease::renew(s, &second).await.unwrap());
    db.drop().await;
}

#[tokio::test]
async fn solving_aggregates_and_agent_names_are_unique_and_revision_fenced() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let store = &db.store;
    let agent = Entrant {
        id: "agent:one".into(),
        name: "Solver".into(),
        kind: EntrantKind::Agent,
        owner: Some(Owner {
            id: "owner".into(),
            name: "Owner".into(),
        }),
        retired: false,
        revision: 0,
    };
    let duplicate = Entrant {
        id: "agent:two".into(),
        ..agent.clone()
    };
    let (one, two) = tokio::join!(store.insert_agent(&agent), store.insert_agent(&duplicate));
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let mut saved = store.agents().await.unwrap().pop().unwrap();
    saved.name = "Renamed".into();
    saved.revision = 1;
    store.replace_agent(&saved, 0).await.unwrap();
    assert!(matches!(
        store.replace_agent(&saved, 0).await,
        Err(CoreError::StaleRevision { .. })
    ));
    assert_eq!(store.agents().await.unwrap()[0].name, "Renamed");
    let file = SolveFile {
        review_contract: 0,
        agent_attempted: Default::default(),
        id: "WP-2026-0001:C1:v1:c1".into(),
        record: "WP-2026-0001".into(),
        submission: "source".into(),
        claim: "C1".into(),
        version: 1,
        claims_revision: 1,
        revision: 0,
        admitted: true,
        failure: None,
        review_round: 1,
        task: None,
        report: None,
        audit: None,
        attempts: vec![],
        dependencies: vec![],
        downstream: 0,
    };
    SolvingStore::insert(store, &file).await.unwrap();
    assert!(SolvingStore::insert(store, &file).await.is_err());
    let mut a = file.clone();
    a.revision = 1;
    a.downstream = 1;
    let mut b = file.clone();
    b.revision = 1;
    b.downstream = 2;
    let (one, two) = tokio::join!(
        SolvingStore::replace(store, &a, 0),
        SolvingStore::replace(store, &b, 0)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let stored = SolvingStore::get(store, &file.id).await.unwrap().unwrap();
    assert_eq!(stored.revision, 1);
    assert!([1, 2].contains(&stored.downstream));
    assert_eq!(SolvingStore::all(store).await.unwrap().len(), 1);
    db.drop().await;
}

#[tokio::test]
async fn cma_binding_mongo_has_unique_person_and_revision_cas() {
    use mathesis_core::{model::CmaBinding, ports::CmaBindingStore};
    let Some(db) = TestDb::open().await else {
        return;
    };
    let mut binding = CmaBinding::disconnected("author".into(), chrono::Utc::now());
    binding.revision = 1;
    CmaBindingStore::replace(&db.store, &binding, 0)
        .await
        .unwrap();
    assert!(
        CmaBindingStore::replace(&db.store, &binding, 0)
            .await
            .is_err()
    );
    let old = binding.clone();
    binding.revision = 2;
    binding.generation = 1;
    CmaBindingStore::replace(&db.store, &binding, 1)
        .await
        .unwrap();
    assert!(CmaBindingStore::replace(&db.store, &old, 1).await.is_err());
    assert_eq!(
        CmaBindingStore::get(&db.store, &"author".into())
            .await
            .unwrap()
            .unwrap(),
        binding
    );
    db.drop().await;
}

#[tokio::test]
async fn error_reports_round_trip_with_atomic_per_entrant_revision() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let store = &db.store;
    let mut file = ErrorReportFile {
        id: "reporter".into(),
        revision: 0,
        reports: vec![],
    };
    store.insert_report_file(&file).await.unwrap();
    assert!(store.insert_report_file(&file).await.is_err());
    let now = chrono::Utc::now();
    file.reports.push(ErrorReport {
        id: "report-one".into(),
        record: "WP-2026-0001".into(),
        submission: "source".into(),
        version: 1,
        claims_revision: 1,
        reporter: Entrant {
            id: "reporter".into(),
            name: "Reporter".into(),
            kind: EntrantKind::Person,
            owner: None,
            retired: false,
            revision: 0,
        },
        owner: "reporter".into(),
        subject: ErrorSubject::Statement { claim: "C1".into() },
        text: "At zero the two sides differ.".into(),
        solution: None,
        attempt: None,
        receipt: None,
        resolution: Some(ErrorResolution {
            verdict: ErrorVerdict::Confirmed,
            reason: "A counterexample.".into(),
            evidence: "Left=0, right=1.".into(),
            at: now,
        }),
        effects_applied: false,
        created_at: now,
    });
    file.revision = 1;
    let (one, two) = tokio::join!(
        store.replace_report_file(&file, 0),
        store.replace_report_file(&file, 0)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let stored = store.report_files().await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].revision, 1);
    assert_eq!(stored[0].reports[0].text, file.reports[0].text);
    assert_eq!(
        stored[0].reports[0].resolution.as_ref().unwrap().verdict,
        ErrorVerdict::Confirmed
    );
    db.drop().await;
}

#[tokio::test]
async fn archive_text_search_and_formalization_receipts_survive_reconnect() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let mut s = submission("search-sub", "author", SubmissionStatus::Draft);
    s.analysis_visibility = Visibility::Public;
    s.title = "Prime gaps".into();
    s.abstract_text = "spectral witnesses".into();
    s.msc = vec!["11B83".into()];
    s.authors = vec![Author {
        name: "A. Author".into(),
        person: None,
        orcid: None,
        affiliation: None,
        google_scholar: None,
        website: None,
    }];
    let record = Record {
        id: "WP-2026-0001".into(),
        kind: SubmissionKind::Paper,
        title: s.title.clone(),
        authors: s.authors.clone(),
        submission: s.id.clone(),
        publication: Some(Box::new(s)),
        basis: mathesis_core::policy::AdmissionBasis::EscapeWitness,
        accepted_by: "editor".into(),
        accepted_at: Utc::now(),
    };
    SubmissionStore::insert(&db.store, record.publication.as_ref().unwrap())
        .await
        .unwrap();
    RecordStore::insert(&db.store, &record).await.unwrap();
    let q = PaperSearch {
        q: Some("spectral".into()),
        msc: Some("11".into()),
        author: Some("A. Author".into()),
        kind: Some(SubmissionKind::Paper),
    };
    assert_eq!(
        RecordStore::search(&db.store, &q, 1, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let mut literal = q.clone();
    literal.author = Some(".*".into());
    assert!(
        RecordStore::search(&db.store, &literal, 1, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let exact = PaperSearch {
        q: Some("WP-2026-0001".into()),
        ..Default::default()
    };
    assert_eq!(
        RecordStore::search(&db.store, &exact, 1, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        RecordStore::search(&db.store, &exact, 1, Some("WP-2026-0001".into()))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    // Revoking visibility must stop indexed discovery without changing the immutable record.
    let mut current = SubmissionStore::get(&db.store, &record.submission)
        .await
        .unwrap()
        .unwrap();
    current.analysis_visibility = Visibility::Private;
    SubmissionStore::replace(&db.store, &current, current.revision)
        .await
        .unwrap();
    assert!(
        RecordStore::search(&db.store, &q, 1, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let blob = BlobStore::put(&db.store, b"import Target", "text/plain")
        .await
        .unwrap();
    let mut deposit = LeanDeposit {
        project_files: Default::default(),
        modules: std::collections::BTreeMap::from([("Helpers".into(), blob.clone())]),
        formalizers: vec!["Formalizer".into()],
        source_revision: Some("a".repeat(40)),
        id: "lean-test".into(),
        owner: "author".into(),
        title: "A formalization".into(),
        authors: vec!["Original author".into()],
        statement: "True".into(),
        source_reference: Source {
            kind: SourceKind::Url,
            locator: "https://example.org/source".into(),
            year: None,
        },
        toolchain: "test".into(),
        license: "Apache-2.0".into(),
        source: blob,
        submitted_at: Utc::now(),
        attempts: vec![],
        revision: 0,
    };
    LeanDepositStore::insert(&db.store, &deposit).await.unwrap();
    assert!(LeanDepositStore::insert(&db.store, &deposit).await.is_err());
    deposit.revision = 1;
    deposit.attempts.push("attempt".into());
    LeanDepositStore::replace(&db.store, &deposit, 0)
        .await
        .unwrap();
    assert!(matches!(
        LeanDepositStore::replace(&db.store, &deposit, 0).await,
        Err(CoreError::StaleRevision { .. })
    ));
    let uri = std::env::var("WISHPOOL_TEST_MONGODB_URI").unwrap();
    let reopened = MongoStore::connect(&uri, &db.name).await.unwrap();
    let saved = LeanDepositStore::get(&reopened, "lean-test")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.authors, vec!["Original author"]);
    assert_eq!(saved.attempts, vec!["attempt"]);
    assert_eq!(
        BlobStore::get(&reopened, &saved.source.id)
            .await
            .unwrap()
            .unwrap(),
        b"import Target"
    );
    assert!(
        LeanDepositStore::list(&reopened, &"other".into(), 25, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    reopened.shutdown().await;
    db.drop().await;
}

#[tokio::test]
async fn research_attempt_sources_survive_offline_collection_from_gridfs_history() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let store = &db.store;
    let solution = BlobStore::put(store, b"private research solution", "text/plain")
        .await
        .unwrap();
    let module = BlobStore::put(store, b"private research module", "text/plain")
        .await
        .unwrap();
    store.raw("papers.files").update_many(doc! {"_id":{"$in":[&solution.id,&module.id]}},
        doc! {"$set":{"uploadDate":mongodb::bson::DateTime::from_chrono(Utc::now()-chrono::Duration::days(10))}}).await.unwrap();
    let mut file = RefereeFile::new("research-history".into());
    file.research_obligations.push(ResearchObligation {
        id: "research".into(),
        candidate: ResearchCandidate {
            digest: "fixture".into(),
            subject: CoverageSubject {
                submission: file.id.clone(),
                version: 1,
                claims_revision: 1,
                round: 1,
                claim: "C1".into(),
                source_digest: "source".into(),
                claim_digest: "claim".into(),
            },
            gap: ResearchGap::Bridge { index: 0 },
            unresolved_text: "bridge".into(),
            original_statement: "claim".into(),
            receipt_ids: vec![],
            binding: KnowledgeBinding {
                premises: vec![],
                relation: BindingRelation::Implies,
                parameters: vec![],
                hypotheses: vec![],
                argument: "fixture".into(),
                unresolved: vec![],
                source_lineage: vec![],
            },
        },
        goal: ResearchGoal {
            definitions: vec![],
            hypotheses: vec![],
            quantifiers: "all".into(),
            conclusion: "fixture".into(),
            correspondence: "fixture".into(),
        },
        budget: ResearchBudget {
            max_attempts: 1,
            expires_at: Utc::now(),
        },
        consent: RESEARCH_CONSENT.into(),
        authorized_by: "author".into(),
        authorized_at: Utc::now(),
        events: vec![ResearchEvent::AttemptAdmitted {
            attempt: Box::new(Attempt {
                id: "attempt".into(),
                owner: "author".into(),
                entrant: Entrant {
                    id: "author".into(),
                    name: "Author".into(),
                    kind: EntrantKind::Person,
                    owner: None,
                    retired: false,
                    revision: 0,
                },
                target_digest: "target".into(),
                solution_digest: "solution".into(),
                solution: solution.clone(),
                modules: std::collections::BTreeMap::from([("Lemma".into(), module.clone())]),
                note: "Private".into(),
                submitted_at: Utc::now(),
                receipt: None,
                invalidated: None,
                attribution: None,
            }),
        }],
    });
    // Force the normal checksummed GridFS history representation.
    file.agent_steps.insert(
        "large".into(),
        AgentStep {
            cma_attempts: vec![],
            version: 1,
            claims_revision: 1,
            progress: serde_json::json!({"answer":"x".repeat(600_000)}),
        },
    );
    RefereeStore::insert(store, &file).await.unwrap();
    let read = RefereeStore::get(store, &file.id).await.unwrap().unwrap();
    assert_eq!(read.research_obligations, file.research_obligations);
    let audit = store.storage_audit().await.unwrap();
    assert!(!audit.orphan_files.contains(&solution.id));
    assert!(!audit.orphan_files.contains(&module.id));
    store.collect_orphan_files_offline().await.unwrap();
    assert!(BlobStore::get(store, &solution.id).await.unwrap().is_some());
    assert!(BlobStore::get(store, &module.id).await.unwrap().is_some());
    db.drop().await;
}

#[tokio::test]
async fn binding_global_admission_is_atomic_idempotent_and_expires() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let person = PersonId::from("binding-person");
    let now = Utc::now();
    let outcomes = futures::future::join_all((0..8).map(|n| {
        let key = format!("summary-{n}:request");
        let person = person.clone();
        async move { RefereeStore::admit_binding_submission(s, &person, &key, now).await }
    }))
    .await;
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 5);
    let first = outcomes.iter().position(|r| r.is_ok()).unwrap();
    RefereeStore::admit_binding_submission(s, &person, &format!("summary-{first}:request"), now)
        .await
        .unwrap();
    assert!(
        RefereeStore::admit_binding_submission(s, &person, "different-summary", now)
            .await
            .is_err()
    );
    RefereeStore::admit_binding_submission(
        s,
        &person,
        "different-summary",
        now + chrono::Duration::hours(25),
    )
    .await
    .unwrap();
    db.drop().await;
}
#[tokio::test]
async fn binding_directory_projects_small_metadata_from_checksumming_gridfs_history() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let mut file = RefereeFile::new("binding-history".into());
    file.binding_boards.push(BindingBoard {
        id: "board".into(),
        target: BindingTarget {
            subject: CoverageSubject {
                submission: file.id.clone(),
                version: 1,
                claims_revision: 1,
                round: 1,
                claim: "C1".into(),
                source_digest: "fixture".into(),
                claim_digest: "fixture".into(),
            },
            statement: "Authorized fixture".into(),
            sources: vec![],
        },
        context: "context".into(),
        license: BINDING_LICENSE.into(),
        consent: BINDING_SHARING_CONSENT.into(),
        authorized_by: "author".into(),
        authorized_at: Utc::now(),
        receipt_ids: vec![],
        events: vec![BindingEvent::Revoked {
            by: "author".into(),
            reason: "x".repeat(600_000),
            at: Utc::now(),
        }],
    });
    RefereeStore::insert(s, &file).await.unwrap();
    let doc = s
        .raw(REFEREE_FILES)
        .find_one(doc! {"id":file.id.as_str()})
        .await
        .unwrap()
        .unwrap();
    assert_eq!(doc.get_str("storage_format"), Ok("gridfs-json-v1"));
    assert_eq!(doc.get_bool("binding_directory"), Ok(true));
    assert!(doc.get("binding_boards").is_none());
    let page = RefereeStore::binding_sources(s, 1, None).await.unwrap();
    assert_eq!(page.items, vec![file.id.clone()]);
    assert_eq!(
        RefereeStore::get(s, &file.id)
            .await
            .unwrap()
            .unwrap()
            .binding_boards,
        file.binding_boards
    );
    let snapshot: BlobRef =
        super::from_document(doc.get_document("snapshot").unwrap().clone()).unwrap();
    BlobStore::discard(s, &snapshot.id).await.unwrap();
    assert!(RefereeStore::get(s, &file.id).await.is_err());
    db.drop().await;
}

#[tokio::test]
async fn feedback_history_retains_payload_and_observation_blobs_in_offline_collection() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let s = &db.store;
    let payload = BlobStore::put(s, b"exact authorized payload", "application/json")
        .await
        .unwrap();
    let evidence = BlobStore::put(s, b"exact observation evidence", "application/json")
        .await
        .unwrap();
    let orphan = BlobStore::put(s, b"unreferenced retry blob", "application/json")
        .await
        .unwrap();
    let mut file = RefereeFile::new("feedback-history".into());
    let now = Utc::now();
    file.feedback_proposals.push(FeedbackProposal {
        id: "proposal".into(),
        digest: format!("sha256:{}", payload.sha256),
        producer: "fixture".into(),
        source: FeedbackSource {
            submission: file.id.clone(),
            record: "WP-2026-0001".into(),
            version: 1,
            claims_revision: 1,
            source_digest: "fixture".into(),
        },
        kind: FeedbackKind::OpenProblem { claim: "C1".into() },
        payload: payload.clone(),
        required_signers: vec!["author".into()],
        prepared_by: "author".into(),
        prepared_at: now,
        preparation_consent: FEEDBACK_PREPARATION_CONSENT.into(),
        events: vec![FeedbackEvent::Observed {
            observation: Box::new(FeedbackObservation {
                request: "fixture".into(),
                pull_request: 9,
                upstream_repository: "the-omega-institute/trureturing".into(),
                upstream_base: String::new(),
                reference_state: "unavailable".into(),
                merge_commit: None,
                merged_at: None,
                consumed_source_commit: String::new(),
                consumed_source_tree: String::new(),
                snapshot: String::new(),
                index_revision: String::new(),
                merge_in_consumed_source: false,
                declarations: vec![],
                evidence: evidence.clone(),
                observed_at: now,
                limits: vec!["x".repeat(600_000)],
            }),
        }],
    });
    RefereeStore::insert(s, &file).await.unwrap();
    let doc = s
        .raw(REFEREE_FILES)
        .find_one(doc! {"id":file.id.as_str()})
        .await
        .unwrap()
        .unwrap();
    assert_eq!(doc.get_str("storage_format"), Ok("gridfs-json-v1"));
    assert_eq!(doc.get_bool("feedback_directory"), Ok(true));
    assert!(doc.get("feedback_proposals").is_none());
    let loaded = RefereeStore::get(s, &file.id).await.unwrap().unwrap();
    assert_eq!(loaded.feedback_proposals, file.feedback_proposals);
    assert_eq!(
        RefereeStore::feedback_sources(s, 1, None)
            .await
            .unwrap()
            .items,
        vec![file.id]
    );
    s.raw("papers.files").update_many(doc!{},doc!{"$set":{"uploadDate":mongodb::bson::DateTime::from_chrono(now-chrono::Duration::days(10))}}).await.unwrap();
    let audit = s.storage_audit().await.unwrap();
    assert!(audit.missing_files.is_empty());
    assert!(audit.orphan_files.contains(&orphan.id));
    assert!(!audit.orphan_files.contains(&payload.id));
    assert!(!audit.orphan_files.contains(&evidence.id));
    db.drop().await;
}

#[tokio::test]
async fn review_summary_index_backfills_legacy_histories_and_fences_new_rounds() {
    let Some(database) = TestDb::open().await else {
        return;
    };
    let store = &database.store;
    for large in [false, true] {
        let mut file = RefereeFile::new(
            if large {
                "summary-large"
            } else {
                "summary-inline"
            }
            .into(),
        );
        file.rounds.push(serde_json::from_value(serde_json::json!({
            "number":1,"version":1,"claims_revision":2,"started_at":Utc::now(),
            "referee":{"state":{"state":"failed","reason":"Saved earlier failure","at":Utc::now()}},
            "advice":{"state":{"state":"skipped","reason":"No report"}},
            "letter":{"state":{"state":"skipped","reason":"No report"}}
        })).unwrap());
        if large {
            file.agent_steps.insert(
                "large-step".into(),
                AgentStep {
                    cma_attempts: vec![],
                    version: 1,
                    claims_revision: 2,
                    progress: serde_json::json!({"answer":"x".repeat(600_000)}),
                },
            );
        }
        RefereeStore::insert(store, &file).await.unwrap();
        let before = store
            .raw(REFEREE_FILES)
            .find_one(doc! {"id":file.id.as_str()})
            .await
            .unwrap()
            .unwrap();
        assert!(before.contains_key("review_status_v1"));
        // Legacy read derives only the new index, retaining revision and exact history.
        store
            .raw(REFEREE_FILES)
            .update_one(
                doc! {"id":file.id.as_str()},
                doc! {"$unset":{"review_status_v1":""}},
            )
            .await
            .unwrap();
        let expected = file.summary_for(Some(1), 2);
        assert!(expected.interrupted);
        assert_eq!(
            RefereeStore::summary(store, &file.id, Some(1), 2)
                .await
                .unwrap(),
            expected
        );
        assert_eq!(
            RefereeStore::get(store, &file.id).await.unwrap(),
            Some(file.clone())
        );
        let after = store
            .raw(REFEREE_FILES)
            .find_one(doc! {"id":file.id.as_str()})
            .await
            .unwrap()
            .unwrap();
        assert_eq!(before, after);
        let old_index = after.get_document("review_status_v1").unwrap().clone();
        let mut next = file.rounds[0].clone();
        next.number = 2;
        next.referee = Step::pending();
        file.rounds.push(next);
        RefereeStore::replace(store, &file, 0).await.unwrap();
        file.revision = 1;
        let current = file.summary_for(Some(1), 2);
        assert!(current.in_progress && !current.interrupted);
        assert_eq!(
            RefereeStore::summary(store, &file.id, Some(1), 2)
                .await
                .unwrap(),
            current
        );
        assert_eq!(
            RefereeStore::summary(store, &file.id, Some(2), 2)
                .await
                .unwrap()
                .round,
            None
        );
        // An index from an older revision cannot hide the newly admitted round.
        store
            .raw(REFEREE_FILES)
            .update_one(
                doc! {"id":file.id.as_str()},
                doc! {"$set":{"review_status_v1":old_index}},
            )
            .await
            .unwrap();
        assert_eq!(
            RefereeStore::summary(store, &file.id, Some(1), 2)
                .await
                .unwrap(),
            current
        );
        let header = store
            .raw(REFEREE_FILES)
            .find_one(doc! {"id":file.id.as_str()})
            .await
            .unwrap()
            .unwrap();
        // A new store reads only the index, even when the report body is unavailable.
        // The full report still refuses missing/corrupted evidence.
        if large {
            let snapshot: BlobRef =
                from_document(header.get_document("snapshot").unwrap().clone()).unwrap();
            BlobStore::discard(store, &snapshot.id).await.unwrap();
        } else {
            store
                .raw(REFEREE_FILES)
                .update_one(
                    doc! {"id":file.id.as_str()},
                    doc! {"$set":{"rounds":"invalid"}},
                )
                .await
                .unwrap();
        }
        let reopened = MongoStore::connect(
            &std::env::var("WISHPOOL_TEST_MONGODB_URI").unwrap(),
            &database.name,
        )
        .await
        .unwrap();
        assert_eq!(
            RefereeStore::summary(&reopened, &file.id, Some(1), 2)
                .await
                .unwrap(),
            current
        );
        assert!(RefereeStore::get(&reopened, &file.id).await.is_err());
    }
    database.drop().await;
}

#[tokio::test]
async fn manuscript_delivery_identity_is_unique_persistent_and_legacy_safe() {
    let Some(db) = TestDb::open().await else {
        return;
    };
    let mut first = submission("manuscript-one", "author", SubmissionStatus::Draft);
    first.manuscript = Some(ManuscriptIdentity {
        key: "stable-owner-work-purpose".into(),
        producer: "paper".into(),
        work_id: "note-a".into(),
    });
    let mut duplicate = first.clone();
    duplicate.id = "manuscript-two".into();
    let (a, b) = tokio::join!(
        SubmissionStore::insert(&db.store, &first),
        SubmissionStore::insert(&db.store, &duplicate)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let winner = SubmissionStore::manuscript(&db.store, "stable-owner-work-purpose")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(winner.id, if a.is_ok() { first.id } else { duplicate.id });
    let restarted = MongoStore {
        client: db.store.client.clone(),
        db: db.store.db.clone(),
    };
    assert_eq!(
        SubmissionStore::manuscript(&restarted, "stable-owner-work-purpose")
            .await
            .unwrap()
            .unwrap(),
        winner
    );
    for id in ["legacy-a", "legacy-b"] {
        SubmissionStore::insert(
            &db.store,
            &submission(id, "author", SubmissionStatus::Draft),
        )
        .await
        .unwrap();
    }
    assert_eq!(
        SubmissionStore::active_count(&db.store, &"author".into())
            .await
            .unwrap(),
        3
    );
    db.drop().await;
}
