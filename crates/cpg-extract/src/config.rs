//! Explicit inputs, the constructed Pyrefly configuration and run identity (DESIGN §4.0, §4.2.1).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cpg_schema::codebook::{Codebook, SourceRole};
use cpg_schema::id::{Digest, Id, IdHasher, content_digest, kind};
use pyrefly_config::config::{ConfigFile, ConfigSource};
use pyrefly_python::sys_info::{PythonPlatform, PythonVersion};
use serde_json::Value;

use crate::ExtractError;

/// Environment variables Pyrefly reads that change behaviour or output. Clearing them would need
/// `unsafe` (edition 2024), which the workspace forbids, so the driver refuses to start instead.
pub const REFUSED_ENV: &[&str] = &["PYREFLY_STACK_SIZE", "PYREFLY_FIXPOINT_DETAILS"];
/// Any variable with this prefix is refused too (`PYSA_DUMP*` debug output).
pub const REFUSED_ENV_PREFIX: &str = "PYSA_DUMP";

/// The tool name recorded in `producers`.
pub const TOOL: &str = "lctx-extract";
/// The Pyrefly fork revision this crate is built against, and the digest of the patch it carries
/// over tag 1.3.1 (docs/pins.md). `just deps` fails unless both equal the `Cargo.lock` revision
/// and `third_party/pyrefly-1.3.1.patch` (review F4). Both feed `producer_id`.
pub const PYREFLY_REV: &str = "a07b7baead9e0c7b496346d879b88e2fff9cbda7";
pub const PYREFLY_PATCH_SHA256: &str =
    "fc18dc4a884a8593220370ba053968fd10de65c020ef257931f97b91426fdb73";
pub const RUFF_LINE: &str = "ruff crates 0.0.11";
/// Bumped by hand whenever the mapping changes output for the same inputs (it changes
/// `producer_id`). The variant and id snapshots are what show such a change (DESIGN §4.0).
pub const EXTRACTOR_OUTPUT_VERSION: u32 = 19;
/// The driver thread's stack. Part of the producer config: a deeper solve could overflow a smaller
/// stack, which is a SIGSEGV rather than a panic (review F8).
pub const DRIVER_STACK_BYTES: usize = 512 << 20;

/// What is analyzed: the release's modules under one search root, and its identity.
#[derive(Debug, Clone)]
pub struct Release {
    /// The search root the modules are relative to: a source tree, or site-packages for an
    /// acquired library. Absolute.
    pub root: PathBuf,
    /// The release's modules (`.py`/`.pyi`), absolute, under `root`, sorted. For an acquired
    /// library these are its first-party distributions' `RECORD` entries, verified.
    pub files: Vec<PathBuf>,
    pub release_id: Id,
    pub origin: ReleaseOrigin,
}

/// How a release was obtained; recorded in `releases` and `distributions`.
#[derive(Debug, Clone)]
pub enum ReleaseOrigin {
    /// A source tree compiled under a label (fixtures, local checkouts).
    Tree { label: String },
    /// A library acquired from its committed uv project (Stage A, ADR-0013).
    Library(crate::library::AcquiredLibrary),
    /// A library's upstream tree at its pinned commit (C5): its documents, examples and tests,
    /// analyzed in the library's environment (its context adds the tree to the search path).
    Corpus {
        /// `<repository>@<commit>`.
        label: String,
        library: Option<crate::library::AcquiredLibrary>,
        /// Each module's role, by release-relative path (ADR-0015).
        roles: BTreeMap<String, SourceRole>,
    },
}

