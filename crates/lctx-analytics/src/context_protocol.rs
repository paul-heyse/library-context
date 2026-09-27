//! Source admission for authored synchronous class protocols. Store acquisition and runtime
//! completion are separate owners; this module neither executes source nor assumes arguments
//! complete. A missing/ambiguous premise produces no admitted site.
use cpg_schema::codebook::{
    ArgumentKind, BindingKind, ContextEntryKind, ContextExitKind, DeclarationKind, DefinitionKind,
    ExportSyntaxKind, InvocationPhase, LexicalScopeKind, ModuleOrigin, ParameterKind,
    PysaCalleeKind, PysaSiteKind, PysaTargetKind, SyntaxField as F, SyntaxKind as S,
};
use cpg_schema::context_protocol::{
    ModelContextProtocolsRow, SourceContextArgumentsRow, SourceContextSitesRow, binding_id,
    initializer_signatures, site_id,
};
use cpg_schema::id::Id;
use cpg_schema::summary_contract::{BoundArgument, bind_arguments};
use cpg_schema::tables::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct Inputs<'a> {
    pub protocols: &'a [ModelContextProtocolsRow],
    pub syntax: &'a [SyntaxNodesRow],
    pub calls: &'a [CallSyntaxRow],
    pub provider_calls: &'a [PysaCallsRow],
    pub arguments: &'a [ArgumentsRow],
    pub declarations: &'a [DeclarationsRow],
    pub definitions: &'a [ContextDefinitionsRow],
    pub parameters: &'a [ContextParametersRow],
    pub modules: &'a [ContextModulesRow],
    pub bindings: &'a [BindingsRow],
    pub references: &'a [ReferencesRow],
    pub resolutions: &'a [ReferenceResolutionsRow],
    pub scopes: &'a [ScopesRow],
    pub exports: &'a [ExportSyntaxRow],
    pub regions: &'a [FlowRegionsRow],
    pub unconditional_conditions: &'a [Id],
}

#[derive(Default)]
pub struct Outcome {
    pub sites: Vec<SourceContextSitesRow>,
    pub arguments: Vec<SourceContextArgumentsRow>,
}

fn one<T>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

/// Collection is admitted only for the one positional tuple formal declared by the class
/// protocol. Ordinary calls retain their own nonvariadic binder boundary.
fn bind(
    protocol: &ModelContextProtocolsRow,
    signature: &[cpg_schema::summary_contract::SignatureParameter],
    arguments: &[ArgumentsRow],
) -> Result<Option<Vec<BoundArgument>>, ()> {
    if signature.len() > 128 || arguments.len() > 128 {
        return Err(());
    }
    if protocol.exit == ContextExitKind::SuppressClasses {
        let [parameter] = signature else {
            return Err(());
        };
        if parameter.kind != ParameterKind::VarPositional
            || protocol.exception_formal.as_deref() != Some(&parameter.name)
        {
            return Err(());
        }
        if arguments
            .iter()
            .any(|a| a.kind != ArgumentKind::Positional || a.keyword.is_some())
        {
            return Ok(None);
        }
        return Ok(Some(
            arguments
                .iter()
                .map(|a| BoundArgument {
                    argument_fact_id: a.fact_id,
                    parameter_fact_id: parameter.evidence_id,
                    parameter_name: parameter.name.clone(),
                })
                .collect(),
        ));
    }
    if signature.iter().any(|p| {
        matches!(
            p.kind,
            ParameterKind::VarPositional | ParameterKind::VarKeyword
        )
    }) {
        return Err(());
    }
    let bound = match bind_arguments(signature, arguments) {
        Ok(bound) => bound,
        Err(cpg_schema::codebook::BoundaryReason::UnsupportedControlFlow) => return Ok(None),
        Err(_) => return Err(()),
    };
    // Only the authored entry default has runtime meaning. A future independent optional
    // constructor parameter requires its own contract rather than borrowing a stub default.
    if bound
        .defaults
        .iter()
        .any(|p| protocol.entry_formal.as_deref() != Some(&p.parameter_name))
    {
        return Err(());
    }
    Ok(Some(bound.explicit))
}

