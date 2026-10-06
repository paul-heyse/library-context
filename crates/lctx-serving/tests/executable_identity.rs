//! Executable source membership is distinct from the model and wire semantic contracts.
#[path = "../build.rs"]
mod implementation_capture;

use lctx_model::domain::{self, ContentHash};
use std::{collections::BTreeMap, fs, path::Path};

fn fixture(workspace: &Path) -> std::path::PathBuf {
    let root = workspace.join("crates/lctx-serving");
    fs::create_dir_all(root.join("src")).unwrap();
    for (name, bytes) in [
        ("Cargo.toml", "[workspace]\n"),
        ("Cargo.lock", "version = 4\n"),
        (
            "crates/lctx-serving/Cargo.toml",
            "[package]\nname = 'serving'\n",
        ),
        ("crates/lctx-serving/build.rs", "fn main() {}\n"),
        ("crates/lctx-serving/src/lib.rs", "pub fn operation() {}\n"),
    ] {
        fs::write(workspace.join(name), bytes).unwrap();
    }
    root
}

// Decode the length framing independently so membership and raw content are specified directly.
fn fields(mut bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut values = Vec::new();
    while !bytes.is_empty() {
        let length = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        bytes = &bytes[8..];
        values.push(bytes[..length].to_vec());
        bytes = &bytes[length..];
    }
    values
}

#[test]
fn capture_tracks_added_deleted_and_renamed_helpers_and_raw_source_bytes() {
    let workspace = tempfile::tempdir().unwrap();
    let root = fixture(workspace.path());
    let configuration = BTreeMap::new();
    let original = implementation_capture::capture(&root, &configuration).unwrap();
    let decoded = fields(&original);
    assert_eq!(decoded[0], b"serving-executable-sources/v1");
    assert_eq!(
        decoded[1..11]
            .chunks_exact(2)
            .map(|pair| pair[0].as_slice())
            .collect::<Vec<_>>(),
        [
            b"../../Cargo.lock".as_slice(),
            b"../../Cargo.toml",
            b"Cargo.toml",
            b"build.rs",
            b"src/lib.rs"
        ]
    );
    assert_eq!(decoded[10], b"pub fn operation() {}\n");
    assert_eq!(decoded[11], b"build-configuration/v1");

    fs::create_dir(root.join("src/nested")).unwrap();
    let helper = root.join("src/nested/helper.rs");
    fs::write(&helper, b"pub fn default_value() -> bool { false }\n").unwrap();
    let added = implementation_capture::capture(&root, &configuration).unwrap();
    let added_fields = fields(&added);
    assert_eq!(added_fields[11], b"src/nested/helper.rs");
    assert_eq!(
        added_fields[12],
        b"pub fn default_value() -> bool { false }\n"
    );
    assert_ne!(ContentHash::of(&original), ContentHash::of(&added));

    fs::write(&helper, b"pub fn default_value() -> bool { true }\n").unwrap();
    let changed = implementation_capture::capture(&root, &configuration).unwrap();
    assert_ne!(ContentHash::of(&added), ContentHash::of(&changed));
    fs::write(
        &helper,
        b"pub fn default_value() -> bool { true }\n// comment\n",
    )
    .unwrap();
    assert_ne!(
        changed,
        implementation_capture::capture(&root, &configuration).unwrap()
    );

    let renamed = root.join("src/nested/renamed.rs");
    fs::rename(&helper, &renamed).unwrap();
    let moved = implementation_capture::capture(&root, &configuration).unwrap();
    assert_eq!(fields(&moved)[11], b"src/nested/renamed.rs");
    fs::remove_file(renamed).unwrap();
    assert_eq!(
        original,
        implementation_capture::capture(&root, &configuration).unwrap()
    );
}

#[test]
fn capture_binds_manifests_lockfile_and_sorted_build_configuration_without_checkout_paths() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let root = fixture(first.path());
    let other = fixture(second.path());
    let configuration = BTreeMap::from([
        ("CARGO_FEATURE_NATIVE".into(), "1".into()),
        ("CARGO_CFG_TARGET_FEATURE".into(), "sse,sse2".into()),
    ]);
    let original = implementation_capture::capture(&root, &configuration).unwrap();
    assert_eq!(
        original,
        implementation_capture::capture(&other, &configuration).unwrap()
    );
    let decoded = fields(&original);
    assert_eq!(decoded[12], b"CARGO_CFG_TARGET_FEATURE");
    assert_eq!(decoded[14], b"CARGO_FEATURE_NATIVE");
    let mut changed = configuration.clone();
    changed.insert("CARGO_FEATURE_ADDED".into(), "1".into());
    assert_ne!(
        original,
        implementation_capture::capture(&root, &changed).unwrap()
    );
    changed = configuration.clone();
    changed.insert("CARGO_CFG_TARGET_FEATURE".into(), "sse,sse2,avx".into());
    assert_ne!(
        original,
        implementation_capture::capture(&root, &changed).unwrap()
    );
    assert_ne!(
        original,
        implementation_capture::capture(&root, &BTreeMap::new()).unwrap()
    );
    for file in [
        "build.rs",
        "Cargo.toml",
        "../../Cargo.toml",
        "../../Cargo.lock",
    ] {
        let path = root.join(file);
        let saved = fs::read(&path).unwrap();
        fs::write(&path, [saved.as_slice(), b"\n# changed\n"].concat()).unwrap();
        assert_ne!(
            original,
            implementation_capture::capture(&root, &configuration).unwrap()
        );
        fs::write(path, saved).unwrap();
    }
}

#[test]
fn linked_operation_identity_is_installed_in_native_definitions() {
    let definition = lctx_serving::operation_definition();
    let installed = format!(
        "DEFINE FUNCTION fn::lctx_operation_definition() {{ RETURN '{}'; }};\n",
        definition.hex()
    );
    assert!(lctx_serving::native_definitions().ends_with(&installed));
    // A small two-build adverse control can compare these independently owned identities after
    // changing a helper body without editing declarations or the semantic policy revision.
    println!(
        "identity_control={}",
        serde_json::json!({
            "operation": definition.hex(),
            "model_implementation": domain::implementation_digest().hex(),
            "model_semantic": domain::graph::semantic_contract(&domain::model().unwrap()).hex(),
            "wire": domain::serving::wire_identity().0.hex(),
            "default_unavailable": domain::serving::DefaultValue::from_canonical(
                &domain::catalog::CatalogDefault::Unavailable {}
            )
        })
    );
}
