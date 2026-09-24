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
    EmbeddingView, ExtractionMode, FindingKind, MemberRole, Modality, OperationFacet, Verdict,
};
use cpg_schema::findings::{
    AnalysisInvocationsRow, FindingMembersRow, PublicPathsRow, recipe as findings,
};
use cpg_schema::id::{Digest, Id, content_digest};
use cpg_schema::table::Table;
use datafusion::prelude::SessionContext;
use lctx_analytics::pass_b::{self, Flows};
use serde::Serialize;

use crate::CoreError;
use crate::analyze::{Analysis, CompilerRun};
use crate::sql;
use cpg_schema::metrics::Stages;

/// The largest window of an embedded view, in bytes (§9.7's E0 windows).
pub const WINDOW_BYTES: usize = 4096;

/// What Stage 1 writes.
#[derive(Debug, Default)]
pub struct BehaviorRows {
    pub argument_flows: Vec<ArgumentFlowsRow>,
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
    /// The cache keys the documents use, for the snapshot's key set (`content_digest`).
    pub embedded_keys: Vec<Digest>,
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
#[derive(Clone, Copy)]
struct Hop {
    caller: Id,
    call_site: Id,
    callee: Id,
    modality: Modality,
    conditional: bool,
}

/// The digest of what the behavior scan reads: its declared relations and this module's own.
pub fn digest() -> Digest {
    let mut h = cpg_schema::id::IdHasher::new("behavior-scan");
    h.digest_field(b::digest());
    for r in relations() {
        h.str(r.name).str(&r.sql);
    }
    h.finish_digest()
}

/// Stage 1 for one attempt.
pub async fn run(
    ctx: &SessionContext,
    root: &std::path::Path,
    snapshot_id: Id,
    analysis: &Analysis,
    compiler: CompilerRun,
    public: &[PublicPathsRow],
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
    // into release functions).
    let flows = Flows::build(
        &[ArgumentFlows::to_sorted_batch(&out.argument_flows)?],
        &[Guards::to_sorted_batch(&out.guards)?],
        &[ParameterReads::to_sorted_batch(&out.parameter_reads)?],
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
            for h in &mut path {
                h.conditional = conditional_sites.contains(&h.call_site);
            }
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
            hops.insert(behavior_id, path);
            out.behaviors.push(BehaviorsRow {
                snapshot_id,
                behavior_id,
                operation_node_id: node,
                kind,
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
                site_node_id: site,
                site_module_node_id: None,
                site_start_byte: None,
                site_end_byte: None,
                site_line: None,
                site_text: None,
                occurrences: 1,
                invocation_id: Some(invocation_id),
            });
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
        let e = delegated
            .entry((d.caller_node_id, d.target_node_id, d.modality))
            .or_insert((d.call_site_node_id, 0));
        e.0 = e.0.min(d.call_site_node_id);
        e.1 += 1;
    }
    for (&(op, callee, modality), &(site, n)) in &delegated {
        let reason = hop_reason(modality);
        let (verdict, value) = match reason {
            None => (Verdict::Established, None),
            Some(_) => (Verdict::Unknown, Some(modality.text().to_owned())),
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
            }],
        );
        out.behaviors.push(BehaviorsRow {
            snapshot_id,
            behavior_id,
            operation_node_id: op,
            kind: BehaviorKind::Delegates,
            parameter_node_id: None,
            parameter_name: None,
            callee_node_id: Some(callee),
            target_node_id: None,
            target_name: None,
            value,
            depth: 1,
            conditional: false,
            verdict,
            boundary_reason: reason,
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
    // A behavior id names one claim; two rows under one id are a defect, never merged (§3.4.1;
    // increment 3's deep review, F8).
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
    for r in &mut out.behaviors {
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

    // Names for partners outside the public surface: their qualified names.
    let partners: BTreeSet<Id> = out
        .behaviors
        .iter()
        .filter_map(|r| r.callee_node_id)
        .filter(|n| !path_of.contains_key(n))
        .collect();
    let qualified: BTreeMap<Id, String> = sql::fetch::<QualifiedRow>(
        ctx,
        &qualified_names(),
        sql::Params::new().ids("ids", partners.iter().copied()),
    )
    .await?
    .into_iter()
    .map(|r| (r.node_id, r.qualified_name))
    .collect();
    let name_of = |n: Id| -> Option<String> {
        path_of
            .get(&n)
            .map(|p| (*p).to_owned())
            .or_else(|| qualified.get(&n).cloned())
    };

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

    // Facets, each with the best verdict of what it comes from (increment 3's deep review, F3).
    let mut facets: BTreeMap<(Id, OperationFacet, String), Verdict> = BTreeMap::new();
    let mut put = |node: Id, facet: OperationFacet, value: String, verdict: Verdict| {
        let e = facets.entry((node, facet, value)).or_insert(verdict);
        *e = (*e).min(verdict);
    };
    for s in &sources {
        let kind = match (s.kind, s.is_method) {
            (DeclarationKind::Class, _) => "class",
            (_, true) => "method",
            _ => "function",
        };
        put(
            s.node_id,
            OperationFacet::Kind,
            kind.to_owned(),
            Verdict::Established,
        );
        put(
            s.node_id,
            OperationFacet::Module,
            s.module.clone(),
            Verdict::Established,
        );
        if s.kind == DeclarationKind::AsyncFunction {
            put(
                s.node_id,
                OperationFacet::Async,
                "true".to_owned(),
                Verdict::Established,
            );
        }
        if s.kind == DeclarationKind::Class {
            for d in &s.decorators {
                put(
                    s.node_id,
                    OperationFacet::Decorator,
                    d.clone(),
                    Verdict::Established,
                );
            }
        }
    }
    let attributes = crate::analyze::collect_attributes(ctx, &callables).await?;
    let mut declared: BTreeMap<Id, Vec<(OperationFacet, String)>> = BTreeMap::new();
    for (node, attribute) in attributes {
        let (facet, value) = if let Some(v) = attribute.strip_prefix("parameter type ") {
            (OperationFacet::ParameterType, v)
        } else if let Some(v) = attribute.strip_prefix("parameter ") {
            (OperationFacet::Parameter, v)
        } else if let Some(v) = attribute.strip_prefix("returns ") {
            (OperationFacet::Returns, v)
        } else if let Some(v) = attribute.strip_prefix("raises ") {
            (OperationFacet::Raises, v)
        } else if let Some(v) = attribute.strip_prefix("decorator ") {
            (OperationFacet::Decorator, v)
        } else {
            return Err(CoreError::Analysis(format!(
                "an FCA attribute of no known form: {attribute}"
            )));
        };
        put(node, facet, value.to_owned(), Verdict::Established);
        declared
            .entry(node)
            .or_default()
            .push((facet, value.to_owned()));
    }
    // A class's parameters are its public constructor's: `<class path>.__init__`, its own or
    // inherited through a public path.
    let node_at: BTreeMap<&str, Id> = public
        .iter()
        .map(|p| (p.access_path.as_str(), p.node_id))
        .collect();
    let mut constructor: BTreeMap<Id, Id> = BTreeMap::new();
    for s in sources.iter().filter(|s| s.kind == DeclarationKind::Class) {
        if let Some(&init) = node_at.get(format!("{}.__init__", s.access_path).as_str()) {
            constructor.insert(s.node_id, init);
            for (facet, value) in declared.get(&init).into_iter().flatten() {
                if matches!(
                    facet,
                    OperationFacet::Parameter | OperationFacet::ParameterType
                ) {
                    put(s.node_id, *facet, value.clone(), Verdict::Established);
                }
            }
        }
    }
    for r in &out.behaviors {
        let facet = match r.kind {
            BehaviorKind::Delegates => OperationFacet::DelegatesTo,
            BehaviorKind::Forwards => OperationFacet::ForwardsTo,
            BehaviorKind::HandsOffTo => OperationFacet::HandsOffTo,
            BehaviorKind::TakesFrom => OperationFacet::TakesFrom,
            _ => continue,
        };
        if let Some(name) = r.callee_node_id.and_then(name_of) {
            put(r.operation_node_id, facet, name, r.verdict);
        }
    }
    out.facets = facets
        .into_iter()
        .map(|((node_id, facet, value), verdict)| OperationFacetsRow {
            snapshot_id,
            node_id,
            facet,
            value,
            verdict,
        })
        .collect();

    // Whether each operation's rows for each facet are complete (F3, F4): the served authority
    // for `find_operations`' `complete` and its `unknown` list.
    let status_of: BTreeMap<Id, (Verdict, Option<String>)> = out
        .operations
        .iter()
        .map(|o| (o.node_id, (o.behavior_status, o.status_reason.clone())))
        .collect();
    for s in &sources {
        let class = s.kind == DeclarationKind::Class;
        let (scan, scan_reason) = status_of
            .get(&s.node_id)
            .cloned()
            .unwrap_or((Verdict::NotAnalyzed, None));
        for &facet in <OperationFacet as Codebook>::all() {
            let (verdict, reason) = match facet {
                OperationFacet::Kind
                | OperationFacet::Module
                | OperationFacet::Async
                | OperationFacet::Decorator => (Verdict::Established, None),
                OperationFacet::Parameter | OperationFacet::ParameterType
                    if class && !constructor.contains_key(&s.node_id) =>
                {
                    (
                        Verdict::NotAnalyzed,
                        Some(
                            "no public __init__: a synthesized or unexported constructor"
                                .to_owned(),
                        ),
                    )
                }
                OperationFacet::Parameter | OperationFacet::ParameterType => {
                    (Verdict::Established, None)
                }
                OperationFacet::Returns if class => (
                    Verdict::NotAnalyzed,
                    Some("a class: its constructor returns the instance".to_owned()),
                ),
                OperationFacet::Returns => (Verdict::Established, None),
                OperationFacet::Raises => (
                    Verdict::Unknown,
                    Some("only a typed raise directly in the body is a row".to_owned()),
                ),
                OperationFacet::DelegatesTo | OperationFacet::ForwardsTo => {
                    (scan, scan_reason.clone())
                }
                OperationFacet::HandsOffTo | OperationFacet::TakesFrom => (
                    Verdict::Unknown,
                    Some("official usage is read in two handoff shapes only".to_owned()),
                ),
            };
            out.facet_status.push(OperationFacetStatusRow {
                snapshot_id,
                node_id: s.node_id,
                facet,
                verdict,
                reason,
            });
        }
    }
    stages.mark("behavior: operations and facets");

    // Documents: the signature-and-docstring view and the source-body view of each callable.
    let modules: BTreeSet<Id> = sources.iter().map(|s| s.module_node_id).collect();
    let texts: BTreeMap<Id, String> = sql::fetch::<TextRow>(
        ctx,
        &module_texts(),
        sql::Params::new().ids("ids", modules.iter().copied()),
    )
    .await?
    .into_iter()
    .filter_map(|r| r.text.map(|t| (r.module_node_id, t)))
    .collect();
    for s in sources.iter().filter(|s| s.kind != DeclarationKind::Class) {
        let names: Vec<&str> = parameters_of
            .get(&s.node_id)
            .map(|ps| ps.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();
        let signature = lctx_analytics::neighbours::api_text(
            &s.access_path,
            Some(&names.join(", ")),
            s.docstring.as_deref(),
        );
        let body = texts
            .get(&s.module_node_id)
            .and_then(|t| t.get(s.start_byte as usize..s.end_byte as usize))
            .unwrap_or_default()
            .to_owned();
        for (view, text) in [
            (EmbeddingView::SignatureDoc, signature),
            (EmbeddingView::SourceBody, body),
        ] {
            for (chunk, window) in lctx_analytics::neighbours::windows(&text, WINDOW_BYTES)
                .into_iter()
                .enumerate()
            {
                out.documents.push(OperationDocumentsRow {
                    snapshot_id,
                    node_id: s.node_id,
                    embedding_view: view,
                    chunk: chunk as i64,
                    text: window.to_owned(),
                    spec_hash: None,
                    input_hash: None,
                });
            }
        }
    }
    if let Some(embedder) = analysis.embedder.as_deref() {
        let spec = embedder.spec();
        let spec_hash = spec.hash();
        let texts: Vec<String> = out.documents.iter().map(|d| d.text.clone()).collect();
        let (_, keys, _) = crate::embed::embed_texts(root, snapshot_id, embedder, &texts).await?;
        for d in &mut out.documents {
            d.spec_hash = Some(spec_hash);
            d.input_hash = Some(crate::embed::input_hash(&spec.document_text(&d.text)));
        }
        out.embedded_keys = keys;
        stages.mark("behavior: embed operation documents");
    }
    Ok(out)
}
