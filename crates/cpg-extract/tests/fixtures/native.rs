//! Explicit owned runtime for persisted provider controls.
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{ModelError, admission::Frontier, resources::ResourceBudget};
use lctx_native::RuntimeConfig;
use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

fn initialize_requested_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        let Ok(requested) = std::env::var("RUST_LOG") else {
            return;
        };
        if requested.trim().is_empty() {
            return;
        }
        let Ok(filter) = tracing_subscriber::EnvFilter::try_new(requested) else {
            return;
        };
        // Match the compiler fixture: explicit diagnostics use captured stderr and
        // never replace a subscriber already owned by the test.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(std::io::stderr)
            .try_init();
    });
}

pub async fn create_native(
    budget: &ResourceBudget,
) -> Result<Arc<NativeCompilerStore>, ModelError> {
    initialize_requested_tracing();
    let budget = budget.clone();
    let path = std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").map_err(ModelError::codec)?;
    let config = RuntimeConfig::read(Path::new(&path))?;
    // Some async controls inspect completed rows through the synchronous typed iterator.
    // Its receiver blocks the caller's current-thread runtime, so the SDK router must have
    // its own live executor until explicit native finality and connection joining complete.
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    });
    let tracing = lctx_tracing::Context::current();
    tokio::task::spawn_blocking(move || {
        tracing.scope(move || {
            let factory = lctx_compiler_store::compiler::recovery_factory(&config, budget)?;
            runtime.block_on(NativeCompilerStore::begin(
                &config,
                Frontier::Catalog,
                &factory,
            ))
        })
    })
    .await
    .map_err(ModelError::codec)?
}
