//! An unchanged source store has entry-value premises; it establishes no later heap value.
use crate::Domain;
use crate::domain::transfer::TransferKind;
use crate::domain::{
    analysis::local as owner, assertion::*, attribution::*, conditions::{entry::*, Diagram},
    flow::*, local_fields::FieldData, normalized::{entities::*, symbolic_fields::*},
    obligation::ObligationKind, resources::ResourceBudget, source::*, value::*, *,
};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="local_symbolic_field_stores",rule="unchanged_source_field_store")]
pub struct SymbolicFieldStore {
    #[model(key)] pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)] pub store: Id<SourceFieldStore>,
    #[model(key, premise)] pub value: Id<FlowValueObservation>,
    #[model(key, premise)] pub support: Id<FlowValueSupport>,
    #[model(premise)] pub value_entry: Id<EntryValueWitness>,
    #[model(premise)] pub receiver_entry: Id<EntryValueWitness>,
    pub constructor: Id<EntityRef>,
    pub qualification: Id<AssertionQualification>,
}

pub struct StoreEmission {
    pub row: SymbolicFieldStore,
    pub value: DerivedEntryValue,
    pub receiver: DerivedEntryValue,
}

pub(crate) fn same(data: &EntryData, a: Id<Occurrence>, b: Id<Occurrence>) -> bool {
    data.occurrences.get(a).zip(data.occurrences.get(b)).is_some_and(|(a,b)|
        (a.source,a.start,a.end,a.syntax_kind)==(b.source,b.start,b.end,b.syntax_kind))
}

/// Positional syntax names a ParameterWithDefault wrapper; formal identity names its Parameter
/// child. Follow that supported source edge before asking Entry to prove the formal binding.
fn receiver_declaration(
    data: &FieldData<'_>, invocation: &owner::AnalysisInvocation, syntax: Id<Occurrence>,
) -> Result<Id<Occurrence>, ObligationKind> {
    let e=data.entry;
    let node=e.occurrences.get(syntax).ok_or(ObligationKind::MissingEvidence)?;
    let theory=crate::domain::local_theory::TheoryData{entry:e,inventory:data.theory};
    let mut selected=None;
    for declaration in e.declarations.iter() {
        let Some(formal)=e.occurrences.get(declaration.declaration) else{continue};
        if formal.syntax_kind!=SyntaxKind::Parameter || formal.role!=OccurrenceRole::Parameter
            || e.qualifications.get(declaration.qualification).is_none_or(|q|q.context!=invocation.context){continue}
        let linked=if node.syntax_kind==SyntaxKind::Parameter {
            same(e,syntax,declaration.declaration)
        }else if node.syntax_kind==SyntaxKind::ParameterWithDefault {
            e.placements.iter().any(|p|p.parent.is_some_and(|parent|same(e,parent,syntax))
                && p.field==crate::domain::lexical::SyntaxField::Child && p.ordinal==0
                && same(e,p.occurrence,declaration.declaration)
                && e.qualifications.get(p.qualification).is_some_and(|q|
                    e.placement_supports.iter().any(|s|s.assertion==p.id()
                        && s.origin==Origin::SourceObservation && s.mode==ExtractionMode::NativeTraversal
                        && crate::domain::local_theory::support(&theory,s,q,invocation,FactFamily::Syntax).is_ok())))
        }else{false};
        if !linked{continue}
        if selected.is_some_and(|other|other!=declaration.declaration){return Err(ObligationKind::EntryValueUnknown)}
        selected=Some(declaration.declaration);
    }
    selected.ok_or(ObligationKind::MissingEvidence)
}

