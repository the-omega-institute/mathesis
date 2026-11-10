//! Solving aggregates and strictly public identity/receipt types.
use super::{BlobRef, RefereeAudit, RefereeReport};
use crate::ids::{ClaimId, PersonId, RecordId, SubmissionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrantKind {
    Person,
    Agent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrant {
    pub id: String,
    pub name: String,
    pub kind: EntrantKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    pub retired: bool,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub id: PersonId,
    pub name: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Proved,
    Disproved,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    /// Constants reached through the elaborated proof value, never imports or type names.
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
    pub duration: u64,
}
fn full_revision(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
fn lean_identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn library_identity(name: &str, revision: &str, url: Option<&str>) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        && full_revision(revision)
        && url.is_none_or(|url| {
            url.starts_with("https://")
                && url.len() <= 300
                && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        })
}
/// A library, other than Mathlib, prepared in the verifier workspace. Only the
/// verifier decides which exist; Layer 2 reads them through its port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedLibrary {
    pub name: String,
    /// Full Git revision resolved in the verifier workspace.
    pub revision: String,
    /// Top-level module names owned by the library.
    pub roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
impl PreparedLibrary {
    /// Bounded identity, checked before it is pinned, rendered or compared.
    pub fn valid(&self) -> bool {
        library_identity(&self.name, &self.revision, self.url.as_deref())
            && (1..=16).contains(&self.roots.len())
            && self.roots.iter().all(|root| lean_identifier(root))
    }
    /// Whether `module` is one of this library's roots or lies below one.
    pub fn owns(&self, module: &str) -> bool {
        self.roots.iter().any(|root| {
            module == root
                || module
                    .strip_prefix(root.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
        })
    }
}
/// What a checked project took from prepared libraries: the library
/// declarations its local statements and proof terms reference directly.
/// A report of references at a pinned revision, never a statement about
/// semantic equivalence or coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUse {
    /// Imported libraries with the exact revisions of the checking workspace.
    pub libraries: Vec<LibraryPin>,
    /// Sorted; at most 64 are listed, `total` counts all of them.
    pub declarations: Vec<LibraryDeclaration>,
    pub total: u32,
    /// False when the bounded inspection stopped before every local constant.
    pub complete: bool,
}
impl LibraryUse {
    /// Only a bounded, self-consistent report is stored or shown.
    pub fn valid(&self) -> bool {
        let mut names = std::collections::BTreeSet::new();
        (1..=8).contains(&self.libraries.len())
            && self.libraries.iter().all(|pin| {
                library_identity(&pin.name, &pin.revision, pin.url.as_deref())
                    && names.insert(pin.name.as_str())
            })
            && self.declarations.len() <= 64
            && self.declarations.len() as u64 <= u64::from(self.total)
            && self.declarations.iter().all(|declaration| {
                names.contains(declaration.library.as_str())
                    && declaration.module.len() <= 1024
                    && declaration.module.split('.').all(lean_identifier)
                    && !declaration.name.is_empty()
                    && declaration.name.len() <= 240
                    && !declaration
                        .name
                        .chars()
                        .any(|c| c.is_whitespace() || c.is_control())
            })
    }
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
/// Public reading of a verified receipt: whether the checked proof references
/// declarations of a prepared library or is Lean supplied with the attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProofBasis {
    /// The checked target follows from these library declarations at the pinned
    /// revisions together with the submitted Lean.
    LibraryDeclarations {
        libraries: Vec<LibraryPin>,
        declarations: Vec<LibraryReference>,
        total: u32,
        complete: bool,
    },
    /// The checked proof references no declaration of a prepared library.
    NewLean,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryReference {
    pub library: String,
    pub module: String,
    pub name: String,
    /// Source file at the receipt's revision; absent when the receipt has no
    /// repository address with a known layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
impl ProofBasis {
    /// Derived from the saved receipt alone; a rejected receipt has no basis.
    pub fn of(receipt: &VerificationReceipt) -> Option<Self> {
        if receipt.verdict == Verdict::Rejected {
            return None;
        }
        let Some(report) = receipt.library_use.as_ref().filter(|r| r.total > 0) else {
            return Some(Self::NewLean);
        };
        Some(Self::LibraryDeclarations {
            libraries: report.libraries.clone(),
            declarations: report
                .declarations
                .iter()
                .map(|declaration| LibraryReference {
                    library: declaration.library.clone(),
                    module: declaration.module.clone(),
                    name: declaration.name.clone(),
                    source: report
                        .libraries
                        .iter()
                        .find(|pin| pin.name == declaration.library)
                        .and_then(|pin| library_source(pin, &declaration.module)),
                })
                .collect(),
            total: report.total,
            complete: report.complete,
        })
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::LibraryDeclarations { .. } => "library_declarations",
            Self::NewLean => "new_lean",
        }
    }
}
/// Built only from the receipt's repository address, revision and module path.
/// Only GitHub's `/blob/<revision>/<path>` layout is known; other hosts get no link.
fn library_source(pin: &LibraryPin, module: &str) -> Option<String> {
    let url = pin.url.as_deref()?;
    let mut parts = url.strip_prefix("https://github.com/")?.split('/');
    let named = |part: Option<&str>| {
        part.is_some_and(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    };
    if !named(parts.next())
        || !named(parts.next())
        || parts.next().is_some()
        || !full_revision(&pin.revision)
        || !module.split('.').all(lean_identifier)
    {
        return None;
    }
    Some(format!(
        "{url}/blob/{}/{}.lean",
        pin.revision,
        module.replace('.', "/")
    ))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_declaration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inspect_constants: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    pub target: String,
    pub target_digest: String,
    pub toolchain: String,
    pub solution: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<IntermediateProposition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, BlobRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub owner: PersonId,
    pub entrant: Entrant,
    pub target_digest: String,
    pub solution_digest: String,
    pub solution: BlobRef,
    pub note: String,
    pub submitted_at: DateTime<Utc>,
    pub receipt: Option<VerificationReceipt>,
    #[serde(default)]
    pub invalidated: Option<String>,
}
impl Attempt {
    pub fn verified(&self) -> bool {
        self.receipt
            .as_ref()
            .is_some_and(|r| r.verdict != Verdict::Rejected)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveFile {
    /// Exact collected result; absent for ordinary submissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collected: Option<super::CollectedProof>,
    #[serde(default)]
    pub referee_backend: super::RefereeBackend,
    /// Explicit, target-bound lifetime allowance; never replenished by retry/resume.
    #[serde(default)]
    pub authorization: Option<FormalizationAuthorization>,
    #[serde(default)]
    pub agent_attempted: std::collections::BTreeSet<String>,
    /// Number of durable CMA continuation rounds per confirmed target. This
    /// lets a formalization resume after a failed proof instead of replaying
    /// one frozen response from the first run.
    #[serde(default)]
    pub agent_rounds: std::collections::BTreeMap<String, u32>,
    /// Last rejected attempt already consumed by a continuation.
    #[serde(default)]
    pub agent_after_attempt: std::collections::BTreeMap<String, String>,
    pub id: String,
    pub record: RecordId,
    pub submission: SubmissionId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub revision: u64,
    /// Paper-derived candidates need their own sanitized conjecture audit.
    pub admitted: bool,
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub review_round: u32,
    #[serde(default)]
    pub review_contract: u32,
    pub task: Option<String>,
    pub report: Option<RefereeReport>,
    pub audit: Option<RefereeAudit>,
    pub attempts: Vec<Attempt>,
    /// Confirmed S1 edges only. No model-inferred relationships.
    pub dependencies: Vec<DependencyEdge>,
    pub downstream: usize,
}
impl SolveFile {
    pub fn winner(&self, digest: &str) -> Option<&Attempt> {
        self.attempts
            .iter()
            .filter(|a| a.intermediate.is_none() && a.target_digest == digest && a.verified())
            .min_by_key(|a| (a.receipt.as_ref().unwrap().checked_at, &a.id))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicAttempt {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub entrant: Entrant,
    pub receipt: VerificationReceipt,
    /// Whether the checked proof references prepared-library declarations.
    pub proof_basis: ProofBasis,
    pub solution: String,
    pub also_verified: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct AttemptView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<IntermediateProposition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reuses: Vec<ProofReuse>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<super::FormalizationAttribution>,
    pub id: String,
    pub record: RecordId,
    pub claim: ClaimId,
    pub entrant: Entrant,
    pub state: String,
    pub reason: Option<String>,
    pub receipt: Option<VerificationReceipt>,
    /// Present once the attempt is verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_basis: Option<ProofBasis>,
    pub solution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardRow {
    pub rank: usize,
    pub entrant: Entrant,
    pub solved: usize,
    pub disproved: usize,
    pub score: usize,
    pub others: usize,
    pub total: usize,
    pub last_solve: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SolveCredit {
    pub others: bool,
    pub formalized: bool,
    pub record: RecordId,
    pub claim: ClaimId,
    pub title: String,
    pub attempt: String,
    pub receipt: VerificationReceipt,
}
#[derive(Debug, Clone, Serialize)]
pub struct EntrantProfile {
    pub contributions: Vec<IntermediateCredit>,
    pub entrant: Entrant,
    pub solutions: Vec<SolveCredit>,
    pub others: usize,
    pub total: usize,
    pub formalized: Vec<SolveCredit>,
    pub errors_found: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveNotification {
    pub attempt: String,
    pub record: RecordId,
    pub claim: ClaimId,
    pub entrant: Entrant,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConjectureRef {
    pub record: RecordId,
    pub claim: ClaimId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedDependency {
    pub from: ClaimId,
    pub target: ConjectureRef,
    pub target_version: u32,
    pub target_claims_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizationAuthorization {
    pub target_digest: String,
    pub authorized_by: PersonId,
    pub total_generations: u32,
    pub expires_at: DateTime<Utc>,
    pub paused: bool,
    /// Round identities are also the durable managed-provider request identities.
    pub admitted_rounds: std::collections::BTreeSet<u32>,
    #[serde(default)]
    pub contexts: std::collections::BTreeMap<String, GenerationContext>,
}
impl FormalizationAuthorization {
    pub fn active(&self, digest: &str, now: DateTime<Utc>) -> bool {
        !self.paused && self.target_digest == digest && now < self.expires_at
    }
    pub fn can_admit(&self, digest: &str, round: u32, now: DateTime<Utc>) -> bool {
        self.active(digest, now)
            && (self.admitted_rounds.contains(&round)
                || self.admitted_rounds.len() < self.total_generations as usize)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormalizationControl {
    pub target_digest: String,
    pub total_generations: u32,
    pub expires_at: DateTime<Utc>,
    pub paused: bool,
    /// Explicit authorization to change saved limits or rebind a corrected target.
    /// Pause/resume alone cannot extend limits or authorize a replacement target.
    #[serde(default)]
    pub change_allowance: bool,
}

/// Proposed mathematical content. Only an ordinary independent receipt establishes its proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntermediateProposition {
    pub name: String,
    pub title: String,
    pub proposition: String,
    pub license: String,
}
impl IntermediateProposition {
    pub fn target<T>(&self, modules: &std::collections::BTreeMap<String, T>) -> String {
        let imports = modules
            .keys()
            .map(|name| format!("import {name}\n"))
            .collect::<String>();
        format!(
            "import Mathlib\n{imports}def wishpool_target_prop : Prop := {}\n",
            self.proposition
        )
    }
}
/// Immutable references to original checked work. No new credit event is created by reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofReuse {
    pub record: RecordId,
    pub claim: ClaimId,
    pub version: u32,
    pub claims_revision: u64,
    pub attempt: String,
    pub target_digest: String,
    pub solution_digest: String,
    pub declaration: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewIntermediate {
    pub expected_target: String,
    pub intermediate: IntermediateProposition,
    #[serde(default)]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub deposit: Option<String>,
    #[serde(default)]
    pub as_agent: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReusingAttempt {
    pub expected_target: String,
    pub solution: String,
    pub contributions: Vec<String>,
    #[serde(default)]
    pub modules: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub as_agent: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationContext {
    pub contract: u32,
    pub contributions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntermediateCredit {
    pub origin: ProofReuse,
    pub intermediate: IntermediateProposition,
    pub source_title: String,
    pub source_authors: Vec<super::Author>,
    pub attribution: Option<super::FormalizationAttribution>,
    pub receipt: VerificationReceipt,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HISTORICAL: &str = r#"{"verdict":"proved","reason":"Lean checked the imported target and standard axioms","target_digest":"t","solution_digest":"s","toolchain":"leanprover/lean4:v4.35.0, Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679","axioms":["propext"],"checked_at":"2026-11-01T00:00:00Z","duration":7}"#;

    fn report(url: Option<&str>) -> LibraryUse {
        LibraryUse {
            libraries: vec![LibraryPin {
                name: "trureturing".into(),
                revision: "b".repeat(40),
                url: url.map(str::to_owned),
            }],
            declarations: vec![LibraryDeclaration {
                library: "trureturing".into(),
                module: "D5.S3.Derangements".into(),
                name: "D5.S3.Derangements.result".into(),
            }],
            total: 1,
            complete: true,
        }
    }

    #[test]
    fn historical_receipts_keep_their_exact_bytes_and_library_reports_round_trip() {
        let mut receipt: VerificationReceipt = serde_json::from_str(HISTORICAL).unwrap();
        assert!(receipt.library_use.is_none());
        assert_eq!(serde_json::to_string(&receipt).unwrap(), HISTORICAL);
        assert_eq!(ProofBasis::of(&receipt), Some(ProofBasis::NewLean));

        receipt.library_use = Some(report(Some("https://github.com/example/library")));
        let text = serde_json::to_string(&receipt).unwrap();
        // Same position as the verifier's own receipt.
        assert!(text.starts_with(r#"{"library_use":{"libraries":[{"name":"trureturing""#));
        assert_eq!(
            serde_json::from_str::<VerificationReceipt>(&text).unwrap(),
            receipt
        );
    }

    #[test]
    fn proof_basis_reads_only_the_receipt_and_links_only_a_known_repository_layout() {
        let mut receipt: VerificationReceipt = serde_json::from_str(HISTORICAL).unwrap();
        receipt.library_use = Some(report(Some("https://github.com/example/library")));
        let Some(ProofBasis::LibraryDeclarations {
            libraries,
            declarations,
            total,
            complete,
        }) = ProofBasis::of(&receipt)
        else {
            panic!("library declarations expected");
        };
        assert_eq!((libraries.len(), total, complete), (1, 1, true));
        assert_eq!(
            declarations[0].source.as_deref(),
            Some(
                format!(
                    "https://github.com/example/library/blob/{}/D5/S3/Derangements.lean",
                    "b".repeat(40)
                )
                .as_str()
            )
        );
        assert_eq!(
            serde_json::to_value(ProofBasis::of(&receipt).unwrap()).unwrap()["kind"],
            "library_declarations"
        );
        for url in [None, Some("https://example.org/library")] {
            receipt.library_use = Some(report(url));
            let Some(ProofBasis::LibraryDeclarations { declarations, .. }) =
                ProofBasis::of(&receipt)
            else {
                panic!("library declarations expected");
            };
            assert!(declarations[0].source.is_none(), "{url:?}");
        }
        // An imported library whose declarations the proof never references.
        let mut unused = report(None);
        unused.declarations.clear();
        unused.total = 0;
        receipt.library_use = Some(unused);
        assert_eq!(ProofBasis::of(&receipt), Some(ProofBasis::NewLean));
        assert_eq!(
            serde_json::to_string(&ProofBasis::NewLean).unwrap(),
            r#"{"kind":"new_lean"}"#
        );
        receipt.verdict = Verdict::Rejected;
        assert_eq!(ProofBasis::of(&receipt), None);
    }

    #[test]
    fn only_bounded_consistent_library_reports_and_identities_are_valid() {
        assert!(report(None).valid());
        assert!(report(Some("https://github.com/example/library")).valid());
        let broken: [fn(&mut LibraryUse); 8] = [
            |r| r.libraries.clear(),
            |r| r.libraries[0].revision = "main".into(),
            |r| r.libraries[0].url = Some("http://example.org/x".into()),
            |r| r.libraries[0].name = "tru returing".into(),
            |r| r.declarations[0].library = "other".into(),
            |r| r.declarations[0].module = "D5..S3".into(),
            |r| r.declarations[0].name = "a b".into(),
            |r| r.total = 0,
        ];
        for change in broken {
            let mut r = report(None);
            change(&mut r);
            assert!(!r.valid(), "{r:?}");
        }
        let mut many = report(None);
        many.declarations = (0..65)
            .map(|i| LibraryDeclaration {
                library: "trureturing".into(),
                module: "D5.M".into(),
                name: format!("d{i}"),
            })
            .collect();
        many.total = 65;
        assert!(!many.valid());

        let library = PreparedLibrary {
            name: "trureturing".into(),
            revision: "b".repeat(40),
            roots: vec!["D5".into(), "Trureturing".into()],
            url: None,
        };
        assert!(library.valid());
        assert!(library.owns("D5") && library.owns("D5.S3.X") && library.owns("Trureturing"));
        assert!(!library.owns("D50.X") && !library.owns("d5.X") && !library.owns("Mathlib.D5"));
        for change in [
            (|l| l.roots.clear()) as fn(&mut PreparedLibrary),
            |l| l.roots = vec!["D5.S3".into()],
            |l| l.revision = "b".repeat(39),
            |l| l.url = Some("https://example.org/a b".into()),
        ] {
            let mut l = library.clone();
            change(&mut l);
            assert!(!l.valid(), "{l:?}");
        }
    }
}
