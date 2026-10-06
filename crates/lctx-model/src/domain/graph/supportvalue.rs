use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SupportValue {
    UseSupport(crate::domain::flow::FlowUseSupport),
    DefinitionSupport(crate::domain::flow::FlowDefinitionSupport),
    ReachingSupport(crate::domain::flow::FlowReachingSupport),
    ValueSupport(crate::domain::flow::FlowValueSupport),
    RegionSupport(crate::domain::flow::FlowRegionSupport),
    TestSupport(crate::domain::flow::FlowTestSupport),
    LeafSupport(crate::domain::flow::FlowTestLeafSupport),
    SignatureSupport(crate::domain::calls::SignatureSupport),
    CallTargetSupport(crate::domain::calls::CallTargetSupport),
    ProviderCallSiteSupport(crate::domain::calls::ProviderCallSiteSupport),
    CallSyntaxSupport(crate::domain::calls::CallSyntaxSupport),
    CallResolutionSupport(crate::domain::calls::CallResolutionSupport),
    SymbolDeclarationSupport(crate::domain::declarations::SymbolDeclarationSupport),
    ParameterDeclarationSupport(crate::domain::declarations::ParameterDeclarationSupport),
    TaskReportObservationSupport(crate::domain::deployment::TaskReportSupport),
    DeploymentObservationSupport(crate::domain::deployment::DeploymentSupport),
    DocumentObservationSupport(crate::domain::documents::DocumentSupport),
    PassageObservationSupport(crate::domain::documents::PassageSupport),
    CodeBlockObservationSupport(crate::domain::documents::CodeBlockSupport),
    DocumentLinkObservationSupport(crate::domain::documents::DocumentLinkSupport),
    DocumentMentionObservationSupport(crate::domain::documents::DocumentMentionSupport),
    DocumentComponentObservationSupport(crate::domain::documents::DocumentComponentSupport),
    DocumentAttributeObservationSupport(crate::domain::documents::DocumentAttributeSupport),
    FlowAttributeLoadObservationSupport(crate::domain::flow::FlowAttributeLoadSupport),
    FlowValuePathObservationSupport(crate::domain::flow::FlowValuePathSupport),
    LexicalScopeObservationSupport(crate::domain::lexical::LexicalScopeSupport),
    BindingObservationSupport(crate::domain::lexical::BindingSupport),
    ReferenceObservationSupport(crate::domain::lexical::ReferenceSupport),
    LexicalResolutionSupport(crate::domain::lexical::LexicalResolutionSupport),
    SyntaxObservationSupport(crate::domain::source::SyntaxSupport),
    SymbolObservationSupport(crate::domain::symbols::SymbolSupport),
    FunctionTraitObservationSupport(crate::domain::symbols::FunctionTraitSupport),
    ClassTraitObservationSupport(crate::domain::symbols::ClassTraitSupport),
    ClassAncestryObservationSupport(crate::domain::symbols::ClassAncestrySupport),
    ParameterAnnotationObservationSupport(crate::domain::symbols::ParameterAnnotationSupport),
    PublicNameObservationSupport(crate::domain::symbols::PublicNameSupport),
    ParameterDocObservationSupport(crate::domain::symbols::ParameterDocSupport),
    SyntaxPlacementSupport(crate::domain::syntax::SyntaxPlacementSupport),
    SyntaxDetailObservationSupport(crate::domain::syntax::SyntaxDetailSupport),
    DeclarationObservationSupport(crate::domain::syntax::DeclarationSupport),
    DeclarationDecoratorSupport(crate::domain::syntax::DeclarationDecoratorSupport),
    ImportAliasObservationSupport(crate::domain::syntax::ImportAliasSupport),
    DunderAllObservationSupport(crate::domain::syntax::DunderAllSupport),
    ParameterSyntaxObservationSupport(crate::domain::syntax::ParameterSyntaxSupport),
    ClassFieldSyntaxObservationSupport(crate::domain::syntax::ClassFieldSyntaxSupport),
    TypeObservationSupport(crate::domain::types::TypeSupport),
    TypePresentationSupport(crate::domain::types::TypePresentationSupport),
    TypeVariableRestrictionSupport(crate::domain::types::TypeRestrictionSupport),
    FunctionBodyObservationSupport(crate::domain::types::FunctionBodySupport),
    RecordFieldObservationSupport(crate::domain::types::RecordFieldSupport),
    ClassMemberObservationSupport(crate::domain::class_metadata::ClassMemberSupport),
    ClassMetadataObservationSupport(crate::domain::class_metadata::ClassMetadataSupport),
    SignatureEnumerationObservationSupport(crate::domain::calls::SignatureEnumerationSupport),
    RuffContextObservationSupport(crate::domain::ruff::RuffContextSupport),
    FlowNarrowingSupport(crate::domain::flow::FlowNarrowingSupport),
    NativeExitSupport(crate::domain::protocols::NativeExitSupport),
    NativeTerminalSupport(crate::domain::protocols::NativeTerminalSupport),
    CaptureSupport(crate::domain::captures::CaptureSupport),
    NativeExitDiagnosticSupport(crate::domain::protocols::NativeExitDiagnosticSupport),
    FlowSourceViewSupport(crate::domain::flow::FlowSourceViewSupport),
    NativeSignatureObservationSupport(crate::domain::types::NativeSignatureSupport),
    SignatureTypeObservationSupport(crate::domain::types::SignatureTypeSupport),
    TypeQuerySupport(crate::domain::types::TypeQuerySupport),
    GenericSpecializationSupport(crate::domain::types::GenericSpecializationSupport),
    FlowCaptureTimingSupport(crate::domain::flow_capture::FlowCaptureTimingSupport),
    ExportEnumerationSupport(crate::domain::symbols::ExportEnumerationSupport),
    RuffBindingObservationSupport(crate::domain::ruff::RuffBindingSupport),
    RuffDiagnosticObservationSupport(crate::domain::diagnostics::RuffDiagnosticSupport),
    PyreflyDiagnosticObservationSupport(crate::domain::diagnostics::PyreflyDiagnosticSupport),
    NativeParameterDefinitionObservationSupport(
        crate::domain::diagnostics::NativeParameterDefinitionSupport,
    ),
    RuffDefinitionObservationSupport(crate::domain::ruff::RuffDefinitionSupport),
    FlowUseInventorySupport(crate::domain::flow_inventory::FlowUseInventorySupport),
    NativeOverloadSupport(crate::domain::types::NativeOverloadSupport),
    NativeOverloadCandidateSupport(crate::domain::types::NativeOverloadCandidateSupport),
    ModuleResolutionObservationSupport(crate::domain::symbols::ModuleResolutionSupport),
}
impl Key for SupportValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::UseSupport(row) => {
                sink.part(b"variant", &(0u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DefinitionSupport(row) => {
                sink.part(b"variant", &(1u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ReachingSupport(row) => {
                sink.part(b"variant", &(2u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ValueSupport(row) => {
                sink.part(b"variant", &(3u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RegionSupport(row) => {
                sink.part(b"variant", &(4u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TestSupport(row) => {
                sink.part(b"variant", &(5u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LeafSupport(row) => {
                sink.part(b"variant", &(6u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureSupport(row) => {
                sink.part(b"variant", &(7u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallTargetSupport(row) => {
                sink.part(b"variant", &(8u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ProviderCallSiteSupport(row) => {
                sink.part(b"variant", &(9u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallSyntaxSupport(row) => {
                sink.part(b"variant", &(10u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallResolutionSupport(row) => {
                sink.part(b"variant", &(11u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SymbolDeclarationSupport(row) => {
                sink.part(b"variant", &(12u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterDeclarationSupport(row) => {
                sink.part(b"variant", &(13u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TaskReportObservationSupport(row) => {
                sink.part(b"variant", &(14u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeploymentObservationSupport(row) => {
                sink.part(b"variant", &(15u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentObservationSupport(row) => {
                sink.part(b"variant", &(16u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PassageObservationSupport(row) => {
                sink.part(b"variant", &(17u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CodeBlockObservationSupport(row) => {
                sink.part(b"variant", &(18u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentLinkObservationSupport(row) => {
                sink.part(b"variant", &(19u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentMentionObservationSupport(row) => {
                sink.part(b"variant", &(20u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentComponentObservationSupport(row) => {
                sink.part(b"variant", &(21u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentAttributeObservationSupport(row) => {
                sink.part(b"variant", &(22u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowAttributeLoadObservationSupport(row) => {
                sink.part(b"variant", &(23u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowValuePathObservationSupport(row) => {
                sink.part(b"variant", &(24u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LexicalScopeObservationSupport(row) => {
                sink.part(b"variant", &(25u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BindingObservationSupport(row) => {
                sink.part(b"variant", &(26u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ReferenceObservationSupport(row) => {
                sink.part(b"variant", &(27u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LexicalResolutionSupport(row) => {
                sink.part(b"variant", &(28u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxObservationSupport(row) => {
                sink.part(b"variant", &(29u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SymbolObservationSupport(row) => {
                sink.part(b"variant", &(30u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FunctionTraitObservationSupport(row) => {
                sink.part(b"variant", &(31u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassTraitObservationSupport(row) => {
                sink.part(b"variant", &(32u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassAncestryObservationSupport(row) => {
                sink.part(b"variant", &(33u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterAnnotationObservationSupport(row) => {
                sink.part(b"variant", &(34u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PublicNameObservationSupport(row) => {
                sink.part(b"variant", &(35u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterDocObservationSupport(row) => {
                sink.part(b"variant", &(36u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxPlacementSupport(row) => {
                sink.part(b"variant", &(38u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxDetailObservationSupport(row) => {
                sink.part(b"variant", &(39u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeclarationObservationSupport(row) => {
                sink.part(b"variant", &(40u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeclarationDecoratorSupport(row) => {
                sink.part(b"variant", &(41u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ImportAliasObservationSupport(row) => {
                sink.part(b"variant", &(42u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DunderAllObservationSupport(row) => {
                sink.part(b"variant", &(43u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterSyntaxObservationSupport(row) => {
                sink.part(b"variant", &(44u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassFieldSyntaxObservationSupport(row) => {
                sink.part(b"variant", &(45u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeObservationSupport(row) => {
                sink.part(b"variant", &(46u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypePresentationSupport(row) => {
                sink.part(b"variant", &(47u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeVariableRestrictionSupport(row) => {
                sink.part(b"variant", &(48u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FunctionBodyObservationSupport(row) => {
                sink.part(b"variant", &(49u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RecordFieldObservationSupport(row) => {
                sink.part(b"variant", &(50u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassMemberObservationSupport(row) => {
                sink.part(b"variant", &(57u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassMetadataObservationSupport(row) => {
                sink.part(b"variant", &(56u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureEnumerationObservationSupport(row) => {
                sink.part(b"variant", &(51u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffContextObservationSupport(row) => {
                sink.part(b"variant", &(52u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowNarrowingSupport(row) => {
                sink.part(b"variant", &(58u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeExitSupport(row) => {
                sink.part(b"variant", &(60u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeTerminalSupport(row) => {
                sink.part(b"variant", &(61u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CaptureSupport(row) => {
                sink.part(b"variant", &(63u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeExitDiagnosticSupport(row) => {
                sink.part(b"variant", &(62u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowSourceViewSupport(row) => {
                sink.part(b"variant", &(59u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeSignatureObservationSupport(row) => {
                sink.part(b"variant", &(53u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureTypeObservationSupport(row) => {
                sink.part(b"variant", &(54u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeQuerySupport(row) => {
                sink.part(b"variant", &(68u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::GenericSpecializationSupport(row) => {
                sink.part(b"variant", &(55u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowCaptureTimingSupport(row) => {
                sink.part(b"variant", &(65u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ExportEnumerationSupport(row) => {
                sink.part(b"variant", &(64u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffBindingObservationSupport(row) => {
                sink.part(b"variant", &(70u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffDiagnosticObservationSupport(row) => {
                sink.part(b"variant", &(66u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PyreflyDiagnosticObservationSupport(row) => {
                sink.part(b"variant", &(67u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeParameterDefinitionObservationSupport(row) => {
                sink.part(b"variant", &(69u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffDefinitionObservationSupport(row) => {
                sink.part(b"variant", &(71u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowUseInventorySupport(row) => {
                sink.part(b"variant", &(72u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeOverloadSupport(row) => {
                sink.part(b"variant", &(73u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeOverloadCandidateSupport(row) => {
                sink.part(b"variant", &(74u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModuleResolutionObservationSupport(row) => {
                sink.part(b"variant", &(75u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
        }
    }
}
impl SupportValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::UseSupport(row) => row.validate(),
            Self::DefinitionSupport(row) => row.validate(),
            Self::ReachingSupport(row) => row.validate(),
            Self::ValueSupport(row) => row.validate(),
            Self::RegionSupport(row) => row.validate(),
            Self::TestSupport(row) => row.validate(),
            Self::LeafSupport(row) => row.validate(),
            Self::SignatureSupport(row) => row.validate(),
            Self::CallTargetSupport(row) => row.validate(),
            Self::ProviderCallSiteSupport(row) => row.validate(),
            Self::CallSyntaxSupport(row) => row.validate(),
            Self::CallResolutionSupport(row) => row.validate(),
            Self::SymbolDeclarationSupport(row) => row.validate(),
            Self::ParameterDeclarationSupport(row) => row.validate(),
            Self::TaskReportObservationSupport(row) => row.validate(),
            Self::DeploymentObservationSupport(row) => row.validate(),
            Self::DocumentObservationSupport(row) => row.validate(),
            Self::PassageObservationSupport(row) => row.validate(),
            Self::CodeBlockObservationSupport(row) => row.validate(),
            Self::DocumentLinkObservationSupport(row) => row.validate(),
            Self::DocumentMentionObservationSupport(row) => row.validate(),
            Self::DocumentComponentObservationSupport(row) => row.validate(),
            Self::DocumentAttributeObservationSupport(row) => row.validate(),
            Self::FlowAttributeLoadObservationSupport(row) => row.validate(),
            Self::FlowValuePathObservationSupport(row) => row.validate(),
            Self::LexicalScopeObservationSupport(row) => row.validate(),
            Self::BindingObservationSupport(row) => row.validate(),
            Self::ReferenceObservationSupport(row) => row.validate(),
            Self::LexicalResolutionSupport(row) => row.validate(),
            Self::SyntaxObservationSupport(row) => row.validate(),
            Self::SymbolObservationSupport(row) => row.validate(),
            Self::FunctionTraitObservationSupport(row) => row.validate(),
            Self::ClassTraitObservationSupport(row) => row.validate(),
            Self::ClassAncestryObservationSupport(row) => row.validate(),
            Self::ParameterAnnotationObservationSupport(row) => row.validate(),
            Self::PublicNameObservationSupport(row) => row.validate(),
            Self::ParameterDocObservationSupport(row) => row.validate(),
            Self::SyntaxPlacementSupport(row) => row.validate(),
            Self::SyntaxDetailObservationSupport(row) => row.validate(),
            Self::DeclarationObservationSupport(row) => row.validate(),
            Self::DeclarationDecoratorSupport(row) => row.validate(),
            Self::ImportAliasObservationSupport(row) => row.validate(),
            Self::DunderAllObservationSupport(row) => row.validate(),
            Self::ParameterSyntaxObservationSupport(row) => row.validate(),
            Self::ClassFieldSyntaxObservationSupport(row) => row.validate(),
            Self::TypeObservationSupport(row) => row.validate(),
            Self::TypePresentationSupport(row) => row.validate(),
            Self::TypeVariableRestrictionSupport(row) => row.validate(),
            Self::FunctionBodyObservationSupport(row) => row.validate(),
            Self::RecordFieldObservationSupport(row) => row.validate(),
            Self::ClassMemberObservationSupport(row) => row.validate(),
            Self::ClassMetadataObservationSupport(row) => row.validate(),
            Self::SignatureEnumerationObservationSupport(row) => row.validate(),
            Self::RuffContextObservationSupport(row) => row.validate(),
            Self::FlowNarrowingSupport(row) => row.validate(),
            Self::NativeExitSupport(row) => row.validate(),
            Self::NativeTerminalSupport(row) => row.validate(),
            Self::CaptureSupport(row) => row.validate(),
            Self::NativeExitDiagnosticSupport(row) => row.validate(),
            Self::FlowSourceViewSupport(row) => row.validate(),
            Self::NativeSignatureObservationSupport(row) => row.validate(),
            Self::SignatureTypeObservationSupport(row) => row.validate(),
            Self::TypeQuerySupport(row) => row.validate(),
            Self::GenericSpecializationSupport(row) => row.validate(),
            Self::FlowCaptureTimingSupport(row) => row.validate(),
            Self::ExportEnumerationSupport(row) => row.validate(),
            Self::RuffBindingObservationSupport(row) => row.validate(),
            Self::RuffDiagnosticObservationSupport(row) => row.validate(),
            Self::PyreflyDiagnosticObservationSupport(row) => row.validate(),
            Self::NativeParameterDefinitionObservationSupport(row) => row.validate(),
            Self::RuffDefinitionObservationSupport(row) => row.validate(),
            Self::FlowUseInventorySupport(row) => row.validate(),
            Self::NativeOverloadSupport(row) => row.validate(),
            Self::NativeOverloadCandidateSupport(row) => row.validate(),
            Self::ModuleResolutionObservationSupport(row) => row.validate(),
        }
    }
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::UseSupport(row) => row.references(),
            Self::DefinitionSupport(row) => row.references(),
            Self::ReachingSupport(row) => row.references(),
            Self::ValueSupport(row) => row.references(),
            Self::RegionSupport(row) => row.references(),
            Self::TestSupport(row) => row.references(),
            Self::LeafSupport(row) => row.references(),
            Self::SignatureSupport(row) => row.references(),
            Self::CallTargetSupport(row) => row.references(),
            Self::ProviderCallSiteSupport(row) => row.references(),
            Self::CallSyntaxSupport(row) => row.references(),
            Self::CallResolutionSupport(row) => row.references(),
            Self::SymbolDeclarationSupport(row) => row.references(),
            Self::ParameterDeclarationSupport(row) => row.references(),
            Self::TaskReportObservationSupport(row) => row.references(),
            Self::DeploymentObservationSupport(row) => row.references(),
            Self::DocumentObservationSupport(row) => row.references(),
            Self::PassageObservationSupport(row) => row.references(),
            Self::CodeBlockObservationSupport(row) => row.references(),
            Self::DocumentLinkObservationSupport(row) => row.references(),
            Self::DocumentMentionObservationSupport(row) => row.references(),
            Self::DocumentComponentObservationSupport(row) => row.references(),
            Self::DocumentAttributeObservationSupport(row) => row.references(),
            Self::FlowAttributeLoadObservationSupport(row) => row.references(),
            Self::FlowValuePathObservationSupport(row) => row.references(),
            Self::LexicalScopeObservationSupport(row) => row.references(),
            Self::BindingObservationSupport(row) => row.references(),
            Self::ReferenceObservationSupport(row) => row.references(),
            Self::LexicalResolutionSupport(row) => row.references(),
            Self::SyntaxObservationSupport(row) => row.references(),
            Self::SymbolObservationSupport(row) => row.references(),
            Self::FunctionTraitObservationSupport(row) => row.references(),
            Self::ClassTraitObservationSupport(row) => row.references(),
            Self::ClassAncestryObservationSupport(row) => row.references(),
            Self::ParameterAnnotationObservationSupport(row) => row.references(),
            Self::PublicNameObservationSupport(row) => row.references(),
            Self::ParameterDocObservationSupport(row) => row.references(),
            Self::SyntaxPlacementSupport(row) => row.references(),
            Self::SyntaxDetailObservationSupport(row) => row.references(),
            Self::DeclarationObservationSupport(row) => row.references(),
            Self::DeclarationDecoratorSupport(row) => row.references(),
            Self::ImportAliasObservationSupport(row) => row.references(),
            Self::DunderAllObservationSupport(row) => row.references(),
            Self::ParameterSyntaxObservationSupport(row) => row.references(),
            Self::ClassFieldSyntaxObservationSupport(row) => row.references(),
            Self::TypeObservationSupport(row) => row.references(),
            Self::TypePresentationSupport(row) => row.references(),
            Self::TypeVariableRestrictionSupport(row) => row.references(),
            Self::FunctionBodyObservationSupport(row) => row.references(),
            Self::RecordFieldObservationSupport(row) => row.references(),
            Self::ClassMemberObservationSupport(row) => row.references(),
            Self::ClassMetadataObservationSupport(row) => row.references(),
            Self::SignatureEnumerationObservationSupport(row) => row.references(),
            Self::RuffContextObservationSupport(row) => row.references(),
            Self::FlowNarrowingSupport(row) => row.references(),
            Self::NativeExitSupport(row) => row.references(),
            Self::NativeTerminalSupport(row) => row.references(),
            Self::CaptureSupport(row) => row.references(),
            Self::NativeExitDiagnosticSupport(row) => row.references(),
            Self::FlowSourceViewSupport(row) => row.references(),
            Self::NativeSignatureObservationSupport(row) => row.references(),
            Self::SignatureTypeObservationSupport(row) => row.references(),
            Self::TypeQuerySupport(row) => row.references(),
            Self::GenericSpecializationSupport(row) => row.references(),
            Self::FlowCaptureTimingSupport(row) => row.references(),
            Self::ExportEnumerationSupport(row) => row.references(),
            Self::RuffBindingObservationSupport(row) => row.references(),
            Self::RuffDiagnosticObservationSupport(row) => row.references(),
            Self::PyreflyDiagnosticObservationSupport(row) => row.references(),
            Self::NativeParameterDefinitionObservationSupport(row) => row.references(),
            Self::RuffDefinitionObservationSupport(row) => row.references(),
            Self::FlowUseInventorySupport(row) => row.references(),
            Self::NativeOverloadSupport(row) => row.references(),
            Self::NativeOverloadCandidateSupport(row) => row.references(),
            Self::ModuleResolutionObservationSupport(row) => row.references(),
        }
    }
}

impl SupportValue {
    pub fn semantic_key(&self) -> super::SemanticKey {
        match self {
            Self::UseSupport(row) => super::SemanticKey::of(row.id()),
            Self::DefinitionSupport(row) => super::SemanticKey::of(row.id()),
            Self::ReachingSupport(row) => super::SemanticKey::of(row.id()),
            Self::ValueSupport(row) => super::SemanticKey::of(row.id()),
            Self::RegionSupport(row) => super::SemanticKey::of(row.id()),
            Self::TestSupport(row) => super::SemanticKey::of(row.id()),
            Self::LeafSupport(row) => super::SemanticKey::of(row.id()),
            Self::SignatureSupport(row) => super::SemanticKey::of(row.id()),
            Self::CallTargetSupport(row) => super::SemanticKey::of(row.id()),
            Self::ProviderCallSiteSupport(row) => super::SemanticKey::of(row.id()),
            Self::CallSyntaxSupport(row) => super::SemanticKey::of(row.id()),
            Self::CallResolutionSupport(row) => super::SemanticKey::of(row.id()),
            Self::SymbolDeclarationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ParameterDeclarationSupport(row) => super::SemanticKey::of(row.id()),
            Self::TaskReportObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DeploymentObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DocumentObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::PassageObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::CodeBlockObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DocumentLinkObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DocumentMentionObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DocumentComponentObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DocumentAttributeObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowAttributeLoadObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowValuePathObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::LexicalScopeObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::BindingObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ReferenceObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::LexicalResolutionSupport(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::SymbolObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FunctionTraitObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ClassTraitObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ClassAncestryObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ParameterAnnotationObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::PublicNameObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ParameterDocObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxPlacementSupport(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxDetailObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DeclarationObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DeclarationDecoratorSupport(row) => super::SemanticKey::of(row.id()),
            Self::ImportAliasObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::DunderAllObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ParameterSyntaxObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ClassFieldSyntaxObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::TypeObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::TypePresentationSupport(row) => super::SemanticKey::of(row.id()),
            Self::TypeVariableRestrictionSupport(row) => super::SemanticKey::of(row.id()),
            Self::FunctionBodyObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::RecordFieldObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ClassMemberObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::ClassMetadataObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::SignatureEnumerationObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::RuffContextObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowNarrowingSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeExitSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeTerminalSupport(row) => super::SemanticKey::of(row.id()),
            Self::CaptureSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeExitDiagnosticSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowSourceViewSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeSignatureObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::SignatureTypeObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::TypeQuerySupport(row) => super::SemanticKey::of(row.id()),
            Self::GenericSpecializationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowCaptureTimingSupport(row) => super::SemanticKey::of(row.id()),
            Self::ExportEnumerationSupport(row) => super::SemanticKey::of(row.id()),
            Self::RuffBindingObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::RuffDiagnosticObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::PyreflyDiagnosticObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeParameterDefinitionObservationSupport(row) => {
                super::SemanticKey::of(row.id())
            }
            Self::RuffDefinitionObservationSupport(row) => super::SemanticKey::of(row.id()),
            Self::FlowUseInventorySupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeOverloadSupport(row) => super::SemanticKey::of(row.id()),
            Self::NativeOverloadCandidateSupport(row) => super::SemanticKey::of(row.id()),
            Self::ModuleResolutionObservationSupport(row) => super::SemanticKey::of(row.id()),
        }
    }
}
