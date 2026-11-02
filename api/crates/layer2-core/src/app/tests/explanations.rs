use super::*;
use crate::ports::RecordStore;

fn explanation(paper: &Submission) -> NewPaperExplanation {
    let v = paper.current_version().unwrap();
    NewPaperExplanation {
        version: v.number,
        claims_revision: paper.claims_revision,
        source_sha256: v.archive.sha256.clone(),
        generation: v.explanation_generation,
        text: PlainLanguageText {
            en: "The authors bound gaps in a sequence.".into(),
            zh: "作者研究一个数列中间隔的大小。".into(),
        },
    }
}

#[tokio::test]
async fn exposition_is_bilingual_source_bound_and_does_not_decide_review() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let draft = w.submit(&author, "t", false).await;
    assert!(
        w.app
            .record_paper_explanation(&reviewer, &draft.id, explanation(&draft))
            .await
            .is_err()
    );
    let paper = w.review_paper(&author, &draft).await;
    let input = explanation(&paper);
    assert!(
        w.app
            .record_paper_explanation(&author, &paper.id, input.clone())
            .await
            .is_err()
    );
    let saved = w
        .app
        .record_paper_explanation(&reviewer, &paper.id, input.clone())
        .await
        .unwrap();
    assert_eq!(saved.status, paper.status);
    assert_eq!(saved.decision, paper.decision);
    assert_eq!(saved.reports, paper.reports);
    assert_eq!(
        saved
            .current_version()
            .unwrap()
            .plain_language
            .as_ref()
            .unwrap()
            .text,
        input.text
    );
    let replay = w
        .app
        .record_paper_explanation(&reviewer, &paper.id, input.clone())
        .await
        .unwrap();
    assert_eq!(saved.revision, replay.revision);
    let mut correction = input;
    correction.text.en = "A corrected account of the same manuscript.".into();
    assert!(
        w.app
            .record_paper_explanation(&reviewer, &paper.id, correction.clone())
            .await
            .is_err()
    );
    let admin = w.person("admin", &[]).await;
    let corrected = w
        .app
        .record_paper_explanation(&admin, &paper.id, correction.clone())
        .await
        .unwrap();
    assert_eq!(
        corrected
            .current_version()
            .unwrap()
            .plain_language
            .as_ref()
            .unwrap()
            .text,
        correction.text
    );
    assert_eq!(corrected.status, paper.status);
    assert!(
        w.stores
            .queued()
            .await
            .contains(&(paper.id, JobKind::PaperExplanation))
    );
}

#[tokio::test]
async fn invalid_or_superseded_explanations_are_never_saved() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let draft = w.submit(&author, "t", false).await;
    let paper = w.review_paper(&author, &draft).await;
    let valid = explanation(&paper);
    for bad in 0..7 {
        let mut input = valid.clone();
        match bad {
            0 => input.version += 1,
            1 => input.claims_revision += 1,
            2 => input.source_sha256 = "unrelated source".into(),
            3 => input.generation += 1,
            4 => input.text.zh = "  ".into(),
            5 => input.text.en = "x".repeat(12_001),
            _ => input.text.zh = "English is not the requested Chinese explanation".into(),
        }
        assert!(
            w.app
                .record_paper_explanation(&reviewer, &paper.id, input)
                .await
                .is_err()
        );
    }
    assert!(
        w.app
            .submission(&author, &paper.id)
            .await
            .unwrap()
            .current_version()
            .unwrap()
            .plain_language
            .is_none()
    );
}

#[tokio::test]
async fn public_exposition_follows_the_accepted_version_and_visibility() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let draft = w.submit(&author, "t", false).await;
    let mut paper = w.review_paper(&author, &draft).await;
    let record = RecordId::from("WP-2026-0001");
    paper.status = SubmissionStatus::Accepted {
        record: record.clone(),
    };
    w.app.save(&mut paper).await.unwrap();
    RecordStore::insert(
        w.stores.as_ref(),
        &Record {
            id: record.clone(),
            kind: paper.kind,
            submission: paper.id.clone(),
            title: paper.title.clone(),
            authors: paper.authors.clone(),
            publication: Some(Box::new(paper.clone())),
            basis: AdmissionBasis::EscapeWitness,
            accepted_by: "synthetic-test".into(),
            accepted_at: chrono::Utc::now(),
        },
    )
    .await
    .unwrap();
    // Exposition finishes after the immutable acceptance snapshot was saved.
    let text = explanation(&paper);
    w.app
        .record_paper_explanation(&reviewer, &paper.id, text.clone())
        .await
        .unwrap();
    assert_eq!(
        w.app.paper(&record).await.unwrap().plain_language,
        Some(text.text.clone())
    );
    w.app
        .set_analysis_visibility(&author, &paper.id, Visibility::Private)
        .await
        .unwrap();
    assert!(w.app.paper(&record).await.unwrap().plain_language.is_none());
    w.app
        .set_analysis_visibility(&author, &paper.id, Visibility::Public)
        .await
        .unwrap();
    let mut revision = w
        .app
        .upload_version(
            &author,
            &paper.id,
            Upload {
                filename: "t.tex".into(),
                bytes: b"a new private version".to_vec(),
            },
            String::new(),
            Some("ai-review-request-2026-10-29".into()),
        )
        .await
        .unwrap();
    revision = w.review_version(&author, &revision, 2).await;
    let mut newer = explanation(&revision);
    newer.text.en = "PRIVATE SECOND VERSION".into();
    w.app
        .record_paper_explanation(&reviewer, &revision.id, newer)
        .await
        .unwrap();
    assert_eq!(
        w.app.paper(&record).await.unwrap().plain_language,
        Some(text.text)
    );
}

#[tokio::test]
async fn explanation_retry_does_not_restart_the_referee() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let admin = w.person("admin", &[]).await;
    let draft = w.submit(&author, "t", false).await;
    let paper = w.review_paper(&author, &draft).await;
    assert!(
        w.app
            .request_paper_explanation(&author, &paper.id)
            .await
            .is_err()
    );
    let new = w
        .app
        .request_paper_explanation(&admin, &paper.id)
        .await
        .unwrap();
    assert_eq!(new.current_version().unwrap().explanation_generation, 1);
    assert_eq!(new.claims_revision, paper.claims_revision);
    assert_eq!(new.status, paper.status);
    assert_eq!(new.reports, paper.reports);
    assert!(
        w.stores
            .queued()
            .await
            .contains(&(paper.id, JobKind::PaperExplanation))
    );
}
