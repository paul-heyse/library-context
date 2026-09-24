//! The `flow` family (ADR-0022 §The flow provider; ADR-0012 amendment): `cpg-flow`'s facts for
//! every UTF-8 release module, from the text Pyrefly parsed, as fact rows. A module ty cannot
//! index says why in its coverage row; the others are unaffected.

use std::collections::{BTreeMap, HashMap};

use cpg_flow::{
    Condition as FlowCondition, Input, RuntimeBindings, RuntimeContext, Scope, Sink, SkipCounts,
    Span,
};
use cpg_schema::codebook::{
    ExportSyntaxKind, ExtractionMode, Fidelity, FlowSink, Modality, Origin,
};
use cpg_schema::condition::Condition;
use cpg_schema::condition_kernel::DiagramNode;
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::tables::{
    ConditionLiterals, ConditionLiteralsRow, ConditionNodes, ConditionNodesRow, Conditions,
    ConditionsRow, ExportSyntaxRow, FlowAttributeLoads, FlowAttributeLoadsRow, FlowDefinitions,
    FlowDefinitionsRow, FlowReaching, FlowReachingRow, FlowRegions, FlowRegionsRow, FlowTestLeaves,
    FlowTestLeavesRow, FlowTests, FlowTestsRow, FlowUses, FlowUsesRow, FlowValues, FlowValuesRow,
};

use crate::facts::{FactSink, Provenance, Surface, fact_row};
use crate::lexical::LexicalOut;

/// One module handed to the provider.
pub(crate) struct FlowModule {
    pub node_id: Id,
    pub path: String,
    pub text: String,
    pub runtime: RuntimeBindings,
}

/// The runtime override is allowed only where every lexical candidate is the same recognized
/// import. The import alias's syntax id is the lexical binding's site id.
pub(crate) fn runtime_bindings(lex: &LexicalOut, imports: &[ExportSyntaxRow]) -> RuntimeBindings {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Special {
        Checking,
        Typing,
        Sys,
        Os,
    }
    let by_site: HashMap<Id, Special> = imports
        .iter()
        .filter_map(|row| {
            let target = match (
                row.kind,
                row.resolved_module.as_deref(),
                row.imported_name.as_deref(),
            ) {
                (
                    ExportSyntaxKind::ImportFrom,
                    Some("typing" | "typing_extensions"),
                    Some("TYPE_CHECKING"),
                ) => Special::Checking,
                (ExportSyntaxKind::Import, Some("typing" | "typing_extensions"), None) => {
                    Special::Typing
                }
                (ExportSyntaxKind::Import, Some("sys"), None) => Special::Sys,
                (ExportSyntaxKind::Import, Some("os"), None) => Special::Os,
                _ => return None,
            };
            Some((row.node_id, target))
        })
        .collect();
    let binding: HashMap<Id, Special> = lex
        .bindings
        .iter()
        .filter_map(|b| {
            by_site
                .get(&b.site_node_id)
                .copied()
                .map(|target| (b.node_id, target))
        })
        .collect();
    let mut candidates: HashMap<Id, Vec<Option<Special>>> = HashMap::new();
    for resolution in &lex.resolutions {
        candidates.entry(resolution.reference_id).or_default().push(
            resolution
                .binding_id
                .and_then(|id| binding.get(&id).copied()),
        );
    }
    let mut out = RuntimeBindings::default();
    for reference in &lex.references {
        let Some(found) = candidates.get(&reference.node_id) else {
            continue;
        };
        let Some(Some(special)) = found.first() else {
            continue;
        };
        if !found.iter().all(|item| *item == Some(*special)) {
            continue;
        }
        let Ok(start) = u32::try_from(reference.start_byte) else {
            continue;
        };
        let Ok(end) = u32::try_from(reference.end_byte) else {
            continue;
        };
        let span = Span { start, end };
        match special {
            Special::Checking => {
                out.checking_names.insert(span);
            }
            Special::Typing => {
                out.typing_modules.insert(span);
            }
            Special::Sys => {
                out.sys_modules.insert(span);
            }
            Special::Os => {
                out.os_modules.insert(span);
            }
        }
    }
    out
}

#[derive(Default)]
pub(crate) struct FlowOut {
    pub uses: Vec<FlowUsesRow>,
    pub definitions: Vec<FlowDefinitionsRow>,
    pub reaching: Vec<FlowReachingRow>,
    pub values: Vec<FlowValuesRow>,
    pub regions: Vec<FlowRegionsRow>,
    pub tests: Vec<FlowTestsRow>,
    pub test_leaves: Vec<FlowTestLeavesRow>,
    pub attribute_loads: Vec<FlowAttributeLoadsRow>,
    pub conditions: Vec<ConditionsRow>,
    pub condition_nodes: Vec<ConditionNodesRow>,
    pub literals: Vec<ConditionLiteralsRow>,
    /// Per module, why it has no flow facts (none: indexed).
    pub errors: BTreeMap<Id, String>,
    /// Modules indexed from a recovered tree, with their syntax error counts.
    pub recovered: BTreeMap<Id, usize>,
    pub skips: BTreeMap<Id, SkipCounts>,
    /// `TYPE_CHECKING` names renamed across the release.
    pub renamed: u32,
}

