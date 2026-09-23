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
use cpg_schema::codebook::SourceRole;
use cpg_schema::id::{Digest, IdHasher, content_digest, kind};
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;
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
    /// Every verified analyzer-readable file, site-relative, → the distribution whose `RECORD`
    /// lists it: `source_files.distribution` and `context_modules.distribution` (ADR-0014).
    pub owners: BTreeMap<String, String>,
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
    fs_err::read_to_string(path).map_err(|e| fail(format!("{}: {e}", path.display())))
}

/// `pyproject.toml` as Stage A reads it (H1 C3). `[tool.lctx]` and its `source` table are ours and
/// refuse unknown keys, so a misspelling (`[tool.lctx.sourse]`) fails, naming it, instead of
/// silently disabling what it names; `[project]` and other tools' tables belong to their owners
/// and stay open.
#[derive(Deserialize)]
struct PyProject {
    project: Project,
    tool: Tools,
}

#[derive(Deserialize)]
struct Project {
    dependencies: Vec<String>,
}

#[derive(Deserialize)]
struct Tools {
    lctx: Lctx,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lctx {
    release: Vec<String>,
    source: Option<Source>,
}

/// The library definition: `[project].dependencies[0]`, `[tool.lctx]`.
struct Definition {
    requirement: String,
    release: Vec<String>,
    source: Option<Source>,
}

fn parse_definition(text: &str) -> Result<Definition, String> {
    let p: PyProject = toml::from_str(text).map_err(|e| e.to_string())?;
    let [requirement] = <[String; 1]>::try_from(p.project.dependencies)
        .map_err(|_| "[project] dependencies must be one requirement".to_owned())?;
    let release: Vec<String> = p.tool.lctx.release.iter().map(|r| normalize(r)).collect();
    if release.is_empty() {
        return Err("[tool.lctx] release must name distributions".to_owned());
    }
    Ok(Definition {
        requirement,
        release,
        source: p.tool.lctx.source,
    })
}

fn definition(library_dir: &Path) -> Result<Definition, ExtractError> {
    let path = library_dir.join("pyproject.toml");
    parse_definition(&read(&path)?).map_err(|e| fail(format!("{}: {e}", path.display())))
}

/// `uv.lock`, as far as Stage A reads it. uv owns the format, so unknown keys pass.
#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Deserialize)]
struct LockPackage {
    name: String,
    version: Option<String>,
    source: Option<LockSource>,
    #[serde(default)]
    wheels: Vec<LockArtifact>,
    sdist: Option<LockArtifact>,
}

#[derive(Deserialize)]
struct LockSource {
    #[serde(rename = "virtual")]
    virtual_path: Option<String>,
}

#[derive(Deserialize)]
struct LockArtifact {
    hash: Option<String>,
}

/// Name → (version, sorted artifact sha256s) from `uv.lock`, skipping the virtual project.
fn lock(bytes: &str) -> Result<BTreeMap<String, (String, Vec<String>)>, ExtractError> {
    let lock: Lock = toml::from_str(bytes).map_err(|e| fail(format!("uv.lock: {e}")))?;
    let mut out = BTreeMap::new();
    for p in lock.package {
        let Some(version) = p.version else {
            continue;
        };
        if p.source.is_some_and(|s| s.virtual_path.is_some()) {
            continue;
        }
        let mut artifacts: Vec<String> = p
            .wheels
            .iter()
            .chain(&p.sdist)
            .filter_map(|a| a.hash.as_deref())
            .filter_map(|h| h.strip_prefix("sha256:").map(str::to_owned))
            .collect();
        artifacts.sort();
        artifacts.dedup();
        out.insert(normalize(&p.name), (version, artifacts));
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
///
/// `RECORD` is CSV (PEP 376): a path holding `,` or `"` is quoted, so it is read as CSV, never split
/// on commas (H1 C5).
fn record_entries(record: &[u8]) -> Result<Vec<(String, Option<String>)>, csv::Error> {
    let mut entries = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(record);
    for row in reader.records() {
        let row = row?;
        let Some(path) = row.get(0).map(str::trim) else {
            continue;
        };
        if path.is_empty() || path.starts_with("..") {
            continue;
        }
        let hash = row
            .get(1)
            .and_then(|h| h.strip_prefix("sha256="))
            .map(str::to_owned);
        entries.push((path.to_owned(), hash));
    }
    Ok(entries)
}

/// Installed distributions: normalized name → (version, dist-info directory).
fn installed(site_packages: &Path) -> Result<BTreeMap<String, (String, PathBuf)>, ExtractError> {
    let mut out = BTreeMap::new();
    let entries = fs_err::read_dir(site_packages)
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
    source: Option<&Source>,
    requested: &str,
    locked: &BTreeMap<String, (String, Vec<String>)>,
) -> Result<(), ExtractError> {
    let Some(source) = source else {
        return Ok(());
    };
    let commit = source.commit.as_str();
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail(
            "[tool.lctx.source] must pin `commit` as 40 hex digits",
        ));
    }
    let tag = source.tag.as_str();
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
    let mut owners = BTreeMap::new();
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
        let record_bytes = fs_err::read(dist_info.join("RECORD"))
            .map_err(|e| fail(format!("{}: {e}", dist_info.display())))?;
        let in_release = release.contains(dist);
        let entries = record_entries(&record_bytes)
            .map_err(|e| fail(format!("{}/RECORD: {e}", dist_info.display())))?;
        for (path, hash) in entries {
            if !analyzer_readable(&path) {
                continue;
            }
            let expected =
                hash.ok_or_else(|| fail(format!("{dist}: {path} has no RECORD hash")))?;
            let file = site_packages.join(&path);
            let bytes =
                fs_err::read(&file).map_err(|e| fail(format!("{dist}: {path}: {e}; {remedy}")))?;
            if URL_SAFE_NO_PAD.encode(Sha256::digest(&bytes)) != expected {
                return Err(fail(format!(
                    "{dist}: {path} does not match its RECORD sha256; {remedy}"
                )));
            }
            owners.insert(path.clone(), dist.clone());
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
                owners,
            }),
        },
        venv_root: env_dir.to_path_buf(),
        site_packages: vec![site_packages],
        python_version: python,
        python_platform: std::env::consts::OS.to_owned(),
        snapshot_id,
        corpus: None,
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

