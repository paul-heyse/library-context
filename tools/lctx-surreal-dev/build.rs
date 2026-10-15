use std::path::{Path, PathBuf};

fn input(path: &Path) -> String {
    println!("cargo:rerun-if-changed={}", path.display());
    std::fs::read_to_string(path).expect("read worker build input")
}

fn main() {
    // include_str! tracks edits; the directory tracks additions and removals too.
    println!("cargo:rerun-if-changed=schema");
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let lock: toml::Value = toml::from_str(&input(&root.join("Cargo.lock"))).unwrap();
    let workspace: toml::Value = toml::from_str(&input(&root.join("Cargo.toml"))).unwrap();
    let mut dependencies = serde_json::Map::new();
    for name in ["surrealkit", "surrealdb", "surrealdb-rpc"] {
        let packages: Vec<_> = lock["package"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|package| package["name"].as_str() == Some(name))
            .collect();
        assert_eq!(
            packages.len(),
            1,
            "worker dependency family must be unambiguous: {name}"
        );
        let package = packages[0];
        let patch = workspace["patch"]["crates-io"][name]["path"]
            .as_str()
            .unwrap();
        let manifest = input(&root.join(patch).join("Cargo.toml"));
        let documentation = input(&root.join(patch).join("README.lctx.md"));
        let declared: toml::Value = toml::from_str(&manifest).unwrap();
        assert_eq!(
            declared["package"]["version"].as_str(),
            package["version"].as_str()
        );
        assert!(
            package.get("source").is_none(),
            "expected selected workspace patch"
        );
        dependencies.insert(name.into(), serde_json::json!({
            "version": package["version"].as_str().unwrap(),
            "source": "compiled-resolved-lock-and-workspace-patch",
            "patch_profile": {
                "status": "declared_build_input",
                "manifest": format!("{patch}/Cargo.toml"),
                "manifest_blake3": blake3::hash(manifest.as_bytes()).to_hex().to_string(),
                "documentation": format!("{patch}/README.lctx.md"),
                "documentation_blake3": blake3::hash(documentation.as_bytes()).to_hex().to_string(),
            },
        }));
    }
    std::fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("build-info.json"),
        serde_json::to_vec(&dependencies).unwrap(),
    )
    .unwrap();
}
