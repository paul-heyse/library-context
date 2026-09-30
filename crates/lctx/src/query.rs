//! `lctx query --generation <id> <SQL>`: read-only DataFusion SQL over one leased generation
//! (cutover plan P1.10, P1.11). DDL, DML and statements are refused; a relation outside the
//! generation's frontier is a refusal, never an empty table.
use cpg_core::generation_read::{GenerationSession, InspectionSession, ProviderOptions, ReadError};
use cpg_core::postgres::generations::GenerationId;
use datafusion::error::DataFusionError;
use crate::{Refused, database::{Database, model}};

/// A planning or execution error the read contract refuses, rather than one that failed.
fn refusal(error: &DataFusionError) -> Option<String> {
    match error.find_root() {
        DataFusionError::External(inner) => inner.downcast_ref::<ReadError>()
            .filter(|read| matches!(read, ReadError::Frontier(_) | ReadError::ReadOnly(_))).map(ToString::to_string),
        _ => None,
    }
}

pub async fn query(database: &Database, generation: GenerationId, sql: &str) -> anyhow::Result<()> {
    let serving = database.serving()?;
    let options = ProviderOptions { connections: serving.provider_connections, ..ProviderOptions::default() };
    let session = GenerationSession::open(&serving, model()?, generation, options).await?;
    let inspection = InspectionSession::new(session)?;
    let result = async { inspection.sql(sql).await?.collect().await }.await;
    inspection.close().await?;
    match result {
        Ok(batches) => {
            println!("{}", datafusion::arrow::util::pretty::pretty_format_batches(&batches)?);
            Ok(())
        },
        Err(error) => match refusal(&error) {
            Some(reason) => Err(Refused(reason).into()),
            None => Err(error.into()),
        },
    }
}