pub fn admit(inputs: Inputs<'_>) -> Outcome {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    for node in inputs.syntax {
        children
            .entry((node.snapshot_id, node.parent_node_id))
            .or_default()
            .push(node);
    }
    for nodes in children.values_mut() {
        nodes.sort_by_key(|n| (n.field, n.ordinal, n.fact_id));
    }
    let mut out = Outcome::default();
    for item in inputs
        .syntax
        .iter()
        .filter(|n| n.kind == S::WithItem && n.field == F::Item)
    {
        let Some((mut site, mut arguments)) = admit_item(item, &inputs, &children) else {
            continue;
        };
        site.site_id = site_id(&site, &arguments);
        for argument in &mut arguments {
            argument.site_id = site.site_id;
        }
        out.sites.push(site);
        out.arguments.extend(arguments);
    }
    out.sites.sort_by_key(|s| (s.snapshot_id, s.site_id));
    out.arguments
        .sort_by_key(|a| (a.snapshot_id, a.site_id, a.ordinal));
    out
}

fn admit_item(
    item: &SyntaxNodesRow,
    inputs: &Inputs<'_>,
    children: &HashMap<(Id, Id), Vec<&SyntaxNodesRow>>,
) -> Option<(SourceContextSitesRow, Vec<SourceContextArgumentsRow>)> {
    let snapshot = item.snapshot_id;
    let owner = item.owner_node_id?;
    let declaration = one(inputs
        .declarations
        .iter()
        .filter(|d| d.snapshot_id == snapshot && d.node_id == owner))?;
    if declaration.kind != DeclarationKind::Function
        || inputs.syntax.iter().any(|n| {
            n.snapshot_id == snapshot
                && n.owner_node_id == Some(owner)
                && matches!(n.kind, S::ExprYield | S::ExprYieldFrom)
        })
    {
        return None;
    }
    let with = one(inputs
        .syntax
        .iter()
        .filter(|n| n.snapshot_id == snapshot && n.node_id == item.parent_node_id))?;
    if with.kind != S::StmtWith
        || with.detail.as_deref() == Some("async")
        || with.owner_node_id != Some(owner)
        || with.module_node_id != item.module_node_id
        || item.start_byte < with.start_byte
        || item.end_byte > with.end_byte
    {
        return None;
    }
    let item_children = children.get(&(snapshot, item.node_id))?;
    if item_children.iter().any(|n| {
        !matches!(n.field, F::Value | F::Target)
            || n.owner_node_id != Some(owner)
            || n.module_node_id != item.module_node_id
            || n.start_byte < item.start_byte
            || n.end_byte > item.end_byte
    }) || item_children
        .iter()
        .filter(|n| n.field == F::Target)
        .count()
        > 1
    {
        return None;
    }
    let expression = one(item_children
        .iter()
        .copied()
        .filter(|n| n.field == F::Value && n.kind == S::ExprCall))?;
    let call = one(inputs
        .calls
        .iter()
        .filter(|c| c.snapshot_id == snapshot && c.node_id == expression.node_id))?;
    if call.in_annotation
        || call.owner_node_id != Some(owner)
        || call.module_node_id != item.module_node_id
        || call.start_byte != expression.start_byte
        || call.end_byte != expression.end_byte
    {
        return None;
    }
    let operands = children.get(&(snapshot, call.node_id))?;
    if operands.len() > 129
        || operands.iter().any(|n| {
            n.owner_node_id != Some(owner)
                || n.module_node_id != call.module_node_id
                || n.start_byte < call.start_byte
                || n.end_byte > call.end_byte
        })
    {
        return None;
    }
    let callee = one(operands
        .iter()
        .copied()
        .filter(|n| n.field == F::Callee && n.kind == S::ExprName))?;
    if callee.start_byte != call.callee_start_byte || callee.end_byte != call.callee_end_byte {
        return None;
    }
    let reference = one(inputs.references.iter().filter(|r| {
        r.snapshot_id == snapshot
            && r.name_node_id == callee.node_id
            && r.module_node_id == call.module_node_id
            && Some(r.name.as_str()) == callee.detail.as_deref()
    }))?;
    let resolution = one(inputs
        .resolutions
        .iter()
        .filter(|r| r.snapshot_id == snapshot && r.reference_id == reference.node_id))?;
    if resolution.reason.is_some() || resolution.captured || resolution.builtin_name.is_some() {
        return None;
    }
    let binding = one(inputs
        .bindings
        .iter()
        .filter(|b| b.snapshot_id == snapshot && Some(b.node_id) == resolution.binding_id))?;
    if binding.kind != BindingKind::FromImport
        || binding.module_node_id != call.module_node_id
        || binding.start_byte >= declaration.start_byte
        || inputs
            .bindings
            .iter()
            .filter(|b| {
                b.snapshot_id == snapshot
                    && b.scope_id == binding.scope_id
                    && b.name == binding.name
            })
            .count()
            != 1
    {
        return None;
    }
    let scope = one(inputs
        .scopes
        .iter()
        .filter(|s| s.snapshot_id == snapshot && s.node_id == binding.scope_id))?;
    if scope.kind != LexicalScopeKind::Module {
        return None;
    }
    let export = one(inputs.exports.iter().filter(|e| {
        e.snapshot_id == snapshot
            && e.module_node_id == binding.module_node_id
            && e.start_byte == binding.start_byte
            && e.end_byte == binding.end_byte
    }))?;
    if export.kind != ExportSyntaxKind::ImportFrom {
        return None;
    }
    let mut regions: Vec<_> = inputs
        .regions
        .iter()
        .filter(|r| {
            r.snapshot_id == snapshot
                && r.module_node_id == binding.module_node_id
                && r.scope_kind == LexicalScopeKind::Module
                && r.start_byte <= binding.start_byte
                && r.end_byte >= binding.end_byte
        })
        .collect();
    regions.sort_by_key(|r| (r.end_byte - r.start_byte, r.start_byte, r.fact_id));
    let region = regions.first()?;
    if region.approximated
        || !inputs
            .unconditional_conditions
            .contains(&region.condition_id)
    {
        return None;
    }
    let class = one(inputs.definitions.iter().filter(|d| {
        d.snapshot_id == snapshot
            && d.kind == DefinitionKind::Class
            && Some(d.module_name.as_str()) == export.resolved_module.as_deref()
            && Some(d.qualified_name.as_str()) == export.imported_name.as_deref()
    }))?;
    let protocol = one(inputs
        .protocols
        .iter()
        .filter(|p| p.snapshot_id == snapshot && p.class_node_id == class.symbol_node_id))?;
    let initializer = one(inputs
        .definitions
        .iter()
        .filter(|d| d.snapshot_id == snapshot && d.fact_id == protocol.initialization_fact_id))?;
    let allocation = one(inputs
        .definitions
        .iter()
        .filter(|d| d.snapshot_id == snapshot && d.fact_id == protocol.allocation_fact_id))?;
    // Explicit call occurrences only. Identifier constructor records and implicit entry at
    // this range are different source roles, never additional construction targets.
    let phases: Vec<_> = inputs
        .provider_calls
        .iter()
        .filter(|p| {
            p.snapshot_id == snapshot
                && p.module_node_id == call.module_node_id
                && p.start_byte == call.start_byte
                && p.end_byte == call.end_byte
                && p.site_kind == PysaSiteKind::Regular
        })
        .collect();
    if phases.len() != 2
        || phases.iter().any(|p| {
            p.callee_kind != PysaCalleeKind::Call
                || p.target_kind != PysaTargetKind::Function
                || p.unresolved_reason.is_some()
                || p.higher_order_index.is_some()
                || p.implicit_dunder_call != Some(false)
        })
    {
        return None;
    }
    let new = one(phases
        .iter()
        .copied()
        .filter(|p| p.phase == InvocationPhase::New))?;
    let init = one(phases
        .iter()
        .copied()
        .filter(|p| p.phase == InvocationPhase::Init))?;
    if new.target_module.as_deref() != Some(&allocation.module_name)
        || new.target_key.as_deref() != Some(&allocation.key)
        || init.target_module.as_deref() != Some(&initializer.module_name)
        || init.target_key.as_deref() != Some(&initializer.key)
        || init.receiver_module.as_deref() != Some(&class.module_name)
        || init.receiver_key.as_deref() != Some(&class.key)
    {
        return None;
    }
    let mut arguments: Vec<_> = inputs
        .arguments
        .iter()
        .filter(|a| a.snapshot_id == snapshot && a.call_node_id == call.node_id)
        .cloned()
        .collect();
    arguments.sort_by_key(|a| a.ordinal);
    if arguments.len() > 128
        || arguments.len() as i64 != call.positional_count + call.keyword_count
        || operands.len() != arguments.len() + 1
        || arguments.iter().enumerate().any(|(i, a)| {
            a.ordinal != i as i64
                || !matches!(a.kind, ArgumentKind::Positional | ArgumentKind::Keyword)
        })
    {
        return None;
    }
    let signatures = initializer_signatures(initializer, inputs.parameters).ok()?;
    let mut matched = None;
    for signature in &signatures {
        if let Some(bound) = bind(protocol, signature, &arguments).ok()? {
            if matched
                .as_ref()
                .is_some_and(|previous: &Vec<BoundArgument>| {
                    previous.iter().zip(&bound).any(|(a, b)| {
                        a.parameter_name != b.parameter_name
                            || a.argument_fact_id != b.argument_fact_id
                    })
                })
            {
                return None;
            }
            if matched.is_none() {
                matched = Some(bound);
            }
        }
    }
    let constructor_valid = matched.is_some();
    let entry_argument = matched
        .as_ref()
        .into_iter()
        .flatten()
        .find(|b| {
            protocol.entry == ContextEntryKind::ArgumentOrNone
                && protocol.entry_formal.as_deref() == Some(&b.parameter_name)
        })
        .map(|b| b.argument_fact_id);
    let mut rows = Vec::new();
    for (i, argument) in arguments.iter().enumerate() {
        let operand = one(operands.iter().copied().filter(|n| {
            n.field == F::Argument
                && n.ordinal == argument.ordinal
                && n.start_byte == argument.value_start_byte
                && n.end_byte == argument.value_end_byte
        }))?;
        let mut row = SourceContextArgumentsRow {
            snapshot_id: snapshot,
            site_id: Id([0; 16]),
            ordinal: argument.ordinal,
            argument_fact_id: argument.fact_id,
            expression_fact_id: operand.fact_id,
            parameter_fact_id: matched.as_ref().map(|m| m[i].parameter_fact_id),
            exception_class_node_id: None,
            exception_class_fact_id: None,
            exception_module_fact_id: None,
            reference_fact_id: None,
            resolution_fact_id: None,
        };
        if protocol.exit == ContextExitKind::SuppressClasses
            && operand.kind == S::ExprName
            && let Some((reference, resolution, class, module)) = builtin_class(operand, inputs)
        {
            row.exception_class_node_id = Some(class.symbol_node_id);
            row.exception_class_fact_id = Some(class.fact_id);
            row.exception_module_fact_id = Some(module.fact_id);
            row.reference_fact_id = Some(reference.fact_id);
            row.resolution_fact_id = Some(resolution.fact_id);
        }
        rows.push(row);
    }
    Some((
        SourceContextSitesRow {
            snapshot_id: snapshot,
            site_id: Id([0; 16]),
            function_node_id: owner,
            with_node_id: with.node_id,
            with_fact_id: with.fact_id,
            item_node_id: item.node_id,
            item_fact_id: item.fact_id,
            item_ordinal: item.ordinal,
            call_node_id: call.node_id,
            call_fact_id: call.fact_id,
            expression_fact_id: expression.fact_id,
            protocol_id: binding_id(protocol),
            model_id: protocol.model_id,
            class_node_id: protocol.class_node_id,
            reference_fact_id: reference.fact_id,
            resolution_fact_id: resolution.fact_id,
            import_binding_fact_id: binding.fact_id,
            import_region_fact_id: region.fact_id,
            import_condition_id: region.condition_id,
            export_fact_id: export.fact_id,
            allocation_call_fact_id: new.fact_id,
            initialization_call_fact_id: init.fact_id,
            constructor_valid,
            entry_argument_fact_id: entry_argument,
        },
        rows,
    ))
}

