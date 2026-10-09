#[path = "../../scripts/producer_fingerprint.rs"]
mod producer;
fn main() {
    let owner = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = owner.join("../..").canonicalize().expect("workspace");
    let closure = producer::capture(&root, owner);
    for path in &closure.watches {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    println!(
        "cargo:rustc-env=LCTX_PRODUCER_SOURCE_DIGEST={}",
        producer::digest(&root, &closure.files).to_hex()
    );
}
