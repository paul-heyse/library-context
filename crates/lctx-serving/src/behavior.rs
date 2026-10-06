//! Model-qualified summary packets over one native canonical dependency selection.
use lctx_model::domain::{*,serving::*,resources::ResourceBudget};
use lctx_surrealdb::batches::CanonicalBatches;
use crate::{records::{rows,need,wire},source_evidence::NativePackets};
use std::collections::BTreeSet;
pub async fn packets(source:&CanonicalBatches,member:Id<catalog::CatalogMember>,budget:&ResourceBudget)->Result<(Availability,Vec<BehaviorPacket>),ModelError>{
    use execution::summary_consequences::{ClaimConclusion,ClaimProof};
    let members=rows::<catalog::CatalogMember>(source)?;
    let input=need(&members,member)?.input;
    let links=rows::<catalog::CatalogMemberInvocation>(source)?;
    let links=links.into_iter().filter(|link|link.member==member).map(|link|link.id()).collect::<BTreeSet<_>>();
    let facets=rows::<synthesis::summary::SummaryFacet>(source)?.into_iter().filter(|f|f.member.is_some_and(|member|links.contains(&member))).collect::<Vec<_>>();
    let invocations=rows::<analysis::summary::AnalysisInvocation>(source)?;
    let selected=invocations.iter().filter(|i|i.input==input).map(Record::id).collect::<BTreeSet<_>>();
    let outcomes=rows::<analysis::summary::AnalysisOutcome>(source)?;
    let outcomes=outcomes.iter().filter(|o|selected.contains(&o.invocation)).collect::<Vec<_>>();
    if outcomes.len()!=selected.len(){return Err(ModelError::Schema("summary invocation outcomes"))}
    let unavailable=facets.iter().any(|f|f.qualification.is_none());
    let availability=if outcomes.is_empty(){Availability::Unavailable{reason:Name::new("summary analysis invocation absent").map_err(wire)?}}
        else if outcomes.iter().all(|o|o.status==analysis::AnalysisStatus::NotRequested){Availability::NotRequested{}}
        else if unavailable||outcomes.iter().any(|o|o.status!=analysis::AnalysisStatus::Completed){Availability::Partial{reason:Name::new("summary analysis boundary is partial or unavailable").map_err(wire)?}}
        else{Availability::Available{}};
    let conclusions=rows::<ClaimConclusion>(source)?;
    let qualifications=rows::<assertion::AssertionQualification>(source)?;
    let proofs=rows::<ClaimProof>(source)?;
    let definitions=rows::<analysis::AnalysisDefinition>(source)?;
    let parameters=rows::<analysis::MethodParameters>(source)?;
    let models=rows::<models::ModelCatalog>(source)?;
    let conditions=rows::<conditions::Condition>(source)?;
    let nodes=rows::<conditions::ConditionNode>(source)?;
    let claims=crate::claims::Claims::new(source,budget)?;
    let mut native=NativePackets::new(source,budget)?;
    let mut packets=Vec::new();
    for facet in facets.iter().filter(|facet|facet.qualification.is_some()){
        let conclusion=need(&conclusions,facet.conclusion)?;
        if (facet.qualification,facet.verdict)!=(conclusion.qualification,conclusion.verdict){return Err(ModelError::Conflict("summary facet conclusion"))}
        let q=need(&qualifications,conclusion.qualification.ok_or(ModelError::Schema("qualified summary conclusion"))?)?;
        let invocation=need(&invocations,conclusion.invocation)?;
        if invocation.input!=input||invocation.context!=q.context{return Err(ModelError::Conflict("summary packet input and context"))}
        let definition=need(&definitions,invocation.definition)?;
        let parameters=need(&parameters,definition.parameters)?;
        let model=parameters.model_catalog.ok_or(ModelError::Schema("summary model catalog"))?;need(&models,model)?;
        let basis=claims.basis(q.id())?;
        let captures=if let Some(id)=conclusion.proof{let(value,_charge)=crate::capture::captures(&mut native,need(&proofs,id)?,invocation,q,&basis).await.map_err(ModelError::from)?;value}else{Vec::new()};
        let condition=need(&conditions,q.condition)?;
        let mut visited=BTreeSet::new();let mut pending=vec![condition.root];let mut selected=Vec::new();
        while let Some(id)=pending.pop(){if !visited.insert(id){continue}
            if visited.len()>4096{return Err(ModelError::Invalid("resource_refused: behavior condition rendering nodes".into()))}
            let node=need(&nodes,id)?;if let conditions::ConditionNode::Branch{low,high,..}=node{pending.push(*low);pending.push(*high);}selected.push(node.clone());}
        let diagram=conditions::Diagram::from_records(condition,&selected)?;
        let rendered=diagram.render_terms(16).map_err(|e|ModelError::Invalid(format!("behavior condition rendering: {e:?}")))?;
        let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(facet.id()))?,ProofReference::from_canonical(derivation::RowRef::of(conclusion.id()))?];
        if let Some(id)=conclusion.proof{proof.push(ProofReference::from_canonical(derivation::RowRef::of(id))?);}
        packets.push(BehaviorPacket{captures,claim_basis:basis,condition:condition.id(),verdict:facet.verdict,model,proof,presentation:RenderedConditionPacket::from_canonical(&rendered),presentation_truncated:rendered.truncated});
    }
    Ok((availability,packets))
}
