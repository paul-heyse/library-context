//! Complete event candidate and binding-set grains over immutable typed premises.
#[cfg(test)]
use crate::consumed_rows::PreparedClosure;
use crate::{
    consumed_rows::{ClosureTable, NominalClosure, PreparedEdges},
    workspace::CompletedInputs,
};
#[cfg(test)]
use futures::future::BoxFuture;
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

#[cfg(test)]
fn read_first<'a, R: Record>(
    scopes: &'a CallScopes,
    access: &'a CompletedInputs,
    grain: &'a PreparedClosure,
    visit: impl FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError> + Send + 'a,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut visit = visit;
        if let Some((table, input)) = first_input::<R>(&scopes.inputs) {
            let permit = access.read_at::<R>(input.prefix())?;
            crate::consumed_rows::stream_query_at(
                &permit,
                input,
                access,
                grain.session(),
                &grain.select(table)?,
                |_, batch| visit(batch),
            )
            .await?;
        }
        Ok(())
    })
}

#[cfg(test)]
fn load_event<'a>(
    call_scopes: &'a CallScopes,
    access: &'a CompletedInputs,
    grain: &'a PreparedClosure,
    data: &'a mut EventData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a CallScopes,
        &'a CompletedInputs,
        &'a PreparedClosure,
        &'a mut EventData,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(call_scopes: &'a CallScopes, access: &'a CompletedInputs, grain: &'a PreparedClosure, data: &'a mut EventData) -> BoxFuture<'a, Result<(), ModelError>> {
            read_first::<$ty>(call_scopes, access, grain, |batch| data.$field.decode(batch))
        })*
        const LOADERS: &[Loader] = &[$($field,)*];
    };}
    lctx_model::normalized_event_inputs!(adapters);
    Box::pin(async move {
        for load in LOADERS {
            load(call_scopes, access, grain, data).await?;
        }
        Ok(())
    })
}

