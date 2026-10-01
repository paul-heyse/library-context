//! Acquisition (cutover plan A2, T6; ADR-0089; review focus #5) over a synthetic acquired library:
//! a definition, its lock, the environment `uv sync --frozen` would build (real `RECORD` hashes and
//! a location-dependent console-script line) and a fetched source tree. The acquired rows are
//! validated by the model in a stage-bound memory generation. Every control states its answer first.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cpg_extract::{
    acquisition::{self, ACQUIRE, Acquire, InputInventory},
    bundle::{Declared, ProviderStage, StageContext, run_stage},
};
use lctx_model::domain::{
    batching::TransferLimits,
    input::*,
    memory::MemoryGeneration,
    resources::ResourceBudget,
    source::{Module, SourceArtifact},
    stages::*,
    *,
};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const DEMO: &str = "from dep import helper\n\n\ndef api(x):\n    return helper(x)\n";
const DEP: &str = "def helper(x: int) -> int:\n    return x\n";
const README: &str = "# Demo\n\nUse it:\n\n```python\nfrom demo import api\napi(1)\n```\n";
const DEMO_HASH: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn lock(demo_version: &str, demo_artifacts: &str, extra: &str) -> String {
    format!(
        "version = 1\nrevision = 3\nrequires-python = \"==3.14.*\"\n\n\
         [[package]]\nname = \"demo\"\nversion = \"{demo_version}\"\n{demo_artifacts}\n\
         dependencies = [{{ name = \"dep\" }}]\n\n\
         [[package]]\nname = \"dep\"\nversion = \"2.0\"\n\
         source = {{ registry = \"https://pypi.org/simple\" }}\n\
         wheels = [{{ url = \"https://x/dep.whl\", hash = \"sha256:{}\", size = 1 }}]\n\n\
         [[package]]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         source = {{ virtual = \".\" }}\ndependencies = [{{ name = \"demo\" }}]\n{extra}",
        "b".repeat(64)
    )
}
fn registry() -> String {
    format!(
        "source = {{ registry = \"https://pypi.org/simple\" }}\nwheels = [{{ url = \"https://x/demo.whl\", hash = \"{DEMO_HASH}\", size = 1 }}]"
    )
}
fn sha(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

/// Install `package/__init__.py` (and `py.typed`) as distribution `dist` `version`, with `METADATA`
/// and a `RECORD` whose console-script line carries the environment's own path.
fn install(site: &Path, dist: &str, version: &str, package: &str, source: &str) {
    std::fs::create_dir_all(site.join(package)).unwrap();
    std::fs::write(site.join(package).join("__init__.py"), source).unwrap();
    std::fs::write(site.join(package).join("py.typed"), "").unwrap();
    let info = site.join(format!("{dist}-{version}.dist-info"));
    std::fs::create_dir_all(&info).unwrap();
    let metadata = format!("Metadata-Version: 2.4\nName: {dist}\nVersion: {version}\n");
    std::fs::write(info.join("METADATA"), &metadata).unwrap();
    std::fs::write(info.join("RECORD"), format!(
        "{package}/__init__.py,sha256={},{}\n{package}/py.typed,sha256={},0\n{dist}-{version}.dist-info/METADATA,sha256={},{}\n\
         {dist}-{version}.dist-info/RECORD,,\n../../../bin/{package},sha256={},1\n",
        sha(source.as_bytes()), source.len(), sha(b""), sha(metadata.as_bytes()), metadata.len(), sha(site.to_string_lossy().as_bytes()))).unwrap();
}

struct Fixture {
    _dir: tempfile::TempDir,
    library: PathBuf,
    env: PathBuf,
    site: PathBuf,
    tree: PathBuf,
}
fn fixture_at(sub: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap().join(sub);
    let library = root.join("libraries/demo");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("pyproject.toml"), "[project]\nname = \"lctx-library-demo\"\nversion = \"0\"\n\
         dependencies = [\"demo==1.0\"]\n\n[tool.lctx]\nrelease = [\"demo\"]\n\n\
         [tool.lctx.source]\nrepository = \"https://github.com/x/demo\"\ntag = \"v1.0\"\n\
         commit = \"0123456789abcdef0123456789abcdef01234567\"\ndocuments = [\"README.md\"]\nexamples = [\"examples/*.py\"]\n").unwrap();
    std::fs::write(library.join(".python-version"), "3.14.7\n").unwrap();
    std::fs::write(library.join("uv.lock"), lock("1.0", &registry(), "")).unwrap();
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
    std::fs::write(site.join("_virtualenv.pth"), "import _virtualenv\n").unwrap();
    let tree = root.join("sources/demo");
    std::fs::create_dir_all(tree.join("examples")).unwrap();
    std::fs::write(tree.join("README.md"), README).unwrap();
    std::fs::write(tree.join("examples/main.py"), "import helper\n").unwrap();
    std::fs::write(tree.join("helper.py"), "VALUE = 1\n").unwrap();
    std::fs::write(tree.join("NOTES.txt"), "not captured\n").unwrap();
    Fixture {
        _dir: dir,
        library,
        env,
        site,
        tree,
    }
}
fn fixture() -> Fixture {
    fixture_at("one")
}
fn inventory(f: &Fixture) -> Result<InputInventory, cpg_extract::ExtractError> {
    acquisition::inventory(&f.library, &f.env, Some(&f.tree))
}
fn refused(f: &Fixture, needle: &str) {
    let resources = budget();
    let error = inventory(f)
        .map_err(|e| e.to_string())
        .and_then(|i| {
            acquisition::capture(
                &i,
                &resources,
                cpg_extract::native_context::NativeContextConfig::committed(
                    lctx_model::domain::stages::Profile::Catalog,
                    &resources,
                )
                .unwrap(),
            )
            .map(drop)
            .map_err(|e| e.to_string())
        })
        .unwrap_err();
    assert!(error.contains(needle), "expected `{needle}`, got `{error}`");
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}

