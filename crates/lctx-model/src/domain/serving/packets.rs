//! Cohesive nested packets over canonical identities, not an arbitrary row-flattening language.
use super::*;
use crate::domain::{self,*};
use schemars::JsonSchema;
use serde::{Deserialize,Serialize};
macro_rules! packet {($name:ident {$($field:ident:$ty:ty),*$(,)?})=>{
    #[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)] pub struct $name {$ (pub $field:$ty,)*}
};}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="status",rename_all="snake_case",deny_unknown_fields)]
pub enum Availability {Available {},Partial {reason:Name},NotRequested {},Unavailable {reason:Name}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionPage<T> {
    pub availability:Availability,pub items:Vec<T>,
    #[serde(default,skip_serializing_if="Optional::is_absent")]pub continuation:Optional<CursorToken>,
    pub omitted:u64,pub truncated:bool,
}
packet!(PacketLimits {maximum_page_rows:u32,maximum_response_bytes:u64,signature_indivisible:bool});
packet!(ReleaseIdentity {input:Id<input::InputRevision>,release:Id<input::Release>,distribution:Name,version:Name});
packet!(AccessProvenance {module:Id<source::Module>,path:Vec<Name>,exposures:Vec<Id<catalog::CatalogExposure>>,candidates:Vec<Id<catalog::CatalogCandidate>>,basis:Nullable<catalog::CatalogContractBasis>});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum DefaultValue {Absent {},Unavailable {},Unknown {},Literal {literal:Id<value::Literal>},Expression {expression:Id<source::Occurrence>},Factory {expression:Id<source::Occurrence>}}
impl DefaultValue {pub fn from_canonical(value:&catalog::CatalogDefault)->Self {match value {
    catalog::CatalogDefault::Absent{}=>Self::Absent {},catalog::CatalogDefault::Unavailable{}=>Self::Unavailable {},
    catalog::CatalogDefault::Unknown{}=>Self::Unknown {},catalog::CatalogDefault::Literal{literal}=>Self::Literal{literal:*literal},
    catalog::CatalogDefault::Expression{expression}=>Self::Expression{expression:*expression},catalog::CatalogDefault::Factory{expression}=>Self::Factory{expression:*expression},
}}}
packet!(ParameterPacket {parameter:Id<calls::SignatureParameter>,slot:Nullable<Id<normalized::callables::SignatureSlot>>,formals:Vec<Id<normalized::entities::ParameterEntity>>,ordinal:i64,name:Nullable<Name>,kind:calls::ParameterKind,required:bool,types:Vec<Id<types::TypeTerm>>,default:DefaultValue});
packet!(SignaturePacket {source:Id<calls::Signature>,variant:Id<normalized::callables::SignatureVariant>,analysis:Id<attribution::AnalysisContext>,form:calls::SignatureForm,adjustment:normalized::callables::SignatureAdjustment,parameters:Vec<ParameterPacket>,effective_parameters:Vec<ParameterPacket>,return_types:Vec<Id<types::TypeTerm>>,complete:bool});
packet!(InvocationPacket {callable:Id<catalog::CatalogCallable>,invocation:Id<catalog::CatalogInvocation>,assessment:Id<normalized::callables::EffectiveCallableAssessment>,analysis:Id<attribution::AnalysisContext>,knowledge:normalized::callables::Knowledge,form:Nullable<selection::InvocationForm>});
packet!(OptionPacket {option:Id<catalog::CatalogOption>,subject:Id<catalog::CatalogOptionSubject>,evidence:Id<catalog::CatalogOptionEvidence>,default:DefaultValue});
packet!(OperationCore {member:Id<catalog::CatalogMember>,name:Name,release:ReleaseIdentity,access:AccessProvenance,invocations:Vec<InvocationPacket>,signatures:Vec<SignaturePacket>,signature_knowledge:normalized::callables::Knowledge,options:Vec<OptionPacket>,literal_values:Vec<LiteralPacket>,type_presentations:Vec<TypePresentationPacket>,limits:PacketLimits});
/// Address an actual stored nominal source; wrappers are not manufactured during serving.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum OriginalReference {
    Catalog{source:Id<catalog::evidence::OriginalSource>},Anchor{anchor:Id<retrieval::OriginalAnchor>},
    Prose{slice:Id<synthesis::documentary::ProseSlice>},Artifact{artifact:Id<source::SourceArtifact>},
    Occurrence{occurrence:Id<source::Occurrence>},Span{span:assertion::EvidenceSourceSpanId},
}
packet!(OriginalRange {source:OriginalReference,artifact:Id<source::SourceArtifact>,start:u64,end:u64,digest:ContentHash,encoding:Name,release:Id<input::Release>,context:Id<attribution::AnalysisContext>});
packet!(ScenarioPacket {scenario:Id<catalog::evidence::CatalogScenario>,intent:catalog::evidence::Intent,basis:catalog::evidence::AssociationBasis,spans:Vec<OriginalRange>,parse:deployment::CheckStatus,binding:deployment::CheckStatus,environment:deployment::CheckStatus,execution:deployment::CheckStatus});
packet!(DeploymentPacket {deployment:Id<catalog::evidence::CatalogDeployment>,field:Name,name:Name,value:Text<0,8192>,originals:Vec<OriginalRange>});
packet!(RelationshipPacket {target:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,role:selection::RelationRole,fidelity:selection::Fidelity,witnesses:Vec<Id<selection::Witness>>,proof:Vec<ProofReference>});
packet!(ConflictPacket {requirement:selection::Requirement,reason:selection::Reason,contexts:Vec<Id<selection::Context>>,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>});
packet!(AssertionSupportPacket {support:Id<synthesis::assertions::ProgrammaticAssertionSupport>,role:analysis::policy::SupportRole,source:Id<synthesis::assertions::AssertionSource>,proof:Vec<ProofReference>});
packet!(AssertionPacket {assertion:Id<synthesis::assertions::ProgrammaticAssertion>,kind:analysis::policy::AssertionKind,section:analysis::policy::BriefSection,status:analysis::policy::EvidenceStatus,qualification:Id<assertion::AssertionQualification>,text:Text<0,262144>,supports:Vec<AssertionSupportPacket>});
packet!(CapabilityPacket {capability:Id<synthesis::briefs::Brief>,title:Name,rendered:Text<0,262144>,assertions:Vec<AssertionPacket>,originals:Vec<OriginalRange>,availability:Availability,unreviewed:bool,documentation_only:bool});
packet!(BehaviorPacket {condition:Id<conditions::Condition>,verdict:obligation::Verdict,model:Id<models::ModelCatalog>,proof:Vec<ProofReference>,presentation:RenderedConditionPacket,presentation_truncated:bool});
packet!(OperationPacket {core:OperationCore,scenarios:SectionPage<ScenarioPacket>,deployment:SectionPage<DeploymentPacket>,relationships:SectionPage<RelationshipPacket>,conflicts:SectionPage<ConflictPacket>,briefs:SectionPage<CapabilityPacket>,behavior:SectionPage<BehaviorPacket>});
packet!(RequirementWitnessPacket {context:Id<selection::Context>,basis:selection::EvidenceBasis,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>});
packet!(RequirementResult {claims:Vec<RequirementWitnessPacket>,closure:Vec<Id<selection::Witness>>,requirement:selection::Requirement,outcome:selection::Outcome,reason:selection::Reason,contexts:Vec<Id<selection::Context>>,positive:Vec<Id<selection::Witness>>,negative:Vec<Id<selection::Witness>>,corpus_complete:bool,analyzer_complete:bool,examined:u64,total:Nullable<u64>});
packet!(OperationCandidate {member:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,name:Name,requirements:Vec<RequirementResult>,joint:selection::JointApplicability,signature_knowledge:normalized::callables::Knowledge});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="extent",rename_all="snake_case",deny_unknown_fields)]
pub enum SelectionExtent {CompleteDomain {total:u64},Ranked {returned:u64},EmptyUnderCoverage {corpus_complete:bool,analyzer_complete:bool}}
packet!(ProofReference {relation:Name,row:[u8;16]});
impl ProofReference {pub fn from_canonical(row:domain::derivation::RowRef)->Self {Self{relation:Name::new(row.relation()).expect("canonical relation name bounded"),row:*row.bytes()}}}
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBodyPage {
    pub start:u64,pub end:u64,pub bytes:Vec<u8>,
    #[serde(default,skip_serializing_if="Optional::is_absent")]pub continuation:Optional<CursorToken>,
    pub omitted:u64,pub truncated:bool,
}
packet!(EvidencePacket {original:OriginalRange,body:EvidenceBodyPage,status:analysis::policy::EvidenceStatus,derivation:SectionPage<DerivationStep>});
packet!(DerivationStep {source:ProofReference,rule:Name,conclusion:ProofReference,premises:Vec<PremisePacket>});
packet!(PremisePacket {role:Name,premise:ProofReference});
packet!(EvidenceHit {unit:Id<retrieval::Unit>,family:retrieval::Family,title:Name,originals:Vec<OriginalRange>,associated_members:Vec<Id<catalog::CatalogMember>>});
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum BrowseEntry {Module {module:Id<source::Module>,name:Name,members:u64},Class {member:Id<catalog::CatalogMember>,name:Name,members:u64},Member {candidate:OperationCandidate,ownership:Availability},Vocabulary {facet:selection::Facet,values:Vec<Name>,members:u64}}
packet!(ComparisonEntry {requested:OperationSelector,candidates:Vec<OperationCandidate>,ambiguous:bool});
packet!(RenderedAtom {atom:Id<conditions::EvaluationAtom>,value:bool});
packet!(RenderedConditionPacket {terms:Vec<Vec<RenderedAtom>>,truncated:bool});
impl RenderedConditionPacket {
    pub fn from_canonical(value:&conditions::RenderedCondition)->Self {
        Self{terms:value.terms.iter().map(|term|term.iter().map(|(atom,value)|RenderedAtom{atom:*atom,value:*value}).collect()).collect(),truncated:value.truncated}
    }
}

