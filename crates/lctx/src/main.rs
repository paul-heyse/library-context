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
//! lctx bundle --store DIR --snapshot HEX [--out DIR]             the snapshot's serving generation
//!                                                                (DESIGN §6.4); compile builds it
//!                                                                after publishing
//! lctx compile-fixture DIR --package P --seed OP --store DIR      a dependency-free generated tree,
//!                                                                for runtime challenges (never a
//!                                                                library or analyzed release)
//! ```
//! Common options: `--libraries DIR` (default `libraries`), `--envs DIR` (default `build/envs`),
//! `--sources DIR` (default `build/sources`).
//! Upgrading a library: edit its pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
//! then `lctx compile <name>`.

mod db;
mod propose;
mod rebuild;
mod serving;

/// jemalloc, not glibc malloc (ADR-0016): glibc's per-thread arenas retained about half of a
/// 6,100-7,300 MiB pilot peak; under jemalloc the peak stays flat at extraction's working set
/// (about 3,640 MiB) and the compile is no slower. Pyrefly's own CLI uses it on the same platforms.
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
use lctx_workspace_hack as _; // Contributes Cargo features, not callable APIs (ADR-0079).

/// The command line (H1 C4: clap derive; each command takes only its own options).
#[derive(Parser, Debug)]
#[command(
    name = "lctx",
    version,
    about = "Compile a pinned Python library into a published snapshot"
)]
struct Cli {
    /// Protected PostgreSQL config; otherwise LCTX_DATABASE_CONFIG or ~/.config/library-context/postgres.json.
    #[arg(long, global = true)]
    database_config: Option<PathBuf>,
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
    /// Rebuild from a validated published snapshot without reacquisition or extraction.
    Rebuild {
        #[command(subcommand)]
        command: rebuild::Command,
    },

