//! Authored synchronous class protocols, distinct from observed callable applications.
//! A class assertion supplies runtime lifecycle meaning; constructor facts only bind roles.
use crate::codebook::{ContextEntryKind, ContextExitKind, Origin};
use crate::id::{Id, IdHasher};
use crate::table::table;

/// Complete initializer overloads, with the implicit receiver removed. Constructors remain
/// separate call phases; this view is only for source argument binding, not runtime totality.
pub fn initializer_signatures(
    definition: &crate::tables::ContextDefinitionsRow,
    parameters: &[crate::tables::ContextParametersRow],
) -> Result<Vec<Vec<crate::summary_contract::SignatureParameter>>, &'static str> {
    use crate::codebook::{DefinitionKind, ParameterKind as P, SignatureForm};
    use crate::summary_contract::SignatureParameter;
    if definition.kind != DefinitionKind::Function {
        return Err("initializer is not a function");
    }
    let count = definition
        .signature_count
        .filter(|n| (1..=128).contains(n))
        .ok_or("missing initializer signature set")?;
    let rows = crate::context_observations::parameters(parameters.iter().filter(|p| {
        p.snapshot_id == definition.snapshot_id && p.symbol_node_id == definition.symbol_node_id
    }))?;
    if rows.len() > 1024
        || rows.iter().any(|p| {
            p.module_node_id != definition.module_node_id
                || p.signature_index < 0
                || p.signature_index >= count
                || p.form != SignatureForm::List
        })
    {
        return Err("invalid initializer signature set");
    }
    let mut signatures = Vec::new();
    for index in 0..count {
        let mut signature: Vec<_> = rows
            .iter()
            .copied()
            .filter(|p| p.signature_index == index)
            .collect();
        signature.sort_by_key(|p| p.ordinal);
        if signature.is_empty()
            || signature
                .iter()
                .enumerate()
                .any(|(i, p)| p.ordinal != Some(i as i64))
        {
            return Err("incomplete initializer signature");
        }
        let receiver = signature[0];
        if !matches!(
            receiver.kind,
            Some(P::PositionalOnly | P::PositionalOrKeyword)
        ) || receiver.required != Some(true)
            || receiver.name.as_deref() != Some("self")
        {
            return Err("initializer lacks an explicit receiver");
        }
        let mut names = std::collections::BTreeSet::new();
        let signature = signature
            .into_iter()
            .skip(1)
            .enumerate()
            .map(|(i, p)| {
                let mut parameter = SignatureParameter::from_context(p)
                    .map_err(|_| "incomplete initializer parameter")?;
                if !names.insert(parameter.name.clone()) {
                    return Err("duplicate initializer parameter");
                }
                parameter.ordinal = i as i64;
                Ok(parameter)
            })
            .collect::<Result<Vec<_>, _>>()?;
        signatures.push(signature);
    }
    Ok(signatures)
}

table!(
    ModelContextProtocols, ModelContextProtocolsRow = "model_context_protocols",
    family = Findings,
    key = [snapshot_id, model_id, class_node_id],
    checks = [("revision_positive", "revision > 0")],
    {
        snapshot_id: Id,
        model_id: Id,
        revision: i64,
        class_node_id: Id,
        class_fact_id: Id,
        class_module_fact_id: Id,
        allocation_node_id: Id,
        allocation_fact_id: Id,
        allocation_module_fact_id: Id,
        initialization_node_id: Id,
        initialization_fact_id: Id,
        initialization_module_fact_id: Id,
        entry: ContextEntryKind,
        entry_formal: Option<String>,
        exit: ContextExitKind,
        exception_formal: Option<String>,
        origin: Origin,
    }
);

/// Shape/identity check shared by source and immutable consumers. Source reconstruction
/// separately owns pin matching and constructor signatures; this never fabricates call facts.
pub fn valid_shape(row: &ModelContextProtocolsRow) -> bool {
    row.revision > 0
        && row.origin == Origin::SyntheticModel
        && (row.entry == ContextEntryKind::ArgumentOrNone) == row.entry_formal.is_some()
        && (row.exit == ContextExitKind::SuppressClasses) == row.exception_formal.is_some()
        && row.entry_formal.as_ref().is_none_or(|s| !s.is_empty())
        && row.exception_formal.as_ref().is_none_or(|s| !s.is_empty())
}

