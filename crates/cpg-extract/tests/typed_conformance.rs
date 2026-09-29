//! The stage-bound production subset (plan E1) over the pinned native parse: the permanent
//! capture and syntax stages write through `StageOutput` into a stage-bound memory generation and
//! into a real PostgreSQL 18 conformance generation. The subset cannot be admitted as facts.
use std::{collections::BTreeMap, future::Future, path::Path, sync::Arc, task::{Context, Poll, Waker}};
use cpg_extract::{capture::CapturedInput, typed_stages::{self, CAPTURE, SYNTAX}, typed_syntax::{self, SyntaxFacts, SyntaxInvocation, SyntaxLimits}};
use lctx_model::domain::{*, admission::FrontierContract, assertion::*, attribution::*, batching::TransferLimits, conditions::Diagram, input::*,
    memory::MemoryGeneration, resources::ResourceBudget, source::*, stages::*};
use lctx_postgres::generations::{Error, GenerationStore};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ImageExt, runners::AsyncRunner}};

fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future).as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => value, Poll::Pending => panic!("a memory sink never waits"),
    }
}
fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 30).unwrap() }
fn fixture() -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/typed_semantics");
    let mut files = BTreeMap::new();
    for path in ["sample.py", "_invalid/broken.py"] { files.insert(path.to_owned(), std::fs::read(root.join(path)).unwrap()); }
    // Evidence bytes are retained even when the analyzer cannot decode them.
    files.insert("_invalid/undecodable.py".into(), vec![0xff, 0x00, 0x80]);
    files
}
/// Capture a fresh copy of the files; every capture lives in its own temporary tree.
fn capture(files: &BTreeMap<String, Vec<u8>>) -> CapturedInput {
    let original = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let target = original.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    CapturedInput::capture(original.path(), &files.keys().cloned().collect::<Vec<_>>(), &budget()).unwrap()
}
fn origin() -> InputOrigin { InputOrigin::Tree { label: "typed-conformance".into() } }
fn schedule(model: &ValidatedModel, limits: SyntaxLimits) -> Schedule {
    Schedule::build(model, vec![typed_stages::capture_stage(), typed_stages::syntax_stage(limits)], &[], Profile::Catalog).unwrap()
}
/// Drive both stages into any stage-bound sink.
async fn drive<S: StageSink>(model: &ValidatedModel, execution: &mut Execution<'_>, sink: &S, captured: &CapturedInput, facts: SyntaxFacts,
    budget: &ResourceBudget) -> Result<(), ModelError> {
    let output = StageOutput::new(execution.begin(CAPTURE)?, sink, model, budget.clone(), TransferLimits::default())?;
    typed_stages::run_capture(output, captured, origin(), model, budget).await?;
    let output = StageOutput::new(execution.begin(SYNTAX)?, sink, model, budget.clone(), TransferLimits::default())?;
    typed_stages::run_syntax(output, captured, facts).await
}
struct Run { digest: ContentHash, coverage: Vec<ProviderCoverage>, occurrences: Vec<Occurrence>, observations: Vec<SyntaxObservation>, analyzed: Vec<String> }
/// The whole subset into a stage-bound memory generation.
fn run_memory(files: &BTreeMap<String, Vec<u8>>, limits: SyntaxLimits, budget: &ResourceBudget) -> Result<Run, ModelError> {
    let model = model()?; let captured = capture(files);
    let schedule = schedule(&model, limits);
    let mut execution = schedule.execute();
    let generation = MemoryGeneration::bind(&model, budget, &mut execution)?;
    let facts = typed_syntax::extract(&captured, &model, limits, budget)?;
    let (coverage, analyzed) = (facts.coverage.clone(), facts.analyzed.clone());
    let occurrences = facts.occurrences.iter().flat_map(|b| b.rows().to_vec()).collect();
    let observations = facts.observations.iter().flat_map(|b| b.rows().to_vec()).collect();
    ready(drive(&model, &mut execution, &generation, &captured, facts, budget))?;
    execution.finish()?;
    Ok(Run { digest: generation.validate(&model, budget)?, coverage, occurrences, observations, analyzed })
}
fn artifact(files: &BTreeMap<String, Vec<u8>>, path: &str) -> SourceArtifact {
    let input = InputRevision::from_entries(files.iter().map(|(p, b)| ManifestEntry { path: p.clone(), content: ContentHash::of(b), byte_len: b.len() as i64 }).collect()).unwrap();
    SourceArtifact::from_bytes(input.id(), path.into(), &files[path]).unwrap()
}
fn coverage_of<'a>(run: &'a Run, source: &SourceArtifact) -> &'a ProviderCoverage {
    run.coverage.iter().find(|row| row.scope == CoverageScope::Artifact { artifact: source.id() }.id()).unwrap()
}

#[test]
fn the_subset_schedule_is_not_a_facts_frontier() {
    let model = model().unwrap();
    let refused = FrontierContract::facts(&model, Profile::Catalog).unwrap().preflight(&schedule(&model, SyntaxLimits::default()));
    assert!(matches!(refused, Err(ModelError::Frontier(_))), "{refused:?}");
}

