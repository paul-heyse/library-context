//! Explicit inputs, the constructed Pyrefly configuration and run identity (DESIGN §4.0, §4.2.1).

use std::path::{Path, PathBuf};

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
pub const PYREFLY_REV: &str = "6a93da3460f9279ba32ff4ebec07e1cbdf2eb978";
pub const PYREFLY_PATCH_SHA256: &str =
    "5782fe3e4fe62790e9039783a8be0863c9293f98f0fd73146099189909db4358";
pub const RUFF_LINE: &str = "ruff crates 0.0.11";
/// Bumped by hand whenever the mapping changes output for the same inputs (it changes
/// `producer_id`). The variant and id snapshots are what show such a change (DESIGN §4.0).
pub const EXTRACTOR_OUTPUT_VERSION: u32 = 14;
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
    /// analyzed in the library's environment (so the context is the library's).
    Corpus {
        /// `<repository>@<commit>`.
        label: String,
        library: Option<crate::library::AcquiredLibrary>,
    },
}

impl Release {
    /// Every `.py`/`.pyi` under `root` (dot-directories and `__pycache__` skipped); the id hashes
    /// `label`, so one label on two trees gives one id.
    pub fn from_tree(root: PathBuf, label: &str) -> std::io::Result<Release> {
        fn walk(dir: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
                .map(|e| e.map(|e| e.path()))
                .collect::<Result<_, _>>()?;
            entries.sort();
            for p in entries {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if p.is_dir() {
                    if !name.starts_with('.') && name != "__pycache__" {
                        walk(&p, acc)?;
                    }
                } else if name.ends_with(".py") || name.ends_with(".pyi") {
                    acc.push(p);
                }
            }
            Ok(())
        }
        let mut files = Vec::new();
        if root.is_dir() {
            walk(&root, &mut files)?;
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

    /// A corpus release (C5): the modules are the usage files (examples, tests), and the id hashes
    /// the label and every selected file's release-relative path and content, so the release is
    /// the tree's content, not where it was fetched.
    pub fn corpus(
        root: PathBuf,
        label: &str,
        documents: &[PathBuf],
        usage: Vec<PathBuf>,
        library: Option<crate::library::AcquiredLibrary>,
    ) -> std::io::Result<Release> {
        let mut selected: Vec<(String, Digest)> = Vec::new();
        for f in documents.iter().chain(&usage) {
            let rel = f.strip_prefix(&root).map_err(|_| {
                std::io::Error::other(format!("{} is outside the tree", f.display()))
            })?;
            selected.push((
                rel.display().to_string(),
                content_digest(&std::fs::read(f)?),
            ));
        }
        selected.sort();
        let mut h = IdHasher::new(kind::RELEASE);
        h.str("corpus").str(label).i64(selected.len() as i64);
        for (path, digest) in &selected {
            h.str(path).digest_field(*digest);
        }
        let mut files = usage;
        files.sort();
        Ok(Release {
            release_id: h.finish_id(),
            root,
            files,
            origin: ReleaseOrigin::Corpus {
                label: label.to_owned(),
                library,
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
    fn walk(dir: &Path, infos: &mut Vec<String>, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        entries.sort();
        for p in entries {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if std::fs::symlink_metadata(&p)?.is_dir() {
                if name.ends_with(".dist-info") {
                    infos.push(name);
                } else if name != "__pycache__" {
                    walk(&p, infos, files)?;
                }
            } else if crate::library::analyzer_readable(&name) {
                files.push(p);
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
            h.str(&rel)
                .digest_field(content_digest(&std::fs::read(&f)?));
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
    let mut json = serde_json::to_value(cfg).unwrap_or(Value::Null);
    relativize(&mut json, input);
    // Key order must not depend on the build: DataFusion turns on serde_json's
    // `preserve_order` wherever it shares the dependency graph.
    json.sort_all_objects();
    let search_path: Vec<String> = cfg
        .search_path()
        .map(|p| relative(p, &input.release.root, "release"))
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
    let id = IdHasher::new(kind::PRODUCER)
        .str(TOOL)
        .str(&revision)
        .digest_field(build_digest)
        .finish_id();
    Producer {
        id,
        revision,
        build_digest,
        config_digest,
    }
}

pub(crate) fn run_id(release: Id, context: Id, producer: &Producer, families: &[&str]) -> Id {
    let mut sorted = families.to_vec();
    sorted.sort_unstable();
    IdHasher::new(kind::RUN)
        .id(release)
        .id(context)
        .id(producer.id)
        .strs(sorted)
        .digest_field(producer.config_digest)
        .finish_id()
}
