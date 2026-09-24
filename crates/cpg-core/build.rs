//! The locked identity of the engines the compiler's output depends on (DataFusion, Arrow,
//! Parquet, object_store, delta-rs and its kernel), read from the workspace `Cargo.lock`, so the
//! compiler digest follows what is built rather than a hand-kept string (slice-2 review F4).
//!
//! And the digest of the compiler's own sources (the holistic assessment's A2(e)): every `.rs`
//! file of `cpg-core` (Stages D–F, publication), `lctx-analytics` and `cpg-schema`, so a code
//! change that no version bump names still moves the run and producer ids. `TEMPLATE_VERSION`
//! stays the published lineage, and the analysis ledger stays the alarm for output changes.

/// The source trees whose code decides the compiler's output, relative to the workspace root.
const SOURCES: &[&str] = &[
    "crates/cpg-core/src",
    "crates/lctx-analytics/src",
    "crates/cpg-schema/src",
];

/// Every `.rs` file under `dir`, recursively, as workspace-relative paths.
fn rust_files(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("a source directory")
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            println!("cargo:rerun-if-changed={}", path.display());
            let relative = path.strip_prefix(root).expect("under the workspace");
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

const ENGINES: &[&str] = &[
    "arrow-array",
    "buoyant_kernel",
    "datafusion",
    "deltalake-core",
    "object_store",
    "parquet",
];

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let lock = std::path::Path::new(&dir).join("../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(&lock).expect("the workspace Cargo.lock");
    let mut found: Vec<String> = text
        .split("[[package]]")
        .filter_map(|block| {
            let field = |key: &str| {
                block.lines().find_map(|line| {
                    line.strip_prefix(key)
                        .and_then(|rest| rest.strip_prefix(" = \""))
                        .map(|v| v.trim_end_matches('"').to_owned())
                })
            };
            let name = field("name")?;
            ENGINES.contains(&name.as_str()).then(|| {
                format!(
                    "{name} {} {}",
                    field("version").unwrap_or_default(),
                    field("source").unwrap_or_default()
                )
            })
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        ENGINES.len(),
        "each engine is locked exactly once: {found:?}"
    );
    println!("cargo:rustc-env=LCTX_ENGINES={}", found.join("; "));

    let root = std::path::Path::new(&dir)
        .join("../..")
        .canonicalize()
        .expect("the workspace");
    let mut files = Vec::new();
    for tree in SOURCES {
        rust_files(&root, &root.join(tree), &mut files);
    }
    files.sort();
    let mut h = blake3::Hasher::new();
    for file in &files {
        let text = std::fs::read(root.join(file)).expect("a source file");
        h.update(file.as_bytes());
        h.update(&(text.len() as u64).to_le_bytes());
        h.update(&text);
    }
    println!(
        "cargo:rustc-env=LCTX_SOURCE_DIGEST={}",
        h.finalize().to_hex()
    );
    println!("cargo:rustc-env=LCTX_SOURCE_FILES={}", files.join(";"));
}
