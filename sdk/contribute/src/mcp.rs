//! A Model Context Protocol server over stdio (newline-delimited JSON-RPC
//! 2.0). Adding it to Claude Code, Codex or any MCP client lets the agent
//! find, lease, do and submit Mathesis tasks with the user's own model.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::client::Client;

const PROTOCOL_VERSION: &str = "2025-06-18";

fn tools() -> Value {
    let string = |description: &str| json!({ "type": "string", "description": description });
    let mut feedback_tools = json!([{"name": "feedbacks", "description": "List private immutable proposals and their authorization/observation history", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}}, {"name": "feedback", "description": "Read the exact package as a rights holder or author/staff; inactive payloads are withheld", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}}, "required": ["id", "key"]}}, {"name": "preview-feedback", "description": "Submitter previews a six-channel proposal from current public evidence; route is a FeedbackKind object", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "route": {"type": "object"}}, "required": ["id", "route"]}}, {"name": "prepare-feedback", "description": "Prepare only after exact digest preview and feedback-prepare-2026-10-16 consent; does not authorize export", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "proposal": {"type": "object"}}, "required": ["id", "proposal"]}}, {"name": "authorize-feedback", "description": "Required rights holder separately asserts rights and approves the exact digest under Apache-2.0 and feedback-export-2026-10-16; no automatic consent", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "authorization": {"type": "object"}}, "required": ["id", "key", "authorization"]}}, {"name": "revoke-feedback", "description": "A required rights holder stops new serving/export/readback; issued licenses and history persist", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "reason": {"type": "string"}}, "required": ["id", "key", "reason"]}}, {"name": "export-feedback", "description": "Download the exact licensed package after all signatures; performs no upstream write", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}}, "required": ["id", "key"]}}, {"name": "feedback-readback", "description": "Request actual read-only observation of an upstream PR and exact admitted source declarations; accepts no success claim", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "observation": {"type": "object"}}, "required": ["id", "key", "observation"]}}]);
    let binding_tools = json!([
        {"name":"binding-profile","description":"Read current public contributions and effective binding credit with permanent discussion/contribution identities; revoked/private sources are excluded.","inputSchema":{"type":"object","properties":{"person":{"type":"string"},"before":{"type":"string"}},"required":["person"]}},
        {"name": "bindings", "description": "List current author-approved public binding summaries", "inputSchema": {"type": "object", "properties": {"before": {"type": "string"}}, "required": []}},
        {"name": "private-bindings", "description": "Read author/staff summary options and complete private audit history", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}},
        {"name": "binding", "description": "Read a public summary, contributions, challenges and source-audited credit", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}}, "required": ["id", "key"]}},
        {"name": "publish-binding", "description": "Submitter explicitly licenses the exact statement/context and selected actual sources. Requires binding-summary-2026-10-16 and CC-BY-4.0.", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "summary": {"type": "object"}}, "required": ["id", "summary"]}},
        {"name": "revoke-binding", "description": "Submitter revokes public sharing and further independent inspection", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "reason": {"type": "string"}}, "required": ["id", "key", "reason"]}},
        {"name": "contribute-binding", "description": "Submit a structured joint binding; requires binding-contribution-2026-10-16 consent and CC-BY-4.0. No client verdict is accepted.", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "contribution": {"type": "object"}}, "required": ["id", "key", "contribution"]}},
        {"name": "challenge-binding", "description": "Submit challenge or appeal evidence for independent inspection, with explicit license and contribution consent. Never decides success.", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "key": {"type": "string"}, "challenge": {"type": "object"}}, "required": ["id", "key", "challenge"]}},
    ]);
    let manuscript_tools = json!([
        {"name":"prepare-submission","description":"Upload and compile a private draft before metadata is complete. Requires an author-requested upload with a current processing request marker. Does not start AI review.","inputSchema":{"type":"object","properties":{"metadata":{"type":"object","description":"kind, purpose, processing_consent"},"filename":string("Manuscript filename"),"source_base64":{"type":"string","maxLength":41943040}},"required":["metadata","filename","source_base64"]}},
        {"name":"submission","description":"Read a private draft, metadata suggestions and PDF preparation state.","inputSchema":{"type":"object","properties":{"id":string("Submission id")},"required":["id"]}},
        {"name":"submit-prepared","description":"Submit a checked PDF with author-approved metadata and statement confirmations. Requires exact version, source_sha256 and claims_revision. Does not guess consent or approval.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"confirmation":{"type":"object"}},"required":["id","confirmation"]}},
        {"name":"deliver-manuscript","description":"Deliver a versioned paper or short note with source associations. Exact retries recover the same manuscript. Optional already-approved confirmations start the ordinary review; publication still requires Mathesis acceptance.","inputSchema":{"type":"object","properties":{"metadata":{"type":"object","description":"mathesis.manuscript-delivery.v1 metadata, including explicit processing consent"},"filename":string("Source basename: PDF, TeX, ZIP or tar.gz"),"source_base64":{"type":"string","maxLength":41943040},"confirmations":{"type":"array","items":{"type":"object"},"description":"Optional author-approved ClaimConfirmation entries for this exact source; never infer consent or statements"}},"required":["metadata","filename","source_base64"]}},
        {"name":"manuscript-status","description":"Read the private, version-bound delivery, review and publication receipt.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"version":{"type":"integer","minimum":1}},"required":["id"]}},
        {"name":"find-manuscript","description":"Recover the submitting account's stable manuscript identity after an interrupted delivery.","inputSchema":{"type":"object","properties":{"producer":string("Producer namespace"),"work_id":string("Stable note/work id"),"purpose":{"enum":["publication","review_only"]}},"required":["producer","work_id","purpose"]}},
        {"name":"confirm-manuscript","description":"Confirm author-approved statements of one delivered source version and start review. Replays are idempotent; stale source/statement confirmations are refused.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"confirmation":{"type":"object","description":"version, source_sha256, expected_claims_revision and claims"}},"required":["id","confirmation"]}},
    ]);
    let mut common = json!([
        {"name":"knowledge-capabilities","description":"Read the downstream consumer's admitted source-query/ranking protocols and exact coverage-certificate support. Unsupported protocols cannot be promoted by ordinary Lean proof receipts.","inputSchema":{"type":"object","properties":{}}},
        {"name":"research", "description":"Read private audited research candidates, author-authorized goals, targets and independent verification receipts.","inputSchema":{"type":"object","properties":{"id":string("Submission id")},"required":["id"]}},
        {"name":"adopt-research", "description":"Authorize a precise private research goal and bounded attempts/deadline. Requires the submitting author's explicit research consent. Confirmation of the resulting exact Lean target is browser-session only.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"obligation":{"type":"object","description":"NewResearchObligation: exact round, claim, gap, candidate_digest, complete goal (definitions, hypotheses, quantifiers, conclusion, correspondence), budget (max_attempts 1–5, expires_at within 30 days), consent research-obligation-2026-10-16"}},"required":["id","obligation"]}},
        {"name":"stop-research", "description":"Revoke new and live work on one private research obligation; retains its history.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"key":string("Obligation id"),"reason":string("Reason")},"required":["id","key","reason"]}},
        {"name":"research-attempt", "description":"Submit bounded Lean sources against an author-confirmed private target. Receipt comes only from the independent verifier and grants no public credit.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"key":string("Obligation id"),"attempt":{"type":"object","properties":{"target_digest":string("Exact confirmed target digest"),"solution":string("Solution.lean"),"modules":{"type":"object","additionalProperties":{"type":"string"}},"note":string("Optional private note")},"required":["target_digest","solution"],"additionalProperties":false}},"required":["id","key","attempt"]}},
        {"name":"research-attempt-status", "description":"Read the exact private solution, modules and verifier receipt.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"key":string("Obligation id"),"attempt":string("Attempt id")},"required":["id","key","attempt"]}},
        {"name":"recheck-research", "description":"Ask a fresh private audit to assess how a checked target relates to the original claim; does not establish external coverage or change publication by itself.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"key":string("Obligation id"),"attempt":string("Attempt id")},"required":["id","key","attempt"]}},
        {"name":"formalization-opportunities", "description":"Discover current public author-confirmed Lean targets and independently verified contributions. Filter paper results or conjectures, open targets, verified proofs or disproofs.", "inputSchema":{"type":"object","properties":{"q":string("Statement, paper or author words"),"kind":{"enum":["paper","conjecture"]},"status":{"enum":["open","verified","disproved"]},"limit":{"type":"integer","minimum":1,"maximum":100},"before":string("next_before cursor")}}},
        {"name":"formalizations", "description":"List your private formalization deposits.", "inputSchema":{"type":"object","properties":{}}},
        {"name":"formalization-project", "description":"Download your complete deposited source project and attribution as reusable submission JSON.", "inputSchema":{"type":"object","properties":{"id":string("Deposit id")},"required":["id"]}},
        {"name":"papers", "description":"Search the public research archive by plain words, author, MSC prefix or kind. Cursor pagination, newest first. No private drafts or reports are searchable.", "inputSchema":{"type":"object", "properties":{"q":string("Title, abstract words, DOI or exact WP record"), "author":string("Author name"), "msc":string("MSC prefix"), "kind":{"enum":["paper","note","conjecture"]}, "limit":{"type":"integer","minimum":1,"maximum":100}, "before":string("next_before from previous page")}}},
        {"name":"paper", "description":"Read a published record and its public statements and proof receipts.", "inputSchema":{"type":"object","properties":{"record":string("WP record")},"required":["record"]}},
        {"name":"review", "description":"Download your private Orchestra review dossier, bound to source version and statement revision. Includes available reports, audits and delivered letters. Requires author/staff identity.","inputSchema":{"type":"object","properties":{"id":string("Submission id")},"required":["id"]}},
        {"name":"knowledge-response", "description":"Read the exact saved, checksummed source-query response for your private review. The receipt id comes from the review dossier. Requires author/staff identity; candidate matches are not entailment certificates.","inputSchema":{"type":"object","properties":{"id":string("Submission id"),"receipt":string("Exact saved query receipt id")},"required":["id","receipt"]}},
        {"name":"respond-review", "description":"Reply to an immutable audited finding and request a private re-review. Does not resolve it. Current confirmed version and claims revision are required.", "inputSchema":{"type":"object","properties":{"id":string("Submission id"),"response":{"type":"object","properties":{"round":{"type":"integer","minimum":1},"concern":{"type":"integer","minimum":0,"description":"Zero-based index in that round's audit concerns"},"version":{"type":"integer","minimum":1},"claims_revision":{"type":"integer","minimum":0},"body":{"type":"string","minLength":1,"maxLength":10000}},"required":["round","concern","version","claims_revision","body"],"additionalProperties":false}},"required":["id","response"]}},
        {"name":"verification-bundle", "description":"Download public verified sources, confirmed target, pinned toolchain, attribution and receipt for reuse. Requires a current public target.", "inputSchema":{"type":"object","properties":{"record":string("WP record"),"claim":string("Claim id"),"attempt":string("Verified attempt id")},"required":["record","claim","attempt"]}},
        {"name":"proof-work","description":"Read the current goal and finite public formalization allowance; no private execution data.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"}},"required":["record","claim"],"additionalProperties":false}},
        {"name":"control-proof-work","description":"Submitting author authorizes 1–20 lifetime generations, an expiry within 30 days, and pause/resume. Control requires exact target_digest, total_generations, expires_at and paused. Existing limits or a corrected target binding change only with change_allowance: true; pause/resume retains exact saved limits and never resets consumption.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"},"control":{"type":"object"}},"required":["record","claim","control"],"additionalProperties":false}},
        {"name":"checked-propositions","description":"Read independently checked named auxiliary propositions with exact sources, original contributors, licenses and receipt scope. Final target may be unfinished.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"}},"required":["record","claim"],"additionalProperties":false}},
        {"name":"contribute-proposition","description":"Queue a named proposition through the ordinary independent verifier. Input requires expected_target and intermediate {name,title,proposition,license}, plus self-contained modules or an owned deposit. Never accepts a success receipt.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"},"proposition":{"type":"object"}},"required":["record","claim","proposition"],"additionalProperties":false}},
        {"name":"reuse-proof","description":"Submit a proof using 1–8 checked proposition attempt identities under contributions, expected_target and solution, with optional modules. Copies exact original source modules and retains attribution/licenses. Checked-use links require actual proof inspection.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"},"proof":{"type":"object"}},"required":["record","claim","proof"],"additionalProperties":false}},
        {"name":"result-uses","description":"Read forward/inverse checked contribution use for exact public pinned results. Citations and unused imports do not create edges; complete joint context is retained.","inputSchema":{"type":"object","properties":{"record":{"type":"string"},"claim":{"type":"string"}},"required":["record","claim"],"additionalProperties":false}},
        {"name":"contribution-bundle","description":"Download exact public checked proposition, source, attribution and receipt; stale/private endpoints are hidden.","inputSchema":{"type":"object","properties":{"attempt":{"type":"string"}},"required":["attempt"],"additionalProperties":false}},
        {"name":"submit-formalization", "description":"Deposit an existing Lean module with original authors, source and license. Receipt means stored, not verified. A later verification request uses the independent verifier.", "inputSchema":{"type":"object","properties":{"title":string("Title"),"authors":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":50},"statement":string("Original mathematical statement"),"source_reference":{"type":"object","properties":{"kind":{"enum":["url","doi","arxiv"]},"locator":{"type":"string"}},"required":["kind","locator"]},"toolchain":string("Exact confirmed target toolchain"),"license":string("Source license"),"solution":string("Optional Solution adapter, up to 1 MiB; empty for source-only project deposits"),"modules":{"type":"object","maxProperties":512,"additionalProperties":{"type":"string"},"description":"Dotted module names preserving directories, up to 512 modules / 8 MiB total source"},"project_files":{"type":"object","additionalProperties":{"type":"string"},"description":"Inert lean-toolchain, lake-manifest.json, lakefile.lean/toml, README.md and LICENSE files, up to 1 MiB"},"formalizers":{"type":"array","maxItems":50,"items":{"type":"string"}},"source_revision":string("Optional full 40 or 64 digit Git commit hash")},"required":["title","authors","statement","source_reference","toolchain","license"]}},
        {"name":"preflight-formalization", "description":"Static compatibility, imports, version and dependency checks before deposit or verification. Never a proof receipt.","inputSchema":{"type":"object","properties":{"project":{"type":"object","description":"Same complete JSON body as submit-formalization"}},"required":["project"]}},
        {"name":"formalization", "description":"Read your deposited formalization and linked verification attempt ids.","inputSchema":{"type":"object","properties":{"id":string("Deposit id")},"required":["id"]}},
        {"name":"verify-formalization", "description":"Queue a stored Lean module against a public author-confirmed target. Requires the same toolchain; this does not assert success.","inputSchema":{"type":"object","properties":{"id":string("Deposit id"),"record":string("WP record"),"claim":string("Claim id"),"expected_target":string("Digest of the exact target you reviewed; recommended to reject stale requests"),"solution":string("Optional complete Solution.lean adapter; original modules remain unchanged")},"required":["id","record","claim"]}},
        { "name": "report", "description": "Report an error in an accepted statement, target or review. Evidence is text up to 20000 characters and/or a Lean disproof. Reports remain private until confirmed.", "inputSchema": { "type": "object", "properties": { "record": string("WP record"), "subject": { "oneOf": [{"type":"object","properties":{"kind":{"const":"review"}},"required":["kind"]},{"type":"object","properties":{"kind":{"const":"statement"},"claim":{"type":"string"}},"required":["kind","claim"]},{"type":"object","properties":{"kind":{"const":"target"},"claim":{"type":"string"},"digest":{"type":"string"}},"required":["kind","claim","digest"]}] }, "text": string("Evidence and argument"), "solution": string("Optional complete Lean disproof"), "as_agent": string("Owned agent name or id") }, "required": ["record", "subject"] } },
        { "name": "report-status", "description": "Read your error report and its audit result.", "inputSchema": { "type": "object", "properties": { "id": string("Report id") }, "required": ["id"] } },
        { "name": "conjectures", "description": "List audited public conjectures and their target status.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "target", "description": "Read the exact author-confirmed Target.lean and digest for a conjecture or approved paper statement. Save target.lean as Target.lean in your Lean project.", "inputSchema": { "type": "object", "properties": { "record": string("WP record"), "claim": string("Claim id") }, "required": ["record", "claim"] } },
        { "name": "attempt", "description": "Submit a Lean module importing Target. Use wishpool_solution or wishpool_disproof; only verifier-passed Lean counts.", "inputSchema": { "type": "object", "properties": { "record": string("WP record"), "claim": string("Claim id"), "solution": string("Complete Lean file, at most 1 MB"), "as_agent": string("Optional owned agent name") }, "required": ["record", "claim", "solution"] } },
        { "name": "attempt-status", "description": "Read a queued attempt or verifier receipt.", "inputSchema": { "type": "object", "properties": { "id": string("Attempt id") }, "required": ["id"] } },
        {
            "name": "list_tasks",
            "description": "List open Mathesis tasks on papers whose authors invited contributors. Kinds: judge_escape (judge whether a statement carries new content), literature_check, probe (a conjecture of an accepted paper).",
            "inputSchema": { "type": "object", "properties": { "kind": string("Optional task kind"), "limit": { "type": "integer", "minimum": 1, "maximum": 100 } } }
        },
        {
            "name": "lease_task",
            "description": "Lease a task so no one else works on it. Judgement tasks lease for 2 hours, literature 4, probes 24.",
            "inputSchema": { "type": "object", "properties": { "task_id": string("Task id") }, "required": ["task_id"] }
        },
        {
            "name": "get_task_context",
            "description": "The task, the statement it targets, the statements it depends on, the paper's title and abstract, and the exact rules and submission format for its kind. Read this before working.",
            "inputSchema": { "type": "object", "properties": { "task_id": string("Task id") }, "required": ["task_id"] }
        },
        {
            "name": "submit_contribution",
            "description": "Submit the result for a leased task. `output` must follow the format given by get_task_context. Report your model and, if known, token usage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": string("Task id"),
                    "model": string("The model that did the work, e.g. claude-opus-5-5"),
                    "tool": string("The agent, e.g. claude-code or codex"),
                    "output": { "type": "object", "description": "The kind-specific result" },
                    "input_tokens": { "type": "integer", "minimum": 0 },
                    "output_tokens": { "type": "integer", "minimum": 0 }
                },
                "required": ["task_id", "model", "output"]
            }
        },
        {
            "name": "release_task",
            "description": "Give a leased task back without submitting.",
            "inputSchema": { "type": "object", "properties": { "task_id": string("Task id") }, "required": ["task_id"] }
        }
    ]);
    feedback_tools.as_array_mut().unwrap().push(json!({"name":"feedback-observation","description":"Read checksummed raw observation evidence as an authorized rights holder or author/staff","inputSchema":{"type":"object","properties":{"id":{"type":"string"},"key":{"type":"string"},"request":{"type":"string"}},"required":["id","key","request"]}}));
    common
        .as_array_mut()
        .unwrap()
        .extend(binding_tools.as_array().unwrap().iter().cloned());
    common
        .as_array_mut()
        .unwrap()
        .extend(feedback_tools.as_array().unwrap().iter().cloned());
    common
        .as_array_mut()
        .unwrap()
        .extend(manuscript_tools.as_array().unwrap().iter().cloned());
    common
}

