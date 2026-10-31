use super::*;
use crate::ports::RecordStore;

fn reference(doi: Option<&str>, title: &str) -> BibliographicReference {
    BibliographicReference {
        key: "earlier".into(),
        title: Some(title.into()),
        authors: vec!["E. Earlier (1901)".into()],
        year: Some("1901".into()),
        doi: doi.map(str::to_owned),
        arxiv: None,
        url: None,
        source_location: "references.bib".into(),
        raw: format!("Original source description: {title}"),
        raw_truncated: false,
    }
}
async fn publish(
    w: &World,
    author: &Caller,
    n: u32,
    r: BibliographicReference,
) -> (RecordId, Submission) {
    publish_with(
        w,
        author,
        n,
        Some(Bibliography {
            method: "latex_literal_citations_v1".into(),
            references: vec![r],
            limitations: vec!["Bounded literal extraction".into()],
        }),
    )
    .await
}
async fn publish_with(
    w: &World,
    author: &Caller,
    n: u32,
    bibliography: Option<Bibliography>,
) -> (RecordId, Submission) {
    let mut s = w.submit(author, "t", false).await;
    s.title = format!("Synthetic manuscript {n}");
    s.claims = s.extracted.clone();
    s.versions[0].bibliography = bibliography;
    let id = RecordId::from(format!("WP-2026-{n:04}"));
    s.status = SubmissionStatus::Accepted { record: id.clone() };
    w.app.save(&mut s).await.unwrap();
    RecordStore::insert(
        w.stores.as_ref(),
        &Record {
            id: id.clone(),
            publication: Some(Box::new(s.clone())),
            kind: s.kind,
            submission: s.id.clone(),
            title: s.title.clone(),
            authors: s.authors.clone(),
            basis: AdmissionBasis::EscapeWitness,
            accepted_by: "synthetic-test".into(),
            accepted_at: chrono::Utc::now(),
        },
    )
    .await
    .unwrap();
    (id, s)
}
fn focus(id: &str) -> ResearchGraphQuery {
    ResearchGraphQuery {
        focus: Some(id.into()),
        ..Default::default()
    }
}
#[tokio::test]
async fn shared_earlier_work_connects_two_manuscripts_and_visibility_controls_every_description() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (first, mut a) = publish(
        &w,
        &author,
        1,
        reference(Some("10.1234/OLD"), "An old result"),
    )
    .await;
    let (_, mut b) = publish(
        &w,
        &author,
        2,
        reference(
            Some("https://doi.org/10.1234/old"),
            "Alternative description",
        ),
    )
    .await;
    let graph = w
        .app
        .research_graph(focus("doi:10.1234/old"))
        .await
        .unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.relation == ResearchRelation::Cites && e.authors == ["E. Earlier (1901)"])
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.title.as_deref() == Some("Alternative description"))
    );
    // A hidden supporting occurrence cannot choose a shared node's description.
    b.analysis_visibility = Visibility::Private;
    w.app.save(&mut b).await.unwrap();
    let graph = w
        .app
        .research_graph(focus("doi:10.1234/old"))
        .await
        .unwrap();
    assert_eq!(graph.edges.len(), 1);
    let json = serde_json::to_string(&graph).unwrap();
    assert!(!json.contains("Alternative description"));
    assert!(!json.contains("Synthetic manuscript 2"));
    assert!(graph.nodes.iter().any(|n| n.title == "An old result"));
    a.analysis_visibility = Visibility::Private;
    w.app.save(&mut a).await.unwrap();
    assert!(
        w.app
            .research_graph(focus("doi:10.1234/old"))
            .await
            .unwrap()
            .nodes
            .is_empty()
    );
    assert!(matches!(
        w.app.research_graph(focus(&format!("paper:{first}"))).await,
        Err(CoreError::NotFound { .. })
    ));
    let public = w.app.paper(&first).await.unwrap();
    assert!(public.bibliography.is_none());
    assert!(
        !serde_json::to_string(&public)
            .unwrap()
            .contains("10.1234/OLD")
    );
}

