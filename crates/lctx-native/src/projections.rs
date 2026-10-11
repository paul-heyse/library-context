//! Complete named topology from indexed input/context roots and their semantic dependencies.
use crate::{NativeReader, RecordSelection, batches::CanonicalBatches};
use lctx_model::domain::{
    ModelError, Record,
    attribution::ProviderCoverage,
    graph::{Manifest, ProjectionDefinition},
    normalized::{Rows, entities::EntityRef},
    projection::{
        Arc, ProjectionAvailability, ProjectionGap, ProjectionGapSubject, ProjectionName,
        ProjectionSourceAssessment, ProjectionSourceCoverage, ProjectionSpec,
        normalization::{self, ProjectionData, ProjectionInput, ProjectionKey},
        snapshot::MaterializedGraph,
    },
    resources::ResourceBudget,
    serving::SnapshotHandle,
};
use serde::Serialize;
use std::{collections::BTreeSet, io::Write};
use surrealdb::types::{Bytes, RecordId, SurrealValue, Value, Variables};

// These fields connect a source owner to its own projection records. They are traversed
// only during ownership selection; resolved foreign endpoints get forward closure alone.
const OWNED_FIELDS: &[&str] = &[
    "input",
    "artifact",
    "module",
    "occurrence",
    "declaration",
    "source",
    "scope",
    "qualification",
    "symbol",
    "callable",
    "class",
    "parameter",
    "field",
    "owner",
    "site",
    "event",
    "alternative",
    "assessment",
    "observation",
    "reference",
    "access",
    "exposure",
    "statement",
    "alias",
    "read",
    "parent",
];

pub struct NativeProjection {
    pub snapshot: SnapshotHandle,
    pub manifest: Manifest,
    pub definition: ProjectionDefinition,
    pub source: ProjectionInput,
    pub graph: MaterializedGraph,
    pub coverage: Rows<ProviderCoverage>,
    budget: ResourceBudget,
}

/// Exact completed-view kernel result. It establishes no publication authority.
pub struct ScopedProjection {
    pub source: ProjectionInput,
    pub graph: MaterializedGraph,
    pub coverage: Rows<ProviderCoverage>,
    budget: ResourceBudget,
}
impl ScopedProjection {
    pub fn write_json(&self, writer: impl Write) -> Result<(), ModelError> {
        write_projection_json(
            None,
            None,
            None,
            &self.source,
            &self.graph,
            &self.coverage,
            &self.budget,
            writer,
        )
    }
}
impl NativeProjection {
    /// The published wrapper retains the exact manifest and executable definition authority.
    pub fn write_json(&self, writer: impl Write) -> Result<(), ModelError> {
        write_projection_json(
            Some(&self.snapshot),
            Some(&self.manifest),
            Some(&self.definition),
            &self.source,
            &self.graph,
            &self.coverage,
            &self.budget,
            writer,
        )
    }
}
fn write_projection_json(
    snapshot: Option<&SnapshotHandle>,
    manifest: Option<&Manifest>,
    definition: Option<&ProjectionDefinition>,
    source: &ProjectionInput,
    graph: &MaterializedGraph,
    coverage: &Rows<ProviderCoverage>,
    budget: &ResourceBudget,
    writer: impl Write,
) -> Result<(), ModelError> {
    #[derive(Serialize)]
    struct Export<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        snapshot: Option<&'a SnapshotHandle>,
        #[serde(skip_serializing_if = "Option::is_none")]
        manifest: Option<&'a Manifest>,
        #[serde(skip_serializing_if = "Option::is_none")]
        definition: Option<&'a ProjectionDefinition>,
        specification: serde_json::Value,
        key: ProjectionKey,
        assessment: ProjectionSourceAssessment,
        vertices: Vec<&'a EntityRef>,
        arcs: Vec<Arc>,
        gap_subjects: Vec<&'a ProjectionGapSubject>,
        gaps: Vec<&'a ProjectionGap>,
        source_coverage: Vec<&'a ProjectionSourceCoverage>,
        coverage: Vec<&'a ProviderCoverage>,
    }
    let _held = budget.reserve(
        "native-projection-export-view",
        graph
            .vertex_count()
            .saturating_mul(size_of::<&EntityRef>())
            .saturating_add(graph.arc_count().saturating_mul(size_of::<Arc>()))
            .saturating_add(
                source
                    .subjects()
                    .count()
                    .saturating_mul(size_of::<&ProjectionGapSubject>()),
            )
            .saturating_add(
                source
                    .gaps()
                    .count()
                    .saturating_mul(size_of::<&ProjectionGap>()),
            )
            .saturating_add(coverage.len().saturating_mul(
                size_of::<&ProviderCoverage>() + size_of::<&ProjectionSourceCoverage>(),
            )),
    )?;
    let spec = lctx_model::domain::analysis::ProjectionDefinition::builtin(graph.key().name);
    serde_json::to_writer(writer, &Export {
            snapshot, manifest, definition,
            specification: serde_json::json!({"name":spec.name,"version":spec.version,"policy":spec.policy}),
            key: graph.key(), assessment: source.assessment(),
            vertices: graph.entities().collect(), arcs: graph.arcs().collect(),
            gap_subjects: source.subjects().collect(), gaps: source.gaps().collect(),
            source_coverage: source.coverage().collect(), coverage: coverage.iter().collect(),
        }).map_err(ModelError::codec)
}

