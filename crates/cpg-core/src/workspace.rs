//! Private spillable compilation workspace. Completed inputs are immutable IPC streams.
//!
//! Producers write bounded batches. A completed relation is globally ordered and deduplicated
//! before it becomes visible; interrupted writes never enter the completed-input map.
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, UInt32Array};
use datafusion::{
    arrow::{compute::take, ipc::{reader::FileReader, writer::FileWriter}},
    execution::{disk_manager::{DiskManagerBuilder, DiskManagerMode}, runtime_env::RuntimeEnvBuilder, memory_pool::{MemoryConsumer, MemoryPool, MemoryReservation}},
    execution::options::ArrowReadOptions,
    prelude::{SessionConfig, SessionContext},
};
use futures::TryStreamExt;
use lctx_model::domain::{Batch, ContentHash, ModelError, Record, Relation, ValidatedModel,
    batching::TransferLimits, charged::StateCharge, resources::{ResourceBudget, ResourcePool, Reservation}, stages::{Profile, ProviderOutcome}};
use std::{any::Any, collections::BTreeMap, fs::File, path::{Path, PathBuf}, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}}};

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceOptions {
    pub memory_bytes: usize,
    pub partitions: usize,
    pub batch_rows: usize,
}
impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self { memory_bytes: 256 << 20, partitions: 4, batch_rows: 4096 }
    }
}
/// Cancellation is cooperative between bounded I/O/compute batches. Owned tasks must be drained
/// by their caller before releasing the workspace.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) { self.0.store(true, Ordering::Release); }
    pub fn check(&self) -> Result<(), ModelError> {
        if self.0.load(Ordering::Acquire) { Err(ModelError::Invalid("compilation cancelled".into())) } else { Ok(()) }
    }
}
struct WorkspaceFiles { directory: tempfile::TempDir }
/// The attempt owns one runtime, spill directory, buffer budget, and completed relation registry.
pub struct Workspace {
    files: Arc<WorkspaceFiles>,
    context: SessionContext,
    options: WorkspaceOptions,
    budget: ResourceBudget,
    model: Arc<ValidatedModel>,
    completed: Mutex<BTreeMap<&'static str, Arc<CompletedRelation>>>,
    next: AtomicU64,
    cancellation: Cancellation,
    completion_gate: tokio::sync::Mutex<()>,
}
impl Workspace {
    pub fn new(model: Arc<ValidatedModel>, options: WorkspaceOptions) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 || options.batch_rows == 0 {
            return Err(ModelError::Invalid("workspace sizes must be positive".into()));
        }
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_limit(options.memory_bytes, 1.0)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(DiskManagerMode::Directories(vec![directory.path().to_path_buf()])))
            .build_arc().map_err(ModelError::codec)?;
        let config = SessionConfig::new().with_target_partitions(options.partitions).with_batch_size(options.batch_rows)
            .set_usize("datafusion.execution.sort_spill_reservation_bytes", (options.memory_bytes / 16).min(10 << 20))
            .set_usize("datafusion.execution.sort_in_place_threshold_bytes", (options.memory_bytes / 8).min(1 << 20));
        Ok(Arc::new(Self {
            files: Arc::new(WorkspaceFiles { directory }),
            context: SessionContext::new_with_config_rt(config, runtime.clone()),
            options,
            budget: ResourceBudget::from_pool(Arc::new(WorkspacePool { memory: runtime.memory_pool.clone(), limit: options.memory_bytes }))?,
            model,
            completed: Mutex::default(),
            next: AtomicU64::new(0),
            cancellation: Cancellation::default(),
            completion_gate: tokio::sync::Mutex::new(()),
        }))
    }
    /// Reuse a captured native configuration's pool. DataFusion allocations and typed retained
    /// state draw from this same ceiling; the custom pool adapts allocations rather than copying.
    pub fn with_budget(model: Arc<ValidatedModel>, options: WorkspaceOptions, budget: ResourceBudget) -> Result<Arc<Self>,ModelError> {
        if options.memory_bytes!=budget.limit() {return Err(ModelError::Invalid("workspace memory differs from supplied pool".into()));}
        let mut workspace=Self::new(model,options)?;
        let owner=Arc::get_mut(&mut workspace).expect("new workspace has one owner");
        let pool=Arc::new(BudgetMemoryPool {budget:budget.clone(),allocated:Mutex::new(budget.reserve("workspace-datafusion",0)?)});
        let runtime=RuntimeEnvBuilder::new().with_memory_pool(pool)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(DiskManagerMode::Directories(vec![owner.files.directory.path().to_path_buf()])))
            .build_arc().map_err(ModelError::codec)?;
        owner.context=SessionContext::new_with_config_rt(owner.context.copied_config(),runtime);
        owner.budget=budget;
        Ok(workspace)
    }
    fn coverage_rows<R:Record>(&self)->Result<(Vec<R>,StateCharge),ModelError> {
        let mut rows=Vec::new();
        let mut charge=StateCharge::new(&self.budget,"facts-coverage-input");
        for batch in self.completed::<R>()?.read::<R>(self.model.clone(),self.budget.clone())? {
            for row in batch?.rows() {charge.grow(row.row_bytes().saturating_add(size_of::<R>()))?;rows.push(row.clone());}
        }
        Ok((rows,charge))
    }
    pub fn facts_availability(&self,profile:Profile)->Result<lctx_model::domain::admission::ScopedAvailability,ModelError> {
        use lctx_model::domain::{admission::{FrontierContract,Expected,ScopedAvailability},input::{InputRevision,ArtifactUse},source::{SourceArtifact,CoverageScope},attribution::ProviderCoverage,stages::Schedule};
        let providers=crate::facts::providers(ContentHash::of(b"facts-coverage-contract"));
        let schedule=Schedule::build(&self.model,providers.iter().map(|p|p.declaration(profile)).collect(),&[],profile)?;
        let contract=FrontierContract::facts(&self.model,profile)?.preflight(&schedule)?;
        let (inputs,_inputs)=self.coverage_rows::<InputRevision>()?;
        let (artifacts,_artifacts)=self.coverage_rows::<SourceArtifact>()?;
        let (uses,_uses)=self.coverage_rows::<ArtifactUse>()?;
        let expected=contract.expected_coverage(&inputs,&artifacts,&uses)?.into_keys().collect::<std::collections::BTreeSet<Expected>>();
        let (scope_rows,_scopes)=self.coverage_rows::<CoverageScope>()?;
        let scopes=scope_rows.into_iter().map(|r|(r.id(),r)).collect();
        let (rows,_rows)=self.coverage_rows::<ProviderCoverage>()?;
        ScopedAvailability::from_completed(profile,&expected,&rows,&scopes,&self.budget)
    }
    /// Semantic content over the actual completed typed streams, independent of IPC bytes.
    pub fn content(&self)->Result<ContentHash,ModelError> {
        let mut sink=lctx_model::domain::KeySink::new("compiler-workspace-content/v1");
        for relation in self.completed_relations()? {sink.part(relation.name().as_bytes(),&relation.content().0);}
        Ok(sink.finish())
    }
    /// Reuse the model's ordered invariant state machines with spillable SQL ordering. No global
    /// graph concatenation or resident copy of all completed relations is constructed.
    pub async fn validate(&self)->Result<ContentHash,ModelError> {
        let relations=self.completed_relations()?;
        let names=relations.iter().map(|r|r.name()).collect::<std::collections::BTreeSet<_>>();
        let inputs=self.inputs("artifact-admission",Profile::Catalog,names.iter().copied())?;
        let session=inputs.session(self).await?;
        self.validate_references(&session,&relations).await?;
        for invariant in self.model.invariants_for_scope(&names)? {
            let mut check=(invariant.create)(self.budget());
            for input in &invariant.inputs {
                let order=input.order().iter().map(|name|format!("\"{name}\"")).collect::<Vec<_>>().join(",");
                let sql=format!("SELECT * FROM \"{}\"{}",input.name(),if order.is_empty(){String::new()}else{format!(" ORDER BY {order}")});
                let mut stream=session.sql(&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {self.cancellation.check()?;check.visit_input(input,&batch)?;}
            }
            check.finish()?;
        }
        self.content()
    }
    /// Explicit fixture replay for selected owner source/coverage contracts. Artifact admission
    /// does not automatically recompute every compiler algorithm.
    pub async fn validate_publications(&self,ids:&[&str])->Result<(),ModelError> {
        let relations=self.completed_relations()?;
        let inputs=self.inputs("selected-publication-controls",Profile::Catalog,relations.iter().map(|r|r.name()))?;
        let session=inputs.session(self).await?;
        let mut checks=BTreeMap::new();
        for source in &relations {
            for id in source.relation.publication_refs().iter().filter(|id|ids.contains(id)) {
                if let Some(previous)=checks.insert(*id,source.clone()) {
                    if previous.producer!=source.producer || previous.inputs!=source.inputs {
                        return Err(ModelError::Invalid("publication invariant spans incompatible producer inputs".into()));
                    }
                }
            }
        }
        if ids.iter().any(|id|!checks.contains_key(id)) {return Err(ModelError::Invalid("selected publication control is absent".into()));}
        for (id,source) in checks {
            let invariant=self.model.publication_check(id)?;
            let mut check=(invariant.create)(self.budget());
            for input in &invariant.inputs {
                let order=input.order().iter().map(|name|format!("\"{name}\"")).collect::<Vec<_>>().join(",");
                let sql=format!("SELECT * FROM \"{}\"{}",input.name(),if order.is_empty(){String::new()}else{format!(" ORDER BY {order}")});
                let mut stream=session.sql(&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {self.cancellation.check()?;check.visit_input(input,&batch)?;}
            }
            check.finish(&source.inputs,source.profile)?;
        }
        Ok(())
    }
    async fn validate_references(&self,session:&SessionContext,relations:&[Arc<CompletedRelation>])->Result<(),ModelError> {
        use arrow_array::{builder::{StringBuilder,FixedSizeBinaryBuilder,Int16Builder},Int16Array};
        use arrow_schema::{Schema,Field,DataType};
        let target_schema=Arc::new(Schema::new(vec![Field::new("relation",DataType::Utf8,false),Field::new("id",DataType::FixedSizeBinary(16),false),Field::new("subtype",DataType::Int16,true)]));
        let refs_schema=Arc::new(Schema::new(vec![Field::new("source",DataType::Utf8,false),Field::new("field",DataType::Utf8,false),Field::new("target",DataType::Utf8,false),Field::new("id",DataType::FixedSizeBinary(16),false),Field::new("subtype",DataType::Int16,true)]));
        let target_path=self.path("nominal-keys","validation");
        let refs_path=self.path("nominal-references","validation");
        let mut targets=FileWriter::try_new(File::create(&target_path).map_err(ModelError::codec)?,&target_schema).map_err(ModelError::codec)?;
        let mut references=FileWriter::try_new(File::create(&refs_path).map_err(ModelError::codec)?,&refs_schema).map_err(ModelError::codec)?;
        let mut source_names=StringBuilder::new();let mut fields=StringBuilder::new();let mut target_names=StringBuilder::new();
        let mut ids=FixedSizeBinaryBuilder::new(16);let mut tags=Int16Builder::new();let mut count=0;
        let _buffer=self.budget.reserve("nominal-reference-transfer",self.options.batch_rows.saturating_mul(1024))?;
        macro_rules! flush_refs {()=>{{
            if count>0 {
                let batch=RecordBatch::try_new(refs_schema.clone(),vec![Arc::new(source_names.finish()),Arc::new(fields.finish()),Arc::new(target_names.finish()),Arc::new(ids.finish()),Arc::new(tags.finish())]).map_err(ModelError::codec)?;
                references.write(&batch).map_err(ModelError::codec)?;count=0;
            }
        }};}
        // Each completed batch is decoded once for its namespace keys and declared nominal fields.
        for source in relations {
            for batch in source.batches()? {
                self.cancellation.check()?;
                let batch=batch.map_err(ModelError::codec)?;
                let _input=self.budget.reserve("nominal-reference-input",batch.get_array_memory_size())?;
                let row_ids=batch.column_by_name("id").and_then(|c|c.as_any().downcast_ref::<FixedSizeBinaryArray>()).ok_or(ModelError::Schema(source.name()))?;
                let subtype=source.relation.sum().map(|sum|batch.column_by_name(sum.tag).and_then(|c|c.as_any().downcast_ref::<Int16Array>()).ok_or(ModelError::Schema(source.name()))).transpose()?;
                let mut names=StringBuilder::new();let mut key_ids=FixedSizeBinaryBuilder::new(16);let mut key_tags=Int16Builder::new();
                for row in 0..batch.num_rows() {names.append_value(source.name());key_ids.append_value(row_ids.value(row)).map_err(ModelError::codec)?;if let Some(tags)=subtype {key_tags.append_value(tags.value(row));}else{key_tags.append_null();}}
                targets.write(&RecordBatch::try_new(target_schema.clone(),vec![Arc::new(names.finish()),Arc::new(key_ids.finish()),Arc::new(key_tags.finish())]).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
                for field in source.relation.fields().iter().filter(|f|!f.list()) {
                    let Some((_,target))=field.target() else {continue;};
                    let column=batch.column_by_name(field.name()).and_then(|c|c.as_any().downcast_ref::<FixedSizeBinaryArray>()).ok_or(ModelError::Schema(source.name()))?;
                    for row in 0..batch.num_rows() {
                        if column.is_null(row) {continue;}
                        source_names.append_value(source.name());fields.append_value(field.name());target_names.append_value(target);ids.append_value(column.value(row)).map_err(ModelError::codec)?;
                        if let Some(tag)=field.subtype(){tags.append_value(tag);}else{tags.append_null();}count+=1;
                        if count>=self.options.batch_rows {flush_refs!();}
                    }
                }
            }
        }
        flush_refs!();debug_assert_eq!(count,0);targets.finish().map_err(ModelError::codec)?;references.finish().map_err(ModelError::codec)?;
        drop(targets);drop(references);drop(_buffer);
        session.register_arrow("_nominal_targets",target_path.to_string_lossy(),ArrowReadOptions::default().schema(&target_schema)).await.map_err(ModelError::codec)?;
        session.register_arrow("_nominal_references",refs_path.to_string_lossy(),ArrowReadOptions::default().schema(&refs_schema)).await.map_err(ModelError::codec)?;
        let mut stream=session.sql("SELECT r.source,r.field,r.target FROM _nominal_references r LEFT JOIN _nominal_targets t ON r.target=t.relation AND r.id=t.id WHERE t.id IS NULL OR (r.subtype IS NOT NULL AND (t.subtype IS NULL OR r.subtype<>t.subtype)) LIMIT 1").await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows()>0 {return Err(ModelError::Invalid(format!("missing or wrong-subtype nominal reference: {batch:?}")));}
        }
        Ok(())
    }
    pub fn budget(&self) -> &ResourceBudget { &self.budget }
    pub fn model(&self) -> &Arc<ValidatedModel> { &self.model }
    pub fn cancellation(&self) -> Cancellation { self.cancellation.clone() }
    fn path(&self, relation: &str, suffix: &str) -> PathBuf {
        self.files.directory.path().join(format!("{}-{relation}-{suffix}.arrow", self.next.fetch_add(1, Ordering::Relaxed)))
    }
    pub fn completed<R: Record>(&self) -> Result<Arc<CompletedRelation>, ModelError> {
        self.relation(R::NAME)
    }
    pub fn relation(&self, name: &str) -> Result<Arc<CompletedRelation>, ModelError> {
        self.completed.lock().map_err(|_| poisoned())?.get(name).cloned()
            .ok_or_else(|| ModelError::Invalid(format!("input {name} is not completed")))
    }
    pub fn completed_relations(&self) -> Result<Vec<Arc<CompletedRelation>>, ModelError> {
        Ok(self.completed.lock().map_err(|_| poisoned())?.values().cloned().collect())
    }
    /// Build an explicit immutable input set. Later completions do not change this view.
    pub fn inputs(&self, name: &'static str, profile: Profile, names: impl IntoIterator<Item=&'static str>) -> Result<CompletedInputs, ModelError> {
        let mut relations = BTreeMap::new();
        for name in names { relations.insert(name, self.relation(name)?); }
        Ok(CompletedInputs { name, profile, relations })
    }
    pub fn output(self: &Arc<Self>, name: &'static str, profile: Profile, implementation: ContentHash, inputs: CompletedInputs) -> ProducerOutput {
        ProducerOutput { workspace: self.clone(), name, profile, implementation, inputs,
            expected: None, allowed: None, writers: Mutex::default(), outcome: Mutex::new(None), failed: AtomicBool::new(false) }
    }
    pub fn producer(self:&Arc<Self>,declaration:&lctx_model::domain::stages::Stage,profile:Profile,inputs:CompletedInputs)->ProducerOutput {
        let mut output=self.output(declaration.name,profile,declaration.code,inputs);
        output.expected=Some(declaration.outputs.iter().map(|r|r.name()).collect());
        output.allowed=Some(declaration.outputs.iter().chain(&declaration.contributes).map(|r|r.name()).collect());
        output
    }
    async fn order(&self, producer: &'static str, implementation: ContentHash, pending: PendingRelation, inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>, profile: Profile) -> Result<Arc<CompletedRelation>, ModelError> {
        self.cancellation.check()?;
        let name = pending.relation.name();
        let path = self.path(name, "complete");
        let table = format!("input_{}", self.next.fetch_add(1, Ordering::Relaxed));
        // Union only explicitly contributed vocabulary, never silently replace an owned output.
        let mut paths = vec![pending.path.to_string_lossy().into_owned()];
        if let Ok(previous)=self.relation(name) {
            if pending.contribution || previous.contribution {
                paths.push(previous.path.to_string_lossy().into_owned());
            } else {return Err(ModelError::Invalid(format!("completed output {name} already has an owner")));}
        }
        let frame = self.context.read_arrow(paths, ArrowReadOptions::default().schema(pending.relation.schema().as_ref()))
            .await.map_err(ModelError::codec)?;
        self.context.register_table(&table, frame.into_view()).map_err(ModelError::codec)?;
        let result = async {
            let mut stream = self.context.sql(&format!("SELECT * FROM \"{table}\" ORDER BY id"))
                .await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
            let mut writer = FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, pending.relation.schema().as_ref()).map_err(ModelError::codec)?;
            let mut content = pending.relation.content();
            let mut previous: Option<RecordBatch> = None;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                self.cancellation.check()?;
                let _buffer = self.budget.reserve("workspace-order-batch", batch.get_array_memory_size().saturating_mul(3))?;
                let ids = batch.column_by_name("id").and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>()).ok_or(ModelError::Schema(name))?;
                let mut selected = Vec::with_capacity(batch.num_rows());
                for index in 0..batch.num_rows() {
                    let singleton = pending.relation.canonical(&batch.slice(index, 1))?;
                    let duplicate = previous.as_ref().is_some_and(|previous| {
                        let previous_id = previous.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().expect("canonical identity");
                        previous_id.value(0) == ids.value(index)
                    });
                    if duplicate {
                        if previous.as_ref() != Some(&singleton) { return Err(ModelError::Conflict(name)); }
                    } else {
                        selected.push(index as u32);
                    }
                    previous = Some(singleton);
                }
                if !selected.is_empty() {
                    let indices = UInt32Array::from(selected);
                    let columns = batch.columns().iter().map(|c| take(c.as_ref(), &indices, None).map_err(ModelError::codec)).collect::<Result<Vec<_>, _>>()?;
                    let canonical = RecordBatch::try_new(pending.relation.schema().clone(), columns).map_err(ModelError::codec)?;
                    pending.relation.hash_rows(&canonical, &mut content)?;
                    writer.write(&canonical).map_err(ModelError::codec)?;
                }
            }
            writer.finish().map_err(ModelError::codec)?;
            writer.into_inner().map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;
            self.cancellation.check()?;
            let (rows, content) = content.finish();
            Ok(Arc::new(CompletedRelation { relation: pending.relation, producer, implementation,
                contract: self.model.digest(), content, rows, inputs, profile, contribution:pending.contribution, snapshot: (pending.snapshot)(producer, self.model.digest(), implementation, content, rows)?, path, _files: self.files.clone() }))
        }.await;
        self.context.deregister_table(&table).map_err(ModelError::codec)?;
        result
    }
}
fn poisoned() -> ModelError { ModelError::Invalid("workspace ownership poisoned".into()) }

