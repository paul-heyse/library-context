//! Nonrecursive source-call adapter over the base completion stage. No persisted result is an
//! input, and no model identifier is fabricated for a source callable.
use crate::{call_binding, completion};
use cpg_schema::codebook::{
    BoundaryReason as R, CompletionKind as C, InvocationPhase, PysaCalleeKind, PysaSiteKind,
    PysaTargetKind, ResolutionStatus, SyntaxField as F, SyntaxKind as S,
};
use cpg_schema::derived::{CallTargetsRow, ProviderNodeMapRow, ResolutionsRow};
use cpg_schema::id::Id;
use cpg_schema::source_call::{
    SourceCallBindingsRow, SourceCallHeaderStepsRow, SourceCallNormalsRow,
};
use cpg_schema::tables::{CallSyntaxRow, ParameterSemanticsRow, PysaCallsRow, PysaFunctionsRow};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Targets<'a> {
    pub arguments: &'a [cpg_schema::tables::ArgumentsRow],
    pub calls: &'a [CallSyntaxRow],
    pub targets: &'a [CallTargetsRow],
    pub resolutions: &'a [ResolutionsRow],
    pub providers: &'a [PysaCallsRow],
    pub functions: &'a [PysaFunctionsRow],
    pub node_map: &'a [ProviderNodeMapRow],
    pub parameters: &'a [ParameterSemanticsRow],
}
#[derive(Default)]
pub struct Outcome {
    pub bindings: Vec<SourceCallBindingsRow>,
    pub invocations: Vec<InvocationCandidate>,
    pub normals: Vec<SourceCallNormalsRow>,
    pub headers: Vec<SourceCallHeaderStepsRow>,
    pub refusals: Vec<(Id, Id, R)>,
}
/// Source target observations and the independent binding assessment. The evaluator supplies
/// invocation proof; completion supplies reached-prefix proof. Neither depends on body Normal.
#[derive(Clone, Copy)]
pub struct InvocationCandidate {
    pub snapshot_id: Id,
    pub function_node_id: Id,
    pub call_node_id: Id,
    pub call_fact_id: Id,
    pub syntax_fact_id: Id,
    pub target_node_id: Id,
    pub pysa_fact_id: Id,
    pub argument_count: i64,
    pub binding_id: Option<Id>,
    pub reason: Option<R>,
}
fn grouped<'a, T, K: std::hash::Hash + Eq>(
    rows: &'a [T],
    key: impl Fn(&'a T) -> K,
) -> HashMap<K, Vec<&'a T>> {
    let mut out = HashMap::new();
    for row in rows {
        out.entry(key(row)).or_insert_with(Vec::new).push(row);
    }
    out
}
fn one<'a, T>(rows: Option<&Vec<&'a T>>) -> Result<&'a T, R> {
    match rows.map(Vec::as_slice) {
        Some([r]) => Ok(*r),
        _ => Err(R::MissingEvidence),
    }
}
pub fn prepare(
    source: Targets<'_>,
    inputs: &completion::Inputs<'_>,
    base: &completion::Outcome,
) -> Outcome {
    let bindings = call_binding::Inputs {
        requests: &[],
        mappings: &[],
        syntax: inputs.syntax,
        declarations: inputs.declarations,
        parameters: inputs.parameters,
        arguments: &[],
        bindings: inputs.bindings,
        references: inputs.references,
        resolutions: inputs.resolutions,
        statements: &base.statements,
        statement_steps: &base.statement_steps,
        expressions: inputs.expressions,
    };
    let fresh = call_binding::FreshDefinitionIndex::new(&bindings);
    let targets = grouped(source.targets, |r| (r.snapshot_id, r.call_site_node_id));
    let resolutions = grouped(source.resolutions, |r| (r.snapshot_id, r.call_site_node_id));
    let providers = grouped(source.providers, |r| (r.snapshot_id, r.fact_id));
    let maps = grouped(source.node_map, |r| (r.snapshot_id, r.node_id));
    let functions = grouped(source.functions, |r| (r.snapshot_id, r.fact_id));
    let signatures = grouped(source.parameters, |r| {
        (r.snapshot_id, r.module_node_id, r.function_key.as_str())
    });
    let arguments = grouped(source.arguments, |r| (r.snapshot_id, r.call_node_id));
    let parameters = grouped(inputs.parameters, |r| (r.snapshot_id, r.function_node_id));
    let declarations = grouped(inputs.declarations, |r| (r.snapshot_id, r.node_id));
    let bodies = grouped(&base.bodies, |r| (r.snapshot_id, r.function_node_id));
    let body_steps = grouped(&base.body_steps, |r| (r.snapshot_id, r.body_id));
    let releases = grouped(&base.body_releases, |r| (r.snapshot_id, r.body_id));
    let children = grouped(inputs.syntax, |r| (r.snapshot_id, r.parent_node_id));
    let deferred: std::collections::HashSet<_> = inputs
        .syntax
        .iter()
        .filter(|n| matches!(n.kind, S::ExprYield | S::ExprYieldFrom))
        .filter_map(|n| n.owner_node_id.map(|owner| (n.snapshot_id, owner)))
        .collect();
    let nodes = grouped(inputs.syntax, |r| (r.snapshot_id, r.node_id));
    let mut out = Outcome::default();
    for call in source.calls {
        let Some(caller) = call.owner_node_id else {
            continue;
        };
        let key = (call.snapshot_id, call.node_id);
        let Ok(syntax) = one(nodes.get(&key)) else {
            continue;
        };
        // Only local direct targets enter this adapter. Other forms keep their evaluator's
        // existing refusal; candidates are never relabelled as exact source calls.
        let Ok(target) = one(targets.get(&key)) else {
            continue;
        };
        let Some(callee) = target.target_node_id else {
            continue;
        };
        if !declarations.contains_key(&(call.snapshot_id, callee)) {
            continue;
        }
        let result = (|| {
            if deferred.contains(&(call.snapshot_id, callee)) {
                return Err(R::ScopeBoundary);
            }
            let resolution = one(resolutions.get(&key))?;
            let provider = one(providers.get(&(call.snapshot_id, target.pysa_fact_id)))?;
            if resolution.status != ResolutionStatus::Resolved
                || resolution.call_fact_id != call.fact_id
                || resolution.target_count != 1
                || resolution.has_unresolved_remainder
                || !resolution.candidate_set_complete_under_model
                || resolution.reason.is_some()
                || resolution.unresolved_reason.is_some()
                || target.reason.is_some()
                || target.argument_node_id.is_some()
                || provider.phase != InvocationPhase::Call
                || provider.site_kind != PysaSiteKind::Regular
                || provider.callee_kind != PysaCalleeKind::Call
                || provider.target_kind != PysaTargetKind::Function
                || provider.unresolved_reason.is_some()
                || provider.higher_order_index.is_some()
                || provider.implicit_dunder_call != Some(false)
                || provider.module_node_id != call.module_node_id
                || provider.start_byte != call.start_byte
                || provider.end_byte != call.end_byte
                || call.in_annotation
            {
                return Err(R::MissingEvidence);
            }
            if call.positional_count != 0
                || call.keyword_count != 0
                || arguments.contains_key(&key)
                || parameters.contains_key(&(call.snapshot_id, callee))
            {
                return Err(R::ScopeBoundary);
            }
            let map = one(maps.get(&(call.snapshot_id, Some(callee))))?;
            let function = one(functions.get(&(call.snapshot_id, map.pysa_fact_id)))?;
            if map.reason.is_some()
                || function.signature_count != 1
                || function.is_overload
                || function.is_stub
                || !function.is_def_statement
                || function.is_classmethod
                || function.is_staticmethod
                || function.is_property_getter
                || function.is_property_setter
                || signatures.contains_key(&(
                    call.snapshot_id,
                    function.module_node_id,
                    function.function_key.as_str(),
                ))
            {
                return Err(R::ScopeBoundary);
            }
            let fresh = fresh
                .admit(call_binding::Request {
                    snapshot: call.snapshot_id,
                    caller,
                    call_node: call.node_id,
                    call_fact: call.fact_id,
                    callee,
                })
                .map_err(|r| match r {
                    R::DefaultStabilityUnknown | R::DefaultUnavailable => R::ScopeBoundary,
                    other => other,
                })?;
            let operands = children.get(&key).map_or(&[][..], Vec::as_slice);
            if !matches!(operands,[n] if n.kind==S::ExprName && n.field==F::Callee
                && n.node_id==fresh.reference.name_node_id && n.owner_node_id==Some(caller))
            {
                return Err(R::MissingEvidence);
            }
            if map.declaration_fact_id != Some(fresh.declaration.fact_id) {
                return Err(R::MissingEvidence);
            }
            let mut header: Vec<_> = fresh
                .header_steps
                .iter()
                .enumerate()
                .map(|(i, s)| SourceCallHeaderStepsRow {
                    snapshot_id: call.snapshot_id,
                    binding_id: Id::ZERO,
                    ordinal: i as i64,
                    kind: s.kind,
                    evidence_id: s.evidence_id,
                })
                .collect();
            let mut row = SourceCallBindingsRow {
                snapshot_id: call.snapshot_id,
                binding_id: Id::ZERO,
                function_node_id: caller,
                call_node_id: call.node_id,
                call_fact_id: call.fact_id,
                syntax_fact_id: fresh.call.fact_id,
                callee_node_id: callee,
                pysa_fact_id: provider.fact_id,
                signature_fact_id: function.fact_id,
                declaration_fact_id: fresh.declaration.fact_id,
                header_fact_id: fresh.header.fact_id,
                statement_fact_id: fresh.statement.fact_id,
                binding_fact_id: fresh.binding.fact_id,
                reference_fact_id: fresh.reference.fact_id,
                resolution_fact_id: fresh.resolution.fact_id,
                header_count: header.len() as i64,
                header_digest: cpg_schema::source_call::header_digest(&header),
            };
            row.binding_id = cpg_schema::source_call::binding_identity(&row);
            for s in &mut header {
                s.binding_id = row.binding_id;
            }
            cpg_schema::source_call::admit_binding(&row, &header).map_err(|e| e.reason)?;
            Ok((row, header))
        })();
        let candidate = InvocationCandidate {
            snapshot_id: call.snapshot_id,
            function_node_id: caller,
            call_node_id: call.node_id,
            call_fact_id: call.fact_id,
            syntax_fact_id: syntax.fact_id,
            target_node_id: callee,
            pysa_fact_id: target.pysa_fact_id,
            argument_count: call.positional_count + call.keyword_count,
            binding_id: result.as_ref().ok().map(|(r, _)| r.binding_id),
            reason: result.as_ref().err().copied(),
        };
        out.invocations.push(candidate);
        match result {
            Ok((binding, header)) => {
                let normal = (|| {
                    let body = one(bodies.get(&(call.snapshot_id, callee)))?;
                    if let Some(reason) = body.reason.or(body.release_reason) {
                        return Err(reason);
                    }
                    if !matches!(body.kind, C::Normal | C::Return) {
                        return Err(R::UnsupportedControlFlow);
                    }
                    let mut steps: Vec<_> = body_steps
                        .get(&(call.snapshot_id, body.body_id))
                        .into_iter()
                        .flatten()
                        .map(|s| (**s).clone())
                        .collect();
                    let mut releases: Vec<_> = releases
                        .get(&(call.snapshot_id, body.body_id))
                        .into_iter()
                        .flatten()
                        .map(|s| (**s).clone())
                        .collect();
                    steps.sort_by_key(|s| s.ordinal);
                    releases.sort_by_key(|s| s.ordinal);
                    if header.len() + steps.len() + 9 > 64 {
                        return Err(R::SummaryProofLimit);
                    }
                    let mut row = SourceCallNormalsRow {
                        snapshot_id: call.snapshot_id,
                        certificate_id: Id::ZERO,
                        binding_id: binding.binding_id,
                        body_id: body.body_id,
                        body_count: body.step_count,
                        body_kind: body.kind,
                    };
                    row.certificate_id = cpg_schema::source_call::identity(&row);
                    cpg_schema::source_call::admit(
                        &row, &binding, &header, body, &steps, &releases,
                    )
                    .map_err(|e| e.reason)?;
                    Ok(row)
                })();
                match normal {
                    Ok(row) => out.normals.push(row),
                    Err(reason) => out.refusals.push((call.snapshot_id, call.node_id, reason)),
                }
                out.bindings.push(binding);
                out.headers.extend(header);
            }
            Err(reason) => out.refusals.push((call.snapshot_id, call.node_id, reason)),
        }
    }
    out.bindings.sort_by_key(|r| (r.snapshot_id, r.binding_id));
    out.invocations
        .sort_by_key(|r| (r.snapshot_id, r.call_node_id));
    out.normals
        .sort_by_key(|r| (r.snapshot_id, r.certificate_id));
    out.headers
        .sort_by_key(|r| (r.snapshot_id, r.binding_id, r.ordinal));
    out.refusals.sort();
    out
}
