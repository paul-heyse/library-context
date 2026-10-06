//! One terminally checked lowering for private construction and read-only cold reconciliation.
use lctx_model::domain::{
    embedding::{EmbeddingSpec, value},
    graph::{EntityId, Target},
    retrieval::{Family, Fragment, OriginalAnchor, Subject, Unit, UnitSubject, consumption::RetrievalEmbeddingUse},
    serving::{DatabaseIdentity, Name, SnapshotHandle}, *,
};
use lctx_surrealdb::surrealdb::types::{Bytes, Object, RecordId, Value, Variables, SurrealValue, SerdeWrapper};
use lctx_surrealdb::{Loader, NativeReader, RecordSelection, reader::target_id, ordered_rows::{SortedRows, OrderedRows}, reconciliation::scope_string};
use serde::{Deserialize, Serialize};
const TABLES: [&str; 7] = ["search_api_options", "search_documentation_deployment", "search_scenario", "search_source", "vector", "lex_occurs", "vec_occurs"];
const BATCH_ROWS: usize = 128;
const BATCH_BYTES: usize = 1024 * 1024;
fn table(family: Family) -> &'static str {
    match family { Family::ApiOptions => TABLES[0], Family::DocumentationDeployment => TABLES[1], Family::Scenario => TABLES[2], Family::Source => TABLES[3] }
}
fn reader(loader: &Loader) -> Result<NativeReader, ModelError> {
    Ok(NativeReader::new(loader.shared_client(), SnapshotHandle {
        semantic: ContentHash::of(b"private-lowering"), realization: ContentHash::of(b"private-lowering"),
        database: DatabaseIdentity { namespace: Name::new("private").map_err(ModelError::codec)?, database: Name::new("private").map_err(ModelError::codec)? },
    }))
}
struct Expected {
    pending: Vec<Option<SortedRows>>,
    ordered: Vec<Option<OrderedRows>>,
}
impl Expected {
    fn new() -> Result<Self, ModelError> {
        Ok(Self { pending: (0..7).map(|_| SortedRows::new().map(Some)).collect::<Result<_, _>>()?, ordered: (0..7).map(|_| None).collect() })
    }
    fn emit(&mut self, table: &str, row: Value) -> Result<(), ModelError> {
        let index = TABLES.iter().position(|candidate| *candidate == table).ok_or(ModelError::Schema("derived table"))?;
        self.pending[index].as_mut().ok_or(ModelError::Schema("finished derived family"))?.push(row)
    }
    fn finish(&mut self, index: usize) -> Result<&mut OrderedRows, ModelError> {
        if self.ordered[index].is_none() {
            self.ordered[index] = Some(self.pending[index].take().ok_or(ModelError::Schema("derived ordering"))?.finish()?);
        }
        Ok(self.ordered[index].as_mut().expect("ordered family"))
    }
    async fn reconcile(&mut self, reader: &NativeReader) -> Result<(), ModelError> {
        for (index, table) in TABLES.iter().enumerate() {
            let mut actual = reader.query_stream(format!("SELECT * FROM {table} ORDER BY id"), Variables::new(), 1)?;
            self.finish(index)?.reconcile(&mut actual).await?;
        }
        Ok(())
    }
}
struct Batch<'a> { loader: Option<&'a Loader>, table: &'static str, rows: Vec<Value>, bytes: usize }
impl<'a> Batch<'a> {
    fn new(loader: Option<&'a Loader>, table: &'static str) -> Self { Self { loader, table, rows: Vec::new(), bytes: 0 } }
    async fn emit(&mut self, row: Value) -> Result<(), ModelError> {
        if self.loader.is_none() { return Ok(()); }
        let bytes = serde_json::to_vec(&row).map_err(ModelError::codec)?.len();
        if bytes > BATCH_BYTES { return Err(ModelError::Limit { owner: "native-search-write", limit: "value bytes", observed: bytes, bound: BATCH_BYTES }); }
        if self.rows.len() >= BATCH_ROWS || self.bytes.saturating_add(bytes) > BATCH_BYTES { self.flush().await?; }
        self.bytes += bytes;
        self.rows.push(row);
        Ok(())
    }
    async fn flush(&mut self) -> Result<(), ModelError> {
        if self.rows.is_empty() { return Ok(()); }
        let mut bindings = Variables::new();
        bindings.insert("rows", std::mem::take(&mut self.rows));
        let loader = self.loader.ok_or(ModelError::Schema("derived write owner"))?;
        loader.client().query(format!("INSERT {}IGNORE INTO {} $rows RETURN NONE", if self.table.ends_with("occurs") { "RELATION " } else { "" }, self.table))
            .bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        self.bytes = 0;
        Ok(())
    }
}
fn scope<R: Record>(field: &str, value: impl serde::Serialize) -> Result<Variables, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("scope", format!("{}|{field}|{}", R::NAME, scope_string(&crate_json(value)?)));
    Ok(bindings)
}
fn decode<T: serde::de::DeserializeOwned + Serialize + 'static>(value: Value) -> Result<T, ModelError> {
    SerdeWrapper::<T>::from_value(value).map(|v| v.0).map_err(ModelError::codec)
}
#[derive(Deserialize, Serialize)]
struct DocumentInput { digest: ContentHash, text: String }
#[derive(Deserialize, Serialize)]
struct VectorLink { input: ContentHash, specification: ContentHash }