/// A stream descriptor, not a resident collection or a database read capability.
pub struct CompletedRelation {
    relation: Relation,
    producer: &'static str,
    implementation: ContentHash,
    contract: ContentHash,
    content: ContentHash,
    rows: u64,
    path: PathBuf,
    inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>,
    profile: Profile,
    contribution: bool,
    snapshot: lctx_model::domain::analysis::sources::SourceSnapshot,
    _files: Arc<WorkspaceFiles>,
}
impl CompletedRelation {
    pub fn name(&self) -> &'static str { self.relation.name() }
    pub fn path(&self) -> &Path { &self.path }
    pub fn content(&self) -> ContentHash { self.content }
    pub fn rows(&self) -> u64 { self.rows }
    pub fn producer(&self) -> &'static str { self.producer }
    pub fn implementation(&self) -> ContentHash { self.implementation }
    pub fn contract(&self) -> ContentHash { self.contract }
    pub fn snapshot(&self) -> lctx_model::domain::analysis::sources::SourceSnapshot { self.snapshot.clone() }
    pub fn batches(&self) -> Result<FileReader<File>, ModelError> {
        FileReader::try_new(File::open(&self.path).map_err(ModelError::codec)?, None).map_err(ModelError::codec)
    }
    pub fn read<R: Record>(&self, model: Arc<ValidatedModel>, budget: ResourceBudget) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() { return Err(ModelError::Schema(R::NAME)); }
        Ok(TypedBatches { reader: self.batches()?, model, budget, marker: Default::default(), _files: self._files.clone() })
    }
}
pub struct TypedBatches<R: Record> {
    reader: FileReader<File>, model: Arc<ValidatedModel>, budget: ResourceBudget,
    marker: std::marker::PhantomData<R>, _files: Arc<WorkspaceFiles>,
}
impl<R: Record> Iterator for TypedBatches<R> {
    type Item = Result<Batch<R>, ModelError>;
    fn next(&mut self) -> Option<Self::Item> { self.reader.next().map(|batch| batch.map_err(ModelError::codec).and_then(|batch| Batch::read(&self.model, &batch, &self.budget))) }
}
#[derive(Clone)]
pub struct CompletedInputs { name: &'static str, profile: Profile, relations: BTreeMap<&'static str, Arc<CompletedRelation>> }
impl CompletedInputs {
    pub fn name(&self) -> &'static str { self.name }
    pub fn profile(&self) -> Profile { self.profile }
    pub fn contains<R: Record>(&self) -> bool { self.relations.contains_key(R::NAME) }
    pub fn relation<R: Record>(&self) -> Result<&Arc<CompletedRelation>, ModelError> {
        self.relations.get(R::NAME).ok_or_else(|| ModelError::Invalid(format!("{} lacks explicit input {}", self.name, R::NAME)))
    }
    pub fn read<R: Record>(&self) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        let source = self.relation::<R>()?;
        lctx_model::domain::analysis::sources::CompletedInput::new(source.producer(), source.contract(), source.implementation(), source.content(), source.rows())
    }
    pub fn snapshots(&self) -> impl Iterator<Item=lctx_model::domain::analysis::sources::SourceSnapshot> + '_ {
        self.relations.values().map(|source| source.snapshot())
    }
    pub fn relations(&self) -> impl Iterator<Item=&Arc<CompletedRelation>> { self.relations.values() }
    pub async fn session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        let context = SessionContext::new_with_config_rt(workspace.context.copied_config(), workspace.context.runtime_env());
        for source in self.relations.values() {
            context.register_arrow(source.name(), source.path.to_string_lossy(), ArrowReadOptions::default().schema(source.relation.schema().as_ref()))
                .await.map_err(ModelError::codec)?;
        }
        Ok(context)
    }
}

