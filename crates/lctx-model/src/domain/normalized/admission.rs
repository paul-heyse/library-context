//! Closed physical routes for necessary normalized owner predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Callables,
    Receivers,
    Events,
    Bindings,
}
impl Scope {
    /// Actual publication roots determine applicability before unused dependency aliases load.
    /// Output roots remain included even when no predecessor candidate was eligible.
    pub fn roots(self) -> Vec<super::super::ValidationInput> {
        use super::super::*;
        let mut roots = Vec::new();
        macro_rules! add {
            ($ty:ty) => {
                roots.push(ValidationInput::of::<$ty>(&["id"]));
            };
        }
        match self {
            Self::Callables => {
                add!(super::entities::CallableEntity);
                add!(calls::Signature);
                add!(types::NativeOverloadObservation);
                add!(super::callables::EffectiveCallableAssessment);
                add!(super::callables::EffectiveDecoratorMember);
                add!(super::callables::EffectiveCallablePremise);
                add!(super::callables::EffectiveCallableEvidence);
            }
            Self::Receivers => {
                add!(calls::CallTarget);
                macro_rules! outputs {($($field:ident:$ty:ty,)*) => {$(add!($ty);)*};}
                crate::normalized_receiver_outputs!(outputs);
            }
            Self::Events => {
                add!(calls::ProviderCallSite);
                add!(calls::CallTarget);
                add!(calls::CallResolution);
                add!(flow::FlowValuePathObservation);
                macro_rules! outputs {($($field:ident:$ty:ty,)*) => {$(add!($ty);)*};}
                crate::normalized_event_outputs!(outputs);
            }
            Self::Bindings => {
                add!(super::events::NormalizedCallEvent);
                // Generic binding source/projection vocabulary is shared with later owners;
                // applicability follows the frames and their owned member families.
                add!(super::bindings::CallBindingAttempt);
                add!(super::bindings::CallBinding);
                add!(super::bindings::BindingSetAssessment);
                add!(super::bindings::BindingVariantAssessment);
                add!(super::bindings::BindingSetMember);
                add!(super::bindings::BindingSetCoverage);
            }
        }
        roots
    }
}
