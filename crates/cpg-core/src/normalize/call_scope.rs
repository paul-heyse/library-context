//! Compiler input/read conveniences over the shared selected event kernel.
#[cfg(test)]
use crate::consumed_rows::PreparedClosure;
use crate::{
    consumed_rows::{ClosureTable, NominalClosure},
    workspace::CompletedInputs,
};
#[cfg(test)]
use futures::future::BoxFuture;
pub use lctx_compiler_store::normalize::call_scope::{
    BindingSelection, CallScopeMode, EventSelection,
};
#[cfg(test)]
use lctx_model::domain::normalized::{
    binding_normalization::BindingData, event_normalization::EventData,
};
use lctx_model::domain::*;
#[cfg(test)]
use std::any::TypeId;
use std::sync::Arc;
#[cfg(test)]
fn first_input<R: Record>(inputs: &[ValidationInput]) -> Option<(usize, &ValidationInput)> {
    inputs
        .iter()
        .enumerate()
        .find(|(_, i)| i.type_id() == TypeId::of::<R>())
}
pub(crate) struct CallScopes {
    inner: lctx_compiler_store::normalize::call_scope::CallScopes,
    #[cfg(test)]
    inputs: Vec<ValidationInput>,
}
impl std::ops::Deref for CallScopes {
    type Target = lctx_compiler_store::normalize::call_scope::CallScopes;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl CallScopes {
    fn wrap(inner: lctx_compiler_store::normalize::call_scope::CallScopes) -> Self {
        Self {
            #[cfg(test)]
            inputs: inner.inputs().to_vec(),
            inner,
        }
    }
    pub(super) async fn prepare(
        access: &CompletedInputs,
        session: &datafusion::prelude::SessionContext,
        model: &Arc<ValidatedModel>,
        budget: &resources::ResourceBudget,
        binding: bool,
    ) -> Result<Self, ModelError> {
        Ok(Self::wrap(
            lctx_compiler_store::normalize::call_scope::CallScopes::prepare(
                access, session, model, budget, binding,
            )
            .await?,
        ))
    }
    pub(crate) async fn from_tables_with(
        inputs: Vec<ValidationInput>,
        tables: Vec<ClosureTable>,
        session: &datafusion::prelude::SessionContext,
        budget: &resources::ResourceBudget,
        mode: CallScopeMode,
        model: &ValidatedModel,
        extra: impl FnOnce(&mut NominalClosure, &[ClosureTable]) -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        Ok(Self::wrap(
            lctx_compiler_store::normalize::call_scope::CallScopes::from_tables_with(
                inputs, tables, session, budget, mode, model, extra,
            )
            .await?,
        ))
    }
    #[cfg(test)]
    pub(crate) async fn grain<R: Record>(
        &self,
        id: Id<R>,
        budget: &resources::ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        let root = self.inner.root_for::<R>()?;
        let hex = id
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.inner
            .edges()
            .grain(root, &format!("id=X'{hex}'"), budget)
            .await
    }
    #[cfg(test)]
    pub(super) fn event_data<'a, R: Record>(
        &'a self,
        access: &'a CompletedInputs,
        root: Id<R>,
        preparation_budget: &'a resources::ResourceBudget,
        data_budget: &'a resources::ResourceBudget,
    ) -> BoxFuture<'a, Result<EventData, ModelError>> {
        Box::pin(async move {
            let grain = self.grain(root, preparation_budget).await?;
            let mut data = EventData::new(data_budget);
            load_event(self, access, &grain, &mut data).await?;
            Ok(data)
        })
    }
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

#[cfg(test)]
mod call_scope_controls {
    use super::*;
    use crate::workspace::{Workspace, WorkspaceOptions};
    use lctx_model::domain::{
        artifact::ArtifactChunk,
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
        { let mut inventory=vec![ArtifactChunk::NAME]; macro_rules! inventory {($($field:ident:$ty:ty,)*) => {$(inventory.push(<$ty>::NAME);)*};} lctx_model::normalized_event_inputs!(inventory); inventory },
        );
        macro_rules! declare {($($field:ident:$ty:ty,)*) => {$(output.declare_async::<$ty>().await.unwrap();)*};}
        lctx_model::normalized_event_inputs!(declare);
        output.declare_async::<ArtifactChunk>().await.unwrap();
        for chunk in ArtifactChunk::split(&artifact, b"call")
            .unwrap()
            .chain(ArtifactChunk::split(&unrelated, b"foreign").unwrap())
        {
            output.push(chunk).await.unwrap();
        }
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
        runtime.freeze_inputs_async(PublicationBoundary::Facts).await.unwrap();
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
        // Keep native acquisition and prepared closure reuse in their actual owner pool.
        // The separate ceiling bounds selected rich typed hydration and normalization,
        // preserving the exclusion of unrelated source payload.
        let small = resources::ResourceBudget::fixed(96 << 10).unwrap();
        let selected = scopes
            .event_data(&access, target.id(), runtime.budget(), &small)
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
        drop(error);
        drop(missing);
        drop(receivers);
        drop(selected);
        assert_eq!(small.reserved(), 0);
        drop(scopes);
        drop(session);
        drop(access);
        let budget = runtime.budget().clone();
        let native = runtime.native().clone();
        let connection = native
            .recovery_scope()
            .unwrap()
            .client()
            .connection_completion()
            .unwrap();
        runtime.drain().await.unwrap();
        native.abandon().await.unwrap();
        drop(runtime);
        drop(native);
        connection.wait().await.unwrap();
        drop(connection);
        assert_eq!(budget.reserved(), 0);
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
