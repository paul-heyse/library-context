//! Admitted semantic graph artifacts. IPC is transport; typed canonical content defines identity.
use crate::workspace::Workspace;
use datafusion::{arrow::{array::{Array, BinaryArray, FixedSizeBinaryArray, Int16Array, UInt32Array, UInt64Array},
    datatypes::{DataType, Field, Schema}, record_batch::RecordBatch,
    ipc::{writer::FileWriter}, compute::take}, prelude::SessionContext, execution::options::ArrowReadOptions};
use futures::TryStreamExt;
use lctx_model::domain::{ContentHash, ModelError, graph::*};
use lctx_model::domain::{charged::StateCharge, resources::ResourceBudget};
use std::{fs::File, path::{Path, PathBuf}, sync::Arc};

fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new("id", DataType::FixedSizeBinary(32), false),
        Field::new("content", DataType::FixedSizeBinary(32), false),
        Field::new("payload", DataType::Binary, false), Field::new("kind", DataType::Int16, false),
        Field::new("source_length", DataType::UInt64, true),Field::new("subtype",DataType::Int16,true)]))
}
struct PendingRow {id:ContentHash,content:ContentHash,payload:Vec<u8>,kind:i16,source_length:Option<u64>,subtype:Option<i16>}
struct FamilyWriter {
    path:PathBuf, writer:FileWriter<File>, pending:Vec<PendingRow>, bytes:usize, charge:StateCharge,
}
impl FamilyWriter {
    fn new(path:PathBuf,budget:&ResourceBudget)->Result<Self,ModelError>{
        let writer=FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?,&schema()).map_err(ModelError::codec)?;
        Ok(Self {path,writer,pending:vec![],bytes:0,charge:StateCharge::new(budget,"graph-artifact-buffer")})
    }
    fn push(&mut self,row:PendingRow)->Result<(),ModelError>{
        if row.payload.len()>64<<20 {return Err(ModelError::Invalid("graph element exceeds artifact transport limit".into()));}
        if !self.pending.is_empty() && (self.pending.len()>=4096 || self.bytes.saturating_add(row.payload.len())>1<<20) {self.flush()?;}
        self.charge.grow(row.payload.len().saturating_add(size_of::<PendingRow>()))?;
        self.bytes=self.bytes.saturating_add(row.payload.len());self.pending.push(row);Ok(())
    }
    fn flush(&mut self)->Result<(),ModelError>{
        if self.pending.is_empty(){return Ok(());}
        let _encoding=self.charge.budget().expect("bound artifact budget").reserve("graph-artifact-encoding",self.bytes.saturating_add(self.pending.len()*96))?;
        let ids=FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r|r.id.0)).map_err(ModelError::codec)?;
        let contents=FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r|r.content.0)).map_err(ModelError::codec)?;
        let payloads=BinaryArray::from_iter_values(self.pending.iter().map(|r|r.payload.as_slice()));
        let kinds=Int16Array::from(self.pending.iter().map(|r|r.kind).collect::<Vec<_>>());
        let lengths=UInt64Array::from(self.pending.iter().map(|r|r.source_length).collect::<Vec<_>>());
        let subtypes=Int16Array::from(self.pending.iter().map(|r|r.subtype).collect::<Vec<_>>());
        self.writer.write(&RecordBatch::try_new(schema(),vec![Arc::new(ids),Arc::new(contents),Arc::new(payloads),Arc::new(kinds),Arc::new(lengths),Arc::new(subtypes)]).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        self.pending=Vec::new();self.charge.release(self.charge.reserved());self.bytes=0;Ok(())
    }
    fn finish(mut self)->Result<PathBuf,ModelError>{self.flush()?;self.writer.finish().map_err(ModelError::codec)?;self.writer.into_inner().map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;Ok(self.path)}
}
/// Construction remains private until producer completeness and semantic closure have succeeded.
struct GraphBuilder {
    directory:tempfile::TempDir,entities:FamilyWriter,assertions:FamilyWriter,references:ReferenceWriter,derivations:Vec<GraphDerivation>,derivation_charge:StateCharge,
}
impl GraphBuilder {
    fn new(budget:&ResourceBudget)->Result<Self,ModelError>{
        let directory=tempfile::tempdir().map_err(ModelError::codec)?;
        let entities=FamilyWriter::new(directory.path().join("entities-pending.arrow"),budget)?;
        let assertions=FamilyWriter::new(directory.path().join("assertions-pending.arrow"),budget)?;
        let references=ReferenceWriter::new(&directory.path().join("references-pending.arrow"),budget)?;
        Ok(Self {directory,entities,assertions,references,derivations:vec![],derivation_charge:StateCharge::new(budget,"graph-derivation-topology")})
    }
    fn derivation(&mut self,value:Option<GraphDerivation>)->Result<(),ModelError>{
        if let Some(value)=value {self.derivation_charge.grow(size_of::<GraphDerivation>().saturating_add(value.premises.len().saturating_mul(size_of::<Target>()+128)).saturating_add(256))?;self.derivations.push(value);}
        Ok(())
    }
    fn entity_references(&mut self,value:&Entity)->Result<(),ModelError>{
        value.validate()?;
        self.derivation(value.derivation()?)?;
        for requirement in value.reference_requirements()? {self.references.push(requirement.target,requirement.kind,requirement.subtype,None)?;}
        if let Entity::Occurrence(occurrence)=value {
            self.references.push(Target::Entity(EntityId::of(occurrence.source)),Some(EntityKind::Source),None,Some(occurrence.end as u64))?;
        }
        if let Entity::Evidence(lctx_model::domain::assertion::Evidence::SourceSpan {source,end,..})=value {
            self.references.push(Target::Entity(EntityId::of(*source)),Some(EntityKind::Source),None,Some(*end as u64))?;
        }
        Ok(())
    }
    fn entity(&mut self,value:&Entity)->Result<(),ModelError>{
        self.entity_references(value)?;
        self.entities.push(PendingRow {id:value.id().0,content:value.content(),payload:serde_json::to_vec(value).map_err(ModelError::codec)?,kind:value.kind() as i16,
            source_length:if let Entity::Source(source)=value {Some(source.byte_len as u64)}else{None},subtype:value.subtype()})
    }
    fn assertion_references(&mut self,value:&Assertion)->Result<(),ModelError>{
        value.validate()?;
        self.derivation(value.declared_derivation())?;
        for requirement in value.reference_requirements()? {self.references.push(requirement.target,requirement.kind,requirement.subtype,None)?;}
        Ok(())
    }
    fn assertion(&mut self,value:&Assertion)->Result<(),ModelError>{
        self.assertion_references(value)?;
        self.assertions.push(PendingRow {id:value.id().0,content:value.content(),payload:serde_json::to_vec(value).map_err(ModelError::codec)?,kind:value.kind as i16,source_length:None,subtype:None})
    }
}
async fn order(context:&SessionContext,pending:&Path,output:&Path,family:GraphFamily,budget:&ResourceBudget)->Result<FamilyContent,ModelError>{
    let name=match family {GraphFamily::Entities=>"pending_entities",GraphFamily::Assertions=>"pending_assertions",_=>return Err(ModelError::Invalid("unsupported graph stream".into()))};
    context.register_arrow(name,pending.to_string_lossy(),ArrowReadOptions::default().schema(&schema())).await.map_err(ModelError::codec)?;
    let result=async {
        let mut stream=crate::sql::query(&context,&format!("SELECT * FROM {name} ORDER BY id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        let mut writer=FileWriter::try_new(File::create(output).map_err(ModelError::codec)?,&schema()).map_err(ModelError::codec)?;
        let mut hasher=FamilyHasher::new(family);
        let mut previous_payload:Option<Vec<u8>>=None;
        let mut previous_charge=budget.reserve("graph-artifact-previous-payload",0)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
            let _ordered_copy=budget.reserve("graph-artifact-ordered-copy",lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(2))?;
            let mut selected=Vec::new();
            let ids=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph"))?;
            let contents=batch.column(1).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph"))?;
            let payloads=batch.column(2).as_any().downcast_ref::<BinaryArray>().ok_or(ModelError::Schema("graph"))?;
            for index in 0..batch.num_rows(){
                let id=ContentHash(ids.value(index).try_into().map_err(ModelError::codec)?);
                let content=ContentHash(contents.value(index).try_into().map_err(ModelError::codec)?);
                if hasher.push(id,content)? {selected.push(index as u32);previous_charge.try_resize(payloads.value(index).len())?;previous_payload=Some(payloads.value(index).to_vec());}
                else if previous_payload.as_deref()!=Some(payloads.value(index)){return Err(ModelError::Conflict("graph canonical payload"));}
            }
            if !selected.is_empty() {
                let indices=UInt32Array::from(selected);
                let columns=batch.columns().iter().map(|c|take(c.as_ref(), &indices, None).map_err(ModelError::codec)).collect::<Result<Vec<_>,_>>()?;
                writer.write(&RecordBatch::try_new(schema(),columns).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
            }
        }
        writer.finish().map_err(ModelError::codec)?;writer.into_inner().map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;
        Ok(hasher.finish())
    }.await;
    context.deregister_table(name).map_err(ModelError::codec)?;result
}
/// Immutable admitted content. The constructor is private: callers cannot label pending output admitted.
pub struct AdmittedArtifact {directory:tempfile::TempDir,manifest:Manifest,_manifest_charge:StateCharge}
impl AdmittedArtifact {
    pub fn manifest(&self)->&Manifest {&self.manifest}
    /// Verify a transported copy against this compiler-owned admitted content. This does not
    /// promote arbitrary files to admitted output or rerun semantic producers.
    pub async fn verify_export(&self,path:&Path,workspace:&Arc<Workspace>)->Result<(),ModelError>{
        let manifest:Manifest=serde_json::from_slice(&std::fs::read(path.join("manifest.json")).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        manifest.validate()?;
        if manifest.semantic_contract!=semantic_contract(workspace.model()) {return Err(ModelError::Conflict("artifact semantic contract"));}
        if manifest!=self.manifest {return Err(ModelError::Conflict("artifact manifest"));}
        let mut check=GraphBuilder::new(workspace.budget())?;
        for (file,family) in [("entities.arrow",GraphFamily::Entities),("assertions.arrow",GraphFamily::Assertions)] {
            let reader=datafusion::arrow::ipc::reader::FileReader::try_new(File::open(path.join(file)).map_err(ModelError::codec)?,None).map_err(ModelError::codec)?;
            if reader.schema()!=schema(){return Err(ModelError::Schema("graph artifact"));}
            let mut content=FamilyHasher::new(family);
            for batch in reader {
                workspace.cancellation().check()?;
                let batch=batch.map_err(ModelError::codec)?;
                let _decode=workspace.budget().reserve("graph-artifact-transport-decode",lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(4))?;
                let ids=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph artifact"))?;
                let hashes=batch.column(1).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph artifact"))?;
                let payloads=batch.column(2).as_any().downcast_ref::<BinaryArray>().ok_or(ModelError::Schema("graph artifact"))?;
                let kinds=batch.column(3).as_any().downcast_ref::<Int16Array>().ok_or(ModelError::Schema("graph artifact"))?;
                let lengths=batch.column(4).as_any().downcast_ref::<UInt64Array>().ok_or(ModelError::Schema("graph artifact"))?;
                let subtypes=batch.column(5).as_any().downcast_ref::<Int16Array>().ok_or(ModelError::Schema("graph artifact"))?;
                if batch.columns()[..4].iter().any(|column|column.null_count()!=0) {return Err(ModelError::Schema("graph artifact"));}
                for row in 0..batch.num_rows(){
                    let bytes=payloads.value(row);
                    if bytes.len()>64<<20 {return Err(ModelError::Invalid("graph element exceeds artifact transport limit".into()));}
                    let (id,hash,kind,length,subtype)=if family==GraphFamily::Entities {
                        let value:Entity=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                        let length=if let Entity::Source(source)=&value {Some(source.byte_len as u64)}else{None};
                        let metadata=(value.id().0,value.content(),value.kind() as i16,length,value.subtype());check.entity_references(&value)?;metadata
                    }else{
                        let value:Assertion=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                        let metadata=(value.id().0,value.content(),value.kind as i16,None,None);check.assertion_references(&value)?;metadata
                    };
                    if ids.value(row)!=id.0 || hashes.value(row)!=hash.0 || kinds.value(row)!=kind || (!lengths.is_null(row)).then(||lengths.value(row))!=length || (!subtypes.is_null(row)).then(||subtypes.value(row))!=subtype {return Err(ModelError::Conflict("artifact element metadata"));}
                    if !content.push(id,hash)? {return Err(ModelError::Invalid("artifact stream contains duplicate element".into()));}
                }
            }
            let content=content.finish();
            if !manifest.families.contains(&content){return Err(ModelError::Conflict("artifact graph family"));}
        }
        admit_graph_derivations(&check.derivations)?;
        let GraphBuilder {directory,entities,assertions,references,..}=check;
        entities.finish()?;assertions.finish()?;references.finish()?;
        let base=workspace.inputs("artifact-transport-verification",manifest.profile,[])?.session(workspace).await?;
        let context=SessionContext::new_with_config_rt(base.copied_config().set_bool("datafusion.optimizer.prefer_hash_join",false),base.runtime_env());
        reference_closure(&context,&path.join("entities.arrow"),&path.join("assertions.arrow"),&directory.path().join("references-pending.arrow")).await?;
        use std::io::Read;
        let _copy=workspace.budget().reserve("artifact-original-verification",65536)?;let mut buffer=vec![0u8;65536];
        for original in &manifest.originals {
            let mut input=File::open(path.join(format!("original-{}.bin",original.source.0.hex()))).map_err(ModelError::codec)?;
            let mut content=lctx_model::domain::ContentHasher::default();let mut length=0u64;
            loop {workspace.cancellation().check()?;let count=input.read(&mut buffer).map_err(ModelError::codec)?;if count==0{break;}content.update(&buffer[..count]);length+=count as u64;}
            if length!=original.byte_len || content.finish()!=original.content {return Err(ModelError::Conflict("artifact original bytes"));}
        }
        Ok(())
    }
    /// Create a fresh destination using private staging on its filesystem. Never overwrite an existing artifact.
    pub fn export(&self,destination:&Path)->Result<(),ModelError>{
        if destination.exists(){return Err(ModelError::Invalid("artifact destination already exists".into()));}
        let parent=destination.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(ModelError::codec)?;
        let staged=tempfile::Builder::new().prefix(".lctx-artifact-").tempdir_in(parent).map_err(ModelError::codec)?;
        for entry in std::fs::read_dir(self.directory.path()).map_err(ModelError::codec)? {
            let entry=entry.map_err(ModelError::codec)?;
            if entry.file_type().map_err(ModelError::codec)?.is_file(){let path=staged.path().join(entry.file_name());std::fs::copy(entry.path(),&path).map_err(ModelError::codec)?;File::open(path).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;}
        }
        let manifest_file=staged.path().join("manifest.json");
        std::fs::write(&manifest_file,serde_json::to_vec_pretty(&self.manifest).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        File::open(&manifest_file).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;
        // rename_noreplace is not portable: create the destination ownership first, then move files.
        // A refused ownership race preserves both the private artifact and the other destination.
        std::fs::create_dir(destination).map_err(ModelError::codec)?;
        let mut installed_paths=Vec::new();
        let installed=(||{
            for entry in std::fs::read_dir(staged.path()).map_err(ModelError::codec)? {
                let entry=entry.map_err(ModelError::codec)?;
                if entry.file_name()=="manifest.json" {continue;}
                let path=destination.join(entry.file_name());
                std::fs::hard_link(entry.path(),&path).map_err(ModelError::codec)?;installed_paths.push(path);
            }
            let marker=destination.join("manifest.json");
            std::fs::hard_link(&manifest_file,&marker).map_err(ModelError::codec)?;installed_paths.push(marker);
            File::open(destination).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;Ok(())
        })();
        if installed.is_err(){for path in installed_paths {let _=std::fs::remove_file(path);}let _=std::fs::remove_dir(destination);}
        installed
    }
}

fn reference_schema()->Arc<Schema>{Arc::new(Schema::new(vec![
    Field::new("target",DataType::FixedSizeBinary(32),false),Field::new("namespace",DataType::Int16,false),
    Field::new("expected_kind",DataType::Int16,true),Field::new("expected_subtype",DataType::Int16,true),Field::new("max_end",DataType::UInt64,true),
]))}
struct ReferenceRow {target:ContentHash,namespace:i16,kind:Option<i16>,subtype:Option<i16>,max_end:Option<u64>}
struct ReferenceWriter {writer:FileWriter<File>,pending:Vec<ReferenceRow>,charge:StateCharge}
impl ReferenceWriter {
    fn new(path:&Path,budget:&ResourceBudget)->Result<Self,ModelError>{Ok(Self{writer:FileWriter::try_new(File::create(path).map_err(ModelError::codec)?,&reference_schema()).map_err(ModelError::codec)?,pending:vec![],charge:StateCharge::new(budget,"graph-artifact-references")})}
    fn push(&mut self,target:Target,kind:Option<EntityKind>,subtype:Option<i16>,max_end:Option<u64>)->Result<(),ModelError>{
        self.charge.grow(size_of::<ReferenceRow>())?;
        match target {
            Target::Entity(id)=>self.pending.push(ReferenceRow {target:id.0,namespace:0,kind:kind.map(|k|k as i16),subtype,max_end}),
            Target::Assertion(id)=>self.pending.push(ReferenceRow {target:id.0,namespace:1,kind:None,subtype:None,max_end:None}),
            Target::External {provider,context,..}=>{
                self.charge.release(size_of::<ReferenceRow>());
                self.push(Target::Entity(provider),Some(EntityKind::Provider),None,None)?;
                self.push(Target::Entity(context),Some(EntityKind::Context),None,None)?;
            }
        }
        if self.pending.len()>=4096 {self.flush()?;}Ok(())
    }
    fn flush(&mut self)->Result<(),ModelError>{
        if self.pending.is_empty(){return Ok(());}
        let _encoding=self.charge.budget().expect("bound artifact budget").reserve("graph-artifact-reference-encoding",self.pending.len()*64)?;
        let targets=FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r|r.target.0)).map_err(ModelError::codec)?;
        let namespaces=Int16Array::from(self.pending.iter().map(|r|r.namespace).collect::<Vec<_>>());
        let kinds=Int16Array::from(self.pending.iter().map(|r|r.kind).collect::<Vec<_>>());
        let subtypes=Int16Array::from(self.pending.iter().map(|r|r.subtype).collect::<Vec<_>>());
        let ends=UInt64Array::from(self.pending.iter().map(|r|r.max_end).collect::<Vec<_>>());
        self.writer.write(&RecordBatch::try_new(reference_schema(),vec![Arc::new(targets),Arc::new(namespaces),Arc::new(kinds),Arc::new(subtypes),Arc::new(ends)]).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        self.pending=Vec::new();self.charge.release(self.charge.reserved());Ok(())
    }
    fn finish(mut self)->Result<(),ModelError>{self.flush()?;self.writer.finish().map_err(ModelError::codec)?;Ok(())}
}
async fn reference_closure(context:&SessionContext,entities:&Path,assertions:&Path,references:&Path)->Result<(),ModelError>{
    context.register_arrow("graph_entities",entities.to_string_lossy(),ArrowReadOptions::default().schema(&schema())).await.map_err(ModelError::codec)?;
    context.register_arrow("graph_assertions",assertions.to_string_lossy(),ArrowReadOptions::default().schema(&schema())).await.map_err(ModelError::codec)?;
    context.register_arrow("graph_references",references.to_string_lossy(),ArrowReadOptions::default().schema(&reference_schema())).await.map_err(ModelError::codec)?;
    let result=async{
        let sql="WITH targets AS (SELECT id, 0 AS namespace, kind, subtype, source_length FROM graph_entities UNION ALL SELECT id, 1 AS namespace, NULL AS kind, NULL AS subtype, NULL AS source_length FROM graph_assertions) SELECT r.target,r.namespace,r.expected_kind,r.expected_subtype,t.kind,t.subtype,r.max_end,t.source_length FROM graph_references r LEFT JOIN targets t ON r.target=t.id AND r.namespace=t.namespace WHERE t.id IS NULL OR (r.expected_kind IS NOT NULL AND (t.kind IS NULL OR r.expected_kind<>t.kind)) OR (r.expected_subtype IS NOT NULL AND (t.subtype IS NULL OR r.expected_subtype<>t.subtype)) OR (r.max_end IS NOT NULL AND (t.source_length IS NULL OR r.max_end>t.source_length)) LIMIT 1";
        let mut stream=crate::sql::query(&context,sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows()!=0 {
                let target=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph reference"))?;
                let namespace=batch.column(1).as_any().downcast_ref::<Int16Array>().ok_or(ModelError::Schema("graph reference"))?;
                return Err(ModelError::Invalid(format!("graph reference closure, nominal role or original span failed: {} namespace {}, expected kind/subtype {:?}/{:?}, actual {:?}/{:?}, end/length {:?}/{:?}",ContentHash(target.value(0).try_into().map_err(ModelError::codec)?).hex(),namespace.value(0),batch.column(2),batch.column(3),batch.column(4),batch.column(5),batch.column(6),batch.column(7))));
            }
        }
        Ok(())
    }.await;
    for name in ["graph_entities","graph_assertions","graph_references"]{context.deregister_table(name).map_err(ModelError::codec)?;}
    result
}

/// Lower completed semantic owners into a small number of typed graph streams and admit them.
/// Native publication consumes this immutable result; it does not supply compiler working memory.
pub async fn admit(
    workspace:&Arc<Workspace>,captured:&cpg_extract::bundle::CapturedInputs,
    frontier:lctx_model::domain::admission::Frontier,profile:lctx_model::domain::stages::Profile,
    settings:ContentHash,
)->Result<AdmittedArtifact,ModelError>{
    use lctx_model::domain::{Record, ContentHasher};
    use std::io::{Read, Write};
    workspace.cancellation().check()?;
    workspace.require_compilation(captured,frontier,profile,settings)?;
    // Coverage has an independent obligation universe derived from acquired artifacts and profile.
    // Merely having a collection of well-shaped graph records is insufficient.
    workspace.facts_availability(profile)?;
    let mut builder=GraphBuilder::new(workspace.budget())?;
    macro_rules! emit_entities {
        ($($variant:ident: $record:path),* $(,)?)=>{$(
            if let Ok(source)=workspace.completed::<$record>() {
                for batch in source.read::<$record>(workspace.model().clone(),workspace.budget().clone())? {
                    workspace.cancellation().check()?;
                    for row in batch?.rows() {builder.entity(&Entity::from(row.clone()))?;}
                }
            }
        )*};
    }
    macro_rules! emit_assertions {
        ($($variant:ident: $record:path),* $(,)?)=>{$(
            if let Ok(source)=workspace.completed::<$record>() {
                for batch in source.read::<$record>(workspace.model().clone(),workspace.budget().clone())? {
                    workspace.cancellation().check()?;
                    for row in batch?.rows() {builder.assertion(&Assertion::from_record(row.clone())?)?;}
                }
            }
        )*};
    }
    lctx_model::graph_entity_records!(emit_entities);
    lctx_model::graph_assertion_records!(emit_assertions);
    admit_graph_derivations(&builder.derivations)?;
    let GraphBuilder {directory,entities,assertions,references,..}=builder;
    let pending_entities=entities.finish()?;
    let pending_assertions=assertions.finish()?;
    references.finish()?;
    let entities_path=directory.path().join("entities.arrow");
    let assertions_path=directory.path().join("assertions.arrow");
    let base=workspace.inputs("graph-admission",profile,[])?.session(workspace).await?;
    // Merge joins operate on ordered streams and spillable sorting, avoiding full resident indexes.
    let context=SessionContext::new_with_config_rt(base.copied_config().set_bool("datafusion.optimizer.prefer_hash_join",false),base.runtime_env());
    let entities_content=order(&context,&pending_entities,&entities_path,GraphFamily::Entities,workspace.budget()).await?;
    let assertions_content=order(&context,&pending_assertions,&assertions_path,GraphFamily::Assertions,workspace.budget()).await?;
    reference_closure(&context,&entities_path,&assertions_path,&directory.path().join("references-pending.arrow")).await?;
    let mut captures=Vec::new();let mut originals=Vec::new();
    let _buffer_charge=workspace.budget().reserve("artifact-original-copy",65536)?;
    let mut buffer=vec![0u8;65536];
    for acquired in captured.inputs() {
        let frozen=acquired.captured();
        captures.push(EntityId::of(frozen.revision().id()));
        for source in frozen.artifacts() {
            let id=EntityId::of(source.id());
            let mut input=File::open(frozen.root().join(&source.path)).map_err(ModelError::codec)?;
            let mut output=File::create(directory.path().join(format!("original-{}.bin",id.0.hex()))).map_err(ModelError::codec)?;
            let mut content=ContentHasher::default();let mut byte_len=0u64;
            loop {
                workspace.cancellation().check()?;
                let count=input.read(&mut buffer).map_err(ModelError::codec)?;
                if count==0 {break;}
                output.write_all(&buffer[..count]).map_err(ModelError::codec)?;
                content.update(&buffer[..count]);byte_len+=count as u64;
            }
            if content.finish()!=source.content || byte_len!=source.byte_len as u64 {
                return Err(ModelError::Invalid("original bytes changed after capture".into()));
            }
            output.sync_all().map_err(ModelError::codec)?;
            originals.push(Original {source:id,content:source.content,byte_len});
        }
    }
    captures.sort();captures.dedup();originals.sort_by_key(|o|o.source);originals.dedup();
    let (manifest,manifest_charge)=crate::artifact_manifest::populate(workspace,frontier,profile,settings,captures,
        originals,vec![entities_content,assertions_content]).await?;
    manifest.validate()?;
    for path in [pending_entities,pending_assertions,directory.path().join("references-pending.arrow")] {
        std::fs::remove_file(path).map_err(ModelError::codec)?;
    }
    workspace.cancellation().check()?;
    Ok(AdmittedArtifact {directory,manifest,_manifest_charge:manifest_charge})
}
