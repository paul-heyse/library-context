//! The one DataFusion session factory (DESIGN §15.11; cutover plan WP0.6).
//!
//! Every compute, validation and provider-read session is built here: DataFusion's default
//! features with the planner settings the pipeline has always run under (case-sensitive
//! identifiers, no hash-join IN-list pushdown, [`TARGET_PARTITIONS`]), a memory pool that bounds
//! every operator — recursive CTEs included (review F07) — and the `lctx_id` UDFs. Exhaustion is a
//! `ResourcesExhausted` error, never a partial answer.

use datafusion::execution::context::SessionContext;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::prelude::SessionConfig;

/// Partitions per plan (H1 P2): the 32-thread default doubled derivation's working set for no wall
/// time, and validation runs several plans at once. Outputs are canonically sorted, so no result
/// depends on it.
pub const TARGET_PARTITIONS: usize = 8;

/// The default operator memory pool: generous for the pilot on the operator's workstation, and a
/// bound rather than a tuning target. Phase exits record the measured peak (cutover plan §1).
pub const DEFAULT_MEMORY_LIMIT: usize = 64 * 1024 * 1024 * 1024;

/// How a session is built.
#[derive(Clone, Copy, Debug)]
pub struct SessionOptions {
    /// The operator memory pool's size in bytes.
    pub memory_limit: usize,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            memory_limit: DEFAULT_MEMORY_LIMIT,
        }
    }
}

/// The session configuration every session shares.
pub fn config() -> SessionConfig {
    SessionConfig::default()
        .set_bool("datafusion.sql_parser.enable_ident_normalization", false)
        // Hash-join dynamic filtering can panic on dictionary join keys at this DataFusion line.
        .set_usize("datafusion.optimizer.hash_join_inlist_pushdown_max_size", 0)
        .set_usize("datafusion.optimizer.hash_join_inlist_pushdown_max_distinct_values", 0)
        .with_target_partitions(TARGET_PARTITIONS)
}

/// A session with no tables.
pub fn new_session(options: SessionOptions) -> SessionContext {
    let runtime = RuntimeEnvBuilder::new()
        .with_memory_limit(options.memory_limit, 1.0)
        .build_arc()
        .expect("a memory-limited runtime");
    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_config(config())
        .with_runtime_env(runtime)
        .build();
    let ctx = SessionContext::new_with_state(state);
    ctx.register_udf(crate::udf::lctx_id());
    ctx
}

/// A session over the default options.
pub fn session() -> SessionContext {
    new_session(SessionOptions::default())
}

/// Wrap a session's state so the pool and configuration stay shared by derived sessions.
pub fn same_runtime(ctx: &SessionContext) -> SessionContext {
    SessionContext::new_with_state(ctx.state())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The configuration is pinned: a change is a reviewed change to every plan.
    #[test]
    fn the_session_configuration_is_pinned() {
        use datafusion::execution::FunctionRegistry;
        let ctx = session();
        let state = ctx.state();
        let options = state.config_options();
        assert!(!options.sql_parser.enable_ident_normalization);
        assert_eq!(options.optimizer.hash_join_inlist_pushdown_max_size, 0);
        assert_eq!(options.optimizer.hash_join_inlist_pushdown_max_distinct_values, 0);
        assert_eq!(options.execution.target_partitions, TARGET_PARTITIONS);
        assert!(ctx.udf("lctx_id").is_ok() && ctx.udf("lctx_id_v2").is_err());
    }

    /// The pool bounds every operator: a sort that cannot fit is refused, not truncated.
    #[tokio::test]
    async fn the_memory_pool_refuses_instead_of_truncating() {
        let ctx = new_session(SessionOptions { memory_limit: 64 * 1024 });
        let err = ctx
            .sql("SELECT v FROM generate_series(1, 2000000) AS t(v) ORDER BY v DESC")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Resources exhausted") || err.contains("ResourcesExhausted"), "{err}");
    }
}
