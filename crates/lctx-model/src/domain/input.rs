//! Content revisions and acquisition provenance have independent identities.
use crate::{Domain, DomainCode, DomainSum};
use super::{ContentHash, Id, Key, KeySink, ModelError, Record, Invariant, InvariantCheck, ValidationInput};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "packages", validate = validate_package)]
pub struct Package { #[model(key)] pub name: String }
fn validate_package(value: &Package) -> Result<(), ModelError> {
    if value.name.is_empty() || value.name.starts_with('-') || value.name.ends_with('-') || value.name.contains("--")
        || !value.name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
        return Err(ModelError::Invalid("package name must be normalized".into()));
    }
    Ok(())
}

/// A distribution release is not an acquired environment or an input tree.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "releases", validate = validate_release)]
pub struct Release {
    #[model(key)] pub package: Id<Package>,
    #[model(key)] pub version: String,
}
fn validate_release(value: &Release) -> Result<(), ModelError> {
    if value.version.trim().is_empty() { return Err(ModelError::Invalid("release needs a version".into())); }
    Ok(())
}

/// The digest covers all analyzer-visible entries. Provenance and display labels live separately.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_revisions", invariants = input_invariants)]
pub struct InputRevision { #[model(key)] pub manifest: ContentHash }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry { pub path: String, pub content: ContentHash, pub byte_len: i64 }
impl InputRevision {
    pub fn from_entries(mut entries: Vec<ManifestEntry>) -> Result<Self, ModelError> {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let mut manifest = ManifestBuilder::new();
        for entry in entries { manifest.push(entry)?; }
        Ok(Self { manifest: manifest.finish() })
    }
}

/// Describes acquisition; a label never substitutes for a tree's content manifest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "input_origins")]
pub enum InputOrigin {
    #[model(code = 0)] Installed { library: String, requirement: String, lock_digest: ContentHash, installer: Option<String> },
    #[model(code = 1)] Tree { label: String },
    #[model(code = 2)] Corpus { repository: String, revision: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_acquisitions", invariants = acquisition_invariants)]
pub struct InputAcquisition {
    #[model(key)] pub input: Id<InputRevision>,
    #[model(key, provenance)] pub origin: Id<InputOrigin>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "corpus_libraries")]
pub struct CorpusLibrary {
    #[model(key)] pub corpus: Id<InputRevision>,
    #[model(key)] pub library: Id<InputRevision>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DistributionRole { FirstParty = 0, Dependency = 1 }
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "input_distributions")]
pub struct InputDistribution {
    #[model(key)] pub input: Id<InputRevision>,
    #[model(key)] pub release: Id<Release>,
    #[model(key)] pub role: DistributionRole,
}

pub fn validate_path(path: &str) -> Result<(), ModelError> {
    if path.is_empty() || path.contains('\0') || path.starts_with('/') || path.contains('\\')
        || path.split('/').any(|p| p == "." || p == ".." || p.is_empty()) {
        return Err(ModelError::Invalid("artifact path must be normalized and relative".into()));
    }
    Ok(())
}

struct ManifestBuilder { sink: KeySink, previous: Option<String>, count: u64 }
impl ManifestBuilder {
    fn new() -> Self { Self { sink: KeySink::new("input-manifest"), previous: None, count: 0 } }
    fn push(&mut self, entry: ManifestEntry) -> Result<(), ModelError> {
        validate_path(&entry.path)?;
        if entry.byte_len < 0 || self.previous.as_ref().is_some_and(|p| p >= &entry.path) {
            return Err(ModelError::Invalid("manifest paths must be unique and increasing; lengths nonnegative".into()));
        }
        entry.path.encode(&mut self.sink); entry.content.encode(&mut self.sink); entry.byte_len.encode(&mut self.sink);
        self.previous = Some(entry.path); self.count += 1;
        Ok(())
    }
    fn finish(mut self) -> ContentHash { self.sink.part(b"entry-count", &self.count.to_le_bytes()); self.sink.finish() }
}
fn input_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "input_manifest_membership", inputs: vec![
        ValidationInput::of::<InputRevision>(&["id"]),
        ValidationInput::of::<super::source::SourceArtifact>(&["input", "path"]),
    ], create: || Box::new(InputManifestCheck { expected: Default::default(), current: None }) }]
}
struct InputManifestCheck {
    expected: std::collections::BTreeMap<Id<InputRevision>, ContentHash>,
    current: Option<(Id<InputRevision>, ManifestBuilder)>,
}
impl InputManifestCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((input, manifest)) = self.current.take() {
            if self.expected.remove(&input) != Some(manifest.finish()) {
                return Err(ModelError::Invalid("stored artifacts differ from input manifest".into()));
            }
        }
        Ok(())
    }
}
impl InvariantCheck for InputManifestCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        if relation == InputRevision::NAME {
            for input in InputRevision::decode(batch)? {
                if self.expected.len() >= 1_000_000 { return Err(ModelError::Invalid("input validation cardinality budget exceeded".into())); }
                if self.expected.insert(input.id(), input.manifest).is_some() { return Err(ModelError::Conflict(InputRevision::NAME)); }
            }
        } else if relation == SourceArtifact::NAME {
            for artifact in SourceArtifact::decode(batch)? {
                if self.current.as_ref().is_none_or(|(input, _)| *input != artifact.input) {
                    self.flush()?;
                    if !self.expected.contains_key(&artifact.input) { return Err(ModelError::Invalid("artifact input absent or out of order".into())); }
                    self.current = Some((artifact.input, ManifestBuilder::new()));
                }
                self.current.as_mut().expect("initialized manifest").1.push(artifact.manifest_entry())?;
            }
        } else { return Err(ModelError::Invalid("undeclared manifest input".into())); }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        let empty = ManifestBuilder::new().finish();
        if self.expected.values().any(|digest| *digest != empty) { return Err(ModelError::Invalid("input manifest has missing artifacts".into())); }
        Ok(())
    }
}

