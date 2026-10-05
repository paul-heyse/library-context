//! Semantic graph contracts. The finite graph vocabulary selects semantic entities and
//! assertions; it is not the internal Arrow relation registry or a published execution ledger.
use super::{ContentHash,Key,KeySink,ModelError,Record,Id};
use serde::{Deserialize,Serialize};
mod nativevalue;
mod analysisvalue;
mod supportvalue;
pub use nativevalue::NativeValue;
pub use analysisvalue::AnalysisValue;
pub use supportvalue::SupportValue;
macro_rules! nominal_id {($name:ident)=>{
    #[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,Serialize,Deserialize)]
    pub struct $name(pub ContentHash);
    impl Key for $name {fn encode(&self,sink:&mut KeySink){self.0.encode(sink);}}
};}
nominal_id!(EntityId);
nominal_id!(AssertionId);
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
#[repr(u16)]
pub enum EntityKind {Release=0,Capture=1,Source=2,Module=3,Occurrence=4,Provider=5,Context=6,Run=7,Declaration=8,Exposure=9,InvocationVariant=10,Scope=11,Parameter=12,Field=13,NativeSymbol=14,Type=15,Literal=16,Place=17,Predicate=18,EvaluationAtom=19,Condition=20,AssumptionUniverse=21,Assumption=22,CatalogOption=23,Scenario=24,RetrievalText=25,RetrievalUnit=26,AnalysisDefinition=27,EmbeddingSpecification=28,Package=29,AcquisitionOrigin=30,NativeModule=31,NativeCallable=32,ParameterShape=33,TypeVariable=34,TypeSequence=35,CallableParameterList=36,CallableParameter=37,TypedDictFieldList=38,TypedDictField=39,PlaceRoot=40,PathSegment=41,AccessPath=42,LiteralSet=43,ConditionNode=44,AssumptionSet=45,Qualification=46,Evidence=47,EvidenceSpan=48,CatalogMember=49,Original=50,RetrievalOrigin=51,ProviderSurface=100,MethodParameters=52}
/// A nominal current semantic key, independent of physical family names or Arrow layout.
fn entity_key(kind:EntityKind,domain:&str,key:&[u8;16])->EntityId{
    let mut sink=KeySink::new("graph-entity-key/v1");sink.part(b"kind",&(kind as u16).to_le_bytes());sink.part(b"semantic-type",domain.as_bytes());sink.part(b"key",key);EntityId(sink.finish())
}
impl EntityId {pub fn of<R:GraphEntityRecord>(id:Id<R>)->Self{entity_key(R::GRAPH_KIND,R::NAME,id.bytes())}}
impl AssertionId {
    pub fn of<R:Record>(id:Id<R>)->Self{Self::from_key(R::NAME,id.bytes())}
    fn from_key(domain:&str,key:&[u8;16])->Self{let mut sink=KeySink::new("graph-assertion-key/v1");sink.part(b"semantic-type",domain.as_bytes());sink.part(b"key",key);Self(sink.finish())}
}
/// Only explicitly selected intrinsic declarations have an entity lowering.
pub trait GraphEntityRecord:Record {const GRAPH_KIND:EntityKind;fn into_graph(self)->Entity;}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Entity {
    ProviderSurface(super::assertion::ProviderSurface),
    Package(super::input::Package),
    Release(super::input::Release),
    Capture(super::input::InputRevision),
    AcquisitionOrigin(super::input::InputOrigin),
    Source(super::source::SourceArtifact),
    Module(super::source::Module),
    Occurrence(super::source::Occurrence),
    Provider(super::attribution::Provider),
    Context(super::attribution::AnalysisContext),
    Run(super::attribution::ProviderRun),
    Scope(super::source::CoverageScope),
    Declaration(super::normalized::entities::CallableEntity),
    ClassDeclaration(super::normalized::entities::ClassEntity),
    Parameter(super::normalized::entities::ParameterEntity),
    Field(super::normalized::entities::FieldEntity),
    Exposure(super::normalized::entities::PublicExposure),
    InvocationVariant(super::normalized::callables::SignatureVariant),
    NativeModule(super::calls::ProviderModule),
    NativeSymbol(super::calls::ProviderSymbol),
    NativeCallable(super::calls::ProviderCallable),
    SignatureParameter(super::calls::SignatureParameter),
    ParameterShape(super::calls::ParameterShape),
    Type(super::types::TypeTerm),
    TypeVariable(super::types::TypeVariable),
    TypeSequence(super::types::TypeSequence),
    CallableParameterList(super::types::CallableParameterList),
    CallableParameter(super::types::CallableParameter),
    TypedDictFieldList(super::types::TypedDictFieldList),
    TypedDictField(super::types::TypedDictField),
    Literal(super::value::Literal),
    PlaceRoot(super::value::PlaceRoot),
    PathSegment(super::value::PathSegment),
    AccessPath(super::value::AccessPath),
    Place(super::value::Place),
    Predicate(super::value::Predicate),
    LiteralSet(super::value::LiteralSet),
    EvaluationAtom(super::conditions::EvaluationAtom),
    Condition(super::conditions::Condition),
    ConditionNode(super::conditions::ConditionNode),
    AssumptionUniverse(super::assumptions::AssumptionUniverse),
    Assumption(super::assumptions::Assumption),
    AssumptionSet(super::assumptions::AssumptionSet),
    Qualification(super::assertion::AssertionQualification),
    Evidence(super::assertion::Evidence),
    CatalogMember(super::catalog::CatalogMember),
    CatalogOption(super::catalog::CatalogOption),
    Scenario(super::catalog::evidence::CatalogScenario),
    Original(super::catalog::evidence::OriginalSource),
    RetrievalText(super::retrieval::CorpusText),
    RetrievalUnit(super::retrieval::Unit),
    RetrievalOrigin(super::retrieval::Origin),
    AnalysisDefinition(super::analysis::AnalysisDefinition),
    MethodParameters(super::analysis::MethodParameters),
    EmbeddingSpecification(super::embedding::EmbeddingSpec),
}
macro_rules! graph_entities {($($variant:ident:$kind:ident=>$ty:ty,)*)=>{
    $(impl GraphEntityRecord for $ty{const GRAPH_KIND:EntityKind=EntityKind::$kind;fn into_graph(self)->Entity{Entity::$variant(self)}}
      impl From<$ty> for Entity{fn from(value:$ty)->Self{Self::$variant(value)}})*
    impl Entity{
      pub fn kind(&self)->EntityKind{match self{$(Self::$variant(_)=>EntityKind::$kind,)*}}
      pub fn id(&self)->EntityId{match self{$(Self::$variant(row)=>EntityId::of(row.id()),)*}}
      pub fn content(&self)->ContentHash{let mut sink=KeySink::new("graph-entity-content/v1");sink.part(b"kind",&(self.kind() as u16).to_le_bytes());match self{$(Self::$variant(row)=>row.content_digest().encode(&mut sink),)*};sink.finish()}
      pub fn validate(&self)->Result<(),ModelError>{match self{$(Self::$variant(row)=>row.validate(),)*}}
      pub fn references(&self)->Result<Vec<(Target,Option<EntityKind>)>,ModelError>{let source=match self{$(Self::$variant(row)=>row.references(),)*};source.into_iter().map(|reference|reference_target(&reference)).collect()}
    }
};}
graph_entities! {
    ProviderSurface:ProviderSurface=>super::assertion::ProviderSurface,
    Package:Package=>super::input::Package,
    Release:Release=>super::input::Release,
    Capture:Capture=>super::input::InputRevision,
    AcquisitionOrigin:AcquisitionOrigin=>super::input::InputOrigin,
    Source:Source=>super::source::SourceArtifact,
    Module:Module=>super::source::Module,
    Occurrence:Occurrence=>super::source::Occurrence,
    Provider:Provider=>super::attribution::Provider,
    Context:Context=>super::attribution::AnalysisContext,
    Run:Run=>super::attribution::ProviderRun,
    Scope:Scope=>super::source::CoverageScope,
    Declaration:Declaration=>super::normalized::entities::CallableEntity,
    ClassDeclaration:Declaration=>super::normalized::entities::ClassEntity,
    Parameter:Parameter=>super::normalized::entities::ParameterEntity,
    Field:Field=>super::normalized::entities::FieldEntity,
    Exposure:Exposure=>super::normalized::entities::PublicExposure,
    InvocationVariant:InvocationVariant=>super::normalized::callables::SignatureVariant,
    NativeModule:NativeModule=>super::calls::ProviderModule,
    NativeSymbol:NativeSymbol=>super::calls::ProviderSymbol,
    NativeCallable:NativeCallable=>super::calls::ProviderCallable,
    SignatureParameter:Parameter=>super::calls::SignatureParameter,
    ParameterShape:ParameterShape=>super::calls::ParameterShape,
    Type:Type=>super::types::TypeTerm,
    TypeVariable:TypeVariable=>super::types::TypeVariable,
    TypeSequence:TypeSequence=>super::types::TypeSequence,
    CallableParameterList:CallableParameterList=>super::types::CallableParameterList,
    CallableParameter:CallableParameter=>super::types::CallableParameter,
    TypedDictFieldList:TypedDictFieldList=>super::types::TypedDictFieldList,
    TypedDictField:TypedDictField=>super::types::TypedDictField,
    Literal:Literal=>super::value::Literal,
    PlaceRoot:PlaceRoot=>super::value::PlaceRoot,
    PathSegment:PathSegment=>super::value::PathSegment,
    AccessPath:AccessPath=>super::value::AccessPath,
    Place:Place=>super::value::Place,
    Predicate:Predicate=>super::value::Predicate,
    LiteralSet:LiteralSet=>super::value::LiteralSet,
    EvaluationAtom:EvaluationAtom=>super::conditions::EvaluationAtom,
    Condition:Condition=>super::conditions::Condition,
    ConditionNode:ConditionNode=>super::conditions::ConditionNode,
    AssumptionUniverse:AssumptionUniverse=>super::assumptions::AssumptionUniverse,
    Assumption:Assumption=>super::assumptions::Assumption,
    AssumptionSet:AssumptionSet=>super::assumptions::AssumptionSet,
    Qualification:Qualification=>super::assertion::AssertionQualification,
    Evidence:Evidence=>super::assertion::Evidence,
    CatalogMember:CatalogMember=>super::catalog::CatalogMember,
    CatalogOption:CatalogOption=>super::catalog::CatalogOption,
    Scenario:Scenario=>super::catalog::evidence::CatalogScenario,
    Original:Original=>super::catalog::evidence::OriginalSource,
    RetrievalText:RetrievalText=>super::retrieval::CorpusText,
    RetrievalUnit:RetrievalUnit=>super::retrieval::Unit,
    RetrievalOrigin:RetrievalOrigin=>super::retrieval::Origin,
    AnalysisDefinition:AnalysisDefinition=>super::analysis::AnalysisDefinition,
    MethodParameters:MethodParameters=>super::analysis::MethodParameters,
    EmbeddingSpecification:EmbeddingSpecification=>super::embedding::EmbeddingSpec,
}
/// Mechanical nominal mapping for the selected graph vocabulary. Unsupported internal compiler
/// bookkeeping is deliberately not an entity; its producer must fold it into its semantic owner.
pub fn reference_target(reference:&super::SemanticReference)->Result<(Target,Option<EntityKind>),ModelError>{
    match reference.target {
    <super::assertion::ProviderSurface as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ProviderSurface,reference.target,&reference.key)),Some(EntityKind::ProviderSurface))),
    <super::input::Package as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Package,reference.target,&reference.key)),Some(EntityKind::Package))),
    <super::input::Release as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Release,reference.target,&reference.key)),Some(EntityKind::Release))),
    <super::input::InputRevision as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Capture,reference.target,&reference.key)),Some(EntityKind::Capture))),
    <super::input::InputOrigin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AcquisitionOrigin,reference.target,&reference.key)),Some(EntityKind::AcquisitionOrigin))),
    <super::source::SourceArtifact as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Source,reference.target,&reference.key)),Some(EntityKind::Source))),
    <super::source::Module as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Module,reference.target,&reference.key)),Some(EntityKind::Module))),
    <super::source::Occurrence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Occurrence,reference.target,&reference.key)),Some(EntityKind::Occurrence))),
    <super::attribution::Provider as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Provider,reference.target,&reference.key)),Some(EntityKind::Provider))),
    <super::attribution::AnalysisContext as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Context,reference.target,&reference.key)),Some(EntityKind::Context))),
    <super::attribution::ProviderRun as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Run,reference.target,&reference.key)),Some(EntityKind::Run))),
    <super::source::CoverageScope as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Scope,reference.target,&reference.key)),Some(EntityKind::Scope))),
    <super::normalized::entities::CallableEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Declaration,reference.target,&reference.key)),Some(EntityKind::Declaration))),
    <super::normalized::entities::ClassEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Declaration,reference.target,&reference.key)),Some(EntityKind::Declaration))),
    <super::normalized::entities::ParameterEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Parameter,reference.target,&reference.key)),Some(EntityKind::Parameter))),
    <super::normalized::entities::FieldEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Field,reference.target,&reference.key)),Some(EntityKind::Field))),
    <super::normalized::entities::PublicExposure as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Exposure,reference.target,&reference.key)),Some(EntityKind::Exposure))),
    <super::normalized::callables::SignatureVariant as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::InvocationVariant,reference.target,&reference.key)),Some(EntityKind::InvocationVariant))),
    <super::calls::ProviderModule as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeModule,reference.target,&reference.key)),Some(EntityKind::NativeModule))),
    <super::calls::ProviderSymbol as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeSymbol,reference.target,&reference.key)),Some(EntityKind::NativeSymbol))),
    <super::calls::ProviderCallable as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeCallable,reference.target,&reference.key)),Some(EntityKind::NativeCallable))),
    <super::calls::SignatureParameter as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Parameter,reference.target,&reference.key)),Some(EntityKind::Parameter))),
    <super::calls::ParameterShape as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ParameterShape,reference.target,&reference.key)),Some(EntityKind::ParameterShape))),
    <super::types::TypeTerm as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Type,reference.target,&reference.key)),Some(EntityKind::Type))),
    <super::types::TypeVariable as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypeVariable,reference.target,&reference.key)),Some(EntityKind::TypeVariable))),
    <super::types::TypeSequence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypeSequence,reference.target,&reference.key)),Some(EntityKind::TypeSequence))),
    <super::types::CallableParameterList as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallableParameterList,reference.target,&reference.key)),Some(EntityKind::CallableParameterList))),
    <super::types::CallableParameter as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallableParameter,reference.target,&reference.key)),Some(EntityKind::CallableParameter))),
    <super::types::TypedDictFieldList as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypedDictFieldList,reference.target,&reference.key)),Some(EntityKind::TypedDictFieldList))),
    <super::types::TypedDictField as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypedDictField,reference.target,&reference.key)),Some(EntityKind::TypedDictField))),
    <super::value::Literal as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Literal,reference.target,&reference.key)),Some(EntityKind::Literal))),
    <super::value::PlaceRoot as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::PlaceRoot,reference.target,&reference.key)),Some(EntityKind::PlaceRoot))),
    <super::value::PathSegment as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::PathSegment,reference.target,&reference.key)),Some(EntityKind::PathSegment))),
    <super::value::AccessPath as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AccessPath,reference.target,&reference.key)),Some(EntityKind::AccessPath))),
    <super::value::Place as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Place,reference.target,&reference.key)),Some(EntityKind::Place))),
    <super::value::Predicate as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Predicate,reference.target,&reference.key)),Some(EntityKind::Predicate))),
    <super::value::LiteralSet as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::LiteralSet,reference.target,&reference.key)),Some(EntityKind::LiteralSet))),
    <super::conditions::EvaluationAtom as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::EvaluationAtom,reference.target,&reference.key)),Some(EntityKind::EvaluationAtom))),
    <super::conditions::Condition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Condition,reference.target,&reference.key)),Some(EntityKind::Condition))),
    <super::conditions::ConditionNode as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ConditionNode,reference.target,&reference.key)),Some(EntityKind::ConditionNode))),
    <super::assumptions::AssumptionUniverse as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AssumptionUniverse,reference.target,&reference.key)),Some(EntityKind::AssumptionUniverse))),
    <super::assumptions::Assumption as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Assumption,reference.target,&reference.key)),Some(EntityKind::Assumption))),
    <super::assumptions::AssumptionSet as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AssumptionSet,reference.target,&reference.key)),Some(EntityKind::AssumptionSet))),
    <super::assertion::AssertionQualification as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Qualification,reference.target,&reference.key)),Some(EntityKind::Qualification))),
    <super::assertion::Evidence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Evidence,reference.target,&reference.key)),Some(EntityKind::Evidence))),
    <super::catalog::CatalogMember as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogMember,reference.target,&reference.key)),Some(EntityKind::CatalogMember))),
    <super::catalog::CatalogOption as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogOption,reference.target,&reference.key)),Some(EntityKind::CatalogOption))),
    <super::catalog::evidence::CatalogScenario as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Scenario,reference.target,&reference.key)),Some(EntityKind::Scenario))),
    <super::catalog::evidence::OriginalSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Original,reference.target,&reference.key)),Some(EntityKind::Original))),
    <super::retrieval::CorpusText as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalText,reference.target,&reference.key)),Some(EntityKind::RetrievalText))),
    <super::retrieval::Unit as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalUnit,reference.target,&reference.key)),Some(EntityKind::RetrievalUnit))),
    <super::retrieval::Origin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalOrigin,reference.target,&reference.key)),Some(EntityKind::RetrievalOrigin))),
    <super::analysis::AnalysisDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisDefinition,reference.target,&reference.key)),Some(EntityKind::AnalysisDefinition))),
    <super::analysis::MethodParameters as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::MethodParameters,reference.target,&reference.key)),Some(EntityKind::MethodParameters))),
    <super::embedding::EmbeddingSpec as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::EmbeddingSpecification,reference.target,&reference.key)),Some(EntityKind::EmbeddingSpecification))),
    _=>assertion_reference_target(reference),
    }
}
fn assertion_reference_target(reference:&super::SemanticReference)->Result<(Target,Option<EntityKind>),ModelError>{match reference.target{
    <super::flow::FlowUseObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowDefinitionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowReachingObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowValueObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowRegionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowTestObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowTestLeafObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::Signature as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallTarget as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::ProviderCallSite as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallSyntax as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallResolution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::declarations::SymbolDeclaration as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::declarations::ParameterDeclaration as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::TaskReportObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::DeploymentObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::PassageObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::CodeBlockObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentLinkObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentMentionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentComponentObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentAttributeObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowAttributeLoadObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowValuePathObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::LexicalScopeObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::BindingObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::ReferenceObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::LexicalResolution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::source::SyntaxObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::SymbolObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::FunctionTraitObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ClassTraitObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ClassAncestryObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ParameterAnnotationObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::PublicNameObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ParameterDocObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::SyntaxPlacement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::SyntaxDetailObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DeclarationObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DeclarationDecorator as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ImportAliasObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DunderAllObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ParameterSyntaxObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ClassFieldSyntaxObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypePresentation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeVariableRestriction as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::FunctionBodyObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::RecordFieldObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::class_metadata::ClassMemberObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::class_metadata::ClassMetadataObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::SignatureEnumerationObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffContextObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowNarrowingObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeExitObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeTerminalObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::captures::CaptureObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeExitDiagnostic as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowSourceViewObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeSignatureObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::SignatureTypeObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeQueryObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::GenericSpecializationObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_capture::FlowCaptureTimingObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ExportEnumerationObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffBindingObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::RuffDiagnosticObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::PyreflyDiagnosticObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::NativeParameterDefinitionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffDefinitionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_inventory::FlowUseInventoryObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeOverloadObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeOverloadCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ModuleResolutionObservation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_semantics::LocalAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_semantics::LocalContribution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_semantics::LocalGuardContribution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_theory::TheoryWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::records::ExpressionEvaluation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::source_call_records::SourceCallHeader as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::source_call_records::SourceInvocation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::model_transfer::ModelTransferWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_consequences::SummaryClaim as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_path::SummaryPathWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_production::SummaryRun as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::SymbolEntityResolution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::PublicEnumerationAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::CallBindingAttempt as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::CallBinding as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::BindingVariantAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::CatalogInvocation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::CatalogPath as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::CatalogAlias as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::CatalogConstructor as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::ScenarioAssociation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::DocumentAssociation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::CatalogDeployment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::ScenarioCheck as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::SelectionDomain as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::Conclusion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::UsageScore as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::ControlTraversal as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::Conclusion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::TechniqueResult as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::RankScore as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::Community as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::Neighbour as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::Concept as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::Implication as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::documentary::DocumentaryConclusion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::assertions::ProgrammaticAssertion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::briefs::Brief as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::retrieval::OriginalAnchor as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::retrieval::UnitSubject as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowUseSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowDefinitionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowReachingSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowValueSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowRegionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowTestSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowTestLeafSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::SignatureSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallTargetSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::ProviderCallSiteSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallSyntaxSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallResolutionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::declarations::SymbolDeclarationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::declarations::ParameterDeclarationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::TaskReportSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::DeploymentSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::PassageSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::CodeBlockSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentLinkSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentMentionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentComponentSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::documents::DocumentAttributeSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowAttributeLoadSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowValuePathSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::LexicalScopeSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::BindingSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::ReferenceSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::lexical::LexicalResolutionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::source::SyntaxSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::SymbolSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::FunctionTraitSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ClassTraitSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ClassAncestrySupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ParameterAnnotationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::PublicNameSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ParameterDocSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::SyntaxPlacementSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::SyntaxDetailSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DeclarationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DeclarationDecoratorSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ImportAliasSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::DunderAllSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ParameterSyntaxSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::syntax::ClassFieldSyntaxSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypePresentationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeRestrictionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::FunctionBodySupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::RecordFieldSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::class_metadata::ClassMemberSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::class_metadata::ClassMetadataSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::SignatureEnumerationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffContextSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowNarrowingSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeExitSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeTerminalSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::captures::CaptureSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::protocols::NativeExitDiagnosticSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowSourceViewSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeSignatureSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::SignatureTypeSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::TypeQuerySupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::GenericSpecializationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_capture::FlowCaptureTimingSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ExportEnumerationSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffBindingSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::RuffDiagnosticSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::PyreflyDiagnosticSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::NativeParameterDefinitionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::ruff::RuffDefinitionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_inventory::FlowUseInventorySupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeOverloadSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::types::NativeOverloadCandidateSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::symbols::ModuleResolutionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    _=>Err(invalid("internal reference has no selected semantic graph owner")),
}}
impl From<super::flow::FlowUseObservation> for NativeValue{fn from(row:super::flow::FlowUseObservation)->Self{Self::Use(row)}}
impl From<super::flow::FlowDefinitionObservation> for NativeValue{fn from(row:super::flow::FlowDefinitionObservation)->Self{Self::Definition(row)}}
impl From<super::flow::FlowReachingObservation> for NativeValue{fn from(row:super::flow::FlowReachingObservation)->Self{Self::Reaching(row)}}
impl From<super::flow::FlowValueObservation> for NativeValue{fn from(row:super::flow::FlowValueObservation)->Self{Self::Value(row)}}
impl From<super::flow::FlowRegionObservation> for NativeValue{fn from(row:super::flow::FlowRegionObservation)->Self{Self::Region(row)}}
impl From<super::flow::FlowTestObservation> for NativeValue{fn from(row:super::flow::FlowTestObservation)->Self{Self::Test(row)}}
impl From<super::flow::FlowTestLeafObservation> for NativeValue{fn from(row:super::flow::FlowTestLeafObservation)->Self{Self::Leaf(row)}}
impl From<super::calls::Signature> for NativeValue{fn from(row:super::calls::Signature)->Self{Self::Signature(row)}}
impl From<super::calls::CallTarget> for NativeValue{fn from(row:super::calls::CallTarget)->Self{Self::CallTarget(row)}}
impl From<super::calls::ProviderCallSite> for NativeValue{fn from(row:super::calls::ProviderCallSite)->Self{Self::ProviderCallSite(row)}}
impl From<super::calls::CallSyntax> for NativeValue{fn from(row:super::calls::CallSyntax)->Self{Self::CallSyntax(row)}}
impl From<super::calls::CallResolution> for NativeValue{fn from(row:super::calls::CallResolution)->Self{Self::CallResolution(row)}}
impl From<super::declarations::SymbolDeclaration> for NativeValue{fn from(row:super::declarations::SymbolDeclaration)->Self{Self::SymbolDeclaration(row)}}
impl From<super::declarations::ParameterDeclaration> for NativeValue{fn from(row:super::declarations::ParameterDeclaration)->Self{Self::ParameterDeclaration(row)}}
impl From<super::deployment::TaskReportObservation> for NativeValue{fn from(row:super::deployment::TaskReportObservation)->Self{Self::TaskReportObservation(row)}}
impl From<super::deployment::DeploymentObservation> for NativeValue{fn from(row:super::deployment::DeploymentObservation)->Self{Self::DeploymentObservation(row)}}
impl From<super::documents::DocumentObservation> for NativeValue{fn from(row:super::documents::DocumentObservation)->Self{Self::DocumentObservation(row)}}
impl From<super::documents::PassageObservation> for NativeValue{fn from(row:super::documents::PassageObservation)->Self{Self::PassageObservation(row)}}
impl From<super::documents::CodeBlockObservation> for NativeValue{fn from(row:super::documents::CodeBlockObservation)->Self{Self::CodeBlockObservation(row)}}
impl From<super::documents::DocumentLinkObservation> for NativeValue{fn from(row:super::documents::DocumentLinkObservation)->Self{Self::DocumentLinkObservation(row)}}
impl From<super::documents::DocumentMentionObservation> for NativeValue{fn from(row:super::documents::DocumentMentionObservation)->Self{Self::DocumentMentionObservation(row)}}
impl From<super::documents::DocumentComponentObservation> for NativeValue{fn from(row:super::documents::DocumentComponentObservation)->Self{Self::DocumentComponentObservation(row)}}
impl From<super::documents::DocumentAttributeObservation> for NativeValue{fn from(row:super::documents::DocumentAttributeObservation)->Self{Self::DocumentAttributeObservation(row)}}
impl From<super::flow::FlowAttributeLoadObservation> for NativeValue{fn from(row:super::flow::FlowAttributeLoadObservation)->Self{Self::FlowAttributeLoadObservation(row)}}
impl From<super::flow::FlowValuePathObservation> for NativeValue{fn from(row:super::flow::FlowValuePathObservation)->Self{Self::FlowValuePathObservation(row)}}
impl From<super::lexical::LexicalScopeObservation> for NativeValue{fn from(row:super::lexical::LexicalScopeObservation)->Self{Self::LexicalScopeObservation(row)}}
impl From<super::lexical::BindingObservation> for NativeValue{fn from(row:super::lexical::BindingObservation)->Self{Self::BindingObservation(row)}}
impl From<super::lexical::ReferenceObservation> for NativeValue{fn from(row:super::lexical::ReferenceObservation)->Self{Self::ReferenceObservation(row)}}
impl From<super::lexical::LexicalResolution> for NativeValue{fn from(row:super::lexical::LexicalResolution)->Self{Self::LexicalResolution(row)}}
impl From<super::source::SyntaxObservation> for NativeValue{fn from(row:super::source::SyntaxObservation)->Self{Self::SyntaxObservation(row)}}
impl From<super::symbols::SymbolObservation> for NativeValue{fn from(row:super::symbols::SymbolObservation)->Self{Self::SymbolObservation(row)}}
impl From<super::symbols::FunctionTraitObservation> for NativeValue{fn from(row:super::symbols::FunctionTraitObservation)->Self{Self::FunctionTraitObservation(row)}}
impl From<super::symbols::ClassTraitObservation> for NativeValue{fn from(row:super::symbols::ClassTraitObservation)->Self{Self::ClassTraitObservation(row)}}
impl From<super::symbols::ClassAncestryObservation> for NativeValue{fn from(row:super::symbols::ClassAncestryObservation)->Self{Self::ClassAncestryObservation(row)}}
impl From<super::symbols::ParameterAnnotationObservation> for NativeValue{fn from(row:super::symbols::ParameterAnnotationObservation)->Self{Self::ParameterAnnotationObservation(row)}}
impl From<super::symbols::PublicNameObservation> for NativeValue{fn from(row:super::symbols::PublicNameObservation)->Self{Self::PublicNameObservation(row)}}
impl From<super::symbols::ParameterDocObservation> for NativeValue{fn from(row:super::symbols::ParameterDocObservation)->Self{Self::ParameterDocObservation(row)}}
impl From<super::syntax::SyntaxPlacement> for NativeValue{fn from(row:super::syntax::SyntaxPlacement)->Self{Self::SyntaxPlacement(row)}}
impl From<super::syntax::SyntaxDetailObservation> for NativeValue{fn from(row:super::syntax::SyntaxDetailObservation)->Self{Self::SyntaxDetailObservation(row)}}
impl From<super::syntax::DeclarationObservation> for NativeValue{fn from(row:super::syntax::DeclarationObservation)->Self{Self::DeclarationObservation(row)}}
impl From<super::syntax::DeclarationDecorator> for NativeValue{fn from(row:super::syntax::DeclarationDecorator)->Self{Self::DeclarationDecorator(row)}}
impl From<super::syntax::ImportAliasObservation> for NativeValue{fn from(row:super::syntax::ImportAliasObservation)->Self{Self::ImportAliasObservation(row)}}
impl From<super::syntax::DunderAllObservation> for NativeValue{fn from(row:super::syntax::DunderAllObservation)->Self{Self::DunderAllObservation(row)}}
impl From<super::syntax::ParameterSyntaxObservation> for NativeValue{fn from(row:super::syntax::ParameterSyntaxObservation)->Self{Self::ParameterSyntaxObservation(row)}}
impl From<super::syntax::ClassFieldSyntaxObservation> for NativeValue{fn from(row:super::syntax::ClassFieldSyntaxObservation)->Self{Self::ClassFieldSyntaxObservation(row)}}
impl From<super::types::TypeObservation> for NativeValue{fn from(row:super::types::TypeObservation)->Self{Self::TypeObservation(row)}}
impl From<super::types::TypePresentation> for NativeValue{fn from(row:super::types::TypePresentation)->Self{Self::TypePresentation(row)}}
impl From<super::types::TypeVariableRestriction> for NativeValue{fn from(row:super::types::TypeVariableRestriction)->Self{Self::TypeVariableRestriction(row)}}
impl From<super::types::FunctionBodyObservation> for NativeValue{fn from(row:super::types::FunctionBodyObservation)->Self{Self::FunctionBodyObservation(row)}}
impl From<super::types::RecordFieldObservation> for NativeValue{fn from(row:super::types::RecordFieldObservation)->Self{Self::RecordFieldObservation(row)}}
impl From<super::class_metadata::ClassMemberObservation> for NativeValue{fn from(row:super::class_metadata::ClassMemberObservation)->Self{Self::ClassMemberObservation(row)}}
impl From<super::class_metadata::ClassMetadataObservation> for NativeValue{fn from(row:super::class_metadata::ClassMetadataObservation)->Self{Self::ClassMetadataObservation(row)}}
impl From<super::calls::SignatureEnumerationObservation> for NativeValue{fn from(row:super::calls::SignatureEnumerationObservation)->Self{Self::SignatureEnumerationObservation(row)}}
impl From<super::ruff::RuffContextObservation> for NativeValue{fn from(row:super::ruff::RuffContextObservation)->Self{Self::RuffContextObservation(row)}}
impl From<super::flow::FlowNarrowingObservation> for NativeValue{fn from(row:super::flow::FlowNarrowingObservation)->Self{Self::FlowNarrowing(row)}}
impl From<super::protocols::NativeExitObservation> for NativeValue{fn from(row:super::protocols::NativeExitObservation)->Self{Self::NativeExit(row)}}
impl From<super::protocols::NativeTerminalObservation> for NativeValue{fn from(row:super::protocols::NativeTerminalObservation)->Self{Self::NativeTerminal(row)}}
impl From<super::captures::CaptureObservation> for NativeValue{fn from(row:super::captures::CaptureObservation)->Self{Self::Capture(row)}}
impl From<super::protocols::NativeExitDiagnostic> for NativeValue{fn from(row:super::protocols::NativeExitDiagnostic)->Self{Self::NativeExitDiagnostic(row)}}
impl From<super::flow::FlowSourceViewObservation> for NativeValue{fn from(row:super::flow::FlowSourceViewObservation)->Self{Self::FlowSourceView(row)}}
impl From<super::types::NativeSignatureObservation> for NativeValue{fn from(row:super::types::NativeSignatureObservation)->Self{Self::NativeSignatureObservation(row)}}
impl From<super::types::SignatureTypeObservation> for NativeValue{fn from(row:super::types::SignatureTypeObservation)->Self{Self::SignatureTypeObservation(row)}}
impl From<super::types::TypeQueryObservation> for NativeValue{fn from(row:super::types::TypeQueryObservation)->Self{Self::TypeQuery(row)}}
impl From<super::types::GenericSpecializationObservation> for NativeValue{fn from(row:super::types::GenericSpecializationObservation)->Self{Self::GenericSpecialization(row)}}
impl From<super::flow_capture::FlowCaptureTimingObservation> for NativeValue{fn from(row:super::flow_capture::FlowCaptureTimingObservation)->Self{Self::FlowCaptureTiming(row)}}
impl From<super::symbols::ExportEnumerationObservation> for NativeValue{fn from(row:super::symbols::ExportEnumerationObservation)->Self{Self::ExportEnumeration(row)}}
impl From<super::ruff::RuffBindingObservation> for NativeValue{fn from(row:super::ruff::RuffBindingObservation)->Self{Self::RuffBindingObservation(row)}}
impl From<super::diagnostics::RuffDiagnosticObservation> for NativeValue{fn from(row:super::diagnostics::RuffDiagnosticObservation)->Self{Self::RuffDiagnosticObservation(row)}}
impl From<super::diagnostics::PyreflyDiagnosticObservation> for NativeValue{fn from(row:super::diagnostics::PyreflyDiagnosticObservation)->Self{Self::PyreflyDiagnosticObservation(row)}}
impl From<super::diagnostics::NativeParameterDefinitionObservation> for NativeValue{fn from(row:super::diagnostics::NativeParameterDefinitionObservation)->Self{Self::NativeParameterDefinitionObservation(row)}}
impl From<super::ruff::RuffDefinitionObservation> for NativeValue{fn from(row:super::ruff::RuffDefinitionObservation)->Self{Self::RuffDefinitionObservation(row)}}
impl From<super::flow_inventory::FlowUseInventoryObservation> for NativeValue{fn from(row:super::flow_inventory::FlowUseInventoryObservation)->Self{Self::FlowUseInventory(row)}}
impl From<super::types::NativeOverloadObservation> for NativeValue{fn from(row:super::types::NativeOverloadObservation)->Self{Self::NativeOverload(row)}}
impl From<super::types::NativeOverloadCandidate> for NativeValue{fn from(row:super::types::NativeOverloadCandidate)->Self{Self::NativeOverloadCandidate(row)}}
impl From<super::symbols::ModuleResolutionObservation> for NativeValue{fn from(row:super::symbols::ModuleResolutionObservation)->Self{Self::ModuleResolutionObservation(row)}}
impl From<super::local_semantics::LocalAssessment> for AnalysisValue{fn from(row:super::local_semantics::LocalAssessment)->Self{Self::LocalAssessment(row)}}
impl From<super::local_semantics::LocalContribution> for AnalysisValue{fn from(row:super::local_semantics::LocalContribution)->Self{Self::LocalTransfer(row)}}
impl From<super::local_semantics::LocalGuardContribution> for AnalysisValue{fn from(row:super::local_semantics::LocalGuardContribution)->Self{Self::LocalGuard(row)}}
impl From<super::local_theory::TheoryWitness> for AnalysisValue{fn from(row:super::local_theory::TheoryWitness)->Self{Self::Theory(row)}}
impl From<super::execution::records::ExpressionEvaluation> for AnalysisValue{fn from(row:super::execution::records::ExpressionEvaluation)->Self{Self::ExpressionEvaluation(row)}}
impl From<super::execution::source_call_records::SourceCallHeader> for AnalysisValue{fn from(row:super::execution::source_call_records::SourceCallHeader)->Self{Self::SourceCall(row)}}
impl From<super::execution::source_call_records::SourceInvocation> for AnalysisValue{fn from(row:super::execution::source_call_records::SourceInvocation)->Self{Self::SourceInvocation(row)}}
impl From<super::execution::model_transfer::ModelTransferWitness> for AnalysisValue{fn from(row:super::execution::model_transfer::ModelTransferWitness)->Self{Self::ModelTransfer(row)}}
impl From<super::execution::summary_consequences::SummaryClaim> for AnalysisValue{fn from(row:super::execution::summary_consequences::SummaryClaim)->Self{Self::SummaryClaim(row)}}
impl From<super::execution::summary_path::SummaryPathWitness> for AnalysisValue{fn from(row:super::execution::summary_path::SummaryPathWitness)->Self{Self::SummaryPath(row)}}
impl From<super::execution::summary_production::SummaryRun> for AnalysisValue{fn from(row:super::execution::summary_production::SummaryRun)->Self{Self::SummaryRun(row)}}
impl From<super::normalized::entities::SymbolEntityResolution> for AnalysisValue{fn from(row:super::normalized::entities::SymbolEntityResolution)->Self{Self::NormalizedSymbol(row)}}
impl From<super::normalized::entities::PublicEnumerationAssessment> for AnalysisValue{fn from(row:super::normalized::entities::PublicEnumerationAssessment)->Self{Self::PublicEnumeration(row)}}
impl From<super::normalized::bindings::CallBindingAttempt> for AnalysisValue{fn from(row:super::normalized::bindings::CallBindingAttempt)->Self{Self::BindingAttempt(row)}}
impl From<super::normalized::bindings::CallBinding> for AnalysisValue{fn from(row:super::normalized::bindings::CallBinding)->Self{Self::Binding(row)}}
impl From<super::normalized::bindings::BindingVariantAssessment> for AnalysisValue{fn from(row:super::normalized::bindings::BindingVariantAssessment)->Self{Self::BindingVariant(row)}}
impl From<super::catalog::CatalogInvocation> for AnalysisValue{fn from(row:super::catalog::CatalogInvocation)->Self{Self::CatalogInvocation(row)}}
impl From<super::catalog::CatalogPath> for AnalysisValue{fn from(row:super::catalog::CatalogPath)->Self{Self::CatalogPath(row)}}
impl From<super::catalog::CatalogAlias> for AnalysisValue{fn from(row:super::catalog::CatalogAlias)->Self{Self::CatalogAlias(row)}}
impl From<super::catalog::CatalogConstructor> for AnalysisValue{fn from(row:super::catalog::CatalogConstructor)->Self{Self::CatalogConstructor(row)}}
impl From<super::catalog::evidence::ScenarioAssociation> for AnalysisValue{fn from(row:super::catalog::evidence::ScenarioAssociation)->Self{Self::ScenarioAssociation(row)}}
impl From<super::catalog::evidence::DocumentAssociation> for AnalysisValue{fn from(row:super::catalog::evidence::DocumentAssociation)->Self{Self::DocumentAssociation(row)}}
impl From<super::catalog::evidence::CatalogDeployment> for AnalysisValue{fn from(row:super::catalog::evidence::CatalogDeployment)->Self{Self::Deployment(row)}}
impl From<super::catalog::evidence::ScenarioCheck> for AnalysisValue{fn from(row:super::catalog::evidence::ScenarioCheck)->Self{Self::ScenarioCheck(row)}}
impl From<super::selection::SelectionDomain> for AnalysisValue{fn from(row:super::selection::SelectionDomain)->Self{Self::SelectionDomain(row)}}
impl From<super::structural::Conclusion> for AnalysisValue{fn from(row:super::structural::Conclusion)->Self{Self::StructuralConclusion(row)}}
impl From<super::structural::UsageScore> for AnalysisValue{fn from(row:super::structural::UsageScore)->Self{Self::StructuralUsage(row)}}
impl From<super::structural::controls::ControlTraversal> for AnalysisValue{fn from(row:super::structural::controls::ControlTraversal)->Self{Self::StructuralControl(row)}}
impl From<super::analytics::Conclusion> for AnalysisValue{fn from(row:super::analytics::Conclusion)->Self{Self::AnalyticConclusion(row)}}
impl From<super::analytics::TechniqueResult> for AnalysisValue{fn from(row:super::analytics::TechniqueResult)->Self{Self::AnalyticTechnique(row)}}
impl From<super::analytics::RankScore> for AnalysisValue{fn from(row:super::analytics::RankScore)->Self{Self::Rank(row)}}
impl From<super::analytics::Community> for AnalysisValue{fn from(row:super::analytics::Community)->Self{Self::Community(row)}}
impl From<super::analytics::Neighbour> for AnalysisValue{fn from(row:super::analytics::Neighbour)->Self{Self::Neighbour(row)}}
impl From<super::analytics::Concept> for AnalysisValue{fn from(row:super::analytics::Concept)->Self{Self::Concept(row)}}
impl From<super::analytics::Implication> for AnalysisValue{fn from(row:super::analytics::Implication)->Self{Self::Implication(row)}}
impl From<super::synthesis::documentary::DocumentaryConclusion> for AnalysisValue{fn from(row:super::synthesis::documentary::DocumentaryConclusion)->Self{Self::DocumentaryConclusion(row)}}
impl From<super::synthesis::assertions::ProgrammaticAssertion> for AnalysisValue{fn from(row:super::synthesis::assertions::ProgrammaticAssertion)->Self{Self::ProgrammaticAssertion(row)}}
impl From<super::synthesis::briefs::Brief> for AnalysisValue{fn from(row:super::synthesis::briefs::Brief)->Self{Self::Brief(row)}}
impl From<super::retrieval::OriginalAnchor> for AnalysisValue{fn from(row:super::retrieval::OriginalAnchor)->Self{Self::RetrievalAnchor(row)}}
impl From<super::retrieval::UnitSubject> for AnalysisValue{fn from(row:super::retrieval::UnitSubject)->Self{Self::RetrievalOccurrence(row)}}
/// Unresolved external Python targets retain their provider/context and reason explicitly.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Target {
    Entity(EntityId), Assertion(AssertionId),
    External {provider:EntityId,context:EntityId,name:String,reason:super::normalized::entities::EntityReason},
}
impl Key for Target {
    fn encode(&self,sink:&mut KeySink) {match self {
        Self::Entity(id)=>{sink.part(b"target",&[0]);id.encode(sink);},
        Self::Assertion(id)=>{sink.part(b"target",&[1]);id.encode(sink);},
        Self::External {provider,context,name,reason}=>{sink.part(b"target",&[2]);provider.encode(sink);context.encode(sink);name.encode(sink);reason.encode(sink);}
    }}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[repr(u16)]
pub enum ParticipantRole {Subject=0,Object=1,Caller=2,Callee=3,Argument=4,Parameter=5,Exposure=6,Declaration=7,Evidence=8,Context=9}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {pub role:ParticipantRole,pub target:Target}
impl Key for Participant {fn encode(&self,sink:&mut KeySink){sink.part(b"role",&(self.role as u16).to_le_bytes());self.target.encode(sink);}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InlineQualification {
    pub context:EntityId,pub scope:EntityId,pub condition:EntityId,
    pub modality:super::attribution::Modality,pub approximation:super::assertion::Approximation,
    pub assumptions:Vec<EntityId>,
}
impl Key for InlineQualification {fn encode(&self,sink:&mut KeySink){self.context.encode(sink);self.scope.encode(sink);self.condition.encode(sink);self.modality.encode(sink);self.approximation.encode(sink);self.assumptions.encode(sink);}}

#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Qualification {Ref(EntityId),Inline(InlineQualification),Payload}
impl Key for Qualification{fn encode(&self,sink:&mut KeySink){match self{Self::Ref(id)=>{sink.part(b"qualification",&[0]);id.encode(sink);},Self::Inline(value)=>{sink.part(b"qualification",&[1]);value.encode(sink);},Self::Payload=>sink.part(b"qualification",&[2])}}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticKey {domain:String,key:[u8;16]}
impl SemanticKey{pub fn of<R:Record>(id:Id<R>)->Self{Self{domain:R::NAME.into(),key:*id.bytes()}}}
impl Key for SemanticKey{fn encode(&self,sink:&mut KeySink){self.domain.encode(sink);sink.part(b"key",&self.key);}}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[repr(u16)]
pub enum AssertionKind {SyntaxSpelling=0,DeclarationCorrespondence=1,ExposureTarget=2,InvocationSignature=3,ParameterBinding=4,EvidenceAssociation=5,SelectionAssessment=6,DerivedConclusion=7,
    NativeObservation=8,ValueTransfer=9,Requirement=10,PredicateDomain=11,OptionContract=12,ScenarioAssociation=13,DeploymentAssociation=14,SummaryTransfer=15,StructuralMembership=16,StructuralOrder=17,ConceptMembership=18,EmbeddingWitness=19,RetrievalOccurrence=20,RetrievalFamilyMembership=21,DocumentaryEvidence=22,SynthesisClaim=23,InputAcquisition=24,InputDistribution=25,CorpusMembership=26,ArtifactUse=27}
/// A finite payload vocabulary; provider answers remain separate attributed assertions.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AssertionValue {None,Text(String),Applicability(super::normalized::entities::ResolutionStatus),Verdict(super::obligation::Verdict),Order(i64),Domain{members:Vec<EntityId>,closed:bool},Signature{parameters:Vec<EntityId>,returns:Option<EntityId>},Predicate{predicate:EntityId},Native(NativeValue),Analysis(AnalysisValue),Support(SupportValue),Acquisition(super::input::InputOrigin)}
impl Key for AssertionValue {fn encode(&self,sink:&mut KeySink){match self {
    Self::None=>sink.part(b"value",&[0]),Self::Text(v)=>{sink.part(b"value",&[1]);v.encode(sink);},
    Self::Applicability(v)=>{sink.part(b"value",&[2]);v.encode(sink);},Self::Verdict(v)=>{sink.part(b"value",&[3]);v.encode(sink);},
    Self::Order(v)=>{sink.part(b"value",&[4]);v.encode(sink);},Self::Domain{members,closed}=>{sink.part(b"value",&[5]);members.encode(sink);closed.encode(sink);},Self::Signature{parameters,returns}=>{sink.part(b"value",&[6]);parameters.encode(sink);returns.encode(sink);},Self::Predicate{predicate}=>{sink.part(b"value",&[7]);predicate.encode(sink);},Self::Native(value)=>{sink.part(b"value",&[8]);value.encode(sink);},Self::Analysis(value)=>{sink.part(b"value",&[9]);value.encode(sink);},Self::Acquisition(value)=>{sink.part(b"value",&[10]);value.write_key(sink);},Self::Support(value)=>{sink.part(b"value",&[11]);value.encode(sink);}
}}}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
#[repr(u16)]
pub enum OutcomeKind {Complete=0,Partial=1,Unavailable=2,Failed=3,Refused=4,NotRequested=5}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {pub rule:String,pub revision:u32,pub premises:Vec<AssertionId>,pub assumptions:Vec<EntityId>,pub outcome:OutcomeKind}
impl Key for Derivation {fn encode(&self,sink:&mut KeySink){self.rule.encode(sink);sink.part(b"u32", &self.revision.to_le_bytes());self.premises.encode(sink);self.assumptions.encode(sink);sink.part(b"outcome",&(self.outcome as u16).to_le_bytes());}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    pub source:Option<SemanticKey>,
    pub kind:AssertionKind,pub participants:Vec<Participant>,pub qualification:Qualification,
    pub run:Option<EntityId>,pub evidence:Vec<EntityId>,pub value:AssertionValue,pub derivation:Option<Derivation>,
}
impl Assertion {
    pub fn id(&self)->AssertionId {if let Some(source)=&self.source{return AssertionId::from_key(&source.domain,&source.key);}let mut sink=KeySink::new("graph-assertion/v1");self.encode(&mut sink);AssertionId(sink.finish())}
    pub fn content(&self)->ContentHash {let mut sink=KeySink::new("graph-assertion-content/v1");self.encode(&mut sink);sink.finish()}
    /// Nominal reference obligations for bounded bulk closure over completed families. The
    /// compiler emits these as a compact stream and joins against membership once.
    pub fn references(&self)->Result<Vec<(Target,Option<EntityKind>)>,ModelError>{
        use EntityKind as K;
        let mut refs=Vec::new();
        match &self.qualification {Qualification::Ref(id)=>refs.push((Target::Entity(*id),Some(K::Qualification))),Qualification::Inline(q)=>{refs.extend([(Target::Entity(q.context),Some(K::Context)),(Target::Entity(q.scope),Some(K::Scope)),(Target::Entity(q.condition),Some(K::Condition))]);refs.extend(q.assumptions.iter().map(|id|(Target::Entity(*id),Some(K::Assumption))));},Qualification::Payload=>{}}
        refs.extend(self.run.map(|id|(Target::Entity(id),Some(K::Run))));
        refs.extend(self.evidence.iter().map(|id|(Target::Entity(*id),Some(K::Evidence))));
        refs.extend(self.participants.iter().map(|p|{let kind=match p.role{ParticipantRole::Declaration=>Some(K::Declaration),ParticipantRole::Exposure=>Some(K::Exposure),ParticipantRole::Context=>Some(K::Context),ParticipantRole::Evidence=>Some(K::Evidence),ParticipantRole::Argument=>Some(K::Occurrence),ParticipantRole::Parameter=>Some(K::Parameter),_=>None};(p.target.clone(),kind)}));
        let payload=match &self.value {AssertionValue::Native(value)=>value.references(),AssertionValue::Analysis(value)=>value.references(),AssertionValue::Support(value)=>value.references(),_=>vec![]};
        for reference in payload {refs.push(reference_target(&reference)?);}
        match &self.value {AssertionValue::Domain{members,..}=>refs.extend(members.iter().map(|id|(Target::Entity(*id),None))),AssertionValue::Signature{parameters,returns}=>{refs.extend(parameters.iter().map(|id|(Target::Entity(*id),Some(K::Parameter))));refs.extend(returns.map(|id|(Target::Entity(id),Some(K::Type))));},AssertionValue::Predicate{predicate}=>refs.push((Target::Entity(*predicate),Some(K::Predicate))),_=>{}}
        if let Some(d)=&self.derivation{refs.extend(d.premises.iter().map(|id|(Target::Assertion(*id),None)));refs.extend(d.assumptions.iter().map(|id|(Target::Entity(*id),Some(K::Assumption))));}
        Ok(refs)
    }
    fn encode(&self,sink:&mut KeySink){self.source.encode(sink);sink.part(b"kind",&(self.kind as u16).to_le_bytes());self.participants.encode(sink);self.qualification.encode(sink);self.run.encode(sink);self.evidence.encode(sink);self.value.encode(sink);self.derivation.encode(sink);}
    pub fn validate(&self)->Result<(),ModelError>{
        match &self.value {AssertionValue::Native(v)=>v.validate()?,AssertionValue::Analysis(v)=>v.validate()?,AssertionValue::Acquisition(v)=>v.validate()?,AssertionValue::Support(v)=>v.validate()?,_=>{}}
        let expected_source=match &self.value{AssertionValue::Native(v)=>Some(v.semantic_key()),AssertionValue::Analysis(v)=>Some(v.semantic_key()),AssertionValue::Support(v)=>Some(v.semantic_key()),_=>None};
        if expected_source.is_some() && self.source!=expected_source {return Err(invalid("assertion key differs from its typed semantic payload"));}
        if matches!(self.qualification,Qualification::Payload) && !matches!(self.value,AssertionValue::Native(_)|AssertionValue::Analysis(_)|AssertionValue::Support(_)){return Err(invalid("qualification payload needs a typed contextual owner"));}
        if self.participants.is_empty() && !matches!(self.value,AssertionValue::Native(_)|AssertionValue::Analysis(_)|AssertionValue::Support(_)) {return Err(invalid("assertion needs participants"));}
        if matches!(&self.qualification,Qualification::Inline(q) if !strictly_ordered(&q.assumptions)) || !strictly_ordered(&self.evidence) {return Err(invalid("assertion sets must be sorted and unique"));}
        if let Some(derivation)=&self.derivation {
            if derivation.rule.is_empty() || derivation.revision==0 || derivation.premises.is_empty() || !strictly_ordered(&derivation.assumptions) {return Err(invalid("derivation needs rule revision, ordered premises and canonical assumptions"));}
        }
        // Premises and participants are sequences: never sort them to manufacture canonicality.
        Ok(())
    }
}
fn strictly_ordered<T:Ord>(values:&[T])->bool {values.windows(2).all(|w|w[0]<w[1])}
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}

/// Membership lookup is supplied by immutable completed local indexes. It is not a store grant.
pub trait GraphLookup {
    fn entity_kind(&self,id:EntityId)->Result<Option<EntityKind>,ModelError>;
    fn assertion_exists(&self,id:AssertionId)->Result<bool,ModelError>;
    fn source_length(&self,id:EntityId)->Result<Option<u64>,ModelError>;
}
pub fn admit_entity(entity:&Entity,lookup:&impl GraphLookup)->Result<(),ModelError>{
    entity.validate()?;for (target,kind) in entity.references()?{admit_reference(target,kind,lookup)?;}
    if let Entity::Occurrence(row)=entity {if lookup.source_length(EntityId::of(row.source))?.is_none_or(|length|u64::try_from(row.end).ok().is_none_or(|end|end>length)){return Err(invalid("occurrence extends beyond captured source"));}}
    if let Entity::Evidence(super::assertion::Evidence::SourceSpan{source,end,..})=entity {if lookup.source_length(EntityId::of(*source))?.is_none_or(|length|u64::try_from(*end).ok().is_none_or(|end|end>length)){return Err(invalid("evidence extends beyond captured source"));}}
    Ok(())
}
fn require_kind(lookup:&impl GraphLookup,id:EntityId,kind:EntityKind)->Result<(),ModelError>{if lookup.entity_kind(id)?!=Some(kind){return Err(invalid("missing internal entity or wrong nominal kind"));}Ok(())}
fn admit_reference(target:Target,kind:Option<EntityKind>,lookup:&impl GraphLookup)->Result<(),ModelError>{match target{
    Target::Entity(id)=>if let Some(kind)=kind {require_kind(lookup,id,kind)?;} else if lookup.entity_kind(id)?.is_none(){return Err(invalid("missing internal participant"));},
    Target::Assertion(id)=>{if kind.is_some() || !lookup.assertion_exists(id)?{return Err(invalid("missing referenced assertion or wrong nominal role"));}},
    Target::External{provider,context,name,..}=>{if kind.is_some(){return Err(invalid("external target cannot fill an internal nominal role"));}require_kind(lookup,provider,EntityKind::Provider)?;require_kind(lookup,context,EntityKind::Context)?;if name.is_empty(){return Err(invalid("external target needs name"));}},
}Ok(())}
pub fn admit_assertion(assertion:&Assertion,lookup:&impl GraphLookup)->Result<(),ModelError>{assertion.validate()?;for (target,kind) in assertion.references()?{admit_reference(target,kind,lookup)?;}Ok(())}

pub const ARTIFACT_FORMAT_VERSION:u32=1;
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
#[repr(u16)]
pub enum GraphFamily {Entities=0,Assertions=1,Outcomes=2,Originals=3,Projection=4,EmbeddingValues=5}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyContent {pub family:GraphFamily,pub rows:u64,pub content:ContentHash}
/// Incremental canonical content, independent of IPC framing and transport fragmentation.
/// Ordered equal keys deduplicate only when the complete semantic payload agrees.
pub struct FamilyHasher {family:GraphFamily,sink:KeySink,previous:Option<(ContentHash,ContentHash)>,rows:u64}
impl FamilyHasher {
    pub fn new(family:GraphFamily)->Self {let mut sink=KeySink::new("graph-family/v1");sink.part(b"family",&(family as u16).to_le_bytes());Self{family,sink,previous:None,rows:0}}
    pub fn push(&mut self,key:ContentHash,content:ContentHash)->Result<bool,ModelError>{
        if let Some((previous,payload))=self.previous {
            if key<previous {return Err(invalid("graph family must be ordered by canonical key"));}
            if key==previous {if content!=payload{return Err(ModelError::Conflict("graph family"));}return Ok(false);}
        }
        key.encode(&mut self.sink);content.encode(&mut self.sink);
        self.previous=Some((key,content));self.rows=self.rows.checked_add(1).ok_or_else(||invalid("graph family row count overflow"))?;Ok(true)
    }
    pub fn finish(self)->FamilyContent {FamilyContent {family:self.family,rows:self.rows,content:self.sink.finish()}}
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerImplementation {pub producer:String,pub implementation:ContentHash,pub configuration:ContentHash}
#[derive(Debug,Clone,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeKey {pub producer:String,pub scope:EntityId,pub domain:ContentHash}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {pub key:OutcomeKey,pub status:OutcomeKind,pub observed:u64,pub detail:Option<String>}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {pub source:EntityId,pub content:ContentHash,pub byte_len:u64}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionDefinition {pub name:String,pub definition:ContentHash,pub source_membership:ContentHash,pub declared_losses:Vec<String>}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConsumption {pub specification:ContentHash,pub text:ContentHash,pub dimension:u32,pub values:ContentHash}
/// Content declaration of a compiler artifact. Physical indexes, functions and engine versions
/// enter a publisher's realization identity, never this semantic content manifest.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format_version:u32,pub frontier:super::admission::Frontier,pub profile:super::stages::Profile,
    pub captures:Vec<EntityId>,pub semantic_contract:ContentHash,pub producers:Vec<ProducerImplementation>,
    pub settings:ContentHash,pub families:Vec<FamilyContent>,pub required_outcomes:Vec<OutcomeKey>,
    pub outcomes:Vec<Outcome>,pub originals:Vec<Original>,pub projections:Vec<ProjectionDefinition>,
    pub embeddings:Vec<EmbeddingConsumption>,
}
impl Manifest {
    pub fn content(&self)->ContentHash {
        let mut sink=KeySink::new("graph-artifact/v1");
        sink.part(b"u32", &self.format_version.to_le_bytes());sink.part(b"frontier", self.frontier.name().as_bytes());sink.part(b"profile", self.profile.name().as_bytes());
        self.captures.encode(&mut sink);self.semantic_contract.encode(&mut sink);self.settings.encode(&mut sink);
        sink.part(b"producers",&(self.producers.len() as u64).to_le_bytes());
        for p in &self.producers {p.producer.encode(&mut sink);p.implementation.encode(&mut sink);p.configuration.encode(&mut sink);}
        sink.part(b"families",&(self.families.len() as u64).to_le_bytes());
        for family in &self.families {sink.part(b"family",&(family.family as u16).to_le_bytes());sink.part(b"u64", &family.rows.to_le_bytes());family.content.encode(&mut sink);}
        sink.part(b"required-outcomes",&(self.required_outcomes.len() as u64).to_le_bytes());
        for key in &self.required_outcomes {encode_outcome_key(key,&mut sink);}
        sink.part(b"outcomes",&(self.outcomes.len() as u64).to_le_bytes());
        for outcome in &self.outcomes {encode_outcome_key(&outcome.key,&mut sink);sink.part(b"status",&(outcome.status as u16).to_le_bytes());sink.part(b"u64", &outcome.observed.to_le_bytes());outcome.detail.encode(&mut sink);}
        sink.part(b"originals",&(self.originals.len() as u64).to_le_bytes());
        for original in &self.originals {original.source.encode(&mut sink);original.content.encode(&mut sink);sink.part(b"u64", &original.byte_len.to_le_bytes());}
        sink.part(b"projections",&(self.projections.len() as u64).to_le_bytes());
        for projection in &self.projections {projection.name.encode(&mut sink);projection.definition.encode(&mut sink);projection.source_membership.encode(&mut sink);projection.declared_losses.encode(&mut sink);}
        sink.part(b"embeddings",&(self.embeddings.len() as u64).to_le_bytes());
        for vector in &self.embeddings {vector.specification.encode(&mut sink);vector.text.encode(&mut sink);sink.part(b"u32", &vector.dimension.to_le_bytes());vector.values.encode(&mut sink);}
        sink.finish()
    }
    pub fn validate(&self)->Result<(),ModelError> {
        if self.format_version!=ARTIFACT_FORMAT_VERSION || self.frontier==super::admission::Frontier::Conformance {return Err(invalid("unsupported artifact format or frontier"));}
        if !strictly_ordered(&self.captures) || !strictly_ordered(&self.required_outcomes) {return Err(invalid("manifest captures and obligations must be ordered and unique"));}
        if !self.families.windows(2).all(|w|w[0].family<w[1].family) || !self.producers.windows(2).all(|w|w[0].producer<w[1].producer) {return Err(invalid("manifest families and producers must be ordered and unique"));}
        if self.producers.iter().any(|p|p.producer.is_empty()) {return Err(invalid("manifest producer needs name"));}
        if self.outcomes.len()!=self.required_outcomes.len() || self.outcomes.iter().zip(&self.required_outcomes).any(|(actual,expected)|actual.key!=*expected) {return Err(invalid("manifest outcomes differ from exact required domain"));}
        for outcome in &self.outcomes {
            if outcome.key.producer.is_empty() || (outcome.status==OutcomeKind::NotRequested && outcome.observed!=0) {return Err(invalid("invalid outcome metadata"));}
        }
        if !self.originals.windows(2).all(|w|w[0].source<w[1].source) || !self.projections.windows(2).all(|w|w[0].name<w[1].name) || !self.embeddings.windows(2).all(|w|(w[0].specification,w[0].text)<(w[1].specification,w[1].text)) {return Err(invalid("manifest originals, projections and embeddings must be ordered and unique"));}
        if self.embeddings.iter().any(|v|v.dimension==0) {return Err(invalid("embedding consumption needs dimensions"));}
        Ok(())
    }
}
fn encode_outcome_key(key:&OutcomeKey,sink:&mut KeySink){key.producer.encode(sink);key.scope.encode(sink);key.domain.encode(sink);}

/// Check only the compact derivation dependency topology. Publication does not replay producers.
pub fn admit_derivations<'a>(derivations:impl IntoIterator<Item=(AssertionId,&'a [AssertionId])>)->Result<(),ModelError> {
    let mut graph:petgraph::graphmap::DiGraphMap<AssertionId,()>=petgraph::graphmap::DiGraphMap::new();
    for (assertion,premises) in derivations {graph.add_node(assertion);for premise in premises {graph.add_edge(assertion,*premise,());}}
    petgraph::algo::toposort(&graph,None).map(|_|()).map_err(|_|invalid("cyclic derivation premises"))
}

