use super::*;
use crate::ports::{CollectedPaperStore, CollectedProblemStore, RecordStore};
use sha2::{Digest, Sha256};

const NAME: &str = "Hanna's alternating-sum formula (5) in OEIS A391810";

fn envelope(
    producer: &str,
    slug: &str,
    version: u32,
    name: &str,
    locator: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema":"mathesis.collected-problem.v1", "producer":producer, "problem_id":slug,
        "version":version, "previous_version":(version > 1).then(|| version - 1),
        "authorize_publication":true,
        "problem":{
            "name":name,
            "statement":"For every $n \\ge 1$, $\\sum_{k=0}^{n} (-1)^k a(k) = b(n)$.",
            "background":"The formula was conjectured from the first 200 terms.",
            "posers":[{"name":"Paul D. Hanna","profile_url":"https://oeis.org/wiki/User:Paul_D._Hanna"}],
            "source":{"kind":"oeis","locator":locator,"year":2025},
            "status":"open",
            "status_evidence":[
                {"url":format!("https://oeis.org/{locator}"),"checked_on":"2026-11-08",
                 "observed":"Formula (5) is marked as a conjecture; no proof is linked."},
                {"url":"https://mathoverflow.net/questions/000000","checked_on":"2026-11-10",
                 "observed":"The question has no accepted answer."}],
            "msc":["11B83"],
            "directions":["alternating-sums"],
            "source_license":"CC BY-SA 4.0 (OEIS End-User License Agreement)",
            "license":"CC-BY-SA-4.0"
        }
    })
}

fn delivery(value: serde_json::Value) -> CollectedProblemDelivery {
    serde_json::from_value(value).unwrap()
}

async fn world(operators: &[&str]) -> World {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader)),
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        20,
        operators.iter().map(|s| PersonId::from(*s)).collect(),
    );
    w
}

