//! Explicit publication roots over one immutable externally prepared nominal edge projection.
use crate::{
    consumed_rows::{ClosureTable, PreparedClosure},
    normalize::call_scope::{CallScopeMode, CallScopes},
    workspace::CompletedInputs,
};
use futures::future::BoxFuture;
use lctx_model::domain::{
    execution::model_production::{ModelData, ProductionScope, SelectedCatalog},
    *,
};
use std::{any::TypeId, sync::Arc};
pub(super) struct ModelScopes {
    catalog_selection: String,
    _catalog_program: Arc<scope_program::CompiledScopeProgram>,
    inventory_tables: Vec<ClosureTable>,
    inventories: Vec<Arc<scope_program::CompiledScopeProgram>>,
    _charge: charged::StateCharge,
    calls: CallScopes,
    frame: usize,
    targets: Vec<(Id<models::AuthoredModel>, usize)>,
    contexts: usize,
    terminals: usize,
    exits: usize,
}
pub(super) fn hex<R: Record>(id: Id<R>) -> String {
    format!(
        "X'{}'",
        id.bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
macro_rules! model_decoder_inputs {($read:ident)=>{
            lctx_model::normalized_binding_inputs!($read);
            lctx_model::normalized_binding_outputs!($read);
            lctx_model::model_pin_inputs!($read);
            lctx_model::execution_evaluation_inputs!($read);
            lctx_model::entry_value_inputs!($read);
            $read! {catalogs:models::ModelCatalog,parameters:analysis::MethodParameters,definitions:analysis::AnalysisDefinition,enriched:analysis::enriched_execution::AnalysisInvocation,source_calls:analysis::source_call::AnalysisInvocation,local:analysis::local::AnalysisInvocation,runs:attribution::ProviderRun,members:class_metadata::ClassMemberObservation,metadata:class_metadata::ClassMetadataObservation,origins:calls::CallOrigin,origin_steps:calls::CallOriginStep,terminals:protocols::NativeTerminalObservation,exits:protocols::NativeExitObservation,literals:value::Literal,
            entries:conditions::entry::EntryValueWitness,entry_sources:conditions::entry::EntryAccessSource,evaluations:execution::records::ExpressionEvaluation,sources:execution::records::EvaluationSource,members:execution::records::EvaluationMember,operands:execution::records::EvaluationOperand,base:analysis::base_evaluation::AnalysisInvocation,bodies:execution::body_records::SourceBodyCompletion,body_frames:analysis::base_completion::AnalysisInvocation,headers:execution::source_call_records::SourceCallHeader,header_members:execution::source_call_records::HeaderMember,boundaries:execution::source_call_records::SourceCallBoundary,runs:execution::source_call_records::SourceCallRun,invocations:execution::source_call_records::SourceInvocation,releases:execution::source_call_records::SourceFrameRelease,arguments:execution::source_call_records::SourceFrameArgument,syntax:syntax::ParameterSyntaxObservation,outcomes:execution::source_call_records::SourceCallOutcome,boundaries:execution::source_call_records::InvocationBoundary,outcomes:analysis::source_call::AnalysisOutcome,modeled:execution::modeled_call::ModeledCallEvaluation,modeled_args:execution::modeled_call::ModeledCallArgument,modeled_native:execution::modeled_call::ModeledCallNative,bindings:execution::context_binding::ContextEntryBinding,binding_sources:execution::context_binding::BindingSource,binding_members:execution::context_binding::BindingMember,contexts:execution::context_execution::ContextExecution,items:execution::context_execution::ContextItem,sources:execution::context_execution::ContextSource,members:execution::context_execution::ContextMember,outcomes:execution::enriched_records::ExecutionOutcome,}
};}
struct ModelColumns {
    rows: Vec<Vec<arrow_array::RecordBatch>>,
    consumed: crate::consumed_rows::ConsumedInputs,
    charge: charged::StateCharge,
}
pub(super) struct ModelWindow<'a> {
    scopes: &'a ModelScopes,
    batch: crate::consumed_rows::PreparedRootBatch,
    columns: ModelColumns,
}
impl ModelWindow<'_> {
    pub(super) fn data(
        &self,
        partition: usize,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<ModelData, ModelError> {
        if partition >= self.batch.outcomes().len() {
            return Err(ModelError::Schema("Model window partition"));
        }
        let mut data = ModelData::new(budget);
        for (table, input) in self.scopes.calls.inputs().iter().enumerate() {
            for rows in &self.columns.rows[table] {
                cancellation.check()?;
                let _filter = budget.reserve(
                    "model-columnar-partition",
                    rows.get_array_memory_size() + rows.num_rows() * 32 + 4096,
                )?;
                let mut mask = Vec::with_capacity(rows.num_rows());
                for row in 0..rows.num_rows() {
                    let key = crate::scoped_admission::column(rows, "id", row)?
                        .ok_or(ModelError::Schema("Model union identity"))?;
                    mask.push(self.batch.contains(partition, table, key)?);
                }
                let selected = datafusion::arrow::compute::filter_record_batch(
                    rows,
                    &arrow_array::BooleanArray::from(mask),
                )
                .map_err(ModelError::codec)?;
                if selected.num_rows() != 0 {
                    data.visit_input(input, &selected)?;
                }
            }
        }
        Ok(data)
    }
}
type ColumnReader = for<'a> fn(
    &'a ModelScopes,
    &'a CompletedInputs,
    &'a PreparedClosure,
    &'a resources::ResourceBudget,
    &'a crate::workspace::Cancellation,
    &'a mut ModelColumns,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn read_columns<'a, R: Record>(
    scopes: &'a ModelScopes,
    access: &'a CompletedInputs,
    grain: &'a PreparedClosure,
    _budget: &'a resources::ResourceBudget,
    cancellation: &'a crate::workspace::Cancellation,
    columns: &'a mut ModelColumns,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = columns.consumed.next::<R>(access)? {
            cancellation.check()?;
            let table = scopes
                .calls
                .inputs()
                .iter()
                .position(|i| i.type_id() == input.type_id() && i.prefix() == input.prefix())
                .ok_or(ModelError::Schema("Model union immutable input"))?;
            let selected = scopes.select(grain, table, &input)?;
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                grain.session(),
                &selected,
                |_, batch| {
                    cancellation.check()?;
                    let target = &mut columns.rows[table];
                    let additional = if target.len() == target.capacity() {
                        target.capacity().max(4)
                    } else {
                        0
                    };
                    columns.charge.grow(
                        batch.get_array_memory_size()
                            + additional * size_of::<arrow_array::RecordBatch>(),
                    )?;
                    target.reserve_exact(additional);
                    target.push(batch.clone());
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
impl ModelScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        catalog: &SelectedCatalog,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = ModelData::inputs();
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
        Self::from_tables(inputs, tables, session, catalog, budget, model).await
    }
    async fn from_tables(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        catalog: &SelectedCatalog,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let mut _construction = compiler_scope_program::reserve_construction(
            inputs
                .len()
                .saturating_add(catalog.catalog().models().len().saturating_add(5)),
            tables
                .iter()
                .map(|table| table.relation.fields().len())
                .sum(),
            catalog.catalog().models().len(),
            budget,
        )?;
        let binding_bytes = tables
            .iter()
            .map(|table| table.alias.capacity())
            .sum::<usize>()
            .saturating_add(
                tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .max()
                    .unwrap_or(0)
                    .saturating_mul(catalog.catalog().models().len().saturating_add(6)),
            )
            .saturating_mul(4);
        let construction_bytes = _construction.size().saturating_add(binding_bytes);
        _construction.try_resize(construction_bytes)?;
        let real = tables.len();
        let index = |kind: TypeId| {
            tables[..real]
                .iter()
                .position(|t| t.relation.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("Model scope root absent".into()))
        };
        let frame_source = index(TypeId::of::<attribution::ProviderRun>())?;
        let context_source = index(TypeId::of::<execution::context_execution::ContextExecution>())?;
        let terminal_source = index(TypeId::of::<protocols::NativeTerminalObservation>())?;
        let exit_source = index(TypeId::of::<protocols::NativeExitObservation>())?;
        let frame = tables.len();
        tables.push(tables[frame_source].clone());
        let contexts = tables.len();
        tables.push(tables[context_source].clone());
        let terminals = tables.len();
        tables.push(tables[terminal_source].clone());
        let exits = tables.len();
        tables.push(tables[exit_source].clone());
        let mut targets = Vec::new();
        for compiled in catalog.catalog().models() {
            let root = tables.len();
            tables.push(tables[frame_source].clone());
            targets.push((compiled.declaration().id(), root));
        }
        let inventory_tables = tables[..real].to_vec();
        let records = inventory_tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let mut inventories = Vec::with_capacity(5);
        for kind in [
            compiler_scope_program::ModelRootInventory::Context,
            compiler_scope_program::ModelRootInventory::Event,
            compiler_scope_program::ModelRootInventory::Terminal,
            compiler_scope_program::ModelRootInventory::TerminalEvent,
            compiler_scope_program::ModelRootInventory::Exit,
        ] {
            let program =
                compiler_scope_program::model_roots(inputs.clone(), &records, kind, budget)?;
            inventories.push(crate::scope_compilation::compile(
                program.program(),
                model,
                budget,
                None,
            )?);
        }
        let actual_inputs = inputs.clone();
        let _parameter_copy = budget.reserve(
            "model-scope-parameter-copy",
            1024usize.saturating_add(
                catalog
                    .catalog()
                    .models()
                    .iter()
                    .map(|compiled| match &compiled.model().target {
                        models::Target::Stdlib {
                            module, callable, ..
                        }
                        | models::Target::Dependency {
                            module, callable, ..
                        }
                        | models::Target::Release { module, callable } => module
                            .len()
                            .saturating_add(callable.len())
                            .saturating_mul(2),
                    })
                    .sum::<usize>(),
            ),
        )?;
        let mut parameters =
            scope_program::ScopeParameters(vec![scope_program::ScopeValue::Nominal(
                *catalog.catalog().declaration().id().bytes(),
            )]);
        for compiled in catalog.catalog().models() {
            let (module, callable) = match &compiled.model().target {
                models::Target::Stdlib {
                    module, callable, ..
                }
                | models::Target::Dependency {
                    module, callable, ..
                }
                | models::Target::Release { module, callable } => (module, callable),
            };
            parameters
                .0
                .push(scope_program::ScopeValue::Text(module.clone()));
            parameters.0.push(scope_program::ScopeValue::Text(
                callable.rsplit('.').next().unwrap_or(callable).into(),
            ));
        }
        parameters.0.push(scope_program::ScopeValue::Nominal(
            *calls::CallOrigin::explicit().bytes(),
        ));
        let selection = compiler_scope_program::model_catalog(actual_inputs.clone())?;
        let catalog_program = crate::scope_compilation::compile(&selection, model, budget, None)?;
        let mut charge = charged::StateCharge::new(budget, "model-scope-descriptors");
        charge.grow(
            crate::scope_compilation::lowering_allowance(
                catalog_program.program(),
                &tables,
                &parameters,
            )
            .saturating_mul(4),
        )?;
        let queries = crate::scope_compilation::select_pair_queries(
            catalog_program.program(),
            &tables,
            &parameters,
        )?;
        let catalog_selection = queries
            .into_iter()
            .next()
            .ok_or(ModelError::Schema("Model catalog selector absent"))?
            .2;
        drop(selection);
        drop(records);
        charge.grow(
            catalog_selection.capacity()
                + targets.capacity() * size_of::<(Id<models::AuthoredModel>, usize)>()
                + inventory_tables.capacity() * 512
                + inventory_tables
                    .iter()
                    .map(|t| t.alias.capacity())
                    .sum::<usize>()
                + inventories.capacity() * size_of::<Arc<scope_program::CompiledScopeProgram>>()
                + 16384,
        )?;
        let calls = CallScopes::from_tables_with(
            inputs,
            tables,
            session,
            budget,
            CallScopeMode {
                binding: true,
                admission: false,
            },
            model,
            |plan, tables| {
                let mut declarations = actual_inputs.clone();
                for table in &tables[real..] {
                    let original = tables[..real]
                        .iter()
                        .position(|original| original.alias == table.alias)
                        .ok_or(ModelError::Conflict("Model virtual physical binding"))?;
                    declarations.push(actual_inputs[original].clone());
                }
                let relations = tables
                    .iter()
                    .map(|table| table.relation.clone())
                    .collect::<Vec<_>>();
                let target_ports = targets.iter().map(|(_, root)| *root).collect::<Vec<_>>();
                let program = compiler_scope_program::model(
                    declarations,
                    &relations,
                    compiler_scope_program::ModelPorts {
                        real,
                        frame,
                        contexts,
                        terminals,
                        exits,
                        targets: &target_ports,
                        event: tables.len() - 1,
                    },
                )?;
                plan.extend(crate::scope_compilation::lower_compiled(
                    crate::scope_compilation::compile(&program, model, budget, None)?,
                    tables,
                    &parameters,
                    budget,
                )?)?;
                // Compiled owners and physical lowering reservations now retain the program.
                // Dispose the raw factory allocations before releasing their construction arena.
                drop(program);
                drop(relations);
                drop(target_ports);
                drop(actual_inputs);
                drop(_construction);
                Ok(())
            },
        )
        .await?;
        Ok(Self {
            catalog_selection,
            _catalog_program: catalog_program,
            inventory_tables,
            inventories,
            _charge: charge,
            calls,
            frame,
            targets,
            contexts,
            terminals,
            exits,
        })
    }
    pub(super) fn root_sql(
        &self,
        kind: compiler_scope_program::ModelRootInventory,
        parent: Id<analysis::enriched_execution::AnalysisInvocation>,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
        budget: &resources::ResourceBudget,
    ) -> Result<(String, charged::StateCharge), ModelError> {
        use compiler_scope_program::ModelRootInventory as K;
        let mut charge = charged::StateCharge::new(budget, "model-root-inventory-query");
        charge.grow(1024)?;
        let parameters = scope_program::ScopeParameters(vec![
            scope_program::ScopeValue::Nominal(*parent.bytes()),
            scope_program::ScopeValue::Nominal(*input.bytes()),
            scope_program::ScopeValue::Nominal(*context.bytes()),
            scope_program::ScopeValue::Nominal(*calls::CallOrigin::explicit().bytes()),
        ]);
        let index = |kind| match kind {
            K::Context => 0,
            K::Event => 1,
            K::Terminal => 2,
            K::TerminalEvent => 3,
            K::Exit => 4,
        };
        charge.grow(
            crate::scope_compilation::lowering_allowance(
                self.inventories[index(kind)].program(),
                &self.inventory_tables,
                &parameters,
            )
            .saturating_mul(4),
        )?;
        if let Some(exclusion) = kind.exclusion() {
            charge.grow(
                crate::scope_compilation::lowering_allowance(
                    self.inventories[index(exclusion)].program(),
                    &self.inventory_tables,
                    &parameters,
                )
                .saturating_mul(4),
            )?;
        }
        let lower = |index: usize| -> Result<String, ModelError> {
            let plan = crate::scope_compilation::lower_compiled(
                self.inventories[index].clone(),
                &self.inventory_tables,
                &parameters,
                budget,
            )?;
            let (_, _, sql) = plan
                .into_pair_queries()
                .into_iter()
                .next()
                .ok_or(ModelError::Schema("Model inventory pair absent"))?;
            Ok(format!(
                "SELECT DISTINCT target_id AS id FROM ({sql}) roots"
            ))
        };
        let selected = lower(index(kind))?;
        let query = if let Some(exclusion) = kind.exclusion() {
            format!(
                "SELECT id FROM ({selected} EXCEPT {}) selected_roots ORDER BY id",
                lower(index(exclusion))?
            )
        } else {
            format!("{selected} ORDER BY id")
        };
        Ok((query, charge))
    }
    fn root_binding(
        &self,
        scope: ProductionScope,
        run: Id<attribution::ProviderRun>,
    ) -> Result<crate::consumed_rows::PreparedRoot, ModelError> {
        let (table, key) = match scope {
            ProductionScope::Frame => (self.frame, *run.bytes()),
            ProductionScope::Target(model) => (
                self.targets
                    .iter()
                    .find(|(id, _)| *id == model)
                    .ok_or(ModelError::Schema(models::AuthoredModel::NAME))?
                    .1,
                *run.bytes(),
            ),
            ProductionScope::Context(id) => (self.contexts, *id.bytes()),
            ProductionScope::Terminal(id) => (self.terminals, *id.bytes()),
            ProductionScope::Exit(id) => (self.exits, *id.bytes()),
            ProductionScope::Event(id) => (
                self.calls
                    .root_for::<normalized::events::NormalizedCallEvent>()?,
                *id.bytes(),
            ),
            _ => return Err(ModelError::Invalid("Model requires selected root".into())),
        };
        Ok(crate::consumed_rows::PreparedRoot {
            table,
            key,
            kind: crate::consumed_rows::PreparedRootKind::Virtual,
        })
    }
    pub(super) async fn window<'a>(
        &'a self,
        access: &CompletedInputs,
        kinds: &[ProductionScope],
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<ModelWindow<'a>, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "model-columnar-window");
        charge.grow(
            kinds.len() * size_of::<crate::consumed_rows::PreparedRoot>()
                + self.calls.inputs().len() * size_of::<Vec<arrow_array::RecordBatch>>()
                + 4096,
        )?;
        let roots = kinds
            .iter()
            .map(|kind| self.root_binding(*kind, run))
            .collect::<Result<Vec<_>, _>>()?;
        let batch = self
            .calls
            .edges()
            .batch_with_cancellation(&roots, budget, cancellation)
            .await?;
        drop(roots);
        let mut columns = ModelColumns {
            rows: (0..self.calls.inputs().len()).map(|_| Vec::new()).collect(),
            consumed: crate::consumed_rows::ConsumedInputs::new(
                self.calls.inputs().to_vec(),
                budget,
            )?,
            charge,
        };
        let mut readers: Vec<ColumnReader> = Vec::new();
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(readers.push(read_columns::<$ty>);)*};}
        model_decoder_inputs!(read);
        for read in readers {
            read(
                self,
                access,
                &batch.union,
                budget,
                cancellation,
                &mut columns,
            )
            .await?;
        }
        let terminal = std::mem::replace(
            &mut columns.consumed,
            crate::consumed_rows::ConsumedInputs::new(vec![], budget)?,
        );
        terminal.finish("model-columnar-union")?;
        Ok(ModelWindow {
            scopes: self,
            batch,
            columns,
        })
    }
    #[cfg(test)]
    pub(super) async fn frame(
        &self,
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.calls
            .edges()
            .grain(self.frame, &format!("id={}", hex(run)), budget)
            .await
    }
    #[cfg(test)]
    pub(super) async fn selected(
        &self,
        scope: ProductionScope,
        run: Id<attribution::ProviderRun>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let (root, key) = match scope {
            ProductionScope::Target(model) => (
                self.targets
                    .iter()
                    .find(|(id, _)| *id == model)
                    .ok_or(ModelError::Schema(models::AuthoredModel::NAME))?
                    .1,
                hex(run),
            ),
            ProductionScope::Context(id) => (self.contexts, hex(id)),
            ProductionScope::Terminal(id) => (self.terminals, hex(id)),
            ProductionScope::Exit(id) => (self.exits, hex(id)),
            ProductionScope::Frame => return self.frame(run, budget).await,
            ProductionScope::Event(id) => return self.calls.grain(id, budget).await,
            _ => return Err(ModelError::Invalid("Model requires selected root".into())),
        };
        self.calls
            .edges()
            .grain(root, &format!("id={key}"), budget)
            .await
    }
    fn select(
        &self,
        grain: &PreparedClosure,
        table: usize,
        input: &ValidationInput,
    ) -> Result<String, ModelError> {
        let sql = grain.select(table)?;
        // Compact configuration dependencies may reference another catalog. They remain declared
        // inputs, but only the captured selected source language belongs to this model kernel.
        Ok(if input.type_id() == TypeId::of::<models::ModelCatalog>() {
            format!(
                "SELECT * FROM ({sql}) captured_catalog WHERE id IN (SELECT target_id FROM ({}) selected_catalog)",
                self.catalog_selection
            )
        } else {
            sql
        })
    }
}

