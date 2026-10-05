//! Bounded structural domains under the pinned provider typing model.
//! Finite literals can exhaust that model; an observed class/MRO never closes runtime subclasses.
use crate::domain::{
    analysis::{
        self, local as publication,
        policy::{self, EvidenceStatus, SupportRole},
        support::SourceFacts,
    },
    assertion::*,
    attribution::*,
    calls::*,
    conditions::entry::EntryData,
    flow::*,
    normalized::{Rows, entities::ResolutionStatus, links::*},
    symbols::*,
    types::*,
    value::*,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TheoryReason {
    MissingEvidence = 0,
    IncompatibleFrame = 1,
    IncompleteCoverage = 2,
    DisplayOnly = 3,
    OpaqueType = 4,
    TruncatedType = 5,
    RecursiveType = 6,
    UnsupportedType = 7,
    IncompleteMro = 8,
    OpenClassUniverse = 9,
    UnsupportedScalar = 10,
    UnsupportedPredicate = 11,
    AmbiguousOperand = 12,
    WorkLimit = 13,
    UnsupportedBuiltin = 14,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DomainCompleteness {
    FiniteUnderTypingModel = 0,
    OpenClasses = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PredicateResult {
    AlwaysTrueUnderTypingModel = 0,
    AlwaysFalseUnderTypingModel = 1,
    Mixed = 2,
    Unknown = 3,
    Uninhabited = 4,
}
#[macro_export]
macro_rules! local_theory_inputs {
    ($apply:ident) => {
        $apply! {
         terms:$crate::domain::types::TypeTerm,
         sequences:$crate::domain::types::TypeSequence,
         sequence_members:$crate::domain::types::TypeSequenceMember,
         type_observations:$crate::domain::types::TypeObservation,
         type_supports:$crate::domain::types::TypeSupport,
         type_queries:$crate::domain::types::TypeQueryObservation,
         type_query_supports:$crate::domain::types::TypeQuerySupport,
         literals:$crate::domain::value::Literal,
         sets:$crate::domain::value::LiteralSet,
         set_members:$crate::domain::value::LiteralSetMember,
         ancestry:$crate::domain::symbols::ClassAncestryObservation,
         ancestry_supports:$crate::domain::symbols::ClassAncestrySupport,
         symbol_sequences:$crate::domain::symbols::SymbolSequence,
         symbol_members:$crate::domain::symbols::SymbolSequenceMember,
         provider_modules:$crate::domain::calls::ProviderModule,
         operand_assessments:$crate::domain::normalized::links::TestOperandTypeAssessment,
         operand_links:$crate::domain::normalized::links::TestOperandTypeLink,
         call_syntax:$crate::domain::calls::CallSyntax,
         call_syntax_supports:$crate::domain::calls::CallSyntaxSupport,
         arguments:$crate::domain::calls::CallArgument,
         resolutions:$crate::domain::lexical::LexicalResolution,
         resolution_supports:$crate::domain::lexical::LexicalResolutionSupport,
         lexical_targets:$crate::domain::lexical::LexicalTarget,
         operand_coverage:$crate::domain::normalized::links::TestOperandCoverage,
        }
    };
}
macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{
 pub struct TheoryInventory {$(pub $field:Rows<$ty>,)*}
 impl TheoryInventory {pub fn new(budget:&resources::ResourceBudget)->Self{Self{$($field:Rows::new(budget),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
 pub fn validation_inputs()->Vec<ValidationInput>{let mut inputs=vec![$(ValidationInput::of::<$ty>(&["id"]),)*];for input in &mut inputs{if stages::is_vocabulary(input.name()){*input=input.clone().at_epoch(stages::PublicationBoundary::Facts);}}inputs}
 }
};}
crate::local_theory_inputs!(inputs);
pub struct TheoryData<'a> {
    pub entry: &'a EntryData,
    pub inventory: &'a TheoryInventory,
}
impl std::ops::Deref for TheoryData<'_> {
    type Target = TheoryInventory;
    fn deref(&self) -> &Self::Target {
        self.inventory
    }
}
impl TheoryData<'_> {
    pub fn validation_inputs() -> Vec<ValidationInput> {
        let mut inputs = EntryData::validation_inputs();
        inputs.extend(TheoryInventory::validation_inputs());
        for input in &mut inputs {
            if stages::is_vocabulary(input.name()) {
                *input = input.clone().at_epoch(stages::PublicationBoundary::Facts);
            }
        }
        inputs
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="local_type_domain_assessments",invariant_refs=theory_invariants_refs)]
pub struct TypeDomainAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub observation: Id<TypeObservation>,
    #[model(key)]
    pub support: Id<TypeSupport>,
    pub domain: Option<Id<TypeDomain>>,
    pub reason: Option<TheoryReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_type_domains", rule = "structural_type_domain")]
pub struct TypeDomain {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub observation: Id<TypeObservation>,
    #[model(key, premise)]
    pub support: Id<TypeSupport>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub completeness: DomainCompleteness,
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "local_type_domain_values")]
pub enum DomainValue {
    #[model(code = 0)]
    Literal { literal: Id<Literal> },
    #[model(code = 1)]
    Class { class: Id<ProviderSymbol> },
    #[model(code = 2)]
    None,
    #[model(code = 3)]
    ClassObject { class: Id<ProviderSymbol> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_type_domain_members")]
pub struct TypeDomainMember {
    #[model(key)]
    pub domain: Id<TypeDomain>,
    #[model(key)]
    pub value: Id<DomainValue>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_type_class_ancestry", rule = "complete_structural_mro")]
pub struct ClassDomainAncestry {
    #[model(key)]
    pub domain: Id<TypeDomain>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
    #[model(key, premise)]
    pub observation: Id<ClassAncestryObservation>,
    #[model(key, premise)]
    pub support: Id<ClassAncestrySupport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_type_class_members", rule = "structural_mro_member")]
pub struct ClassDomainMember {
    #[model(key)]
    pub ancestry: Id<ClassDomainAncestry>,
    #[model(key)]
    pub ancestor: Id<ProviderSymbol>,
    #[model(premise)]
    pub native_member: Option<Id<SymbolSequenceMember>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_scalar_predicate_assessments")]
pub struct ScalarAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key)]
    pub support: Id<FlowTestLeafSupport>,
    pub witness: Option<Id<TheoryWitness>>,
    pub reason: Option<TheoryReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "local_theory_witnesses", rule = "structural_scalar_predicate")]
pub struct TheoryWitness {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key, premise)]
    pub support: Id<FlowTestLeafSupport>,
    #[model(key, premise)]
    pub operand_link: Id<TestOperandTypeLink>,
    #[model(key, premise)]
    pub domain: Id<TypeDomain>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub builtin: Option<Id<BuiltinOperandWitness>>,
    pub result: PredicateResult,
    status: EvidenceStatus,
}
/// Exact native syntax and builtin resolution; display predicate text is never authority.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "local_builtin_operand_witnesses",
    rule = "native_builtin_operand"
)]
pub struct BuiltinOperandWitness {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key, premise)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key, premise)]
    pub call: Id<CallSyntax>,
    #[model(key, premise)]
    pub syntax_support: Id<CallSyntaxSupport>,
    #[model(key, premise)]
    pub callee_resolution: Id<lexical::LexicalResolution>,
    #[model(key, premise)]
    pub callee_support: Id<lexical::LexicalResolutionSupport>,
    #[model(key, premise)]
    pub class_observation: Id<TypeObservation>,
    #[model(key, premise)]
    pub class_support: Id<TypeSupport>,
    #[model(key, premise)]
    pub class_resolution: Id<lexical::LexicalResolution>,
    #[model(key, premise)]
    pub class_resolution_support: Id<lexical::LexicalResolutionSupport>,
    #[model(key, premise)]
    pub call_placement: Option<Id<syntax::SyntaxPlacement>>,
    #[model(key, premise)]
    pub call_placement_support: Option<Id<syntax::SyntaxPlacementSupport>>,
    #[model(key, premise)]
    pub placement: Option<Id<syntax::SyntaxPlacement>>,
    #[model(key, premise)]
    pub placement_support: Option<Id<syntax::SyntaxPlacementSupport>>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
}
/// A replayed builtin syntax/resolution proof, usable without assuming a finite typing domain.
/// Request values remain outside the typing observations and the canonical relation inventory.
pub struct CheckedBuiltinOperand {
    witness: BuiltinOperandWitness,
    class_name: String,
    operand_premises: [derivation::RowRef; 3],
}
impl CheckedBuiltinOperand {
    pub fn witness(&self) -> &BuiltinOperandWitness {
        &self.witness
    }
    pub fn class_name(&self) -> &str {
        &self.class_name
    }
    pub fn operand_premises(&self) -> &[derivation::RowRef; 3] {
        &self.operand_premises
    }
    pub fn derive(
        data: &TheoryData,
        invocation: &publication::AnalysisInvocation,
        leaf: Id<FlowTestLeafObservation>,
        native_support: Id<FlowTestLeafSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<Self, TheoryReason>, ModelError> {
        let result = (|| -> Result<Self, Refusal> {
            let leaf = need(&data.entry.leaves, leaf)?;
            let native = need(&data.entry.leaf_supports, native_support)?;
            if native.assertion != leaf.id()
                || native.origin != Origin::AnalyzerAssertion
                || native.mode != ExtractionMode::NativeTraversal
            {
                return Err(TheoryReason::MissingEvidence.into());
            }
            let q = need(&data.entry.qualifications, leaf.qualification)?;
            support(data, native, q, invocation, FactFamily::Flow)?;
            let mut assessments = data
                .operand_assessments
                .iter()
                .filter(|a| a.leaf == leaf.id());
            let assessment = assessments.next().ok_or(TheoryReason::MissingEvidence)?;
            if assessments.next().is_some() || assessment.status != ResolutionStatus::Resolved {
                return Err(TheoryReason::AmbiguousOperand.into());
            }
            let mut links = data
                .operand_links
                .iter()
                .filter(|l| l.assessment == assessment.id());
            let link = links.next().ok_or(TheoryReason::MissingEvidence)?;
            if links.next().is_some() {
                return Err(TheoryReason::AmbiguousOperand.into());
            }
            let observation = need(&data.type_observations, link.observation)?;
            if observation.role != TypeRole::TestOperand
                || Some(observation.subject) != leaf.operand
            {
                return Err(TheoryReason::IncompatibleFrame.into());
            }
            frame(
                data,
                need(&data.entry.qualifications, observation.qualification)?,
                invocation,
            )?;
            let atom = need(&data.entry.atoms, leaf.atom)?;
            if atom.context != invocation.context {
                return Err(TheoryReason::IncompatibleFrame.into());
            }
            let predicate = need(&data.entry.predicates, atom.predicate)?;
            if !matches!(
                predicate,
                Predicate::TypeIs { .. } | Predicate::IsInstance { .. }
            ) {
                return Err(TheoryReason::UnsupportedPredicate.into());
            }
            let witness = builtin_operand(data, invocation, leaf, predicate, budget)?;
            let class_name = need(&data.entry.symbols, witness.class)?.name.clone();
            Ok(Self {
                witness,
                class_name,
                operand_premises: [
                    derivation::RowRef::of(assessment.id()),
                    derivation::RowRef::of(link.id()),
                    derivation::RowRef::of(observation.id()),
                ],
            })
        })();
        match result {
            Ok(proof) => Ok(Ok(proof)),
            Err(Refusal::Boundary(reason)) => Ok(Err(reason)),
            Err(Refusal::Resource(error)) => Err(error),
        }
    }
}
fn same_node(data: &TheoryData, a: Id<source::Occurrence>, b: Id<source::Occurrence>) -> bool {
    match (data.entry.occurrences.get(a), data.entry.occurrences.get(b)) {
        (Some(a), Some(b)) => {
            a.source == b.source
                && a.structural_path == b.structural_path
                && a.syntax_kind == b.syntax_kind
                && a.start == b.start
                && a.end == b.end
        }
        _ => false,
    }
}
fn builtin_resolution<'a>(
    data: &'a TheoryData,
    invocation: &publication::AnalysisInvocation,
    occurrence: Id<source::Occurrence>,
    expected: &str,
) -> Result<
    (
        &'a lexical::LexicalResolution,
        &'a lexical::LexicalResolutionSupport,
    ),
    TheoryReason,
