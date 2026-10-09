//! Selected SourceCall dependency grains and private attempt-owned result payloads.
use lctx_model::domain::{
    analysis::source_call::AnalysisInvocation, normalized::events::NormalizedCallEvent, *,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::Mutex,
};
#[derive(Clone, Copy)]
struct Slot {
    offset: u64,
    length: usize,
}
impl HeapSize for Slot {
    fn heap_bytes(&self) -> usize {
        0
    }
}
/// Anonymous bytes are a private compiler physical resource. The actual model issuer verifies
/// their immutable frame/event digest before accepting any rehydrated private value.
pub(super) struct SourcePayloadSpool {
    file: Option<Mutex<File>>,
    slots: charged::ChargedMap<(Id<AnalysisInvocation>, Id<NormalizedCallEvent>), Slot>,
    charge: charged::StateCharge,
}
impl SourcePayloadSpool {
    pub(super) fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            file: None,
            slots: Default::default(),
            charge: charged::StateCharge::new(budget, "source-call-private-payload-index"),
        }
    }
    pub(super) fn insert(
        &mut self,
        frame: Id<AnalysisInvocation>,
        event: Id<NormalizedCallEvent>,
        bytes: &[u8],
    ) -> Result<(), ModelError> {
        if self.slots.contains_key(&(frame, event)) {
            return Err(ModelError::Conflict(
                "SourceCall private payload duplicated",
            ));
        }
        if self.file.is_none() {
            self.file = Some(Mutex::new(tempfile::tempfile().map_err(ModelError::codec)?));
        }
        let file = self
            .file
            .as_mut()
            .expect("opened private spool")
            .get_mut()
            .map_err(|_| ModelError::Conflict("SourceCall private spool lock poisoned"))?;
        let offset = file.seek(SeekFrom::End(0)).map_err(ModelError::codec)?;
        offset
            .checked_add(u64::try_from(bytes.len()).map_err(ModelError::codec)?)
            .ok_or(ModelError::Conflict(
                "SourceCall private payload offset overflow",
            ))?;
        file.write_all(bytes).map_err(ModelError::codec)?;
        self.slots.insert(
            &mut self.charge,
            (frame, event),
            Slot {
                offset,
                length: bytes.len(),
            },
        )?;
        Ok(())
    }
    pub(super) fn read(
        &self,
        frame: Id<AnalysisInvocation>,
        event: Id<NormalizedCallEvent>,
        budget: &resources::ResourceBudget,
    ) -> Result<(Vec<u8>, Box<dyn resources::Reservation>), ModelError> {
        if !self
            .charge
            .budget()
            .expect("bound spool")
            .shares_pool(budget)
        {
            return Err(ModelError::Conflict(
                "SourceCall private spool budget changed",
            ));
        }
        let slot = self
            .slots
            .get(&(frame, event))
            .ok_or(ModelError::Conflict("SourceCall private payload absent"))?;
        let reservation = budget.reserve("source-call-private-payload-read", slot.length)?;
        let mut bytes = vec![0; slot.length];
        let mut file = self
            .file
            .as_ref()
            .ok_or(ModelError::Conflict("SourceCall private spool absent"))?
            .lock()
            .map_err(|_| ModelError::Conflict("SourceCall private spool lock poisoned"))?;
        file.seek(SeekFrom::Start(slot.offset))
            .map_err(ModelError::codec)?;
        file.read_exact(&mut bytes).map_err(ModelError::codec)?;
        Ok((bytes, reservation))
    }
}

