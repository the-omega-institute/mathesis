use super::*;

async fn prepare(w: &World, author: &Caller, purpose: SubmissionPurpose) -> Submission {
    w.app
        .prepare_submission(
            author,
            PrepareSubmission {
                kind: SubmissionKind::Paper,
                purpose,
                processing_consent: Some(AI_PROCESSING_CONSENT_VERSION.into()),
            },
            Upload {
                filename: "t.tex".into(),
                bytes: b"manuscript".to_vec(),
            },
        )
        .await
        .unwrap()
}

fn submission_request(s: &Submission) -> SubmitPrepared {
    SubmitPrepared {
        version: s.current_version().unwrap().number,
        source_sha256: s.current_version().unwrap().archive.sha256.clone(),
        claims_revision: s.claims_revision,
        metadata: NewPaper {
            title: Some("Author-corrected title".into()),
            abstract_text: Some("Author-corrected abstract".into()),
            authors: s.authors.clone(),
            kind: s.kind,
            purpose: s.purpose,
            processing_consent: Some(AI_PROCESSING_CONSENT_VERSION.into()),
            review_scope: String::new(),
            make_public_after_acceptance: true,
            typed_conjecture: None,
            open_to_contributors: false,
            msc: vec![],
            doi: None,
            ai_disclosure: AiDisclosure {
                level: AiUse::Assisted,
                statement: "Proofreading.".into(),
            },
        },
        claims: s
            .extracted
            .iter()
            .map(|c| ClaimConfirmation {
                id: c.id.clone(),
                kind: c.kind,
                role: c.role,
                excluded: false,
                depends_on: vec![],
                depends_on_conjectures: vec![],
                settles: None,
            })
            .collect(),
    }
}

#[tokio::test]
async fn preparation_compiles_privately_then_submits_once_with_corrected_metadata() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let s = prepare(&w, &author, SubmissionPurpose::Publication).await;
    assert_eq!(s.title, "Gaps in zero runs");
    assert_eq!(s.abstract_text, "We bound the gaps.");
    assert_eq!(s.status, SubmissionStatus::Draft);
    assert_eq!(s.analysis_visibility, Visibility::Private);
    assert!(s.claims.is_empty());
    let input = submission_request(&s);
    assert!(
        w.app
            .confirm_claims(&author, &s.id, input.claims.clone())
            .await
            .is_err()
    );
    assert!(
        w.app
            .submit_prepared(&author, &s.id, input.clone())
            .await
            .is_err()
    );
    let bot = w.person("bot", &[Role::Reviewer]).await;
    w.app
        .record_compilation(&bot, &s.id, 1, Ok(b"%PDF".to_vec()))
        .await
        .unwrap();
    let submitted = w
        .app
        .submit_prepared(&author, &s.id, input.clone())
        .await
        .unwrap();
    assert_eq!(submitted.status, SubmissionStatus::InReview);
    assert_eq!(submitted.title, "Author-corrected title");
    assert_eq!(submitted.abstract_text, "Author-corrected abstract");
    assert_eq!(submitted.ai_disclosure.level, AiUse::Assisted);
    let replay = w
        .app
        .submit_prepared(&author, &s.id, input.clone())
        .await
        .unwrap();
    assert_eq!(replay, submitted);
    let mut changed = input;
    changed.metadata.title = Some("Unexpected overwrite".into());
    assert!(matches!(
        w.app.submit_prepared(&author, &s.id, changed).await,
        Err(CoreError::Conflict(_))
    ));
}

