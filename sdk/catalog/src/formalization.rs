//! Public exact sources and receipts; this projection cannot create verification or credit.
use super::*;

const PINS: &str = include_str!("../verifier-pins.json");
const INPUTS: &[(&str, &str)] = &[
    (
        "lean-toolchain",
        include_str!("../../../infra/verifier/lean-toolchain"),
    ),
    (
        "lakefile.toml",
        include_str!("../../../infra/verifier/lakefile.toml"),
    ),
    (
        "mathesis-libraries.json",
        include_str!("../../../infra/verifier/mathesis-libraries.json"),
    ),
    (
        "library-modules.txt",
        include_str!("../../../infra/verifier/library-modules.txt"),
    ),
    (
        "Lean.Dockerfile",
        include_str!("../../../infra/verifier/Lean.Dockerfile"),
    ),
];
pub(crate) const QUEUE: &str = "https://github.com/the-omega-institute/trureturing-requests";

pub(crate) fn pins() -> Result<Value> {
    let value: Value = serde_json::from_str(PINS)?;
    for (name, source) in INPUTS {
        if value["inputs"][name] != *source
            || value["input_sha256"][name] != sha256(source.as_bytes())
        {
            return Err(msg(
                "verifier pins changed; run python3 sdk/catalog/scripts/refresh-verifier-pins.py",
            ));
        }
    }
    Ok(value)
}

fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| msg(format!("missing string {key}")))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| msg(format!("missing array {key}")))
}
fn target_bytes(v: &Value, field: &str, digest: &str) -> Result<()> {
    if sha256(required(v, field)?.as_bytes()) != required(v, digest)? {
        return Err(msg("public target SHA-256 mismatch"));
    }
    Ok(())
}
fn statement_path(record: &str, claim: &str) -> String {
    format!(
        "/papers/{}/statements/{}",
        urlencoding(record),
        urlencoding(claim)
    )
}

/// Runs inside the existing atomic snapshot transaction. Any missing source or
/// mismatched receipt aborts, rather than silently omitting a verified result.
pub(crate) fn collect(
    client: &Client,
    base: &str,
    temp: &Path,
    rows: &[Value],
    listings: &mut BTreeMap<String, ListingCount>,
) -> Result<()> {
    let mut targets = Vec::new();
    let mut bundles = Vec::new();
    for row in rows.iter().filter(|r| !r["target_digest"].is_null()) {
        let record = required(row, "record")?;
        let claim = required(row, "claim")?;
        let path = statement_path(record, claim);
        let detail = get_json(client, &format!("{base}{path}"))?;
        let target = &detail["target"];
        target_bytes(target, "lean", "digest")?;
        if target["digest"] != row["target_digest"] || target["toolchain"] != row["toolchain"] {
            return Err(msg("confirmed target changed during snapshot"));
        }
        let raw = request_bytes(client, &format!("{base}{path}/target"))?;
        if raw != required(target, "lean")?.as_bytes() {
            return Err(msg("target download differs from statement detail"));
        }
        targets.push(
            json!({"record":record,"claim":claim,"version":row["version"],"text":target["lean"],
            "digest":target["digest"],"toolchain":target["toolchain"]}),
        );
        let attempts = get_json(client, &format!("{base}{path}/attempts"))?;
        let attempts = attempts
            .as_array()
            .ok_or_else(|| msg("verified attempts are not an array"))?;
        if row["verified_count"]
            .as_u64()
            .is_some_and(|n| n != attempts.len() as u64)
        {
            return Err(msg("public attempt count changed during snapshot"));
        }
        for attempt in attempts {
            let id = required(attempt, "id")?;
            let bundle = get_json(
                client,
                &format!("{base}{path}/attempts/{}/bundle", urlencoding(id)),
            )?;
            validate_bundle(&bundle)?;
            if bundle["record"] != record
                || bundle["claim"] != claim
                || bundle["version"] != row["version"]
                || bundle["attempt"]["id"] != id
                || bundle["verification_request"]["target"] != target["lean"]
                || bundle["attempt"]["receipt"] != attempt["receipt"]
            {
                return Err(msg("verification bundle identity changed during snapshot"));
            }
            bundles.push(bundle);
        }
    }
    let (problems, count) = collect_pages(
        client,
        base,
        "/research-problems",
        &temp.join("raw/research-problems"),
    )?;
    listings.insert("research-problems".into(), count);
    let mut curated = Vec::new();
    for problem in problems {
        let id = required(&problem, "id")?;
        let detail = get_json(
            client,
            &format!("{base}/research-problems/{}", urlencoding(id)),
        )?;
        if detail["summary"]["id"] != id {
            return Err(msg("research problem detail identity mismatch"));
        }
        if !detail["curated"].is_null() {
            if detail["curated"]["lean_target"]["state"] == "elaborated" {
                target_bytes(&detail["curated"]["lean_target"], "text", "digest")?;
            }
            // Only the curator's public record is needed; note pagination does not
            // affect this versioned object and conveys no additional target.
            curated.push(json!({"id":id,"summary":detail["summary"],"curated":detail["curated"]}));
        }
    }
    curated.sort_by_key(|v| text(v, "id"));
    bundles.sort_by_key(|v| {
        (
            text(v, "record"),
            text(v, "claim"),
            text(&v["attempt"], "id"),
        )
    });
    let mut leaderboards = Map::new();
    for (name, path) in [
        ("conjectures", "/leaderboard"),
        ("formalizations", "/leaderboard/formalizations"),
    ] {
        let rows = get_json(client, &format!("{base}{path}?period=all&entrants=all"))?;
        let count = rows
            .as_array()
            .ok_or_else(|| msg("leaderboard is not an array"))?
            .len();
        listings.insert(
            format!("leaderboard/{name}"),
            ListingCount {
                pages: 1,
                rows: count,
            },
        );
        leaderboards.insert(name.into(), rows);
    }
    write_json(
        &temp.join("raw/formalization.json"),
        &json!({"schema":"mathesis.formalization-snapshot.v1",
        "pins":pins()?,"targets":targets,"bundles":bundles,"curated":curated,"leaderboards":leaderboards}),
    )
}

