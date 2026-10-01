//! Static source observations for S0. Each exact source keeps its own execution conditions.
//! The outer qualification describes a source-graph observation, never executable completion.
use super::{*,build::{need,invalid}};
use crate::domain::{*,analysis::{self,structural as owner,policy::{FindingKind,EvidenceStatus},support::SourceFacts},assertion::{AssertionQualification,Approximation},attribution::Modality,conditions::Diagram,normalized::entities::EntityRef,source::CoverageScope};
use crate::{Domain,DomainSum};
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="structural_conclusion_sources")]
pub enum ConclusionSource {
 #[model(code=0)] Public {candidate:Id<PublicCandidate>},
 #[model(code=1)] Path {path:Id<Path>},
 #[model(code=2)] Unresolved {event:Id<UnresolvedEvent>},
 #[model(code=3)] Stop {traversal:Id<Traversal>},
 #[model(code=4)] Usage {score:Id<UsageScore>},
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="structural_conclusions",rule="structural_source_observation")]
pub struct Conclusion {
 #[model(key)] pub frame:Id<StructuralFrame>,
 #[model(key,premise)] pub source:Id<ConclusionSource>,
 pub invocation:Id<owner::Invocation>,
 pub subject:Id<EntityRef>,
 pub kind:FindingKind,
 qualification:Id<AssertionQualification>,
}
impl Conclusion {pub fn qualification(&self)->Id<AssertionQualification>{self.qualification}}
impl analysis::support::sealed::DerivedEvidence for Conclusion {}
impl analysis::support::DerivedEvidence for Conclusion {fn source_facts(&self)->SourceFacts{SourceFacts{qualification:self.qualification,status:EvidenceStatus::StructurallyObserved,heuristic:false}}}
/// This scope is intentionally static. Conditions on each supporting hop remain on PathStep.
pub fn qualification(invocation:&owner::Invocation,modality:Modality,approximation:Approximation)->AssertionQualification{
 AssertionQualification{context:invocation.context,scope:CoverageScope::Input{input:invocation.input}.id(),condition:Diagram::always().id(),modality,approximation}
}
pub(super) fn produce(out:&mut Output,invocation:&owner::Invocation,frame:&StructuralFrame,data:&build::Data,budget:&resources::ResourceBudget)->Result<(),ModelError>{
 let mut observations=charged::ChargedVec::default();let mut charge=charged::StateCharge::new(budget,"structural-conclusions");
 for row in out.public.iter(){observations.push(&mut charge,Candidate{source:ConclusionSource::Public{candidate:row.id()},subject:row.entity,kind:FindingKind::PublicAlias,modality:Modality::Candidate,approximation:Approximation::Over,usage:false})?;}
 for path in out.paths.iter(){let reach=need(&out.reaches,path.reach)?;let traversal=need(&out.traversals,reach.traversal)?;let mut modality=Modality::Definite;let mut approximation=Approximation::Exact;
  for step in out.steps.iter().filter(|s|s.path==path.id()){if let StepEvidence::Call{qualification,modality:m,..}=need(&out.evidence,step.evidence)?{let q=need(&data.projection.qualifications,*qualification)?;if q.context!=invocation.context{return Err(invalid("structural step has a foreign context"));}modality=modality.weakest(*m).weakest(q.modality);approximation=approximation.join(q.approximation);}}
  let kind=match reach.kind{ReachKind::Direct=>FindingKind::DirectDelegation,ReachKind::BoundedPath=>FindingKind::BoundedDelegationPath,_=>FindingKind::ImplementationBoundary};observations.push(&mut charge,Candidate{source:ConclusionSource::Path{path:path.id()},subject:traversal.seed,kind,modality,approximation,usage:false})?;
 }
 for event in out.unresolved.iter(){let t=need(&out.traversals,event.traversal)?;observations.push(&mut charge,Candidate{source:ConclusionSource::Unresolved{event:event.id()},subject:t.seed,kind:FindingKind::IncompleteResolution,modality:Modality::Candidate,approximation:Approximation::Over,usage:false})?;}
 for row in out.traversals.iter().filter(|r|r.stop.is_some()){observations.push(&mut charge,Candidate{source:ConclusionSource::Stop{traversal:row.id()},subject:row.seed,kind:FindingKind::TraversalStop,modality:Modality::Definite,approximation:Approximation::Exact,usage:false})?;}
 for row in out.usage_scores.iter(){observations.push(&mut charge,Candidate{source:ConclusionSource::Usage{score:row.id()},subject:row.target,kind:FindingKind::DirectUsage,modality:Modality::Candidate,approximation:Approximation::Over,usage:true})?;}
 for row in observations.iter(){let q=qualification(invocation,row.modality,row.approximation);out.conclusion_qualifications.insert(q.clone())?;out.conclusion_sources.insert(row.source.clone())?;out.conclusions.insert(Conclusion{frame:frame.id(),source:row.source.id(),invocation:if row.usage{frame.usage_invocation}else{frame.invocation},subject:row.subject,kind:row.kind,qualification:q.id()})?;}Ok(())
}
// A charged temporary inventory avoids retaining an unaccounted vector while the result grows.
struct Candidate {source:ConclusionSource,subject:Id<EntityRef>,kind:FindingKind,modality:Modality,approximation:Approximation,usage:bool}

impl HeapSize for Candidate {}
