//! Original evidence and contextual observations (ADR-0076).
use crate::{Digest, Id, IdHasher, table::table};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    NotRun,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextStatus {
    Complete,
    ContextDependent,
    Truncated,
    Refused,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Demonstration,
    AssertionTest,
    ExpectedFailure,
    SkipXfail,
    Mixed,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Exact,
    MappedWithEvidence,
    Other,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Span,
    Scenario,
    Deployment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EvidenceRef {
    Span(crate::wire::SpanId),
    Scenario(crate::wire::ScenarioId),
    Deployment(crate::wire::DeploymentId),
}
impl EvidenceRef {
    pub fn new(kind: EvidenceKind, id: Id) -> Self {
        match kind {
            EvidenceKind::Span => Self::Span(crate::wire::SpanId::from_storage(id)),
            EvidenceKind::Scenario => Self::Scenario(crate::wire::ScenarioId::from_storage(id)),
            EvidenceKind::Deployment => {
                Self::Deployment(crate::wire::DeploymentId::from_storage(id))
            }
        }
    }
    pub fn kind(&self) -> EvidenceKind {
        match self {
            Self::Span(_) => EvidenceKind::Span,
            Self::Scenario(_) => EvidenceKind::Scenario,
            Self::Deployment(_) => EvidenceKind::Deployment,
        }
    }
    pub fn id(&self) -> Id {
        match self {
            Self::Span(id) => id.storage(),
            Self::Scenario(id) => id.storage(),
            Self::Deployment(id) => id.storage(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub parse: CheckStatus,
    pub binding: CheckStatus,
    pub environment: CheckStatus,
    pub execution: CheckStatus,
}
impl Default for Checks {
    fn default() -> Self {
        Self {
            parse: CheckStatus::NotRun,
            binding: CheckStatus::NotRun,
            environment: CheckStatus::NotRun,
            execution: CheckStatus::NotRun,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextRequirement {
    pub kind: String,
    pub expression: String,
    pub evidence: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDetail {
    pub spans: Vec<Id>,
    pub context: ContextStatus,
    pub intent: Intent,
    pub checks: Checks,
    pub requirements: Vec<ContextRequirement>,
    pub extraction: String,
    pub analysis_module: Option<Id>,
    pub option_bindings: Vec<OptionBinding>,
    pub omitted_options: u64,
    pub omitted_requirements: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptionBinding {
    pub site_id: Id,
    pub ordinal: i64,
    pub keyword: Option<String>,
    pub kind: String,
    pub expression: String,
    pub span_id: Option<Id>,
    pub coordinate_space: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentEnvironment {
    pub release_id: Id,
    pub lock_digest: Digest,
    pub environment_digest: Digest,
    pub runtime_digest: Digest,
    pub interpreter_digest: Digest,
    pub python_version: String,
    pub platform: String,
    pub requirement: String,
    pub metadata: std::collections::BTreeMap<String, Digest>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskReceipt {
    pub format: u32,
    pub policy: String,
    pub task: String,
    pub runner_sha256: String,
    pub source_path: String,
    pub source_sha256: String,
    pub environment: DeploymentEnvironment,
    pub command: Vec<String>,
    pub tool: String,
    pub arguments: std::collections::BTreeMap<String, i64>,
    pub elapsed_ms: u64,
    pub timeout_seconds: u32,
    pub execution: CheckStatus,
    pub tools: Vec<String>,
    pub result: Option<String>,
    pub diagnostic: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskObservation {
    pub target_artifact: Id,
    pub receipt: TaskReceipt,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssociationSupport {
    pub edge_id: Id,
    pub fact_id: Id,
    pub support_fact_id: Option<Id>,
    pub target_id: Id,
    pub modality: String,
    pub phase: String,
    pub unresolved_reason: Option<String>,
    pub context_span_id: Id,
    pub analysis_module: Id,
    pub start_byte: i64,
    pub end_byte: i64,
    pub coordinate_space: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentDetail {
    pub distribution: Option<String>,
    pub version: Option<String>,
    pub field: String,
    pub original: String,
    pub name: Option<String>,
    pub extras: Vec<String>,
    pub marker: Option<String>,
    pub constraint: Option<String>,
    pub interpretation: CheckStatus,
    pub diagnostic: Option<String>,
    pub environment_digest: Option<Digest>,
    pub lock_digest: Option<Digest>,
    pub task: Option<TaskObservation>,
    pub referenced_path: Option<String>,
}

table!(
    /// Raw acquired bytes absent from SourceFiles. Interpretation is a separate observation.
    CapturedArtifacts, CapturedArtifactsRow = "captured_artifacts", family = Provenance,
    key = [snapshot_id, artifact_id], checks = [("bytes", "byte_len >= 0")],
    { snapshot_id: Id, artifact_id: Id, release_id: Id, context_id: Id,
      path: String, source_kind: String, source_digest: Digest, byte_len: i64,
      body: crate::column::Blob, alignment: String, provenance: String,
      observations: String }
);
table!(
    /// Generation-addressable original artifact. Body is copied only for referenced roots.
    CatalogArtifacts, CatalogArtifactsRow = "catalog_artifacts", family = Findings,
    key = [snapshot_id, artifact_id], checks = [("bytes", "byte_len >= 0")],
    { snapshot_id: Id, artifact_id: Id, release_id: Id, path: String,
      source_kind: String, source_digest: Digest, byte_len: i64,
      body: crate::column::Blob, alignment: String, provenance: String }
);
table!(
    CatalogSpans, CatalogSpansRow = "catalog_spans", family = Findings,
    key = [snapshot_id, span_id], checks = [("span", "start_byte >= 0 AND end_byte >= start_byte")],
    { snapshot_id: Id, span_id: Id, artifact_id: Id, start_byte: i64, end_byte: i64, extraction: String }
);
table!(
    /// Payload is the serialized ScenarioDetail contract; publication validates it.
    CatalogScenarios, CatalogScenariosRow = "catalog_scenarios", family = Findings,
    key = [snapshot_id, scenario_id], checks = [],
    { snapshot_id: Id, scenario_id: Id, primary_span_id: Id, detail: String }
);
table!(
    /// Payload is the serialized DeploymentDetail contract, not an unconstrained document.
    CatalogDeployments, CatalogDeploymentsRow = "catalog_deployments", family = Findings,
    key = [snapshot_id, deployment_id], checks = [],
    { snapshot_id: Id, deployment_id: Id, span_id: Id, ordinal: i64, derivation: String, detail: String }
);
table!(
    /// Association identity includes its site and role; multiplicity is never erased.
    CatalogAssociations, CatalogAssociationsRow = "catalog_associations", family = Findings,
    key = [snapshot_id, association_id], checks = [],
    { snapshot_id: Id, association_id: Id, member_id: Option<Id>, release_id: Option<Id>, evidence_id: Id, evidence_kind: String,
      role: String, basis: String, site_id: Option<Id>, intent: String, support: String }
);

pub fn artifact_id(release: Id, path: &str, digest: Digest) -> Id {
    IdHasher::new("evidence-artifact")
        .id(release)
        .str(path)
        .digest_field(digest)
        .finish_id()
}
pub fn span_id(artifact: Id, start: i64, end: i64) -> Id {
    IdHasher::new("evidence-span")
        .id(artifact)
        .i64(start)
        .i64(end)
        .finish_id()
}

/// Shared canonical/import/read-back validator for independently rooted evidence.
pub fn validate(
    artifacts: &[CatalogArtifactsRow],
    spans: &[CatalogSpansRow],
    scenarios: &[CatalogScenariosRow],
    deployments: &[CatalogDeploymentsRow],
    associations: &[CatalogAssociationsRow],
) -> Result<(), String> {
    use std::collections::{BTreeMap, BTreeSet};
    let artifacts: BTreeMap<_, _> = artifacts.iter().map(|a| (a.artifact_id, a)).collect();
    let span_map: BTreeMap<_, _> = spans.iter().map(|s| (s.span_id, s)).collect();
    for a in artifacts.values() {
        if a.byte_len != a.body.0.len() as i64
            || crate::id::content_digest(&a.body.0) != a.source_digest
            || artifact_id(a.release_id, &a.path, a.source_digest) != a.artifact_id
        {
            return Err("artifact bytes or identity disagree".into());
        }
        serde_json::from_value::<Alignment>(serde_json::Value::String(a.alignment.clone()))
            .map_err(|_| "unknown release alignment")?;
    }
    for s in spans {
        let a = artifacts
            .get(&s.artifact_id)
            .ok_or("dangling evidence artifact")?;
        if s.start_byte < 0
            || s.end_byte < s.start_byte
            || s.end_byte > a.byte_len
            || span_id(s.artifact_id, s.start_byte, s.end_byte) != s.span_id
        {
            return Err("invalid original evidence span".into());
        }
        if let Ok(text) = std::str::from_utf8(&a.body.0)
            && (!text.is_char_boundary(s.start_byte as usize)
                || !text.is_char_boundary(s.end_byte as usize))
        {
            return Err("evidence span splits UTF8".into());
        }
    }
    let scenario_ids: BTreeSet<_> = scenarios.iter().map(|s| s.scenario_id).collect();
    let deployment_ids: BTreeSet<_> = deployments.iter().map(|s| s.deployment_id).collect();
    for scenario in scenarios {
        let detail: ScenarioDetail = serde_json::from_str(&scenario.detail)
            .map_err(|e| format!("invalid scenario detail: {e}"))?;
        if detail.spans.first() != Some(&scenario.primary_span_id)
            || detail.spans.iter().any(|s| !span_map.contains_key(s))
            || detail
                .requirements
                .iter()
                .filter_map(|r| r.evidence)
                .any(|s| !span_map.contains_key(&s))
        {
            return Err("dangling scenario context".into());
        }
        if IdHasher::new("contextual-scenario-v1")
            .id(scenario.primary_span_id)
            .finish_id()
            != scenario.scenario_id
        {
            return Err("scenario identity drift".into());
        }
        if detail
            .option_bindings
            .iter()
            .filter_map(|o| o.span_id)
            .any(|id| !span_map.contains_key(&id))
        {
            return Err("dangling option source".into());
        }
        for option in &detail.option_bindings {
            if let Some(id) = option.span_id {
                let span = span_map[&id];
                let source = artifacts[&span.artifact_id];
                if option.coordinate_space != "original"
                    || source.body.0[span.start_byte as usize..span.end_byte as usize]
                        != *option.expression.as_bytes()
                {
                    return Err("option expression disagrees with original source".into());
                }
            } else if option.coordinate_space != "synthetic_analysis" {
                return Err("option lacks original source".into());
            }
        }
        // Execution outcomes require separate validated observations, never a source heuristic.
        if detail.checks.execution != CheckStatus::NotRun {
            return Err("unattributed scenario execution status".into());
        }
    }
    for deployment in deployments {
        if !span_map.contains_key(&deployment.span_id) {
            return Err("dangling deployment span".into());
        }
        let expected = match deployment.derivation.as_str() {
            "metadata" if deployment.ordinal >= 0 => IdHasher::new("deployment-metadata")
                .id(deployment.span_id)
                .i64(deployment.ordinal)
                .finish_id(),
            "snippet" if deployment.ordinal == 0 => IdHasher::new("deployment-snippet")
                .id(deployment.span_id)
                .finish_id(),
            _ => return Err("unknown deployment derivation".into()),
        };
        if expected != deployment.deployment_id {
            return Err("deployment identity drift".into());
        }
        let detail = serde_json::from_str::<DeploymentDetail>(&deployment.detail)
            .map_err(|e| format!("invalid deployment detail: {e}"))?;
        if let Some(task) = detail.task {
            use sha2::Digest as _;
            let receipt_span = span_map[&deployment.span_id];
            let receipt_artifact = artifacts[&receipt_span.artifact_id];
            let original: TaskReceipt = serde_json::from_slice(
                &receipt_artifact.body.0
                    [receipt_span.start_byte as usize..receipt_span.end_byte as usize],
            )
            .map_err(|_| "invalid original task receipt")?;
            if original != task.receipt {
                return Err("task receipt disagrees with original artifact".into());
            }
            let source = artifacts
                .get(&task.target_artifact)
                .ok_or("dangling task source")?;
            if source.path != task.receipt.source_path
                || format!("{:x}", sha2::Sha256::digest(&source.body.0))
                    != task.receipt.source_sha256
                || detail.environment_digest != Some(task.receipt.environment.environment_digest)
                || detail.lock_digest != Some(task.receipt.environment.lock_digest)
            {
                return Err("task receipt source/environment drift".into());
            }
        }
    }
    for a in associations {
        for (field, value) in [
            ("evidence_kind", &a.evidence_kind),
            ("role", &a.role),
            ("basis", &a.basis),
            ("intent", &a.intent),
        ] {
            if !crate::catalog::vocabulary("catalog_associations", field)
                .is_some_and(|values| values.contains(&value.as_str()))
            {
                return Err("unknown evidence association code".into());
            }
        }
        let exists = match a.evidence_kind.as_str() {
            "span" => span_map.contains_key(&a.evidence_id),
            "scenario" => scenario_ids.contains(&a.evidence_id),
            "deployment" => deployment_ids.contains(&a.evidence_id),
            _ => false,
        };
        if !exists {
            return Err("dangling or mistyped evidence association".into());
        }
        if a.member_id.is_some() == a.release_id.is_some() {
            return Err("association must have exactly one member or release subject".into());
        }
        if let Some(release) = a.release_id {
            let deployment = deployments
                .iter()
                .find(|d| d.deployment_id == a.evidence_id)
                .ok_or("release association requires a deployment")?;
            let original = artifacts[&span_map[&deployment.span_id].artifact_id];
            if a.evidence_kind != "deployment"
                || a.basis != "release_distribution"
                || a.role != "declares"
                || a.intent != "unknown"
                || a.site_id.is_some()
                || original.release_id != release
            {
                return Err("invalid release deployment scope".into());
            }
        }
        if a.member_id.is_some() && a.basis == "release_distribution" {
            return Err("release evidence relabeled as member evidence".into());
        }
        let id = IdHasher::new("evidence-association")
            .opt_id(a.member_id)
            .opt_id(a.release_id)
            .id(a.evidence_id)
            .str(&a.evidence_kind)
            .str(&a.role)
            .str(&a.basis)
            .opt_id(a.site_id)
            .str(&a.intent)
            .finish_id();
        let support: Vec<AssociationSupport> = serde_json::from_str(&a.support)
            .map_err(|e| format!("invalid association support: {e}"))?;
        if a.release_id.is_some() && !support.is_empty() {
            return Err("release evidence has provider call support".into());
        }
        for edge in &support {
            use crate::Codebook;
            let context = span_map
                .get(&edge.context_span_id)
                .ok_or("missing association site context")?;
            if edge.start_byte < 0
                || edge.end_byte < edge.start_byte
                || !matches!(
                    edge.coordinate_space.as_str(),
                    "original" | "synthetic_analysis"
                )
            {
                return Err("invalid association site context".into());
            }
            if edge.coordinate_space == "original"
                && (edge.start_byte < context.start_byte || edge.end_byte > context.end_byte)
            {
                return Err("association site escapes original context".into());
            }
            if !crate::codebook::Modality::all()
                .iter()
                .any(|m| m.text() == edge.modality)
                || !crate::codebook::InvocationPhase::all()
                    .iter()
                    .any(|p| p.text() == edge.phase)
            {
                return Err("unknown provider observation".into());
            }
            if a.basis == "resolved_target"
                && (edge.modality != "definite" || edge.unresolved_reason.is_some())
            {
                return Err("candidate evidence relabeled as resolved".into());
            }
        }
        let id = if let [edge] = support.as_slice() {
            IdHasher::new("evidence-association-supported")
                .id(id)
                .id(edge.edge_id)
                .finish_id()
        } else if support.is_empty() {
            id
        } else {
            return Err("association must preserve one provider edge per identity".into());
        };
        if id != a.association_id {
            return Err("association identity drift".into());
        }
    }
    Ok(())
}

pub fn validate_projection(
    tables: &std::collections::BTreeMap<String, Vec<arrow_array::RecordBatch>>,
) -> Result<(), crate::serving_projection::ProjectionError> {
    use crate::{Table, query::QueryRow, serving_projection::corrupt};
    fn read<T: Table>(
        tables: &std::collections::BTreeMap<String, Vec<arrow_array::RecordBatch>>,
    ) -> Result<Vec<T::Row>, crate::serving_projection::ProjectionError>
    where
        T::Row: QueryRow,
    {
        let mut out = vec![];
        for batch in tables
            .get(T::NAME)
            .ok_or_else(|| corrupt("missing evidence relation"))?
        {
            let ids = vec![Id::ZERO; batch.num_rows()];
            let mut columns = vec![<Id as crate::column::ArrowColumn>::array(ids.iter())];
            columns.extend(batch.columns().iter().cloned());
            let canonical = arrow_array::RecordBatch::try_new(T::schema(), columns)
                .map_err(|_| corrupt("evidence canonical codec"))?;
            out.extend(T::Row::read_batch(&canonical).map_err(|_| corrupt("evidence row codec"))?);
        }
        Ok(out)
    }
    validate(
        &read::<CatalogArtifacts>(tables)?,
        &read::<CatalogSpans>(tables)?,
        &read::<CatalogScenarios>(tables)?,
        &read::<CatalogDeployments>(tables)?,
        &read::<CatalogAssociations>(tables)?,
    )
    .map_err(corrupt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_observation_must_equal_original_receipt() {
        use sha2::Digest as _;
        let source = b"print('original')";
        let source_id = artifact_id(
            Id::ZERO,
            "examples/fastmcp_config/server.py",
            crate::id::content_digest(source),
        );
        let receipt = TaskReceipt {
            format: 1,
            policy: "fastmcp-stdio-v1".into(),
            task: "programmatic".into(),
            runner_sha256: "a".repeat(64),
            source_path: "examples/fastmcp_config/server.py".into(),
            source_sha256: format!("{:x}", sha2::Sha256::digest(source)),
            environment: DeploymentEnvironment {
                release_id: Id::ZERO,
                lock_digest: Digest([1; 32]),
                environment_digest: Digest([2; 32]),
                runtime_digest: Digest([3; 32]),
                interpreter_digest: Digest([4; 32]),
                python_version: "3.14.7".into(),
                platform: "linux".into(),
                requirement: "fastmcp==4.0.5".into(),
                metadata: Default::default(),
            },
            command: vec![],
            tool: "add".into(),
            arguments: Default::default(),
            elapsed_ms: 1,
            timeout_seconds: 45,
            execution: CheckStatus::Failed,
            tools: vec![],
            result: None,
            diagnostic: Some("observed failure".into()),
        };
        let bytes = serde_json::to_vec(&receipt).unwrap();
        let receipt_id = artifact_id(Id::ZERO, "task.json", crate::id::content_digest(&bytes));
        let artifact = |id, path: &str, body: &[u8]| CatalogArtifactsRow {
            snapshot_id: Id::ZERO,
            artifact_id: id,
            release_id: Id::ZERO,
            path: path.into(),
            source_kind: "task_observation".into(),
            source_digest: crate::id::content_digest(body),
            byte_len: body.len() as i64,
            body: crate::column::Blob(body.to_vec()),
            alignment: "exact".into(),
            provenance: "fixture".into(),
        };
        let artifacts = vec![
            artifact(source_id, &receipt.source_path, source),
            artifact(receipt_id, "task.json", &bytes),
        ];
        let span = CatalogSpansRow {
            snapshot_id: Id::ZERO,
            span_id: span_id(receipt_id, 0, bytes.len() as i64),
            artifact_id: receipt_id,
            start_byte: 0,
            end_byte: bytes.len() as i64,
            extraction: "original".into(),
        };
        let mut detail = DeploymentDetail {
            distribution: None,
            version: None,
            field: "task_observation".into(),
            original: "programmatic".into(),
            name: None,
            extras: vec![],
            marker: None,
            constraint: None,
            interpretation: CheckStatus::Passed,
            diagnostic: None,
            environment_digest: Some(receipt.environment.environment_digest),
            lock_digest: Some(receipt.environment.lock_digest),
            task: Some(TaskObservation {
                target_artifact: source_id,
                receipt,
            }),
            referenced_path: None,
        };
        let mut deployment = CatalogDeploymentsRow {
            snapshot_id: Id::ZERO,
            deployment_id: IdHasher::new("deployment-metadata")
                .id(span.span_id)
                .i64(0)
                .finish_id(),
            span_id: span.span_id,
            ordinal: 0,
            derivation: "metadata".into(),
            detail: serde_json::to_string(&detail).unwrap(),
        };
        assert!(
            validate(
                &artifacts,
                std::slice::from_ref(&span),
                &[],
                std::slice::from_ref(&deployment),
                &[]
            )
            .is_ok()
        );
        detail.task.as_mut().unwrap().receipt.execution = CheckStatus::Passed;
        deployment.detail = serde_json::to_string(&detail).unwrap();
        assert!(
            validate(&artifacts, &[span], &[], &[deployment], &[])
                .unwrap_err()
                .contains("disagrees with original")
        );
    }
}
