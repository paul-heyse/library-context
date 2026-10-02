//! Explicit numerical realization. Service startup remains read-only and never selects.
use crate::{
    database::{Database, model},
    generation::parse_generation,
};
use cpg_core::postgres::generations::{GenerationId, GenerationStore};
use lctx_model::domain::{resources::ResourceBudget, serving::ResourceLimits};
#[derive(Debug, clap::Subcommand)]
pub enum ServingCommand {
    /// Derive the selected exact 1,024-dimensional vectors from a published Catalog generation.
    Prepare {
        #[arg(long,value_parser=parse_generation)]
        generation: GenerationId,
    },
}
pub async fn serving(command: ServingCommand, database: &Database) -> anyhow::Result<()> {
    match command {
        ServingCommand::Prepare { generation } => {
            let limits = ResourceLimits::default();
            let memory = ResourceBudget::fixed(limits.shared_bytes as usize)?;
            let preparation = ResourceBudget::scoped(&memory, limits.preparation_bytes as usize)?;
            let store = GenerationStore::open(database.owner().await?, model()?).await?;
            let reader = database.reader().await?;
            let result = store
                .prepare_vector_artifact(&reader, generation, preparation)
                .await;
            reader.close().await;
            let key = result?;
            println!(
                "{}",
                serde_json::json!({"generation":generation.hex(),"artifact_key":key.hex(),"selected":false})
            );
            Ok(())
        }
    }
}
