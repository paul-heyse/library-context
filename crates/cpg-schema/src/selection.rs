//! Pure contextual selection. Callers supply attributed observations and scoped closure;
//! absence of a row is never interpreted as a negative observation.
use crate::{Id, wire::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod catalog;

pub const POLICY_REVISION: u32 = 1;
pub const MAX_OBSERVATIONS: usize = 200_000;
pub const MAX_CONTEXT_WORK: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub context: SelectionContext,
    pub basis: EvidenceBasis,
    pub value: Option<bool>,
    pub evidence: Vec<WitnessEvidence>,
    /// Concrete alternatives where this same declaration is applicable. Empty means unknown.
    pub admissible: Vec<SelectionContext>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub closure: Vec<WitnessEvidence>,
    pub observations: Vec<Observation>,
}
#[derive(Debug, Clone)]
pub struct Classified {
    pub result: RequirementResult,
    pub admissible: BTreeSet<SelectionContext>,
    /// Runtime alternatives require a condition from their exact evaluation identity.
    pub conditions: BTreeMap<SelectionContext, crate::condition_kernel::BoundedCondition>,
}

/// Reduce comparable claims first; an opposite observation in another overload is not conflict.
pub fn classify(requirement: &Requirement, domain: &Domain) -> Result<Classified, WireError> {
    if domain.observations.len() > MAX_OBSERVATIONS
        || domain
            .observations
            .iter()
            .map(|o| o.evidence.len() + o.admissible.len())
            .sum::<usize>()
            > MAX_OBSERVATIONS
    {
        return Err(WireError("resource_refused: selection observations".into()));
    }
    type ComparableClaims = (
        Vec<WitnessEvidence>,
        Vec<WitnessEvidence>,
        bool,
        BTreeSet<SelectionContext>,
    );
    let mut claims: BTreeMap<(SelectionContext, EvidenceBasis), ComparableClaims> = BTreeMap::new();
    for o in &domain.observations {
        let (positive, negative, unknown, admissible) =
            claims.entry((o.context.clone(), o.basis)).or_default();
        match o.value {
            Some(true) => {
                positive.extend(o.evidence.clone());
                admissible.extend(o.admissible.clone());
            }
            Some(false) => negative.extend(o.evidence.clone()),
            None => *unknown = true,
        }
        // A positive/negative observation must be attributed; a bare Boolean is not a witness.
        if o.value.is_some() && o.evidence.is_empty() {
            return Err(WireError("unattributed selection observation".into()));
        }
    }
    let mut witnesses = Vec::new();
    let mut positives = 0;
    let mut negatives = 0;
    let mut conflicts = 0;
    let mut unknowns = 0;
    let mut admissible = BTreeSet::new();
    for ((context, basis), (positive, negative, unknown, contexts)) in &claims {
        if !positive.is_empty() && !negative.is_empty() {
            conflicts += 1;
            witnesses.push(RequirementWitness::Conflict {
                context: context.clone(),
                basis: *basis,
                positive: positive.clone(),
                negative: negative.clone(),
            });
        } else if !positive.is_empty() {
            positives += 1;
            admissible.extend(contexts.clone());
            witnesses.push(RequirementWitness::Positive {
                context: context.clone(),
                basis: *basis,
                evidence: positive.clone(),
            });
        } else if !negative.is_empty() {
            negatives += 1;
            witnesses.push(RequirementWitness::Negative {
                context: context.clone(),
                basis: *basis,
                evidence: negative.clone(),
            });
        } else if *unknown {
            unknowns += 1;
        }
    }
    let closed = domain.corpus_complete && domain.analyzer_complete && !domain.closure.is_empty();
    let (outcome, reason) =
        if requirement.quantifier() == Quantifier::AllApplicable && negatives > 0 {
            (
                SelectionOutcome::Contradicted,
                SelectionReason::Counterexample,
            )
        } else if conflicts > 0 {
            (
                SelectionOutcome::Conflicting,
                SelectionReason::ComparableConflict,
            )
        } else if requirement.quantifier() == Quantifier::AnyApplicable && positives > 0 {
            (SelectionOutcome::Supported, SelectionReason::Witness)
        } else if closed && claims.is_empty() {
            (
                SelectionOutcome::Contradicted,
                SelectionReason::NoApplicableDomain,
            )
        } else if closed && unknowns == 0 && positives == 0 && negatives == claims.len() {
            (
                SelectionOutcome::Contradicted,
                SelectionReason::ClosedAbsence,
            )
        } else if closed && unknowns == 0 && positives == claims.len() && positives > 0 {
            (SelectionOutcome::Supported, SelectionReason::Witness)
        } else {
            (
                SelectionOutcome::Unresolved,
                SelectionReason::IncompleteDomain,
            )
        };
    Ok(Classified {
        result: RequirementResult {
            requirement: requirement.clone(),
            outcome,
            witnesses,
            reason,
            coverage: SelectionCoverage {
                corpus_complete: domain.corpus_complete,
                analyzer_complete: domain.analyzer_complete,
                evaluation_complete: true,
                examined: claims.len() as u64,
                total: closed.then_some(claims.len() as u64),
                closure: domain.closure.clone(),
            },
        },
        admissible,
        conditions: BTreeMap::new(),
    })
}
fn member(context: &SelectionContext) -> Option<PublicMemberId> {
    match context {
        SelectionContext::Member { member }
        | SelectionContext::Binding { member, .. }
        | SelectionContext::Signature { member, .. }
        | SelectionContext::Configuration { member, .. }
        | SelectionContext::Source { member, .. }
        | SelectionContext::Scenario { member, .. }
        | SelectionContext::Runtime { member, .. } => Some(*member),
        SelectionContext::Release { .. } => None,
    }
}
fn intersect(a: &SelectionContext, b: &SelectionContext) -> Option<SelectionContext> {
    if a == b {
        return Some(a.clone());
    }
    // Member-level declaration predicates do not constrain a signature/instance dimension.
    match (a, b) {
        (SelectionContext::Member { member: m }, _) if member(b) == Some(*m) => Some(b.clone()),
        (_, SelectionContext::Member { member: m }) if member(a) == Some(*m) => Some(a.clone()),
        (
            SelectionContext::Binding {
                member: ma,
                binding: x,
            },
            SelectionContext::Signature {
                member: mb,
                binding: y,
                ..
            },
        ) if ma == mb && x == y => Some(b.clone()),
        (
            SelectionContext::Signature {
                member: ma,
                binding: x,
                ..
            },
            SelectionContext::Binding {
                member: mb,
                binding: y,
            },
        ) if ma == mb && x == y => Some(a.clone()),
        _ => None,
    }
}
/// Intersect the entire conjunction. Pairwise compatibility is insufficient.
pub fn joint(results: &[Classified], policy: JointPolicy) -> JointApplicability {
    if policy == JointPolicy::IndependentRecords {
        return JointApplicability::IndependentRecords;
    }
    if results.is_empty() {
        return JointApplicability::CompatibleModeledContext;
    }
    if results
        .iter()
        .any(|r| r.result.outcome != SelectionOutcome::Supported)
    {
        return JointApplicability::NotEstablished;
    }
    let admissible = |context: &SelectionContext| {
        matches!(
            context,
            SelectionContext::Member { .. }
                | SelectionContext::Binding { .. }
                | SelectionContext::Signature { .. }
                | SelectionContext::Runtime { .. }
        )
    };
    if results
        .iter()
        .any(|r| r.admissible.iter().any(|c| !admissible(c)))
    {
        return JointApplicability::NotEstablished;
    }
    use crate::condition_kernel::BoundedCondition;
    let condition = |r: &Classified, c: &SelectionContext| -> Option<BoundedCondition> {
        if matches!(c, SelectionContext::Runtime { .. }) {
            r.conditions.get(c).cloned()
        } else {
            Some(BoundedCondition::always())
        }
    };
    let mut contexts = BTreeMap::new();
    let mut uncertain = false;
    let mut work = 0;
    for c in &results[0].admissible {
        match condition(&results[0], c) {
            Some(d) if d.boundary().is_none() => {
                contexts.insert(c.clone(), d);
            }
            _ => uncertain = true,
        }
    }
    for r in &results[1..] {
        let mut next = BTreeMap::<SelectionContext, BoundedCondition>::new();
        for (a, left) in &contexts {
            for b in &r.admissible {
                work += 1;
                if work > MAX_CONTEXT_WORK {
                    return JointApplicability::NotEstablished;
                }
                let Some(c) = intersect(a, b) else {
                    if matches!(a, SelectionContext::Runtime { .. })
                        || matches!(b, SelectionContext::Runtime { .. })
                    {
                        uncertain = true;
                    }
                    continue;
                };
                let Some(right) = condition(r, b) else {
                    uncertain = true;
                    continue;
                };
                let conjunction = left.and(&right);
                match conjunction.diagram() {
                    Err(_) => uncertain = true,
                    Ok(d) if d.is_false() => {}
                    Ok(_) => {
                        if let Some(previous) = next.get_mut(&c) {
                            *previous = previous.or(&conjunction);
                            if previous.boundary().is_some() {
                                uncertain = true;
                            }
                        } else {
                            next.insert(c, conjunction);
                        }
                    }
                }
            }
        }
        contexts = next;
    }
    if results.iter().any(|r| r.admissible.is_empty()) {
        return JointApplicability::NotEstablished;
    }
    if contexts
        .values()
        .any(|c| c.diagram().is_ok_and(|d| !d.is_false()))
    {
        JointApplicability::CompatibleModeledContext
    } else if !uncertain
        && results
            .iter()
            .all(|r| r.result.coverage.corpus_complete && r.result.coverage.analyzer_complete)
    {
        JointApplicability::ContradictoryModeledContext
    } else {
        JointApplicability::NotEstablished
    }
}
pub fn aggregate(
    results: &[Classified],
    joint: JointApplicability,
    policy: JointPolicy,
) -> SelectionOutcome {
    if results
        .iter()
        .any(|r| r.result.outcome == SelectionOutcome::Contradicted)
        || joint == JointApplicability::ContradictoryModeledContext
    {
        return SelectionOutcome::Contradicted;
    }
    if results
        .iter()
        .any(|r| r.result.outcome == SelectionOutcome::Conflicting)
    {
        return SelectionOutcome::Conflicting;
    }
    if results
        .iter()
        .any(|r| r.result.outcome == SelectionOutcome::Unresolved)
        || (policy == JointPolicy::RequireCompatible && joint == JointApplicability::NotEstablished)
    {
        return SelectionOutcome::Unresolved;
    }
    SelectionOutcome::Supported
}
/// Stable key includes normalized defaults and semantic ordering; page size is deliberately absent.
pub fn selection_digest(selection: &Selection) -> Result<String, WireError> {
    use sha2::{Digest as _, Sha256};
    let mut terms = selection
        .requirements
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    terms.sort();
    terms.dedup();
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            POLICY_REVISION,
            selection.mode,
            selection.joint,
            terms
        ))?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u8) -> Id {
        Id([n; 16])
    }
    fn ctx(n: u8) -> SelectionContext {
        SelectionContext::Signature {
            member: PublicMemberId::from_storage(id(1)),
            binding: BindingId::from_storage(id(2)),
            signature: SignatureId::from_storage(id(n)),
        }
    }
    fn obs(n: u8, value: Option<bool>) -> Observation {
        Observation {
            context: ctx(n),
            basis: EvidenceBasis::SourceDeclaration,
            value,
            evidence: vec![WitnessEvidence::Fact { id: id(n) }],
            admissible: vec![ctx(n)],
        }
    }
    fn req(q: Quantifier) -> Requirement {
        Requirement::DeclaresParameter {
            name: Text::new("timeout".into()).unwrap(),
            quantifier: q,
        }
    }
    fn domain(observations: Vec<Observation>, closed: bool) -> Domain {
        Domain {
            corpus_complete: closed,
            analyzer_complete: closed,
            closure: vec![WitnessEvidence::Domain { id: id(99) }],
            observations,
        }
    }
    #[test]
    fn overloads_do_not_conflict_and_quantifiers_differ() {
        let d = domain(vec![obs(3, Some(true)), obs(4, Some(false))], true);
        assert_eq!(
            classify(&req(Quantifier::AnyApplicable), &d)
                .unwrap()
                .result
                .outcome,
            SelectionOutcome::Supported
        );
        assert_eq!(
            classify(&req(Quantifier::AllApplicable), &d)
                .unwrap()
                .result
                .outcome,
            SelectionOutcome::Contradicted
        );
    }
    #[test]
    fn absence_requires_closure_and_empty_is_not_vacuous() {
        for q in [Quantifier::AnyApplicable, Quantifier::AllApplicable] {
            assert_eq!(
                classify(&req(q), &domain(vec![], true))
                    .unwrap()
                    .result
                    .reason,
                SelectionReason::NoApplicableDomain
            );
            assert_eq!(
                classify(&req(q), &domain(vec![obs(3, Some(false))], false))
                    .unwrap()
                    .result
                    .outcome,
                if q == Quantifier::AllApplicable {
                    SelectionOutcome::Contradicted
                } else {
                    SelectionOutcome::Unresolved
                }
            );
            assert_eq!(
                classify(&req(q), &domain(vec![obs(3, None)], true))
                    .unwrap()
                    .result
                    .outcome,
                SelectionOutcome::Unresolved
            );
        }
    }
    #[test]
    fn comparable_conflict_precedes_existential_witness_but_not_clean_counterexample() {
        let d = domain(
            vec![
                obs(3, Some(true)),
                obs(3, Some(false)),
                obs(4, Some(true)),
                obs(5, Some(false)),
            ],
            true,
        );
        assert_eq!(
            classify(&req(Quantifier::AnyApplicable), &d)
                .unwrap()
                .result
                .outcome,
            SelectionOutcome::Conflicting
        );
        assert_eq!(
            classify(&req(Quantifier::AllApplicable), &d)
                .unwrap()
                .result
                .outcome,
            SelectionOutcome::Contradicted
        );
    }
    #[test]
    fn whole_conjunction_intersection_and_missing_context() {
        let r = req(Quantifier::AnyApplicable);
        let mut results = vec![];
        for ns in [[3, 4], [4, 5], [3, 5]] {
            results.push(
                classify(
                    &r,
                    &domain(ns.into_iter().map(|n| obs(n, Some(true))).collect(), true),
                )
                .unwrap(),
            );
        }
        assert_eq!(
            joint(&results, JointPolicy::RequireCompatible),
            JointApplicability::ContradictoryModeledContext
        );
        assert_eq!(
            joint(&results[..2], JointPolicy::RequireCompatible),
            JointApplicability::CompatibleModeledContext
        );
        results[1].admissible.clear();
        assert_eq!(
            joint(&results, JointPolicy::RequireCompatible),
            JointApplicability::NotEstablished
        );
    }
    #[test]
    fn runtime_conditions_require_same_instance_evaluation_and_bounded_proof() {
        use crate::{
            Digest,
            condition::Atom,
            condition_kernel::{BoundedCondition, KernelBoundary},
        };
        let runtime = SelectionContext::Runtime {
            member: PublicMemberId::from_storage(id(1)),
            binding: BindingId::from_storage(id(2)),
            signature: SignatureId::from_storage(id(3)),
            owner: id(4),
            instance: id(5),
            evaluation: id(6),
            model: Digest([7; 32]),
        };
        let mut a = classify(
            &req(Quantifier::AnyApplicable),
            &domain(
                vec![Observation {
                    context: runtime.clone(),
                    basis: EvidenceBasis::BoundedModel,
                    value: Some(true),
                    evidence: vec![WitnessEvidence::Fact { id: id(8) }],
                    admissible: vec![runtime.clone()],
                }],
                true,
            ),
        )
        .unwrap();
        let mut b = a.clone();
        assert_eq!(
            joint(&[a.clone(), b.clone()], JointPolicy::RequireCompatible),
            JointApplicability::NotEstablished
        );
        let condition = BoundedCondition::atom(Atom::Truthy { place: "x".into() });
        a.conditions.insert(runtime.clone(), condition.clone());
        b.conditions.insert(runtime.clone(), condition.not());
        assert_eq!(
            joint(&[a.clone(), b.clone()], JointPolicy::RequireCompatible),
            JointApplicability::ContradictoryModeledContext
        );
        b.conditions.insert(
            runtime.clone(),
            BoundedCondition::unknown(KernelBoundary::NodeLimit),
        );
        assert_eq!(
            joint(&[a.clone(), b.clone()], JointPolicy::RequireCompatible),
            JointApplicability::NotEstablished
        );
        let mut other = runtime.clone();
        if let SelectionContext::Runtime { evaluation, .. } = &mut other {
            *evaluation = id(9);
        }
        b.admissible = BTreeSet::from([other.clone()]);
        b.conditions = BTreeMap::from([(other, condition)]);
        assert_eq!(
            joint(&[a, b], JointPolicy::RequireCompatible),
            JointApplicability::NotEstablished
        );
    }
    #[test]
    fn digest_ignores_term_order_and_duplicates() {
        let a = req(Quantifier::AnyApplicable);
        let b = req(Quantifier::AllApplicable);
        let x = Selection {
            requirements: vec![a.clone(), b.clone()],
            ..Default::default()
        };
        let y = Selection {
            requirements: vec![b, a.clone(), a],
            ..Default::default()
        };
        assert_eq!(selection_digest(&x).unwrap(), selection_digest(&y).unwrap());
    }
}
