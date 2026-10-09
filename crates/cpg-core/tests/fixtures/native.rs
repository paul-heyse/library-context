//! Compiler controls require the owned disposable authenticated native fixture.
use lctx_model::domain::admission::Frontier;
use lctx_surrealdb::{RuntimeConfig, compiler::NativeCompilerStore};
use std::sync::{Arc, OnceLock};
#[allow(
    dead_code,
    reason = "Shared fixture serves controls that create native workspaces"
)]
pub fn store() -> Arc<NativeCompilerStore> {
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
                runtime
                    .block_on(NativeCompilerStore::begin(&config, Frontier::Catalog))
                    .unwrap()
            })
        })
    })
    .join()
    .unwrap()
}
