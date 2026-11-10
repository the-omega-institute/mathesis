use super::*;
use crate::ports::CollectedPaperStore;
use sha2::{Digest, Sha256};

fn package(
    producer: &str,
    work: &str,
    version: u32,
    problems: serde_json::Value,
) -> (PreReviewedDelivery, Upload, Upload) {
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
        "paper":{"title":format!("Collected {work} v{version}"), "abstract":"A reviewed note.",
            "authors":[{"name":"Collected Author"}], "license":"CC-BY-4.0", "problems":problems,
            "lean":{"url":"https://github.com/example/project/blob/0123456789012345678901234567890123456789/Note.lean",
                "revision":"0123456789012345678901234567890123456789"}},
        "review":{"verdict":"PASS", "pdf_sha256":format!("{:x}",Sha256::digest(&pdf.bytes)),
            "source_sha256":format!("{:x}",Sha256::digest(&source.bytes)),
            "record":{"private_evidence":"must not be exposed"}}
    }))
    .unwrap();
    (delivery, pdf, source)
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

async fn collect(
    w: &World,
    operator: &Caller,
    producer: &str,
    work: &str,
    version: u32,
    problems: serde_json::Value,
) {
    let (delivery, pdf, source) = package(producer, work, version, problems);
    w.app
        .collect_pre_reviewed(operator, delivery, pdf, source)
        .await
        .unwrap();
}

const HANNA: &str = "Hanna's alternating-sum formula (5) in OEIS A391810";

fn hanna() -> serde_json::Value {
    serde_json::json!({"name":HANNA, "source":{"kind":"oeis","locator":"A391810"}})
}

