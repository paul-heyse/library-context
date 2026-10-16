//! Complete independent receiving protection over exact opaque source capabilities.
use super::*;
use d::{charged::StateCharge, resources::ResourceBudget};
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::types::{Object, Variables};

/// Complete source manifests, successful exact CAS inventory and the actual receiving
/// pin stay together through the last scoped reader/borrower. This is not serializable.
pub(crate) struct ReceivingViews {
    sources: Vec<std::sync::Weak<HeldInstalledView>>,
    received: Vec<Arc<HeldInstalledView>>,
    transferred: crate::control::live_view::TransferredExpected,
    context: (ContentHash, i64),
    charge: StateCharge,
}
impl ReceivingViews {
    pub(crate) fn views(&self) -> &[Arc<HeldInstalledView>] {
        &self.received
    }
    pub(crate) fn ensure_for(
        &self,
        view: &HeldInstalledView,
        client: &Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
        budget: &ResourceBudget,
    ) -> Result<bool, ModelError> {
        if !self.sources.iter().any(|source| std::ptr::eq(source.as_ptr(), view))
            && !self.received.iter().any(|source| std::ptr::eq(source.as_ref(), view))
        {
            return Ok(false);
        }
        if !self.charge.budget().is_some_and(|pool| pool.shares_pool(budget)) {
            return Err(ModelError::Conflict("receiving premise foreign resource pool"));
        }
        view.require_pool(budget)?;
        if context(view)? != self.context {
            return Err(ModelError::Conflict("receiving premise selected context"));
        }
        self.transferred.ensure_live(client)?;
        Ok(true)
    }
}
fn context(view: &HeldInstalledView) -> Result<(ContentHash, i64), ModelError> {
    if let Some(source) = view.earned_source() {
        Ok(source.context())
    } else {
        let selected = view.retained_completion()?;
        Ok((selected.generation, selected.validity_epoch))
    }
}
fn include(
    records: &mut BTreeMap<RecordId, Arc<crate::control::ExpectedRecord>>,
    expected: Arc<crate::control::ExpectedRecord>,
    charge: &mut StateCharge,
) -> Result<(), ModelError> {
    let id = expected.record();
    if let Some(previous) = records.get(id) {
        if previous.incarnation() != expected.incarnation() || previous.row() != expected.row() {
            return Err(ModelError::Conflict("receiving conflicting selected physical copy"));
        }
    } else {
        charge.grow(crate::selection::node_bytes(id).saturating_add(256))?;
        records.insert(id.clone(), expected);
    }
    Ok(())
}
pub(crate) async fn protect_receiving_views(
    views: &[Arc<HeldInstalledView>],
    pin: &Arc<crate::control::ReaderPin>,
    reader: &crate::NativeReader<()>,
    budget: &ResourceBudget,
) -> Result<ReceivingViews, ModelError> {
    let mut charge = StateCharge::new(budget, "receiving complete immutable premise");
    charge.grow(views.len().saturating_mul(std::mem::size_of::<Arc<HeldInstalledView>>() + std::mem::size_of::<std::sync::Weak<HeldInstalledView>>()).saturating_add(4096))?;
    let mut records_charge = StateCharge::new(budget, "receiving exact acquisition map");
    let mut records = BTreeMap::<RecordId, Arc<crate::control::ExpectedRecord>>::new();
    let mut selected_context = None;
    // Complete correspondence, exact pool and unequal-copy refusal precede effects.
    // Source handles stay borrowed across every page and final handoff.
    for view in views {
        view.require_pool(budget)?;
        let observed = context(view)?;
        if selected_context.is_some_and(|known| known != observed) {
            return Err(ModelError::Conflict("receiving contributor installation context"));
        }
        selected_context = Some(observed);
        if let Some(source) = view.earned_source() {
            source.pin().ensure_live()?;
            for expected in source.contributors().values() {
                include(&mut records, expected.clone(), &mut records_charge)?;
            }
        } else {
            // Independent owners earn retained selector/parent/admission and publication
            // premises once. Existing operation scopes do not replay them per window.
            view.ensure_current(reader, budget).await?;
            let selector = view.retained_completion()?;
            let expected = crate::control::inspect_expected_read(reader, selector.record()?, budget)
                .await?.ok_or(ModelError::Conflict("retained receiving selector missing"))?;
            if expected.incarnation() != view.completion_incarnation() || expected.row() != view.expected_bytes() {
                return Err(ModelError::Conflict("retained receiving selector changed"));
            }
            include(&mut records, Arc::new(expected), &mut records_charge)?;
            let boundary = RecordId::new("native_completed_state", selector.parent_completion.hex());
            let expected = crate::control::inspect_expected_read(reader, boundary, budget)
                .await?.ok_or(ModelError::Conflict("retained receiving parent missing"))?;
            let data = crate::completed_state::FrozenCompletion::from_row(expected.row(), true)?;
            if data.identity()? != selector.parent_completion || data.generation != observed.0 || data.validity_epoch != observed.1 {
                return Err(ModelError::Conflict("retained receiving parent changed"));
            }
            include(&mut records, Arc::new(expected), &mut records_charge)?;
        }
    }
    let selected_context = selected_context.ok_or(ModelError::Conflict("receiving exact view inventory empty"))?;
    let transferred = crate::control::live_view::transfer_expected_many(pin, records.into_values().collect(), budget).await?;
    drop(records_charge);
    // Every page has compared full rows/incarnations and earned actual receiver holds.
    // A second complete row/source-hold replay grants no new protection.
    let actual: Vec<Object> = reader.query_native(
        "SELECT generation,validity_epoch FROM native_installation:current", Variables::new(),
    ).await?;
    let [actual] = actual.as_slice() else {
        return Err(ModelError::Conflict("receiving current installation cardinality"));
    };
    if actual.get("generation") != Some(&Value::String(selected_context.0.hex()))
        || actual.get("validity_epoch") != Some(&Value::Number(surrealdb::types::Number::Int(selected_context.1)))
    {
        return Err(ModelError::Conflict("receiving current installation changed"));
    }
    for view in views {
        if let Some(source) = view.earned_source() {
            source.pin().ensure_live()?;
        }
    }
    transferred.ensure_live(&reader.shared_client())?;
    if transferred.records().is_empty() {
        return Err(ModelError::Conflict("receiving protected physical inventory empty"));
    }
    let received = views.iter().map(|view| view.receiving_copy(pin, budget).map(Arc::new)).collect::<Result<Vec<_>, _>>()?;
    Ok(ReceivingViews { sources: views.iter().map(Arc::downgrade).collect(), received, transferred, context: selected_context, charge })
}
