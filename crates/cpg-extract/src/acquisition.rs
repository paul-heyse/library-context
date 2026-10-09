//! Acquisition (cutover plan A2, T6; ADR-0089). A pure inventory reads the library's committed
//! project, its acquired environment and its fetched source tree, and writes nothing. Capture
//! freezes that inventory, verifies every frozen byte a `RECORD` lists, and derives Markdown Python
//! blocks into the frozen copy's reserved `_lctx/` namespace. The `acquire` stage states every
//! acquisition relation from the captured inputs.
//!
//! The closure is what the analyzers read: site-packages sources, stubs, `py.typed` and `.pth`
//! files, each distribution's `METADATA` and `entry_points.txt`, and the corpus's selected files
//! plus every analyzer-readable file it can import. A `RECORD` is verification evidence, not
//! analyzer input: its console-script lines carry the environment's location, so its in-site
//! entries are digested into the distribution's verification and its bytes are not captured.
use lctx_model::domain::producer_contract::{ProducerContract, ConfigurationBinding};
use crate::contracts::supplier;
use crate::bundle::ProviderSink;
use crate::{
    ExtractError,
    bundle::{self, CapturedInputs, Declared, ProviderStage, StageContext},
    capture::{CaptureError, CapturedInput, Derived},
    library::{self, fail},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use lctx_model::domain::{
    ContentHash, KeySink, ModelError, Record,
    artifact::ArtifactChunk,
    attribution::{FactFamily, Provider},
    input::*,
    resources::ResourceBudget,
    source::SourceArtifact,
    stages::{Effect, Profile, ProviderOutcome, RelationUse, Stage},
};
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

pub const ACQUIRE: &str = "acquire";

/// Whether a site-packages file belongs to the analyzer-readable closure: what Pyrefly's module
/// finder reads, and the `.pth` files that extend its search path.
pub fn in_closure(path: &str) -> bool {
    library::analyzer_readable(path) || path.ends_with(".pth")
}
/// A distribution's captured metadata: `METADATA` and `entry_points.txt` directly in its dist-info.
fn metadata_file(path: &str) -> bool {
    matches!(path.rsplit_once('/'), Some((dir, "METADATA" | "entry_points.txt")) if dir.ends_with(".dist-info") && !dir.contains('/'))
}

/// One installed distribution, as its lock pins it and its `RECORD` verifies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryDistribution {
    /// PEP 503 normalized.
    pub name: String,
    pub version: String,
    pub first_party: bool,
    /// The lock's artifact sha256s, hex, sorted.
    pub artifact_sha256: Vec<String>,
    /// The `RECORD`'s in-site entries, sorted: location independent.
    pub record_digest: ContentHash,
}
/// One file of the environment closure, site-relative, with the distributions whose `RECORD`
/// lists it (none for a loose file), its role and the sha256 its `RECORD` states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryFile {
    pub path: String,
    pub owners: Vec<String>,
    pub role: SourceRole,
    pub record_sha256: Option<String>,
}
/// An acquired library environment.
#[derive(Debug, Clone)]
pub struct LibraryInventory {
    pub name: String,
    pub requirement: String,
    pub lock_digest: ContentHash,
    pub installer: Option<String>,
    pub python_version: String,
    pub platform: String,
    pub site_packages: PathBuf,
    pub distributions: Vec<InventoryDistribution>,
    pub files: Vec<InventoryFile>,
    /// The acquisition configuration: the definition, lock and interpreter pin.
    pub configuration: ContentHash,
}
/// The library's fetched source tree and what the definition selects from it.
#[derive(Debug, Clone)]
pub struct CorpusInventory {
    pub repository: String,
    pub commit: String,
    pub root: PathBuf,
    /// Every captured path and its roles; an importable helper has none.
    pub files: BTreeMap<String, Vec<SourceRole>>,
    pub documents: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct InputInventory {
    pub library: LibraryInventory,
    pub corpus: Option<CorpusInventory>,
}

#[derive(Debug, thiserror::Error)]
pub enum AcquireError {
    #[error(transparent)]
    Extract(#[from] ExtractError),
    #[error(transparent)]
    Capture(#[from] CaptureError),
}

/// Read what one attempt acquires, without writing anything. `source_tree` is the fetched tree the
/// definition's `[tool.lctx.source]` names; it is required exactly when one is declared.
/// Inventory only the installed input, for captured deployment identity without corpus analysis.
pub fn inventory_installed(
    library_dir: &Path,
    env_dir: &Path,
) -> Result<InputInventory, ExtractError> {
    let definition = library::definition(library_dir)?;
    Ok(InputInventory {
        library: library_inventory(library_dir, env_dir, &definition)?,
        corpus: None,
    })
}
pub fn inventory(
    library_dir: &Path,
    env_dir: &Path,
    source_tree: Option<&Path>,
) -> Result<InputInventory, ExtractError> {
    let definition = library::definition(library_dir)?;
    let library = library_inventory(library_dir, env_dir, &definition)?;
    let corpus = match (&definition.source, source_tree) {
        (None, None) => None,
        (None, Some(_)) => {
            return Err(fail(
                "a source tree was given, but the definition declares no [tool.lctx.source]",
            ));
        }
        (Some(source), Some(tree)) => Some(corpus_inventory(tree, source)?),
        (Some(_), None) => {
            return Err(fail(
                "the definition declares [tool.lctx.source]; its fetched tree is required (run `lctx acquire`)",
            ));
        }
    };
    Ok(InputInventory { library, corpus })
}

fn library_inventory(
    library_dir: &Path,
    env_dir: &Path,
    definition: &library::Definition,
) -> Result<LibraryInventory, ExtractError> {
    let name = library_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| fail("library directory has no name"))?
        .to_owned();
    let pyproject = library::read(&library_dir.join("pyproject.toml"))?;
    let lock_text = library::read(&library_dir.join("uv.lock"))?;
    let pinned_python = library::read(&library_dir.join(".python-version"))?
        .trim()
        .to_owned();
    let locked = library::lock(&lock_text)?;
    library::check_source(
        definition.source.as_ref(),
        &library::requirement_name(&definition.requirement),
        &locked,
    )?;
    let remedy = format!("run `lctx acquire {name} --reinstall`");
    let venv = library::pyvenv(env_dir)?;
    let python = venv
        .get("version_info")
        .ok_or_else(|| fail("pyvenv.cfg has no version_info"))?
        .clone();
    if python != pinned_python {
        return Err(fail(format!(
            "the environment runs Python {python}, .python-version pins {pinned_python}; {remedy}"
        )));
    }
    let (major, minor, _) = library::version_triple(&python)?;
    let site = env_dir
        .join("lib")
        .join(format!("python{major}.{minor}"))
        .join("site-packages");
    let installed = library::installed(&site)?;
    let mut release = definition.release.clone();
    release.sort();
    for dist in &release {
        let (_, artifacts) = locked
            .get(dist)
            .ok_or_else(|| fail(format!("release distribution {dist} is not in uv.lock")))?;
        if artifacts.is_empty() {
            return Err(fail(format!(
                "release distribution {dist} is locked without artifact hashes (a git or local source); \
                only hash-verified releases are compiled"
            )));
        }
        if !installed.contains_key(dist) {
            return Err(fail(format!(
                "release distribution {dist} is not installed; {remedy}"
            )));
        }
    }
    let mut listed: BTreeMap<String, (Vec<String>, String)> = BTreeMap::new();
    let mut distributions = Vec::new();
    for (dist, (version, info)) in &installed {
        let artifacts = match locked.get(dist) {
            Some((locked_version, artifacts)) if locked_version == version => artifacts,
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
        };
        let record = fs_err::read(info.join("RECORD"))
            .map_err(|e| fail(format!("{}: {e}", info.display())))?;
        let mut entries = library::record_entries(&record)
            .map_err(|e| fail(format!("{}/RECORD: {e}", info.display())))?;
        entries.sort();
        let mut digest = KeySink::new("record-entries");
        for (path, hash) in &entries {
            digest.part(b"path", path.as_bytes());
            digest.part(b"sha256", hash.as_deref().unwrap_or_default().as_bytes());
            if !in_closure(path) && !metadata_file(path) {
                continue;
            }
            let hash = hash
                .clone()
                .ok_or_else(|| fail(format!("{dist}: {path} has no RECORD hash")))?;
            let entry = listed
                .entry(path.clone())
                .or_insert_with(|| (Vec::new(), hash.clone()));
            if entry.1 != hash {
                return Err(fail(format!(
                    "{path} has different RECORD hashes in two distributions"
                )));
            }
            entry.0.push(dist.clone());
        }
        distributions.push(InventoryDistribution {
            name: dist.clone(),
            version: version.clone(),
            first_party: release.contains(dist),
            artifact_sha256: artifacts.clone(),
            record_digest: digest.finish(),
        });
    }
    let mut files = BTreeMap::new();
    let walk = walkdir::WalkDir::new(&site)
        .follow_links(false)
        .sort_by_file_name()
        .min_depth(1)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != "__pycache__");
    for entry in walk {
        let entry = entry.map_err(|e| fail(format!("walking {}: {e}", site.display())))?;
        let path = relative(&site, entry.path())?;
        if entry.file_type().is_symlink() {
            if entry.path().is_dir() || in_closure(&path) {
                return Err(fail(format!(
                    "site-packages holds the symlink {path}; the closure is captured from regular files only"
                )));
            }
            continue;
        }
        if !entry.file_type().is_file()
            || !(in_closure(&path) || (metadata_file(&path) && listed.contains_key(&path)))
        {
            continue;
        }
        let (owners, record_sha256) = listed
            .get(&path)
            .map_or((Vec::new(), None), |(owners, hash)| {
                (owners.clone(), Some(hash.clone()))
            });
        let role = if metadata_file(&path) {
            SourceRole::DistributionMetadata
        } else if owners.iter().any(|owner| release.contains(owner)) {
            SourceRole::Release
        } else {
            SourceRole::Dependency
        };
        files.insert(
            path.clone(),
            InventoryFile {
                path,
                owners,
                role,
                record_sha256,
            },
        );
    }
    if let Some(missing) = listed.keys().find(|path| !files.contains_key(*path)) {
        return Err(fail(format!(
            "{missing} is listed in a RECORD but missing from the environment; {remedy}"
        )));
    }
    let mut configuration = KeySink::new("acquisition-configuration");
    configuration.part(b"pyproject", pyproject.as_bytes());
    configuration.part(b"lock", lock_text.as_bytes());
    configuration.part(b"python", pinned_python.as_bytes());
    Ok(LibraryInventory {
        name,
        requirement: definition.requirement.clone(),
        lock_digest: ContentHash::of(lock_text.as_bytes()),
        installer: venv.get("uv").map(|v| format!("uv {v}")),
        python_version: python,
        platform: std::env::consts::OS.to_owned(),
        site_packages: site,
        distributions,
        files: files.into_values().collect(),
        configuration: configuration.finish(),
    })
}

fn corpus_inventory(
    tree: &Path,
    source: &library::Source,
) -> Result<CorpusInventory, ExtractError> {
    // The retired compiler wrote blocks into the tree; the reserved namespace belongs to the capture.
    for reserved in ["_lctx_blocks", "_lctx"] {
        if std::fs::symlink_metadata(tree.join(reserved)).is_ok() {
            return Err(fail(format!(
                "the source tree {} holds `{reserved}`, which acquisition reserves; remove it",
                tree.display()
            )));
        }
    }
    let walked = library::walk_tree(tree)?;
    // The whole root is on the corpus import path: a link that could supply a module is refused
    // rather than read through.
    if let Some((path, _)) = walked
        .symlinks
        .iter()
        .find(|(path, is_dir)| *is_dir || library::analyzer_readable(path))
    {
        return Err(fail(format!(
            "the source tree holds an analyzer-readable symlink {path} in {}; materialize it as regular files",
            tree.display()
        )));
    }
    let rel = |paths: Vec<PathBuf>| {
        paths
            .into_iter()
            .map(|p| relative(tree, &p))
            .collect::<Result<Vec<_>, _>>()
    };
    let documents = rel(library::pick(
        tree,
        &walked,
        "documents",
        &source.documents,
        &source.documents_exclude,
    )?)?;
    let examples = rel(library::pick(
        tree,
        &walked,
        "examples",
        &source.examples,
        &source.examples_exclude,
    )?)?;
    let tests = rel(library::pick(
        tree,
        &walked,
        "tests",
        &source.tests,
        &source.tests_exclude,
    )?)?;
    let assets = rel(library::pick(
        tree,
        &walked,
        "assets",
        &source.assets,
        &source.assets_exclude,
    )?)?;
    if let Some(both) = examples.iter().find(|f| tests.contains(f)) {
        return Err(fail(format!(
            "[tool.lctx.source] `examples` and `tests` both select {both}"
        )));
    }
    let mut files: BTreeMap<String, Vec<SourceRole>> = walked
        .files
        .iter()
        .filter(|(path, _)| library::analyzer_readable(path))
        .map(|(path, _)| (path.clone(), Vec::new()))
        .collect();
    for (paths, role) in [
        (&documents, SourceRole::Document),
        (&examples, SourceRole::Example),
        (&tests, SourceRole::Test),
        (&assets, SourceRole::Configuration),
    ] {
        for path in paths {
            let roles = files.entry(path.clone()).or_default();
            if !roles.contains(&role) {
                roles.push(role);
            }
        }
    }
    Ok(CorpusInventory {
        repository: source.repository.clone(),
        commit: source.commit.clone(),
        root: tree.to_path_buf(),
        files,
        documents,
    })
}

fn relative(root: &Path, path: &Path) -> Result<String, ExtractError> {
    Ok(path
        .strip_prefix(root)
        .map_err(|_| ExtractError::RelativePath(path.to_path_buf()))?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

/// How one captured input was acquired: its origin, and the classes and uses of its artifacts.
#[derive(Debug, Clone)]
pub enum Acquisition {
    /// An installed library environment.
    Installed(LibraryInventory),
    /// The pinned source tree of the library input at `library` in the attempt's inputs.
    Corpus {
        repository: String,
        commit: String,
        uses: BTreeMap<String, Vec<SourceRole>>,
        library: usize,
    },
    /// A labelled tree (fixtures and local checkouts): every original is unowned; Python sources
    /// are release modules and Markdown files documents.
    Tree { label: String },
}
/// One captured input and how it was acquired.
pub struct AcquiredInput {
    captured: CapturedInput,
    acquisition: Acquisition,
}
impl AcquiredInput {
    pub fn new(captured: CapturedInput, acquisition: Acquisition) -> Self {
        Self {
            captured,
            acquisition,
        }
    }
    pub fn tree(captured: CapturedInput, label: &str) -> Self {
        Self::new(
            captured,
            Acquisition::Tree {
                label: label.to_owned(),
            },
        )
    }
    pub fn captured(&self) -> &CapturedInput {
        &self.captured
    }
    pub fn acquisition(&self) -> &Acquisition {
        &self.acquisition
    }
    /// The acquisition's typed uses, shared by emission, producer roots and coverage admission.
    pub fn uses(&self) -> Vec<ArtifactUse> {
        let mut uses = Vec::new();
        let derived: BTreeSet<_> = self
            .captured
            .derivations()
            .iter()
            .map(|d| d.path())
            .collect();
        for artifact in self.captured.artifacts() {
            let roles: Vec<SourceRole> = if derived.contains(artifact.path.as_str()) {
                if matches!(
                    self.captured
                        .derivations()
                        .iter()
                        .find(|d| d.path() == artifact.path),
                    Some(crate::capture::Derivation::PythonCodeBlock { .. })
                ) {
                    vec![SourceRole::DocBlock]
                } else {
                    vec![SourceRole::TaskReceipt]
                }
            } else {
                match &self.acquisition {
                    Acquisition::Installed(library) => library
                        .files
                        .iter()
                        .filter(|f| f.path == artifact.path)
                        .map(|f| f.role)
                        .collect(),
                    Acquisition::Corpus { uses, .. } => {
                        uses.get(&artifact.path).cloned().unwrap_or_default()
                    }
                    Acquisition::Tree { .. } => {
                        match lctx_model::domain::admission::ArtifactClass::of(&artifact.path) {
                            Some(lctx_model::domain::admission::ArtifactClass::PythonSource) => {
                                vec![SourceRole::Release]
                            }
                            Some(lctx_model::domain::admission::ArtifactClass::Document) => {
                                vec![SourceRole::Document]
                            }
                            None => vec![],
                        }
                    }
                }
            };
            uses.extend(roles.into_iter().map(|role| ArtifactUse {
                artifact: artifact.id(),
                input: self.captured.revision().id(),
                role,
            }));
        }
        uses
    }
    pub fn origin(&self) -> InputOrigin {
        match &self.acquisition {
            Acquisition::Installed(library) => InputOrigin::Installed {
                library: library.name.clone(),
                requirement: library.requirement.clone(),
                lock_digest: library.lock_digest,
                installer: library.installer.clone(),
            },
            Acquisition::Corpus {
                repository, commit, ..
            } => InputOrigin::Corpus {
                repository: repository.clone(),
                revision: commit.clone(),
            },
            Acquisition::Tree { label } => InputOrigin::Tree {
                label: label.clone(),
            },
        }
    }
}

/// Capture an inventory: the environment closure, then the corpus with its derived blocks. Every
/// frozen byte a `RECORD` lists must match its sha256, so what is verified is what is analyzed.
pub fn capture(
    inventory: &InputInventory,
    budget: &ResourceBudget,
    config: crate::native_context::NativeContextConfig,
) -> Result<CapturedInputs, AcquireError> {
    capture_receipts(inventory, budget, &[], config)
}
pub fn capture_receipts(
    inventory: &InputInventory,
    budget: &ResourceBudget,
    receipts: &[PathBuf],
    config: crate::native_context::NativeContextConfig,
) -> Result<CapturedInputs, AcquireError> {
    if !receipts.is_empty() && inventory.corpus.is_none() {
        return Err(fail("task receipts require a declared corpus input").into());
    }
    let library = &inventory.library;
    let paths: Vec<String> = library.files.iter().map(|file| file.path.clone()).collect();
    let captured = CapturedInput::capture(&library.site_packages, &paths, budget)?;
    for file in &library.files {
        let Some(expected) = &file.record_sha256 else {
            continue;
        };
        let mut frozen =
            std::fs::File::open(captured.root().join(&file.path)).map_err(CaptureError::from)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = frozen.read(&mut buffer).map_err(CaptureError::from)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if URL_SAFE_NO_PAD.encode(hasher.finalize()) != *expected {
            return Err(fail(format!(
                "{}: {} does not match its RECORD sha256; run `lctx acquire {} --reinstall`",
                file.owners.join(", "),
                file.path,
                library.name
            ))
            .into());
        }
    }
    let mut inputs = vec![AcquiredInput::new(
        captured,
        Acquisition::Installed(library.clone()),
    )];
    if let Some(corpus) = &inventory.corpus {
        let paths: Vec<String> = corpus.files.keys().cloned().collect();
        let captured = CapturedInput::capture_derived(
            &corpus.root,
            &paths,
            budget,
            &corpus.documents,
            derive_blocks,
        )?
        .with_receipts(receipts)?;
        inputs.push(AcquiredInput::new(
            captured,
            Acquisition::Corpus {
                repository: corpus.repository.clone(),
                commit: corpus.commit.clone(),
                uses: corpus.files.clone(),
                library: 0,
            },
        ));
    }
    Ok(CapturedInputs::new(inputs, config))
}
/// A document's Python code blocks, derived as modules of their own.
pub fn derive_blocks(document: &str, bytes: &[u8]) -> Result<Vec<Derived>, CaptureError> {
    if bytes.len() > 64 << 20 {
        return Ok(vec![]);
    }
    Ok(crate::document_parser::python_blocks(bytes)
        .into_iter()
        .map(|(ordinal, start, end, code)| Derived {
            path: crate::document_parser::block_module_path(document, ordinal),
            document: document.to_owned(),
            ordinal,
            fence: (start as i64, end as i64),
            bytes: code.into_bytes(),
        })
        .collect())
}

/// The acquisition provider's identity: this module, the capture and the definition reader.
pub fn acquisition_provider() -> Provider {
    Provider {
        tool: "lctx-acquire".into(),
        revision: env!("CARGO_PKG_VERSION").into(),
        build_digest: bundle::build_digest(&[
            include_str!("acquisition.rs"),
            include_str!("capture.rs"),
            include_str!("library.rs"),
        ]),
    }
}
/// The `acquire` stage: states every captured input's acquisition relations.
pub struct Acquire {
    configuration: ContentHash,
}
impl Acquire {
    pub fn new(configuration: ContentHash) -> Self {
        Self { configuration }
    }
    /// The stage of an inventory's acquisition configuration.
    pub fn of(inventory: &InputInventory) -> Self {
        Self::new(inventory.library.configuration)
    }
}
fn outputs() -> Vec<RelationUse> {
    vec![
        RelationUse::of::<Package>(),
        RelationUse::of::<Release>(),
        RelationUse::of::<InputRevision>(),
        RelationUse::of::<InputOrigin>(),
        RelationUse::of::<InputAcquisition>(),
        RelationUse::of::<CorpusLibrary>(),
        RelationUse::of::<InputDistribution>(),
        RelationUse::of::<DistributionVerification>(),
        RelationUse::of::<EnvironmentFingerprint>(),
        RelationUse::of::<SourceArtifact>(),
        RelationUse::of::<ArtifactChunk>(),
        RelationUse::of::<ArtifactOwnership>(),
        RelationUse::of::<UnownedArtifact>(),
        RelationUse::of::<DerivedArtifact>(),
        RelationUse::of::<ArtifactUse>(),
    ]
}
fn capture_error(error: CaptureError) -> ModelError {
    match error {
        CaptureError::Model(error) => error,
        other => ModelError::Invalid(other.to_string()),
    }
}
pub fn contract() -> ProducerContract {
        ProducerContract {
            name: ACQUIRE,
            inputs: vec![],
            outputs: outputs(),
            contributes: vec![],
            profiles: vec![Profile::Catalog, Profile::Behavioral],
            effect: Effect::Acquisition,
            semantic_revision: 1,
            configuration: ConfigurationBinding::ProducerSettings,
            suppliers: vec![supplier("acquisition", "lctx-acquire", env!("CARGO_PKG_VERSION"), vec![FactFamily::Artifacts])],
            not_requested: vec![],
        }
}
impl Declared for Acquire {
    fn declaration(&self, _: Profile) -> Stage {
        let provider = acquisition_provider();
        contract().bind(provider.build_digest, self.configuration, [("acquisition", provider)])
    }
}
impl<S: ProviderSink + 'static> ProviderStage<S> for Acquire {
    fn run(&mut self, context: &mut StageContext<S>) -> Result<ProviderOutcome, ModelError> {
        context.declare::<Package>()?;
        context.declare::<Release>()?;
        context.declare::<InputRevision>()?;
        context.declare::<InputOrigin>()?;
        context.declare::<InputAcquisition>()?;
        context.declare::<CorpusLibrary>()?;
        context.declare::<InputDistribution>()?;
        context.declare::<DistributionVerification>()?;
        context.declare::<EnvironmentFingerprint>()?;
        context.declare::<SourceArtifact>()?;
        context.declare::<ArtifactChunk>()?;
        context.declare::<ArtifactOwnership>()?;
        context.declare::<UnownedArtifact>()?;
        context.declare::<DerivedArtifact>()?;
        context.declare::<ArtifactUse>()?;
        let captured = context.captured();
        for input in captured.inputs() {
            let frozen = input.captured();
            let revision = frozen.revision();
            let origin = input.origin();
            let acquisition = InputAcquisition {
                input: revision.id(),
                origin: origin.id(),
            };
            context.emit(revision.clone())?;
            context.emit(origin)?;
            context.emit(acquisition.clone())?;
            let artifacts: BTreeMap<&str, &SourceArtifact> = frozen
                .artifacts()
                .iter()
                .map(|a| (a.path.as_str(), a))
                .collect();
            for artifact in frozen.artifacts() {
                context.emit(artifact.clone())?;
            }
            for index in 0..frozen.artifacts().len() {
                frozen
                    .emit_chunks(index, |chunk| context.emit(chunk.clone()))
                    .map_err(capture_error)?;
            }
            let derived: BTreeMap<&str, &crate::capture::Derivation> =
                frozen.derivations().iter().map(|d| (d.path(), d)).collect();
            for derivation in frozen.derivations() {
                let artifact = artifacts[derivation.path()].id();
                context.emit(match derivation {
                    crate::capture::Derivation::PythonCodeBlock {
                        document,
                        ordinal,
                        fence,
                        ..
                    } => DerivedArtifact::PythonCodeBlock {
                        artifact,
                        document: artifacts[document.as_str()].id(),
                        ordinal: *ordinal,
                        fence_start: fence.0,
                        fence_end: fence.1,
                    },
                    crate::capture::Derivation::TaskReceipt { .. } => {
                        DerivedArtifact::TaskReceipt {
                            artifact,
                            input: revision.id(),
                        }
                    }
                })?;
            }
            let unowned = |context: &mut StageContext<S>, artifact: &SourceArtifact| {
                context.emit(UnownedArtifact {
                    artifact: artifact.id(),
                    acquisition: acquisition.id(),
                })
            };
            match input.acquisition() {
                Acquisition::Installed(library) => {
                    let mut verifications = BTreeMap::new();
                    for dist in &library.distributions {
                        let package = Package {
                            name: dist.name.clone(),
                        };
                        let release = Release {
                            package: package.id(),
                            version: dist.version.clone(),
                        };
                        let verification = DistributionVerification {
                            acquisition: acquisition.id(),
                            release: release.id(),
                            record_digest: dist.record_digest,
                            artifact_sha256: dist.artifact_sha256.clone(),
                        };
                        let role = if dist.first_party {
                            DistributionRole::FirstParty
                        } else {
                            DistributionRole::Dependency
                        };
                        context.emit(InputDistribution {
                            input: revision.id(),
                            release: release.id(),
                            role,
                        })?;
                        if dist.first_party {
                            context.emit(EnvironmentFingerprint {
                                acquisition: acquisition.id(),
                                release: release.id(),
                                lock_digest: library.lock_digest,
                                environment_digest: revision.manifest,
                                python_version: library.python_version.clone(),
                                platform: library.platform.clone(),
                            })?;
                        }
                        verifications.insert(dist.name.as_str(), verification.id());
                        context.emit(package)?;
                        context.emit(release)?;
                        context.emit(verification)?;
                    }
                    for file in &library.files {
                        let artifact = artifacts[file.path.as_str()];
                        if file.owners.is_empty() {
                            unowned(context, artifact)?;
                        }
                        for owner in &file.owners {
                            context.emit(ArtifactOwnership {
                                artifact: artifact.id(),
                                distribution: verifications[owner.as_str()],
                            })?;
                        }
                    }
                }
                Acquisition::Corpus { library, .. } => {
                    let library = captured.inputs().get(*library).ok_or_else(|| {
                        ModelError::Invalid("a corpus names an absent library input".into())
                    })?;
                    context.emit(CorpusLibrary {
                        corpus: revision.id(),
                        library: library.captured().revision().id(),
                    })?;
                    for artifact in frozen
                        .artifacts()
                        .iter()
                        .filter(|a| !derived.contains_key(a.path.as_str()))
                    {
                        unowned(context, artifact)?;
                    }
                }
                Acquisition::Tree { .. } => {
                    for artifact in frozen
                        .artifacts()
                        .iter()
                        .filter(|a| !derived.contains_key(a.path.as_str()))
                    {
                        unowned(context, artifact)?;
                    }
                }
            }
            for usage in input.uses() {
                context.emit(usage)?;
            }
            frozen.verify().map_err(capture_error)?;
        }
        Ok(ProviderOutcome::Complete)
    }
}
