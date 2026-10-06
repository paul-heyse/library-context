//! Finite runtime exception results under entry to an actually completed source body.
//! Omitted/unsupported bodies provide no absence evidence. Native Raised types are unrelated.
use super::{
    enriched_records::{BodyExecution, ExecutionOutcome},
    summary_production::{SummaryData, SummaryRecords},
};
use crate::domain::{
    analysis, assertion, attribution, conditions, input, normalized::entities::EntityRef,
    resources::ResourceBudget, *,
};
#[derive(Debug, Clone, PartialEq, Eq, crate::Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "summary_exception_outcomes",
    rule = "finite_exception_body_summary"
)]
pub struct SummaryExceptionOutcome {
    #[model(key, premise)]
    pub invocation: Id<analysis::summary::AnalysisInvocation>,
    #[model(key, premise)]
    pub body: Id<BodyExecution>,
    pub input: Id<input::InputRevision>,
    pub context: Id<attribution::AnalysisContext>,
    pub owner: Id<EntityRef>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub outcome: Id<ExecutionOutcome>,
    /// Exact runtime exception identity; None means this admitted finite body returned normally.
    pub exception: Option<super::ExactRuntimeException>,
    pub status: analysis::policy::EvidenceStatus,
}
pub(super) fn derive(
    data: &SummaryData,
    invocation: &analysis::summary::AnalysisInvocation,
    out: &mut SummaryRecords,
    limit: u32,
    budget: &ResourceBudget,
) -> Result<usize, ModelError> {
    let invalid = |message: &str| ModelError::Invalid(message.into());
    let _iteration = budget.reserve(
        "summary-exception-iteration",
        data.exception_bodies
            .len()
            .saturating_mul(size_of::<Id<BodyExecution>>()),
    )?;
    if data.exception_bodies.len() > limit as usize {
        return Err(invalid("finite exception summary exceeds work limit"));
    }
    let mut work = 0;
    for body in data.exception_bodies.iter() {
        work += 1;
        let parent = data
            .enriched_invocations
            .get(body.invocation)
            .ok_or_else(|| invalid("exception body invocation absent"))?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        let q = data
            .entry
            .qualifications
            .get(body.qualification)
            .ok_or_else(|| invalid("exception body qualification absent"))?;
        if q.context != invocation.context {
            return Err(invalid("exception body crosses summary context"));
        }
        // This finite entry contract is exact and requires no typing premise.
        if q.condition != conditions::Diagram::always().id()
            || q.assumptions != assumptions::AssumptionSet::empty().id()
            || q.modality != attribution::Modality::Definite
            || q.approximation != assertion::Approximation::Exact
        {
            continue;
        }
        analysis::policy::behavioral_support(body.status, false)
            .map_err(|_| invalid("exception body support is unavailable"))?;
        let outcome = data
            .exception_values
            .get(body.outcome)
            .ok_or_else(|| invalid("exception body outcome absent"))?;
        let exception = match outcome {
            ExecutionOutcome::Raise { exception, .. } => Some(*exception),
            ExecutionOutcome::Normal | ExecutionOutcome::Return { .. } => None,
            _ => continue,
        };
        out.exception_outcomes.insert(SummaryExceptionOutcome {
            invocation: invocation.id(),
            body: body.id(),
            input: invocation.input,
            context: invocation.context,
            owner: body.owner,
            qualification: body.qualification,
            outcome: body.outcome,
            exception,
            status: body.status,
        })?;
    }
    Ok(work)
}

