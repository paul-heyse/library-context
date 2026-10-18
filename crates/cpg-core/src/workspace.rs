//! Private persisted compilation workspace with exact immutable contribution views.
//! DataFusion scratch is spillable and derived; native memberships own completed inputs.
use arrow_array::RecordBatch;
use datafusion::{
    execution::{
        disk_manager::{DiskManagerBuilder, DiskManagerMode},
        memory_pool::{MemoryPool, MemoryReservation},
        runtime_env::RuntimeEnvBuilder,
    },
    prelude::{SessionConfig, SessionContext},
};
use futures::{
    FutureExt, TryStreamExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::{
    Batch, ContentHash, HeapSize, ModelError, Record, Relation, ValidatedModel, analysis,
    batching::TransferLimits,
    charged::StateCharge,
    resources::{Reservation, ResourceBudget},
    stages::{Profile, ProviderOutcome},
    uniform_outcome,
};
use lctx_tracing::InContext;
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

type OwnedTaskJoin = Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
type ChargedBatch = Result<(RecordBatch, Box<dyn Reservation>), ModelError>;

mod cache_calls;
mod stage_execution;
pub(crate) use lctx_compiler_store::private_source;
mod native_materialization;
mod private_execution;
use stage_execution::StageExecution;
mod membership_demand;
use membership_demand::MembershipDemand;
mod input_catalog;
use input_catalog::{InputAliases, InputPort, InputSchema};

pub use lctx_compiler_store::admission::{AdmissionOptions as WorkspaceOptions, Cancellation};
struct WorkspaceFiles {
    _directory: Arc<tempfile::TempDir>,
}
/// Execution ownership survives a compiler catalog handoff without retaining that catalog.
type RetentionJoin = futures::future::Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
struct WorkspaceExecutionOwner {
    _files: Arc<WorkspaceFiles>,
    cancellation: Cancellation,
    bridge: Option<Arc<crate::native_bridge::NativeBridge>>,
    native_calls: Option<Arc<crate::native_calls::NativeCalls>>,
    native: Option<Arc<lctx_compiler_store::compiler::NativeCompilerStore>>,
    stages: StageExecution,
    retention: Mutex<Vec<(RetentionJoin, Box<dyn Reservation>)>>,
    preparations: Mutex<Vec<(RetentionJoin, Box<dyn Reservation>)>>,
    provider_drains: Arc<Mutex<Vec<OwnedTaskJoin>>>,
    product_cache: Arc<Mutex<Option<Arc<lctx_compiler_store::NativeProductCache>>>>,
    membership_demand: Arc<Mutex<Option<MembershipDemand>>>,
    signal: Option<Arc<lctx_native::signal::SignalRegistration>>,
    admission_memo: Arc<lctx_compiler_store::admission::AdmissionMemo>,
}
struct FrozenFactsAvailability {
    profile: Profile,
    providers: BTreeSet<&'static str>,
    availability: Arc<lctx_model::domain::admission::ScopedAvailability>,
    _charge: StateCharge,
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
struct CompletedOwners {
    by_spec: BTreeMap<ContentHash, lctx_model::domain::completed::CompletedContribution>,
    charge: StateCharge,
    // Completed views name full contributions; duplicate handoffs name producer specs.
    spec_by_completed: BTreeMap<ContentHash, ContentHash>,
}
/// The attempt owns one runtime, spill directory, buffer budget, and completed relation registry.
pub struct Workspace {
    files: Arc<WorkspaceFiles>,
    execution: Arc<WorkspaceExecutionOwner>,
    context: SessionContext,
    options: WorkspaceOptions,
    budget: ResourceBudget,
    scope_programs: Arc<lctx_model::domain::scope_program::ScopeProgramRuntime>,
    product_cache: Arc<Mutex<Option<Arc<lctx_compiler_store::NativeProductCache>>>>,
    membership_demand: Arc<Mutex<Option<MembershipDemand>>>,
    admitted_product_reuse: AtomicBool,
    model: Arc<ValidatedModel>,
    native: Option<Arc<lctx_compiler_store::compiler::NativeCompilerStore>>,
    bridge: Option<Arc<crate::native_bridge::NativeBridge>>,
    native_calls: Option<Arc<crate::native_calls::NativeCalls>>,
    content_frozen: AtomicBool,
    completed_owners: Mutex<CompletedOwners>,
    completed: Mutex<BTreeMap<&'static str, Arc<CompletedRelation>>>,
    completed_by_producer: Mutex<BTreeMap<(&'static str, &'static str), Arc<CompletedRelation>>>,
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
    #[cfg(test)]
    composition_control: composition_controls::Control,
    compilation_complete: Mutex<Option<CompilationCompletion>>,
    provider_drains: Arc<Mutex<Vec<OwnedTaskJoin>>>,
    admission_memo: Arc<lctx_compiler_store::admission::AdmissionMemo>,
    frozen_facts_availability: Mutex<Option<FrozenFactsAvailability>>,
    operation: ContentHash,
    private_validation: Arc<lctx_compiler_store::private_validation::PrivateValidation>,
    private_spill: Option<Arc<private_source::PrivateSpill>>,
    private_products: Mutex<
        Vec<(
            lctx_model::domain::compilation_product::ProductRequest,
            lctx_model::domain::completed::CompletedContribution,
            StateCharge,
        )>,
    >,
}
impl WorkspaceExecutionOwner {
    async fn join_retention(&self) -> Result<(), ModelError> {
        let retained = self
            .retention
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .map(|(join, _)| join.clone())
            .collect::<Vec<_>>();
        let mut terminal = lctx_model::domain::completion::Completion::default();
        for retained in retained {
            terminal.step(
                "accepted product retention",
                retained.await.map_err(ModelError::SharedCause),
            );
        }
        lctx_model::domain::completion::complete(Ok(()), terminal)
    }
}
impl lctx_compiler_store::admission::ExecutionOwner for WorkspaceExecutionOwner {
    fn drain_owned_report(&self) -> BoxFuture<'_, lctx_model::domain::completion::Completion> {
        async move {
            use lctx_model::domain::completion::{Completion, LocalState};
            let mut completion = Completion::default();
            // Required consumers may already have used the completed source. Accepted
            // optional retention remains an observed child, never a detached cache miss.
            let retained = match self.retention.lock() {
                Ok(retained) => retained.iter().map(|(join, _)| join.clone()).collect::<Vec<_>>(),
                Err(_) => { completion.step("retention join ownership", Err(poisoned())); Vec::new() }
            };
            let retained_count = retained.len();
            for retained in retained {
                completion.step("accepted product retention", retained.await.map_err(ModelError::SharedCause));
            }
            self.cancellation.cancel();
            // Stages can register provider descendants. Seal/join them before snapshotting providers.
            match self.stages.seal() {
                Ok(stages) => for stage in stages {
                    completion.step("producer task join", stage.await.map_err(ModelError::SharedCause));
                },
                Err(error) => {
                    completion.local = LocalState::Outstanding;
                    completion.step("producer task ownership", Err(error));
                    // Unauthenticated stage ownership cannot establish that descendant
                    // registration stopped. Retain provider/native owners for retry.
                    return completion;
                }
            }
            // A producer present in the stage snapshot may have registered a child after
            // our first snapshot. Actual stage drainage closes that registration window.
            let late_retention = match self.retention.lock() {
                Ok(retained) => retained.iter().skip(retained_count).map(|(join, _)| join.clone()).collect::<Vec<_>>(),
                Err(_) => { completion.step("late retention join ownership", Err(poisoned())); Vec::new() }
            };
            for retained in late_retention {
                completion.step("accepted late product retention", retained.await.map_err(ModelError::SharedCause));
            }
            // A stage's actual join alone does not retain its body error when result delivery
            // was abandoned. Observe every accepted preparation result after registration closes.
            let preparations = match self.preparations.lock() {
                Ok(preparations) => preparations.iter().map(|(join, _)| join.clone()).collect::<Vec<_>>(),
                Err(_) => { completion.local = LocalState::Outstanding; completion.step("preparation ownership", Err(poisoned())); Vec::new() }
            };
            for preparation in preparations {
                completion.step("accepted preparation result", preparation.await.map_err(ModelError::SharedCause));
            }
            let drains = match self.provider_drains.lock() {
                Ok(drains) => drains.clone(),
                Err(_) => {
                    completion.local = LocalState::Outstanding;
                    completion.step("provider drain ownership", Err(poisoned()));
                    Vec::new()
                }
            };
            for task in drains {
                completion.step("provider join", task.await.map_err(ModelError::SharedCause));
            }
            if let Some(bridge) = &self.bridge { completion.merge(bridge.drain_finality_report().await); }
            if let Some(calls) = &self.native_calls { completion.merge(calls.drain_finality_report().await); }
            completion.step(
                "private scoped check release",
                self.admission_memo.close_scoped(),
            );
            if let Some(native) = &self.native { completion.merge(native.drain_finality_report().await); }
            let cache = match self.product_cache.lock() {
                Ok(mut cache) => cache.take(),
                Err(_) => {
                    completion.local = LocalState::Outstanding;
                    completion.step("product cache ownership", Err(poisoned()));
                    None
                }
            };
            if let Some(cache) = cache {
                if completion.local == LocalState::Terminal
                    && completion.remote == lctx_model::domain::completion::RemoteState::Confirmed
                    && completion
                        .failures
                        .iter()
                        .all(|failure| failure.error.permits_storage_cleanup())
                {
                    completion.step("product cache session close", cache.close().await);
                } else {
                    cache.retain_unknown();
                    completion.local = LocalState::Outstanding;
                }
            }
            if completion.local == LocalState::Terminal
                && completion.remote == lctx_model::domain::completion::RemoteState::Confirmed
                && completion.failures.iter().all(|failure| failure.error.permits_storage_cleanup())
            {
                match self.membership_demand.lock() {
                    Ok(mut demand) => { demand.take(); }
                    Err(_) => completion.step("membership demand owner release", Err(poisoned())),
                }
            }
            completion
        }
        .with_tracing_span(tracing::info_span!(target: "lctx_phase", "compiler_drain", native=self.native.is_some()))
        .boxed()
    }
    fn complete_signal<'a>(
        &'a self,
        completion: &'a lctx_model::domain::completion::Completion,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        async move {
            match &self.signal {
                Some(signal) => signal.complete(completion).await,
                None => Ok(()),
            }
        }
        .boxed()
    }
    fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        if let Some(bridge) = &self.bridge {
            bridge.append_operation_failures(completion);
        }
        if let Some(calls) = &self.native_calls {
            calls.append_operation_failures(completion);
        }
        if let Some(native) = &self.native {
            native.append_operation_failures(completion);
        }
    }
}
pub(crate) type PreparationJoin<T> = Shared<BoxFuture<'static, Result<Arc<T>, Arc<ModelError>>>>;
impl Workspace {
    pub(crate) fn spawn_preparation<T: Send + Sync + 'static>(
        &self,
        work: impl std::future::Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<PreparationJoin<T>, ModelError> {
        // Reserve and hold the destination before accepting work. After spawn succeeds,
        // publishing its terminal observer has no remaining fallible allocation step.
        let mut observers = self.execution.preparations.lock().map_err(|_| poisoned())?;
        let charge = self.budget.reserve(
            "accepted preparation observer",
            size_of::<(RetentionJoin, Box<dyn Reservation>)>() + 256,
        )?;
        observers.try_reserve(1).map_err(ModelError::codec)?;
        let result = self
            .spawn_stage(work)?
            .map(|result| result.map(Arc::new).map_err(Arc::new))
            .boxed()
            .shared();
        let observer = result
            .clone()
            .map(|result| result.map(drop))
            .boxed()
            .shared();
        observers.push((observer.clone(), charge));
        // A normally consumed result also completes its terminal observer now. Otherwise
        // the unpolled observer would keep the full successful value until final drain,
        // even after its preparation registry and last useful borrower released it.
        Ok(async move {
            let value = result.await;
            let _terminal = observer.await;
            value
        }
        .boxed()
        .shared())
    }
    fn retain_child(
        &self,
        work: impl std::future::Future<Output = Result<(), ModelError>> + Send + 'static,
    ) -> Result<(), ModelError> {
        let slot = self.budget.reserve(
            "accepted retention child join",
            size_of::<(RetentionJoin, Box<dyn Reservation>)>() + 128,
        )?;
        let task = self.spawn_stage(work)?;
        let joined = task.map(|result| result.map_err(Arc::new)).boxed().shared();
        self.execution
            .retention
            .lock()
            .map_err(|_| poisoned())?
            .push((joined, slot));
        Ok(())
    }
    /// Schedule owned producer work; only its terminal result is returned to the coordinator.
    pub(crate) fn spawn_stage<T: Send + 'static>(
        &self,
        work: impl std::future::Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<BoxFuture<'static, Result<T, ModelError>>, ModelError> {
        self.execution.stages.spawn(self.cancellation.clone(), work)
    }
    pub fn new(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    ) -> Result<Arc<Self>, ModelError> {
        let budget = match native.recovery_budget()? {
            Some(budget) => budget,
            None => ResourceBudget::fixed(options.memory_bytes)?,
        };
        Self::construct(model, options, budget, Some(native), None)
    }
    /// Reuse the actual native session's pool. A supplied foreign pool is refused before
    /// constructing runtime services or admitting any workspace-owned state.
    pub fn with_budget(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
        native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::construct(model, options, budget, Some(native), None)
    }
    fn construct(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
        native: Option<Arc<lctx_compiler_store::compiler::NativeCompilerStore>>,
        programs: Option<Arc<lctx_model::domain::scope_program::ScopeProgramRuntime>>,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 || options.batch_rows == 0 {
            return Err(ModelError::Invalid(
                "workspace sizes must be positive".into(),
            ));
        }
        if options.memory_bytes != budget.limit() {
            return Err(ModelError::Invalid(
                "workspace memory differs from supplied pool".into(),
            ));
        }
        if let Some(native) = &native
            && let Some(attached) = native.recovery_budget()?
            && !attached.shares_pool(&budget)
        {
            return Err(ModelError::Conflict(
                "workspace native resource pool correspondence",
            ));
        }
        let private_spill = if native.is_none() {
            Some(private_source::PrivateSpill::new(budget.clone())?)
        } else {
            None
        };
        let budget = match &private_spill {
            Some(spill) => budget.with_reclaimer(spill.clone()),
            None => budget,
        };
        // Explicitly unbound mechanical sources may construct a compute-only workspace.
        // This does not create native receipt/session authority; native preparation still
        // requires the source's existing recovery-scope admission route.
        let directory = Arc::new(tempfile::tempdir().map_err(ModelError::codec)?);
        let pool = Arc::new(BudgetMemoryPool {
            budget: budget.clone(),
            allocated: Mutex::new(budget.reserve("workspace-datafusion", 0)?),
        });
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_pool(pool)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(
                DiskManagerMode::Directories(vec![directory.path().to_path_buf()]),
            ))
            .build_arc()
            .map_err(ModelError::codec)?;
        let config = SessionConfig::new()
            .with_target_partitions(options.partitions)
            // Native transfer windows must not change DataFusion's compute vector size or
            // its statistics-based threshold for beneficial physical repartitioning.
            .set_bool(
                "datafusion.execution.use_row_number_estimates_to_optimize_partitioning",
                true,
            )
            .set_usize(
                "datafusion.execution.sort_spill_reservation_bytes",
                (options.memory_bytes / 16).min(10 << 20),
            )
            .set_usize(
                "datafusion.execution.sort_in_place_threshold_bytes",
                (options.memory_bytes / 8).min(1 << 20),
            );
        let admission_memo = lctx_compiler_store::admission::AdmissionMemo::new(&budget);
        let completed_owners = Mutex::new(CompletedOwners {
            by_spec: BTreeMap::new(),
            charge: StateCharge::new(&budget, "compiler-completed-owners"),
            spec_by_completed: BTreeMap::new(),
        });
        let cancellation = Cancellation::default();
        let token = cancellation.clone();
        let signal = Some(Arc::new(lctx_native::signal::SignalRegistration::new(
            "compiler workspace",
            move || token.cancel(),
        )?));
        let bridge = if native.is_some() {
            Some(Arc::new(crate::native_bridge::NativeBridge::new(
                cancellation.clone(),
            )?))
        } else {
            None
        };
        let native_calls = native.as_ref().map(|native| {
            Arc::new(crate::native_calls::NativeCalls::new(
                native.clone(),
                cancellation.clone(),
                &budget,
            ))
        });
        let files = Arc::new(WorkspaceFiles {
            _directory: directory,
        });
        let provider_drains = Arc::new(Mutex::default());
        let product_cache = Arc::new(Mutex::default());
        let membership_demand = Arc::new(Mutex::default());
        let execution = Arc::new(WorkspaceExecutionOwner {
            _files: files.clone(),
            cancellation: cancellation.clone(),
            bridge: bridge.clone(),
            native_calls: native_calls.clone(),
            native: native.clone(),
            stages: StageExecution::new(&budget),
            retention: Mutex::new(Vec::new()),
            preparations: Mutex::new(Vec::new()),
            provider_drains: provider_drains.clone(),
            product_cache: product_cache.clone(),
            membership_demand: membership_demand.clone(),
            signal: signal.clone(),
            admission_memo: admission_memo.clone(),
        });
        Ok(Arc::new(Self {
            files,
            execution,
            context: SessionContext::new_with_config_rt(config, runtime.clone()),
            options,
            scope_programs: match programs {
                Some(programs) => programs,
                None => {
                    lctx_model::domain::scope_program::ScopeProgramRuntime::new(&model, &budget)?
                }
            },
            product_cache,
            membership_demand,
            admitted_product_reuse: AtomicBool::new(true),
            budget: budget.clone(),
            model: model.clone(),
            native,
            bridge,
            native_calls,
            content_frozen: AtomicBool::new(false),
            completed_owners,
            completed: Mutex::default(),
            completed_by_producer: Mutex::default(),
            frozen_shared: Mutex::default(),
            cancellation: cancellation.clone(),
            completion_gate: tokio::sync::Mutex::new(()),
            #[cfg(test)]
            composition_control: composition_controls::Control::default(),
            compilation_complete: Mutex::default(),
            provider_drains,
            admission_memo,
            frozen_facts_availability: Mutex::default(),
            operation: lctx_native::control::fresh_identity("private-compiler-operation")?,
            private_products: Mutex::new(Vec::new()),
            private_spill,
            private_validation: Arc::new(
                lctx_compiler_store::private_validation::PrivateValidation::new(
                    model.clone(),
                    options,
                    budget.clone(),
                    cancellation.clone(),
                    runtime,
                )?,
            ),
        }))
    }
    /// Repeated attempts can reuse one model/runtime program owner while each completed source,
    /// native contribution, cancellation token and semantic capability remains attempt-local.
    pub fn with_program_runtime(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
        native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
        programs: Arc<lctx_model::domain::scope_program::ScopeProgramRuntime>,
    ) -> Result<Arc<Self>, ModelError> {
        programs.require_model(&model)?;
        Self::construct(model, options, budget, Some(native), Some(programs))
    }
    /// Restore the exact current and frozen bindings after independent native state import.
    pub async fn restore(&self, profile: Profile) -> Result<(), ModelError> {
        self.writable()?;
        if !self.completed.lock().map_err(|_| poisoned())?.is_empty()
            || !self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .is_empty()
        {
            return Err(ModelError::Conflict(
                "native restoration requires an empty workspace",
            ));
        }
        let runtime = self.admission_engine()?;
        runtime.restore(profile).await?;
        let lctx_compiler_store::admission::CompletedInventory {
            contributions,
            bindings,
            transfer: _transfer,
        } = runtime.completed_inventory()?;
        let mut owners = BTreeMap::new();
        let mut owner_identities = BTreeMap::new();
        let mut owner_charge = StateCharge::new(&self.budget, "compiler-completed-owners");
        for contribution in contributions {
            owner_charge.grow(
                std::mem::size_of_val(&contribution)
                    .saturating_add(lctx_native::installed_view::contribution_heap(
                        &contribution,
                    ))
                    .saturating_add(256),
            )?;
            let spec = contribution.spec.identity()?;
            if owner_identities
                .insert(contribution.identity()?, spec)
                .is_some()
            {
                return Err(ModelError::Conflict(
                    "duplicate restored completed contribution",
                ));
            }
            if owners.insert(spec, contribution).is_some() {
                return Err(ModelError::Conflict(
                    "duplicate restored completed producer spec",
                ));
            }
        }
        let mut completed = BTreeMap::new();
        let mut frozen = BTreeMap::new();
        for binding in bindings {
            let relation = self
                .model
                .relation(binding.source.relation())
                .ok_or(ModelError::Schema("restored compiler relation"))?
                .clone();
            let name = relation.name();
            let retained = self.budget.reserve(
                "restored completed relation backing",
                std::mem::size_of::<CompletedRelation>()
                    + 128
                    + binding.source.producer().len()
                    + binding.source.heap_bytes()
                    + lctx_native::installed_view::view_heap(&binding.view),
            )?;
            let source = Arc::new(CompletedRelation {
                private: None,
                relation,
                producer: binding.source.producer().into(),
                implementation: binding.source.implementation(),
                configuration: binding.configuration,
                contract: binding.source.model(),
                rows: binding.view.rows,
                view: binding.view,
                native: self.native.clone(),
                bridge: self.bridge.clone(),
                budget: self.budget.clone(),
                batch_rows: self.options.batch_rows,
                cancellation: self.cancellation.clone(),
                inputs: Arc::from([]),
                profile,
                contribution: lctx_model::domain::stages::is_epoch_shared(name),
                snapshot: binding.source,
                _files: self.files.clone(),
                _selection_charge: Some(Arc::from(retained)),
                _execution: self.execution.clone(),
            });
            if let Some(boundary) = binding.boundary {
                let boundary = lctx_model::domain::stages::PublicationBoundary::ALL
                    .into_iter()
                    .find(|b| b.name() == boundary)
                    .ok_or(ModelError::Schema("restored compiler boundary"))?;
                frozen.insert((boundary, name), source);
            } else {
                completed.insert(name, source);
            }
        }
        *self.completed_owners.lock().map_err(|_| poisoned())? = CompletedOwners {
            by_spec: owners,
            charge: owner_charge,
            spec_by_completed: owner_identities,
        };
        *self.completed.lock().map_err(|_| poisoned())? = completed;
        *self.frozen_shared.lock().map_err(|_| poisoned())? = frozen;
        Ok(())
    }
    fn writable(&self) -> Result<(), ModelError> {
        if self.content_frozen.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("compiler content is frozen"));
        }
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
        let _gate =
            lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock()).await;
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
    /// Fence the complete native completion/binding/visibility handoff before admission.
    /// The permanent mutation fence also applies to restored workspaces, which do not
    /// invent a normal compilation attestation.
    pub(crate) async fn freeze_for_admission(
        &self,
        request: Option<(
            &cpg_extract::bundle::CapturedInputs,
            lctx_model::domain::admission::Frontier,
            Profile,
            ContentHash,
        )>,
    ) -> Result<(), ModelError> {
        // Final artifact freeze is the enclosing boundary, not producer readiness.
        // All optional children accepted by completed stages settle before native
        // admission freezes mutation/retained-product inventory.
        self.execution.join_retention().await?;
        let _gate =
            lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock()).await;
        self.cancellation.check()?;
        if let Some((captures, frontier, profile, settings)) = request {
            self.require_compilation(captures, frontier, profile, settings)?;
        }
        self.content_frozen.store(true, Ordering::Release);
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
    pub fn facts_availability(
        &self,
        profile: Profile,
    ) -> Result<Arc<lctx_model::domain::admission::ScopedAvailability>, ModelError> {
        if let Some(frozen) = &*self
            .frozen_facts_availability
            .lock()
            .map_err(|_| poisoned())?
        {
            return if frozen.profile == profile {
                Ok(frozen.availability.clone())
            } else {
                Err(ModelError::Conflict("frozen facts profile"))
            };
        }
        let runtime = self.admission_runtime(profile)?;
        self.bridge()?
            .call(async move { runtime.facts_availability_async(profile).await })
    }
    pub async fn facts_availability_async(
        &self,
        profile: Profile,
    ) -> Result<Arc<lctx_model::domain::admission::ScopedAvailability>, ModelError> {
        if let Some(frozen) = &*self
            .frozen_facts_availability
            .lock()
            .map_err(|_| poisoned())?
        {
            return if frozen.profile == profile {
                Ok(frozen.availability.clone())
            } else {
                Err(ModelError::Conflict("frozen facts profile"))
            };
        }
        self.admission_runtime(profile)?
            .facts_availability_async(profile)
            .await
    }
    /// Expected-domain consumers share the exact selected compact producer owners.
    /// Their own captured sources still reconstruct and validate scope membership.
    pub async fn coverage_admission<'a>(
        &self,
        access: &CompletedInputs,
        sources: &'a analysis::sources::CapturedSources,
    ) -> Result<analysis::expected::CoverageAdmission<'a>, ModelError> {
        let profile = sources.profile().ok_or(ModelError::Conflict(
            "coverage admission captured profile absent",
        ))?;
        if access.profile() != profile {
            return Err(ModelError::Conflict("coverage selected profile"));
        }
        let owners = self.uniform_owners(access).await?;
        let mut admission = analysis::expected::CoverageAdmission::new(sources, self.budget())?;
        if let Some(source) = access.relations.get(&(
            uniform_outcome::UniformNotRequested::NAME,
            Some(lctx_model::domain::stages::PublicationBoundary::Facts),
        )) {
            let permit = CompletedInputs::read_source::<uniform_outcome::UniformNotRequested>(
                &source.source,
            )?;
            if sources.iter().any(|captured| captured == permit.source()) {
                admission.select_facts_uniform_source(&permit)?;
            }
        }
        for owner in owners.owners() {
            admission.admit_uniform_owner(owner)?;
            if owner.analysis_owner().is_some() {
                // A shared admission may know more owners than this consumer captured. Reuse
                // metadata only from a declared, captured role that actually selected its owner.
                for source in access.relations.values().filter(|source| {
                    source.name() == uniform_outcome::UniformNotRequested::NAME
                        && source.view.contributions.contains(&owner.contribution())
                }) {
                    let permit =
                        access.read_at::<uniform_outcome::UniformNotRequested>(source.requested)?;
                    if sources.iter().any(|captured| captured == permit.source()) {
                        admission.admit_uniform_analysis(owner, &permit, &source.view)?;
                    }
                }
            }
        }
        Ok(admission)
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
        let private = if self.is_private() {
            Some(self.private_checked(inputs).await?)
        } else {
            self.validate_scope(inputs.profile, true, Some(inputs))
                .await?;
            None
        };
        Ok(CheckedInputs {
            _private: private,
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
        self.bind_admission_epochs(&mut inputs)?;
        let private = if self.is_private() {
            Some(self.private_checked(&inputs).await?)
        } else {
            self.validate_scope(profile, true, Some(&inputs)).await?;
            None
        };
        Ok(CheckedInputs {
            _private: private,
            inputs,
            attempt: self.files.clone(),
            policy: self.model.digest(),
        })
    }
    fn bind_admission_epochs(&self, inputs: &mut CompletedInputs) -> Result<(), ModelError> {
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        if frozen.is_empty() {
            // Detached artifacts contain canonical streams rather than execution epochs.
            // Resolve each requested validation role explicitly to that canonical source;
            // never describe it as a frozen epoch or reuse it as an ordinary producer view.
            for declaration in self
                .model
                .invariants()
                .iter()
                .flat_map(|invariant| &invariant.inputs)
            {
                if declaration.prefix().is_none()
                    || inputs
                        .relations
                        .contains_key(&(declaration.name(), declaration.prefix()))
                {
                    continue;
                }
                if let Some(source) = inputs
                    .relations
                    .get(&(declaration.name(), None))
                    .map(|binding| binding.source.clone())
                {
                    let role = inputs.relations.len();
                    inputs.relations.insert(
                        (declaration.name(), declaration.prefix()),
                        ResolvedInput {
                            role,
                            requested: declaration.prefix(),
                            resolved: None,
                            source,
                        },
                    );
                }
            }
        } else {
            for ((boundary, name), source) in frozen.iter() {
                let role = inputs.relations.len();
                inputs.relations.insert(
                    (*name, Some(*boundary)),
                    ResolvedInput {
                        role,
                        requested: Some(*boundary),
                        resolved: Some(*boundary),
                        source: source.clone(),
                    },
                );
            }
        }
        Ok(())
    }
    async fn validate_scope(
        &self,
        profile: Profile,
        admission: bool,
        selected: Option<&CompletedInputs>,
    ) -> Result<ContentHash, ModelError> {
        let owned;
        let inputs = if let Some(inputs) = selected {
            inputs
        } else {
            let relations = self.completed_relations()?;
            owned = {
                let mut inputs = self.inputs(
                    "artifact-admission",
                    profile,
                    relations.iter().map(|r| r.name()),
                )?;
                self.bind_admission_epochs(&mut inputs)?;
                inputs
            };
            &owned
        };
        if self.is_private() {
            return Ok(self.private_checked(inputs).await?.identity());
        }
        let session = inputs.session(self).await?;
        let bytes = inputs
            .relations
            .values()
            .map(|binding| {
                serde_json::to_vec(&binding.source.view)
                    .map(|v| v.len().saturating_mul(8).saturating_add(1024))
                    .map_err(ModelError::codec)
            })
            .try_fold(0usize, |total, bytes| {
                bytes.map(|bytes| total.saturating_add(bytes))
            })?;
        let _selection = self
            .budget
            .reserve("compiler-admission-selected-metadata", bytes)?;
        let bindings = inputs
            .ordered_bindings()
            .into_iter()
            .map(
                |(_, bound)| lctx_compiler_store::admission::SelectedBinding {
                    role: bound.role,
                    requested: bound.requested,
                    resolved: bound.resolved,
                    binding: lctx_model::domain::completed::CompletedBinding {
                        boundary: bound.resolved.map(|b| b.name().into()),
                        source: bound.snapshot(),
                        view: bound.view.clone(),
                        configuration: None,
                    },
                },
            )
            .collect();
        self.selected_admission_runtime(inputs)?
            .validate_compiler_selection(profile, admission, bindings, &session, inputs)
            .await
    }
    pub async fn admit_frontier(
        &self,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
    ) -> Result<(), ModelError> {
        self.admission_runtime(profile)?
            .admit_frontier(frontier, profile)
            .await
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
            session
                .register_table(
                    &table,
                    self.native()?
                        .table_provider(
                            &source.view,
                            source.relation.clone(),
                            self.budget.clone(),
                            self.options.batch_rows,
                        )
                        .await?,
                )
                .map_err(ModelError::codec)?;
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
    fn admission_engine(
        &self,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        self.admission_engine_for(false)
    }
    fn admission_engine_for(
        &self,
        selected: bool,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        let construct = if selected {
            lctx_compiler_store::admission::AdmissionRuntime::borrow_selected_runtime
        } else {
            lctx_compiler_store::admission::AdmissionRuntime::borrow_runtime
        };
        construct(
            self.model.clone(),
            self.options,
            self.budget.clone(),
            self.native()?.clone(),
            self.cancellation.clone(),
            self.native_calls()?.clone(),
            self.admission_memo.clone(),
            self.context.copied_config(),
            self.context.runtime_env(),
            self.execution.clone(),
        )
    }
    /// Private assurance metadata follows exact captured roots and the authentic input
    /// bindings of their actual producer owners, independently of later completions.
    fn selected_admission_runtime(
        &self,
        inputs: &CompletedInputs,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        if !Arc::ptr_eq(&inputs.attempt, &self.files)
            || inputs.model != self.model.digest()
            || !inputs.budget.shares_pool(self.budget())
        {
            return Err(ModelError::Conflict("selected assurance workspace"));
        }
        let runtime = self.admission_engine_for(true)?;
        let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
        let current = self.completed.lock().map_err(|_| poisoned())?;
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let produced = self.completed_by_producer.lock().map_err(|_| poisoned())?;
        let mut charge = StateCharge::new(self.budget(), "selected assurance metadata transfer");
        let mut roots = Vec::new();
        let mut dependencies = Vec::new();
        let mut selected = BTreeMap::new();
        let mut pending = Vec::new();
        for (_, binding) in inputs.ordered_bindings() {
            let root = lctx_model::domain::completed::CompletedBinding {
                boundary: binding.resolved.map(|boundary| boundary.name().into()),
                source: binding.snapshot(),
                view: binding.source.view.clone(),
                configuration: None,
            };
            if !roots.contains(&root) {
                charge.grow(
                    serde_json::to_vec(&root)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(8)
                        .saturating_add(1024),
                )?;
                pending.extend(root.view.contributions.iter().copied());
                roots.push(root);
            }
        }
        while let Some(id) = pending.pop() {
            if selected.contains_key(&id) {
                continue;
            }
            let spec = owners
                .spec_by_completed
                .get(&id)
                .ok_or(ModelError::Conflict(
                    "selected assurance authentic owner absent",
                ))?;
            let owner = owners.by_spec.get(spec).ok_or(ModelError::Conflict(
                "selected assurance authentic owner correspondence absent",
            ))?;
            charge.grow(
                serde_json::to_vec(owner)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(4)
                    .saturating_add(256),
            )?;
            selected.insert(id, owner.clone());
            for input in &owner.spec.inputs {
                let mut actual = frozen
                    .iter()
                    .filter(|(_, source)| source.snapshot() == *input)
                    .map(|((boundary, _), source)| (Some(boundary.name().to_owned()), source))
                    .collect::<Vec<_>>();
                if !lctx_model::domain::stages::is_epoch_shared(input.relation())
                    || actual.is_empty()
                {
                    let source = inputs
                        .relations
                        .values()
                        .map(|binding| &binding.source)
                        .chain(current.values())
                        .chain(produced.values())
                        .find(|source| source.snapshot() == *input)
                        .ok_or(ModelError::Conflict(
                            "selected assurance producer input binding absent",
                        ))?;
                    actual.push((None, source));
                }
                for (boundary, source) in actual {
                    let binding = lctx_model::domain::completed::CompletedBinding {
                        boundary,
                        source: input.clone(),
                        view: source.view.clone(),
                        configuration: None,
                    };
                    if roots.contains(&binding) || dependencies.contains(&binding) {
                        continue;
                    }
                    charge.grow(
                        serde_json::to_vec(&binding)
                            .map_err(ModelError::codec)?
                            .len()
                            .saturating_mul(8)
                            .saturating_add(1024),
                    )?;
                    pending.extend(binding.view.contributions.iter().copied());
                    dependencies.push(binding);
                }
            }
        }
        runtime.bind_selection_snapshot(
            inputs.profile,
            selected.into_values().collect(),
            roots,
            dependencies,
        )?;
        Ok(runtime)
    }
    pub async fn uniform_owners(
        &self,
        access: &CompletedInputs,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmittedUniformOwners>, ModelError> {
        if self.is_private() {
            return self
                .private_checked(access)
                .await?
                .admitted_uniform_owners(&self.budget);
        }
        self.selected_admission_runtime(access)?
            .uniform_owners_async(access.profile())
            .await
    }
    /// Freeze exactly the provider-owned facts inventory before upper contributions can
    /// enlarge shared metadata. The provider list is supplied by the actual facts owner.
    pub async fn freeze_facts_availability(
        &self,
        profile: Profile,
        provider_names: &BTreeSet<&'static str>,
    ) -> Result<(), ModelError> {
        use lctx_model::domain::{
            attribution::{AnalysisContext, Provider, ProviderCoverage, ProviderRun, RunFamily},
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
            uniform_outcome::UniformNotRequested,
        };
        let _completion = if self.is_private() {
            None
        } else {
            Some(
                lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock())
                    .await,
            )
        };
        if let Some(frozen) = &*self
            .frozen_facts_availability
            .lock()
            .map_err(|_| poisoned())?
        {
            return if frozen.profile == profile && frozen.providers == *provider_names {
                Ok(())
            } else {
                Err(ModelError::Conflict("frozen facts provider selection"))
            };
        }
        let mut relations = BTreeMap::new();
        for name in [
            InputRevision::NAME,
            SourceArtifact::NAME,
            ArtifactUse::NAME,
            CoverageScope::NAME,
            ProviderCoverage::NAME,
            Provider::NAME,
            ProviderRun::NAME,
            RunFamily::NAME,
            AnalysisContext::NAME,
            UniformNotRequested::NAME,
        ] {
            let sources = self
                .completed_by_producer
                .lock()
                .map_err(|_| poisoned())?
                .iter()
                .filter(|((producer, relation), _)| {
                    *relation == name && provider_names.contains(producer)
                })
                .map(|(_, source)| source.clone())
                .collect::<Vec<_>>();
            let template = sources.first().ok_or(ModelError::Conflict(
                "provider facts availability relation absent",
            ))?;
            if sources.iter().any(|source| source.profile != profile) {
                return Err(ModelError::Conflict("provider facts availability profile"));
            }
            let mut source = (**template).clone();
            if sources.len() > 1 && self.is_private() {
                source = (*self.compose_private(&sources)?).clone();
            }
            if sources.len() > 1 && !self.is_private() {
                let native = self.native()?.clone();
                let relation = source.relation.clone();
                let views = sources
                    .iter()
                    .map(|source| source.view.clone())
                    .collect::<Vec<_>>();
                let budget = self.budget.clone();
                source.view = self
                    .native_calls()?
                    .call(async move {
                        native
                            .compose_completed_views(&relation, &views, &budget)
                            .await
                    })
                    .await?;
                source.rows = source.view.rows;
                source.snapshot = analysis::sources::SourceSnapshot::of_completed_view(
                    &source.relation,
                    self.model.digest(),
                    &source.view,
                )?;
            }
            source._selection_charge = Some(Arc::from(
                self.budget.reserve(
                    "provider facts availability descriptor",
                    size_of::<CompletedRelation>()
                        .saturating_add(1024)
                        .saturating_add(source.view.contributions.len().saturating_mul(96)),
                )?,
            ));
            relations.insert(
                (name, None),
                ResolvedInput {
                    role: relations.len(),
                    requested: None,
                    resolved: None,
                    source: Arc::new(source),
                },
            );
        }
        let inputs = CompletedInputs {
            attempt: self.files.clone(),
            model: self.model.digest(),
            program: ContentHash::of(b"provider-facts-availability/v1"),
            budget: self.budget.clone(),
            producing_scope: None,
            prepared_session: Arc::new(tokio::sync::OnceCell::new()),
            name: "provider-facts-availability",
            profile,
            relations,
        };
        let runtime = if self.is_private() {
            None
        } else {
            Some(self.selected_admission_runtime(&inputs)?)
        };
        // Exact provider descriptors are now privately captured. Unrelated upper handoffs
        // need not wait for the following native coverage proof.
        drop(_completion);
        let availability = if let Some(runtime) = runtime {
            runtime.facts_availability_async(profile).await?
        } else {
            let bindings = inputs
                .ordered_bindings()
                .into_iter()
                .map(|(_, b)| Self::private_binding(&b.source, b.role, b.requested, b.resolved))
                .collect::<Result<Vec<_>, _>>()?;
            let (owners, exact, _transfer) = {
                let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
                let produced = self.completed_by_producer.lock().map_err(|_| poisoned())?;
                let transfer = self.budget.reserve(
                    "private facts contributor transfer",
                    Self::private_snapshot_bytes(&owners, produced.values()),
                )?;
                let exact = produced
                    .iter()
                    .filter(|((producer, relation), _)| {
                        provider_names.contains(producer) && *relation == UniformNotRequested::NAME
                    })
                    .enumerate()
                    .map(|(role, (_, source))| Self::private_binding(source, role, None, None))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    owners
                        .by_spec
                        .values()
                        .filter(|owner| provider_names.contains(owner.spec.producer.as_str()))
                        .cloned()
                        .collect(),
                    exact,
                    transfer,
                )
            };
            self.private_validation
                .facts_availability(profile, bindings, exact, owners)
                .await?
        };
        let mut charge = StateCharge::new(self.budget(), "provider facts availability owner");
        charge.grow(
            provider_names
                .len()
                .saturating_mul(size_of::<&'static str>().saturating_add(128)),
        )?;
        let mut frozen = self
            .frozen_facts_availability
            .lock()
            .map_err(|_| poisoned())?;
        if let Some(known) = frozen.as_ref() {
            return if known.profile == profile && known.providers == *provider_names {
                Ok(())
            } else {
                Err(ModelError::Conflict("frozen facts provider selection"))
            };
        }
        *frozen = Some(FrozenFactsAvailability {
            profile,
            providers: provider_names.clone(),
            availability,
            _charge: charge,
        });
        Ok(())
    }
    pub(crate) fn admission_runtime(
        &self,
        profile: Profile,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        let runtime = self.admission_engine()?;
        let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
        let mut bytes = owners.charge.reserved().saturating_mul(3);
        let current = self.completed.lock().map_err(|_| poisoned())?;
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        for source in current.values().chain(frozen.values()) {
            bytes = bytes.saturating_add(
                serde_json::to_vec(&source.view)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(8)
                    .saturating_add(1024),
            );
        }
        let _transfer = self.budget.reserve("compiler-admission-handoff", bytes)?;
        let contributions = owners.by_spec.values().cloned().collect();
        let mut bindings = Vec::new();
        for source in current.values() {
            bindings.push(lctx_model::domain::completed::CompletedBinding {
                boundary: None,
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        for ((boundary, _), source) in frozen.iter() {
            bindings.push(lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        runtime.install_completed(profile, contributions, bindings)?;
        Ok(runtime)
    }
    pub(crate) fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        match &self.native_calls {
            Some(calls) => calls.call(future),
            None => futures::future::ready(Err(ModelError::Conflict(
                "private operation has no native effect owner",
            )))
            .boxed(),
        }
    }
    pub fn native(
        &self,
    ) -> Result<&Arc<lctx_compiler_store::compiler::NativeCompilerStore>, ModelError> {
        self.native.as_ref().ok_or(ModelError::Conflict(
            "private operation has no native target",
        ))
    }
    pub fn is_private(&self) -> bool {
        self.native.is_none()
    }
    pub fn operation_identity(&self) -> ContentHash {
        self.operation
    }
    pub fn fail(&self) {
        if let Some(native) = &self.native {
            native.fail();
        }
        self.cancellation.cancel();
    }
    pub fn set_frontier(
        &self,
        frontier: lctx_model::domain::admission::Frontier,
    ) -> Result<(), ModelError> {
        if let Some(native) = &self.native {
            native.set_frontier(frontier)?;
        }
        Ok(())
    }
    fn native_calls(&self) -> Result<&Arc<crate::native_calls::NativeCalls>, ModelError> {
        self.native_calls.as_ref().ok_or(ModelError::Conflict(
            "private operation has no native request owner",
        ))
    }
    fn bridge(&self) -> Result<&Arc<crate::native_bridge::NativeBridge>, ModelError> {
        self.bridge.as_ref().ok_or(ModelError::Conflict(
            "private operation has no native bridge",
        ))
    }
    pub fn private(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
    ) -> Result<Arc<Self>, ModelError> {
        let budget = ResourceBudget::fixed(options.memory_bytes)?;
        Self::private_with_budget(model, options, budget)
    }
    pub fn private_with_budget(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        Self::construct(model, options, budget, None, None)
    }

    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub(crate) fn scope_programs(
        &self,
    ) -> &Mutex<lctx_model::domain::scope_program::ScopeInterner> {
        self.scope_programs.programs()
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
    /// Join execution before the enclosing owner reconciles or abandons native state.
    /// Report actual finality; aggregate operation diagnostics remain with `drain_report`.
    pub async fn drain_owned_report(&self) -> lctx_model::domain::completion::Completion {
        use lctx_compiler_store::admission::ExecutionOwner;
        self.execution.drain_owned_report().await
    }
    /// Settle the original signal lifetime using the enclosing cleanup outcome.
    pub async fn complete_signal(
        &self,
        completion: &lctx_model::domain::completion::Completion,
    ) -> Result<(), ModelError> {
        use lctx_compiler_store::admission::ExecutionOwner;
        self.execution.complete_signal(completion).await
    }
    pub async fn drain_report(&self) -> lctx_model::domain::completion::Completion {
        let mut completion = self.drain_owned_report().await;
        let signal = self.complete_signal(&completion).await;
        completion.step("compiler workspace signal finality", signal);
        // Acknowledged operation refusals still fail the caller, after actual
        // execution finality has been recorded independently.
        self.append_operation_failures(&mut completion);
        completion
    }
    pub fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        use lctx_compiler_store::admission::ExecutionOwner;
        self.execution.append_operation_failures(completion);
    }
    pub async fn drain(&self) -> Result<(), ModelError> {
        lctx_model::domain::completion::complete(Ok(()), self.drain_report().await)
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
    pub(crate) fn completed_owner(
        &self,
        producer: &str,
    ) -> Result<
        (
            lctx_model::domain::completed::CompletedContribution,
            Box<dyn Reservation>,
        ),
        ModelError,
    > {
        let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
        let mut matches = owners
            .by_spec
            .values()
            .filter(|owner| owner.spec.producer == producer);
        let owner = matches
            .next()
            .ok_or(ModelError::Conflict("stage has no completed owner"))?;
        if matches.next().is_some() {
            return Err(ModelError::Conflict("stage has multiple completed owners"));
        }
        let charge = self.budget.reserve(
            "terminal stage owner descriptor",
            std::mem::size_of_val(owner)
                .saturating_add(1024)
                .saturating_add(
                    owner
                        .spec
                        .inputs
                        .iter()
                        .map(|source| std::mem::size_of_val(source) + source.heap_bytes())
                        .sum::<usize>(),
                )
                .saturating_add(owner.outputs.len().saturating_mul(256)),
        )?;
        Ok((owner.clone(), charge))
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
            let role = relations.len();
            relations.insert(
                (name, None),
                ResolvedInput {
                    role,
                    requested: None,
                    resolved: None,
                    source: self.relation(name)?,
                },
            );
        }
        Ok(CompletedInputs {
            attempt: self.files.clone(),
            model: self.model.digest(),
            program: ContentHash::of(name.as_bytes()),
            budget: self.budget.clone(),
            producing_scope: None,
            prepared_session: Arc::new(tokio::sync::OnceCell::new()),
            name,
            profile,
            relations,
        })
    }
    /// Earn and bind an exact relation selection without closing unrelated prefix members.
    async fn bind_exact_selection(
        &self,
        selection: &lctx_model::domain::stages::InputSelection,
    ) -> Result<(), ModelError> {
        if self.is_private() {
            return self.bind_private_selection(selection).await;
        }
        let Some(boundary) = selection.resolved else {
            return Ok(());
        };
        let _scratch = self.budget.reserve(
            "exact input composition scratch",
            selection
                .producers
                .len()
                .saturating_mul(1024)
                .saturating_add(1024),
        )?;
        let sources = {
            let owners = self.completed_by_producer.lock().map_err(|_| poisoned())?;
            selection
                .producers
                .iter()
                .map(|producer| {
                    owners
                        .get(&(*producer, selection.relation))
                        .cloned()
                        .ok_or(ModelError::Conflict("exact input owner is not completed"))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let template = sources
            .first()
            .ok_or(ModelError::Conflict("empty exact contributor selection"))?;
        let views = sources
            .iter()
            .map(|source| source.view.clone())
            .collect::<Vec<_>>();
        let expected = views
            .iter()
            .flat_map(|view| view.contributions.iter().copied())
            .collect();
        if let Some(existing) = self
            .frozen_shared
            .lock()
            .map_err(|_| poisoned())?
            .get(&(boundary, selection.relation))
        {
            return if existing.view.contributions == expected {
                Ok(())
            } else {
                Err(ModelError::Conflict(
                    "exact input boundary changed contributors",
                ))
            };
        }
        let native = self.native()?.clone();
        let relation = template.relation.clone();
        let budget = self.budget.clone();
        let view = self
            .native_calls()?
            .call(async move {
                native
                    .compose_completed_views(&relation, &views, &budget)
                    .await
            })
            .await?;
        let retained: Arc<dyn Reservation> = Arc::from(
            self.budget.reserve(
                "exact input descriptor",
                std::mem::size_of::<CompletedRelation>()
                    .saturating_add(1024)
                    .saturating_add(view.contributions.len().saturating_mul(96)),
            )?,
        );
        let mut source = (**template).clone();
        source._selection_charge = Some(retained);
        source.rows = view.rows;
        source.snapshot = analysis::sources::SourceSnapshot::of_completed_view(
            &source.relation,
            self.model.digest(),
            &view,
        )?;
        source.view = view;
        let source = Arc::new(source);
        let binding = lctx_model::domain::completed::CompletedBinding {
            boundary: Some(boundary.name().into()),
            source: source.snapshot(),
            view: source.view.clone(),
            configuration: None,
        };
        let native = self.native()?.clone();
        self.native_calls()?
            .call(async move { native.bind(binding).await })
            .await?;
        self.frozen_shared
            .lock()
            .map_err(|_| poisoned())?
            .insert((boundary, selection.relation), source);
        self.refresh_membership_demand()?;
        Ok(())
    }
    pub(crate) async fn bind_scheduled_inputs(
        &self,
        schedule: &lctx_model::domain::stages::Schedule,
        declaration: &lctx_model::domain::stages::Stage,
    ) -> Result<lctx_model::domain::stages::Stage, ModelError> {
        if self.is_private() {
            let selections = schedule.selections(declaration.name)?;
            self.bind_private_selections(selections).await?;
            let mut selected = declaration.clone();
            for selection in selections {
                if let Some(boundary) = selection.resolved {
                    selected.inputs[selection.role] =
                        selected.inputs[selection.role].at_epoch(boundary);
                }
            }
            return Ok(selected);
        }
        let _completion = if self.is_private() {
            None
        } else {
            Some(
                lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock())
                    .await,
            )
        };
        self.writable()?;
        let mut selected = declaration.clone();
        for selection in schedule.selections(declaration.name)? {
            self.bind_exact_selection(selection).await?;
            if let Some(boundary) = selection.resolved {
                selected.inputs[selection.role] =
                    selected.inputs[selection.role].at_epoch(boundary);
            }
        }
        Ok(selected)
    }
    pub(crate) async fn freeze_scheduled_prefix(
        &self,
        schedule: &lctx_model::domain::stages::Schedule,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        if self.is_private() {
            self.bind_private_selections(schedule.prefix_selections(boundary)?)
                .await?;
            return self.finish_membership_prefix(schedule.digest(), boundary);
        }
        let _completion = if self.is_private() {
            None
        } else {
            Some(
                lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock())
                    .await,
            )
        };
        self.writable()?;
        for selection in schedule.prefix_selections(boundary)? {
            self.bind_exact_selection(selection).await?;
        }
        self.finish_membership_prefix(schedule.digest(), boundary)?;
        Ok(())
    }
    /// Bind each declared semantic boundary to immutable shared streams. Only small
    /// descriptors are retained; later contributions cannot widen an earlier producer's inputs.
    pub async fn freeze_inputs_async(
        &self,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        let _completion =
            lctx_native::phase::wait("compiler_completion_gate", self.completion_gate.lock()).await;
        self.writable()?;
        if self
            .frozen_shared
            .lock()
            .map_err(|_| poisoned())?
            .keys()
            .any(|(existing, _)| *existing == boundary)
        {
            return Err(ModelError::Conflict(
                "compiler input boundary already frozen",
            ));
        }
        let sources = self
            .completed
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|(name, _)| lctx_model::domain::stages::is_epoch_shared(name))
            .map(|(name, source)| (*name, source.clone()))
            .collect::<Vec<_>>();
        for (_, source) in &sources {
            if self.is_private() {
                continue;
            }
            let native = self.native()?.clone();
            let binding = lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            };
            self.native_calls()?
                .call(async move { native.bind(binding).await })
                .await?;
        }
        let mut frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        for (name, source) in sources {
            frozen.insert((boundary, name), source);
        }
        Ok(())
    }
    pub fn freeze_inputs(
        &self,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        let _completion = self
            .completion_gate
            .try_lock()
            .map_err(|_| ModelError::Conflict("compiler completion handoff in progress"))?;
        self.writable()?;
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
            if self.is_private() {
                frozen.insert((boundary, *name), source.clone());
                continue;
            }
            let native = self.native()?.clone();
            let binding = lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            };
            let calls = self.native_calls()?.clone();
            self.bridge()?
                .call(async move { calls.call(async move { native.bind(binding).await }).await })?;
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
        let current = self.completed.lock().map_err(|_| poisoned())?;
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let mut relations = BTreeMap::new();
        for (role, (declared, input)) in declaration.inputs.iter().zip(&selected.inputs).enumerate()
        {
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
                current.get(input.name()).cloned().ok_or_else(|| {
                    ModelError::Invalid(format!("input {} is not completed", input.name()))
                })?
            };
            if relations
                .insert(
                    (declared.name(), declared.prefix()),
                    ResolvedInput {
                        role,
                        requested: declared.prefix(),
                        resolved: input.prefix(),
                        source,
                    },
                )
                .is_some()
            {
                return Err(ModelError::Invalid(
                    "duplicate selected compiler input".into(),
                ));
            }
        }
        Ok(CompletedInputs {
            attempt: self.files.clone(),
            model: self.model.digest(),
            program: declaration.code,
            budget: self.budget.clone(),
            producing_scope: None,
            prepared_session: Arc::new(tokio::sync::OnceCell::new()),
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
            implementation: execution_implementation(implementation),
            configuration: None,
            captured_binding: None,
            inputs,
            expected: outputs.clone(),
            allowed: outputs,
            writers: Mutex::default(),
            outcome: Mutex::new(None),
            contribution: Arc::new(Mutex::new(None)),
            registration: Arc::new(tokio::sync::Mutex::new(())),
            producing_scope: Mutex::default(),
            ingress: Arc::default(),
            failed: AtomicBool::new(false),
            product_capture: Mutex::default(),
            captured_request: std::sync::OnceLock::new(),
        }
    }
    pub fn producer(
        self: &Arc<Self>,
        declaration: &lctx_model::domain::stages::Stage,
        profile: Profile,
        inputs: CompletedInputs,
    ) -> ProducerOutput {
        let mut output = self.output(
            declaration.name,
            profile,
            declaration.code,
            inputs,
            declaration
                .outputs
                .iter()
                .chain(&declaration.contributes)
                .map(|relation| relation.name()),
        );
        output.configuration = Some(declaration.configuration);
        output.captured_binding = declaration.captured_binding.clone();
        output.expected = declaration.outputs.iter().map(|r| r.name()).collect();
        output
    }
}
/// Native ingestion belongs to compiler composition, separately from raw stage/supplier code.
/// Imported specifications bypass ordinary output creation and retain their captured identity.
fn execution_implementation(stage: ContentHash) -> ContentHash {
    let mut sink = lctx_model::domain::KeySink::new("compiler-execution-implementation/v1");
    sink.part(b"stage", &stage.0);
    sink.part(b"compiler", env!("LCTX_COMPILER_SOURCE_DIGEST").as_bytes());
    sink.finish()
}
fn poisoned() -> ModelError {
    ModelError::Invalid("workspace ownership poisoned".into())
}

/// A stream descriptor, not a resident collection or a database read capability.
#[derive(Clone)]
pub struct CompletedRelation {
    private: Option<Arc<private_source::PrivateRelation>>,
    relation: Relation,
    producer: String,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    contract: ContentHash,
    rows: u64,
    view: lctx_model::domain::completed::CompletedView,
    native: Option<Arc<lctx_compiler_store::compiler::NativeCompilerStore>>,
    bridge: Option<Arc<crate::native_bridge::NativeBridge>>,
    budget: ResourceBudget,
    batch_rows: usize,
    cancellation: Cancellation,
    inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>,
    profile: Profile,
    contribution: bool,
    snapshot: lctx_model::domain::analysis::sources::SourceSnapshot,
    _files: Arc<WorkspaceFiles>,
    _selection_charge: Option<Arc<dyn Reservation>>,
    // Useful immutable borrowers retain the operation owner, without retaining
    // Workspace or its completed-relation registry and creating a cycle.
    _execution: Arc<WorkspaceExecutionOwner>,
}
impl CompletedRelation {
    pub fn schema(&self) -> &Arc<arrow_schema::Schema> {
        self.relation.schema()
    }
    pub fn name(&self) -> &'static str {
        self.relation.name()
    }
    pub fn view(&self) -> &lctx_model::domain::completed::CompletedView {
        &self.view
    }
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
    pub fn private_source(&self) -> Result<&Arc<private_source::PrivateRelation>, ModelError> {
        self.private
            .as_ref()
            .ok_or(ModelError::Conflict("completed source is native-backed"))
    }
    pub fn table_provider(
        &self,
    ) -> Result<Arc<dyn datafusion::catalog::TableProvider>, ModelError> {
        input_catalog::private_provider(self)
    }
    pub fn batches(&self) -> Result<NativeBatches, ModelError> {
        self.batches_scoped(None)
    }
    fn batches_scoped(
        &self,
        scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    ) -> Result<NativeBatches, ModelError> {
        self.cancellation.check()?;
        if let Some(source) = &self.private {
            return NativeBatches::private(
                source,
                self.cancellation.clone(),
                self._execution.clone(),
            );
        }
        let (driver, reader) = self.prepare_batches(scope);
        self.bridge
            .as_ref()
            .ok_or(ModelError::Conflict("native source bridge absent"))?
            .launch(driver)?;
        Ok(reader)
    }
    pub async fn batches_async(&self) -> Result<NativeBatches, ModelError> {
        self.cancellation.check()?;
        if let Some(source) = &self.private {
            return NativeBatches::private(
                source,
                self.cancellation.clone(),
                self._execution.clone(),
            );
        }
        let (driver, reader) = self.prepare_batches(None);
        self.bridge
            .as_ref()
            .ok_or(ModelError::Conflict("native source bridge absent"))?
            .launch_async(driver)
            .await?;
        Ok(reader)
    }
    fn prepare_batches(
        &self,
        scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    ) -> (BoxFuture<'static, ()>, NativeBatches) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let native = self
            .native
            .clone()
            .expect("native-backed source has native owner");
        let view = self.view.clone();
        let relation = self.relation.clone();
        let budget = self.budget.clone();
        let batch_rows = self.batch_rows;
        let cancellation = self.cancellation.clone();
        let driver: BoxFuture<'static, ()> = Box::pin(async move {
            let read = async {
                let mut stream = native
                    .scan_batches(&view, &relation, None, None, &budget, batch_rows)
                    .await?;
                loop {
                    let batch = tokio::select! {
                        ()=cancellation.cancelled()=>return Ok(()),
                        batch=stream.try_next()=>batch.map_err(ModelError::codec)?,
                    };
                    let Some(batch) = batch else {
                        break;
                    };
                    cancellation.check()?;
                    let charge = budget.reserve(
                        "native-provider-handoff",
                        lctx_model::domain::logical_batch_bytes(&batch)?,
                    )?;
                    tokio::select! {
                        ()=cancellation.cancelled()=>return Ok(()),
                        result=sender.send(Ok((batch,charge)))=>if result.is_err(){return Ok(());},
                    }
                }
                Ok::<(), ModelError>(())
            };
            let result = match scope {
                Some(scope) => scope.run(read).await,
                None => read.await,
            };
            if let Err(error) = result {
                tokio::select! {
                    ()=cancellation.cancelled()=>{},
                    _=sender.send(Err(error))=>{},
                }
            }
        });
        let reader = NativeBatches {
            private: None,
            receiver,
            cancellation: self.cancellation.clone(),
            charge: None,
            done: false,
            _execution: Some(self._execution.clone()),
        };
        (driver, reader)
    }
    pub async fn read_async<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        Ok(TypedBatches {
            reader: self.batches_async().await?,
            model,
            budget,
            marker: Default::default(),
            _files: self._files.clone(),
        })
    }
    pub fn read<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<TypedBatches<R>, ModelError> {
        self.read_scoped(model, budget, None)
    }
    fn read_scoped<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
        scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    ) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        Ok(TypedBatches {
            reader: self.batches_scoped(scope)?,
            model,
            budget,
            marker: Default::default(),
            _files: self._files.clone(),
        })
    }
}
pub struct NativeBatches {
    private: Option<private_source::PrivateBatches>,
    receiver: tokio::sync::mpsc::Receiver<ChargedBatch>,
    cancellation: Cancellation,
    charge: Option<Box<dyn Reservation>>,
    done: bool,
    _execution: Option<Arc<WorkspaceExecutionOwner>>,
}
impl NativeBatches {
    fn private(
        source: &Arc<private_source::PrivateRelation>,
        cancellation: Cancellation,
        execution: Arc<WorkspaceExecutionOwner>,
    ) -> Result<Self, ModelError> {
        let (_sender, receiver) = tokio::sync::mpsc::channel(1);
        Ok(Self {
            private: Some(source.read_batches()?),
            receiver,
            cancellation,
            charge: None,
            done: false,
            _execution: Some(execution),
        })
    }
    /// Async compiler callers await the same handoff that wakes provider threads.
    pub async fn next_async(&mut self) -> Option<Result<RecordBatch, ModelError>> {
        if self.done {
            return None;
        }
        if let Some(private) = &mut self.private {
            if let Err(error) = self.cancellation.check() {
                self.done = true;
                return Some(Err(error));
            }
            let item = private.next();
            if item.is_none() || item.as_ref().is_some_and(Result::is_err) {
                self.done = true;
            }
            return item;
        }
        self.charge = None;
        let item = tokio::select! {
            biased;
            () = self.cancellation.cancelled() => Some(Err(ModelError::Invalid("compilation cancelled".into()))),
            item = lctx_native::phase::wait("native_batch_read", self.receiver.recv()) => item,
        };
        match item {
            Some(Ok((batch, charge))) => {
                self.charge = Some(charge);
                Some(Ok(batch))
            }
            Some(Err(error)) => {
                self.receiver.close();
                self.done = true;
                Some(Err(error))
            }
            None => {
                self.done = true;
                None
            }
        }
    }
}
impl Iterator for NativeBatches {
    type Item = Result<RecordBatch, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        // Genuine synchronous provider callbacks run on their existing blocking owner. Tokio
        // compiler code uses next_async; no sleeping poll loop or blocking_recv is required.
        // This independent executor cannot replenish an enclosing Tokio task's budget.
        // Exclude only the synchronous channel handoff; the driver remains cooperative.
        futures::executor::block_on(tokio::task::unconstrained(self.next_async()))
    }
}