#[tokio::test]
async fn focused_older_work_is_openable_after_unrelated_archive_pages() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    publish(
        &w,
        &author,
        1,
        reference(Some("10.1234/old"), "An old result"),
    )
    .await;
    publish(
        &w,
        &author,
        2,
        reference(Some("10.1234/old"), "An old result, second description"),
    )
    .await;
    for n in 3..=553 {
        if n <= 35 {
            let mut r = reference(Some("10.1234/old-extra"), "Unrelated citation");
            r.authors.clear();
            publish(&w, &author, n, r).await;
        } else {
            publish_with(&w, &author, n, None).await;
        }
    }

    let graph = w
        .app
        .research_graph(focus("doi:10.1234/old"))
        .await
        .unwrap();
    assert_eq!(graph.edges.len(), 2);
    assert!(graph.edges.iter().all(|edge| edge.to == "doi:10.1234/old"));
    assert!(graph.next_before.is_none());
    for text in [
        "An old result",
        "E. Earlier",
        "10.1234/OLD",
        "https://doi.org/10.1234/old",
    ] {
        let page = w
            .app
            .research_graph(ResearchGraphQuery {
                q: Some(text.into()),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.edges.len(), 1, "{text}");
        assert_eq!(page.coverage.len(), 1);
        let next = page.next_before.expect("a second matching source");
        let second = w
            .app
            .research_graph(ResearchGraphQuery {
                q: Some(text.into()),
                before: Some(next),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(second.edges.len(), 1);
        assert!(second.next_before.is_none());
        // Opening a discovered identity is independent of its discovery cursor.
        assert_eq!(
            w.app
                .research_graph(focus(&second.edges[0].to))
                .await
                .unwrap()
                .edges
                .len(),
            2
        );
    }
}
#[tokio::test]
async fn graph_and_references_use_accepted_sources_during_a_private_revision() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (record, mut s) = publish(
        &w,
        &author,
        1,
        reference(Some("10.1234/old"), "Accepted old reference"),
    )
    .await;
    let mut private_version = s.versions[0].clone();
    private_version.number = 2;
    private_version.bibliography.as_mut().unwrap().references =
        vec![reference(Some("10.1234/secret"), "PRIVATE NEW REFERENCE")];
    s.versions.push(private_version);
    s.title = "PRIVATE NEW TITLE".into();
    s.status = SubmissionStatus::Draft;
    s.claims_revision += 1;
    w.app.save(&mut s).await.unwrap();
    let graph = w
        .app
        .research_graph(focus(&format!("paper:{record}")))
        .await
        .unwrap();
    let json = serde_json::to_string(&graph).unwrap();
    assert!(!json.contains("PRIVATE"));
    assert!(!json.contains("secret"));
    assert!(graph.edges.iter().all(|e| e.version == 1));
    let public = w.app.paper(&record).await.unwrap();
    assert_eq!(
        public.bibliography.unwrap().references[0].work,
        "doi:10.1234/old"
    );
    assert!(
        w.app
            .research_graph(ResearchGraphQuery {
                q: Some("PRIVATE".into()),
                ..Default::default()
            })
            .await
            .unwrap()
            .nodes
            .is_empty()
    );
    assert!(
        w.app
            .research_graph(focus("doi:10.1234/secret"))
            .await
            .unwrap()
            .nodes
            .is_empty()
    );
}
#[tokio::test]
async fn ambiguous_metadata_never_merges_works_and_pages_are_explicitly_bounded() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (a, _) = publish(&w, &author, 1, reference(None, "Identical title")).await;
    let (b, _) = publish(&w, &author, 2, reference(None, "Identical title")).await;
    let graph = w
        .app
        .research_graph(ResearchGraphQuery::default())
        .await
        .unwrap();
    let literature: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == ResearchNodeKind::Literature)
        .collect();
    assert_eq!(literature.len(), 2);
    assert_ne!(literature[0].id, literature[1].id);
    let first = w
        .app
        .research_graph(ResearchGraphQuery {
            limit: Some(1),
            q: Some("no match".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(first.nodes.is_empty());
    assert!(first.next_before.is_none());
    assert!(first.coverage.is_empty());
    let first = w
        .app
        .research_graph(ResearchGraphQuery {
            limit: Some(1),
            q: Some("Identical title".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(first.next_before, Some(b.to_string()));
    assert_eq!(first.edges.len(), 1);
    let second = w
        .app
        .research_graph(ResearchGraphQuery {
            limit: Some(1),
            before: first.next_before,
            q: Some("Identical title".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(second.coverage[0].record, a);
    assert!(second.next_before.is_none());
    let local = w
        .app
        .research_graph(ResearchGraphQuery {
            focus: Some(format!("reference:{a}:1:earlier")),
            limit: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(local.edges.len(), 1);
    assert_eq!(local.edges[0].record, a);
    assert!(local.next_before.is_none());
    assert!(
        w.app
            .research_graph(focus("unsupported:whatever"))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn legacy_and_pdf_coverage_remain_explicit_without_invented_edges() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let (id, s) = publish_with(&w, &author, 1, None).await;
    let version_json = serde_json::to_value(&s.versions[0]).unwrap();
    assert!(version_json.get("bibliography").is_none());
    let digest = s.versions[0].archive.sha256.clone();
    let restored: PaperVersion = serde_json::from_value(version_json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), version_json);
    assert_eq!(restored.archive.sha256, digest);

    let graph = w
        .app
        .research_graph(focus(&format!("paper:{id}")))
        .await
        .unwrap();
    assert!(graph.edges.is_empty());
    assert_eq!(graph.coverage[0].method, None);
    assert!(graph.coverage[0].limitations[0].contains("not been indexed"));
    let (id, _) = publish_with(
        &w,
        &author,
        2,
        Some(Bibliography {
            method: "pdf_unindexed_v1".into(),
            references: vec![],
            limitations: vec!["PDF references are not indexed".into()],
        }),
    )
    .await;
    let graph = w
        .app
        .research_graph(focus(&format!("paper:{id}")))
        .await
        .unwrap();
    assert!(graph.edges.is_empty());
    assert_eq!(
        graph.coverage[0].method.as_deref(),
        Some("pdf_unindexed_v1")
    );
}
