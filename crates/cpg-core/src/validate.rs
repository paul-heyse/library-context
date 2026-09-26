//! Cross-table validation (DESIGN §8): the generated rules, one read-only query each, over a
//! snapshot session. Validators only read; they never repair. Tests and publication share them.
//!
//! H1 P2: the rules read a cached copy of the session. Every table is read once through its
//! pinned, snapshot-filtered Delta view into memory, and the rules run [`CONCURRENT_RULES`] at a
//! time. Each rule is still one query, with the same SQL and name; only the provider behind each
//! table name differs.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use cpg_schema::behavior::{
    AnalysisConditionNodesRow, AnalysisConditionsRow, ExitSitesRow, FlowTestExactOriginsRow,
    FlowTestValueLinksRow, HandlerActionsRow, HandlerClausesRow, HandlerReturnNoneSitesRow,
    HandlerTypesRow, ModelApplicationsRow, ModelArgumentBindingsRow, ModelCallbacksRow,
    ModelEffectsRow, ModelExceptionsRow, ModelFormalPathsRow, ModelResourcesRow, ModelTargetsRow,
    ModelTransfersRow, ModeledArgumentEvaluationsRow, ModeledAssignmentReturnPathsRow,
    ModeledCallbackSitesRow, ModeledEffectSitesRow, ModeledExactValueTransfersRow,
    ModeledExceptionHandlerCandidatesRow, ModeledExceptionHandlerWalksRow,
    ModeledExceptionReturnNonePathsRow, ModeledExceptionSitesRow, ModeledResourceSitesRow,
    ModeledTransferSitesRow, ReturnExitStatusesRow, SummaryBoundariesRow, SummaryComponentsRow,
    SummaryFlowStepsRow, SummaryFlowsRow, ValueFlowContributionsRow,
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::{Codebook, TestTypeOrigin};
use cpg_schema::condition::{Atom, EvaluationIdentity};
use cpg_schema::condition_kernel::{ConditionRoot, DiagramNode, hydrate_catalog};
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::rules::{Rule, rules};
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::datasource::MemTable;
use datafusion::physical_plan::{ExecutionPlan, collect};
use datafusion::prelude::SessionContext;
use futures::{StreamExt, TryStreamExt};

use crate::{CoreError, snapshot, sql};

/// Rules in flight at once. With `snapshot::TARGET_PARTITIONS` partitions each, 8 × 8 tasks share
/// the tokio runtime's bounded worker pool (guidelines §9); the probe measured 8 as the knee
/// (12–18 s sequential → 3.0–3.5 s, then 0.6 s with the cache).
pub const CONCURRENT_RULES: usize = 8;

/// A rule whose query returned rows: how many, and the first few.
#[derive(Debug, Clone)]
pub struct Violation {
    pub rule: String,
    pub rows: usize,
    pub sample: String,
}

/// One rule's cost, from its own physical plan (H1 P5): wall time, summed operator compute time,
/// and the largest hash-join build it held. Unlike a process-wide peak-RSS delta, these hold
/// when rules run concurrently.
#[derive(Debug, Clone)]
pub struct RuleCost {
    pub rule: String,
    pub seconds: f64,
    pub compute_seconds: f64,
    pub build_bytes: usize,
}

/// Run every rule; an empty result means the snapshot is valid.
pub async fn validate(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    Ok(validate_costed(ctx).await?.0)
}

/// Every table `ctx` registers, read once through its registered (pinned, filtered) view into an
/// in-memory table under the same name. A doctored view is honoured, because what is cached is
/// whatever the session registers; the batches as they were before the write never are.
pub async fn cached_session(ctx: &SessionContext) -> Result<SessionContext, CoreError> {
    let cache = snapshot::empty_session();
    let names = ctx
        .catalog("datafusion")
        .and_then(|c| c.schema("public"))
        .map(|s| s.table_names())
        .unwrap_or_default();
    for name in names {
        let df = ctx.table(name.as_str()).await?;
        let schema = Arc::new(df.schema().as_arrow().clone());
        let partitions = df.collect_partitioned().await?;
        cache.register_table(
            name.as_str(),
            Arc::new(MemTable::try_new(schema, partitions)?),
        )?;
    }
    Ok(cache)
}

/// [`validate`], with each rule's cost, so a stage report can name what dominates. Violations and
/// costs come back in `rules()` order, whatever order the rules finished in.
pub async fn validate_costed(
    ctx: &SessionContext,
) -> Result<(Vec<Violation>, Vec<RuleCost>), CoreError> {
    let cache = cached_session(ctx).await?;
    let mut results: Vec<(usize, Option<Violation>, RuleCost)> =
        futures::stream::iter(rules().into_iter().enumerate())
            .map(|(i, rule)| {
                let cache = cache.clone();
                async move {
                    tokio::spawn(async move { run(&cache, rule).await.map(|(v, c)| (i, v, c)) })
                        .await
                        .map_err(|e| CoreError::Rule(e.to_string()))?
                }
            })
            .buffer_unordered(CONCURRENT_RULES)
            .try_collect()
            .await?;
    results.sort_by_key(|(i, _, _)| *i);
    let mut violations = Vec::new();
    let mut costs = Vec::new();
    for (_, v, c) in results {
        violations.extend(v);
        costs.push(c);
    }
    violations.extend(validate_condition_graph(&cache).await?);
    violations.extend(validate_test_type_links(&cache).await?);
    violations.extend(validate_entry_proofs(&cache).await?);
    violations.extend(validate_models(&cache).await?);
    violations.extend(validate_exit_sites(&cache).await?);
    violations.extend(validate_handlers(&cache).await?);
    violations.extend(validate_value_flow_contributions(&cache).await?);
    violations.extend(validate_value_flow_predecessor_candidates(&cache).await?);
    violations.extend(validate_value_flow_predecessor_compatibility(&cache).await?);
    violations.extend(validate_modeled_exact_value_transfers(&cache).await?);
    violations.extend(validate_modeled_argument_evaluations(&cache).await?);
    violations.extend(validate_modeled_assignment_return_paths(&cache).await?);
    violations.extend(validate_summary_flows(&cache).await?);
    violations.extend(validate_summary_components(&cache).await?);
    Ok((violations, costs))
}

