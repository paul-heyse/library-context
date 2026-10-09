#[path = "../../../scripts/producer_fingerprint.rs"]
mod fingerprint;

use std::path::Path;

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}
fn package(root: &Path, name: &str, dependencies: &str) {
    write(root, &format!("crates/{name}/Cargo.toml"), &format!(
        "[package]\nname = {name:?}\nversion = \"0.1.0\"\n{dependencies}"
    ));
    write(root, &format!("crates/{name}/src/lib.rs"), name);
}
fn fixture(root: &Path) {
    write(root, "Cargo.toml", r#"
[workspace]
members = ["crates/*"]
[workspace.dependencies]
model = { package = "lctx-model", path = "crates/lctx-model" }
builder = { path = "crates/builder" }
[patch.crates-io]
patched = { path = "third_party/patched" }
"#);
    for name in ["Cargo.lock", "rust-toolchain.toml", "scripts/producer_fingerprint.rs",
        "third_party/provider.patch", "scripts/deployment_check.py", "specs/embedding/spec.json"] {
        write(root, name, name);
    }
    package(root, "cpg-extract", r#"
[dependencies]
model.workspace = true
flow = { path = "../cpg-flow" }
optional = { path = "../optional", optional = true }
[build-dependencies]
builder.workspace = true
[target.'cfg(windows)'.dependencies]
target = { path = "../target" }
[target.'cfg(windows)'.build-dependencies]
target-builder = { path = "../target-builder" }
[dev-dependencies]
core = { path = "../cpg-core" }
[target.'cfg(unix)'.dev-dependencies]
serving = { path = "../serving" }
"#);
    package(root, "cpg-flow", "[dependencies]\nmodel.workspace = true\n");
    package(root, "lctx-model", "[dependencies]\nmacros = { path = \"../lctx-model-macros\" }\n");
    package(root, "lctx-model-macros", "");
    package(root, "cpg-core", "[dependencies]\nprovider = { path = \"../cpg-extract\" }\nnative = { path = \"../native\" }\n");
    package(root, "native", "[dependencies]\nmodel.workspace = true\n");
    for name in ["optional", "builder", "target", "target-builder", "serving"] {
        package(root, name, "");
    }
    write(root, "third_party/patched/Cargo.toml", "[package]\nname = \"patched\"\nversion = \"0.1.0\"\n[dependencies]\nhelper = { path = \"../../crates/patch-helper\" }\n");
    write(root, "third_party/patched/src/lib.rs", "patched source");
    package(root, "patch-helper", "");
    // Custom build/target roots come from Cargo declarations, not a separate input registry.
    package(root, "custom", "[lib]\npath = \"implementation/lib.rs\"\n");
    write(root, "crates/custom/implementation/lib.rs", "custom library");
    write(root, "crates/custom/implementation/module.rs", "adjacent module");
    write(root, "crates/builder/Cargo.toml", "[package]\nname = \"builder\"\nversion = \"0.1.0\"\nbuild = \"setup/build.rs\"\n[build-dependencies]\ncustom = { path = \"../custom\" }\n");
    write(root, "crates/builder/setup/build.rs", "custom build");
    write(root, "crates/builder/setup/build_support.rs", "adjacent build module");
    write(root, "crates/cpg-extract/build_support.rs", "root build module");
    write(root, "crates/cpg-extract/build.rs", "provider build adapter");
    write(root, "crates/cpg-extract/src/runtime_scripts.rs", "runtime membership");
    write(root, "crates/lctx-model/models/external.toml", "model assets");
    write(root, "crates/native/sql/database.sql", "schema");
    write(root, "crates/native/migrations/migration.sql", "migration");
}
fn capture(root: &Path, owner: &str) -> fingerprint::SourceClosure {
    fingerprint::capture(root, &root.join(format!("crates/{owner}")))
}
fn digest(root: &Path, owner: &str) -> blake3::Hash {
    fingerprint::digest(root, &capture(root, owner).files)
}

#[test]
fn production_fingerprint_is_rooted_and_survives_relocation() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    fixture(first.path());
    fixture(second.path());
    let original = digest(first.path(), "cpg-extract");
    let closure = capture(first.path(), "cpg-extract");
    let relocated = capture(second.path(), "cpg-extract");
    assert_eq!(closure.files, relocated.files);
    assert_eq!(closure.watches, relocated.watches);
    assert_eq!(original, digest(second.path(), "cpg-extract"));
    assert!(closure.files.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(closure.files.iter().all(|name| !Path::new(name).is_absolute() && !name.contains("..")));
    for name in ["crates/cpg-extract/src/lib.rs", "crates/cpg-flow/src/lib.rs",
        "crates/lctx-model/src/lib.rs", "crates/lctx-model-macros/src/lib.rs",
        "crates/optional/src/lib.rs", "crates/target/src/lib.rs", "crates/target-builder/src/lib.rs",
        "crates/builder/setup/build.rs", "crates/builder/setup/build_support.rs",
        "crates/cpg-extract/build_support.rs", "crates/custom/implementation/module.rs",
        "third_party/patched/src/lib.rs", "crates/patch-helper/src/lib.rs",
        "third_party/provider.patch", "scripts/deployment_check.py", "specs/embedding/spec.json"] {
        assert!(closure.files.contains(&name.into()), "missing {name}");
    }
    for name in ["crates/cpg-core/src/lib.rs", "crates/native/sql/database.sql", "crates/serving/src/lib.rs"] {
        assert!(!closure.files.contains(&name.into()), "unrelated {name}");
    }
    for name in ["crates/cpg-extract/src", "crates/lctx-model/models", "third_party/provider.patch", "scripts/deployment_check.py"] {
        assert!(closure.watches.contains(&name.into()), "missing watch {name}");
    }
    assert!(!closure.watches.iter().any(|path| path.starts_with("crates/serving") || path.starts_with("crates/cpg-core")));
    assert!(closure.watches.contains(&"third_party".into()), "new patch membership must trigger Cargo");
    assert!(!closure.watches.iter().any(|path| path == "crates/cpg-extract"));
    assert!(!closure.watches.iter().any(|path| path.contains("/tests") || path.contains("administration.py")));
    for name in ["crates/cpg-core/src/lib.rs", "crates/native/src/lib.rs", "crates/serving/src/lib.rs",
        "crates/cpg-extract/tests/control.rs", "scripts/administration.py",
        "scripts/__pycache__/deployment_check.cpython-314.pyc", "third_party/unrelated/src/lib.rs"] {
        write(second.path(), name, "unrelated edit");
        assert_eq!(original, digest(second.path(), "cpg-extract"), "unrelated {name}");
    }
}

#[test]
fn provider_and_compiler_capture_their_own_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fixture(root);
    let provider = digest(root, "cpg-extract");
    let compiler = digest(root, "cpg-core");
    assert_ne!(provider, compiler);
    for name in ["crates/cpg-core/src/lib.rs", "crates/native/src/lib.rs", "crates/native/sql/database.sql", "crates/native/migrations/migration.sql"] {
        let previous = std::fs::read(root.join(name)).unwrap();
        write(root, name, "changed native ingestion");
        assert_eq!(provider, digest(root, "cpg-extract"), "provider changed for {name}");
        assert_ne!(compiler, digest(root, "cpg-core"), "compiler omitted {name}");
        std::fs::write(root.join(name), previous).unwrap();
    }
    // Every captured byte affects identity, including manifests, inherited paths, model assets,
    // analyzer patches, scripts, custom build/targets, and conservatively inactive dependencies.
    for name in capture(root, "cpg-extract").files {
        let path = root.join(&name);
        let previous = std::fs::read(&path).unwrap();
        let mut changed = previous.clone();
        changed.extend_from_slice(b"\n# relevant mutation\n");
        std::fs::write(&path, changed).unwrap();
        assert_ne!(provider, digest(root, "cpg-extract"), "provider omitted {name}");
        assert_ne!(compiler, digest(root, "cpg-core"), "compiler omitted {name}");
        std::fs::write(path, previous).unwrap();
    }
}

#[test]
fn fingerprint_tracks_membership_deletion_and_runtime_declarations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fixture(root);
    let provider = digest(root, "cpg-extract");
    for name in ["crates/cpg-extract/src/new.rs", "crates/lctx-model/models/new.toml", "third_party/new.patch"] {
        write(root, name, "new production member");
        assert_ne!(provider, digest(root, "cpg-extract"), "added {name}");
        std::fs::remove_file(root.join(name)).unwrap();
        assert_eq!(provider, digest(root, "cpg-extract"));
    }
    for name in ["crates/cpg-flow/src/lib.rs", "crates/lctx-model/models/external.toml", "third_party/provider.patch"] {
        let previous = std::fs::read(root.join(name)).unwrap();
        std::fs::remove_file(root.join(name)).unwrap();
        assert_ne!(provider, digest(root, "cpg-extract"), "deleted {name}");
        std::fs::write(root.join(name), previous).unwrap();
    }
    write(root, "scripts/second_runtime.py", "new declared runtime");
    assert_eq!(provider, digest(root, "cpg-extract"));
    let expanded = fingerprint::capture_with_scripts(root, &root.join("crates/cpg-extract"),
        &["scripts/deployment_check.py", "scripts/second_runtime.py"]);
    assert_ne!(provider, fingerprint::digest(root, &expanded.files));
    assert!(expanded.watches.contains(&"scripts/second_runtime.py".into()));
}

