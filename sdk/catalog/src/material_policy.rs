//! Pure decisions for the hosted paper-material queue. Transports, rendering and
//! optimistic ledger writes stay with the caller; this module performs no I/O.
use super::*;

pub const SCHEMA: &str = "mathesis.material-policy.v1";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Intent,
    Provisioning,
    Authoring,
    SourcesReady,
    SourcesSaved,
    PublicationPending,
    Published,
    NeedsAttention,
}

#[derive(Debug, Deserialize)]
pub struct Job {
    pub phase: Phase,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub id: String,
    pub source_archive_sha256: String,
    pub toolchain_image: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Admission {
        jobs: Vec<Job>,
        capacity: u32,
    },
    ResumeBuild {
        state: Value,
        recovery: Option<Recovery>,
        toolchain_image: String,
        observed_at: String,
    },
    AdaptTemplate {
        state: Value,
        template_commit: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub schema: String,
    #[serde(flatten)]
    pub action: Action,
}

fn require(condition: bool, reason: &str) -> Result<()> {
    if condition { Ok(()) } else { Err(msg(reason)) }
}

fn phase(state: &Value) -> Result<Phase> {
    Ok(serde_json::from_value(
        state
            .get("phase")
            .cloned()
            .ok_or_else(|| msg("missing material phase"))?,
    )?)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn nonempty(state: &Value, field: &str) -> bool {
    state
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty())
}

fn append(state: &mut Value, field: &str, item: Value) -> Result<()> {
    let history = state
        .as_object_mut()
        .ok_or_else(|| msg("material state must be an object"))?
        .entry(field)
        .or_insert_with(|| json!([]));
    history
        .as_array_mut()
        .ok_or_else(|| msg(format!("{field} must be an array")))?
        .push(item);
    Ok(())
}

fn transition(state: Value, changed: bool) -> Value {
    json!({"schema": SCHEMA, "changed": changed, "state": state})
}

pub fn decide(request: Request) -> Result<Value> {
    require(
        request.schema == SCHEMA,
        "unsupported material policy schema",
    )?;
    match request.action {
        Action::Admission { jobs, capacity } => {
            require(
                capacity > 0,
                "authoring concurrency must be a positive integer",
            )?;
            let capacity = capacity as usize;
            let in_flight = jobs
                .iter()
                .filter(|job| {
                    matches!(
                        job.phase,
                        Phase::Intent
                            | Phase::Provisioning
                            | Phase::Authoring
                            | Phase::SourcesReady
                    )
                })
                .count();
            let buffered = jobs
                .iter()
                .filter(|job| job.phase == Phase::SourcesSaved)
                .count();
            let slots = if buffered >= capacity {
                0
            } else {
                capacity.saturating_sub(in_flight)
            };
            Ok(
                json!({"schema": SCHEMA, "in_flight": in_flight, "buffered": buffered, "slots": slots}),
            )
        }
        Action::ResumeBuild {
            mut state,
            recovery,
            toolchain_image,
            observed_at,
        } => {
            let current_phase = phase(&state)?;
            let Some(recovery) = recovery else {
                return Ok(transition(state, false));
            };
            if current_phase != Phase::NeedsAttention {
                return Ok(transition(state, false));
            }
            let history = match state.get("build_recovery_history") {
                Some(value) => value
                    .as_array()
                    .ok_or_else(|| msg("build_recovery_history must be an array"))?
                    .as_slice(),
                None => &[],
            };
            if history
                .iter()
                .any(|entry| entry.get("id").and_then(Value::as_str) == Some(&recovery.id))
            {
                return Ok(transition(state, false));
            }
            require(
                !recovery.id.trim().is_empty(),
                "build recovery needs a nonempty operation id",
            )?;
            require(
                state.get("blocked_stage").and_then(Value::as_str)
                    == Some("host_build_or_publication")
                    && hex(&recovery.source_archive_sha256, 64)
                    && state.get("source_archive_sha256").and_then(Value::as_str)
                        == Some(&recovery.source_archive_sha256)
                    && recovery.toolchain_image == toolchain_image,
                "saved build recovery does not match the exact source and toolchain",
            )?;
            time::OffsetDateTime::parse(
                &observed_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| msg("invalid material observation time"))?;
            let mut entry = serde_json::to_value(recovery)?;
            entry["resumed_at"] = json!(observed_at);
            entry["previous_stage"] = state["blocked_stage"].clone();
            append(&mut state, "build_recovery_history", entry)?;
            state["phase"] = json!("sources_saved");
            state
                .as_object_mut()
                .expect("phase checked an object")
                .remove("blocked_stage");
            Ok(transition(state, true))
        }
        Action::AdaptTemplate {
            mut state,
            template_commit,
        } => {
            let current_phase = phase(&state)?;
            require(
                hex(&template_commit, 40),
                "template adaptation needs an immutable commit",
            )?;
            let previous = state
                .get("last_published_template")
                .unwrap_or(&state["identity"]["template_commit"]);
            if current_phase != Phase::Published || previous.as_str() == Some(&template_commit) {
                return Ok(transition(state, false));
            }
            require(
                nonempty(&state, "source_archive_sha256") && nonempty(&state, "published_commit"),
                "template adaptation requires saved checked output",
            )?;
            let history: Map<String, Value> = [
                "published_commit",
                "expected_manifest",
                "publication_intent",
                "build_receipt",
            ]
            .into_iter()
            .filter_map(|field| {
                state
                    .get(field)
                    .map(|value| (field.to_owned(), value.clone()))
            })
            .collect();
            append(&mut state, "publication_history", Value::Object(history))?;
            state["publication_base"] = state["published_commit"].clone();
            state["template_adaptation"] = json!({"template_commit": template_commit,
                "reused_source_archive_sha256": state["source_archive_sha256"]});
            state
                .as_object_mut()
                .expect("phase checked an object")
                .remove("publication_intent");
            state["phase"] = json!("sources_saved");
            Ok(transition(state, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(action: Value) -> Result<Value> {
        let mut request = action;
        request["schema"] = json!(SCHEMA);
        decide(serde_json::from_value(request)?)
    }

    fn failed() -> Value {
        json!({"key": "original-job", "phase": "needs_attention", "blocked_stage": "host_build_or_publication",
            "source_archive_sha256": "a".repeat(64), "agent_id": "original-agent", "unknown_receipt": {"bytes": [1,2,3]}})
    }

    fn recovery(state: Value) -> Value {
        json!({"action": "resume_build", "state": state, "observed_at": "2026-11-23T09:00:00Z",
            "toolchain_image": "pinned-image", "recovery": {"id": "checked-repair", "source_archive_sha256": "a".repeat(64),
                "toolchain_image": "pinned-image"}})
    }

    #[test]
    fn recovery_preserves_identity_receipts_and_is_consumed_only_once() {
        let original = failed();
        let result = run(recovery(original.clone())).unwrap();
        assert_eq!(result["changed"], true);
        let mut resumed = result["state"].clone();
        assert_eq!(resumed["phase"], "sources_saved");
        assert_eq!(resumed["agent_id"], original["agent_id"]);
        assert_eq!(resumed["unknown_receipt"], original["unknown_receipt"]);
        assert!(resumed.get("blocked_stage").is_none());
        resumed["phase"] = json!("needs_attention");
        resumed["blocked_stage"] = json!("host_build_or_publication");
        let repeated = run(recovery(resumed.clone())).unwrap();
        assert_eq!(repeated["changed"], false);
        assert_eq!(repeated["state"], resumed);
    }

    #[test]
    fn recovery_refuses_changed_input_and_provider_failures_without_mutating_state() {
        for field in ["source_archive_sha256", "blocked_stage"] {
            let mut state = failed();
            state[field] = json!("different");
            assert!(
                run(recovery(state))
                    .unwrap_err()
                    .to_string()
                    .contains("exact source")
            );
        }
        let mut input = recovery(failed());
        input["toolchain_image"] = json!("different-image");
        assert!(run(input).is_err());
        let mut input = recovery(failed());
        input["recovery"]["id"] = json!("");
        assert!(run(input).is_err());
        let mut input = recovery(failed());
        input["observed_at"] = json!("tomorrow");
        assert!(run(input).is_err());
    }

    #[test]
    fn running_or_published_jobs_are_never_restarted_by_build_recovery() {
        for phase in [
            "intent",
            "provisioning",
            "authoring",
            "sources_ready",
            "sources_saved",
            "publication_pending",
            "published",
        ] {
            let mut state = failed();
            state["phase"] = json!(phase);
            let result = run(recovery(state.clone())).unwrap();
            assert_eq!(result["state"], state);
            assert_eq!(result["changed"], false);
        }
    }

    #[test]
    fn admission_bounds_remote_work_and_saved_output_independently() {
        for (phases, expected) in [
            (
                vec!["intent", "provisioning", "authoring", "sources_ready"],
                0,
            ),
            (vec!["sources_saved"; 4], 0),
            (
                vec!["published", "needs_attention", "publication_pending"],
                4,
            ),
            (vec!["authoring", "sources_saved"], 3),
            (vec!["authoring"; 5], 0),
        ] {
            let jobs: Vec<_> = phases
                .into_iter()
                .map(|phase| json!({"phase": phase}))
                .collect();
            assert_eq!(
                run(json!({"action": "admission", "jobs": jobs, "capacity": 4})).unwrap()["slots"],
                expected
            );
        }
        for capacity in [json!(0), json!(-1), json!(true), json!(1.5)] {
            assert!(run(json!({"action": "admission", "jobs": [], "capacity": capacity})).is_err());
        }
        assert!(run(json!({"action": "admission", "jobs": [{"phase": "new_provider_status"}], "capacity": 4})).is_err());
    }

    #[test]
    fn template_adaptation_keeps_the_original_mathematical_job_and_publication_history() {
        let state = json!({"phase": "published", "identity": {"template_commit": "a".repeat(40)},
            "key": "immutable", "agent_id": "original-agent", "source_archive_sha256": "b".repeat(64),
            "published_commit": "c".repeat(40), "publication_intent": {"source_tree": {"proof": "old"}},
            "build_receipt": {"checked": true}});
        let result = run(
            json!({"action": "adapt_template", "state": state, "template_commit": "d".repeat(40)}),
        )
        .unwrap();
        let updated = &result["state"];
        assert_eq!(updated["phase"], "sources_saved");
        assert_eq!(updated["identity"], state["identity"]);
        assert_eq!(updated["agent_id"], state["agent_id"]);
        assert_eq!(updated["publication_base"], state["published_commit"]);
        assert_eq!(
            updated["publication_history"][0]["publication_intent"],
            state["publication_intent"]
        );
        assert_eq!(
            updated["publication_history"][0]["build_receipt"],
            state["build_receipt"]
        );
        assert!(updated.get("publication_intent").is_none());
        assert_eq!(run(json!({"action": "adapt_template", "state": updated, "template_commit": "d".repeat(40)})).unwrap()["changed"], false);
        let mut missing = state.clone();
        missing
            .as_object_mut()
            .unwrap()
            .remove("source_archive_sha256");
        assert!(run(json!({"action": "adapt_template", "state": missing, "template_commit": "d".repeat(40)})).is_err());
    }
}