pub(crate) fn validate_bundle(bundle: &Value) -> Result<()> {
    if ![Some(1), Some(2)].contains(&bundle["schema_version"].as_u64()) {
        return Err(msg("unknown verification bundle schema"));
    }
    let request = &bundle["verification_request"];
    let attempt = &bundle["attempt"];
    let receipt = &attempt["receipt"];
    target_bytes(request, "target", "target_digest")?;
    if !matches!(receipt["verdict"].as_str(), Some("proved" | "disproved"))
        || receipt["target_digest"] != request["target_digest"]
        || receipt["toolchain"] != request["toolchain"]
        || attempt["solution"] != request["solution"]
    {
        return Err(msg(
            "bundle does not bind a public checked solution to its target",
        ));
    }
    let modules: BTreeMap<String, String> =
        serde_json::from_value(request.get("modules").cloned().unwrap_or_else(|| json!({})))?;
    let attempt_modules = attempt.get("modules").cloned().unwrap_or_else(|| json!({}));
    if serde_json::to_value(&modules)? != attempt_modules {
        return Err(msg("bundle supporting modules differ"));
    }
    let solution = required(request, "solution")?;
    let bytes = if modules.is_empty() {
        solution.as_bytes().to_vec()
    } else {
        serde_json::to_vec(&("mathesis-project-v1", solution, &modules))?
    };
    if receipt["solution_digest"] != sha256(&bytes) {
        return Err(msg("bundle solution SHA-256 mismatch"));
    }
    let mut paths = BTreeSet::new();
    for name in modules.keys() {
        let root = name
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.len() > 240
            || [
                "target", "solution", "check", "mathlib", "init", "lean", "std", "lake",
            ]
            .contains(&root.as_str())
            || !name.split('.').all(|s| {
                !s.is_empty()
                    && s.len() <= 64
                    && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            || !paths.insert(name.to_ascii_lowercase())
        {
            return Err(msg("unsafe or colliding supporting module name"));
        }
    }
    Ok(())
}

fn component(s: &str) -> String {
    // An injective single component even for arbitrary public identities. Ordinary
    // work/claim names remain readable; '%' is always escaped.
    urlencoding(s).replace('.', "%2E")
}
fn node_dir(n: &Node) -> String {
    format!(
        "{}/v{}/{}",
        component(&n.work),
        n.version,
        component(&n.claim)
    )
}
fn write_source(root: &Path, path: &str, source: &str) -> Result<()> {
    if let Some(parent) = root.join(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(root.join(path), source)?;
    Ok(())
}
fn target_for<'a>(data: &'a Value, n: &Node) -> Option<&'a Value> {
    data["targets"]
        .as_array()?
        .iter()
        .find(|v| v["record"] == n.record && v["claim"] == n.claim && v["version"] == n.version)
}
fn bundles_for<'a>(data: &'a Value, n: &Node) -> Vec<&'a Value> {
    data["bundles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| v["record"] == n.record && v["claim"] == n.claim && v["version"] == n.version)
        .collect()
}
fn classification(n: &Node) -> Result<&'static str> {
    if is_verified(n) {
        match n.proof_basis.as_ref().and_then(|v| v["kind"].as_str()) {
            Some("library_declarations") => Ok("covered_by_existing_base"),
            Some("new_lean") => Ok("newly_formalized"),
            _ => Err(msg("verified node has no public proof basis")),
        }
    } else if n.status == "disproved" {
        Ok("disproved")
    } else if n.target_digest.is_some() {
        Ok("open_target")
    } else {
        Ok("no_target_yet")
    }
}

