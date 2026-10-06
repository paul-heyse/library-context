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
    pub(crate) async fn run(
        self,
        access: CompletedInputs,
        output: ProducerOutput,
        runtime: &Workspace,
        model: &Arc<ValidatedModel>,
    ) -> Result<(), ModelError> {
        match self {
            Self::Entities => super::entities(access, output, runtime, model).await,
            Self::Relations => super::relations(access, output, runtime, model).await,
            Self::Callables => super::callables(access, output, runtime, model).await,
            Self::Receivers => super::receivers(access, output, runtime, model).await,
            Self::CallableAspects => {
                crate::catalog_core::aspects(access, output, runtime, model).await
            }
            Self::Events => super::events(access, output, runtime, model).await,
            Self::Bindings => super::bindings(access, output, runtime, model).await,
            Self::Projections => super::projections(access, output, runtime, model).await,
            Self::Coverage => super::coverage(access, output, runtime, model).await,
        }
    }
}
