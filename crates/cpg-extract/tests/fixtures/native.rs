//! Explicit owned runtime for persisted provider controls.
use std::{sync::{Arc,OnceLock},path::Path};
use lctx_surrealdb::{RuntimeConfig,compiler::NativeCompilerStore};
use lctx_model::domain::{ModelError,admission::Frontier};
pub async fn create_native() -> Result<Arc<NativeCompilerStore>,ModelError> {
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").map_err(ModelError::codec)?;
    let config=RuntimeConfig::read(Path::new(&path))?;
    static RUNTIME:OnceLock<tokio::runtime::Runtime>=OnceLock::new();
    let runtime=RUNTIME.get_or_init(||tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap());
    tokio::task::spawn_blocking(move ||runtime.block_on(NativeCompilerStore::begin(&config,Frontier::Catalog))).await.map_err(ModelError::codec)?
}
