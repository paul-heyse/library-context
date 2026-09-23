//! `lctx-extract`: run one extraction and write each raw table as `<out>/<table>.arrow`, for
//! debugging the raw facts. Production compiles go through `lctx compile` (ADR-0013).
//!
//! Usage, an acquired library (`lctx acquire` built the environment):
//!   lctx-extract --library DIR --env DIR --snapshot HEX32 --out DIR
//! or a source tree compiled under a label:
//!   lctx-extract --release-root DIR --release-label TEXT --venv-root DIR --site-packages DIR
//!                [--site-packages DIR] --snapshot HEX32 --out DIR [--python 3.14.0] [--platform linux]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{ArgGroup, Parser};
use cpg_extract::{ExtractInput, Release, extract, library, write_ipc};
use cpg_schema::id::Id;

/// jemalloc, not glibc malloc (ADR-0016): glibc's per-thread arenas retained about half of a 6.1-7.3
/// GB pilot peak; jemalloc peaks at the working set (3.6 GB) and is no slower. Pyrefly's own CLI
/// uses it on the same platforms.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// The command line (H1 C4: clap derive).
#[derive(Parser, Debug)]
#[command(
    name = "lctx-extract",
    about = "Extract one release's raw tables to Arrow IPC files"
)]
#[command(group(ArgGroup::new("what").required(true).args(["library", "release_root"])))]
struct Args {
    /// An acquired library's definition directory (with `--env`).
    #[arg(long, requires = "env")]
    library: Option<PathBuf>,
    /// The library's acquired environment.
    #[arg(long)]
    env: Option<PathBuf>,
    /// A source tree compiled under a label (with `--release-label` and `--venv-root`).
    #[arg(long, requires_all = ["label", "venv_root"])]
    release_root: Option<PathBuf>,
    #[arg(long = "release-label")]
    label: Option<String>,
    #[arg(long)]
    venv_root: Option<PathBuf>,
    #[arg(long)]
    site_packages: Vec<PathBuf>,
    /// The snapshot id: 32 hex digits.
    #[arg(long, value_parser = parse_id)]
    snapshot: Id,
    #[arg(long)]
    out: PathBuf,
    /// MAJOR.MINOR.MICRO (default 3.14.0).
    #[arg(long, value_parser = parse_python)]
    python: Option<(u32, u32, u32)>,
    /// Default `linux`.
    #[arg(long)]
    platform: Option<String>,
}

fn parse_id(s: &str) -> Result<Id, String> {
    Id::from_hex(s).ok_or_else(|| format!("{s:?} is not 32 hex digits"))
}

fn parse_python(s: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<u32> = s
        .split('.')
        .map(|p| p.parse().map_err(|_| format!("{s}: not MAJOR.MINOR.MICRO")))
        .collect::<Result<_, _>>()?;
    let [x, y, z] = parts[..] else {
        return Err(format!("{s}: not MAJOR.MINOR.MICRO"));
    };
    Ok((x, y, z))
}

fn input(a: Args) -> Result<ExtractInput, String> {
    if let (Some(library_dir), Some(env)) = (a.library, a.env) {
        return library::acquired(&library_dir, &env, a.snapshot).map_err(|e| e.to_string());
    }
    let (Some(root), Some(label), Some(venv_root)) = (a.release_root, a.label, a.venv_root) else {
        return Err(
            "give --library and --env, or --release-root, --release-label and --venv-root"
                .to_owned(),
        );
    };
    Ok(ExtractInput {
        release: Release::from_tree(root, &label).map_err(|e| e.to_string())?,
        venv_root,
        site_packages: a.site_packages,
        python_version: a.python.unwrap_or((3, 14, 0)),
        python_platform: a.platform.unwrap_or_else(|| "linux".to_owned()),
        snapshot_id: a.snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
}

/// Warnings from Pyrefly, delta-kernel and DataFusion (its `log` records through the tracing-log
/// bridge) go to stderr; without a subscriber they were dropped (H1 O1). `LCTX_LOG` overrides the
/// level (`LCTX_LOG=debug`); it changes output only, never an identity.
fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LCTX_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

fn main() -> ExitCode {
    init_logging();
    let run = || -> Result<(), String> {
        let a = Args::parse();
        let out = a.out.clone();
        let input = input(a)?;
        let output = extract(&input).map_err(|e| e.to_string())?;
        write_ipc(&out, &output).map_err(|e| e.to_string())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lctx-extract: {e}");
            ExitCode::from(1)
        }
    }
}
