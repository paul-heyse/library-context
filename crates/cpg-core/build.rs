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
    "crates/lctx-model/src",
    "crates/cpg-core/src",
    "crates/lctx-analytics/src",
    "crates/cpg-schema/src",
    "crates/lctx-postgres/src/cache.rs",
];

/// Every `.rs` or `.sql` file under `dir`, recursively, as workspace-relative paths.
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
        } else if path.extension().is_some_and(|e| e == "rs" || e == "sql") {
            println!("cargo:rerun-if-changed={}", path.display());
            let relative = path.strip_prefix(root).expect("under the workspace");
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

const ENGINES: &[&str] = &[
    "arrow-array",
    "biodivine-lib-bdd",
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
        let source = root.join(tree);
        if source.is_file() {
            println!("cargo:rerun-if-changed={}", source.display());
            files.push((*tree).to_owned());
        } else {
            rust_files(&root, &source, &mut files);
        }
    }
    files.sort();
    let mut h = blake3::Hasher::new();
    for file in &files {
        let text = std::fs::read(root.join(file)).expect("a source file");
        h.update(file.as_bytes());
        h.update(&(text.len() as u64).to_le_bytes());
        h.update(&text);
    }
    // Canonical producer identity excludes serving/retrieval-only realization code. Shared
    // schema, semantic queries, models and validators are deliberately included conservatively.
    let mut semantic = blake3::Hasher::new();
    for file in &files {
        if matches!(
            file.as_str(),
            "crates/cpg-schema/src/wire/journeys.rs"
                | "crates/cpg-schema/src/wire/responses.rs"
                | "crates/cpg-schema/src/wire/dispatch.rs"
        ) || file == "crates/cpg-schema/src/retrieval.rs"
            || file == "crates/cpg-core/src/evidence.rs"
            || file == "crates/cpg-core/src/catalog_domains.rs"
            || file == "crates/cpg-core/src/stage_cache.rs"
            || file.starts_with("crates/cpg-core/src/bundle")
            || file.starts_with("crates/cpg-core/src/retrieval")
            || file.starts_with("crates/cpg-core/src/rebuild")
        {
            continue;
        }
        let bytes = std::fs::read(root.join(file)).expect("semantic source");
        semantic.update(file.as_bytes());
        semantic.update(&(bytes.len() as u64).to_le_bytes());
        semantic.update(&bytes);
    }
    semantic.update(found.join(";").as_bytes());
    // Producer identity (review F11): every canonical producer's code, association and evidence
    // included. Only serving-only realization code is outside it.
    let mut producer = blake3::Hasher::new();
    let mut excluded = Vec::new();
    for file in &files {
        if serving_only(file) {
            excluded.push(file.clone());
            continue;
        }
        let bytes = std::fs::read(root.join(file)).expect("producer source");
        producer.update(file.as_bytes());
        producer.update(&(bytes.len() as u64).to_le_bytes());
        producer.update(&bytes);
    }
    producer.update(found.join(";").as_bytes());
    println!(
        "cargo:rustc-env=LCTX_PRODUCER_SOURCE_DIGEST={}",
        producer.finalize().to_hex()
    );
    println!("cargo:rustc-env=LCTX_PRODUCER_EXCLUDED={}", excluded.join(";"));
    let mut association = blake3::Hasher::new();
    for file in [
        "crates/cpg-core/src/evidence.rs",
        "crates/cpg-core/src/catalog_domains.rs",
    ] {
        let bytes = std::fs::read(root.join(file)).expect("association source");
        association.update(file.as_bytes());
        association.update(&(bytes.len() as u64).to_le_bytes());
        association.update(&bytes);
    }
    println!(
        "cargo:rustc-env=LCTX_ASSOCIATION_SOURCE_DIGEST={}",
        association.finalize().to_hex()
    );
    println!(
        "cargo:rustc-env=LCTX_SEMANTIC_SOURCE_DIGEST={}",
        semantic.finalize().to_hex()
    );
    println!(
        "cargo:rustc-env=LCTX_SOURCE_DIGEST={}",
        h.finalize().to_hex()
    );
    println!("cargo:rustc-env=LCTX_SOURCE_FILES={}", files.join(";"));
}

/// Code that realizes serving artifacts from a published generation and never produces a
/// canonical relation.
fn serving_only(file: &str) -> bool {
    matches!(
        file,
        "crates/cpg-schema/src/wire/journeys.rs"
            | "crates/cpg-schema/src/wire/responses.rs"
            | "crates/cpg-schema/src/wire/dispatch.rs"
            | "crates/cpg-schema/src/retrieval.rs"
    ) || file.starts_with("crates/cpg-core/src/bundle")
        || file.starts_with("crates/cpg-core/src/retrieval")
}
