#[macro_export]
macro_rules! catalog_selection_inputs {($m:ident)=>{$m! {
 shapes:$crate::domain::calls::ParameterShape,
 signature_parameters:$crate::domain::calls::SignatureParameter,
 signatures:$crate::domain::calls::Signature,
 type_observations:$crate::domain::types::TypeObservation,
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
}};}
#[macro_export]
macro_rules! catalog_selection_outputs {($m:ident)=>{$m! {
 contexts:$crate::domain::selection::Context,
 witnesses:$crate::domain::selection::Witness,
 domains:$crate::domain::selection::SelectionDomain,
 members:$crate::domain::selection::DomainContext,
 closure:$crate::domain::selection::DomainClosure,
 evidence:$crate::domain::selection::DomainEvidence,
}};}
