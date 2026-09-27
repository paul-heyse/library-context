//! Prove a bare return read from immutable lexical source facts. Provider reaching rows
//! nominate an origin but do not prove it here. No store, SQL or Python execution is involved.

use std::collections::{BTreeMap, HashMap, HashSet};
use cpg_schema::behavior::{ExitSitesRow, ValueFlowContributionsRow};
use cpg_schema::codebook::{BindingKind, DeclarationKind, ExitSiteKind, FlowSink,
    LexicalScopeKind, SyntaxField, SyntaxKind};
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
    let bindings = index(input.bindings, |r| (r.snapshot_id, r.node_id));
    let scopes = index(input.scopes, |r| (r.snapshot_id, r.node_id));
    let references = index(input.references, |r| (r.snapshot_id, r.name_node_id));
    let resolutions = index(input.resolutions, |r| (r.snapshot_id, r.reference_id));
    let values = index(input.values, |r| (r.snapshot_id, r.fact_id));
    let exits = index(input.exits, |r| (r.snapshot_id, r.site_node_id));
    let mut names = HashMap::<_, Vec<_>>::new();
    for binding in input.bindings {
        names.entry((binding.snapshot_id, binding.scope_id, binding.name.as_str()))
            .or_default().push(binding);
    }
    let mut names_at = HashMap::<_, Vec<_>>::new();
    for node in input.syntax.iter().filter(|n| n.kind == SyntaxKind::ExprName) {
        names_at.entry((node.snapshot_id, node.module_node_id, node.start_byte, node.end_byte))
            .or_default().push(node);
    }
    // A deletion or nested nonlocal declaration can invalidate lexical initialization.
    // Reject the containing declaration chain; unrelated functions remain independent.
    let mut hazards = HashSet::new();
    for node in input.syntax.iter().filter(|n| matches!(n.kind,
        SyntaxKind::StmtDelete | SyntaxKind::StmtNonlocal | SyntaxKind::ExprYield | SyntaxKind::ExprYieldFrom)) {
        let mut owner = node.owner_node_id;
        let mut visited = HashSet::new();
        while let Some(id) = owner {
            if !visited.insert(id) { break; }
            hazards.insert((node.snapshot_id, id));
            owner = one(&declarations, (node.snapshot_id, id)).and_then(|d| d.parent_node_id);
        }
    }
    let mut out = BTreeMap::new();
    for contribution in input.contributions {
        let snapshot = contribution.snapshot_id;
        let function = contribution.function_node_id;
        if contribution.sink_function_node_id != Some(function) || !contribution.identity
            || !contribution.upstream_identity || contribution.through_call
            || contribution.local_through_call || contribution.upstream_through_call
            || contribution.captured || hazards.contains(&(snapshot, function)) { continue; }
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
        let Some(reference) = one(&references, (snapshot, expression.node_id)) else { continue };
        let Some(resolution) = one(&resolutions, (snapshot, reference.node_id)) else { continue };
        if resolution.captured || resolution.reason.is_some() || resolution.builtin_name.is_some() { continue; }
        let Some(binding) = resolution.binding_id.and_then(|id| one(&bindings, (snapshot, id))) else { continue };
        let Some(scope) = one(&scopes, (snapshot, binding.scope_id)) else { continue };
        if binding.kind != BindingKind::Parameter || binding.site_node_id != parameter
            || binding.name != formal.name || reference.name != formal.name
            || reference.scope_id != binding.scope_id || scope.owner_node_id != function
            || scope.module_node_id != value.module_node_id
            || scope.kind != LexicalScopeKind::Function || binding.module_node_id != value.module_node_id
            || reference.module_node_id != value.module_node_id
            || reference.start_byte != expression.start_byte || reference.end_byte != expression.end_byte
            || names.get(&(snapshot, binding.scope_id, binding.name.as_str())).map(Vec::len) != Some(1) { continue; }
        let mut row = SourceParameterIdentitiesRow {
            snapshot_id: snapshot, identity_id: Id::ZERO, function_node_id: function,
            parameter_node_id: parameter, source_flow_fact_id: value.fact_id,
            source_origin_id: contribution.origin_id, condition_id: contribution.condition_id,
            return_site_fact_id: exit.source_fact_id, expression_fact_id: expression.fact_id,
            reference_fact_id: reference.fact_id, resolution_fact_id: resolution.fact_id,
            binding_fact_id: binding.fact_id, parameter_fact_id: formal.fact_id,
            module_node_id: value.module_node_id, start_byte: expression.start_byte, end_byte: expression.end_byte,
        };
        row.identity_id = identity(&row);
        out.insert(row.identity_id, row);
    }
    out.into_values().collect()
}
