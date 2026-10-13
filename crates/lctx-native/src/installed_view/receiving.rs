//! Independent receiving pins over the exact opaque source capabilities.
use super::*;
use d::{charged::StateCharge, resources::ResourceBudget};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use surrealdb::types::{Object, Variables};

pub(crate) async fn protect_receiving_views(
    views: &[Arc<HeldInstalledView>],
    pin: &crate::control::ReaderPin,
    reader: &crate::NativeReader<()>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "receiving exact contributor inventory");
    let _scratch = budget.reserve("receiving fixed identity scratch", 4096)?;
    let mut records = BTreeMap::<RecordId, Arc<crate::control::ExpectedRecord>>::new();
    let mut holds = BTreeSet::<(RecordId, RecordId)>::new();
    let mut context = None;
    // Complete correspondence and conflicting-copy refusal precede receiving effects.
    // Arc handles borrow charged observations owned by the actual source pins.
    for view in views {
        let Some(source) = view.earned_source() else {
            continue;
        };
        source.pin().ensure_live()?;
        if context.is_some_and(|known| known != source.context()) {
            return Err(ModelError::Conflict(
                "receiving contributor installation context",
            ));
        }
        context = Some(source.context());
        for expected in source.contributors().values() {
            let id = expected.record();
            if let Some(previous) = records.get(id) {
                if previous.incarnation() != expected.incarnation()
                    || previous.row() != expected.row()
                {
                    return Err(ModelError::Conflict(
                        "receiving conflicting selected physical copy",
                    ));
                }
            } else {
                charge.grow(crate::selection::node_bytes(id).saturating_add(256))?;
                records.insert(id.clone(), expected.clone());
            }
            let pair = (source.owner().clone(), id.clone());
            if !holds.contains(&pair) {
                charge.grow(
                    crate::selection::node_bytes(&pair.0)
                        .saturating_add(crate::selection::node_bytes(&pair.1))
                        .saturating_add(128),
                )?;
                holds.insert(pair);
            }
        }
    }
    charge.grow(
        records
            .len()
            .saturating_mul(std::mem::size_of::<Arc<crate::control::ExpectedRecord>>() + 1024)
            .saturating_add(
                holds
                    .len()
                    .saturating_mul(std::mem::size_of::<(RecordId, RecordId)>()),
            )
            .saturating_add(4096),
    )?;
    let selected = records.into_values().collect::<Vec<_>>();
    crate::control::live_view::transfer_expected_many(pin, &selected, budget).await?;
    if let Some((generation, epoch)) = context {
        let actual: Vec<Object> = reader
            .query_native(
                "SELECT generation,validity_epoch FROM native_installation:current",
                Variables::new(),
            )
            .await?;
        let [actual] = actual.as_slice() else {
            return Err(ModelError::Conflict(
                "receiving current installation cardinality",
            ));
        };
        if actual.get("generation") != Some(&Value::String(generation.hex()))
            || actual.get("validity_epoch")
                != Some(&Value::Number(surrealdb::types::Number::Int(epoch)))
        {
            return Err(ModelError::Conflict(
                "receiving current installation changed",
            ));
        }
        for window in selected.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let ids = window
                .iter()
                .map(|expected| expected.record().clone())
                .collect::<Vec<_>>();
            let mut actual =
                crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
            for expected in window {
                let row = actual.take_required(expected.record())?;
                if row.incarnation() != expected.incarnation() || row.row() != expected.row() {
                    return Err(ModelError::Conflict(
                        "receiving current contributor incarnation/full bytes changed",
                    ));
                }
            }
            actual.finish()?;
        }
        let pairs = holds.into_iter().collect::<Vec<_>>();
        for pairs in pairs.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let mut actual =
                crate::control::inspect_hold_window_read(reader, pairs, budget).await?;
            for (owner, object) in pairs {
                actual.take_required(owner, object)?;
            }
            actual.finish()?;
        }
    }
    for view in views {
        if let Some(source) = view.earned_source() {
            source.pin().ensure_live()?;
        } else {
            // Retained classes preserve their independent parent/selector/admission
            // and publication witnesses; mappings cannot replace that authority.
            view.protect_with_pin(pin, budget).await?;
            view.ensure_current(reader, budget).await?;
        }
    }
    pin.ensure_live()
}
