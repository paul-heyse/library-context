//! Canonical original bytes, independent of interpretation and transport batch boundaries.
use super::charged::{ChargedMap, StateCharge};
use crate::Domain;
use super::{ContentHash, EvidenceBytes, Id, Invariant, InvariantCheck, ModelError, Record, ValidationInput};
use super::source::SourceArtifact;

/// Part of the model contract, not a tunable transport batch size.
pub const ARTIFACT_CHUNK_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "artifact_chunks", validate = validate_chunk)]
pub struct ArtifactChunk {
    #[model(key)] pub artifact: Id<SourceArtifact>,
    #[model(key)] pub ordinal: i64,
    pub body: EvidenceBytes,
}
fn validate_chunk(row: &ArtifactChunk) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.body.0.is_empty() || row.body.0.len() > ARTIFACT_CHUNK_BYTES {
        return Err(ModelError::Invalid("artifact chunk needs nonnegative ordinal and 1..=1 MiB bytes".into()));
    }
    Ok(())
}
impl ArtifactChunk {
    /// Split already captured bytes lazily. Streaming acquisition can emit the same records
    /// incrementally and use ArtifactVerifier to detect a change from the captured metadata.
    pub fn split<'a>(artifact: &SourceArtifact, body: &'a [u8]) -> Result<impl Iterator<Item = Self> + 'a, ModelError> {
        artifact.validate()?;
        if usize::try_from(artifact.byte_len).ok() != Some(body.len()) || artifact.content != ContentHash::of(body) {
            return Err(ModelError::Invalid("captured artifact bytes differ from metadata".into()));
        }
        let id = artifact.id();
        Ok(body.chunks(ARTIFACT_CHUNK_BYTES).enumerate().map(move |(ordinal, bytes)| Self {
            artifact: id, ordinal: ordinal as i64, body: EvidenceBytes(bytes.to_vec()),
        }))
    }
}

/// The single content proof used by capture and sealed validation. Memory is independent of
/// artifact length. Callers supply chunks in ordinal order and must consume finish before success.
pub struct ArtifactVerifier {
    artifact: Id<SourceArtifact>, expected_digest: ContentHash, expected_len: i64,
    digest: blake3::Hasher, byte_len: i64, ordinal: i64,
}
impl ArtifactVerifier {
    pub fn new(artifact: &SourceArtifact) -> Result<Self, ModelError> {
        artifact.validate()?;
        Ok(Self { artifact: artifact.id(), expected_digest: artifact.content, expected_len: artifact.byte_len,
            digest: blake3::Hasher::new(), byte_len: 0, ordinal: 0 })
    }
    pub fn push(&mut self, chunk: &ArtifactChunk) -> Result<(), ModelError> {
        chunk.validate()?;
        let remaining = self.expected_len - self.byte_len;
        if chunk.artifact != self.artifact || chunk.ordinal != self.ordinal
            || chunk.body.0.len() as i64 != remaining.min(ARTIFACT_CHUNK_BYTES as i64) {
            return Err(ModelError::Invalid("artifact chunks are missing, misplaced or noncanonical".into()));
        }
        self.digest.update(&chunk.body.0);
        self.byte_len += chunk.body.0.len() as i64;
        self.ordinal += 1;
        Ok(())
    }
    pub fn finish(self) -> Result<(), ModelError> {
        if self.byte_len != self.expected_len || self.digest.finalize().as_bytes() != &self.expected_digest.0 {
            return Err(ModelError::Invalid("artifact chunks differ from complete length or content digest".into()));
        }
        Ok(())
    }
}

