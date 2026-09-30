//! Harness-equivalence oracle (DESIGN §4.2.5): the `pyrefly` stage's in-process Pysa reports,
//! handed out by its session hook, equal the pinned Pyrefly CLI's `--report-pysa-format json` output
//! over the same frozen input, per analyzed module, compared as sets. It checks our driver
//! (configuration, reporter lifecycle, lazy solving), not Pysa's correctness. The CLI is the `uv`
//! dev-group Pyrefly of the same revision (docs/pins.md); it is never a production input.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use cpg_extract::{
    pyrefly_stage::{Pyrefly, PysaTap},
    typed_syntax::SyntaxLimits,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};

inspector!(Nothing, SourceArtifact);

fn repo() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}
fn strip_module_ids(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("module_id");
            m.values_mut().for_each(strip_module_ids);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_module_ids),
        _ => {}
    }
}
/// Lists compared as multisets (hash-set-valued lists come out in varying order).
fn canon(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.iter().map(|(k, x)| (k.clone(), canon(x))).collect()),
        Value::Array(a) => {
            let mut items: Vec<Value> = a.iter().map(canon).collect();
            items.sort_by_key(|x| x.to_string());
            Value::Array(items)
        }
        other => other.clone(),
    }
}

async fn check(fixture: &str) {
    let captured = typed_driver::capture(&typed_driver::files(fixture), fixture);
    let root = captured.inputs()[0].captured().root().to_path_buf();
    let tap: PysaTap = Arc::new(Mutex::new(BTreeMap::new()));
    typed_driver::run_with(
        captured.clone(),
        Pyrefly::with_tap(SyntaxLimits::default(), tap.clone()),
        Nothing(Default::default()),
    )
    .await
    .unwrap();
    let mut ours = tap.lock().unwrap().clone();
    ours.values_mut().for_each(strip_module_ids);
    assert!(!ours.is_empty(), "the session reported its modules");

    // The CLI over the same frozen root, configured as the session is for a tree.
    let dir = tempfile::tempdir().unwrap();
    let toml = dir.path().join("pyrefly.toml");
    std::fs::write(&toml, format!("project-includes = [\"{r}/**/*.py\", \"{r}/**/*.pyi\"]\nsearch-path = [\"{r}\"]\nsite-package-path = []\n\
        python-version = \"3.14.7\"\npython-platform = \"linux\"\nskip-interpreter-query = true\ndisable-search-path-heuristics = true\n\
        disable-project-excludes-heuristics = true\n", r = root.display())).unwrap();
    let report = dir.path().join("pysa");
    let status = Command::new("uv")
        .current_dir(repo())
        .args(["run", "--no-sync", "pyrefly", "check", "-c"])
        .arg(&toml)
        .arg("--report-pysa")
        .arg(&report)
        .args(["--report-pysa-format", "json", "--summary=none", "-j", "1"])
        .status()
        .expect("blocked: `uv` with the dev-group pyrefly is required (docs/pins.md)");
    assert!(status.code().is_some(), "pyrefly CLI was killed");
    let mut theirs: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    for kind in ["definitions", "call_graphs"] {
        for entry in std::fs::read_dir(report.join(kind)).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let module = name.rsplit_once(':').unwrap().0.to_owned();
            if !ours.contains_key(&module) {
                continue;
            } // typeshed and dependencies: outside the analyzed roots
            let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            strip_module_ids(&mut value);
            theirs
                .entry(module)
                .or_default()
                .insert(kind.to_owned(), value);
        }
    }
    assert_eq!(
        theirs.len(),
        ours.len(),
        "the CLI reported every analyzed module"
    );
    for (module, pair) in &ours {
        for kind in ["definitions", "call_graphs"] {
            assert_eq!(
                canon(&pair[kind]),
                canon(&theirs[module][kind]),
                "{fixture}: {module} {kind} differs from the CLI"
            );
        }
    }
}

#[tokio::test]
async fn in_process_equals_cli_on_the_variant_fixture() {
    check("pysa_variants").await;
}

#[tokio::test]
async fn in_process_equals_cli_on_the_unicode_fixture() {
    check("unicode_bom").await;
}
