//! Public Mathesis catalog collection, deterministic rendering and checking.
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tar::Archive;

pub const GENERATOR_VERSION: &str = "0.1.0";
pub const SNAPSHOT_SCHEMA: &str = "1";
pub const CATALOG_SCHEMA: &str = "1";
pub const MAX_DOWNLOAD_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_ARCHIVE_ENTRIES: usize = 4096;
pub const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Io(io::Error),
    Json(serde_json::Error),
    Http(reqwest::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(s) => f.write_str(s),
            Self::Io(e) => e.fmt(f),
            Self::Json(e) => e.fmt(f),
            Self::Http(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingCount {
    pub pages: usize,
    pub rows: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub schema_version: String,
    pub generator_version: String,
    pub api_base: String,
    pub captured_at: String,
    pub listings: BTreeMap<String, ListingCount>,
    pub collected_works: usize,
    pub accepted_records: usize,
    pub directory_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: PathBuf,
    pub meta: SnapshotMeta,
    pub collected: Vec<Value>,
    pub accepted: Vec<Value>,
    pub opportunities: Vec<Value>,
    pub proofs: Vec<(String, Value)>,
}

fn msg<T: Into<String>>(s: T) -> Error {
    Error::Message(s.into())
}
fn json_bytes(v: &Value) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(v)?)
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = json_bytes(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}
fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}
fn sha256_file(path: &Path) -> Result<String> {
    Ok(sha256(&fs::read(path)?))
}
fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}
fn arr(v: &Value, key: &str) -> Vec<Value> {
    v.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
fn normalize_base(s: &str) -> String {
    s.trim_end_matches('/').to_owned()
}
fn public_root(s: &str) -> String {
    let b = normalize_base(s);
    b.strip_suffix("/api/v1").unwrap_or(&b).to_owned()
}
fn api_root(s: &str) -> String {
    let b = public_root(s);
    format!("{b}/api/v1")
}

fn http_client() -> Result<Client> {
    Ok(Client::builder().timeout(Duration::from_secs(90)).build()?)
}
fn request_bytes(client: &Client, url: &str) -> Result<Vec<u8>> {
    let mut last = String::new();
    for attempt in 0..4u64 {
        let response = client
            .get(url)
            .header(USER_AGENT, format!("mathesis-catalog/{GENERATOR_VERSION}"))
            .send();
        match response {
            Ok(r) if r.status().is_success() => {
                let mut out = Vec::new();
                let mut limited = r.take((MAX_DOWNLOAD_BYTES + 1) as u64);
                limited.read_to_end(&mut out)?;
                if out.len() > MAX_DOWNLOAD_BYTES {
                    return Err(msg(format!(
                        "download exceeds {} bytes: {url}",
                        MAX_DOWNLOAD_BYTES
                    )));
                }
                return Ok(out);
            }
            Ok(r) if r.status().as_u16() == 429 || r.status().is_server_error() => {
                last = format!("HTTP {}", r.status());
            }
            Ok(r) => return Err(msg(format!("GET {url} failed with {}", r.status()))),
            Err(e) => {
                last = e.to_string();
            }
        }
        if attempt < 3 {
            std::thread::sleep(Duration::from_millis(100 * (1 << attempt)));
        }
    }
    Err(msg(format!("GET {url} failed after retries: {last}")))
}
fn get_json(client: &Client, url: &str) -> Result<Value> {
    let bytes = request_bytes(client, url)?;
    serde_json::from_slice(&bytes).map_err(|e| {
        msg(format!(
            "invalid JSON from {url}: {e}; prefix={:?}",
            String::from_utf8_lossy(&bytes[..bytes.len().min(160)])
        ))
    })
}

fn page_url(base: &str, path: &str, before: Option<&str>) -> String {
    let mut u = format!("{}{}?limit=100", normalize_base(base), path);
    if let Some(cursor) = before {
        u.push_str("&before=");
        u.push_str(&urlencoding(cursor));
    }
    u
}
fn urlencoding(s: &str) -> String {
    s.bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => vec![b as char],
            x => format!("%{x:02X}").chars().collect(),
        })
        .collect()
}
fn listing_page(value: Value) -> Result<(Vec<Value>, Option<String>)> {
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| msg("listing response has no items array"))?;
    let cursor = value
        .get("next_before")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok((items, cursor))
}
fn collect_pages(
    client: &Client,
    base: &str,
    path: &str,
    dir: &Path,
) -> Result<(Vec<Value>, ListingCount)> {
    let mut all = Vec::new();
    let mut before = None;
    let mut pages = 0;
    loop {
        pages += 1;
        let value = get_json(client, &page_url(base, path, before.as_deref()))?;
        let (items, next) = listing_page(value.clone())?;
        write_json(&dir.join(format!("page-{pages:04}.json")), &value)?;
        all.extend(items);
        if next.is_none() {
            break;
        }
        before = next;
        if pages > 100_000 {
            return Err(msg(format!("pagination limit exceeded for {path}")));
        }
    }
    Ok((
        all.clone(),
        ListingCount {
            pages,
            rows: all.len(),
        },
    ))
}
fn collect_status_pages(
    client: &Client,
    base: &str,
    status: Option<&str>,
    dir: &Path,
) -> Result<(Vec<Value>, ListingCount)> {
    let mut all = Vec::new();
    let mut before = None;
    let mut pages = 0;
    loop {
        pages += 1;
        let mut path = "/formalizations/opportunities?limit=100".to_owned();
        if let Some(s) = status {
            path.push_str("&status=");
            path.push_str(s);
        }
        if let Some(b) = before.as_deref() {
            path.push_str("&before=");
            path.push_str(&urlencoding(b));
        }
        let url = format!("{}{}", normalize_base(base), path);
        let value = get_json(client, &url)?;
        let (items, next) = listing_page(value.clone())?;
        write_json(&dir.join(format!("page-{pages:04}.json")), &value)?;
        all.extend(items);
        if next.is_none() {
            break;
        }
        before = next;
        if pages > 100_000 {
            return Err(msg("formalization pagination limit exceeded"));
        }
    }
    Ok((
        all.clone(),
        ListingCount {
            pages,
            rows: all.len(),
        },
    ))
}
fn unique_node_key(record: &str, version: i64, claim: &str) -> String {
    format!("collected:{record}:{version}:{claim}")
}
fn opportunity_key(v: &Value) -> Option<String> {
    Some(unique_node_key(
        text(v, "record")?.as_str(),
        v.get("version")?.as_i64()?,
        text(v, "claim")?.as_str(),
    ))
}
fn proof_key(work: &str, v: &Value) -> Option<String> {
    Some(unique_node_key(
        work,
        v.get("version")?.as_i64()?,
        text(v, "claim")?.as_str(),
    ))
}

/// Traverse every listing page and every formalization status into an atomic snapshot.
mod archive;
mod check;
mod cli;
mod fetch;
mod formalization;
mod model;
mod publishing;
mod render;
mod snapshot;
mod stats;

pub use archive::unpack_archive;
pub use check::check;
pub use cli::run_cli;
pub use fetch::fetch_files;
pub use model::{LeanSource, MscStats, Node, Stats, Work};
pub(crate) use model::{descriptor_hash, descriptor_url, make_nodes, mapping_file, paper_work};
#[cfg(test)]
pub(crate) use model::{repository_name, valid_repo_name};
#[cfg(test)]
pub(crate) use render::{citation, collect_files};
pub use render::{render, render_with_materials};
pub(crate) use snapshot::atomic_replace_dir;
pub(crate) use snapshot::safe_component;
pub(crate) use snapshot::work_dir_name;
pub use snapshot::{load_snapshot, snapshot};
pub(crate) use stats::{compute_stats, is_verified, msc_labels};

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        msg("formatting failed")
    }
}
impl PartialEq for Stats {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_value(self).ok() == serde_json::to_value(other).ok()
    }
}
impl Eq for Stats {}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod formalization_tests;
