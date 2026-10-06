//! Offline, private, finite-first evaluation. No production imports or serving dependencies.
pub mod contracts;
pub mod experiment;
pub mod witness;
use std::collections::{BTreeMap, BTreeSet};
use contracts::*;

const MAX_FINITE_ITEMS: usize = 256;

pub fn observation_digest(observation: &Observation) -> String {
    // A typed observation has no floats/maps with unstable ordering.
    let bytes = serde_json::to_vec(observation).expect("typed observation serializes");
    blake3::hash(&bytes).to_hex().to_string()
}

fn result(case: &Case, applicability: Applicability, epistemic: Epistemic, reason: impl Into<String>) -> Judgment {
    Judgment { task_id: case.task.id.clone(), observation_digest: observation_digest(&case.observation), scorable: applicability == Applicability::Applicable && matches!(epistemic, Epistemic::Sufficient | Epistemic::Insufficient), applicability, epistemic, execution: case.observation.status.clone(), reason: reason.into(), assignments: vec![], countermodels: vec![] }
}

pub fn validate_task(task: &EvaluationTask) -> Result<(), String> {
    if task.id.is_empty() || task.family.is_empty() || task.oracle.supported_domain.is_empty() || task.oracle.revision.is_empty() || task.oracle.input_digest.is_empty() || task.request.question.is_empty() { return Err("missing task/oracle/public request identity or supported domain".into()); }
    if task.predicates.len() > 16 || task.model.as_ref().is_some_and(|m| m.worlds.len() > MAX_FINITE_ITEMS || m.information.len() > MAX_FINITE_ITEMS) { return Err("finite model bound exceeded".into()); }
    let names: BTreeSet<_> = task.predicates.iter().map(|p| p.name.clone()).collect();
    if names.len() != task.predicates.len() { return Err("duplicate predicate".into()); }
    for p in &task.predicates {
        if p.name.is_empty() || p.role.is_empty() || p.accepted_text.is_empty() || p.accepted_text.iter().any(String::is_empty) || p.anchors.is_empty() { return Err("predicate requires independent readable text, anchors, role and status".into()); }
    }
    if task.intent != Intent::NotApplicable && task.witness.is_none() && task.model.is_none() { return Err("positive task has no information question".into()); }
    if let Some(expr) = &task.witness { witness::validate(expr, &names, 0)?; }
    if let Some(model) = &task.model {
        if model.information.iter().any(|info| !names.contains(&info.predicate)) { return Err("world information refers to unknown predicate".into()); }
        let world_names: BTreeSet<_> = model.worlds.iter().map(|w| &w.name).collect();
        if world_names.len() != model.worlds.len() { return Err("duplicate world identity".into()); }
        if model.worlds.iter().any(|w| model.information.iter().any(|info| !w.facts.contains_key(&info.fact))) { return Err("finite world lacks declared information fact".into()); }
    }
    Ok(())
}

fn active_segments(case: &Case) -> Result<BTreeSet<usize>, String> {
    let observation = &case.observation;
    if observation.segments.is_empty() { return Err("missing exact initial bytes".into()); }
    if observation.spans.len() > MAX_FINITE_ITEMS || observation.expansions.len() > MAX_FINITE_ITEMS { return Err("observation finite bound exceeded".into()); }
    let mut active = BTreeSet::from([0]);
    if case.mode == Mode::Expandable {
        if observation.expansions.len() > case.task.envelope.max_calls { return Err("journey call budget exhausted".into()); }
        for call in &observation.expansions {
            if call.realization != observation.realization { return Err("stale expansion realization".into()); }
            if !case.task.request.allowed_followups.contains(&call.operation) { return Err("unsupported public follow-up".into()); }
            let visible = observation.visible_references.iter().any(|reference| {
                reference.reference == call.reference && active.contains(&reference.segment) && observation.segments.get(reference.segment).and_then(|bytes| bytes.get(reference.start..reference.end)).is_some_and(|text| text == reference.reference)
            });
            if !visible { return Err("expansion reference was not readable before the call".into()); }
            if call.status != OperationStatus::Completed { return Err(format!("expansion {:?}", call.status)); }
            let segment = call.response_segment.ok_or("completed expansion has no response bytes")?;
            if segment == 0 || active.contains(&segment) || segment >= observation.segments.len() { return Err("invalid expansion response segment".into()); }
            active.insert(segment);
        }
    }
    let bytes: usize = active.iter().map(|index| observation.segments[*index].len()).sum();
    if bytes > case.task.envelope.max_bytes { return Err("packet byte budget exhausted".into()); }
    for span in &observation.spans {
        let bytes = observation.segments.get(span.segment).ok_or("delivery map references missing bytes")?;
        if bytes.get(span.start..span.end) != Some(span.text.as_str()) || span.text.is_empty() { return Err("delivery map differs from exact emitted bytes".into()); }
    }
    Ok(active)
}

