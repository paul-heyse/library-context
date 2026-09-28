use anyhow::{Context, bail};
use clap::Subcommand;
use cpg_core::postgres::{Config, Store};
use cpg_schema::id::{Digest, Id, content_digest};
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Apply embedded SQL migrations using the migration identity.
    Migrate,
    /// Show server, effective identity, schema compatibility and pool state.
    Status,
    /// Refuse an unsupported schema; this command never migrates.
    Check,
    /// Rebuild discovery rows from authoritative Delta publications and verified generations.
    Reconcile {
        #[arg(long)]
        store: PathBuf,
        #[arg(long, default_value = "build/generations")]
        generations: PathBuf,
    },
    /// Import only legacy vectors whose exact request texts can be re-admitted.
    ImportCache {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        version: u64,
        /// JSON array of exact request texts reconstructed from pinned inputs.
        #[arg(long)]
        requests: PathBuf,
        #[arg(long, value_enum)]
        embedder: crate::EmbedderChoice,
        #[arg(long, default_value = "http://127.0.0.1:8000")]
        embed_url: String,
    },
}

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

#[derive(Debug, Subcommand)]
pub enum Snapshots {
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    Show {
        #[arg(value_parser=crate::parse_id)]
        snapshot: Id,
    },
}

#[derive(Debug, Subcommand)]
pub enum Generations {
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    Show {
        key: String,
    },
}

pub async fn connect(config: Option<&Path>) -> anyhow::Result<Store> {
    let config = Config::load(&Config::path(config)?)?;
    Ok(config.connect_application().await?)
}

pub async fn command(command: Command, config: Option<&Path>) -> anyhow::Result<()> {
    if matches!(command, Command::Migrate) {
        let settings = Config::load(&Config::path(config)?)?;
        let db = settings.connect_migrator().await?;
        db.migrate().await?;
        db.check().await?;
        db.close().await;
        println!("PostgreSQL migrations applied; schema current");
        return Ok(());
    }
    let db = connect(config).await?;
    match command {
        Command::Migrate => unreachable!("handled before opening application pool"),
        Command::Check => {
            db.check().await?;
            println!("PostgreSQL 18 schema check passed");
        }
        Command::Status => println!("{}", serde_json::to_string_pretty(&db.health().await?)?),
        Command::Reconcile { store, generations } => {
            db.check().await?;
            let before = db.reconciliation_started().await?;
            let root = existing_or_absolute(&store)?;
            let generations = existing_or_absolute(&generations)?;
            let mut snapshots = std::collections::BTreeMap::new();
            for row in cpg_core::snapshot::catalog(&root).await? {
                if snapshots
                    .insert(row.snapshot_id, (row.content_digest, row.compiler_digest))
                    .is_some_and(|prior| prior != (row.content_digest, row.compiler_digest))
                {
                    bail!("inconsistent Delta publication metadata");
                }
            }
            for (id, (content, compiler)) in &snapshots {
                db.observe_publication(&root.to_string_lossy(), *id, *content, *compiler)
                    .await?;
            }
            let mut locations = Vec::new();
            if generations.exists() {
                for entry in std::fs::read_dir(&generations)? {
                    let path = entry?.path();
                    if path.join("MANIFEST.json").is_file() {
                        let manifest = match cpg_core::bundle::verify(&path) {
                            Ok(manifest) => manifest,
                            Err(_) => {
                                eprintln!(
                                    "discovery skipped an invalid generation at {}",
                                    path.display()
                                );
                                continue;
                            }
                        };
                        let id = manifest["snapshot_id"].as_str().and_then(Id::from_hex);
                        // A shared generation root may contain preserved baseline/other stores.
                        if !id.is_some_and(|id| snapshots.contains_key(&id)) {
                            continue;
                        }
                        record_generation(&db, &root, &path).await?;
                        locations.push(std::fs::canonicalize(path)?.to_string_lossy().into_owned());
                    }
                }
            }
            db.finish_reconciliation(
                &root.to_string_lossy(),
                &generations.to_string_lossy(),
                &before,
                &snapshots.keys().copied().collect::<Vec<_>>(),
                &locations,
            )
            .await?;
            println!(
                "reconciled {} snapshots and {} verified generations",
                snapshots.len(),
                locations.len()
            );
        }
        Command::ImportCache {
            store,
            version,
            requests,
            embedder,
            embed_url,
        } => {
            db.check().await?;
            let embedder = crate::embedder_of(embedder, &embed_url)
                .context("cache import needs a tokenizer/spec, not --embedder none")?;
            let texts: Vec<String> = serde_json::from_slice(&std::fs::read(requests)?)?;
            let imported = cpg_core::postgres::legacy::import(
                &db,
                &crate::absolute(&store)?,
                version,
                embedder.as_ref(),
                &texts,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&imported)?);
        }
    }
    db.close().await;
    Ok(())
}