/// Acquisition-specific verification does not change enduring package/version identity.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "distribution_verifications", validate = validate_distribution)]
pub struct DistributionVerification {
    #[model(key, provenance)] pub acquisition: Id<InputAcquisition>,
    #[model(key)] pub release: Id<Release>,
    #[model(key)] pub record_digest: ContentHash,
    pub artifact_sha256: Vec<String>,
}
fn validate_distribution(row: &DistributionVerification) -> Result<(), ModelError> {
    if row.artifact_sha256.iter().any(|digest| digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        || row.artifact_sha256.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ModelError::Invalid("artifact SHA256 hashes must be canonical, sorted and unique".into()));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "artifact_ownership", invariants = ownership_invariants)]
pub struct ArtifactOwnership {
    #[model(key)] pub artifact: Id<super::source::SourceArtifact>,
    #[model(key, provenance)] pub distribution: Id<DistributionVerification>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SourceRole { Release = 0, Example = 1, Test = 2, DocBlock = 3 }
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "artifact_uses")]
/// A source artifact used in its own revision, or a library artifact explicitly used by a
/// corpus linked through CorpusLibrary. The use never changes the artifact's content identity.
pub struct ArtifactUse {
    #[model(key)] pub artifact: Id<super::source::SourceArtifact>,
    #[model(key)] pub input: Id<InputRevision>,
    #[model(key)] pub role: SourceRole,
}

fn ownership_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "artifact_ownership_input", inputs: vec![
        ValidationInput::of::<InputAcquisition>(&["id"]),
        ValidationInput::of::<DistributionVerification>(&["id"]),
        ValidationInput::of::<super::source::SourceArtifact>(&["id"]),
        ValidationInput::of::<ArtifactOwnership>(&["id"]),
    ], create: || Box::new(OwnershipCheck::default()) }]
}
#[derive(Default)]
struct OwnershipCheck {
    acquisitions: std::collections::BTreeMap<Id<InputAcquisition>, Id<InputRevision>>,
    distributions: std::collections::BTreeMap<Id<DistributionVerification>, Id<InputRevision>>,
    artifacts: std::collections::BTreeMap<Id<super::source::SourceArtifact>, Id<InputRevision>>,
}
impl InvariantCheck for OwnershipCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        let missing = || ModelError::Invalid("artifact ownership crosses or lacks acquired input".into());
        let budget = || ModelError::Invalid("ownership validation cardinality budget exceeded".into());
        if relation == InputAcquisition::NAME {
            for row in InputAcquisition::decode(batch)? {
                if self.acquisitions.len() >= 1_000_000 { return Err(budget()); }
                if self.acquisitions.insert(row.id(), row.input).is_some() { return Err(ModelError::Conflict(InputAcquisition::NAME)); }
            }
        } else if relation == DistributionVerification::NAME {
            for row in DistributionVerification::decode(batch)? {
                if self.distributions.len() >= 1_000_000 { return Err(budget()); }
                let input = *self.acquisitions.get(&row.acquisition).ok_or_else(missing)?;
                if self.distributions.insert(row.id(), input).is_some() { return Err(ModelError::Conflict(DistributionVerification::NAME)); }
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                if self.artifacts.len() >= 1_000_000 { return Err(budget()); }
                if self.artifacts.insert(row.id(), row.input).is_some() { return Err(ModelError::Conflict(SourceArtifact::NAME)); }
            }
        } else if relation == ArtifactOwnership::NAME {
            for row in ArtifactOwnership::decode(batch)? {
                let artifact_input = self.artifacts.get(&row.artifact).ok_or_else(missing)?;
                let verified_input = self.distributions.get(&row.distribution).ok_or_else(missing)?;
                if artifact_input != verified_input { return Err(missing()); }
            }
        } else { return Err(ModelError::Invalid("undeclared ownership validation input".into())); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}

