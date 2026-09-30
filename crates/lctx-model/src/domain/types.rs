//! Structural provider type observations. Renderings do not define modeled type identity.
//! Variable restrictions are qualified relationships, so recursive bounds need no recursive key.
use crate::{Assertion,Domain,DomainCode,DomainSum};
use super::charged::{ChargedMap, ChargedSet, ChargedVec, StateCharge};
use super::{*,assertion::AssertionQualification,attribution::{AnalysisContext,FactFamily,Provider,ProviderRun,Fidelity},
    calls::{ProviderSymbol,SymbolKind},source::{CoverageScope,Occurrence},value::Literal,obligation::ObligationKind};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum AnyFlavor { Explicit = 0, Implicit = 1, Error = 2 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum NeverFlavor { NoReturn = 0, Never = 1 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TypeVariableKind { TypeVar = 0, ParamSpec = 1, TypeVarTuple = 2, IntVar = 3 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TypeVariableOrigin { ScopedLegacy = 0, Pep695 = 1, Synthetic = 2, SyntheticSelf = 3, MapIntTuples = 4, NormalizedMapIntTuples = 5 }
/// Native quantified identity is qualified by provider/context. The range is in the provider's
/// module coordinate space; it does not assert that a captured declaration was found there.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "type_variables", validate = validate_variable)]
pub struct TypeVariable {
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub module: Id<super::calls::ProviderModule>,
    #[model(key)] pub anchor_start: i64,
    #[model(key)] pub anchor_end: i64,
    #[model(key)] pub slot: i64,
    #[model(key)] pub origin: TypeVariableOrigin,
    #[model(key)] pub kind: TypeVariableKind,
    pub name: String,
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
fn validate_variable(row: &TypeVariable) -> Result<(),ModelError> {
    if row.name.is_empty() || row.anchor_start < 0 || row.anchor_end < row.anchor_start || row.slot < 0 {
        return Err(invalid("invalid native type-variable identity"));
    }
    Ok(())
}
/// Representative structural vocabulary. Codes retain the corresponding existing type-kind
/// codes. Further native forms are added by the complete P2 producer mapping, never relabelled.
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name = "type_terms", invariants = type_invariants)]
pub enum TypeTerm {
    #[model(code = 0)] ClassInstance { class: Id<ProviderSymbol>, arguments: Id<TypeSequence> },
    #[model(code = 1)] ClassObject { class: Id<ProviderSymbol> },
    #[model(code = 2)] TypeOf { target: Id<TypeTerm> },
    #[model(code = 4)] Union { members: Id<TypeSequence> },
    #[model(code = 5)] Intersection { members: Id<TypeSequence> },
    #[model(code = 10)] Tuple { elements: Id<TypeSequence> },
    #[model(code = 11)] Literal { value: Id<Literal> },
    #[model(code = 12)] TypeVar { variable: Id<TypeVariable> },
    #[model(code = 13)] ParamSpec { variable: Id<TypeVariable> },
    #[model(code = 14)] TypeVarTuple { variable: Id<TypeVariable> },
    #[model(code = 16)] Any { flavor: AnyFlavor },
    #[model(code = 17)] Never { flavor: NeverFlavor },
    #[model(code = 18)] None,
    #[model(code = 26)] Other { provider: Id<Provider>, context: Id<AnalysisContext>, variant: String, display: String },
    #[model(code = 27)] Truncated { provider: Id<Provider>, context: Id<AnalysisContext>, reason: ObligationKind, display: String },
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TypeChildRole { Argument = 0, Member = 1, Element = 4, Variadic = 5 }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "type_sequences", invariants = sequence_invariants)]
pub struct TypeSequence { #[model(key)] pub members: ContentHash }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "type_sequence_members", validate = validate_member)]
pub struct TypeSequenceMember {
    #[model(key)] pub sequence: Id<TypeSequence>,
    #[model(key)] pub ordinal: i64,
    pub role: TypeChildRole,
    pub child: Id<TypeTerm>,
}
fn validate_member(row: &TypeSequenceMember) -> Result<(),ModelError> {
    if row.ordinal < 0 { return Err(invalid("negative type-child ordinal")); } Ok(())
}
fn sequence_digest(items: &[(TypeChildRole,Id<TypeTerm>)]) -> ContentHash {
    let mut sink = KeySink::new("type-sequence");
    for (ordinal,(role,child)) in items.iter().enumerate() { (ordinal as i64).encode(&mut sink); role.encode(&mut sink); child.encode(&mut sink); }
    (items.len() as i64).encode(&mut sink); sink.finish()
}
impl TypeSequence {
    pub fn new(items: &[(TypeChildRole,Id<TypeTerm>)]) -> Result<(Self,Vec<TypeSequenceMember>),ModelError> {
        if items.len() > 4096 { return Err(invalid("type-sequence work limit")); }
        let row = Self { members: sequence_digest(items) };
        let members = items.iter().enumerate().map(|(ordinal,(role,child))| TypeSequenceMember {
            sequence: row.id(),ordinal: ordinal as i64,role: *role,child: *child,
        }).collect();
        Ok((row,members))
    }
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TypeRole { Parameter = 0, Return = 1, CallResult = 2, Argument = 3, Raised = 4 }
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "type_observations")]
#[assertion(support = TypeSupport, name = "type_supports", family = FactFamily::Types, subjects(subject, term))]
pub struct TypeObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub subject: Id<Occurrence>,
    #[model(key)] pub role: TypeRole,
    #[model(key)] pub declared: bool,
    #[model(key)] pub term: Id<TypeTerm>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "type_presentations")]
#[assertion(support = TypePresentationSupport, name = "type_presentation_supports", family = FactFamily::Types, subjects(scope, term))]
pub struct TypePresentation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub term: Id<TypeTerm>,
    #[model(key)] pub display: String,
    #[model(key)] pub detail: Option<String>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TypeRestrictionKind { Bound = 12, Constraint = 13, Default = 14 }
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "type_variable_restrictions", validate = validate_restriction)]
#[assertion(support = TypeRestrictionSupport, name = "type_restriction_supports", family = FactFamily::Types, subjects(scope, variable, term))]
pub struct TypeVariableRestriction {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub variable: Id<TypeVariable>,
    #[model(key)] pub kind: TypeRestrictionKind,
    #[model(key)] pub ordinal: i64,
    #[model(key)] pub term: Id<TypeTerm>,
}
fn validate_restriction(row: &TypeVariableRestriction) -> Result<(),ModelError> {
    if row.ordinal < 0 || (row.kind != TypeRestrictionKind::Constraint && row.ordinal != 0) {
        return Err(invalid("invalid type restriction ordinal"));
    }
    Ok(())
}