/// A library's upstream tree (`[tool.lctx.source]`, C5): where it is, at which commit, and which of
/// its files are the corpus. The selections are globs from the tree's root: `**` spans
/// directories, `*` stays within one name.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub repository: String,
    pub tag: String,
    pub commit: String,
    #[serde(default)]
    pub documents: Vec<String>,
    #[serde(default)]
    pub documents_exclude: Vec<String>,
    /// Official example code (`source_role` `example`, ADR-0015).
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub examples_exclude: Vec<String>,
    /// The library's own tests (`source_role` `test`).
    #[serde(default)]
    pub tests: Vec<String>,
    /// Also what excludes a directory link under the tests (ADR-0018).
    #[serde(default)]
    pub tests_exclude: Vec<String>,
}

/// The declared `[tool.lctx.source]`, checked as Stage A checks it; `None` when not declared. Its
/// keys are typed (a string where a list belongs is refused) and unknown ones are refused (C5 review
/// F5; H1 C3).
pub fn source(library_dir: &Path) -> Result<Option<Source>, ExtractError> {
    Ok(definition(library_dir)?.source)
}

/// A tree walked once (H1 C2): every file outside dot-directories, relative (`/`-separated) and
/// absolute, in path order, and every symlink met there. A symlink is never followed.
struct Walked {
    files: Vec<(String, PathBuf)>,
    /// `(relative path, is a directory link)`.
    symlinks: Vec<(String, bool)>,
}

