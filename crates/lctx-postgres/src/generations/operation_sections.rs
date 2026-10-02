//! Optional operation sections hydrate only selected finite metadata and admitted originals.
use super::{CatalogService,Error,RequestExecution};
use super::catalog_service::{name,retain,wire_error};
use lctx_model::domain::{*,catalog::{self,evidence::*},normalized::{events::*,entities::*},serving::*,serving::identity::policy_identity};
use std::collections::{BTreeSet,BTreeMap};
fn absent<T>()->SectionPage<T>{SectionPage{availability:Availability::NotRequested{},items:vec![],continuation:Optional::default(),omitted:0,truncated:false}}
fn need<R:Record>(batch:&Batch<R>,id:Id<R>)->Result<R,Error>{batch.rows().iter().find(|r|r.id()==id).cloned().ok_or(Error::Contract)}
fn borrowed<R:Record>(batch:&Batch<R>,id:Id<R>)->Result<&R,Error>{batch.rows().iter().find(|r|r.id()==id).ok_or(Error::Contract)}
fn ids<R:Record>(rows:&[R])->Vec<Id<R>>{rows.iter().map(Record::id).collect::<BTreeSet<_>>().into_iter().collect()}
fn peek(token:&CursorToken)->Result<Cursor,Error>{let text=token.as_str();if !text.len().is_multiple_of(2) || !text.bytes().all(|byte| byte.is_ascii_hexdigit()){return Err(Error::Codec("cursor encoding".into()));}let bytes=text.as_bytes().as_chunks::<2>().0.iter().map(|c|std::str::from_utf8(c).ok().and_then(|s|u8::from_str_radix(s,16).ok()).ok_or_else(||Error::Codec("cursor encoding".into()))).collect::<Result<Vec<_>,_>>()?;serde_json::from_slice(&bytes).map_err(|e|Error::Codec(e.to_string()))}
fn label(section:OperationSection)->&'static str{match section{OperationSection::Scenarios=>"scenarios",OperationSection::Deployment=>"deployment",OperationSection::Relationships=>"relationships",OperationSection::Conflicts=>"conflicts",OperationSection::Briefs=>"briefs",OperationSection::Behavior=>"behavior"}}
impl CatalogService {
    pub(super) fn validate_section_cursor(&self,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<(),Error>{if let Some(token)=&r.page.cursor.0{let cursor=peek(token)?;if cursor.binding.member!=Some(member)||cursor.binding.group.as_str()!="operation"||!r.sections.iter().any(|s|label(*s)==cursor.binding.section.as_str()){return Err(Error::Codec("operation continuation section/member".into()));}self.section_page(r,member,cursor.binding.section.as_str(),100,Vec::<()>::new(),false)?;}Ok(())}
    fn section_binding(&self,r:&GetOperationRequest,member:Id<catalog::CatalogMember>,section:&str)->Result<CursorBinding,Error>{
        Ok(CursorBinding{generation:self.generation(),request:Request::GetOperation(r.clone()).canonical_identity().map_err(wire_error)?,policy:policy_identity(&("operation-sections/v1","positive-resolved-scenario/original-only",2u32,"invocation-policy-admitted-direct-target",5u32,"canonical-local-kind-invocation-signature-conflicts","summary-qualified-facet-original-condition-model",16usize,4096usize,"whole-row-mcp-fit/scenarios-deployment-relationships-conflicts-briefs-behavior"))?,wire:self.wire(),channels:ChannelState{lexical:false,vector:VectorChannel::Disabled{}}.identity(),group:name("operation")?,section:name(section)?,member:Some(member),ordering:ContentHash::of(b"canonical-section-identity-order/v1")})
    }
    // The cardinality-independent binding is validated before any section body is hydrated.
    fn section_page<T>(&self,r:&GetOperationRequest,member:Id<catalog::CatalogMember>,section:&str,maximum:u32,rows:Vec<T>,check_offset:bool)->Result<SectionPage<T>,Error>{
        if r.page.size==0||r.page.size>ResourceLimits::default().maximum_page_rows{return Err(Error::ResourceRefused("page rows"));}
        let binding=self.section_binding(r,member,section)?;
        let offset=if let Some(token)=&r.page.cursor.0{let cursor=peek(token)?;if cursor.binding.section.as_str()==section{Cursor::decode(token,&binding).map_err(wire_error)?.offset}else{0}}else{0};
        let offset=usize::try_from(offset).map_err(|_|Error::Codec("cursor offset".into()))?;if check_offset&&offset>rows.len(){return Err(Error::Codec("cursor offset outside complete section".into()));}
        let total=rows.len();let end=offset.saturating_add(r.page.size.min(maximum) as usize).min(total);let continuation=if end<total{Optional(Some(Cursor{binding,offset:end as u64}.encode().map_err(wire_error)?))}else{Optional::default()};
        Ok(SectionPage{availability:Availability::Available{},items:rows.into_iter().skip(offset).take(end.saturating_sub(offset)).collect(),continuation,omitted:total.saturating_sub(end) as u64,truncated:end<total})
    }
    pub(super) async fn optional_sections(&self,e:&RequestExecution,r:&GetOperationRequest,core:OperationCore)->Result<OperationPacket,Error>{
        self.validate_section_cursor(r,core.member)?;let mut packet=OperationPacket{core,scenarios:absent(),deployment:absent(),relationships:absent(),conflicts:absent(),briefs:absent(),behavior:absent()};
        let mut requested=Vec::new();for section in &r.sections{if !requested.contains(section){requested.push(*section);}}for section in &requested{match section{OperationSection::Scenarios=>packet.scenarios=self.scenario_section(e,r,packet.core.member).await?,OperationSection::Deployment=>packet.deployment=self.deployment_section(e,r,packet.core.member,packet.core.release.release).await?,OperationSection::Relationships=>packet.relationships=self.relationship_section(e,r,packet.core.member).await?,OperationSection::Conflicts=>packet.conflicts=self.conflict_section(e,r,packet.core.member).await?,OperationSection::Briefs=>packet.briefs=self.brief_section(e,r,packet.core.member).await?,OperationSection::Behavior=>packet.behavior=self.behavior_section(e,r,packet.core.member).await?}}
        retain(e,&packet)?;Ok(packet)
    }
    async fn scenario_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<ScenarioPacket>,Error>{
        let this=self.clone();let request=r.clone();let retained=e.clone();let page=e.cpu(move|b|{let d=this.prepared().data();let _temporary=b.reserve("scenario-section-metadata",d.evidence.associations.len()*size_of::<ScenarioAssociation>()*2)?;let mut rows=d.evidence.associations.iter().filter(|a|a.member==member&&a.basis==AssociationBasis::ResolvedTarget&&matches!(a.intent,Intent::Demonstration|Intent::AssertionTest)).cloned().collect::<Vec<_>>();rows.sort_by_key(|a|(a.scenario,a.id()));rows.dedup_by_key(|a|a.scenario);let page=this.section_page(&request,member,"scenarios",2,rows,true)?;retained.retain("scenario-section-metadata",size_of::<SectionPage<ScenarioAssociation>>()+page.items.iter().map(|row|size_of::<ScenarioAssociation>()+row.heap_bytes()).sum::<usize>())?;Ok(page)}).await?;
        let scenario_ids=page.items.iter().map(|a|a.scenario).collect::<Vec<_>>();let spans=e.query(move|lease|Box::pin(async move{lease.read_for::<ScenarioSpan,CatalogScenario>("scenario",&scenario_ids).await})).await?;
        let _packet_charge=e.budget().reserve("scenario-section-dtos",spans.rows().len()*size_of::<OriginalRange>()*3+page.items.len()*size_of::<ScenarioPacket>()*2)?;let mut items=Vec::new();for association in &page.items{let scenario=self.prepared().data().evidence.scenarios.get(association.scenario).ok_or(Error::Contract)?;let context=self.prepared().data().source.core.qualifications.get(association.qualification).ok_or(Error::Contract)?.context;
            let mut original_spans=spans.rows().iter().filter(|s|s.scenario==scenario.id()&&s.role!=SpanRole::ExtractedPython).collect::<Vec<_>>();original_spans.sort_by_key(|s|(s.ordinal,s.id()));if original_spans.is_empty(){return Err(Error::Contract);}let mut originals=Vec::new();for span in original_spans{originals.push(self.service().original_range(e,OriginalReference::Catalog{source:span.source},Some(context)).await?);}
            items.push(ScenarioPacket{scenario:scenario.id(),intent:association.intent,basis:association.basis,spans:originals,parse:scenario.parse,binding:scenario.binding,environment:scenario.environment,execution:scenario.execution});}
        let result=SectionPage{availability:page.availability,items,continuation:page.continuation,omitted:page.omitted,truncated:page.truncated};retain(e,&result)?;Ok(result)
    }
    async fn deployment_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>,release:Id<input::Release>)->Result<SectionPage<DeploymentPacket>,Error>{
        let links=e.query(move|lease|Box::pin(async move{lease.read_for::<ReleaseDeployment,input::Release>("release",&[release]).await})).await?;let this=self.clone();let request=r.clone();let retained=e.clone();let page=e.cpu(move|b|{let _ids=b.reserve("deployment-section-id-sets",links.rows().len().saturating_mul(128))?;let mut rows=links.rows().iter().map(|r|r.deployment).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();rows.sort();let page=this.section_page(&request,member,"deployment",100,rows,true)?;retain(&retained,&page)?;Ok(page)}).await?;
        let packet_bytes=page.items.iter().try_fold(0usize,|bytes,id|{let catalog=self.prepared().data().evidence.deployments.get(*id).ok_or(Error::Contract)?;let row=self.prepared().data().source.facts.deployment.get(catalog.observation).ok_or(Error::Contract)?;Ok::<_,Error>(bytes.saturating_add(2*size_of::<DeploymentPacket>()).saturating_add(row.heap_bytes().saturating_mul(4)))} )?;
        let _packet_charge=e.budget().reserve("deployment-section-dtos",packet_bytes)?;let mut items=Vec::new();for id in &page.items{let catalog=self.prepared().data().evidence.deployments.get(*id).ok_or(Error::Contract)?;let row=self.prepared().data().source.facts.deployment.get(catalog.observation).ok_or(Error::Contract)?;let field=name(row.field.clone())?;
            let context=self.prepared().data().source.core.qualifications.get(row.qualification).ok_or(Error::Contract)?.context;let original=self.service().original_range(e,OriginalReference::Span{span:row.span},Some(context)).await?;
            items.push(DeploymentPacket{deployment:*id,field,name:name(row.name.as_deref().unwrap_or(&row.field))?,value:Text::new(row.original.clone()).map_err(wire_error)?,originals:vec![original]});}
        let result=SectionPage{availability:page.availability,items,continuation:page.continuation,omitted:page.omitted,truncated:page.truncated};retain(e,&result)?;Ok(result)
    }
    async fn relationship_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<RelationshipPacket>,Error>{
        let d=self.prepared().data();let _owner_charge=e.budget().reserve("relationship-owner-sets",d.source.catalog.callables.len()*64+d.source.core.ownership.len()*64)?;let callable_ids=d.source.catalog.callables.iter().filter(|c|c.member==member).map(|c|d.source.core.assessments.get(c.assessment).map(|a|a.callable).ok_or(Error::Contract)).collect::<Result<BTreeSet<_>,_>>()?;
        let entities=callable_ids.into_iter().map(|callable|EntityRef::Callable{callable}.id()).collect::<BTreeSet<_>>();let owners=d.source.core.ownership.iter().filter(|o|entities.contains(&o.entity)).map(Record::id).collect::<Vec<_>>();
        let (events,policies,admissions,alternatives)=e.query(move|lease|Box::pin(async move{let events=lease.read_for::<NormalizedCallEvent,OccurrenceOwnership>("owner",&owners).await?;let policies=lease.read_for::<CallPolicyAssessment,NormalizedCallEvent>("event",&ids(events.rows())).await?;let chosen=policies.rows().iter().filter(|p|p.policy==CallPolicy::Invocation).map(Record::id).collect::<Vec<_>>();let admissions=lease.read_for::<CallPolicyAdmission,CallPolicyAssessment>("assessment",&chosen).await?;let alternatives=lease.read_ids::<NormalizedCallAlternative>(&admissions.rows().iter().map(|a|a.alternative).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;Ok((events,policies,admissions,alternatives))})).await?;
        let this=self.clone();let request=r.clone();let retained=e.clone();e.cpu(move|b|{let d=this.prepared().data();let mut rows=Vec::new();let mut charge=b.reserve("relationship-section-dtos",0)?;for admission in admissions.rows(){let policy=need(&policies,admission.assessment)?;if policy.policy!=CallPolicy::Invocation{return Err(Error::Contract);}let alternative=need(&alternatives,admission.alternative)?;let event=need(&events,policy.event)?;if alternative.event!=event.id(){return Err(Error::Contract);}let Some(entity)=alternative.entity else{continue;};let mut targets=BTreeSet::new();if let Some(EntityRef::Callable{callable})=d.source.core.refs.get(entity){for c in d.source.catalog.callables.iter(){if d.source.core.assessments.get(c.assessment).is_some_and(|a|a.callable==*callable){targets.insert(c.member);}}}
        if let Some(EntityRef::Class{class})=d.source.core.refs.get(entity){for c in d.source.catalog.classes.iter().filter(|c|c.class==*class){targets.insert(c.member);}}
            for target in targets{charge.try_resize((rows.len()+1)*1024)?;rows.push(RelationshipPacket{target,analysis:event.context,role:selection::RelationRole::Invokes,fidelity:if alternative.status==ResolutionStatus::Resolved{selection::Fidelity::ResolvedTarget}else{selection::Fidelity::CandidateTargets},witnesses:vec![],proof:vec![ProofReference::from_canonical(derivation::RowRef::of(admission.id())),ProofReference::from_canonical(derivation::RowRef::of(policy.id())),ProofReference::from_canonical(derivation::RowRef::of(alternative.id())),ProofReference::from_canonical(derivation::RowRef::of(event.id()))]});}}
        rows.sort_by(|a,b|a.target.cmp(&b.target).then(a.analysis.cmp(&b.analysis)).then(a.proof[0].row.cmp(&b.proof[0].row)));let page=this.section_page(&request,member,"relationships",5,rows,true)?;retain(&retained,&page)?;Ok(page)}).await
    }
    async fn conflict_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<ConflictPacket>,Error>{
        let this=self.clone();let request=r.clone();let retained=e.clone();e.cpu(move|b|{let d=this.prepared().data();let contexts=this.prepared().output().domains.iter().filter(|domain|domain.member==member).map(|domain|domain.analysis).collect::<BTreeSet<_>>();
            let _temporary=b.reserve("local-conflict-projection",d.source.core.slots.len().saturating_mul(512).saturating_add(8192))?;let mut predicates=vec![];for kind in [selection::MemberKind::Function,selection::MemberKind::Method,selection::MemberKind::Class,selection::MemberKind::Property,selection::MemberKind::Module,selection::MemberKind::Variable]{predicates.push(selection::Predicate::MemberKind{kind});}for form in [selection::InvocationForm::Function,selection::InvocationForm::Method,selection::InvocationForm::Class,selection::InvocationForm::Static,selection::InvocationForm::Property,selection::InvocationForm::ContextManager,selection::InvocationForm::AsyncContextManager]{predicates.push(selection::Predicate::InvocationForm{form});}
            let callable_ids=d.source.catalog.callables.iter().filter(|c|c.member==member).map(Record::id).collect::<BTreeSet<_>>();let variants=d.source.catalog.invocations.iter().filter(|i|callable_ids.contains(&i.callable)).map(|i|i.variant).collect::<BTreeSet<_>>();let mut names=BTreeMap::<String,BTreeSet<i16>>::new();for slot in d.source.core.slots.iter().filter(|s|variants.contains(&s.variant)){let parameter=d.facts.signature_parameters.get(slot.parameter).ok_or(Error::Contract)?;let shape=d.facts.shapes.get(parameter.shape).ok_or(Error::Contract)?;if let Some(name)=&shape.name{names.entry(name.as_str().to_owned()).or_default().insert(shape.kind as i16);}}
            for (name,kinds) in names{predicates.push(selection::Predicate::DeclaresParameter{name:name.clone()});predicates.push(selection::Predicate::ParameterRequired{name:name.clone(),required:true});for state in [selection::DefaultState::Absent,selection::DefaultState::LiteralNone,selection::DefaultState::Literal,selection::DefaultState::SourceExpression,selection::DefaultState::FactoryExpression,selection::DefaultState::Unknown,selection::DefaultState::OptionalExpressionUnavailable]{predicates.push(selection::Predicate::ParameterDefaultState{name:name.clone(),state});}for code in kinds{let kind=d.facts.shapes.iter().find(|s|s.kind as i16==code).ok_or(Error::Contract)?.kind;predicates.push(selection::Predicate::ParameterKind{name:name.clone(),kind});}}
            let mut rows=Vec::new();let mut row_charge=b.reserve("local-conflict-dtos",0)?;let mut row_bytes=0usize;for analysis in contexts{for predicate in &predicates{let requirement=selection::Requirement{predicate:predicate.clone(),quantifier:selection::Quantifier::AnyApplicable};let classified=this.prepared().classify(member,analysis,&requirement,b)?;if classified.outcome!=selection::Outcome::Conflicting{continue;}let mut contexts=BTreeSet::new();let mut positive=BTreeSet::new();let mut negative=BTreeSet::new();for witness in &classified.witnesses{use selection::algebra::{ClaimContext,RequirementWitness};let(context,p,n)=match witness{RequirementWitness::Positive{context,evidence,..}=>(context,evidence.as_slice(),&[][..]),RequirementWitness::Negative{context,evidence,..}=>(context,&[][..],evidence.as_slice()),RequirementWitness::Conflict{context,positive,negative,..}=>(context,positive.as_slice(),negative.as_slice())};let ClaimContext::Declaration(context)=context else{return Err(Error::Contract);};contexts.insert(context.id());positive.extend(p.iter().map(Record::id));negative.extend(n.iter().map(Record::id));}row_bytes=row_bytes.saturating_add(size_of::<ConflictPacket>()+requirement.predicate.heap_bytes()+contexts.len()*32+(positive.len()+negative.len())*32);row_charge.try_resize(row_bytes)?;rows.push(ConflictPacket{requirement,reason:classified.reason,contexts:contexts.into_iter().collect(),positive:positive.into_iter().collect(),negative:negative.into_iter().collect()});}}
            rows.sort_by_key(|r|serde_json::to_vec(r).unwrap_or_default());rows.dedup();let page=this.section_page(&request,member,"conflicts",100,rows,true)?;retain(&retained,&page)?;Ok(page)
        }).await
    }
    async fn brief_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<CapabilityPacket>,Error>{
        let briefs=e.query(move|lease|Box::pin(async move{let members=lease.read_for::<catalog::CatalogMemberInvocation,catalog::CatalogMember>("member",&[member]).await?;let seeds=lease.read_for::<synthesis::seeds::SelectedSeed,catalog::CatalogMemberInvocation>("member",&ids(members.rows())).await?;lease.read_for::<synthesis::briefs::Brief,synthesis::seeds::SelectedSeed>("seed",&ids(seeds.rows())).await})).await?;
        let this=self.clone();let request=r.clone();let page=e.cpu(move|_b|{let rows=ids(briefs.rows());this.section_page(&request,member,"briefs",100,rows,true)}).await?;let mut items=Vec::new();for capability in &page.items{let response=self.service().capability(e,&GetCapabilityRequest{capability:*capability,page:PageRequest::default()}).await?;items.push(response.capability);}let result=SectionPage{availability:page.availability,items,continuation:page.continuation,omitted:page.omitted,truncated:page.truncated};retain(e,&result)?;Ok(result)
    }
    async fn behavior_section(&self,e:&RequestExecution,r:&GetOperationRequest,member:Id<catalog::CatalogMember>)->Result<SectionPage<BehaviorPacket>,Error>{
        use execution::summary_consequences::{ClaimConclusion,ClaimProof};
        let input=self.prepared().data().source.catalog.members.get(member).ok_or(Error::Contract)?.input;
        let (facets,outcomes)=e.query(move|lease|Box::pin(async move{
            let members=lease.read_for::<catalog::CatalogMemberInvocation,catalog::CatalogMember>("member",&[member]).await?;
            let facets=lease.read_for::<synthesis::summary::SummaryFacet,catalog::CatalogMemberInvocation>("member",&ids(members.rows())).await?;
            let invocations=lease.read_for::<analysis::summary::AnalysisInvocation,input::InputRevision>("input",&[input]).await?;
            let outcomes=lease.read_for::<analysis::summary::AnalysisOutcome,analysis::summary::AnalysisInvocation>("invocation",&ids(invocations.rows())).await?;
            if outcomes.rows().len()!=invocations.rows().len(){return Err(Error::Contract);}Ok((facets,outcomes))
        })).await?;
        let this=self.clone();let request=r.clone();let retained=e.clone();
        let (page,unavailable)=e.cpu(move|budget|{
            let _temporary=budget.reserve("behavior-section-metadata",std::mem::size_of_val(facets.rows())*2)?;
            let unavailable=facets.rows().iter().filter(|f|f.qualification.is_none()).count() as u64;
            let mut rows=facets.rows().iter().filter(|f|f.qualification.is_some()).cloned().collect::<Vec<_>>();rows.sort_by_key(Record::id);
            let mut page=this.section_page(&request,member,"behavior",100,rows,true)?;
            if !outcomes.rows().is_empty()&&outcomes.rows().iter().all(|o|o.status==analysis::AnalysisStatus::NotRequested){page.availability=Availability::NotRequested{};}
            else if outcomes.rows().is_empty(){page.availability=Availability::Unavailable{reason:name("summary analysis invocation absent")?};}
            else if outcomes.rows().iter().any(|o|o.status!=analysis::AnalysisStatus::Completed){page.availability=Availability::Partial{reason:name("summary analysis boundary is partial or unavailable")?};}
            retained.retain("behavior-selected-facets",size_of::<SectionPage<synthesis::summary::SummaryFacet>>()+page.items.iter().map(|row|size_of::<synthesis::summary::SummaryFacet>()+row.heap_bytes()).sum::<usize>())?;
            Ok((page,unavailable))
        }).await?;
        // Each relation is hydrated once for the exact selected page, not once per facet.
        let facts=e.query(move|lease|Box::pin(async move{
            let id_charge=lease.budget.reserve("behavior-page-id-sets",page.items.len().saturating_mul(2048))?;
            let conclusions=lease.read_ids::<ClaimConclusion>(&page.items.iter().map(|f|f.conclusion).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            for facet in &page.items{let conclusion=borrowed(&conclusions,facet.conclusion)?;if (facet.qualification,facet.verdict)!=(conclusion.qualification,conclusion.verdict){return Err(Error::Contract);}}
            let qualification_ids=conclusions.rows().iter().map(|c|c.qualification.ok_or(Error::Contract)).collect::<Result<BTreeSet<_>,_>>()?.into_iter().collect::<Vec<_>>();
            let qualifications=lease.read_ids::<assertion::AssertionQualification>(&qualification_ids).await?;
            let proofs=lease.read_ids::<ClaimProof>(&conclusions.rows().iter().filter_map(|c|c.proof).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            let invocations=lease.read_ids::<analysis::summary::AnalysisInvocation>(&conclusions.rows().iter().map(|c|c.invocation).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            let definitions=lease.read_ids::<analysis::AnalysisDefinition>(&invocations.rows().iter().map(|i|i.definition).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            let parameters=lease.read_ids::<analysis::MethodParameters>(&definitions.rows().iter().map(|d|d.parameters).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            let model_ids=parameters.rows().iter().map(|p|p.model_catalog.ok_or(Error::Contract)).collect::<Result<BTreeSet<_>,_>>()?.into_iter().collect::<Vec<_>>();
            let models=lease.read_ids::<models::ModelCatalog>(&model_ids).await?;
            let conditions=lease.read_ids::<conditions::Condition>(&qualifications.rows().iter().map(|q|q.condition).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()).await?;
            // Shared roots are read once at each frontier. Each rendered condition retains its
            // existing 4096-node bound; the page union cannot exceed the sum of those bounds.
            let maximum_nodes=page.items.len().saturating_mul(4096);
            let mut nodes=normalized::Rows::new(&lease.budget);let mut node_scratch=lease.budget.reserve("behavior-node-frontiers",0)?;
            let mut frontier=conditions.rows().iter().map(|c|c.root).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
            while !frontier.is_empty(){
                if nodes.len().saturating_add(frontier.len())>maximum_nodes{return Err(Error::ResourceRefused("behavior condition rendering nodes"));}
                node_scratch.try_resize(nodes.len().saturating_add(frontier.len()).saturating_mul(128))?;
                let batch=lease.read_ids::<conditions::ConditionNode>(&frontier).await?;let mut next=BTreeSet::new();
                for id in &frontier{let node=borrowed(&batch,*id)?;if let conditions::ConditionNode::Branch{low,high,..}=node{next.insert(*low);next.insert(*high);}nodes.insert(node.clone())?;}
                frontier=next.into_iter().filter(|id|nodes.get(*id).is_none()).collect();
            }
            Ok((page,conclusions,qualifications,proofs,invocations,definitions,parameters,models,conditions,nodes,id_charge,node_scratch))
        })).await?;
        let retained=e.clone();e.cpu(move|budget|{
            let (page,conclusions,qualifications,proofs,invocations,definitions,parameters,models,conditions,nodes,_id_charge,_node_scratch)=facts;
            let mut items=Vec::new();
            for facet in &page.items{
                let conclusion=borrowed(&conclusions,facet.conclusion)?;
                let qualification=borrowed(&qualifications,conclusion.qualification.ok_or(Error::Contract)?)?;
                if let Some(proof)=conclusion.proof{borrowed(&proofs,proof)?;}
                let invocation=borrowed(&invocations,conclusion.invocation)?;let definition=borrowed(&definitions,invocation.definition)?;
                let parameters=borrowed(&parameters,definition.parameters)?;let model=parameters.model_catalog.ok_or(Error::Contract)?;borrowed(&models,model)?;
                let condition=borrowed(&conditions,qualification.condition)?;
                let _temporary=budget.reserve("behavior-term-render",4096usize.saturating_mul(16*size_of::<RenderedAtom>()+4*size_of::<conditions::ConditionNode>()+128))?;
                let mut selected=BTreeSet::new();let mut pending=vec![condition.root];let mut condition_nodes=Vec::new();
                while let Some(id)=pending.pop(){if !selected.insert(id){continue;}
        if selected.len()>4096{return Err(Error::ResourceRefused("behavior condition rendering nodes"));}
                    let node=nodes.get(id).ok_or(Error::Contract)?;if let conditions::ConditionNode::Branch{low,high,..}=node{pending.push(*low);pending.push(*high);}condition_nodes.push(node.clone());}
                let diagram=conditions::Diagram::from_records(condition,&condition_nodes)?;
                let rendered=diagram.render_terms(16).map_err(|_|Error::ResourceRefused("behavior condition term rendering"))?;
                let mut proof=vec![ProofReference::from_canonical(derivation::RowRef::of(facet.id())),ProofReference::from_canonical(derivation::RowRef::of(conclusion.id()))];if let Some(id)=conclusion.proof{proof.push(ProofReference::from_canonical(derivation::RowRef::of(id)));}
                let packet=BehaviorPacket{condition:condition.id(),verdict:facet.verdict,model,proof,presentation:RenderedConditionPacket::from_canonical(&rendered),presentation_truncated:rendered.truncated};retain(&retained,&packet)?;items.push(packet);
            }
            let availability=if unavailable>0&&!matches!(page.availability,Availability::NotRequested{}){Availability::Partial{reason:name("unqualified summary boundary has no condition/model packet")?}}else{page.availability};
            let result=SectionPage{availability,items,continuation:page.continuation,omitted:page.omitted+unavailable,truncated:page.truncated||unavailable>0};retain(&retained,&result)?;Ok(result)
        }).await
    }
}

impl CatalogService {
    pub(super) async fn fit_operation_response(&self,e:&RequestExecution,r:&GetOperationRequest,response:GetOperationResponse)->Result<GetOperationResponse,Error>{
        let this=self.clone();let request=r.clone();let retained=e.clone();e.cpu(move|budget|{
            // Counting is allocation-free; only an admitted candidate is encoded into MCP scratch.
            let _encoding=budget.reserve("operation-response-encoding",ResourceLimits::default().expanded_response_bytes as usize*12)?;
            let mut response=Response::GetOperation(response);let bound=ResourceLimits::default().response_bytes(request.page.expanded) as usize;
            loop {
                if response.json_len().map_err(wire_error)?<=bound{
                    let raw=response.to_json().map_err(wire_error)?;
                    match tool_result("get_operation",&raw,request.page.expanded){Ok(_)=>{let Response::GetOperation(response)=response else{return Err(Error::Contract);};retain(&retained,&response)?;return Ok(response);},Err(WireError::ResourceRefused(_))=>{},Err(error)=>return Err(wire_error(error)),}
                }
                let Response::GetOperation(inner)=&mut response else{return Err(Error::Contract);};let OperationResolution::Unique{packet}=&mut inner.operation else{return Err(Error::ResourceRefused("indivisible operation resolution"));};let member=packet.core.member;
                // Stable presentation priority preserves positive source examples longest.
                if !packet.behavior.items.is_empty(){this.trim_optional_page(&request,member,"behavior",&mut packet.behavior)?;}
                else if !packet.briefs.items.is_empty(){this.trim_optional_page(&request,member,"briefs",&mut packet.briefs)?;}
                else if !packet.conflicts.items.is_empty(){this.trim_optional_page(&request,member,"conflicts",&mut packet.conflicts)?;}
                else if !packet.relationships.items.is_empty(){this.trim_optional_page(&request,member,"relationships",&mut packet.relationships)?;}
                else if !packet.deployment.items.is_empty(){this.trim_optional_page(&request,member,"deployment",&mut packet.deployment)?;}
                else if !packet.scenarios.items.is_empty(){this.trim_optional_page(&request,member,"scenarios",&mut packet.scenarios)?;}
                else{return Err(Error::ResourceRefused("indivisible mandatory operation packet"));}
            }
        }).await
    }
    fn trim_optional_page<T>(&self,r:&GetOperationRequest,member:Id<catalog::CatalogMember>,section:&str,page:&mut SectionPage<T>)->Result<(),Error>{
        page.items.pop().ok_or(Error::Contract)?;let binding=self.section_binding(r,member,section)?;
        let offset=match &r.page.cursor.0{Some(token)if peek(token)?.binding.section.as_str()==section=>Cursor::decode(token,&binding).map_err(wire_error)?.offset,_=>0};
        page.continuation=Optional(Some(Cursor{binding,offset:offset.checked_add(page.items.len() as u64).ok_or(Error::Contract)?}.encode().map_err(wire_error)?));
        page.omitted=page.omitted.checked_add(1).ok_or(Error::Contract)?;page.truncated=true;page.availability=Availability::Partial{reason:name("optional page byte bound; use expanded budget or dedicated route")?};Ok(())
    }
}