pub async fn materialize(
    reader: &NativeReader,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<NativeProjection, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert(
        "publication",
        RecordId::new("publication", reader.handle().publication.hex()),
    );
    let manifest_bytes: Vec<Bytes> = reader
        .query("SELECT VALUE manifest FROM $publication", bindings)
        .await?;
    let manifest = Manifest::decode(
        manifest_bytes
            .first()
            .filter(|_| manifest_bytes.len() == 1)
            .ok_or(ModelError::Schema("published projection manifest"))?,
    )?;
    if manifest.content() != reader.handle().semantic {
        return Err(ModelError::Conflict("published projection manifest"));
    }
    let definition = manifest
        .projections
        .iter()
        .find(|p| p.name == format!("{:?}", key.name))
        .cloned()
        .ok_or(ModelError::Schema("named projection is not admitted"))?;
    if definition.definition
        != lctx_model::domain::analysis::ProjectionDefinition::builtin(key.name).content_digest()
    {
        return Err(ModelError::Conflict("named projection definition"));
    }
    let scoped = materialize_kernel(reader, key, budget).await?;
    Ok(NativeProjection {
        snapshot: reader.handle().clone(),
        manifest,
        definition,
        source: scoped.source,
        graph: scoped.graph,
        coverage: scoped.coverage,
        budget: scoped.budget,
    })
}