/// The acquired rows the model validated, and the generation's content digest.
#[derive(Default, Clone)]
struct Rows {
    digest: Option<ContentHash>,
    revisions: Vec<InputRevision>,
    origins: Vec<InputOrigin>,
    artifacts: Vec<SourceArtifact>,
    ownership: Vec<ArtifactOwnership>,
    unowned: Vec<UnownedArtifact>,
    derived: Vec<DerivedArtifact>,
    uses: Vec<ArtifactUse>,
    distributions: Vec<InputDistribution>,
    verifications: Vec<DistributionVerification>,
    fingerprints: Vec<EnvironmentFingerprint>,
    corpus_libraries: Vec<CorpusLibrary>,
    releases: Vec<Release>,
}
impl Rows {
    fn path(&self, id: Id<SourceArtifact>) -> &str {
        &self.artifacts.iter().find(|a| a.id() == id).unwrap().path
    }
    fn class(&self, path: &str) -> &'static str {
        let id = self
            .artifacts
            .iter()
            .find(|a| a.path == path)
            .unwrap_or_else(|| panic!("{path} is not captured"))
            .id();
        let classes: Vec<&str> = [
            (self.ownership.iter().any(|o| o.artifact == id), "owned"),
            (self.unowned.iter().any(|u| u.artifact == id), "unowned"),
            (self.derived.iter().any(|d| d.artifact() == id), "derived"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, c)| c)
        .collect();
        assert_eq!(classes.len(), 1, "{path} has classes {classes:?}");
        classes[0]
    }
    fn roles(&self, path: &str) -> Vec<SourceRole> {
        let id = self.artifacts.iter().find(|a| a.path == path).unwrap().id();
        self.uses
            .iter()
            .filter(|u| u.artifact == id)
            .map(|u| u.role)
            .collect()
    }
}
/// Reads every acquisition relation through its handoff.
struct Inspect(Arc<Mutex<Rows>>);
fn inspect_stage() -> Stage {
    let acquire = Acquire::new(ContentHash::of(b"x")).declaration(Profile::Catalog);
    Stage {
        name: "inspect",
        inputs: acquire.outputs,
        outputs: vec![RelationUse::of::<Module>()],
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![Profile::Catalog, Profile::Behavioral],
        effect: Effect::Pure,
        code: ContentHash::of(b"inspect"),
        configuration: ContentHash::of(b"inspect"),
    }
}
impl Declared for Inspect {
    fn declaration(&self, _: Profile) -> Stage {
        inspect_stage()
    }
}
impl ProviderStage<MemoryGeneration> for Inspect {
    fn run(
        &mut self,
        context: &mut StageContext<MemoryGeneration>,
    ) -> Result<ProviderOutcome, ModelError> {
        fn all<R: Record>(
            context: &mut StageContext<MemoryGeneration>,
        ) -> Result<Vec<R>, ModelError> {
            Ok(context
                .handoff::<R>()?
                .iter()
                .flat_map(|b| b.rows().to_vec())
                .collect())
        }
        let mut rows = self.0.lock().unwrap();
        rows.revisions = all(context)?;
        rows.origins = all(context)?;
        rows.artifacts = all(context)?;
        rows.ownership = all(context)?;
        rows.unowned = all(context)?;
        rows.derived = all(context)?;
        rows.uses = all(context)?;
        rows.distributions = all(context)?;
        rows.verifications = all(context)?;
        rows.fingerprints = all(context)?;
        rows.corpus_libraries = all(context)?;
        rows.releases = all(context)?;
        context.declare::<Module>()?;
        Ok(ProviderOutcome::Complete)
    }
}
async fn acquire(f: &Fixture) -> Result<Rows, String> {
    let budget = budget();
    let inventory = inventory(f).map_err(|e| e.to_string())?;
    let captured = Arc::new(
        acquisition::capture(
            &inventory,
            &budget,
            cpg_extract::native_context::NativeContextConfig::committed(
                lctx_model::domain::stages::Profile::Catalog,
                &budget,
            )
            .unwrap(),
        )
        .map_err(|e| e.to_string())?,
    );
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let stage = Acquire::of(&inventory).declaration(Profile::Catalog);
    let schedule =
        Schedule::build(&model, vec![stage, inspect_stage()], &[], Profile::Catalog).unwrap();
    let mut execution = schedule.execute();
    let generation = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let rows = Arc::new(Mutex::new(Rows::default()));
    let providers: Vec<(&str, Box<dyn ProviderStage<MemoryGeneration>>)> = vec![
        (ACQUIRE, Box::new(Acquire::of(&inventory))),
        ("inspect", Box::new(Inspect(rows.clone()))),
    ];
    for (name, provider) in providers {
        run_stage(
            provider,
            execution.begin(name).unwrap(),
            &generation,
            &model,
            &captured,
            &budget,
            TransferLimits::default(),
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    execution.finish().unwrap();
    let digest = generation
        .validate(&model, &budget)
        .map_err(|e| e.to_string())?;
    let mut rows = rows.lock().unwrap().clone();
    rows.digest = Some(digest);
    Ok(rows)
}
/// Every file and directory under `roots`, with its bytes.
fn snapshot(roots: &[&Path]) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    for root in roots {
        for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
            let entry = entry.unwrap();
            out.insert(
                entry.path().to_path_buf(),
                entry
                    .file_type()
                    .is_file()
                    .then(|| std::fs::read(entry.path()).unwrap()),
            );
        }
    }
    out
}

