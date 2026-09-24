//! The locked identity of the engines the compiler's output depends on (DataFusion, Arrow,
//! Parquet, object_store, delta-rs and its kernel), read from the workspace `Cargo.lock`, so the
//! compiler digest follows what is built rather than a hand-kept string (slice-2 review F4).

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
}
