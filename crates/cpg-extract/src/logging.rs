//! The binaries' log subscriber (H1 O1): Pyrefly and DataFusion (its `log` records, through the
//! tracing-log bridge) report through it, to stderr.

/// Warnings plus coarse phase evidence. `LCTX_LOG` can select a different output boundary.
pub const DEFAULT_LOG_FILTER: &str = "warn,lctx_phase=info";

/// Construct a stderr subscriber without changing the process-global subscriber. Async callers
/// attach this dispatch with `WithSubscriber` so each poll has its own attribution.
pub fn dispatch() -> tracing::Dispatch {
    tracing::Dispatch::new(subscriber())
}

fn subscriber() -> impl tracing::Subscriber + Send + Sync {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LCTX_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_LOG_FILTER));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .finish()
}

/// Install the subscriber. `LCTX_LOG` replaces [`DEFAULT_LOG_FILTER`] (`LCTX_LOG=debug`); it
/// changes output only, never an identity.
pub fn init_logging() {
    use tracing_subscriber::util::SubscriberInitExt;
    let _ = subscriber().try_init();
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_LOG_FILTER;

    #[test]
    fn the_default_filter_parses_and_shows_warnings() {
        let filter = tracing_subscriber::EnvFilter::try_new(DEFAULT_LOG_FILTER).unwrap();
        assert_eq!(filter.to_string(), "lctx_phase=info,warn");
    }
}
