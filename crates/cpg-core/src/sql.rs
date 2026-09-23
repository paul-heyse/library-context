//! The one SQL entry point (DESIGN §4.3; review F7): DDL, DML and statements are disallowed, so no
//! query can write to a table and bypass `DeltaTable::write`'s CHECK and invariant validation.

use datafusion::dataframe::DataFrame;
use datafusion::error::Result;
use datafusion::execution::context::SQLOptions;
use datafusion::prelude::SessionContext;

pub fn read_only() -> SQLOptions {
    SQLOptions::new()
        .with_allow_ddl(false)
        .with_allow_dml(false)
        .with_allow_statements(false)
}

/// Plan a read-only query. Any other code path that calls `ctx.sql` is a rule violation
/// (`rules/sql-through-helper.yml`).
pub async fn query(ctx: &SessionContext, sql: &str) -> Result<DataFrame> {
    ctx.sql_with_options(sql, read_only()).await
}

/// Run a read-only query and render its result as a table (the `lctx query` output).
pub async fn render(ctx: &SessionContext, sql: &str) -> Result<String> {
    let batches = query(ctx, sql).await?.collect().await?;
    Ok(datafusion::arrow::util::pretty::pretty_format_batches(&batches)?.to_string())
}
