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