#[test]
fn stages_relocate_deterministically_and_disclose_coverage() {
    let files = fixture();
    let (left, right) = (run_memory(&files, SyntaxLimits::default(), &budget()).unwrap(), run_memory(&files, SyntaxLimits::default(), &budget()).unwrap());
    assert_eq!(left.digest, right.digest, "identity, configuration and provenance are independent of the capture path");
    assert!(left.coverage.iter().all(|row| row.status != CoverageStatus::CompleteUnderStatedModel), "an identifier subset is never complete");
    let sample = artifact(&files, "sample.py");
    assert_eq!(coverage_of(&left, &sample).reason, Some(ObligationKind::OutsideProviderModel));
    assert_eq!(coverage_of(&left, &artifact(&files, "_invalid/broken.py")).reason, Some(ObligationKind::SyntaxError));
    let undecodable = coverage_of(&left, &artifact(&files, "_invalid/undecodable.py"));
    assert_eq!((undecodable.status, undecodable.reason), (CoverageStatus::Unavailable, Some(ObligationKind::UndecodableSource)));
    assert!(!left.analyzed.contains(&"_invalid/undecodable.py".to_owned()), "undecodable bytes never reach the analyzer");
    assert!(left.observations.iter().any(|row| row.spelling == "α"));
    let with_items: Vec<_> = left.occurrences.iter().filter(|o| o.role == OccurrenceRole::WithItem).collect();
    assert_eq!(with_items.len(), 2); assert_ne!(with_items[0].id(), with_items[1].id());
    assert_ne!(with_items[0].structural_path, with_items[1].structural_path);
}

#[test]
fn admission_and_traversal_refusals_are_disclosed() {
    let files = fixture();
    let (sample, broken) = (artifact(&files, "sample.py"), artifact(&files, "_invalid/broken.py"));
    // A source over the size bound is refused before Pyrefly is given it; others are unaffected.
    let small = run_memory(&files, SyntaxLimits { source_bytes: files["sample.py"].len() - 1, ..SyntaxLimits::default() }, &budget()).unwrap();
    let refused = coverage_of(&small, &sample);
    assert_eq!((refused.status, refused.reason), (CoverageStatus::Unavailable, Some(ObligationKind::ResourceRefused)));
    assert!(!small.analyzed.contains(&"sample.py".to_owned()));
    assert!(small.occurrences.iter().all(|o| o.source != sample.id()));
    assert!(small.occurrences.iter().any(|o| o.source == broken.id()));
    // A traversal bound keeps the emitted, ancestor-closed prefix and states Partial.
    let bounded = run_memory(&files, SyntaxLimits { nodes: 2, ..SyntaxLimits::default() }, &budget()).unwrap();
    let partial = coverage_of(&bounded, &sample);
    assert_eq!((partial.status, partial.reason), (CoverageStatus::Partial, Some(ObligationKind::ResourceRefused)));
    let prefix: Vec<_> = bounded.occurrences.iter().filter(|o| o.source == sample.id()).collect();
    assert_eq!(prefix.len(), 2);
    assert_eq!(prefix.iter().map(|o| o.structural_path.clone()).collect::<Vec<_>>(), [vec![0], vec![0, 0]]);
}

#[test]
fn a_short_budget_fails_the_attempt_and_releases_everything() {
    let files = fixture();
    let short = ResourceBudget::fixed(16 << 10).unwrap();
    assert!(matches!(run_memory(&files, SyntaxLimits::default(), &short), Err(ModelError::Resource { .. })));
    assert_eq!(short.reserved(), 0);
}

#[test]
fn changed_text_is_refused_before_emission() {
    let files = fixture();
    let text = std::str::from_utf8(&files["sample.py"]).unwrap();
    let parsed = ruff_python_parser::parse_module(text).unwrap();
    let mut source = artifact(&files, "sample.py"); source.content = ContentHash::of(b"changed");
    let provider = typed_syntax::syntax_provider();
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec![], site_package_path: vec![],
        config_digest: ContentHash::of(b"c"), environment_digest: ContentHash::of(b"e"), lock_digest: None };
    let (run, _) = ProviderRun::new(provider.id(), context.id(), source.input, context.config_digest, [FactFamily::Syntax]).unwrap();
    let surface = ProviderSurface { provider: provider.id(), family: FactFamily::Syntax, name: "s".into() };
    let qualification = AssertionQualification { context: context.id(), scope: CoverageScope::Artifact { artifact: source.id() }.id(),
        condition: Diagram::always().id(), modality: Modality::Definite, approximation: Approximation::Exact };
    let error = typed_syntax::emit(parsed.syntax(), text, SyntaxInvocation { source: &source, qualification: &qualification, run: &run, surface: &surface },
        SyntaxLimits::default(), |_| panic!("changed content emitted")).unwrap_err();
    assert!(error.to_string().contains("differs from captured"), "{error}");
}

