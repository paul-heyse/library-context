//! Cross-table validation (DESIGN §8): the generated rules, one read-only query each, over a
//! snapshot session. Validators only read; they never repair. Tests and publication share them.

use cpg_schema::rules::rules;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;

use crate::{CoreError, sql};

/// A rule whose query returned rows: how many, and the first few.
#[derive(Debug, Clone)]
pub struct Violation {
    pub rule: String,
    pub rows: usize,
    pub sample: String,
}

/// One rule's cost: its wall time, and how far it raised the process's peak RSS (C6).
#[derive(Debug, Clone)]
pub struct RuleCost {
    pub rule: String,
    pub seconds: f64,
    pub raised_peak_bytes: u64,
}

/// Run every rule; an empty result means the snapshot is valid.
pub async fn validate(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    Ok(validate_costed(ctx).await?.0)
}

/// [`validate`], with each rule's cost, so a stage report can name what dominates.
pub async fn validate_costed(
    ctx: &SessionContext,
) -> Result<(Vec<Violation>, Vec<RuleCost>), CoreError> {
    let mut violations = Vec::new();
    let mut costs = Vec::new();
    for rule in rules() {
        let before = cpg_schema::metrics::peak_rss_bytes().unwrap_or(0);
        let started = std::time::Instant::now();
        let batches = sql::query(ctx, &rule.sql).await?.collect().await?;
        costs.push(RuleCost {
            rule: rule.name.clone(),
            seconds: started.elapsed().as_secs_f64(),
            raised_peak_bytes: cpg_schema::metrics::peak_rss_bytes()
                .unwrap_or(0)
                .saturating_sub(before),
        });
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        if rows > 0 {
            let first: Vec<_> = batches
                .iter()
                .filter(|b| b.num_rows() > 0)
                .take(1)
                .map(|b| b.slice(0, b.num_rows().min(3)))
                .collect();
            violations.push(Violation {
                rule: rule.name,
                rows,
                sample: pretty_format_batches(&first)?.to_string(),
            });
        }
    }
    Ok((violations, costs))
}
