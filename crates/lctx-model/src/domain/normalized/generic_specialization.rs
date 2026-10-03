//! Bounded structural substitution. Native binder identity and qualified evidence own the map.
use crate::domain::{normalized::Rows, resources::ResourceBudget, types::*, *};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Boundary { UnboundVariable(Id<TypeVariable>), UnresolvedParamSpec(Id<TypeVariable>), TypeVarTuple(Id<TypeVariable>), NamedInt(Id<TypeVariable>), RecursiveAlias(Id<TypeTerm>), AliasReference(Id<TypeTerm>), MissingNativeBinding, Qualification, Opaque(Id<TypeTerm>), WorkBound(Id<TypeTerm>) }
pub struct Environment {
    pub qualification: Id<assertion::AssertionQualification>,
    pub declaration: Id<NativeSignatureObservation>,
    pub site: Id<source::Occurrence>,
    pub bindings: Vec<GenericSpecializationObservation>,
}
pub struct Graph {
    pub terms: Rows<TypeTerm>, pub sequences: Rows<TypeSequence>, pub members: Rows<TypeSequenceMember>,
    pub lists: Rows<CallableParameterList>, pub slots: Rows<CallableParameter>,
    pub dict_lists: Rows<TypedDictFieldList>, pub fields: Rows<TypedDictField>,
    pub variables: Rows<TypeVariable>,
}
impl Graph {
    pub fn new(b:&ResourceBudget)->Self {Self {terms:Rows::new(b),sequences:Rows::new(b),members:Rows::new(b),lists:Rows::new(b),slots:Rows::new(b),dict_lists:Rows::new(b),fields:Rows::new(b),variables:Rows::new(b)}}
    pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {
        macro_rules! field {($($field:ident:$ty:ty),*)=>{$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*};}
        field!(terms:TypeTerm,sequences:TypeSequence,members:TypeSequenceMember,lists:CallableParameterList,slots:CallableParameter,dict_lists:TypedDictFieldList,fields:TypedDictField,variables:TypeVariable);Ok(false)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializationStatus { Resolved, Partial, Unknown }
pub struct Specialized {
    pub status: SpecializationStatus,
    pub role: calls::SignatureRole, pub site:Id<source::Occurrence>, pub receiver:Option<Id<TypeTerm>>, pub term: Id<TypeTerm>, pub qualification:Id<assertion::AssertionQualification>, pub declaration:Id<NativeSignatureObservation>,
    pub support: Vec<Id<GenericSpecializationObservation>>, pub boundaries:Vec<Boundary>, pub structure:Graph,
}
/// `structure` is an overlay over the input graph; retained bounded/recursive terms
/// may reference unchanged input children. No standalone persisted type closure is claimed.
/// The caller must supply model-validated native bindings; cross-context and contradictory maps
/// are still rejected here. Restrictions are evidence, not runtime assignability constraints.
pub fn substitute(graph:&Graph,term:Id<TypeTerm>,environment:&Environment,budget:&ResourceBudget)->Result<Specialized,ModelError> {
    let _scratch = budget.reserve("generic-substitution-scratch", environment.bindings.len().saturating_mul(256).saturating_add(graph.members.len().saturating_add(graph.slots.len()).saturating_add(graph.fields.len()).saturating_mul(256)))?;
    let mut map=BTreeMap::new();
    let mut support=Vec::new();
    for binding in &environment.bindings {
        if binding.qualification!=environment.qualification || binding.declaration!=environment.declaration || binding.site!=environment.site || environment.bindings.first().is_some_and(|first| first.receiver!=binding.receiver) {return Err(invalid("substitution crosses native declaration basis"));}
        if let Some(previous)=map.insert(binding.variable,binding.argument) && previous!=binding.argument {return Err(invalid("contradictory substitution binding"));}
        support.push(binding.id());
    }
    support.sort();support.dedup();
    let mut state=Substitution {input:graph,output:Graph::new(budget),boundaries:Vec::new(),active:BTreeSet::new(),work:0};
    if map.is_empty() {state.boundaries.push(Boundary::MissingNativeBinding);}
    let term=state.term(term,&map,0)?;
    let status = if state.boundaries.is_empty() {SpecializationStatus::Resolved} else if support.is_empty() {SpecializationStatus::Unknown} else {SpecializationStatus::Partial};
    Ok(Specialized {status,role:calls::SignatureRole::Specialized,site:environment.site,receiver:environment.bindings.first().map(|b|b.receiver),term,qualification:environment.qualification,declaration:environment.declaration,support,boundaries:state.boundaries,structure:state.output})
}
fn invalid(text:&str)->ModelError {ModelError::Invalid(text.into())}
struct Substitution<'a> {input:&'a Graph,output:Graph,boundaries:Vec<Boundary>,active:BTreeSet<Id<TypeTerm>>,work:usize}
impl Substitution<'_> {
    fn sequence(&mut self,id:Id<TypeSequence>,map:&BTreeMap<Id<TypeVariable>,Id<TypeTerm>>,depth:usize)->Result<Id<TypeSequence>,ModelError> {
        if self.input.sequences.get(id).is_none() {return Err(invalid("substitution sequence absent"));}
        let mut members:Vec<_>=self.input.members.iter().filter(|m|m.sequence==id).cloned().collect();members.sort_by_key(|m|m.ordinal);
        let mut children=Vec::new();for member in members {children.push((member.role,self.term(member.child,map,depth+1)?));}
        let (row,members)=TypeSequence::new(&children)?;let id=self.output.sequences.insert(row)?;for m in members {self.output.members.insert(m)?;}Ok(id)
    }
    fn list(&mut self,id:Id<CallableParameterList>,map:&BTreeMap<Id<TypeVariable>,Id<TypeTerm>>,depth:usize)->Result<Id<CallableParameterList>,ModelError> {
        if self.input.lists.get(id).is_none() {return Err(invalid("substitution callable list absent"));}
        let mut members:Vec<_>=self.input.slots.iter().filter(|m|m.list==id).cloned().collect();members.sort_by_key(|m|m.ordinal);
        let mut slots=Vec::new();for m in members {slots.push(Slot {name:m.name,kind:m.kind,required:m.required,term:self.term(m.term,map,depth+1)?});}
        let (row,members)=CallableParameterList::new(&slots)?;let id=self.output.lists.insert(row)?;for m in members {self.output.slots.insert(m)?;}Ok(id)
    }
    fn term(&mut self,id:Id<TypeTerm>,map:&BTreeMap<Id<TypeVariable>,Id<TypeTerm>>,depth:usize)->Result<Id<TypeTerm>,ModelError> {
        let mut term=self.input.terms.get(id).cloned().ok_or_else(||invalid("substitution term absent"))?;
        self.work+=1;
        if depth>=128 || self.work>4096 {self.boundaries.push(Boundary::WorkBound(id));self.output.terms.insert(term)?;return Ok(id);}
        if !self.active.insert(id) {self.boundaries.push(Boundary::RecursiveAlias(id));self.output.terms.insert(term)?;return Ok(id);}
        match &mut term {
            TypeTerm::TypeVar {variable} => {
                let native = self.input.variables.get(*variable).ok_or_else(||invalid("substitution variable absent"))?;
                if native.kind == TypeVariableKind::IntVar { self.boundaries.push(Boundary::NamedInt(*variable)); self.active.remove(&id); return self.output.terms.insert(term); }
                if native.kind != TypeVariableKind::TypeVar { return Err(invalid("type variable kind does not match term")); }
                if let Some(replacement)=map.get(variable) {let result=self.term(*replacement,map,depth+1)?;self.active.remove(&id);return Ok(result);}
                self.boundaries.push(Boundary::UnboundVariable(*variable));
            }
            TypeTerm::ParamSpec {variable} => self.boundaries.push(Boundary::UnresolvedParamSpec(*variable)),
            TypeTerm::TypeVarTuple {variable} => self.boundaries.push(Boundary::TypeVarTuple(*variable)),
            TypeTerm::VariableForm {variable,..} => {
                let variable=self.input.variables.get(*variable).ok_or_else(||invalid("substitution variable absent"))?;
                self.boundaries.push(match variable.kind {TypeVariableKind::ParamSpec=>Boundary::UnresolvedParamSpec(variable.id()),TypeVariableKind::TypeVarTuple=>Boundary::TypeVarTuple(variable.id()),TypeVariableKind::IntVar=>Boundary::NamedInt(variable.id()),TypeVariableKind::TypeVar=>Boundary::UnboundVariable(variable.id())});
            }
            TypeTerm::Generic {parameters,body} => {
                let mut inner=map.clone();
                for member in self.input.members.iter().filter(|m|m.sequence==*parameters) {
                    match self.input.terms.get(member.child) {Some(TypeTerm::TypeVar {variable}|TypeTerm::ParamSpec {variable}|TypeTerm::TypeVarTuple {variable})=>{inner.remove(variable);},_=>return Err(invalid("generic binder is not a native variable"))}
                }
                *parameters=self.sequence(*parameters,&BTreeMap::new(),depth)?;
                *body=self.term(*body,&inner,depth+1)?;
            }
            TypeTerm::ClassInstance {arguments,..}|TypeTerm::TypedDict {arguments,..}|TypeTerm::SelfType {arguments,..}=>*arguments=self.sequence(*arguments,map,depth)?,
            TypeTerm::TypeAliasReference {arguments,..}=>{*arguments=self.sequence(*arguments,map,depth)?;self.boundaries.push(Boundary::AliasReference(id));}
            TypeTerm::Union {members}|TypeTerm::Intersection {members}=>*members=self.sequence(*members,map,depth)?,
            TypeTerm::Overload {signatures,..}=>*signatures=self.sequence(*signatures,map,depth)?,
            TypeTerm::Overloaded {alternatives}=>*alternatives=self.sequence(*alternatives,map,depth)?,
            TypeTerm::Tuple {elements}=>*elements=self.sequence(*elements,map,depth)?,
            TypeTerm::TypeOf {target}|TypeTerm::TypeAlias {target,..}|TypeTerm::Annotated {target}|TypeTerm::Unpack {target}|TypeTerm::TypeGuard {target,..}|TypeTerm::TypeForm {target}=>*target=self.term(*target,map,depth+1)?,
            TypeTerm::BoundMethod {receiver,function}=>{*receiver=self.term(*receiver,map,depth+1)?;*function=self.term(*function,map,depth+1)?;}
            TypeTerm::Callable {parameters,param_spec,returns,..}=>{*parameters=self.list(*parameters,map,depth)?;if let Some(p)=param_spec {*p=self.term(*p,map,depth+1)?;}*returns=self.term(*returns,map,depth+1)?;}
            TypeTerm::ParamList {parameters,param_spec}=>{*parameters=self.list(*parameters,map,depth)?;if let Some(p)=param_spec {*p=self.term(*p,map,depth+1)?;}}
            TypeTerm::AnonymousTypedDict {fields,..}=>{
                let mut members:Vec<_>=self.input.fields.iter().filter(|m|m.list==*fields).cloned().collect();members.sort_by_key(|m|m.ordinal);
                let mut slots=Vec::new();for m in members {slots.push((m.name,m.required,self.term(m.term,map,depth+1)?));}
                let (row,members)=TypedDictFieldList::new(&slots)?;*fields=self.output.dict_lists.insert(row)?;for m in members {self.output.fields.insert(m)?;}
            }
            TypeTerm::Other {..}|TypeTerm::Truncated {..}|TypeTerm::Any {..}=>self.boundaries.push(Boundary::Opaque(id)),
            TypeTerm::ClassObject {..}|TypeTerm::Literal {..}|TypeTerm::Module {..}|TypeTerm::Never {..}|TypeTerm::None|TypeTerm::SpecialForm {..}|TypeTerm::EnumLiteral {..}|TypeTerm::LiteralString=>{}
        }
        self.active.remove(&id);self.output.terms.insert(term)
    }
}
