#![allow(
    dead_code,
    reason = "Shared contract fixtures expose helpers to multiple targeted suites"
)]
//! Shared driver for the typed producer suites: a fixture tree is acquired and run through
//! the native production providers into exact immutable native compiler views. Inspection reads
//! completed outputs after producers finish; it reads the owned disposable compiler store.
use cpg_core::workspace::{ProducerOutput, Workspace, WorkspaceOptions};
use cpg_extract::{
    acquisition::{Acquire, AcquiredInput},
    assembly::Assemble,
    bundle::{CapturedInputs, ProviderStage},
    capture::CapturedInput,
    pyrefly_stage::Pyrefly,
    typed_syntax::SyntaxLimits,
};
use lctx_model::domain::{
    batching::TransferLimits, resources::ResourceBudget, source::SourceArtifact, stages::*, *,
};
use std::{collections::BTreeMap, path::Path, sync::Arc};

pub fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}

/// Every file of `fixtures/python/<fixture>`, by relative path.
pub fn files(fixture: &str) -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python")
        .join(fixture);
    let mut files = BTreeMap::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            files.insert(
                path.strip_prefix(&root).unwrap().display().to_string(),
                std::fs::read(&path).unwrap(),
            );
        }
    }
    files
}

/// The rows an inspection read, by relation.
pub type Tables =
    std::sync::Arc<std::sync::Mutex<BTreeMap<&'static str, arrow_array::RecordBatch>>>;
pub fn rows<R: Record>(tables: &Tables) -> Vec<R> {
    tables
        .lock()
        .unwrap()
        .get(R::NAME)
        .map(|b| R::decode(b).unwrap())
        .unwrap_or_default()
}

pub trait Inspector {
    fn tables(&self) -> Tables;
}
/// Copy small fixture outputs for independent assertions, after the actual compiler completed them.
pub fn observe(workspace: &Workspace, tables: &Tables) -> Result<(), ModelError> {
    let mut tables = tables
        .lock()
        .map_err(|_| ModelError::Invalid("test observer poisoned".into()))?;
    for relation in workspace.completed_relations()? {
        let batches = relation
            .batches()?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ModelError::codec)?;
        if let Some(first) = batches.first() {
            tables.insert(
                relation.name(),
                arrow_select::concat::concat_batches(&first.schema(), &batches)
                    .map_err(ModelError::codec)?,
            );
        }
    }
    Ok(())
}

/// Names the independent expectations' observed tables without declaring a production writer.
#[macro_export]
macro_rules! inspector {
    ($name:ident, $($ty:ty),+ $(,)?) => {
        pub struct $name(pub typed_driver::Tables);
        impl typed_driver::Inspector for $name {
            fn tables(&self) -> typed_driver::Tables {
                self.0.clone()
            }
        }
    };
}

/// Capture `files` as a tree input in a fresh temporary directory.
pub fn capture(
    files: &BTreeMap<String, Vec<u8>>,
    label: &str,
    profile: Profile,
) -> Arc<CapturedInputs> {
    capture_with_ruff(files, label, profile, Default::default())
}

pub fn capture_with_ruff(
    files: &BTreeMap<String, Vec<u8>>,
    label: &str,
    profile: Profile,
    settings: cpg_extract::ruff_context::ContextSettings,
) -> Arc<CapturedInputs> {
    let original = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let target = original.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let documents = files
        .keys()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    let captured = CapturedInput::capture_derived(
        original.path(),
        &files.keys().cloned().collect::<Vec<_>>(),
        &budget(),
        &documents,
        cpg_extract::acquisition::derive_blocks,
    )
    .unwrap();
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(captured, label)],
        cpg_extract::native_context::NativeContextConfig::committed(profile, &budget())
            .unwrap()
            .with_ruff(settings)
            .unwrap(),
    ))
}

/// Run the native facts compiler over `files` and return its semantic content digest.
pub async fn run<I>(
    files: &BTreeMap<String, Vec<u8>>,
    inspect: I,
) -> Result<ContentHash, ModelError>
where
    I: Inspector,
{
    run_with(
        capture(files, "typed-driver", Profile::Catalog),
        Pyrefly::new(SyntaxLimits::default()),
        inspect,
    )
    .await
}
/// Run the stages over an already captured input with a given `pyrefly` stage.
pub async fn run_with<I>(
    captured: Arc<CapturedInputs>,
    pyrefly: Pyrefly,
    inspect: I,
) -> Result<ContentHash, ModelError>
where
    I: Inspector,
{
    run_profile(captured, pyrefly, inspect, Profile::Catalog).await
}
pub async fn run_behavioral<I: Inspector>(
    files: &BTreeMap<String, Vec<u8>>,
    inspect: I,
) -> Result<ContentHash, ModelError> {
    run_profile(
        capture(files, "typed-driver", Profile::Behavioral),
        Pyrefly::new(SyntaxLimits::default()),
        inspect,
        Profile::Behavioral,
    )
    .await
}
pub async fn run_profile<I: Inspector>(
    captured: Arc<CapturedInputs>,
    pyrefly: Pyrefly,
    inspect: I,
    profile: Profile,
) -> Result<ContentHash, ModelError> {
    run_profile_with_limits(
        captured,
        pyrefly,
        inspect,
        profile,
        TransferLimits::default(),
        false,
    )
    .await
}
pub async fn run_profile_with_limits<I: Inspector>(
    captured: Arc<CapturedInputs>,
    pyrefly: Pyrefly,
    inspect: I,
    profile: Profile,
    limits: TransferLimits,
    reverse_providers: bool,
) -> Result<ContentHash, ModelError> {
    let resources = captured.config().budget().clone();
    run_profile_with_budget(
        captured,
        pyrefly,
        inspect,
        profile,
        limits,
        reverse_providers,
        resources,
        native_fixture::create_native().await?,
    )
    .await
}
#[allow(
    clippy::too_many_arguments,
    reason = "Focused controls choose every stage driver dimension explicitly"
)]
pub async fn run_profile_with_budget<I: Inspector>(
    captured: Arc<CapturedInputs>,
    pyrefly: Pyrefly,
    inspect: I,
    profile: Profile,
    limits: TransferLimits,
    reverse_providers: bool,
    resources: ResourceBudget,
) -> Result<ContentHash, ModelError> {
    let model = Arc::new(model()?);
    let mut providers: Vec<Box<dyn ProviderStage<ProducerOutput>>> = vec![
        Box::new(Acquire::new(ContentHash::of(b"typed-driver"))),
        Box::new(pyrefly),
        Box::new(cpg_extract::document_parser::Documents),
        Box::new(cpg_extract::deployment::Deployment),
        Box::new(Assemble),
    ];
    if profile == Profile::Behavioral {
        providers.push(Box::new(cpg_extract::ty_flow::TyFlow::default()));
    }
    if reverse_providers {
        providers.reverse();
    }
    let workspace = Workspace::with_budget(
        model,
        WorkspaceOptions {
            memory_bytes: resources.limit(),
            ..WorkspaceOptions::default()
        },
        resources,
        native_fixture::create_native().await?,
    )?;
    cpg_core::facts::compile_facts(&workspace, &captured, profile, providers, limits).await?;
    workspace.validate().await?;
    observe(&workspace, &inspect.tables())?;
    workspace.identity()
}

/// The artifact at `path` among `artifacts`.
pub fn artifact<'a>(artifacts: &'a [SourceArtifact], path: &str) -> &'a SourceArtifact {
    artifacts.iter().find(|a| a.path == path).unwrap()
}

#[path="../fixtures/native.rs"]
mod native_fixture;
