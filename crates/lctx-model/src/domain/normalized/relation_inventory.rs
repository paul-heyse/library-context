//! Additional N2 input and output inventories; N1 premises are also declared completed inputs.
#[macro_export]
macro_rules! normalized_relation_inputs {
    ($apply:ident) => {
        $apply! {
            symbol_observations: $crate::domain::symbols::SymbolObservation => Signatures,
            references: $crate::domain::lexical::ReferenceObservation => Lexical,
            ruff_contexts: $crate::domain::ruff::RuffContextObservation => Lexical,
            ruff_context_supports: $crate::domain::ruff::RuffContextSupport => Lexical,
            ruff_bindings: $crate::domain::ruff::RuffBindingObservation => Lexical,
            ruff_binding_supports: $crate::domain::ruff::RuffBindingSupport => Lexical,
            ruff_definitions: $crate::domain::ruff::RuffDefinitionObservation => Lexical,
            ruff_definition_supports: $crate::domain::ruff::RuffDefinitionSupport => Lexical,
            native_surfaces: $crate::domain::assertion::ProviderSurface => Artifacts,
            native_providers: $crate::domain::attribution::Provider => Artifacts,
            native_evidence: $crate::domain::assertion::Evidence => Artifacts,
            lexical_resolutions: $crate::domain::lexical::LexicalResolution => Lexical,
            lexical_targets: $crate::domain::lexical::LexicalTarget => Lexical,
            imports: $crate::domain::syntax::ImportAliasObservation => Exports,
            module_resolutions: $crate::domain::symbols::ModuleResolutionObservation => Exports,
            ancestry: $crate::domain::symbols::ClassAncestryObservation => Signatures,
            sequence_members: $crate::domain::symbols::SymbolSequenceMember => Signatures,
            mentions: $crate::domain::documents::DocumentMentionObservation => Docs,
            variables: $crate::domain::types::TypeVariable => Types,
            declaration_syntax: $crate::domain::syntax::DeclarationObservation => Syntax,
            binding_observations: $crate::domain::lexical::BindingObservation => Lexical,
            roots: $crate::domain::value::PlaceRoot => Flow,
            leaves: $crate::domain::flow::FlowTestLeafObservation => Flow,
            type_observations: $crate::domain::types::TypeObservation => Types,
            coverage: $crate::domain::attribution::ProviderCoverage => Artifacts,
            scopes: $crate::domain::source::CoverageScope => Artifacts,
            artifacts: $crate::domain::source::SourceArtifact => Artifacts,
        }
    };
}
#[macro_export]
macro_rules! normalized_relation_outputs {
    ($apply:ident) => { $apply! {
        type_binder_premises: $crate::domain::normalized::links::TypeBinderPremise,
        mention_symbol_candidates: $crate::domain::normalized::links::MentionSymbolCandidate,
        reference_targets: $crate::domain::normalized::links::ReferenceEntityTarget,
        reference_entity_assessments: $crate::domain::normalized::links::ReferenceEntityAssessment,
        reference_entity_candidates: $crate::domain::normalized::links::ReferenceEntityCandidate,
        reference_binding_characterizations: $crate::domain::normalized::links::ReferenceBindingCharacterization,
        declaration_native_characterizations: $crate::domain::normalized::links::DeclarationNativeCharacterization,
        import_module_assessments: $crate::domain::normalized::links::ImportModuleAssessment,
        import_module_candidates: $crate::domain::normalized::links::ImportModuleCandidate,
        ancestry_entity_assessments: $crate::domain::normalized::links::AncestryEntityAssessment,
        ancestry_entity_members: $crate::domain::normalized::links::AncestryEntityMember,
        mention_entity_assessments: $crate::domain::normalized::links::MentionEntityAssessment,
        mention_entity_candidates: $crate::domain::normalized::links::MentionEntityCandidate,
        type_entity_links: $crate::domain::normalized::links::TypeEntityLink,
        type_binder_assessments: $crate::domain::normalized::links::TypeBinderAssessment,
        type_binder_candidates: $crate::domain::normalized::links::TypeBinderCandidate,
        place_entity_links: $crate::domain::normalized::links::PlaceEntityLink,
        test_operand_type_assessments: $crate::domain::normalized::links::TestOperandTypeAssessment,
        test_operand_type_links: $crate::domain::normalized::links::TestOperandTypeLink,
        test_operand_coverage: $crate::domain::normalized::links::TestOperandCoverage,
    } };
}
