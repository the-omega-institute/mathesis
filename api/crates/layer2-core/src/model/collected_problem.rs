//! Curated open problems collected directly by a trusted operator. Collection is
//! distinct from review: a status here is the curator's dated reading of named
//! sources, never a Mathesis audit result.
use super::{Source, SourceContributor, require_text, validate_msc};
use crate::{CoreError, CoreResult, ids::PersonId};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub const COLLECTED_PROBLEM_SCHEMA: &str = "mathesis.collected-problem.v1";
/// Licences a curated restatement may carry. Share-alike is admitted because
/// common sources (OEIS, MathOverflow) are themselves share-alike.
pub const COLLECTED_PROBLEM_LICENSES: [&str; 3] = ["CC-BY-4.0", "CC-BY-SA-4.0", "CC0-1.0"];

/// The curator's reading at intake. A settlement is never asserted through collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuratedStatus {
    Open,
    PartiallyResolved,
}

/// One public page the curator opened, the day it was opened and what it showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEvidence {
    pub url: String,
    pub checked_on: NaiveDate,
    pub observed: String,
}

/// The declaration every Lean target carries exactly once.
pub const LEAN_TARGET_MARKER: &str = "def wishpool_target_prop : Prop :=";

/// The curator's own Lean reading of the curated statement. It is the collection
/// operator's formalization, never one confirmed by the original poser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedLeanTarget {
    /// Exact `Target.lean` text: `import Mathlib`, the curator's definitions and
    /// one `def wishpool_target_prop : Prop := <statement>`.
    pub text: String,
    /// How the Lean proposition reads the curated statement, in plain words.
    pub correspondence: String,
}

/// Whether a target states its proposition in Mathlib's vocabulary alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetVocabulary {
    Mathlib,
    OwnDefinitions,
}

impl CuratedLeanTarget {
    pub fn validate(&self) -> CoreResult<()> {
        require_text("Lean target", &self.text, 200_000)?;
        require_text("Lean target correspondence", &self.correspondence, 5_000)?;
        let imports = self
            .text
            .lines()
            .filter(|line| line.trim_start().starts_with("import "))
            .count();
        if !self.text.starts_with("import Mathlib\n") || imports != 1 {
            return Err(CoreError::invalid(
                "a Lean target starts with `import Mathlib` and imports nothing else",
            ));
        }
        // One occurrence of the name keeps the elaboration witness an exact
        // rewrite of this text.
        if self.text.matches(LEAN_TARGET_MARKER).count() != 1
            || self.text.matches("wishpool_target_prop").count() != 1
            || self.proposition().trim().is_empty()
        {
            return Err(CoreError::invalid(
                "a Lean target declares `def wishpool_target_prop : Prop := <statement>` exactly once and names it nowhere else",
            ));
        }
        Ok(())
    }

    fn proposition(&self) -> &str {
        self.text
            .split_once(LEAN_TARGET_MARKER)
            .map(|(_, proposition)| proposition)
            .unwrap_or_default()
    }

    /// `Mathlib` only when everything between the import and the target is
    /// provably inert: blank lines, comments, and commands that declare no
    /// constant. Anything else counts as the curator's own definitions.
    pub fn vocabulary(&self) -> TargetVocabulary {
        let preamble = self
            .text
            .split_once(LEAN_TARGET_MARKER)
            .map(|(preamble, _)| preamble)
            .unwrap_or_default();
        // Remove block and documentation comments, which may nest.
        let mut code = String::new();
        let (mut depth, mut rest) = (0usize, preamble);
        while !rest.is_empty() {
            if rest.starts_with("/-") {
                depth += 1;
                rest = &rest[2..];
            } else if depth > 0 && rest.starts_with("-/") {
                depth -= 1;
                rest = &rest[2..];
            } else {
                let c = rest.chars().next().expect("non-empty");
                if depth == 0 {
                    code.push(c);
                }
                rest = &rest[c.len_utf8()..];
            }
        }
        let inert = code.lines().all(|line| {
            let line = line.split("--").next().unwrap_or_default().trim();
            line.is_empty()
                || [
                    "import",
                    "open",
                    "namespace",
                    "section",
                    "end",
                    "universe",
                    "variable",
                ]
                .contains(&line.split_whitespace().next().unwrap_or_default())
        });
        if depth == 0 && inert {
            TargetVocabulary::Mathlib
        } else {
            TargetVocabulary::OwnDefinitions
        }
    }

