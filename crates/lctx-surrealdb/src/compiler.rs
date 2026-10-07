//! Private native compiler authority. Completed memberships, rather than a publication handle,
//! govern reads. Canonical graph payloads and query fields are immutable mechanical forms.
use crate::{Loader, RuntimeConfig, reader::{self, NativeRows}};
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use lctx_model::domain::{self as d, *, completed::*, admission::Frontier, stages::ProviderOutcome};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap}, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}}};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Bytes, Object, RecordId, Value, Variables, SurrealValue, SerdeWrapper, ToSql}};
use tokio::sync::Notify;

#[derive(Clone)]
pub enum NativePredicate {
    Keys(Vec<[u8;16]>),
    Field { field: String, values: Vec<Value> },
    /// SQL comes only from the finite operation/physical planner; all values remain bindings.
    Sql { sql: String, bindings: Variables },
}

pub struct NativeCompilerStore {
    client: Arc<Surreal<Client>>,
    database: d::serving::Name,
    namespace: d::serving::Name,
    specifications: Mutex<BTreeMap<ContentHash, ContributionSpec>>,
    failed: AtomicBool,
    sealed: AtomicBool,
    reads: AtomicUsize,
    drained: Notify,
    runtime: tokio::runtime::Handle,
    frontier: Mutex<Frontier>,
}
impl std::fmt::Debug for NativeCompilerStore {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.debug_struct("NativeCompilerStore").field("database",&self.database).finish_non_exhaustive() }
}

pub fn compiler_schema() -> String {
    "DEFINE TABLE compiler_contribution SCHEMAFULL; DEFINE FIELD spec ON compiler_contribution TYPE bytes; DEFINE FIELD descriptor ON compiler_contribution TYPE option<bytes>; DEFINE FIELD completed ON compiler_contribution TYPE bool; DEFINE FIELD logical ON compiler_contribution TYPE option<string>; DEFINE INDEX logical_contribution ON compiler_contribution FIELDS logical UNIQUE; DEFINE TABLE compiler_membership SCHEMAFULL; DEFINE FIELD contribution ON compiler_membership TYPE record<compiler_contribution>; DEFINE FIELD relation ON compiler_membership TYPE string; DEFINE FIELD semantic_key ON compiler_membership TYPE string; DEFINE FIELD node ON compiler_membership TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_membership TYPE string; DEFINE INDEX contribution_rows ON compiler_membership FIELDS contribution,relation,semantic_key UNIQUE; DEFINE INDEX member_keys ON compiler_membership FIELDS relation,semantic_key,contribution; DEFINE TABLE compiler_view SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view TYPE bytes; DEFINE TABLE compiler_binding SCHEMAFULL; DEFINE FIELD descriptor ON compiler_binding TYPE bytes; DEFINE TABLE compiler_alias SCHEMAFULL; DEFINE FIELD source ON compiler_alias TYPE record<entity>; DEFINE FIELD target ON compiler_alias TYPE record<entity>; DEFINE INDEX alias_source ON compiler_alias FIELDS source,target UNIQUE;".to_string()+&crate::schema::compiler_record_schema()
}

