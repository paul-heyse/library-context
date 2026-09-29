//! Explicit published-input rebuild commands; publication never implies operator selection.
use super::{EmbedderChoice, absolute, embedder_of, parse_id};
use clap::Subcommand;
use cpg_schema::Id;
use std::path::{Path, PathBuf};
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Rebuild catalog/selected enrichment from captured facts; a fresh snapshot is validated.
    Catalog {
        #[arg(long)]
        store: PathBuf,
        #[arg(long,value_parser=parse_id)]
        snapshot: Id,
        #[arg(long)]
        out: PathBuf,
        /// Required for behavioral input; must describe the selected analytics configuration.
        #[arg(long)]
        analytics_config: Option<PathBuf>,
        #[arg(long, default_value = "default", allow_hyphen_values = true)]
        analytics: String,
        /// Override declared public roots using the already captured facts.
        #[arg(long)]
        public_root: Vec<String>,
        #[arg(long, value_enum, default_value = "none")]
        embedder: EmbedderChoice,
        #[arg(long, default_value = "http://127.0.0.1:8000")]
        embed_url: String,
        /// Force clean recomputation as a qualification oracle.
        #[arg(long)]
        clean: bool,
    },
    /// Rebuild only retrieval/rendering/embedding and its serving generation.
    Retrieval {
        #[arg(long)]
        store: PathBuf,
        #[arg(long,value_parser=parse_id)]
        snapshot: Id,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value = "none")]
        embedder: EmbedderChoice,
        #[arg(long, default_value = "http://127.0.0.1:8000")]
        embed_url: String,
    },
}
impl Command {
    pub fn run(self, database: Option<&Path>) -> anyhow::Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let (store, snapshot, out, choice, url) = match &self {
                Self::Catalog {
                    store,
                    snapshot,
                    out,
                    embedder,
                    embed_url,
                    ..
                }
                | Self::Retrieval {
                    store,
                    snapshot,
                    out,
                    embedder,
                    embed_url,
                } => (
                    absolute(store)?,
                    *snapshot,
                    absolute(out)?,
                    *embedder,
                    embed_url,
                ),
            };
            let embedder = embedder_of(choice, url);
            let cache = if embedder.is_some() {
                let db = super::db::connect(database).await?;
                db.check().await?;
                Some(db)
            } else {
                None
            };
            let (generation, receipt) = match self {
                Self::Retrieval { .. } => {
                    cpg_core::rebuild::retrieval(&store, snapshot, &out, embedder.as_deref(), cache)
                        .await?
                }
                Self::Catalog {
                    analytics_config,
                    analytics,
                    public_root,
                    clean,
                    ..
                } => {
                    let original = cpg_core::rebuild::configuration(&store, snapshot).await?;
                    let profile = if original.profile == "behavioral" {
                        cpg_schema::catalog::CompileProfile::Behavioral
                    } else {
                        cpg_schema::catalog::CompileProfile::Catalog
                    };
                    let roots = if public_root.is_empty() {
                        original.public_roots
                    } else {
                        public_root
                    };
                    let analysis = if profile.behavioral() {
                        let config = analytics_config.ok_or_else(|| {
                            anyhow::anyhow!("behavioral rebuild requires --analytics-config")
                        })?;
                        Some(cpg_core::analyze::Analysis {
                            config: lctx_analytics::config::AnalyticsConfig::load(&config)?,
                            techniques: cpg_core::analyze::Techniques::parse(&analytics)
                                .map_err(anyhow::Error::msg)?,
                            embedder: embedder.clone(),
                            embedding_cache: cache.clone(),
                        })
                    } else {
                        None
                    };
                    let inputs = cpg_core::catalog::CompileInputs {
                        public_roots: roots,
                        profile,
                        embedder: embedder.clone(),
                        embedding_cache: cache.clone(),
                    };
                    let target = super::random_id()?;
                    println!("attempt  {}", target.hex());
                    let (published, mut receipt) = cpg_core::rebuild::catalog(
                        &store,
                        snapshot,
                        target,
                        &inputs,
                        analysis.as_ref(),
                        clean,
                    )
                    .await?;
                    let generation = cpg_core::bundle::bundle_with_embedding(
                        &store,
                        published.snapshot_id,
                        &out,
                        embedder.as_deref(),
                        cache,
                    )
                    .await?;
                    receipt.steps.push(cpg_core::rebuild::Step {
                        stage: cpg_core::rebuild::Stage::Retrieval,
                        dependencies: vec![cpg_core::rebuild::Stage::CatalogFinalization],
                        key: cpg_schema::IdHasher::new("retrieval-realization-v1")
                            .str(&generation.key)
                            .finish_digest(),
                        outcome: cpg_core::rebuild::Outcome::Recomputed,
                    });
                    (generation, receipt)
                }
            };
            // Report is explanatory, never consulted to admit reuse or readiness.
            println!("rebuild {}", serde_json::to_string(&receipt)?);
            println!("generation {}", generation.dir.display());
            Ok::<_, anyhow::Error>(())
        })
    }
}
