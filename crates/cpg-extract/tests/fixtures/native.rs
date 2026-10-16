//! Explicit owned runtime for persisted provider controls.
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{ModelError, admission::Frontier};
use lctx_native::RuntimeConfig;
use std::{
    path::Path,
    sync::{Arc, OnceLock},
};
pub async fn create_native() -> Result<Arc<NativeCompilerStore>, ModelError> {
    let path = std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").map_err(ModelError::codec)?;
    let config = RuntimeConfig::read(Path::new(&path))?;
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    });
    tokio::task::spawn_blocking(move || {
        runtime.block_on(NativeCompilerStore::begin(
            &config,
            Frontier::Catalog,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        ))
    })
    .await
    .map_err(ModelError::codec)?
}
