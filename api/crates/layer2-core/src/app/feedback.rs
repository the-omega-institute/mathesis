//! Immutable licensed proposal packages; no upstream publication is performed.
use super::App;
use crate::{
    CoreError, CoreResult,
    ids::SubmissionId,
    model::*,
    ports::{JobKind, JobLease, LeasedJob},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_PAYLOAD: usize = 12 * 1024 * 1024;
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn encode(value: &impl serde::Serialize) -> CoreResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| CoreError::invalid("cannot encode feedback evidence"))
}
fn proposal<'a>(file: &'a RefereeFile, key: &str) -> CoreResult<&'a FeedbackProposal> {
    file.feedback_proposals
        .iter()
        .find(|p| p.id == key)
        .ok_or_else(|| CoreError::not_found("feedback proposal", key))
}
fn may_read(caller: &Caller, paper: &Submission, p: &FeedbackProposal) -> bool {
    paper.involves(&caller.person)
        || App::is_staff(caller)
        || p.required_signers.contains(&caller.person)
}
fn room(p: &FeedbackProposal) -> CoreResult<()> {
    if p.events.len() >= 100 {
        Err(CoreError::conflict("feedback history is full"))
    } else {
        Ok(())
    }
}
impl App {
    async fn feedback_preview_inner(
        &self,
        id: &SubmissionId,
        kind: FeedbackKind,
    ) -> CoreResult<FeedbackPreview> {
        kind.validate()?;
        let paper = self.load(id).await?;
        let SubmissionStatus::Accepted { record } = &paper.status else {
            return Err(CoreError::conflict(
                "feedback requires an accepted public publication",
            ));
        };
        if paper.purpose != SubmissionPurpose::Publication || paper.claims_revision == 0 {
            return Err(CoreError::conflict("confirmed publication claims required"));
        }
        let current = self.conjecture_source(record).await?;
        if current.revision != paper.revision {
            return Err(CoreError::conflict("source changed"));
        }
        let version = paper
            .current_version()
            .ok_or_else(|| CoreError::conflict("source version missing"))?;
        let source = FeedbackSource {
            submission: id.clone(),
            record: record.clone(),
            version: version.number,
            claims_revision: paper.claims_revision,
            source_digest: format!("sha256:{}", version.archive.sha256),
        };
        let mut signers = vec![paper.submitter.clone()];
        let (producer, evidence, scope) = match &kind {
            FeedbackKind::OpenProblem { claim } => {
                let problem = self.stored_conjecture(record, claim).await?;
                if problem.summary.status != "open" {
                    return Err(CoreError::conflict(
                        "only currently admitted open problems can be proposed",
                    ));
                }
                (
                    format!("mathesis:open-problem:{record}:{claim}"),
                    json!({"problem":problem}),
                    "Independently reviewed open-problem admission; no proof, preregistration or upstream eligibility receipt.",
                )
            }
            FeedbackKind::VerifiedSolution { claim, attempt }
            | FeedbackKind::DtrRegistration { claim, attempt, .. } => {
                let bundle = self.verification_bundle(record, claim, attempt).await?;
                if lean_solution_digest(
                    &bundle.verification_request.solution,
                    &bundle.verification_request.modules,
                ) != bundle.attempt.receipt.solution_digest
                    || format!(
                        "{:x}",
                        Sha256::digest(bundle.verification_request.target.as_bytes())
                    ) != bundle.attempt.receipt.target_digest
                {
                    return Err(CoreError::Unavailable(
                        "verified package bytes do not match the saved receipt".into(),
                    ));
                }
                let (file, saved) = self.find_attempt(attempt).await?;
                if file.record != *record || file.claim != *claim || !saved.verified() {
                    return Err(CoreError::conflict("verified attempt identity changed"));
                }
                signers.push(saved.owner.clone());
                let mut bundle = serde_json::to_value(bundle)
                    .map_err(|_| CoreError::invalid("cannot encode verification bundle"))?;
                // Preserve attribution as filed. A later profile rename or another
                // solver's earlier proof changes neither these bytes nor their rights.
                bundle["attempt"]["entrant"] = json!(saved.entrant);
                if let Some(attempt) = bundle["attempt"].as_object_mut() {
                    attempt.remove("also_verified");
                }
                (
                    format!("mathesis:verified-attempt:{attempt}"),
                    json!({"verification_bundle":bundle,"solution_owner":saved.owner,"original_attribution":saved.attribution}),
                    "Independent verifier receipt for the exact author-confirmed Lean target. Any DTR slots are a proposed registration, not an admitted registration or coverage certificate.",
                )
            }
            FeedbackKind::LibraryBinding {
                board,
                contribution,
            } => {
                let public = self.public_binding_board(id, board).await?;
                let c = public
                    .contributions
                    .iter()
                    .find(|c| c.id == *contribution)
                    .ok_or_else(|| CoreError::not_found("binding contribution", contribution))?;
                let a = public
                    .audits
                    .iter()
                    .rev()
                    .find(|a| {
                        public.contributions.iter().any(|other| {
                            other.id == a.contribution && other.canonical == c.canonical
                        })
                    })
                    .filter(|a| a.verdict == BindingVerdict::Supported)
                    .ok_or_else(|| {
                        CoreError::conflict("binding is not currently independently supported")
                    })?;
                signers.push(c.contributor.clone());
                (
                    format!("mathesis:binding:{id}:{board}:{contribution}"),
                    json!({"target":public.target,"context":public.context,"summary_license":public.license,"summary_approved_by":public.authorized_by,"contribution":c,"independent_judgment":a}),
                    "Source-audited joint binding; no recognized coverage certificate, formal proof or upstream acceptance.",
                )
            }
            FeedbackKind::WitnessFormalization { claim, witness } => {
                let public = self.paper(record).await?;
                let text = public
                    .new_content
                    .iter()
                    .find(|w| w.claim == *claim)
                    .and_then(|w| w.lemmas.get(*witness))
                    .ok_or_else(|| {
                        CoreError::conflict("current public content witness unavailable")
                    })?;
                let statement = public
                    .claims
                    .iter()
                    .find(|c| c.id == *claim)
                    .ok_or_else(|| CoreError::not_found("claim", claim.as_str()))?;
                // A witness proposal does not borrow another owner's proof grant.
                let statement = json!({"id":statement.id,"kind":statement.kind,"role":statement.role,"label":statement.label,"statement":statement.statement,"section":statement.section,"depends_on":statement.depends_on});
                (
                    format!("mathesis:content-witness:{record}:{claim}:{witness}"),
                    json!({"statement":statement,"witness":text}),
                    "Public audited mathematical witness offered for formalization. This witness has no proof receipt by this operation.",
                )
            }
            FeedbackKind::Erratum { report, upstream } => {
                let saved = self
                    .all_error_reports()
                    .await?
                    .into_iter()
                    .find(|r| r.id == *report)
                    .ok_or_else(|| CoreError::not_found("error report", report))?;
                if saved.submission != *id
                    || saved.record != *record
                    || saved.version != source.version
                    || saved.claims_revision != source.claims_revision
                    || !saved
                        .resolution
                        .as_ref()
                        .is_some_and(|r| r.verdict == ErrorVerdict::Confirmed)
                {
                    return Err(CoreError::conflict(
                        "confirmed report does not match the current source",
                    ));
                }
                let public = self.error_report(None, report).await?;
                signers.push(saved.owner);
                (
                    format!("mathesis:error-report:{report}"),
                    json!({"confirmed_report":public,"asserted_affected_upstream":upstream}),
                    "Confirmed Mathesis error report. The supplied upstream identities are proposed affected subjects, not a verified upstream defect or correction.",
                )
            }
        };
        signers.sort();
        signers.dedup();
        let payload = json!({"protocol":FEEDBACK_PROTOCOL,"producer":producer,"source":source,"route":kind,"attribution":{"authors":paper.authors,"title":paper.title,"submitter":paper.submitter},"requested_license":FEEDBACK_LICENSE,"manuscript_license":"Not recorded by this operation; export requires the submitting author’s explicit applicable-rights grant.","required_signers":signers,"verification_scope":scope,"evidence":evidence,"rights_notice":"The original attribution and licenses remain attached. Every required signer must separately assert applicable rights and authorize this exact payload under Apache-2.0. Preparation and prior public sharing grant no export authorization.","upstream_admission":"Proposal only. Existing upstream intake and checks decide admission; no D5 registration, first-tier eligibility, frozen status or certificate is asserted."});
        let bytes = encode(&payload)?;
        if bytes.len() > MAX_PAYLOAD {
            return Err(CoreError::invalid("feedback package exceeds 12 MiB"));
        }
        if self.load(id).await?.revision != paper.revision {
            return Err(CoreError::conflict(
                "source changed while preparing evidence",
            ));
        }
        Ok(FeedbackPreview {
            digest: digest(&bytes),
            producer,
            source,
            kind,
            required_signers: signers,
            license: FEEDBACK_LICENSE.into(),
            payload,
        })
    }
    pub async fn preview_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        kind: FeedbackKind,
    ) -> CoreResult<FeedbackPreview> {
        Self::require_submitter(caller, &self.load(id).await?)?;
        self.feedback_preview_inner(id, kind).await
    }
    async fn feedback_current(&self, p: &FeedbackProposal) -> CoreResult<bool> {
        if p.revoked() {
            return Ok(false);
        }
        match self
            .feedback_preview_inner(&p.source.submission, p.kind.clone())
            .await
        {
            Ok(now) => Ok(now.digest == p.digest
                && now.source == p.source
                && now.required_signers == p.required_signers),
            Err(CoreError::NotFound { .. } | CoreError::Conflict(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
    async fn feedback_payload(&self, p: &FeedbackProposal) -> CoreResult<Value> {
        if p.payload.bytes > MAX_PAYLOAD as u64 {
            return Err(CoreError::Unavailable(
                "feedback payload exceeds limit".into(),
            ));
        }
        let bytes = self
            .ports
            .blobs
            .get(&p.payload.id)
            .await?
            .ok_or_else(|| CoreError::Unavailable("feedback payload missing".into()))?;
        if bytes.len() as u64 != p.payload.bytes
            || digest(&bytes) != p.digest
            || format!("sha256:{}", p.payload.sha256) != p.digest
        {
            return Err(CoreError::Unavailable(
                "feedback payload checksum mismatch".into(),
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| CoreError::Unavailable("feedback payload is invalid".into()))
    }
    pub async fn feedback_observation_evidence(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        request: &str,
    ) -> CoreResult<Value> {
        let view = self.feedback(caller, id, key).await?;
        let blob = view
            .proposal
            .events
            .iter()
            .find_map(|e| match e {
                FeedbackEvent::Observed { observation } if observation.request == request => {
                    Some(&observation.evidence)
                }
                _ => None,
            })
            .ok_or_else(|| CoreError::not_found("feedback observation", request))?;
        if blob.bytes > 4 * 1024 * 1024 {
            return Err(CoreError::Unavailable(
                "observation evidence exceeds limit".into(),
            ));
        }
        let bytes = self
            .ports
            .blobs
            .get(&blob.id)
            .await?
            .ok_or_else(|| CoreError::Unavailable("observation evidence missing".into()))?;
        if bytes.len() as u64 != blob.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != blob.sha256
        {
            return Err(CoreError::Unavailable(
                "observation evidence checksum mismatch".into(),
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| CoreError::Unavailable("invalid observation evidence".into()))
    }
    pub async fn prepare_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        new: NewFeedbackProposal,
    ) -> CoreResult<FeedbackView> {
        if new.preparation_consent != FEEDBACK_PREPARATION_CONSENT {
            return Err(CoreError::invalid(
                "explicit proposal preparation consent required",
            ));
        }
        let preview = self.preview_feedback(caller, id, new.kind).await?;
        if preview.digest != new.preview_digest {
            return Err(CoreError::conflict(
                "preview changed; inspect the exact package again",
            ));
        }
        let key = preview.digest.clone();
        let (file, _) = self.referee_file(id).await?;
        if proposal(&file, &key).is_ok() {
            return self.feedback(caller, id, &key).await;
        }
        let payload = self
            .ports
            .blobs
            .put(&encode(&preview.payload)?, "application/json")
            .await?;
        loop {
            Self::require_submitter(caller, &self.load(id).await?)?;
            let (mut file, stored) = self.referee_file(id).await?;
            if proposal(&file, &key).is_ok() {
                return self.feedback(caller, id, &key).await;
            }
            if file.feedback_proposals.len() >= 40 {
                return Err(CoreError::conflict(
                    "at most forty proposals per submission",
                ));
            }
            let p = FeedbackProposal {
                id: key.clone(),
                digest: preview.digest.clone(),
                producer: preview.producer.clone(),
                source: preview.source.clone(),
                kind: preview.kind.clone(),
                payload: payload.clone(),
                required_signers: preview.required_signers.clone(),
                prepared_by: caller.person.clone(),
                prepared_at: self.ports.clock.now(),
                preparation_consent: new.preparation_consent.clone(),
                events: vec![],
            };
            if !self.feedback_current(&p).await? {
                return Err(CoreError::conflict("proposal evidence changed"));
            }
            file.feedback_proposals.push(p);
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.feedback(caller, id, &key).await
    }
    pub async fn feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
    ) -> CoreResult<FeedbackView> {
        let paper = self.load(id).await?;
        let (file, _) = self.referee_file(id).await?;
        let p = proposal(&file, key)?;
        if !may_read(caller, &paper, p) {
            return Err(CoreError::not_found("feedback proposal", key));
        }
        let current = self.feedback_current(p).await?;
        // A required contributor can inspect only the exact currently shareable package.
        if !current && !paper.involves(&caller.person) && !Self::is_staff(caller) {
            return Err(CoreError::not_found("feedback proposal", key));
        }
        let state = if p.revoked() {
            "revoked"
        } else if !current {
            "source_or_evidence_changed"
        } else if p.authorized() {
            "authorized"
        } else {
            "awaiting_authorizations"
        };
        Ok(FeedbackView {
            proposal: p.clone(),
            payload: if current {
                Some(self.feedback_payload(p).await?)
            } else {
                None
            },
            state: state.into(),
        })
    }
    pub async fn feedback_proposals(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<Vec<FeedbackProposal>> {
        self.submission(caller, id).await?;
        Ok(self.referee_file(id).await?.0.feedback_proposals)
    }
    pub async fn authorize_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        input: FeedbackAuthorization,
    ) -> CoreResult<FeedbackView> {
        if input.consent != FEEDBACK_EXPORT_CONSENT
            || input.license != FEEDBACK_LICENSE
            || !input.rights_asserted
        {
            return Err(CoreError::invalid(
                "separate explicit Apache-2.0 authorization and applicable-rights assertion required",
            ));
        }
        loop {
            let (mut file, stored) = self.referee_file(id).await?;
            let p = proposal(&file, key)?;
            if !p.required_signers.contains(&caller.person) {
                return Err(CoreError::not_found("feedback authorization", key));
            }
            if input.digest != p.digest || !self.feedback_current(p).await? {
                return Err(CoreError::conflict(
                    "proposal digest or current evidence changed",
                ));
            }
            self.feedback_payload(p).await?;
            if p.events
                .iter()
                .any(|e| matches!(e,FeedbackEvent::Authorized{by,..} if *by==caller.person))
            {
                break;
            }
            room(p)?;
            file.feedback_proposals
                .iter_mut()
                .find(|p| p.id == key)
                .unwrap()
                .events
                .push(FeedbackEvent::Authorized {
                    by: caller.person.clone(),
                    digest: input.digest.clone(),
                    license: input.license.clone(),
                    consent: input.consent.clone(),
                    rights_asserted: true,
                    at: self.ports.clock.now(),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                result => {
                    result?;
                    break;
                }
            }
        }
        self.feedback(caller, id, key).await
    }
    pub async fn revoke_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        reason: String,
    ) -> CoreResult<()> {
        require_text("revocation reason", &reason, 2000)?;
        loop {
            let (mut file, stored) = self.referee_file(id).await?;
            let p = proposal(&file, key)?;
            if !p.required_signers.contains(&caller.person) {
                return Err(CoreError::not_found("feedback proposal", key));
            }
            if p.revoked() {
                return Ok(());
            }
            // Revocation is always possible, even after the ordinary history cap.
            file.feedback_proposals
                .iter_mut()
                .find(|p| p.id == key)
                .unwrap()
                .events
                .push(FeedbackEvent::Revoked {
                    by: caller.person.clone(),
                    reason: reason.clone(),
                    at: self.ports.clock.now(),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                r => return r,
            }
        }
    }
    pub async fn export_feedback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
    ) -> CoreResult<FeedbackExport> {
        loop {
            let paper = self.load(id).await?;
            let (mut file, stored) = self.referee_file(id).await?;
            let p = proposal(&file, key)?;
            if !p.required_signers.contains(&caller.person) {
                return Err(CoreError::not_found("feedback export", key));
            }
            if !p.authorized() || !self.feedback_current(p).await? {
                return Err(CoreError::conflict(
                    "all exact authorizations and current public evidence are required",
                ));
            }
            let payload = self.feedback_payload(p).await?;
            let out = FeedbackExport {
                protocol: FEEDBACK_PROTOCOL.into(),
                proposal: p.id.clone(),
                digest: p.digest.clone(),
                authorizations: p
                    .events
                    .iter()
                    .filter(|e| matches!(e, FeedbackEvent::Authorized { .. }))
                    .cloned()
                    .collect(),
                proposal_markdown: format!(
                    "# Mathesis proposal\n\nProducer: {}\n\nMathesis-Proposal: {}\n\nSource: {} · version {} · claims revision {}\n\nRequested license: {}\n\nVerification scope: {}\n\n{}\n\nThe attached exact JSON payload preserves statements, attribution, source identities and available verification receipts. This package performs no upstream write and establishes no upstream acceptance, freeze or registration.\n",
                    p.producer,
                    p.digest,
                    p.source.record,
                    p.source.version,
                    p.source.claims_revision,
                    FEEDBACK_LICENSE,
                    payload["verification_scope"]
                        .as_str()
                        .unwrap_or("unavailable"),
                    payload["rights_notice"].as_str().unwrap_or("")
                ),
                payload,
            };
            if !may_read(caller, &paper, p) {
                return Err(CoreError::not_found("feedback export", key));
            }
            // Identical downloads reuse the durable per-owner issuance event.
            if p.events
                .iter()
                .any(|e| matches!(e,FeedbackEvent::Exported{by,..} if *by==caller.person))
            {
                return Ok(out);
            }
            room(p)?;
            file.feedback_proposals
                .iter_mut()
                .find(|p| p.id == key)
                .unwrap()
                .events
                .push(FeedbackEvent::Exported {
                    by: caller.person.clone(),
                    at: self.ports.clock.now(),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                r => {
                    r?;
                    return Ok(out);
                }
            }
        }
    }
    pub async fn request_feedback_readback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
        key: &str,
        request: FeedbackReadbackRequest,
    ) -> CoreResult<FeedbackView> {
        request.validate()?;
        loop {
            let paper = self.load(id).await?;
            Self::require_submitter(caller, &paper)?;
            let (mut file, stored) = self.referee_file(id).await?;
            let p = proposal(&file, key)?;
            if !p.authorized()
                || !p
                    .events
                    .iter()
                    .any(|e| matches!(e, FeedbackEvent::Exported { .. }))
                || !self.feedback_current(p).await?
            {
                return Err(CoreError::conflict(
                    "a current authorized and exported proposal is required",
                ));
            }
            if let Some(old) = p.events.iter().find_map(|e| match e {
                FeedbackEvent::ReadbackRequested { request: old, .. }
                    if old.observation_key == request.observation_key =>
                {
                    Some(old)
                }
                _ => None,
            }) {
                if old != &request {
                    return Err(CoreError::conflict(
                        "observation key already binds another request",
                    ));
                }
                break;
            }
            if p.events
                .iter()
                .filter(|e| matches!(e, FeedbackEvent::ReadbackRequested { .. }))
                .count()
                >= 10
            {
                return Err(CoreError::conflict(
                    "at most ten explicit observations per proposal",
                ));
            }
            room(p)?;
            let request_id = digest(&encode(&(key, &request))?);
            file.feedback_proposals
                .iter_mut()
                .find(|p| p.id == key)
                .unwrap()
                .events
                .push(FeedbackEvent::ReadbackRequested {
                    id: request_id,
                    request: request.clone(),
                    by: caller.person.clone(),
                    at: self.ports.clock.now(),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                r => {
                    r?;
                    break;
                }
            }
        }
        self.ports
            .queue
            .enqueue(id, JobKind::FeedbackReadback)
            .await?;
        self.feedback(caller, id, key).await
    }
    pub async fn pending_feedback_readback(
        &self,
        caller: &Caller,
        id: &SubmissionId,
    ) -> CoreResult<Option<PendingFeedback>> {
        caller.require(Role::Reviewer)?;
        let (file, _) = self.referee_file(id).await?;
        for p in file.feedback_proposals {
            if p.authorized()
                && let Some((request_id, request)) = p.pending_readback()
                && self.feedback_current(&p).await?
            {
                return Ok(Some(PendingFeedback {
                    proposal: p,
                    request_id,
                    request,
                }));
            }
        }
        Ok(None)
    }
    /// Composition only. There is deliberately no HTTP/CLI operation for filing success.
    pub async fn record_feedback_observation(
        &self,
        caller: &Caller,
        job: &LeasedJob,
        lease: &dyn JobLease,
        pending: &PendingFeedback,
        mut result: InspectedFeedback,
    ) -> CoreResult<()> {
        caller.require(Role::Reviewer)?;
        let o = &result.observation;
        if job.kind != JobKind::FeedbackReadback
            || job.submission != pending.proposal.source.submission
            || o.request != pending.request_id
            || o.pull_request != pending.request.pull_request
            || o.index_revision != pending.request.index_revision
            || o.declarations
                .iter()
                .map(|d| &d.identity)
                .collect::<Vec<_>>()
                != pending.request.declarations.iter().collect::<Vec<_>>()
            || result.evidence.len() > 4 * 1024 * 1024
        {
            return Err(CoreError::invalid(
                "observation does not match the admitted request",
            ));
        }
        if !lease.renew(job).await? {
            return Err(CoreError::conflict("observation lease lost"));
        }
        result.observation.evidence = self
            .ports
            .blobs
            .put(&result.evidence, "application/json")
            .await?;
        loop {
            if !lease.renew(job).await? {
                return Err(CoreError::conflict("observation lease lost"));
            }
            let (mut file, stored) = self.referee_file(&job.submission).await?;
            let p = proposal(&file, &pending.proposal.id)?;
            if p.events.iter().any(|e|matches!(e,FeedbackEvent::Observed{observation} if observation.request==pending.request_id)){return Ok(());}
            if !p.authorized()
                || p.pending_readback()
                    != Some((pending.request_id.clone(), pending.request.clone()))
                || !self.feedback_current(p).await?
            {
                return Err(CoreError::conflict(
                    "observation authorization or current evidence changed",
                ));
            }
            room(p)?;
            file.feedback_proposals
                .iter_mut()
                .find(|p| p.id == pending.proposal.id)
                .unwrap()
                .events
                .push(FeedbackEvent::Observed {
                    observation: Box::new(result.observation.clone()),
                });
            match self.store_referee_file(&mut file, stored).await {
                Err(CoreError::StaleRevision { .. }) => continue,
                r => return r,
            }
        }
    }
    pub(crate) async fn reconcile_feedback(&self) -> CoreResult<()> {
        let mut before = None;
        loop {
            let page = self.ports.referees.feedback_sources(100, before).await?;
            for id in page.items {
                if let Some(file) = self.ports.referees.get(&id).await? {
                    for p in &file.feedback_proposals {
                        if p.authorized()
                            && p.pending_readback().is_some()
                            && self.feedback_current(p).await?
                        {
                            self.ports
                                .queue
                                .enqueue(&id, JobKind::FeedbackReadback)
                                .await?;
                            break;
                        }
                    }
                }
            }
            before = page.next_before;
            if before.is_none() {
                return Ok(());
            }
        }
    }
}