pub struct TypedBatches<R: Record> {
    reader: NativeBatches,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    marker: std::marker::PhantomData<R>,
    _files: Arc<WorkspaceFiles>,
}
impl<R: Record> TypedBatches<R> {
    pub async fn next_async(&mut self) -> Option<Result<Batch<R>, ModelError>> {
        self.reader
            .next_async()
            .await
            .map(|batch| batch.and_then(|batch| Batch::read(&self.model, &batch, &self.budget)))
    }
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
    _private: Option<Arc<lctx_compiler_store::private_validation::CheckedPrivateScope>>,
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
            _private: self._private.clone(),
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
            || !Arc::ptr_eq(&inputs.attempt, &workspace.files)
            || inputs.model != workspace.model.digest()
            || !inputs.budget.shares_pool(workspace.budget())
            || self.inputs.profile != inputs.profile
            || self.inputs.relations.iter().any(|(key, source)| {
                inputs.relations.get(key).is_none_or(|other| {
                    source.requested != other.requested
                        || source.resolved != other.resolved
                        || !Arc::ptr_eq(&source.source, &other.source)
                })
            })
        {
            return Err(ModelError::Conflict("checked compiler inputs"));
        }
        workspace.cancellation.check()
    }
}
#[derive(Clone)]
pub struct CompletedInputs {
    attempt: Arc<WorkspaceFiles>,
    model: ContentHash,
    program: ContentHash,
    budget: ResourceBudget,
    producing_scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    prepared_session: Arc<tokio::sync::OnceCell<PreparedInputSession>>,
    name: &'static str,
    profile: Profile,
    relations: BTreeMap<
        (
            &'static str,
            Option<lctx_model::domain::stages::PublicationBoundary>,
        ),
        ResolvedInput,
    >,
}
struct PreparedInputSession {
    context: SessionContext,
    aliases: Arc<InputAliases>,
    templates: Mutex<(
        BTreeMap<ContentHash, Arc<PreparedLogicalTemplate>>,
        StateCharge,
    )>,
}
struct PreparedLogicalTemplate {
    plan: datafusion::logical_expr::LogicalPlan,
    ports: Vec<PreparedTemplatePort>,
    _charge: StateCharge,
}
struct PreparedTemplatePort {
    reference: datafusion::common::TableReference,
    provider: Arc<dyn datafusion::catalog::TableProvider>,
}
impl PreparedInputSession {
    fn prepare_template(
        &self,
        plan: &datafusion::logical_expr::LogicalPlan,
        budget: &ResourceBudget,
        sql_bytes: usize,
    ) -> Result<(bool, PreparedLogicalTemplate), ModelError> {
        use datafusion::common::tree_node::{TreeNode, TreeNodeRecursion};
        let mut charge = StateCharge::new(budget, "completed-input-logical-preparation");
        charge.grow(sql_bytes.saturating_mul(32).saturating_add(4096))?;
        let mut compatible = true;
        let mut ports = BTreeMap::new();
        plan.apply_with_subqueries(|node| {
            // Reserve traversal/expression and eventual cloned-plan allowance before growth.
            charge
                .grow(2048)
                .map_err(|error| datafusion::error::DataFusionError::External(Box::new(error)))?;
            if let datafusion::logical_expr::LogicalPlan::TableScan(scan) = node {
                if let Some(port) = self.aliases.ports.get(scan.table_name.table()) {
                    let canonical = port.resolved().map_err(|error| {
                        datafusion::error::DataFusionError::External(Box::new(error))
                    })?;
                    let captured = datafusion::datasource::source_as_provider(&scan.source)?;
                    if !Arc::ptr_eq(canonical, &captured) {
                        return Err(datafusion::error::DataFusionError::External(Box::new(
                            ModelError::Conflict("prepared completed input provider replaced"),
                        )));
                    }
                    if !ports.contains_key(&scan.table_name) {
                        let reference_bytes = scan
                            .table_name
                            .table()
                            .len()
                            .saturating_add(scan.table_name.schema().map_or(0, str::len))
                            .saturating_add(scan.table_name.catalog().map_or(0, str::len));
                        // Map and final vector coexist during collection. Reserve both.
                        charge
                            .grow(
                                size_of::<PreparedTemplatePort>()
                                    .saturating_mul(2)
                                    .saturating_add(reference_bytes.saturating_mul(4))
                                    .saturating_add(256),
                            )
                            .map_err(|error| {
                                datafusion::error::DataFusionError::External(Box::new(error))
                            })?;
                        ports.insert(scan.table_name.clone(), captured);
                    }
                } else {
                    compatible = false;
                }
            }
            for expression in node.expressions() {
                expression.apply(|expression| {
                    if matches!(expression, datafusion::logical_expr::Expr::Literal(..)) {
                        compatible = false;
                    }
                    Ok(TreeNodeRecursion::Continue)
                })?;
            }
            Ok(TreeNodeRecursion::Continue)
        })
        .map_err(crate::sql::model_error)?;
        let ports = ports
            .into_iter()
            .map(|(reference, provider)| PreparedTemplatePort {
                reference,
                provider,
            })
            .collect();
        Ok((
            compatible,
            PreparedLogicalTemplate {
                plan: plan.clone(),
                ports,
                _charge: charge,
            },
        ))
    }
}
impl PreparedLogicalTemplate {
    async fn verify_ports(&self, session: &SessionContext) -> Result<(), ModelError> {
        for port in &self.ports {
            let current = session
                .table_provider(port.reference.clone())
                .await
                .map_err(crate::sql::model_error)?;
            if !Arc::ptr_eq(&port.provider, &current) {
                return Err(ModelError::Conflict(
                    "prepared completed input provider replaced",
                ));
            }
        }
        Ok(())
    }
}
/// The declaration role and its actual frozen selector travel with the exact source.
/// This is the sole resolved inventory, not a second alias-to-view catalog.
#[derive(Clone)]
struct ResolvedInput {
    role: usize,
    requested: Option<lctx_model::domain::stages::PublicationBoundary>,
    resolved: Option<lctx_model::domain::stages::PublicationBoundary>,
    source: Arc<CompletedRelation>,
}
impl std::ops::Deref for ResolvedInput {
    type Target = Arc<CompletedRelation>;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
impl CompletedInputs {
    /// Compatibility for immutable consumer preparation, not semantic authority or an
    /// invocation's ordered source declaration. Both descriptors retain their actual sources;
    /// pointer correspondence cannot outlive that protection or cross an operation/pool.
    pub(crate) fn same_preparation_sources(&self, other: &Self) -> bool {
        // `program` identifies the consuming producer, whose declarations remain fresh.
        // A typed registry fixes the shared preparation algorithm; actual retained source
        // correspondence below includes each source producer's implementation identity.
        self.model == other.model
            && self.profile == other.profile
            && Arc::ptr_eq(&self.attempt, &other.attempt)
            && self.budget.shares_pool(&other.budget)
            && !self.relations.is_empty()
            && self.relations.len() == other.relations.len()
            && self.relations.iter().all(|(key, source)| {
                other.relations.get(key).is_some_and(|candidate| {
                    source.requested == candidate.requested
                        && source.resolved == candidate.resolved
                        && Arc::ptr_eq(&source.source, &candidate.source)
                })
            })
    }
    fn ordered_bindings(
        &self,
    ) -> Vec<(
        (
            &'static str,
            Option<lctx_model::domain::stages::PublicationBoundary>,
        ),
        &ResolvedInput,
    )> {
        let mut bindings = self
            .relations
            .iter()
            .map(|(key, value)| (*key, value))
            .collect::<Vec<_>>();
        bindings.sort_by_key(|(_, binding)| binding.role);
        bindings
    }
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
                    || consumer.relations.get(key).is_none_or(|other| {
                        source.resolved != other.resolved
                            || !Arc::ptr_eq(&source.source, &other.source)
                    })
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
        if choices.any(|(_, other)| {
            source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source)
        }) {
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
                if choices.any(|(_, other)| {
                    source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source)
                }) {
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
                        && Arc::ptr_eq(&source.source, &candidate.source)
                })
                .ok_or(ModelError::Conflict("checked input selector"))?
                .0;
            relations.insert(*original, source.clone());
        }
        if relations.is_empty() {
            return Err(ModelError::Conflict("empty checked input closure"));
        }
        Ok(Self {
            attempt: self.attempt.clone(),
            model: self.model,
            program: self.program,
            budget: self.budget.clone(),
            producing_scope: self.producing_scope.clone(),
            prepared_session: Arc::new(tokio::sync::OnceCell::new()),
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
        if sources.any(|other| {
            other.resolved != source.resolved || other.view.identity != source.view.identity
        }) {
            return Err(ModelError::Invalid(format!(
                "{} must select a completed view of {}",
                self.name,
                R::NAME
            )));
        }
        Ok(&source.source)
    }
    pub fn relation_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<&Arc<CompletedRelation>, ModelError> {
        if let Some(source) = self.relations.get(&(R::NAME, prefix)) {
            return Ok(&source.source);
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
                .find(|((name, _), source)| {
                    *name == R::NAME && source.view.identity == selected.view.identity
                })
                .map(|((_, prefix), _)| *prefix)
                .expect("selected declared source")
        };
        Ok(Self::table(R::NAME, actual_prefix))
    }
    pub fn snapshots(
        &self,
    ) -> impl Iterator<Item = lctx_model::domain::analysis::sources::SourceSnapshot> + '_ {
        self.ordered_bindings()
            .into_iter()
            .map(|(_, source)| source.snapshot())
    }
    pub fn relations(&self) -> impl Iterator<Item = &Arc<CompletedRelation>> {
        self.relations.values().map(|binding| &binding.source)
    }
    pub async fn session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        // The immutable inventory owns provider registration, so general compatible checks
        // reuse it as well. Selecting ports or changing producing scope creates a fresh cell.
        if !Arc::ptr_eq(&self.attempt, &workspace.files)
            || self.model != workspace.model.digest()
            || !self.budget.shares_pool(workspace.budget())
            || self
                .relations
                .values()
                .any(|binding| !Arc::ptr_eq(&binding.source._files, &workspace.files))
        {
            return Err(ModelError::Conflict(
                "completed input session foreign attempt",
            ));
        }
        self.prepared_session
            .get_or_try_init(|| self.prepare_session(workspace))
            .await
            .map(|prepared| prepared.context.clone())
    }
    /// Candidate providers are mutable selected ports. Borrow the prepared base providers
    /// into a private catalog so their arrays and charges end with the semantic predicate.
    pub(crate) async fn predicate_session(
        &self,
        workspace: &Workspace,
    ) -> Result<SessionContext, ModelError> {
        let base = self.session(workspace).await?;
        let prepared = self
            .prepared_session
            .get()
            .expect("prepared immutable input session");
        let context = SessionContext::new_with_config_rt(base.copied_config(), base.runtime_env());
        InputSchema::install(&context, prepared.aliases.clone(), &self.budget)?;
        Ok(context)
    }
    async fn prepare_session(
        &self,
        workspace: &Workspace,
    ) -> Result<PreparedInputSession, ModelError> {
        if !Arc::ptr_eq(&self.attempt, &workspace.files)
            || self.model != workspace.model.digest()
            || !self.budget.shares_pool(workspace.budget())
            || self
                .relations
                .values()
                .any(|binding| !Arc::ptr_eq(&binding.source._files, &workspace.files))
        {
            return Err(ModelError::Conflict(
                "completed input session foreign attempt",
            ));
        }
        let context = SessionContext::new_with_config_rt(
            workspace.context.copied_config(),
            workspace.context.runtime_env(),
        );
        // Reserve the captured descriptor catalog before its keys, nodes and port cells.
        // A plain alias can add one name per binding; it shares that binding's port.
        let mut charge = StateCharge::new(&self.budget, "completed-input-captured-aliases");
        charge.grow(
            size_of::<InputAliases>()
                + self
                    .relations
                    .keys()
                    .map(|(name, prefix)| {
                        name.len()
                            .saturating_mul(2)
                            .saturating_add(
                                prefix
                                    .map_or(7, |boundary| boundary.name().len())
                                    .saturating_mul(2),
                            )
                            .saturating_add(512)
                    })
                    .sum::<usize>(),
        )?;
        let mut aliases = BTreeMap::new();
        for ((name, prefix), source) in self.ordered_bindings() {
            let table = Self::table(name, prefix);
            tracing::info!(target: "lctx_phase", observation="captured_input_binding", producer=self.name,
                relation=name, selector=?prefix, view=?source.view.identity, rows=source.rows());
            let port = InputPort::capture(workspace, source, self.producing_scope.clone())?;
            aliases.insert(table, port);
        }
        // A plain typed read is available only when every declared selector names the same
        // immutable stream. Distinct semantic views require their qualified alias.
        let names = self
            .relations
            .keys()
            .map(|(name, _)| *name)
            .collect::<std::collections::BTreeSet<_>>();
        for name in names {
            if let Some(key) = self.plain_binding(name) {
                // This is the exact qualified binding already captured above, including its
                // producing scope and operational guards. An alias needs no second acquisition.
                let port = aliases[&Self::table(key.0, key.1)].clone();
                aliases.insert(name.to_owned(), port);
            }
        }
        let aliases = Arc::new(InputAliases {
            ports: aliases,
            _charge: charge,
        });
        InputSchema::install(&context, aliases.clone(), &self.budget)?;
        Ok(PreparedInputSession {
            context,
            aliases,
            templates: Mutex::new((
                BTreeMap::new(),
                StateCharge::new(&self.budget, "completed-input-logical-templates"),
            )),
        })
    }
    fn plain_binding(
        &self,
        name: &'static str,
    ) -> Option<(
        &'static str,
        Option<lctx_model::domain::stages::PublicationBoundary>,
    )> {
        let mut sources = self
            .relations
            .iter()
            .filter(|((candidate, _), _)| *candidate == name);
        let (key, source) = sources.next()?;
        sources
            .all(|(_, other)| {
                other.resolved == source.resolved && other.view.identity == source.view.identity
            })
            .then_some(*key)
    }
    /// Read-only logical preparation is bound to these exact captured ports and operational
    /// read scope. Physical plans, streams and predicate state remain fresh per request.
    pub(crate) async fn query_template(
        &self,
        session: &SessionContext,
        input: &lctx_model::domain::ValidationInput,
        sql: &str,
    ) -> Result<datafusion::dataframe::DataFrame, ModelError> {
        use lctx_model::domain::Key;
        let Some(prepared) = self
            .prepared_session
            .get()
            .filter(|prepared| prepared.context.session_id() == session.session_id())
        else {
            return crate::sql::query(session, sql)
                .await
                .map_err(crate::sql::model_error);
        };
        let mut key = lctx_model::domain::KeySink::new("completed-input-logical-template/v1");
        self.model.encode(&mut key);
        self.program.encode(&mut key);
        self.profile.name().to_owned().encode(&mut key);
        self.name.to_owned().encode(&mut key);
        input.encode_contract(&mut key);
        sql.to_owned().encode(&mut key);
        let key = key.finish();
        let cached = {
            prepared
                .templates
                .lock()
                .map_err(|_| poisoned())?
                .0
                .get(&key)
                .cloned()
        };
        if let Some(template) = cached {
            // Only ports captured by this plan participate. An unrelated catalog mutation
            // neither invalidates this template nor adds work to every cache hit.
            template.verify_ports(session).await?;
            return Ok(datafusion::dataframe::DataFrame::new(
                session.state(),
                template.plan.clone(),
            ));
        }
        let frame = crate::sql::query(session, sql)
            .await
            .map_err(crate::sql::model_error)?;
        let plan = frame.logical_plan();
        // Planning awaits catalog lookup. Check the providers the actual TableScans captured,
        // then recheck those same references against the current catalog before retaining it.
        let (compatible, template) = prepared.prepare_template(plan, &self.budget, sql.len())?;
        template.verify_ports(session).await?;
        // Temporary selected/candidate providers require typed rebinding and are deliberately
        // outside this immutable template owner. Never capture them under only a SQL alias.
        if compatible {
            let mut templates = prepared.templates.lock().map_err(|_| poisoned())?;
            if !templates.0.contains_key(&key) {
                // The verified template keeps its preparation reservation. Only the cache
                // entry needs a second owner; concurrent duplicate misses drop their charge.
                templates.1.grow(
                    size_of::<ContentHash>() + size_of::<Arc<PreparedLogicalTemplate>>() + 128,
                )?;
                templates.0.insert(key, Arc::new(template));
            }
        }
        Ok(frame)
    }
}

