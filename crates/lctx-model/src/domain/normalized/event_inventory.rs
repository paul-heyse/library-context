#[macro_export]
macro_rules! normalized_event_inputs {
    ($apply:ident) => {
        $apply! {
            qualifications: $crate::domain::assertion::AssertionQualification,
            symbols: $crate::domain::calls::ProviderSymbol,
            provider_callables: $crate::domain::calls::ProviderCallable,
            provider_modules: $crate::domain::calls::ProviderModule,
            call_sites: $crate::domain::calls::ProviderCallSite,
            site_supports: $crate::domain::calls::ProviderCallSiteSupport,
            targets: $crate::domain::calls::CallTarget,
            target_supports: $crate::domain::calls::CallTargetSupport,
            channels: $crate::domain::calls::CallChannel,
            destinations: $crate::domain::calls::CallDestination,
            receivers: $crate::domain::calls::Receiver,
            resolutions: $crate::domain::calls::CallResolution,
            resolution_supports: $crate::domain::calls::CallResolutionSupport,
            members: $crate::domain::calls::CallResolutionMember,
            symbol_resolutions: $crate::domain::normalized::entities::SymbolEntityResolution,
            refs: $crate::domain::normalized::entities::EntityRef,
            owners: $crate::domain::normalized::entities::OccurrenceOwnership,
            paths: $crate::domain::flow::FlowValuePathObservation,
            steps: $crate::domain::flow::FlowCallStep,
        }
    };
}
#[macro_export]
macro_rules! normalized_event_outputs {
    ($apply:ident) => {
        $apply! {
            events: $crate::domain::normalized::events::NormalizedCallEvent,
            sources: $crate::domain::normalized::events::CallEventSource,
            source_evidence: $crate::domain::normalized::events::CallEventSourceEvidence,
            resolutions: $crate::domain::normalized::events::CallEventResolution,
            resolution_evidence: $crate::domain::normalized::events::CallEventResolutionEvidence,
            alternatives: $crate::domain::normalized::events::NormalizedCallAlternative,
            alternative_evidence: $crate::domain::normalized::events::CallAlternativeEvidence,
            assessments: $crate::domain::normalized::events::EventAssessment,
            phase_targets: $crate::domain::normalized::events::EventPhaseTarget,
            policy_assessments: $crate::domain::normalized::events::CallPolicyAssessment,
            admissions: $crate::domain::normalized::events::CallPolicyAdmission,
            flow_links: $crate::domain::normalized::events::FlowCallEventLink,
        }
    };
}
