use super::*;
use mathesis_review::knowledge::sha256;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

fn fixture() -> (Claim, KnowledgeQueryReceipt, serde_json::Value) {
    let claim = Claim {
        id: "C1".into(),
        kind: ClaimKind::Theorem,
        label: "Bound".into(),
        latex_label: None,
        statement: "For n > 0, the supplied bound holds.".into(),
        role: ClaimRole::Main,
        has_proof: true,
        section: None,
        depends_on: vec![],
        settles: None,
    };
    let subject = CoverageSubject {
        submission: "paper".into(),
        version: 1,
        claims_revision: 1,
        round: 1,
        claim: claim.id.clone(),
        source_digest: sha256(b"source"),
        claim_digest: sha256(&serde_json::to_vec(&claim).unwrap()),
    };
    let declaration = KnowledgeDeclarationRef {
        snapshot: sha256(b"snapshot"),
        module_path: "Bound.lean".into(),
        declaration_name_key: "bound".into(),
        statement_id: sha256(b"bound statement"),
        type_digest: sha256(b"bound type"),
    };
    let mut receipt = KnowledgeQueryReceipt {
        id: "actual-query-1".into(),
        protocol: KNOWLEDGE_QUERY_PROTOCOL.into(),
        subject,
        snapshot: Some(declaration.snapshot.clone()),
        index_revision: Some(sha256(b"index")),
        query: claim.statement.clone(),
        request_digest: String::new(),
        scopes: vec!["bounded source fixture".into()],
        state: KnowledgeQueryState::Complete,
        response: Some(BlobRef {
            id: "fixture-response".into(),
            bytes: 2,
            sha256: sha256(b"{}")[7..].into(),
        }),
        returned: vec![declaration.clone()],
        limits: vec![],
        retryable: false,
        started_at: chrono::Utc::now(),
        completed_at: chrono::Utc::now(),
    };
    receipt.request_digest = KnowledgeQueryReceipt::request_digest(
        &receipt.subject,
        &receipt.snapshot,
        &receipt.index_revision,
        &receipt.query,
        &receipt.scopes,
    );
    let raw = json!({"receipt_ids":[receipt.id],"relation":"covered","summary":"Substitute the positive integer n into the bound.","limits":[],
        "binding":{"premise_statement_ids":[declaration.statement_id],"relation":"instance",
        "parameters":[{"parameter":"k","expression":"n"}],
        "hypotheses":[{"hypothesis":"k > 0","argument":"The confirmed claim assumes n > 0."}],
        "argument":"Apply the supplied bound at k = n.","unresolved":[]}});
    (claim, receipt, raw)
}

fn visible(receipt: &KnowledgeQueryReceipt) -> BTreeMap<String, BTreeSet<String>> {
    [(
        receipt.id.clone(),
        receipt
            .returned
            .iter()
            .map(|d| d.statement_id.clone())
            .collect(),
    )]
    .into()
}

#[test]
fn mapping_resolves_exact_identities_and_preserves_joint_premises() {
    let (claim, mut receipt, mut raw) = fixture();
    let mut second = receipt.returned[0].clone();
    second.statement_id = sha256(b"second premise");
    second.declaration_name_key = "second".into();
    raw["binding"]["premise_statement_ids"]
        .as_array_mut()
        .unwrap()
        .push(json!(second.statement_id));
    receipt.returned.push(second);
    let result = comparison(
        raw,
        &claim,
        std::slice::from_ref(&receipt),
        &"auditor".into(),
        &visible(&receipt),
    )
    .unwrap();
    assert_eq!(result.binding.unwrap().premises, receipt.returned);
    assert_eq!(
        result.qualification,
        CoverageQualification::SourceAudited {
            auditor: "auditor".into(),
            audit_id: "referee-audit:paper:r1:v1:c1".into(),
        }
    );
}

