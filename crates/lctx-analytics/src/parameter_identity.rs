//! Prove a bare return read from immutable lexical source facts. Provider reaching rows
//! nominate an origin but do not prove it here. No store, SQL or Python execution is involved.

use std::collections::{BTreeMap, HashMap};
use cpg_schema::behavior::{ExitSitesRow, ValueFlowContributionsRow};
use cpg_schema::codebook::{DeclarationKind, ExitSiteKind, FlowSink,
    SyntaxField, SyntaxKind};
use cpg_schema::id::Id;
use cpg_schema::parameter_identity::{SourceParameterIdentitiesRow, identity};
use cpg_schema::tables::{BindingsRow, DeclarationsRow, FlowValuesRow, ParameterSyntaxRow,
    ReferencesRow, ReferenceResolutionsRow, ScopesRow, SyntaxNodesRow};

pub struct Inputs<'a> {
    pub declarations: &'a [DeclarationsRow],
    pub parameters: &'a [ParameterSyntaxRow],
    pub syntax: &'a [SyntaxNodesRow],
    pub bindings: &'a [BindingsRow],
    pub scopes: &'a [ScopesRow],
    pub references: &'a [ReferencesRow],
    pub resolutions: &'a [ReferenceResolutionsRow],
    pub values: &'a [FlowValuesRow],
    pub contributions: &'a [ValueFlowContributionsRow],
    pub exits: &'a [ExitSitesRow],
}

fn index<T>(rows: &[T], key: impl Fn(&T) -> (Id, Id)) -> HashMap<(Id, Id), Vec<&T>> {
    let mut out = HashMap::<_, Vec<_>>::new();
    for row in rows { out.entry(key(row)).or_default().push(row); }
    out
}
fn one<'a, T>(rows: &'a HashMap<(Id, Id), Vec<&'a T>>, key: (Id, Id)) -> Option<&'a T> {
    match rows.get(&key)?.as_slice() { [row] => Some(*row), _ => None }
}

pub fn prove(input: Inputs<'_>) -> Vec<SourceParameterIdentitiesRow> {
    let declarations = index(input.declarations, |r| (r.snapshot_id, r.node_id));
    let parameters = index(input.parameters, |r| (r.snapshot_id, r.node_id));
    let syntax = index(input.syntax, |r| (r.snapshot_id, r.node_id));
    let reads=crate::lexical_identity::ParameterReads::new(input.declarations,input.parameters,input.syntax,
        input.bindings,input.scopes,input.references,input.resolutions);
    let values = index(input.values, |r| (r.snapshot_id, r.fact_id));
    let exits = index(input.exits, |r| (r.snapshot_id, r.site_node_id));
    let mut names_at = HashMap::<_, Vec<_>>::new();
    for node in input.syntax.iter().filter(|n| n.kind == SyntaxKind::ExprName) {
        names_at.entry((node.snapshot_id, node.module_node_id, node.start_byte, node.end_byte))
            .or_default().push(node);
    }
    let mut out = BTreeMap::new();
    for contribution in input.contributions {
        let snapshot = contribution.snapshot_id;
        let function = contribution.function_node_id;
        if contribution.sink_function_node_id != Some(function) || !contribution.identity
            || !contribution.upstream_identity || contribution.through_call
            || contribution.local_through_call || contribution.upstream_through_call
            || contribution.captured { continue; }
        let Some(parameter) = contribution.parameter_node_id else { continue };
        let Some(declaration) = one(&declarations, (snapshot, function)) else { continue };
        if declaration.kind != DeclarationKind::Function || !declaration.decorators.is_empty() { continue; }
        let Some(formal) = one(&parameters, (snapshot, parameter)) else { continue };
        if formal.function_node_id != function { continue; }
        let Some(value) = one(&values, (snapshot, contribution.flow_value_fact_id)) else { continue };
        if value.sink != FlowSink::Return || !value.identity || value.through_call || value.approximated
            || value.use_id != contribution.use_id || value.module_node_id != declaration.module_node_id { continue; }
        let Some([expression]) = names_at.get(&(snapshot, value.module_node_id,
            value.sink_start_byte, value.sink_end_byte)).map(Vec::as_slice) else { continue };
        if expression.owner_node_id != Some(function) || expression.field != SyntaxField::Value { continue; }
        let Some(return_node) = one(&syntax, (snapshot, expression.parent_node_id)) else { continue };
        if return_node.kind != SyntaxKind::StmtReturn || return_node.owner_node_id != Some(function) { continue; }
        let Some(exit) = one(&exits, (snapshot, return_node.node_id)) else { continue };
        if exit.kind != ExitSiteKind::Return || exit.function_node_id != function { continue; }
        let Some(read)=reads.prove(expression,function,parameter) else {continue;};
        let mut row = SourceParameterIdentitiesRow {
            snapshot_id: snapshot, identity_id: Id::ZERO, function_node_id: function,
            parameter_node_id: parameter, source_flow_fact_id: value.fact_id,
            source_origin_id: contribution.origin_id, condition_id: contribution.condition_id,
            return_site_fact_id: exit.source_fact_id, expression_fact_id: expression.fact_id,
            reference_fact_id: read.reference.fact_id, resolution_fact_id: read.resolution.fact_id,
            binding_fact_id: read.binding.fact_id, parameter_fact_id: formal.fact_id,
            module_node_id: value.module_node_id, start_byte: expression.start_byte, end_byte: expression.end_byte,
        };
        row.identity_id = identity(&row);
        out.insert(row.identity_id, row);
    }
    out.into_values().collect()
}
