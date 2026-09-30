//! The owner rule over Pyrefly's retained Ruff parse of fixtures/python/semantic_owner/owner.py.
//! Pre-written answers name each call's owner by the callee's spelling.
use cpg_extract::typed_syntax::{self, SyntaxInvocation, SyntaxLimits};
use lctx_model::domain::{
    assertion::*, attribution::*, conditions::*, input::*, occurrence_owner::*,
    resources::ResourceBudget, source::*, *,
};
use pyrefly::state::{require::Require, state::State};
use pyrefly_config::{
    config::{ConfigFile, ConfigSource},
    finder::ConfigFinder,
};
use pyrefly_python::{
    module_path::ModulePath,
    sys_info::{PythonPlatform, PythonVersion},
};
use pyrefly_util::{arc_id::ArcId, thread_pool::ThreadCount};
use std::{collections::BTreeMap, path::Path};

fn occurrences() -> (Vec<Occurrence>, Vec<u8>) {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/semantic_owner/owner.py"),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("owner.py"), &bytes).unwrap();
    let mut cfg = ConfigFile {
        source: ConfigSource::File(dir.path().join("pyrefly.toml")),
        search_path_from_args: vec![dir.path().to_path_buf()],
        disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true,
        enable_fallback_search_path: false,
        ..ConfigFile::default()
    };
    cfg.python_environment.python_version = Some(PythonVersion::new(3, 14, 7));
    cfg.python_environment.python_platform = Some(PythonPlatform::new("linux"));
    cfg.python_environment.site_package_path = Some(vec![]);
    cfg.interpreters.skip_interpreter_query = true;
    assert!(cfg.configure().is_empty());
    let handle = cfg.handle_from_module_path(ModulePath::filesystem(dir.path().join("owner.py")));
    let state = State::new(
        ConfigFinder::new_constant(ArcId::new(cfg)),
        ThreadCount::Inline,
    );
    let mut txn = state.new_transaction(Require::Exports, None);
    txn.run(std::slice::from_ref(&handle), Require::Everything, None);
    let ast = txn.get_ast(&handle).expect("retained native AST");
    let text = txn
        .get_module_info(&handle)
        .unwrap()
        .lined_buffer()
        .contents()
        .clone();
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "owner.py".into(),
        content: ContentHash::of(&bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "owner.py".into(), &bytes).unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"owner"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "pyrefly-retained-ruff-ast".into(),
        revision: "pinned".into(),
        build_digest: ContentHash::of(b"owner-test"),
    };
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
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
        qualified_name: "owner".into(),
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
    let mut occurrences = Vec::new();
    typed_syntax::emit(
        &ast,
        &text,
        SyntaxInvocation {
            source: &source,
            qualification: &qualification,
            run: &run,
            surface: &surface,
        },
        SyntaxLimits::default(),
        |event| {
            occurrences.push(event.occurrence);
            Ok(())
        },
    )
    .unwrap();
    (occurrences, bytes)
}

#[test]
fn calls_in_a_real_parse_belong_to_their_innermost_declaration_body() {
    let (occurrences, bytes) = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(occurrences)
        .unwrap()
        .join()
        .unwrap();
    let table = OwnerTable::build(&occurrences, &ResourceBudget::fixed(64 << 20).unwrap()).unwrap();
    for occurrence in &occurrences {
        assert_eq!(
            table.owner(occurrence.id()),
            Some(owner_of(occurrence, &occurrences).unwrap()),
            "sweep equals oracle"
        );
    }
    let by_path: BTreeMap<&[i32], &Occurrence> = occurrences
        .iter()
        .map(|o| (o.structural_path.as_slice(), o))
        .collect();
    let text = |o: &Occurrence| {
        std::str::from_utf8(&bytes[o.start as usize..o.end as usize])
            .unwrap()
            .to_owned()
    };
    let mut owners = BTreeMap::new();
    for call in occurrences
        .iter()
        .filter(|o| o.syntax_kind == SyntaxKind::ExprCall)
    {
        let mut callee = call.structural_path.clone();
        callee.push(0);
        let owner = occurrences
            .iter()
            .find(|o| Some(o.id()) == table.owner(call.id()))
            .unwrap();
        let label = match owner.syntax_kind {
            SyntaxKind::ModModule => "module".to_owned(),
            SyntaxKind::ExprLambda => "lambda".to_owned(),
            // A decorated declaration's range starts at its decorator; label it by its keyword.
            _ => {
                let whole = text(owner);
                let at = ["def ", "class "]
                    .iter()
                    .filter_map(|keyword| whole.find(keyword))
                    .min()
                    .unwrap();
                whole[at..].split(['(', ':']).next().unwrap().to_owned()
            }
        };
        owners.insert(text(by_path[callee.as_slice()]), label);
    }
    let expected: BTreeMap<String, String> = [
        ("g", "module"),
        ("ann", "module"),
        ("h", "def f"),
        ("k", "def f"),
        ("deco", "module"),
        ("base", "module"),
        ("k2", "class C"),
        ("m2", "lambda"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .collect();
    assert_eq!(owners, expected);
}
