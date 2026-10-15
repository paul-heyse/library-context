//! Receiver applicability owns one target and its complete syntax/placement candidate domains.
use crate::{
    admission::InputTables,
    consumed_rows::{ClosureTable, PreparedEdges},
};
use lctx_model::domain::{normalized::receiver::ReceiverData, *};
use std::{any::TypeId, sync::Arc};

pub struct ReceiverScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}

impl ReceiverScopes {
    pub async fn prepare(
        access: &impl InputTables,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = ReceiverData::validation_inputs()
            .into_iter()
            .filter(|input| access.contains_relation(input.name()))
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
        Self::from_tables(inputs, tables, session, budget, model).await
    }
    pub async fn from_tables(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program =
            normalized::normalization_scope_program::receiver(inputs.clone(), &relations, budget)?;
        let plan = crate::scope_compilation::lower_compiled(
            crate::scope_compilation::compile(program.program(), model, budget, None)?,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let mut charge = charged::StateCharge::new(budget, "receiver-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            _charge: charge,
        })
    }
    pub fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
}

// Partition dictionaries borrow the one decoded root union, including unavailable optional inputs.
macro_rules! selected_inputs {($($field:ident:$ty:ty,)*)=>{
    pub struct DataSelection<'a>{rows:&'a ReceiverData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> DataSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a ReceiverData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::receiver::ReceiverDataView<'_>,ModelError>{Ok(normalized::receiver::ReceiverDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_receiver_inputs!(selected_inputs);
impl ReceiverScopes {
    pub async fn load_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<ReceiverData, ModelError> {
        let mut rows = ReceiverData::new(budget);
        crate::scoped_batch::hydrate_union(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                if !rows.visit(input.name(), batch)? {
                    return Err(ModelError::Schema("normalization union input undeclared"));
                }
                Ok(())
            },
        )
        .await?;
        Ok(rows)
    }
    pub fn edges(&self) -> &PreparedEdges {
        &self.edges
    }
    pub fn table_for<R: Record>(&self) -> Result<usize, ModelError> {
        self.tables
            .iter()
            .position(|t| t.relation.type_id() == TypeId::of::<R>())
            .ok_or(ModelError::Schema("normalization batch root absent"))
    }
}

macro_rules! selected_Receiver_output {($($field:ident:$ty:ty,)*)=>{
    pub struct ReceiverOutputSelection<'a>{rows:&'a normalized::receiver::ReceiverOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> ReceiverOutputSelection<'a>{
        pub fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::receiver::ReceiverOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub fn view(&self)->Result<normalized::receiver::ReceiverOutputView<'_>,ModelError>{Ok(normalized::receiver::ReceiverOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_receiver_outputs!(selected_Receiver_output);
impl ReceiverScopes {
    pub async fn load_receiver_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::admission::Cancellation,
    ) -> Result<(ReceiverData, normalized::receiver::ReceiverOutput), ModelError> {
        let mut rows = ReceiverData::new(budget);
        let mut stored = normalized::receiver::ReceiverOutput::new(budget);
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