pub(crate) fn render(
    snapshot: &Snapshot,
    root: &Path,
    works: &[Work],
    nodes: &[Node],
    site: &str,
) -> Result<()> {
    let data = read_json(&snapshot.root.join("raw/formalization.json"))?;
    if data["schema"] != "mathesis.formalization-snapshot.v1" || data["pins"] != pins()? {
        return Err(msg(
            "formalization snapshot needs this SDK's verifier pins; take a fresh snapshot",
        ));
    }
    let pins = &data["pins"];
    for (path, source) in [
        (
            "lean-toolchain",
            required(&pins["inputs"], "lean-toolchain")?,
        ),
        ("lakefile.toml", required(&pins["inputs"], "lakefile.toml")?),
        (
            "ci/mathlib/lean-toolchain",
            required(&pins["inputs"], "lean-toolchain")?,
        ),
        (
            "ci/mathlib/lakefile.toml",
            required(pins, "mathlib_lakefile")?,
        ),
    ] {
        write_source(root, path, source)?;
    }
    write_json(&root.join("lake-manifest.json"), &pins["lake_manifest"])?;
    write_json(
        &root.join("ci/mathlib/lake-manifest.json"),
        &pins["mathlib_manifest"],
    )?;
    write_json(&root.join("environment/verifier-pins.json"), pins)?;
    for (name, source) in INPUTS {
        write_source(root, &format!("environment/verifier/{name}"), source)?;
    }
    write_json(
        &root.join("mathesis-libraries.json"),
        &serde_json::from_str::<Value>(required(&pins["inputs"], "mathesis-libraries.json")?)?,
    )?;
    for (area, explanation) in [
        (
            "targets",
            "Confirmed exact targets, grouped by work, version and claim. Each Target.lean compiles separately. No confirmed paper targets in this snapshot unless listed in index/modules.json.",
        ),
        (
            "proofs",
            "Exact public verified proof bundles. Each attempt has its own module root and original receipt. No proof bundles in this snapshot unless listed in index/modules.json.",
        ),
        (
            "disproofs",
            "Exact public verified disproof bundles, displayed separately from proofs. No disproof bundles in this snapshot unless listed in index/modules.json.",
        ),
        (
            "curated",
            "The curator's elaborated formalizations. These are not confirmed by the original posers and are not open for submission.",
        ),
    ] {
        write_source(
            root,
            &format!("{area}/README.md"),
            &format!("# {area}\n\n{explanation}\n"),
        )?;
    }
    let mut index = Vec::new();
    let mut modules = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    for key in [
        "covered_by_existing_base",
        "newly_formalized",
        "open_target",
        "no_target_yet",
        "disproved",
        "curated_targets",
        "proof_bundles",
        "disproof_bundles",
    ] {
        counts.insert(key.into(), 0);
    }
    for n in nodes.iter().filter(|n| n.scope == "in_scope") {
        let class = classification(n)?;
        *counts.entry(class.into()).or_default() += 1;
        let mut entry = serde_json::to_value(n)?;
        entry["classification"] = json!(class);
        entry["reason"] = if class == "no_target_yet" {
            json!(n.next_step)
        } else {
            Value::Null
        };
        entry["request_queue_url"] = json!(QUEUE);
        entry["trureturing_write_back"] = json!({"request_url":null,"state":null,"revision":null});
        entry["target_path"] = Value::Null;
        entry["results"] = json!([]);
        if let Some(target) = target_for(&data, n) {
            target_bytes(target, "text", "digest")?;
            if Some(required(target, "digest")?.to_owned()) != n.target_digest
                || Some(required(target, "toolchain")?.to_owned()) != n.toolchain
            {
                return Err(msg("node and exact target disagree"));
            }
            let path = format!("targets/{}/Target.lean", node_dir(n));
            write_source(root, &path, required(target, "text")?)?;
            write_json(
                &root.join(path.replace("Target.lean", "target.json")),
                target,
            )?;
            entry["target_path"] = json!(path);
            modules.push(json!({"path":path,"kind":"open_or_confirmed_target","toolchain":target["toolchain"],"compatible":target["toolchain"]==pins["toolchain"],"digest":target["digest"]}));
        } else if n.target_digest.is_some() {
            return Err(msg("confirmed node has no snapshotted Lean text"));
        }
        let node_bundles = bundles_for(&data, n);
        if let Some(receipt) = &n.receipt
            && !node_bundles.iter().any(|b| {
                b["attempt"]["receipt"] == *receipt
                    && Some(b["attempt"]["proof_basis"].clone()) == n.proof_basis
            })
        {
            return Err(msg(
                "directory winning receipt/basis has no matching public bundle",
            ));
        }
        for bundle in node_bundles {
            validate_bundle(bundle)?;
            let attempt = &bundle["attempt"];
            if Some(required(&attempt["receipt"], "target_digest")?.to_owned()) != n.target_digest {
                return Err(msg("bundle target differs from node"));
            }
            let area = if attempt["receipt"]["verdict"] == "proved" {
                "proofs"
            } else {
                "disproofs"
            };
            let dir = format!(
                "{area}/{}/{}",
                node_dir(n),
                component(required(attempt, "id")?)
            );
            let request = &bundle["verification_request"];
            for (name, key) in [("Target.lean", "target"), ("Solution.lean", "solution")] {
                write_source(root, &format!("{dir}/{name}"), required(request, key)?)?;
            }
            if let Some(support) = request["modules"].as_object() {
                for (name, source) in support {
                    write_source(
                        root,
                        &format!("{dir}/{}.lean", name.replace('.', "/")),
                        source.as_str().ok_or_else(|| msg("module is not text"))?,
                    )?;
                }
            }
            write_json(
                &root.join(format!("{dir}/receipt.json")),
                &attempt["receipt"],
            )?;
            write_json(&root.join(format!("{dir}/bundle.json")), bundle)?;
            entry["results"].as_array_mut().unwrap().push(json!({"attempt":attempt["id"],"path":dir,"verdict":attempt["receipt"]["verdict"],"also_verified":attempt["also_verified"],"proof_basis":attempt["proof_basis"]}));
            modules.push(json!({"path":dir,"kind":"bundle","toolchain":request["toolchain"],"compatible":request["toolchain"]==pins["toolchain"],"libraries":attempt["receipt"]["library_use"]["libraries"]}));
            *counts
                .entry(if area == "proofs" {
                    "proof_bundles".into()
                } else {
                    "disproof_bundles".into()
                })
                .or_default() += 1;
        }
        if matches!(
            class,
            "covered_by_existing_base" | "newly_formalized" | "disproved"
        ) && entry["results"].as_array().unwrap().is_empty()
        {
            return Err(msg("verified node has no public bundle"));
        }
        index.push(entry);
    }
    let mut curated_index = Vec::new();
    for item in array(&data, "curated")? {
        let target = &item["curated"]["lean_target"];
        if target["state"] != "elaborated" {
            continue;
        }
        target_bytes(target, "text", "digest")?;
        let id = required(item, "id")?;
        let version = item["curated"]["version"]
            .as_u64()
            .ok_or_else(|| msg("curated version missing"))?;
        let dir = format!("curated/{}/v{version}", component(id));
        let path = format!("{dir}/Target.lean");
        write_source(root, &path, required(target, "text")?)?;
        write_json(&root.join(format!("{dir}/problem.json")), item)?;
        write_source(
            root,
            &format!("{dir}/README.md"),
            "This is the curator's formalization, not confirmed by the original poser. Elaboration establishes a well-formed proposition, not faithfulness or a proof. It is not open for proof submission and earns no credit.\n",
        )?;
        curated_index.push(json!({"id":id,"version":version,"path":path,"digest":target["digest"],"toolchain":target["toolchain"],"state":"elaborated","open_for_submission":false}));
        modules.push(json!({"path":path,"kind":"curated_target","toolchain":target["toolchain"],"compatible":target["toolchain"]==pins["toolchain"],"digest":target["digest"]}));
        *counts.entry("curated_targets".into()).or_default() += 1;
    }
    modules.sort_by_key(|v| text(v, "path"));
    write_json(
        &root.join("index/modules.json"),
        &json!({"modules":modules}),
    )?;
    write_json(
        &root.join("index/curated.json"),
        &json!({"targets":curated_index}),
    )?;
    write_json(
        &root.join("index/nodes.json"),
        &json!({"schema":"mathesis.formalization-nodes.v1","nodes":index}),
    )?;
    for work in works.iter().filter(|w| w.publication == "public") {
        let wn: Vec<_> = index
            .iter()
            .filter(|n| n["work"] == work.work)
            .cloned()
            .collect();
        let mut split = Map::new();
        for key in [
            "covered_by_existing_base",
            "newly_formalized",
            "open_target",
            "no_target_yet",
            "disproved",
        ] {
            split.insert(
                key.into(),
                json!(
                    wn.iter()
                        .filter(|n| n["classification"] == key)
                        .collect::<Vec<_>>()
                ),
            );
        }
        write_json(
            &root.join(format!("index/papers/{}.json", component(&work.work))),
            &json!({"schema":"mathesis.formalization-paper.v1","work":work.work,"version":work.current_version,"title":work.title,"authors":work.authors,"lean_source":work.lean_source,"split":split,"request_queue_url":QUEUE,"trureturing_write_back":{"request_url":null,"state":null,"revision":null}}),
        )?;
    }
    let leaderboards = &data["leaderboards"];
    for name in ["conjectures", "formalizations"] {
        array(leaderboards, name)?;
    }
    write_json(
        &root.join("leaderboard.json"),
        &json!({"schema":"mathesis.formalization-leaderboards.v1","snapshot_time":snapshot.meta.captured_at,"period":"all","entrants":"all","routes":{"conjectures":"/api/v1/leaderboard","formalizations":"/api/v1/leaderboard/formalizations"},"boards":leaderboards}),
    )?;
    let mut board = String::from(
        "# Mathesis leaderboards\n\nPublic API order and credit are preserved. Period: all time; entrants: all people and agents. Repository pull requests and CI confer no verification or credit.\n",
    );
    for (name, label) in [
        ("formalizations", "Paper formalizations"),
        ("conjectures", "Conjecture solves and disproofs"),
    ] {
        writeln!(
            board,
            "\n## {label}\n\n| Rank | Entrant | Others | Total | Proved | Disproved | Last solve |\n|---:|---|---:|---:|---:|---:|---|"
        )?;
        for row in array(leaderboards, name)? {
            writeln!(
                board,
                "| {} | {} | {} | {} | {} | {} | {} |",
                row["rank"],
                required(&row["entrant"], "name")?.replace('|', "\\|"),
                row["others"],
                row["total"],
                row["solved"],
                row["disproved"],
                required(row, "last_solve")?
            )?;
        }
        if array(leaderboards, name)?.is_empty() {
            board.push_str("\nNo rows in this public snapshot.\n");
        }
    }
    writeln!(
        board,
        "\nSnapshot: `{}`. See [machine-readable data](leaderboard.json).",
        snapshot.meta.captured_at
    )?;
    write_source(root, "LEADERBOARD.md", &board)?;
    write_json(
        &root.join("snapshot.json"),
        &json!({"schema":"mathesis.formalization-repository.v1","captured_at":snapshot.meta.captured_at,"api_base":snapshot.meta.api_base,"snapshot_sha256":sha256_file(&snapshot.root.join("snapshot.json"))?,"counts":counts,"papers":works.iter().filter(|w|w.publication=="public").count(),"nodes":index.len(),"leaderboard_rows":{"formalizations":array(leaderboards,"formalizations")?.len(),"conjectures":array(leaderboards,"conjectures")?.len()}}),
    )?;
    let readme = format!(
        "# Mathesis formalization\n\nThis repository is a projection of public Mathesis routes. Mathesis, its MongoDB records and independent verifier receipts remain the authority. A clone can read and compile the projected sources independently of trureturing publication. Repository pull requests and CI confer no verification or credit.\n\nSnapshot: `{}`. API: {}.\n\n| Category | Count | Scope |\n|---|---:|---|\n| Papers | {} | Current public CC-BY-4.0 collected works |\n| Statements | {} | Current in-scope versioned nodes |\n| Covered by existing base | {} | Matching proved receipt with library_declarations; declarations and library pins retained |\n| Newly formalized | {} | Matching proved receipt with new_lean; this describes sources, not novelty |\n| Open targets | {} | Confirmed exact targets without a verified result |\n| No target yet | {} | Public statements; reason copied from the API |\n| Disproved statements | {} | Kept separate from proved results |\n| Curated targets | {} | Curator's elaborated propositions; not open for submission |\n| Proof bundles | {} | All public verified proof attempts, including also verified |\n| Disproof bundles | {} | All public verified disproof attempts |\n\nA paper-level Lean source link alone establishes no statement-level coverage. Covered targets follow from the recorded declarations together with the submitted Lean; this is not a semantic-equivalence or coverage certificate. Curated targets are the curator's formalizations, not confirmed by the original posers; elaboration shows well-formedness, not faithfulness or a proof.\n\n## Build exact modules\n\nAll targets declare the same root name. There is no aggregate import. `index/modules.json` enumerates every build unit, its toolchain and compatibility with this projection's pins. `lean-toolchain`, Lake files and manifests are generated from the same Mathesis source tree's verifier inputs; `environment/verifier-pins.json` retains those inputs and the pinned upstream locks.\n\nWith the pinned toolchain installed, run `cd ci/mathlib && lake exe cache get` once to prepare Mathlib, then return to the repository root. `python3 scripts/build.py targets` compiles every compatible `targets/**/Target.lean` and `curated/**/Target.lean` separately against that Mathlib-only workspace. `python3 scripts/build.py <exact-path-from-index>` compiles one target. No target proves its proposition.\n\nFor a proof or disproof bundle, first prepare the root Lake workspace (`lake exe cache get`) and compile the required prepared-library imports from source at the recorded revision; the reference verifier's module list is `environment/verifier/library-modules.txt`. This can take about 87 minutes. Then run `python3 scripts/build.py <bundle-directory-from-index> --workspace .`. It compiles supporting modules in dependency order, Target and Solution in a temporary isolated module root, preserving exact bytes. It does not run the independent Mathesis verifier's restricted-source/axiom/constant checks. For historical pins use `--workspace /path/to/matching/prepared/workspace`; mismatches fail explicitly. See each unchanged `bundle.json` for the real verifier replay instructions.\n\n`python3 scripts/check.py` checks generated hashes, target and solution digests, bundle identity and source layout, and index counts without Lean. CI runs this and compiles compatible targets with cached Mathlib. CI does not pull the GHCR base image, build trureturing, compile proof/disproof bundles, guarantee target faithfulness, or award credit. Anonymous pull access to the reference base image was unavailable in the 2026-11-13 probe; see the Mathesis design document for the recorded HTTP evidence. Incompatible historical targets are listed and reported as skipped by the target build command.\n\n## Take part\n\nUse [Mathesis Contribute]({}/contribute) and [sdk/contribute](https://github.com/the-omega-institute/mathesis/tree/develop/sdk/contribute). For a confirmed target, the attempt path is `POST /api/v1/papers/{{record}}/statements/{{claim}}/attempts` (conjectures also have their documented route). Sign in through Mathesis, submit an attempt and wait for the independent verifier receipt; the public leaderboards determine credit. Curated targets do not accept attempts.\n\n[index/nodes.json](index/nodes.json) and `index/papers/<work>.json` keep the base/new/open/no-target split and exact API reasons. [LEADERBOARD.md](LEADERBOARD.md) and [leaderboard.json](leaderboard.json) preserve both public boards, with separate scopes. Finished work is displayed here and may be written back through the [trureturing request queue]({QUEUE}); request URL, state and revision remain null until publicly known. This generator never submits a request or invents write-back state.\n",
        snapshot.meta.captured_at,
        snapshot.meta.api_base,
        works.iter().filter(|w| w.publication == "public").count(),
        index.len(),
        counts["covered_by_existing_base"],
        counts["newly_formalized"],
        counts["open_target"],
        counts["no_target_yet"],
        counts["disproved"],
        counts["curated_targets"],
        counts["proof_bundles"],
        counts["disproof_bundles"],
        normalize_base(site)
    );
    write_source(root, "README.md", &readme)?;
    for (path, source) in [
        (
            "scripts/check.py",
            include_str!("../template/formalization/scripts/check.py"),
        ),
        (
            "scripts/build.py",
            include_str!("../template/formalization/scripts/build.py"),
        ),
        (
            ".github/workflows/check.yml",
            include_str!("../template/formalization/.github/workflows/check.yml"),
        ),
        (".gitignore", ".lake/\n__pycache__/\n*.olean\n*.ilean\n"),
    ] {
        write_source(root, path, source)?;
    }
    let mut hashes = BTreeMap::new();
    super::render::collect_files(root, root, &mut hashes, false)?;
    write_json(
        &root.join(".mathesis/generated.json"),
        &json!({"schema":"mathesis.formalization-generated.v1","sha256":hashes}),
    )?;
    Ok(())
}

