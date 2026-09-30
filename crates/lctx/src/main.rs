//! `lctx`: pinned Python libraries and their compilation (DESIGN §4.0, ADR-0013). Every analyzed
//! library is a committed uv project under `libraries/<name>/`.
//!
//! The semantic-model cutover exposes facts and normalized frontiers. Downstream commands return with
//! phases 4–5; unsupported frontiers exit 3 before acquisition or database effects.
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
//! lctx compile <name> --through facts|normalized --profile catalog|behavioral   publish; selection is explicit
//! ```
//! Common options: `--database FILE` (the protected `postgres.json`; its siblings select the roles),
//! `--libraries DIR` (default `libraries`), `--envs DIR` (default `build/envs`), `--sources DIR`
//! (default `build/sources`). Exit status: 0 ok, 1 error, 2 refused, 3 unavailable.

mod compile;
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
#[command(
    name = "lctx",
    version,
    about = "Pinned Python libraries and their compilation"
)]
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
    /// Publish facts or normalized records and graph snapshots; selection is explicit.
    Compile {
        name: String,
        #[arg(long)]
        through: String,
        #[arg(long,default_value="catalog",value_parser=compile::profile)]
        profile: lctx_model::domain::stages::Profile,
        /// Explicit captured task report, repeatable; never executes analyzed code.
        #[arg(long)]
        task_receipt: Vec<PathBuf>,
        #[arg(long,default_value_t=lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)]
        memory_bytes: usize,
    },
    /// Inspect one generated Python file's flow facts as JSON (runtime oracle input).
    Flow {
        file: PathBuf,
        /// Analyzed Python interpreter version, MAJOR.MINOR.MICRO.
        #[arg(long, default_value = "3.14.7")]
        python: String,
        #[arg(long, default_value = "linux")]
        platform: String,
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
    let store = |e: &Store| {
        matches!(
            e,
            Store::State
                | Store::Absent
                | Store::NotInstalled
                | Store::Busy
                | Store::Contract
                | Store::Frontier(_)
                | Store::Confirmation
                | Store::Orphaned
        )
    };
    error.chain().any(|cause| {
        cause.is::<Refused>()
            || cause.downcast_ref::<Store>().is_some_and(store)
            || cause.downcast_ref::<ReadError>().is_some_and(|e| match e {
                ReadError::Frontier(_) => true,
                ReadError::Store(inner) => store(inner),
                _ => false,
            })
    })
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

fn flow_file(file: &Path, python: &str, platform: &str) -> anyhow::Result<()> {
    if python != "3.14.7" || platform != "linux" {
        return Err(anyhow::anyhow!(
            "the flow probe uses the pinned 3.14.7/linux context"
        ));
    }
    use lctx_model::domain::{
        assertion::*, conditions::*, flow::*, lexical::BindingEvent, resources::ResourceBudget,
        source::*, stages::Profile, syntax::SubjectBoundary, value::*, *,
    };
    use std::sync::Arc;
    let budget = ResourceBudget::fixed(1 << 30)?;
    let name = file
        .file_name()
        .context("flow file has no name")?
        .to_str()
        .context("flow file name is not UTF-8")?
        .to_owned();
    let captured = cpg_extract::capture::CapturedInput::capture(
        file.parent().unwrap_or(Path::new(".")),
        &[name],
        &budget,
    )?;
    let captured = Arc::new(cpg_extract::bundle::CapturedInputs::new(vec![
        cpg_extract::acquisition::AcquiredInput::tree(captured, "flow-probe"),
    ]));
    let (model, generation, digest) = tokio::runtime::Runtime::new()?.block_on(
        cpg_core::facts::inspect(captured, budget.clone(), Profile::Behavioral),
    )?;
    let occurrences = generation.read::<Occurrence>(&model, &budget)?;
    let uses = generation.read::<FlowUse>(&model, &budget)?;
    let definitions = generation.read::<FlowDefinition>(&model, &budget)?;
    let def_observations = generation.read::<FlowDefinitionObservation>(&model, &budget)?;
    let qs = generation.read::<AssertionQualification>(&model, &budget)?;
    let conditions = generation.read::<Condition>(&model, &budget)?;
    let nodes = generation.read::<ConditionNode>(&model, &budget)?;
    let places = generation.read::<Place>(&model, &budget)?;
    let roots = generation.read::<PlaceRoot>(&model, &budget)?;
    let paths = generation.read::<AccessPath>(&model, &budget)?;
    let segments = generation.read::<PathSegment>(&model, &budget)?;
    let targets = generation.read::<ReachingDefinition>(&model, &budget)?;
    let events = generation.read::<BindingEvent>(&model, &budget)?;
    let at = |id| {
        occurrences
            .rows()
            .iter()
            .find(|o| o.id() == id)
            .context("flow occurrence missing")
    };
    let span = |id| -> anyhow::Result<_> {
        let o = at(id)?;
        Ok(serde_json::json!([o.start, o.end]))
    };
    let condition = |id| -> anyhow::Result<_> {
        let q = qs
            .rows()
            .iter()
            .find(|q| q.id() == id)
            .context("flow qualification missing")?;
        let c = conditions
            .rows()
            .iter()
            .find(|c| c.id() == q.condition)
            .context("flow condition missing")?;
        let d = Diagram::from_records(c, nodes.rows())?;
        Ok(
            serde_json::json!({"id":c.id().hex(),"root":c.root.hex(),"is_false":d.is_false(),"is_true":d.is_true(),"approximation":format!("{:?}",q.approximation)}),
        )
    };
    let place_name = |id| -> anyhow::Result<String> {
        let p = places
            .rows()
            .iter()
            .find(|p| p.id() == id)
            .context("flow place missing")?;
        let root = roots
            .rows()
            .iter()
            .find(|r| r.id() == p.root)
            .context("flow place root missing")?;
        let name = match root {
            PlaceRoot::Formal { declaration } => events
                .rows()
                .iter()
                .find(|e| e.site == *declaration)
                .context("formal binding event missing")?
                .name
                .clone(),
            PlaceRoot::Local { name, .. } | PlaceRoot::Global { name, .. } => name.clone(),
            _ => format!("{root:?}"),
        };
        let path = paths
            .rows()
            .iter()
            .find(|a| a.id() == p.path)
            .context("flow access path missing")?;
        let mut out = name;
        for id in [path.first, path.second].into_iter().flatten() {
            match segments
                .rows()
                .iter()
                .find(|s| s.id() == id)
                .context("flow path segment missing")?
            {
                PathSegment::Attribute { name } => {
                    out.push('.');
                    out.push_str(name);
                }
                PathSegment::Item { .. } | PathSegment::AnyItem => out.push_str("[item]"),
            }
        }
        Ok(out)
    };
    let result = serde_json::json!({
        "model":model.digest().hex(),"content":digest.hex(),
        "uses":uses.rows().iter().map(|u|Ok(serde_json::json!({"id":u.id().hex(),"occurrence":u.occurrence.hex(),"place_id":u.place.hex(),"place":place_name(u.place)?,"span":span(u.occurrence)?}))).collect::<anyhow::Result<Vec<_>>>()?,
        "definitions":definitions.rows().iter().map(|d|{let obs=def_observations.rows().iter().find(|o|o.definition==d.id()).context("flow definition observation missing")?;Ok(serde_json::json!({"id":d.id().hex(),"place_id":d.place.hex(),"place":place_name(d.place)?,"target":span(d.occurrence)?,"kind":format!("{:?}",obs.kind)}))}).collect::<anyhow::Result<Vec<_>>>()?,
        "reaching":generation.read::<FlowReachingObservation>(&model,&budget)?.rows().iter().map(|r|{let target=targets.rows().iter().find(|t|t.id()==r.target).context("flow reaching target missing")?;Ok(serde_json::json!({"id":r.id().hex(),"use":r.use_.hex(),"definition":match target {ReachingDefinition::Bound {definition}=>Some(definition.hex()),_=>None},"target":format!("{target:?}"),"condition":condition(r.qualification)?,"loop_carried":r.loop_carried}))}).collect::<anyhow::Result<Vec<_>>>()?,
        "values":generation.read::<FlowValueObservation>(&model,&budget)?.rows().iter().map(|v|Ok(serde_json::json!({"id":v.id().hex(),"sink":format!("{:?}",v.kind),"span":span(v.sink)?,"use":v.use_.hex(),"identity":v.transfer==lctx_model::domain::transfer::TransferKind::Identity,"through_call":v.through_call,"condition":condition(v.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
        "regions":generation.read::<FlowRegionObservation>(&model,&budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"span":span(r.statement)?,"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
        "tests":generation.read::<FlowTestObservation>(&model,&budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"test":r.test.hex(),"span":span(r.test)?,"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
        "test_leaves":generation.read::<FlowTestLeafObservation>(&model,&budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"test":r.test.hex(),"atom":r.atom.hex(),"operand":r.operand.map(|id|id.hex()),"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
        "attribute_loads":generation.read::<FlowAttributeLoadObservation>(&model,&budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"qualification":r.qualification.hex(),"occurrence":r.occurrence.hex(),"name":r.name})).collect::<Vec<_>>(),
        "evaluation_atoms":generation.read::<EvaluationAtom>(&model,&budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"evaluation":r.evaluation.hex(),"context":r.context.hex(),"predicate":r.predicate.hex(),"operand":r.operand.map(|id|id.hex())})).collect::<Vec<_>>(),
        "predicates":generation.read::<Predicate>(&model,&budget)?.rows().iter().map(|r|match r {Predicate::IsNone=>serde_json::json!({"id":r.id().hex(),"kind":"is_none"}),Predicate::IsValue {value}=>serde_json::json!({"id":r.id().hex(),"kind":"is_value","value":value.hex()}),Predicate::Equals {value}=>serde_json::json!({"id":r.id().hex(),"kind":"equals","value":value.hex()}),Predicate::MemberOf {values}=>serde_json::json!({"id":r.id().hex(),"kind":"member_of","values":values.hex()}),Predicate::Truthy=>serde_json::json!({"id":r.id().hex(),"kind":"truthy"}),Predicate::IsInstance {class_expression}=>serde_json::json!({"id":r.id().hex(),"kind":"is_instance","class_expression":class_expression}),Predicate::TypeIs {class_expression}=>serde_json::json!({"id":r.id().hex(),"kind":"type_is","class_expression":class_expression}),Predicate::Opaque {text}=>serde_json::json!({"id":r.id().hex(),"kind":"opaque","text":text}),Predicate::InvokedGuard {source}=>serde_json::json!({"id":r.id().hex(),"kind":"invoked_guard","source":source.hex()}),Predicate::BoundGuard {source}=>serde_json::json!({"id":r.id().hex(),"kind":"bound_guard","source":source.hex()})}).collect::<Vec<_>>(),
        "call_paths":generation.read::<FlowCallPath>(&model,&budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"steps_digest":r.steps.hex()})).collect::<Vec<_>>(),
        "call_steps":generation.read::<FlowCallStep>(&model,&budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"path":r.path.hex(),"ordinal":r.ordinal,"call":r.call.hex(),"operand":r.operand.hex(),"role":format!("{:?}",r.role)})).collect::<Vec<_>>(),
        "value_paths":generation.read::<FlowValuePathObservation>(&model,&budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"qualification":r.qualification.hex(),"value":r.value.hex(),"path":r.path.hex()})).collect::<Vec<_>>(),
        "conditions":conditions.rows().iter().map(|c|serde_json::json!({"id":c.id().hex(),"root":c.root.hex()})).collect::<Vec<_>>(),
        "condition_nodes":nodes.rows().iter().map(|node|match node {ConditionNode::False=>serde_json::json!({"id":node.id().hex(),"kind":"false"}),ConditionNode::True=>serde_json::json!({"id":node.id().hex(),"kind":"true"}),ConditionNode::Branch {atom,low,high}=>serde_json::json!({"id":node.id().hex(),"kind":"branch","atom":atom.hex(),"low":low.hex(),"high":high.hex()})}).collect::<Vec<_>>(),
        "boundaries":generation.read::<SubjectBoundary>(&model,&budget)?.rows().iter().map(|b|serde_json::json!({"subject":b.subject.map(|id|id.hex()),"reason":format!("{:?}",b.reason),"detail":b.detail})).collect::<Vec<_>>()
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
        Cmd::Model {
            command: ModelCommand::Describe { format },
        } => {
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
            let inventory = cpg_extract::acquisition::inventory_installed(
                &libraries.join(&name),
                &envs.join(&name),
            )?;
            let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 30)?;
            let captured = cpg_extract::acquisition::capture(&inventory, &budget)?;
            let mut identity =
                serde_json::to_value(cpg_extract::deployment::identity(&captured.inputs()[0])?)?;
            // These two hashes are observations by the explicit task operator. The facts
            // compiler retains them as reports; neither expands analyzer acquisition.
            fn reported_hash(path: &Path) -> anyhow::Result<lctx_model::domain::ContentHash> {
                use std::io::Read;
                let mut file = std::fs::File::open(path)?;
                let mut buffer = [0u8; 65536];
                let mut hash = lctx_model::domain::ContentHasher::default();
                loop {
                    let count = file.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
                Ok(hash.finish())
            }
            let environment = envs.join(&name);
            identity["runtime_digest"] =
                serde_json::to_value(reported_hash(&environment.join("pyvenv.cfg"))?)?;
            identity["interpreter_digest"] =
                serde_json::to_value(reported_hash(&environment.join("bin/python"))?)?;
            println!("{}", serde_json::to_string(&identity)?);
            Ok(())
        }
        Cmd::Compile {
            name,
            through,
            profile,
            task_receipt,
            memory_bytes,
        } => {
            if !matches!(through.as_str(), "facts" | "normalized") {
                return Err(Unavailable(
                    "only --through facts or normalized is available during the semantic cutover",
                )
                .into());
            }
            cpg_extract::bundle::refuse_ambient(std::env::vars_os())?;
            runtime()?.block_on(compile::compile(
                &name,
                profile,
                if through == "facts" {
                    lctx_model::domain::admission::Frontier::Facts
                } else {
                    lctx_model::domain::admission::Frontier::Normalized
                },
                &task_receipt,
                memory_bytes,
                &libraries,
                &envs,
                &sources,
                cli.database.as_deref(),
            ))
        }
        Cmd::Flow {
            file,
            python,
            platform,
        } => flow_file(&file, &python, &platform),
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
        assert!(matches!(
            cli.command,
            Cmd::Acquire {
                reinstall: false,
                ..
            }
        ));
        assert_eq!(cli.envs, std::path::PathBuf::from("e"));
        // An option of another command is refused, not ignored.
        assert!(parse(&["acquire", "fastmcp", "--store", "s"]).is_err());
        // Retired pipeline commands no longer parse.
        for retired in [
            "bundle",
            "diff",
            "serving",
            "rebuild",
            "parity",
            "db",
            "snapshots",
            "generations",
            "compile-fixture",
        ] {
            assert!(parse(&[retired]).is_err(), "{retired}");
        }
        // Compile parses any arguments so that it can refuse them all.
        assert!(matches!(
            parse(&["compile", "fastmcp", "--through", "facts"])
                .unwrap()
                .command,
            Cmd::Compile { .. }
        ));
    }

    /// The hand parsers accepted a sign (`from_str_radix` reads `+f`) and panicked slicing
    /// non-ASCII input (H1 C4).
    #[test]
    fn an_attempt_id_is_exactly_32_hex_digits() {
        for bad in [
            "+f".repeat(16),
            "é".repeat(16),
            "ab".repeat(15),
            "zz".repeat(16),
        ] {
            assert!(parse(&["runs", "show", &bad]).is_err(), "{bad}");
        }
        assert!(parse(&["runs", "show", &"AB".repeat(16)]).is_ok());
    }
}