/// A pre-reviewed work naming one problem, as in the collection channel.
async fn collect_work(
    w: &World,
    operator: &Caller,
    producer: &str,
    work: &str,
    version: u32,
    problems: serde_json::Value,
) {
    let pdf = Upload {
        filename: "note.pdf".into(),
        bytes: format!("%PDF-{work}-{version}").into_bytes(),
    };
    let source = Upload {
        filename: "source.zip".into(),
        bytes: format!("PK\x03\x04{work}-{version}").into_bytes(),
    };
    let delivery = serde_json::from_value(serde_json::json!({
        "schema":"mathesis.pre-reviewed-delivery.v1", "producer":producer, "work_id":work,
        "version":version, "previous_version":(version > 1).then(|| version - 1),
        "authorize_publication":true,
        "paper":{"title":format!("Collected {work}"), "abstract":"A reviewed note.",
            "authors":[{"name":"Collected Author"}], "license":"CC-BY-4.0", "problems":problems},
        "review":{"verdict":"PASS", "pdf_sha256":format!("{:x}",Sha256::digest(&pdf.bytes)),
            "source_sha256":format!("{:x}",Sha256::digest(&source.bytes)),
            "record":{"private_evidence":"must not be exposed"}}
    }))
    .unwrap();
    w.app
        .collect_pre_reviewed(operator, delivery, pdf, source)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_trusted_operator_collects_a_curated_open_problem_as_a_lasting_public_entry() {
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    // The collection channel's rule: roles, including Administrator, do not open it.
    let admin = w.person("admin", &[]).await;
    let reader = w.person("reader", &[]).await;
    for caller in [&admin, &reader] {
        assert_eq!(
            w.app
                .collect_problem(
                    caller,
                    delivery(envelope("outreach", "hanna-formula-5", 1, NAME, "A391810")),
                )
                .await
                .unwrap_err(),
            CoreError::forbidden("trusted collection operator required")
        );
    }
    assert!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );

    let receipt = w
        .app
        .collect_problem(
            &curator,
            delivery(envelope("outreach", "hanna-formula-5", 1, NAME, "A391810")),
        )
        .await
        .unwrap();
    assert_eq!(receipt.schema, "mathesis.collected-problem-receipt.v1");
    assert_eq!(receipt.status, "collected");
    assert_eq!(
        (receipt.problem_id.as_str(), receipt.version),
        ("hanna-formula-5", 1)
    );
    assert_eq!(
        receipt.public_path,
        format!("/problems/{}", receipt.research_problem)
    );

    // Listed with no work naming it yet, under the curator's dated reading.
    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(page.items.len(), 1);
    let summary = &page.items[0];
    assert_eq!(summary.id, receipt.research_problem);
    assert_eq!(summary.title, NAME);
    assert_eq!(summary.status, ResearchProblemStatus::CuratedOpen);
    assert_eq!(
        summary.status_as_of,
        chrono::NaiveDate::from_ymd_opt(2026, 11, 10)
    );
    assert_eq!(summary.notes, 0);
    assert_eq!(summary.source_id, "A391810");
    assert_eq!(
        summary.source,
        Some(Source {
            kind: SourceKind::Oeis,
            locator: "A391810".into(),
            year: Some(2025),
        })
    );

    let detail = w.app.research_problem(&summary.id).await.unwrap();
    assert!(detail.notes.is_empty() && detail.collected_notes.is_empty());
    assert!(detail.descriptions.is_empty() && detail.next_before.is_none());
    let curated = detail.curated.as_ref().unwrap();
    assert_eq!(curated.name, NAME);
    assert!(curated.statement.contains("\\sum_{k=0}^{n}"));
    assert_eq!(
        curated.background.as_deref(),
        Some("The formula was conjectured from the first 200 terms.")
    );
    assert_eq!(curated.posers[0].name, "Paul D. Hanna");
    assert!(curated.posers[0].profile_url.is_some() && curated.posers_unknown.is_none());
    assert_eq!(curated.source.locator, "A391810");
    assert_eq!(curated.status, CuratedStatus::Open);
    assert_eq!(Some(curated.status_as_of), summary.status_as_of);
    assert_eq!(curated.status_evidence.len(), 2);
    assert_eq!(
        curated.status_evidence[1].observed,
        "The question has no accepted answer."
    );
    assert_eq!(
        (curated.msc.as_slice(), curated.directions.as_slice()),
        (
            &["11B83".to_owned()][..],
            &["alternating-sums".to_owned()][..]
        )
    );
    assert_eq!(curated.license, "CC-BY-SA-4.0");
    assert!(curated.source_license.starts_with("CC BY-SA 4.0"));
    assert_eq!(curated.version, 1);

    // The public projection names no operator, producer, slug or request digest.
    let stored = CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.published_on, curated.published_on);
    let json = serde_json::to_string(&(&detail, &page.items)).unwrap();
    for private in [
        "curator",
        "outreach",
        "hanna-formula-5",
        &stored.request_digest,
    ] {
        assert!(!json.contains(private), "leaked {private}");
    }
    // Wire names of the curated reading.
    let json: serde_json::Value = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["summary"]["status"], "curated_open");
    assert_eq!(json["summary"]["status_as_of"], "2026-11-10");
    assert_eq!(json["curated"]["status"], "open");
    assert_eq!(
        json["curated"]["status_evidence"][0]["checked_on"],
        "2026-11-08"
    );

    // Collection creates no record, review, target, proof job or credit.
    assert!(w.stores.queued().await.is_empty());
    assert!(
        crate::ports::SolvingStore::all(w.stores.as_ref())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        RecordStore::list(w.stores.as_ref(), 10, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn exact_retries_share_a_receipt_and_published_problem_versions_are_immutable() {
    let w = world(&["curator", "second-curator"]).await;
    let curator = w.person("curator", &[]).await;
    let second = w.person("second-curator", &[]).await;
    let first = || delivery(envelope("outreach", "hanna-formula-5", 1, NAME, "A391810"));
    let receipt = w.app.collect_problem(&curator, first()).await.unwrap();
    let stored = CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", Some(1))
        .await
        .unwrap()
        .unwrap();

    // The same bytes return the same receipt and leave the stored version untouched.
    let retry = w.app.collect_problem(&curator, first()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&receipt).unwrap(),
        serde_json::to_value(&retry).unwrap()
    );
    assert_eq!(
        stored,
        CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", None)
            .await
            .unwrap()
            .unwrap()
    );

    // A changed payload under a published version is refused.
    let mut changed = envelope("outreach", "hanna-formula-5", 1, NAME, "A391810");
    changed["problem"]["statement"] = "A different statement.".into();
    assert_eq!(
        w.app
            .collect_problem(&curator, delivery(changed))
            .await
            .unwrap_err(),
        CoreError::conflict("published versions are immutable; deliver a new version")
    );

    // A slug belongs to its operator and producer.
    let mut next = envelope("outreach", "hanna-formula-5", 2, NAME, "A391810");
    next["problem"]["status"] = "partially_resolved".into();
    next["problem"]["status_evidence"] = serde_json::json!([{
        "url":"https://arxiv.org/abs/2611.00001","checked_on":"2026-11-11",
        "observed":"The preprint proves the formula for even n only."}]);
    for (caller, producer) in [(&second, "outreach"), (&curator, "another-producer")] {
        let mut other = next.clone();
        other["producer"] = producer.into();
        assert_eq!(
            w.app
                .collect_problem(caller, delivery(other))
                .await
                .unwrap_err(),
            CoreError::conflict("problem identity belongs to a different collection operator")
        );
    }

    // A new version names the exact current predecessor.
    let mut skipped = envelope("outreach", "hanna-formula-5", 3, NAME, "A391810");
    skipped["previous_version"] = serde_json::Value::Null;
    assert!(matches!(
        w.app.collect_problem(&curator, delivery(skipped)).await,
        Err(CoreError::Conflict(_))
    ));
    let fresh = envelope(
        "outreach",
        "another-problem",
        2,
        "Another question",
        "A000001",
    );
    assert_eq!(
        w.app
            .collect_problem(&curator, delivery(fresh))
            .await
            .unwrap_err(),
        CoreError::conflict("first collected version cannot name a predecessor")
    );

    // Name and source are the entry's identity: a later version keeps both.
    for (name, locator) in [("A renamed formula", "A391810"), (NAME, "A391811")] {
        assert!(matches!(
            w.app
                .collect_problem(
                    &curator,
                    delivery(envelope("outreach", "hanna-formula-5", 2, name, locator)),
                )
                .await,
            Err(CoreError::Conflict(_))
        ));
    }
    // One named source question has one slug.
    assert!(matches!(
        w.app
            .collect_problem(
                &curator,
                delivery(envelope("outreach", "hanna-duplicate", 1, NAME, "A391810")),
            )
            .await,
        Err(CoreError::Conflict(_))
    ));

    let second_receipt = w
        .app
        .collect_problem(&curator, delivery(next))
        .await
        .unwrap();
    assert_eq!(second_receipt.research_problem, receipt.research_problem);
    assert_eq!(second_receipt.version, 2);
    let detail = w
        .app
        .research_problem(&receipt.research_problem)
        .await
        .unwrap();
    assert_eq!(
        detail.summary.status,
        ResearchProblemStatus::CuratedPartiallyResolved
    );
    assert_eq!(
        detail.summary.status_as_of,
        chrono::NaiveDate::from_ymd_opt(2026, 11, 11)
    );
    let curated = detail.curated.unwrap();
    assert_eq!(
        (curated.version, curated.status),
        (2, CuratedStatus::PartiallyResolved)
    );
    assert_eq!(curated.status_evidence.len(), 1);
    // The earlier version stays stored exactly as collected, and its retry still answers.
    assert_eq!(
        stored,
        CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", Some(1))
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        serde_json::to_value(&receipt).unwrap(),
        serde_json::to_value(w.app.collect_problem(&curator, first()).await.unwrap()).unwrap()
    );
    assert_eq!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
}

#[tokio::test]
async fn a_curated_problem_and_the_same_problem_named_by_a_collected_work_are_one_entry() {
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    let named = |name: &str| serde_json::json!([{"name":name,"source":{"kind":"oeis","locator":"A391810"}}]);
    let receipt = w
        .app
        .collect_problem(
            &curator,
            delivery(envelope("outreach", "hanna-formula-5", 1, NAME, "A391810")),
        )
        .await
        .unwrap();
    let id = receipt.research_problem;

    // A collected work naming the same question joins the curated entry.
    collect_work(&w, &curator, "outreach", "a-proof", 1, named(NAME)).await;
    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, id);
    assert_eq!(page.items[0].notes, 1);
    // Being named by a collected work settles nothing: the curated reading stands.
    assert_eq!(page.items[0].status, ResearchProblemStatus::CuratedOpen);
    let detail = w.app.research_problem(&id).await.unwrap();
    assert!(detail.curated.is_some());
    assert_eq!(detail.collected_notes.len(), 1);
    assert_eq!(detail.collected_notes[0].work, "a-proof");
    let paper = w.app.collected_paper("a-proof", None).await.unwrap();
    assert_eq!(paper.problem_links[0].as_ref().unwrap().id, id);

    // Another producer naming the same question is another problem.
    collect_work(&w, &curator, "another-producer", "b-proof", 1, named(NAME)).await;
    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(page.items.len(), 2);
    let other = page.items.iter().find(|p| p.id != id).unwrap();
    assert_eq!(other.status, ResearchProblemStatus::RelatedResearch);
    assert!(other.status_as_of.is_none());
    assert!(
        w.app
            .research_problem(&other.id)
            .await
            .unwrap()
            .curated
            .is_none()
    );

    // The order of collection does not matter.
    let later = "Hanna's formula (6) in OEIS A391810";
    collect_work(&w, &curator, "outreach", "c-proof", 1, named(later)).await;
    let before = w.app.collected_paper("c-proof", None).await.unwrap();
    let link = before.problem_links[0].as_ref().unwrap().id.clone();
    let receipt = w
        .app
        .collect_problem(
            &curator,
            delivery(envelope(
                "outreach",
                "hanna-formula-6",
                1,
                later,
                " A391810 ",
            )),
        )
        .await
        .unwrap();
    assert_eq!(receipt.research_problem, link);
    let detail = w.app.research_problem(&link).await.unwrap();
    assert_eq!(detail.summary.status, ResearchProblemStatus::CuratedOpen);
    assert_eq!((detail.summary.notes, detail.collected_notes.len()), (1, 1));

    // The entry outlives the works that name it.
    let stored = CollectedPaperStore::get(w.stores.as_ref(), "a-proof", Some(1))
        .await
        .unwrap();
    collect_work(
        &w,
        &curator,
        "outreach",
        "a-proof",
        2,
        serde_json::json!([]),
    )
    .await;
    let detail = w.app.research_problem(&id).await.unwrap();
    assert_eq!(detail.summary.notes, 0);
    assert!(detail.collected_notes.is_empty() && detail.curated.is_some());
    assert_eq!(
        stored,
        CollectedPaperStore::get(w.stores.as_ref(), "a-proof", Some(1))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn status_evidence_cannot_be_dated_after_the_collection_day() {
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    let today = chrono::Utc::now().date_naive();
    let mut future = envelope("outreach", "hanna-formula-5", 1, NAME, "A391810");
    future["problem"]["status_evidence"][0]["checked_on"] =
        (today + chrono::Days::new(3)).to_string().into();
    assert!(matches!(
        w.app.collect_problem(&curator, delivery(future)).await,
        Err(CoreError::Invalid(_))
    ));
    let mut current = envelope("outreach", "hanna-formula-5", 1, NAME, "A391810");
    current["problem"]["status_evidence"][0]["checked_on"] = today.to_string().into();
    w.app
        .collect_problem(&curator, delivery(current))
        .await
        .unwrap();
    assert_eq!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items[0]
            .status_as_of,
        Some(today)
    );
}

#[tokio::test]
async fn curated_entries_page_with_the_rest_of_the_library() {
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    // More entries than one store page.
    for i in 0..103 {
        w.app
            .collect_problem(
                &curator,
                delivery(envelope(
                    "outreach",
                    &format!("problem-{i:03}"),
                    1,
                    &format!("Question {i}"),
                    "A391810",
                )),
            )
            .await
            .unwrap();
    }
    let mut seen = BTreeSet::new();
    let mut before = None;
    loop {
        let page = w
            .app
            .list_research_problems(Some(40), before)
            .await
            .unwrap();
        assert!(page.items.len() <= 40);
        for item in page.items {
            assert_eq!(item.status, ResearchProblemStatus::CuratedOpen);
            assert!(seen.insert(item.id));
        }
        before = page.next_before;
        if before.is_none() {
            break;
        }
    }
    assert_eq!(seen.len(), 103);
}

const TARGET: &str = "import Mathlib\n\ndef wishpool_target_prop : Prop :=\n  ∀ n : ℕ, 1 ≤ n → ∃ k : ℕ, 1 ≤ k ∧ k ≤ 2 * n\n";
const TOOLCHAIN: &str =
    "leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679";

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn with_target(mut value: serde_json::Value, text: &str) -> serde_json::Value {
    value["problem"]["lean_target"] = serde_json::json!({
        "text": text,
        "correspondence": "The sum over k is the finite sum of the first n + 1 terms.",
    });
    value
}

/// Answers as the independent verifier would for an elaboration witness and
/// keeps every request it was sent.
struct TargetVerifier {
    refusal: Option<&'static str>,
    requests: tokio::sync::Mutex<Vec<VerificationRequest>>,
}
#[async_trait::async_trait]
impl crate::ports::Verifier for TargetVerifier {
    async fn toolchain(&self) -> CoreResult<String> {
        Ok(TOOLCHAIN.into())
    }
    async fn verify(&self, r: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        self.requests.lock().await.push(r.clone());
        Ok(VerificationReceipt {
            used_constants: vec![],
            library_use: None,
            verdict: if self.refusal.is_some() {
                Verdict::Rejected
            } else {
                Verdict::Proved
            },
            reason: self
                .refusal
                .unwrap_or("Lean checked the imported target and standard axioms")
                .into(),
            target_digest: r.target_digest.clone(),
            solution_digest: lean_solution_digest(&r.solution, &r.modules),
            toolchain: r.toolchain.clone(),
            axioms: vec![],
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}

async fn target_world(refusal: Option<&'static str>) -> (World, Arc<TargetVerifier>) {
    let verifier = Arc::new(TargetVerifier {
        refusal,
        requests: Default::default(),
    });
    let mut w = World::new().await;
    let mut ports = w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.verifier = verifier.clone();
    w.app = App::with_internal_accounts(
        ports,
        Policy::default(),
        BTreeSet::from(["admin".into()]),
        20,
        BTreeSet::from([PersonId::from("curator")]),
    );
    (w, verifier)
}

fn target_job(slug: &str, version: u32) -> crate::ports::LeasedJob {
    crate::ports::LeasedJob {
        submission: SubmissionId(format!("collected-problem:{slug}:{version}")),
        kind: crate::ports::JobKind::CollectedProblemTarget,
        attempts: 1,
        lease: "target-fixture".into(),
    }
}

async fn public_target(w: &World, id: &str) -> Option<crate::app::PublicCuratedTarget> {
    w.app
        .research_problem(id)
        .await
        .unwrap()
        .curated
        .unwrap()
        .lean_target
}

#[tokio::test]
async fn a_curated_lean_target_is_public_once_the_independent_verifier_elaborated_it() {
    let (w, verifier) = target_world(None).await;
    let curator = w.person("curator", &[]).await;
    let reviewer = w.person("worker", &[Role::Reviewer]).await;
    let envelope = with_target(
        envelope("outreach", "hanna-formula-5", 1, NAME, "A391810"),
        TARGET,
    );
    let receipt = w
        .app
        .collect_problem(&curator, delivery(envelope.clone()))
        .await
        .unwrap();
    let job = target_job("hanna-formula-5", 1);
    assert_eq!(
        w.stores.queued().await,
        vec![(
            job.submission.clone(),
            crate::ports::JobKind::CollectedProblemTarget
        )]
    );

    // Before the check the reader learns that a formalization exists, never its text.
    let pending = public_target(&w, &receipt.research_problem).await.unwrap();
    assert_eq!(pending.state, crate::app::PublicTargetState::Checking);
    assert_eq!(pending.digest, sha(TARGET));
    assert_eq!(pending.vocabulary, TargetVocabulary::Mathlib);
    assert!(pending.text.is_none() && pending.toolchain.is_none() && pending.reason.is_none());

    // Only the worker runs the check, and a lost lease writes nothing.
    assert!(
        w.app
            .check_collected_problem_target_leased(&curator, &job, &TestAgentLease(true))
            .await
            .is_err()
    );
    assert!(verifier.requests.lock().await.is_empty());
    assert!(
        w.app
            .check_collected_problem_target_leased(&reviewer, &job, &TestAgentLease(false))
            .await
            .is_err()
    );
    assert_eq!(
        public_target(&w, &receipt.research_problem)
            .await
            .unwrap()
            .state,
        crate::app::PublicTargetState::Checking
    );

    w.app
        .check_collected_problem_target_leased(&reviewer, &job, &TestAgentLease(true))
        .await
        .unwrap();
    let elaborated = public_target(&w, &receipt.research_problem).await.unwrap();
    assert_eq!(elaborated.state, crate::app::PublicTargetState::Elaborated);
    assert_eq!(elaborated.text.as_deref(), Some(TARGET));
    assert_eq!(elaborated.toolchain.as_deref(), Some(TOOLCHAIN));
    assert!(elaborated.checked_at.is_some() && elaborated.reason.is_none());

    // The verifier never received a claim that the statement holds: it checked
    // `p → p` for the curator's proposition `p` in the pinned environment.
    let requests = verifier.requests.lock().await.clone();
    let request = requests.last().unwrap();
    assert_eq!(request.toolchain, TOOLCHAIN);
    assert_eq!(request.target_digest, sha(&request.target));
    assert_ne!(request.target_digest, sha(TARGET));
    assert_eq!(
        request.target,
        "import Mathlib\n\ndef wishpool_target_prop : Prop := let p : Prop := (\n  ∀ n : ℕ, 1 ≤ n → ∃ k : ℕ, 1 ≤ k ∧ k ≤ 2 * n\n\n); p → p\n"
    );
    assert_eq!(
        request.solution,
        "import Target\ntheorem wishpool_solution : wishpool_target_prop := fun h => h\n"
    );
    assert!(request.modules.is_empty() && request.required_declaration.is_none());

    // A checked version keeps its check: another run and an exact retry of the
    // delivery ask the verifier nothing and queue nothing.
    let asked = requests.len();
    w.app
        .check_collected_problem_target_leased(&reviewer, &job, &TestAgentLease(true))
        .await
        .unwrap();
    assert_eq!(verifier.requests.lock().await.len(), asked);
    let stored = CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", Some(1))
        .await
        .unwrap()
        .unwrap();
    let again = w
        .app
        .collect_problem(&curator, delivery(envelope))
        .await
        .unwrap();
    assert_eq!(again.request_digest, receipt.request_digest);
    assert_eq!(
        CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", Some(1))
            .await
            .unwrap()
            .unwrap(),
        stored
    );
}

#[tokio::test]
async fn a_refused_lean_target_shows_the_verifier_reason_and_never_its_text() {
    let (w, _) = target_world(Some("Target.lean:4:2: unknown identifier 'Foo.bar'")).await;
    let curator = w.person("curator", &[]).await;
    let reviewer = w.person("worker", &[Role::Reviewer]).await;
    let receipt = w
        .app
        .collect_problem(
            &curator,
            delivery(with_target(
                envelope("outreach", "hanna-formula-5", 1, NAME, "A391810"),
                TARGET,
            )),
        )
        .await
        .unwrap();
    w.app
        .check_collected_problem_target_leased(
            &reviewer,
            &target_job("hanna-formula-5", 1),
            &TestAgentLease(true),
        )
        .await
        .unwrap();
    let refused = public_target(&w, &receipt.research_problem).await.unwrap();
    assert_eq!(refused.state, crate::app::PublicTargetState::Refused);
    assert!(refused.text.is_none());
    assert_eq!(
        refused.reason.as_deref(),
        Some("Target.lean:4:2: unknown identifier 'Foo.bar'")
    );
    // The curator's reading of the problem itself is unaffected.
    assert_eq!(
        w.app
            .research_problem(&receipt.research_problem)
            .await
            .unwrap()
            .summary
            .status,
        crate::app::ResearchProblemStatus::CuratedOpen
    );
}

#[tokio::test]
async fn an_unavailable_or_late_verifier_leaves_the_target_check_pending() {
    // The default port has no verifier behind it.
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    let reviewer = w.person("worker", &[Role::Reviewer]).await;
    let receipt = w
        .app
        .collect_problem(
            &curator,
            delivery(with_target(
                envelope("outreach", "hanna-formula-5", 1, NAME, "A391810"),
                TARGET,
            )),
        )
        .await
        .unwrap();
    let job = target_job("hanna-formula-5", 1);
    assert!(matches!(
        w.app
            .check_collected_problem_target_leased(&reviewer, &job, &TestAgentLease(true))
            .await,
        Err(CoreError::Unavailable(_))
    ));
    assert_eq!(
        public_target(&w, &receipt.research_problem)
            .await
            .unwrap()
            .state,
        crate::app::PublicTargetState::Checking
    );

    let (late, _) = target_world(Some("verifier deadline exceeded")).await;
    let curator = late.person("curator", &[]).await;
    let reviewer = late.person("worker", &[Role::Reviewer]).await;
    let receipt = late
        .app
        .collect_problem(
            &curator,
            delivery(with_target(
                envelope("outreach", "hanna-formula-5", 1, NAME, "A391810"),
                TARGET,
            )),
        )
        .await
        .unwrap();
    assert!(matches!(
        late.app
            .check_collected_problem_target_leased(&reviewer, &job, &TestAgentLease(true))
            .await,
        Err(CoreError::Unavailable(_))
    ));
    assert_eq!(
        public_target(&late, &receipt.research_problem)
            .await
            .unwrap()
            .state,
        crate::app::PublicTargetState::Checking
    );
}

#[tokio::test]
async fn entries_without_a_lean_target_keep_their_bytes_and_queue_no_check() {
    let w = world(&["curator"]).await;
    let curator = w.person("curator", &[]).await;
    let plain = envelope("outreach", "hanna-formula-5", 1, NAME, "A391810");
    let receipt = w
        .app
        .collect_problem(&curator, delivery(plain.clone()))
        .await
        .unwrap();
    assert!(w.stores.queued().await.is_empty());
    assert!(public_target(&w, &receipt.research_problem).await.is_none());
    // The canonical form of an envelope without a target has no new field, so
    // digests of entries collected before targets existed stay what they were.
    let canonical = serde_json::to_value(delivery(plain)).unwrap();
    assert!(canonical["problem"].get("lean_target").is_none());
    let stored = serde_json::to_value(
        CollectedProblemStore::get(w.stores.as_ref(), "hanna-formula-5", Some(1))
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(stored.get("target_check").is_none());

    // A target arrives as the next version of the same entry.
    let v2 = with_target(
        envelope("outreach", "hanna-formula-5", 2, NAME, "A391810"),
        TARGET,
    );
    let second = w.app.collect_problem(&curator, delivery(v2)).await.unwrap();
    assert_eq!(second.research_problem, receipt.research_problem);
    assert_eq!(
        w.stores.queued().await,
        vec![(
            SubmissionId("collected-problem:hanna-formula-5:2".into()),
            crate::ports::JobKind::CollectedProblemTarget
        )]
    );
    for broken in [
        "import Mathlib\nimport Aesop\ndef wishpool_target_prop : Prop := True\n",
        "def wishpool_target_prop : Prop := True\n",
        "import Mathlib\ndef wishpool_target_prop : Prop := wishpool_target_prop\n",
        "import Mathlib\ndef wishpool_target_prop : Prop :=\n",
        "import Mathlib\ntheorem wishpool_target : True := trivial\n",
    ] {
        assert!(matches!(
            w.app
                .collect_problem(
                    &curator,
                    delivery(with_target(
                        envelope("outreach", "hanna-formula-5", 3, NAME, "A391810"),
                        broken
                    )),
                )
                .await,
            Err(CoreError::Invalid(_))
        ));
    }
}
