//! Supported structural questions consume exact, supported characterization; no runtime closure.
use super::{*,build::need,classification::ClassificationData,evaluate::{candidate_entity,type_match}};
use crate::domain::{charged::{ChargedVec,StateCharge},normalized::{decorator_identity,entities::*},resources::ResourceBudget,*};
pub(super) struct Answer {pub value:Option<bool>,pub basis:EvidenceBasis,pub evidence:ChargedVec<Witness>,charge:StateCharge}
impl Answer {fn witness(&mut self,w:Witness)->Result<(),ModelError>{self.evidence.push(&mut self.charge,w)}}
fn exact(d:&ClassificationData,id:Id<assertion::AssertionQualification>,ctx:Id<attribution::AnalysisContext>)->bool {d.source.core.qualifications.get(id).is_some_and(|q|q.context==ctx&&q.modality==attribution::Modality::Definite&&q.approximation==assertion::Approximation::Exact&&q.condition==conditions::Diagram::always().id()&&q.assumptions==assumptions::AssumptionSet::empty().id())}
fn candidate(c:&Context)->Option<Id<catalog::CatalogCandidate>>{match c{Context::Binding{candidate,..}|Context::Signature{candidate,..}=>Some(*candidate),_=>None}}
pub(super) fn declaration(d:&ClassificationData,c:&Context)->Result<Option<Id<source::Occurrence>>,ModelError>{
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
    // A retained mismatch cannot close an omitted/unavailable native raise trace. Existing
    // module Types coverage certifies only this emitted typing set, never runtime exceptions.
    let mut complete=false;
    if !matched&&seen&&!unknown
      && let Some(declaration)=declaration(d,c)? {
        let artifact=need(&d.source.core.occurrences,declaration)?.source;
        let scope=(source::CoverageScope::Artifact{artifact}).id();
        let mut found=false;let mut all_complete=true;
        for inventory in d.source.core.native_coverage.iter().filter(|r|r.context==c.analysis()&&r.scope==scope&&r.family==attribution::FactFamily::Types) {
          found=true;all_complete&=inventory.status==attribution::CoverageStatus::CompleteUnderStatedModel;
          if inventory.status==attribution::CoverageStatus::CompleteUnderStatedModel {out.witness(Witness::NativeTypingCoverage{coverage:inventory.id()})?;}
        }
        complete=found&&all_complete;
    }
    out.value=if matched{Some(true)}else if seen&&!unknown&&complete{Some(false)}else{None};
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
        if let normalized::links::ReferenceEntityTarget::Binding{event,entity:target}=target
          && *target==entity {
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

#[cfg(test)]
mod raised_coverage_tests {
    use super::*;
    use crate::domain::attribution::*;
    use crate::domain::normalized::Rows;
    fn id<R>(byte:u8)->Id<R>{serde_json::from_value(serde_json::json!(vec![byte;16])).unwrap()}
    fn fixture()->(ResourceBudget,ClassificationData,Context,Id<source::SourceArtifact>,Id<types::TypeTerm>,Id<types::TypeTerm>){
        let budget=ResourceBudget::fixed(8<<20).unwrap();
        let mut d=ClassificationData::new(&budget);
        let artifact=id(1);let analysis=id(2);
        let declaration=d.source.core.occurrences.insert(source::Occurrence{source:artifact,start:0,end:20,syntax_kind:source::SyntaxKind::StmtFunctionDef,role:source::OccurrenceRole::Syntax,structural_path:vec![0]}).unwrap();
        let callable=d.source.core.source_callables.insert(CallableEntity::Source{declaration,kind:CallableKind::Function}).unwrap();
        let entity=d.source.core.refs.insert(EntityRef::Callable{callable}).unwrap();
        let target=d.source.core.entity_candidates.insert(SymbolEntityCandidate{resolution:id(3),entity}).unwrap();
        let candidate=d.source.catalog.candidates.insert(catalog::CatalogCandidate{exposure:id(4),candidate:None,entity:Some(target),path:None,alias:None}).unwrap();
        let c=Context::Signature{member:id(5),candidate,invocation:id(6),analysis};
        let q=d.source.core.qualifications.insert(assertion::AssertionQualification{context:analysis,scope:(source::CoverageScope::Artifact{artifact}).id(),condition:conditions::Diagram::always().id(),modality:Modality::Definite,approximation:assertion::Approximation::Exact,assumptions:assumptions::AssumptionSet::empty_id()}).unwrap();
        let mut sites=Vec::new();
        for ordinal in 0..2 {
            let occurrence=d.source.core.occurrences.insert(source::Occurrence{source:artifact,start:ordinal+1,end:ordinal+2,syntax_kind:source::SyntaxKind::StmtRaise,role:source::OccurrenceRole::Syntax,structural_path:vec![0,ordinal as i32]}).unwrap();
            d.source.core.ownership.insert(OccurrenceOwnership{occurrence,owner:declaration,entity}).unwrap();sites.push(occurrence);
        }
        let known=d.facts.type_terms.insert(types::TypeTerm::None).unwrap();
        let other=d.facts.type_terms.insert(types::TypeTerm::LiteralString).unwrap();
        // Only the first raise has a native answer; the second site models an omitted trace.
        let observation=d.facts.type_observations.insert(types::TypeObservation{qualification:q,subject:sites[0],role:types::TypeRole::Raised,declared:false,term:known}).unwrap();
        d.facts.type_supports.insert(types::TypeSupport{assertion:observation,run:id(7),surface:id(8),evidence:id(9),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:crate::domain::attribution::Fidelity::NativeStructural}).unwrap();
        (budget,d,c,artifact,known,other)
    }
    fn coverage(artifact:Id<source::SourceArtifact>,context:Id<AnalysisContext>,status:CoverageStatus)->ProviderCoverage {
        ProviderCoverage{scope:(source::CoverageScope::Artifact{artifact}).id(),provider:Some(id(10)),context,family:FactFamily::Types,run:Some(id(11)),status,reason:(status!=CoverageStatus::CompleteUnderStatedModel).then_some(ObligationKind::MissingEvidence),diagnostic:None}
    }
    #[test]
    fn missing_or_incomplete_raise_inventory_retains_positive_but_never_false_counterexample(){
        let (budget,mut d,c,artifact,known,other)=fixture();
        let base=coverage(artifact,c.analysis(),CoverageStatus::CompleteUnderStatedModel);
        let cases=[None,Some(coverage(artifact,c.analysis(),CoverageStatus::Partial)),Some(ProviderCoverage{context:id(12),..base.clone()}),Some(ProviderCoverage{scope:(source::CoverageScope::Artifact{artifact:id(13)}).id(),..base.clone()}),Some(ProviderCoverage{family:FactFamily::Signatures,..base})];
        for case in cases {
            d.source.core.native_coverage=Rows::new(&budget);
            if let Some(row)=case{d.source.core.native_coverage.insert(row).unwrap();}
            let positive=answer(&d,&FacetValue::RaisedClass{r#type:StructuralType::CanonicalTerm{term:known}},&c,&budget).unwrap();
            assert_eq!(positive.value,Some(true),"an actual matching native observation survives other missing evidence");
            let negative=answer(&d,&FacetValue::RaisedClass{r#type:StructuralType::CanonicalTerm{term:other}},&c,&budget).unwrap();
            assert_eq!(negative.value,None,"an omitted raise trace is not a complete nonmatching set");
            assert!(!negative.evidence.iter().any(|w|matches!(w,Witness::NativeTypingCoverage{..})));
        }
    }
    #[test]
    fn complete_native_raise_set_requires_source_context_closure_witness(){
        let (budget,mut d,c,artifact,_,other)=fixture();
        // The closed case contains only the actually characterized raise site.
        let observed=d.facts.type_observations.iter().next().unwrap().subject;
        let owned=d.source.core.ownership.iter().find(|o|o.occurrence==observed).unwrap().clone();
        d.source.core.ownership=Rows::new(&budget);d.source.core.ownership.insert(owned).unwrap();
        let row=coverage(artifact,c.analysis(),CoverageStatus::CompleteUnderStatedModel);
        let coverage=d.source.core.native_coverage.insert(row.clone()).unwrap();
        let negative=answer(&d,&FacetValue::RaisedClass{r#type:StructuralType::CanonicalTerm{term:other}},&c,&budget).unwrap();
        assert_eq!(negative.value,Some(false));
        let witness=Witness::NativeTypingCoverage{coverage};
        assert!(negative.evidence.iter().any(|w|w==&witness));
        d.validate_witness(&c,&witness).unwrap();
        d.source.core.native_coverage=Rows::new(&budget);
        let foreign=d.source.core.native_coverage.insert(ProviderCoverage{context:id(14),..row}).unwrap();
        assert!(d.validate_witness(&c,&Witness::NativeTypingCoverage{coverage:foreign}).is_err());
    }
}