cpg_schema::relations! {
    inventory exit_relations;
    stored_exit_sites = "validate_stored_exit_sites", deps = ["exit_sites"],
        sql = "SELECT * FROM exit_sites".to_owned();
    stored_return_exit_statuses = "validate_stored_return_exit_statuses", deps = ["return_exit_statuses"],
        sql = "SELECT * FROM return_exit_statuses".to_owned();
    stored_handler_clauses = "validate_stored_handler_clauses", deps = ["handler_clauses"],
        sql = "SELECT * FROM handler_clauses".to_owned();
    stored_handler_types = "validate_stored_handler_types", deps = ["handler_types"],
        sql = "SELECT * FROM handler_types".to_owned();
    stored_modeled_exception_handler_candidates = "validate_stored_modeled_exception_handler_candidates",
        deps = ["modeled_exception_handler_candidates"],
        sql = "SELECT * FROM modeled_exception_handler_candidates".to_owned();
    stored_modeled_exception_handler_walks = "validate_stored_modeled_exception_handler_walks",
        deps = ["modeled_exception_handler_walks"],
        sql = "SELECT * FROM modeled_exception_handler_walks".to_owned();
    stored_handler_actions = "validate_stored_handler_actions", deps = ["handler_actions"],
        sql = "SELECT * FROM handler_actions".to_owned();
    stored_handler_return_none_sites = "validate_stored_handler_return_none_sites",
        deps = ["handler_return_none_sites"],
        sql = "SELECT * FROM handler_return_none_sites".to_owned();
    stored_modeled_exception_return_none_paths = "validate_stored_modeled_exception_return_none_paths",
        deps = ["modeled_exception_return_none_paths"],
        sql = "SELECT * FROM modeled_exception_return_none_paths".to_owned();
    stored_value_flow_contributions = "validate_stored_value_flow_contributions",
        deps = ["value_flow_contributions"],
        sql = "SELECT * FROM value_flow_contributions".to_owned();
    stored_analysis_conditions = "validate_stored_analysis_conditions",
        deps = ["analysis_conditions"],
        sql = "SELECT * FROM analysis_conditions".to_owned();
    stored_analysis_condition_nodes = "validate_stored_analysis_condition_nodes",
        deps = ["analysis_condition_nodes"],
        sql = "SELECT * FROM analysis_condition_nodes".to_owned();
    stored_value_flow_predecessor_candidates = "validate_stored_value_flow_predecessor_candidates",
        deps = ["value_flow_predecessor_candidates"],
        sql = "SELECT * FROM value_flow_predecessor_candidates".to_owned();
    stored_value_flow_predecessor_compatibility = "validate_stored_value_flow_predecessor_compatibility",
        deps = ["value_flow_predecessor_compatibility"],
        sql = "SELECT * FROM value_flow_predecessor_compatibility".to_owned();
    stored_modeled_exact_value_transfers = "validate_stored_modeled_exact_value_transfers",
        deps = ["modeled_exact_value_transfers"],
        sql = "SELECT * FROM modeled_exact_value_transfers".to_owned();
    stored_modeled_argument_evaluations = "validate_stored_modeled_argument_evaluations",
        deps = ["modeled_argument_evaluations"],
        sql = "SELECT * FROM modeled_argument_evaluations".to_owned();
    stored_modeled_assignment_return_paths = "validate_stored_modeled_assignment_return_paths",
        deps = ["modeled_assignment_return_paths"],
        sql = "SELECT * FROM modeled_assignment_return_paths".to_owned();
    stored_summary_flows = "validate_stored_summary_flows", deps = ["summary_flows"],
        sql = "SELECT * FROM summary_flows".to_owned();
    stored_summary_components = "validate_stored_summary_components", deps = ["summary_components"],
        sql = "SELECT * FROM summary_components".to_owned();
    stored_summary_flow_steps = "validate_stored_summary_flow_steps", deps = ["summary_flow_steps"],
        sql = "SELECT * FROM summary_flow_steps".to_owned();
    stored_summary_boundaries = "validate_stored_summary_boundaries", deps = ["summary_boundaries"],
        sql = "SELECT * FROM summary_boundaries".to_owned();
    value_flow_snapshot = "validate_value_flow_snapshot", deps = ["releases"],
        sql = "SELECT DISTINCT snapshot_id FROM releases".to_owned();
    value_flow_analysis_count = "validate_value_flow_analysis_count", deps = ["analysis_invocations"],
        sql = "SELECT count(*) AS count FROM analysis_invocations".to_owned();
}

cpg_schema::query_row! {
    struct ValueFlowSnapshotRow {
        snapshot_id: Id,
    }
}

cpg_schema::query_row! {
    struct ValueFlowAnalysisCountRow {
        count: i64,
    }
}