#[tokio::test]
async fn an_installed_library_and_its_corpus_acquire_every_class_and_role() {
    let f = fixture();
    let rows = acquire(&f).await.unwrap();
    let [library, corpus] = [0, 1].map(|i| {
        rows.revisions
            .iter()
            .find(|r| {
                rows.artifacts
                    .iter()
                    .any(|a| a.input == r.id() && (i == 0) == a.path.starts_with("demo"))
            })
            .unwrap()
            .id()
    });
    let paths = |input: Id<InputRevision>| {
        let mut p = rows
            .artifacts
            .iter()
            .filter(|a| a.input == input)
            .map(|a| a.path.as_str())
            .collect::<Vec<_>>();
        p.sort();
        p
    };
    assert_eq!(
        paths(library),
        [
            "_virtualenv.pth",
            "_virtualenv.py",
            "demo-1.0.dist-info/METADATA",
            "demo/__init__.py",
            "demo/py.typed",
            "dep-2.0.dist-info/METADATA",
            "dep/__init__.py",
            "dep/py.typed"
        ],
        "the closure: sources, stubs markers, .pth and metadata; never a RECORD"
    );
    let block = paths(corpus)
        .into_iter()
        .find(|p| p.starts_with("_lctx/blocks/"))
        .unwrap()
        .to_owned();
    assert_eq!(
        paths(corpus),
        ["README.md", &block, "examples/main.py", "helper.py"],
        "the selection plus every importable module; NOTES.txt is not"
    );
    for (path, class, roles) in [
        ("demo/__init__.py", "owned", vec![SourceRole::Release]),
        ("demo/py.typed", "owned", vec![SourceRole::Release]),
        ("dep/__init__.py", "owned", vec![SourceRole::Dependency]),
        (
            "dep-2.0.dist-info/METADATA",
            "owned",
            vec![SourceRole::DistributionMetadata],
        ),
        ("_virtualenv.py", "unowned", vec![SourceRole::Dependency]),
        ("_virtualenv.pth", "unowned", vec![SourceRole::Dependency]),
        ("README.md", "unowned", vec![SourceRole::Document]),
        ("examples/main.py", "unowned", vec![SourceRole::Example]),
        ("helper.py", "unowned", vec![]),
        (block.as_str(), "derived", vec![SourceRole::DocBlock]),
    ] {
        assert_eq!(
            (rows.class(path), rows.roles(path)),
            (class, roles),
            "{path}"
        );
    }
    let derived = &rows.derived[0];
    let DerivedArtifact::PythonCodeBlock {
        document, ordinal, ..
    } = derived
    else {
        panic!("expected a Python block")
    };
    assert_eq!((rows.path(*document), *ordinal), ("README.md", 0));
    let DerivedArtifact::PythonCodeBlock {
        fence_start,
        fence_end,
        ..
    } = derived
    else {
        panic!("expected a Python block")
    };
    let fence = &README.as_bytes()[*fence_start as usize..*fence_end as usize];
    assert!(
        fence.starts_with(b"```python") && fence.ends_with(b"```"),
        "{}",
        String::from_utf8_lossy(fence)
    );
    let owner = |path: &str| {
        let id = rows.artifacts.iter().find(|a| a.path == path).unwrap().id();
        let verification = rows
            .ownership
            .iter()
            .find(|o| o.artifact == id)
            .unwrap()
            .distribution;
        let release = rows
            .verifications
            .iter()
            .find(|v| v.id() == verification)
            .unwrap()
            .release;
        rows.releases
            .iter()
            .find(|r| r.id() == release)
            .unwrap()
            .version
            .clone()
    };
    assert_eq!(
        (owner("demo/__init__.py"), owner("dep/__init__.py")),
        ("1.0".to_owned(), "2.0".to_owned())
    );
    let roles: Vec<_> = rows
        .distributions
        .iter()
        .map(|d| {
            (
                rows.releases
                    .iter()
                    .find(|r| r.id() == d.release)
                    .unwrap()
                    .version
                    .clone(),
                d.role,
            )
        })
        .collect();
    assert_eq!(roles.len(), 2);
    assert!(
        roles.contains(&("1.0".into(), DistributionRole::FirstParty))
            && roles.contains(&("2.0".into(), DistributionRole::Dependency))
    );
    assert_eq!(
        rows.verifications
            .iter()
            .find(|v| rows
                .releases
                .iter()
                .any(|r| r.id() == v.release && r.version == "1.0"))
            .unwrap()
            .artifact_sha256,
        ["a".repeat(64)]
    );
    let fingerprint = &rows.fingerprints[0];
    assert_eq!(
        rows.fingerprints.len(),
        1,
        "one fingerprint, for the first-party release"
    );
    let revision = rows.revisions.iter().find(|r| r.id() == library).unwrap();
    assert_eq!(
        (
            fingerprint.environment_digest,
            fingerprint.python_version.as_str(),
            fingerprint.platform.as_str(),
            fingerprint.lock_digest
        ),
        (
            revision.manifest,
            "3.14.7",
            std::env::consts::OS,
            ContentHash::of(lock("1.0", &registry(), "").as_bytes())
        )
    );
    assert_eq!(rows.corpus_libraries, [CorpusLibrary { corpus, library }]);
    assert_eq!(rows.origins.len(), 2);
    assert!(rows.origins.iter().any(|o| matches!(o, InputOrigin::Installed { library, requirement, installer, .. }
        if library == "demo" && requirement == "demo==1.0" && installer.as_deref() == Some("uv 0.12.18"))), "{:?}", rows.origins);
    assert!(rows.origins.iter().any(|o| matches!(o, InputOrigin::Corpus { repository, revision }
        if repository == "https://github.com/x/demo" && revision == "0123456789abcdef0123456789abcdef01234567")), "{:?}", rows.origins);
}

