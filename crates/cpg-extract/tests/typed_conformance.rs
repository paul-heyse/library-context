//! Disposable P0-E conformance harness. This cannot create/select a facts frontier.
//! The permanent emitter consumes Pyrefly's retained AST, never legacy rows or IDs.
use std::{collections::BTreeMap, path::Path, sync::Arc};
use cpg_extract::typed_syntax::{self, SyntaxEvent, SyntaxInvocation, SyntaxLimits};
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, conditions::*, input::*, source::*};
use lctx_postgres::generations::{Error, GenerationStore};
use pyrefly::{state::{require::Require, state::State}};
use pyrefly_config::{config::{ConfigFile, ConfigSource}, error_kind::ErrorKind, finder::ConfigFinder};
use pyrefly_python::{module_path::ModulePath, sys_info::{PythonPlatform, PythonVersion}};
use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ImageExt, runners::AsyncRunner}};

struct Captured {
    frozen: cpg_extract::capture::CapturedInput,
    input: InputRevision,
    files: BTreeMap<String,Vec<u8>>,
}
impl Captured {
    fn fixture() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/typed_semantics");
        let mut files = BTreeMap::new();
        for path in ["sample.py", "_invalid/broken.py"] {
            files.insert(path.to_owned(), std::fs::read(root.join(path)).unwrap());
        }
        // Evidence bytes are retained even when the analyzer cannot decode them.
        files.insert("_invalid/undecodable.py".into(), vec![0xff, 0x00, 0x80]);
        Self::new(files)
    }
    fn new(files: BTreeMap<String,Vec<u8>>) -> Self {
        assert!(files.values().map(Vec::len).sum::<usize>() < 1024 * 1024, "bounded disposable fixture harness");
        let input = InputRevision::from_entries(files.iter().map(|(path, bytes)| ManifestEntry {
            path: path.clone(), content: ContentHash::of(bytes), byte_len: bytes.len() as i64,
        }).collect()).unwrap();
        let original = tempfile::tempdir().unwrap();
        for (path, bytes) in &files {
            let target = original.path().join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target,bytes).unwrap();
        }
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let frozen = cpg_extract::capture::CapturedInput::capture(original.path(),&files.keys().cloned().collect::<Vec<_>>(),&budget).unwrap();
        assert_eq!(frozen.revision(),&input);
        Self { frozen,input,files }
    }
}
struct Extracted {
    batches: BTreeMap<&'static str, arrow_array::RecordBatch>,
    events: Vec<SyntaxEvent>,
    coverage: Vec<ProviderCoverage>,
}
fn add<R: Record>(batches: &mut BTreeMap<&'static str,arrow_array::RecordBatch>, model: &ValidatedModel, rows: Vec<R>) {
    let mut combined = batches.get(R::NAME).map(|batch| R::decode(batch).unwrap()).unwrap_or_default();
    combined.extend(rows);
    batches.insert(R::NAME,Batch::new(model,combined, &budget()).unwrap().arrow().clone());
}
fn relativize(value: &mut serde_json::Value, root: &Path) {
    match value {
        serde_json::Value::String(s) => {
            if let Ok(relative) = Path::new(s).strip_prefix(root) { *s = format!("$input/{}",relative.display()); }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(|item| relativize(item,root)),
        serde_json::Value::Object(items) => items.values_mut().for_each(|item| relativize(item,root)),
        _ => {},
    }
}
fn extract(captured: Captured) -> Extracted {
    std::thread::Builder::new().stack_size(512 << 20).spawn(move || extract_inner(captured)).unwrap().join().unwrap()
}
fn extract_inner(captured: Captured) -> Extracted {
    for (name,_) in std::env::vars_os() {
        let name = name.to_string_lossy();
        assert!(!matches!(name.as_ref(),"PYREFLY_STACK_SIZE"|"PYREFLY_FIXPOINT_DETAILS") && !name.starts_with("PYSA_DUMP"));
    }
    let root = captured.frozen.root();
    let mut cfg = ConfigFile { source: ConfigSource::File(root.join("pyrefly.toml")),
        search_path_from_args: vec![root.to_path_buf()], disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true, enable_fallback_search_path: false, ..ConfigFile::default() };
    cfg.python_environment.python_version = Some(PythonVersion::new(3,14,7));
    cfg.python_environment.python_platform = Some(PythonPlatform::new("linux"));
    cfg.python_environment.site_package_path = Some(vec![]);
    cfg.interpreters.skip_interpreter_query = true;
    assert!(cfg.configure().is_empty());
    let mut config = serde_json::to_value(&cfg).unwrap(); relativize(&mut config,root);
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(),
        search_path: vec!["$input".into()], site_package_path: vec![],
        config_digest: ContentHash::of(&serde_json::to_vec(&config).unwrap()),
        environment_digest: captured.input.manifest, lock_digest: None };
    let handles: Vec<_> = captured.files.iter().filter(|(_,bytes)| std::str::from_utf8(bytes).is_ok())
        .map(|(path,_)| (path.clone(),cfg.handle_from_module_path(ModulePath::filesystem(root.join(path)))))
        .collect();
    let finder = ConfigFinder::new_constant(ArcId::new(cfg));
    let state = State::new(finder,ThreadCount::Inline);
    let mut txn = state.new_transaction(Require::Exports,None);
    txn.run(&handles.iter().map(|(_,handle)| handle.clone()).collect::<Vec<_>>(),Require::Everything,None);
    let model = model().unwrap();
    let provider = Provider { tool: "pyrefly-retained-ruff-ast".into(),
        revision: "a07b7baead9e0c7b496346d879b88e2fff9cbda7;ruff=0.0.11".into(),
        build_digest: ContentHash::of(concat!(include_str!("../../../Cargo.lock"),include_str!("../src/typed_syntax.rs")).as_bytes()) };
    let (run,families) = ProviderRun::new(provider.id(),context.id(),captured.input.id(),context.config_digest,[FactFamily::Syntax]).unwrap();
    let surface = ProviderSurface { provider: provider.id(),family: FactFamily::Syntax,name: "retained AST identifier observations".into() };
    let origin = InputOrigin::Tree { label: "typed-conformance".into() };
    let acquisition = InputAcquisition { input: captured.input.id(),origin: origin.id() };
    let (condition,nodes) = Diagram::always().records();
    let mut batches = BTreeMap::new();
    macro_rules! add_one { ($($row:expr),+ $(,)?) => { $(add(&mut batches,&model,vec![$row.clone()]);)+ }; }
    add_one!(captured.input,context,provider,run,surface,origin,acquisition,condition);
    add(&mut batches,&model,families); add(&mut batches,&model,nodes);
    let mut events = vec![]; let mut coverage = vec![];
    for (path,bytes) in &captured.files {
        let source = SourceArtifact::from_bytes(captured.input.id(),path.clone(),bytes).unwrap();
        add(&mut batches,&model,ArtifactChunk::split(&source,bytes).unwrap().collect());
        add_one!(source);
        let Some((_,handle)) = handles.iter().find(|(p,_)| p == path) else {
            let scope = CoverageScope::Artifact { artifact: source.id() };
            coverage.push(ProviderCoverage { scope: scope.id(),provider: provider.id(),context: context.id(),
                family: FactFamily::Syntax,run: Some(run.id()),status: CoverageStatus::Unavailable,
                reason: Some(ObligationKind::UndecodableSource),diagnostic: None });
            add_one!(scope); continue;
        };
        let module = Module { source: source.id(),qualified_name: handle.module().to_string() };
        let scope = CoverageScope::Module { module: module.id() };
        let qualification = AssertionQualification { context: context.id(),scope: scope.id(),condition: condition.id(),
            modality: Modality::Definite,approximation: Approximation::Exact };
        let ast = txn.get_ast(handle).expect("retained native AST");
        let info = txn.get_module_info(handle).unwrap();
        let text = info.lined_buffer().contents().clone();
        assert_eq!(text.as_bytes(),bytes);
        let invocation = || SyntaxInvocation { source: &source,qualification: &qualification,run: &run,surface: &surface };
        if path == "sample.py" {
            let mut changed = source.clone(); changed.content = ContentHash::of(b"changed");
            let error = typed_syntax::emit(&ast,&text,SyntaxInvocation { source: &changed,..invocation() },SyntaxLimits::default(),|_| panic!("changed content emitted")).unwrap_err();
            assert!(error.to_string().contains("differs from captured"));
            let mut count = 0;
            assert!(typed_syntax::emit(&ast,&text,invocation(),SyntaxLimits { nodes: 2,depth: 256 },|_| { count += 1; Ok(()) }).is_err());
            assert_eq!(count,2);
            let mut count = 0;
            assert!(typed_syntax::emit(&ast,&text,invocation(),SyntaxLimits::default(),|_| { count += 1; Err(ModelError::Invalid("sink refused".into())) }).is_err());
            assert_eq!(count,1);
        }
        typed_syntax::emit(&ast,&text,invocation(),SyntaxLimits::default(),|event| { events.push(event); Ok(()) }).unwrap();
        let errors = txn.get_errors([handle]).collect_errors();
        let parse_error = [&errors.ordinary,&errors.directives,&errors.suppressed,&errors.disabled,&errors.baseline]
            .into_iter().flatten().any(|error| error.error_kind() == ErrorKind::ParseError);
        coverage.push(ProviderCoverage { scope: scope.id(),provider: provider.id(),context: context.id(),
            family: FactFamily::Syntax,run: Some(run.id()),status: CoverageStatus::Partial,
            reason: Some(if parse_error { ObligationKind::SyntaxError } else { ObligationKind::OutsideProviderModel }),
            diagnostic: Some("Identifier observation subset; complete syntax family not implemented".into()) });
        add_one!(module,scope,qualification);
    }
    for event in &events {
        add_one!(event.occurrence);
        if let Some((assertion,evidence,support)) = &event.observation { add_one!(assertion,evidence,support); }
    }
    add(&mut batches,&model,coverage.clone());
    captured.frozen.verify().unwrap();
    Extracted { batches,events,coverage }
}

