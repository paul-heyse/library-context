//! Same-context finite selection results retain every attributed witness.
use lctx_model::domain::{*,serving::*,selection::{algebra::*,evaluate::CandidateSelection}};
use crate::records::wire;
pub fn packet(value:&CandidateSelection,data:&selection::classification::ClassificationData,domains:&[LibraryDomainPacket])->Result<OperationCandidate,ModelError>{
 let member=data.source.catalog.members.get(value.member).ok_or(ModelError::Schema("selected catalog member"))?;
 let releases=domains.iter().flat_map(|d|&d.captures).filter(|c|c.release.input==member.input).map(|c|c.release.clone()).collect::<Vec<_>>();
 if releases.is_empty(){return Err(ModelError::Schema("selected member release admission"))}
 let mut requirements=Vec::new();
 for r in &value.requirements {
  let mut claims=Vec::new();let mut contexts=Vec::new();let mut positive=Vec::new();let mut negative=Vec::new();
  for witness in &r.witnesses {
   let (context,basis,pos,neg)=match witness {
    RequirementWitness::Positive{context,basis,evidence}=>(context,*basis,evidence.as_slice(),&[][..]),
    RequirementWitness::Negative{context,basis,evidence}=>(context,*basis,&[][..],evidence.as_slice()),
    RequirementWitness::Conflict{context,basis,positive,negative}=>(context,*basis,positive.as_slice(),negative.as_slice()),
   };
   let context=match context{ClaimContext::Declaration(c)=>c,ClaimContext::Runtime(c)=>&c.declaration};
   let p=pos.iter().map(Record::id).collect::<Vec<_>>();let n=neg.iter().map(Record::id).collect::<Vec<_>>();
   let mut exceptions=Vec::new();
   for evidence in pos.iter().chain(neg) {if let selection::Witness::SummaryException{outcome}=evidence{
    let outcome=data.facts.exception_outcomes.get(*outcome).ok_or(ModelError::Schema("runtime exception outcome"))?;
    let q=data.source.core.qualifications.get(outcome.qualification).ok_or(ModelError::Schema("runtime exception qualification"))?;
    exceptions.push(BehavioralExceptionPacket::from_canonical(outcome,q)?);
   }}
   contexts.push(context.id());positive.extend(p.iter().copied());negative.extend(n.iter().copied());
   claims.push(RequirementWitnessPacket{context:context.id(),basis,positive:p,negative:n,behavioral_exceptions:exceptions});
  }
  contexts.sort();contexts.dedup();positive.sort();positive.dedup();negative.sort();negative.dedup();
  requirements.push(RequirementResult{claims,closure:r.closure.iter().map(Record::id).collect(),requirement:r.requirement.clone(),outcome:r.outcome,reason:r.reason,contexts,positive,negative,corpus_complete:r.corpus_complete,analyzer_complete:r.analyzer_complete,examined:r.examined as u64,total:Nullable(r.total.map(|v|v as u64))});
 }
 let assessments=data.source.catalog.callables.iter().filter(|c|c.member==member.id()).map(|c|data.source.core.assessments.get(c.assessment).ok_or(ModelError::Schema("effective callable assessment"))).collect::<Result<Vec<_>,_>>()?;
 let knowledge=if assessments.iter().any(|a|a.signatures==normalized::callables::Knowledge::Conflicting){normalized::callables::Knowledge::Conflicting}else if !assessments.is_empty() && assessments.iter().all(|a|a.signatures==normalized::callables::Knowledge::Known){normalized::callables::Knowledge::Known}else{normalized::callables::Knowledge::Unknown};
 Ok(OperationCandidate{releases,member:member.id(),analysis:value.analysis,name:Name::new(value.path.join(".")).map_err(wire)?,requirements,joint:value.joint,signature_knowledge:knowledge})
}
pub fn key(candidate:&OperationCandidate)->ContentHash{
 let mut sink=KeySink::new("native-operation-candidate/v1");candidate.member.encode(&mut sink);candidate.analysis.encode(&mut sink);sink.finish()
}
