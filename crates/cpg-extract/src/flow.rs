//! The `flow` family (ADR-0022 §The flow provider; ADR-0012 amendment): `cpg-flow`'s facts for
//! every UTF-8 release module, from the text Pyrefly parsed, as fact rows. A module ty cannot
//! index says why in its coverage row; the others are unaffected.

use std::collections::BTreeMap;

use cpg_flow::{Input, RuntimeContext, Scope, Sink};
use cpg_schema::codebook::{ExtractionMode, Fidelity, FlowSink, Modality, Origin};
use cpg_schema::condition::Condition;
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::tables::{
    ConditionLiterals, ConditionLiteralsRow, Conditions, ConditionsRow, FlowDefinitions,
    FlowDefinitionsRow, FlowReaching, FlowReachingRow, FlowRegions, FlowRegionsRow, FlowUses,
    FlowUsesRow, FlowValues, FlowValuesRow,
};

use crate::facts::{FactSink, Provenance, Surface, fact_row};

/// One module handed to the provider.
pub(crate) struct FlowModule {
    pub node_id: Id,
    pub path: String,
    pub text: String,
}

#[derive(Default)]
pub(crate) struct FlowOut {
    pub uses: Vec<FlowUsesRow>,
    pub definitions: Vec<FlowDefinitionsRow>,
    pub reaching: Vec<FlowReachingRow>,
    pub values: Vec<FlowValuesRow>,
    pub regions: Vec<FlowRegionsRow>,
    pub conditions: Vec<ConditionsRow>,
    pub literals: Vec<ConditionLiteralsRow>,
    /// Per module, why it has no flow facts (none: indexed).
    pub errors: BTreeMap<Id, String>,
    /// Modules indexed from a recovered tree, with their syntax error counts.
    pub recovered: BTreeMap<Id, usize>,
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
        })
        .collect();
    let flows = cpg_flow::index(&inputs, context);
    let mut out = FlowOut::default();
    let mut conditions: BTreeMap<Id, Condition> = BTreeMap::new();
    let mut condition = |c: &Condition| -> Id {
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
                }
            ));
        }
    }
    for (condition_id, c) in conditions {
        out.conditions.push(fact_row!(
            sink,
            Conditions,
            provenance(),
            ConditionsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                condition_id,
                encoding: c.encode(),
                stated: c != Condition::OverBudget,
            }
        ));
        if let Condition::Dnf(conjunctions) = &c {
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
    out
}
