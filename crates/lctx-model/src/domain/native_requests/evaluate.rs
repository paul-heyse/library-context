use super::*;
use crate::domain::{
    Key, KeySink, ModelError, Record,
    attribution::CoverageStatus,
    conditions::{BooleanOperation, Diagram, DiagramAdmissionError},
    obligation::{self, VerdictInput},
    resources::ResourceBudget,
    value::{Literal, Predicate},
};
use std::collections::BTreeMap;

/// An existing opaque path can remain unexamined when exact callable admission is unavailable.
/// This retains the original proof frame and does not admit any scalar assignments.
pub fn unexamined(
    request: &ExactRequest<'_>,
    entry: &crate::domain::conditions::entry::DerivedEntryValue,
    cause: ObligationKind,
    path: &NativePath,
    budget: &ResourceBudget,
) -> Result<Assessment, ModelError> {
    let w = entry.witness();
    if request.owner != w.owner
        || request.formal != w.formal
        || path.owner != w.owner
        || path.input != entry.place().id()
        || path.qualification.context != w.context
    {
        return Err(ModelError::Invalid(
            "unexamined native request crosses its actual Entry/path frame".into(),
        ));
    }
    path.qualification.validate()?;
    let charge = budget.reserve(
        "native-unexamined-path",
        path.proof
            .len()
            .saturating_mul(size_of::<RowRef>())
            .saturating_add(request.value.bytes().saturating_mul(3))
            .saturating_add(8192),
    )?;
    request.value.validate()?;
    let mut proof = super::ingress::entry_premises(entry);
    proof.extend_from_slice(&path.proof);
    proof.sort();
    proof.dedup();
    Ok(Assessment {
        qualification:path.qualification.id(),
        path: path.identity,
        original_condition: path.condition.id(),
        restricted_result: None,
        verdict: Verdict::Unknown,
        exact: ExactOutcome::Unknown,
        basis: Basis::Unexamined,
        assumptions: request.assumptions,
        proof,
        work: Work::default(),
        reason: Some(cause),
        unexamined: path.condition.support().len(),
        rendered: None,
        presentation_truncated: false,
        proof_truncated: false,
        _charge: Some(charge),
    })
}