/// Recompute every bounded condition check from the persisted analysis BDD catalog.
async fn validate_value_flow_predecessor_compatibility(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ValueFlowPredecessorCompatibilityRow> = sql::fetch(
        ctx,
        &stored_value_flow_predecessor_compatibility(),
        sql::Params::new(),
    )
    .await?;
    let mut expected = match crate::summaries::predecessor_compatibility(ctx).await {
        Ok(rows) => rows,
        Err(error) => {
            return Ok(vec![Violation {
                rule: "value-flow-predecessor-compatibility-input".to_owned(),
                rows: 1,
                sample: error.to_string(),
            }]);
        }
    };
    let key = |r: &ValueFlowPredecessorCompatibilityRow| {
        (
            r.successor_fact_id,
            r.predecessor_fact_id,
            r.source_key.clone(),
            r.reaching_fact_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "value-flow-predecessor-compatibility-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} compatibility rows; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Rebuild the one-definition predecessor candidates and reject both missing and forged edges.
async fn validate_value_flow_predecessor_candidates(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ValueFlowPredecessorCandidatesRow> = sql::fetch(
        ctx,
        &stored_value_flow_predecessor_candidates(),
        sql::Params::new(),
    )
    .await?;
    let mut expected: Vec<ValueFlowPredecessorCandidatesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::value_flow_predecessor_candidates(),
        sql::Params::new(),
    )
    .await?;
    let key = |r: &ValueFlowPredecessorCandidatesRow| {
        (
            r.successor_fact_id,
            r.predecessor_fact_id,
            r.source_key.clone(),
            r.reaching_fact_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "value-flow-predecessor-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} predecessors; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Reconstruct the exact local model join; omitted or forged candidate paths cannot publish.
async fn validate_modeled_exact_value_transfers(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ModeledExactValueTransfersRow> = sql::fetch(
        ctx,
        &stored_modeled_exact_value_transfers(),
        sql::Params::new(),
    )
    .await?;
    let mut expected: Vec<ModeledExactValueTransfersRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_exact_value_transfers(),
        sql::Params::new(),
    )
    .await?;
    let key = |r: &ModeledExactValueTransfersRow| {
        (
            r.flow_value_fact_id,
            r.parameter_node_id,
            r.pysa_fact_id,
            r.model_id,
            r.rule_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "modeled-exact-value-transfer-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} direct transfers; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Reconstruct every candidate-local argument decision from the exact model step and syntax.
async fn validate_modeled_argument_evaluations(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ModeledArgumentEvaluationsRow> = sql::fetch(
        ctx,
        &stored_modeled_argument_evaluations(),
        sql::Params::new(),
    )
    .await?;
    let mut expected: Vec<ModeledArgumentEvaluationsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_argument_evaluations(),
        sql::Params::new(),
    )
    .await?;
    let key = |r: &ModeledArgumentEvaluationsRow| {
        (
            r.candidate_flow_fact_id,
            r.parameter_node_id,
            r.pysa_fact_id,
            r.model_id,
            r.rule_id,
            r.argument_fact_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "modeled-argument-evaluation-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} modeled argument evaluations; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Rebuild the two-step source/model path; no missing or forged candidate may publish.
async fn validate_modeled_assignment_return_paths(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ModeledAssignmentReturnPathsRow> = sql::fetch(
        ctx,
        &stored_modeled_assignment_return_paths(),
        sql::Params::new(),
    )
    .await?;
    let mut expected: Vec<ModeledAssignmentReturnPathsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_assignment_return_paths(),
        sql::Params::new(),
    )
    .await?;
    let key = |r: &ModeledAssignmentReturnPathsRow| {
        (
            r.successor_fact_id,
            r.predecessor_fact_id,
            r.reaching_fact_id,
            r.parameter_node_id,
            r.pysa_fact_id,
            r.model_id,
            r.rule_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "modeled-assignment-return-path-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!("stored {} paths; derived {}", actual.len(), expected.len()),
        }])
    }
}

/// Reconstruct every source-call SCC and its canonical schedule from the published call graph.
async fn validate_summary_components(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<SummaryComponentsRow> =
        sql::fetch(ctx, &stored_summary_components(), sql::Params::new()).await?;
    let mut expected = crate::summaries::call_components(ctx).await?;
    actual.sort_by_key(|row| row.function_node_id);
    expected.sort_by_key(|row| row.function_node_id);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "summary-component-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} component memberships; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Rebuild every finite flow with the same bounded condition kernel used at write.
async fn validate_summary_flows(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<SummaryFlowsRow> =
        sql::fetch(ctx, &stored_summary_flows(), sql::Params::new()).await?;
    let summary = match crate::summaries::finite_flows(ctx).await {
        Ok(rows) => rows,
        Err(error) => {
            return Ok(vec![Violation {
                rule: "summary-flow-input".to_owned(),
                rows: 1,
                sample: error.to_string(),
            }]);
        }
    };
    let mut expected = summary.flows;
    let mut expected_steps = summary.steps;
    actual.sort_by_key(|r| r.summary_id);
    expected.sort_by_key(|r| r.summary_id);
    let mut violations = Vec::new();
    if actual != expected {
        violations.push(Violation {
            rule: "summary-flow-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} summary flows; derived {}",
                actual.len(),
                expected.len()
            ),
        });
    }
    let mut actual_steps: Vec<SummaryFlowStepsRow> =
        sql::fetch(ctx, &stored_summary_flow_steps(), sql::Params::new()).await?;
    let key = |r: &SummaryFlowStepsRow| (r.summary_id, r.ordinal);
    actual_steps.sort_by_key(key);
    expected_steps.sort_by_key(key);
    if actual_steps != expected_steps {
        violations.push(Violation {
            rule: "summary-flow-step-source-equality".to_owned(),
            rows: actual_steps.len().abs_diff(expected_steps.len()).max(1),
            sample: format!(
                "stored {} summary steps; derived {}",
                actual_steps.len(),
                expected_steps.len()
            ),
        });
    }
    violations.extend(validate_summary_boundaries(ctx, summary.boundaries).await?);
    Ok(violations)
}

/// Reconstruct the explicit complement of the finite source-to-return summary proof.
async fn validate_summary_boundaries(
    ctx: &SessionContext,
    mut expected: Vec<SummaryBoundariesRow>,
) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<SummaryBoundariesRow> =
        sql::fetch(ctx, &stored_summary_boundaries(), sql::Params::new()).await?;
    let key = |r: &SummaryBoundariesRow| {
        (
            r.function_node_id,
            r.parameter_node_id,
            r.source_flow_fact_id,
            r.condition_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "summary-boundary-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} boundaries; derived {}",
                actual.len(),
                expected.len()
            ),
        }])
    }
}

/// Reconstruct from the raw flow provider, including rows the display view deliberately omits.
/// This rejects both forged contributions and missing ones before snapshot publication.
async fn validate_value_flow_contributions(
    ctx: &SessionContext,
) -> Result<Vec<Violation>, CoreError> {
    let snapshots: Vec<ValueFlowSnapshotRow> =
        sql::fetch(ctx, &value_flow_snapshot(), sql::Params::new()).await?;
    if snapshots.len() != 1 {
        return Ok(vec![Violation {
            rule: "value-flow-contribution-snapshot".to_owned(),
            rows: snapshots.len().abs_diff(1).max(1),
            sample: format!("expected one release snapshot; found {}", snapshots.len()),
        }]);
    }
    let mut actual: Vec<ValueFlowContributionsRow> =
        sql::fetch(ctx, &stored_value_flow_contributions(), sql::Params::new()).await?;
    let mut actual_conditions: Vec<AnalysisConditionsRow> =
        sql::fetch(ctx, &stored_analysis_conditions(), sql::Params::new()).await?;
    let mut actual_nodes: Vec<AnalysisConditionNodesRow> =
        sql::fetch(ctx, &stored_analysis_condition_nodes(), sql::Params::new()).await?;
    let invocation_count: Vec<ValueFlowAnalysisCountRow> =
        sql::fetch(ctx, &value_flow_analysis_count(), sql::Params::new()).await?;
    if invocation_count[0].count == 0 {
        return if actual.is_empty() && actual_conditions.is_empty() && actual_nodes.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![Violation {
                rule: "value-flow-contributions-without-analysis".to_owned(),
                rows: actual.len() + actual_conditions.len() + actual_nodes.len(),
                sample: "flow analysis rows exist without an analysis invocation".to_owned(),
            }])
        };
    }
    let model = crate::flow_model::run(ctx, snapshots[0].snapshot_id).await?;
    let mut expected = model.value_flow_contributions;
    let (expected_conditions, expected_nodes) = crate::flow_model::condition_catalog_rows(
        snapshots[0].snapshot_id,
        &model.condition_models,
    );
    let key = |r: &ValueFlowContributionsRow| {
        (
            r.flow_value_fact_id,
            r.use_id,
            r.source_key.clone(),
            r.identity,
            r.through_call,
            r.local_through_call,
            r.upstream_identity,
            r.upstream_through_call,
            r.condition_id,
        )
    };
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    let mut violations = Vec::new();
    if actual != expected {
        violations.push(Violation {
            rule: "value-flow-contribution-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!(
                "stored {} contributions; derived {}",
                actual.len(),
                expected.len()
            ),
        });
    }
    actual_conditions.sort_by_key(|r| r.condition_id);
    actual_nodes.sort_by_key(|r| r.node_id);
    if actual_conditions != expected_conditions || actual_nodes != expected_nodes {
        violations.push(Violation {
            rule: "analysis-condition-source-equality".to_owned(),
            rows: actual_conditions.len().abs_diff(expected_conditions.len())
                + actual_nodes.len().abs_diff(expected_nodes.len()).max(1),
            sample: format!(
                "stored {} roots/{} nodes; derived {} roots/{} nodes",
                actual_conditions.len(),
                actual_nodes.len(),
                expected_conditions.len(),
                expected_nodes.len()
            ),
        });
    }
    let roots: Vec<ConditionRoot> = actual_conditions
        .iter()
        .map(|r| ConditionRoot {
            condition_id: r.condition_id,
            root_id: r.root_id,
            boundary_reason: r.boundary_reason.clone(),
        })
        .collect();
    let nodes: Vec<DiagramNode> = actual_nodes
        .iter()
        .map(|r| DiagramNode {
            node_id: r.node_id,
            atom: r.atom.clone(),
            low: r.low_id,
            high: r.high_id,
        })
        .collect();
    if let Err(error) = hydrate_catalog(&roots, &nodes) {
        violations.push(Violation {
            rule: "analysis-condition-graph".to_owned(),
            rows: 1,
            sample: error,
        });
    }
    Ok(violations)
}

/// Publication and consumers use the same derivation as the compiler. Equality also rejects a
/// missing structural site, an altered condition and a mismatched source/region citation.
async fn validate_exit_sites(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut actual: Vec<ExitSitesRow> =
        sql::fetch(ctx, &stored_exit_sites(), sql::Params::new()).await?;
    let mut expected: Vec<ExitSitesRow> =
        sql::fetch(ctx, &cpg_schema::behavior::exit_sites(), sql::Params::new()).await?;
    let key = |row: &ExitSitesRow| (row.function_node_id, row.site_node_id, row.kind);
    actual.sort_by_key(key);
    expected.sort_by_key(key);
    let mut violations = Vec::new();
    if actual != expected {
        violations.push(Violation {
            rule: "exit-site-source-equality".to_owned(),
            rows: actual.len().abs_diff(expected.len()).max(1),
            sample: format!("stored {} sites; derived {}", actual.len(), expected.len()),
        });
    }
    let mut actual_statuses: Vec<ReturnExitStatusesRow> =
        sql::fetch(ctx, &stored_return_exit_statuses(), sql::Params::new()).await?;
    let mut expected_statuses: Vec<ReturnExitStatusesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::return_exit_statuses(),
        sql::Params::new(),
    )
    .await?;
    actual_statuses.sort_by_key(|row| row.site_node_id);
    expected_statuses.sort_by_key(|row| row.site_node_id);
    if actual_statuses != expected_statuses {
        violations.push(Violation {
            rule: "return-exit-status-source-equality".to_owned(),
            rows: actual_statuses
                .len()
                .abs_diff(expected_statuses.len())
                .max(1),
            sample: format!(
                "stored {} statuses; derived {}",
                actual_statuses.len(),
                expected_statuses.len()
            ),
        });
    }
    Ok(violations)
}

