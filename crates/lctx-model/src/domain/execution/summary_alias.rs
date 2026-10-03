//! Continue a proven value through one exact, unrebound local definition. This is not Entry
//! authority and cannot justify guard substitution or object-state stability.
use super::summary_path::{PathEmission, SummaryPathRoute, SummaryPathWitness};
use crate::domain::{
    analysis::{
        self,
        native::{NativeAssertionPremise, NativeQualification},
        support::SourceFacts,
    },
    assertion::*,
    attribution::*,
    composition::CompositionOperand,
    conditions::{entry::EntryData, *},
    flow::*,
    normalized::{Rows, entities::*},
    obligation::ObligationKind,
    resources::ResourceBudget,
    transfer::{summary::*, *},
    value::*,
    *,
};
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ObligationKind> {
    rows.get(id).ok_or(ObligationKind::MissingEvidence)
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
#[allow(
    clippy::too_many_arguments,
    reason = "Public summary-alias proof keeps independent source, native, vocabulary and alias witness authorities explicit."
)]
pub fn derive(
    data: &EntryData,
    native: &Rows<NativeQualification>,
    vocabulary: &conditions::rebase::GuardCatalog<'_>,
    invocation: &analysis::summary::AnalysisInvocation,
    source: &dyn CompositionOperand,
    facts: SourceFacts,
    value_support: Id<FlowValueSupport>,
    reaching_support: Id<FlowReachingSupport>,
    definition_support: Id<FlowDefinitionSupport>,
    use_support: Id<FlowUseSupport>,
    budget: &ResourceBudget,
) -> Result<Result<PathEmission, ObligationKind>, ModelError> {
    let descriptor = source.descriptor();
    let setup = (|| {
        let vs = need(&data.value_supports, value_support)?;
        let value = need(&data.values, vs.assertion)?;
        let rs = need(&data.reaching_supports, reaching_support)?;
        let reaching = need(&data.reaching, rs.assertion)?;
        let ds = need(&data.definition_supports, definition_support)?;
        let definition = need(&data.definition_observations, ds.assertion)?;
        let us = need(&data.use_supports, use_support)?;
        let observed = need(&data.use_observations, us.assertion)?;
        if value.through_call
            || value.use_ != reaching.use_
            || value.use_ != observed.use_
            || observed.annotation
            || reaching.loop_carried
            || !matches!(definition.kind, lexical::BindingEventKind::Assignment)
        {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let run = vs.attribution().ok_or(ObligationKind::MissingEvidence)?.run;
        for actual in [rs.attribution(), ds.attribution(), us.attribution()] {
            if actual.ok_or(ObligationKind::MissingEvidence)?.run != run {
                return Err(ObligationKind::IncompatibleContexts);
            }
        }
        let provider = need(&data.runs, run)?;
        if (provider.input, provider.context) != (invocation.input, invocation.context) {
            return Err(ObligationKind::IncompatibleContexts);
        }
        let q = need(&data.qualifications, value.qualification)?;
        for id in [
            q.id(),
            observed.qualification,
            reaching.qualification,
            definition.qualification,
        ] {
            let qualification = need(&data.qualifications, id)?;
            if (qualification.context, qualification.scope)
                != (descriptor.context, descriptor.scope)
                || qualification.modality != Modality::Definite
                || qualification.approximation != Approximation::Exact
            {
                return Err(ObligationKind::Approximation);
            }
        }
        if facts.qualification != source.qualification().id() {
            return Err(ObligationKind::MissingEvidence);
        }
        let output = vocabulary
            .places
            .get(&descriptor.output)
            .ok_or(ObligationKind::MissingEvidence)?;
        if output.path != AccessPath::empty().id() {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let Some(PlaceRoot::Occurrence { occurrence }) = vocabulary.roots.get(&output.root) else {
            return Err(ObligationKind::CapturedStateUnavailable);
        };
        if definition.value != Some(*occurrence)
            || need(&data.targets, reaching.target)?
                != &(ReachingDefinition::Bound {
                    definition: definition.definition,
                })
        {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let use_ = need(&data.uses, value.use_)?;
        let bound = need(&data.definitions, definition.definition)?;
        if bound.place != use_.place {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let place = need(&data.places, use_.place)?;
        if place.path != AccessPath::empty().id()
            || !matches!(need(&data.roots, place.root)?, PlaceRoot::Local { .. })
        {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let lexical = need(&data.lexical_scopes, observed.scope)?;
        if definition.scope != observed.scope {
            return Err(ObligationKind::ScopeBoundary);
        }
        let EntityRef::Callable { callable } = need(&data.refs, descriptor.owner)? else {
            return Err(ObligationKind::ScopeBoundary);
        };
        let CallableEntity::Source { declaration, .. } = need(&data.callables, *callable)? else {
            return Err(ObligationKind::ScopeBoundary);
        };
        if lexical.owner != *declaration
            || !data
                .owners
                .iter()
                .any(|o| o.occurrence == use_.occurrence && o.entity == descriptor.owner)
            || !data
                .owners
                .iter()
                .any(|o| o.occurrence == value.sink && o.entity == descriptor.owner)
        {
            return Err(ObligationKind::ScopeBoundary);
        }
        let mut same_run = 0;
        for candidate in data.reaching.iter().filter(|r| r.use_ == use_.id()) {
            if data.reaching_supports.iter().any(|s| {
                s.assertion == candidate.id() && s.attribution().is_some_and(|a| a.run == run)
            }) {
                same_run += 1;
                if candidate.id() != reaching.id() {
                    return Err(ObligationKind::CapturedStateUnavailable);
                }
            }
        }
        if same_run != 1 {
            return Err(ObligationKind::CapturedStateUnavailable);
        }
        let premises = [
            (
                NativeAssertionPremise::Value {
                    assertion: value.id(),
                    support: vs.id(),
                },
                value.qualification,
            ),
            (
                NativeAssertionPremise::Use {
                    assertion: observed.id(),
                    support: us.id(),
                },
                observed.qualification,
            ),
            (
                NativeAssertionPremise::Reaching {
                    assertion: reaching.id(),
                    support: rs.id(),
                },
                reaching.qualification,
            ),
            (
                NativeAssertionPremise::Definition {
                    assertion: definition.id(),
                    support: ds.id(),
                },
                definition.qualification,
            ),
        ];
        let mut status = facts.status;
        for (premise, q) in &premises {
            let native = native
                .iter()
                .find(|n| {
                    n.premise == premise.id()
                        && n.qualification == *q
                        && n.fidelity == Fidelity::NativeStructural
                        && n.family == FactFamily::Flow
                })
                .ok_or(ObligationKind::MissingEvidence)?;
            status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                [status, native.status],
            );
        }
        let root = match value.kind {
            FlowSinkKind::Return => PlaceRoot::Return {
                callable: *declaration,
            },
            FlowSinkKind::Yield => PlaceRoot::Yield {
                callable: *declaration,
            },
            FlowSinkKind::Raise => PlaceRoot::Raise {
                callable: *declaration,
            },
            FlowSinkKind::Argument | FlowSinkKind::Definition => PlaceRoot::Occurrence {
                occurrence: value.sink,
            },
        };
        Ok((value, observed, reaching, definition, root, status))
    })();
    let (value, observed, reaching, definition, root, status) = match setup {
        Ok(r) => r,
        Err(r) => return Ok(Err(r)),
    };
    let reservation = budget.reserve(
        "summary-alias-conditions",
        data.condition_nodes
            .len()
            .saturating_mul(2048)
            .saturating_add(source.condition().allocation_allowance())
            .saturating_add(4096),
    )?;
    let nodes = data.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let mut condition = source.condition().clone();
    let mut charge = charged::StateCharge::new(budget, "summary-alias-combined-condition");
    for id in [
        value.qualification,
        observed.qualification,
        reaching.qualification,
        definition.qualification,
    ] {
        let q = data
            .qualifications
            .get(id)
            .ok_or_else(|| invalid("alias qualification absent"))?;
        let native = Diagram::from_records(
            data.conditions
                .get(q.condition)
                .ok_or_else(|| invalid("alias condition absent"))?,
            &nodes,
        )?;
        let joined = match condition.admitted_binary(&native, BooleanOperation::Conjunction, budget)
        {
            Ok(r) => r,
            Err(DiagramAdmissionError::Boundary(e)) => return Ok(Err(obligation::from_kernel(e))),
            Err(DiagramAdmissionError::Resource(e)) => return Err(e),
        };
        let (next, temporary) = joined.into_parts();
        charge.grow(next.allocation_allowance())?;
        condition = next;
        drop(temporary);
    }
    let qualification = AssertionQualification {
        condition: condition.id(),
        ..source.qualification().clone()
    };
    let place = Place {
        root: root.id(),
        path: AccessPath::empty().id(),
    };
    let key = TransferKey::from_descriptor(TransferDescriptor {
        output: place.id(),
        kind: TransferKind::Derived,
        provenance: ProvenanceClass::DerivedSummary,
        ..descriptor
    });
    let branch = TransferBranch::new(key, qualification.clone(), condition, budget)?;
    let route = SummaryPathRoute::Alias {
        value: value.id(),
        value_support,
        use_observation: observed.id(),
        use_support,
        reaching: reaching.id(),
        reaching_support,
        definition: definition.id(),
        definition_support,
    };
    let source = source.premise();
    let witness = SummaryPathWitness {
        invocation: invocation.id(),
        transfer: branch.key().id(),
        qualification: qualification.id(),
        source: source.id(),
        route: route.id(),
        status,
        heuristic: facts.heuristic,
    };
    Ok(Ok(PathEmission {
        route,
        witness,
        source,
        branch,
        root,
        place,
        _charge: reservation,
    }))
}
