//! Contract 4 budgets the complete receipt entry, including declaration metadata.
//! The stored response stays exact; this is only the model's bounded reading view.
use super::{Failure, prefix};
use mathesis_core::model::KnowledgeQueryReceipt;
use mathesis_review::knowledge::index::SourceQueryResult;
use serde_json::{Value, json};

fn fits(value: &Value, maximum: usize) -> bool {
    serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= maximum)
}

pub(super) fn project(
    receipt: &KnowledgeQueryReceipt,
    payload: Option<SourceQueryResult>,
    maximum: usize,
) -> Result<Value, Failure> {
    let mut entry = json!({"receipt_id":receipt.id,"claim":receipt.subject.claim,
        "snapshot":receipt.snapshot,"index_revision":receipt.index_revision,"state":receipt.state,
        "response_identity":receipt.response,"scope":receipt.scopes,"limits":receipt.limits,
        "prompt_evidence_limited":false,"result":payload});
    if fits(&entry, maximum) {
        return Ok(entry);
    }
    entry["prompt_evidence_limited"] = json!(true);
    if let Some(hits) = entry
        .get_mut("result")
        .and_then(|r| r.get_mut("hits"))
        .and_then(Value::as_array_mut)
    {
        let per_hit = maximum / hits.len().max(1) / 2;
        for hit in hits {
            if let Some(source) = hit["source_excerpt"].as_str() {
                let excerpt = prefix(source, per_hit).to_owned();
                if excerpt.len() < source.len() {
                    hit["source_excerpt"] = json!(excerpt);
                    hit["excerpt_truncated"] = json!(true);
                }
            }
        }
        entry["result"]["result_truncated"] = json!(true);
    }
    while !fits(&entry, maximum) {
        // Declaration catalogues can exceed the entire per-receipt budget even
        // after source trimming. Retain candidates from each ranked module first.
        let hits = entry
            .get_mut("result")
            .and_then(|r| r.get_mut("hits"))
            .and_then(Value::as_array_mut);
        if let Some(hits) = hits {
            if let Some(hit) = hits
                .iter_mut()
                .filter(|h| h["declarations"].as_array().is_some_and(|d| d.len() > 1))
                .max_by_key(|h| h["declarations"].as_array().map_or(0, Vec::len))
            {
                hit["declarations"].as_array_mut().unwrap().pop();
                hit["declarations_truncated"] = json!(true);
                continue;
            }
            if let Some(hit) = hits
                .iter_mut()
                .filter(|h| h["source_excerpt"].as_str().is_some_and(|s| s.len() > 256))
                .max_by_key(|h| h["source_excerpt"].as_str().map_or(0, str::len))
            {
                let source = hit["source_excerpt"].as_str().unwrap();
                let excerpt = prefix(source, (source.len() / 2).max(256)).to_owned();
                hit["source_excerpt"] = json!(excerpt);
                hit["excerpt_truncated"] = json!(true);
                continue;
            }
            if hits.len() > 1 {
                hits.pop();
                continue;
            }
            if let Some(hit) = hits.first_mut()
                && hit["declarations"]
                    .as_array()
                    .is_some_and(|d| !d.is_empty())
            {
                hit["declarations"] = json!([]);
                hit["declarations_truncated"] = json!(true);
                continue;
            }
        }
        // Oversized receipt annotations must not consume the last readable
        // excerpt. Their exact text remains available in the saved receipt.
        if entry["scope"] != json!([]) {
            entry["scope"] = json!([]);
            entry["limits"] = json!([
                "Detailed scope and limits omitted from this prompt; consult the saved receipt."
            ]);
            continue;
        }
        if !entry["result"].is_null() {
            // Never turn omitted hits into an apparently successful empty search.
            entry["result"] = Value::Null;
            continue;
        }
        return Err(Failure::Permanent(
            "knowledge receipt identity exceeded its prompt byte budget".into(),
        ));
    }
    Ok(entry)
}

#[cfg(test)]
mod tests;
