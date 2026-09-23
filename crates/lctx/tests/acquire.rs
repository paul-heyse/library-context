//! ADR-0013 review F3: `lctx acquire` runs uv hermetically. A stub `uv` first on `PATH` records
//! the arguments and environment it receives.

use std::path::Path;
use std::process::Command;

fn stub(dir: &Path) {
    let uv = dir.join("uv");
    std::fs::write(
        &uv,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$STUB_OUT/args\"\nenv > \"$STUB_OUT/env\"\n",
    )
    .unwrap();
    let chmod = Command::new("chmod").arg("+x").arg(&uv).status().unwrap();
    assert!(chmod.success());
}

fn acquire(extra: &[&str]) -> (Vec<String>, String, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    stub(&root);
    let library = root.join("libraries/demo");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("uv.lock"), "version = 1\n").unwrap();
    std::fs::write(library.join(".python-version"), "3.14.7\n").unwrap();
    let path = format!(
        "{}:{}",
        root.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let status = Command::new(env!("CARGO_BIN_EXE_lctx"))
        .args(["acquire", "demo", "--libraries"])
        .arg(root.join("libraries"))
        .arg("--envs")
        .arg(root.join("envs"))
        .args(extra)
        .env("PATH", path)
        .env("STUB_OUT", &root)
        .env("UV_INDEX_URL", "https://example.invalid/simple")
        .env("UV_CONFIG_FILE", "/tmp/elsewhere.toml")
        .env("VIRTUAL_ENV", "/tmp/some-venv")
        .status()
        .unwrap();
    assert!(status.success());
    let args = std::fs::read_to_string(root.join("args"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    let env = std::fs::read_to_string(root.join("env")).unwrap();
    // Keep the directory alive by leaking its path for the assertions below.
    let envs = root.join("envs/demo");
    std::mem::forget(dir);
    (args, env, envs)
}

#[test]
fn acquire_is_frozen_config_free_pinned_and_copying() {
    let (args, env, envs) = acquire(&[]);
    let library = envs
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("libraries/demo");
    assert_eq!(
        args,
        [
            "sync",
            "--project",
            &library.to_string_lossy(),
            "--frozen",
            "--no-install-project",
            "--no-config",
            "--python",
            "3.14.7",
            "--link-mode",
            "copy",
        ]
    );
    for ambient in ["UV_INDEX_URL", "UV_CONFIG_FILE", "VIRTUAL_ENV"] {
        assert!(
            !env.lines().any(|l| l.starts_with(&format!("{ambient}="))),
            "{ambient} reached uv"
        );
    }
    assert!(
        env.lines()
            .any(|l| l == format!("UV_PROJECT_ENVIRONMENT={}", envs.display())),
        "the environment path is absolute and explicit"
    );
}

#[test]
fn reinstall_rebuilds_every_package() {
    let (args, _, _) = acquire(&["--reinstall"]);
    assert_eq!(args.last().map(String::as_str), Some("--reinstall"));
}
