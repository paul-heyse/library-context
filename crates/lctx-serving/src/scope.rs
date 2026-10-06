//! Indexed request roots, finite owned children, and required outgoing semantic dependencies.
use lctx_model::domain::{ModelError, ValidationInput, resources::ResourceBudget};
use lctx_surrealdb::{NativeReader, batches::CanonicalBatches};
use std::collections::BTreeSet;
use surrealdb::types::{Variables, RecordId};

/// Incoming ownership is deliberately separate from outgoing reference closure.
/// Shared capture/context/provider/type identity must never pull in unrelated owners.
pub const OWNED_FIELDS: &[&str] = &[
    "member", "occurrence", "declaration", "exposure", "candidate", "callable", "invocation", "variant", "signature",
    "parameter", "slot", "domain", "witness", "assessment", "observation", "subject",
    "conclusion", "proof", "assertion", "universe", "set", "sequence", "list", "field_list", "parameters",
];

pub async fn hydrate(reader:&NativeReader, roots:Vec<RecordId>, inputs:&[ValidationInput],
    budget:&ResourceBudget)->Result<CanonicalBatches,ModelError> {
    hydrate_with(reader,roots,inputs,OWNED_FIELDS,budget).await
}
/// Each component chooses its semantic ownership closure; shared identities are never owners.
pub async fn hydrate_with(reader:&NativeReader, roots:Vec<RecordId>, inputs:&[ValidationInput],
    owned_fields:&[&str],budget:&ResourceBudget)->Result<CanonicalBatches,ModelError> {
    let types: BTreeSet<_> = inputs.iter().map(|i|i.name().to_owned()).collect();
    let mut seen=BTreeSet::new();
    let mut frontier:Vec<_>=roots.into_iter().filter(|id|seen.insert(id.clone())).collect();
    let mut charge=budget.reserve("native-request-closure",frontier.len()*128)?;
    while !frontier.is_empty() {
        let mut vars=Variables::new();
        vars.insert("frontier",frontier);
        vars.insert("types",types.iter().cloned().collect::<Vec<_>>());
        vars.insert("fields",owned_fields.to_vec());
        // No prefix replay: every node enters the frontier exactly once. The four adjacency
        // selections use the physical incoming/outgoing indexes on graph relation endpoints.
        let next:Vec<RecordId>=reader.query("RETURN array::distinct(array::concat(\
            (SELECT VALUE out FROM reference WHERE in IN $frontier AND out.semantic_type IN $types),\
            (SELECT VALUE out FROM participant WHERE in IN $frontier AND out.semantic_type IN $types),\
            (SELECT VALUE in FROM reference WHERE out IN $frontier AND field IN $fields AND in.semantic_type IN $types),\
            (SELECT VALUE in FROM participant WHERE out IN $frontier AND field IN $fields AND in.semantic_type IN $types),            (SELECT VALUE id FROM entity WHERE semantic_type='provider_runs' AND semantic_type IN $types AND scope_input IN (SELECT VALUE scope_input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','catalog_members','retrieval_units','input_revisions'])),            (SELECT VALUE id FROM assertion WHERE semantic_type='input_distributions' AND semantic_type IN $types AND scope_input IN array::concat((SELECT VALUE scope_input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','catalog_members','retrieval_units']),(SELECT VALUE <string>body.library FROM assertion WHERE semantic_type='corpus_libraries' AND scope_corpus IN (SELECT VALUE scope_input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','retrieval_units'])))),(SELECT VALUE id FROM assertion WHERE semantic_type='corpus_libraries' AND semantic_type IN $types AND scope_corpus IN (SELECT VALUE scope_input FROM entity WHERE id IN $frontier AND semantic_type IN ['source_artifacts','retrieval_units']))));",vars).await?;
        frontier=next.into_iter().filter(|id|seen.insert(id.clone())).collect();
        charge.try_resize(seen.len().saturating_mul(128))?;
    }
    let mut vars=Variables::new();
    vars.insert("nodes",seen.into_iter().collect::<Vec<_>>());
    vars.insert("types",types.into_iter().collect::<Vec<_>>());
    reader.canonical_batches("RETURN array::concat(\
        (SELECT 'entity' AS node_kind, canonical FROM entity WHERE id IN $nodes AND semantic_type IN $types),\
        (SELECT 'assertion' AS node_kind, canonical FROM assertion WHERE id IN $nodes AND semantic_type IN $types));".into(),vars,budget).await
}