type SnapshotBuilder = fn(&str, ContentHash, ContentHash, ContentHash, u64) -> Result<lctx_model::domain::analysis::sources::SourceSnapshot, ModelError>;
fn snapshot<R: Record>(producer: &str, contract: ContentHash, implementation: ContentHash, content: ContentHash, rows: u64) -> Result<lctx_model::domain::analysis::sources::SourceSnapshot, ModelError> {
    Ok(lctx_model::domain::analysis::sources::CompletedInput::<R>::new(producer, contract, implementation, content, rows)?.snapshot())
}
struct PendingRelation { relation: Relation, path: PathBuf, contribution: bool, snapshot: SnapshotBuilder }
trait ErasedWriter: Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn close(self: Box<Self>, model: &ValidatedModel, budget: &ResourceBudget) -> Result<PendingRelation, ModelError>;
}
struct Writer<R: Record> {
    ipc: FileWriter<File>, path: PathBuf, pending: Vec<R>, bytes: usize,
    charge: StateCharge, limits: TransferLimits, contribution: bool,
}
impl<R: Record> Writer<R> {
    fn flush(&mut self, model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.pending.is_empty() {
            let batch = Batch::new(model, std::mem::take(&mut self.pending), budget)?;
            self.ipc.write(batch.arrow()).map_err(ModelError::codec)?;
            self.charge = StateCharge::new(budget, R::NAME);
            self.bytes = 0;
        }
        Ok(())
    }
}
impl<R: Record> ErasedWriter for Writer<R> {
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn close(mut self: Box<Self>, model: &ValidatedModel, budget: &ResourceBudget) -> Result<PendingRelation, ModelError> {
        self.flush(model, budget)?;
        self.ipc.finish().map_err(ModelError::codec)?;
        self.ipc.into_inner().map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;
        Ok(PendingRelation { relation: Relation::of::<R>(), path: self.path, contribution: self.contribution, snapshot: snapshot::<R> })
    }
}
/// One producer owns pending streams; completion makes its whole output set visible atomically.
pub struct ProducerOutput {
    workspace: Arc<Workspace>, name: &'static str, profile: Profile, implementation: ContentHash,
    inputs: CompletedInputs, expected: Option<std::collections::BTreeSet<&'static str>>, allowed: Option<std::collections::BTreeSet<&'static str>>, writers: Mutex<BTreeMap<&'static str, Box<dyn ErasedWriter>>>,
    outcome: Mutex<Option<ProviderOutcome>>, failed: AtomicBool,
}
impl ProducerOutput {
    pub fn inputs(&self) -> &CompletedInputs { &self.inputs }
    pub fn profile(&self) -> Profile { self.profile }
    pub fn workspace(&self) -> &Arc<Workspace> { &self.workspace }
    fn check(&self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        if self.failed.load(Ordering::Acquire) || self.outcome.lock().map_err(|_| poisoned())?.is_some() {
            return Err(ModelError::Invalid("producer output is closed or failed".into()));
        }
        Ok(())
    }
    fn guarded<T>(&self,operation:impl FnOnce()->Result<T,ModelError>)->Result<T,ModelError> {
        let result=self.check().and_then(|_|operation());
        if result.is_err() {self.failed.store(true,Ordering::Release);}
        result
    }
    pub fn declare<R: Record>(&self) -> Result<(), ModelError> { self.declare_kind::<R>(lctx_model::domain::stages::is_vocabulary(R::NAME)) }
    fn declare_kind<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.guarded(||self.declare_kind_inner::<R>(contribution))
    }
    fn declare_kind_inner<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.check()?;
        if self.allowed.as_ref().is_some_and(|names|!names.contains(R::NAME)) {self.failed.store(true,Ordering::Release);return Err(ModelError::Invalid(format!("{} did not declare {}",self.name,R::NAME)));}
        self.workspace.model.require::<R>()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        if writers.contains_key(R::NAME) { return Err(ModelError::Invalid(format!("output {} declared twice", R::NAME))); }
        let path = self.workspace.path(R::NAME, "pending");
        let ipc = FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, R::schema().as_ref()).map_err(ModelError::codec)?;
        writers.insert(R::NAME, Box::new(Writer::<R> { ipc, path, pending: Vec::new(), bytes: 0,
            charge: StateCharge::new(self.workspace.budget(), R::NAME), limits: TransferLimits { rows: self.workspace.options.batch_rows, ..Default::default() }, contribution }));
        Ok(())
    }
    pub fn write<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.guarded(||self.write_inner(batch))
    }
    fn write_inner<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers.get_mut(R::NAME).and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>()).ok_or_else(|| ModelError::Invalid(format!("output {} was not declared", R::NAME)))?;
        writer.flush(&self.workspace.model, self.workspace.budget())?;
        let result = writer.ipc.write(batch.arrow()).map_err(ModelError::codec);
        if result.is_err() { self.failed.store(true, Ordering::Release); }
        result
    }
    pub fn contribute<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        if !self.writers.lock().map_err(|_| poisoned())?.contains_key(R::NAME) { self.declare_kind::<R>(true)?; }
        self.write(batch)
    }
    pub async fn push<R: Record>(&self, row: R) -> Result<(), ModelError> {
        self.push_sync(row)
    }
    pub fn push_sync<R: Record>(&self, row: R) -> Result<(), ModelError> {
        let result=(|| {
        self.check()?;
        row.validate()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers.get_mut(R::NAME).and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>()).ok_or_else(|| ModelError::Invalid(format!("output {} was not declared", R::NAME)))?;
        let bytes = row.row_bytes();
        if bytes > writer.limits.max_row { return Err(ModelError::Invalid(format!("{} exceeds row limit", R::NAME))); }
        if !writer.pending.is_empty() && (writer.pending.len() >= writer.limits.rows || writer.bytes.saturating_add(bytes) > writer.limits.bytes) { writer.flush(&self.workspace.model, self.workspace.budget())?; }
        writer.charge.grow(bytes)?;
        writer.pending.push(row);
        writer.bytes += bytes;
        Ok(())
        })();
        if result.is_err() {self.failed.store(true,Ordering::Release);}
        result
    }
    pub fn mark_finished(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.check()?;
        if outcome==ProviderOutcome::Failed {self.failed.store(true,Ordering::Release);return Err(ModelError::Invalid("failed producer cannot complete outputs".into()));}
        *self.outcome.lock().map_err(|_| poisoned())? = Some(outcome);
        Ok(())
    }
    pub async fn finish(self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.mark_finished(outcome)?;
        self.complete().await
    }
    pub async fn complete(self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        if self.failed.load(Ordering::Acquire) || self.outcome.lock().map_err(|_| poisoned())?.is_none() { return Err(ModelError::Invalid("producer did not complete successfully".into())); }
        let _completion=self.workspace.completion_gate.lock().await;
        let writers = self.writers.into_inner().map_err(|_| poisoned())?;
        if let Some(expected)=&self.expected {
            if let Some(name)=expected.iter().find(|name|!writers.contains_key(**name)) {return Err(ModelError::Invalid(format!("{} omitted completed output {name}",self.name)));}
        }
        let input_snapshots:Arc<[_]>=self.inputs.snapshots().collect::<Vec<_>>().into();
        let mut completed = Vec::new();
        for (_, writer) in writers {
            let pending = writer.close(&self.workspace.model, self.workspace.budget())?;
            completed.push(self.workspace.order(self.name, self.implementation, pending, input_snapshots.clone(), self.profile).await?);
        }
        self.workspace.cancellation.check()?;
        let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
        for source in completed { visible.insert(source.name(), source); }
        Ok(())
    }
}