impl NativeCompilerStore {
    pub async fn begin(config:&RuntimeConfig, frontier:Frontier)->Result<Arc<Self>,ModelError> {
        let mut sink=KeySink::new("private-native-attempt/v1");
        sink.part(b"clock",&std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(ModelError::codec)?.as_nanos().to_le_bytes());
        sink.part(b"process",&std::process::id().to_le_bytes());
        let database=d::serving::Name::new(format!("snapshot_{}",sink.finish().hex())).map_err(ModelError::codec)?;
        let client=reader::authenticated(&config.endpoint,&config.root_credentials(),None).await?;
        let version=client.version().await.map_err(ModelError::codec)?.to_string();
        if !version.starts_with("3.3.") { return Err(ModelError::Invalid("native compiler requires reviewed SurrealDB3.3".into())); }
        client.query(format!("DEFINE NAMESPACE IF NOT EXISTS `{}`",config.namespace.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        client.use_ns(config.namespace.as_str()).await.map_err(ModelError::codec)?;
        client.query(format!("DEFINE DATABASE `{}` STRICT",database.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        client.use_db(database.as_str()).await.map_err(ModelError::codec)?;
        let setup=async {
            Loader::new(client.clone()).install("").await?;
            client.query(compiler_schema()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            Ok::<(),ModelError>(())
        }.await;
        if let Err(error)=setup {
            let cleanup=async {client.query(format!("REMOVE DATABASE `{}`",database.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok::<(),ModelError>(())}.await;
            return Err(if cleanup.is_err(){ModelError::infrastructure(Infrastructure::Unconfirmed,format!("native setup left owned unselected database {}",database.as_str()))}else{error});
        }
        Ok(Arc::new(Self {client,database,namespace:config.namespace.clone(),specifications:Mutex::default(),failed:AtomicBool::new(false),sealed:AtomicBool::new(false),reads:AtomicUsize::new(0),drained:Notify::new(),runtime:tokio::runtime::Handle::current(),frontier:Mutex::new(frontier)}))
    }
    /// Bind an explicitly owned database session for cold transport/reconciliation.
    /// This does not create a published handle or infer a current relation view.
    pub fn from_existing(client:Arc<Surreal<Client>>,namespace:d::serving::Name,database:d::serving::Name)->Arc<Self> {
        Arc::new(Self {client,database,namespace,specifications:Mutex::default(),failed:AtomicBool::new(false),sealed:AtomicBool::new(false),reads:AtomicUsize::new(0),drained:Notify::new(),runtime:tokio::runtime::Handle::current(),frontier:Mutex::new(Frontier::Facts)})
    }
    pub async fn install_state_schema(&self)->Result<(),ModelError> {
        self.client.query(compiler_schema()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok(())
    }
    pub fn set_frontier(&self,frontier:Frontier)->Result<(),ModelError> {
        self.check()?;
        if !self.specifications.lock().map_err(|_|ModelError::Conflict("contribution owner"))?.is_empty(){return Err(ModelError::Conflict("frontier after compiler writes"));}
        *self.frontier.lock().map_err(|_|ModelError::Conflict("frontier owner"))?=frontier;Ok(())
    }
    pub async fn contributions(&self)->Result<Vec<CompletedContribution>,ModelError> {
        let mut rows=NativeRows::new(self.client.query("SELECT descriptor FROM compiler_contribution WHERE completed=true ORDER BY logical").stream_items().map_err(ModelError::codec)?,1)?;
        let mut contributions=Vec::new();while let Some(row)=rows.next().await?{let object=value_object(&row).ok_or(ModelError::Schema("contribution descriptor row"))?;let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("contribution descriptor bytes"));};let descriptor:CompletedContribution=serde_json::from_slice(bytes).map_err(ModelError::codec)?;descriptor.identity()?;contributions.push(descriptor);}Ok(contributions)
    }
    pub async fn scan_canonical(self:&Arc<Self>,entities:bool)->Result<CompilerRows,ModelError> {
        self.scan_graph(entities,"id,content,canonical,kind,subtype,semantic_type,semantic_key").await
    }
    pub async fn scan_graph_headers(self:&Arc<Self>,entities:bool)->Result<CompilerRows,ModelError> {
        self.scan_graph(entities,"id,kind,subtype,body.byte_len AS source_length").await
    }
    async fn scan_graph(self:&Arc<Self>,entities:bool,fields:&str)->Result<CompilerRows,ModelError> {
        self.check()?;
        let membership="(SELECT VALUE node FROM compiler_membership WHERE contribution.completed=true GROUP BY node)";
        let table=if entities {"entity"}else{"assertion"};
        let aliases=if entities {format!("OR id IN (SELECT VALUE target FROM compiler_alias WHERE source IN {membership} GROUP BY target)")}else{String::new()};
        let sql=format!("SELECT {fields} FROM {table} WHERE id IN {membership} {aliases} ORDER BY id");
        let rows=NativeRows::new(self.client.query(sql).stream_items().map_err(ModelError::codec)?,1)?.with_row_bytes(64<<20);
        self.reads.fetch_add(1,Ordering::AcqRel);Ok(CompilerRows{rows:Some(rows),store:self.clone()})
    }
    pub fn client(&self)->&Surreal<Client> { &self.client }
    pub fn shared_client(&self)->Arc<Surreal<Client>> { self.client.clone() }
    pub fn database(&self)->&d::serving::Name { &self.database }
    pub fn namespace(&self)->&d::serving::Name { &self.namespace }
    pub fn fail(&self) { self.failed.store(true,Ordering::Release); self.drained.notify_waiters(); }
    pub fn check(&self)->Result<(),ModelError> {
        if self.failed.load(Ordering::Acquire) || self.sealed.load(Ordering::Acquire) { return Err(ModelError::Conflict("native compiler authority closed or failed")); } Ok(())
    }
    pub async fn abandon(&self)->Result<(),ModelError> {
        self.drain().await?;
        self.client.query(format!("REMOVE DATABASE `{}`",self.database.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(())
    }
    pub async fn drain(&self)->Result<(),ModelError> {
        loop {
            let notification=self.drained.notified();tokio::pin!(notification);notification.as_mut().enable();
            if self.reads.load(Ordering::Acquire)==0 {break;} notification.await;
        }
        Ok(())
    }
    pub async fn end_writes(&self)->Result<(),ModelError> { self.drain().await?; self.check()?; self.sealed.store(true,Ordering::Release); Ok(()) }
    pub async fn begin_contribution(&self,spec:ContributionSpec)->Result<ContentHash,ModelError> {
        self.check()?; self.validate_inputs(&spec.inputs).await?; let id=spec.identity()?;
        { let mut specifications=self.specifications.lock().map_err(|_|ModelError::Conflict("native contribution owner"))?;
          if specifications.insert(id,spec.clone()).is_some() { self.fail(); return Err(ModelError::Conflict("duplicate native contribution")); } }
        let mut row=Object::new(); row.insert("id",RecordId::new("compiler_contribution",id.hex())); row.insert("spec",Bytes::from(serde_json::to_vec(&spec).map_err(ModelError::codec)?)); row.insert("completed",false);
        let mut b=Variables::new(); b.insert("row",row);
        let write=async {self.client.query("INSERT INTO compiler_contribution $row RETURN NONE").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok::<(),ModelError>(())}.await;
        if let Err(e)=write { self.fail(); return Err(e); }
        Ok(id)
    }
    pub async fn write_batch(&self,contribution:&ContentHash,relation:&Relation,batch:&RecordBatch)->Result<(),ModelError> {
        self.check()?;
        if !self.specifications.lock().map_err(|_|ModelError::Conflict("native contribution owner"))?.contains_key(contribution) { return Err(ModelError::Conflict("unknown producer contribution")); }
        let result=self.write_batch_inner(contribution,relation,batch).await;
        if result.is_err() { self.fail(); } result
    }
    async fn write_batch_inner(&self,contribution:&ContentHash,relation:&Relation,batch:&RecordBatch)->Result<(),ModelError> {
        let batch=relation.canonical(batch)?;
        let ids=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("native nominal ID column"))?;
        let mut bodies=crate::codec::batch_bodies(relation,&batch)?;
        if relation.name()==d::artifact::ArtifactChunk::NAME {
            for (body,row) in bodies.iter_mut().zip(d::artifact::ArtifactChunk::decode(&batch)?) {
                self.persist_original_chunk(&row).await?;
                let start=u64::try_from(row.ordinal).map_err(ModelError::codec)?.checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64).ok_or(ModelError::Schema("original chunk start"))?;
                let mut metadata=Object::new();metadata.insert("__type",relation.name().to_string());metadata.insert("artifact",Value::from_t(row.artifact.bytes().iter().map(|byte|i64::from(*byte)).collect::<Vec<_>>()));metadata.insert("ordinal",row.ordinal);metadata.insert("original",RecordId::new("original",d::graph::EntityId::of(row.artifact).0.hex()));metadata.insert("start",start);metadata.insert("len",row.body.0.len() as u64);metadata.insert("digest",ContentHash::of(&row.body.0).hex());*body=Value::Object(metadata);
            }
        }
        if relation.name()==d::source::SourceArtifact::NAME {
            let rows=d::source::SourceArtifact::decode(&batch)?.into_iter().map(|row| {
                let mut header=Object::new();header.insert("id",RecordId::new("original",d::graph::EntityId::of(row.id()).0.hex()));header.insert("byte_len",row.byte_len);header.insert("content",row.content.hex());Value::Object(header)
            }).collect();self.ensure_original_rows("original",rows).await?;
        }
        let mut graph=vec![None;batch.num_rows()];
        macro_rules! entities {($($variant:ident:$ty:ty,)*)=>{$(if relation.name()==<$ty>::NAME {for (i,row) in <$ty>::decode(&batch)?.into_iter().enumerate() {graph[i]=Some(GraphRow::Entity(d::graph::Entity::from(row)));}})*};}
        lctx_model::graph_entity_records!(entities);
        macro_rules! assertions {($($variant:ident:$ty:ty,)*)=>{$(if relation.name()==<$ty>::NAME {for (i,row) in <$ty>::decode(&batch)?.into_iter().enumerate() {graph[i]=Some(GraphRow::Assertion(d::graph::Assertion::from_record(row)?));}})*};}
        lctx_model::graph_assertion_records!(assertions);
        let native_aliases=if *self.frontier.lock().map_err(|_|ModelError::Conflict("frontier owner"))?!=Frontier::Facts {
            graph.iter().filter_map(|row|if let Some(GraphRow::Entity(source))=row {source.canonical_place_endpoint().map(|target|(source.id(),target))}else{None}).collect::<Vec<_>>()
        }else{Vec::new()};
        let mut nodes=BTreeMap::<String,Value>::new(); let mut members=BTreeMap::<String,Value>::new();
        for (i,(body,graph)) in bodies.into_iter().zip(graph).enumerate() {
            let key=hex::encode(ids.value(i));
            let (node,content,canonical,kind,subtype)=match graph {
                Some(GraphRow::Entity(row))=>(RecordId::new("entity",row.id().0.hex()),row.content(),serde_json::to_vec(&row).map_err(ModelError::codec)?,Some(row.kind() as i64),row.subtype()),
                Some(GraphRow::Assertion(row))=>(RecordId::new("assertion",row.id().0.hex()),row.content(),serde_json::to_vec(&row).map_err(ModelError::codec)?,Some(row.kind as i64),None),
                None=>{ let bytes=serde_json::to_vec(&body).map_err(ModelError::codec)?; let mut sink=KeySink::new("compiler-backing-key/v1"); relation.name().to_string().encode(&mut sink); sink.part(b"key",ids.value(i)); (RecordId::new("compiler_record",sink.finish().hex()),ContentHash::of(&bytes),bytes,None,None) }
            };
            let mut physical=Object::new(); physical.insert("id",node.clone()); physical.insert("semantic_type",relation.name().to_string()); physical.insert("semantic_key",key.clone()); physical.insert("content",content.hex()); physical.insert("canonical",Bytes::from(canonical)); physical.insert("body",body);
            if let Some(kind)=kind {physical.insert("kind",kind);physical.insert("subtype",subtype.map(Value::from_t).unwrap_or(Value::Null));}
            let physical=Value::Object(physical);
            let name=node.to_sql();
            if let Some(previous)=nodes.insert(name,physical.clone()) && previous!=physical { return Err(ModelError::Conflict("native same-key payload")); }
            let mut sink=KeySink::new("compiler-membership/v1"); contribution.encode(&mut sink); relation.name().to_string().encode(&mut sink); sink.part(b"key",ids.value(i));
            let mut member=Object::new();member.insert("id",RecordId::new("compiler_membership",sink.finish().hex()));member.insert("contribution",RecordId::new("compiler_contribution",contribution.hex()));member.insert("relation",relation.name().to_string());member.insert("semantic_key",key);member.insert("node",node);member.insert("content",content.hex());members.insert(sink_key(&member)?,Value::Object(member));
        }
        let alias_values=crate::codec::entity_views(&native_aliases.iter().map(|(_,target)|target.clone()).collect::<Vec<_>>())?;
        let mut aliases=Vec::new();
        for ((source,target),view) in native_aliases.into_iter().zip(alias_values) {
            let node=RecordId::new("entity",target.id().0.hex());let mut physical=Object::new();physical.insert("id",node.clone());physical.insert("semantic_type",view.semantic_type);physical.insert("semantic_key",view.semantic_key);physical.insert("kind",target.kind() as i64);physical.insert("subtype",target.subtype().map(Value::from_t).unwrap_or(Value::Null));physical.insert("content",target.content().hex());physical.insert("canonical",Bytes::from(serde_json::to_vec(&target).map_err(ModelError::codec)?));physical.insert("body",view.body);
            let physical=Value::Object(physical);if let Some(previous)=nodes.insert(node.to_sql(),physical.clone()) && previous!=physical {return Err(ModelError::Conflict("native alias payload"));}
            let mut sink=KeySink::new("native-place-alias/v1");source.0.encode(&mut sink);target.id().0.encode(&mut sink);
            let mut alias=Object::new();alias.insert("id",RecordId::new("compiler_alias",sink.finish().hex()));alias.insert("source",RecordId::new("entity",source.0.hex()));alias.insert("target",node);aliases.push(Value::Object(alias));
        }
        // Read only the candidate physical IDs; compare full canonical payloads, not digest alone.
        let mut b=Variables::new();b.insert("ids",nodes.values().map(|v|value_object(v).and_then(|o|o.get("id")).cloned().ok_or(ModelError::Schema("native payload id"))).collect::<Result<Vec<_>,_>>()?);
        let mut response=self.client.query("SELECT * FROM $ids").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let existing:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
        for stored in existing {
            let object=value_object(&stored).ok_or(ModelError::Schema("stored native row"))?;
            let id=object.get("id").ok_or(ModelError::Schema("stored native row id"))?.to_sql();
            if let Some(expected)=nodes.remove(&id) { let expected=value_object(&expected).ok_or(ModelError::Schema("expected native row"))?;
                for field in ["semantic_type","semantic_key","canonical","body","kind","subtype","content"] {if expected.get(field)!=object.get(field) {return Err(ModelError::Conflict("native same-key payload"));}}
            }
        }
        for table in ["entity","assertion","compiler_record"] {
            let rows=nodes.values().filter(|v|value_object(v).and_then(|o|o.get("id")).is_some_and(|id|id.to_sql().starts_with(&format!("{table}:")))).cloned().collect::<Vec<_>>();
            if !rows.is_empty() { let mut b=Variables::new();b.insert("rows",rows);self.client.query(format!("INSERT INTO {table} $rows RETURN NONE")).bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?; }
        }
        if !aliases.is_empty(){let mut b=Variables::new();b.insert("rows",aliases);self.client.query("FOR $row IN $rows { UPSERT $row.id CONTENT $row; };").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;}
        // Repeated identical writes within this contribution are idempotent, without hiding
        // semantic uniqueness failures. Membership contains no separately editable payload.
        let mut b=Variables::new();b.insert("rows",members.into_values().collect::<Vec<_>>());
        self.client.query("FOR $row IN $rows { UPSERT $row.id CONTENT $row; }; ").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(())
    }

    async fn persist_original_chunk(&self,row:&d::artifact::ArtifactChunk)->Result<(),ModelError> {
        let source=RecordId::new("original",d::graph::EntityId::of(row.artifact).0.hex());
        let base=u64::try_from(row.ordinal).map_err(ModelError::codec)?.checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64).ok_or(ModelError::Schema("original chunk start"))?;
        let rows=row.body.0.chunks(65536).enumerate().map(|(index,bytes)| {
            let start=base+(index*65536) as u64;
            let mut chunk=Object::new();chunk.insert("id",RecordId::new("original_chunk",format!("{}_{}",d::graph::EntityId::of(row.artifact).0.hex(),start)));chunk.insert("source",source.clone());chunk.insert("start",start);chunk.insert("bytes",Bytes::from(bytes.to_vec()));chunk.insert("content",ContentHash::of(bytes).hex());Value::Object(chunk)
        }).collect();
        self.ensure_original_rows("original_chunk",rows).await
    }
    async fn ensure_original_rows(&self,table:&str,rows:Vec<Value>)->Result<(),ModelError> {
        let mut candidates=BTreeMap::new();for row in rows {let id=value_object(&row).and_then(|row|row.get("id")).ok_or(ModelError::Schema("original row key"))?.to_sql();if let Some(previous)=candidates.insert(id,row.clone()) && previous!=row {return Err(ModelError::Conflict("original same-key payload"));}}
        let ids=candidates.values().map(|row|value_object(row).expect("checked object").get("id").expect("checked key").clone()).collect::<Vec<_>>();let mut b=Variables::new();b.insert("ids",ids);
        let mut result=self.client.query("SELECT * FROM $ids").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let actual:Vec<Value>=result.take(0).map_err(ModelError::codec)?;
        for row in actual {let id=value_object(&row).and_then(|row|row.get("id")).ok_or(ModelError::Schema("stored original row key"))?.to_sql();if candidates.remove(&id)!=Some(row) {return Err(ModelError::Conflict("original same-key payload"));}}
        if !candidates.is_empty(){let mut b=Variables::new();b.insert("rows",candidates.into_values().collect::<Vec<_>>());self.client.query(format!("INSERT INTO {table} $rows RETURN NONE")).bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;}
        Ok(())
    }
    pub async fn complete_contribution(self:&Arc<Self>,id:ContentHash,outcome:ProviderOutcome,outputs:&[Relation],previous:&BTreeMap<String,CompletedView>)->Result<BTreeMap<String,CompletedView>,ModelError> {
        let result=self.complete_contribution_inner(id,outcome,outputs,previous).await;
        if result.is_err(){self.fail();}
        result
    }
    async fn complete_contribution_inner(self:&Arc<Self>,id:ContentHash,outcome:ProviderOutcome,outputs:&[Relation],previous:&BTreeMap<String,CompletedView>)->Result<BTreeMap<String,CompletedView>,ModelError> {
        self.drain().await?;self.check()?;
        let spec=self.specifications.lock().map_err(|_|ModelError::Conflict("native contribution owner"))?.get(&id).cloned().ok_or(ModelError::Conflict("unknown native contribution"))?;
        let mut content=BTreeMap::<String,(u64,KeySink)>::new();
        for output in outputs {content.insert(output.name().into(),(0,KeySink::new("native-contribution-output/v1")));}
        let mut b=Variables::new();b.insert("contribution",RecordId::new("compiler_contribution",id.hex()));
        let mut stream=NativeRows::new(self.client.query("SELECT relation,semantic_key,content FROM compiler_membership WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(b).stream_items().map_err(ModelError::codec)?,1)?;
        while let Some(row)=stream.next().await? {
            let row=SerdeWrapper::<MembershipContent>::from_value(row).map_err(ModelError::codec)?.0;
            let (count,sink)=content.get_mut(&row.relation).ok_or(ModelError::Conflict("undeclared native output"))?;
            row.semantic_key.encode(sink);row.content.encode(sink);*count+=1;
        }
        let descriptor=CompletedContribution {spec,outcome:outcome.code(),outputs:content.into_iter().map(|(name,(rows,sink))|(name,OutputContent{rows,content:sink.finish()})).collect()};
        let logical=descriptor.identity()?;
        let mut b=Variables::new();b.insert("id",RecordId::new("compiler_contribution",id.hex()));b.insert("descriptor",Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?));b.insert("logical",logical.hex());
        let response=self.client.query("UPDATE $id SET descriptor=$descriptor,logical=$logical,completed=true WHERE completed=false RETURN VALUE completed").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let mut response=response; let success:Vec<bool>=response.take(0).map_err(ModelError::codec)?; if success!=vec![true] {self.fail();return Err(ModelError::Conflict("native completion transition"));}
        let mut views=BTreeMap::new();
        for relation in outputs {
            let mut contributions=previous.get(relation.name()).map(|v|v.contributions.clone()).unwrap_or_default();contributions.insert(logical);
            let mut b=Variables::new();b.insert("contributions",contributions.iter().map(ContentHash::hex).collect::<Vec<_>>());b.insert("relation",relation.name().to_string());
            let mut keys=NativeRows::new(self.client.query("SELECT semantic_key FROM compiler_membership WHERE relation=$relation AND contribution IN (SELECT VALUE id FROM compiler_contribution WHERE completed=true AND logical IN $contributions) GROUP BY semantic_key").bind(b).stream_items().map_err(ModelError::codec)?,1)?;
            let mut count=0u64;
            while keys.next().await?.is_some() {count=count.checked_add(1).ok_or(ModelError::Schema("native union count overflow"))?;}
            let view=CompletedView::new(relation.name().into(),contributions,count)?;
            let mut row=Object::new();row.insert("id",RecordId::new("compiler_view",view.identity.hex()));row.insert("descriptor",Bytes::from(serde_json::to_vec(&view).map_err(ModelError::codec)?));let mut b=Variables::new();b.insert("row",row);
            self.client.query("UPSERT $row.id CONTENT $row RETURN NONE").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            views.insert(relation.name().to_string(),view);
        }
        Ok(views)
    }

    async fn validate_inputs(&self,inputs:&[d::analysis::sources::SourceSnapshot])->Result<(),ModelError> {
        if inputs.is_empty(){return Ok(());}
        let mut b=Variables::new();b.insert("ids",inputs.iter().map(|source|RecordId::new("compiler_view",source.view().hex())).collect::<Vec<_>>());
        let mut response=self.client.query("SELECT VALUE descriptor FROM $ids").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let descriptors:Vec<Bytes>=response.take(0).map_err(ModelError::codec)?;
        let mut views=BTreeMap::new();for bytes in descriptors {let view:CompletedView=serde_json::from_slice(&bytes).map_err(ModelError::codec)?;view.validate()?;if let Some(old)=views.insert(view.identity,view.clone()) && old!=view {return Err(ModelError::Conflict("completed view collision"));}}
        for source in inputs {let view=views.get(&source.view()).ok_or(ModelError::Conflict("missing dependency view"))?;
            if view.relation!=source.relation() || source.rows()<0 || view.rows!=source.rows() as u64 {return Err(ModelError::Conflict("dependency view metadata"));}}
        Ok(())
    }
    /// Independent cold import/audit checks. Compilation carries completed validation instead.
    pub async fn verify_state(&self)->Result<(),ModelError> {
        let mut rows=NativeRows::new(self.client.query("SELECT VALUE id FROM compiler_membership WHERE node.semantic_type IS NONE OR node.semantic_type != relation OR node.semantic_key != semantic_key OR node.content != content OR contribution.completed != true LIMIT 1").stream_items().map_err(ModelError::codec)?,1)?;
        if rows.next().await?.is_some(){return Err(ModelError::Conflict("completed membership backing/visibility"));}
        let mut descriptors=NativeRows::new(self.client.query("SELECT descriptor FROM compiler_contribution ORDER BY id").stream_items().map_err(ModelError::codec)?,1)?;
        while let Some(row)=descriptors.next().await? {
            let object=value_object(&row).ok_or(ModelError::Schema("contribution descriptor row"))?;
            let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("contribution descriptor bytes"));};
            let contribution:CompletedContribution=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            contribution.identity()?;self.validate_inputs(&contribution.spec.inputs).await?;
            let mut actual=contribution.outputs.keys().map(|name|(name.clone(),(0u64,KeySink::new("native-contribution-output/v1")))).collect::<BTreeMap<_,_>>();
            let mut variables=Variables::new();variables.insert("contribution",RecordId::new("compiler_contribution",contribution.spec.identity()?.hex()));
            let mut members=NativeRows::new(self.client.query("SELECT relation,semantic_key,content FROM compiler_membership WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(variables).stream_items().map_err(ModelError::codec)?,1)?;
            while let Some(row)=members.next().await? {
                let member=SerdeWrapper::<MembershipContent>::from_value(row).map_err(ModelError::codec)?.0;
                let (count,sink)=actual.get_mut(&member.relation).ok_or(ModelError::Conflict("undeclared restored output"))?;
                member.semantic_key.encode(sink);member.content.encode(sink);*count+=1;
            }
            let actual=actual.into_iter().map(|(name,(rows,sink))|(name,OutputContent {rows,content:sink.finish()})).collect::<BTreeMap<_,_>>();
            if actual!=contribution.outputs {return Err(ModelError::Conflict("restored contribution output membership"));}
        }
        for view in self.views().await? {
            let mut b=Variables::new();b.insert("relation",view.relation.clone());b.insert("contributions",view.contributions.iter().map(ContentHash::hex).collect::<Vec<_>>());
            let mut rows=NativeRows::new(self.client.query("SELECT semantic_key FROM compiler_membership WHERE relation=$relation AND contribution IN (SELECT VALUE id FROM compiler_contribution WHERE completed=true AND logical IN $contributions) GROUP BY semantic_key").bind(b).stream_items().map_err(ModelError::codec)?,1)?;
            let mut count=0u64;while rows.next().await?.is_some(){count+=1;}if count!=view.rows{return Err(ModelError::Conflict("completed view cardinality"));}
            let mut b=Variables::new();b.insert("logical",view.contributions.iter().map(ContentHash::hex).collect::<Vec<_>>());
            let mut response=self.client.query("SELECT VALUE descriptor FROM compiler_contribution WHERE completed=true AND logical IN $logical").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let descriptors:Vec<Bytes>=response.take(0).map_err(ModelError::codec)?;
            if descriptors.len()!=view.contributions.len(){return Err(ModelError::Conflict("completed view contribution membership"));}
            for bytes in descriptors {let contribution:CompletedContribution=serde_json::from_slice(&bytes).map_err(ModelError::codec)?;if !contribution.outputs.contains_key(&view.relation){return Err(ModelError::Conflict("completed view output ownership"));}}
        }
        for binding in self.bindings().await? {self.registered_view(&binding.view).await?;}
        Ok(())
    }
    async fn registered_view(&self,view:&CompletedView)->Result<(),ModelError> {
        view.validate()?;
        let mut b=Variables::new();b.insert("id",RecordId::new("compiler_view",view.identity.hex()));
        let mut response=self.client.query("SELECT VALUE descriptor FROM $id").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let descriptors:Vec<Bytes>=response.take(0).map_err(ModelError::codec)?;
        if descriptors.len()!=1 || serde_json::from_slice::<CompletedView>(&descriptors[0]).map_err(ModelError::codec)?!=*view {return Err(ModelError::Conflict("unregistered native completed view"));}
        Ok(())
    }
    pub async fn bind(&self,binding:CompletedBinding)->Result<(),ModelError> {
        self.check()?;binding.validate()?;self.registered_view(&binding.view).await?;
        let mut row=Object::new();row.insert("id",RecordId::new("compiler_binding",binding.key().hex()));
        row.insert("descriptor",Bytes::from(serde_json::to_vec(&binding).map_err(ModelError::codec)?));
        let mut b=Variables::new();b.insert("row",row);
        if binding.boundary.is_some() {
            let mut lookup=Variables::new();lookup.insert("id",RecordId::new("compiler_binding",binding.key().hex()));
            let mut response=self.client.query("SELECT VALUE descriptor FROM $id").bind(lookup).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let stored:Vec<Bytes>=response.take(0).map_err(ModelError::codec)?;
            if !stored.is_empty() {
                if stored.len()!=1 || serde_json::from_slice::<CompletedBinding>(&stored[0]).map_err(ModelError::codec)?!=binding {return Err(ModelError::Conflict("frozen native binding replacement"));}
                return Ok(());
            }
        }
        self.client.query("UPSERT $row.id CONTENT $row RETURN NONE").bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        Ok(())
    }
    pub async fn bindings(&self)->Result<Vec<CompletedBinding>,ModelError> {
        let mut stream=NativeRows::new(self.client.query("SELECT descriptor FROM compiler_binding ORDER BY id").stream_items().map_err(ModelError::codec)?,1)?;
        let mut bindings=Vec::new();
        while let Some(row)=stream.next().await? {
            let object=value_object(&row).ok_or(ModelError::Schema("native binding row"))?;
            let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("native binding descriptor"));};
            let binding:CompletedBinding=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            binding.validate()?;bindings.push(binding);
        }
        Ok(bindings)
    }
    /// Compact retained dependency inspection. No mutable scheduling authority is exposed.
    pub async fn views(&self)->Result<Vec<CompletedView>,ModelError> {
        let mut rows=NativeRows::new(self.client.query("SELECT descriptor FROM compiler_view ORDER BY id").stream_items().map_err(ModelError::codec)?,1)?;
        let mut views=Vec::new();
        while let Some(row)=rows.next().await? {
            let object=value_object(&row).ok_or(ModelError::Schema("native completed view"))?;
            let Some(Value::Bytes(bytes))=object.get("descriptor") else {return Err(ModelError::Schema("native completed view descriptor"));};
            let view:CompletedView=serde_json::from_slice(bytes).map_err(ModelError::codec)?;view.validate()?;views.push(view);
        }
        Ok(views)
    }
    pub async fn completed_state(&self)->Result<CompletedStateIdentity,ModelError> {
        self.drain().await?;
        let mut sink=KeySink::new("native-completed-state/v1");
        let mut counts=[0u64;6];
        for (i,table) in STATE_TABLES.iter().enumerate() {
            table.to_string().encode(&mut sink);
            let mut rows=NativeRows::new(self.client.query(format!("SELECT * FROM {table} ORDER BY id")).stream_items().map_err(ModelError::codec)?,1)?;
            while let Some(row)=rows.next().await? {
                validate_state_row(table,&row)?;
                sink.part(b"row",&serde_json::to_vec(&row).map_err(ModelError::codec)?);
                counts[i]=counts[i].checked_add(1).ok_or(ModelError::Schema("completed state row count"))?;
            }
        }
        Ok(CompletedStateIdentity {format_version:STATE_FORMAT_VERSION,contributions:counts[0],memberships:counts[1],backing_rows:counts[3],content:sink.finish()})
    }
    /// Explicit detached transport only. Ordinary publication seals this database directly.
    pub async fn export_state(&self,path:&std::path::Path)->Result<CompletedStateIdentity,ModelError> {
        use std::io::Write;
        let expected=self.completed_state().await?;
        let mut file=std::io::BufWriter::new(std::fs::File::create(path).map_err(ModelError::codec)?);
        for table in STATE_TABLES {
            let mut rows=NativeRows::new(self.client.query(format!("SELECT * FROM {table} ORDER BY id")).stream_items().map_err(ModelError::codec)?,1)?;
            while let Some(row)=rows.next().await? {
                serde_json::to_writer(&mut file,&StateRow{table:table.into(),row}).map_err(ModelError::codec)?;
                file.write_all(b"\n").map_err(ModelError::codec)?;
            }
        }
        file.flush().map_err(ModelError::codec)?;file.get_ref().sync_all().map_err(ModelError::codec)?;
        Ok(expected)
    }
    pub async fn import_state(&self,path:&std::path::Path,expected:&CompletedStateIdentity)->Result<(),ModelError> {
        use std::io::{BufRead,Read};
        self.check()?;
        // Each envelope owns one bounded row; only fixed generated table names are admitted.
        let mut file=std::io::BufReader::new(std::fs::File::open(path).map_err(ModelError::codec)?);
        let mut bytes=Vec::new();let mut previous:Option<(usize,String)>=None;
        loop {
            bytes.clear();let read=file.by_ref().take((64<<20)+1).read_until(b'\n',&mut bytes).map_err(ModelError::codec)?;
            if read==0 {break;}if read>64<<20 {self.fail();return Err(ModelError::Schema("completed state transport row bound"));}
            let row:StateRow=serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
            let table=STATE_TABLES.iter().position(|name|*name==row.table).ok_or(ModelError::Schema("completed state transport table"))?;
            validate_state_row(&row.table,&row.row)?;
            let id=value_object(&row.row).and_then(|object|object.get("id")).ok_or(ModelError::Schema("completed state transport key"))?.to_sql();
            if previous.as_ref().is_some_and(|previous|previous >= &(table,id.clone())) {return Err(ModelError::Conflict("completed state transport order"));}
            previous=Some((table,id));
            let mut b=Variables::new();b.insert("row",row.row);
            self.client.query(format!("INSERT INTO {} $row RETURN NONE",row.table)).bind(b).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        }
        if &self.completed_state().await? != expected {self.fail();return Err(ModelError::Conflict("completed state transport identity"));}
        self.verify_state().await?;
        Ok(())
    }
    pub async fn scan_rows(self:&Arc<Self>,view:&CompletedView,relation:&Relation,columns:Option<&[String]>,predicate:Option<NativePredicate>)->Result<CompilerRows,ModelError> {
        self.check()?; self.registered_view(view).await?;
        if view.relation!=relation.name() {return Err(ModelError::Conflict("native view relation"));}
        let mut b=Variables::new();b.insert("relation",relation.name().to_string());b.insert("contributions",view.contributions.iter().map(ContentHash::hex).collect::<Vec<_>>());
        let selected=predicate.is_some();
        let predicate=match predicate {
            None=>"true".into(),
            Some(NativePredicate::Keys(keys))=>{b.insert("keys",keys.iter().map(hex::encode).collect::<Vec<_>>());"semantic_key IN $keys".into()},
            Some(NativePredicate::Field{field,values})=>{if !relation.fields().iter().any(|f|f.name()==field) {return Err(ModelError::Conflict("native selected field"));}b.insert("values",values);format!("body.`{field}` IN $values")},
            Some(NativePredicate::Sql{sql,bindings})=>{b.extend(bindings);sql},
        };
        let fields=columns.map(|c|c.to_vec()).unwrap_or_else(||relation.schema().fields().iter().map(|f|f.name().clone()).collect());
        let mut projections=Vec::new();
        for field in &fields {if field=="id" {projections.push("semantic_key AS id".to_string());} else if relation.name()==d::artifact::ArtifactChunk::NAME && field=="body" {
                projections.push("(SELECT start,bytes,content FROM original_chunk WHERE source=$parent.body.original AND start >= $parent.body.start AND start < $parent.body.start+$parent.body.len ORDER BY start) AS __original_chunks, body.start AS __original_start, body.len AS __original_len, body.digest AS __original_digest".to_string());
            } else if relation.fields().iter().any(|f|f.name()==field) {projections.push(format!("body.`{field}` AS `{field}`"));} else {return Err(ModelError::Conflict("native projection field"));}}
        if projections.is_empty() {projections.push("semantic_key AS __row".into());}
        // Start with exact selected keys, then fetch their immutable payloads. Membership overlap
        // is deduplicated before rich hydration; private pending rows never enter this query.
        let table=if relation.name()==d::analytics::QualityStep::NAME || relation.name()==d::artifact::ArtifactChunk::NAME {"compiler_record"} else if relation_is_entity(relation.name()) {"entity"} else {"assertion"};
        let candidate=if selected {format!("AND semantic_key IN (SELECT VALUE semantic_key FROM {table} WHERE semantic_type=$relation AND ({predicate}))")} else {String::new()};
        let membership=format!("(SELECT VALUE node FROM compiler_membership WHERE relation=$relation {candidate} AND contribution IN (SELECT VALUE id FROM compiler_contribution WHERE completed=true AND logical IN $contributions) GROUP BY node)");
        let sql=format!("SELECT {} FROM {table} WHERE semantic_type=$relation AND ({predicate}) AND id IN {membership} ORDER BY semantic_key",projections.join(","));
        let rows=NativeRows::new(self.client.query(sql).bind(b).stream_items().map_err(ModelError::codec)?,1)?.with_row_bytes(64<<20);
        self.reads.fetch_add(1,Ordering::AcqRel);
        Ok(CompilerRows {rows:Some(rows),store:self.clone()})
    }
    pub fn table_provider(self:&Arc<Self>,view:&CompletedView,relation:Relation,budget:d::resources::ResourceBudget,batch_rows:usize)->Result<Arc<dyn datafusion::catalog::TableProvider>,ModelError> {
        crate::compiler_provider::table_provider(self.clone(),view.clone(),relation,budget,batch_rows)
    }
    pub async fn scan_batches(self:&Arc<Self>,view:&CompletedView,relation:&Relation,projection:Option<Vec<usize>>,predicate:Option<NativePredicate>,budget:&d::resources::ResourceBudget,batch_rows:usize)->Result<datafusion::physical_plan::SendableRecordBatchStream,ModelError> {
        crate::compiler_provider::scan_batches(self.clone(),view.clone(),relation.clone(),projection,predicate,budget.clone(),batch_rows).await
    }
}

pub struct CompilerRows {rows:Option<NativeRows>,store:Arc<NativeCompilerStore>}
impl CompilerRows {
    pub async fn next(&mut self)->Result<Option<Value>,ModelError> {
        let Some(rows)=&mut self.rows else {return Ok(None);};
        let result=rows.next().await.and_then(|row|row.map(hydrate_original_row).transpose());
        if result.is_err() {self.store.fail();rows.drain_transport().await;}
        if !matches!(&result,Ok(Some(_))) {self.rows.take();self.store.reads.fetch_sub(1,Ordering::AcqRel);self.store.drained.notify_waiters();}
        result
    }
}
impl Drop for CompilerRows {
    fn drop(&mut self) {if let Some(mut rows)=self.rows.take(){let store=self.store.clone();self.store.runtime.spawn(async move {loop { match rows.next().await {Ok(Some(_))=>{},Ok(None)=>break,Err(_)=>{store.fail();rows.drain_transport().await;break;}} }store.reads.fetch_sub(1,Ordering::AcqRel);store.drained.notify_waiters();});}}
}
#[derive(Clone)]
enum GraphRow {Entity(d::graph::Entity),Assertion(d::graph::Assertion)}
#[derive(Serialize,Deserialize)]
struct MembershipContent {relation:String,semantic_key:String,content:String}
fn sink_key(row:&Object)->Result<String,ModelError> {Ok(row.get("id").ok_or(ModelError::Schema("membership id"))?.to_sql())}

fn value_object(value:&Value)->Option<&Object> {if let Value::Object(object)=value {Some(object)} else {None}}

macro_rules! entity_names {($($variant:ident:$ty:ty,)*)=>{fn relation_is_entity(name:&str)->bool {[$(<$ty>::NAME,)*].contains(&name)}};}
lctx_model::graph_entity_records!(entity_names);

const STATE_TABLES:[&str;6]=["compiler_contribution","compiler_membership","compiler_view","compiler_record","compiler_binding","compiler_alias"];
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRow {table:String,row:Value}
fn validate_state_row(table:&str,row:&Value)->Result<(),ModelError> {
    let object=value_object(row).ok_or(ModelError::Schema("completed state object"))?;
    let Some(Value::RecordId(id))=object.get("id") else{return Err(ModelError::Schema("completed state key"));};
    if id.table.as_str()!=table {return Err(ModelError::Schema("completed state table/key"));}
    if table=="compiler_contribution" {
        if object.get("completed")!=Some(&Value::Bool(true)) {return Err(ModelError::Conflict("pending contribution cannot be sealed"));}
        let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("completed contribution descriptor"));};
        let descriptor:CompletedContribution=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if object.get("logical")!=Some(&Value::String(descriptor.identity()?.hex())) || &id.key!=&surrealdb::types::RecordIdKey::String(descriptor.spec.identity()?.hex()) {return Err(ModelError::Conflict("completed contribution identity"));}
        let Some(Value::Bytes(spec))=object.get("spec") else{return Err(ModelError::Schema("completed contribution specification"));};
        if serde_json::from_slice::<ContributionSpec>(spec).map_err(ModelError::codec)? != descriptor.spec {return Err(ModelError::Conflict("completed contribution specification"));}
    } else if table=="compiler_view" {
        let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("completed view descriptor"));};
        let descriptor:CompletedView=serde_json::from_slice(bytes).map_err(ModelError::codec)?;descriptor.validate()?;
        if &id.key!=&surrealdb::types::RecordIdKey::String(descriptor.identity.hex()) {return Err(ModelError::Conflict("completed view key"));}
    } else if table=="compiler_binding" {
        let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("completed binding descriptor"));};
        let descriptor:CompletedBinding=serde_json::from_slice(bytes).map_err(ModelError::codec)?;descriptor.validate()?;
        if &id.key!=&surrealdb::types::RecordIdKey::String(descriptor.key().hex()) {return Err(ModelError::Conflict("completed binding key"));}
    }
    Ok(())
}

