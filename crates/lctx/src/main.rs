//! `lctx`: pinned Python libraries and their compilation (DESIGN §4.0, ADR-0013). Every analyzed
//! library is a committed uv project under `libraries/<name>/`.
//!
//! The semantic-model cutover (plan §4.1.1) retired the Delta pipeline's commands. Until phase 2
//! lands `lctx compile --through facts`, `compile` exits 3 (unavailable) before doing any work.
//!
//! ```text
//! lctx library init <name> --requirement REQ [--python 3.14.7]   write, lock, acquire, propose release
//! lctx acquire <name> [--reinstall]                              uv sync --frozen into build/envs/<name>,
//!                                                                and the declared source tree at its
//!                                                                commit into build/sources/<name>
//! lctx model describe [--format text|json]                       the typed model, with no database
//! lctx store install|check|reset [--confirm DB]                  the generation store (service owner)
//! lctx generation list|show|select|clear-selection|retire|abort  generations (reader; owner changes)
//! lctx query --generation ID SQL                                 read-only SQL over one leased generation
//! lctx runs list|show|mark-interrupted                           operational compile-attempt history
//! lctx flow FILE                                                 one file's flow facts (oracle input)
//! lctx compile <name> …                                          unavailable until cutover phase 2
//! ```
//! Common options: `--database FILE` (the protected `postgres.json`; its siblings select the roles),
//! `--libraries DIR` (default `libraries`), `--envs DIR` (default `build/envs`), `--sources DIR`
//! (default `build/sources`). Exit status: 0 ok, 1 error, 2 refused, 3 unavailable.

mod database;
mod generation;
mod model;
mod propose;
mod query;
mod runs;
mod store;

/// jemalloc, not glibc malloc (ADR-0016): glibc's per-thread arenas retained about half of a
/// 6,100-7,300 MiB pilot peak; under jemalloc the peak stays flat at extraction's working set
/// (about 3,640 MiB) and the compile is no slower. Pyrefly's own CLI uses it on the same platforms.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use cpg_extract::library;
use cpg_schema::id::Id;
use lctx_workspace_hack as _; // Contributes Cargo features, not callable APIs (ADR-0079).