/// Candidate-family and admission policy passed together to the model scope declaration.
pub(crate) struct CallScopeMode {
    pub(crate) binding: bool,
    pub(crate) admission: bool,
}
pub(crate) struct CallScopes {
    inputs: Vec<ValidationInput>,
    event_roots: Vec<(TypeId, usize)>,
    edges: PreparedEdges,
    _charge: charged::StateCharge,
}
impl CallScopes {
    pub(super) async fn prepare(
        access: &CompletedInputs,
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
    pub(super) async fn from_tables(
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
    pub(crate) async fn from_tables_with(
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
    pub(crate) fn edges(&self) -> &PreparedEdges {
        &self.edges
    }
    pub(crate) fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    #[cfg(test)]
    pub(crate) async fn grain<R: Record>(
        &self,
        id: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let root = self
            .event_roots
            .iter()
            .find(|(kind, _)| *kind == TypeId::of::<R>())
            .ok_or(ModelError::Schema(R::NAME))?
            .1;
        let hex = id
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.edges
            .grain(root, &format!("id=X'{hex}'"), budget)
            .await
    }
    #[cfg(test)]
    pub(super) fn event_data<'a, R: Record>(
        &'a self,
        access: &'a CompletedInputs,
        root: Id<R>,
        budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<EventData, ModelError>> {
        Box::pin(async move {
            let grain = self.grain(root, budget).await?;
            let mut data = EventData::new(budget);
            load_event(self, access, &grain, &mut data).await?;
            Ok(data)
        })
    }
}

#[cfg(test)]
mod call_scope_controls {
    use super::*;
    use crate::workspace::{Workspace, WorkspaceOptions};
    use lctx_model::domain::{
        assertion::*,
        attribution::*,
        calls::*,
        normalized::{entities::*, event_normalization, receiver},
        source::*,
        stages::*,
    };
    #[test]
    fn normalization_construction_is_unique_and_wider_scopes_select_first_epoch() {
        for declarations in [
            EventData::validation_inputs(),
            BindingData::validation_inputs(),
        ] {
            let mut nominal = std::collections::HashSet::new();
            for declaration in declarations {
                assert!(
                    nominal.insert(declaration.type_id()),
                    "{} repeats",
                    declaration.name()
                );
            }
        }
        let declarations = [
            ValidationInput::of::<calls::CallTarget>(&["id"]),
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(stages::PublicationBoundary::Facts),
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(stages::PublicationBoundary::Summary),
        ];
        let (table, selected) =
            first_input::<assertion::AssertionQualification>(&declarations).unwrap();
        assert_eq!(table, 1);
        assert_eq!(selected.prefix(), Some(stages::PublicationBoundary::Facts));
        assert!(first_input::<calls::CallResolution>(&declarations).is_none());
    }
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[tokio::test]
    async fn event_grain_keeps_orphans_and_declared_skew_without_unrelated_source_payload() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(
            model.clone(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let artifact =
            SourceArtifact::from_bytes(nominal(1), "selected.py".into(), b"call").unwrap();
        let unrelated = SourceArtifact::from_bytes(
            nominal(2),
            format!("{}.py", "x".repeat(128 << 10)),
            b"foreign",
        )
        .unwrap();
        let site = Occurrence {
            source: artifact.id(),
            start: 0,
            end: 4,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        };
        let scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        let qualification = AssertionQualification {
            context: nominal(3),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let foreign = AssertionQualification {
            context: nominal(4),
            ..qualification.clone()
        };
        let destination = CallDestination::Unresolved {
            reason: obligation::ObligationKind::UnresolvedTarget,
            native: None,
        };
        let receiver = Receiver::Unknown {
            reason: obligation::ObligationKind::AmbiguousBinding,
        };
        let target = CallTarget {
            qualification: qualification.id(),
            site: site.id(),
            origin: CallOrigin::explicit(),
            destination: destination.id(),
            channel: CallChannel::Direct.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            receiver_class: None,
            passing: None,
            class_method: None,
            static_method: None,
        };
        let orphan = CallTarget {
            phase: CallPhase::Init,
            ..target.clone()
        };
        let skew = CallTarget {
            qualification: foreign.id(),
            ..target.clone()
        };
        let (resolution, _) = CallResolution::new(
            &qualification,
            site.id(),
            target.origin,
            target.channel,
            target.phase,
            true,
            std::slice::from_ref(&target),
        )
        .unwrap();
        let member = CallResolutionMember {
            resolution: resolution.id(),
            target: skew.id(),
        };
        let support = CallResolutionSupport {
            assertion: resolution.id(),
            run: nominal(5),
            surface: nominal(6),
            evidence: nominal(7),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        let entity = EntityRef::Occurrence {
            occurrence: site.id(),
        };
        let owner = OccurrenceOwnership {
            occurrence: site.id(),
            owner: site.id(),
            entity: entity.id(),
        };
        let access = runtime
            .inputs("call-scope-facts", Profile::Catalog, [])
            .unwrap();
        let output = runtime.output(
            "call-scope-facts",
            Profile::Catalog,
            ContentHash::of(b"call-scope-control"),
            access,
        { let mut inventory=Vec::new(); macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(inventory.push(<$ty>::NAME);)*};} lctx_model::normalized_event_inputs!(inventory); inventory },
        );
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare::<$ty>().unwrap();)*};}
        lctx_model::normalized_event_inputs!(declare);
        macro_rules! push {($($row:expr),* $(,)?) => {$(output.push($row).await.unwrap();)*};}
        push!(
            artifact.clone(),
            unrelated,
            site,
            scope,
            qualification.clone(),
            foreign,
            destination,
            receiver,
            CallChannel::Direct,
            target.clone(),
            orphan.clone(),
            skew.clone(),
            resolution.clone(),
            member.clone(),
            support,
            entity,
            owner
        );
        output.finish(ProviderOutcome::Complete).await.unwrap();
        runtime.freeze_inputs(PublicationBoundary::Facts).unwrap();
        let mut declaration = event_normalization::stage(Profile::Catalog);
        declaration.inputs = EventData::stage_inputs()
            .into_iter()
            .filter(|input| {
                input.name() != flow::FlowValuePathObservation::NAME
                    && input.name() != flow::FlowCallStep::NAME
            })
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
        let mut scopes = CallScopes::prepare(&access, &session, &model, runtime.budget(), false)
            .await
            .unwrap();
        // A wider caller can retain a later declaration. It is deliberately unavailable to
        // this access/closure: touching it would either fail admission or select no table.
        // The first facts declaration alone must feed the normalization data.
        scopes.inputs.push(
            ValidationInput::of::<AssertionQualification>(&["id"])
                .at_epoch(PublicationBoundary::Summary),
        );
        let small = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let selected = scopes
            .event_data(&access, target.id(), &small)
            .await
            .unwrap();
        assert_eq!(selected.artifacts.len(), 1);
        assert_eq!(selected.artifacts.get(artifact.id()), Some(&artifact));
        assert!(selected.targets.get(orphan.id()).is_some());
        assert!(selected.targets.get(skew.id()).is_some());
        assert_eq!(selected.resolutions.get(resolution.id()), Some(&resolution));
        assert_eq!(selected.members.get(member.id()), Some(&member));
        assert_eq!(
            selected.qualifications.get(qualification.id()),
            Some(&qualification)
        );
        let receivers = receiver::normalize_produced(&receiver::ReceiverData::new(&small), &small)
            .unwrap()
            .1;
        let missing = event_normalization::admit_event(
            &selected,
            &event_normalization::EventOutput::new(&small),
            (target.site, target.origin, qualification.context),
            &small,
        )
        .unwrap_err();
        assert!(
            missing
                .to_string()
                .contains("no required normalized outcome"),
            "{missing}"
        );
        let error = event_normalization::normalize_event_produced(
            &selected,
            (target.site, target.origin, qualification.context),
            &receivers,
            &small,
        )
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("crosses qualification context or scope"),
            "{error}"
        );
    }
    #[test]
    fn empty_owner_indices_are_not_interchangeable_across_attempt_budgets() {
        let first = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let second = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut receiver =
            receiver::normalize_produced(&receiver::ReceiverData::new(&first), &first)
                .unwrap()
                .1;
        let foreign = receiver::normalize_produced(&receiver::ReceiverData::new(&second), &second)
            .unwrap()
            .1;
        assert!(receiver.append(foreign).is_err());
        let mut event = event_normalization::normalize_events_produced(
            &EventData::new(&first),
            &receiver,
            &first,
        )
        .unwrap()
        .1;
        let foreign_receiver =
            receiver::normalize_produced(&receiver::ReceiverData::new(&second), &second)
                .unwrap()
                .1;
        let foreign = event_normalization::normalize_events_produced(
            &EventData::new(&second),
            &foreign_receiver,
            &second,
        )
        .unwrap()
        .1;
        assert!(event.append(foreign).is_err());
    }
}

