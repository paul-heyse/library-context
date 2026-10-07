//! Private evaluation meanings. None of these contracts enter production requests.
use std::collections::BTreeMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Assignment = BTreeMap<String, String>;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intent { Positive, ExpectedFailure, Unknown, NotApplicable }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Completeness { Complete, Incomplete, Unsupported }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OracleBasis {
    pub kind: String,
    pub input_digest: String,
    pub qualification: String,
    pub supported_domain: String,
    pub revision: String,
    pub completeness: Completeness,
    pub unknown_limits: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicRequest {
    pub question: String,
    pub context: Assignment,
    pub allowed_followups: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Envelope { pub max_calls: usize, pub max_bytes: usize }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Witness {
    Leaf { predicate: String },
    All { children: Vec<Witness> },
    Any { children: Vec<Witness> },
    Exists { variables: Vec<String>, child: Box<Witness> },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus { Supported, ExpectedFailure, Unknown, Authored, Skipped }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualificationRequirement { pub id: String, pub accepted_text: Vec<String> }
/// Acceptable text is independently authored, never recovered from the renderer.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    pub name: String,
    pub role: String,
    pub accepted_text: Vec<String>,
    pub anchors: Vec<String>,
    pub context: Assignment,
    pub qualifications: Vec<QualificationRequirement>,
    pub candidate_status: CandidateStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub name: String,
    pub context: Assignment,
    pub facts: Assignment,
    pub answer: String,
}
/// An observed qualified alternative constrains a fact to any matching value.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Information {
    pub fact: String,
    pub value: String,
    pub predicate: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FiniteModel { pub worlds: Vec<World>, pub information: Vec<Information> }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvaluationTask {
    pub id: String,
    pub split: Split,
    pub family: String,
    pub request: PublicRequest,
    pub envelope: Envelope,
    pub intent: Intent,
    pub oracle: OracleBasis,
    pub predicates: Vec<Predicate>,
    pub witness: Option<Witness>,
    pub model: Option<FiniteModel>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Split { Development, Validation }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus { Completed, Failed, Refused, Stale, BudgetExhausted, Cancelled }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObserverFormat { FinitePacketV1 }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expansion {
    pub reference: String,
    pub operation: String,
    pub realization: String,
    pub status: OperationStatus,
    pub response_segment: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub realization: String,
    /// Exact UTF-8 emitted bytes, initial response at index zero.
    pub segments: Vec<String>,
    pub observer_format: ObserverFormat,
    pub expansions: Vec<Expansion>,
    pub status: OperationStatus,
    pub failure: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Immediate, Expandable }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Applicability { Applicable, NotApplicable, Unsupported, Incomplete, InvalidTask }
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Epistemic {
    Sufficient, Insufficient, ModelRelativeUnknown, InconsistentModel,
    InventoryInfeasible, BudgetInfeasible, Inconclusive, NotApplicable,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    pub task_id: String,
    pub observation_digest: String,
    pub applicability: Applicability,
    pub epistemic: Epistemic,
    pub execution: OperationStatus,
    pub reason: String,
    pub assignments: Vec<Assignment>,
    pub countermodels: Vec<String>,
    pub scorable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Case { pub task: EvaluationTask, pub observation: Observation, pub mode: Mode }
