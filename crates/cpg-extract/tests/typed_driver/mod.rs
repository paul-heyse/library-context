#![allow(dead_code)]
//! Shared driver for the typed producer suites: a fixture tree is acquired and run through
//! `acquire`, `pyrefly` and `assemble` into a stage-bound memory generation the model validates;
//! an inspecting stage reads back what the stages handed off.
use std::{collections::BTreeMap, path::Path, sync::Arc};
use cpg_extract::{acquisition::{AcquiredInput, Acquire}, assembly::Assemble, bundle::{CapturedInputs, Declared, ProviderStage, run_stage},
    capture::CapturedInput, pyrefly_stage::Pyrefly, typed_syntax::SyntaxLimits};
use lctx_model::domain::{*, batching::TransferLimits, memory::MemoryGeneration, resources::ResourceBudget, source::SourceArtifact, stages::*};

pub fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 30).unwrap() }

/// Every file of `fixtures/python/<fixture>`, by relative path.
pub fn files(fixture: &str) -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python").join(fixture);
    let mut files = BTreeMap::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { pending.push(path); continue; }
            files.insert(path.strip_prefix(&root).unwrap().display().to_string(), std::fs::read(&path).unwrap());
        }
    }
    files
}

/// The rows an inspection read, by relation.
pub type Tables = std::sync::Arc<std::sync::Mutex<BTreeMap<&'static str, arrow_array::RecordBatch>>>;
pub fn rows<R: Record>(tables: &Tables) -> Vec<R> { tables.lock().unwrap().get(R::NAME).map(|b| R::decode(b).unwrap()).unwrap_or_default() }

/// Collects the handoffs of the relations named by `$ty` into `$rows`.
#[macro_export]
macro_rules! inspector {
    ($name:ident, $($ty:ty),+ $(,)?) => {
        pub struct $name(pub typed_driver::Tables);
        impl cpg_extract::bundle::Declared for $name {
            fn declaration(&self, _: lctx_model::domain::stages::Profile) -> lctx_model::domain::stages::Stage {
                use lctx_model::domain::{ContentHash, stages::*};
                Stage { name: "inspect", inputs: vec![$(RelationUse::of::<$ty>()),+], outputs: vec![RelationUse::of::<lctx_model::domain::types::TypePresentationSupport>()],
                    contributes: vec![], coverage: vec![], provider: None, profiles: vec![Profile::Catalog, Profile::Behavioral], effect: Effect::Pure,
                    code: ContentHash::of(b"inspect"), configuration: ContentHash::of(b"inspect") }
            }
        }
        impl<S: lctx_model::domain::stages::StageSink + 'static> cpg_extract::bundle::ProviderStage<S> for $name {
            fn run(&mut self, context: &mut cpg_extract::bundle::StageContext<S>) -> Result<lctx_model::domain::stages::ProviderOutcome, lctx_model::domain::ModelError> {
                use lctx_model::domain::Record;
                let mut tables = self.0.lock().unwrap();
                $(
                    let batches: Vec<arrow_array::RecordBatch> = context.handoff::<$ty>()?.iter().map(|b| b.arrow().clone()).collect();
                    let schema = <$ty>::schema();
                    tables.insert(<$ty>::NAME, arrow_select::concat::concat_batches(&schema, &batches).unwrap());
                )+
                context.declare::<lctx_model::domain::types::TypePresentationSupport>()?;
                Ok(lctx_model::domain::stages::ProviderOutcome::Complete)
            }
        }
    };
}

/// Capture `files` as a tree input in a fresh temporary directory.
pub fn capture(files: &BTreeMap<String, Vec<u8>>, label: &str) -> Arc<CapturedInputs> {
    let original = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let target = original.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let captured = CapturedInput::capture(original.path(), &files.keys().cloned().collect::<Vec<_>>(), &budget()).unwrap();
    Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(captured, label)]))
}

/// Run `acquire → pyrefly → assemble → inspect` over `files` into memory; validate the generation
/// and return its content digest.
pub async fn run<I>(files: &BTreeMap<String, Vec<u8>>, inspect: I) -> Result<ContentHash, ModelError>
where I: Declared + ProviderStage<MemoryGeneration> + 'static {
    run_with(capture(files, "typed-driver"), Pyrefly::new(SyntaxLimits::default()), inspect).await
}
/// Run the stages over an already captured input with a given `pyrefly` stage.
pub async fn run_with<I>(captured: Arc<CapturedInputs>, pyrefly: Pyrefly, inspect: I) -> Result<ContentHash, ModelError>
where I: Declared + ProviderStage<MemoryGeneration> + 'static {
    let model = Arc::new(model()?);
    let providers: Vec<Box<dyn ProviderStage<MemoryGeneration>>> = vec![Box::new(Acquire::new(ContentHash::of(b"typed-driver"))),
        Box::new(pyrefly), Box::new(Assemble), Box::new(inspect)];
    let stages = providers.iter().map(|p| p.declaration(Profile::Catalog)).collect();
    let schedule = Schedule::build(&model, stages, &[], Profile::Catalog)?;
    let mut execution = schedule.execute();
    let generation = MemoryGeneration::bind(&model, &budget(), &mut execution)?;
    let mut providers: BTreeMap<&str, Box<dyn ProviderStage<MemoryGeneration>>> = providers.into_iter().map(|p| (p.declaration(Profile::Catalog).name, p)).collect();
    let names: Vec<&'static str> = execution.schedule().stages().iter().map(|s| s.name).collect();
    for name in names {
        run_stage(providers.remove(name).unwrap(), execution.begin(name)?, &generation, &model, &captured, &budget(), TransferLimits::default()).await?;
    }
    execution.finish()?;
    generation.validate(&model, &budget())
}

/// The artifact at `path` among `artifacts`.
pub fn artifact<'a>(artifacts: &'a [SourceArtifact], path: &str) -> &'a SourceArtifact { artifacts.iter().find(|a| a.path == path).unwrap() }
