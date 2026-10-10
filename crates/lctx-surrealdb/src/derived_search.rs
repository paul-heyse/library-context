//! One terminally checked lowering for private construction and read-only cold reconciliation.
use crate::surrealdb::types::{Object, RecordId, Value, Variables};
use crate::{
    Loader, NativeReader, RecordSelection,
    ordered_rows::{OrderedRows, SortedRows},
    prepared::scope_string,
    reader::target_id,
};
use lctx_model::domain::{
    embedding::{
        EmbeddingSpec,
        projection::{ProjectedValue, ProjectionDefinition},
        value::FullValue,
    },
    graph::{EntityId, Target},
    retrieval::{
        ContentPart, Family, Origin, OriginalAnchor, PartPurpose, SearchWindow, Subject, Unit,
        WindowBinding, WindowPart, WindowSourceMap, consumption::RetrievalEmbeddingUse,
    },
    *,
};
use std::collections::{BTreeMap, BTreeSet};
const TABLES: [&str; 7] = [
    "search_api_options",
    "search_documentation_deployment",
    "search_scenario",
    "search_source",
    "vector",
    "lex_occurs",
    "vec_occurs",
];
const BATCH_ROWS: usize = 128;
const BATCH_BYTES: usize = 1024 * 1024;
fn table(family: Family) -> &'static str {
    match family {
        Family::ApiOptions => TABLES[0],
        Family::DocumentationDeployment => TABLES[1],
        Family::Scenario => TABLES[2],
        Family::Source => TABLES[3],
    }
}
fn reader(loader: &Loader) -> Result<NativeReader<()>, ModelError> {
    Ok(loader.reader())
}