fn builtin_class<'a>(
    node: &SyntaxNodesRow,
    inputs: &Inputs<'a>,
) -> Option<(
    &'a ReferencesRow,
    &'a ReferenceResolutionsRow,
    &'a ContextDefinitionsRow,
    &'a ContextModulesRow,
)> {
    let reference = one(inputs
        .references
        .iter()
        .filter(|r| r.snapshot_id == node.snapshot_id && r.name_node_id == node.node_id))?;
    let resolution = one(inputs
        .resolutions
        .iter()
        .filter(|r| r.snapshot_id == node.snapshot_id && r.reference_id == reference.node_id))?;
    if resolution.binding_id.is_some()
        || resolution.reason.is_some()
        || resolution.captured
        || resolution.builtin_name.as_deref() != Some(&reference.name)
    {
        return None;
    }
    let class = one(inputs.definitions.iter().filter(|d| {
        d.snapshot_id == node.snapshot_id
            && d.module_name == "builtins"
            && d.qualified_name == reference.name
            && d.kind == DefinitionKind::Class
    }))?;
    let module = one(inputs.modules.iter().filter(|m| {
        m.snapshot_id == node.snapshot_id
            && m.module_node_id == class.module_node_id
            && m.module_name == "builtins"
            && m.origin == ModuleOrigin::BundledTypeshed
    }))?;
    Some((reference, resolution, class, module))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpg_schema::codebook::Origin;
    use cpg_schema::summary_contract::SignatureParameter;

    #[test]
    fn binding_budget_is_unknown_while_a_missing_required_argument_is_a_known_mismatch() {
        let protocol = ModelContextProtocolsRow {
            snapshot_id: Id::ZERO,
            model_id: Id::ZERO,
            revision: 1,
            class_node_id: Id::ZERO,
            class_fact_id: Id::ZERO,
            class_module_fact_id: Id::ZERO,
            allocation_node_id: Id::ZERO,
            allocation_fact_id: Id::ZERO,
            allocation_module_fact_id: Id::ZERO,
            initialization_node_id: Id::ZERO,
            initialization_fact_id: Id::ZERO,
            initialization_module_fact_id: Id::ZERO,
            entry: ContextEntryKind::ArgumentOrNone,
            entry_formal: Some("enter_result".into()),
            exit: ContextExitKind::Preserve,
            exception_formal: None,
            origin: Origin::SyntheticModel,
        };
        let signature: Vec<_> = (0..129)
            .map(|n| SignatureParameter {
                evidence_id: Id([n as u8; 16]),
                ordinal: n,
                name: format!("arg{n}"),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            })
            .collect();
        assert!(bind(&protocol, &signature, &[]).is_err());
        assert_eq!(bind(&protocol, &signature[..1], &[]), Ok(None));
        assert_eq!(bind(&protocol, &[], &[]), Ok(Some(Vec::new())));
    }
}