#[tokio::test]
async fn pinned_native_parse_typed_domain_and_postgres_conformance() {
    let first = Captured::fixture(); let relocated = Captured::new(first.files.clone());
    assert_ne!(first.frozen.root(),relocated.frozen.root());
    let left = extract(first); let right = extract(relocated);
    assert_eq!(left.events,right.events,"structural identity is independent of checkout path");
    assert_eq!(left.batches,right.batches,"configuration and provenance also relocate deterministically");
    assert!(left.coverage.iter().all(|row| row.status != CoverageStatus::CompleteUnderStatedModel));
    for reason in [ObligationKind::OutsideProviderModel,ObligationKind::SyntaxError,ObligationKind::UndecodableSource] {
        assert!(left.coverage.iter().any(|row| row.reason == Some(reason)));
    }
    assert!(left.events.iter().any(|event| event.observation.as_ref().is_some_and(|(row,_,_)| row.spelling == "α")));
    let with_items: Vec<_> = left.events.iter().filter(|event| event.occurrence.role == OccurrenceRole::WithItem).collect();
    assert_eq!(with_items.len(),2); assert_ne!(with_items[0].occurrence.id(),with_items[1].occurrence.id());
    assert_ne!(with_items[0].occurrence.structural_path,with_items[1].occurrence.structural_path);
    let (image,tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PostgreSQL18 required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap()); let store = GenerationStore::install(owner,model.clone()).await.unwrap();
    let generation = store.create_conformance(ContentHash::of(b"typed-conformance-harness"),"catalog").await.unwrap();
    macro_rules! write_read { ($($ty:ty),+ $(,)?) => { $(
        if let Some(batch) = left.batches.get(<$ty>::NAME) {
            let rows = <$ty>::decode(batch).unwrap();
            store.copy(&writer,generation,&Batch::new(&model,rows, &budget()).unwrap(), &budget()).await.unwrap();
        }
    )+ }; }
    write_read!(InputRevision,InputOrigin,InputAcquisition,SourceArtifact,ArtifactChunk,Module,Occurrence,
        Provider,AnalysisContext,ProviderRun,RunFamily,ProviderSurface,Condition,ConditionNode,
        CoverageScope,AssertionQualification,Evidence,SyntaxObservation,SyntaxSupport,ProviderCoverage);
    store.seal(generation).await.unwrap(); store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
    assert!(matches!(store.select(generation).await,Err(Error::Frontier)));
    let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
    assert_eq!(lease.read::<Occurrence>().await.unwrap().arrow(),&left.batches[Occurrence::NAME]);
    assert_eq!(lease.read::<SyntaxObservation>().await.unwrap().arrow(),&left.batches[SyntaxObservation::NAME]);
    assert_eq!(lease.read::<SyntaxSupport>().await.unwrap().arrow(),&left.batches[SyntaxSupport::NAME]);
    assert_eq!(lease.read::<ArtifactChunk>().await.unwrap().arrow(),&left.batches[ArtifactChunk::NAME]);
    assert_eq!(lease.read::<ProviderCoverage>().await.unwrap().arrow(),&left.batches[ProviderCoverage::NAME]);
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
