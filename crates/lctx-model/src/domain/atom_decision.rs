//! Located predicate decisions and premise-preserving condition restriction. Native types describe
//! a typing world; these refinements never remove the conservative runtime alternative.
use super::{
    analysis::{
        self, local as owner,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::*,
    assumptions::*,
    conditions::*,
    flow::*,
    local_semantics::{LocalData, LocalRecords},
    local_theory::{PredicateResult, TheoryReason, TheoryWitness},
    normalized::Rows,
    resources::ResourceBudget,
    transfer::{local::*, *},
    *,
};
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AtomOutcome {
    True = 0,
    False = 1,
    Mixed = 2,
    Uninhabited = 3,
    Refused = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="local_atom_decisions", rule="located_typing_atom_decision", invariant_refs=invariants_refs)]
pub struct AtomDecision {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key, premise)]
    pub support: Id<FlowTestLeafSupport>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(premise)]
    pub theory: Option<Id<TheoryWitness>>,
    pub outcome: AtomOutcome,
    pub reason: Option<TheoryReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "local_atom_restrictions",
    rule = "conditional_atom_transfer_restriction"
)]
pub struct AtomRestriction {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)]
    pub decision: Id<AtomDecision>,
    #[model(key, premise)]
    pub original: Id<TransferAlternative>,
    #[model(key, premise)]
    pub original_support: Id<TransferSupport>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    pub status: analysis::policy::EvidenceStatus,
}
/// Select the original runtime alternative and its exact, separately supported typing-world
/// restrictions. Consumers still replay each selected alternative's evidence and qualification.
/// This scan retains no collection or proof state outside its caller's charged inventory.
pub(crate) fn selected_local_alternatives<'a>(
    original: &TransferAlternative,
    alternatives: &'a Rows<TransferAlternative>,
    restrictions: &'a Rows<AtomRestriction>,
) -> impl Iterator<Item = &'a TransferAlternative> + 'a {
    let transfer = original.transfer;
    let original = original.id();
    alternatives.iter().filter(move |alternative| {
        alternative.id() == original
            || restrictions.iter().any(|restriction| {
                restriction.original == original
                    && restriction.qualification == alternative.qualification
                    && alternative.transfer == transfer
            })
    })
}
impl analysis::support::sealed::DerivedEvidence for AtomRestriction {}
impl DerivedEvidence for AtomRestriction {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("atom decision evidence absent"))
}
fn outcome(result: PredicateResult) -> AtomOutcome {
    match result {
        PredicateResult::AlwaysTrueUnderTypingModel => AtomOutcome::True,
        PredicateResult::AlwaysFalseUnderTypingModel => AtomOutcome::False,
        PredicateResult::Mixed => AtomOutcome::Mixed,
        PredicateResult::Unknown => AtomOutcome::Refused,
        PredicateResult::Uninhabited => AtomOutcome::Uninhabited,
    }
}
fn condition(
    data: &LocalData,
    records: &LocalRecords,
    id: Id<Condition>,
    nodes: &[ConditionNode],
) -> Result<Diagram, ModelError> {
    Diagram::from_records(
        records
            .conditions
            .get(id)
            .or_else(|| data.entry.conditions.get(id))
            .ok_or_else(|| invalid("atom restriction condition absent"))?,
        nodes,
    )
}
/// The one question policy: a native finite-domain decision permits a separate answer under
/// its actual TypeConformance premise. Empty/unknown domains permit no branch evidence.
pub fn produce(
    data: &LocalData,
    invocation: &owner::AnalysisInvocation,
    definition: &analysis::AnalysisDefinition,
    rows: &mut LocalRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let allowance = data
        .entry
        .condition_nodes
        .len()
        .checked_add(rows.nodes.len())
        .and_then(|n| n.checked_mul(2048))
        .ok_or_else(|| invalid("atom restriction decode overflow"))?;
    let _decode = budget.reserve("atom_restriction_decode", allowance)?;
    let mut nodes = data
        .entry
        .condition_nodes
        .iter()
        .chain(rows.nodes.iter())
        .cloned()
        .collect::<Vec<_>>();
    nodes.sort_by_key(Record::id);
    nodes.dedup_by_key(|node| node.id());
    let _originals = budget.reserve(
        "atom_restriction_originals",
        rows.alternatives
            .len()
            .saturating_mul(size_of::<TransferAlternative>() + 64),
    )?;
    // Snapshot the conservative inventory. New conditional alternatives cannot recursively prune
    // one another, and no Cartesian product of independent premise sets is constructed here.
    let originals = rows.alternatives.iter().cloned().collect::<Vec<_>>();
    let _assessments = budget.reserve(
        "atom_decision_assessments",
        rows.theory
            .scalar_assessments
            .len()
            .saturating_mul(size_of::<super::local_theory::ScalarAssessment>() + 64),
    )?;
    let assessments = rows
        .theory
        .scalar_assessments
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut work = 0usize;
    for assessment in assessments {
        let leaf = need(&data.entry.leaves, assessment.leaf)?;
        let rawq = need(&data.entry.qualifications, leaf.qualification)?;
        let mut q = rawq.clone();
        let mut reason = assessment.reason;
        let mut result = AtomOutcome::Refused;
        if let Some(id) = assessment.witness {
            let witness = need(&rows.theory.witnesses, id)?;
            result = outcome(witness.result);
            if matches!(
                result,
                AtomOutcome::True | AtomOutcome::False | AtomOutcome::Mixed
            ) {
                let domain = need(&rows.theory.domains, witness.domain)?;
                let type_observation = need(&data.theory.type_observations, domain.observation)?;
                let type_q = need(&data.entry.qualifications, type_observation.qualification)?;
                // B0 premises are lower unconditional native facts. A conditional/narrower native
                // type cannot be laundered into a fresh globally applicable conformance premise.
                if type_q.assumptions != AssumptionSet::empty_id()
                    || type_q.condition != Diagram::always().id()
                    || rawq.assumptions != AssumptionSet::empty_id()
                {
                    result = AtomOutcome::Refused;
                    reason = Some(TheoryReason::IncompatibleFrame);
                } else {
                    let assumption = Assumption::TypeConformance {
                        observation: domain.observation,
                        support: domain.support,
                    };
                    let basis = AssumptionSet::new([assumption.id()])?;
                    // FlowTestLeaf qualification records the predicate formula, not an ambient
                    // reachability guard. The interpreted operand domain covers this evaluation;
                    // its always-qualified conformance premise applies to both polarities.
                    q.condition = type_q.condition;
                    q.assumptions = basis.set.id();
                    rows.assumptions.insert(assumption)?;
                    rows.assumption_sets.insert(basis.set)?;
                    for member in basis.members {
                        rows.assumption_members.insert(member)?;
                    }
                }
            }
        }
        rows.qualifications.insert(q.clone())?;
        let decision = AtomDecision {
            invocation: invocation.id(),
            leaf: leaf.id(),
            support: assessment.support,
            qualification: q.id(),
            theory: assessment.witness,
            outcome: result,
            reason,
        };
        let decision_id = decision.id();
        rows.atom_decisions.insert(decision)?;
        let assignment = match result {
            AtomOutcome::True => true,
            AtomOutcome::False => false,
            _ => continue,
        };
        let atom = need(&data.entry.atoms, leaf.atom)?;
        let evaluation = need(&data.entry.occurrences, atom.evaluation)?;
        let test = need(&data.entry.occurrences, leaf.test)?;
        // Compound predicates retain separate source evaluation atoms. Exact containment is
        // checked in addition to the native operand-link replay; no repeated evaluation is
        // unified by spelling, place or a shared interpreted domain.
        if atom.context != invocation.context
            || evaluation.source != test.source
            || evaluation.start < test.start
            || evaluation.end > test.end
        {
            continue;
        }

        for original in &originals {
            work = work
                .checked_add(1)
                .ok_or_else(|| invalid("atom restriction work overflow"))?;
            if work > 100_000 {
                return Err(ModelError::Limit {
                    owner: "atom_restriction",
                    limit: "candidate_pairs",
                    observed: work,
                    bound: 100_000,
                });
            }
            let oldq = rows
                .qualifications
                .get(original.qualification)
                .or_else(|| data.entry.qualifications.get(original.qualification))
                .ok_or_else(|| invalid("atom original qualification absent"))?
                .clone();
            if oldq.context != q.context
                || oldq.scope != q.scope
                || oldq.assumptions != AssumptionSet::empty_id()
            {
                continue;
            }
            let before = condition(data, rows, oldq.condition, &nodes)?;
            if !before.support().contains(&leaf.atom) {
                continue;
            }
            let replacement = if assignment {
                Diagram::always()
            } else {
                Diagram::never()
            };
            let admitted = match before.admitted_substitution(&[(leaf.atom, &replacement)], budget)
            {
                Ok(v) => v,
                Err(DiagramAdmissionError::Boundary(_)) => continue,
                Err(DiagramAdmissionError::Resource(e)) => return Err(e),
            };
            let (restricted, _retained) = admitted.into_parts();
            let qualification = AssertionQualification {
                condition: restricted.id(),
                assumptions: q.assumptions,
                ..oldq
            };
            let mut supports = rows
                .supports
                .iter()
                .filter(|s| s.assertion == original.id());
            let Some(original_support) = supports.next().cloned() else {
                continue;
            };
            if supports.next().is_some() {
                continue;
            }
            drop(supports);
            let earlier = need(&rows.sources, original_support.source)?;
            let owner::SupportSource::AnalysisDerivation { derivation } = earlier else {
                continue;
            };
            let theory_status = need(
                &rows.theory.witnesses,
                assessment
                    .witness
                    .ok_or_else(|| invalid("atom restriction has no theory witness"))?,
            )?
            .source_facts()
            .status;
            let status = analysis::policy::derive_status(&[
                (
                    analysis::policy::SupportRole::Support,
                    need(&rows.derivations, *derivation)?.status,
                ),
                (analysis::policy::SupportRole::Support, theory_status),
            ]);
            let restriction = AtomRestriction {
                invocation: invocation.id(),
                decision: decision_id,
                original: original.id(),
                original_support: original_support.id(),
                qualification: qualification.id(),
                status,
            };
            let source = owner::SupportSource::AtomRestriction {
                witness: restriction.id(),
            };
            let subject = owner::ObligationSubject::Transfer {
                transfer: original.transfer,
            };
            let evidence = owner::support::EvidencePremise::derived(
                &source,
                &restriction,
                &qualification,
                &restricted,
            )?;
            let (derivation, proposition, members, qualified) = owner::AnalysisDerivation::emit(
                invocation,
                definition,
                subject.id(),
                analysis::AnalysisChannel::Value,
                super::calls::CallPhase::Call,
                analysis::support::QualificationOperation::Conjunction,
                &[evidence],
                budget,
            )?;
            let generated = owner::SupportSource::AnalysisDerivation {
                derivation: derivation.id(),
            };
            let key = need(&rows.keys, original.transfer)?;
            let alternative = key.alternative(&qualified.qualification);
            let (condition, new_nodes) = qualified.condition.records();
            rows.conditions.insert(condition)?;
            for node in new_nodes {
                rows.nodes.insert(node)?;
            }
            rows.qualifications.insert(qualified.qualification)?;
            rows.atom_restrictions.insert(restriction)?;
            rows.sources.insert(source)?;
            rows.sources.insert(generated.clone())?;
            rows.subjects.insert(subject)?;
            rows.derivations.insert(derivation)?;
            rows.propositions.insert(proposition)?;
            for member in members {
                rows.premises.insert(member)?;
            }
            rows.supports.insert(TransferSupport {
                assertion: alternative.id(),
                source: generated.id(),
            })?;
            rows.alternatives.insert(alternative)?;
        }
    }
    Ok(())
}
pub(crate) fn invariants() -> Vec<Invariant> {
    let mut inputs = LocalData::validation_inputs();
    inputs.push(ValidationInput::of::<owner::AnalysisInvocation>(&["id"]));
    macro_rules! current {($($field:ident:$ty:ty,)*)=>{$(inputs.push(ValidationInput::of::<$ty>(&["id"]));)*};}
    crate::local_semantic_outputs!(current);
    crate::local_theory_outputs!(current);
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "located_atom_restriction_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                data: LocalData::new(budget),
                rows: LocalRecords::new(budget),
                invocations: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct Check {
    data: LocalData,
    rows: LocalRecords,
    invocations: Rows<owner::AnalysisInvocation>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if input.prefix() == Some(super::stages::PublicationBoundary::Facts) {
            self.data.visit(input.name(), batch)?;
            return Ok(());
        }
        self.visit(input.name(), batch)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! decode {($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME {self.rows.$field.decode(batch)?;return Ok(());})*};}
        crate::local_semantic_outputs!(decode);
        if self.rows.theory.visit(name, batch)? {
            return Ok(());
        }
        if name == owner::AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
            return Ok(());
        }
        if self.data.visit(name, batch)? {
            return Ok(());
        }
        Err(invalid("undeclared atom decision replay input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let mut expected = LocalRecords::new(&self.budget);
        for invocation in self.invocations.iter() {
            let rows = super::local_semantics::produce(
                &self.data,
                invocation,
                &super::local_semantics::definition().1,
                &self.budget,
            )?;
            for row in rows.atom_decisions.iter() {
                expected.atom_decisions.insert(row.clone())?;
            }
            for row in rows.atom_restrictions.iter() {
                expected.atom_restrictions.insert(row.clone())?;
                let q = need(&rows.qualifications, row.qualification)?;
                if self.rows.qualifications.get(q.id()) != Some(q) {
                    return Err(invalid(
                        "atom restriction qualification differs from replay",
                    ));
                }
                let original = need(&rows.alternatives, row.original)?;
                let derived = TransferAlternative {
                    qualification: q.id(),
                    ..original.clone()
                };
                if self.rows.alternatives.get(derived.id()) != Some(&derived) {
                    return Err(invalid(
                        "conditional transfer missing from atom restriction",
                    ));
                }
            }
        }
        for decision in expected.atom_decisions.iter() {
            let q = need(&self.rows.qualifications, decision.qualification)?;
            if matches!(
                decision.outcome,
                AtomOutcome::True | AtomOutcome::False | AtomOutcome::Mixed
            ) && q.assumptions == AssumptionSet::empty_id()
            {
                return Err(invalid("typing atom decision lost its premise"));
            }
        }
        macro_rules! compare {($($field:ident),*)=>{$(if self.rows.$field.len()!=expected.$field.len() || self.rows.$field.iter().any(|r|expected.$field.get(r.id())!=Some(r)) {return Err(invalid(concat!(stringify!($field)," differs from replay")));})*};}
        compare!(atom_decisions, atom_restrictions);
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["located_atom_restriction_replay"]
}
