//! Complete callable/descriptor and native-origin candidate groups selected before rich decode.
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges, identifier},
    workspace::CompletedInputs,
};
use arrow_array::Array;
pub(super) use lctx_model::domain::normalized::normalization_scope_program::CallableKernel as Kernel;
use lctx_model::domain::{
    calls::Signature,
    normalized::{callable_normalization::CallableData, entities::CallableEntity},
    *,
};
use std::{any::TypeId, sync::Arc};
pub(super) fn nominal<R>(bytes: &[u8]) -> Result<Id<R>, ModelError> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new(bytes.iter().copied()))
    .map_err(ModelError::codec)
}
pub(super) struct CallableScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    signature_root: Option<usize>,
    program: ContentHash,
    _charge: charged::StateCharge,
}

impl CallableScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
        kernel: Kernel,
    ) -> Result<Self, ModelError> {
        let inputs = CallableData::validation_inputs();
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
        Self::prepare_bound(inputs, tables, session, budget, kernel, model).await
    }
    async fn prepare_bound(
        inputs: Vec<ValidationInput>,
        mut tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        kernel: Kernel,
        model: &ValidatedModel,
    ) -> Result<Self, ModelError> {
        let input_relations = tables
            .iter()
            .map(|table| table.relation.clone())
            .collect::<Vec<_>>();
        let program = normalized::normalization_scope_program::callable(
            inputs.clone(),
            &input_relations,
            kernel,
            budget,
        )?;
        let signature_root = program.signature_root();
        if signature_root.is_some() {
            let source = inputs
                .iter()
                .position(|input| input.type_id() == TypeId::of::<Signature>())
                .ok_or(ModelError::Schema("signature demand binding"))?;
            tables.push(tables[source].clone());
        }
        let compiled = crate::scope_compilation::compile(program.program(), model, budget, None)?;
        let identity = compiled.identity();
        let plan = crate::scope_compilation::lower_compiled(
            compiled,
            &tables,
            &scope_program::ScopeParameters(vec![]),
            budget,
        )?;
        let mut charge = charged::StateCharge::new(budget, "callable-scope-descriptors");
        charge.grow(
            inputs.capacity() * size_of::<ValidationInput>()
                + tables.capacity() * size_of::<ClosureTable>()
                + tables
                    .iter()
                    .map(|table| table.alias.capacity())
                    .sum::<usize>(),
        )?;
        let edges = plan.prepare(session, budget).await?;
        Ok(Self {
            inputs,
            tables,
            edges,
            signature_root,
            program: identity,
            _charge: charge,
        })
    }
}