#[derive(Debug)]
struct WorkspacePool { memory: Arc<dyn MemoryPool>, limit: usize }
#[derive(Debug)]
struct WorkspaceReservation { reservation: MemoryReservation, owner: &'static str, pool: Arc<dyn MemoryPool>, limit: usize }
impl ResourcePool for WorkspacePool {
    fn reserve(&self, owner: &'static str, bytes: usize) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reserved = WorkspaceReservation { reservation: MemoryConsumer::new(owner).register(&self.memory), owner, pool: self.memory.clone(), limit: self.limit };
        reserved.try_resize(bytes)?;
        Ok(Box::new(reserved))
    }
    fn reserved(&self) -> usize { self.memory.reserved() }
    fn limit(&self) -> usize { self.limit }
}
impl Reservation for WorkspaceReservation {
    fn size(&self) -> usize { self.reservation.size() }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.reservation.try_resize(bytes).map_err(|_| ModelError::Resource { owner: self.owner,
            requested: bytes.saturating_sub(self.reservation.size()), used: self.pool.reserved(), limit: self.limit })
    }
}

impl cpg_extract::bundle::ProviderSink for ProducerOutput {
    fn read<R: Record>(&self) -> Result<Box<dyn Iterator<Item=Result<Batch<R>, ModelError>> + Send>, ModelError> {
        self.check()?;
        Ok(Box::new(self.inputs.relation::<R>()?.read::<R>(self.workspace.model.clone(), self.workspace.budget.clone())?))
    }
    fn declare<R: Record>(&self) -> Result<(), ModelError> { ProducerOutput::declare::<R>(self) }
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> { ProducerOutput::write(self, &batch) }
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> { ProducerOutput::contribute(self, &batch) }
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> { self.mark_finished(outcome) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::input::Package;
    fn model() -> Arc<ValidatedModel> {
        Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap())
    }
    async fn packages(memory: usize, batch: usize, reverse: bool) -> Arc<CompletedRelation> {
        let workspace = Workspace::new(model(), WorkspaceOptions { memory_bytes: memory, partitions: 1, batch_rows: batch }).unwrap();
        let inputs = workspace.inputs("packages", Profile::Catalog, []).unwrap();
        let output = workspace.output("packages", Profile::Catalog, ContentHash::of(b"fixture"), inputs);
        output.declare::<Package>().unwrap();
        let rows: Vec<_> = (0..20000).map(|n| Package { name: format!("package-{n:08}") }).collect();
        if reverse { for row in rows.into_iter().rev() { output.push(row).await.unwrap(); } }
        else { for row in rows { output.push(row).await.unwrap(); } }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.completed::<Package>().unwrap()
    }
    #[tokio::test]
    async fn canonical_stream_is_independent_of_batching_order_and_memory_limit() {
        let ordinary = packages(64 << 20, 4096, false).await;
        let constrained = packages(2 << 20, 128, true).await;
        assert_eq!(ordinary.content(), constrained.content());
        assert_eq!(constrained.rows(), 20000);
        let mut last = None;
        let mut rows = 0;
        for batch in constrained.batches().unwrap() {
            for row in Package::decode(&batch.unwrap()).unwrap() {
                let id = row.id();
                assert!(last.is_none_or(|previous| previous < id));
                last = Some(id);
                rows += 1;
            }
        }
        assert_eq!(rows, 20000);
    }
    #[tokio::test]
    async fn cancellation_and_incomplete_output_never_become_completed() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let output = workspace.output("packages", Profile::Catalog, ContentHash::of(b"fixture"), workspace.inputs("packages", Profile::Catalog, []).unwrap());
        output.declare::<Package>().unwrap();
        output.push(Package { name: "private".into() }).await.unwrap();
        assert!(workspace.completed::<Package>().is_err());
        workspace.cancellation().cancel();
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn omitted_declared_outputs_and_ignored_row_refusals_never_complete() {
        use lctx_model::domain::stages::{Stage,RelationUse,Effect};
        let workspace=Workspace::new(model(),WorkspaceOptions::default()).unwrap();
        let declaration=Stage {name:"packages",inputs:vec![],outputs:vec![RelationUse::of::<Package>()],contributes:vec![],coverage:vec![],profiles:vec![Profile::Catalog],effect:Effect::Pure,code:ContentHash::of(b"fixture"),configuration:ContentHash::of(b"fixture")};
        let output=workspace.producer(&declaration,Profile::Catalog,workspace.inputs("packages",Profile::Catalog,[]).unwrap());
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
        let budget=ResourceBudget::fixed(4096).unwrap();
        let constrained=Workspace::with_budget(model(),WorkspaceOptions {memory_bytes:4096,..Default::default()},budget).unwrap();
        let output=constrained.output("packages",Profile::Catalog,ContentHash::of(b"fixture"),constrained.inputs("packages",Profile::Catalog,[]).unwrap());
        output.declare::<Package>().unwrap();
        assert!(output.push(Package {name:"x".repeat(5000)}).await.is_err());
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(constrained.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn conflicting_payload_refuses_the_whole_output_set() {
        use lctx_model::domain::{input::InputRevision,source::SourceArtifact};
        let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions::default()).unwrap();
        let output=workspace.output("source",Profile::Catalog,ContentHash::of(b"fixture"),workspace.inputs("source",Profile::Catalog,[]).unwrap());
        output.declare::<Package>().unwrap();
        output.declare::<SourceArtifact>().unwrap();
        output.push(Package {name:"private".into()}).await.unwrap();
        let first=SourceArtifact {input:InputRevision {manifest:ContentHash::of(b"manifest")}.id(),path:"module.py".into(),content:ContentHash::of(b"x"),byte_len:1};
        let mut second=first.clone();second.byte_len=2;
        output.push(first).await.unwrap();output.push(second).await.unwrap();
        assert!(matches!(output.finish(ProviderOutcome::Complete).await,Err(ModelError::Conflict(_))));
        assert!(workspace.completed::<Package>().is_err());
        assert!(workspace.completed::<SourceArtifact>().is_err());
    }
    #[tokio::test]
    async fn ordinary_owner_adopts_native_contributions_then_refuses_another_owner() {
        let workspace=Workspace::new(model(),WorkspaceOptions::default()).unwrap();
        let contributed=workspace.output("native",Profile::Catalog,ContentHash::of(b"native"),workspace.inputs("native",Profile::Catalog,[]).unwrap());
        let batch=Batch::new(workspace.model(),vec![Package {name:"package".into()}],workspace.budget()).unwrap();
        contributed.contribute(&batch).unwrap();drop(batch);
        contributed.finish(ProviderOutcome::Complete).await.unwrap();
        let owner=workspace.output("assembly",Profile::Catalog,ContentHash::of(b"assembly"),workspace.inputs("assembly",Profile::Catalog,[]).unwrap());
        owner.declare::<Package>().unwrap();owner.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.completed::<Package>().unwrap().rows(),1);
        let duplicate=workspace.output("duplicate",Profile::Catalog,ContentHash::of(b"duplicate"),workspace.inputs("duplicate",Profile::Catalog,[]).unwrap());
        duplicate.declare::<Package>().unwrap();
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        assert_eq!(workspace.completed::<Package>().unwrap().producer(),"assembly");
    }
    #[tokio::test]
    async fn nominal_reference_closure_reuses_completed_streams_and_refuses_missing_targets() {
        use lctx_model::domain::input::Release;
        let model=Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>(),Relation::of::<Release>()]).unwrap());
        for valid in [true,false] {
            let workspace=Workspace::new(model.clone(),WorkspaceOptions::default()).unwrap();
            let output=workspace.output("release",Profile::Catalog,ContentHash::of(b"fixture"),workspace.inputs("release",Profile::Catalog,[]).unwrap());
            output.declare::<Package>().unwrap();output.declare::<Release>().unwrap();
            let package=Package {name:"package".into()};
            let reference=if valid {package.id()} else {Package {name:"absent".into()}.id()};
            output.push(package).await.unwrap();
            output.push(Release {package:reference,version:"1.0".into()}).await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let content=workspace.content().unwrap();
            let admitted=workspace.validate().await;
            if valid {assert_eq!(admitted.unwrap(),content);}else{assert!(admitted.is_err());}
        }
    }
    #[tokio::test]
    async fn completed_input_keeps_files_alive_and_later_contributions_do_not_change_it() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let output = workspace.output("packages", Profile::Catalog, ContentHash::of(b"fixture"), workspace.inputs("packages", Profile::Catalog, []).unwrap());
        output.declare::<Package>().unwrap();
        output.push(Package { name: "one".into() }).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let original = workspace.completed::<Package>().unwrap();
        let bound = workspace.inputs("reader", Profile::Catalog, [Package::NAME]).unwrap();
        let next = workspace.output("more", Profile::Catalog, ContentHash::of(b"fixture"), workspace.inputs("more", Profile::Catalog, []).unwrap());
        let batch = Batch::new(workspace.model(), vec![Package { name: "two".into() }], workspace.budget()).unwrap();
        next.contribute(&batch).unwrap();
        drop(batch);
        next.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(bound.relation::<Package>().unwrap().content(), original.content());
        assert_eq!(original.rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        let path = original.path().to_path_buf();
        drop(bound);
        drop(workspace);
        assert!(path.exists());
        assert_eq!(original.batches().unwrap().next().unwrap().unwrap().num_rows(), 1);
        drop(original);
        assert!(!path.exists());
    }
}

