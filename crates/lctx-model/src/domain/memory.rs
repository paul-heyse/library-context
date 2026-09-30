//! A pure in-memory generation. It receives the same stage-bound batches as the PostgreSQL store and
//! validates them the same way: unique keys, complete (and subtype-correct) references, the ordered
//! content digest and every model invariant fed in its declared input order. Physical row-size
//! admission stays with the batch writer and the COPY boundary.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::resources::{ResourceBudget, TRANSFER_ROWS};
use super::stages::{AttemptIdentity, Execution, StageSink, WritePermit, StageCompletion, CompletedStage, RelationReceipt};
use super::{Batch, ContentHash, KeySink, ModelError, Record, Relation, ValidatedModel};
use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray, Int16Array, RecordBatch, UInt32Array};
use arrow_row::{RowConverter, SortField};
use arrow_schema::SortOptions;
use std::{collections::BTreeMap, sync::Mutex};

struct Stored {
    charge: StateCharge,
    relations: BTreeMap<&'static str, Vec<RecordBatch>>,
    frozen: std::collections::BTreeSet<&'static str>,
}
/// Stage-bound when created by `bind`; a conformance generation accepts direct `put` instead.
pub struct MemoryGeneration {
    model: ContentHash,
    attempt: Option<AttemptIdentity>,
    stored: Mutex<Stored>,
    relations: BTreeMap<&'static str, Relation>,
    budget: ResourceBudget,
}
impl MemoryGeneration {
    /// A conformance generation for fixtures that are not produced by a schedule.
    pub fn conformance(model: &ValidatedModel, budget: &ResourceBudget) -> Self {
        Self {
            model: model.digest(),
            attempt: None,
            relations: model.relations().iter().map(|r| (r.name(), r.clone())).collect(),
            budget: budget.clone(),
            stored: Mutex::new(Stored {
                charge: StateCharge::new(budget, "memory-generation"),
                relations: BTreeMap::new(),
                frozen: Default::default(),
            }),
        }
    }
    /// The one sink of an execution; only that execution's write permits are accepted.
    pub fn bind(
        model: &ValidatedModel,
        budget: &ResourceBudget,
        execution: &mut Execution<'_>,
    ) -> Result<Self, ModelError> {
        if execution.schedule().model() != model.digest() {
            return Err(ModelError::Invalid(
                "memory generation model differs from its schedule".into(),
            ));
        }
        execution.bind_sink()?;
        Ok(Self {
            attempt: Some(execution.identity()),
            ..Self::conformance(model, budget)
        })
    }
    pub fn put<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        if self.attempt.is_some() {
            return Err(ModelError::Invalid(
                "a stage-bound generation accepts only permitted writes".into(),
            ));
        }
        self.store(R::NAME, batch.arrow())
    }
    fn store(&self, relation: &'static str, arrow: &RecordBatch) -> Result<(), ModelError> {
        let mut stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        if stored.frozen.contains(relation) {
            return Err(ModelError::Invalid("completed relation is immutable".into()));
        }
        // Stored batches outlive their producer's reservation, so the generation charges them.
        stored.charge.grow(arrow.get_array_memory_size())?;
        stored
            .relations
            .entry(relation)
            .or_default()
            .push(arrow.clone());
        Ok(())
    }
    /// Read typed records from this in-memory generation. This is an inspection helper for bounded
    /// development inputs; persistent product readers pin a PostgreSQL generation instead.
    pub fn read<R: Record>(
        &self,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<Batch<R>, ModelError> {
        if model.digest() != self.model {
            return Err(ModelError::Invalid("memory read model differs".into()));
        }
        let stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        let mut charge = StateCharge::new(budget, "memory-generation-read");
        let mut rows = Vec::new();
        for batch in stored.relations.get(R::NAME).into_iter().flatten() {
            charge.grow(batch.get_array_memory_size().saturating_mul(4))?;
            rows.extend(R::decode(batch)?);
        }
        Batch::new(model, rows, budget)
    }
    /// Validate the stored contents; the result equals the PostgreSQL store's content digest.
    pub fn validate(
        &self,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<ContentHash, ModelError> {
        if model.digest() != self.model {
            return Err(ModelError::Invalid(
                "memory generation model differs".into(),
            ));
        }
        self.validate_inner(model, budget)
    }
    /// Validate and admit exactly the facts scope, with the same relation digest and invariant
    /// selection as persistent publication. Relations above the frontier are not empty facts.
    pub fn validate_facts(
        &self,
        model: &ValidatedModel,
        budget: &ResourceBudget,
        preflight: super::admission::Preflight,
        receipt: &super::stages::ExecutionReceipt,
    ) -> Result<super::admission::FrontierAdmission, ModelError> {
        if model.digest() != self.model || preflight.contract().model() != model.digest() {
            return Err(ModelError::Invalid("memory facts model differs".into()));
        }
        let scope = ValidatedModel::validate(super::facts_relations())?;
        let content = self.validate_inner(&scope, budget)?;
        let mut check = super::admission::AdmissionCheck::new(preflight, budget);
        let stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        let mut charge = StateCharge::new(budget, "memory-admission-input");
        for input in super::admission::AdmissionCheck::inputs() {
            let relation = model
                .relations()
                .iter()
                .find(|r| r.name() == input.name())
                .expect("admission model input");
            let parts = stored
                .relations
                .get(input.name())
                .map(Vec::as_slice)
                .unwrap_or_default();
            charge.grow(
                parts
                    .iter()
                    .map(|b| b.get_array_memory_size())
                    .sum::<usize>()
                    .saturating_mul(4),
            )?;
            let batch = arrow_select::concat::concat_batches(relation.schema(), parts)
                .map_err(ModelError::codec)?;
            let ordered = order(relation, &batch, input.order())?;
            for chunk in chunks(&ordered) {
                check.visit(input.name(), &chunk)?;
            }
        }
        check.finish(receipt, content)
    }
    fn validate_inner(
        &self,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<ContentHash, ModelError> {
        let stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        let mut charge = StateCharge::new(budget, "memory-generation-validation");
        let mut sorted = BTreeMap::new();
        for relation in model.relations() {
            let batches = stored
                .relations
                .get(relation.name())
                .map(Vec::as_slice)
                .unwrap_or_default();
            charge.grow(
                batches
                    .iter()
                    .map(|b| b.get_array_memory_size())
                    .sum::<usize>()
                    .saturating_mul(4)
                    .saturating_add(4096),
            )?;
            let all = arrow_select::concat::concat_batches(relation.schema(), batches)
                .map_err(ModelError::codec)?;
            sorted.insert(relation.name(), order(relation, &all, &["id"])?);
        }
        let keys = Keys::collect(model, &sorted, &mut charge)?;
        keys.references(model, &sorted)?;
        let mut content = KeySink::new("generation-content");
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
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .expect("validated invariant member");
                let batch = order(relation, &sorted[relation.name()], input.order())?;
                for chunk in chunks(&batch) {
                    check.visit(input.name(), &chunk)?;
                }
            }
            check.finish()?;
        }
        Ok(content.finish())
    }
}
impl StageSink for MemoryGeneration {
    async fn complete(&self, completion: StageCompletion) -> Result<CompletedStage, ModelError> {
        if self.attempt != Some(completion.identity().attempt()) || completion.model() != self.model {
            return Err(ModelError::Invalid("foreign memory stage completion".into()));
        }
        let mut stored = self.stored.lock().map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        let mut receipts = BTreeMap::new();
        for name in completion.outputs() {
            if stored.frozen.contains(name) { return Err(ModelError::Invalid("stage already completed".into())); }
            let relation = self.relations.get(name).ok_or_else(|| ModelError::Invalid("undeclared completion output".into()))?;
            let parts = stored.relations.get(name).ok_or_else(|| ModelError::Invalid("stage output not written".into()))?;
            let _reservation = self.budget.reserve("memory-stage-receipt", parts.iter().map(|b| b.get_array_memory_size()).sum::<usize>().saturating_mul(4).saturating_add(4096))?;
            let all = arrow_select::concat::concat_batches(relation.schema(), parts).map_err(ModelError::codec)?;
            let ordered = order(relation, &all, &["id"])?;
            let mut content = relation.content();
            relation.hash_rows(&ordered, &mut content)?;
            let (rows, content) = content.finish();
            receipts.insert(*name, RelationReceipt { rows, content });
        }
        stored.frozen.extend(completion.outputs());
        completion.acknowledge(receipts)
    }
    fn copy<R: Record>(
        &self,
        permit: WritePermit<'_, R>,
        batch: &Batch<R>,
    ) -> impl Future<Output = Result<(), ModelError>> + Send {
        let result =
            if self.attempt != Some(permit.identity().attempt()) || permit.model() != self.model {
                Err(ModelError::Invalid(
                    "write permit belongs to another generation attempt".into(),
                ))
            } else {
                self.store(R::NAME, batch.arrow())
            };
        std::future::ready(result)
    }
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
