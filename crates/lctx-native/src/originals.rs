//! Shared original bytes. Logical artifact identity retains its exact canonical source payload;
//! only the full-content-addressed blob owns physical bytes. Completion is earned by readback.
use lctx_model::domain::{
    ContentHash, ContentHasher, KeySink, ModelError, Record,
    graph::{Entity, EntityId},
    resources::ResourceBudget,
    source::SourceArtifact,
};
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::{Bytes, Object, RecordId, SurrealValue, Value, Variables};
use surrealdb::{Surreal, engine::remote::grpc::Client};

pub const CHUNK_BYTES: usize = 65536;
pub const FORMAT: &str = "OriginalBlob/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blob {
    pub content: ContentHash,
    pub byte_len: u64,
}
impl Blob {
    pub fn new(content: ContentHash, byte_len: u64) -> Result<Self, ModelError> {
        if byte_len > i64::MAX as u64 || (byte_len == 0 && content != ContentHash::of(b"")) {
            return Err(ModelError::Schema("original blob declaration"));
        }
        Ok(Self { content, byte_len })
    }
    pub fn id(&self) -> RecordId {
        let mut key = KeySink::new("original-blob/v1");
        key.part(b"content", &self.content.0);
        key.part(b"byte_len", &self.byte_len.to_le_bytes());
        key.part(b"format", FORMAT.as_bytes());
        RecordId::new("original_blob", key.finish().hex())
    }
    pub fn value(&self, completed: bool) -> Value {
        let mut value = Object::new();
        value.insert("id", self.id());
        value.insert("content", self.content.hex());
        value.insert("byte_len", self.byte_len as i64);
        value.insert("format", FORMAT);
        value.insert("completed", completed);
        Value::Object(value)
    }
    pub fn piece_count(&self) -> u64 {
        self.byte_len.div_ceil(CHUNK_BYTES as u64)
    }
    pub fn piece_len(&self, start: u64) -> Result<usize, ModelError> {
        if !start.is_multiple_of(CHUNK_BYTES as u64) || start >= self.byte_len {
            return Err(ModelError::Schema("original blob piece position"));
        }
        Ok((self.byte_len - start).min(CHUNK_BYTES as u64) as usize)
    }
    pub fn piece(&self, start: u64, bytes: &[u8]) -> Result<Value, ModelError> {
        if bytes.len() != self.piece_len(start)? {
            return Err(ModelError::Conflict("original blob piece length"));
        }
        let mut row = Object::new();
        row.insert("id", chunk_id(&self.id(), start)?);
        row.insert("blob", self.id());
        row.insert("start", start as i64);
        row.insert("bytes", Bytes::from(bytes.to_vec()));
        row.insert("content", ContentHash::of(bytes).hex());
        Ok(Value::Object(row))
    }
}
pub fn chunk_id(blob: &RecordId, start: u64) -> Result<RecordId, ModelError> {
    if blob.table.as_str() != "original_blob"
        || !start.is_multiple_of(CHUNK_BYTES as u64)
        || start > i64::MAX as u64
    {
        return Err(ModelError::Schema("original blob chunk address"));
    }
    let mut key = KeySink::new("original-blob-chunk/v1");
    key.part(
        b"blob",
        &serde_json::to_vec(blob).map_err(ModelError::codec)?,
    );
    key.part(b"offset", &start.to_le_bytes());
    Ok(RecordId::new("original_chunk", key.finish().hex()))
}
#[derive(Clone, Debug)]
pub struct Layout {
    pub id: RecordId,
    pub artifact: RecordId,
    pub blob: Blob,
}
impl Layout {
    pub fn source(row: &SourceArtifact) -> Result<Self, ModelError> {
        row.validate()?;
        Ok(Self {
            id: RecordId::new("original", EntityId::of(row.id()).0.hex()),
            artifact: crate::loader::entity_payload_id(&Entity::Source(row.clone()))?,
            blob: Blob::new(row.content, row.byte_len as u64)?,
        })
    }
    pub fn value(&self) -> Value {
        let mut row = Object::new();
        row.insert("id", self.id.clone());
        row.insert("artifact", self.artifact.clone());
        row.insert("blob", self.blob.id());
        row.insert("content", self.blob.content.hex());
        row.insert("byte_len", self.blob.byte_len as i64);
        Value::Object(row)
    }
    pub fn validate_source(&self, source: &SourceArtifact) -> Result<(), ModelError> {
        exact(
            &self.value(),
            &Self::source(source)?.value(),
            "original source layout authority",
        )
    }
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct BlobRow {
    id: RecordId,
    content: String,
    byte_len: i64,
    format: String,
    completed: bool,
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct LayoutRow {
    id: RecordId,
    artifact: RecordId,
    blob: RecordId,
    content: String,
    byte_len: i64,
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct Piece {
    pub id: RecordId,
    pub blob: RecordId,
    pub start: u64,
    pub content: String,
    pub bytes: Bytes,
}
fn hash(text: &str) -> Result<ContentHash, ModelError> {
    let bytes = hex::decode(text).map_err(ModelError::codec)?;
    let result = ContentHash(
        bytes
            .try_into()
            .map_err(|_| ModelError::Schema("original content width"))?,
    );
    if result.hex() != text {
        return Err(ModelError::Schema("original content encoding"));
    }
    Ok(result)
}
fn exact(actual: &Value, expected: &Value, owner: &'static str) -> Result<(), ModelError> {
    if actual != expected {
        return Err(ModelError::Conflict(owner));
    }
    Ok(())
}
pub fn decode_blob(value: Value) -> Result<(Blob, bool), ModelError> {
    let row = BlobRow::from_value(value.clone()).map_err(ModelError::codec)?;
    let blob = Blob::new(
        hash(&row.content)?,
        u64::try_from(row.byte_len).map_err(ModelError::codec)?,
    )?;
    if row.id != blob.id() || row.format != FORMAT {
        return Err(ModelError::Schema("original blob identity"));
    }
    exact(
        &value,
        &blob.value(row.completed),
        "original blob complete envelope",
    )?;
    Ok((blob, row.completed))
}
pub fn decode_layout(value: Value) -> Result<Layout, ModelError> {
    let row = LayoutRow::from_value(value.clone()).map_err(ModelError::codec)?;
    if row.id.table.as_str() != "original" || row.artifact.table.as_str() != "entity" {
        return Err(ModelError::Schema("original layout identity"));
    }
    let layout = Layout {
        id: row.id,
        artifact: row.artifact,
        blob: Blob::new(
            hash(&row.content)?,
            u64::try_from(row.byte_len).map_err(ModelError::codec)?,
        )?,
    };
    if row.blob != layout.blob.id() {
        return Err(ModelError::Conflict("original layout blob"));
    }
    exact(&value, &layout.value(), "original layout complete envelope")?;
    Ok(layout)
}
pub fn decode_piece(value: Value, blob: &Blob) -> Result<Piece, ModelError> {
    let row = Piece::from_value(value.clone()).map_err(ModelError::codec)?;
    if row.blob != blob.id() {
        return Err(ModelError::Conflict("original piece parent"));
    }
    exact(
        &value,
        &blob.piece(row.start, &row.bytes)?,
        "original piece complete envelope",
    )?;
    Ok(row)
}
async fn rows<Context>(
    reader: &crate::NativeReader<Context>,
    sql: &str,
    vars: Variables,
) -> Result<Vec<Value>, ModelError> {
    reader.query_native(sql, vars).await
}
pub async fn layout<Context>(
    reader: &crate::NativeReader<Context>,
    id: &RecordId,
) -> Result<Option<Layout>, ModelError> {
    let mut vars = Variables::new();
    vars.insert("id", id.clone());
    let values = rows(reader, "SELECT * FROM $id", vars).await?;
    match values.as_slice() {
        [] => Ok(None),
        [value] => decode_layout(value.clone()).map(Some),
        _ => Err(ModelError::Conflict("original layout inventory")),
    }
}
/// The layout points at a full immutable payload, never an unqualified nominal anchor.
pub async fn validate_layout<Context>(
    reader: &crate::NativeReader<Context>,
    layout: &Layout,
) -> Result<(), ModelError> {
    let mut vars = Variables::new();
    vars.insert("id", layout.artifact.clone());
    let values = rows(reader, "SELECT * FROM $id", vars).await?;
    let [Value::Object(row)] = values.as_slice() else {
        return Err(ModelError::Conflict("original source payload missing"));
    };
    let Some(Value::Bytes(canonical)) = row.get("canonical") else {
        return Err(ModelError::Schema("original source canonical payload"));
    };
    let budget = reader.resource_budget()?;
    let _decode = budget.reserve(
        "original source payload decode",
        canonical.len().saturating_mul(8).saturating_add(4096),
    )?;
    let entity: Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
    entity.validate()?;
    if crate::loader::entity_payload_id(&entity)? != layout.artifact {
        return Err(ModelError::Conflict("original source payload identity"));
    }
    let Entity::Source(source) = entity else {
        return Err(ModelError::Schema("original source payload kind"));
    };
    layout.validate_source(&source)
}
pub async fn resolve_source<Context>(
    reader: &crate::NativeReader<Context>,
    source: ContentHash,
    content: ContentHash,
    length: u64,
    budget: &ResourceBudget,
) -> Result<Layout, ModelError> {
    let mut vars = Variables::new();
    vars.insert(
        "anchor",
        crate::reader::target_id(lctx_model::domain::graph::Target::Entity(EntityId(source))),
    );
    let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(vars,vec![],vec!["SELECT id,canonical FROM entity WITH INDEX anchor_payload WHERE anchor=$anchor AND semantic_type='source_artifacts' ORDER BY id".into()])?)?;
    let result = async {
        let mut found = None;
        while let Some(value) = stream.next().await? {
            let Value::Object(row) = value else {
                return Err(ModelError::Schema("original source candidate"));
            };
            let id = RecordId::from_value(
                row.get("id")
                    .cloned()
                    .ok_or(ModelError::Schema("original source candidate identity"))?,
            )
            .map_err(ModelError::codec)?;
            // Ingress claims the exact typed source payload; completed-view admission is earned separately.

            let Some(Value::Bytes(canonical)) = row.get("canonical") else {
                return Err(ModelError::Schema("original source candidate canonical"));
            };
            let _decode = budget.reserve(
                "original source candidate decode",
                canonical.len().saturating_mul(8).saturating_add(4096),
            )?;
            let entity: Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
            entity.validate()?;
            let Entity::Source(source_row) = entity else {
                return Err(ModelError::Schema("original source candidate kind"));
            };
            let layout = Layout::source(&source_row)?;
            if layout.artifact != id
                || layout.id != RecordId::new("original", source.hex())
                || layout.blob.content != content
                || layout.blob.byte_len != length
            {
                return Err(ModelError::Conflict("original source declaration"));
            }
            if found.replace(layout).is_some() {
                return Err(ModelError::Conflict("ambiguous original source payload"));
            }
        }
        found.ok_or(ModelError::Conflict("original source payload missing"))
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "original source candidate drainage",
        stream.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)
}
pub async fn header<Context>(
    reader: &crate::NativeReader<Context>,
    blob: &Blob,
) -> Result<bool, ModelError> {
    let mut vars = Variables::new();
    vars.insert("id", blob.id());
    let values = rows(reader, "SELECT * FROM $id", vars).await?;
    let [value] = values.as_slice() else {
        return Err(ModelError::Conflict("original blob header"));
    };
    let (actual, completed) = decode_blob(value.clone())?;
    if &actual != blob {
        return Err(ModelError::Conflict("original blob declaration"));
    }
    Ok(completed)
}
/// Full readback earns completion independently of the producer's streaming hash.
pub async fn validate_blob<Context>(
    reader: &crate::NativeReader<Context>,
    blob: &Blob,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    header(reader, blob).await?;
    let _charge = budget.reserve(
        "original blob completion window",
        CHUNK_BYTES * 64 * 4 + 65536,
    )?;
    let mut position = 0;
    let mut hasher = ContentHasher::default();
    let mut count = 0u64;
    loop {
        let mut vars = Variables::new();
        vars.insert("blob", blob.id());
        vars.insert("start", position);
        let values=rows(reader,"SELECT * FROM original_chunk WITH INDEX position WHERE blob=$blob AND start >= $start ORDER BY start LIMIT 64",vars).await?;
        if values.is_empty() {
            break;
        }
        let mut protected = BTreeSet::new();
        for value in values {
            let piece = decode_piece(value, blob)?;
            protected.insert(piece.id.clone());
            if piece.start != position {
                return Err(ModelError::Conflict("original blob continuity"));
            }
            position = position
                .checked_add(piece.bytes.len() as u64)
                .ok_or(ModelError::Schema("original blob bounds"))?;
            hasher.update(&piece.bytes);
            count += 1;
        }
        crate::installed_view::audit::audit_owned_keys(reader, &blob.id(), &protected, budget)
            .await?;
    }
    if position != blob.byte_len || count != blob.piece_count() || hasher.finish() != blob.content {
        return Err(ModelError::Conflict("original blob full readback"));
    }
    Ok(())
}
fn object_guard(id: &RecordId) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "native_guard",
        ContentHash::of(
            &serde_json::to_vec(&Value::RecordId(id.clone())).map_err(ModelError::codec)?,
        )
        .hex(),
    ))
}
/// The complete exact declaration is checked on every use. A retained native
/// blob owner and independently verified piece holds freeze completed bytes;
/// unrelated incidence/replay revisions do not invalidate that property.
pub(crate) async fn active_blob_premise<Context>(
    reader: &crate::NativeReader<Context>,
    blob: &Blob,
) -> Result<i64, ModelError> {
    if !header(reader, blob).await? {
        return Err(ModelError::Conflict("original observation blob incomplete"));
    }
    let guard = object_guard(&blob.id())?;
    let mut variables = Variables::new();
    variables.insert("guard", guard.clone());
    let values = rows(
        reader,
        "SELECT id,incarnation,revision,phase FROM $guard",
        variables,
    )
    .await?;
    let [Value::Object(row)] = values.as_slice() else {
        return Err(ModelError::Conflict("original observation guard missing"));
    };
    let incarnation = i64::from_value(
        row.get("incarnation")
            .cloned()
            .ok_or(ModelError::Schema("original observation incarnation"))?,
    )
    .map_err(ModelError::codec)?;
    let revision = i64::from_value(
        row.get("revision")
            .cloned()
            .ok_or(ModelError::Schema("original observation revision"))?,
    )
    .map_err(ModelError::codec)?;
    if row.get("id") != Some(&Value::RecordId(guard))
        || row.get("phase") != Some(&Value::String("active".into()))
        || incarnation <= 0
        || revision < 0
    {
        return Err(ModelError::Conflict("original observation inactive guard"));
    }
    Ok(incarnation)
}
/// Readback is tied to the same active blob incarnation/revision and immutable layout
/// hold. The final effect never reactivates a guard or replaces frozen-completion fields.
#[cfg(test)]
#[derive(Clone)]
struct CompletionAuditBarrier {
    observed: std::sync::Arc<tokio::sync::Barrier>,
    resume: std::sync::Arc<tokio::sync::Barrier>,
}
#[cfg(test)]
tokio::task_local! { static COMPLETION_AUDIT_BARRIER:CompletionAuditBarrier; }
pub async fn complete_blob<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: Option<ContentHash>,
    layout: &Layout,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let blob = &layout.blob;
    let guard = object_guard(&blob.id())?;
    let mut original_incarnation = None;
    let mut retry = crate::control::PremiseRetry::new();
    #[cfg(test)]
    let mut initial_audit = true;
    loop {
        retry.check()?;
        reader.ensure_scope_live()?;
        crate::prepared::check_read_cancellation(reader.read_cancellation().as_ref())?;
        validate_layout(reader, layout).await?;
        header(reader, blob).await?;
        let mut vars = Variables::new();
        vars.insert("guard", guard.clone());
        let captured = rows(
            reader,
            "SELECT id,incarnation,revision,phase FROM $guard",
            vars,
        )
        .await?;
        let [Value::Object(before)] = captured.as_slice() else {
            return Err(ModelError::Conflict("original completion guard missing"));
        };
        let incarnation = i64::from_value(
            before
                .get("incarnation")
                .cloned()
                .ok_or(ModelError::Schema("original guard incarnation"))?,
        )
        .map_err(ModelError::codec)?;
        let revision = i64::from_value(
            before
                .get("revision")
                .cloned()
                .ok_or(ModelError::Schema("original guard revision"))?,
        )
        .map_err(ModelError::codec)?;
        if incarnation <= 0 || revision < 0 {
            return Err(ModelError::Conflict("original completion guard identity"));
        }
        if before.get("id") != Some(&Value::RecordId(guard.clone()))
            || before.get("phase") != Some(&Value::String("active".into()))
        {
            return Err(ModelError::Conflict("original completion guard not active"));
        }
        if original_incarnation.is_some_and(|original| original != incarnation) {
            return Err(ModelError::Conflict(
                "original completion incarnation changed",
            ));
        }
        original_incarnation = Some(incarnation);
        validate_blob(reader, blob, budget).await?;
        #[cfg(test)]
        if initial_audit {
            initial_audit = false;
            if let Ok(barrier) = COMPLETION_AUDIT_BARRIER.try_with(Clone::clone) {
                barrier.observed.wait().await;
                barrier.resume.wait().await;
            }
        }
        retry.check()?;
        let mut vars = Variables::new();
        vars.insert("blob", blob.id());
        vars.insert("guard", guard.clone());
        vars.insert("incarnation", incarnation);
        vars.insert("revision", revision);
        vars.insert("row", blob.value(false));
        vars.insert("layout", layout.id.clone());
        vars.insert("layout_row", layout.value());
        let outcome=crate::control::original_completion_effect(reader.client(),attempt,
        "LET $frozen=SELECT * FROM ONLY $guard FOR UPDATE; IF $frozen=NONE OR $frozen.phase!='active' OR $frozen.incarnation!=$incarnation { THROW 'original blob audit incarnation changed'; }; LET $logical=SELECT * FROM ONLY $layout FOR UPDATE; IF $logical!=$layout_row OR array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$layout AND object=$blob LIMIT 1)!=1 { THROW 'original blob audit layout hold changed'; }; LET $prior=SELECT * FROM ONLY $blob FOR UPDATE; IF $prior.id!=$row.id OR $prior.content!=$row.content OR $prior.byte_len!=$row.byte_len OR $prior.format!=$row.format OR type::is_bool($prior.completed)=false OR array::len(object::keys($prior))!=5 { THROW 'original blob declaration changed'; }; IF $frozen.revision!=$revision { UPDATE $__effect SET original_completion_outcome='premise_changed' RETURN NONE; } ELSE { IF !$prior.completed { IF $frozen.revision>=9223372036854775807 { THROW 'original blob completion revision exhausted'; }; UPDATE $blob SET completed=true RETURN NONE; UPDATE $guard SET revision+=1 RETURN NONE; }; UPDATE $__effect SET original_completion_outcome='applied' RETURN NONE; };",vars).await?;
        match outcome {
            crate::control::OriginalCompletionOutcome::Applied => return Ok(()),
            crate::control::OriginalCompletionOutcome::PremiseChanged => {
                // The prior request is conclusively committed as a no-op. Re-enter
                // ordinary owned reads; never infer this condition from an error.
                retry.changed().await?;
            }
        }
    }
}
pub async fn complete_selected<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: Option<ContentHash>,
    sources: &[RecordId],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut completed = BTreeSet::new();
    let _charge = budget.reserve(
        "original selected completion",
        sources.len().saturating_mul(512),
    )?;
    for source in sources {
        let layout = layout(reader, source)
            .await?
            .ok_or(ModelError::Conflict("original layout missing"))?;
        validate_layout(reader, &layout).await?;
        if let Some(attempt) = attempt {
            drain_pending(reader, attempt, None, &layout, budget, true).await?;
        }
        if completed.insert(layout.blob.id()) {
            complete_blob(reader, attempt, &layout, budget).await?;
        }
    }
    Ok(())
}
/// Retained raw bytes and their caller-owned allocation authority travel together.
#[derive(Debug)]
pub struct OriginalBytes {
    bytes: Vec<u8>,
    _charge: std::sync::Arc<dyn lctx_model::domain::resources::Reservation>,
}
impl std::ops::Deref for OriginalBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}
impl AsRef<[u8]> for OriginalBytes {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
impl PartialEq<Vec<u8>> for OriginalBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.bytes == *other
    }
}
impl OriginalBytes {
    /// Reuse the caller's retained aggregate charge for a raw DTO/text buffer. This is a
    /// trusted consumer boundary: the size check cannot prove that the same reservation
    /// still covers every other transferred buffer or is held through their full lifetime.
    pub fn into_vec(
        self,
        retained: &dyn lctx_model::domain::resources::Reservation,
    ) -> Result<Vec<u8>, ModelError> {
        if retained.size() < self.bytes.capacity() {
            return Err(ModelError::Conflict(
                "original output aggregate charge reuse",
            ));
        }
        Ok(self.bytes)
    }
}
#[derive(Debug)]
pub struct OriginalRanges {
    values: Vec<OriginalBytes>,
}
impl std::ops::Deref for OriginalRanges {
    type Target = [OriginalBytes];
    fn deref(&self) -> &[OriginalBytes] {
        &self.values
    }
}
impl IntoIterator for OriginalRanges {
    type Item = OriginalBytes;
    type IntoIter = std::vec::IntoIter<OriginalBytes>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}