pub async fn runs(command: Runs, config: Option<&Path>) -> anyhow::Result<()> {
    let db = connect(config).await?;
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

pub async fn snapshots(command: Snapshots, config: Option<&Path>) -> anyhow::Result<()> {
    let db = connect(config).await?;
    db.check().await?;
    let (id, limit, offset) = match command {
        Snapshots::List { limit, offset } => (None, limit, offset),
        Snapshots::Show { snapshot } => (Some(snapshot), 1000, 0),
    };
    let rows = db.snapshots(id, limit, offset).await?;
    for row in &rows {
        let id = Id::from_hex(&row.snapshot).context("invalid discovery snapshot")?;
        let canonical = cpg_core::snapshot::catalog(Path::new(&row.store_path)).await?;
        let found: Vec<_> = canonical.iter().filter(|r| r.snapshot_id == id).collect();
        if found.is_empty()
            || found.iter().any(|r| {
                r.content_digest.hex() != row.content_digest
                    || r.compiler_digest.hex() != row.compiler_digest
            })
        {
            bail!("stale snapshot discovery row; run lctx db reconcile");
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    db.close().await;
    Ok(())
}

pub async fn generations(command: Generations, config: Option<&Path>) -> anyhow::Result<()> {
    let db = connect(config).await?;
    db.check().await?;
    let (key, limit, offset) = match command {
        Generations::List { limit, offset } => (None, limit, offset),
        Generations::Show { key } => (Some(key), 1000, 0),
    };
    let rows = db.generations(key.as_deref(), limit, offset).await?;
    for row in &rows {
        let manifest =
            verified_generation(Path::new(&row.store_path), Path::new(&row.location)).await?;
        let bytes = std::fs::read(Path::new(&row.location).join("MANIFEST.json"))?;
        if manifest["generation"].as_str() != Some(&row.generation_key)
            || manifest["snapshot_id"].as_str() != Some(&row.snapshot)
            || content_digest(&bytes).hex() != row.manifest_digest
        {
            bail!("stale generation discovery row; run lctx db reconcile");
        }
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    db.close().await;
    Ok(())
}

pub async fn record_generation(db: &Store, store: &Path, path: &Path) -> anyhow::Result<()> {
    let path = std::fs::canonicalize(path)?;
    let store = existing_or_absolute(store)?;
    let manifest = verified_generation(&store, &path).await?;
    let snapshot = Id::from_hex(
        manifest["snapshot_id"]
            .as_str()
            .context("manifest snapshot")?,
    )
    .context("manifest snapshot id")?;
    let key = manifest["generation"]
        .as_str()
        .context("manifest generation")?;
    let digest: Digest = content_digest(&std::fs::read(path.join("MANIFEST.json"))?);
    db.observe_publication(
        &store.to_string_lossy(),
        snapshot,
        Digest::from_hex(
            manifest["content_digest"]
                .as_str()
                .context("manifest content")?,
        )
        .context("manifest content digest")?,
        Digest::from_hex(
            manifest["compiler_digest"]
                .as_str()
                .context("manifest compiler")?,
        )
        .context("manifest compiler digest")?,
    )
    .await?;
    db.record_generation(
        &path.to_string_lossy(),
        &store.to_string_lossy(),
        key,
        snapshot,
        digest,
    )
    .await?;
    db.event(
        snapshot,
        &format!("reconcile/generated/{key}"),
        "generated",
        "verified generation observed; historical creation time unknown",
    )
    .await?;
    Ok(())
}

fn existing_or_absolute(path: &Path) -> anyhow::Result<PathBuf> {
    if path.exists() {
        Ok(std::fs::canonicalize(path)?)
    } else {
        crate::absolute(path)
    }
}

async fn verified_generation(store: &Path, path: &Path) -> anyhow::Result<serde_json::Value> {
    let manifest = cpg_core::bundle::verify(path)?;
    let snapshot = Id::from_hex(
        manifest["snapshot_id"]
            .as_str()
            .context("manifest snapshot")?,
    )
    .context("manifest snapshot id")?;
    let rows = cpg_core::snapshot::catalog(store).await?;
    let matching: Vec<_> = rows.iter().filter(|r| r.snapshot_id == snapshot).collect();
    if matching.is_empty()
        || matching.iter().any(|r| {
            Some(r.content_digest.hex().as_str()) != manifest["content_digest"].as_str()
                || Some(r.compiler_digest.hex().as_str()) != manifest["compiler_digest"].as_str()
        })
    {
        bail!("generation is not backed by matching canonical publication in selected store");
    }
    Ok(manifest)
}