#[test]
fn repository_provider_closure_has_only_its_local_production_members() {
    let owner = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = owner.join("../..").canonicalize().unwrap();
    let provider = fingerprint::capture(&root, owner);
    let manifests: Vec<_> = provider.files.iter().filter(|name| {
        name.starts_with("crates/") && name.ends_with("/Cargo.toml")
    }).map(String::as_str).collect();
    assert_eq!(manifests, [
        "crates/cpg-extract/Cargo.toml", "crates/cpg-flow/Cargo.toml",
        "crates/lctx-model-macros/Cargo.toml", "crates/lctx-model/Cargo.toml",
    ]);
    let compiler = fingerprint::capture(&root, &root.join("crates/cpg-core"));
    for name in ["crates/cpg-core/src/native_bridge.rs", "crates/cpg-core/src/workspace.rs", "crates/lctx-surrealdb/src/compiler.rs"] {
        assert!(compiler.files.contains(&name.into()), "compiler omitted {name}");
        assert!(!provider.files.contains(&name.into()), "provider included {name}");
    }
    for name in ["crates/lctx-publisher", "crates/lctx-serving", "crates/lctx-analytics", "crates/lctx"] {
        assert!(!compiler.files.iter().any(|path| path.starts_with(&format!("{name}/"))), "compiler included {name}");
        assert!(!provider.watches.iter().any(|path| path == name || path.starts_with(&format!("{name}/"))), "provider watched {name}");
    }
    for name in ["third_party/pyrefly-1.4.0-dev.3.patch", "third_party/ruff-0.16.10.patch", "third_party/allocative/src/lib.rs", "crates/lctx-model/models/external.toml", "specs/embedding/qwen3-embedding-8b.json", "scripts/deployment_check.py"] {
        assert!(provider.files.contains(&name.into()), "provider omitted {name}");
    }
}
