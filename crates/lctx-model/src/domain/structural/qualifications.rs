//! Preserve both the call basis and the Local value basis when following an argument.
use super::{Output, build::{Data, invalid, need}};
use crate::domain::{assertion::AssertionQualification, assumptions::ResolvedAssumptions,
    conditions::{BooleanOperation, Diagram}, resources::ResourceBudget, source::SourceArtifact, *};

pub(super) fn intersect(
    data: &Data,
    call: &AssertionQualification,
    local: &AssertionQualification,
    source: Id<SourceArtifact>,
    output: &mut Output,
    budget: &ResourceBudget,
) -> Result<AssertionQualification, ModelError> {
    if call.context != local.context
        || !data.ownership.within(source, data.ownership.scope(call.scope)?)?
        || !data.ownership.within(source, data.ownership.scope(local.scope)?)?
    {
        return Err(invalid("Structural argument qualification has a foreign context/scope"));
    }
    let entry = &data.handoffs.entry;
    let _charge = budget.reserve("structural qualification nodes", entry.condition_nodes.len()
        .checked_mul(2048).ok_or_else(|| invalid("Structural condition allocation overflow"))?)?;
    let nodes = entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let a = Diagram::from_records(need(&entry.conditions, call.condition)?, &nodes)?;
    let b = Diagram::from_records(need(&entry.conditions, local.condition)?, &nodes)?;
    let admitted = a.admitted_binary(&b, BooleanOperation::Conjunction, budget)
        .map_err(|e| match e {
            conditions::DiagramAdmissionError::Resource(error) => error,
            conditions::DiagramAdmissionError::Boundary(boundary) => invalid(format!("Structural argument condition boundary: {boundary:?}")),
        })?;
    let (condition, _condition_charge) = admitted.into_parts();
    let _basis_charge = budget.reserve("structural qualification assumptions",
        assumptions::MAX_ASSUMPTIONS * 4 * size_of::<assumptions::AssumptionSetMember>())?;
    let call_basis = data.assumptions.resolve(call.assumptions)?;
    let local_basis = data.assumptions.resolve(local.assumptions)?;
    let basis = ResolvedAssumptions::union([&call_basis, &local_basis])?;
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
    for node in nodes { output.flow_condition_nodes.insert(node)?; }
    output.flow_assumption_sets.insert(basis.set)?;
    for member in basis.members { output.flow_assumption_members.insert(member)?; }
    output.conclusion_qualifications.insert(qualification.clone())?;
    Ok(qualification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{assertion::Approximation, assumptions::{Assumption, AssumptionSet}, attribution::Modality, source::CoverageScope};
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
        let input_scope = CoverageScope::Input { input: artifact.input };
        let artifact_scope = CoverageScope::Artifact { artifact: artifact.id() };
        load(&mut data, &[input_scope.clone(), artifact_scope.clone()]);
        let a = Assumption::TypeConformance { observation: id(2), support: id(3) };
        let b = Assumption::TypeConformance { observation: id(4), support: id(5) };
        load(&mut data, &[a.clone(), b.clone()]);
        let basis_a = AssumptionSet::new([a.id()]).unwrap();
        let basis_b = AssumptionSet::new([b.id()]).unwrap();
        data.assumptions.insert(&basis_a).unwrap();
        data.assumptions.insert(&basis_b).unwrap();
        let conditional = Diagram::from_atom(id(6));
        for diagram in [Diagram::always(), conditional.clone()] {
            let (condition, nodes) = diagram.records();
            data.handoffs.entry.conditions.insert(condition).unwrap();
            for node in nodes { data.handoffs.entry.condition_nodes.insert(node).unwrap(); }
        }
        let call = AssertionQualification {
            context: id(7), scope: input_scope.id(), condition: Diagram::always().id(),
            modality: Modality::Definite, approximation: Approximation::Exact, assumptions: basis_a.set.id(),
        };
        let local = AssertionQualification {
            scope: artifact_scope.id(), condition: conditional.id(),
            modality: Modality::Candidate, approximation: Approximation::Over,
            assumptions: basis_b.set.id(), ..call.clone()
        };
        let mut output = Output::new(&budget);
        let q = intersect(&data, &call, &local, artifact.id(), &mut output, &budget).unwrap();
        assert_eq!(q.condition, conditional.id());
        assert_eq!(q.modality, Modality::Candidate);
        assert_eq!(q.approximation, Approximation::Over);
        assert_eq!(q.scope, artifact_scope.id());
        assert_eq!(q.assumptions, AssumptionSet::new([a.id(), b.id()]).unwrap().set.id());
        assert_eq!(output.flow_assumption_members.len(), 2);
        assert!(output.flow_conditions.get(conditional.id()).is_some());
        // Conditional target/call basis is equally authoritative; conjunction is symmetric.
        let conditional_call = AssertionQualification { condition: conditional.id(), ..call.clone() };
        let unconditional_local = AssertionQualification { condition: Diagram::always().id(), ..local.clone() };
        let target_q = intersect(&data, &conditional_call, &unconditional_local, artifact.id(), &mut output, &budget).unwrap();
        assert_eq!(target_q.condition, conditional.id());
        assert_eq!(target_q.assumptions, q.assumptions);
        let opposite = conditional.not().unwrap();
        let (opposite_record, opposite_nodes) = opposite.records();
        data.handoffs.entry.conditions.insert(opposite_record).unwrap();
        for node in opposite_nodes {data.handoffs.entry.condition_nodes.insert(node).unwrap();}
        let exclusive_local = AssertionQualification {condition: opposite.id(), ..local.clone()};
        let false_q = intersect(&data, &conditional_call, &exclusive_local, artifact.id(), &mut output, &budget).unwrap();
        assert_eq!(false_q.condition, Diagram::never().id(), "incompatible qualified evidence cannot become an unconditional flow");
        assert_eq!(false_q.assumptions, q.assumptions);
        let denied = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(intersect(&data, &call, &local, artifact.id(), &mut output, &denied), Err(ModelError::Resource {..})));
        assert_eq!(denied.reserved(), 0);
        let foreign = AssertionQualification { context: id(8), ..local.clone() };
        assert!(intersect(&data, &call, &foreign, artifact.id(), &mut output, &budget).is_err());
        let missing = AssertionQualification { assumptions: id(9), ..local };
        assert!(intersect(&data, &call, &missing, artifact.id(), &mut output, &budget).is_err());
        drop(output);
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
