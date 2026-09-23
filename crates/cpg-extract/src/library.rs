//! Stage A (DESIGN §4.0, ADR-0013): read an acquired library.
//!
//! A library is a committed uv project, `libraries/<name>/`: one pinned requirement, `[tool.lctx]`
//! naming its first-party distributions, `.python-version`, and a `uv.lock` recording every
//! distribution of the closure with its artifacts' sha256. `lctx acquire` materializes it with
//! `uv sync --frozen`. This module reads that project and environment, with no network and no
//! interpreter, under one equivalence: **the analyzer-readable bytes** (`.py`, `.pyi`, `py.typed`,
//! the files Pyrefly's module finder reads), each verified against its distribution's `RECORD`.
//! - The environment must be the lock's: every installed version is the locked one, and the
//!   interpreter is `.python-version`'s.
//! - Every analyzer-readable file of every distribution matches its `RECORD` sha256.
//! - The release's modules are exactly its distributions' `.py`/`.pyi` `RECORD` entries, and
//!   `release_id` hashes their names, versions and verified content, never the lock entry.
//! - A release distribution the lock records without artifact hashes (a git or local source) is
//!   refused: nothing would have verified it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cpg_schema::id::{Digest, IdHasher, content_digest, kind};
use sha2::{Digest as _, Sha256};

use crate::ExtractError;
use crate::config::{ExtractInput, Release, ReleaseOrigin, TestHooks};

/// A distribution installed in the environment.
#[derive(Debug, Clone, PartialEq)]
pub struct Distribution {
    /// PEP 503 normalized.
    pub name: String,
    pub version: String,
    /// sha256 (hex) of every artifact the lock records for this version, sorted.
    pub artifact_sha256: Vec<String>,
    pub record_digest: Digest,
}

/// The library an acquired release came from.
#[derive(Debug, Clone, PartialEq)]
pub struct AcquiredLibrary {
    /// The library directory's name (`libraries/<name>`).
    pub name: String,
    pub requirement: String,
    pub lock_digest: Digest,
    /// The release distributions as `name==version`, sorted.
    pub release: Vec<String>,
    /// The installer `pyvenv.cfg` names (`uv 0.12.18`).
    pub installer: Option<String>,
    pub distributions: Vec<Distribution>,
}

fn fail(msg: impl Into<String>) -> ExtractError {
    ExtractError::Library(msg.into())
}

/// PEP 503: lowercase, with runs of `-`, `_` and `.` as one `-`.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut sep = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            sep = true;
        } else {
            if sep && !out.is_empty() {
                out.push('-');
            }
            sep = false;
            out.push(c.to_ascii_lowercase());
        }
    }
    out
}

/// The files Pyrefly's module finder reads (`py.typed` changes how a package resolves).
pub fn analyzer_readable(path: &str) -> bool {
    path.ends_with(".py") || path.ends_with(".pyi") || path.rsplit('/').next() == Some("py.typed")
}

/// The distribution name a requirement names: the text before any extras, specifier or marker.
pub fn requirement_name(requirement: &str) -> String {
    normalize(
        requirement
            .split(|c: char| "[<>=!~ ;(@".contains(c))
            .next()
            .unwrap_or_default(),
    )
}

fn read(path: &Path) -> Result<String, ExtractError> {
    std::fs::read_to_string(path).map_err(|e| fail(format!("{}: {e}", path.display())))
}

/// The library definition: `[project].dependencies[0]`, `[tool.lctx]`.
struct Definition {
    requirement: String,
    release: Vec<String>,
    source: Option<toml::Table>,
}

fn definition(library_dir: &Path) -> Result<Definition, ExtractError> {
    let path = library_dir.join("pyproject.toml");
    let t = read(&path)?
        .parse::<toml::Table>()
        .map_err(|e| fail(format!("{}: {e}", path.display())))?;
    let requirement = t
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
        .and_then(|d| match d.as_slice() {
            [one] => one.as_str().map(str::to_owned),
            _ => None,
        })
        .ok_or_else(|| fail("pyproject.toml: [project] dependencies must be one requirement"))?;
    let lctx = t.get("tool").and_then(|t| t.get("lctx"));
    let release = lctx
        .and_then(|l| l.get("release"))
        .and_then(|r| r.as_array())
        .map(|r| {
            r.iter()
                .filter_map(|v| v.as_str().map(normalize))
                .collect::<Vec<_>>()
        })
        .filter(|r| !r.is_empty())
        .ok_or_else(|| fail("pyproject.toml: [tool.lctx] release must name distributions"))?;
    let source = lctx
        .and_then(|l| l.get("source"))
        .and_then(|s| s.as_table())
        .cloned();
    Ok(Definition {
        requirement,
        release,
        source,
    })
}