impl CallScopes {
    pub(crate) fn root_for<R: Record>(&self) -> Result<usize, ModelError> {
        self.event_roots
            .iter()
            .find(|(kind, _)| *kind == TypeId::of::<R>())
            .map(|(_, root)| *root)
            .ok_or(ModelError::Schema("call batch root absent"))
    }
}

macro_rules! selected_Event {($($field:ident:$ty:ty,)*)=>{
    pub(super) struct EventSelection<'a>{rows:&'a EventData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> EventSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a EventData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:first_input::<$ty>(inputs).map(|(table,_)|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::event_normalization::EventDataView<'_>,ModelError>{Ok(normalized::event_normalization::EventDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_event_inputs!(selected_Event);
impl CallScopes {
    pub(super) async fn load_event_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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
    pub(super) struct BindingSelection<'a>{rows:&'a BindingData,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> BindingSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a BindingData,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:first_input::<$ty>(inputs).map(|(table,_)|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::binding_normalization::BindingDataView<'_>,ModelError>{Ok(normalized::binding_normalization::BindingDataView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_binding_inputs!(selected_Binding);
impl CallScopes {
    pub(super) async fn load_binding_batch(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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
    pub(super) struct EventOutputSelection<'a>{rows:&'a normalized::event_normalization::EventOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> EventOutputSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::event_normalization::EventOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::event_normalization::EventOutputView<'_>,ModelError>{Ok(normalized::event_normalization::EventOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_event_outputs!(selected_Event_output);
impl CallScopes {
    pub(super) async fn load_event_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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
    pub(super) struct BindingOutputSelection<'a>{rows:&'a normalized::binding_normalization::BindingOutput,$($field:Option<crate::scoped_batch::SelectedRows<'a,$ty>>,)*}
    impl<'a> BindingOutputSelection<'a>{
        pub(super) fn new(batch:&crate::consumed_rows::PreparedRootBatch,partition:usize,inputs:&[ValidationInput],rows:&'a normalized::binding_normalization::BindingOutput,budget:&resources::ResourceBudget)->Result<Self,ModelError>{
            Ok(Self{rows,$($field:inputs.iter().position(|i|i.type_id()==TypeId::of::<$ty>()).map(|table|crate::scoped_batch::SelectedRows::new(batch,partition,table,&rows.$field,budget)).transpose()?,)*})
        }
        pub(super) fn view(&self)->Result<normalized::binding_normalization::BindingOutputView<'_>,ModelError>{Ok(normalized::binding_normalization::BindingOutputView{$($field:match &self.$field{Some(selected)=>selected.view()?,None=>self.rows.$field.view()},)*})}
    }
};}
lctx_model::normalized_binding_outputs!(selected_Binding_output);
impl CallScopes {
    pub(super) async fn load_binding_admission(
        &self,
        batch: &crate::consumed_rows::PreparedRootBatch,
        budget: &resources::ResourceBudget,
        cancellation: &crate::workspace::Cancellation,
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
