//! `lctx`: the one production path from a pinned Python library to a published snapshot (DESIGN
//! §4.0, ADR-0013). Every analyzed library is a committed uv project under `libraries/<name>/`.
//!
//! ```text
//! lctx library init <name> --requirement REQ [--python 3.14.7]   write, lock, acquire, propose release
//! lctx acquire <name> [--reinstall]                              uv sync --frozen into build/envs/<name>,
//!                                                                and the declared source tree at its
//!                                                                commit into build/sources/<name>
//! lctx compile <name> --store DIR                                acquire, Stage A, extract, derive,
//!                                                                validate, publish
//! lctx query --store DIR --snapshot HEX "SQL"                    read-only SQL over a published
//!                                                                snapshot (its tables by name)
//! ```
//! Common options: `--libraries DIR` (default `libraries`), `--envs DIR` (default `build/envs`),
//! `--sources DIR` (default `build/sources`).
//! Upgrading a library: edit its pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
//! then `lctx compile <name>`.

mod propose;

/// jemalloc, not glibc malloc (ADR-0016): glibc's per-thread arenas retained about half of a 6.1-7.3
/// GB pilot peak; jemalloc peaks at the working set (3.6 GB) and is no slower. Pyrefly's own CLI
/// uses it on the same platforms.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use cpg_extract::{extract, library};
use cpg_schema::id::Id;

/// The command line (H1 C4: clap derive; each command takes only its own options).
#[derive(Parser, Debug)]
#[command(
    name = "lctx",
    version,
    about = "Compile a pinned Python library into a published snapshot"
)]
struct Cli {
    /// Library definitions: `<DIR>/<name>/`.
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
    /// Acquire, then Stage A, extract, derive, validate and publish.
    Compile {
        name: String,
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// Reinstall every package while acquiring.
        #[arg(long)]
        reinstall: bool,
    },
    /// Read-only SQL over a published snapshot (its tables by name).
    Query {
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// The snapshot id: 32 hex digits.
        #[arg(long, value_parser = parse_id)]
        snapshot: Id,
        /// The tables' latest versions instead: an attempt validation rejected, for inspection only.
        #[arg(long)]
        unpublished: bool,
        sql: String,
    },
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

fn random_id() -> anyhow::Result<Id> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("randomness: {e}"))?;
    Ok(Id(bytes))
}