/// Review focus #5: acquisition reads; it never writes into the source tree, the environment or
/// the definition.
#[tokio::test]
async fn acquisition_never_writes_the_tree_the_environment_or_the_definition() {
    let f = fixture();
    let before = snapshot(&[&f.tree, &f.env, &f.library]);
    acquire(&f).await.unwrap();
    assert_eq!(
        snapshot(&[&f.tree, &f.env, &f.library]),
        before,
        "byte-identical after acquisition"
    );
    assert!(!f.tree.join("_lctx").exists() && !f.tree.join("_lctx_blocks").exists());
}

#[tokio::test]
async fn a_reserved_namespace_or_stale_blocks_in_the_tree_are_refused() {
    for reserved in ["_lctx_blocks", "_lctx"] {
        let f = fixture();
        std::fs::create_dir_all(f.tree.join(reserved)).unwrap();
        std::fs::write(f.tree.join(reserved).join("block_0.py"), "x = 1\n").unwrap();
        refused(
            &f,
            &format!("holds `{reserved}`, which acquisition reserves"),
        );
    }
    let f = fixture();
    assert!(
        acquisition::inventory(&f.library, &f.env, None)
            .unwrap_err()
            .to_string()
            .contains("its fetched tree is required")
    );
}

#[tokio::test]
async fn acquisition_is_location_independent() {
    let (one, two) = (fixture_at("one"), fixture_at("two/deeper"));
    let (a, b) = (acquire(&one).await.unwrap(), acquire(&two).await.unwrap());
    assert_eq!(
        a.digest, b.digest,
        "where the environment and tree sit is not identity; the RECORDs' console-script lines differ"
    );
    assert_eq!(a.revisions, b.revisions);
}

