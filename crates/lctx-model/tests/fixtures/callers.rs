#[allow(
    dead_code,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)]
#[path = "lexical.rs"]
mod lexical_fixture;
use lctx_model::domain::{assertion::*, attribution::*, calls::*, input::*, source::*, *};

pub fn fixture(form: &str, mismatch: &str) -> lexical_fixture::Fixture {
    let mut f = lexical_fixture::Fixture::new();
    let q = f.rows::<AssertionQualification>()[0].clone();
    let input = f.rows::<InputRevision>()[0].clone();
    let context = f
        .rows::<AnalysisContext>()
        .into_iter()
        .find(|c| c.id() == q.context)
        .unwrap();
    let scope = f
        .rows::<CoverageScope>()
        .into_iter()
        .find(|s| s.id() == q.scope)
        .unwrap();
    let source = match scope {
        CoverageScope::Artifact { artifact } => artifact,
        _ => unreachable!(),
    };
    let site = f
        .rows::<Occurrence>()
        .into_iter()
        .find(|o| o.source == source)
        .unwrap();
    let module = Module {
        source: site.source,
        qualified_name: "example".into(),
    };
    let acquired = ProviderModule::Acquired {
        module: module.id(),
    };
    let provider = Provider {
        tool: "caller-control".into(),
        revision: "one".into(),
        build_digest: ContentHash::of(b"caller"),
    };
    let alien = Provider {
        tool: "foreign-caller".into(),
        ..provider.clone()
    };
    let other_context = AnalysisContext {
        python_platform: "foreign".into(),
        ..context.clone()
    };
    let (run, families) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        context.config_digest,
        [FactFamily::Calls],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Calls,
        name: "caller-control".into(),
    };
    let owner = if mismatch == "provider" {
        alien.id()
    } else {
        provider.id()
    };
    let native_context = if mismatch == "context" {
        other_context.id()
    } else {
        context.id()
    };
    let symbol = ProviderSymbol {
        provider: owner,
        context: native_context,
        module: acquired.id(),
        native_key: "f".into(),
        name: "f".into(),
        kind: if form == "class" {
            SymbolKind::Class
        } else {
            SymbolKind::Function
        },
    };
    let caller = match form {
        "module" => ProviderCallable::ModuleBody {
            provider: owner,
            context: native_context,
            module: acquired.id(),
        },
        "symbol" => ProviderCallable::Symbol {
            symbol: symbol.id(),
        },
        "class" => ProviderCallable::ClassBody { class: symbol.id() },
        "decorator" => ProviderCallable::DecoratorApplication {
            function: symbol.id(),
        },
        _ => unreachable!(),
    };
    let row = ProviderCallSite {
        qualification: q.id(),
        site: site.id(),
        origin: CallOrigin::explicit(),
        kind: PysaSiteKind::Regular,
        caller: caller.id(),
        callee: PysaCalleeKind::Call,
        is_attribute: None,
    };
    let evidence = Evidence::Occurrence {
        occurrence: site.id(),
    };
    let support = ProviderCallSiteSupport {
        assertion: row.id(),
        run: run.id(),
        surface: surface.id(),
        evidence: evidence.id(),
        origin: Origin::AnalyzerAssertion,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    macro_rules! append {
        ($ty:ty, $rows:expr) => {
            let mut rows = f.rows::<$ty>();
            rows.extend($rows);
            f.put(rows);
        };
    }
    append!(Provider, vec![provider, alien]);
    append!(AnalysisContext, vec![other_context]);
    append!(ProviderRun, vec![run]);
    append!(RunFamily, families);
    append!(ProviderSurface, vec![surface]);
    append!(Evidence, vec![evidence]);
    f.put(vec![CallOrigin::new(&[]).unwrap().0]);
    f.put(vec![module]);
    f.put(vec![acquired]);
    f.put(vec![symbol]);
    f.put(vec![caller]);
    f.put(vec![row]);
    f.put(vec![support]);
    f
}
