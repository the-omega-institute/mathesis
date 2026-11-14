use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub identity: String,
    pub lineage: String,
    pub work: String,
    pub record: String,
    pub version: i64,
    pub claim: String,
    pub label: String,
    pub kind: String,
    pub title: String,
    pub statement: String,
    pub macros: BTreeMap<String, String>,
    pub status: String,
    pub target_digest: Option<String>,
    pub toolchain: Option<String>,
    pub receipt: Option<Value>,
    pub proof_basis: Option<Value>,
    pub authors: Vec<Value>,
    pub contributor: Option<Value>,
    pub attribution: Option<Value>,
    pub lean_source: Option<LeanSource>,
    pub proof_state: Option<String>,
    pub next_step: Option<String>,
    pub scope: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanSource {
    pub url: String,
    pub revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub work: String,
    pub repository_name: Option<String>,
    pub record: String,
    pub title: String,
    pub authors: Vec<Value>,
    pub license: Option<String>,
    pub publication: String,
    pub doi: Option<String>,
    pub concept_doi: Option<String>,
    pub versions: Vec<Value>,
    pub current_version: Option<i64>,
    pub abstract_text: Option<String>,
    pub lean_source: Option<LeanSource>,
    pub pdf: Option<Value>,
    pub source: Option<Value>,
    pub msc: Vec<String>,
    pub published_on: Option<String>,
    pub previous_version: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub snapshot_time: String,
    pub public_papers: usize,
    pub registered_statements: usize,
    pub independently_verified_statements: usize,
    pub verified_from_existing_library_declarations: usize,
    pub verified_with_new_lean: usize,
    pub verified_counterexamples_disproofs: usize,
    pub open_formalization_targets: usize,
    pub statements_awaiting_formal_target: usize,
    pub awaiting_target_needs_attention: usize,
    pub papers_with_lean_source: usize,
    pub papers_with_doi: usize,
    pub by_msc: BTreeMap<String, MscStats>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MscStats {
    pub papers: usize,
    pub registered: usize,
    pub verified: usize,
    pub open_targets: usize,
}

fn lean_source(v: &Value) -> Option<LeanSource> {
    let x = v.get("lean")?;
    Some(LeanSource {
        url: text(x, "url")?,
        revision: text(x, "revision")?,
    })
}
pub(crate) fn descriptor_hash(v: &Value) -> Option<String> {
    for key in ["sha256", "sha_256", "hash"] {
        if let Some(s) = v.get(key).and_then(Value::as_str) {
            return Some(s.to_ascii_lowercase());
        }
    }
    None
}
pub(crate) fn descriptor_url(v: &Value, base: &str) -> Option<String> {
    let raw = v.get("url").and_then(Value::as_str)?;
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_owned())
    } else {
        Some(format!(
            "{}{}",
            normalize_base(base),
            if raw.starts_with('/') {
                raw.to_owned()
            } else {
                format!("/{raw}")
            }
        ))
    }
}
fn current_version(v: &Value) -> Option<i64> {
    text(v, "revision")
        .and_then(|s| s.parse().ok())
        .or_else(|| v.get("version").and_then(Value::as_i64))
}
pub(crate) fn paper_work(
    v: &Value,
    consent: bool,
    mapping: &BTreeMap<String, String>,
) -> Result<Work> {
    let work = text(v, "id")
        .or_else(|| text(v, "record"))
        .ok_or_else(|| msg("paper has no work/record id"))?;
    let record = text(v, "record").unwrap_or_else(|| work.clone());
    let consent = consent && !accepted::archive_eligible(v);
    let license = text(v, "license");
    let in_scope = license.as_deref() == Some("CC-BY-4.0");
    let repository_name = if consent || !in_scope {
        None
    } else {
        Some(repository_name(&work, mapping))
    };
    Ok(Work {
        work: work.clone(),
        repository_name,
        record,
        title: text(v, "title").unwrap_or_default(),
        authors: arr(v, "authors"),
        license,
        publication: if consent {
            "requires_author_consent".into()
        } else if in_scope {
            "public".into()
        } else {
            "out_of_scope".into()
        },
        doi: text(v, "doi").or_else(|| v.get("archive").and_then(|x| text(x, "doi"))),
        concept_doi: text(v, "concept_doi")
            .or_else(|| v.get("archive").and_then(|x| text(x, "concept_doi"))),
        versions: vec![
            json!({"version": current_version(v), "revision": v.get("revision").cloned().unwrap_or(Value::Null), "published_on": v.get("published_on").cloned().unwrap_or(Value::Null), "previous_version": v.get("previous_version").cloned().unwrap_or(Value::Null)}),
        ],
        current_version: current_version(v),
        abstract_text: text(v, "abstract").or_else(|| text(v, "abstract_text")),
        lean_source: lean_source(v),
        pdf: v.get("pdf").cloned(),
        source: v.get("source").cloned(),
        msc: v
            .get("msc")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        published_on: text(v, "published_on"),
        previous_version: v.get("previous_version").cloned(),
    })
}
pub(crate) fn repository_name(work: &str, mapping: &BTreeMap<String, String>) -> String {
    if let Some(m) = mapping.get(work) {
        return m.clone();
    }
    if valid_repo_name(work) {
        work.to_owned()
    } else {
        format!("paper-{}", &sha256(work.as_bytes())[..12])
    }
}
pub(crate) fn valid_repo_name(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.len() <= 100
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
pub(crate) fn mapping_file(path: Option<&Path>) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let value = read_json(path)?;
    let obj = value
        .as_object()
        .ok_or_else(|| msg("repository mapping must be a JSON object"))?;
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        let n = v
            .as_str()
            .ok_or_else(|| msg("repository mapping values must be strings"))?;
        if !valid_repo_name(n) {
            return Err(msg(format!("invalid repository name in mapping: {n}")));
        }
        if out.values().any(|v: &String| v.eq_ignore_ascii_case(n)) {
            return Err(msg(format!("duplicate repository in mapping: {n}")));
        }
        out.insert(k.clone(), n.to_owned());
    }
    Ok(out)
}
fn macros(v: &Value) -> BTreeMap<String, String> {
    v.get("macros")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}
