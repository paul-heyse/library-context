//! Content revisions and acquisition provenance have independent identities.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    ContentHash, Id, Invariant, InvariantCheck, Key, KeySink, ModelError, Record, ValidationInput,
};
use crate::{Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "packages", validate = validate_package)]
pub struct Package {
    #[model(key)]
    pub name: String,
}
fn validate_package(value: &Package) -> Result<(), ModelError> {
    if value.name.is_empty()
        || value.name.starts_with('-')
        || value.name.ends_with('-')
        || value.name.contains("--")
        || !value
            .name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(ModelError::Invalid(
            "package name must be normalized".into(),
        ));
    }
    Ok(())
}

/// A distribution release is not an acquired environment or an input tree.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "releases", validate = validate_release)]
pub struct Release {
    #[model(key)]
    pub package: Id<Package>,
    #[model(key)]
    pub version: String,
}
fn validate_release(value: &Release) -> Result<(), ModelError> {
    if value.version.trim().is_empty() {
        return Err(ModelError::Invalid("release needs a version".into()));
    }
    Ok(())
}

/// The digest covers all analyzer-visible entries. Provenance and display labels live separately.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_revisions", invariant_refs = input_invariants_refs)]
pub struct InputRevision {
    #[model(key)]
    pub manifest: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: String,
    pub content: ContentHash,
    pub byte_len: i64,
}
impl InputRevision {
    pub fn from_entries(mut entries: Vec<ManifestEntry>) -> Result<Self, ModelError> {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let mut manifest = ManifestBuilder::new();
        for entry in entries {
            manifest.push(entry)?;
        }
        Ok(Self {
            manifest: manifest.finish(),
        })
    }
}

/// Describes acquisition; a label never substitutes for a tree's content manifest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "input_origins")]
pub enum InputOrigin {
    #[model(code = 0)]
    Installed {
        library: String,
        requirement: String,
        lock_digest: ContentHash,
        installer: Option<String>,
    },
    #[model(code = 1)]
    Tree { label: String },
    #[model(code = 2)]
    Corpus {
        repository: String,
        revision: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_acquisitions", invariant_refs = acquisition_invariants_refs)]
pub struct InputAcquisition {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key, provenance)]
    pub origin: Id<InputOrigin>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "corpus_libraries")]
pub struct CorpusLibrary {
    #[model(key)]
    pub corpus: Id<InputRevision>,
    #[model(key)]
    pub library: Id<InputRevision>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DistributionRole {
    FirstParty = 0,
    Dependency = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_distributions")]
pub struct InputDistribution {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub release: Id<Release>,
    #[model(key)]
    pub role: DistributionRole,
}

pub fn validate_path(path: &str) -> Result<(), ModelError> {
    if path.is_empty()
        || path.contains('\0')
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|p| p == "." || p == ".." || p.is_empty())
    {
        return Err(ModelError::Invalid(
            "artifact path must be normalized and relative".into(),
        ));
    }
    Ok(())
}

struct ManifestBuilder {
    sink: KeySink,
    previous: Option<String>,
    count: u64,
}
impl ManifestBuilder {
    fn new() -> Self {
        Self {
            sink: KeySink::new("input-manifest"),
            previous: None,
            count: 0,
        }
    }
    fn push(&mut self, entry: ManifestEntry) -> Result<(), ModelError> {
        validate_path(&entry.path)?;
        if entry.byte_len < 0 || self.previous.as_ref().is_some_and(|p| p >= &entry.path) {
            return Err(ModelError::Invalid(
                "manifest paths must be unique and increasing; lengths nonnegative".into(),
            ));
        }
        entry.path.encode(&mut self.sink);
        entry.content.encode(&mut self.sink);
        entry.byte_len.encode(&mut self.sink);
        self.previous = Some(entry.path);
        self.count += 1;
        Ok(())
    }
    fn finish(mut self) -> ContentHash {
        self.sink.part(b"entry-count", &self.count.to_le_bytes());
        self.sink.finish()
    }
}
pub(crate) fn input_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "input_manifest_membership",
        inputs: vec![
            ValidationInput::of::<InputRevision>(&["id"]),
            ValidationInput::of::<super::source::SourceArtifact>(&["input", "path", "id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(InputManifestCheck {
                charge: StateCharge::new(budget, "input_manifest_membership"),
                expected: Default::default(),
                current: None,
            })
        }),
    }]
}
struct InputManifestCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<InputRevision>, ContentHash>,
    current: Option<(Id<InputRevision>, ManifestBuilder)>,
}
impl InputManifestCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((input, manifest)) = self.current.take()
            && self.expected.remove(&mut self.charge, &input) != Some(manifest.finish())
        {
            return Err(ModelError::Invalid(
                "stored artifacts differ from input manifest".into(),
            ));
        }
        Ok(())
    }
}
impl InvariantCheck for InputManifestCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        if relation == InputRevision::NAME {
            for input in InputRevision::decode(batch)? {
                if self
                    .expected
                    .insert(&mut self.charge, input.id(), input.manifest)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(InputRevision::NAME));
                }
            }
        } else if relation == SourceArtifact::NAME {
            for artifact in SourceArtifact::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(input, _)| *input != artifact.input)
                {
                    self.flush()?;
                    if !self.expected.contains_key(&artifact.input) {
                        return Err(ModelError::Invalid(
                            "artifact input absent or out of order".into(),
                        ));
                    }
                    self.current = Some((artifact.input, ManifestBuilder::new()));
                }
                self.current
                    .as_mut()
                    .expect("initialized manifest")
                    .1
                    .push(artifact.manifest_entry())?;
            }
        } else {
            return Err(ModelError::Invalid("undeclared manifest input".into()));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        let empty = ManifestBuilder::new().finish();
        if self.expected.values().any(|digest| *digest != empty) {
            return Err(ModelError::Invalid(
                "input manifest has missing artifacts".into(),
            ));
        }
        Ok(())
    }
}