    /// A target and a solution the independent verifier can check without a
    /// proof of the statement: for the curator's proposition `p`, the solution
    /// proves `p → p`, which exists exactly when the definitions and `p`
    /// elaborate as a proposition in the pinned environment. The wrapper adds
    /// no line before the proposition ends, so the verifier's diagnostics cite
    /// the curator's own line numbers.
    pub fn elaboration_witness(&self) -> (String, String) {
        let (definitions, proposition) = self
            .text
            .split_once(LEAN_TARGET_MARKER)
            .unwrap_or((&self.text, ""));
        (
            format!("{definitions}{LEAN_TARGET_MARKER} let p : Prop := ({proposition}\n); p → p\n"),
            "import Target\ntheorem wishpool_solution : wishpool_target_prop := fun h => h\n"
                .to_owned(),
        )
    }
}

/// The outcome of the independent verifier reading a curated Lean target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetCheckState {
    /// The exact text is a well-formed proposition in the pinned environment.
    Elaborated,
    /// The verifier did not accept the text; a corrected target is a new version.
    Refused,
}

/// Machine result attached once to an immutable version. It says nothing about
/// whether the proposition states the problem faithfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedTargetCheck {
    pub state: TargetCheckState,
    /// SHA-256 of the exact target text that was read.
    pub target_digest: String,
    /// The verifier's pinned Lean and Mathlib.
    pub toolchain: String,
    /// The verifier's own reason.
    pub reason: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedProblem {
    pub name: String,
    /// LaTeX source text of the curated restatement.
    pub statement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Original posers, ordered and named as the source gives them.
    #[serde(default)]
    pub posers: Vec<SourceContributor>,
    /// Why no poser is named; required exactly when `posers` is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posers_unknown: Option<String>,
    pub source: Source,
    pub status: CuratedStatus,
    pub status_evidence: Vec<StatusEvidence>,
    #[serde(default)]
    pub msc: Vec<String>,
    /// Research-direction tags, as lowercase slugs.
    #[serde(default)]
    pub directions: Vec<String>,
    /// The source's own licence terms, as the source states them.
    pub source_license: String,
    /// Licence of the curated restatement.
    pub license: String,
    /// The curator's Lean reading of the statement, when one is supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_target: Option<CuratedLeanTarget>,
}