async fn validate_handlers(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut actual_clauses: Vec<HandlerClausesRow> =
        sql::fetch(ctx, &stored_handler_clauses(), sql::Params::new()).await?;
    let mut expected_clauses: Vec<HandlerClausesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::handler_clauses(),
        sql::Params::new(),
    )
    .await?;
    actual_clauses.sort_by_key(|r| r.handler_node_id);
    expected_clauses.sort_by_key(|r| r.handler_node_id);
    let mut actual_types: Vec<HandlerTypesRow> =
        sql::fetch(ctx, &stored_handler_types(), sql::Params::new()).await?;
    let mut expected_types: Vec<HandlerTypesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::handler_types(),
        sql::Params::new(),
    )
    .await?;
    actual_types.sort_by_key(|r| r.handler_node_id);
    expected_types.sort_by_key(|r| r.handler_node_id);
    let mut actual_candidates: Vec<ModeledExceptionHandlerCandidatesRow> = sql::fetch(
        ctx,
        &stored_modeled_exception_handler_candidates(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_candidates: Vec<ModeledExceptionHandlerCandidatesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_exception_handler_candidates(),
        sql::Params::new(),
    )
    .await?;
    let candidate_key = |r: &ModeledExceptionHandlerCandidatesRow| {
        (
            r.call_site_node_id,
            r.pysa_fact_id,
            r.model_id,
            r.rule_id,
            r.handler_node_id,
        )
    };
    actual_candidates.sort_by_key(candidate_key);
    expected_candidates.sort_by_key(candidate_key);
    let mut actual_walks: Vec<ModeledExceptionHandlerWalksRow> = sql::fetch(
        ctx,
        &stored_modeled_exception_handler_walks(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_walks: Vec<ModeledExceptionHandlerWalksRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_exception_handler_walks(),
        sql::Params::new(),
    )
    .await?;
    let walk_key = |r: &ModeledExceptionHandlerWalksRow| {
        (r.call_site_node_id, r.pysa_fact_id, r.model_id, r.rule_id)
    };
    actual_walks.sort_by_key(walk_key);
    expected_walks.sort_by_key(walk_key);
    let mut actual_actions: Vec<HandlerActionsRow> =
        sql::fetch(ctx, &stored_handler_actions(), sql::Params::new()).await?;
    let mut expected_actions: Vec<HandlerActionsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::handler_actions(),
        sql::Params::new(),
    )
    .await?;
    let key = |r: &HandlerActionsRow| (r.handler_node_id, r.action_node_id);
    actual_actions.sort_by_key(key);
    expected_actions.sort_by_key(key);
    let mut actual_returns: Vec<HandlerReturnNoneSitesRow> =
        sql::fetch(ctx, &stored_handler_return_none_sites(), sql::Params::new()).await?;
    let mut expected_returns: Vec<HandlerReturnNoneSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::handler_return_none_sites(),
        sql::Params::new(),
    )
    .await?;
    actual_returns.sort_by_key(|r| r.handler_node_id);
    expected_returns.sort_by_key(|r| r.handler_node_id);
    let mut actual_return_paths: Vec<ModeledExceptionReturnNonePathsRow> = sql::fetch(
        ctx,
        &stored_modeled_exception_return_none_paths(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_return_paths: Vec<ModeledExceptionReturnNonePathsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_exception_return_none_paths(),
        sql::Params::new(),
    )
    .await?;
    let return_path_key = |r: &ModeledExceptionReturnNonePathsRow| {
        (r.call_site_node_id, r.pysa_fact_id, r.model_id, r.rule_id)
    };
    actual_return_paths.sort_by_key(return_path_key);
    expected_return_paths.sort_by_key(return_path_key);
    let mut violations = Vec::new();
    if actual_clauses != expected_clauses {
        violations.push(Violation {
            rule: "handler-clause-source-equality".to_owned(),
            rows: actual_clauses.len().abs_diff(expected_clauses.len()).max(1),
            sample: format!(
                "stored {} clauses; derived {}",
                actual_clauses.len(),
                expected_clauses.len()
            ),
        });
    }
    if actual_actions != expected_actions {
        violations.push(Violation {
            rule: "handler-action-source-equality".to_owned(),
            rows: actual_actions.len().abs_diff(expected_actions.len()).max(1),
            sample: format!(
                "stored {} actions; derived {}",
                actual_actions.len(),
                expected_actions.len()
            ),
        });
    }
    if actual_returns != expected_returns {
        violations.push(Violation {
            rule: "handler-return-none-source-equality".to_owned(),
            rows: actual_returns.len().abs_diff(expected_returns.len()).max(1),
            sample: format!(
                "stored {} direct return-None sites; derived {}",
                actual_returns.len(),
                expected_returns.len()
            ),
        });
    }
    if actual_return_paths != expected_return_paths {
        violations.push(Violation {
            rule: "modeled-exception-return-none-path-source-equality".to_owned(),
            rows: actual_return_paths
                .len()
                .abs_diff(expected_return_paths.len())
                .max(1),
            sample: format!(
                "stored {} conditional return paths; derived {}",
                actual_return_paths.len(),
                expected_return_paths.len()
            ),
        });
    }
    if actual_types != expected_types {
        violations.push(Violation {
            rule: "handler-type-source-equality".to_owned(),
            rows: actual_types.len().abs_diff(expected_types.len()).max(1),
            sample: format!(
                "stored {} handler types; derived {}",
                actual_types.len(),
                expected_types.len()
            ),
        });
    }
    if actual_candidates != expected_candidates {
        violations.push(Violation {
            rule: "modeled-exception-handler-source-equality".to_owned(),
            rows: actual_candidates
                .len()
                .abs_diff(expected_candidates.len())
                .max(1),
            sample: format!(
                "stored {} candidates; derived {}",
                actual_candidates.len(),
                expected_candidates.len()
            ),
        });
    }
    if actual_walks != expected_walks {
        violations.push(Violation {
            rule: "modeled-exception-handler-walk-source-equality".to_owned(),
            rows: actual_walks.len().abs_diff(expected_walks.len()).max(1),
            sample: format!(
                "stored {} walks; derived {}",
                actual_walks.len(),
                expected_walks.len()
            ),
        });
    }
    Ok(violations)
}