> {
    let mut resolutions = data.resolutions.iter().filter(|row| {
        same_node(data, row.read, occurrence)
            && data
                .entry
                .qualifications
                .get(row.qualification)
                .is_some_and(|q| q.context == invocation.context)
    });
    let row = resolutions.next().ok_or(TheoryReason::UnsupportedBuiltin)?;
    if resolutions.next().is_some()
        || row.captured
        || !matches!(need(&data.lexical_targets,row.target)?,lexical::LexicalTarget::Builtin{name,variable:false}if name==expected)
    {
        return Err(TheoryReason::UnsupportedBuiltin);
    }
    let q = need(&data.entry.qualifications, row.qualification)?;
    if q.condition != conditions::Diagram::always().id() {
        return Err(TheoryReason::UnsupportedBuiltin);
    }
    let mut supports = data
        .resolution_supports
        .iter()
        .filter(|s| s.assertion == row.id());
    let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
    if supports.next().is_some()
        || native.origin != Origin::DerivedAnalysis
        || native.mode != ExtractionMode::Recognizer
        || native.fidelity != Fidelity::NormalizedStructural
    {
        return Err(TheoryReason::MissingEvidence);
    }
    support(data, native, q, invocation, FactFamily::Lexical)?;
    Ok((row, native))
}
fn builtin_operand(
    data: &TheoryData,
    invocation: &publication::AnalysisInvocation,
    leaf: &FlowTestLeafObservation,
    predicate: &Predicate,
    budget: &resources::ResourceBudget,
) -> Result<BuiltinOperandWitness, Refusal> {
    let type_is = matches!(predicate, Predicate::TypeIs { .. });
    let expected = if type_is { "type" } else { "isinstance" };
    let operand = leaf.operand.ok_or(TheoryReason::MissingEvidence)?;
    let buffer = data
        .arguments
        .iter()
        .try_fold(0usize, |sum, row| {
            sum.checked_add(row.heap_bytes())
                .and_then(|n| n.checked_add(size_of::<CallArgument>() + size_of::<Actual>()))
        })
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| invalid("builtin operand buffer overflow"))?;
    let _buffer = budget.reserve("local_builtin_operand_buffer", buffer)?;
    let mut selected = None;
    for call in data.call_syntax.iter() {
        if call.in_annotation {
            continue;
        }
        let related = if type_is {
            data.entry.placements.iter().any(|p| {
                p.parent
                    .is_some_and(|parent| same_node(data, parent, leaf.test))
                    && same_node(data, p.occurrence, call.site)
                    && matches!(
                        p.field,
                        lexical::SyntaxField::Left | lexical::SyntaxField::Right
                    )
            })
        } else {
            same_node(data, call.site, leaf.test)
        };
        if !related {
            continue;
        }
        let mut arguments = data
            .arguments
            .iter()
            .filter(|row| row.call == call.id())
            .cloned()
            .collect::<Vec<_>>();
        arguments.sort_by_key(|row| row.ordinal);
        let actuals = call.actuals(&arguments)?;
        if actuals.len() != if type_is { 1 } else { 2 }
            || actuals.iter().any(|a| a.kind != ArgumentKind::Positional)
            || !same_node(data, actuals[0].occurrence, operand)
        {
            continue;
        }
        if selected.is_some() {
            return Err(TheoryReason::AmbiguousOperand.into());
        }
        selected = Some((call, actuals));
    }
    let (call, actuals) = selected.ok_or(TheoryReason::UnsupportedBuiltin)?;
    let mut call_placement = None;
    let mut call_placement_support = None;
    if type_is {
        let mut candidates = data.entry.placements.iter().filter(|p| {
            p.parent
                .is_some_and(|parent| same_node(data, parent, leaf.test))
                && same_node(data, p.occurrence, call.site)
                && matches!(
                    p.field,
                    lexical::SyntaxField::Left | lexical::SyntaxField::Right
                )
                && data
                    .entry
                    .qualifications
                    .get(p.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        });
        let p = candidates.next().ok_or(TheoryReason::UnsupportedBuiltin)?;
        if candidates.next().is_some() {
            return Err(TheoryReason::AmbiguousOperand.into());
        }
        let pq = need(&data.entry.qualifications, p.qualification)?;
        if pq.condition != conditions::Diagram::always().id() {
            return Err(TheoryReason::UnsupportedBuiltin.into());
        }
        let mut supports = data
            .entry
            .placement_supports
            .iter()
            .filter(|s| s.assertion == p.id());
        let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
        if supports.next().is_some()
            || native.origin != Origin::SourceObservation
            || native.mode != ExtractionMode::NativeTraversal
        {
            return Err(TheoryReason::MissingEvidence.into());
        }
        support(data, native, pq, invocation, FactFamily::Syntax)?;
        call_placement = Some(p.id());
        call_placement_support = Some(native.id());
    }
    let sq = need(&data.entry.qualifications, call.qualification)?;
    if sq.condition != conditions::Diagram::always().id() {
        return Err(TheoryReason::UnsupportedBuiltin.into());
    }
    let mut syntaxes = data
        .call_syntax_supports
        .iter()
        .filter(|s| s.assertion == call.id());
    let native_syntax = syntaxes.next().ok_or(TheoryReason::MissingEvidence)?;
    if syntaxes.next().is_some()
        || native_syntax.origin != Origin::SourceObservation
        || native_syntax.mode != ExtractionMode::NativeTraversal
    {
        return Err(TheoryReason::MissingEvidence.into());
    }
    support(data, native_syntax, sq, invocation, FactFamily::Syntax)?;
    let (callee, callee_support) = builtin_resolution(data, invocation, call.callee, expected)?;
    let mut placement = None;
    let mut placement_support = None;
    let class_occurrence = if type_is {
        let mut candidates = data.entry.placements.iter().filter(|p| {
            p.parent
                .is_some_and(|parent| same_node(data, parent, leaf.test))
                && !same_node(data, p.occurrence, call.site)
                && matches!(
                    p.field,
                    lexical::SyntaxField::Left | lexical::SyntaxField::Right
                )
                && data
                    .entry
                    .qualifications
                    .get(p.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        });
        let p = candidates.next().ok_or(TheoryReason::UnsupportedBuiltin)?;
        if candidates.next().is_some() {
            return Err(TheoryReason::AmbiguousOperand.into());
        }
        let pq = need(&data.entry.qualifications, p.qualification)?;
        if pq.condition != conditions::Diagram::always().id() {
            return Err(TheoryReason::UnsupportedBuiltin.into());
        }
        let mut supports = data
            .entry
            .placement_supports
            .iter()
            .filter(|s| s.assertion == p.id());
        let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
        if supports.next().is_some()
            || native.origin != Origin::SourceObservation
            || native.mode != ExtractionMode::NativeTraversal
        {
            return Err(TheoryReason::MissingEvidence.into());
        }
        support(data, native, pq, invocation, FactFamily::Syntax)?;
        placement = Some(p.id());
        placement_support = Some(native.id());
        p.occurrence
    } else {
        actuals[1].occurrence
    };
    let mut observations = data.type_observations.iter().filter(|o| {
        same_node(data, o.subject, class_occurrence)
            && o.role
                == if type_is {
                    TypeRole::TestOperand
                } else {
                    TypeRole::Argument
                }
            && matches!(data.terms.get(o.term), Some(TypeTerm::ClassObject { .. }))
            && data
                .entry
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == invocation.context)
    });
    let observation = observations
        .next()
        .ok_or(TheoryReason::UnsupportedBuiltin)?;
    if observations.next().is_some() {
        return Err(TheoryReason::AmbiguousOperand.into());
    }
    let TypeTerm::ClassObject { class } = need(&data.terms, observation.term)? else {
        unreachable!()
    };
    let symbol = need(&data.entry.symbols, *class)?;
    if symbol.context != invocation.context
        || symbol.kind != SymbolKind::Class
        || if type_is {
            !matches!(symbol.name.as_str(), "str" | "int" | "bool")
                || !matches!(need(&data.provider_modules,symbol.module)?,ProviderModule::Bundled{provider,bundle:ModuleBundle::Typeshed,name}if *provider==symbol.provider&&name=="builtins")
        } else {
            !matches!(
                need(&data.provider_modules, symbol.module)?,
                ProviderModule::Acquired { .. }
                    | ProviderModule::Bundled {
                        bundle: ModuleBundle::Typeshed,
                        ..
                    }
            )
        }
    {
        return Err(TheoryReason::UnsupportedBuiltin.into());
    }
    let cq = need(&data.entry.qualifications, observation.qualification)?;
    if cq.condition != conditions::Diagram::always().id() {
        return Err(TheoryReason::UnsupportedBuiltin.into());
    }
    let mut supports = data
        .type_supports
        .iter()
        .filter(|s| s.assertion == observation.id());
    let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
    if supports.next().is_some()
        || native.origin != Origin::AnalyzerAssertion
        || native.mode != ExtractionMode::NativeTraversal
    {
        return Err(TheoryReason::MissingEvidence.into());
    }
    support(data, native, cq, invocation, FactFamily::Types)?;
    if need(&data.entry.runs, native.run)?.provider != symbol.provider {
        return Err(TheoryReason::IncompatibleFrame.into());
    }
    let (class_resolution, class_resolution_support) = if matches!(need(&data.provider_modules,symbol.module)?,ProviderModule::Bundled{bundle:ModuleBundle::Typeshed,name,..}if name=="builtins")
    {
        builtin_resolution(data, invocation, class_occurrence, &symbol.name)?
    } else {
        let mut resolutions = data.resolutions.iter().filter(|r| {
            same_node(data, r.read, class_occurrence)
                && data
                    .entry
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        });
        let row = resolutions.next().ok_or(TheoryReason::UnsupportedBuiltin)?;
        if resolutions.next().is_some()
            || !matches!(
                need(&data.lexical_targets, row.target)?,
                lexical::LexicalTarget::Binding { .. }
            )
        {
            return Err(TheoryReason::UnsupportedBuiltin.into());
        }
        let q = need(&data.entry.qualifications, row.qualification)?;
        if q.condition != conditions::Diagram::always().id() {
            return Err(TheoryReason::UnsupportedBuiltin.into());
        }
        let mut supports = data
            .resolution_supports
            .iter()
            .filter(|s| s.assertion == row.id());
        let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
        if supports.next().is_some()
            || native.origin != Origin::DerivedAnalysis
            || native.mode != ExtractionMode::Recognizer
            || native.fidelity != Fidelity::NormalizedStructural
        {
            return Err(TheoryReason::MissingEvidence.into());
        }
        support(data, native, q, invocation, FactFamily::Lexical)?;
        (row, native)
    };
    Ok(BuiltinOperandWitness {
        invocation: invocation.id(),
        leaf: leaf.id(),
        call: call.id(),
        syntax_support: native_syntax.id(),
        callee_resolution: callee.id(),
        callee_support: callee_support.id(),
        class_observation: observation.id(),
        class_support: native.id(),
        class_resolution: class_resolution.id(),
        class_resolution_support: class_resolution_support.id(),
        call_placement,
        call_placement_support,
        placement,
        placement_support,
        class: *class,
    })
}
impl analysis::support::sealed::DerivedEvidence for TheoryWitness {}
impl analysis::support::DerivedEvidence for TheoryWitness {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, TheoryReason> {
    rows.required(id, || TheoryReason::MissingEvidence)
}
fn frame(
    data: &TheoryData,
    q: &AssertionQualification,
    invocation: &publication::AnalysisInvocation,
) -> Result<(), TheoryReason> {
    if q.context != invocation.context
        || q.modality != Modality::Definite
        || q.approximation != Approximation::Exact
    {
        return Err(TheoryReason::IncompatibleFrame);
    }
    let input = crate::domain::normalized::signature_applicability::ScopeCatalog {
        scopes: &data.entry.scopes,
        artifacts: &data.entry.artifacts,
        modules: &data.entry.modules,
    }
    .input(q.scope);
    if input != Some(invocation.input) {
        return Err(TheoryReason::IncompatibleFrame);
    }
    Ok(())
}
pub(crate) fn support<S: Support>(
    data: &TheoryData,
    s: &S,
    q: &AssertionQualification,
    invocation: &publication::AnalysisInvocation,
    family: FactFamily,
) -> Result<Id<ProviderRun>, TheoryReason> {
    frame(data, q, invocation)?;
    let a = s.attribution().ok_or(TheoryReason::MissingEvidence)?;
    let run = need(&data.entry.runs, a.run)?;
    let surface = need(&data.entry.surfaces, a.surface)?;
    if (run.input, run.context) != (invocation.input, invocation.context)
        || surface.provider != run.provider
        || surface.family != family
    {
        return Err(TheoryReason::IncompatibleFrame);
    }
    if a.fidelity == Fidelity::DisplayOnly {
        return Err(TheoryReason::DisplayOnly);
    }
    if a.fidelity != Fidelity::NativeStructural
        && !(family == FactFamily::Signatures && a.fidelity == Fidelity::ReportProjection)
        && !(family == FactFamily::Lexical && a.fidelity == Fidelity::NormalizedStructural)
    {
        return Err(TheoryReason::MissingEvidence);
    }
    match need(&data.entry.evidence, a.evidence)? {
        Evidence::Invocation { run } if *run == a.run => {}
        Evidence::Occurrence { occurrence }
            if data
                .entry
                .occurrences
                .get(*occurrence)
                .and_then(|o| data.entry.artifacts.get(o.source))
                .is_some_and(|source| source.input == invocation.input) => {}
        Evidence::SourceSpan { source, .. }
            if data
                .entry
                .artifacts
                .get(*source)
                .is_some_and(|source| source.input == invocation.input) => {}
        _ => return Err(TheoryReason::MissingEvidence),
    }
    let mut covered = false;
    for coverage in data.entry.coverage.iter().filter(|c| {
        c.scope == q.scope
            && c.family == family
            && c.context == q.context
            && c.run == Some(a.run)
            && c.provider == Some(run.provider)
    }) {
        if coverage.status != CoverageStatus::CompleteUnderStatedModel {
            // Partial inventories cannot prove absence. A completed, explicitly located native
            // type query can still interpret its own term when an unrelated query is missing.
            // Parse/source/resource losses and partial queries retain the old refusal.
            let wanted = crate::domain::derivation::RowRef::of(s.assertion());
            let mut queries = data.type_queries.iter().filter(|query| {
                query.qualification == q.id()
                    && query.status == types::TypeQueryStatus::Available
                    && query
                        .observation
                        .is_some_and(|id| crate::domain::derivation::RowRef::of(id) == wanted)
            });
            let selected = queries.next();
            let located = data.type_observations.iter().find(|observation| {
                crate::domain::derivation::RowRef::of(observation.id()) == wanted
            });
            let local=family==FactFamily::Types && coverage.status==CoverageStatus::Partial && coverage.reason==Some(obligation::ObligationKind::MissingEvidence) && queries.next().is_none() && selected.is_some_and(|query|located.is_some_and(|observation|(query.subject,query.role,query.declared)==(observation.subject,observation.role,observation.declared)) && data.type_query_supports.iter().any(|support|support.assertion==query.id() && support.run==a.run && support.surface==a.surface && support.origin==Origin::AnalyzerAssertion && support.mode==ExtractionMode::NativeTraversal && support.fidelity==Fidelity::NativeStructural && matches!(data.entry.evidence.get(support.evidence),Some(Evidence::Occurrence {occurrence}) if *occurrence==query.subject)));
            if !local {
                return Err(TheoryReason::IncompleteCoverage);
            }
        }
        covered = true;
    }
    if !covered {
        return Err(TheoryReason::IncompleteCoverage);
    }
    Ok(a.run)
}
/// Opaque bounded result retains the exact domain and complete native ancestry premises.
pub struct DerivedDomain {
    domain: TypeDomain,
    values: Rows<DomainValue>,
    members: Rows<TypeDomainMember>,
    ancestry: Rows<ClassDomainAncestry>,
    classes: Rows<ClassDomainMember>,
    status: EvidenceStatus,
}
impl DerivedDomain {
    pub fn domain(&self) -> &TypeDomain {
        &self.domain
    }
    pub fn values(&self) -> impl Iterator<Item = &DomainValue> {
        self.values.iter()
    }
    pub fn ancestry(&self) -> impl Iterator<Item = &ClassDomainAncestry> {
        self.ancestry.iter()
    }
    pub fn classes(&self) -> impl Iterator<Item = &ClassDomainMember> {
        self.classes.iter()
    }
}
enum Refusal {
    Boundary(TheoryReason),
    Resource(ModelError),
}
impl From<TheoryReason> for Refusal {
    fn from(reason: TheoryReason) -> Self {
        Self::Boundary(reason)
    }
}
impl From<ModelError> for Refusal {
    fn from(error: ModelError) -> Self {
        Self::Resource(error)
    }
}
impl TypeDomain {
    pub fn derive(
        data: &TheoryData,
        invocation: &publication::AnalysisInvocation,
        observation: Id<TypeObservation>,
        native_support: Id<TypeSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<DerivedDomain, TheoryReason>, ModelError> {
        match derive_domain(data, invocation, observation, native_support, budget) {
            Ok(value) => Ok(Ok(value)),
            Err(Refusal::Boundary(reason)) => Ok(Err(reason)),
            Err(Refusal::Resource(error)) => Err(error),
        }
    }
}
fn derive_domain(
    data: &TheoryData,
    invocation: &publication::AnalysisInvocation,
    observation: Id<TypeObservation>,
    native_support: Id<TypeSupport>,
    budget: &resources::ResourceBudget,
) -> Result<DerivedDomain, Refusal> {
    if invocation.definition != crate::domain::local_semantics::definition().1.id() {
        return Err(invalid("Local theory invocation differs from supported definition").into());
    }
    let observation = need(&data.type_observations, observation)?;
    let native = need(&data.type_supports, native_support)?;
    if native.assertion != observation.id() {
        return Err(TheoryReason::MissingEvidence.into());
    }
    if native.origin != Origin::AnalyzerAssertion || native.mode != ExtractionMode::NativeTraversal
    {
        return Err(TheoryReason::MissingEvidence.into());
    }
    let q = need(&data.entry.qualifications, observation.qualification)?;
    let run = support(data, native, q, invocation, FactFamily::Types)?;
    let provider = need(&data.entry.runs, run)?.provider;
    let mut charge = charged::StateCharge::new(budget, "local_type_domain_work");
    let mut stack = charged::ChargedVec::default();
    let mut state = charged::ChargedMap::<Id<TypeTerm>, bool>::default();
    let mut values = Rows::new(budget);
    let mut pending_classes = charged::ChargedSet::default();
    stack.push(&mut charge, (observation.term, true, 0usize))?;
    let mut work = 0;
    let buffer = data
        .sequence_members
        .len()
        .checked_mul(size_of::<TypeSequenceMember>() * 4)
        .and_then(|n| {
            n.checked_add(
                data.symbol_members
                    .len()
                    .saturating_mul(size_of::<SymbolSequenceMember>() * 4),
            )
        })
        .ok_or_else(|| invalid("Local theory sequence allocation overflow"))?;
    let _buffer = budget.reserve("local_type_membership_buffer", buffer)?;
    while let Some((id, enter, depth)) = stack.take_last(&mut charge) {
        work += 1;
        if depth > 256 {
            return Err(TheoryReason::WorkLimit.into());
        }
        if work > 32_768 {
            return Err(TheoryReason::WorkLimit.into());
        }
        if !enter {
            state.insert(&mut charge, id, false)?;
            continue;
        }
        if let Some(active) = state.get(&id) {
            if *active {
                return Err(TheoryReason::RecursiveType.into());
            }
            continue;
        }
        state.insert(&mut charge, id, true)?;
        stack.push(&mut charge, (id, false, depth))?;
        match need(&data.terms, id)? {
            TypeTerm::Literal { value } => {
                match need(&data.literals, *value)? {
                    Literal::None
                    | Literal::Bool { .. }
                    | Literal::Integer { .. }
                    | Literal::String { .. } => {}
                    _ => return Err(TheoryReason::UnsupportedScalar.into()),
                }
                values.insert(DomainValue::Literal { literal: *value })?;
            }
            TypeTerm::None => {
                values.insert(DomainValue::None)?;
            }
            TypeTerm::Never { .. } => {}
            TypeTerm::Union { members } => {
                let sequence = need(&data.sequences, *members)?;
                let mut rows = data
                    .sequence_members
                    .iter()
                    .filter(|row| row.sequence == *members)
                    .collect::<Vec<_>>();
                if rows.len() > 4096 {
                    return Err(TheoryReason::WorkLimit.into());
                }
                rows.sort_by_key(|row| row.ordinal);
                let mut items = Vec::with_capacity(rows.len());
                for (ordinal, row) in rows.iter().enumerate() {
                    if row.ordinal != ordinal as i64 || row.role != TypeChildRole::Member {
                        return Err(TheoryReason::MissingEvidence.into());
                    }
                    items.push((row.role, row.child));
                }
                if TypeSequence::new(&items)?.0 != *sequence {
                    return Err(TheoryReason::MissingEvidence.into());
                }
                for (_, child) in items {
                    stack.push(&mut charge, (child, true, depth + 1))?;
                }
            }
            TypeTerm::Annotated { target }
            | TypeTerm::TypeAlias {
                target,
                untyped: false,
                ..
            } => {
                stack.push(&mut charge, (*target, true, depth + 1))?;
            }
            TypeTerm::ClassInstance { class, .. } | TypeTerm::SelfType { class, .. } => {
                let symbol = need(&data.entry.symbols, *class)?;
                if symbol.provider != provider
                    || symbol.context != invocation.context
                    || symbol.kind != SymbolKind::Class
                {
                    return Err(TheoryReason::IncompatibleFrame.into());
                }
                values.insert(DomainValue::Class { class: *class })?;
                pending_classes.insert(&mut charge, *class)?;
            }
            TypeTerm::ClassObject { class } => {
                let symbol = need(&data.entry.symbols, *class)?;
                if symbol.provider != provider
                    || symbol.context != invocation.context
                    || symbol.kind != SymbolKind::Class
                {
                    return Err(TheoryReason::IncompatibleFrame.into());
                }
                values.insert(DomainValue::ClassObject { class: *class })?;
                pending_classes.insert(&mut charge, *class)?;
            }
            TypeTerm::Other { .. } | TypeTerm::Any { .. } => {
                return Err(TheoryReason::OpaqueType.into());
            }
            TypeTerm::Truncated { .. } => return Err(TheoryReason::TruncatedType.into()),
            _ => return Err(TheoryReason::UnsupportedType.into()),
        }
    }
    if values.len() > 4096 {
        return Err(TheoryReason::WorkLimit.into());
    }
    let mut sink = KeySink::new("Local structural type domain");
    for row in values.iter() {
        row.id().encode(&mut sink);
    }
    let domain = TypeDomain {
        invocation: invocation.id(),
        observation: observation.id(),
        support: native.id(),
        qualification: q.id(),
        completeness: if pending_classes.is_empty() {
            DomainCompleteness::FiniteUnderTypingModel
        } else {
            DomainCompleteness::OpenClasses
        },
        members: sink.finish(),
    };
    let mut members = Rows::new(budget);
    for row in values.iter() {
        members.insert(TypeDomainMember {
            domain: domain.id(),
            value: row.id(),
        })?;
    }
    let mut ancestry = Rows::new(budget);
    let mut classes = Rows::new(budget);
    for class in pending_classes.iter() {
        let mut observations = data.ancestry.iter().filter(|row| {
            row.class == *class
                && row.relation == AncestryRelation::Mro
                && data
                    .entry
                    .qualifications
                    .get(row.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        });
        let row = observations.next().ok_or(TheoryReason::IncompleteMro)?;
        if observations.next().is_some() || row.linearization != Some(Linearization::Complete) {
            return Err(TheoryReason::IncompleteMro.into());
        }
        let mq = need(&data.entry.qualifications, row.qualification)?;
        frame(data, mq, invocation)?;
        if mq.condition != conditions::Diagram::always().id() && mq.condition != q.condition {
            return Err(TheoryReason::IncompatibleFrame.into());
        }
        let mut supports = data
            .ancestry_supports
            .iter()
            .filter(|s| s.assertion == row.id());
        let native = supports.next().ok_or(TheoryReason::MissingEvidence)?;
        if native.origin != Origin::AnalyzerAssertion
            || native.mode != ExtractionMode::NativeTraversal
            || supports.next().is_some()
            || support(data, native, mq, invocation, FactFamily::Signatures)? != run
        {
            return Err(TheoryReason::IncompatibleFrame.into());
        }
        let mut sequence = data
            .symbol_members
            .iter()
            .filter(|m| m.sequence == row.ancestors)
            .collect::<Vec<_>>();
        if sequence.len() > 4096 {
            return Err(TheoryReason::WorkLimit.into());
        }
        sequence.sort_by_key(|m| m.ordinal);
        let mut ids = Vec::with_capacity(sequence.len());
        for (ordinal, member) in sequence.iter().enumerate() {
            work += 1;
            if work > 32_768 {
                return Err(TheoryReason::WorkLimit.into());
            }
            let symbol = need(&data.entry.symbols, member.symbol)?;
            if member.ordinal != ordinal as i64
                || member.symbol == *class
                || ids.contains(&member.symbol)
                || symbol.provider != provider
                || symbol.context != invocation.context
                || symbol.kind != SymbolKind::Class
            {
                return Err(TheoryReason::IncompleteMro.into());
            }
            ids.push(member.symbol);
        }
        if SymbolSequence::new(&ids)?.0 != *need(&data.symbol_sequences, row.ancestors)? {
            return Err(TheoryReason::IncompleteMro.into());
        }
        let proof = ClassDomainAncestry {
            domain: domain.id(),
            class: *class,
            observation: row.id(),
            support: native.id(),
        };
        classes.insert(ClassDomainMember {
            ancestry: proof.id(),
            ancestor: *class,
            native_member: None,
        })?;
        for native_member in sequence {
            classes.insert(ClassDomainMember {
                ancestry: proof.id(),
                ancestor: native_member.symbol,
                native_member: Some(native_member.id()),
            })?;
        }
        ancestry.insert(proof)?;
    }
    Ok(DerivedDomain {
        domain,
        values,
        members,
        ancestry,
        classes,
        status: policy::native_status(FactFamily::Types, native.fidelity),
    })
}
fn scalar_value<'a>(
    data: &'a TheoryData,
    value: &DomainValue,
) -> Result<&'a Literal, TheoryReason> {
    match value {
        DomainValue::Literal { literal } => need(&data.literals, *literal),
        DomainValue::None => Ok(&Literal::None),
        DomainValue::Class { .. } | DomainValue::ClassObject { .. } => {
            Err(TheoryReason::OpenClassUniverse)
        }
    }
}
/// The retained primitive envelope: no numeric coercion, user equality or mutable truthiness.
fn evaluate_scalar(
    data: &TheoryData,
    predicate: &Predicate,
    value: &Literal,
) -> Result<bool, TheoryReason> {
    match predicate {
        Predicate::IsNone => Ok(matches!(value, Literal::None)),
        Predicate::IsValue { value: expected } => {
            let expected = need(&data.literals, *expected)?;
            if !matches!(expected, Literal::None | Literal::Bool { .. }) {
                return Err(TheoryReason::UnsupportedPredicate);
            }
            Ok(expected == value)
        }
        Predicate::Equals { value: expected } => match (need(&data.literals, *expected)?, value) {
            (Literal::String { value: a }, Literal::String { value: b }) => Ok(a == b),
            (Literal::Integer { decimal: a }, Literal::Integer { decimal: b }) => Ok(a == b),
            _ => Err(TheoryReason::UnsupportedScalar),
        },
        Predicate::MemberOf { values } => {
            let set = need(&data.sets, *values)?;
            let members = data
                .set_members
                .iter()
                .filter(|row| row.set == *values)
                .collect::<Vec<_>>();
            let mut literals = Vec::with_capacity(members.len());
            for member in members {
                let literal = need(&data.literals, member.value)?;
                if !matches!(literal, Literal::String { .. }) {
                    return Err(TheoryReason::UnsupportedScalar);
                }
                literals.push(literal.clone());
            }
            if LiteralSet::of(literals.iter().map(Record::id)).0 != *set {
                return Err(TheoryReason::MissingEvidence);
            }
            if !matches!(value, Literal::String { .. }) {
                return Err(TheoryReason::UnsupportedScalar);
            }
            Ok(literals.iter().any(|candidate| candidate == value))
        }
        Predicate::Truthy => match value {
            Literal::None => Ok(false),
            Literal::Bool { value } => Ok(*value),
            Literal::Integer { decimal } => Ok(decimal != "0"),
            Literal::String { value } => Ok(!value.is_empty()),
            _ => Err(TheoryReason::UnsupportedScalar),
        },
        Predicate::TypeIs { .. } | Predicate::IsInstance { .. } => {
            Err(TheoryReason::UnsupportedBuiltin)
        }
        _ => Err(TheoryReason::UnsupportedPredicate),
    }
}
/// An assessment is admitted only against the independently validated N2 operand bridge.
/// It remains a statement under the provider typing model, never a concrete execution witness.
pub struct DerivedPredicate {
    witness: TheoryWitness,
    domain: DerivedDomain,
    builtin: Option<BuiltinOperandWitness>,
}
impl DerivedPredicate {
    pub fn witness(&self) -> &TheoryWitness {
        &self.witness
    }
    pub fn domain(&self) -> &DerivedDomain {
        &self.domain
    }
}
impl TheoryWitness {
    pub fn derive(
        data: &TheoryData,
        invocation: &publication::AnalysisInvocation,
        leaf: Id<FlowTestLeafObservation>,
        native_support: Id<FlowTestLeafSupport>,
        budget: &resources::ResourceBudget,
    ) -> Result<Result<DerivedPredicate, TheoryReason>, ModelError> {
        match derive_predicate(data, invocation, leaf, native_support, budget) {
            Ok(value) => Ok(Ok(value)),
            Err(Refusal::Boundary(reason)) => Ok(Err(reason)),
            Err(Refusal::Resource(error)) => Err(error),
        }
    }
}
fn derive_predicate(
    data: &TheoryData,
    invocation: &publication::AnalysisInvocation,
    leaf: Id<FlowTestLeafObservation>,
    native_support: Id<FlowTestLeafSupport>,
    budget: &resources::ResourceBudget,
) -> Result<DerivedPredicate, Refusal> {
    let leaf = need(&data.entry.leaves, leaf)?;
    let native = need(&data.entry.leaf_supports, native_support)?;
    if native.assertion != leaf.id()
        || native.origin != Origin::AnalyzerAssertion
        || native.mode != ExtractionMode::NativeTraversal
    {
        return Err(TheoryReason::MissingEvidence.into());
    }
    let q = need(&data.entry.qualifications, leaf.qualification)?;
    support(data, native, q, invocation, FactFamily::Flow)?;
    let mut assessments = data
        .operand_assessments
        .iter()
        .filter(|row| row.leaf == leaf.id());
    let assessment = assessments.next().ok_or(TheoryReason::MissingEvidence)?;
    if assessments.next().is_some() || assessment.status != ResolutionStatus::Resolved {
        return Err(TheoryReason::AmbiguousOperand.into());
    }
    let mut links = data
        .operand_links
        .iter()
        .filter(|row| row.assessment == assessment.id());
    let link = links.next().ok_or(TheoryReason::MissingEvidence)?;
    if links.next().is_some() {
        return Err(TheoryReason::AmbiguousOperand.into());
    }
    let observation = need(&data.type_observations, link.observation)?;
    if observation.role != TypeRole::TestOperand || Some(observation.subject) != leaf.operand {
        return Err(TheoryReason::IncompatibleFrame.into());
    }
    let oq = need(&data.entry.qualifications, observation.qualification)?;
    frame(data, oq, invocation)?;
    let mut supports = data
        .type_supports
        .iter()
        .filter(|s| s.assertion == observation.id());
    let native_type = supports.next().ok_or(TheoryReason::MissingEvidence)?;
    if supports.next().is_some() {
        return Err(TheoryReason::AmbiguousOperand.into());
    }
    let operand = need(&data.entry.occurrences, observation.subject)?;
    if !data
        .entry
        .artifacts
        .get(operand.source)
        .is_some_and(|source| source.input == invocation.input)
    {
        return Err(TheoryReason::IncompatibleFrame.into());
    }
    // A narrower conditional type observation cannot prove a predicate outside its domain.
    if oq.condition != q.condition {
        let allowance = data
            .entry
            .condition_nodes
            .len()
            .checked_mul(2048)
            .ok_or_else(|| invalid("Local theory condition allocation overflow"))?;
        let _decode = budget.reserve("local_theory_condition_decode", allowance)?;
        let nodes = data
            .entry
            .condition_nodes
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let leaf_condition =
            conditions::Diagram::from_records(need(&data.entry.conditions, q.condition)?, &nodes)?;
        let type_condition =
            conditions::Diagram::from_records(need(&data.entry.conditions, oq.condition)?, &nodes)?;
        let covered = leaf_condition
            .admitted_binary(
                &type_condition,
                conditions::BooleanOperation::Conjunction,
                budget,
            )
            .map_err(|e| match e {
                conditions::DiagramAdmissionError::Resource(e) => Refusal::Resource(e),
                conditions::DiagramAdmissionError::Boundary(_) => {
                    Refusal::Boundary(TheoryReason::WorkLimit)
                }
            })?;
        if covered.into_parts().0.id() != leaf_condition.id() {
            return Err(TheoryReason::IncompatibleFrame.into());
        }
    }
    let atom = need(&data.entry.atoms, leaf.atom)?;
    if atom.context != invocation.context {
        return Err(TheoryReason::IncompatibleFrame.into());
    }
    let predicate = need(&data.entry.predicates, atom.predicate)?;
    let domain = derive_domain(data, invocation, observation.id(), native_type.id(), budget)?;
    if domain.domain.completeness != DomainCompleteness::FiniteUnderTypingModel
        && !matches!(predicate, Predicate::IsInstance { .. })
    {
        return Err(TheoryReason::OpenClassUniverse.into());
    }
    // Literal membership cloning is bounded and reserved before predicate evaluation.
    let allowance = data
        .literals
        .iter()
        .try_fold(0usize, |sum, row| {
            sum.checked_add(row.heap_bytes())
                .and_then(|n| n.checked_add(size_of::<Literal>()))
        })
        .ok_or_else(|| invalid("Local scalar literal allocation overflow"))?
        .checked_mul(3)
        .and_then(|n| {
            n.checked_add(
                data.set_members
                    .len()
                    .saturating_mul(size_of::<LiteralSetMember>() * 4),
            )
        })
        .ok_or_else(|| invalid("Local scalar membership allocation overflow"))?;
    let _buffer = budget.reserve("local_scalar_membership", allowance)?;
    let builtin = if matches!(
        predicate,
        Predicate::TypeIs { .. } | Predicate::IsInstance { .. }
    ) {
        Some(builtin_operand(data, invocation, leaf, predicate, budget)?)
    } else {
        None
    };
    let mut true_ = false;
    let mut false_ = false;
    let mut work = 0;
    for value in domain.values.iter() {
        work += 1;
        if work > 4096 {
            return Err(TheoryReason::WorkLimit.into());
        }
        let truth =
            if domain.domain.completeness == DomainCompleteness::OpenClasses {
                let Some(builtin) = &builtin else {
                    return Err(TheoryReason::OpenClassUniverse.into());
                };
                let DomainValue::Class { class } = value else {
                    return Err(TheoryReason::OpenClassUniverse.into());
                };
                let ancestry = domain
                    .ancestry
                    .iter()
                    .find(|a| a.class == *class)
                    .ok_or(TheoryReason::IncompleteMro)?;
                if !domain.classes.iter().any(|member| {
                    member.ancestry == ancestry.id() && member.ancestor == builtin.class
                }) {
                    return Err(TheoryReason::OpenClassUniverse.into());
                }
                true
            } else {
                let literal = scalar_value(data, value)?;
                if let Some(builtin) = &builtin {
                    let class = need(&data.entry.symbols, builtin.class)?;
                    match class.name.as_str() {
                        "str" => matches!(literal, Literal::String { .. }),
                        "bool" => matches!(literal, Literal::Bool { .. }),
                        "int" => {
                            matches!(literal, Literal::Integer { .. })
                                || matches!(predicate, Predicate::IsInstance { .. })
                                    && matches!(literal, Literal::Bool { .. })
                        }
                        _ => return Err(TheoryReason::UnsupportedBuiltin.into()),
                    }
                } else {
                    evaluate_scalar(data, predicate, literal)?
                }
            };
        if truth {
            true_ = true;
        } else {
            false_ = true;
        }
    }
    let result = match (true_, false_) {
        (true, false) => PredicateResult::AlwaysTrueUnderTypingModel,
        (false, true) => PredicateResult::AlwaysFalseUnderTypingModel,
        (false, false) => PredicateResult::Uninhabited,
        (true, true) => PredicateResult::Mixed,
    };
    let status = policy::derive_status(&[
        (SupportRole::Support, domain.status),
        (
            SupportRole::Support,
            policy::native_status(FactFamily::Flow, native.fidelity),
        ),
    ]);
    Ok(DerivedPredicate {
        witness: TheoryWitness {
            invocation: invocation.id(),
            leaf: leaf.id(),
            support: native.id(),
            operand_link: link.id(),
            domain: domain.domain.id(),
            qualification: q.id(),
            builtin: builtin.as_ref().map(Record::id),
            result,
            status,
        },
        domain,
        builtin,
    })
}
#[macro_export]
macro_rules! local_theory_outputs {
    ($apply:ident) => {
        $apply! {
         type_assessments:$crate::domain::local_theory::TypeDomainAssessment,
         domains:$crate::domain::local_theory::TypeDomain,
         values:$crate::domain::local_theory::DomainValue,
         members:$crate::domain::local_theory::TypeDomainMember,
         ancestry:$crate::domain::local_theory::ClassDomainAncestry,
         classes:$crate::domain::local_theory::ClassDomainMember,
         scalar_assessments:$crate::domain::local_theory::ScalarAssessment,
         builtin_operands:$crate::domain::local_theory::BuiltinOperandWitness,
         witnesses:$crate::domain::local_theory::TheoryWitness,
        }
    };
}
macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{
 pub struct TheoryRecords{$(pub $field:Rows<$ty>,)*}
 impl TheoryRecords{pub fn new(budget:&resources::ResourceBudget)->Self{Self{$($field:Rows::new(budget),)*}}
 pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
 fn insert_domain(&mut self,domain:DerivedDomain)->Result<(),ModelError>{self.domains.insert(domain.domain)?;for row in domain.values.iter(){self.values.insert(row.clone())?;}for row in domain.members.iter(){self.members.insert(row.clone())?;}for row in domain.ancestry.iter(){self.ancestry.insert(row.clone())?;}for row in domain.classes.iter(){self.classes.insert(row.clone())?;}Ok(())}
 }
};}
crate::local_theory_outputs!(outputs);
pub fn relations() -> Vec<Relation> {
    let mut relations = Vec::new();
    macro_rules! relations{($($field:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};}
    crate::local_theory_outputs!(relations);
    relations
}
pub fn produce(
    data: &TheoryData,
    invocation: &publication::AnalysisInvocation,
    budget: &resources::ResourceBudget,
) -> Result<TheoryRecords, ModelError> {
    let mut records = TheoryRecords::new(budget);
    for support in data.type_supports.iter() {
        let Some(run) = data.entry.runs.get(support.run) else {
            return Err(invalid("type support run absent"));
        };
        if (run.input, run.context) != (invocation.input, invocation.context) {
            continue;
        }
        let assessment =
            match TypeDomain::derive(data, invocation, support.assertion, support.id(), budget)? {
                Ok(domain) => {
                    let id = domain.domain.id();
                    records.insert_domain(domain)?;
                    TypeDomainAssessment {
                        invocation: invocation.id(),
                        observation: support.assertion,
                        support: support.id(),
                        domain: Some(id),
                        reason: None,
                    }
                }
                Err(reason) => TypeDomainAssessment {
                    invocation: invocation.id(),
                    observation: support.assertion,
                    support: support.id(),
                    domain: None,
                    reason: Some(reason),
                },
            };
        records.type_assessments.insert(assessment)?;
    }
    for support in data.entry.leaf_supports.iter() {
        let Some(run) = data.entry.runs.get(support.run) else {
            return Err(invalid("scalar support run absent"));
        };
        if (run.input, run.context) != (invocation.input, invocation.context) {
            continue;
        }
        let assessment =
            match TheoryWitness::derive(data, invocation, support.assertion, support.id(), budget)?
            {
                Ok(proof) => {
                    let witness = proof.witness.id();
                    records.witnesses.insert(proof.witness)?;
                    if let Some(builtin) = proof.builtin {
                        records.builtin_operands.insert(builtin)?;
                    }
                    records.insert_domain(proof.domain)?;
                    ScalarAssessment {
                        invocation: invocation.id(),
                        leaf: support.assertion,
                        support: support.id(),
                        witness: Some(witness),
                        reason: None,
                    }
                }
                Err(reason) => ScalarAssessment {
                    invocation: invocation.id(),
                    leaf: support.assertion,
                    support: support.id(),
                    witness: None,
                    reason: Some(reason),
                },
            };
        records.scalar_assessments.insert(assessment)?;
    }
    Ok(records)
}
pub(crate) fn theory_invariants() -> Vec<Invariant> {
    let mut inputs = TheoryData::validation_inputs();
    inputs.push(ValidationInput::of::<publication::AnalysisInvocation>(&[
        "id",
    ]));
    macro_rules! output_inputs{($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    crate::local_theory_outputs!(output_inputs);
    vec![Invariant {
        revision: 1,
        name: "local_structural_theory_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(TheoryCheck {
                entry: EntryData::new(budget),
                inventory: TheoryInventory::new(budget),
                records: TheoryRecords::new(budget),
                invocations: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct TheoryCheck {
    entry: EntryData,
    inventory: TheoryInventory,
    records: TheoryRecords,
    invocations: Rows<publication::AnalysisInvocation>,
    budget: resources::ResourceBudget,
}
impl InvariantCheck for TheoryCheck {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if stages::is_vocabulary(input.name())
            && input.prefix() != Some(stages::PublicationBoundary::Facts)
        {
            return Err(invalid("Local theory requires Facts vocabulary"));
        }
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.entry.visit(name, batch)?
            || self.inventory.visit(name, batch)?
            || self.records.visit(name, batch)?
        {
            return Ok(());
        }
        if name == publication::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
            return Ok(());
        }
        Err(invalid("undeclared Local theory replay input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let data = TheoryData {
            entry: &self.entry,
            inventory: &self.inventory,
        };
        let mut expected = TheoryRecords::new(&self.budget);
        for assessment in self.records.type_assessments.iter() {
            let invocation = need(&self.invocations, assessment.invocation)
                .map_err(|_| invalid("type assessment invocation absent"))?;
            match TypeDomain::derive(
                &data,
                invocation,
                assessment.observation,
                assessment.support,
                &self.budget,
            )? {
                Ok(domain) => {
                    if assessment.domain != Some(domain.domain.id()) || assessment.reason.is_some()
                    {
                        return Err(invalid("stored type assessment differs from replay"));
                    }
                    expected.insert_domain(domain)?;
                }
                Err(reason) => {
                    if assessment.domain.is_some() || assessment.reason != Some(reason) {
                        return Err(invalid("stored type boundary differs from replay"));
                    }
                }
            }
        }
        for assessment in self.records.scalar_assessments.iter() {
            let invocation = need(&self.invocations, assessment.invocation)
                .map_err(|_| invalid("scalar assessment invocation absent"))?;
            match TheoryWitness::derive(
                &data,
                invocation,
                assessment.leaf,
                assessment.support,
                &self.budget,
            )? {
                Ok(proof) => {
                    if assessment.witness != Some(proof.witness.id()) || assessment.reason.is_some()
                    {
                        return Err(invalid("stored scalar assessment differs from replay"));
                    }
                    expected.witnesses.insert(proof.witness)?;
                    if let Some(builtin) = proof.builtin {
                        expected.builtin_operands.insert(builtin)?;
                    }
                    expected.insert_domain(proof.domain)?;
                }
                Err(reason) => {
                    if assessment.witness.is_some() || assessment.reason != Some(reason) {
                        return Err(invalid("stored scalar boundary differs from replay"));
                    }
                }
            }
        }
        macro_rules! compare{($($field:ident),*)=>{$(if self.records.$field.len()!=expected.$field.len()||self.records.$field.iter().any(|row|expected.$field.get(row.id())!=Some(row)){return Err(invalid(concat!("stored Local theory ",stringify!($field)," differs from replay")));})*};}
        compare!(
            domains,
            values,
            members,
            ancestry,
            classes,
            builtin_operands,
            witnesses
        );
        Ok(())
    }
}

pub(crate) fn theory_invariants_refs() -> Vec<&'static str> { vec!["local_structural_theory_replay"] }