impl CuratedProblem {
    /// The reading is as current as its most recent evidence check.
    pub fn status_as_of(&self) -> NaiveDate {
        self.status_evidence
            .iter()
            .map(|e| e.checked_on)
            .max()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectedProblemDelivery {
    pub schema: String,
    pub producer: String,
    pub problem_id: String,
    pub version: u32,
    pub previous_version: Option<u32>,
    pub authorize_publication: bool,
    pub problem: CuratedProblem,
}

fn slug(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn public_page(field: &str, url: &str) -> CoreResult<()> {
    super::manuscript::public_link(url).map_err(|_| {
        CoreError::invalid(format!(
            "{field} requires a public HTTPS URL without credentials"
        ))
    })
}

impl CollectedProblemDelivery {
    pub fn validate(&self) -> CoreResult<()> {
        if self.schema != COLLECTED_PROBLEM_SCHEMA || !self.authorize_publication {
            return Err(CoreError::invalid(
                "require collected problem schema and explicit publication authorization",
            ));
        }
        if !slug(&self.producer, 160) || !slug(&self.problem_id, 160) {
            return Err(CoreError::invalid(
                "producer and problem_id require lowercase ASCII slugs",
            ));
        }
        if self.version == 0
            || self
                .previous_version
                .is_some_and(|v| v == 0 || v >= self.version)
        {
            return Err(CoreError::invalid(
                "version must advance its exact predecessor",
            ));
        }
        let p = &self.problem;
        // The bound of a problem named by a collected work, so the two can agree.
        require_text("problem name", &p.name, 2000)?;
        require_text("problem statement", &p.statement, 20_000)?;
        if let Some(background) = &p.background {
            require_text("problem background", background, 50_000)?;
        }
        p.source.validate()?;
        if p.posers.len() > 50 || p.posers.is_empty() != p.posers_unknown.is_some() {
            return Err(CoreError::invalid(
                "name up to 50 original posers, or state why none is known",
            ));
        }
        for poser in &p.posers {
            require_text("original poser", &poser.name, 300)?;
            if let Some(url) = &poser.profile_url {
                public_page("poser profile", url)?;
            }
        }
        if let Some(reason) = &p.posers_unknown {
            require_text("unknown poser reason", reason, 1000)?;
        }
        if !(1..=10).contains(&p.status_evidence.len()) {
            return Err(CoreError::invalid("status requires 1 to 10 evidence items"));
        }
        for evidence in &p.status_evidence {
            public_page("status evidence", &evidence.url)?;
            require_text("status observation", &evidence.observed, 2000)?;
        }
        validate_msc(&p.msc)?;
        let mut directions = std::collections::BTreeSet::new();
        if p.directions.len() > 16
            || p.directions
                .iter()
                .any(|d| !slug(d, 80) || !directions.insert(d))
        {
            return Err(CoreError::invalid(
                "at most 16 distinct research directions as lowercase ASCII slugs",
            ));
        }
        require_text("source license", &p.source_license, 500)?;
        if !COLLECTED_PROBLEM_LICENSES.contains(&p.license.as_str()) {
            return Err(CoreError::invalid(
                "the curated restatement requires CC-BY-4.0, CC-BY-SA-4.0 or CC0-1.0",
            ));
        }
        if let Some(target) = &p.lean_target {
            target.validate()?;
        }
        Ok(())
    }
}

/// Immutable versions. The unique problem/predecessor and key/predecessor pairs
/// fence concurrent forks and keep one slug per library entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedProblemVersion {
    pub id: String,
    pub owner: PersonId,
    /// The lasting problem-library identity, shared with collected works naming
    /// the same question.
    pub key: String,
    pub delivery: CollectedProblemDelivery,
    pub request_digest: String,
    pub published_on: DateTime<Utc>,
    /// Set once by the target check job; absent while the check is pending and
    /// for versions without a Lean target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_check: Option<CuratedTargetCheck>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> serde_json::Value {
        serde_json::json!({
            "schema": "mathesis.collected-problem.v1",
            "producer": "outreach",
            "problem_id": "gap-question",
            "version": 1,
            "previous_version": null,
            "authorize_publication": true,
            "problem": {
                "name": "The gap question",
                "statement": "Is every gap at most $2$?",
                "posers": [{"name": "A. Poser", "profile_url": "https://example.org/~poser"}],
                "source": {"kind": "url", "locator": "https://mathoverflow.net/questions/000000"},
                "status": "open",
                "status_evidence": [{"url": "https://mathoverflow.net/questions/000000",
                    "checked_on": "2026-11-10", "observed": "No accepted answer."}],
                "source_license": "CC BY-SA 4.0",
                "license": "CC-BY-SA-4.0"
            }
        })
    }

    fn parse(value: serde_json::Value) -> Result<CollectedProblemDelivery, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn rejected(edit: impl FnOnce(&mut serde_json::Value)) -> bool {
        let mut value = envelope();
        edit(&mut value);
        parse(value).is_ok_and(|d| d.validate().is_err())
    }

    #[test]
    fn the_minimal_envelope_is_valid_and_optional_fields_default() {
        let delivery = parse(envelope()).unwrap();
        delivery.validate().unwrap();
        let p = &delivery.problem;
        assert!(p.background.is_none() && p.posers_unknown.is_none());
        assert!(p.msc.is_empty() && p.directions.is_empty());
        assert_eq!(p.status, CuratedStatus::Open);
        assert_eq!(
            p.status_as_of(),
            chrono::NaiveDate::from_ymd_opt(2026, 11, 10).unwrap()
        );
    }

    #[test]
    fn intake_admits_only_open_or_partially_resolved_and_no_unknown_fields() {
        for status in ["settled", "solved", "proved", "related_research", ""] {
            let mut value = envelope();
            value["problem"]["status"] = status.into();
            assert!(parse(value).is_err(), "{status}");
        }
        let mut value = envelope();
        value["problem"]["status"] = "partially_resolved".into();
        parse(value).unwrap().validate().unwrap();
        for pointer in [
            "",
            "/problem",
            "/problem/status_evidence/0",
            "/problem/posers/0",
        ] {
            let mut value = envelope();
            value.pointer_mut(pointer).unwrap()["audit"] = "passed".into();
            assert!(parse(value).is_err(), "{pointer}");
        }
    }

    #[test]
    fn envelope_identity_and_version_rules_follow_the_collection_channel() {
        assert!(rejected(
            |v| v["schema"] = "mathesis.pre-reviewed-delivery.v1".into()
        ));
        assert!(rejected(|v| v["authorize_publication"] = false.into()));
        assert!(rejected(|v| v["producer"] = "Outreach".into()));
        assert!(rejected(|v| v["problem_id"] = "gap question".into()));
        assert!(rejected(|v| v["problem_id"] = "g".repeat(161).into()));
        assert!(rejected(|v| v["version"] = 0.into()));
        assert!(rejected(|v| v["previous_version"] = 1.into()));
        assert!(rejected(|v| {
            v["version"] = 2.into();
            v["previous_version"] = 0.into();
        }));
        assert!(!rejected(|v| {
            v["version"] = 3.into();
            v["previous_version"] = 2.into();
        }));
    }

    #[test]
    fn mathematical_text_is_bounded_like_a_typed_conjecture() {
        assert!(rejected(|v| v["problem"]["name"] = " ".into()));
        assert!(rejected(|v| v["problem"]["name"] = "n".repeat(2_001).into()));
        assert!(rejected(|v| v["problem"]["statement"] = "".into()));
        assert!(!rejected(
            |v| v["problem"]["statement"] = "x".repeat(20_000).into()
        ));
        assert!(rejected(
            |v| v["problem"]["statement"] = "x".repeat(20_001).into()
        ));
        assert!(!rejected(
            |v| v["problem"]["background"] = "x".repeat(50_000).into()
        ));
        assert!(rejected(
            |v| v["problem"]["background"] = "x".repeat(50_001).into()
        ));
        assert!(rejected(|v| v["problem"]["background"] = "  ".into()));
        assert!(rejected(
            |v| v["problem"]["source"] = serde_json::json!({"kind": "oeis", "locator": "391810"})
        ));
        assert!(!rejected(
            |v| v["problem"]["source"] =
                serde_json::json!({"kind": "arxiv", "locator": "2401.12345", "year": 2024})
        ));
        assert!(!rejected(
            |v| v["problem"]["source"] =
                serde_json::json!({"kind": "doi", "locator": "10.1000/example"})
        ));
    }

    #[test]
    fn posers_are_named_or_explicitly_unknown() {
        assert!(rejected(|v| v["problem"]["posers"] = serde_json::json!([])));
        assert!(!rejected(|v| {
            v["problem"]["posers"] = serde_json::json!([]);
            v["problem"]["posers_unknown"] = "The source page names no author.".into();
        }));
        assert!(rejected(
            |v| v["problem"]["posers_unknown"] = "Also unknown.".into()
        ));
        assert!(rejected(|v| {
            v["problem"]["posers"] = serde_json::json!([]);
            v["problem"]["posers_unknown"] = " ".into();
        }));
        assert!(rejected(|v| v["problem"]["posers"][0]["name"] = "".into()));
        assert!(rejected(
            |v| v["problem"]["posers"][0]["profile_url"] = "http://example.org/~poser".into()
        ));
        assert!(rejected(|v| v["problem"]["posers"][0]["profile_url"] =
            "https://user:secret@example.org/".into()));
        assert!(rejected(|v| v["problem"]["posers"] =
            serde_json::json!(vec![
                serde_json::json!({"name": "A. Poser"});
                51
            ])));
    }

    #[test]
    fn status_evidence_records_one_to_ten_dated_observations_of_public_pages() {
        assert!(rejected(
            |v| v["problem"]["status_evidence"] = serde_json::json!([])
        ));
        let item = envelope()["problem"]["status_evidence"][0].clone();
        assert!(!rejected(
            |v| v["problem"]["status_evidence"] = serde_json::json!(vec![item.clone(); 10])
        ));
        assert!(rejected(
            |v| v["problem"]["status_evidence"] = serde_json::json!(vec![item.clone(); 11])
        ));
        assert!(rejected(|v| v["problem"]["status_evidence"][0]["url"] =
            "http://mathoverflow.net/questions/000000".into()));
        assert!(rejected(
            |v| v["problem"]["status_evidence"][0]["url"] = "javascript:alert(1)".into()
        ));
        assert!(rejected(
            |v| v["problem"]["status_evidence"][0]["observed"] = " ".into()
        ));
        assert!(rejected(
            |v| v["problem"]["status_evidence"][0]["observed"] = "x".repeat(2_001).into()
        ));
        let mut undated = envelope();
        undated["problem"]["status_evidence"][0]["checked_on"] = "last week".into();
        assert!(parse(undated).is_err());
        // The reading is as current as its most recent check.
        let mut value = envelope();
        value["problem"]["status_evidence"] = serde_json::json!([
            {"url": "https://oeis.org/A000001", "checked_on": "2026-11-09", "observed": "Open."},
            {"url": "https://oeis.org/A000002", "checked_on": "2026-10-01", "observed": "Open."}]);
        assert_eq!(
            parse(value).unwrap().problem.status_as_of().to_string(),
            "2026-11-09"
        );
    }

    #[test]
    fn classification_tags_and_licences_are_bounded() {
        assert!(!rejected(
            |v| v["problem"]["msc"] = serde_json::json!(["11B39", "05C"])
        ));
        assert!(rejected(
            |v| v["problem"]["msc"] = serde_json::json!(["number theory"])
        ));
        assert!(!rejected(
            |v| v["problem"]["directions"] = serde_json::json!(["zero-runs", "golden-mean-shift"])
        ));
        assert!(rejected(
            |v| v["problem"]["directions"] = serde_json::json!(["Zero Runs"])
        ));
        assert!(rejected(
            |v| v["problem"]["directions"] = serde_json::json!(["zero-runs", "zero-runs"])
        ));
        assert!(rejected(
            |v| v["problem"]["directions"] = serde_json::json!(["d".repeat(81)])
        ));
        assert!(rejected(
            |v| v["problem"]["directions"] =
                serde_json::json!((0..17).map(|i| format!("tag-{i}")).collect::<Vec<_>>())
        ));
        assert!(rejected(|v| v["problem"]["source_license"] = "".into()));
        assert!(rejected(
            |v| v["problem"]["source_license"] = "x".repeat(501).into()
        ));
        for license in ["CC-BY-4.0", "CC-BY-SA-4.0", "CC0-1.0"] {
            assert!(!rejected(|v| v["problem"]["license"] = license.into()));
        }
        for license in ["CC-BY-NC-4.0", "All rights reserved", ""] {
            assert!(rejected(|v| v["problem"]["license"] = license.into()));
        }
    }

    fn target(text: &str) -> CuratedLeanTarget {
        CuratedLeanTarget {
            text: text.into(),
            correspondence: "The proposition reads the statement literally.".into(),
        }
    }

    #[test]
    fn a_lean_target_is_one_mathlib_only_proposition_named_once() {
        let good = "import Mathlib\n\ndef wishpool_target_prop : Prop :=\n  ∀ n : ℕ, n = n\n";
        assert!(target(good).validate().is_ok());
        for bad in [
            // Mathlib first, and nothing else imported.
            "def wishpool_target_prop : Prop := True\n",
            "import Mathlib.Data.Nat.Basic\ndef wishpool_target_prop : Prop := True\n",
            "import Mathlib\nimport D5.S1\ndef wishpool_target_prop : Prop := True\n",
            "import Mathlib\n  import Aesop\ndef wishpool_target_prop : Prop := True\n",
            // Exactly one declaration of the proposition, named nowhere else.
            "import Mathlib\ntheorem wishpool_target : True := trivial\n",
            "import Mathlib\ndef wishpool_target_prop : Prop := True\ndef wishpool_target_prop : Prop := True\n",
            "import Mathlib\n-- wishpool_target_prop\ndef wishpool_target_prop : Prop := True\n",
            "import Mathlib\ndef wishpool_target_prop : Prop :=\n   \n",
        ] {
            assert!(target(bad).validate().is_err(), "{bad}");
        }
        let mut wordless = target(good);
        wordless.correspondence = " ".into();
        assert!(wordless.validate().is_err());
        let mut long = target(good);
        long.correspondence = "c".repeat(5_001);
        assert!(long.validate().is_err());
        assert!(
            target(&format!("{good}{}", "-- pad\n".repeat(30_000)))
                .validate()
                .is_err()
        );
        // The envelope carries it as an optional field and refuses unknown ones.
        assert!(!rejected(|v| {
            v["problem"]["lean_target"] =
                serde_json::json!({"text": good, "correspondence": "Literal reading."})
        }));
        for unparsed in [
            serde_json::json!({"text": good}),
            serde_json::json!({
                "text": good, "correspondence": "Literal reading.", "proof": "by simp"
            }),
        ] {
            let mut value = envelope();
            value["problem"]["lean_target"] = unparsed;
            assert!(parse(value).is_err());
        }
    }

    #[test]
    fn vocabulary_is_mathlib_only_when_the_preamble_declares_nothing() {
        let statement = "def wishpool_target_prop : Prop := ∀ n : ℕ, n = n\n";
        for inert in [
            "import Mathlib\n",
            "import Mathlib\n\nopen Nat Finset\n\n-- the statement\n",
            "import Mathlib\n/- a block\n/- nested -/ def hidden := 1 -/\nopen scoped BigOperators\n",
            "import Mathlib\nuniverse u\nvariable (n : ℕ) -- def x := 1\n/-- The curated statement. -/\n",
        ] {
            assert_eq!(
                target(&format!("{inert}{statement}")).vocabulary(),
                TargetVocabulary::Mathlib,
                "{inert}"
            );
        }
        for defining in [
            "import Mathlib\ndef gap (n : ℕ) : ℕ := n + 1\n",
            "import Mathlib\nnoncomputable def c : ℝ := 2\n",
            "import Mathlib\nabbrev S := Finset ℕ\n",
            "import Mathlib\nlocal notation \"φ\" => (1 : ℝ)\n",
            "import Mathlib\nset_option maxRecDepth 4000\n",
            // An unclosed comment hides nothing from the reading.
            "import Mathlib\n/- unclosed\n",
        ] {
            assert_eq!(
                target(&format!("{defining}{statement}")).vocabulary(),
                TargetVocabulary::OwnDefinitions,
                "{defining}"
            );
        }
    }

    #[test]
    fn the_elaboration_witness_keeps_the_definitions_and_wraps_only_the_proposition() {
        let text = "import Mathlib\ndef gap (n : ℕ) : ℕ := n + 1\ndef wishpool_target_prop : Prop :=\n  ∀ n, gap n > n\n";
        let (witness, solution) = target(text).elaboration_witness();
        assert_eq!(
            witness,
            "import Mathlib\ndef gap (n : ℕ) : ℕ := n + 1\ndef wishpool_target_prop : Prop := let p : Prop := (\n  ∀ n, gap n > n\n\n); p → p\n"
        );
        // Every line of the curator's text keeps its number in the witness.
        for (number, line) in text.lines().enumerate().skip(3) {
            assert_eq!(witness.lines().nth(number), Some(line));
        }
        assert_eq!(witness.matches(LEAN_TARGET_MARKER).count(), 1);
        assert_eq!(
            solution,
            "import Target\ntheorem wishpool_solution : wishpool_target_prop := fun h => h\n"
        );
    }
}
