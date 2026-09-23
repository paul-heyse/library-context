//! Pass B (DESIGN §9.2): controls and local restrictions, a bounded worklist over the declared
//! argument flows and parameter guards (`cpg_schema::flows`).
//!
//! From each of the seed's parameters, the worklist follows flows whose value is that parameter
//! (directly, or through one identity alias) into callees inside the subsystem, keyed by
//! `(callable, formal, source parameter)`, up to the depth bound. It reports:
//! - `forwarding`: a seed parameter reaches a callee's formal, with the call chain as witness;
//! - `transformed_argument`: the seed supplies a callee's formal with a literal;
//! - `conditional_raise`: a reached callable raises in the branch of an `if` testing the formal
//!   the seed's parameter reaches, unless a call on the way sits inside a `try` of its caller,
//!   where a handler may catch it.
//!
//! Every finding is `structurally_observed`: it states what the code does, never a public
//! precondition (Stage F says "the implementation raises").

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use arrow_array::{
    Array, BooleanArray, FixedSizeBinaryArray, Int16Array, RecordBatch, StringArray,
};
use cpg_schema::codebook::{
    ArcKind, Codebook, CoverageStatus, FindingKind, InvocationPhase, MemberRole, Modality,
    StopReason,
};
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, StepKey, WitnessesRow,
    recipe::FindingKey,
};
use cpg_schema::flows::value_class;
use cpg_schema::id::Id;

use crate::AnalyticsError;

/// One argument flow (a row of `cpg_schema::flows::argument_flows_sql`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub caller: Id,
    pub call_site: Id,
    pub target: Id,
    pub edge_id: Id,
    pub modality: Modality,
    pub phase: InvocationPhase,
    pub formal: Id,
    pub formal_name: String,
    pub value_class: i16,
    pub source_parameter: Option<Id>,
    pub alias_name: Option<String>,
    pub value_text: Option<String>,
    pub in_try: bool,
}

/// One parameter guard (a row of `cpg_schema::flows::guards_sql`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard {
    pub function: Id,
    pub test: Id,
    pub raise: Id,
    pub parameter: Id,
}

/// The flows and guards, indexed by caller and by function.
#[derive(Debug, Default)]
pub struct Flows {
    pub flows: Vec<Flow>,
    pub guards: Vec<Guard>,
    by_caller: BTreeMap<Id, Vec<usize>>,
    guards_of: BTreeMap<(Id, Id), Vec<usize>>,
}

fn col<'a, T: 'static>(b: &'a RecordBatch, name: &str) -> Result<&'a T, AnalyticsError> {
    b.column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<T>())
        .ok_or_else(|| AnalyticsError::Graph(format!("column {name} has the wrong type")))
}

fn id_at(a: &FixedSizeBinaryArray, i: usize) -> Option<Id> {
    (!a.is_null(i)).then(|| Id(<[u8; 16]>::try_from(a.value(i)).expect("16 bytes")))
}

fn text_at(a: &StringArray, i: usize) -> Option<String> {
    (!a.is_null(i)).then(|| a.value(i).to_owned())
}

