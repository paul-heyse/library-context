//! Canonical acquisition emission inventory, separate from immutable file byte identity.
use super::super::{input::*, source::SourceArtifact, *};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedAcquisition {
    pub revision: InputRevision,
    pub origin: InputOrigin,
    /// The acquired library's content and origin identities, never a caller vector index.
    pub library: Option<(Id<InputRevision>, Id<InputOrigin>)>,
    pub configuration: Option<ContentHash>,
    pub interpreter: Option<(String, String)>,
    pub artifacts: Vec<SourceArtifact>,
    pub selected_absent: BTreeSet<String>,
    pub record_sha256: BTreeMap<String, String>,
    pub packages: Vec<Package>,
    pub releases: Vec<Release>,
    pub distributions: Vec<InputDistribution>,
    pub verifications: Vec<DistributionVerification>,
    pub fingerprints: Vec<EnvironmentFingerprint>,
    pub ownership: Vec<ArtifactOwnership>,
    pub unowned: Vec<UnownedArtifact>,
    pub derivations: Vec<DerivedArtifact>,
    pub uses: Vec<ArtifactUse>,
}
impl CapturedAcquisition {
    /// Original or independently empty tree; callers add its exact typed population before binding.
    pub fn tree(revision: InputRevision, label: String) -> Self {
        Self {
            revision,
            origin: InputOrigin::Tree { label },
            library: None,
            configuration: None,
            interpreter: None,
            artifacts: vec![],
            selected_absent: BTreeSet::new(),
            record_sha256: BTreeMap::new(),
            packages: vec![],
            releases: vec![],
            distributions: vec![],
            verifications: vec![],
            fingerprints: vec![],
            ownership: vec![],
            unowned: vec![],
            derivations: vec![],
            uses: vec![],
        }
    }
    /// Sets are canonicalized; the outer captured-input order and multiplicity are preserved.
    pub fn canonicalize(&mut self) {
        self.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        macro_rules! rows { ($($field:ident),*) => {$(self.$field.sort_by_key(Record::id); self.$field.dedup();)*}; }
        rows!(
            packages,
            releases,
            distributions,
            verifications,
            fingerprints,
            ownership,
            unowned,
            derivations,
            uses
        );
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        macro_rules! ordered {($($field:ident),*)=>{$(if self.$field.windows(2).any(|pair|pair[0].id()>=pair[1].id()){return Err(ModelError::Conflict("captured acquisition canonical rows"));})*};}
        ordered!(
            packages,
            releases,
            distributions,
            verifications,
            fingerprints,
            ownership,
            unowned,
            derivations,
            uses
        );
        if self
            .artifacts
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
            || InputRevision::from_entries(
                self.artifacts
                    .iter()
                    .map(SourceArtifact::manifest_entry)
                    .collect(),
            )? != self.revision
        {
            return Err(ModelError::Conflict(
                "captured acquisition canonical manifest",
            ));
        }
        let paths = self
            .artifacts
            .iter()
            .map(|a| a.path.as_str())
            .collect::<BTreeSet<_>>();
        let ids = self
            .artifacts
            .iter()
            .map(Record::id)
            .collect::<BTreeSet<_>>();
        let acquisition = InputAcquisition {
            input: self.revision.id(),
            origin: self.origin.id(),
        }
        .id();
        for artifact in &self.artifacts {
            artifact.validate()?;
            if artifact.input != self.revision.id() {
                return Err(ModelError::Conflict("captured acquisition artifact input"));
            }
        }
        for path in &self.selected_absent {
            validate_path(path)?;
            if paths.contains(path.as_str()) {
                return Err(ModelError::Conflict("selected absent artifact is present"));
            }
        }
        if self
            .record_sha256
            .keys()
            .any(|p| !paths.contains(p.as_str()))
            || self
                .uses
                .iter()
                .any(|u| !ids.contains(&u.artifact) || u.input != self.revision.id())
            || self
                .unowned
                .iter()
                .any(|u| !ids.contains(&u.artifact) || u.acquisition != acquisition)
            || self.ownership.iter().any(|u| {
                !ids.contains(&u.artifact)
                    || !self.verifications.iter().any(|v| v.id() == u.distribution)
            })
            || self.verifications.iter().any(|v| {
                v.acquisition != acquisition || !self.releases.iter().any(|r| r.id() == v.release)
            })
            || self.distributions.iter().any(|d| {
                d.input != self.revision.id() || !self.releases.iter().any(|r| r.id() == d.release)
            })
        {
            return Err(ModelError::Conflict("captured acquisition attribution"));
        }
        let derived = self
            .derivations
            .iter()
            .map(|d| match d {
                DerivedArtifact::PythonCodeBlock { artifact, .. }
                | DerivedArtifact::TaskReceipt { artifact, .. } => *artifact,
            })
            .collect::<BTreeSet<_>>();
        let owned = self
            .ownership
            .iter()
            .map(|u| u.artifact)
            .collect::<BTreeSet<_>>();
        let unowned = self
            .unowned
            .iter()
            .map(|u| u.artifact)
            .collect::<BTreeSet<_>>();
        if !owned.is_disjoint(&unowned)
            || self.artifacts.iter().any(|a| {
                !derived.contains(&a.id()) && !owned.contains(&a.id()) && !unowned.contains(&a.id())
            })
            || self
                .releases
                .iter()
                .any(|r| !self.packages.iter().any(|p| p.id() == r.package))
            || self.fingerprints.iter().any(|f| {
                f.acquisition != acquisition
                    || f.environment_digest != self.revision.manifest
                    || !self.releases.iter().any(|r| r.id() == f.release)
            })
        {
            return Err(ModelError::Conflict(
                "captured acquisition ownership or environment",
            ));
        }
        for derivation in &self.derivations {
            derivation.validate()?;
            match derivation {
                DerivedArtifact::PythonCodeBlock {
                    artifact, document, ..
                } if !ids.contains(artifact) || !ids.contains(document) => {
                    return Err(ModelError::Conflict("captured derivation artifacts"));
                }
                DerivedArtifact::TaskReceipt { artifact, input }
                    if !ids.contains(artifact) || *input != self.revision.id() =>
                {
                    return Err(ModelError::Conflict("captured receipt artifacts"));
                }
                _ => {}
            }
        }
        for package in &self.packages {
            package.validate()?;
        }
        for release in &self.releases {
            release.validate()?;
        }
        for fingerprint in &self.fingerprints {
            fingerprint.validate()?;
        }
        for verification in &self.verifications {
            verification.validate()?;
        }
        Ok(())
    }
    pub fn heap_bytes(&self) -> usize {
        let origin = match &self.origin {
            InputOrigin::Installed {
                library,
                requirement,
                installer,
                ..
            } => {
                library.capacity()
                    + requirement.capacity()
                    + installer.as_ref().map_or(0, String::capacity)
            }
            InputOrigin::Tree { label } => label.capacity(),
            InputOrigin::Corpus {
                repository,
                revision,
            } => repository.capacity() + revision.capacity(),
        };
        let mut bytes = origin
            + self.interpreter.as_ref().map_or(0, |(python, platform)| {
                python.capacity() + platform.capacity()
            });
        // Inline element storage plus retained strings; no allocation is needed for accounting.
        bytes += self.artifacts.capacity() * size_of::<SourceArtifact>()
            + self
                .artifacts
                .iter()
                .map(|a| a.path.capacity())
                .sum::<usize>();
        bytes += self
            .selected_absent
            .iter()
            .map(|s| s.capacity() + 64)
            .sum::<usize>();
        bytes += self
            .record_sha256
            .iter()
            .map(|(p, h)| p.capacity() + h.capacity() + 96)
            .sum::<usize>();
        bytes += self.packages.capacity() * size_of::<Package>()
            + self
                .packages
                .iter()
                .map(|p| p.name.capacity())
                .sum::<usize>();
        bytes += self.releases.capacity() * size_of::<Release>()
            + self
                .releases
                .iter()
                .map(|r| r.version.capacity())
                .sum::<usize>();
        bytes += self.verifications.capacity() * size_of::<DistributionVerification>()
            + self
                .verifications
                .iter()
                .map(|v| {
                    v.artifact_sha256.capacity() * size_of::<String>()
                        + v.artifact_sha256
                            .iter()
                            .map(String::capacity)
                            .sum::<usize>()
                })
                .sum::<usize>();
        bytes += self.fingerprints.capacity() * size_of::<EnvironmentFingerprint>()
            + self
                .fingerprints
                .iter()
                .map(|f| f.python_version.capacity() + f.platform.capacity())
                .sum::<usize>();
        bytes += self.distributions.capacity() * size_of::<InputDistribution>()
            + self.ownership.capacity() * size_of::<ArtifactOwnership>()
            + self.unowned.capacity() * size_of::<UnownedArtifact>()
            + self.derivations.capacity() * size_of::<DerivedArtifact>()
            + self.uses.capacity() * size_of::<ArtifactUse>();
        bytes
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        self.validate()?;
        let mut sink = KeySink::new("captured-acquisition/v1");
        sink.part(
            b"typed-emission-inventory",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
}

/// Share one immutable descriptor across producer declarations without changing its wire shape.
pub(crate) mod shared {
    use super::*;
    use std::sync::Arc;
    pub fn serialize<S: serde::Serializer>(
        value: &Arc<CapturedAcquisition>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_ref().serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Arc<CapturedAcquisition>, D::Error> {
        CapturedAcquisition::deserialize(deserializer).map(Arc::new)
    }
}
