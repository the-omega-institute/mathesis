//! Portable public proof evidence. No private review or probe material.
use super::App;
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId},
    model::*,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct VerificationBundle {
    pub schema_version: u32,
    pub reused_contributions: Vec<super::CheckedContribution>,
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub source_sha256: String,
    pub statement: String,
    pub attempt: PublicAttempt,
    pub verification_request: VerificationRequest,
    pub instructions: &'static str,
}
impl App {
    pub async fn verification_bundle(
        &self,
        record: &RecordId,
        claim: &ClaimId,
        attempt: &str,
    ) -> CoreResult<VerificationBundle> {
        let source = self.conjecture_source(record).await?;
        let target = self.target(record, claim).await?;
        let proof = self
            .public_attempts(record, claim)
            .await?
            .into_iter()
            .find(|a| a.id == attempt)
            .ok_or_else(|| CoreError::not_found("verified attempt", attempt))?;
        let mut reused_contributions = vec![];
        for pin in &proof.reuses {
            reused_contributions.push(self.checked_contribution(&pin.attempt).await?);
        }
        let current = self.conjecture_source(record).await?;
        let current_target = self.target(record, claim).await?;
        if current.revision != source.revision
            || current_target.digest != target.digest
            || proof.receipt.target_digest != target.digest
            || proof.receipt.toolchain != target.toolchain
        {
            return Err(CoreError::conflict(
                "source or confirmed target changed; reload the evidence",
            ));
        }
        let version = source
            .current_version()
            .ok_or_else(|| CoreError::conflict("source version unavailable"))?;
        Ok(VerificationBundle {
            schema_version: if reused_contributions.is_empty() {
                1
            } else {
                2
            },
            reused_contributions,
            record: record.clone(),
            claim: claim.clone(),
            version: version.number,
            claims_revision: source.claims_revision,
            source_sha256: version.archive.sha256.clone(),
            statement: source
                .claim(claim)
                .ok_or_else(|| CoreError::not_found("claim", claim.as_str()))?
                .statement
                .clone(),
            verification_request: VerificationRequest {
                required_declaration: None,
                inspect_constants: proof.reuses.iter().map(|r| r.declaration.clone()).collect(),
                modules: proof.modules.clone(),
                target: target.lean,
                target_digest: target.digest,
                toolchain: target.toolchain,
                solution: proof.solution.clone(),
            },
            attempt: proof,
            instructions: "Extract verification_request as JSON and pass it on stdin to mathesis-verifier with WISHPOOL_LEAN_WORKSPACE pointing to a prepared workspace matching the pinned Lean/Mathlib toolchain. Supporting modules map flat module names to complete source. Do not run contributor Lake scripts. The original receipt records the prior check; a new check produces its own receipt. This bundle proves or disproves the author-confirmed Lean target, not an independent guarantee of the natural-language statement. The historical verifier binary revision is not recorded.",
        })
    }
}
