//! Preserve both the call basis and the Local value basis when following an argument.
use super::{
    Output,
    build::{Data, invalid, need},
};
use crate::domain::{
    assertion::AssertionQualification,
    assumptions::ResolvedAssumptions,
    conditions::{BooleanOperation, Diagram},
    resources::ResourceBudget,
    source::SourceArtifact,
    *,
};

pub(super) fn intersect(
    data: &Data,
    call: &AssertionQualification,
    local: &AssertionQualification,
    alias: Option<&super::handoffs::ValueSource>,
    source: Id<SourceArtifact>,
    output: &mut Output,
    budget: &ResourceBudget,
) -> Result<AssertionQualification, ModelError> {
    if call.context != local.context
        || !data
            .ownership
            .within(source, data.ownership.scope(call.scope)?)?
        || !data
            .ownership
            .within(source, data.ownership.scope(local.scope)?)?
    {
        return Err(invalid(
            "Structural argument qualification has a foreign context/scope",
        ));
    }
    let entry = &data.handoffs.entry;
    let _charge = budget.reserve(
        "structural qualification nodes",
        entry
            .condition_nodes
            .len()
            .checked_mul(2048)
            .ok_or_else(|| invalid("Structural condition allocation overflow"))?,
    )?;
    let nodes = entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let a = Diagram::from_records(need(&entry.conditions, call.condition)?, &nodes)?;
    let b = Diagram::from_records(need(&entry.conditions, local.condition)?, &nodes)?;
    let admitted = a
        .admitted_binary(&b, BooleanOperation::Conjunction, budget)
        .map_err(|e| match e {
            conditions::DiagramAdmissionError::Resource(error) => error,
            conditions::DiagramAdmissionError::Boundary(boundary) => invalid(format!(
                "Structural argument condition boundary: {boundary:?}"
            )),
        })?;
    let (mut condition, _condition_charge) = admitted.into_parts();
    let _basis_charge = budget.reserve(
        "structural qualification assumptions",
        assumptions::MAX_ASSUMPTIONS * 16 * size_of::<assumptions::AssumptionSetMember>(),
    )?;
    let mut alias_bases = Vec::new();
    let mut _alias_condition_charge = None;
    if let Some(alias) = alias {
        let alias_condition = super::handoffs::named_condition(&data.handoffs, alias, budget)?;
        let admitted = condition
            .admitted_binary(&alias_condition, BooleanOperation::Conjunction, budget)
            .map_err(|e| match e {
                conditions::DiagramAdmissionError::Resource(error) => error,
                conditions::DiagramAdmissionError::Boundary(boundary) => {
                    invalid(format!("Structural alias condition boundary: {boundary:?}"))
                }
            })?;
        let (result, reservation) = admitted.into_parts();
        condition = result;
        _alias_condition_charge = Some(reservation);
        let super::handoffs::ValueSource::Named {
            observation,
            reaching,
            definition,
            inventory,
            region,
            ..
        } = alias
        else {
            return Err(invalid("Structural alias must be named"));
        };
        let mut qualifications = vec![
            need(&entry.use_observations, *observation)?.qualification,
            need(&entry.reaching, *reaching)?.qualification,
            need(&entry.definition_observations, *definition)?.qualification,
            need(&entry.inventories, *inventory)?.qualification,
            need(
                &entry.source_views,
                need(&entry.inventories, *inventory)?.view,
            )?
            .qualification,
        ];
        if let Some(region) = region {
            qualifications.push(need(&entry.regions, *region)?.qualification);
        }
        for id in qualifications {
            let q = need(&entry.qualifications, id)?;
            if q.context != call.context
                || !data
                    .ownership
                    .within(source, data.ownership.scope(q.scope)?)?
            {
                return Err(invalid(
                    "Structural alias qualification has a foreign context/scope",
                ));
            }
            alias_bases.push(data.assumptions.resolve(q.assumptions)?);
        }
    }
    let call_basis = data.assumptions.resolve(call.assumptions)?;
    let local_basis = data.assumptions.resolve(local.assumptions)?;
    let basis = ResolvedAssumptions::union(
        [&call_basis, &local_basis]
            .into_iter()
            .chain(alias_bases.iter()),
    )?;
    let qualification = AssertionQualification {
        context: call.context,
        scope: source::CoverageScope::Artifact { artifact: source }.id(),
        condition: condition.id(),
        modality: call.modality.weakest(local.modality),
        approximation: call.approximation.join(local.approximation),
        assumptions: basis.set.id(),
    };
    // Scope rows are predecessor vocabulary; never fabricate a missing artifact scope.
    data.ownership.scope(qualification.scope)?;
    let (record, nodes) = condition.records();
    output.flow_conditions.insert(record)?;
    for node in nodes {
        output.flow_condition_nodes.insert(node)?;
    }
    output.flow_assumption_sets.insert(basis.set)?;
    for member in basis.members {
        output.flow_assumption_members.insert(member)?;
    }
    output
        .conclusion_qualifications
        .insert(qualification.clone())?;
    Ok(qualification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        assertion::Approximation,
        assumptions::{Assumption, AssumptionSet},
        attribution::Modality,
        source::CoverageScope,
    };
    fn id<R: Record>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn load<R: Record>(data: &mut Data, rows: &[R]) {
        assert!(data.visit(R::NAME, &R::encode(rows).unwrap()).unwrap());
    }
    #[test]
    fn singleton_conditional_basis_is_not_promoted_or_erased() {
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut data = Data::new(&budget);
        let artifact = SourceArtifact::from_bytes(id(1), "api.py".into(), b"pass\n").unwrap();
        load(&mut data, std::slice::from_ref(&artifact));
        let input_scope = CoverageScope::Input {
            input: artifact.input,
        };
        let artifact_scope = CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        load(&mut data, &[input_scope.clone(), artifact_scope.clone()]);
        let a = Assumption::TypeConformance {
            observation: id(2),
            support: id(3),
        };
        let b = Assumption::TypeConformance {
            observation: id(4),
            support: id(5),
        };
        load(&mut data, &[a.clone(), b.clone()]);
        let basis_a = AssumptionSet::new([a.id()]).unwrap();
        let basis_b = AssumptionSet::new([b.id()]).unwrap();
        data.assumptions.insert(&basis_a).unwrap();
        data.assumptions.insert(&basis_b).unwrap();
        let conditional = Diagram::from_atom(id(6));
        for diagram in [Diagram::always(), conditional.clone()] {
            let (condition, nodes) = diagram.records();
            data.handoffs.entry.conditions.insert(condition).unwrap();
            for node in nodes {
                data.handoffs.entry.condition_nodes.insert(node).unwrap();
            }
        }
        let call = AssertionQualification {
            context: id(7),
            scope: input_scope.id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: basis_a.set.id(),
        };
        let local = AssertionQualification {
            scope: artifact_scope.id(),
            condition: conditional.id(),
            modality: Modality::Candidate,
            approximation: Approximation::Over,
            assumptions: basis_b.set.id(),
            ..call.clone()
        };
        let mut output = Output::new(&budget);
        let q = intersect(
            &data,
            &call,
            &local,
            None,
            artifact.id(),
            &mut output,
            &budget,
        )
        .unwrap();
        assert_eq!(q.condition, conditional.id());
        assert_eq!(q.modality, Modality::Candidate);
        assert_eq!(q.approximation, Approximation::Over);
        assert_eq!(q.scope, artifact_scope.id());
        assert_eq!(
            q.assumptions,
            AssumptionSet::new([a.id(), b.id()]).unwrap().set.id()
        );
        assert_eq!(output.flow_assumption_members.len(), 2);
        assert!(output.flow_conditions.get(conditional.id()).is_some());
        // Conditional target/call basis is equally authoritative; conjunction is symmetric.
        let conditional_call = AssertionQualification {
            condition: conditional.id(),
            ..call.clone()
        };
        let unconditional_local = AssertionQualification {
            condition: Diagram::always().id(),
            ..local.clone()
        };
        let target_q = intersect(
            &data,
            &conditional_call,
            &unconditional_local,
            None,
            artifact.id(),
            &mut output,
            &budget,
        )
        .unwrap();
        assert_eq!(target_q.condition, conditional.id());
        assert_eq!(target_q.assumptions, q.assumptions);
        let opposite = conditional.not().unwrap();
        let (opposite_record, opposite_nodes) = opposite.records();
        data.handoffs
            .entry
            .conditions
            .insert(opposite_record)
            .unwrap();
        for node in opposite_nodes {
            data.handoffs.entry.condition_nodes.insert(node).unwrap();
        }
        let exclusive_local = AssertionQualification {
            condition: opposite.id(),
            ..local.clone()
        };
        let false_q = intersect(
            &data,
            &conditional_call,
            &exclusive_local,
            None,
            artifact.id(),
            &mut output,
            &budget,
        )
        .unwrap();
        assert_eq!(
            false_q.condition,
            Diagram::never().id(),
            "incompatible qualified evidence cannot become an unconditional flow"
        );
        assert_eq!(false_q.assumptions, q.assumptions);
        let denied = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            intersect(
                &data,
                &call,
                &local,
                None,
                artifact.id(),
                &mut output,
                &denied
            ),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(denied.reserved(), 0);
        let foreign = AssertionQualification {
            context: id(8),
            ..local.clone()
        };
        assert!(
            intersect(
                &data,
                &call,
                &foreign,
                None,
                artifact.id(),
                &mut output,
                &budget
            )
            .is_err()
        );
        let missing = AssertionQualification {
            assumptions: id(9),
            ..local
        };
        assert!(
            intersect(
                &data,
                &call,
                &missing,
                None,
                artifact.id(),
                &mut output,
                &budget
            )
            .is_err()
        );
        drop(output);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn conditional_named_alias_is_conjoined_and_its_assumptions_survive() {
        use crate::domain::{
            attribution::*, flow::*, flow_inventory::*, lexical::BindingEventKind,
            structural::handoffs::ValueSource,
        };
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let mut data = Data::new(&budget);
        let artifact = SourceArtifact::from_bytes(id(1), "api.py".into(), b"pass\n").unwrap();
        load(&mut data, std::slice::from_ref(&artifact));
        let scope = CoverageScope::Input {
            input: artifact.input,
        };
        load(
            &mut data,
            &[
                scope.clone(),
                CoverageScope::Artifact {
                    artifact: artifact.id(),
                },
            ],
        );
        let empty = AssumptionSet::new([]).unwrap();
        data.assumptions.insert(&empty).unwrap();
        let mut bases = Vec::new();
        for n in 10..13 {
            let assumption = Assumption::TypeConformance {
                observation: id(n),
                support: id(n),
            };
            load(&mut data, std::slice::from_ref(&assumption));
            let basis = AssumptionSet::new([assumption.id()]).unwrap();
            data.assumptions.insert(&basis).unwrap();
            bases.push(basis);
        }
        let conditional = Diagram::from_atom(id(20));
        for diagram in [Diagram::always(), conditional.clone()] {
            let (condition, nodes) = diagram.records();
            data.handoffs.entry.conditions.insert(condition).unwrap();
            for node in nodes {
                data.handoffs.entry.condition_nodes.insert(node).unwrap();
            }
        }
        let call = AssertionQualification {
            context: id(21),
            scope: scope.id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: empty.set.id(),
        };
        let local = call.clone();
        let use_q = AssertionQualification {
            condition: conditional.id(),
            assumptions: bases[0].set.id(),
            ..call.clone()
        };
        let reach_q = AssertionQualification {
            assumptions: bases[1].set.id(),
            ..use_q.clone()
        };
        let def_q = AssertionQualification {
            assumptions: bases[2].set.id(),
            ..call.clone()
        };
        for q in [&call, &use_q, &reach_q, &def_q] {
            data.handoffs
                .entry
                .qualifications
                .insert((*q).clone())
                .unwrap();
        }
        let observation = FlowUseObservation {
            qualification: use_q.id(),
            use_: id(22),
            scope: id(23),
            annotation: false,
        };
        let support = FlowUseSupport {
            assertion: observation.id(),
            run: id(24),
            surface: id(25),
            evidence: id(26),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        let reaching = FlowReachingObservation {
            qualification: reach_q.id(),
            use_: id(22),
            target: id(27),
            loop_carried: false,
        };
        let definition = FlowDefinitionObservation {
            qualification: def_q.id(),
            definition: id(28),
            scope: id(23),
            kind: BindingEventKind::Assignment,
            value: None,
        };
        data.handoffs
            .entry
            .use_observations
            .insert(observation.clone())
            .unwrap();
        data.handoffs
            .entry
            .use_supports
            .insert(support.clone())
            .unwrap();
        data.handoffs
            .entry
            .reaching
            .insert(reaching.clone())
            .unwrap();
        data.handoffs
            .entry
            .definition_observations
            .insert(definition.clone())
            .unwrap();
        let view = FlowSourceViewObservation {
            qualification: call.id(),
            source: artifact.id(),
            original_content: artifact.content,
            view_content: artifact.content,
            byte_len: artifact.byte_len,
            renamed_type_checking: 0,
        };
        let state = CandidateState {
            kind: FlowCandidateKind::Bound,
            pruned: false,
            loop_expanded: false,
            unattached: false,
            reachability: Some(use_q.id()),
            narrowing: Some(use_q.id()),
            narrowing_unavailable: false,
            narrowing_precision_lost: false,
            condition_unavailable: false,
            reachability_lost: false,
            mapped_count: 1,
        };
        let (inventory, _, _) = FlowUseInventoryObservation::new(
            use_q.id(),
            observation.use_,
            observation.scope,
            view.id(),
            &[state],
            &[(0, reaching.id(), id(29))],
        )
        .unwrap();
        data.handoffs.entry.source_views.insert(view).unwrap();
        data.handoffs
            .entry
            .inventories
            .insert(inventory.clone())
            .unwrap();
        // Typed kernel control over a caller-supplied replayed alias; native admission has separate controls.
        let alias = ValueSource::Named {
            observation: observation.id(),
            support: support.id(),
            reaching: reaching.id(),
            reaching_support: id(29),
            definition: definition.id(),
            definition_support: id(30),
            inventory: inventory.id(),
            inventory_support: id(32),
            region: None,
            region_support: None,
            coverage: id(33),
        };
        let mut output = Output::new(&budget);
        let q = intersect(
            &data,
            &call,
            &local,
            Some(&alias),
            artifact.id(),
            &mut output,
            &budget,
        )
        .unwrap();
        assert_eq!(
            q.condition,
            conditional.id(),
            "Always call/Local cannot erase a conditional alias"
        );
        assert_eq!(
            q.assumptions,
            ResolvedAssumptions::union(bases.iter()).unwrap().set.id()
        );
        assert!(output.flow_conditions.get(conditional.id()).is_some());
        let mut forged = alias.clone();
        if let ValueSource::Named { region_support, .. } = &mut forged {
            *region_support = Some(id(34));
        }
        assert!(
            intersect(
                &data,
                &call,
                &local,
                Some(&forged),
                artifact.id(),
                &mut output,
                &budget
            )
            .is_err(),
            "unpaired optional region proof cannot replay"
        );
        let unconditional = FlowUseObservation {
            qualification: call.id(),
            ..observation
        };
        let use_support = FlowUseSupport {
            assertion: unconditional.id(),
            ..support
        };
        data.handoffs
            .entry
            .use_observations
            .insert(unconditional.clone())
            .unwrap();
        data.handoffs
            .entry
            .use_supports
            .insert(use_support.clone())
            .unwrap();
        if let ValueSource::Named {
            observation,
            support,
            ..
        } = &mut forged
        {
            *observation = unconditional.id();
            *support = use_support.id();
        }
        if let ValueSource::Named { region_support, .. } = &mut forged {
            *region_support = None;
        }
        assert!(
            intersect(
                &data,
                &call,
                &local,
                Some(&forged),
                artifact.id(),
                &mut output,
                &budget
            )
            .is_err(),
            "erasing both region refs cannot turn Always into a conditional reaching proof"
        );
    }
}
