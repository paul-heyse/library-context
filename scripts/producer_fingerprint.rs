//! Build-time production closure rooted at its owning crate. Cargo retains resolution authority.
//! Paths in captures are workspace-relative; machine paths never enter implementation identities.
use std::{
    collections::BTreeSet,
    path::Path,
};
#[path = "../crates/cpg-extract/src/runtime_scripts.rs"]
mod runtime_scripts;

/// Files determine identity; directory watches also detect membership changes and deletions.
pub struct SourceClosure {
    pub files: Vec<String>,
    pub watches: Vec<String>,
}

pub fn capture(root: &Path, owner: &Path) -> SourceClosure {
    macro_rules! script_paths {
        ($($name:ident => $path:literal,)*) => { &[$($path),*] };
    }
    capture_with_scripts(root, owner, runtime_scripts::runtime_scripts!(script_paths))
}

/// Explicit runtime paths let controls exercise the same declaration-driven capture as embedding.
pub fn capture_with_scripts(root: &Path, owner: &Path, scripts: &[&str]) -> SourceClosure {
    let root = root.canonicalize().expect("fingerprint workspace root");
    let owner = owner.canonicalize().expect("fingerprint owner root");
    let workspace = manifest(&root.join("Cargo.toml"));
    let mut closure = Collector {
        root: &root,
        workspace: &workspace,
        members: BTreeSet::new(),
        files: BTreeSet::new(),
        watches: BTreeSet::new(),
    };
    closure.package(&owner);
    // Local patches may affect transitive external dependencies. Keep these conservative rather
    // than reconstructing Cargo's feature/version resolution or invoking Cargo from its build.
    if let Some(patches) = workspace.get("patch").and_then(toml::Value::as_table) {
        for registry in patches.values().filter_map(toml::Value::as_table) {
            for dependency in registry.values() {
                if let Some(path) = dependency.get("path").and_then(toml::Value::as_str) {
                    closure.package(&root.join(path));
                }
            }
        }
    }
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "scripts/producer_fingerprint.rs"] {
        closure.walk(&root.join(name));
    }
    // Patch bytes are small conservative inputs; do not walk unrelated vendored package trees.
    let third_party = root.join("third_party");
    if third_party.is_dir() {
        // Cargo cannot watch a glob of immediate patch files. A containing-directory watch
        // detects newly added patches; identity still excludes unrelated vendored bytes.
        // This deliberately conservative watch may rerun capture on other vendor edits.
        closure.watches.insert(closure.relative(&third_party));
        for entry in std::fs::read_dir(&third_party).expect("third-party patches") {
            let path = entry.expect("patch entry").path();
            if path.extension().is_some_and(|extension| extension == "patch") {
                closure.walk(&path);
            }
        }
    }
    if closure.members.contains("crates/cpg-extract") {
        // The declaration itself is already in the extraction source tree. Its assets are reused,
        // not independently maintained in another important-files inventory.
        for script in scripts {
            closure.walk(&root.join(script));
        }
    }
    if closure.members.contains("crates/lctx-model") && root.join("specs").exists() {
        closure.walk(&root.join("specs"));
    }
    SourceClosure {
        files: closure.files.into_iter().collect(),
        watches: closure.watches.into_iter().collect(),
    }
}

fn manifest(path: &Path) -> toml::Value {
    let bytes = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("fingerprint manifest {}: {error}", path.display()));
    toml::from_str(&bytes)
        .unwrap_or_else(|error| panic!("fingerprint manifest {}: {error}", path.display()))
}

