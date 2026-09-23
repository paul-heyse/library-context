//! The binaries' log subscriber (H1 O1): Pyrefly, delta-kernel and DataFusion (its `log` records,
//! through the tracing-log bridge) report through it, to stderr.

/// `warn`, except delta-rs's writer statistics, which warn once per Binary column per write that
/// it records no Delta-log statistics for them (256 lines per pilot compile). That limit is known,
/// documented (§4.3) and designed around (per-commit reads, ADR-0017), so it would only bury the
/// warnings the subscriber exists to show (H1 review F6).
pub const DEFAULT_LOG_FILTER: &str = "warn,deltalake_core::writer::stats=error";

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
    fn the_default_filter_parses_and_quiets_the_known_stats_warning() {
        let filter = tracing_subscriber::EnvFilter::try_new(DEFAULT_LOG_FILTER).unwrap();
        let shown = filter.to_string();
        assert!(
            shown.contains("deltalake_core::writer::stats=error"),
            "{shown}"
        );
        assert!(shown.contains("warn"), "{shown}");
    }
}