/// One producing contribution owns its bounded acceptance stream across all typed relations.
#[derive(Default)]
struct ProducerIngress {
    state: tokio::sync::Mutex<ProducerIngressState>,
}
#[derive(Default)]
struct ProducerIngressState {
    driver: Option<lctx_compiler_store::compiler::NativeIngress>,
    pending: Option<lctx_compiler_store::compiler::IngressSequence>,
    closed: bool,
}
/// Keep every acknowledged operation diagnostic without relabelling a refusal as
/// a failed join. Genuine nested finality remains typed inside the original error.
fn ingress_results(
    primary: Result<(), ModelError>,
    related: Result<(), ModelError>,
) -> Result<(), ModelError> {
    match (primary, related) {
        (Err(primary), Err(related)) => Err(primary.alongside(related)),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}
impl ProducerIngress {
    async fn settle(mut state: ProducerIngressState) -> Result<(), ModelError> {
        let mut completion = lctx_model::domain::completion::Completion::default();
        let mut operation = Ok(());
        if let Some(sequence) = state.pending.take() {
            operation = sequence.await_committed().await.map(drop);
        }
        if let Some(driver) = state.driver.take() {
            let drained = driver.finish_report().await;
            operation = ingress_results(operation, drained.operation);
            completion.merge(drained.completion);
        }
        lctx_model::domain::completion::complete(operation, completion)
    }
    async fn accept(
        &self,
        native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
        producer: ContentHash,
        batch: lctx_model::domain::CheckedCanonicalBatch,
        scope: Option<lctx_compiler_store::compiler::ProducingScope>,
        budget: ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut state = self.state.lock().await;
        if state.closed {
            return Err(ModelError::Conflict("producer ingress closed"));
        }
        if state.driver.is_none() {
            let create = async { native.ingress(producer, &budget) };
            state.driver = Some(match scope {
                Some(scope) => scope.run(create).await?,
                None => create.await?,
            });
        }
        let previous = state.pending.take();
        let accepted = state
            .driver
            .as_mut()
            .expect("created ingress")
            .accept(batch)
            .await;
        let result = match accepted {
            Ok(sequence) => {
                state.pending = Some(sequence);
                Ok(())
            }
            Err(error) => Err(error),
        };
        let mut result = result;
        if let Some(previous) = previous {
            result = ingress_results(result, previous.await_committed().await.map(drop));
        }
        if result.is_err() {
            let draining = std::mem::replace(
                &mut *state,
                ProducerIngressState {
                    closed: true,
                    ..Default::default()
                },
            );
            drop(state);
            return ingress_results(result, Self::settle(draining).await);
        }
        Ok(())
    }
    async fn acknowledge(&self) -> Result<(), ModelError> {
        let mut state = self.state.lock().await;
        if let Some(sequence) = state.pending.take() {
            sequence.await_committed().await.map(drop)?;
        }
        Ok(())
    }
    async fn finish(&self) -> Result<(), ModelError> {
        let mut state = self.state.lock().await;
        let draining = std::mem::replace(
            &mut *state,
            ProducerIngressState {
                closed: true,
                ..Default::default()
            },
        );
        drop(state);
        Self::settle(draining).await
    }
}

struct PendingRelation {
    private: Option<Arc<private_source::PrivateRelation>>,
    relation: Relation,
    contribution: bool,
}
type BatchWrite = BoxFuture<'static, Result<(), ModelError>>;
type ClosePreparation = Box<dyn FnOnce() -> Result<PreparedClose, ModelError> + Send>;
struct PreparedClose {
    pending: PendingRelation,
    write: Option<BatchWrite>,
    calls: Arc<crate::native_calls::NativeCalls>,
    keep_alive: Box<dyn ErasedWriter>,
}
/// The only record-specific native write leaf owns the rows and their full reservation.
fn owned_batch_write<R: Record>(
    ingress: Arc<ProducerIngress>,
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    producer: ContentHash,
    batch: Batch<R>,
    scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    budget: ResourceBudget,
) -> BatchWrite {
    async move {
        let batch = batch.into_checked()?;
        ingress.accept(native, producer, batch, scope, budget).await
    }
    .boxed()
}
/// Admission and final-writer lifetime are independent of record and closure types.
fn submit_batch_write(
    calls: Arc<crate::native_calls::NativeCalls>,
    write: BatchWrite,
    keep_alive: Option<Box<dyn ErasedWriter>>,
) -> BatchWrite {
    let operation = async move {
        let result = write.await;
        drop(keep_alive);
        result
    }
    .boxed();
    async move { calls.call_boxed(operation).await }.boxed()
}
fn close_writer(
    prepare: ClosePreparation,
) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
    async move {
        let PreparedClose {
            pending,
            write,
            calls,
            keep_alive,
        } = prepare()?;
        if let Some(write) = write {
            submit_batch_write(calls, write, Some(keep_alive)).await?;
        } else {
            drop(keep_alive);
        }
        Ok(pending)
    }
    .boxed()
}
trait ErasedWriter: Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn close(
        self: Box<Self>,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> futures::future::BoxFuture<'static, Result<PendingRelation, ModelError>>;
}
struct Writer<R: Record> {
    private: Option<private_source::PrivateWriter>,
    ingress: Arc<ProducerIngress>,
    producing_scope: Option<lctx_compiler_store::compiler::ProducingScope>,
    native_calls: Option<Arc<crate::native_calls::NativeCalls>>,
    native: Option<Arc<lctx_compiler_store::compiler::NativeCompilerStore>>,
    bridge: Option<Arc<crate::native_bridge::NativeBridge>>,
    producer: ContentHash,
    pending: Vec<R>,
    bytes: usize,
    charge: Box<dyn Reservation>,
    limits: TransferLimits,
    contribution: bool,
}
/// Explicit empty output of a whole unrequested owner. This writer has no row ingress
/// or downcast to a typed mutable buffer; it can only complete a zero-row membership.
struct EmptyWriter {
    relation: Relation,
    contribution: bool,
}
impl ErasedWriter for EmptyWriter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(
        self: Box<Self>,
        _model: Arc<ValidatedModel>,
        _budget: ResourceBudget,
    ) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
        Box::pin(async move {
            Ok(PendingRelation {
                private: None,
                relation: self.relation,
                contribution: self.contribution,
            })
        })
    }
}
impl<R: Record> Writer<R> {
    fn native(
        &self,
    ) -> Result<&Arc<lctx_compiler_store::compiler::NativeCompilerStore>, ModelError> {
        self.native
            .as_ref()
            .ok_or(ModelError::Conflict("private writer has no native store"))
    }
    fn native_calls(&self) -> Result<&Arc<crate::native_calls::NativeCalls>, ModelError> {
        self.native_calls
            .as_ref()
            .ok_or(ModelError::Conflict("private writer has no native calls"))
    }
    fn write_batch(&mut self, batch: Batch<R>, budget: &ResourceBudget) -> Result<(), ModelError> {
        if let Some(private) = &mut self.private {
            return private.push(batch.into_checked()?);
        }
        let write = owned_batch_write(
            self.ingress.clone(),
            self.native()?.clone(),
            self.producer,
            batch,
            self.producing_scope.clone(),
            budget.clone(),
        );
        self.bridge
            .as_ref()
            .ok_or(ModelError::Conflict("native writer bridge absent"))?
            .call_boxed(submit_batch_write(
                self.native_calls()?.clone(),
                write,
                None,
            ))
    }
    fn prepare_close(
        mut self: Box<Self>,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<PreparedClose, ModelError> {
        let pending = PendingRelation {
            private: None,
            relation: Relation::of::<R>(),
            contribution: self.contribution,
        };
        let write = if self.pending.is_empty() {
            None
        } else {
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(&model, std::mem::take(&mut self.pending), charge)?;
            let write = owned_batch_write(
                self.ingress.clone(),
                self.native()?.clone(),
                self.producer,
                batch,
                self.producing_scope.clone(),
                budget.clone(),
            );
            let ingress = self.ingress.clone();
            Some(
                async move {
                    write.await?;
                    ingress.acknowledge().await
                }
                .boxed(),
            )
        };
        Ok(PreparedClose {
            pending,
            write,
            calls: self.native_calls()?.clone(),
            keep_alive: self,
        })
    }
    fn flush(&mut self, model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.pending.is_empty() {
            // Transfer the existing row reservation into encoding; these are the same rows,
            // rather than an additional resident copy of the pending batch.
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(model, std::mem::take(&mut self.pending), charge)?;
            self.write_batch(batch, budget)?;
            self.bytes = 0;
        }
        Ok(())
    }
}
impl<R: Record> ErasedWriter for Writer<R> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(
        self: Box<Self>,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
        if self.private.is_some() {
            return async move {
                let mut writer = self;
                writer.flush(&model, &budget)?;
                let source = writer.private.take().expect("private writer").finish()?;
                Ok(PendingRelation {
                    private: Some(source),
                    relation: Relation::of::<R>(),
                    contribution: writer.contribution,
                })
            }
            .boxed();
        }
        close_writer(Box::new(move || self.prepare_close(model, budget)))
    }
}
/// One producer owns pending streams; completion makes its whole output set visible atomically.
pub struct ProducerOutput {
    workspace: Arc<Workspace>,
    name: &'static str,
    profile: Profile,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    captured_binding: Option<lctx_model::domain::producer_contract::CapturedProducerBinding>,
    inputs: CompletedInputs,
    expected: std::collections::BTreeSet<&'static str>,
    allowed: std::collections::BTreeSet<&'static str>,
    writers: Mutex<BTreeMap<&'static str, Box<dyn ErasedWriter>>>,
    outcome: Mutex<Option<ProviderOutcome>>,
    contribution: Arc<Mutex<Option<ContentHash>>>,
    registration: Arc<tokio::sync::Mutex<()>>,
    producing_scope: Mutex<Option<lctx_compiler_store::compiler::ProducingScope>>,
    ingress: Arc<ProducerIngress>,
    failed: AtomicBool,
    product_capture: Mutex<Option<lctx_model::domain::compilation_product::ProductRequest>>,
    captured_request: std::sync::OnceLock<Result<ReservedCapturedRequest, Arc<ModelError>>>,
}
type ReservedCapturedRequest = (
    lctx_model::domain::producer_contract::request::CapturedRequest,
    Box<dyn Reservation>,
);
pub(crate) type ReservedProductRequest = (
    lctx_model::domain::compilation_product::ProductRequest,
    Box<dyn Reservation>,
);
impl ProducerOutput {
    pub(crate) fn implementation_identity(&self) -> ContentHash {
        self.implementation
    }
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    fn producing_scope(&self) -> Result<lctx_compiler_store::compiler::ProducingScope, ModelError> {
        let mut scope = self.producing_scope.lock().map_err(|_| poisoned())?;
        if let Some(scope) = scope.as_ref() {
            return Ok(scope.clone());
        }
        let created = self.workspace.native()?.producing_scope(
            self.contribution_spec().identity()?,
            self.workspace.budget(),
        )?;
        *scope = Some(created.clone());
        Ok(created)
    }
    /// Bind explicit read descendants to this producer, retaining semantic input identity.
    pub(crate) fn bound_inputs(&self) -> Result<CompletedInputs, ModelError> {
        let mut inputs = self.inputs.clone();
        inputs.producing_scope = if self.workspace.is_private() {
            None
        } else {
            Some(self.producing_scope()?)
        };
        inputs.prepared_session = Arc::new(tokio::sync::OnceCell::new());
        Ok(inputs)
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
    /// The same registration operation serves sync/async declarations and empty completion.
    /// The gate retains single native registration; the ID precedes typed writer visibility.
    fn contribution_spec(&self) -> lctx_model::domain::completed::ContributionSpec {
        if let Some(Ok((request, _))) = self.captured_request.get() {
            return request.contribution().clone();
        }
        lctx_model::domain::completed::ContributionSpec {
            captured_binding: self.captured_binding.clone(),
            producer: self.name.into(),
            profile: lctx_model::domain::producer_contract::output_profile(
                self.name,
                self.captured_binding.as_ref(),
                self.profile,
            ),
            model: self.workspace.model.digest(),
            implementation: self.implementation,
            configuration: self.configuration,
            inputs: self.inputs.snapshots().collect(),
            outputs: self.allowed.iter().map(|name| (*name).to_owned()).collect(),
        }
    }
    fn registration_request(&self) -> BoxFuture<'static, Result<ContentHash, ModelError>> {
        if self.workspace.is_private() {
            return futures::future::ready(self.contribution_spec().identity()).boxed();
        }
        match self.contribution.lock() {
            Ok(contribution) => {
                if let Some(id) = *contribution {
                    return futures::future::ready(Ok(id)).boxed();
                }
            }
            Err(_) => return futures::future::ready(Err(poisoned())).boxed(),
        }
        let registration = self.registration.clone();
        let contribution = self.contribution.clone();
        let native = self.workspace.native().cloned();
        let calls = self.workspace.native_calls().cloned();
        let descriptor = self.contribution_spec();
        let scope = self.producing_scope();
        async move {
            let scope = scope?;
            let native = native?;
            let calls = calls?;
            let _registration = registration.lock().await;
            if let Some(id) = *contribution.lock().map_err(|_| poisoned())? {
                return Ok(id);
            }
            let id = calls
                .call(async move { scope.run(native.begin_contribution(descriptor)).await })
                .await?;
            *contribution.lock().map_err(|_| poisoned())? = Some(id);
            Ok(id)
        }
        .boxed()
    }
    fn check_output<R: Record>(&self) -> Result<(), ModelError> {
        self.check()?;
        if !self.allowed.contains(R::NAME) {
            return Err(ModelError::Invalid(format!(
                "{} did not declare {}",
                self.name,
                R::NAME
            )));
        }
        self.workspace.model.require::<R>()?;
        Ok(())
    }
    /// Asynchronous compiler consumers register natively without blocking their runtime.
    pub fn declare_async<R: Record>(&self) -> BoxFuture<'_, Result<(), ModelError>> {
        self.declare_kind_async::<R>(lctx_model::domain::stages::is_epoch_shared(R::NAME))
    }
    fn declare_kind_async<R: Record>(
        &self,
        contribution: bool,
    ) -> BoxFuture<'_, Result<(), ModelError>> {
        Box::pin(async move {
            let result = async {
                self.check_output::<R>()?;
                let producer = self.registration_request().await?;
                self.insert_writer::<R>(producer, contribution)
            }
            .await;
            if result.is_err() {
                self.failed.store(true, Ordering::Release);
            }
            result
        })
    }
    pub fn declare<R: Record>(&self) -> Result<(), ModelError> {
        self.declare_kind::<R>(lctx_model::domain::stages::is_epoch_shared(R::NAME))
    }
    /// Declare the remaining actual stage outputs empty after its whole-owner request
    /// policy and captured domain have been reconstructed. Existing writers are retained.
    pub(crate) async fn declare_empty_outputs(&self) -> Result<(), ModelError> {
        self.check()?;
        self.registration_request().await?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        for name in &self.allowed {
            if writers.contains_key(name) {
                continue;
            }
            let relation = self
                .workspace
                .model
                .relation(name)
                .ok_or(ModelError::Schema("empty owner output relation"))?
                .clone();
            writers.insert(
                relation.name(),
                Box::new(EmptyWriter {
                    relation,
                    contribution: lctx_model::domain::stages::is_epoch_shared(name),
                }),
            );
        }
        Ok(())
    }
    fn declare_kind<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.guarded(|| {
            self.check_output::<R>()?;
            let producer = if self.workspace.is_private() {
                self.contribution_spec().identity()?
            } else {
                self.workspace.bridge()?.call(self.registration_request())?
            };
            self.insert_writer::<R>(producer, contribution)
        })
    }
    fn insert_writer<R: Record>(
        &self,
        producer: ContentHash,
        contribution: bool,
    ) -> Result<(), ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        if writers.contains_key(R::NAME) {
            return Err(ModelError::Invalid(format!(
                "output {} declared twice",
                R::NAME
            )));
        }
        writers.insert(
            R::NAME,
            Box::new(Writer::<R> {
                private: if self.workspace.is_private() {
                    Some(private_source::PrivateWriter::new_with_spill(
                        self.workspace.model.digest(),
                        Relation::of::<R>(),
                        self.workspace.budget.clone(),
                        self.workspace.files._directory.clone(),
                        self.workspace.private_spill.clone(),
                    )?)
                } else {
                    None
                },
                ingress: self.ingress.clone(),
                producing_scope: if self.workspace.is_private() {
                    None
                } else {
                    Some(self.producing_scope()?)
                },
                native_calls: self.workspace.native_calls.clone(),
                native: self.workspace.native.clone(),
                bridge: self.workspace.bridge.clone(),
                producer,
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
    pub fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        self.guarded(|| self.write_inner(batch))
    }
    fn write_inner<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers
            .get_mut(R::NAME)
            .and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>())
            .ok_or_else(|| ModelError::Invalid(format!("output {} was not declared", R::NAME)))?;
        writer.flush(&self.workspace.model, self.workspace.budget())?;
        let result = writer.write_batch(batch, self.workspace.budget());
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
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
    /// Canonical model output transfers ownership into the existing native batch path.
    /// Native incidence still arbitrates duplicate equality across batch flushes.
    pub(crate) fn canonical_limits<R: Record>(&self) -> Result<TransferLimits, ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers
            .get_mut(R::NAME)
            .and_then(|writer| writer.as_any_mut().downcast_mut::<Writer<R>>())
            .ok_or(ModelError::Schema("undeclared canonical output"))?;
        Ok(writer.limits)
    }
    pub(crate) async fn push_owned_batch<R: Record>(
        &self,
        canonical: lctx_model::domain::normalized::CanonicalBatch<R>,
    ) -> Result<(), ModelError> {
        let result = async {
            self.check()?;
            let limits = self.canonical_limits::<R>()?;
            let batch =
                canonical.into_batch(&self.workspace.model, self.workspace.budget(), limits)?;
            if self.workspace.is_private() {
                return self.write_inner(batch);
            }
            let (producer, previous) = {
                let mut writers = self.writers.lock().map_err(|_| poisoned())?;
                let writer = writers
                    .get_mut(R::NAME)
                    .and_then(|writer| writer.as_any_mut().downcast_mut::<Writer<R>>())
                    .ok_or(ModelError::Schema("undeclared canonical output"))?;
                if batch
                    .rows()
                    .iter()
                    .any(|row| row.row_bytes() > writer.limits.max_row)
                {
                    return Err(ModelError::Schema("canonical output row limit"));
                }
                let previous = if writer.pending.is_empty() {
                    None
                } else {
                    let charge = std::mem::replace(
                        &mut writer.charge,
                        self.workspace.budget.reserve(R::NAME, 0)?,
                    );
                    let pending = Batch::with_reservation(
                        &self.workspace.model,
                        std::mem::take(&mut writer.pending),
                        charge,
                    )?;
                    writer.bytes = 0;
                    Some(pending)
                };
                (writer.producer, previous)
            };
            for batch in previous.into_iter().chain(std::iter::once(batch)) {
                let write = owned_batch_write(
                    self.ingress.clone(),
                    self.workspace.native()?.clone(),
                    producer,
                    batch,
                    Some(self.producing_scope()?),
                    self.workspace.budget().clone(),
                );
                submit_batch_write(self.workspace.native_calls()?.clone(), write, None).await?;
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub(crate) async fn push_rows<R: Record>(
        &self,
        rows: &mut lctx_model::domain::normalized::Rows<R>,
    ) -> Result<(), ModelError> {
        let result = async {
            self.check()?;
            let limits = self.canonical_limits::<R>()?;
            while let Some(batch) = rows.take_batch(limits.rows, limits.bytes)? {
                self.push_owned_batch(batch).await?;
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub async fn push<R: Record>(&self, row: R) -> Result<(), ModelError> {
        if self.workspace.is_private() {
            return self.push_sync(row);
        }
        let result = async {
            self.check()?;
            row.validate()?;
            let transfer = {
                let mut writers = self.writers.lock().map_err(|_| poisoned())?;
                let writer = writers
                    .get_mut(R::NAME)
                    .and_then(|writer| writer.as_any_mut().downcast_mut::<Writer<R>>())
                    .ok_or(ModelError::Schema("undeclared async output"))?;
                let bytes = row.row_bytes();
                if bytes > writer.limits.max_row {
                    return Err(ModelError::Invalid(format!(
                        "{} exceeds row limit",
                        R::NAME
                    )));
                }
                let transfer = if !writer.pending.is_empty()
                    && (writer.pending.len() >= writer.limits.rows
                        || writer.bytes.saturating_add(bytes) > writer.limits.bytes)
                {
                    let charge = std::mem::replace(
                        &mut writer.charge,
                        self.workspace.budget.reserve(R::NAME, 0)?,
                    );
                    let batch = Batch::with_reservation(
                        &self.workspace.model,
                        std::mem::take(&mut writer.pending),
                        charge,
                    )?;
                    writer.bytes = 0;
                    Some((writer.producer, batch))
                } else {
                    None
                };
                writer
                    .charge
                    .try_resize(writer.charge.size().saturating_add(bytes))?;
                writer.pending.push(row);
                writer.bytes += bytes;
                transfer
            };
            if let Some((producer, batch)) = transfer {
                let write = owned_batch_write(
                    self.ingress.clone(),
                    self.workspace.native()?.clone(),
                    producer,
                    batch,
                    Some(self.producing_scope()?),
                    self.workspace.budget().clone(),
                );
                submit_batch_write(self.workspace.native_calls()?.clone(), write, None).await?;
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
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
    pub fn finish(self, outcome: ProviderOutcome) -> BoxFuture<'static, Result<(), ModelError>> {
        Box::pin(async move {
            if let Err(error) = self.mark_finished(outcome) {
                let drained = self.finish_ingress().await;
                self.workspace.fail();
                self.workspace.cancellation.cancel();
                return ingress_results(Err(error), drained);
            }
            self.complete().await
        })
    }
    pub(crate) async fn finish_ingress(&self) -> Result<(), ModelError> {
        self.ingress.finish().await
    }
    pub fn complete(self) -> BoxFuture<'static, Result<(), ModelError>> {
        let ingress = self.ingress.clone();
        let workspace = self.workspace.clone();
        Box::pin(async move {
            let result = self.complete_inner().await;
            let result = ingress_results(result, ingress.finish().await);
            if result.is_err() {
                // Required completion cannot be caught and relabelled as an optional
                // capture miss, including local failure after native binding effects.
                workspace.fail();
                workspace.cancellation.cancel();
            }
            result
        })
    }
    fn complete_inner(self) -> BoxFuture<'static, Result<(), ModelError>> {
        if self.workspace.is_private() {
            return self.complete_private();
        }
        Box::pin(async move {
            self.workspace.cancellation.check()?;
            self.workspace.writable()?;
            if self.failed.load(Ordering::Acquire)
                || self.outcome.lock().map_err(|_| poisoned())?.is_none()
            {
                return Err(ModelError::Invalid(
                    "producer did not complete successfully".into(),
                ));
            }
            let spec = self.contribution_spec();
            let producing_scope = self.producing_scope()?;
            let registration = self.registration_request();
            let product_request = self.product_capture.lock().map_err(|_| poisoned())?.take();
            let writers = self.writers.into_inner().map_err(|_| poisoned())?;
            if let Some(name) = self
                .expected
                .iter()
                .find(|name| !writers.contains_key(**name))
            {
                return Err(ModelError::Invalid(format!(
                    "{} omitted completed output {name}",
                    self.name
                )));
            }
            let input_snapshots: Arc<[_]> = self.inputs.snapshots().collect::<Vec<_>>().into();
            // Flush bounded pending typed batches directly before completing exact native memberships.
            let mut pending = Vec::new();
            for writer in writers.into_values() {
                pending.push(
                    writer
                        .close(self.workspace.model.clone(), self.workspace.budget.clone())
                        .await?,
                );
            }
            {
                for name in &self.allowed {
                    if !pending.iter().any(|source| source.relation.name() == *name) {
                        let relation = self
                            .workspace
                            .model
                            .relation(name)
                            .ok_or(ModelError::Schema("declared native contribution relation"))?
                            .clone();
                        pending.push(PendingRelation {
                            private: None,
                            relation,
                            contribution: true,
                        });
                    }
                }
            }
            let id = registration.await?;
            self.ingress.finish().await?;
            producing_scope.close_and_wait().await?;
            let outputs = pending
                .iter()
                .map(|source| source.relation.clone())
                .collect::<Vec<_>>();
            let outcome = self
                .outcome
                .into_inner()
                .map_err(|_| poisoned())?
                .expect("checked producer outcome");
            let expected_outcome = outcome as i16;
            let native = self.workspace.native()?.clone();
            let budget = self.workspace.budget.clone();
            let production = self
                .workspace
                .native_calls()?
                .call(async move {
                    native
                        .complete_singleton_scoped(&producing_scope, id, outcome, &outputs, &budget)
                        .await
                })
                .await?;
            let descriptor = production.descriptor();
            if descriptor.spec != spec || descriptor.outcome != expected_outcome {
                return Err(ModelError::Conflict("completed native producer handoff"));
            }
            // Immutable composition and host carriers are prepared privately. Only
            // capture/recheck, binding and the coherent visible handoff hold the gate.
            let mut handoff_charge =
                StateCharge::new(&self.workspace.budget, "compiler completed handoff");
            let owner_key = descriptor.spec.identity()?;
            let completed_key = descriptor.identity()?;
            // Local preparation backing drops on any failure; transfer to the persistent
            // owner happens only with the coherent handoff.
            handoff_charge.grow(
                std::mem::size_of_val(descriptor)
                    + lctx_native::installed_view::contribution_heap(descriptor)
                    + 256,
            )?;
            let descriptor_copy = descriptor.clone();
            handoff_charge.grow(pending.len().saturating_mul(512))?;
            let make_relation = |source: &PendingRelation,
                                 supplied: &lctx_model::domain::completed::CompletedView|
             -> Result<Arc<CompletedRelation>, ModelError> {
                let retained = self.workspace.budget.reserve(
                    "completed relation backing",
                    std::mem::size_of::<CompletedRelation>()
                        + 128
                        + self.name.len()
                        + source.relation.name().len().saturating_mul(3)
                        + 64
                        + lctx_native::installed_view::view_heap(supplied),
                )?;
                Ok(Arc::new(CompletedRelation {
                    private: None,
                    snapshot: analysis::sources::SourceSnapshot::of_completed_view(
                        &source.relation,
                        self.workspace.model.digest(),
                        supplied,
                    )?,
                    relation: source.relation.clone(),
                    producer: self.name.into(),
                    implementation: self.implementation,
                    configuration: self.configuration,
                    contract: self.workspace.model.digest(),
                    rows: supplied.rows,
                    view: supplied.clone(),
                    native: self.workspace.native.clone(),
                    bridge: self.workspace.bridge.clone(),
                    budget: self.workspace.budget.clone(),
                    batch_rows: self.workspace.options.batch_rows,
                    cancellation: self.workspace.cancellation.clone(),
                    inputs: input_snapshots.clone(),
                    profile: self.profile,
                    contribution: source.contribution,
                    _files: self.workspace.files.clone(),
                    _selection_charge: Some(Arc::from(retained)),
                    _execution: self.workspace.execution.clone(),
                }))
            };
            let singletons = pending
                .iter()
                .map(|source| {
                    let view = production
                        .outputs()
                        .get(source.relation.name())
                        .ok_or(ModelError::Conflict("exact producer output absent"))?;
                    make_relation(source, view)
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let mut predecessors: BTreeMap<&'static str, Option<Arc<CompletedRelation>>> = {
                let _gate = lctx_native::phase::wait(
                    "compiler_completion_gate",
                    self.workspace.completion_gate.lock(),
                )
                .await;
                self.workspace.cancellation.check()?;
                self.workspace.writable()?;
                let visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
                pending
                    .iter()
                    .map(|source| {
                        let name = source.relation.name();
                        (name, visible.get(name).cloned())
                    })
                    .collect()
            };
            let mut prepared = BTreeMap::new();
            let _completion = loop {
                for (name, previous) in std::mem::take(&mut predecessors) {
                    self.workspace.cancellation.check()?;
                    // Admit the predecessor clone before moving it into registered work.
                    let clone_charge = self.workspace.budget.reserve(
                        "composition predecessor transfer",
                        previous.as_ref().map_or(0, |source| {
                            lctx_native::installed_view::view_heap(&source.view) + 128
                        }),
                    )?;
                    let prior = previous.as_ref().map(|source| source.view.clone());
                    let native = self.workspace.native()?.clone();
                    let budget = self.workspace.budget.clone();
                    let composed = production.clone();
                    #[cfg(test)]
                    self.workspace.composition_control.attempt(self.name, name);
                    let result = self
                        .workspace
                        .native_calls()?
                        .call(async move {
                            let result = native
                                .prepare_production_view(composed, name.to_owned(), prior, &budget)
                                .await;
                            drop(clone_charge);
                            result
                        })
                        .await;
                    let composed = match result {
                        Ok(composed) => composed,
                        Err(error) => {
                            // Recheck even a failed preparation before reporting it. Native
                            // failures retain original finality and are never replayed: an
                            // unequal-payload conflict is monotone as contributors grow;
                            // resource/owner/transport failures are actual attempt failures.
                            let _gate = lctx_native::phase::wait(
                                "compiler_completion_gate",
                                self.workspace.completion_gate.lock(),
                            )
                            .await;
                            let visible =
                                self.workspace.completed.lock().map_err(|_| poisoned())?;
                            let changed = visible.get(name).map(|source| &source.view)
                                != previous.as_ref().map(|source| &source.view);
                            tracing::debug!(
                                relation = name,
                                predecessor_changed = changed,
                                "private composition failed before handoff"
                            );
                            return Err(error);
                        }
                    };
                    let source = pending
                        .iter()
                        .find(|source| source.relation.name() == name)
                        .ok_or(ModelError::Schema("prepared output relation"))?;
                    let host = make_relation(source, composed.view())?;
                    prepared.insert(name, (composed, host));
                }
                #[cfg(test)]
                self.workspace.composition_control.prepared(self.name).await;
                let gate = lctx_native::phase::wait(
                    "compiler_completion_gate",
                    self.workspace.completion_gate.lock(),
                )
                .await;
                self.workspace.cancellation.check()?;
                self.workspace.writable()?;
                {
                    let owners = self
                        .workspace
                        .completed_owners
                        .lock()
                        .map_err(|_| poisoned())?;
                    if owners.by_spec.contains_key(&owner_key)
                        || owners.spec_by_completed.contains_key(&completed_key)
                    {
                        return Err(ModelError::Conflict(
                            "duplicate completed native producer handoff",
                        ));
                    }
                }
                {
                    let visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
                    let exact = self
                        .workspace
                        .completed_by_producer
                        .lock()
                        .map_err(|_| poisoned())?;
                    for source in &pending {
                        let name = source.relation.name();
                        if !source.contribution
                            && visible.get(name).is_some_and(|source| !source.contribution)
                        {
                            return Err(ModelError::Invalid(format!(
                                "completed output {name} already has an owner"
                            )));
                        }
                        if exact.contains_key(&(self.name, name)) {
                            return Err(ModelError::Conflict("duplicate exact producer output"));
                        }
                        let (composition, _) = prepared
                            .get(name)
                            .ok_or(ModelError::Schema("completed native output absent"))?;
                        if visible.get(name).map(|source| &source.view) != composition.predecessor()
                        {
                            predecessors.insert(name, visible.get(name).cloned());
                        }
                    }
                }
                if predecessors.is_empty() {
                    break gate;
                }
                // Only affected relations are invalidated. Prepared independent outputs
                // survive, and submitted native work already reached terminality above.
                for name in predecessors.keys() {
                    prepared.remove(name);
                }
                drop(gate);
                // Release settled stale ingredients under the existing current/frozen/
                // remaining-consumer demand owner before another rebase allocates.
                // Unresolved native flights and actual borrowers remain protected.
                self.workspace.refresh_membership_demand()?;
            };
            // Bind scratch is admitted before cloning all affected bindings.
            let _bindings_charge = self.workspace.budget.reserve(
                "completed visibility bind preparation",
                prepared
                    .values()
                    .map(|(_, source)| {
                        std::mem::size_of::<lctx_model::domain::completed::CompletedBinding>()
                            + 512
                            + lctx_native::installed_view::view_heap(&source.view)
                    })
                    .sum(),
            )?;
            self.workspace
                .completed_owners
                .lock()
                .map_err(|_| poisoned())?
                .charge
                .prepare_absorb(&handoff_charge)?;
            let bindings = prepared
                .values()
                .map(
                    |(_, source)| lctx_model::domain::completed::CompletedBinding {
                        boundary: None,
                        source: source.snapshot(),
                        view: source.view.clone(),
                        configuration: None,
                    },
                )
                .collect::<Vec<_>>();
            let mut completed = Vec::with_capacity(prepared.len());
            for ((composition, source), binding) in prepared.into_values().zip(bindings) {
                let native = self.workspace.native()?.clone();
                let budget = self.workspace.budget.clone();
                self.workspace
                    .native_calls()?
                    .call(async move {
                        native
                            .publish_prepared_production_view(composition, binding, &budget)
                            .await
                    })
                    .await?;
                completed.push(source);
            }
            self.workspace.cancellation.check()?;
            let mut owners = self
                .workspace
                .completed_owners
                .lock()
                .map_err(|_| poisoned())?;
            let mut by_producer = self
                .workspace
                .completed_by_producer
                .lock()
                .map_err(|_| poisoned())?;
            let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
            owners.charge.absorb(handoff_charge)?;
            owners.spec_by_completed.insert(completed_key, owner_key);
            owners.by_spec.insert(owner_key, descriptor_copy);
            for singleton in singletons {
                by_producer.insert((self.name, singleton.name()), singleton);
            }
            for source in completed {
                visible.insert(source.name(), source);
            }
            drop((owners, by_producer, visible));
            self.workspace.refresh_membership_demand()?;
            // The coherent required handoff is complete. Retention protects its own exact
            // source and is observed by the enclosing execution owner, not this readiness.
            if let Some(request) = product_request {
                let retained = self.workspace.clone();
                self.workspace.retain_child(async move {
                    retained.retain_product(request, production).await
                })?;
            }
            Ok(())
        })
    }
}

#[path = "workspace_products.rs"]
mod products;
#[cfg(test)]
pub(crate) use products::validate_product_rows;
pub(crate) use products::{ProductCandidate, decode_product_rows};

impl cpg_extract::bundle::ProviderSink for ProducerOutput {
    fn cancelled(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(self.workspace.cancellation.cancelled())
    }
    fn cancel(&self) {
        self.workspace.cancellation.cancel();
    }
    fn drain_provider(
        &self,
        thread: std::thread::JoinHandle<()>,
    ) -> tokio::sync::oneshot::Receiver<Result<(), ModelError>> {
        let (done, finished) = tokio::sync::oneshot::channel();
        let task = tokio::task::spawn_blocking(move || {
            let result = thread.join().map_err(|payload| {
                Arc::new(ModelError::Cause(Box::new(
                    lctx_model::domain::completion::ThreadPanic::new("provider", payload),
                )))
            });
            let _ = done.send(
                result
                    .as_ref()
                    .copied()
                    .map_err(|error| ModelError::SharedCause(error.clone())),
            );
            result
        });
        let owner = tracing::info_span!(target: "lctx_phase", "provider_join_owner", producer=self.name, operation=%self.workspace.operation_identity().hex());
        let joined = async move {
            lctx_native::phase::wait("provider_join", task)
                .await
                .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
        }
        .with_tracing_span(owner)
        .boxed()
        .shared();
        self.workspace
            .provider_drains
            .lock()
            .expect("provider drainage ownership")
            .push(joined);
        finished
    }
    fn read<R: Record>(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError> {
        self.check()?;
        Ok(Box::new(self.inputs.relation::<R>()?.read_scoped::<R>(
            self.workspace.model.clone(),
            self.workspace.budget.clone(),
            if self.workspace.is_private() {
                None
            } else {
                Some(self.producing_scope()?)
            },
        )?))
    }
    fn declare<R: Record>(&self) -> Result<(), ModelError> {
        ProducerOutput::declare::<R>(self)
    }
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::write(self, batch)
    }
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::contribute(self, batch)
    }
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        if self.workspace.is_private() {
            return self.mark_finished(outcome);
        }
        let ingress = self.ingress.clone();
        self.workspace
            .bridge()?
            .call(async move { ingress.finish().await })?;
        self.mark_finished(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{completion, input::Package};
    #[test]
    fn ingress_operation_group_preserves_resource_primary_and_related_unknown() {
        let budget = ResourceBudget::fixed(1).unwrap();
        let refused = budget.reserve("ingress-primary", 2).unwrap_err();
        let related = ModelError::infrastructure(
            lctx_model::domain::Infrastructure::Unconfirmed,
            "lost ingress acknowledgement",
        );
        let error = ingress_results(Err(refused), Err(related)).unwrap_err();
        assert!(matches!(
            error.primary(),
            Some(ModelError::Resource {
                owner: "ingress-primary",
                ..
            })
        ));
        assert!(!error.permits_storage_cleanup());
        assert!(matches!(error, ModelError::Operations(ref errors) if errors.related.len()==1));
        assert_eq!(budget.reserved(), 0);
    }
    fn model() -> Arc<ValidatedModel> {
        Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap())
    }
    #[tokio::test]
    async fn workspace_constructors_reuse_native_budget_and_share_the_drained_admission_memo() {
        for explicit in [false, true] {
            let budget = ResourceBudget::fixed(32 << 20).unwrap();
            let native = crate::test_native::store_with_budget(budget.clone());
            let options = WorkspaceOptions {
                memory_bytes: budget.limit(),
                ..Default::default()
            };
            let foreign = ResourceBudget::fixed(budget.limit()).unwrap();
            assert!(matches!(
                Workspace::with_budget(model(), options, foreign.clone(), native.clone()),
                Err(ModelError::Conflict(
                    "workspace native resource pool correspondence"
                ))
            ));
            assert_eq!(
                foreign.reserved(),
                0,
                "foreign pool is refused before workspace admission"
            );
            assert!(Workspace::new(model(), WorkspaceOptions::default(), native.clone()).is_err());
            let workspace = if explicit {
                Workspace::with_budget(model(), options, budget.clone(), native.clone())
            } else {
                Workspace::new(model(), options, native.clone())
            }
            .unwrap();
            assert!(workspace.budget.shares_pool(&budget));
            assert!(
                native
                    .recovery_budget()
                    .unwrap()
                    .unwrap()
                    .shares_pool(&budget)
            );
            assert!(Arc::ptr_eq(
                &workspace.admission_memo,
                &workspace.execution.admission_memo
            ));
            let connection = native
                .recovery_scope()
                .unwrap()
                .client()
                .connection_completion()
                .unwrap();
            let mut finality = workspace.drain_owned_report().await;
            finality.merge(native.abandon_finality_report().await);
            let signal = workspace.complete_signal(&finality).await;
            finality.step("workspace constructor control signal", signal);
            lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
            drop(workspace);
            drop(native);
            connection.wait().await.unwrap();
            drop(connection);
            assert_eq!(
                budget.reserved(),
                0,
                "actual owners release after explicit finality and drop"
            );
        }
    }
    #[tokio::test]
    async fn borrowed_admission_runtime_retains_producer_lifetime_until_explicit_drain() {
        use lctx_model::domain::serving::Name;
        let native = lctx_compiler_store::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_native::surrealdb::Surreal::init()),
            Name::new("planning").unwrap(),
            Name::new("planning").unwrap(),
        );
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), native).unwrap();
        let producer = Arc::downgrade(&workspace);
        let files = Arc::downgrade(&workspace.files);
        let bridge = Arc::downgrade(workspace.bridge().unwrap());
        let signal = Arc::downgrade(workspace.execution.signal.as_ref().unwrap());
        let cancellation = workspace.cancellation();
        let borrowed = workspace.admission_engine().unwrap();
        drop(workspace.admission_engine().unwrap());
        cancellation.check().unwrap();
        drop(workspace);
        assert!(
            producer.upgrade().is_none(),
            "the read catalog is not retained"
        );
        assert!(files.upgrade().is_some());
        assert!(signal.upgrade().is_some());
        cancellation.check().unwrap();
        borrowed.cancellation().check().unwrap();
        let completion = borrowed.drain_owned_report().await;
        assert!(completion.failures.is_empty(), "{completion:?}");
        assert_eq!(
            completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert_eq!(
            completion.remote,
            lctx_model::domain::completion::RemoteState::Confirmed
        );
        assert!(
            borrowed.cancellation().check().is_err(),
            "explicit drainage still cancels"
        );
        borrowed.complete_signal(&completion).await.unwrap();
        drop(borrowed);
        assert!(files.upgrade().is_none());
        assert!(bridge.upgrade().is_none());
        assert!(signal.upgrade().is_none());
    }
    #[tokio::test]
    async fn native_transfer_window_configuration_does_not_authorize_an_unconnected_provider() {
        use lctx_model::domain::{completed::CompletedView, input::Release, serving::Name};
        let native = lctx_compiler_store::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_native::surrealdb::Surreal::init()),
            Name::new("planning").unwrap(),
            Name::new("planning").unwrap(),
        );
        let workspace = Workspace::new(
            Arc::new(
                ValidatedModel::declared(vec![
                    Relation::of::<Package>(),
                    Relation::of::<Release>(),
                ])
                .unwrap(),
            ),
            WorkspaceOptions {
                memory_bytes: 128 << 20,
                batch_rows: 7,
                ..Default::default()
            },
            native.clone(),
        )
        .unwrap();
        let config = workspace.context.copied_config();
        assert_eq!(workspace.options.batch_rows, 7);
        assert_eq!(config.batch_size(), SessionConfig::default().batch_size());
        assert_eq!(
            config.target_partitions(),
            SessionConfig::default().target_partitions()
        );
        let view = CompletedView::new(
            Release::NAME.into(),
            [ContentHash::of(b"unconnected-planning")].into(),
            0,
        )
        .unwrap();
        assert!(
            native
                .table_provider(
                    &view,
                    Relation::of::<Release>(),
                    workspace.budget.clone(),
                    7
                )
                .await
                .is_err(),
            "even an empty optimizer input needs earned native authority before construction"
        );
        let mut completion = workspace.drain_owned_report().await;
        let settled = workspace.complete_signal(&completion).await;
        completion.step("unconnected transfer control signal finality", settled);
        lctx_model::domain::completion::complete::<()>(Ok(()), completion).unwrap();
    }
    #[tokio::test]
    async fn exact_scheduled_relation_selection_excludes_later_arrival_and_unfinished_peers() {
        use lctx_model::domain::{
            ValidationInput,
            input::InputRevision,
            source::{Module, SourceArtifact},
            stages::*,
            value::{AccessPath, Literal, Place, PlaceRoot},
        };
        let declared = Arc::new(lctx_model::domain::model().unwrap());
        let make = |name, inputs, outputs| Stage {
            name,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            profiles: Profile::ALL.to_vec(),
            effect: Effect::Pure,
            code: ContentHash::of(b"exact-source-control"),
            configuration: ContentHash::of(b"exact-source-control"),
            captured_binding: None,
        };
        let first = make("first_places", vec![], vec![RelationUse::of::<Place>()]);
        let later = make("later_places", vec![], vec![RelationUse::of::<Place>()]);
        let peer = make(
            "unfinished_literals",
            vec![],
            vec![RelationUse::of::<Literal>()],
        );
        let reader = make(
            "exact_reader",
            vec![RelationUse::completed::<Place>().at_epoch(PublicationBoundary::Facts)],
            vec![RelationUse::of::<Package>()],
        );
        let schedule = Schedule::build_with_publications(
            &declared,
            vec![first.clone(), later.clone(), peer, reader.clone()],
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(
                    PublicationBoundary::Facts,
                    vec!["first_places", "unfinished_literals"],
                ),
                PublicationGroup::new(PublicationBoundary::Local, vec!["later_places"]),
            ],
        )
        .unwrap();
        let input = InputRevision {
            manifest: ContentHash::of(b"exact-source-control"),
        };
        let source =
            SourceArtifact::from_bytes(input.id(), "fixture.py".into(), b"value = 1\n").unwrap();
        let module = Module {
            source: source.id(),
            qualified_name: "fixture".into(),
        };
        let place = |name: &str| Place {
            root: PlaceRoot::Global {
                module: module.id(),
                name: name.into(),
            }
            .id(),
            path: AccessPath::empty().id(),
        };
        let earlier = place("earlier");
        let late = place("later");
        let mut identities = Vec::new();
        for reversed in [false, true] {
            let native = crate::test_native::store();
            let mut workspace_owner = None;
            let result = std::panic::AssertUnwindSafe(async {
                let workspace =
                    Workspace::new(declared.clone(), Default::default(), native.clone()).unwrap();
                workspace_owner = Some(workspace.clone());
                assert!(
                    workspace
                        .bind_scheduled_inputs(&schedule, &reader)
                        .await
                        .is_err()
                );
                let order = if reversed {
                    [(&later, &late), (&first, &earlier)]
                } else {
                    [(&first, &earlier), (&later, &late)]
                };
                for (stage, row) in order {
                    let access = workspace.stage_inputs(stage, Profile::Catalog).unwrap();
                    let output = workspace.producer(stage, Profile::Catalog, access);
                    output.declare_async::<Place>().await.unwrap();
                    output.push(row.clone()).await.unwrap();
                    output.finish(ProviderOutcome::Complete).await.unwrap();
                }
                let selected = workspace
                    .bind_scheduled_inputs(&schedule, &reader)
                    .await
                    .unwrap();
                let access = workspace
                    .stage_inputs_selected(&reader, &selected, Profile::Catalog)
                    .unwrap();
                let exact = access.relation::<Place>().unwrap();
                assert_eq!(exact.rows(), 1);
                let mut actual = Vec::new();
                for batch in exact.batches_async().await.unwrap() {
                    actual.extend(Place::decode(&batch.unwrap()).unwrap());
                }
                assert_eq!(actual, vec![earlier.clone()]);
                identities.push(exact.view_identity());
                workspace
                    .freeze_scheduled_prefix(&schedule, PublicationBoundary::Local)
                    .await
                    .unwrap_err();
                // Whole-prefix completion still requires the unfinished literal owner.
                let local_places = schedule
                    .prefix_selections(PublicationBoundary::Local)
                    .unwrap()
                    .iter()
                    .find(|selection| selection.relation == Place::NAME)
                    .unwrap();
                workspace.bind_exact_selection(local_places).await.unwrap();
                assert_eq!(
                    workspace
                        .input_relation(
                            &ValidationInput::of::<Place>(&[]).at_epoch(PublicationBoundary::Local)
                        )
                        .unwrap()
                        .rows(),
                    2
                );
            })
            .catch_unwind()
            .await;
            // This control owns each source attempt: execution drainage alone
            // cannot settle its session or the workspace's enclosing signal.
            use lctx_model::domain::completion::{self, LocalState, RemoteState, StorageState};
            let mut finality = match &workspace_owner {
                Some(workspace) => workspace.drain_owned_report().await,
                None => completion::Completion::default(),
            };
            if finality.local == LocalState::Terminal
                && finality.remote == RemoteState::Confirmed
                && finality
                    .failures
                    .iter()
                    .all(|failure| failure.error.permits_storage_cleanup())
            {
                let mut source = native.abandon_finality_report().await;
                if source.local == LocalState::Outstanding {
                    finality.local = LocalState::Outstanding;
                }
                if source.remote == RemoteState::Unknown {
                    finality.remote = RemoteState::Unknown;
                }
                finality.storage.append(&mut source.storage);
                finality.committed.append(&mut source.committed);
                finality.failures.append(&mut source.failures);
            } else {
                finality.storage.push(StorageState::Orphan(format!(
                    "native_attempt:{}",
                    native.attempt().hex()
                )));
            }
            if let Some(workspace) = &workspace_owner {
                let signal = workspace.complete_signal(&finality).await;
                finality.step("exact selection workspace signal", signal);
            }
            let mut diagnostics = completion::Completion::default();
            if let Some(workspace) = &workspace_owner {
                workspace.append_operation_failures(&mut diagnostics);
            } else {
                native.append_operation_failures(&mut diagnostics);
            }
            if let Err(panic) = result {
                if let Err(error) = completion::complete(Ok(()), finality) {
                    eprintln!("exact selection finalization after assertion panic: {error:#?}");
                }
                if !diagnostics.failures.is_empty() {
                    eprintln!(
                        "exact selection operation diagnostics after assertion panic: {diagnostics:#?}"
                    );
                }
                std::panic::resume_unwind(panic);
            }
            completion::complete(Ok(()), finality).unwrap();
            completion::complete(Ok(()), diagnostics).unwrap();
        }
        assert_eq!(identities[0], identities[1]);
    }

    #[tokio::test]
    async fn logical_templates_share_only_fixed_ports_program_and_input_order() {
        use datafusion::datasource::MemTable;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let context = SessionContext::new();
        let prepared = PreparedInputSession {
            context: context.clone(),
            aliases: InputAliases::ready(BTreeMap::new(), &budget).unwrap(),
            templates: Mutex::new((
                BTreeMap::new(),
                StateCharge::new(&budget, "template-control"),
            )),
        };
        let cell = tokio::sync::OnceCell::new();
        assert!(cell.set(prepared).is_ok());
        let access = CompletedInputs {
            attempt: Arc::new(WorkspaceFiles {
                _directory: Arc::new(tempfile::tempdir().unwrap()),
            }),
            model: ContentHash::of(b"model"),
            program: ContentHash::of(b"program"),
            budget: budget.clone(),
            producing_scope: None,
            prepared_session: Arc::new(cell),
            name: "template-control",
            profile: Profile::Catalog,
            relations: BTreeMap::new(),
        };
        let input = lctx_model::domain::ValidationInput::of::<Package>(&["id"]);
        access
            .query_template(&context, &input, "SELECT current_date() AS id")
            .await
            .unwrap();
        access
            .query_template(&context, &input, "SELECT current_date() AS id")
            .await
            .unwrap();
        assert_eq!(
            access
                .prepared_session
                .get()
                .unwrap()
                .templates
                .lock()
                .unwrap()
                .0
                .len(),
            1
        );
        let order = lctx_model::domain::ValidationInput::of::<Package>(&["name"]);
        access
            .query_template(&context, &order, "SELECT current_date() AS id")
            .await
            .unwrap();
        assert_eq!(
            access
                .prepared_session
                .get()
                .unwrap()
                .templates
                .lock()
                .unwrap()
                .0
                .len(),
            2
        );
        access
            .query_template(&context, &input, "SELECT 7 AS id")
            .await
            .unwrap();
        access
            .query_template(&context, &input, "SELECT 8 AS id")
            .await
            .unwrap();
        assert_eq!(
            access
                .prepared_session
                .get()
                .unwrap()
                .templates
                .lock()
                .unwrap()
                .0
                .len(),
            2,
            "per-root literal SQL must not retain plan history"
        );
        let batch = Package::encode(&[Package {
            name: "temporary".into(),
        }])
        .unwrap();
        context
            .register_table(
                "temporary_selected_port",
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
        access
            .query_template(&context, &input, "SELECT * FROM temporary_selected_port")
            .await
            .unwrap();
        assert_eq!(
            access
                .prepared_session
                .get()
                .unwrap()
                .templates
                .lock()
                .unwrap()
                .0
                .len(),
            2,
            "selected providers require typed rebinding"
        );
        assert!(budget.reserved() > 0);
        drop(access);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn logical_templates_check_only_actual_captured_ports() {
        use datafusion::{catalog::TableProvider, datasource::MemTable};
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let context = SessionContext::new();
        let provider = |name: &str| -> Arc<dyn TableProvider> {
            let batch = Package::encode(&[Package { name: name.into() }]).unwrap();
            Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap())
        };
        let used = provider("used");
        let unrelated = provider("unrelated");
        context.register_table("fixed_port", used.clone()).unwrap();
        context
            .register_table("unrelated_port", unrelated.clone())
            .unwrap();
        let aliases = InputAliases::ready(
            [
                ("fixed_port".into(), used.clone()),
                ("unrelated_port".into(), unrelated),
            ]
            .into(),
            &budget,
        )
        .unwrap();
        let alias_baseline = budget.reserved();
        let prepared = PreparedInputSession {
            context: context.clone(),
            aliases,
            templates: Mutex::new((
                BTreeMap::new(),
                StateCharge::new(&budget, "template-port-control"),
            )),
        };
        let cell = tokio::sync::OnceCell::new();
        assert!(cell.set(prepared).is_ok());
        let access = CompletedInputs {
            attempt: Arc::new(WorkspaceFiles {
                _directory: Arc::new(tempfile::tempdir().unwrap()),
            }),
            model: ContentHash::of(b"model"),
            program: ContentHash::of(b"program"),
            budget: budget.clone(),
            producing_scope: None,
            prepared_session: Arc::new(cell),
            name: "template-port-control",
            profile: Profile::Catalog,
            relations: BTreeMap::new(),
        };
        let input = lctx_model::domain::ValidationInput::of::<Package>(&["id"]);
        let sql = "SELECT * FROM fixed_port";
        access.query_template(&context, &input, sql).await.unwrap();
        let prepared = access.prepared_session.get().unwrap();
        {
            let templates = prepared.templates.lock().unwrap();
            let template = templates.0.values().next().unwrap();
            assert_eq!(template.ports.len(), 1);
            assert_eq!(template.ports[0].reference.table(), "fixed_port");
            assert!(Arc::ptr_eq(&template.ports[0].provider, &used));
        }
        assert!(
            context
                .deregister_table("unrelated_port")
                .unwrap()
                .is_some()
        );
        context
            .register_table("unrelated_port", provider("replaced-unrelated"))
            .unwrap();
        access.query_template(&context, &input, sql).await.unwrap();
        access
            .query_template(&context, &input, "SELECT name FROM fixed_port")
            .await
            .unwrap();
        assert_eq!(
            prepared.templates.lock().unwrap().0.len(),
            2,
            "unrelated mutation affects neither hits nor misses for referenced fixed ports"
        );
        let retained_budget = budget.reserved();
        let literal = crate::sql::query(&context, "SELECT 7 AS id").await.unwrap();
        let (compatible, transient) = prepared
            .prepare_template(literal.logical_plan(), &budget, 14)
            .unwrap();
        assert!(!compatible);
        assert!(
            budget.reserved() > retained_budget,
            "even noncacheable preparation is charged while live"
        );
        drop(transient);
        drop(literal);
        assert_eq!(budget.reserved(), retained_budget);

        let replacement = provider("replaced-used");
        assert!(context.deregister_table("fixed_port").unwrap().is_some());
        context
            .register_table("fixed_port", replacement.clone())
            .unwrap();
        let hit_error = access
            .query_template(&context, &input, sql)
            .await
            .err()
            .unwrap();
        assert!(matches!(
            hit_error,
            ModelError::Conflict("prepared completed input provider replaced")
        ));
        let miss_error = access
            .query_template(&context, &input, "SELECT id FROM fixed_port")
            .await
            .err()
            .unwrap();
        assert!(matches!(
            miss_error,
            ModelError::Conflict("prepared completed input provider replaced")
        ));
        assert_eq!(
            prepared.templates.lock().unwrap().0.len(),
            2,
            "a substituted provider never becomes a cached miss"
        );
        assert_eq!(
            budget.reserved(),
            retained_budget,
            "failed preparation releases its local reservation"
        );

        // Simulate catalog replacement during planning, followed by restoration before
        // inspection. Checking the current alias alone would miss this stale capture.
        let stale = crate::sql::query(&context, sql).await.unwrap();
        assert!(context.deregister_table("fixed_port").unwrap().is_some());
        context.register_table("fixed_port", used.clone()).unwrap();
        assert!(matches!(
            prepared.prepare_template(stale.logical_plan(), &budget, sql.len()),
            Err(ModelError::Conflict(
                "prepared completed input provider replaced"
            ))
        ));
        assert_eq!(budget.reserved(), retained_budget);
        access.query_template(&context, &input, sql).await.unwrap();
        let captured = crate::sql::query(&context, sql).await.unwrap();
        let (_, template) = prepared
            .prepare_template(captured.logical_plan(), &budget, sql.len())
            .unwrap();
        assert!(
            template._charge.reserved() > 0,
            "preparation owns its allowance before catalog verification awaits"
        );
        let refused_budget = ResourceBudget::fixed(64).unwrap();
        assert!(matches!(
            prepared.prepare_template(captured.logical_plan(), &refused_budget, sql.len()),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(refused_budget.reserved(), 0);
        assert!(context.deregister_table("fixed_port").unwrap().is_some());
        context.register_table("fixed_port", replacement).unwrap();
        assert!(
            matches!(
                template.verify_ports(&context).await,
                Err(ModelError::Conflict(
                    "prepared completed input provider replaced"
                ))
            ),
            "replacement after capture is checked before retaining a planned miss"
        );
        drop(template);
        drop(captured);
        drop(stale);
        assert_eq!(budget.reserved(), retained_budget);
        assert!(context.deregister_table("fixed_port").unwrap().is_some());
        context.register_table("fixed_port", used).unwrap();
        let concurrent_sql = "SELECT name AS copied_name FROM fixed_port";
        let (left, right) = tokio::join!(
            access.query_template(&context, &input, concurrent_sql),
            access.query_template(&context, &input, concurrent_sql)
        );
        drop((left.unwrap(), right.unwrap()));
        {
            let templates = prepared.templates.lock().unwrap();
            assert_eq!(templates.0.len(), 3);
            let retained = templates
                .0
                .values()
                .map(|template| template._charge.accounted_bytes())
                .sum::<usize>();
            assert_eq!(
                budget.reserved(),
                alias_baseline + retained + templates.1.accounted_bytes(),
                "concurrent duplicate misses retain only the winning template reservation"
            );
        }
        assert!(budget.reserved() > 0);
        drop(access);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn async_native_handoff_retains_charge_and_wakes_on_cancellation() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = Cancellation::default();
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let batch = Package::encode(&[Package {
            name: "handoff".into(),
        }])
        .unwrap();
        sender
            .send(Ok((
                batch,
                budget.reserve("handoff-control", 4096).unwrap(),
            )))
            .await
            .unwrap();
        let mut reader = NativeBatches {
            private: None,
            receiver,
            cancellation: cancellation.clone(),
            charge: None,
            done: false,
            _execution: None,
        };
        assert!(reader.next_async().await.unwrap().is_ok());
        assert_eq!(budget.reserved(), 4096);
        let cancel = async {
            tokio::task::yield_now().await;
            cancellation.cancel();
        };
        let (next, ()) = tokio::join!(reader.next_async(), cancel);
        assert!(next.unwrap().is_err());
        assert_eq!(budget.reserved(), 0);
        assert!(reader.next_async().await.is_none());
    }
    #[tokio::test(flavor = "current_thread")]
    async fn sync_native_batches_progress_with_order_errors_eof_and_release() {
        for emit_error in [false, true] {
            let budget = ResourceBudget::fixed(1 << 20).unwrap();
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            let producer_budget = budget.clone();
            let producer = std::thread::spawn(move || {
                for index in 0..512 {
                    let batch = Package::encode(&[Package {
                        name: format!("handoff-{index}"),
                    }])
                    .unwrap();
                    let charge = producer_budget
                        .reserve(
                            "sync-handoff-control",
                            lctx_model::domain::logical_batch_bytes(&batch).unwrap(),
                        )
                        .unwrap();
                    sender.blocking_send(Ok((batch, charge))).unwrap();
                }
                if emit_error {
                    sender
                        .blocking_send(Err(ModelError::Schema("native batch handoff refusal")))
                        .unwrap();
                }
            });
            let mut reader = NativeBatches {
                private: None,
                receiver,
                cancellation: Cancellation::default(),
                charge: None,
                done: false,
                _execution: None,
            };
            // Consume a wide synchronous provider stream without yielding the enclosing
            // Tokio task; each actual batch retains its handoff reservation until next().
            for index in 0..512 {
                let batch = reader.next().unwrap().unwrap();
                assert_eq!(
                    Package::decode(&batch).unwrap()[0].name,
                    format!("handoff-{index}")
                );
                assert!(budget.reserved() > 0);
            }
            if emit_error {
                assert!(matches!(
                    reader.next(),
                    Some(Err(ModelError::Schema("native batch handoff refusal")))
                ));
            }
            assert!(reader.next().is_none());
            assert!(reader.next().is_none());
            producer.join().unwrap();
            drop(reader);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[tokio::test]
    async fn completed_inventory_preserves_roles_and_actual_epochs() {
        use futures::FutureExt;
        use lctx_model::domain::completion::{self, LocalState, RemoteState, StorageState};
        use lctx_model::domain::stages::PublicationBoundary;
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let output = workspace.output(
                "inventory-source",
                Profile::Catalog,
                ContentHash::of(b"inventory-source"),
                workspace
                    .inputs("inventory-source", Profile::Catalog, [])
                    .unwrap(),
                [Package::NAME],
            );
            output.declare_async::<Package>().await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let source = workspace.completed::<Package>().unwrap();
            let facts = PublicationBoundary::Facts;
            let normalized = PublicationBoundary::Dispatch;
            let access = CompletedInputs {
                attempt: workspace.files.clone(),
                model: workspace.model.digest(),
                program: ContentHash::of(b"inventory-consumer"),
                budget: workspace.budget.clone(),
                producing_scope: None,
                prepared_session: Arc::new(tokio::sync::OnceCell::new()),
                name: "inventory-consumer",
                profile: Profile::Catalog,
                relations: [
                    (
                        (Package::NAME, Some(facts)),
                        ResolvedInput {
                            role: 1,
                            requested: Some(facts),
                            resolved: Some(facts),
                            source: source.clone(),
                        },
                    ),
                    (
                        (Package::NAME, Some(normalized)),
                        ResolvedInput {
                            role: 0,
                            requested: Some(normalized),
                            resolved: Some(normalized),
                            source,
                        },
                    ),
                ]
                .into(),
            };
            let roles = access
                .ordered_bindings()
                .iter()
                .map(|(_, binding)| binding.resolved)
                .collect::<Vec<_>>();
            assert_eq!(roles, vec![Some(normalized), Some(facts)]);
            assert!(access.plain_binding(Package::NAME).is_none());
            let mut compatible = access.clone();
            compatible.prepared_session = Arc::new(tokio::sync::OnceCell::new());
            for binding in compatible.relations.values_mut() {
                binding.resolved = Some(normalized);
            }
            let selected = compatible.plain_binding(Package::NAME).unwrap();
            let compatible_session = compatible.session(&workspace).await.unwrap();
            assert!(
                compatible
                    .prepared_session
                    .get()
                    .unwrap()
                    .aliases
                    .ports
                    .values()
                    .all(|port| port.resolved().is_err()),
                "capturing a real completed inventory does not eagerly earn every native view"
            );
            let plain = compatible_session
                .table_provider(Package::NAME)
                .await
                .unwrap();
            let qualified = compatible_session
                .table_provider(CompletedInputs::table(selected.0, selected.1))
                .await
                .unwrap();
            assert!(
                Arc::ptr_eq(&plain, &qualified),
                "plain aliases borrow the selected captured provider"
            );
            let prepared = compatible.prepared_session.get().unwrap();
            for ((name, prefix), _) in compatible.ordered_bindings() {
                let provider = compatible_session
                    .table_provider(CompletedInputs::table(name, prefix))
                    .await
                    .unwrap();
                assert!(Arc::ptr_eq(
                    &provider,
                    prepared.aliases.ports[&CompletedInputs::table(name, prefix)]
                        .resolved()
                        .unwrap()
                ));
            }
            let mut distinct_view = compatible.clone();
            let binding = distinct_view
                .relations
                .get_mut(&(Package::NAME, Some(facts)))
                .unwrap();
            Arc::make_mut(&mut binding.source).view.identity = ContentHash::of(b"distinct-view");
            assert!(
                distinct_view.plain_binding(Package::NAME).is_none(),
                "matching epochs do not alias distinct views"
            );
            drop(distinct_view);
            drop(compatible_session);
            drop(compatible);
            assert!(access.relation::<Package>().is_err());
            assert!(
                access
                    .table_for(&lctx_model::domain::ValidationInput::of::<Package>(&["id"]))
                    .is_err()
            );
            let session = access.session(&workspace).await.unwrap();
            assert!(!session.table_exist(Package::NAME).unwrap());
            assert!(
                session
                    .table_exist(CompletedInputs::table(Package::NAME, Some(facts)))
                    .unwrap()
            );
            let left = access.predicate_session(&workspace).await.unwrap();
            let right = access.predicate_session(&workspace).await.unwrap();
            left.register_batch(
                "cached_normalization_control",
                Package::encode(&[Package {
                    name: "left".into(),
                }])
                .unwrap(),
            )
            .unwrap();
            right
                .register_batch(
                    "cached_normalization_control",
                    Package::encode(&[Package {
                        name: "right".into(),
                    }])
                    .unwrap(),
                )
                .unwrap();
            let (left_rows, right_rows) = tokio::join!(
                left.table("cached_normalization_control"),
                right.table("cached_normalization_control")
            );
            let (left_rows, right_rows) =
                tokio::join!(left_rows.unwrap().collect(), right_rows.unwrap().collect());
            assert_eq!(
                Package::decode(&left_rows.unwrap()[0]).unwrap()[0].name,
                "left"
            );
            assert_eq!(
                Package::decode(&right_rows.unwrap()[0]).unwrap()[0].name,
                "right"
            );
            drop(left);
            drop(right);
            assert!(
                !session.table_exist("cached_normalization_control").unwrap(),
                "candidate arrays never enter the retained input catalog"
            );
            let output = workspace.output(
                "inventory-consumer",
                Profile::Catalog,
                ContentHash::of(b"consumer"),
                access,
                [Package::NAME],
            );
            let request = output.product_request().unwrap();
            assert_eq!(request.dependencies[0].role, "input/0");
            assert_eq!(
                request.dependencies[0].prefix.as_deref(),
                Some(normalized.name())
            );
            assert_eq!(
                request.dependencies[1].prefix.as_deref(),
                Some(facts.name())
            );
        })
        .catch_unwind()
        .await;
        // The assertion scope drops all providers and candidate catalogs before joining the
        // workspace. Confirm native abandonment before settling its enclosing fixture signal.
        let mut finality = workspace.drain_owned_report().await;
        if finality.local == LocalState::Terminal
            && finality.remote == RemoteState::Confirmed
            && finality
                .failures
                .iter()
                .all(|failure| failure.error.permits_storage_cleanup())
        {
            finality.merge(workspace.native().unwrap().abandon_finality_report().await);
        } else {
            finality.storage.push(StorageState::Orphan(format!(
                "native_attempt:{}",
                workspace.native().unwrap().attempt().hex()
            )));
        }
        let signal = workspace.complete_signal(&finality).await;
        finality.step("completed inventory workspace signal", signal);
        let mut diagnostics = completion::Completion::default();
        workspace.append_operation_failures(&mut diagnostics);
        if let Err(panic) = result {
            if let Err(error) = completion::complete::<()>(Ok(()), finality) {
                eprintln!("completed inventory finalization after assertion panic: {error:#?}");
            }
            if !diagnostics.failures.is_empty() {
                eprintln!(
                    "completed inventory operation diagnostics after assertion panic: {diagnostics:#?}"
                );
            }
            std::panic::resume_unwind(panic);
        }
        completion::complete::<()>(Ok(()), finality).unwrap();
        completion::complete::<()>(Ok(()), diagnostics).unwrap();
    }
    #[tokio::test]
    async fn writer_close_preparation_is_lazy_and_refusal_releases_pending_charge() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let task_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let native = crate::test_native::store();
        let cancellation = Cancellation::default();
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &task_budget,
        ));
        let bridge = Arc::new(crate::native_bridge::NativeBridge::new(cancellation).unwrap());
        let pending_charge = 64;
        let writer = Box::new(Writer::<Package> {
            private: None,
            ingress: Arc::default(),
            producing_scope: None,
            native_calls: Some(calls.clone()),
            native: Some(native),
            bridge: Some(bridge.clone()),
            producer: ContentHash::of(b"lazy-close"),
            pending: vec![Package {
                name: "retained".into(),
            }],
            bytes: 8,
            charge: budget.reserve(Package::NAME, pending_charge).unwrap(),
            limits: TransferLimits::default(),
            contribution: true,
        });
        // The missing declaration makes preparation visibly fail on first poll. Constructing
        // the close future must neither encode the pending rows nor release their reservation.
        let undeclared = Arc::new(
            ValidatedModel::declared(vec![
                Relation::of::<lctx_model::domain::input::InputOrigin>(),
            ])
            .unwrap(),
        );
        let mut closing = writer.close(undeclared, budget.clone());
        assert_eq!(budget.reserved(), pending_charge);
        assert!(matches!(
            futures::poll!(&mut closing),
            std::task::Poll::Ready(Err(_))
        ));
        assert_eq!(budget.reserved(), 0);
        drop(closing);
        bridge.drain().await.unwrap();
        calls.drain().await.unwrap();
    }
    struct CloseOwnerWitness(Arc<AtomicBool>);
    impl Drop for CloseOwnerWitness {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    impl ErasedWriter for CloseOwnerWitness {
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
        fn close(
            self: Box<Self>,
            _: Arc<ValidatedModel>,
            _: ResourceBudget,
        ) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
            unreachable!("witness is retained by an already prepared final close")
        }
    }
    async fn cancelled_final_close_retains_batch_and_writer(late_failure: bool) {
        let batch_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = Batch::new(
            &model(),
            vec![Package {
                name: "final-retained-batch".into(),
            }],
            &batch_budget,
        )
        .unwrap();
        let charged = batch_budget.reserved();
        assert!(charged > 0);
        let native = crate::test_native::store();
        let task_budget = native.recovery_scope().unwrap().budget().clone();
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            Cancellation::default(),
            &task_budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let running = entered.clone();
        let finish = release.clone();
        let write: BatchWrite = async move {
            running.notify_one();
            finish.notified().await;
            assert_eq!(
                Package::decode(batch.arrow())?[0].name,
                "final-retained-batch"
            );
            if late_failure {
                Err(ModelError::Schema("late final batch failure"))
            } else {
                Ok(())
            }
        }
        .boxed();
        let prepared = PreparedClose {
            pending: PendingRelation {
                private: None,
                relation: Relation::of::<Package>(),
                contribution: true,
            },
            write: Some(write),
            calls: calls.clone(),
            keep_alive: Box::new(CloseOwnerWitness(dropped.clone())),
        };
        let mut caller = tokio::spawn(close_writer(Box::new(move || Ok(prepared))));
        tokio::select! {
            () = entered.notified() => {},
            result = &mut caller => panic!("final close returned before entering the submitted write: {:?}", result.map(|outcome| outcome.map(|_| ()))),
        }
        caller.abort();
        assert!(caller.await.is_err());
        assert_eq!(batch_budget.reserved(), charged);
        assert!(!dropped.load(Ordering::Acquire));
        // Interruption of drainage also retains the final operation and its erased owner.
        let mut interrupted = Box::pin(calls.drain());
        assert!(futures::poll!(&mut interrupted).is_pending());
        drop(interrupted);
        assert_eq!(batch_budget.reserved(), charged);
        assert!(!dropped.load(Ordering::Acquire));
        release.notify_one();
        let drained = calls.drain().await;
        if late_failure {
            let ModelError::Completion(outcome) = drained.unwrap_err() else {
                panic!("completion owns the late final-write failure")
            };
            assert!(
                matches!(&outcome.completion.failures[0].error,ModelError::SharedCause(error) if matches!(error.as_ref(),ModelError::Schema("late final batch failure")))
            );
        } else {
            drained.unwrap();
        }
        assert_eq!(batch_budget.reserved(), 0);
        assert!(dropped.load(Ordering::Acquire));
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            native.abandon_finality_report().await,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn cancelled_final_close_keeps_batch_and_writer_until_native_terminality() {
        cancelled_final_close_retains_batch_and_writer(false).await;
    }
    #[tokio::test]
    async fn cancelled_final_close_retains_late_failure_without_releasing_batch_early() {
        cancelled_final_close_retains_batch_and_writer(true).await;
    }
    // One row beyond the larger transfer batch exercises multiple batches in both
    // variants; this is a canonical-stream contract control, not a volume benchmark.
    const CANONICAL_STREAM_ROWS: usize = 4097;
    async fn packages(
        memory: usize,
        batch: usize,
        reverse: bool,
    ) -> (Arc<Workspace>, Arc<CompletedRelation>) {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions {
                memory_bytes: memory,
                partitions: 1,
                batch_rows: batch,
            },
            crate::test_native::store_with_limit(memory),
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
        let rows: Vec<_> = (0..CANONICAL_STREAM_ROWS)
            .map(|n| Package {
                name: format!("package-{n:08}"),
            })
            .collect();
        if reverse {
            let mut canonical = lctx_model::domain::normalized::Rows::new(workspace.budget());
            for row in rows.into_iter().rev() {
                canonical.insert(row).unwrap();
            }
            output.push_rows(&mut canonical).await.unwrap();
            assert!(canonical.is_empty(), "the canonical owner is consumed");
            // Equality is arbitrated by actual native incidence, including IDs that
            // appeared in an earlier canonical batch and no longer have host backing.
            canonical
                .insert(Package {
                    name: "package-00000000".into(),
                })
                .unwrap();
            output.push_rows(&mut canonical).await.unwrap();
        } else {
            for row in rows {
                output.push(row).await.unwrap();
            }
        }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let completed = workspace.completed::<Package>().unwrap();
        (workspace, completed)
    }
    #[tokio::test]
    async fn selected_assurance_retains_exact_owner_closure_and_refuses_missing_input_binding() {
        use futures::FutureExt;
        let native = crate::test_native::store();
        let workspace =
            Workspace::new(model(), WorkspaceOptions::default(), native.clone()).unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let first = workspace.output(
                "selected-assurance-first",
                Profile::Catalog,
                ContentHash::of(b"first"),
                workspace
                    .inputs("selected-assurance-first", Profile::Catalog, [])
                    .unwrap(),
                [Package::NAME],
            );
            first.declare_async::<Package>().await.unwrap();
            first.finish(ProviderOutcome::Complete).await.unwrap();
            let captured = workspace
                .inputs(
                    "selected-assurance-consumer",
                    Profile::Catalog,
                    [Package::NAME],
                )
                .unwrap();
            let selected_runtime = workspace.selected_admission_runtime(&captured).unwrap();
            let before = selected_runtime.completed_inventory().unwrap();
            assert_eq!(before.contributions.len(), 1);
            assert!(
                selected_runtime
                    .bind_selection_snapshot(Profile::Catalog, Vec::new(), Vec::new(), Vec::new())
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("selected assurance snapshot already bound")
            );
            assert!(
                selected_runtime
                    .admit_semantics(Profile::Catalog)
                    .await
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("selected assurance is not complete inventory authority")
            );
            assert!(
                selected_runtime
                    .install_completed(Profile::Catalog, Vec::new(), Vec::new())
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("selected assurance is not complete inventory authority")
            );
            assert!(
                lctx_compiler_store::artifact::admit(
                    &selected_runtime,
                    lctx_model::domain::admission::Frontier::Analysis,
                    Profile::Catalog,
                    ContentHash::of(b"selected-read-only-control"),
                )
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("selected assurance is not complete inventory authority")
            );
            // The refused full-admission route must not freeze the native attempt;
            // the following independent producer still completes normally.
            let uniform_before = workspace.uniform_owners(&captured).await.unwrap();
            let second = workspace.output(
                "selected-assurance-second",
                Profile::Catalog,
                ContentHash::of(b"second"),
                captured.clone(),
                [Package::NAME],
            );
            second
                .contribute(
                    Batch::<Package>::new(workspace.model(), Vec::new(), workspace.budget())
                        .unwrap(),
                )
                .unwrap();
            second.finish(ProviderOutcome::Complete).await.unwrap();
            let after = workspace
                .selected_admission_runtime(&captured)
                .unwrap()
                .completed_inventory()
                .unwrap();
            assert_eq!(before.contributions, after.contributions);
            assert_eq!(before.bindings, after.bindings);
            assert!(Arc::ptr_eq(
                &uniform_before,
                &workspace.uniform_owners(&captured).await.unwrap()
            ));
            let latest = workspace
                .inputs(
                    "selected-assurance-latest",
                    Profile::Catalog,
                    [Package::NAME],
                )
                .unwrap();
            let complete = workspace
                .selected_admission_runtime(&latest)
                .unwrap()
                .completed_inventory()
                .unwrap();
            assert_eq!(complete.contributions.len(), 2);
            assert!(
                complete.bindings.iter().any(|binding| binding.source
                    == captured.relations[&(Package::NAME, None)].snapshot())
            );
            let removed = workspace
                .completed_by_producer
                .lock()
                .unwrap()
                .remove(&("selected-assurance-first", Package::NAME))
                .unwrap();
            assert!(workspace.selected_admission_runtime(&latest).is_err());
            workspace
                .completed_by_producer
                .lock()
                .unwrap()
                .insert(("selected-assurance-first", Package::NAME), removed);
            workspace.selected_admission_runtime(&latest).unwrap();
        })
        .catch_unwind()
        .await;
        let finality = workspace.drain_report().await;
        let abandon = native.abandon_finality_report().await;
        lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap();
        lctx_model::domain::completion::complete::<()>(Ok(()), abandon).unwrap();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }
    #[tokio::test]
    async fn checked_preparation_sources_reject_foreign_context_pool_profile_epoch_and_changed_view()
     {
        use futures::FutureExt;
        use lctx_model::domain::ValidationInput;
        use lctx_model::domain::stages::PublicationBoundary;
        let native = crate::test_native::store();
        let workspace =
            Workspace::new(model(), WorkspaceOptions::default(), native.clone()).unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let output = workspace.output(
                "checked-property-source",
                Profile::Catalog,
                ContentHash::of(b"source-one"),
                workspace
                    .inputs("checked-property-source", Profile::Catalog, [])
                    .unwrap(),
                [Package::NAME],
            );
            output.declare_async::<Package>().await.unwrap();
            output
                .push(Package {
                    name: "first".into(),
                })
                .await
                .unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let access = workspace
                .inputs(
                    "checked-property-consumer",
                    Profile::Catalog,
                    [Package::NAME],
                )
                .unwrap();
            let required = [ValidationInput::of::<Package>(&["id"])];
            let checked = workspace
                .checked_inputs(&access)
                .await
                .unwrap()
                .select(&required)
                .unwrap();
            checked.require_subset(&workspace, &access).unwrap();
            let mut fresh_owner = access.clone();
            fresh_owner.name = "a-distinct-owner";
            fresh_owner.program = ContentHash::of(b"fresh-owner-program");
            fresh_owner
                .relations
                .values_mut()
                .for_each(|binding| binding.role += 10);
            assert!(
                access.same_preparation_sources(&fresh_owner),
                "shared data is independent of fresh owner program/source ordering"
            );
            checked.require_subset(&workspace, &fresh_owner).unwrap();
            for variant in 0..7 {
                let mut foreign = access.clone();
                match variant {
                    0 => foreign.profile = Profile::Behavioral,
                    1 => foreign.model = ContentHash::of(b"foreign-model"),
                    2 => foreign.budget = ResourceBudget::fixed(workspace.budget.limit()).unwrap(),
                    3 => {
                        foreign.attempt = Arc::new(WorkspaceFiles {
                            _directory: Arc::new(tempfile::tempdir().unwrap()),
                        })
                    }
                    4 => {
                        foreign.relations.values_mut().next().unwrap().resolved =
                            Some(PublicationBoundary::Facts)
                    }
                    5 => {
                        foreign.relations.values_mut().next().unwrap().requested =
                            Some(PublicationBoundary::Facts)
                    }
                    _ => {
                        let binding = foreign.relations.remove(&(Package::NAME, None)).unwrap();
                        foreign
                            .relations
                            .insert((Package::NAME, Some(PublicationBoundary::Facts)), binding);
                    }
                }
                assert!(
                    !access.same_preparation_sources(&foreign),
                    "variant {variant} cannot reuse preparation"
                );
                assert!(
                    checked.require_subset(&workspace, &foreign).is_err(),
                    "variant {variant} cannot consume checked properties"
                );
            }
            let output = workspace.output(
                "changed-property-source",
                Profile::Catalog,
                ContentHash::of(b"source-two"),
                workspace
                    .inputs("changed-property-source", Profile::Catalog, [])
                    .unwrap(),
                [Package::NAME],
            );
            // A second ordinary owner is forbidden. An explicit contribution grows the
            // current composed view while the earlier captured view remains immutable.
            output
                .contribute(
                    Batch::new(
                        workspace.model(),
                        vec![Package {
                            name: "second".into(),
                        }],
                        workspace.budget(),
                    )
                    .unwrap(),
                )
                .unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let changed = workspace
                .inputs(
                    "changed-property-consumer",
                    Profile::Catalog,
                    [Package::NAME],
                )
                .unwrap();
            assert!(!access.same_preparation_sources(&changed));
            assert!(checked.require_subset(&workspace, &changed).is_err());
            checked.require_subset(&workspace, &access).unwrap();
        })
        .catch_unwind()
        .await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "checked preparation workspace drainage",
            workspace.drain().await,
        );
        terminal.merge(native.abandon_finality_report().await);
        let cleaned = lctx_model::domain::completion::complete::<()>(Ok(()), terminal);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("checked preparation cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test]
    async fn sync_and_async_declarations_share_registration_before_writer_visibility() {
        use lctx_model::domain::input::Release;
        let declared = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let workspace = Workspace::new(
            declared,
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = Arc::new(
            workspace.output(
                "mixed-declarations",
                Profile::Catalog,
                ContentHash::of(b"mixed-declarations"),
                workspace
                    .inputs("mixed-declarations", Profile::Catalog, [])
                    .unwrap(),
                [Package::NAME, Release::NAME],
            ),
        );
        let sync = output.clone();
        let (registered, joined) = tokio::join!(
            output.declare_async::<Release>(),
            tokio::task::spawn_blocking(move || sync.declare::<Package>())
        );
        registered.unwrap();
        joined.unwrap().unwrap();
        assert!(output.contribution.lock().unwrap().is_some());
        assert_eq!(
            output
                .writers
                .lock()
                .unwrap()
                .keys()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            [Package::NAME, Release::NAME].into_iter().collect()
        );
        let output = Arc::try_unwrap(output).ok().unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let contributions = workspace
            .native()
            .unwrap()
            .contributions(workspace.budget())
            .await
            .unwrap();
        assert_eq!(contributions.len(), 1);
        assert_eq!(contributions[0].outputs.len(), 2);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 0);
        assert_eq!(workspace.completed::<Release>().unwrap().rows(), 0);
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn async_undeclared_output_refuses_before_native_registration_and_duplicate_refuses() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let refused = workspace.output(
            "undeclared-async",
            Profile::Catalog,
            ContentHash::of(b"undeclared-async"),
            workspace
                .inputs("undeclared-async", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        assert!(refused.declare_async::<Package>().await.is_err());
        assert!(
            workspace
                .native()
                .unwrap()
                .contributions(workspace.budget())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(refused.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.cancellation().check().is_err());
        let duplicate_workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let duplicate = duplicate_workspace.output(
            "duplicate-async",
            Profile::Catalog,
            ContentHash::of(b"duplicate-async"),
            duplicate_workspace
                .inputs("duplicate-async", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        duplicate.declare_async::<Package>().await.unwrap();
        assert!(
            matches!(duplicate.declare_async::<Package>().await,Err(ModelError::Invalid(message)) if message.contains("declared twice"))
        );
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        for attempt in [workspace, duplicate_workspace] {
            let budget = attempt.budget().clone();
            let native = attempt.native().unwrap().clone();
            let connection = native
                .recovery_scope()
                .unwrap()
                .client()
                .connection_completion()
                .unwrap();
            attempt.drain().await.unwrap();
            native.abandon().await.unwrap();
            drop(attempt);
            drop(native);
            connection.wait().await.unwrap();
            drop(connection);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[tokio::test]
    async fn shared_stream_preserves_order_exact_snapshot_and_read_only_refusal() {
        use lctx_model::domain::{ValidationInput, analysis::sources::CompletedInput};
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "stream-control",
            Profile::Catalog,
            ContentHash::of(b"stream-control"),
            workspace
                .inputs("stream-control", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        output.declare_async::<Package>().await.unwrap();
        for name in ["z", "a", "m"] {
            output.push(Package { name: name.into() }).await.unwrap();
        }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let inputs = workspace
            .inputs("stream-consumer", Profile::Catalog, [Package::NAME])
            .unwrap();
        let session = inputs.session(&workspace).await.unwrap();
        let permit = inputs.read::<Package>().unwrap();
        let declaration = ValidationInput::of::<Package>(&["name"]);
        let selected = format!(
            "SELECT * FROM {}",
            crate::consumed_rows::identifier(&inputs.table_for(&declaration).unwrap())
        );
        let mut names = Vec::new();
        crate::consumed_rows::stream_query_at(
            &permit,
            &declaration,
            &inputs,
            &session,
            &selected,
            |_, batch| {
                names.extend(Package::decode(batch)?.into_iter().map(|row| row.name));
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!(names, ["a", "m", "z"]);
        let foreign = CompletedInput::<Package>::new(
            "different-view",
            workspace.model.digest(),
            ContentHash::of(b"different-implementation"),
            ContentHash::of(b"different-view"),
            3,
        )
        .unwrap();
        let mut visited = false;
        assert!(matches!(
            crate::consumed_rows::stream_query_at(
                &foreign,
                &declaration,
                &inputs,
                &session,
                &selected,
                |_, _| {
                    visited = true;
                    Ok(())
                }
            )
            .await,
            Err(ModelError::Conflict(
                "scoped stream completed input mismatch"
            ))
        ));
        assert!(!visited);
        assert!(
            crate::consumed_rows::stream_query_at(
                &permit,
                &declaration,
                &inputs,
                &session,
                "DELETE FROM packages",
                |_, _| {
                    visited = true;
                    Ok(())
                }
            )
            .await
            .is_err()
        );
        assert!(!visited);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 3);
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn canonical_stream_is_independent_of_batching_order_and_memory_limit() {
        let (ordinary_workspace, ordinary) = packages(64 << 20, 4096, false).await;
        // The pool covers simultaneous native compression and guarded stream preparation,
        // as well as bounded ingestion and reads; it remains smaller than the ordinary pool.
        let (constrained_workspace, constrained) = packages(32 << 20, 128, true).await;
        assert_eq!(ordinary.view_identity(), constrained.view_identity());
        assert_eq!(
            constrained.rows(),
            u64::try_from(CANONICAL_STREAM_ROWS).unwrap()
        );
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
        assert_eq!(rows, CANONICAL_STREAM_ROWS);
        drop(ordinary);
        drop(constrained);
        for workspace in [ordinary_workspace, constrained_workspace] {
            let budget = workspace.budget().clone();
            let native = workspace.native().unwrap().clone();
            let connection = native
                .recovery_scope()
                .unwrap()
                .client()
                .connection_completion()
                .unwrap();
            workspace.drain().await.unwrap();
            native.abandon().await.unwrap();
            drop(workspace);
            drop(native);
            connection.wait().await.unwrap();
            drop(connection);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[tokio::test]
    async fn provider_completion_retains_typed_panic_and_terminal_state() {
        use cpg_extract::bundle::ProviderSink;
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "provider-panic",
            Profile::Catalog,
            ContentHash::of(b"provider-panic"),
            workspace
                .inputs("provider-panic", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        let acknowledgement =
            output.drain_provider(std::thread::spawn(|| std::panic::panic_any(17_u32)));
        let acknowledged = acknowledgement.await.unwrap().unwrap_err();
        let ModelError::SharedCause(cause) = acknowledged else {
            panic!()
        };
        assert!(
            matches!(cause.as_ref(),ModelError::Cause(error) if error.downcast_ref::<lctx_model::domain::completion::ThreadPanic>().is_some())
        );
        let completion = workspace.drain_report().await;
        assert_eq!(
            completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert_eq!(
            completion.remote,
            lctx_model::domain::completion::RemoteState::Confirmed
        );
        assert_eq!(completion.failures.len(), 1);
        assert!(matches!(
            &completion.failures[0].error,
            ModelError::SharedCause(_)
        ));
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn stage_join_precedes_late_provider_descendant_snapshot() {
        use cpg_extract::bundle::ProviderSink;
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "late-provider",
            Profile::Catalog,
            ContentHash::of(b"late-provider"),
            workspace
                .inputs("late-provider", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        let (release, released) = std::sync::mpsc::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let observed = completed.clone();
        // Actual provider work starts before stop; its eventual join registration is delayed.
        let provider = std::thread::spawn(move || {
            released.recv().unwrap();
            observed.store(true, Ordering::Release);
        });
        let cancellation = workspace.cancellation();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (registered, registering) = tokio::sync::oneshot::channel();
        let stage = workspace
            .spawn_stage(async move {
                entered.send(()).unwrap();
                cancellation.cancelled().await;
                let _acknowledgement = output.drain_provider(provider);
                registered.send(()).unwrap();
                Ok(())
            })
            .unwrap();
        entering.await.unwrap();
        let mut drain = Box::pin(workspace.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        registering.await.unwrap();
        assert!(futures::poll!(&mut drain).is_pending());
        assert!(!completed.load(Ordering::Acquire));
        release.send(()).unwrap();
        completion::complete(Ok(()), drain.await).unwrap();
        stage.await.unwrap();
        assert!(completed.load(Ordering::Acquire));
        completion::complete(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn independently_joined_stage_completes_its_native_contribution_without_self_drain() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "joined-contribution",
            Profile::Catalog,
            ContentHash::of(b"joined-contribution"),
            workspace
                .inputs("joined-contribution", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        workspace
            .spawn_stage(async move { output.finish(ProviderOutcome::Complete).await })
            .unwrap()
            .await
            .unwrap();
        completion::complete(Ok(()), workspace.drain_report().await).unwrap();
        completion::complete(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn interrupted_workspace_drain_retains_provider_join_for_retry() {
        use cpg_extract::bundle::ProviderSink;
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "provider-drain",
            Profile::Catalog,
            ContentHash::of(b"provider-drain"),
            workspace
                .inputs("provider-drain", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        let entered = Arc::new(tokio::sync::Notify::new());
        let completed = Arc::new(AtomicBool::new(false));
        let thread_entered = entered.clone();
        let thread_completed = completed.clone();
        let thread = std::thread::spawn(move || {
            thread_entered.notify_one();
            std::thread::sleep(std::time::Duration::from_millis(80));
            thread_completed.store(true, Ordering::Release);
        });
        let acknowledgement = output.drain_provider(thread);
        entered.notified().await;
        let first_owner = workspace.clone();
        let first = tokio::spawn(async move { first_owner.drain().await });
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        first.abort();
        assert!(first.await.is_err());
        workspace.drain().await.unwrap();
        assert!(completed.load(Ordering::Acquire));
        acknowledgement.await.unwrap().unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn manual_output_inventory_is_closed_and_empty_inventory_is_metadata_only() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "metadata-only",
            Profile::Catalog,
            ContentHash::of(b"metadata-only"),
            workspace
                .inputs("metadata-only", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        let identity = output.contribution.clone();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        // Metadata-only completion is durable without inventing a relation binding. The
        // publication closure accessor deliberately omits owners with no reachable outputs.
        let identity = identity
            .lock()
            .unwrap()
            .expect("metadata-only native registration");
        let contribution = workspace
            .native()
            .unwrap()
            .completed_contribution(identity)
            .await
            .unwrap();
        assert!(contribution.spec.outputs.is_empty());
        assert!(contribution.outputs.is_empty());
        assert_eq!(workspace.completed_owners.lock().unwrap().by_spec.len(), 1);
        assert!(
            workspace
                .native()
                .unwrap()
                .contributions(workspace.budget())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(workspace.completed::<Package>().is_err());
        let refused = workspace.output(
            "undeclared",
            Profile::Catalog,
            ContentHash::of(b"undeclared"),
            workspace
                .inputs("undeclared", Profile::Catalog, [])
                .unwrap(),
            [],
        );
        assert!(refused.declare::<Package>().is_err());
        assert!(refused.finish(ProviderOutcome::Complete).await.is_err());
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_drains_an_idle_full_native_handoff() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions {
                batch_rows: 1,
                ..Default::default()
            },
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "idle-handoff",
            Profile::Catalog,
            ContentHash::of(b"idle-handoff"),
            workspace
                .inputs("idle-handoff", Profile::Catalog, [])
                .unwrap(),
            [<Package>::NAME],
        );
        output.declare_async::<Package>().await.unwrap();
        for name in ["one", "two", "three"] {
            output.push(Package { name: name.into() }).await.unwrap();
        }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let relation = workspace.completed::<Package>().unwrap();
        let mut batches = relation.batches().unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        tokio::time::timeout(std::time::Duration::from_secs(5), workspace.drain())
            .await
            .unwrap()
            .unwrap();
        assert!(batches.next().unwrap().is_err());
        assert!(batches.next().is_none());
    }
    #[tokio::test]
    async fn cancellation_and_incomplete_output_never_become_completed() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
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
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let declaration = Stage {
            captured_binding: None,
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
        // Establish the real native owner first. This control concerns an ignored row
        // refusal with4KiB available, not a promise to bootstrap native storage in4KiB.
        let budget =
            ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap();
        let constrained = Workspace::with_budget(
            model(),
            WorkspaceOptions {
                memory_bytes: budget.limit(),
                ..Default::default()
            },
            budget.clone(),
            crate::test_native::store_with_budget(budget),
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
        let pressure = constrained
            .budget()
            .reserve(
                "ignored row refusal pressure",
                constrained.budget().limit() - constrained.budget().reserved() - 4096,
            )
            .unwrap();
        assert!(
            output
                .push(Package {
                    name: "x".repeat(5000)
                })
                .await
                .is_err()
        );
        drop(pressure);
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(constrained.completed::<Package>().is_err());
        for workspace in [workspace, constrained] {
            let mut finality = workspace.drain_owned_report().await;
            finality.merge(workspace.native().unwrap().abandon_finality_report().await);
            let signal = workspace.complete_signal(&finality).await;
            finality.step("row refusal control signal", signal);
            lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
        }
    }
    #[tokio::test]
    async fn conflicting_payload_refuses_the_whole_output_set() {
        use lctx_model::domain::{input::InputRevision, source::SourceArtifact};
        let workspace = Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions::default(),
            crate::test_native::store(),
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
        let finality = workspace.drain_owned_report().await;
        workspace.complete_signal(&finality).await.unwrap();
        lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn ordinary_owner_adopts_native_contributions_then_refuses_another_owner() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
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
        contributed.contribute(batch).unwrap();
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
        let finality = workspace.drain_owned_report().await;
        workspace.complete_signal(&finality).await.unwrap();
        lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn nominal_reference_closure_reuses_completed_streams_and_refuses_missing_targets() {
        use lctx_model::domain::input::Release;
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        for valid in [true, false] {
            let workspace = Workspace::new(
                model.clone(),
                WorkspaceOptions::default(),
                crate::test_native::store(),
            )
            .unwrap();
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
    async fn completed_input_keeps_native_membership_alive_and_later_contributions_do_not_change_it()
     {
        use lctx_compiler_store::admission::ExecutionOwner;
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
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
        next.contribute(batch).unwrap();
        next.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(
            bound.relation::<Package>().unwrap().view_identity(),
            original.view_identity()
        );
        assert_eq!(original.rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        let weak_workspace = Arc::downgrade(&workspace);
        let weak_execution = Arc::downgrade(&workspace.execution);
        let weak_signal = Arc::downgrade(workspace.execution.signal.as_ref().unwrap());
        let native = workspace.native().unwrap().clone();
        drop(bound);
        drop(workspace);
        assert!(
            weak_workspace.upgrade().is_none(),
            "borrowers do not retain Workspace"
        );
        assert!(weak_execution.upgrade().is_some());
        assert!(weak_signal.upgrade().is_some());
        let mut source_reader = original.batches().unwrap();
        assert_eq!(
            source_reader
                .next_async()
                .await
                .unwrap()
                .unwrap()
                .num_rows(),
            1
        );
        assert!(source_reader.next_async().await.is_none());
        drop(source_reader);
        let mut reader = original.batches().unwrap();
        drop(original);
        assert!(
            weak_execution.upgrade().is_some(),
            "the reader retains its own execution lease"
        );
        assert_eq!(reader.next_async().await.unwrap().unwrap().num_rows(), 1);
        assert!(reader.next_async().await.is_none());
        // Require clean finality only after real readers reach EOF. Interrupting
        // an accepted native request can truthfully retain remote uncertainty.
        let execution = weak_execution.upgrade().unwrap();
        let finality = execution.drain_owned_report().await;
        assert!(execution.cancellation.check().is_err());
        drop(reader);
        execution.complete_signal(&finality).await.unwrap();
        lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            native.abandon_finality_report().await,
        )
        .unwrap();
        drop(execution);
        assert!(
            weak_execution.upgrade().is_none(),
            "no completed-registry ownership cycle"
        );
        assert!(weak_signal.upgrade().is_none());
    }
    #[tokio::test]
    async fn explicit_workspace_drain_refuses_borrowed_relation_reads() {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let output = workspace.output(
            "drained-borrower",
            Profile::Catalog,
            ContentHash::of(b"drained-borrower"),
            workspace
                .inputs("drained-borrower", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        output.declare_async::<Package>().await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let source = workspace.completed::<Package>().unwrap();
        workspace.drain().await.unwrap();
        assert!(
            source.batches().is_err(),
            "a retained execution lease cannot reopen explicit drainage"
        );
        drop(source);
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
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
            WorkspaceOptions::default(),
            crate::test_native::store(),
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

impl lctx_compiler_store::admission::AdmissionResources for Workspace {
    fn admission_resources(
        self: &Arc<Self>,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        self.admission_engine()
    }
}
impl lctx_compiler_store::admission::NativeRuntime for Workspace {
    fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        Workspace::native_call(self, future)
    }
}
impl lctx_compiler_store::admission::InputTables for CompletedInputs {
    fn table_for(&self, input: &lctx_model::domain::ValidationInput) -> Result<String, ModelError> {
        self.validation_table(input)
    }
    fn contains_relation(&self, name: &str) -> bool {
        self.relations
            .keys()
            .any(|(candidate, _)| *candidate == name)
    }
}
impl lctx_compiler_store::admission::ValidationPlanner for CompletedInputs {
    fn query<'a>(
        &'a self,
        session: &'a SessionContext,
        input: &'a lctx_model::domain::ValidationInput,
        sql: &'a str,
    ) -> BoxFuture<'a, Result<datafusion::dataframe::DataFrame, ModelError>> {
        self.query_template(session, input, sql).boxed()
    }
}

#[cfg(test)]
mod composition_controls {
    use super::*;
    use lctx_model::domain::{
        input::{Package, Release},
        value::Literal,
    };

    #[derive(Default)]
    pub(super) struct Control {
        pauses: Mutex<BTreeMap<&'static str, Arc<Pause>>>,
        attempts: Mutex<BTreeMap<(&'static str, &'static str), usize>>,
    }
    #[derive(Default)]
    struct Pause {
        ready: tokio::sync::Notify,
        resume: tokio::sync::Notify,
    }
    impl Control {
        pub(super) fn attempt(&self, producer: &'static str, relation: &'static str) {
            *self
                .attempts
                .lock()
                .unwrap()
                .entry((producer, relation))
                .or_default() += 1;
        }
        pub(super) async fn prepared(&self, producer: &'static str) {
            let pause = self.pauses.lock().unwrap().remove(producer);
            if let Some(pause) = pause {
                pause.ready.notify_one();
                pause.resume.notified().await;
            }
        }
    }
    #[tokio::test]
    async fn private_composition_rebases_only_changed_outputs_and_never_publishes_cancelled_work() {
        for cancelled in [false, true] {
            let model = Arc::new(
                ValidatedModel::declared(vec![
                    Relation::of::<Package>(),
                    Relation::of::<Release>(),
                    Relation::of::<Literal>(),
                ])
                .unwrap(),
            );
            let workspace = Workspace::new(
                model,
                WorkspaceOptions::default(),
                crate::test_native::store(),
            )
            .unwrap();
            let output = workspace.output(
                "paused",
                Profile::Catalog,
                ContentHash::of(b"private-composition"),
                workspace.inputs("paused", Profile::Catalog, []).unwrap(),
                [Package::NAME, Release::NAME],
            );
            let package = Package { name: "one".into() };
            output
                .contribute(
                    Batch::new(workspace.model(), vec![package.clone()], workspace.budget())
                        .unwrap(),
                )
                .unwrap();
            output
                .contribute(
                    Batch::new(
                        workspace.model(),
                        vec![Release {
                            package: package.id(),
                            version: "1".into(),
                        }],
                        workspace.budget(),
                    )
                    .unwrap(),
                )
                .unwrap();
            let pause = Arc::new(Pause::default());
            workspace
                .composition_control
                .pauses
                .lock()
                .unwrap()
                .insert("paused", pause.clone());
            let mut completing = tokio::spawn(output.finish(ProviderOutcome::Complete));
            tokio::select! {
                () = pause.ready.notified() => {},
                result = &mut completing => panic!("output finish returned before private composition pause: {result:?}"),
            }
            assert!(
                workspace.completed::<Package>().is_err(),
                "private preparations are invisible"
            );
            let peer = workspace.output(
                "peer",
                Profile::Catalog,
                ContentHash::of(b"peer"),
                workspace.inputs("peer", Profile::Catalog, []).unwrap(),
                [Package::NAME],
            );
            peer.contribute(
                Batch::new(
                    workspace.model(),
                    vec![Package { name: "two".into() }],
                    workspace.budget(),
                )
                .unwrap(),
            )
            .unwrap();
            peer.finish(ProviderOutcome::Complete).await.unwrap();
            let old = workspace.completed::<Package>().unwrap();
            let other = workspace.output(
                "unrelated",
                Profile::Catalog,
                ContentHash::of(b"other"),
                workspace.inputs("unrelated", Profile::Catalog, []).unwrap(),
                [Literal::NAME],
            );
            other
                .contribute(
                    Batch::new(workspace.model(), vec![Literal::None], workspace.budget()).unwrap(),
                )
                .unwrap();
            other.finish(ProviderOutcome::Complete).await.unwrap();
            if cancelled {
                workspace.cancellation.cancel();
            }
            pause.resume.notify_one();
            let result = completing.await.unwrap();
            if cancelled {
                assert!(result.is_err());
                assert_eq!(
                    workspace.completed::<Package>().unwrap().view_identity(),
                    old.view_identity()
                );
                assert!(workspace.completed::<Release>().is_err());
            } else {
                result.unwrap();
                assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
                assert_eq!(workspace.completed::<Release>().unwrap().rows(), 1);
                assert_eq!(workspace.completed::<Literal>().unwrap().rows(), 1);
                assert_eq!(old.rows(), 1, "old selections remain exact");
                assert_eq!(
                    workspace
                        .composition_control
                        .attempts
                        .lock()
                        .unwrap()
                        .get(&("paused", Package::NAME)),
                    Some(&2)
                );
                assert_eq!(
                    workspace
                        .composition_control
                        .attempts
                        .lock()
                        .unwrap()
                        .get(&("paused", Release::NAME)),
                    Some(&1)
                );
                workspace.validate().await.unwrap();
            }
            let completion = workspace.drain_owned_report().await;
            workspace.complete_signal(&completion).await.unwrap();
            lctx_model::domain::completion::complete::<()>(Ok(()), completion).unwrap();
            lctx_model::domain::completion::complete::<()>(
                Ok(()),
                workspace.native().unwrap().abandon_finality_report().await,
            )
            .unwrap();
        }
    }
    #[tokio::test]
    async fn host_private_composition_rebases_only_changed_outputs_and_never_publishes_cancelled_work()
     {
        for cancelled in [false, true] {
            let model = Arc::new(
                ValidatedModel::declared(vec![
                    Relation::of::<Package>(),
                    Relation::of::<Release>(),
                    Relation::of::<Literal>(),
                ])
                .unwrap(),
            );
            let workspace = Workspace::private(model, WorkspaceOptions::default()).unwrap();
            let output = workspace.output(
                "paused",
                Profile::Catalog,
                ContentHash::of(b"private-composition"),
                workspace.inputs("paused", Profile::Catalog, []).unwrap(),
                [Package::NAME, Release::NAME],
            );
            let package = Package { name: "one".into() };
            output
                .contribute(
                    Batch::new(workspace.model(), vec![package.clone()], workspace.budget())
                        .unwrap(),
                )
                .unwrap();
            output
                .contribute(
                    Batch::new(
                        workspace.model(),
                        vec![Release {
                            package: package.id(),
                            version: "1".into(),
                        }],
                        workspace.budget(),
                    )
                    .unwrap(),
                )
                .unwrap();
            let pause = Arc::new(Pause::default());
            workspace
                .composition_control
                .pauses
                .lock()
                .unwrap()
                .insert("paused", pause.clone());
            let mut completing = tokio::spawn(output.finish(ProviderOutcome::Complete));
            tokio::select! {
                () = pause.ready.notified() => {},
                result = &mut completing => panic!("output finish returned before private composition pause: {result:?}"),
            }
            assert!(
                workspace.completed::<Package>().is_err(),
                "private preparations are invisible"
            );
            let peer = workspace.output(
                "peer",
                Profile::Catalog,
                ContentHash::of(b"peer"),
                workspace.inputs("peer", Profile::Catalog, []).unwrap(),
                [Package::NAME],
            );
            peer.contribute(
                Batch::new(
                    workspace.model(),
                    vec![Package { name: "two".into() }],
                    workspace.budget(),
                )
                .unwrap(),
            )
            .unwrap();
            peer.finish(ProviderOutcome::Complete).await.unwrap();
            let old = workspace.completed::<Package>().unwrap();
            let other = workspace.output(
                "unrelated",
                Profile::Catalog,
                ContentHash::of(b"other"),
                workspace.inputs("unrelated", Profile::Catalog, []).unwrap(),
                [Literal::NAME],
            );
            other
                .contribute(
                    Batch::new(workspace.model(), vec![Literal::None], workspace.budget()).unwrap(),
                )
                .unwrap();
            other.finish(ProviderOutcome::Complete).await.unwrap();
            if cancelled {
                workspace.cancellation.cancel();
            }
            pause.resume.notify_one();
            let result = completing.await.unwrap();
            if cancelled {
                assert!(result.is_err());
                assert_eq!(
                    workspace.completed::<Package>().unwrap().view_identity(),
                    old.view_identity()
                );
                assert!(workspace.completed::<Release>().is_err());
            } else {
                result.unwrap();
                assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
                assert_eq!(workspace.completed::<Release>().unwrap().rows(), 1);
                assert_eq!(workspace.completed::<Literal>().unwrap().rows(), 1);
                assert_eq!(old.rows(), 1, "old selections remain exact");
                assert_eq!(
                    workspace
                        .composition_control
                        .attempts
                        .lock()
                        .unwrap()
                        .get(&("paused", Package::NAME)),
                    Some(&2)
                );
                assert_eq!(
                    workspace
                        .composition_control
                        .attempts
                        .lock()
                        .unwrap()
                        .get(&("paused", Release::NAME)),
                    Some(&1)
                );
                workspace.validate().await.unwrap();
            }
            let completion = workspace.drain_owned_report().await;
            workspace.complete_signal(&completion).await.unwrap();
            lctx_model::domain::completion::complete::<()>(Ok(()), completion).unwrap();
        }
    }
}