#[tokio::test]
async fn submission_requires_owner_current_source_and_valid_author_metadata() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let stranger = w.person("stranger", &[]).await;
    let s = prepare(&w, &author, SubmissionPurpose::ReviewOnly).await;
    let bot = w.person("bot", &[Role::Reviewer]).await;
    w.app
        .record_compilation(&bot, &s.id, 1, Ok(b"%PDF".to_vec()))
        .await
        .unwrap();
    let input = submission_request(&s);
    assert!(matches!(
        w.app.submit_prepared(&stranger, &s.id, input.clone()).await,
        Err(CoreError::NotFound { .. })
    ));
    let mut bad = input.clone();
    bad.source_sha256 = "0".repeat(64);
    assert!(w.app.submit_prepared(&author, &s.id, bad).await.is_err());
    let mut bad = input.clone();
    bad.metadata.authors.clear();
    assert!(w.app.submit_prepared(&author, &s.id, bad).await.is_err());
    let mut bad = input.clone();
    bad.metadata.kind = SubmissionKind::Note;
    assert!(w.app.submit_prepared(&author, &s.id, bad).await.is_err());
    let stored = w.app.submission(&author, &s.id).await.unwrap();
    assert_eq!(stored.title, s.title);
    assert!(stored.intake.unwrap().confirmed_request.is_none());
    let submitted = w.app.submit_prepared(&author, &s.id, input).await.unwrap();
    assert_eq!(submitted.analysis_visibility, Visibility::Private);
    assert!(submitted.decision.is_none());
}

#[tokio::test]
async fn replacing_prepared_files_invalidates_the_old_submission_request() {
    let w = World::new().await;
    let author = w.person("author", &[]).await;
    let s = prepare(&w, &author, SubmissionPurpose::Publication).await;
    let input = submission_request(&s);
    let revised = w
        .app
        .upload_version(
            &author,
            &s.id,
            Upload {
                filename: "tt.tex".into(),
                bytes: b"revised manuscript".to_vec(),
            },
            String::new(),
            Some(AI_PROCESSING_CONSENT_VERSION.into()),
        )
        .await
        .unwrap();
    assert_eq!(revised.extracted.len(), 2);
    assert!(revised.intake.as_ref().unwrap().confirmed_request.is_none());
    let bot = w.person("bot", &[Role::Reviewer]).await;
    w.app
        .record_compilation(&bot, &s.id, 2, Ok(b"%PDF".to_vec()))
        .await
        .unwrap();
    assert!(w.app.submit_prepared(&author, &s.id, input).await.is_err());
    assert_eq!(
        w.app.submission(&author, &s.id).await.unwrap().status,
        SubmissionStatus::Draft
    );
}

#[tokio::test]
async fn author_chooses_purpose_at_final_submission_then_cannot_change_it_by_replay() {
    for (initial, final_purpose, visibility) in [
        (
            SubmissionPurpose::Publication,
            SubmissionPurpose::ReviewOnly,
            Visibility::Private,
        ),
        (
            SubmissionPurpose::ReviewOnly,
            SubmissionPurpose::Publication,
            Visibility::Public,
        ),
    ] {
        let w = World::new().await;
        let author = w.person("author", &[]).await;
        let s = prepare(&w, &author, initial).await;
        let bot = w.person("bot", &[Role::Reviewer]).await;
        w.app
            .record_compilation(&bot, &s.id, 1, Ok(b"%PDF".to_vec()))
            .await
            .unwrap();
        let mut input = submission_request(&s);
        input.metadata.purpose = final_purpose;
        // Even a conflicting public flag must not expose a review-only request.
        input.metadata.make_public_after_acceptance = true;
        let submitted = w
            .app
            .submit_prepared(&author, &s.id, input.clone())
            .await
            .unwrap();
        assert_eq!(submitted.purpose, final_purpose);
        assert_eq!(submitted.analysis_visibility, visibility);
        assert_eq!(submitted.status, SubmissionStatus::InReview);
        assert_eq!(
            submitted.decision.is_none(),
            final_purpose == SubmissionPurpose::ReviewOnly
        );
        let replay = w
            .app
            .submit_prepared(&author, &s.id, input.clone())
            .await
            .unwrap();
        assert_eq!(replay, submitted);
        input.metadata.purpose = initial;
        assert!(matches!(
            w.app.submit_prepared(&author, &s.id, input).await,
            Err(CoreError::Conflict(_))
        ));
        assert_eq!(w.app.submission(&author, &s.id).await.unwrap(), submitted);
    }
}
