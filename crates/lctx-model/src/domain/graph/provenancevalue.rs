use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ProvenanceValue {
    RetainedGuardSubstitution(crate::domain::conditions::stability::GuardSubstitution),
    NativeDiagnosticAnnotation(crate::domain::diagnostics::DiagnosticAnnotation),
    NativeDiagnosticSubject(crate::domain::diagnostics::DiagnosticSubject),
    RetainedSummaryCaptureContribution(
        crate::domain::execution::summary_capture::SummaryCaptureContribution,
    ),
    NativeFlowUseCandidate(crate::domain::flow_inventory::FlowUseCandidate),
    RetainedEffectiveCallableEvidence(
        crate::domain::normalized::callables::EffectiveCallableEvidence,
    ),
    RetainedEffectiveCallablePremise(
        crate::domain::normalized::callables::EffectiveCallablePremise,
    ),
    RetainedSymbolEntityEvidence(crate::domain::normalized::entities::SymbolEntityEvidence),
    RetainedSymbolEntityPremise(crate::domain::normalized::entities::SymbolEntityPremise),
    RetainedCallAlternativeEvidence(crate::domain::normalized::events::CallAlternativeEvidence),
    RetainedCallEventResolution(crate::domain::normalized::events::CallEventResolution),
    RetainedCallEventResolutionEvidence(
        crate::domain::normalized::events::CallEventResolutionEvidence,
    ),
    RetainedEventPhaseTarget(crate::domain::normalized::events::EventPhaseTarget),
    RetainedFlowCallEventLink(crate::domain::normalized::events::FlowCallEventLink),
    RetainedDeclarationNativeCharacterization(
        crate::domain::normalized::links::DeclarationNativeCharacterization,
    ),
    RetainedImportModuleAssessment(crate::domain::normalized::links::ImportModuleAssessment),
    RetainedImportModuleCandidate(crate::domain::normalized::links::ImportModuleCandidate),
    RetainedMentionSymbolCandidate(crate::domain::normalized::links::MentionSymbolCandidate),
    RetainedPlaceEntityLink(crate::domain::normalized::links::PlaceEntityLink),
    RetainedTestOperandCoverage(crate::domain::normalized::links::TestOperandCoverage),
    RetainedTypeBinderAssessment(crate::domain::normalized::links::TypeBinderAssessment),
    RetainedTypeBinderCandidate(crate::domain::normalized::links::TypeBinderCandidate),
    RetainedTypeBinderPremise(crate::domain::normalized::links::TypeBinderPremise),
    RetainedTypeEntityLink(crate::domain::normalized::links::TypeEntityLink),
    RetainedOverloadVariantAssessment(
        crate::domain::normalized::overload_association::OverloadVariantAssessment,
    ),
    RetainedOverloadVariantCandidate(
        crate::domain::normalized::overload_association::OverloadVariantCandidate,
    ),
    RetainedReceiverEvidence(crate::domain::normalized::receiver::ReceiverEvidence),
    RetainedReceiverPremise(crate::domain::normalized::receiver::ReceiverPremise),
    RetainedAssertionSource(crate::domain::synthesis::assertions::AssertionSource),
    RetainedProgrammaticAssertionSupport(
        crate::domain::synthesis::assertions::ProgrammaticAssertionSupport,
    ),

    NativeQualification(crate::domain::analysis::native::NativeQualification),
    LocalAnalysisObligation(crate::domain::analysis::local::AnalysisObligation),
    LocalCoverageRequirement(crate::domain::analysis::local::CoverageRequirement),
    LocalCoverageRequiredSource(crate::domain::analysis::local::CoverageRequiredSource),
    BaseEvaluationAnalysisObligation(crate::domain::analysis::base_evaluation::AnalysisObligation),
    BaseEvaluationCoverageRequirement(
        crate::domain::analysis::base_evaluation::CoverageRequirement,
    ),
    BaseEvaluationCoverageRequiredSource(
        crate::domain::analysis::base_evaluation::CoverageRequiredSource,
    ),
    BaseCompletionAnalysisObligation(crate::domain::analysis::base_completion::AnalysisObligation),
    BaseCompletionCoverageRequirement(
        crate::domain::analysis::base_completion::CoverageRequirement,
    ),
    BaseCompletionCoverageRequiredSource(
        crate::domain::analysis::base_completion::CoverageRequiredSource,
    ),
    SourceCallAnalysisObligation(crate::domain::analysis::source_call::AnalysisObligation),
    SourceCallCoverageRequirement(crate::domain::analysis::source_call::CoverageRequirement),
    SourceCallCoverageRequiredSource(crate::domain::analysis::source_call::CoverageRequiredSource),
    EnrichedExecutionAnalysisObligation(
        crate::domain::analysis::enriched_execution::AnalysisObligation,
    ),
    EnrichedExecutionCoverageRequirement(
        crate::domain::analysis::enriched_execution::CoverageRequirement,
    ),
    EnrichedExecutionCoverageRequiredSource(
        crate::domain::analysis::enriched_execution::CoverageRequiredSource,
    ),
    ModelAnalysisObligation(crate::domain::analysis::model::AnalysisObligation),
    ModelCoverageRequirement(crate::domain::analysis::model::CoverageRequirement),
    ModelCoverageRequiredSource(crate::domain::analysis::model::CoverageRequiredSource),
    SummaryAnalysisObligation(crate::domain::analysis::summary::AnalysisObligation),
    SummaryCoverageRequirement(crate::domain::analysis::summary::CoverageRequirement),
    SummaryCoverageRequiredSource(crate::domain::analysis::summary::CoverageRequiredSource),
    StructuralAnalysisObligation(crate::domain::analysis::structural::AnalysisObligation),
    StructuralCoverageRequirement(crate::domain::analysis::structural::CoverageRequirement),
    StructuralCoverageRequiredSource(crate::domain::analysis::structural::CoverageRequiredSource),
    AnalyticEmbeddingAnalysisObligation(
        crate::domain::analysis::analytic_embedding::AnalysisObligation,
    ),
    AnalyticEmbeddingCoverageRequirement(
        crate::domain::analysis::analytic_embedding::CoverageRequirement,
    ),
    AnalyticEmbeddingCoverageRequiredSource(
        crate::domain::analysis::analytic_embedding::CoverageRequiredSource,
    ),
    AnalyticAnalysisObligation(crate::domain::analysis::analytic::AnalysisObligation),
    AnalyticCoverageRequirement(crate::domain::analysis::analytic::CoverageRequirement),
    AnalyticCoverageRequiredSource(crate::domain::analysis::analytic::CoverageRequiredSource),
    CatalogCoreAnalysisObligation(crate::domain::analysis::catalog_core::AnalysisObligation),
    CatalogCoreCoverageRequirement(crate::domain::analysis::catalog_core::CoverageRequirement),
    CatalogCoreCoverageRequiredSource(
        crate::domain::analysis::catalog_core::CoverageRequiredSource,
    ),
    CatalogEvidenceAnalysisObligation(
        crate::domain::analysis::catalog_evidence::AnalysisObligation,
    ),
    CatalogEvidenceCoverageRequirement(
        crate::domain::analysis::catalog_evidence::CoverageRequirement,
    ),
    CatalogEvidenceCoverageRequiredSource(
        crate::domain::analysis::catalog_evidence::CoverageRequiredSource,
    ),
    SelectionAnalysisObligation(crate::domain::analysis::selection::AnalysisObligation),
    SelectionCoverageRequirement(crate::domain::analysis::selection::CoverageRequirement),
    SelectionCoverageRequiredSource(crate::domain::analysis::selection::CoverageRequiredSource),
    SynthesisAnalysisObligation(crate::domain::analysis::synthesis::AnalysisObligation),
    SynthesisCoverageRequirement(crate::domain::analysis::synthesis::CoverageRequirement),
    SynthesisCoverageRequiredSource(crate::domain::analysis::synthesis::CoverageRequiredSource),
    RetrievalAnalysisObligation(crate::domain::analysis::retrieval::AnalysisObligation),
    RetrievalCoverageRequirement(crate::domain::analysis::retrieval::CoverageRequirement),
    RetrievalCoverageRequiredSource(crate::domain::analysis::retrieval::CoverageRequiredSource),

    Acquisition(crate::domain::input::InputAcquisition),
    CorpusMembership(crate::domain::input::CorpusLibrary),
    ArtifactUse(crate::domain::input::ArtifactUse),
    ArtifactOwnership(crate::domain::input::ArtifactOwnership),
    UnownedArtifact(crate::domain::input::UnownedArtifact),
    DerivedArtifact(crate::domain::input::DerivedArtifact),
    Distribution(crate::domain::input::InputDistribution),
    DistributionVerification(crate::domain::input::DistributionVerification),
    Environment(crate::domain::input::EnvironmentFingerprint),
    Coverage(crate::domain::attribution::ProviderCoverage),
    RunFamily(crate::domain::attribution::RunFamily),
    LocalAnalysisInput(crate::domain::analysis::local::AnalysisInput),
    LocalAnalysisOutcome(crate::domain::analysis::local::AnalysisOutcome),
    LocalAnalysisCoverage(crate::domain::analysis::local::AnalysisCoverage),
    LocalAnalysisCoveragePremise(crate::domain::analysis::local::AnalysisCoveragePremise),
    LocalAnalysisProposition(crate::domain::analysis::local::AnalysisProposition),
    LocalAnalysisDerivation(crate::domain::analysis::local::AnalysisDerivation),
    LocalAnalysisDerivationPremise(crate::domain::analysis::local::AnalysisDerivationPremise),
    BaseEvaluationAnalysisInput(crate::domain::analysis::base_evaluation::AnalysisInput),
    BaseEvaluationAnalysisOutcome(crate::domain::analysis::base_evaluation::AnalysisOutcome),
    BaseEvaluationAnalysisCoverage(crate::domain::analysis::base_evaluation::AnalysisCoverage),
    BaseEvaluationAnalysisCoveragePremise(
        crate::domain::analysis::base_evaluation::AnalysisCoveragePremise,
    ),
    BaseEvaluationAnalysisProposition(
        crate::domain::analysis::base_evaluation::AnalysisProposition,
    ),
    BaseEvaluationAnalysisDerivation(crate::domain::analysis::base_evaluation::AnalysisDerivation),
    BaseEvaluationAnalysisDerivationPremise(
        crate::domain::analysis::base_evaluation::AnalysisDerivationPremise,
    ),
    BaseCompletionAnalysisInput(crate::domain::analysis::base_completion::AnalysisInput),
    BaseCompletionAnalysisOutcome(crate::domain::analysis::base_completion::AnalysisOutcome),
    BaseCompletionAnalysisCoverage(crate::domain::analysis::base_completion::AnalysisCoverage),
    BaseCompletionAnalysisCoveragePremise(
        crate::domain::analysis::base_completion::AnalysisCoveragePremise,
    ),
    BaseCompletionAnalysisProposition(
        crate::domain::analysis::base_completion::AnalysisProposition,
    ),
    BaseCompletionAnalysisDerivation(crate::domain::analysis::base_completion::AnalysisDerivation),
    BaseCompletionAnalysisDerivationPremise(
        crate::domain::analysis::base_completion::AnalysisDerivationPremise,
    ),
    SourceCallAnalysisInput(crate::domain::analysis::source_call::AnalysisInput),
    SourceCallAnalysisOutcome(crate::domain::analysis::source_call::AnalysisOutcome),
    SourceCallAnalysisCoverage(crate::domain::analysis::source_call::AnalysisCoverage),
    SourceCallAnalysisCoveragePremise(
        crate::domain::analysis::source_call::AnalysisCoveragePremise,
    ),
    SourceCallAnalysisProposition(crate::domain::analysis::source_call::AnalysisProposition),
    SourceCallAnalysisDerivation(crate::domain::analysis::source_call::AnalysisDerivation),
    SourceCallAnalysisDerivationPremise(
        crate::domain::analysis::source_call::AnalysisDerivationPremise,
    ),
    EnrichedExecutionAnalysisInput(crate::domain::analysis::enriched_execution::AnalysisInput),
    EnrichedExecutionAnalysisOutcome(crate::domain::analysis::enriched_execution::AnalysisOutcome),
    EnrichedExecutionAnalysisCoverage(
        crate::domain::analysis::enriched_execution::AnalysisCoverage,
    ),
    EnrichedExecutionAnalysisCoveragePremise(
        crate::domain::analysis::enriched_execution::AnalysisCoveragePremise,
    ),
    EnrichedExecutionAnalysisProposition(
        crate::domain::analysis::enriched_execution::AnalysisProposition,
    ),
    EnrichedExecutionAnalysisDerivation(
        crate::domain::analysis::enriched_execution::AnalysisDerivation,
    ),
    EnrichedExecutionAnalysisDerivationPremise(
        crate::domain::analysis::enriched_execution::AnalysisDerivationPremise,
    ),
    ModelAnalysisInput(crate::domain::analysis::model::AnalysisInput),
    ModelAnalysisOutcome(crate::domain::analysis::model::AnalysisOutcome),
    ModelAnalysisCoverage(crate::domain::analysis::model::AnalysisCoverage),
    ModelAnalysisCoveragePremise(crate::domain::analysis::model::AnalysisCoveragePremise),
    ModelAnalysisProposition(crate::domain::analysis::model::AnalysisProposition),
    ModelAnalysisDerivation(crate::domain::analysis::model::AnalysisDerivation),
    ModelAnalysisDerivationPremise(crate::domain::analysis::model::AnalysisDerivationPremise),
    SummaryAnalysisInput(crate::domain::analysis::summary::AnalysisInput),
    SummaryAnalysisOutcome(crate::domain::analysis::summary::AnalysisOutcome),
    SummaryAnalysisCoverage(crate::domain::analysis::summary::AnalysisCoverage),
    SummaryAnalysisCoveragePremise(crate::domain::analysis::summary::AnalysisCoveragePremise),
    SummaryAnalysisProposition(crate::domain::analysis::summary::AnalysisProposition),
    SummaryAnalysisDerivation(crate::domain::analysis::summary::AnalysisDerivation),
    SummaryAnalysisDerivationPremise(crate::domain::analysis::summary::AnalysisDerivationPremise),
    StructuralAnalysisInput(crate::domain::analysis::structural::AnalysisInput),
    StructuralAnalysisOutcome(crate::domain::analysis::structural::AnalysisOutcome),
    StructuralAnalysisCoverage(crate::domain::analysis::structural::AnalysisCoverage),
    StructuralAnalysisCoveragePremise(crate::domain::analysis::structural::AnalysisCoveragePremise),
    StructuralAnalysisProposition(crate::domain::analysis::structural::AnalysisProposition),
    StructuralAnalysisDerivation(crate::domain::analysis::structural::AnalysisDerivation),
    StructuralAnalysisDerivationPremise(
        crate::domain::analysis::structural::AnalysisDerivationPremise,
    ),
    AnalyticEmbeddingAnalysisInput(crate::domain::analysis::analytic_embedding::AnalysisInput),
    AnalyticEmbeddingAnalysisOutcome(crate::domain::analysis::analytic_embedding::AnalysisOutcome),
    AnalyticEmbeddingAnalysisCoverage(
        crate::domain::analysis::analytic_embedding::AnalysisCoverage,
    ),
    AnalyticEmbeddingAnalysisCoveragePremise(
        crate::domain::analysis::analytic_embedding::AnalysisCoveragePremise,
    ),
    AnalyticEmbeddingAnalysisProposition(
        crate::domain::analysis::analytic_embedding::AnalysisProposition,
    ),
    AnalyticEmbeddingAnalysisDerivation(
        crate::domain::analysis::analytic_embedding::AnalysisDerivation,
    ),
    AnalyticEmbeddingAnalysisDerivationPremise(
        crate::domain::analysis::analytic_embedding::AnalysisDerivationPremise,
    ),
    AnalyticAnalysisInput(crate::domain::analysis::analytic::AnalysisInput),
    AnalyticAnalysisOutcome(crate::domain::analysis::analytic::AnalysisOutcome),
    AnalyticAnalysisCoverage(crate::domain::analysis::analytic::AnalysisCoverage),
    AnalyticAnalysisCoveragePremise(crate::domain::analysis::analytic::AnalysisCoveragePremise),
    AnalyticAnalysisProposition(crate::domain::analysis::analytic::AnalysisProposition),
    AnalyticAnalysisDerivation(crate::domain::analysis::analytic::AnalysisDerivation),
    AnalyticAnalysisDerivationPremise(crate::domain::analysis::analytic::AnalysisDerivationPremise),
    CatalogCoreAnalysisInput(crate::domain::analysis::catalog_core::AnalysisInput),
    CatalogCoreAnalysisOutcome(crate::domain::analysis::catalog_core::AnalysisOutcome),
    CatalogCoreAnalysisCoverage(crate::domain::analysis::catalog_core::AnalysisCoverage),
    CatalogCoreAnalysisCoveragePremise(
        crate::domain::analysis::catalog_core::AnalysisCoveragePremise,
    ),
    CatalogCoreAnalysisProposition(crate::domain::analysis::catalog_core::AnalysisProposition),
    CatalogCoreAnalysisDerivation(crate::domain::analysis::catalog_core::AnalysisDerivation),
    CatalogCoreAnalysisDerivationPremise(
        crate::domain::analysis::catalog_core::AnalysisDerivationPremise,
    ),
    CatalogEvidenceAnalysisInput(crate::domain::analysis::catalog_evidence::AnalysisInput),
    CatalogEvidenceAnalysisOutcome(crate::domain::analysis::catalog_evidence::AnalysisOutcome),
    CatalogEvidenceAnalysisCoverage(crate::domain::analysis::catalog_evidence::AnalysisCoverage),
    CatalogEvidenceAnalysisCoveragePremise(
        crate::domain::analysis::catalog_evidence::AnalysisCoveragePremise,
    ),
    CatalogEvidenceAnalysisProposition(
        crate::domain::analysis::catalog_evidence::AnalysisProposition,
    ),
    CatalogEvidenceAnalysisDerivation(
        crate::domain::analysis::catalog_evidence::AnalysisDerivation,
    ),
    CatalogEvidenceAnalysisDerivationPremise(
        crate::domain::analysis::catalog_evidence::AnalysisDerivationPremise,
    ),
    SelectionAnalysisInput(crate::domain::analysis::selection::AnalysisInput),
    SelectionAnalysisOutcome(crate::domain::analysis::selection::AnalysisOutcome),
    SelectionAnalysisCoverage(crate::domain::analysis::selection::AnalysisCoverage),
    SelectionAnalysisCoveragePremise(crate::domain::analysis::selection::AnalysisCoveragePremise),
    SelectionAnalysisProposition(crate::domain::analysis::selection::AnalysisProposition),
    SelectionAnalysisDerivation(crate::domain::analysis::selection::AnalysisDerivation),
    SelectionAnalysisDerivationPremise(
        crate::domain::analysis::selection::AnalysisDerivationPremise,
    ),
    SynthesisAnalysisInput(crate::domain::analysis::synthesis::AnalysisInput),
    SynthesisAnalysisOutcome(crate::domain::analysis::synthesis::AnalysisOutcome),
    SynthesisAnalysisCoverage(crate::domain::analysis::synthesis::AnalysisCoverage),
    SynthesisAnalysisCoveragePremise(crate::domain::analysis::synthesis::AnalysisCoveragePremise),
    SynthesisAnalysisProposition(crate::domain::analysis::synthesis::AnalysisProposition),
    SynthesisAnalysisDerivation(crate::domain::analysis::synthesis::AnalysisDerivation),
    SynthesisAnalysisDerivationPremise(
        crate::domain::analysis::synthesis::AnalysisDerivationPremise,
    ),
    RetrievalAnalysisInput(crate::domain::analysis::retrieval::AnalysisInput),
    RetrievalAnalysisOutcome(crate::domain::analysis::retrieval::AnalysisOutcome),
    RetrievalAnalysisCoverage(crate::domain::analysis::retrieval::AnalysisCoverage),
    RetrievalAnalysisCoveragePremise(crate::domain::analysis::retrieval::AnalysisCoveragePremise),
    RetrievalAnalysisProposition(crate::domain::analysis::retrieval::AnalysisProposition),
    RetrievalAnalysisDerivation(crate::domain::analysis::retrieval::AnalysisDerivation),
    RetrievalAnalysisDerivationPremise(
        crate::domain::analysis::retrieval::AnalysisDerivationPremise,
    ),
    AnalyticGraphArc(crate::domain::analytics::GraphArc),
    AnalyticPairSource(crate::domain::analytics::PairSource),
    AnalyticPairContribution(crate::domain::analytics::PairContribution),
    AnalyticIncidenceSource(crate::domain::analytics::IncidenceSource),
    AnalyticIncidence(crate::domain::analytics::Incidence),
    AnalyticTypeMetadataSelection(crate::domain::analytics::TypeMetadataSelection),
    AnalyticDecoratorSelection(crate::domain::analytics::DecoratorSelection),
    StructuralUsageSite(crate::domain::structural::UsageSite),
    StructuralUsageEvidence(crate::domain::structural::UsageEvidence),
    CallPolicyAssessment(crate::domain::normalized::events::CallPolicyAssessment),
    CallPolicyAdmission(crate::domain::normalized::events::CallPolicyAdmission),
    CatalogSourceCharacterization(crate::domain::catalog::evidence::SourceCharacterization),
    CatalogSourceCharacterizationScenario(
        crate::domain::catalog::evidence::SourceCharacterizationScenario,
    ),
    CatalogSourceUsage(crate::domain::catalog::evidence::SourceUsage),
    CatalogDiagnosticUseAssessment(crate::domain::catalog::evidence::DiagnosticUseAssessment),
    CatalogDiagnosticUseLink(crate::domain::catalog::evidence::DiagnosticUseLink),
    CatalogDiagnosticUsePath(crate::domain::catalog::evidence::DiagnosticUsePath),
    CatalogDiagnosticUseTarget(crate::domain::catalog::evidence::DiagnosticUseTarget),
    NormalizedCallEventSource(crate::domain::normalized::events::CallEventSource),
    NormalizedCallEventSourceEvidence(crate::domain::normalized::events::CallEventSourceEvidence),
    SelectionFieldLocationLink(crate::domain::catalog::evidence::FieldLocationLink),
    SelectionConstructorCandidateLink(crate::domain::catalog::evidence::ConstructorCandidateLink),
    SelectionFieldAccessAssessment(crate::domain::catalog::evidence::FieldAccessAssessment),
    SelectionFieldLocation(crate::domain::local_fields::FieldLocation),
    SelectionFieldLocationCandidate(crate::domain::local_fields::FieldLocationCandidate),
    SelectionSourceFieldLink(crate::domain::catalog::evidence::SourceFieldLink),
    SelectionReferenceBindingCharacterization(
        crate::domain::normalized::links::ReferenceBindingCharacterization,
    ),
}
impl Key for ProvenanceValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::RetainedGuardSubstitution(row) => {
                sink.part(b"variant", &143u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeDiagnosticAnnotation(row) => {
                sink.part(b"variant", &144u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeDiagnosticSubject(row) => {
                sink.part(b"variant", &145u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedSummaryCaptureContribution(row) => {
                sink.part(b"variant", &146u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeFlowUseCandidate(row) => {
                sink.part(b"variant", &147u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedEffectiveCallableEvidence(row) => {
                sink.part(b"variant", &148u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedEffectiveCallablePremise(row) => {
                sink.part(b"variant", &149u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedSymbolEntityEvidence(row) => {
                sink.part(b"variant", &150u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedSymbolEntityPremise(row) => {
                sink.part(b"variant", &151u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedCallAlternativeEvidence(row) => {
                sink.part(b"variant", &152u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedCallEventResolution(row) => {
                sink.part(b"variant", &153u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedCallEventResolutionEvidence(row) => {
                sink.part(b"variant", &154u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedEventPhaseTarget(row) => {
                sink.part(b"variant", &155u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedFlowCallEventLink(row) => {
                sink.part(b"variant", &156u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedDeclarationNativeCharacterization(row) => {
                sink.part(b"variant", &157u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedImportModuleAssessment(row) => {
                sink.part(b"variant", &158u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedImportModuleCandidate(row) => {
                sink.part(b"variant", &159u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedMentionSymbolCandidate(row) => {
                sink.part(b"variant", &160u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedPlaceEntityLink(row) => {
                sink.part(b"variant", &161u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedTestOperandCoverage(row) => {
                sink.part(b"variant", &162u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedTypeBinderAssessment(row) => {
                sink.part(b"variant", &163u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedTypeBinderCandidate(row) => {
                sink.part(b"variant", &164u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedTypeBinderPremise(row) => {
                sink.part(b"variant", &165u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedTypeEntityLink(row) => {
                sink.part(b"variant", &166u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedOverloadVariantAssessment(row) => {
                sink.part(b"variant", &167u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedOverloadVariantCandidate(row) => {
                sink.part(b"variant", &168u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedReceiverEvidence(row) => {
                sink.part(b"variant", &169u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedReceiverPremise(row) => {
                sink.part(b"variant", &170u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedAssertionSource(row) => {
                sink.part(b"variant", &171u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedProgrammaticAssertionSupport(row) => {
                sink.part(b"variant", &172u16.to_le_bytes());
                row.content_digest().encode(sink);
            }

            Self::SelectionReferenceBindingCharacterization(row) => {
                sink.part(b"variant", &142u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionSourceFieldLink(row) => {
                sink.part(b"variant", &141u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionFieldLocationLink(row) => {
                sink.part(b"variant", &136u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionConstructorCandidateLink(row) => {
                sink.part(b"variant", &137u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionFieldAccessAssessment(row) => {
                sink.part(b"variant", &138u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionFieldLocation(row) => {
                sink.part(b"variant", &139u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionFieldLocationCandidate(row) => {
                sink.part(b"variant", &140u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NormalizedCallEventSource(row) => {
                sink.part(b"variant", &134u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NormalizedCallEventSourceEvidence(row) => {
                sink.part(b"variant", &135u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogSourceCharacterization(row) => {
                sink.part(b"variant", &127u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogSourceCharacterizationScenario(row) => {
                sink.part(b"variant", &128u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogSourceUsage(row) => {
                sink.part(b"variant", &129u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogDiagnosticUseAssessment(row) => {
                sink.part(b"variant", &130u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogDiagnosticUseLink(row) => {
                sink.part(b"variant", &131u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogDiagnosticUsePath(row) => {
                sink.part(b"variant", &132u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogDiagnosticUseTarget(row) => {
                sink.part(b"variant", &133u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeQualification(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::native::NativeQualification as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::local::AnalysisObligation as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::local::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::local::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::base_evaluation::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::base_evaluation::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationCoverageRequiredSource(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::base_evaluation::CoverageRequiredSource as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::base_completion::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::base_completion::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionCoverageRequiredSource(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::base_completion::CoverageRequiredSource as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::source_call::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceCallCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::source_call::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceCallCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::source_call::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisObligation(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::enriched_execution::AnalysisObligation as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionCoverageRequirement(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::enriched_execution::CoverageRequirement as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionCoverageRequiredSource(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::enriched_execution::CoverageRequiredSource as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::model::AnalysisObligation as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::model::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::model::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::summary::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::summary::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::summary::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::structural::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::structural::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::structural::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisObligation(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::analytic_embedding::AnalysisObligation as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingCoverageRequirement(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::analytic_embedding::CoverageRequirement as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingCoverageRequiredSource(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::analytic_embedding::CoverageRequiredSource as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::analytic::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::analytic::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::analytic::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::catalog_core::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::catalog_core::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::catalog_core::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::catalog_evidence::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceCoverageRequirement(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::catalog_evidence::CoverageRequirement as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceCoverageRequiredSource(row) => {
                sink.part(b"semantic-type",<crate::domain::analysis::catalog_evidence::CoverageRequiredSource as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::selection::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SelectionCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::selection::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SelectionCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::selection::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::synthesis::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::synthesis::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::synthesis::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisObligation(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::retrieval::AnalysisObligation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::RetrievalCoverageRequirement(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::retrieval::CoverageRequirement as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::RetrievalCoverageRequiredSource(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analysis::retrieval::CoverageRequiredSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }

            Self::Acquisition(row) => {
                sink.part(b"variant", &(0u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CorpusMembership(row) => {
                sink.part(b"variant", &(1u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ArtifactUse(row) => {
                sink.part(b"variant", &(2u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ArtifactOwnership(row) => {
                sink.part(b"variant", &(3u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::UnownedArtifact(row) => {
                sink.part(b"variant", &(4u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DerivedArtifact(row) => {
                sink.part(b"variant", &(5u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Distribution(row) => {
                sink.part(b"variant", &(6u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DistributionVerification(row) => {
                sink.part(b"variant", &(7u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Environment(row) => {
                sink.part(b"variant", &(8u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Coverage(row) => {
                sink.part(b"variant", &(9u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RunFamily(row) => {
                sink.part(b"variant", &(10u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisInput(row) => {
                sink.part(b"variant", &(11u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisOutcome(row) => {
                sink.part(b"variant", &(12u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisCoverage(row) => {
                sink.part(b"variant", &(13u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(14u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisProposition(row) => {
                sink.part(b"variant", &(15u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisDerivation(row) => {
                sink.part(b"variant", &(16u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(17u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisInput(row) => {
                sink.part(b"variant", &(18u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisOutcome(row) => {
                sink.part(b"variant", &(19u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisCoverage(row) => {
                sink.part(b"variant", &(20u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(21u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisProposition(row) => {
                sink.part(b"variant", &(22u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisDerivation(row) => {
                sink.part(b"variant", &(23u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseEvaluationAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(24u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisInput(row) => {
                sink.part(b"variant", &(25u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisOutcome(row) => {
                sink.part(b"variant", &(26u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisCoverage(row) => {
                sink.part(b"variant", &(27u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(28u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisProposition(row) => {
                sink.part(b"variant", &(29u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisDerivation(row) => {
                sink.part(b"variant", &(30u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseCompletionAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(31u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisInput(row) => {
                sink.part(b"variant", &(32u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisOutcome(row) => {
                sink.part(b"variant", &(33u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisCoverage(row) => {
                sink.part(b"variant", &(34u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(35u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisProposition(row) => {
                sink.part(b"variant", &(36u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisDerivation(row) => {
                sink.part(b"variant", &(37u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(38u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisInput(row) => {
                sink.part(b"variant", &(39u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisOutcome(row) => {
                sink.part(b"variant", &(40u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisCoverage(row) => {
                sink.part(b"variant", &(41u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(42u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisProposition(row) => {
                sink.part(b"variant", &(43u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisDerivation(row) => {
                sink.part(b"variant", &(44u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::EnrichedExecutionAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(45u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisInput(row) => {
                sink.part(b"variant", &(46u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisOutcome(row) => {
                sink.part(b"variant", &(47u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisCoverage(row) => {
                sink.part(b"variant", &(48u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(49u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisProposition(row) => {
                sink.part(b"variant", &(50u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisDerivation(row) => {
                sink.part(b"variant", &(51u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(52u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisInput(row) => {
                sink.part(b"variant", &(53u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisOutcome(row) => {
                sink.part(b"variant", &(54u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisCoverage(row) => {
                sink.part(b"variant", &(55u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(56u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisProposition(row) => {
                sink.part(b"variant", &(57u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisDerivation(row) => {
                sink.part(b"variant", &(58u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(59u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisInput(row) => {
                sink.part(b"variant", &(60u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisOutcome(row) => {
                sink.part(b"variant", &(61u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisCoverage(row) => {
                sink.part(b"variant", &(62u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(63u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisProposition(row) => {
                sink.part(b"variant", &(64u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisDerivation(row) => {
                sink.part(b"variant", &(65u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(66u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisInput(row) => {
                sink.part(b"variant", &(67u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisOutcome(row) => {
                sink.part(b"variant", &(68u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisCoverage(row) => {
                sink.part(b"variant", &(69u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(70u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisProposition(row) => {
                sink.part(b"variant", &(71u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisDerivation(row) => {
                sink.part(b"variant", &(72u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticEmbeddingAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(73u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisInput(row) => {
                sink.part(b"variant", &(74u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisOutcome(row) => {
                sink.part(b"variant", &(75u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisCoverage(row) => {
                sink.part(b"variant", &(76u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(77u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisProposition(row) => {
                sink.part(b"variant", &(78u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisDerivation(row) => {
                sink.part(b"variant", &(79u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(80u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisInput(row) => {
                sink.part(b"variant", &(81u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisOutcome(row) => {
                sink.part(b"variant", &(82u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisCoverage(row) => {
                sink.part(b"variant", &(83u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(84u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisProposition(row) => {
                sink.part(b"variant", &(85u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisDerivation(row) => {
                sink.part(b"variant", &(86u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogCoreAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(87u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisInput(row) => {
                sink.part(b"variant", &(88u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisOutcome(row) => {
                sink.part(b"variant", &(89u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisCoverage(row) => {
                sink.part(b"variant", &(90u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(91u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisProposition(row) => {
                sink.part(b"variant", &(92u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisDerivation(row) => {
                sink.part(b"variant", &(93u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogEvidenceAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(94u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisInput(row) => {
                sink.part(b"variant", &(95u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisOutcome(row) => {
                sink.part(b"variant", &(96u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisCoverage(row) => {
                sink.part(b"variant", &(97u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(98u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisProposition(row) => {
                sink.part(b"variant", &(99u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisDerivation(row) => {
                sink.part(b"variant", &(100u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(101u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisInput(row) => {
                sink.part(b"variant", &(102u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisOutcome(row) => {
                sink.part(b"variant", &(103u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisCoverage(row) => {
                sink.part(b"variant", &(104u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(105u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisProposition(row) => {
                sink.part(b"variant", &(106u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisDerivation(row) => {
                sink.part(b"variant", &(107u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SynthesisAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(108u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisInput(row) => {
                sink.part(b"variant", &(109u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisOutcome(row) => {
                sink.part(b"variant", &(110u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisCoverage(row) => {
                sink.part(b"variant", &(111u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisCoveragePremise(row) => {
                sink.part(b"variant", &(112u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisProposition(row) => {
                sink.part(b"variant", &(113u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisDerivation(row) => {
                sink.part(b"variant", &(114u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnalysisDerivationPremise(row) => {
                sink.part(b"variant", &(115u16).to_le_bytes());
                row.content_digest().encode(sink);
            }

            Self::AnalyticGraphArc(row) => {
                sink.part(b"variant", &(116u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticPairSource(row) => {
                sink.part(b"variant", &(117u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticPairContribution(row) => {
                sink.part(b"variant", &(118u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticIncidenceSource(row) => {
                sink.part(b"variant", &(119u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticIncidence(row) => {
                sink.part(b"variant", &(120u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticTypeMetadataSelection(row) => {
                sink.part(b"variant", &(121u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticDecoratorSelection(row) => {
                sink.part(b"variant", &(122u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralUsageSite(row) => {
                sink.part(b"variant", &(123u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralUsageEvidence(row) => {
                sink.part(b"variant", &(124u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallPolicyAssessment(row) => {
                sink.part(b"variant", &(125u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallPolicyAdmission(row) => {
                sink.part(b"variant", &(126u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
        }
    }
}
impl ProvenanceValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::RetainedGuardSubstitution(row) => row.validate(),
            Self::NativeDiagnosticAnnotation(row) => row.validate(),
            Self::NativeDiagnosticSubject(row) => row.validate(),
            Self::RetainedSummaryCaptureContribution(row) => row.validate(),
            Self::NativeFlowUseCandidate(row) => row.validate(),
            Self::RetainedEffectiveCallableEvidence(row) => row.validate(),
            Self::RetainedEffectiveCallablePremise(row) => row.validate(),
            Self::RetainedSymbolEntityEvidence(row) => row.validate(),
            Self::RetainedSymbolEntityPremise(row) => row.validate(),
            Self::RetainedCallAlternativeEvidence(row) => row.validate(),
            Self::RetainedCallEventResolution(row) => row.validate(),
            Self::RetainedCallEventResolutionEvidence(row) => row.validate(),
            Self::RetainedEventPhaseTarget(row) => row.validate(),
            Self::RetainedFlowCallEventLink(row) => row.validate(),
            Self::RetainedDeclarationNativeCharacterization(row) => row.validate(),
            Self::RetainedImportModuleAssessment(row) => row.validate(),
            Self::RetainedImportModuleCandidate(row) => row.validate(),
            Self::RetainedMentionSymbolCandidate(row) => row.validate(),
            Self::RetainedPlaceEntityLink(row) => row.validate(),
            Self::RetainedTestOperandCoverage(row) => row.validate(),
            Self::RetainedTypeBinderAssessment(row) => row.validate(),
            Self::RetainedTypeBinderCandidate(row) => row.validate(),
            Self::RetainedTypeBinderPremise(row) => row.validate(),
            Self::RetainedTypeEntityLink(row) => row.validate(),
            Self::RetainedOverloadVariantAssessment(row) => row.validate(),
            Self::RetainedOverloadVariantCandidate(row) => row.validate(),
            Self::RetainedReceiverEvidence(row) => row.validate(),
            Self::RetainedReceiverPremise(row) => row.validate(),
            Self::RetainedAssertionSource(row) => row.validate(),
            Self::RetainedProgrammaticAssertionSupport(row) => row.validate(),

            Self::SelectionReferenceBindingCharacterization(row) => row.validate(),
            Self::SelectionSourceFieldLink(row) => row.validate(),
            Self::SelectionFieldLocationLink(row) => row.validate(),
            Self::SelectionConstructorCandidateLink(row) => row.validate(),
            Self::SelectionFieldAccessAssessment(row) => row.validate(),
            Self::SelectionFieldLocation(row) => row.validate(),
            Self::SelectionFieldLocationCandidate(row) => row.validate(),
            Self::NormalizedCallEventSource(row) => row.validate(),
            Self::NormalizedCallEventSourceEvidence(row) => row.validate(),
            Self::CatalogSourceCharacterization(row) => row.validate(),
            Self::CatalogSourceCharacterizationScenario(row) => row.validate(),
            Self::CatalogSourceUsage(row) => row.validate(),
            Self::CatalogDiagnosticUseAssessment(row) => row.validate(),
            Self::CatalogDiagnosticUseLink(row) => row.validate(),
            Self::CatalogDiagnosticUsePath(row) => row.validate(),
            Self::CatalogDiagnosticUseTarget(row) => row.validate(),
            Self::NativeQualification(row) => row.validate(),
            Self::LocalAnalysisObligation(row) => row.validate(),
            Self::LocalCoverageRequirement(row) => row.validate(),
            Self::LocalCoverageRequiredSource(row) => row.validate(),
            Self::BaseEvaluationAnalysisObligation(row) => row.validate(),
            Self::BaseEvaluationCoverageRequirement(row) => row.validate(),
            Self::BaseEvaluationCoverageRequiredSource(row) => row.validate(),
            Self::BaseCompletionAnalysisObligation(row) => row.validate(),
            Self::BaseCompletionCoverageRequirement(row) => row.validate(),
            Self::BaseCompletionCoverageRequiredSource(row) => row.validate(),
            Self::SourceCallAnalysisObligation(row) => row.validate(),
            Self::SourceCallCoverageRequirement(row) => row.validate(),
            Self::SourceCallCoverageRequiredSource(row) => row.validate(),
            Self::EnrichedExecutionAnalysisObligation(row) => row.validate(),
            Self::EnrichedExecutionCoverageRequirement(row) => row.validate(),
            Self::EnrichedExecutionCoverageRequiredSource(row) => row.validate(),
            Self::ModelAnalysisObligation(row) => row.validate(),
            Self::ModelCoverageRequirement(row) => row.validate(),
            Self::ModelCoverageRequiredSource(row) => row.validate(),
            Self::SummaryAnalysisObligation(row) => row.validate(),
            Self::SummaryCoverageRequirement(row) => row.validate(),
            Self::SummaryCoverageRequiredSource(row) => row.validate(),
            Self::StructuralAnalysisObligation(row) => row.validate(),
            Self::StructuralCoverageRequirement(row) => row.validate(),
            Self::StructuralCoverageRequiredSource(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisObligation(row) => row.validate(),
            Self::AnalyticEmbeddingCoverageRequirement(row) => row.validate(),
            Self::AnalyticEmbeddingCoverageRequiredSource(row) => row.validate(),
            Self::AnalyticAnalysisObligation(row) => row.validate(),
            Self::AnalyticCoverageRequirement(row) => row.validate(),
            Self::AnalyticCoverageRequiredSource(row) => row.validate(),
            Self::CatalogCoreAnalysisObligation(row) => row.validate(),
            Self::CatalogCoreCoverageRequirement(row) => row.validate(),
            Self::CatalogCoreCoverageRequiredSource(row) => row.validate(),
            Self::CatalogEvidenceAnalysisObligation(row) => row.validate(),
            Self::CatalogEvidenceCoverageRequirement(row) => row.validate(),
            Self::CatalogEvidenceCoverageRequiredSource(row) => row.validate(),
            Self::SelectionAnalysisObligation(row) => row.validate(),
            Self::SelectionCoverageRequirement(row) => row.validate(),
            Self::SelectionCoverageRequiredSource(row) => row.validate(),
            Self::SynthesisAnalysisObligation(row) => row.validate(),
            Self::SynthesisCoverageRequirement(row) => row.validate(),
            Self::SynthesisCoverageRequiredSource(row) => row.validate(),
            Self::RetrievalAnalysisObligation(row) => row.validate(),
            Self::RetrievalCoverageRequirement(row) => row.validate(),
            Self::RetrievalCoverageRequiredSource(row) => row.validate(),

            Self::Acquisition(row) => row.validate(),
            Self::CorpusMembership(row) => row.validate(),
            Self::ArtifactUse(row) => row.validate(),
            Self::ArtifactOwnership(row) => row.validate(),
            Self::UnownedArtifact(row) => row.validate(),
            Self::DerivedArtifact(row) => row.validate(),
            Self::Distribution(row) => row.validate(),
            Self::DistributionVerification(row) => row.validate(),
            Self::Environment(row) => row.validate(),
            Self::Coverage(row) => row.validate(),
            Self::RunFamily(row) => row.validate(),
            Self::LocalAnalysisInput(row) => row.validate(),
            Self::LocalAnalysisOutcome(row) => row.validate(),
            Self::LocalAnalysisCoverage(row) => row.validate(),
            Self::LocalAnalysisCoveragePremise(row) => row.validate(),
            Self::LocalAnalysisProposition(row) => row.validate(),
            Self::LocalAnalysisDerivation(row) => row.validate(),
            Self::LocalAnalysisDerivationPremise(row) => row.validate(),
            Self::BaseEvaluationAnalysisInput(row) => row.validate(),
            Self::BaseEvaluationAnalysisOutcome(row) => row.validate(),
            Self::BaseEvaluationAnalysisCoverage(row) => row.validate(),
            Self::BaseEvaluationAnalysisCoveragePremise(row) => row.validate(),
            Self::BaseEvaluationAnalysisProposition(row) => row.validate(),
            Self::BaseEvaluationAnalysisDerivation(row) => row.validate(),
            Self::BaseEvaluationAnalysisDerivationPremise(row) => row.validate(),
            Self::BaseCompletionAnalysisInput(row) => row.validate(),
            Self::BaseCompletionAnalysisOutcome(row) => row.validate(),
            Self::BaseCompletionAnalysisCoverage(row) => row.validate(),
            Self::BaseCompletionAnalysisCoveragePremise(row) => row.validate(),
            Self::BaseCompletionAnalysisProposition(row) => row.validate(),
            Self::BaseCompletionAnalysisDerivation(row) => row.validate(),
            Self::BaseCompletionAnalysisDerivationPremise(row) => row.validate(),
            Self::SourceCallAnalysisInput(row) => row.validate(),
            Self::SourceCallAnalysisOutcome(row) => row.validate(),
            Self::SourceCallAnalysisCoverage(row) => row.validate(),
            Self::SourceCallAnalysisCoveragePremise(row) => row.validate(),
            Self::SourceCallAnalysisProposition(row) => row.validate(),
            Self::SourceCallAnalysisDerivation(row) => row.validate(),
            Self::SourceCallAnalysisDerivationPremise(row) => row.validate(),
            Self::EnrichedExecutionAnalysisInput(row) => row.validate(),
            Self::EnrichedExecutionAnalysisOutcome(row) => row.validate(),
            Self::EnrichedExecutionAnalysisCoverage(row) => row.validate(),
            Self::EnrichedExecutionAnalysisCoveragePremise(row) => row.validate(),
            Self::EnrichedExecutionAnalysisProposition(row) => row.validate(),
            Self::EnrichedExecutionAnalysisDerivation(row) => row.validate(),
            Self::EnrichedExecutionAnalysisDerivationPremise(row) => row.validate(),
            Self::ModelAnalysisInput(row) => row.validate(),
            Self::ModelAnalysisOutcome(row) => row.validate(),
            Self::ModelAnalysisCoverage(row) => row.validate(),
            Self::ModelAnalysisCoveragePremise(row) => row.validate(),
            Self::ModelAnalysisProposition(row) => row.validate(),
            Self::ModelAnalysisDerivation(row) => row.validate(),
            Self::ModelAnalysisDerivationPremise(row) => row.validate(),
            Self::SummaryAnalysisInput(row) => row.validate(),
            Self::SummaryAnalysisOutcome(row) => row.validate(),
            Self::SummaryAnalysisCoverage(row) => row.validate(),
            Self::SummaryAnalysisCoveragePremise(row) => row.validate(),
            Self::SummaryAnalysisProposition(row) => row.validate(),
            Self::SummaryAnalysisDerivation(row) => row.validate(),
            Self::SummaryAnalysisDerivationPremise(row) => row.validate(),
            Self::StructuralAnalysisInput(row) => row.validate(),
            Self::StructuralAnalysisOutcome(row) => row.validate(),
            Self::StructuralAnalysisCoverage(row) => row.validate(),
            Self::StructuralAnalysisCoveragePremise(row) => row.validate(),
            Self::StructuralAnalysisProposition(row) => row.validate(),
            Self::StructuralAnalysisDerivation(row) => row.validate(),
            Self::StructuralAnalysisDerivationPremise(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisInput(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisOutcome(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisCoverage(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisCoveragePremise(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisProposition(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisDerivation(row) => row.validate(),
            Self::AnalyticEmbeddingAnalysisDerivationPremise(row) => row.validate(),
            Self::AnalyticAnalysisInput(row) => row.validate(),
            Self::AnalyticAnalysisOutcome(row) => row.validate(),
            Self::AnalyticAnalysisCoverage(row) => row.validate(),
            Self::AnalyticAnalysisCoveragePremise(row) => row.validate(),
            Self::AnalyticAnalysisProposition(row) => row.validate(),
            Self::AnalyticAnalysisDerivation(row) => row.validate(),
            Self::AnalyticAnalysisDerivationPremise(row) => row.validate(),
            Self::CatalogCoreAnalysisInput(row) => row.validate(),
            Self::CatalogCoreAnalysisOutcome(row) => row.validate(),
            Self::CatalogCoreAnalysisCoverage(row) => row.validate(),
            Self::CatalogCoreAnalysisCoveragePremise(row) => row.validate(),
            Self::CatalogCoreAnalysisProposition(row) => row.validate(),
            Self::CatalogCoreAnalysisDerivation(row) => row.validate(),
            Self::CatalogCoreAnalysisDerivationPremise(row) => row.validate(),
            Self::CatalogEvidenceAnalysisInput(row) => row.validate(),
            Self::CatalogEvidenceAnalysisOutcome(row) => row.validate(),
            Self::CatalogEvidenceAnalysisCoverage(row) => row.validate(),
            Self::CatalogEvidenceAnalysisCoveragePremise(row) => row.validate(),
            Self::CatalogEvidenceAnalysisProposition(row) => row.validate(),
            Self::CatalogEvidenceAnalysisDerivation(row) => row.validate(),
            Self::CatalogEvidenceAnalysisDerivationPremise(row) => row.validate(),
            Self::SelectionAnalysisInput(row) => row.validate(),
            Self::SelectionAnalysisOutcome(row) => row.validate(),
            Self::SelectionAnalysisCoverage(row) => row.validate(),
            Self::SelectionAnalysisCoveragePremise(row) => row.validate(),
            Self::SelectionAnalysisProposition(row) => row.validate(),
            Self::SelectionAnalysisDerivation(row) => row.validate(),
            Self::SelectionAnalysisDerivationPremise(row) => row.validate(),
            Self::SynthesisAnalysisInput(row) => row.validate(),
            Self::SynthesisAnalysisOutcome(row) => row.validate(),
            Self::SynthesisAnalysisCoverage(row) => row.validate(),
            Self::SynthesisAnalysisCoveragePremise(row) => row.validate(),
            Self::SynthesisAnalysisProposition(row) => row.validate(),
            Self::SynthesisAnalysisDerivation(row) => row.validate(),
            Self::SynthesisAnalysisDerivationPremise(row) => row.validate(),
            Self::RetrievalAnalysisInput(row) => row.validate(),
            Self::RetrievalAnalysisOutcome(row) => row.validate(),
            Self::RetrievalAnalysisCoverage(row) => row.validate(),
            Self::RetrievalAnalysisCoveragePremise(row) => row.validate(),
            Self::RetrievalAnalysisProposition(row) => row.validate(),
            Self::RetrievalAnalysisDerivation(row) => row.validate(),
            Self::RetrievalAnalysisDerivationPremise(row) => row.validate(),

            Self::AnalyticGraphArc(row) => row.validate(),
            Self::AnalyticPairSource(row) => row.validate(),
            Self::AnalyticPairContribution(row) => row.validate(),
            Self::AnalyticIncidenceSource(row) => row.validate(),
            Self::AnalyticIncidence(row) => row.validate(),
            Self::AnalyticTypeMetadataSelection(row) => row.validate(),
            Self::AnalyticDecoratorSelection(row) => row.validate(),
            Self::StructuralUsageSite(row) => row.validate(),
            Self::StructuralUsageEvidence(row) => row.validate(),
            Self::CallPolicyAssessment(row) => row.validate(),
            Self::CallPolicyAdmission(row) => row.validate(),
        }
    }
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::RetainedGuardSubstitution(row) => row.references(),
            Self::NativeDiagnosticAnnotation(row) => row.references(),
            Self::NativeDiagnosticSubject(row) => row.references(),
            Self::RetainedSummaryCaptureContribution(row) => row.references(),
            Self::NativeFlowUseCandidate(row) => row.references(),
            Self::RetainedEffectiveCallableEvidence(row) => row.references(),
            Self::RetainedEffectiveCallablePremise(row) => row.references(),
            Self::RetainedSymbolEntityEvidence(row) => row.references(),
            Self::RetainedSymbolEntityPremise(row) => row.references(),
            Self::RetainedCallAlternativeEvidence(row) => row.references(),
            Self::RetainedCallEventResolution(row) => row.references(),
            Self::RetainedCallEventResolutionEvidence(row) => row.references(),
            Self::RetainedEventPhaseTarget(row) => row.references(),
            Self::RetainedFlowCallEventLink(row) => row.references(),
            Self::RetainedDeclarationNativeCharacterization(row) => row.references(),
            Self::RetainedImportModuleAssessment(row) => row.references(),
            Self::RetainedImportModuleCandidate(row) => row.references(),
            Self::RetainedMentionSymbolCandidate(row) => row.references(),
            Self::RetainedPlaceEntityLink(row) => row.references(),
            Self::RetainedTestOperandCoverage(row) => row.references(),
            Self::RetainedTypeBinderAssessment(row) => row.references(),
            Self::RetainedTypeBinderCandidate(row) => row.references(),
            Self::RetainedTypeBinderPremise(row) => row.references(),
            Self::RetainedTypeEntityLink(row) => row.references(),
            Self::RetainedOverloadVariantAssessment(row) => row.references(),
            Self::RetainedOverloadVariantCandidate(row) => row.references(),
            Self::RetainedReceiverEvidence(row) => row.references(),
            Self::RetainedReceiverPremise(row) => row.references(),
            Self::RetainedAssertionSource(row) => row.references(),
            Self::RetainedProgrammaticAssertionSupport(row) => row.references(),

            Self::SelectionReferenceBindingCharacterization(row) => row.references(),
            Self::SelectionSourceFieldLink(row) => row.references(),
            Self::SelectionFieldLocationLink(row) => row.references(),
            Self::SelectionConstructorCandidateLink(row) => row.references(),
            Self::SelectionFieldAccessAssessment(row) => row.references(),
            Self::SelectionFieldLocation(row) => row.references(),
            Self::SelectionFieldLocationCandidate(row) => row.references(),
            Self::NormalizedCallEventSource(row) => row.references(),
            Self::NormalizedCallEventSourceEvidence(row) => row.references(),
            Self::CatalogSourceCharacterization(row) => row.references(),
            Self::CatalogSourceCharacterizationScenario(row) => row.references(),
            Self::CatalogSourceUsage(row) => row.references(),
            Self::CatalogDiagnosticUseAssessment(row) => row.references(),
            Self::CatalogDiagnosticUseLink(row) => row.references(),
            Self::CatalogDiagnosticUsePath(row) => row.references(),
            Self::CatalogDiagnosticUseTarget(row) => row.references(),
            Self::NativeQualification(row) => row.references(),
            Self::LocalAnalysisObligation(row) => row.references(),
            Self::LocalCoverageRequirement(row) => row.references(),
            Self::LocalCoverageRequiredSource(row) => row.references(),
            Self::BaseEvaluationAnalysisObligation(row) => row.references(),
            Self::BaseEvaluationCoverageRequirement(row) => row.references(),
            Self::BaseEvaluationCoverageRequiredSource(row) => row.references(),
            Self::BaseCompletionAnalysisObligation(row) => row.references(),
            Self::BaseCompletionCoverageRequirement(row) => row.references(),
            Self::BaseCompletionCoverageRequiredSource(row) => row.references(),
            Self::SourceCallAnalysisObligation(row) => row.references(),
            Self::SourceCallCoverageRequirement(row) => row.references(),
            Self::SourceCallCoverageRequiredSource(row) => row.references(),
            Self::EnrichedExecutionAnalysisObligation(row) => row.references(),
            Self::EnrichedExecutionCoverageRequirement(row) => row.references(),
            Self::EnrichedExecutionCoverageRequiredSource(row) => row.references(),
            Self::ModelAnalysisObligation(row) => row.references(),
            Self::ModelCoverageRequirement(row) => row.references(),
            Self::ModelCoverageRequiredSource(row) => row.references(),
            Self::SummaryAnalysisObligation(row) => row.references(),
            Self::SummaryCoverageRequirement(row) => row.references(),
            Self::SummaryCoverageRequiredSource(row) => row.references(),
            Self::StructuralAnalysisObligation(row) => row.references(),
            Self::StructuralCoverageRequirement(row) => row.references(),
            Self::StructuralCoverageRequiredSource(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisObligation(row) => row.references(),
            Self::AnalyticEmbeddingCoverageRequirement(row) => row.references(),
            Self::AnalyticEmbeddingCoverageRequiredSource(row) => row.references(),
            Self::AnalyticAnalysisObligation(row) => row.references(),
            Self::AnalyticCoverageRequirement(row) => row.references(),
            Self::AnalyticCoverageRequiredSource(row) => row.references(),
            Self::CatalogCoreAnalysisObligation(row) => row.references(),
            Self::CatalogCoreCoverageRequirement(row) => row.references(),
            Self::CatalogCoreCoverageRequiredSource(row) => row.references(),
            Self::CatalogEvidenceAnalysisObligation(row) => row.references(),
            Self::CatalogEvidenceCoverageRequirement(row) => row.references(),
            Self::CatalogEvidenceCoverageRequiredSource(row) => row.references(),
            Self::SelectionAnalysisObligation(row) => row.references(),
            Self::SelectionCoverageRequirement(row) => row.references(),
            Self::SelectionCoverageRequiredSource(row) => row.references(),
            Self::SynthesisAnalysisObligation(row) => row.references(),
            Self::SynthesisCoverageRequirement(row) => row.references(),
            Self::SynthesisCoverageRequiredSource(row) => row.references(),
            Self::RetrievalAnalysisObligation(row) => row.references(),
            Self::RetrievalCoverageRequirement(row) => row.references(),
            Self::RetrievalCoverageRequiredSource(row) => row.references(),

            Self::Acquisition(row) => row.references(),
            Self::CorpusMembership(row) => row.references(),
            Self::ArtifactUse(row) => row.references(),
            Self::ArtifactOwnership(row) => row.references(),
            Self::UnownedArtifact(row) => row.references(),
            Self::DerivedArtifact(row) => row.references(),
            Self::Distribution(row) => row.references(),
            Self::DistributionVerification(row) => row.references(),
            Self::Environment(row) => row.references(),
            Self::Coverage(row) => row.references(),
            Self::RunFamily(row) => row.references(),
            Self::LocalAnalysisInput(row) => row.references(),
            Self::LocalAnalysisOutcome(row) => row.references(),
            Self::LocalAnalysisCoverage(row) => row.references(),
            Self::LocalAnalysisCoveragePremise(row) => row.references(),
            Self::LocalAnalysisProposition(row) => row.references(),
            Self::LocalAnalysisDerivation(row) => row.references(),
            Self::LocalAnalysisDerivationPremise(row) => row.references(),
            Self::BaseEvaluationAnalysisInput(row) => row.references(),
            Self::BaseEvaluationAnalysisOutcome(row) => row.references(),
            Self::BaseEvaluationAnalysisCoverage(row) => row.references(),
            Self::BaseEvaluationAnalysisCoveragePremise(row) => row.references(),
            Self::BaseEvaluationAnalysisProposition(row) => row.references(),
            Self::BaseEvaluationAnalysisDerivation(row) => row.references(),
            Self::BaseEvaluationAnalysisDerivationPremise(row) => row.references(),
            Self::BaseCompletionAnalysisInput(row) => row.references(),
            Self::BaseCompletionAnalysisOutcome(row) => row.references(),
            Self::BaseCompletionAnalysisCoverage(row) => row.references(),
            Self::BaseCompletionAnalysisCoveragePremise(row) => row.references(),
            Self::BaseCompletionAnalysisProposition(row) => row.references(),
            Self::BaseCompletionAnalysisDerivation(row) => row.references(),
            Self::BaseCompletionAnalysisDerivationPremise(row) => row.references(),
            Self::SourceCallAnalysisInput(row) => row.references(),
            Self::SourceCallAnalysisOutcome(row) => row.references(),
            Self::SourceCallAnalysisCoverage(row) => row.references(),
            Self::SourceCallAnalysisCoveragePremise(row) => row.references(),
            Self::SourceCallAnalysisProposition(row) => row.references(),
            Self::SourceCallAnalysisDerivation(row) => row.references(),
            Self::SourceCallAnalysisDerivationPremise(row) => row.references(),
            Self::EnrichedExecutionAnalysisInput(row) => row.references(),
            Self::EnrichedExecutionAnalysisOutcome(row) => row.references(),
            Self::EnrichedExecutionAnalysisCoverage(row) => row.references(),
            Self::EnrichedExecutionAnalysisCoveragePremise(row) => row.references(),
            Self::EnrichedExecutionAnalysisProposition(row) => row.references(),
            Self::EnrichedExecutionAnalysisDerivation(row) => row.references(),
            Self::EnrichedExecutionAnalysisDerivationPremise(row) => row.references(),
            Self::ModelAnalysisInput(row) => row.references(),
            Self::ModelAnalysisOutcome(row) => row.references(),
            Self::ModelAnalysisCoverage(row) => row.references(),
            Self::ModelAnalysisCoveragePremise(row) => row.references(),
            Self::ModelAnalysisProposition(row) => row.references(),
            Self::ModelAnalysisDerivation(row) => row.references(),
            Self::ModelAnalysisDerivationPremise(row) => row.references(),
            Self::SummaryAnalysisInput(row) => row.references(),
            Self::SummaryAnalysisOutcome(row) => row.references(),
            Self::SummaryAnalysisCoverage(row) => row.references(),
            Self::SummaryAnalysisCoveragePremise(row) => row.references(),
            Self::SummaryAnalysisProposition(row) => row.references(),
            Self::SummaryAnalysisDerivation(row) => row.references(),
            Self::SummaryAnalysisDerivationPremise(row) => row.references(),
            Self::StructuralAnalysisInput(row) => row.references(),
            Self::StructuralAnalysisOutcome(row) => row.references(),
            Self::StructuralAnalysisCoverage(row) => row.references(),
            Self::StructuralAnalysisCoveragePremise(row) => row.references(),
            Self::StructuralAnalysisProposition(row) => row.references(),
            Self::StructuralAnalysisDerivation(row) => row.references(),
            Self::StructuralAnalysisDerivationPremise(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisInput(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisOutcome(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisCoverage(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisCoveragePremise(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisProposition(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisDerivation(row) => row.references(),
            Self::AnalyticEmbeddingAnalysisDerivationPremise(row) => row.references(),
            Self::AnalyticAnalysisInput(row) => row.references(),
            Self::AnalyticAnalysisOutcome(row) => row.references(),
            Self::AnalyticAnalysisCoverage(row) => row.references(),
            Self::AnalyticAnalysisCoveragePremise(row) => row.references(),
            Self::AnalyticAnalysisProposition(row) => row.references(),
            Self::AnalyticAnalysisDerivation(row) => row.references(),
            Self::AnalyticAnalysisDerivationPremise(row) => row.references(),
            Self::CatalogCoreAnalysisInput(row) => row.references(),
            Self::CatalogCoreAnalysisOutcome(row) => row.references(),
            Self::CatalogCoreAnalysisCoverage(row) => row.references(),
            Self::CatalogCoreAnalysisCoveragePremise(row) => row.references(),
            Self::CatalogCoreAnalysisProposition(row) => row.references(),
            Self::CatalogCoreAnalysisDerivation(row) => row.references(),
            Self::CatalogCoreAnalysisDerivationPremise(row) => row.references(),
            Self::CatalogEvidenceAnalysisInput(row) => row.references(),
            Self::CatalogEvidenceAnalysisOutcome(row) => row.references(),
            Self::CatalogEvidenceAnalysisCoverage(row) => row.references(),
            Self::CatalogEvidenceAnalysisCoveragePremise(row) => row.references(),
            Self::CatalogEvidenceAnalysisProposition(row) => row.references(),
            Self::CatalogEvidenceAnalysisDerivation(row) => row.references(),
            Self::CatalogEvidenceAnalysisDerivationPremise(row) => row.references(),
            Self::SelectionAnalysisInput(row) => row.references(),
            Self::SelectionAnalysisOutcome(row) => row.references(),
            Self::SelectionAnalysisCoverage(row) => row.references(),
            Self::SelectionAnalysisCoveragePremise(row) => row.references(),
            Self::SelectionAnalysisProposition(row) => row.references(),
            Self::SelectionAnalysisDerivation(row) => row.references(),
            Self::SelectionAnalysisDerivationPremise(row) => row.references(),
            Self::SynthesisAnalysisInput(row) => row.references(),
            Self::SynthesisAnalysisOutcome(row) => row.references(),
            Self::SynthesisAnalysisCoverage(row) => row.references(),
            Self::SynthesisAnalysisCoveragePremise(row) => row.references(),
            Self::SynthesisAnalysisProposition(row) => row.references(),
            Self::SynthesisAnalysisDerivation(row) => row.references(),
            Self::SynthesisAnalysisDerivationPremise(row) => row.references(),
            Self::RetrievalAnalysisInput(row) => row.references(),
            Self::RetrievalAnalysisOutcome(row) => row.references(),
            Self::RetrievalAnalysisCoverage(row) => row.references(),
            Self::RetrievalAnalysisCoveragePremise(row) => row.references(),
            Self::RetrievalAnalysisProposition(row) => row.references(),
            Self::RetrievalAnalysisDerivation(row) => row.references(),
            Self::RetrievalAnalysisDerivationPremise(row) => row.references(),

            Self::AnalyticGraphArc(row) => row.references(),
            Self::AnalyticPairSource(row) => row.references(),
            Self::AnalyticPairContribution(row) => row.references(),
            Self::AnalyticIncidenceSource(row) => row.references(),
            Self::AnalyticIncidence(row) => row.references(),
            Self::AnalyticTypeMetadataSelection(row) => row.references(),
            Self::AnalyticDecoratorSelection(row) => row.references(),
            Self::StructuralUsageSite(row) => row.references(),
            Self::StructuralUsageEvidence(row) => row.references(),
            Self::CallPolicyAssessment(row) => row.references(),
            Self::CallPolicyAdmission(row) => row.references(),
        }
    }
    pub fn semantic_key(&self) -> SemanticKey {
        match self {
            Self::RetainedGuardSubstitution(row) => SemanticKey::of(row.id()),
            Self::NativeDiagnosticAnnotation(row) => SemanticKey::of(row.id()),
            Self::NativeDiagnosticSubject(row) => SemanticKey::of(row.id()),
            Self::RetainedSummaryCaptureContribution(row) => SemanticKey::of(row.id()),
            Self::NativeFlowUseCandidate(row) => SemanticKey::of(row.id()),
            Self::RetainedEffectiveCallableEvidence(row) => SemanticKey::of(row.id()),
            Self::RetainedEffectiveCallablePremise(row) => SemanticKey::of(row.id()),
            Self::RetainedSymbolEntityEvidence(row) => SemanticKey::of(row.id()),
            Self::RetainedSymbolEntityPremise(row) => SemanticKey::of(row.id()),
            Self::RetainedCallAlternativeEvidence(row) => SemanticKey::of(row.id()),
            Self::RetainedCallEventResolution(row) => SemanticKey::of(row.id()),
            Self::RetainedCallEventResolutionEvidence(row) => SemanticKey::of(row.id()),
            Self::RetainedEventPhaseTarget(row) => SemanticKey::of(row.id()),
            Self::RetainedFlowCallEventLink(row) => SemanticKey::of(row.id()),
            Self::RetainedDeclarationNativeCharacterization(row) => SemanticKey::of(row.id()),
            Self::RetainedImportModuleAssessment(row) => SemanticKey::of(row.id()),
            Self::RetainedImportModuleCandidate(row) => SemanticKey::of(row.id()),
            Self::RetainedMentionSymbolCandidate(row) => SemanticKey::of(row.id()),
            Self::RetainedPlaceEntityLink(row) => SemanticKey::of(row.id()),
            Self::RetainedTestOperandCoverage(row) => SemanticKey::of(row.id()),
            Self::RetainedTypeBinderAssessment(row) => SemanticKey::of(row.id()),
            Self::RetainedTypeBinderCandidate(row) => SemanticKey::of(row.id()),
            Self::RetainedTypeBinderPremise(row) => SemanticKey::of(row.id()),
            Self::RetainedTypeEntityLink(row) => SemanticKey::of(row.id()),
            Self::RetainedOverloadVariantAssessment(row) => SemanticKey::of(row.id()),
            Self::RetainedOverloadVariantCandidate(row) => SemanticKey::of(row.id()),
            Self::RetainedReceiverEvidence(row) => SemanticKey::of(row.id()),
            Self::RetainedReceiverPremise(row) => SemanticKey::of(row.id()),
            Self::RetainedAssertionSource(row) => SemanticKey::of(row.id()),
            Self::RetainedProgrammaticAssertionSupport(row) => SemanticKey::of(row.id()),

            Self::SelectionReferenceBindingCharacterization(row) => SemanticKey::of(row.id()),
            Self::SelectionSourceFieldLink(row) => SemanticKey::of(row.id()),
            Self::SelectionFieldLocationLink(row) => SemanticKey::of(row.id()),
            Self::SelectionConstructorCandidateLink(row) => SemanticKey::of(row.id()),
            Self::SelectionFieldAccessAssessment(row) => SemanticKey::of(row.id()),
            Self::SelectionFieldLocation(row) => SemanticKey::of(row.id()),
            Self::SelectionFieldLocationCandidate(row) => SemanticKey::of(row.id()),
            Self::NormalizedCallEventSource(row) => SemanticKey::of(row.id()),
            Self::NormalizedCallEventSourceEvidence(row) => SemanticKey::of(row.id()),
            Self::CatalogSourceCharacterization(row) => SemanticKey::of(row.id()),
            Self::CatalogSourceCharacterizationScenario(row) => SemanticKey::of(row.id()),
            Self::CatalogSourceUsage(row) => SemanticKey::of(row.id()),
            Self::CatalogDiagnosticUseAssessment(row) => SemanticKey::of(row.id()),
            Self::CatalogDiagnosticUseLink(row) => SemanticKey::of(row.id()),
            Self::CatalogDiagnosticUsePath(row) => SemanticKey::of(row.id()),
            Self::CatalogDiagnosticUseTarget(row) => SemanticKey::of(row.id()),
            Self::NativeQualification(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::LocalCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::LocalCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::SourceCallCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::SourceCallCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::ModelCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::ModelCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::SummaryCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::SummaryCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::StructuralCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::StructuralCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::AnalyticCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::AnalyticCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::SelectionCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::SelectionCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::SynthesisCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::SynthesisCoverageRequiredSource(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisObligation(row) => SemanticKey::of(row.id()),
            Self::RetrievalCoverageRequirement(row) => SemanticKey::of(row.id()),
            Self::RetrievalCoverageRequiredSource(row) => SemanticKey::of(row.id()),

            Self::Acquisition(row) => SemanticKey::of(row.id()),
            Self::CorpusMembership(row) => SemanticKey::of(row.id()),
            Self::ArtifactUse(row) => SemanticKey::of(row.id()),
            Self::ArtifactOwnership(row) => SemanticKey::of(row.id()),
            Self::UnownedArtifact(row) => SemanticKey::of(row.id()),
            Self::DerivedArtifact(row) => SemanticKey::of(row.id()),
            Self::Distribution(row) => SemanticKey::of(row.id()),
            Self::DistributionVerification(row) => SemanticKey::of(row.id()),
            Self::Environment(row) => SemanticKey::of(row.id()),
            Self::Coverage(row) => SemanticKey::of(row.id()),
            Self::RunFamily(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::LocalAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::BaseEvaluationAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::BaseCompletionAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::SourceCallAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::EnrichedExecutionAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::ModelAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::SummaryAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::StructuralAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::AnalyticEmbeddingAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::AnalyticAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::CatalogCoreAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::CatalogEvidenceAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::SelectionAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::SynthesisAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisInput(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisOutcome(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisCoverage(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisCoveragePremise(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisProposition(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisDerivation(row) => SemanticKey::of(row.id()),
            Self::RetrievalAnalysisDerivationPremise(row) => SemanticKey::of(row.id()),

            Self::AnalyticGraphArc(row) => SemanticKey::of(row.id()),
            Self::AnalyticPairSource(row) => SemanticKey::of(row.id()),
            Self::AnalyticPairContribution(row) => SemanticKey::of(row.id()),
            Self::AnalyticIncidenceSource(row) => SemanticKey::of(row.id()),
            Self::AnalyticIncidence(row) => SemanticKey::of(row.id()),
            Self::AnalyticTypeMetadataSelection(row) => SemanticKey::of(row.id()),
            Self::AnalyticDecoratorSelection(row) => SemanticKey::of(row.id()),
            Self::StructuralUsageSite(row) => SemanticKey::of(row.id()),
            Self::StructuralUsageEvidence(row) => SemanticKey::of(row.id()),
            Self::CallPolicyAssessment(row) => SemanticKey::of(row.id()),
            Self::CallPolicyAdmission(row) => SemanticKey::of(row.id()),
        }
    }
}