#[tokio::test]
async fn collected_works_join_a_named_problem_once_at_their_current_public_version() {
    let w = world(&["publisher"]).await;
    let operator = w.person("publisher", &[]).await;
    collect(
        &w,
        &operator,
        "outreach",
        "first-note",
        1,
        serde_json::json!([hanna()]),
    )
    .await;
    // One source can carry several distinct questions; a repeated entry counts once.
    let other = serde_json::json!({"name":"Hanna's formula (6) in OEIS A391810",
        "source":{"kind":"oeis","locator":"A391810"}});
    collect(
        &w,
        &operator,
        "outreach",
        "second-note",
        1,
        serde_json::json!([hanna(), other, hanna()]),
    )
    .await;
    let queued = w.stores.queued().await;

    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(page.items.len(), 2);
    let p = page.items.iter().find(|p| p.title == HANNA).unwrap();
    let sibling = page.items.iter().find(|o| o.id != p.id).unwrap();
    assert_eq!((p.notes, sibling.notes), (2, 1));
    assert_eq!(p.status, ResearchProblemStatus::RelatedResearch);
    assert_eq!(p.source_id, "A391810");
    assert_eq!(
        p.source,
        Some(Source {
            kind: SourceKind::Oeis,
            locator: "A391810".into(),
            year: None,
        })
    );
    let detail = w.app.research_problem(&p.id).await.unwrap();
    assert!(detail.notes.is_empty() && detail.descriptions.is_empty());
    assert!(detail.next_before.is_none());
    assert_eq!(
        detail
            .collected_notes
            .iter()
            .map(|n| (n.work.as_str(), n.version))
            .collect::<Vec<_>>(),
        vec![("second-note", 1), ("first-note", 1)]
    );
    let first = &detail.collected_notes[1];
    assert_eq!(first.title, "Collected first-note v1");
    assert_eq!(first.authors[0].name, "Collected Author");
    assert_eq!(first.public_path, "/papers/first-note?version=1");
    assert_eq!(first.license, "CC-BY-4.0");
    assert!(first.lean.as_ref().unwrap().url.ends_with("/Note.lean"));

    // The reader projection links each named problem without exposing its operator or producer.
    let public = w.app.collected_paper("second-note", None).await.unwrap();
    assert_eq!(public.paper.problems.len(), 3);
    assert_eq!(
        public
            .problem_links
            .iter()
            .map(|link| link.as_ref().unwrap().id.as_str())
            .collect::<Vec<_>>(),
        vec![p.id.as_str(), sibling.id.as_str(), p.id.as_str()]
    );
    assert_eq!(public.problem_links[0].as_ref().unwrap().title, HANNA);
    let listed = w
        .app
        .collected_papers(PaperSearch::default(), None, None)
        .await
        .unwrap();
    assert!(listed.items.iter().all(|paper| {
        paper.problem_links.len() == paper.paper.problems.len()
            && paper.problem_links.iter().all(Option::is_some)
    }));
    let json = serde_json::to_string(&(&public, &detail, &page.items)).unwrap();
    assert!(!json.contains("publisher") && !json.contains("outreach"));
    assert!(!json.contains("private_evidence"));

    // A new version replaces the work's entry; the earlier version stays readable.
    collect(
        &w,
        &operator,
        "outreach",
        "first-note",
        2,
        serde_json::json!([hanna()]),
    )
    .await;
    let detail = w.app.research_problem(&p.id).await.unwrap();
    assert_eq!(detail.summary.notes, 2);
    let first = &detail.collected_notes[1];
    assert_eq!(
        (first.version, first.title.as_str()),
        (2, "Collected first-note v2")
    );
    assert_eq!(first.public_path, "/papers/first-note?version=2");
    let old = w.app.collected_paper("first-note", Some(1)).await.unwrap();
    assert_eq!(old.revision, "1");
    assert_eq!(old.problem_links[0].as_ref().unwrap().id, p.id);

    // The current version decides the association. Stored deliveries are untouched.
    let stored = w.stores.get("first-note", Some(1)).await.unwrap();
    collect(
        &w,
        &operator,
        "outreach",
        "first-note",
        3,
        serde_json::json!([]),
    )
    .await;
    let detail = w.app.research_problem(&p.id).await.unwrap();
    assert_eq!(detail.summary.notes, 1);
    assert_eq!(detail.collected_notes.len(), 1);
    assert_eq!(detail.collected_notes[0].work, "second-note");
    let old = w.app.collected_paper("first-note", Some(1)).await.unwrap();
    assert_eq!(old.paper.problems.len(), 1);
    assert!(old.problem_links.iter().all(Option::is_none));
    let current =
        serde_json::to_value(w.app.collected_paper("first-note", None).await.unwrap()).unwrap();
    assert!(current.get("problem_links").is_none());
    assert_eq!(stored, w.stores.get("first-note", Some(1)).await.unwrap());

    // A problem without a remaining public work has no page.
    collect(
        &w,
        &operator,
        "outreach",
        "second-note",
        2,
        serde_json::json!([]),
    )
    .await;
    assert!(matches!(
        w.app.research_problem(&p.id).await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    // Reading the library creates no review, target or proof work.
    let later = [
        "collected:first-note:2",
        "collected:first-note:3",
        "collected:second-note:2",
    ];
    let after: Vec<_> = w
        .stores
        .queued()
        .await
        .into_iter()
        .filter(|(id, _)| !later.contains(&id.as_str()))
        .collect();
    assert_eq!(after, queued);
    assert!(
        crate::ports::SolvingStore::all(w.stores.as_ref())
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn collected_problem_identity_needs_the_same_operator_producer_source_and_name() {
    let w = world(&["publisher", "author"]).await;
    let operator = w.person("publisher", &[]).await;
    let author = w.person("author", &[]).await;
    collect(
        &w,
        &operator,
        "outreach",
        "a-note",
        1,
        serde_json::json!([hanna()]),
    )
    .await;
    // The source year and surrounding whitespace describe the same named source question.
    collect(
        &w,
        &operator,
        "outreach",
        "b-note",
        1,
        serde_json::json!([{"name":format!(" {HANNA} "),
            "source":{"kind":"oeis","locator":"A391810","year":2026}}]),
    )
    .await;
    // Another producer, operator, name or source never merges.
    collect(
        &w,
        &operator,
        "another-producer",
        "c-note",
        1,
        serde_json::json!([hanna()]),
    )
    .await;
    collect(
        &w,
        &author,
        "outreach",
        "d-note",
        1,
        serde_json::json!([hanna()]),
    )
    .await;
    collect(
        &w,
        &operator,
        "outreach",
        "e-note",
        1,
        serde_json::json!([{"name":format!("{HANNA}, revision 39"),
            "source":{"kind":"oeis","locator":"A391810"}}]),
    )
    .await;
    collect(
        &w,
        &operator,
        "outreach",
        "f-note",
        1,
        serde_json::json!([{"name":HANNA, "source":{"kind":"url","locator":"https://oeis.org/A391810"}}]),
    )
    .await;
    let page = w.app.list_research_problems(None, None).await.unwrap();
    let mut notes: Vec<_> = page.items.iter().map(|p| p.notes).collect();
    notes.sort_unstable();
    assert_eq!(notes, vec![1, 1, 1, 1, 2]);
    let shared = page.items.iter().find(|p| p.notes == 2).unwrap();
    assert_eq!(shared.title, HANNA);

    // A delivered manuscript with the same submitter, producer, name and source keeps
    // its own identity: nothing establishes that its problem ID is this question.
    collect(
        &w,
        &author,
        "scientific-source",
        "g-note",
        1,
        serde_json::json!([{"name":"An original conjecture","source":{"kind":"oeis","locator":"A123456"}}]),
    )
    .await;
    let (record, _) = super::problem_library::publish(&w, &author, "first", 1).await;
    let page = w.app.list_research_problems(None, None).await.unwrap();
    let same_name: Vec<_> = page
        .items
        .iter()
        .filter(|p| p.title == "An original conjecture")
        .collect();
    assert_eq!(same_name.len(), 2);
    for p in same_name {
        let detail = w.app.research_problem(&p.id).await.unwrap();
        assert_eq!(p.notes, 1);
        assert_eq!(detail.notes.len() + detail.collected_notes.len(), 1);
        assert_eq!(detail.notes.is_empty(), p.source.is_some());
        if let Some(note) = detail.notes.first() {
            assert_eq!(note.record, record);
        }
    }
}

#[tokio::test]
async fn collected_notes_page_with_the_same_bounded_cursor() {
    let w = world(&["publisher"]).await;
    let operator = w.person("publisher", &[]).await;
    // More works than one store page, collected in an order unrelated to their identities.
    for i in 0..101 {
        collect(
            &w,
            &operator,
            "outreach",
            &format!("note-{:03}", (i * 37) % 101 + 1),
            1,
            serde_json::json!([hanna()]),
        )
        .await;
    }
    let listing = w.app.list_research_problems(Some(1), None).await.unwrap();
    assert!(listing.next_before.is_none());
    let id = &listing.items[0].id;
    let first = w.app.research_problem(id).await.unwrap();
    assert_eq!(first.collected_notes.len(), 25);
    assert_eq!(first.next_before.as_deref(), Some("note-077"));
    let mut works = vec![];
    let mut before = None;
    loop {
        let page = w
            .app
            .research_problem_page(id, before.as_deref())
            .await
            .unwrap();
        assert_eq!(page.summary.notes, 101);
        assert!(page.collected_notes.len() <= 25);
        works.extend(page.collected_notes.into_iter().map(|n| n.work));
        before = page.next_before;
        if before.is_none() {
            break;
        }
    }
    let expected: Vec<_> = (1..=101).rev().map(|n| format!("note-{n:03}")).collect();
    assert_eq!(works, expected);
}
