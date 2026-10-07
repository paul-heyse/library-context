//! One terminally checked lowering for private construction and read-only cold reconciliation.
use lctx_model::domain::{
    embedding::{EmbeddingSpec, value::FullValue, projection::{ProjectedValue, ProjectionDefinition}},
    graph::{EntityId, Target},
    retrieval::{
        Family, SearchWindow, ContentPart, WindowPart, WindowBinding, WindowSourceMap,
        PartPurpose, Origin, OriginalAnchor, Subject, Unit,
        consumption::RetrievalEmbeddingUse,
    },
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    *,
};
use lctx_surrealdb::surrealdb::types::{
    Object, RecordId, SerdeWrapper, SurrealValue, Value, Variables,
};
use lctx_surrealdb::{
    Loader, NativeReader, RecordSelection,
    ordered_rows::{OrderedRows, SortedRows},
    reader::target_id,
    reconciliation::scope_string,
};
use serde::{Deserialize, Serialize};
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
fn reader(loader: &Loader) -> Result<NativeReader, ModelError> {
    Ok(NativeReader::new(
        loader.shared_client(),
        SnapshotHandle {
            semantic: ContentHash::of(b"private-lowering"),
            realization: ContentHash::of(b"private-lowering"),
            database: DatabaseIdentity {
                namespace: Name::new("private").map_err(ModelError::codec)?,
                database: Name::new("private").map_err(ModelError::codec)?,
            },
        },
    ))
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
    async fn reconcile(&mut self, reader: &NativeReader) -> Result<(), ModelError> {
        for (index, table) in TABLES.iter().enumerate() {
            let mut actual = reader.query_stream(
                format!("SELECT * FROM {table} ORDER BY id"),
                Variables::new(),
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
        let mut bindings = Variables::new();
        bindings.insert("rows", std::mem::take(&mut self.rows));
        let loader = self
            .loader
            .ok_or(ModelError::Schema("derived write owner"))?;
        loader
            .client()
            .query(format!(
                "INSERT {}IGNORE INTO {} $rows RETURN NONE",
                if self.table.ends_with("occurs") {
                    "RELATION "
                } else {
                    ""
                },
                self.table
            ))
            .bind(bindings)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        self.bytes = 0;
        Ok(())
    }
}
fn scope<R: Record>(field: &str, value: impl serde::Serialize) -> Result<Variables, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert(
        "scope",
        format!("{}|{field}|{}", R::NAME, scope_string(&crate_json(value)?)),
    );
    Ok(bindings)
}
fn decode<T: serde::de::DeserializeOwned + Serialize + 'static>(
    value: Value,
) -> Result<T, ModelError> {
    SerdeWrapper::<T>::from_value(value)
        .map(|v| v.0)
        .map_err(ModelError::codec)
}
#[derive(Deserialize, Serialize)]
struct Cohort { input: Id<input::InputRevision>, family: Family }
#[derive(Deserialize, Serialize)]
struct VectorLink { projection: String }
async fn one<R: Record + serde::de::DeserializeOwned>(reader: &NativeReader, id: Id<R>) -> Result<R,ModelError> {
    let mut rows=reader.records::<R>(RecordSelection::Keys(vec![*id.bytes()])).await?;
    if rows.len()!=1 {return Err(ModelError::Schema("canonical search companion"));}
    Ok(rows.remove(0))
}
fn vector_id(projection: &str, unit: &Unit) -> RecordId {
    RecordId::new("vector",format!("{}_{}_{:02}",projection,unit.input.hex(),unit.family as i16))
}
async fn lower(reader: &NativeReader, expected: &mut Expected, loader: Option<&Loader>) -> Result<(), ModelError> {
    // Projection bytes are decoded once at canonical grain. Cohort fanout carries only scalar
    // identity and the 1024 lowering; full4096 remains solely in the canonical entity payload.
    let mut projections=reader.record_stream::<ProjectedValue>("true",Variables::new(),"semantic_key")?;
    while let Some(projection)=projections.next().await? {
        let full=one::<FullValue>(reader,projection.value).await?;
        let policy=one::<ProjectionDefinition>(reader,projection.definition).await?;
        let encoder=one::<EmbeddingSpec>(reader,full.encoder).await?;
        full.verify_encoder(&encoder)?;
        projection.verify(&full,&policy)?;
        if full.dimensions!=4096 || projection.dimensions!=1024 {return Err(ModelError::Schema("native full4096/projection1024"));}
        let embedding=projection.values()?;
        let mut vars=Variables::new();
        vars.insert("projection",target_id(Target::Entity(EntityId::of(projection.id()))));
        let mut cohorts=reader.query_stream("SELECT body.input AS input,body.family AS family FROM entity WHERE semantic_type='retrieval_units' AND id IN (SELECT VALUE out FROM reference WHERE field='unit' AND in IN (SELECT VALUE out FROM participant WHERE field='window' AND in IN (SELECT VALUE in FROM participant WHERE field='projection' AND out=$projection))) GROUP BY input,family ORDER BY input,family",vars,1)?;
        while let Some(cohort)=cohorts.next().await? {
            let cohort:Cohort=decode(cohort)?;
            let mut row=Object::new();
            row.insert("id",RecordId::new("vector",format!("{}_{}_{:02}",projection.id().hex(),cohort.input.hex(),cohort.family as i16)));
            row.insert("encoder_hash",encoder.service_hash.hex());
            row.insert("policy_key",policy.id().hex());
            row.insert("library_input",scope_string(&crate_json(cohort.input)?));
            row.insert("family",cohort.family as i16);
            row.insert("full_key",full.id().hex());
            row.insert("projection_key",projection.id().hex());
            row.insert("embedding",embedding.clone());
            expected.emit("vector",Value::Object(row))?;
        }
    }
    // Exact window text is shared per family; primary part associations alone nominate targets.
    let mut windows=reader.record_stream::<SearchWindow>("true",Variables::new(),"semantic_key")?;
    while let Some(window)=windows.next().await? {
        let unit=one::<Unit>(reader,window.unit).await?;
        let mut parts=reader.record_stream::<WindowPart>("scope_keys CONTAINS $scope",scope::<WindowPart>("window",window.id())?,"semantic_key")?;
        while let Some(link)=parts.next().await? {
            let part=one::<ContentPart>(reader,link.part).await?;
            if part.unit!=unit.id() {return Err(ModelError::Conflict("window primary part unit"));}
            if part.purpose!=PartPurpose::Primary {continue;}
            let mut row=Object::new();
            row.insert("id",RecordId::new(table(unit.family),window.digest.hex()));
            row.insert("text",window.text.as_str().to_owned());
            row.insert("digest",crate_json(window.digest)?);
            row.insert("scope_digest",scope_string(row.get("digest").expect("digest")));
            expected.emit(table(unit.family),Value::Object(row))?;
        }
    }
    for (index,table) in TABLES[..5].iter().enumerate() {
        let rows=expected.finish(index)?;
        if loader.is_some() {
            let mut batch=Batch::new(loader,table);
            while let Some(row)=rows.next_row()? {batch.emit(row).await?;}
            batch.flush().await?; rows.rewind()?;
        }
    }
    let mut lexical=Batch::new(loader,"lex_occurs");
    let mut vectors=Batch::new(loader,"vec_occurs");
    let mut windows=reader.record_stream::<SearchWindow>("true",Variables::new(),"semantic_key")?;
    while let Some(window)=windows.next().await? {
        let unit=one::<Unit>(reader,window.unit).await?;
        let mut parts=reader.record_stream::<WindowPart>("scope_keys CONTAINS $scope",scope::<WindowPart>("window",window.id())?,"semantic_key")?;
        while let Some(link)=parts.next().await? {
            let part=one::<ContentPart>(reader,link.part).await?;
            if part.purpose!=PartPurpose::Primary {continue;}
            let mut vars=scope::<WindowBinding>("window",window.id())?;
            vars.insert("part",crate_json(part.id())?);
            let mut bindings=reader.record_stream::<WindowBinding>("scope_keys CONTAINS $scope AND body.part=$part",vars,"semantic_key")?;
            let mut found=false;
            while let Some(binding)=bindings.next().await? {
                found=true;
                emit_witness(reader,expected,&mut lexical,&mut vectors,&unit,&window,&part,Some(&binding)).await?;
            }
            if !found {emit_witness(reader,expected,&mut lexical,&mut vectors,&unit,&window,&part,None).await?;}
        }
    }
    lexical.flush().await?; vectors.flush().await?; Ok(())
}
#[allow(clippy::too_many_arguments, reason="One primary witness retains separate physical writers and canonical lineage")]
async fn emit_witness(reader:&NativeReader,expected:&mut Expected,lexical:&mut Batch<'_>,vectors:&mut Batch<'_>,unit:&Unit,window:&SearchWindow,part:&ContentPart,binding:Option<&WindowBinding>)->Result<(),ModelError>{
    let mut option_key=String::new();
    let mut source_path=String::new();
    let member=if let Some(binding)=binding {
        if binding.window!=window.id() || binding.part!=part.id() {return Err(ModelError::Conflict("primary binding lineage"));}
        match one::<Subject>(reader,binding.subject).await? {
            Subject::Member{member}=>Some(member),
            Subject::Option{option}=>{
                let option=one::<catalog::CatalogOption>(reader,option).await?;
                option_key=option_name(reader,&option).await?;
                Some(option.member)
            },
            Subject::Source{artifact}=>{source_path=one::<source::SourceArtifact>(reader,artifact).await?.path;None},
            Subject::Definition{entity}=>match one::<Origin>(reader,unit.origin).await? {
                Origin::Definition{member,entity:owner} if owner==entity=>Some(member),
                _=>None,
            },
            _=>None,
        }
    } else {None};
    let (name,path)=if let Some(member)=member {
        let member=one::<catalog::CatalogMember>(reader,member).await?;
        if member.input!=unit.input {return Err(ModelError::Conflict("foreign primary member input"));}
        (member.path.last().cloned().unwrap_or_default(),member.name)
    } else {(String::new(),source_path)};
    // Only maps overlapping this actual primary part's window nominate original anchors.
    let mut vars=scope::<WindowSourceMap>("window",window.id())?;
    vars.insert("part",crate_json(part.id())?);
    let mut maps=reader.record_stream::<WindowSourceMap>("scope_keys CONTAINS $scope AND body.part=$part",vars,"semantic_key")?;
    let mut anchor=None;
    while let Some(map)=maps.next().await? {
        if let Some(original)=map.original {
            let mut vars=scope::<OriginalAnchor>("unit",unit.id())?;
            vars.insert("original",crate_json(original)?);
            let mut anchors=reader.record_stream::<OriginalAnchor>("scope_keys CONTAINS $scope AND body.original=$original",vars,"semantic_key")?;
            while let Some(row)=anchors.next().await? {anchor=Some(anchor.map_or(row.id(),|old:Id<OriginalAnchor>|old.min(row.id())));}
        }
    }
    let witness=Witness{unit,window,part,binding:binding.map(Record::id),member,anchor,name,path,option_key};
    let mut identity=KeySink::new("native-search-occurrence/v2");
    unit.id().encode(&mut identity);window.id().encode(&mut identity);part.id().encode(&mut identity);witness.binding.encode(&mut identity);
    let key=identity.finish().hex();
    let row=occurrence("lex_occurs",&key,RecordId::new(table(unit.family),window.digest.hex()),&witness)?;
    expected.emit("lex_occurs",row.clone())?;lexical.emit(row).await?;
    let mut links=reader.query_stream("SELECT (SELECT VALUE out.semantic_key FROM participant WHERE in=$parent.id AND field='projection')[0] AS projection FROM assertion WHERE semantic_type='retrieval_embedding_uses' AND body.availability=0 AND scope_keys CONTAINS $scope ORDER BY semantic_key",scope::<RetrievalEmbeddingUse>("window",window.id())?,1)?;
    while let Some(link)=links.next().await? {
        let link:VectorLink=decode(link)?;
        let row=occurrence("vec_occurs",&format!("{key}_{}",link.projection),vector_id(&link.projection,unit),&witness)?;
        expected.emit("vec_occurs",row.clone())?;vectors.emit(row).await?;
    }
    Ok(())
}
async fn option_name(reader:&NativeReader,option:&catalog::CatalogOption)->Result<String,ModelError>{
    use catalog::CatalogOptionSubject;
    use normalized::{callables::SignatureSlot,entities::{FieldEntity,ParameterEntity}};
    let parameter=match one::<CatalogOptionSubject>(reader,option.subject).await? {
        CatalogOptionSubject::Field{field}=>return Ok(one::<FieldEntity>(reader,field).await?.name.as_str().to_owned()),
        CatalogOptionSubject::Parameter{slot}=>Some(one::<SignatureSlot>(reader,slot).await?.parameter),
        CatalogOptionSubject::SourceParameter{parameter}=>match one::<ParameterEntity>(reader,parameter).await? {
            ParameterEntity::NativeSlot{parameter,..}=>Some(parameter),
            ParameterEntity::Source{..}=>{
                let mut links=reader.record_stream::<normalized::entities::ParameterEntityLink>("scope_keys CONTAINS $scope",scope::<normalized::entities::ParameterEntityLink>("entity",parameter)?,"semantic_key")?;
                let mut name=None;
                while let Some(link)=links.next().await? {
                    let parameter=one::<calls::SignatureParameter>(reader,link.parameter).await?;
                    let shape=one::<calls::ParameterShape>(reader,parameter.shape).await?;
                    if let Some(actual)=shape.name {
                        let actual=actual.as_str().to_owned();
                        if name.as_ref().is_some_and(|old|old!=&actual) {return Err(ModelError::Conflict("source option has competing declared names"));}
                        name=Some(actual);
                    }
                }
                return Ok(name.unwrap_or_default());
            },
        },
    };
    if let Some(parameter)=parameter {
        let parameter=one::<calls::SignatureParameter>(reader,parameter).await?;
        return Ok(one::<calls::ParameterShape>(reader,parameter.shape).await?.name.map(|s|s.as_str().to_owned()).unwrap_or_default());
    }
    Ok(String::new())
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
struct Witness<'a> {
    unit: &'a Unit, window:&'a SearchWindow, part:&'a ContentPart,
    binding:Option<Id<WindowBinding>>,member:Option<Id<catalog::CatalogMember>>,anchor:Option<Id<OriginalAnchor>>,
    name:String,path:String,option_key:String,
}
fn occurrence(table:&str,key:&str,input:RecordId,w:&Witness<'_>)->Result<Value,ModelError>{
    let out=w.member.map(|m|target_id(Target::Entity(EntityId::of(m)))).unwrap_or_else(||target_id(Target::Entity(EntityId::of(w.unit.id()))));
    let mut row=Object::new();
    row.insert("id",RecordId::new(table,key));row.insert("in",input);row.insert("out",out);
    row.insert("family",w.unit.family as i16);row.insert("unit",crate_json(w.unit.id())?);
    row.insert("window",crate_json(w.window.id())?);row.insert("part",crate_json(w.part.id())?);row.insert("binding",crate_json(w.binding)?);
    row.insert("context",crate_json(w.unit.context)?);row.insert("member",crate_json(w.member)?);row.insert("anchor",crate_json(w.anchor)?);
    row.insert("input",crate_json(w.unit.input)?);row.insert("eligible",true);
    row.insert("exact_name",w.name.clone());row.insert("exact_path",w.path.clone());row.insert("exact_option",w.option_key.clone());
    row.insert("occurrence_key",format!("{}|{:02}|{}|{}|{}|{}|{}",w.member.map(|id|format!("0{}",id.hex())).unwrap_or_else(||format!("1{}",w.unit.id().hex())),w.unit.family as i16,w.unit.id().hex(),w.window.id().hex(),w.part.id().hex(),w.unit.context.hex(),key));
    for field in ["input","member","context"] {row.insert(format!("scope_{field}"),scope_string(row.get(field).expect("occurrence field")));}
    Ok(Value::Object(row))
}