use super::execution_scope::nominal;
#[cfg(test)]
use super::execution_scope::predicate;
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure, PreparedEdges},
    workspace::CompletedInputs,
};
#[cfg(test)]
use futures::TryStreamExt;
use lctx_model::domain::{
    execution::source_call_records::SourceCallData,
    normalized::{entities::EntityRef, events::NormalizedCallAlternative},
    source::Occurrence,
    syntax::SyntaxPlacement,
};
#[cfg(test)]
use lctx_model::domain::{lexical::*, normalized::entities::*, source::*, syntax::*};
use std::{any::TypeId, sync::Arc};
pub(super) struct SelectionSql {
    sql: String,
    _charge: Box<dyn resources::Reservation>,
}
impl std::ops::Deref for SelectionSql {
    type Target = str;
    fn deref(&self) -> &str {
        &self.sql
    }
}
struct SourceSelectors {
    events: source_call_scope_program::CompiledSelection,
    owners: Option<source_call_scope_program::CompiledSelection>,
    statements: Option<source_call_scope_program::CompiledSelection>,
    docstrings: source_call_scope_program::CompiledSelection,
}
impl SourceSelectors {
    fn new(
        inputs: &[ValidationInput],
        tables: &[ClosureTable],
        model: &ValidatedModel,
        enriched: bool,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let _construction =
            budget.reserve("source-selector-construction", inputs.len() * 2048 + 4096)?;
        let input_relations = tables
            .iter()
            .map(|t| t.relation.clone())
            .collect::<Vec<_>>();
        let root = |kind| {
            source_call_scope_program::root_inventory(
                inputs.to_vec(),
                &input_relations,
                kind,
                budget,
            )?
            .compile(model, budget, None)
        };
        use source_call_scope_program::RootInventory as R;
        Ok(Self {
            events: root(R::Events)?,
            owners: if enriched {
                Some(root(R::Owners)?)
            } else {
                None
            },
            statements: if enriched {
                Some(root(R::UnownedStatements)?)
            } else {
                None
            },
            docstrings: source_call_scope_program::docstring_payloads(
                inputs.to_vec(),
                &input_relations,
                budget,
            )?
            .compile(model, budget, None)?,
        })
    }
}
/// Bind physical aliases and the selected grain to the model's typed set DAG.
fn selection_sql(
    selector: &source_call_scope_program::CompiledSelection,
    tables: &[ClosureTable],
    parameters: &scope_program::ScopeParameters,
    scope: Option<&PreparedClosure>,
    budget: &resources::ResourceBudget,
) -> Result<SelectionSql, ModelError> {
    use source_call_scope_program::SelectionNode as N;
    let program = selector.program().program();
    let charge = budget.reserve(
        "source-selector-query",
        crate::scope_compilation::lowering_allowance(program, tables, parameters)
            .saturating_mul(2)
            .saturating_add(program.allowance())
            .saturating_add(selector.nodes().len() * 4096),
    )?;
    let mut clauses = Vec::new();
    for (i, rule) in program.rules.iter().enumerate() {
        let single = scope_program::ScopeProgram {
            inputs: program.inputs.clone(),
            ports: program.ports.clone(),
            rules: vec![rule.clone()],
        };
        let queries = crate::scope_compilation::select_pair_queries(&single, tables, parameters)?;
        let query = queries
            .first()
            .ok_or(ModelError::Schema("source selector pair query absent"))?;
        clauses.push(format!("q{i} AS ({})", query.2));
    }
    for (i, node) in selector.nodes().iter().enumerate() {
        let sql = match *node {
            N::Query(q) => format!("SELECT DISTINCT target_id AS id FROM q{q}"),
            N::SelectedInput(input) => format!(
                "SELECT id FROM ({}) selected",
                scope
                    .ok_or(ModelError::Schema("source selected grain absent"))?
                    .select(input)?
            ),
            N::Union(a, b) => format!("SELECT id FROM n{a} UNION SELECT id FROM n{b}"),
            N::Intersection(a, b) => format!("SELECT id FROM n{a} INTERSECT SELECT id FROM n{b}"),
            N::Difference(a, b) => format!("SELECT id FROM n{a} EXCEPT SELECT id FROM n{b}"),
            N::FilterSources { query, sources } => format!(
                "SELECT DISTINCT p.target_id AS id FROM q{query} p WHERE p.source_id IN (SELECT id FROM n{sources})"
            ),
        };
        clauses.push(format!("n{i} AS ({sql})"));
    }
    Ok(SelectionSql {
        sql: format!(
            "WITH {} SELECT id FROM n{} ORDER BY id",
            clauses.join(","),
            selector.nodes().len() - 1
        ),
        _charge: charge,
    })
}
pub(super) struct SourceCallScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    selectors: SourceSelectors,
    event: usize,
    owner: Option<usize>,
    statement: Option<usize>,
    _charge: charged::StateCharge,
}
/// Rich columns are read once for a bounded union; owning model kernels decode only each
/// exact partition. Opaque content identities are retained separately per logical grain.
pub(super) struct SourceCallWindow<'a> {
    scopes: &'a SourceCallScopes,
    batch: crate::consumed_rows::PreparedRootBatch,
    rows: Vec<Vec<arrow_array::RecordBatch>>,
    opaque: charged::ChargedSet<(usize, [u8; 16])>,
    _charge: charged::StateCharge,
}
impl SourceCallWindow<'_> {
    fn visit_partition(
        &self,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
        mut visit: impl FnMut(&ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for (table, input) in self.scopes.inputs.iter().enumerate() {
            for rows in &self.rows[table] {
                cancellation.check()?;
                let _filter = budget.reserve(
                    "source-columnar-partition",
                    rows.get_array_memory_size() + rows.num_rows() * 32 + 4096,
                )?;
                let mut mask = Vec::with_capacity(rows.num_rows());
                for row in 0..rows.num_rows() {
                    let key = crate::scoped_admission::column(rows, "id", row)?
                        .ok_or(ModelError::Schema("SourceCall union identity"))?;
                    mask.push(
                        self.batch.contains(partition, table, key)?
                            && !(input.type_id() == TypeId::of::<value::Literal>()
                                && self.opaque.contains(&(partition, key))),
                    );
                }
                let selected = datafusion::arrow::compute::filter_record_batch(
                    rows,
                    &arrow_array::BooleanArray::from(mask),
                )
                .map_err(ModelError::codec)?;
                if selected.num_rows() != 0 {
                    visit(input, &selected)?;
                }
            }
        }
        Ok(())
    }
    fn project(
        &self,
        partition: usize,
        data: &mut execution::evaluation::EvaluationData,
    ) -> Result<(), ModelError> {
        if partition >= self.batch.outcomes().len() {
            return Err(ModelError::Schema("SourceCall window partition"));
        }
        for (_, key) in self
            .opaque
            .range((partition, [0; 16])..=(partition, [255; 16]))
        {
            data.project_opaque_literal(
                nominal(key)?,
                source_call_scope_program::OPAQUE_DOCSTRING_KIND,
            )?;
        }
        Ok(())
    }
    pub(super) fn data(
        &self,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<SourceCallData, ModelError> {
        let mut data = SourceCallData::new(budget);
        self.project(partition, &mut data.evaluation)?;
        self.visit_partition(partition, budget, cancellation, |input, batch| {
            data.visit_input(input, batch)?;
            Ok(())
        })?;
        Ok(data)
    }
    pub(super) fn enriched_data(
        &self,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<execution::enriched_production::EnrichedData, ModelError> {
        let mut data = execution::enriched_production::EnrichedData::new(budget);
        self.project(partition, &mut data.source.evaluation)?;
        self.visit_partition(partition, budget, cancellation, |input, batch| {
            data.visit_input(input, batch)?;
            Ok(())
        })?;
        Ok(data)
    }
}
impl SourceCallScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = SourceCallData::inputs();
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema(input.name()))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget, model).await
    }
    pub(super) async fn prepare_enriched(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = execution::enriched_production::EnrichedData::inputs();
        let tables = inputs
            .iter()
            .map(|input| {
                Ok(ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema(input.name()))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::prepare_bound(inputs, tables, session, budget, model).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let index = |kind: TypeId| {
            inputs
                .iter()
                .position(|input| input.type_id() == kind)
                .ok_or(ModelError::Schema("SourceCall dependency input"))
        };
        let event = index(TypeId::of::<NormalizedCallEvent>())?;
        let occurrence = index(TypeId::of::<Occurrence>())?;
        let enriched = inputs.iter().any(|input| {
            input.type_id() == TypeId::of::<execution::source_call_records::SourceCallHeader>()
        });
        let private_sources = [
            index(TypeId::of::<NormalizedCallAlternative>())?,
            occurrence,
            occurrence,
            index(TypeId::of::<SyntaxPlacement>())?,
            if enriched {
                index(TypeId::of::<EntityRef>())?
            } else {
                occurrence
            },
            occurrence,
        ];
        let private_count = if enriched { 6 } else { 4 };
        // Bindings remain owned after the raw factory is disposed. Reserve their retained
        // slots, order arrays and private alias/field copies before constructing those copies.
        let mut charge = charged::StateCharge::new(budget, "SourceCall-scope-descriptors");
        let additional = tables
            .len()
            .saturating_add(private_count)
            .saturating_sub(tables.capacity());
        charge.grow(
            inputs
                .capacity()
                .saturating_mul(size_of::<ValidationInput>())
                .saturating_add(
                    inputs
                        .iter()
                        .map(|input| size_of_val(input.order()))
                        .sum::<usize>(),
                )
                .saturating_add(
                    tables
                        .capacity()
                        .saturating_add(additional)
                        .saturating_mul(size_of::<ClosureTable>()),
                )
                .saturating_add(
                    tables
                        .iter()
                        .map(|table| {
                            table
                                .alias
                                .capacity()
                                .saturating_add(size_of_val(table.relation.fields()))
                        })
                        .sum::<usize>(),
                )
                .saturating_add(
                    private_sources[..private_count]
                        .iter()
                        .map(|source| {
                            tables[*source]
                                .alias
                                .capacity()
                                .saturating_add(size_of_val(tables[*source].relation.fields()))
                        })
                        .sum::<usize>(),
                ),
        )?;
        tables.reserve_exact(private_count);
        let callee = tables.len();
        tables.push(tables[private_sources[0]].clone());
        let header = tables.len();
        tables.push(tables[occurrence].clone());
        let payload = tables.len();
        tables.push(tables[occurrence].clone());
        let children = tables.len();
        tables.push(tables[private_sources[3]].clone());
        let (owner, statement) = if enriched {
            let owner = tables.len();
            tables.push(tables[private_sources[4]].clone());
            let statement = tables.len();
            tables.push(tables[occurrence].clone());
            (Some(owner), Some(statement))
        } else {
            (None, None)
        };
        let mut declarations = inputs.clone();
        for table in &tables[inputs.len()..] {
            let original = inputs
                .iter()
                .position(|input| input.type_id() == table.relation.type_id())
                .ok_or(ModelError::Schema("SourceCall virtual input"))?;
            declarations.push(inputs[original].clone());
        }
        let input_relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = source_call_scope_program::build(
            declarations,
            &input_relations,
            inputs.len(),
            source_call_scope_program::SourceCallPorts {
                callee,
                header,
                payload,
                children,
                owner,
                statement,
            },
            budget,
        )?;
        let compiled = crate::scope_compilation::compile(program.program(), model, budget, None)?;
        // Compilation owns the exact AST/canonical frame. Dispose construction scratch before
        // allocating physical fallback queries rather than retaining two preparation arenas.
        drop(program);
        drop(input_relations);
        let plan = crate::scope_compilation::lower_compiled(
            compiled,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        // PreparedEdges retains the compiled owner and compact topology. Its discovery does
        // not need the original lowering strings when independent selectors are compiled.
        drop(plan);
        let selectors =
            SourceSelectors::new(&inputs, &tables[..inputs.len()], model, enriched, budget)?;
        Ok(Self {
            inputs,
            tables,
            edges,
            selectors,
            event,
            owner,
            statement,
            _charge: charge,
        })
    }
    pub(super) async fn event_window<'a>(
        &'a self,
        keys: &[[u8; 16]],
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<SourceCallWindow<'a>, ModelError> {
        self.window(
            self.event,
            crate::consumed_rows::PreparedRootKind::Physical,
            keys,
            budget,
            cancellation,
        )
        .await
    }
    pub(super) async fn owner_window<'a>(
        &'a self,
        keys: &[[u8; 16]],
        unowned: bool,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<SourceCallWindow<'a>, ModelError> {
        let root = if unowned { self.statement } else { self.owner }
            .ok_or(ModelError::Schema("Enriched root namespace"))?;
        self.window(
            root,
            crate::consumed_rows::PreparedRootKind::Virtual,
            keys,
            budget,
            cancellation,
        )
        .await
    }
    async fn window<'a>(
        &'a self,
        root: usize,
        kind: crate::consumed_rows::PreparedRootKind,
        keys: &[[u8; 16]],
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<SourceCallWindow<'a>, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "source-columnar-window");
        charge.grow(
            keys.len() * size_of::<crate::consumed_rows::PreparedRoot>()
                + self.inputs.len() * size_of::<Vec<arrow_array::RecordBatch>>()
                + 4096,
        )?;
        let roots = keys
            .iter()
            .map(|key| crate::consumed_rows::PreparedRoot {
                table: root,
                key: *key,
                kind,
            })
            .collect::<Vec<_>>();
        let batch = self
            .edges
            .batch_with_cancellation(&roots, budget, cancellation)
            .await?;
        drop(roots);
        let mut opaque = charged::ChargedSet::default();
        for partition in 0..keys.len() {
            cancellation.check()?;
            let scope = batch.partition_scope(partition, budget)?;
            let selected = selection_sql(
                &self.selectors.docstrings,
                &self.tables,
                &scope_program::ScopeParameters(vec![]),
                Some(&scope),
                budget,
            )?;
            let input = &self.inputs[self.index(TypeId::of::<value::Literal>())?];
            crate::consumed_rows::stream_batches(
                input,
                scope.session(),
                &selected,
                None,
                &mut |rows| {
                    cancellation.check()?;
                    let _transfer = budget
                        .reserve("source-opaque-kind-transfer", rows.get_array_memory_size())?;
                    for row in 0..rows.num_rows() {
                        let key = crate::scoped_admission::column(rows, "id", row)?
                            .ok_or(ModelError::Schema("SourceCall opaque identity"))?;
                        opaque.insert(&mut charge, (partition, key))?;
                    }
                    Ok(())
                },
            )
            .await?;
        }
        let literals = self.index(TypeId::of::<value::Literal>())?;
        let mut literal_keys = charged::ChargedSet::default();
        for partition in 0..keys.len() {
            for key in batch.keys(partition, literals)? {
                if !opaque.contains(&(partition, key)) {
                    literal_keys.insert(&mut charge, key)?;
                }
            }
        }
        charge.grow(literal_keys.len() * 96 + 4096)?;
        let literal_filter = if literal_keys.is_empty() {
            "FALSE".into()
        } else {
            format!(
                "id IN ({})",
                literal_keys
                    .iter()
                    .map(|key| format!("X'{}'", hex::encode(key)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let mut rows = (0..self.inputs.len())
            .map(|_| Vec::new())
            .collect::<Vec<_>>();
        for (table, input) in self.inputs.iter().enumerate() {
            cancellation.check()?;
            let selected = batch.union.select(table)?;
            let selected = if table == literals {
                format!("SELECT * FROM ({selected}) literals WHERE {literal_filter}")
            } else {
                selected
            };
            crate::consumed_rows::stream_batches(
                input,
                batch.union.session(),
                &selected,
                None,
                &mut |data| {
                    cancellation.check()?;
                    let target = &mut rows[table];
                    let additional = if target.len() == target.capacity() {
                        target.capacity().max(4)
                    } else {
                        0
                    };
                    charge.grow(
                        data.get_array_memory_size()
                            + additional * size_of::<arrow_array::RecordBatch>(),
                    )?;
                    target.reserve_exact(additional);
                    target.push(data.clone());
                    Ok(())
                },
            )
            .await?;
        }
        Ok(SourceCallWindow {
            scopes: self,
            batch,
            rows,
            opaque,
            _charge: charge,
        })
    }
    fn index(&self, kind: TypeId) -> Result<usize, ModelError> {
        self.inputs
            .iter()
            .position(|input| input.type_id() == kind)
            .ok_or(ModelError::Schema("SourceCall dependency type"))
    }
    pub(super) fn roots(&self, inv: &AnalysisInvocation) -> Result<SelectionSql, ModelError> {
        selection_sql(
            &self.selectors.events,
            &self.tables,
            &scope_program::ScopeParameters(vec![
                scope_program::ScopeValue::Nominal(*inv.input.bytes()),
                scope_program::ScopeValue::Nominal(*inv.context.bytes()),
            ]),
            None,
            self._charge.budget().expect("scope budget"),
        )
    }
    #[cfg(test)]
    pub(super) async fn scope(
        &self,
        event: Id<NormalizedCallEvent>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.edges
            .grain(self.event, &predicate(event), budget)
            .await
    }
    pub(super) fn owner_roots(
        &self,
        inv: &analysis::enriched_execution::AnalysisInvocation,
        unowned: bool,
    ) -> Result<SelectionSql, ModelError> {
        let selector = if unowned {
            self.selectors.statements.as_ref()
        } else {
            self.selectors.owners.as_ref()
        }
        .ok_or(ModelError::Schema("Enriched root selector"))?;
        selection_sql(
            selector,
            &self.tables,
            &scope_program::ScopeParameters(vec![
                scope_program::ScopeValue::Nominal(*inv.input.bytes()),
                scope_program::ScopeValue::Nominal(*inv.context.bytes()),
            ]),
            None,
            self._charge.budget().expect("scope budget"),
        )
    }
    #[cfg(test)]
    pub(super) async fn owner_scope(
        &self,
        owner: Id<EntityRef>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.edges
            .grain(
                self.owner
                    .ok_or(ModelError::Schema("Enriched owner namespace"))?,
                &predicate(owner),
                budget,
            )
            .await
    }
    #[cfg(test)]
    async fn docstring_literals(
        &self,
        scope: &PreparedClosure,
        sql: &str,
        data: &mut execution::evaluation::EvaluationData,
        budget: &resources::ResourceBudget,
    ) -> Result<String, ModelError> {
        let opaque = selection_sql(
            &self.selectors.docstrings,
            &self.tables,
            &scope_program::ScopeParameters(vec![]),
            Some(scope),
            budget,
        )?;
        let projection = format!(
            "SELECT l.id,l.kind FROM ({sql}) l WHERE l.id IN ({opaque}) ORDER BY l.id",
            opaque = &*opaque
        );
        let mut stream = crate::sql::query(scope.session(), &projection)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "execution-docstring-kind-transfer",
                logical_batch_bytes(&batch)?,
            )?;
            for row in 0..batch.num_rows() {
                let key = crate::scoped_admission::column(&batch, "id", row)?
                    .ok_or(ModelError::Schema("docstring literal projection ID"))?;
                data.project_opaque_literal(
                    nominal(&key)?,
                    source_call_scope_program::OPAQUE_DOCSTRING_KIND,
                )?;
            }
            tokio::task::yield_now().await;
        }
        Ok(format!(
            "SELECT l.* FROM ({sql}) l WHERE l.id NOT IN ({opaque})",
            opaque = &*opaque
        ))
    }
    #[cfg(test)]
    pub(super) async fn enriched_data(
        &self,
        scope: &PreparedClosure,
        budget: &resources::ResourceBudget,
    ) -> Result<execution::enriched_production::EnrichedData, ModelError> {
        let mut data = execution::enriched_production::EnrichedData::new(budget);
        for (table, input) in self.inputs.iter().enumerate() {
            let sql = scope.select(table)?;
            let sql = if input.type_id() == TypeId::of::<value::Literal>() {
                self.docstring_literals(scope, &sql, &mut data.source.evaluation, budget)
                    .await?
            } else {
                sql
            };
            let mut stream = crate::sql::query(scope.session(), &sql)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "Enriched-selected-input-transfer",
                    lctx_model::domain::logical_batch_bytes(&batch)?,
                )?;
                data.visit_input(input, &batch)?;
                tokio::task::yield_now().await;
            }
        }
        Ok(data)
    }
}