/// A named rendering of the canonical literal owner; transient ExactScalar has a narrower domain.
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum LiteralValue {None {},Bool{value:bool},Integer{decimal:String},String{value:String},Bytes{value:Vec<u8>},Float{bits:i64}}
packet!(LiteralPacket {literal:Id<value::Literal>,value:LiteralValue});
impl LiteralPacket {
    pub fn from_canonical(row:&value::Literal)->Result<Self,ModelError>{
        row.validate()?;
        let value=match row {
            value::Literal::None=>LiteralValue::None {},
            value::Literal::Bool{value}=>LiteralValue::Bool{value:*value},
            value::Literal::Integer{decimal}=>LiteralValue::Integer{decimal:decimal.clone()},
            value::Literal::String{value}=>LiteralValue::String{value:value.as_str().to_owned()},
            value::Literal::Bytes{value}=>LiteralValue::Bytes{value:value.0.clone()},
            value::Literal::Float{bits}=>LiteralValue::Float{bits:*bits},
        };Ok(Self{literal:row.id(),value})
    }
}
packet!(TypePresentationPacket {presentation:Id<types::TypePresentation>,term:Id<types::TypeTerm>,qualification:Id<assertion::AssertionQualification>,display:Text<0,262144>,detail:Nullable<Text<0,262144>>});
impl TypePresentationPacket {
    pub fn from_canonical(row:&types::TypePresentation)->Result<Self,WireError>{
        Ok(Self{presentation:row.id(),term:row.term,qualification:row.qualification,
            display:Text::new(row.display.clone())?,detail:Nullable(row.detail.clone().map(Text::new).transpose()?)})
    }
}

impl CapabilityPacket {
    /// Keep canonical authored bytes intact; expose the same attributed claims as the packet.
    pub fn resource_text(&self)->Result<String,WireError>{
        let mut text=self.rendered.as_str().to_owned();text.push_str("\n\n## Assertion evidence\n");
        for assertion in &self.assertions{
            let metadata=serde_json::json!({"assertion":assertion.assertion,"status":assertion.status,"status_name":format!("{:?}",assertion.status),"kind":assertion.kind,"qualification":assertion.qualification,"text":assertion.text,"supports":assertion.supports});
            text.push_str("\n```json\n");text.push_str(&serde_json::to_string(&metadata)?);text.push_str("\n```\n");
        }
        Ok(text)
    }
}