/// Resolve immutable view payloads once per request, before evaluating candidate rows.
pub fn prepare_selected_payloads<Context>(
    reader: &NativeReader<Context>,
    preparation: &mut Vec<String>,
) -> String {
    if reader.view_bindings().get("lctx_views").is_none() {
        return "true".into();
    }
    if !preparation
        .iter()
        .any(|statement| statement.starts_with("LET $lctx_selected_payloads ="))
    {
        preparation.push("LET $lctx_selected_nodes = SELECT VALUE node FROM compiler_view_member WITH INDEX view_nodes WHERE view IN $lctx_views".into());
        preparation.push("LET $lctx_selected_payloads = array::distinct(array::concat($lctx_selected_nodes,(SELECT VALUE target FROM compiler_alias WITH INDEX alias_source WHERE source IN $lctx_selected_nodes)))".into());
    }
    "array::len($this.dependencies)>0 AND $this.dependencies ALLINSIDE $lctx_selected_payloads"
        .into()
}
/// Point sources selected through indexed payload candidates, including unexpected actual rows.
pub struct ExactOccurrences {
    pub lexical: String,
    pub vector: String,
    pub documents: String,
    pub vectors: String,
    pub payloads: Option<&'static str>,
}
pub fn prepare_exact_occurrences<Context>(
    reader: &NativeReader<Context>,
    preparation: &mut Vec<String>,
) -> ExactOccurrences {
    let predicate = prepare_selected_payloads(reader, preparation);
    if predicate == "true" {
        return ExactOccurrences {
            lexical: "lex_occurs".into(),
            vector: "vec_occurs".into(),
            documents: "(SELECT VALUE in FROM lex_occurs)".into(),
            vectors: "(SELECT VALUE in FROM vec_occurs)".into(),
            payloads: None,
        };
    }
    for (table, name) in [("lex_occurs", "lex"), ("vec_occurs", "vec")] {
        preparation.push(format!("LET $lctx_selected_{name}_occurrences = SELECT VALUE id FROM {table} WITH INDEX exact_unit_payload WHERE unit_payload IN $lctx_selected_payloads AND ({predicate})"));
    }
    preparation.push("LET $lctx_selected_documents = array::distinct(SELECT VALUE in FROM $lctx_selected_lex_occurrences)".into());
    preparation.push("LET $lctx_selected_vectors = array::distinct(SELECT VALUE in FROM $lctx_selected_vec_occurrences)".into());
    ExactOccurrences {
        lexical: "$lctx_selected_lex_occurrences".into(),
        vector: "$lctx_selected_vec_occurrences".into(),
        documents: "$lctx_selected_documents".into(),
        vectors: "$lctx_selected_vectors".into(),
        payloads: Some("$lctx_selected_payloads"),
    }
}
fn typed_payload<R: Record>(row: &R) -> Result<RecordId, ModelError> {
    let relation = Relation::of::<R>();
    let batch = R::encode(std::slice::from_ref(row))?;
    match crate::adapter::select(R::NAME)?
        .graph(&batch)?
        .into_iter()
        .next()
        .flatten()
    {
        Some(crate::adapter::GraphRow::Entity(row)) => crate::loader::entity_payload_id(&row),
        Some(crate::adapter::GraphRow::Assertion(row)) => crate::loader::assertion_payload_id(&row),
        None => {
            let body = crate::codec::batch_bodies(&relation, &batch)?
                .into_iter()
                .next()
                .ok_or(ModelError::Schema("derived dependency body"))?;
            crate::loader::payload_id(
                "compiler_record",
                R::NAME,
                row.id().bytes(),
                ContentHash::of(&serde_json::to_vec(&body).map_err(ModelError::codec)?),
            )
        }
    }
}
struct Expected {
    pending: Vec<Option<SortedRows>>,
    ordered: Vec<Option<OrderedRows>>,
}
impl Expected {
    fn new() -> Result<Self, ModelError> {
        Ok(Self {
            pending: (0..7)
                .map(|_| SortedRows::new().map(Some))
                .collect::<Result<_, _>>()?,
            ordered: (0..7).map(|_| None).collect(),
        })
    }
    fn emit(&mut self, table: &str, row: Value) -> Result<(), ModelError> {
        let index = TABLES
            .iter()
            .position(|candidate| *candidate == table)
            .ok_or(ModelError::Schema("derived table"))?;
        self.pending[index]
            .as_mut()
            .ok_or(ModelError::Schema("finished derived family"))?
            .push(row)
    }
    fn finish(&mut self, index: usize) -> Result<&mut OrderedRows, ModelError> {
        if self.ordered[index].is_none() {
            self.ordered[index] = Some(
                self.pending[index]
                    .take()
                    .ok_or(ModelError::Schema("derived ordering"))?
                    .finish()?,
            );
        }
        Ok(self.ordered[index].as_mut().expect("ordered family"))
    }
    async fn reconcile(&mut self, reader: &NativeReader<()>) -> Result<(), ModelError> {
        if reader.view_bindings().get("lctx_views").is_some() {
            let mut preparation = Vec::new();
            let selected = prepare_exact_occurrences(reader, &mut preparation);
            let query = crate::prepared::PreparedQuery::new(
                Variables::new(),
                preparation,
                vec![format!(
                    "SELECT * FROM array::distinct(array::concat({},{},{},{})) ORDER BY id",
                    selected.lexical, selected.vector, selected.documents, selected.vectors
                )],
            )?;
            let mut actual = reader.stream_prepared(query)?;
            let result = async {
                let mut indices = (0..TABLES.len()).collect::<Vec<_>>();
                indices.sort_by_key(|index| TABLES[*index]);
                for index in indices {
                    while let Some(expected) = self.finish(index)?.next_row()? {
                        let Some(row) = actual.next().await? else {
                            return Err(ModelError::Serving(
                                lctx_model::domain::serving::FailureKind::Corrupt,
                            ));
                        };
                        if serde_json::to_vec(&expected).map_err(ModelError::codec)?
                            != serde_json::to_vec(&row).map_err(ModelError::codec)?
                        {
                            return Err(ModelError::Serving(
                                lctx_model::domain::serving::FailureKind::Corrupt,
                            ));
                        }
                    }
                }
                if actual.next().await?.is_some() {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Corrupt,
                    ));
                }
                Ok(())
            }
            .await;
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "derived actual rows transport drain",
                actual.drain_transport().await,
            );
            return lctx_model::domain::completion::complete(result, completion);
        }
        for (index, table) in TABLES.iter().enumerate() {
            let mut actual = reader.query_stream(
                format!(
                    "SELECT * FROM {table} WHERE {} ORDER BY id",
                    match *table {
                        "lex_occurs" | "vec_occurs" => "true",
                        "vector" => "id IN (SELECT VALUE in FROM vec_occurs)",
                        _ => "id IN (SELECT VALUE in FROM lex_occurs)",
                    }
                ),
                reader.view_bindings(),
                1,
            )?;
            self.finish(index)?.reconcile(&mut actual).await?;
        }
        Ok(())
    }
}
struct Batch<'a> {
    loader: Option<&'a Loader>,
    table: &'static str,
    rows: Vec<Value>,
    bytes: usize,
}
impl<'a> Batch<'a> {
    fn new(loader: Option<&'a Loader>, table: &'static str) -> Self {
        Self {
            loader,
            table,
            rows: Vec::new(),
            bytes: 0,
        }
    }
    async fn emit(&mut self, row: Value) -> Result<(), ModelError> {
        if self.loader.is_none() {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&row).map_err(ModelError::codec)?.len();
        if bytes > BATCH_BYTES {
            return Err(ModelError::Limit {
                owner: "native-search-write",
                limit: "value bytes",
                observed: bytes,
                bound: BATCH_BYTES,
            });
        }
        if self.rows.len() >= BATCH_ROWS || self.bytes.saturating_add(bytes) > BATCH_BYTES {
            self.flush().await?;
        }
        self.bytes += bytes;
        self.rows.push(row);
        Ok(())
    }
    async fn flush(&mut self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            return Ok(());
        }
        let loader = self
            .loader
            .ok_or(ModelError::Schema("derived write owner"))?;
        loader
            .insert(
                self.table,
                std::mem::take(&mut self.rows),
                self.table.ends_with("occurs"),
            )
            .await?;
        self.bytes = 0;
        Ok(())
    }
}
const FRONTIER_ROWS: usize = 64;
type Index<R> = BTreeMap<Id<R>, R>;
fn need<R: Record>(index: &Index<R>, id: Id<R>) -> Result<&R, ModelError> {
    index
        .get(&id)
        .ok_or(ModelError::Schema("canonical search companion"))
}
async fn keyed<R: Record + serde::de::DeserializeOwned>(
    reader: &NativeReader<()>,
    ids: impl IntoIterator<Item = Id<R>>,
) -> Result<Index<R>, ModelError> {
    let ids = ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    for chunk in ids.chunks(FRONTIER_ROWS) {
        for row in reader
            .records::<R>(RecordSelection::Keys(
                chunk.iter().map(|id| *id.bytes()).collect(),
            ))
            .await?
        {
            if !chunk.contains(&row.id()) || result.insert(row.id(), row).is_some() {
                return Err(ModelError::Conflict("canonical companion identity"));
            }
        }
    }
    if result.len() != ids.len() {
        return Err(ModelError::Schema("canonical search companion"));
    }
    Ok(result)
}
async fn scoped<R: Record + serde::de::DeserializeOwned, T: serde::Serialize>(
    reader: &NativeReader<()>,
    field: &str,
    ids: impl IntoIterator<Item = T>,
) -> Result<Index<R>, ModelError> {
    let values = ids
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ModelError::codec)?;
    let rows = reader
        .records::<R>(RecordSelection::Scope {
            field: field.into(),
            values,
        })
        .await?;
    let mut result = BTreeMap::new();
    for row in rows {
        if result.insert(row.id(), row).is_some() {
            return Err(ModelError::Conflict("canonical scoped companion"));
        }
    }
    Ok(result)
}
async fn fill<R: Record + serde::de::DeserializeOwned>(
    reader: &NativeReader<()>,
    index: &mut Index<R>,
    ids: impl IntoIterator<Item = Id<R>>,
) -> Result<(), ModelError> {
    let missing = ids
        .into_iter()
        .filter(|id| !index.contains_key(id))
        .collect::<BTreeSet<_>>();
    index.extend(keyed(reader, missing).await?);
    Ok(())
}
struct Basic {
    windows: Index<SearchWindow>,
    units: Index<Unit>,
    links: Index<WindowPart>,
    parts: Index<ContentPart>,
}
impl Basic {
    async fn load(
        reader: &NativeReader<()>,
        windows: Index<SearchWindow>,
    ) -> Result<Self, ModelError> {
        let units = keyed(reader, windows.values().map(|w| w.unit)).await?;
        let links = scoped::<WindowPart, _>(reader, "window", windows.keys().copied()).await?;
        let parts = keyed(reader, links.values().map(|link| link.part)).await?;
        for link in links.values() {
            if need(&windows, link.window)?.unit != need(&parts, link.part)?.unit {
                return Err(ModelError::Conflict("window part unit"));
            }
        }
        Ok(Self {
            windows,
            units,
            links,
            parts,
        })
    }
    fn primary(&self, window: Id<SearchWindow>) -> impl Iterator<Item = &ContentPart> {
        self.links
            .values()
            .filter(move |link| link.window == window)
            .filter_map(|link| self.parts.get(&link.part))
            .filter(|part| part.purpose == PartPurpose::Primary)
    }
}
#[derive(serde::Serialize)]
struct LoweredProjection {
    dependencies: Vec<RecordId>,
    id: Id<ProjectedValue>,
    value: Id<FullValue>,
    encoder: Id<EmbeddingSpec>,
    encoder_hash: ContentHash,
    policy: Id<ProjectionDefinition>,
    input: ContentHash,
    tokens: i64,
    embedding: Vec<f32>,
}
fn vector_id(projection: &LoweredProjection, unit: &Unit) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "vector",
        ContentHash::of(
            &serde_json::to_vec(&("native-vector/v2", projection, unit.input, unit.family))
                .map_err(ModelError::codec)?,
        )
        .hex(),
    ))
}
fn cohort_row(p: &LoweredProjection, unit: &Unit) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", vector_id(p, unit)?);
    row.insert("dependencies", p.dependencies.clone());
    row.insert("encoder_hash", p.encoder_hash.hex());
    row.insert("policy_key", p.policy.hex());
    row.insert("library_input", scope_string(&crate_json(unit.input)?));
    row.insert("family", unit.family as i16);
    row.insert("full_key", p.value.hex());
    row.insert("projection_key", p.id.hex());
    row.insert("embedding", p.embedding.clone());
    Ok(Value::Object(row))
}
async fn lower_vectors(
    reader: &NativeReader<()>,
    expected: &mut Expected,
) -> Result<(), ModelError> {
    let mut uses = reader.record_stream::<RetrievalEmbeddingUse>(
        "body.availability=0",
        Variables::new(),
        "body.projection,body.window,semantic_key",
    )?;
    let mut encoders = Index::new();
    let mut policies = Index::new();
    let mut last: Option<LoweredProjection> = None;
    loop {
        let mut batch = Vec::new();
        while batch.len() < FRONTIER_ROWS {
            let Some(row) = uses.next().await? else { break };
            batch.push(row);
        }
        if batch.is_empty() {
            break;
        }
        let windows = keyed(reader, batch.iter().map(|row| row.window)).await?;
        let basic = Basic::load(reader, windows).await?;
        let ids = batch
            .iter()
            .map(|row| {
                row.projection
                    .ok_or(ModelError::Schema("available projection"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let new = keyed::<ProjectedValue>(
            reader,
            ids.into_iter()
                .filter(|id| last.as_ref().is_none_or(|last| last.id != *id)),
        )
        .await?;
        let full = keyed(reader, new.values().map(|p| p.value)).await?;
        fill(reader, &mut encoders, full.values().map(|v| v.encoder)).await?;
        fill(reader, &mut policies, new.values().map(|p| p.definition)).await?;
        let mut prepared = BTreeMap::new();
        for p in new.values() {
            let full = need(&full, p.value)?;
            let encoder = need(&encoders, full.encoder)?;
            let policy = need(&policies, p.definition)?;
            full.verify_encoder(encoder)?;
            p.verify(full, policy)?;
            if full.dimensions != 4096 || p.dimensions != 1024 {
                return Err(ModelError::Schema("native full4096/projection1024"));
            }
            prepared.insert(
                p.id(),
                LoweredProjection {
                    dependencies: vec![
                        typed_payload(p)?,
                        typed_payload(full)?,
                        typed_payload(encoder)?,
                        typed_payload(policy)?,
                    ],
                    id: p.id(),
                    value: full.id(),
                    encoder: full.encoder,
                    encoder_hash: encoder.service_hash,
                    policy: policy.id(),
                    input: full.input,
                    tokens: full.tokens,
                    embedding: p.values()?,
                },
            );
        }
        for consumed in batch {
            let id = consumed
                .projection
                .ok_or(ModelError::Schema("available projection"))?;
            if last.as_ref().is_none_or(|old| old.id != id) {
                if last.as_ref().is_some_and(|old| old.id > id) {
                    return Err(ModelError::Conflict("canonical projection order"));
                }
                last = Some(
                    prepared
                        .remove(&id)
                        .ok_or(ModelError::Schema("prepared canonical projection"))?,
                );
            }
            let p = last.as_ref().expect("prepared projection");
            let window = need(&basic.windows, consumed.window)?;
            let unit = need(&basic.units, window.unit)?;
            if consumed.value != Some(p.value)
                || consumed.specification != p.encoder
                || consumed.input != p.input
                || consumed.admitted_tokens != Some(p.tokens)
                || window.encoded_digest != p.input
            {
                return Err(ModelError::Conflict(
                    "retrieval canonical winner references",
                ));
            }
            if basic.primary(window.id()).next().is_some() {
                expected.emit("vector", cohort_row(p, unit)?)?;
            }
        }
    }
    Ok(())
}
async fn window_batch(
    stream: &mut crate::reader::CanonicalRecords<SearchWindow>,
) -> Result<Index<SearchWindow>, ModelError> {
    let mut windows = BTreeMap::new();
    while windows.len() < FRONTIER_ROWS {
        let Some(window) = stream.next().await? else {
            break;
        };
        windows.insert(window.id(), window);
    }
    Ok(windows)
}
struct Companions {
    bindings: Index<WindowBinding>,
    subjects: Index<Subject>,
    origins: Index<Origin>,
    options: Index<catalog::CatalogOption>,
    members: Index<catalog::CatalogMember>,
    artifacts: Index<source::SourceArtifact>,
    names: BTreeMap<Id<catalog::CatalogOption>, OptionName>,
    original_rows: Index<OriginalAnchor>,
    maps: Index<WindowSourceMap>,
    vectors: BTreeMap<(String, String, i16), (RecordId, Vec<RecordId>)>,
    anchors: BTreeMap<(Id<SearchWindow>, Id<ContentPart>), Id<OriginalAnchor>>,
    uses: Index<RetrievalEmbeddingUse>,
}
impl Companions {
    async fn load(reader: &NativeReader<()>, b: &Basic) -> Result<Self, ModelError> {
        let bindings =
            scoped::<WindowBinding, _>(reader, "window", b.windows.keys().copied()).await?;
        let subjects = keyed(reader, bindings.values().map(|r| r.subject)).await?;
        let origins = keyed(reader, b.units.values().map(|u| u.origin)).await?;
        let options = keyed(
            reader,
            subjects.values().filter_map(|s| {
                if let Subject::Option { option } = s {
                    Some(*option)
                } else {
                    None
                }
            }),
        )
        .await?;
        let mut member_ids = subjects
            .values()
            .filter_map(|s| {
                if let Subject::Member { member } = s {
                    Some(*member)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        member_ids.extend(options.values().map(|o| o.member));
        member_ids.extend(origins.values().filter_map(|o| {
            if let Origin::Definition { member, .. } = o {
                Some(*member)
            } else {
                None
            }
        }));
        let members = keyed(reader, member_ids).await?;
        let artifacts = keyed(
            reader,
            subjects.values().filter_map(|s| {
                if let Subject::Source { artifact } = s {
                    Some(*artifact)
                } else {
                    None
                }
            }),
        )
        .await?;
        let names = option_names(reader, &options).await?;
        let maps =
            scoped::<WindowSourceMap, _>(reader, "window", b.windows.keys().copied()).await?;
        let originals = maps
            .values()
            .filter(|map| {
                map.part.is_some_and(|part| {
                    b.parts
                        .get(&part)
                        .is_some_and(|p| p.purpose == PartPurpose::Primary)
                })
            })
            .filter_map(|map| map.original)
            .collect::<BTreeSet<_>>();
        let mut vars = Variables::new();
        vars.insert("originals", crate_json(&originals)?);
        let scopes = b
            .units
            .keys()
            .map(|id| {
                Ok(format!(
                    "{}|unit|{}",
                    OriginalAnchor::NAME,
                    scope_string(&crate_json(id)?)
                ))
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        vars.insert("scopes", scopes);
        let mut rows = reader.record_stream::<OriginalAnchor>(
            "scope_keys CONTAINSANY $scopes AND body.original IN $originals",
            vars,
            "semantic_key",
        )?;
        let mut anchors = BTreeMap::new();
        let mut original_rows = Index::new();
        while let Some(anchor) = rows.next().await? {
            original_rows.insert(anchor.id(), anchor.clone());
            anchors
                .entry((anchor.unit, anchor.original))
                .and_modify(|id: &mut Id<OriginalAnchor>| *id = (*id).min(anchor.id()))
                .or_insert(anchor.id());
        }
        // Map each actual part/window to a supporting anchor, without a unit-anchor cross product.
        let mut supports = BTreeMap::new();
        for map in maps.values() {
            if let (Some(part), Some(original)) = (map.part, map.original) {
                let unit = need(&b.parts, part)?.unit;
                if let Some(anchor) = anchors.get(&(unit, original)) {
                    supports
                        .entry((map.window, part))
                        .and_modify(|id: &mut Id<OriginalAnchor>| *id = (*id).min(*anchor))
                        .or_insert(*anchor);
                }
            }
        }
        let uses: Index<RetrievalEmbeddingUse> =
            scoped(reader, "window", b.windows.keys().copied()).await?;
        let projections = uses
            .values()
            .filter_map(|row| row.projection.map(|id| id.hex()))
            .collect::<BTreeSet<_>>();
        let mut vars = reader.view_bindings();
        vars.insert("projections", projections.into_iter().collect::<Vec<_>>());
        let mut preparation = Vec::new();
        let selected = prepare_selected_payloads(reader, &mut preparation);
        let values:Vec<Object>=reader.query_prepared_native(crate::prepared::PreparedQuery::new(vars,preparation,vec![format!("SELECT id,projection_key,library_input,family,dependencies FROM vector WITH INDEX cohort WHERE projection_key IN $projections AND ({selected})")])?).await?;
        let mut vectors = BTreeMap::new();
        for row in values {
            let (
                Some(Value::RecordId(id)),
                Some(Value::String(projection)),
                Some(Value::String(input)),
                Some(Value::Number(surrealdb::types::Number::Int(family))),
                Some(Value::Array(dependencies)),
            ) = (
                row.get("id"),
                row.get("projection_key"),
                row.get("library_input"),
                row.get("family"),
                row.get("dependencies"),
            )
            else {
                return Err(ModelError::Schema("exact vector cohort"));
            };
            let dependencies = dependencies
                .iter()
                .map(|value| {
                    if let Value::RecordId(id) = value {
                        Ok(id.clone())
                    } else {
                        Err(ModelError::Schema("vector dependency"))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            if vectors
                .insert(
                    (projection.clone(), input.clone(), *family as i16),
                    (id.clone(), dependencies),
                )
                .is_some()
            {
                return Err(ModelError::Conflict("competing exact vector cohort"));
            }
        }

        Ok(Self {
            bindings,
            subjects,
            origins,
            options,
            members,
            artifacts,
            names,
            original_rows,
            maps,
            vectors,
            anchors: supports,
            uses,
        })
    }
}
struct OptionName {
    value: String,
    dependencies: Vec<RecordId>,
}
async fn option_names(
    reader: &NativeReader<()>,
    options: &Index<catalog::CatalogOption>,
) -> Result<BTreeMap<Id<catalog::CatalogOption>, OptionName>, ModelError> {
    use catalog::CatalogOptionSubject as Subject;
    use normalized::{
        callables::SignatureSlot,
        entities::{FieldEntity, ParameterEntity, ParameterEntityLink},
    };
    let subjects = keyed(reader, options.values().map(|o| o.subject)).await?;
    let fields = keyed::<FieldEntity>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::Field { field } = s {
                Some(*field)
            } else {
                None
            }
        }),
    )
    .await?;
    let slots = keyed::<SignatureSlot>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::Parameter { slot } = s {
                Some(*slot)
            } else {
                None
            }
        }),
    )
    .await?;
    let entities = keyed::<ParameterEntity>(
        reader,
        subjects.values().filter_map(|s| {
            if let Subject::SourceParameter { parameter } = s {
                Some(*parameter)
            } else {
                None
            }
        }),
    )
    .await?;
    let links =
        scoped::<ParameterEntityLink, _>(reader, "entity", entities.keys().copied()).await?;
    let mut ids = slots.values().map(|s| s.parameter).collect::<BTreeSet<_>>();
    ids.extend(links.values().map(|l| l.parameter));
    ids.extend(entities.values().filter_map(|e| {
        if let ParameterEntity::NativeSlot { parameter, .. } = e {
            Some(*parameter)
        } else {
            None
        }
    }));
    let parameters = keyed::<calls::SignatureParameter>(reader, ids).await?;
    let shapes =
        keyed::<calls::ParameterShape>(reader, parameters.values().map(|p| p.shape)).await?;
    let name = |id| -> Result<String, ModelError> {
        Ok(need(&shapes, need(&parameters, id)?.shape)?
            .name
            .as_ref()
            .map(|s| s.as_str().to_owned())
            .unwrap_or_default())
    };
    let mut result = BTreeMap::new();
    for option in options.values() {
        let mut dependencies = vec![typed_payload(need(&subjects, option.subject)?)?];
        let mut parameter_name = |id| -> Result<String, ModelError> {
            let parameter = need(&parameters, id)?;
            let shape = need(&shapes, parameter.shape)?;
            dependencies.push(typed_payload(parameter)?);
            dependencies.push(typed_payload(shape)?);
            name(id)
        };
        let value = match need(&subjects, option.subject)? {
            Subject::Field { field } => {
                let field = need(&fields, *field)?;
                dependencies.push(typed_payload(field)?);
                field.name.as_str().to_owned()
            }
            Subject::Parameter { slot } => {
                let slot = need(&slots, *slot)?;
                let parameter = slot.parameter;
                let slot_payload = typed_payload(slot)?;
                let value = parameter_name(parameter)?;
                dependencies.push(slot_payload);
                value
            }
            Subject::SourceParameter { parameter } => match need(&entities, *parameter)? {
                ParameterEntity::NativeSlot { parameter: id, .. } => {
                    let value = parameter_name(*id)?;
                    dependencies.push(typed_payload(need(&entities, *parameter)?)?);
                    value
                }
                ParameterEntity::Source { .. } => {
                    let mut names = BTreeSet::new();
                    for link in links.values().filter(|link| link.entity == *parameter) {
                        let n = parameter_name(link.parameter)?;
                        // Dependency accumulation happens after the closure borrow ends.
                        if !n.is_empty() {
                            names.insert(n);
                        }
                    }
                    if names.len() > 1 {
                        return Err(ModelError::Conflict(
                            "source option has competing declared names",
                        ));
                    }
                    names.into_iter().next().unwrap_or_default()
                }
            },
        };
        if let Subject::SourceParameter { parameter } = need(&subjects, option.subject)? {
            dependencies.push(typed_payload(need(&entities, *parameter)?)?);
            for link in links.values().filter(|link| link.entity == *parameter) {
                dependencies.push(typed_payload(link)?);
            }
        }
        dependencies.sort();
        dependencies.dedup();
        result.insert(
            option.id(),
            OptionName {
                value,
                dependencies,
            },
        );
    }
    Ok(result)
}
async fn lower(
    reader: &NativeReader<()>,
    expected: &mut Expected,
    loader: Option<&Loader>,
) -> Result<(), ModelError> {
    lower_vectors(reader, expected).await?;
    let mut windows =
        reader.record_stream::<SearchWindow>("true", Variables::new(), "semantic_key")?;
    loop {
        let batch = window_batch(&mut windows).await?;
        if batch.is_empty() {
            break;
        }
        let b = Basic::load(reader, batch).await?;
        for window in b.windows.values() {
            if b.primary(window.id()).next().is_none() {
                continue;
            }
            let unit = need(&b.units, window.unit)?;
            let mut row = Object::new();
            row.insert("id", RecordId::new(table(unit.family), window.digest.hex()));
            row.insert("text", window.text.as_str().to_owned());
            row.insert("digest", crate_json(window.digest)?);
            row.insert(
                "scope_digest",
                scope_string(row.get("digest").expect("digest")),
            );
            expected.emit(table(unit.family), Value::Object(row))?;
        }
    }
    for (index, table) in TABLES[..5].iter().enumerate() {
        let rows = expected.finish(index)?;
        if loader.is_some() {
            let mut batch = Batch::new(loader, table);
            while let Some(row) = rows.next_row()? {
                batch.emit(row).await?;
            }
            batch.flush().await?;
            rows.rewind()?;
        }
    }
    let mut lexical = Batch::new(loader, "lex_occurs");
    let mut vectors = Batch::new(loader, "vec_occurs");
    let mut windows =
        reader.record_stream::<SearchWindow>("true", Variables::new(), "semantic_key")?;
    loop {
        let batch = window_batch(&mut windows).await?;
        if batch.is_empty() {
            break;
        }
        let b = Basic::load(reader, batch).await?;
        let c = Companions::load(reader, &b).await?;
        for window in b.windows.values() {
            let unit = need(&b.units, window.unit)?;
            for part in b.primary(window.id()) {
                let bindings = c
                    .bindings
                    .values()
                    .filter(|binding| binding.window == window.id() && binding.part == part.id())
                    .collect::<Vec<_>>();
                if bindings.is_empty() {
                    emit_witness(
                        expected,
                        &mut lexical,
                        &mut vectors,
                        unit,
                        window,
                        part,
                        None,
                        &c,
                        &b,
                    )
                    .await?;
                } else {
                    for binding in bindings {
                        emit_witness(
                            expected,
                            &mut lexical,
                            &mut vectors,
                            unit,
                            window,
                            part,
                            Some(binding),
                            &c,
                            &b,
                        )
                        .await?;
                    }
                }
            }
        }
    }
    lexical.flush().await?;
    vectors.flush().await?;
    Ok(())
}
#[allow(
    clippy::too_many_arguments,
    reason = "Canonical lineage and distinct physical writers remain explicit"
)]
async fn emit_witness(
    expected: &mut Expected,
    lexical: &mut Batch<'_>,
    vectors: &mut Batch<'_>,
    unit: &Unit,
    window: &SearchWindow,
    part: &ContentPart,
    binding: Option<&WindowBinding>,
    c: &Companions,
    b: &Basic,
) -> Result<(), ModelError> {
    let mut option_key = String::new();
    let mut source_path = String::new();
    let member = if let Some(binding) = binding {
        match need(&c.subjects, binding.subject)? {
            Subject::Member { member } => Some(*member),
            Subject::Option { option } => {
                option_key = c
                    .names
                    .get(option)
                    .ok_or(ModelError::Schema("option name"))?
                    .value
                    .clone();
                Some(need(&c.options, *option)?.member)
            }
            Subject::Definition { entity } => match need(&c.origins, unit.origin)? {
                Origin::Definition {
                    member,
                    entity: owner,
                } if owner == entity => Some(*member),
                _ => None,
            },
            Subject::Source { artifact } => {
                source_path = need(&c.artifacts, *artifact)?.path.clone();
                None
            }
            _ => None,
        }
    } else {
        None
    };
    let (name, path) = if let Some(id) = member {
        let member = need(&c.members, id)?;
        if member.input != unit.input {
            return Err(ModelError::Conflict("foreign primary member input"));
        }
        (
            member.path.last().cloned().unwrap_or_default(),
            member.name.clone(),
        )
    } else {
        (String::new(), source_path)
    };
    let anchor = c.anchors.get(&(window.id(), part.id())).copied();
    let mut dependencies = vec![
        typed_payload(unit)?,
        typed_payload(window)?,
        typed_payload(part)?,
        typed_payload(need(&c.origins, unit.origin)?)?,
    ];
    for link in b
        .links
        .values()
        .filter(|link| link.window == window.id() && link.part == part.id())
    {
        dependencies.push(typed_payload(link)?);
    }
    if let Some(binding) = binding {
        dependencies.push(typed_payload(binding)?);
        let subject = need(&c.subjects, binding.subject)?;
        dependencies.push(typed_payload(subject)?);
        match subject {
            Subject::Option { option } => {
                dependencies.push(typed_payload(need(&c.options, *option)?)?);
                dependencies.extend(
                    c.names
                        .get(option)
                        .ok_or(ModelError::Schema("option name dependencies"))?
                        .dependencies
                        .clone(),
                );
            }
            Subject::Source { artifact } => {
                dependencies.push(typed_payload(need(&c.artifacts, *artifact)?)?)
            }
            _ => {}
        }
    }
    if let Some(member) = member {
        dependencies.push(typed_payload(need(&c.members, member)?)?);
    }
    if let Some(anchor) = anchor {
        dependencies.push(typed_payload(need(&c.original_rows, anchor)?)?);
        for map in c
            .maps
            .values()
            .filter(|map| map.window == window.id() && map.part == Some(part.id()))
        {
            dependencies.push(typed_payload(map)?);
        }
    }
    dependencies.sort();
    dependencies.dedup();
    let witness = Witness {
        unit,
        window,
        part,
        binding: binding.map(Record::id),
        member,
        anchor,
        name,
        path,
        option_key,
    };
    let mut identity = KeySink::new("native-search-occurrence/v3");
    identity.part(
        b"dependencies",
        &serde_json::to_vec(&dependencies).map_err(ModelError::codec)?,
    );
    identity.part(
        b"names",
        &serde_json::to_vec(&(&witness.name, &witness.path, &witness.option_key))
            .map_err(ModelError::codec)?,
    );
    unit.id().encode(&mut identity);
    window.id().encode(&mut identity);
    part.id().encode(&mut identity);
    witness.binding.encode(&mut identity);
    let key = identity.finish().hex();
    let row = occurrence(
        "lex_occurs",
        &key,
        RecordId::new(table(unit.family), window.digest.hex()),
        &witness,
        &dependencies,
    )?;
    expected.emit("lex_occurs", row.clone())?;
    lexical.emit(row).await?;
    for consumed in c.uses.values().filter(|use_| {
        use_.window == window.id()
            && use_.availability == embedding::analytic::VectorAvailability::Available
    }) {
        let projection = consumed
            .projection
            .ok_or(ModelError::Schema("available projection"))?;
        let (vector, vector_dependencies) = c
            .vectors
            .get(&(
                projection.hex(),
                scope_string(&crate_json(unit.input)?),
                unit.family as i16,
            ))
            .ok_or(ModelError::Schema("selected exact vector cohort"))?;
        let mut vector_dependencies = vector_dependencies.clone();
        vector_dependencies.extend(dependencies.clone());
        vector_dependencies.push(typed_payload(consumed)?);
        vector_dependencies.sort();
        vector_dependencies.dedup();
        let vector_key = ContentHash::of(
            &serde_json::to_vec(&(&key, &vector_dependencies, vector))
                .map_err(ModelError::codec)?,
        )
        .hex();
        let row = occurrence(
            "vec_occurs",
            &vector_key,
            vector.clone(),
            &witness,
            &vector_dependencies,
        )?;
        expected.emit("vec_occurs", row.clone())?;
        vectors.emit(row).await?;
    }
    Ok(())
}
#[derive(Debug)]
struct SearchPhaseFailure {
    phase: &'static str,
    cause: ModelError,
}
impl std::fmt::Display for SearchPhaseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.phase, self.cause)
    }
}
impl std::error::Error for SearchPhaseFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
fn phase<T>(phase: &'static str, result: Result<T, ModelError>) -> Result<T, ModelError> {
    result.map_err(|cause| ModelError::Cause(Box::new(SearchPhaseFailure { phase, cause })))
}
pub async fn materialize_search(loader: &Loader) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let mut expected = Expected::new()?;
    phase(
        "derived search canonical lowering",
        lower(&reader, &mut expected, Some(loader)).await,
    )?;
    phase(
        "derived search actual-row reconciliation",
        expected.reconcile(&reader).await,
    )?;
    phase(
        "frozen lexical statistics construction",
        crate::lexical_stats::materialize(loader).await,
    )
}
pub async fn reconcile_search(loader: &Loader) -> Result<(), ModelError> {
    let reader = reader(loader)?;
    let mut expected = Expected::new()?;
    phase(
        "derived search independent cold lowering",
        lower(&reader, &mut expected, None).await,
    )?;
    phase(
        "derived search cold actual-row reconciliation",
        expected.reconcile(&reader).await,
    )?;
    phase(
        "frozen lexical statistics cold comparison",
        crate::lexical_stats::reconcile(loader).await,
    )
}
fn crate_json<T: serde::Serialize>(value: T) -> Result<Value, ModelError> {
    crate::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}
