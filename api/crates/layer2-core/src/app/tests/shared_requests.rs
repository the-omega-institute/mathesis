use super::*;

fn request(number: u64, ranked: bool) -> SharedFormalizationRequest {
    SharedFormalizationRequest {
        number,
        title: "A precise lemma".into(),
        url: format!("https://github.com/{REQUEST_REPOSITORY}/issues/{number}"),
        state: "merged".into(),
        updated_at: "2026-11-24T00:00:00Z".parse().unwrap(),
        requested_by: "paper-author".into(),
        claimants: vec![],
        completions: vec![RequestCompletion {
            url: "https://github.com/the-omega-institute/trureturing/pull/1".into(),
            revision: "a".repeat(40),
            merged_at: "2026-11-23T00:00:00Z".parse().unwrap(),
            contributor: RequestContributor {
                login: "external".into(),
                url: "https://github.com/external".into(),
                agent: false,
            },
            ranked,
        }],
        paper_repository: None,
        lean_source: None,
    }
}

#[tokio::test]
async fn shared_request_snapshot_is_atomic_and_closure_or_claiming_does_not_award_credit() {
    let w = World::new().await;
    let mut claimed = request(2, false);
    claimed.state = "claimed".into();
    claimed.completions.clear();
    claimed.claimants = vec![RequestContributor {
        login: "claimant".into(),
        url: "https://github.com/claimant".into(),
        agent: false,
    }];
    let mut closed = request(3, false);
    closed.state = "closed".into();
    closed.completions.clear();
    let snapshot = SharedRequestsSnapshot {
        revision: 1,
        observed_at: Some("2026-11-24T00:00:00Z".parse().unwrap()),
        requests: vec![request(1, true), claimed, closed, request(4, false)],
    };
    w.app
        .replace_shared_formalization_requests(&snapshot, 0)
        .await
        .unwrap();
    let rows = w
        .app
        .shared_formalization_requests()
        .await
        .unwrap()
        .leaderboard(false);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].completed, 1);
    assert_eq!(rows[0].requests, [1]);
    let mut invalid = snapshot.clone();
    invalid.revision = 2;
    invalid.requests[0].url = "https://example.test/fake".into();
    assert!(
        w.app
            .replace_shared_formalization_requests(&invalid, 1)
            .await
            .is_err()
    );
    assert_eq!(
        w.app.shared_formalization_requests().await.unwrap(),
        snapshot
    );
    assert!(
        w.app
            .replace_shared_formalization_requests(&snapshot, 0)
            .await
            .is_err()
    );
    assert!(
        w.app
            .formalization_leaderboard("all", "all")
            .await
            .unwrap()
            .is_empty(),
        "shared work never fabricates a local verifier credit"
    );
}

#[test]
fn board_deduplicates_multiple_prs_for_a_request_and_filters_by_completion_month() {
    let mut first = request(1, true);
    first.completions.push(first.completions[0].clone());
    let mut earlier = request(2, true);
    earlier.completions[0].merged_at = "2026-10-31T23:00:00Z".parse().unwrap();
    let snapshot = SharedRequestsSnapshot {
        revision: 1,
        observed_at: Some("2026-11-24T00:00:00Z".parse().unwrap()),
        requests: vec![first, earlier],
    };
    assert_eq!(snapshot.leaderboard(false)[0].completed, 2);
    assert_eq!(snapshot.leaderboard(true)[0].completed, 1);
}

#[tokio::test]
async fn collected_intake_and_reconciliation_never_admit_independent_proof_work() {
    let mut w = World::new().await;
    w.app = App::with_internal_accounts(
        w.stores.ports(Arc::new(SystemClock), Arc::new(FakeReader)),
        Policy::default(),
        BTreeSet::new(),
        1,
        BTreeSet::from(["publisher".into()]),
    );
    let owner = w.person("publisher", &[]).await;
    let (d, p, s) = super::collected::package(1);
    w.app.collect_pre_reviewed(&owner, d, p, s).await.unwrap();
    w.app.reconcile_collected_proofs().await.unwrap();
    assert!(
        w.stores
            .queued()
            .await
            .iter()
            .all(|(_, kind)| *kind != JobKind::CollectedFormalization)
    );
}

#[tokio::test]
async fn shared_request_import_refuses_forged_contributor_links_and_false_completion_states() {
    let w = World::new().await;
    let base = SharedRequestsSnapshot {
        revision: 1,
        observed_at: Some(chrono::Utc::now()),
        requests: vec![request(1, true)],
    };
    let mut anchored = base.clone();
    anchored.requests[0].lean_source = Some(format!(
        "https://github.com/the-omega-institute/trureturing/blob/{}/A.lean#L25",
        "b".repeat(40)
    ));
    w.app
        .replace_shared_formalization_requests(&anchored, 0)
        .await
        .unwrap();
    let mut malicious = base.clone();
    malicious.revision = 2;
    malicious.requests[0].completions[0].contributor.url = "javascript:alert(1)".into();
    assert!(
        w.app
            .replace_shared_formalization_requests(&malicious, 1)
            .await
            .is_err()
    );
    malicious = base.clone();
    malicious.requests[0].completions[0].contributor.login = "paper-author".into();
    malicious.requests[0].completions[0].contributor.url = "https://github.com/paper-author".into();
    assert!(
        w.app
            .replace_shared_formalization_requests(&malicious, 1)
            .await
            .is_err()
    );
    malicious = base.clone();
    malicious.requests[0].state = "closed".into();
    assert!(
        w.app
            .replace_shared_formalization_requests(&malicious, 1)
            .await
            .is_err()
    );
    assert_eq!(
        w.app.shared_formalization_requests().await.unwrap(),
        anchored
    );
}
