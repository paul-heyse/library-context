//! Captured deployment descriptions and task reports. Reported execution/environment fields are
//! evidence values, never a certification that this pipeline executed or reproduced the task.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    assertion::{AssertionQualification, EvidenceSourceSpanId},
    attribution::FactFamily,
    input::Release,
    source::SourceArtifact,
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CheckStatus {
    Passed = 0,
    Failed = 1,
    NotRun = 2,
    Blocked = 3,
}
/// Full u64 receipt duration. Decimal text is a lossless physical lowering, not domain identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Milliseconds(pub u64);
impl Key for Milliseconds {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"milliseconds-u64", &self.0.to_le_bytes());
    }
}
impl FlatValue for Milliseconds {}
impl HeapSize for Milliseconds {}
impl FieldValue for Milliseconds {
    const SCALAR: Scalar = Scalar::Text;
}
impl serde::Serialize for Milliseconds {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> serde::Deserialize<'de> for Milliseconds {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = <String as serde::Deserialize>::deserialize(d)?;
        let value: u64 = text.parse().map_err(serde::de::Error::custom)?;
        if value.to_string() != text {
            return Err(serde::de::Error::custom(
                "noncanonical unsigned milliseconds",
            ));
        }
        Ok(Self(value))
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ReportCollectionKind {
    EnvironmentMetadata = 0,
    Invocation = 1,
}
/// Atomic collection values. Ordinals preserve duplicate command arguments and tool entries;
/// named maps retain exact keys instead of an opaque JSON relationship payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "report_values", validate = validate_value)]
pub enum ReportValue {
    #[model(code = 0)]
    Command { ordinal: i64, text: String },
    #[model(code = 1)]
    Argument { name: String, value: i64 },
    #[model(code = 2)]
    Tool { ordinal: i64, name: String },
    #[model(code = 3)]
    Metadata { name: String, digest: ContentHash },
}
fn validate_value(value: &ReportValue) -> Result<(), ModelError> {
    match value {
        ReportValue::Command { ordinal, .. } | ReportValue::Tool { ordinal, .. }
            if *ordinal < 0 =>
        {
            Err(invalid("negative report entry ordinal"))
        }
        ReportValue::Argument { name, .. } | ReportValue::Metadata { name, .. }
            if name.is_empty() =>
        {
            Err(invalid("empty report map key"))
        }
        _ => Ok(()),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "report_collections", invariant_refs = collection_invariants_refs)]
pub struct ReportCollection {
    #[model(key)]
    pub kind: ReportCollectionKind,
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "report_entries", validate = validate_entry)]
pub struct ReportEntry {
    #[model(key)]
    pub collection: Id<ReportCollection>,
    #[model(key)]
    pub ordinal: i64,
    pub value: Id<ReportValue>,
}
fn validate_entry(row: &ReportEntry) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative report membership ordinal"));
    }
    Ok(())
}
fn value_order(value: &ReportValue) -> (i16, i64, &str) {
    match value {
        ReportValue::Command { ordinal, .. } => (0, *ordinal, ""),
        ReportValue::Argument { name, .. } => (1, 0, name),
        ReportValue::Tool { ordinal, .. } => (2, *ordinal, ""),
        ReportValue::Metadata { name, .. } => (3, 0, name),
    }
}
fn member_digest(
    kind: ReportCollectionKind,
    ids: impl IntoIterator<Item = Id<ReportValue>>,
) -> ContentHash {
    let mut sink = KeySink::new("report-collection");
    kind.encode(&mut sink);
    let mut count = 0i64;
    for id in ids {
        id.encode(&mut sink);
        count += 1;
    }
    count.encode(&mut sink);
    sink.finish()
}
fn check_values<'a>(
    kind: ReportCollectionKind,
    values: impl IntoIterator<Item = &'a ReportValue>,
) -> Result<(), ModelError> {
    let mut previous = None;
    let mut commands = 0;
    let mut tools = 0;
    for value in values {
        value.validate()?;
        let key = value_order(value);
        if previous.is_some_and(|p| p >= key) {
            return Err(invalid(
                "report entries duplicate or not in canonical order",
            ));
        }
        previous = Some(key);
        match (kind, value) {
            (ReportCollectionKind::Invocation, ReportValue::Command { ordinal, .. })
                if *ordinal == commands =>
            {
                commands += 1
            }
            (ReportCollectionKind::Invocation, ReportValue::Tool { ordinal, .. })
                if *ordinal == tools =>
            {
                tools += 1
            }
            (ReportCollectionKind::Invocation, ReportValue::Argument { .. })
            | (ReportCollectionKind::EnvironmentMetadata, ReportValue::Metadata { .. }) => {}
            _ => return Err(invalid("report collection kind or sequence gap")),
        }
    }
    Ok(())
}
impl ReportCollection {
    pub fn new(
        kind: ReportCollectionKind,
        mut values: Vec<ReportValue>,
    ) -> Result<(Self, Vec<ReportValue>, Vec<ReportEntry>), ModelError> {
        if values.len() > 4096 {
            return Err(invalid("report collection work limit"));
        }
        values.sort_by(|a, b| value_order(a).cmp(&value_order(b)));
        check_values(kind, &values)?;
        let row = Self {
            kind,
            members: member_digest(kind, values.iter().map(Record::id)),
        };
        let entries = values
            .iter()
            .enumerate()
            .map(|(ordinal, value)| ReportEntry {
                collection: row.id(),
                ordinal: ordinal as i64,
                value: value.id(),
            })
            .collect();
        Ok((row, values, entries))
    }
}
/// A value reported by captured evidence; links a named release, not the active interpreter.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "reported_environments", invariant_refs = report_shape_invariants_refs)]
pub struct ReportedEnvironment {
    #[model(key)]
    pub release: Id<Release>,
    #[model(key)]
    pub lock_digest: ContentHash,
    #[model(key)]
    pub environment_digest: ContentHash,
    #[model(key)]
    pub runtime_digest: ContentHash,
    #[model(key)]
    pub interpreter_digest: ContentHash,
    #[model(key)]
    pub python_version: String,
    #[model(key)]
    pub platform: String,
    #[model(key)]
    pub requirement: String,
    #[model(key)]
    pub metadata: Id<ReportCollection>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "task_reports", validate = validate_report)]
