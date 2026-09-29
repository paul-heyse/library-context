//! Qualified propositions and concrete support relationships share one validation contract.
//!
//! Qualification must distinguish proposition identity; payload-only qualification is refused.
//! ```compile_fail
//! use lctx_model::{Domain, Assertion, domain::{Id, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence}};
//! #[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
//! #[model(name = "bad_assertions")]
//! #[assertion(support = BadSupport, name = "bad_supports", family = FactFamily::Syntax, subjects(occurrence))]
//! struct Bad { qualification: Id<AssertionQualification>, #[model(key)] occurrence: Id<Occurrence> }
//! ```
//! ```
//! use lctx_model::{Domain, Assertion, domain::{Id, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence}};
//! #[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
//! #[model(name = "good_assertions")]
//! #[assertion(support = GoodSupport, name = "good_supports", family = FactFamily::Syntax, subjects(occurrence))]
//! struct Good { #[model(key)] qualification: Id<AssertionQualification>, #[model(key)] occurrence: Id<Occurrence> }
//! ```
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use std::collections::BTreeSet;
use std::marker::PhantomData;
use crate::{Domain, DomainCode, DomainSum};
use super::{*, attribution::*, source::*, input::*, conditions::*, value::{Place,PlaceRoot,Predicate}, transfer::TransferKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Approximation { Exact = 0, Over = 1, Under = 2, Mixed = 3, Unknown = 4 }

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assertion_qualifications", invariants = qualification_invariants)]
pub struct AssertionQualification {
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub condition: Id<Condition>,
    #[model(key)] pub modality: Modality,
    #[model(key)] pub approximation: Approximation,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_surfaces", validate = validate_surface)]
pub struct ProviderSurface {
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key)] pub family: FactFamily,
    #[model(key)] pub name: String,
}
fn validate_surface(row: &ProviderSurface) -> Result<(), ModelError> {
    if row.name.is_empty() || !row.family.is_coverage_family() { return Err(invalid("invalid provider surface")); }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "evidence", validate = validate_evidence, invariants = evidence_invariants)]