fn provenance() -> Provenance {
    Provenance {
        surface: Surface::TyFlow,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity: Fidelity::NormalizedStructural,
    }
}

fn scope(
    s: Scope,
) -> (
    cpg_schema::codebook::LexicalScopeKind,
    Option<i64>,
    Option<i64>,
) {
    (
        s.kind,
        s.name.map(|n| i64::from(n.start)),
        s.name.map(|n| i64::from(n.end)),
    )
}

fn sink_code(s: Sink) -> FlowSink {
    match s {
        Sink::Definition => FlowSink::Definition,
        Sink::Argument => FlowSink::Argument,
        Sink::Return => FlowSink::Return,
        Sink::Yield => FlowSink::Yield,
        Sink::Raise => FlowSink::Raise,
    }
}

/// Index every module and turn its facts into rows.
pub(crate) fn run(
    sink: &mut FactSink,
    modules: &[FlowModule],
    context: &RuntimeContext,
) -> FlowOut {
    let inputs: Vec<Input> = modules
        .iter()
        .map(|m| Input {
            path: m.path.clone(),
            text: m.text.clone(),
            runtime: m.runtime.clone(),
        })
        .collect();
    let flows = cpg_flow::index(&inputs, context);
    let mut out = FlowOut::default();
    let mut conditions: BTreeMap<Id, FlowCondition> = BTreeMap::new();
    let mut condition = |c: &FlowCondition| -> Id {
        let id = c.id();
        conditions.entry(id).or_insert_with(|| c.clone());
        id
    };
    for (m, flow) in modules.iter().zip(flows) {
        out.renamed += flow.renamed;
        if let Some(e) = flow.error {
            out.errors.insert(m.node_id, e);
            continue;
        }
        if flow.syntax_errors > 0 {
            out.recovered.insert(m.node_id, flow.syntax_errors);
        }
        out.skips.insert(m.node_id, flow.skips.clone());
        let module = m.node_id;
        let use_ids: Vec<Id> = flow
            .uses
            .iter()
            .map(|u| {
                IdHasher::new("flow_use")
                    .id(module)
                    .i64(i64::from(u.span.start))
                    .i64(i64::from(u.span.end))
                    .finish_id()
            })
            .collect();
        for (u, &use_id) in flow.uses.iter().zip(&use_ids) {
            let (scope_kind, scope_start_byte, scope_end_byte) = scope(u.scope);
            out.uses.push(fact_row!(
                sink,
                FlowUses,
                provenance(),
                FlowUsesRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    use_id,
                    module_node_id: module,
                    place: u.place.clone(),
                    scope_kind,
                    scope_start_byte,
                    scope_end_byte,
                    start_byte: i64::from(u.span.start),
                    end_byte: i64::from(u.span.end),
                    annotation: u.annotation,
                }
            ));
        }
        let def_ids: Vec<Id> = flow
            .defs
            .iter()
            .map(|d| {
                IdHasher::new("flow_definition")
                    .id(module)
                    .i64(i64::from(cpg_schema::Codebook::code(d.kind)))
                    .i64(i64::from(d.target.start))
                    .i64(i64::from(d.target.end))
                    .str(&d.place)
                    .finish_id()
            })
            .collect();
        for (d, &definition_id) in flow.defs.iter().zip(&def_ids) {
            let (scope_kind, scope_start_byte, scope_end_byte) = scope(d.scope);
            out.definitions.push(fact_row!(
                sink,
                FlowDefinitions,
                provenance(),
                FlowDefinitionsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    definition_id,
                    module_node_id: module,
                    place: d.place.clone(),
                    kind: d.kind,
                    scope_kind,
                    scope_start_byte,
                    scope_end_byte,
                    start_byte: i64::from(d.target.start),
                    end_byte: i64::from(d.target.end),
                    value_start_byte: d.value.map(|v| i64::from(v.start)),
                    value_end_byte: d.value.map(|v| i64::from(v.end)),
                }
            ));
        }
        for r in &flow.reaching {
            let condition_id = condition(&r.condition);
            out.reaching.push(fact_row!(
                sink,
                FlowReaching,
                provenance(),
                FlowReachingRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    use_id: use_ids[r.use_ix as usize],
                    definition_id: r.def_ix.map(|d| def_ids[d as usize]),
                    condition_id,
                    approximated: r.condition.approximated(),
                    loop_carried: r.loop_carried,
                }
            ));
        }
        for v in &flow.values {
            let condition_id = condition(&v.condition);
            out.values.push(fact_row!(
                sink,
                FlowValues,
                provenance(),
                FlowValuesRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: module,
                    sink: sink_code(v.sink),
                    sink_start_byte: i64::from(v.span.start),
                    sink_end_byte: i64::from(v.span.end),
                    use_id: use_ids[v.use_ix as usize],
                    identity: v.identity,
                    through_call: v.through_call,
                    condition_id,
                    approximated: v.condition.approximated() || v.through_call,
                }
            ));
        }
        for t in &flow.tests {
            let condition_id = condition(&t.condition);
            let (scope_kind, scope_start_byte, scope_end_byte) = scope(t.scope);
            out.tests.push(fact_row!(
                sink,
                FlowTests,
                provenance(),
                FlowTestsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: module,
                    scope_kind,
                    scope_start_byte,
                    scope_end_byte,
                    start_byte: i64::from(t.span.start),
                    end_byte: i64::from(t.span.end),
                    condition_id,
                }
            ));
        }
        for leaf in &flow.test_leaves {
            let condition_id = condition(&leaf.condition);
            let (scope_kind, scope_start_byte, scope_end_byte) = scope(leaf.scope);
            out.test_leaves.push(fact_row!(
                sink,
                FlowTestLeaves,
                provenance(),
                FlowTestLeavesRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: module,
                    scope_kind,
                    scope_start_byte,
                    scope_end_byte,
                    predicate_key: leaf.predicate_key.clone(),
                    test_start_byte: i64::from(leaf.test_span.start),
                    test_end_byte: i64::from(leaf.test_span.end),
                    condition_id,
                    atom_id: IdHasher::new("bdd-atom").str(&leaf.atom).finish_id(),
                    atom: leaf.atom.clone(),
                    leaf_start_byte: i64::from(leaf.leaf_span.start),
                    leaf_end_byte: i64::from(leaf.leaf_span.end),
                }
            ));
        }
        for a in &flow.attribute_loads {
            out.attribute_loads.push(fact_row!(
                sink,
                FlowAttributeLoads,
                provenance(),
                FlowAttributeLoadsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: module,
                    start_byte: i64::from(a.span.start),
                    end_byte: i64::from(a.span.end),
                    name: a.name.clone(),
                }
            ));
        }
        for r in &flow.regions {
            let condition_id = condition(&r.condition);
            let (scope_kind, scope_start_byte, scope_end_byte) = scope(r.scope);
            out.regions.push(fact_row!(
                sink,
                FlowRegions,
                provenance(),
                FlowRegionsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: module,
                    scope_kind,
                    scope_start_byte,
                    scope_end_byte,
                    start_byte: i64::from(r.span.start),
                    end_byte: i64::from(r.span.end),
                    condition_id,
                    approximated: r.condition.approximated(),
                }
            ));
        }
    }
    let mut nodes: BTreeMap<Id, DiagramNode> = BTreeMap::new();
    for (condition_id, c) in conditions {
        let (root_id, encoding, display_truncated, boundary_reason) = match c.diagram() {
            Ok(diagram) => {
                let (root, closure) = diagram.root_and_nodes();
                for node in closure {
                    nodes.entry(node.node_id).or_insert(node);
                }
                let (encoding, truncated) = match diagram.render_terms(16) {
                    Ok(rendered) => {
                        let text = if rendered.terms.is_empty() {
                            "false".to_owned()
                        } else {
                            rendered
                                .terms
                                .iter()
                                .map(|term| {
                                    if term.is_empty() {
                                        "true".to_owned()
                                    } else {
                                        term.iter()
                                            .map(|(atom, positive)| {
                                                format!(
                                                    "{}{}",
                                                    if *positive { "" } else { "!" },
                                                    atom
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join(" & ")
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" | ")
                        };
                        (text, rendered.truncated)
                    }
                    Err(_) => ("<render budget>".to_owned(), true),
                };
                (Some(root), encoding, truncated, None)
            }
            Err(reason) => (None, c.encode(), false, Some(reason.code().to_owned())),
        };
        out.conditions.push(fact_row!(
            sink,
            Conditions,
            provenance(),
            ConditionsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                condition_id,
                root_id,
                encoding,
                stated: root_id.is_some(),
                display_truncated,
                boundary_reason,
            }
        ));
        if let Condition::Dnf(conjunctions) = c.legacy() {
            for (ci, conjunction) in conjunctions.iter().enumerate() {
                for (li, l) in conjunction.iter().enumerate() {
                    out.literals.push(fact_row!(
                        sink,
                        ConditionLiterals,
                        provenance(),
                        ConditionLiteralsRow {
                            snapshot_id: Id::ZERO,
                            fact_id: Id::ZERO,
                            condition_id,
                            conjunction: ci as i64,
                            ordinal: li as i64,
                            atom: l.atom.kind(),
                            positive: l.positive,
                            place: l.atom.place().map(str::to_owned),
                            argument: l.atom.argument(),
                        }
                    ));
                }
            }
        }
    }
    for node in nodes.into_values() {
        out.condition_nodes.push(fact_row!(
            sink,
            ConditionNodes,
            provenance(),
            ConditionNodesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id: node.node_id,
                atom: node.atom,
                low_id: node.low,
                high_id: node.high,
            }
        ));
    }
    out
}
