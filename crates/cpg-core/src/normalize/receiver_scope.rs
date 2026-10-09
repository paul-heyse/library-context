//! Receiver applicability owns one target and its complete syntax/placement candidate domains.
use crate::{
    consumed_rows::{ClosureTable, PreparedEdges},
    workspace::CompletedInputs,
};
#[cfg(test)]
use futures::future::BoxFuture;
#[cfg(test)]
use lctx_model::domain::calls::*;
use lctx_model::domain::{normalized::receiver::ReceiverData, *};
use std::{any::TypeId, sync::Arc};

pub(super) struct ReceiverScopes {
    inputs: Vec<ValidationInput>,
    tables: Vec<ClosureTable>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}

#[cfg(test)]
fn load_receiver_data<'a>(
    descriptor: &'a ReceiverScopes,
    access: &'a CompletedInputs,
    scope: &'a crate::consumed_rows::PreparedClosure,
    data: &'a mut ReceiverData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a ReceiverScopes,
        &'a CompletedInputs,
        &'a crate::consumed_rows::PreparedClosure,
        &'a mut ReceiverData,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(descriptor: &'a ReceiverScopes, access: &'a CompletedInputs, scope: &'a crate::consumed_rows::PreparedClosure, data: &'a mut ReceiverData) -> BoxFuture<'a, Result<(), ModelError>> {
            Box::pin(async move {
                if let Some((table, input)) = descriptor.inputs.iter().enumerate().find(|(_, input)| input.type_id() == TypeId::of::<$ty>()) {
                    let permit = access.read_at::<$ty>(input.prefix())?;
                    crate::consumed_rows::stream_query_at(&permit, input, access, scope.session(), &scope.select(table)?, |_, batch| data.$field.decode(batch)).await?;
                }
                Ok(())
            })
        })*
        const LOADERS: &[Loader] = &[$($field,)*];
    };}
    lctx_model::normalized_receiver_inputs!(adapters);
    Box::pin(async move {
        for load in LOADERS {
            load(descriptor, access, scope, data).await?;
        }
        Ok(())
    })
}

impl ReceiverScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let inputs: Vec<_> = ReceiverData::validation_inputs()
            .into_iter()
            .filter(|input| {
                access
                    .relations()
                    .any(|relation| relation.name() == input.name())
            })
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
    pub(super) async fn from_tables(
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
    pub(super) fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    #[cfg(test)]
    pub(super) fn data<'a>(
        &'a self,
        access: &'a CompletedInputs,
        target: Id<CallTarget>,
        budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<ReceiverData, ModelError>> {
        Box::pin(async move {
            let root = self
                .tables
                .iter()
                .position(|table| table.relation.type_id() == TypeId::of::<CallTarget>())
                .ok_or_else(|| ModelError::Invalid("receiver root table absent".into()))?;
            let bytes = target
                .bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let scope = self
                .edges
                .grain(root, &format!("id=X'{bytes}'"), budget)
                .await?;
            let mut data = ReceiverData::new(budget);
            load_receiver_data(self, access, &scope, &mut data).await?;
            Ok(data)
        })
    }
}

