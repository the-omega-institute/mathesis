use super::mapping;
use mathesis_core::model::{
    Claim, ClaimKind, ClaimRole, Correctness, EvidenceStatus, MethodologyReading, OpenProblemRef,
    ProofShape, Source, SourceKind,
};
use mathesis_review::{
    advisor::AdvisorInput,
    referee_prompts::{AuditOut, AuditedClaimOut, RefereeOut},
};

fn claim() -> Claim {
    Claim {
        id: "C1".into(),
        kind: ClaimKind::Theorem,
        label: "Theorem 1".into(),
        latex_label: None,
        statement: "The gap estimate".into(),
        role: ClaimRole::Main,
        has_proof: true,
        section: None,
        depends_on: vec![],
        settles: None,
    }
}

fn method() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/content-methodology.json"
    )))
    .unwrap()
}

fn checked(
    statement: &Claim,
    methodology: Option<serde_json::Value>,
    current: bool,
) -> mathesis_core::model::AuditedClaim {
    let input = AdvisorInput {
        letter_contract: 0,
        computations: None,
        knowledge: None,
        review_method: current.then(|| mathesis_core::model::REVIEW_METHOD.into()),
        confirmed_claims: current.then(|| serde_json::json!([statement])),
        followup: None,
        kind: "paper".into(),
        title: "T".into(),
        abstract_text: String::new(),
        authors: vec![],
        statements: mapping::confirmed_statements(std::slice::from_ref(statement)),
        referee: RefereeOut::default(),
        audit: None,
        decision: None,
        text: None,
        source_dir: None,
        main_file: None,
    };
    mapping::audit(
        AuditOut {
            verdict: "accept".into(),
            summary: "The source proof was read.".into(),
            claims: vec![AuditedClaimOut {
                claim: "C1".into(),
                correctness: "correct".into(),
                comment: "The gap estimate follows from the construction.".into(),
                shape: Some("content".into()),
                witnesses: vec!["The gap estimate".into()],
                methodology,
                ..Default::default()
            }],
            ..Default::default()
        },
        &input,
        std::slice::from_ref(statement),
    )
    .unwrap()
    .claims
    .remove(0)
}

#[test]
fn missing_new_evidence_preserves_correctness_but_never_fabricates_binding_or_novelty() {
    let reading = checked(&claim(), None, true);
    assert_eq!(reading.correctness, Correctness::Correct);
    assert_eq!(reading.shape, Some(ProofShape::Undetermined));
    assert!(reading.witnesses.is_empty());
    assert_eq!(
        reading.methodology.unwrap().literature,
        EvidenceStatus::Insufficient
    );
    let legacy = checked(&claim(), None, false);
    assert_eq!(legacy.shape, Some(ProofShape::Content));
    assert!(legacy.methodology.is_none());
}

#[test]
fn live_conclusion_is_preserved_and_unusable_evidence_cannot_grant_content() {
    let reading = checked(&claim(), Some(method()), true);
    assert_eq!(reading.shape, Some(ProofShape::Content));
    assert!(reading.methodology.unwrap().supports_content(false));
    for key in ["location", "non_binding_step", "live_use"] {
        let mut raw = method();
        raw["witnesses"][0][key] = "".into();
        assert_eq!(
            checked(&claim(), Some(raw), true).shape,
            Some(ProofShape::Undetermined)
        );
    }
    let mut raw = method();
    raw["baseline"][0]["source"] = "Unopened attribution".into();
    assert_eq!(
        checked(&claim(), Some(raw), true).shape,
        Some(ProofShape::Undetermined)
    );
    let mut raw = method();
    raw["literature"] = "unrecognized".into();
    assert_eq!(
        checked(&claim(), Some(raw), true).shape,
        Some(ProofShape::Undetermined)
    );
}

#[test]
fn settlement_citation_must_match_the_confirmed_problem_source() {
    let mut statement = claim();
    statement.settles = Some(OpenProblemRef {
        name: "Problem".into(),
        source: Source {
            kind: SourceKind::Arxiv,
            locator: "2401.01234".into(),
            year: None,
        },
    });
    let mut raw = method();
    raw["settlement"] = serde_json::json!({"status":"supported", "problem":statement.settles, "statement":"Original assertion", "assumptions":"Same hypotheses", "prior_status":"Open in searched sources", "resolution":"Proof", "matching_argument":"All quantifiers and premises match", "opened_source":"Problem [opened: arXiv:2401.99999]"});
    let reading = checked(&statement, Some(raw.clone()), true)
        .methodology
        .unwrap();
    assert_eq!(
        reading.settlement.unwrap().status,
        EvidenceStatus::Insufficient
    );
    raw["settlement"]["opened_source"] = "Problem [opened: arXiv:2401.01234]".into();
    assert!(
        checked(&statement, Some(raw), true)
            .methodology
            .unwrap()
            .supports_settlement(statement.settles.as_ref().unwrap())
    );
}

#[test]
fn conjecture_content_has_no_live_proof_witness_and_requires_supported_search() {
    let mut reading: MethodologyReading = serde_json::from_value(method()).unwrap();
    assert!(reading.validate(true).is_err());
    reading.witnesses.clear();
    assert!(reading.supports_content(true));
    reading.literature = EvidenceStatus::Insufficient;
    assert!(!reading.supports_content(true));
}

#[test]
fn model_cannot_assert_the_system_knowledge_admission_gate() {
    let mut raw = method();
    raw["knowledge"] = serde_json::json!({"receipt_ids":["fabricated"],"complete":true,"audited_relation":"unknown"});
    assert!(
        checked(&claim(), Some(raw), true)
            .methodology
            .unwrap()
            .knowledge
            .is_none()
    );
}
