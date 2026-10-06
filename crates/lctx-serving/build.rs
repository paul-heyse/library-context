//! Capture executable provenance independently of semantic compatibility.
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

fn sources(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            sources(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn frame(content: &mut Vec<u8>, value: &[u8]) {
    content.extend_from_slice(&(value.len() as u64).to_le_bytes());
    content.extend_from_slice(value);
}

pub(crate) fn capture(
    root: &Path,
    configuration: &BTreeMap<String, String>,
) -> io::Result<Vec<u8>> {
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files)?;
    files.extend(
        [
            "build.rs",
            "Cargo.toml",
            "../../Cargo.toml",
            "../../Cargo.lock",
        ]
        .into_iter()
        .map(|name| root.join(name)),
    );
    files.sort();
    let mut content = Vec::new();
    frame(&mut content, b"serving-executable-sources/v1");
    for file in files {
        let relative = file.strip_prefix(root).expect("capture-relative source");
        frame(
            &mut content,
            relative.to_str().expect("UTF-8 source path").as_bytes(),
        );
        frame(&mut content, &fs::read(file)?);
    }
    frame(&mut content, b"build-configuration/v1");
    for (name, value) in configuration {
        frame(&mut content, name.as_bytes());
        frame(&mut content, value.as_bytes());
    }
    Ok(content)
}

#[allow(
    dead_code,
    reason = "The capture is also included by its focused integration controls"
)]
fn main() {
    use std::env;
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // Watching the directory also invalidates additions and deletions, including new helpers.
    for path in [
        "src",
        "build.rs",
        "Cargo.toml",
        "../../Cargo.toml",
        "../../Cargo.lock",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    let configuration = env::vars()
        .filter(|(name, _)| {
            name.starts_with("CARGO_FEATURE_")
                || name.starts_with("CARGO_CFG_")
                || name == "CARGO_ENCODED_RUSTFLAGS"
        })
        .collect();
    let content = capture(&root, &configuration).expect("serving executable capture");
    fs::write(
        Path::new(&env::var_os("OUT_DIR").expect("OUT_DIR")).join("serving-implementation.bin"),
        content,
    )
    .expect("serving executable capture output");
}