/// Content identity for one class protocol assertion and its exact binding, independent of
/// any source occurrence or provider's method presentation.
pub fn binding_id(row: &ModelContextProtocolsRow) -> Id {
    use crate::codebook::Codebook;
    IdHasher::new("context-protocol-binding")
        .id(row.model_id)
        .i64(row.revision)
        .id(row.class_node_id)
        .id(row.class_fact_id)
        .id(row.class_module_fact_id)
        .id(row.allocation_node_id)
        .id(row.allocation_fact_id)
        .id(row.allocation_module_fact_id)
        .id(row.initialization_node_id)
        .id(row.initialization_fact_id)
        .id(row.initialization_module_fact_id)
        .i64(i64::from(row.entry.code()))
        .opt_str(row.entry_formal.as_deref())
        .i64(i64::from(row.exit.code()))
        .opt_str(row.exception_formal.as_deref())
        .finish_id()
}

table!(
    /// Admission of a fresh direct context construction. This certifies source binding and
    /// constructor argument shape; entry and argument completion are separate obligations.
    SourceContextSites, SourceContextSitesRow = "source_context_sites",
    family = Findings,
    key = [snapshot_id, site_id],
    checks = [("ordinal_nonnegative", "item_ordinal >= 0")],
    {
        snapshot_id: Id,
        site_id: Id,
        function_node_id: Id,
        with_node_id: Id,
        with_fact_id: Id,
        item_node_id: Id,
        item_fact_id: Id,
        item_ordinal: i64,
        call_node_id: Id,
        call_fact_id: Id,
        expression_fact_id: Id,
        protocol_id: Id,
        model_id: Id,
        class_node_id: Id,
        reference_fact_id: Id,
        resolution_fact_id: Id,
        import_binding_fact_id: Id,
        import_region_fact_id: Id,
        import_condition_id: Id,
        export_fact_id: Id,
        allocation_call_fact_id: Id,
        initialization_call_fact_id: Id,
        constructor_valid: bool,
        entry_argument_fact_id: Option<Id>,
    }
);

table!(
    /// Source order is retained independently of formal binding. Exception class inputs are
    /// admitted only through an unshadowed builtin resolution; arbitrary values stay opaque.
    SourceContextArguments, SourceContextArgumentsRow = "source_context_arguments",
    family = Findings,
    key = [snapshot_id, site_id, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        site_id: Id,
        ordinal: i64,
        argument_fact_id: Id,
        expression_fact_id: Id,
        parameter_fact_id: Option<Id>,
        exception_class_node_id: Option<Id>,
        exception_class_fact_id: Option<Id>,
        exception_module_fact_id: Option<Id>,
        reference_fact_id: Option<Id>,
        resolution_fact_id: Option<Id>,
    }
);

/// Identity includes every ordered argument, so neither reordering nor swapping one site's
/// evidence into another preserves a certificate. The source validator also reconstructs it.
pub fn site_id(row: &SourceContextSitesRow, arguments: &[SourceContextArgumentsRow]) -> Id {
    let mut h = IdHasher::new("source-context-site");
    h.id(row.function_node_id)
        .id(row.with_node_id)
        .id(row.with_fact_id)
        .id(row.item_node_id)
        .id(row.item_fact_id)
        .i64(row.item_ordinal)
        .id(row.call_node_id)
        .id(row.call_fact_id)
        .id(row.expression_fact_id)
        .id(row.protocol_id)
        .id(row.model_id)
        .id(row.class_node_id)
        .id(row.reference_fact_id)
        .id(row.resolution_fact_id)
        .id(row.import_binding_fact_id)
        .id(row.import_region_fact_id)
        .id(row.import_condition_id)
        .id(row.export_fact_id)
        .id(row.allocation_call_fact_id)
        .id(row.initialization_call_fact_id)
        .i64(i64::from(row.constructor_valid))
        .opt_id(row.entry_argument_fact_id)
        .i64(arguments.len() as i64);
    for a in arguments {
        h.i64(a.ordinal)
            .id(a.argument_fact_id)
            .id(a.expression_fact_id)
            .opt_id(a.parameter_fact_id)
            .opt_id(a.exception_class_node_id)
            .opt_id(a.exception_class_fact_id)
            .opt_id(a.exception_module_fact_id)
            .opt_id(a.reference_fact_id)
            .opt_id(a.resolution_fact_id);
    }
    h.finish_id()
}