    /// Immutable PostgreSQL serving projections and explicit operator selection.
    Serving {
        #[arg(long)]
        importer_config: Option<PathBuf>,
        #[command(subcommand)]
        command: serving::Command,
    },
    /// PostgreSQL migrations, diagnostics and explicit reconciliation/import.
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
    /// Operational compile-attempt history.
    Runs {
        #[command(subcommand)]
        command: db::Runs,
    },
    /// Reconciled publication discovery, checked against Delta when read.
    Snapshots {
        #[command(subcommand)]
        command: db::Snapshots,
    },
    /// Reconciled generation discovery, checked against the immutable files when read.
    Generations {
        #[command(subcommand)]
        command: db::Generations,
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
    /// Acquire, then Stage A, extract, derive, validate and publish.
    DeploymentIdentity { name: String },
    Compile {
        name: String,
        /// Explicit receipts produced by scripts/deployment_check.py; never executes their source.
        #[arg(long)]
        evidence_observations: Vec<PathBuf>,
        /// Catalog contracts by default; behavioral adds the retained analysis and brief pipeline.
        #[arg(long, value_enum, default_value = "catalog")]
        profile: Profile,
        /// Explicit roots when the library has no analytics.toml; repeat for multiple roots.
        #[arg(long)]
        public_root: Vec<String>,
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// Reinstall every package while acquiring.
        #[arg(long)]
        reinstall: bool,
        /// How brief documents are embedded: `vllm` (the live service; `blocked` without it),
        /// `fake` (the deterministic test embedder) or `none`.
        #[arg(long, value_enum, default_value = "fake")]
        embedder: EmbedderChoice,
        /// The vLLM service `--embedder vllm` uses.
        #[arg(long, default_value = "http://127.0.0.1:8000")]
        embed_url: String,
        /// Where the published snapshot's serving generation is built (DESIGN §6.4).
        #[arg(long, default_value = "build/generations")]
        generations: PathBuf,
        /// The analytics variant (DESIGN §9.8's ablation): `default`, or changes to it such as
        /// `-knn` or `+rca,+type-layer`.
        #[arg(long, default_value = "default", allow_hyphen_values = true)]
        analytics: String,
    },
    /// Build a published snapshot's serving generation (DESIGN §6.4): `<out>/<key>/`.
    Bundle {
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// The snapshot id: 32 hex digits.
        #[arg(long, value_parser = parse_id)]
        snapshot: Id,
        /// The generations directory.
        #[arg(long, default_value = "build/generations")]
        out: PathBuf,
    },
    /// Compare two published snapshots by content id (DESIGN §9.8's ablation diff).
    Diff {
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// The snapshot compared from: 32 hex digits.
        #[arg(long, value_parser = parse_id)]
        from: Id,
        /// The snapshot compared to.
        #[arg(long, value_parser = parse_id)]
        to: Id,
        /// Write the comparison as JSON here too.
        #[arg(long)]
        json: Option<PathBuf>,
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
    /// Compile a small dependency-free release tree (a generated program for a runtime
    /// challenge) into a store and build its generation. Development only: no uv project,
    /// Stage A or corpus; a minimal analytics config seeds `--seed`.
    CompileFixture {
        /// The directory holding the top-level package.
        dir: PathBuf,
        /// The top-level package (the analytics subsystem and public root).
        #[arg(long)]
        package: String,
        /// Catalog by default; runtime semantic challenges explicitly select behavioral.
        #[arg(long, value_enum, default_value = "catalog")]
        profile: Profile,
        /// A public operation seeding optional behavioral analysis.
        #[arg(long)]
        seed: Option<String>,
        /// The Delta store.
        #[arg(long)]
        store: PathBuf,
        /// Where the serving generation is built.
        #[arg(long, default_value = "build/generations")]
        generations: PathBuf,
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

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Catalog,
    Behavioral,
}
impl Profile {
    fn schema(self) -> cpg_schema::catalog::CompileProfile {
        match self {
            Self::Catalog => cpg_schema::catalog::CompileProfile::Catalog,
            Self::Behavioral => cpg_schema::catalog::CompileProfile::Behavioral,
        }
    }
}

/// The embedder a compile uses (DESIGN §11.1).
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum EmbedderChoice {
    Vllm,
    Fake,
    None,
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

fn embedder_of(
    choice: EmbedderChoice,
    url: &str,
) -> Option<std::sync::Arc<dyn cpg_core::embed::Embedder>> {
    match choice {
        EmbedderChoice::Vllm => Some(std::sync::Arc::new(lctx_embed::VllmEmbedder::new(
            url,
            lctx_embed::qwen_spec(),
        ))),
        EmbedderChoice::Fake => Some(std::sync::Arc::new(cpg_core::embed::FakeEmbedder::new())),
        EmbedderChoice::None => None,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the compile command's flags, one each"
)]
fn compile(
    database_config: Option<&Path>,
    library_dir: &Path,
    env_dir: &Path,
    sources: &Path,
    store: &Path,
    reinstall: bool,
    embedder: Option<std::sync::Arc<dyn cpg_core::embed::Embedder>>,
    generations: &Path,
    techniques: cpg_core::analyze::Techniques,
    profile: Profile,
    public_roots: Vec<String>,
    evidence_observations: Vec<PathBuf>,
) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let snapshot = random_id()?;
    let config_path = cpg_core::postgres::Config::path(database_config);
    let pg = if embedder.is_some() {
        let db = runtime.block_on(db::connect(Some(&config_path?)))?;
        runtime.block_on(db.check())?;
        Some(db)
    } else if config_path.as_ref().is_ok_and(|p| p.exists())
        || database_config.is_some()
        || std::env::var_os("LCTX_DATABASE_CONFIG").is_some()
    {
        match runtime.block_on(async {
            let db = db::connect(Some(&config_path?)).await?;
            db.check().await?;
            Ok::<_, anyhow::Error>(db)
        }) {
            Ok(db) => Some(db),
            Err(error) => {
                eprintln!("journal unavailable: {error}");
                None
            }
        }
    } else {
        None
    };
    if let Some(db) = &pg
        && let Err(error) = runtime.block_on(db.start_attempt(
            snapshot,
            cpg_core::attempt::compiler_digest(),
            &library_dir.to_string_lossy(),
            &store.to_string_lossy(),
        ))
    {
        eprintln!("journal start unavailable: {error}");
    }
    let result = (|| -> anyhow::Result<()> {
        let started = Instant::now();
        acquire(library_dir, env_dir, reinstall)?;
        let tree = fetch_source(library_dir, sources)?;
        let acquired = started.elapsed();
        let config_path = library_dir.join("analytics.toml");
        let public_roots = if config_path.exists() {
            anyhow::ensure!(
                public_roots.is_empty(),
                "--public-root conflicts with analytics.toml roots"
            );
            lctx_analytics::config::AnalyticsConfig::public_roots(&config_path)?
        } else {
            public_roots
        };
        cpg_schema::catalog::validate_roots(&public_roots).map_err(anyhow::Error::msg)?;
        let mut input = library::acquired(library_dir, env_dir, snapshot)?;
        input.profile = profile.schema();
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
        cpg_extract::observations::attach(&input, &mut output, &evidence_observations)?;
        let extracted = started.elapsed();
        if let Some(db) = &pg {
            for (index, stage) in output.stages.iter().enumerate() {
                if let Err(error) = runtime.block_on(db.event(
                    snapshot,
                    &format!("extract/{index}"),
                    "stage",
                    &format!("{} {:.6}s", stage.name, stage.seconds),
                )) {
                    eprintln!("extraction journal unavailable: {error}");
                    break;
                }
            }
        }
        let tables = std::mem::take(&mut output.tables);
        // The pre-registered analytics config (DESIGN §1.4, §9): without one, no analysis runs.
        let config_path = library_dir.join("analytics.toml");
        let analysis = if profile == Profile::Behavioral {
            if let Some(e) = &embedder {
                let spec = e.spec();
                println!("embedder {} (spec {})", spec.model, spec.hash().hex());
            }
            println!("analytics techniques {}", techniques.label());
            Some(cpg_core::analyze::Analysis {
                embedding_cache: pg.clone(),
                config: lctx_analytics::config::AnalyticsConfig::load(&config_path)?,
                embedder: embedder.clone(),
                techniques,
            })
        } else {
            None
        };
        let inputs = cpg_core::catalog::CompileInputs {
            public_roots,
            profile: profile.schema(),
            embedding_cache: pg.clone(),
            embedder,
        };
        let published = runtime.block_on(cpg_core::attempt::compile_catalog(
            store,
            snapshot,
            tables,
            &inputs,
            analysis.as_ref(),
        ))?;
        if let Some(db) = &pg {
            if let Err(error) = runtime.block_on(async {
                db.event(
                    snapshot,
                    "published",
                    "published",
                    &published.content_digest.hex(),
                )
                .await?;
                db.record_snapshot(
                    &store.to_string_lossy(),
                    snapshot,
                    published.content_digest,
                    cpg_core::attempt::compiler_digest(),
                )
                .await
            }) {
                eprintln!("publication succeeded; journal/discovery needs reconciliation: {error}");
            }
            for (index, stage) in published.stages.iter().enumerate() {
                if let Err(error) = runtime.block_on(db.event(
                    snapshot,
                    &format!("stage/{index}"),
                    "stage",
                    &format!("{} {:.6}s", stage.name, stage.seconds),
                )) {
                    eprintln!("stage journal unavailable: {error}");
                    break;
                }
            }
        }
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
        // Stage G (§6.4): the generation, from the published snapshot alone.
        let bundling = Instant::now();
        let generation = runtime.block_on(cpg_core::bundle::bundle_with_embedding(
            store,
            published.snapshot_id,
            generations,
            inputs.embedder.as_deref(),
            pg.clone(),
        ))?;
        if let Some(db) = &pg {
            if let Err(error) = runtime.block_on(db::record_generation(db, store, &generation.dir))
            {
                eprintln!("generation succeeded; discovery needs reconciliation: {error}");
            }
            if let Err(error) =
                runtime.block_on(db.event(snapshot, "generated", "generated", &generation.key))
            {
                eprintln!("generation journal unavailable: {error}");
            }
        }
        println!(
            "generation {} ({:.2}s)",
            generation.dir.display(),
            bundling.elapsed().as_secs_f64()
        );
        println!(
            "extract {:.1}s, total {:.1}s",
            extracted.as_secs_f64(),
            started.elapsed().as_secs_f64()
        );
        Ok(())
    })();
    if let Some(db) = &pg {
        if result.is_err() {
            // Preserve the original error; diagnostics may contain source text and are not
            // persisted into the operational database.
            if let Err(error) = runtime.block_on(db.event(
                snapshot,
                "failed",
                "failed",
                "compile or generation failed; see operator stderr",
            )) {
                eprintln!("failure journal unavailable: {error}");
            }
        }
        runtime.block_on(db.close());
    }
    result
}

/// Extract, compile and bundle a dependency-free generated tree (the runtime challenge's input),
/// printing the snapshot and generation. The empty environment sits beside the store.
fn compile_fixture(
    dir: &Path,
    package: &str,
    seed: Option<&str>,
    profile: Profile,
    store: &Path,
    generations: &Path,
) -> anyhow::Result<()> {
    if !package
        .chars()
        .all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        return Err(anyhow::anyhow!(
            "package must be a Python identifier: {package:?}"
        ));
    }
    let venv = store.with_extension("venv");
    let site = venv.join("site-packages");
    fs_err::create_dir_all(&site)?;
    let snapshot = random_id()?;
    let input = cpg_extract::ExtractInput {
        profile: profile.schema(),
        release: cpg_extract::Release::from_tree(fs_err::canonicalize(dir)?, package)?,
        venv_root: fs_err::canonicalize(&venv)?,
        site_packages: vec![fs_err::canonicalize(&site)?],
        python_version: (3, 14, 7),
        python_platform: "linux".to_owned(),
        snapshot_id: snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: cpg_extract::TestHooks::default(),
    };
    let mut output = extract(&input)?;
    let analysis = if profile == Profile::Behavioral {
        let seed = seed.ok_or_else(|| anyhow::anyhow!("--profile behavioral requires --seed"))?;
        let config = lctx_analytics::config::AnalyticsConfig::parse(&format!(
            "version = 1\n[subsystem]\nmodule_prefixes = [\"{package}\"]\n\
         public_roots = [\"{package}\"]\n[seeds]\nprimary = [{seed:?}]\ndistractors = []\n\
         [pass_a]\nmax_depth = 2\nmax_vertices = 128\nmax_edges = 512\nmax_witnesses = 3\n\
         [briefs]\nbudget = 1\n"
        ))
        .map_err(|e| anyhow::anyhow!("analytics config: {e}"))?;
        let analysis = cpg_core::analyze::Analysis {
            embedding_cache: None,
            config,
            embedder: None,
            techniques: cpg_core::analyze::Techniques::default(),
        };
        Some(analysis)
    } else {
        anyhow::ensure!(seed.is_none(), "--seed requires --profile behavioral");
        None
    };
    let runtime = tokio::runtime::Runtime::new()?;
    let tables = std::mem::take(&mut output.tables);
    let published = runtime.block_on(cpg_core::attempt::compile_catalog(
        store,
        snapshot,
        tables,
        &cpg_core::catalog::CompileInputs {
            public_roots: vec![package.into()],
            profile: profile.schema(),
            embedder: None,
            embedding_cache: None,
        },
        analysis.as_ref(),
    ))?;
    let generation = runtime.block_on(cpg_core::bundle::bundle(
        store,
        published.snapshot_id,
        generations,
    ))?;
    println!("snapshot {}", published.snapshot_id.hex());
    println!("generation {}", generation.dir.display());
    Ok(())
}

/// Build (or confirm) a published snapshot's serving generation and print its directory.
fn bundle(store: &Path, id: Id, out: &Path) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let generation = runtime.block_on(cpg_core::bundle::bundle(store, id, out))?;
    cpg_core::bundle::verify(&generation.dir)?;
    println!("generation {}", generation.dir.display());
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
            let versions = cpg_core::snapshot::attempt_versions(store, id).await?;
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
        Cmd::Rebuild { command } => command.run(cli.database_config.as_deref()),
        Cmd::Serving {
            command,
            importer_config,
        } => tokio::runtime::Runtime::new()?.block_on(serving::command(
            command,
            cli.database_config.as_deref(),
            importer_config.as_deref(),
        )),
        Cmd::Db { command } => tokio::runtime::Runtime::new()?
            .block_on(db::command(command, cli.database_config.as_deref())),
        Cmd::Runs { command } => tokio::runtime::Runtime::new()?
            .block_on(db::runs(command, cli.database_config.as_deref())),
        Cmd::Snapshots { command } => tokio::runtime::Runtime::new()?
            .block_on(db::snapshots(command, cli.database_config.as_deref())),
        Cmd::Generations { command } => tokio::runtime::Runtime::new()?
            .block_on(db::generations(command, cli.database_config.as_deref())),
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
        Cmd::Flow {
            file,
            python,
            platform,
            runtime_bindings,
        } => flow_file(&file, &python, &platform, runtime_bindings.as_deref()),
        Cmd::CompileFixture {
            dir,
            package,
            seed,
            profile,
            store,
            generations,
        } => compile_fixture(
            &absolute(&dir)?,
            &package,
            seed.as_deref(),
            profile,
            &absolute(&store)?,
            &absolute(&generations)?,
        ),
        Cmd::Diff {
            store,
            from,
            to,
            json,
        } => {
            let store = absolute(&store)?;
            let diff =
                tokio::runtime::Runtime::new()?.block_on(cpg_core::diff::diff(&store, from, to))?;
            print!("{}", diff.render());
            if let Some(path) = json {
                std::fs::write(path, diff.to_json())?;
            }
            Ok(())
        }
        Cmd::DeploymentIdentity { name } => {
            let input = library::acquired(&libraries.join(&name), &envs.join(&name), Id::ZERO)?;
            println!(
                "{}",
                serde_json::to_string(&cpg_extract::observations::identity(&input)?)?
            );
            Ok(())
        }
        Cmd::Compile {
            name,
            evidence_observations,
            profile,
            public_root,
            store,
            reinstall,
            embedder,
            embed_url,
            generations,
            analytics,
        } => {
            anyhow::ensure!(
                profile == Profile::Behavioral || analytics == "default",
                "--analytics requires --profile behavioral"
            );
            compile(
                cli.database_config.as_deref(),
                &libraries.join(&name),
                &envs.join(&name),
                &sources.join(&name),
                &absolute(&store)?,
                reinstall,
                embedder_of(embedder, &embed_url),
                &absolute(&generations)?,
                cpg_core::analyze::Techniques::parse(&analytics).map_err(|e| anyhow::anyhow!(e))?,
                profile,
                public_root,
                evidence_observations,
            )
        }
        Cmd::Bundle {
            store,
            snapshot,
            out,
        } => bundle(&absolute(&store)?, snapshot, &absolute(&out)?),
    }
}

fn main() -> ExitCode {
    cpg_extract::logging::init_logging();
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
