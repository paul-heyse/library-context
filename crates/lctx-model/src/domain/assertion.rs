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
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use crate::{Domain, DomainCode, DomainSum};
use super::{*, attribution::*, source::*, input::*, conditions::*};

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
pub enum Subject { Occurrence(Id<Occurrence>), Artifact(Id<SourceArtifact>), Module(Id<Module>), Scope(Id<CoverageScope>) }
impl From<Id<Occurrence>> for Subject { fn from(value: Id<Occurrence>) -> Self { Self::Occurrence(value) } }
impl From<Id<SourceArtifact>> for Subject { fn from(value: Id<SourceArtifact>) -> Self { Self::Artifact(value) } }
impl From<Id<Module>> for Subject { fn from(value: Id<Module>) -> Self { Self::Module(value) } }
impl From<Id<CoverageScope>> for Subject { fn from(value: Id<CoverageScope>) -> Self { Self::Scope(value) } }
pub trait Assertion: Record {
    const FAMILY: FactFamily;
    fn qualification(&self) -> Id<AssertionQualification>;
    fn subjects(&self) -> Vec<Subject>;
}
pub struct SupportAttribution { pub run: Id<ProviderRun>, pub surface: Id<ProviderSurface>, pub evidence: Id<Evidence> }
pub trait Support: Record {
    type Assertion: Assertion;
    fn assertion(&self) -> Id<Self::Assertion>;
    fn attribution(&self) -> SupportAttribution;
}