#[tokio::test]
async fn acquisition_follows_the_environment_and_its_lock() {
    let base = acquire(&fixture()).await.unwrap();
    let library = |rows: &Rows| rows.fingerprints[0].environment_digest;
    // A dependency reinstalled with other content.
    let changed = fixture();
    install(
        &changed.site,
        "dep",
        "2.0",
        "dep",
        "def helper(x: int) -> str:\n    return str(x)\n",
    );
    assert_ne!(library(&acquire(&changed).await.unwrap()), library(&base));
    // An unowned stub inside an owned package joins the closure, unowned.
    let stub = fixture();
    std::fs::write(stub.site.join("dep/extra.pyi"), "def extra() -> int: ...\n").unwrap();
    let with_stub = acquire(&stub).await.unwrap();
    assert_ne!(library(&with_stub), library(&base));
    assert_eq!(
        (
            with_stub.class("dep/extra.pyi"),
            with_stub.roles("dep/extra.pyi")
        ),
        ("unowned", vec![SourceRole::Dependency])
    );
    // A lock-only change keeps the captured environment and changes the fingerprint's lock.
    let relocked = fixture();
    std::fs::write(
        relocked.library.join("uv.lock"),
        lock("1.0", &registry(), "\n# a lock-only change\n"),
    )
    .unwrap();
    let relocked = acquire(&relocked).await.unwrap();
    assert_eq!(library(&relocked), library(&base));
    assert_ne!(
        relocked.fingerprints[0].lock_digest,
        base.fingerprints[0].lock_digest
    );
    assert_ne!(relocked.digest, base.digest);
}