/// Materialize a finite completed-view graph using its model-owned source assessment.
/// The caller supplies actual typed completed records, never a synthetic publication marker.
pub async fn materialize_scoped(
    reader: &NativeReader<()>,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<ScopedProjection, ModelError> {
    materialize_kernel(reader, key, budget).await
}
async fn materialize_kernel<Context>(
    reader: &NativeReader<Context>,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<ScopedProjection, ModelError> {
    if reader.view_bindings().get("lctx_views").is_none() {
        return Err(ModelError::Conflict(
            "projection requires exact completed views",
        ));
    }
    let expected_key = ProjectionSourceAssessment {
        input: key.input,
        context: key.context,
        projection: key.name,
        version: ProjectionSpec::VERSION,
        vertices: 0,
        arcs: 0,
        gaps: 0,
        availability: ProjectionAvailability::CompleteUnderStatedModel,
    };
    let assessments = reader
        .records::<ProjectionSourceAssessment>(RecordSelection::Keys(vec![
            *expected_key.id().bytes(),
        ]))
        .await?;
    let expected = assessments
        .first()
        .filter(|_| assessments.len() == 1)
        .ok_or(ModelError::Schema(
            "named projection input/context is not admitted",
        ))?;
    let batches = hydrate(reader, key, budget).await?;
    let mut data = ProjectionData::new(budget);
    for (name, batch) in &batches.batches {
        if !data.visit(name, batch)? {
            return Err(ModelError::Schema("undeclared native projection input"));
        }
    }
    let source = normalization::describe(&data, key, budget)?;
    if source.assessment() != *expected {
        return Err(ModelError::Conflict("native projection source assessment"));
    }
    let mut coverage = Rows::new(budget);
    for link in source.coverage() {
        coverage.insert(
            data.coverage
                .get(link.coverage)
                .ok_or(ModelError::Schema("native projection coverage source"))?
                .clone(),
        )?;
    }
    let graph = MaterializedGraph::build(&source, budget)?;
    Ok(ScopedProjection {
        source,
        graph,
        coverage,
        budget: budget.clone(),
    })
}

#[allow(
    clippy::mutable_key_type,
    reason = "Sealed native records use string IDs; SDK regex caches are not present and IDs are never mutated"
)]
async fn hydrate<Context>(
    reader: &NativeReader<Context>,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
    let types: BTreeSet<_> = ProjectionData::stage_inputs()
        .iter()
        .map(|r| r.name().to_owned())
        .collect();
    let mut bindings = Variables::new();
    bindings.insert("types", types.iter().cloned().collect::<Vec<_>>());
    bindings.insert(
        "input",
        crate::loader::json_value(serde_json::to_value(key.input).map_err(ModelError::codec)?)?,
    );
    bindings.insert(
        "context",
        crate::loader::json_value(serde_json::to_value(key.context).map_err(ModelError::codec)?)?,
    );
    bindings.insert(
        "symbols",
        <lctx_model::domain::calls::ProviderSymbol as Record>::NAME,
    );
    let preparation = vec![
        "LET $__projection_input_scopes = $types.map(|$type| $type+'|input|'+<string>$input)"
            .into(),
        format!(
            "LET $__projection_context_scope = {}",
            crate::prepared::scope_constant("$symbols", "context", "$context")
        ),
    ];
    let capture = crate::reader::target_id(lctx_model::domain::graph::Target::Entity(
        lctx_model::domain::graph::EntityId::of(key.input),
    ));
    bindings.insert("capture", capture);
    let mut seen = BTreeSet::new(); let mut charge = budget.reserve("native-projection-selection", 0)?;
    let mut frontier = Vec::new();
    for (table, predicate) in [
        ("entity", "anchor=$capture OR (semantic_type IN $types AND scope_keys CONTAINSANY $__projection_input_scopes AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)) OR (semantic_type=$symbols AND scope_keys CONTAINS $__projection_context_scope)"),
        ("assertion", "semantic_type IN $types AND scope_keys CONTAINSANY $__projection_input_scopes AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)"),
    ] {
        frontier.extend(projection_candidates(reader, format!("SELECT id FROM {table} WHERE {predicate}"), bindings.clone(), preparation.clone(), &mut seen, charge.as_mut(), budget).await?);
    }
    // Owner-first traversal retains isolates and owned relationships. Every native frontier
    // argument is finite; selected membership is tested against the prepared disk cursor.
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for window in frontier.chunks(128) {
            let mut vars = bindings.clone(); vars.insert("frontier", window.to_vec()); vars.insert("fields", OWNED_FIELDS.to_vec());
            for table in ["reference", "participant"] {
                next.extend(projection_candidates(reader, format!("SELECT in AS id FROM {table} WITH INDEX incoming WHERE out IN (SELECT VALUE anchor FROM $frontier) AND field IN $fields AND in.semantic_type IN $types AND (in.body.context=NONE OR in.body.context=NULL OR in.scope_context=<string>$context)"), vars.clone(), vec![], &mut seen, charge.as_mut(), budget).await?);
            }
        }
        frontier = next;
    }
    frontier = seen.iter().cloned().collect();
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for window in frontier.chunks(128) {
            for edge in ["reference", "participant"] {
                let mut vars = bindings.clone(); vars.insert("frontier", window.to_vec());
                // Endpoint anchors are read from an actual indexed edge stream, never a
                // closure-sized server array. Resolve each finite anchor window separately.
                let mut edges = reader.stream_prepared(crate::prepared::PreparedQuery::new(vars, vec![], vec![format!("SELECT out AS id FROM {edge} WITH INDEX outgoing WHERE in IN $frontier")])?)?;
                let result = async {
                    loop {
                        let mut anchors = Vec::new(); while anchors.len() < 128 { let Some(row) = edges.next().await? else { break; }; anchors.push(projection_id(&row)?); }
                        if anchors.is_empty() { break; }
                        let mut vars = bindings.clone(); vars.insert("anchors", anchors);
                        for table in ["entity", "assertion"] {
                            next.extend(projection_candidates(reader, format!("SELECT id FROM {table} WITH INDEX anchor_payload WHERE anchor IN $anchors AND semantic_type IN $types"), vars.clone(), vec![], &mut seen, charge.as_mut(), budget).await?);
                        }
                    }
                    Ok(())
                }.await;
                let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("projection outgoing edge drainage", edges.drain_transport().await); lctx_model::domain::completion::complete(result, completion)?;
            }
        }
        frontier = next;
    }
    let nodes = seen.into_iter().collect::<Vec<_>>(); let types = types.into_iter().collect::<Vec<_>>();
    let result = reader.canonical_point_batches(&nodes, &types, budget).await;
    drop(charge); result

}

fn projection_id(row: &Value) -> Result<RecordId, ModelError> {
    let Value::Object(row) = row else { return Err(ModelError::Schema("projection candidate object")); };
    RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("projection candidate identity"))?).map_err(ModelError::codec)
}
async fn projection_candidates<Context>(reader: &NativeReader<Context>, sql: String, vars: Variables, preparation: Vec<String>, seen: &mut BTreeSet<RecordId>, charge: &mut dyn lctx_model::domain::resources::Reservation, budget: &ResourceBudget) -> Result<Vec<RecordId>, ModelError> {
    let mut rows = reader.stream_prepared(crate::prepared::PreparedQuery::new(vars, preparation, vec![sql])?)?;
    let mut next = Vec::new();
    let result = async {
        loop {
            let mut batch = Vec::new(); while batch.len() < 128 { let Some(row) = rows.next().await? else { break; }; batch.push(projection_id(&row)?); }
            if batch.is_empty() { break; }
            let batch = reader.selected_candidate_ids(&batch, budget).await?;
            for node in batch { if !seen.contains(&node) { charge.try_resize(seen.len().saturating_add(1).saturating_mul(384))?; seen.insert(node.clone()); next.push(node); } }
        }
        Ok(())
    }.await;
    let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("projection indexed candidate drainage", rows.drain_transport().await); lctx_model::domain::completion::complete(result, completion)?;
    Ok(next)
}

pub fn name(raw: &str) -> Result<ProjectionName, String> {
    ProjectionName::ALL
        .into_iter()
        .find(|name| format!("{name:?}") == raw)
        .ok_or_else(|| format!("unknown named projection {raw:?}"))
}