fn acquisition_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "input_acquisition_boundaries", inputs: vec![
        ValidationInput::of::<InputOrigin>(&["id"]),
        ValidationInput::of::<InputAcquisition>(&["id"]),
        ValidationInput::of::<CorpusLibrary>(&["id"]),
        ValidationInput::of::<InputDistribution>(&["id"]),
        ValidationInput::of::<DistributionVerification>(&["id"]),
        ValidationInput::of::<super::source::SourceArtifact>(&["id"]),
        ValidationInput::of::<ArtifactUse>(&["id"]),
    ], create: || Box::new(AcquisitionBoundaries::default()) }]
}
#[derive(Default)]
struct AcquisitionBoundaries {
    corpus_origins: std::collections::BTreeSet<Id<InputOrigin>>,
    acquired: std::collections::BTreeSet<Id<InputRevision>>,
    corpus_inputs: std::collections::BTreeSet<Id<InputRevision>>,
    acquisitions: std::collections::BTreeMap<Id<InputAcquisition>, Id<InputRevision>>,
    corpus_libraries: std::collections::BTreeSet<(Id<InputRevision>, Id<InputRevision>)>,
    distributions: std::collections::BTreeMap<(Id<InputRevision>, Id<Release>), DistributionRole>,
    artifacts: std::collections::BTreeMap<Id<super::source::SourceArtifact>, Id<InputRevision>>,
}
impl InvariantCheck for AcquisitionBoundaries {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        use super::source::SourceArtifact;
        // Temporary cardinality admission until shared allocation reservations reach validators.
        let entries = self.corpus_origins.len() + self.acquired.len() + self.corpus_inputs.len()
            + self.acquisitions.len() + self.corpus_libraries.len() + self.distributions.len() + self.artifacts.len();
        if entries.saturating_add(batch.num_rows().saturating_mul(3)) > 3_000_000 {
            return Err(ModelError::Invalid("acquisition validation cardinality budget exceeded".into()));
        }
        if relation == InputOrigin::NAME {
            for row in InputOrigin::decode(batch)? {
                if matches!(row, InputOrigin::Corpus { .. }) { self.corpus_origins.insert(row.id()); }
            }
        } else if relation == InputAcquisition::NAME {
            for row in InputAcquisition::decode(batch)? {
                self.acquired.insert(row.input);
                if self.corpus_origins.contains(&row.origin) { self.corpus_inputs.insert(row.input); }
                if self.acquisitions.insert(row.id(), row.input).is_some() { return Err(ModelError::Conflict(InputAcquisition::NAME)); }
            }
        } else if relation == CorpusLibrary::NAME {
            for row in CorpusLibrary::decode(batch)? {
                if !self.corpus_inputs.contains(&row.corpus) || !self.acquired.contains(&row.library) {
                    return Err(ModelError::Invalid("corpus library needs acquired corpus and library inputs".into()));
                }
                self.corpus_libraries.insert((row.corpus, row.library));
            }
        } else if relation == InputDistribution::NAME {
            for row in InputDistribution::decode(batch)? {
                if !self.acquired.contains(&row.input) { return Err(ModelError::Invalid("distribution input is not acquired".into())); }
                if self.distributions.insert((row.input, row.release), row.role).is_some() {
                    return Err(ModelError::Invalid("distribution has contradictory input roles".into()));
                }
            }
        } else if relation == DistributionVerification::NAME {
            for row in DistributionVerification::decode(batch)? {
                let input = self.acquisitions.get(&row.acquisition)
                    .ok_or_else(|| ModelError::Invalid("distribution verification acquisition absent".into()))?;
                if !self.distributions.contains_key(&(*input, row.release)) {
                    return Err(ModelError::Invalid("verification release absent from acquired input".into()));
                }
            }
        } else if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? { self.artifacts.insert(row.id(), row.input); }
        } else if relation == ArtifactUse::NAME {
            for row in ArtifactUse::decode(batch)? {
                let input = self.artifacts.get(&row.artifact)
                    .ok_or_else(|| ModelError::Invalid("artifact use source absent".into()))?;
                if row.input != *input && !self.corpus_libraries.contains(&(row.input, *input)) {
                    return Err(ModelError::Invalid("artifact use crosses undeclared corpus/library boundary".into()));
                }
            }
        } else { return Err(ModelError::Invalid("undeclared acquisition validation input".into())); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}
