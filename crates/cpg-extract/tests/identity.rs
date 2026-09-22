//! Review F1, F8, F9 and spike S2 oracles: identity is location-independent, output is
//! deterministic across processes, panics abort, and ambient knobs are refused.

mod common;

use std::path::Path;
use std::process::Command;

use common::{input, layout};
use cpg_extract::{ExtractError, extract};

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
fn a_panic_aborts_the_whole_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let l = layout("pysa_variants", dir.path());
    let mut i = input(&l, "pysa_variants");
    i.fault_at_module = Some("variants".to_owned());
    assert!(matches!(extract(&i), Err(ExtractError::Panicked)));
}

#[test]
fn relative_paths_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let l = layout("pysa_variants", dir.path());
    let mut i = input(&l, "pysa_variants");
    i.release_root = "relative/tree".into();
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
