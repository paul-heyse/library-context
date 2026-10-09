//! Indexed request roots, finite owned children, and required outgoing semantic dependencies.
use lctx_model::domain::{ModelError, ValidationInput, resources::ResourceBudget};
use lctx_surrealdb::{NativeReader, batches::CanonicalBatches};
use std::collections::BTreeSet;
use surrealdb::types::{RecordId, Variables};

/// Incoming ownership is deliberately separate from outgoing reference closure.
/// Shared capture/context/provider/type identity must never pull in unrelated owners.
pub const OWNED_FIELDS: &[&str] = &[
    "member",
    "occurrence",
    "declaration",
    "exposure",
    "candidate",
    "callable",
    "invocation",
    "variant",
    "signature",
    "parameter",
    "field_option",
    "parameter_option",
    "slot",
    "domain",
    "witness",
    "assessment",
    "observation",
    "subject",
    "conclusion",
    "proof",
    "assertion",
    "universe",
    "set",
    "sequence",
    "list",
    "field_list",
    "parameters",
];

pub async fn hydrate(
    reader: &NativeReader,
    roots: Vec<RecordId>,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
    hydrate_with(reader, roots, inputs, OWNED_FIELDS, budget).await
}
/// Each component chooses its semantic ownership closure; shared identities are never owners.
#[allow(
    clippy::mutable_key_type,
    reason = "Graph node IDs are immutable generated string keys; the SDK key union includes unused mutable regex caches"
)]
pub async fn hydrate_with(
    reader: &NativeReader,
    roots: Vec<RecordId>,
    inputs: &[ValidationInput],
    owned_fields: &[&str],
    budget: &ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
    hydrate_with_owners(reader, roots, inputs, inputs, owned_fields, budget).await
}
/// Incoming ownership is the consumer's finite read inventory; outgoing dependencies
/// can include a wider canonical proof universe without admitting unrelated owners.
#[allow(
    clippy::mutable_key_type,
    reason = "Graph node IDs are immutable generated string keys; the SDK key union includes unused mutable regex caches"
)]
pub async fn hydrate_with_owners(
    reader: &NativeReader,
    roots: Vec<RecordId>,
    inputs: &[ValidationInput],
    incoming_inputs: &[ValidationInput],
    owned_fields: &[&str],
    budget: &ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
    let types: BTreeSet<_> = inputs.iter().map(|i| i.name().to_owned()).collect();
    let incoming_types: Vec<_> = incoming_inputs
        .iter()
        .map(|i| i.name().to_owned())
        .collect();
    let mut seen = BTreeSet::new();
    let mut frontier: Vec<_> = roots
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect();
    let mut charge = budget.reserve("native-request-closure", frontier.len() * 128)?;
    while !frontier.is_empty() {
        let mut vars = Variables::new();
        vars.insert("frontier", frontier);
        vars.insert("types", types.iter().cloned().collect::<Vec<_>>());
        vars.insert("incoming_types", incoming_types.clone());
        vars.insert("fields", owned_fields.to_vec());
        // No prefix replay: every node enters the frontier exactly once. Four keyed graph
        // walks combine the exact incoming admission rules before returning their endpoints.
        // Capture companions use finite keys computed once, not nested frontier queries
        // evaluated inside each assertion's scope predicate. Corpus membership still resolves
        // distribution inputs even when corpus rows themselves are outside the selected types.
        let corpus_keys=lctx_surrealdb::prepared::scope_constants("'corpus_libraries'","corpus","$source_inputs");
        let distribution_keys=lctx_surrealdb::prepared::scope_constants("'input_distributions'","input","array::concat($capture_inputs,$capture_corpora.map(|$c|$c.library))");
        let next:Vec<RecordId>=reader.query(format!("RETURN {{\
            LET $capture_inputs = SELECT VALUE body.input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','catalog_members','retrieval_units'];\
            LET $source_inputs = SELECT VALUE body.input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','retrieval_units'];\
            LET $corpus_keys = {corpus_keys};\
            LET $capture_corpora = SELECT id, body.library AS library FROM assertion WHERE semantic_type='corpus_libraries' AND scope_keys CONTAINSANY $corpus_keys;\
            LET $distribution_keys = {distribution_keys};\
            LET $corpus_ids = IF 'corpus_libraries' IN $types THEN $capture_corpora.map(|$c|$c.id) ELSE [] END;\
            RETURN array::distinct(array::concat(\
            (SELECT VALUE out FROM $frontier->reference WHERE out.semantic_type IN $types),\
            (SELECT VALUE out FROM $frontier->participant WHERE out.semantic_type IN $types),\
            (SELECT VALUE in FROM $frontier<-reference WHERE in.semantic_type IN $incoming_types AND (field IN $fields OR (field='input' AND in.semantic_type='provider_runs'))),\
            (SELECT VALUE in FROM $frontier<-participant WHERE in.semantic_type IN $incoming_types AND (field IN $fields OR\
                (field='scope' AND in.semantic_type='provider_coverage') OR\
                (field='run' AND in.semantic_type='run_families') OR\
                (field IN ['atom','binding'] AND in.semantic_type='guard_substitutions') OR\
                (field='variable' AND in.semantic_type='type_binder_assessments') OR\
                (field='term' AND in.semantic_type='type_entity_links') OR\
                (field='place' AND in.semantic_type='place_entity_links') OR\
                (field='resolution' AND in.semantic_type='normalized_call_resolution_evidence'))),\
            (SELECT VALUE id FROM assertion WHERE semantic_type='input_distributions' AND semantic_type IN $types AND scope_keys CONTAINSANY $distribution_keys),\
            $corpus_ids)); }};"),vars).await?;
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
        (SELECT 'entity' AS node_kind, canonical FROM $nodes WHERE record::table(id)='entity' AND semantic_type IN $types),\
        (SELECT 'assertion' AS node_kind, canonical FROM $nodes WHERE record::table(id)='assertion' AND semantic_type IN $types));".into(),vars,budget).await
}
