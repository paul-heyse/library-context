#[path = "../../scripts/producer_fingerprint.rs"]
mod production;
fn main() {
    let owner = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = owner.join("../..").canonicalize().expect("workspace");
    let closure = production::capture(&root, owner);
    for path in &closure.watches {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    println!(
        "cargo:rustc-env=LCTX_NATIVE_SOURCE_DIGEST={}",
        production::digest(&root, &closure.files).to_hex()
    );
}
