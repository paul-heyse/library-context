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
use super::{
    attribution::*,
    conditions::*,
    input::*,
    source::*,
    value::{Place, PlaceRoot, Predicate},
    *,
};
use crate::{Domain, DomainCode, DomainSum};
use std::collections::BTreeSet;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Approximation {
    Exact = 0,
    Over = 1,
    Under = 2,
    Mixed = 3,
    Unknown = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "assertion_qualifications", invariants = qualification_invariants)]
pub struct AssertionQualification {
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub condition: Id<Condition>,
    #[model(key)]
    pub modality: Modality,
    #[model(key)]
    pub approximation: Approximation,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_surfaces", validate = validate_surface)]
pub struct ProviderSurface {
    #[model(key, provenance)]
    pub provider: Id<Provider>,
    #[model(key)]
    pub family: FactFamily,
    #[model(key)]
    pub name: String,
}
fn validate_surface(row: &ProviderSurface) -> Result<(), ModelError> {
    if row.name.is_empty() {
        return Err(invalid("invalid provider surface"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "evidence", validate = validate_evidence, invariants = evidence_invariants)]
pub enum Evidence {
    #[model(code = 0)]
    Occurrence { occurrence: Id<Occurrence> },
    #[model(code = 1)]
    SourceSpan {
        source: Id<SourceArtifact>,
        start: i64,
        end: i64,
    },
    /// Explicit invocation evidence for facts with no source declaration. Never invent a span.
    #[model(code = 2)]
    Invocation { run: Id<ProviderRun> },
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_evidence(row: &Evidence) -> Result<(), ModelError> {
    if let Evidence::SourceSpan { start, end, .. } = row
        && (*start < 0 || end < start)
    {
        return Err(invalid("invalid evidence span"));
    }
    Ok(())
}
#[derive(Debug, Clone, Copy)]
pub enum Subject {
    EvaluationAtom(Id<EvaluationAtom>),
    FlowValue(Id<super::flow::FlowValueObservation>),
    FlowCallPath(Id<super::flow::FlowCallPath>),
    TypeTerm(Id<super::types::TypeTerm>),
    TypeVariable(Id<super::types::TypeVariable>),
    FlowUse(Id<super::flow::FlowUse>),
    FlowDefinition(Id<super::flow::FlowDefinition>),
    ReachingDefinition(Id<super::flow::ReachingDefinition>),
    DocumentNode {
        id: Id<super::documents::DocumentNode>,
        tag: i16,
    },
    SourceSpan(Id<Evidence>),
    Occurrence(Id<Occurrence>),
    Artifact(Id<SourceArtifact>),
    Module(Id<Module>),
    Scope(Id<CoverageScope>),
    Place(Id<Place>),
    Transfer(derivation::RowRef),
    LexicalScope(Id<super::lexical::LexicalScope>),
    BindingEvent(Id<super::lexical::BindingEvent>),
    LexicalTarget(Id<super::lexical::LexicalTarget>),
    /// A native symbol: stated only by its own provider in its own context, and located in its
    /// module's source when that module is acquired.
    Symbol(Id<super::calls::ProviderSymbol>),
    Callable(Id<super::calls::ProviderCallable>),
    Destination(Id<super::calls::CallDestination>),
    /// A provider module: an acquired one is located in its source; a bundled, namespace or
    /// unresolved one belongs to its provider (and context) and has no captured source.
    ProviderModule(Id<super::calls::ProviderModule>),
    /// A public name's origin: a traced one is its provider module; an untraced one names nothing.
    ExportOrigin(Id<super::symbols::ExportOrigin>),
}
impl From<Id<Occurrence>> for Subject {
    fn from(value: Id<Occurrence>) -> Self {
        Self::Occurrence(value)
    }
}
impl From<Id<SourceArtifact>> for Subject {
    fn from(value: Id<SourceArtifact>) -> Self {
        Self::Artifact(value)
    }
}
impl From<Id<Module>> for Subject {
    fn from(value: Id<Module>) -> Self {
        Self::Module(value)
    }
}
impl From<Id<CoverageScope>> for Subject {
    fn from(value: Id<CoverageScope>) -> Self {
        Self::Scope(value)
    }
}
impl From<Id<Place>> for Subject {
    fn from(value: Id<Place>) -> Self {
        Self::Place(value)
    }
}
impl From<Id<super::lexical::LexicalScope>> for Subject {
    fn from(value: Id<super::lexical::LexicalScope>) -> Self {
        Self::LexicalScope(value)
    }
}
impl From<Id<super::lexical::BindingEvent>> for Subject {
    fn from(value: Id<super::lexical::BindingEvent>) -> Self {
        Self::BindingEvent(value)
    }
}
impl From<Id<super::lexical::LexicalTarget>> for Subject {
    fn from(value: Id<super::lexical::LexicalTarget>) -> Self {
        Self::LexicalTarget(value)
    }
}
/// Additional nominal subject dependencies beyond common provenance and condition operands.
/// The derive supplies them from actual field types; source assertions do not load transfer keys.
pub trait SubjectValue {
    fn inputs() -> Vec<ValidationInput>;
    fn append_subjects(&self, subjects: &mut Vec<Subject>);
}
impl<const C: i16> SubjectValue for ArmId<super::documents::DocumentNode, C> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::documents::DocumentNode>(&[
            "id",
        ])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::DocumentNode {
            id: self.id(),
            tag: C,
        });
    }
}
impl SubjectValue for EvidenceSourceSpanId {
    fn inputs() -> Vec<ValidationInput> {
        vec![]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::SourceSpan(self.id()));
    }
}
impl SubjectValue for Id<EvaluationAtom> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<EvaluationAtom>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::EvaluationAtom(*self));
    }
}
impl SubjectValue for Id<super::flow::FlowValueObservation> {
    fn inputs() -> Vec<ValidationInput> {
        let mut inputs = <Id<super::flow::FlowUse> as SubjectValue>::inputs();
        inputs.push(ValidationInput::of::<super::flow::FlowValueObservation>(&[
            "id",
        ]));
        inputs
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::FlowValue(*self));
    }
}
impl SubjectValue for Id<super::flow::FlowCallPath> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::flow::FlowCallPath>(&["id"]),
            ValidationInput::of::<super::flow::FlowCallStep>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::FlowCallPath(*self));
    }
}
impl SubjectValue for Id<super::flow::FlowUse> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::flow::FlowUse>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::FlowUse(*self));
    }
}
impl SubjectValue for Id<super::flow::FlowDefinition> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::flow::FlowDefinition>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::FlowDefinition(*self));
    }
}
impl SubjectValue for Id<super::flow::ReachingDefinition> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::flow::FlowDefinition>(&["id"]),
            ValidationInput::of::<super::flow::ReachingDefinition>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::ReachingDefinition(*self));
    }
}
impl SubjectValue for Id<super::types::TypeTerm> {
    fn inputs() -> Vec<ValidationInput> {
        super::types::TypeIndex::inputs()
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::TypeTerm(*self));
    }
}
impl SubjectValue for Id<super::types::TypeVariable> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::types::TypeVariable>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::TypeVariable(*self));
    }
}
impl<T: SubjectValue> SubjectValue for Option<T> {
    fn inputs() -> Vec<ValidationInput> {
        T::inputs()
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        if let Some(value) = self {
            value.append_subjects(subjects);
        }
    }
}
macro_rules! simple_subject { ($($ty:ty),+) => { $(impl SubjectValue for Id<$ty> {
    fn inputs() -> Vec<ValidationInput> { vec![] }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) { subjects.push((*self).into()); }
})+ }; }
simple_subject!(Occurrence, SourceArtifact, Module, CoverageScope);
impl SubjectValue for Id<Place> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<PlaceRoot>(&["id"]),
            ValidationInput::of::<Place>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push((*self).into());
    }
}
impl SubjectValue for Id<super::lexical::LexicalScope> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::lexical::LexicalScope>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push((*self).into());
    }
}
impl SubjectValue for Id<super::lexical::BindingEvent> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::lexical::BindingEvent>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push((*self).into());
    }
}
impl SubjectValue for Id<super::calls::ProviderModule> {
    fn inputs() -> Vec<ValidationInput> {
        vec![ValidationInput::of::<super::calls::ProviderModule>(&["id"])]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::ProviderModule(*self));
    }
}
impl SubjectValue for Id<super::symbols::ExportOrigin> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::calls::ProviderModule>(&["id"]),
            ValidationInput::of::<super::symbols::ExportOrigin>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::ExportOrigin(*self));
    }
}
impl SubjectValue for Id<super::calls::ProviderSymbol> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::calls::ProviderModule>(&["id"]),
            ValidationInput::of::<super::calls::ProviderSymbol>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::Symbol(*self));
    }
}
impl SubjectValue for Id<super::calls::CallDestination> {
    fn inputs() -> Vec<ValidationInput> {
        let mut inputs = <Id<super::calls::ProviderCallable> as SubjectValue>::inputs();
        inputs.push(ValidationInput::of::<super::calls::CallDestination>(&[
            "id",
        ]));
        inputs
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::Destination(*self));
    }
}
impl SubjectValue for Id<super::calls::ProviderCallable> {
    fn inputs() -> Vec<ValidationInput> {
        let mut inputs = <Id<super::calls::ProviderSymbol> as SubjectValue>::inputs();
        inputs.push(ValidationInput::of::<super::calls::ProviderCallable>(&[
            "id",
        ]));
        inputs
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push(Subject::Callable(*self));
    }
}
impl SubjectValue for Id<super::lexical::LexicalTarget> {
    fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::lexical::BindingEvent>(&["id"]),
            ValidationInput::of::<super::lexical::LexicalTarget>(&["id"]),
        ]
    }
    fn append_subjects(&self, subjects: &mut Vec<Subject>) {
        subjects.push((*self).into());
    }
}
pub trait Assertion: Record {
    const FAMILY: FactFamily;
    /// The fidelity every support must state, when the proposition admits only one (a display
    /// rendering is display-only and never establishes structure).
    const FIDELITY: Option<Fidelity> = None;
    fn qualification(&self) -> Id<AssertionQualification>;
    /// What the proposition is about: each is located inside the qualification's scope.
    fn subjects(&self) -> Vec<Subject>;
    /// What the proposition refers to elsewhere (a re-export's origin, an inherited field's
    /// declaration): each belongs to the asserting provider and to bytes the invocation captured,
    /// wherever they lie.
    fn referents(&self) -> Vec<Subject> {
        Vec::new()
    }
    fn subject_inputs() -> Vec<ValidationInput> {
        Vec::new()
    }
}
pub struct SupportAttribution {
    pub run: Id<ProviderRun>,
    pub surface: Id<ProviderSurface>,
    pub evidence: Id<Evidence>,
    pub fidelity: Fidelity,
}
pub trait Support: Record {
    const DERIVED: bool = false;
    type Assertion: Assertion;
    type Source: DerivedSupportSource;
    fn assertion(&self) -> Id<Self::Assertion>;
    fn attribution(&self) -> Option<SupportAttribution>;
    fn source(&self) -> Option<Id<Self::Source>> {
        None
    }
}