struct Collector<'a> {
    root: &'a Path,
    workspace: &'a toml::Value,
    members: BTreeSet<String>,
    files: BTreeSet<String>,
    watches: BTreeSet<String>,
}
impl Collector<'_> {
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(self.root)
            .unwrap_or_else(|_| panic!("fingerprint input outside workspace: {}", path.display()))
            .to_str().expect("UTF-8 fingerprint path").replace('\\', "/")
    }
    fn walk(&mut self, path: &Path) {
        if path.file_name().is_some_and(|name| name == "__pycache__")
            || path.extension().is_some_and(|extension| extension == "pyc") {
            return;
        }
        let path = path.canonicalize()
            .unwrap_or_else(|error| panic!("fingerprint input {}: {error}", path.display()));
        let relative = self.relative(&path);
        if !self.watches.insert(relative.clone()) {
            return;
        }
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).expect("production source directory") {
                self.walk(&entry.expect("production source entry").path());
            }
        } else {
            self.files.insert(relative);
        }
    }
    fn package(&mut self, path: &Path) {
        let path = path.canonicalize().expect("local production dependency root");
        if !self.members.insert(self.relative(&path)) {
            return;
        }
        let declaration = manifest(&path.join("Cargo.toml"));
        // Watch production directories and files, never the whole package (which would make
        // Cargo recursively watch dev/test roots). New build/target/asset roots become relevant
        // through changes to these captured manifests or sources.
        self.walk(&path.join("Cargo.toml"));
        // Build scripts and root-level Rust modules can live beside the manifest.
        for entry in std::fs::read_dir(&path).expect("production package directory") {
            let source = entry.expect("production package entry").path();
            if source.is_file() && source.extension().is_some_and(|extension| extension == "rs") {
                self.walk(&source);
            }
        }
        for directory in ["src", "sql", "migrations", "models"] {
            if path.join(directory).exists() {
                self.walk(&path.join(directory));
            }
        }
        if let Some(build) = declaration.get("package").and_then(|package| package.get("build"))
            .and_then(toml::Value::as_str) {
            self.source(&path, build);
        } else if path.join("build.rs").exists() {
            self.walk(&path.join("build.rs"));
        }
        if let Some(lib) = declaration.get("lib") {
            self.target_source(&path, lib);
        }
        if let Some(binaries) = declaration.get("bin").and_then(toml::Value::as_array) {
            for binary in binaries {
                self.target_source(&path, binary);
            }
        }
        self.dependencies(&path, &declaration);
        if let Some(targets) = declaration.get("target").and_then(toml::Value::as_table) {
            for target in targets.values() {
                self.dependencies(&path, target);
            }
        }
    }
    fn target_source(&mut self, root: &Path, target: &toml::Value) {
        if let Some(source) = target.get("path").and_then(toml::Value::as_str) {
            self.source(root, source);
        }
    }
    fn source(&mut self, root: &Path, source: &str) {
        let source = root.join(source);
        // Custom build/target directories can contain adjacent modules and embedded assets.
        let parent = source.parent().expect("source parent").canonicalize().expect("source parent");
        if parent != root {
            self.walk(&parent);
        } else {
            self.walk(&source);
        }
    }
    fn dependencies(&mut self, root: &Path, declaration: &toml::Value) {
        for kind in ["dependencies", "build-dependencies"] {
            if let Some(dependencies) = declaration.get(kind).and_then(toml::Value::as_table) {
                for (name, dependency) in dependencies {
                    let (base, dependency) = if dependency.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                        let inherited = self.workspace.get("workspace")
                            .and_then(|workspace| workspace.get("dependencies"))
                            .and_then(|dependencies| dependencies.get(name))
                            .unwrap_or_else(|| panic!("missing inherited workspace dependency {name}"));
                        (self.root, inherited)
                    } else {
                        (root, dependency)
                    };
                    if let Some(path) = dependency.get("path").and_then(toml::Value::as_str) {
                        self.package(&base.join(path));
                    }
                }
            }
        }
    }
}

pub fn digest(root: &Path, files: &[String]) -> blake3::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(b"lctx-rooted-production-closure-v2");
    for file in files {
        let bytes = std::fs::read(root.join(file)).expect("production source");
        hash.update(&(file.len() as u64).to_le_bytes());
        hash.update(file.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    hash.finalize()
}
