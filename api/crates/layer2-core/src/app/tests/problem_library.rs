use super::*;
use crate::ports::RecordStore;

fn delivery(work: &str, version: &str, previous: Option<u32>) -> NewManuscriptDelivery {
    serde_json::from_value(serde_json::json!({
        "schema":"mathesis.manuscript-delivery.v1", "producer":"scientific-source", "work_id":work,
        "version":version, "previous_version":previous,
        "paper":{"kind":"note", "title":format!("Note {work} {version}"),"authors":[{"name":"Note Author"}],
            "processing_consent":"ai-review-request-2026-10-29", "ai_disclosure":{"level":"assisted","statement":"Proofreading."}},
        "origin":{"problem_ids":["original-problem","another-problem"],"source_url":"https://example.org/note", "license":"CC-BY-4.0",
            "problems":[{"id":"original-problem","problem":{"name":"An original conjecture", "source":{"kind":"oeis","locator":"A123456"}},
            "statement":"For every positive integer, the proposed identity holds.","proposed_by":[{"name":"Original Poser"}],
            "resolutions":[{"claim_label":"C1","outcome":"proved"}]}]}
    })).unwrap()
}

fn upload() -> Upload {
    Upload {
        filename: "t.tex".into(),
        bytes: b"scientific source".to_vec(),
    }
}

async fn publish(w: &World, author: &Caller, work: &str, n: u32) -> (RecordId, Submission) {
    let receipt = w
        .app
        .deliver_manuscript(author, delivery(work, "1", None), upload())
        .await
        .unwrap();
    let draft = w.app.submission(author, &receipt.submission).await.unwrap();
    let mut s = w.review_paper(author, &draft).await;
    let id = RecordId::from(format!("WP-2026-{n:04}"));
    s.status = SubmissionStatus::Accepted { record: id.clone() };
    w.app.save(&mut s).await.unwrap();
    // Synthetic public input, as in the citation-graph fixtures. No live review runs.
    RecordStore::insert(
        w.stores.as_ref(),
        &Record {
            id: id.clone(),
            kind: s.kind,
            submission: s.id.clone(),
            title: s.title.clone(),
            authors: s.authors.clone(),
            publication: Some(Box::new(s.clone())),
            basis: AdmissionBasis::EscapeWitness,
            accepted_by: "synthetic-test".into(),
            accepted_at: chrono::Utc::now(),
        },
    )
    .await
    .unwrap();
    (id, s)
}

#[tokio::test]
async fn source_problems_group_notes_and_preserve_credit_without_inventing_a_solution() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (record, mut first) = publish(&w, &author, "first", 1).await;
    publish(&w, &author, "second", 2).await;
    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(page.items.len(), 2);
    let p = page
        .items
        .iter()
        .find(|p| p.source_id == "original-problem")
        .unwrap();
    assert_eq!(p.notes, 2);
    assert_eq!(p.status, ResearchProblemStatus::RelatedResearch);
    let detail = w.app.research_problem(&p.id).await.unwrap();
    assert_eq!(detail.descriptions[0].proposed_by[0].name, "Original Poser");
    assert_eq!(detail.notes[0].authors[0].name, "Note Author");
    assert!(
        detail
            .notes
            .iter()
            .flat_map(|n| &n.results)
            .all(|r| r.outcome.is_none())
    );
    assert!(
        w.app
            .paper(&record)
            .await
            .unwrap()
            .problems
            .iter()
            .any(|link| link.id == p.id)
    );
    let one = w.app.list_research_problems(Some(1), None).await.unwrap();
    let two = w
        .app
        .list_research_problems(Some(1), one.next_before)
        .await
        .unwrap();
    assert_ne!(one.items[0].id, two.items[0].id);
    assert!(two.next_before.is_none());
    // Another submitting identity cannot attach itself to this problem's history.
    let other = w.person("other", &[]).await;
    publish(&w, &other, "first", 3).await;
    assert_eq!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items
            .len(),
        4
    );
    first.analysis_visibility = Visibility::Private;
    w.app.save(&mut first).await.unwrap();
    assert!(w.app.paper(&record).await.unwrap().problems.is_empty());
    assert_eq!(w.app.research_problem(&p.id).await.unwrap().notes.len(), 1);
}

