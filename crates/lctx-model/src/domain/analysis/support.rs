//! One qualified support contract; native assertions remain backed by their actual native supports.
use super::{invalid, AnalysisInvocation, obligations::{AnalysisChannel,ObligationSubject}};
use crate::domain::{assertion::{Assertion, AssertionQualification, Approximation, Support}, attribution::*, conditions::Diagram, flow::*, *};
use crate::{Domain, DomainCode, DomainSum};
pub use super::native::NativeAssertionPremise;
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


