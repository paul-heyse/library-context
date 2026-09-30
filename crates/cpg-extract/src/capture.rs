//! Capture the complete selected analyzer input before constructing any provider state.
//! The provider reads only the owned frozen tree. Paths and acquisition labels never replace
//! byte identity. The caller selects the complete input inventory and quiesces its acquisition;
//! this is a checked copy of that inventory, not a filesystem-wide atomic snapshot.
use lctx_model::domain::{
    artifact::{ARTIFACT_CHUNK_BYTES, ArtifactCapture, ArtifactChunk},
    input::{DERIVED_ROOT, InputRevision, ManifestEntry, validate_path},
    resources::{Reservation, ResourceBudget},
    source::SourceArtifact,
    *,
};
use std::{
    fs::{self, File, Metadata},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const BUFFER_BYTES: usize = 64 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("input changed during capture: {0}")]
    Changed(PathBuf),
    #[error("input must contain only regular files under normalized paths: {0}")]
    Unsupported(PathBuf),
    /// An original path inside the reserved derived namespace, or a derivation outside it.
    #[error("the {DERIVED_ROOT} namespace is reserved for derived artifacts: {0}")]
    Reserved(String),
}
/// One artifact the compiler derives from a captured document: written only into the frozen copy's
/// reserved `_lctx/` namespace, never into the source tree (ADR-0089). The fence is the derived
/// bytes' origin span in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    pub path: String,
    pub document: String,
    pub ordinal: i64,
    pub fence: (i64, i64),
    pub bytes: Vec<u8>,
}
/// A derivation's provenance, retained with the capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Derivation {
    PythonCodeBlock {
        path: String,
        document: String,
        ordinal: i64,
        fence: (i64, i64),
    },
    TaskReceipt {
        path: String,
    },
}
impl Derivation {
    pub fn path(&self) -> &str {
        match self {
            Self::PythonCodeBlock { path, .. } | Self::TaskReceipt { path } => path,
        }
    }
}
/// Owns its frozen tree and the reservation for retained artifact metadata. Keeping this value
/// alive keeps provider paths valid; dropping it deletes only its own temporary directory.
pub struct CapturedInput {
    directory: tempfile::TempDir,
    revision: InputRevision,
    artifacts: Vec<SourceArtifact>,
    derivations: Vec<Derivation>,
    budget: ResourceBudget,
    _metadata: Box<dyn Reservation>,
}
impl CapturedInput {
    pub fn capture(
        root: &Path,
        paths: &[String],
        budget: &ResourceBudget,
    ) -> Result<Self, CaptureError> {
        Self::capture_with(root, paths, budget, |_| Ok(()), &[], |_, _| Ok(Vec::new()))
    }
    /// Capture `paths`, then derive artifacts from the frozen copy of each of `documents`: `derive`
    /// receives a document's path and frozen bytes, and each derived artifact is written read-only
    /// under `_lctx/` in the frozen copy and joins the input's manifest.
    pub fn capture_derived(
        root: &Path,
        paths: &[String],
        budget: &ResourceBudget,
        documents: &[String],
        derive: impl FnMut(&str, &[u8]) -> Result<Vec<Derived>, CaptureError>,
    ) -> Result<Self, CaptureError> {
        Self::capture_with(root, paths, budget, |_| Ok(()), documents, derive)
    }
    fn capture_with(
        root: &Path,
        paths: &[String],
        budget: &ResourceBudget,
        mut after_copy: impl FnMut(&Path) -> Result<(), CaptureError>,
        documents: &[String],
        mut derive: impl FnMut(&str, &[u8]) -> Result<Vec<Derived>, CaptureError>,
    ) -> Result<Self, CaptureError> {
        if let Some(document) = documents.iter().find(|d| !paths.contains(d)) {
            return Err(ModelError::Invalid(format!(
                "derivation document {document} is not captured"
            ))
            .into());
        }
        if let Some(reserved) = paths.iter().find(|path| path.starts_with(DERIVED_ROOT)) {
            return Err(CaptureError::Reserved(reserved.clone()));
        }
        if fs::symlink_metadata(root)?.file_type().is_symlink() || !root.is_dir() {
            return Err(CaptureError::Unsupported(root.into()));
        }
        let root = root.canonicalize()?;
        // Admission precedes the new inventory, hash buffer and cloned metadata allocations.
        // Input inventory ownership is the acquisition caller's reservation, not this copy's.
        let metadata_bytes = paths.iter().try_fold(4096usize, |sum, path| {
            validate_path(path)?;
            let strings = path
                .len()
                .checked_add(root.as_os_str().len())
                .and_then(|n| n.checked_mul(4));
            let overhead = 4 * std::mem::size_of::<SourceArtifact>()
                + 2 * std::mem::size_of::<Stamp>()
                + 2 * std::mem::size_of::<ManifestEntry>();
            strings
                .and_then(|n| n.checked_add(overhead))
                .and_then(|n| sum.checked_add(n))
                .ok_or_else(|| ModelError::Invalid("capture metadata accounting overflow".into()))
        })?;
        let mut metadata = budget.reserve("captured-input-metadata", metadata_bytes)?;
        let _buffer_charge = budget.reserve("input-capture-buffer", BUFFER_BYTES)?;
        let mut buffer = vec![0u8; BUFFER_BYTES];
        let mut ordered: Vec<_> = paths.iter().collect();
        ordered.sort_unstable();
        if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ModelError::Invalid("duplicate input capture path".into()).into());
        }
        let directory = private_directory()?;
        let mut entries = Vec::with_capacity(paths.len());
        let mut stamps = Vec::with_capacity(paths.len());
        for path in ordered {
            let original = regular_file(&root, path)?;
            let before = Stamp::of(&fs::metadata(&original)?)?;
            let mut input = File::open(&original)?;
            if Stamp::of(&input.metadata()?)? != before {
                return Err(CaptureError::Changed(original));
            }
            let target = directory.path().join(path);
            fs::create_dir_all(target.parent().expect("artifact parent"))?;
            let mut output = File::create(&target)?;
            // Never chase an append-only writer indefinitely. One byte past the admitted size
            // detects growth, then the fingerprint/length check aborts this copy.
            let read_limit = before
                .len
                .checked_add(1)
                .ok_or_else(|| ModelError::Invalid("artifact length overflow".into()))?;
            let (content, byte_len) =
                copy_digest(&mut (&mut input).take(read_limit), &mut output, &mut buffer)?;
            output.flush()?;
            if Stamp::of(&input.metadata()?)? != before || byte_len as u64 != before.len {
                return Err(CaptureError::Changed(original));
            }
            drop(output);
            let mut permissions = fs::metadata(&target)?.permissions();
            permissions.set_readonly(true);
            fs::set_permissions(&target, permissions)?;
            entries.push(ManifestEntry {
                path: path.clone(),
                content,
                byte_len,
            });
            stamps.push((path.clone(), before));
            after_copy(&original)?;
        }
        // A change to an earlier file while later files were copied invalidates the whole attempt.
        for (relative, before) in &stamps {
            let path = regular_file(&root, relative)?;
            if Stamp::of(&fs::metadata(&path)?)? != *before {
                return Err(CaptureError::Changed(path));
            }
        }
        // Derivations read the frozen copies, so the source tree is never read twice or written.
        let mut derivations = Vec::new();
        // Original metadata admission covers the temporary clone; derived metadata grows before retention.
        for entry in entries
            .clone()
            .into_iter()
            .filter(|entry| documents.contains(&entry.path))
        {
            let _document = budget.reserve(
                "derivation-document",
                usize::try_from(entry.byte_len)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(32)
                    .saturating_add(4096),
            )?;
            let frozen = fs::read(directory.path().join(&entry.path))?;
            for derived in derive(&entry.path, &frozen)? {
                validate_path(&derived.path)?;
                if !derived.path.starts_with(DERIVED_ROOT) || derived.document != entry.path {
                    return Err(CaptureError::Reserved(derived.path));
                }
                let target = directory.path().join(&derived.path);
                if target.exists() {
                    return Err(CaptureError::Reserved(derived.path));
                }
                fs::create_dir_all(target.parent().expect("derived parent"))?;
                fs::write(&target, &derived.bytes)?;
                let mut permissions = fs::metadata(&target)?.permissions();
                permissions.set_readonly(true);
                fs::set_permissions(&target, permissions)?;
                let byte_len = i64::try_from(derived.bytes.len())
                    .map_err(|_| ModelError::Invalid("artifact length overflow".into()))?;
                metadata.try_resize(
                    metadata
                        .size()
                        .checked_add(
                            (derived.path.len()
                                + derived.document.len()
                                + size_of::<Derivation>()
                                + size_of::<ManifestEntry>()
                                + size_of::<SourceArtifact>()
                                + 256)
                                .saturating_mul(4),
                        )
                        .ok_or_else(|| ModelError::Invalid("derived metadata overflow".into()))?,
                )?;
                entries.push(ManifestEntry {
                    path: derived.path.clone(),
                    content: ContentHash::of(&derived.bytes),
                    byte_len,
                });
                derivations.push(Derivation::PythonCodeBlock {
                    path: derived.path,
                    document: derived.document,
                    ordinal: derived.ordinal,
                    fence: derived.fence,
                });
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let revision = InputRevision::from_entries(entries.clone())?;
        let artifacts: Vec<_> = entries
            .into_iter()
            .map(|entry| SourceArtifact {
                input: revision.id(),
                path: entry.path,
                content: entry.content,
                byte_len: entry.byte_len,
            })
            .collect();
        for artifact in &artifacts {
            artifact.validate()?;
        }
        let retained = 4096
            + artifacts.capacity() * std::mem::size_of::<SourceArtifact>()
            + artifacts.iter().map(|a| a.path.capacity()).sum::<usize>()
            + derivations.capacity() * size_of::<Derivation>()
            + derivations
                .iter()
                .map(|d| match d {
                    Derivation::PythonCodeBlock { path, document, .. } => {
                        path.capacity() + document.capacity()
                    }
                    Derivation::TaskReceipt { path } => path.capacity(),
                })
                .sum::<usize>();
        // Temporary metadata must be released before shrinking its reservation.
        drop(stamps);
        metadata.try_resize(retained)?;
        Ok(Self {
            directory,
            revision,
            artifacts,
            derivations,
            budget: budget.clone(),
            _metadata: metadata,
        })
    }
    /// Import explicitly supplied receipt bytes before providers are created. Byte-addressed
    /// names make receipt order and external filesystem location irrelevant. Interpretation is
    /// the deployment provider's job, including malformed and oversized receipt diagnostics.
    pub fn with_receipts(mut self, paths: &[PathBuf]) -> Result<Self, CaptureError> {
        let _buffer_charge = self
            .budget
            .reserve("receipt-capture-buffer", BUFFER_BYTES)?;
        let mut buffer = vec![0u8; BUFFER_BYTES];
        let cloned_metadata = self
            .artifacts
            .iter()
            .map(|a| a.path.len() + size_of::<ManifestEntry>() + 128)
            .sum::<usize>()
            .saturating_mul(4);
        let _cloned = self
            .budget
            .reserve("receipt-revision-metadata", cloned_metadata)?;
        let mut entries = self
            .artifacts
            .iter()
            .map(|a| ManifestEntry {
                path: a.path.clone(),
                content: a.content,
                byte_len: a.byte_len,
            })
            .collect::<Vec<_>>();
        for (ordinal, path) in paths.iter().enumerate() {
            if fs::symlink_metadata(path)?.file_type().is_symlink() || !path.is_file() {
                return Err(CaptureError::Unsupported(path.clone()));
            }
            self._metadata.try_resize(
                self._metadata
                    .size()
                    .checked_add(path.as_os_str().len() + 4096)
                    .ok_or_else(|| ModelError::Invalid("receipt metadata overflow".into()))?,
            )?;
            let before = Stamp::of(&fs::metadata(path)?)?;
            let mut input = File::open(path)?;
            if Stamp::of(&input.metadata()?)? != before {
                return Err(CaptureError::Changed(path.clone()));
            }
            let temporary = self
                .root()
                .join(format!("_lctx/task-receipts/capture-{ordinal}"));
            fs::create_dir_all(temporary.parent().unwrap())?;
            let mut output = File::create(&temporary)?;
            let (content, byte_len) = copy_digest(
                &mut (&mut input).take(
                    before
                        .len
                        .checked_add(1)
                        .ok_or_else(|| ModelError::Invalid("receipt length overflow".into()))?,
                ),
                &mut output,
                &mut buffer,
            )?;
            output.flush()?;
            drop(output);
            if Stamp::of(&input.metadata()?)? != before
                || byte_len as u64 != before.len
                || Stamp::of(&fs::metadata(path)?)? != before
            {
                return Err(CaptureError::Changed(path.clone()));
            }
            let logical = format!(
                "_lctx/task-receipts/{}.json",
                content
                    .0
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            let target = self.root().join(&logical);
            if target.exists() {
                fs::remove_file(temporary)?;
                continue;
            }
            fs::rename(temporary, &target)?;
            let mut permissions = fs::metadata(&target)?.permissions();
            permissions.set_readonly(true);
            fs::set_permissions(target, permissions)?;
            entries.push(ManifestEntry {
                path: logical.clone(),
                content,
                byte_len,
            });
            self.derivations
                .push(Derivation::TaskReceipt { path: logical });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        self.revision = InputRevision::from_entries(entries.clone())?;
        self.artifacts = entries
            .into_iter()
            .map(|entry| SourceArtifact {
                input: self.revision.id(),
                path: entry.path,
                content: entry.content,
                byte_len: entry.byte_len,
            })
            .collect();
        self.derivations.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(self)
    }
    pub fn revision(&self) -> &InputRevision {
        &self.revision
    }
    pub fn artifacts(&self) -> &[SourceArtifact] {
        &self.artifacts
    }
    /// The artifacts derived into `_lctx/`, with their provenance.
    pub fn derivations(&self) -> &[Derivation] {
        &self.derivations
    }
    /// Read-only provider input. Retain this CapturedInput while any provider holds this path.
    pub fn root(&self) -> &Path {
        self.directory.path()
    }
    /// Feed canonical chunks through a borrowed callback. A retaining consumer must reserve its
    /// own copies. The capture owns two possible chunk buffers and the read buffer during emission.
    /// Any error invalidates the surrounding staging attempt, including earlier callback output.
    pub fn emit_chunks(
        &self,
        index: usize,
        mut output: impl FnMut(&ArtifactChunk) -> Result<(), ModelError>,
    ) -> Result<usize, CaptureError> {
        let artifact = self
            .artifacts
            .get(index)
            .ok_or_else(|| ModelError::Invalid("unknown captured artifact".into()))?;
        let _charge = self.budget.reserve(
            "captured-artifact-stream",
            2 * ARTIFACT_CHUNK_BYTES + BUFFER_BYTES,
        )?;
        let mut buffer = vec![0u8; BUFFER_BYTES];
        let mut file = File::open(regular_file(self.root(), &artifact.path)?)?;
        let mut capture = ArtifactCapture::new(artifact)?;
        let mut count = 0usize;
        let mut emit = |chunk: ArtifactChunk| {
            output(&chunk)?;
            count += 1;
            Ok(())
        };
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            capture.feed(&buffer[..read], &mut emit)?;
        }
        capture.finish(&mut emit)?;
        Ok(count)
    }
    /// Recheck frozen bytes after a provider finishes, before its rows can publish.
    pub fn verify(&self) -> Result<(), CaptureError> {
        for index in 0..self.artifacts.len() {
            self.emit_chunks(index, |_| Ok(()))?;
        }
        Ok(())
    }
}
fn private_directory() -> std::io::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("lctx-input-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    builder.tempdir()
}
fn regular_file(root: &Path, relative: &str) -> Result<PathBuf, CaptureError> {
    validate_path(relative)?;
    let mut path = root.to_path_buf();
    for component in relative.split('/') {
        path.push(component);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(CaptureError::Unsupported(path));
        }
    }
    if !fs::metadata(&path)?.is_file() {
        return Err(CaptureError::Unsupported(path));
    }
    Ok(path)
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}
impl Stamp {
    fn of(metadata: &Metadata) -> std::io::Result<Self> {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            identity: (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            ),
        })
    }
}
fn copy_digest(
    input: &mut impl Read,
    output: &mut File,
    buffer: &mut [u8],
) -> Result<(ContentHash, i64), CaptureError> {
    let mut digest = ContentHasher::default();
    let mut length = 0i64;
    loop {
        let read = input.read(buffer)?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as i64)
            .ok_or_else(|| ModelError::Invalid("artifact length overflow".into()))?;
        digest.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    Ok((digest.finish(), length))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_refuses_changed_input_and_cleans_reservations() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("input.py"), b"before").unwrap();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let result = CapturedInput::capture_with(
            root.path(),
            &["input.py".into()],
            &budget,
            |original| {
                fs::write(original, b"changed during capture")?;
                Ok(())
            },
            &[],
            |_, _| Ok(Vec::new()),
        );
        assert!(matches!(result, Err(CaptureError::Changed(_))));
        assert_eq!(budget.reserved(), 0);
    }
}
