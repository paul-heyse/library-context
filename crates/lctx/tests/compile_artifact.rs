//! CLI refusal boundaries act before acquiring environments or contacting services.
use std::process::Command;
#[test]
fn ordinary_compile_is_unavailable_before_acquisition() {
    let root=tempfile::tempdir().unwrap();
    let result=Command::new(env!("CARGO_BIN_EXE_lctx")).current_dir(root.path()).args(["compile","absent","--through","catalog"]).output().unwrap();
    assert_eq!(result.status.code(),Some(3));
    assert!(String::from_utf8_lossy(&result.stderr).contains("publisher"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(),0);
}
#[test]
fn artifact_destination_is_required_and_existing_content_is_preserved() {
    let root=tempfile::tempdir().unwrap();let output=root.path().join("graph");
    std::fs::create_dir(&output).unwrap();std::fs::write(output.join("sentinel"),b"preserve").unwrap();
    let missing=Command::new(env!("CARGO_BIN_EXE_lctx")).args(["compile","absent","--through","facts","--artifact-only"]).output().unwrap();
    assert_eq!(missing.status.code(),Some(2));
    let existing=Command::new(env!("CARGO_BIN_EXE_lctx")).current_dir(root.path()).args(["compile","absent","--through","facts","--artifact-only","--output"]).arg(&output).output().unwrap();
    assert_eq!(existing.status.code(),Some(2));
    assert_eq!(std::fs::read(output.join("sentinel")).unwrap(),b"preserve");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(),1);
}