/// A generated companion has exactly one nominal owner source and its immutable input frame.
pub struct DerivedSupportFrame {
    pub input: Id<super::input::InputRevision>,
    pub context: Id<super::attribution::AnalysisContext>,
    pub qualification: Id<AssertionQualification>,
    pub evidence: super::analysis::support::SourceFacts,
}
pub trait DerivedSupportSource: Sized + Send + Sync + 'static {
    fn inputs() -> Vec<ValidationInput>;
    fn index(budget: &super::resources::ResourceBudget) -> Box<dyn DerivedSupportIndex<Self>>;
}
pub trait DerivedSupportIndex<S>: Send + Sync {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError>;
    fn frame(&self, source: Id<S>) -> Result<DerivedSupportFrame, ModelError>;
}
pub enum NoDerivedSource {}
struct NoDerivedIndex;
impl DerivedSupportSource for NoDerivedSource {
    fn inputs() -> Vec<ValidationInput> {
        Vec::new()
    }
    fn index(_: &super::resources::ResourceBudget) -> Box<dyn DerivedSupportIndex<Self>> {
        Box::new(NoDerivedIndex)
    }
}
impl DerivedSupportIndex<NoDerivedSource> for NoDerivedIndex {
    fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        Ok(false)
    }
    fn frame(&self, _: Id<NoDerivedSource>) -> Result<DerivedSupportFrame, ModelError> {
        Err(invalid("native companion cannot name a derived source"))
    }
}