#[tokio::test]
async fn accepted_revisions_advance_one_note_and_private_revisions_keep_the_last_accepted_input() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (record, first) = publish(&w, &author, "stable-note", 1).await;
    let problem_id = w.app.paper(&record).await.unwrap().problems[0].id.clone();
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    w.app
        .record_compilation(&reviewer, &first.id, 1, Ok(b"%PDF rewritten".to_vec()))
        .await
        .unwrap();

    let second = w
        .app
        .deliver_manuscript(&author, delivery("stable-note", "2", Some(1)), upload())
        .await
        .unwrap();
    assert_eq!(
        w.app.paper(&record).await.unwrap().summary.title,
        first.title
    );
    assert!(second.publication.is_none());
    let mut accepted = w.app.submission(&author, &second.submission).await.unwrap();
    accepted = w.review_version(&author, &accepted, 2).await;
    let editor = w.person("editor", &[Role::Editor]).await;
    w.literature(&editor, &accepted, vec![]).await;
    let (shape, witnesses) = content();
    w.app
        .judge_claim(
            &editor,
            &accepted.id,
            &"C1".into(),
            shape,
            witnesses,
            "New result.".into(),
        )
        .await
        .unwrap();
    w.app.adopt_judgements(&editor, &accepted.id).await.unwrap();
    w.app.decide(&editor, &accepted.id).await.unwrap();
    let public = w.app.paper(&record).await.unwrap();
    assert_eq!(public.publication.unwrap().version, 2);
    assert_eq!(public.summary.title, "Note stable-note 2");
    assert_eq!(
        w.app
            .paper_file(None, &first.id, Some(1), true)
            .await
            .unwrap()
            .bytes,
        b"%PDF"
    );

    assert_eq!(public.problems[0].id, problem_id);
    assert_eq!(
        w.app.research_problem(&problem_id).await.unwrap().notes[0].version,
        2
    );
    assert_eq!(
        w.app
            .manuscript_receipt(&author, &first.id, Some(2))
            .await
            .unwrap()
            .publication
            .unwrap()
            .version,
        2
    );
    w.app
        .deliver_manuscript(&author, delivery("stable-note", "3", Some(2)), upload())
        .await
        .unwrap();
    let p = w.app.paper(&record).await.unwrap();
    assert_eq!(p.summary.title, "Note stable-note 2");
    assert_eq!(p.versions.len(), 2);
    assert_eq!(
        w.app.research_problem(&problem_id).await.unwrap().notes[0].version,
        2
    );
    assert_eq!(
        RecordStore::get(w.stores.as_ref(), &record)
            .await
            .unwrap()
            .unwrap()
            .publication
            .unwrap()
            .title,
        first.title
    );
}

