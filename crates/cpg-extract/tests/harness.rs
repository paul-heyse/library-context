//! Harness-equivalence oracle (DESIGN §4.2.5): the in-process Pysa structs equal the pinned Pyrefly
//! CLI's `--report-pysa-format json` output, per project module, compared as sets. It checks our
//! driver (configuration, reporter lifecycle, lazy solving), not Pysa's correctness. The CLI is the
//! `uv` dev-group Pyrefly of the same revision (docs/pins.md); it is never a production input.

mod common;

use std::collections::BTreeMap;
use std::process::Command;

use common::{input, layout, repo};
use cpg_extract::extract;
use serde_json::Value;

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

fn check(fixture: &str) {
    let dir = tempfile::tempdir().unwrap();
    let l = layout(fixture, dir.path());
    let mut i = input(&l, fixture);
    i.keep_pysa_json = true;
    let output = extract(&i).unwrap();
    let ours = output.pysa_json.clone();
    let public: std::collections::BTreeSet<String> = {
        let t = output.table("public_names").unwrap();
        let mut set = std::collections::BTreeSet::new();
        for col in ["access_path", "origin_path"] {
            let a = t.column(t.schema().index_of(col).unwrap());
            let a = a
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .unwrap();
            set.extend(a.iter().flatten().map(str::to_owned));
        }
        set
    };

    let toml = dir.path().join("pyrefly.toml");
    std::fs::write(
        &toml,
        format!(
            "project-includes = [\"{r}/**/*.py\", \"{r}/**/*.pyi\"]\nsearch-path = [\"{r}\"]\n\
             site-package-path = [\"{s}\"]\npython-version = \"3.14.0\"\npython-platform = \"linux\"\n\
             skip-interpreter-query = true\ndisable-search-path-heuristics = true\n\
             disable-project-excludes-heuristics = true\n",
            r = l.release_root.display(),
            s = l.site_packages.display()
        ),
    )
    .unwrap();
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
            let p = entry.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let module = name.rsplit_once(':').unwrap().0.to_owned();
            if !ours.contains_key(&module) {
                continue; // typeshed and dependencies: outside the analyzed boundary
            }
            let mut v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
            strip_module_ids(&mut v);
            theirs.entry(module).or_default().insert(kind.to_owned(), v);
        }
    }
    assert_eq!(
        theirs.len(),
        ours.len(),
        "the CLI reported every project module"
    );

    // Every symbol the CLI's `--public-only` report lists is explained by our public set: an
    // exact match or a public parent prefix (DESIGN §4.2.5).
    let coverage = Command::new("uv")
        .current_dir(repo())
        .args(["run", "--no-sync", "pyrefly", "coverage", "report", "-c"])
        .arg(&toml)
        .args(["--public-only", "-j", "1"])
        .output()
        .expect("blocked: `uv` with the dev-group pyrefly is required (docs/pins.md)");
    let report: Value = serde_json::from_slice(&coverage.stdout).expect("coverage JSON");
    for module in report["module_reports"].as_array().unwrap() {
        for name in module["names"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            let parts: Vec<&str> = name.split('.').collect();
            let explained = (1..=parts.len()).any(|n| public.contains(&parts[..n].join(".")));
            assert!(
                explained,
                "{fixture}: CLI public symbol {name} is not in our public set"
            );
        }
    }
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

#[test]
fn in_process_equals_cli_on_the_variant_fixture() {
    check("pysa_variants");
}

#[test]
fn in_process_equals_cli_on_the_unicode_fixture() {
    check("unicode_bom");
}
