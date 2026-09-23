//! Stage A (DESIGN §4.0, ADR-0013) over a synthetic acquired environment: a library definition,
//! its `uv.lock`, and the environment `uv sync --frozen` would build, with real `RECORD` hashes.
//! The oracles are the ADR-0013 review's: one equivalence (verified analyzer-readable bytes) for
//! `release_id` and the environment, a hermetic pin, and a pinned docs source.

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

/// `uv.lock` for `demo` (release) and `dep`. `demo_artifacts` is demo's source and artifacts
/// artifacts, e.g. a git source with none; `extra` appends raw lock text.
fn lock(demo_version: &str, demo_artifacts: &str, extra: &str) -> String {
    format!(
        "version = 1\nrevision = 3\nrequires-python = \"==3.14.*\"\n\n\
         [[package]]\nname = \"demo\"\nversion = \"{demo_version}\"\n{demo_artifacts}\n\
         dependencies = [{{ name = \"dep\" }}]\n\n\
         [[package]]\nname = \"dep\"\nversion = \"2.0\"\n\
         source = {{ registry = \"https://pypi.org/simple\" }}\n\
         wheels = [{{ url = \"https://x/dep.whl\", hash = \"sha256:bbbb\", size = 1 }}]\n\n\
         [[package]]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         source = {{ virtual = \".\" }}\ndependencies = [{{ name = \"demo\" }}]\n{extra}"
    )
}

const REGISTRY: &str = "source = { registry = \"https://pypi.org/simple\" }\n\
     wheels = [{ url = \"https://x/demo.whl\", hash = \"sha256:aaaa\", size = 1 }]";

