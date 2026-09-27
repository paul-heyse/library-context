//! Source reconstruction of a direct single-model return's lexical value basis.
use crate::summaries::finite::{ModeledSummaryFlowSeed, modeled_return_proof};
use cpg_schema::behavior::{ExitSitesRow, ModeledArgumentEvaluationsRow};
use cpg_schema::codebook::{ExitSiteKind, FlowSink, SyntaxField, SyntaxKind};
use cpg_schema::completion_proof::proof_digest;
use cpg_schema::id::Id;
use cpg_schema::modeled_identity::{SourceModeledIdentitiesRow, identity};
use cpg_schema::tables::*;
use std::collections::{BTreeMap, HashMap};

pub struct Inputs<'a> {
    pub declarations: &'a [DeclarationsRow],
    pub parameters: &'a [ParameterSyntaxRow],
    pub syntax: &'a [SyntaxNodesRow],
    pub bindings: &'a [BindingsRow],
    pub scopes: &'a [ScopesRow],
    pub references: &'a [ReferencesRow],
    pub resolutions: &'a [ReferenceResolutionsRow],
    pub values: &'a [FlowValuesRow],
    pub exits: &'a [ExitSitesRow],
    pub calls: &'a [CallSyntaxRow],
    pub arguments: &'a [ArgumentsRow],
    pub seeds: &'a [ModeledSummaryFlowSeed],
    pub evaluations: &'a [ModeledArgumentEvaluationsRow],
}
fn one<T>(mut rows: impl Iterator<Item = T>) -> Option<T> {
    let row = rows.next()?;
    rows.next().is_none().then_some(row)
}
pub fn prove(input: Inputs<'_>) -> Vec<SourceModeledIdentitiesRow> {
    let reads = crate::lexical_identity::ParameterReads::new(
        input.declarations,
        input.parameters,
        input.syntax,
        input.bindings,
        input.scopes,
        input.references,
        input.resolutions,
    );
    let mut evaluations = HashMap::<_, Vec<_>>::new();
    for e in input.evaluations {
        evaluations
            .entry((
                e.candidate_flow_fact_id,
                e.parameter_node_id,
                e.pysa_fact_id,
                e.model_id,
                e.rule_id,
            ))
            .or_default()
            .push(e.clone());
    }
    let mut out = BTreeMap::new();
    for s in input.seeds {
        let snapshot = s.snapshot_id;
        let function = s.function_node_id;
        let Some(call) = one(input.calls.iter().filter(|c| {
            c.snapshot_id == snapshot
                && c.fact_id == s.call_fact_id
                && c.owner_node_id == Some(function)
                && !c.in_annotation
        })) else {
            continue;
        };
        let Some(call_expr) = one(input.syntax.iter().filter(|n| {
            n.snapshot_id == snapshot
                && n.node_id == call.node_id
                && n.kind == SyntaxKind::ExprCall
                && n.owner_node_id == Some(function)
                && n.field == SyntaxField::Value
        })) else {
            continue;
        };
        let Some(ret) = one(input.syntax.iter().filter(|n| {
            n.snapshot_id == snapshot
                && n.node_id == call_expr.parent_node_id
                && n.kind == SyntaxKind::StmtReturn
                && n.owner_node_id == Some(function)
        })) else {
            continue;
        };
        if one(input.exits.iter().filter(|e| {
            e.snapshot_id == snapshot
                && e.source_fact_id == s.return_site_fact_id
                && e.site_node_id == ret.node_id
                && e.function_node_id == function
                && e.kind == ExitSiteKind::Return
        }))
        .is_none()
        {
            continue;
        }
        let Some(value) = one(input.values.iter().filter(|v| {
            v.snapshot_id == snapshot
                && v.fact_id == s.source_flow_fact_id
                && v.sink == FlowSink::Return
                && v.module_node_id == call.module_node_id
                && v.sink_start_byte == call.start_byte
                && v.sink_end_byte == call.end_byte
        })) else {
            continue;
        };
        let Some(argument) = one(input.arguments.iter().filter(|a| {
            a.snapshot_id == snapshot
                && a.fact_id == s.source_argument_fact_id
                && a.call_node_id == call.node_id
        })) else {
            continue;
        };
        let Some(expression) = one(input.syntax.iter().filter(|n| {
            n.snapshot_id == snapshot
                && n.module_node_id == call.module_node_id
                && n.start_byte == argument.value_start_byte
                && n.end_byte == argument.value_end_byte
                && n.kind == SyntaxKind::ExprName
        })) else {
            continue;
        };
        let Some(read) = reads.prove(expression, function, s.parameter_node_id) else {
            continue;
        };
        // The seed adapter ties the raw use to this model-selected argument. Independently
        // require its lexical expression to be a direct child of the actual call argument.
        if one(input.syntax.iter().filter(|n| {
            n.snapshot_id == snapshot
                && n.node_id == expression.parent_node_id
                && (n.node_id == call.node_id
                    || (n.parent_node_id == call.node_id && n.kind == SyntaxKind::Keyword))
        }))
        .is_none()
        {
            continue;
        }
        let Some(proof) = modeled_return_proof(s, &evaluations) else {
            continue;
        };
        if proof.len() > cpg_schema::summary_contract::MAX_SUMMARY_PROOF_STEPS {
            continue;
        }
        let mut row = SourceModeledIdentitiesRow {
            snapshot_id: snapshot,
            identity_id: Id::ZERO,
            function_node_id: function,
            parameter_node_id: s.parameter_node_id,
            source_flow_fact_id: value.fact_id,
            source_origin_id: s.source_origin_id,
            condition_id: s.condition_id,
            return_site_fact_id: s.return_site_fact_id,
            call_fact_id: call.fact_id,
            call_expression_fact_id: call_expr.fact_id,
            source_argument_fact_id: argument.fact_id,
            pysa_fact_id: s.pysa_fact_id,
            model_id: s.model_id,
            rule_id: s.rule_id,
            callee_resolution_fact_id: s.callee_resolution_fact_id,
            expression_fact_id: expression.fact_id,
            reference_fact_id: read.reference.fact_id,
            resolution_fact_id: read.resolution.fact_id,
            binding_fact_id: read.binding.fact_id,
            parameter_fact_id: read.parameter.fact_id,
            scope_fact_id: read.scope.fact_id,
            module_node_id: call.module_node_id,
            start_byte: expression.start_byte,
            end_byte: expression.end_byte,
            model_proof_count: proof.len() as i64,
            model_proof_digest: proof_digest(&proof),
        };
        row.identity_id = identity(&row);
        out.insert(row.identity_id, row);
    }
    out.into_values().collect()
}
