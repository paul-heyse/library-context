//! Executable inventories shared by the model declaration, validator and stage driver.
#[macro_export]
macro_rules! normalized_entity_inputs {
    ($apply:ident) => { $apply! {
        occurrences: $crate::domain::source::Occurrence => Syntax,
        modules: $crate::domain::source::Module => Artifacts,
        symbols: $crate::domain::calls::ProviderSymbol => Signatures,
        provider_modules: $crate::domain::calls::ProviderModule => Signatures,
        declarations: $crate::domain::declarations::SymbolDeclaration => Signatures,
        declaration_supports: $crate::domain::declarations::SymbolDeclarationSupport => Signatures,
        qualifications: $crate::domain::assertion::AssertionQualification => Artifacts,
        function_traits: $crate::domain::symbols::FunctionTraitObservation => Signatures,
        class_traits: $crate::domain::symbols::ClassTraitObservation => Signatures,
        signatures: $crate::domain::calls::Signature => Signatures,
        parameters: $crate::domain::calls::SignatureParameter => Signatures,
        parameter_declarations: $crate::domain::declarations::ParameterDeclaration => Signatures,
        fields: $crate::domain::types::RecordFieldObservation => Types,
        public_names: $crate::domain::symbols::PublicNameObservation => Exports,
        export_enumerations: $crate::domain::symbols::ExportEnumerationObservation => Exports,
        export_enumeration_supports: $crate::domain::symbols::ExportEnumerationSupport => Exports,
        export_origins: $crate::domain::symbols::ExportOrigin => Exports,
        public_supports: $crate::domain::symbols::PublicNameSupport => Exports,
        runs: $crate::domain::attribution::ProviderRun => Artifacts,
        syntax_fields: $crate::domain::syntax::ClassFieldSyntaxObservation => Syntax,
        bindings: $crate::domain::lexical::BindingEvent => Lexical,
        terms: $crate::domain::types::TypeTerm => Types,
        places: $crate::domain::value::Place => Flow,
    } };
}
#[macro_export]
macro_rules! normalized_entity_outputs {
    ($apply:ident) => {
        $apply! {
            callables: $crate::domain::normalized::entities::CallableEntity,
            classes: $crate::domain::normalized::entities::ClassEntity,
            parameters: $crate::domain::normalized::entities::ParameterEntity,
            fields: $crate::domain::normalized::entities::FieldEntity,
            refs: $crate::domain::normalized::entities::EntityRef,
            resolutions: $crate::domain::normalized::entities::SymbolEntityResolution,
            candidates: $crate::domain::normalized::entities::SymbolEntityCandidate,
            premises: $crate::domain::normalized::entities::SymbolEntityPremise,
            evidence: $crate::domain::normalized::entities::SymbolEntityEvidence,
            owners: $crate::domain::normalized::entities::OccurrenceOwnership,
            parameter_links: $crate::domain::normalized::entities::ParameterEntityLink,
            field_links: $crate::domain::normalized::entities::FieldEntityLink,
            field_declarations: $crate::domain::normalized::entities::FieldDeclarationLink,
            exposures: $crate::domain::normalized::entities::PublicExposure,
            public_enumerations: $crate::domain::normalized::entities::PublicEnumerationAssessment,
            exposure_candidates: $crate::domain::normalized::entities::PublicExposureCandidate,
        }
    };
}