struct Witness<'a> {
    unit: &'a Unit,
    window: &'a SearchWindow,
    part: &'a ContentPart,
    binding: Option<Id<WindowBinding>>,
    member: Option<Id<catalog::CatalogMember>>,
    anchor: Option<Id<OriginalAnchor>>,
    name: String,
    path: String,
    option_key: String,
}
fn occurrence(
    table: &str,
    key: &str,
    input: RecordId,
    w: &Witness<'_>,
    dependencies: &[RecordId],
) -> Result<Value, ModelError> {
    let out = w
        .member
        .map(|m| target_id(Target::Entity(EntityId::of(m))))
        .unwrap_or_else(|| target_id(Target::Entity(EntityId::of(w.unit.id()))));
    let mut row = Object::new();
    row.insert("id", RecordId::new(table, key));
    row.insert("dependencies", dependencies.to_vec());
    row.insert("unit_payload", typed_payload(w.unit)?);
    row.insert("in", input);
    row.insert("out", out);
    row.insert("family", w.unit.family as i16);
    row.insert("unit", crate_json(w.unit.id())?);
    row.insert(
        "unit_node",
        target_id(Target::Entity(EntityId::of(w.unit.id()))),
    );
    row.insert("window", crate_json(w.window.id())?);
    row.insert("part", crate_json(w.part.id())?);
    row.insert("binding", crate_json(w.binding)?);
    row.insert("context", crate_json(w.unit.context)?);
    row.insert("member", crate_json(w.member)?);
    row.insert("anchor", crate_json(w.anchor)?);
    row.insert("input", crate_json(w.unit.input)?);
    row.insert("eligible", true);
    row.insert("exact_name", w.name.clone());
    row.insert("exact_path", w.path.clone());
    row.insert("exact_option", w.option_key.clone());
    row.insert(
        "occurrence_key",
        format!(
            "{}|{:02}|{}|{}|{}|{}|{}",
            w.member
                .map(|id| format!("0{}", id.hex()))
                .unwrap_or_else(|| format!("1{}", w.unit.id().hex())),
            w.unit.family as i16,
            w.unit.id().hex(),
            w.window.id().hex(),
            w.part.id().hex(),
            w.unit.context.hex(),
            key
        ),
    );
    for field in ["input", "member", "context", "window"] {
        row.insert(
            format!("scope_{field}"),
            scope_string(row.get(field).expect("occurrence field")),
        );
    }
    Ok(Value::Object(row))
}
