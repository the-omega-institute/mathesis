use super::*;

pub(crate) fn msc_labels() -> BTreeMap<String, String> {
    let source = include_str!("msc_labels.json");
    let parsed: Value = serde_json::from_str(source).unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    if let Some(labels) = parsed.as_object() {
        for (code, title) in labels {
            if let Some(title) = title.as_str() {
                out.insert(code.clone(), title.to_owned());
            }
        }
    }
    out
}
fn top_msc(codes: &[String]) -> String {
    codes
        .first()
        .map(|s| s.chars().take(2).collect())
        .filter(|s: &String| s.len() == 2)
        .unwrap_or_else(|| "unclassified".into())
}
pub(crate) fn is_verified(n: &Node) -> bool {
    n.status == "verified"
        && n.receipt
            .as_ref()
            .and_then(|r| text(r, "verdict"))
            .as_deref()
            == Some("proved")
        && n.target_digest.is_some()
        && n.receipt.as_ref().and_then(|r| text(r, "target_digest")) == n.target_digest
}
fn is_disproved(n: &Node) -> bool {
    n.status == "disproved"
        || n.receipt
            .as_ref()
            .and_then(|r| text(r, "verdict"))
            .as_deref()
            == Some("disproved")
}
pub(crate) fn compute_stats(snapshot: &Snapshot, works: &[Work], nodes: &[Node]) -> Stats {
    let mut by_msc = BTreeMap::<String, MscStats>::new();
    for w in works.iter().filter(|w| w.publication == "public") {
        let key = top_msc(&w.msc);
        let x = by_msc.entry(key.clone()).or_default();
        x.papers += 1;
    }
    for n in nodes.iter().filter(|n| n.scope == "in_scope") {
        let key = works
            .iter()
            .find(|w| w.work == n.work)
            .map(|w| top_msc(&w.msc))
            .unwrap_or_else(|| "unclassified".into());
        let x = by_msc.entry(key).or_default();
        x.registered += 1;
        if is_verified(n) {
            x.verified += 1;
        }
        if n.status == "open" {
            x.open_targets += 1;
        }
    }
    let scoped_nodes: Vec<&Node> = nodes.iter().filter(|n| n.scope == "in_scope").collect();
    let verified: Vec<&Node> = scoped_nodes
        .iter()
        .copied()
        .filter(|n| is_verified(n))
        .collect();
    let awaiting = scoped_nodes
        .iter()
        .filter(|n| n.target_digest.is_none() && !is_verified(n) && !is_disproved(n))
        .count();
    Stats {
        snapshot_time: snapshot.meta.captured_at.clone(),
        public_papers: works.iter().filter(|w| w.publication == "public").count(),
        registered_statements: nodes.iter().filter(|n| n.scope == "in_scope").count(),
        independently_verified_statements: verified.len(),
        verified_from_existing_library_declarations: verified
            .iter()
            .filter(|n| {
                n.proof_basis
                    .as_ref()
                    .and_then(|b| text(b, "kind"))
                    .as_deref()
                    == Some("library_declarations")
            })
            .count(),
        verified_with_new_lean: verified
            .iter()
            .filter(|n| {
                n.proof_basis
                    .as_ref()
                    .and_then(|b| text(b, "kind"))
                    .as_deref()
                    == Some("new_lean")
            })
            .count(),
        verified_counterexamples_disproofs: nodes
            .iter()
            .filter(|n| n.scope == "in_scope" && is_disproved(n))
            .count(),
        open_formalization_targets: nodes
            .iter()
            .filter(|n| n.scope == "in_scope" && n.status == "open" && !is_verified(n))
            .count(),
        statements_awaiting_formal_target: awaiting,
        awaiting_target_needs_attention: nodes
            .iter()
            .filter(|n| {
                n.scope == "in_scope" && {
                    n.proof_state.as_deref() == Some("needs_attention") && n.target_digest.is_none()
                }
            })
            .count(),
        papers_with_lean_source: works
            .iter()
            .filter(|w| w.publication == "public" && w.lean_source.is_some())
            .count(),
        papers_with_doi: works
            .iter()
            .filter(|w| w.publication == "public" && w.doi.as_deref().is_some_and(valid_doi))
            .count(),
        by_msc,
    }
}
fn valid_doi(s: &str) -> bool {
    s.starts_with("10.") && s.contains('/')
}