/// Acquisition-specific verification does not change enduring package/version identity.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "distribution_verifications", validate = validate_distribution)]
pub struct DistributionVerification {
    #[model(key, provenance)]
    pub acquisition: Id<InputAcquisition>,
    #[model(key)]
    pub release: Id<Release>,
    #[model(key)]
    pub record_digest: ContentHash,
    pub artifact_sha256: Vec<String>,
}
fn validate_distribution(row: &DistributionVerification) -> Result<(), ModelError> {
    if row.artifact_sha256.iter().any(|digest| {
        digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) || row
        .artifact_sha256
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(ModelError::Invalid(
            "artifact SHA256 hashes must be canonical, sorted and unique".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "artifact_ownership", invariant_refs = ownership_invariants_refs)]
pub struct ArtifactOwnership {
    #[model(key)]
    pub artifact: Id<super::source::SourceArtifact>,
    #[model(key, provenance)]
    pub distribution: Id<DistributionVerification>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
/// Why an input uses an artifact. Append-only: a new role takes the next code.
pub enum SourceRole {
    Release = 0,
    Example = 1,
    Test = 2,
    DocBlock = 3,
    Dependency = 4,
    Document = 5,
    DistributionMetadata = 6,
    Configuration = 7,
    TaskReceipt = 8,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "artifact_uses")]
/// A source artifact used in its own revision, or a library artifact explicitly used by a
/// corpus linked through CorpusLibrary. The use never changes the artifact's content identity.
pub struct ArtifactUse {
    #[model(key)]
    pub artifact: Id<super::source::SourceArtifact>,
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub role: SourceRole,
}

/// The reserved namespace of a frozen capture: only artifacts the compiler derives live under it,
/// and no original artifact may (ADR-0089).
pub const DERIVED_ROOT: &str = "_lctx/";

/// A captured artifact no verified distribution's `RECORD` owns: a loose file or an unowned stub in
/// an environment, or any file of a tree or corpus input.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "unowned_artifacts", invariant_refs = class_invariants_refs)]
pub struct UnownedArtifact {
    #[model(key)]
    pub artifact: Id<super::source::SourceArtifact>,
    #[model(key, provenance)]
    pub acquisition: Id<InputAcquisition>,
}
/// Provenance of an artifact captured into the reserved namespace. Python blocks name an
/// original document and its byte fence; externally reported task receipts name their corpus
/// input and never pretend to have a document fence. Codes are append-only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "derived_artifacts", validate = validate_derived)]
pub enum DerivedArtifact {
    #[model(code = 0)]
    PythonCodeBlock {
        artifact: Id<super::source::SourceArtifact>,
        document: Id<super::source::SourceArtifact>,
        ordinal: i64,
        fence_start: i64,
        fence_end: i64,
    },
    #[model(code = 1)]
    TaskReceipt {
        artifact: Id<super::source::SourceArtifact>,
        input: Id<InputRevision>,
    },
}
impl DerivedArtifact {
    pub fn artifact(&self) -> Id<super::source::SourceArtifact> {
        match self {
            Self::PythonCodeBlock { artifact, .. } | Self::TaskReceipt { artifact, .. } => {
                *artifact
            }
        }
    }
}
fn validate_derived(row: &DerivedArtifact) -> Result<(), ModelError> {
    if let DerivedArtifact::PythonCodeBlock {
        artifact,
        document,
        ordinal,
        fence_start,
        fence_end,
    } = row
        && (*ordinal < 0 || *fence_start < 0 || fence_end < fence_start || artifact == document)
    {
        return Err(ModelError::Invalid(
            "a derivation needs a nonnegative ordinal, an ordered fence and a distinct document"
                .into(),
        ));
    }
    Ok(())
}
/// The environment an installed input was acquired from, stated in the terms a deployment receipt
/// reports (`ReportedEnvironment`), so the two can be compared. The compiler never runs the
/// interpreter, so the runtime and interpreter digests a receipt may state have no acquired twin.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "environment_fingerprints", validate = validate_fingerprint)]
pub struct EnvironmentFingerprint {
    #[model(key, provenance)]
    pub acquisition: Id<InputAcquisition>,
    #[model(key)]
    pub release: Id<Release>,
    #[model(key)]
    pub lock_digest: ContentHash,
    #[model(key)]
    pub environment_digest: ContentHash,
    #[model(key)]
    pub python_version: String,
    #[model(key)]
    pub platform: String,
}
fn validate_fingerprint(row: &EnvironmentFingerprint) -> Result<(), ModelError> {
    if row.python_version.trim().is_empty() || row.platform.trim().is_empty() {
        return Err(ModelError::Invalid(
            "an environment fingerprint names its Python version and platform".into(),
        ));
    }
    Ok(())
}

/// Every captured artifact has at most one class: owned by distributions, unowned, or derived. Only
/// derived artifacts live under `_lctx/`, each derived from an original document of its own input,
/// within that document's bytes. Facts admission requires every artifact to have a class.
pub(crate) fn class_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "artifact_classes",
        inputs: vec![
            ValidationInput::of::<InputAcquisition>(&["id"]),
            ValidationInput::of::<super::source::SourceArtifact>(&["id"]),
            ValidationInput::of::<ArtifactOwnership>(&["artifact", "id"]),
            ValidationInput::of::<UnownedArtifact>(&["id"]),
            ValidationInput::of::<DerivedArtifact>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ClassCheck {
                charge: StateCharge::new(budget, "artifact_classes"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Debug, Clone, Copy)]
struct Captured {
    input: Id<InputRevision>,
    byte_len: i64,
    reserved: bool,
    owned: bool,
    unowned: bool,
    derived: bool,
}
impl super::HeapSize for Captured {}
#[derive(Default)]
struct ClassCheck {
    charge: StateCharge,
    acquisitions: ChargedMap<Id<InputAcquisition>, Id<InputRevision>>,
    artifacts: ChargedMap<Id<super::source::SourceArtifact>, Captured>,
    documents: ChargedSet<Id<super::source::SourceArtifact>>,
}
impl ClassCheck {
    fn class(
        &mut self,
        artifact: Id<super::source::SourceArtifact>,
        set: impl FnOnce(&mut Captured) -> bool,
    ) -> Result<Captured, ModelError> {
        let mut captured = *self
            .artifacts
            .get(&artifact)
            .ok_or_else(|| ModelError::Invalid("a classified artifact is not captured".into()))?;
        if !set(&mut captured) {
            return Ok(captured);
        }
        if [captured.owned, captured.unowned, captured.derived]
            .iter()
            .filter(|c| **c)
            .count()
            > 1
        {
            return Err(ModelError::Invalid(
                "a captured artifact has more than one of the owned, unowned and derived classes"
                    .into(),
            ));
        }
        self.artifacts
            .insert(&mut self.charge, artifact, captured)?;
        Ok(captured)
    }
}
impl InvariantCheck for ClassCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        if relation == InputAcquisition::NAME {
            for row in InputAcquisition::decode(batch)? {
                self.acquisitions
                    .insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                let captured = Captured {
                    input: row.input,
                    byte_len: row.byte_len,
                    reserved: row.path.starts_with(DERIVED_ROOT),
                    owned: false,
                    unowned: false,
                    derived: false,
                };
                self.artifacts
                    .insert(&mut self.charge, row.id(), captured)?;
            }
        } else if relation == ArtifactOwnership::NAME {
            // An artifact several distributions own is still one owned artifact.
            for row in ArtifactOwnership::decode(batch)? {
                self.class(row.artifact, |c| !std::mem::replace(&mut c.owned, true))?;
            }
        } else if relation == UnownedArtifact::NAME {
            for row in UnownedArtifact::decode(batch)? {
                let input = *self.acquisitions.get(&row.acquisition).ok_or_else(|| {
                    ModelError::Invalid("an unowned artifact's acquisition is absent".into())
                })?;
                let captured = self.class(row.artifact, |c| {
                    c.unowned = true;
                    true
                })?;
                if captured.input != input {
                    return Err(ModelError::Invalid(
                        "an unowned artifact belongs to another acquisition's input".into(),
                    ));
                }
            }
        } else if relation == DerivedArtifact::NAME {
            for row in DerivedArtifact::decode(batch)? {
                let captured = self.class(row.artifact(), |c| {
                    c.derived = true;
                    true
                })?;
                match row {
                    DerivedArtifact::PythonCodeBlock {
                        document,
                        fence_end,
                        ..
                    } => {
                        let origin = *self.artifacts.get(&document).ok_or_else(|| {
                            ModelError::Invalid(
                                "a derived artifact's document is not captured".into(),
                            )
                        })?;
                        if origin.input != captured.input
                            || origin.reserved
                            || fence_end > origin.byte_len
                        {
                            return Err(ModelError::Invalid("a derivation needs an original document of the same input, within its bytes".into()));
                        }
                        self.documents.insert(&mut self.charge, document)?;
                    }
                    DerivedArtifact::TaskReceipt { input, .. } => {
                        if input != captured.input {
                            return Err(ModelError::Invalid(
                                "a captured task receipt belongs to another input".into(),
                            ));
                        }
                    }
                }
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared artifact class input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.artifacts.values().any(|c| c.reserved != c.derived) {
            return Err(ModelError::Invalid(
                "only derived artifacts, and all of them, live under the reserved _lctx/ namespace"
                    .into(),
            ));
        }
        if self
            .documents
            .iter()
            .any(|d| self.artifacts.get(d).is_some_and(|c| c.derived))
        {
            return Err(ModelError::Invalid(
                "a derivation's document is itself derived".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn ownership_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "artifact_ownership_input",
        inputs: vec![
            ValidationInput::of::<InputAcquisition>(&["id"]),
            ValidationInput::of::<DistributionVerification>(&["id"]),
            ValidationInput::of::<super::source::SourceArtifact>(&["id"]),
            ValidationInput::of::<ArtifactOwnership>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(OwnershipCheck {
                charge: StateCharge::new(budget, "artifact_ownership_input"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct OwnershipCheck {
    charge: StateCharge,
    acquisitions: ChargedMap<Id<InputAcquisition>, Id<InputRevision>>,
    distributions: ChargedMap<Id<DistributionVerification>, Id<InputRevision>>,
    artifacts: ChargedMap<Id<super::source::SourceArtifact>, Id<InputRevision>>,
}
impl InvariantCheck for OwnershipCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        let missing =
            || ModelError::Invalid("artifact ownership crosses or lacks acquired input".into());
        if relation == InputAcquisition::NAME {
            for row in InputAcquisition::decode(batch)? {
                if self
                    .acquisitions
                    .insert(&mut self.charge, row.id(), row.input)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(InputAcquisition::NAME));
                }
            }
        } else if relation == DistributionVerification::NAME {
            for row in DistributionVerification::decode(batch)? {
                let input = *self
                    .acquisitions
                    .get(&row.acquisition)
                    .ok_or_else(missing)?;
                if self
                    .distributions
                    .insert(&mut self.charge, row.id(), input)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(DistributionVerification::NAME));
                }
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                if self
                    .artifacts
                    .insert(&mut self.charge, row.id(), row.input)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(SourceArtifact::NAME));
                }
            }
        } else if relation == ArtifactOwnership::NAME {
            for row in ArtifactOwnership::decode(batch)? {
                let artifact_input = self.artifacts.get(&row.artifact).ok_or_else(missing)?;
                let verified_input = self
                    .distributions
                    .get(&row.distribution)
                    .ok_or_else(missing)?;
                if artifact_input != verified_input {
                    return Err(missing());
                }
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared ownership validation input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

pub(crate) fn acquisition_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "input_acquisition_boundaries",
        inputs: vec![
            ValidationInput::of::<InputOrigin>(&["id"]),
            ValidationInput::of::<InputAcquisition>(&["id"]),
            ValidationInput::of::<CorpusLibrary>(&["id"]),
            ValidationInput::of::<InputDistribution>(&["id"]),
            ValidationInput::of::<DistributionVerification>(&["id"]),
            ValidationInput::of::<EnvironmentFingerprint>(&["id"]),
            ValidationInput::of::<super::source::SourceArtifact>(&["id"]),
            ValidationInput::of::<ArtifactUse>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(AcquisitionBoundaries {
                charge: StateCharge::new(budget, "input_acquisition_boundaries"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct AcquisitionBoundaries {
    charge: StateCharge,
    corpus_origins: ChargedSet<Id<InputOrigin>>,
    acquired: ChargedSet<Id<InputRevision>>,
    corpus_inputs: ChargedSet<Id<InputRevision>>,
    acquisitions: ChargedMap<Id<InputAcquisition>, Id<InputRevision>>,
    corpus_libraries: ChargedSet<(Id<InputRevision>, Id<InputRevision>)>,
    distributions: ChargedMap<(Id<InputRevision>, Id<Release>), DistributionRole>,
    artifacts: ChargedMap<Id<super::source::SourceArtifact>, Id<InputRevision>>,
}
impl InvariantCheck for AcquisitionBoundaries {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        if relation == InputOrigin::NAME {
            for row in InputOrigin::decode(batch)? {
                if matches!(row, InputOrigin::Corpus { .. }) {
                    self.corpus_origins.insert(&mut self.charge, row.id())?;
                }
            }
        } else if relation == InputAcquisition::NAME {
            for row in InputAcquisition::decode(batch)? {
                self.acquired.insert(&mut self.charge, row.input)?;
                if self.corpus_origins.contains(&row.origin) {
                    self.corpus_inputs.insert(&mut self.charge, row.input)?;
                }
                if self
                    .acquisitions
                    .insert(&mut self.charge, row.id(), row.input)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(InputAcquisition::NAME));
                }
            }
        } else if relation == CorpusLibrary::NAME {
            for row in CorpusLibrary::decode(batch)? {
                if !self.corpus_inputs.contains(&row.corpus)
                    || !self.acquired.contains(&row.library)
                {
                    return Err(ModelError::Invalid(
                        "corpus library needs acquired corpus and library inputs".into(),
                    ));
                }
                self.corpus_libraries
                    .insert(&mut self.charge, (row.corpus, row.library))?;
            }
        } else if relation == InputDistribution::NAME {
            for row in InputDistribution::decode(batch)? {
                if !self.acquired.contains(&row.input) {
                    return Err(ModelError::Invalid(
                        "distribution input is not acquired".into(),
                    ));
                }
                if self
                    .distributions
                    .insert(&mut self.charge, (row.input, row.release), row.role)?
                    .is_some()
                {
                    return Err(ModelError::Invalid(
                        "distribution has contradictory input roles".into(),
                    ));
                }
            }
        } else if relation == DistributionVerification::NAME {
            for row in DistributionVerification::decode(batch)? {
                let input = self.acquisitions.get(&row.acquisition).ok_or_else(|| {
                    ModelError::Invalid("distribution verification acquisition absent".into())
                })?;
                if !self.distributions.contains_key(&(*input, row.release)) {
                    return Err(ModelError::Invalid(
                        "verification release absent from acquired input".into(),
                    ));
                }
            }
        } else if relation == EnvironmentFingerprint::NAME {
            for row in EnvironmentFingerprint::decode(batch)? {
                let input = self.acquisitions.get(&row.acquisition).ok_or_else(|| {
                    ModelError::Invalid("environment fingerprint acquisition absent".into())
                })?;
                if !self.distributions.contains_key(&(*input, row.release)) {
                    return Err(ModelError::Invalid(
                        "environment fingerprint names a release its input does not distribute"
                            .into(),
                    ));
                }
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.artifacts
                    .insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == ArtifactUse::NAME {
            for row in ArtifactUse::decode(batch)? {
                let input = self
                    .artifacts
                    .get(&row.artifact)
                    .ok_or_else(|| ModelError::Invalid("artifact use source absent".into()))?;
                if row.input != *input && !self.corpus_libraries.contains(&(row.input, *input)) {
                    return Err(ModelError::Invalid(
                        "artifact use crosses undeclared corpus/library boundary".into(),
                    ));
                }
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared acquisition validation input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

pub(crate) fn input_invariants_refs() -> Vec<&'static str> {
    vec!["input_manifest_membership"]
}
pub(crate) fn class_invariants_refs() -> Vec<&'static str> {
    vec!["artifact_classes"]
}
pub(crate) fn ownership_invariants_refs() -> Vec<&'static str> {
    vec!["artifact_ownership_input"]
}
pub(crate) fn acquisition_invariants_refs() -> Vec<&'static str> {
    vec!["input_acquisition_boundaries"]
}
