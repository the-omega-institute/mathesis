//! Public results awaiting formalization, confirmed targets and verified receipts.
use super::{App, clamp};
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId},
    model::*,
    ports::Listing,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalizationQuery {
    pub q: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub before: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct FormalizationOpportunity {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub title: String,
    pub label: String,
    pub statement: String,
    pub authors: Vec<Author>,
    pub kind: String,
    pub status: String,
    pub macros: std::collections::BTreeMap<String, String>,
    pub target_digest: Option<String>,
    pub toolchain: Option<String>,
    pub verified_count: usize,
    pub contributor: Option<Entrant>,
    pub attribution: Option<FormalizationAttribution>,
    pub receipt: Option<VerificationReceipt>,
}
impl App {
    pub async fn formalization_opportunities(
        &self,
        query: FormalizationQuery,
    ) -> CoreResult<Listing<FormalizationOpportunity>> {
        if query
            .kind
            .as_deref()
            .is_some_and(|k| !["paper", "conjecture"].contains(&k))
            || query.status.as_deref().is_some_and(|s| {
                !["unverified", "preparing", "open", "verified", "disproved"].contains(&s)
            })
            || query.q.as_ref().is_some_and(|s| s.chars().count() > 200)
            || query.before.as_ref().is_some_and(|s| s.len() > 250)
        {
            return Err(CoreError::invalid("invalid formalization query"));
        }
        let words: Vec<_> = query
            .q
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let limit = clamp(query.limit) as usize;
        // Existing clients retain the confirmed-target-only directory unless
        // they explicitly request the broader pool.
        let include_preparing = matches!(query.status.as_deref(), Some("unverified" | "preparing"));
        let mut cursor = query.before;
        let mut items = vec![];
        let mut keys = vec![];
        loop {
            let page = self
                .ports
                .solving
                .list_formalization_candidates(100, cursor, include_preparing)
                .await?;
            for file in page.items {
                let source = match self.conjecture_source(&file.record).await {
                    Ok(s) => s,
                    Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => continue,
                    Err(e) => return Err(e),
                };
                if source.current_version().map(|v| v.number) != Some(file.version)
                    || source.claims_revision != file.claims_revision
                {
                    continue;
                }
                let Some(claim) = source.claim(&file.claim) else {
                    continue;
                };
                let kind = if claim.kind.is_open() {
                    "conjecture"
                } else {
                    "paper"
                };
                if query.kind.as_deref().is_some_and(|k| k != kind) {
                    continue;
                }
                let searchable = format!(
                    "{} {} {} {} {}",
                    file.record,
                    source.title,
                    claim.label,
                    claim.statement,
                    source
                        .authors
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .to_lowercase();
                if !words.iter().all(|word| searchable.contains(word)) {
                    continue;
                }
                let preparing = include_preparing
                    && !file.record.as_str().starts_with("WP-")
                    && !claim.kind.is_open()
                    && file.collected.as_ref().is_some_and(|p| p.target.is_none());
                if preparing {
                    // A source update invalidates a pending row just as it does
                    // a confirmed target. Never publish its private run data.
                    match self.collected_solve_file(&file.record, &file.claim).await {
                        Ok(current) if current.id == file.id => {}
                        Ok(_) | Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => {
                            continue;
                        }
                        Err(error) => return Err(error),
                    }
                }
                let target = match self.target(&file.record, &file.claim).await {
                    Ok(t) => Some(t),
                    Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) if preparing => None,
                    Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => continue,
                    Err(e) => return Err(e),
                };
                let file = if target.is_some() {
                    self.public_solve_file(&file.record, &file.claim).await?
                } else {
                    file
                };
                let winner = target
                    .as_ref()
                    .and_then(|target| file.winner(&target.digest));
                let status = if target.is_none() {
                    "preparing"
                } else {
                    match winner.and_then(|a| a.receipt.as_ref()).map(|r| r.verdict) {
                        Some(Verdict::Proved) => "verified",
                        Some(Verdict::Disproved) => "disproved",
                        _ => "open",
                    }
                };
                if query.status.as_deref().is_some_and(|s| {
                    if s == "unverified" {
                        !["preparing", "open"].contains(&status)
                    } else {
                        s != status
                    }
                }) {
                    continue;
                }
                // Only matching public rows contribute to cursors and counts.
                if items.len() == limit {
                    return Ok(Listing {
                        items,
                        next_before: keys.pop(),
                    });
                }
                let contributor = match winner {
                    Some(a) => Some(self.current_entrant(&a.entrant).await?),
                    None => None,
                };
                keys.push(file.id.clone());
                items.push(FormalizationOpportunity {
                    record: file.record.clone(),
                    claim: file.claim.clone(),
                    version: file.version,
                    title: source.title.clone(),
                    label: claim.label.clone(),
                    statement: claim.statement.clone(),
                    authors: source.authors.clone(),
                    kind: kind.into(),
                    status: status.into(),
                    macros: source.macros.clone(),
                    target_digest: target.as_ref().map(|target| target.digest.clone()),
                    toolchain: target.as_ref().map(|target| target.toolchain.clone()),
                    verified_count: file
                        .attempts
                        .iter()
                        .filter(|a| {
                            a.intermediate.is_none()
                                && a.verified()
                                && target
                                    .as_ref()
                                    .is_some_and(|target| a.target_digest == target.digest)
                        })
                        .count(),
                    contributor,
                    attribution: winner.and_then(|a| a.attribution.clone()),
                    receipt: winner.and_then(|a| a.receipt.clone()),
                });
            }
            cursor = page.next_before;
            if cursor.is_none() {
                return Ok(Listing {
                    items,
                    next_before: None,
                });
            }
        }
    }
}
