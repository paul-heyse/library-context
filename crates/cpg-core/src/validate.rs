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
    Ok((violations, costs))
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
