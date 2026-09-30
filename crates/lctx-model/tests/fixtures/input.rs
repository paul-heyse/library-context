//! A captured installed input for the acquisition contracts (plan A1): a distribution owns
//! `demo/__init__.py`, a loose `extra.pyi` and the README are unowned, and the README's Python block
//! is derived under `_lctx/`. A second, tree-acquired input holds `other.md`.
use arrow_array::RecordBatch;
use lctx_model::domain::{artifact::*, input::*, source::*, *};
use std::collections::BTreeMap;

pub const README: &[u8] = b"# Demo\n\n```python\nimport demo\n```\n";
pub const BLOCK: &[u8] = b"import demo\n";
pub const BLOCK_PATH: &str = "_lctx/d_README_md_00000000/block_0.py";

/// The relations the fixture fills, for a caller's typed put.
#[macro_export]
macro_rules! input_relations {
    ($apply:ident) => {
        $apply!(
            Package,
            Release,
            InputRevision,
            InputOrigin,
            InputAcquisition,
            InputDistribution,
            DistributionVerification,
            EnvironmentFingerprint,
            SourceArtifact,
            ArtifactChunk,
            ArtifactOwnership,
            UnownedArtifact,
            DerivedArtifact
        )
    };
}
#[derive(Clone)]
pub struct Fixture {
    pub package: Package,
    pub release: Release,
    pub undistributed: Release,
    pub input: InputRevision,
    pub origin: InputOrigin,
    pub acquisition: InputAcquisition,
    pub distribution: InputDistribution,
    pub verification: DistributionVerification,
    pub fingerprint: EnvironmentFingerprint,
    pub files: BTreeMap<String, Vec<u8>>,
    pub artifacts: BTreeMap<String, SourceArtifact>,
    pub ownership: Vec<ArtifactOwnership>,
    pub unowned: Vec<UnownedArtifact>,
    pub derived: Vec<DerivedArtifact>,
    pub other_input: InputRevision,
    pub other_origin: InputOrigin,
    pub other_acquisition: InputAcquisition,
    pub other: SourceArtifact,
}
impl Fixture {
    pub fn new() -> Self {
        let files: BTreeMap<String, Vec<u8>> = [
            ("demo/__init__.py", b"VALUE = 1\n".as_slice()),
            ("extra.pyi", b"X: int\n"),
            ("README.md", README),
            (BLOCK_PATH, BLOCK),
        ]
        .into_iter()
        .map(|(path, bytes)| (path.to_owned(), bytes.to_vec()))
        .collect();
        Self::with_files(files)
    }
    /// The fixture over other captured files; classes name artifacts by path and skip absent ones.
    pub fn with_files(files: BTreeMap<String, Vec<u8>>) -> Self {
        let package = Package {
            name: "demo".into(),
        };
        let release = Release {
            package: package.id(),
            version: "1.0".into(),
        };
        let undistributed = Release {
            package: package.id(),
            version: "2.0".into(),
        };
        let input = InputRevision::from_entries(
            files
                .iter()
                .map(|(path, bytes)| ManifestEntry {
                    path: path.clone(),
                    content: ContentHash::of(bytes),
                    byte_len: bytes.len() as i64,
                })
                .collect(),
        )
        .unwrap();
        let artifacts: BTreeMap<String, SourceArtifact> = files
            .iter()
            .map(|(path, bytes)| {
                (
                    path.clone(),
                    SourceArtifact::from_bytes(input.id(), path.clone(), bytes).unwrap(),
                )
            })
            .collect();
        let origin = InputOrigin::Installed {
            library: "demo".into(),
            requirement: "demo==1.0".into(),
            lock_digest: ContentHash::of(b"lock"),
            installer: Some("uv".into()),
        };
        let acquisition = InputAcquisition {
            input: input.id(),
            origin: origin.id(),
        };
        let distribution = InputDistribution {
            input: input.id(),
            release: release.id(),
            role: DistributionRole::FirstParty,
        };
        let verification = DistributionVerification {
            acquisition: acquisition.id(),
            release: release.id(),
            record_digest: ContentHash::of(b"RECORD"),
            artifact_sha256: vec![],
        };
        let fingerprint = EnvironmentFingerprint {
            acquisition: acquisition.id(),
            release: release.id(),
            lock_digest: ContentHash::of(b"lock"),
            environment_digest: input.manifest,
            python_version: "3.14.7".into(),
            platform: "linux".into(),
        };
        let id = |path: &str| artifacts.get(path).map(Record::id);
        let ownership = id("demo/__init__.py")
            .map(|artifact| ArtifactOwnership {
                artifact,
                distribution: verification.id(),
            })
            .into_iter()
            .collect();
        let unowned = ["extra.pyi", "README.md"]
            .iter()
            .filter_map(|path| id(path))
            .map(|artifact| UnownedArtifact {
                artifact,
                acquisition: acquisition.id(),
            })
            .collect();
        let derived = match (id(BLOCK_PATH), id("README.md")) {
            (Some(artifact), Some(document)) => vec![DerivedArtifact::PythonCodeBlock {
                artifact,
                document,
                ordinal: 0,
                fence_start: 8,
                fence_end: README.len() as i64,
            }],
            _ => vec![],
        };
        let other_input = InputRevision::from_entries(vec![ManifestEntry {
            path: "other.md".into(),
            content: ContentHash::of(b"other"),
            byte_len: 5,
        }])
        .unwrap();
        let other =
            SourceArtifact::from_bytes(other_input.id(), "other.md".into(), b"other").unwrap();
        let other_origin = InputOrigin::Tree {
            label: "other".into(),
        };
        let other_acquisition = InputAcquisition {
            input: other_input.id(),
            origin: other_origin.id(),
        };
        Self {
            package,
            release,
            undistributed,
            input,
            origin,
            acquisition,
            distribution,
            verification,
            fingerprint,
            files,
            artifacts,
            ownership,
            unowned,
            derived,
            other_input,
            other_origin,
            other_acquisition,
            other,
        }
    }
    pub fn artifact(&self, path: &str) -> &SourceArtifact {
        &self.artifacts[path]
    }
    /// One relation's rows, decoded from the physical batches.
    pub fn rows<R: Record>(&self) -> Vec<R> {
        let model = model().unwrap();
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 30).unwrap();
        self.batches(&model, &budget)
            .into_iter()
            .filter(|(name, _)| *name == R::NAME)
            .flat_map(|(_, batch)| R::decode(&batch).unwrap())
            .collect()
    }
    /// Every relation's rows, in the model's physical form.
    pub fn batches(
        &self,
        model: &ValidatedModel,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Vec<(&'static str, RecordBatch)> {
        let mut chunks = Vec::new();
        for (path, bytes) in &self.files {
            chunks.extend(ArtifactChunk::split(&self.artifacts[path], bytes).unwrap());
        }
        chunks.extend(ArtifactChunk::split(&self.other, b"other").unwrap());
        let mut artifacts: Vec<SourceArtifact> = self.artifacts.values().cloned().collect();
        artifacts.push(self.other.clone());
        let mut out = Vec::new();
        macro_rules! add { ($($ty:ty => $rows:expr),+ $(,)?) => { $( out.push((<$ty>::NAME, Batch::<$ty>::new(model, $rows, budget).unwrap().arrow().clone())); )+ }; }
        add!(Package => vec![self.package.clone()], Release => vec![self.release.clone(), self.undistributed.clone()],
            InputRevision => vec![self.input.clone(), self.other_input.clone()], InputOrigin => vec![self.origin.clone(), self.other_origin.clone()],
            InputAcquisition => vec![self.acquisition.clone(), self.other_acquisition.clone()], InputDistribution => vec![self.distribution.clone()],
            DistributionVerification => vec![self.verification.clone()], EnvironmentFingerprint => vec![self.fingerprint.clone()],
            SourceArtifact => artifacts, ArtifactChunk => chunks, ArtifactOwnership => self.ownership.clone(),
            UnownedArtifact => self.unowned.clone(), DerivedArtifact => self.derived.clone());
        out
    }
}
