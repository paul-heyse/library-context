//! Pass B (DESIGN §9.2): controls and local restrictions, a bounded worklist over the declared
//! argument flows, parameter guards and parameter reads (`cpg_schema::flows`).
//!
//! From each of the seed's parameters, the worklist follows flows whose value is that parameter
//! (directly, or through one identity alias) into callees inside the subsystem, keyed by
//! `(callable, formal, source parameter, suppression)`, up to the depth bound. It reports:
//! - `forwarding`: a seed parameter reaches a callee's formal, with the call chain as witness;
//! - `transformed_argument`: the seed supplies a callee's formal with a literal;
//! - `conditional_raise`: a reached callable raises in the branch of an `if` testing the formal
//!   the seed's parameter reaches, unless a call on the way sits inside a `try` or `with` of its
//!   caller (a handler or context manager may absorb it) or inside a construct of its caller that
//!   also tests the flowing value (the caller may pass only values the callee accepts);
//! - `unfollowed_argument`: a reached formal's name is read by an argument of a call into the
//!   subsystem in a form the worklist does not follow: after the name is rebound, inside an
//!   expression, or unpacked or taken by no single formal (slice 2.1 review F4).
//!
//! A call on a path that its caller makes only on some paths is a `conditional_call` member, so
//! Stage F qualifies what it says (review F1). Every finding is `structurally_observed`: it states
//! what the code does, never a public precondition (Stage F says "the implementation raises").

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use arrow_array::{
    Array, BooleanArray, FixedSizeBinaryArray, Int16Array, RecordBatch, StringArray,
};
use cpg_schema::codebook::{
    ArcKind, Codebook, CoverageStatus, FindingKind, InvocationPhase, MemberRole, Modality,
    StopReason, UnfollowedReason,
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
    pub argument: Id,
    pub formal: Id,
    pub formal_name: String,
    pub value_class: i16,
    pub source_parameter: Option<Id>,
    pub alias_name: Option<String>,
    pub value_text: Option<String>,
    /// The call site lies inside a `try` or `with` of its caller.
    pub may_catch: bool,
    /// The caller makes the call only on some paths.
    pub conditional: bool,
    /// A construct around the call also reads the flowing value.
    pub value_tested: bool,
}

/// One parameter read (a row of `cpg_schema::flows::parameter_reads_sql`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    pub caller: Id,
    pub call_site: Id,
    pub target: Id,
    pub edge_id: Id,
    pub modality: Modality,
    pub phase: InvocationPhase,
    pub argument: Id,
    pub parameter: Id,
    pub rebound: bool,
    pub bare: bool,
    pub unpacked: bool,
}

impl Read {
    /// Why the worklist does not follow this read.
    pub fn reason(&self) -> UnfollowedReason {
        if self.rebound {
            UnfollowedReason::Rebound
        } else if self.bare || self.unpacked {
            UnfollowedReason::Unmapped
        } else {
            UnfollowedReason::Computed
        }
    }
}

/// One call on a witness path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Step {
    caller: Id,
    call_site: Id,
    callee: Id,
    edge_id: Id,
    modality: Modality,
    phase: InvocationPhase,
    conditional: bool,
}

impl Step {
    fn of_flow(f: &Flow) -> Self {
        Step {
            caller: f.caller,
            call_site: f.call_site,
            callee: f.target,
            edge_id: f.edge_id,
            modality: f.modality,
            phase: f.phase,
            conditional: f.conditional,
        }
    }
}

/// One parameter guard (a row of `cpg_schema::flows::guards_sql`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard {
    pub function: Id,
    pub test: Id,
    pub raise: Id,
    pub parameter: Id,
}

