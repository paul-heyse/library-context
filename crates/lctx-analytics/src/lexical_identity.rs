//! Immutable lexical parameter identity, independent of provider reaching and completion.
use cpg_schema::codebook::{BindingKind, DeclarationKind, LexicalScopeKind, SyntaxKind};
use cpg_schema::id::Id;
use cpg_schema::tables::*;
use std::collections::{HashMap, HashSet};

fn index<T>(rows: &[T], key: impl Fn(&T) -> (Id, Id)) -> HashMap<(Id, Id), Vec<&T>> {
    let mut out = HashMap::<_, Vec<_>>::new();
    for row in rows {
        out.entry(key(row)).or_default().push(row);
    }
    out
}
fn one<'a, T>(rows: &HashMap<(Id, Id), Vec<&'a T>>, key: (Id, Id)) -> Option<&'a T> {
    match rows.get(&key)?.as_slice() {
        [row] => Some(*row),
        _ => None,
    }
}
pub(crate) struct Read<'a> {
    pub reference: &'a ReferencesRow,
    pub resolution: &'a ReferenceResolutionsRow,
    pub binding: &'a BindingsRow,
    pub parameter: &'a ParameterSyntaxRow,
    pub scope: &'a ScopesRow,
}
pub(crate) struct ParameterReads<'a> {
    declarations: HashMap<(Id, Id), Vec<&'a DeclarationsRow>>,
    parameters: HashMap<(Id, Id), Vec<&'a ParameterSyntaxRow>>,
    bindings: HashMap<(Id, Id), Vec<&'a BindingsRow>>,
    scopes: HashMap<(Id, Id), Vec<&'a ScopesRow>>,
    references: HashMap<(Id, Id), Vec<&'a ReferencesRow>>,
    resolutions: HashMap<(Id, Id), Vec<&'a ReferenceResolutionsRow>>,
    names: HashMap<(Id, Id, &'a str), usize>,
    hazards: HashSet<(Id, Id)>,
}
impl<'a> ParameterReads<'a> {
    pub fn new(
        declarations: &'a [DeclarationsRow],
        parameters: &'a [ParameterSyntaxRow],
        syntax: &[SyntaxNodesRow],
        bindings: &'a [BindingsRow],
        scopes: &'a [ScopesRow],
        references: &'a [ReferencesRow],
        resolutions: &'a [ReferenceResolutionsRow],
    ) -> Self {
        let declarations = index(declarations, |r| (r.snapshot_id, r.node_id));
        let mut names = HashMap::new();
        for b in bindings {
            *names
                .entry((b.snapshot_id, b.scope_id, b.name.as_str()))
                .or_insert(0) += 1;
        }
        let mut hazards = HashSet::new();
        for n in syntax.iter().filter(|n| {
            matches!(
                n.kind,
                SyntaxKind::StmtDelete
                    | SyntaxKind::StmtNonlocal
                    | SyntaxKind::ExprYield
                    | SyntaxKind::ExprYieldFrom
            )
        }) {
            let mut owner = n.owner_node_id;
            let mut seen = HashSet::new();
            while let Some(id) = owner {
                if !seen.insert(id) {
                    break;
                }
                hazards.insert((n.snapshot_id, id));
                owner = one(&declarations, (n.snapshot_id, id)).and_then(|d| d.parent_node_id);
            }
        }
        Self {
            declarations,
            parameters: index(parameters, |r| (r.snapshot_id, r.node_id)),
            bindings: index(bindings, |r| (r.snapshot_id, r.node_id)),
            scopes: index(scopes, |r| (r.snapshot_id, r.node_id)),
            references: index(references, |r| (r.snapshot_id, r.name_node_id)),
            resolutions: index(resolutions, |r| (r.snapshot_id, r.reference_id)),
            names,
            hazards,
        }
    }
    pub fn prove(
        &self,
        expression: &SyntaxNodesRow,
        function: Id,
        parameter: Id,
    ) -> Option<Read<'a>> {
        let snapshot = expression.snapshot_id;
        if expression.kind != SyntaxKind::ExprName
            || expression.owner_node_id != Some(function)
            || self.hazards.contains(&(snapshot, function))
        {
            return None;
        }
        let d = one(&self.declarations, (snapshot, function))?;
        let formal = one(&self.parameters, (snapshot, parameter))?;
        if d.kind != DeclarationKind::Function
            || !d.decorators.is_empty()
            || formal.function_node_id != function
            || expression.module_node_id != d.module_node_id
        {
            return None;
        }
        let reference = one(&self.references, (snapshot, expression.node_id))?;
        let resolution = one(&self.resolutions, (snapshot, reference.node_id))?;
        if resolution.captured || resolution.reason.is_some() || resolution.builtin_name.is_some() {
            return None;
        }
        let binding = one(&self.bindings, (snapshot, resolution.binding_id?))?;
        let scope = one(&self.scopes, (snapshot, binding.scope_id))?;
        if binding.kind != BindingKind::Parameter
            || binding.site_node_id != parameter
            || binding.name != formal.name
            || reference.name != formal.name
            || expression.detail.as_deref() != Some(&formal.name)
            || reference.scope_id != binding.scope_id
            || scope.owner_node_id != function
            || scope.kind != LexicalScopeKind::Function
            || scope.module_node_id != d.module_node_id
            || binding.module_node_id != d.module_node_id
            || reference.module_node_id != d.module_node_id
            || reference.start_byte != expression.start_byte
            || reference.end_byte != expression.end_byte
            || self
                .names
                .get(&(snapshot, binding.scope_id, binding.name.as_str()))
                != Some(&1)
        {
            return None;
        }
        Some(Read {
            reference,
            resolution,
            binding,
            parameter: formal,
            scope,
        })
    }
}