#[tokio::test]
async fn the_subset_publishes_a_conformance_generation_equal_to_memory() {
    let files = fixture();
    let memory = run_memory(&files, SyntaxLimits::default(), &budget()).unwrap();
    let (image, tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PostgreSQL18 required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap()); let store = GenerationStore::install(owner, model.clone()).await.unwrap();
    let captured = capture(&files); let budget = budget();
    let schedule = schedule(&model, SyntaxLimits::default());
    let mut execution = schedule.execute();
    let attempt = store.begin_conformance(writer, &mut execution, budget.clone()).await.unwrap();
    let facts = typed_syntax::extract(&captured, &model, SyntaxLimits::default(), &budget).unwrap();
    drive(&model, &mut execution, &attempt, &captured, facts, &budget).await.unwrap();
    let generation = attempt.seal(execution.finish().unwrap()).await.unwrap();
    assert_eq!(store.validate(generation, &budget).await.unwrap(), memory.digest, "the store and memory validate the same content");
    store.publish(generation).await.unwrap();
    assert!(matches!(store.select(generation).await, Err(Error::Frontier)), "a conformance generation is never selectable");
    let mut lease = store.pin(&reader, generation, budget.clone()).await.unwrap();
    let mut stored = Vec::new();
    lease.visit::<Occurrence>(|batch| { stored.extend(batch.rows().iter().cloned()); Ok(()) }).await.unwrap();
    stored.sort_by_key(Record::id);
    let mut expected = memory.occurrences.clone(); expected.sort_by_key(Record::id);
    assert_eq!(stored, expected);
    lease.release().await.unwrap();
    assert_eq!(budget.reserved(), 0);
}

/// P0-E envelope over a pinned input tree named by `LCTX_P0E_INPUT`: rows, peak budget
/// reservation, peak RSS and time for extraction, both stages and validation into memory.
/// Run with `-- --ignored typed_subset_envelope --nocapture`.
#[test]
#[ignore = "measurement: set LCTX_P0E_INPUT to a pinned input tree"]
fn typed_subset_envelope() {
    let root = std::path::PathBuf::from(std::env::var_os("LCTX_P0E_INPUT").expect("LCTX_P0E_INPUT names the input tree"));
    let mut paths: Vec<String> = walkdir::WalkDir::new(&root).into_iter().map(Result::unwrap).filter(|e| e.file_type().is_file())
        .map(|e| e.path().strip_prefix(&root).unwrap().to_string_lossy().into_owned())
        .filter(|p| p.ends_with(".py") || p.ends_with(".pyi")).collect();
    paths.sort();
    let budget = ResourceBudget::fixed(16 << 30).unwrap();
    let started = std::time::Instant::now();
    let captured = CapturedInput::capture(&root, &paths, &budget).unwrap();
    let model = model().unwrap();
    let schedule = schedule(&model, SyntaxLimits::default());
    let mut execution = schedule.execute();
    let generation = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let captured_at = started.elapsed();
    let facts = typed_syntax::extract(&captured, &model, SyntaxLimits::default(), &budget).unwrap();
    let extracted_at = started.elapsed();
    let rows = |batches: &[Batch<Occurrence>]| batches.iter().map(|b| b.rows().len()).sum::<usize>();
    let (occurrences, observations) = (rows(&facts.occurrences), facts.observations.iter().map(|b| b.rows().len()).sum::<usize>());
    let (partial, unavailable) = (facts.coverage.iter().filter(|r| r.status == CoverageStatus::Partial).count(),
        facts.coverage.iter().filter(|r| r.status == CoverageStatus::Unavailable).count());
    let bytes: i64 = captured.artifacts().iter().map(|a| a.byte_len).sum();
    ready(drive(&model, &mut execution, &generation, &captured, facts, &budget)).unwrap();
    execution.finish().unwrap();
    let staged_at = started.elapsed();
    let digest = generation.validate(&model, &budget).unwrap();
    let validated_at = started.elapsed();
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let hwm = status.lines().find(|l| l.starts_with("VmHWM")).unwrap_or("VmHWM: unknown").to_owned();
    println!("envelope files={} bytes={bytes} occurrences={occurrences} observations={observations} coverage_partial={partial} coverage_unavailable={unavailable}",
        paths.len());
    println!("envelope peak_reservation={:?} limit={} {hwm}", budget.peak(), budget.limit());
    println!("envelope capture={captured_at:?} extract={extracted_at:?} stages={staged_at:?} validate={validated_at:?} digest={digest:?}");
    // Exhaustion mid-attempt: half the measured peak refuses, and every reservation returns.
    drop(generation);
    let half = ResourceBudget::fixed(budget.peak().unwrap() / 2).unwrap();
    let mut execution = schedule.execute();
    let generation = MemoryGeneration::bind(&model, &half, &mut execution).unwrap();
    let refused = typed_syntax::extract(&captured, &model, SyntaxLimits::default(), &half)
        .and_then(|facts| ready(drive(&model, &mut execution, &generation, &captured, facts, &half)))
        .and_then(|()| generation.validate(&model, &half).map(drop));
    println!("envelope half_budget={} refused={:?} reserved_after={}", half.limit(), refused.as_ref().err().map(|e| e.to_string()), { drop(generation); half.reserved() });
    assert!(matches!(refused, Err(ModelError::Resource { .. })));
    assert_eq!(half.reserved(), 0);
}