async fn lower(reader: &NativeReader, expected: &mut Expected, loader: Option<&Loader>) -> Result<(), ModelError> {
    // Project related text only. Ordered adjacent deduplication prepares each shared document
    // once while retaining at most one prior text; no corpus inventory is collected in Rust.
    for family in [Family::ApiOptions, Family::DocumentationDeployment, Family::Scenario, Family::Source] {
        let mut bind = Variables::new(); bind.insert("family", family as i16);
        let mut documents = reader.query_stream("SELECT body.digest,body.text FROM entity WHERE semantic_type='retrieval_fragments' AND scope_corpus IN (SELECT VALUE scope_corpus FROM entity WHERE semantic_type='retrieval_units' AND body.family=$family) ORDER BY body.digest,body.text", bind, 1)?;
        let mut previous: Option<DocumentInput> = None;
        while let Some(row) = documents.next().await? {
            let Value::Object(row) = row else { return Err(ModelError::Schema("retrieval document projection")); };
            let row: DocumentInput = decode(row.get("body").ok_or(ModelError::Schema("retrieval document projection"))?.clone())?;
            if previous.as_ref().is_some_and(|prior| prior.digest == row.digest && prior.text == row.text) { continue; }
            if ContentHash::of(row.text.as_bytes()) != row.digest { return Err(ModelError::Conflict("retrieval fragment text digest")); }
            let mut value = Object::new(); value.insert("id", RecordId::new(table(family), row.digest.hex()));
            value.insert("text", row.text.clone()); value.insert("digest", crate_json(row.digest)?);
            value.insert("scope_digest", scope_string(value.get("digest").expect("document digest")));
            expected.emit(table(family), Value::Object(value))?;
            previous = Some(row);
        }
    }
    // Retained numeric vectors are decoded at their shared grain, never during occurrence fanout.
    let mut uses = reader.record_stream::<RetrievalEmbeddingUse>("body.availability=0 AND id IN (SELECT VALUE in FROM participant WHERE field='fragment' AND out IN (SELECT VALUE id FROM entity WHERE semantic_type='retrieval_fragments' AND scope_corpus IN (SELECT VALUE scope_corpus FROM entity WHERE semantic_type='retrieval_units')))", Variables::new(), "body.specification,body.input,semantic_key")?;
    let mut last_spec = None;
    let mut configuration = None;
    let mut last_value: Option<(RecordId, ContentHash, Vec<u8>)> = None;
    while let Some(consumed) = uses.next().await? {
        if last_spec != Some(consumed.specification) {
            let specs = reader.records::<EmbeddingSpec>(RecordSelection::Keys(vec![*consumed.specification.bytes()])).await?;
            let spec = specs.first().filter(|_| specs.len()==1).ok_or(ModelError::Schema("retrieval vector specification"))?;
            configuration = Some(spec.configuration()?); last_spec = Some(consumed.specification);
        }
        let spec = configuration.as_ref().expect("vector specification");
        let bytes = consumed.bytes.as_ref().ok_or(ModelError::Schema("retrieval winning bytes"))?.0.as_slice();
        let digest = consumed.value_digest.ok_or(ModelError::Schema("retrieval value digest"))?;
        let id = RecordId::new("vector", format!("{}_{}", spec.hash().hex(), consumed.input.hex()));
        if let Some((prior, prior_digest, prior_bytes)) = &last_value {
            if prior == &id {
                if *prior_digest != digest || prior_bytes != bytes { return Err(ModelError::Conflict("retrieval shared vector winner")); }
                continue;
            }
        }
        let vector = value::decode_vector(bytes, spec.dimensions).map_err(ModelError::Invalid)?;
        lctx_model::domain::embedding::check_vector(&vector, spec.dimensions).map_err(ModelError::Invalid)?;
        if value::value_digest(&vector) != digest { return Err(ModelError::Conflict("retrieval winning vector")); }
        let mut row = Object::new(); row.insert("id", id.clone());
        row.insert("specification", crate_json(spec.hash())?); row.insert("input", crate_json(consumed.input)?);
        row.insert("digest", crate_json(digest)?); row.insert("bytes", Bytes::from(bytes.to_vec())); row.insert("embedding", vector);
        for field in ["specification", "input"] { row.insert(format!("scope_{field}"), scope_string(row.get(field).expect("vector field"))); }
        expected.emit("vector", Value::Object(row))?;
        last_value = Some((id, digest, bytes.to_vec()));
    }
    // External deduplication checks conflicting shared IDs before writing them, and endpoints
    // precede their ENFORCED witness relations. Audit does exactly the same preparation, no writes.
    for (index, table) in TABLES[..5].iter().enumerate() {
        let rows = expected.finish(index)?;
        if loader.is_some() {
            let mut batch = Batch::new(loader, table);
            while let Some(row) = rows.next()? { batch.emit(row).await?; }
            batch.flush().await?; rows.rewind()?;
        }
    }
    let mut lexical = Batch::new(loader, "lex_occurs");
    let mut vectors = Batch::new(loader, "vec_occurs");
    let mut units = reader.record_stream::<Unit>("true", Variables::new(), "semantic_key")?;
    while let Some(unit) = units.next().await? {
        let mut fragments = reader.record_stream::<Fragment>("scope_keys CONTAINS $scope", scope::<Fragment>("corpus", unit.corpus)?, "semantic_key")?;
        while let Some(fragment) = fragments.next().await? {
            // Prepare the fragment's lightweight vector links once, spilling arbitrary fanout.
            // Rewinding this attempt-owned file never hydrates winning numeric values again.
            let mut link_rows = SortedRows::new()?;
            let mut source_links = reader.query_stream("SELECT body.input AS input, (SELECT VALUE out.body.service_hash FROM participant WHERE in=$parent.id AND field='specification')[0] AS specification, semantic_key FROM assertion WHERE semantic_type='retrieval_embedding_uses' AND body.availability=0 AND scope_keys CONTAINS $scope ORDER BY semantic_key", scope::<RetrievalEmbeddingUse>("fragment", fragment.id())?, 1)?;
            while let Some(link) = source_links.next().await? {
                let decoded: VectorLink = decode(link.clone())?;
                let Value::Object(mut link) = link else { return Err(ModelError::Schema("retrieval vector link")); };
                link.remove("semantic_key"); // selected only for source ordering, not shared link identity
                link.insert("id", RecordId::new("vector", format!("{}_{}", decoded.specification.hex(), decoded.input.hex())));
                link_rows.push(Value::Object(link))?;
            }
            let mut links = link_rows.finish()?;
            let mut members = reader.record_stream::<Subject>("body.member IS NOT NONE AND body.member IS NOT NULL AND id IN (SELECT VALUE out FROM participant WHERE field='subject' AND in IN (SELECT VALUE id FROM assertion WHERE semantic_type='retrieval_unit_subjects' AND scope_keys CONTAINS $scope))", scope::<UnitSubject>("unit", unit.id())?, "semantic_key")?;
            let mut member_row = members.next().await?;
            loop {
                let member = match member_row.as_ref() { Some(Subject::Member { member }) => Some(*member), None => None, _ => return Err(ModelError::Schema("retrieval member witness")) };
                let mut anchors = reader.record_stream::<OriginalAnchor>("scope_keys CONTAINS $scope", scope::<OriginalAnchor>("unit", unit.id())?, "semantic_key")?;
                let mut anchor_row = anchors.next().await?;
                loop {
                    let anchor = anchor_row.as_ref().map(Record::id);
                    let mut identity = KeySink::new("native-search-occurrence/v1");
                    unit.id().encode(&mut identity); fragment.id().encode(&mut identity); member.encode(&mut identity); anchor.encode(&mut identity); unit.context.encode(&mut identity); unit.family.encode(&mut identity);
                    let key = identity.finish().hex();
                    let out = member.map(|m| target_id(Target::Entity(EntityId::of(m)))).unwrap_or_else(|| target_id(Target::Entity(EntityId::of(unit.id()))));
                    let row = occurrence("lex_occurs", &key, RecordId::new(table(unit.family), fragment.digest.hex()), out.clone(), &unit, &fragment, member, anchor)?;
                    expected.emit("lex_occurs", row.clone())?; lexical.emit(row).await?;
                    // Only link fields cross this boundary. No winning vector/configuration is
                    // hydrated or decoded again for a member/anchor/context witness.
                    links.rewind()?;
                    while let Some(link) = links.next()? {
                        let link: VectorLink = decode(link)?;
                        let vector_id = RecordId::new("vector", format!("{}_{}", link.specification.hex(), link.input.hex()));
                        let vector_key = format!("{}_{}_{}", key, link.specification.hex(), link.input.hex());
                        let row = occurrence("vec_occurs", &vector_key, vector_id, out.clone(), &unit, &fragment, member, anchor)?;
                        expected.emit("vec_occurs", row.clone())?; vectors.emit(row).await?;
                    }
                    if anchor_row.is_none() { break; }
                    anchor_row = anchors.next().await?;
                    if anchor_row.is_none() { break; }
                }
                if member_row.is_none() { break; }
                member_row = members.next().await?;
                if member_row.is_none() { break; }
            }
        }
    }
    lexical.flush().await?; vectors.flush().await?;
    Ok(())
}
pub async fn materialize_search(loader: &Loader) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let mut expected = Expected::new()?;
    lower(&reader, &mut expected, Some(loader)).await?;
    expected.reconcile(&reader).await
}
pub async fn reconcile_search(loader: &Loader) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let mut expected = Expected::new()?;
    lower(&reader, &mut expected, None).await?;
    expected.reconcile(&reader).await
}
fn crate_json<T: serde::Serialize>(value: T) -> Result<Value, ModelError> {
    lctx_surrealdb::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}
