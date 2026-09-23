//! Stage A (DESIGN §4.0, ADR-0013) over a synthetic acquired environment: a library definition,
//! its `uv.lock`, and the environment `uv sync --frozen` would build, with real `RECORD` hashes.

mod common;

use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use common::cell;
use cpg_extract::{ExtractError, extract, library};
use cpg_schema::id::Id;
use sha2::{Digest as _, Sha256};

const DEMO: &str = "from dep import helper\n\n\ndef api(x):\n    return helper(x)\n";
const DEP: &str = "def helper(x: int) -> int:\n    return x\n";

fn lock(demo_version: &str, demo_hash: &str) -> String {
    format!(
        "version = 1\nrevision = 3\nrequires-python = \"==3.14.*\"\n\n\
         [[package]]\nname = \"demo\"\nversion = \"{demo_version}\"\n\
         source = {{ registry = \"https://pypi.org/simple\" }}\n\
         dependencies = [{{ name = \"dep\" }}]\n\
         wheels = [{{ url = \"https://x/demo.whl\", hash = \"sha256:{demo_hash}\", size = 1 }}]\n\n\
         [[package]]\nname = \"dep\"\nversion = \"2.0\"\n\
         source = {{ registry = \"https://pypi.org/simple\" }}\n\
         wheels = [{{ url = \"https://x/dep.whl\", hash = \"sha256:bbbb\", size = 1 }}]\n\n\
         [[package]]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         source = {{ virtual = \".\" }}\ndependencies = [{{ name = \"demo\" }}]\n"
    )
}

fn sha(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

/// Install `package` as distribution `dist` `version`: its module and a `RECORD`.
fn install(site: &Path, dist: &str, version: &str, package: &str, source: &str) {
    std::fs::create_dir_all(site.join(package)).unwrap();
    std::fs::write(site.join(package).join("__init__.py"), source).unwrap();
    let info = site.join(format!("{dist}-{version}.dist-info"));
    std::fs::create_dir_all(&info).unwrap();
    std::fs::write(
        info.join("RECORD"),
        format!(
            "{package}/__init__.py,sha256={},{}\n{dist}-{version}.dist-info/RECORD,,\n\
             ../../../bin/{package},sha256=xyz,1\n",
            sha(source.as_bytes()),
            source.len()
        ),
    )
    .unwrap();
}

/// A library `demo` (release `demo`, dependency `dep`) and its acquired environment.
struct Acquired {
    _dir: tempfile::TempDir,
    library: PathBuf,
    env: PathBuf,
    site: PathBuf,
}

fn acquired() -> Acquired {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let library = root.join("libraries/demo");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(
        library.join("pyproject.toml"),
        "[project]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         dependencies = [\"demo==1.0\"]\n\n[tool.lctx]\nrelease = [\"demo\"]\n",
    )
    .unwrap();
    std::fs::write(library.join("uv.lock"), lock("1.0", "aaaa")).unwrap();
    let env = root.join("envs/demo");
    let site = env.join("lib/python3.14/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(env.join("pyvenv.cfg"), "home = /x\nversion_info = 3.14.7\n").unwrap();
    install(&site, "demo", "1.0", "demo", DEMO);
    install(&site, "dep", "2.0", "dep", DEP);
    std::fs::write(site.join("_virtualenv.py"), "# loose\n").unwrap();
    Acquired {
        _dir: dir,
        library,
        env,
        site,
    }
}

fn input(a: &Acquired) -> Result<cpg_extract::ExtractInput, ExtractError> {
    library::acquired(&a.library, &a.env, Id([7; 16]))
}

#[test]
fn an_acquired_library_compiles_its_release_distributions_only() {
    let a = acquired();
    let i = input(&a).unwrap();
    assert_eq!(i.release.root, a.site);
    assert_eq!(i.release.files, [a.site.join("demo/__init__.py")]);
    assert_eq!(i.python_version, (3, 14, 7));
    let out = extract(&i).unwrap();
    let files = out.table("source_files").unwrap();
    assert_eq!(
        files.num_rows(),
        1,
        "the dependency is context, not release"
    );
    assert_eq!(cell(files, "path", 0), "demo/__init__.py");
    let releases = out.table("releases").unwrap();
    assert_eq!(cell(releases, "library", 0), "demo");
    assert_eq!(cell(releases, "requirement", 0), "demo==1.0");
    let dists = out.table("distributions").unwrap();
    let rows: Vec<String> = (0..dists.num_rows())
        .map(|r| {
            format!(
                "{} {} {}",
                cell(dists, "name", r),
                cell(dists, "version", r),
                cell(dists, "in_release", r)
            )
        })
        .collect();
    assert_eq!(rows, ["demo 1.0 true", "dep 2.0 false"]);
    // The dependency resolves: the call targets `dep.helper`.
    let calls = out.table("pysa_calls").unwrap();
    assert!((0..calls.num_rows()).any(|r| cell(calls, "target_name", r) == "helper"));
}

#[test]
fn a_release_file_that_differs_from_its_record_is_refused() {
    let a = acquired();
    std::fs::write(
        a.site.join("demo/__init__.py"),
        "def api(x):\n    return 0\n",
    )
    .unwrap();
    let err = input(&a).unwrap_err();
    assert!(
        err.to_string().contains("does not match its RECORD"),
        "{err}"
    );
}

#[test]
fn an_environment_not_synced_to_the_lock_is_refused() {
    let a = acquired();
    std::fs::write(a.library.join("uv.lock"), lock("1.1", "aaaa")).unwrap();
    let err = input(&a).unwrap_err();
    assert!(err.to_string().contains("uv.lock has 1.1"), "{err}");
}

#[test]
fn the_release_id_follows_the_release_distributions_locked_artifacts() {
    let a = acquired();
    let base = input(&a).unwrap().release.release_id;
    std::fs::write(a.library.join("uv.lock"), lock("1.0", "cccc")).unwrap();
    assert_ne!(base, input(&a).unwrap().release.release_id);
    std::fs::write(
        a.library.join("uv.lock"),
        lock("1.0", "aaaa").replace("sha256:bbbb", "sha256:dddd"),
    )
    .unwrap();
    assert_eq!(
        base,
        input(&a).unwrap().release.release_id,
        "a dependency's artifacts belong to the context, not the release"
    );
}

#[test]
fn the_environment_digest_reads_records_and_loose_files() {
    let a = acquired();
    let context = |a: &Acquired| extract(&input(a).unwrap()).unwrap().context_id;
    let base = context(&a);
    // Bytes a RECORD owns are covered by the RECORD, not re-read.
    std::fs::write(
        a.site.join("dep/__init__.py"),
        "def helper(x):\n    return 1\n",
    )
    .unwrap();
    assert_eq!(base, context(&a));
    // A changed RECORD (a reinstalled dependency) changes the context.
    let record = a.site.join("dep-2.0.dist-info/RECORD");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(&record, format!("{text}dep/extra.py,sha256=e,1\n")).unwrap();
    let changed = context(&a);
    assert_ne!(base, changed);
    // So does a loose file no RECORD owns.
    std::fs::write(a.site.join("_virtualenv.py"), "# changed\n").unwrap();
    assert_ne!(changed, context(&a));
}