#[tokio::test]
async fn a_tampered_record_or_file_is_refused() {
    let f = fixture();
    std::fs::write(
        f.site.join("dep/__init__.py"),
        "def helper(x):\n    return 1\n",
    )
    .unwrap();
    refused(
        &f,
        "dep: dep/__init__.py does not match its RECORD sha256; run `lctx acquire demo --reinstall`",
    );
    let f = fixture();
    let record = f.site.join("demo-1.0.dist-info/RECORD");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(
        &record,
        text.replacen(
            &format!("demo/__init__.py,sha256={}", sha(DEMO.as_bytes())),
            "demo/__init__.py,",
            1,
        ),
    )
    .unwrap();
    refused(&f, "demo: demo/__init__.py has no RECORD hash");
    let f = fixture();
    std::fs::remove_file(f.site.join("dep/py.typed")).unwrap();
    refused(
        &f,
        "dep/py.typed is listed in a RECORD but missing from the environment",
    );
}

#[tokio::test]
async fn the_environment_must_be_the_locks_and_the_pins() {
    let f = fixture();
    std::fs::write(f.library.join("uv.lock"), lock("1.1", &registry(), "")).unwrap();
    let pyproject = f.library.join("pyproject.toml");
    let text = std::fs::read_to_string(&pyproject).unwrap();
    std::fs::write(&pyproject, text.replace("tag = \"v1.0\"", "tag = \"v1.1\"")).unwrap();
    refused(&f, "uv.lock has 1.1");
    let f = fixture();
    std::fs::write(
        f.env.join("pyvenv.cfg"),
        "home = /x\nversion_info = 3.14.5\n",
    )
    .unwrap();
    refused(&f, "runs Python 3.14.5, .python-version pins 3.14.7");
    let f = fixture();
    std::fs::write(f.library.join("uv.lock"), lock("1.0", "source = { git = \"https://github.com/x/demo?rev=abc#0123456789abcdef0123456789abcdef01234567\" }", "")).unwrap();
    refused(&f, "locked without artifact hashes");
}

#[tokio::test]
async fn the_source_is_pinned_and_names_the_locked_version() {
    let f = fixture();
    let pyproject = f.library.join("pyproject.toml");
    let text = std::fs::read_to_string(&pyproject).unwrap();
    std::fs::write(&pyproject, text.replace("tag = \"v1.0\"", "tag = \"v0.9\"")).unwrap();
    refused(&f, "does not name the locked demo 1.0");
    std::fs::write(
        &pyproject,
        text.replace(
            "commit = \"0123456789abcdef0123456789abcdef01234567\"",
            "commit = \"0123abc\"",
        ),
    )
    .unwrap();
    refused(&f, "must pin `commit`");
    std::fs::write(
        &pyproject,
        text.replace(
            "examples = [\"examples/*.py\"]",
            "examples = [\"samples/*.py\"]",
        ),
    )
    .unwrap();
    refused(&f, "glob `samples/*.py` selects nothing");
}

/// A `RECORD` path holding a comma is quoted (PEP 376): it is read as CSV, captured and verified.
#[tokio::test]
async fn a_quoted_record_path_is_captured_and_verified() {
    let f = fixture();
    let odd = "X = 1\n";
    std::fs::write(f.site.join("demo/a,b.py"), odd).unwrap();
    let record = f.site.join("demo-1.0.dist-info/RECORD");
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
    let rows = acquire(&f).await.unwrap();
    assert_eq!(
        (rows.class("demo/a,b.py"), rows.roles("demo/a,b.py")),
        ("owned", vec![SourceRole::Release])
    );
    std::fs::write(f.site.join("demo/a,b.py"), "X = 2\n").unwrap();
    refused(&f, "demo/a,b.py does not match its RECORD sha256");
}
