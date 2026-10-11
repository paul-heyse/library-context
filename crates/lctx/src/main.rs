//! `lctx` acquires pinned library inputs and compiles admitted graph artifacts.
//! `compile --artifact-only --output DIR` exports a native-backed artifact; ordinary compilation
//! publishes an immutable native snapshot without changing the selected handle.

mod acquisition;
mod compile;
mod compile_options;
mod model;
mod newnative;
mod propose;

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
use lctx_workspace_hack as _; // Contributes Cargo features, not callable APIs (ADR-0136).

/// The command line (H1 C4: clap derive; each command takes only its own options).
#[derive(Parser, Debug)]
#[command(
    name = "lctx",
    version,
    about = "Pinned Python libraries and their compilation"
)]
struct Cli {
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
    /// Compile a cumulative admitted graph and publish it, or explicitly export its complete artifact.
    Compile {
        name: String,
        /// Compile and export without publication.
        #[arg(long, requires = "output")]
        artifact_only: bool,
        /// Destination for the admitted artifact; must not already exist.
        #[arg(long, requires = "artifact_only")]
        output: Option<PathBuf>,
        /// Native runtime configuration for compilation and explicit export.
        #[arg(long)]
        runtime_config: Option<PathBuf>,
        #[arg(long)]
        through: String,
        #[arg(long,default_value="catalog",value_parser=compile::profile)]
        profile: lctx_model::domain::stages::Profile,
        /// Explicit captured task report, repeatable; never executes analyzed code.
        #[arg(long)]
        task_receipt: Vec<PathBuf>,
        #[arg(long,default_value_t=lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)]
        memory_bytes: usize,
        /// Optional analytics: default or comma-separated +technique/-technique.
        #[arg(long, allow_hyphen_values = true)]
        techniques: Option<String>,
        #[arg(long, value_enum)]
        embedder: Option<compile_options::EmbeddingChoice>,
        #[arg(long)]
        embedding_endpoint: Option<String>,
        #[arg(long)]
        embedding_spec: Option<PathBuf>,
    },
    /// Publish an exported trusted compiler artifact without selecting it.
    PublishArtifact {
        artifact: PathBuf,
        #[arg(long, default_value_os_t = newnative::default_config())]
        runtime_config: PathBuf,
        #[arg(long, default_value_t = lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)]
        memory_bytes: usize,
    },
    /// Read or explicitly select a complete immutable snapshot handle.
    Snapshot {
        #[arg(long, global = true, default_value_os_t = newnative::default_config())]
        runtime_config: PathBuf,
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    /// Execute one public tool against a fixed native snapshot.
    Tool {
        #[arg(value_parser = public_tool)]
        tool: String,
        /// JSON request file, validated against the Rust public contract.
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        handle: Option<PathBuf>,
        #[arg(long, default_value_os_t = newnative::default_config())]
        runtime_config: PathBuf,
    },
    /// Explicit initialization or readiness of the configured native control database.
    Store {
        #[arg(long, global = true, default_value_os_t = newnative::default_config())]
        runtime_config: PathBuf,
        #[command(subcommand)]
        command: StoreCommand,
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
enum SnapshotCommand {
    /// List complete live publications in the configured namespace.
    List,
    /// Stream one explicit snapshot through canonical/original and realization verification.
    Audit { handle: PathBuf },
    /// Back up a published snapshot to a new local SQL dump, excluding credentials/history.
    Backup {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        handle: Option<PathBuf>,
    },
    /// Restore a trusted current-format SQL dump into a fresh, unselected published snapshot.
    Restore {
        input: PathBuf,
        /// Publication digest to recover from a multi-publication backup.
        #[arg(long,value_parser=content_hash)]
        publication: Option<lctx_model::domain::ContentHash>,
    },
    /// Retire an unselected snapshot after every known reader process has been stopped.
    Retire {
        handle: PathBuf,
        #[arg(long, required = true)]
        readers_stopped: bool,
    },
    /// Export one complete admitted input/context topology, including its coverage and gaps.
    Export {
        #[arg(long, value_parser = lctx_surrealdb::projections::name)]
        projection: lctx_model::domain::projection::ProjectionName,
        /// Captured input revision ID, as 32 hexadecimal digits.
        #[arg(long, value_parser = nominal_id::<lctx_model::domain::input::InputRevision>)]
        input: lctx_model::domain::Id<lctx_model::domain::input::InputRevision>,
        /// Analysis context ID, as 32 hexadecimal digits.
        #[arg(long, value_parser = nominal_id::<lctx_model::domain::attribution::AnalysisContext>)]
        context: lctx_model::domain::Id<lctx_model::domain::attribution::AnalysisContext>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        handle: Option<PathBuf>,
        #[arg(long, default_value_t = lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)]
        memory_bytes: usize,
    },
    /// Validate the published marker, then atomically select this handle.
    Select { handle: PathBuf },
    /// Validate and print the explicit handle, or the current selection.
    Show {
        #[arg(long)]
        handle: Option<PathBuf>,
    },
    /// Read model-declared relation bodies within one exact published view.
    Query {
        relation: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long)]
        handle: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum StoreCommand {
    Init {
        /// Install under maintenance without reopening native borrower admission.
        #[arg(long)]
        keep_closed: bool,
    },
    Check,
    /// Print this executable's native schema identity without accessing stored state.
    Schema,
    /// Print the exact resumable upgrade contract without accessing stored state.
    UpgradeContract {
        #[arg(long, value_parser=content_hash)]
        expected_schema: lctx_model::domain::ContentHash,
        #[arg(long, value_parser=content_hash)]
        operation: lctx_model::domain::ContentHash,
        #[arg(long, value_parser=content_hash)]
        execution: lctx_model::domain::ContentHash,
    },
    /// Independently check base installation and pinned executable definitions.
    UpgradeCheck,
    /// Qualify atomic progress primitives on the exact owned validation upgrade slot.
    UpgradeQualify {
        #[arg(long, value_parser=content_hash)]
        expected_schema: lctx_model::domain::ContentHash,
        #[arg(long, value_parser=content_hash)]
        operation: lctx_model::domain::ContentHash,
        #[arg(long)]
        execution_contract: PathBuf,
    },
    /// Upgrade one exact legacy scope while its service owner holds closed maintenance.
    Upgrade {
        #[arg(long, value_parser=content_hash)]
        expected_schema: lctx_model::domain::ContentHash,
        #[arg(long, value_parser=content_hash)]
        operation: lctx_model::domain::ContentHash,
        #[arg(long)]
        execution_contract: PathBuf,
        /// Stop after durable checkpoints; only an explicit execution contract permits stepping.
        #[arg(long, requires="execution_contract", value_parser=clap::value_parser!(u64).range(1..))]
        stop_after_pages: Option<u64>,
    },
    /// Explicit drained maintenance of retained control history.
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
    /// Claim distinct maintenance successors for exact interrupted obligations.
    Recover {
        #[arg(long, value_parser=content_hash)]
        cleanup: Vec<lctx_model::domain::ContentHash>,
        #[arg(long, value_parser=content_hash)]
        retirement: Vec<lctx_model::domain::ContentHash>,
        #[arg(long, default_value_t = 128)]
        limit: usize,
    },
    /// Maintenance-only durable effect reconciliation and drainage barrier.
    Drain,
    /// Release named abandoned owners after explicit predecessor drainage under closed maintenance.
    Reconcile {
        #[arg(long, value_parser=content_hash)]
        pin: Vec<lctx_model::domain::ContentHash>,
        #[arg(long, value_parser=content_hash)]
        backup_hold: Vec<lctx_model::domain::ContentHash>,
        #[arg(long, required = true)]
        readers_stopped: bool,
    },
    /// Maintenance-only fencing of native borrower admission.
    CloseAdmission,
    /// Reopen native borrowers after checked maintenance recovery.
    OpenAdmission,
}

#[derive(Subcommand, Debug)]
enum HistoryCommand {
    /// Record the reviewed and qualified consumer inventory before allowing collection.
    Qualify {
        #[arg(long, value_parser=content_hash)]
        evidence: lctx_model::domain::ContentHash,
    },
    /// Permanently close the old issuance era after actual drainage.
    Cut,
    /// Collect one bounded page behind a qualified permanent fence.
    Compact {
        #[arg(long, default_value_t = 128)]
        limit: usize,
    },
    /// Continue the same persisted collector without refreshing its issuance authority.
    Resume {
        #[arg(long, value_parser=content_hash)]
        identity: lctx_model::domain::ContentHash,
        /// Revision returned by the previous page; retry it to recover the same outcome.
        #[arg(long)]
        expected_revision: u64,
        #[arg(long, default_value_t = 128)]
        limit: usize,
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
    error
        .chain()
        .any(|cause| cause.is::<Refused>() || cause.is::<lctx_model::domain::serving::WireError>())
}

fn content_hash(raw: &str) -> Result<lctx_model::domain::ContentHash, String> {
    if raw.len() != 64 {
        return Err("content digest must contain 64 hexadecimal digits".into());
    }
    let mut bytes = [0u8; 32];
    for (index, pair) in raw.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair).map_err(|_| "content digest must be hexadecimal")?;
        bytes[index] =
            u8::from_str_radix(pair, 16).map_err(|_| "content digest must be hexadecimal")?;
    }
    Ok(lctx_model::domain::ContentHash(bytes))
}

fn nominal_id<T>(raw: &str) -> Result<lctx_model::domain::Id<T>, String> {
    if raw.len() != 32 {
        return Err("nominal ID must contain 32 hexadecimal digits".into());
    }
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|_| "nominal ID must be hexadecimal")?;
            u8::from_str_radix(pair, 16).map_err(|_| "nominal ID must be hexadecimal")
        })
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::from_value(serde_json::json!(bytes)).map_err(|error| error.to_string())
}

