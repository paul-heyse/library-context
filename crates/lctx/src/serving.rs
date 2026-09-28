//! Explicit serving publication and selection. Canonical Delta publication is independent.
use clap::Subcommand;
use cpg_core::postgres::{Config, import::Source, profiles::Policy, serving::RoleConfig};
use cpg_schema::id::{Digest, Id};
use std::path::{Path, PathBuf};
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build/verify a published snapshot's projection, then import or resume it. Never selects.
    Import {
        #[arg(long)]
        store: PathBuf,
        #[arg(long,value_parser=crate::parse_id)]
        snapshot: Id,
        #[arg(long, default_value = "build/generations")]
        bundles: PathBuf,
        #[arg(long, default_value = "build/serving-artifacts")]
        artifacts: PathBuf,
    },
    /// Import an already exported, fully verified projection bundle. Never selects.
    ImportBundle {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long, default_value = "build/serving-artifacts")]
        artifacts: PathBuf,
    },
    /// Install indexes without activating any retrieval profile.
    BuildHnsw {
        #[arg(long,value_parser=parse_digest)]
        generation: Digest,
    },
    /// Execute a frozen independent-reference pack; failed profiles remain unselectable.
    QualifyHnsw {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        serving_config: PathBuf,
    },
    Status {
        #[arg(long,value_parser=parse_digest)]
        generation: Option<Digest>,
        #[arg(long)]
        verify_artifacts: bool,
    },
    /// Register content-addressed artifacts restored under a new root. Never selects.
    RelocateArtifacts {
        #[arg(long,value_parser=parse_digest)]
        generation: Digest,
        #[arg(long)]
        artifacts: PathBuf,
    },
    /// Backup-only validated inventory at an existing exported PostgreSQL snapshot.
    RecoveryInventory {
        #[arg(long)]
        snapshot: String,
    },
    /// Pin future servers to this ready generation/profile. Existing servers retain their pin.
    Select {
        #[arg(long)]
        library: String,
        #[arg(long,value_parser=parse_digest)]
        generation: Digest,
        #[arg(long,value_parser=parse_digest)]
        profile: Option<Digest>,
    },
    /// Revalidate stored frozen rows/artifacts and reconcile readiness. Does not select.
    Reconcile {
        #[arg(long,value_parser=parse_digest)]
        generation: Digest,
    },
    /// Clean only an inactive unpublished generation; retain terminal diagnostic history.
    Cleanup {
        #[arg(long,value_parser=parse_digest)]
        generation: Digest,
    },
}
pub fn parse_digest(s: &str) -> Result<Digest, String> {
    Digest::from_hex(s).ok_or_else(|| "expected a full 64-character hexadecimal digest".into())
}
pub async fn command(
    cmd: Command,
    config: Option<&Path>,
    importer: Option<&Path>,
) -> anyhow::Result<()> {
    let path = match importer {
        Some(path) => path.to_owned(),
        None => Config::path(config)?.with_file_name("postgres-importer.json"),
    };
    let config = RoleConfig::load(&path)?;
    if let Command::Status {
        generation,
        verify_artifacts,
    } = &cmd
    {
        println!(
            "{}",
            serde_json::to_string_pretty(&config.diagnose(*generation, *verify_artifacts).await)?
        );
        return Ok(());
    }
    // Prepare and validate source before opening the bounded importer pool.
    let prepared = if let Command::Import {
        store,
        snapshot,
        bundles,
        ..
    } = &cmd
    {
        let generation = cpg_core::bundle::bundle(store, *snapshot, bundles).await?;
        Some(tokio::task::spawn_blocking(move || Source::open(&generation.dir)).await??)
    } else if let Command::ImportBundle { bundle, .. } = &cmd {
        let bundle = bundle.clone();
        Some(tokio::task::spawn_blocking(move || Source::open(&bundle)).await??)
    } else {
        None
    };
    let db = config.open_importer().await?;
    let result=async {
        match cmd {
            Command::Import{artifacts,..}|Command::ImportBundle{artifacts,..}=>println!("{}",serde_json::to_string_pretty(&db.import(prepared.expect("prepared import"),artifacts).await?)?),
            Command::BuildHnsw{generation}=>{db.build_hnsw(generation).await?;println!("HNSW indexes built; no profile selected");},
            Command::QualifyHnsw{pack,serving_config}=>{use std::io::Read;let mut bytes=Vec::new();std::fs::File::open(pack)?.take(128*1024*1024+1).read_to_end(&mut bytes)?;anyhow::ensure!(bytes.len()<=128*1024*1024,"qualification pack byte budget");let reader=RoleConfig::load(&serving_config)?.open_serving().await?;let report=db.qualify_hnsw(&reader,&bytes).await;reader.close().await;let report=report?;println!("{}",serde_json::to_string_pretty(&report)?);anyhow::ensure!(report["passed"]==true || (report["phase"]=="calibration" && !report["chosen_policy"].is_null()),"ANN profile failed qualification; exact remains available");},
            Command::Status{..}=>unreachable!("diagnostic path runs before normal schema admission"),
            Command::RelocateArtifacts{generation,artifacts}=>println!("{}",serde_json::to_string_pretty(&db.relocate_artifacts(generation,artifacts).await?)?),
            Command::RecoveryInventory{snapshot}=>println!("{}",serde_json::to_string(&db.recovery_inventory(&snapshot).await?)?),
            Command::Reconcile{generation}=>println!("{}",serde_json::to_string_pretty(&db.reconcile(generation).await?)?),
            Command::Select{library,generation,profile}=>{let profile=profile.unwrap_or(Digest::from_hex(&Policy::exact().digest()?).expect("profile digest"));db.select(&library,generation,profile).await?;println!("{}",serde_json::json!({"library":library,"generation":generation.hex(),"profile":profile.hex(),"selected":true}));},
            Command::Cleanup{generation}=>{db.cleanup(generation).await?;println!("{}",serde_json::json!({"generation":generation.hex(),"cleaned":true}));},
        }
        Ok::<_,anyhow::Error>(())
    }.await;
    db.close().await;
    result
}