pub struct TaskReport {
    #[model(key)]
    pub format: i64,
    #[model(key)]
    pub policy: String,
    #[model(key)]
    pub task: String,
    #[model(key)]
    pub runner_sha256: String,
    /// Reported coordinates and SHA256, not an assertion of captured artifact equality.
    #[model(key)]
    pub source_path: String,
    #[model(key)]
    pub source_sha256: String,
    #[model(key)]
    pub environment: Id<ReportedEnvironment>,
    #[model(key)]
    pub invocation: Id<ReportCollection>,
    #[model(key)]
    pub tool: String,
    #[model(key)]
    pub elapsed_ms: Milliseconds,
    #[model(key)]
    pub timeout_seconds: i64,
    #[model(key)]
    pub execution: CheckStatus,
    #[model(key)]
    pub result: Option<String>,
    #[model(key)]
    pub diagnostic: Option<String>,
}
fn validate_report(row: &TaskReport) -> Result<(), ModelError> {
    if !(0..=i64::from(u32::MAX)).contains(&row.format)
        || !(0..=i64::from(u32::MAX)).contains(&row.timeout_seconds)
    {
        return Err(invalid("receipt format/timeout out of u32 range"));
    }
    for digest in [&row.runner_sha256, &row.source_sha256] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("receipt SHA256 is not canonical"));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "task_report_observations")]
#[assertion(support = TaskReportSupport, name = "task_report_supports", family = FactFamily::Deployment, subjects(receipt, target))]
pub struct TaskReportObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub receipt: EvidenceSourceSpanId,
    /// Independently attributed association. Reported path/hash alone never establishes this link.
    #[model(key)]
    pub target: Id<SourceArtifact>,
    #[model(key)]
    pub report: Id<TaskReport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "deployment_observations", validate = validate_deployment)]