cpg_schema::relations! {
    inventory model_relations;
    model_contexts = "validate_model_contexts", deps = ["contexts"],
        sql = "SELECT * FROM contexts".to_owned();
    model_producers = "validate_model_producers", deps = ["producers"],
        sql = "SELECT * FROM producers".to_owned();
    model_modules = "validate_model_modules", deps = ["context_modules"],
        sql = "SELECT * FROM context_modules".to_owned();
    model_definitions = "validate_model_definitions", deps = ["context_definitions"],
        sql = "SELECT * FROM context_definitions".to_owned();
    model_parameters = "validate_model_parameters", deps = ["context_parameters"],
        sql = "SELECT * FROM context_parameters".to_owned();
    model_targets = "validate_model_targets", deps = ["model_targets"],
        sql = "SELECT * FROM model_targets".to_owned();
    model_applications = "validate_model_applications", deps = ["model_applications"],
        sql = "SELECT * FROM model_applications".to_owned();
    model_formal_paths = "validate_model_formal_paths", deps = ["model_formal_paths"],
        sql = "SELECT * FROM model_formal_paths".to_owned();
    model_argument_bindings = "validate_model_argument_bindings", deps = ["model_argument_bindings"],
        sql = "SELECT * FROM model_argument_bindings".to_owned();
    modeled_callback_sites = "validate_modeled_callback_sites", deps = ["modeled_callback_sites"],
        sql = "SELECT * FROM modeled_callback_sites".to_owned();
    modeled_resource_sites = "validate_modeled_resource_sites", deps = ["modeled_resource_sites"],
        sql = "SELECT * FROM modeled_resource_sites".to_owned();
    modeled_transfer_sites = "validate_modeled_transfer_sites", deps = ["modeled_transfer_sites"],
        sql = "SELECT * FROM modeled_transfer_sites".to_owned();
    modeled_effect_sites = "validate_modeled_effect_sites", deps = ["modeled_effect_sites"],
        sql = "SELECT * FROM modeled_effect_sites".to_owned();
    modeled_exception_sites = "validate_modeled_exception_sites", deps = ["modeled_exception_sites"],
        sql = "SELECT * FROM modeled_exception_sites".to_owned();
    model_transfers = "validate_model_transfers", deps = ["model_transfers"],
        sql = "SELECT * FROM model_transfers".to_owned();
    model_effects = "validate_model_effects", deps = ["model_effects"],
        sql = "SELECT * FROM model_effects".to_owned();
    model_callbacks = "validate_model_callbacks", deps = ["model_callbacks"],
        sql = "SELECT * FROM model_callbacks".to_owned();
    model_resources = "validate_model_resources", deps = ["model_resources"],
        sql = "SELECT * FROM model_resources".to_owned();
    model_exceptions = "validate_model_exceptions", deps = ["model_exceptions"],
        sql = "SELECT * FROM model_exceptions".to_owned();
}

