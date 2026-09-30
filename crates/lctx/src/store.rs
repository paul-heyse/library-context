//! `lctx store install|check|reset` as the verified service owner (cutover plan P1.6, P1.11).
use clap::Subcommand;
use cpg_core::postgres::generations::GenerationStore;
use crate::{Refused, database::{Database, model}};

#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    /// Apply the service baseline and install (or confirm) the generation store for this model.
    Install,
    /// Compare the live store with this binary's lowering, roles and privileges; exit 2 on findings.
    Check,
    /// Drop every generation and the control schema, then install this model. Without
    /// `--confirm <database>` this is a dry run that lists what would be dropped and exits 2.
    Reset {
        #[arg(long)]
        confirm: Option<String>,
    },
}

pub async fn store(command: StoreCommand, database: &Database) -> anyhow::Result<()> {
    let model = model()?;
    match command {
        StoreCommand::Install => {
            let migrator = database.migrator().await?;
            migrator.migrate().await?;
            let store = GenerationStore::install(migrator.owner().await?, model).await?;
            println!("installed: model {} lowering {}", store.model().digest().hex(), store.physical_digest().hex());
            migrator.close().await;
        },
        StoreCommand::Check => {
            let report = GenerationStore::check(&database.owner().await?, &model).await?;
            for finding in &report.findings { println!("{finding}"); }
            println!("{} generations, {} findings", report.generations, report.findings.len());
            if !report.clean() { return Err(Refused("the store differs from this binary's lowering".into()).into()); }
        },
        StoreCommand::Reset { confirm } => {
            let owner = database.owner().await?;
            match confirm {
                None => {
                    let plan = GenerationStore::reset_plan(&owner).await?;
                    println!("database {}: would drop {} generation schemas{}", plan.database, plan.schemas.len(),
                        if plan.control { " and the control schema" } else { "" });
                    for schema in &plan.schemas { println!("  {schema}"); }
                    return Err(Refused(format!("dry run; confirm with --confirm {}", plan.database)).into());
                },
                Some(confirm) => {
                    let (dropped, store) = GenerationStore::reset(owner, model, &confirm).await?;
                    println!("reset {}: dropped {} generation schemas; installed model {}", dropped.database, dropped.schemas.len(), store.model().digest().hex());
                },
            }
        },
    }
    Ok(())
}