/// The command line (H1 C4: clap derive; each command takes only its own options).
#[derive(Parser, Debug)]
#[command(name = "lctx", version, about = "Pinned Python libraries and their compilation")]
struct Cli {
    /// Protected PostgreSQL config (`postgres.json`; its siblings select the roles); otherwise
    /// LCTX_DATABASE_CONFIG or ~/.config/library-context/postgres.json.
    #[arg(long, global = true)]
    database: Option<PathBuf>,
    /// Library projects: `<DIR>/<name>/`.
    #[arg(long, global = true, default_value = "libraries")]
    libraries: PathBuf,
    /// Acquired environments: `<DIR>/<name>/`.
    #[arg(long, global = true, default_value = "build/envs")]
    envs: PathBuf,
    /// Fetched source trees: `<DIR>/<name>/<commit>/`.
    #[arg(long, global = true, default_value = "build/sources")]
    sources: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// The typed model this binary lowers.
    Model {
        #[command(subcommand)]
        command: ModelCommand,
    },
    /// The generation store, as the service owner.
    Store {
        #[command(subcommand)]
        command: store::StoreCommand,
    },
    /// Generations: listed and shown by the reader; selected, retired and aborted by the owner.
    Generation {
        #[command(subcommand)]
        command: generation::GenerationCommand,
    },
    /// Read-only SQL over one leased generation.
    Query {
        #[arg(long, value_parser = generation::parse_generation)]
        generation: cpg_core::postgres::generations::GenerationId,
        sql: String,
    },
    /// Operational compile-attempt history.
    Runs {
        #[command(subcommand)]
        command: runs::Runs,
    },
    /// Library definitions.
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
    /// `uv sync --frozen` into the environment, and fetch the declared source tree.
    Acquire {
        name: String,
        /// Reinstall every package (`uv sync --reinstall`).
        #[arg(long)]
        reinstall: bool,
    },
    /// The acquired environment's deployment identity, as JSON (scripts/deployment_check.py).
    DeploymentIdentity { name: String },
    /// Unavailable until cutover phase 2 (`--through facts`); exits 3 before doing any work.
    Compile {
        name: String,
        /// Accepted and ignored so that no argument reaches a retired pipeline.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        rest: Vec<String>,
    },
    /// Inspect one generated Python file's flow facts as JSON (runtime oracle input).
    Flow {
        file: PathBuf,
        /// Analyzed Python interpreter version, MAJOR.MINOR.MICRO.
        #[arg(long, default_value = "3.14.7")]
        python: String,
        #[arg(long, default_value = "linux")]
        platform: String,
        /// Optional resolved name-load spans, for the runtime differential oracle.
        #[arg(long)]
        runtime_bindings: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum ModelCommand {
    Describe {
        #[arg(long, value_enum, default_value = "text")]
        format: model::Format,
    },
}

/// A command that exists but whose capability is suspended by the cutover: exit status 3.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Unavailable(&'static str);

/// A request the store or the read contract refuses by its rules: exit status 2.
#[derive(Debug, thiserror::Error)]
#[error("refused: {0}")]
pub struct Refused(pub String);

/// Whether an error is a refusal: the store, a lease or the read contract declined the request.
fn refused(error: &anyhow::Error) -> bool {
    use cpg_core::generation_read::ReadError;
    use cpg_core::postgres::generations::Error as Store;
    let store = |e: &Store| matches!(e, Store::State | Store::Absent | Store::NotInstalled | Store::Busy | Store::Contract | Store::Frontier(_)
        | Store::Confirmation | Store::Orphaned);
    error.chain().any(|cause| cause.is::<Refused>() || cause.downcast_ref::<Store>().is_some_and(store)
        || cause.downcast_ref::<ReadError>().is_some_and(|e| match e { ReadError::Frontier(_) => true, ReadError::Store(inner) => store(inner), _ => false }))
}

fn parse_id(s: &str) -> Result<Id, String> {
    Id::from_hex(s).ok_or_else(|| format!("{s:?} is not 32 hex digits"))
}

fn absolute(p: &Path) -> anyhow::Result<PathBuf> {
    std::path::absolute(p).with_context(|| p.display().to_string())
}

/// Run uv for one library, with the environment pinned to `env_dir` and every other `UV_*`
/// setting and `VIRTUAL_ENV` removed, so nothing ambient steers resolution or installation.
fn uv(args: &[&str], env_dir: &Path) -> anyhow::Result<()> {
    let mut command = Command::new("uv");
    command.args(args);
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy().into_owned();
        if key.starts_with("UV_") || key == "VIRTUAL_ENV" {
            command.env_remove(key);
        }
    }
    command.env("UV_PROJECT_ENVIRONMENT", env_dir);
    let status = command.status().context("blocked: `uv` could not run")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("uv {} failed ({status})", args.join(" ")))
    }
}

/// `uv sync --frozen` into the library's environment, ignoring user and system uv configuration
/// (`--no-config`), on the interpreter `.python-version` pins, and copying files rather than
/// hard-linking them from the uv cache, which other environments share (ADR-0013 review F3).
/// `reinstall` rebuilds every package: the remedy when Stage A finds a changed file.
fn acquire(library_dir: &Path, env_dir: &Path, reinstall: bool) -> anyhow::Result<()> {
    if !library_dir.join("uv.lock").exists() {
        return Err(anyhow::anyhow!(
            "{} has no uv.lock; run `lctx library init` or `uv lock --project {}`",
            library_dir.display(),
            library_dir.display()
        ));
    }
    let python = fs_err::read_to_string(library_dir.join(".python-version"))
        .with_context(|| format!("{}/.python-version", library_dir.display()))?;
    let project = library_dir.to_string_lossy();
    let mut args = vec![
        "sync",
        "--project",
        &project,
        "--frozen",
        "--no-install-project",
        "--no-config",
        "--python",
        python.trim(),
        "--link-mode",
        "copy",
    ];
    if reinstall {
        args.push("--reinstall");
    }
    uv(&args, env_dir)
}

