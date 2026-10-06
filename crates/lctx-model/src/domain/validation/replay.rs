//! Stateless replay of completed, bounded batches through the shared model invariants.
//! This driver is for small independent algorithm/contract controls. Compiler publication uses
//! streamed graph shape and bulk nominal closure, not a materialized relation store.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::resources::{ResourceBudget, TRANSFER_ROWS};
use super::*;
use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray, Int16Array, RecordBatch, UInt32Array};
use arrow_row::{RowConverter, SortField};
use arrow_schema::SortOptions;
use std::collections::BTreeMap;

pub fn replay(
    model: &ValidatedModel,
    batches: &[(&'static str, RecordBatch)],
    budget: &ResourceBudget,
) -> Result<ContentHash, ModelError> {
    let mut charge = StateCharge::new(budget, "completed-batch-replay");
    let mut sorted = BTreeMap::new();
    for relation in model.relations() {
        let selected = batches
            .iter()
            .filter(|(name, _)| *name == relation.name())
            .map(|(_, batch)| batch.clone())
            .collect::<Vec<_>>();
        charge.grow(
            selected
                .iter()
                .map(logical_batch_bytes)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .sum::<usize>()
                .saturating_mul(4)
                .saturating_add(4096),
        )?;
        let all = arrow_select::concat::concat_batches(relation.schema(), &selected)
            .map_err(ModelError::codec)?;
        sorted.insert(relation.name(), order(relation, &all, &["id"])?);
    }
    let keys = Keys::collect(model, &sorted, &mut charge)?;
    keys.references(model, &sorted)?;
    let mut content = KeySink::new("completed-relations-control/v1");
    for relation in model.relations() {
        let mut rows = relation.content();
        for chunk in chunks(&sorted[relation.name()]) {
            relation.hash_rows(&chunk, &mut rows)?;
        }
        let (_, digest) = rows.finish();
        content.part(relation.name().as_bytes(), &digest.0);
    }
    for invariant in model.invariants() {
        let mut check = (invariant.create)(budget);
        for input in &invariant.inputs {
            let relation = model
                .relation(input.name())
                .expect("validated invariant member");
            let batch = order(relation, &sorted[relation.name()], input.order())?;
            for chunk in chunks(&batch) {
                check.visit_input(input, &chunk)?;
            }
        }
        check.finish()?;
    }
    Ok(content.finish())
}
/// Ascending on the named columns with nulls last and byte-wise text, as the store's `ORDER BY`
/// with `COLLATE "C"` produces. Ties keep their prior order.
pub fn order(
    relation: &Relation,
    batch: &RecordBatch,
    columns: &[&str],
) -> Result<RecordBatch, ModelError> {
    let options = SortOptions {
        descending: false,
        nulls_first: false,
    };
    let arrays: Vec<ArrayRef> = columns
        .iter()
        .map(|name| {
            batch.column_by_name(name).cloned().ok_or_else(|| {
                ModelError::Invalid(format!("{} has no order column {name}", relation.name()))
            })
        })
        .collect::<Result<_, _>>()?;
    let converter = RowConverter::new(
        arrays
            .iter()
            .map(|a| SortField::new_with_options(a.data_type().clone(), options))
            .collect(),
    )
    .map_err(ModelError::codec)?;
    let rows = converter
        .convert_columns(&arrays)
        .map_err(ModelError::codec)?;
    let mut indices: Vec<u32> =
        (0..u32::try_from(batch.num_rows()).map_err(ModelError::codec)?).collect();
    indices.sort_by(|a, b| rows.row(*a as usize).cmp(&rows.row(*b as usize)));
    arrow_select::take::take_record_batch(batch, &UInt32Array::from(indices))
        .map_err(ModelError::codec)
}
fn chunks(batch: &RecordBatch) -> impl Iterator<Item = RecordBatch> + '_ {
    (0..batch.num_rows())
        .step_by(TRANSFER_ROWS)
        .map(move |start| batch.slice(start, TRANSFER_ROWS.min(batch.num_rows() - start)))
}
fn ids(batch: &RecordBatch, column: &str) -> Result<FixedSizeBinaryArray, ModelError> {
    batch
        .column_by_name(column)
        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>().cloned())
        .ok_or_else(|| ModelError::Invalid(format!("reference column {column} is not an identity")))
}
fn key(array: &FixedSizeBinaryArray, row: usize) -> Result<[u8; 16], ModelError> {
    array
        .value(row)
        .try_into()
        .map_err(|_| ModelError::Invalid("identity is not 16 bytes".into()))
}

/// Every stored identity, and the tag of every sum row, as the store's keys and subtype keys hold them.
struct Keys {
    ids: BTreeMap<&'static str, ChargedSet<[u8; 16]>>,
    tags: BTreeMap<&'static str, ChargedMap<[u8; 16], i16>>,
}
impl Keys {
    fn collect(
        model: &ValidatedModel,
        sorted: &BTreeMap<&'static str, RecordBatch>,
        charge: &mut StateCharge,
    ) -> Result<Self, ModelError> {
        let mut keys = Self {
            ids: BTreeMap::new(),
            tags: BTreeMap::new(),
        };
        for relation in model.relations() {
            let batch = &sorted[relation.name()];
            let column = ids(batch, "id")?;
            let tags = relation
                .sum()
                .map(|sum| {
                    batch
                        .column_by_name(sum.tag)
                        .and_then(|c| c.as_any().downcast_ref::<Int16Array>().cloned())
                        .ok_or_else(|| {
                            ModelError::Invalid(format!("{} has no tag column", relation.name()))
                        })
                })
                .transpose()?;
            let (mut set, mut map) = (ChargedSet::default(), ChargedMap::default());
            for row in 0..batch.num_rows() {
                let id = key(&column, row)?;
                if !set.insert(charge, id)? {
                    return Err(ModelError::Conflict(relation.name()));
                }
                if let Some(tags) = &tags {
                    map.insert(charge, id, tags.value(row))?;
                }
            }
            keys.ids.insert(relation.name(), set);
            keys.tags.insert(relation.name(), map);
        }
        Ok(keys)
    }
    fn references(
        &self,
        model: &ValidatedModel,
        sorted: &BTreeMap<&'static str, RecordBatch>,
    ) -> Result<(), ModelError> {
        for relation in model.relations() {
            let batch = &sorted[relation.name()];
            for field in relation.fields().iter().filter(|f| !f.list()) {
                let Some((_, target)) = field.target() else {
                    continue;
                };
                let column = ids(batch, field.name())?;
                for row in (0..batch.num_rows()).filter(|row| !column.is_null(*row)) {
                    let id = key(&column, row)?;
                    let present = match field.subtype() {
                        Some(code) => self.tags[target].get(&id) == Some(&code),
                        None => self.ids[target].contains(&id),
                    };
                    if !present {
                        return Err(ModelError::Invalid(format!(
                            "{}.{} references an absent or wrong-subtype {target}",
                            relation.name(),
                            field.name()
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}
