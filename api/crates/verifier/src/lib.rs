//! Independent Lean verification. No database, identity, model, or credentials.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, process::Command};

pub const STANDARD_AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];
pub const OUTPUT_CAP: usize = 64 * 1024;

pub fn workspace_ready(workspace: &Path) -> Result<(), String> {
    let toolchain = std::fs::read_to_string(workspace.join("lean-toolchain"))
        .map_err(|_| "Lean toolchain pin is missing")?;
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(workspace.join("lake-manifest.json"))
            .map_err(|_| "dependency manifest is missing")?,
    )
    .map_err(|_| "dependency manifest is invalid")?;
    let mathlib = manifest["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|package| package["name"] == "mathlib"))
        .ok_or("Mathlib dependency is missing")?;
    if mathlib["rev"].as_str().is_none_or(|revision| {
        revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err("Mathlib must pin a full Git revision".into());
    }
    let home = std::env::var_os("HOME").ok_or("Lean installation is unavailable")?;
    let binary = PathBuf::from(home)
        .join(".elan/toolchains")
        .join(toolchain.trim().replace('/', "--").replace(':', "---"))
        .join("bin/lean");
    if !binary.is_file()
        || !workspace
            .join(".lake/packages/mathlib/.lake/build/lib/lean/Mathlib.olean")
            .is_file()
    {
        return Err("pinned Lean binary or built Mathlib artifacts are missing".into());
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, String>,
    pub target: String,
    pub target_digest: String,
    pub toolchain: String,
    pub solution: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Proved,
    Disproved,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub verdict: Verdict,
    pub reason: String,
    pub target_digest: String,
    pub solution_digest: String,
    pub toolchain: String,
    pub axioms: Vec<String>,
    pub checked_at: DateTime<Utc>,
    /// Milliseconds, including workspace preparation.
    pub duration: u64,
}
pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// Ignore inert comments only when locating hash commands in supporting modules.
/// Strings remain visible; nested Lean block comments and line comments are handled.
fn has_hash_command(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    let mut string = false;
    while i < bytes.len() {
        if depth > 0 {
            if bytes[i..].starts_with(b"/-") {
                depth += 1;
                i += 2;
            } else if bytes[i..].starts_with(b"-/") {
                depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if string {
            if bytes[i] == b'#' {
                return true;
            }
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                string = false;
            }
        } else if bytes[i..].starts_with(b"--") {
            i += bytes[i..]
                .iter()
                .position(|b| *b == b'\n')
                .unwrap_or(bytes.len() - i);
            continue;
        } else if bytes[i..].starts_with(b"/-") {
            depth = 1;
            i += 2;
            continue;
        } else if bytes[i] == b'"' {
            string = true;
        } else if bytes[i] == b'#' {
            return true;
        }
        i += 1;
    }
    false
}

/// Conservative root scan. Comments never stand in for target/solution declarations.
pub fn forbidden(text: &str) -> Option<String> {
    forbidden_in(text, false)
}
fn forbidden_in(text: &str, library: bool) -> Option<String> {
    for token in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if matches!(
            token,
            "axiom"
                | "constant"
                | "opaque"
                | "sorry"
                | "sorryAx"
                | "admit"
                | "implemented_by"
                | "extern"
                | "unsafe"
                | "macro"
                | "macro_rules"
                | "notation"
                | "syntax"
                | "elab"
                | "elab_rules"
                | "initialize"
                | "init"
                | "builtin_init"
                | "builtin_initialize"
                | "set_option"
                | "run_cmd"
                | "run_tac"
                | "run_elab"
                | "run_meta"
                | "attribute"
                | "declare_syntax_cat"
                | "export"
        ) || token.contains("skipKernelTC")
            || (!library && matches!(token, "namespace" | "section" | "end"))
        {
            return Some(format!("forbidden construct: {token}"));
        }
    }
    if (if library {
        has_hash_command(text)
    } else {
        text.contains('#')
    }) || text.contains('«')
        || text.contains('»')
        || text.contains('\u{202e}')
    {
        return Some("commands and quoted identifiers are forbidden".into());
    }
    None
}
fn allowed_import(name: &str, allowed: &[&str]) -> bool {
    allowed.contains(&name)
        || (name.starts_with("Mathlib.")
            && name.split('.').all(|p| {
                !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            }))
}
fn imports(text: &str, allowed: &[&str]) -> Result<(), String> {
    let words: Vec<_> = text.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if *word == "import" && words.get(i + 1).is_none_or(|m| !allowed_import(m, allowed)) {
            return Err("import must resolve to Mathlib, Target or a submitted module".into());
        }
    }
    // Imports must occupy their own line, one module per command. Reject
    // comment tricks and multi-module imports before invoking any compiler.
    for line in text.lines().filter(|l| l.contains("import")) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 || parts[0] != "import" || !allowed_import(parts[1], allowed) {
            return Err("imports must be a single allowed module per line".into());
        }
    }
    Ok(())
}
pub fn validate_target(text: &str) -> Result<(), String> {
    if text.len() > 800_000 {
        return Err("target is too large".into());
    }
    if let Some(reason) = forbidden(text) {
        return Err(reason);
    }
    imports(text, &["Mathlib"])?;
    if !text.starts_with("import Mathlib\n") {
        return Err("Target.lean must import Mathlib first".into());
    }
    let marker = "def wishpool_target_prop : Prop :=";
    if text.matches(marker).count() != 1 {
        return Err("declare exactly one def wishpool_target_prop : Prop".into());
    }
    let (_, statement) = text.split_once(marker).unwrap();
    if statement.trim().is_empty() {
        return Err("empty target proposition".into());
    }
    Ok(())
}
/// Convert the old final theorem mechanically; binder-bearing declarations need
/// regeneration because moving binders into a proposition requires interpretation.
pub fn convert_target(text: &str) -> Result<String, String> {
    if validate_target(text).is_ok() {
        return Ok(text.into());
    }
    let prefix = text
        .trim_end()
        .strip_suffix(":= by sorry")
        .ok_or("legacy target has no final proof hole")?;
    let (definitions, ty) = prefix
        .split_once("theorem wishpool_target :")
        .ok_or("legacy target needs regeneration")?;
    if forbidden(definitions).is_some() || forbidden(ty).is_some() {
        return Err("legacy target needs regeneration".into());
    }
    let imports = if definitions.starts_with("import Mathlib\n") {
        ""
    } else {
        "import Mathlib\n"
    };
    let converted = format!(
        "{imports}{definitions}def wishpool_target_prop : Prop := {}\n",
        ty.trim()
    );
    validate_target(&converted)?;
    Ok(converted)
}
pub fn validate_solution(text: &str) -> Result<Verdict, String> {
    validate_solution_imports(text, &["Mathlib", "Target"])
}
fn validate_solution_imports(text: &str, allowed: &[&str]) -> Result<Verdict, String> {
    if text.len() > 1_048_576 {
        return Err("solution exceeds 1 MB".into());
    }
    if let Some(reason) = forbidden(text) {
        return Err(reason);
    }
    imports(text, allowed)?;
    if !text.lines().any(|l| l.trim() == "import Target") {
        return Err("import Target is required".into());
    }
    let proof = text.matches("theorem wishpool_solution").count();
    let disproof = text.matches("theorem wishpool_disproof").count();
    match (proof, disproof) {
        (1, 0) => Ok(Verdict::Proved),
        (0, 1) => Ok(Verdict::Disproved),
        _ => Err("declare exactly one wishpool_solution or wishpool_disproof theorem".into()),
    }
}
pub fn axioms_of(text: &str, name: &str) -> Option<Vec<String>> {
    let (_, rest) = text.split_once(&format!("'{name}'"))?;
    let rest = rest.trim_start();
    if rest.starts_with("does not depend on any axioms") {
        return Some(vec![]);
    }
    let rest = rest
        .strip_prefix("depends on axioms:")?
        .trim_start()
        .strip_prefix('[')?;
    let (list, _) = rest.split_once(']')?;
    Some(
        list.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}
fn import_artifacts(olean: &Path) -> serde_json::Value {
    let mut parts = vec![olean.to_path_buf()];
    let server = olean.with_extension("olean.server");
    if server.is_file() {
        parts.push(server);
        let private = olean.with_extension("olean.private");
        if private.is_file() {
            parts.push(private);
        }
    }
    let mut ir = vec![];
    let sig = olean.with_extension("ir.sig");
    if sig.is_file() {
        ir.push(sig);
        let compiled = olean.with_extension("ir");
        if compiled.is_file() {
            ir.push(compiled);
        }
    }
    serde_json::json!([parts, ir])
}
fn ensure_module_parent(work: &Path, module: &str) -> Result<(), String> {
    let root = std::fs::symlink_metadata(work).map_err(|e| e.to_string())?;
    if !root.is_dir() || root.file_type().is_symlink() {
        return Err("verification workspace must be a real temporary directory".into());
    }
    let mut path = work.to_path_buf();
    let mut parts = module.split('.').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            break;
        }
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(format!(
                    "module {module}: output parent is a link or non-directory"
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&path).map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}
fn prepare_module_outputs(work: &Path, modules: &[&str]) -> Result<(), String> {
    // Validate every parent before touching artifacts. Prepared workspaces can
    // contain links; all untrusted writes must remain inside this fresh copy.
    for name in modules {
        ensure_module_parent(work, name)?;
    }
    let remove = |path: PathBuf| -> Result<(), String> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("cannot clear prepared output: {error}")),
        }
    };
    for name in modules {
        for suffix in [
            "olean",
            "olean.private",
            "olean.server",
            "ilean",
            "lean",
            "ir",
            "ir.sig",
        ] {
            remove(work.join(format!("{}.{suffix}", name.replace('.', "/"))))?;
        }
        // Setup JSON lives at the workspace root, named by the dotted module.
        remove(work.join(format!("{name}.setup.json")))?;
    }
    Ok(())
}
fn command(program: &Path, work: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", work)
        .current_dir(work)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    cmd
}
async fn capture(mut cmd: Command) -> Result<String, String> {
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    async fn bounded(reader: impl tokio::io::AsyncRead + Unpin) -> Result<Vec<u8>, String> {
        let mut bytes = vec![];
        reader
            .take((OUTPUT_CAP + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.len() > OUTPUT_CAP {
            return Err("compiler output cap exceeded".into());
        }
        Ok(bytes)
    }
    let (out, err) = tokio::try_join!(bounded(stdout), bounded(stderr))?;
    let status = child.wait().await.map_err(|e| e.to_string())?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );
    if !status.success() || text.contains("error:") {
        return Err(text.chars().take(4000).collect());
    }
    Ok(text)
}
#[derive(Clone)]
pub struct Checker {
    pub workspace: PathBuf,
    pub scratch: PathBuf,
    pub timeout: Duration,
}
impl Checker {
    pub async fn verify(&self, request: &Request) -> Receipt {
        let start = Instant::now();
        let mut receipt = Receipt {
            verdict: Verdict::Rejected,
            reason: String::new(),
            target_digest: digest(&request.target),
            solution_digest: solution_digest(&request.solution, &request.modules),
            toolchain: request.toolchain.clone(),
            axioms: vec![],
            checked_at: Utc::now(),
            duration: 0,
        };
        match tokio::time::timeout(
            self.timeout.min(Duration::from_secs(3600)),
            self.check(request),
        )
        .await
        {
            Ok(Ok((verdict, toolchain, axioms))) => {
                receipt.verdict = verdict;
                receipt.toolchain = toolchain;
                receipt.axioms = axioms;
                receipt.reason = "Lean checked the imported target and standard axioms".into();
            }
            Ok(Err(reason)) => receipt.reason = reason,
            Err(_) => receipt.reason = "verifier deadline exceeded".into(),
        }
        receipt.checked_at = Utc::now();
        receipt.duration = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        receipt
    }
    async fn check(&self, request: &Request) -> Result<(Verdict, String, Vec<String>), String> {
        self.check_using(request, None).await
    }
    async fn check_using(
        &self,
        request: &Request,
        binary: Option<&Path>,
    ) -> Result<(Verdict, String, Vec<String>), String> {
        if digest(&request.target) != request.target_digest {
            return Err("target digest mismatch".into());
        }
        validate_target(&request.target)?;
        let (verdict, modules) = validate_project(&request.solution, &request.modules)?;
        let read = |name: &str| {
            std::fs::read_to_string(self.workspace.join(name))
                .map_err(|e| format!("workspace: {e}"))
        };
        let lean_id = read("lean-toolchain")?.trim().to_string();
        let manifest: serde_json::Value =
            serde_json::from_str(&read("lake-manifest.json")?).map_err(|e| e.to_string())?;
        let rev = manifest["packages"]
            .as_array()
            .and_then(|p| p.iter().find(|p| p["name"] == "mathlib"))
            .and_then(|p| p["rev"].as_str())
            .ok_or("workspace must pin a Mathlib revision")?;
        let toolchain = format!("{lean_id}, Mathlib {rev}");
        if request.toolchain != toolchain && request.toolchain != lean_id {
            return Err("pinned toolchain/Mathlib mismatch; regenerate the target".into());
        }
        std::fs::create_dir_all(&self.scratch).map_err(|e| e.to_string())?;
        let temp = tempfile::Builder::new()
            .prefix("verifier-")
            .tempdir_in(&self.scratch)
            .map_err(|e| e.to_string())?;
        let work = temp.path().join("project");
        // Copy-on-write on macOS; ordinary fresh copy in the Linux deployment.
        let mut cp = command(Path::new("/bin/cp"), temp.path());
        #[cfg(target_os = "macos")]
        cp.arg("-c");
        cp.arg("-R").arg(&self.workspace).arg(&work);
        capture(cp).await?;
        let build_order: Vec<&str> = std::iter::once("Target")
            .chain(modules.iter().map(String::as_str))
            .chain(["Solution", "Check"])
            .collect();
        prepare_module_outputs(&work, &build_order)?;
        std::fs::write(work.join("Target.lean"), &request.target).map_err(|e| e.to_string())?;
        std::fs::write(work.join("Solution.lean"), &request.solution).map_err(|e| e.to_string())?;
        for (name, source) in &request.modules {
            let path = work.join(format!("{}.lean", name.replace('.', "/")));
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(path, source).map_err(|e| e.to_string())?;
        }
        let home = std::env::var_os("HOME").ok_or("operator HOME is unavailable")?;
        let lean = PathBuf::from(home)
            .join(".elan/toolchains")
            .join(lean_id.replace('/', "--").replace(':', "---"))
            .join("bin/lean");
        let lean = binary.map(Path::to_path_buf).unwrap_or(lean);
        if !lean.is_file() {
            return Err("pinned Lean binary is not installed".into());
        }
        // Lean's --setup resolves imports without passing LEAN_PATH or any
        // operator environment to untrusted elaboration.
        let mut artifacts = serde_json::Map::new();
        fn artifacts_in(
            dir: &Path,
            root: &Path,
            out: &mut serde_json::Map<String, serde_json::Value>,
        ) -> Result<(), String> {
            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.is_dir() {
                    artifacts_in(&path, root, out)?;
                } else if path.extension().is_some_and(|s| s == "olean") {
                    let name = path
                        .strip_prefix(root)
                        .unwrap()
                        .with_extension("")
                        .to_string_lossy()
                        .replace(['/', '\\'], ".");
                    out.insert(name, import_artifacts(&path));
                }
            }
            Ok(())
        }
        let packages = work.join(".lake/packages");
        for entry in std::fs::read_dir(packages).map_err(|e| e.to_string())? {
            let lib = entry
                .map_err(|e| e.to_string())?
                .path()
                .join(".lake/build/lib/lean");
            if lib.is_dir() {
                artifacts_in(&lib, &lib, &mut artifacts)?;
            }
        }
        if modules.iter().any(|name| {
            artifacts
                .keys()
                .any(|trusted| trusted.eq_ignore_ascii_case(name))
        }) {
            return Err("project modules cannot shadow trusted library modules".into());
        }
        for text in std::iter::once(&request.solution).chain(request.modules.values()) {
            for line in text
                .lines()
                .filter(|l| l.trim_start().starts_with("import "))
            {
                let imported = line.split_whitespace().nth(1).unwrap_or_default();
                if imported.starts_with("Mathlib.") && !artifacts.contains_key(imported) {
                    return Err(format!("missing prepared dependency: {imported}"));
                }
            }
        }
        for name in build_order {
            let setup = work.join(format!("{name}.setup.json"));
            std::fs::write(
                &setup,
                // Lean's derived FromJson for ModuleSetup requires every field.
                serde_json::json!({
                    "name": name,
                    "package?": null,
                    "isModule": false,
                    "imports?": null,
                    "importArts": artifacts,
                    "dynlibs": [],
                    "plugins": [],
                    "options": {}
                })
                .to_string(),
            )
            .map_err(|e| e.to_string())?;
            if name == "Check" {
                let (constant, ty) = if verdict == Verdict::Proved {
                    ("wishpool_solution", "_root_.wishpool_target_prop")
                } else {
                    ("wishpool_disproof", "¬ _root_.wishpool_target_prop")
                };
                std::fs::write(work.join("Check.lean"), format!("import Solution\n#check (_root_.{constant} : {ty})\n#print axioms {constant}\n")).map_err(|e| e.to_string())?;
            }
            let mut cmd = command(&lean, &work);
            cmd.args(["-j", "2", "--setup"])
                .arg(&setup)
                .arg("-o")
                .arg(format!("{}.olean", name.replace('.', "/")))
                .arg(format!("{}.lean", name.replace('.', "/")));
            let text = capture(cmd).await?;
            if name == "Check" {
                let constant = if verdict == Verdict::Proved {
                    "wishpool_solution"
                } else {
                    "wishpool_disproof"
                };
                let axioms =
                    axioms_of(&text, constant).ok_or("missing independent axiom receipt")?;
                if axioms
                    .iter()
                    .any(|a| !STANDARD_AXIOMS.contains(&a.as_str()))
                {
                    return Err("solution depends on non-standard axioms".into());
                }
                return Ok((verdict, toolchain, axioms));
            }
            artifacts.insert(
                name.into(),
                import_artifacts(&work.join(format!("{}.olean", name.replace('.', "/")))),
            );
        }
        unreachable!()
    }
}
#[cfg(test)]
mod tests;

