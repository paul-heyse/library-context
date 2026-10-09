//! Computed normalization stages over admitted completed-stage inputs.
use crate::producer_operations::{Declaration, declare, declare_ordered, emit};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use arrow_array::Array;
use futures::{TryStreamExt, future::BoxFuture};
use lctx_model::domain::{
    normalized::{
        Rows,
        entity_normalization::{self, EntityData},
    },
    stages::*,
    *,
};
use std::sync::Arc;
mod admission;
pub(crate) mod call_scope;
mod callable_scope;
mod projection_admission;
mod receiver_scope;
pub use admission::{validate_bindings, validate_events, validate_receivers};
pub use projection_admission::validate_projections;

/// Actual normalization output authority bound to the immutable attempt descriptors.
pub(crate) struct ProducedNormalization<T> {
    premises: CompletedInputs,
    outputs: CompletedInputs,
    value: T,
}
impl<T> ProducedNormalization<T> {
    fn borrow(&self, access: &CompletedInputs, workspace: &Workspace) -> Result<&T, ModelError> {
        self.premises.require_subset(workspace, access)?;
        self.outputs.require_subset(workspace, access)?;
        Ok(&self.value)
    }
}
fn selected_premises(
    access: &CompletedInputs,
    inputs: Vec<ValidationInput>,
) -> Result<CompletedInputs, ModelError> {
    access.select(
        &inputs
            .into_iter()
            .filter(|input| access.table_for(input).is_ok())
            .collect::<Vec<_>>(),
    )
}
struct ModuleKeys {
    stream: datafusion::physical_plan::SendableRecordBatchStream,
    batch: Option<arrow_array::RecordBatch>,
    row: usize,
}
impl ModuleKeys {
    async fn new(session: &datafusion::prelude::SessionContext) -> Result<Self, ModelError> {
        let sql = format!(
            "SELECT source,id FROM {} ORDER BY source,id",
            source::Module::NAME
        );
        let stream = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        Ok(Self {
            stream,
            batch: None,
            row: 0,
        })
    }
    async fn next(&mut self) -> Result<Option<([u8; 16], [u8; 16])>, ModelError> {
        loop {
            if let Some(batch) = &self.batch
                && self.row < batch.num_rows()
            {
                let key = |column: usize| {
                    batch
                        .column(column)
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                        .ok_or(ModelError::Schema("module key stream"))?
                        .value(self.row)
                        .try_into()
                        .map_err(ModelError::codec)
                };
                let result = (key(0)?, key(1)?);
                self.row += 1;
                return Ok(Some(result));
            }
            self.batch = self.stream.try_next().await.map_err(ModelError::codec)?;
            self.row = 0;
            if self.batch.is_none() {
                return Ok(None);
            }
        }
    }
}
struct EntityScopes {
    inputs: Vec<ValidationInput>,
    edges: crate::consumed_rows::PreparedEdges,
    roots: Vec<(std::any::TypeId, usize)>,
    _charge: charged::StateCharge,
}
impl EntityScopes {
    async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &ValidatedModel,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = EntityData::validation_inputs()
            .into_iter()
            .filter(|input| access.table_for(input).is_ok())
            .collect::<Vec<_>>();
        let mut tables = inputs
            .iter()
            .map(|input| {
                Ok(crate::consumed_rows::ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("entity scope model relation"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program =
            normalized::normalization_scope_program::entity(inputs.clone(), &relations, budget)?;
        let roots = program.entity_roots().to_vec();
        for (kind, _) in &roots {
            let source = tables[..inputs.len()]
                .iter()
                .position(|table| table.relation.type_id() == *kind)
                .ok_or(ModelError::Schema("entity demand binding"))?;
            tables.push(tables[source].clone());
        }
        let compiled = crate::scope_compilation::compile(program.program(), model, budget, None)?;
        let plan = crate::scope_compilation::lower_compiled(
            compiled,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        let mut charge = charged::StateCharge::new(budget, "entity-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + roots.capacity() * size_of::<(std::any::TypeId, usize)>(),
        )?;
        Ok(Self {
            inputs,
            edges,
            roots,
            _charge: charge,
        })
    }
    async fn produce(
        &self,
        roots: &[crate::consumed_rows::PreparedRoot],
        kernel: entity_normalization::EntityKernel,
        runtime: &Workspace,
        output: &ProducerOutput,
    ) -> Result<(), ModelError> {
        let selected = self
            .edges
            .batch_with_cancellation(roots, runtime.budget(), &runtime.cancellation())
            .await?;
        let mut data = EntityData::new(runtime.budget());
        crate::scoped_batch::hydrate_union_first(
            &selected,
            &self.inputs,
            runtime.budget(),
            &runtime.cancellation(),
            &mut |_, input, batch| data.visit(input.name(), batch).map(|_| ()),
        )
        .await?;
        for partition in 0..roots.len() {
            runtime.cancellation().check()?;
            macro_rules! selected_inputs {($($field:ident:$ty:ty => $family:ident,)*)=>{{
    $(let $field=if let Some(table)=self.inputs.iter().position(|input|input.type_id()==std::any::TypeId::of::<$ty>()){
      // Entity's former rich nominal work list refused required available premises. Keep that
      // refusal after shared hydration rather than converting it to an absent borrowed row.
      for key in selected.keys(partition,table)?{let id=callable_scope::nominal::<$ty>(&key)?;if data.$field.get(id).is_none(){return Err(ModelError::Invalid(format!("normalization scope is missing a required {} premise",<$ty>::NAME)));}}
      Some(crate::scoped_batch::SelectedRows::new(&selected,partition,table,&data.$field,runtime.budget())?)
    }else{None};)*
    let view=entity_normalization::EntityDataView{$($field:if let Some(selection)=&$field{selection.view()?}else{data.$field.view()},)*};
    let rows=entity_normalization::normalize_scope_view(&view,kernel,runtime.budget())?;
    emit_entities(&rows,output,matches!(kernel,entity_normalization::EntityKernel::Public|entity_normalization::EntityKernel::Enumeration)).await?;
   }};}
            lctx_model::normalized_entity_inputs!(selected_inputs);
            tokio::task::yield_now().await;
        }
        Ok(())
    }
}
fn emit_relations<'a>(
    rows: &'a normalized::relation_normalization::RelationOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::relation_normalization::RelationOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::relation_normalization::RelationOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_relation_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_callables<'a>(
    rows: &'a normalized::callable_normalization::CallableOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::callable_normalization::CallableOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::callable_normalization::CallableOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_callable_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_receivers<'a>(
    rows: &'a normalized::receiver::ReceiverOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::receiver::ReceiverOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::receiver::ReceiverOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_receiver_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_bindings<'a>(
    rows: &'a normalized::binding_normalization::BindingOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::binding_normalization::BindingOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::binding_normalization::BindingOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_binding_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_projections<'a>(
    rows: &'a projection::normalization::ProjectionOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a projection::normalization::ProjectionOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a projection::normalization::ProjectionOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::projection_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_aspects<'a>(
    rows: &'a normalized::callable_aspects::AspectOutput,
    output: &'a ProducerOutput,
    class: bool,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::callable_aspects::AspectOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::callable_aspects::AspectOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::callable_aspect_outputs!(adapters);
    Box::pin(async move {
        for (field, emission) in EMISSIONS {
            if !class || !matches!(*field, "sources" | "aspects" | "defaults" | "fields") {
                emission(rows, output).await?;
            }
        }
        Ok(())
    })
}

fn emit_entities<'a>(
    rows: &'a entity_normalization::EntityOutput,
    output: &'a ProducerOutput,
    public_only: bool,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a entity_normalization::EntityOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a entity_normalization::EntityOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_entity_outputs!(adapters);
    Box::pin(async move {
        for (field, emission) in EMISSIONS {
            if !public_only
                || matches!(
                    *field,
                    "exposures" | "public_enumerations" | "exposure_candidates"
                )
            {
                emission(rows, output).await?;
            }
        }
        Ok(())
    })
}

type VocabularyEmitter = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a Workspace,
    &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn emit_vocabulary<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    runtime: &'a Workspace,
    output: &'a ProducerOutput,
    to_ref: impl Fn(Id<R>) -> normalized::entities::EntityRef + Send + 'a,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let _input = access.read::<R>()?;
        let mut stream = crate::sql::query(session, &format!("SELECT * FROM {}", R::NAME))
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _decode = runtime
                .budget()
                .reserve("entity-vocabulary-decode", decode_allowance::<R>(&batch)?)?;
            for row in R::decode(&batch)? {
                output.push(to_ref(row.id())).await?;
            }
        }
        Ok(())
    })
}
macro_rules! vocabulary_adapter {
    ($name:ident, $ty:ty, $variant:ident, $field:ident) => {
        fn $name<'a>(
            access: &'a CompletedInputs,
            session: &'a datafusion::prelude::SessionContext,
            runtime: &'a Workspace,
            output: &'a ProducerOutput,
        ) -> BoxFuture<'a, Result<(), ModelError>> {
            emit_vocabulary::<$ty>(access, session, runtime, output, |id| {
                normalized::entities::EntityRef::$variant { $field: id }
            })
        }
    };
}
vocabulary_adapter!(emit_module_refs, source::Module, Module, module);
vocabulary_adapter!(emit_type_refs, types::TypeTerm, Type, term);
vocabulary_adapter!(emit_place_refs, value::Place, Place, place);

pub async fn entities(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        calls::*, source::*, symbols::*, syntax::ClassFieldSyntaxObservation,
    };
    let session = access.session(runtime).await?;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_entity_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    let mut owners = entity_normalization::OwnershipSweep::new(runtime.budget());
    // Merge compact module keys with the structural stream. A rich occurrence-side hash join
    // cannot spill when statistics pick the wrong build side; two external sorts can.
    let _occurrences = access.read::<Occurrence>()?;
    let _modules = access.read::<Module>()?;
    let mut modules = ModuleKeys::new(&session).await?;
    let mut pending = modules.next().await?;
    let mut source_module: Option<([u8; 16], Option<Id<Module>>)> = None;
    let sql = format!(
        "SELECT * FROM {} ORDER BY source,structural_path",
        Occurrence::NAME
    );
    let mut stream = crate::sql::query(&session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let _decode = runtime.budget().reserve(
            "ownership-stream-decode",
            decode_allowance::<Occurrence>(&batch)?,
        )?;
        for row in Occurrence::decode(&batch)?.iter() {
            if source_module
                .as_ref()
                .is_none_or(|(source, _)| source != row.source.bytes())
            {
                while pending
                    .as_ref()
                    .is_some_and(|(source, _)| source < row.source.bytes())
                {
                    pending = modules.next().await?;
                }
                let mut count = 0usize;
                let mut only = None;
                while let Some((source, module)) = pending
                    && source == *row.source.bytes()
                {
                    count = count
                        .checked_add(1)
                        .ok_or(ModelError::Schema("source module membership count"))?;
                    only = Some(
                        serde_json::from_value(
                            serde_json::to_value(module).map_err(ModelError::codec)?,
                        )
                        .map_err(ModelError::codec)?,
                    );
                    pending = modules.next().await?;
                }
                source_module = Some((*row.source.bytes(), if count == 1 { only } else { None }));
            }
            let module = source_module.as_ref().expect("selected source").1;
            let rows = owners.push(row, module, runtime.budget())?;
            emit_entities(&rows, &output, false).await?;
        }
    }
    drop(stream);
    drop(modules);
    drop(owners);
    // These vocabulary rows have no reducer state or dependency dictionary.
    for emit_vocabulary in [
        emit_module_refs as VocabularyEmitter,
        emit_type_refs,
        emit_place_refs,
    ] {
        emit_vocabulary(&access, &session, runtime, &output).await?;
    }
    let scopes = EntityScopes::prepare(&access, &session, _model, runtime.budget()).await?;
    for (relation, kernel) in [
        (
            ProviderSymbol::NAME,
            entity_normalization::EntityKernel::Symbol,
        ),
        (
            ClassFieldSyntaxObservation::NAME,
            entity_normalization::EntityKernel::SyntaxField,
        ),
        (
            PublicNameObservation::NAME,
            entity_normalization::EntityKernel::Public,
        ),
        (
            ExportEnumerationObservation::NAME,
            entity_normalization::EntityKernel::Enumeration,
        ),
    ] {
        let mut roots =
            crate::sql::query(&session, &format!("SELECT id FROM {relation} ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("entity scope keys"))?;
            let root = scopes
                .roots
                .iter()
                .find(|(kind, _)| {
                    _model
                        .relation(relation)
                        .is_some_and(|record| record.type_id() == *kind)
                })
                .ok_or(ModelError::Schema("entity requested root"))?
                .1;
            let _charge = runtime.budget().reserve(
                "entity-root-window",
                32 * size_of::<crate::consumed_rows::PreparedRoot>(),
            )?;
            let mut window = Vec::with_capacity(32);
            for index in 0..keys.len() {
                window.push(crate::consumed_rows::PreparedRoot {
                    table: root,
                    key: keys.value(index).try_into().map_err(ModelError::codec)?,
                    kind: crate::consumed_rows::PreparedRootKind::Virtual,
                });
                if window.len() == 32 {
                    scopes.produce(&window, kernel, runtime, &output).await?;
                    window.clear();
                }
            }
            if !window.is_empty() {
                scopes.produce(&window, kernel, runtime, &output).await?;
            }
        }
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

struct RelationScopes {
    inputs: Vec<ValidationInput>,
    edges: crate::consumed_rows::PreparedEdges,
    root: usize,
    _charge: charged::StateCharge,
}
impl RelationScopes {
    async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &ValidatedModel,
        kernel: normalized::relation_normalization::RelationKernel,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs = normalized::relation_normalization::RelationData::validation_inputs()
            .into_iter()
            .filter(|input| access.table_for(input).is_ok())
            .collect::<Vec<_>>();
        let mut tables = inputs
            .iter()
            .map(|input| {
                Ok(crate::consumed_rows::ClosureTable {
                    relation: model
                        .relation(input.name())
                        .ok_or(ModelError::Schema("relation scope model binding"))?
                        .clone(),
                    alias: access.table_for(input)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = normalized::normalization_scope_program::relation(
            inputs.clone(),
            &relations,
            kernel,
            budget,
        )?;
        let (kind, root) = *program
            .entity_roots()
            .first()
            .ok_or(ModelError::Schema("relation scope demand port"))?;
        let physical = tables
            .iter()
            .position(|table| table.relation.type_id() == kind)
            .ok_or(ModelError::Schema("relation scope root binding"))?;
        tables.push(tables[physical].clone());
        let compiled = crate::scope_compilation::compile(program.program(), model, budget, None)?;
        let plan = crate::scope_compilation::lower_compiled(
            compiled,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let edges = plan.prepare(session, budget).await?;
        let mut charge = charged::StateCharge::new(budget, "relation-scope-descriptors");
        charge.grow(inputs.capacity() * size_of::<ValidationInput>())?;
        Ok(Self {
            inputs,
            edges,
            root,
            _charge: charge,
        })
    }
    fn selected<'a, R: Record>(
        &self,
        selected: &crate::consumed_rows::PreparedRootBatch,
        partition: usize,
        rows: &'a Rows<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<Option<crate::scoped_batch::SelectedRows<'a, R>>, ModelError> {
        let Some(table) = self
            .inputs
            .iter()
            .position(|input| input.type_id() == std::any::TypeId::of::<R>())
        else {
            return Ok(None);
        };
        for key in selected.keys(partition, table)? {
            if rows.get(callable_scope::nominal::<R>(&key)?).is_none() {
                return Err(ModelError::Invalid(format!(
                    "normalization scope is missing a required {} premise",
                    R::NAME
                )));
            }
        }
        Ok(Some(crate::scoped_batch::SelectedRows::new(
            selected, partition, table, rows, budget,
        )?))
    }
    async fn produce(
        &self,
        roots: &[crate::consumed_rows::PreparedRoot],
        kernel: normalized::relation_normalization::RelationKernel,
        runtime: &Workspace,
        output: &ProducerOutput,
    ) -> Result<(), ModelError> {
        use normalized::relation_normalization::{self, RelationData};
        let selected = self
            .edges
            .batch_with_cancellation(roots, runtime.budget(), &runtime.cancellation())
            .await?;
        let mut data = RelationData::new(runtime.budget());
        crate::scoped_batch::hydrate_union_first(
            &selected,
            &self.inputs,
            runtime.budget(),
            &runtime.cancellation(),
            &mut |_, input, batch| data.visit(input.name(), batch).map(|_| ()),
        )
        .await?;
        for partition in 0..roots.len() {
            runtime.cancellation().check()?;
            let facts =
                SelectedRelationFacts::new(self, &selected, partition, &data, runtime.budget())?;
            let entities =
                SelectedRelationEntities::new(self, &selected, partition, &data, runtime.budget())?;
            let relations =
                SelectedRelationInputs::new(self, &selected, partition, &data, runtime.budget())?;
            let view = relations.view(facts.view(&data)?, entities.view(&data)?, &data)?;
            relation_normalization::require_scope_premises(&view, kernel)?;
            let rows =
                relation_normalization::normalize_scope_view(&view, kernel, runtime.budget())?;
            emit_relations(&rows, output).await?;
            tokio::task::yield_now().await;
        }
        Ok(())
    }
}

macro_rules! selected_relation_facts {($($field:ident:$ty:ty=>$family:ident,)*)=>{
 struct SelectedRelationFacts<'a>{$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
 impl<'a> SelectedRelationFacts<'a>{fn new(scope:&RelationScopes,selected:&crate::consumed_rows::PreparedRootBatch,partition:usize,data:&'a normalized::relation_normalization::RelationData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{Ok(Self{$($field:scope.selected(selected,partition,&data.facts.$field,budget)?,)*})}
 fn view<'b>(&'b self,data:&'b normalized::relation_normalization::RelationData)->Result<normalized::entity_normalization::EntityDataView<'b>,ModelError>{Ok(normalized::entity_normalization::EntityDataView{$($field:if let Some(s)=&self.$field{s.view()?}else{data.facts.$field.view()},)*})}}
};}
lctx_model::normalized_entity_inputs!(selected_relation_facts);
macro_rules! selected_relation_entities {($($field:ident:$ty:ty,)*)=>{
 struct SelectedRelationEntities<'a>{$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
 impl<'a> SelectedRelationEntities<'a>{fn new(scope:&RelationScopes,selected:&crate::consumed_rows::PreparedRootBatch,partition:usize,data:&'a normalized::relation_normalization::RelationData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{Ok(Self{$($field:scope.selected(selected,partition,&data.entities.$field,budget)?,)*})}
 fn view<'b>(&'b self,data:&'b normalized::relation_normalization::RelationData)->Result<normalized::entity_normalization::EntityOutputView<'b>,ModelError>{Ok(normalized::entity_normalization::EntityOutputView{$($field:if let Some(s)=&self.$field{s.view()?}else{data.entities.$field.view()},)*})}}
};}
lctx_model::normalized_entity_outputs!(selected_relation_entities);
macro_rules! selected_relation_inputs {($($field:ident:$ty:ty=>$family:ident,)*)=>{
 struct SelectedRelationInputs<'a>{$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
 impl<'a> SelectedRelationInputs<'a>{fn new(scope:&RelationScopes,selected:&crate::consumed_rows::PreparedRootBatch,partition:usize,data:&'a normalized::relation_normalization::RelationData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{Ok(Self{$($field:scope.selected(selected,partition,&data.$field,budget)?,)*})}
 fn view<'b>(&'b self,facts:normalized::entity_normalization::EntityDataView<'b>,entities:normalized::entity_normalization::EntityOutputView<'b>,data:&'b normalized::relation_normalization::RelationData)->Result<normalized::relation_normalization::RelationDataView<'b>,ModelError>{Ok(normalized::relation_normalization::RelationDataView{facts,entities,$($field:if let Some(s)=&self.$field{s.view()?}else{data.$field.view()},)*})}}
};}
lctx_model::normalized_relation_inputs!(selected_relation_inputs);

pub async fn relations(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        documents::*, flow::*, lexical::*, normalized::relation_normalization::RelationKernel,
        ruff::*, symbols::*, syntax::*, types::*, value::*,
    };
    let session = access.session(runtime).await?;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_relation_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    let roots = [
        (ReferenceObservation::NAME, RelationKernel::Reference),
        (
            RuffDefinitionObservation::NAME,
            RelationKernel::NativeDefinition,
        ),
        (ImportAliasObservation::NAME, RelationKernel::Import),
        (ClassAncestryObservation::NAME, RelationKernel::Ancestry),
        (DocumentMentionObservation::NAME, RelationKernel::Mention),
        (TypeTerm::NAME, RelationKernel::Type),
        (TypeVariable::NAME, RelationKernel::Binder),
        (Place::NAME, RelationKernel::Place),
        (FlowTestLeafObservation::NAME, RelationKernel::TestOperand),
    ];
    for (relation, kernel) in roots {
        if relation == FlowTestLeafObservation::NAME
            && !access.contains::<FlowTestLeafObservation>()
        {
            continue;
        }
        let scopes =
            RelationScopes::prepare(&access, &session, _model, kernel, runtime.budget()).await?;
        let mut stream =
            crate::sql::query(&session, &format!("SELECT id FROM {relation} ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("relation scope keys"))?;
            let _charge = runtime.budget().reserve(
                "relation-root-window",
                32 * size_of::<crate::consumed_rows::PreparedRoot>(),
            )?;
            let mut window = Vec::with_capacity(32);
            for index in 0..keys.len() {
                window.push(crate::consumed_rows::PreparedRoot {
                    table: scopes.root,
                    key: keys.value(index).try_into().map_err(ModelError::codec)?,
                    kind: crate::consumed_rows::PreparedRootKind::Virtual,
                });
                if window.len() == 32 {
                    scopes.produce(&window, kernel, runtime, &output).await?;
                    window.clear();
                }
            }
            if !window.is_empty() {
                scopes.produce(&window, kernel, runtime, &output).await?;
            }
        }
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn callables(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        calls::Signature, normalized::callable_normalization, normalized::entities::CallableEntity,
        types::NativeOverloadObservation,
    };
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_callable_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    let session = access.session(runtime).await?;
    for (kernel, root) in [
        (callable_scope::Kernel::Callable, CallableEntity::NAME),
        (callable_scope::Kernel::Signature, Signature::NAME),
        (
            callable_scope::Kernel::Overload,
            NativeOverloadObservation::NAME,
        ),
    ] {
        let scopes = callable_scope::CallableScopes::prepare(
            &access,
            &session,
            model,
            runtime.budget(),
            kernel,
        )
        .await?;
        let declaration = callable_normalization::CallableData::validation_inputs()
            .into_iter()
            .find(|input| input.name() == root)
            .ok_or(ModelError::Schema("callable root input"))?;
        let table = access.table_for(&declaration)?;
        let mut stream =
            crate::sql::query(&session, &format!("SELECT id FROM \"{table}\" ORDER BY id"))
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("callable root projection"))?;
            for start in (0..keys.len()).step_by(32) {
                let end = (start + 32).min(keys.len());
                let (table, kind) = match kernel {
                    callable_scope::Kernel::Callable => (
                        scopes.table_for::<CallableEntity>()?,
                        crate::consumed_rows::PreparedRootKind::Physical,
                    ),
                    callable_scope::Kernel::Signature => (
                        scopes
                            .signature_root()
                            .ok_or(ModelError::Schema("signature demand root absent"))?,
                        crate::consumed_rows::PreparedRootKind::Virtual,
                    ),
                    callable_scope::Kernel::Overload => (
                        scopes.table_for::<NativeOverloadObservation>()?,
                        crate::consumed_rows::PreparedRootKind::Physical,
                    ),
                };
                let _root_charge = runtime
                    .budget()
                    .reserve("callable-root-window", (end - start) * 128 + 4096)?;
                let roots = (start..end)
                    .map(|index| {
                        Ok(crate::consumed_rows::PreparedRoot {
                            table,
                            key: keys.value(index).try_into().map_err(ModelError::codec)?,
                            kind,
                        })
                    })
                    .collect::<Result<Vec<_>, ModelError>>()?;
                let selected = scopes
                    .edges()
                    .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
                    .await?;
                let data = scopes
                    .load_batch(&selected, runtime.budget(), &runtime.cancellation())
                    .await?;
                for (partition, index) in (start..end).enumerate() {
                    let selection = callable_scope::DataSelection::new(
                        &selected,
                        partition,
                        scopes.inputs(),
                        &data,
                        runtime.budget(),
                    )?;
                    let view = selection.view()?;
                    let rows =
                        crate::stage_runtime::borrowed_cpu(access.name(), || match kernel {
                            callable_scope::Kernel::Callable => {
                                callable_normalization::normalize_callable_view(
                                    &view,
                                    callable_scope::nominal(keys.value(index))?,
                                    runtime.budget(),
                                )
                            }
                            callable_scope::Kernel::Signature => {
                                callable_normalization::normalize_signature_view(
                                    &view,
                                    callable_scope::nominal(keys.value(index))?,
                                    runtime.budget(),
                                )
                            }
                            callable_scope::Kernel::Overload => {
                                callable_normalization::normalize_overload_view(
                                    &view,
                                    callable_scope::nominal(keys.value(index))?,
                                    runtime.budget(),
                                )
                            }
                        })?;
                    emit_callables(&rows, &output).await?;
                }
            }
        }
        drop(scopes);
    }
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

/// Dispatch target for the model's typed necessary Callables admission scope.
pub async fn validate_callables(
    invariant: &Invariant,
    tables: Vec<crate::consumed_rows::ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &crate::workspace::Cancellation,
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    callable_scope::validate_callables(invariant, tables, session, budget, cancellation, model)
        .await
}

/// Complete callable metadata is produced one actual assessment, initializer or class at a time.
pub async fn aspects(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::normalized::callable_aspects::{self, AspectKernel};
    let session = access.session(runtime).await?;
    let inputs = callable_aspects::scoped_inputs();
    macro_rules! acknowledge {($($field:ident:$ty:ty,)*)=>{$(let binding=inputs.iter().find(|input|input.type_id()==std::any::TypeId::of::<$ty>()).ok_or(ModelError::Schema("aspect source binding"))?;access.read_at::<$ty>(binding.prefix())?;)*};}
    lctx_model::callable_aspect_inputs!(acknowledge);
    let tables = inputs
        .iter()
        .map(|input| {
            Ok(crate::consumed_rows::ClosureTable {
                relation: model
                    .relation(input.name())
                    .ok_or(ModelError::Schema(input.name()))?
                    .clone(),
                alias: access.table_for(input)?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let prepared = crate::scoped_aspects::AspectScopes::prepare_in(
        inputs,
        tables,
        &callable_aspects::aspect_scope(),
        model,
        &session,
        runtime.budget(),
        Some(runtime.scope_programs()),
    )
    .await?;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::callable_aspect_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    for (index, root) in prepared.roots.iter().enumerate() {
        let (roots_sql, _query) = prepared.inventory_sql(index, runtime.budget())?;
        let mut roots = crate::sql::query(&session, &roots_sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            runtime.cancellation().check()?;
            let keys = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("aspect owner keys"))?;
            for first in (0..keys.len()).step_by(32) {
                let stop = (first + 32).min(keys.len());
                let _roots = runtime
                    .budget()
                    .reserve("aspect-batch-roots", (stop - first) * 128)?;
                let requested = (first..stop)
                    .map(|row| {
                        Ok(crate::consumed_rows::PreparedRoot {
                            table: *root,
                            key: keys.value(row).try_into().map_err(ModelError::codec)?,
                            kind: crate::consumed_rows::PreparedRootKind::Virtual,
                        })
                    })
                    .collect::<Result<Vec<_>, ModelError>>()?;
                let selected = prepared
                    .edges
                    .batch_with_cancellation(&requested, runtime.budget(), &runtime.cancellation())
                    .await?;
                let (data, prior) = crate::scoped_aspects::load_batch(
                    &selected,
                    &prepared.inputs,
                    runtime.budget(),
                    &runtime.cancellation(),
                )
                .await?;
                drop(prior);
                for (partition, request) in requested.iter().enumerate() {
                    runtime.cancellation().check()?;
                    let owner = crate::scoped_aspects::AspectSelection::new(
                        &selected,
                        partition,
                        &prepared.inputs,
                        &data,
                        runtime.budget(),
                    )?;
                    let kernel = crate::scoped_aspects::kernel(index, request.key)?;
                    let rows = crate::stage_runtime::borrowed_cpu("callable_aspects", || {
                        callable_aspects::normalize_scope_view(
                            &owner.view()?,
                            kernel,
                            runtime.budget(),
                        )
                    })?;
                    // Class defaults are scratch; canonical emission remains field-owned.
                    emit_aspects(&rows, &output, matches!(kernel, AspectKernel::Class(_))).await?;
                }
            }
        }
    }
    drop(prepared);
    drop(session);
    output.finish(ProviderOutcome::Complete).await
}

pub async fn receivers(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    receivers_produced(access, output, runtime, model)
        .await
        .map(|_| ())
}
pub(crate) async fn receivers_produced(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<ProducedNormalization<normalized::receiver::VerifiedReceivers>, ModelError> {
    use lctx_model::domain::normalized::receiver;
    let premises = selected_premises(&access, receiver::ReceiverData::validation_inputs())?;
    let mut verified = receiver::normalize_produced(
        &receiver::ReceiverData::new(runtime.budget()),
        runtime.budget(),
    )?
    .1;
    let profile = access.profile();
    let session = access.session(runtime).await?;
    let scopes =
        receiver_scope::ReceiverScopes::prepare(&access, &session, model, runtime.budget()).await?;
    let declaration = ValidationInput::of::<calls::CallTarget>(&["id"]);
    let _permit = access.read_at::<calls::CallTarget>(declaration.prefix())?;
    let table = access.table_for(&declaration)?;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_receiver_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    let mut stream = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&table)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let keys = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("receiver root projection"))?;
        for start in (0..keys.len()).step_by(32) {
            let end = (start + 32).min(keys.len());
            let _root_charge = runtime
                .budget()
                .reserve("receiver-root-window", (end - start) * 128 + 4096)?;
            let table = scopes.table_for::<calls::CallTarget>()?;
            let roots = (start..end)
                .map(|index| {
                    Ok(crate::consumed_rows::PreparedRoot {
                        table,
                        key: keys.value(index).try_into().map_err(ModelError::codec)?,
                        kind: crate::consumed_rows::PreparedRootKind::Physical,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let selected = scopes
                .edges()
                .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
                .await?;
            let data = scopes
                .load_batch(&selected, runtime.budget(), &runtime.cancellation())
                .await?;
            for (partition, index) in (start..end).enumerate() {
                let selection = receiver_scope::DataSelection::new(
                    &selected,
                    partition,
                    scopes.inputs(),
                    &data,
                    runtime.budget(),
                )?;
                let view = selection.view()?;
                let (rows, authority) = crate::stage_runtime::borrowed_cpu(access.name(), || {
                    receiver::normalize_target_produced_view(
                        &view,
                        callable_scope::nominal(keys.value(index))?,
                        runtime.budget(),
                    )
                })?;
                verified.append(authority)?;
                emit_receivers(&rows, &output).await?;
                tokio::task::yield_now().await;
            }
        }
    }
    output.finish(ProviderOutcome::Complete).await?;
    let outputs = runtime.inputs(
        "receiver-produced-authority",
        profile,
        receiver::relations()
            .into_iter()
            .map(|relation| relation.name()),
    )?;
    Ok(ProducedNormalization {
        premises,
        outputs,
        value: verified,
    })
}

fn emit_event_rows<'a>(
    rows: &'a normalized::event_normalization::EventOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a normalized::event_normalization::EventOutput,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a normalized::event_normalization::EventOutput, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[(&str, Emission)] = &[$((stringify!($field), $field),)*];
    };}
    lctx_model::normalized_event_outputs!(adapters);
    Box::pin(async move {
        for (_, emission) in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "Event selection, borrowed authorities and charged deduplication stay with their owners."
)]
fn event_roots<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    runtime: &'a Workspace,
    output: &'a ProducerOutput,
    scopes: &'a call_scope::CallScopes,
    receivers: &'a normalized::receiver::VerifiedReceivers,
    seen: &'a mut charged::ChargedSet<normalized::event_normalization::EventKey>,
    charge: &'a mut charged::StateCharge,
    verified: &'a mut normalized::event_normalization::VerifiedEvents,
    event_key: impl Fn(
        &R,
        Id<attribution::AnalysisContext>,
    ) -> normalized::event_normalization::EventKey
    + Send
    + 'a,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        use normalized::event_normalization::{self, EventData};
        let input = ValidationInput::of::<R>(&["id"]);
        let _permit = access.read_at::<R>(input.prefix())?;
        let table = access.table_for(&input)?;
        let qualifications = access.table_for(
            &EventData::validation_inputs()
                .into_iter()
                .find(|input| {
                    input.type_id() == std::any::TypeId::of::<assertion::AssertionQualification>()
                })
                .expect("event qualification declaration"),
        )?;
        let sql = format!(
            "SELECT r.*,q.context AS root_context FROM {} r JOIN {} q ON q.id=r.qualification ORDER BY r.id",
            crate::consumed_rows::identifier(&table),
            crate::consumed_rows::identifier(&qualifications)
        );
        let mut stream = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let mut rows = Rows::<R>::new(runtime.budget());
            // Decode the record projection without the extra context column.
            let projection = batch
                .project(&(0..batch.num_columns() - 1).collect::<Vec<_>>())
                .map_err(ModelError::codec)?;
            rows.decode(&projection)?;
            let contexts = batch
                .column(batch.num_columns() - 1)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or_else(|| {
                    ModelError::Invalid("event context key has another Arrow type".into())
                })?;
            let _window_charge = runtime
                .budget()
                .reserve("event-root-window", rows.len() * 256 + 4096)?;
            let mut requests = Vec::new();
            for (ordinal, root) in rows.iter().enumerate() {
                let context = callable_scope::nominal(contexts.value(ordinal))?;
                let key = event_key(root, context);
                if seen.insert(charge, key)? {
                    requests.push((root.id(), key));
                }
            }
            for window in requests.chunks(32) {
                let root = scopes.root_for::<R>()?;
                let roots = window
                    .iter()
                    .map(|(id, _)| crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: *id.bytes(),
                        kind: crate::consumed_rows::PreparedRootKind::Virtual,
                    })
                    .collect::<Vec<_>>();
                let selected = scopes
                    .edges()
                    .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
                    .await?;
                let data = scopes
                    .load_event_batch(&selected, runtime.budget(), &runtime.cancellation())
                    .await?;
                for (partition, (_, key)) in window.iter().enumerate() {
                    let selection = call_scope::EventSelection::new(
                        &selected,
                        partition,
                        scopes.inputs(),
                        &data,
                        runtime.budget(),
                    )?;
                    let view = selection.view()?;
                    let (rows, authority) =
                        crate::stage_runtime::borrowed_cpu(access.name(), || {
                            event_normalization::normalize_event_produced_view(
                                &view,
                                *key,
                                receivers,
                                runtime.budget(),
                            )
                        })?;
                    verified.append(authority)?;
                    emit_event_rows(&rows, output).await?;
                    tokio::task::yield_now().await;
                }
            }
        }

        Ok(())
    })
}
type EventRootLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a Workspace,
    &'a ProducerOutput,
    &'a call_scope::CallScopes,
    &'a normalized::receiver::VerifiedReceivers,
    &'a mut charged::ChargedSet<normalized::event_normalization::EventKey>,
    &'a mut charged::StateCharge,
    &'a mut normalized::event_normalization::VerifiedEvents,
) -> BoxFuture<'a, Result<(), ModelError>>;
macro_rules! event_root_adapter {
    ($name:ident, $ty:ty) => {
        #[allow(clippy::too_many_arguments, reason = "Thin typed adapter preserves the explicit event selection and authority owners.")]
        fn $name<'a>(access: &'a CompletedInputs, session: &'a datafusion::prelude::SessionContext, runtime: &'a Workspace, output: &'a ProducerOutput, scopes: &'a call_scope::CallScopes, receivers: &'a normalized::receiver::VerifiedReceivers, seen: &'a mut charged::ChargedSet<normalized::event_normalization::EventKey>, charge: &'a mut charged::StateCharge, verified: &'a mut normalized::event_normalization::VerifiedEvents) -> BoxFuture<'a, Result<(), ModelError>> {
            event_roots::<$ty>(access, session, runtime, output, scopes, receivers, seen, charge, verified, |root, context| (root.site, root.origin, context))
        }
    };
}
event_root_adapter!(provider_site_roots, calls::ProviderCallSite);
event_root_adapter!(target_roots, calls::CallTarget);
event_root_adapter!(resolution_roots, calls::CallResolution);

pub(crate) async fn events_produced(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    receivers: &ProducedNormalization<normalized::receiver::VerifiedReceivers>,
) -> Result<ProducedNormalization<normalized::event_normalization::VerifiedEvents>, ModelError> {
    use normalized::event_normalization::{self, EventData, EventKey};
    let receivers = receivers.borrow(&access, runtime)?;
    let premises = selected_premises(&access, EventData::validation_inputs())?;
    let profile = access.profile();
    let session = access.session(runtime).await?;
    let scopes =
        call_scope::CallScopes::prepare(&access, &session, model, runtime.budget(), false).await?;
    let mut verified = event_normalization::normalize_events_produced(
        &EventData::new(runtime.budget()),
        receivers,
        runtime.budget(),
    )?
    .1;
    let mut seen: charged::ChargedSet<EventKey> = Default::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "event-root-keys");
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_event_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    // Every provider site, target and resolution is a root, including unsupported/empty sets.
    // Compact deduplication roots the complete qualified domain exactly once.
    const ROOTS: &[EventRootLoader] = &[provider_site_roots, target_roots, resolution_roots];
    for roots in ROOTS {
        roots(
            &access,
            &session,
            runtime,
            &output,
            &scopes,
            receivers,
            &mut seen,
            &mut charge,
            &mut verified,
        )
        .await?;
    }
    if access.contains::<flow::FlowValuePathObservation>() {
        let input = ValidationInput::of::<flow::FlowValuePathObservation>(&["id"]);
        let _permit = access.read_at::<flow::FlowValuePathObservation>(input.prefix())?;
        let table = access.table_for(&input)?;
        let mut stream = crate::sql::query(
            &session,
            &format!(
                "SELECT * FROM {} ORDER BY id",
                crate::consumed_rows::identifier(&table)
            ),
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let mut paths = Rows::<flow::FlowValuePathObservation>::new(runtime.budget());
            paths.decode(&batch)?;
            let _window_charge = runtime
                .budget()
                .reserve("flow-path-root-window", paths.len() * 128 + 4096)?;
            let requests = paths.iter().map(Record::id).collect::<Vec<_>>();
            for window in requests.chunks(32) {
                let root = scopes.root_for::<flow::FlowValuePathObservation>()?;
                let roots = window
                    .iter()
                    .map(|id| crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: *id.bytes(),
                        kind: crate::consumed_rows::PreparedRootKind::Virtual,
                    })
                    .collect::<Vec<_>>();
                let selected = scopes
                    .edges()
                    .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
                    .await?;
                let data = scopes
                    .load_event_batch(&selected, runtime.budget(), &runtime.cancellation())
                    .await?;
                for (partition, id) in window.iter().enumerate() {
                    let selection = call_scope::EventSelection::new(
                        &selected,
                        partition,
                        scopes.inputs(),
                        &data,
                        runtime.budget(),
                    )?;
                    let rows = event_normalization::normalize_flow_path_view(
                        &selection.view()?,
                        *id,
                        &verified,
                        runtime.budget(),
                    )?;
                    for row in rows.flow_links.iter() {
                        output.push(row.clone()).await?;
                    }
                }
            }
        }
    }
    output.finish(ProviderOutcome::Complete).await?;
    let outputs = runtime.inputs(
        "event-produced-authority",
        profile,
        normalized::events::relations()
            .into_iter()
            .map(|relation| relation.name()),
    )?;
    Ok(ProducedNormalization {
        premises,
        outputs,
        value: verified,
    })
}
async fn prepare_enumeration_authority(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    application: &mut normalized::binding_normalization::VerifiedBindings,
    runtime: &Workspace,
) -> Result<(), ModelError> {
    use calls::{
        ProviderSymbol, Signature, SignatureEnumerationMember, SignatureEnumerationObservation,
    };
    use normalized::binding_normalization::BindingData;
    let all = BindingData::validation_inputs();
    let kinds = [
        std::any::TypeId::of::<SignatureEnumerationObservation>(),
        std::any::TypeId::of::<SignatureEnumerationMember>(),
        std::any::TypeId::of::<Signature>(),
        std::any::TypeId::of::<assertion::AssertionQualification>(),
        std::any::TypeId::of::<ProviderSymbol>(),
    ];
    let declared = kinds
        .iter()
        .map(|kind| {
            all.iter()
                .find(|input| input.type_id() == *kind)
                .cloned()
                .ok_or(ModelError::Schema("binding enumeration input"))
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let mut tables = declared
        .iter()
        .map(|input| {
            Ok(crate::consumed_rows::ClosureTable {
                relation: runtime
                    .model()
                    .relation(input.name())
                    .ok_or(ModelError::Schema("enumeration model input"))?
                    .clone(),
                alias: access.table_for(input)?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let root = tables.len();
    tables.push(tables[4].clone());
    let input_relations = tables
        .iter()
        .take(root)
        .map(|table| table.relation.clone())
        .collect::<Vec<_>>();
    let program = normalized::enumeration_scope_program::build(
        declared.clone(),
        &input_relations,
        runtime.budget(),
    )?;
    let compiled = crate::scope_compilation::compile(
        program.program(),
        runtime.model(),
        runtime.budget(),
        Some(runtime.scope_programs()),
    )?;
    let edges = crate::scope_compilation::lower_compiled(
        compiled,
        &tables,
        &scope_program::ScopeParameters(vec![]),
        runtime.budget(),
    )?
    .prepare(session, runtime.budget())
    .await?;
    // The root scan carries only the primitive symbol key, never native record bodies.
    let mut stream = crate::sql::query(
        session,
        &format!(
            "SELECT DISTINCT symbol FROM {} ORDER BY symbol",
            crate::consumed_rows::identifier(&tables[0].alias)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let _roots = runtime.budget().reserve(
            "binding-native-enumeration-root-batch",
            logical_batch_bytes(&batch)?,
        )?;
        let roots = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("binding enumeration symbol key"))?;
        for start in (0..roots.len()).step_by(32) {
            let end = (start + 32).min(roots.len());
            let _window = runtime
                .budget()
                .reserve("enumeration-demand-window", (end - start) * 128 + 4096)?;
            let requests = (start..end)
                .map(|index| {
                    if roots.is_null(index) {
                        return Err(ModelError::Schema("binding enumeration symbol key"));
                    }
                    Ok(crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: roots.value(index).try_into().map_err(ModelError::codec)?,
                        kind: crate::consumed_rows::PreparedRootKind::Virtual,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let selected = edges
                .batch_with_cancellation(&requests, runtime.budget(), &runtime.cancellation())
                .await?;
            let mut data = BindingData::new(runtime.budget());
            crate::scoped_batch::hydrate_union(
                &selected,
                &declared,
                runtime.budget(),
                &runtime.cancellation(),
                &mut |_, input, batch| {
                    if !data.visit(input.name(), batch)? {
                        return Err(ModelError::Schema("enumeration union input"));
                    }
                    Ok(())
                },
            )
            .await?;
            for partition in 0..requests.len() {
                let borrowed = call_scope::BindingSelection::new(
                    &selected,
                    partition,
                    &declared,
                    &data,
                    runtime.budget(),
                )?;
                application.admit_enumerations_view(&borrowed.view()?, runtime.budget())?;
            }
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

pub(crate) async fn bindings_prepared(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    receivers: &ProducedNormalization<normalized::receiver::VerifiedReceivers>,
    events: &ProducedNormalization<normalized::event_normalization::VerifiedEvents>,
) -> Result<normalized::binding_normalization::VerifiedBindings, ModelError> {
    use normalized::binding_normalization::{self, BindingData};
    let receivers = receivers.borrow(&access, runtime)?;
    let events = events.borrow(&access, runtime)?;
    let session = access.session(runtime).await?;
    let scopes =
        call_scope::CallScopes::prepare(&access, &session, model, runtime.budget(), true).await?;
    let mut application = binding_normalization::normalize_produced(
        &BindingData::new(runtime.budget()),
        receivers,
        events,
        runtime.budget(),
    )?
    .1;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::normalized_binding_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    let input = ValidationInput::of::<normalized::events::NormalizedCallEvent>(&["id"]);
    let _permit = access.read_at::<normalized::events::NormalizedCallEvent>(input.prefix())?;
    let table = access.table_for(&input)?;
    let mut stream = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&table)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let keys = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("binding root projection"))?;
        for start in (0..keys.len()).step_by(32) {
            let end = (start + 32).min(keys.len());
            let _root_charge = runtime
                .budget()
                .reserve("binding-root-window", (end - start) * 128 + 4096)?;
            let root = scopes.root_for::<normalized::events::NormalizedCallEvent>()?;
            let roots = (start..end)
                .map(|index| {
                    Ok(crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: keys.value(index).try_into().map_err(ModelError::codec)?,
                        kind: crate::consumed_rows::PreparedRootKind::Virtual,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let selected = scopes
                .edges()
                .batch_with_cancellation(&roots, runtime.budget(), &runtime.cancellation())
                .await?;
            let data = scopes
                .load_binding_batch(&selected, runtime.budget(), &runtime.cancellation())
                .await?;
            for (partition, index) in (start..end).enumerate() {
                let selection = call_scope::BindingSelection::new(
                    &selected,
                    partition,
                    scopes.inputs(),
                    &data,
                    runtime.budget(),
                )?;
                let view = selection.view()?;
                let (rows, verified) = crate::stage_runtime::borrowed_cpu(access.name(), || {
                    binding_normalization::normalize_event_produced_view(
                        &view,
                        callable_scope::nominal(keys.value(index))?,
                        receivers,
                        events,
                        runtime.budget(),
                    )
                })?;
                application.append(verified)?;
                emit_bindings(&rows, &output).await?;
                tokio::task::yield_now().await;
            }
        }
    }
    prepare_enumeration_authority(&access, &session, &mut application, runtime).await?;
    output.finish(ProviderOutcome::Complete).await?;
    Ok(application)
}

fn read_projection<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    data: &'a mut projection::normalization::CompactProjectionData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        use projection::{compact::compact_columns, normalization::ProjectionData};
        let input = ProjectionData::validation_inputs()
            .into_iter()
            .find(|input| input.type_id() == std::any::TypeId::of::<R>())
            .expect("projection declared input");
        let permit = access.read_at::<R>(input.prefix())?;
        let table = access.table_for(&input)?;
        let columns = compact_columns(R::NAME)
            .map(|columns| {
                columns
                    .iter()
                    .map(|column| crate::consumed_rows::identifier(column))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "*".into());
        let sql = format!(
            "SELECT {columns} FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&table)
        );
        crate::consumed_rows::stream_query_at(&permit, &input, access, session, &sql, |_, batch| {
            data.visit(R::NAME, batch).map(|_| ())
        })
        .await
    })
}
fn load_projection_inputs<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    data: &'a mut projection::normalization::CompactProjectionData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a CompletedInputs,
        &'a datafusion::prelude::SessionContext,
        &'a mut projection::normalization::CompactProjectionData,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {const LOADERS: &[Loader] = &[$(read_projection::<$ty>,)*];};}
    lctx_model::projection_inputs!(adapters);
    Box::pin(async move {
        for load in LOADERS {
            load(access, session, data).await?;
        }
        Ok(())
    })
}

/// Generation-local computational snapshots are built once, after their canonical inputs finish.
pub async fn projections(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::projection::normalization::CompactProjectionData;
    let session = access.session(runtime).await?;
    let mut data = CompactProjectionData::new(runtime.budget());
    load_projection_inputs(&access, &session, &mut data).await?;
    let prepared = data.prepare(runtime.budget())?;
    macro_rules! declarations {($($field:ident:$ty:ty,)*) => {const DECLARATIONS: &[Declaration] = &[$(declare::<$ty>,)*];};}
    lctx_model::projection_outputs!(declarations);
    declare_ordered(&output, DECLARATIONS).await?;
    for key in prepared.keys() {
        let rows = crate::stage_runtime::borrowed_cpu(access.name(), || {
            prepared.produce(key, runtime.budget())
        })?;
        emit_projections(&rows, &output).await?;
        drop(rows);
        tokio::task::yield_now().await;
    }
    output.finish(ProviderOutcome::Complete).await
}

pub(crate) mod pipeline;

/// Assemble exact normalized scope outcomes over the private, admitted facts checkpoint.
pub async fn coverage(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use lctx_model::domain::{normalized::coverage::*, source::SourceArtifact};
    let sources = access.snapshots().collect::<Vec<_>>();
    let profile = access.profile();
    let evidence = runtime.facts_availability_async(profile).await?;
    let session = access.session(runtime).await?;
    let _permit = access.read::<SourceArtifact>()?;
    let table = access.table_at::<SourceArtifact>(None)?;
    let mut artifacts = ArtifactInputIndex::new(runtime.budget());
    // Coverage retains only the primitive artifact/input correspondence. Paths and artifact
    // payload metadata are excluded by the physical projection before any model decoding.
    let mut stream = crate::sql::query(
        &session,
        &format!("SELECT id,input FROM \"{table}\" ORDER BY id"),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        runtime.cancellation().check()?;
        artifacts.ingest(&batch)?;
        tokio::task::yield_now().await;
    }
    drop(stream);
    drop(session);
    output.declare_async::<NormalizationComputation>().await?;
    output.declare_async::<NormalizationOutputReceipt>().await?;
    output.declare_async::<NormalizationCoverage>().await?;
    output.declare_async::<NormalizationPremise>().await?;
    output.declare_async::<NormalizationEvidenceSet>().await?;
    output
        .declare_async::<NormalizationEvidenceMember>()
        .await?;
    let prepared = CoveragePreparation::new(&evidence, &artifacts, runtime.budget())?;
    for record in prepared.evidence_records() {
        runtime.cancellation().check()?;
        let (set, member) = record?;
        output.push(set).await?;
        output.push(member).await?;
    }
    for capability in Capability::ALL {
        let stage = capability.producer(profile);
        let mut computation = NormalizationComputation {
            capability,
            policy: lctx_model::domain::normalized::policy_revision(),
            producer: stage.name.into(),
            declaration: stage.digest(),
            profile: profile.name().into(),
            availability: EvidenceAvailability::NoScope,
        };
        let mut aggregate = CoverageAggregate::default();
        for (scope, context) in prepared.scopes(capability) {
            runtime.cancellation().check()?;
            let scoped = prepared.outcome(capability, scope, context)?;
            aggregate.include(scoped.availability);
            let row = NormalizationCoverage {
                computation: computation.id(),
                scope,
                context,
                availability: scoped.availability,
            };
            let outcome = row.id();
            output.push(row).await?;
            for premise in &scoped.premises {
                output
                    .push(NormalizationPremise {
                        outcome,
                        premise: premise.id(),
                    })
                    .await?;
            }
        }
        computation.availability = aggregate.finish(capability, profile);
        let id = computation.id();
        output.push(computation).await?;
        for relation in &stage.outputs {
            let source = sources
                .iter()
                .find(|source| source.relation() == relation.name())
                .ok_or_else(|| {
                    ModelError::Frontier(
                        "normalization output has no completed relation view".into(),
                    )
                })?;
            output
                .push(NormalizationOutputReceipt {
                    computation: id,
                    relation: relation.name().into(),
                    rows: source.rows(),
                    view: source.view(),
                })
                .await?;
        }
    }
    output.finish(ProviderOutcome::Complete).await
}
