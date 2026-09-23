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

/// Run every rule; an empty result means the snapshot is valid.
pub async fn validate(ctx: &SessionContext) -> Result<Vec<Violation>, CoreError> {
    let mut violations = Vec::new();
    for rule in rules() {
        let batches = sql::query(ctx, &rule.sql).await?.collect().await?;
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
    Ok(violations)
}