/// Hash the entire project while preserving historical single-file receipts.
pub fn solution_digest(solution: &str, modules: &BTreeMap<String, String>) -> String {
    if modules.is_empty() {
        return digest(solution);
    }
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&("mathesis-project-v1", solution, modules))
                .expect("source serialization")
        )
    )
}
/// Independently validate all modules, including unused ones, before I/O.
fn validate_project(
    solution: &str,
    modules: &BTreeMap<String, String>,
) -> Result<(Verdict, Vec<String>), String> {
    if modules.len() > 512
        || solution.len() + modules.values().map(String::len).sum::<usize>() > 8 * 1_048_576
    {
        return Err("project exceeds 512 supporting modules or 8 MB total source".into());
    }
    let mut seen = BTreeSet::new();
    for (name, text) in modules {
        if name.len() > 240
            || !name.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 64
                    && part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            || [
                "target", "solution", "check", "mathlib", "init", "lean", "std", "lake",
            ]
            .contains(
                &name
                    .split('.')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
            )
            || !seen.insert(name.to_ascii_lowercase())
            || text.trim().is_empty()
        {
            return Err("invalid or reserved project module name".into());
        }
        if let Some(reason) = forbidden_in(text, true) {
            return Err(format!("{name}: {reason}"));
        }
        if [
            "wishpool_solution",
            "wishpool_disproof",
            "wishpool_target_prop",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            return Err("supporting modules cannot declare the target or solution".into());
        }
    }
    let allowed: Vec<&str> = ["Mathlib", "Target"]
        .into_iter()
        .chain(modules.keys().map(String::as_str))
        .collect();
    let verdict = validate_solution_imports(solution, &allowed)?;
    let mut dependencies = BTreeMap::new();
    for (name, text) in modules {
        imports(text, &allowed)?;
        let deps: BTreeSet<String> = text
            .lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                (words.next() == Some("import"))
                    .then(|| words.next())
                    .flatten()
            })
            .filter(|module| modules.contains_key(*module))
            .map(str::to_owned)
            .collect();
        dependencies.insert(name.clone(), deps);
    }
    let mut order = Vec::new();
    while !dependencies.is_empty() {
        let next = dependencies
            .iter()
            .find(|(_, deps)| deps.is_empty())
            .map(|(name, _)| name.clone())
            .ok_or("cyclic project imports")?;
        dependencies.remove(&next);
        for deps in dependencies.values_mut() {
            deps.remove(&next);
        }
        order.push(next);
    }
    Ok((verdict, order))
}