fn call(client: &Client, name: &str, args: &Value) -> Result<Value, String> {
    let task = || {
        args["task_id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "task_id is required".to_owned())
    };
    let required = |key: &str| {
        args[key]
            .as_str()
            .ok_or_else(|| format!("{key} is required"))
    };
    let result = match name {
        "prepare-submission" => {
            use base64::Engine;
            let encoded = required("source_base64")?;
            if encoded.len() > 41943040 { return Err("manuscript exceeds 30 MiB".into()); }
            let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|_| "invalid source_base64")?;
            client.prepare_submission(&args["metadata"], required("filename")?, &bytes)
        },
        "submission" => client.submission(required("id")?),
        "submit-prepared" => client.submit_prepared(required("id")?, &args["confirmation"]),
        "deliver-manuscript" => {
            use base64::Engine;
            let encoded = required("source_base64")?;
            if encoded.len() > 41943040 { return Err("manuscript exceeds 30 MiB".into()); }
            let source = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|_| "invalid source_base64".to_owned())?;
            client.deliver_manuscript(&args["metadata"], required("filename")?, &source).and_then(|receipt| {
                match args.get("confirmations") { Some(claims) => client.confirm_delivered_manuscript(receipt, claims), None => Ok(receipt) }
            })
        },
        "manuscript-status" => client.manuscript_status(required("id")?, args["version"].as_u64()),
        "find-manuscript" => client.find_manuscript(required("producer")?, required("work_id")?, required("purpose")?),
        "confirm-manuscript" => client.confirm_manuscript(required("id")?, &args["confirmation"]),
        "formalization-opportunities" => client.formalization_opportunities(args),
        "formalizations" => client.get("/formalizations"),
        "formalization-project" => client.formalization_project(required("id")?),
        "papers" => client.papers(args),
        "paper" => client.paper(required("record")?),
        "review" => client.review(required("id")?),
        "feedback-observation"=>client.feedback_observation(required("id")?,required("key")?,required("request")?),
        "feedbacks"=>client.feedbacks(required("id")?),
        "feedback"=>client.feedback(required("id")?,required("key")?),
        "preview-feedback"=>client.preview_feedback(required("id")?,&args["route"]),
        "prepare-feedback"=>client.prepare_feedback(required("id")?,&args["proposal"]),
        "authorize-feedback"=>client.authorize_feedback(required("id")?,required("key")?,&args["authorization"]),
        "revoke-feedback"=>client.revoke_feedback(required("id")?,required("key")?,required("reason")?),
        "export-feedback"=>client.export_feedback(required("id")?,required("key")?),
        "feedback-readback"=>client.feedback_readback(required("id")?,required("key")?,&args["observation"]),
        "knowledge-capabilities" => client.knowledge_capabilities(),
        "binding-profile" => client.binding_profile(required("person")?,args["before"].as_str()),
        "bindings" => client.bindings(args["before"].as_str()),
        "private-bindings" => client.private_bindings(required("id")?),
        "binding" => client.binding(required("id")?,required("key")?),
        "publish-binding" => client.publish_binding(required("id")?,&args["summary"]),
        "revoke-binding" => client.revoke_binding(required("id")?,required("key")?,required("reason")?),
        "contribute-binding" => client.contribute_binding(required("id")?,required("key")?,&args["contribution"]),
        "challenge-binding" => client.challenge_binding(required("id")?,required("key")?,&args["challenge"]),
        "research" => client.research(required("id")?),
        "adopt-research" => client.adopt_research(required("id")?, &args["obligation"]),
        "stop-research" => client.stop_research(required("id")?, required("key")?, required("reason")?),
        "research-attempt" => client.research_attempt(required("id")?, required("key")?, &args["attempt"]),
        "research-attempt-status" => client.research_attempt_status(required("id")?, required("key")?, required("attempt")?),
        "recheck-research" => client.recheck_research(required("id")?, required("key")?, required("attempt")?),
        "knowledge-response" => client.knowledge_response(required("id")?, required("receipt")?),
        "respond-review" => client.respond_review(required("id")?, &args["response"]),
        "proof-work" => client.proof_work(required("record")?, required("claim")?),
        "control-proof-work" => client.control_proof_work(required("record")?, required("claim")?, &args["control"]),
        "checked-propositions" => client.checked_propositions(required("record")?, required("claim")?),
        "contribute-proposition" => client.contribute_proposition(required("record")?, required("claim")?, &args["proposition"]),
        "reuse-proof" => client.reuse_proof(required("record")?, required("claim")?, &args["proof"]),
        "result-uses" => client.result_uses(required("record")?, required("claim")?),
        "contribution-bundle" => client.contribution_bundle(required("attempt")?),
        "verification-bundle" => client.verification_bundle(required("record")?, required("claim")?, required("attempt")?),
        "submit-formalization" => client.post("/formalizations", args),
        "preflight-formalization" => client.post("/formalizations/preflight", &args["project"]),
        "formalization" => client.formalization(required("id")?),
        "verify-formalization" => client.verify_formalization_project(required("id")?, required("record")?, required("claim")?, args["expected_target"].as_str(), args["solution"].as_str()),
        "report" => client.report(required("record")?, &json!({"subject": args["subject"], "text": args["text"].as_str().unwrap_or(""), "solution":args["solution"], "as_agent":args["as_agent"]})),
        "report-status" => client.report_status(required("id")?),
        "conjectures" => client.conjectures(),
        "target" => client.target(required("record")?, required("claim")?),
        "attempt" => client.attempt(
            required("record")?,
            required("claim")?,
            required("solution")?,
            args["as_agent"].as_str(),
        ),
        "attempt-status" => client.attempt_status(required("id")?),
        "list_tasks" => client.open_tasks(
            args["kind"].as_str(),
            args["limit"].as_u64().unwrap_or(20).min(100) as u32,
        ),
        "lease_task" => client.lease(&task()?),
        "get_task_context" => client.context(&task()?),
        "release_task" => client.release(&task()?),
        "submit_contribution" => {
            let tokens = match (
                args["input_tokens"].as_u64(),
                args["output_tokens"].as_u64(),
            ) {
                (None, None) => Value::Null,
                (input, output) => {
                    json!({ "input": input.unwrap_or(0), "output": output.unwrap_or(0), "metered": false })
                }
            };
            let body = json!({
                "agent": { "tool": args["tool"].as_str().unwrap_or("mcp-agent"), "model": args["model"] },
                "output": args["output"],
                "tokens": tokens,
            });
            client.submit(&task()?, &body)
        }
        other => return Err(format!("unknown tool {other}")),
    };
    result.map_err(|e| e.to_string())
}

/// Handle one JSON-RPC message; `None` for notifications.
pub fn handle(client: &Client, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned()?;
    let method = message["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "mathesis-contribute", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Search papers, read your private review, deposit Lean formalizations, and contribute verified attempts. A stored source is not a verified proof. Never invent citations, receipts or witnesses."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = message["params"]["name"].as_str().unwrap_or_default();
            let args = &message["params"]["arguments"];
            Ok(match call(client, name, args) {
                Ok(value) => {
                    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }], "isError": false })
                }
                Err(error) => {
                    json!({ "content": [{ "type": "text", "text": error }], "isError": true })
                }
            })
        }
        _ => Err(json!({ "code": -32601, "message": format!("method not found: {method}") })),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    })
}

pub fn serve(client: &Client) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(message) => handle(client, &message),
            Err(error) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": error.to_string() } }),
            ),
        };
        if let Some(response) = response {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
