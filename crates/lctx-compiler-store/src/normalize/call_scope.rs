//! Complete event candidate and binding-set grains over immutable typed premises.
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedEdges},
    admission::InputTables,
};
use lctx_model::domain::{
    normalized::{binding_normalization::BindingData, event_normalization::EventData},
    *,
};
use std::{any::TypeId, sync::Arc};

// Ordinary prepare uses facts_inputs, which deduplicates each nominal relation before
// filtering unavailable tables. Wider summary/model scopes may retain several epochs;
// these normalization readers deliberately select only the first matching declaration.
fn first_input<R: Record>(inputs: &[ValidationInput]) -> Option<(usize, &ValidationInput)> {
    inputs
        .iter()
        .enumerate()
        .find(|(_, input)| input.type_id() == TypeId::of::<R>())
}


pub struct CallScopeMode {
    pub binding: bool,
    pub admission: bool,
}
pub struct CallScopes {
    inputs: Vec<ValidationInput>,
    event_roots: Vec<(TypeId, usize)>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}
impl CallScopes {
    pub async fn prepare(
        access: &impl InputTables,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
        binding: bool,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = if binding {
            BindingData::validation_inputs()
        } else {
            EventData::validation_inputs()
        }
        .into_iter()
        .filter(|input| access.table_for(input).is_ok())
        .collect();
        let tables: Vec<_> = inputs
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
            .collect::<Result<_, ModelError>>()?;
        Self::from_tables(inputs, tables, session, budget, binding, false, model).await
    }
    pub async fn from_tables(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        binding: bool,
        admission: bool,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        Self::from_tables_with(
            inputs,
            tables,
            session,
            budget,
            CallScopeMode { binding, admission },
            model,
            |_, _| Ok(()),
        )
        .await
    }
    pub async fn from_tables_with(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        mode: CallScopeMode,
        model: &ValidatedModel,
        extra: impl FnOnce(&mut NominalClosure, &[ClosureTable]) -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        let CallScopeMode { binding, admission } = mode;
        let mut roots = if binding {
            vec![TypeId::of::<normalized::events::NormalizedCallEvent>()]
        } else {
            vec![
                TypeId::of::<calls::ProviderCallSite>(),
                TypeId::of::<calls::CallTarget>(),
                TypeId::of::<calls::CallResolution>(),
            ]
        };
        if !binding
            && tables.iter().any(|table| {
                table.relation.type_id() == TypeId::of::<flow::FlowValuePathObservation>()
            })
        {
            roots.push(TypeId::of::<flow::FlowValuePathObservation>());
        }
        if !binding && admission {
            roots.push(TypeId::of::<normalized::events::NormalizedCallEvent>());
        }
        // Own the caller's bindings before adding private root copies or entering preparation.
        // The raw factory and the compiled plan have independent reservations.
        let mut charge = charged::StateCharge::new(budget, "call-scope-descriptors");
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
                .saturating_add(tables.capacity().saturating_mul(size_of::<ClosureTable>()))
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
                .saturating_add(roots.len().saturating_mul(size_of::<(TypeId, usize)>())),
        )?;
        let additional = tables
            .len()
            .saturating_add(roots.len())
            .saturating_sub(tables.capacity());
        charge.grow(additional.saturating_mul(size_of::<ClosureTable>()))?;
        tables.reserve_exact(roots.len());
        let mut event_roots = Vec::with_capacity(roots.len());
        for kind in roots {
            let source = tables
                .iter()
                .position(|table| table.relation.type_id() == kind)
                .ok_or_else(|| ModelError::Invalid("call root stream absent".into()))?;
            event_roots.push((kind, tables.len()));
            charge.grow(
                tables[source]
                    .alias
                    .capacity()
                    .saturating_add(size_of_val(tables[source].relation.fields())),
            )?;
            tables.push(tables[source].clone());
        }
        let real = inputs.len();
        let mut declarations = inputs.clone();
        for table in &tables[real..] {
            let source = tables[..real]
                .iter()
                .position(|original| original.alias == table.alias)
                .ok_or(ModelError::Conflict("call virtual physical binding"))?;
            declarations.push(inputs[source].clone());
        }
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = normalized::normalization_scope_program::calls(
            declarations,
            &relations,
            real,
            &event_roots,
            binding,
            admission,
            budget,
        )?;
        let mut plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(program.program(), model, budget, None)?,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        drop(program);
        drop(relations);
        extra(&mut plan, &tables)?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            event_roots,
            edges,
            _charge: charge,
        })
    }
    pub fn edges(&self) -> &PreparedEdges {
        &self.edges
    }
    pub fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
}


