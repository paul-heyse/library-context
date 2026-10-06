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

impl NativeProjection {
    /// Serialize nominal vertices/arcs and exact model-owned side records, with the pin and
    /// admitted projection contract. No petgraph node or edge index crosses this boundary.
    pub fn write_json(&self, writer: impl Write) -> Result<(), ModelError> {
        #[derive(Serialize)]
        struct Export<'a> {
            snapshot: &'a SnapshotHandle,
            manifest: &'a Manifest,
            definition: &'a ProjectionDefinition,
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
        let _held = self.budget.reserve(
            "native-projection-export-view",
            self.graph
                .vertex_count()
                .saturating_mul(size_of::<&EntityRef>())
                .saturating_add(self.graph.arc_count().saturating_mul(size_of::<Arc>()))
                .saturating_add(
                    self.source
                        .subjects()
                        .count()
                        .saturating_mul(size_of::<&ProjectionGapSubject>()),
                )
                .saturating_add(
                    self.source
                        .gaps()
                        .count()
                        .saturating_mul(size_of::<&ProjectionGap>()),
                )
                .saturating_add(self.coverage.len().saturating_mul(
                    size_of::<&ProviderCoverage>() + size_of::<&ProjectionSourceCoverage>(),
                )),
        )?;
        let spec =
            lctx_model::domain::analysis::ProjectionDefinition::builtin(self.graph.key().name);
        serde_json::to_writer(writer, &Export {
            snapshot: &self.snapshot, manifest: &self.manifest, definition: &self.definition,
            specification: serde_json::json!({"name":spec.name,"version":spec.version,"policy":spec.policy}),
            key: self.graph.key(), assessment: self.source.assessment(),
            vertices: self.graph.entities().collect(), arcs: self.graph.arcs().collect(),
            gap_subjects: self.source.subjects().collect(), gaps: self.source.gaps().collect(),
            source_coverage: self.source.coverage().collect(), coverage: self.coverage.iter().collect(),
        }).map_err(ModelError::codec)
    }
}

pub async fn materialize(
    reader: &NativeReader,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<NativeProjection, ModelError> {
    let manifest_bytes: Vec<Bytes> = reader
        .query(
            "SELECT VALUE manifest FROM publication:current",
            Variables::new(),
        )
        .await?;
    let manifest: Manifest = serde_json::from_slice(
        manifest_bytes
            .first()
            .filter(|_| manifest_bytes.len() == 1)
            .ok_or(ModelError::Schema("published projection manifest"))?,
    )
    .map_err(ModelError::codec)?;
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
    Ok(NativeProjection {
        snapshot: reader.handle().clone(),
        manifest,
        definition,
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
async fn hydrate(
    reader: &NativeReader,
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
    let roots: Vec<RecordId> = reader.query("RETURN array::concat(\
        (SELECT VALUE id FROM entity WHERE semantic_type IN $types AND scope_keys CONTAINSANY $types.map(|$type|$type+'|input|'+<string>$input) AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)),\
        (SELECT VALUE id FROM assertion WHERE semantic_type IN $types AND scope_keys CONTAINSANY $types.map(|$type|$type+'|input|'+<string>$input) AND (body.context=NONE OR body.context=NULL OR scope_context=<string>$context)),\
        (SELECT VALUE id FROM entity WHERE semantic_type=$symbols AND scope_keys CONTAINS ($symbols+'|context|'+<string>$context)));", bindings).await?;
    // Input sum records (for example CoverageScope::Input) use arm-prefixed physical
    // columns. Their logical input reference is authoritative native adjacency, so include
    // the exact capture as an ownership anchor rather than guessing those columns.
    let capture = crate::reader::target_id(lctx_model::domain::graph::Target::Entity(
        lctx_model::domain::graph::EntityId::of(key.input),
    ));
    let mut seen = BTreeSet::new();
    let mut frontier: Vec<_> = std::iter::once(capture)
        .chain(roots)
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
        let next: Vec<RecordId> = reader.query("RETURN array::distinct(array::concat(\
            (SELECT VALUE in FROM reference WHERE out IN $frontier AND field IN $fields AND in.semantic_type IN $types AND (in.body.context=NONE OR in.body.context=NULL OR in.scope_context=<string>$context)),\
            (SELECT VALUE in FROM participant WHERE out IN $frontier AND field IN $fields AND in.semantic_type IN $types AND (in.body.context=NONE OR in.body.context=NULL OR in.scope_context=<string>$context))));", vars).await?;
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
        let next: Vec<RecordId> = reader.query("RETURN array::distinct(array::concat(\
            (SELECT VALUE out FROM reference WHERE in IN $frontier AND out.semantic_type IN $types),\
            (SELECT VALUE out FROM participant WHERE in IN $frontier AND out.semantic_type IN $types)));", vars).await?;
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