/// A raise-value continuation inside a handler/finalizer region needs the actual escaping site.
/// Lack of body completion is a refusal, never an assertion that no exception escapes.
pub(super) fn filter_raise(
    data: &SummaryData,
    outcomes: &normalized::Rows<SummaryExceptionOutcome>,
    owner: Id<EntityRef>,
    emission: &super::summary_path::PathEmission,
    input: Id<input::InputRevision>,
    budget: &ResourceBudget,
) -> Result<Result<Option<Id<SummaryExceptionOutcome>>, obligation::ObligationKind>, ModelError> {
    use super::summary_path::SummaryPathRoute;
    use crate::domain::value::PlaceRoot;
    if !matches!(emission.root, PlaceRoot::Raise { .. }) {
        return Ok(Ok(None));
    }
    let value_id = match emission.route {
        SummaryPathRoute::Call { value, .. } | SummaryPathRoute::Alias { value, .. } => value,
        SummaryPathRoute::EscapingRaise { .. } => {
            return Err(ModelError::Invalid(
                "exception filter received an already filtered route".into(),
            ));
        }
    };
    let Some(value) = data.entry.values.get(value_id) else {
        return Ok(Err(obligation::ObligationKind::MissingEvidence));
    };
    let Some(sink) = data.occurrence(value.sink) else {
        return Ok(Err(obligation::ObligationKind::MissingEvidence));
    };
    filter_sink(
        data,
        outcomes,
        owner,
        input,
        emission.branch.key().context,
        &sink,
        budget,
    )
}
fn filter_sink(
    data: &SummaryData,
    outcomes: &normalized::Rows<SummaryExceptionOutcome>,
    owner: Id<EntityRef>,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
    sink: &source::properties::OccurrenceProperties,
    budget: &ResourceBudget,
) -> Result<Result<Option<Id<SummaryExceptionOutcome>>, obligation::ObligationKind>, ModelError> {
    use crate::domain::source::SyntaxKind;
    let reason = obligation::ObligationKind::UnsupportedControlFlow;
    let _iteration = budget.reserve(
        "summary-raise-handler-filter",
        data.occurrence_values()
            .count()
            .saturating_mul(size_of::<Id<source::Occurrence>>())
            .saturating_add(outcomes.len().saturating_mul(64)),
    )?;
    let contains = |node: &source::properties::OccurrenceProperties| {
        node.source == sink.source && node.start <= sink.start && node.end >= sink.end
    };
    let mut owned_nodes = charged::ChargedSet::default();
    let mut ownership_charge = charged::StateCharge::new(budget, "summary-raise-owner-index");
    for row in data.entry.owners.iter().filter(|row| row.entity == owner) {
        owned_nodes.insert(&mut ownership_charge, row.occurrence)?;
    }
    let owned = |node: &source::properties::OccurrenceProperties| owned_nodes.contains(&node.id());
    if !owned(sink) {
        return Ok(Err(obligation::ObligationKind::MissingEvidence));
    }
    if !data.occurrence_values().any(|node| {
        contains(&node)
            && owned(&node)
            && matches!(node.syntax_kind, SyntaxKind::StmtTry | SyntaxKind::StmtWith)
    }) {
        return Ok(Ok(None));
    }
    let Some(raise) = data
        .occurrence_values()
        .filter(|node| contains(node) && owned(node) && node.syntax_kind == SyntaxKind::StmtRaise)
        .min_by_key(|node| node.end - node.start)
    else {
        return Ok(Err(reason));
    };
    let mut results = outcomes
        .iter()
        .filter(|row| row.owner == owner && row.context == context && row.input == input);
    let Some(result) = results.next() else {
        return Ok(Err(obligation::ObligationKind::IncompleteCoverage));
    };
    if results.next().is_some() {
        return Ok(Err(obligation::ObligationKind::AmbiguousBinding));
    }
    let Some(ExecutionOutcome::Raise { site, .. }) = data.exception_values.get(result.outcome)
    else {
        return Ok(Err(reason));
    };
    let Some(site) = data.occurrence(*site) else {
        return Ok(Err(obligation::ObligationKind::MissingEvidence));
    };
    if (site.source, site.start, site.end, site.syntax_kind)
        != (raise.source, raise.start, raise.end, raise.syntax_kind)
    {
        return Ok(Err(reason));
    }
    Ok(Ok(Some(result.id())))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn exact_exception_escape_filter_requires_same_actual_site_and_retains_unknown() {
        use crate::domain::{
            normalized::{Rows, entities::OccurrenceOwnership},
            source::{Occurrence, OccurrenceRole, SyntaxKind},
        };
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = SummaryData::new(&b);
        let node = |start, end, syntax_kind| Occurrence {
            source: id(1),
            start,
            end,
            syntax_kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![],
        };
        let sink = node(30, 40, SyntaxKind::ExprName);
        let raise = node(24, 40, SyntaxKind::StmtRaise);
        let frame = node(10, 80, SyntaxKind::StmtTry);
        for n in [&sink, &raise, &frame] {
            data.entry.occurrences.insert(n.clone()).unwrap();
            data.entry
                .owners
                .insert(OccurrenceOwnership {
                    occurrence: n.id(),
                    owner: id(2),
                    entity: id(3),
                })
                .unwrap();
        }
        let mut outcomes = Rows::new(&b);
        let run = |data: &SummaryData, rows: &Rows<SummaryExceptionOutcome>| {
            filter_sink(
                data,
                rows,
                id(3),
                id(4),
                id(5),
                &source::properties::OccurrenceProperties::from_row(&sink),
                &b,
            )
            .unwrap()
        };
        assert_eq!(
            run(&data, &outcomes),
            Err(obligation::ObligationKind::IncompleteCoverage)
        );
        let outcome = ExecutionOutcome::Raise {
            site: raise.id(),
            exception: super::super::ExactRuntimeException::ValueError,
        };
        data.exception_values.insert(outcome.clone()).unwrap();
        let result = SummaryExceptionOutcome {
            invocation: id(6),
            body: id(7),
            input: id(4),
            context: id(5),
            owner: id(3),
            qualification: id(8),
            outcome: outcome.id(),
            exception: Some(super::super::ExactRuntimeException::ValueError),
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
        };
        outcomes.insert(result.clone()).unwrap();
        assert_eq!(run(&data, &outcomes), Ok(Some(result.id())));
        outcomes = Rows::new(&b);
        data.exception_values
            .insert(ExecutionOutcome::Normal)
            .unwrap();
        outcomes
            .insert(SummaryExceptionOutcome {
                outcome: ExecutionOutcome::Normal.id(),
                exception: None,
                ..result.clone()
            })
            .unwrap();
        assert_eq!(
            run(&data, &outcomes),
            Err(obligation::ObligationKind::UnsupportedControlFlow)
        );
        outcomes = Rows::new(&b);
        let replaced = node(50, 60, SyntaxKind::StmtRaise);
        data.entry.occurrences.insert(replaced.clone()).unwrap();
        let outcome = ExecutionOutcome::Raise {
            site: replaced.id(),
            exception: super::super::ExactRuntimeException::RuntimeError,
        };
        data.exception_values.insert(outcome.clone()).unwrap();
        outcomes
            .insert(SummaryExceptionOutcome {
                outcome: outcome.id(),
                exception: Some(super::super::ExactRuntimeException::RuntimeError),
                ..result
            })
            .unwrap();
        assert_eq!(
            run(&data, &outcomes),
            Err(obligation::ObligationKind::UnsupportedControlFlow)
        );
        data.entry.occurrences = Rows::new(&b);
        data.entry.occurrences.insert(sink.clone()).unwrap();
        assert_eq!(
            run(&data, &outcomes),
            Ok(None),
            "unframed scope is preserved"
        );
        data.entry.owners = Rows::new(&b);
        assert_eq!(
            run(&data, &outcomes),
            Err(obligation::ObligationKind::MissingEvidence),
            "missing ownership cannot bypass the handler filter"
        );
    }
}
