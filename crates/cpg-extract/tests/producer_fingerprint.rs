#[path = "../../../scripts/producer_fingerprint.rs"]
mod fingerprint;

#[test]
fn production_fingerprint_tracks_the_source_closure_and_survives_relocation() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let sources = [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "scripts/producer_fingerprint.rs",
        "crates/cpg-extract/src/runtime_scripts.rs",
        "third_party/provider.patch",
        "crates/producer/Cargo.toml",
        "crates/producer/build.rs",
        "crates/producer/src/lib.rs",
        "crates/model/Cargo.toml",
        "crates/model/src/domain/types.rs",
        "crates/store/Cargo.toml",
        "crates/store/src/schema.sql",
        "crates/store/sql/database.sql",
        "crates/store/migrations/service.sql",
        "crates/model/models/external.toml",
        "crates/schema/Cargo.toml",
        "scripts/deployment_check.py",
        "specs/embedding/spec.json",
    ];
    for root in [first.path(), second.path()] {
        for name in sources {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, name).unwrap();
        }
    }
    let files = fingerprint::files(first.path());
    let original = fingerprint::digest(first.path(), &files);
    assert_eq!(files, fingerprint::files(second.path()));
    let cache = second
        .path()
        .join("scripts/__pycache__/deployment_check.cpython-314.pyc");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(cache, "machine-local cache").unwrap();
    assert_eq!(files, fingerprint::files(second.path()));
    assert_eq!(
        original,
        fingerprint::digest(second.path(), &fingerprint::files(second.path()))
    );
    assert_eq!(original, fingerprint::digest(second.path(), &files));
    let administrative = second.path().join("scripts/administration.py");
    std::fs::write(&administrative, "unrelated administration").unwrap();
    assert_eq!(files, fingerprint::files(second.path()));
    assert_eq!(
        original,
        fingerprint::digest(second.path(), &fingerprint::files(second.path()))
    );
    std::fs::write(&administrative, "changed administration").unwrap();
    assert_eq!(
        original,
        fingerprint::digest(second.path(), &fingerprint::files(second.path()))
    );
    let runtime = second.path().join("scripts/second_runtime.py");
    std::fs::write(runtime, "new declared runtime").unwrap();
    let expanded = fingerprint::files_with_scripts(
        second.path(),
        &["scripts/deployment_check.py", "scripts/second_runtime.py"],
    );
    assert_ne!(original, fingerprint::digest(second.path(), &expanded));
    for name in sources {
        let path = second.path().join(name);
        std::fs::write(&path, format!("{name}: changed")).unwrap();
        assert_ne!(
            original,
            fingerprint::digest(second.path(), &files),
            "{name}"
        );
        std::fs::write(path, name).unwrap();
    }
    let tests = second.path().join("crates/producer/tests/control.rs");
    std::fs::create_dir_all(tests.parent().unwrap()).unwrap();
    std::fs::write(tests, "test only").unwrap();
    assert_eq!(files, fingerprint::files(second.path()));
    let added = second.path().join("crates/producer/src/new.rs");
    std::fs::write(added, "new production module").unwrap();
    let changed = fingerprint::files(second.path());
    assert_ne!(files, changed);
    assert_ne!(original, fingerprint::digest(second.path(), &changed));
}