fn qualification_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "qualification_condition_context", inputs: vec![
        ValidationInput::of::<EvaluationAtom>(&["id"]), ValidationInput::of::<ConditionNode>(&["id"]),
        ValidationInput::of::<Condition>(&["id"]), ValidationInput::of::<AssertionQualification>(&["id"]),
    ], create: || Box::new(QualificationCheck::default()) }]
}
#[derive(Default)]
struct QualificationCheck {
    atoms: BTreeMap<Id<EvaluationAtom>, Id<AnalysisContext>>,
    nodes: BTreeMap<Id<ConditionNode>, ConditionNode>,
    contexts: BTreeMap<Id<Condition>, BTreeSet<Id<AnalysisContext>>>,
    visits: usize,
}
impl InvariantCheck for QualificationCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? { self.atoms.insert(row.id(), row.context); }
        } else if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? { self.nodes.insert(row.id(), row); }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                let closure = super::conditions::kernel::closure(row.root, &self.nodes)?;
                self.visits += closure.len();
                if self.visits > 1_000_000 { return Err(invalid("qualification closure work budget exceeded")); }
                let mut contexts = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom, .. } = &self.nodes[&id] {
                        contexts.insert(*self.atoms.get(atom).ok_or_else(|| invalid("condition atom missing"))?);
                    }
                }
                self.contexts.insert(row.id(), contexts);
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                let contexts = self.contexts.get(&row.condition).ok_or_else(|| invalid("qualification condition missing"))?;
                if contexts.iter().any(|context| *context != row.context) { return Err(invalid("qualification crosses condition contexts")); }
            }
        } else { return Err(invalid("undeclared qualification validation input")); }
        if self.atoms.len() + self.nodes.len() + self.contexts.len() > 1_000_000 { return Err(invalid("qualification cardinality budget exceeded")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}
fn evidence_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "evidence_source_bounds", inputs: vec![
        ValidationInput::of::<SourceArtifact>(&["id"]), ValidationInput::of::<Evidence>(&["id"]),
    ], create: || Box::new(EvidenceCheck { lengths: BTreeMap::new() }) }]
}
struct EvidenceCheck { lengths: BTreeMap<Id<SourceArtifact>, i64> }
impl InvariantCheck for EvidenceCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? { self.lengths.insert(row.id(), row.byte_len); }
            if self.lengths.len() > 1_000_000 { return Err(invalid("evidence cardinality budget exceeded")); }
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
    vec![Invariant { name: S::NAME, inputs: vec![
        ValidationInput::of::<SourceArtifact>(&["id"]), ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<Module>(&["id"]), ValidationInput::of::<CorpusLibrary>(&["id"]),
        ValidationInput::of::<EvaluationAtom>(&["id"]), ValidationInput::of::<ConditionNode>(&["id"]),
        ValidationInput::of::<Condition>(&["id"]),
        ValidationInput::of::<InputDistribution>(&["id"]), ValidationInput::of::<CoverageScope>(&["id"]),
        ValidationInput::of::<AssertionQualification>(&["id"]), ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<RunFamily>(&["id"]), ValidationInput::of::<ProviderSurface>(&["id"]),
        ValidationInput::of::<Evidence>(&["id"]), ValidationInput::of::<A>(&["id"]), ValidationInput::of::<S>(&["id"]),
    ], create: || Box::new(SupportCheck::<A,S>::new()) }]
}
struct SupportCheck<A: Assertion, S: Support<Assertion=A>> {
    sources: BTreeMap<Id<SourceArtifact>, Id<InputRevision>>,
    occurrences: BTreeMap<Id<Occurrence>, Id<SourceArtifact>>,
    modules: BTreeMap<Id<Module>, Id<SourceArtifact>>,
    corpus: BTreeSet<(Id<InputRevision>, Id<InputRevision>)>,
    atoms: BTreeMap<Id<EvaluationAtom>, Id<Occurrence>>,
    nodes: BTreeMap<Id<ConditionNode>, ConditionNode>,
    conditions: BTreeMap<Id<Condition>, BTreeSet<Id<SourceArtifact>>>,
    closure_visits: usize,
    distributions: BTreeSet<(Id<InputRevision>, Id<Release>)>,
    scopes: BTreeMap<Id<CoverageScope>, CoverageScope>,
    qualifications: BTreeMap<Id<AssertionQualification>, AssertionQualification>,
    runs: BTreeMap<Id<ProviderRun>, ProviderRun>,
    families: BTreeSet<(Id<ProviderRun>, FactFamily)>,
    surfaces: BTreeMap<Id<ProviderSurface>, ProviderSurface>,
    evidence: BTreeMap<Id<Evidence>, Evidence>,
    assertions: BTreeMap<Id<A>, A>, supported: BTreeSet<Id<A>>, marker: PhantomData<S>,
}
impl<A: Assertion, S: Support<Assertion=A>> SupportCheck<A,S> {
    fn new() -> Self { Self { sources: BTreeMap::new(), occurrences: BTreeMap::new(), modules: BTreeMap::new(),
        corpus: BTreeSet::new(), distributions: BTreeSet::new(), scopes: BTreeMap::new(), qualifications: BTreeMap::new(),
        atoms: BTreeMap::new(), nodes: BTreeMap::new(), conditions: BTreeMap::new(), closure_visits: 0,
        runs: BTreeMap::new(), families: BTreeSet::new(), surfaces: BTreeMap::new(), evidence: BTreeMap::new(),
        assertions: BTreeMap::new(), supported: BTreeSet::new(), marker: PhantomData } }
    fn input(&self, source: Id<SourceArtifact>) -> Result<Id<InputRevision>, ModelError> {
        self.sources.get(&source).copied().ok_or_else(|| invalid("assertion source missing"))
    }
    fn acquired(&self, run_input: Id<InputRevision>, source: Id<SourceArtifact>) -> Result<bool, ModelError> {
        let input = self.input(source)?; Ok(input == run_input || self.corpus.contains(&(run_input,input)))
    }
    fn source(&self, subject: Subject) -> Result<Option<Id<SourceArtifact>>, ModelError> {
        Ok(match subject {
            Subject::Artifact(id) => Some(id),
            Subject::Occurrence(id) => Some(*self.occurrences.get(&id).ok_or_else(|| invalid("assertion occurrence missing"))?),
            Subject::Module(id) => Some(*self.modules.get(&id).ok_or_else(|| invalid("assertion module missing"))?),
            Subject::Scope(_) => None,
        })
    }
    fn within(&self, source: Id<SourceArtifact>, scope: &CoverageScope) -> Result<bool, ModelError> {
        Ok(match scope {
            CoverageScope::Artifact { artifact } => *artifact == source,
            CoverageScope::Module { module } => self.modules.get(module) == Some(&source),
            CoverageScope::Input { input } => self.acquired(*input,source)?,
            CoverageScope::Release { release } => self.distributions.contains(&(self.input(source)?,*release)),
        })
    }
    fn check_support(&mut self, support: S) -> Result<(), ModelError> {
        let assertion = self.assertions.get(&support.assertion()).ok_or_else(|| invalid("supported assertion missing"))?;
        let q = self.qualifications.get(&assertion.qualification()).ok_or_else(|| invalid("assertion qualification missing"))?;
        let scope = self.scopes.get(&q.scope).ok_or_else(|| invalid("assertion scope missing"))?;
        let provenance = support.attribution();
        let run = self.runs.get(&provenance.run).ok_or_else(|| invalid("support invocation missing"))?;
        let surface = self.surfaces.get(&provenance.surface).ok_or_else(|| invalid("support surface missing"))?;
        if q.context != run.context || surface.provider != run.provider || surface.family != A::FAMILY
            || !self.families.contains(&(provenance.run,A::FAMILY)) { return Err(invalid("support disagrees with qualified assertion or invocation")); }
        let owns_scope = match scope {
            CoverageScope::Input { input } => *input == run.input,
            CoverageScope::Artifact { artifact } => self.acquired(run.input,*artifact)?,
            CoverageScope::Module { module } => self.acquired(run.input,*self.modules.get(module).ok_or_else(|| invalid("scope module missing"))?)?,
            CoverageScope::Release { release } => self.distributions.contains(&(run.input,*release))
                || self.corpus.iter().any(|(corpus,input)| *corpus == run.input && self.distributions.contains(&(*input,*release))),
        };
        if !owns_scope { return Err(invalid("support invocation does not own assertion scope")); }
        for source in self.conditions.get(&q.condition).ok_or_else(|| invalid("assertion condition missing"))? {
            if !self.acquired(run.input,*source)? || !self.within(*source,scope)? {
                return Err(invalid("condition evaluation crosses assertion scope or invocation input"));
            }
        }
        let mut sources = BTreeSet::new();
        for subject in assertion.subjects() {
            if let Subject::Scope(id) = subject {
                if id != q.scope { return Err(invalid("assertion subject scope differs")); }
            } else if let Some(source) = self.source(subject)? {
                if !self.within(source,scope)? || !self.acquired(run.input,source)? { return Err(invalid("assertion crosses declared scope or invocation input")); }
                sources.insert(source);
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
            if !self.within(source,scope)? || !self.acquired(run.input,source)? || (!sources.is_empty() && !sources.contains(&source)) {
                return Err(invalid("support evidence crosses assertion scope or input"));
            }
        }
        self.supported.insert(support.assertion()); Ok(())
    }
}
impl<A: Assertion, S: Support<Assertion=A>> InvariantCheck for SupportCheck<A,S> {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME { for r in SourceArtifact::decode(batch)? { self.sources.insert(r.id(),r.input); } }
        else if relation == Occurrence::NAME { for r in Occurrence::decode(batch)? { self.occurrences.insert(r.id(),r.source); } }
        else if relation == Module::NAME { for r in Module::decode(batch)? { self.modules.insert(r.id(),r.source); } }
        else if relation == CorpusLibrary::NAME { for r in CorpusLibrary::decode(batch)? { self.corpus.insert((r.corpus,r.library)); } }
        else if relation == EvaluationAtom::NAME { for r in EvaluationAtom::decode(batch)? { self.atoms.insert(r.id(),r.evaluation); } }
        else if relation == ConditionNode::NAME { for r in ConditionNode::decode(batch)? { self.nodes.insert(r.id(),r); } }
        else if relation == Condition::NAME {
            for r in Condition::decode(batch)? {
                let closure = super::conditions::kernel::closure(r.root,&self.nodes)?;
                self.closure_visits += closure.len();
                if self.closure_visits > 1_000_000 { return Err(invalid("support condition closure budget exceeded")); }
                let mut sources = BTreeSet::new();
                for id in closure {
                    if let ConditionNode::Branch { atom,.. } = &self.nodes[&id] {
                        let occurrence = self.atoms.get(atom).ok_or_else(|| invalid("condition atom missing"))?;
                        sources.insert(*self.occurrences.get(occurrence).ok_or_else(|| invalid("condition evaluation occurrence missing"))?);
                    }
                }
                self.conditions.insert(r.id(),sources);
            }
        }
        else if relation == InputDistribution::NAME { for r in InputDistribution::decode(batch)? { self.distributions.insert((r.input,r.release)); } }
        else if relation == CoverageScope::NAME { for r in CoverageScope::decode(batch)? { self.scopes.insert(r.id(),r); } }
        else if relation == AssertionQualification::NAME { for r in AssertionQualification::decode(batch)? { self.qualifications.insert(r.id(),r); } }
        else if relation == ProviderRun::NAME { for r in ProviderRun::decode(batch)? { self.runs.insert(r.id(),r); } }
        else if relation == RunFamily::NAME { for r in RunFamily::decode(batch)? { self.families.insert((r.run,r.family)); } }
        else if relation == ProviderSurface::NAME { for r in ProviderSurface::decode(batch)? { self.surfaces.insert(r.id(),r); } }
        else if relation == Evidence::NAME { for r in Evidence::decode(batch)? { self.evidence.insert(r.id(),r); } }
        else if relation == A::NAME { for r in A::decode(batch)? { self.assertions.insert(r.id(),r); } }
        else if relation == S::NAME { for r in S::decode(batch)? { self.check_support(r)?; } }
        else { return Err(invalid("undeclared support validation input")); }
        let entries = self.sources.len()+self.occurrences.len()+self.modules.len()+self.corpus.len()+self.distributions.len()+self.scopes.len()
            +self.qualifications.len()+self.runs.len()+self.families.len()+self.surfaces.len()+self.evidence.len()+self.assertions.len()+self.supported.len();
        if entries + self.atoms.len()+self.nodes.len()+self.conditions.len() > 3_000_000 { return Err(invalid("support validation cardinality budget exceeded")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.assertions.len() != self.supported.len() { return Err(invalid("assertion has no attributed support")); }
        Ok(())
    }
}
