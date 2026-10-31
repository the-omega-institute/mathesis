use super::{App, clamp};
use crate::{
    CoreError, CoreResult,
    ids::{ClaimId, RecordId},
    model::*,
    ports::Listing,
};
use sha2::{Digest, Sha256};
impl App {
    pub async fn submit_lean_deposit(
        &self,
        caller: &Caller,
        new: NewLeanDeposit,
    ) -> CoreResult<LeanDeposit> {
        new.validate()?;
        // Same owner and complete metadata/source body replay to the same durable deposit.
        let hash = Sha256::digest(
            serde_json::to_vec(&new).map_err(|_| CoreError::invalid("deposit metadata"))?,
        );
        let id = format!(
            "lean-{:x}",
            Sha256::digest(format!("{}:{hash:x}", caller.person))
        );
        if let Some(existing) = self.ports.deposits.get(&id).await? {
            return Ok(existing);
        }
        let source = self
            .ports
            .blobs
            .put(new.solution.as_bytes(), "text/plain; charset=utf-8")
            .await?;
        let mut modules = std::collections::BTreeMap::new();
        for (name, text) in &new.modules {
            modules.insert(
                name.clone(),
                self.ports
                    .blobs
                    .put(text.as_bytes(), "text/plain; charset=utf-8")
                    .await?,
            );
        }
        let deposit = LeanDeposit {
            modules,
            project_files: new.project_files,
            formalizers: new.formalizers,
            source_revision: new.source_revision,
            id: id.clone(),
            owner: caller.person.clone(),
            title: new.title,
            authors: new.authors,
            statement: new.statement,
            source_reference: new.source_reference,
            toolchain: new.toolchain,
            license: new.license,
            source,
            submitted_at: self.ports.clock.now(),
            attempts: vec![],
            revision: 0,
        };
        match self.ports.deposits.insert(&deposit).await {
            Ok(()) => Ok(deposit),
            Err(CoreError::Conflict(_)) => {
                let existing = self
                    .ports
                    .deposits
                    .get(&id)
                    .await?
                    .ok_or_else(|| CoreError::conflict("deposit admission conflict"))?;
                // The durable winner is known. Discard only this request's fresh
                // writes that it does not reference. Never clean up on an uncertain
                // insert outcome: the database may have committed our own sources.
                let kept: std::collections::BTreeSet<_> = std::iter::once(&existing.source.id)
                    .chain(existing.modules.values().map(|blob| &blob.id))
                    .collect();
                for blob in std::iter::once(&deposit.source).chain(deposit.modules.values()) {
                    if !kept.contains(&blob.id) {
                        // Cleanup failure must not turn a successfully admitted
                        // idempotent request into a failed submission.
                        let _ = self.ports.blobs.discard(&blob.id).await;
                    }
                }
                Ok(existing)
            }
            Err(e) => Err(e),
        }
    }
    pub async fn lean_deposit(&self, caller: &Caller, id: &str) -> CoreResult<LeanDeposit> {
        self.ports
            .deposits
            .get(id)
            .await?
            .filter(|d| d.owner == caller.person || Self::is_staff(caller))
            .ok_or_else(|| CoreError::not_found("deposit", id))
    }
    pub async fn lean_deposits(
        &self,
        caller: &Caller,
        limit: Option<u32>,
        before: Option<String>,
    ) -> CoreResult<Listing<LeanDeposit>> {
        self.ports
            .deposits
            .list(&caller.person, clamp(limit), before)
            .await
    }
    pub async fn lean_deposit_source(&self, caller: &Caller, id: &str) -> CoreResult<String> {
        let d = self.lean_deposit(caller, id).await?;
        let bytes = self
            .ports
            .blobs
            .get(&d.source.id)
            .await?
            .ok_or_else(|| CoreError::Unavailable("Lean source is missing".into()))?;
        String::from_utf8(bytes)
            .map_err(|_| CoreError::Unavailable("stored Lean source is not UTF-8".into()))
    }
    pub async fn lean_deposit_modules(
        &self,
        caller: &Caller,
        id: &str,
    ) -> CoreResult<std::collections::BTreeMap<String, String>> {
        let deposit = self.lean_deposit(caller, id).await?;
        self.lean_module_sources(&deposit.modules).await
    }
    pub(crate) async fn lean_module_sources(
        &self,
        modules: &std::collections::BTreeMap<String, BlobRef>,
    ) -> CoreResult<std::collections::BTreeMap<String, String>> {
        let mut sources = std::collections::BTreeMap::new();
        for (name, blob) in modules {
            let bytes = self
                .ports
                .blobs
                .get(&blob.id)
                .await?
                .ok_or_else(|| CoreError::Unavailable("Lean module is missing".into()))?;
            if bytes.len() as u64 != blob.bytes
                || format!("{:x}", Sha256::digest(&bytes)) != blob.sha256
            {
                return Err(CoreError::Unavailable(
                    "Lean module integrity failure".into(),
                ));
            }
            sources.insert(
                name.clone(),
                String::from_utf8(bytes)
                    .map_err(|_| CoreError::Unavailable("Lean module is not UTF-8".into()))?,
            );
        }
        Ok(sources)
    }
    pub async fn verify_lean_deposit(
        &self,
        caller: &Caller,
        id: &str,
        record: &RecordId,
        claim: &ClaimId,
    ) -> CoreResult<AttemptView> {
        self.verify_lean_deposit_against(caller, id, record, claim, None)
            .await
    }
    pub async fn verify_lean_deposit_against(
        &self,
        caller: &Caller,
        id: &str,
        record: &RecordId,
        claim: &ClaimId,
        expected_target: Option<&str>,
    ) -> CoreResult<AttemptView> {
        self.verify_lean_project(caller, id, record, claim, expected_target, None)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn verify_lean_project(
        &self,
        caller: &Caller,
        id: &str,
        record: &RecordId,
        claim: &ClaimId,
        expected_target: Option<&str>,
        adapter: Option<String>,
    ) -> CoreResult<AttemptView> {
        let deposit = self.lean_deposit(caller, id).await?;
        if deposit.owner != caller.person {
            return Err(CoreError::forbidden(
                "only the depositor can submit a verification attempt",
            ));
        }
        let target = self.target(record, claim).await?;
        if expected_target.is_some_and(|digest| digest != target.digest) {
            return Err(CoreError::conflict(
                "target changed; review the current target before verification",
            ));
        }
        if target.toolchain != deposit.toolchain {
            return Err(CoreError::invalid(
                "deposit toolchain must match the confirmed target toolchain",
            ));
        }
        let source = match adapter {
            Some(source) => source,
            None => self.lean_deposit_source(caller, id).await?,
        };
        let modules = self.lean_deposit_modules(caller, id).await?;
        let report =
            lean_project_preflight(&source, &modules, &deposit.project_files, &target.toolchain);
        if !report.errors.is_empty() {
            return Err(CoreError::invalid(report.errors.join("; ")));
        }
        let attribution = FormalizationAttribution {
            formalizers: deposit.formalizers.clone(),
            source_revision: deposit.source_revision.clone(),
            deposit: id.into(),
            authors: deposit.authors.clone(),
            source_reference: deposit.source_reference.clone(),
            license: deposit.license.clone(),
        };
        let attempt = self
            .submit_attributed_attempt(
                caller,
                record,
                claim,
                super::solving::AttemptInput {
                    intermediate: None,
                    reuses: vec![],
                    expected_target: Some(target.digest),
                    solution: source,
                    modules,
                    as_agent: None,
                    note: format!("Formalization deposit {id}"),
                    attribution: Some(attribution),
                },
            )
            .await?;
        loop {
            let mut d = self.lean_deposit(caller, id).await?;
            if d.attempts.contains(&attempt.id) {
                return Ok(attempt);
            }
            let expected = d.revision;
            d.attempts.push(attempt.id.clone());
            d.revision += 1;
            match self.ports.deposits.replace(&d, expected).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    return Ok(attempt);
                }
            }
        }
    }
}
