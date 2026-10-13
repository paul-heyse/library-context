use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Development-only native schema and SurrealKit worker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print internal compiled provenance without reading a binding or connecting.
    BuildInfo,
    /// Regenerate derived assets, or check exact file/content correspondence.
    Generate {
        #[arg(long, default_value = env!("CARGO_MANIFEST_DIR"))]
        root: PathBuf,
        #[arg(long)]
        check: bool,
    },
    /// Materialize dependency-closed schema into an already owned empty test project.
    Project {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,
        #[arg(long)]
        sample_suite: bool,
    },
    /// Apply generated definitions only under a host-issued whole-campaign binding.
    Sync {
        #[arg(long)]
        binding: PathBuf,
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,
    },
    /// Run beta's public declarative tester; parent owns all target disposal.
    Test {
        #[arg(long)]
        binding: PathBuf,
        #[arg(long)]
        prepare_project: bool,
        #[arg(long)]
        sample_suite: bool,
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::BuildInfo => {
            println!("{}", serde_json::to_string(&lctx_surreal_dev::build_info()?)?);
            Ok(())
        }
        Command::Generate { root, check } => lctx_surreal_dev::generate(&root, check),
        Command::Project {
            output,
            modules,
            sample_suite,
        } => {
            lctx_surreal_dev::project(&output, &modules)?;
            if sample_suite {
                lctx_surreal_dev::sample::write(&output)?;
            }
            Ok(())
        }
        Command::Sync { binding, modules } => {
            let (binding, runtime) =
                lctx_surreal_dev::worker::TargetBinding::read(&binding, "sync")?;
            lctx_surreal_dev::worker::sync(&binding, &runtime, &modules).await
        }
        Command::Test {
            binding,
            prepare_project,
            sample_suite,
            modules,
        } => {
            let (binding, runtime) =
                lctx_surreal_dev::worker::TargetBinding::read(&binding, "test")?;
            if !prepare_project && (sample_suite || !modules.is_empty()) {
                anyhow::bail!("sample/module options require --prepare-project");
            }
            if sample_suite
                && !lctx_surreal_dev::selected_modules(&modules)?.contains(&"completion")
            {
                anyhow::bail!(
                    "sample suite requires the completion responsibility and its dependencies"
                );
            }
            if prepare_project {
                lctx_surreal_dev::project(&binding.project_dir, &modules)?;
            }
            if sample_suite {
                lctx_surreal_dev::sample::write(&binding.project_dir)?;
            }
            lctx_surreal_dev::worker::test(&binding, &runtime).await
        }
    }
}