impl PartialEq<Vec<Vec<u8>>> for OriginalRanges {
    fn eq(&self, other: &Vec<Vec<u8>>) -> bool {
        self.values.len() == other.len()
            && self.values.iter().zip(other).all(|(a, b)| a.bytes == *b)
    }
}
impl OriginalRanges {
    pub(crate) fn empty() -> Self {
        Self { values: Vec::new() }
    }
    pub fn pop(&mut self) -> Option<OriginalBytes> {
        self.values.pop()
    }
}
/// A bounded, indexed physical union. Callers first authorize the logical source identities.
/// Duplicate ranges preserve request order; even empty ranges require a completed valid layout.
pub async fn read_ranges<Context>(
    reader: &crate::NativeReader<Context>,
    requests: &[(RecordId, u64, usize)],
    budget: &ResourceBudget,
) -> Result<OriginalRanges, ModelError> {
    let total = requests
        .iter()
        .try_fold(0usize, |n, (_, _, len)| n.checked_add(*len))
        .ok_or(ModelError::Schema("original union bound"))?;
    if requests.len() > 32 || total > 256 * 1024 {
        return Err(ModelError::Schema("original union bound"));
    }
    let _charge = budget.reserve(
        "original range physical union",
        total
            .saturating_mul(8)
            .saturating_add(requests.len().saturating_mul(CHUNK_BYTES * 4 + 4096)),
    )?;
    let mut layouts = BTreeMap::new();
    let mut blobs = BTreeMap::new();
    let mut wanted = BTreeMap::new();
    for (source, start, length) in requests {
        if !layouts.contains_key(source) {
            let value = layout(reader, source)
                .await?
                .ok_or(ModelError::Conflict("original layout missing"))?;
            layouts.insert(source.clone(), value);
        }
        let layout = &layouts[source];
        validate_layout(reader, layout).await?;
        if !blobs.contains_key(&layout.blob.id()) {
            if !header(reader, &layout.blob).await? {
                return Err(ModelError::Conflict("original blob incomplete"));
            }
            blobs.insert(layout.blob.id(), layout.blob.clone());
        }
        let end = start
            .checked_add(*length as u64)
            .filter(|end| *end <= layout.blob.byte_len)
            .ok_or(ModelError::Schema("original range bounds"))?;
        if *length > 0 {
            let mut at = start / CHUNK_BYTES as u64 * CHUNK_BYTES as u64;
            while at < end {
                wanted.insert(chunk_id(&layout.blob.id(), at)?, (layout.blob.id(), at));
                at += CHUNK_BYTES as u64;
            }
        }
    }
    let mut pieces = BTreeMap::new();
    if !wanted.is_empty() {
        let mut vars = Variables::new();
        vars.insert("chunks", wanted.keys().cloned().collect::<Vec<_>>());
        pieces = index_pieces(
            rows(reader, "SELECT * FROM $chunks", vars).await?,
            &wanted,
            &blobs,
        )?;
        if pieces.len() != wanted.len() {
            return Err(ModelError::Conflict("missing original piece"));
        }
    }
    assemble_outputs(requests, &layouts, &pieces, budget)
}
fn assemble_outputs(
    requests: &[(RecordId, u64, usize)],
    layouts: &BTreeMap<RecordId, Layout>,
    pieces: &BTreeMap<RecordId, Piece>,
    budget: &ResourceBudget,
) -> Result<OriginalRanges, ModelError> {
    let total = requests
        .iter()
        .try_fold(0usize, |n, (_, _, len)| n.checked_add(*len))
        .ok_or(ModelError::Schema("original output bound"))?;
    let retained: std::sync::Arc<dyn lctx_model::domain::resources::Reservation> = budget
        .reserve(
            "original retained range outputs",
            total.saturating_add(requests.len().saturating_mul(256)),
        )?
        .into();
    let values = requests
        .iter()
        .map(|(source, start, length)| {
            let layout = layouts
                .get(source)
                .ok_or(ModelError::Conflict("original output layout missing"))?;
            Ok(OriginalBytes {
                bytes: assemble_range(&layout.blob, pieces, *start, *length)?,
                _charge: retained.clone(),
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    Ok(OriginalRanges { values })
}
fn index_pieces(
    values: Vec<Value>,
    wanted: &BTreeMap<RecordId, (RecordId, u64)>,
    blobs: &BTreeMap<RecordId, Blob>,
) -> Result<BTreeMap<RecordId, Piece>, ModelError> {
    let mut pieces = BTreeMap::new();
    for value in values {
        let row = Piece::from_value(value.clone()).map_err(ModelError::codec)?;
        let Some((blob, start)) = wanted.get(&row.id) else {
            return Err(ModelError::Conflict("unrequested original piece"));
        };
        if &row.blob != blob || &row.start != start {
            return Err(ModelError::Conflict("original piece address"));
        }
        let row = decode_piece(value, &blobs[blob])?;
        if pieces.insert(row.id.clone(), row).is_some() {
            return Err(ModelError::Conflict("duplicate original piece"));
        }
    }
    Ok(pieces)
}
fn assemble_range(
    blob: &Blob,
    pieces: &BTreeMap<RecordId, Piece>,
    start: u64,
    length: usize,
) -> Result<Vec<u8>, ModelError> {
    let end = start
        .checked_add(length as u64)
        .filter(|end| *end <= blob.byte_len)
        .ok_or(ModelError::Schema("original range bounds"))?;
    let mut result = Vec::with_capacity(length);
    let mut at = start;
    while at < end {
        let offset = at / CHUNK_BYTES as u64 * CHUNK_BYTES as u64;
        let piece = pieces
            .get(&chunk_id(&blob.id(), offset)?)
            .ok_or(ModelError::Conflict("missing original piece"))?;
        let local = (at - offset) as usize;
        if local >= piece.bytes.len() {
            return Err(ModelError::Conflict("original piece continuity"));
        }
        let count = (end - at).min((piece.bytes.len() - local) as u64) as usize;
        result.extend_from_slice(&piece.bytes[local..local + count]);
        at += count as u64;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn install_completion_layout(
        client: &Surreal<Client>,
        attempt: ContentHash,
        layout: &Layout,
        budget: &ResourceBudget,
    ) {
        let owner = RecordId::new("native_attempt", attempt.hex());
        for window in crate::control::ImmutableWindows::new(
            vec![layout.value()],
            Some(attempt),
            None,
            Some(owner),
        )
        .with_budget(Some(budget))
        {
            crate::control::install_window(client, Some(attempt), window.unwrap(), Some(budget))
                .await
                .unwrap();
        }
    }
    async fn install_completion_pieces(
        client: &Surreal<Client>,
        attempt: ContentHash,
        blob: &Blob,
        body: &[u8],
        budget: &ResourceBudget,
    ) {
        let pieces = body
            .chunks(CHUNK_BYTES)
            .enumerate()
            .map(|(index, bytes)| blob.piece((index * CHUNK_BYTES) as u64, bytes).unwrap());
        for window in crate::control::ImmutableWindows::from_charged_iter(
            pieces.map(|row| Ok((row, None))),
            Some(attempt),
            None,
            None,
        )
        .with_budget(Some(budget))
        {
            crate::control::install_window(client, Some(attempt), window.unwrap(), Some(budget))
                .await
                .unwrap();
        }
    }
    /// Both finalizers observe the same revision, then exact layout/piece replay
    /// invalidates it. They must recapture, including when the peer completes first.
    #[tokio::test(flavor = "multi_thread")]
    async fn same_blob_completion_recaptures_equal_replay_and_peer_completion() {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        use lctx_model::domain::{admission::Frontier, completion, input::InputRevision};
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = crate::NativeReader::private(client.clone()).with_budget(&budget);
        let result=std::panic::AssertUnwindSafe(async{
   let input=InputRevision::from_entries(vec![]).unwrap().id();
   let mut body=vec![9;CHUNK_BYTES+7];body[..32].copy_from_slice(&store.attempt().0);
   for bytes in [body.as_slice(),b"".as_slice()]{
    let sources=[SourceArtifact::from_bytes(input,format!("left-{}.bin",bytes.len()),bytes).unwrap(),SourceArtifact::from_bytes(input,format!("right-{}.bin",bytes.len()),bytes).unwrap()];
    crate::Loader::for_attempt_views(client.clone(),store.attempt(),vec![]).with_budget(&budget).entities(&sources.iter().cloned().map(Entity::Source).collect::<Vec<_>>()).await.unwrap();
    let a=Layout::source(&sources[0]).unwrap();let b=Layout::source(&sources[1]).unwrap();assert_eq!(a.blob,b.blob);assert_ne!(a.artifact,b.artifact);
    install_completion_layout(&client,store.attempt(),&a,&budget).await;install_completion_layout(&client,store.attempt(),&b,&budget).await;install_completion_pieces(&client,store.attempt(),&a.blob,bytes,&budget).await;
    if !bytes.is_empty(){assert!(!header(&reader,&a.blob).await.unwrap());}
    let hook=CompletionAuditBarrier{observed:std::sync::Arc::new(tokio::sync::Barrier::new(3)),resume:std::sync::Arc::new(tokio::sync::Barrier::new(3))};
    let replay=async{hook.observed.wait().await;install_completion_layout(&client,store.attempt(),&a,&budget).await;install_completion_pieces(&client,store.attempt(),&a.blob,bytes,&budget).await;hook.resume.wait().await;};
    let (left,right,())=tokio::join!(COMPLETION_AUDIT_BARRIER.scope(hook.clone(),complete_blob(&reader,Some(store.attempt()),&a,&budget)),COMPLETION_AUDIT_BARRIER.scope(hook.clone(),complete_blob(&reader,Some(store.attempt()),&b,&budget)),replay);
    left.unwrap();right.unwrap();assert!(header(&reader,&a.blob).await.unwrap());validate_blob(&reader,&a.blob,&budget).await.unwrap();
   }
   let mut variables=Variables::new();variables.insert("attempt",RecordId::new("native_attempt",store.attempt().hex()));
   let outcomes:Vec<Value>=reader.query_native("SELECT VALUE original_completion_outcome FROM native_effect WHERE attempt=$attempt AND original_completion_outcome!=NONE",variables).await.unwrap();
   assert!(outcomes.iter().filter(|value|**value==Value::String("premise_changed".into())).count()>=4,"both audited revisions changed for nonempty and empty blobs: {outcomes:?}");
   assert_eq!(outcomes.iter().filter(|value|**value==Value::String("applied".into())).count(),4);
  }).catch_unwind().await;
        let mut cleanup = completion::Completion::default();
        cleanup.step("completion race reader close", reader.close().await);
        cleanup.step("completion race attempt cleanup", store.abandon().await);
        cleanup.step(
            "completion race observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("completion race cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn completion_changed_bytes_aba_retirement_and_cancel_refuse_without_adopting_a_new_premise()
     {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        use lctx_model::domain::{admission::Frontier, completion, input::InputRevision};
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = crate::NativeReader::private(client.clone()).with_budget(&budget);
        let result=std::panic::AssertUnwindSafe(async{
   let input=InputRevision::from_entries(vec![]).unwrap().id();
   for defect in ["piece","incarnation","retired","cancel"]{
    let bytes=[store.attempt().0.as_slice(),defect.as_bytes()].concat();let source=SourceArtifact::from_bytes(input,format!("{defect}.bin"),&bytes).unwrap();let layout=Layout::source(&source).unwrap();
    crate::Loader::for_attempt_views(client.clone(),store.attempt(),vec![]).with_budget(&budget).entities(&[Entity::Source(source)]).await.unwrap();
    install_completion_layout(&client,store.attempt(),&layout,&budget).await;install_completion_pieces(&client,store.attempt(),&layout.blob,&bytes,&budget).await;
    let guard=object_guard(&layout.blob.id()).unwrap();let mut vars=Variables::new();vars.insert("guard",guard.clone());
    let prior:Vec<Value>=reader.query_native("SELECT * FROM $guard",vars).await.unwrap();let [prior]=prior.as_slice()else{panic!("exact completion guard");};
    let hook=CompletionAuditBarrier{observed:std::sync::Arc::new(tokio::sync::Barrier::new(2)),resume:std::sync::Arc::new(tokio::sync::Barrier::new(2))};
    let cancellation=crate::control::RetryCancellation::default();
    let mutate=async{
     hook.observed.wait().await;
     if defect=="cancel"{cancellation.cancel();}else{
      let mut variables=Variables::new();variables.insert("guard",guard.clone());variables.insert("piece",chunk_id(&layout.blob.id(),0).unwrap());
      let changed=vec![0u8;bytes.len()];variables.insert("bytes",Bytes::from(changed.clone()));variables.insert("content",ContentHash::of(&changed).hex());
      let sql=match defect{"piece"=>"UPDATE $piece SET bytes=$bytes,content=$content RETURN NONE; UPDATE $guard SET revision+=1 RETURN NONE;","incarnation"=>"UPDATE $guard SET incarnation+=1,revision+=1 RETURN NONE;",_=>"UPDATE $guard SET phase='retiring',revision+=1 RETURN NONE;"};
      client.query(sql).bind(variables).await.unwrap().check().unwrap();
     }
     hook.resume.wait().await;
    };
    let (failed,())=tokio::join!(COMPLETION_AUDIT_BARRIER.scope(hook.clone(),crate::control::with_retry_cancellation(cancellation.clone(),complete_blob(&reader,Some(store.attempt()),&layout,&budget))),mutate);
    let failed=failed.expect_err("an invalidated audit cannot complete a different premise");
    match defect{"piece"=>assert!(format!("{failed:?}").contains("original blob full readback"),"{failed:?}"),"cancel"=>assert!(format!("{failed:?}").contains("recapture cancelled"),"{failed:?}"),_=>assert!(format!("{failed:?}").contains("incarnation changed"),"{failed:?}")};
    assert!(!header(&reader,&layout.blob).await.unwrap(),"no failed audit publishes completeness");
    // Exact fault restoration is restricted to this test's fresh owned objects.
    let mut variables=Variables::new();variables.insert("guard",guard.clone());variables.insert("guard_row",prior.clone());variables.insert("piece",chunk_id(&layout.blob.id(),0).unwrap());variables.insert("piece_row",layout.blob.piece(0,&bytes).unwrap());
    client.query("UPSERT $piece CONTENT $piece_row RETURN NONE; UPSERT $guard CONTENT $guard_row RETURN NONE;").bind(variables).await.unwrap().check().unwrap();
    complete_blob(&reader,Some(store.attempt()),&layout,&budget).await.unwrap();assert!(header(&reader,&layout.blob).await.unwrap(),"independent healthy owner remains eligible after canceled/refused waiter");
   }
  }).catch_unwind().await;
        let mut cleanup = completion::Completion::default();
        cleanup.step("completion refusal reader close", reader.close().await);
        cleanup.step("completion refusal attempt cleanup", store.abandon().await);
        cleanup.step(
            "completion refusal observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("completion refusal cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[test]
    fn blob_piece_envelopes_refuse_extra_fields_bounds_and_corruption() {
        let body = vec![7; CHUNK_BYTES + 3];
        let blob = Blob::new(ContentHash::of(&body), body.len() as u64).unwrap();
        assert_eq!(
            decode_blob(blob.value(false)).unwrap(),
            (blob.clone(), false)
        );
        let first = blob.piece(0, &body[..CHUNK_BYTES]).unwrap();
        decode_piece(first.clone(), &blob).unwrap();
        assert!(blob.piece(0, &body[..3]).is_err());
        assert!(blob.piece(1, &body[..3]).is_err());
        let mut corrupt = first.clone();
        if let Value::Object(row) = &mut corrupt {
            row.insert("content", ContentHash::of(b"wrong").hex());
        }
        assert!(decode_piece(corrupt, &blob).is_err());
        let mut extra = first;
        if let Value::Object(row) = &mut extra {
            row.insert("source", "old artifact body");
        }
        assert!(decode_piece(extra, &blob).is_err());
        assert!(Blob::new(ContentHash::of(b"wrong"), 0).is_err());
        let empty = Blob::new(ContentHash::of(b""), 0).unwrap();
        assert_eq!(empty.piece_count(), 0);
        assert!(empty.piece(0, b"").is_err());
    }
    #[test]
    fn equal_content_layouts_share_only_blob_and_preserve_exact_source_payload() {
        use lctx_model::domain::{Id, input::InputRevision};
        let input: Id<InputRevision> =
            serde_json::from_value(serde_json::to_value([1u8; 16]).unwrap()).unwrap();
        let one = SourceArtifact::from_bytes(input, "one.py".into(), b"shared").unwrap();
        let mut two = one.clone();
        two.path = "two.py".into();
        let one = Layout::source(&one).unwrap();
        let two = Layout::source(&two).unwrap();
        assert_eq!(one.blob, two.blob);
        assert_ne!(one.id, two.id);
        assert_ne!(one.artifact, two.artifact);
        assert!(decode_layout(one.value()).is_ok());
        assert_ne!(
            chunk_id(&one.blob.id(), 0).unwrap(),
            chunk_id(&one.blob.id(), CHUNK_BYTES as u64).unwrap()
        );
    }
    #[test]
    fn original_union_reuses_overlapping_blob_pieces_and_preserves_logical_order() {
        let mut body = vec![0; CHUNK_BYTES + 2];
        body[7..10].copy_from_slice(b"abc");
        body[CHUNK_BYTES - 2..].copy_from_slice(b"xyzw");
        let a = Blob::new(ContentHash::of(&body), body.len() as u64).unwrap();
        let b = Blob::new(ContentHash::of(b"BB"), 2).unwrap();
        let blobs = BTreeMap::from([(a.id(), a.clone()), (b.id(), b.clone())]);
        let wanted = BTreeMap::from([
            (chunk_id(&a.id(), 0).unwrap(), (a.id(), 0)),
            (
                chunk_id(&a.id(), CHUNK_BYTES as u64).unwrap(),
                (a.id(), CHUNK_BYTES as u64),
            ),
            (chunk_id(&b.id(), 0).unwrap(), (b.id(), 0)),
        ]);
        let values = vec![
            b.piece(0, b"BB").unwrap(),
            a.piece(CHUNK_BYTES as u64, b"zw").unwrap(),
            a.piece(0, &body[..CHUNK_BYTES]).unwrap(),
        ];
        let pieces = index_pieces(values.clone(), &wanted, &blobs).unwrap();
        assert_eq!(pieces.len(), 3);
        let actual = [
            (&a, 65534, 4),
            (&a, 7, 3),
            (&a, 65534, 4),
            (&b, 0, 2),
            (&a, 0, 0),
        ]
        .into_iter()
        .map(|(blob, start, len)| assemble_range(blob, &pieces, start, len).unwrap())
        .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                b"xyzw".to_vec(),
                b"abc".to_vec(),
                b"xyzw".to_vec(),
                b"BB".to_vec(),
                vec![]
            ]
        );
        let mut missing = index_pieces(values.clone(), &wanted, &blobs).unwrap();
        missing.remove(&chunk_id(&a.id(), 65536).unwrap());
        assert!(assemble_range(&a, &missing, 65535, 2).is_err());
        assert!(assemble_range(&a, &pieces, u64::MAX, 1).is_err());
        let mut duplicate = values.clone();
        duplicate.push(values[0].clone());
        assert!(index_pieces(duplicate, &wanted, &blobs).is_err());
        let mut corrupt = values.clone();
        if let Value::Object(row) = &mut corrupt[0] {
            row.insert("bytes", Bytes::from(b"CC".to_vec()));
        }
        assert!(index_pieces(corrupt, &wanted, &blobs).is_err());
        let mut unrequested = values;
        if let Value::Object(row) = &mut unrequested[0] {
            row.insert("id", RecordId::new("original_chunk", "unrequested"));
        }
        assert!(index_pieces(unrequested, &wanted, &blobs).is_err());
    }
    struct OutputFixture {
        layouts: BTreeMap<RecordId, Layout>,
        pieces: BTreeMap<RecordId, Piece>,
        requests: Vec<(RecordId, u64, usize)>,
    }
    fn output_fixture() -> OutputFixture {
        use lctx_model::domain::input::InputRevision;
        let mut body = vec![0; CHUNK_BYTES + 2];
        body[7..10].copy_from_slice(b"abc");
        body[CHUNK_BYTES - 2..].copy_from_slice(b"xyzw");
        let input = InputRevision::from_entries(vec![]).unwrap().id();
        let a = Layout::source(&SourceArtifact::from_bytes(input, "a.bin".into(), &body).unwrap())
            .unwrap();
        let b = Layout::source(&SourceArtifact::from_bytes(input, "b.bin".into(), &body).unwrap())
            .unwrap();
        let pieces = [
            a.blob.piece(0, &body[..CHUNK_BYTES]).unwrap(),
            a.blob
                .piece(CHUNK_BYTES as u64, &body[CHUNK_BYTES..])
                .unwrap(),
        ]
        .into_iter()
        .map(|value| {
            let piece = decode_piece(value, &a.blob).unwrap();
            (piece.id.clone(), piece)
        })
        .collect();
        OutputFixture {
            requests: vec![
                (a.id.clone(), 65534, 4),
                (b.id.clone(), 65534, 4),
                (a.id.clone(), 7, 3),
                (a.id.clone(), 0, 0),
            ],
            layouts: BTreeMap::from([(a.id.clone(), a), (b.id.clone(), b)]),
            pieces,
        }
    }
    #[test]
    fn duplicate_ranges_own_separate_buffers_and_keep_charge_after_iterator_drop() {
        let f = output_fixture();
        let budget = ResourceBudget::fixed(8192).unwrap();
        let mut ranges = assemble_outputs(&f.requests, &f.layouts, &f.pieces, &budget).unwrap();
        assert_eq!(
            ranges,
            vec![b"xyzw".to_vec(), b"xyzw".to_vec(), b"abc".to_vec(), vec![]],
            "shared blob hydration preserves logical multiplicity, order and empty ranges"
        );
        let retained = budget.reserved();
        assert_eq!(
            retained,
            11 + 4 * 256,
            "all duplicate output bytes are charged before construction"
        );
        ranges.values[0].bytes[0] = b'X';
        assert_eq!(
            ranges[1].as_ref(),
            b"xyzw",
            "duplicate logical requests never alias a mutable output buffer"
        );
        let mut iterator = ranges.into_iter();
        let first = iterator.next().unwrap();
        drop(iterator);
        assert_eq!(
            budget.reserved(),
            retained,
            "yielded bytes keep the output authority after range and iterator owners disappear"
        );
        drop(first);
        assert_eq!(
            budget.reserved(),
            0,
            "the last retained output releases its charge"
        );
    }
    #[test]
    fn output_refusal_and_late_missing_piece_release_partial_allocations() {
        let mut f = output_fixture();
        let required = 11 + 4 * 256;
        let small = ResourceBudget::fixed(required - 1).unwrap();
        assert!(assemble_outputs(&f.requests, &f.layouts, &f.pieces, &small).is_err());
        assert_eq!(
            small.reserved(),
            0,
            "allocation refusal publishes no output charge"
        );
        let budget = ResourceBudget::fixed(8192).unwrap();
        let source = f.requests[0].0.clone();
        let tail = chunk_id(&f.layouts[&source].blob.id(), CHUNK_BYTES as u64).unwrap();
        f.pieces.remove(&tail);
        let requests = vec![(source.clone(), 7, 3), (source, 65534, 4)];
        assert!(
            assemble_outputs(&requests, &f.layouts, &f.pieces, &budget).is_err(),
            "a complete first output cannot hide a missing suffix in a later request"
        );
        assert_eq!(
            budget.reserved(),
            0,
            "late semantic refusal releases both partial buffers and their aggregate owner"
        );
    }
    #[test]
    fn raw_output_consumer_explicitly_retains_its_aggregate_charge() {
        let f = output_fixture();
        let budget = ResourceBudget::fixed(8192).unwrap();
        let caller = budget.reserve("consumer aggregate originals", 11).unwrap();
        let ranges = assemble_outputs(&f.requests, &f.layouts, &f.pieces, &budget).unwrap();
        let raw = ranges
            .into_iter()
            .map(|bytes| bytes.into_vec(caller.as_ref()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            budget.reserved(),
            11,
            "native owner is released only at the explicit trusted consumer boundary"
        );
        assert_eq!(raw[0], raw[1]);
        assert_eq!(raw.iter().map(Vec::len).sum::<usize>(), 11);
        drop(raw);
        assert_eq!(
            budget.reserved(),
            11,
            "the actual consumer keeps authority through its raw buffer lifetime"
        );
        drop(caller);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn all_blob_envelopes_refuse_changed_id_extra_and_missing_fields() {
        let blob = Blob::new(ContentHash::of(&[0xff, 0, 0x80]), 3).unwrap();
        for defect in ["id", "extra", "missing_content"] {
            let mut value = blob.piece(0, &[0xff, 0, 0x80]).unwrap();
            let Value::Object(row) = &mut value else {
                unreachable!()
            };
            match defect {
                "id" => {
                    row.insert("id", RecordId::new("original_chunk", "wrong"));
                }
                "extra" => {
                    row.insert("extra", Value::Null);
                }
                _ => {
                    row.remove("content");
                }
            }
            assert!(
                decode_piece(value, &blob).is_err(),
                "exact piece envelope: {defect}"
            );
        }
        let mut header = blob.value(true);
        if let Value::Object(row) = &mut header {
            row.insert("extra", Value::Null);
        }
        assert!(decode_blob(header).is_err());
    }
}

/// Attempt-owned fallback for arbitrary typed relation ordering. These rows are temporary
/// native work, excluded from logical completed-original closure and detached export.
#[derive(Default)]
pub struct PendingOriginals {
    gate: tokio::sync::Mutex<bool>,
    budget: std::sync::Mutex<Option<ResourceBudget>>,
}
fn pending_id(attempt: ContentHash, source: &RecordId) -> Result<RecordId, ModelError> {
    let mut key = KeySink::new("pending-original/v1");
    key.part(b"attempt", &attempt.0);
    key.part(
        b"source",
        &serde_json::to_vec(source).map_err(ModelError::codec)?,
    );
    Ok(RecordId::new("native_pending_original", key.finish().hex()))
}
fn ordinal_id(root: &RecordId, ordinal: i64) -> Result<RecordId, ModelError> {
    if ordinal < 0 {
        return Err(ModelError::Schema("pending original ordinal"));
    }
    let mut key = KeySink::new("pending-original-ordinal/v1");
    key.part(
        b"root",
        &serde_json::to_vec(root).map_err(ModelError::codec)?,
    );
    key.part(b"ordinal", &ordinal.to_le_bytes());
    Ok(RecordId::new(
        "native_pending_original_ordinal",
        key.finish().hex(),
    ))
}
fn pending_piece_id(ordinal: &RecordId, start: usize) -> Result<RecordId, ModelError> {
    let mut key = KeySink::new("pending-original-piece/v1");
    key.part(
        b"ordinal",
        &serde_json::to_vec(ordinal).map_err(ModelError::codec)?,
    );
    key.part(b"start", &(start as u64).to_le_bytes());
    Ok(RecordId::new(
        "native_pending_original_piece",
        key.finish().hex(),
    ))
}
fn pending_root(attempt: ContentHash, source: &RecordId) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", pending_id(attempt, source)?);
    row.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    row.insert("logical_source", source.clone());
    Ok(Value::Object(row))
}
fn pending_ordinal(
    root: &RecordId,
    ordinal: i64,
    length: usize,
    content: ContentHash,
) -> Result<Value, ModelError> {
    if length == 0 || length > lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES {
        return Err(ModelError::Schema("pending original chunk length"));
    }
    let mut row = Object::new();
    row.insert("id", ordinal_id(root, ordinal)?);
    row.insert("root", root.clone());
    row.insert("ordinal", ordinal);
    row.insert("byte_len", length as i64);
    row.insert("content", content.hex());
    Ok(Value::Object(row))
}
fn pending_piece(ordinal: &RecordId, start: usize, bytes: &[u8]) -> Result<Value, ModelError> {
    if !start.is_multiple_of(CHUNK_BYTES) || bytes.is_empty() || bytes.len() > CHUNK_BYTES {
        return Err(ModelError::Schema("pending original piece bounds"));
    }
    let mut row = Object::new();
    row.insert("id", pending_piece_id(ordinal, start)?);
    row.insert("ordinal", ordinal.clone());
    row.insert("start", start as i64);
    row.insert("content", ContentHash::of(bytes).hex());
    row.insert("bytes", Bytes::from(bytes.to_vec()));
    Ok(Value::Object(row))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct PendingRoot {
    id: RecordId,
    attempt: RecordId,
    logical_source: RecordId,
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct PendingOrdinal {
    id: RecordId,
    root: RecordId,
    ordinal: i64,
    byte_len: usize,
    content: String,
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct PendingPiece {
    id: RecordId,
    ordinal: RecordId,
    start: usize,
    content: String,
    bytes: Bytes,
}
fn decode_pending_ordinal(value: Value, root: &RecordId) -> Result<PendingOrdinal, ModelError> {
    let row = PendingOrdinal::from_value(value.clone()).map_err(ModelError::codec)?;
    exact(
        &value,
        &pending_ordinal(root, row.ordinal, row.byte_len, hash(&row.content)?)?,
        "pending ordinal full envelope",
    )?;
    if row.root != *root {
        return Err(ModelError::Conflict("pending ordinal parent"));
    }
    Ok(row)
}
/// One bounded typed ordinal's independent readback. Physical envelopes and exact order
/// are checked before exposing each piece; complete length/hash is checked at actual EOF.
struct PendingReplay {
    ordinal: RecordId,
    length: usize,
    digest: ContentHash,
    position: usize,
    hash: ContentHasher,
}
impl PendingReplay {
    fn new(ordinal: &PendingOrdinal) -> Result<Self, ModelError> {
        Ok(Self {
            ordinal: ordinal.id.clone(),
            length: ordinal.byte_len,
            digest: hash(&ordinal.content)?,
            position: 0,
            hash: ContentHasher::default(),
        })
    }
    fn push(&mut self, value: Value) -> Result<PendingPiece, ModelError> {
        let piece = PendingPiece::from_value(value.clone()).map_err(ModelError::codec)?;
        exact(
            &value,
            &pending_piece(&self.ordinal, piece.start, &piece.bytes)?,
            "pending piece full envelope",
        )?;
        let remaining = self
            .length
            .checked_sub(self.position)
            .ok_or(ModelError::Schema("pending replay bounds"))?;
        if piece.ordinal != self.ordinal
            || piece.start != self.position
            || piece.bytes.len() != remaining.min(CHUNK_BYTES)
        {
            return Err(ModelError::Conflict("pending original piece continuity"));
        }
        self.hash.update(&piece.bytes);
        self.position = self
            .position
            .checked_add(piece.bytes.len())
            .ok_or(ModelError::Schema("pending piece offset"))?;
        Ok(piece)
    }
    fn finish(self) -> Result<(), ModelError> {
        if self.position != self.length || self.hash.finish() != self.digest {
            return Err(ModelError::Conflict("pending original full readback"));
        }
        Ok(())
    }
}
pub(crate) fn pending_root_identity(
    attempt: ContentHash,
    value: &Value,
) -> Result<RecordId, ModelError> {
    let row = PendingRoot::from_value(value.clone()).map_err(ModelError::codec)?;
    if row.logical_source.table.as_str() != "original"
        || row.attempt != RecordId::new("native_attempt", attempt.hex())
    {
        return Err(ModelError::Conflict("pending original root owner"));
    }
    exact(
        value,
        &pending_root(attempt, &row.logical_source)?,
        "pending root full envelope",
    )?;
    Ok(row.id)
}
async fn install_owned(
    client: &Surreal<Client>,
    attempt: ContentHash,
    contribution: Option<ContentHash>,
    owner: RecordId,
    row: Value,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for window in
        crate::control::ImmutableWindows::new(vec![row], Some(attempt), contribution, Some(owner))
            .with_budget(Some(budget))
    {
        crate::control::install_window(client, Some(attempt), window?, Some(budget)).await?;
    }
    Ok(())
}
async fn stage<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    contribution: Option<ContentHash>,
    source: &RecordId,
    chunk: &lctx_model::domain::artifact::ArtifactChunk,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let root = pending_id(attempt, source)?;
    install_owned(
        reader.client(),
        attempt,
        contribution,
        RecordId::new("native_attempt", attempt.hex()),
        pending_root(attempt, source)?,
        budget,
    )
    .await?;
    let ordinal = ordinal_id(&root, chunk.ordinal)?;
    install_owned(
        reader.client(),
        attempt,
        contribution,
        root.clone(),
        pending_ordinal(
            &root,
            chunk.ordinal,
            chunk.body.0.len(),
            ContentHash::of(&chunk.body.0),
        )?,
        budget,
    )
    .await?;
    // An ordinal can be partially installed after a failed producer; exact replay fills only
    // matching pieces, and drainage refuses incomplete or extra physical inventory.
    let pieces = pending_piece_rows(&ordinal, &chunk.body.0, budget);
    for window in crate::control::ImmutableWindows::from_charged_iter(
        pieces,
        Some(attempt),
        contribution,
        Some(ordinal.clone()),
    )
    .with_budget(Some(budget))
    {
        crate::control::install_window(reader.client(), Some(attempt), window?, Some(budget))
            .await?;
    }
    Ok(())
}
type ChargedPiece = Result<
    (
        Value,
        Option<Box<dyn lctx_model::domain::resources::Reservation>>,
    ),
    ModelError,
>;
fn pending_piece_rows<'a>(
    ordinal: &'a RecordId,
    body: &'a [u8],
    budget: &'a ResourceBudget,
) -> impl Iterator<Item = ChargedPiece> + Send + 'a {
    body.chunks(CHUNK_BYTES)
        .enumerate()
        .map(move |(index, bytes)| {
            let charge = budget.reserve(
                "pending original native piece",
                bytes.len().saturating_mul(2) + 4096,
            )?;
            Ok((
                pending_piece(ordinal, index * CHUNK_BYTES, bytes)?,
                Some(charge),
            ))
        })
}
fn charged_blob_piece(
    blob: &Blob,
    start: u64,
    bytes: &[u8],
    budget: &ResourceBudget,
) -> ChargedPiece {
    let charge = budget.reserve(
        "native original blob piece",
        bytes.len().saturating_mul(2) + 4096,
    )?;
    Ok((blob.piece(start, bytes)?, Some(charge)))
}
/// Flatten only this already charged ingress slice, borrowing bodies rather than copying
/// a corpus. The immutable window retains each lazily acquired piece charge through ACK.
fn blob_piece_rows<'a>(
    segments: impl Iterator<Item = (&'a Blob, u64, &'a [u8])> + Send + 'a,
    budget: &'a ResourceBudget,
) -> impl Iterator<Item = ChargedPiece> + Send + 'a {
    segments.flat_map(move |(blob, start, bytes)| {
        bytes
            .chunks(CHUNK_BYTES)
            .enumerate()
            .map(move |(index, bytes)| {
                let at = start
                    .checked_add((index * CHUNK_BYTES) as u64)
                    .ok_or(ModelError::Schema("original piece offset"))?;
                charged_blob_piece(blob, at, bytes, budget)
            })
    })
}
fn replay_page_rows<'a>(
    values: impl Iterator<Item = Value> + Send + 'a,
    replay: &'a mut PendingReplay,
    blob: &'a Blob,
    base: u64,
    budget: &'a ResourceBudget,
) -> impl Iterator<Item = ChargedPiece> + Send + 'a {
    values.map(move |value| {
        let piece = replay.push(value)?;
        let start = base
            .checked_add(piece.start as u64)
            .ok_or(ModelError::Schema("pending original piece offset"))?;
        charged_blob_piece(blob, start, &piece.bytes, budget)
    })
}
async fn drain_pending<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    contribution: Option<ContentHash>,
    layout: &Layout,
    budget: &ResourceBudget,
    replay: bool,
) -> Result<(), ModelError> {
    let root = pending_id(attempt, &layout.id)?;
    let mut vars = Variables::new();
    vars.insert("root", root.clone());
    vars.insert("after", Value::None);
    let _page = budget.reserve("pending original replay page", 32 * 4096 + CHUNK_BYTES * 8)?;
    loop {
        let values=rows(reader,"SELECT * FROM native_pending_original_ordinal WITH INDEX pending_ordinals WHERE root=$root AND ($after=NONE OR id>$after) ORDER BY id LIMIT 32",vars.clone()).await?;
        if values.is_empty() {
            break;
        }
        for value in values {
            let ordinal = decode_pending_ordinal(value, &root)?;
            let base = u64::try_from(ordinal.ordinal)
                .map_err(ModelError::codec)?
                .checked_mul(lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64)
                .ok_or(ModelError::Schema("pending ordinal offset"))?;
            let expected = (layout
                .blob
                .byte_len
                .checked_sub(base)
                .ok_or(ModelError::Schema("pending ordinal beyond source"))?)
            .min(lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64);
            if expected == 0 || ordinal.byte_len as u64 != expected {
                return Err(ModelError::Conflict("pending original typed chunk length"));
            }
            let mut pieces = Variables::new();
            pieces.insert("ordinal", ordinal.id.clone());
            pieces.insert("start", 0i64);
            let mut replay_state = PendingReplay::new(&ordinal)?;
            loop {
                let values=rows(reader,"SELECT * FROM native_pending_original_piece WITH INDEX pending_pieces WHERE ordinal=$ordinal AND start >= $start ORDER BY start LIMIT 4",pieces.clone()).await?;
                if values.is_empty() {
                    break;
                }
                if replay {
                    // query_native has drained the bounded page before any mutation. All
                    // its validated pieces feed one window iterator, not one effect each.
                    let page = replay_page_rows(
                        values.into_iter(),
                        &mut replay_state,
                        &layout.blob,
                        base,
                        budget,
                    );
                    for window in crate::control::ImmutableWindows::from_charged_iter(
                        page,
                        Some(attempt),
                        contribution,
                        None,
                    )
                    .with_budget(Some(budget))
                    {
                        crate::control::install_window(
                            reader.client(),
                            Some(attempt),
                            window?,
                            Some(budget),
                        )
                        .await?;
                    }
                } else {
                    for value in values {
                        let piece = replay_state.push(value)?;
                        let start = base
                            .checked_add(piece.start as u64)
                            .ok_or(ModelError::Schema("pending original piece offset"))?;
                        let mut vars = Variables::new();
                        vars.insert("id", chunk_id(&layout.blob.id(), start)?);
                        let actual = rows(reader, "SELECT * FROM $id", vars).await?;
                        let [actual] = actual.as_slice() else {
                            return Err(ModelError::Conflict(
                                "pending original acknowledged blob piece missing",
                            ));
                        };
                        exact(
                            actual,
                            &layout.blob.piece(start, &piece.bytes)?,
                            "pending original acknowledged full bytes",
                        )?;
                    }
                }
                pieces.insert("start", replay_state.position);
            }
            replay_state.finish()?;
            vars.insert("after", ordinal.id);
        }
    }
    // Keep the native attempt hold until terminal cleanup. It authenticates replay after a
    // partial drain and protects a concurrent late raw writer; terminal cleanup nominates
    // the complete known root set before releasing any attempt ownership.
    Ok(())
}
impl PendingOriginals {
    pub fn has_ingress(&self) -> Result<bool, ModelError> {
        self.budget
            .lock()
            .map(|budget| budget.is_some())
            .map_err(|_| ModelError::Conflict("original ingress budget owner"))
    }
    fn retain_budget(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        let mut retained = self
            .budget
            .lock()
            .map_err(|_| ModelError::Conflict("original ingress budget owner"))?;
        if retained.is_none() {
            *retained = Some(budget.clone());
        }
        Ok(())
    }
    /// Finalization owns no session capability. It closes only the helper's private reader
    /// and uses the allocation authority explicitly supplied by an earlier ingress call.
    pub async fn close_owned(
        &self,
        client: std::sync::Arc<Surreal<Client>>,
        attempt: ContentHash,
        cancellation: Result<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>, ModelError>,
        validate_completed: bool,
    ) -> lctx_model::domain::completion::Completion {
        // Native finality does not require a failed producer to invent missing bytes.
        // Always close ingress; only a successfully frozen owner earns semantic readback.
        *self.gate.lock().await = true;
        let mut completion = lctx_model::domain::completion::Completion::default();
        if !validate_completed {
            return completion;
        }
        let budget = self
            .budget
            .lock()
            .map(|budget| budget.clone())
            .map_err(|_| ModelError::Conflict("original ingress budget owner"));
        let result = async {
            let budget = budget?;
            let Some(budget) = budget else {
                *self.gate.lock().await = true;
                return Ok(());
            };
            let cancellation = cancellation?;
            let reader = crate::NativeReader::private(client).with_budget(&budget);
            let reader = if let Some(flag) = cancellation {
                reader.with_read_cancellation(flag)
            } else {
                reader
            };
            let pending = self.close(&reader, attempt, &budget).await;
            let mut reader_completion = lctx_model::domain::completion::Completion::default();
            reader_completion.step("original final readback reader close", reader.close().await);
            lctx_model::domain::completion::complete(
                lctx_model::domain::completion::complete(Ok(()), pending),
                reader_completion,
            )
        }
        .await;
        completion.step("pending original final readback owner", result);
        completion
    }
    pub async fn chunks<Context>(
        &self,
        reader: &crate::NativeReader<Context>,
        attempt: ContentHash,
        contribution: Option<ContentHash>,
        chunks: &[lctx_model::domain::artifact::ArtifactChunk],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let closed = self.gate.lock().await;
        if *closed {
            return Err(ModelError::Conflict("original ingress closed"));
        }
        if !chunks.is_empty() {
            self.retain_budget(budget)?;
        }
        // Bodies remain borrowed from the caller's charged slice. Only fixed-size blob
        // declarations and offsets are retained while layout/root setup runs separately.
        let metadata_bytes = chunks
            .len()
            .checked_mul(std::mem::size_of::<(Blob, u64, &[u8])>())
            .ok_or(ModelError::Schema("original ingress slice metadata"))?;
        let _metadata = budget.reserve("original ingress slice metadata", metadata_bytes)?;
        let mut segments = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            chunk.validate()?;
            let source = RecordId::new("original", EntityId::of(chunk.artifact).0.hex());
            match layout(reader, &source).await? {
                Some(layout) => {
                    validate_layout(reader, &layout).await?;
                    let owner = contribution
                        .map(|id| RecordId::new("compiler_contribution", id.hex()))
                        .unwrap_or_else(|| RecordId::new("native_attempt", attempt.hex()));
                    install_owned(
                        reader.client(),
                        attempt,
                        contribution,
                        owner,
                        layout.value(),
                        budget,
                    )
                    .await?;
                    let start = u64::try_from(chunk.ordinal)
                        .map_err(ModelError::codec)?
                        .checked_mul(lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64)
                        .ok_or(ModelError::Schema("original typed chunk offset"))?;
                    let expected = layout
                        .blob
                        .byte_len
                        .checked_sub(start)
                        .ok_or(ModelError::Schema("original typed chunk bounds"))?
                        .min(lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64);
                    if chunk.body.0.len() as u64 != expected {
                        return Err(ModelError::Conflict("original typed chunk length"));
                    }
                    segments.push((layout.blob, start, chunk.body.0.as_slice()));
                }
                None => stage(reader, attempt, contribution, &source, chunk, budget).await?,
            }
        }
        let pieces = blob_piece_rows(
            segments
                .iter()
                .map(|(blob, start, bytes)| (blob, *start, *bytes)),
            budget,
        );
        for window in crate::control::ImmutableWindows::from_charged_iter(
            pieces,
            Some(attempt),
            contribution,
            None,
        )
        .with_budget(Some(budget))
        {
            crate::control::install_window(reader.client(), Some(attempt), window?, Some(budget))
                .await?;
        }
        Ok(())
    }
    pub async fn sources<Context>(
        &self,
        reader: &crate::NativeReader<Context>,
        attempt: ContentHash,
        contribution: Option<ContentHash>,
        sources: &[SourceArtifact],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let closed = self.gate.lock().await;
        if *closed {
            return Err(ModelError::Conflict("original ingress closed"));
        }
        if !sources.is_empty() {
            self.retain_budget(budget)?;
        }
        for source in sources {
            let layout = Layout::source(source)?;
            validate_layout(reader, &layout).await?;
            let owner = contribution
                .map(|id| RecordId::new("compiler_contribution", id.hex()))
                .unwrap_or_else(|| RecordId::new("native_attempt", attempt.hex()));
            install_owned(
                reader.client(),
                attempt,
                contribution,
                owner,
                layout.value(),
                budget,
            )
            .await?;
            drain_pending(reader, attempt, contribution, &layout, budget, true).await?;
        }
        Ok(())
    }
    /// After all mutation owners have drained, independently compare retained staging bytes
    /// with completed blob pieces. Close is read-only even after attempt admission; missing
    /// headers or incomplete blobs refuse finalization without deleting replay state.
    pub async fn close<Context>(
        &self,
        reader: &crate::NativeReader<Context>,
        attempt: ContentHash,
        budget: &ResourceBudget,
    ) -> lctx_model::domain::completion::Completion {
        *self.gate.lock().await = true;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "pending original final readback",
            validate_pending(reader, attempt, budget).await,
        );
        completion
    }
}
pub async fn complete_pending<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    process_pending(reader, attempt, budget, true).await?;
    process_owned_layouts(reader, attempt, budget, true).await
}
/// Repeated freeze/final drain earns no new write authority after an acknowledged admission.
pub async fn validate_pending<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    process_pending(reader, attempt, budget, false).await?;
    process_owned_layouts(reader, attempt, budget, false).await
}
/// Raw typed ingress owns layouts directly through the attempt, even when the source header
/// arrived first and no pending rows were needed. Provider contributions complete their own
/// exact memberships separately. Enumerate only this attempt's retained holds in index order.
async fn process_owned_layouts<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    budget: &ResourceBudget,
    replay: bool,
) -> Result<(), ModelError> {
    let mut vars = Variables::new();
    vars.insert("owner", RecordId::new("native_attempt", attempt.hex()));
    vars.insert("after", Value::None);
    let _page = budget.reserve("attempt original layout page", 32 * 4096)?;
    let mut after = None::<RecordId>;
    loop {
        let objects:Vec<RecordId>=reader.query_native("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND ($after=NONE OR object>$after) ORDER BY object LIMIT 32",vars.clone()).await?;
        if objects.is_empty() {
            return Ok(());
        }
        for source in objects {
            if after.as_ref().is_some_and(|previous| &source <= previous) {
                return Err(ModelError::Conflict("attempt original hold order"));
            }
            if source.table.as_str() == "original" {
                let layout = layout(reader, &source)
                    .await?
                    .ok_or(ModelError::Conflict("attempt original layout missing"))?;
                validate_layout(reader, &layout).await?;
                if replay {
                    complete_blob(reader, Some(attempt), &layout, budget).await?;
                } else if !header(reader, &layout.blob).await? {
                    return Err(ModelError::Conflict("attempt original blob incomplete"));
                }
            }
            after = Some(source);
        }
        vars.insert(
            "after",
            after.clone().map(Value::RecordId).unwrap_or(Value::None),
        );
    }
}
async fn process_pending<Context>(
    reader: &crate::NativeReader<Context>,
    attempt: ContentHash,
    budget: &ResourceBudget,
    replay: bool,
) -> Result<(), ModelError> {
    let mut vars = Variables::new();
    vars.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    vars.insert("after", Value::None);
    let _charge = budget.reserve("pending original root page", 32 * 4096)?;
    loop {
        let values=rows(reader,"SELECT * FROM native_pending_original WITH INDEX pending_attempt WHERE attempt=$attempt AND ($after=NONE OR id>$after) ORDER BY id LIMIT 32",vars.clone()).await?;
        if values.is_empty() {
            return Ok(());
        }
        for value in values {
            let root = PendingRoot::from_value(value.clone()).map_err(ModelError::codec)?;
            exact(
                &value,
                &pending_root(attempt, &root.logical_source)?,
                "pending root full envelope",
            )?;
            if root.attempt != RecordId::new("native_attempt", attempt.hex()) {
                return Err(ModelError::Conflict("pending original attempt"));
            }
            let layout =
                layout(reader, &root.logical_source)
                    .await?
                    .ok_or(ModelError::Conflict(
                        "original ingress has unresolved source layouts",
                    ))?;
            validate_layout(reader, &layout).await?;
            if !replay && !header(reader, &layout.blob).await? {
                return Err(ModelError::Conflict("pending original blob not completed"));
            }
            drain_pending(reader, attempt, None, &layout, budget, replay).await?;
            vars.insert("after", root.id);
        }
    }
}
#[cfg(test)]
mod pending_controls {
    use super::*;
    #[test]
    fn charged_original_slice_preserves_typed_chunk_order_and_exact_blob_owners() {
        let body = vec![0xff; lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES + 3];
        let other = b"other";
        let a = Blob::new(ContentHash::of(&body), body.len() as u64).unwrap();
        let b = Blob::new(ContentHash::of(other), other.len() as u64).unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let caller = budget
            .reserve("original input slice", body.len() + other.len())
            .unwrap();
        let retained = budget.reserved();
        let boundary = lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES;
        let segments = [
            (&a, 0, &body[..boundary]),
            (&a, boundary as u64, &body[boundary..]),
            (&b, 0, other.as_slice()),
        ];
        let mut pieces = blob_piece_rows(segments.into_iter(), &budget);
        for index in 0..a.piece_count() {
            let (value, charge) = pieces.next().unwrap().unwrap();
            assert!(budget.reserved() > retained);
            let piece = decode_piece(value, &a).unwrap();
            assert_eq!(piece.start, index * CHUNK_BYTES as u64);
            assert_eq!(piece.id, chunk_id(&a.id(), piece.start).unwrap());
            assert_eq!(
                piece.bytes.as_ref(),
                &body[piece.start as usize..piece.start as usize + piece.bytes.len()]
            );
            drop(piece);
            drop(charge);
            assert_eq!(budget.reserved(), retained);
        }
        let (value, charge) = pieces.next().unwrap().unwrap();
        let piece = decode_piece(value, &b).unwrap();
        assert_eq!(piece.start, 0);
        assert_eq!(piece.bytes.as_ref(), other);
        drop(piece);
        drop(charge);
        assert!(pieces.next().is_none());
        assert_eq!(budget.reserved(), retained);
        drop(caller);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn original_windows_combine_blob_owners_and_all_pieces_of_one_pending_ordinal() {
        let attempt = ContentHash([11; 32]);
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let a = Blob::new(ContentHash::of(b"one"), 3).unwrap();
        let b = Blob::new(ContentHash::of(b"two"), 3).unwrap();
        let segments = [(&a, 0, b"one".as_slice()), (&b, 0, b"two".as_slice())];
        let windows = crate::control::ImmutableWindows::from_charged_iter(
            blob_piece_rows(segments.into_iter(), &budget),
            Some(attempt),
            None,
            None,
        )
        .with_budget(Some(&budget))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(
            windows.len(),
            1,
            "two exact blob owners share a useful immutable window, with row-derived holds"
        );
        assert!(
            budget.reserved() > 0,
            "expanded rows and input copies survive until acknowledgement owner drops"
        );
        drop(windows);
        assert_eq!(budget.reserved(), 0);
        let root = pending_id(attempt, &RecordId::new("original", "source")).unwrap();
        let ordinal = ordinal_id(&root, 0).unwrap();
        let body = vec![7; CHUNK_BYTES + 3];
        let windows = crate::control::ImmutableWindows::from_charged_iter(
            pending_piece_rows(&ordinal, &body, &budget),
            Some(attempt),
            None,
            Some(ordinal.clone()),
        )
        .with_budget(Some(&budget))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(
            windows.len(),
            1,
            "one ordinal owns all its pieces in the same window stream"
        );
        drop(windows);
        assert_eq!(budget.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        let mut refused = crate::control::ImmutableWindows::from_charged_iter(
            pending_piece_rows(&ordinal, &body, &tiny),
            Some(attempt),
            None,
            Some(ordinal.clone()),
        )
        .with_budget(Some(&tiny));
        assert!(refused.next().unwrap().is_err());
        assert!(refused.next().is_none());
        drop(refused);
        assert_eq!(
            tiny.reserved(),
            0,
            "budget refusal leaves no partially retained window"
        );
    }
    #[test]
    fn drained_pending_page_combines_pieces_and_refuses_late_corruption_before_window_exposure() {
        let attempt = ContentHash([12; 32]);
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let root = pending_id(attempt, &RecordId::new("original", "source")).unwrap();
        let body = vec![0xff; CHUNK_BYTES + 3];
        let ordinal = decode_pending_ordinal(
            pending_ordinal(&root, 1, body.len(), ContentHash::of(&body)).unwrap(),
            &root,
        )
        .unwrap();
        let base = lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64;
        let full = vec![0xff; base as usize + body.len()];
        let blob = Blob::new(ContentHash::of(&full), full.len() as u64).unwrap();
        let values = [
            pending_piece(&ordinal.id, 0, &body[..CHUNK_BYTES]).unwrap(),
            pending_piece(&ordinal.id, CHUNK_BYTES, &body[CHUNK_BYTES..]).unwrap(),
        ];
        let mut replay = PendingReplay::new(&ordinal).unwrap();
        let windows = crate::control::ImmutableWindows::from_charged_iter(
            replay_page_rows(
                values.clone().into_iter(),
                &mut replay,
                &blob,
                base,
                &budget,
            ),
            Some(attempt),
            None,
            None,
        )
        .with_budget(Some(&budget))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(
            windows.len(),
            1,
            "the fully drained two-piece page is one immutable window"
        );
        replay.finish().unwrap();
        drop(windows);
        assert_eq!(budget.reserved(), 0);
        let mut corrupted = values;
        if let Value::Object(row) = &mut corrupted[1] {
            row.insert("bytes", Bytes::from(vec![1, 2, 3]));
        }
        let mut replay = PendingReplay::new(&ordinal).unwrap();
        let mut refused = crate::control::ImmutableWindows::from_charged_iter(
            replay_page_rows(corrupted.into_iter(), &mut replay, &blob, base, &budget),
            Some(attempt),
            None,
            None,
        )
        .with_budget(Some(&budget));
        assert!(
            refused.next().unwrap().is_err(),
            "a valid prefix cannot expose a window containing a corrupted suffix"
        );
        assert!(refused.next().is_none());
        drop(refused);
        assert_eq!(
            budget.reserved(),
            0,
            "late page refusal releases prior construction and input charges"
        );
    }
    #[tokio::test]
    async fn failed_partial_ingress_closes_without_fabricating_semantic_completion() {
        let pending = PendingOriginals::default();
        let budget = ResourceBudget::fixed(4096).unwrap();
        pending.retain_budget(&budget).unwrap();
        // No connected service: terminal abandonment must not query absent headers or require
        // missing bytes. The enclosing attempt owner independently preserves effect receipts.
        let completion = pending
            .close_owned(
                std::sync::Arc::new(Surreal::init()),
                ContentHash([9; 32]),
                Err(ModelError::Conflict(
                    "unresolved native acknowledgement remains owned by attempt",
                )),
                false,
            )
            .await;
        lctx_model::domain::completion::complete(Ok(()), completion).unwrap();
        assert!(
            *pending.gate.lock().await,
            "terminal ingress is fenced even without semantic completion"
        );
        assert!(
            pending.has_ingress().unwrap(),
            "allocation authority remains identifiable until owner destruction"
        );
        let reader =
            crate::NativeReader::private(std::sync::Arc::new(Surreal::init())).with_budget(&budget);
        assert!(
            pending
                .chunks(&reader, ContentHash([9; 32]), None, &[], &budget)
                .await
                .is_err(),
            "no late ingress can reopen the closed producer"
        );
        assert_eq!(
            budget.reserved(),
            0,
            "native staging finality owns no host byte spool"
        );
    }
    #[test]
    fn replay_refuses_missing_reordered_duplicate_and_corrupted_suffixes() {
        let root = pending_id(ContentHash([3; 32]), &RecordId::new("original", "source")).unwrap();
        let mut body = vec![0xff; CHUNK_BYTES + 3];
        body[CHUNK_BYTES..].copy_from_slice(&[0, 0x80, 7]);
        let ordinal = decode_pending_ordinal(
            pending_ordinal(&root, 0, body.len(), ContentHash::of(&body)).unwrap(),
            &root,
        )
        .unwrap();
        let values = vec![
            pending_piece(&ordinal.id, 0, &body[..CHUNK_BYTES]).unwrap(),
            pending_piece(&ordinal.id, CHUNK_BYTES, &body[CHUNK_BYTES..]).unwrap(),
        ];
        let mut valid = PendingReplay::new(&ordinal).unwrap();
        for value in values.clone() {
            valid.push(value).unwrap();
        }
        valid.finish().unwrap();
        for defect in [
            "missing_suffix",
            "reordered",
            "duplicate",
            "short_first",
            "extra_envelope",
            "changed_suffix",
            "coherent_changed_suffix",
            "extra_piece",
        ] {
            let mut corrupted = values.clone();
            match defect {
                "missing_suffix" => {
                    corrupted.pop();
                }
                "reordered" => corrupted.reverse(),
                "duplicate" => corrupted.insert(1, corrupted[0].clone()),
                "short_first" => {
                    corrupted[0] = pending_piece(&ordinal.id, 0, &body[..CHUNK_BYTES - 1]).unwrap()
                }
                "extra_envelope" => {
                    if let Value::Object(row) = &mut corrupted[1] {
                        row.insert("extra", Value::Null);
                    }
                }
                "changed_suffix" => {
                    if let Value::Object(row) = &mut corrupted[1] {
                        row.insert("bytes", Bytes::from(vec![0, 0x80, 8]));
                    }
                }
                "coherent_changed_suffix" => {
                    corrupted[1] = pending_piece(&ordinal.id, CHUNK_BYTES, &[0, 0x80, 8]).unwrap()
                }
                _ => corrupted.push(pending_piece(&ordinal.id, CHUNK_BYTES * 2, &[1]).unwrap()),
            }
            let result = (|| {
                let mut replay = PendingReplay::new(&ordinal)?;
                for value in corrupted {
                    replay.push(value)?;
                }
                replay.finish()
            })();
            assert!(
                result.is_err(),
                "a valid prefix never hides malformed persisted replay: {defect}"
            );
        }
        let mut extra = pending_ordinal(&root, 0, body.len(), ContentHash::of(&body)).unwrap();
        if let Value::Object(row) = &mut extra {
            row.insert("extra", Value::Null);
        }
        assert!(
            decode_pending_ordinal(extra, &root).is_err(),
            "ordinal metadata is a closed envelope as well"
        );
    }
    #[test]
    fn pending_rows_preserve_attempt_source_ordinal_and_full_bytes() {
        let a = ContentHash([1; 32]);
        let b = ContentHash([2; 32]);
        let source = RecordId::new("original", "source");
        let root = pending_id(a, &source).unwrap();
        assert_ne!(root, pending_id(b, &source).unwrap());
        let ordinal = ordinal_id(&root, 3).unwrap();
        assert_ne!(ordinal, ordinal_id(&root, 4).unwrap());
        let bytes = [0xff, 0, 0x80];
        let value = pending_piece(&ordinal, 0, &bytes).unwrap();
        let row = PendingPiece::from_value(value.clone()).unwrap();
        assert_eq!(row.bytes.as_ref(), &bytes);
        let mut corrupt = value.clone();
        if let Value::Object(row) = &mut corrupt {
            row.insert("bytes", Bytes::from(vec![0xfe, 0, 0x80]));
        }
        assert!(exact(&corrupt, &value, "pending full byte replay").is_err());
        assert!(pending_piece(&ordinal, 1, &bytes).is_err());
        assert!(pending_ordinal(&root, 3, 0, ContentHash::of(b"")).is_err());
    }
}