#[assertion(support = DeploymentSupport, name = "deployment_supports", family = FactFamily::Deployment, subjects(span))]
pub struct DeploymentObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub span: EvidenceSourceSpanId,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub distribution: Option<String>,
    #[model(key)]
    pub version: Option<String>,
    #[model(key)]
    pub field: String,
    #[model(key)]
    pub original: String,
    #[model(key)]
    pub name: Option<String>,
    #[model(key)]
    pub extras: Vec<String>,
    #[model(key)]
    pub marker: Option<String>,
    #[model(key)]
    pub constraint: Option<String>,
    #[model(key)]
    pub interpretation: CheckStatus,
    #[model(key)]
    pub diagnostic: Option<String>,
    #[model(key)]
    pub environment_digest: Option<ContentHash>,
    #[model(key)]
    pub lock_digest: Option<ContentHash>,
    #[model(key)]
    pub referenced_path: Option<String>,
}
fn validate_deployment(row: &DeploymentObservation) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative deployment ordinal"));
    }
    Ok(())
}

pub(crate) fn collection_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "report_collection_membership",
        inputs: vec![
            ValidationInput::of::<ReportCollection>(&["id"]),
            ValidationInput::of::<ReportValue>(&["id"]),
            ValidationInput::of::<ReportEntry>(&["collection", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CollectionCheck {
                charge: StateCharge::new(budget, "report_collection_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct CollectionCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<ReportCollection>, ReportCollection>,
    values: ChargedMap<Id<ReportValue>, ReportValue>,
    current: Option<(Id<ReportCollection>, Vec<Id<ReportValue>>)>,
}
impl CollectionCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, ids)) = self.current.take() {
            let row = self
                .expected
                .remove(&mut self.charge, &id)
                .ok_or_else(|| invalid("report collection absent or repeated"))?;
            if row.members != member_digest(row.kind, ids.iter().copied()) {
                return Err(invalid("report collection membership differs"));
            }
            let values = ids
                .iter()
                .map(|id| {
                    self.values
                        .get(id)
                        .ok_or_else(|| invalid("report value absent"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            check_values(row.kind, values)?;
        }
        Ok(())
    }
}
impl InvariantCheck for CollectionCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ReportCollection::NAME {
            for row in ReportCollection::decode(batch)? {
                self.expected.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ReportValue::NAME {
            for row in ReportValue::decode(batch)? {
                self.values.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ReportEntry::NAME {
            for row in ReportEntry::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _)| *id != row.collection)
                {
                    self.flush()?;
                    self.current = Some((row.collection, Vec::new()));
                }
                let (_, values) = self.current.as_mut().expect("current collection");
                if row.ordinal != values.len() as i64 || values.len() >= 4096 {
                    return Err(invalid("report membership gap, duplicate or work limit"));
                }
                values.push(row.value);
            }
        } else {
            return Err(invalid("undeclared report membership input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .expected
            .values()
            .any(|row| row.members != member_digest(row.kind, []))
        {
            return Err(invalid("report collection has missing entries"));
        }
        Ok(())
    }
}
pub(crate) fn report_shape_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "report_collection_roles",
        inputs: vec![
            ValidationInput::of::<ReportCollection>(&["id"]),
            ValidationInput::of::<ReportedEnvironment>(&["id"]),
            ValidationInput::of::<TaskReport>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ReportShape {
                charge: StateCharge::new(budget, "report_collection_roles"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct ReportShape {
    charge: StateCharge,
    metadata: ChargedSet<Id<ReportCollection>>,
    invocation: ChargedSet<Id<ReportCollection>>,
}
impl InvariantCheck for ReportShape {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ReportCollection::NAME {
            for row in ReportCollection::decode(batch)? {
                match row.kind {
                    ReportCollectionKind::EnvironmentMetadata => {
                        self.metadata.insert(&mut self.charge, row.id())?;
                    }
                    ReportCollectionKind::Invocation => {
                        self.invocation.insert(&mut self.charge, row.id())?;
                    }
                }
            }
        } else if relation == ReportedEnvironment::NAME {
            for row in ReportedEnvironment::decode(batch)? {
                if !self.metadata.contains(&row.metadata) {
                    return Err(invalid("environment needs metadata collection"));
                }
            }
        } else if relation == TaskReport::NAME {
            for row in TaskReport::decode(batch)? {
                if !self.invocation.contains(&row.invocation) {
                    return Err(invalid("task needs invocation collection"));
                }
            }
        } else {
            return Err(invalid("undeclared report shape input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

pub(crate) fn collection_invariants_refs() -> Vec<&'static str> {
    vec!["report_collection_membership"]
}
pub(crate) fn report_shape_invariants_refs() -> Vec<&'static str> {
    vec!["report_collection_roles"]
}
