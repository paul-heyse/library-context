use super::*;

/// Selected typed semantic payloads. Participant references are lowered into graph roles by the
/// compiler; these native/result values retain their owner's exact data and codebook meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NativeValue {
    Use(crate::domain::flow::FlowUseObservation),
    Definition(crate::domain::flow::FlowDefinitionObservation),
    Reaching(crate::domain::flow::FlowReachingObservation),
    Value(crate::domain::flow::FlowValueObservation),
    Region(crate::domain::flow::FlowRegionObservation),
    Test(crate::domain::flow::FlowTestObservation),
    Leaf(crate::domain::flow::FlowTestLeafObservation),
    Signature(crate::domain::calls::Signature),
    CallTarget(crate::domain::calls::CallTarget),
    ProviderCallSite(crate::domain::calls::ProviderCallSite),
    CallSyntax(crate::domain::calls::CallSyntax),
    CallResolution(crate::domain::calls::CallResolution),
    SymbolDeclaration(crate::domain::declarations::SymbolDeclaration),
    ParameterDeclaration(crate::domain::declarations::ParameterDeclaration),
    TaskReportObservation(crate::domain::deployment::TaskReportObservation),
    DeploymentObservation(crate::domain::deployment::DeploymentObservation),
    DocumentObservation(crate::domain::documents::DocumentObservation),
    PassageObservation(crate::domain::documents::PassageObservation),
    CodeBlockObservation(crate::domain::documents::CodeBlockObservation),
    DocumentLinkObservation(crate::domain::documents::DocumentLinkObservation),
    DocumentMentionObservation(crate::domain::documents::DocumentMentionObservation),
    DocumentComponentObservation(crate::domain::documents::DocumentComponentObservation),
    DocumentAttributeObservation(crate::domain::documents::DocumentAttributeObservation),
    FlowAttributeLoadObservation(crate::domain::flow::FlowAttributeLoadObservation),
    FlowValuePathObservation(crate::domain::flow::FlowValuePathObservation),
    LexicalScopeObservation(crate::domain::lexical::LexicalScopeObservation),
    BindingObservation(crate::domain::lexical::BindingObservation),
    ReferenceObservation(crate::domain::lexical::ReferenceObservation),
    LexicalResolution(crate::domain::lexical::LexicalResolution),
    SyntaxObservation(crate::domain::source::SyntaxObservation),
    SymbolObservation(crate::domain::symbols::SymbolObservation),
    FunctionTraitObservation(crate::domain::symbols::FunctionTraitObservation),
    ClassTraitObservation(crate::domain::symbols::ClassTraitObservation),
    ClassAncestryObservation(crate::domain::symbols::ClassAncestryObservation),
    ParameterAnnotationObservation(crate::domain::symbols::ParameterAnnotationObservation),
    PublicNameObservation(crate::domain::symbols::PublicNameObservation),
    ParameterDocObservation(crate::domain::symbols::ParameterDocObservation),
    SyntaxPlacement(crate::domain::syntax::SyntaxPlacement),
    SyntaxDetailObservation(crate::domain::syntax::SyntaxDetailObservation),
    DeclarationObservation(crate::domain::syntax::DeclarationObservation),
    DeclarationDecorator(crate::domain::syntax::DeclarationDecorator),
    ImportAliasObservation(crate::domain::syntax::ImportAliasObservation),
    DunderAllObservation(crate::domain::syntax::DunderAllObservation),
    ParameterSyntaxObservation(crate::domain::syntax::ParameterSyntaxObservation),
    ClassFieldSyntaxObservation(crate::domain::syntax::ClassFieldSyntaxObservation),
    TypeObservation(crate::domain::types::TypeObservation),
    TypePresentation(crate::domain::types::TypePresentation),
    TypeVariableRestriction(crate::domain::types::TypeVariableRestriction),
    FunctionBodyObservation(crate::domain::types::FunctionBodyObservation),
    RecordFieldObservation(crate::domain::types::RecordFieldObservation),
    ClassMemberObservation(crate::domain::class_metadata::ClassMemberObservation),
    ClassMetadataObservation(crate::domain::class_metadata::ClassMetadataObservation),
    SignatureEnumerationObservation(crate::domain::calls::SignatureEnumerationObservation),
    RuffContextObservation(crate::domain::ruff::RuffContextObservation),
    FlowNarrowing(crate::domain::flow::FlowNarrowingObservation),
    NativeExit(crate::domain::protocols::NativeExitObservation),
    NativeTerminal(crate::domain::protocols::NativeTerminalObservation),
    Capture(crate::domain::captures::CaptureObservation),
    NativeExitDiagnostic(crate::domain::protocols::NativeExitDiagnostic),
    FlowSourceView(crate::domain::flow::FlowSourceViewObservation),
    NativeSignatureObservation(crate::domain::types::NativeSignatureObservation),
    SignatureTypeObservation(crate::domain::types::SignatureTypeObservation),
    TypeQuery(crate::domain::types::TypeQueryObservation),
    GenericSpecialization(crate::domain::types::GenericSpecializationObservation),
    FlowCaptureTiming(crate::domain::flow_capture::FlowCaptureTimingObservation),
    ExportEnumeration(crate::domain::symbols::ExportEnumerationObservation),
    RuffBindingObservation(crate::domain::ruff::RuffBindingObservation),
    RuffDiagnosticObservation(crate::domain::diagnostics::RuffDiagnosticObservation),
    PyreflyDiagnosticObservation(crate::domain::diagnostics::PyreflyDiagnosticObservation),
    NativeParameterDefinitionObservation(
        crate::domain::diagnostics::NativeParameterDefinitionObservation,
    ),
    RuffDefinitionObservation(crate::domain::ruff::RuffDefinitionObservation),
    FlowUseInventory(crate::domain::flow_inventory::FlowUseInventoryObservation),
    NativeOverload(crate::domain::types::NativeOverloadObservation),
    NativeOverloadCandidate(crate::domain::types::NativeOverloadCandidate),
    ModuleResolutionObservation(crate::domain::symbols::ModuleResolutionObservation),
}
impl Key for NativeValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::Use(row) => {
                sink.part(b"variant", &(0u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Definition(row) => {
                sink.part(b"variant", &(1u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Reaching(row) => {
                sink.part(b"variant", &(2u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Value(row) => {
                sink.part(b"variant", &(3u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Region(row) => {
                sink.part(b"variant", &(4u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Test(row) => {
                sink.part(b"variant", &(5u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Leaf(row) => {
                sink.part(b"variant", &(6u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Signature(row) => {
                sink.part(b"variant", &(7u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallTarget(row) => {
                sink.part(b"variant", &(8u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ProviderCallSite(row) => {
                sink.part(b"variant", &(9u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallSyntax(row) => {
                sink.part(b"variant", &(10u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CallResolution(row) => {
                sink.part(b"variant", &(11u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SymbolDeclaration(row) => {
                sink.part(b"variant", &(12u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterDeclaration(row) => {
                sink.part(b"variant", &(13u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TaskReportObservation(row) => {
                sink.part(b"variant", &(14u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeploymentObservation(row) => {
                sink.part(b"variant", &(15u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentObservation(row) => {
                sink.part(b"variant", &(16u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PassageObservation(row) => {
                sink.part(b"variant", &(17u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CodeBlockObservation(row) => {
                sink.part(b"variant", &(18u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentLinkObservation(row) => {
                sink.part(b"variant", &(19u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentMentionObservation(row) => {
                sink.part(b"variant", &(20u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentComponentObservation(row) => {
                sink.part(b"variant", &(21u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentAttributeObservation(row) => {
                sink.part(b"variant", &(22u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowAttributeLoadObservation(row) => {
                sink.part(b"variant", &(23u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowValuePathObservation(row) => {
                sink.part(b"variant", &(24u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LexicalScopeObservation(row) => {
                sink.part(b"variant", &(25u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BindingObservation(row) => {
                sink.part(b"variant", &(26u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ReferenceObservation(row) => {
                sink.part(b"variant", &(27u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LexicalResolution(row) => {
                sink.part(b"variant", &(28u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxObservation(row) => {
                sink.part(b"variant", &(29u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SymbolObservation(row) => {
                sink.part(b"variant", &(30u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FunctionTraitObservation(row) => {
                sink.part(b"variant", &(31u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassTraitObservation(row) => {
                sink.part(b"variant", &(32u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassAncestryObservation(row) => {
                sink.part(b"variant", &(33u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterAnnotationObservation(row) => {
                sink.part(b"variant", &(34u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PublicNameObservation(row) => {
                sink.part(b"variant", &(35u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterDocObservation(row) => {
                sink.part(b"variant", &(36u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxPlacement(row) => {
                sink.part(b"variant", &(38u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SyntaxDetailObservation(row) => {
                sink.part(b"variant", &(39u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeclarationObservation(row) => {
                sink.part(b"variant", &(40u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DeclarationDecorator(row) => {
                sink.part(b"variant", &(41u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ImportAliasObservation(row) => {
                sink.part(b"variant", &(42u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DunderAllObservation(row) => {
                sink.part(b"variant", &(43u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ParameterSyntaxObservation(row) => {
                sink.part(b"variant", &(44u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassFieldSyntaxObservation(row) => {
                sink.part(b"variant", &(45u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeObservation(row) => {
                sink.part(b"variant", &(46u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypePresentation(row) => {
                sink.part(b"variant", &(47u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeVariableRestriction(row) => {
                sink.part(b"variant", &(48u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FunctionBodyObservation(row) => {
                sink.part(b"variant", &(49u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RecordFieldObservation(row) => {
                sink.part(b"variant", &(50u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassMemberObservation(row) => {
                sink.part(b"variant", &(57u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ClassMetadataObservation(row) => {
                sink.part(b"variant", &(56u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureEnumerationObservation(row) => {
                sink.part(b"variant", &(51u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffContextObservation(row) => {
                sink.part(b"variant", &(52u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowNarrowing(row) => {
                sink.part(b"variant", &(58u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeExit(row) => {
                sink.part(b"variant", &(60u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeTerminal(row) => {
                sink.part(b"variant", &(61u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Capture(row) => {
                sink.part(b"variant", &(63u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeExitDiagnostic(row) => {
                sink.part(b"variant", &(62u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowSourceView(row) => {
                sink.part(b"variant", &(59u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeSignatureObservation(row) => {
                sink.part(b"variant", &(53u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureTypeObservation(row) => {
                sink.part(b"variant", &(54u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeQuery(row) => {
                sink.part(b"variant", &(68u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::GenericSpecialization(row) => {
                sink.part(b"variant", &(55u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowCaptureTiming(row) => {
                sink.part(b"variant", &(65u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ExportEnumeration(row) => {
                sink.part(b"variant", &(64u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffBindingObservation(row) => {
                sink.part(b"variant", &(70u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffDiagnosticObservation(row) => {
                sink.part(b"variant", &(66u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PyreflyDiagnosticObservation(row) => {
                sink.part(b"variant", &(67u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeParameterDefinitionObservation(row) => {
                sink.part(b"variant", &(69u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RuffDefinitionObservation(row) => {
                sink.part(b"variant", &(71u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::FlowUseInventory(row) => {
                sink.part(b"variant", &(72u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeOverload(row) => {
                sink.part(b"variant", &(73u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeOverloadCandidate(row) => {
                sink.part(b"variant", &(74u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModuleResolutionObservation(row) => {
                sink.part(b"variant", &(75u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
        }
    }
}
impl NativeValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::Use(row) => row.validate(),
            Self::Definition(row) => row.validate(),
            Self::Reaching(row) => row.validate(),
            Self::Value(row) => row.validate(),
            Self::Region(row) => row.validate(),
            Self::Test(row) => row.validate(),
            Self::Leaf(row) => row.validate(),
            Self::Signature(row) => row.validate(),
            Self::CallTarget(row) => row.validate(),
            Self::ProviderCallSite(row) => row.validate(),
            Self::CallSyntax(row) => row.validate(),
            Self::CallResolution(row) => row.validate(),
            Self::SymbolDeclaration(row) => row.validate(),
            Self::ParameterDeclaration(row) => row.validate(),
            Self::TaskReportObservation(row) => row.validate(),
            Self::DeploymentObservation(row) => row.validate(),
            Self::DocumentObservation(row) => row.validate(),
            Self::PassageObservation(row) => row.validate(),
            Self::CodeBlockObservation(row) => row.validate(),
            Self::DocumentLinkObservation(row) => row.validate(),
            Self::DocumentMentionObservation(row) => row.validate(),
            Self::DocumentComponentObservation(row) => row.validate(),
            Self::DocumentAttributeObservation(row) => row.validate(),
            Self::FlowAttributeLoadObservation(row) => row.validate(),
            Self::FlowValuePathObservation(row) => row.validate(),
            Self::LexicalScopeObservation(row) => row.validate(),
            Self::BindingObservation(row) => row.validate(),
            Self::ReferenceObservation(row) => row.validate(),
            Self::LexicalResolution(row) => row.validate(),
            Self::SyntaxObservation(row) => row.validate(),
            Self::SymbolObservation(row) => row.validate(),
            Self::FunctionTraitObservation(row) => row.validate(),
            Self::ClassTraitObservation(row) => row.validate(),
            Self::ClassAncestryObservation(row) => row.validate(),
            Self::ParameterAnnotationObservation(row) => row.validate(),
            Self::PublicNameObservation(row) => row.validate(),
            Self::ParameterDocObservation(row) => row.validate(),
            Self::SyntaxPlacement(row) => row.validate(),
            Self::SyntaxDetailObservation(row) => row.validate(),
            Self::DeclarationObservation(row) => row.validate(),
            Self::DeclarationDecorator(row) => row.validate(),
            Self::ImportAliasObservation(row) => row.validate(),
            Self::DunderAllObservation(row) => row.validate(),
            Self::ParameterSyntaxObservation(row) => row.validate(),
            Self::ClassFieldSyntaxObservation(row) => row.validate(),
            Self::TypeObservation(row) => row.validate(),
            Self::TypePresentation(row) => row.validate(),
            Self::TypeVariableRestriction(row) => row.validate(),
            Self::FunctionBodyObservation(row) => row.validate(),
            Self::RecordFieldObservation(row) => row.validate(),
            Self::ClassMemberObservation(row) => row.validate(),
            Self::ClassMetadataObservation(row) => row.validate(),
            Self::SignatureEnumerationObservation(row) => row.validate(),
            Self::RuffContextObservation(row) => row.validate(),
            Self::FlowNarrowing(row) => row.validate(),
            Self::NativeExit(row) => row.validate(),
            Self::NativeTerminal(row) => row.validate(),
            Self::Capture(row) => row.validate(),
            Self::NativeExitDiagnostic(row) => row.validate(),
            Self::FlowSourceView(row) => row.validate(),
            Self::NativeSignatureObservation(row) => row.validate(),
            Self::SignatureTypeObservation(row) => row.validate(),
            Self::TypeQuery(row) => row.validate(),
            Self::GenericSpecialization(row) => row.validate(),
            Self::FlowCaptureTiming(row) => row.validate(),
            Self::ExportEnumeration(row) => row.validate(),
            Self::RuffBindingObservation(row) => row.validate(),
            Self::RuffDiagnosticObservation(row) => row.validate(),
            Self::PyreflyDiagnosticObservation(row) => row.validate(),
            Self::NativeParameterDefinitionObservation(row) => row.validate(),
            Self::RuffDefinitionObservation(row) => row.validate(),
            Self::FlowUseInventory(row) => row.validate(),
            Self::NativeOverload(row) => row.validate(),
            Self::NativeOverloadCandidate(row) => row.validate(),
            Self::ModuleResolutionObservation(row) => row.validate(),
        }
    }
}

impl NativeValue {
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::Use(row) => row.references(),
            Self::Definition(row) => row.references(),
            Self::Reaching(row) => row.references(),
            Self::Value(row) => row.references(),
            Self::Region(row) => row.references(),
            Self::Test(row) => row.references(),
            Self::Leaf(row) => row.references(),
            Self::Signature(row) => row.references(),
            Self::CallTarget(row) => row.references(),
            Self::ProviderCallSite(row) => row.references(),
            Self::CallSyntax(row) => row.references(),
            Self::CallResolution(row) => row.references(),
            Self::SymbolDeclaration(row) => row.references(),
            Self::ParameterDeclaration(row) => row.references(),
            Self::TaskReportObservation(row) => row.references(),
            Self::DeploymentObservation(row) => row.references(),
            Self::DocumentObservation(row) => row.references(),
            Self::PassageObservation(row) => row.references(),
            Self::CodeBlockObservation(row) => row.references(),
            Self::DocumentLinkObservation(row) => row.references(),
            Self::DocumentMentionObservation(row) => row.references(),
            Self::DocumentComponentObservation(row) => row.references(),
            Self::DocumentAttributeObservation(row) => row.references(),
            Self::FlowAttributeLoadObservation(row) => row.references(),
            Self::FlowValuePathObservation(row) => row.references(),
            Self::LexicalScopeObservation(row) => row.references(),
            Self::BindingObservation(row) => row.references(),
            Self::ReferenceObservation(row) => row.references(),
            Self::LexicalResolution(row) => row.references(),
            Self::SyntaxObservation(row) => row.references(),
            Self::SymbolObservation(row) => row.references(),
            Self::FunctionTraitObservation(row) => row.references(),
            Self::ClassTraitObservation(row) => row.references(),
            Self::ClassAncestryObservation(row) => row.references(),
            Self::ParameterAnnotationObservation(row) => row.references(),
            Self::PublicNameObservation(row) => row.references(),
            Self::ParameterDocObservation(row) => row.references(),
            Self::SyntaxPlacement(row) => row.references(),
            Self::SyntaxDetailObservation(row) => row.references(),
            Self::DeclarationObservation(row) => row.references(),
            Self::DeclarationDecorator(row) => row.references(),
            Self::ImportAliasObservation(row) => row.references(),
            Self::DunderAllObservation(row) => row.references(),
            Self::ParameterSyntaxObservation(row) => row.references(),
            Self::ClassFieldSyntaxObservation(row) => row.references(),
            Self::TypeObservation(row) => row.references(),
            Self::TypePresentation(row) => row.references(),
            Self::TypeVariableRestriction(row) => row.references(),
            Self::FunctionBodyObservation(row) => row.references(),
            Self::RecordFieldObservation(row) => row.references(),
            Self::ClassMemberObservation(row) => row.references(),
            Self::ClassMetadataObservation(row) => row.references(),
            Self::SignatureEnumerationObservation(row) => row.references(),
            Self::RuffContextObservation(row) => row.references(),
            Self::FlowNarrowing(row) => row.references(),
            Self::NativeExit(row) => row.references(),
            Self::NativeTerminal(row) => row.references(),
            Self::Capture(row) => row.references(),
            Self::NativeExitDiagnostic(row) => row.references(),
            Self::FlowSourceView(row) => row.references(),
            Self::NativeSignatureObservation(row) => row.references(),
            Self::SignatureTypeObservation(row) => row.references(),
            Self::TypeQuery(row) => row.references(),
            Self::GenericSpecialization(row) => row.references(),
            Self::FlowCaptureTiming(row) => row.references(),
            Self::ExportEnumeration(row) => row.references(),
            Self::RuffBindingObservation(row) => row.references(),
            Self::RuffDiagnosticObservation(row) => row.references(),
            Self::PyreflyDiagnosticObservation(row) => row.references(),
            Self::NativeParameterDefinitionObservation(row) => row.references(),
            Self::RuffDefinitionObservation(row) => row.references(),
            Self::FlowUseInventory(row) => row.references(),
            Self::NativeOverload(row) => row.references(),
            Self::NativeOverloadCandidate(row) => row.references(),
            Self::ModuleResolutionObservation(row) => row.references(),
        }
    }
}

impl NativeValue {
    pub fn semantic_key(&self) -> super::SemanticKey {
        match self {
            Self::Use(row) => super::SemanticKey::of(row.id()),
            Self::Definition(row) => super::SemanticKey::of(row.id()),
            Self::Reaching(row) => super::SemanticKey::of(row.id()),
            Self::Value(row) => super::SemanticKey::of(row.id()),
            Self::Region(row) => super::SemanticKey::of(row.id()),
            Self::Test(row) => super::SemanticKey::of(row.id()),
            Self::Leaf(row) => super::SemanticKey::of(row.id()),
            Self::Signature(row) => super::SemanticKey::of(row.id()),
            Self::CallTarget(row) => super::SemanticKey::of(row.id()),
            Self::ProviderCallSite(row) => super::SemanticKey::of(row.id()),
            Self::CallSyntax(row) => super::SemanticKey::of(row.id()),
            Self::CallResolution(row) => super::SemanticKey::of(row.id()),
            Self::SymbolDeclaration(row) => super::SemanticKey::of(row.id()),
            Self::ParameterDeclaration(row) => super::SemanticKey::of(row.id()),
            Self::TaskReportObservation(row) => super::SemanticKey::of(row.id()),
            Self::DeploymentObservation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentObservation(row) => super::SemanticKey::of(row.id()),
            Self::PassageObservation(row) => super::SemanticKey::of(row.id()),
            Self::CodeBlockObservation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentLinkObservation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentMentionObservation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentComponentObservation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentAttributeObservation(row) => super::SemanticKey::of(row.id()),
            Self::FlowAttributeLoadObservation(row) => super::SemanticKey::of(row.id()),
            Self::FlowValuePathObservation(row) => super::SemanticKey::of(row.id()),
            Self::LexicalScopeObservation(row) => super::SemanticKey::of(row.id()),
            Self::BindingObservation(row) => super::SemanticKey::of(row.id()),
            Self::ReferenceObservation(row) => super::SemanticKey::of(row.id()),
            Self::LexicalResolution(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxObservation(row) => super::SemanticKey::of(row.id()),
            Self::SymbolObservation(row) => super::SemanticKey::of(row.id()),
            Self::FunctionTraitObservation(row) => super::SemanticKey::of(row.id()),
            Self::ClassTraitObservation(row) => super::SemanticKey::of(row.id()),
            Self::ClassAncestryObservation(row) => super::SemanticKey::of(row.id()),
            Self::ParameterAnnotationObservation(row) => super::SemanticKey::of(row.id()),
            Self::PublicNameObservation(row) => super::SemanticKey::of(row.id()),
            Self::ParameterDocObservation(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxPlacement(row) => super::SemanticKey::of(row.id()),
            Self::SyntaxDetailObservation(row) => super::SemanticKey::of(row.id()),
            Self::DeclarationObservation(row) => super::SemanticKey::of(row.id()),
            Self::DeclarationDecorator(row) => super::SemanticKey::of(row.id()),
            Self::ImportAliasObservation(row) => super::SemanticKey::of(row.id()),
            Self::DunderAllObservation(row) => super::SemanticKey::of(row.id()),
            Self::ParameterSyntaxObservation(row) => super::SemanticKey::of(row.id()),
            Self::ClassFieldSyntaxObservation(row) => super::SemanticKey::of(row.id()),
            Self::TypeObservation(row) => super::SemanticKey::of(row.id()),
            Self::TypePresentation(row) => super::SemanticKey::of(row.id()),
            Self::TypeVariableRestriction(row) => super::SemanticKey::of(row.id()),
            Self::FunctionBodyObservation(row) => super::SemanticKey::of(row.id()),
            Self::RecordFieldObservation(row) => super::SemanticKey::of(row.id()),
            Self::ClassMemberObservation(row) => super::SemanticKey::of(row.id()),
            Self::ClassMetadataObservation(row) => super::SemanticKey::of(row.id()),
            Self::SignatureEnumerationObservation(row) => super::SemanticKey::of(row.id()),
            Self::RuffContextObservation(row) => super::SemanticKey::of(row.id()),
            Self::FlowNarrowing(row) => super::SemanticKey::of(row.id()),
            Self::NativeExit(row) => super::SemanticKey::of(row.id()),
            Self::NativeTerminal(row) => super::SemanticKey::of(row.id()),
            Self::Capture(row) => super::SemanticKey::of(row.id()),
            Self::NativeExitDiagnostic(row) => super::SemanticKey::of(row.id()),
            Self::FlowSourceView(row) => super::SemanticKey::of(row.id()),
            Self::NativeSignatureObservation(row) => super::SemanticKey::of(row.id()),
            Self::SignatureTypeObservation(row) => super::SemanticKey::of(row.id()),
            Self::TypeQuery(row) => super::SemanticKey::of(row.id()),
            Self::GenericSpecialization(row) => super::SemanticKey::of(row.id()),
            Self::FlowCaptureTiming(row) => super::SemanticKey::of(row.id()),
            Self::ExportEnumeration(row) => super::SemanticKey::of(row.id()),
            Self::RuffBindingObservation(row) => super::SemanticKey::of(row.id()),
            Self::RuffDiagnosticObservation(row) => super::SemanticKey::of(row.id()),
            Self::PyreflyDiagnosticObservation(row) => super::SemanticKey::of(row.id()),
            Self::NativeParameterDefinitionObservation(row) => super::SemanticKey::of(row.id()),
            Self::RuffDefinitionObservation(row) => super::SemanticKey::of(row.id()),
            Self::FlowUseInventory(row) => super::SemanticKey::of(row.id()),
            Self::NativeOverload(row) => super::SemanticKey::of(row.id()),
            Self::NativeOverloadCandidate(row) => super::SemanticKey::of(row.id()),
            Self::ModuleResolutionObservation(row) => super::SemanticKey::of(row.id()),
        }
    }
}