pub(crate) fn content_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "artifact_chunk_content", inputs: vec![
        ValidationInput::of::<SourceArtifact>(&["id"]),
        ValidationInput::of::<ArtifactChunk>(&["artifact", "ordinal"]),
    ], create: std::sync::Arc::new(|budget| Box::new(ArtifactContents { charge: StateCharge::new(budget, "artifact_chunk_content"), ..Default::default() })) }]
}
#[derive(Default)]
struct ArtifactContents { charge: StateCharge,
    expected: ChargedMap<Id<SourceArtifact>, (ContentHash, i64)>,
    current: Option<(Id<SourceArtifact>, ArtifactVerifier)>,
}
impl ArtifactContents {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((_, verifier)) = self.current.take() { verifier.finish()?; }
        Ok(())
    }
}
impl InvariantCheck for ArtifactContents {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for artifact in SourceArtifact::decode(batch)? {
                if self.expected.insert(&mut self.charge, artifact.id(), (artifact.content, artifact.byte_len))?.is_some() {
                    return Err(ModelError::Conflict(SourceArtifact::NAME));
                }
            }
        } else if relation == ArtifactChunk::NAME {
            for chunk in ArtifactChunk::decode(batch)? {
                if self.current.as_ref().is_none_or(|(id, _)| *id != chunk.artifact) {
                    self.flush()?;
                    let (expected_digest, expected_len) = self.expected.remove(&mut self.charge, &chunk.artifact)
                        .ok_or_else(|| ModelError::Invalid("chunk artifact absent or repeated".into()))?;
                    self.current = Some((chunk.artifact, ArtifactVerifier {
                        artifact: chunk.artifact, expected_digest, expected_len,
                        digest: blake3::Hasher::new(), byte_len: 0, ordinal: 0,
                    }));
                }
                self.current.as_mut().expect("initialized artifact").1.push(&chunk)?;
            }
        } else { return Err(ModelError::Invalid("undeclared artifact validation input".into())); }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self.expected.values().any(|(digest, len)| *len != 0 || *digest != ContentHash::of(b"")) {
            return Err(ModelError::Invalid("artifact is missing its chunks".into()));
        }
        Ok(())
    }
}

/// Converts arbitrary incoming byte fragments into canonical chunk records. At most one chunk
/// is buffered. A successful finish certifies the supplied bytes against the artifact metadata;
/// callers must abandon staged output after any feed/finish error.
pub struct ArtifactCapture {
    artifact: Id<SourceArtifact>, ordinal: i64, pending: Vec<u8>, verifier: ArtifactVerifier, failed: bool,
}
impl ArtifactCapture {
    pub fn new(artifact: &SourceArtifact) -> Result<Self, ModelError> {
        Ok(Self { artifact: artifact.id(), ordinal: 0, pending: Vec::with_capacity(ARTIFACT_CHUNK_BYTES), verifier: ArtifactVerifier::new(artifact)?, failed: false })
    }
    pub fn feed(&mut self, mut bytes: &[u8], mut emit: impl FnMut(ArtifactChunk) -> Result<(), ModelError>) -> Result<(), ModelError> {
        if self.failed { return Err(ModelError::Invalid("artifact capture already failed".into())); }
        while !bytes.is_empty() {
            let count = bytes.len().min(ARTIFACT_CHUNK_BYTES - self.pending.len());
            self.pending.extend_from_slice(&bytes[..count]); bytes = &bytes[count..];
            if self.pending.len() == ARTIFACT_CHUNK_BYTES {
                if let Err(error) = self.flush(&mut emit) { self.failed = true; return Err(error); }
            }
        }
        Ok(())
    }
    fn flush(&mut self, emit: &mut impl FnMut(ArtifactChunk) -> Result<(), ModelError>) -> Result<(), ModelError> {
        if self.pending.is_empty() { return Ok(()); }
        let chunk = ArtifactChunk { artifact: self.artifact, ordinal: self.ordinal, body: EvidenceBytes(std::mem::take(&mut self.pending)) };
        self.verifier.push(&chunk)?;
        emit(chunk)?;
        self.ordinal += 1;
        Ok(())
    }
    pub fn finish(mut self, mut emit: impl FnMut(ArtifactChunk) -> Result<(), ModelError>) -> Result<(), ModelError> {
        if self.failed { return Err(ModelError::Invalid("artifact capture already failed".into())); }
        self.flush(&mut emit)?;
        self.verifier.finish()
    }
}