#[derive(Debug)]
struct BudgetMemoryPool {budget:ResourceBudget,allocated:Mutex<Box<dyn Reservation>>}
impl std::fmt::Display for BudgetMemoryPool {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {write!(f,"shared compiler allocation pool")}}
impl MemoryPool for BudgetMemoryPool {
    fn name(&self)->&str {"CompilerResourcePool"}
    fn grow(&self,reservation:&MemoryReservation,bytes:usize) {self.try_grow(reservation,bytes).expect("infallible DataFusion allocation must already fit");}
    fn shrink(&self,_:&MemoryReservation,bytes:usize) {let mut allocated=self.allocated.lock().expect("memory pool poisoned");let size=allocated.size();allocated.try_resize(size-bytes).expect("returning reservation");}
    fn try_grow(&self,_:&MemoryReservation,bytes:usize)->datafusion::error::Result<()> {
        let mut allocated=self.allocated.lock().map_err(|_|datafusion::error::DataFusionError::Internal("memory pool poisoned".into()))?;
        let size=allocated.size();allocated.try_resize(size.saturating_add(bytes)).map_err(|e|datafusion::error::DataFusionError::ResourcesExhausted(e.to_string()))
    }
    fn reserved(&self)->usize {self.budget.reserved()}
    fn memory_limit(&self)->datafusion::execution::memory_pool::MemoryLimit {datafusion::execution::memory_pool::MemoryLimit::Finite(self.budget.limit())}
}
