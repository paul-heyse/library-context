//! Every owned semantic definition and its generator participates in the model contract.
use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn sources(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .expect("model sources")
        .map(|e| e.expect("source entry").path())
    {
        if entry.is_dir() {
            sources(&entry, files);
        } else if entry.extension().is_some_and(|ext| ext == "rs") {
            files.push(entry);
        }
    }
}
fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../lctx-model-macros/src");
    let mut files = Vec::new();
    sources(Path::new("src"), &mut files);
    sources(Path::new("../lctx-model-macros/src"), &mut files);
    files.extend([
        PathBuf::from("Cargo.toml"),
        PathBuf::from("../../Cargo.toml"),
        PathBuf::from("../../Cargo.lock"),
        PathBuf::from("../lctx-model-macros/Cargo.toml"),
    ]);
    files.sort();
    let mut content = Vec::new();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
        let name = file.to_str().expect("source path").as_bytes();
        let bytes = fs::read(&file).expect("source content");
        content.extend_from_slice(&(name.len() as u64).to_le_bytes());
        content.extend_from_slice(name);
        content.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        content.extend_from_slice(&bytes);
    }
    fs::write(
        Path::new(&env::var_os("OUT_DIR").expect("OUT_DIR")).join("semantic-contract.bin"),
        content,
    )
    .expect("contract capture");
}
