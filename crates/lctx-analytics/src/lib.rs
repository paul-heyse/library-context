//! Projections to graphs and the analysis kernels (DESIGN §5, §9; ADR-0019).
//!
//! Arrow batches in, typed rows out. Nothing here reads Delta or runs SQL: `cpg-core` runs the
//! projection queries on the attempt's session, hands the batches over, and writes the rows back
//! through the attempt's write path. So every kernel is tested on small fixtures without a store.

pub mod communities;
pub mod config;
pub mod graph;
pub mod pass_a;
pub mod pass_b;
pub mod pass_c;
pub mod ranking;

/// What an analysis refuses.
#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    #[error("analytics config: {0}")]
    Config(String),
    #[error("projection column {0} is missing or has the wrong type")]
    Column(String),
    #[error("projection {0} are not in canonical order")]
    Order(String),
    #[error("an arc or seed names {0}, which is not a vertex of the projection")]
    UnknownVertex(String),
    #[error("graph: {0}")]
    Graph(String),
}

/// `name version` of each library the analyses run, `; `-separated, read from `Cargo.lock` at
/// build time: each invocation records them (guidelines §8) and the compiler digest includes
/// them.
pub const LIBRARIES: &str = env!("LCTX_ANALYTICS_LIBRARIES");

/// [`LIBRARIES`] as a list.
pub fn libraries() -> Vec<String> {
    LIBRARIES.split("; ").map(str::to_owned).collect()
}
