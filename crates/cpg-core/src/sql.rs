//! The one SQL entry point (DESIGN §4.3; review F7): DDL, DML and statements are disallowed, so no
//! compute query can write to a completed input and bypass its compiler owner.

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