pub fn judge(case: &Case) -> Judgment {
    if let Err(reason) = validate_task(&case.task) { return result(case, Applicability::InvalidTask, Epistemic::Inconclusive, reason); }
    if case.task.intent == Intent::NotApplicable { return result(case, Applicability::NotApplicable, Epistemic::NotApplicable, "explicit no-demand task"); }
    if case.observation.status != OperationStatus::Completed { return result(case, Applicability::Applicable, Epistemic::Inconclusive, case.observation.failure.clone().unwrap_or_else(|| "operation did not complete".into())); }
    match case.task.oracle.completeness {
        Completeness::Unsupported => return result(case, Applicability::Unsupported, Epistemic::Inconclusive, "unsupported oracle domain"),
        Completeness::Incomplete => return result(case, Applicability::Incomplete, Epistemic::Inconclusive, "incomplete oracle inventory"),
        Completeness::Complete => {}
    }
    let active = match active_segments(case) { Ok(active) => active, Err(reason) => {
        let epistemic = if reason.contains("budget") { Epistemic::BudgetInfeasible } else if reason.contains("delivery map") { Epistemic::Insufficient } else { Epistemic::Inconclusive };
        let mut judgment = result(case, Applicability::Applicable, epistemic, &reason);
        if case.mode == Mode::Expandable {
            if reason.contains("budget") { judgment.execution = OperationStatus::BudgetExhausted; }
            else if reason.contains("stale") { judgment.execution = OperationStatus::Stale; }
            else if let Some(call) = case.observation.expansions.iter().find(|call| call.status != OperationStatus::Completed) { judgment.execution = call.status.clone(); }
        }
        return judgment;
    }};
    let mut leaves: BTreeMap<String, Vec<Assignment>> = BTreeMap::new();
    for predicate in &case.task.predicates {
        let matches: Vec<_> = case.observation.spans.iter().filter(|span| active.contains(&span.segment)
            && span.role == predicate.role && predicate.accepted_text.contains(&span.text)
            && predicate.anchors.contains(&span.anchor) && predicate.candidate_status == span.candidate_status
            && predicate.qualifications.iter().all(|q| span.qualifications.contains(q))
            && predicate.context.iter().all(|(k,v)| span.context.get(k) == Some(v))
            && case.task.request.context.iter().all(|(k,v)| span.context.get(k) == Some(v)))
            .map(|span| span.context.clone()).collect();
        leaves.insert(predicate.name.clone(), matches);
    }
    fn proof_bound(expr: &Witness, leaves: &BTreeMap<String, Vec<Assignment>>) -> usize {
        match expr {
            Witness::Leaf { predicate } => leaves.get(predicate).map_or(0, Vec::len),
            Witness::All { children } => children.iter().fold(1_usize, |n, child| n.saturating_mul(proof_bound(child, leaves))).min(4097),
            Witness::Any { children } => children.iter().fold(0_usize, |n, child| n.saturating_add(proof_bound(child, leaves))).min(4097),
            Witness::Exists { child, .. } => proof_bound(child, leaves),
        }
    }
    if case.task.witness.as_ref().is_some_and(|expr| proof_bound(expr, &leaves) > 4096) {
        return result(case, Applicability::Applicable, Epistemic::Inconclusive, "finite witness work bound exceeded");
    }
    let assignments = case.task.witness.as_ref().map(|expr| witness::evaluate(expr, &leaves)).unwrap_or_default();
    if let Some(model) = &case.task.model {
        let worlds: Vec<_> = model.worlds.iter().filter(|world| {
            if !case.task.request.context.iter().all(|(key,value)| world.context.get(key) == Some(value)) { return false; }
            let mut observed: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
            for info in &model.information {
                if leaves.get(&info.predicate).is_some_and(|rows| rows.iter().any(|row| witness::compatible(row, &world.context))) {
                    observed.entry(&info.fact).or_default().insert(&info.value);
                }
            }
            observed.iter().all(|(key, values)| world.facts.get(*key).is_some_and(|value| values.contains(value.as_str())))
        }).collect();
        if worlds.is_empty() { return result(case, Applicability::Applicable, Epistemic::InconsistentModel, "no admissible world agrees; not vacuous sufficiency"); }
        let first = worlds[0];
        if let Some(other) = worlds.iter().find(|world| world.answer != first.answer) {
            let mut judgment = result(case, Applicability::Applicable, Epistemic::Insufficient, "matching worlds answer differently; readable distinction missing");
            judgment.countermodels = vec![first.name.clone(), other.name.clone()]; return judgment;
        }
    }
    if case.task.witness.is_some() && assignments.is_empty() {
        let missing: Vec<_> = leaves.iter().filter(|(_, rows)| rows.is_empty()).map(|(name, _)| name.as_str()).collect();
        let reason = if missing.is_empty() { "individually readable predicates have incompatible contexts".into() } else { format!("missing readable predicates: {}", missing.join(", ")) };
        return result(case, Applicability::Applicable, Epistemic::Insufficient, reason);
    }
    if case.task.intent == Intent::Unknown { return result(case, Applicability::Applicable, Epistemic::ModelRelativeUnknown, "task intentionally asks for a qualified unknown; no numeric success claim"); }
    let mut judgment = result(case, Applicability::Applicable, Epistemic::Sufficient, "independent witness / declared complete finite class is determinate");
    judgment.assignments = assignments; judgment
}

#[cfg(test)]
mod tests;