fn sequence_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "type_sequence_membership",inputs: vec![ValidationInput::of::<TypeSequence>(&["id"]),
        ValidationInput::of::<TypeSequenceMember>(&["sequence","ordinal"])],create: std::sync::Arc::new(|budget| Box::new(SequenceCheck { charge: StateCharge::new(budget,"type_sequence_membership"),..Default::default() })) }]
}
#[derive(Default)]
struct SequenceCheck { charge: StateCharge,expected: ChargedMap<Id<TypeSequence>,ContentHash>,current: Option<(Id<TypeSequence>,Vec<(TypeChildRole,Id<TypeTerm>)>)> }
impl SequenceCheck {
    fn flush(&mut self) -> Result<(),ModelError> {
        if let Some((id,members)) = self.current.take() {
            if self.expected.remove(&mut self.charge,&id) != Some(sequence_digest(&members)) { return Err(invalid("type sequence membership differs")); }
        } Ok(())
    }
}
impl InvariantCheck for SequenceCheck {
    fn visit(&mut self, relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == TypeSequence::NAME { for row in TypeSequence::decode(batch)? {
            self.expected.insert(&mut self.charge,row.id(),row.members)?;
        } }
        else if relation == TypeSequenceMember::NAME { for row in TypeSequenceMember::decode(batch)? {
            if self.current.as_ref().is_none_or(|(id,_)| *id != row.sequence) { self.flush()?; self.current = Some((row.sequence,Vec::new())); }
            let (_,members) = self.current.as_mut().expect("current sequence");
            if row.ordinal != members.len() as i64 || members.len() >= 4096 { return Err(invalid("type sequence gaps, duplicates or work limit")); }
            members.push((row.role,row.child));
        } } else { return Err(invalid("undeclared type sequence input")); }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(),ModelError> {
        self.flush()?;
        if self.expected.values().any(|digest| *digest != sequence_digest(&[])) { return Err(invalid("type sequence has missing members")); }
        Ok(())
    }
}

fn type_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "structural_type_shapes",inputs: TypeIndex::inputs(),create: std::sync::Arc::new(|budget| Box::new(TypeIndex::new(budget,"structural_type_shapes"))) }]
}
/// The same native-owner closure is used by shared support validation and structural checks.
#[derive(Default)]
pub(crate) struct TypeIndex {
    charge: StateCharge,
    symbols: ChargedMap<Id<ProviderSymbol>,ProviderSymbol>,variables: ChargedMap<Id<TypeVariable>,TypeVariable>,
    terms: ChargedMap<Id<TypeTerm>,TypeTerm>,sequences: ChargedSet<Id<TypeSequence>>,
    members: ChargedMap<Id<TypeSequence>,Vec<TypeSequenceMember>>,
    /// Terms whose closure is verified for (provider, context, display-only support); a shared
    /// term is walked once per key, so total work is linear in the term graph.
    verified: ChargedSet<(Id<TypeTerm>,Id<Provider>,Id<AnalysisContext>,bool)>,
}
impl TypeIndex {
    pub fn new(budget: &super::resources::ResourceBudget, owner: &'static str) -> Self { Self { charge: StateCharge::new(budget,owner),..Self::default() } }
    pub fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<ProviderSymbol>(&["id"]),ValidationInput::of::<TypeVariable>(&["id"]),
        ValidationInput::of::<TypeSequence>(&["id"]),ValidationInput::of::<TypeSequenceMember>(&["sequence","ordinal"]),ValidationInput::of::<TypeTerm>(&["id"])] }
    pub fn visit_input(&mut self, relation: &str,batch: &arrow_array::RecordBatch) -> Result<bool,ModelError> {
        if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.symbols.insert(&mut self.charge,row.id(),row)?; } }
        else if relation == TypeVariable::NAME { for row in TypeVariable::decode(batch)? { self.variables.insert(&mut self.charge,row.id(),row)?; } }
        else if relation == TypeSequence::NAME { for row in TypeSequence::decode(batch)? { self.sequences.insert(&mut self.charge,row.id())?; } }
        else if relation == TypeSequenceMember::NAME { for row in TypeSequenceMember::decode(batch)? { self.members.update(&mut self.charge,row.sequence,|members| members.push(row))?; } }
        else if relation == TypeTerm::NAME { for row in TypeTerm::decode(batch)? { self.terms.insert(&mut self.charge,row.id(),row)?; } }
        else { return Ok(false); }
        Ok(true)
    }
    fn sequence(&self, id: Id<TypeSequence>) -> Result<&[TypeSequenceMember],ModelError> {
        if !self.sequences.contains(&id) { return Err(invalid("type sequence absent")); }
        Ok(self.members.get(&id).map(Vec::as_slice).unwrap_or_default())
    }
    pub fn variable_owner(&self, id: Id<TypeVariable>,run: &ProviderRun) -> Result<(),ModelError> {
        let v = self.variables.get(&id).ok_or_else(|| invalid("type variable absent"))?;
        if (v.provider,v.context) != (run.provider,run.context) { return Err(invalid("type variable belongs to another provider/context")); } Ok(())
    }
    pub fn term_support(&mut self, id: Id<TypeTerm>,run: &ProviderRun,fidelity: Fidelity) -> Result<(),ModelError> {
        let key = |term: Id<TypeTerm>| (term,run.provider,run.context,fidelity == Fidelity::DisplayOnly);
        if self.verified.contains(&key(id)) { return Ok(()); }
        // The walk's scratch is charged to the same budget and released when the walk ends.
        let mut scratch = StateCharge::new(self.charge.budget().ok_or_else(|| invalid("type closure needs a validation budget"))?,"type_closure");
        let mut pending = ChargedVec::default(); let mut seen = ChargedSet::default();
        pending.push(&mut scratch,id)?;
        while let Some(id) = pending.take_last(&mut scratch) {
            if self.verified.contains(&key(id)) || !seen.insert(&mut scratch,id)? { continue; }
            let mut children = Vec::new();
            let term = self.terms.get(&id).ok_or_else(|| invalid("type term absent"))?;
            match term {
                TypeTerm::ClassInstance { class,arguments } => { self.class_owner(*class,run)?; children.extend(self.sequence(*arguments)?.iter().map(|m| m.child)); },
                TypeTerm::ClassObject { class } => self.class_owner(*class,run)?,
                TypeTerm::TypeOf { target } => children.push(*target),
                TypeTerm::Union { members } | TypeTerm::Intersection { members } => children.extend(self.sequence(*members)?.iter().map(|m| m.child)),
                TypeTerm::Tuple { elements } => children.extend(self.sequence(*elements)?.iter().map(|m| m.child)),
                TypeTerm::TypeVar { variable } | TypeTerm::ParamSpec { variable } | TypeTerm::TypeVarTuple { variable } => self.variable_owner(*variable,run)?,
                TypeTerm::Other { provider,context,.. } | TypeTerm::Truncated { provider,context,.. } => {
                    if (*provider,*context) != (run.provider,run.context) { return Err(invalid("opaque type belongs to another provider/context")); }
                    if fidelity != Fidelity::DisplayOnly { return Err(invalid("opaque type closure requires display-only support")); }
                },
                TypeTerm::Literal { .. } | TypeTerm::Any { .. } | TypeTerm::Never { .. } | TypeTerm::None => {},
            }
            for child in children { pending.push(&mut scratch,child)?; }
        }
        // Every term the walk reached has its own closure inside this verified one.
        for term in seen.iter() { self.verified.insert(&mut self.charge,key(*term))?; }
        Ok(())
    }
    pub fn symbol(&self, id: Id<ProviderSymbol>) -> Result<&ProviderSymbol,ModelError> { self.symbols.get(&id).ok_or_else(|| invalid("provider symbol absent")) }
    fn class_owner(&self, id: Id<ProviderSymbol>,run: &ProviderRun) -> Result<(),ModelError> {
        let symbol = self.symbols.get(&id).ok_or_else(|| invalid("type class symbol absent"))?;
        if (symbol.provider,symbol.context) != (run.provider,run.context) { return Err(invalid("type class belongs to another provider/context")); } Ok(())
    }
}
impl InvariantCheck for TypeIndex {
    fn visit(&mut self, relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if !self.visit_input(relation,batch)? { return Err(invalid("undeclared type shape input")); } Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> {
        for term in self.terms.values() {
            let class = match term { TypeTerm::ClassInstance { class,.. } | TypeTerm::ClassObject { class } => Some(class),_ => None };
            if let Some(class) = class {
                if self.symbols.get(class).is_none_or(|s| s.kind != SymbolKind::Class) { return Err(invalid("class term lacks a native class symbol")); }
            }
            match term {
                TypeTerm::ClassInstance { arguments,.. } => {
                    if self.sequence(*arguments)?.iter().any(|m| m.role != TypeChildRole::Argument) { return Err(invalid("class type children must be arguments")); }
                },
                TypeTerm::Union { members } | TypeTerm::Intersection { members } => {
                    let items = self.sequence(*members)?;
                    if items.is_empty() || items.iter().any(|m| m.role != TypeChildRole::Member) { return Err(invalid("union/intersection needs members")); }
                },
                TypeTerm::Tuple { elements } => {
                    let items = self.sequence(*elements)?;
                    if items.iter().any(|m| !matches!(m.role,TypeChildRole::Element|TypeChildRole::Variadic)) || items.iter().filter(|m| m.role == TypeChildRole::Variadic).count() > 1 {
                        return Err(invalid("tuple needs fixed elements and at most one variadic segment"));
                    }
                },
                TypeTerm::TypeVar { variable } | TypeTerm::ParamSpec { variable } | TypeTerm::TypeVarTuple { variable } => {
                    let v = self.variables.get(variable).ok_or_else(|| invalid("type variable absent"))?;
                    let valid = match term { TypeTerm::TypeVar { .. } => matches!(v.kind,TypeVariableKind::TypeVar|TypeVariableKind::IntVar),
                        TypeTerm::ParamSpec { .. } => v.kind == TypeVariableKind::ParamSpec,TypeTerm::TypeVarTuple { .. } => v.kind == TypeVariableKind::TypeVarTuple,_ => false };
                    if !valid { return Err(invalid("type variable kind differs from term")); }
                },
                _ => {},
            }
        }
        Ok(())
    }
}