fn hydrate_original_row(value:Value)->Result<Value,ModelError> {
    let Value::Object(mut row)=value else{return Ok(value);};
    let Some(Value::Array(chunks))=row.remove("__original_chunks") else{return Ok(Value::Object(row));};
    let start=match row.remove("__original_start"){Some(Value::Number(surrealdb::types::Number::Int(n)))=>u64::try_from(n).map_err(ModelError::codec)?,_=>return Err(ModelError::Schema("original typed chunk start"))};
    let len=match row.remove("__original_len"){Some(Value::Number(surrealdb::types::Number::Int(n)))=>usize::try_from(n).map_err(ModelError::codec)?,_=>return Err(ModelError::Schema("original typed chunk length"))};
    let Some(Value::String(digest))=row.remove("__original_digest") else{return Err(ModelError::Schema("original typed chunk digest"));};
    if len>d::artifact::ARTIFACT_CHUNK_BYTES {return Err(ModelError::Schema("original typed chunk byte bound"));}
    let mut bytes=Vec::with_capacity(len);let mut position=start;
    for chunk in chunks {let Some(chunk)=value_object(&chunk) else{return Err(ModelError::Schema("original physical chunk"));};
        let Some(Value::Number(surrealdb::types::Number::Int(at)))=chunk.get("start") else{return Err(ModelError::Schema("original physical chunk position"));};
        let Some(Value::Bytes(body))=chunk.get("bytes") else{return Err(ModelError::Schema("original physical chunk bytes"));};
        if u64::try_from(*at).map_err(ModelError::codec)?!=position || body.is_empty() || body.len()>65536 || chunk.get("content")!=Some(&Value::String(ContentHash::of(body).hex())) {return Err(ModelError::Conflict("original physical chunk integrity"));}
        bytes.extend_from_slice(body);position+=body.len() as u64;
    }
    if bytes.len()!=len || ContentHash::of(&bytes).hex()!=digest {return Err(ModelError::Conflict("original typed chunk integrity"));}
    row.insert("body",Bytes::from(bytes));Ok(Value::Object(row))
}
