//! The production syntax stages (plan E1, carried by A0–A4) over the pinned native parse: `acquire`,
//! `pyrefly` and `assemble` run through the provider framework into a stage-bound memory generation
//! and into a real PostgreSQL 18 conformance generation, which validate the same content. The
//! schedule cannot be admitted as facts.
use cpg_extract::{
    acquisition::{Acquire, AcquiredInput},
    assembly::Assemble,
    bundle::{CapturedInputs, ProviderStage, run_stage},
    capture::CapturedInput,
    pyrefly_stage::{Pyrefly, pyrefly_provider},
    typed_syntax::{self, SyntaxInvocation, SyntaxLimits},
};
use lctx_model::domain::{
    admission::FrontierContract, assertion::*, attribution::*, batching::TransferLimits,
    conditions::Diagram, input::*, memory::MemoryGeneration, obligation::ObligationKind,
    resources::ResourceBudget, source::*, stages::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn fixture() -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/typed_semantics");
    let mut files = BTreeMap::new();
    for path in ["sample.py", "_invalid/broken.py"] {
        files.insert(path.to_owned(), std::fs::read(root.join(path)).unwrap());
    }
    // Evidence bytes are retained even when the analyzer cannot decode them.
    files.insert("_invalid/undecodable.py".into(), vec![0xff, 0x00, 0x80]);
    files
}
/// Capture a fresh copy of the files; every capture lives in its own temporary tree.
fn capture(files: &BTreeMap<String, Vec<u8>>, budget: &ResourceBudget, config_budget: &ResourceBudget) -> Result<Arc<CapturedInputs>, ModelError> {
    let original = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let target = original.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let captured = CapturedInput::capture(
        original.path(),
        &files.keys().cloned().collect::<Vec<_>>(),
        budget,
    )
    .unwrap();
    Ok(Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        captured,
        "typed-conformance",
    )], cpg_extract::native_context::NativeContextConfig::committed(lctx_model::domain::stages::Profile::Catalog, config_budget)?)))
}

/// What the stages wrote, read back through handoffs.
#[derive(Default, Clone)]
struct Rows {
    coverage: Vec<ProviderCoverage>,
    occurrences: Vec<Occurrence>,
    observations: Vec<SyntaxObservation>,
}
#[path = "typed_driver/mod.rs"]
mod typed_driver;
fn providers<S: StageSink + 'static>(limits: SyntaxLimits) -> Vec<Box<dyn ProviderStage<S>>> {
    vec![
        Box::new(Acquire::new(ContentHash::of(b"typed-conformance"))),
        Box::new(Pyrefly::new(limits)),
        Box::new(Assemble),
    ]
}
fn schedule(model: &ValidatedModel, limits: SyntaxLimits) -> Schedule {
    let stages = providers::<MemoryGeneration>(limits)
        .iter()
        .map(|p| p.declaration(Profile::Catalog))
        .collect();
    Schedule::build(model, stages, &[], Profile::Catalog).unwrap()
}
/// Drive every scheduled stage into `sink`.
async fn drive<S: StageSink + Send + 'static>(
    model: &Arc<ValidatedModel>,
    execution: &mut Execution<'_>,
    sink: &Arc<S>,
    captured: &Arc<CapturedInputs>,
    limits: SyntaxLimits,
    rows: &Arc<Mutex<Rows>>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let tables = typed_driver::Tables::default();
    let observed = typed_driver::ObservedSink {
        generation: sink.clone(),
        tables: tables.clone(),
    };
    let mut providers: BTreeMap<&str, Box<dyn ProviderStage<typed_driver::ObservedSink<S>>>> =
        providers::<typed_driver::ObservedSink<S>>(limits)
            .into_iter()
            .map(|p| (p.declaration(Profile::Catalog).name, p))
            .collect();
    let names: Vec<&'static str> = execution
        .schedule()
        .stages()
        .iter()
        .map(|s| s.name)
        .collect();
    for name in names {
        run_stage(
            providers.remove(name).unwrap(),
            execution.begin(name)?,
            &observed,
            model,
            captured,
            budget,
            TransferLimits::default(),
        )
        .await?;
    }
    let mut rows = rows.lock().unwrap();
    rows.coverage = typed_driver::rows(&tables);
    rows.occurrences = typed_driver::rows(&tables);
    rows.observations = typed_driver::rows(&tables);
    Ok(())
}
struct Run {
    digest: ContentHash,
    rows: Rows,
}
/// The whole schedule into a stage-bound memory generation.
async fn run_memory(
    files: &BTreeMap<String, Vec<u8>>,
    limits: SyntaxLimits,
    budget: &ResourceBudget,
) -> Result<Run, ModelError> {
    let model = Arc::new(ValidatedModel::validate(facts_relations())?);
    let captured = capture(files, &ResourceBudget::fixed(1 << 30).unwrap(), budget)?;
    let schedule = schedule(&model, limits);
    let mut execution = schedule.execute();
    let generation = Arc::new(MemoryGeneration::bind(&model, budget, &mut execution)?);
    let rows = Arc::new(Mutex::new(Rows::default()));
    drive(
        &model,
        &mut execution,
        &generation,
        &captured,
        limits,
        &rows,
        budget,
    )
    .await?;
    execution.finish()?;
    let rows = rows.lock().unwrap().clone();
    Ok(Run {
        digest: generation.validate(&model, budget)?,
        rows,
    })
}
fn artifact(files: &BTreeMap<String, Vec<u8>>, path: &str) -> SourceArtifact {
    let input = InputRevision::from_entries(
        files
            .iter()
            .map(|(p, b)| ManifestEntry {
                path: p.clone(),
                content: ContentHash::of(b),
                byte_len: b.len() as i64,
            })
            .collect(),
    )
    .unwrap();
    SourceArtifact::from_bytes(input.id(), path.into(), &files[path]).unwrap()
}
fn syntax_of<'a>(run: &'a Run, source: &SourceArtifact) -> &'a ProviderCoverage {
    run.rows
        .coverage
        .iter()
        .find(|row| {
            row.scope
                == CoverageScope::Artifact {
                    artifact: source.id(),
                }
                .id()
                && row.family == FactFamily::Syntax
        })
        .unwrap()
}