pub enum Evidence {
    #[model(code = 0)] Occurrence { occurrence: Id<Occurrence> },
    #[model(code = 1)] SourceSpan { source: Id<SourceArtifact>, start: i64, end: i64 },
    /// Explicit invocation evidence for facts with no source declaration. Never invent a span.
    #[model(code = 2)] Invocation { run: Id<ProviderRun> },
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
fn validate_evidence(row: &Evidence) -> Result<(), ModelError> {
    if let Evidence::SourceSpan { start, end, .. } = row {
        if *start < 0 || end < start { return Err(invalid("invalid evidence span")); }
    }
    Ok(())
}
#[derive(Debug, Clone, Copy)]
pub enum Subject { TypeTerm(Id<super::types::TypeTerm>), TypeVariable(Id<super::types::TypeVariable>), FlowUse(Id<super::flow::FlowUse>), FlowDefinition(Id<super::flow::FlowDefinition>), ReachingDefinition(Id<super::flow::ReachingDefinition>), DocumentNode { id: Id<super::documents::DocumentNode>, tag: i16 }, SourceSpan(Id<Evidence>), Occurrence(Id<Occurrence>), Artifact(Id<SourceArtifact>), Module(Id<Module>), Scope(Id<CoverageScope>), Place(Id<Place>), Transfer(Id<TransferKey>),
    LexicalScope(Id<super::lexical::LexicalScope>), BindingEvent(Id<super::lexical::BindingEvent>), LexicalTarget(Id<super::lexical::LexicalTarget>) }
impl From<Id<Occurrence>> for Subject { fn from(value: Id<Occurrence>) -> Self { Self::Occurrence(value) } }
impl From<Id<SourceArtifact>> for Subject { fn from(value: Id<SourceArtifact>) -> Self { Self::Artifact(value) } }
impl From<Id<Module>> for Subject { fn from(value: Id<Module>) -> Self { Self::Module(value) } }
impl From<Id<CoverageScope>> for Subject { fn from(value: Id<CoverageScope>) -> Self { Self::Scope(value) } }
impl From<Id<Place>> for Subject { fn from(value: Id<Place>) -> Self { Self::Place(value) } }
impl From<Id<TransferKey>> for Subject { fn from(value: Id<TransferKey>) -> Self { Self::Transfer(value) } }
impl From<Id<super::lexical::LexicalScope>> for Subject { fn from(value: Id<super::lexical::LexicalScope>) -> Self { Self::LexicalScope(value) } }
impl From<Id<super::lexical::BindingEvent>> for Subject { fn from(value: Id<super::lexical::BindingEvent>) -> Self { Self::BindingEvent(value) } }
impl From<Id<super::lexical::LexicalTarget>> for Subject { fn from(value: Id<super::lexical::LexicalTarget>) -> Self { Self::LexicalTarget(value) } }
/// Additional nominal subject dependencies beyond common provenance and condition operands.
/// The derive supplies them from actual field types; source assertions do not load transfer keys.
pub trait SubjectValue {
    fn inputs() -> Vec<ValidationInput>;
    fn append_subjects(&self, subjects: &mut Vec<Subject>);
}
impl<const C: i16> SubjectValue for ArmId<super::documents::DocumentNode,C> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::documents::DocumentNode>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::DocumentNode { id: self.id(),tag: C }); }
}
impl SubjectValue for EvidenceSourceSpanId {
    fn inputs() -> Vec<ValidationInput> { vec![] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::SourceSpan(self.id())); }
}
impl SubjectValue for Id<super::flow::FlowUse> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::flow::FlowUse>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::FlowUse(*self)); }
}
impl SubjectValue for Id<super::flow::FlowDefinition> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::flow::FlowDefinition>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::FlowDefinition(*self)); }
}
impl SubjectValue for Id<super::flow::ReachingDefinition> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::flow::FlowDefinition>(&["id"]),ValidationInput::of::<super::flow::ReachingDefinition>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::ReachingDefinition(*self)); }
}
impl SubjectValue for Id<super::types::TypeTerm> {
    fn inputs() -> Vec<ValidationInput> { super::types::TypeIndex::inputs() }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::TypeTerm(*self)); }
}
impl SubjectValue for Id<super::types::TypeVariable> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::types::TypeVariable>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push(Subject::TypeVariable(*self)); }
}
impl<T: SubjectValue> SubjectValue for Option<T> {
    fn inputs() -> Vec<ValidationInput> { T::inputs() }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { if let Some(value) = self { value.append_subjects(subjects); } }
}
macro_rules! simple_subject { ($($ty:ty),+) => { $(impl SubjectValue for Id<$ty> {
    fn inputs() -> Vec<ValidationInput> { vec![] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
})+ }; }
simple_subject!(Occurrence,SourceArtifact,Module,CoverageScope);
impl SubjectValue for Id<Place> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<PlaceRoot>(&["id"]),ValidationInput::of::<Place>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
}
impl SubjectValue for Id<TransferKey> {
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
    fn inputs() -> Vec<ValidationInput> {
        let mut inputs = <Id<Place> as SubjectValue>::inputs(); inputs.push(ValidationInput::of::<TransferKey>(&["id"])); inputs
    }
}
impl SubjectValue for Id<super::lexical::LexicalScope> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::lexical::LexicalScope>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
}
impl SubjectValue for Id<super::lexical::BindingEvent> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::lexical::BindingEvent>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
}
impl SubjectValue for Id<super::lexical::LexicalTarget> {
    fn inputs() -> Vec<ValidationInput> { vec![ValidationInput::of::<super::lexical::BindingEvent>(&["id"]),ValidationInput::of::<super::lexical::LexicalTarget>(&["id"])] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
}
pub trait Assertion: Record {
    const FAMILY: FactFamily;
    fn qualification(&self) -> Id<AssertionQualification>;
    fn subjects(&self) -> Vec<Subject>;
    fn subject_inputs() -> Vec<ValidationInput> { Vec::new() }
}
pub struct SupportAttribution { pub run: Id<ProviderRun>, pub surface: Id<ProviderSurface>, pub evidence: Id<Evidence>, pub fidelity: Fidelity }
pub trait Support: Record {
    type Assertion: Assertion;
    fn assertion(&self) -> Id<Self::Assertion>;
    fn attribution(&self) -> SupportAttribution;
}

fn qualification_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "qualification_condition_context", inputs: vec![
        ValidationInput::of::<EvaluationAtom>(&["id"]), ValidationInput::of::<ConditionNode>(&["id"]),
        ValidationInput::of::<Condition>(&["id"]), ValidationInput::of::<AssertionQualification>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(QualificationCheck { charge: StateCharge::new(budget, "qualification_condition_context"), ..Default::default() })) }]
}
#[derive(Default)]
struct QualificationCheck {
    charge: StateCharge,
    atoms: ChargedMap<Id<EvaluationAtom>, Id<AnalysisContext>>,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    contexts: ChargedMap<Id<Condition>, BTreeSet<Id<AnalysisContext>>>,
}
impl InvariantCheck for QualificationCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? { self.atoms.insert(&mut self.charge, row.id(), row.context)?; }
        } else if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? { self.nodes.insert(&mut self.charge, row.id(), row)?; }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                // Each root's closure is bounded by the kernel's node and atom limits.
                let closure = super::conditions::kernel::closure(row.root, &self.nodes)?;
                let mut contexts = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom, .. } = &self.nodes[&id] {
                        contexts.insert(*self.atoms.get(atom).ok_or_else(|| invalid("condition atom missing"))?);
                    }
                }
                self.contexts.insert(&mut self.charge, row.id(), contexts)?;
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                let contexts = self.contexts.get(&row.condition).ok_or_else(|| invalid("qualification condition missing"))?;
                if contexts.iter().any(|context| *context != row.context) { return Err(invalid("qualification crosses condition contexts")); }
            }
        } else { return Err(invalid("undeclared qualification validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}
fn evidence_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "evidence_source_bounds", inputs: vec![
        ValidationInput::of::<SourceArtifact>(&["id"]), ValidationInput::of::<Evidence>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(EvidenceCheck { charge: StateCharge::new(budget, "evidence_source_bounds"), lengths: ChargedMap::default() })) }]
}
struct EvidenceCheck { charge: StateCharge, lengths: ChargedMap<Id<SourceArtifact>, i64> }
impl InvariantCheck for EvidenceCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? { self.lengths.insert(&mut self.charge, row.id(), row.byte_len)?; }
        } else if relation == Evidence::NAME {
            for row in Evidence::decode(batch)? {
                if let Evidence::SourceSpan { source, end, .. } = row {
                    if !self.lengths.get(&source).is_some_and(|len| end <= *len) { return Err(invalid("evidence outside source bytes")); }
                }
            }
        } else { return Err(invalid("undeclared evidence validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}

/// The concrete generated support type supplies its nominal target; no relation-name join exists.
pub fn support_invariants<A: Assertion, S: Support<Assertion=A>>() -> Vec<Invariant> {
    let mut inputs = vec![
        ValidationInput::of::<SourceArtifact>(&["id"]), ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<Module>(&["id"]), ValidationInput::of::<CorpusLibrary>(&["id"]),
        ValidationInput::of::<PlaceRoot>(&["id"]), ValidationInput::of::<Place>(&["id"]),
        ValidationInput::of::<Predicate>(&["id"]), ValidationInput::of::<EvaluationAtom>(&["id"]), ValidationInput::of::<ConditionNode>(&["id"]),
        ValidationInput::of::<Condition>(&["id"]),
        ValidationInput::of::<InputDistribution>(&["id"]), ValidationInput::of::<CoverageScope>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]), ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<RunFamily>(&["id"]), ValidationInput::of::<ProviderSurface>(&["id"]),
        ValidationInput::of::<Evidence>(&["id"]),
    ];
    for input in A::subject_inputs() { if !inputs.iter().any(|existing| existing.name() == input.name()) { inputs.push(input); } }
    inputs.extend([ValidationInput::of::<A>(&["id"]),ValidationInput::of::<S>(&["id"])]);
    vec![Invariant { name: S::NAME,inputs,create: std::sync::Arc::new(|budget| Box::new(SupportCheck::<A,S>::new(budget))) }]
}
struct SupportCheck<A: Assertion, S: Support<Assertion=A>> {
    charge: StateCharge,
    types: super::types::TypeIndex,
    flow_uses: ChargedMap<Id<super::flow::FlowUse>,super::flow::FlowUse>,
    flow_definitions: ChargedMap<Id<super::flow::FlowDefinition>,super::flow::FlowDefinition>,
    reaching: ChargedMap<Id<super::flow::ReachingDefinition>,super::flow::ReachingDefinition>,
    document_nodes: ChargedMap<Id<super::documents::DocumentNode>,super::documents::DocumentNode>,
    lexical_scopes: ChargedMap<Id<super::lexical::LexicalScope>,Id<Occurrence>>,
    bindings: ChargedMap<Id<super::lexical::BindingEvent>,Id<Occurrence>>,
    lexical_targets: ChargedMap<Id<super::lexical::LexicalTarget>,super::lexical::LexicalTarget>,
    ownership: super::ownership::ScopeIndex,
    places: ChargedMap<Id<Place>,Place>, roots: ChargedMap<Id<PlaceRoot>,PlaceRoot>, transfers: ChargedMap<Id<TransferKey>,TransferKey>,
    occurrences: ChargedMap<Id<Occurrence>, Id<SourceArtifact>>,
    guards: super::conditions::rebase::GuardIndex,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    conditions: ChargedMap<Id<Condition>, BTreeSet<Id<SourceArtifact>>>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    runs: ChargedMap<Id<ProviderRun>, ProviderRun>,
    families: ChargedSet<(Id<ProviderRun>, FactFamily)>,
    surfaces: ChargedMap<Id<ProviderSurface>, ProviderSurface>,
    evidence: ChargedMap<Id<Evidence>, Evidence>,
    assertions: ChargedMap<Id<A>, A>, supported: ChargedSet<Id<A>>, marker: PhantomData<S>,
}
impl<A: Assertion, S: Support<Assertion=A>> SupportCheck<A,S> {
    fn new(budget: &super::resources::ResourceBudget) -> Self { Self { charge: StateCharge::new(budget,S::NAME),
        types: super::types::TypeIndex::new(budget,S::NAME),flow_uses: Default::default(),flow_definitions: Default::default(),reaching: Default::default(),document_nodes: Default::default(), lexical_scopes: Default::default(), bindings: Default::default(), lexical_targets: Default::default(),
        places: Default::default(), roots: Default::default(), transfers: Default::default(), ownership: super::ownership::ScopeIndex::new(budget,S::NAME), occurrences: Default::default(),
        qualifications: Default::default(),
        guards: super::conditions::rebase::GuardIndex::new(budget,S::NAME), nodes: Default::default(), conditions: Default::default(),
        runs: Default::default(), families: Default::default(), surfaces: Default::default(), evidence: Default::default(),
        assertions: Default::default(), supported: Default::default(), marker: PhantomData } }
    fn source(&self, subject: Subject) -> Result<Option<Id<SourceArtifact>>, ModelError> {
        Ok(match subject {
            Subject::TypeTerm(_) | Subject::TypeVariable(_) => None,
            Subject::SourceSpan(id) => match self.evidence.get(&id) {
                Some(Evidence::SourceSpan { source,.. }) => Some(*source),
                _ => return Err(invalid("document subject span missing or wrong evidence subtype")),
            },
            Subject::DocumentNode { id,tag } => {
                let node = self.document_nodes.get(&id).filter(|n| n.tag() == tag).ok_or_else(|| invalid("document subject missing or wrong subtype"))?;
                self.source(Subject::SourceSpan(node.span().id()))?
            },
            Subject::LexicalScope(id) => self.source(Subject::Occurrence(*self.lexical_scopes.get(&id).ok_or_else(|| invalid("lexical scope absent"))?))?,
            Subject::BindingEvent(id) => self.source(Subject::Occurrence(*self.bindings.get(&id).ok_or_else(|| invalid("binding event absent"))?))?,
            Subject::LexicalTarget(id) => match self.lexical_targets.get(&id).ok_or_else(|| invalid("lexical target absent"))? {
                super::lexical::LexicalTarget::Binding { event } => self.source(Subject::BindingEvent(*event))?,
                super::lexical::LexicalTarget::Builtin { .. } | super::lexical::LexicalTarget::Unresolved { .. } => None,
            },
            Subject::Artifact(id) => Some(id),
            Subject::Occurrence(id) => Some(*self.occurrences.get(&id).ok_or_else(|| invalid("assertion occurrence missing"))?),
            Subject::Module(id) => Some(self.ownership.module_source(id)?),
            Subject::Scope(_) => None,
            Subject::Place(id) => {
                let place = self.places.get(&id).ok_or_else(|| invalid("assertion place absent"))?;
                let root = self.roots.get(&place.root).ok_or_else(|| invalid("assertion place root absent"))?;
                self.source(match root {
                    PlaceRoot::Formal { declaration } => Subject::Occurrence(*declaration),
                    PlaceRoot::Receiver { callable } | PlaceRoot::Return { callable } | PlaceRoot::Yield { callable }
                        | PlaceRoot::Raise { callable } => Subject::Occurrence(*callable),
                    PlaceRoot::Field { class,.. } => Subject::Occurrence(*class),
                    PlaceRoot::Global { module,.. } => Subject::Module(*module),
                    PlaceRoot::Occurrence { occurrence } => Subject::Occurrence(*occurrence),
                })?
            }
            Subject::Transfer(_) | Subject::FlowUse(_) | Subject::FlowDefinition(_) | Subject::ReachingDefinition(_) => return Err(invalid("composite subject requires all sources")),
        })
    }
    fn subject_sources(&self, subject: Subject) -> Result<Vec<Id<SourceArtifact>>,ModelError> {
        let subjects = match subject {
            Subject::Transfer(id) => {
                let transfer = self.transfers.get(&id).ok_or_else(|| invalid("assertion transfer key absent"))?;
                let mut subjects = vec![Subject::Place(transfer.input),Subject::Place(transfer.output)];
                if let Some(site) = transfer.call_site { subjects.push(Subject::Occurrence(site)); }
                subjects
            },
            Subject::FlowUse(id) => {
                let event = self.flow_uses.get(&id).ok_or_else(|| invalid("flow use absent"))?;
                vec![Subject::Occurrence(event.occurrence),Subject::Place(event.place)]
            },
            Subject::FlowDefinition(id) => {
                let event = self.flow_definitions.get(&id).ok_or_else(|| invalid("flow definition absent"))?;
                vec![Subject::Occurrence(event.occurrence),Subject::Place(event.place)]
            },
            Subject::ReachingDefinition(id) => match self.reaching.get(&id).ok_or_else(|| invalid("reaching definition absent"))? {
                super::flow::ReachingDefinition::Bound { definition } => return self.subject_sources(Subject::FlowDefinition(*definition)),
                super::flow::ReachingDefinition::Unbound => vec![],
            },
            other => vec![other],
        };
        let mut sources = Vec::new();
        for subject in subjects { sources.extend(self.source(subject)?); }
        Ok(sources)
    }

    fn check_support(&mut self, support: S) -> Result<(), ModelError> {
        let assertion = self.assertions.get(&support.assertion()).ok_or_else(|| invalid("supported assertion missing"))?;
        let q = self.qualifications.get(&assertion.qualification()).ok_or_else(|| invalid("assertion qualification missing"))?;
        let scope = self.ownership.scope(q.scope)?;
        let provenance = support.attribution();
        let run = self.runs.get(&provenance.run).ok_or_else(|| invalid("support invocation missing"))?;
        let surface = self.surfaces.get(&provenance.surface).ok_or_else(|| invalid("support surface missing"))?;
        if q.context != run.context || surface.provider != run.provider || surface.family != A::FAMILY
            || !self.families.contains(&(provenance.run,A::FAMILY)) { return Err(invalid("support disagrees with qualified assertion or invocation")); }
        if !self.ownership.owns_scope(run.input,scope)? { return Err(invalid("support invocation does not own assertion scope")); }
        for source in self.conditions.get(&q.condition).ok_or_else(|| invalid("assertion condition missing"))? {
            if !self.ownership.acquired(run.input,*source)? || !self.ownership.within(*source,scope)? {
                return Err(invalid("condition evaluation crosses assertion scope or invocation input"));
            }
        }
        let mut sources = BTreeSet::new();
        for subject in assertion.subjects() {
            match subject {
                Subject::TypeTerm(id) => self.types.term_support(id,run,provenance.fidelity)?,
                Subject::TypeVariable(id) => self.types.variable_owner(id,run)?,
                _ => {},
            }
            if let Subject::Scope(id) = subject {
                if id != q.scope { return Err(invalid("assertion subject scope differs")); }
            } else {
                for source in self.subject_sources(subject)? {
                    if !self.ownership.within(source,scope)? || !self.ownership.acquired(run.input,source)? { return Err(invalid("assertion crosses declared scope or invocation input")); }
                    sources.insert(source);
                }
            }
        }
        let evidence = self.evidence.get(&provenance.evidence).ok_or_else(|| invalid("support evidence missing"))?;
        let source = match evidence {
            Evidence::Occurrence { occurrence } => self.source(Subject::Occurrence(*occurrence))?,
            Evidence::SourceSpan { source, .. } => Some(*source),
            Evidence::Invocation { run: evidence_run } => {
                if *evidence_run != provenance.run { return Err(invalid("evidence names another invocation")); } None
            }
        };
        if let Some(source) = source {
            if !self.ownership.within(source,scope)? || !self.ownership.acquired(run.input,source)? || (!sources.is_empty() && !sources.contains(&source)) {
                return Err(invalid("support evidence crosses assertion scope or input"));
            }
        }
        self.supported.insert(&mut self.charge,support.assertion())?; Ok(())
    }
}
impl<A: Assertion, S: Support<Assertion=A>> InvariantCheck for SupportCheck<A,S> {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.ownership.visit(relation,batch)? || self.types.visit_input(relation,batch)? || self.guards.visit_input(relation,batch)? {}
        else if relation == Occurrence::NAME { for r in Occurrence::decode(batch)? { self.occurrences.insert(&mut self.charge,r.id(),r.source)?; } }
        else if relation == ConditionNode::NAME { for r in ConditionNode::decode(batch)? { self.nodes.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == Condition::NAME {
            for r in Condition::decode(batch)? {
                // Each root's closure is bounded by the kernel's node and atom limits.
                let closure = super::conditions::kernel::closure(r.root,&self.nodes)?;
                let mut sources = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom,.. } = &self.nodes[&id] {
                        for guard in self.guards.lineage(*atom)? {
                            sources.insert(*self.occurrences.get(&guard.evaluation).ok_or_else(|| invalid("condition evaluation occurrence missing"))?);
                            if let Some(operand) = guard.operand { sources.extend(self.source(Subject::Place(operand))?); }
                        }
                    }
                }
                self.conditions.insert(&mut self.charge,r.id(),sources)?;
            }
        }
        else if relation == AssertionQualification::NAME { for r in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == ProviderRun::NAME { for r in ProviderRun::decode(batch)? { self.runs.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == RunFamily::NAME { for r in RunFamily::decode(batch)? { self.families.insert(&mut self.charge,(r.run,r.family))?; } }
        else if relation == ProviderSurface::NAME { for r in ProviderSurface::decode(batch)? { self.surfaces.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == Evidence::NAME { for r in Evidence::decode(batch)? { self.evidence.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == PlaceRoot::NAME { for r in PlaceRoot::decode(batch)? { self.roots.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == Place::NAME { for r in Place::decode(batch)? { self.places.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == TransferKey::NAME { for r in TransferKey::decode(batch)? { self.transfers.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == super::flow::FlowUse::NAME { for r in super::flow::FlowUse::decode(batch)? { self.flow_uses.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == super::flow::FlowDefinition::NAME { for r in super::flow::FlowDefinition::decode(batch)? { self.flow_definitions.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == super::flow::ReachingDefinition::NAME { for r in super::flow::ReachingDefinition::decode(batch)? { self.reaching.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == super::documents::DocumentNode::NAME { for r in super::documents::DocumentNode::decode(batch)? { self.document_nodes.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == super::lexical::LexicalScope::NAME { for r in super::lexical::LexicalScope::decode(batch)? { self.lexical_scopes.insert(&mut self.charge,r.id(),r.owner)?; } }
        else if relation == super::lexical::BindingEvent::NAME { for r in super::lexical::BindingEvent::decode(batch)? { self.bindings.insert(&mut self.charge,r.id(),r.site)?; } }
        else if relation == super::lexical::LexicalTarget::NAME { for r in super::lexical::LexicalTarget::decode(batch)? { self.lexical_targets.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == A::NAME { for r in A::decode(batch)? { self.assertions.insert(&mut self.charge,r.id(),r)?; } }
        else if relation == S::NAME { for r in S::decode(batch)? { self.check_support(r)?; } }
        else { return Err(invalid("undeclared support validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.assertions.len() != self.supported.len() { return Err(invalid("assertion has no attributed support")); }
        Ok(())
    }
}
