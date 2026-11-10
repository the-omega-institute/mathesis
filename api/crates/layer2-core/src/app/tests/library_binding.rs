//! Prepared libraries of the verifier: admission rules, receipts and projections.
//! Fixtures exercise bookkeeping only; no mathematical validation is claimed.
use super::*;
use crate::ports::Verifier;
use std::collections::BTreeMap;

fn library() -> PreparedLibrary {
    PreparedLibrary {
        name: "trureturing".into(),
        revision: "b".repeat(40),
        roots: vec!["D5".into(), "Trureturing".into()],
        url: Some("https://github.com/example/library".into()),
    }
}
fn report() -> LibraryUse {
    LibraryUse {
        libraries: vec![LibraryPin {
            name: "trureturing".into(),
            revision: "b".repeat(40),
            url: Some("https://github.com/example/library".into()),
        }],
        declarations: vec![LibraryDeclaration {
            library: "trureturing".into(),
            module: "D5.S3.Result".into(),
            name: "D5.S3.Result.result".into(),
        }],
        total: 1,
        complete: true,
    }
}
/// `libraries: None` is a verifier whose environment cannot be read.
struct LibraryVerifier {
    libraries: Option<Vec<PreparedLibrary>>,
    report: Option<LibraryUse>,
}
#[async_trait::async_trait]
impl Verifier for LibraryVerifier {
    async fn libraries(&self) -> CoreResult<Vec<PreparedLibrary>> {
        self.libraries
            .clone()
            .ok_or_else(|| CoreError::Unavailable("verifier environment is unavailable".into()))
    }
    async fn verify(&self, r: &VerificationRequest) -> CoreResult<VerificationReceipt> {
        let imports_library = [&r.solution]
            .into_iter()
            .chain(r.modules.values())
            .any(|text| text.lines().any(|line| line.starts_with("import D5.")));
        Ok(VerificationReceipt {
            used_constants: vec![],
            library_use: self.report.clone().filter(|_| imports_library),
            verdict: Verdict::Proved,
            reason: "fixture boundary".into(),
            target_digest: r.target_digest.clone(),
            solution_digest: lean_solution_digest(&r.solution, &r.modules),
            toolchain: r.toolchain.clone(),
            axioms: vec!["propext".into()],
            checked_at: chrono::Utc::now(),
            duration: 1,
        })
    }
}
async fn world(libraries: Option<Vec<PreparedLibrary>>, report: Option<LibraryUse>) -> World {
    let stores = Arc::new(MemoryStores::default());
    let mut ports = stores.ports(Arc::new(SystemClock), Arc::new(FakeReader));
    ports.verifier = Arc::new(LibraryVerifier { libraries, report });
    World {
        app: App::with_attempt_limit(
            ports,
            Policy::default(),
            BTreeSet::from([PersonId::from("admin")]),
            20,
        ),
        stores,
    }
}
const FROM_LIBRARY: &str = "import Target\nimport D5.S3.Result\ntheorem wishpool_solution : wishpool_target_prop := D5.S3.Result.result\n";
const NEW_LEAN: &str =
    "import Target\ntheorem wishpool_solution : wishpool_target_prop := True.intro\n";
fn input(solution: &str, modules: &[(&str, &str)]) -> crate::app::solving::AttemptInput {
    crate::app::solving::AttemptInput {
        intermediate: None,
        reuses: vec![],
        expected_target: None,
        modules: modules
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect(),
        solution: solution.into(),
        as_agent: None,
        note: String::new(),
        attribution: None,
    }
}
fn directory(status: Option<&str>, basis: Option<&str>) -> crate::app::FormalizationQuery {
    crate::app::FormalizationQuery {
        status: status.map(str::to_owned),
        basis: basis.map(str::to_owned),
        ..Default::default()
    }
}