fn sha(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

/// Install `package/__init__.py` as distribution `dist` `version`, with a `RECORD` whose console
/// script line carries the environment's own path (as real installers write it).
fn install(site: &Path, dist: &str, version: &str, package: &str, source: &str) {
    std::fs::create_dir_all(site.join(package)).unwrap();
    std::fs::write(site.join(package).join("__init__.py"), source).unwrap();
    let info = site.join(format!("{dist}-{version}.dist-info"));
    std::fs::create_dir_all(&info).unwrap();
    std::fs::write(
        info.join("RECORD"),
        format!(
            "{package}/__init__.py,sha256={},{}\n{dist}-{version}.dist-info/RECORD,,\n\
             ../../../bin/{package},sha256={},1\n",
            sha(source.as_bytes()),
            source.len(),
            sha(site.to_string_lossy().as_bytes()),
        ),
    )
    .unwrap();
}

struct Acquired {
    _dir: tempfile::TempDir,
    library: PathBuf,
    env: PathBuf,
    site: PathBuf,
}

fn acquired_at(sub: &str) -> Acquired {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap().join(sub);
    let library = root.join("libraries/demo");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(
        library.join("pyproject.toml"),
        "[project]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         dependencies = [\"demo==1.0\"]\n\n[tool.lctx]\nrelease = [\"demo\"]\n\n\
         [tool.lctx.source]\nrepository = \"https://github.com/x/demo\"\ntag = \"v1.0\"\n\
         commit = \"0123456789abcdef0123456789abcdef01234567\"\n",
    )
    .unwrap();
    std::fs::write(library.join(".python-version"), "3.14.7\n").unwrap();
    std::fs::write(library.join("uv.lock"), lock("1.0", REGISTRY, "")).unwrap();
    let env = root.join("envs/demo");
    let site = env.join("lib/python3.14/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(
        env.join("pyvenv.cfg"),
        "home = /x\nuv = 0.12.18\nversion_info = 3.14.7\n",
    )
    .unwrap();
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

fn acquired() -> Acquired {
    acquired_at("one")
}

fn input(a: &Acquired) -> Result<cpg_extract::ExtractInput, ExtractError> {
    library::acquired(&a.library, &a.env, Id([7; 16]))
}

fn refused(a: &Acquired, needle: &str) {
    let err = input(a).map(|_| ()).unwrap_err();
    assert!(err.to_string().contains(needle), "{err}");
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
    assert_eq!(cell(releases, "distributions", 0), "[demo==1.0]");
    assert_eq!(cell(releases, "installer", 0), "uv 0.12.18");
    let dists = out.table("distributions").unwrap();
    let rows: Vec<String> = (0..dists.num_rows())
        .map(|r| format!("{} {}", cell(dists, "name", r), cell(dists, "version", r)))
        .collect();
    assert_eq!(rows, ["demo 1.0", "dep 2.0"]);
    assert_eq!(
        cell(dists, "context_id", 0),
        out.context_id.hex(),
        "the environment is the context's"
    );
    let calls = out.table("pysa_calls").unwrap();
    assert!((0..calls.num_rows()).any(|r| cell(calls, "target_name", r) == "helper"));
}

#[test]
fn a_changed_analyzer_readable_byte_is_refused_anywhere() {
    // The release's own files and the dependencies' are verified against their RECORDs.
    let a = acquired();
    std::fs::write(
        a.site.join("dep/__init__.py"),
        "def helper(x):\n    return 1\n",
    )
    .unwrap();
    refused(
        &a,
        "dep: dep/__init__.py does not match its RECORD sha256; run `lctx acquire demo --reinstall`",
    );
    let b = acquired();
    std::fs::write(
        b.site.join("demo/__init__.py"),
        "def api(x):\n    return 0\n",
    )
    .unwrap();
    refused(&b, "does not match its RECORD");
}

#[test]
fn the_environment_must_be_the_locks_and_the_pins() {
    let a = acquired();
    std::fs::write(a.library.join("uv.lock"), lock("1.1", REGISTRY, "")).unwrap();
    let pyproject = a.library.join("pyproject.toml");
    let text = std::fs::read_to_string(&pyproject).unwrap();
    std::fs::write(&pyproject, text.replace("tag = \"v1.0\"", "tag = \"v1.1\"")).unwrap();
    refused(&a, "uv.lock has 1.1");
    let b = acquired();
    std::fs::write(
        b.env.join("pyvenv.cfg"),
        "home = /x\nversion_info = 3.14.5\n",
    )
    .unwrap();
    refused(&b, "runs Python 3.14.5, .python-version pins 3.14.7");
}

#[test]
fn a_release_locked_without_artifact_hashes_is_refused() {
    let a = acquired();
    std::fs::write(
        a.library.join("uv.lock"),
        lock(
            "1.0",
            "source = { git = \"https://github.com/x/demo?rev=abc#0123456789abcdef0123456789abcdef01234567\" }",
            "",
        ),
    )
    .unwrap();
    refused(&a, "locked without artifact hashes");
}

#[test]
fn the_release_id_is_the_verified_release_content() {
    let a = acquired();
    let base = input(&a).unwrap().release.release_id;
    // A re-listed artifact (a new wheel uploaded for the same version) leaves it.
    std::fs::write(
        a.library.join("uv.lock"),
        lock(
            "1.0",
            &REGISTRY.replace(
                "size = 1 }]",
                "size = 1 }, { url = \"https://x/demo-cp315.whl\", hash = \"sha256:cccc\", size = 1 }]",
            ),
            "",
        ),
    )
    .unwrap();
    assert_eq!(base, input(&a).unwrap().release.release_id);
    // Different release content (reinstalled consistently) changes it.
    let b = acquired();
    install(
        &b.site,
        "demo",
        "1.0",
        "demo",
        "def api(x):\n    return x\n",
    );
    assert_ne!(base, input(&b).unwrap().release.release_id);
}

#[test]
fn the_environment_digest_is_the_analyzer_readable_bytes() {
    let context = |a: &Acquired| extract(&input(a).unwrap()).unwrap().context_id;
    // Two paths: the RECORDs' console-script lines differ, the analyzed bytes do not.
    let (one, two) = (acquired_at("one"), acquired_at("two/deeper"));
    let base = context(&one);
    assert_eq!(
        base,
        context(&two),
        "where an environment sits is not identity"
    );
    // An analyzer-readable file no RECORD owns, inside an owned package, moves it.
    std::fs::write(one.site.join("dep/extra.pyi"), "def extra() -> int: ...\n").unwrap();
    let with_stub = context(&one);
    assert_ne!(base, with_stub);
    // So does a loose file at the top level, and a lock-only change (the lock digest).
    std::fs::write(one.site.join("_virtualenv.py"), "# changed\n").unwrap();
    let loose = context(&one);
    assert_ne!(with_stub, loose);
    std::fs::write(
        one.library.join("uv.lock"),
        lock("1.0", REGISTRY, "\n# a lock-only change\n"),
    )
    .unwrap();
    assert_ne!(loose, context(&one));
}

#[test]
fn the_docs_source_is_pinned_and_names_the_locked_version() {
    let a = acquired();
    let pyproject = a.library.join("pyproject.toml");
    let text = std::fs::read_to_string(&pyproject).unwrap();
    std::fs::write(&pyproject, text.replace("tag = \"v1.0\"", "tag = \"v0.9\"")).unwrap();
    refused(&a, "does not name the locked demo 1.0");
    std::fs::write(
        &pyproject,
        text.replace(
            "commit = \"0123456789abcdef0123456789abcdef01234567\"\n",
            "",
        ),
    )
    .unwrap();
    refused(&a, "missing field `commit`");
    std::fs::write(
        &pyproject,
        text.replace(
            "commit = \"0123456789abcdef0123456789abcdef01234567\"",
            "commit = \"0123abc\"",
        ),
    )
    .unwrap();
    refused(&a, "must pin `commit`");
}

/// A `RECORD` path holding a comma is quoted (PEP 376). It is read as CSV, so the file is verified
/// and compiled like any other, and a changed byte in it is refused (H1 C5).
#[test]
fn a_quoted_record_path_is_verified() {
    let a = acquired();
    let odd = "X = 1\n";
    std::fs::write(a.site.join("demo/a,b.py"), odd).unwrap();
    let record = a.site.join("demo-1.0.dist-info/RECORD");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(
        &record,
        format!(
            "\"demo/a,b.py\",sha256={},{}\n{text}",
            sha(odd.as_bytes()),
            odd.len()
        ),
    )
    .unwrap();
    let i = input(&a).unwrap();
    assert!(
        i.release.files.contains(&a.site.join("demo/a,b.py")),
        "{:?}",
        i.release.files
    );
    std::fs::write(a.site.join("demo/a,b.py"), "X = 2\n").unwrap();
    assert!(input(&a).is_err());
}
