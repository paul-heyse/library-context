//! ADR-0013 review F3: `lctx acquire` runs uv hermetically; C5: it fetches the declared source tree
//! hermetically too. Stub `uv` and `git` first on `PATH` record the arguments and environment
//! they receive.

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

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A stub `git`: logs each call's arguments and environment, makes `init` create `.git`, and
/// answers `rev-parse HEAD` with the pinned commit.
fn stub_git(dir: &Path) {
    let git = dir.join("git");
    std::fs::write(
        &git,
        format!(
            "#!/bin/sh
printf '%s ' \"$@\" >> \"$STUB_OUT/git-args\"\necho >> \"$STUB_OUT/git-args\"\n\
             env > \"$STUB_OUT/git-env\"\n\
             case \"$1\" in init) mkdir -p .git ;; rev-parse) echo {COMMIT} ;; esac\n"
        ),
    )
    .unwrap();
    let chmod = Command::new("chmod").arg("+x").arg(&git).status().unwrap();
    assert!(chmod.success());
}

fn acquire(extra: &[&str]) -> (Vec<String>, String, std::path::PathBuf) {
    acquire_with(extra, "")
}

fn acquire_with(extra: &[&str], source: &str) -> (Vec<String>, String, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    stub(&root);
    stub_git(&root);
    let library = root.join("libraries/demo");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(
        library.join("pyproject.toml"),
        format!(
            "[project]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
             dependencies = [\"demo==1.0\"]\n\n[tool.lctx]\nrelease = [\"demo\"]\n{source}"
        ),
    )
    .unwrap();
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
        .arg("--sources")
        .arg(root.join("sources"))
        .args(extra)
        .env("PATH", path)
        .env("STUB_OUT", &root)
        .env("UV_INDEX_URL", "https://example.invalid/simple")
        .env("UV_CONFIG_FILE", "/tmp/elsewhere.toml")
        .env("VIRTUAL_ENV", "/tmp/some-venv")
        .env("GIT_DIR", "/tmp/elsewhere.git")
        .env("GIT_CONFIG_PARAMETERS", "'url.x.insteadOf'='y'")
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

#[test]
fn a_declared_source_is_fetched_hermetically_at_its_commit() {
    let source = format!(
        "\n[tool.lctx.source]\nrepository = \"https://example.invalid/demo\"\ntag = \"v1.0\"\n\
         commit = \"{COMMIT}\"\ndocuments = [\"docs/**/*.md\"]\n"
    );
    let (_, _, envs) = acquire_with(&[], &source);
    let root = envs.parent().and_then(Path::parent).unwrap().to_path_buf();
    let calls = std::fs::read_to_string(root.join("git-args")).unwrap();
    let calls: Vec<&str> = calls.lines().map(str::trim_end).collect();
    assert_eq!(
        calls,
        [
            "init -q",
            &format!("fetch -q --depth 1 https://example.invalid/demo {COMMIT}"),
            "-c advice.detachedHead=false checkout -q FETCH_HEAD",
            "rev-parse HEAD",
        ]
    );
    assert!(root.join("sources/demo").join(COMMIT).join(".git").is_dir());
    let env = std::fs::read_to_string(root.join("git-env")).unwrap();
    for ambient in ["GIT_DIR", "GIT_CONFIG_PARAMETERS"] {
        assert!(
            !env.lines().any(|l| l.starts_with(&format!("{ambient}="))),
            "{ambient} reached git"
        );
    }
    for set in [
        "GIT_CONFIG_NOSYSTEM=1",
        "GIT_CONFIG_GLOBAL=/dev/null",
        "GIT_TERMINAL_PROMPT=0",
    ] {
        assert!(env.lines().any(|l| l == set), "{set} is not set");
    }
}