/// Rebuild model rows from the committed bytes and the pinned context views. A stored row cannot
/// invent a model id, transfer path or target citation, and a missing applicable row is invalid.
async fn validate_models(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let contexts: Vec<cpg_schema::tables::ContextsRow> =
        sql::fetch(ctx, &model_contexts(), sql::Params::new()).await?;
    let producers: Vec<cpg_schema::tables::ProducersRow> =
        sql::fetch(ctx, &model_producers(), sql::Params::new()).await?;
    let modules: Vec<cpg_schema::tables::ContextModulesRow> =
        sql::fetch(ctx, &model_modules(), sql::Params::new()).await?;
    let definitions: Vec<cpg_schema::tables::ContextDefinitionsRow> =
        sql::fetch(ctx, &model_definitions(), sql::Params::new()).await?;
    let parameters: Vec<cpg_schema::tables::ContextParametersRow> =
        sql::fetch(ctx, &model_parameters(), sql::Params::new()).await?;
    let mut actual_targets: Vec<ModelTargetsRow> =
        sql::fetch(ctx, &model_targets(), sql::Params::new()).await?;
    let mut actual_applications: Vec<ModelApplicationsRow> =
        sql::fetch(ctx, &model_applications(), sql::Params::new()).await?;
    let mut actual_formals: Vec<ModelFormalPathsRow> =
        sql::fetch(ctx, &model_formal_paths(), sql::Params::new()).await?;
    let mut actual_arguments: Vec<ModelArgumentBindingsRow> =
        sql::fetch(ctx, &model_argument_bindings(), sql::Params::new()).await?;
    let mut actual_callback_sites: Vec<ModeledCallbackSitesRow> =
        sql::fetch(ctx, &modeled_callback_sites(), sql::Params::new()).await?;
    let mut actual_resource_sites: Vec<ModeledResourceSitesRow> =
        sql::fetch(ctx, &modeled_resource_sites(), sql::Params::new()).await?;
    let mut actual_transfer_sites: Vec<ModeledTransferSitesRow> =
        sql::fetch(ctx, &modeled_transfer_sites(), sql::Params::new()).await?;
    let mut actual_effect_sites: Vec<ModeledEffectSitesRow> =
        sql::fetch(ctx, &modeled_effect_sites(), sql::Params::new()).await?;
    let mut actual_exception_sites: Vec<ModeledExceptionSitesRow> =
        sql::fetch(ctx, &modeled_exception_sites(), sql::Params::new()).await?;
    let mut actual_transfers: Vec<ModelTransfersRow> =
        sql::fetch(ctx, &model_transfers(), sql::Params::new()).await?;
    let mut actual_effects: Vec<ModelEffectsRow> =
        sql::fetch(ctx, &model_effects(), sql::Params::new()).await?;
    let mut actual_callbacks: Vec<ModelCallbacksRow> =
        sql::fetch(ctx, &model_callbacks(), sql::Params::new()).await?;
    let mut actual_resources: Vec<ModelResourcesRow> =
        sql::fetch(ctx, &model_resources(), sql::Params::new()).await?;
    let mut actual_exceptions: Vec<ModelExceptionsRow> =
        sql::fetch(ctx, &model_exceptions(), sql::Params::new()).await?;
    let all_empty = actual_targets.is_empty()
        && actual_applications.is_empty()
        && actual_formals.is_empty()
        && actual_arguments.is_empty()
        && actual_callback_sites.is_empty()
        && actual_resource_sites.is_empty()
        && actual_transfer_sites.is_empty()
        && actual_effect_sites.is_empty()
        && actual_exception_sites.is_empty()
        && actual_transfers.is_empty()
        && actual_effects.is_empty()
        && actual_callbacks.is_empty()
        && actual_resources.is_empty()
        && actual_exceptions.is_empty();
    let analyzed = producers.iter().any(|p| p.tool == crate::analyze::TOOL);
    if !analyzed && all_empty {
        return Ok(Vec::new());
    }
    let Some(snapshot_id) = contexts.first().map(|row| row.snapshot_id) else {
        return Ok(if all_empty {
            Vec::new()
        } else {
            vec![Violation {
                rule: "model-catalog-context".into(),
                rows: 1,
                sample: "model rows without a pinned analysis context".into(),
            }]
        });
    };
    let catalog = cpg_schema::models::Catalog::committed().map_err(CoreError::Analysis)?;
    let mut expected_targets = catalog
        .bind_targets(snapshot_id, &contexts, &modules, &definitions)
        .map_err(CoreError::Analysis)?;
    let mut expected = catalog
        .compile_rules(&expected_targets, &definitions, &parameters)
        .map_err(CoreError::Analysis)?;
    let mut expected_applications: Vec<ModelApplicationsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::model_applications(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_arguments: Vec<ModelArgumentBindingsRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::model_argument_bindings(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_callback_sites: Vec<ModeledCallbackSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_callback_sites(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_resource_sites: Vec<ModeledResourceSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_resource_sites(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_transfer_sites: Vec<ModeledTransferSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_transfer_sites(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_effect_sites: Vec<ModeledEffectSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_effect_sites(),
        sql::Params::new(),
    )
    .await?;
    let mut expected_exception_sites: Vec<ModeledExceptionSitesRow> = sql::fetch(
        ctx,
        &cpg_schema::behavior::modeled_exception_sites(),
        sql::Params::new(),
    )
    .await?;
    expected_targets.sort_by_key(|row| (row.model_id, row.target_node_id));
    actual_targets.sort_by_key(|row| (row.model_id, row.target_node_id));
    expected_applications
        .sort_by_key(|row| (row.call_site_node_id, row.pysa_fact_id, row.model_id));
    actual_applications.sort_by_key(|row| (row.call_site_node_id, row.pysa_fact_id, row.model_id));
    expected.formals.sort_by_key(|row| {
        (
            row.model_id,
            row.target_node_id,
            row.rule_id,
            row.path_role.code(),
            row.path_id,
        )
    });
    actual_formals.sort_by_key(|row| {
        (
            row.model_id,
            row.target_node_id,
            row.rule_id,
            row.path_role.code(),
            row.path_id,
        )
    });
    expected_arguments.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
            row.path_role.code(),
            row.path_id,
        )
    });
    actual_arguments.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
            row.path_role.code(),
            row.path_id,
        )
    });
    expected_callback_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    actual_callback_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    expected_resource_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    actual_resource_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    expected_transfer_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    actual_transfer_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    expected_effect_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    actual_effect_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    expected_exception_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    actual_exception_sites.sort_by_key(|row| {
        (
            row.call_site_node_id,
            row.pysa_fact_id,
            row.model_id,
            row.rule_id,
        )
    });
    expected
        .transfers
        .sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    actual_transfers.sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    expected
        .effects
        .sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    actual_effects.sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    expected
        .callbacks
        .sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    actual_callbacks.sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    expected
        .resources
        .sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    actual_resources.sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    expected
        .exceptions
        .sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    actual_exceptions.sort_by_key(|row| (row.model_id, row.target_node_id, row.rule_id));
    let mut violations = Vec::new();
    if !analyzed {
        violations.push(Violation {
            rule: "model-catalog-context".into(),
            rows: 1,
            sample: "model rows without a compiler run".into(),
        });
    }
    if expected_targets != actual_targets {
        violations.push(Violation {
            rule: "model-catalog-target-equality".into(),
            rows: 1,
            sample: format!(
                "expected {} target rows, stored {}",
                expected_targets.len(),
                actual_targets.len()
            ),
        });
    }
    if expected_applications != actual_applications {
        violations.push(Violation {
            rule: "model-application-source-equality".into(),
            rows: actual_applications
                .len()
                .abs_diff(expected_applications.len())
                .max(1),
            sample: format!(
                "expected {} model call applications, stored {}",
                expected_applications.len(),
                actual_applications.len()
            ),
        });
    }
    if expected.formals != actual_formals {
        violations.push(Violation {
            rule: "model-catalog-formal-path-equality".into(),
            rows: actual_formals.len().abs_diff(expected.formals.len()).max(1),
            sample: format!(
                "expected {} typed model formal paths, stored {}",
                expected.formals.len(),
                actual_formals.len()
            ),
        });
    }
    if expected_arguments != actual_arguments {
        violations.push(Violation {
            rule: "model-argument-source-equality".into(),
            rows: actual_arguments
                .len()
                .abs_diff(expected_arguments.len())
                .max(1),
            sample: format!(
                "expected {} model argument bindings, stored {}",
                expected_arguments.len(),
                actual_arguments.len()
            ),
        });
    }
    if expected_callback_sites != actual_callback_sites {
        violations.push(Violation {
            rule: "modeled-callback-site-source-equality".into(),
            rows: actual_callback_sites
                .len()
                .abs_diff(expected_callback_sites.len())
                .max(1),
            sample: format!(
                "expected {} modeled callback sites, stored {}",
                expected_callback_sites.len(),
                actual_callback_sites.len()
            ),
        });
    }
    if expected_resource_sites != actual_resource_sites {
        violations.push(Violation {
            rule: "modeled-resource-site-source-equality".into(),
            rows: actual_resource_sites
                .len()
                .abs_diff(expected_resource_sites.len())
                .max(1),
            sample: format!(
                "expected {} modeled resource sites, stored {}",
                expected_resource_sites.len(),
                actual_resource_sites.len()
            ),
        });
    }
    if expected_transfer_sites != actual_transfer_sites {
        violations.push(Violation {
            rule: "modeled-transfer-site-source-equality".into(),
            rows: actual_transfer_sites
                .len()
                .abs_diff(expected_transfer_sites.len())
                .max(1),
            sample: format!(
                "expected {} modeled transfer sites, stored {}",
                expected_transfer_sites.len(),
                actual_transfer_sites.len()
            ),
        });
    }
    if expected_effect_sites != actual_effect_sites {
        violations.push(Violation {
            rule: "modeled-effect-site-source-equality".into(),
            rows: actual_effect_sites
                .len()
                .abs_diff(expected_effect_sites.len())
                .max(1),
            sample: format!(
                "expected {} modeled effect sites, stored {}",
                expected_effect_sites.len(),
                actual_effect_sites.len()
            ),
        });
    }
    if expected_exception_sites != actual_exception_sites {
        violations.push(Violation {
            rule: "modeled-exception-site-source-equality".into(),
            rows: actual_exception_sites
                .len()
                .abs_diff(expected_exception_sites.len())
                .max(1),
            sample: format!(
                "expected {} modeled exception sites, stored {}",
                expected_exception_sites.len(),
                actual_exception_sites.len()
            ),
        });
    }
    if expected.transfers != actual_transfers {
        violations.push(Violation {
            rule: "model-catalog-transfer-equality".into(),
            rows: 1,
            sample: format!(
                "expected {} transfer rows, stored {}",
                expected.transfers.len(),
                actual_transfers.len()
            ),
        });
    }
    if expected.effects != actual_effects {
        violations.push(Violation {
            rule: "model-catalog-effect-equality".into(),
            rows: 1,
            sample: format!(
                "expected {} effect rows, stored {}",
                expected.effects.len(),
                actual_effects.len()
            ),
        });
    }
    for (kind, expected_len, actual_len, unequal) in [
        (
            "callback",
            expected.callbacks.len(),
            actual_callbacks.len(),
            expected.callbacks != actual_callbacks,
        ),
        (
            "resource",
            expected.resources.len(),
            actual_resources.len(),
            expected.resources != actual_resources,
        ),
        (
            "exception",
            expected.exceptions.len(),
            actual_exceptions.len(),
            expected.exceptions != actual_exceptions,
        ),
    ] {
        if unequal {
            violations.push(Violation {
                rule: format!("model-catalog-{kind}-equality"),
                rows: 1,
                sample: format!("expected {expected_len} {kind} rows, stored {actual_len}"),
            });
        }
    }
    Ok(violations)
}

cpg_schema::query_row! {
    struct SourceSnapshot {
        snapshot_id: Id,
    }
}
cpg_schema::relations! {
    inventory test_value_relations;
    test_value_links = "validate_test_value_links", deps = ["flow_test_value_links"],
        sql = "SELECT * FROM flow_test_value_links".to_owned();
    exact_origins = "validate_exact_origins", deps = ["flow_test_exact_origins"],
        sql = "SELECT * FROM flow_test_exact_origins".to_owned();
    test_value_source_snapshots = "validate_test_value_source_snapshots", deps = ["flow_uses"],
        sql = "SELECT DISTINCT snapshot_id FROM flow_uses LIMIT 2".to_owned();
}