impl Release {
    /// Every `.py`/`.pyi` under `root` (dot-directories and `__pycache__` skipped); the id hashes
    /// `label`, so one label on two trees gives one id.
    pub fn from_tree(root: PathBuf, label: &str) -> std::io::Result<Release> {
        // One walkdir pass that follows no link (H1 C2): a link in a source tree is refused,
        // naming it, as in a fetched corpus.
        let mut files = Vec::new();
        if root.is_dir() {
            let entries = walkdir::WalkDir::new(&root)
                .follow_links(false)
                .sort_by_file_name()
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !(e.file_type().is_dir() && (name.starts_with('.') || name == "__pycache__"))
                });
            for entry in entries {
                let entry = entry.map_err(std::io::Error::other)?;
                let name = entry.file_name().to_string_lossy();
                if entry.file_type().is_symlink() {
                    return Err(std::io::Error::other(format!(
                        "{} is a symlink; a source tree is read without following links",
                        entry.path().display()
                    )));
                }
                if entry.file_type().is_file() && (name.ends_with(".py") || name.ends_with(".pyi"))
                {
                    files.push(entry.into_path());
                }
            }
        }
        Ok(Release {
            release_id: IdHasher::new(kind::RELEASE).str(label).finish_id(),
            root,
            files,
            origin: ReleaseOrigin::Tree {
                label: label.to_owned(),
            },
        })
    }

    /// A corpus release (C5): the modules are the usage files (examples, tests, doc blocks), and
    /// the id hashes the label and every selected file's release-relative path, content and role,
    /// so the release is the tree's content, not where it was fetched.
    pub fn corpus(
        root: PathBuf,
        label: &str,
        documents: &[PathBuf],
        usage: Vec<(PathBuf, SourceRole)>,
        library: Option<crate::library::AcquiredLibrary>,
        library_release: Id,
    ) -> std::io::Result<Release> {
        let relative = |f: &PathBuf| {
            f.strip_prefix(&root)
                .map(|r| r.display().to_string())
                .map_err(|_| std::io::Error::other(format!("{} is outside the tree", f.display())))
        };
        let mut selected: Vec<(String, Digest, Option<i64>)> = Vec::new();
        for f in documents {
            selected.push((relative(f)?, content_digest(&fs_err::read(f)?), None));
        }
        let mut roles = BTreeMap::new();
        for (f, role) in &usage {
            let rel = relative(f)?;
            selected.push((
                rel.clone(),
                content_digest(&fs_err::read(f)?),
                Some(i64::from(role.code())),
            ));
            roles.insert(rel, *role);
        }
        selected.sort();
        let mut h = IdHasher::new(kind::RELEASE);
        // The library release is an input: its public names are the documents' vocabulary, and
        // its files are what the usage code reaches (C5 review F1).
        h.str("corpus")
            .str(label)
            .id(library_release)
            .i64(selected.len() as i64);
        for (path, digest, role) in &selected {
            h.str(path).digest_field(*digest).opt_i64(*role);
        }
        let mut files: Vec<PathBuf> = usage.into_iter().map(|(f, _)| f).collect();
        files.sort();
        Ok(Release {
            release_id: h.finish_id(),
            root,
            files,
            origin: ReleaseOrigin::Corpus {
                label: label.to_owned(),
                library,
                roles,
            },
        })
    }

    /// The library whose environment the release is analyzed in: its `RECORD` owners name the
    /// distribution of every installed file.
    pub(crate) fn environment_library(&self) -> Option<&crate::library::AcquiredLibrary> {
        match &self.origin {
            ReleaseOrigin::Tree { .. } => None,
            ReleaseOrigin::Library(l) => Some(l),
            ReleaseOrigin::Corpus { library, .. } => library.as_ref(),
        }
    }

    pub(crate) fn lock_digest(&self) -> Option<Digest> {
        self.environment_library().map(|l| l.lock_digest)
    }
}

/// Everything an extraction may depend on, passed explicitly (DESIGN §4.0).
#[derive(Debug, Clone)]
pub struct ExtractInput {
    pub release: Release,
    /// The analysis environment's root; site-package paths are recorded relative to it. Absolute.
    pub venv_root: PathBuf,
    /// Site-package directories inside `venv_root`, in order. Absolute.
    pub site_packages: Vec<PathBuf>,
    pub python_version: (u32, u32, u32),
    pub python_platform: String,
    /// The compile attempt this extraction belongs to (execution identity, DM-12).
    pub snapshot_id: Id,
    /// The library's upstream corpus, compiled as a second run of the attempt (C5).
    pub corpus: Option<CorpusInput>,
    /// Keep the Pysa structs as JSON for the harness-equivalence test (§4.2.5).
    pub keep_pysa_json: bool,
    /// Test oracles only; the CLI never sets them.
    #[doc(hidden)]
    pub test_hooks: TestHooks,
}

/// A corpus run's input (C5): its release (the usage modules, and the identity) and its documents.
#[derive(Debug, Clone)]
pub struct CorpusInput {
    pub release: Release,
    /// Selected documents, absolute, under `release.root`, sorted.
    pub documents: Vec<PathBuf>,
}

/// Hooks the tests use to show the driver's contracts can fail (review F1, F6).
#[derive(Debug, Clone, Default)]
pub struct TestHooks {
    /// Panic in the driver loop at this module (before its Pysa collectors run).
    pub fault_at_module: Option<String>,
    /// Hand Pyrefly the module handles in reverse order instead of sorted.
    pub reverse_module_order: bool,
}

