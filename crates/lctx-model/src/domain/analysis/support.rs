//! One qualified support contract; native assertions remain backed by their actual native supports.
use super::{invalid, AnalysisInvocation, obligations::{AnalysisChannel,ObligationSubject}};
use crate::domain::{assertion::{Assertion, AssertionQualification, Approximation, Support}, attribution::*, conditions::Diagram, flow::*, *};
use crate::{Domain, DomainCode, DomainSum};
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "analysis_support_sources", rule = "analysis_support_source")]
pub enum SupportSource {
    #[model(code = 0)] NativeAssertion { #[model(premise)] premise: Id<NativeAssertionPremise> },
    #[model(code = 1)] AnalysisDerivation { #[model(premise)] derivation: Id<AnalysisDerivation> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum QualificationOperation { Conjunction = 0, AlternativeUnion = 1 }
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_propositions")]
pub struct AnalysisProposition {
    #[model(key)] pub subject: Id<ObligationSubject>,
    #[model(key)] pub channel: AnalysisChannel,
    #[model(key)] pub phase: calls::CallPhase,
    #[model(key)] pub qualification: Id<AssertionQualification>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_derivations", invariants = support_invariants)]
pub struct AnalysisDerivation {
    #[model(key)] pub invocation: Id<AnalysisInvocation>,
    #[model(key)] pub proposition: Id<AnalysisProposition>,
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub operation: QualificationOperation,
    #[model(key)] pub inputs: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_derivation_premises", rule = "qualified_analysis_derivation", conclusion = derivation)]
pub struct AnalysisDerivationPremise {
    #[model(key)] pub derivation: Id<AnalysisDerivation>,
    #[model(key, premise)] pub source: Id<SupportSource>,
}
/// Canonical input to the shared qualification operation. Construction checks the BDD identity;
/// nominal source linkage is checked by the stored invariant, rather than accepted as a boolean.
#[derive(Debug,Clone,Copy)]
pub struct QualifiedPremise<'a> {
    pub source: &'a SupportSource,
    pub qualification: &'a AssertionQualification,
    pub condition: &'a Diagram,
}
pub struct QualifiedResult {
    pub qualification: AssertionQualification,
    pub condition: Diagram,
}
/// Both native and derived premises use this operation. Conditions use the authoritative BDD;
/// modalities/approximations use their shared weakest/join operations. No desired result status
/// is accepted. The source DAG and exact membership are validated separately.
pub fn qualify(operation: QualificationOperation, premises: &[QualifiedPremise<'_>]) -> Result<QualifiedResult,ModelError> {
    let first = premises.first().ok_or_else(|| invalid("qualified derivation needs evidence"))?;
    let mut condition = first.condition.clone();
    let mut modality = Modality::Definite;
    let mut approximation = Approximation::Exact;
    for (index,premise) in premises.iter().enumerate() {
        let q = premise.qualification;
        if (q.context,q.scope) != (first.qualification.context,first.qualification.scope) || q.condition != premise.condition.id() {
            return Err(invalid("qualification premise crosses frame or changes condition"));
        }
        modality = modality.weakest(q.modality);
        approximation = approximation.join(q.approximation);
        if index != 0 {
            condition = match operation { QualificationOperation::Conjunction => condition.and(premise.condition),QualificationOperation::AlternativeUnion => condition.or(premise.condition) }
                .map_err(|boundary| invalid(&format!("qualification boundary: {:?}",obligation::from_kernel(boundary))))?;
        }
    }
    Ok(QualifiedResult { qualification: AssertionQualification { context: first.qualification.context,scope: first.qualification.scope,condition: condition.id(),modality,approximation },condition })
}
pub(super) fn input_digest(sources: &std::collections::BTreeSet<Id<SupportSource>>) -> ContentHash {
    let mut sink = KeySink::new("analysis-derivation-premises");
    for source in sources { source.encode(&mut sink); }
    sink.finish()
}
impl AnalysisDerivation {
    pub fn emit(invocation: Id<AnalysisInvocation>,subject:Id<ObligationSubject>,channel:AnalysisChannel,phase:calls::CallPhase,operation: QualificationOperation,premises: &[QualifiedPremise<'_>]) -> Result<(Self,AnalysisProposition,Vec<AnalysisDerivationPremise>,QualifiedResult),ModelError> {
        let sources = premises.iter().map(|p|p.source.id()).collect::<std::collections::BTreeSet<_>>();
        if sources.len() != premises.len() { return Err(invalid("duplicate qualified derivation source")); }
        let result = qualify(operation,premises)?;
        let proposition=AnalysisProposition {subject,channel,phase,qualification:result.qualification.id()};
        let row = Self { invocation,proposition:proposition.id(),qualification: result.qualification.id(),operation,inputs: input_digest(&sources) };
        let members = sources.into_iter().map(|source|AnalysisDerivationPremise { derivation: row.id(),source }).collect();
        Ok((row,proposition,members,result))
    }
}

fn support_invariants() -> Vec<Invariant> {
    let mut inputs = vec![ValidationInput::of::<AnalysisProposition>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<conditions::ConditionNode>(&["id"]),ValidationInput::of::<conditions::Condition>(&["id"])];
    macro_rules! native_inputs { ($($a:ty => $s:ty),*) => { $( inputs.extend([ValidationInput::of::<$a>(&["id"]),ValidationInput::of::<$s>(&["id"])]); )* }; }
    native_inputs!(FlowUseObservation=>FlowUseSupport,FlowDefinitionObservation=>FlowDefinitionSupport,FlowReachingObservation=>FlowReachingSupport,FlowValueObservation=>FlowValueSupport,FlowRegionObservation=>FlowRegionSupport,FlowTestObservation=>FlowTestSupport,FlowTestLeafObservation=>FlowTestLeafSupport,Signature=>SignatureSupport,CallTarget=>CallTargetSupport,ProviderCallSite=>ProviderCallSiteSupport,CallSyntax=>CallSyntaxSupport,CallResolution=>CallResolutionSupport,SymbolDeclaration=>SymbolDeclarationSupport,ParameterDeclaration=>ParameterDeclarationSupport,TaskReportObservation=>TaskReportSupport,DeploymentObservation=>DeploymentSupport,DocumentObservation=>DocumentSupport,PassageObservation=>PassageSupport,CodeBlockObservation=>CodeBlockSupport,DocumentLinkObservation=>DocumentLinkSupport,DocumentMentionObservation=>DocumentMentionSupport,DocumentComponentObservation=>DocumentComponentSupport,DocumentAttributeObservation=>DocumentAttributeSupport,FlowAttributeLoadObservation=>FlowAttributeLoadSupport,FlowValuePathObservation=>FlowValuePathSupport,LexicalScopeObservation=>LexicalScopeSupport,BindingObservation=>BindingSupport,ReferenceObservation=>ReferenceSupport,LexicalResolution=>LexicalResolutionSupport,SyntaxObservation=>SyntaxSupport,SymbolObservation=>SymbolSupport,FunctionTraitObservation=>FunctionTraitSupport,ClassTraitObservation=>ClassTraitSupport,ClassAncestryObservation=>ClassAncestrySupport,ParameterAnnotationObservation=>ParameterAnnotationSupport,PublicNameObservation=>PublicNameSupport,ParameterDocObservation=>ParameterDocSupport,DependencyModuleObservation=>DependencyModuleSupport,SyntaxPlacement=>SyntaxPlacementSupport,SyntaxDetailObservation=>SyntaxDetailSupport,DeclarationObservation=>DeclarationSupport,DeclarationDecorator=>DeclarationDecoratorSupport,ImportAliasObservation=>ImportAliasSupport,DunderAllObservation=>DunderAllSupport,ParameterSyntaxObservation=>ParameterSyntaxSupport,ClassFieldSyntaxObservation=>ClassFieldSyntaxSupport,TypeObservation=>TypeSupport,TypePresentation=>TypePresentationSupport,TypeVariableRestriction=>TypeRestrictionSupport,FunctionBodyObservation=>FunctionBodySupport,RecordFieldObservation=>RecordFieldSupport);
    inputs.extend([ValidationInput::of::<NativeAssertionPremise>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<SupportSource>(&["id"]),ValidationInput::of::<AnalysisDerivationPremise>(&["id"])]);
    vec![Invariant { name: "analysis_qualified_derivation",inputs,create: std::sync::Arc::new(|budget| Box::new(SupportCheck::new(budget))) }]
}
struct SupportCheck {
    charge: charged::StateCharge,
    propositions: charged::ChargedMap<Id<AnalysisProposition>,AnalysisProposition>,
    invocations: charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,
    qualifications: charged::ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    conditions: charged::ChargedMap<Id<conditions::Condition>,conditions::Condition>,
    nodes: charged::ChargedMap<Id<conditions::ConditionNode>,conditions::ConditionNode>,
    native: charged::ChargedMap<derivation::RowRef,Id<AssertionQualification>>,
    native_support: charged::ChargedMap<derivation::RowRef,derivation::RowRef>,
    native_premises: charged::ChargedMap<Id<NativeAssertionPremise>,NativeAssertionPremise>,
    sources: charged::ChargedMap<Id<SupportSource>,SupportSource>,
    derivations: charged::ChargedMap<Id<AnalysisDerivation>,AnalysisDerivation>,
    members: charged::ChargedMap<Id<AnalysisDerivation>,std::collections::BTreeSet<Id<SupportSource>>>,
}
impl SupportCheck {
    fn new(budget: &resources::ResourceBudget) -> Self { Self { charge: charged::StateCharge::new(budget,"analysis_qualified_derivation"),propositions:Default::default(),invocations:Default::default(),qualifications:Default::default(),conditions:Default::default(),nodes:Default::default(),native:Default::default(),native_support:Default::default(),native_premises:Default::default(),sources:Default::default(),derivations:Default::default(),members:Default::default() } }
    fn source_qualification(&self,source: &SupportSource) -> Result<Id<AssertionQualification>,ModelError> {
        match source {
            SupportSource::AnalysisDerivation { derivation } => self.derivations.get(derivation).map(|row|row.qualification).ok_or_else(||invalid("analysis source derivation absent")),
            SupportSource::NativeAssertion { premise } => {
                let native = self.native_premises.get(premise).ok_or_else(||invalid("native nominal premise absent"))?;
                let (assertion,support)=native.assertion_and_support();
                if self.native_support.get(&support) != Some(&assertion) { return Err(invalid("native premise support names another assertion")); }
                self.native.get(&assertion).copied().ok_or_else(||invalid("native premise assertion absent"))
            }
        }
    }
}
impl InvariantCheck for SupportCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        macro_rules! insert { ($r:ty,$field:ident) => { if relation == <$r>::NAME { for row in <$r>::decode(batch)? { if self.$field.insert(&mut self.charge,row.id(),row)?.is_some() { return Err(ModelError::Conflict(<$r>::NAME)); } } return Ok(()); } }; }
        insert!(AnalysisProposition,propositions);insert!(AnalysisInvocation,invocations);insert!(AssertionQualification,qualifications);insert!(conditions::Condition,conditions);insert!(conditions::ConditionNode,nodes);insert!(NativeAssertionPremise,native_premises);insert!(SupportSource,sources);insert!(AnalysisDerivation,derivations);
        macro_rules! visit_native { ($($a:ty => $s:ty),*) => { $(
            if relation == <$a>::NAME { for row in <$a>::decode(batch)? { self.native.insert(&mut self.charge,derivation::RowRef::of(row.id()),row.qualification())?; } return Ok(()); }
            if relation == <$s>::NAME { for row in <$s>::decode(batch)? { self.native_support.insert(&mut self.charge,derivation::RowRef::of(row.id()),derivation::RowRef::of(row.assertion()))?; } return Ok(()); }
        )* }; }
        visit_native!(FlowUseObservation=>FlowUseSupport,FlowDefinitionObservation=>FlowDefinitionSupport,FlowReachingObservation=>FlowReachingSupport,FlowValueObservation=>FlowValueSupport,FlowRegionObservation=>FlowRegionSupport,FlowTestObservation=>FlowTestSupport,FlowTestLeafObservation=>FlowTestLeafSupport,Signature=>SignatureSupport,CallTarget=>CallTargetSupport,ProviderCallSite=>ProviderCallSiteSupport,CallSyntax=>CallSyntaxSupport,CallResolution=>CallResolutionSupport,SymbolDeclaration=>SymbolDeclarationSupport,ParameterDeclaration=>ParameterDeclarationSupport,TaskReportObservation=>TaskReportSupport,DeploymentObservation=>DeploymentSupport,DocumentObservation=>DocumentSupport,PassageObservation=>PassageSupport,CodeBlockObservation=>CodeBlockSupport,DocumentLinkObservation=>DocumentLinkSupport,DocumentMentionObservation=>DocumentMentionSupport,DocumentComponentObservation=>DocumentComponentSupport,DocumentAttributeObservation=>DocumentAttributeSupport,FlowAttributeLoadObservation=>FlowAttributeLoadSupport,FlowValuePathObservation=>FlowValuePathSupport,LexicalScopeObservation=>LexicalScopeSupport,BindingObservation=>BindingSupport,ReferenceObservation=>ReferenceSupport,LexicalResolution=>LexicalResolutionSupport,SyntaxObservation=>SyntaxSupport,SymbolObservation=>SymbolSupport,FunctionTraitObservation=>FunctionTraitSupport,ClassTraitObservation=>ClassTraitSupport,ClassAncestryObservation=>ClassAncestrySupport,ParameterAnnotationObservation=>ParameterAnnotationSupport,PublicNameObservation=>PublicNameSupport,ParameterDocObservation=>ParameterDocSupport,DependencyModuleObservation=>DependencyModuleSupport,SyntaxPlacement=>SyntaxPlacementSupport,SyntaxDetailObservation=>SyntaxDetailSupport,DeclarationObservation=>DeclarationSupport,DeclarationDecorator=>DeclarationDecoratorSupport,ImportAliasObservation=>ImportAliasSupport,DunderAllObservation=>DunderAllSupport,ParameterSyntaxObservation=>ParameterSyntaxSupport,ClassFieldSyntaxObservation=>ClassFieldSyntaxSupport,TypeObservation=>TypeSupport,TypePresentation=>TypePresentationSupport,TypeVariableRestriction=>TypeRestrictionSupport,FunctionBodyObservation=>FunctionBodySupport,RecordFieldObservation=>RecordFieldSupport);
        if relation == AnalysisDerivationPremise::NAME {
            for row in AnalysisDerivationPremise::decode(batch)? { if !self.members.update(&mut self.charge,row.derivation,|members|members.insert(row.source))? { return Err(invalid("duplicate derivation premise")); } }
            return Ok(());
        }
        Err(invalid("undeclared qualified derivation input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        for (id,row) in self.derivations.iter() {
            let invocation=self.invocations.get(&row.invocation).ok_or_else(||invalid("derived invocation absent"))?;
            let q=self.qualifications.get(&row.qualification).ok_or_else(||invalid("derived qualification absent"))?;
            if self.propositions.get(&row.proposition).ok_or_else(||invalid("derived proposition absent"))?.qualification!=q.id() { return Err(invalid("derived proposition changes qualification")); }
            if q.context != invocation.context { return Err(invalid("derived qualification crosses invocation context")); }
            let members=self.members.get(id).ok_or_else(||invalid("derived evidence membership missing"))?;
            if input_digest(members)!=row.inputs { return Err(invalid("derived evidence membership digest differs")); }
            let node_bytes = self.nodes.values().try_fold(0usize,|bytes,node| bytes.checked_add(size_of::<conditions::ConditionNode>() + node.heap_bytes() + 128).ok_or_else(||invalid("qualification node allocation overflow")))?;
            let _reservation=self.charge.budget().ok_or_else(||invalid("qualification budget absent"))?.reserve("analysis_qualified_derivation",node_bytes.checked_mul(members.len()+1).ok_or_else(||invalid("qualification allocation overflow"))?)?;
            let nodes = self.nodes.values().cloned().collect::<Vec<_>>();
            let mut conditions=Vec::new();
            let mut qualifications=Vec::new();
            let mut sources=Vec::new();
            for member in members {
                let source=self.sources.get(member).ok_or_else(||invalid("derived support source absent"))?;
                let source_q=self.qualifications.get(&self.source_qualification(source)?).ok_or_else(||invalid("source qualification absent"))?;
                let condition=self.conditions.get(&source_q.condition).ok_or_else(||invalid("source condition absent"))?;
                conditions.push(Diagram::from_records(condition,&nodes)?);
                qualifications.push(source_q);
                sources.push(source);
            }
            let premises=sources.iter().zip(&qualifications).zip(&conditions).map(|((source,qualification),condition)|QualifiedPremise { source,qualification,condition }).collect::<Vec<_>>();
            if qualify(row.operation,&premises)?.qualification != *q { return Err(invalid("derived qualification strengthens or changes its evidence")); }
        }
        for (id,_) in self.members.iter() { if !self.derivations.contains_key(id) { return Err(invalid("orphan derivation membership")); } }
        for (_,source) in self.sources.iter() { self.source_qualification(source)?; }
        Ok(())
    }
}
pub fn relations()->Vec<Relation> { vec![Relation::of::<AnalysisProposition>(),Relation::of::<NativeAssertionPremise>(),Relation::of::<SupportSource>(),Relation::of::<AnalysisDerivation>(),Relation::of::<AnalysisDerivationPremise>()] }

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