#[test]
fn the_syntax_schedule_is_not_a_facts_frontier() {
    let model = model().unwrap();
    let refused = FrontierContract::facts(&model, Profile::Catalog)
        .unwrap()
        .preflight(&schedule(&model, SyntaxLimits::default()));
    assert!(
        matches!(refused, Err(ModelError::Frontier(_))),
        "{refused:?}"
    );
}

#[tokio::test]
async fn stages_relocate_deterministically_and_disclose_coverage() {
    let files = fixture();
    let (left, right) = (
        run_memory(&files, SyntaxLimits::default(), &budget())
            .await
            .unwrap(),
        run_memory(&files, SyntaxLimits::default(), &budget())
            .await
            .unwrap(),
    );
    assert_eq!(
        left.digest, right.digest,
        "identity, configuration and provenance are independent of the capture path"
    );
    let sample = syntax_of(&left, &artifact(&files, "sample.py"));
    assert_eq!(
        (sample.status, sample.reason),
        (CoverageStatus::CompleteUnderStatedModel, None),
        "a clean module's syntax is complete"
    );
    let broken = syntax_of(&left, &artifact(&files, "_invalid/broken.py"));
    assert_eq!(
        (broken.status, broken.reason),
        (CoverageStatus::Partial, Some(ObligationKind::SyntaxError))
    );
    let undecodable = syntax_of(&left, &artifact(&files, "_invalid/undecodable.py"));
    assert_eq!(
        (undecodable.status, undecodable.reason),
        (
            CoverageStatus::Unavailable,
            Some(ObligationKind::UndecodableSource)
        )
    );
    assert!(
        left.rows
            .occurrences
            .iter()
            .all(|o| o.source != artifact(&files, "_invalid/undecodable.py").id()),
        "undecodable bytes never reach the analyzer"
    );
    // Exports and Signatures follow the syntax: complete for the clean module, partial or unavailable otherwise.
    for family in [FactFamily::Exports, FactFamily::Signatures] {
        let of = |path: &str| {
            left.rows
                .coverage
                .iter()
                .find(|c| {
                    c.family == family
                        && c.scope
                            == CoverageScope::Artifact {
                                artifact: artifact(&files, path).id(),
                            }
                            .id()
                })
                .map(|c| (c.status, c.reason))
                .unwrap()
        };
        assert_eq!(
            of("sample.py"),
            (CoverageStatus::CompleteUnderStatedModel, None),
            "{family:?}"
        );
        assert_eq!(
            of("_invalid/broken.py"),
            (CoverageStatus::Partial, Some(ObligationKind::SyntaxError)),
            "{family:?}"
        );
        assert_eq!(
            of("_invalid/undecodable.py"),
            (
                CoverageStatus::Unavailable,
                Some(ObligationKind::UndecodableSource)
            ),
            "{family:?}"
        );
    }
    let lexical = left
        .rows
        .coverage
        .iter()
        .find(|c| {
            c.family == FactFamily::Lexical
                && c.scope
                    == CoverageScope::Artifact {
                        artifact: artifact(&files, "sample.py").id(),
                    }
                    .id()
        })
        .unwrap();
    assert_eq!(
        lexical.status,
        CoverageStatus::CompleteUnderStatedModel,
        "the recognizer covers a clean module"
    );
    assert!(left.rows.observations.iter().any(|row| row.spelling == "α"));
    let with_items: Vec<_> = left
        .rows
        .occurrences
        .iter()
        .filter(|o| o.role == OccurrenceRole::WithItem)
        .collect();
    assert_eq!(with_items.len(), 2);
    assert_ne!(with_items[0].id(), with_items[1].id());
    assert_ne!(with_items[0].structural_path, with_items[1].structural_path);
}

