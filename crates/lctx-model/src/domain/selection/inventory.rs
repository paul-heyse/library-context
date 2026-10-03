#[macro_export]
macro_rules! catalog_selection_inputs {
    ($m:ident) => {
        $m! {
         shapes:$crate::domain::calls::ParameterShape,
         signature_parameters:$crate::domain::calls::SignatureParameter,
         signatures:$crate::domain::calls::Signature,
         type_observations:$crate::domain::types::TypeObservation,
         call_syntax:$crate::domain::calls::CallSyntax,
         generic_specializations:$crate::domain::types::GenericSpecializationObservation,
         signature_subjects:$crate::domain::types::SignatureTypeSubject,
         type_sequence_headers:$crate::domain::types::TypeSequence,
         type_callable_lists:$crate::domain::types::CallableParameterList,
         type_callable_slots:$crate::domain::types::CallableParameter,
         type_dict_lists:$crate::domain::types::TypedDictFieldList,
         type_dict_fields:$crate::domain::types::TypedDictField,
         type_variables:$crate::domain::types::TypeVariable,
         type_terms:$crate::domain::types::TypeTerm,
         type_sequences:$crate::domain::types::TypeSequenceMember,
         literals:$crate::domain::value::Literal,
         packages:$crate::domain::input::Package,
         releases:$crate::domain::input::Release,
         core_coverage:$crate::domain::analysis::catalog_core::AnalysisCoverage,
         evidence_coverage:$crate::domain::analysis::catalog_evidence::AnalysisCoverage,
         evidence_invocations:$crate::domain::analysis::catalog_evidence::Invocation,
         evidence_sources:$crate::domain::analysis::catalog_evidence::InvocationSource,
         evidence_inputs:$crate::domain::analysis::catalog_evidence::AnalysisInput,
         evidence_links:$crate::domain::catalog::evidence::EvidenceInvocation,
        }
    };
}
#[macro_export]
macro_rules! catalog_selection_outputs {
    ($m:ident) => {
        $m! {
         contexts:$crate::domain::selection::Context,
         witnesses:$crate::domain::selection::Witness,
         domains:$crate::domain::selection::SelectionDomain,
         members:$crate::domain::selection::DomainContext,
         closure:$crate::domain::selection::DomainClosure,
         evidence:$crate::domain::selection::DomainEvidence,
        }
    };
}