impl From<super::flow::FlowUseSupport> for SupportValue{fn from(row:super::flow::FlowUseSupport)->Self{Self::UseSupport(row)}}
impl From<super::flow::FlowDefinitionSupport> for SupportValue{fn from(row:super::flow::FlowDefinitionSupport)->Self{Self::DefinitionSupport(row)}}
impl From<super::flow::FlowReachingSupport> for SupportValue{fn from(row:super::flow::FlowReachingSupport)->Self{Self::ReachingSupport(row)}}
impl From<super::flow::FlowValueSupport> for SupportValue{fn from(row:super::flow::FlowValueSupport)->Self{Self::ValueSupport(row)}}
impl From<super::flow::FlowRegionSupport> for SupportValue{fn from(row:super::flow::FlowRegionSupport)->Self{Self::RegionSupport(row)}}
impl From<super::flow::FlowTestSupport> for SupportValue{fn from(row:super::flow::FlowTestSupport)->Self{Self::TestSupport(row)}}
impl From<super::flow::FlowTestLeafSupport> for SupportValue{fn from(row:super::flow::FlowTestLeafSupport)->Self{Self::LeafSupport(row)}}
impl From<super::calls::SignatureSupport> for SupportValue{fn from(row:super::calls::SignatureSupport)->Self{Self::SignatureSupport(row)}}
impl From<super::calls::CallTargetSupport> for SupportValue{fn from(row:super::calls::CallTargetSupport)->Self{Self::CallTargetSupport(row)}}
impl From<super::calls::ProviderCallSiteSupport> for SupportValue{fn from(row:super::calls::ProviderCallSiteSupport)->Self{Self::ProviderCallSiteSupport(row)}}
impl From<super::calls::CallSyntaxSupport> for SupportValue{fn from(row:super::calls::CallSyntaxSupport)->Self{Self::CallSyntaxSupport(row)}}
impl From<super::calls::CallResolutionSupport> for SupportValue{fn from(row:super::calls::CallResolutionSupport)->Self{Self::CallResolutionSupport(row)}}
impl From<super::declarations::SymbolDeclarationSupport> for SupportValue{fn from(row:super::declarations::SymbolDeclarationSupport)->Self{Self::SymbolDeclarationSupport(row)}}
impl From<super::declarations::ParameterDeclarationSupport> for SupportValue{fn from(row:super::declarations::ParameterDeclarationSupport)->Self{Self::ParameterDeclarationSupport(row)}}
impl From<super::deployment::TaskReportSupport> for SupportValue{fn from(row:super::deployment::TaskReportSupport)->Self{Self::TaskReportObservationSupport(row)}}
impl From<super::deployment::DeploymentSupport> for SupportValue{fn from(row:super::deployment::DeploymentSupport)->Self{Self::DeploymentObservationSupport(row)}}
impl From<super::documents::DocumentSupport> for SupportValue{fn from(row:super::documents::DocumentSupport)->Self{Self::DocumentObservationSupport(row)}}
impl From<super::documents::PassageSupport> for SupportValue{fn from(row:super::documents::PassageSupport)->Self{Self::PassageObservationSupport(row)}}
impl From<super::documents::CodeBlockSupport> for SupportValue{fn from(row:super::documents::CodeBlockSupport)->Self{Self::CodeBlockObservationSupport(row)}}
impl From<super::documents::DocumentLinkSupport> for SupportValue{fn from(row:super::documents::DocumentLinkSupport)->Self{Self::DocumentLinkObservationSupport(row)}}
impl From<super::documents::DocumentMentionSupport> for SupportValue{fn from(row:super::documents::DocumentMentionSupport)->Self{Self::DocumentMentionObservationSupport(row)}}
impl From<super::documents::DocumentComponentSupport> for SupportValue{fn from(row:super::documents::DocumentComponentSupport)->Self{Self::DocumentComponentObservationSupport(row)}}
impl From<super::documents::DocumentAttributeSupport> for SupportValue{fn from(row:super::documents::DocumentAttributeSupport)->Self{Self::DocumentAttributeObservationSupport(row)}}
impl From<super::flow::FlowAttributeLoadSupport> for SupportValue{fn from(row:super::flow::FlowAttributeLoadSupport)->Self{Self::FlowAttributeLoadObservationSupport(row)}}
impl From<super::flow::FlowValuePathSupport> for SupportValue{fn from(row:super::flow::FlowValuePathSupport)->Self{Self::FlowValuePathObservationSupport(row)}}
impl From<super::lexical::LexicalScopeSupport> for SupportValue{fn from(row:super::lexical::LexicalScopeSupport)->Self{Self::LexicalScopeObservationSupport(row)}}
impl From<super::lexical::BindingSupport> for SupportValue{fn from(row:super::lexical::BindingSupport)->Self{Self::BindingObservationSupport(row)}}
impl From<super::lexical::ReferenceSupport> for SupportValue{fn from(row:super::lexical::ReferenceSupport)->Self{Self::ReferenceObservationSupport(row)}}
impl From<super::lexical::LexicalResolutionSupport> for SupportValue{fn from(row:super::lexical::LexicalResolutionSupport)->Self{Self::LexicalResolutionSupport(row)}}
impl From<super::source::SyntaxSupport> for SupportValue{fn from(row:super::source::SyntaxSupport)->Self{Self::SyntaxObservationSupport(row)}}
impl From<super::symbols::SymbolSupport> for SupportValue{fn from(row:super::symbols::SymbolSupport)->Self{Self::SymbolObservationSupport(row)}}
impl From<super::symbols::FunctionTraitSupport> for SupportValue{fn from(row:super::symbols::FunctionTraitSupport)->Self{Self::FunctionTraitObservationSupport(row)}}
impl From<super::symbols::ClassTraitSupport> for SupportValue{fn from(row:super::symbols::ClassTraitSupport)->Self{Self::ClassTraitObservationSupport(row)}}
impl From<super::symbols::ClassAncestrySupport> for SupportValue{fn from(row:super::symbols::ClassAncestrySupport)->Self{Self::ClassAncestryObservationSupport(row)}}
impl From<super::symbols::ParameterAnnotationSupport> for SupportValue{fn from(row:super::symbols::ParameterAnnotationSupport)->Self{Self::ParameterAnnotationObservationSupport(row)}}
impl From<super::symbols::PublicNameSupport> for SupportValue{fn from(row:super::symbols::PublicNameSupport)->Self{Self::PublicNameObservationSupport(row)}}
impl From<super::symbols::ParameterDocSupport> for SupportValue{fn from(row:super::symbols::ParameterDocSupport)->Self{Self::ParameterDocObservationSupport(row)}}
impl From<super::syntax::SyntaxPlacementSupport> for SupportValue{fn from(row:super::syntax::SyntaxPlacementSupport)->Self{Self::SyntaxPlacementSupport(row)}}
impl From<super::syntax::SyntaxDetailSupport> for SupportValue{fn from(row:super::syntax::SyntaxDetailSupport)->Self{Self::SyntaxDetailObservationSupport(row)}}
impl From<super::syntax::DeclarationSupport> for SupportValue{fn from(row:super::syntax::DeclarationSupport)->Self{Self::DeclarationObservationSupport(row)}}
impl From<super::syntax::DeclarationDecoratorSupport> for SupportValue{fn from(row:super::syntax::DeclarationDecoratorSupport)->Self{Self::DeclarationDecoratorSupport(row)}}
impl From<super::syntax::ImportAliasSupport> for SupportValue{fn from(row:super::syntax::ImportAliasSupport)->Self{Self::ImportAliasObservationSupport(row)}}
impl From<super::syntax::DunderAllSupport> for SupportValue{fn from(row:super::syntax::DunderAllSupport)->Self{Self::DunderAllObservationSupport(row)}}
impl From<super::syntax::ParameterSyntaxSupport> for SupportValue{fn from(row:super::syntax::ParameterSyntaxSupport)->Self{Self::ParameterSyntaxObservationSupport(row)}}
impl From<super::syntax::ClassFieldSyntaxSupport> for SupportValue{fn from(row:super::syntax::ClassFieldSyntaxSupport)->Self{Self::ClassFieldSyntaxObservationSupport(row)}}
impl From<super::types::TypeSupport> for SupportValue{fn from(row:super::types::TypeSupport)->Self{Self::TypeObservationSupport(row)}}
impl From<super::types::TypePresentationSupport> for SupportValue{fn from(row:super::types::TypePresentationSupport)->Self{Self::TypePresentationSupport(row)}}
impl From<super::types::TypeRestrictionSupport> for SupportValue{fn from(row:super::types::TypeRestrictionSupport)->Self{Self::TypeVariableRestrictionSupport(row)}}
impl From<super::types::FunctionBodySupport> for SupportValue{fn from(row:super::types::FunctionBodySupport)->Self{Self::FunctionBodyObservationSupport(row)}}
impl From<super::types::RecordFieldSupport> for SupportValue{fn from(row:super::types::RecordFieldSupport)->Self{Self::RecordFieldObservationSupport(row)}}
impl From<super::class_metadata::ClassMemberSupport> for SupportValue{fn from(row:super::class_metadata::ClassMemberSupport)->Self{Self::ClassMemberObservationSupport(row)}}
impl From<super::class_metadata::ClassMetadataSupport> for SupportValue{fn from(row:super::class_metadata::ClassMetadataSupport)->Self{Self::ClassMetadataObservationSupport(row)}}
impl From<super::calls::SignatureEnumerationSupport> for SupportValue{fn from(row:super::calls::SignatureEnumerationSupport)->Self{Self::SignatureEnumerationObservationSupport(row)}}
impl From<super::ruff::RuffContextSupport> for SupportValue{fn from(row:super::ruff::RuffContextSupport)->Self{Self::RuffContextObservationSupport(row)}}
impl From<super::flow::FlowNarrowingSupport> for SupportValue{fn from(row:super::flow::FlowNarrowingSupport)->Self{Self::FlowNarrowingSupport(row)}}
impl From<super::protocols::NativeExitSupport> for SupportValue{fn from(row:super::protocols::NativeExitSupport)->Self{Self::NativeExitSupport(row)}}
impl From<super::protocols::NativeTerminalSupport> for SupportValue{fn from(row:super::protocols::NativeTerminalSupport)->Self{Self::NativeTerminalSupport(row)}}
impl From<super::captures::CaptureSupport> for SupportValue{fn from(row:super::captures::CaptureSupport)->Self{Self::CaptureSupport(row)}}
impl From<super::protocols::NativeExitDiagnosticSupport> for SupportValue{fn from(row:super::protocols::NativeExitDiagnosticSupport)->Self{Self::NativeExitDiagnosticSupport(row)}}
impl From<super::flow::FlowSourceViewSupport> for SupportValue{fn from(row:super::flow::FlowSourceViewSupport)->Self{Self::FlowSourceViewSupport(row)}}
impl From<super::types::NativeSignatureSupport> for SupportValue{fn from(row:super::types::NativeSignatureSupport)->Self{Self::NativeSignatureObservationSupport(row)}}
impl From<super::types::SignatureTypeSupport> for SupportValue{fn from(row:super::types::SignatureTypeSupport)->Self{Self::SignatureTypeObservationSupport(row)}}
impl From<super::types::TypeQuerySupport> for SupportValue{fn from(row:super::types::TypeQuerySupport)->Self{Self::TypeQuerySupport(row)}}
impl From<super::types::GenericSpecializationSupport> for SupportValue{fn from(row:super::types::GenericSpecializationSupport)->Self{Self::GenericSpecializationSupport(row)}}
impl From<super::flow_capture::FlowCaptureTimingSupport> for SupportValue{fn from(row:super::flow_capture::FlowCaptureTimingSupport)->Self{Self::FlowCaptureTimingSupport(row)}}
impl From<super::symbols::ExportEnumerationSupport> for SupportValue{fn from(row:super::symbols::ExportEnumerationSupport)->Self{Self::ExportEnumerationSupport(row)}}
impl From<super::ruff::RuffBindingSupport> for SupportValue{fn from(row:super::ruff::RuffBindingSupport)->Self{Self::RuffBindingObservationSupport(row)}}
impl From<super::diagnostics::RuffDiagnosticSupport> for SupportValue{fn from(row:super::diagnostics::RuffDiagnosticSupport)->Self{Self::RuffDiagnosticObservationSupport(row)}}
impl From<super::diagnostics::PyreflyDiagnosticSupport> for SupportValue{fn from(row:super::diagnostics::PyreflyDiagnosticSupport)->Self{Self::PyreflyDiagnosticObservationSupport(row)}}
impl From<super::diagnostics::NativeParameterDefinitionSupport> for SupportValue{fn from(row:super::diagnostics::NativeParameterDefinitionSupport)->Self{Self::NativeParameterDefinitionObservationSupport(row)}}
impl From<super::ruff::RuffDefinitionSupport> for SupportValue{fn from(row:super::ruff::RuffDefinitionSupport)->Self{Self::RuffDefinitionObservationSupport(row)}}
impl From<super::flow_inventory::FlowUseInventorySupport> for SupportValue{fn from(row:super::flow_inventory::FlowUseInventorySupport)->Self{Self::FlowUseInventorySupport(row)}}
impl From<super::types::NativeOverloadSupport> for SupportValue{fn from(row:super::types::NativeOverloadSupport)->Self{Self::NativeOverloadSupport(row)}}
impl From<super::types::NativeOverloadCandidateSupport> for SupportValue{fn from(row:super::types::NativeOverloadCandidateSupport)->Self{Self::NativeOverloadCandidateSupport(row)}}
impl From<super::symbols::ModuleResolutionSupport> for SupportValue{fn from(row:super::symbols::ModuleResolutionSupport)->Self{Self::ModuleResolutionObservationSupport(row)}}