pub(crate) fn refuse_ambient() -> Result<(), ExtractError> {
    for (k, _) in std::env::vars_os() {
        let k = k.to_string_lossy();
        if REFUSED_ENV.contains(&k.as_ref()) || k.starts_with(REFUSED_ENV_PREFIX) {
            return Err(ExtractError::Ambient(k.into_owned()));
        }
    }
    Ok(())
}

pub(crate) fn require_absolute(input: &ExtractInput) -> Result<(), ExtractError> {
    let paths = [&input.release.root, &input.venv_root]
        .into_iter()
        .chain(input.site_packages.iter())
        .chain(input.release.files.iter());
    for p in paths {
        if !p.is_absolute() {
            return Err(ExtractError::RelativePath(p.clone()));
        }
    }
    Ok(())
}

/// The constructed configuration: no discovery, heuristics, fallback or interpreter query.
pub(crate) fn pyrefly_config(input: &ExtractInput) -> Result<ConfigFile, ExtractError> {
    let (major, minor, micro) = input.python_version;
    // A corpus resolves imports as its tests run: the tree first, then the environment's
    // site-packages, which is the library run's search path. So a module both runs import is the
    // same file in both (C5b); with site-packages only as site packages, Pyrefly's bundled stubs
    // would win for the corpus and not for the library.
    let mut search_path = vec![input.release.root.clone()];
    if let ReleaseOrigin::Corpus { .. } = input.release.origin {
        search_path.extend(input.site_packages.iter().cloned());
    }
    let mut cfg = ConfigFile {
        source: ConfigSource::File(input.release.root.join("pyrefly.toml")),
        search_path_from_args: search_path,
        disable_search_path_heuristics: true,
        disable_project_excludes_heuristics: true,
        enable_fallback_search_path: false,
        ..ConfigFile::default()
    };
    cfg.python_environment.python_version = Some(PythonVersion::new(major, minor, micro));
    cfg.python_environment.python_platform = Some(PythonPlatform::new(&input.python_platform));
    // Explicit Some prevents configure() from discovering typings/ or querying an interpreter.
    cfg.python_environment.site_package_path = Some(input.site_packages.clone());
    cfg.interpreters.skip_interpreter_query = true;
    let errors = cfg.configure();
    if !errors.is_empty() {
        return Err(ExtractError::Config(errors.len()));
    }
    Ok(cfg)
}

fn relative(path: &Path, root: &Path, label: &str) -> String {
    match path.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => format!("${label}"),
        Ok(rel) => format!("${label}/{}", rel.display()),
        Err(_) => path.display().to_string(),
    }
}