#[cfg(test)]
mod receiver_scope_controls {
    use super::*;
    use crate::workspace::{Workspace, WorkspaceOptions};
    use lctx_model::domain::{
        assertion::*, attribution::*, normalized::receiver, source::*, stages::*,
    };
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn target_scope_preserves_conflicting_candidates_and_excludes_foreign_source_payload() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(
            model.clone(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let artifact =
            SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"value").unwrap();
        let unrelated = SourceArtifact::from_bytes(
            nominal(2),
            format!("{}.py", "x".repeat(128 << 10)),
            b"foreign",
        )
        .unwrap();
        let occurrence = |start, kind| Occurrence {
            source: artifact.id(),
            start,
            end: start + 1,
            syntax_kind: kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![start as i32],
        };
        let site = occurrence(0, SyntaxKind::ExprCall);
        let callee = occurrence(1, SyntaxKind::ExprAttribute);
        let first = occurrence(2, SyntaxKind::ExprName);
        let second = occurrence(3, SyntaxKind::ExprName);
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let q = AssertionQualification {
            context: nominal(3),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let foreign_q = AssertionQualification {
            context: nominal(4),
            ..q.clone()
        };
        let receiver = Receiver::Unknown {
            reason: obligation::ObligationKind::AmbiguousBinding,
        };
        let destination = CallDestination::Unresolved {
            reason: obligation::ObligationKind::UnresolvedTarget,
            native: None,
        };
        let target = CallTarget {
            qualification: q.id(),
            site: site.id(),
            origin: CallOrigin::explicit(),
            destination: destination.id(),
            channel: CallChannel::Direct.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            receiver_class: None,
            passing: Some(ReceiverPassing::Object),
            class_method: Some(true),
            static_method: Some(false),
        };
        let (call, _) = CallSyntax::new(q.id(), site.id(), callee.id(), false, &[]).unwrap();
        let other_call = CallSyntax {
            in_annotation: true,
            ..call.clone()
        };
        let foreign_call = CallSyntax {
            qualification: foreign_q.id(),
            ..call.clone()
        };
        let placement = syntax::SyntaxPlacement {
            qualification: q.id(),
            occurrence: first.id(),
            parent: Some(callee.id()),
            field: lexical::SyntaxField::Value,
            ordinal: 0,
        };
        let conflict = syntax::SyntaxPlacement {
            qualification: foreign_q.id(),
            occurrence: second.id(),
            ..placement.clone()
        };
        let input = runtime
            .inputs("receiver-scope-facts", Profile::Catalog, [])
            .unwrap();
        let output = runtime.output(
            "receiver-scope-facts",
            Profile::Catalog,
            ContentHash::of(b"receiver-scope-control"),
            input,
        { let mut inventory=Vec::new(); macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(inventory.push(<$ty>::NAME);)*};} lctx_model::normalized_receiver_inputs!(inventory); inventory },
        );
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>().unwrap();)*};}
        lctx_model::normalized_receiver_inputs!(declare);
        macro_rules! push {($($row:expr),* $(,)?) => {$(output.push($row).await.unwrap();)*};}
        push!(
            artifact.clone(),
            unrelated,
            site,
            callee,
            first,
            second,
            scope,
            q,
            foreign_q,
            receiver,
            destination,
            target.clone(),
            call.clone(),
            other_call.clone(),
            foreign_call,
            placement.clone(),
            conflict.clone()
        );
        output.finish(ProviderOutcome::Complete).await.unwrap();
        runtime.freeze_inputs(PublicationBoundary::Facts).unwrap();
        let mut declaration = receiver::stage(Profile::Catalog);
        declaration.inputs = ReceiverData::stage_inputs()
            .into_iter()
            .map(|input| {
                if is_vocabulary(input.name()) {
                    input.at_epoch(PublicationBoundary::Facts)
                } else {
                    input
                }
            })
            .collect();
        let access = runtime
            .stage_inputs(&declaration, Profile::Catalog)
            .unwrap();
        let session = access.session(&runtime).await.unwrap();
        let scopes = ReceiverScopes::prepare(&access, &session, &model, runtime.budget())
            .await
            .unwrap();
        let small = resources::ResourceBudget::fixed(32 << 10).unwrap();
        let selected = scopes.data(&access, target.id(), &small).await.unwrap();
        assert_eq!(selected.targets.len(), 1);
        assert_eq!(selected.artifacts.len(), 1);
        assert_eq!(selected.artifacts.get(artifact.id()), Some(&artifact));
        assert_eq!(selected.syntax.len(), 2);
        assert!(selected.syntax.get(call.id()).is_some());
        assert!(selected.syntax.get(other_call.id()).is_some());
        assert_eq!(selected.placements.len(), 2);
        assert!(selected.placements.get(placement.id()).is_some());
        assert!(selected.placements.get(conflict.id()).is_some());
        let records = receiver::normalize_target(&selected, target.id(), &small).unwrap();
        assert_eq!(records.receiver_assessments.len(), 1);
        assert!(matches!(
            records.receiver_assessments.iter().next(),
            Some(receiver::ReceiverAssessment::Unknown {
                reason: receiver::ReceiverReason::DescriptorUnknown,
                ..
            })
        ));
    }
}

// Partition dictionaries borrow the one decoded root union, including unavailable optional inputs.
macro_rules! selected_inputs {($($field:ident:$ty:ty,)*)=>{
    pub(super) struct DataSelection<'a>{rows:&'a ReceiverData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> DataSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a ReceiverData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::receiver::ReceiverDataView<'_>,ModelError>{Ok(normalized::receiver::ReceiverDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_receiver_inputs!(selected_inputs);
impl ReceiverScopes {
    pub(super) async fn load_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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

macro_rules! selected_Receiver_output {($($field:ident:$ty:ty,)*)=>{
    pub(super) struct ReceiverOutputSelection<'a>{rows:&'a normalized::receiver::ReceiverOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> ReceiverOutputSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::receiver::ReceiverOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::receiver::ReceiverOutputView<'_>,ModelError>{Ok(normalized::receiver::ReceiverOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_receiver_outputs!(selected_Receiver_output);
impl ReceiverScopes {
    pub(super) async fn load_receiver_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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