/// Reconstruct every proof from the pinned raw views. This also catches missing, duplicate and
/// doctored rows; no consumer may treat a persisted link as authority before this check passes.
async fn validate_entry_proofs(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut actual_links: Vec<FlowTestValueLinksRow> =
        sql::fetch(ctx, &test_value_links(), sql::Params::new()).await?;
    let mut actual_origins: Vec<FlowTestExactOriginsRow> =
        sql::fetch(ctx, &exact_origins(), sql::Params::new()).await?;
    let sources: Vec<SourceSnapshot> =
        sql::fetch(ctx, &test_value_source_snapshots(), sql::Params::new()).await?;
    let (mut expected_links, mut expected_origins) = if let [source] = sources.as_slice() {
        crate::entry_links::all(ctx, source.snapshot_id).await?
    } else {
        (Vec::new(), Vec::new())
    };
    let link_key = |row: &FlowTestValueLinksRow| {
        (
            row.operation_node_id,
            row.formal_node_id,
            row.leaf_fact_id,
            row.use_id,
            row.link_id,
        )
    };
    actual_links.sort_by_key(&link_key);
    expected_links.sort_by_key(&link_key);
    let origin_key = |row: &FlowTestExactOriginsRow| {
        (
            row.operation_node_id,
            row.formal_node_id,
            row.leaf_fact_id,
            row.use_id,
            row.origin_id,
        )
    };
    actual_origins.sort_by_key(&origin_key);
    expected_origins.sort_by_key(&origin_key);
    let mut violations = Vec::new();
    if sources.len() > 1 || actual_links != expected_links {
        violations.push(Violation {
            rule: "flow-test-value-proof-link".to_owned(),
            rows: actual_links.len().abs_diff(expected_links.len()).max(1),
            sample: format!(
                "stored {} links; derived {} from {} source snapshot(s)",
                actual_links.len(),
                expected_links.len(),
                sources.len()
            ),
        });
    }
    if sources.len() > 1 || actual_origins != expected_origins {
        violations.push(Violation {
            rule: "flow-test-exact-origin".to_owned(),
            rows: actual_origins.len().abs_diff(expected_origins.len()).max(1),
            sample: format!(
                "stored {} origins; derived {} from {} source snapshot(s)",
                actual_origins.len(),
                expected_origins.len(),
                sources.len()
            ),
        });
    }
    Ok(violations)
}

cpg_schema::query_row! {
    struct TestTypeLink {
        fact_id: Id,
        module_node_id: Id,
        leaf_fact_id: Id,
        atom_id: Id,
        use_id: Id,
        use_fact_id: Id,
        operand_start_byte: i64,
        operand_end_byte: i64,
        role: String,
        place: String,
        term_node_id: Id,
        term_fact_id: Id,
        origin: i64,
    }
}
cpg_schema::query_row! {
    struct LinkLeaf {
        fact_id: Id,
        module_node_id: Id,
        atom_id: Id,
        atom: String,
        leaf_start_byte: i64,
        leaf_end_byte: i64,
    }
}
cpg_schema::query_row! {
    struct LinkUse {
        fact_id: Id,
        use_id: Id,
        module_node_id: Id,
        place: String,
        start_byte: i64,
        end_byte: i64,
        annotation: bool,
    }
}
cpg_schema::query_row! {
    struct LinkTerm {
        node_id: Id,
        fact_id: Id,
    }
}
cpg_schema::relations! {
    inventory test_type_relations;
    test_type_links = "validate_test_type_links", deps = ["flow_test_types"],
        sql = "SELECT fact_id, module_node_id, leaf_fact_id, atom_id, use_id, use_fact_id, operand_start_byte, operand_end_byte, role, place, term_node_id, term_fact_id, CAST(origin AS BIGINT) AS origin FROM flow_test_types".to_owned();
    test_type_leaves = "validate_test_type_leaves", deps = ["flow_test_leaves"],
        sql = "SELECT fact_id, module_node_id, atom_id, atom, leaf_start_byte, leaf_end_byte FROM flow_test_leaves".to_owned();
    test_type_uses = "validate_test_type_uses", deps = ["flow_uses"],
        sql = "SELECT fact_id, use_id, module_node_id, place, start_byte, end_byte, annotation FROM flow_uses".to_owned();
    test_type_terms = "validate_test_type_terms", deps = ["type_terms"],
        sql = "SELECT node_id, fact_id FROM type_terms".to_owned();
}

async fn validate_test_type_links(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let links: Vec<TestTypeLink> = sql::fetch(ctx, &test_type_links(), sql::Params::new()).await?;
    if links.is_empty() {
        return Ok(Vec::new());
    }
    let leaves: Vec<LinkLeaf> = sql::fetch(ctx, &test_type_leaves(), sql::Params::new()).await?;
    let uses: Vec<LinkUse> = sql::fetch(ctx, &test_type_uses(), sql::Params::new()).await?;
    let terms: Vec<LinkTerm> = sql::fetch(ctx, &test_type_terms(), sql::Params::new()).await?;
    let leaves: HashMap<Id, Vec<LinkLeaf>> =
        leaves.into_iter().fold(HashMap::new(), |mut map, row| {
            map.entry(row.fact_id).or_default().push(row);
            map
        });
    let uses: HashMap<Id, Vec<LinkUse>> = uses.into_iter().fold(HashMap::new(), |mut map, row| {
        map.entry(row.use_id).or_default().push(row);
        map
    });
    let terms: HashSet<(Id, Id)> = terms.into_iter().map(|t| (t.node_id, t.fact_id)).collect();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for link in links {
        let leaf = leaves
            .get(&link.leaf_fact_id)
            .filter(|rows| rows.len() == 1)
            .and_then(|rows| rows.first());
        let use_row = uses
            .get(&link.use_id)
            .filter(|rows| rows.len() == 1)
            .and_then(|rows| rows.first());
        let valid = match (leaf, use_row) {
            (Some(leaf), Some(use_row)) => {
                let atom = Atom::parse_encoded(&leaf.atom).ok();
                matches!(
                    atom.as_ref(),
                    Some(Atom::Evaluated {
                        identity: EvaluationIdentity::Site { .. },
                        ..
                    })
                ) && atom.as_ref().and_then(Atom::place) == Some(link.place.as_str())
                    && leaf.module_node_id == link.module_node_id
                    && leaf.atom_id == link.atom_id
                    && leaf.leaf_start_byte <= link.operand_start_byte
                    && link.operand_end_byte <= leaf.leaf_end_byte
                    && use_row.module_node_id == link.module_node_id
                    && use_row.fact_id == link.use_fact_id
                    && use_row.start_byte == link.operand_start_byte
                    && use_row.end_byte == link.operand_end_byte
                    && use_row.place == link.place
                    && !use_row.annotation
                    && link.role == "tested_place"
                    && link.origin
                        == i64::from(cpg_schema::Codebook::code(TestTypeOrigin::PyreflyTrace))
                    && terms.contains(&(link.term_node_id, link.term_fact_id))
            }
            _ => false,
        };
        if !valid || !seen.insert((link.leaf_fact_id, link.use_id)) {
            errors.push(format!("invalid test type link {}", link.fact_id.hex()));
        }
    }
    if errors.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "flow-test-type-proof-link".to_owned(),
            rows: errors.len(),
            sample: errors.into_iter().take(3).collect::<Vec<_>>().join("; "),
        }])
    }
}

cpg_schema::query_row! {
    struct PersistedCondition {
        condition_id: Id,
        root_id: Option<Id>,
        boundary_reason: Option<String>,
    }
}
cpg_schema::query_row! {
    struct PersistedNode {
        node_id: Id,
        atom: String,
        low_id: Id,
        high_id: Id,
    }
}
cpg_schema::query_row! {
    struct PersistedLeaf {
        module_node_id: Id,
        predicate_key: String,
        test_start_byte: i64,
        test_end_byte: i64,
        condition_id: Id,
        atom_id: Id,
        atom: String,
        leaf_start_byte: i64,
        leaf_end_byte: i64,
    }
}
cpg_schema::query_row! {
    struct PersistedSource {
        module_node_id: Id,
        path: String,
        text: Option<String>,
    }
}

