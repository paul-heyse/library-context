//! Cross-table validation (DESIGN §8): the generated rules, one read-only query each, over a
//! snapshot session. Validators only read; they never repair. Tests and publication share them.
//!
//! H1 P2: the rules read a cached copy of the session. Every table is read once through its
//! pinned, snapshot-filtered Delta view into memory, and the rules run [`CONCURRENT_RULES`] at a
//! time. Each rule is still one query, with the same SQL and name; only the provider behind each
//! table name differs.

use std::sync::Arc;
use std::time::Instant;

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
    Ok((violations, costs))
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
