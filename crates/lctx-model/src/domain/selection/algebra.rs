//! Attributed finite claims; no absence or runtime compatibility is inferred from spelling.
use super::*;
/// Public selector bound, also used when internal inventories evaluate independent batches.
pub const MAX_REQUIREMENTS: usize = 16;
use crate::domain::{
    charged::{ChargedMap, ChargedSet, StateCharge},
    conditions::{BooleanOperation, Diagram, DiagramAdmissionError, EvaluationAtom},
    normalized::Rows,
    resources::{Reservation, ResourceBudget},
    *,
};
const MAX_OBSERVATIONS: usize = 200_000;
const MAX_CONTEXT_WORK: usize = 100_000;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeContext {
    pub declaration: Context,
    pub owner: Id<normalized::entities::EntityRef>,
    pub instance: Id<value::Place>,
    pub evaluation: Id<source::Occurrence>,
    pub model: Id<models::ModelCatalog>,
}
impl RuntimeContext {
    pub fn validate(&self) -> Result<(), ModelError> {
        if !matches!(self.declaration, Context::Signature { .. }) {
            return Err(invalid(
                "runtime selection requires one exact signature declaration",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimContext {
    Declaration(Context),
    Runtime(RuntimeContext),
}
impl HeapSize for ClaimContext {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl ClaimContext {
    pub fn member(&self) -> Option<Id<catalog::CatalogMember>> {
        match self {
            Self::Declaration(c) => c.member(),
            Self::Runtime(c) => c.declaration.member(),
        }
    }
    pub fn analysis(&self) -> Id<attribution::AnalysisContext> {
        match self {
            Self::Declaration(c) => c.analysis(),
            Self::Runtime(c) => c.declaration.analysis(),
        }
    }
}
/// A condition is admitted against actual evaluation atoms, not a producer-selected digest.
pub struct RuntimeCondition {
    pub context: RuntimeContext,
    diagram: Diagram,
    _reservation: Box<dyn Reservation>,
}
impl RuntimeCondition {
    pub fn admit(
        context: RuntimeContext,
        diagram: Diagram,
        atoms: &Rows<EvaluationAtom>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        context.validate()?;
        for atom in diagram.support() {
            let row = atoms.get(*atom).ok_or_else(|| {
                invalid("runtime selection condition lacks its exact evaluation atom")
            })?;
            if row.context != context.declaration.analysis() || row.evaluation != context.evaluation
            {
                return Err(invalid(
                    "runtime selection condition crosses evaluation identity",
                ));
            }
        }
        let reservation = budget.reserve(
            "selection-runtime-condition",
            diagram.allocation_allowance(),
        )?;
        Ok(Self {
            context,
            diagram,
            _reservation: reservation,
        })
    }
}
pub struct Observation {
    pub context: ClaimContext,
    pub basis: EvidenceBasis,
    pub value: Option<bool>,
    pub evidence: Vec<Witness>,
    pub admissible: Vec<ClaimContext>,
}
pub struct Domain {
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub closure: Vec<Witness>,
    pub observations: Vec<Observation>,
}
#[derive(Debug)]
pub enum RequirementWitness {
    Positive {
        context: ClaimContext,
        basis: EvidenceBasis,
        evidence: Vec<Witness>,
    },
    Negative {
        context: ClaimContext,
        basis: EvidenceBasis,
        evidence: Vec<Witness>,
    },
    Conflict {
        context: ClaimContext,
        basis: EvidenceBasis,
        positive: Vec<Witness>,
        negative: Vec<Witness>,
    },
}
pub struct Classified {
    pub requirement: Requirement,
    pub outcome: Outcome,
    pub reason: Reason,
    pub witnesses: Vec<RequirementWitness>,
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub examined: usize,
    pub total: Option<usize>,
    pub closure: Vec<Witness>,
    admissible: ChargedSet<ClaimContext>,
    conditions: ChargedMap<ClaimContext, RuntimeCondition>,
    charge: StateCharge,
}
impl HeapSize for RuntimeCondition {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl Classified {
    pub fn admissible(&self) -> impl Iterator<Item = &ClaimContext> {
        self.admissible.iter()
    }
    pub fn clear_admissible(&mut self) {
        self.admissible = ChargedSet::default();
    }
    pub fn condition(&mut self, condition: RuntimeCondition) -> Result<(), ModelError> {
        let context = ClaimContext::Runtime(condition.context.clone());
        if !self.admissible.contains(&context) {
            return Err(invalid(
                "condition does not belong to an admissible exact runtime context",
            ));
        }
        self.conditions
            .insert(&mut self.charge, context, condition)?;
        Ok(())
    }
}
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
#[derive(Default)]
struct Claims {
    positive: Vec<Witness>,
    negative: Vec<Witness>,
    unknown: bool,
    admissible: std::collections::BTreeSet<ClaimContext>,
}
impl HeapSize for Claims {
    fn heap_bytes(&self) -> usize {
        self.positive.heap_bytes() + self.negative.heap_bytes() + self.admissible.heap_bytes()
    }
}
pub fn classify(
    requirement: &Requirement,
    domain: &Domain,
    budget: &ResourceBudget,
) -> Result<Classified, ModelError> {
    requirement.predicate.validate()?;
    let work = domain
        .observations
        .iter()
        .try_fold(domain.observations.len(), |n, o| {
            n.checked_add(o.evidence.len())
                .and_then(|n| n.checked_add(o.admissible.len()))
                .ok_or_else(|| invalid("selection observation size overflow"))
        })?;
    if work > MAX_OBSERVATIONS {
        return Err(invalid("selection observation work refused"));
    }
    let mut charge = StateCharge::new(budget, "selection-classify");
    let mut claims: ChargedMap<(ClaimContext, EvidenceBasis), Claims> = ChargedMap::default();
    for observation in &domain.observations {
        if observation.value.is_some() && observation.evidence.is_empty() {
            return Err(invalid("unattributed selection observation"));
        }
        if let ClaimContext::Runtime(runtime) = &observation.context {
            runtime.validate()?;
        }
        for context in &observation.admissible {
            if context.member() != observation.context.member()
                || context.analysis() != observation.context.analysis()
            {
                return Err(invalid(
                    "selection admissibility changes member or analysis context",
                ));
            }
        }
        claims.update(
            &mut charge,
            (observation.context.clone(), observation.basis),
            |claims| match observation.value {
                Some(true) => {
                    claims.positive.extend(observation.evidence.iter().cloned());
                    claims
                        .admissible
                        .extend(observation.admissible.iter().cloned());
                }
                Some(false) => claims.negative.extend(observation.evidence.iter().cloned()),
                None => claims.unknown = true,
            },
        )?;
    }
    let mut witnesses = Vec::new();
    let mut positives = 0;
    let mut negatives = 0;
    let mut conflicts = 0;
    let mut unknowns = 0;
    let mut admissible = ChargedSet::default();
    let mut output_charge = StateCharge::new(budget, "selection-classified");
    for ((context, basis), claim) in claims.iter() {
        if !claim.positive.is_empty() || !claim.negative.is_empty() {
            output_charge.grow(
                size_of::<RequirementWitness>()
                    + claim.positive.heap_bytes()
                    + claim.negative.heap_bytes(),
            )?;
        }
        let witness = if !claim.positive.is_empty() && !claim.negative.is_empty() {
            conflicts += 1;
            Some(RequirementWitness::Conflict {
                context: context.clone(),
                basis: *basis,
                positive: claim.positive.clone(),
                negative: claim.negative.clone(),
            })
        } else if !claim.positive.is_empty() {
            positives += 1;
            for context in &claim.admissible {
                admissible.insert(&mut output_charge, context.clone())?;
            }
            Some(RequirementWitness::Positive {
                context: context.clone(),
                basis: *basis,
                evidence: claim.positive.clone(),
            })
        } else if !claim.negative.is_empty() {
            negatives += 1;
            Some(RequirementWitness::Negative {
                context: context.clone(),
                basis: *basis,
                evidence: claim.negative.clone(),
            })
        } else {
            if claim.unknown {
                unknowns += 1;
            }
            None
        };
        if let Some(witness) = witness {
            witnesses.push(witness);
        }
    }
    let closed = domain.corpus_complete && domain.analyzer_complete && !domain.closure.is_empty();
    let (outcome, reason) = if requirement.quantifier == Quantifier::AllApplicable && negatives > 0
    {
        (Outcome::Contradicted, Reason::Counterexample)
    } else if conflicts > 0 {
        (Outcome::Conflicting, Reason::ComparableConflict)
    } else if requirement.quantifier == Quantifier::AnyApplicable && positives > 0 {
        (Outcome::Supported, Reason::Witness)
    } else if closed && claims.is_empty() {
        (Outcome::Contradicted, Reason::NoApplicableDomain)
    } else if closed && unknowns == 0 && positives == 0 && negatives == claims.len() {
        (Outcome::Contradicted, Reason::ClosedAbsence)
    } else if closed && unknowns == 0 && positives == claims.len() && positives > 0 {
        (Outcome::Supported, Reason::Witness)
    } else {
        (Outcome::Unresolved, Reason::IncompleteDomain)
    };
    output_charge.admit(&domain.closure)?;
    output_charge.admit(requirement)?;
    Ok(Classified {
        requirement: requirement.clone(),
        outcome,
        reason,
        witnesses,
        corpus_complete: domain.corpus_complete,
        analyzer_complete: domain.analyzer_complete,
        examined: claims.len(),
        total: closed.then_some(claims.len()),
        closure: domain.closure.clone(),
        admissible,
        conditions: ChargedMap::default(),
        charge: output_charge,
    })
}
fn intersect(a: &ClaimContext, b: &ClaimContext) -> Option<ClaimContext> {
    if a == b {
        return Some(a.clone());
    }
    if a.analysis() != b.analysis() {
        return None;
    }
    match (a, b) {
        (ClaimContext::Declaration(Context::Member { member, .. }), _)
            if b.member() == Some(*member) =>
        {
            Some(b.clone())
        }
        (_, ClaimContext::Declaration(Context::Member { member, .. }))
            if a.member() == Some(*member) =>
        {
            Some(a.clone())
        }
        (
            ClaimContext::Declaration(Context::Binding {
                member, candidate, ..
            }),
            ClaimContext::Declaration(Context::Signature {
                member: m,
                candidate: c,
                ..
            }),
        ) if member == m && candidate == c => Some(b.clone()),
        (
            ClaimContext::Declaration(Context::Signature {
                member, candidate, ..
            }),
            ClaimContext::Declaration(Context::Binding {
                member: m,
                candidate: c,
                ..
            }),
        ) if member == m && candidate == c => Some(a.clone()),
        _ => None,
    }
}
fn compatible_context(context: &ClaimContext) -> bool {
    matches!(
        context,
        ClaimContext::Declaration(
            Context::Member { .. } | Context::Binding { .. } | Context::Signature { .. }
        ) | ClaimContext::Runtime(_)
    )
}
fn condition<'a>(
    result: &'a Classified,
    context: &ClaimContext,
    always: &'a Diagram,
) -> Option<&'a Diagram> {
    match context {
        ClaimContext::Runtime(_) => result.conditions.get(context).map(|c| &c.diagram),
        _ => Some(always),
    }
}
struct Joined {
    diagram: Diagram,
    _reservation: Box<dyn Reservation>,
}
impl HeapSize for Joined {
    fn heap_bytes(&self) -> usize {
        0
    }
}
pub fn joint(
    results: &[&Classified],
    policy: JointPolicy,
    budget: &ResourceBudget,
) -> Result<JointApplicability, ModelError> {
    if policy == JointPolicy::IndependentRecords {
        return Ok(JointApplicability::IndependentRecords);
    }
    if results.is_empty() {
        return Ok(JointApplicability::CompatibleModeledContext);
    }
    if results.iter().any(|r| {
        r.outcome != Outcome::Supported
            || r.admissible.is_empty()
            || r.admissible.iter().any(|c| !compatible_context(c))
    }) {
        return Ok(JointApplicability::NotEstablished);
    }
    let always = Diagram::always();
    let mut charge = StateCharge::new(budget, "selection-joint");
    let mut contexts: ChargedMap<ClaimContext, Joined> = ChargedMap::default();
    let mut uncertain = false;
    let mut work = 0;
    for c in results[0].admissible.iter() {
        if let Some(diagram) = condition(results[0], c, &always) {
            let reservation =
                budget.reserve("selection-joint-condition", diagram.allocation_allowance())?;
            contexts.insert(
                &mut charge,
                c.clone(),
                Joined {
                    diagram: diagram.clone(),
                    _reservation: reservation,
                },
            )?;
        } else {
            uncertain = true;
        }
    }
    for result in &results[1..] {
        let mut next: ChargedMap<ClaimContext, Joined> = ChargedMap::default();
        for (a, left) in contexts.iter() {
            for b in result.admissible.iter() {
                work += 1;
                if work > MAX_CONTEXT_WORK {
                    return Ok(JointApplicability::NotEstablished);
                }
                let Some(c) = intersect(a, b) else {
                    if a.analysis() != b.analysis()
                        || matches!(a, ClaimContext::Runtime(_))
                        || matches!(b, ClaimContext::Runtime(_))
                    {
                        uncertain = true;
                    }
                    continue;
                };
                let Some(right) = condition(result, b, &always) else {
                    uncertain = true;
                    continue;
                };
                let combined =
                    match left
                        .diagram
                        .admitted_binary(right, BooleanOperation::Conjunction, budget)
                    {
                        Ok(combined) => combined,
                        Err(DiagramAdmissionError::Resource(e)) => return Err(e),
                        Err(DiagramAdmissionError::Boundary(_)) => {
                            uncertain = true;
                            continue;
                        }
                    };
                let (diagram, reservation) = combined.into_parts();
                if diagram.is_false() {
                    continue;
                }
                let joined = if let Some(previous) = next.get(&c) {
                    match previous.diagram.admitted_binary(
                        &diagram,
                        BooleanOperation::Disjunction,
                        budget,
                    ) {
                        Ok(joined) => {
                            let (diagram, reservation) = joined.into_parts();
                            Joined {
                                diagram,
                                _reservation: reservation,
                            }
                        }
                        Err(DiagramAdmissionError::Resource(e)) => return Err(e),
                        Err(DiagramAdmissionError::Boundary(_)) => {
                            uncertain = true;
                            continue;
                        }
                    }
                } else {
                    Joined {
                        diagram,
                        _reservation: reservation,
                    }
                };
                next.insert(&mut charge, c, joined)?;
            }
        }
        contexts = next;
    }
    if contexts.values().any(|c| !c.diagram.is_false()) {
        Ok(JointApplicability::CompatibleModeledContext)
    } else if !uncertain
        && results
            .iter()
            .all(|r| r.corpus_complete && r.analyzer_complete && r.total.is_some())
    {
        Ok(JointApplicability::ContradictoryModeledContext)
    } else {
        Ok(JointApplicability::NotEstablished)
    }
}
pub fn aggregate(
    results: &[&Classified],
    joint: JointApplicability,
    policy: JointPolicy,
) -> Outcome {
    if results.iter().any(|r| r.outcome == Outcome::Contradicted)
        || joint == JointApplicability::ContradictoryModeledContext
    {
        Outcome::Contradicted
    } else if results.iter().any(|r| r.outcome == Outcome::Conflicting) {
        Outcome::Conflicting
    } else if results.iter().any(|r| r.outcome == Outcome::Unresolved)
        || (policy == JointPolicy::RequireCompatible && joint == JointApplicability::NotEstablished)
    {
        Outcome::Unresolved
    } else {
        Outcome::Supported
    }
}
pub fn selection_digest(selection: &Selection) -> Result<ContentHash, ModelError> {
    if selection.requirements.len() > MAX_REQUIREMENTS {
        return Err(invalid("selection accepts at most sixteen terms"));
    }
    for r in &selection.requirements {
        r.predicate.validate()?;
    }
    let mut terms = selection
        .requirements
        .iter()
        .map(serde_json::to_vec)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ModelError::codec)?;
    terms.sort();
    terms.dedup();
    let mut key = KeySink::new("finite-selection/v1");
    selection.mode.encode(&mut key);
    selection.joint.encode(&mut key);
    for term in terms {
        ContentHash::of(&term).encode(&mut key);
    }
    Ok(key.finish())
}