impl CallScopes {
    pub fn root_for<R: Record>(&self) -> Result<usize, ModelError> {
        self.event_roots
            .iter()
            .find(|(kind, _)| *kind == TypeId::of::<R>())
            .map(|(_, root)| *root)
            .ok_or(ModelError::Schema("call batch root absent"))
    }
}

macro_rules! selected_Event {($($field:ident:$ty:ty,)*)=>{
    pub struct EventSelection<'a>{rows:&'a EventData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> EventSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a EventData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:first_input::<$ty>(inputs).map(|(table,_)|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::event_normalization::EventDataView<'_>,ModelError>{Ok(normalized::event_normalization::EventDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_event_inputs!(selected_Event);
impl CallScopes {
    pub async fn load_event_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<EventData, ModelError> {
        let mut rows = EventData::new(budget);
        crate::scoped_batch::hydrate_union_first(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                if !rows.visit(input.name(), batch)? {
                    return Err(ModelError::Schema("call union input undeclared"));
                }
                Ok(())
            },
        )
        .await?;
        Ok(rows)
    }
}

macro_rules! selected_Binding {($($field:ident:$ty:ty,)*)=>{
    pub struct BindingSelection<'a>{rows:&'a BindingData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> BindingSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a BindingData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:first_input::<$ty>(inputs).map(|(table,_)|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::binding_normalization::BindingDataView<'_>,ModelError>{Ok(normalized::binding_normalization::BindingDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_binding_inputs!(selected_Binding);
impl CallScopes {
    pub async fn load_binding_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<BindingData, ModelError> {
        let mut rows = BindingData::new(budget);
        crate::scoped_batch::hydrate_union_first(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                if !rows.visit(input.name(), batch)? {
                    return Err(ModelError::Schema("call union input undeclared"));
                }
                Ok(())
            },
        )
        .await?;
        Ok(rows)
    }
}

macro_rules! selected_Event_output {($($field:ident:$ty:ty,)*)=>{
    pub struct EventOutputSelection<'a>{rows:&'a normalized::event_normalization::EventOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> EventOutputSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::event_normalization::EventOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::event_normalization::EventOutputView<'_>,ModelError>{Ok(normalized::event_normalization::EventOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_event_outputs!(selected_Event_output);
impl CallScopes {
    pub async fn load_event_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<(EventData, normalized::event_normalization::EventOutput), ModelError> {
        let mut rows = EventData::new(budget);
        let mut stored = normalized::event_normalization::EventOutput::new(budget);
        crate::scoped_batch::hydrate_union_first(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                let accepted = rows.visit(input.name(), batch)?;
                let output = stored.visit(input.name(), batch)?;
                if !accepted && !output {
                    return Err(ModelError::Schema("admission union input undeclared"));
                }
                Ok(())
            },
        )
        .await?;
        Ok((rows, stored))
    }
}

macro_rules! selected_Binding_output {($($field:ident:$ty:ty,)*)=>{
    pub struct BindingOutputSelection<'a>{rows:&'a normalized::binding_normalization::BindingOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> BindingOutputSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::binding_normalization::BindingOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::binding_normalization::BindingOutputView<'_>,ModelError>{Ok(normalized::binding_normalization::BindingOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_binding_outputs!(selected_Binding_output);
impl CallScopes {
    pub async fn load_binding_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<
        (
            BindingData,
            normalized::binding_normalization::BindingOutput,
        ),
        ModelError,
    > {
        let mut rows = BindingData::new(budget);
        let mut stored = normalized::binding_normalization::BindingOutput::new(budget);
        crate::scoped_batch::hydrate_union_first(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                let accepted = rows.visit(input.name(), batch)?;
                let output = stored.visit(input.name(), batch)?;
                if !accepted && !output {
                    return Err(ModelError::Schema("admission union input undeclared"));
                }
                Ok(())
            },
        )
        .await?;
        Ok((rows, stored))
    }
}
