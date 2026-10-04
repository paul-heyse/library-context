//! Canonical correspondence for owned final Ruff records. Invocation-local IDs never escape.
use crate::{lexical_records::LexicalRecords, syntax_records::Spans};
use lctx_model::domain::{assertion::AssertionQualification, charged::StateCharge, lexical::*, resources::ResourceBudget, ruff::*, source::*, *};
use ruff_linter::semantic_facts::{Fact, Binding};
use ruff_python_semantic::BindingFlags;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct NativeRows { facts: Vec<Fact> }
impl NativeRows {
    pub fn capture(&mut self, fact:&Fact, charge:&mut StateCharge)->Result<(),ModelError> {
        if matches!(fact,Fact::Scope(_)|Fact::Definition{..}|Fact::Binding(_)|Fact::Reference(_)|Fact::Unresolved{..}) {
            let extra=match fact {Fact::Binding(b)=>b.name.len()+b.qualified_name.as_ref().map_or(0,|n|n.iter().map(|s|s.len()+64).sum()),Fact::Definition{name,..}=>name.as_ref().map_or(0,String::len),Fact::Unresolved{name,..}=>name.len(),_=>0};
            charge.grow(1024+extra)?;self.facts.push(fact.clone());
        }
        Ok(())
    }
}
pub struct NativeLexicalRecords {
    pub scopes:Vec<LexicalScope>,
    pub scope_observations:Vec<(LexicalScopeObservation,bool)>,
    pub bindings:Vec<RuffBindingObservation>,
    pub definitions:Vec<RuffDefinitionObservation>,
    pub targets:Vec<LexicalTarget>,
    pub resolutions:Vec<(LexicalResolution,bool)>,
    pub contexts:Vec<RuffContextObservation>,
    pub events:Vec<BindingEvent>,
    pub unlocated:usize,
    _charge:StateCharge,
}
impl NativeLexicalRecords {
    fn new(budget:&ResourceBudget)->Self {Self {scopes:vec![],scope_observations:vec![],bindings:vec![],definitions:vec![],targets:vec![],resolutions:vec![],contexts:vec![],events:vec![],unlocated:0,_charge:StateCharge::new(budget,"native-ruff-lexical")}}
}
fn binding_kind(kind:&str)->Option<RuffBindingKind> {
    use RuffBindingKind::*;
    Some(match kind {
        "Annotation"=>Annotation,"Argument"=>Argument,"NamedExprAssignment"=>NamedExprAssignment,"Assignment"=>Assignment,"TypeParam"=>TypeParam,
        "LoopVar"=>LoopVar,"WithItemVar"=>WithItemVar,"Global"=>Global,"Nonlocal"=>Nonlocal,"Builtin"=>Builtin,"ClassDefinition"=>ClassDefinition,
        "FunctionDefinition"=>FunctionDefinition,"Export"=>Export,"FutureImport"=>FutureImport,"Import"=>Import,"FromImport"=>FromImport,
        "SubmoduleImport"=>SubmoduleImport,"Deletion"=>Deletion,"BoundException"=>BoundException,"UnboundException"=>UnboundException,"DunderClassCell"=>DunderClassCell,_=>return None,
    })
}
/// Native identifier and kind select a canonical event; a spelling never searches the tree.
fn binding_site(binding:&Binding, spans:&Spans)->Option<Id<Occurrence>> {
    use SyntaxKind as K;
    let kind=binding_kind(binding.kind)?;
    if matches!(kind,RuffBindingKind::Builtin|RuffBindingKind::DunderClassCell) {return None;}
    if let Ok(id)=spans.get(binding.range,K::ExprName) {return Some(id);}
    let id=spans.get(binding.range,K::Identifier).ok()?;
    let (parent,_)=spans.parent(id)?;
    let actual=spans.nodes().find_map(|(site,kind)|(site==parent).then_some(kind))?;
    let admitted=match kind {
        RuffBindingKind::Argument=>actual==K::Parameter,
        RuffBindingKind::FunctionDefinition=>actual==K::StmtFunctionDef,
        RuffBindingKind::ClassDefinition=>actual==K::StmtClassDef,
        RuffBindingKind::Import|RuffBindingKind::FromImport|RuffBindingKind::SubmoduleImport|RuffBindingKind::FutureImport=>actual==K::Alias,
        RuffBindingKind::Global=>actual==K::StmtGlobal,
        RuffBindingKind::Nonlocal=>actual==K::StmtNonlocal,
        RuffBindingKind::BoundException|RuffBindingKind::UnboundException=>actual==K::ExceptHandlerExceptHandler,
        RuffBindingKind::TypeParam=>matches!(actual,K::TypeParamTypeVar|K::TypeParamParamSpec|K::TypeParamTypeVarTuple),
        _=>matches!(actual,K::PatternMatchAs|K::PatternMatchStar|K::PatternMatchMapping),
    };
    admitted.then_some(parent)
}
fn locate<T>(native:Option<u32>, map:&BTreeMap<u32,Id<T>>)->(Option<Id<T>>,NativeRelationLocation) {
    match native {None=>(None,NativeRelationLocation::Absent),Some(id)=>match map.get(&id).copied(){Some(id)=>(Some(id),NativeRelationLocation::Located),None=>(None,NativeRelationLocation::Unlocated)}}
}
impl NativeRows {
    pub fn lower(&self, spans:&Spans, source:&LexicalRecords, qualification:&AssertionQualification, candidate:Id<AssertionQualification>, budget:&ResourceBudget)->Result<NativeLexicalRecords,ModelError> {
        let mut out=NativeLexicalRecords::new(budget);
        let _transient=budget.reserve("native-ruff-local-correspondence",self.facts.len().saturating_mul(1024))?;
        let mut scopes=BTreeMap::new();let mut definitions=BTreeMap::new();let mut module_definitions=BTreeSet::new();let mut events=BTreeMap::new();let mut bindings=BTreeMap::new();
        for fact in &self.facts {
            match fact {
                Fact::Scope(scope)=>{
                    let (owner,kind)=match (scope.kind,scope.range) {
                        ("Module",None)=>(spans.nodes().find_map(|(id,kind)|(kind==SyntaxKind::ModModule).then_some(id)),LexicalScopeKind::Module),
                        ("Class",Some(range))=>(spans.get(range,SyntaxKind::StmtClassDef).ok(),LexicalScopeKind::Class),
                        ("Function",Some(range))=>(spans.get(range,SyntaxKind::StmtFunctionDef).ok(),LexicalScopeKind::Function),
                        ("Lambda",Some(range))=>(spans.get(range,SyntaxKind::ExprLambda).ok(),LexicalScopeKind::Lambda),
                        _=>{out.unlocated+=1;continue;},
                    };
                    if let Some(owner)=owner {let row=LexicalScope{owner,kind};out._charge.grow(512)?;scopes.insert(scope.id,row.id());if !source.scopes.iter().any(|s|s.id()==row.id()){out.scopes.push(row);}} else {out.unlocated+=1;}
                }
                Fact::Definition{id,range,kind,..}=>{
                    if *kind=="Module" {module_definitions.insert(*id);continue;}
                    let expected=if matches!(*kind,"Class"|"NestedClass"){SyntaxKind::StmtClassDef}else{SyntaxKind::StmtFunctionDef};
                    if let Some(declaration)=range.and_then(|range|spans.get(range,expected).ok()) {definitions.insert(*id,declaration);}else{out.unlocated+=1;}
                }
                Fact::Binding(binding)=>{
                    bindings.insert(binding.id,binding);
                    if let Some(site)=binding_site(binding,spans) {
                        // The event itself already has a canonical source identity. Native range,
                        // kind and canonical parent established it before this exact event lookup.
                        let hits=source.events.iter().filter(|e|e.site==site).collect::<Vec<_>>();
                        let hit=match hits.as_slice(){[event]=>Some(*event),_=>{let exact=hits.into_iter().filter(|e|e.name==binding.name).collect::<Vec<_>>();match exact.as_slice(){[event]=>Some(*event),_=>None}}};
                        if let Some(event)=hit {events.insert(binding.id,event.id());out._charge.grow(512+event.name.len())?;out.events.push(event.clone());}else{out.unlocated+=1;}
                    }else if !matches!(binding.kind,"Builtin"|"DunderClassCell"){out.unlocated+=1;}
                }
                _=>{}
            }
        }
        for fact in &self.facts {
            match fact {
                Fact::Scope(scope)=>{
                    let Some(id)=scopes.get(&scope.id).copied() else {continue;};
                    let (parent,location)=locate(scope.parent,&scopes);
                    if location==NativeRelationLocation::Unlocated {out.unlocated+=1;continue;}
                    let row=LexicalScopeObservation{qualification:qualification.id(),scope:id,parent};
                    let emit=!source.scope_observations.iter().any(|s|s.id()==row.id());out._charge.grow(512)?;out.scope_observations.push((row,emit));
                }
                Fact::Definition{id,parent,name,kind,..}=>{
                    let Some(declaration)=definitions.get(id).copied() else {continue;};
                    let kind=match *kind {"Class"=>RuffDefinitionKind::Class,"NestedClass"=>RuffDefinitionKind::NestedClass,"Function"=>RuffDefinitionKind::Function,"NestedFunction"=>RuffDefinitionKind::NestedFunction,"Method"=>RuffDefinitionKind::Method,_=>{out.unlocated+=1;continue;}};
                    let (parent,parent_location)=if parent.is_some_and(|id|module_definitions.contains(&id)){(None,NativeRelationLocation::Absent)}else{locate(*parent,&definitions)};
                    out._charge.grow(1024+name.as_ref().map_or(0,String::len))?;out.definitions.push(RuffDefinitionObservation{qualification:qualification.id(),declaration,kind,name:name.clone(),parent,parent_location});
                }
                Fact::Binding(binding)=>{
                    let Some(event)=events.get(&binding.id).copied() else {continue;};let Some(kind)=binding_kind(binding.kind) else {out.unlocated+=1;continue;};
                    let scope=scopes.get(&binding.scope).copied();let scope_location=if scope.is_some(){AttachmentStatus::Located}else{AttachmentStatus::Unlocated};
                    let (shadowed,shadowed_location)=locate(binding.shadowed,&events);let (outer_shadowed,outer_shadowed_location)=locate(binding.outer_shadowed,&events);let (definition_scope,definition_scope_location)=locate(binding.definition_scope,&scopes);
                    if scope_location==AttachmentStatus::Unlocated || [shadowed_location,outer_shadowed_location,definition_scope_location].contains(&NativeRelationLocation::Unlocated){out.unlocated+=1;}
                    let flags=BindingFlags::from_bits_retain(binding.flags);
                    out._charge.grow(2048+binding.name.len()+binding.qualified_name.as_ref().map_or(0,|names|names.iter().map(|s|s.len()+64).sum()))?;
                    out.bindings.push(RuffBindingObservation{qualification:qualification.id(),event,kind,native_name:binding.name.clone(),scope,scope_location,shadowed,shadowed_location,outer_shadowed,outer_shadowed_location,definition_scope,definition_scope_location,typing:binding.context.is_typing(),qualified_name:binding.qualified_name.clone(),
                        explicit_export:flags.contains(BindingFlags::EXPLICIT_EXPORT),external:flags.contains(BindingFlags::EXTERNAL),alias:flags.contains(BindingFlags::ALIAS),nonlocal:flags.contains(BindingFlags::NONLOCAL),global:flags.contains(BindingFlags::GLOBAL),deleted:flags.contains(BindingFlags::DELETED),invalid_all_format:flags.contains(BindingFlags::INVALID_ALL_FORMAT),invalid_all_object:flags.contains(BindingFlags::INVALID_ALL_OBJECT),private_declaration:flags.contains(BindingFlags::PRIVATE_DECLARATION),unpacked_assignment:flags.contains(BindingFlags::UNPACKED_ASSIGNMENT),in_except_handler:flags.contains(BindingFlags::IN_EXCEPT_HANDLER),annotated_type_alias:flags.contains(BindingFlags::ANNOTATED_TYPE_ALIAS),deferred_type_alias:flags.contains(BindingFlags::DEFERRED_TYPE_ALIAS),in_assert_statement:flags.contains(BindingFlags::IN_ASSERT_STATEMENT),lazy:flags.contains(BindingFlags::LAZY)});
                }
                Fact::Reference(reference)=>{
                    let Some(subject)=spans.reference(reference.range,reference.is_load) else {out.unlocated+=1;continue;};
                    let binding=bindings.get(&reference.binding).copied();let final_binding=events.get(&reference.binding).copied();
                    let qualified_name=binding.and_then(|b|b.qualified_name.as_ref());
                    out._charge.grow(1024+qualified_name.map_or(0,HeapSize::heap_bytes))?;out.contexts.push(RuffContextObservation {qualification:qualification.id(),subject,phase:ContextPhase::FinalReference,reference_load:Some(reference.is_load),typing:Some(reference.typing_context),typing_only_annotation:Some(reference.typing_only_annotation),runtime_annotation:Some(reference.runtime_annotation),string_annotation:Some(reference.string_annotation),type_checking:Some(reference.type_checking),qualified_name:qualified_name.cloned(),final_binding,final_binding_location:Some(if final_binding.is_some(){AttachmentStatus::Located}else{AttachmentStatus::Unlocated}),unresolved_wildcard:None,unresolved_annotation_binding:None});
                    // A different native scope does not supply closure capture authority.
                    if reference.is_load && let (Some(binding),Some(event))=(binding,final_binding)
                        && reference.scope==binding.scope {let target=LexicalTarget::Binding{event};let row=LexicalResolution{qualification:qualification.id(),read:subject,target:target.id(),captured:false};let emit=!source.resolutions.iter().any(|r|r.id()==row.id());out._charge.grow(1024)?;out.targets.push(target);out.resolutions.push((row,emit));}
                }
                Fact::Unresolved{range,wildcard_import,annotation_binding,..}=>{
                    let Some(subject)=spans.reference(*range,true) else {out.unlocated+=1;continue;};
                    out._charge.grow(2048)?;out.contexts.push(RuffContextObservation{qualification:qualification.id(),subject,phase:ContextPhase::FinalUnresolved,reference_load:None,typing:None,typing_only_annotation:None,runtime_annotation:None,string_annotation:None,type_checking:None,qualified_name:None,final_binding:None,final_binding_location:None,unresolved_wildcard:Some(*wildcard_import),unresolved_annotation_binding:Some(annotation_binding.is_some())});
                    let target=LexicalTarget::Unresolved{reason:obligation::ObligationKind::UnresolvedTarget};let row=LexicalResolution{qualification:if *wildcard_import{candidate}else{qualification.id()},read:subject,target:target.id(),captured:false};let emit=!source.resolutions.iter().any(|r|r.id()==row.id());out.targets.push(target);out.resolutions.push((row,emit));
                }
                _=>{}
            }
        }
        out.events.sort_by_key(Record::id);out.events.dedup_by_key(|row|row.id());
        out.targets.sort_by_key(Record::id);out.targets.dedup_by_key(|row|row.id());
        Ok(out)
    }
}
