//! Full canonical and physical readback before a database can become a published snapshot.
use crate::{Loader, codec};
use lctx_model::domain::{
    ContentHash, ContentHasher, ModelError,
    graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest, Target},
};
use surrealdb::types::{Bytes, RecordId, SurrealValue, Value, Variables};
fn canonical_size(actual: &Value) -> Result<usize, ModelError> {
    match actual {
        Value::Object(object) => match object.get("canonical") {
            Some(Value::Bytes(bytes)) => Ok(bytes.len()),
            _ => Err(ModelError::Schema("native canonical payload")),
        },
        _ => Err(ModelError::Schema("native canonical object")),
    }
}
fn decode_entity(actual: &Value) -> Result<Entity, ModelError> {
    let node = Node::from_value(actual.clone())
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))?;
    serde_json::from_slice(&node.canonical)
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))
}
fn decode_assertion(actual: &Value) -> Result<Assertion, ModelError> {
    let node = Node::from_value(actual.clone())
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))?;
    serde_json::from_slice(&node.canonical)
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Node {
    id: RecordId,
    canonical: Bytes,
}
impl Loader {
    #[allow(
        clippy::mutable_key_type,
        reason = "SDK RecordId includes regex caches; these generated string IDs are never mutated"
    )]
    pub async fn reconcile(&self, manifest: &Manifest) -> Result<(), ModelError> {
        manifest.validate()?;
        let budget = self.read_budget()?;
        let mut expected = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        let reader = self.reader();
        for (table, family) in [
            ("entity", GraphFamily::Entities),
            ("assertion", GraphFamily::Assertions),
        ] {
            let mut rows = reader.selected_payload_rows(table, "true", Variables::new(), vec![], "anchor,id", None)?;
            let mut hasher = FamilyHasher::new(family);
            let limits = lctx_model::domain::batching::TransferLimits::default();
            let mut pending = rows.next().await?;
            while let Some(first) = pending.take() {
                let mut window = vec![first];
                let mut bytes = canonical_size(&window[0])?;
                while window.len() < limits.rows && bytes < limits.bytes {
                    let Some(next) = rows.next().await? else {
                        break;
                    };
                    let next_bytes = canonical_size(&next)?;
                    if bytes.saturating_add(next_bytes) > limits.bytes {
                        pending = Some(next);
                        break;
                    }
                    bytes += next_bytes;
                    window.push(next);
                }
                let entities = if family == GraphFamily::Entities {
                    window
                        .iter()
                        .map(decode_entity)
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    vec![]
                };
                let assertions = if family == GraphFamily::Assertions {
                    window
                        .iter()
                        .map(decode_assertion)
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    vec![]
                };
                let views = if family == GraphFamily::Entities {
                    codec::entity_views(&entities)?
                } else {
                    codec::assertion_views(&assertions)?
                };
                for (index, (actual, view)) in window.into_iter().zip(views).enumerate() {
                    let row = Node::from_value(actual.clone()).map_err(|_| {
                        ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
                    })?;
                    let (key, content, kind, subtype, view, canonical) = if family
                        == GraphFamily::Entities
                    {
                        let entity = &entities[index];
                        entity.validate().map_err(crate::reader::canonical_error)?;
                        for (position, reference) in
                            codec::entity_references(entity).into_iter().enumerate()
                        {
                            let target = lctx_model::domain::graph::reference_target(&reference)?.0;
                            remember_external(&mut expected, &target)?;
                            expected.push(crate::loader::edge(
                                "reference",
                                row.id.clone(),
                                target,
                                reference.field,
                                0,
                                Some(position as u32),
                            )?)?;
                        }
                        (
                            entity.id().0,
                            entity.content(),
                            entity.kind() as i64,
                            entity.subtype(),
                            view,
                            serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                        )
                    } else {
                        let assertion = &assertions[index];
                        assertion
                            .validate()
                            .map_err(crate::reader::canonical_error)?;
                        for p in &assertion.participants {
                            remember_external(&mut expected, &p.target)?;
                            expected.push(crate::loader::edge(
                                "participant",
                                row.id.clone(),
                                p.target.clone(),
                                p.field.as_deref().unwrap_or(""),
                                p.role as i64,
                                p.position,
                            )?)?;
                        }
                        for (position, (target, _)) in
                            assertion.references()?.into_iter().enumerate()
                        {
                            if assertion.participants.iter().any(|p| p.target == target) {
                                continue;
                            }
                            remember_external(&mut expected, &target)?;
                            expected.push(crate::loader::edge(
                                "participant",
                                row.id.clone(),
                                target,
                                "__reference",
                                -1,
                                Some(position as u32),
                            )?)?;
                        }
                        (
                            assertion.id().0,
                            assertion.content(),
                            assertion.kind as i64,
                            None,
                            view,
                            serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                        )
                    };
                    let physical = crate::adapter::physical_row(
                        crate::loader::payload_id(table, &view.semantic_type, &key.0, content)?,
                        content,
                        canonical,
                        Some((kind, subtype)),
                        view,
                    )?;
                    if serde_json::to_vec(&actual).map_err(ModelError::codec)?
                        != serde_json::to_vec(&physical).map_err(ModelError::codec)?
                    {
                        return Err(ModelError::Serving(
                            lctx_model::domain::serving::FailureKind::Corrupt,
                        ));
                    }
                    if !hasher.push(key, content)? {
                        return Err(ModelError::Conflict(
                            "duplicate native canonical graph element",
                        ));
                    }
                }
                if pending.is_none() {
                    pending = rows.next().await?;
                }
            }
            if !manifest.families.contains(&hasher.finish()) {
                return Err(ModelError::Conflict(
                    "native canonical family reconciliation",
                ));
            }
        }
        let mut actual = actual_roles(&reader, &budget)?;
        expected.finish()?.reconcile(&mut actual).await?;
        if self.view_ids().is_none()
            && table_count(self, "original").await? != manifest.originals.len() as u64
        {
            return Err(ModelError::Conflict("native original inventory"));
        }
        let mut original_chunks = 0;
        for original in &manifest.originals {
            let source = RecordId::new("original", original.source.0.hex());
            let mut bind = Variables::new();
            bind.insert("source", source.clone());
            self.check_read_admission()?;
            let mut response = self
                .client()
                .query("SELECT * FROM $source")
                .bind(bind)
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let headers: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
            let [header] = headers.as_slice() else {
                return Err(ModelError::Conflict("native original header"));
            };
            validate_original_header(header, &source, original.content, original.byte_len)?;
            let mut hash = ContentHasher::default();
            let mut position = 0u64;
            loop {
                let mut bind = Variables::new();
                bind.insert("source", source.clone());
                bind.insert("start", position);
                self.check_read_admission()?;
                let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start ORDER BY start LIMIT 128").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let chunks: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
                if chunks.is_empty() {
                    break;
                }
                for actual in chunks {
                    let chunk = decode_original_chunk(actual)?;
                    original_chunks += 1;
                    if chunk.source != source || chunk.start != position {
                        return Err(ModelError::Conflict("native original chunk"));
                    }
                    hash.update(&chunk.bytes);
                    position += chunk.bytes.len() as u64;
                }
            }
            if position != original.byte_len || hash.finish() != original.content {
                return Err(ModelError::Conflict("native original readback"));
            }
        }
        if self.view_ids().is_none()
            && table_count(self, "original_chunk").await? != original_chunks
        {
            return Err(ModelError::Conflict("native original chunk inventory"));
        }
        Ok(())
    }
}
/// Actual outgoing rows are nominated by selected sources, never by expected role IDs.
pub fn actual_roles<Context>(reader: &crate::NativeReader<Context>, budget: &lctx_model::domain::resources::ResourceBudget) -> Result<crate::reader::NativeRows, ModelError> {
    let selection = reader.selection_preparation(); let client = reader.shared_client(); let cancellation = reader.read_cancellation(); let budget = budget.clone();
    crate::reader::NativeRows::owned(move |sender| async move {
        let selection = selection.await?;
        let mut actual = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        let mut external = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        for table in ["participant", "reference"] {
            let mut rows = if let Some(selection) = &selection { selection.outgoing(table)? } else {
                crate::prepared::PreparedQuery::new(Variables::new(), vec![], vec![format!("SELECT * FROM {table}")])?.stream_cancellable(&client, cancellation.as_ref())?
            };
            let result = async { while let Some(row) = rows.next().await? {
                let Value::Object(object) = &row else { return Err(ModelError::Schema("actual native role")); };
                if let Some(Value::RecordId(out)) = object.get("out") {
                    if out.table.as_str() == "external" {
                        let mut pointer = surrealdb::types::Object::new(); pointer.insert("id", out.clone()); external.push(Value::Object(pointer))?;
                    }
                }
                actual.push(row)?;
            } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("actual role drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut external = external.finish()?;
        loop {
            let mut ids = Vec::new(); while ids.len() < 128 { let Some(row) = external.next_row()? else { break; };
                let Value::Object(row) = row else { return Err(ModelError::Schema("actual external pointer")); };
                ids.push(RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("actual external id"))?).map_err(ModelError::codec)?);
            }
            if ids.is_empty() { break; }
            let mut vars = Variables::new(); vars.insert("ids", ids);
            let mut rows = crate::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT * FROM $ids".into()])?.stream_cancellable(&client, cancellation.as_ref())?;
            let result = async { while let Some(row) = rows.next().await? { actual.push(row)?; } Ok(()) }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default(); terminal.step("actual external drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut actual = actual.finish()?;
        while let Some(row) = actual.next_row()? { if sender.send(row).await.is_err() { break; } }
        Ok(())
    })
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Chunk {
    id: RecordId,
    source: RecordId,
    start: u64,
    content: String,
    bytes: Bytes,
}
impl<Context> crate::NativeReader<Context> {
    /// One indexed union query for a bounded set of exact original byte ranges.
    /// Results preserve request order, including duplicates; source/chunk integrity is checked.
    pub async fn original_bytes_batch(
        &self,
        ranges: &[(lctx_model::domain::graph::EntityId, u64, usize)],
    ) -> Result<Vec<Vec<u8>>, ModelError> {
        if ranges.len() > 32
            || ranges
                .iter()
                .try_fold(0usize, |sum, (_, _, n)| sum.checked_add(*n))
                .is_none_or(|n| n > 256 * 1024)
        {
            return Err(ModelError::Invalid("original union byte bound".into()));
        }
        if ranges.is_empty() {
            return Ok(vec![]);
        }
        self.authorize_original_ranges(ranges)?;
        let mut eligible = self.view_bindings();
        if eligible.get("lctx_views").is_some() {
            let anchors = ranges
                .iter()
                .map(|(source, _, _)| crate::reader::target_id(Target::Entity(*source)))
                .collect::<Vec<_>>();
            eligible.insert("anchors", anchors.clone());
            let mut candidates = self.stream_prepared(crate::prepared::PreparedQuery::new(eligible, vec![], vec!["SELECT id,anchor FROM entity WITH INDEX anchor_payload WHERE anchor IN $anchors".into()])?)?;
            let mut found = Vec::new();
            let result = async {
                while let Some(row) = candidates.next().await? {
                    let Value::Object(row) = row else { return Err(ModelError::Schema("original candidate object")); };
                    let node = RecordId::from_value(row.get("id").cloned().ok_or(ModelError::Schema("original candidate identity"))?).map_err(ModelError::codec)?;
                    if !self.selected_candidate_ids(&[node], &self.resource_budget()?).await?.is_empty() { found.push(RecordId::from_value(row.get("anchor").cloned().ok_or(ModelError::Schema("original candidate anchor"))?).map_err(ModelError::codec)?); }
                }
                Ok(())
            }.await;
            let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("original candidate drainage", candidates.drain_transport().await); lctx_model::domain::completion::complete(result, completion)?;
            if anchors.iter().any(|anchor| !found.contains(anchor)) {
                return Err(ModelError::Conflict("original source outside exact view"));
            }
        }
        let (sources, physical) = physical_original_ranges(ranges)?;
        if physical.is_empty() {
            return Ok(ranges.iter().map(|_| vec![]).collect());
        }
        let mut vars = Variables::new();
        vars.insert(
            "chunks",
            physical
                .iter()
                .map(|key| RecordId::new("original_chunk", key.clone()))
                .collect::<Vec<_>>(),
        );
        let sql = "SELECT * FROM $chunks ORDER BY source,start";
        let chunks: Vec<Value> = self.query_native(sql, vars).await?;
        let chunks = index_original_chunks(chunks, &physical)?;
        ranges
            .iter()
            .zip(sources)
            .map(|((_, start, length), source)| {
                assemble_original(&chunks, &source, *start, *length)
            })
            .collect()
    }
    pub async fn original_bytes(
        &self,
        source: lctx_model::domain::graph::EntityId,
        start: u64,
        length: usize,
    ) -> Result<Vec<u8>, ModelError> {
        if length > 256 << 10 {
            return Err(ModelError::Invalid(
                "original byte page exceeds response bound".into(),
            ));
        }
        self.original_bytes_batch(&[(source, start, length)])
            .await?
            .pop()
            .ok_or(ModelError::Schema("original logical result inventory"))
    }
}

fn remember_external(
    rows: &mut crate::ordered_rows::SortedRows,
    target: &Target,
) -> Result<(), ModelError> {
    if matches!(target, Target::External { .. }) {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", crate::reader::target_id(target.clone()));
        row.insert(
            "canonical",
            Bytes::from(serde_json::to_vec(target).map_err(ModelError::codec)?),
        );
        rows.push(Value::Object(row))?;
    }
    Ok(())
}
pub fn add_scope_fields(
    object: &mut surrealdb::types::Object,
    body: &Value,
    semantic_type: &str,
    table: crate::schema::ScopeTable,
) -> Result<(), ModelError> {
    crate::adapter::add_scope_fields(object, body, semantic_type, table)
}

pub async fn table_count(loader: &Loader, table: &str) -> Result<u64, ModelError> {
    loader.check_read_admission()?;
    let mut response = loader
        .client()
        .query(format!("SELECT count() AS total FROM {table} GROUP ALL"))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let counts: Vec<InventoryCount> = response.take(0).map_err(ModelError::codec)?;
    Ok(counts.first().map_or(0, |row| row.total))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct InventoryCount {
    total: u64,
}

fn exact_original_row(
    actual: &Value,
    expected: Value,
    message: &'static str,
) -> Result<(), ModelError> {
    if serde_json::to_vec(actual).map_err(ModelError::codec)?
        != serde_json::to_vec(&expected).map_err(ModelError::codec)?
    {
        return Err(ModelError::Conflict(message));
    }
    Ok(())
}
fn validate_original_header(
    actual: &Value,
    source: &RecordId,
    content: ContentHash,
    length: u64,
) -> Result<(), ModelError> {
    let mut expected = surrealdb::types::Object::new();
    expected.insert("id", source.clone());
    expected.insert("content", content.hex());
    expected.insert(
        "byte_len",
        i64::try_from(length).map_err(ModelError::codec)?,
    );
    exact_original_row(actual, Value::Object(expected), "native original header")
}
fn original_chunk_value(chunk: &Chunk) -> Result<Value, ModelError> {
    let surrealdb::types::RecordIdKey::String(source) = &chunk.source.key else {
        return Err(ModelError::Schema("original source key"));
    };
    if chunk.source.table.as_str() != "original"
        || !chunk.start.is_multiple_of(65536)
        || chunk.bytes.is_empty()
        || chunk.bytes.len() > 65536
        || chunk
            .start
            .checked_add(chunk.bytes.len() as u64)
            .is_none_or(|end| end > i64::MAX as u64)
    {
        return Err(ModelError::Conflict("original chunk content"));
    }
    let mut expected = surrealdb::types::Object::new();
    expected.insert(
        "id",
        RecordId::new("original_chunk", format!("{source}_{}", chunk.start)),
    );
    expected.insert("source", chunk.source.clone());
    expected.insert(
        "start",
        i64::try_from(chunk.start).map_err(ModelError::codec)?,
    );
    expected.insert("bytes", chunk.bytes.clone());
    expected.insert("content", ContentHash::of(&chunk.bytes).hex());
    Ok(Value::Object(expected))
}
fn decode_original_chunk(actual: Value) -> Result<Chunk, ModelError> {
    let chunk = Chunk::from_value(actual.clone()).map_err(ModelError::codec)?;
    exact_original_row(
        &actual,
        original_chunk_value(&chunk)?,
        "original chunk complete envelope",
    )?;
    Ok(chunk)
}
type OriginalChunks = std::collections::BTreeMap<String, std::collections::BTreeMap<u64, Chunk>>;
fn physical_original_ranges(
    ranges: &[(lctx_model::domain::graph::EntityId, u64, usize)],
) -> Result<(Vec<RecordId>, std::collections::BTreeSet<String>), ModelError> {
    let mut physical = std::collections::BTreeSet::new();
    let mut sources = Vec::with_capacity(ranges.len());
    for (source, start, length) in ranges {
        let end = start
            .checked_add(*length as u64)
            .filter(|end| *end <= i64::MAX as u64)
            .ok_or(ModelError::Schema("original union byte range"))?;
        let source_key = source.0.hex();
        sources.push(RecordId::new("original", source_key.clone()));
        if *length == 0 {
            continue;
        }
        let mut chunk = start / 65536 * 65536;
        while chunk < end {
            physical.insert(format!("{source_key}_{chunk}"));
            chunk = chunk
                .checked_add(65536)
                .ok_or(ModelError::Schema("original chunk bounds"))?;
        }
    }
    Ok((sources, physical))
}
fn index_original_chunks(
    chunks: Vec<Value>,
    requested: &std::collections::BTreeSet<String>,
) -> Result<OriginalChunks, ModelError> {
    let mut index = OriginalChunks::new();
    for actual in chunks {
        let chunk = decode_original_chunk(actual)?;
        let surrealdb::types::RecordIdKey::String(source) = &chunk.source.key else {
            return Err(ModelError::Schema("original source key"));
        };
        if !requested.contains(&format!("{source}_{}", chunk.start)) {
            return Err(ModelError::Conflict("original chunk content"));
        }
        if index
            .entry(source.clone())
            .or_default()
            .insert(chunk.start, chunk)
            .is_some()
        {
            return Err(ModelError::Conflict("original chunk inventory"));
        }
    }
    Ok(index)
}
fn assemble_original(
    chunks: &OriginalChunks,
    source: &RecordId,
    start: u64,
    length: usize,
) -> Result<Vec<u8>, ModelError> {
    let end = start
        .checked_add(length as u64)
        .ok_or(ModelError::Schema("original union bounds"))?;
    let mut position = start;
    let mut result = Vec::with_capacity(length);
    let surrealdb::types::RecordIdKey::String(source) = &source.key else {
        return Err(ModelError::Schema("original source key"));
    };
    if let Some(chunks) = chunks.get(source) {
        for (_, chunk) in chunks.range(start / 65536 * 65536..end) {
            if position == end {
                break;
            }
            let offset = position
                .checked_sub(chunk.start)
                .filter(|value| *value < chunk.bytes.len() as u64)
                .ok_or(ModelError::Schema("original chunk continuity"))?
                as usize;
            let count = (end - position).min((chunk.bytes.len() - offset) as u64) as usize;
            result.extend_from_slice(&chunk.bytes[offset..offset + count]);
            position += count as u64;
        }
    }
    if position != end {
        return Err(ModelError::Invalid(
            "original byte range unavailable".into(),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::graph::EntityId;
    fn source(label: &[u8]) -> EntityId {
        EntityId(ContentHash::of(label))
    }
    fn chunk(source: EntityId, start: u64, bytes: Vec<u8>) -> Value {
        Chunk {
            id: RecordId::new("original_chunk", format!("{}_{start}", source.0.hex())),
            source: RecordId::new("original", source.0.hex()),
            start,
            content: ContentHash::of(&bytes).hex(),
            bytes: Bytes::from(bytes),
        }
        .into_value()
    }
    #[test]
    fn original_reconstruction_rejects_changed_ids_extra_and_missing_envelope_fields() {
        let chunk =
            decode_original_chunk(chunk(source(b"complete-envelope"), 0, vec![0xff, 0, 0x80]))
                .unwrap();
        let canonical = original_chunk_value(&chunk).unwrap();
        decode_original_chunk(canonical.clone()).unwrap();
        for defect in ["id", "extra", "missing_content"] {
            let mut altered = canonical.as_object().unwrap().clone();
            match defect {
                "id" => {
                    altered.insert("id", RecordId::new("original_chunk", "wrong"));
                }
                "extra" => {
                    altered.insert("extra", Value::Null);
                }
                "missing_content" => {
                    altered.remove("content");
                }
                _ => unreachable!(),
            }
            assert!(
                decode_original_chunk(Value::Object(altered)).is_err(),
                "complete original chunk envelope: {defect}"
            );
        }
        let mut header = surrealdb::types::Object::new();
        header.insert("id", chunk.source.clone());
        header.insert("content", chunk.content.clone());
        header.insert("byte_len", 3i64);
        validate_original_header(
            &Value::Object(header.clone()),
            &chunk.source,
            ContentHash::of(&chunk.bytes),
            3,
        )
        .unwrap();
        header.insert("extra", Value::Null);
        assert!(
            validate_original_header(
                &Value::Object(header),
                &chunk.source,
                ContentHash::of(&chunk.bytes),
                3
            )
            .is_err()
        );
    }
    #[test]
    fn original_union_reuses_overlapping_chunks_and_preserves_logical_order() {
        let a = source(b"a");
        let b = source(b"b");
        let ranges = [
            (a, 65534, 4),
            (a, 7, 3),
            (a, 65534, 4),
            (b, 0, 2),
            (a, 0, 0),
        ];
        let (sources, physical) = physical_original_ranges(&ranges).unwrap();
        assert_eq!(
            physical.len(),
            3,
            "overlap and duplicates select each physical chunk once"
        );
        let mut first = vec![0; 65536];
        first[7..10].copy_from_slice(b"abc");
        first[65534..].copy_from_slice(b"xy");
        let index = index_original_chunks(
            vec![
                chunk(b, 0, b"BB".to_vec()),
                chunk(a, 65536, b"zw".to_vec()),
                chunk(a, 0, first),
            ],
            &physical,
        )
        .unwrap();
        let results = ranges
            .iter()
            .zip(sources)
            .map(|((_, start, len), source)| {
                assemble_original(&index, &source, *start, *len).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            results,
            vec![
                b"xyzw".to_vec(),
                b"abc".to_vec(),
                b"xyzw".to_vec(),
                b"BB".to_vec(),
                vec![]
            ]
        );
    }
    #[test]
    fn original_union_rejects_missing_corrupt_unrequested_and_duplicate_chunks() {
        let a = source(b"integrity");
        let (sources, physical) = physical_original_ranges(&[(a, 65535, 2)]).unwrap();
        let index = index_original_chunks(vec![chunk(a, 0, vec![1; 65536])], &physical).unwrap();
        assert!(assemble_original(&index, &sources[0], 65535, 2).is_err());
        let mut corrupt = chunk(a, 0, vec![1; 65536]);
        let Value::Object(fields) = &mut corrupt else {
            panic!("original chunk");
        };
        fields.insert("bytes", Bytes::from(vec![2; 65536]));
        assert!(index_original_chunks(vec![corrupt], &physical).is_err());
        assert!(index_original_chunks(vec![chunk(a, 131072, vec![1])], &physical).is_err());
        assert!(
            index_original_chunks(vec![chunk(a, 0, vec![1]), chunk(a, 0, vec![1])], &physical)
                .is_err()
        );
        assert!(physical_original_ranges(&[(a, u64::MAX, 1)]).is_err());
    }
}