/// Share only the immutable configuration/frame domain; no source, proof or output row is
/// copied from an earlier rich kernel. Rows validates duplicate identities mechanically.
pub(super) fn enriched_configuration(
    from: &execution::enriched_production::EnrichedData,
    to: &mut execution::enriched_production::EnrichedData,
) -> Result<(), ModelError> {
    for row in from.parameters.iter() {
        to.parameters.insert(row.clone())?;
    }
    for row in from.catalogs.iter() {
        to.catalogs.insert(row.clone())?;
    }
    for row in from.source_invocations.iter() {
        to.source_invocations.insert(row.clone())?;
    }
    for row in from.source.definitions.iter() {
        to.source.definitions.insert(row.clone())?;
    }
    Ok(())
}
pub(super) fn hydrate_selected(
    values: &execution::source_call_records::ProducedSourceCalls,
    spool: &SourcePayloadSpool,
    parent: &AnalysisInvocation,
    data: &execution::enriched_production::EnrichedData,
    budget: &resources::ResourceBudget,
) -> Result<execution::source_call_records::HydratedSourceCalls, ModelError> {
    let mut hydrated = values.hydrate_empty(parent, budget)?;
    let mut events = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(budget, "Enriched-selected-private-events");
    for header in data
        .source_headers
        .iter()
        .filter(|row| row.invocation == parent.id())
    {
        if events.insert(&mut charge, header.event)? {
            let (bytes, _read) = spool.read(parent.id(), header.event, budget)?;
            hydrated.append(values.hydrate(parent, header.event, &bytes, budget)?)?;
        }
    }
    Ok(hydrated)
}