#[tokio::test]
async fn admission_and_traversal_refusals_are_disclosed() {
    let files = fixture();
    let (sample, broken) = (
        artifact(&files, "sample.py"),
        artifact(&files, "_invalid/broken.py"),
    );
    // A source over the size bound is refused before Pyrefly is given it; others are unaffected.
    let small = run_memory(
        &files,
        SyntaxLimits {
            source_bytes: files["sample.py"].len() - 1,
            ..SyntaxLimits::default()
        },
        &budget(),
    )
    .await
    .unwrap();
    let refused = syntax_of(&small, &sample);
    assert_eq!(
        (refused.status, refused.reason),
        (
            CoverageStatus::Unavailable,
            Some(ObligationKind::ResourceRefused)
        )
    );
    assert!(
        small
            .rows
            .occurrences
            .iter()
            .all(|o| o.source != sample.id())
    );
    assert!(
        small
            .rows
            .occurrences
            .iter()
            .any(|o| o.source == broken.id())
    );
    // A traversal bound keeps the emitted, ancestor-closed prefix and states Partial.
    let bounded = run_memory(
        &files,
        SyntaxLimits {
            nodes: 2,
            ..SyntaxLimits::default()
        },
        &budget(),
    )
    .await
    .unwrap();
    let partial = syntax_of(&bounded, &sample);
    assert_eq!(
        (partial.status, partial.reason),
        (
            CoverageStatus::Partial,
            Some(ObligationKind::ResourceRefused)
        )
    );
    let mut prefix: Vec<_> = bounded
        .rows
        .occurrences
        .iter()
        .filter(|o| o.source == sample.id())
        .map(|o| o.structural_path.clone())
        .collect();
    prefix.sort();
    assert_eq!(prefix, [vec![0], vec![0, 0]]);
}

