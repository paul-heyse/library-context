#[macro_export]
macro_rules! retrieval_inputs {
    ($m:ident) => {
        $m! {
         packages:$crate::domain::input::Package,
         releases:$crate::domain::input::Release,
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
         components:$crate::domain::documents::DocumentComponentObservation,
        }
    };
}
#[macro_export]
macro_rules! retrieval_outputs {
    ($m:ident) => {
        $m! {
         origins:$crate::domain::retrieval::Origin,
         corpus:$crate::domain::retrieval::CorpusText,
         units:$crate::domain::retrieval::Unit,
         subjects:$crate::domain::retrieval::Subject,
         unit_subjects:$crate::domain::retrieval::UnitSubject,
         anchors:$crate::domain::retrieval::OriginalAnchor,
         anchor_sources:$crate::domain::retrieval::AnchorSource,
         roots:$crate::domain::retrieval::UnitRoot,
         parts:$crate::domain::retrieval::ContentPart,
         part_contexts:$crate::domain::retrieval::PartContext,
         part_maps:$crate::domain::retrieval::PartSourceMap,
         windows:$crate::domain::retrieval::SearchWindow,
         window_parts:$crate::domain::retrieval::WindowPart,
         window_maps:$crate::domain::retrieval::WindowSourceMap,
         bindings:$crate::domain::retrieval::WindowBinding,
        }
    };
}

#[macro_export]
macro_rules! retrieval_synthesis_inputs {
    ($m:ident) => {
        $m! {
         member_frames:$crate::domain::catalog::CatalogMemberInvocation,
         evidence_outcomes:$crate::domain::analysis::catalog_evidence::AnalysisOutcome,
         analysis_definitions:$crate::domain::analysis::AnalysisDefinition,
         parameters:$crate::domain::analysis::MethodParameters,
         synthesis_invocations:$crate::domain::analysis::synthesis::Invocation,
         synthesis_outcomes:$crate::domain::analysis::synthesis::AnalysisOutcome,
         synthesis_frames:$crate::domain::synthesis::frames::Frame,
         seed_plans:$crate::domain::synthesis::seeds::SeedPlan,
         seeds:$crate::domain::synthesis::seeds::SelectedSeed,
         briefs:$crate::domain::synthesis::briefs::Brief,
         brief_documents:$crate::domain::synthesis::briefs::BriefDocument,
         brief_sources:$crate::domain::synthesis::briefs::BriefSource,
         documentary:$crate::domain::synthesis::documentary::DocumentaryConclusion,
         prose_slices:$crate::domain::synthesis::documentary::ProseSlice,
         prose_sources:$crate::domain::synthesis::documentary::ProseSource,
        }
    };
}