/// Necessary owner admission uses the same candidate selector as production, but compares only
/// the exact advertised family of each actual callable. Global membership probes prevent an
/// unsupported advertised root or an unowned premise from disappearing from selected work.
pub(super) async fn validate_callables(
    invariant: &Invariant,
    tables: Vec<ClosureTable>,
    session: &datafusion::prelude::SessionContext,
    budget: &resources::ResourceBudget,
    cancellation: &crate::workspace::Cancellation,
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    use futures::TryStreamExt;
    use normalized::{callable_normalization, callables::*};
    let index = |kind: TypeId| {
        tables
            .iter()
            .position(|table| table.relation.type_id() == kind)
            .ok_or(ModelError::Schema("callable admission input"))
    };
    let table = |kind: TypeId| -> Result<String, ModelError> {
        Ok(identifier(&tables[index(kind)?].alias))
    };
    let callable = table(TypeId::of::<CallableEntity>())?;
    let assessment = table(TypeId::of::<EffectiveCallableAssessment>())?;
    let decorator = table(TypeId::of::<EffectiveDecoratorMember>())?;
    let premise = table(TypeId::of::<EffectiveCallablePremise>())?;
    let evidence = table(TypeId::of::<EffectiveCallableEvidence>())?;
    for query in [
        format!(
            "SELECT a.id FROM {assessment} a LEFT ANTI JOIN {callable} c ON c.id=a.callable LIMIT 1"
        ),
        format!(
            "SELECT d.id FROM {decorator} d LEFT ANTI JOIN {assessment} a ON a.id=d.assessment LIMIT 1"
        ),
        format!(
            "SELECT e.id FROM {evidence} e LEFT ANTI JOIN {assessment} a ON a.id=e.assessment LIMIT 1"
        ),
        format!(
            "SELECT e.id FROM {evidence} e LEFT ANTI JOIN {premise} p ON p.id=e.premise LIMIT 1"
        ),
        format!(
            "SELECT p.id FROM {premise} p LEFT ANTI JOIN {evidence} e ON e.premise=p.id LIMIT 1"
        ),
    ] {
        let mut rows = crate::sql::query(session, &query)
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
            cancellation.check()?;
            if batch.num_rows() != 0 {
                return Err(ModelError::Invalid(
                    "callable advertised family lacks actual root or membership".into(),
                ));
            }
        }
    }
    let root = index(TypeId::of::<CallableEntity>())?;
    let prepared = CallableScopes::prepare_bound(
        invariant.inputs.clone(),
        tables,
        session,
        budget,
        Kernel::Callable,
        model,
    )
    .await?;
    let mut roots = crate::sql::query(session, &format!("SELECT id FROM {callable} ORDER BY id"))
        .await
        .map_err(crate::sql::model_error)?
        .execute_stream()
        .await
        .map_err(crate::sql::model_error)?;
    while let Some(batch) = roots.try_next().await.map_err(crate::sql::model_error)? {
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("callable admission roots"))?;
        for start in (0..ids.len()).step_by(32) {
            let end = (start + 32).min(ids.len());
            let _root_charge =
                budget.reserve("callable-admission-window", (end - start) * 128 + 4096)?;
            let roots = (start..end)
                .map(|index| {
                    Ok(crate::consumed_rows::PreparedRoot {
                        table: root,
                        key: ids.value(index).try_into().map_err(ModelError::codec)?,
                        kind: crate::consumed_rows::PreparedRootKind::Physical,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let selected = prepared
                .edges
                .batch_with_cancellation(&roots, budget, cancellation)
                .await?;
            let (data, stored) = prepared
                .load_admission(&selected, budget, cancellation)
                .await?;
            for (partition, index) in (start..end).enumerate() {
                let inputs =
                    DataSelection::new(&selected, partition, &prepared.inputs, &data, budget)?;
                let owner = nominal(ids.value(index))?;
                let outputs = callable_normalization::CallableOwnerSelection::new(&stored, owner, budget)?;
                callable_normalization::admit_callable_view(
                    &inputs.view()?,
                    &outputs.view()?,
                    owner,
                    budget,
                )?;
            }
        }
    }
    Ok(())
}

// Partition dictionaries borrow the one decoded root union, including unavailable optional inputs.
macro_rules! selected_inputs {($($field:ident:$ty:ty,)*)=>{
    pub(super) struct DataSelection<'a>{rows:&'a CallableData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> DataSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a CallableData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::callable_normalization::CallableDataView<'_>,ModelError>{Ok(normalized::callable_normalization::CallableDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_callable_inputs!(selected_inputs);
impl CallableScopes {
    pub(super) async fn load_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<CallableData, ModelError> {
        let mut rows = CallableData::new(budget);
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
    pub(super) fn program(&self) -> ContentHash { self.program }
    pub(super) fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    pub(super) fn signature_root(&self) -> Option<usize> {
        self.signature_root
    }
    pub(super) fn edges(&self) -> &PreparedEdges {
        &self.edges
    }
    pub(super) fn table_for<R: Record>(&self) -> Result<usize, ModelError> {
        self.tables
            .iter()
            .position(|t| t.relation.type_id() == TypeId::of::<R>())
            .ok_or(ModelError::Schema("normalization batch root absent"))
    }
}

impl CallableScopes {
    async fn load_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
    ) -> Result<
        (
            CallableData,
            normalized::callable_normalization::CallableOutput,
        ),
        ModelError,
    > {
        let mut rows = CallableData::new(budget);
        let mut stored = normalized::callable_normalization::CallableOutput::new(budget);
        crate::scoped_batch::hydrate_union(
            batch,
            &self.inputs,
            budget,
            cancellation,
            &mut |_, input, batch| {
                let accepted = rows.visit(input.name(), batch)?;
                let output = stored.visit(input.name(), batch)?;
                if !accepted && !output {
                    return Err(ModelError::Schema("callable admission union input"));
                }
                Ok(())
            },
        )
        .await?;
        Ok((rows, stored))
    }
}
