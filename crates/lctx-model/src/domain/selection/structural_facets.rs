//! Supported structural questions consume exact, supported characterization; no runtime closure.
use super::{*,build::need,classification::ClassificationData,evaluate::{candidate_entity,type_match}};
use crate::domain::{charged::{ChargedVec,StateCharge},normalized::{decorator_identity,entities::*},resources::ResourceBudget,*};
pub(super) struct Answer {pub value:Option<bool>,pub basis:EvidenceBasis,pub evidence:ChargedVec<Witness>,charge:StateCharge}
impl Answer {fn witness(&mut self,w:Witness)->Result<(),ModelError>{self.evidence.push(&mut self.charge,w)}}
fn exact(d:&ClassificationData,id:Id<assertion::AssertionQualification>,ctx:Id<attribution::AnalysisContext>)->bool {d.source.core.qualifications.get(id).is_some_and(|q|q.context==ctx&&q.modality==attribution::Modality::Definite&&q.approximation==assertion::Approximation::Exact&&q.condition==conditions::Diagram::always().id()&&q.assumptions==assumptions::AssumptionSet::empty().id())}
fn candidate(c:&Context)->Option<Id<catalog::CatalogCandidate>>{match c{Context::Binding{candidate,..}|Context::Signature{candidate,..}=>Some(*candidate),_=>None}}
fn declaration(d:&ClassificationData,c:&Context)->Result<Option<Id<source::Occurrence>>,ModelError>{
 let Some(candidate)=candidate(c) else{return Ok(None);};let Some(entity)=candidate_entity(d,candidate)? else{return Ok(None);};
 Ok(match need(&d.source.core.refs,entity)?{EntityRef::Callable{callable}=>match d.source.core.source_callables.get(*callable){Some(CallableEntity::Source{declaration,..})=>Some(*declaration),_=>None},_=>None})
}
pub(super) fn applicable(d:&ClassificationData,v:&FacetValue,c:&Context)->Result<bool,ModelError>{
 match v{
  FacetValue::ClassMetadata{..}=>Ok(matches!(c,Context::Binding{..})),
  FacetValue::Deprecation{role,..}=>{let Context::Signature{invocation,..}=c else{return Ok(false);};let invocation=need(&d.source.catalog.invocations,*invocation)?;Ok(need(&d.source.core.variants,invocation.variant)?.role==*role)},
  _=>{let Context::Signature{invocation,..}=c else{return Ok(false);};let invocation=need(&d.source.catalog.invocations,*invocation)?;Ok(need(&d.source.core.variants,invocation.variant)?.role.runtime_source())}
 }
}
fn combine(a:Option<bool>,b:Option<bool>)->Option<bool>{match(a,b){(Some(a),Some(b))if a==b=>Some(a),_=>None}}
pub(super) fn answer(d:&ClassificationData,v:&FacetValue,c:&Context,budget:&ResourceBudget)->Result<Answer,ModelError>{
 let mut out=Answer{value:None,basis:EvidenceBasis::ProviderDeclaration,evidence:ChargedVec::default(),charge:StateCharge::new(budget,"selection-structural-facets")};
 match v{
  FacetValue::Async{asynchronous}=>{
    out.basis=EvidenceBasis::SourceDeclaration;
    let Some(declaration)=declaration(d,c)? else{return Ok(out);};let mut first=true;
    for row in d.source.core.declarations.iter().filter(|r|r.declaration==declaration){
      if !exact(d,row.qualification,c.analysis()){continue;}
      let mut supported=false;for support in d.facts.declaration_supports.iter().filter(|s|s.assertion==row.id()){supported=true;out.witness(Witness::SourceCharacterization{observation:row.id(),support:support.id()})?;}
      if supported{let value=Some((row.kind==syntax::DeclarationKind::AsyncFunction)==*asynchronous);out.value=if first{value}else{combine(out.value,value)};first=false;}
    }
  }
  FacetValue::Deprecation{deprecated,..}=>{
    let Context::Signature{invocation,..}=c else{return Ok(out);};let variant=need(&d.source.core.variants,need(&d.source.catalog.invocations,*invocation)?.variant)?;
    let Some(row)=variant.native.and_then(|id|d.source.core.native_signatures.get(id)) else{return Ok(out);};
    if !exact(d,row.qualification,c.analysis()){return Ok(out);}
    let value=match row.deprecation{types::CallableDeprecation::Unavailable=>None,types::CallableDeprecation::NotDeprecated=>Some(!*deprecated),types::CallableDeprecation::Deprecated=>Some(*deprecated)};
    for support in d.facts.native_signature_supports.iter().filter(|s|s.assertion==row.id()){out.witness(Witness::NativeCallableMetadata{observation:row.id(),support:support.id()})?;out.value=value;}
  }
  FacetValue::ClassMetadata{trait_kind,present}=>{
    let Some(candidate)=candidate(c)else{return Ok(out);};let Some(entity)=candidate_entity(d,candidate)?else{return Ok(out);};
    if !matches!(need(&d.source.core.refs,entity)?,EntityRef::Class{..}){return Ok(out);}
    let mut first=true;
    for row in d.source.core.class_metadata.iter().filter(|m|d.source.core.resolutions.iter().any(|r|r.symbol==m.class&&r.context==c.analysis()&&r.status==ResolutionStatus::Resolved&&r.entity==Some(entity))){
      if !exact(d,row.qualification,c.analysis()){continue;}
      let value=match trait_kind{ClassFacet::FinalDeclaration=>Some(row.final_declaration),ClassFacet::Protocol=>Some(row.protocol),ClassFacet::RuntimeCheckable=>Some(row.runtime_checkable),ClassFacet::Enumeration=>Some(row.enumeration),ClassFacet::ExplicitlyAbstract=>Some(row.explicitly_abstract),ClassFacet::AbstractMembers if row.abstract_members.is_empty()&&!row.abstract_absence_known=>None,ClassFacet::AbstractMembers=>Some(!row.abstract_members.is_empty()),ClassFacet::ExplicitSlots=>Some(row.explicit_slots)}.map(|known|known==*present);
      let mut supported=false;for support in d.facts.metadata_supports.iter().filter(|s|s.assertion==row.id()){supported=true;out.witness(Witness::ClassMetadata{observation:row.id(),support:support.id()})?;}
      if supported{out.value=if first{value}else{combine(out.value,value)};first=false;}
    }
  }
  FacetValue::RaisedClass{r#type}=>{
    let Some(candidate)=candidate(c)else{return Ok(out);};let Some(entity)=candidate_entity(d,candidate)?else{return Ok(out);};let mut seen=false;let mut unknown=false;let mut matched=false;
    for row in d.facts.type_observations.iter().filter(|r|r.role==types::TypeRole::Raised&&d.source.core.ownership.iter().any(|o|o.occurrence==r.subject&&o.entity==entity)){
      if !exact(d,row.qualification,c.analysis()){unknown=true;continue;}
      let mut supported=false;for support in d.facts.type_supports.iter().filter(|s|s.assertion==row.id()){supported=true;out.witness(Witness::RaisedType{observation:row.id(),support:support.id()})?;}
      if !supported{unknown=true;continue;}seen=true;match type_match(d,row.term,r#type,c.analysis())?{Some(true)=>matched=true,Some(false)=>{},None=>unknown=true}
    }
    out.value=if matched{Some(true)}else if seen&&!unknown{Some(false)}else{None};
  }
  FacetValue::DecoratorQualifiedName{module,path}=>{
    out.basis=EvidenceBasis::SourceDeclaration;
    let Some(declaration)=declaration(d,c)?else{return Ok(out);};
    let n=&d.source.core;let inputs=decorator_identity::Inputs{qualifications:&n.qualifications,occurrences:&n.occurrences,placements:&n.placements,references:&n.references,assessments:&n.reference_assessments,candidates:&n.reference_candidates,targets:&n.reference_targets,resolutions:&n.lexical_resolutions};
    let mut known=false;let mut unknown=false;let mut matched=false;
    for row in d.facts.decorators.iter().filter(|r|r.declaration==declaration){
      let selected=inputs.select(row,c.analysis(),budget)?;
      for selection in selected.selections.iter(){
        let(Some(entity),Some(assessment),Some(candidate))=(selection.entity,selection.assessment,selection.candidate)else{unknown=true;continue;};
        let mut supported=false;for support in d.facts.decorator_supports.iter().filter(|s|s.assertion==row.id()){supported=true;out.witness(Witness::ResolvedDecorator{observation:row.id(),support:support.id(),assessment,candidate})?;}
        if !supported{unknown=true;continue;}
        // Resolution chooses the identity first. Native module/name metadata characterize that
        // identity; a matching source spelling never selects a decorator target.
        let mut selected_name=None;
        let target=need(&n.reference_targets,need(&n.reference_candidates,candidate)?.target)?;
        if let normalized::links::ReferenceEntityTarget::Binding{event,entity:target}=target {
          if *target==entity {
            let event=need(&n.binding_events,*event)?;
            for binding in n.bindings.iter().filter(|r|r.event==event.id()&&matches!(r.kind,lexical::BindingEventKind::FunctionDef|lexical::BindingEventKind::ClassDef)&&exact(d,r.qualification,c.analysis())) {
              let mut supported=false;for support in d.facts.binding_supports.iter().filter(|r|r.assertion==binding.id()){out.witness(Witness::LexicalDefinition{observation:binding.id(),support:support.id()})?;supported=true;}if !supported{continue;}
              let scope=need(&n.lexical_scopes,binding.scope)?;
              if scope.kind!=lexical::LexicalScopeKind::Module {continue;}
              let site=need(&n.occurrences,event.site)?;
              for owner in n.modules.iter().filter(|r|r.source==site.source) {
                let value=owner.qualified_name==*module&&path.len()==1&&event.name==path[0];
                selected_name=Some(match selected_name{None=>Some(value),Some(old)=>combine(old,Some(value))});
              }
            }
          }
        }
        for resolution in n.resolutions.iter().filter(|r|r.entity==Some(entity)&&r.context==c.analysis()&&r.status==ResolutionStatus::Resolved){
          let symbol=need(&n.symbols,resolution.symbol)?;
          let native_module=match need(&n.provider_modules,symbol.module)?{calls::ProviderModule::Acquired{module}=>Some(need(&n.modules,*module)?.qualified_name.as_str()),calls::ProviderModule::Bundled{name,..}=>Some(name.as_str()),_=>None};
          if let Some(native_module)=native_module {
            // ProviderSymbol.name is one native symbol name. Nested source paths require a
            // declared parent path rather than treating the leaf name as a qualified identity.
            let top_level=match need(&n.refs,entity)?{EntityRef::Callable{callable}=>match n.source_callables.get(*callable){Some(CallableEntity::Source{declaration,..})=>n.declarations.iter().any(|r|r.declaration==*declaration&&r.parent.is_none()),Some(CallableEntity::External{..}|CallableEntity::Synthetic{..})=>symbol.kind==calls::SymbolKind::Function,_=>false},_=>false};
            if !top_level{continue;}
            let value=native_module==module&&path.len()==1&&symbol.name==path[0];selected_name=Some(match selected_name{None=>Some(value),Some(old)=>combine(old,Some(value))});
          }
        }
        match selected_name.flatten(){Some(value)=>{known=true;matched|=value;},None=>unknown=true}
      }
    }
    // An empty/incomplete decorator inventory is not evidence of absence.
    out.value=if matched{Some(true)}else if known&&!unknown{Some(false)}else{None};
  }
  _=>return Err(ModelError::Invalid("structural facet was not lowered to its predicate".into())),
 }
 Ok(out)
}
