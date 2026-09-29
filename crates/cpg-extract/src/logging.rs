//! The binaries' log subscriber (H1 O1): Pyrefly and DataFusion (its `log` records, through the
//! tracing-log bridge) report through it, to stderr.

/// `warn`. The delta-rs statistics filters left with the Delta store (cutover plan P1.4).
pub const DEFAULT_LOG_FILTER: &str = "warn";

/// Install the subscriber. `LCTX_LOG` replaces [`DEFAULT_LOG_FILTER`] (`LCTX_LOG=debug`); it
/// changes output only, never an identity.
pub fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LCTX_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_LOG_FILTER;

    #[test]
    fn the_default_filter_parses_and_shows_warnings() {
        let filter = tracing_subscriber::EnvFilter::try_new(DEFAULT_LOG_FILTER).unwrap();
        assert_eq!(filter.to_string(), "warn");
    }
}