#[tokio::test]
async fn claimed_settlement_needs_a_matching_supported_source_audit() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (_, mut s) = publish(&w, &author, "settlement", 1).await;
    let problem = s
        .current_version()
        .unwrap()
        .delivery
        .as_ref()
        .unwrap()
        .origin
        .problems[0]
        .problem
        .clone();
    s.claims[0].settles = Some(problem.clone());
    let mut methodology = MethodologyReading::insufficient("synthetic fixture");
    methodology.literature = EvidenceStatus::Supported;
    methodology.searched = vec!["Original problem and stated hypotheses".into()];
    methodology.baseline = vec![PrerequisiteReading {
        statement: "The original problem's definitions".into(),
        location: "OEIS source".into(),
        source: Some("https://oeis.org/A123456".into()),
    }];
    methodology.settlement = Some(SettlementReading {
        status: EvidenceStatus::Supported,
        problem: problem.clone(),
        statement: "Original statement".into(),
        assumptions: "All original assumptions".into(),
        prior_status: "Open in the cited source".into(),
        resolution: "A proof within the original scope".into(),
        matching_argument: "Exact statement and hypotheses".into(),
        opened_source: "https://oeis.org/A123456".into(),
    });
    s.reports.push(StageReport {
        stage: Stage::Escape,
        outcome: Outcome::Pass,
        summary: "Synthetic source audit".into(),
        payload: StagePayload::Escape {
            assessments: vec![EscapeAssessment {
                claim: "C1".into(),
                correctness: Some(Correctness::Correct),
                shape: ProofShape::BindOnly,
                witnesses: vec![],
                rationale: "Matched the original problem".into(),
                conjecture: None,
                methodology: Some(methodology),
                escape_rate: None,
            }],
        },
        evidence: vec![],
        reviewer: ReviewerIdentity::Human {
            person: "synthetic-test".into(),
        },
        claims_revision: s.claims_revision,
        filed_at: chrono::Utc::now(),
    });
    // Keep the immutable fixture's accepted input aligned to this source audit.
    s.status = SubmissionStatus::Accepted {
        record: "WP-2026-0001".into(),
    };
    let mut snapshot = s.clone();
    snapshot.published_progress = None;
    s.published_progress = Some(PublishedProgress {
        version: 1,
        claims_revision: s.claims_revision,
        formalization: s.formalization.clone(),
        conjectures: s.conjectures.clone(),
        publication: Some(Box::new(snapshot)),
    });
    w.app.save(&mut s).await.unwrap();
    let page = w.app.list_research_problems(None, None).await.unwrap();
    assert_eq!(
        page.items
            .iter()
            .find(|p| p.source_id == "original-problem")
            .unwrap()
            .status,
        ResearchProblemStatus::Settled
    );
    assert_eq!(
        page.items
            .iter()
            .find(|p| p.source_id == "another-problem")
            .unwrap()
            .status,
        ResearchProblemStatus::RelatedResearch
    );
    // The same result cannot settle a different named problem just because its note is accepted.
    s.claims[0].settles.as_mut().unwrap().name = "A different question".into();
    let mut snapshot = s.clone();
    snapshot.published_progress = None;
    s.published_progress.as_mut().unwrap().publication = Some(Box::new(snapshot));
    w.app.save(&mut s).await.unwrap();
    assert!(
        w.app
            .list_research_problems(None, None)
            .await
            .unwrap()
            .items
            .iter()
            .all(|p| p.status == ResearchProblemStatus::RelatedResearch)
    );
}

#[test]
fn old_delivery_bytes_are_preserved_and_descriptions_must_match_the_association() {
    let mut d = delivery("work", "1", None);
    d.origin.problems.clear();
    assert!(
        serde_json::to_value(&d).unwrap()["origin"]
            .get("problems")
            .is_none()
    );
    let mut d = delivery("work", "1", None);
    d.origin.problems[0].id = "unassociated-problem".into();
    assert!(d.validate().is_err());
}

#[tokio::test]
async fn many_related_notes_have_a_bounded_complete_public_pagination() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    for n in 1..=26 {
        publish(&w, &author, &format!("note-{n}"), n).await;
    }
    let listing = w.app.list_research_problems(None, None).await.unwrap();
    let id = &listing.items[0].id;
    let first = w.app.research_problem(id).await.unwrap();
    assert_eq!(first.summary.notes, 26);
    assert_eq!(first.notes.len(), 25);
    let second = w
        .app
        .research_problem_page(id, first.next_before.as_deref())
        .await
        .unwrap();
    assert_eq!(second.notes.len(), 1);
    assert!(second.next_before.is_none());
    assert!(
        !first
            .notes
            .iter()
            .any(|n| n.record == second.notes[0].record)
    );
}
