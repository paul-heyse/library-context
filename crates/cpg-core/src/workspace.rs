//! Private persisted compilation workspace with exact immutable contribution views.
//! DataFusion scratch is spillable and derived; native memberships own completed inputs.
use arrow_array::RecordBatch;
use datafusion::{
    execution::{
        disk_manager::{DiskManagerBuilder, DiskManagerMode},
        memory_pool::{MemoryConsumer, MemoryPool, MemoryReservation},
        runtime_env::RuntimeEnvBuilder,
    },
    prelude::{SessionConfig, SessionContext},
};
use futures::{TryStreamExt,FutureExt,future::{BoxFuture,Shared}};
use lctx_model::domain::{
    Batch, ContentHash, ModelError, Record, Relation, ValidatedModel,
    batching::TransferLimits,
    charged::StateCharge,
    resources::{Reservation, ResourceBudget, ResourcePool},
    stages::{Profile, ProviderOutcome},
};
use std::{
    any::Any,
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

type ProviderDrain = Shared<BoxFuture<'static,Result<(),Arc<str>>>>;
type ChargedBatch = Result<(RecordBatch, Box<dyn Reservation>), ModelError>;

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceOptions {
    pub memory_bytes: usize,
    pub partitions: usize,
    pub batch_rows: usize,
}
impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self {
            memory_bytes: 256 << 20,
            partitions: 4,
            batch_rows: 4096,
        }
    }
}
/// Cancellation is cooperative between bounded I/O/compute batches. Owned tasks must be drained
/// by their caller before releasing the workspace.
#[derive(Default)]
struct CancellationState {
    cancelled: AtomicBool,
    notify: tokio::sync::Notify,
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<CancellationState>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.notify.notify_waiters();
    }
    pub fn check(&self) -> Result<(), ModelError> {
        if self.0.cancelled.load(Ordering::Acquire) {
            Err(ModelError::Invalid("compilation cancelled".into()))
        } else {
            Ok(())
        }
    }
    pub(crate) async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.0.cancelled.load(Ordering::Acquire) { return; }
            notified.await;
        }
    }
}
struct WorkspaceFiles {
    directory: tempfile::TempDir,
}
#[derive(PartialEq, Eq)]
struct CompilationCompletion {
    frontier: lctx_model::domain::admission::Frontier,
    profile: Profile,
    configuration: ContentHash,
    captures: ContentHash,
    content: ContentHash,
}
fn capture_identity(
    captured: &cpg_extract::bundle::CapturedInputs,
    budget: &ResourceBudget,
) -> Result<ContentHash, ModelError> {
    use lctx_model::domain::Key;
    let _keys = budget.reserve(
        "compiler-capture-keys",
        captured.inputs().len().saturating_mul(64),
    )?;
    let mut ids = captured
        .inputs()
        .iter()
        .map(|input| input.captured().revision().id())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ModelError::Invalid("duplicate compiler capture".into()));
    }
    let mut sink = lctx_model::domain::KeySink::new("compiler-captures/v1");
    for id in ids {
        id.encode(&mut sink);
    }
    Ok(sink.finish())
}
/// The attempt owns one runtime, spill directory, buffer budget, and completed relation registry.
pub struct Workspace {
    files: Arc<WorkspaceFiles>,
    context: SessionContext,
    options: WorkspaceOptions,
    budget: ResourceBudget,
    model: Arc<ValidatedModel>,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    bridge: Arc<crate::native_bridge::NativeBridge>,
    native_calls:Arc<crate::native_calls::NativeCalls>,
    completed: Mutex<BTreeMap<&'static str, Arc<CompletedRelation>>>,
    frozen_shared: Mutex<
        BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            Arc<CompletedRelation>,
        >,
    >,
    cancellation: Cancellation,
    completion_gate: tokio::sync::Mutex<()>,
    compilation_complete: Mutex<Option<CompilationCompletion>>,
    provider_drains: Mutex<Vec<ProviderDrain>>,
    checked_premises: Mutex<(std::collections::BTreeSet<ContentHash>, StateCharge)>,
}
impl Workspace {
    pub fn new(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 || options.batch_rows == 0 {
            return Err(ModelError::Invalid(
                "workspace sizes must be positive".into(),
            ));
        }
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_limit(options.memory_bytes, 1.0)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(
                DiskManagerMode::Directories(vec![directory.path().to_path_buf()]),
            ))
            .build_arc()
            .map_err(ModelError::codec)?;
        let config = SessionConfig::new()
            .with_target_partitions(options.partitions)
            .with_batch_size(options.batch_rows)
            .set_usize(
                "datafusion.execution.sort_spill_reservation_bytes",
                (options.memory_bytes / 16).min(10 << 20),
            )
            .set_usize(
                "datafusion.execution.sort_in_place_threshold_bytes",
                (options.memory_bytes / 8).min(1 << 20),
            );
        let budget = ResourceBudget::from_pool(Arc::new(WorkspacePool { memory: runtime.memory_pool.clone(), limit: options.memory_bytes }))?;
        let checked_premises = Mutex::new((Default::default(), StateCharge::new(&budget, "compiler-validity-premises")));
        let cancellation = Cancellation::default();
        let bridge = Arc::new(crate::native_bridge::NativeBridge::new(cancellation.clone())?);
        let native_calls=Arc::new(crate::native_calls::NativeCalls::new(native.clone(),cancellation.clone(),&budget));
        Ok(Arc::new(Self {
            files: Arc::new(WorkspaceFiles { directory }),
            context: SessionContext::new_with_config_rt(config, runtime.clone()),
            options,
            budget,
            model,
            native,
            bridge,
            native_calls,
            completed: Mutex::default(),
            frozen_shared: Mutex::default(),
            cancellation,
            completion_gate: tokio::sync::Mutex::new(()),
            compilation_complete: Mutex::default(),
            provider_drains: Mutex::default(),
            checked_premises,
        }))
    }
    /// Reuse a captured native configuration's pool. DataFusion allocations and typed retained
    /// state draw from this same ceiling; the custom pool adapts allocations rather than copying.
    pub fn with_budget(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
        native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes != budget.limit() {
            return Err(ModelError::Invalid(
                "workspace memory differs from supplied pool".into(),
            ));
        }
        let mut workspace = Self::new(model, options, native)?;
        let owner = Arc::get_mut(&mut workspace).expect("new workspace has one owner");
        let pool = Arc::new(BudgetMemoryPool {
            budget: budget.clone(),
            allocated: Mutex::new(budget.reserve("workspace-datafusion", 0)?),
        });
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_pool(pool)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(
                DiskManagerMode::Directories(vec![owner.files.directory.path().to_path_buf()]),
            ))
            .build_arc()
            .map_err(ModelError::codec)?;
        owner.context = SessionContext::new_with_config_rt(owner.context.copied_config(), runtime);
        owner.checked_premises = Mutex::new((Default::default(), StateCharge::new(&budget, "compiler-validity-premises")));
        owner.native_calls=Arc::new(crate::native_calls::NativeCalls::new(owner.native.clone(),owner.cancellation.clone(),&budget));
        owner.budget = budget;
        Ok(workspace)
    }
    /// Restore the exact current and frozen bindings after independent native state import.
    pub async fn restore(&self, profile: Profile) -> Result<(), ModelError> {
        self.writable()?;
        if !self.completed.lock().map_err(|_| poisoned())?.is_empty()
            || !self.frozen_shared.lock().map_err(|_| poisoned())?.is_empty()
        { return Err(ModelError::Conflict("native restoration requires an empty workspace")); }
        let contributions=self.native.contributions().await?;
        let mut output_names=std::collections::BTreeSet::new();
        for contribution in contributions {
            contribution.identity()?;
            let spec=contribution.spec;
            if spec.model!=self.model.digest() || spec.profile!=profile{return Err(ModelError::Conflict("restored contribution model or profile"));}
            for input in spec.inputs {
                if input.model()!=self.model.digest() || input.rows()<0 || self.model.relation(input.relation()).is_none(){return Err(ModelError::Conflict("restored contribution dependency model"));}
            }
            for output in spec.outputs {
                if self.model.relation(&output).is_none(){return Err(ModelError::Schema("restored contribution output relation"));}
                output_names.insert(output);
            }
        }
        let bindings = self.native.bindings().await?;
        let current_names=bindings.iter().filter(|binding|binding.boundary.is_none()).map(|binding|binding.view.relation.clone()).collect::<std::collections::BTreeSet<_>>();
        if current_names!=output_names{return Err(ModelError::Conflict("restored current output binding inventory"));}
        let mut completed = BTreeMap::new(); let mut frozen = BTreeMap::new();
        for binding in bindings {
            binding.validate()?;
            if binding.source.model() != self.model.digest() { return Err(ModelError::Conflict("restored compiler model")); }
            let relation = self.model.relation(binding.source.relation()).ok_or(ModelError::Schema("restored compiler relation"))?.clone();
            let name = relation.name();
            if binding.configuration.is_some() || binding.source!=lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&relation,self.model.digest(),&binding.view)?{return Err(ModelError::Conflict("restored neutral completed view binding"));}
            let source = Arc::new(CompletedRelation {
                relation, producer: binding.source.producer().into(), implementation: binding.source.implementation(),
                configuration: binding.configuration, contract: binding.source.model(), rows: binding.view.rows,
                view: binding.view, native: self.native.clone(), bridge: self.bridge.clone(), budget: self.budget.clone(),
                batch_rows: self.options.batch_rows, cancellation: self.cancellation.clone(), inputs: Arc::from([]), profile,
                contribution: lctx_model::domain::stages::is_epoch_shared(name), snapshot: binding.source, _files: self.files.clone(),
            });
            if let Some(boundary) = binding.boundary {
                let boundary = lctx_model::domain::stages::PublicationBoundary::ALL.into_iter().find(|candidate| candidate.name() == boundary)
                    .ok_or(ModelError::Schema("restored compiler boundary"))?;
                if frozen.insert((boundary, name), source).is_some() { return Err(ModelError::Conflict("duplicate frozen restored binding")); }
            } else if completed.insert(name, source).is_some() { return Err(ModelError::Conflict("duplicate current restored binding")); }
        }
        *self.completed.lock().map_err(|_| poisoned())? = completed;
        *self.frozen_shared.lock().map_err(|_| poisoned())? = frozen;
        Ok(())
    }
    fn writable(&self) -> Result<(), ModelError> {
        if self
            .compilation_complete
            .lock()
            .map_err(|_| poisoned())?
            .is_some()
        {
            return Err(ModelError::Invalid(
                "completed compiler workspace is immutable".into(),
            ));
        }
        Ok(())
    }
    pub(crate) async fn finish_compilation(
        &self,
        captures: ContentHash,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
        configuration: ContentHash,
    ) -> Result<(), ModelError> {
        let _gate = self.completion_gate.lock().await;
        self.cancellation.check()?;
        self.writable()?;
        let content = self.identity()?;
        *self.compilation_complete.lock().map_err(|_| poisoned())? = Some(CompilationCompletion {
            frontier,
            profile,
            configuration,
            captures,
            content,
        });
        Ok(())
    }
    pub(crate) fn captures(
        &self,
        captured: &cpg_extract::bundle::CapturedInputs,
    ) -> Result<ContentHash, ModelError> {
        capture_identity(captured, &self.budget)
    }
    pub(crate) fn require_compilation(
        &self,
        captured: &cpg_extract::bundle::CapturedInputs,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
        configuration: ContentHash,
    ) -> Result<(), ModelError> {
        let expected = CompilationCompletion {
            frontier,
            profile,
            configuration,
            captures: self.captures(captured)?,
            content: self.identity()?,
        };
        if self
            .compilation_complete
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            != Some(&expected)
        {
            return Err(ModelError::Invalid(
                "artifact requires the completed requested compilation and exact captures".into(),
            ));
        }
        Ok(())
    }
    fn coverage_rows<R: Record>(&self) -> Result<(Vec<R>, StateCharge), ModelError> {
        let mut rows = Vec::new();
        let mut charge = StateCharge::new(&self.budget, "facts-coverage-input");
        for batch in self
            .completed::<R>()?
            .read::<R>(self.model.clone(), self.budget.clone())?
        {
            for row in batch?.rows() {
                charge.grow(row.row_bytes().saturating_add(size_of::<R>()))?;
                rows.push(row.clone());
            }
        }
        Ok((rows, charge))
    }
    async fn coverage_rows_async<R:Record>(&self)->Result<(Vec<R>,StateCharge),ModelError>{
        let source=self.completed::<R>()?;
        let mut stream=self.native.scan_batches(source.view(),&Relation::of::<R>(),None,None,&self.budget,self.options.batch_rows).await?;
        let mut rows=Vec::new();let mut charge=StateCharge::new(&self.budget,"facts-coverage-input");
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{
            self.cancellation.check()?;
            let batch=Batch::<R>::read(&self.model,&batch,&self.budget)?;
            for row in batch.rows(){charge.grow(row.row_bytes()+size_of::<R>())?;rows.push(row.clone());}
        }
        Ok((rows,charge))
    }
    fn availability_from_rows(&self,profile:Profile,inputs:&[lctx_model::domain::input::InputRevision],artifacts:&[lctx_model::domain::source::SourceArtifact],uses:&[lctx_model::domain::input::ArtifactUse],scope_rows:Vec<lctx_model::domain::source::CoverageScope>,rows:&[lctx_model::domain::attribution::ProviderCoverage])->Result<lctx_model::domain::admission::ScopedAvailability,ModelError>{
        use lctx_model::domain::{admission::{Expected,FrontierContract,ScopedAvailability},stages::Schedule};
        let providers = crate::facts::providers(ContentHash::of(b"facts-coverage-contract"));
        let schedule = Schedule::build(
            &self.model,
            providers.iter().map(|p| p.declaration(profile)).collect(),
            &[],
            profile,
        )?;
        let contract = FrontierContract::facts(&self.model, profile)?.preflight(&schedule)?;
        let expected=contract.expected_coverage(inputs,artifacts,uses)?.into_keys().collect::<std::collections::BTreeSet<Expected>>();
        let scopes=scope_rows.into_iter().map(|row|(row.id(),row)).collect();
        ScopedAvailability::from_completed(profile,&expected,rows,&scopes,&self.budget)
    }
    pub fn facts_availability(&self,profile:Profile)->Result<lctx_model::domain::admission::ScopedAvailability,ModelError>{
        use lctx_model::domain::{input::{InputRevision,ArtifactUse},source::{SourceArtifact,CoverageScope},attribution::ProviderCoverage};
        let (inputs,_inputs)=self.coverage_rows::<InputRevision>()?;
        let (artifacts,_artifacts)=self.coverage_rows::<SourceArtifact>()?;
        let (uses,_uses)=self.coverage_rows::<ArtifactUse>()?;
        let (scopes,_scopes)=self.coverage_rows::<CoverageScope>()?;
        let (rows,_rows)=self.coverage_rows::<ProviderCoverage>()?;
        self.availability_from_rows(profile,&inputs,&artifacts,&uses,scopes,&rows)
    }
    pub async fn facts_availability_async(&self,profile:Profile)->Result<lctx_model::domain::admission::ScopedAvailability,ModelError>{
        use lctx_model::domain::{input::{InputRevision,ArtifactUse},source::{SourceArtifact,CoverageScope},attribution::ProviderCoverage};
        let (inputs,_inputs)=self.coverage_rows_async::<InputRevision>().await?;
        let (artifacts,_artifacts)=self.coverage_rows_async::<SourceArtifact>().await?;
        let (uses,_uses)=self.coverage_rows_async::<ArtifactUse>().await?;
        let (scopes,_scopes)=self.coverage_rows_async::<CoverageScope>().await?;
        let (rows,_rows)=self.coverage_rows_async::<ProviderCoverage>().await?;
        self.availability_from_rows(profile,&inputs,&artifacts,&uses,scopes,&rows)
    }
    /// Semantic content over the actual completed typed streams, independent of IPC bytes.
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = lctx_model::domain::KeySink::new("compiler-workspace-view-set/v1");
        for relation in self.completed_relations()? {
            sink.part(relation.name().as_bytes(), &relation.view_identity().0);
        }
        Ok(sink.finish())
    }
    /// Reuse the model's ordered invariant state machines with spillable SQL ordering. No global
    /// graph concatenation or resident copy of all completed relations is constructed.
    pub async fn validate(&self) -> Result<ContentHash, ModelError> {
        let profile = self
            .completed_relations()?
            .first()
            .map_or(Profile::Catalog, |r| r.profile);
        self.validate_scope(profile, false, None).await
    }
    /// Establish necessary semantic properties for immutable completed input descriptors.
    pub async fn checked_inputs(
        &self,
        inputs: &CompletedInputs,
    ) -> Result<CheckedInputs, ModelError> {
        self.validate_scope(inputs.profile, true, Some(inputs))
            .await?;
        Ok(CheckedInputs {
            inputs: inputs.clone(),
            attempt: self.files.clone(),
            policy: self.model.digest(),
        })
    }
    pub async fn admit_semantics(&self, profile: Profile) -> Result<CheckedInputs, ModelError> {
        let relations = self.completed_relations()?;
        let mut inputs = self.inputs(
            "semantic-admission",
            profile,
            relations.iter().map(|r| r.name()),
        )?;
        // Whole-artifact admission validates actual frozen owner premises in a live attempt.
        // A portable import has only canonical streams and uses the explicitly detached path.
        // Consumer admission remains exact and never substitutes a missing selected epoch.
        self.validate_scope(profile, true, None).await?;
        for ((boundary, name), relation) in
            self.frozen_shared.lock().map_err(|_| poisoned())?.iter()
        {
            inputs
                .relations
                .insert((*name, Some(*boundary)), relation.clone());
        }
        Ok(CheckedInputs {
            inputs,
            attempt: self.files.clone(),
            policy: self.model.digest(),
        })
    }
    async fn validate_scope(
        &self,
        profile: Profile,
        admission: bool,
        selected: Option<&CompletedInputs>,
    ) -> Result<ContentHash, ModelError> {
        let relations = selected
            .map(|inputs| inputs.relations().cloned().collect())
            .unwrap_or(self.completed_relations()?);
        let names = relations
            .iter()
            .map(|r| r.name())
            .collect::<std::collections::BTreeSet<_>>();
        // A checked dependency closure retains its selected publication epochs.
        // Reconstructing this set by name would silently substitute the latest stream.
        let inputs = match selected {
            Some(inputs) => inputs.clone(),
            None => self.inputs("artifact-admission", profile, names.iter().copied())?,
        };
        let session = inputs.session(self).await?;
        self.validate_references(&session, &relations, &inputs).await?;
        // Catalog explicitly leaves native flow unrequested. Empty premises come from that
        // provider declaration, never from arbitrary missing requested relations.
        let mut unrequested = std::collections::BTreeSet::new();
        if profile == Profile::Catalog {
            let declaration = crate::facts::providers(ContentHash::of(b"flow-profile-premises"))
                .into_iter()
                .map(|provider| provider.declaration(Profile::Behavioral))
                .find(|stage| stage.name == "ty_flow")
                .ok_or_else(|| ModelError::Invalid("native flow declaration missing".into()))?;
            for relation in declaration
                .outputs
                .iter()
                .filter(|relation| !lctx_model::domain::stages::is_vocabulary(relation.name()))
            {
                if !names.contains(relation.name()) {
                    session
                        .register_batch(
                            relation.name(),
                            RecordBatch::new_empty(
                                self.model
                                    .relation(relation.name())
                                    .ok_or(ModelError::Schema(relation.name()))?
                                    .schema()
                                    .clone(),
                            ),
                        )
                        .map_err(ModelError::codec)?;
                    unrequested.insert(relation.name());
                }
            }
        }
        let frozen_tables = match selected {
            Some(inputs) => inputs
                .relations
                .keys()
                .filter_map(|(name, prefix)| {
                    prefix.map(|boundary| {
                        (
                            (boundary, *name),
                            CompletedInputs::table(name, Some(boundary)),
                        )
                    })
                })
                .collect(),
            None => self.validation_views(&session).await?,
        };
        let checks = if admission {
            self.model.admission_candidates_for_scope(&names)?
        } else {
            self.model
                .invariants_for_scope_with_premises(&names, &unrequested)?
        };
        for invariant in checks {
            let premise = self.validation_premise(&invariant, selected, &unrequested, profile)?;
            if let Some(premise) = premise
                && self.checked_premises.lock().map_err(|_| poisoned())?.0.contains(&premise)
            { continue; }
            let mut check = (invariant.create)(self.budget());
            let execution_scope = check.execution_scope();
            if let Some(scope) = &execution_scope {
                let alias = if let Some(inputs) = selected {
                    inputs.validation_table(&scope.root)?
                } else {
                    Self::validation_table(&scope.root, &frozen_tables)?.to_owned()
                };
                let sql = format!(
                    "SELECT id FROM {} LIMIT 1",
                    crate::consumed_rows::identifier(&alias)
                );
                let mut roots = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                let mut nonempty = false;
                while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    nonempty |= batch.num_rows() != 0;
                }
                if !nonempty {
                    self.remember_premise(premise)?;
                continue;
                }
            }
            let normalization_scope = check.normalization_scope();
            let projection_scope = check.projection_scope();
            let applicability_roots = normalization_scope
                .map(|scope| scope.roots())
                .or_else(|| projection_scope.map(|scope| scope.roots()));
            if let Some(declared_roots) = applicability_roots {
                // A missing requested candidate stream cannot establish an empty domain.
                // Only actual complete-empty roots and declared unrequested flow roots do.
                if declared_roots
                    .iter()
                    .all(|root| names.contains(root.name()) || unrequested.contains(root.name()))
                {
                    let mut nonempty = false;
                    for root in &declared_roots {
                        let alias = if unrequested.contains(root.name()) {
                            root.name().to_owned()
                        } else if let Some(inputs) = selected {
                            inputs.validation_table(root)?
                        } else {
                            Self::validation_table(root, &frozen_tables)?.to_owned()
                        };
                        let mut rows = crate::sql::query(
                            &session,
                            &format!(
                                "SELECT id FROM {} LIMIT 1",
                                crate::consumed_rows::identifier(&alias)
                            ),
                        )
                        .await
                        .map_err(ModelError::codec)?
                        .execute_stream()
                        .await
                        .map_err(ModelError::codec)?;
                        while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                            self.cancellation.check()?;
                            nonempty |= batch.num_rows() != 0;
                        }
                        if nonempty {
                            break;
                        }
                    }
                    if !nonempty {
                    self.remember_premise(premise)?;
                continue;
                    }
                }
            }
            if let Some(input) = invariant
                .inputs
                .iter()
                .find(|input| !names.contains(input.name()) && !unrequested.contains(input.name()))
            {
                return Err(ModelError::Invalid(format!(
                    "{} requires validation premise {} outside scope",
                    invariant.name,
                    input.name()
                )));
            }
            if let Some(scope) = check.retrieval_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_retrieval::validate_retrieval(
                    &invariant,
                    scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = projection_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::normalize::validate_projections(
                    scope,
                    &invariant,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = normalization_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                use lctx_model::domain::normalized::admission::Scope;
                match scope {
                    Scope::Callables => {
                        crate::normalize::validate_callables(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Receivers => {
                        crate::normalize::validate_receivers(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Events => {
                        crate::normalize::validate_events(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Bindings => {
                        crate::normalize::validate_bindings(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                }
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = execution_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_execution::validate_execution(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = check.aspect_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_aspects::validate_aspects(
                    &invariant,
                    &scope,
                    tables,
                    &self.model,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = check.inventory_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_inventory::validate_inventory(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            if let Some(scope) = check.support_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_admission::validate_support(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                self.remember_premise(premise)?;
                continue;
            }
            for input in &invariant.inputs {
                let order = input
                    .order()
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                let selected_table;
                let table = if unrequested.contains(input.name()) {
                    input.name()
                } else if let Some(inputs) = selected {
                    selected_table = inputs.validation_table(input)?;
                    selected_table.as_str()
                } else {
                    Self::validation_table(input, &frozen_tables)?
                };
                let sql = format!(
                    "SELECT * FROM \"{}\"{}",
                    table,
                    if order.is_empty() {
                        String::new()
                    } else {
                        format!(" ORDER BY {order}")
                    }
                );
                let mut stream = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish()?;
            self.remember_premise(premise)?;
        }
        self.identity()
    }
    fn validation_premise(&self, invariant: &lctx_model::domain::Invariant,
        selected: Option<&CompletedInputs>, unrequested: &std::collections::BTreeSet<&'static str>, profile: Profile,
    ) -> Result<Option<ContentHash>, ModelError> {
        use lctx_model::domain::Key;
        let mut sink = lctx_model::domain::KeySink::new("compiler-checked-premises/v1");
        self.model.digest().encode(&mut sink);
        invariant.digest().encode(&mut sink);
        profile.name().to_string().encode(&mut sink);
        for input in &invariant.inputs {
            if unrequested.contains(input.name()) {
                sink.part(b"unrequested", input.name().as_bytes());
                continue;
            }
            let relation = if let Some(selected) = selected {
                selected.relations.get(&(input.name(), input.prefix())).cloned().or_else(|| {
                    let mut choices = selected.relations.iter().filter(|((name, _), _)| *name == input.name()).map(|(_, relation)| relation);
                    let first = choices.next()?;
                    choices.all(|other| other.view == first.view).then(|| first.clone())
                })
            } else { self.input_relation(input).ok() };
            let Some(relation) = relation else { return Ok(None); };
            relation.view.identity.encode(&mut sink);
            relation.snapshot.identity().encode(&mut sink);
        }
        Ok(Some(sink.finish()))
    }
    fn remember_premise(&self, premise: Option<ContentHash>) -> Result<(), ModelError> {
        if let Some(premise) = premise {
            let mut checked = self.checked_premises.lock().map_err(|_| poisoned())?;
            if !checked.0.contains(&premise) { checked.1.grow(128)?; checked.0.insert(premise); }
        }
        Ok(())
    }
    /// Independent frontier obligations use the actual profile and retained capture/method
    /// definitions. They require no producer replay or historical publication receipts.
    pub async fn admit_frontier(
        &self,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
    ) -> Result<(), ModelError> {
        use lctx_model::domain::admission::Frontier;
        let ids: &[&str] = match frontier {
            Frontier::Facts | Frontier::Normalized => &[],
            Frontier::Conformance => {
                return Err(ModelError::Frontier(
                    "diagnostic conformance is not a complete artifact".into(),
                ));
            }
            Frontier::Analysis => &["complete_analysis_frontier"],
            Frontier::Catalog => &["complete_analysis_frontier", "complete_catalog_frontier"],
        };
        let relations = self.completed_relations()?;
        let access = self.inputs(
            "frontier-admission",
            profile,
            relations.iter().map(|r| r.name()),
        )?;
        let session = access.session(self).await?;
        for id in ids {
            let invariant = self.model.publication_check(id)?;
            let mut check = (invariant.create)(self.budget());
            for input in &invariant.inputs {
                let mut stream = crate::sql::query(
                    &session,
                    &format!("SELECT * FROM \"{}\" ORDER BY id", input.name()),
                )
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish(&[], profile)?;
        }
        Ok(())
    }
    async fn validation_views(
        &self,
        session: &SessionContext,
    ) -> Result<
        BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            String,
        >,
        ModelError,
    > {
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?.clone();
        let mut tables = BTreeMap::new();
        for ((boundary, name), source) in &frozen {
            let table = format!("_frozen_{}_{name}", boundary.name().to_ascii_lowercase());
            session.register_table(&table, self.native.table_provider(&source.view, source.relation.clone(), self.budget.clone(), self.options.batch_rows)?).map_err(ModelError::codec)?;
            tables.insert((*boundary, *name), table);
        }
        Ok(tables)
    }
    fn validation_table<'a>(
        input: &lctx_model::domain::ValidationInput,
        tables: &'a BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            String,
        >,
    ) -> Result<&'a str, ModelError> {
        match input.prefix() {
            Some(boundary) if !tables.is_empty() => tables
                .get(&(boundary, input.name()))
                .map(String::as_str)
                .ok_or_else(|| {
                    ModelError::Invalid(format!("missing frozen validation input {}", input.name()))
                }),
            _ => Ok(input.name()),
        }
    }
    /// Explicit fixture replay for selected owner source/coverage contracts. Artifact admission
    /// does not automatically recompute every compiler algorithm.
    pub async fn validate_publications(&self, ids: &[&str]) -> Result<(), ModelError> {
        let relations = self.completed_relations()?;
        let inputs = self.inputs(
            "selected-publication-controls",
            Profile::Catalog,
            relations.iter().map(|r| r.name()),
        )?;
        let session = inputs.session(self).await?;
        let frozen_tables = self.validation_views(&session).await?;
        let mut checks = BTreeMap::new();
        for source in &relations {
            for id in source
                .relation
                .publication_refs()
                .iter()
                .filter(|id| ids.contains(id))
            {
                if let Some(previous) = checks.insert(*id, source.clone())
                    && (previous.producer != source.producer || previous.inputs != source.inputs)
                {
                    return Err(ModelError::Invalid(
                        "publication invariant spans incompatible producer inputs".into(),
                    ));
                }
            }
        }
        if ids.iter().any(|id| !checks.contains_key(id)) {
            return Err(ModelError::Invalid(
                "selected publication control is absent".into(),
            ));
        }
        for (id, source) in checks {
            let invariant = self.model.publication_check(id)?;
            let mut check = (invariant.create)(self.budget());
            for input in &invariant.inputs {
                let order = input
                    .order()
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                let table = Self::validation_table(input, &frozen_tables)?;
                let sql = format!(
                    "SELECT * FROM \"{}\"{}",
                    table,
                    if order.is_empty() {
                        String::new()
                    } else {
                        format!(" ORDER BY {order}")
                    }
                );
                let mut stream = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish(&source.inputs, source.profile)?;
        }
        Ok(())
    }
    async fn validate_references(&self, session:&SessionContext, relations:&[Arc<CompletedRelation>], inputs:&CompletedInputs) -> Result<(),ModelError> {
        let quote=|name:&str|format!("\"{}\"",name.replace('"',"\"\""));
        for source in relations {
            for field in source.relation.fields().iter().filter(|field|!field.list()) {
                let Some((_,target))=field.target() else {continue;};
                let targets=inputs.relations.iter().filter(|((name,_),_)|*name==target).collect::<Vec<_>>();
                let target_source=targets.first().map(|(_,source)|*source);
                let mut key=lctx_model::domain::KeySink::new("compiler-reference-premise/v1");
                key.part(b"model",&self.model.digest().0);
                key.part(b"source",&source.view_identity().0);
                key.part(b"field",field.name().as_bytes());
                let target_views=targets.iter().map(|(_,source)|source.view_identity()).collect::<std::collections::BTreeSet<_>>();
                for view in target_views {key.part(b"target",&view.0);}
                if targets.is_empty(){key.part(b"missing-target",target.as_bytes());}
                let premise=key.finish();
                if self.checked_premises.lock().map_err(|_|poisoned())?.0.contains(&premise){continue;}
                let source_key=inputs.relations.iter().find(|(_,bound)|bound.view==source.view).map(|(key,_)|key).ok_or(ModelError::Schema("selected reference source binding"))?;
                let source_name=quote(&CompletedInputs::table(source_key.0,source_key.1));let field_name=quote(field.name());
                let sql=if let Some(target_source)=target_source {
                    let subtype=if let Some(tag)=field.subtype() {
                        let tag_field=target_source.relation.sum().ok_or(ModelError::Schema("nominal subtype target"))?.tag;
                        format!(" OR t.{} IS NULL OR t.{}<>{tag}",quote(tag_field),quote(tag_field))
                    } else {String::new()};
                    let target_tables=targets.iter().map(|((name,prefix),_)|format!("SELECT * FROM {}",quote(&CompletedInputs::table(name,*prefix)))).collect::<Vec<_>>().join(" UNION ALL ");
                    format!("SELECT s.{field_name} FROM {source_name} s LEFT JOIN ({target_tables}) t ON s.{field_name}=t.id WHERE s.{field_name} IS NOT NULL AND (t.id IS NULL{subtype}) LIMIT 1")
                } else {format!("SELECT {field_name} FROM {source_name} WHERE {field_name} IS NOT NULL LIMIT 1")};
                let mut stream=crate::sql::query(session,&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    if batch.num_rows()>0 {return Err(ModelError::Invalid(format!("missing or wrong-subtype nominal reference {}.{} -> {target}",source.name(),field.name())));}
                }
                self.remember_premise(Some(premise))?;
            }
        }
        Ok(())
    }
    pub(crate) async fn native_call<T:Send+'static>(&self,future:impl Future<Output=Result<T,ModelError>>+Send+'static)->Result<T,ModelError>{self.native_calls.call(future).await}
    pub fn native(&self) -> &Arc<lctx_surrealdb::compiler::NativeCompilerStore> { &self.native }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub fn options(&self) -> WorkspaceOptions {
        self.options
    }
    pub fn model(&self) -> &Arc<ValidatedModel> {
        &self.model
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    /// Drain abandoned provider threads before discarding attempt-owned native state.
    pub async fn drain(&self)->Result<(),ModelError>{
        self.cancellation.cancel();
        let drains=self.provider_drains.lock().map_err(|_|poisoned())?.clone();
        let mut error=None;
        for task in drains{if let Err(failure)=task.await{error.get_or_insert_with(||ModelError::Invalid(failure.to_string()));}}
        for result in [self.bridge.drain().await,self.native_calls.drain().await,self.native.drain().await]{if let Err(failure)=result{error.get_or_insert(failure);}}
        error.map_or(Ok(()),Err)
    }
    pub fn completed<R: Record>(&self) -> Result<Arc<CompletedRelation>, ModelError> {
        self.relation(R::NAME)
    }
    pub fn relation(&self, name: &str) -> Result<Arc<CompletedRelation>, ModelError> {
        self.completed
            .lock()
            .map_err(|_| poisoned())?
            .get(name)
            .cloned()
            .ok_or_else(|| ModelError::Invalid(format!("input {name} is not completed")))
    }
    /// Resolve a replay's declared completed view without widening it to later shared values.
    pub fn input_relation(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<Arc<CompletedRelation>, ModelError> {
        if let Some(boundary) = input.prefix() {
            return self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .get(&(boundary, input.name()))
                .cloned()
                .ok_or_else(|| {
                    ModelError::Invalid(format!(
                        "missing frozen {:?} input {}",
                        boundary,
                        input.name()
                    ))
                });
        }
        self.relation(input.name())
    }
    pub fn completed_relations(&self) -> Result<Vec<Arc<CompletedRelation>>, ModelError> {
        Ok(self
            .completed
            .lock()
            .map_err(|_| poisoned())?
            .values()
            .cloned()
            .collect())
    }
    /// Build an explicit immutable input set. Later completions do not change this view.
    pub fn inputs(
        &self,
        name: &'static str,
        profile: Profile,
        names: impl IntoIterator<Item = &'static str>,
    ) -> Result<CompletedInputs, ModelError> {
        let mut relations = BTreeMap::new();
        for name in names {
            relations.insert((name, None), self.relation(name)?);
        }
        Ok(CompletedInputs {
            name,
            profile,
            relations,
        })
    }
    /// Bind each declared semantic boundary to immutable shared streams. Only small
    /// descriptors are retained; later contributions cannot widen an earlier producer's inputs.
    pub async fn freeze_inputs_async(&self,boundary:lctx_model::domain::stages::PublicationBoundary)->Result<(),ModelError>{
        let _completion=self.completion_gate.lock().await;
        if self.frozen_shared.lock().map_err(|_|poisoned())?.keys().any(|(existing,_)|*existing==boundary){return Err(ModelError::Conflict("compiler input boundary already frozen"));}
        let sources=self.completed.lock().map_err(|_|poisoned())?.iter().filter(|(name,_)|lctx_model::domain::stages::is_epoch_shared(name)).map(|(name,source)|(*name,source.clone())).collect::<Vec<_>>();
        for (_,source) in &sources {
            let native=self.native.clone();
            let binding=lctx_model::domain::completed::CompletedBinding{boundary:Some(boundary.name().into()),source:source.snapshot(),view:source.view.clone(),configuration:None};
            self.native_calls.call(async move{native.bind(binding).await}).await?;
        }
        let mut frozen=self.frozen_shared.lock().map_err(|_|poisoned())?;
        for (name,source) in sources{frozen.insert((boundary,name),source);}
        Ok(())
    }
    pub fn freeze_inputs(
        &self,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        let completed = self.completed.lock().map_err(|_| poisoned())?;
        let mut frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        if frozen.keys().any(|(existing, _)| *existing == boundary) {
            return Err(ModelError::Invalid(
                "compiler input boundary already frozen".into(),
            ));
        }
        for (name, source) in completed
            .iter()
            .filter(|(name, _)| lctx_model::domain::stages::is_epoch_shared(name))
        {
            let native = self.native.clone();
            let binding = lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()), source: source.snapshot(), view: source.view.clone(), configuration: None,
            };
            let calls=self.native_calls.clone();
            self.bridge.call(async move{calls.call(async move{native.bind(binding).await}).await})?;
            frozen.insert((boundary, *name), source.clone());
        }
        Ok(())
    }
    pub fn stage_inputs(
        &self,
        declaration: &lctx_model::domain::stages::Stage,
        profile: Profile,
    ) -> Result<CompletedInputs, ModelError> {
        self.stage_inputs_selected(declaration, declaration, profile)
    }
    pub(crate) fn stage_inputs_selected(
        &self,
        declaration: &lctx_model::domain::stages::Stage,
        selected: &lctx_model::domain::stages::Stage,
        profile: Profile,
    ) -> Result<CompletedInputs, ModelError> {
        if declaration.inputs.len() != selected.inputs.len() {
            return Err(ModelError::Invalid(
                "selected compiler inputs differ from declaration".into(),
            ));
        }
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let mut relations = BTreeMap::new();
        for (declared, input) in declaration.inputs.iter().zip(&selected.inputs) {
            if declared.name() != input.name() {
                return Err(ModelError::Invalid(
                    "selected compiler input type changed".into(),
                ));
            }
            let source = if let Some(boundary) = input.prefix() {
                frozen
                    .get(&(boundary, input.name()))
                    .cloned()
                    .ok_or_else(|| {
                        ModelError::Invalid(format!(
                            "{} lacks frozen {:?} input {}",
                            declaration.name,
                            boundary,
                            input.name()
                        ))
                    })?
            } else {
                self.relation(input.name())?
            };
            if relations
                .insert((declared.name(), declared.prefix()), source)
                .is_some()
            {
                return Err(ModelError::Invalid(
                    "duplicate selected compiler input".into(),
                ));
            }
        }
        Ok(CompletedInputs {
            name: declaration.name,
            profile,
            relations,
        })
    }
    /// Plan the complete relation inventory before registering or writing a contribution.
    /// An empty inventory is a metadata-only contribution, never an unknown inventory.
    pub fn output(
        self: &Arc<Self>,
        name: &'static str,
        profile: Profile,
        implementation: ContentHash,
        inputs: CompletedInputs,
        outputs: impl IntoIterator<Item = &'static str>,
    ) -> ProducerOutput {
        let outputs: std::collections::BTreeSet<_> = outputs.into_iter().collect();
        ProducerOutput {
            workspace: self.clone(),
            name,
            profile,
            implementation,
            configuration: None,
            inputs,
            expected: outputs.clone(),
            allowed: outputs,
            writers: Mutex::default(),
            outcome: Mutex::new(None),
            contribution: Mutex::new(None),
            registration: tokio::sync::Mutex::new(()),
            failed: AtomicBool::new(false),
        }
    }
    pub fn producer(
        self: &Arc<Self>,
        declaration: &lctx_model::domain::stages::Stage,
        profile: Profile,
        inputs: CompletedInputs,
    ) -> ProducerOutput {
        let mut output = self.output(declaration.name, profile, declaration.code, inputs, declaration.outputs.iter().chain(&declaration.contributes).map(|relation| relation.name()));
        output.configuration = Some(declaration.configuration);
        output.expected = declaration.outputs.iter().map(|r| r.name()).collect();
        output
    }

}
fn poisoned() -> ModelError {
    ModelError::Invalid("workspace ownership poisoned".into())
}

/// A stream descriptor, not a resident collection or a database read capability.
pub struct CompletedRelation {
    relation: Relation,
    producer: String,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    contract: ContentHash,
    rows: u64,
    view: lctx_model::domain::completed::CompletedView,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    bridge: Arc<crate::native_bridge::NativeBridge>,
    budget: ResourceBudget,
    batch_rows: usize,
    cancellation: Cancellation,
    inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>,
    profile: Profile,
    contribution: bool,
    snapshot: lctx_model::domain::analysis::sources::SourceSnapshot,
    _files: Arc<WorkspaceFiles>,
}
impl CompletedRelation {
    pub fn name(&self) -> &'static str {
        self.relation.name()
    }
    pub fn view(&self) -> &lctx_model::domain::completed::CompletedView { &self.view }
    pub fn view_identity(&self) -> ContentHash {
        self.view.identity
    }
    pub fn rows(&self) -> u64 {
        self.rows
    }
    pub fn producer(&self) -> &str {
        &self.producer
    }
    pub fn implementation(&self) -> ContentHash {
        self.implementation
    }
    pub fn configuration(&self) -> Option<ContentHash> {
        self.configuration
    }
    pub fn contract(&self) -> ContentHash {
        self.contract
    }
    pub fn snapshot(&self) -> lctx_model::domain::analysis::sources::SourceSnapshot {
        self.snapshot.clone()
    }
    pub fn batches(&self) -> Result<NativeBatches, ModelError> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let native = self.native.clone(); let view = self.view.clone(); let relation = self.relation.clone();
        let budget = self.budget.clone(); let batch_rows = self.batch_rows;
        let cancellation = self.cancellation.clone();
        self.bridge.launch(Box::pin(async move {
            let result = async {
                let mut stream = native.scan_batches(&view, &relation, None, None, &budget, batch_rows).await?;
                loop {
                    let batch=tokio::select! {
                        ()=cancellation.cancelled()=>return Ok(()),
                        batch=stream.try_next()=>batch.map_err(ModelError::codec)?,
                    };
                    let Some(batch)=batch else{break;};
                    cancellation.check()?;
                    let charge=budget.reserve("native-provider-handoff",lctx_model::domain::logical_batch_bytes(&batch)?)?;
                    tokio::select! {
                        ()=cancellation.cancelled()=>return Ok(()),
                        result=sender.send(Ok((batch,charge)))=>if result.is_err(){return Ok(());},
                    }
                }
                Ok::<(), ModelError>(())
            }.await;
            if let Err(error)=result {
                tokio::select! {
                    ()=cancellation.cancelled()=>{},
                    _=sender.send(Err(error))=>{},
                }
            }
        }))?;
        Ok(NativeBatches { receiver, cancellation: self.cancellation.clone(), charge: None, done:false })
    }
    pub fn read<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        Ok(TypedBatches {
            reader: self.batches()?,
            model,
            budget,
            marker: Default::default(),
            _files: self._files.clone(),
        })
    }
}
pub struct NativeBatches {
    receiver: tokio::sync::mpsc::Receiver<ChargedBatch>,
    cancellation: Cancellation,
    charge: Option<Box<dyn Reservation>>,
    done:bool,
}
impl Iterator for NativeBatches {
    type Item = Result<RecordBatch, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done{return None;}
        self.charge = None;
        loop {
            if let Err(error) = self.cancellation.check() { self.receiver.close(); self.done=true; return Some(Err(error)); }
            match self.receiver.try_recv() {
                Ok(Ok((batch, charge))) => { self.charge = Some(charge); return Some(Ok(batch)); },
                Ok(Err(error)) => { self.receiver.close(); self.done=true; return Some(Err(error)); },
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {self.done=true;return None;},
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
    }
}
pub struct TypedBatches<R: Record> {
    reader: NativeBatches,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    marker: std::marker::PhantomData<R>,
    _files: Arc<WorkspaceFiles>,
}
impl<R: Record> Iterator for TypedBatches<R> {
    type Item = Result<Batch<R>, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next().map(|batch| {
            batch
                .map_err(ModelError::codec)
                .and_then(|batch| Batch::read(&self.model, &batch, &self.budget))
        })
    }
}
/// Privately constructed after owner admission succeeds. Its immutable descriptors keep their
/// attempt files alive. Consumers compare descriptors and policy, without rehashing input bytes.
#[derive(Clone)]
pub struct CheckedInputs {
    inputs: CompletedInputs,
    attempt: Arc<WorkspaceFiles>,
    policy: ContentHash,
}
impl CheckedInputs {
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    /// Project an admitted authority onto an exact immutable dependency subset. No rows are
    /// read again; the selected descriptors retain the same attempt and admission policy.
    pub fn select(
        &self,
        required: &[lctx_model::domain::ValidationInput],
    ) -> Result<Self, ModelError> {
        Ok(Self {
            inputs: self.inputs.select(required)?,
            attempt: self.attempt.clone(),
            policy: self.policy,
        })
    }
    pub fn require(
        &self,
        workspace: &Workspace,
        inputs: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if self.inputs.relations.len() != inputs.relations.len() {
            return Err(ModelError::Conflict("checked compiler input domain"));
        }
        self.require_subset(workspace, inputs)
    }
    /// A consumer may declare additional inputs; every descriptor underlying this authority must
    /// still be the identical completed relation in its selected dependency closure.
    pub fn require_subset(
        &self,
        workspace: &Workspace,
        inputs: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if !Arc::ptr_eq(&self.attempt, &workspace.files)
            || self.policy != workspace.model.digest()
            || self.inputs.profile != inputs.profile
            || self.inputs.relations.iter().any(|(key, source)| {
                inputs
                    .relations
                    .get(key)
                    .is_none_or(|other| !Arc::ptr_eq(source, other))
            })
        {
            return Err(ModelError::Conflict("checked compiler inputs"));
        }
        workspace.cancellation.check()
    }
}
#[derive(Clone)]
pub struct CompletedInputs {
    name: &'static str,
    profile: Profile,
    relations: BTreeMap<
        (
            &'static str,
            Option<lctx_model::domain::stages::PublicationBoundary>,
        ),
        Arc<CompletedRelation>,
    >,
}
impl CompletedInputs {
    /// Check lifetime and identity of an actual producer's immutable streams. This does not
    /// admit semantic contents: only the model owner's opaque produced value carries authority.
    pub fn require_subset(
        &self,
        workspace: &Workspace,
        consumer: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if self.profile != consumer.profile
            || self.relations.is_empty()
            || self.relations.iter().any(|(key, source)| {
                !Arc::ptr_eq(&source._files, &workspace.files)
                    || consumer
                        .relations
                        .get(key)
                        .is_none_or(|other| !Arc::ptr_eq(source, other))
            })
        {
            return Err(ModelError::Conflict(
                "completed producer dependency identity",
            ));
        }
        workspace.cancellation.check()
    }
    /// Resolve the exact table selected by a model-owned validation declaration.
    pub fn table_for(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<String, ModelError> {
        self.validation_table(input)
    }
    fn validation_table(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<String, ModelError> {
        let key = (input.name(), input.prefix());
        if self.relations.contains_key(&key) {
            return Ok(Self::table(key.0, key.1));
        }
        if input.prefix().is_some() {
            return Err(ModelError::Conflict(
                "checked admission requires a missing input epoch",
            ));
        }
        let mut choices = self
            .relations
            .iter()
            .filter(|((name, _), _)| *name == input.name());
        let (key, source) = choices
            .next()
            .ok_or(ModelError::Conflict("checked admission input is absent"))?;
        if choices.any(|(_, other)| !Arc::ptr_eq(source, other)) {
            return Err(ModelError::Conflict(
                "checked admission input epoch is ambiguous",
            ));
        }
        Ok(Self::table(key.0, key.1))
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    /// Select a model-declared immutable dependency closure without copying or rehashing rows.
    pub fn select(
        &self,
        required: &[lctx_model::domain::ValidationInput],
    ) -> Result<Self, ModelError> {
        let mut relations = BTreeMap::new();
        for input in required {
            let key = (input.name(), input.prefix());
            let source = if let Some(source) = self.relations.get(&key) {
                source
            } else {
                if input.prefix().is_some() {
                    return Err(ModelError::Conflict("missing checked input prefix"));
                }
                let mut choices = self
                    .relations
                    .iter()
                    .filter(|((name, _), _)| *name == input.name());
                let (_, source) = choices
                    .next()
                    .ok_or(ModelError::Conflict("missing checked input"))?;
                if choices.any(|(_, other)| !Arc::ptr_eq(source, other)) {
                    return Err(ModelError::Conflict("ambiguous checked input"));
                }
                source
            };
            // Retain the original declaration key so later subset checks cannot erase epochs.
            let original = self
                .relations
                .iter()
                .find(|((name, prefix), candidate)| {
                    *name == input.name()
                        && (input.prefix().is_none() || *prefix == input.prefix())
                        && Arc::ptr_eq(source, candidate)
                })
                .ok_or(ModelError::Conflict("checked input selector"))?
                .0;
            relations.insert(*original, source.clone());
        }
        if relations.is_empty() {
            return Err(ModelError::Conflict("empty checked input closure"));
        }
        Ok(Self {
            name: self.name,
            profile: self.profile,
            relations,
        })
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn contains<R: Record>(&self) -> bool {
        self.relations.keys().any(|(name, _)| *name == R::NAME)
    }
    pub fn relation<R: Record>(&self) -> Result<&Arc<CompletedRelation>, ModelError> {
        let mut sources = self
            .relations
            .iter()
            .filter(|((name, _), _)| *name == R::NAME)
            .map(|(_, source)| source);
        let source = sources.next().ok_or_else(|| {
            ModelError::Invalid(format!("{} lacks explicit input {}", self.name, R::NAME))
        })?;
        if sources.any(|other| other.view.identity != source.view.identity) {
            return Err(ModelError::Invalid(format!(
                "{} must select a completed view of {}",
                self.name,
                R::NAME
            )));
        }
        Ok(source)
    }
    pub fn relation_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<&Arc<CompletedRelation>, ModelError> {
        if let Some(source) = self.relations.get(&(R::NAME, prefix)) {
            return Ok(source);
        }
        if prefix.is_none() {
            return self.relation::<R>();
        }
        Err(ModelError::Invalid(format!(
            "{} lacks declared {:?} input {}",
            self.name,
            prefix,
            R::NAME
        )))
    }
    pub fn read<R: Record>(
        &self,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        Self::read_source(self.relation::<R>()?)
    }
    pub fn read_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        Self::read_source(self.relation_at::<R>(prefix)?)
    }
    fn read_source<R: Record>(
        source: &CompletedRelation,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        lctx_model::domain::analysis::sources::CompletedInput::new(
            source.snapshot.producer(),
            source.snapshot.model(),
            source.snapshot.implementation(),
            source.view_identity(),
            source.rows(),
        )
    }
    fn table(
        name: &str,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> String {
        format!(
            "_view_{}_{}",
            prefix
                .map(|p| p.name())
                .unwrap_or("default")
                .to_ascii_lowercase(),
            name
        )
    }
    pub fn table_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<String, ModelError> {
        let selected = self.relation_at::<R>(prefix)?;
        let actual_prefix = if self.relations.contains_key(&(R::NAME, prefix)) {
            prefix
        } else {
            self.relations
                .iter()
                .find(|((name, _), source)| *name == R::NAME && source.view.identity == selected.view.identity)
                .map(|((_, prefix), _)| *prefix)
                .expect("selected declared source")
        };
        Ok(Self::table(R::NAME, actual_prefix))
    }
    pub fn snapshots(
        &self,
    ) -> impl Iterator<Item = lctx_model::domain::analysis::sources::SourceSnapshot> + '_ {
        self.relations.values().map(|source| source.snapshot())
    }
    pub fn relations(&self) -> impl Iterator<Item = &Arc<CompletedRelation>> {
        self.relations.values()
    }
    pub async fn session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        let context = SessionContext::new_with_config_rt(
            workspace.context.copied_config(),
            workspace.context.runtime_env(),
        );
        for ((name, prefix), source) in &self.relations {
            let table = Self::table(name, *prefix);
            context.register_table(&table, workspace.native.table_provider(&source.view, source.relation.clone(), workspace.budget.clone(), workspace.options.batch_rows)?).map_err(ModelError::codec)?;
        }
        // A plain typed read is available only when every declared selector names the same
        // immutable stream. Distinct semantic views require their qualified alias.
        let names = self
            .relations
            .keys()
            .map(|(name, _)| *name)
            .collect::<std::collections::BTreeSet<_>>();
        for name in names {
            let mut sources = self
                .relations
                .iter()
                .filter(|((candidate, _), _)| *candidate == name)
                .map(|(_, source)| source);
            let source = sources.next().expect("declared input");
            if sources.all(|other| other.view.identity == source.view.identity) {
                context.register_table(name, workspace.native.table_provider(&source.view, source.relation.clone(), workspace.budget.clone(), workspace.options.batch_rows)?).map_err(ModelError::codec)?;
            }
        }
        Ok(context)
    }
}

struct PendingRelation {
    relation: Relation,
    contribution: bool,
}
trait ErasedWriter: Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn close(
        self: Box<Self>,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> futures::future::BoxFuture<'static,Result<PendingRelation, ModelError>>;
}
struct Writer<R: Record> {
    native_calls:Arc<crate::native_calls::NativeCalls>,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    bridge: Arc<crate::native_bridge::NativeBridge>,
    producer: ContentHash,
    pending: Vec<R>,
    bytes: usize,
    charge: Box<dyn Reservation>,
    limits: TransferLimits,
    contribution: bool,
}
impl<R: Record> Writer<R> {
    fn write_arrow(&mut self, batch: &RecordBatch) -> Result<(), ModelError> {
        let native = self.native.clone(); let producer = self.producer; let relation = Relation::of::<R>(); let batch = batch.clone();
        let calls=self.native_calls.clone();
        self.bridge.call(async move{calls.call(async move{native.write_batch(&producer,&relation,&batch).await}).await})
    }
    fn flush(&mut self, model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.pending.is_empty() {
            // Transfer the existing row reservation into encoding; these are the same rows,
            // rather than an additional resident copy of the pending batch.
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(model, std::mem::take(&mut self.pending), charge)?;
            self.write_arrow(batch.arrow())?;
            self.bytes = 0;
        }
        Ok(())
    }
}
impl<R: Record> ErasedWriter for Writer<R> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(mut self: Box<Self>, model: Arc<ValidatedModel>, budget: ResourceBudget) -> futures::future::BoxFuture<'static,Result<PendingRelation,ModelError>> {
        Box::pin(async move {
            if !self.pending.is_empty() {
                let charge=std::mem::replace(&mut self.charge,budget.reserve(R::NAME,0)?);
                let batch=Batch::with_reservation(&model,std::mem::take(&mut self.pending),charge)?;
                let native=self.native.clone();let producer=self.producer;
                self.native_calls.call(async move{native.write_batch(&producer,&Relation::of::<R>(),batch.arrow()).await}).await?;
            }
            Ok(PendingRelation{relation:Relation::of::<R>(),contribution:self.contribution})
        })
    }
}
/// One producer owns pending streams; completion makes its whole output set visible atomically.
pub struct ProducerOutput {
    workspace: Arc<Workspace>,
    name: &'static str,
    profile: Profile,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    inputs: CompletedInputs,
    expected: std::collections::BTreeSet<&'static str>,
    allowed: std::collections::BTreeSet<&'static str>,
    writers: Mutex<BTreeMap<&'static str, Box<dyn ErasedWriter>>>,
    outcome: Mutex<Option<ProviderOutcome>>,
    contribution: Mutex<Option<ContentHash>>,
    registration: tokio::sync::Mutex<()>,
    failed: AtomicBool,
}
impl ProducerOutput {
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn workspace(&self) -> &Arc<Workspace> {
        &self.workspace
    }
    fn check(&self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        self.workspace.writable()?;
        if self.failed.load(Ordering::Acquire)
            || self.outcome.lock().map_err(|_| poisoned())?.is_some()
        {
            return Err(ModelError::Invalid(
                "producer output is closed or failed".into(),
            ));
        }
        Ok(())
    }
    fn guarded<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        let result = self.check().and_then(|_| operation());
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    /// Asynchronous compiler consumers register natively without blocking their runtime.
    pub async fn declare_async<R:Record>(&self)->Result<(),ModelError>{
        let _registration=self.registration.lock().await;
        self.check()?;
        let absent=self.contribution.lock().map_err(|_|poisoned())?.is_none();
        if absent {
            let descriptor=lctx_model::domain::completed::ContributionSpec{
                producer:self.name.into(),profile:self.profile,model:self.workspace.model.digest(),implementation:self.implementation,
                configuration:self.configuration,inputs:self.inputs.snapshots().collect(),
                outputs:self.allowed.iter().map(|name|(*name).to_owned()).collect(),
            };
            let native=self.workspace.native.clone();
            let result=self.workspace.native_calls.call(async move{native.begin_contribution(descriptor).await}).await;
            match result {
                Ok(id)=>*self.contribution.lock().map_err(|_|poisoned())?=Some(id),
                Err(error)=>{self.failed.store(true,Ordering::Release);return Err(error);}
            }
        }
        self.declare::<R>()
    }
    pub fn declare<R: Record>(&self) -> Result<(), ModelError> {
        self.declare_kind::<R>(lctx_model::domain::stages::is_epoch_shared(R::NAME))
    }
    fn declare_kind<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.guarded(|| self.declare_kind_inner::<R>(contribution))
    }
    fn declare_kind_inner<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.check()?;
        if !self.allowed.contains(R::NAME) {
            self.failed.store(true, Ordering::Release);
            return Err(ModelError::Invalid(format!(
                "{} did not declare {}",
                self.name,
                R::NAME
            )));
        }
        self.workspace.model.require::<R>()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        if writers.contains_key(R::NAME) {
            return Err(ModelError::Invalid(format!(
                "output {} declared twice",
                R::NAME
            )));
        }
        let mut contribution_id = self.contribution.lock().map_err(|_| poisoned())?;
        let producer = if let Some(id) = *contribution_id { id } else {
            let native = self.workspace.native.clone();
            let descriptor = lctx_model::domain::completed::ContributionSpec {
                producer: self.name.into(), profile: self.profile, model: self.workspace.model.digest(),
                implementation: self.implementation, configuration: self.configuration,
                inputs: self.inputs.snapshots().collect(),
                outputs: self.allowed.iter().map(|name| (*name).to_owned()).collect(),
            };
            let calls=self.workspace.native_calls.clone();
            let id=self.workspace.bridge.call(async move{calls.call(async move{native.begin_contribution(descriptor).await}).await})?;
            *contribution_id = Some(id); id
        };
        writers.insert(
            R::NAME,
            Box::new(Writer::<R> {
                native_calls:self.workspace.native_calls.clone(),native: self.workspace.native.clone(), bridge: self.workspace.bridge.clone(), producer,
                pending: Vec::new(),
                bytes: 0,
                charge: self.workspace.budget().reserve(R::NAME, 0)?,
                limits: TransferLimits {
                    rows: self.workspace.options.batch_rows,
                    bytes: (self.workspace.options.memory_bytes / 8)
                        .clamp(1, lctx_model::domain::resources::TRANSFER_BYTES),
                    ..Default::default()
                },
                contribution,
            }),
        );
        Ok(())
    }
    pub fn write<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.guarded(|| self.write_inner(batch))
    }
    fn write_inner<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers
            .get_mut(R::NAME)
            .and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>())
            .ok_or_else(|| ModelError::Invalid(format!("output {} was not declared", R::NAME)))?;
        writer.flush(&self.workspace.model, self.workspace.budget())?;
        let result = writer.write_arrow(batch.arrow());
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn contribute<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        if !self
            .writers
            .lock()
            .map_err(|_| poisoned())?
            .contains_key(R::NAME)
        {
            self.declare_kind::<R>(true)?;
        }
        self.write(batch)
    }
    pub async fn push<R: Record>(&self, row: R) -> Result<(), ModelError> {
        let result=async {
            self.check()?; row.validate()?;
            let transfer={
                let mut writers=self.writers.lock().map_err(|_|poisoned())?;
                let writer=writers.get_mut(R::NAME).and_then(|writer|writer.as_any_mut().downcast_mut::<Writer<R>>()).ok_or(ModelError::Schema("undeclared async output"))?;
                let bytes=row.row_bytes();
                if bytes>writer.limits.max_row {return Err(ModelError::Invalid(format!("{} exceeds row limit",R::NAME)));}
                let transfer=if !writer.pending.is_empty() && (writer.pending.len()>=writer.limits.rows || writer.bytes.saturating_add(bytes)>writer.limits.bytes) {
                    let charge=std::mem::replace(&mut writer.charge,self.workspace.budget.reserve(R::NAME,0)?);
                    let batch=Batch::with_reservation(&self.workspace.model,std::mem::take(&mut writer.pending),charge)?;
                    writer.bytes=0;
                    Some((writer.producer,batch))
                } else {None};
                writer.charge.try_resize(writer.charge.size().saturating_add(bytes))?;
                writer.pending.push(row); writer.bytes+=bytes;
                transfer
            };
            if let Some((producer,batch))=transfer {
                let native=self.workspace.native.clone();
                self.workspace.native_calls.call(async move{native.write_batch(&producer,&Relation::of::<R>(),batch.arrow()).await}).await?;
            }
            Ok(())
        }.await;
        if result.is_err(){self.failed.store(true,Ordering::Release);}
        result
    }
    pub fn push_sync<R: Record>(&self, row: R) -> Result<(), ModelError> {
        let result = (|| {
            self.check()?;
            row.validate()?;
            let mut writers = self.writers.lock().map_err(|_| poisoned())?;
            let writer = writers
                .get_mut(R::NAME)
                .and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>())
                .ok_or_else(|| {
                    ModelError::Invalid(format!("output {} was not declared", R::NAME))
                })?;
            let bytes = row.row_bytes();
            if bytes > writer.limits.max_row {
                return Err(ModelError::Invalid(format!(
                    "{} exceeds row limit",
                    R::NAME
                )));
            }
            if !writer.pending.is_empty()
                && (writer.pending.len() >= writer.limits.rows
                    || writer.bytes.saturating_add(bytes) > writer.limits.bytes)
            {
                writer.flush(&self.workspace.model, self.workspace.budget())?;
            }
            writer
                .charge
                .try_resize(writer.charge.size().saturating_add(bytes))?;
            writer.pending.push(row);
            writer.bytes += bytes;
            Ok(())
        })();
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn mark_finished(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.check()?;
        if outcome == ProviderOutcome::Failed {
            self.failed.store(true, Ordering::Release);
            return Err(ModelError::Invalid(
                "failed producer cannot complete outputs".into(),
            ));
        }
        *self.outcome.lock().map_err(|_| poisoned())? = Some(outcome);
        Ok(())
    }
    pub async fn finish(self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.mark_finished(outcome)?;
        self.complete().await
    }
    pub async fn complete(self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        self.workspace.writable()?;
        if self.failed.load(Ordering::Acquire)
            || self.outcome.lock().map_err(|_| poisoned())?.is_none()
        {
            return Err(ModelError::Invalid(
                "producer did not complete successfully".into(),
            ));
        }
        let _completion = self.workspace.completion_gate.lock().await;
        self.workspace.writable()?;
        let writers = self.writers.into_inner().map_err(|_| poisoned())?;
        if let Some(name) = self.expected.iter().find(|name| !writers.contains_key(**name))
        {
            return Err(ModelError::Invalid(format!(
                "{} omitted completed output {name}",
                self.name
            )));
        }
        let input_snapshots: Arc<[_]> = self.inputs.snapshots().collect::<Vec<_>>().into();
        // Flush bounded pending typed batches directly before completing exact native memberships.
        let mut pending=Vec::new();
        for writer in writers.into_values() {
            pending.push(writer.close(self.workspace.model.clone(),self.workspace.budget.clone()).await?);
        }
        {
            for name in &self.allowed {
                if !pending.iter().any(|source| source.relation.name() == *name) {
                    let relation = self.workspace.model.relation(name).ok_or(ModelError::Schema("declared native contribution relation"))?.clone();
                    pending.push(PendingRelation { relation, contribution: true });
                }
            }
        }
        let id = match self.contribution.into_inner().map_err(|_| poisoned())? {
            Some(id) => id,
            None => {
                let descriptor=lctx_model::domain::completed::ContributionSpec {
                    producer:self.name.into(),profile:self.profile,model:self.workspace.model.digest(),implementation:self.implementation,
                    configuration:self.configuration,inputs:self.inputs.snapshots().collect(),
                    outputs:self.allowed.iter().map(|name|(*name).to_owned()).collect(),
                };
                let native=self.workspace.native.clone();
                self.workspace.native_calls.call(async move{native.begin_contribution(descriptor).await}).await?
            }
        };
        let previous = self.workspace.completed.lock().map_err(|_| poisoned())?.iter()
            .map(|(name, source)| ((*name).to_owned(), source.view.clone())).collect();
        for source in &pending {
            if let Ok(previous) = self.workspace.relation(source.relation.name())
                && !source.contribution && !previous.contribution
            { return Err(ModelError::Invalid(format!("completed output {} already has an owner", source.relation.name()))); }
        }
        let outputs = pending.iter().map(|source| source.relation.clone()).collect::<Vec<_>>();
        let outcome = self.outcome.into_inner().map_err(|_| poisoned())?.expect("checked producer outcome");
        let native=self.workspace.native.clone();
        let views=self.workspace.native_calls.call(async move{native.complete_contribution(id,outcome,&outputs,&previous).await}).await?;
        let mut completed = Vec::new();
        for pending in pending {
            let name = pending.relation.name();
            let view = views.get(name).ok_or(ModelError::Schema("completed native output absent"))?.clone();
            completed.push(Arc::new(CompletedRelation {
                snapshot: lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&pending.relation,self.workspace.model.digest(),&view)?,
                relation: pending.relation, producer: self.name.into(), implementation: self.implementation, configuration: self.configuration,
                contract: self.workspace.model.digest(), rows: view.rows, view,
                native: self.workspace.native.clone(), bridge: self.workspace.bridge.clone(), budget: self.workspace.budget.clone(),
                batch_rows: self.workspace.options.batch_rows, cancellation: self.workspace.cancellation.clone(),
                inputs: input_snapshots.clone(), profile: self.profile, contribution: pending.contribution, _files: self.workspace.files.clone(),
            }));
        }
        for source in &completed {
            let native=self.workspace.native.clone();
            let binding=lctx_model::domain::completed::CompletedBinding{boundary:None,source:source.snapshot(),view:source.view.clone(),configuration:None};
            self.workspace.native_calls.call(async move{native.bind(binding).await}).await?;
        }
        self.workspace.cancellation.check()?;
        let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
        for source in completed {
            visible.insert(source.name(), source);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct WorkspacePool {
    memory: Arc<dyn MemoryPool>,
    limit: usize,
}
#[derive(Debug)]
struct WorkspaceReservation {
    reservation: MemoryReservation,
    owner: &'static str,
    pool: Arc<dyn MemoryPool>,
    limit: usize,
}
impl ResourcePool for WorkspacePool {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reserved = WorkspaceReservation {
            reservation: MemoryConsumer::new(owner).register(&self.memory),
            owner,
            pool: self.memory.clone(),
            limit: self.limit,
        };
        reserved.try_resize(bytes)?;
        Ok(Box::new(reserved))
    }
    fn reserved(&self) -> usize {
        self.memory.reserved()
    }
    fn limit(&self) -> usize {
        self.limit
    }
}
impl Reservation for WorkspaceReservation {
    fn size(&self) -> usize {
        self.reservation.size()
    }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.reservation
            .try_resize(bytes)
            .map_err(|_| ModelError::Resource {
                owner: self.owner,
                requested: bytes.saturating_sub(self.reservation.size()),
                used: self.pool.reserved(),
                limit: self.limit,
            })
    }
}

impl cpg_extract::bundle::ProviderSink for ProducerOutput {
    fn cancel(&self) { self.workspace.cancellation.cancel(); }
    fn drain_provider(&self, thread: std::thread::JoinHandle<()>) -> tokio::sync::oneshot::Receiver<Result<(), ModelError>> {
        let (done, finished) = tokio::sync::oneshot::channel();
        let task = tokio::task::spawn_blocking(move || {
            let result=thread.join().map_err(|_|Arc::<str>::from("provider thread panicked while draining"));
            let _=done.send(result.as_ref().copied().map_err(|error|ModelError::Invalid(error.to_string())));
            result
        });
        let joined=async move{task.await.map_err(|error|Arc::<str>::from(error.to_string()))?}.boxed().shared();
        self.workspace.provider_drains.lock().expect("provider drainage ownership").push(joined);
        finished
    }
    fn read<R: Record>(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError> {
        self.check()?;
        Ok(Box::new(self.inputs.relation::<R>()?.read::<R>(
            self.workspace.model.clone(),
            self.workspace.budget.clone(),
        )?))
    }
    fn declare<R: Record>(&self) -> Result<(), ModelError> {
        ProducerOutput::declare::<R>(self)
    }
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::write(self, &batch)
    }
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::contribute(self, &batch)
    }
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.mark_finished(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::input::Package;
    fn model() -> Arc<ValidatedModel> {
        Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap())
    }
    async fn packages(memory: usize, batch: usize, reverse: bool) -> Arc<CompletedRelation> {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions {
                memory_bytes: memory,
                partitions: 1,
                batch_rows: batch,
            }, crate::test_native::store()
)
        .unwrap();
        let inputs = workspace.inputs("packages", Profile::Catalog, []).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            inputs,
        [<Package>::NAME],
        );
        output.declare::<Package>().unwrap();
        let rows: Vec<_> = (0..20000)
            .map(|n| Package {
                name: format!("package-{n:08}"),
            })
            .collect();
        if reverse {
            for row in rows.into_iter().rev() {
                output.push(row).await.unwrap();
            }
        } else {
            for row in rows {
                output.push(row).await.unwrap();
            }
        }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.completed::<Package>().unwrap()
    }
    #[tokio::test]
    async fn canonical_stream_is_independent_of_batching_order_and_memory_limit() {
        let ordinary = packages(64 << 20, 4096, false).await;
        let constrained = packages(2 << 20, 128, true).await;
        assert_eq!(ordinary.view_identity(), constrained.view_identity());
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
    async fn interrupted_workspace_drain_retains_provider_join_for_retry() {
        use cpg_extract::bundle::ProviderSink;
        let workspace=Workspace::new(model(),WorkspaceOptions::default(),crate::test_native::store()).unwrap();
        let output=workspace.output("provider-drain",Profile::Catalog,ContentHash::of(b"provider-drain"),workspace.inputs("provider-drain",Profile::Catalog,[]).unwrap(),[]);
        let entered=Arc::new(tokio::sync::Notify::new());let completed=Arc::new(AtomicBool::new(false));
        let thread_entered=entered.clone();let thread_completed=completed.clone();
        let thread=std::thread::spawn(move ||{thread_entered.notify_one();std::thread::sleep(std::time::Duration::from_millis(80));thread_completed.store(true,Ordering::Release);});
        let acknowledgement=output.drain_provider(thread);entered.notified().await;
        let first_owner=workspace.clone();let first=tokio::spawn(async move{first_owner.drain().await});
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;first.abort();assert!(first.await.is_err());
        workspace.drain().await.unwrap();assert!(completed.load(Ordering::Acquire));acknowledgement.await.unwrap().unwrap();
    }
    #[tokio::test]
    async fn manual_output_inventory_is_closed_and_empty_inventory_is_metadata_only() {
        let workspace=Workspace::new(model(),WorkspaceOptions::default(),crate::test_native::store()).unwrap();
        let output=workspace.output("metadata-only",Profile::Catalog,ContentHash::of(b"metadata-only"),workspace.inputs("metadata-only",Profile::Catalog,[]).unwrap(),[]);
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let contributions=workspace.native().contributions().await.unwrap();
        assert_eq!(contributions.len(),1);
        assert!(contributions[0].spec.outputs.is_empty());
        assert!(contributions[0].outputs.is_empty());
        assert!(workspace.completed::<Package>().is_err());
        let refused=workspace.output("undeclared",Profile::Catalog,ContentHash::of(b"undeclared"),workspace.inputs("undeclared",Profile::Catalog,[]).unwrap(),[]);
        assert!(refused.declare::<Package>().is_err());
        assert!(refused.finish(ProviderOutcome::Complete).await.is_err());
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_drains_an_idle_full_native_handoff() {
        let workspace=Workspace::new(model(),WorkspaceOptions{batch_rows:1,..Default::default()},crate::test_native::store()).unwrap();
        let output=workspace.output("idle-handoff",Profile::Catalog,ContentHash::of(b"idle-handoff"),workspace.inputs("idle-handoff",Profile::Catalog,[]).unwrap(), [<Package>::NAME]);
        output.declare_async::<Package>().await.unwrap();
        for name in ["one","two","three"] {output.push(Package{name:name.into()}).await.unwrap();}
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let relation=workspace.completed::<Package>().unwrap();
        let mut batches=relation.batches().unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        tokio::time::timeout(std::time::Duration::from_secs(5),workspace.drain()).await.unwrap().unwrap();
        assert!(batches.next().unwrap().is_err());
        assert!(batches.next().is_none());
    }
    #[tokio::test]
    async fn cancellation_and_incomplete_output_never_become_completed() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        output.declare::<Package>().unwrap();
        output
            .push(Package {
                name: "private".into(),
            })
            .await
            .unwrap();
        assert!(workspace.completed::<Package>().is_err());
        workspace.cancellation().cancel();
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn omitted_declared_outputs_and_ignored_row_refusals_never_complete() {
        use lctx_model::domain::stages::{Effect, RelationUse, Stage};
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
        let declaration = Stage {
            name: "packages",
            inputs: vec![],
            outputs: vec![RelationUse::of::<Package>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: Effect::Pure,
            code: ContentHash::of(b"fixture"),
            configuration: ContentHash::of(b"fixture"),
        };
        let output = workspace.producer(
            &declaration,
            Profile::Catalog,
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        );
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
        let budget = ResourceBudget::fixed(4096).unwrap();
        let constrained = Workspace::with_budget(
            model(),
            WorkspaceOptions {
                memory_bytes: 4096,
                ..Default::default()
            },
            budget, crate::test_native::store()
)
        .unwrap();
        let output = constrained.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            constrained
                .inputs("packages", Profile::Catalog, [])
                .unwrap(),
        [<Package>::NAME],
        );
        output.declare::<Package>().unwrap();
        assert!(
            output
                .push(Package {
                    name: "x".repeat(5000)
                })
                .await
                .is_err()
        );
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(constrained.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn conflicting_payload_refuses_the_whole_output_set() {
        use lctx_model::domain::{input::InputRevision, source::SourceArtifact};
        let workspace = Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions::default(), crate::test_native::store()
)
        .unwrap();
        let output = workspace.output(
            "source",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("source", Profile::Catalog, []).unwrap(),
        [<Package>::NAME, <SourceArtifact>::NAME],
        );
        output.declare::<Package>().unwrap();
        output.declare::<SourceArtifact>().unwrap();
        output
            .push(Package {
                name: "private".into(),
            })
            .await
            .unwrap();
        let first = SourceArtifact {
            input: InputRevision {
                manifest: ContentHash::of(b"manifest"),
            }
            .id(),
            path: "module.py".into(),
            content: ContentHash::of(b"x"),
            byte_len: 1,
        };
        let mut second = first.clone();
        second.byte_len = 2;
        output.push(first).await.unwrap();
        output.push(second).await.unwrap();
        assert!(matches!(
            output.finish(ProviderOutcome::Complete).await,
            Err(ModelError::Conflict(_))
        ));
        assert!(workspace.completed::<Package>().is_err());
        assert!(workspace.completed::<SourceArtifact>().is_err());
    }
    #[tokio::test]
    async fn ordinary_owner_adopts_native_contributions_then_refuses_another_owner() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
        let contributed = workspace.output(
            "native",
            Profile::Catalog,
            ContentHash::of(b"native"),
            workspace.inputs("native", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        let batch = Batch::new(
            workspace.model(),
            vec![Package {
                name: "package".into(),
            }],
            workspace.budget(),
        )
        .unwrap();
        contributed.contribute(&batch).unwrap();
        drop(batch);
        contributed.finish(ProviderOutcome::Complete).await.unwrap();
        let owner = workspace.output(
            "assembly",
            Profile::Catalog,
            ContentHash::of(b"assembly"),
            workspace.inputs("assembly", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        owner.declare::<Package>().unwrap();
        owner.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 1);
        let duplicate = workspace.output(
            "duplicate",
            Profile::Catalog,
            ContentHash::of(b"duplicate"),
            workspace.inputs("duplicate", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        duplicate.declare::<Package>().unwrap();
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        assert_eq!(
            workspace.completed::<Package>().unwrap().producer(),
            "assembly"
        );
    }
    #[tokio::test]
    async fn nominal_reference_closure_reuses_completed_streams_and_refuses_missing_targets() {
        use lctx_model::domain::input::Release;
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        for valid in [true, false] {
            let workspace = Workspace::new(model.clone(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
            let output = workspace.output(
                "release",
                Profile::Catalog,
                ContentHash::of(b"fixture"),
                workspace.inputs("release", Profile::Catalog, []).unwrap(),
            [<Package>::NAME, <Release>::NAME],
            );
            output.declare::<Package>().unwrap();
            output.declare::<Release>().unwrap();
            let package = Package {
                name: "package".into(),
            };
            let reference = if valid {
                package.id()
            } else {
                Package {
                    name: "absent".into(),
                }
                .id()
            };
            output.push(package).await.unwrap();
            output
                .push(Release {
                    package: reference,
                    version: "1.0".into(),
                })
                .await
                .unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let content = workspace.identity().unwrap();
            let admitted = workspace.validate().await;
            if valid {
                assert_eq!(admitted.unwrap(), content);
            } else {
                assert!(admitted.is_err());
            }
        }
    }
    #[tokio::test]
    async fn completed_input_keeps_native_membership_alive_and_later_contributions_do_not_change_it() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), crate::test_native::store()
).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        output.declare::<Package>().unwrap();
        output.push(Package { name: "one".into() }).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let original = workspace.completed::<Package>().unwrap();
        let bound = workspace
            .inputs("reader", Profile::Catalog, [Package::NAME])
            .unwrap();
        let next = workspace.output(
            "more",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("more", Profile::Catalog, []).unwrap(),
        [<Package>::NAME],
        );
        let batch = Batch::new(
            workspace.model(),
            vec![Package { name: "two".into() }],
            workspace.budget(),
        )
        .unwrap();
        next.contribute(&batch).unwrap();
        drop(batch);
        next.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(
            bound.relation::<Package>().unwrap().view_identity(),
            original.view_identity()
        );
        assert_eq!(original.rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        drop(bound);
        drop(workspace);
        assert_eq!(
            original
                .batches()
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .num_rows(),
            1
        );
        drop(original);
    }
}

#[derive(Debug)]
struct BudgetMemoryPool {
    budget: ResourceBudget,
    allocated: Mutex<Box<dyn Reservation>>,
}
impl std::fmt::Display for BudgetMemoryPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shared compiler allocation pool")
    }
}
impl MemoryPool for BudgetMemoryPool {
    fn name(&self) -> &str {
        "CompilerResourcePool"
    }
    fn grow(&self, reservation: &MemoryReservation, bytes: usize) {
        self.try_grow(reservation, bytes)
            .expect("infallible DataFusion allocation must already fit");
    }
    fn shrink(&self, _: &MemoryReservation, bytes: usize) {
        let mut allocated = self.allocated.lock().expect("memory pool poisoned");
        let size = allocated.size();
        allocated
            .try_resize(size - bytes)
            .expect("returning reservation");
    }
    fn try_grow(&self, _: &MemoryReservation, bytes: usize) -> datafusion::error::Result<()> {
        let mut allocated = self.allocated.lock().map_err(|_| {
            datafusion::error::DataFusionError::Internal("memory pool poisoned".into())
        })?;
        let size = allocated.size();
        allocated
            .try_resize(size.saturating_add(bytes))
            .map_err(|e| datafusion::error::DataFusionError::ResourcesExhausted(e.to_string()))
    }
    fn reserved(&self) -> usize {
        self.budget.reserved()
    }
    fn memory_limit(&self) -> datafusion::execution::memory_pool::MemoryLimit {
        datafusion::execution::memory_pool::MemoryLimit::Finite(self.budget.limit())
    }
}

#[cfg(test)]
mod empty_execution_admission_controls {
    use super::*;
    use lctx_model::domain::execution::source_call_records::SourceCallHeader;
    #[tokio::test]
    async fn complete_empty_execution_root_needs_no_unrequested_parent_streams() {
        let workspace = Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions::default(), crate::test_native::store()
)
        .unwrap();
        let inputs = workspace
            .inputs("empty-source-headers", Profile::Catalog, [])
            .unwrap();
        let output = workspace.output(
            "empty-source-headers",
            Profile::Catalog,
            ContentHash::of(b"empty-header-control"),
            inputs,
        [<SourceCallHeader>::NAME],
        );
        output.declare::<SourceCallHeader>().unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let selected = workspace
            .inputs(
                "empty-header-consumer",
                Profile::Catalog,
                [SourceCallHeader::NAME],
            )
            .unwrap();
        workspace.checked_inputs(&selected).await.unwrap();
        assert_eq!(workspace.completed::<SourceCallHeader>().unwrap().rows(), 0);
    }
}