/// Name → (version, sorted artifact sha256s) from `uv.lock`, skipping the virtual project.
fn lock(bytes: &str) -> Result<BTreeMap<String, (String, Vec<String>)>, ExtractError> {
    let t = bytes
        .parse::<toml::Table>()
        .map_err(|e| fail(format!("uv.lock: {e}")))?;
    let mut out = BTreeMap::new();
    for p in t
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
    {
        let (Some(name), Some(version)) = (
            p.get("name").and_then(|v| v.as_str()),
            p.get("version").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        if p.get("source").and_then(|s| s.get("virtual")).is_some() {
            continue;
        }
        let mut artifacts: Vec<String> = p
            .get("wheels")
            .and_then(|w| w.as_array())
            .into_iter()
            .flatten()
            .chain(p.get("sdist"))
            .filter_map(|a| a.get("hash").and_then(|h| h.as_str()))
            .filter_map(|h| h.strip_prefix("sha256:").map(str::to_owned))
            .collect();
        artifacts.sort();
        artifacts.dedup();
        out.insert(normalize(name), (version.to_owned(), artifacts));
    }
    Ok(out)
}

/// `key = value` lines of `pyvenv.cfg`.
fn pyvenv(env_dir: &Path) -> Result<BTreeMap<String, String>, ExtractError> {
    Ok(read(&env_dir.join("pyvenv.cfg"))?
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_owned(), v.trim().to_owned()))
        })
        .collect())
}

fn version_triple(v: &str) -> Result<(u32, u32, u32), ExtractError> {
    let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
    match parts[..] {
        [a, b, c, ..] => Ok((a, b, c)),
        _ => Err(fail(format!("python version {v}"))),
    }
}

/// One `RECORD` entry inside site-packages: its path and sha256, when recorded. Entries outside
/// site-packages (`../../../bin/…`) carry the environment's location and are dropped.
fn record_entries(record: &str) -> Vec<(String, Option<String>)> {
    record
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let path = fields.next()?.trim();
            if path.is_empty() || path.starts_with("..") {
                return None;
            }
            let hash = fields
                .next()
                .and_then(|h| h.strip_prefix("sha256="))
                .map(str::to_owned);
            Some((path.to_owned(), hash))
        })
        .collect()
}

/// Installed distributions: normalized name → (version, dist-info directory).
fn installed(site_packages: &Path) -> Result<BTreeMap<String, (String, PathBuf)>, ExtractError> {
    let mut out = BTreeMap::new();
    let entries = std::fs::read_dir(site_packages)
        .map_err(|e| fail(format!("{}: {e}", site_packages.display())))?;
    for entry in entries {
        let path = entry.map_err(|e| fail(e.to_string()))?.path();
        let Some(stem) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".dist-info"))
        else {
            continue;
        };
        let (name, version) = stem
            .rsplit_once('-')
            .ok_or_else(|| fail(format!("dist-info name {stem}")))?;
        out.insert(normalize(name), (version.to_owned(), path));
    }
    Ok(out)
}

/// `[tool.lctx.source]`, when declared, pins the upstream tree by a full commit, and its tag names
/// the locked version of the requested distribution (so docs never drift from code).
fn check_source(
    source: Option<&toml::Table>,
    requested: &str,
    locked: &BTreeMap<String, (String, Vec<String>)>,
) -> Result<(), ExtractError> {
    let Some(source) = source else {
        return Ok(());
    };
    let commit = source.get("commit").and_then(|c| c.as_str()).unwrap_or("");
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail(
            "[tool.lctx.source] must pin `commit` as 40 hex digits",
        ));
    }
    let tag = source.get("tag").and_then(|t| t.as_str()).unwrap_or("");
    let version = locked.get(requested).map(|(v, _)| v.as_str()).unwrap_or("");
    if version.is_empty() || !tag.contains(version) {
        return Err(fail(format!(
            "[tool.lctx.source] tag {tag:?} does not name the locked {requested} {version}"
        )));
    }
    Ok(())
}