fn walk_tree(root: &Path) -> Result<Walked, ExtractError> {
    let mut walked = Walked {
        files: Vec::new(),
        symlinks: Vec::new(),
    };
    let entries = walkdir::WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'));
    for entry in entries {
        let entry = entry.map_err(|e| fail(format!("walking {}: {e}", root.display())))?;
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| ExtractError::RelativePath(entry.path().to_path_buf()))?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let kind = entry.file_type();
        if kind.is_symlink() {
            walked.symlinks.push((rel, entry.path().is_dir()));
        } else if kind.is_file() {
            walked.files.push((rel, entry.into_path()));
        }
    }
    Ok(walked)
}

/// A key's globs, compiled by globset: `*` and `?` stay within one name, `**` spans directories,
/// `[…]` and `{a,b}` as globset reads them (`literal_separator`; DESIGN §4.0).
fn glob_set(key: &str, patterns: &[String]) -> Result<(GlobSet, Vec<Glob>), ExtractError> {
    let mut set = GlobSetBuilder::new();
    let mut globs = Vec::new();
    for p in patterns {
        let glob = GlobBuilder::new(p)
            .literal_separator(true)
            .build()
            .map_err(|e| fail(format!("[tool.lctx.source] `{key}` glob `{p}`: {e}")))?;
        set.add(glob.clone());
        globs.push(glob);
    }
    let set = set
        .build()
        .map_err(|e| fail(format!("[tool.lctx.source] `{key}`: {e}")))?;
    Ok((set, globs))
}

