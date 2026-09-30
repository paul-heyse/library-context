//! Focused N3 premises. Other validators retain their own read-eligibility closure.
#[macro_export]
macro_rules! normalized_callable_inputs {
    ($apply:ident) => { $apply! {
        occurrences: $crate::domain::source::Occurrence,
        qualifications: $crate::domain::assertion::AssertionQualification,
        coverage: $crate::domain::attribution::ProviderCoverage,
        scopes: $crate::domain::source::CoverageScope,
        declarations: $crate::domain::syntax::DeclarationObservation,
        decorators: $crate::domain::syntax::DeclarationDecorator,
        placements: $crate::domain::syntax::SyntaxPlacement,
        traits: $crate::domain::symbols::FunctionTraitObservation,
        bodies: $crate::domain::types::FunctionBodyObservation,
        signatures: $crate::domain::calls::Signature,
        parameters: $crate::domain::calls::SignatureParameter,
        shapes: $crate::domain::calls::ParameterShape,
        references: $crate::domain::lexical::ReferenceObservation,
        lexical_targets: $crate::domain::lexical::LexicalTarget,
        lexical_resolutions: $crate::domain::lexical::LexicalResolution,
        callables: $crate::domain::normalized::entities::CallableEntity,
        refs: $crate::domain::normalized::entities::EntityRef,
        resolutions: $crate::domain::normalized::entities::SymbolEntityResolution,
        owners: $crate::domain::normalized::entities::OccurrenceOwnership,
        parameter_links: $crate::domain::normalized::entities::ParameterEntityLink,
        reference_assessments: $crate::domain::normalized::links::ReferenceEntityAssessment,
        reference_candidates: $crate::domain::normalized::links::ReferenceEntityCandidate,
        reference_targets: $crate::domain::normalized::links::ReferenceEntityTarget,
    } };
}
#[macro_export]
macro_rules! normalized_callable_outputs {
    ($apply:ident) => { $apply! {
        assessments: $crate::domain::normalized::callables::EffectiveCallableAssessment,
        decorators: $crate::domain::normalized::callables::EffectiveDecoratorMember,
        premises: $crate::domain::normalized::callables::EffectiveCallablePremise,
        evidence: $crate::domain::normalized::callables::EffectiveCallableEvidence,
        variants: $crate::domain::normalized::callables::SignatureVariant,
        slots: $crate::domain::normalized::callables::SignatureSlot,
        slot_entities: $crate::domain::normalized::callables::SignatureSlotEntity,
    } };
}
