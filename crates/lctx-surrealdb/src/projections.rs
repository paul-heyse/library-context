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
use surrealdb::types::{Bytes, RecordId, Variables};

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
    let selected_entity = reader.selected_node_predicate("id");
    let roots: Vec<RecordId> = reader.query_prepared_native(crate::prepared::PreparedQuery::new(bindings,preparation,vec![format!("RETURN array::concat(\
        (SELECT VALUE id FROM entity WHERE ({selected_entity}) AND (anchor=$capture OR (semantic_type IN $types AND scope_keys CONTAINSANY $__projection_input_scopes AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)))),\
        (SELECT VALUE id FROM assertion WHERE semantic_type IN $types AND ({selected_entity}) AND scope_keys CONTAINSANY $__projection_input_scopes AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)),\
        (SELECT VALUE id FROM entity WHERE semantic_type=$symbols AND ({selected_entity}) AND scope_keys CONTAINS $__projection_context_scope))")])?).await?;
    let mut seen = BTreeSet::new();
    let mut frontier: Vec<_> = roots
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect();
    let mut charge = budget.reserve(
        "native-projection-selection",
        seen.len().saturating_mul(128),
    )?;
    // Ownership first: input isolates and every owned relationship are selected before following
    // resolved endpoints. Context/provider identities cannot pull in unrelated input owners.
    while !frontier.is_empty() {
        let mut vars = Variables::new();
        vars.insert("frontier", frontier);
        vars.insert("types", types.iter().cloned().collect::<Vec<_>>());
        vars.insert("fields", OWNED_FIELDS.to_vec());
        vars.insert(
            "context",
            crate::loader::json_value(
                serde_json::to_value(key.context).map_err(ModelError::codec)?,
            )?,
        );
        let selected = reader.selected_node_predicate("in");
        let next: Vec<RecordId> = reader.query(format!("RETURN array::distinct(array::concat(\
            (SELECT VALUE in FROM reference WHERE out IN (SELECT VALUE anchor FROM $frontier) AND field IN $fields AND ({selected}) AND in.semantic_type IN $types AND (in.body.context=NONE OR in.body.context=NULL OR in.scope_context=<string>$context)),\
            (SELECT VALUE in FROM participant WHERE out IN (SELECT VALUE anchor FROM $frontier) AND field IN $fields AND ({selected}) AND in.semantic_type IN $types AND (in.body.context=NONE OR in.body.context=NULL OR in.scope_context=<string>$context))));"), vars).await?;
        frontier = next
            .into_iter()
            .filter(|id| seen.insert(id.clone()))
            .collect();
        charge.try_resize(seen.len().saturating_mul(128))?;
    }
    frontier = seen.iter().cloned().collect();
    while !frontier.is_empty() {
        let mut vars = Variables::new();
        vars.insert("frontier", frontier);
        vars.insert("types", types.iter().cloned().collect::<Vec<_>>());
        let selected = reader.selected_node_predicate("id");
        let next: Vec<RecordId> = reader.query(format!("RETURN array::concat(\
            (SELECT VALUE id FROM entity WHERE semantic_type IN $types AND ({selected}) AND anchor IN array::concat((SELECT VALUE out FROM reference WHERE in IN $frontier),(SELECT VALUE out FROM participant WHERE in IN $frontier))),\
            (SELECT VALUE id FROM assertion WHERE semantic_type IN $types AND ({selected}) AND anchor IN array::concat((SELECT VALUE out FROM reference WHERE in IN $frontier),(SELECT VALUE out FROM participant WHERE in IN $frontier))));"), vars).await?;
        frontier = next
            .into_iter()
            .filter(|id| seen.insert(id.clone()))
            .collect();
        charge.try_resize(seen.len().saturating_mul(128))?;
    }
    let mut vars = Variables::new();
    vars.insert("nodes", seen.into_iter().collect::<Vec<_>>());
    vars.insert("types", types.into_iter().collect::<Vec<_>>());
    reader.canonical_batches("RETURN array::concat(\
        (SELECT 'entity' AS node_kind, canonical FROM entity WHERE id IN $nodes AND semantic_type IN $types),\
        (SELECT 'assertion' AS node_kind, canonical FROM assertion WHERE id IN $nodes AND semantic_type IN $types));".into(), vars, budget).await
}

pub fn name(raw: &str) -> Result<ProjectionName, String> {
    ProjectionName::ALL
        .into_iter()
        .find(|name| format!("{name:?}") == raw)
        .ok_or_else(|| format!("unknown named projection {raw:?}"))
}