impl Flows {
    /// Index the declared relations' batches (already in their total order).
    pub fn build(flows: &[RecordBatch], guards: &[RecordBatch]) -> Result<Self, AnalyticsError> {
        let mut out = Flows::default();
        for b in flows {
            let ids = |n| col::<FixedSizeBinaryArray>(b, n);
            let (caller, site, target, edge, formal, source) = (
                ids("caller_node_id")?,
                ids("call_site_node_id")?,
                ids("target_node_id")?,
                ids("edge_id")?,
                ids("formal_node_id")?,
                ids("source_parameter_node_id")?,
            );
            let modality = col::<Int16Array>(b, "modality")?;
            let phase = col::<Int16Array>(b, "phase")?;
            let class = col::<Int16Array>(b, "value_class")?;
            let formal_name = col::<StringArray>(b, "formal_name")?;
            let alias = col::<StringArray>(b, "alias_name")?;
            let value = col::<StringArray>(b, "value_text")?;
            let in_try = col::<BooleanArray>(b, "in_try")?;
            for i in 0..b.num_rows() {
                let bad = |what: &str| AnalyticsError::Graph(format!("a flow's {what}"));
                out.flows.push(Flow {
                    caller: id_at(caller, i).ok_or_else(|| bad("caller"))?,
                    call_site: id_at(site, i).ok_or_else(|| bad("call site"))?,
                    target: id_at(target, i).ok_or_else(|| bad("target"))?,
                    edge_id: id_at(edge, i).ok_or_else(|| bad("edge"))?,
                    modality: Modality::from_code(modality.value(i))
                        .ok_or_else(|| bad("modality"))?,
                    phase: InvocationPhase::from_code(phase.value(i))
                        .ok_or_else(|| bad("phase"))?,
                    formal: id_at(formal, i).ok_or_else(|| bad("formal"))?,
                    formal_name: formal_name.value(i).to_owned(),
                    value_class: class.value(i),
                    source_parameter: id_at(source, i),
                    alias_name: text_at(alias, i),
                    value_text: text_at(value, i),
                    in_try: in_try.value(i),
                });
            }
        }
        for b in guards {
            let ids = |n| col::<FixedSizeBinaryArray>(b, n);
            let (function, test, raise, parameter) = (
                ids("function_node_id")?,
                ids("test_node_id")?,
                ids("raise_node_id")?,
                ids("parameter_node_id")?,
            );
            for i in 0..b.num_rows() {
                let bad = || AnalyticsError::Graph("a guard's ids".to_owned());
                out.guards.push(Guard {
                    function: id_at(function, i).ok_or_else(bad)?,
                    test: id_at(test, i).ok_or_else(bad)?,
                    raise: id_at(raise, i).ok_or_else(bad)?,
                    parameter: id_at(parameter, i).ok_or_else(bad)?,
                });
            }
        }
        for (i, f) in out.flows.iter().enumerate() {
            out.by_caller.entry(f.caller).or_default().push(i);
        }
        for (i, g) in out.guards.iter().enumerate() {
            out.guards_of
                .entry((g.function, g.parameter))
                .or_default()
                .push(i);
        }
        Ok(out)
    }
}

/// A seed parameter the worklist starts from (the receiver aside).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedParameter {
    pub node: Id,
    pub name: String,
}

/// One seed's Pass B results.
#[derive(Debug, Clone, PartialEq)]
pub struct PassBResult {
    pub completion: CoverageStatus,
    pub stop_reason: Option<StopReason>,
    pub states_examined: i64,
    pub flows_examined: i64,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
    pub witnesses: Vec<WitnessesRow>,
}

struct Draft {
    kind: FindingKind,
    related: Id,
    condition: Option<Id>,
    path: Vec<usize>,
    members: Vec<(MemberRole, Option<Id>, String)>,
}