cpg_schema::relations! {
    inventory relations;
    condition_contract = "validate_condition_contract",
        deps = ["conditions"],
        sql = "SELECT DISTINCT condition_id, root_id, boundary_reason FROM conditions".to_owned();
    node_contract = "validate_node_contract",
        deps = ["condition_nodes"],
        sql = "SELECT node_id, atom, low_id, high_id FROM condition_nodes".to_owned();
    leaf_contract = "validate_leaf_contract",
        deps = ["flow_test_leaves"],
        sql = "SELECT module_node_id, predicate_key, test_start_byte, test_end_byte, condition_id, atom_id, atom, leaf_start_byte, leaf_end_byte FROM flow_test_leaves".to_owned();
    source_contract = "validate_source_contract",
        deps = ["source_files"],
        sql = "SELECT module_node_id, path, text FROM source_files".to_owned();
}

/// Validate the same persisted root closure that the native query loader hydrates. The display
/// string is deliberately never consulted. Leaf evidence must identify an actual support atom
/// in that root and a source evaluation in the claimed module.
async fn validate_condition_graph(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let names = ctx
        .catalog("datafusion")
        .and_then(|c| c.schema("public"))
        .map(|s| s.table_names())
        .unwrap_or_default();
    if !names.iter().any(|n| n == "conditions") {
        return Ok(Vec::new());
    }
    let conditions: Vec<PersistedCondition> =
        sql::fetch(ctx, &condition_contract(), sql::Params::new()).await?;
    let nodes: Vec<PersistedNode> = sql::fetch(ctx, &node_contract(), sql::Params::new()).await?;
    let leaves: Vec<PersistedLeaf> = sql::fetch(ctx, &leaf_contract(), sql::Params::new()).await?;
    let sources: Vec<PersistedSource> =
        sql::fetch(ctx, &source_contract(), sql::Params::new()).await?;
    let mut errors = Vec::new();
    let nodes: Vec<DiagramNode> = nodes
        .into_iter()
        .map(|row| DiagramNode {
            node_id: row.node_id,
            atom: row.atom,
            low: row.low_id,
            high: row.high_id,
        })
        .collect();
    let conditions: Vec<ConditionRoot> = conditions
        .into_iter()
        .map(|row| ConditionRoot {
            condition_id: row.condition_id,
            root_id: row.root_id,
            boundary_reason: row.boundary_reason,
        })
        .collect();
    let diagrams = hydrate_catalog(&conditions, &nodes).unwrap_or_else(|e| {
        errors.push(e);
        HashMap::new()
    });
    let source_by_id: HashMap<Id, PersistedSource> =
        sources.into_iter().map(|s| (s.module_node_id, s)).collect();
    let mut leaf_keys = HashSet::new();
    for leaf in leaves {
        let key = (
            leaf.module_node_id,
            leaf.predicate_key.clone(),
            leaf.atom_id,
        );
        if !leaf_keys.insert(key) {
            errors.push(format!("duplicate test leaf {}", leaf.atom_id.hex()));
        }
        let Some(diagram) = diagrams.get(&leaf.condition_id) else {
            errors.push(format!(
                "test leaf has no stated root {}",
                leaf.condition_id.hex()
            ));
            continue;
        };
        if !diagram.support().contains(&leaf.atom) {
            errors.push(format!(
                "test leaf is outside root support {}",
                leaf.atom_id.hex()
            ));
        }
        if IdHasher::new("bdd-atom").str(&leaf.atom).finish_id() != leaf.atom_id {
            errors.push(format!("test leaf atom id mismatch {}", leaf.atom_id.hex()));
        }
        let Some(source) = source_by_id.get(&leaf.module_node_id) else {
            errors.push(format!(
                "test leaf has no source module {}",
                leaf.module_node_id.hex()
            ));
            continue;
        };
        let Some(text) = source.text.as_ref() else {
            errors.push(format!(
                "test leaf source is not UTF-8 {}",
                leaf.module_node_id.hex()
            ));
            continue;
        };
        let module_key = IdHasher::new("flow-evaluation-module")
            .str(&source.path)
            .str(text)
            .finish_id()
            .hex();
        if leaf.test_start_byte < 0
            || leaf.test_end_byte < leaf.test_start_byte
            || leaf.leaf_start_byte < 0
            || leaf.leaf_end_byte < leaf.leaf_start_byte
            || leaf.test_end_byte as usize > text.len()
            || leaf.leaf_end_byte as usize > text.len()
        {
            errors.push(format!(
                "test leaf span outside source {}",
                leaf.atom_id.hex()
            ));
        }
        let identity = match Atom::parse_encoded(&leaf.atom) {
            Ok(Atom::Evaluated { identity, .. }) => identity,
            _ => {
                errors.push(format!(
                    "test leaf lacks evaluation identity {}",
                    leaf.atom_id.hex()
                ));
                continue;
            }
        };
        match identity {
            EvaluationIdentity::Site { module, start, end }
                if module == module_key
                    && i64::from(start) == leaf.leaf_start_byte
                    && i64::from(end) == leaf.leaf_end_byte
                    && leaf.test_start_byte <= leaf.leaf_start_byte
                    && leaf.leaf_end_byte <= leaf.test_end_byte => {}
            EvaluationIdentity::Synthetic { module, predicate }
                if module == module_key
                    && predicate == leaf.predicate_key
                    && leaf.leaf_start_byte == leaf.test_start_byte
                    && leaf.leaf_end_byte == leaf.test_end_byte => {}
            _ => errors.push(format!(
                "test leaf identity mismatch {}",
                leaf.atom_id.hex()
            )),
        }
    }
    if errors.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![Violation {
            rule: "condition-graph-and-leaf-provenance".to_owned(),
            rows: errors.len(),
            sample: errors.into_iter().take(3).collect::<Vec<_>>().join("; "),
        }])
    }
}

async fn run(ctx: &SessionContext, rule: Rule) -> Result<(Option<Violation>, RuleCost), CoreError> {
    let started = Instant::now();
    let plan = sql::query(ctx, &rule.sql)
        .await?
        .create_physical_plan()
        .await?;
    let batches = collect(plan.clone(), ctx.task_ctx()).await?;
    let (mut compute, mut build) = (0usize, 0usize);
    visit(&plan, &mut |node| {
        if let Some(m) = node.metrics() {
            compute += m.elapsed_compute().unwrap_or(0);
            build = build.max(m.sum_by_name("build_mem_used").map_or(0, |v| v.as_usize()));
        }
    });
    let cost = RuleCost {
        rule: rule.name.clone(),
        seconds: started.elapsed().as_secs_f64(),
        compute_seconds: compute as f64 / 1e9,
        build_bytes: build,
    };
    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    if rows == 0 {
        return Ok((None, cost));
    }
    let first: Vec<_> = batches
        .iter()
        .filter(|b| b.num_rows() > 0)
        .take(1)
        .map(|b| b.slice(0, b.num_rows().min(3)))
        .collect();
    let violation = Violation {
        rule: rule.name,
        rows,
        sample: pretty_format_batches(&first)?.to_string(),
    };
    Ok((Some(violation), cost))
}

fn visit(plan: &Arc<dyn ExecutionPlan>, f: &mut impl FnMut(&dyn ExecutionPlan)) {
    f(plan.as_ref());
    for child in plan.children() {
        visit(child, f);
    }
}
