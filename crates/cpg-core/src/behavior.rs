//! Stage 1 of the behavior model (ADR-0021, ADR-0022; DESIGN §3.2, §9's opening; the plan's Stage
//! 1): the whole public surface, not the seeds. Runs after `public_paths` is written and before
//! Stage E.
//!
//! 1. Pass B's and C's declared relations are persisted (`cpg_schema::behavior`), with every
//!    public callable's depth-1 call arcs (`delegations`).
//! 2. `operations`: every public node at its preferred path.
//! 3. Pass B's worklist runs from **every public callable** (`pass_b_surface`, one invocation),
//!    into any release function. Its findings become `behaviors` rows, never brief findings.
//!    Delegations and official-usage handoffs become rows too. Each row carries a verdict (§3.9).
//! 4. `operation_facets` for `find_operations`: FCA's declared attributes of each callable
//!    (`concepts::attributes_sql`, the one authority for them), its kind, module and async, and
//!    whom it delegates to, forwards to and hands off to.
//! 5. `operation_documents`: each callable's signature-and-docstring view and its source-body
//!    view, cut into windows at line ends, embedded through the cache when an embedder is set.

use std::collections::{BTreeMap, BTreeSet};

use cpg_schema::behavior::{
    self as b, ArgumentFlows, ArgumentFlowsRow, BehaviorStepsRow, BehaviorsRow, DelegationsRow,
    Guards, GuardsRow, HandoffsRow, OperationDocumentsRow, OperationFacetStatusRow,
    OperationFacetsRow, OperationSourceRow, OperationsRow, ParameterReads, ParameterReadsRow,
};
use cpg_schema::codebook::{
    AnalyticMethod, BehaviorKind, BoundaryReason, Codebook, CoverageStatus, DeclarationKind,
    ExtractionMode, FindingKind, FlowSink, FlowTransfer, MemberRole, Modality,
    ValueClass, Verdict,
};
use cpg_schema::condition_kernel::{BoundedCondition as ModelCondition, KernelBoundary};
use cpg_schema::findings::{
    AnalysisInvocationsRow, FindingMembersRow, recipe as findings,
};
use cpg_schema::id::{Digest, Id, content_digest};
use cpg_schema::table::Table;
use datafusion::prelude::SessionContext;
use lctx_analytics::pass_b::{self, Flows};
use serde::Serialize;

use crate::CoreError;
use crate::analyze::{Analysis, CompilerRun};
use crate::flow_model::{ModelRaiseGuards, normal_path_model};
use crate::sql;
use cpg_schema::metrics::Stages;

/// The largest window of an embedded view, in bytes (§9.7's E0 windows).
pub const WINDOW_BYTES: usize = 4096;

/// What Stage 1 writes.
#[derive(Debug, Default)]
pub struct BehaviorRows {
    pub argument_flows: Vec<ArgumentFlowsRow>,
    /// Discharge evidence of graded call-transfer claims (ADR-0064).
    pub discharges: Vec<cpg_schema::behavior::BehaviorDischargesRow>,
    pub guards: Vec<GuardsRow>,
    pub parameter_reads: Vec<ParameterReadsRow>,
    pub handoffs: Vec<HandoffsRow>,
    pub delegations: Vec<DelegationsRow>,
    pub operations: Vec<OperationsRow>,
    pub facets: Vec<OperationFacetsRow>,
    pub facet_status: Vec<OperationFacetStatusRow>,
    pub behaviors: Vec<BehaviorsRow>,
    pub steps: Vec<BehaviorStepsRow>,
    pub documents: Vec<OperationDocumentsRow>,
    pub invocations: Vec<AnalysisInvocationsRow>,
}

/// A handoff pair's formal name, first occurrence (path, byte, site) and occurrence count.
struct Handed {
    formal_name: String,
    first: (String, i64, Id),
    occurrences: i64,
}

#[derive(Serialize)]
struct SurfaceParameters<'a> {
    scope: &'a str,
    max_depth: u32,
    public_roots: &'a [String],
    window_bytes: usize,
}

cpg_schema::query_row! {
    struct QualifiedRow {
        node_id: Id,
        qualified_name: String,
    }
}

cpg_schema::query_row! {
    struct TextRow {
        module_node_id: Id,
        text: Option<String>,
    }
}

cpg_schema::relations! {
    inventory relations;
    /// Declarations' qualified names (`$ids`).
    qualified_names = "behavior_qualified_names",
        deps = ["declarations"],
        sql = "SELECT node_id, qualified_name FROM declarations \
               WHERE array_has($ids, node_id) ORDER BY node_id"
            .to_owned();
    /// Modules' texts (`$ids`), release and usage alike.
    module_texts = "behavior_module_texts",
        deps = ["source_files"],
        sql = "SELECT DISTINCT module_node_id, text FROM source_files \
               WHERE array_has($ids, module_node_id) ORDER BY module_node_id"
            .to_owned();
    /// Syntax nodes' spans (`$ids`).
    site_spans = "behavior_site_spans",
        deps = ["syntax_nodes"],
        sql = "SELECT DISTINCT node_id, module_node_id, start_byte, end_byte FROM syntax_nodes \
               WHERE array_has($ids, node_id) ORDER BY node_id"
            .to_owned();
}