/// Run Pass B from one seed. `inside` says whether a callable is in the subsystem.
pub fn run(
    flows: &Flows,
    seed: Id,
    parameters: &[SeedParameter],
    inside: impl Fn(Id) -> bool,
    max_depth: u32,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<PassBResult, AnalyticsError> {
    let name_of: BTreeMap<Id, &str> = parameters
        .iter()
        .map(|p| (p.node, p.name.as_str()))
        .collect();
    // (callable, formal, source parameter) → the flow rows that reached it.
    let mut visited: BTreeMap<(Id, Id, Id), Vec<usize>> = BTreeMap::new();
    let mut queue: VecDeque<(Id, Id, Id)> = VecDeque::new();
    for p in parameters {
        visited.insert((seed, p.node, p.node), Vec::new());
        queue.push_back((seed, p.node, p.node));
    }
    let mut drafts: Vec<Draft> = Vec::new();
    let mut depth_limited = false;
    let mut flows_examined = 0i64;
    while let Some(state) = queue.pop_front() {
        let (callable, formal, source) = state;
        let path = visited[&state].clone();
        for &i in flows
            .by_caller
            .get(&callable)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let f = &flows.flows[i];
            if f.source_parameter != Some(formal)
                || !matches!(f.value_class, value_class::PARAMETER | value_class::ALIAS)
                || !inside(f.target)
            {
                continue;
            }
            flows_examined += 1;
            if path.len() as u32 >= max_depth {
                depth_limited = true;
                continue;
            }
            let next = (f.target, f.formal, source);
            if visited.contains_key(&next) {
                continue;
            }
            let mut to = path.clone();
            to.push(i);
            let mut members = vec![(
                MemberRole::SourceParameter,
                Some(source),
                name_of[&source].to_owned(),
            )];
            for &j in &to {
                if let Some(alias) = &flows.flows[j].alias_name {
                    members.push((MemberRole::Alias, None, alias.clone()));
                }
            }
            drafts.push(Draft {
                kind: FindingKind::Forwarding,
                related: f.formal,
                condition: None,
                path: to.clone(),
                members,
            });
            visited.insert(next, to);
            queue.push_back(next);
        }
    }
    // Literals the seed itself supplies to a callee's formal.
    let mut literals: BTreeSet<(Id, String)> = BTreeSet::new();
    for &i in flows
        .by_caller
        .get(&seed)
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let f = &flows.flows[i];
        if f.value_class == value_class::LITERAL
            && inside(f.target)
            && let Some(v) = &f.value_text
            && literals.insert((f.formal, v.clone()))
        {
            drafts.push(Draft {
                kind: FindingKind::TransformedArgument,
                related: f.formal,
                condition: None,
                path: vec![i],
                members: vec![(MemberRole::Value, None, v.clone())],
            });
        }
    }
    // Guards on every reached formal, unless a call on the way may be caught.
    let formal_name: BTreeMap<Id, &str> = flows
        .flows
        .iter()
        .map(|f| (f.formal, f.formal_name.as_str()))
        .collect();
    let mut guarded: BTreeSet<(Id, Id, Id)> = BTreeSet::new();
    for (&(callable, formal, source), path) in &visited {
        if path.iter().any(|&i| flows.flows[i].in_try) {
            continue;
        }
        for &g in flows
            .guards_of
            .get(&(callable, formal))
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let guard = &flows.guards[g];
            if guarded.insert((guard.raise, guard.test, source)) {
                let mut members = vec![(
                    MemberRole::SourceParameter,
                    Some(source),
                    name_of[&source].to_owned(),
                )];
                if !path.is_empty() {
                    members.push((
                        MemberRole::Formal,
                        Some(formal),
                        formal_name
                            .get(&formal)
                            .copied()
                            .unwrap_or_default()
                            .to_owned(),
                    ));
                }
                drafts.push(Draft {
                    kind: FindingKind::ConditionalRaise,
                    related: guard.raise,
                    condition: Some(guard.test),
                    path: path.clone(),
                    members,
                });
            }
        }
    }

    let mut findings = Vec::new();
    let mut members = Vec::new();
    let mut witnesses = Vec::new();
    for d in drafts {
        let steps: Vec<StepKey> = d
            .path
            .iter()
            .map(|&i| {
                let f = &flows.flows[i];
                StepKey {
                    call_site: f.call_site,
                    callee: f.target,
                    modality: f.modality.code(),
                    arc_kind: ArcKind::Call.code(),
                    phase: Some(f.phase.code()),
                }
            })
            .collect();
        let paths = if steps.is_empty() {
            Vec::new()
        } else {
            vec![steps]
        };
        let member_keys: Vec<MemberKey> = d
            .members
            .iter()
            .enumerate()
            .map(|(ordinal, (role, node, label))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: *node,
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let status = FINDING_STATUS
            .iter()
            .find(|(k, _)| *k == d.kind)
            .map(|(_, s)| *s)
            .ok_or_else(|| AnalyticsError::Graph(format!("no status policy for {:?}", d.kind)))?;
        let depth = Some(d.path.len() as i64);
        let finding_id = FindingKey {
            finding_kind: d.kind.code(),
            subject: seed,
            related: Some(d.related),
            condition: d.condition,
            evidence_status: status.code(),
            depth,
            stop_reason: None,
            witnesses_omitted: false,
            paths: &paths,
            members: &member_keys,
        }
        .id();
        for (ordinal, (role, node, label)) in d.members.iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role: *role,
                ordinal: ordinal as i64,
                node_id: *node,
                cited_fact_id: None,
                label: Some(label.clone()),
                weight: None,
            });
        }
        for (step, &i) in d.path.iter().enumerate() {
            let f = &flows.flows[i];
            witnesses.push(WitnessesRow {
                snapshot_id,
                finding_id,
                path: 0,
                step: step as i64,
                caller_node_id: f.caller,
                call_site_node_id: f.call_site,
                callee_node_id: f.target,
                edge_id: f.edge_id,
                modality: f.modality,
                arc_kind: ArcKind::Call,
                phase: Some(f.phase),
            });
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: d.kind,
            subject_node_id: seed,
            related_node_id: Some(d.related),
            evidence_status: status,
            depth,
            stop_reason: None,
            witnesses_omitted: false,
            score: None,
            condition_node_id: d.condition,
        });
    }
    Ok(PassBResult {
        completion: CoverageStatus::CompleteUnderStatedModel,
        stop_reason: depth_limited.then_some(StopReason::DepthLimit),
        states_examined: visited.len() as i64,
        flows_examined,
        findings,
        members,
        witnesses,
    })
}
