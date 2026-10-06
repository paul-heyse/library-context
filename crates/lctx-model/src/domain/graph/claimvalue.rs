use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ClaimValue {
    LocalControlSupports(crate::domain::transfer::local::ControlSupport),
    LocalTransferSelections(crate::domain::transfer::local::Selection),

    CallResolutionMembers(crate::domain::calls::CallResolutionMember),
    SignatureEnumerationMembers(crate::domain::calls::SignatureEnumerationMember),
    AnalyticCommunityLabelMembers(crate::domain::analytics::CommunityLabelMember),
    BindingSetMembers(crate::domain::normalized::bindings::BindingSetMember),
    BindingSetCoverage(crate::domain::normalized::bindings::BindingSetCoverage),
    NormalizedDispatchPremises(crate::domain::normalized::dispatch::DispatchPremise),
    NormalizedDispatchEvidence(crate::domain::normalized::dispatch::DispatchEvidence),
    SummaryControlInfluences(crate::domain::transfer::summary::ControlInfluence),
    SummaryControlSupports(crate::domain::transfer::summary::ControlSupport),
    SummaryTransferSelections(crate::domain::transfer::summary::Selection),

    AssumptionUniverseSupports(crate::domain::assumptions_universe::AssumptionUniverseSupport),
    AnalyticTextAssessments(crate::domain::embedding::text::TextAssessment),
    SynthesisConfiguredSeedDecisions(crate::domain::synthesis::seeds::ConfiguredSeedDecision),
    SynthesisAutomaticSeedDecisions(crate::domain::synthesis::automatic::Decision),
    StructuralReaches(crate::domain::structural::Reach),
    StructuralUnfollowedArguments(crate::domain::structural::controls::UnfollowedArgument),
    StructuralHandoffValues(crate::domain::structural::handoffs::ValueSource),
    StructuralHandoffOccurrences(crate::domain::structural::handoffs::Handoff),
    SummaryBehavioralConclusions(crate::domain::execution::summary_consequences::ClaimConclusion),
    SourceFieldClassAssessments(crate::domain::normalized::symbolic_fields::SourceFieldClass),
    SourceFieldStores(crate::domain::normalized::symbolic_fields::SourceFieldStore),
    SourceFieldReaders(crate::domain::normalized::symbolic_fields::SourceFieldReader),
    SourceFieldAssociations(crate::domain::normalized::symbolic_fields::SourceFieldAssociation),
    CallEventAssessments(crate::domain::normalized::events::EventAssessment),

    LocalTypeDomains(crate::domain::local_theory::TypeDomain),
    LocalTypeDomainAssessments(crate::domain::local_theory::TypeDomainAssessment),
    LocalTypeDomainMembers(crate::domain::local_theory::TypeDomainMember),
    ReportEntries(crate::domain::deployment::ReportEntry),
    CallOriginSteps(crate::domain::calls::CallOriginStep),
    LocalSymbolicFieldStores(crate::domain::local_symbolic::SymbolicFieldStore),
    FlowCaptureCandidates(crate::domain::flow_capture::FlowCaptureCandidate),
    LocalAtomDecisions(crate::domain::atom_decision::AtomDecision),
    FlowCallSteps(crate::domain::flow::FlowCallStep),
    ProjectionSourceAssessments(crate::domain::projection::ProjectionSourceAssessment),
    ProjectionGaps(crate::domain::projection::ProjectionGap),
    ProjectionSourceCoverages(crate::domain::projection::ProjectionSourceCoverage),
    AnalyticTextWindows(crate::domain::embedding::text::TextWindow),
    SynthesisSeedPlans(crate::domain::synthesis::seeds::SeedPlan),
    SynthesisSelectedSeedSources(crate::domain::synthesis::seeds::SelectedSeedSource),
    SynthesisAuthoredCodeSources(crate::domain::synthesis::patterns::AuthoredCodeSource),
    SynthesisProseSources(crate::domain::synthesis::documentary::ProseSource),
    EntryAccessSources(crate::domain::conditions::entry::EntryAccessSource),
    StructuralPublicCandidates(crate::domain::structural::PublicCandidate),
    StructuralTraversals(crate::domain::structural::Traversal),
    StructuralPaths(crate::domain::structural::Path),
    StructuralUnresolvedEvents(crate::domain::structural::UnresolvedEvent),
    StructuralLiteralArguments(crate::domain::structural::controls::LiteralArgument),
    StructuralControlPaths(crate::domain::structural::controls::ControlPath),
    StructuralConditionalRaises(crate::domain::structural::controls::ConditionalRaise),
    StructuralUnfollowedPaths(crate::domain::structural::controls::UnfollowedPath),
    StructuralHandoffGroups(crate::domain::structural::handoffs::Group),
    ClosedTargetAssessments(crate::domain::execution::closed_targets::ClosedTargetAssessment),
    ModelContextResources(crate::domain::execution::model_protocol::ContextResource),
    CapturedEntryBindings(crate::domain::execution::capture_bridge::CapturedEntryBinding),
    AnalyticDocumentNeighbours(crate::domain::analytics::DocumentNeighbour),
    AnalyticCommunityLabels(crate::domain::analytics::CommunityLabel),
    SourceFieldReaderLinks(crate::domain::normalized::symbolic_fields::SourceFieldReaderLink),
    ClassFieldDefaultAssessments(
        crate::domain::normalized::callable_aspects::FieldDefaultAssessment,
    ),
    ReferenceEntityTargets(crate::domain::normalized::links::ReferenceEntityTarget),
    ReferenceEntityAssessments(crate::domain::normalized::links::ReferenceEntityAssessment),
    AncestryEntityAssessments(crate::domain::normalized::links::AncestryEntityAssessment),
    MentionEntityAssessments(crate::domain::normalized::links::MentionEntityAssessment),
    TestOperandTypeAssessments(crate::domain::normalized::links::TestOperandTypeAssessment),
    SymbolEntityCandidates(crate::domain::normalized::entities::SymbolEntityCandidate),
    ParameterEntityLinks(crate::domain::normalized::entities::ParameterEntityLink),
    FieldEntityLinks(crate::domain::normalized::entities::FieldEntityLink),
    FieldDeclarationLinks(crate::domain::normalized::entities::FieldDeclarationLink),
    PublicExposureCandidates(crate::domain::normalized::entities::PublicExposureCandidate),
    NormalizedCallAlternativeSources(crate::domain::normalized::events::CallAlternativeSource),
    SignatureSlotEntities(crate::domain::normalized::callables::SignatureSlotEntity),
    NormalizedDispatchAssessments(crate::domain::normalized::dispatch::DispatchAssessment),
    LocalTransferAlternatives(crate::domain::transfer::local::TransferAlternative),
    LocalTransferSupports(crate::domain::transfer::local::TransferSupport),
    ModelTransferAlternatives(crate::domain::transfer::model::TransferAlternative),
    ModelTransferSupports(crate::domain::transfer::model::TransferSupport),
    SummaryTransferAlternatives(crate::domain::transfer::summary::TransferAlternative),
    SummaryTransferSupports(crate::domain::transfer::summary::TransferSupport),

    LocalBuiltinOperandWitnesses(crate::domain::local_theory::BuiltinOperandWitness),
    BindingSources(crate::domain::calls::BindingSource),
    BindingProjections(crate::domain::calls::BindingProjection),
    LocalAtomRestrictions(crate::domain::atom_decision::AtomRestriction),
    AnalysisEmbeddingUses(crate::domain::embedding::analytic::AnalysisEmbeddingUse),
    CatalogMemberInvocations(crate::domain::catalog::CatalogMemberInvocation),
    SynthesisSelectedSeeds(crate::domain::synthesis::seeds::SelectedSeed),
    SynthesisAuthoredCodeConclusions(crate::domain::synthesis::patterns::AuthoredCodeConclusion),
    SynthesisProseSlices(crate::domain::synthesis::documentary::ProseSlice),
    SynthesisDocumentarySources(crate::domain::synthesis::documentary::DocumentarySource),
    EntryValueWitnesses(crate::domain::conditions::entry::EntryValueWitness),
    StabilityWitnesses(crate::domain::conditions::stability::StabilityWitness),
    StructuralConclusionSources(crate::domain::structural::ConclusionSource),
    SummaryTerminalWitnesses(crate::domain::execution::summary_terminal::SummaryTerminalWitness),
    BaseStatementCompletions(crate::domain::execution::completion_records::StatementCompletion),
    ModeledCallEvaluations(crate::domain::execution::modeled_call::ModeledCallEvaluation),
    SummaryPathRoutes(crate::domain::execution::summary_path::SummaryPathRoute),
    ModelApplications(crate::domain::execution::model_production::ModelApplication),
    SummaryCaptureWitnesses(crate::domain::execution::summary_capture::SummaryCaptureWitness),
    BaseSourceBodyCompletions(crate::domain::execution::body_records::SourceBodyCompletion),
    ConditionalTerminalFrontiers(
        crate::domain::execution::protocol_interpretation::ConditionalTerminalFrontier,
    ),
    NormalContinuationRestrictions(
        crate::domain::execution::protocol_interpretation::NormalContinuationRestriction,
    ),
    SummaryClaimProofs(crate::domain::execution::summary_consequences::ClaimProof),
    SummaryControlWitnesses(crate::domain::execution::summary_control::SummaryControlWitness),
    DefinitionEvaluations(crate::domain::execution::definition::DefinitionEvaluation),
    ModelAppliedRules(crate::domain::execution::model_rules::AppliedRule),
    ContextExecutions(crate::domain::execution::context_execution::ContextExecution),
    StatementExecutions(crate::domain::execution::enriched_records::StatementExecution),
    BodyExecutions(crate::domain::execution::enriched_records::BodyExecution),
    SourceExecutionInvocations(
        crate::domain::execution::enriched_records::SourceExecutionInvocation,
    ),
    ContextEntryBindings(crate::domain::execution::context_binding::ContextEntryBinding),
    SummarySymbolicFieldAlternatives(
        crate::domain::execution::summary_symbolic::SymbolicFieldAlternative,
    ),
    ModelContextTransferWitnesses(
        crate::domain::execution::model_context_transfer::ContextTransferWitness,
    ),
    SourceCallOutcomes(crate::domain::execution::source_call_records::SourceCallOutcome),
    SourceCallFrameReleases(crate::domain::execution::source_call_records::SourceFrameRelease),
    SummaryTransferPremises(crate::domain::transfer::summary::SummaryPremise),
    SummaryTransferWitnesses(crate::domain::transfer::summary::SummaryWitness),
    AnalyticConclusionSources(crate::domain::analytics::ConclusionSource),
    NormalizationCoverage(crate::domain::normalized::coverage::NormalizationCoverage),
    ReferenceEntityCandidates(crate::domain::normalized::links::ReferenceEntityCandidate),
    AncestryEntityMembers(crate::domain::normalized::links::AncestryEntityMember),
    MentionEntityCandidates(crate::domain::normalized::links::MentionEntityCandidate),
    TestOperandTypeLinks(crate::domain::normalized::links::TestOperandTypeLink),
    OccurrenceOwnership(crate::domain::normalized::entities::OccurrenceOwnership),
    NormalizedCallEvents(crate::domain::normalized::events::NormalizedCallEvent),
    NormalizedCallAlternatives(crate::domain::normalized::events::NormalizedCallAlternative),
    EffectiveCallableAssessments(crate::domain::normalized::callables::EffectiveCallableAssessment),
    BindingSetAssessments(crate::domain::normalized::bindings::BindingSetAssessment),
    ReceiverAssessments(crate::domain::normalized::receiver::ReceiverAssessment),
    NormalizedDispatchMembers(crate::domain::normalized::dispatch::DispatchMember),
    LocalControlInfluences(crate::domain::transfer::local::ControlInfluence),
}
impl Key for ClaimValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::LocalControlSupports(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::local::ControlSupport as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalTransferSelections(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::local::Selection as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }

            Self::CallResolutionMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::calls::CallResolutionMember as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SignatureEnumerationMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::calls::SignatureEnumerationMember as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityLabelMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analytics::CommunityLabelMember as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BindingSetMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::bindings::BindingSetMember as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BindingSetCoverage(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::bindings::BindingSetCoverage as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedDispatchPremises(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::dispatch::DispatchPremise as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedDispatchEvidence(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::dispatch::DispatchEvidence as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryControlInfluences(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::ControlInfluence as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryControlSupports(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::ControlSupport as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryTransferSelections(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::Selection as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }

            Self::AssumptionUniverseSupports(row) => {
                sink.part(b"semantic-type",<crate::domain::assumptions_universe::AssumptionUniverseSupport as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticTextAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::embedding::text::TextAssessment as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisConfiguredSeedDecisions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::seeds::ConfiguredSeedDecision as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisAutomaticSeedDecisions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::automatic::Decision as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralReaches(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::Reach as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralUnfollowedArguments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::controls::UnfollowedArgument as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralHandoffValues(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::handoffs::ValueSource as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralHandoffOccurrences(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::handoffs::Handoff as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryBehavioralConclusions(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::summary_consequences::ClaimConclusion as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceFieldClassAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::symbolic_fields::SourceFieldClass as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceFieldStores(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::symbolic_fields::SourceFieldStore as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceFieldReaders(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::symbolic_fields::SourceFieldReader as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceFieldAssociations(row) => {
                sink.part(b"semantic-type",<crate::domain::normalized::symbolic_fields::SourceFieldAssociation as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallEventAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::events::EventAssessment as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }

            Self::LocalTypeDomains(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::local_theory::TypeDomain as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalTypeDomainAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::local_theory::TypeDomainAssessment as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalTypeDomainMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::local_theory::TypeDomainMember as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ReportEntries(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::deployment::ReportEntry as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CallOriginSteps(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::calls::CallOriginStep as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalSymbolicFieldStores(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::local_symbolic::SymbolicFieldStore as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::FlowCaptureCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::flow_capture::FlowCaptureCandidate as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalAtomDecisions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::atom_decision::AtomDecision as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::FlowCallSteps(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::flow::FlowCallStep as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ProjectionSourceAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::projection::ProjectionSourceAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ProjectionGaps(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::projection::ProjectionGap as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ProjectionSourceCoverages(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::projection::ProjectionSourceCoverage as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticTextWindows(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::embedding::text::TextWindow as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisSeedPlans(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::seeds::SeedPlan as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisSelectedSeedSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::seeds::SelectedSeedSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisAuthoredCodeSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::patterns::AuthoredCodeSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisProseSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::documentary::ProseSource as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::EntryAccessSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::conditions::entry::EntryAccessSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralPublicCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::PublicCandidate as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralTraversals(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::Traversal as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralPaths(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::Path as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralUnresolvedEvents(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::UnresolvedEvent as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralLiteralArguments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::controls::LiteralArgument as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralControlPaths(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::controls::ControlPath as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralConditionalRaises(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::controls::ConditionalRaise as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralUnfollowedPaths(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::controls::UnfollowedPath as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralHandoffGroups(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::handoffs::Group as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ClosedTargetAssessments(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::closed_targets::ClosedTargetAssessment as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelContextResources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::model_protocol::ContextResource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CapturedEntryBindings(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::capture_bridge::CapturedEntryBinding as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticDocumentNeighbours(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analytics::DocumentNeighbour as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityLabels(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analytics::CommunityLabel as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceFieldReaderLinks(row) => {
                sink.part(b"semantic-type",<crate::domain::normalized::symbolic_fields::SourceFieldReaderLink as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassFieldDefaultAssessments(row) => {
                sink.part(b"semantic-type",<crate::domain::normalized::callable_aspects::FieldDefaultAssessment as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ReferenceEntityTargets(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::ReferenceEntityTarget as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ReferenceEntityAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::ReferenceEntityAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AncestryEntityAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::AncestryEntityAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::MentionEntityAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::MentionEntityAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::TestOperandTypeAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::TestOperandTypeAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SymbolEntityCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::SymbolEntityCandidate as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ParameterEntityLinks(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::ParameterEntityLink as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::FieldEntityLinks(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::FieldEntityLink as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::FieldDeclarationLinks(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::FieldDeclarationLink as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::PublicExposureCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::PublicExposureCandidate as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedCallAlternativeSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::events::CallAlternativeSource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SignatureSlotEntities(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::callables::SignatureSlotEntity as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedDispatchAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::dispatch::DispatchAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalTransferAlternatives(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::local::TransferAlternative as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalTransferSupports(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::local::TransferSupport as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelTransferAlternatives(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::model::TransferAlternative as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelTransferSupports(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::model::TransferSupport as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryTransferAlternatives(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::TransferAlternative as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryTransferSupports(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::TransferSupport as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }

            Self::LocalBuiltinOperandWitnesses(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::local_theory::BuiltinOperandWitness as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BindingSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::calls::BindingSource as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::BindingProjections(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::calls::BindingProjection as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalAtomRestrictions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::atom_decision::AtomRestriction as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalysisEmbeddingUses(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::embedding::analytic::AnalysisEmbeddingUse as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::CatalogMemberInvocations(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::catalog::CatalogMemberInvocation as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisSelectedSeeds(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::seeds::SelectedSeed as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisAuthoredCodeConclusions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::patterns::AuthoredCodeConclusion as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisProseSlices(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::documentary::ProseSlice as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SynthesisDocumentarySources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::synthesis::documentary::DocumentarySource as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::EntryValueWitnesses(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::conditions::entry::EntryValueWitness as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StabilityWitnesses(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::conditions::stability::StabilityWitness as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StructuralConclusionSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::structural::ConclusionSource as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryTerminalWitnesses(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::summary_terminal::SummaryTerminalWitness as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseStatementCompletions(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::completion_records::StatementCompletion as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModeledCallEvaluations(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::modeled_call::ModeledCallEvaluation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryPathRoutes(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::summary_path::SummaryPathRoute as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelApplications(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::model_production::ModelApplication as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryCaptureWitnesses(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::summary_capture::SummaryCaptureWitness as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::BaseSourceBodyCompletions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::body_records::SourceBodyCompletion as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ConditionalTerminalFrontiers(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::protocol_interpretation::ConditionalTerminalFrontier as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::NormalContinuationRestrictions(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::protocol_interpretation::NormalContinuationRestriction as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryClaimProofs(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::summary_consequences::ClaimProof as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryControlWitnesses(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::summary_control::SummaryControlWitness as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::DefinitionEvaluations(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::definition::DefinitionEvaluation as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ModelAppliedRules(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::model_rules::AppliedRule as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ContextExecutions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::context_execution::ContextExecution as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::StatementExecutions(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::enriched_records::StatementExecution as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::BodyExecutions(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::execution::enriched_records::BodyExecution as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SourceExecutionInvocations(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::enriched_records::SourceExecutionInvocation as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ContextEntryBindings(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::context_binding::ContextEntryBinding as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummarySymbolicFieldAlternatives(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::summary_symbolic::SymbolicFieldAlternative as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelContextTransferWitnesses(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::model_context_transfer::ContextTransferWitness as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallOutcomes(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::source_call_records::SourceCallOutcome as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCallFrameReleases(row) => {
                sink.part(b"semantic-type",<crate::domain::execution::source_call_records::SourceFrameRelease as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryTransferPremises(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::SummaryPremise as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::SummaryTransferWitnesses(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::summary::SummaryWitness as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AnalyticConclusionSources(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::analytics::ConclusionSource as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizationCoverage(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::coverage::NormalizationCoverage as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ReferenceEntityCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::ReferenceEntityCandidate as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::AncestryEntityMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::AncestryEntityMember as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::MentionEntityCandidates(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::MentionEntityCandidate as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::TestOperandTypeLinks(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::links::TestOperandTypeLink as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::OccurrenceOwnership(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::entities::OccurrenceOwnership as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedCallEvents(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::events::NormalizedCallEvent as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedCallAlternatives(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::events::NormalizedCallAlternative as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::EffectiveCallableAssessments(row) => {
                sink.part(b"semantic-type",<crate::domain::normalized::callables::EffectiveCallableAssessment as Record>::NAME.as_bytes());
                row.content_digest().encode(sink);
            }
            Self::BindingSetAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::bindings::BindingSetAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::ReceiverAssessments(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::receiver::ReceiverAssessment as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::NormalizedDispatchMembers(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::normalized::dispatch::DispatchMember as Record>::NAME
                        .as_bytes(),
                );
                row.content_digest().encode(sink);
            }
            Self::LocalControlInfluences(row) => {
                sink.part(
                    b"semantic-type",
                    <crate::domain::transfer::local::ControlInfluence as Record>::NAME.as_bytes(),
                );
                row.content_digest().encode(sink);
            }
        }
    }
}
impl ClaimValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::LocalControlSupports(row) => row.validate(),
            Self::LocalTransferSelections(row) => row.validate(),

            Self::CallResolutionMembers(row) => row.validate(),
            Self::SignatureEnumerationMembers(row) => row.validate(),
            Self::AnalyticCommunityLabelMembers(row) => row.validate(),
            Self::BindingSetMembers(row) => row.validate(),
            Self::BindingSetCoverage(row) => row.validate(),
            Self::NormalizedDispatchPremises(row) => row.validate(),
            Self::NormalizedDispatchEvidence(row) => row.validate(),
            Self::SummaryControlInfluences(row) => row.validate(),
            Self::SummaryControlSupports(row) => row.validate(),
            Self::SummaryTransferSelections(row) => row.validate(),

            Self::AssumptionUniverseSupports(row) => row.validate(),
            Self::AnalyticTextAssessments(row) => row.validate(),
            Self::SynthesisConfiguredSeedDecisions(row) => row.validate(),
            Self::SynthesisAutomaticSeedDecisions(row) => row.validate(),
            Self::StructuralReaches(row) => row.validate(),
            Self::StructuralUnfollowedArguments(row) => row.validate(),
            Self::StructuralHandoffValues(row) => row.validate(),
            Self::StructuralHandoffOccurrences(row) => row.validate(),
            Self::SummaryBehavioralConclusions(row) => row.validate(),
            Self::SourceFieldClassAssessments(row) => row.validate(),
            Self::SourceFieldStores(row) => row.validate(),
            Self::SourceFieldReaders(row) => row.validate(),
            Self::SourceFieldAssociations(row) => row.validate(),
            Self::CallEventAssessments(row) => row.validate(),

            Self::LocalTypeDomains(row) => row.validate(),
            Self::LocalTypeDomainAssessments(row) => row.validate(),
            Self::LocalTypeDomainMembers(row) => row.validate(),
            Self::ReportEntries(row) => row.validate(),
            Self::CallOriginSteps(row) => row.validate(),
            Self::LocalSymbolicFieldStores(row) => row.validate(),
            Self::FlowCaptureCandidates(row) => row.validate(),
            Self::LocalAtomDecisions(row) => row.validate(),
            Self::FlowCallSteps(row) => row.validate(),
            Self::ProjectionSourceAssessments(row) => row.validate(),
            Self::ProjectionGaps(row) => row.validate(),
            Self::ProjectionSourceCoverages(row) => row.validate(),
            Self::AnalyticTextWindows(row) => row.validate(),
            Self::SynthesisSeedPlans(row) => row.validate(),
            Self::SynthesisSelectedSeedSources(row) => row.validate(),
            Self::SynthesisAuthoredCodeSources(row) => row.validate(),
            Self::SynthesisProseSources(row) => row.validate(),
            Self::EntryAccessSources(row) => row.validate(),
            Self::StructuralPublicCandidates(row) => row.validate(),
            Self::StructuralTraversals(row) => row.validate(),
            Self::StructuralPaths(row) => row.validate(),
            Self::StructuralUnresolvedEvents(row) => row.validate(),
            Self::StructuralLiteralArguments(row) => row.validate(),
            Self::StructuralControlPaths(row) => row.validate(),
            Self::StructuralConditionalRaises(row) => row.validate(),
            Self::StructuralUnfollowedPaths(row) => row.validate(),
            Self::StructuralHandoffGroups(row) => row.validate(),
            Self::ClosedTargetAssessments(row) => row.validate(),
            Self::ModelContextResources(row) => row.validate(),
            Self::CapturedEntryBindings(row) => row.validate(),
            Self::AnalyticDocumentNeighbours(row) => row.validate(),
            Self::AnalyticCommunityLabels(row) => row.validate(),
            Self::SourceFieldReaderLinks(row) => row.validate(),
            Self::ClassFieldDefaultAssessments(row) => row.validate(),
            Self::ReferenceEntityTargets(row) => row.validate(),
            Self::ReferenceEntityAssessments(row) => row.validate(),
            Self::AncestryEntityAssessments(row) => row.validate(),
            Self::MentionEntityAssessments(row) => row.validate(),
            Self::TestOperandTypeAssessments(row) => row.validate(),
            Self::SymbolEntityCandidates(row) => row.validate(),
            Self::ParameterEntityLinks(row) => row.validate(),
            Self::FieldEntityLinks(row) => row.validate(),
            Self::FieldDeclarationLinks(row) => row.validate(),
            Self::PublicExposureCandidates(row) => row.validate(),
            Self::NormalizedCallAlternativeSources(row) => row.validate(),
            Self::SignatureSlotEntities(row) => row.validate(),
            Self::NormalizedDispatchAssessments(row) => row.validate(),
            Self::LocalTransferAlternatives(row) => row.validate(),
            Self::LocalTransferSupports(row) => row.validate(),
            Self::ModelTransferAlternatives(row) => row.validate(),
            Self::ModelTransferSupports(row) => row.validate(),
            Self::SummaryTransferAlternatives(row) => row.validate(),
            Self::SummaryTransferSupports(row) => row.validate(),

            Self::LocalBuiltinOperandWitnesses(row) => row.validate(),
            Self::BindingSources(row) => row.validate(),
            Self::BindingProjections(row) => row.validate(),
            Self::LocalAtomRestrictions(row) => row.validate(),
            Self::AnalysisEmbeddingUses(row) => row.validate(),
            Self::CatalogMemberInvocations(row) => row.validate(),
            Self::SynthesisSelectedSeeds(row) => row.validate(),
            Self::SynthesisAuthoredCodeConclusions(row) => row.validate(),
            Self::SynthesisProseSlices(row) => row.validate(),
            Self::SynthesisDocumentarySources(row) => row.validate(),
            Self::EntryValueWitnesses(row) => row.validate(),
            Self::StabilityWitnesses(row) => row.validate(),
            Self::StructuralConclusionSources(row) => row.validate(),
            Self::SummaryTerminalWitnesses(row) => row.validate(),
            Self::BaseStatementCompletions(row) => row.validate(),
            Self::ModeledCallEvaluations(row) => row.validate(),
            Self::SummaryPathRoutes(row) => row.validate(),
            Self::ModelApplications(row) => row.validate(),
            Self::SummaryCaptureWitnesses(row) => row.validate(),
            Self::BaseSourceBodyCompletions(row) => row.validate(),
            Self::ConditionalTerminalFrontiers(row) => row.validate(),
            Self::NormalContinuationRestrictions(row) => row.validate(),
            Self::SummaryClaimProofs(row) => row.validate(),
            Self::SummaryControlWitnesses(row) => row.validate(),
            Self::DefinitionEvaluations(row) => row.validate(),
            Self::ModelAppliedRules(row) => row.validate(),
            Self::ContextExecutions(row) => row.validate(),
            Self::StatementExecutions(row) => row.validate(),
            Self::BodyExecutions(row) => row.validate(),
            Self::SourceExecutionInvocations(row) => row.validate(),
            Self::ContextEntryBindings(row) => row.validate(),
            Self::SummarySymbolicFieldAlternatives(row) => row.validate(),
            Self::ModelContextTransferWitnesses(row) => row.validate(),
            Self::SourceCallOutcomes(row) => row.validate(),
            Self::SourceCallFrameReleases(row) => row.validate(),
            Self::SummaryTransferPremises(row) => row.validate(),
            Self::SummaryTransferWitnesses(row) => row.validate(),
            Self::AnalyticConclusionSources(row) => row.validate(),
            Self::NormalizationCoverage(row) => row.validate(),
            Self::ReferenceEntityCandidates(row) => row.validate(),
            Self::AncestryEntityMembers(row) => row.validate(),
            Self::MentionEntityCandidates(row) => row.validate(),
            Self::TestOperandTypeLinks(row) => row.validate(),
            Self::OccurrenceOwnership(row) => row.validate(),
            Self::NormalizedCallEvents(row) => row.validate(),
            Self::NormalizedCallAlternatives(row) => row.validate(),
            Self::EffectiveCallableAssessments(row) => row.validate(),
            Self::BindingSetAssessments(row) => row.validate(),
            Self::ReceiverAssessments(row) => row.validate(),
            Self::NormalizedDispatchMembers(row) => row.validate(),
            Self::LocalControlInfluences(row) => row.validate(),
        }
    }
}
impl ClaimValue {
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::LocalControlSupports(row) => row.references(),
            Self::LocalTransferSelections(row) => row.references(),

            Self::CallResolutionMembers(row) => row.references(),
            Self::SignatureEnumerationMembers(row) => row.references(),
            Self::AnalyticCommunityLabelMembers(row) => row.references(),
            Self::BindingSetMembers(row) => row.references(),
            Self::BindingSetCoverage(row) => row.references(),
            Self::NormalizedDispatchPremises(row) => row.references(),
            Self::NormalizedDispatchEvidence(row) => row.references(),
            Self::SummaryControlInfluences(row) => row.references(),
            Self::SummaryControlSupports(row) => row.references(),
            Self::SummaryTransferSelections(row) => row.references(),

            Self::AssumptionUniverseSupports(row) => row.references(),
            Self::AnalyticTextAssessments(row) => row.references(),
            Self::SynthesisConfiguredSeedDecisions(row) => row.references(),
            Self::SynthesisAutomaticSeedDecisions(row) => row.references(),
            Self::StructuralReaches(row) => row.references(),
            Self::StructuralUnfollowedArguments(row) => row.references(),
            Self::StructuralHandoffValues(row) => row.references(),
            Self::StructuralHandoffOccurrences(row) => row.references(),
            Self::SummaryBehavioralConclusions(row) => row.references(),
            Self::SourceFieldClassAssessments(row) => row.references(),
            Self::SourceFieldStores(row) => row.references(),
            Self::SourceFieldReaders(row) => row.references(),
            Self::SourceFieldAssociations(row) => row.references(),
            Self::CallEventAssessments(row) => row.references(),

            Self::LocalTypeDomains(row) => row.references(),
            Self::LocalTypeDomainAssessments(row) => row.references(),
            Self::LocalTypeDomainMembers(row) => row.references(),
            Self::ReportEntries(row) => row.references(),
            Self::CallOriginSteps(row) => row.references(),
            Self::LocalSymbolicFieldStores(row) => row.references(),
            Self::FlowCaptureCandidates(row) => row.references(),
            Self::LocalAtomDecisions(row) => row.references(),
            Self::FlowCallSteps(row) => row.references(),
            Self::ProjectionSourceAssessments(row) => row.references(),
            Self::ProjectionGaps(row) => row.references(),
            Self::ProjectionSourceCoverages(row) => row.references(),
            Self::AnalyticTextWindows(row) => row.references(),
            Self::SynthesisSeedPlans(row) => row.references(),
            Self::SynthesisSelectedSeedSources(row) => row.references(),
            Self::SynthesisAuthoredCodeSources(row) => row.references(),
            Self::SynthesisProseSources(row) => row.references(),
            Self::EntryAccessSources(row) => row.references(),
            Self::StructuralPublicCandidates(row) => row.references(),
            Self::StructuralTraversals(row) => row.references(),
            Self::StructuralPaths(row) => row.references(),
            Self::StructuralUnresolvedEvents(row) => row.references(),
            Self::StructuralLiteralArguments(row) => row.references(),
            Self::StructuralControlPaths(row) => row.references(),
            Self::StructuralConditionalRaises(row) => row.references(),
            Self::StructuralUnfollowedPaths(row) => row.references(),
            Self::StructuralHandoffGroups(row) => row.references(),
            Self::ClosedTargetAssessments(row) => row.references(),
            Self::ModelContextResources(row) => row.references(),
            Self::CapturedEntryBindings(row) => row.references(),
            Self::AnalyticDocumentNeighbours(row) => row.references(),
            Self::AnalyticCommunityLabels(row) => row.references(),
            Self::SourceFieldReaderLinks(row) => row.references(),
            Self::ClassFieldDefaultAssessments(row) => row.references(),
            Self::ReferenceEntityTargets(row) => row.references(),
            Self::ReferenceEntityAssessments(row) => row.references(),
            Self::AncestryEntityAssessments(row) => row.references(),
            Self::MentionEntityAssessments(row) => row.references(),
            Self::TestOperandTypeAssessments(row) => row.references(),
            Self::SymbolEntityCandidates(row) => row.references(),
            Self::ParameterEntityLinks(row) => row.references(),
            Self::FieldEntityLinks(row) => row.references(),
            Self::FieldDeclarationLinks(row) => row.references(),
            Self::PublicExposureCandidates(row) => row.references(),
            Self::NormalizedCallAlternativeSources(row) => row.references(),
            Self::SignatureSlotEntities(row) => row.references(),
            Self::NormalizedDispatchAssessments(row) => row.references(),
            Self::LocalTransferAlternatives(row) => row.references(),
            Self::LocalTransferSupports(row) => row.references(),
            Self::ModelTransferAlternatives(row) => row.references(),
            Self::ModelTransferSupports(row) => row.references(),
            Self::SummaryTransferAlternatives(row) => row.references(),
            Self::SummaryTransferSupports(row) => row.references(),

            Self::LocalBuiltinOperandWitnesses(row) => row.references(),
            Self::BindingSources(row) => row.references(),
            Self::BindingProjections(row) => row.references(),
            Self::LocalAtomRestrictions(row) => row.references(),
            Self::AnalysisEmbeddingUses(row) => row.references(),
            Self::CatalogMemberInvocations(row) => row.references(),
            Self::SynthesisSelectedSeeds(row) => row.references(),
            Self::SynthesisAuthoredCodeConclusions(row) => row.references(),
            Self::SynthesisProseSlices(row) => row.references(),
            Self::SynthesisDocumentarySources(row) => row.references(),
            Self::EntryValueWitnesses(row) => row.references(),
            Self::StabilityWitnesses(row) => row.references(),
            Self::StructuralConclusionSources(row) => row.references(),
            Self::SummaryTerminalWitnesses(row) => row.references(),
            Self::BaseStatementCompletions(row) => row.references(),
            Self::ModeledCallEvaluations(row) => row.references(),
            Self::SummaryPathRoutes(row) => row.references(),
            Self::ModelApplications(row) => row.references(),
            Self::SummaryCaptureWitnesses(row) => row.references(),
            Self::BaseSourceBodyCompletions(row) => row.references(),
            Self::ConditionalTerminalFrontiers(row) => row.references(),
            Self::NormalContinuationRestrictions(row) => row.references(),
            Self::SummaryClaimProofs(row) => row.references(),
            Self::SummaryControlWitnesses(row) => row.references(),
            Self::DefinitionEvaluations(row) => row.references(),
            Self::ModelAppliedRules(row) => row.references(),
            Self::ContextExecutions(row) => row.references(),
            Self::StatementExecutions(row) => row.references(),
            Self::BodyExecutions(row) => row.references(),
            Self::SourceExecutionInvocations(row) => row.references(),
            Self::ContextEntryBindings(row) => row.references(),
            Self::SummarySymbolicFieldAlternatives(row) => row.references(),
            Self::ModelContextTransferWitnesses(row) => row.references(),
            Self::SourceCallOutcomes(row) => row.references(),
            Self::SourceCallFrameReleases(row) => row.references(),
            Self::SummaryTransferPremises(row) => row.references(),
            Self::SummaryTransferWitnesses(row) => row.references(),
            Self::AnalyticConclusionSources(row) => row.references(),
            Self::NormalizationCoverage(row) => row.references(),
            Self::ReferenceEntityCandidates(row) => row.references(),
            Self::AncestryEntityMembers(row) => row.references(),
            Self::MentionEntityCandidates(row) => row.references(),
            Self::TestOperandTypeLinks(row) => row.references(),
            Self::OccurrenceOwnership(row) => row.references(),
            Self::NormalizedCallEvents(row) => row.references(),
            Self::NormalizedCallAlternatives(row) => row.references(),
            Self::EffectiveCallableAssessments(row) => row.references(),
            Self::BindingSetAssessments(row) => row.references(),
            Self::ReceiverAssessments(row) => row.references(),
            Self::NormalizedDispatchMembers(row) => row.references(),
            Self::LocalControlInfluences(row) => row.references(),
        }
    }
}
impl ClaimValue {
    pub fn semantic_key(&self) -> SemanticKey {
        match self {
            Self::LocalControlSupports(row) => SemanticKey::of(row.id()),
            Self::LocalTransferSelections(row) => SemanticKey::of(row.id()),

            Self::CallResolutionMembers(row) => SemanticKey::of(row.id()),
            Self::SignatureEnumerationMembers(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityLabelMembers(row) => SemanticKey::of(row.id()),
            Self::BindingSetMembers(row) => SemanticKey::of(row.id()),
            Self::BindingSetCoverage(row) => SemanticKey::of(row.id()),
            Self::NormalizedDispatchPremises(row) => SemanticKey::of(row.id()),
            Self::NormalizedDispatchEvidence(row) => SemanticKey::of(row.id()),
            Self::SummaryControlInfluences(row) => SemanticKey::of(row.id()),
            Self::SummaryControlSupports(row) => SemanticKey::of(row.id()),
            Self::SummaryTransferSelections(row) => SemanticKey::of(row.id()),

            Self::AssumptionUniverseSupports(row) => SemanticKey::of(row.id()),
            Self::AnalyticTextAssessments(row) => SemanticKey::of(row.id()),
            Self::SynthesisConfiguredSeedDecisions(row) => SemanticKey::of(row.id()),
            Self::SynthesisAutomaticSeedDecisions(row) => SemanticKey::of(row.id()),
            Self::StructuralReaches(row) => SemanticKey::of(row.id()),
            Self::StructuralUnfollowedArguments(row) => SemanticKey::of(row.id()),
            Self::StructuralHandoffValues(row) => SemanticKey::of(row.id()),
            Self::StructuralHandoffOccurrences(row) => SemanticKey::of(row.id()),
            Self::SummaryBehavioralConclusions(row) => SemanticKey::of(row.id()),
            Self::SourceFieldClassAssessments(row) => SemanticKey::of(row.id()),
            Self::SourceFieldStores(row) => SemanticKey::of(row.id()),
            Self::SourceFieldReaders(row) => SemanticKey::of(row.id()),
            Self::SourceFieldAssociations(row) => SemanticKey::of(row.id()),
            Self::CallEventAssessments(row) => SemanticKey::of(row.id()),

            Self::LocalTypeDomains(row) => SemanticKey::of(row.id()),
            Self::LocalTypeDomainAssessments(row) => SemanticKey::of(row.id()),
            Self::LocalTypeDomainMembers(row) => SemanticKey::of(row.id()),
            Self::ReportEntries(row) => SemanticKey::of(row.id()),
            Self::CallOriginSteps(row) => SemanticKey::of(row.id()),
            Self::LocalSymbolicFieldStores(row) => SemanticKey::of(row.id()),
            Self::FlowCaptureCandidates(row) => SemanticKey::of(row.id()),
            Self::LocalAtomDecisions(row) => SemanticKey::of(row.id()),
            Self::FlowCallSteps(row) => SemanticKey::of(row.id()),
            Self::ProjectionSourceAssessments(row) => SemanticKey::of(row.id()),
            Self::ProjectionGaps(row) => SemanticKey::of(row.id()),
            Self::ProjectionSourceCoverages(row) => SemanticKey::of(row.id()),
            Self::AnalyticTextWindows(row) => SemanticKey::of(row.id()),
            Self::SynthesisSeedPlans(row) => SemanticKey::of(row.id()),
            Self::SynthesisSelectedSeedSources(row) => SemanticKey::of(row.id()),
            Self::SynthesisAuthoredCodeSources(row) => SemanticKey::of(row.id()),
            Self::SynthesisProseSources(row) => SemanticKey::of(row.id()),
            Self::EntryAccessSources(row) => SemanticKey::of(row.id()),
            Self::StructuralPublicCandidates(row) => SemanticKey::of(row.id()),
            Self::StructuralTraversals(row) => SemanticKey::of(row.id()),
            Self::StructuralPaths(row) => SemanticKey::of(row.id()),
            Self::StructuralUnresolvedEvents(row) => SemanticKey::of(row.id()),
            Self::StructuralLiteralArguments(row) => SemanticKey::of(row.id()),
            Self::StructuralControlPaths(row) => SemanticKey::of(row.id()),
            Self::StructuralConditionalRaises(row) => SemanticKey::of(row.id()),
            Self::StructuralUnfollowedPaths(row) => SemanticKey::of(row.id()),
            Self::StructuralHandoffGroups(row) => SemanticKey::of(row.id()),
            Self::ClosedTargetAssessments(row) => SemanticKey::of(row.id()),
            Self::ModelContextResources(row) => SemanticKey::of(row.id()),
            Self::CapturedEntryBindings(row) => SemanticKey::of(row.id()),
            Self::AnalyticDocumentNeighbours(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityLabels(row) => SemanticKey::of(row.id()),
            Self::SourceFieldReaderLinks(row) => SemanticKey::of(row.id()),
            Self::ClassFieldDefaultAssessments(row) => SemanticKey::of(row.id()),
            Self::ReferenceEntityTargets(row) => SemanticKey::of(row.id()),
            Self::ReferenceEntityAssessments(row) => SemanticKey::of(row.id()),
            Self::AncestryEntityAssessments(row) => SemanticKey::of(row.id()),
            Self::MentionEntityAssessments(row) => SemanticKey::of(row.id()),
            Self::TestOperandTypeAssessments(row) => SemanticKey::of(row.id()),
            Self::SymbolEntityCandidates(row) => SemanticKey::of(row.id()),
            Self::ParameterEntityLinks(row) => SemanticKey::of(row.id()),
            Self::FieldEntityLinks(row) => SemanticKey::of(row.id()),
            Self::FieldDeclarationLinks(row) => SemanticKey::of(row.id()),
            Self::PublicExposureCandidates(row) => SemanticKey::of(row.id()),
            Self::NormalizedCallAlternativeSources(row) => SemanticKey::of(row.id()),
            Self::SignatureSlotEntities(row) => SemanticKey::of(row.id()),
            Self::NormalizedDispatchAssessments(row) => SemanticKey::of(row.id()),
            Self::LocalTransferAlternatives(row) => SemanticKey::of(row.id()),
            Self::LocalTransferSupports(row) => SemanticKey::of(row.id()),
            Self::ModelTransferAlternatives(row) => SemanticKey::of(row.id()),
            Self::ModelTransferSupports(row) => SemanticKey::of(row.id()),
            Self::SummaryTransferAlternatives(row) => SemanticKey::of(row.id()),
            Self::SummaryTransferSupports(row) => SemanticKey::of(row.id()),

            Self::LocalBuiltinOperandWitnesses(row) => SemanticKey::of(row.id()),
            Self::BindingSources(row) => SemanticKey::of(row.id()),
            Self::BindingProjections(row) => SemanticKey::of(row.id()),
            Self::LocalAtomRestrictions(row) => SemanticKey::of(row.id()),
            Self::AnalysisEmbeddingUses(row) => SemanticKey::of(row.id()),
            Self::CatalogMemberInvocations(row) => SemanticKey::of(row.id()),
            Self::SynthesisSelectedSeeds(row) => SemanticKey::of(row.id()),
            Self::SynthesisAuthoredCodeConclusions(row) => SemanticKey::of(row.id()),
            Self::SynthesisProseSlices(row) => SemanticKey::of(row.id()),
            Self::SynthesisDocumentarySources(row) => SemanticKey::of(row.id()),
            Self::EntryValueWitnesses(row) => SemanticKey::of(row.id()),
            Self::StabilityWitnesses(row) => SemanticKey::of(row.id()),
            Self::StructuralConclusionSources(row) => SemanticKey::of(row.id()),
            Self::SummaryTerminalWitnesses(row) => SemanticKey::of(row.id()),
            Self::BaseStatementCompletions(row) => SemanticKey::of(row.id()),
            Self::ModeledCallEvaluations(row) => SemanticKey::of(row.id()),
            Self::SummaryPathRoutes(row) => SemanticKey::of(row.id()),
            Self::ModelApplications(row) => SemanticKey::of(row.id()),
            Self::SummaryCaptureWitnesses(row) => SemanticKey::of(row.id()),
            Self::BaseSourceBodyCompletions(row) => SemanticKey::of(row.id()),
            Self::ConditionalTerminalFrontiers(row) => SemanticKey::of(row.id()),
            Self::NormalContinuationRestrictions(row) => SemanticKey::of(row.id()),
            Self::SummaryClaimProofs(row) => SemanticKey::of(row.id()),
            Self::SummaryControlWitnesses(row) => SemanticKey::of(row.id()),
            Self::DefinitionEvaluations(row) => SemanticKey::of(row.id()),
            Self::ModelAppliedRules(row) => SemanticKey::of(row.id()),
            Self::ContextExecutions(row) => SemanticKey::of(row.id()),
            Self::StatementExecutions(row) => SemanticKey::of(row.id()),
            Self::BodyExecutions(row) => SemanticKey::of(row.id()),
            Self::SourceExecutionInvocations(row) => SemanticKey::of(row.id()),
            Self::ContextEntryBindings(row) => SemanticKey::of(row.id()),
            Self::SummarySymbolicFieldAlternatives(row) => SemanticKey::of(row.id()),
            Self::ModelContextTransferWitnesses(row) => SemanticKey::of(row.id()),
            Self::SourceCallOutcomes(row) => SemanticKey::of(row.id()),
            Self::SourceCallFrameReleases(row) => SemanticKey::of(row.id()),
            Self::SummaryTransferPremises(row) => SemanticKey::of(row.id()),
            Self::SummaryTransferWitnesses(row) => SemanticKey::of(row.id()),
            Self::AnalyticConclusionSources(row) => SemanticKey::of(row.id()),
            Self::NormalizationCoverage(row) => SemanticKey::of(row.id()),
            Self::ReferenceEntityCandidates(row) => SemanticKey::of(row.id()),
            Self::AncestryEntityMembers(row) => SemanticKey::of(row.id()),
            Self::MentionEntityCandidates(row) => SemanticKey::of(row.id()),
            Self::TestOperandTypeLinks(row) => SemanticKey::of(row.id()),
            Self::OccurrenceOwnership(row) => SemanticKey::of(row.id()),
            Self::NormalizedCallEvents(row) => SemanticKey::of(row.id()),
            Self::NormalizedCallAlternatives(row) => SemanticKey::of(row.id()),
            Self::EffectiveCallableAssessments(row) => SemanticKey::of(row.id()),
            Self::BindingSetAssessments(row) => SemanticKey::of(row.id()),
            Self::ReceiverAssessments(row) => SemanticKey::of(row.id()),
            Self::NormalizedDispatchMembers(row) => SemanticKey::of(row.id()),
            Self::LocalControlInfluences(row) => SemanticKey::of(row.id()),
        }
    }
}