fn qualification_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "qualification_condition_context",
        inputs: vec![
            ValidationInput::of::<EvaluationAtom>(&["id"]),
            ValidationInput::of::<ConditionNode>(&["id"]),
            ValidationInput::of::<Condition>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(QualificationCheck {
                charge: StateCharge::new(budget, "qualification_condition_context"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct QualificationCheck {
    charge: StateCharge,
    atoms: ChargedMap<Id<EvaluationAtom>, Id<AnalysisContext>>,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    contexts: ChargedMap<Id<Condition>, BTreeSet<Id<AnalysisContext>>>,
}
impl InvariantCheck for QualificationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? {
                self.atoms.insert(&mut self.charge, row.id(), row.context)?;
            }
        } else if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? {
                self.nodes.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                // Each root's closure is bounded by the kernel's node and atom limits.
                let closure = super::conditions::kernel::closure(row.root, &self.nodes)?;
                let mut contexts = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom, .. } = &self.nodes[&id] {
                        contexts.insert(
                            *self
                                .atoms
                                .get(atom)
                                .ok_or_else(|| invalid("condition atom missing"))?,
                        );
                    }
                }
                self.contexts.insert(&mut self.charge, row.id(), contexts)?;
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                let contexts = self
                    .contexts
                    .get(&row.condition)
                    .ok_or_else(|| invalid("qualification condition missing"))?;
                if contexts.iter().any(|context| *context != row.context) {
                    return Err(invalid("qualification crosses condition contexts"));
                }
            }
        } else {
            return Err(invalid("undeclared qualification validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
fn evidence_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "evidence_source_bounds",
        inputs: vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<Evidence>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(EvidenceCheck {
                charge: StateCharge::new(budget, "evidence_source_bounds"),
                lengths: ChargedMap::default(),
            })
        }),
    }]
}
struct EvidenceCheck {
    charge: StateCharge,
    lengths: ChargedMap<Id<SourceArtifact>, i64>,
}
impl InvariantCheck for EvidenceCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.lengths
                    .insert(&mut self.charge, row.id(), row.byte_len)?;
            }
        } else if relation == Evidence::NAME {
            for row in Evidence::decode(batch)? {
                if let Evidence::SourceSpan { source, end, .. } = row
                    && !self.lengths.get(&source).is_some_and(|len| end <= *len)
                {
                    return Err(invalid("evidence outside source bytes"));
                }
            }
        } else {
            return Err(invalid("undeclared evidence validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

/// The concrete generated support type supplies its nominal target; no relation-name join exists.
pub fn support_invariants<A: Assertion, S: Support<Assertion = A>>() -> Vec<Invariant> {
    let mut inputs = vec![
        ValidationInput::of::<SourceArtifact>(&["id"]),
        ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<Module>(&["id"]),
        ValidationInput::of::<CorpusLibrary>(&["id"]),
        ValidationInput::of::<PlaceRoot>(&["id"]),
        ValidationInput::of::<Place>(&["id"]),
        ValidationInput::of::<Predicate>(&["id"]),
        ValidationInput::of::<EvaluationAtom>(&["id"]),
        ValidationInput::of::<ConditionNode>(&["id"]),
        ValidationInput::of::<Condition>(&["id"]),
        ValidationInput::of::<InputDistribution>(&["id"]),
        ValidationInput::of::<CoverageScope>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]),
        ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<RunFamily>(&["id"]),
        ValidationInput::of::<ProviderSurface>(&["id"]),
        ValidationInput::of::<Evidence>(&["id"]),
    ];
    inputs.extend(S::Source::inputs());
    for input in A::subject_inputs() {
        if !inputs
            .iter()
            .any(|existing| existing.name() == input.name())
        {
            inputs.push(input);
        }
    }
    inputs.extend([
        ValidationInput::of::<A>(&["id"]),
        ValidationInput::of::<S>(&["id"]),
    ]);
    vec![Invariant {
        name: S::NAME,
        inputs,
        create: std::sync::Arc::new(|budget| Box::new(SupportCheck::<A, S>::new(budget))),
    }]
}
struct SupportCheck<A: Assertion, S: Support<Assertion = A>> {
    charge: StateCharge,
    types: super::types::TypeIndex,
    flow_values:
        ChargedMap<Id<super::flow::FlowValueObservation>, super::flow::FlowValueObservation>,
    flow_paths: ChargedMap<Id<super::flow::FlowCallPath>, Vec<Id<Occurrence>>>,
    flow_uses: ChargedMap<Id<super::flow::FlowUse>, super::flow::FlowUse>,
    flow_definitions: ChargedMap<Id<super::flow::FlowDefinition>, super::flow::FlowDefinition>,
    reaching: ChargedMap<Id<super::flow::ReachingDefinition>, super::flow::ReachingDefinition>,
    document_nodes: ChargedMap<Id<super::documents::DocumentNode>, super::documents::DocumentNode>,
    lexical_scopes: ChargedMap<Id<super::lexical::LexicalScope>, Id<Occurrence>>,
    bindings: ChargedMap<Id<super::lexical::BindingEvent>, Id<Occurrence>>,
    lexical_targets: ChargedMap<Id<super::lexical::LexicalTarget>, super::lexical::LexicalTarget>,
    export_origins:
        ChargedMap<Id<super::symbols::ExportOrigin>, Option<Id<super::calls::ProviderModule>>>,
    destinations: ChargedMap<Id<super::calls::CallDestination>, super::calls::CallDestination>,
    callables: ChargedMap<Id<super::calls::ProviderCallable>, super::calls::ProviderCallable>,
    ownership: super::ownership::ScopeIndex,
    places: ChargedMap<Id<Place>, Place>,
    roots: ChargedMap<Id<PlaceRoot>, PlaceRoot>,
    transfers: ChargedMap<derivation::RowRef, transfer::TransferDescriptor>,
    occurrences: ChargedMap<Id<Occurrence>, Id<SourceArtifact>>,
    guards: super::conditions::rebase::GuardIndex,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    conditions: ChargedMap<Id<Condition>, BTreeSet<Id<SourceArtifact>>>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    runs: ChargedMap<Id<ProviderRun>, ProviderRun>,
    families: ChargedSet<(Id<ProviderRun>, FactFamily)>,
    surfaces: ChargedMap<Id<ProviderSurface>, ProviderSurface>,
    evidence: ChargedMap<Id<Evidence>, Evidence>,
    assertions: ChargedMap<Id<A>, A>,
    supported: ChargedSet<Id<A>>,
    derived: Box<dyn DerivedSupportIndex<S::Source>>,
    marker: PhantomData<S>,
}
impl<A: Assertion, S: Support<Assertion = A>> SupportCheck<A, S> {
    fn new(budget: &super::resources::ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, S::NAME),
            types: super::types::TypeIndex::new(budget, S::NAME),
            flow_values: Default::default(),
            flow_paths: Default::default(),
            flow_uses: Default::default(),
            flow_definitions: Default::default(),
            reaching: Default::default(),
            document_nodes: Default::default(),
            lexical_scopes: Default::default(),
            bindings: Default::default(),
            lexical_targets: Default::default(),
            export_origins: Default::default(),
            callables: Default::default(),
            destinations: Default::default(),
            places: Default::default(),
            roots: Default::default(),
            transfers: Default::default(),
            ownership: super::ownership::ScopeIndex::new(budget, S::NAME),
            occurrences: Default::default(),
            qualifications: Default::default(),
            guards: super::conditions::rebase::GuardIndex::new(budget, S::NAME),
            nodes: Default::default(),
            conditions: Default::default(),
            runs: Default::default(),
            families: Default::default(),
            surfaces: Default::default(),
            evidence: Default::default(),
            assertions: Default::default(),
            supported: Default::default(),
            derived: S::Source::index(budget),
            marker: PhantomData,
        }
    }
    fn source(&self, subject: Subject) -> Result<Option<Id<SourceArtifact>>, ModelError> {
        Ok(match subject {
            Subject::TypeTerm(_) | Subject::TypeVariable(_) => None,
            Subject::SourceSpan(id) => match self.evidence.get(&id) {
                Some(Evidence::SourceSpan { source, .. }) => Some(*source),
                _ => {
                    return Err(invalid(
                        "document subject span missing or wrong evidence subtype",
                    ));
                }
            },
            Subject::DocumentNode { id, tag } => {
                let node = self
                    .document_nodes
                    .get(&id)
                    .filter(|n| n.tag() == tag)
                    .ok_or_else(|| invalid("document subject missing or wrong subtype"))?;
                self.source(Subject::SourceSpan(node.span().id()))?
            }
            Subject::LexicalScope(id) => self.source(Subject::Occurrence(
                *self
                    .lexical_scopes
                    .get(&id)
                    .ok_or_else(|| invalid("lexical scope absent"))?,
            ))?,
            Subject::BindingEvent(id) => self.source(Subject::Occurrence(
                *self
                    .bindings
                    .get(&id)
                    .ok_or_else(|| invalid("binding event absent"))?,
            ))?,
            Subject::LexicalTarget(id) => match self
                .lexical_targets
                .get(&id)
                .ok_or_else(|| invalid("lexical target absent"))?
            {
                super::lexical::LexicalTarget::Binding { event } => {
                    self.source(Subject::BindingEvent(*event))?
                }
                super::lexical::LexicalTarget::Builtin { .. }
                | super::lexical::LexicalTarget::Unresolved { .. } => None,
            },
            Subject::Artifact(id) => Some(id),
            Subject::Occurrence(id) => Some(
                *self
                    .occurrences
                    .get(&id)
                    .ok_or_else(|| invalid("assertion occurrence missing"))?,
            ),
            Subject::Module(id) => Some(self.ownership.module_source(id)?),
            Subject::Scope(_) => None,
            Subject::Symbol(id) => {
                self.source(Subject::ProviderModule(self.types.symbol(id)?.module))?
            }
            Subject::Destination(id) => match self
                .destinations
                .get(&id)
                .ok_or_else(|| invalid("call destination absent"))?
            {
                super::calls::CallDestination::Resolved { symbol }
                | super::calls::CallDestination::Overrides { symbol } => {
                    self.source(Subject::Symbol(*symbol))?
                }
                super::calls::CallDestination::Callable { callable } => {
                    self.source(Subject::Callable(*callable))?
                }
                super::calls::CallDestination::Unresolved { .. }
                | super::calls::CallDestination::SyntheticFormatting => None,
            },
            Subject::Callable(id) => {
                let callable = self
                    .callables
                    .get(&id)
                    .ok_or_else(|| invalid("provider callable absent"))?;
                self.source(Subject::ProviderModule(
                    callable.module(|id| self.types.symbol(id).cloned())?,
                ))?
            }
            Subject::ExportOrigin(id) => match self
                .export_origins
                .get(&id)
                .ok_or_else(|| invalid("export origin absent"))?
            {
                Some(module) => self.source(Subject::ProviderModule(*module))?,
                None => None,
            },
            Subject::ProviderModule(id) => match self.types.module(id)? {
                super::calls::ProviderModule::Acquired { module } => {
                    Some(self.ownership.module_source(*module)?)
                }
                _ => None,
            },
            Subject::Place(id) => {
                let place = self
                    .places
                    .get(&id)
                    .ok_or_else(|| invalid("assertion place absent"))?;
                let root = self
                    .roots
                    .get(&place.root)
                    .ok_or_else(|| invalid("assertion place root absent"))?;
                self.source(match root {
                    PlaceRoot::Formal { declaration } | PlaceRoot::Entry { declaration } => {
                        Subject::Occurrence(*declaration)
                    }
                    PlaceRoot::Receiver { callable }
                    | PlaceRoot::Return { callable }
                    | PlaceRoot::Yield { callable }
                    | PlaceRoot::Raise { callable } => Subject::Occurrence(*callable),
                    PlaceRoot::Field { class, .. } => Subject::Occurrence(*class),
                    PlaceRoot::Global { module, .. } => Subject::Module(*module),
                    PlaceRoot::Occurrence { occurrence }
                    | PlaceRoot::ClassOf { actual: occurrence }
                    | PlaceRoot::Local {
                        scope: occurrence, ..
                    } => Subject::Occurrence(*occurrence),
                })?
            }
            Subject::EvaluationAtom(_)
            | Subject::FlowValue(_)
            | Subject::FlowCallPath(_)
            | Subject::Transfer(_)
            | Subject::FlowUse(_)
            | Subject::FlowDefinition(_)
            | Subject::ReachingDefinition(_) => {
                return Err(invalid("composite subject requires all sources"));
            }
        })
    }
    fn subject_sources(&self, subject: Subject) -> Result<Vec<Id<SourceArtifact>>, ModelError> {
        let subjects = match subject {
            Subject::EvaluationAtom(id) => {
                let mut subjects = Vec::new();
                for atom in self.guards.lineage(id)? {
                    subjects.push(Subject::Occurrence(atom.evaluation));
                    if let Some(place) = atom.operand {
                        subjects.push(Subject::Place(place));
                    }
                }
                subjects
            }
            Subject::FlowCallPath(id) => self
                .flow_paths
                .get(&id)
                .ok_or_else(|| invalid("flow path absent"))?
                .iter()
                .copied()
                .map(Subject::Occurrence)
                .collect(),
            Subject::FlowValue(id) => {
                let value = self
                    .flow_values
                    .get(&id)
                    .ok_or_else(|| invalid("flow value absent"))?;
                let mut sources = self.subject_sources(Subject::FlowUse(value.use_))?;
                sources.extend(self.source(Subject::Occurrence(value.sink))?);
                return Ok(sources);
            }
            Subject::Transfer(id) => {
                let transfer = self
                    .transfers
                    .get(&id)
                    .ok_or_else(|| invalid("assertion transfer key absent"))?;
                let mut subjects = vec![
                    Subject::Place(transfer.input),
                    Subject::Place(transfer.output),
                ];
                if let Some(site) = transfer.call_site {
                    subjects.push(Subject::Occurrence(site));
                }
                subjects
            }
            Subject::FlowUse(id) => {
                let event = self
                    .flow_uses
                    .get(&id)
                    .ok_or_else(|| invalid("flow use absent"))?;
                vec![
                    Subject::Occurrence(event.occurrence),
                    Subject::Place(event.place),
                ]
            }
            Subject::FlowDefinition(id) => {
                let event = self
                    .flow_definitions
                    .get(&id)
                    .ok_or_else(|| invalid("flow definition absent"))?;
                vec![
                    Subject::Occurrence(event.occurrence),
                    Subject::Place(event.place),
                ]
            }
            Subject::ReachingDefinition(id) => match self
                .reaching
                .get(&id)
                .ok_or_else(|| invalid("reaching definition absent"))?
            {
                super::flow::ReachingDefinition::Bound { definition } => {
                    return self.subject_sources(Subject::FlowDefinition(*definition));
                }
                super::flow::ReachingDefinition::Unbound
                | super::flow::ReachingDefinition::Nested => vec![],
            },
            other => vec![other],
        };
        let mut sources = Vec::new();
        for subject in subjects {
            sources.extend(self.source(subject)?);
        }
        Ok(sources)
    }

    fn check_support(&mut self, support: S) -> Result<(), ModelError> {
        let assertion = self
            .assertions
            .get(&support.assertion())
            .ok_or_else(|| invalid("supported assertion missing"))?;
        let q = self
            .qualifications
            .get(&assertion.qualification())
            .ok_or_else(|| invalid("assertion qualification missing"))?;
        let scope = self.ownership.scope(q.scope)?;
        if let Some(source) = support.source() {
            let frame = self.derived.frame(source)?;
            if A::FAMILY == FactFamily::Flow {
                super::analysis::policy::behavioral_support(
                    frame.evidence.status,
                    frame.evidence.heuristic,
                )?;
            }
            if frame.qualification != assertion.qualification()
                || frame.context != q.context
                || !self.ownership.owns_scope(frame.input, scope)?
            {
                return Err(invalid(
                    "derived support changes qualification, scope or context",
                ));
            }
            for source in self
                .conditions
                .get(&q.condition)
                .ok_or_else(|| invalid("assertion condition missing"))?
            {
                if !self.ownership.acquired(frame.input, *source)?
                    || !self.ownership.within(*source, scope)?
                {
                    return Err(invalid("derived condition crosses scope/input"));
                }
            }
            for subject in assertion.subjects() {
                if let Subject::Scope(id) = subject
                    && id != q.scope
                {
                    return Err(invalid("derived assertion scope differs"));
                }
                for source in self.subject_sources(subject)? {
                    if !self.ownership.acquired(frame.input, source)?
                        || !self.ownership.within(source, scope)?
                    {
                        return Err(invalid("derived assertion crosses scope/input"));
                    }
                }
            }
            for subject in assertion.referents() {
                for source in self.subject_sources(subject)? {
                    if !self.ownership.acquired(frame.input, source)? {
                        return Err(invalid("derived referent crosses invocation input"));
                    }
                }
            }
            self.supported
                .insert(&mut self.charge, support.assertion())?;
            return Ok(());
        }
        let provenance = support
            .attribution()
            .ok_or_else(|| invalid("support has no attributed source"))?;
        let run = self
            .runs
            .get(&provenance.run)
            .ok_or_else(|| invalid("support invocation missing"))?;
        let surface = self
            .surfaces
            .get(&provenance.surface)
            .ok_or_else(|| invalid("support surface missing"))?;
        if A::FIDELITY.is_some_and(|required| required != provenance.fidelity) {
            return Err(invalid("assertion requires another support fidelity"));
        }
        if q.context != run.context
            || surface.provider != run.provider
            || surface.family != A::FAMILY
            || !self.families.contains(&(provenance.run, A::FAMILY))
        {
            return Err(invalid(
                "support disagrees with qualified assertion or invocation",
            ));
        }
        if !self.ownership.owns_scope(run.input, scope)? {
            return Err(invalid("support invocation does not own assertion scope"));
        }
        for source in self
            .conditions
            .get(&q.condition)
            .ok_or_else(|| invalid("assertion condition missing"))?
        {
            if !self.ownership.acquired(run.input, *source)?
                || !self.ownership.within(*source, scope)?
            {
                return Err(invalid(
                    "condition evaluation crosses assertion scope or invocation input",
                ));
            }
        }
        let mut sources = BTreeSet::new();
        let located = assertion.subjects().into_iter().map(|s| (s, true));
        for (subject, located) in
            located.chain(assertion.referents().into_iter().map(|s| (s, false)))
        {
            let subject = match subject {
                Subject::Destination(id) => match self
                    .destinations
                    .get(&id)
                    .ok_or_else(|| invalid("call destination absent"))?
                {
                    super::calls::CallDestination::Resolved { symbol }
                    | super::calls::CallDestination::Overrides { symbol } => {
                        Subject::Symbol(*symbol)
                    }
                    super::calls::CallDestination::Callable { callable } => {
                        Subject::Callable(*callable)
                    }
                    super::calls::CallDestination::Unresolved { .. }
                    | super::calls::CallDestination::SyntheticFormatting => continue,
                },
                Subject::ExportOrigin(id) => match self
                    .export_origins
                    .get(&id)
                    .ok_or_else(|| invalid("export origin absent"))?
                {
                    Some(module) => Subject::ProviderModule(*module),
                    None => continue,
                },
                subject => subject,
            };
            match subject {
                Subject::EvaluationAtom(id) => {
                    if self
                        .guards
                        .lineage(id)?
                        .iter()
                        .any(|a| a.context != q.context)
                    {
                        return Err(invalid("a leaf atom belongs to its qualified context"));
                    }
                }
                Subject::TypeTerm(id) => self.types.term_support(id, run, provenance.fidelity)?,
                Subject::TypeVariable(id) => self.types.variable_owner(id, run)?,
                Subject::Callable(id) => {
                    use super::calls::ProviderCallable;
                    let callable = self
                        .callables
                        .get(&id)
                        .ok_or_else(|| invalid("provider callable absent"))?;
                    let owner = match callable {
                        ProviderCallable::ModuleBody {
                            provider, context, ..
                        } => (*provider, *context),
                        ProviderCallable::Symbol { symbol }
                        | ProviderCallable::DecoratorApplication { function: symbol }
                        | ProviderCallable::ClassBody { class: symbol } => {
                            let symbol = self.types.symbol(*symbol)?;
                            (symbol.provider, symbol.context)
                        }
                    };
                    if owner != (run.provider, q.context) {
                        return Err(invalid(
                            "a callable is stated only by its own provider and context",
                        ));
                    }
                    callable.module(|id| self.types.symbol(id).cloned())?;
                }
                Subject::Symbol(id) => {
                    let symbol = self.types.symbol(id)?;
                    if (symbol.provider, symbol.context) != (run.provider, q.context) {
                        return Err(invalid(
                            "a symbol is stated only by its own provider and context",
                        ));
                    }
                }
                Subject::ProviderModule(id) => match self.types.module(id)? {
                    super::calls::ProviderModule::Acquired { .. } => {}
                    super::calls::ProviderModule::Bundled { provider, .. }
                        if *provider == run.provider => {}
                    super::calls::ProviderModule::Namespace {
                        provider, context, ..
                    }
                    | super::calls::ProviderModule::Unresolved {
                        provider, context, ..
                    } if (*provider, *context) == (run.provider, q.context) => {}
                    _ => {
                        return Err(invalid(
                            "a provider module is stated only by its own provider and context",
                        ));
                    }
                },
                _ => {}
            }
            if let Subject::Scope(id) = subject {
                if id != q.scope || !located {
                    return Err(invalid("assertion subject scope differs"));
                }
            } else {
                for source in self.subject_sources(subject)? {
                    if !self.ownership.acquired(run.input, source)? {
                        return Err(invalid(
                            "assertion crosses declared scope or invocation input",
                        ));
                    }
                    if located {
                        if !self.ownership.within(source, scope)? {
                            return Err(invalid(
                                "assertion crosses declared scope or invocation input",
                            ));
                        }
                        sources.insert(source);
                    }
                }
            }
        }
        let evidence = self
            .evidence
            .get(&provenance.evidence)
            .ok_or_else(|| invalid("support evidence missing"))?;
        let source = match evidence {
            Evidence::Occurrence { occurrence } => self.source(Subject::Occurrence(*occurrence))?,
            Evidence::SourceSpan { source, .. } => Some(*source),
            Evidence::Invocation { run: evidence_run } => {
                if *evidence_run != provenance.run {
                    return Err(invalid("evidence names another invocation"));
                }
                None
            }
        };
        if let Some(source) = source
            && (!self.ownership.within(source, scope)?
                || !self.ownership.acquired(run.input, source)?
                || (!sources.is_empty() && !sources.contains(&source)))
        {
            return Err(invalid("support evidence crosses assertion scope or input"));
        }
        self.supported
            .insert(&mut self.charge, support.assertion())?;
        Ok(())
    }
}
impl<A: Assertion, S: Support<Assertion = A>> InvariantCheck for SupportCheck<A, S> {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if self.derived.visit(relation, batch)? {
            return Ok(());
        }

