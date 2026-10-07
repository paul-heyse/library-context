//! Full canonical and physical readback before a database can become a published snapshot.
use crate::{Loader, codec, loader::json_value};
use lctx_model::domain::{
    ContentHash, ContentHasher, ModelError,
    graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest, Target},
};
use surrealdb::types::{Bytes, RecordId, SurrealValue, ToSql, Value, Variables};
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
        let mut expected = crate::ordered_rows::SortedRows::new()?;
        let snapshot = lctx_model::domain::serving::SnapshotHandle {
            semantic: manifest.content(),
            realization: ContentHash::of(b"private-reconciliation"),
            database: lctx_model::domain::serving::DatabaseIdentity {
                namespace: lctx_model::domain::serving::Name::new("private")
                    .map_err(ModelError::codec)?,
                database: lctx_model::domain::serving::Name::new("private")
                    .map_err(ModelError::codec)?,
            },
        };
        let reader = crate::NativeReader::new(self.shared_client(), snapshot);
        for (table, family) in [
            ("entity", GraphFamily::Entities),
            ("assertion", GraphFamily::Assertions),
        ] {
            let mut rows = reader.query_stream(
                format!("SELECT * FROM {table} ORDER BY id"),
                Variables::new(),
                1,
            )?;
            let mut hasher = FamilyHasher::new(family);
            while let Some(actual) = rows.next().await? {
                let row = Node::from_value(actual.clone()).map_err(|_| {
                    ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
                })?;
                let (key, content, kind, subtype, view, canonical) = if family
                    == GraphFamily::Entities
                {
                    let entity: Entity = serde_json::from_slice(&row.canonical).map_err(|_| {
                        ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
                    })?;
                    entity.validate().map_err(crate::reader::canonical_error)?;
                    for (position, reference) in
                        codec::entity_references(&entity).into_iter().enumerate()
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
                        codec::entity_view(&entity)?,
                        serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                    )
                } else {
                    let assertion: Assertion =
                        serde_json::from_slice(&row.canonical).map_err(|_| {
                            ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
                        })?;
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
                    for (position, (target, _)) in assertion.references()?.into_iter().enumerate() {
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
                        codec::assertion_view(&assertion)?,
                        serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                    )
                };
                let mut physical = surrealdb::types::Object::new();
                physical.insert("id", RecordId::new(table, key.hex()));
                physical.insert("semantic_type", view.semantic_type.clone());
                physical.insert("semantic_key", view.semantic_key);
                physical.insert("kind", kind);
                physical.insert("subtype", subtype.map(Value::from_t).unwrap_or(Value::Null));
                physical.insert("content", content.hex());
                physical.insert("canonical", Bytes::from(canonical));
                let body = json_value(view.body)?;
                add_scope_fields(&mut physical, &body, &view.semantic_type)?;
                physical.insert("body", body);
                if serde_json::to_vec(&actual).map_err(ModelError::codec)?
                    != serde_json::to_vec(&Value::Object(physical)).map_err(ModelError::codec)?
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
            if !manifest.families.contains(&hasher.finish()) {
                return Err(ModelError::Conflict(
                    "native canonical family reconciliation",
                ));
            }
        }
        let mut actual = reader.query_stream("SELECT * FROM external ORDER BY id; SELECT * FROM participant ORDER BY id; SELECT * FROM reference ORDER BY id", Variables::new(), 3)?;
        expected.finish()?.reconcile(&mut actual).await?;
        if table_count(self, "original").await? != manifest.originals.len() as u64 {
            return Err(ModelError::Conflict("native original inventory"));
        }
        let mut original_chunks = 0;
        for original in &manifest.originals {
            let source = RecordId::new("original", original.source.0.hex());
            let mut bind = Variables::new();
            bind.insert("source", source.clone());
            let mut response = self
                .client()
                .query("SELECT * FROM $source")
                .bind(bind)
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let headers: Vec<Header> = response.take(0).map_err(ModelError::codec)?;
            if headers.len() != 1
                || headers[0].content != original.content.hex()
                || headers[0].byte_len != original.byte_len
            {
                return Err(ModelError::Conflict("native original header"));
            }
            let mut hash = ContentHasher::default();
            let mut position = 0u64;
            loop {
                let mut bind = Variables::new();
                bind.insert("source", source.clone());
                bind.insert("start", position);
                let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start ORDER BY start LIMIT 128").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let chunks: Vec<Chunk> = response.take(0).map_err(ModelError::codec)?;
                if chunks.is_empty() {
                    break;
                }
                for chunk in chunks {
                    original_chunks += 1;
                    if chunk.start != position
                        || chunk.bytes.is_empty()
                        || chunk.bytes.len() > 65536
                        || ContentHash::of(&chunk.bytes).hex() != chunk.content
                    {
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
        if table_count(self, "original_chunk").await? != original_chunks {
            return Err(ModelError::Conflict("native original chunk inventory"));
        }
        Ok(())
    }
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Header {
    id: RecordId,
    content: String,
    byte_len: u64,
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
impl crate::NativeReader {
    /// One indexed union query for a bounded set of exact original byte ranges.
    /// Results preserve request order, including duplicates; source/chunk integrity is checked.
    pub async fn original_bytes_batch(&self,ranges:&[(lctx_model::domain::graph::EntityId,u64,usize)])->Result<Vec<Vec<u8>>,ModelError> {
        if ranges.len()>32 || ranges.iter().try_fold(0usize,|sum,(_,_,n)|sum.checked_add(*n)).is_none_or(|n|n>256*1024) {return Err(ModelError::Invalid("original union byte bound".into()));}
        if ranges.is_empty(){return Ok(vec![]);}
        let mut vars=Variables::new(); let mut clauses=vec![]; let mut sources=vec![];
        for (i,(source,start,length)) in ranges.iter().enumerate() {
            let end=start.checked_add(*length as u64).filter(|end|*end<=i64::MAX as u64).ok_or(ModelError::Schema("original union byte range"))?;
            let source=RecordId::new("original",source.0.hex());sources.push(source.clone());
            if *length==0 {continue;}
            vars.insert(format!("source_{i}"),source);vars.insert(format!("start_{i}"),start/65536*65536);vars.insert(format!("end_{i}"),end);
            clauses.push(format!("(source=$source_{i} AND start >= $start_{i} AND start < $end_{i})"));
        }
        if clauses.is_empty(){return Ok(ranges.iter().map(|_|vec![]).collect());}
        let sql=format!("SELECT * FROM original_chunk WHERE {} ORDER BY source,start",clauses.join(" OR "));
        let mut response=self.client().query(sql).bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let chunks:Vec<Chunk>=response.take(0).map_err(ModelError::codec)?;
        for chunk in &chunks {if ContentHash::of(&chunk.bytes).hex()!=chunk.content {return Err(ModelError::Conflict("original chunk content"));}}
        ranges.iter().zip(sources).map(|((_,start,length),source)|assemble_original(&chunks,&source,*start,*length)).collect()
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
        let end = start
            .checked_add(length as u64)
            .ok_or(ModelError::Schema("original byte range"))?;
        let mut bind = Variables::new();
        bind.insert("source", RecordId::new("original", source.0.hex()));
        bind.insert("start", start / 65536 * 65536);
        bind.insert("end", end);
        let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start AND start < $end ORDER BY start").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let chunks: Vec<Chunk> = response.take(0).map_err(ModelError::codec)?;
        let mut result = Vec::with_capacity(length);
        let mut position = start;
        for chunk in chunks {
            if ContentHash::of(&chunk.bytes).hex() != chunk.content {
                return Err(ModelError::Conflict("original chunk content"));
            }
            let offset = position
                .checked_sub(chunk.start)
                .filter(|v| *v < chunk.bytes.len() as u64)
                .ok_or(ModelError::Schema("original chunk continuity"))?
                as usize;
            let count = (end - position).min((chunk.bytes.len() - offset) as u64) as usize;
            result.extend_from_slice(&chunk.bytes[offset..offset + count]);
            position += count as u64;
        }
        if position != end {
            return Err(ModelError::Invalid(
                "original byte range unavailable".into(),
            ));
        }
        Ok(result)
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
/// The schema's persisted <string> lowering, derived from canonical fields, including NULL.
pub fn scope_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_sql(),
    }
}
pub fn add_scope_fields(
    object: &mut surrealdb::types::Object,
    body: &Value,
    semantic_type: &str,
) -> Result<(), ModelError> {
    let Value::Object(body) = body else {
        return Err(ModelError::Schema("canonical scope body"));
    };
    let mut keys = Vec::new();
    for field in crate::schema::SCOPE_FIELDS {
        if let Some(value) = body
            .get(*field)
            .filter(|value| !matches!(value, Value::None))
        {
            let text = scope_string(value);
            object.insert(format!("scope_{field}"), text.clone());
            if !matches!(value, Value::Null) {
                keys.push(format!("{semantic_type}|{field}|{text}"));
            }
        }
    }
    object.insert("scope_keys", keys);
    Ok(())
}
pub async fn table_count(loader: &Loader, table: &str) -> Result<u64, ModelError> {
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

fn assemble_original(chunks:&[Chunk],source:&RecordId,start:u64,length:usize)->Result<Vec<u8>,ModelError> {
    let end=start.checked_add(length as u64).ok_or(ModelError::Schema("original union bounds"))?;
    let mut position=start;let mut result=Vec::with_capacity(length);
    for chunk in chunks.iter().filter(|chunk|chunk.source==*source && chunk.start<end && chunk.start+chunk.bytes.len() as u64>start) {
        if position==end {break;}
        let offset=position.checked_sub(chunk.start).filter(|v|*v<chunk.bytes.len() as u64).ok_or(ModelError::Schema("original chunk continuity"))? as usize;
        let count=(end-position).min((chunk.bytes.len()-offset) as u64) as usize;
        result.extend_from_slice(&chunk.bytes[offset..offset+count]);position+=count as u64;
    }
    if position!=end{return Err(ModelError::Invalid("original byte range unavailable".into()));}Ok(result)
}