#[tokio::test]
async fn a_short_budget_fails_the_attempt_and_releases_everything() {
    let files = fixture();
    let short = ResourceBudget::fixed(64 << 10).unwrap();
    assert!(matches!(
        run_memory(&files, SyntaxLimits::default(), &short).await,
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(short.reserved(), 0);
}

#[test]
fn changed_text_is_refused_before_emission() {
    let files = fixture();
    let text = std::str::from_utf8(&files["sample.py"]).unwrap();
    let parsed = ruff_python_parser::parse_module(text).unwrap();
    let mut source = artifact(&files, "sample.py");
    source.content = ContentHash::of(b"changed");
    let provider = pyrefly_provider();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"c"),
        environment_digest: ContentHash::of(b"e"),
        lock_digest: None,
    };
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        source.input,
        context.config_digest,
        [FactFamily::Syntax],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Syntax,
        name: "s".into(),
    };
    let qualification = AssertionQualification {
        context: context.id(),
        scope: CoverageScope::Artifact {
            artifact: source.id(),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let error = typed_syntax::emit(
        parsed.syntax(),
        text,
        SyntaxInvocation {
            source: &source,
            qualification: &qualification,
            run: &run,
            surface: &surface,
        },
        SyntaxLimits::default(),
        |_| panic!("changed content emitted"),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("differs from captured"),
        "{error}"
    );
}

#[tokio::test]
async fn the_stages_publish_a_conformance_generation_equal_to_memory() {
    let files = fixture();
    let memory = run_memory(&files, SyntaxLimits::default(), &budget())
        .await
        .unwrap();
    let db = DisposableDatabase::start().await;
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let budget = budget();
    let captured = capture(&files, &budget, &budget).unwrap();
    let schedule = schedule(&model, SyntaxLimits::default());
    let mut execution = schedule.execute();
    let attempt = Arc::new(
        store
            .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
            .await
            .unwrap(),
    );
    let rows = Arc::new(Mutex::new(Rows::default()));
    drive(
        &model,
        &mut execution,
        &attempt,
        &captured,
        SyntaxLimits::default(),
        &rows,
        &budget,
    )
    .await
    .unwrap();
    let validated = Arc::try_unwrap(attempt)
        .unwrap_or_else(|_| panic!("observer retained attempt"))
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap();
    assert_eq!(
        validated.content(),
        memory.digest,
        "the store and memory validate the same content"
    );
    let generation = validated.publish().await.unwrap();
    assert!(
        matches!(store.select(generation).await, Err(Error::Frontier(_))),
        "a conformance generation is never selectable"
    );
    let mut lease = store
        .pin(&db.reader, generation, budget.clone())
        .await
        .unwrap();
    let mut stored = Vec::new();
    lease
        .visit::<Occurrence>(|batch| {
            stored.extend(batch.rows().iter().cloned());
            Ok(())
        })
        .await
        .unwrap();
    stored.sort_by_key(Record::id);
    let mut expected = memory.rows.occurrences.clone();
    expected.sort_by_key(Record::id);
    assert_eq!(stored, expected);
    lease.release().await.unwrap();
    drop(captured);
    assert_eq!(budget.reserved(), 0);
}

/// P0-E envelope over a pinned input tree named by `LCTX_P0E_INPUT`: rows, peak budget
/// reservation, peak RSS and time for capture, the stages and validation into memory.
/// Run with `-- --ignored typed_subset_envelope --nocapture`.
#[tokio::test]
#[ignore = "measurement: set LCTX_P0E_INPUT to a pinned input tree"]
async fn typed_subset_envelope() {
    let root = std::path::PathBuf::from(
        std::env::var_os("LCTX_P0E_INPUT").expect("LCTX_P0E_INPUT names the input tree"),
    );
    let mut paths: Vec<String> = walkdir::WalkDir::new(&root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            e.path()
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|p| p.ends_with(".py") || p.ends_with(".pyi"))
        .collect();
    paths.sort();
    let budget = ResourceBudget::fixed(16 << 30).unwrap();
    let started = std::time::Instant::now();
    let captured = Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(&root, &paths, &budget).unwrap(),
        "envelope",
    )], cpg_extract::native_context::NativeContextConfig::committed(lctx_model::domain::stages::Profile::Catalog, &budget).unwrap()));
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let schedule = schedule(&model, SyntaxLimits::default());
    let mut execution = schedule.execute();
    let generation = Arc::new(MemoryGeneration::bind(&model, &budget, &mut execution).unwrap());
    let captured_at = started.elapsed();
    let rows = Arc::new(Mutex::new(Rows::default()));
    drive(
        &model,
        &mut execution,
        &generation,
        &captured,
        SyntaxLimits::default(),
        &rows,
        &budget,
    )
    .await
    .unwrap();
    execution.finish().unwrap();
    let staged_at = started.elapsed();
    let digest = generation.validate(&model, &budget).unwrap();
    let validated_at = started.elapsed();
    let rows = rows.lock().unwrap().clone();
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let hwm = status
        .lines()
        .find(|l| l.starts_with("VmHWM"))
        .unwrap_or("VmHWM: unknown")
        .to_owned();
    println!(
        "envelope files={} occurrences={} observations={} coverage_rows={}",
        paths.len(),
        rows.occurrences.len(),
        rows.observations.len(),
        rows.coverage.len()
    );
    println!(
        "envelope peak_reservation={:?} limit={} {hwm}",
        budget.peak(),
        budget.limit()
    );
    println!(
        "envelope capture={captured_at:?} stages={staged_at:?} validate={validated_at:?} digest={digest:?}"
    );
}
