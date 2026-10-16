//! Read-only Trureturing publication adapter. No Lean execution or admission
//! judgement occurs here. The binary supplies bytes and an independently
//! admitted publication identity; self-asserted transport trust is ignored.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ReviewError, ReviewResult};

pub mod index;
pub mod readback;
#[cfg(test)]
mod tests;

const DOMAIN: &[u8] = b"trureturing:statement:v1\0";
pub const MAX_MATERIAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicationIdentity {
    pub bundle_digest: String,
    pub source_commit: String,
    pub source_tree: String,
}

impl PublicationIdentity {
    pub fn validate(&self) -> ReviewResult<()> {
        let git_id = |s: &str| {
            s.len() == 40
                && s.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        };
        ensure(
            hash_id(&self.bundle_digest)
                && git_id(&self.source_commit)
                && git_id(&self.source_tree),
            "invalid admitted publication identity",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclarationIdentity {
    pub snapshot: String,
    pub module_path: String,
    pub name_key: String,
    pub statement_id: String,
    pub type_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedDeclaration {
    pub identity: DeclarationIdentity,
    pub name: String,
    pub kind: String,
    pub frozen_node_id: String,
    pub freeze_status: String,
    pub source_sha256: String,
    pub axioms: Vec<String>,
    /// These are module imports, never declaration proof dependencies.
    pub module_imports: Vec<String>,
    /// These are module-level frozen prerequisites, not declaration dependencies.
    pub prerequisite_frozen_node_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDeclaration {
    pub declaration: PublishedDeclaration,
    /// Inspector canonical expression, not pretty Lean syntax or a proof.
    pub canonical_statement: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumedPublication {
    pub identity: PublicationIdentity,
    pub declarations: Vec<MaterialDeclaration>,
    /// Empty means all exported modules were requested; never means all math.
    pub selected_modules: BTreeSet<String>,
    pub materials_sha256: String,
}

/// Consume existing original sidecars and a ZIP without extracting or executing
/// anything. The archive hash covers all bytes; declaration identity validation
/// covers exactly the requested modules. Selection is retained in the result.
pub fn consume<R: Read + Seek>(
    expected: &PublicationIdentity,
    files: &BTreeMap<String, Vec<u8>>,
    sidecars: &BTreeMap<String, Vec<u8>>,
    materials: R,
    selected_modules: &BTreeSet<String>,
) -> ReviewResult<ConsumedPublication> {
    let declarations = catalog(expected, files, selected_modules)?;
    let (mut archive, archive_hash) = inspect_archive(expected, files, sidecars, materials)?;
    let mut consumed = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let name = format!("sha256/{}", &declaration.identity.type_sha256[7..]);
        let mut entry = archive
            .by_name(&name)
            .map_err(|_| invalid("declaration material missing"))?;
        ensure(
            entry.size() <= MAX_MATERIAL_BYTES as u64,
            "material exceeds input limit",
        )?;
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_MATERIAL_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid("cannot decode declaration material"))?;
        verify_material(&declaration, &bytes)?;
        consumed.push(MaterialDeclaration {
            declaration,
            canonical_statement: String::from_utf8(bytes)
                .map_err(|_| invalid("invalid UTF-8 material"))?,
        });
    }
    Ok(ConsumedPublication {
        identity: expected.clone(),
        declarations: consumed,
        selected_modules: selected_modules.clone(),
        materials_sha256: archive_hash,
    })
}

/// Validate original provenance and the whole archive without expanding canonical
/// expressions. This validates the container, not each declaration material.
/// Call `catalog` first to bind all projection bytes to the admitted identity.
pub fn inspect_archive<R: Read + Seek>(
    expected: &PublicationIdentity,
    files: &BTreeMap<String, Vec<u8>>,
    sidecars: &BTreeMap<String, Vec<u8>>,
    mut materials: R,
) -> ReviewResult<(zip::ZipArchive<R>, String)> {
    let provenance: serde_json::Value = decode(file(files, "report-provenance.v1.json")?)?;
    ensure(
        provenance["schema"] == "pages-report-provenance.v1"
            && provenance["source_commit"] == expected.source_commit
            && provenance["source_tree"] == expected.source_tree
            && provenance["source_repo"] == "the-omega-institute/trureturing",
        "report source mismatch",
    )?;
    let required = provenance["report_files"]
        .as_object()
        .ok_or_else(|| invalid("report file identities missing"))?;
    ensure(required.len() == 5, "unsupported report sidecar inventory")?;
    let report_hash = sha256(file(files, "raw-lean-report.json")?);
    ensure(
        required
            .get("raw-lean-report.json")
            .and_then(|v| v.as_str())
            == Some(&report_hash),
        "original report checksum mismatch",
    )?;
    for name in [
        "raw-lean-report.json.sha256",
        "raw-lean-report.json.input.attestation",
        "raw-lean-report.json.provenance.json",
    ] {
        ensure(
            required.get(name).and_then(|v| v.as_str()) == Some(&sha256(file(sidecars, name)?)),
            "original sidecar checksum mismatch",
        )?;
    }
    ensure(
        file(sidecars, "raw-lean-report.json.sha256")?
            == format!("{}  raw-lean-report.json\n", &report_hash[7..]).as_bytes(),
        "report sha256 sidecar mismatch",
    )?;
    validate_original_provenance(
        file(sidecars, "raw-lean-report.json.provenance.json")?,
        file(sidecars, "raw-lean-report.json.input.attestation")?,
        &report_hash,
    )?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = materials
            .read(&mut buffer)
            .map_err(|_| invalid("cannot read materials archive"))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure(
            total <= 4 * 1024 * 1024 * 1024,
            "materials archive exceeds byte limit",
        )?;
        hash.update(&buffer[..count]);
    }
    let archive_hash = format!("sha256:{:x}", hash.finalize());
    ensure(
        required
            .get("raw-lean-report.json.materials.zip")
            .and_then(|v| v.as_str())
            == Some(&archive_hash),
        "materials archive checksum mismatch",
    )?;
    materials
        .seek(SeekFrom::Start(0))
        .map_err(|_| invalid("cannot rewind materials archive"))?;
    let mut archive =
        zip::ZipArchive::new(materials).map_err(|_| invalid("invalid materials ZIP"))?;
    ensure(
        archive.len() <= 1_000_000,
        "materials entry count exceeds limit",
    )?;
    let mut names = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| invalid("cannot read materials entry"))?;
        let name = entry
            .name()
            .map_err(|_| invalid("invalid materials entry name"))?;
        ensure(
            name.starts_with("sha256/")
                && hash_id(&name.replacen('/', ":", 1))
                && names.insert(name.into_owned())
                && !entry.is_dir()
                && !entry.encrypted()
                && entry
                    .unix_mode()
                    .is_none_or(|mode| mode & 0o170000 == 0 || mode & 0o170000 == 0o100000),
            "invalid or duplicate materials entry",
        )?;
    }
    Ok((archive, archive_hash))
}

#[derive(Debug, Deserialize)]
struct Envelope {
    schema: String,
    bundle_ref: String,
    release_digest: String,
    source_commit: String,
    source_tree: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: String,
    sha256sums_digest: String,
    source: Source,
    artifacts: BTreeMap<String, Artifact>,
}

#[derive(Debug, Deserialize)]
struct Source {
    source_repo: String,
    source_commit: String,
    source_tree: String,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    file: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct Export {
    schema: String,
    schema_version: u32,
    dialect: String,
    source_commit: String,
    source_tree: String,
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
struct Node {
    repo_path: String,
    frozen_node_id: String,
    freeze_status: String,
    prerequisite_frozen_node_ids: Vec<String>,
    declarations: Vec<ExportDeclaration>,
}

#[derive(Debug, Deserialize)]
struct ExportDeclaration {
    declaration_name_key: String,
    kind: String,
    statement_id: String,
}

#[derive(Debug, Deserialize)]
struct Report {
    schema: String,
    modules: Vec<Module>,
}

#[derive(Debug, Deserialize)]
struct Module {
    module: String,
    source_path: String,
    source_sha256: String,
    imports: Vec<String>,
    declarations: Vec<Declaration>,
}

#[derive(Debug, Deserialize)]
struct Declaration {
    name: String,
    name_key: String,
    kind: String,
    include_in_statement: bool,
    statement_id: String,
    type_sha256: String,
    axioms: Vec<String>,
}

fn invalid(reason: &str) -> ReviewError {
    ReviewError::Output(format!("knowledge publication: {reason}"))
}

fn ensure(condition: bool, reason: &str) -> ReviewResult<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(reason))
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn hash_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|value| {
        value.len() == 64
            && value
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

fn input_coordinates(
    producer: &str,
    resident: &str,
    sources: &str,
    config: &str,
) -> (String, String) {
    let common = format!(
        "repository_inspector_sha256={resident}\nlean_sources_sha256={sources}\nlean_config_sha256={config}\n"
    );
    let pair =
        format!("schema=stratalint-lean-report-input-v1\nproducer_sha256={producer}\n{common}");
    let repository = format!("schema=stratalint-lean-report-repository-input-v1\n{common}");
    (sha256(pair.as_bytes()), sha256(repository.as_bytes()))
}

fn validate_original_provenance(
    bytes: &[u8],
    attestation: &[u8],
    report_hash: &str,
) -> ReviewResult<()> {
    let original: serde_json::Value = decode(bytes)?;
    let field = |name: &str| {
        original[name]
            .as_str()
            .ok_or_else(|| invalid("missing original provenance field"))
    };
    let names = [
        "producer_sha256",
        "repository_inspector_sha256",
        "lean_sources_sha256",
        "lean_config_sha256",
    ];
    for name in names {
        ensure(
            hash_id(&format!("sha256:{}", field(name)?)),
            "invalid original provenance digest",
        )?;
    }
    ensure(
        original["schema"] == "stratalint-lean-report-provenance-v4"
            && original["side"] == "candidate"
            && original["source_side"] == "candidate"
            && matches!(original["mode"].as_str(), Some("produced" | "cached"))
            && original["module_origins"].is_object()
            && field("report_sha256")? == &report_hash[7..],
        "original report provenance mismatch",
    )?;
    let (input, repository) = input_coordinates(
        field(names[0])?,
        field(names[1])?,
        field(names[2])?,
        field(names[3])?,
    );
    ensure(
        original["input_address"] == input,
        "report input coordinate mismatch",
    )?;
    let expected = format!(
        "schema=stratalint-lean-report-input-attestation-v1\nrepository_input_sha256={}\nproducer_sha256={}\nreport_sha256={}\n",
        &repository[7..],
        field("producer_sha256")?,
        &report_hash[7..]
    );
    ensure(
        attestation == expected.as_bytes(),
        "report input attestation mismatch",
    )
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> ReviewResult<T> {
    ensure(
        bytes.len() <= MAX_REPORT_BYTES,
        "artifact exceeds input limit",
    )?;
    serde_json::from_slice(bytes).map_err(|_| invalid("invalid artifact JSON"))
}

fn file<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> ReviewResult<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("required artifact missing"))
}

/// Verifies the complete Pages transport projection and joins native export
/// identities to the original inspector report. PublicationIdentity must come
/// from a separately authenticated admission, not from these same bytes.
pub fn catalog(
    expected: &PublicationIdentity,
    files: &BTreeMap<String, Vec<u8>>,
    selected_modules: &BTreeSet<String>,
) -> ReviewResult<Vec<PublishedDeclaration>> {
    expected.validate()?;
    let sums = file(files, "SHA256SUMS")?;
    ensure(
        sha256(sums) == expected.bundle_digest,
        "release digest mismatch",
    )?;
    let manifest: Manifest = decode(file(files, "release-manifest.v1.json")?)?;
    let envelope: Envelope = decode(file(files, "truth-release-publication.v1.json")?)?;
    ensure(
        manifest.schema == "pages-report-projection.v1"
            && envelope.schema == "pages-source-publication.v1",
        "unsupported publication schema",
    )?;
    ensure(
        manifest.source.source_repo == "the-omega-institute/trureturing"
            && manifest.source.source_commit == expected.source_commit
            && manifest.source.source_tree == expected.source_tree
            && envelope.source_commit == expected.source_commit
            && envelope.source_tree == expected.source_tree
            && envelope.bundle_ref == expected.bundle_digest
            && envelope.release_digest == expected.bundle_digest
            && manifest.sha256sums_digest == expected.bundle_digest,
        "publication identity mismatch",
    )?;
    let expected_files = [
        ("raw_lean_report", "raw-lean-report.json"),
        ("report_provenance", "report-provenance.v1.json"),
        ("source_snapshot", "source-snapshot.v1.json"),
        ("truth_export", "truth-export.v1.json"),
        ("truth_graph", "truth-graph.v1.json"),
    ];
    ensure(
        manifest.artifacts.len() == expected_files.len(),
        "unexpected artifact inventory",
    )?;
    let mut lines = BTreeMap::new();
    for (key, name) in expected_files {
        let artifact = manifest
            .artifacts
            .get(key)
            .ok_or_else(|| invalid("missing artifact identity"))?;
        ensure(
            artifact.file == name && hash_id(&artifact.sha256),
            "invalid artifact path or digest",
        )?;
        ensure(
            sha256(file(files, name)?) == artifact.sha256,
            "artifact checksum mismatch",
        )?;
        lines.insert(name, format!("{}  {name}\n", &artifact.sha256[7..]));
    }
    ensure(
        lines.values().cloned().collect::<String>().as_bytes() == sums,
        "checksum inventory mismatch",
    )?;
    let snapshot: serde_json::Value = decode(file(files, "source-snapshot.v1.json")?)?;
    ensure(
        snapshot["source_commit"] == expected.source_commit
            && snapshot["source_tree"] == expected.source_tree,
        "snapshot source mismatch",
    )?;
    let export: Export = decode(file(files, "truth-export.v1.json")?)?;
    ensure(
        export.schema == "stratalint.truth-export"
            && export.schema_version == 2
            && export.dialect == "stratalint.truth-export.v2"
            && export.source_commit == expected.source_commit
            && export.source_tree == expected.source_tree,
        "native export identity mismatch",
    )?;
    let report: Report = decode(file(files, "raw-lean-report.json")?)?;
    ensure(
        report.schema == "stratalint-raw-lean-report-v3",
        "unsupported report schema",
    )?;
    let mut modules = BTreeMap::new();
    for module in report.modules {
        ensure(
            module.source_path == format!("{}.lean", module.module.replace('.', "/"))
                && hash_id(&module.source_sha256),
            "invalid report module identity",
        )?;
        ensure(
            modules.insert(module.source_path.clone(), module).is_none(),
            "duplicate report module",
        )?;
    }
    let mut seen = BTreeSet::new();
    let mut catalog = Vec::new();
    for node in export.nodes {
        ensure(
            seen.insert(node.repo_path.clone()),
            "duplicate export module",
        )?;
        ensure(
            hash_id(&node.frozen_node_id)
                && matches!(
                    node.freeze_status.as_str(),
                    "frozen" | "proven-not-yet-frozen"
                )
                && node
                    .prerequisite_frozen_node_ids
                    .iter()
                    .all(|id| hash_id(id)),
            "invalid export node",
        )?;
        let module = modules
            .get(&node.repo_path)
            .ok_or_else(|| invalid("export module absent from report"))?;
        let mut declarations = BTreeMap::new();
        for declaration in &module.declarations {
            ensure(
                declarations
                    .insert(&declaration.name_key, declaration)
                    .is_none(),
                "duplicate declaration name key",
            )?;
        }
        let mut exported = BTreeSet::new();
        for declaration in node.declarations {
            ensure(
                exported.insert(declaration.declaration_name_key.clone()),
                "duplicate export declaration",
            )?;
            let raw = declarations
                .get(&declaration.declaration_name_key)
                .ok_or_else(|| invalid("export declaration absent from report"))?;
            ensure(
                raw.include_in_statement
                    && declaration.kind == raw.kind
                    && declaration.statement_id == raw.statement_id
                    && hash_id(&raw.statement_id)
                    && hash_id(&raw.type_sha256),
                "declaration identity mismatch",
            )?;
            if !selected_modules.is_empty() && !selected_modules.contains(&node.repo_path) {
                continue;
            }
            catalog.push(PublishedDeclaration {
                identity: DeclarationIdentity {
                    snapshot: expected.bundle_digest.clone(),
                    module_path: node.repo_path.clone(),
                    name_key: raw.name_key.clone(),
                    statement_id: raw.statement_id.clone(),
                    type_sha256: raw.type_sha256.clone(),
                },
                name: raw.name.clone(),
                kind: raw.kind.clone(),
                frozen_node_id: node.frozen_node_id.clone(),
                freeze_status: node.freeze_status.clone(),
                source_sha256: module.source_sha256.clone(),
                axioms: raw.axioms.clone(),
                module_imports: module.imports.clone(),
                prerequisite_frozen_node_ids: node.prerequisite_frozen_node_ids.clone(),
            });
        }
    }
    ensure(
        selected_modules.is_subset(&seen),
        "requested module absent from export",
    )?;
    Ok(catalog)
}

/// Existing inspector protocol: UTF-8 JSON with spaces and uppercase surrogate
/// pairs for supplementary Unicode scalars. This is byte identity validation,
/// not a reimplementation of a mathematical judge.
fn canonical_string(value: &str) -> String {
    let encoded = serde_json::to_string(value).expect("string serialization cannot fail");
    let mut result = String::with_capacity(encoded.len());
    for c in encoded.chars() {
        if (c as u32) >= 0x10000 {
            let offset = c as u32 - 0x10000;
            result.push_str(&format!(
                "\\u{:04X}\\u{:04X}",
                0xD800 + offset / 0x400,
                0xDC00 + offset % 0x400
            ));
        } else {
            result.push(c);
        }
    }
    result
}

pub fn material_identities(
    module_path: &str,
    kind: &str,
    name_key: &str,
    material: &[u8],
) -> ReviewResult<(String, String)> {
    ensure(
        material.len() <= MAX_MATERIAL_BYTES,
        "material exceeds input limit",
    )?;
    let text = std::str::from_utf8(material).map_err(|_| invalid("material is not UTF-8"))?;
    ensure(
        text.starts_with("statement-v1("),
        "unsupported statement material",
    )?;
    let canonical = format!(
        "{{\"declaration_name_key\": {}, \"kind\": {}, \"module_path\": {}, \"schema\": \"declaration-statement-v1\", \"statement_material\": {}}}\n",
        canonical_string(name_key),
        canonical_string(kind),
        canonical_string(module_path),
        canonical_string(text),
    );
    let hash = |bytes: &[u8]| {
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update(bytes);
        format!("sha256:{:x}", hash.finalize())
    };
    Ok((hash(material), hash(canonical.as_bytes())))
}

pub fn verify_material(declaration: &PublishedDeclaration, material: &[u8]) -> ReviewResult<()> {
    let (raw, identified) = material_identities(
        &declaration.identity.module_path,
        &declaration.kind,
        &declaration.identity.name_key,
        material,
    )?;
    ensure(
        raw == declaration.identity.type_sha256 && identified == declaration.identity.statement_id,
        "statement material identity mismatch",
    )
}