#[test]
fn fabricated_ids_missing_hypotheses_and_unread_sources_never_establish_coverage() {
    let (claim, receipt, raw) = fixture();
    for (path, value) in [
        ("/receipt_ids/0", json!("invented")),
        (
            "/binding/premise_statement_ids/0",
            json!(sha256(b"not returned")),
        ),
        ("/binding/hypotheses/0/argument", json!("")),
        (
            "/binding/unresolved",
            json!(["positivity has not been proved"]),
        ),
        ("/binding/relation", json!("proof_use")),
    ] {
        let mut invalid = raw.clone();
        *invalid.pointer_mut(path).unwrap() = value;
        assert!(
            comparison(
                invalid,
                &claim,
                std::slice::from_ref(&receipt),
                &"auditor".into(),
                &visible(&receipt)
            )
            .is_err(),
            "{path}"
        );
    }
    assert!(
        comparison(
            raw.clone(),
            &claim,
            std::slice::from_ref(&receipt),
            &"auditor".into(),
            &BTreeMap::new()
        )
        .is_err()
    );
    let mut other = receipt.clone();
    other.subject.claim = "C2".into();
    assert!(comparison(raw, &claim, &[other], &"auditor".into(), &visible(&receipt)).is_err());
}

#[test]
fn unseen_results_and_unavailable_queries_remain_candidates() {
    let (claim, receipt, _) = fixture();
    let raw = json!({"receipt_ids":[receipt.id],"relation":"unknown","binding":null,"summary":"No conclusion.","limits":[]});
    let result = comparison(
        raw.clone(),
        &claim,
        std::slice::from_ref(&receipt),
        &"auditor".into(),
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.qualification, CoverageQualification::Candidate);
    let mut failed = receipt;
    failed.state = KnowledgeQueryState::Unavailable;
    failed.returned.clear();
    failed.response = None;
    failed.limits.push("Unavailable".into());
    let result = comparison(
        raw,
        &claim,
        std::slice::from_ref(&failed),
        &"auditor".into(),
        &visible(&failed),
    )
    .unwrap();
    assert_eq!(result.qualification, CoverageQualification::Candidate);
}

#[test]
fn independently_refuted_proof_cannot_retain_content_witnesses_or_known_true_status() {
    use mathesis_review::referee_prompts::{AuditedClaimOut, RefereeOut};
    let (claim, receipt, mut proposal) = fixture();
    proposal["relation"] = json!("refuted");
    proposal["binding"]["relation"] = json!("refutes");
    let method: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/content-methodology.json"
    )))
    .unwrap();
    let raw = AuditOut {
        verdict: "accept".into(),
        summary: "Read the source.".into(),
        claims: vec![AuditedClaimOut {
            claim: claim.id.to_string(),
            correctness: "correct".into(),
            comment: "The bound follows.".into(),
            shape: Some("content".into()),
            witnesses: vec!["A live bound".into()],
            methodology: Some(method),
            coverage: Some(proposal),
            ..Default::default()
        }],
        ..Default::default()
    };
    let input = AdvisorInput {
        letter_contract: 0,
        computations: None,
        knowledge: Some(
            json!({"receipts":[{"receipt_id":receipt.id,"result":{"hits":[{"source_excerpt":"theorem bound ...", "declarations":[{"identity":{"statement_id":receipt.returned[0].statement_id}}]}]}}]}),
        ),
        review_method: Some(REVIEW_METHOD.into()),
        confirmed_claims: Some(json!([claim])),
        followup: None,
        kind: "paper".into(),
        title: "T".into(),
        abstract_text: String::new(),
        authors: vec![],
        statements: super::super::mapping::confirmed_statements(std::slice::from_ref(&claim)),
        referee: RefereeOut::default(),
        audit: None,
        decision: None,
        text: None,
        source_dir: None,
        main_file: None,
    };
    let result = audit(raw, &input, &[claim], &[receipt], &"auditor".into()).unwrap();
    let reading = &result.claims[0];
    assert_eq!(reading.correctness, Correctness::Error);
    assert_eq!(reading.shape, Some(ProofShape::Undetermined));
    assert!(reading.witnesses.is_empty());
    let method = reading.methodology.as_ref().unwrap();
    assert_eq!(method.escape, ProofShape::Undetermined);
    assert!(method.witnesses.is_empty());
}