fn compile(
    library_dir: &Path,
    env_dir: &Path,
    sources: &Path,
    store: &Path,
    reinstall: bool,
) -> anyhow::Result<()> {
    let started = Instant::now();
    acquire(library_dir, env_dir, reinstall)?;
    let tree = fetch_source(library_dir, sources)?;
    let acquired = started.elapsed();
    let snapshot = random_id()?;
    let mut input = library::acquired(library_dir, env_dir, snapshot)?;
    if let Some((tree, source)) = &tree {
        input.corpus = Some(library::corpus(tree, source, &input)?);
    }
    let staged = started.elapsed() - acquired;
    // Printed first, so an attempt validation rejects can be inspected (`query --unpublished`).
    println!("attempt  {}", snapshot.hex());
    println!(
        "release {} ({} modules)",
        input.release.release_id.hex(),
        input.release.files.len()
    );
    if let Some(c) = &input.corpus {
        println!(
            "corpus  {} ({} documents, {} usage modules)",
            c.release.release_id.hex(),
            c.documents.len(),
            c.release.files.len()
        );
    }
    let mut output = extract(&input)?;
    let extracted = started.elapsed();
    let runtime = tokio::runtime::Runtime::new()?;
    let tables = std::mem::take(&mut output.tables);
    let published = runtime.block_on(cpg_core::attempt::compile_owned(store, snapshot, tables))?;
    println!("snapshot {} published", published.snapshot_id.hex());
    println!("content  {}", published.content_digest.hex());
    for (name, rows) in &published.rows {
        println!(
            "  {name:<20} {rows:>7} rows  v{}",
            published.versions[*name]
        );
    }
    println!("stages (wall time, peak RSS so far):");
    println!(
        "  {:<52} {:>7.2}s",
        "acquire (uv sync --frozen, source fetch)",
        acquired.as_secs_f64()
    );
    println!(
        "  {:<52} {:>7.2}s",
        "Stage A (verify RECORDs)",
        staged.as_secs_f64()
    );
    for stage in output.stages.iter().chain(&published.stages) {
        println!(
            "  {:<52} {:>7.2}s  {:>6} MiB",
            stage.name,
            stage.seconds,
            stage
                .peak_rss_bytes
                .map_or("?".to_owned(), |b| (b >> 20).to_string())
        );
    }
    println!(
        "extract {:.1}s, total {:.1}s",
        extracted.as_secs_f64(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Write `libraries/<name>/`, lock it, acquire it, and propose `[tool.lctx] release` as the
/// requested distribution plus every installed distribution sharing a source repository with it.
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

/// Read-only SQL over one published snapshot: every table registered under its own name, at its
/// recorded version, filtered to the snapshot (DESIGN §6.2). With `unpublished`, the tables' latest
/// versions instead: an attempt validation rejected, for inspection only.
fn query(store: &Path, id: Id, sql: &str, unpublished: bool) -> anyhow::Result<()> {
    let hex = id.hex();
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let ctx = if unpublished {
            let versions = cpg_core::snapshot::latest(store).await?;
            cpg_core::snapshot::session(store, id, &versions).await?
        } else {
            cpg_core::snapshot::published(store, id)
                .await?
                .with_context(|| format!("snapshot {hex} is not published in {}", store.display()))?
                .1
        };
        let text = cpg_core::sql::render(&ctx, sql).await?;
        println!("{text}");
        Ok(())
    })
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let libraries = absolute(&cli.libraries)?;
    let envs = absolute(&cli.envs)?;
    let sources = absolute(&cli.sources)?;
    match cli.command {
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
        Cmd::Query {
            store,
            snapshot,
            unpublished,
            sql,
        } => query(&absolute(&store)?, snapshot, &sql, unpublished),
        Cmd::Compile {
            name,
            store,
            reinstall,
        } => compile(
            &libraries.join(&name),
            &envs.join(&name),
            &sources.join(&name),
            &absolute(&store)?,
            reinstall,
        ),
    }
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
    match run() {
        Ok(()) => ExitCode::SUCCESS,
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
        let cli = parse(&["compile", "fastmcp", "--store", "s", "--reinstall"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::Compile {
                reinstall: true,
                ..
            }
        ));
        let cli = parse(&["acquire", "fastmcp", "--envs", "e"]).unwrap();
        assert!(matches!(
            cli.command,
            Cmd::Acquire {
                reinstall: false,
                ..
            }
        ));
        assert_eq!(cli.envs, std::path::PathBuf::from("e"));
        let cli = parse(&[
            "query",
            "--store",
            "s",
            "--snapshot",
            &"ab".repeat(16),
            "SELECT 1",
        ]);
        assert!(matches!(cli.unwrap().command, Cmd::Query { .. }));
        // An option of another command is refused, not ignored.
        assert!(parse(&["acquire", "fastmcp", "--store", "s"]).is_err());
        assert!(
            parse(&[
                "query",
                "--store",
                "s",
                "--snapshot",
                &"ab".repeat(16),
                "--requirement",
                "x",
                "q"
            ])
            .is_err()
        );
    }

    /// The hand parsers accepted a sign (`from_str_radix` reads `+f`) and panicked slicing
    /// non-ASCII input (H1 C4).
    #[test]
    fn a_snapshot_id_is_exactly_32_hex_digits() {
        for bad in [
            "+f".repeat(16),
            "é".repeat(16),
            "ab".repeat(15),
            "zz".repeat(16),
        ] {
            assert!(
                parse(&["query", "--store", "s", "--snapshot", &bad, "q"]).is_err(),
                "{bad}"
            );
        }
        assert!(parse(&["query", "--store", "s", "--snapshot", &"AB".repeat(16), "q"]).is_ok());
    }
}
