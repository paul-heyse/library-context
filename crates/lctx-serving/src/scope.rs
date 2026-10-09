//! Indexed request roots, finite owned children, and required outgoing semantic dependencies.
pub use lctx_model::domain::serving_scope::OWNED_FIELDS;
use lctx_model::domain::{
    ModelError, ValidationInput, resources::ResourceBudget, serving_scope::ServingScopeProgram,
};
use lctx_surrealdb::{NativeReader, batches::CanonicalBatches};
use std::collections::BTreeSet;
use surrealdb::types::RecordId;

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
    let program = ServingScopeProgram::new(inputs, incoming_inputs, owned_fields, budget)?;
    let prepared = lctx_surrealdb::scope::PreparedServingScope::new(program, budget)?;
    // Preparation is request-owned, reused for every frontier. Only root values change.
    let mut charge = budget.reserve("native-request-closure", roots.len().saturating_mul(384))?;
    let mut seen = BTreeSet::new();
    let mut frontier: Vec<_> = roots
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect();
    while !frontier.is_empty() {
        let next: Vec<RecordId> = reader
            .query_prepared(prepared.frontier_query(frontier)?)
            .await?;
        // Admit the result handoff before growing the retained set/frontier. The existing
        // NativeReader checks the whole response and all statement terminals before exposure.
        charge.try_resize(seen.len().saturating_add(next.len()).saturating_mul(384))?;
        frontier = next
            .into_iter()
            .filter(|id| seen.insert(id.clone()))
            .collect();
        charge.try_resize(seen.len().saturating_mul(384))?;
        tokio::task::yield_now().await;
    }
    prepared
        .hydrate(reader, seen.into_iter().collect(), budget)
        .await
}
