//! mathesis-contribute: give your idle model tokens to mathematics.
//!
//! ```text
//! MATHESIS_URL=https://mathesis.example.org MATHESIS_TOKEN=<NyxID access token> \
//!   mathesis-contribute tasks [kind]       open tasks
//!   mathesis-contribute show <task>        task, target statement and rules
//!   mathesis-contribute lease <task>       lease a task
//!   mathesis-contribute release <task>     give it back
//!   mathesis-contribute submit <task> <result.json>
//!   mathesis-contribute mcp                MCP server on stdio for your agent
//! ```
//!
//! Claude Code: `claude mcp add mathesis -e MATHESIS_NYXID_SERVICE=mathesis -- mathesis-contribute mcp`.

#![recursion_limit = "256"]

mod client;
mod mcp;
mod rules;

use std::process::ExitCode;

use client::Client;

fn setting(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(format!("MATHESIS_{name}")).or_else(|_| std::env::var(format!("WISHPOOL_{name}")))
}
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let client = if let Ok(service) = setting("NYXID_SERVICE") {
        if service.is_empty() || service.starts_with('-') {
            eprintln!(
                "MATHESIS_NYXID_SERVICE (or legacy WISHPOOL_NYXID_SERVICE) must name a connected service"
            );
            return ExitCode::from(2);
        }
        Client::via_nyxid(&service)
    } else if let Ok(url) = setting("URL") {
        Client::new(&url, &setting("TOKEN").unwrap_or_default())
    } else {
        eprintln!(
            "set MATHESIS_NYXID_SERVICE for a connected NyxID service, or MATHESIS_URL (and MATHESIS_TOKEN for private operations); legacy WISHPOOL variables are also supported"
        );
        return ExitCode::from(2);
    };
    let print = |result: client::ApiResult<serde_json::Value>| match result {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    };
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["prepare-submission", metadata, source] => print((|| {
            let metadata = read_json(metadata).map_err(|detail| client::ApiError {
                status: 422,
                detail,
            })?;
            let bytes = std::fs::read(source).map_err(|e| client::ApiError {
                status: 422,
                detail: e.to_string(),
            })?;
            let filename = std::path::Path::new(source)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| client::ApiError {
                    status: 422,
                    detail: "invalid source filename".into(),
                })?;
            client.prepare_submission(&metadata, filename, &bytes)
        })()),
        ["submission", id] => print(client.submission(id)),
        ["submit-prepared", id, file] => print(
            read_json(file)
                .map_err(|detail| client::ApiError {
                    status: 422,
                    detail,
                })
                .and_then(|body| client.submit_prepared(id, &body)),
        ),
        ["deliver-manuscript", metadata, source] => {
            print(deliver_manuscript(&client, metadata, source, None))
        }
        ["deliver-manuscript", metadata, source, "--confirm", claims] => {
            print(deliver_manuscript(&client, metadata, source, Some(claims)))
        }
        ["manuscript-status", id] => print(client.manuscript_status(id, None)),
        ["find-manuscript", producer, work, purpose] => {
            print(client.find_manuscript(producer, work, purpose))
        }
        ["confirm-manuscript", id, file] => match read_json(file) {
            Ok(body) => print(client.confirm_manuscript(id, &body)),
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(2)
            }
        },
        ["formalization-opportunities"] => {
            print(client.formalization_opportunities(&serde_json::json!({})))
        }
        ["formalization-opportunities", query] => {
            print(client.formalization_opportunities(&serde_json::json!({"q": query})))
        }
        ["formalizations"] => print(client.get("/formalizations")),
        ["formalization-project", id] => print(client.formalization_project(id)),
        ["papers"] => print(client.papers(&serde_json::json!({}))),
        ["papers", query] => print(client.papers(&serde_json::json!({"q": query}))),
        ["paper", record] => print(client.paper(record)),
        ["review", id] => print(client.review(id)),
        ["feedback-observation", id, key, request] => {
            print(client.feedback_observation(id, key, request))
        }
        ["feedbacks", id] => print(client.feedbacks(id)),
        ["feedback", id, key] => print(client.feedback(id, key)),
        ["export-feedback", id, key] => print(client.export_feedback(id, key)),
        ["revoke-feedback", id, key, reason] => print(client.revoke_feedback(id, key, reason)),
        [
            command @ ("preview-feedback" | "prepare-feedback"),
            id,
            file,
        ] => match read_json(file) {
            Ok(body) => print(if *command == "preview-feedback" {
                client.preview_feedback(id, &body)
            } else {
                client.prepare_feedback(id, &body)
            }),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        [
            command @ ("authorize-feedback" | "feedback-readback"),
            id,
            key,
            file,
        ] => match read_json(file) {
            Ok(body) => print(if *command == "authorize-feedback" {
                client.authorize_feedback(id, key, &body)
            } else {
                client.feedback_readback(id, key, &body)
            }),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["knowledge-capabilities"] => print(client.knowledge_capabilities()),
        ["binding-profile", person] => print(client.binding_profile(person, None)),
        ["binding-profile", person, before] => print(client.binding_profile(person, Some(before))),
        ["bindings"] => print(client.bindings(None)),
        ["bindings", before] => print(client.bindings(Some(before))),
        ["private-bindings", id] => print(client.private_bindings(id)),
        ["binding", id, key] => print(client.binding(id, key)),
        ["revoke-binding", id, key, reason] => print(client.revoke_binding(id, key, reason)),
        ["publish-binding", id, file] => match read_json(file) {
            Ok(body) => print(client.publish_binding(id, &body)),
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        [
            command @ ("contribute-binding" | "challenge-binding"),
            id,
            key,
            file,
        ] => match read_json(file) {
            Ok(body) => print(if *command == "contribute-binding" {
                client.contribute_binding(id, key, &body)
            } else {
                client.challenge_binding(id, key, &body)
            }),
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        ["research", id] => print(client.research(id)),
        ["stop-research", id, key, reason] => print(client.stop_research(id, key, reason)),
        ["research-attempt-status", id, key, attempt] => {
            print(client.research_attempt_status(id, key, attempt))
        }
        ["recheck-research", id, key, attempt] => print(client.recheck_research(id, key, attempt)),
        ["adopt-research", id, file] => match read_json(file) {
            Ok(body) => print(client.adopt_research(id, &body)),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["research-attempt", id, key, file] => match read_json(file) {
            Ok(body) => print(client.research_attempt(id, key, &body)),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["knowledge-response", id, receipt] => print(client.knowledge_response(id, receipt)),
        ["proof-work", record, claim] => print(client.proof_work(record, claim)),
        ["checked-propositions", record, claim] => {
            print(client.checked_propositions(record, claim))
        }
        ["result-uses", record, claim] => print(client.result_uses(record, claim)),
        ["contribution-bundle", attempt] => print(client.contribution_bundle(attempt)),
        [
            command @ ("control-proof-work" | "contribute-proposition" | "reuse-proof"),
            record,
            claim,
            file,
        ] => match read_json(file) {
            Ok(body) => print(match *command {
                "control-proof-work" => client.control_proof_work(record, claim, &body),
                "contribute-proposition" => client.contribute_proposition(record, claim, &body),
                _ => client.reuse_proof(record, claim, &body),
            }),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["verification-bundle", record, claim, attempt] => {
            print(client.verification_bundle(record, claim, attempt))
        }
        ["respond-review", id, file] => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(body) => print(client.respond_review(id, &body)),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["formalization", id] => print(client.formalization(id)),
        ["verify-formalization", id, record, claim] => {
            print(client.verify_formalization(id, record, claim))
        }
        ["preflight-formalization", file] => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
        {
            Ok(body) => print(client.post("/formalizations/preflight", &body)),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["verify-formalization", id, record, claim, adapter] => {
            match std::fs::read_to_string(adapter) {
                Ok(source) => print(client.verify_formalization_project(
                    id,
                    record,
                    claim,
                    None,
                    Some(&source),
                )),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        ["submit-formalization", file] => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(body) => print(client.post("/formalizations", &body)),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        ["conjectures"] => print(client.conjectures()),
        ["target" | "statement-target", record, claim] => match client.target(record, claim) {
            Ok(value) => match value["target"]["lean"].as_str() {
                Some(lean) => match std::fs::write("Target.lean", lean) {
                    Ok(()) => {
                        println!("Wrote Target.lean (digest {})", value["target"]["digest"]);
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                },
                None => {
                    eprintln!("the author has not confirmed a target");
                    ExitCode::FAILURE
                }
            },
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        ["report", record, file] => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
        {
            Ok(body) => print(client.report(record, &body)),
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(2)
            }
        },
        ["report-status", id] => print(client.report_status(id)),
        ["attempt-status", id] => print(client.attempt_status(id)),
        ["attempt" | "statement-attempt", record, claim, file] => {
            submit_attempt(&client, record, claim, file, None)
        }
        [
            "attempt" | "statement-attempt",
            record,
            claim,
            file,
            "--as-agent",
            name,
        ] => submit_attempt(&client, record, claim, file, Some(name)),
        ["tasks"] => print(client.open_tasks(None, 50)),
        ["tasks", kind] => print(client.open_tasks(Some(kind), 50)),
        ["show", task] => print(client.context(task)),
        ["lease", task] => print(client.lease(task)),
        ["release", task] => print(client.release(task)),
        ["submit", task, file] => match std::fs::read_to_string(file)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))
        {
            Ok(body) => print(client.submit(task, &body)),
            Err(error) => {
                eprintln!("cannot read {file}: {error}");
                ExitCode::from(2)
            }
        },
        ["mcp"] => match mcp::serve(&client) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("mcp: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "usage: mathesis-contribute prepare-submission <preparation.json> <source> | submission <id> | submit-prepared <id> <confirmation.json> | deliver-manuscript <metadata.json> <source> [--confirm claims.json] | manuscript-status <id> | find-manuscript <producer> <work-id> <publication|review_only> | confirm-manuscript <id> <confirmation.json> | formalization-opportunities [query] | formalizations | formalization-project <id> | papers [query] | paper <record> | review <submission-id> | feedbacks <submission> | feedback <submission> <proposal> | preview-feedback <submission> <route.json> | prepare-feedback <submission> <proposal.json> | authorize-feedback <submission> <proposal> <authorization.json> | revoke-feedback <submission> <proposal> <reason> | export-feedback <submission> <proposal> | feedback-readback <submission> <proposal> <observation.json> | feedback-observation <submission> <proposal> <request> | knowledge-capabilities | binding-profile <person> [cursor] | bindings [cursor] | private-bindings <submission-id> | binding <submission-id> <board-id> | publish-binding <submission-id> <summary.json> | revoke-binding <submission-id> <board-id> <reason> | contribute-binding <submission-id> <board-id> <binding.json> | challenge-binding <submission-id> <board-id> <challenge.json> | research <submission-id> | adopt-research <submission-id> <obligation.json> | research-attempt <submission-id> <obligation-id> <attempt.json> | research-attempt-status <submission-id> <obligation-id> <attempt-id> | recheck-research <submission-id> <obligation-id> <attempt-id> | stop-research <submission-id> <obligation-id> <reason> | knowledge-response <submission-id> <receipt-id> | respond-review <submission-id> <response.json> | verification-bundle <record> <claim> <attempt> | proof-work <record> <claim> | control-proof-work <record> <claim> <authorization.json> | checked-propositions <record> <claim> | contribute-proposition <record> <claim> <proposition.json> | reuse-proof <record> <claim> <attempt.json> | result-uses <record> <claim> | contribution-bundle <attempt> | preflight-formalization <deposit.json> | submit-formalization <deposit.json> | formalization <id> | verify-formalization <id> <record> <claim> [Solution.lean] | conjectures | target <record> <claim> | attempt <record> <claim> <file> [--as-agent NAME] | attempt-status <id> | report <record> <report.json> | report-status <id> | tasks [kind] | show <task> | lease <task> | release <task> | submit <task> <result.json> | mcp"
            );
            ExitCode::from(2)
        }
    }
}

fn deliver_manuscript(
    client: &Client,
    metadata: &str,
    source: &str,
    confirmations: Option<&str>,
) -> client::ApiResult<serde_json::Value> {
    let invalid = |detail: String| client::ApiError { status: 0, detail };
    let metadata = read_json(metadata).map_err(invalid)?;
    let claims = confirmations.map(read_json).transpose().map_err(invalid)?;
    let path = std::path::Path::new(source);
    if std::fs::metadata(path)
        .map_err(|e| invalid(e.to_string()))?
        .len()
        > 30 * 1024 * 1024
    {
        return Err(invalid("Manuscript source exceeds 30 MiB".into()));
    }
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| invalid("Invalid manuscript filename".into()))?;
    let source = std::fs::read(path).map_err(|e| invalid(e.to_string()))?;
    let receipt = client.deliver_manuscript(&metadata, filename, &source)?;
    match claims {
        Some(claims) => client.confirm_delivered_manuscript(receipt, &claims),
        None => Ok(receipt),
    }
}

fn submit_attempt(
    client: &Client,
    record: &str,
    claim: &str,
    file: &str,
    agent: Option<&str>,
) -> ExitCode {
    let result = std::fs::read_to_string(file)
        .map_err(|e| e.to_string())
        .and_then(|s| {
            if s.len() > 1_048_576 {
                return Err("solution exceeds 1 MB".into());
            }
            client
                .attempt(record, claim, &s, agent)
                .map_err(|e| e.to_string())
        });
    match result {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json(file: &str) -> Result<serde_json::Value, String> {
    std::fs::read_to_string(file)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn mcp_lists_tools_and_rejects_unknown_methods() {
        let client = Client::new("http://127.0.0.1:9", "dev:test");
        let init = mcp::handle(
            &client,
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        )
        .unwrap();
        assert_eq!(init["result"]["capabilities"]["tools"], json!({}));
        let tools = mcp::handle(
            &client,
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        )
        .unwrap();
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "knowledge-capabilities",
                "research",
                "adopt-research",
                "stop-research",
                "research-attempt",
                "research-attempt-status",
                "recheck-research",
                "formalization-opportunities",
                "formalizations",
                "formalization-project",
                "papers",
                "paper",
                "review",
                "knowledge-response",
                "respond-review",
                "verification-bundle",
                "proof-work",
                "control-proof-work",
                "checked-propositions",
                "contribute-proposition",
                "reuse-proof",
                "result-uses",
                "contribution-bundle",
                "submit-formalization",
                "preflight-formalization",
                "formalization",
                "verify-formalization",
                "report",
                "report-status",
                "conjectures",
                "target",
                "attempt",
                "attempt-status",
                "list_tasks",
                "lease_task",
                "get_task_context",
                "submit_contribution",
                "release_task",
                "binding-profile",
                "bindings",
                "private-bindings",
                "binding",
                "publish-binding",
                "revoke-binding",
                "contribute-binding",
                "challenge-binding",
                "feedbacks",
                "feedback",
                "preview-feedback",
                "prepare-feedback",
                "authorize-feedback",
                "revoke-feedback",
                "export-feedback",
                "feedback-readback",
                "feedback-observation",
                "prepare-submission",
                "submission",
                "submit-prepared",
                "deliver-manuscript",
                "manuscript-status",
                "find-manuscript",
                "confirm-manuscript"
            ]
        );
        assert!(
            mcp::handle(
                &client,
                &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
            )
            .is_none()
        );
        let unknown = mcp::handle(
            &client,
            &json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
        )
        .unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
        // A tool error is reported in-band, not as a protocol error.
        let failed = mcp::handle(&client, &json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "lease_task", "arguments": {} } })).unwrap();
        assert_eq!(failed["result"]["isError"], true);
    }

    #[test]
    fn rules_cover_every_kind() {
        for kind in ["judge_escape", "literature_check", "probe"] {
            assert!(!rules::for_kind(kind).is_empty());
        }
    }
}