fn proof_for(snapshot: &Snapshot, work: &str, version: i64, claim: &str) -> Option<Value> {
    snapshot
        .proofs
        .iter()
        .find(|(w, _)| w == work)
        .and_then(|(_, v)| v.as_array())
        .and_then(|a| {
            a.iter().find(|p| {
                p.get("version").and_then(Value::as_i64) == Some(version)
                    && text(p, "claim").as_deref() == Some(claim)
            })
        })
        .cloned()
}
pub(crate) fn make_nodes(snapshot: &Snapshot) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut works: BTreeMap<String, (Value, String)> = snapshot
        .collected
        .iter()
        .filter_map(|v| {
            text(v, "id").map(|id| {
                let scope = if text(v, "license").as_deref() == Some("CC-BY-4.0") {
                    "in_scope"
                } else {
                    "out_of_scope"
                };
                (id, (v.clone(), scope.to_owned()))
            })
        })
        .collect();
    works.extend(snapshot.accepted.iter().filter_map(|v| {
        text(v, "record").map(|record| {
            let scope = if accepted::archive_eligible(v) {
                "in_scope"
            } else {
                "requires_author_consent"
            };
            (record, (v.clone(), scope.to_owned()))
        })
    }));
    for row in &snapshot.opportunities {
        let record = text(row, "record").ok_or_else(|| msg("formalization row has no record"))?;
        let version = row
            .get("version")
            .and_then(Value::as_i64)
            .ok_or_else(|| msg("formalization row has no version"))?;
        let claim = text(row, "claim").ok_or_else(|| msg("formalization row has no claim"))?;
        let work = record.clone();
        let (paper, scope) = works
            .get(&work)
            .cloned()
            .unwrap_or_else(|| (json!({}), "out_of_scope".to_owned()));
        let is_collected = snapshot
            .collected
            .iter()
            .any(|v| text(v, "id").as_deref() == Some(work.as_str()));
        let proof = is_collected
            .then(|| proof_for(snapshot, &work, version, &claim))
            .flatten();
        // The directory chooses the winning receipt. Collected proofs also lists
        // later checked attempts, which must not replace that winner's status.
        let status = if row["target_digest"].is_string() {
            text(row, "status").unwrap_or_else(|| "open".into())
        } else {
            proof
                .as_ref()
                .and_then(|p| text(p, "state"))
                .unwrap_or_else(|| text(row, "status").unwrap_or_else(|| "preparing".into()))
        };
        let mut receipt = row.get("receipt").cloned().filter(|v| !v.is_null());
        let mut basis = row.get("proof_basis").cloned().filter(|v| !v.is_null());
        if receipt.is_none() {
            receipt = proof
                .as_ref()
                .and_then(|p| p.get("proofs"))
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(|p| p.get("receipt"))
                .cloned();
        }
        if basis.is_none() {
            basis = proof
                .as_ref()
                .and_then(|p| p["proofs"].as_array())
                .and_then(|a| a.first())
                .and_then(|a| a.get("proof_basis"))
                .cloned();
        }
        let target_digest = row
            .get("target_digest")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                proof
                    .as_ref()
                    .and_then(|p| p.get("target_digest"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        let toolchain = row
            .get("toolchain")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                proof
                    .as_ref()
                    .and_then(|p| p.get("toolchain"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        nodes.push(Node {
            identity: if is_collected {
                unique_node_key(&work, version, &claim)
            } else {
                format!("record:{work}:{version}:{claim}")
            },
            lineage: if is_collected {
                format!("collected:{work}:{claim}")
            } else {
                format!("record:{work}:{claim}")
            },
            work: work.clone(),
            record,
            version,
            claim,
            label: text(row, "label").unwrap_or_default(),
            kind: text(row, "kind").unwrap_or_else(|| "paper".into()),
            title: text(row, "title").unwrap_or_else(|| text(&paper, "title").unwrap_or_default()),
            statement: text(row, "statement").unwrap_or_default(),
            macros: macros(row),
            status,
            target_digest,
            toolchain,
            receipt,
            proof_basis: basis,
            authors: if arr(row, "authors").is_empty() {
                arr(&paper, "authors")
            } else {
                arr(row, "authors")
            },
            contributor: row.get("contributor").cloned().filter(|v| !v.is_null()),
            attribution: row.get("attribution").cloned().filter(|v| !v.is_null()),
            lean_source: lean_source(&paper),
            proof_state: proof.as_ref().and_then(|p| text(p, "state")),
            next_step: proof
                .as_ref()
                .and_then(|p| text(p, "next_step"))
                .or_else(|| {
                    row.get("next_step")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
            scope,
        });
    }
    nodes.sort_by(|a, b| a.identity.cmp(&b.identity));
    Ok(nodes)
}
