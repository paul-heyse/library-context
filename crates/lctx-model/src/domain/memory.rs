//! A pure in-memory generation. It receives the same stage-bound batches as the PostgreSQL store and
//! validates them the same way: unique keys, complete (and subtype-correct) references, the ordered
//! content digest and every model invariant fed in its declared input order. Physical row-size
//! admission stays with the batch writer and the COPY boundary.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::resources::{ResourceBudget, TRANSFER_ROWS};
use super::stages::{
    AttemptIdentity, ClosedGroup, CompletedStage, ComputedStage, Execution, GroupCompletion,
    RelationReceipt, StageCompletion, StageSink, WritePermit, is_vocabulary,
};
use super::{Batch, ContentHash, KeySink, ModelError, Record, Relation, ValidatedModel};
use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray, Int16Array, RecordBatch, UInt32Array};
use arrow_row::{RowConverter, SortField};
use arrow_schema::SortOptions;
use std::{collections::BTreeMap, sync::Mutex};

struct Stored {
    charge: StateCharge,
    relations: BTreeMap<&'static str, Vec<RecordBatch>>,
    frozen: std::collections::BTreeSet<&'static str>,
    deltas: BTreeMap<(&'static str, &'static str), Vec<RecordBatch>>,
    sealed: std::collections::BTreeSet<(&'static str, &'static str)>,
}
/// Stage-bound when created by `bind`; a conformance generation accepts direct `put` instead.
pub struct MemoryGeneration {
    model: ContentHash,
    attempt: Option<AttemptIdentity>,
    stored: Mutex<Stored>,
    relations: BTreeMap<&'static str, Relation>,
    budget: ResourceBudget,
    grouped: std::collections::BTreeSet<&'static str>,
}
impl MemoryGeneration {
    /// A conformance generation for fixtures that are not produced by a schedule.
    pub fn conformance(model: &ValidatedModel, budget: &ResourceBudget) -> Self {
        Self {
            model: model.digest(),
            attempt: None,
            relations: model
                .relations()
                .iter()
                .map(|r| (r.name(), r.clone()))
                .collect(),
            budget: budget.clone(),
            grouped: Default::default(),
            stored: Mutex::new(Stored {
                charge: StateCharge::new(budget, "memory-generation"),
                relations: BTreeMap::new(),
                frozen: Default::default(),
                deltas: Default::default(),
                sealed: Default::default(),
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
            grouped: execution
                .schedule()
                .publication_groups()
                .iter()
                .flat_map(|g| g.stages.iter().copied())
                .collect(),
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
            return Err(ModelError::Invalid(
                "completed relation is immutable".into(),
            ));
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
        for input in check.inputs() {
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
    async fn compute(&self, completion: StageCompletion) -> Result<ComputedStage, ModelError> {
        if self.attempt != Some(completion.identity().attempt())
            || completion.model() != self.model
            || !self.grouped.contains(completion.stage())
        {
            return Err(ModelError::Invalid("foreign memory computation".into()));
        }
        let mut stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory state poisoned".into()))?;
        let mut receipts = BTreeMap::new();
        for name in completion.outputs() {
            let key = (completion.stage(), *name);
            if stored.sealed.contains(&key) {
                return Err(ModelError::Invalid("delta already sealed".into()));
            }
            let relation = self
                .relations
                .get(name)
                .ok_or_else(|| ModelError::Invalid("unknown delta".into()))?;
            let parts = stored
                .deltas
                .get(&key)
                .ok_or_else(|| ModelError::Invalid("delta unwritten".into()))?;
            let _charge = self.budget.reserve(
                "memory-delta-receipt",
                parts
                    .iter()
                    .map(|b| b.get_array_memory_size())
                    .sum::<usize>()
                    .saturating_mul(5)
                    .saturating_add(4096),
            )?;
            let all = arrow_select::concat::concat_batches(relation.schema(), parts)
                .map_err(ModelError::codec)?;
            let all = unique(relation, &all)?;
            let mut content = relation.content();
            relation.hash_rows(&all, &mut content)?;
            let (rows, content) = content.finish();
            receipts.insert(*name, RelationReceipt { rows, content });
            stored.sealed.insert(key);
        }
        completion.seal(receipts)
    }
    async fn close_group(&self, group: GroupCompletion) -> Result<ClosedGroup, ModelError> {
        let model = ValidatedModel::validate(self.relations.values().cloned().collect())?;
        let mut stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory state poisoned".into()))?;
        let total = stored
            .relations
            .values()
            .chain(stored.deltas.values())
            .flatten()
            .map(|b| b.get_array_memory_size())
            .sum::<usize>();
        let _temporary = self.budget.reserve(
            "memory-publication",
            total
                .saturating_mul(8)
                .saturating_add(4096 * self.relations.len()),
        )?;
        let mut candidate = stored.relations.clone();
        for stage in group.stages() {
            if self.attempt != Some(stage.completion().identity().attempt())
                || stage.completion().model() != self.model
            {
                return Err(ModelError::Invalid("foreign memory publication".into()));
            }
            for name in stage.completion().outputs() {
                let key = (stage.completion().stage(), *name);
                if !stored.sealed.contains(&key)
                    || (!is_vocabulary(name) && stored.frozen.contains(name))
                {
                    return Err(ModelError::Invalid(
                        "unsealed or completed group output".into(),
                    ));
                }
                let relation = &self.relations[name];
                let parts = stored
                    .deltas
                    .get(&key)
                    .ok_or_else(|| ModelError::Invalid("missing delta".into()))?;
                let delta = unique(
                    relation,
                    &arrow_select::concat::concat_batches(relation.schema(), parts)
                        .map_err(ModelError::codec)?,
                )?;
                let mut content = relation.content();
                relation.hash_rows(&delta, &mut content)?;
                let (rows, content) = content.finish();
                if stage.deltas().get(name) != Some(&RelationReceipt { rows, content }) {
                    return Err(ModelError::Invalid("delta receipt changed".into()));
                }
                candidate.entry(name).or_default().extend(parts.clone());
            }
        }
        let mut sorted = BTreeMap::new();
        for relation in model.relations() {
            let all = arrow_select::concat::concat_batches(
                relation.schema(),
                candidate
                    .get(relation.name())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
            .map_err(ModelError::codec)?;
            sorted.insert(
                relation.name(),
                if is_vocabulary(relation.name()) {
                    unique(relation, &all)?
                } else {
                    order(relation, &all, &["id"])?
                },
            );
        }
        let mut charge = StateCharge::new(&self.budget, "memory-group-references");
        let keys = Keys::collect(&model, &sorted, &mut charge)?;
        keys.references(&model, &sorted)?;
        for invariant in model
            .invariants()
            .iter()
            .filter(|i| i.inputs.iter().all(|r| candidate.contains_key(r.name())))
        {
            let mut check = (invariant.create)(&self.budget);
            for input in &invariant.inputs {
                let relation = &self.relations[input.name()];
                let batch = order(relation, &sorted[input.name()], input.order())?;
                for chunk in chunks(&batch) {
                    check.visit(input.name(), &chunk)?;
                }
            }
            check.finish()?;
        }
        let mut outputs = BTreeMap::new();
        for stage in group.stages() {
            let mut receipts = BTreeMap::new();
            for name in stage.completion().outputs() {
                let relation = &self.relations[name];
                let mut content = relation.content();
                relation.hash_rows(&sorted[name], &mut content)?;
                let (rows, content) = content.finish();
                receipts.insert(*name, RelationReceipt { rows, content });
            }
            outputs.insert(stage.completion().stage(), receipts);
        }
        let mut vocabulary = BTreeMap::new();
        for name in candidate.keys().filter(|name| is_vocabulary(name)) {
            let relation = &self.relations[name];
            let mut content = relation.content();
            relation.hash_rows(&sorted[name], &mut content)?;
            let (rows, content) = content.finish();
            vocabulary.insert(*name, RelationReceipt { rows, content });
        }
        // Admit the replacement before changing canonical state; failures expose no group output.
        let new_size = candidate
            .keys()
            .map(|name| sorted[name].get_array_memory_size())
            .sum::<usize>();
        stored.charge.grow(new_size)?;
        for stage in group.stages() {
            for name in stage.completion().outputs() {
                stored.deltas.remove(&(stage.completion().stage(), *name));
                stored.frozen.insert(name);
            }
        }
        stored.relations = candidate
            .keys()
            .map(|name| (*name, vec![sorted[name].clone()]))
            .collect();
        stored.charge.release(total);
        group.acknowledge(outputs, vocabulary)
    }
    async fn complete(&self, completion: StageCompletion) -> Result<CompletedStage, ModelError> {
        if self.attempt != Some(completion.identity().attempt()) || completion.model() != self.model
        {
            return Err(ModelError::Invalid(
                "foreign memory stage completion".into(),
            ));
        }
        let mut stored = self
            .stored
            .lock()
            .map_err(|_| ModelError::Invalid("memory generation poisoned".into()))?;
        let mut receipts = BTreeMap::new();
        for name in completion.outputs() {
            if stored.frozen.contains(name) {
                return Err(ModelError::Invalid("stage already completed".into()));
            }
            let relation = self
                .relations
                .get(name)
                .ok_or_else(|| ModelError::Invalid("undeclared completion output".into()))?;
            let parts = stored
                .relations
                .get(name)
                .ok_or_else(|| ModelError::Invalid("stage output not written".into()))?;
            let _reservation = self.budget.reserve(
                "memory-stage-receipt",
                parts
                    .iter()
                    .map(|b| b.get_array_memory_size())
                    .sum::<usize>()
                    .saturating_mul(4)
                    .saturating_add(4096),
            )?;
            let all = arrow_select::concat::concat_batches(relation.schema(), parts)
                .map_err(ModelError::codec)?;
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
            } else if self.grouped.contains(permit.stage()) {
                let mut stored = self
                    .stored
                    .lock()
                    .map_err(|_| ModelError::Invalid("memory state poisoned".into()));
                match &mut stored {
                    Ok(stored) => {
                        let key = (permit.stage(), R::NAME);
                        if stored.sealed.contains(&key) {
                            Err(ModelError::Invalid("sealed delta is immutable".into()))
                        } else {
                            stored
                                .charge
                                .grow(batch.arrow().get_array_memory_size())
                                .map(|()| {
                                    stored
                                        .deltas
                                        .entry(key)
                                        .or_default()
                                        .push(batch.arrow().clone())
                                })
                        }
                    }
                    Err(_) => Err(ModelError::Invalid("memory state poisoned".into())),
                }
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

/// Canonical vocabulary deduplication compares complete semantic rows, never just their IDs.
fn unique(relation: &Relation, batch: &RecordBatch) -> Result<RecordBatch, ModelError> {
    let sorted = order(relation, batch, &["id"])?;
    let converter = RowConverter::new(
        sorted
            .schema()
            .fields()
            .iter()
            .map(|f| SortField::new(f.data_type().clone()))
            .collect(),
    )
    .map_err(ModelError::codec)?;
    let rows = converter
        .convert_columns(sorted.columns())
        .map_err(ModelError::codec)?;
    let ids = sorted
        .column_by_name("id")
        .expect("domain id")
        .as_any()
        .downcast_ref::<FixedSizeBinaryArray>()
        .expect("domain id width");
    let mut indices = Vec::new();
    for index in 0..sorted.num_rows() {
        if index > 0 && ids.value(index) == ids.value(index - 1) {
            if rows.row(index) != rows.row(index - 1) {
                return Err(ModelError::Conflict(relation.name()));
            }
        } else {
            indices.push(u32::try_from(index).map_err(ModelError::codec)?);
        }
    }
    arrow_select::take::take_record_batch(&sorted, &UInt32Array::from(indices))
        .map_err(ModelError::codec)
}