/// Re-derive both entry operations from the native flow source; a stored receipt is no authority.
pub fn derive(
    data: &FieldData<'_>, invocation: &owner::AnalysisInvocation, store: &SourceFieldStore,
    support: &FlowValueSupport, budget: &ResourceBudget,
) -> Result<Result<StoreEmission, ObligationKind>, ModelError> {
    let e=data.entry;
    let setup=(|| {
        let value=e.values.get(support.assertion).ok_or(ObligationKind::MissingEvidence)?;
        let q=e.qualifications.get(value.qualification).ok_or(ObligationKind::MissingEvidence)?;
        let sq=e.qualifications.get(store.qualification).ok_or(ObligationKind::MissingEvidence)?;
        if !store.plain_initializer || value.kind!=FlowSinkKind::Definition
            || value.transfer!=TransferKind::Identity || value.through_call
            || q.context!=invocation.context || sq.context!=q.context
            || q.condition!=Diagram::always().id() || q.modality!=Modality::Definite
            || q.approximation!=Approximation::Exact {
            return Err(ObligationKind::ScopeBoundary);
        }
        let run=e.runs.get(support.run).ok_or(ObligationKind::MissingEvidence)?;
        if (run.input,run.context)!=(invocation.input,invocation.context) {
            return Err(ObligationKind::IncompatibleContexts);
        }
        let theory=crate::domain::local_theory::TheoryData{entry:e,inventory:data.theory};
        crate::domain::local_theory::support(&theory,support,q,invocation,FactFamily::Flow)
            .map_err(|_|ObligationKind::MissingEvidence)?;
        let use_=e.uses.get(value.use_).ok_or(ObligationKind::MissingEvidence)?;
        if !same(e,use_.occurrence,store.value) || !same(e,value.sink,store.value) {
            return Err(ObligationKind::MissingEvidence);
        }
        let formal=e.declarations.get(store.formal).ok_or(ObligationKind::MissingEvidence)?;
        let receiver=data.inventory.symbolic_parameter_syntax.get(store.receiver)
            .ok_or(ObligationKind::MissingEvidence)?;
        let owner=e.owners.iter().find(|o|same(e,o.occurrence,store.target))
            .ok_or(ObligationKind::MissingEvidence)?;
        let EntityRef::Callable{callable}=e.refs.get(owner.entity).ok_or(ObligationKind::MissingEvidence)?
            else{return Err(ObligationKind::MissingEvidence)};
        if !matches!(e.callables.get(*callable),Some(CallableEntity::Source{declaration,..})if same(e,*declaration,store.constructor)) {
            return Err(ObligationKind::MissingEvidence);
        }
        let mut bases=e.placements.iter().filter(|p|p.parent.is_some_and(|p|same(e,p,store.target))
            && p.field==crate::domain::lexical::SyntaxField::Value);
        let base=bases.next().ok_or(ObligationKind::MissingEvidence)?;
        if bases.next().is_some(){return Err(ObligationKind::MissingEvidence)}
        let mut uses=e.uses.iter().filter(|u|same(e,u.occurrence,base.occurrence));
        let receiver_use=uses.next().ok_or(ObligationKind::EntryValueUnknown)?;
        if uses.next().is_some(){return Err(ObligationKind::EntryValueUnknown)}
        let receiver_formal=receiver_declaration(data,invocation,receiver.parameter)?;
        Ok((value,q,owner.entity,run.id(),use_.occurrence,formal.declaration,receiver_use.occurrence,receiver_formal))
    })();
    let (value,q,constructor,run,access,formal,receiver_access,receiver_formal)=match setup{Ok(v)=>v,Err(r)=>return Ok(Err(r))};
    let request=|access,declaration|EntryRequest{owner:constructor,formal:ParameterEntity::Source{declaration}.id(),access,context:invocation.context,run};
    let value_entry=match EntryValueWitness::derive(data.entry,request(access,formal),budget)?{Ok(v)=>v,Err(r)=>return Ok(Err(r))};
    let receiver_entry=match EntryValueWitness::derive(data.entry,request(receiver_access,receiver_formal),budget)?{Ok(v)=>v,Err(r)=>return Ok(Err(r))};
    if !value_entry.condition().is_true() || !receiver_entry.condition().is_true(){return Ok(Err(ObligationKind::ScopeBoundary))}
    Ok(Ok(StoreEmission{row:SymbolicFieldStore{invocation:invocation.id(),store:store.id(),value:value.id(),support:support.id(),value_entry:value_entry.witness().id(),receiver_entry:receiver_entry.witness().id(),constructor,qualification:q.id()},value:value_entry,receiver:receiver_entry}))
}