fn public_tool(name: &str) -> Result<String, String> {
    lctx_model::domain::serving::Tool::from_name(name)
        .map(|tool| tool.name().to_owned())
        .map_err(|error| error.to_string())
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

fn init(
    name: &str,
    requirement: &str,
    python: &str,
    libraries: &Path,
    envs: &Path,
    sources: &Path,
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
    let mut acquired = acquisition::Lease::open(
        &library_dir,
        &env_dir,
        &sources.join(name),
        true,
        false,
        false,
    )?;
    let site = fs_err::read_dir(acquired.environment.join("lib"))?
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
            Some((library::normalize(stem.rsplit_once('-')?.0).ok()?, metadata))
        })
        .collect();
    let (release, found) = propose::propose(&dist, &installed);
    acquired.finish()?;
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

fn flow_workspace_options(
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> cpg_core::workspace::WorkspaceOptions {
    cpg_core::workspace::WorkspaceOptions {
        memory_bytes: budget.limit(),
        ..Default::default()
    }
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
    let budget = ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES)?;
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
    let captured = Arc::new(cpg_extract::bundle::CapturedInputs::new(
        vec![cpg_extract::acquisition::AcquiredInput::tree(
            captured,
            "flow-probe",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(Profile::Behavioral, &budget)?,
    ));
    let runtime = tokio::runtime::Runtime::new()?;
    let native_path = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(crate::newnative::default_config);
    let native_config = crate::newnative::config(&native_path)?;
    let native = runtime.block_on(lctx_surrealdb::compiler::NativeCompilerStore::begin(
        &native_config,
        lctx_model::domain::admission::Frontier::Facts,
    ))?;
    let (model, workspace) = match runtime.block_on(cpg_core::facts::inspect(
        captured,
        flow_workspace_options(&budget),
        Profile::Behavioral,
        native.clone(),
    )) {
        Ok(result) => result,
        Err(error) => {
            native.fail();
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "facts inspection abandon",
                runtime.block_on(native.abandon()),
            );
            return lctx_model::domain::completion::complete::<()>(Err(error), completion)
                .map_err(Into::into);
        }
    };
    let output = (|| -> anyhow::Result<String> {
        let digest = workspace.identity()?;
        fn read<R: Record>(
            workspace: &cpg_core::workspace::Workspace,
            budget: &ResourceBudget,
        ) -> Result<Batch<R>, ModelError> {
            let mut rows = Vec::new();
            for batch in workspace
                .completed::<R>()?
                .read::<R>(workspace.model().clone(), budget.clone())?
            {
                rows.extend(batch?.rows().iter().cloned());
            }
            Batch::new(workspace.model(), rows, budget)
        }
        let occurrences = read::<Occurrence>(&workspace, &budget)?;
        let uses = read::<FlowUse>(&workspace, &budget)?;
        let definitions = read::<FlowDefinition>(&workspace, &budget)?;
        let def_observations = read::<FlowDefinitionObservation>(&workspace, &budget)?;
        let qs = read::<AssertionQualification>(&workspace, &budget)?;
        let conditions = read::<Condition>(&workspace, &budget)?;
        let nodes = read::<ConditionNode>(&workspace, &budget)?;
        let places = read::<Place>(&workspace, &budget)?;
        let roots = read::<PlaceRoot>(&workspace, &budget)?;
        let paths = read::<AccessPath>(&workspace, &budget)?;
        let segments = read::<PathSegment>(&workspace, &budget)?;
        let targets = read::<ReachingDefinition>(&workspace, &budget)?;
        let events = read::<BindingEvent>(&workspace, &budget)?;
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
            "reaching":read::<FlowReachingObservation>(&workspace, &budget)?.rows().iter().map(|r|{let target=targets.rows().iter().find(|t|t.id()==r.target).context("flow reaching target missing")?;Ok(serde_json::json!({"id":r.id().hex(),"use":r.use_.hex(),"definition":match target {ReachingDefinition::Bound {definition}=>Some(definition.hex()),_=>None},"target":format!("{target:?}"),"condition":condition(r.qualification)?,"loop_carried":r.loop_carried}))}).collect::<anyhow::Result<Vec<_>>>()?,
            "values":read::<FlowValueObservation>(&workspace, &budget)?.rows().iter().map(|v|Ok(serde_json::json!({"id":v.id().hex(),"sink":format!("{:?}",v.kind),"span":span(v.sink)?,"use":v.use_.hex(),"identity":v.transfer==lctx_model::domain::transfer::TransferKind::Identity,"through_call":v.through_call,"condition":condition(v.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
            "regions":read::<FlowRegionObservation>(&workspace, &budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"span":span(r.statement)?,"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
            "tests":read::<FlowTestObservation>(&workspace, &budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"test":r.test.hex(),"span":span(r.test)?,"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
            "test_leaves":read::<FlowTestLeafObservation>(&workspace, &budget)?.rows().iter().map(|r|Ok(serde_json::json!({"id":r.id().hex(),"test":r.test.hex(),"atom":r.atom.hex(),"operand":r.operand.map(|id|id.hex()),"condition":condition(r.qualification)?}))).collect::<anyhow::Result<Vec<_>>>()?,
            "attribute_loads":read::<FlowAttributeLoadObservation>(&workspace, &budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"qualification":r.qualification.hex(),"occurrence":r.occurrence.hex(),"name":r.name})).collect::<Vec<_>>(),
            "evaluation_atoms":read::<EvaluationAtom>(&workspace, &budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"evaluation":r.evaluation.hex(),"context":r.context.hex(),"predicate":r.predicate.hex(),"operand":r.operand.map(|id|id.hex())})).collect::<Vec<_>>(),
            "predicates":read::<Predicate>(&workspace, &budget)?.rows().iter().map(|r|match r {Predicate::IsNone=>serde_json::json!({"id":r.id().hex(),"kind":"is_none"}),Predicate::IsValue {value}=>serde_json::json!({"id":r.id().hex(),"kind":"is_value","value":value.hex()}),Predicate::Equals {value}=>serde_json::json!({"id":r.id().hex(),"kind":"equals","value":value.hex()}),Predicate::MemberOf {values}=>serde_json::json!({"id":r.id().hex(),"kind":"member_of","values":values.hex()}),Predicate::Truthy=>serde_json::json!({"id":r.id().hex(),"kind":"truthy"}),Predicate::IsInstance {class_expression}=>serde_json::json!({"id":r.id().hex(),"kind":"is_instance","class_expression":class_expression}),Predicate::TypeIs {class_expression}=>serde_json::json!({"id":r.id().hex(),"kind":"type_is","class_expression":class_expression}),Predicate::Opaque {text}=>serde_json::json!({"id":r.id().hex(),"kind":"opaque","text":text}),Predicate::InvokedGuard {source}=>serde_json::json!({"id":r.id().hex(),"kind":"invoked_guard","source":source.hex()}),Predicate::BoundGuard {source}=>serde_json::json!({"id":r.id().hex(),"kind":"bound_guard","source":source.hex()}),Predicate::NonTerminalCall {awaiting}=>serde_json::json!({"id":r.id().hex(),"kind":"nonterminal_call","awaiting":awaiting}),Predicate::NonEmptyIterable=>serde_json::json!({"id":r.id().hex(),"kind":"nonempty_iterable"}),Predicate::ContextManagerSuppresses {asynchronous}=>serde_json::json!({"id":r.id().hex(),"kind":"context_manager_suppresses","asynchronous":asynchronous}),Predicate::FinallyNormalPathImpossible=>serde_json::json!({"id":r.id().hex(),"kind":"finally_normal_path_impossible"})}).collect::<Vec<_>>(),
            "call_paths":read::<FlowCallPath>(&workspace, &budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"steps_digest":r.steps.hex()})).collect::<Vec<_>>(),
            "call_steps":read::<FlowCallStep>(&workspace, &budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"path":r.path.hex(),"ordinal":r.ordinal,"call":r.call.hex(),"operand":r.operand.hex(),"role":format!("{:?}",r.role)})).collect::<Vec<_>>(),
            "value_paths":read::<FlowValuePathObservation>(&workspace, &budget)?.rows().iter().map(|r|serde_json::json!({"id":r.id().hex(),"qualification":r.qualification.hex(),"value":r.value.hex(),"path":r.path.hex()})).collect::<Vec<_>>(),
            "conditions":conditions.rows().iter().map(|c|serde_json::json!({"id":c.id().hex(),"root":c.root.hex()})).collect::<Vec<_>>(),
            "condition_nodes":nodes.rows().iter().map(|node|match node {ConditionNode::False=>serde_json::json!({"id":node.id().hex(),"kind":"false"}),ConditionNode::True=>serde_json::json!({"id":node.id().hex(),"kind":"true"}),ConditionNode::Branch {atom,low,high}=>serde_json::json!({"id":node.id().hex(),"kind":"branch","atom":atom.hex(),"low":low.hex(),"high":high.hex()})}).collect::<Vec<_>>(),
            "boundaries":read::<SubjectBoundary>(&workspace, &budget)?.rows().iter().map(|b|serde_json::json!({"subject":b.subject.map(|id|id.hex()),"reason":format!("{:?}",b.reason),"detail":b.detail})).collect::<Vec<_>>()
        });
        Ok(serde_json::to_string(&result)?)
    })();
    let mut completion = runtime.block_on(workspace.drain_report());
    completion.step(
        "facts inspection abandon",
        runtime.block_on(native.abandon()),
    );
    let output = lctx_model::domain::completion::complete(
        output.map_err(crate::newnative::operation_error),
        completion,
    )?;
    println!("{output}");
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
            let described = model::describe(&lctx_model::domain::model()?);
            match format {
                model::Format::Json => println!("{}", serde_json::to_string_pretty(&described)?),
                model::Format::Text => print!("{}", model::text(&described)),
            }
            Ok(())
        }
        Cmd::Library {
            command:
                LibraryCommand::Init {
                    name,
                    requirement,
                    python,
                },
        } => init(&name, &requirement, &python, &libraries, &envs, &sources),
        Cmd::Acquire { name, reinstall } => {
            let library_dir = libraries.join(&name);
            acquisition::Lease::open(
                &library_dir,
                &envs.join(&name),
                &sources.join(&name),
                true,
                reinstall,
                true,
            )?
            .finish()
        }
        Cmd::DeploymentIdentity { name } => {
            let mut acquired = acquisition::Lease::open(
                &libraries.join(&name),
                &envs.join(&name),
                &sources.join(&name),
                false,
                false,
                false,
            )?;
            let inventory = cpg_extract::acquisition::inventory_installed(
                &libraries.join(&name),
                &acquired.environment,
            )?;
            let budget = lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )?;
            let captured = cpg_extract::acquisition::capture(
                &inventory,
                &budget,
                cpg_extract::native_context::NativeContextConfig::committed(
                    lctx_model::domain::stages::Profile::Catalog,
                    &budget,
                )?,
            )?;
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
            let environment = &acquired.environment;
            identity["runtime_digest"] =
                serde_json::to_value(reported_hash(&environment.join("pyvenv.cfg"))?)?;
            identity["interpreter_digest"] =
                serde_json::to_value(reported_hash(&environment.join("bin/python"))?)?;
            acquired.finish()?;
            println!("{}", serde_json::to_string(&identity)?);
            Ok(())
        }
        Cmd::Compile {
            name,
            artifact_only,
            output,
            runtime_config,
            through,
            profile,
            task_receipt,
            memory_bytes,
            techniques,
            embedder,
            embedding_endpoint,
            embedding_spec,
        } => {
            if artifact_only && output.as_ref().is_some_and(|output| output.exists()) {
                return Err(Refused("artifact destination already exists".into()).into());
            }
            let runtime_config = runtime_config.unwrap_or_else(newnative::default_config);
            let target = if artifact_only {
                compile::Target::Artifact(
                    output
                        .as_deref()
                        .expect("clap requires an artifact destination"),
                    &runtime_config,
                )
            } else {
                compile::Target::Native(&runtime_config)
            };
            if !matches!(
                through.as_str(),
                "facts" | "normalized" | "analysis" | "catalog"
            ) {
                return Err(Unavailable(
                    "--through serving is unavailable; use facts, normalized, analysis or catalog",
                )
                .into());
            }
            cpg_extract::bundle::refuse_ambient(std::env::vars_os())?;
            runtime()?.block_on(compile::compile(
                &name,
                profile,
                lctx_model::domain::admission::Frontier::ALL
                    .into_iter()
                    .find(|frontier| frontier.name() == through)
                    .expect("validated compile frontier"),
                &task_receipt,
                memory_bytes,
                &compile_options::Options {
                    techniques,
                    embedder,
                    embedding_endpoint,
                    embedding_spec,
                },
                &libraries,
                &envs,
                &sources,
                target,
            ))
        }
        Cmd::PublishArtifact {
            artifact,
            runtime_config,
            memory_bytes,
        } => {
            let config = newnative::config(&runtime_config)?;
            let handle =
                runtime()?.block_on(newnative::publish(&artifact, &config, memory_bytes))?;
            println!("{}", serde_json::to_string_pretty(&handle)?);
            Ok(())
        }
        Cmd::Snapshot {
            runtime_config,
            command,
        } => {
            let config = newnative::config(&runtime_config)?;
            let runtime = runtime()?;
            match command {
                SnapshotCommand::List => {
                    let handles = runtime.block_on(newnative::list(&config))?;
                    println!("{}", serde_json::to_string_pretty(&handles)?);
                }
                SnapshotCommand::Audit { handle } => {
                    let audited = runtime.block_on(newnative::audit(&config, &handle))?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({"handle":audited,"audit":"passed",
                        "checked":["publication","canonical graph","original bytes","graph adjacency","current semantic contract","definition identity"],
                        "outside_scope":["credentials","live queries","database mode"]})
                        )?
                    );
                }
                SnapshotCommand::Backup { output, handle } => {
                    runtime.block_on(newnative::backup(&config, handle.as_deref(), &output))?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({"output":output}))?
                    );
                }
                SnapshotCommand::Restore { input, publication } => {
                    let handle =
                        runtime.block_on(newnative::restore(&config, &input, publication))?;
                    println!("{}", serde_json::to_string_pretty(&handle)?);
                }
                SnapshotCommand::Retire {
                    handle,
                    readers_stopped,
                } => {
                    let progress =
                        runtime.block_on(newnative::retire(&config, &handle, readers_stopped))?;
                    println!("{}", serde_json::to_string_pretty(&progress)?);
                }
                SnapshotCommand::Export {
                    projection,
                    input,
                    context,
                    output,
                    handle,
                    memory_bytes,
                } => {
                    let key = lctx_model::domain::projection::normalization::ProjectionKey {
                        input,
                        context,
                        name: projection,
                    };
                    runtime.block_on(newnative::export(
                        &config,
                        handle.as_deref(),
                        key,
                        &output,
                        memory_bytes,
                    ))?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({"output":output,"key":key})
                        )?
                    );
                }
                SnapshotCommand::Select { handle } => {
                    let selected = runtime.block_on(newnative::select(&config, &handle))?;
                    println!("{}", serde_json::to_string_pretty(&selected)?);
                }
                SnapshotCommand::Show { handle } => {
                    let details = runtime.block_on(newnative::show(&config, handle.as_deref()))?;
                    println!("{}", serde_json::to_string_pretty(&details)?);
                }
                SnapshotCommand::Query {
                    relation,
                    limit,
                    handle,
                } => {
                    let response = runtime.block_on(newnative::query(
                        &config,
                        handle.as_deref(),
                        &relation,
                        limit,
                    ))?;
                    println!("{}", serde_json::to_string_pretty(&response)?);
                }
            }
            Ok(())
        }
        Cmd::Tool {
            tool,
            request,
            handle,
            runtime_config,
        } => {
            let raw = fs_err::read_to_string(&request)?;
            let config = newnative::config(&runtime_config)?;
            let response =
                runtime()?.block_on(newnative::tool(&config, handle.as_deref(), &tool, &raw))?;
            println!("{response}");
            Ok(())
        }
        Cmd::Store {
            runtime_config,
            command,
        } => {
            if matches!(&command, StoreCommand::Schema) {
                println!("{}", serde_json::to_string(&serde_json::json!({
                    "schema_version":lctx_surrealdb::control::SCHEMA_VERSION,
                    "schema":lctx_surrealdb::compiler::base_schema_identity().hex(),
                }))?);
                return Ok(());
            }
            let config = newnative::config(&runtime_config)?;
            if let StoreCommand::UpgradeContract { expected_schema, operation, execution } = &command {
                let contract = lctx_surrealdb::upgrade::UpgradeExecutionContract::current(
                    *operation, *execution, *expected_schema, &config);
                contract.validate(&config)?;
                println!("{}", serde_json::to_string(&contract)?);
                return Ok(());
            }
            let runtime = runtime()?;
            let reports_ready = !matches!(
                &command,
                StoreCommand::Init { keep_closed: true }
                    | StoreCommand::Drain
                    | StoreCommand::CloseAdmission
                    | StoreCommand::Reconcile { .. }
                    | StoreCommand::Upgrade { .. }
                    | StoreCommand::UpgradeCheck
                    | StoreCommand::UpgradeQualify { .. }
                    | StoreCommand::History { .. }
                    | StoreCommand::Recover { .. }
            );
            match command {
                StoreCommand::Init { keep_closed } => {
                    runtime.block_on(newnative::install(&config, keep_closed))?
                }
                StoreCommand::Check => {
                    runtime.block_on(newnative::ready(&config))?;
                }
                StoreCommand::Schema => unreachable!("metadata command returned above"),
                StoreCommand::UpgradeContract { .. } => unreachable!("metadata command returned above"),
                StoreCommand::UpgradeCheck => {
                    let definitions = runtime.block_on(async {
                        lctx_publisher::check_definitions(&config, &lctx_serving::native_definitions()).await
                    })?;
                    println!("{}", serde_json::json!({"ready":false,"definitions_complete":true,
                        "definitions":definitions.hex(),"namespace":config.namespace,"database":config.database}));
                    return Ok(());
                }
                StoreCommand::UpgradeQualify { expected_schema, operation, execution_contract } => {
                    anyhow::ensure!(config.database.as_str() == "validation", "upgrade qualification requires validation");
                    let contract: lctx_surrealdb::upgrade::UpgradeExecutionContract =
                        serde_json::from_slice(&std::fs::read(execution_contract)?)?;
                    anyhow::ensure!(contract.migration == operation && contract.source == expected_schema,
                        "upgrade qualification arguments differ from immutable execution contract");
                    let qualification = runtime.block_on(lctx_surrealdb::upgrade::qualify_upgrade_pages(&config, &contract))?;
                    println!("{}", serde_json::json!({"ready":false,"qualification":qualification,
                        "namespace":config.namespace,"database":config.database}));
                    return Ok(());
                }
                StoreCommand::Upgrade { expected_schema, operation, execution_contract, stop_after_pages } => {
                    let advance = runtime.block_on(async {
                        let blueprint=lctx_serving::native_definitions();
                        let advance = {
                            anyhow::ensure!(stop_after_pages.is_none() || config.database.as_str() == "validation",
                                "upgrade stepping is restricted to the owned validation scope");
                            let contract: lctx_surrealdb::upgrade::UpgradeExecutionContract =
                                serde_json::from_slice(&std::fs::read(execution_contract)?)?;
                            anyhow::ensure!(contract.migration == operation && contract.source == expected_schema,
                                "upgrade arguments differ from immutable execution contract");
                            lctx_surrealdb::upgrade::upgrade_with_contract(&config, &contract, &blueprint, stop_after_pages).await?
                        };
                        // Executable epochs have a separate owner from the base format marker.
                        // Repeat safely after unknown installation acknowledgement; stay closed.
                        if advance.published {
                            lctx_publisher::install_definitions(&config,&blueprint).await?;
                        }
                        anyhow::Ok(advance)
                    })?;
                    println!("{}", serde_json::json!({"ready":false,"definitions_complete":advance.published,
                        "upgrade":advance,"namespace":config.namespace,"database":config.database}));
                    return Ok(());
                }
                StoreCommand::History { command } => {
                    let receipt = runtime.block_on(async {
                        use lctx_model::domain::{ModelError, completion::{Completion, complete}};
                        let client = lctx_surrealdb::upgrade::maintenance_client(&config).await?;
                        let result = async {
                            match command {
                                HistoryCommand::Qualify { evidence } => {
                                    lctx_surrealdb::control::qualify_history_inventory(&client, evidence).await?;
                                    Ok(serde_json::json!({"inventory":evidence.hex()}))
                                }
                                HistoryCommand::Cut => serde_json::to_value(lctx_surrealdb::control::cut_era(&client).await?).map_err(ModelError::codec),
                                HistoryCommand::Compact { limit } => serde_json::to_value(lctx_surrealdb::control::compact_history(&client, limit).await?).map_err(ModelError::codec),
                                HistoryCommand::Resume { identity, expected_revision, limit } => serde_json::to_value(lctx_surrealdb::control::resume_history_compaction(&client, identity, expected_revision, limit).await?).map_err(ModelError::codec),
                            }
                        }.await;
                        let mut completion = Completion::default();
                        completion.step("history maintenance session invalidation",client.invalidate().await.map_err(ModelError::codec));
                        complete(result, completion)
                    })?;
                    println!("{}", serde_json::to_string(&serde_json::json!({"ready":false,"receipt":receipt}))?);
                    return Ok(());
                }
                StoreCommand::Recover { cleanup, retirement, limit } => {
                    let receipts = runtime.block_on(async {
                        use lctx_model::domain::{ModelError, completion::{Completion, complete}};
                        anyhow::ensure!(!cleanup.is_empty() || !retirement.is_empty(), "recover requires named obligations");
                        let client = lctx_surrealdb::upgrade::maintenance_client(&config).await?;
                        let result = async {
                            let mut receipts=Vec::new();
                            for predecessor in cleanup {
                                let successor=lctx_surrealdb::control::recover_cleanup(&client, predecessor).await?;
                                // Emit the durable identity before continuing: a later error cannot hide it.
                                println!("{}",serde_json::json!({"cleanup_predecessor":predecessor.hex(),"successor":successor.hex()}));
                                lctx_surrealdb::control::resume_cleanup(&client,successor).await?;
                                receipts.push(serde_json::json!({"cleanup":successor.hex(),"completed":true}));
                            }
                            for predecessor in retirement {
                                let successor=lctx_surrealdb::control::recover_retirement(&client,predecessor).await?;
                                println!("{}",serde_json::json!({"retirement_predecessor":predecessor.hex(),"successor":successor.hex()}));
                                receipts.push(serde_json::to_value(lctx_surrealdb::control::resume_retirement(&client,successor,limit).await?).map_err(ModelError::codec)?);
                            }
                            Ok::<_, ModelError>(receipts)
                        }.await;
                        let mut completion = Completion::default();
                        completion.step("recovery maintenance session invalidation",client.invalidate().await.map_err(ModelError::codec));
                        Ok::<_, anyhow::Error>(complete(result,completion)?)
                    })?;
                    println!("{}",serde_json::json!({"ready":false,"receipts":receipts}));
                    return Ok(());
                }
                StoreCommand::Drain => {
                    runtime.block_on(lctx_surrealdb::compiler::drain_installation(&config))?;
                }
                StoreCommand::Reconcile {
                    pin,
                    backup_hold,
                    readers_stopped,
                } => {
                    runtime.block_on(lctx_surrealdb::compiler::reconcile_maintenance(
                        &config,
                        &pin,
                        &backup_hold,
                        readers_stopped,
                    ))?;
                }
                StoreCommand::CloseAdmission => {
                    runtime.block_on(lctx_surrealdb::compiler::close_admission(&config))?;
                }
                StoreCommand::OpenAdmission => {
                    runtime.block_on(lctx_surrealdb::compiler::open_admission(&config))?;
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ready":reports_ready, "namespace":config.namespace, "database":config.database,
                }))?
            );
            Ok(())
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
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Result<Cli, String> {
        Cli::try_parse_from(std::iter::once("lctx").chain(args.iter().copied()))
            .map_err(|e| e.to_string())
    }

    #[test]
    fn maintenance_installation_can_retain_closed_admission() {
        for (args, expected) in [
            (vec!["store", "init"], false),
            (vec!["store", "init", "--keep-closed"], true),
        ] {
            let cli = parse(&args).expect("maintenance installation command");
            let Cmd::Store {
                command: super::StoreCommand::Init { keep_closed },
                ..
            } = cli.command
            else {
                panic!("maintenance installation command");
            };
            assert_eq!(keep_closed, expected);
        }
    }

    #[test]
    fn maintenance_upgrade_and_recovery_require_exact_identities() {
        let identity="ab".repeat(32);
        assert!(parse(&["store","schema"]).is_ok());
        assert!(parse(&["store","upgrade"]).is_err());
        assert!(parse(&["store","upgrade","--expected-schema",&identity,"--operation",&identity]).is_err());
        assert!(parse(&["store","upgrade","--expected-schema",&identity,"--operation",&identity,"--execution-contract","/owned/execution.json"]).is_ok());
        assert!(parse(&["store","upgrade","--expected-schema","wrong","--operation",&identity]).is_err());
        assert!(parse(&["store","upgrade-contract","--expected-schema",&identity,"--operation",&identity,"--execution",&identity]).is_ok());
        assert!(parse(&["store","upgrade-contract","--expected-schema",&identity,"--operation",&identity]).is_err());
        assert!(parse(&["store","upgrade","--expected-schema",&identity,"--operation",&identity,"--stop-after-pages","1"]).is_err());
        assert!(parse(&["store","upgrade","--expected-schema",&identity,"--operation",&identity,"--execution-contract","/owned/execution.json","--stop-after-pages","1"]).is_ok());
        assert!(parse(&["store","upgrade","--expected-schema",&identity,"--operation",&identity,"--execution-contract","/owned/execution.json","--stop-after-pages","0"]).is_err());
        assert!(parse(&["store","upgrade-qualify","--expected-schema",&identity,"--operation",&identity,"--execution-contract","/owned/execution.json"]).is_ok());
        assert!(parse(&["store","upgrade-qualify","--expected-schema",&identity,"--operation",&identity]).is_err());
        assert!(parse(&["store","history","qualify","--evidence",&identity]).is_ok());
        assert!(parse(&["store","history","resume","--identity",&identity]).is_err());
        assert!(parse(&["store","history","resume","--identity",&identity,"--expected-revision","0"]).is_ok());
        assert!(parse(&["store","history","resume","--identity",&identity,"--expected-revision","-1"]).is_err());
        assert!(parse(&["store","history","resume"]).is_err());
        assert!(parse(&["store","recover","--cleanup",&identity,"--retirement",&identity]).is_ok());
    }

    #[test]
    fn maintenance_reconciliation_requires_explicit_stopped_readers_and_exact_ids() {
        let identity = "ab".repeat(32);
        assert!(parse(&["store", "reconcile", "--pin", &identity]).is_err());
        assert!(
            parse(&[
                "store",
                "reconcile",
                "--pin",
                "not-a-digest",
                "--readers-stopped"
            ])
            .is_err()
        );
        let command = parse(&[
            "store",
            "reconcile",
            "--pin",
            &identity,
            "--readers-stopped",
        ])
        .unwrap();
        let Cmd::Store {
            command:
                super::StoreCommand::Reconcile {
                    pin,
                    backup_hold,
                    readers_stopped,
                },
            ..
        } = command.command
        else {
            panic!("maintenance command")
        };
        assert!(readers_stopped);
        assert_eq!(pin.len(), 1);
        assert!(backup_hold.is_empty());
    }

    #[tokio::test]
    async fn flow_workspace_shares_the_capture_budget() {
        use lctx_model::domain::resources::ResourceBudget;
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let path = PathBuf::from(
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned compiler fixture"),
        );
        let config = lctx_surrealdb::RuntimeConfig::read(&path).unwrap();
        let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
        )
        .await
        .unwrap();
        let workspace = cpg_core::workspace::Workspace::with_budget(
            std::sync::Arc::new(lctx_model::domain::model().unwrap()),
            super::flow_workspace_options(&budget),
            budget.clone(),
            native.clone(),
        )
        .unwrap();
        let before = budget.reserved();
        let held = workspace
            .budget()
            .reserve("flow-probe-retained-input", 1024)
            .unwrap();
        assert_eq!(budget.reserved(), before + 1024);
        drop(held);
        assert_eq!(budget.reserved(), before);
        workspace.drain().await.unwrap();
        native.abandon().await.unwrap();
    }

    #[test]
    fn native_commands_keep_artifact_and_selection_boundaries_explicit() {
        assert!(
            parse(&[
                "compile",
                "fastmcp",
                "--through",
                "facts",
                "--runtime-config",
                "native.json"
            ])
            .is_ok()
        );
        let Cmd::Compile {
            artifact_only,
            output,
            runtime_config,
            ..
        } = parse(&[
            "compile",
            "fastmcp",
            "--through",
            "facts",
            "--artifact-only",
            "--output",
            "graph",
            "--runtime-config",
            "native.json",
        ])
        .unwrap()
        .command
        else {
            panic!("expected the explicit native artifact compile route");
        };
        assert!(artifact_only);
        assert_eq!(output, Some(PathBuf::from("graph")));
        assert_eq!(runtime_config, Some(PathBuf::from("native.json")));
        assert!(matches!(
            parse(&["publish-artifact", "graph"]).unwrap().command,
            Cmd::PublishArtifact { .. }
        ));
        assert!(matches!(
            parse(&["snapshot", "select", "handle.json"])
                .unwrap()
                .command,
            Cmd::Snapshot {
                command: super::SnapshotCommand::Select { .. },
                ..
            }
        ));
        assert!(
            parse(&[
                "snapshot",
                "export",
                "--projection",
                "CallableInvocation",
                "--output",
                "graph.json"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "snapshot",
                "export",
                "--projection",
                "CallableInvocation",
                "--input",
                "00000000000000000000000000000000",
                "--context",
                "11111111111111111111111111111111",
                "--output",
                "graph.json"
            ])
            .is_ok()
        );
        assert!(
            parse(&[
                "snapshot",
                "export",
                "--projection",
                "unknown",
                "--input",
                "00000000000000000000000000000000",
                "--context",
                "11111111111111111111111111111111",
                "--output",
                "graph.json"
            ])
            .is_err()
        );
        assert!(parse(&["snapshot", "select"]).is_err());
        assert!(parse(&["snapshot", "backup", "--output", "dump.sql"]).is_ok());
        assert!(parse(&["snapshot", "list"]).is_ok());
        assert!(parse(&["snapshot", "audit"]).is_err());
        assert!(parse(&["snapshot", "audit", "handle.json"]).is_ok());
        assert!(parse(&["snapshot", "restore", "dump.sql"]).is_ok());
        assert!(parse(&["snapshot", "retire", "handle.json"]).is_err());
        assert!(parse(&["snapshot", "retire", "handle.json", "--readers-stopped"]).is_ok());
        assert!(
            parse(&[
                "snapshot",
                "query",
                "SELECT * FROM entity",
                "--handle",
                "handle.json"
            ])
            .is_ok()
        );
        assert!(parse(&["tool", "search_operations", "--request", "request.json"]).is_ok());
        assert!(parse(&["tool", "search"]).is_err());
        assert!(parse(&["store", "check", "--runtime-config", "native.json"]).is_ok());
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
            "runs",
            "compile-fixture",
        ] {
            assert!(parse(&[retired]).is_err(), "{retired}");
        }
        // Ordinary compilation accepts native runtime configuration independently of artifact output.
        assert!(matches!(
            parse(&["compile", "fastmcp", "--through", "facts"])
                .unwrap()
                .command,
            Cmd::Compile { .. }
        ));
    }
}