#[macro_export]
macro_rules! graph_entity_records{($apply:ident)=>{$apply!{
    ProviderSurface:$crate::domain::assertion::ProviderSurface,
    Package:$crate::domain::input::Package,
    Release:$crate::domain::input::Release,
    Capture:$crate::domain::input::InputRevision,
    AcquisitionOrigin:$crate::domain::input::InputOrigin,
    Source:$crate::domain::source::SourceArtifact,
    Module:$crate::domain::source::Module,
    Occurrence:$crate::domain::source::Occurrence,
    Provider:$crate::domain::attribution::Provider,
    Context:$crate::domain::attribution::AnalysisContext,
    Run:$crate::domain::attribution::ProviderRun,
    Scope:$crate::domain::source::CoverageScope,
    Declaration:$crate::domain::normalized::entities::CallableEntity,
    ClassDeclaration:$crate::domain::normalized::entities::ClassEntity,
    Parameter:$crate::domain::normalized::entities::ParameterEntity,
    Field:$crate::domain::normalized::entities::FieldEntity,
    Exposure:$crate::domain::normalized::entities::PublicExposure,
    InvocationVariant:$crate::domain::normalized::callables::SignatureVariant,
    NativeModule:$crate::domain::calls::ProviderModule,
    NativeSymbol:$crate::domain::calls::ProviderSymbol,
    NativeCallable:$crate::domain::calls::ProviderCallable,
    SignatureParameter:$crate::domain::calls::SignatureParameter,
    ParameterShape:$crate::domain::calls::ParameterShape,
    Type:$crate::domain::types::TypeTerm,
    TypeVariable:$crate::domain::types::TypeVariable,
    TypeSequence:$crate::domain::types::TypeSequence,
    CallableParameterList:$crate::domain::types::CallableParameterList,
    CallableParameter:$crate::domain::types::CallableParameter,
    TypedDictFieldList:$crate::domain::types::TypedDictFieldList,
    TypedDictField:$crate::domain::types::TypedDictField,
    Literal:$crate::domain::value::Literal,
    PlaceRoot:$crate::domain::value::PlaceRoot,
    PathSegment:$crate::domain::value::PathSegment,
    AccessPath:$crate::domain::value::AccessPath,
    Place:$crate::domain::value::Place,
    Predicate:$crate::domain::value::Predicate,
    LiteralSet:$crate::domain::value::LiteralSet,
    EvaluationAtom:$crate::domain::conditions::EvaluationAtom,
    Condition:$crate::domain::conditions::Condition,
    ConditionNode:$crate::domain::conditions::ConditionNode,
    AssumptionUniverse:$crate::domain::assumptions::AssumptionUniverse,
    Assumption:$crate::domain::assumptions::Assumption,
    AssumptionSet:$crate::domain::assumptions::AssumptionSet,
    Qualification:$crate::domain::assertion::AssertionQualification,
    Evidence:$crate::domain::assertion::Evidence,
    CatalogMember:$crate::domain::catalog::CatalogMember,
    CatalogOption:$crate::domain::catalog::CatalogOption,
    Scenario:$crate::domain::catalog::evidence::CatalogScenario,
    Original:$crate::domain::catalog::evidence::OriginalSource,
    RetrievalText:$crate::domain::retrieval::CorpusText,
    RetrievalUnit:$crate::domain::retrieval::Unit,
    RetrievalOrigin:$crate::domain::retrieval::Origin,
    AnalysisDefinition:$crate::domain::analysis::AnalysisDefinition,
    EmbeddingSpecification:$crate::domain::embedding::EmbeddingSpec,
}};}
impl From<super::flow::FlowUseObservation> for Assertion{fn from(row:super::flow::FlowUseObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowDefinitionObservation> for Assertion{fn from(row:super::flow::FlowDefinitionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowReachingObservation> for Assertion{fn from(row:super::flow::FlowReachingObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowValueObservation> for Assertion{fn from(row:super::flow::FlowValueObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowRegionObservation> for Assertion{fn from(row:super::flow::FlowRegionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowTestObservation> for Assertion{fn from(row:super::flow::FlowTestObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowTestLeafObservation> for Assertion{fn from(row:super::flow::FlowTestLeafObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::Signature> for Assertion{fn from(row:super::calls::Signature)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::CallTarget> for Assertion{fn from(row:super::calls::CallTarget)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::ProviderCallSite> for Assertion{fn from(row:super::calls::ProviderCallSite)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::CallSyntax> for Assertion{fn from(row:super::calls::CallSyntax)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::CallResolution> for Assertion{fn from(row:super::calls::CallResolution)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::declarations::SymbolDeclaration> for Assertion{fn from(row:super::declarations::SymbolDeclaration)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::declarations::ParameterDeclaration> for Assertion{fn from(row:super::declarations::ParameterDeclaration)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::deployment::TaskReportObservation> for Assertion{fn from(row:super::deployment::TaskReportObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::deployment::DeploymentObservation> for Assertion{fn from(row:super::deployment::DeploymentObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::DocumentObservation> for Assertion{fn from(row:super::documents::DocumentObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::PassageObservation> for Assertion{fn from(row:super::documents::PassageObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::CodeBlockObservation> for Assertion{fn from(row:super::documents::CodeBlockObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::DocumentLinkObservation> for Assertion{fn from(row:super::documents::DocumentLinkObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::DocumentMentionObservation> for Assertion{fn from(row:super::documents::DocumentMentionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::DocumentComponentObservation> for Assertion{fn from(row:super::documents::DocumentComponentObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::documents::DocumentAttributeObservation> for Assertion{fn from(row:super::documents::DocumentAttributeObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowAttributeLoadObservation> for Assertion{fn from(row:super::flow::FlowAttributeLoadObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowValuePathObservation> for Assertion{fn from(row:super::flow::FlowValuePathObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::lexical::LexicalScopeObservation> for Assertion{fn from(row:super::lexical::LexicalScopeObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::lexical::BindingObservation> for Assertion{fn from(row:super::lexical::BindingObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::lexical::ReferenceObservation> for Assertion{fn from(row:super::lexical::ReferenceObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::lexical::LexicalResolution> for Assertion{fn from(row:super::lexical::LexicalResolution)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::source::SyntaxObservation> for Assertion{fn from(row:super::source::SyntaxObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::SymbolObservation> for Assertion{fn from(row:super::symbols::SymbolObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::FunctionTraitObservation> for Assertion{fn from(row:super::symbols::FunctionTraitObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ClassTraitObservation> for Assertion{fn from(row:super::symbols::ClassTraitObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ClassAncestryObservation> for Assertion{fn from(row:super::symbols::ClassAncestryObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ParameterAnnotationObservation> for Assertion{fn from(row:super::symbols::ParameterAnnotationObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::PublicNameObservation> for Assertion{fn from(row:super::symbols::PublicNameObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ParameterDocObservation> for Assertion{fn from(row:super::symbols::ParameterDocObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::SyntaxPlacement> for Assertion{fn from(row:super::syntax::SyntaxPlacement)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::SyntaxDetailObservation> for Assertion{fn from(row:super::syntax::SyntaxDetailObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::DeclarationObservation> for Assertion{fn from(row:super::syntax::DeclarationObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::DeclarationDecorator> for Assertion{fn from(row:super::syntax::DeclarationDecorator)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::ImportAliasObservation> for Assertion{fn from(row:super::syntax::ImportAliasObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::DunderAllObservation> for Assertion{fn from(row:super::syntax::DunderAllObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::ParameterSyntaxObservation> for Assertion{fn from(row:super::syntax::ParameterSyntaxObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::syntax::ClassFieldSyntaxObservation> for Assertion{fn from(row:super::syntax::ClassFieldSyntaxObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::TypeObservation> for Assertion{fn from(row:super::types::TypeObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::TypePresentation> for Assertion{fn from(row:super::types::TypePresentation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::TypeVariableRestriction> for Assertion{fn from(row:super::types::TypeVariableRestriction)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::FunctionBodyObservation> for Assertion{fn from(row:super::types::FunctionBodyObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::RecordFieldObservation> for Assertion{fn from(row:super::types::RecordFieldObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::class_metadata::ClassMemberObservation> for Assertion{fn from(row:super::class_metadata::ClassMemberObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::class_metadata::ClassMetadataObservation> for Assertion{fn from(row:super::class_metadata::ClassMetadataObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::calls::SignatureEnumerationObservation> for Assertion{fn from(row:super::calls::SignatureEnumerationObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::ruff::RuffContextObservation> for Assertion{fn from(row:super::ruff::RuffContextObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowNarrowingObservation> for Assertion{fn from(row:super::flow::FlowNarrowingObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::protocols::NativeExitObservation> for Assertion{fn from(row:super::protocols::NativeExitObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::protocols::NativeTerminalObservation> for Assertion{fn from(row:super::protocols::NativeTerminalObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::captures::CaptureObservation> for Assertion{fn from(row:super::captures::CaptureObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::protocols::NativeExitDiagnostic> for Assertion{fn from(row:super::protocols::NativeExitDiagnostic)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow::FlowSourceViewObservation> for Assertion{fn from(row:super::flow::FlowSourceViewObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::NativeSignatureObservation> for Assertion{fn from(row:super::types::NativeSignatureObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::SignatureTypeObservation> for Assertion{fn from(row:super::types::SignatureTypeObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::TypeQueryObservation> for Assertion{fn from(row:super::types::TypeQueryObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::GenericSpecializationObservation> for Assertion{fn from(row:super::types::GenericSpecializationObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow_capture::FlowCaptureTimingObservation> for Assertion{fn from(row:super::flow_capture::FlowCaptureTimingObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ExportEnumerationObservation> for Assertion{fn from(row:super::symbols::ExportEnumerationObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::ruff::RuffBindingObservation> for Assertion{fn from(row:super::ruff::RuffBindingObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::diagnostics::RuffDiagnosticObservation> for Assertion{fn from(row:super::diagnostics::RuffDiagnosticObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::diagnostics::PyreflyDiagnosticObservation> for Assertion{fn from(row:super::diagnostics::PyreflyDiagnosticObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::diagnostics::NativeParameterDefinitionObservation> for Assertion{fn from(row:super::diagnostics::NativeParameterDefinitionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::ruff::RuffDefinitionObservation> for Assertion{fn from(row:super::ruff::RuffDefinitionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::flow_inventory::FlowUseInventoryObservation> for Assertion{fn from(row:super::flow_inventory::FlowUseInventoryObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::NativeOverloadObservation> for Assertion{fn from(row:super::types::NativeOverloadObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::types::NativeOverloadCandidate> for Assertion{fn from(row:super::types::NativeOverloadCandidate)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::symbols::ModuleResolutionObservation> for Assertion{fn from(row:super::symbols::ModuleResolutionObservation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::NativeObservation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Native(row.into()),derivation:None}}}
impl From<super::local_semantics::LocalAssessment> for Assertion{fn from(row:super::local_semantics::LocalAssessment)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::local_semantics::LocalContribution> for Assertion{fn from(row:super::local_semantics::LocalContribution)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::local_semantics::LocalGuardContribution> for Assertion{fn from(row:super::local_semantics::LocalGuardContribution)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::local_theory::TheoryWitness> for Assertion{fn from(row:super::local_theory::TheoryWitness)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::records::ExpressionEvaluation> for Assertion{fn from(row:super::execution::records::ExpressionEvaluation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::source_call_records::SourceCallHeader> for Assertion{fn from(row:super::execution::source_call_records::SourceCallHeader)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::source_call_records::SourceInvocation> for Assertion{fn from(row:super::execution::source_call_records::SourceInvocation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::model_transfer::ModelTransferWitness> for Assertion{fn from(row:super::execution::model_transfer::ModelTransferWitness)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::summary_consequences::SummaryClaim> for Assertion{fn from(row:super::execution::summary_consequences::SummaryClaim)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::summary_path::SummaryPathWitness> for Assertion{fn from(row:super::execution::summary_path::SummaryPathWitness)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::execution::summary_production::SummaryRun> for Assertion{fn from(row:super::execution::summary_production::SummaryRun)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::normalized::entities::SymbolEntityResolution> for Assertion{fn from(row:super::normalized::entities::SymbolEntityResolution)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::normalized::entities::PublicEnumerationAssessment> for Assertion{fn from(row:super::normalized::entities::PublicEnumerationAssessment)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::normalized::bindings::CallBindingAttempt> for Assertion{fn from(row:super::normalized::bindings::CallBindingAttempt)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::normalized::bindings::CallBinding> for Assertion{fn from(row:super::normalized::bindings::CallBinding)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::normalized::bindings::BindingVariantAssessment> for Assertion{fn from(row:super::normalized::bindings::BindingVariantAssessment)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::CatalogInvocation> for Assertion{fn from(row:super::catalog::CatalogInvocation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::CatalogPath> for Assertion{fn from(row:super::catalog::CatalogPath)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::CatalogAlias> for Assertion{fn from(row:super::catalog::CatalogAlias)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::CatalogConstructor> for Assertion{fn from(row:super::catalog::CatalogConstructor)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::evidence::ScenarioAssociation> for Assertion{fn from(row:super::catalog::evidence::ScenarioAssociation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::evidence::DocumentAssociation> for Assertion{fn from(row:super::catalog::evidence::DocumentAssociation)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::evidence::CatalogDeployment> for Assertion{fn from(row:super::catalog::evidence::CatalogDeployment)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::catalog::evidence::ScenarioCheck> for Assertion{fn from(row:super::catalog::evidence::ScenarioCheck)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::selection::SelectionDomain> for Assertion{fn from(row:super::selection::SelectionDomain)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::structural::Conclusion> for Assertion{fn from(row:super::structural::Conclusion)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::structural::UsageScore> for Assertion{fn from(row:super::structural::UsageScore)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::structural::controls::ControlTraversal> for Assertion{fn from(row:super::structural::controls::ControlTraversal)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::Conclusion> for Assertion{fn from(row:super::analytics::Conclusion)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::TechniqueResult> for Assertion{fn from(row:super::analytics::TechniqueResult)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::RankScore> for Assertion{fn from(row:super::analytics::RankScore)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::Community> for Assertion{fn from(row:super::analytics::Community)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::Neighbour> for Assertion{fn from(row:super::analytics::Neighbour)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::Concept> for Assertion{fn from(row:super::analytics::Concept)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::analytics::Implication> for Assertion{fn from(row:super::analytics::Implication)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::synthesis::documentary::DocumentaryConclusion> for Assertion{fn from(row:super::synthesis::documentary::DocumentaryConclusion)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::synthesis::assertions::ProgrammaticAssertion> for Assertion{fn from(row:super::synthesis::assertions::ProgrammaticAssertion)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::synthesis::briefs::Brief> for Assertion{fn from(row:super::synthesis::briefs::Brief)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::retrieval::OriginalAnchor> for Assertion{fn from(row:super::retrieval::OriginalAnchor)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::retrieval::UnitSubject> for Assertion{fn from(row:super::retrieval::UnitSubject)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::DerivedConclusion,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Analysis(row.into()),derivation:None}}}
impl From<super::flow::FlowUseSupport> for Assertion{fn from(row:super::flow::FlowUseSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowDefinitionSupport> for Assertion{fn from(row:super::flow::FlowDefinitionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowReachingSupport> for Assertion{fn from(row:super::flow::FlowReachingSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowValueSupport> for Assertion{fn from(row:super::flow::FlowValueSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowRegionSupport> for Assertion{fn from(row:super::flow::FlowRegionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowTestSupport> for Assertion{fn from(row:super::flow::FlowTestSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowTestLeafSupport> for Assertion{fn from(row:super::flow::FlowTestLeafSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::SignatureSupport> for Assertion{fn from(row:super::calls::SignatureSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::CallTargetSupport> for Assertion{fn from(row:super::calls::CallTargetSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::ProviderCallSiteSupport> for Assertion{fn from(row:super::calls::ProviderCallSiteSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::CallSyntaxSupport> for Assertion{fn from(row:super::calls::CallSyntaxSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::CallResolutionSupport> for Assertion{fn from(row:super::calls::CallResolutionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::declarations::SymbolDeclarationSupport> for Assertion{fn from(row:super::declarations::SymbolDeclarationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::declarations::ParameterDeclarationSupport> for Assertion{fn from(row:super::declarations::ParameterDeclarationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::deployment::TaskReportSupport> for Assertion{fn from(row:super::deployment::TaskReportSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::deployment::DeploymentSupport> for Assertion{fn from(row:super::deployment::DeploymentSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::DocumentSupport> for Assertion{fn from(row:super::documents::DocumentSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::PassageSupport> for Assertion{fn from(row:super::documents::PassageSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::CodeBlockSupport> for Assertion{fn from(row:super::documents::CodeBlockSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::DocumentLinkSupport> for Assertion{fn from(row:super::documents::DocumentLinkSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::DocumentMentionSupport> for Assertion{fn from(row:super::documents::DocumentMentionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::DocumentComponentSupport> for Assertion{fn from(row:super::documents::DocumentComponentSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::documents::DocumentAttributeSupport> for Assertion{fn from(row:super::documents::DocumentAttributeSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowAttributeLoadSupport> for Assertion{fn from(row:super::flow::FlowAttributeLoadSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowValuePathSupport> for Assertion{fn from(row:super::flow::FlowValuePathSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::lexical::LexicalScopeSupport> for Assertion{fn from(row:super::lexical::LexicalScopeSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::lexical::BindingSupport> for Assertion{fn from(row:super::lexical::BindingSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::lexical::ReferenceSupport> for Assertion{fn from(row:super::lexical::ReferenceSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::lexical::LexicalResolutionSupport> for Assertion{fn from(row:super::lexical::LexicalResolutionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::source::SyntaxSupport> for Assertion{fn from(row:super::source::SyntaxSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::SymbolSupport> for Assertion{fn from(row:super::symbols::SymbolSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::FunctionTraitSupport> for Assertion{fn from(row:super::symbols::FunctionTraitSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ClassTraitSupport> for Assertion{fn from(row:super::symbols::ClassTraitSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ClassAncestrySupport> for Assertion{fn from(row:super::symbols::ClassAncestrySupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ParameterAnnotationSupport> for Assertion{fn from(row:super::symbols::ParameterAnnotationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::PublicNameSupport> for Assertion{fn from(row:super::symbols::PublicNameSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ParameterDocSupport> for Assertion{fn from(row:super::symbols::ParameterDocSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::SyntaxPlacementSupport> for Assertion{fn from(row:super::syntax::SyntaxPlacementSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::SyntaxDetailSupport> for Assertion{fn from(row:super::syntax::SyntaxDetailSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::DeclarationSupport> for Assertion{fn from(row:super::syntax::DeclarationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::DeclarationDecoratorSupport> for Assertion{fn from(row:super::syntax::DeclarationDecoratorSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::ImportAliasSupport> for Assertion{fn from(row:super::syntax::ImportAliasSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::DunderAllSupport> for Assertion{fn from(row:super::syntax::DunderAllSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::ParameterSyntaxSupport> for Assertion{fn from(row:super::syntax::ParameterSyntaxSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::syntax::ClassFieldSyntaxSupport> for Assertion{fn from(row:super::syntax::ClassFieldSyntaxSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::TypeSupport> for Assertion{fn from(row:super::types::TypeSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::TypePresentationSupport> for Assertion{fn from(row:super::types::TypePresentationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::TypeRestrictionSupport> for Assertion{fn from(row:super::types::TypeRestrictionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::FunctionBodySupport> for Assertion{fn from(row:super::types::FunctionBodySupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::RecordFieldSupport> for Assertion{fn from(row:super::types::RecordFieldSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::class_metadata::ClassMemberSupport> for Assertion{fn from(row:super::class_metadata::ClassMemberSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::class_metadata::ClassMetadataSupport> for Assertion{fn from(row:super::class_metadata::ClassMetadataSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::calls::SignatureEnumerationSupport> for Assertion{fn from(row:super::calls::SignatureEnumerationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::ruff::RuffContextSupport> for Assertion{fn from(row:super::ruff::RuffContextSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowNarrowingSupport> for Assertion{fn from(row:super::flow::FlowNarrowingSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::protocols::NativeExitSupport> for Assertion{fn from(row:super::protocols::NativeExitSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::protocols::NativeTerminalSupport> for Assertion{fn from(row:super::protocols::NativeTerminalSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::captures::CaptureSupport> for Assertion{fn from(row:super::captures::CaptureSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::protocols::NativeExitDiagnosticSupport> for Assertion{fn from(row:super::protocols::NativeExitDiagnosticSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow::FlowSourceViewSupport> for Assertion{fn from(row:super::flow::FlowSourceViewSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::NativeSignatureSupport> for Assertion{fn from(row:super::types::NativeSignatureSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::SignatureTypeSupport> for Assertion{fn from(row:super::types::SignatureTypeSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::TypeQuerySupport> for Assertion{fn from(row:super::types::TypeQuerySupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::GenericSpecializationSupport> for Assertion{fn from(row:super::types::GenericSpecializationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow_capture::FlowCaptureTimingSupport> for Assertion{fn from(row:super::flow_capture::FlowCaptureTimingSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ExportEnumerationSupport> for Assertion{fn from(row:super::symbols::ExportEnumerationSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::ruff::RuffBindingSupport> for Assertion{fn from(row:super::ruff::RuffBindingSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::diagnostics::RuffDiagnosticSupport> for Assertion{fn from(row:super::diagnostics::RuffDiagnosticSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::diagnostics::PyreflyDiagnosticSupport> for Assertion{fn from(row:super::diagnostics::PyreflyDiagnosticSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::diagnostics::NativeParameterDefinitionSupport> for Assertion{fn from(row:super::diagnostics::NativeParameterDefinitionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::ruff::RuffDefinitionSupport> for Assertion{fn from(row:super::ruff::RuffDefinitionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::flow_inventory::FlowUseInventorySupport> for Assertion{fn from(row:super::flow_inventory::FlowUseInventorySupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::NativeOverloadSupport> for Assertion{fn from(row:super::types::NativeOverloadSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::types::NativeOverloadCandidateSupport> for Assertion{fn from(row:super::types::NativeOverloadCandidateSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}
impl From<super::symbols::ModuleResolutionSupport> for Assertion{fn from(row:super::symbols::ModuleResolutionSupport)->Self{let source=SemanticKey::of(row.id());let qualification=row.references().into_iter().find(|reference|reference.target==super::assertion::AssertionQualification::NAME).map(|reference|Qualification::Ref(entity_key(EntityKind::Qualification,reference.target,&reference.key))).unwrap_or(Qualification::Payload);Self{source:Some(source),kind:AssertionKind::EvidenceAssociation,participants:vec![],qualification,run:None,evidence:vec![],value:AssertionValue::Support(row.into()),derivation:None}}}

#[macro_export]
macro_rules! graph_assertion_records{($apply:ident)=>{$apply!{
    Use:$crate::domain::flow::FlowUseObservation,
    Definition:$crate::domain::flow::FlowDefinitionObservation,
    Reaching:$crate::domain::flow::FlowReachingObservation,
    Value:$crate::domain::flow::FlowValueObservation,
    Region:$crate::domain::flow::FlowRegionObservation,
    Test:$crate::domain::flow::FlowTestObservation,
    Leaf:$crate::domain::flow::FlowTestLeafObservation,
    Signature:$crate::domain::calls::Signature,
    CallTarget:$crate::domain::calls::CallTarget,
    ProviderCallSite:$crate::domain::calls::ProviderCallSite,
    CallSyntax:$crate::domain::calls::CallSyntax,
    CallResolution:$crate::domain::calls::CallResolution,
    SymbolDeclaration:$crate::domain::declarations::SymbolDeclaration,
    ParameterDeclaration:$crate::domain::declarations::ParameterDeclaration,
    TaskReportObservation:$crate::domain::deployment::TaskReportObservation,
    DeploymentObservation:$crate::domain::deployment::DeploymentObservation,
    DocumentObservation:$crate::domain::documents::DocumentObservation,
    PassageObservation:$crate::domain::documents::PassageObservation,
    CodeBlockObservation:$crate::domain::documents::CodeBlockObservation,
    DocumentLinkObservation:$crate::domain::documents::DocumentLinkObservation,
    DocumentMentionObservation:$crate::domain::documents::DocumentMentionObservation,
    DocumentComponentObservation:$crate::domain::documents::DocumentComponentObservation,
    DocumentAttributeObservation:$crate::domain::documents::DocumentAttributeObservation,
    FlowAttributeLoadObservation:$crate::domain::flow::FlowAttributeLoadObservation,
    FlowValuePathObservation:$crate::domain::flow::FlowValuePathObservation,
    LexicalScopeObservation:$crate::domain::lexical::LexicalScopeObservation,
    BindingObservation:$crate::domain::lexical::BindingObservation,
    ReferenceObservation:$crate::domain::lexical::ReferenceObservation,
    LexicalResolution:$crate::domain::lexical::LexicalResolution,
    SyntaxObservation:$crate::domain::source::SyntaxObservation,
    SymbolObservation:$crate::domain::symbols::SymbolObservation,
    FunctionTraitObservation:$crate::domain::symbols::FunctionTraitObservation,
    ClassTraitObservation:$crate::domain::symbols::ClassTraitObservation,
    ClassAncestryObservation:$crate::domain::symbols::ClassAncestryObservation,
    ParameterAnnotationObservation:$crate::domain::symbols::ParameterAnnotationObservation,
    PublicNameObservation:$crate::domain::symbols::PublicNameObservation,
    ParameterDocObservation:$crate::domain::symbols::ParameterDocObservation,
    SyntaxPlacement:$crate::domain::syntax::SyntaxPlacement,
    SyntaxDetailObservation:$crate::domain::syntax::SyntaxDetailObservation,
    DeclarationObservation:$crate::domain::syntax::DeclarationObservation,
    DeclarationDecorator:$crate::domain::syntax::DeclarationDecorator,
    ImportAliasObservation:$crate::domain::syntax::ImportAliasObservation,
    DunderAllObservation:$crate::domain::syntax::DunderAllObservation,
    ParameterSyntaxObservation:$crate::domain::syntax::ParameterSyntaxObservation,
    ClassFieldSyntaxObservation:$crate::domain::syntax::ClassFieldSyntaxObservation,
    TypeObservation:$crate::domain::types::TypeObservation,
    TypePresentation:$crate::domain::types::TypePresentation,
    TypeVariableRestriction:$crate::domain::types::TypeVariableRestriction,
    FunctionBodyObservation:$crate::domain::types::FunctionBodyObservation,
    RecordFieldObservation:$crate::domain::types::RecordFieldObservation,
    ClassMemberObservation:$crate::domain::class_metadata::ClassMemberObservation,
    ClassMetadataObservation:$crate::domain::class_metadata::ClassMetadataObservation,
    SignatureEnumerationObservation:$crate::domain::calls::SignatureEnumerationObservation,
    RuffContextObservation:$crate::domain::ruff::RuffContextObservation,
    FlowNarrowing:$crate::domain::flow::FlowNarrowingObservation,
    NativeExit:$crate::domain::protocols::NativeExitObservation,
    NativeTerminal:$crate::domain::protocols::NativeTerminalObservation,
    Capture:$crate::domain::captures::CaptureObservation,
    NativeExitDiagnostic:$crate::domain::protocols::NativeExitDiagnostic,
    FlowSourceView:$crate::domain::flow::FlowSourceViewObservation,
    NativeSignatureObservation:$crate::domain::types::NativeSignatureObservation,
    SignatureTypeObservation:$crate::domain::types::SignatureTypeObservation,
    TypeQuery:$crate::domain::types::TypeQueryObservation,
    GenericSpecialization:$crate::domain::types::GenericSpecializationObservation,
    FlowCaptureTiming:$crate::domain::flow_capture::FlowCaptureTimingObservation,
    ExportEnumeration:$crate::domain::symbols::ExportEnumerationObservation,
    RuffBindingObservation:$crate::domain::ruff::RuffBindingObservation,
    RuffDiagnosticObservation:$crate::domain::diagnostics::RuffDiagnosticObservation,
    PyreflyDiagnosticObservation:$crate::domain::diagnostics::PyreflyDiagnosticObservation,
    NativeParameterDefinitionObservation:$crate::domain::diagnostics::NativeParameterDefinitionObservation,
    RuffDefinitionObservation:$crate::domain::ruff::RuffDefinitionObservation,
    FlowUseInventory:$crate::domain::flow_inventory::FlowUseInventoryObservation,
    NativeOverload:$crate::domain::types::NativeOverloadObservation,
    NativeOverloadCandidate:$crate::domain::types::NativeOverloadCandidate,
    ModuleResolutionObservation:$crate::domain::symbols::ModuleResolutionObservation,
    LocalAssessment:$crate::domain::local_semantics::LocalAssessment,
    LocalTransfer:$crate::domain::local_semantics::LocalContribution,
    LocalGuard:$crate::domain::local_semantics::LocalGuardContribution,
    Theory:$crate::domain::local_theory::TheoryWitness,
    ExpressionEvaluation:$crate::domain::execution::records::ExpressionEvaluation,
    SourceCall:$crate::domain::execution::source_call_records::SourceCallHeader,
    SourceInvocation:$crate::domain::execution::source_call_records::SourceInvocation,
    ModelTransfer:$crate::domain::execution::model_transfer::ModelTransferWitness,
    SummaryClaim:$crate::domain::execution::summary_consequences::SummaryClaim,
    SummaryPath:$crate::domain::execution::summary_path::SummaryPathWitness,
    SummaryRun:$crate::domain::execution::summary_production::SummaryRun,
    NormalizedSymbol:$crate::domain::normalized::entities::SymbolEntityResolution,
    PublicEnumeration:$crate::domain::normalized::entities::PublicEnumerationAssessment,
    BindingAttempt:$crate::domain::normalized::bindings::CallBindingAttempt,
    Binding:$crate::domain::normalized::bindings::CallBinding,
    BindingVariant:$crate::domain::normalized::bindings::BindingVariantAssessment,
    CatalogInvocation:$crate::domain::catalog::CatalogInvocation,
    CatalogPath:$crate::domain::catalog::CatalogPath,
    CatalogAlias:$crate::domain::catalog::CatalogAlias,
    CatalogConstructor:$crate::domain::catalog::CatalogConstructor,
    ScenarioAssociation:$crate::domain::catalog::evidence::ScenarioAssociation,
    DocumentAssociation:$crate::domain::catalog::evidence::DocumentAssociation,
    Deployment:$crate::domain::catalog::evidence::CatalogDeployment,
    ScenarioCheck:$crate::domain::catalog::evidence::ScenarioCheck,
    SelectionDomain:$crate::domain::selection::SelectionDomain,
    StructuralConclusion:$crate::domain::structural::Conclusion,
    StructuralUsage:$crate::domain::structural::UsageScore,
    StructuralControl:$crate::domain::structural::controls::ControlTraversal,
    AnalyticConclusion:$crate::domain::analytics::Conclusion,
    AnalyticTechnique:$crate::domain::analytics::TechniqueResult,
    Rank:$crate::domain::analytics::RankScore,
    Community:$crate::domain::analytics::Community,
    Neighbour:$crate::domain::analytics::Neighbour,
    Concept:$crate::domain::analytics::Concept,
    Implication:$crate::domain::analytics::Implication,
    DocumentaryConclusion:$crate::domain::synthesis::documentary::DocumentaryConclusion,
    ProgrammaticAssertion:$crate::domain::synthesis::assertions::ProgrammaticAssertion,
    Brief:$crate::domain::synthesis::briefs::Brief,
    RetrievalAnchor:$crate::domain::retrieval::OriginalAnchor,
    RetrievalOccurrence:$crate::domain::retrieval::UnitSubject,
    UseSupport:$crate::domain::flow::FlowUseSupport,
    DefinitionSupport:$crate::domain::flow::FlowDefinitionSupport,
    ReachingSupport:$crate::domain::flow::FlowReachingSupport,
    ValueSupport:$crate::domain::flow::FlowValueSupport,
    RegionSupport:$crate::domain::flow::FlowRegionSupport,
    TestSupport:$crate::domain::flow::FlowTestSupport,
    LeafSupport:$crate::domain::flow::FlowTestLeafSupport,
    SignatureSupport:$crate::domain::calls::SignatureSupport,
    CallTargetSupport:$crate::domain::calls::CallTargetSupport,
    ProviderCallSiteSupport:$crate::domain::calls::ProviderCallSiteSupport,
    CallSyntaxSupport:$crate::domain::calls::CallSyntaxSupport,
    CallResolutionSupport:$crate::domain::calls::CallResolutionSupport,
    SymbolDeclarationSupport:$crate::domain::declarations::SymbolDeclarationSupport,
    ParameterDeclarationSupport:$crate::domain::declarations::ParameterDeclarationSupport,
    TaskReportObservationSupport:$crate::domain::deployment::TaskReportSupport,
    DeploymentObservationSupport:$crate::domain::deployment::DeploymentSupport,
    DocumentObservationSupport:$crate::domain::documents::DocumentSupport,
    PassageObservationSupport:$crate::domain::documents::PassageSupport,
    CodeBlockObservationSupport:$crate::domain::documents::CodeBlockSupport,
    DocumentLinkObservationSupport:$crate::domain::documents::DocumentLinkSupport,
    DocumentMentionObservationSupport:$crate::domain::documents::DocumentMentionSupport,
    DocumentComponentObservationSupport:$crate::domain::documents::DocumentComponentSupport,
    DocumentAttributeObservationSupport:$crate::domain::documents::DocumentAttributeSupport,
    FlowAttributeLoadObservationSupport:$crate::domain::flow::FlowAttributeLoadSupport,
    FlowValuePathObservationSupport:$crate::domain::flow::FlowValuePathSupport,
    LexicalScopeObservationSupport:$crate::domain::lexical::LexicalScopeSupport,
    BindingObservationSupport:$crate::domain::lexical::BindingSupport,
    ReferenceObservationSupport:$crate::domain::lexical::ReferenceSupport,
    LexicalResolutionSupport:$crate::domain::lexical::LexicalResolutionSupport,
    SyntaxObservationSupport:$crate::domain::source::SyntaxSupport,
    SymbolObservationSupport:$crate::domain::symbols::SymbolSupport,
    FunctionTraitObservationSupport:$crate::domain::symbols::FunctionTraitSupport,
    ClassTraitObservationSupport:$crate::domain::symbols::ClassTraitSupport,
    ClassAncestryObservationSupport:$crate::domain::symbols::ClassAncestrySupport,
    ParameterAnnotationObservationSupport:$crate::domain::symbols::ParameterAnnotationSupport,
    PublicNameObservationSupport:$crate::domain::symbols::PublicNameSupport,
    ParameterDocObservationSupport:$crate::domain::symbols::ParameterDocSupport,
    SyntaxPlacementSupport:$crate::domain::syntax::SyntaxPlacementSupport,
    SyntaxDetailObservationSupport:$crate::domain::syntax::SyntaxDetailSupport,
    DeclarationObservationSupport:$crate::domain::syntax::DeclarationSupport,
    DeclarationDecoratorSupport:$crate::domain::syntax::DeclarationDecoratorSupport,
    ImportAliasObservationSupport:$crate::domain::syntax::ImportAliasSupport,
    DunderAllObservationSupport:$crate::domain::syntax::DunderAllSupport,
    ParameterSyntaxObservationSupport:$crate::domain::syntax::ParameterSyntaxSupport,
    ClassFieldSyntaxObservationSupport:$crate::domain::syntax::ClassFieldSyntaxSupport,
    TypeObservationSupport:$crate::domain::types::TypeSupport,
    TypePresentationSupport:$crate::domain::types::TypePresentationSupport,
    TypeVariableRestrictionSupport:$crate::domain::types::TypeRestrictionSupport,
    FunctionBodyObservationSupport:$crate::domain::types::FunctionBodySupport,
    RecordFieldObservationSupport:$crate::domain::types::RecordFieldSupport,
    ClassMemberObservationSupport:$crate::domain::class_metadata::ClassMemberSupport,
    ClassMetadataObservationSupport:$crate::domain::class_metadata::ClassMetadataSupport,
    SignatureEnumerationObservationSupport:$crate::domain::calls::SignatureEnumerationSupport,
    RuffContextObservationSupport:$crate::domain::ruff::RuffContextSupport,
    FlowNarrowingSupport:$crate::domain::flow::FlowNarrowingSupport,
    NativeExitSupport:$crate::domain::protocols::NativeExitSupport,
    NativeTerminalSupport:$crate::domain::protocols::NativeTerminalSupport,
    CaptureSupport:$crate::domain::captures::CaptureSupport,
    NativeExitDiagnosticSupport:$crate::domain::protocols::NativeExitDiagnosticSupport,
    FlowSourceViewSupport:$crate::domain::flow::FlowSourceViewSupport,
    NativeSignatureObservationSupport:$crate::domain::types::NativeSignatureSupport,
    SignatureTypeObservationSupport:$crate::domain::types::SignatureTypeSupport,
    TypeQuerySupport:$crate::domain::types::TypeQuerySupport,
    GenericSpecializationSupport:$crate::domain::types::GenericSpecializationSupport,
    FlowCaptureTimingSupport:$crate::domain::flow_capture::FlowCaptureTimingSupport,
    ExportEnumerationSupport:$crate::domain::symbols::ExportEnumerationSupport,
    RuffBindingObservationSupport:$crate::domain::ruff::RuffBindingSupport,
    RuffDiagnosticObservationSupport:$crate::domain::diagnostics::RuffDiagnosticSupport,
    PyreflyDiagnosticObservationSupport:$crate::domain::diagnostics::PyreflyDiagnosticSupport,
    NativeParameterDefinitionObservationSupport:$crate::domain::diagnostics::NativeParameterDefinitionSupport,
    RuffDefinitionObservationSupport:$crate::domain::ruff::RuffDefinitionSupport,
    FlowUseInventorySupport:$crate::domain::flow_inventory::FlowUseInventorySupport,
    NativeOverloadSupport:$crate::domain::types::NativeOverloadSupport,
    NativeOverloadCandidateSupport:$crate::domain::types::NativeOverloadCandidateSupport,
    ModuleResolutionObservationSupport:$crate::domain::symbols::ModuleResolutionSupport,
}};}
