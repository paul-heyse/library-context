#[path = "../../scripts/producer_fingerprint.rs"]
mod producer;
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace");
    let files = producer::files(&root);
    println!(
        "cargo:rustc-env=LCTX_PRODUCER_SOURCE_DIGEST={}",
        producer::digest(&root, &files).to_hex()
    );
}
