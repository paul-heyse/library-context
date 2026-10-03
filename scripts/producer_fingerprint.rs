//! Shared build-time source closure. Paths are workspace-relative; machine paths never enter IDs.
use std::path::Path;
#[path = "../crates/cpg-extract/src/runtime_scripts.rs"]
mod runtime_scripts;
pub fn files(root: &Path) -> Vec<String> {
    macro_rules! script_paths {
        ($($name:ident => $path:literal,)*) => { &[$($path),*] };
    }
    files_with_scripts(root, runtime_scripts::runtime_scripts!(script_paths))
}
/// Explicit paths are also used by mutation controls; runtime embedding uses the same declaration.
pub fn files_with_scripts(root: &Path, scripts: &[&str]) -> Vec<String> {
    fn walk(root: &Path, path: &Path, out: &mut Vec<String>) {
        if path.file_name().is_some_and(|name| name == "__pycache__")
            || path.extension().is_some_and(|ext| ext == "pyc")
        {
            return;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .expect("source directory")
                .map(|e| e.expect("entry").path())
                .collect();
            entries.sort();
            for entry in entries {
                walk(root, &entry, out);
            }
        } else {
            out.push(
                path.strip_prefix(root)
                    .expect("workspace path")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).expect("workspace crates") {
        let path = entry.expect("crate").path();
        if path.join("Cargo.toml").is_file() {
            walk(root, &path.join("Cargo.toml"), &mut files);
            for directory in ["src", "sql", "migrations", "models"] {
                if path.join(directory).is_dir() {
                    walk(root, &path.join(directory), &mut files);
                }
            }
            if path.join("build.rs").is_file() {
                walk(root, &path.join("build.rs"), &mut files);
            }
        }
    }
    // Keep all non-script roots conservative, including unused siblings and input membership.
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "third_party",
        "specs",
        "scripts/producer_fingerprint.rs",
        "crates/cpg-extract/src/runtime_scripts.rs",
    ] {
        if root.join(name).exists() {
            walk(root, &root.join(name), &mut files);
        }
    }
    for script in scripts {
        walk(root, &root.join(script), &mut files);
    }
    files.sort();
    files.dedup();
    files
}
pub fn digest(root: &Path, files: &[String]) -> blake3::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(b"lctx-production-closure-v1");
    for file in files {
        let bytes = std::fs::read(root.join(file)).expect("production source");
        hash.update(&(file.len() as u64).to_le_bytes());
        hash.update(file.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    hash.finalize()
}