#[tokio::test]
async fn library_receipts_are_saved_and_shown_as_proved_from_library_declarations() {
    let w = world(Some(vec![library()]), Some(report())).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let other = w.person("other", &[]).await;
    let (_, record) = super::solving::target(&w, &author, &reviewer).await;
    let claim: ClaimId = "C1".into();

    // Before any verified proof the row carries no basis and matches no basis filter.
    let open = w
        .app
        .formalization_opportunities(directory(None, None))
        .await
        .unwrap();
    assert_eq!(open.items.len(), 1);
    assert!(open.items[0].proof_basis.is_none());
    for basis in ["library_declarations", "new_lean"] {
        assert!(
            w.app
                .formalization_opportunities(directory(None, Some(basis)))
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
    assert!(matches!(
        w.app
            .formalization_opportunities(directory(None, Some("equivalent")))
            .await,
        Err(CoreError::Invalid(_))
    ));

    let first = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &claim,
            FROM_LIBRARY.into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    assert!(first.proof_basis.is_none());
    w.app.verify_attempt(&reviewer, &first.id).await.unwrap();
    let later = w
        .app
        .submit_attempt(
            &other,
            &record,
            &claim,
            NEW_LEAN.into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    w.app.verify_attempt(&reviewer, &later.id).await.unwrap();

    let saved = w.app.attempt(None, &first.id).await.unwrap();
    assert_eq!(saved.receipt.as_ref().unwrap().library_use, Some(report()));
    let basis = saved.proof_basis.unwrap();
    assert_eq!(basis.kind(), "library_declarations");
    let ProofBasis::LibraryDeclarations {
        libraries,
        declarations,
        total,
        complete,
    } = &basis
    else {
        unreachable!()
    };
    assert_eq!(libraries[0].revision, "b".repeat(40));
    assert_eq!((declarations.len(), *total, *complete), (1, 1, true));
    assert_eq!(declarations[0].module, "D5.S3.Result");
    assert_eq!(
        declarations[0].source.as_deref(),
        Some(
            format!(
                "https://github.com/example/library/blob/{}/D5/S3/Result.lean",
                "b".repeat(40)
            )
            .as_str()
        )
    );
    assert_eq!(
        w.app
            .attempt(None, &later.id)
            .await
            .unwrap()
            .proof_basis
            .unwrap(),
        ProofBasis::NewLean
    );

    let public = w.app.public_attempts(&record, &claim).await.unwrap();
    assert_eq!(
        public
            .iter()
            .map(|a| (a.id.as_str(), a.proof_basis.kind(), a.also_verified))
            .collect::<Vec<_>>(),
        [
            (first.id.as_str(), "library_declarations", false),
            (later.id.as_str(), "new_lean", true)
        ]
    );
    assert_eq!(
        w.app
            .conjecture(&record, &claim)
            .await
            .unwrap()
            .verified_attempts[0]
            .proof_basis,
        basis
    );

    // The winning receipt decides the row; the two verified kinds are filterable.
    for (status, wanted, found) in [
        (None, None, true),
        (Some("verified"), None, true),
        (Some("verified"), Some("library_declarations"), true),
        (None, Some("library_declarations"), true),
        (Some("verified"), Some("new_lean"), false),
        (Some("unverified"), Some("library_declarations"), false),
    ] {
        let rows = w
            .app
            .formalization_opportunities(directory(status, wanted))
            .await
            .unwrap()
            .items;
        assert_eq!(rows.len(), usize::from(found), "{status:?} {wanted:?}");
        if found {
            assert_eq!(rows[0].status, "verified");
            assert_eq!(rows[0].proof_basis.as_ref(), Some(&basis));
            assert_eq!(
                rows[0].receipt.as_ref().unwrap().library_use,
                Some(report())
            );
        }
    }

    let bundle = w
        .app
        .verification_bundle(&record, &claim, &first.id)
        .await
        .unwrap();
    assert_eq!(bundle.attempt.proof_basis, basis);
    assert_eq!(bundle.attempt.receipt.library_use, Some(report()));
    assert!(
        bundle
            .instructions
            .contains("attempt.receipt.library_use.libraries")
    );
    let plain = w
        .app
        .verification_bundle(&record, &claim, &later.id)
        .await
        .unwrap();
    assert_eq!(plain.attempt.proof_basis, ProofBasis::NewLean);
    assert!(!plain.instructions.contains("library"));
    let value = serde_json::to_value(&plain).unwrap();
    assert_eq!(
        value["attempt"]["proof_basis"],
        serde_json::json!({"kind":"new_lean"})
    );
    assert!(value["attempt"]["receipt"].get("library_use").is_none());
}

#[tokio::test]
async fn a_malformed_library_report_is_never_saved_as_a_receipt() {
    let mut broken = report();
    broken.declarations[0].library = "undeclared".into();
    let w = world(Some(vec![library()]), Some(broken)).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (_, record) = super::solving::target(&w, &author, &reviewer).await;
    let attempt = w
        .app
        .submit_attempt(
            &solver,
            &record,
            &"C1".into(),
            FROM_LIBRARY.into(),
            None,
            String::new(),
        )
        .await
        .unwrap();
    assert!(matches!(
        w.app.verify_attempt(&reviewer, &attempt.id).await,
        Err(CoreError::Unavailable(_))
    ));
    let saved = w.app.attempt(Some(&solver), &attempt.id).await.unwrap();
    assert!(saved.receipt.is_none() && saved.proof_basis.is_none());
}

#[tokio::test]
async fn admission_refuses_module_names_below_declared_roots_and_is_unchanged_otherwise() {
    let below = [("D5.Mine", "theorem mine : True := True.intro")];
    let importing = [(
        "Research.Helper",
        "import Mathlib\nimport D5.S3.Result\ntheorem helper : True := True.intro",
    )];
    let solution = "import Target\nimport Research.Helper\ntheorem wishpool_solution : wishpool_target_prop := helper\n";

    let w = world(Some(vec![library()]), None).await;
    let author = w.person("author", &[]).await;
    let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
    let solver = w.person("solver", &[]).await;
    let (_, record) = super::solving::target(&w, &author, &reviewer).await;
    let claim: ClaimId = "C1".into();
    for name in ["D5.Mine", "d5.Mine", "Trureturing", "TRURETURING.X"] {
        let Err(CoreError::Invalid(reason)) = w
            .app
            .submit_attributed_attempt(
                &solver,
                &record,
                &claim,
                input(solution, &[(name, below[0].1)]),
            )
            .await
        else {
            panic!("{name} must be refused");
        };
        assert!(
            reason.contains(name) && reason.contains("prepared library trureturing"),
            "{reason}"
        );
    }
    let admitted = w
        .app
        .submit_attributed_attempt(&solver, &record, &claim, input(solution, &importing))
        .await
        .unwrap();
    assert_eq!(admitted.state, "queued");
    // The historical refusals keep their exact message beside a declared library.
    let historical = "invalid input: supporting modules need distinct dotted Lean module names, nonempty sources, and cannot replace trusted modules";
    assert_eq!(
        w.app
            .submit_attributed_attempt(
                &solver,
                &record,
                &claim,
                input(solution, &[("Mathlib.User", "x")])
            )
            .await
            .unwrap_err()
            .to_string(),
        historical
    );

    // No declared library, an unreadable environment or an invalid declaration:
    // exactly the historical rule.
    let invalid = PreparedLibrary {
        revision: "main".into(),
        ..library()
    };
    for libraries in [Some(vec![]), None, Some(vec![invalid])] {
        let w = world(libraries, None).await;
        let author = w.person("author", &[]).await;
        let reviewer = w.person("reviewer", &[Role::Reviewer]).await;
        let solver = w.person("solver", &[]).await;
        let (_, record) = super::solving::target(&w, &author, &reviewer).await;
        assert_eq!(
            w.app
                .submit_attributed_attempt(&solver, &record, &claim, input(solution, &below))
                .await
                .unwrap()
                .state,
            "queued"
        );
        assert_eq!(
            w.app
                .submit_attributed_attempt(
                    &solver,
                    &record,
                    &claim,
                    input(solution, &[("Mathlib.User", "x")])
                )
                .await
                .unwrap_err()
                .to_string(),
            historical
        );
    }
}

#[tokio::test]
async fn preflight_resolves_library_imports_only_for_a_verifier_that_declares_them() {
    let solution = "import Target\nimport D5.S3.Result\nimport Research.Helper\ntheorem wishpool_solution : wishpool_target_prop := helper\n";
    let project = |modules: &[(&str, &str)]| NewLeanDeposit {
        modules: modules
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect(),
        project_files: BTreeMap::new(),
        formalizers: vec![],
        source_revision: None,
        title: "Project".into(),
        authors: vec!["Research author".into()],
        statement: "True".into(),
        source_reference: Source {
            kind: SourceKind::Url,
            locator: "https://example.org/source".into(),
            year: None,
        },
        toolchain: "leanprover/lean4:v4.35.0".into(),
        license: "Apache-2.0".into(),
        solution: solution.into(),
    };
    let helper = (
        "Research.Helper",
        "import Trureturing\nimport D5x.Other\ntheorem helper : True := True.intro",
    );
    let body = project(&[helper]);
    let historical = lean_project_preflight(
        &body.solution,
        &body.modules,
        &body.project_files,
        &body.toolchain,
    );
    assert_eq!(
        historical.errors,
        [
            "Research.Helper: missing import Trureturing; include its source module. Custom dependencies are not fetched.",
            "Research.Helper: missing import D5x.Other; include its source module. Custom dependencies are not fetched.",
            "Solution: missing import D5.S3.Result; include its source module. Custom dependencies are not fetched.",
        ]
    );
    for libraries in [Some(vec![]), None] {
        let w = world(libraries, None).await;
        let report = w.app.lean_preflight(&body).await.unwrap();
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::to_value(&historical).unwrap()
        );
    }

    let w = world(Some(vec![library()]), None).await;
    let report = w.app.lean_preflight(&body).await.unwrap();
    // Only the import outside every declared root is still missing.
    assert_eq!(
        report.errors,
        [
            "Research.Helper: missing import D5x.Other; include its source module. Custom dependencies are not fetched."
        ]
    );
    assert!(
        report.warnings.iter().any(|warning| {
            warning.contains("trureturing") && warning.contains(&"b".repeat(40))
        })
    );
    let shadowing = w
        .app
        .lean_preflight(&project(&[
            ("Research.Helper", "theorem helper : True := True.intro"),
            ("D5.S3.Result", "theorem result : True := True.intro"),
        ]))
        .await
        .unwrap();
    assert!(
        shadowing
            .errors
            .iter()
            .any(|error| error.contains("D5.S3.Result")
                && error.contains("prepared library trureturing")),
        "{:?}",
        shadowing.errors
    );
}