cpg_schema::query_row! {
    struct SpanRow {
        node_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
}

/// The 1-based line of a byte offset in `text`.
fn line_of(text: &str, byte: usize) -> i64 {
    1 + text.as_bytes()[..byte.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as i64
}

/// The boundary a hop through a non-definite arc meets (increment 3's deep review, F1).
fn hop_reason(modality: Modality) -> Option<BoundaryReason> {
    match modality {
        Modality::Definite => None,
        Modality::Candidate => Some(BoundaryReason::OverrideDispatch),
        Modality::Potential => Some(BoundaryReason::AmbiguousBinding),
    }
}

/// The order a scan's boundaries are named in: the first is `operations.boundary_reason`.
fn reason_rank(r: BoundaryReason) -> u8 {
    match r {
        BoundaryReason::BudgetReached => 0,
        BoundaryReason::OverrideDispatch => 1,
        BoundaryReason::AmbiguousBinding => 2,
        BoundaryReason::UnresolvedTarget => 3,
        BoundaryReason::OutsideProviderModel => 4,
        _ => 5,
    }
}

/// One step of a behavior's path before its id is known.
#[derive(Clone)]
struct Hop {
    caller: Id,
    call_site: Id,
    callee: Id,
    modality: Modality,
    conditional: bool,
    condition: Option<String>,
}

/// Per function, each guard that raises: its start byte and the negation of its condition (the
/// function's normal path past it).
/// A hop's condition, when its flow is one the flow IR attributes (`true` is none).
fn hop_condition(flows: &Flows, conditions: &HopConditions, index: usize) -> Option<String> {
    let f = flows.flows.get(index)?;
    conditions
        .get(&(f.call_site, f.formal, f.source_parameter?))
        .filter(|c| c.as_str() != "true")
        .cloned()
}

/// The condition a hop's value reaches its callee under, by (call site, formal, source parameter).
type HopConditions = BTreeMap<(Id, Id, Id), String>;

/// Stage 2's argument flows: each Stage 1 arc row, its value's source parameters replaced by the
/// flow IR's (identity rows are followed, derived rows are `other`); a read the flow IR attributes
/// is no longer "unfollowed".
fn v2_flows(
    stage1: &[ArgumentFlowsRow],
    reads: &[ParameterReadsRow],
    value_flows: &[cpg_schema::behavior::ValueFlowsRow],
    stated: &dyn Fn(Id, Id) -> Option<String>,
    exact_condition: &dyn Fn(Id) -> bool,
    feasible_call: &dyn Fn(Id) -> bool,
) -> (Vec<ArgumentFlowsRow>, Vec<ParameterReadsRow>, HopConditions) {
    let mut sources: BTreeMap<Id, Vec<(Id, &cpg_schema::behavior::ValueFlowsRow)>> =
        BTreeMap::new();
    for v in value_flows {
        if let (Some(a), Some(p)) = (v.argument_node_id, v.parameter_node_id) {
            sources.entry(a).or_default().push((p, v));
        }
    }
    let mut rows = Vec::new();
    let mut conditions = HopConditions::new();
    for r in stage1 {
        if !feasible_call(r.call_site_node_id) {
            continue;
        }
        let Some(found) = sources.get(&r.argument_node_id) else {
            rows.push(r.clone());
            continue;
        };
        // Pass B only follows unchanged parameter values. Keep one unfollowed alternative
        // per source; the full derived/call distinction remains in value_flows and behaviors.
        let mut emitted = BTreeSet::new();
        for &(p, v) in found {
            // The flow IR's condition on the caller's normal path decides whether the call
            // passes the value only on some paths (it replaces Stage 1's syntactic flag).
            let condition = stated(r.caller_node_id, v.condition_id);
            let mut row = r.clone();
            row.source_parameter_node_id = Some(p);
            row.alias_name = None;
            let follows =
                v.identity && !v.captured && !v.approximated && exact_condition(v.condition_id);
            row.value_class = if follows {
                ValueClass::Parameter
            } else {
                ValueClass::Other
            };
            row.conditional = follows && condition.is_some();
            if follows && let Some(c) = condition {
                conditions.insert((r.call_site_node_id, r.formal_node_id, p), c);
            }
            if emitted.insert((p, row.value_class)) {
                rows.push(row);
            }
        }
    }
    let attributed: BTreeSet<(Id, Id)> = value_flows
        .iter()
        .filter_map(|v| Some((v.argument_node_id?, v.parameter_node_id?)))
        .collect();
    let reads = reads
        .iter()
        .filter(|r| feasible_call(r.call_site_node_id))
        .filter(|r| !attributed.contains(&(r.argument_node_id, r.parameter_node_id)))
        .cloned()
        .collect();
    (rows, reads, conditions)
}

/// The digest of what the behavior scan reads: its declared relations and this module's own.
pub fn digest() -> Digest {
    let mut h = cpg_schema::id::IdHasher::new("behavior-scan");
    h.digest_field(b::digest());
    h.digest_field(crate::flow_model::digest());
    for r in relations() {
        h.str(r.name).str(&r.sql);
    }
    h.finish_digest()
}

/// Stage 1 for one attempt.
#[expect(
    clippy::too_many_arguments,
    reason = "the attempt's context, the analysis, the compiler run, and the two inputs it reads"
)]
pub async fn run(
    ctx: &SessionContext,
    embeddings: &mut crate::embed::Session,
    snapshot_id: Id,
    analysis: &Analysis,
    compiler: CompilerRun,
    catalog: &crate::catalog::Contracts,
    flow: &crate::flow_model::FlowModelRows,
    discharges: &lctx_analytics::summaries::discharge::Decisions,
    stages: &mut Stages,
) -> Result<BehaviorRows, CoreError> {
    let mut out = BehaviorRows {
        argument_flows: sql::fetch(ctx, &b::argument_flows(), sql::Params::new()).await?,
        guards: sql::fetch(ctx, &b::guards(), sql::Params::new()).await?,
        parameter_reads: sql::fetch(ctx, &b::parameter_reads(), sql::Params::new()).await?,
        handoffs: sql::fetch(ctx, &b::handoffs(), sql::Params::new()).await?,
        delegations: sql::fetch(ctx, &b::delegations(), sql::Params::new()).await?,
        ..Default::default()
    };
    stages.mark("behavior: relations");

    // Operations: every public node at its preferred path.
    let sources: Vec<OperationSourceRow> =
        sql::fetch(ctx, &b::operation_sources(), sql::Params::new()).await?;
    let callables: Vec<Id> = sources
        .iter()
        .filter(|s| s.kind != DeclarationKind::Class)
        .map(|s| s.node_id)
        .collect();
    let path_of: BTreeMap<Id, &str> = sources
        .iter()
        .map(|s| (s.node_id, s.access_path.as_str()))
        .collect();

    // Pass B from every public callable, into any release function (the flows hold only arcs
    // into release functions). Stage 2: an argument's source parameters are the flow IR's
    // (`value_flows`), each identity or derived under its condition, so a rebound fallback is
    // followed; Stage 1's value classes stand only where the flow IR names no source.
    let mut guards = ModelRaiseGuards::new();
    for r in flow.raise_sites.iter().filter(|r| r.escapes) {
        if let Some(c) = flow.condition_models.get(&r.condition_id)
            && c.diagram().is_ok()
            && !c.is_always()
        {
            guards
                .entry(r.function_node_id)
                .or_default()
                .push((r.start_byte, c.not()));
        }
    }
    // A stated condition on its function's normal path, or none for `true`.
    let stated = |function: Id, id: Id| -> Option<String> {
        let c = flow.condition_models.get(&id)?;
        let c = normal_path_model(&guards, function, c, None);
        (!c.is_always()).then(|| c.encode())
    };
    let exact_condition = |id: Id| {
        flow.condition_models.get(&id).is_some_and(|condition| {
            condition.diagram().is_ok() && !condition.approximated() && !condition.is_never()
        })
    };
    // Raw provider call/read rows remain evidence. Only an exact false runtime condition
    // excludes a derived claim; missing and bounded conditions remain open alternatives.
    let feasible_call = |site: Id| {
        !flow
            .call_condition_ids
            .get(&site)
            .and_then(|id| flow.condition_models.get(id))
            .is_some_and(ModelCondition::is_never)
    };
    let (flow_rows, read_rows, hop_conditions) = v2_flows(
        &out.argument_flows,
        &out.parameter_reads,
        &flow.value_flows,
        &stated,
        &exact_condition,
        &feasible_call,
    );
    let flows = Flows::build(
        &[ArgumentFlows::to_sorted_batch(&flow_rows)?],
        &[Guards::to_sorted_batch(&out.guards)?],
        &[ParameterReads::to_sorted_batch(&read_rows)?],
    )
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let parameters_of = crate::analyze::seed_parameters(ctx, &callables).await?;
    let max_depth = analysis.config.pass_a.max_depth;
    let parameters = serde_json::to_string(&SurfaceParameters {
        scope: "public_paths",
        max_depth,
        public_roots: &analysis.config.subsystem.public_roots,
        window_bytes: WINDOW_BYTES,
    })
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let parameters_digest = content_digest(parameters.as_bytes());
    let scan_digest = digest();
    let invocation_id = findings::invocation(
        AnalyticMethod::PassBSurface.code(),
        parameters_digest,
        Some(scan_digest),
        None,
        None,
    );
    let formal_name: BTreeMap<Id, &str> = out
        .argument_flows
        .iter()
        .map(|f| (f.formal_node_id, f.formal_name.as_str()))
        .collect();
    let mut partial = BTreeSet::new();
    // Every boundary each operation's scan met, for its status (increment 3's deep review, F2).
    let mut boundaries: BTreeMap<Id, BTreeSet<(u8, BoundaryReason, String)>> = BTreeMap::new();
    let mut meet = |op: Id, reason: BoundaryReason, text: String| {
        boundaries
            .entry(op)
            .or_default()
            .insert((reason_rank(reason), reason, text));
    };
    for &op in &callables {
        if flow.decorated.contains(&op) {
            meet(
                op,
                BoundaryReason::OutsideProviderModel,
                "a decorator may replace the callable binding".to_owned(),
            );
        }
    }
    // The reads each (callable, formal) makes: a frontier state with any is a depth cut.
    let mut reads_at: BTreeSet<(Id, Id)> = out
        .parameter_reads
        .iter()
        .map(|r| (r.caller_node_id, r.parameter_node_id))
        .collect();
    reads_at.extend(
        out.argument_flows
            .iter()
            .filter_map(|f| f.source_parameter_node_id.map(|p| (f.caller_node_id, p))),
    );
    let open_reads: BTreeSet<(Id, Id)> =
        sql::fetch::<b::OpenSiteReadRow>(ctx, &b::open_site_reads(), sql::Params::new())
            .await?
            .into_iter()
            .map(|r| (r.caller_node_id, r.parameter_node_id))
            .collect();
    let mut hops: BTreeMap<Id, Vec<Hop>> = BTreeMap::new();
    let (mut states, mut examined) = (0i64, 0i64);
    for &node in &callables {
        let own_parameters = parameters_of
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let result = pass_b::run(
            &flows,
            node,
            own_parameters,
            |_| true,
            max_depth,
            snapshot_id,
            invocation_id,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        states += result.states_examined;
        examined += result.flows_examined;
        if result.completion != CoverageStatus::CompleteUnderStatedModel
            || result.stop_reason.is_some()
        {
            partial.insert(node);
            meet(
                node,
                BoundaryReason::BudgetReached,
                format!("the scan stopped at the depth bound ({max_depth})"),
            );
        }
        let mut members: BTreeMap<Id, Vec<&FindingMembersRow>> = BTreeMap::new();
        for m in &result.members {
            members.entry(m.finding_id).or_default().push(m);
        }
        // Each finding's first witness path, hop by hop with its modality (review F6).
        let mut paths: BTreeMap<Id, Vec<(i64, Hop)>> = BTreeMap::new();
        for w in result.witnesses.iter().filter(|w| w.path == 0) {
            paths.entry(w.finding_id).or_default().push((
                w.step,
                Hop {
                    caller: w.caller_node_id,
                    call_site: w.call_site_node_id,
                    callee: w.callee_node_id,
                    modality: w.modality,
                    conditional: false,
                    condition: None,
                },
            ));
        }
        // The values the scan tracks: the operation's parameters and every formal it reached.
        let mut tracked: BTreeSet<(Id, Id)> =
            own_parameters.iter().map(|p| (node, p.node)).collect();
        for f in &result.findings {
            if f.finding_kind == FindingKind::Forwarding
                && let (Some(callee), Some(formal)) = (
                    paths
                        .get(&f.finding_id)
                        .and_then(|p| p.iter().max_by_key(|(s, _)| *s))
                        .map(|(_, h)| h.callee),
                    f.related_node_id,
                )
            {
                tracked.insert((callee, formal));
                if f.depth.unwrap_or(0) >= i64::from(max_depth)
                    && reads_at.contains(&(callee, formal))
                {
                    meet(
                        node,
                        BoundaryReason::BudgetReached,
                        format!("a reached formal is read past the depth bound ({max_depth})"),
                    );
                }
            }
        }
        for &(caller, formal) in &tracked {
            if caller != node && open_reads.contains(&(caller, formal)) {
                meet(
                    node,
                    BoundaryReason::UnresolvedTarget,
                    "a tracked value reaches a call site resolution leaves open".to_owned(),
                );
            }
        }
        for f in &result.findings {
            let ms = members
                .get(&f.finding_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let member = |role: MemberRole| ms.iter().find(|m| m.role == role);
            let source = member(MemberRole::SourceParameter);
            let conditional_sites: BTreeSet<Id> = ms
                .iter()
                .filter(|m| m.role == MemberRole::ConditionalCall)
                .filter_map(|m| m.node_id)
                .collect();
            let conditional = !conditional_sites.is_empty();
            let mut path: Vec<Hop> = paths
                .get(&f.finding_id)
                .map(|p| {
                    let mut p = p.clone();
                    p.sort_by_key(|(s, _)| *s);
                    p.into_iter().map(|(_, h)| h).collect()
                })
                .unwrap_or_default();
            let flow_path = result
                .flow_paths
                .get(&f.finding_id)
                .cloned()
                .unwrap_or_default();
            for (k, h) in path.iter_mut().enumerate() {
                h.conditional = conditional_sites.contains(&h.call_site);
                // The flow-to-hop condition was already factored by the diagram kernel
                // when `v2_flows` built this path.
                h.condition = flow_path
                    .get(k)
                    .and_then(|&i| hop_condition(&flows, &hop_conditions, i));
            }
            // A raise directly in the operation is Stage 2's raise sites' (the flow IR's region
            // conditions), not a syntactic guard.
            if f.finding_kind == FindingKind::ConditionalRaise && path.is_empty() {
                continue;
            }
            let row_condition = path.first().and_then(|h| h.condition.clone());
            let conditional = conditional || path.iter().any(|h| h.condition.is_some());
            let (callee, site) = path
                .last()
                .map(|h| (Some(h.callee), Some(h.call_site)))
                .unwrap_or((Some(node), None));
            // One verdict policy per arc (review F1): a hop through a non-definite arc makes the
            // row unknown, as the delegation over it is.
            let hop = path.iter().find_map(|h| hop_reason(h.modality));
            let positive = |c: bool| match (hop, c) {
                (Some(_), _) => Verdict::Unknown,
                (None, true) => Verdict::Conditional,
                (None, false) => Verdict::Established,
            };
            let (kind, verdict, reason, target_name, value, site) = match f.finding_kind {
                FindingKind::Forwarding => (
                    BehaviorKind::Forwards,
                    positive(conditional),
                    hop,
                    f.related_node_id
                        .and_then(|n| formal_name.get(&n))
                        .map(|s| (*s).to_owned()),
                    None,
                    site,
                ),
                FindingKind::TransformedArgument => (
                    BehaviorKind::SuppliesLiteral,
                    positive(conditional),
                    hop,
                    f.related_node_id
                        .and_then(|n| formal_name.get(&n))
                        .map(|s| (*s).to_owned()),
                    member(MemberRole::Value).and_then(|m| m.label.clone()),
                    site,
                ),
                FindingKind::ConditionalRaise => (
                    BehaviorKind::RaisesWhen,
                    positive(true),
                    hop,
                    member(MemberRole::Formal).and_then(|m| m.label.clone()),
                    None,
                    f.condition_node_id,
                ),
                FindingKind::UnfollowedArgument => (
                    BehaviorKind::Unfollowed,
                    Verdict::Unknown,
                    hop.or(Some(BoundaryReason::OutsideProviderModel)),
                    member(MemberRole::Formal).and_then(|m| m.label.clone()),
                    member(MemberRole::Reason).and_then(|m| m.label.clone()),
                    site,
                ),
                _ => continue,
            };
            if let Some(r) = reason {
                let text = match r {
                    BoundaryReason::OverrideDispatch => {
                        "a path crosses an override-open call".to_owned()
                    }
                    BoundaryReason::AmbiguousBinding => {
                        "a path crosses a potential call".to_owned()
                    }
                    _ => "a parameter is read in a form the scan does not follow".to_owned(),
                };
                meet(node, r, text);
            }
            let parameter = source.and_then(|m| m.node_id);
            let target = f.related_node_id;
            let behavior_id = b::behavior_id(
                node,
                kind,
                parameter,
                callee,
                target,
                value.as_deref(),
                site,
            );
            let mut row = BehaviorsRow {
                snapshot_id,
                behavior_id,
                operation_node_id: node,
                kind,
                transfer: (kind == BehaviorKind::Forwards).then_some(FlowTransfer::Identity),
                condition_scope_node_id: node,
                parameter_node_id: parameter,
                parameter_name: source.and_then(|m| m.label.clone()),
                callee_node_id: callee,
                target_node_id: target,
                target_name,
                value,
                depth: f.depth.unwrap_or(0),
                conditional,
                verdict,
                boundary_reason: reason,
                condition: row_condition,
                callee_text: None,
                phase: None,
                premise_key: None,
                site_node_id: site,
                site_module_node_id: None,
                site_start_byte: None,
                site_end_byte: None,
                site_line: None,
                site_text: None,
                occurrences: 1,
                invocation_id: Some(invocation_id),
            };
            if row.transfer.is_some() {
                row.behavior_id = b::flow_behavior_id(&row);
            }
            hops.insert(row.behavior_id, path);
            out.behaviors.push(row);
        }
    }
    out.invocations.push(AnalysisInvocationsRow {
        snapshot_id,
        invocation_id,
        run_id: compiler.run_id,
        model_id: compiler.model("pass-b-surface"),
        extraction_mode: ExtractionMode::GraphAnalysis,
        method: AnalyticMethod::PassBSurface,
        parameters,
        parameters_digest,
        projection_digest: Some(scan_digest),
        library_versions: lctx_analytics::libraries(),
        subject_node_id: None,
        seed: None,
        iterations: None,
        residual: None,
        converged: None,
        quality_history: Vec::new(),
        candidate_set_size: Some(callables.len() as i64),
        vertices_examined: Some(states),
        arcs_examined: Some(examined),
        completion: if partial.is_empty() {
            CoverageStatus::CompleteUnderStatedModel
        } else {
            CoverageStatus::Partial
        },
        stop_reason: (!partial.is_empty()).then_some(cpg_schema::codebook::StopReason::DepthLimit),
        diagnostics: None,
    });
    stages.mark("behavior: Pass B from every public callable");

    // Delegations: one row per (operation, callee, modality), its first call site the witness.
    let mut delegated: BTreeMap<(Id, Id, Modality), (Id, i64)> = BTreeMap::new();
    for d in &out.delegations {
        if !feasible_call(d.call_site_node_id) {
            continue;
        }
        let e = delegated
            .entry((d.caller_node_id, d.target_node_id, d.modality))
            .or_insert((d.call_site_node_id, 0));
        e.0 = e.0.min(d.call_site_node_id);
        e.1 += 1;
    }
    for (&(op, callee, modality), &(site, n)) in &delegated {
        let reason = hop_reason(modality);
        let condition = flow
            .call_condition_ids
            .get(&site)
            .and_then(|id| stated(op, *id));
        let (verdict, value) = match (reason, &condition) {
            (None, None) => (Verdict::Established, None),
            (None, Some(_)) => (Verdict::Conditional, None),
            (Some(_), _) => (Verdict::Unknown, Some(modality.text().to_owned())),
        };
        if let Some(r) = reason {
            meet(
                op,
                r,
                "it makes an override-open or potential call".to_owned(),
            );
        }
        let behavior_id = b::behavior_id(
            op,
            BehaviorKind::Delegates,
            None,
            Some(callee),
            None,
            value.as_deref(),
            Some(site),
        );
        hops.insert(
            behavior_id,
            vec![Hop {
                caller: op,
                call_site: site,
                callee,
                modality,
                conditional: false,
                condition: None,
            }],
        );
        out.behaviors.push(BehaviorsRow {
            snapshot_id,
            behavior_id,
            operation_node_id: op,
            kind: BehaviorKind::Delegates,
            transfer: None,
            condition_scope_node_id: op,
            parameter_node_id: None,
            parameter_name: None,
            callee_node_id: Some(callee),
            target_node_id: None,
            target_name: None,
            value,
            depth: 1,
            conditional: condition.is_some(),
            verdict,
            boundary_reason: reason,
            condition,
            callee_text: None,
            phase: None,
            premise_key: None,
            site_node_id: Some(site),
            site_module_node_id: None,
            site_start_byte: None,
            site_end_byte: None,
            site_line: None,
            site_text: None,
            occurrences: n,
            invocation_id: None,
        });
    }

    // Handoffs in official usage, for public operations on either side.
    let mut handed: BTreeMap<(Id, Id, Id), Handed> = BTreeMap::new();
    for h in &out.handoffs {
        let at = (
            h.path.clone(),
            h.consumer_start_byte,
            h.consumer_site_node_id,
        );
        let e = handed
            .entry((h.consumer_node_id, h.producer_node_id, h.formal_node_id))
            .or_insert_with(|| Handed {
                formal_name: h.formal_name.clone(),
                first: at.clone(),
                occurrences: 0,
            });
        if at < e.first {
            e.first = at;
        }
        e.occurrences += 1;
    }
    for (&(consumer, producer, formal), h) in &handed {
        let (name, site, n) = (&h.formal_name, &h.first.2, &h.occurrences);
        for (op, kind, partner) in [
            (consumer, BehaviorKind::TakesFrom, producer),
            (producer, BehaviorKind::HandsOffTo, consumer),
        ] {
            if !path_of.contains_key(&op) {
                continue;
            }
            out.behaviors.push(BehaviorsRow {
                snapshot_id,
                behavior_id: b::behavior_id(
                    op,
                    kind,
                    None,
                    Some(partner),
                    Some(formal),
                    None,
                    Some(*site),
                ),
                operation_node_id: op,
                kind,
                transfer: None,
                condition_scope_node_id: op,
                parameter_node_id: None,
                parameter_name: None,
                callee_node_id: Some(partner),
                target_node_id: Some(formal),
                target_name: Some(name.clone()),
                value: None,
                depth: 0,
                conditional: false,
                verdict: Verdict::Established,
                boundary_reason: None,
                condition: None,
                callee_text: None,
                phase: None,
                premise_key: None,
                site_node_id: Some(*site),
                site_module_node_id: None,
                site_start_byte: None,
                site_end_byte: None,
                site_line: None,
                site_text: None,
                occurrences: *n,
                invocation_id: None,
            });
        }
    }
    // Stage 2 (ADR-0022): what the flow IR says about each public callable's own body. Rows
    // with one id are one claim: their conditions are joined by `or`.
    let own: BTreeSet<Id> = callables.iter().copied().collect();
    let mut release_targets: BTreeMap<Id, BTreeSet<(Id, Id, String, Modality)>> = BTreeMap::new();
    for r in &out.argument_flows {
        release_targets
            .entry(r.argument_node_id)
            .or_default()
            .insert((
                r.target_node_id,
                r.formal_node_id,
                r.formal_name.clone(),
                r.modality,
            ));
    }
    let release_target: BTreeMap<_, _> = release_targets
        .into_iter()
        .filter_map(|(argument, targets)| {
            (targets.len() == 1)
                .then(|| (argument, targets.into_iter().next().expect("one target")))
        })
        .collect();
    // A call site's one release callee, for an argument no formal receives (`*args`, `**kwargs`).
    let mut site_callees: BTreeMap<Id, BTreeSet<(Id, Modality)>> = BTreeMap::new();
    for d in &out.delegations {
        site_callees
            .entry(d.call_site_node_id)
            .or_default()
            .insert((d.target_node_id, d.modality));
    }
    let site_target: BTreeMap<Id, (Id, Modality)> = site_callees
        .into_iter()
        .filter_map(|(site, ts)| (ts.len() == 1).then(|| (site, *ts.first().expect("one"))))
        .collect();
    let name_of_parameter: BTreeMap<Id, (Id, String)> = parameters_of
        .iter()
        .flat_map(|(op, ps)| ps.iter().map(move |p| (p.node, (*op, p.name.clone()))))
        .collect();
    let mut stage2: BTreeMap<Id, (BehaviorsRow, ModelCondition, Id)> = BTreeMap::new();
    fn claim(
        stage2: &mut BTreeMap<Id, (BehaviorsRow, ModelCondition, Id)>,
        mut row: BehaviorsRow,
        condition: &ModelCondition,
        scope: Id,
    ) -> Id {
        let c = condition.clone();
        row.condition_scope_node_id = scope;
        row.behavior_id = b::flow_behavior_id(&row);
        let id = row.behavior_id;
        match stage2.get_mut(&row.behavior_id) {
            Some((_, e, _)) => *e = e.or(&c),
            None => {
                stage2.insert(row.behavior_id, (row, c, scope));
            }
        }
        id
    }
    // A graded call-transfer claim's member origins (the flow model's merge, accumulated over
    // every merged row) and whether each merged row was open only because of the call
    // (ADR-0064). The grade is applied after the merge, so it does not depend on row order.
    let mut discharge_members: BTreeMap<Id, (BTreeSet<Id>, bool)> = BTreeMap::new();
    let blank = |op: Id, kind: BehaviorKind| BehaviorsRow {
        snapshot_id,
        behavior_id: Id::ZERO,
        operation_node_id: op,
        kind,
        transfer: None,
        condition_scope_node_id: op,
        parameter_node_id: None,
        parameter_name: None,
        callee_node_id: None,
        target_node_id: None,
        target_name: None,
        value: None,
        depth: 0,
        conditional: false,
        verdict: Verdict::Established,
        boundary_reason: None,
        condition: None,
        callee_text: None,
        phase: None,
        premise_key: None,
        site_node_id: None,
        site_module_node_id: None,
        site_start_byte: None,
        site_end_byte: None,
        site_line: None,
        site_text: None,
        occurrences: 1,
        invocation_id: Some(invocation_id),
    };
    for (index, v) in flow
        .value_flows
        .iter()
        .enumerate()
        .filter(|(_, v)| own.contains(&v.function_node_id) && v.parameter_node_id.is_some())
    {
        let site_at = |row: &mut BehaviorsRow| {
            row.site_module_node_id = Some(v.module_node_id);
            row.site_start_byte = Some(v.sink_start_byte);
            row.site_end_byte = Some(v.sink_end_byte);
        };
        let mut row = blank(v.function_node_id, BehaviorKind::Forwards);
        row.transfer = Some(FlowTransfer::of(v.identity, v.through_call));
        row.parameter_node_id = v.parameter_node_id;
        row.parameter_name = Some(v.source_name.clone());
        row.depth = 1;
        match v.sink {
            FlowSink::Argument => {
                let target = v.argument_node_id.and_then(|a| release_target.get(&a));
                let text = v
                    .call_site_node_id
                    .and_then(|c| flow.callee_text.get(&c))
                    .cloned();
                if v.identity
                    && !v.captured
                    && !v.approximated
                    && exact_condition(v.condition_id)
                    && target.is_some()
                {
                    // A release callee: Pass B's forward, with its condition.
                    continue;
                }
                row.kind = if v.identity {
                    BehaviorKind::Forwards
                } else {
                    BehaviorKind::Derives
                };
                if let Some((t, formal_id, formal, modality)) = target {
                    row.callee_node_id = Some(*t);
                    row.target_node_id = Some(*formal_id);
                    row.target_name = Some(formal.clone());
                    if let Some(reason) = hop_reason(*modality) {
                        row.verdict = Verdict::Unknown;
                        row.boundary_reason = Some(reason);
                    }
                } else if let Some(&(t, modality)) =
                    v.call_site_node_id.and_then(|c| site_target.get(&c))
                {
                    // A release callee no formal of which receives the argument (`**kwargs`);
                    // the call's text keeps the claim apart from a mapped one at the same site.
                    row.callee_node_id = Some(t);
                    row.value = text;
                    if let Some(reason) = hop_reason(modality) {
                        row.verdict = Verdict::Unknown;
                        row.boundary_reason = Some(reason);
                    }
                } else {
                    row.callee_text = text.clone();
                    row.value = text;
                }
                row.site_node_id = v.call_site_node_id;
                site_at(&mut row);
            }
            FlowSink::Definition => {
                row.kind = BehaviorKind::Stores;
                row.target_name = v.place.clone();
                row.value = Some(if v.identity { "unchanged" } else { "computed" }.to_owned());
                site_at(&mut row);
                row.value = Some(format!(
                    "{}:{}",
                    row.value.take().unwrap_or_default(),
                    v.sink_start_byte
                ));
            }
            FlowSink::Return | FlowSink::Yield => {
                row.kind = BehaviorKind::Returns;
                row.value = Some(if v.identity { "unchanged" } else { "computed" }.to_owned());
                row.depth = 0;
                site_at(&mut row);
                row.value = Some(format!(
                    "{}:{}",
                    row.value.take().unwrap_or_default(),
                    v.sink_start_byte
                ));
            }
            FlowSink::Raise => continue,
        }
        if v.through_call {
            // Only inside a call: whether its result carries the value is Stage 3's summaries.
            row.verdict = Verdict::Unknown;
            row.boundary_reason = Some(BoundaryReason::CallTransfer);
        }
        if v.captured {
            row.verdict = Verdict::Unknown;
            row.boundary_reason = Some(BoundaryReason::ScopeBoundary);
        }
        if v.approximated && row.verdict != Verdict::Unknown {
            row.verdict = Verdict::Unknown;
            row.boundary_reason = Some(BoundaryReason::MissingEvidence);
        }
        let condition = flow
            .condition_models
            .get(&v.condition_id)
            .cloned()
            .unwrap_or_else(|| ModelCondition::unknown(KernelBoundary::SourceOverBudget));
        // P1 discharges return claims only (`caller_return_summary`, ADR-0064).
        let graded = row.kind == BehaviorKind::Returns && v.through_call;
        let call_only = row.boundary_reason == Some(BoundaryReason::CallTransfer);
        let behavior_id = claim(&mut stage2, row, &condition, v.function_node_id);
        if graded {
            let (members, only) = discharge_members
                .entry(behavior_id)
                .or_insert_with(|| (BTreeSet::new(), true));
            members.extend(flow.value_flow_members[index].iter().copied());
            *only &= call_only;
        }
    }
    // Only a raise that may leave its function is a `raises_when` fate (ADR-0022 §Conditions).
    for r in flow
        .raise_sites
        .iter()
        .filter(|r| own.contains(&r.function_node_id) && r.escapes)
    {
        for name in &r.parameters {
            let Some(p) = parameters_of
                .get(&r.function_node_id)
                .and_then(|ps| ps.iter().find(|p| &p.name == name))
            else {
                continue;
            };
            let mut row = blank(r.function_node_id, BehaviorKind::RaisesWhen);
            row.parameter_node_id = Some(p.node);
            row.parameter_name = Some(name.clone());
            row.value = Some(format!("{}:{}", r.text, r.start_byte));
            row.site_module_node_id = Some(r.module_node_id);
            row.site_start_byte = Some(r.start_byte);
            row.site_end_byte = Some(r.end_byte);
            row.site_line = Some(r.line);
            row.site_text = Some(r.text.clone());
            let condition = flow
                .condition_models
                .get(&r.condition_id)
                .cloned()
                .unwrap_or_else(|| ModelCondition::unknown(KernelBoundary::SourceOverBudget));
            claim(&mut stage2, row, &condition, r.function_node_id);
        }
    }
    for a in flow.ambient_reads.iter() {
        let Some(reader) = a.reader_node_id.filter(|r| own.contains(r)) else {
            continue;
        };
        let mut row = blank(reader, BehaviorKind::ReadsSetting);
        row.target_name = Some(format!("{}.{}", a.global, a.field));
        row.value = Some(format!("{}:{}", a.spelled, a.start_byte));
        row.phase = Some(a.phase);
        row.site_module_node_id = Some(a.module_node_id);
        row.site_start_byte = Some(a.start_byte);
        row.site_end_byte = Some(a.end_byte);
        row.site_line = Some(a.line);
        row.site_text = Some(a.spelled.clone());
        let condition = flow
            .condition_models
            .get(&a.condition_id)
            .cloned()
            .unwrap_or_else(|| ModelCondition::unknown(KernelBoundary::SourceOverBudget));
        claim(&mut stage2, row, &condition, reader);
    }
    for p in flow
        .premises
        .iter()
        .filter(|p| p.kind == cpg_schema::codebook::PremiseKind::Parameter)
    {
        let Some((op, name)) = p.subject_node_id.and_then(|n| name_of_parameter.get(&n)) else {
            continue;
        };
        if !own.contains(op) {
            continue;
        }
        let mut row = blank(*op, BehaviorKind::IsRead);
        row.parameter_node_id = p.subject_node_id;
        row.parameter_name = Some(name.clone());
        row.premise_key = Some(p.place_key.clone());
        row.verdict = if p.holds {
            Verdict::RefutedUnderModel
        } else {
            Verdict::Unknown
        };
        row.boundary_reason = if p.holds {
            None
        } else {
            p.boundary_reason.or(Some(BoundaryReason::MissingEvidence))
        };
        row.value = p.reason.clone();
        claim(&mut stage2, row, &ModelCondition::always(), *op);
    }
    // A parameter stored to its receiver's field, then read by any method of a relative: the
    // value's fate there, stated under the read's own condition (in that method's places).
    let mut field_origins: BTreeMap<(Id, String), Vec<&cpg_schema::behavior::ValueFlowsRow>> =
        BTreeMap::new();
    for v in &flow.value_flows {
        if let (Some(c), None) = (v.class_node_id, v.parameter_node_id) {
            field_origins
                .entry((c, v.source_name.clone()))
                .or_default()
                .push(v);
        }
    }
    // Only a value stored unchanged composes with its later reads: a computed store (an object
    // built from many parameters) would attribute every read of the object to each of them.
    let stores: Vec<BehaviorsRow> = stage2
        .values()
        .filter(|(r, condition, _)| {
            r.kind == BehaviorKind::Stores
                && r.transfer == Some(FlowTransfer::Identity)
                && r.verdict == Verdict::Established
                && condition.is_always()
                && !condition.approximated()
                && !flow.decorated.contains(&r.operation_node_id)
        })
        .map(|(r, _, _)| r.clone())
        .collect();
    for store in stores {
        let (Some(class), Some(place)) = (
            flow.method_class.get(&store.operation_node_id),
            store.target_name.as_deref(),
        ) else {
            continue;
        };
        let Some((_, field)) = place.split_once('.') else {
            continue;
        };
        if field.contains('.') || field.contains('[') {
            continue;
        }
        let family: BTreeSet<Id> = std::iter::once(*class)
            .chain(flow.relatives.get(class).into_iter().flatten().copied())
            .collect();
        for c in &family {
            for v in field_origins
                .get(&(*c, field.to_owned()))
                .into_iter()
                .flatten()
            {
                let via = flow
                    .qualified
                    .get(&v.function_node_id)
                    .cloned()
                    .unwrap_or_default();
                let mut row = store.clone();
                row.transfer = Some(FlowTransfer::of(v.identity, v.through_call));
                row.depth = 2;
                row.site_node_id = None;
                row.site_module_node_id = Some(v.module_node_id);
                row.site_start_byte = Some(v.sink_start_byte);
                row.site_end_byte = Some(v.sink_end_byte);
                row.site_line = None;
                row.site_text = None;
                row.target_name = None;
                row.callee_node_id = None;
                row.callee_text = None;
                match v.sink {
                    FlowSink::Argument => {
                        let target = v.argument_node_id.and_then(|a| release_target.get(&a));
                        row.kind = if v.identity {
                            BehaviorKind::Forwards
                        } else {
                            BehaviorKind::Derives
                        };
                        if let Some((t, formal_id, formal, modality)) = target {
                            row.callee_node_id = Some(*t);
                            row.target_node_id = Some(*formal_id);
                            row.target_name = Some(formal.clone());
                            if let Some(reason) = hop_reason(*modality) {
                                row.verdict = Verdict::Unknown;
                                row.boundary_reason = Some(reason);
                            }
                        } else if let Some(&(t, modality)) =
                            v.call_site_node_id.and_then(|c| site_target.get(&c))
                        {
                            row.callee_node_id = Some(t);
                            if let Some(reason) = hop_reason(modality) {
                                row.verdict = Verdict::Unknown;
                                row.boundary_reason = Some(reason);
                            }
                        } else {
                            row.callee_text = v
                                .call_site_node_id
                                .and_then(|c| flow.callee_text.get(&c))
                                .cloned();
                        }
                    }
                    FlowSink::Return | FlowSink::Yield => row.kind = BehaviorKind::Returns,
                    FlowSink::Definition => {
                        row.kind = BehaviorKind::Stores;
                        row.target_name = v.place.clone();
                    }
                    FlowSink::Raise => row.kind = BehaviorKind::RaisesWhen,
                }
                row.value = Some(format!(
                    "via {place} in {via}{}:{}",
                    row.callee_text
                        .as_deref()
                        .map(|t| format!(", into {t}"))
                        .unwrap_or_default(),
                    v.sink_start_byte
                ));
                if v.through_call {
                    row.verdict = Verdict::Unknown;
                    row.boundary_reason = Some(BoundaryReason::CallTransfer);
                }
                if v.captured {
                    row.verdict = Verdict::Unknown;
                    row.boundary_reason = Some(BoundaryReason::ScopeBoundary);
                }
                if v.approximated && row.verdict != Verdict::Unknown {
                    row.verdict = Verdict::Unknown;
                    row.boundary_reason = Some(BoundaryReason::MissingEvidence);
                }
                let condition = flow
                    .condition_models
                    .get(&v.condition_id)
                    .cloned()
                    .unwrap_or_else(|| ModelCondition::unknown(KernelBoundary::SourceOverBudget));
                claim(&mut stage2, row, &condition, v.function_node_id);
            }
        }
    }
    // A parameter that decides a branch: the literals its function's statements test over it.
    for &op in &own {
        for p in parameters_of.get(&op).into_iter().flatten() {
            if let Some(literals) = flow.tested.get(&(op, p.name.clone())) {
                let mut row = blank(op, BehaviorKind::Tests);
                row.parameter_node_id = Some(p.node);
                row.parameter_name = Some(p.name.clone());
                row.value = Some(literals.iter().cloned().collect::<Vec<_>>().join(" ; "));
                claim(&mut stage2, row, &ModelCondition::always(), op);
            }
        }
    }
    for (_, (mut row, condition, scope)) in stage2 {
        let condition = normal_path_model(&guards, scope, &condition, row.site_start_byte);
        if condition.is_never() {
            continue;
        }
        if let Some((members, call_only)) = discharge_members.get(&row.behavior_id) {
            let (proved, evidence) = lctx_analytics::summaries::discharge::grade(
                snapshot_id,
                row.behavior_id,
                members,
                discharges,
            );
            out.discharges.extend(evidence);
            // The members' proofs discharge the call and their certified value approximation;
            // the condition, capture, decoration and reachability rules below still apply.
            if proved && *call_only {
                row.verdict = Verdict::Established;
                row.boundary_reason = None;
            }
        }
        match condition.diagram() {
            Ok(_) if condition.is_always() => {}
            Err(_) => {
                if row.verdict == Verdict::Established {
                    row.verdict = Verdict::Unknown;
                    row.boundary_reason = Some(BoundaryReason::BudgetReached);
                }
            }
            Ok(_) => {
                // An `unknown` claim keeps the condition it would hold under.
                if row.verdict == Verdict::Established {
                    row.verdict = Verdict::Conditional;
                }
                row.conditional = true;
                row.condition = Some(condition.encode());
            }
        }
        if condition.approximated() && row.verdict != Verdict::Unknown {
            row.verdict = Verdict::Unknown;
            row.boundary_reason = Some(BoundaryReason::MissingEvidence);
        }
        if row.depth == 1
            && row.condition_scope_node_id == row.operation_node_id
            && let (Some(site), Some(callee)) = (row.site_node_id, row.callee_node_id)
            && let Some(delegation) = out.delegations.iter().find(|d| {
                d.caller_node_id == row.operation_node_id
                    && d.call_site_node_id == site
                    && d.target_node_id == callee
            })
        {
            hops.insert(
                row.behavior_id,
                vec![Hop {
                    caller: row.operation_node_id,
                    call_site: site,
                    callee,
                    modality: delegation.modality,
                    conditional: row.conditional,
                    condition: row.condition.clone(),
                }],
            );
        }
        out.behaviors.push(row);
    }

    // A behavior id names one claim; two rows under one id are a defect, never merged (§3.4.1;
    // increment 3's deep review, F8).
    // An operation the runtime cannot reach states nothing it does (ADR-0022 §Composed layers).
    for r in &mut out.behaviors {
        if flow.unreachable.contains(&r.operation_node_id) {
            r.verdict = Verdict::Unknown;
            r.boundary_reason = Some(BoundaryReason::RuntimeUnreachable);
        }
        let decorated_path = hops.get(&r.behavior_id).into_iter().flatten().any(|hop| {
            flow.decorated.contains(&hop.caller) || flow.decorated.contains(&hop.callee)
        });
        if flow.decorated.contains(&r.operation_node_id)
            || flow.decorated.contains(&r.condition_scope_node_id)
            || decorated_path
        {
            if r.verdict != Verdict::Unknown {
                r.verdict = Verdict::Unknown;
                r.boundary_reason = Some(BoundaryReason::OutsideProviderModel);
            }
            meet(
                r.operation_node_id,
                BoundaryReason::OutsideProviderModel,
                "a decorator may replace the callable binding".to_owned(),
            );
        }
    }
    out.behaviors.sort_by_key(|r| r.behavior_id);
    if let Some(w) = out
        .behaviors
        .windows(2)
        .find(|w| w[0].behavior_id == w[1].behavior_id)
    {
        return Err(CoreError::Analysis(format!(
            "two behaviors share the id {}",
            w[0].behavior_id.hex()
        )));
    }
    for r in &out.behaviors {
        for (step, h) in hops.get(&r.behavior_id).into_iter().flatten().enumerate() {
            out.steps.push(BehaviorStepsRow {
                snapshot_id,
                behavior_id: r.behavior_id,
                step: step as i64,
                caller_node_id: h.caller,
                call_site_node_id: h.call_site,
                callee_node_id: h.callee,
                modality: h.modality,
                conditional: h.conditional,
                condition: h.condition.clone(),
            });
        }
    }

    // Where each behavior is shown: its site's module, span, line and verbatim text.
    let sites: BTreeSet<Id> = out
        .behaviors
        .iter()
        .filter_map(|r| r.site_node_id)
        .collect();
    let spans: BTreeMap<Id, SpanRow> = sql::fetch::<SpanRow>(
        ctx,
        &site_spans(),
        sql::Params::new().ids("ids", sites.iter().copied()),
    )
    .await?
    .into_iter()
    .map(|r| (r.node_id, r))
    .collect();
    let site_modules: BTreeSet<Id> = spans.values().map(|r| r.module_node_id).collect();
    let site_texts: BTreeMap<Id, String> = sql::fetch::<TextRow>(
        ctx,
        &module_texts(),
        sql::Params::new().ids("ids", site_modules.iter().copied()),
    )
    .await?
    .into_iter()
    .filter_map(|r| r.text.map(|t| (r.module_node_id, t)))
    .collect();
    let direct_modules: BTreeSet<Id> = out
        .behaviors
        .iter()
        .filter(|r| r.site_line.is_none())
        .filter_map(|r| r.site_module_node_id)
        .collect();
    let direct_texts: BTreeMap<Id, String> = sql::fetch::<TextRow>(
        ctx,
        &module_texts(),
        sql::Params::new().ids("ids", direct_modules.iter().copied()),
    )
    .await?
    .into_iter()
    .filter_map(|r| r.text.map(|t| (r.module_node_id, t)))
    .collect();
    for r in &mut out.behaviors {
        if r.site_line.is_none()
            && let (Some(m), Some(s)) = (r.site_module_node_id, r.site_start_byte)
            && let Some(text) = direct_texts.get(&m)
        {
            r.site_line = Some(line_of(text, s as usize));
            if let Some(e) = r.site_end_byte {
                r.site_text = text.get(s as usize..e as usize).map(str::to_owned);
            }
        }
        let Some(span) = r.site_node_id.and_then(|s| spans.get(&s)) else {
            continue;
        };
        r.site_module_node_id = Some(span.module_node_id);
        r.site_start_byte = Some(span.start_byte);
        r.site_end_byte = Some(span.end_byte);
        if let Some(text) = site_texts.get(&span.module_node_id) {
            r.site_line = Some(line_of(text, span.start_byte as usize));
            r.site_text = text
                .get(span.start_byte as usize..span.end_byte as usize)
                .map(str::to_owned);
        }
    }

    // Operations: established only when the scan met no boundary in its region (increment 3's
    // deep review, F2); its own open call sites count too.
    let open: BTreeMap<Id, i64> =
        sql::fetch::<b::OpenSitesRow>(ctx, &b::open_sites(), sql::Params::new())
            .await?
            .into_iter()
            .map(|r| (r.node_id, r.sites))
            .collect();
    for (&node, &n) in &open {
        boundaries.entry(node).or_default().insert((
            reason_rank(BoundaryReason::UnresolvedTarget),
            BoundaryReason::UnresolvedTarget,
            format!("{n} call site(s) resolution leaves open"),
        ));
    }
    for s in &sources {
        let met = boundaries.get(&s.node_id);
        let (status, reason, status_reason) = if s.kind == DeclarationKind::Class {
            (
                Verdict::NotAnalyzed,
                None,
                Some("a class: its controls are its __init__'s".to_owned()),
            )
        } else if flow.unreachable.contains(&s.node_id) {
            (
                Verdict::Unknown,
                Some(BoundaryReason::RuntimeUnreachable),
                Some("its declaration is unreachable at runtime".to_owned()),
            )
        } else if let Some(met) = met.filter(|m| !m.is_empty()) {
            (
                Verdict::Unknown,
                met.first().map(|(_, r, _)| *r),
                Some(
                    met.iter()
                        .map(|(_, _, t)| t.as_str())
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            )
        } else {
            (Verdict::Established, None, None)
        };
        out.operations.push(OperationsRow {
            snapshot_id,
            node_id: s.node_id,
            access_path: s.access_path.clone(),
            kind: s.kind,
            is_method: s.is_method,
            qualified_name: s.qualified_name.clone(),
            module: s.module.clone(),
            docstring_summary: s.docstring.as_deref().and_then(b::docstring_summary),
            behavior_status: status,
            boundary_reason: reason,
            status_reason,
        });
    }

    crate::catalog::populate(ctx, embeddings, snapshot_id, analysis.embedder.as_deref(), catalog,
        &mut out, stages).await?;
    Ok(out)
}
