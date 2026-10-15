//! Pure facts inventory shared by cold admission and executable adapter binding.

use super::{
    ConfigurationBinding, ContextDependencyPolicy, OutputProfilePolicy, ProducerContract,
    SupplierRole,
};
use crate::domain::{
    ContentHash, artifact::ArtifactChunk, attribution::*, calls::*, class_metadata::*,
    declarations::*, deployment::*, diagnostics::*, documents::*, flow::*, flow_capture::*,
    flow_inventory::*, input::*, lexical::*, ruff::*, source::*, stages::*, symbols::*, syntax::*,
    types::*,
};

pub const ACQUIRE: &str = "acquire";
pub const PYREFLY: &str = "pyrefly";
pub const TY_FLOW: &str = "ty_flow";
pub const DOCUMENTS: &str = "documents";
pub const DEPLOYMENT: &str = "deployment";
pub const ASSEMBLE: &str = "assemble";
pub const PYREFLY_REVISION: &str = "63cda076956013cd0bd1d0d05c785f747fe6adc0;ruff=0.0.14";
pub const RUFF_REVISION: &str = "9080ee7a82a4ec7359ba2ade60839ef1108a2968";
pub const TY_REVISION: &str = "ty_python_core 0.0.16 (ruff 0.0.16, salsa 0.28.5)";
pub const FAMILIES: [FactFamily; 6] = [
    FactFamily::Syntax,
    FactFamily::Lexical,
    FactFamily::Exports,
    FactFamily::Signatures,
    FactFamily::Calls,
    FactFamily::Types,
];
fn supplier(
    name: &'static str,
    tool: &'static str,
    analyzer_revision: &'static str,
    families: Vec<FactFamily>,
) -> SupplierRole {
    SupplierRole {
        name,
        tool,
        analyzer_revision,
        semantic_revision: 1,
        families,
        configuration: ConfigurationBinding::CapturedAnalysisContext,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactsProducer {
    Acquisition,
    Pyrefly,
    TyFlow,
    Documents,
    Deployment,
    Assembly,
}
impl FactsProducer {
    pub const ALL: [Self; 6] = [
        Self::Acquisition,
        Self::Pyrefly,
        Self::TyFlow,
        Self::Documents,
        Self::Deployment,
        Self::Assembly,
    ];
    pub fn contract(self) -> ProducerContract {
        match self {
            Self::Acquisition => acquisition(),
            Self::Pyrefly => pyrefly(),
            Self::TyFlow => ty_flow(),
            Self::Documents => documents(),
            Self::Deployment => deployment(),
            Self::Assembly => assembly(),
        }
    }
}
pub fn facts() -> Vec<ProducerContract> {
    FactsProducer::ALL
        .into_iter()
        .map(FactsProducer::contract)
        .collect()
}

macro_rules! uses { ($($ty:ty),+ $(,)?) => { vec![$(RelationUse::of::<$ty>()),+] }; }

pub fn vocabulary() -> Vec<lctx_model::domain::stages::RelationUse> {
    use lctx_model::domain::stages::RelationUse;
    use lctx_model::domain::{
        assertion::*, assumptions::*, attribution::*, conditions::*, source::CoverageScope,
        syntax::*, value::*,
    };
    macro_rules! uses { ($($ty:ty),+) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    uses!(
        Evidence,
        Literal,
        LiteralSet,
        LiteralSetMember,
        SyntaxDetail,
        Assumption,
        AssumptionUniverse,
        AssumptionSet,
        AssumptionSetMember,
        AssertionQualification,
        Condition,
        ConditionNode,
        Provider,
        AnalysisContext,
        ProviderRun,
        RunFamily,
        ProviderSurface,
        CoverageScope,
        ProviderCoverage,
        crate::domain::uniform_outcome::UniformNotRequested,
        SubjectBoundary,
        AttachmentOutcome,
        AttachmentCandidate,
        PlaceRoot,
        PathSegment,
        AccessPath,
        Place,
        Predicate,
        EvaluationAtom
    )
}

fn acquisition_outputs() -> Vec<RelationUse> {
    vec![
        RelationUse::of::<Package>(),
        RelationUse::of::<Release>(),
        RelationUse::of::<InputRevision>(),
        RelationUse::of::<InputOrigin>(),
        RelationUse::of::<InputAcquisition>(),
        RelationUse::of::<CorpusLibrary>(),
        RelationUse::of::<InputDistribution>(),
        RelationUse::of::<DistributionVerification>(),
        RelationUse::of::<EnvironmentFingerprint>(),
        RelationUse::of::<SourceArtifact>(),
        RelationUse::of::<ArtifactChunk>(),
        RelationUse::of::<ArtifactOwnership>(),
        RelationUse::of::<UnownedArtifact>(),
        RelationUse::of::<DerivedArtifact>(),
        RelationUse::of::<ArtifactUse>(),
    ]
}

pub fn acquisition() -> ProducerContract {
    ProducerContract {
        name: ACQUIRE,
        inputs: vec![],
        outputs: acquisition_outputs(),
        contributes: vec![],
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        output_profile: OutputProfilePolicy::Neutral,
        context_dependency: ContextDependencyPolicy::AcquisitionOnly,
        effect: Effect::Acquisition,
        semantic_revision: 2,
        configuration: ConfigurationBinding::ProducerSettings,
        suppliers: vec![SupplierRole {
            configuration: ConfigurationBinding::CurrentAssemblyContext,
            ..supplier(
                "acquisition",
                "lctx-acquire",
                env!("CARGO_PKG_VERSION"),
                vec![FactFamily::Artifacts],
            )
        }],
        not_requested: vec![],
    }
}

fn pyrefly_outputs() -> Vec<RelationUse> {
    uses!(
        Module,
        Occurrence,
        RuffContextObservation,
        RuffContextSupport,
        RuffBindingObservation,
        RuffBindingSupport,
        RuffDefinitionObservation,
        RuffDefinitionSupport,
        RuffDiagnosticObservation,
        RuffDiagnosticSupport,
        PyreflyDiagnosticObservation,
        PyreflyDiagnosticSupport,
        DiagnosticSubject,
        DiagnosticAnnotation,
        NativeParameterDefinitionObservation,
        NativeParameterDefinitionSupport,
        SyntaxObservation,
        SyntaxSupport,
        SyntaxPlacement,
        SyntaxPlacementSupport,
        SyntaxDetailObservation,
        SyntaxDetailSupport,
        DeclarationObservation,
        DeclarationSupport,
        DeclarationDecorator,
        DeclarationDecoratorSupport,
        ImportAliasObservation,
        ImportAliasSupport,
        DunderAllObservation,
        DunderAllSupport,
        ParameterSyntaxObservation,
        ParameterSyntaxSupport,
        ClassFieldSyntaxObservation,
        ClassFieldSyntaxSupport,
        CallSyntax,
        CallSyntaxSupport,
        CallArgument,
        LexicalScope,
        BindingEvent,
        LexicalTarget,
        LexicalScopeObservation,
        LexicalScopeSupport,
        BindingObservation,
        BindingSupport,
        ReferenceObservation,
        ReferenceSupport,
        LexicalResolution,
        LexicalResolutionSupport,
        ProviderModule,
        ProviderSymbol,
        ParameterShape,
        Signature,
        SignatureSupport,
        SignatureParameter,
        SignatureEnumerationObservation,
        SignatureEnumerationMember,
        SignatureEnumerationSupport,
        SymbolDeclaration,
        SymbolDeclarationSupport,
        ParameterDeclaration,
        ParameterDeclarationSupport,
        SymbolSequence,
        SymbolSequenceMember,
        lctx_model::domain::captures::CaptureObservation,
        lctx_model::domain::captures::CaptureSupport,
        SymbolObservation,
        SymbolSupport,
        FunctionTraitObservation,
        FunctionTraitSupport,
        ClassTraitObservation,
        ClassTraitSupport,
        ClassAncestryObservation,
        ClassAncestrySupport,
        ParameterAnnotationObservation,
        ParameterAnnotationSupport,
        ExportOrigin,
        PublicNameObservation,
        PublicNameSupport,
        ParameterDocObservation,
        ParameterDocSupport,
        ModuleResolutionObservation,
        ModuleResolutionSupport,
        ProviderCallable,
        CallOrigin,
        CallOriginStep,
        CallDestination,
        CallChannel,
        Receiver,
        ProviderCallSite,
        ProviderCallSiteSupport,
        CallTarget,
        CallTargetSupport,
        CallResolution,
        CallResolutionSupport,
        CallResolutionMember,
        ClassMetadataObservation,
        ClassMetadataSupport,
        ClassMemberObservation,
        ClassMemberSupport,
        RecordOptions,
        RecordTransformDefaults,
        RecordTransformFieldSpecifier,
        TypeTerm,
        TypeSequence,
        TypeSequenceMember,
        CallableParameterList,
        CallableParameter,
        TypedDictFieldList,
        TypedDictField,
        TypeVariable,
        TypeVariableRestriction,
        TypeRestrictionSupport,
        NativeSignatureObservation,
        NativeSignatureSupport,
        NativeOverloadObservation,
        NativeOverloadSupport,
        NativeOverloadCandidate,
        NativeOverloadCandidateSupport,
        SignatureTypeSubject,
        SignatureTypeObservation,
        SignatureTypeSupport,
        GenericSpecializationObservation,
        GenericSpecializationSupport,
        TypeObservation,
        lctx_model::domain::types::TypeQueryObservation,
        lctx_model::domain::types::TypeQuerySupport,
        TypeSupport,
        TypePresentation,
        TypePresentationSupport,
        FunctionBodyObservation,
        FunctionBodySupport,
        RecordFieldObservation,
        RecordFieldSupport,
        lctx_model::domain::protocols::NativeExitObservation,
        lctx_model::domain::protocols::NativeExitSupport,
        lctx_model::domain::protocols::NativeTerminalObservation,
        lctx_model::domain::protocols::NativeTerminalSupport,
        lctx_model::domain::protocols::NativeExitDiagnostic,
        lctx_model::domain::protocols::NativeExitDiagnosticSupport,
        ExportEnumerationObservation,
        ExportEnumerationSupport
    )
}

pub fn pyrefly() -> ProducerContract {
    ProducerContract {
        name: PYREFLY,
        inputs: vec![],
        outputs: pyrefly_outputs(),
        contributes: vocabulary(),
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        output_profile: OutputProfilePolicy::Specific,
        context_dependency: ContextDependencyPolicy::ModelSelection,
        effect: Effect::Extraction,
        semantic_revision: 2,
        configuration: ConfigurationBinding::ProducerSettings,
        suppliers: vec![
            supplier(
                "pyrefly",
                "pyrefly",
                PYREFLY_REVISION,
                FAMILIES
                    .into_iter()
                    .filter(|family| *family != FactFamily::Syntax)
                    .collect(),
            ),
            supplier(
                "pyrefly-ruff",
                "ruff",
                RUFF_REVISION,
                vec![FactFamily::Syntax, FactFamily::Exports, FactFamily::Lexical],
            ),
        ],
        not_requested: vec![],
    }
}

macro_rules! output_types {
    ($apply:ident) => {
        $apply!(
            FlowUse,
            FlowDefinition,
            ReachingDefinition,
            FlowUseObservation,
            FlowUseSupport,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            FlowNarrowingObservation,
            FlowNarrowingSupport,
            FlowSourceViewObservation,
            FlowSourceViewSupport,
            FlowUseInventoryObservation,
            FlowUseInventorySupport,
            FlowUseCandidate,
            FlowUseInventoryMember,
            FlowCaptureTarget,
            FlowCaptureInventory,
            FlowCaptureCandidate,
            FlowCaptureTimingObservation,
            FlowCaptureTimingSupport,
            FlowValueObservation,
            FlowValueSupport,
            FlowRegionObservation,
            FlowRegionSupport,
            FlowTestObservation,
            FlowTestSupport,
            FlowTestLeafObservation,
            FlowTestLeafSupport,
            FlowAttributeLoadObservation,
            FlowAttributeLoadSupport,
            FlowCallPath,
            FlowCallStep,
            FlowValuePathObservation,
            FlowValuePathSupport
        )
    };
}

pub fn ty_flow() -> ProducerContract {
    macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
    ProducerContract {
        name: TY_FLOW,
        inputs: uses!(
            Module,
            Occurrence,
            LexicalScope,
            DeclarationObservation,
            BindingEvent,
            BindingObservation,
            LexicalTarget,
            LexicalResolution,
            ReferenceObservation,
            ImportAliasObservation,
            SyntaxObservation
        ),
        outputs: output_types!(uses),
        contributes: vocabulary(),
        profiles: vec![Profile::Behavioral],
        output_profile: OutputProfilePolicy::Specific,
        context_dependency: ContextDependencyPolicy::ModelSelection,
        effect: Effect::Extraction,
        semantic_revision: 2,
        configuration: ConfigurationBinding::ProducerSettings,
        suppliers: vec![supplier(
            "ty-flow",
            "ty",
            TY_REVISION,
            vec![FactFamily::Flow],
        )],
        not_requested: vec![FactFamily::Flow],
    }
}

macro_rules! document_types {
    ($apply:ident) => {
        $apply!(
            DocumentNode,
            DocumentAttributeValue,
            DocumentObservation,
            DocumentSupport,
            PassageObservation,
            PassageSupport,
            CodeBlockObservation,
            CodeBlockSupport,
            DocumentLinkObservation,
            DocumentLinkSupport,
            DocumentMentionObservation,
            DocumentMentionSupport,
            DocumentComponentObservation,
            DocumentComponentSupport,
            DocumentAttributeObservation,
            DocumentAttributeSupport
        )
    };
}

pub fn documents() -> ProducerContract {
    macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
    ProducerContract {
        name: DOCUMENTS,
        inputs: uses!(
            Module,
            Occurrence,
            SyntaxObservation,
            DeclarationObservation,
            PublicNameObservation,
            ExportOrigin,
            ProviderModule
        ),
        outputs: document_types!(uses),
        contributes: vocabulary(),
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        output_profile: OutputProfilePolicy::Neutral,
        context_dependency: ContextDependencyPolicy::SemanticContext,
        effect: Effect::Extraction,
        semantic_revision: 2,
        configuration: ConfigurationBinding::Fixed(ContentHash::of(
            b"mdx-frontmatter; source=64MiB; nodes=1000000; depth=256",
        )),
        suppliers: vec![supplier(
            "documents",
            "markdown-rs",
            "1.0.0",
            vec![FactFamily::Docs],
        )],
        not_requested: vec![],
    }
}

macro_rules! outputs {
    ($f:ident) => {
        $f!(
            DeploymentObservation,
            DeploymentSupport,
            ReportValue,
            ReportCollection,
            ReportEntry,
            ReportedEnvironment,
            TaskReport,
            TaskReportObservation,
            TaskReportSupport
        )
    };
}

pub fn deployment() -> ProducerContract {
    macro_rules! uses {($($ty:ty),+)=>{vec![$(RelationUse::of::<$ty>()),+]}}
    ProducerContract {
        name: DEPLOYMENT,
        inputs: uses!(
            SourceArtifact,
            ArtifactUse,
            EnvironmentFingerprint,
            Package,
            Release,
            InputAcquisition,
            CorpusLibrary
        ),
        outputs: outputs!(uses),
        contributes: vocabulary(),
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        output_profile: OutputProfilePolicy::Neutral,
        context_dependency: ContextDependencyPolicy::SemanticContext,
        effect: Effect::Extraction,
        semantic_revision: 2,
        configuration: ConfigurationBinding::Fixed(ContentHash::of(
            b"receipt-v1;64KiB;metadata-and-configuration=64MiB;report-only-runtime",
        )),
        suppliers: vec![supplier(
            "deployment",
            "lctx-deployment",
            env!("CARGO_PKG_VERSION"),
            vec![FactFamily::Deployment],
        )],
        not_requested: vec![],
    }
}

pub fn assembly() -> ProducerContract {
    ProducerContract {
        name: ASSEMBLE,
        inputs: vec![],
        outputs: vocabulary(),
        contributes: vec![],
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        output_profile: OutputProfilePolicy::Specific,
        context_dependency: ContextDependencyPolicy::SemanticContext,
        effect: Effect::Pure,
        semantic_revision: 2,
        configuration: ConfigurationBinding::Fixed(ContentHash::of(b"assemble")),
        suppliers: vec![],
        not_requested: vec![],
    }
}
