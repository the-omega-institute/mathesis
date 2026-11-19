use super::*;

#[tokio::test]
async fn removal_is_owner_only_withdrawn_recoverable_and_idempotent() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let outsider = w.person("outsider", &[]).await;
    let admin = w.person("admin", &[]).await;
    let paper = w.submit(&author, "t", false).await;
    assert!(matches!(
        w.app.set_removed_from_work(&author, &paper.id, true).await,
        Err(CoreError::Conflict(_))
    ));
    let withdrawn = w.app.withdraw(&author, &paper.id).await.unwrap();
    assert!(matches!(
        w.app
            .set_removed_from_work(&outsider, &paper.id, true)
            .await,
        Err(CoreError::NotFound { .. })
    ));
    assert!(matches!(
        w.app.set_removed_from_work(&admin, &paper.id, true).await,
        Err(CoreError::Forbidden(_))
    ));
    let removed = w
        .app
        .set_removed_from_work(&author, &paper.id, true)
        .await
        .unwrap();
    assert!(removed.removed_from_work_at.is_some());
    assert_eq!(removed.status, SubmissionStatus::Withdrawn);
    assert_eq!(removed.versions, withdrawn.versions);
    assert_eq!(removed.manuscript, withdrawn.manuscript);
    let again = w
        .app
        .set_removed_from_work(&author, &paper.id, true)
        .await
        .unwrap();
    assert_eq!(again.revision, removed.revision);
    assert_eq!(again.removed_from_work_at, removed.removed_from_work_at);
    assert!(
        w.app
            .list_submissions(&author, SubmissionScope::Mine, None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        w.app
            .list_submissions(&author, SubmissionScope::Removed, None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        w.app
            .list_submissions(&outsider, SubmissionScope::Removed, None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        w.app
            .list_submissions(&author, SubmissionScope::All, None, None)
            .await
            .is_err()
    );
    assert_eq!(
        w.app
            .list_submissions(&admin, SubmissionScope::All, None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let restored = w
        .app
        .set_removed_from_work(&author, &paper.id, false)
        .await
        .unwrap();
    assert!(restored.removed_from_work_at.is_none());
    assert_eq!(restored.versions, withdrawn.versions);
    assert_eq!(
        w.app
            .list_submissions(&author, SubmissionScope::Mine, None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
}

#[tokio::test]
async fn removal_filter_is_applied_before_pagination() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let first = w.submit(&author, "t", false).await;
    let second = w.submit(&author, "l", false).await;
    let third = w.submit(&author, "t", false).await;
    w.app.withdraw(&author, &second.id).await.unwrap();
    w.app
        .set_removed_from_work(&author, &second.id, true)
        .await
        .unwrap();
    let page = w
        .app
        .list_submissions(&author, SubmissionScope::Mine, Some(1), None)
        .await
        .unwrap();
    assert_eq!(page.items[0].id, third.id);
    let older = w
        .app
        .list_submissions(&author, SubmissionScope::Mine, Some(1), page.next_before)
        .await
        .unwrap();
    assert_eq!(older.items[0].id, first.id);
    assert!(older.next_before.is_none());
}
