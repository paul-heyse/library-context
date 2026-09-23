//! The locked versions of the libraries the analyses run (petgraph, fixedbitset; later
//! leiden-rs and its RNG), read from the workspace `Cargo.lock`, so each invocation's record and
//! the compiler digest follow what is built rather than a hand-kept string (guidelines §8).

const LIBRARIES: &[&str] = &["fixedbitset", "petgraph"];

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
            LIBRARIES
                .contains(&name.as_str())
                .then(|| format!("{name} {}", field("version").unwrap_or_default()))
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
