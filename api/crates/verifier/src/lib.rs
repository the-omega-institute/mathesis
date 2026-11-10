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
/// Workspace file declaring the prepared libraries, other than Mathlib, that a
/// solution may import. Only the operator who prepares the workspace writes it.
pub const LIBRARIES_FILE: &str = "mathesis-libraries.json";
/// Library declarations listed in one receipt; the total is reported separately.
pub const LIBRARY_DECLARATION_CAP: usize = 64;

/// A prepared library whose compiled modules solutions and supporting modules
/// may import. Targets never import it: a statement stays readable over Mathlib.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    /// Lake package name in the prepared workspace.
    pub name: String,
    /// Full Git revision resolved in the workspace manifest.
    pub revision: String,
    /// Top-level module names owned by this library.
    pub roots: Vec<String>,
    /// HTTPS source repository recorded in the workspace manifest, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryDeclarations {
    libraries: Vec<DeclaredLibrary>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredLibrary {
    package: String,
    roots: Vec<String>,
}
fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn full_revision(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
/// Read the operator's library declaration and bind each library to the exact
/// revision in the workspace manifest. A workspace without the file has none.
pub fn libraries(workspace: &Path) -> Result<Vec<Library>, String> {
    let bytes = match std::fs::read(workspace.join(LIBRARIES_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(_) => return Err("library declaration is unreadable".into()),
    };
    if bytes.len() > 16 * 1024 {
        return Err("library declaration is too large".into());
    }
    let declared: LibraryDeclarations =
        serde_json::from_slice(&bytes).map_err(|_| "library declaration is invalid")?;
    if declared.libraries.len() > 8 {
        return Err("at most eight prepared libraries are supported".into());
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(workspace.join("lake-manifest.json"))
            .map_err(|_| "dependency manifest is missing")?,
    )
    .map_err(|_| "dependency manifest is invalid")?;
    let mut names = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut libraries = vec![];
    for library in declared.libraries {
        let package = library.package;
        if package.is_empty()
            || package.len() > 64
            || !package
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || package.eq_ignore_ascii_case("mathlib")
            || !names.insert(package.to_ascii_lowercase())
        {
            return Err("invalid or repeated library package".into());
        }
        if library.roots.is_empty() || library.roots.len() > 16 {
            return Err(format!(
                "library {package}: declare one to sixteen module roots"
            ));
        }
        for root in &library.roots {
            if !identifier(root)
                || RESERVED_ROOTS.contains(&root.to_ascii_lowercase().as_str())
                || !roots.insert(root.to_ascii_lowercase())
            {
                return Err(format!(
                    "library {package}: invalid, reserved or repeated root"
                ));
            }
        }
        let entry = manifest["packages"]
            .as_array()
            .and_then(|p| p.iter().find(|p| p["name"] == package.as_str()));
        let revision = entry
            .and_then(|p| p["rev"].as_str())
            .filter(|rev| full_revision(rev))
            .ok_or_else(|| format!("library {package} must pin a full Git revision"))?;
        // Reported for readers who follow a declaration to its source; never fetched.
        let url = entry
            .and_then(|p| p["url"].as_str())
            .filter(|url| {
                url.starts_with("https://")
                    && url.len() <= 300
                    && !url.chars().any(|c| c.is_whitespace() || c.is_control())
            })
            .map(|url| url.trim_end_matches(".git").to_owned());
        libraries.push(Library {
            name: package,
            revision: revision.into(),
            roots: library.roots,
            url,
        });
    }
    Ok(libraries)
}
/// Roots that belong to the checker, Lean or Mathlib's own dependency closure.
const RESERVED_ROOTS: [&str; 16] = [
    "target",
    "solution",
    "check",
    "mathlib",
    "init",
    "lean",
    "std",
    "lake",
    "batteries",
    "aesop",
    "qq",
    "proofwidgets",
    "plausible",
    "importgraph",
    "leansearchclient",
    "cli",
];

/// The prepared verifier is authoritative for new proof work. Published
/// receipts continue to name their own immutable environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub lean_toolchain: String,
    pub mathlib_revision: String,
    /// Prepared libraries beyond Mathlib. The label below stays the Lean/Mathlib
    /// pin that targets carry; a receipt names the library revision it used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub libraries: Vec<Library>,
}
impl Environment {
    pub fn label(&self) -> String {
        format!("{}, Mathlib {}", self.lean_toolchain, self.mathlib_revision)
    }
    pub fn parse(label: &str) -> Result<Self, String> {
        let (lean, rev) = label
            .split_once(", Mathlib ")
            .ok_or("invalid Lean environment")?;
        let version = lean
            .strip_prefix("leanprover/lean4:v")
            .ok_or("invalid Lean toolchain")?;
        if version.is_empty()
            || version.len() > 80
            || !version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
            || rev.len() != 40
            || !rev.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid pinned Lean environment".into());
        }
        Ok(Self {
            lean_toolchain: lean.into(),
            mathlib_revision: rev.into(),
            libraries: vec![],
        })
    }
    pub fn read(workspace: &Path) -> Result<Self, String> {
        let lean = std::fs::read_to_string(workspace.join("lean-toolchain"))
            .map_err(|_| "Lean toolchain pin is missing")?;
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(workspace.join("lake-manifest.json"))
                .map_err(|_| "dependency manifest is missing")?,
        )
        .map_err(|_| "dependency manifest is invalid")?;
        let rev = manifest["packages"]
            .as_array()
            .and_then(|p| p.iter().find(|p| p["name"] == "mathlib"))
            .and_then(|p| p["rev"].as_str())
            .ok_or("Mathlib revision is missing")?;
        let mut environment = Self::parse(&format!("{}, Mathlib {rev}", lean.trim()))?;
        environment.libraries = libraries(workspace)?;
        Ok(environment)
    }
}

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
    for library in libraries(workspace)? {
        let built = workspace
            .join(".lake/packages")
            .join(&library.name)
            .join(".lake/build/lib/lean");
        if !library
            .roots
            .iter()
            .all(|root| built.join(format!("{root}.olean")).is_file() || built.join(root).is_dir())
        {
            return Err(format!(
                "built artifacts of library {} are missing",
                library.name
            ));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_declaration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inspect_constants: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_constants: Vec<String>,
    /// Present only when the checked project imported a prepared library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_use: Option<LibraryUse>,
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
/// What a checked project took from prepared libraries, read from the
/// elaborated environment by the trusted Check module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUse {
    /// Imported libraries with the exact revisions of this workspace.
    pub libraries: Vec<LibraryPin>,
    /// Library declarations that local statements or proof terms reference
    /// directly; sorted, at most `LIBRARY_DECLARATION_CAP`.
    pub declarations: Vec<LibraryDeclaration>,
    /// Number of referenced declarations, including those not listed.
    pub total: u32,
    /// False when the bounded inspection stopped before every local constant.
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryPin {
    pub name: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryDeclaration {
    pub library: String,
    pub module: String,
    pub name: String,
}
pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// Ignore inert comments only when locating hash commands in supporting modules.
/// Quoted literals retain the old conservative hash rule: Lean interpolation
/// and quote characters must not disguise commands as comments.
fn has_hash_command(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
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
        if bytes[i] == b'"' {
            return text.contains('#');
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
/// The declared library root that owns a module name, if any.
fn library_root<'a>(name: &str, roots: &[&'a str]) -> Option<&'a str> {
    roots.iter().copied().find(|root| {
        name == *root
            || name
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('.'))
    })
}
fn allowed_import(name: &str, allowed: &[&str], roots: &[&str]) -> bool {
    allowed.contains(&name)
        || ((name.starts_with("Mathlib.") || library_root(name, roots).is_some())
            && name.split('.').all(|p| {
                !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            }))
}
/// `roots` are prepared library roots; pass none where libraries are not importable.
fn imports(text: &str, allowed: &[&str], roots: &[&str]) -> Result<(), String> {
    let words: Vec<_> = text.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if *word == "import"
            && words
                .get(i + 1)
                .is_none_or(|m| !allowed_import(m, allowed, roots))
        {
            return Err(if roots.is_empty() {
                "import must resolve to Mathlib, Target or a submitted module".into()
            } else {
                "import must resolve to Mathlib, Target, a submitted module or a prepared library"
                    .into()
            });
        }
    }
    // Imports must occupy their own line, one module per command. Reject
    // comment tricks and multi-module imports before invoking any compiler.
    for line in text.lines().filter(|l| l.contains("import")) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2 || parts[0] != "import" || !allowed_import(parts[1], allowed, roots) {
            return Err("imports must be a single allowed module per line".into());
        }
    }
    Ok(())
}
pub fn validate_target(text: &str) -> Result<(), String> {
    validate_target_imports(text, &["Mathlib"])
}
fn validate_target_imports(text: &str, allowed: &[&str]) -> Result<(), String> {
    if text.len() > 800_000 {
        return Err("target is too large".into());
    }
    if let Some(reason) = forbidden(text) {
        return Err(reason);
    }
    // A target never imports a prepared library.
    imports(text, allowed, &[])?;
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
    validate_solution_imports(text, &["Mathlib", "Target"], &[])
}
fn validate_solution_imports(
    text: &str,
    allowed: &[&str],
    roots: &[&str],
) -> Result<Verdict, String> {
    if text.len() > 1_048_576 {
        return Err("solution exceeds 1 MB".into());
    }
    if let Some(reason) = forbidden(text) {
        return Err(reason);
    }
    imports(text, allowed, roots)?;
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
            used_constants: vec![],
            library_use: None,
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
            Ok(Ok((verdict, toolchain, axioms, used_constants, library_use))) => {
                receipt.used_constants = used_constants;
                receipt.library_use = library_use;
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
    async fn check(&self, request: &Request) -> Result<Checked, String> {
        self.check_using(request, None).await
    }
    async fn check_using(
        &self,
        request: &Request,
        binary: Option<&Path>,
    ) -> Result<Checked, String> {
        if request
            .required_declaration
            .as_ref()
            .is_some_and(|name| !valid_declaration_name(name))
        {
            return Err("invalid required declaration".into());
        }
        if request.inspect_constants.len() > 8
            || request
                .inspect_constants
                .iter()
                .any(|name| !valid_declaration_name(name))
        {
            return Err("inspect at most eight fully qualified declarations".into());
        }
        if digest(&request.target) != request.target_digest {
            return Err("target digest mismatch".into());
        }
        // The workspace alone decides which libraries exist; a request cannot add one.
        let libraries = libraries(&self.workspace)?;
        let roots: Vec<&str> = libraries
            .iter()
            .flat_map(|library| library.roots.iter().map(String::as_str))
            .collect();
        let (verdict, modules) =
            validate_project_with(&request.solution, &request.modules, &roots)?;
        let auxiliary = request.required_declaration.is_some();
        if auxiliary {
            if verdict != Verdict::Proved {
                return Err("a named proposition requires a proof, not a disproof".into());
            }
            // Auxiliary types may mention definitions in these exact modules. They
            // cannot refer back to Target, which is compiled after the modules.
            let allowed: Vec<_> = std::iter::once("Mathlib")
                .chain(request.modules.keys().map(String::as_str))
                .collect();
            for source in request.modules.values() {
                imports(source, &allowed, &roots)?;
            }
            validate_target_imports(&request.target, &allowed)?;
        } else {
            validate_target(&request.target)?;
        }
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
        let module_order: Vec<_> = if auxiliary {
            modules
                .iter()
                .map(String::as_str)
                .chain(["Target"])
                .collect()
        } else {
            std::iter::once("Target")
                .chain(modules.iter().map(String::as_str))
                .collect()
        };
        let build_order: Vec<&str> = module_order
            .into_iter()
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
        let mut imported_libraries = BTreeSet::new();
        for text in [&request.target, &request.solution]
            .into_iter()
            .chain(request.modules.values())
        {
            for line in text
                .lines()
                .filter(|l| l.trim_start().starts_with("import "))
            {
                let imported = line.split_whitespace().nth(1).unwrap_or_default();
                if imported.starts_with("Mathlib.") && !artifacts.contains_key(imported) {
                    return Err(format!("missing prepared dependency: {imported}"));
                }
                if let Some(root) = library_root(imported, &roots) {
                    let library = libraries
                        .iter()
                        .find(|library| library.roots.iter().any(|r| r == root))
                        .expect("root belongs to a declared library");
                    // The module must be built by the declaring package itself,
                    // never by another package that ships a module of that name.
                    if !work
                        .join(".lake/packages")
                        .join(&library.name)
                        .join(".lake/build/lib/lean")
                        .join(format!("{}.olean", imported.replace('.', "/")))
                        .is_file()
                    {
                        return Err(format!("missing prepared dependency: {imported}"));
                    }
                    imported_libraries.insert(library.name.clone());
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
                let mut source = check_source(
                    constant,
                    ty,
                    &request.inspect_constants,
                    request.required_declaration.as_deref(),
                    &request.modules,
                );
                if !imported_libraries.is_empty() {
                    source.push_str(&library_inspection(constant, &request.modules, &roots));
                }
                std::fs::write(work.join("Check.lean"), source).map_err(|e| e.to_string())?;
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
                let mut axioms =
                    axioms_of(&text, constant).ok_or("missing independent axiom receipt")?;
                if let Some(required) = &request.required_declaration {
                    axioms.extend(
                        axioms_of(&text, required)
                            .ok_or("missing named proposition axiom receipt")?,
                    );
                    axioms.sort();
                    axioms.dedup();
                }
                if axioms
                    .iter()
                    .any(|a| !STANDARD_AXIOMS.contains(&a.as_str()))
                {
                    return Err("solution depends on non-standard axioms".into());
                }
                let used_constants = request
                    .inspect_constants
                    .iter()
                    .filter(|name| {
                        text.lines().any(|line| {
                            line.trim().ends_with(&format!("MATHESIS_PROOF_USE:{name}"))
                        })
                    })
                    .cloned()
                    .collect();
                let library_use = if imported_libraries.is_empty() {
                    None
                } else {
                    Some(library_use(&text, &libraries, &imported_libraries)?)
                };
                return Ok((verdict, toolchain, axioms, used_constants, library_use));
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
/// Verdict, environment label, axioms, inspected constants and library use.
type Checked = (
    Verdict,
    String,
    Vec<String>,
    Vec<String>,
    Option<LibraryUse>,
);

/// Independently validate all modules, including unused ones, before I/O.
#[cfg(test)]
fn validate_project(
    solution: &str,
    modules: &BTreeMap<String, String>,
) -> Result<(Verdict, Vec<String>), String> {
    validate_project_with(solution, modules, &[])
}
/// `roots` are the prepared library roots of the workspace. A submitted module
/// may import below them and can never be named below them.
fn validate_project_with(
    solution: &str,
    modules: &BTreeMap<String, String>,
    roots: &[&str],
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
            || roots
                .iter()
                .any(|root| root.eq_ignore_ascii_case(name.split('.').next().unwrap_or_default()))
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
    let verdict = validate_solution_imports(solution, &allowed, roots)?;
    let mut dependencies = BTreeMap::new();
    for (name, text) in modules {
        imports(text, &allowed, roots)?;
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

fn valid_declaration_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && name.split('.').all(|part| {
            !part.is_empty()
                && part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}
/// Only this trusted module prints inspection markers. Source-module output is discarded.
fn check_source(
    constant: &str,
    ty: &str,
    candidates: &[String],
    required: Option<&str>,
    modules: &BTreeMap<String, String>,
) -> String {
    let mut source = format!(
        "import Solution\nimport Lean\n#check (_root_.{constant} : {ty})\n#print axioms {constant}\n"
    );
    if candidates.is_empty() && required.is_none() {
        return source;
    }
    let names = candidates
        .iter()
        .map(|name| format!("`{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let module_names = modules
        .keys()
        .map(|name| format!("`{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    if let Some(name) = required {
        source.push_str(&format!(
            "#check (_root_.{name} : _root_.wishpool_target_prop)\n#print axioms {name}\n"
        ));
    }
    source.push_str("\nrun_meta do\n  let env ← Lean.getEnv\n");
    if let Some(name) = required {
        source.push_str(&format!(
            r#"  let some declaration := env.find? `{name} | throwError "missing named proposition"
  unless declaration.isTheorem do throwError "named proposition must be a theorem"
  let some idx := env.getModuleIdxFor? `{name} | throwError "missing proposition source"
  unless ([{module_names}] : List Lean.Name).contains env.header.moduleNames[idx]! do
    throwError "named proposition must be declared in a submitted module"
"#
        ));
    }
    if candidates.is_empty() {
        return source;
    }
    source.push_str(&format!(
        r#"  let localModules : List Lean.Name := [`Solution, `Target{separator}{module_names}]
  let mut seen : Std.HashSet Lean.Name := {{}}
  let mut pending : List Lean.Name := [`{constant}]
  let mut fuel : Nat := 50000
  while !pending.isEmpty do
    if fuel == 0 then throwError "proof inspection limit exceeded"
    fuel := fuel - 1
    let name := pending.head!
    pending := pending.tail!
    if seen.contains name then continue
    seen := seen.insert name
    if let some idx := env.getModuleIdxFor? name then
      unless localModules.contains env.header.moduleNames[idx]! do continue
    if let some info := env.find? name then
      if let some value := info.value? (allowOpaque := true) then
        pending := value.getUsedConstants.toList ++ pending
  for name in ([{names}] : List Lean.Name) do
    if seen.contains name then
      Lean.logInfo m!"MATHESIS_PROOF_USE:{{name}}"
"#,
        separator = if module_names.is_empty() { "" } else { ", " }
    ));
    source
}

/// Appended to the trusted Check module when the project imports a prepared
/// library. It walks the statements and proof terms of local declarations and
/// reports every referenced constant whose declaring module belongs to a library
/// root. Local modules cannot be named below those roots, so a local declaration
/// that merely reuses a library name is never reported.
fn library_inspection(
    constant: &str,
    modules: &BTreeMap<String, String>,
    roots: &[&str],
) -> String {
    let names = |items: &mut dyn Iterator<Item = &str>| {
        items
            .map(|name| format!("`{name}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let local = names(
        &mut ["Solution", "Target"]
            .into_iter()
            .chain(modules.keys().map(String::as_str)),
    );
    let roots = names(&mut roots.iter().copied());
    format!(
        r#"
run_meta do
  let env ← Lean.getEnv
  let localModules : List Lean.Name := [{local}]
  let libraryRoots : List Lean.Name := [{roots}]
  let mut seen : Std.HashSet Lean.Name := {{}}
  let mut pending : List Lean.Name := [`{constant}]
  let mut found : Array String := #[]
  let mut fuel : Nat := 200000
  let mut complete := true
  while !pending.isEmpty do
    if fuel == 0 then
      complete := false
      break
    fuel := fuel - 1
    let name := pending.head!
    pending := pending.tail!
    if seen.contains name then continue
    seen := seen.insert name
    if let some idx := env.getModuleIdxFor? name then
      let owner := env.header.moduleNames[idx]!
      unless localModules.contains owner do
        if libraryRoots.contains owner.getRoot then
          found := found.push s!"{{owner}}:{{name}}"
        continue
    if let some info := env.find? name then
      pending := info.type.getUsedConstants.toList ++ pending
      if let some value := info.value? (allowOpaque := true) then
        pending := value.getUsedConstants.toList ++ pending
  let sorted := found.qsort (· < ·)
  Lean.logInfo s!"MATHESIS_LIBRARY_USE_TOTAL:{{sorted.size}}:{{complete}}"
  for entry in sorted.extract 0 {cap} do
    Lean.logInfo s!"MATHESIS_LIBRARY_USE:{{entry}}"
"#,
        cap = LIBRARY_DECLARATION_CAP
    )
}

/// Read the library report printed by the trusted Check module.
fn library_use(
    text: &str,
    libraries: &[Library],
    imported: &BTreeSet<String>,
) -> Result<LibraryUse, String> {
    let after = |line: &'_ str, marker: &str| -> Option<String> {
        line.split_once(marker)
            .map(|(_, rest)| rest.trim().to_owned())
    };
    let (total, complete) = text
        .lines()
        .find_map(|line| after(line, "MATHESIS_LIBRARY_USE_TOTAL:"))
        .and_then(|rest| {
            let (total, complete) = rest.split_once(':')?;
            Some((total.parse::<u32>().ok()?, complete.parse::<bool>().ok()?))
        })
        .ok_or("missing library inspection receipt")?;
    let mut declarations = vec![];
    for entry in text
        .lines()
        .filter_map(|line| after(line, "MATHESIS_LIBRARY_USE:"))
    {
        let Some((module, name)) = entry.split_once(':') else {
            continue;
        };
        let root = module.split('.').next().unwrap_or_default();
        let Some(library) = libraries
            .iter()
            .find(|library| library.roots.iter().any(|r| r == root))
        else {
            return Err("library inspection named an undeclared library".into());
        };
        // Unusual names stay counted in `total` but are not carried as text.
        if name.is_empty()
            || name.len() > 240
            || name.chars().any(|c| c.is_whitespace() || c.is_control())
            || !module.split('.').all(identifier)
        {
            continue;
        }
        declarations.push(LibraryDeclaration {
            library: library.name.clone(),
            module: module.into(),
            name: name.into(),
        });
    }
    declarations.sort_by(|a, b| (&a.module, &a.name).cmp(&(&b.module, &b.name)));
    declarations.dedup();
    declarations.truncate(LIBRARY_DECLARATION_CAP);
    if declarations.len() as u64 > u64::from(total) {
        return Err("library inspection receipt is inconsistent".into());
    }
    Ok(LibraryUse {
        libraries: libraries
            .iter()
            .filter(|library| imported.contains(&library.name))
            .map(|library| LibraryPin {
                name: library.name.clone(),
                revision: library.revision.clone(),
                url: library.url.clone(),
            })
            .collect(),
        declarations,
        total,
        complete,
    })
}
