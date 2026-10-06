//! Native syntax coverage, relocation and resource controls through actual compiler streams.
//! PostgreSQL parity and old stage-grant conformance routes are retired with the compiler pivot.
use cpg_extract::{
    acquisition::AcquiredInput,
    bundle::CapturedInputs,
    capture::CapturedInput,
    pyrefly_stage::{Pyrefly, pyrefly_provider},
    typed_syntax::{self, SyntaxInvocation, SyntaxLimits},
};
use lctx_model::domain::{
    assertion::*, attribution::*, batching::TransferLimits, conditions::Diagram, input::*,
    obligation::ObligationKind, resources::ResourceBudget, source::*, stages::*, *,
};
use std::{collections::BTreeMap, path::Path, sync::Arc};

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
fn capture(
    files: &BTreeMap<String, Vec<u8>>,
    budget: &ResourceBudget,
    config_budget: &ResourceBudget,
) -> Result<Arc<CapturedInputs>, ModelError> {
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
    Ok(Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(captured, "typed-conformance")],
        cpg_extract::native_context::NativeContextConfig::committed(
            lctx_model::domain::stages::Profile::Catalog,
            config_budget,
        )?,
    )))
}

/// What the native providers completed, read back from immutable streams.
#[derive(Default, Clone)]
struct Rows {
    coverage: Vec<ProviderCoverage>,
    occurrences: Vec<Occurrence>,
    observations: Vec<SyntaxObservation>,
}
#[path = "typed_driver/mod.rs"]
mod typed_driver;
inspector!(Inspect, ProviderCoverage, Occurrence, SyntaxObservation);
struct Run {
    digest: ContentHash,
    rows: Rows,
}
async fn run_workspace(
    files: &BTreeMap<String, Vec<u8>>,
    limits: SyntaxLimits,
    resources: &ResourceBudget,
) -> Result<Run, ModelError> {
    let captured = capture(files, &budget(), resources)?;
    let tables = typed_driver::Tables::default();
    let digest = typed_driver::run_profile_with_budget(
        captured,
        Pyrefly::new(limits),
        Inspect(tables.clone()),
        Profile::Catalog,
        TransferLimits::default(),
        false,
        resources.clone(),
    )
    .await?;
    Ok(Run {
        digest,
        rows: Rows {
            coverage: typed_driver::rows(&tables),
            occurrences: typed_driver::rows(&tables),
            observations: typed_driver::rows(&tables),
        },
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

#[tokio::test]
async fn transfer_batches_and_provider_enumeration_preserve_completed_content() {
    let files = BTreeMap::from([(
        "sample.py".into(),
        b"def api(value: int) -> int:\n    return value + 1\n".to_vec(),
    )]);
    let captured = typed_driver::capture(&files, "typed-conformance", Profile::Catalog);
    let first = typed_driver::Tables::default();
    let second = typed_driver::Tables::default();
    let ordinary = typed_driver::run_profile_with_limits(
        captured.clone(),
        Pyrefly::new(SyntaxLimits::default()),
        Inspect(first.clone()),
        Profile::Catalog,
        TransferLimits::default(),
        false,
    )
    .await
    .unwrap();
    let single_rows = typed_driver::run_profile_with_limits(
        captured,
        Pyrefly::new(SyntaxLimits::default()),
        Inspect(second.clone()),
        Profile::Catalog,
        TransferLimits {
            rows: 1,
            ..TransferLimits::default()
        },
        true,
    )
    .await
    .unwrap();
    assert_eq!(ordinary, single_rows);
    assert_eq!(
        typed_driver::rows::<Occurrence>(&first),
        typed_driver::rows::<Occurrence>(&second)
    );
    assert_eq!(
        typed_driver::rows::<ProviderCoverage>(&first),
        typed_driver::rows::<ProviderCoverage>(&second)
    );
}

#[tokio::test]
async fn stages_relocate_deterministically_and_disclose_coverage() {
    let files = fixture();
    let (left, right) = (
        run_workspace(&files, SyntaxLimits::default(), &budget())
            .await
            .unwrap(),
        run_workspace(&files, SyntaxLimits::default(), &budget())
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
    // The recognizer and independent Ruff contextual pass both disclose Lexical coverage.
    let lexical = left
        .rows
        .coverage
        .iter()
        .filter(|c| {
            c.provider == Some(pyrefly_provider().id())
                && c.family == FactFamily::Lexical
                && c.scope
                    == CoverageScope::Artifact {
                        artifact: artifact(&files, "sample.py").id(),
                    }
                    .id()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lexical.len(),
        1,
        "one recognizer result for the captured artifact"
    );
    assert_eq!(
        (lexical[0].status, lexical[0].reason),
        (CoverageStatus::CompleteUnderStatedModel, None),
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
    let small = run_workspace(
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
    let bounded = run_workspace(
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
async fn a_short_budget_fails_the_workspace_and_releases_everything() {
    let files = fixture();
    let short = ResourceBudget::fixed(64 << 10).unwrap();
    assert!(matches!(
        run_workspace(&files, SyntaxLimits::default(), &short).await,
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(short.reserved(), 0);
}

#[test]
fn changed_text_is_refused_before_emission() {
    let files = fixture();
    let text = std::str::from_utf8(&files["sample.py"]).unwrap();
    let parsed = ruff_python_parser_latest::parse_module(text).unwrap();
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
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
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
