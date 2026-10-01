//! Public paths derive from normalized identity plus declaration-parent/binding evidence.
use super::{*,build::{CatalogData,CatalogOutput}};
use crate::domain::{resources::ResourceBudget,charged::{ChargedMap,ChargedSet,StateCharge},lexical::{BindingEventKind,LexicalScopeKind},syntax::DeclarationObservation,symbols::{AncestryRelation,Linearization}};
const MAX_DEPTH:usize=32;
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
fn need<R:Record>(rows:&normalized::Rows<R>,id:Id<R>)->Result<&R,ModelError> {rows.get(id).ok_or_else(||invalid("catalog path premise absent"))}
fn class_entity(data:&CatalogData,id:Id<EntityRef>)->Option<Id<ClassEntity>> {match data.refs.get(id) {Some(EntityRef::Class {class})=>Some(*class),_=>None}}
fn boundary(out:&mut CatalogOutput,parent:Id<CatalogCandidate>,reason:PublicPathBoundaryReason,binding:Option<Id<lexical::BindingObservation>>)->Result<(),ModelError> {out.boundaries.insert(CatalogPathBoundary {parent,reason,binding})?;Ok(())}
fn declaration_entity(data:&CatalogData,declaration:&DeclarationObservation)->Option<Id<EntityRef>> {
    let entity=match declaration.kind {
        syntax::DeclarationKind::Class=>data.source_classes.iter().find(|r|matches!(r,ClassEntity::Source {declaration:anchor} if *anchor==declaration.declaration)).map(|r|EntityRef::Class {class:r.id()}),
        _=>data.source_callables.iter().find(|r|matches!(r,CallableEntity::Source {declaration:anchor,..} if *anchor==declaration.declaration)).map(|r|EntityRef::Callable {callable:r.id()}),
    }?;
    data.refs.get(entity.id()).map(Record::id)
}
pub(super) fn expand(data:&CatalogData,out:&mut CatalogOutput,budget:&ResourceBudget)->Result<(),ModelError> {
    let mut charge=StateCharge::new(budget,"catalog-path-expansion");
    // Each path tracks its class ancestry; repeated classes retain an explicit finite boundary.
    let mut pending:ChargedMap<Id<CatalogClass>,Vec<Id<ClassEntity>>>=Default::default();let mut done=ChargedSet::default();
    for class in out.classes.iter() {pending.insert(&mut charge,class.id(),vec![class.class])?;}
    loop {
        let next=pending.iter().find(|(id,_)|!done.contains(*id)).map(|(id,trail)|(*id,trail.clone()));
        let Some((id,trail))=next else {break};done.insert(&mut charge,id)?;
        let class=need(&out.classes,id)?.clone();let root=need(&out.candidates,class.candidate)?.clone();
        let exposure_link=need(&out.exposures,root.exposure)?.clone();let exposure=need(&data.exposures,exposure_link.exposure)?;
        let parent_member=need(&out.members,class.member)?.clone();
        if parent_member.path.len()>=MAX_DEPTH {boundary(out,class.candidate,PublicPathBoundaryReason::Depth,None)?;continue;}
        let mut chain:ChargedMap<(i64,Id<ClassEntity>,Option<Id<AncestryEntityMember>>),bool>=Default::default();
        chain.insert(&mut charge,(-1,class.class,None),true)?;
        for ancestry in data.ancestry.iter() {
            let owner=need(&data.resolutions,ancestry.class)?;
            if owner.context!=exposure.context || owner.entity.and_then(|e|class_entity(data,e))!=Some(class.class) {continue;}
            let raw=need(&data.ancestry_observations,ancestry.observation)?;
            if raw.relation!=AncestryRelation::Mro {continue;}
            let complete=ancestry.status==ResolutionStatus::Resolved && raw.linearization==Some(Linearization::Complete);
            if !complete {boundary(out,class.candidate,PublicPathBoundaryReason::OpenAncestry,None)?;}
            for ancestor in data.ancestors.iter().filter(|r|r.assessment==ancestry.id()) {
                let resolution=need(&data.resolutions,ancestor.resolution)?;
                let ordinal=need(&data.sequence_members,ancestor.member)?.ordinal;
                if let Some(base)=resolution.entity.and_then(|e|class_entity(data,e)) {chain.insert(&mut charge,(ordinal,base,Some(ancestor.id())),complete && resolution.status==ResolutionStatus::Resolved)?;}
                else {boundary(out,class.candidate,PublicPathBoundaryReason::OpenAncestry,None)?;}
            }
        }
        let mut first_gap=i64::MAX;
        for ancestry in data.ancestry.iter() {
            let owner=need(&data.resolutions,ancestry.class)?;
            if owner.context!=exposure.context || owner.entity.and_then(|e|class_entity(data,e))!=Some(class.class) {continue;}
            if need(&data.ancestry_observations,ancestry.observation)?.relation!=AncestryRelation::Mro {continue;}
            for ancestor in data.ancestors.iter().filter(|r|r.assessment==ancestry.id()) {
                let resolution=need(&data.resolutions,ancestor.resolution)?;
                let base=resolution.entity.and_then(|e|class_entity(data,e));
                if resolution.status!=ResolutionStatus::Resolved || !base.is_some_and(|c|matches!(data.source_classes.get(c),Some(ClassEntity::Source {..}))) {first_gap=first_gap.min(need(&data.sequence_members,ancestor.member)?.ordinal);boundary(out,class.candidate,PublicPathBoundaryReason::OpenAncestry,None)?;}
            }
        }
        let mut nearest:ChargedMap<String,i64>=Default::default();
        for ((rank,base,_),_) in chain.iter() {
            let Some(ClassEntity::Source {declaration:anchor})=data.source_classes.get(*base) else {continue};
            for declaration in data.declarations.iter().filter(|r|r.parent==Some(*anchor)) {
                for binding in data.bindings.iter() {
                    let event=need(&data.binding_events,binding.event)?;
                    if event.site!=declaration.declaration || need(&data.qualifications,binding.qualification)?.context!=exposure.context {continue;}
                    let previous=nearest.get(&event.name).copied().unwrap_or(i64::MAX);nearest.insert(&mut charge,event.name.clone(),previous.min(*rank))?;
                }
            }
        }
        for ((rank,base,ancestor),complete) in chain.iter() {
            let Some(ClassEntity::Source {declaration:anchor})=data.source_classes.get(*base) else {continue};
            for declaration in data.declarations.iter().filter(|r|r.parent==Some(*anchor)) {
                if need(&data.qualifications,declaration.qualification)?.context!=exposure.context {continue;}
                let Some(entity)=declaration_entity(data,declaration) else {boundary(out,class.candidate,PublicPathBoundaryReason::MissingCorrespondence,None)?;continue;};
                for binding in data.bindings.iter() {
                    let event=need(&data.binding_events,binding.event)?;
                    if event.site!=declaration.declaration || need(&data.qualifications,binding.qualification)?.context!=exposure.context {continue;}
                    if event.name.starts_with('_') && !matches!(event.name.as_str(),"__init__"|"__call__") {continue;}
                    let q=need(&data.qualifications,binding.qualification)?;
                    let definite=q.modality==attribution::Modality::Definite && q.approximation==assertion::Approximation::Exact && q.condition==conditions::Diagram::always().id() && binding.static_branch.is_none();
                    let mut disposition=if exposure.status==ResolutionStatus::Resolved && root.alias.is_none() && *complete && *rank<first_gap && definite && nearest.get(&event.name)==Some(rank) {PublicPathDisposition::Effective}else if nearest.get(&event.name).is_some_and(|r|r<rank) {PublicPathDisposition::Shadowed}else {PublicPathDisposition::Candidate};
                    for ((other_rank,other_base,_),_) in chain.iter().filter(|((r,_,_),_)|r<=rank) {
                        let Some(ClassEntity::Source {declaration:other_anchor})=data.source_classes.get(*other_base) else {continue};
                        for rebind in data.bindings.iter() {
                            let scope=need(&data.lexical_scopes,rebind.scope)?;let rebound_event=need(&data.binding_events,rebind.event)?;
                            if scope.owner!=*other_anchor || scope.kind!=LexicalScopeKind::Class || rebound_event.name!=event.name || need(&data.qualifications,rebind.qualification)?.context!=exposure.context {continue;}
                            if !matches!(rebind.kind,BindingEventKind::FunctionDef|BindingEventKind::ClassDef|BindingEventKind::AnnotationOnly) {
                                boundary(out,class.candidate,PublicPathBoundaryReason::Rebound,Some(rebind.id()))?;disposition=PublicPathDisposition::Candidate;
                            } else if *other_rank==*rank && rebind.ordinal>binding.ordinal && !declaration.overload {disposition=PublicPathDisposition::Shadowed;}
                        }
                    }
                    let mut segments=parent_member.path.clone();segments.push(event.name.clone());
                    let member=out.members.insert(CatalogMember {input:parent_member.input,access:parent_member.access,name:segments.join("."),path:segments})?;
                    let path=out.paths.insert(CatalogPath {parent:class.candidate,declaration:declaration.id(),binding:binding.id(),entity,ancestry:*ancestor,disposition})?;
                    let nested_exposure=out.exposures.insert(CatalogExposure {member,exposure:exposure.id()})?;
                    let candidate=out.candidates.insert(CatalogCandidate {exposure:nested_exposure,candidate:root.candidate,entity:None,path:Some(path),alias:root.alias})?;
                    match need(&data.refs,entity)? {
                        EntityRef::Callable {callable}=>build::callable(data,out,member,candidate,*callable,exposure.context)?,
                        EntityRef::Class {class:nested}=>{
                            let nested_class=out.classes.insert(CatalogClass {member,candidate,class:*nested})?;
                            build::constructors(data,out,nested_class,exposure.context)?;build::fields(data,out,member,*nested,exposure.context)?;
                            if trail.contains(nested) {boundary(out,candidate,PublicPathBoundaryReason::RecursiveClass,None)?;}
                            else {let mut next_trail=trail.clone();next_trail.push(*nested);pending.insert(&mut charge,nested_class,next_trail)?;}
                        }
                        _=>{}
                    }
                }
            }
        }
    }
    Ok(())
}
