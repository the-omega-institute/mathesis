use super::*;
use mathesis_review::knowledge::sha256;

fn fixture() -> (KnowledgeQueryReceipt, SourceQueryResult) {
    let snapshot = sha256(b"publication");
    let index = sha256(b"index");
    let source = "theorem sample : ∀ n : Nat, n = n := by intro n; rfl\n".repeat(400);
    let source_hash = sha256(source.as_bytes());
    let hits = (0..6).map(|h| {
        let module = format!("Example/Module{h}.lean");
        let count = match h { 0 => 57, 1 => 43, _ => 0 };
        let declarations = (0..count).map(|d| json!({
            "identity":{"snapshot":snapshot,"module_path":module,"name_key":format!("sample{d}"),
                "statement_id":sha256(format!("statement {h} {d}").as_bytes()),"type_sha256":sha256(b"type")},
            "name":format!("sample{d}"),"kind":"theorem","frozen_node_id":sha256(b"node"),
            "freeze_status":"frozen","source_sha256":source_hash,"axioms":[],
            "module_imports":["Mathlib"],"prerequisite_frozen_node_ids":[sha256(b"prerequisite")]
        })).collect::<Vec<_>>();
        json!({"module_path":module,"source_sha256":source_hash,"source_excerpt":source,
            "excerpt_truncated":false,"declarations":declarations,"declarations_truncated":false})
    }).collect::<Vec<_>>();
    let result: SourceQueryResult = serde_json::from_value(json!({
        "ranking_protocol":"mathesis.source-ranker.identifier-paths.v1","snapshot":snapshot,
        "index_revision":index,"searched_modules":8673,"omitted_modules":6,
        "matching_modules":8000,"result_truncated":true,"hits":hits,"limits":["Bounded candidates only."]
    })).unwrap();
    let receipt: KnowledgeQueryReceipt = serde_json::from_value(json!({
        "id":"query-1","protocol":"mathesis.artifact-query.v1",
        "subject":{"submission":"paper","version":1,"claims_revision":1,"round":1,"claim":"C1",
            "source_digest":sha256(b"manuscript"),"claim_digest":sha256(b"claim")},
        "snapshot":snapshot,"index_revision":index,"request_digest":sha256(b"query"),"query":"sample",
        "scopes":["Exact saved source query"],"state":"complete",
        "response":{"id":"saved-response","bytes":123456,"sha256":"a".repeat(64)},
        "returned":[],"limits":["Six source modules omitted from the index."],"retryable":false,
        "started_at":"2026-10-27T00:00:00Z","completed_at":"2026-10-27T00:01:00Z"
    })).unwrap();
    (receipt, result)
}

#[test]
fn dense_catalogues_retain_readable_sources_inside_the_complete_entry_budget() {
    let (receipt, original) = fixture();
    let bytes_before = serde_json::to_vec(&original).unwrap();
    // Production: 32 statements, two completed searches per statement.
    let budget = (800_000 - 1024) / 64;
    let entry = project(&receipt, Some(original.clone()), budget).unwrap();
    assert!(fits(&entry, budget));
    assert_eq!(entry["prompt_evidence_limited"], true);
    assert_eq!(entry["result"]["result_truncated"], true);
    assert_eq!(entry["response_identity"], json!(receipt.response));
    assert_eq!(entry["snapshot"], json!(receipt.snapshot));
    let hits = entry["result"]["hits"].as_array().unwrap();
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| !h["declarations"].as_array().unwrap().is_empty())
    );
    for hit in hits {
        let saved = original
            .hits
            .iter()
            .find(|h| h.module_path == hit["module_path"].as_str().unwrap())
            .unwrap();
        let excerpt = hit["source_excerpt"].as_str().unwrap();
        assert!(excerpt.len() >= 256 && saved.source_excerpt.starts_with(excerpt));
        assert_eq!(hit["source_sha256"], saved.source_sha256);
        assert_eq!(hit["excerpt_truncated"], true);
        for declaration in hit["declarations"].as_array().unwrap() {
            assert!(saved.declarations.iter().any(|d| json!(d) == *declaration));
        }
        if hit["declarations"].as_array().unwrap().len() < saved.declarations.len() {
            assert_eq!(hit["declarations_truncated"], true);
        }
    }
    assert_eq!(bytes_before, serde_json::to_vec(&original).unwrap());
    assert_eq!(entry, project(&receipt, Some(original), budget).unwrap());
}

#[test]
fn small_results_are_not_shortened_and_empty_searches_stay_empty() {
    let (receipt, mut result) = fixture();
    result.hits.truncate(1);
    result.hits[0].declarations.truncate(1);
    result.hits[0].source_excerpt = "theorem sample : True := True.intro".into();
    let entry = project(&receipt, Some(result.clone()), 20_000).unwrap();
    assert_eq!(entry["result"], json!(result));
    assert_eq!(entry["prompt_evidence_limited"], false);
    result.hits.clear();
    result.matching_modules = 0;
    result.result_truncated = false;
    let entry = project(&receipt, Some(result), 20_000).unwrap();
    assert_eq!(entry["result"]["hits"], json!([]));
    assert_eq!(entry["prompt_evidence_limited"], false);
}

#[test]
fn missing_responses_cannot_become_readable_results_when_metadata_is_trimmed() {
    let (mut receipt, _) = fixture();
    receipt.scopes = vec!["large scope ".repeat(1000)];
    receipt.response = None;
    let entry = project(&receipt, None, 2000).unwrap();
    assert!(fits(&entry, 2000));
    assert!(entry["result"].is_null());
    assert!(entry["response_identity"].is_null());
    assert_eq!(entry["prompt_evidence_limited"], true);
    assert!(project(&receipt, None, 10).is_err());
}

#[test]
fn omitted_hits_never_turn_into_a_successful_empty_search() {
    let (receipt, result) = fixture();
    for budget in [900, 1500, 2500, 6000] {
        if let Ok(entry) = project(&receipt, Some(result.clone()), budget) {
            assert!(fits(&entry, budget));
            assert_eq!(entry["prompt_evidence_limited"], true);
            if let Some(hits) = entry["result"]["hits"].as_array() {
                assert!(!hits.is_empty());
                assert_eq!(entry["result"]["result_truncated"], true);
            } else {
                assert!(entry["result"].is_null());
            }
        }
    }
}
