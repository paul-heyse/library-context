#[macro_export]
macro_rules! retrieval_inputs {($m:ident)=>{$m! {
 definitions:$crate::domain::retrieval::Definition,
 chunks:$crate::domain::artifact::ArtifactChunk,
 shapes:$crate::domain::calls::ParameterShape,
 signature_parameters:$crate::domain::calls::SignatureParameter,
 signatures:$crate::domain::calls::Signature,
 literals:$crate::domain::value::Literal,
 evidence_invocations:$crate::domain::analysis::catalog_evidence::Invocation,
 evidence_sources:$crate::domain::analysis::catalog_evidence::InvocationSource,
 evidence_inputs:$crate::domain::analysis::catalog_evidence::AnalysisInput,
 evidence_links:$crate::domain::catalog::evidence::EvidenceInvocation,
}};}
#[macro_export]
macro_rules! retrieval_outputs {($m:ident)=>{$m! {
 origins:$crate::domain::retrieval::Origin,
 corpus:$crate::domain::retrieval::CorpusText,
 units:$crate::domain::retrieval::Unit,
 subjects:$crate::domain::retrieval::Subject,
 unit_subjects:$crate::domain::retrieval::UnitSubject,
 anchors:$crate::domain::retrieval::OriginalAnchor,
 anchor_sources:$crate::domain::retrieval::AnchorSource,
 roots:$crate::domain::retrieval::UnitRoot,
 fragments:$crate::domain::retrieval::Fragment,
}};}