        if self.ownership.visit(relation, batch)?
            || self.types.visit_input(relation, batch)?
            || self.guards.visit_input(relation, batch)?
        {
        } else if relation == Occurrence::NAME {
            for r in Occurrence::decode(batch)? {
                self.occurrences
                    .insert(&mut self.charge, r.id(), r.source)?;
            }
        } else if relation == super::calls::CallDestination::NAME {
            for r in super::calls::CallDestination::decode(batch)? {
                self.destinations.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::calls::ProviderCallable::NAME {
            for r in super::calls::ProviderCallable::decode(batch)? {
                self.callables.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == ConditionNode::NAME {
            for r in ConditionNode::decode(batch)? {
                self.nodes.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == Condition::NAME {
            for r in Condition::decode(batch)? {
                // Each root's closure is bounded by the kernel's node and atom limits.
                let closure = super::conditions::kernel::closure(r.root, &self.nodes)?;
                let mut sources = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom, .. } = &self.nodes[&id] {
                        for guard in self.guards.lineage(*atom)? {
                            sources.insert(*self.occurrences.get(&guard.evaluation).ok_or_else(
                                || invalid("condition evaluation occurrence missing"),
                            )?);
                            if let Some(operand) = guard.operand {
                                sources.extend(self.source(Subject::Place(operand))?);
                            }
                        }
                    }
                }
                self.conditions.insert(&mut self.charge, r.id(), sources)?;
            }
        } else if relation == AssertionQualification::NAME {
            for r in AssertionQualification::decode(batch)? {
                self.qualifications.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == ProviderRun::NAME {
            for r in ProviderRun::decode(batch)? {
                self.runs.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == RunFamily::NAME {
            for r in RunFamily::decode(batch)? {
                self.families.insert(&mut self.charge, (r.run, r.family))?;
            }
        } else if relation == ProviderSurface::NAME {
            for r in ProviderSurface::decode(batch)? {
                self.surfaces.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == Evidence::NAME {
            for r in Evidence::decode(batch)? {
                self.evidence.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == PlaceRoot::NAME {
            for r in PlaceRoot::decode(batch)? {
                self.roots.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == Place::NAME {
            for r in Place::decode(batch)? {
                self.places.insert(&mut self.charge, r.id(), r)?;
            }
        } else if A::subject_inputs()
            .iter()
            .any(|input| input.name() == relation)
            && let Some(rows) = transfer::subject_rows(relation, batch)?
        {
            for (id, descriptor) in rows {
                self.transfers.insert(&mut self.charge, id, descriptor)?;
            }
        } else if relation == super::flow::FlowValueObservation::NAME && relation != A::NAME {
            for r in super::flow::FlowValueObservation::decode(batch)? {
                self.flow_values.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::flow::FlowCallPath::NAME {
            for r in super::flow::FlowCallPath::decode(batch)? {
                self.flow_paths
                    .insert(&mut self.charge, r.id(), Vec::new())?;
            }
        } else if relation == super::flow::FlowCallStep::NAME {
            for r in super::flow::FlowCallStep::decode(batch)? {
                self.flow_paths.update(&mut self.charge, r.path, |values| {
                    values.extend([r.call, r.operand])
                })?;
            }
        } else if relation == super::flow::FlowUse::NAME {
            for r in super::flow::FlowUse::decode(batch)? {
                self.flow_uses.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::flow::FlowDefinition::NAME {
            for r in super::flow::FlowDefinition::decode(batch)? {
                self.flow_definitions.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::flow::ReachingDefinition::NAME {
            for r in super::flow::ReachingDefinition::decode(batch)? {
                self.reaching.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::documents::DocumentNode::NAME {
            for r in super::documents::DocumentNode::decode(batch)? {
                self.document_nodes.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::lexical::LexicalScope::NAME {
            for r in super::lexical::LexicalScope::decode(batch)? {
                self.lexical_scopes
                    .insert(&mut self.charge, r.id(), r.owner)?;
            }
        } else if relation == super::lexical::BindingEvent::NAME {
            for r in super::lexical::BindingEvent::decode(batch)? {
                self.bindings.insert(&mut self.charge, r.id(), r.site)?;
            }
        } else if relation == super::lexical::LexicalTarget::NAME {
            for r in super::lexical::LexicalTarget::decode(batch)? {
                self.lexical_targets.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == super::symbols::ExportOrigin::NAME {
            for r in super::symbols::ExportOrigin::decode(batch)? {
                let module = match &r {
                    super::symbols::ExportOrigin::Traced { module, .. } => Some(*module),
                    super::symbols::ExportOrigin::Untraced => None,
                };
                self.export_origins
                    .insert(&mut self.charge, r.id(), module)?;
            }
        } else if relation == A::NAME {
            for r in A::decode(batch)? {
                self.assertions.insert(&mut self.charge, r.id(), r)?;
            }
        } else if relation == S::NAME {
            for r in S::decode(batch)? {
                self.check_support(r)?;
            }
        } else {
            return Err(invalid("undeclared support validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.assertions.len() != self.supported.len() {
            return Err(invalid("assertion has no attributed support"));
        }
        Ok(())
    }
}