/// Run git hermetically: every `GIT_*` variable removed, no system or global configuration, no
/// prompts, so nothing ambient steers what is fetched (C5, like `uv`).
fn git(args: &[&str], dir: &Path) -> anyhow::Result<String> {
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy().into_owned();
        if key.starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    // Nor attributes: an ambient `eol` rule would rewrite the checked-out bytes (C5 review F4).
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = command.output().context("blocked: `git` could not run")?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(anyhow::anyhow!(
            "git {} failed ({}): {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The library's declared source tree at its pinned commit, fetched once into
/// `<sources>/<commit>` (a shallow fetch of that one commit) and checked by `rev-parse` every
/// time; `None` when no source is declared.
fn fetch_source(
    library_dir: &Path,
    sources: &Path,
) -> anyhow::Result<Option<(PathBuf, library::Source)>> {
    let Some(source) = library::source(library_dir)? else {
        return Ok(None);
    };
    let tree = sources.join(&source.commit);
    if !tree.join(".git").is_dir() {
        let partial = sources.join(format!("{}.partial", source.commit));
        if partial.exists() {
            fs_err::remove_dir_all(&partial)?;
        }
        fs_err::create_dir_all(&partial)?;
        // No template directory: the system one is the last ambient git input (a hook there would
        // run on checkout; H1 C6).
        git(&["init", "-q", "--template="], &partial)?;
        git(
            &[
                "fetch",
                "-q",
                "--depth",
                "1",
                &source.repository,
                &source.commit,
            ],
            &partial,
        )?;
        git(
            &[
                "-c",
                "advice.detachedHead=false",
                "-c",
                "core.attributesFile=/dev/null",
                "-c",
                "core.autocrlf=false",
                "checkout",
                "-q",
                "FETCH_HEAD",
            ],
            &partial,
        )?;
        fs_err::rename(&partial, &tree)?;
    }
    let head = git(&["rev-parse", "HEAD"], &tree)?;
    if head != source.commit {
        return Err(anyhow::anyhow!(
            "{} is at {head}, not the pinned {}; delete it to refetch",
            tree.display(),
            source.commit
        ));
    }
    Ok(Some((tree, source)))
}

fn init(
    name: &str,
    requirement: &str,
    python: &str,
    libraries: &Path,
    envs: &Path,
) -> anyhow::Result<()> {
    let dist = library::requirement_name(requirement);
    let (major, minor) = {
        let mut parts = python.split('.');
        (parts.next().unwrap_or("3"), parts.next().unwrap_or("14"))
    };
    let library_dir = libraries.join(name);
    if library_dir.exists() {
        return Err(anyhow::anyhow!("{} already exists", library_dir.display()));
    }
    fs_err::create_dir_all(&library_dir)?;
    let write = |release: &[String]| {
        let release = release
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs_err::write(
            library_dir.join("pyproject.toml"),
            format!(
                "# {name} as an analyzed library (DESIGN §4.0, ADR-0013), written by `lctx library \
                 init`.\n[project]\nname = \"lctx-library-{name}\"\nversion = \"0\"\n\
                 requires-python = \"=={major}.{minor}.*\"\ndependencies = [\"{requirement}\"]\n\n\
                 [tool.uv]\npackage = false\n\n[tool.lctx]\n# The first-party distributions whose \
                 code is compiled. Proposed from shared source\n# repositories: review before \
                 compiling.\nrelease = [{release}]\n"
            ),
        )
    };
    write(std::slice::from_ref(&dist))?;
    fs_err::write(library_dir.join(".python-version"), format!("{python}\n"))?;
    let env_dir = envs.join(name);
    uv(
        &["lock", "--project", &library_dir.to_string_lossy()],
        &env_dir,
    )?;
    acquire(&library_dir, &env_dir, false)?;
    let site = fs_err::read_dir(env_dir.join("lib"))?
        .filter_map(|e| e.ok().map(|e| e.path().join("site-packages")))
        .find(|p| p.is_dir())
        .context("the acquired environment has no site-packages")?;
    let installed: Vec<(String, String)> = fs_err::read_dir(&site)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|p| {
            let stem = p
                .file_name()?
                .to_str()?
                .strip_suffix(".dist-info")?
                .to_owned();
            let metadata = fs_err::read_to_string(p.join("METADATA")).unwrap_or_default();
            Some((library::normalize(stem.rsplit_once('-')?.0), metadata))
        })
        .collect();
    let (release, found) = propose::propose(&dist, &installed);
    write(&release)?;
    println!(
        "{} written and locked; proposed release = {release:?}{}. Review it, then commit the \
         directory (pyproject.toml, .python-version, uv.lock).",
        library_dir.display(),
        if found {
            ""
        } else {
            " (the distribution names no source repository, so it is proposed alone)"
        }
    );
    Ok(())
}

fn flow_file(
    file: &Path,
    python: &str,
    platform: &str,
    runtime_bindings: Option<&Path>,
) -> anyhow::Result<()> {
    let parts: Vec<u32> = python
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .with_context(|| format!("invalid Python version {python:?}"))?;
    let [major, minor, micro] = parts.as_slice() else {
        return Err(anyhow::anyhow!("Python version must be MAJOR.MINOR.MICRO"));
    };
    let text = fs_err::read_to_string(file)?;
    let path = file
        .file_name()
        .context("flow file has no name")?
        .to_string_lossy()
        .into_owned();
    let runtime = if let Some(path) = runtime_bindings {
        serde_json::from_slice::<cpg_flow::RuntimeBindings>(&fs_err::read(path)?)?
    } else {
        Default::default()
    };
    for span in runtime
        .checking_names
        .iter()
        .chain(&runtime.typing_modules)
        .chain(&runtime.sys_modules)
        .chain(&runtime.os_modules)
    {
        let name = text
            .get(span.start as usize..span.end as usize)
            .filter(|name| !name.is_empty())
            .context("runtime binding span lies outside the source or crosses a codepoint")?;
        if !name.chars().all(|c| c == '_' || c.is_alphanumeric()) {
            return Err(anyhow::anyhow!(
                "runtime binding span does not cover a name: {span:?}"
            ));
        }
    }
    let input = cpg_flow::Input {
        path,
        text,
        runtime,
    };
    let flow = cpg_flow::index(
        &[input],
        &cpg_flow::RuntimeContext {
            python_version: (*major, *minor, *micro),
            platform: platform.to_owned(),
        },
    )
    .pop()
    .context("flow provider returned no module")?;
    if let Some(error) = flow.error {
        return Err(anyhow::anyhow!(error));
    }
    let span = |s: cpg_flow::Span| serde_json::json!([s.start, s.end]);
    let result = serde_json::json!({
        "uses": flow.uses.iter().map(|u| serde_json::json!({"place": u.place, "span": span(u.span)})).collect::<Vec<_>>(),
        "definitions": flow.defs.iter().map(|d| serde_json::json!({"place": d.place, "kind": format!("{:?}", d.kind), "target": span(d.target), "value": d.value.map(&span)})).collect::<Vec<_>>(),
        "reaching": flow.reaching.iter().map(|r| serde_json::json!({"use_ix": r.use_ix, "def_ix": r.def_ix, "condition": r.condition.encode(), "loop_carried": r.loop_carried})).collect::<Vec<_>>(),
        "values": flow.values.iter().map(|v| serde_json::json!({"sink": format!("{:?}", v.sink), "span": span(v.span), "use_ix": v.use_ix, "identity": v.identity, "through_call": v.through_call, "condition": v.condition.encode()})).collect::<Vec<_>>(),
        "regions": flow.regions.iter().map(|r| serde_json::json!({"span": span(r.span), "condition": r.condition.encode()})).collect::<Vec<_>>(),
        "skips": {"reaching_ty_false": flow.skips.reaching_ty_false, "reaching_runtime_view": flow.skips.reaching_runtime_view, "reaching_stable_contradiction": flow.skips.reaching_stable_contradiction,
            "values_runtime_view": flow.skips.values_runtime_view, "values_stable_contradiction": flow.skips.values_stable_contradiction},
    });
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

#[derive(Subcommand, Debug)]
enum LibraryCommand {
    /// Write, lock and acquire `libraries/<name>/`, and propose `[tool.lctx] release`.
    Init {
        name: String,
        /// The one pinned requirement, e.g. `fastmcp[tasks]==4.0.5`.
        #[arg(long)]
        requirement: String,
        /// The interpreter, MAJOR.MINOR.MICRO.
        #[arg(long, default_value = "3.14.7")]
        python: String,
    },
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let libraries = absolute(&cli.libraries)?;
    let envs = absolute(&cli.envs)?;
    let sources = absolute(&cli.sources)?;
    let runtime = || tokio::runtime::Runtime::new();
    match cli.command {
        Cmd::Model { command: ModelCommand::Describe { format } } => {
            let described = model::describe(&*database::model()?);
            match format {
                model::Format::Json => println!("{}", serde_json::to_string_pretty(&described)?),
                model::Format::Text => print!("{}", model::text(&described)),
            }
            Ok(())
        }
        Cmd::Store { command } => {
            let database = database::Database::discover(cli.database.as_deref())?;
            runtime()?.block_on(store::store(command, &database))
        }
        Cmd::Generation { command } => {
            let database = database::Database::discover(cli.database.as_deref())?;
            runtime()?.block_on(generation::generation(command, &database))
        }
        Cmd::Query { generation, sql } => {
            let database = database::Database::discover(cli.database.as_deref())?;
            runtime()?.block_on(query::query(&database, generation, &sql))
        }
        Cmd::Runs { command } => {
            let database = database::Database::discover(cli.database.as_deref())?;
            runtime()?.block_on(runs::runs(command, &database))
        }
        Cmd::Library {
            command:
                LibraryCommand::Init {
                    name,
                    requirement,
                    python,
                },
        } => init(&name, &requirement, &python, &libraries, &envs),
        Cmd::Acquire { name, reinstall } => {
            let library_dir = libraries.join(&name);
            acquire(&library_dir, &envs.join(&name), reinstall)?;
            fetch_source(&library_dir, &sources.join(&name)).map(|_| ())
        }
        Cmd::DeploymentIdentity { name } => {
            let input = library::acquired(&libraries.join(&name), &envs.join(&name), Id::ZERO)?;
            println!(
                "{}",
                serde_json::to_string(&cpg_extract::observations::identity(&input)?)?
            );
            Ok(())
        }
        Cmd::Compile { .. } => Err(Unavailable(
            "compile is unavailable until cutover phase 2 lands `lctx compile --through facts`",
        )
        .into()),
        Cmd::Flow {
            file,
            python,
            platform,
            runtime_bindings,
        } => flow_file(&file, &python, &platform, runtime_bindings.as_deref()),
    }
}

fn main() -> ExitCode {
    cpg_extract::logging::init_logging();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is::<Unavailable>() => {
            eprintln!("lctx: {e}");
            ExitCode::from(3)
        }
        Err(e) if refused(&e) => {
            eprintln!("lctx: {e:#}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("lctx: {e:#}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Cmd};

    fn parse(args: &[&str]) -> Result<Cli, String> {
        Cli::try_parse_from(std::iter::once("lctx").chain(args.iter().copied()))
            .map_err(|e| e.to_string())
    }

    #[test]
    fn each_command_takes_its_own_options() {
        let cli = parse(&["acquire", "fastmcp", "--envs", "e"]).unwrap();
        assert!(matches!(cli.command, Cmd::Acquire { reinstall: false, .. }));
        assert_eq!(cli.envs, std::path::PathBuf::from("e"));
        // An option of another command is refused, not ignored.
        assert!(parse(&["acquire", "fastmcp", "--store", "s"]).is_err());
        // Retired pipeline commands no longer parse.
        for retired in ["bundle", "diff", "serving", "rebuild", "parity", "db", "snapshots", "generations", "compile-fixture"] {
            assert!(parse(&[retired]).is_err(), "{retired}");
        }
        // Compile parses any arguments so that it can refuse them all.
        assert!(matches!(parse(&["compile", "fastmcp", "--store", "s", "--reinstall"]).unwrap().command, Cmd::Compile { .. }));
    }

    /// The hand parsers accepted a sign (`from_str_radix` reads `+f`) and panicked slicing
    /// non-ASCII input (H1 C4).
    #[test]
    fn an_attempt_id_is_exactly_32_hex_digits() {
        for bad in ["+f".repeat(16), "é".repeat(16), "ab".repeat(15), "zz".repeat(16)] {
            assert!(parse(&["runs", "show", &bad]).is_err(), "{bad}");
        }
        assert!(parse(&["runs", "show", &"AB".repeat(16)]).is_ok());
    }
}
