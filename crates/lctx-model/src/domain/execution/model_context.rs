//! Pinned class protocols and exact native exception ancestry. No ambient import is admissible.
use crate::domain::{*,models::{Catalog,CompiledContextProtocol,ContextEntry,ContextExit},normalized::Rows,calls::ProviderSymbol,symbols::{ClassAncestryObservation,AncestryRelation,Linearization},attribution::{AnalysisContext,ObligationKind},resources::ResourceBudget};
use super::model_application::{ModelApplicationData,append_native_evidence,RuntimeEvidenceError};
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ObligationKind>{rows.get(id).ok_or(ObligationKind::MissingEvidence)}
fn one<'a,T:'a>(mut rows:impl Iterator<Item=&'a T>)->Result<&'a T,ObligationKind>{let row=rows.next().ok_or(ObligationKind::MissingEvidence)?;if rows.next().is_some(){return Err(ObligationKind::AmbiguousBinding)}Ok(row)}
/// Complete native ancestry admits both a positive subclass match and an exact negative.
/// Its lifetime retains the input tables and the charged bounded ancestry inventory.
pub struct CheckedExactClass<'a>{symbol:&'a ProviderSymbol,mro:&'a ClassAncestryObservation,ancestors:charged::ChargedSet<Id<ProviderSymbol>>,premises:Rows<analysis::native::NativeAssertionPremise>,status:analysis::policy::EvidenceStatus,_charge:charged::StateCharge}
impl<'a> CheckedExactClass<'a>{
 pub fn symbol(&self)->Id<ProviderSymbol>{self.symbol.id()}
 pub fn premises(&self)->&Rows<analysis::native::NativeAssertionPremise>{&self.premises}
 pub fn status(&self)->analysis::policy::EvidenceStatus{self.status}
 pub fn ancestry(&self)->Id<ClassAncestryObservation>{self.mro.id()}
 pub fn matches(&self,handler:&Self)->Result<bool,ObligationKind>{if (self.symbol.provider,self.symbol.context)!=(handler.symbol.provider,handler.symbol.context){return Err(ObligationKind::IncompatibleContexts)}Ok(self.symbol.id()==handler.symbol.id()||self.ancestors.contains(&handler.symbol.id()))}
 pub fn derive(data:&'a ModelApplicationData,symbol:Id<ProviderSymbol>,context:Id<AnalysisContext>,budget:&ResourceBudget)->Result<Result<Self,ObligationKind>,ModelError>{
  let mut charge=charged::StateCharge::new(budget,"model-exact-class");charge.grow(size_of::<Self>())?;let mut ancestors=charged::ChargedSet::default();
  let result=(||{let b=&data.bindings;let symbol=need(&b.symbols,symbol)?;if symbol.kind!=calls::SymbolKind::Class||symbol.context!=context{return Err(ObligationKind::IncompatibleContexts)}
   let mro=one(b.ancestry.iter().filter(|a|a.class==symbol.id()&&a.relation==AncestryRelation::Mro))?;
   super::model_application::exact(b,mro.qualification,context)?;
   if mro.linearization!=Some(Linearization::Complete){return Err(ObligationKind::IncompleteCoverage)}
   if !b.ancestry_supports.iter().any(|s|s.assertion==mro.id()&&b.runs.get(s.run).is_some_and(|r|r.context==context)){return Err(ObligationKind::MissingEvidence)}
   let sequence=need(&b.sequences,mro.ancestors)?;let _=sequence;
   for member in b.sequence_members.iter().filter(|m|m.sequence==mro.ancestors){let ancestor=need(&b.symbols,member.symbol)?;if ancestor.kind!=calls::SymbolKind::Class||ancestor.context!=context||ancestor.provider!=symbol.provider{return Err(ObligationKind::IncompatibleContexts)}}
   Ok((symbol,mro))})();
  match result{Err(r)=>Ok(Err(r)),Ok((symbol,mro))=>{for member in data.bindings.sequence_members.iter().filter(|m|m.sequence==mro.ancestors){ancestors.insert(&mut charge,member.symbol)?;}let mut premises=Rows::new(budget);let mut status=analysis::policy::EvidenceStatus::StructurallyObserved;
 let evidence=(||->Result<(),RuntimeEvidenceError>{append_native_evidence(data,derivation::RowRef::of(mro.id()),mro.qualification,context,&mut premises,&mut status)?;
 for id in std::iter::once(symbol.id()).chain(ancestors.iter().copied()) {let observation=one(data.bindings.symbol_observations.iter().filter(|o|o.symbol==id&&data.bindings.qualifications.get(o.qualification).is_some_and(|q|q.context==context)))?;append_native_evidence(data,derivation::RowRef::of(observation.id()),observation.qualification,context,&mut premises,&mut status)?;}Ok(())})();
 match evidence{Err(RuntimeEvidenceError::Boundary(reason))=>Ok(Err(reason)),Err(RuntimeEvidenceError::Model(error))=>Err(error),Ok(())=>Ok(Ok(Self{symbol,mro,ancestors,premises,status,_charge:charge}))}}}
 }
}
/// Protocol selection pins one native class plus allocation/initialization and entry/exit definitions.
/// It does not assert that source evaluation, entry or exit has happened.
pub struct CheckedContextProtocol<'a>{compiled:&'a CompiledContextProtocol,class:CheckedExactClass<'a>,allocation:Id<ProviderSymbol>,initialization:Id<ProviderSymbol>,entry:Id<ProviderSymbol>,exit:Id<ProviderSymbol>,_charge:charged::StateCharge}
impl<'a> CheckedContextProtocol<'a>{
 pub fn compiled(&self)->&CompiledContextProtocol{self.compiled}
 pub fn class(&self)->&CheckedExactClass<'a>{&self.class}
 pub fn allocation(&self)->Id<ProviderSymbol>{self.allocation}
 pub fn initialization(&self)->Id<ProviderSymbol>{self.initialization}
 pub fn entry(&self)->Id<ProviderSymbol>{self.entry}
 pub fn exit(&self)->Id<ProviderSymbol>{self.exit}
 pub fn derive(catalog:&'a Catalog,data:&'a ModelApplicationData,class:Id<ProviderSymbol>,input:Id<input::InputRevision>,context:Id<AnalysisContext>,budget:&ResourceBudget)->Result<Result<Self,ObligationKind>,ModelError>{
  let mut charge=charged::StateCharge::new(budget,"model-context-protocol");charge.grow(size_of::<Self>())?;
  let symbol=match need(&data.bindings.symbols,class){Ok(s)=>s,Err(r)=>return Ok(Err(r))};
  let compiled=match one(catalog.context_protocols().iter().filter(|c|super::model_application::matches_target(data,&c.model().target,symbol,input,context).is_ok())){Ok(c)=>c,Err(r)=>return Ok(Err(r))};
  let class=match CheckedExactClass::derive(data,class,context,budget)?{Ok(c)=>c,Err(r)=>return Ok(Err(r))};
  let target_bytes=match &compiled.model().target{models::Target::Stdlib{python,module,callable}=>python.len()+module.len()+callable.len(),models::Target::Dependency{distribution,version,module,callable}=>distribution.len()+version.len()+module.len()+callable.len(),models::Target::Release{module,callable}=>module.len()+callable.len()};
  charge.grow(target_bytes.saturating_mul(4).saturating_add(256))?;
  let find=|target:&models::Target|one(data.bindings.symbols.iter().filter(|s|s.context==context&&super::model_application::matches_target(data,target,s,input,context).is_ok())).map(Record::id);
  let result=(||{
   let allocation=find(&compiled.model().allocation)?;let initialization=find(&compiled.model().initialization)?;
   let member=|suffix:&str|{let mut target=compiled.model().target.clone();match &mut target{models::Target::Stdlib{callable,..}|models::Target::Dependency{callable,..}|models::Target::Release{callable,..}=>{callable.push('.');callable.push_str(suffix)}}find(&target)};
   let entry=member("__enter__")?;let exit=member("__exit__")?;
   Ok(Self{compiled,class,allocation,initialization,entry,exit,_charge:charge})
  })();Ok(result)
 }
 /// A checked initializer supplies exact formal identities. Variadic elements preserve their
 /// original projection and cannot masquerade as a whole source expression.
 pub fn entry_value(&self,application:&CheckedProtocolInitialization<'_>,data:&ModelApplicationData)->Result<ContextValue,ObligationKind>{
  let signature=need(&data.bindings.signatures,application.bound().bound().signature())?;
  if signature.symbol!=self.initialization || application.shape().phase()!=calls::CallPhase::Init{return Err(ObligationKind::IncompatibleContexts)}
  match &self.compiled.model().entry{
   ContextEntry::NoneValue=>Ok(ContextValue::None),
   ContextEntry::ArgumentOrNone{formal}=>{
    let formal=super::model_application::formal(&data.bindings,signature,Some(formal))?.ok_or(ObligationKind::MissingEvidence)?;
    let binding=one(application.bound().bound().bindings().iter().filter(|b|b.formal==formal))?;
    match (&binding.source,&binding.projection){(calls::BindingSource::Actual{occurrence},calls::BindingProjection::Whole)=>Ok(ContextValue::Actual(*occurrence)),(calls::BindingSource::Default,calls::BindingProjection::Whole)=>Ok(ContextValue::None),_=>Err(ObligationKind::UnsupportedUnpacking)}
   }
  }
 }
 pub fn preserves(&self)->bool{matches!(self.compiled.model().exit,ContextExit::Preserve)}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ContextValue{None,Actual(Id<source::Occurrence>)}

/// Exact protocol constructor applicability consumes the same sole normalized binder. A
/// protocol has its own authored initializer target; it does not require an invented ordinary
/// behavior model for that initializer.
pub struct CheckedProtocolInitialization<'a>{protocol:&'a CheckedContextProtocol<'a>,bound:&'a normalized::binding_normalization::ValidatedBoundCall,shape:&'a normalized::binding_normalization::BindingShapeAdmission,_charge:charged::StateCharge}
impl<'a> CheckedProtocolInitialization<'a>{
 pub fn protocol(&self)->&CheckedContextProtocol<'a>{self.protocol}
 pub fn bound(&self)->&normalized::binding_normalization::ValidatedBoundCall{self.bound}
 pub fn shape(&self)->&normalized::binding_normalization::BindingShapeAdmission{self.shape}
 pub fn derive(protocol:&'a CheckedContextProtocol<'a>,data:&ModelApplicationData,bound:&'a normalized::binding_normalization::ValidatedBoundCall,shape:&'a normalized::binding_normalization::BindingShapeAdmission,effective:Option<&normalized::binding_normalization::EffectiveInvocationAdmission>,budget:&ResourceBudget)->Result<Result<Self,ObligationKind>,ModelError>{
  let mut charge=charged::StateCharge::new(budget,"checked-protocol-initializer");charge.grow(size_of::<Self>())?;
  let result=(||{if !shape.admits(bound)||shape.phase()!=calls::CallPhase::Init||shape.context()!=protocol.class.symbol.context{return Err(ObligationKind::IncompatibleContexts)}
   let signature=need(&data.bindings.signatures,bound.bound().signature())?;if signature.symbol!=protocol.initialization{return Err(ObligationKind::MissingEvidence)}
   super::model_application::runtime_identity(data,bound,shape,effective)?;
   Ok(Self{protocol,bound,shape,_charge:charge})
  })();Ok(result)
 }
}
