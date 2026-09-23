//! Review F1, F8, F9 and spike S2 oracles, and the slice-1 review's F2, F6 and F9: identity is
//! location-independent but follows the dependency environment, output is deterministic across
//! processes and module orders, panics abort, and ambient knobs are refused.

mod common;

use std::path::Path;
use std::process::Command;

use common::{cell, copy_tree, fixture, input, layout};
use cpg_extract::{ExtractError, ExtractOutput, extract};

#[test]
fn identities_do_not_depend_on_install_location() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let la = layout("unicode_bom", &a.path().join("one/deeper"));
    let lb = layout("unicode_bom", b.path());
    let x = extract(&input(&la, "unicode_bom")).unwrap();
    let y = extract(&input(&lb, "unicode_bom")).unwrap();
    assert_eq!(
        x.context_id, y.context_id,
        "context digest is root-relative"
    );
    assert_eq!(x.run_id, y.run_id);
    for ((n, bx), (_, by)) in x.tables.iter().zip(&y.tables) {
        assert_eq!(bx, by, "table {n} differs between install locations");
    }
}

#[test]
fn identities_follow_the_dependency_environment() {
    let dir = tempfile::tempdir().unwrap();
    let run_with = |site: &str| {
        let root = dir.path().join(site);
        let l = layout("dep_env/release", &root);
        copy_tree(&fixture("dep_env").join(site), &l.site_packages);
        extract(&input(&l, "dep_env")).unwrap()
    };
    let (a, b) = (run_with("site_a"), run_with("site_b"));
    let contexts = |o: &ExtractOutput| cell(o.table("contexts").unwrap(), "environment_digest", 0);
    assert_ne!(contexts(&a), contexts(&b));
    assert_ne!(
        a.context_id, b.context_id,
        "the context names the dependency content"
    );
    assert_ne!(a.run_id, b.run_id);
    let pysa = |o: &ExtractOutput| {
        let t = o.table("pysa_calls").unwrap();
        (0..t.num_rows())
            .map(|r| (cell(t, "phase", r), cell(t, "target_name", r)))
            .collect::<Vec<_>>()
    };
    assert_ne!(pysa(&a), pysa(&b), "the environments give different facts");
}

#[test]
fn module_order_does_not_change_the_cycle_output() {
    let dir = tempfile::tempdir().unwrap();
    let l = layout("import_cycle", dir.path());
    let sorted = extract(&input(&l, "import_cycle")).unwrap();
    let mut i = input(&l, "import_cycle");
    i.test_hooks.reverse_module_order = true;
    let reversed = extract(&i).unwrap();
    for ((n, x), (_, y)) in sorted.tables.iter().zip(&reversed.tables) {
        assert_eq!(x, y, "table {n} depends on module order");
    }
    // The cycle's solved return types reach the output: `left(3).bit_length()` and
    // `right(3).upper()` resolve on `int` and `str`.
    let calls = sorted.table("pysa_calls").unwrap();
    let resolved: Vec<String> = (0..calls.num_rows())
        .map(|r| {
            format!(
                "{}:{}",
                cell(calls, "target_module", r),
                cell(calls, "target_name", r)
            )
        })
        .collect();
    assert!(
        resolved.contains(&"builtins:bit_length".to_owned()),
        "{resolved:?}"
    );
    assert!(
        resolved.contains(&"builtins:upper".to_owned()),
        "{resolved:?}"
    );
}

/// The extractor's id recipes on one fixture: a changed recipe, codebook text or `NodeKind` name
/// shows here (DESIGN §3.4.1). Syntax ids are producer-scoped: a ruff bump may move them.
#[test]
fn extractor_id_recipes_snapshot() {
    let (_dir, out) = common::run("unicode_bom");
    let mut lines = vec![
        format!("producer_id {}", out.producer_id.hex()),
        format!("context_id  {}", out.context_id.hex()),
        format!("run_id      {}", out.run_id.hex()),
    ];
    for (table, columns) in [
        ("source_files", &["path", "module_node_id", "fact_id"][..]),
        (
            "declarations",
            &["qualified_name", "node_id", "fact_id"][..],
        ),
        ("parameter_syntax", &["name", "node_id", "fact_id"][..]),
        ("call_syntax", &["start_byte", "node_id", "fact_id"][..]),
    ] {
        let t = out.table(table).unwrap();
        let first = columns
            .iter()
            .map(|c| cell(t, c, 0))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("{table}[0] {first}"));
    }
    insta::assert_snapshot!(lines.join("\n"));
}

#[test]
fn a_panic_aborts_the_whole_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let l = layout("pysa_variants", dir.path());
    let mut i = input(&l, "pysa_variants");
    i.test_hooks.fault_at_module = Some("variants".to_owned());
    assert!(matches!(extract(&i), Err(ExtractError::Panicked)));
}

#[test]
fn relative_paths_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let l = layout("pysa_variants", dir.path());
    let mut i = input(&l, "pysa_variants");
    i.release.root = "relative/tree".into();
    assert!(matches!(extract(&i), Err(ExtractError::RelativePath(_))));
}

fn cli(root: &Path, out: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_lctx-extract"));
    c.args(["--release-root"])
        .arg(root.join("release"))
        .arg("--venv-root")
        .arg(root.join("venv"))
        .arg("--site-packages")
        .arg(root.join("venv/lib/python3.14/site-packages"))
        .args(["--release-label", "import_cycle", "--snapshot"])
        .arg("07".repeat(16))
        .arg("--out")
        .arg(out);
    c
}

#[test]
fn import_cycle_output_is_identical_across_processes() {
    let dir = tempfile::tempdir().unwrap();
    layout("import_cycle", dir.path());
    let mut outputs = Vec::new();
    for n in 0..3 {
        let out = dir.path().join(format!("out{n}"));
        let status = cli(dir.path(), &out).status().unwrap();
        assert!(status.success());
        let mut files: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        outputs.push(
            files
                .iter()
                .map(|f| (f.file_name().unwrap().to_owned(), std::fs::read(f).unwrap()))
                .collect::<Vec<_>>(),
        );
    }
    assert!(!outputs[0].is_empty());
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[1], outputs[2]);
}

#[test]
fn ambient_pyrefly_knobs_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    layout("import_cycle", dir.path());
    for (k, v) in [("PYREFLY_STACK_SIZE", "1"), ("PYSA_DUMP_CALL_GRAPH", "1")] {
        let out = cli(dir.path(), &dir.path().join("refused"))
            .env(k, v)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{k} must be refused");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("refusing to run") && stderr.contains(k),
            "{stderr}"
        );
    }
}