/// Shared admission after source reconstruction or immutable IPC decoding. Runtime lifecycle
/// order belongs to completion, not to serving code.
pub fn admits_site(
    row: &SourceContextSitesRow,
    arguments: &[SourceContextArgumentsRow],
    protocol: &ModelContextProtocolsRow,
) -> bool {
    valid_shape(protocol)
        && row.protocol_id == binding_id(protocol)
        && row.model_id == protocol.model_id
        && row.class_node_id == protocol.class_node_id
        && row.snapshot_id == protocol.snapshot_id
        && row.item_ordinal >= 0
        && arguments.len() <= 128
        && arguments.iter().enumerate().all(|(i, a)| {
            a.snapshot_id == row.snapshot_id
                && a.site_id == row.site_id
                && a.ordinal == i as i64
                && a.parameter_fact_id.is_some() == row.constructor_valid
                && [
                    a.exception_class_fact_id,
                    a.exception_module_fact_id,
                    a.reference_fact_id,
                    a.resolution_fact_id,
                ]
                .iter()
                .all(|id| id.is_some() == a.exception_class_node_id.is_some())
        })
        && row.entry_argument_fact_id.is_none_or(|id| {
            row.constructor_valid
                && protocol.entry == ContextEntryKind::ArgumentOrNone
                && arguments.iter().any(|a| a.argument_fact_id == id)
        })
        && row.site_id == site_id(row, arguments)
}

/// Structural lifecycle obligations in a finite source proof. Completion owns transitions;
/// this shared consumer prevents foreign sites, orphan entry, reordered exit and incomplete
/// cleanup from becoming an admitted immutable summary.
pub fn admit_proof<'a>(
    function: Id,
    steps: &[crate::id::recipe::SummaryFlowProofStep],
    resolve: impl Fn(Id) -> Option<&'a SourceContextSitesRow>,
) -> Result<std::collections::BTreeSet<Id>, &'static str> {
    use crate::codebook::SummaryFlowStepKind as K;
    let mut entered = Vec::new();
    let mut constructed = None;
    let mut cited = std::collections::BTreeSet::new();
    for step in steps {
        if !matches!(
            step.kind,
            K::ContextConstruction | K::ContextEntry | K::ContextExit
        ) {
            continue;
        }
        let site = resolve(step.evidence_id).ok_or("missing context site certificate")?;
        if site.function_node_id != function {
            return Err("foreign context site certificate");
        }
        cited.insert(site.site_id);
        match step.kind {
            K::ContextConstruction => {
                if constructed.take().is_some() {
                    return Err("context construction has no entry");
                }
                if site.constructor_valid {
                    constructed = Some(site.site_id);
                }
            }
            K::ContextEntry => {
                if constructed.take() != Some(site.site_id)
                    || !site.constructor_valid
                    || entered.contains(&site.site_id)
                {
                    return Err("context entry has no fresh construction");
                }
                entered.push(site.site_id);
            }
            K::ContextExit => {
                if constructed.is_some() || entered.pop() != Some(site.site_id) {
                    return Err("context cleanup is not in reverse entry order");
                }
            }
            _ => unreachable!(),
        }
    }
    if constructed.is_some() || !entered.is_empty() {
        return Err("context proof has incomplete cleanup");
    }
    Ok(cited)
}