/// Inspect one admitted path. `verdict` describes this finite path under its stated model;
/// `CompatibleUnderMayModel` never asserts a concrete execution or whole-operation support.
/// Negative S0 question proofs remain a separate owner and do not enter this operation.
#[expect(
    clippy::too_many_arguments,
    reason = "The pure operation keeps request, resolved context, admitted path, checked links, coverage, obligations and resource limits explicit"
)]
pub fn assess(
    request: &ExactRequest<'_>,
    context: &NativeContext,
    path: &NativePath,
    atoms: &[CheckedAtom],
    coverage: CoverageStatus,
    open: &[ObligationKind],
    limits: Limits,
    budget: &ResourceBudget,
) -> Result<Assessment, ModelError> {
    if request.owner != context.owner
        || request.formal != context.formal
        || path.owner != context.owner
        || path.input != context.place
        || path.qualification.context != context.context
    {
        return Err(ModelError::Invalid(
            "native request crosses resolved path/formal context".into(),
        ));
    }
    let mut out = Assessment {
        qualification:path.qualification.id(),
        path: path.identity,
        original_condition: path.condition.id(),
        restricted_result: None,
        verdict: Verdict::Unknown,
        exact: ExactOutcome::Unknown,
        basis: Basis::Unexamined,
        assumptions: request.assumptions,
        proof: vec![],
        work: Work {
            peak_bdd_nodes: path.condition.node_count(),
            ..Work::default()
        },
        reason: None,
        unexamined: path.condition.support().len(),
        rendered: None,
        presentation_truncated: false,
        proof_truncated: false,
        _charge: None,
    };
    let proof_charge = match budget.reserve(
        "native-original-path-proof",
        (context.proof.len() + path.proof.len())
            .saturating_mul(size_of::<RowRef>())
            .saturating_add(8192),
    ) {
        Ok(charge) => charge,
        Err(ModelError::Resource { .. }) => {
            out.reason = Some(ObligationKind::ResourceRefused);
            out.proof_truncated = true;
            return Ok(out);
        }
        Err(error) => return Err(error),
    };
    out.proof.extend_from_slice(&context.proof);
    out.proof.extend_from_slice(&path.proof);
    out.proof.sort();
    out.proof.dedup();
    out._charge = Some(proof_charge);
    if path.kind != crate::domain::transfer::TransferKind::Identity {
        out.reason = Some(ObligationKind::ConditionTransferUnsupported);
        return Ok(out);
    }
    let baseline = obligation::verdict(VerdictInput {
        condition: Some(&path.condition),
        open,
        coverage,
        approximation: path.qualification.approximation,
        modality: path.qualification.modality,
    });
    if baseline.verdict == Verdict::NotAnalyzed || baseline.verdict == Verdict::Unknown {
        out.verdict = baseline.verdict;
        out.reason = baseline.reason;
        out.exact = if baseline.verdict == Verdict::NotAnalyzed {
            ExactOutcome::NotAnalyzed
        } else {
            ExactOutcome::Unknown
        };
        return Ok(out);
    }
    let allowance = path
        .condition
        .allocation_allowance()
        .saturating_mul(3)
        .saturating_add(request.value.bytes().saturating_mul(3))
        .saturating_add(atoms.len().min(limits.atoms).saturating_mul(8192))
        .saturating_add(atoms.iter().take(limits.atoms).fold(0usize, |sum, a| {
            sum.saturating_add(a.proof.len().saturating_mul(size_of::<RowRef>()))
        }))
        .saturating_add(limits.render_terms.min(4096).saturating_mul(4096))
        .saturating_add(8192);
    let charge = match budget.reserve("native-exact-request", allowance) {
        Ok(charge) => charge,
        Err(ModelError::Resource { .. }) => {
            out.reason = Some(ObligationKind::ResourceRefused);
            return Ok(out);
        }
        Err(error) => return Err(error),
    };
    out._charge = Some(charge);
    request.value.validate()?;
    let value = request.value.literal();
    let mut assignments = BTreeMap::new();
    let mut assignment_proofs = BTreeMap::new();
    for atom in atoms {
        if out.work.atoms_examined >= limits.atoms {
            out.reason = Some(ObligationKind::ConditionWorkLimit);
            return Ok(out);
        }
        out.work.atoms_examined += 1;
        if !path.condition.support().contains(&atom.atom) {
            continue;
        }
        if (atom.owner, atom.formal, atom.context)
            != (context.owner, context.formal, context.context)
        {
            out.reason = Some(ObligationKind::IncompatibleContexts);
            return Ok(out);
        }
        // An Entry proof scoped to a narrower evaluation cannot assign outside that scope.
        if !atom.coverage.is_true() {
            let scope_cost = path
                .condition
                .node_count()
                .saturating_mul(atom.coverage.node_count());
            if out.work.bdd_preflight_pairs.saturating_add(scope_cost) > limits.bdd_pairs {
                out.reason = Some(ObligationKind::ConditionWorkLimit);
                return Ok(out);
            }
            out.work.bdd_preflight_pairs += scope_cost;
            let covered = match path.condition.admitted_binary(
                &atom.coverage,
                BooleanOperation::Conjunction,
                budget,
            ) {
                Ok(value) => value,
                Err(error) => {
                    out.reason = Some(boundary(error));
                    return Ok(out);
                }
            };
            if covered.id() != path.condition.id() {
                continue;
            }
        }
        let predicate_cost = atom.literals.len().max(1);
        if out
            .work
            .predicate_comparisons
            .saturating_add(predicate_cost)
            > limits.predicate_comparisons
        {
            out.reason = Some(ObligationKind::ConditionWorkLimit);
            return Ok(out);
        }
        out.work.predicate_comparisons += predicate_cost;
        let Some(truth) = evaluate(atom, &value, request.assumptions) else {
            continue;
        };
        if assignments
            .get(&atom.atom)
            .is_some_and(|previous| *previous != truth)
        {
            out.reason = Some(ObligationKind::MissingEvidence);
            return Ok(out);
        }
        if !assignments.contains_key(&atom.atom) && assignments.len() >= limits.assignments {
            out.reason = Some(ObligationKind::ConditionWorkLimit);
            return Ok(out);
        }
        assignments.insert(atom.atom, truth);
        out.unexamined = path
            .condition
            .support()
            .len()
            .saturating_sub(assignments.len());
        assignment_proofs.entry(atom.atom).or_insert(&atom.proof);
    }
    let mut fixed = assignments
        .iter()
        .map(|(atom, truth)| (*atom, *truth))
        .collect::<Vec<_>>();
    let cost = path.condition.node_count().saturating_mul(fixed.len());
    if out.work.bdd_preflight_pairs.saturating_add(cost) > limits.bdd_pairs {
        out.reason = Some(ObligationKind::ConditionWorkLimit);
        return Ok(out);
    }
    out.work.bdd_preflight_pairs += cost;
    let reduced = match path.condition.restrict_atoms(&fixed) {
        Ok(condition) => condition,
        Err(error) => {
            out.reason = Some(obligation::from_kernel(error));
            return Ok(out);
        }
    };
    out.work.assignments_applied = fixed.len();
    out.work.peak_bdd_nodes = out.work.peak_bdd_nodes.max(reduced.node_count());
    if fixed.is_empty() && !path.condition.is_true() && !path.condition.is_false() {
        out.reason = Some(ObligationKind::EntryValueUnknown);
        return Ok(out);
    }
    // Drop unnecessary assignment citations deterministically while the same work budget lasts.
    // A bounded valid superset is retained if optional minimization cannot finish.
    if reduced.is_false() {
        let mut at = 0;
        while at < fixed.len() {
            let cost = path
                .condition
                .node_count()
                .saturating_mul(fixed.len().saturating_sub(1));
            if out.work.bdd_preflight_pairs.saturating_add(cost) > limits.bdd_pairs {
                out.proof_truncated = true;
                break;
            }
            let mut trial = fixed.clone();
            trial.remove(at);
            out.work.bdd_preflight_pairs += cost;
            match path.condition.restrict_atoms(&trial) {
                Ok(diagram) if diagram.is_false() => {
                    fixed = trial;
                }
                Ok(diagram) => {
                    out.work.peak_bdd_nodes = out.work.peak_bdd_nodes.max(diagram.node_count());
                    at += 1;
                }
                Err(_) => {
                    out.proof_truncated = true;
                    break;
                }
            }
        }
    }
    for (atom, _) in &fixed {
        if let Some(proof) = assignment_proofs.get(atom) {
            out.proof.extend_from_slice(proof);
        }
    }
    out.proof.sort();
    out.proof.dedup();
    out.basis = if fixed.is_empty() {
        Basis::AdmittedFinitePath
    } else {
        Basis::CheckedExactScalarRestriction
    };
    out.exact = if reduced.is_false() {
        ExactOutcome::RefutedPathUnderModel
    } else {
        ExactOutcome::CompatibleUnderMayModel
    };
    out.verdict = if reduced.is_false() {
        Verdict::RefutedUnderModel
    } else {
        baseline.verdict
    };
    out.restricted_result = Some(result_identity(
        request, context, path, &fixed, &reduced, &out.proof,
    ));
    let rendered = reduced
        .render_terms(limits.render_terms.min(4096))
        .map_err(|e| ModelError::Invalid(format!("native condition rendering refused: {e:?}")))?;
    out.presentation_truncated = rendered.truncated;
    out.rendered = Some(rendered);
    Ok(out)
}
fn boundary(error: DiagramAdmissionError) -> ObligationKind {
    match error {
        DiagramAdmissionError::Boundary(error) => obligation::from_kernel(error),
        DiagramAdmissionError::Resource(_) => ObligationKind::ResourceRefused,
    }
}
fn result_identity(
    request: &ExactRequest<'_>,
    context: &NativeContext,
    path: &NativePath,
    fixed: &[(Id<crate::domain::conditions::EvaluationAtom>, bool)],
    reduced: &Diagram,
    proof: &[crate::domain::derivation::RowRef],
) -> RestrictedResultId {
    let mut key = KeySink::new("native exact request restricted result");
    key.part(b"revision", &MODEL_REVISION.to_le_bytes());
    definition().encode(&mut key);
    request.generation.encode(&mut key);
    request.owner.encode(&mut key);
    request.formal.encode(&mut key);
    context.context.encode(&mut key);
    path.condition.id().encode(&mut key);
    key.part(b"path relation", path.identity.relation().as_bytes());
    key.part(b"path", path.identity.bytes());
    request.value.literal().id().encode(&mut key);
    key.part(
        b"builtin namespace",
        &[request.assumptions.builtin_namespace as u8],
    );
    for (atom, truth) in fixed {
        atom.encode(&mut key);
        truth.encode(&mut key);
    }
    reduced.id().encode(&mut key);
    for row in proof {
        key.part(b"proof relation", row.relation().as_bytes());
        key.part(b"proof row", row.bytes());
    }
    RestrictedResultId(key.finish())
}

