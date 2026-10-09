#[path = "../../scripts/producer_fingerprint.rs"]
mod compiler;
fn main() {
    let owner = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = owner.join("../..").canonicalize().expect("workspace");
    let closure = compiler::capture(&root, owner);
    for path in &closure.watches {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    println!(
        "cargo:rustc-env=LCTX_COMPILER_SOURCE_DIGEST={}",
        compiler::digest(&root, &closure.files).to_hex()
    );
}