/// Build the extraction input for an acquired library: `library_dir` holds the definition and
/// lock, `env_dir` the environment `uv sync --frozen` built from them. Both absolute.
pub fn acquired(
    library_dir: &Path,
    env_dir: &Path,
    snapshot_id: cpg_schema::id::Id,
) -> Result<ExtractInput, ExtractError> {
    let name = library_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| fail("library directory has no name"))?
        .to_owned();
    let def = definition(library_dir)?;
    let lock_text = read(&library_dir.join("uv.lock"))?;
    let locked = lock(&lock_text)?;
    check_source(
        def.source.as_ref(),
        &requirement_name(&def.requirement),
        &locked,
    )?;
    let remedy = format!("run `lctx acquire {name} --reinstall`");

    let cfg = pyvenv(env_dir)?;
    let installed_python = cfg
        .get("version_info")
        .ok_or_else(|| fail("pyvenv.cfg has no version_info"))?;
    let pinned_python = read(&library_dir.join(".python-version"))?
        .trim()
        .to_owned();
    if *installed_python != pinned_python {
        return Err(fail(format!(
            "the environment runs Python {installed_python}, .python-version pins {pinned_python}; {remedy}"
        )));
    }
    let python = version_triple(installed_python)?;
    let site_packages = env_dir
        .join("lib")
        .join(format!("python{}.{}", python.0, python.1))
        .join("site-packages");
    let dists = installed(&site_packages)?;

    let mut release = def.release.clone();
    release.sort();
    for dist in &release {
        let (_, artifacts) = locked
            .get(dist)
            .ok_or_else(|| fail(format!("release distribution {dist} is not in uv.lock")))?;
        if artifacts.is_empty() {
            return Err(fail(format!(
                "release distribution {dist} is locked without artifact hashes (a git or local \
                 source); ADR-0013 compiles only hash-verified releases"
            )));
        }
        if !dists.contains_key(dist) {
            return Err(fail(format!(
                "release distribution {dist} is not installed; {remedy}"
            )));
        }
    }

    let mut files = Vec::new();
    let mut release_content: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
    let mut distributions = Vec::new();
    for (dist, (version, dist_info)) in &dists {
        match locked.get(dist) {
            Some((locked_version, _)) if locked_version == version => {}
            Some((locked_version, _)) => {
                return Err(fail(format!(
                    "{dist} {version} is installed but uv.lock has {locked_version}; {remedy}"
                )));
            }
            None => {
                return Err(fail(format!(
                    "{dist} {version} is installed but not in uv.lock; {remedy}"
                )));
            }
        }
        let record_bytes = std::fs::read(dist_info.join("RECORD"))
            .map_err(|e| fail(format!("{}: {e}", dist_info.display())))?;
        let in_release = release.contains(dist);
        for (path, hash) in record_entries(&String::from_utf8_lossy(&record_bytes)) {
            if !analyzer_readable(&path) {
                continue;
            }
            let expected =
                hash.ok_or_else(|| fail(format!("{dist}: {path} has no RECORD hash")))?;
            let file = site_packages.join(&path);
            let bytes =
                std::fs::read(&file).map_err(|e| fail(format!("{dist}: {path}: {e}; {remedy}")))?;
            if URL_SAFE_NO_PAD.encode(Sha256::digest(&bytes)) != expected {
                return Err(fail(format!(
                    "{dist}: {path} does not match its RECORD sha256; {remedy}"
                )));
            }
            if in_release {
                if path.ends_with(".py") || path.ends_with(".pyi") {
                    files.push(file);
                }
                release_content
                    .entry(dist.as_str())
                    .or_default()
                    .push((path, expected));
            }
        }
        distributions.push(Distribution {
            name: dist.clone(),
            version: version.clone(),
            artifact_sha256: locked.get(dist).map(|(_, a)| a.clone()).unwrap_or_default(),
            record_digest: content_digest(&record_bytes),
        });
    }
    files.sort();

    // release_id: what is analyzed, so a re-listed artifact or a lock-only change leaves it.
    let mut hasher = IdHasher::new(kind::RELEASE);
    hasher.str("library").i64(release.len() as i64);
    for dist in &release {
        let version = &dists[dist].0;
        let mut entries = release_content.remove(dist.as_str()).unwrap_or_default();
        entries.sort();
        hasher.str(dist).str(version).i64(entries.len() as i64);
        for (path, sha) in &entries {
            hasher.str(path).str(sha);
        }
    }

    Ok(ExtractInput {
        release: Release {
            root: site_packages.clone(),
            files,
            release_id: hasher.finish_id(),
            origin: ReleaseOrigin::Library(AcquiredLibrary {
                name,
                requirement: def.requirement,
                lock_digest: content_digest(lock_text.as_bytes()),
                release: release
                    .iter()
                    .map(|d| format!("{d}=={}", dists[d].0))
                    .collect(),
                installer: cfg.get("uv").map(|v| format!("uv {v}")),
                distributions,
            }),
        },
        venv_root: env_dir.to_path_buf(),
        site_packages: vec![site_packages],
        python_version: python,
        python_platform: std::env::consts::OS.to_owned(),
        snapshot_id,
        keep_pysa_json: false,
        test_hooks: TestHooks::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize_as_pep_503() {
        assert_eq!(normalize("fastmcp_slim"), "fastmcp-slim");
        assert_eq!(normalize("Foo.Bar--baz"), "foo-bar-baz");
        assert_eq!(requirement_name("fastmcp[tasks]==4.0.5"), "fastmcp");
        assert_eq!(requirement_name("foo @ git+https://x"), "foo");
    }

    #[test]
    fn analyzer_readable_files_are_what_pyrefly_reads() {
        assert!(analyzer_readable("pkg/mod.py"));
        assert!(analyzer_readable("pkg/mod.pyi"));
        assert!(analyzer_readable("pkg/py.typed"));
        assert!(!analyzer_readable("pkg/data.json"));
        assert!(!analyzer_readable("_virtualenv.pth"));
    }
}