// The finite CPython controls deliberately leave coercion, integer membership, user equality,
// mutable truthiness and identity of arbitrary objects unknown. Names never admit builtins.
fn evaluate(atom: &CheckedAtom, value: &Literal, assumptions: Assumptions) -> Option<bool> {
    match &atom.predicate {
        Predicate::IsNone => Some(matches!(value, Literal::None)),
        Predicate::IsValue { .. } => match atom.literals.first()? {
            expected @ (Literal::None | Literal::Bool { .. }) => Some(expected == value),
            _ => None,
        },
        Predicate::Equals { .. } => match (atom.literals.first()?, value) {
            (Literal::String { value: a }, Literal::String { value: b }) => Some(a == b),
            (Literal::Integer { decimal: a }, Literal::Integer { decimal: b }) => Some(a == b),
            _ => None,
        },
        Predicate::MemberOf { .. }
            if atom
                .literals
                .iter()
                .all(|l| matches!(l, Literal::String { .. })) =>
        {
            if matches!(value, Literal::String { .. }) {
                Some(atom.literals.contains(value))
            } else {
                None
            }
        }
        Predicate::Truthy => match value {
            Literal::None => Some(false),
            Literal::Bool { value } => Some(*value),
            Literal::Integer { decimal } => Some(decimal != "0"),
            Literal::String { value } => Some(!value.is_empty()),
            _ => None,
        },
        Predicate::TypeIs { .. }
            if assumptions.builtin_namespace == BuiltinNamespace::StandardCpython =>
        {
            match atom.builtin.as_deref()? {
                "str" => Some(matches!(value, Literal::String { .. })),
                "bool" => Some(matches!(value, Literal::Bool { .. })),
                "int" => Some(matches!(value, Literal::Integer { .. })),
                _ => None,
            }
        }
        _ => None,
    }
}