/// The path before a glob's first metacharacter, as whole segments: `docs` for `docs/**/*.mdx`,
/// empty for `**/*.py`.
fn literal_prefix(glob: &str) -> String {
    glob.split('/')
        .take_while(|seg| !seg.contains(['*', '?', '[', '{']))
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether one of two relative paths lies at or under the other.
fn nested(a: &str, b: &str) -> bool {
    a.is_empty()
        || b.is_empty()
        || a == b
        || a.starts_with(&format!("{b}/"))
        || b.starts_with(&format!("{a}/"))
}

/// The files of `walked` some `include` glob matches and no `exclude` glob does, absolute and in
/// path order. Fails closed (H1 C2):
/// - each include glob must select a file, so an upstream move of `docs/` fails instead of
///   publishing an empty selection (C5 review F5);
/// - a symlink an include glob matches is refused, naming it: it would read outside the tree;
/// - a directory link that could hold a selection (it and an include glob's literal prefix lie one
///   under the other) is refused unless an exclude glob covers it (matches it, or a name directly
///   inside it): following it could escape the tree or loop, and skipping it would drop files
///   silently.
fn pick(
    tree: &Path,
    walked: &Walked,
    key: &str,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<PathBuf>, ExtractError> {
    let (inc, inc_globs) = glob_set(key, include)?;
    let (exc, _) = glob_set(&format!("{key}_exclude"), exclude)?;
    for (rel, is_dir) in &walked.symlinks {
        // An exclude covers a directory link only when it names the link itself, or matches any
        // name under it: two unrelated probe names, one nested. `docs/**/_*` matches a name
        // starting with `_` and leaves the rest selectable, so it covers nothing (H1 review F5).
        let covered = exc.is_match(rel)
            || (exc.is_match(format!("{rel}/lctx-link-probe"))
                && exc.is_match(format!("{rel}/lctx-link-probe/lctx-link-probe.x")));
        let refused = if *is_dir {
            !covered
                && inc_globs
                    .iter()
                    .any(|g| nested(rel, &literal_prefix(g.glob())))
        } else {
            inc.is_match(rel) && !exc.is_match(rel)
        };
        if refused {
            return Err(fail(format!(
                "[tool.lctx.source] `{key}` would read through the symlink {rel} in {}; a fetched \
                 tree is read without following links",
                tree.display()
            )));
        }
    }
    let mut hits = vec![0usize; include.len()];
    let mut picked = Vec::new();
    for (rel, path) in &walked.files {
        let matched = inc.matches(rel);
        for g in &matched {
            hits[*g] += 1;
        }
        if !matched.is_empty() && !exc.is_match(rel) {
            picked.push(path.clone());
        }
    }
    if let Some(g) = hits.iter().position(|n| *n == 0) {
        return Err(fail(format!(
            "[tool.lctx.source] glob `{}` selects nothing in {}",
            include[g],
            tree.display()
        )));
    }
    Ok(picked)
}

/// The corpus run's input (C5): the fetched `tree`'s selected documents, and its usage modules (the
/// selected examples and tests, and each document's Python code blocks, written as modules of their
/// own under `<tree>/_lctx_blocks/`, which is cleared first), in the environment of `library` (the
/// library run's input). The blocks derive from the documents, so the release id, which hashes the
/// documents, already covers them.
pub fn corpus(
    tree: &Path,
    source: &Source,
    library: &ExtractInput,
) -> Result<crate::config::CorpusInput, ExtractError> {
    // The previous compile's materialized blocks go first, so no glob can select them (C6 review
    // O4).
    let blocks = tree.join("_lctx_blocks");
    if blocks.exists() {
        fs_err::remove_dir_all(&blocks)?;
    }
    let walked = walk_tree(tree)?;
    let documents = pick(
        tree,
        &walked,
        "documents",
        &source.documents,
        &source.documents_exclude,
    )?;
    let examples = pick(
        tree,
        &walked,
        "examples",
        &source.examples,
        &source.examples_exclude,
    )?;
    let tests = pick(tree, &walked, "tests", &source.tests, &source.tests_exclude)?;
    // A module has one role (ADR-0015): a file both keys select is refused, not ranked.
    if let Some(both) = examples.iter().find(|f| tests.contains(f)) {
        return Err(fail(format!(
            "[tool.lctx.source] `examples` and `tests` both select {}",
            both.display()
        )));
    }
    let mut usage: Vec<(PathBuf, SourceRole)> = examples
        .into_iter()
        .map(|f| (f, SourceRole::Example))
        .chain(tests.into_iter().map(|f| (f, SourceRole::Test)))
        .collect();
    for d in &documents {
        let rel = d
            .strip_prefix(tree)
            .map_err(|_| ExtractError::RelativePath(d.clone()))?
            .display()
            .to_string();
        for (ordinal, code) in crate::docs::python_blocks(&fs_err::read(d)?) {
            let module = tree.join(crate::docs::block_module_path(&rel, ordinal));
            if let Some(dir) = module.parent() {
                fs_err::create_dir_all(dir)?;
            }
            fs_err::write(&module, code)?;
            usage.push((module, SourceRole::DocBlock));
        }
    }
    let label = format!("{}@{}", source.repository, source.commit);
    let environment = match &library.release.origin {
        ReleaseOrigin::Library(l) => Some(l.clone()),
        ReleaseOrigin::Tree { .. } | ReleaseOrigin::Corpus { .. } => None,
    };
    let release = Release::corpus(
        tree.to_path_buf(),
        &label,
        &documents,
        usage,
        environment,
        library.release.release_id,
    )?;
    Ok(crate::config::CorpusInput { release, documents })
}

#[cfg(test)]
mod glob_tests {
    use super::{glob_set, literal_prefix, nested};

    fn matches(pattern: &str, path: &str) -> bool {
        glob_set("t", &[pattern.to_owned()])
            .unwrap()
            .0
            .is_match(path)
    }

    #[test]
    fn globs_span_directories_only_with_double_star() {
        assert!(matches("docs/**/*.mdx", "docs/a.mdx"));
        assert!(matches("docs/**/*.mdx", "docs/servers/tools.mdx"));
        assert!(!matches("docs/*.mdx", "docs/servers/tools.mdx"));
        assert!(matches("docs/v2/**", "docs/v2/servers/tools.mdx"));
        assert!(!matches("docs/v2/**", "docs/v20.mdx"));
        assert!(matches("tests/**/*.py", "tests/test_a.py"));
        assert!(!matches("tests/**/*.py", "tests/data/a.pyc"));
    }

    /// globset's full syntax (H1 C2): the hand matcher read these as literals.
    #[test]
    fn braces_classes_and_single_characters_select() {
        assert!(matches("docs/{guide,api}/*.mdx", "docs/api/a.mdx"));
        assert!(!matches("docs/{guide,api}/*.mdx", "docs/old/a.mdx"));
        assert!(matches("tests/test_?.py", "tests/test_a.py"));
        assert!(!matches("tests/test_?.py", "tests/test_ab.py"));
        assert!(matches("examples/[ab]*.py", "examples/bot.py"));
        assert!(!matches("examples/[ab]*.py", "examples/cat.py"));
    }

    /// The hand matcher backtracked exponentially (14 stars ran for minutes); globset compiles to
    /// a regex.
    #[test]
    fn many_stars_match_in_linear_time() {
        let pattern = "*a*a*a*a*a*a*a*a*a*a*a*a*a*a*b";
        let started = std::time::Instant::now();
        assert!(!matches(pattern, &"a".repeat(60)));
        assert!(started.elapsed().as_secs() < 2);
    }

    #[test]
    fn a_link_can_hold_a_selection_when_nested_with_a_prefix() {
        assert_eq!(literal_prefix("docs/**/*.mdx"), "docs");
        assert_eq!(literal_prefix("**/*.py"), "");
        assert!(nested("docs/linked", "docs"));
        assert!(nested("docs", "docs/api"));
        assert!(!nested("examples/x", "docs"));
        assert!(nested("anything", ""));
    }
}

#[cfg(test)]
mod definition_tests {
    use super::{lock, parse_definition};

    const BASE: &str =
        "[project]\nname = \"x\"\ndependencies = [\"pkg==1\"]\n\n[tool.uv]\npackage = false\n\n";

    fn parsed(tail: &str) -> Result<super::Definition, String> {
        parse_definition(&format!("{BASE}{tail}"))
    }

    #[test]
    fn the_pilot_definition_and_lock_read_as_before() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/fastmcp");
        let def =
            parse_definition(&fs_err::read_to_string(dir.join("pyproject.toml")).unwrap()).unwrap();
        assert_eq!(def.release, ["fastmcp", "fastmcp-slim", "fastmcp-tasks"]);
        let source = def.source.unwrap();
        assert_eq!(source.tag, "v4.0.5");
        assert_eq!(source.tests, ["tests/**/*.py"]);
        let locked = lock(&fs_err::read_to_string(dir.join("uv.lock")).unwrap()).unwrap();
        assert_eq!(locked.len(), 108);
        assert!(locked.values().all(|(_, hashes)| !hashes.is_empty()));
    }

    /// A misspelled table or key names itself instead of silently disabling the corpus (H1 C3).
    #[test]
    fn unknown_lctx_keys_and_mistyped_values_are_refused() {
        let err =
            parsed("[tool.lctx]\nrelease = [\"pkg\"]\n\n[tool.lctx.sourse]\nrepository = \"r\"\n")
                .err()
                .unwrap();
        assert!(err.contains("sourse"), "{err}");
        let src = "[tool.lctx]\nrelease = [\"pkg\"]\n\n[tool.lctx.source]\nrepository = \"r\"\ntag = \"t\"\ncommit = \"c\"\n";
        let err = parsed(&format!("{src}usage = [\"x\"]\n")).err().unwrap();
        assert!(err.contains("usage"), "{err}");
        let err = parsed(&format!("{src}documents = \"docs/**\"\n"))
            .err()
            .unwrap();
        assert!(
            err.contains("documents") || err.contains("sequence"),
            "{err}"
        );
        let err = parsed("[tool.lctx]\nrelease = []\n").err().unwrap();
        assert!(err.contains("release"), "{err}");
        assert!(parsed(&format!("{src}documents = [\"docs/**\"]\n")).is_ok());
    }
}