/// The flows, guards and parameter reads, indexed by caller and by function.
#[derive(Debug, Default)]
pub struct Flows {
    pub flows: Vec<Flow>,
    pub guards: Vec<Guard>,
    pub reads: Vec<Read>,
    by_caller: BTreeMap<Id, Vec<usize>>,
    guards_of: BTreeMap<(Id, Id), Vec<usize>>,
    reads_of: BTreeMap<(Id, Id), Vec<usize>>,
    /// (argument, source parameter) of every flow the worklist can follow.
    followed: BTreeSet<(Id, Id)>,
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
    pub fn build(
        flows: &[RecordBatch],
        guards: &[RecordBatch],
        reads: &[RecordBatch],
    ) -> Result<Self, AnalyticsError> {
        let mut out = Flows::default();
        for b in flows {
            let ids = |n| col::<FixedSizeBinaryArray>(b, n);
            let (caller, site, target, edge, argument, formal, source) = (
                ids("caller_node_id")?,
                ids("call_site_node_id")?,
                ids("target_node_id")?,
                ids("edge_id")?,
                ids("argument_node_id")?,
                ids("formal_node_id")?,
                ids("source_parameter_node_id")?,
            );
            let modality = col::<Int16Array>(b, "modality")?;
            let phase = col::<Int16Array>(b, "phase")?;
            let class = col::<Int16Array>(b, "value_class")?;
            let formal_name = col::<StringArray>(b, "formal_name")?;
            let alias = col::<StringArray>(b, "alias_name")?;
            let value = col::<StringArray>(b, "value_text")?;
            let may_catch = col::<BooleanArray>(b, "may_catch")?;
            let conditional = col::<BooleanArray>(b, "conditional")?;
            let value_tested = col::<BooleanArray>(b, "value_tested")?;
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
                    argument: id_at(argument, i).ok_or_else(|| bad("argument"))?,
                    formal: id_at(formal, i).ok_or_else(|| bad("formal"))?,
                    formal_name: formal_name.value(i).to_owned(),
                    value_class: class.value(i),
                    source_parameter: id_at(source, i),
                    alias_name: text_at(alias, i),
                    value_text: text_at(value, i),
                    may_catch: may_catch.value(i),
                    conditional: conditional.value(i),
                    value_tested: value_tested.value(i),
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
        for b in reads {
            let ids = |n| col::<FixedSizeBinaryArray>(b, n);
            let (caller, site, target, edge, argument, parameter) = (
                ids("caller_node_id")?,
                ids("call_site_node_id")?,
                ids("target_node_id")?,
                ids("edge_id")?,
                ids("argument_node_id")?,
                ids("parameter_node_id")?,
            );
            let modality = col::<Int16Array>(b, "modality")?;
            let phase = col::<Int16Array>(b, "phase")?;
            let flag = |n| col::<BooleanArray>(b, n);
            let (rebound, bare, unpacked) = (flag("rebound")?, flag("bare")?, flag("unpacked")?);
            for i in 0..b.num_rows() {
                let bad = |what: &str| AnalyticsError::Graph(format!("a parameter read's {what}"));
                out.reads.push(Read {
                    caller: id_at(caller, i).ok_or_else(|| bad("caller"))?,
                    call_site: id_at(site, i).ok_or_else(|| bad("call site"))?,
                    target: id_at(target, i).ok_or_else(|| bad("target"))?,
                    edge_id: id_at(edge, i).ok_or_else(|| bad("edge"))?,
                    modality: Modality::from_code(modality.value(i))
                        .ok_or_else(|| bad("modality"))?,
                    phase: InvocationPhase::from_code(phase.value(i))
                        .ok_or_else(|| bad("phase"))?,
                    argument: id_at(argument, i).ok_or_else(|| bad("argument"))?,
                    parameter: id_at(parameter, i).ok_or_else(|| bad("parameter"))?,
                    rebound: rebound.value(i),
                    bare: bare.value(i),
                    unpacked: unpacked.value(i),
                });
            }
        }
        for (i, f) in out.flows.iter().enumerate() {
            out.by_caller.entry(f.caller).or_default().push(i);
            if let Some(source) = f.source_parameter
                && matches!(f.value_class, value_class::PARAMETER | value_class::ALIAS)
            {
                out.followed.insert((f.argument, source));
            }
        }
        for (i, r) in out.reads.iter().enumerate() {
            out.reads_of
                .entry((r.caller, r.parameter))
                .or_default()
                .push(i);
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
    /// Each finding's path as indices into [`Flows::flows`] (its witness path's flows, hop by
    /// hop; empty for a read or guard that no flow reaches), for callers that need a hop's
    /// argument and source parameter (the behavior scan's per-hop conditions).
    pub flow_paths: BTreeMap<Id, Vec<usize>>,
}

struct Draft {
    kind: FindingKind,
    related: Id,
    condition: Option<Id>,
    path: Vec<Step>,
    flows: Vec<usize>,
    members: Vec<(MemberRole, Option<Id>, String)>,
}

/// The `conditional_call` members of a path: each call its caller makes only on some paths.
fn conditional_members(path: &[Step]) -> Vec<(MemberRole, Option<Id>, String)> {
    path.iter()
        .filter(|s| s.conditional)
        .map(|s| {
            (
                MemberRole::ConditionalCall,
                Some(s.call_site),
                "conditional".to_owned(),
            )
        })
        .collect()
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
    // Suppressed and clean paths to the same formal are distinct states: only the latter may
    // support a conditional-raise finding. A suppressed path arriving first must not hide it.
    let mut visited: BTreeMap<(Id, Id, Id, bool), Vec<usize>> = BTreeMap::new();
    let mut queue: VecDeque<(Id, Id, Id, bool)> = VecDeque::new();
    for p in parameters {
        visited.insert((seed, p.node, p.node, false), Vec::new());
        queue.push_back((seed, p.node, p.node, false));
    }
    let steps_of = |path: &[usize]| -> Vec<Step> {
        path.iter()
            .map(|&i| Step::of_flow(&flows.flows[i]))
            .collect()
    };
    let source_member = |source: Id| {
        (
            MemberRole::SourceParameter,
            Some(source),
            name_of[&source].to_owned(),
        )
    };
    let mut drafts: Vec<Draft> = Vec::new();
    let mut depth_limited = false;
    let mut flows_examined = 0i64;
    while let Some(state) = queue.pop_front() {
        let (callable, formal, source, suppressed) = state;
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
            let next = (
                f.target,
                f.formal,
                source,
                suppressed || f.may_catch || f.value_tested,
            );
            if visited.contains_key(&next) {
                continue;
            }
            let mut to = path.clone();
            to.push(i);
            let mut members = vec![source_member(source)];
            for &j in &to {
                if let Some(alias) = &flows.flows[j].alias_name {
                    members.push((MemberRole::Alias, None, alias.clone()));
                }
            }
            let steps = steps_of(&to);
            members.extend(conditional_members(&steps));
            drafts.push(Draft {
                kind: FindingKind::Forwarding,
                related: f.formal,
                condition: None,
                path: steps,
                flows: to.clone(),
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
            let steps = vec![Step::of_flow(f)];
            let mut members = vec![(MemberRole::Value, None, v.clone())];
            members.extend(conditional_members(&steps));
            drafts.push(Draft {
                kind: FindingKind::TransformedArgument,
                related: f.formal,
                condition: None,
                path: steps,
                flows: vec![i],
                members,
            });
        }
    }
    // Guards on every reached formal, unless a call on the way may be absorbed, or is made only
    // for values its caller tests.
    let formal_name: BTreeMap<Id, &str> = flows
        .flows
        .iter()
        .map(|f| (f.formal, f.formal_name.as_str()))
        .collect();
    let mut guarded: BTreeSet<(Id, Id, Id)> = BTreeSet::new();
    for (&(callable, formal, source, suppressed), path) in &visited {
        if suppressed {
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
                let mut members = vec![source_member(source)];
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
                let steps = steps_of(path);
                members.extend(conditional_members(&steps));
                drafts.push(Draft {
                    kind: FindingKind::ConditionalRaise,
                    related: guard.raise,
                    condition: Some(guard.test),
                    path: steps,
                    flows: path.clone(),
                    members,
                });
            }
        }
    }
    // Reads of a reached formal the worklist does not follow (review F4), within the depth bound.
    let mut unfollowed: BTreeSet<(Id, Id, Id, UnfollowedReason)> = BTreeSet::new();
    for (&(callable, formal, source, _suppressed), path) in &visited {
        if path.len() as u32 >= max_depth {
            continue;
        }
        for &r in flows
            .reads_of
            .get(&(callable, formal))
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let read = &flows.reads[r];
            if !inside(read.target) || flows.followed.contains(&(read.argument, formal)) {
                continue;
            }
            let reason = read.reason();
            if !unfollowed.insert((read.call_site, read.target, source, reason)) {
                continue;
            }
            let mut steps = steps_of(path);
            steps.push(Step {
                caller: read.caller,
                call_site: read.call_site,
                callee: read.target,
                edge_id: read.edge_id,
                modality: read.modality,
                phase: read.phase,
                conditional: false,
            });
            let mut members = vec![source_member(source)];
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
            members.push((MemberRole::Reason, None, reason.text().to_owned()));
            drafts.push(Draft {
                kind: FindingKind::UnfollowedArgument,
                related: read.target,
                condition: None,
                path: steps,
                flows: path.clone(),
                members,
            });
        }
    }

    let mut findings = Vec::new();
    let mut members = Vec::new();
    let mut witnesses = Vec::new();
    let mut flow_paths = BTreeMap::new();
    for d in drafts {
        let steps: Vec<StepKey> = d
            .path
            .iter()
            .map(|s| StepKey {
                call_site: s.call_site,
                callee: s.callee,
                modality: s.modality.code(),
                arc_kind: ArcKind::Call.code(),
                phase: Some(s.phase.code()),
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
        flow_paths.insert(finding_id, d.flows.clone());
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
        for (step, s) in d.path.iter().enumerate() {
            witnesses.push(WitnessesRow {
                snapshot_id,
                finding_id,
                path: 0,
                step: step as i64,
                caller_node_id: s.caller,
                call_site_node_id: s.call_site,
                callee_node_id: s.callee,
                edge_id: s.edge_id,
                modality: s.modality,
                arc_kind: ArcKind::Call,
                phase: Some(s.phase),
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
        flow_paths,
    })
}

#[cfg(test)]
mod suppression_tests {
    use super::{Flow, Flows, Guard, SeedParameter, run};
    use cpg_schema::codebook::{FindingKind, InvocationPhase, Modality};
    use cpg_schema::flows::value_class;
    use cpg_schema::id::Id;

    fn id(n: u8) -> Id {
        Id([n; 16])
    }

    #[test]
    fn clean_path_found_second_still_supports_guard() {
        let seed = id(1);
        let source = id(2);
        let target = id(3);
        let formal = id(4);
        let make_flow = |site, may_catch| Flow {
            caller: seed,
            call_site: id(site),
            target,
            edge_id: id(site + 10),
            modality: Modality::Definite,
            phase: InvocationPhase::Call,
            argument: id(site + 20),
            formal,
            formal_name: "x".to_owned(),
            value_class: value_class::PARAMETER,
            source_parameter: Some(source),
            alias_name: None,
            value_text: None,
            may_catch,
            conditional: false,
            value_tested: false,
        };
        let mut flows = Flows::default();
        flows.flows = vec![make_flow(5, true), make_flow(6, false)];
        flows.by_caller.insert(seed, vec![0, 1]);
        flows.guards.push(Guard {
            function: target,
            test: id(7),
            raise: id(8),
            parameter: formal,
        });
        flows.guards_of.insert((target, formal), vec![0]);
        let out = run(
            &flows,
            seed,
            &[SeedParameter {
                node: source,
                name: "x".to_owned(),
            }],
            |_| true,
            3,
            Id::ZERO,
            id(9),
        )
        .unwrap();
        let guards: Vec<_> = out
            .findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::ConditionalRaise)
            .collect();
        assert_eq!(guards.len(), 1);
        let witness = out
            .witnesses
            .iter()
            .find(|w| w.finding_id == guards[0].finding_id)
            .unwrap();
        assert_eq!(witness.call_site_node_id, id(6));
    }
}
