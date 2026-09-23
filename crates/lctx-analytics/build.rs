//! The locked versions of the libraries the analyses run (petgraph, fixedbitset, leiden-rs and
//! its RNG), read from the workspace `Cargo.lock`, so each invocation's record and the compiler
//! digest follow what is built rather than a hand-kept string (guidelines §8).

/// Each library by name and version prefix: the lock holds several `rand` majors, and leiden-rs
/// draws from the 0.9 line (rand does not promise sequences across versions; ADR-0011).
const LIBRARIES: &[(&str, &str)] = &[
    ("fixedbitset", ""),
    ("leiden-rs", ""),
    ("petgraph", ""),
    ("rand", "0.9."),
    ("rand_chacha", "0.9."),
    ("rand_core", "0.9."),
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
            let version = field("version").unwrap_or_default();
            LIBRARIES
                .iter()
                .any(|(n, prefix)| *n == name && version.starts_with(prefix))
                .then(|| format!("{name} {version}"))
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        LIBRARIES.len(),
        "each analysis library is locked exactly once: {found:?}"
    );
    println!(
        "cargo:rustc-env=LCTX_ANALYTICS_LIBRARIES={}",
        found.join("; ")
    );
}
