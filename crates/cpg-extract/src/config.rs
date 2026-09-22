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
pub const PYREFLY_REV: &str = "b9f28575ce2baa93dbc416670a34592501b3fcd4";
pub const PYREFLY_PATCH_SHA256: &str =
    "b7b82828e3e3470c93d35d5aef0e24904cd3b570b61772f4883701805959b767";
pub const RUFF_LINE: &str = "ruff crates 0.0.11";
/// Bumped by hand whenever the mapping changes output for the same inputs (it changes
/// `producer_id`). The variant and id snapshots are what show such a change (DESIGN §4.0).
pub const EXTRACTOR_OUTPUT_VERSION: u32 = 2;
/// The driver thread's stack. Part of the producer config: a deeper solve could overflow a smaller
/// stack, which is a SIGSEGV rather than a panic (review F8).
pub const DRIVER_STACK_BYTES: usize = 512 << 20;

/// Everything an extraction may depend on, passed explicitly (DESIGN §4.0).
#[derive(Debug, Clone)]
pub struct ExtractInput {
    /// The immutable analysis tree holding the release's code. Absolute.
    pub release_root: PathBuf,
    /// The analysis venv root; site-package paths are recorded relative to it. Absolute.
    pub venv_root: PathBuf,
    /// Site-package directories inside `venv_root`, in order. Absolute.
    pub site_packages: Vec<PathBuf>,
    pub python_version: (u32, u32, u32),
    pub python_platform: String,
    /// From acquisition (Stage A): distributions, versions and artifact digests.
    pub release_id: Id,
    /// The compile attempt this extraction belongs to (execution identity, DM-12).
    pub snapshot_id: Id,
    /// Keep the Pysa structs as JSON for the harness-equivalence test (§4.2.5).
    pub keep_pysa_json: bool,
    /// Test oracles only; the CLI never sets them.
    #[doc(hidden)]
    pub test_hooks: TestHooks,
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
    let paths = [&input.release_root, &input.venv_root]
        .into_iter()
        .chain(input.site_packages.iter());
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
    let mut cfg = ConfigFile {
        source: ConfigSource::File(input.release_root.join("pyrefly.toml")),
        search_path_from_args: vec![input.release_root.clone()],
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
            if p.starts_with(&input.release_root) {
                *s = relative(p, &input.release_root, "release");
            } else if p.starts_with(&input.venv_root) {
                *s = relative(p, &input.venv_root, "venv");
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| relativize(v, input)),
        Value::Object(map) => map.values_mut().for_each(|v| relativize(v, input)),
        _ => {}
    }
}

/// Digest of the site-package roots' content (review F2): per root in order, every file's
/// root-relative path and content digest (a symlink's target text), in path order. Bytecode
/// caches are skipped; the analyzer never reads them.
fn site_packages_digest(roots: &[PathBuf]) -> std::io::Result<Digest> {
    fn files(dir: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<_, _>>()?;
        entries.sort();
        for p in entries {
            if std::fs::symlink_metadata(&p)?.is_dir() {
                if p.file_name() != Some("__pycache__".as_ref()) {
                    files(&p, acc)?;
                }
            } else {
                acc.push(p);
            }
        }
        Ok(())
    }
    let mut h = IdHasher::new(kind::SITE_PACKAGES);
    for root in roots {
        let mut listed = Vec::new();
        files(root, &mut listed)?;
        h.i64(listed.len() as i64);
        for f in listed {
            let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy();
            let content = if std::fs::symlink_metadata(&f)?.is_symlink() {
                content_digest(std::fs::read_link(&f)?.to_string_lossy().as_bytes())
            } else {
                content_digest(&std::fs::read(&f)?)
            };
            h.str(&rel).digest_field(content);
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
    pub site_packages_digest: Digest,
}

pub(crate) fn context(cfg: &ConfigFile, input: &ExtractInput) -> Result<Context, ExtractError> {
    let mut json = serde_json::to_value(cfg).unwrap_or(Value::Null);
    relativize(&mut json, input);
    // Key order must not depend on the build: DataFusion turns on serde_json's
    // `preserve_order` wherever it shares the dependency graph.
    json.sort_all_objects();
    let search_path: Vec<String> = cfg
        .search_path()
        .map(|p| relative(p, &input.release_root, "release"))
        .collect();
    let site_package_path: Vec<String> = cfg
        .site_package_path()
        .map(|p| relative(p, &input.venv_root, "venv"))
        .collect();
    let (major, minor, micro) = input.python_version;
    let python_version = format!("{major}.{minor}.{micro}");
    let config_digest = content_digest(json.to_string().as_bytes());
    let site_packages_digest = site_packages_digest(&input.site_packages)?;
    let id = IdHasher::new(kind::CONTEXT)
        .str(&python_version)
        .str(&input.python_platform)
        .strs(search_path.iter().map(String::as_str))
        .strs(site_package_path.iter().map(String::as_str))
        .digest_field(config_digest)
        .digest_field(site_packages_digest)
        .finish_id();
    Ok(Context {
        id,
        python_version,
        python_platform: input.python_platform.clone(),
        search_path,
        site_package_path,
        config_digest,
        site_packages_digest,
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