/// Replace absolute roots inside the serialized config so identity never depends on where a
/// checkout or tempdir sits (review F9).
fn relativize(value: &mut Value, input: &ExtractInput) {
    match value {
        Value::String(s) => {
            let p = Path::new(s.as_str());
            if p.starts_with(&input.release.root) {
                *s = relative(p, &input.release.root, "release");
            } else if p.starts_with(&input.venv_root) {
                *s = relative(p, &input.venv_root, "venv");
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| relativize(v, input)),
        Value::Object(map) => map.values_mut().for_each(|v| relativize(v, input)),
        _ => {}
    }
}

/// Digest of the dependency environment (review F2; ADR-0013 review F1): the installed
/// distributions (dist-info names, which carry their versions) and every analyzer-readable file
/// (`.py`, `.pyi`, `py.typed`: what Pyrefly's module finder reads) with its root-relative path
/// and content digest, in path order. Nothing outside the roots (console scripts carry the
/// environment's absolute path) and nothing Pyrefly never reads (`.pth`, bytecode, data) enters
/// it, so a moved environment keeps its identity and any analyzer-visible change moves it.
fn environment_digest(roots: &[PathBuf]) -> std::io::Result<Digest> {
    /// A root's `.dist-info` directory names and analyzer-readable files, in path order: real
    /// directories are walked (`__pycache__` skipped), `.dist-info` ones only named, and a link is
    /// never followed as a directory; a linked file is read like any file (H1 C2: walkdir, the
    /// same semantics as the hand walk it replaced).
    fn walk(root: &Path, infos: &mut Vec<String>, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        let mut entries = walkdir::WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
            .min_depth(1)
            .into_iter();
        while let Some(entry) = entries.next() {
            let entry = entry.map_err(std::io::Error::other)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().is_dir() {
                if name.ends_with(".dist-info") {
                    infos.push(name);
                    entries.skip_current_dir();
                } else if name == "__pycache__" {
                    entries.skip_current_dir();
                }
            } else if crate::library::analyzer_readable(&name) {
                files.push(entry.into_path());
            }
        }
        Ok(())
    }
    let mut h = IdHasher::new(kind::ENVIRONMENT);
    for root in roots {
        let (mut infos, mut files) = (Vec::new(), Vec::new());
        walk(root, &mut infos, &mut files)?;
        infos.sort();
        h.strs(infos.iter().map(String::as_str));
        h.i64(files.len() as i64);
        for f in files {
            let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy();
            h.str(&rel).digest_field(content_digest(&fs_err::read(&f)?));
        }
    }
    Ok(h.finish_digest())
}

/// The analysis context: the configured file, every `#[serde(skip)]` input (root-relative), and
/// the content of the dependency environment.
pub(crate) struct Context {
    pub id: Id,
    pub python_version: String,
    pub python_platform: String,
    pub search_path: Vec<String>,
    pub site_package_path: Vec<String>,
    pub config_digest: Digest,
    pub environment_digest: Digest,
    pub lock_digest: Option<Digest>,
}

pub(crate) fn context(cfg: &ConfigFile, input: &ExtractInput) -> Result<Context, ExtractError> {
    // A configuration that cannot be serialized is refused, never hashed as `null` (H1 C7).
    let mut json = serde_json::to_value(cfg)
        .map_err(|e| ExtractError::Library(format!("the pyrefly configuration: {e}")))?;
    relativize(&mut json, input);
    // Key order must not depend on the build: DataFusion turns on serde_json's
    // `preserve_order` wherever it shares the dependency graph.
    json.sort_all_objects();
    // Each entry relative to its root: the release, or the environment (a corpus also searches
    // site-packages, C5 review F1).
    let search_path: Vec<String> = cfg
        .search_path()
        .map(|p| {
            if p.starts_with(&input.release.root) {
                relative(p, &input.release.root, "release")
            } else {
                relative(p, &input.venv_root, "venv")
            }
        })
        .collect();
    let site_package_path: Vec<String> = cfg
        .site_package_path()
        .map(|p| relative(p, &input.venv_root, "venv"))
        .collect();
    let (major, minor, micro) = input.python_version;
    let python_version = format!("{major}.{minor}.{micro}");
    let config_digest = content_digest(json.to_string().as_bytes());
    let environment_digest = environment_digest(&input.site_packages)?;
    let lock_digest = input.release.lock_digest();
    let id = IdHasher::new(kind::CONTEXT)
        .str(&python_version)
        .str(&input.python_platform)
        .strs(search_path.iter().map(String::as_str))
        .strs(site_package_path.iter().map(String::as_str))
        .digest_field(config_digest)
        .digest_field(environment_digest)
        .opt_digest(lock_digest)
        .finish_id();
    Ok(Context {
        id,
        python_version,
        python_platform: input.python_platform.clone(),
        search_path,
        site_package_path,
        config_digest,
        environment_digest,
        lock_digest,
    })
}

/// The producer: tool, pinned analyzers, driver settings and output version.
pub(crate) struct Producer {
    pub id: Id,
    pub revision: String,
    pub build_digest: Digest,
    /// The producer's own config digest (driver thread settings), part of `run_id`.
    pub config_digest: Digest,
}

pub(crate) fn producer() -> Producer {
    let revision =
        format!("pyrefly 1.3.1 {PYREFLY_REV} (patch sha256 {PYREFLY_PATCH_SHA256}); {RUFF_LINE}");
    let config = format!("threads=inline; stack_bytes={DRIVER_STACK_BYTES}");
    let build = format!("{}/{EXTRACTOR_OUTPUT_VERSION}", env!("CARGO_PKG_VERSION"));
    let build_digest = content_digest(build.as_bytes());
    let config_digest = content_digest(config.as_bytes());
    let id = cpg_schema::id::recipe::producer(TOOL, &revision, build_digest);
    Producer {
        id,
        revision,
        build_digest,
        config_digest,
    }
}

pub(crate) fn run_id(release: Id, context: Id, producer: &Producer, families: &[&str]) -> Id {
    cpg_schema::id::recipe::run(
        release,
        context,
        producer.id,
        families,
        producer.config_digest,
    )
}
