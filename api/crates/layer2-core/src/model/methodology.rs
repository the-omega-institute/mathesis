use serde::{Deserialize, Serialize};

use super::{OpenProblemRef, ProofShape, require_text};
use crate::{CoreError, CoreResult};

pub const REVIEW_CONTRACT: u32 = 3;
/// Adds manuscript-origin context without changing contract 3 prompt bytes.
pub const MANUSCRIPT_PROVENANCE_CONTRACT: u32 = 4;
pub const REVIEW_METHOD: &str = "trureturing-content-2026-10-15";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Supported,
    Unsupported,
    Insufficient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrerequisiteReading {
    pub statement: String,
    pub location: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessKind {
    Intermediate,
    Conclusion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentWitness {
    pub kind: WitnessKind,
    pub statement: String,
    pub location: String,
    pub non_binding_step: String,
    pub live_use: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UtilityKind {
    GeneralTheorem,
    FiniteEnumeration,
    Checker,
    NumericReduction,
    CertifiedInstance,
    Counterexample,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtilityReading {
    pub kind: UtilityKind,
    pub status: EvidenceStatus,
    pub rationale: String,
    pub problem: String,
    pub progress: String,
    pub prerequisites: String,
    pub discharge: String,
}

impl UtilityReading {
    pub fn supports_admission(&self) -> bool {
        self.status == EvidenceStatus::Supported
            && !self.rationale.trim().is_empty()
            && match self.kind {
                UtilityKind::GeneralTheorem => true,
                UtilityKind::FiniteEnumeration | UtilityKind::CertifiedInstance => false,
                UtilityKind::Checker
                | UtilityKind::NumericReduction
                | UtilityKind::Counterexample => [
                    &self.problem,
                    &self.progress,
                    &self.prerequisites,
                    &self.discharge,
                ]
                .iter()
                .all(|text| !text.trim().is_empty()),
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementReading {
    pub status: EvidenceStatus,
    pub problem: OpenProblemRef,
    pub statement: String,
    pub assumptions: String,
    pub prior_status: String,
    pub resolution: String,
    pub matching_argument: String,
    pub opened_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformationGainReading {
    pub model: String,
    pub readout: String,
    pub task: String,
    pub correspondence: String,
    pub evidence: String,
    pub status: EvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodologyReading {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<super::KnowledgeAdmission>,
    pub baseline: Vec<PrerequisiteReading>,
    pub searched: Vec<String>,
    pub literature: EvidenceStatus,
    pub binding_analysis: String,
    pub delta: String,
    pub escape: ProofShape,
    pub witnesses: Vec<ContentWitness>,
    pub utility: UtilityReading,
    pub settlement: Option<SettlementReading>,
    pub information_gain: Option<InformationGainReading>,
    pub limits: Vec<String>,
}

impl MethodologyReading {
    pub fn insufficient(reason: &str) -> Self {
        Self {
            knowledge: None,
            baseline: vec![],
            searched: vec![],
            literature: EvidenceStatus::Insufficient,
            binding_analysis: reason.into(),
            delta: String::new(),
            escape: ProofShape::Undetermined,
            witnesses: vec![],
            utility: UtilityReading {
                kind: UtilityKind::GeneralTheorem,
                status: EvidenceStatus::Insufficient,
                rationale: reason.into(),
                problem: String::new(),
                progress: String::new(),
                prerequisites: String::new(),
                discharge: String::new(),
            },
            settlement: None,
            information_gain: None,
            limits: vec![reason.into()],
        }
    }

    pub fn validate(&self, conjecture: bool) -> CoreResult<()> {
        if serde_json::to_vec(self).map_or(true, |value| value.len() > 80_000)
            || self.baseline.len() > 40
            || self.witnesses.len() > 20
            || self.searched.len() > 40
            || self.limits.len() > 40
        {
            return Err(CoreError::invalid(
                "methodology evidence exceeds its bounds",
            ));
        }
        require_text("binding analysis", &self.binding_analysis, 10_000)?;
        require_text("utility rationale", &self.utility.rationale, 10_000)?;
        for prerequisite in &self.baseline {
            require_text("prerequisite statement", &prerequisite.statement, 10_000)?;
            require_text("prerequisite location", &prerequisite.location, 2_000)?;
        }
        for scope in &self.searched {
            require_text("search scope", scope, 2_000)?;
        }
        if self.literature == EvidenceStatus::Supported
            && (self.searched.is_empty() || self.baseline.is_empty())
        {
            return Err(CoreError::invalid(
                "supported literature needs a search scope and baseline",
            ));
        }
        if self.escape != ProofShape::Undetermined && self.baseline.is_empty() {
            return Err(CoreError::invalid(
                "an escape reading needs explicit prerequisites",
            ));
        }
        if self.escape == ProofShape::Content {
            require_text("mathematical delta", &self.delta, 10_000)?;
            if !conjecture && self.witnesses.is_empty() {
                return Err(CoreError::invalid(
                    "content needs a live intermediate or conclusion witness",
                ));
            }
        }
        if (conjecture || self.escape != ProofShape::Content) && !self.witnesses.is_empty() {
            return Err(CoreError::invalid(
                "only proved content can carry live proof witnesses",
            ));
        }
        for witness in &self.witnesses {
            require_text("witness statement", &witness.statement, 10_000)?;
            require_text("witness location", &witness.location, 2_000)?;
            require_text("non-binding step", &witness.non_binding_step, 10_000)?;
            require_text("live proof use", &witness.live_use, 10_000)?;
        }
        if let Some(settlement) = &self.settlement {
            settlement.problem.source.validate()?;
            require_text("problem name", &settlement.problem.name, 300)?;
            if settlement.status == EvidenceStatus::Supported {
                for text in [
                    &settlement.statement,
                    &settlement.assumptions,
                    &settlement.prior_status,
                    &settlement.resolution,
                    &settlement.matching_argument,
                    &settlement.opened_source,
                ] {
                    require_text("settlement evidence", text, 10_000)?;
                }
            }
        }
        if let Some(gain) = &self.information_gain {
            for text in [
                &gain.model,
                &gain.readout,
                &gain.task,
                &gain.correspondence,
                &gain.evidence,
            ] {
                require_text("information gain evidence", text, 10_000)?;
            }
        }
        Ok(())
    }

    pub fn supports_content(&self, conjecture: bool) -> bool {
        self.validate(conjecture).is_ok()
            && self.literature == EvidenceStatus::Supported
            && self.escape == ProofShape::Content
            && self.utility.supports_admission()
            && self
                .knowledge
                .as_ref()
                .is_none_or(|k| k.permits_new_content())
    }

    pub fn supports_settlement(&self, problem: &OpenProblemRef) -> bool {
        self.validate(false).is_ok()
            && self.literature == EvidenceStatus::Supported
            && self.knowledge.as_ref().is_none_or(|k| {
                k.sufficient() && k.audited_relation != Some(super::CoverageRelation::Refuted)
            })
            && self.settlement.as_ref().is_some_and(|reading| {
                reading.status == EvidenceStatus::Supported && &reading.problem == problem
            })
    }
}
