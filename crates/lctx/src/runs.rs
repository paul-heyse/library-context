//! Operational compile-attempt history in the retained `lctx_ops` services (plan P1.1).
use anyhow::{Context, bail};
use clap::Subcommand;
use cpg_schema::id::Id;
use crate::database::Database;

#[derive(Debug, Subcommand)]
pub enum Runs {
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    Show {
        #[arg(value_parser=crate::parse_id)]
        attempt: Id,
        #[arg(long, default_value = "100")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Explicit operator reconciliation after confirming the process was interrupted.
    MarkInterrupted {
        #[arg(value_parser=crate::parse_id)]
        attempt: Id,
    },
}

pub async fn runs(command: Runs, database: &Database) -> anyhow::Result<()> {
    let db = database.application().await?;
    db.check().await?;
    match command {
        Runs::List { limit, offset } => println!(
            "{}",
            serde_json::to_string_pretty(&db.runs(None, limit, offset).await?)?
        ),
        Runs::Show {
            attempt,
            limit,
            offset,
        } => println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"attempt":db.runs(Some(attempt),1,0).await?,"events":db.events(attempt,limit,offset).await?})
            )?
        ),
        Runs::MarkInterrupted { attempt } => {
            let run = db
                .runs(Some(attempt), 1, 0)
                .await?
                .pop()
                .context("unknown attempt")?;
            if run.outcome != "unfinished" {
                bail!("only unfinished attempts can be reconciled as interrupted");
            }
            db.event(
                attempt,
                "operator/interrupted",
                "interrupted",
                "explicit operator reconciliation",
            )
            .await?;
            println!("attempt marked interrupted by explicit operator reconciliation");
        }
    }
    db.close().await;
    Ok(())
}