#[allow(
    clippy::too_many_arguments,
    reason = "The lowering keeps physical endpoints and distinct semantic occurrence witnesses explicit"
)]
fn occurrence(
    table: &str,
    key: &str,
    input: RecordId,
    out: RecordId,
    unit: &Unit,
    fragment: &Fragment,
    member: Option<Id<catalog::CatalogMember>>,
    anchor: Option<Id<OriginalAnchor>>,
) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", RecordId::new(table, key));
    row.insert("in", input);
    row.insert("out", out.clone());
    row.insert("family", unit.family as i16);
    row.insert("unit", crate_json(unit.id())?);
    row.insert("fragment", crate_json(fragment.id())?);
    row.insert("context", crate_json(unit.context)?);
    row.insert("member", crate_json(member)?);
    row.insert("anchor", crate_json(anchor)?);
    row.insert("input", crate_json(unit.input)?);
    row.insert("eligible", true);
    row.insert(
        "occurrence_key",
        format!(
            "{}|{:02}|{}|{}|{}|{}",
            member
                .map(|id| format!("0{}", id.hex()))
                .unwrap_or_else(|| format!("1{}", unit.id().hex())),
            unit.family as i16,
            unit.id().hex(),
            fragment.id().hex(),
            unit.context.hex(),
            anchor
                .map(|id| format!("1{}", id.hex()))
                .unwrap_or_else(|| "0".into())
        ),
    );
    for field in ["input", "member", "context"] {
        row.insert(format!("scope_{field}"), scope_string(row.get(field).expect("occurrence field")));
    }
    Ok(Value::Object(row))
}
