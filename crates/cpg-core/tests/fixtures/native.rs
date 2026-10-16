//! Compiler controls require the owned disposable authenticated native fixture.
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{admission::Frontier, resources::ResourceBudget};
use lctx_native::RuntimeConfig;
use std::sync::{Arc, OnceLock};

fn initialize_requested_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        let Ok(requested) = std::env::var("RUST_LOG") else { return; };
        if requested.trim().is_empty() { return; }
        let Ok(filter) = tracing_subscriber::EnvFilter::try_new(requested) else { return; };
        // Explicit diagnostics use ordinary captured stderr. An existing global
        // subscriber keeps ownership; fixture setup never replaces it.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(std::io::stderr)
            .try_init();
    });
}
#[allow(
    dead_code,
    reason = "Shared fixture serves controls that create native workspaces"
)]
pub fn store() -> Arc<NativeCompilerStore> {
    store_with_limit(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)
}
#[allow(
    dead_code,
    reason = "Shared fixture preserves each control's requested native/workspace limit"
)]
pub fn store_with_limit(memory_bytes: usize) -> Arc<NativeCompilerStore> {
    store_with_budget(
        ResourceBudget::fixed(memory_bytes).expect("owned native control recovery budget"),
    )
}
#[allow(
    dead_code,
    reason = "Shared fixture accepts the control's actual pool before native preparation"
)]
pub fn store_with_budget(budget: ResourceBudget) -> Arc<NativeCompilerStore> {
    initialize_requested_tracing();
    let path = std::env::var("LCTX_COMPILER_RUNTIME_CONFIG")
        .expect("owned disposable compiler runtime configuration is required");
    let config = RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    // Keep the SDK router and native stream-drain runtime alive across synchronous test calls.
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    });
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let span = tracing::Span::current();
    std::thread::spawn(move || {
        tracing::dispatcher::with_default(&dispatch, || {
            span.in_scope(|| {
                let factory = lctx_compiler_store::compiler::recovery_factory(&config, budget)
                    .expect("owned native control factory");
                runtime
                    .block_on(NativeCompilerStore::begin(
                        &config,
                        Frontier::Catalog,
                        &factory,
                    ))
                    .unwrap()
            })
        })
    })
    .join()
    .unwrap()
}
