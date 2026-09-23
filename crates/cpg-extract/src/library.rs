//! Stage A (DESIGN §4.0, ADR-0013): read an acquired library.
//!
//! A library is a committed uv project, `libraries/<name>/`: a pinned requirement, `[tool.lctx]`
//! naming its first-party distributions, and a `uv.lock` recording every distribution of the
//! closure with its artifacts' sha256. `lctx acquire` materializes it with `uv sync --frozen`.
//! This module reads that project and environment, with no network and no interpreter:
//! - every installed distribution must be the version the lock names;
//! - every file of a release distribution is checked against its `RECORD` sha256;
//! - the release's modules are exactly the `.py`/`.pyi` entries of those `RECORD`s;
//! - `release_id` hashes the release distributions' names, versions and locked artifact hashes.

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
    /// One of the library's first-party distributions.
    pub in_release: bool,
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

fn read(path: &Path) -> Result<String, ExtractError> {
    std::fs::read_to_string(path).map_err(|e| fail(format!("{}: {e}", path.display())))
}

fn table(path: &Path) -> Result<toml::Table, ExtractError> {
    read(path)?
        .parse::<toml::Table>()
        .map_err(|e| fail(format!("{}: {e}", path.display())))
}

/// `[project].dependencies[0]` and `[tool.lctx].release` of a library definition.
fn definition(library_dir: &Path) -> Result<(String, Vec<String>), ExtractError> {
    let t = table(&library_dir.join("pyproject.toml"))?;
    let requirement = t
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
        .and_then(|d| match d.as_slice() {
            [one] => one.as_str().map(str::to_owned),
            _ => None,
        })
        .ok_or_else(|| fail("pyproject.toml: [project] dependencies must be one requirement"))?;
    let release = t
        .get("tool")
        .and_then(|t| t.get("lctx"))
        .and_then(|l| l.get("release"))
        .and_then(|r| r.as_array())
        .map(|r| {
            r.iter()
                .filter_map(|v| v.as_str().map(normalize))
                .collect::<Vec<_>>()
        })
        .filter(|r| !r.is_empty())
        .ok_or_else(|| fail("pyproject.toml: [tool.lctx] release must name distributions"))?;
    Ok((requirement, release))
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

/// `version_info` from the environment's `pyvenv.cfg`.
fn python_version(env_dir: &Path) -> Result<(u32, u32, u32), ExtractError> {
    let cfg = read(&env_dir.join("pyvenv.cfg"))?;
    let v = cfg
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "version_info").then(|| v.trim().to_owned())
        })
        .ok_or_else(|| fail("pyvenv.cfg has no version_info"))?;
    let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
    match parts[..] {
        [a, b, c, ..] => Ok((a, b, c)),
        _ => Err(fail(format!("pyvenv.cfg version_info {v}"))),
    }
}

/// One `RECORD` entry: a path relative to site-packages and its sha256, when recorded.
fn record_entries(record: &str) -> Vec<(String, Option<String>)> {
    record
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let path = fields.next()?.trim();
            if path.is_empty() {
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
    let (requirement, release) = definition(library_dir)?;
    let lock_text = read(&library_dir.join("uv.lock"))?;
    let locked = lock(&lock_text)?;
    let python = python_version(env_dir)?;
    let site_packages = env_dir
        .join("lib")
        .join(format!("python{}.{}", python.0, python.1))
        .join("site-packages");
    let dists = installed(&site_packages)?;

    for (dist, (version, _)) in &dists {
        match locked.get(dist) {
            Some((locked_version, _)) if locked_version == version => {}
            Some((locked_version, _)) => {
                return Err(fail(format!(
                    "{dist} {version} is installed but uv.lock has {locked_version}; run `lctx acquire`"
                )));
            }
            None => {
                return Err(fail(format!(
                    "{dist} {version} is installed but not in uv.lock"
                )));
            }
        }
    }

    let mut files = Vec::new();
    let mut hasher = IdHasher::new(kind::RELEASE);
    hasher.str("library").i64(release.len() as i64);
    let mut sorted_release = release.clone();
    sorted_release.sort();
    for dist in &sorted_release {
        let (version, dist_info) = dists
            .get(dist)
            .ok_or_else(|| fail(format!("release distribution {dist} is not installed")))?;
        let (_, artifacts) = &locked[dist];
        hasher
            .str(dist)
            .str(version)
            .strs(artifacts.iter().map(String::as_str));
        let record = read(&dist_info.join("RECORD"))?;
        for (path, hash) in record_entries(&record) {
            if path.starts_with("..") {
                continue;
            }
            let file = site_packages.join(&path);
            if let Some(expected) = hash {
                let bytes =
                    std::fs::read(&file).map_err(|e| fail(format!("{dist}: {path}: {e}")))?;
                if URL_SAFE_NO_PAD.encode(Sha256::digest(&bytes)) != expected {
                    return Err(fail(format!(
                        "{dist}: {path} does not match its RECORD sha256; run `lctx acquire`"
                    )));
                }
            }
            if path.ends_with(".py") || path.ends_with(".pyi") {
                files.push(file);
            }
        }
    }
    files.sort();

    let distributions = dists
        .iter()
        .map(|(dist, (version, dist_info))| {
            let record = std::fs::read(dist_info.join("RECORD"))
                .map_err(|e| fail(format!("{}: {e}", dist_info.display())))?;
            Ok(Distribution {
                name: dist.clone(),
                version: version.clone(),
                in_release: release.contains(dist),
                artifact_sha256: locked.get(dist).map(|(_, a)| a.clone()).unwrap_or_default(),
                record_digest: content_digest(&record),
            })
        })
        .collect::<Result<Vec<_>, ExtractError>>()?;

    Ok(ExtractInput {
        release: Release {
            root: site_packages.clone(),
            files,
            release_id: hasher.finish_id(),
            origin: ReleaseOrigin::Library(AcquiredLibrary {
                name,
                requirement,
                lock_digest: content_digest(lock_text.as_bytes()),
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
    }
}
