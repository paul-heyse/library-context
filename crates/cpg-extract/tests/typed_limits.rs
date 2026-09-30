//! Total traversal work of typed syntax emission (plan R3; input-validation review F02).
//! Inputs are parsed by the pinned Ruff parser Pyrefly uses; the emitter's bounds are what is
//! under test, so no analyzer transaction is needed.
use cpg_extract::typed_syntax::{
    self, SyntaxError, SyntaxInvocation, SyntaxLimit, SyntaxLimits, SyntaxWork,
};
use lctx_model::domain::{
    assertion::*, attribution::*, conditions::Diagram, input::*, source::*, *,
};

struct Input {
    source: SourceArtifact,
    qualification: AssertionQualification,
    run: ProviderRun,
    surface: ProviderSurface,
}
fn input(text: &str) -> Input {
    let manifest = InputRevision::from_entries(vec![ManifestEntry {
        path: "big.py".into(),
        content: ContentHash::of(text.as_bytes()),
        byte_len: text.len() as i64,
    }])
    .unwrap();
    let source =
        SourceArtifact::from_bytes(manifest.id(), "big.py".into(), text.as_bytes()).unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"limits"),
        environment_digest: manifest.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "pyrefly-retained-ruff-ast".into(),
        revision: "pinned".into(),
        build_digest: ContentHash::of(b"limits"),
    };
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        manifest.id(),
        context.config_digest,
        [FactFamily::Syntax],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Syntax,
        name: "syntax".into(),
    };
    let module = Module {
        source: source.id(),
        qualified_name: "big".into(),
    };
    let qualification = AssertionQualification {
        context: context.id(),
        scope: CoverageScope::Module {
            module: module.id(),
        }
        .id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    Input {
        source,
        qualification,
        run,
        surface,
    }
}
/// Parse and emit on a thread with room for the parser's own recursion; returns the result and
/// the structural paths of the events the sink received.
fn paths(text: String, limits: SyntaxLimits) -> (Result<SyntaxWork, SyntaxError>, Vec<Vec<i32>>) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let parsed = ruff_python_parser::parse_module(&text).expect("valid Python");
            let input = input(&text);
            let mut events = Vec::new();
            let invocation = SyntaxInvocation {
                source: &input.source,
                qualification: &input.qualification,
                run: &input.run,
                surface: &input.surface,
            };
            let result = typed_syntax::emit(parsed.syntax(), &text, invocation, limits, |event| {
                events.push(event.occurrence.structural_path);
                Ok(())
            });
            (result, events)
        })
        .unwrap()
        .join()
        .unwrap()
}
fn run(text: String, limits: SyntaxLimits) -> (Result<SyntaxWork, SyntaxError>, usize) {
    let (result, events) = paths(text, limits);
    (result, events.len())
}
fn refused(result: Result<SyntaxWork, SyntaxError>) -> (SyntaxLimit, SyntaxWork) {
    match result {
        Err(SyntaxError::Refused { limit, work }) => (limit, work),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
fn limited(nodes: usize) -> SyntaxLimits {
    SyntaxLimits {
        nodes,
        ..SyntaxLimits::default()
    }
}

#[test]
fn a_halted_statement_list_stops_at_once() {
    let text = "x = 1\n".repeat(50_000);
    // Module, then (assign, name, number) per statement: the fourth statement is refused.
    let (result, events) = run(text.clone(), limited(10));
    let (limit, work) = refused(result);
    assert_eq!((limit, work.emitted, events), (SyntaxLimit::Nodes, 10, 10));
    assert!(
        work.residual <= SyntaxLimits::default().depth,
        "the owned statement loop stops: {work:?}"
    );
    // The kept prefix is a tree: every emitted node's parent was emitted before it.
    let (_, emitted) = paths(text.clone(), limited(10));
    for (index, path) in emitted.iter().enumerate().skip(1) {
        assert!(
            emitted[..index].contains(&path[..path.len() - 1].to_vec()),
            "{path:?} lacks its parent"
        );
    }
    let (result, events) = run(text, SyntaxLimits::default());
    assert_eq!(result.unwrap().emitted, 1 + 3 * 50_000);
    assert_eq!(events, 1 + 3 * 50_000);
}

#[test]
fn a_halted_element_list_costs_one_return_per_remaining_element() {
    let text = format!("x = [{}]\n", vec!["1"; 50_000].join(", "));
    // Module, assign, name, list, then six elements; the seventh element is refused.
    let (limit, work) = refused(run(text.clone(), limited(10)).0);
    assert_eq!((limit, work.emitted), (SyntaxLimit::Nodes, 10));
    assert_eq!(
        work.residual,
        50_000 - 7,
        "each remaining element is one immediate return"
    );
    assert_eq!(work.deepest, 4, "no descent after the halt");
    assert_eq!(
        run(text, SyntaxLimits::default()).0.unwrap().emitted,
        4 + 50_000
    );
}

#[test]
fn deep_nesting_is_refused_at_the_depth_bound() {
    let text = format!("x = {}1{}\n", "[".repeat(300), "]".repeat(300));
    let (limit, work) = refused(run(text, SyntaxLimits::default()).0);
    assert_eq!(limit, SyntaxLimit::Depth);
    assert!(
        work.deepest <= 257,
        "the visitor stack never exceeds the bound plus the refused frame: {work:?}"
    );
    let shallow = format!("x = {}1{}\n", "[".repeat(100), "]".repeat(100));
    // Module, assign, 100 lists, then the number.
    assert_eq!(
        run(shallow, SyntaxLimits::default()).0.unwrap().deepest,
        2 + 100 + 1
    );
}

#[test]
fn an_oversized_source_is_refused_before_any_callback() {
    let limits = SyntaxLimits {
        source_bytes: 16 << 20,
        ..SyntaxLimits::default()
    };
    let text = "#".repeat(17 << 20);
    let big = input(&text);
    let error = typed_syntax::admit(&big.source, limits).unwrap_err();
    assert!(
        matches!(error, SyntaxError::Refused { limit: SyntaxLimit::SourceBytes, work } if work == SyntaxWork::default())
    );
    assert_eq!(
        error.coverage(),
        Some((CoverageStatus::Unavailable, ObligationKind::ResourceRefused)),
        "nothing was emitted"
    );
    // The emitter repeats the admission before any traversal or sink call.
    let empty = ruff_python_parser::parse_module("").unwrap();
    let invocation = SyntaxInvocation {
        source: &big.source,
        qualification: &big.qualification,
        run: &big.run,
        surface: &big.surface,
    };
    let (limit, work) = refused(typed_syntax::emit(
        empty.syntax(),
        &text,
        invocation,
        limits,
        |_| panic!("an oversized source emitted"),
    ));
    assert_eq!(
        (limit, work),
        (SyntaxLimit::SourceBytes, SyntaxWork::default())
    );
    let within = input(&"#".repeat(16 << 20));
    assert!(typed_syntax::admit(&within.source, limits).is_ok());
}

#[test]
fn the_callback_cap_bounds_work_without_emission_limits() {
    let text = "x = 1\n".repeat(100);
    let (limit, work) = refused(
        run(
            text.clone(),
            SyntaxLimits {
                callbacks: 20,
                ..SyntaxLimits::default()
            },
        )
        .0,
    );
    assert_eq!((limit, work.callbacks), (SyntaxLimit::Callbacks, 21));
    assert!(work.emitted < 1 + 3 * 100);
    let full = run(text, SyntaxLimits::default()).0.unwrap();
    assert_eq!((full.emitted, full.residual), (1 + 3 * 100, 0));
    assert!(
        full.callbacks >= 3 * 100,
        "every statement, target and value is a dispatch"
    );
}
