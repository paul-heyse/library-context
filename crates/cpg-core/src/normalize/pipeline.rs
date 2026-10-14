//! Pure normalization routes over explicit completed workspace inputs.
use super::*;
use lctx_model::domain::{
    normalized::{
        binding_normalization, callable_normalization, entity_normalization, event_normalization,
        receiver, relation_normalization,
    },
    projection,
};
#[derive(Clone, Copy)]
pub(crate) enum Normalization {
    Entities,
    Relations,
    Callables,
    Receivers,
    CallableAspects,
    Events,
    Bindings,
    Projections,
    Coverage,
}
impl Normalization {
    /// Row-only products may substitute under the exact completed-input contract. The
    /// receiver/event/binding routes additionally return owner-private application state;
    /// coverage reads current attempt availability and completed output receipts.
    pub(crate) fn reusable_rows(self) -> bool {
        matches!(
            self,
            Self::Entities
                | Self::Relations
                | Self::Callables
                | Self::CallableAspects
                | Self::Projections
        )
    }
    pub(crate) const ALL: [Self; 9] = [
        Self::Entities,
        Self::Relations,
        Self::Callables,
        Self::Receivers,
        Self::CallableAspects,
        Self::Events,
        Self::Bindings,
        Self::Projections,
        Self::Coverage,
    ];
    pub(crate) fn declaration(self, profile: Profile) -> Stage {
        match self {
            Self::Entities => entity_normalization::stage(),
            Self::Relations => relation_normalization::stage(profile),
            Self::Callables => callable_normalization::stage(profile),
            Self::Receivers => receiver::stage(profile),
            Self::CallableAspects => {
                lctx_model::domain::normalized::callable_aspects::stage(profile)
            }
            Self::Events => event_normalization::stage(profile),
            Self::Bindings => binding_normalization::stage(profile),
            Self::Projections => projection::normalization::stage(profile),
            Self::Coverage => lctx_model::domain::normalized::coverage::stage(profile),
        }
    }
    // Erase each complete normalization stage before composing it into the compiler driver.
    // Admission and work remain lazy; this boundary allocates once per stage.
    pub(crate) fn run<'a>(
        self,
        access: CompletedInputs,
        output: ProducerOutput,
        runtime: &'a Workspace,
        model: &'a Arc<ValidatedModel>,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        Box::pin(async move {
            if self.reusable_rows()
                && let Some(outcome) = output.reuse_product().await?
            {
                return output.finish(outcome).await;
            }
            match self {
                Self::Entities => super::entities(access, output, runtime, model).await,
                Self::Relations => super::relations(access, output, runtime, model).await,
                Self::Callables => super::callables(access, output, runtime, model).await,
                Self::Receivers => super::receivers(access, output, runtime, model).await,
                Self::CallableAspects => super::aspects(access, output, runtime, model).await,
                Self::Events | Self::Bindings => Err(ModelError::Invalid(
                    "event/binding normalization requires actual predecessor authority".into(),
                )),
                Self::Projections => super::projections(access, output, runtime, model).await,
                Self::Coverage => super::coverage(access, output, runtime, model).await,
            }
        })
    }
}

#[cfg(test)]
mod stage_future_controls {
    use super::*;

    #[tokio::test]
    async fn stage_future_is_lazy_releases_owned_output_and_preserves_cancellation() {
        let model = Arc::new(model().unwrap());
        // The disconnected client makes accidental native submission fail this control.
        let native = lctx_compiler_store::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_native::surrealdb::Surreal::init()),
            serving::Name::new("normalization-laziness").unwrap(),
            serving::Name::new("normalization-laziness").unwrap(),
        );
        let runtime = Workspace::new(model.clone(), Default::default(), native).unwrap();
        for normalization in Normalization::ALL {
            let access = runtime
                .inputs("normalization-laziness", Profile::Catalog, [])
                .unwrap();
            let output = runtime.producer(
                &normalization.declaration(Profile::Catalog),
                Profile::Catalog,
                access.clone(),
            );
            let reserved = runtime.budget().reserved();
            let owners = Arc::strong_count(&runtime);
            let operation = normalization.run(access, output, &runtime, &model);
            assert_eq!(runtime.budget().reserved(), reserved);
            assert_eq!(Arc::strong_count(&runtime), owners);
            drop(operation);
            assert_eq!(runtime.budget().reserved(), reserved);
            assert_eq!(Arc::strong_count(&runtime), owners - 1);
        }

        let normalization = Normalization::Entities;
        let access = runtime
            .inputs("normalization-cancellation", Profile::Catalog, [])
            .unwrap();
        let output = runtime.producer(
            &normalization.declaration(Profile::Catalog),
            Profile::Catalog,
            access.clone(),
        );
        let reserved = runtime.budget().reserved();
        let owners = Arc::strong_count(&runtime);
        let operation = normalization.run(access, output, &runtime, &model);
        runtime.cancellation().cancel();
        assert!(
            matches!(operation.await, Err(ModelError::Invalid(message)) if message == "compilation cancelled")
        );
        assert_eq!(runtime.budget().reserved(), reserved);
        assert_eq!(Arc::strong_count(&runtime), owners - 1);
    }
}
