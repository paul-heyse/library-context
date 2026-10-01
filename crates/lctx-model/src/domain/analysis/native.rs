//! Immutable native premise inventory over actual attributed assertions/supports.
use super::invalid;
use crate::domain::{assertion::{Assertion, AssertionQualification, Approximation, Support}, attribution::*, conditions::Diagram, flow::*, *};
use crate::{Domain, DomainSum};
use crate::domain::calls::{Signature,SignatureSupport};
use crate::domain::calls::{CallTarget,CallTargetSupport};
use crate::domain::calls::{ProviderCallSite,ProviderCallSiteSupport};
use crate::domain::calls::{CallSyntax,CallSyntaxSupport};
use crate::domain::calls::{CallResolution,CallResolutionSupport};
use crate::domain::declarations::{SymbolDeclaration,SymbolDeclarationSupport};
use crate::domain::declarations::{ParameterDeclaration,ParameterDeclarationSupport};
use crate::domain::deployment::{TaskReportObservation,TaskReportSupport};
use crate::domain::deployment::{DeploymentObservation,DeploymentSupport};
use crate::domain::documents::{DocumentObservation,DocumentSupport};
use crate::domain::documents::{PassageObservation,PassageSupport};
use crate::domain::documents::{CodeBlockObservation,CodeBlockSupport};
use crate::domain::documents::{DocumentLinkObservation,DocumentLinkSupport};
use crate::domain::documents::{DocumentMentionObservation,DocumentMentionSupport};
use crate::domain::documents::{DocumentComponentObservation,DocumentComponentSupport};
use crate::domain::documents::{DocumentAttributeObservation,DocumentAttributeSupport};
use crate::domain::flow::{FlowAttributeLoadObservation,FlowAttributeLoadSupport};
use crate::domain::flow::{FlowValuePathObservation,FlowValuePathSupport};
use crate::domain::lexical::{LexicalScopeObservation,LexicalScopeSupport};
use crate::domain::lexical::{BindingObservation,BindingSupport};
use crate::domain::lexical::{ReferenceObservation,ReferenceSupport};
use crate::domain::lexical::{LexicalResolution,LexicalResolutionSupport};
use crate::domain::source::{SyntaxObservation,SyntaxSupport};
use crate::domain::symbols::{SymbolObservation,SymbolSupport};
use crate::domain::symbols::{FunctionTraitObservation,FunctionTraitSupport};
use crate::domain::symbols::{ClassTraitObservation,ClassTraitSupport};
use crate::domain::symbols::{ClassAncestryObservation,ClassAncestrySupport};
use crate::domain::symbols::{ParameterAnnotationObservation,ParameterAnnotationSupport};
use crate::domain::symbols::{PublicNameObservation,PublicNameSupport};
use crate::domain::symbols::{ParameterDocObservation,ParameterDocSupport};
use crate::domain::symbols::{DependencyModuleObservation,DependencyModuleSupport};
use crate::domain::syntax::{SyntaxPlacement,SyntaxPlacementSupport};
use crate::domain::syntax::{SyntaxDetailObservation,SyntaxDetailSupport};
use crate::domain::syntax::{DeclarationObservation,DeclarationSupport};
use crate::domain::syntax::{DeclarationDecorator,DeclarationDecoratorSupport};
use crate::domain::syntax::{ImportAliasObservation,ImportAliasSupport};
use crate::domain::syntax::{DunderAllObservation,DunderAllSupport};
use crate::domain::syntax::{ParameterSyntaxObservation,ParameterSyntaxSupport};
use crate::domain::syntax::{ClassFieldSyntaxObservation,ClassFieldSyntaxSupport};
use crate::domain::types::{TypeObservation,TypeSupport};
use crate::domain::types::{TypePresentation,TypePresentationSupport};
use crate::domain::types::{TypeVariableRestriction,TypeRestrictionSupport};
use crate::domain::types::{FunctionBodyObservation,FunctionBodySupport};
use crate::domain::types::{RecordFieldObservation,RecordFieldSupport};
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "native_analysis_premises", rule = "native_analysis_premise")]
pub enum NativeAssertionPremise {
    #[model(code = 0)] Use { #[model(premise)] assertion: Id<FlowUseObservation>, #[model(premise)] support: Id<FlowUseSupport> },
    #[model(code = 1)] Definition { #[model(premise)] assertion: Id<FlowDefinitionObservation>, #[model(premise)] support: Id<FlowDefinitionSupport> },
    #[model(code = 2)] Reaching { #[model(premise)] assertion: Id<FlowReachingObservation>, #[model(premise)] support: Id<FlowReachingSupport> },
    #[model(code = 3)] Value { #[model(premise)] assertion: Id<FlowValueObservation>, #[model(premise)] support: Id<FlowValueSupport> },
    #[model(code = 4)] Region { #[model(premise)] assertion: Id<FlowRegionObservation>, #[model(premise)] support: Id<FlowRegionSupport> },
    #[model(code = 5)] Test { #[model(premise)] assertion: Id<FlowTestObservation>, #[model(premise)] support: Id<FlowTestSupport> },
    #[model(code = 6)] Leaf { #[model(premise)] assertion: Id<FlowTestLeafObservation>, #[model(premise)] support: Id<FlowTestLeafSupport> },
    #[model(code = 7)] Signature { #[model(premise)] assertion: Id<Signature>, #[model(premise)] support: Id<SignatureSupport> },
    #[model(code = 8)] CallTarget { #[model(premise)] assertion: Id<CallTarget>, #[model(premise)] support: Id<CallTargetSupport> },
    #[model(code = 9)] ProviderCallSite { #[model(premise)] assertion: Id<ProviderCallSite>, #[model(premise)] support: Id<ProviderCallSiteSupport> },
    #[model(code = 10)] CallSyntax { #[model(premise)] assertion: Id<CallSyntax>, #[model(premise)] support: Id<CallSyntaxSupport> },
    #[model(code = 11)] CallResolution { #[model(premise)] assertion: Id<CallResolution>, #[model(premise)] support: Id<CallResolutionSupport> },
    #[model(code = 12)] SymbolDeclaration { #[model(premise)] assertion: Id<SymbolDeclaration>, #[model(premise)] support: Id<SymbolDeclarationSupport> },
    #[model(code = 13)] ParameterDeclaration { #[model(premise)] assertion: Id<ParameterDeclaration>, #[model(premise)] support: Id<ParameterDeclarationSupport> },
    #[model(code = 14)] TaskReportObservation { #[model(premise)] assertion: Id<TaskReportObservation>, #[model(premise)] support: Id<TaskReportSupport> },
    #[model(code = 15)] DeploymentObservation { #[model(premise)] assertion: Id<DeploymentObservation>, #[model(premise)] support: Id<DeploymentSupport> },
    #[model(code = 16)] DocumentObservation { #[model(premise)] assertion: Id<DocumentObservation>, #[model(premise)] support: Id<DocumentSupport> },
    #[model(code = 17)] PassageObservation { #[model(premise)] assertion: Id<PassageObservation>, #[model(premise)] support: Id<PassageSupport> },
    #[model(code = 18)] CodeBlockObservation { #[model(premise)] assertion: Id<CodeBlockObservation>, #[model(premise)] support: Id<CodeBlockSupport> },
    #[model(code = 19)] DocumentLinkObservation { #[model(premise)] assertion: Id<DocumentLinkObservation>, #[model(premise)] support: Id<DocumentLinkSupport> },
    #[model(code = 20)] DocumentMentionObservation { #[model(premise)] assertion: Id<DocumentMentionObservation>, #[model(premise)] support: Id<DocumentMentionSupport> },
    #[model(code = 21)] DocumentComponentObservation { #[model(premise)] assertion: Id<DocumentComponentObservation>, #[model(premise)] support: Id<DocumentComponentSupport> },
    #[model(code = 22)] DocumentAttributeObservation { #[model(premise)] assertion: Id<DocumentAttributeObservation>, #[model(premise)] support: Id<DocumentAttributeSupport> },
    #[model(code = 23)] FlowAttributeLoadObservation { #[model(premise)] assertion: Id<FlowAttributeLoadObservation>, #[model(premise)] support: Id<FlowAttributeLoadSupport> },
    #[model(code = 24)] FlowValuePathObservation { #[model(premise)] assertion: Id<FlowValuePathObservation>, #[model(premise)] support: Id<FlowValuePathSupport> },
    #[model(code = 25)] LexicalScopeObservation { #[model(premise)] assertion: Id<LexicalScopeObservation>, #[model(premise)] support: Id<LexicalScopeSupport> },
    #[model(code = 26)] BindingObservation { #[model(premise)] assertion: Id<BindingObservation>, #[model(premise)] support: Id<BindingSupport> },
    #[model(code = 27)] ReferenceObservation { #[model(premise)] assertion: Id<ReferenceObservation>, #[model(premise)] support: Id<ReferenceSupport> },
    #[model(code = 28)] LexicalResolution { #[model(premise)] assertion: Id<LexicalResolution>, #[model(premise)] support: Id<LexicalResolutionSupport> },
    #[model(code = 29)] SyntaxObservation { #[model(premise)] assertion: Id<SyntaxObservation>, #[model(premise)] support: Id<SyntaxSupport> },
    #[model(code = 30)] SymbolObservation { #[model(premise)] assertion: Id<SymbolObservation>, #[model(premise)] support: Id<SymbolSupport> },
    #[model(code = 31)] FunctionTraitObservation { #[model(premise)] assertion: Id<FunctionTraitObservation>, #[model(premise)] support: Id<FunctionTraitSupport> },
    #[model(code = 32)] ClassTraitObservation { #[model(premise)] assertion: Id<ClassTraitObservation>, #[model(premise)] support: Id<ClassTraitSupport> },
    #[model(code = 33)] ClassAncestryObservation { #[model(premise)] assertion: Id<ClassAncestryObservation>, #[model(premise)] support: Id<ClassAncestrySupport> },
    #[model(code = 34)] ParameterAnnotationObservation { #[model(premise)] assertion: Id<ParameterAnnotationObservation>, #[model(premise)] support: Id<ParameterAnnotationSupport> },
    #[model(code = 35)] PublicNameObservation { #[model(premise)] assertion: Id<PublicNameObservation>, #[model(premise)] support: Id<PublicNameSupport> },
    #[model(code = 36)] ParameterDocObservation { #[model(premise)] assertion: Id<ParameterDocObservation>, #[model(premise)] support: Id<ParameterDocSupport> },
    #[model(code = 37)] DependencyModuleObservation { #[model(premise)] assertion: Id<DependencyModuleObservation>, #[model(premise)] support: Id<DependencyModuleSupport> },
    #[model(code = 38)] SyntaxPlacement { #[model(premise)] assertion: Id<SyntaxPlacement>, #[model(premise)] support: Id<SyntaxPlacementSupport> },
    #[model(code = 39)] SyntaxDetailObservation { #[model(premise)] assertion: Id<SyntaxDetailObservation>, #[model(premise)] support: Id<SyntaxDetailSupport> },
    #[model(code = 40)] DeclarationObservation { #[model(premise)] assertion: Id<DeclarationObservation>, #[model(premise)] support: Id<DeclarationSupport> },
    #[model(code = 41)] DeclarationDecorator { #[model(premise)] assertion: Id<DeclarationDecorator>, #[model(premise)] support: Id<DeclarationDecoratorSupport> },
    #[model(code = 42)] ImportAliasObservation { #[model(premise)] assertion: Id<ImportAliasObservation>, #[model(premise)] support: Id<ImportAliasSupport> },
    #[model(code = 43)] DunderAllObservation { #[model(premise)] assertion: Id<DunderAllObservation>, #[model(premise)] support: Id<DunderAllSupport> },
    #[model(code = 44)] ParameterSyntaxObservation { #[model(premise)] assertion: Id<ParameterSyntaxObservation>, #[model(premise)] support: Id<ParameterSyntaxSupport> },
    #[model(code = 45)] ClassFieldSyntaxObservation { #[model(premise)] assertion: Id<ClassFieldSyntaxObservation>, #[model(premise)] support: Id<ClassFieldSyntaxSupport> },
    #[model(code = 46)] TypeObservation { #[model(premise)] assertion: Id<TypeObservation>, #[model(premise)] support: Id<TypeSupport> },
    #[model(code = 47)] TypePresentation { #[model(premise)] assertion: Id<TypePresentation>, #[model(premise)] support: Id<TypePresentationSupport> },
    #[model(code = 48)] TypeVariableRestriction { #[model(premise)] assertion: Id<TypeVariableRestriction>, #[model(premise)] support: Id<TypeRestrictionSupport> },
    #[model(code = 49)] FunctionBodyObservation { #[model(premise)] assertion: Id<FunctionBodyObservation>, #[model(premise)] support: Id<FunctionBodySupport> },
    #[model(code = 50)] RecordFieldObservation { #[model(premise)] assertion: Id<RecordFieldObservation>, #[model(premise)] support: Id<RecordFieldSupport> },
}
impl NativeAssertionPremise {
    pub fn assertion_and_support(&self)->(derivation::RowRef,derivation::RowRef) {
        match self {
            Self::Use {assertion,support}=>(derivation::RowRef::of::<FlowUseObservation>(*assertion),derivation::RowRef::of::<FlowUseSupport>(*support)),
            Self::Definition {assertion,support}=>(derivation::RowRef::of::<FlowDefinitionObservation>(*assertion),derivation::RowRef::of::<FlowDefinitionSupport>(*support)),
            Self::Reaching {assertion,support}=>(derivation::RowRef::of::<FlowReachingObservation>(*assertion),derivation::RowRef::of::<FlowReachingSupport>(*support)),
            Self::Value {assertion,support}=>(derivation::RowRef::of::<FlowValueObservation>(*assertion),derivation::RowRef::of::<FlowValueSupport>(*support)),
            Self::Region {assertion,support}=>(derivation::RowRef::of::<FlowRegionObservation>(*assertion),derivation::RowRef::of::<FlowRegionSupport>(*support)),
            Self::Test {assertion,support}=>(derivation::RowRef::of::<FlowTestObservation>(*assertion),derivation::RowRef::of::<FlowTestSupport>(*support)),
            Self::Leaf {assertion,support}=>(derivation::RowRef::of::<FlowTestLeafObservation>(*assertion),derivation::RowRef::of::<FlowTestLeafSupport>(*support)),
            Self::Signature {assertion,support}=>(derivation::RowRef::of::<Signature>(*assertion),derivation::RowRef::of::<SignatureSupport>(*support)),
            Self::CallTarget {assertion,support}=>(derivation::RowRef::of::<CallTarget>(*assertion),derivation::RowRef::of::<CallTargetSupport>(*support)),
            Self::ProviderCallSite {assertion,support}=>(derivation::RowRef::of::<ProviderCallSite>(*assertion),derivation::RowRef::of::<ProviderCallSiteSupport>(*support)),
            Self::CallSyntax {assertion,support}=>(derivation::RowRef::of::<CallSyntax>(*assertion),derivation::RowRef::of::<CallSyntaxSupport>(*support)),
            Self::CallResolution {assertion,support}=>(derivation::RowRef::of::<CallResolution>(*assertion),derivation::RowRef::of::<CallResolutionSupport>(*support)),
            Self::SymbolDeclaration {assertion,support}=>(derivation::RowRef::of::<SymbolDeclaration>(*assertion),derivation::RowRef::of::<SymbolDeclarationSupport>(*support)),
            Self::ParameterDeclaration {assertion,support}=>(derivation::RowRef::of::<ParameterDeclaration>(*assertion),derivation::RowRef::of::<ParameterDeclarationSupport>(*support)),
            Self::TaskReportObservation {assertion,support}=>(derivation::RowRef::of::<TaskReportObservation>(*assertion),derivation::RowRef::of::<TaskReportSupport>(*support)),
            Self::DeploymentObservation {assertion,support}=>(derivation::RowRef::of::<DeploymentObservation>(*assertion),derivation::RowRef::of::<DeploymentSupport>(*support)),
            Self::DocumentObservation {assertion,support}=>(derivation::RowRef::of::<DocumentObservation>(*assertion),derivation::RowRef::of::<DocumentSupport>(*support)),
            Self::PassageObservation {assertion,support}=>(derivation::RowRef::of::<PassageObservation>(*assertion),derivation::RowRef::of::<PassageSupport>(*support)),
            Self::CodeBlockObservation {assertion,support}=>(derivation::RowRef::of::<CodeBlockObservation>(*assertion),derivation::RowRef::of::<CodeBlockSupport>(*support)),
            Self::DocumentLinkObservation {assertion,support}=>(derivation::RowRef::of::<DocumentLinkObservation>(*assertion),derivation::RowRef::of::<DocumentLinkSupport>(*support)),
            Self::DocumentMentionObservation {assertion,support}=>(derivation::RowRef::of::<DocumentMentionObservation>(*assertion),derivation::RowRef::of::<DocumentMentionSupport>(*support)),
            Self::DocumentComponentObservation {assertion,support}=>(derivation::RowRef::of::<DocumentComponentObservation>(*assertion),derivation::RowRef::of::<DocumentComponentSupport>(*support)),
            Self::DocumentAttributeObservation {assertion,support}=>(derivation::RowRef::of::<DocumentAttributeObservation>(*assertion),derivation::RowRef::of::<DocumentAttributeSupport>(*support)),
            Self::FlowAttributeLoadObservation {assertion,support}=>(derivation::RowRef::of::<FlowAttributeLoadObservation>(*assertion),derivation::RowRef::of::<FlowAttributeLoadSupport>(*support)),
            Self::FlowValuePathObservation {assertion,support}=>(derivation::RowRef::of::<FlowValuePathObservation>(*assertion),derivation::RowRef::of::<FlowValuePathSupport>(*support)),
            Self::LexicalScopeObservation {assertion,support}=>(derivation::RowRef::of::<LexicalScopeObservation>(*assertion),derivation::RowRef::of::<LexicalScopeSupport>(*support)),
            Self::BindingObservation {assertion,support}=>(derivation::RowRef::of::<BindingObservation>(*assertion),derivation::RowRef::of::<BindingSupport>(*support)),
            Self::ReferenceObservation {assertion,support}=>(derivation::RowRef::of::<ReferenceObservation>(*assertion),derivation::RowRef::of::<ReferenceSupport>(*support)),
            Self::LexicalResolution {assertion,support}=>(derivation::RowRef::of::<LexicalResolution>(*assertion),derivation::RowRef::of::<LexicalResolutionSupport>(*support)),
            Self::SyntaxObservation {assertion,support}=>(derivation::RowRef::of::<SyntaxObservation>(*assertion),derivation::RowRef::of::<SyntaxSupport>(*support)),
            Self::SymbolObservation {assertion,support}=>(derivation::RowRef::of::<SymbolObservation>(*assertion),derivation::RowRef::of::<SymbolSupport>(*support)),
            Self::FunctionTraitObservation {assertion,support}=>(derivation::RowRef::of::<FunctionTraitObservation>(*assertion),derivation::RowRef::of::<FunctionTraitSupport>(*support)),
            Self::ClassTraitObservation {assertion,support}=>(derivation::RowRef::of::<ClassTraitObservation>(*assertion),derivation::RowRef::of::<ClassTraitSupport>(*support)),
            Self::ClassAncestryObservation {assertion,support}=>(derivation::RowRef::of::<ClassAncestryObservation>(*assertion),derivation::RowRef::of::<ClassAncestrySupport>(*support)),
            Self::ParameterAnnotationObservation {assertion,support}=>(derivation::RowRef::of::<ParameterAnnotationObservation>(*assertion),derivation::RowRef::of::<ParameterAnnotationSupport>(*support)),
            Self::PublicNameObservation {assertion,support}=>(derivation::RowRef::of::<PublicNameObservation>(*assertion),derivation::RowRef::of::<PublicNameSupport>(*support)),
            Self::ParameterDocObservation {assertion,support}=>(derivation::RowRef::of::<ParameterDocObservation>(*assertion),derivation::RowRef::of::<ParameterDocSupport>(*support)),
            Self::DependencyModuleObservation {assertion,support}=>(derivation::RowRef::of::<DependencyModuleObservation>(*assertion),derivation::RowRef::of::<DependencyModuleSupport>(*support)),
            Self::SyntaxPlacement {assertion,support}=>(derivation::RowRef::of::<SyntaxPlacement>(*assertion),derivation::RowRef::of::<SyntaxPlacementSupport>(*support)),
            Self::SyntaxDetailObservation {assertion,support}=>(derivation::RowRef::of::<SyntaxDetailObservation>(*assertion),derivation::RowRef::of::<SyntaxDetailSupport>(*support)),
            Self::DeclarationObservation {assertion,support}=>(derivation::RowRef::of::<DeclarationObservation>(*assertion),derivation::RowRef::of::<DeclarationSupport>(*support)),
            Self::DeclarationDecorator {assertion,support}=>(derivation::RowRef::of::<DeclarationDecorator>(*assertion),derivation::RowRef::of::<DeclarationDecoratorSupport>(*support)),
            Self::ImportAliasObservation {assertion,support}=>(derivation::RowRef::of::<ImportAliasObservation>(*assertion),derivation::RowRef::of::<ImportAliasSupport>(*support)),
            Self::DunderAllObservation {assertion,support}=>(derivation::RowRef::of::<DunderAllObservation>(*assertion),derivation::RowRef::of::<DunderAllSupport>(*support)),
            Self::ParameterSyntaxObservation {assertion,support}=>(derivation::RowRef::of::<ParameterSyntaxObservation>(*assertion),derivation::RowRef::of::<ParameterSyntaxSupport>(*support)),
            Self::ClassFieldSyntaxObservation {assertion,support}=>(derivation::RowRef::of::<ClassFieldSyntaxObservation>(*assertion),derivation::RowRef::of::<ClassFieldSyntaxSupport>(*support)),
            Self::TypeObservation {assertion,support}=>(derivation::RowRef::of::<TypeObservation>(*assertion),derivation::RowRef::of::<TypeSupport>(*support)),
            Self::TypePresentation {assertion,support}=>(derivation::RowRef::of::<TypePresentation>(*assertion),derivation::RowRef::of::<TypePresentationSupport>(*support)),
            Self::TypeVariableRestriction {assertion,support}=>(derivation::RowRef::of::<TypeVariableRestriction>(*assertion),derivation::RowRef::of::<TypeRestrictionSupport>(*support)),
            Self::FunctionBodyObservation {assertion,support}=>(derivation::RowRef::of::<FunctionBodyObservation>(*assertion),derivation::RowRef::of::<FunctionBodySupport>(*support)),
            Self::RecordFieldObservation {assertion,support}=>(derivation::RowRef::of::<RecordFieldObservation>(*assertion),derivation::RowRef::of::<RecordFieldSupport>(*support)),
        }
    }
}

