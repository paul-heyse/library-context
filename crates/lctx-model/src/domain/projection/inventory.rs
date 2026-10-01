#[macro_export]
macro_rules! projection_inputs {
    ($apply:ident) => {
        $apply! {
            alternative_sources: $crate::domain::normalized::events::CallAlternativeSource,
            dispatch_assessments: $crate::domain::normalized::dispatch::DispatchAssessment,
            dispatch_members: $crate::domain::normalized::dispatch::DispatchMember,
            dispatch_premises: $crate::domain::normalized::dispatch::DispatchPremise,
            dispatch_evidence: $crate::domain::normalized::dispatch::DispatchEvidence,
            refs: $crate::domain::normalized::entities::EntityRef,
            callables: $crate::domain::normalized::entities::CallableEntity,
            classes: $crate::domain::normalized::entities::ClassEntity,
            parameters: $crate::domain::normalized::entities::ParameterEntity,
            fields: $crate::domain::normalized::entities::FieldEntity,
            owners: $crate::domain::normalized::entities::OccurrenceOwnership,
            symbols: $crate::domain::calls::ProviderSymbol,
            provider_modules: $crate::domain::calls::ProviderModule,
            resolutions: $crate::domain::normalized::entities::SymbolEntityResolution,
            occurrences: $crate::domain::source::Occurrence,
            modules: $crate::domain::source::Module,
            artifacts: $crate::domain::source::SourceArtifact,
            scopes: $crate::domain::source::CoverageScope,
            qualifications: $crate::domain::assertion::AssertionQualification,
            coverage: $crate::domain::attribution::ProviderCoverage,
            runs: $crate::domain::attribution::ProviderRun,
            events: $crate::domain::normalized::events::NormalizedCallEvent,
            event_assessments: $crate::domain::normalized::events::EventAssessment,
            alternatives: $crate::domain::normalized::events::NormalizedCallAlternative,
            policies: $crate::domain::normalized::events::CallPolicyAssessment,
            admissions: $crate::domain::normalized::events::CallPolicyAdmission,
            targets: $crate::domain::calls::CallTarget,
            import_assessments: $crate::domain::normalized::links::ImportModuleAssessment,
            import_candidates: $crate::domain::normalized::links::ImportModuleCandidate,
            imports: $crate::domain::syntax::ImportAliasObservation,
            reference_assessments: $crate::domain::normalized::links::ReferenceEntityAssessment,
            reference_candidates: $crate::domain::normalized::links::ReferenceEntityCandidate,
            reference_targets: $crate::domain::normalized::links::ReferenceEntityTarget,
            references: $crate::domain::lexical::ReferenceObservation,
            exposures: $crate::domain::normalized::entities::PublicExposure,
            exposure_candidates: $crate::domain::normalized::entities::PublicExposureCandidate,
        }
    };
}
#[macro_export]
macro_rules! projection_outputs {
    ($apply:ident) => {
        $apply! {
            assessments: $crate::domain::projection::ProjectionSourceAssessment,
            subjects: $crate::domain::projection::ProjectionGapSubject,
            gaps: $crate::domain::projection::ProjectionGap,
            coverage: $crate::domain::projection::ProjectionSourceCoverage,
            snapshots: $crate::domain::projection::ProjectionSnapshot,
            chunks: $crate::domain::projection::ProjectionSnapshotChunk,
        }
    };
}