#[cfg(test)]
mod controls {
    use super::*;
    use assertion::*;
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    fn id<R>(n: u8) -> Id<R> {
        nominal(&[n; 16]).unwrap()
    }
    fn register<R: Record>(session: &SessionContext, rows: &[R]) {
        let batch = <R as Record>::encode(rows).unwrap();
        session.deregister_table(R::NAME).unwrap();
        session
            .register_table(
                R::NAME,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    async fn prepare(
        session: &SessionContext,
        budget: &resources::ResourceBudget,
        enriched: bool,
    ) -> SourceCallScopes {
        let inputs = if enriched {
            execution::enriched_production::EnrichedData::inputs()
        } else {
            SourceCallData::inputs()
        };
        let model = model().unwrap();
        for input in &inputs {
            let schema = model.relation(input.name()).unwrap().schema().clone();
            session.deregister_table(input.name()).unwrap();
            session
                .register_table(
                    input.name(),
                    Arc::new(
                        MemTable::try_new(
                            schema.clone(),
                            vec![vec![arrow_array::RecordBatch::new_empty(schema)]],
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
        }
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: input.name().into(),
            })
            .collect();
        SourceCallScopes::prepare_bound(inputs, tables, session, budget, &model)
            .await
            .unwrap()
    }
    // Preparation is repeated after fixture writes because the edge index is immutable.
    async fn bound(
        session: &SessionContext,
        budget: &resources::ResourceBudget,
        enriched: bool,
    ) -> SourceCallScopes {
        let inputs = if enriched {
            execution::enriched_production::EnrichedData::inputs()
        } else {
            SourceCallData::inputs()
        };
        let model = model().unwrap();
        let tables = inputs
            .iter()
            .map(|input| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: input.name().into(),
            })
            .collect();
        SourceCallScopes::prepare_bound(inputs, tables, session, budget, &model)
            .await
            .unwrap()
    }
    fn fixture(
        session: &SessionContext,
    ) -> (
        NormalizedCallEvent,
        EntityRef,
        Occurrence,
        [NormalizedCallAlternative; 2],
        value::Literal,
        Occurrence,
    ) {
        let artifact =
            SourceArtifact::from_bytes(id(1), "scope.py".into(), b"def caller():\n  pass\n")
                .unwrap();
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: id(2),
            scope: scope.id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: Approximation::Exact,
        };
        let caller = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 100,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Declaration,
            structural_path: vec![0],
        };
        let call = Occurrence {
            start: 80,
            end: 90,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 3],
            ..caller.clone()
        };
        let caller_entity = CallableEntity::Source {
            declaration: caller.id(),
            kind: CallableKind::Function,
        };
        let owner = EntityRef::Callable {
            callable: caller_entity.id(),
        };
        let membership = OccurrenceOwnership {
            occurrence: call.id(),
            owner: caller.id(),
            entity: owner.id(),
        };
        let rich = Occurrence {
            start: 110,
            end: 120,
            syntax_kind: SyntaxKind::StmtPass,
            structural_path: vec![9; 2_000_000],
            ..caller.clone()
        };
        let literal = value::Literal::String {
            value: "λ".repeat(3 << 20).into(),
        };
        let doc = Occurrence {
            start: 5,
            end: 15,
            syntax_kind: SyntaxKind::ExprStringLiteral,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 0, 0],
            ..caller.clone()
        };
        let doc_stmt = Occurrence {
            start: 4,
            end: 16,
            syntax_kind: SyntaxKind::StmtExpr,
            structural_path: vec![0, 0],
            ..doc.clone()
        };
        let detail = SyntaxDetail::Literal {
            literal: literal.id(),
        };
        let observation = SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: doc.id(),
            ordinal: 0,
            detail: detail.id(),
        };
        let event = NormalizedCallEvent {
            site: call.id(),
            origin: id(8),
            context: id(2),
            owner: membership.id(),
        };
        let mut callees = Vec::new();
        let mut entities = vec![caller_entity];
        let mut references = vec![owner.clone()];
        let mut occurrences = vec![
            caller.clone(),
            call.clone(),
            doc.clone(),
            doc_stmt.clone(),
            rich.clone(),
        ];
        let mut declarations = vec![DeclarationObservation {
            qualification: q.id(),
            declaration: caller.id(),
            name: call.id(),
            kind: DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: Some(doc.id()),
        }];
        for n in [1, 2] {
            let declaration = Occurrence {
                start: 20 * n,
                end: 20 * n + 10,
                structural_path: vec![0, n as i32],
                ..caller.clone()
            };
            let callable = CallableEntity::Source {
                declaration: declaration.id(),
                kind: CallableKind::Function,
            };
            let reference = EntityRef::Callable {
                callable: callable.id(),
            };
            callees.push(NormalizedCallAlternative {
                event: event.id(),
                source: id(n as u8 + 10),
                resolution: None,
                correspondence: None,
                entity: Some(reference.id()),
                status: ResolutionStatus::Resolved,
                reason: normalized::links::LinkReason::ExplicitIdentity,
            });
            entities.push(callable);
            references.push(reference);
            occurrences.push(declaration.clone());
            declarations.push(DeclarationObservation {
                qualification: q.id(),
                declaration: declaration.id(),
                name: call.id(),
                kind: DeclarationKind::Function,
                parent: Some(caller.id()),
                overload: false,
                docstring: None,
            });
        }
        let CallableEntity::Source {
            declaration: callee,
            ..
        } = &entities[1]
        else {
            unreachable!("source callee fixture");
        };
        let later_return = Occurrence {
            start: 24,
            end: 29,
            syntax_kind: SyntaxKind::StmtReturn,
            structural_path: vec![0, 1, 1],
            ..caller.clone()
        };
        let returned_value = Occurrence {
            start: 27,
            end: 28,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0, 1, 1, 0],
            ..caller.clone()
        };
        occurrences.extend([later_return.clone(), returned_value.clone()]);
        register(
            session,
            &[execution::source_call_records::SourceCallHeader {
                invocation: id(13),
                event: event.id(),
                attempt: id(14),
                owner: owner.id(),
                callee: callees[0].entity.unwrap(),
                declaration: *callee,
                qualification: q.id(),
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                premises: ContentHash::of(b"header-scope-control"),
            }],
        );
        register(session, std::slice::from_ref(&artifact));
        register(
            session,
            &[input::ArtifactUse {
                artifact: artifact.id(),
                input: artifact.input,
                role: input::SourceRole::Release,
            }],
        );
        register(session, &[scope]);
        register(session, std::slice::from_ref(&q));
        register(session, &occurrences);
        register(session, &entities);
        register(session, &references);
        register(session, &[membership]);
        register(session, std::slice::from_ref(&event));
        register(session, &callees);
        register(session, &declarations);
        register(
            session,
            &[
                SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: call.id(),
                    parent: Some(caller.id()),
                    field: SyntaxField::Body,
                    ordinal: 3,
                },
                SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: doc_stmt.id(),
                    parent: Some(caller.id()),
                    field: SyntaxField::Body,
                    ordinal: 0,
                },
                SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: doc.id(),
                    parent: Some(doc_stmt.id()),
                    field: SyntaxField::Value,
                    ordinal: 0,
                },
                SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: later_return.id(),
                    parent: Some(*callee),
                    field: SyntaxField::Body,
                    ordinal: 1,
                },
                SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: returned_value.id(),
                    parent: Some(later_return.id()),
                    field: SyntaxField::Value,
                    ordinal: 0,
                },
            ],
        );
        register(session, std::slice::from_ref(&literal));
        register(session, &[detail]);
        register(session, &[observation]);
        (
            event,
            owner,
            rich,
            callees.try_into().unwrap(),
            literal,
            returned_value,
        )
    }
    #[tokio::test]
    async fn event_and_owner_closures_keep_all_alternatives_and_project_unused_docstring() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        drop(prepare(&session, &budget, true).await);
        let (event, owner, rich, alternatives, literal, returned_value) = fixture(&session);
        drop(literal);
        let prepared = bound(&session, &budget, true).await;
        let retained = budget.reserved();
        for scope in [
            prepared.scope(event.id(), &budget).await.unwrap(),
            prepared.owner_scope(owner.id(), &budget).await.unwrap(),
        ] {
            let data = prepared.enriched_data(&scope, &budget).await.unwrap();
            for alternative in &alternatives {
                assert_eq!(
                    data.source
                        .bindings
                        .event_alternatives
                        .get(alternative.id()),
                    Some(alternative)
                );
            }
            assert_eq!(
                data.source.evaluation.occurrences.get(returned_value.id()),
                Some(&returned_value),
                "an admitted header needs value children of every callee Body statement",
            );
            assert!(data.source.evaluation.occurrences.get(rich.id()).is_none());
            assert!(
                data.source.evaluation.literals.is_empty(),
                "unused rich docstring bytes must not be decoded"
            );
            drop(data);
            drop(scope);
        }
        {
            let cancelled = crate::workspace::Cancellation::default();
            let keys = [*event.id().bytes(), *event.id().bytes(), [255; 16]];
            let window = prepared
                .event_window(&keys, &budget, &cancelled)
                .await
                .unwrap();
            for partition in 0..2 {
                let selected = window
                    .enriched_data(partition, &budget, &cancelled)
                    .unwrap();
                for alternative in &alternatives {
                    assert_eq!(
                        selected
                            .source
                            .bindings
                            .event_alternatives
                            .get(alternative.id()),
                        Some(alternative)
                    );
                }
                assert!(selected.source.evaluation.literals.is_empty());
                assert!(
                    selected
                        .source
                        .evaluation
                        .occurrences
                        .get(rich.id())
                        .is_none()
                );
            }
            let absent = window.enriched_data(2, &budget, &cancelled).unwrap();
            assert!(absent.source.bindings.event_alternatives.is_empty());
            assert!(window.enriched_data(3, &budget, &cancelled).is_err());
        }
        assert_eq!(budget.reserved(), retained);
        drop(prepared);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn empty_root_stream_and_private_spool_preserve_exact_issuer_and_release_resources() {
        let session = SessionContext::new();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let prepared = prepare(&session, &budget, false).await;
        let definition = execution::configuration::source_calls().1;
        let frame = AnalysisInvocation::new(id(1), id(2), definition.id(), None, []).0;
        let mut stream = crate::sql::query(&session, &prepared.roots(&frame).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            assert_eq!(batch.num_rows(), 0);
        }
        drop(stream);
        drop(prepared);
        let data = SourceCallData::new(&budget);
        let (_, issuer, payload) = execution::source_call_records::prepare_event_produced(
            &data,
            &frame,
            &definition,
            stages::Profile::Catalog,
            &budget,
            None,
            None,
            None,
            id(3),
        )
        .unwrap();
        let mut spool = SourcePayloadSpool::new(&budget);
        spool.insert(frame.id(), id(3), payload.bytes()).unwrap();
        assert!(spool.insert(frame.id(), id(3), payload.bytes()).is_err());
        drop(payload);
        let retained = budget.reserved();
        let (bytes, reservation) = spool.read(frame.id(), id(3), &budget).unwrap();
        let hydrated = issuer.hydrate(&frame, id(3), &bytes, &budget).unwrap();
        assert!(issuer.hydrate(&frame, id(4), &bytes, &budget).is_err());
        assert!(spool.read(frame.id(), id(4), &budget).is_err());
        let foreign = resources::ResourceBudget::fixed(4 << 20).unwrap();
        assert!(spool.read(frame.id(), id(3), &foreign).is_err());
        drop(hydrated);
        drop(bytes);
        drop(reservation);
        assert_eq!(budget.reserved(), retained);
        drop(spool);
        drop(issuer);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