impl NativeAssertionPremise {
    pub fn family(&self)->FactFamily {match self {
        Self::Use {..}=><FlowUseObservation as Assertion>::FAMILY,
        Self::Definition {..}=><FlowDefinitionObservation as Assertion>::FAMILY,
        Self::Reaching {..}=><FlowReachingObservation as Assertion>::FAMILY,
        Self::Value {..}=><FlowValueObservation as Assertion>::FAMILY,
        Self::Region {..}=><FlowRegionObservation as Assertion>::FAMILY,
        Self::Test {..}=><FlowTestObservation as Assertion>::FAMILY,
        Self::Leaf {..}=><FlowTestLeafObservation as Assertion>::FAMILY,
        Self::Signature {..}=><Signature as Assertion>::FAMILY,
        Self::CallTarget {..}=><CallTarget as Assertion>::FAMILY,
        Self::ProviderCallSite {..}=><ProviderCallSite as Assertion>::FAMILY,
        Self::CallSyntax {..}=><CallSyntax as Assertion>::FAMILY,
        Self::CallResolution {..}=><CallResolution as Assertion>::FAMILY,
        Self::SymbolDeclaration {..}=><SymbolDeclaration as Assertion>::FAMILY,
        Self::ParameterDeclaration {..}=><ParameterDeclaration as Assertion>::FAMILY,
        Self::TaskReportObservation {..}=><TaskReportObservation as Assertion>::FAMILY,
        Self::DeploymentObservation {..}=><DeploymentObservation as Assertion>::FAMILY,
        Self::DocumentObservation {..}=><DocumentObservation as Assertion>::FAMILY,
        Self::PassageObservation {..}=><PassageObservation as Assertion>::FAMILY,
        Self::CodeBlockObservation {..}=><CodeBlockObservation as Assertion>::FAMILY,
        Self::DocumentLinkObservation {..}=><DocumentLinkObservation as Assertion>::FAMILY,
        Self::DocumentMentionObservation {..}=><DocumentMentionObservation as Assertion>::FAMILY,
        Self::DocumentComponentObservation {..}=><DocumentComponentObservation as Assertion>::FAMILY,
        Self::DocumentAttributeObservation {..}=><DocumentAttributeObservation as Assertion>::FAMILY,
        Self::FlowAttributeLoadObservation {..}=><FlowAttributeLoadObservation as Assertion>::FAMILY,
        Self::FlowValuePathObservation {..}=><FlowValuePathObservation as Assertion>::FAMILY,
        Self::LexicalScopeObservation {..}=><LexicalScopeObservation as Assertion>::FAMILY,
        Self::BindingObservation {..}=><BindingObservation as Assertion>::FAMILY,
        Self::ReferenceObservation {..}=><ReferenceObservation as Assertion>::FAMILY,
        Self::LexicalResolution {..}=><LexicalResolution as Assertion>::FAMILY,
        Self::SyntaxObservation {..}=><SyntaxObservation as Assertion>::FAMILY,
        Self::SymbolObservation {..}=><SymbolObservation as Assertion>::FAMILY,
        Self::FunctionTraitObservation {..}=><FunctionTraitObservation as Assertion>::FAMILY,
        Self::ClassTraitObservation {..}=><ClassTraitObservation as Assertion>::FAMILY,
        Self::ClassAncestryObservation {..}=><ClassAncestryObservation as Assertion>::FAMILY,
        Self::ParameterAnnotationObservation {..}=><ParameterAnnotationObservation as Assertion>::FAMILY,
        Self::PublicNameObservation {..}=><PublicNameObservation as Assertion>::FAMILY,
        Self::ParameterDocObservation {..}=><ParameterDocObservation as Assertion>::FAMILY,
        Self::DependencyModuleObservation {..}=><DependencyModuleObservation as Assertion>::FAMILY,
        Self::SyntaxPlacement {..}=><SyntaxPlacement as Assertion>::FAMILY,
        Self::SyntaxDetailObservation {..}=><SyntaxDetailObservation as Assertion>::FAMILY,
        Self::DeclarationObservation {..}=><DeclarationObservation as Assertion>::FAMILY,
        Self::DeclarationDecorator {..}=><DeclarationDecorator as Assertion>::FAMILY,
        Self::ImportAliasObservation {..}=><ImportAliasObservation as Assertion>::FAMILY,
        Self::DunderAllObservation {..}=><DunderAllObservation as Assertion>::FAMILY,
        Self::ParameterSyntaxObservation {..}=><ParameterSyntaxObservation as Assertion>::FAMILY,
        Self::ClassFieldSyntaxObservation {..}=><ClassFieldSyntaxObservation as Assertion>::FAMILY,
        Self::TypeObservation {..}=><TypeObservation as Assertion>::FAMILY,
        Self::TypePresentation {..}=><TypePresentation as Assertion>::FAMILY,
        Self::TypeVariableRestriction {..}=><TypeVariableRestriction as Assertion>::FAMILY,
        Self::FunctionBodyObservation {..}=><FunctionBodyObservation as Assertion>::FAMILY,
        Self::RecordFieldObservation {..}=><RecordFieldObservation as Assertion>::FAMILY,
    }}
}

/// One-shot typed projection of an actual native pair. All payload is recomputed from the
/// paired assertion/support at inventory closure; it never selects a new attribution or claim.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="native_qualifications",rule="native_qualification")]
pub struct NativeQualification {
    #[model(key,premise)] pub premise:Id<NativeAssertionPremise>,
    pub qualification:Id<AssertionQualification>,
    pub family:FactFamily,
    pub fidelity:Fidelity,
    pub status:super::policy::EvidenceStatus,
}
pub fn relations()->Vec<Relation> {vec![Relation::of::<NativeAssertionPremise>(),Relation::of::<NativeQualification>()]}
