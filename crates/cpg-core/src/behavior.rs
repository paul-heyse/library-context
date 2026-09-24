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
    self as b, ArgumentFlows, ArgumentFlowsRow, BehaviorsRow, DelegationsRow, Guards, GuardsRow,
    HandoffsRow, OperationDocumentsRow, OperationFacetsRow, OperationSourceRow, OperationsRow,
    ParameterReads, ParameterReadsRow,
};
use cpg_schema::codebook::{
    AnalyticMethod, BehaviorKind, Codebook, CoverageStatus, DeclarationKind, EmbeddingView,
    ExtractionMode, FindingKind, MemberRole, Modality, OperationFacet, Verdict,
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
    pub behaviors: Vec<BehaviorsRow>,
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
    /// The release modules' texts (`$ids`).
    module_texts = "behavior_module_texts",
        deps = ["source_files"],
        sql = format!(
            "SELECT DISTINCT module_node_id, text FROM source_files \
             WHERE role = {release} AND array_has($ids, module_node_id) ORDER BY module_node_id",
            release = cpg_schema::codebook::SourceRole::Release.code(),
        );
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
    let _ = public;

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
    let (mut states, mut examined) = (0i64, 0i64);
    for &node in &callables {
        let result = pass_b::run(
            &flows,
            node,
            parameters_of
                .get(&node)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            |_| true,
            max_depth,
            snapshot_id,
            invocation_id,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        states += result.states_examined;
        examined += result.flows_examined;
        if result.completion != CoverageStatus::CompleteUnderStatedModel {
            partial.insert(node);
        }
        let mut members: BTreeMap<Id, Vec<&FindingMembersRow>> = BTreeMap::new();
        for m in &result.members {
            members.entry(m.finding_id).or_default().push(m);
        }
        let mut last_step: BTreeMap<Id, (i64, Id, Id)> = BTreeMap::new();
        for w in result.witnesses.iter().filter(|w| w.path == 0) {
            let e = last_step.entry(w.finding_id).or_insert((
                w.step,
                w.callee_node_id,
                w.call_site_node_id,
            ));
            if w.step >= e.0 {
                *e = (w.step, w.callee_node_id, w.call_site_node_id);
            }
        }
        for f in &result.findings {
            let ms = members
                .get(&f.finding_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let member = |role: MemberRole| ms.iter().find(|m| m.role == role);
            let source = member(MemberRole::SourceParameter);
            let conditional = member(MemberRole::ConditionalCall).is_some();
            let (callee, site) = last_step
                .get(&f.finding_id)
                .map(|&(_, c, s)| (Some(c), Some(s)))
                .unwrap_or((Some(node), None));
            let (kind, verdict, target_name, value, site) = match f.finding_kind {
                FindingKind::Forwarding => (
                    BehaviorKind::Forwards,
                    if conditional {
                        Verdict::Conditional
                    } else {
                        Verdict::Established
                    },
                    f.related_node_id
                        .and_then(|n| formal_name.get(&n))
                        .map(|s| (*s).to_owned()),
                    None,
                    site,
                ),
                FindingKind::TransformedArgument => (
                    BehaviorKind::SuppliesLiteral,
                    if conditional {
                        Verdict::Conditional
                    } else {
                        Verdict::Established
                    },
                    f.related_node_id
                        .and_then(|n| formal_name.get(&n))
                        .map(|s| (*s).to_owned()),
                    member(MemberRole::Value).and_then(|m| m.label.clone()),
                    site,
                ),
                FindingKind::ConditionalRaise => (
                    BehaviorKind::RaisesWhen,
                    Verdict::Conditional,
                    member(MemberRole::Formal).and_then(|m| m.label.clone()),
                    None,
                    f.condition_node_id,
                ),
                FindingKind::UnfollowedArgument => (
                    BehaviorKind::Unfollowed,
                    Verdict::Unknown,
                    member(MemberRole::Formal).and_then(|m| m.label.clone()),
                    member(MemberRole::Reason).and_then(|m| m.label.clone()),
                    site,
                ),
                _ => continue,
            };
            let parameter = source.and_then(|m| m.node_id);
            let target = f.related_node_id;
            out.behaviors.push(BehaviorsRow {
                snapshot_id,
                behavior_id: b::behavior_id(
                    node,
                    kind,
                    parameter,
                    callee,
                    target,
                    value.as_deref(),
                    site,
                ),
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
                site_node_id: site,
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
        stop_reason: None,
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
        let (verdict, value) = match modality {
            Modality::Definite => (Verdict::Established, None),
            _ => (Verdict::Unknown, Some(modality.text().to_owned())),
        };
        out.behaviors.push(BehaviorsRow {
            snapshot_id,
            behavior_id: b::behavior_id(
                op,
                BehaviorKind::Delegates,
                None,
                Some(callee),
                None,
                value.as_deref(),
                Some(site),
            ),
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
            site_node_id: Some(site),
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
                site_node_id: Some(*site),
                occurrences: *n,
                invocation_id: None,
            });
        }
    }
    out.behaviors.sort_by_key(|r| r.behavior_id);
    out.behaviors.dedup_by_key(|r| r.behavior_id);

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

    // Operations.
    for s in &sources {
        let status = if s.kind == DeclarationKind::Class {
            Verdict::NotAnalyzed
        } else if partial.contains(&s.node_id) {
            Verdict::Unknown
        } else {
            Verdict::Established
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
        });
    }

    // Facets.
    let mut facets: BTreeSet<(Id, OperationFacet, String)> = BTreeSet::new();
    for s in &sources {
        let kind = match (s.kind, s.is_method) {
            (DeclarationKind::Class, _) => "class",
            (_, true) => "method",
            _ => "function",
        };
        facets.insert((s.node_id, OperationFacet::Kind, kind.to_owned()));
        facets.insert((s.node_id, OperationFacet::Module, s.module.clone()));
        if s.kind == DeclarationKind::AsyncFunction {
            facets.insert((s.node_id, OperationFacet::Async, "true".to_owned()));
        }
    }
    let attributes = crate::analyze::collect_attributes(ctx, &callables).await?;
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
        facets.insert((node, facet, value.to_owned()));
    }
    for r in &out.behaviors {
        let facet = match r.kind {
            BehaviorKind::Delegates if r.verdict == Verdict::Established => {
                OperationFacet::DelegatesTo
            }
            BehaviorKind::Forwards => OperationFacet::ForwardsTo,
            BehaviorKind::HandsOffTo => OperationFacet::HandsOffTo,
            BehaviorKind::TakesFrom => OperationFacet::TakesFrom,
            _ => continue,
        };
        if let Some(name) = r.callee_node_id.and_then(name_of) {
            facets.insert((r.operation_node_id, facet, name));
        }
    }
    out.facets = facets
        .into_iter()
        .map(|(node_id, facet, value)| OperationFacetsRow {
            snapshot_id,
            node_id,
            facet,
            value,
        })
        .collect();
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
