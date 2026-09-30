//! The typed model's reviewable snapshot (P0 exit F06): relations, fields with their roles and
//! types, references, sums and every codebook's (code, label) pairs, from `lctx model describe
//! --format json`. A schema change is a schema migration: its diff is reviewed before acceptance.
#[test]
fn model_description_snapshot() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lctx"))
        .args(["model", "describe", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    insta::assert_snapshot!("model_describe", String::from_utf8(output.stdout).unwrap());
}
