//! `lctx generation list|show` as the reader, and `select|clear-selection|retire|abort` as the
//! service owner (cutover plan P1.8, P1.11). Compile never selects; selection is this explicit step.
use crate::{
    Refused,
    database::{Database, model},
};
use clap::Subcommand;
use cpg_core::postgres::generations::{
    GenerationCatalog, GenerationDetail, GenerationId, GenerationState, GenerationStore,
    GenerationSummary, ListFilter,
};
use lctx_model::domain::admission::Frontier;
use serde_json::{Value, json};

pub fn parse_generation(s: &str) -> Result<GenerationId, String> {
    GenerationId::from_hex(s)
        .ok_or_else(|| format!("{s:?} is not a generation id (32 lowercase hex digits)"))
}
fn parse_state(s: &str) -> Result<GenerationState, String> {
    [
        GenerationState::Staging,
        GenerationState::Sealed,
        GenerationState::Validated,
        GenerationState::Published,
        GenerationState::Failed,
    ]
    .into_iter()
    .find(|state| state.name() == s)
    .ok_or_else(|| format!("{s:?} is not a generation state"))
}
fn parse_frontier(s: &str) -> Result<Frontier, String> {
    Frontier::ALL
        .into_iter()
        .find(|f| f.name() == s)
        .ok_or_else(|| format!("{s:?} is not a frontier"))
}

#[derive(Debug, Subcommand)]
pub enum GenerationCommand {
    /// Generations with their state, frontier, profile, selection, readers and writer.
    List {
        #[arg(long, value_parser = parse_state)]
        state: Option<GenerationState>,
        #[arg(long, value_parser = parse_frontier)]
        frontier: Option<Frontier>,
    },
    /// One generation's digests, receipts, admission and failure.
    Show {
        #[arg(value_parser = parse_generation)]
        generation: GenerationId,
    },
    /// Point the selection at a published facts generation.
    Select {
        #[arg(value_parser = parse_generation)]
        generation: GenerationId,
    },
    ClearSelection,
    /// Remove an unselected published generation that no reader leases.
    Retire {
        #[arg(value_parser = parse_generation)]
        generation: GenerationId,
    },
    /// Remove a failed or interrupted generation.
    Abort {
        #[arg(value_parser = parse_generation)]
        generation: GenerationId,
    },
}

fn summary(row: &GenerationSummary) -> Value {
    json!({ "id": row.id.hex(), "state": row.state.name(), "frontier": row.frontier.name(), "profile": row.profile.name(), "selected": row.selected,
        "created_at": row.created_at, "readers": row.readers, "writer": format!("{:?}", row.writer).to_lowercase() })
}
fn detail(detail: &GenerationDetail) -> Value {
    let mut shown = summary(&detail.summary);
    shown["model_digest"] = json!(detail.model.hex());
    shown["physical_digest"] = json!(detail.physical.hex());
    shown["producer_digest"] = json!(detail.producer.hex());
    shown["schedule_digest"] = json!(detail.schedule.map(|d| d.hex()));
    shown["content_digest"] = json!(detail.content.map(|d| d.hex()));
    shown["relations"] = json!(
        detail
            .relations
            .iter()
            .map(|(name, rows)| json!({ "relation": name, "rows": rows }))
            .collect::<Vec<_>>()
    );
    if let Some((contract, families)) = &detail.admission {
        shown["admission"] = json!({ "contract_digest": contract.hex(),
            "families": families.iter().map(|(family, availability)| json!({ "family": format!("{family:?}"), "availability": format!("{availability:?}") })).collect::<Vec<_>>() });
    }
    if let Some((from, class, message)) = &detail.failure {
        shown["failure"] =
            json!({ "from_state": from.name(), "class": class.name(), "detail": message });
    }
    shown
}

pub async fn generation(command: GenerationCommand, database: &Database) -> anyhow::Result<()> {
    match command {
        GenerationCommand::List { state, frontier } => {
            let catalog = GenerationCatalog::new(database.reader().await?);
            let rows = catalog.list(&ListFilter { state, frontier }).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&rows.iter().map(summary).collect::<Vec<_>>())?
            );
        }
        GenerationCommand::Show { generation } => {
            let catalog = GenerationCatalog::new(database.reader().await?);
            let shown = catalog
                .show(generation)
                .await?
                .ok_or_else(|| Refused(format!("no generation {}", generation.hex())))?;
            let mut document=detail(&shown);
            if shown.summary.state==GenerationState::Published {
                let serving=database.serving()?;
                let model=model()?;
                let session=cpg_core::generation_read::GenerationSession::open(&serving,model.clone(),generation,cpg_core::generation_read::ProviderOptions {connections:serving.provider_connections,..Default::default()}).await?;
                let report=cpg_core::analysis_report::read(session,&model).await?;
                document["analysis"]=serde_json::to_value(&report)?;
                println!("{}",serde_json::to_string_pretty(&document)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&document)?);
            }
        }
        owner_command => {
            let store = GenerationStore::open(database.owner().await?, model()?).await?;
            match owner_command {
                GenerationCommand::Select { generation } => {
                    store.select(generation).await?;
                    println!("selected {}", generation.hex());
                }
                GenerationCommand::ClearSelection => {
                    store.clear_selection().await?;
                    println!("selection cleared");
                }
                GenerationCommand::Retire { generation } => {
                    println!("{:?} {}", store.retire(generation).await?, generation.hex())
                }
                GenerationCommand::Abort { generation } => {
                    println!("{:?} {}", store.abort(generation).await?, generation.hex())
                }
                GenerationCommand::List { .. } | GenerationCommand::Show { .. } => {
                    unreachable!("reader commands handled above")
                }
            }
        }
    }
    Ok(())
}