#[cfg(test)]
mod model_scope_controls {
    use super::*;
    use lctx_model::domain::normalized::Rows;
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn target_closure_preserves_absence_and_same_context_candidates_without_foreign_payload()
    {
        let model = model().unwrap();
        let session = datafusion::prelude::SessionContext::new();
        let budget = resources::ResourceBudget::fixed(16 << 20).unwrap();
        let catalog = models::Catalog::parse(
            "external.toml",
            include_str!("../../../lctx-model/models/external.toml"),
        )
        .unwrap();
        let parsed = SelectedCatalog::read(catalog.declaration(), &budget).unwrap();
        let inputs = ModelData::inputs();
        let tables: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| ClosureTable {
                relation: model.relation(input.name()).unwrap().clone(),
                alias: format!("model_scope_{i}"),
            })
            .collect();
        for table in &tables {
            session
                .register_batch(
                    &table.alias,
                    arrow_array::RecordBatch::new_empty(table.relation.schema().clone()),
                )
                .unwrap();
        }
        fn replace<R: Record>(
            session: &datafusion::prelude::SessionContext,
            inputs: &[ValidationInput],
            tables: &[ClosureTable],
            rows: &[R],
        ) {
            for (index, _) in inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| input.type_id() == TypeId::of::<R>())
            {
                session.deregister_table(&tables[index].alias).unwrap();
                session
                    .register_batch(&tables[index].alias, R::encode(rows).unwrap())
                    .unwrap();
            }
        }
        let selected =
            source::SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"x").unwrap();
        let foreign = source::SourceArtifact::from_bytes(
            nominal(1),
            format!("{}.py", "z".repeat(256 << 10)),
            b"x",
        )
        .unwrap();
        let run = attribution::ProviderRun {
            provider: nominal(2),
            context: nominal(3),
            input: selected.input,
            configuration: ContentHash::of(b"run"),
            requested_families: ContentHash::of(b"families"),
        };
        let module = calls::ProviderModule::Bundled {
            provider: run.provider,
            bundle: calls::ModuleBundle::Typeshed,
            name: "typing".into(),
        };
        let symbol = calls::ProviderSymbol {
            provider: run.provider,
            context: run.context,
            module: module.id(),
            native_key: "cast".into(),
            name: "cast".into(),
            kind: calls::SymbolKind::Function,
        };
        let other = calls::ProviderSymbol {
            context: nominal(4),
            native_key: "foreign".repeat(65536),
            ..symbol.clone()
        };
        let alternative =
            source::SourceArtifact::from_bytes(nominal(1), "eligible.pyi".into(), b"y").unwrap();
        let wrong_role =
            source::SourceArtifact::from_bytes(nominal(1), "configuration.py".into(), b"z")
                .unwrap();
        let wrong_suffix =
            source::SourceArtifact::from_bytes(nominal(1), "guide.py.txt".into(), b"w").unwrap();
        let expected = if selected.id() < alternative.id() {
            &selected
        } else {
            &alternative
        };
        replace(
            &session,
            &inputs,
            &tables,
            &[
                selected.clone(),
                alternative.clone(),
                wrong_role.clone(),
                wrong_suffix.clone(),
                foreign,
            ],
        );
        replace(
            &session,
            &inputs,
            &tables,
            &[
                input::ArtifactUse {
                    artifact: selected.id(),
                    input: selected.input,
                    role: input::SourceRole::Release,
                },
                input::ArtifactUse {
                    artifact: alternative.id(),
                    input: alternative.input,
                    role: input::SourceRole::DocBlock,
                },
                input::ArtifactUse {
                    artifact: wrong_role.id(),
                    input: wrong_role.input,
                    role: input::SourceRole::Configuration,
                },
                input::ArtifactUse {
                    artifact: wrong_suffix.id(),
                    input: wrong_suffix.input,
                    role: input::SourceRole::Example,
                },
            ],
        );
        replace(&session, &inputs, &tables, std::slice::from_ref(&run));
        replace(&session, &inputs, &tables, &[module]);
        replace(&session, &inputs, &tables, &[symbol.clone(), other]);
        let selected_catalog = catalog.declaration();
        let source = format!("{}\n#{}", selected_catalog.source, "unused".repeat(1 << 20));
        let unused_catalog = models::ModelCatalog {
            source_name: "unused.toml".into(),
            content: ContentHash::of(source.as_bytes()),
            source,
            ..selected_catalog.clone()
        };
        let unused_parameters = analysis::MethodParameters {
            depth: None,
            proof_steps: None,
            work: None,
            members: None,
            seed: None,
            iterations: None,
            threshold: None,
            resolution: None,
            damping: None,
            model_catalog: Some(unused_catalog.id()),
        };
        replace(
            &session,
            &inputs,
            &tables,
            &[selected_catalog.clone(), unused_catalog],
        );
        let (model_parameters, model_definition) =
            execution::configuration::models(selected_catalog.id());
        replace(
            &session,
            &inputs,
            &tables,
            &[unused_parameters, model_parameters.clone()],
        );
        replace(
            &session,
            &inputs,
            &tables,
            std::slice::from_ref(&model_definition),
        );
        let scopes =
            ModelScopes::from_tables(inputs.clone(), tables, &session, &parsed, &budget, &model)
                .await
                .unwrap();
        let tiny = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let cast=parsed.catalog().models().iter().find(|compiled|matches!(&compiled.model().target,models::Target::Stdlib{callable,..}if callable=="cast")).unwrap();
        {
            use crate::consumed_rows::PreparedRootKind;
            let kinds = [
                ProductionScope::Target(cast.declaration().id()),
                ProductionScope::Target(cast.declaration().id()),
                ProductionScope::Context(nominal(255)),
            ];
            let roots = kinds
                .iter()
                .map(|kind| scopes.root_binding(*kind, run.id()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(
                roots
                    .iter()
                    .all(|root| root.kind == PreparedRootKind::Virtual)
            );
            let batch = scopes.calls.edges().batch(&roots, &budget).await.unwrap();
            let symbol_table = inputs
                .iter()
                .position(|input| input.type_id() == TypeId::of::<calls::ProviderSymbol>())
                .unwrap();
            for partition in 0..2 {
                assert_eq!(
                    batch
                        .keys(partition, symbol_table)
                        .unwrap()
                        .collect::<Vec<_>>(),
                    vec![*symbol.id().bytes()]
                );
            }
            assert_eq!(
                batch.keys(2, symbol_table).unwrap().count(),
                0,
                "absent context cannot inherit target or frame membership"
            );
            let mut columns = ModelColumns {
                rows: (0..inputs.len()).map(|_| Vec::new()).collect(),
                consumed: crate::consumed_rows::ConsumedInputs::new(vec![], &budget).unwrap(),
                charge: charged::StateCharge::new(&budget, "model-window-control"),
            };
            columns
                .charge
                .grow(inputs.len() * size_of::<Vec<arrow_array::RecordBatch>>() + 4096)
                .unwrap();
            use futures::TryStreamExt;
            for (table, input) in inputs.iter().enumerate() {
                let query = scopes.select(&batch.union, table, input).unwrap();
                let mut stream = crate::sql::query(batch.union.session(), &query)
                    .await
                    .unwrap()
                    .execute_stream()
                    .await
                    .unwrap();
                while let Some(rows) = stream.try_next().await.unwrap() {
                    columns
                        .charge
                        .grow(rows.get_array_memory_size() + size_of::<arrow_array::RecordBatch>())
                        .unwrap();
                    columns.rows[table].push(rows);
                }
            }
            let window = ModelWindow {
                scopes: &scopes,
                batch,
                columns,
            };
            let invocation = analysis::model::AnalysisInvocation::new(
                run.input,
                run.context,
                model_definition.id(),
                None,
                [],
            )
            .0;
            let mut independent = ModelData::new(&budget);
            independent
                .catalogs
                .insert(selected_catalog.clone())
                .unwrap();
            independent
                .parameters
                .insert(model_parameters.clone())
                .unwrap();
            independent
                .definitions
                .insert(model_definition.clone())
                .unwrap();
            independent
                .early
                .bindings
                .symbols
                .insert(symbol.clone())
                .unwrap();
            let oracle = execution::model_production::apply_selected(
                &independent,
                &invocation,
                &model_definition,
                stages::Profile::Catalog,
                &parsed,
                None,
                kinds[0],
                None,
                &budget,
            )
            .unwrap();
            for (partition, kind) in kinds.iter().copied().enumerate().take(2) {
                let selected = window
                    .data(
                        partition,
                        &budget,
                        &crate::workspace::Cancellation::default(),
                    )
                    .unwrap();
                assert_eq!(selected.early.bindings.symbols.len(), 1);
                assert_eq!(
                    selected.early.bindings.symbols.get(symbol.id()),
                    Some(&symbol)
                );
                assert_eq!(selected.catalogs.len(), 1);
                assert_eq!(
                    selected.catalogs.get(selected_catalog.id()),
                    Some(selected_catalog)
                );
                let actual = execution::model_production::apply_selected(
                    &selected,
                    &invocation,
                    &model_definition,
                    stages::Profile::Catalog,
                    &parsed,
                    None,
                    kind,
                    None,
                    &budget,
                )
                .unwrap();
                assert_eq!(actual.run, oracle.run);
                assert_eq!(actual.outcome, oracle.outcome);
                assert!(actual.applications.is_empty());
                assert!(actual.targets.is_empty());
                assert!(actual.operations.is_empty());
            }
            let absent = window
                .data(2, &budget, &crate::workspace::Cancellation::default())
                .unwrap();
            assert!(absent.catalogs.is_empty());
            assert!(absent.early.bindings.symbols.is_empty());
            assert!(
                execution::model_production::apply_selected(
                    &absent,
                    &invocation,
                    &model_definition,
                    stages::Profile::Catalog,
                    &parsed,
                    None,
                    kinds[2],
                    None,
                    &budget
                )
                .is_err()
            );
            assert!(
                window
                    .data(3, &budget, &crate::workspace::Cancellation::default())
                    .is_err()
            );
        }

        let grain = scopes
            .selected(
                ProductionScope::Target(cast.declaration().id()),
                run.id(),
                &tiny,
            )
            .await
            .unwrap();
        let catalog_table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<models::ModelCatalog>())
            .unwrap();
        let mut catalogs = Rows::<models::ModelCatalog>::new(&tiny);
        let mut stream = crate::sql::query(
            grain.session(),
            &scopes
                .select(&grain, catalog_table, &inputs[catalog_table])
                .unwrap(),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            catalogs.decode(&batch).unwrap();
        }
        assert_eq!(catalogs.len(), 1);
        assert_eq!(catalogs.get(selected_catalog.id()), Some(selected_catalog));
        drop(catalogs);
        drop(stream);
        let mut selected_symbols = Rows::<calls::ProviderSymbol>::new(&tiny);
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<calls::ProviderSymbol>())
            .unwrap();
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        use futures::TryStreamExt;
        while let Some(batch) = stream.try_next().await.unwrap() {
            selected_symbols.decode(&batch).unwrap();
        }
        assert_eq!(selected_symbols.len(), 1);
        assert_eq!(selected_symbols.get(symbol.id()), Some(&symbol));
        drop(selected_symbols);
        drop(stream);
        drop(grain);
        let absent=parsed.catalog().models().iter().find(|compiled|matches!(&compiled.model().target,models::Target::Stdlib{callable,..}if callable=="assert_type")).unwrap();
        let grain = scopes
            .selected(
                ProductionScope::Target(absent.declaration().id()),
                run.id(),
                &tiny,
            )
            .await
            .unwrap();
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            assert_eq!(batch.num_rows(), 0);
        }
        let table = inputs
            .iter()
            .position(|input| input.type_id() == TypeId::of::<source::SourceArtifact>())
            .unwrap();
        let mut artifacts = Rows::<source::SourceArtifact>::new(&tiny);
        let mut stream = crate::sql::query(grain.session(), &grain.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            artifacts.decode(&batch).unwrap();
        }
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts.get(expected.id()), Some(expected));
        assert!(artifacts.get(wrong_role.id()).is_none());
        assert!(artifacts.get(wrong_suffix.id()).is_none());
    }
}