/// The global manifest covers every file. Independently reconstruct this tree's
/// counts and source bindings as well, so a consistently rehashed bad index fails.
pub(crate) fn check_output(output: &Path) -> Result<()> {
    let root = output.join("formalization");
    let meta = read_json(&root.join("snapshot.json"))?;
    let nodes = read_json(&root.join("index/nodes.json"))?;
    let units = read_json(&root.join("index/modules.json"))?;
    let curated = read_json(&root.join("index/curated.json"))?;
    let boards = read_json(&root.join("leaderboard.json"))?;
    let env = read_json(&root.join("environment/verifier-pins.json"))?;
    if env != pins()? {
        return Err(msg("rendered verifier pins differ"));
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for key in [
        "covered_by_existing_base",
        "newly_formalized",
        "open_target",
        "no_target_yet",
        "disproved",
        "curated_targets",
        "proof_bundles",
        "disproof_bundles",
    ] {
        counts.insert(key.into(), 0);
    }
    for node in array(&nodes, "nodes")? {
        let n: Node = serde_json::from_value(node.clone())?;
        let class = classification(&n)?;
        if node["classification"] != class
            || n.scope != "in_scope"
            || (class == "no_target_yet" && node["reason"] != json!(n.next_step))
        {
            return Err(msg("formalization node classification/reason differs"));
        }
        *counts.entry(class.into()).or_default() += 1;
    }
    let mut seen = BTreeSet::new();
    for unit in array(&units, "modules")? {
        let path = required(unit, "path")?;
        if !Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
            || !seen.insert(path.to_ascii_lowercase())
        {
            return Err(msg("unsafe or duplicate build unit"));
        }
        if unit["compatible"] != json!(unit["toolchain"] == env["toolchain"]) {
            return Err(msg("build compatibility differs"));
        }
        if unit["kind"] == "bundle" {
            let bundle = read_json(&root.join(path).join("bundle.json"))?;
            validate_bundle(&bundle)?;
            let request = &bundle["verification_request"];
            for (file, field) in [("Target.lean", "target"), ("Solution.lean", "solution")] {
                if fs::read(root.join(path).join(file))? != required(request, field)?.as_bytes() {
                    return Err(msg("rendered bundle source differs"));
                }
            }
            if let Some(support) = request["modules"].as_object() {
                for (name, source) in support {
                    if fs::read(
                        root.join(path)
                            .join(format!("{}.lean", name.replace('.', "/"))),
                    )? != source
                        .as_str()
                        .ok_or_else(|| msg("supporting module not text"))?
                        .as_bytes()
                    {
                        return Err(msg("rendered supporting source differs"));
                    }
                }
            }
            if read_json(&root.join(path).join("receipt.json"))? != bundle["attempt"]["receipt"] {
                return Err(msg("rendered receipt differs"));
            }
            let key = if bundle["attempt"]["receipt"]["verdict"] == "proved" {
                "proof_bundles"
            } else {
                "disproof_bundles"
            };
            *counts.entry(key.into()).or_default() += 1;
        } else if sha256_file(&root.join(path))? != required(unit, "digest")? {
            return Err(msg("rendered target SHA-256 differs"));
        }
    }
    counts.insert("curated_targets".into(), array(&curated, "targets")?.len());
    if meta["counts"] != json!(counts) || meta["nodes"] != json!(array(&nodes, "nodes")?.len()) {
        return Err(msg("formalization counts differ"));
    }
    for name in ["formalizations", "conjectures"] {
        if meta["leaderboard_rows"][name] != json!(array(&boards["boards"], name)?.len()) {
            return Err(msg("formalization leaderboard count differs"));
        }
    }
    Ok(())
}
