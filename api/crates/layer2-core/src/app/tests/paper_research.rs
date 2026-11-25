use super::*;
use crate::ports::CollectedPaperStore;
use serde_json::json;

#[tokio::test]
async fn public_reading_keeps_source_questions_and_version_bound_requests_without_guessing_proofs()
{
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader)),
        Policy::default(),
        BTreeSet::new(),
        1,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let (mut delivery, pdf, source) = super::collected::package(1);
    delivery.paper.lean = Some(CollectedLean {
        url: format!(
            "https://github.com/the-omega-institute/trureturing/blob/{}/Proof.lean",
            "a".repeat(40)
        ),
        revision: "a".repeat(40),
    });
    w.app
        .collect_pre_reviewed(&owner, delivery, pdf, source)
        .await
        .unwrap();
    let source = w.stores.get("a-note", Some(1)).await.unwrap().unwrap();
    let revision = source.source_index.as_ref().unwrap().revision;
    w.stores
        .save_source_index(
            "a-note",
            1,
            &CollectedSourceIndex {
                revision: revision + 1,
                source_sha256: source.source.sha256,
                bibliography: None,
                limitations: vec![],
                results: Some(CollectedResults {
                    statements: vec![
                        CollectedStatement {
                            id: "result".into(),
                            kind: ClaimKind::Theorem,
                            name: "Theorem 1".into(),
                            title: Some("A precise bound".into()),
                            statement: "The finite bound holds.".into(),
                            section: None,
                        },
                        CollectedStatement {
                            id: "question".into(),
                            kind: ClaimKind::Conjecture,
                            name: "Question 2".into(),
                            title: None,
                            statement: "Does the bound extend?".into(),
                            section: None,
                        },
                    ],
                    ..Default::default()
                }),
            },
            revision,
        )
        .await
        .unwrap();
    let mut requests = vec![];
    for number in 1..=3 {
        requests.push(SharedFormalizationRequest {
            number,
            title: "A precise bound".into(),
            body: "A formalization target.".into(),
            comments: vec![],
            url: format!("https://github.com/{REQUEST_REPOSITORY}/issues/{number}"),
            state: if number == 1 { "merged" } else { "open" }.into(),
            updated_at: chrono::Utc::now(),
            requested_by: "publisher".into(),
            claimants: vec![],
            completions: if number == 1 {
                vec![RequestCompletion {
                    url: "https://github.com/the-omega-institute/trureturing/pull/1".into(),
                    revision: "a".repeat(40),
                    merged_at: chrono::Utc::now(),
                    contributor: RequestContributor {
                        login: "contributor".into(),
                        url: "https://github.com/contributor".into(),
                        agent: false,
                    },
                    ranked: false,
                }]
            } else {
                vec![]
            },
            paper_repository: (number == 2)
                .then(|| "https://github.com/mathesis-papers/paper".into()),
            lean_source: None,
        });
    }
    w.app
        .replace_shared_formalization_requests(
            &SharedRequestsSnapshot {
                revision: 1,
                observed_at: Some(chrono::Utc::now()),
                requests,
            },
            0,
        )
        .await
        .unwrap();
    let materials = json!({"schema":"mathesis.materials-index.v1","generated_at":"2026-11-24T00:00:00Z","entries":[{
        "kind":"collected","id":"a-note","version":1,
        "repository_url":"https://github.com/mathesis-papers/paper", "site_url":"https://papers.trureturing.org/paper/", "reading":{}, "slides":[], "visualizations":[],
        "formalization_requests":[{"label":"A precise bound","url":format!("https://github.com/{REQUEST_REPOSITORY}/issues/1")}]
    }]});
    w.app
        .reconcile_materials_index(&serde_json::to_vec(&materials).unwrap())
        .await
        .unwrap();
    let reading = w.app.paper_research("a-note", Some(1)).await.unwrap();
    assert_eq!(reading.results.len(), 1);
    assert_eq!(reading.results[0].name, "A precise bound");
    assert_eq!(
        reading.results[0].state, "published",
        "a source link or merged PR is not a proof receipt"
    );
    assert_eq!(reading.questions.len(), 1);
    assert!(reading.lean_source.is_some());
    assert_eq!(
        reading.requests.len(),
        2,
        "same title on another issue must not join"
    );
    assert_eq!(reading.requests[0].association, "paper_link");
    assert_eq!(reading.requests[1].association, "repository");
    assert!(w.app.paper_research("a-note", Some(2)).await.is_err());
    assert!(w.app.paper_research("missing", None).await.is_err());
    let (delivery, pdf, source) = super::collected::package(2);
    w.app
        .collect_pre_reviewed(&owner, delivery, pdf, source)
        .await
        .unwrap();
    let historical = w.app.paper_research("a-note", Some(1)).await.unwrap();
    assert_eq!(historical.results.len(), 1);
    assert!(
        historical.requests.is_empty(),
        "historical source must not inherit current request associations"
    );
    let latest = w.app.paper_research("a-note", None).await.unwrap();
    assert_eq!(latest.version, 2);
    assert!(latest.requests.is_empty());
}
