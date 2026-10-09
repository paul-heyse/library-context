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
use futures::{
    FutureExt, TryStreamExt,
    future::{BoxFuture, Shared},
};
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

type ProviderDrain = Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
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
            memory_bytes: lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            partitions: SessionConfig::default().target_partitions(),
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
            if self.0.cancelled.load(Ordering::Acquire) {
                return;
            }
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
#[derive(PartialEq, Eq)]
struct FactsAvailabilityKey {
    profile: Profile,
    views: [ContentHash; 9],
}
pub(crate) struct AdmittedFacts {
    pub contract: ContentHash,
    pub availability: Arc<lctx_model::domain::admission::ScopedAvailability>,
    pub reporting: BTreeMap<
        (
            lctx_model::domain::attribution::FactFamily,
            Option<lctx_model::domain::Id<lctx_model::domain::attribution::Provider>>,
        ),
        &'static str,
    >,
    _reporting_charge: StateCharge,
}
/// The attempt owns one runtime, spill directory, buffer budget, and completed relation registry.
pub struct Workspace {
    files: Arc<WorkspaceFiles>,
    context: SessionContext,
    options: WorkspaceOptions,
    budget: ResourceBudget,
    scope_programs: Arc<lctx_model::domain::scope_program::ScopeProgramRuntime>,
    product_cache: Mutex<Option<Arc<lctx_surrealdb::NativeProductCache>>>,
    model: Arc<ValidatedModel>,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    bridge: Arc<crate::native_bridge::NativeBridge>,
    native_calls: Arc<crate::native_calls::NativeCalls>,
    content_frozen: AtomicBool,
    completed_owners: Mutex<(
        BTreeMap<ContentHash, lctx_model::domain::completed::CompletedContribution>,
        StateCharge,
    )>,
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
    facts_availability: Mutex<Option<(FactsAvailabilityKey, Arc<AdmittedFacts>)>>,
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
        let budget = ResourceBudget::from_pool(Arc::new(WorkspacePool {
            memory: runtime.memory_pool.clone(),
            limit: options.memory_bytes,
        }))?;
        let checked_premises = Mutex::new((
            Default::default(),
            StateCharge::new(&budget, "compiler-validity-premises"),
        ));
        let completed_owners = Mutex::new((
            BTreeMap::new(),
            StateCharge::new(&budget, "compiler-completed-owners"),
        ));
        let cancellation = Cancellation::default();
        let bridge = Arc::new(crate::native_bridge::NativeBridge::new(
            cancellation.clone(),
        )?);
        let native_calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &budget,
        ));
        Ok(Arc::new(Self {
            files: Arc::new(WorkspaceFiles { directory }),
            context: SessionContext::new_with_config_rt(config, runtime.clone()),
            options,
            scope_programs: lctx_model::domain::scope_program::ScopeProgramRuntime::new(&model,&budget)?,
            product_cache: Mutex::default(),
            budget,
            model,
            native,
            bridge,
            native_calls,
            content_frozen: AtomicBool::new(false),
            completed_owners,
            completed: Mutex::default(),
            frozen_shared: Mutex::default(),
            cancellation,
            completion_gate: tokio::sync::Mutex::new(()),
            compilation_complete: Mutex::default(),
            provider_drains: Mutex::default(),
            checked_premises,
            facts_availability: Mutex::default(),
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
        owner.checked_premises = Mutex::new((
            Default::default(),
            StateCharge::new(&budget, "compiler-validity-premises"),
        ));
        owner.completed_owners = Mutex::new((
            BTreeMap::new(),
            StateCharge::new(&budget, "compiler-completed-owners"),
        ));
        owner.native_calls = Arc::new(crate::native_calls::NativeCalls::new(
            owner.native.clone(),
            owner.cancellation.clone(),
            &budget,
        ));
        owner.scope_programs = lctx_model::domain::scope_program::ScopeProgramRuntime::new(&owner.model,&budget)?;
        owner.budget = budget;
        Ok(workspace)
    }
    /// Repeated attempts can reuse one model/runtime program owner while each completed source,
    /// native contribution, cancellation token and semantic capability remains attempt-local.
    pub fn with_program_runtime(
        model:Arc<ValidatedModel>,options:WorkspaceOptions,budget:ResourceBudget,
        native:Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
        programs:Arc<lctx_model::domain::scope_program::ScopeProgramRuntime>,
    )->Result<Arc<Self>,ModelError> {
        programs.require_model(&model)?;
        let mut workspace=Self::with_budget(model,options,budget,native)?;
        Arc::get_mut(&mut workspace).expect("new workspace has one owner").scope_programs=programs;
        Ok(workspace)
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
        let contributions = self.native.contributions().await?;
        let mut output_names = std::collections::BTreeSet::new();
        let mut owners = BTreeMap::new();
        let mut owner_charge = StateCharge::new(&self.budget, "compiler-completed-owners");
        for contribution in contributions {
            owner_charge.grow(
                serde_json::to_vec(&contribution)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(4)
                    .saturating_add(256),
            )?;
            if owners
                .insert(contribution.spec.identity()?, contribution.clone())
                .is_some()
            {
                return Err(ModelError::Conflict(
                    "duplicate restored contribution owner",
                ));
            }
            contribution.identity()?;
            let spec = contribution.spec;
            if spec.model != self.model.digest() || spec.profile != profile {
                return Err(ModelError::Conflict(
                    "restored contribution model or profile",
                ));
            }
            for input in spec.inputs {
                if input.model() != self.model.digest()
                    || input.rows() < 0
                    || self.model.relation(input.relation()).is_none()
                {
                    return Err(ModelError::Conflict(
                        "restored contribution dependency model",
                    ));
                }
            }
            for output in spec.outputs {
                if self.model.relation(&output).is_none() {
                    return Err(ModelError::Schema("restored contribution output relation"));
                }
                output_names.insert(output);
            }
        }
        let bindings = self.native.bindings().await?;
        let current_names = bindings
            .iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view.relation.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if current_names != output_names {
            return Err(ModelError::Conflict(
                "restored current output binding inventory",
            ));
        }
        let mut completed = BTreeMap::new();
        let mut frozen = BTreeMap::new();
        for binding in bindings {
            binding.validate()?;
            if binding.source.model() != self.model.digest() {
                return Err(ModelError::Conflict("restored compiler model"));
            }
            let relation = self
                .model
                .relation(binding.source.relation())
                .ok_or(ModelError::Schema("restored compiler relation"))?
                .clone();
            let name = relation.name();
            if binding.configuration.is_some()
                || binding.source
                    != lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                        &relation,
                        self.model.digest(),
                        &binding.view,
                    )?
            {
                return Err(ModelError::Conflict(
                    "restored neutral completed view binding",
                ));
            }
            let source = Arc::new(CompletedRelation {
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
            });
            if let Some(boundary) = binding.boundary {
                let boundary = lctx_model::domain::stages::PublicationBoundary::ALL
                    .into_iter()
                    .find(|candidate| candidate.name() == boundary)
                    .ok_or(ModelError::Schema("restored compiler boundary"))?;
                if frozen.insert((boundary, name), source).is_some() {
                    return Err(ModelError::Conflict("duplicate frozen restored binding"));
                }
            } else if completed.insert(name, source).is_some() {
                return Err(ModelError::Conflict("duplicate current restored binding"));
            }
        }
        *self.completed_owners.lock().map_err(|_| poisoned())? = (owners, owner_charge);
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
        if let Some((captures, frontier, profile, settings)) = request {
            let capture_identity = self.captures(captures)?;
            let completed = self.compilation_complete.lock().map_err(|_| poisoned())?;
            if completed.as_ref().is_none_or(|done| {
                done.captures != capture_identity
                    || done.frontier != frontier
                    || done.profile != profile
                    || done.configuration != settings
            }) {
                return Err(ModelError::Invalid(
                    "artifact requires the completed requested compilation and exact captures"
                        .into(),
                ));
            }
        }
        let _gate = self.completion_gate.lock().await;
        self.cancellation.check()?;
        self.content_frozen.store(true, Ordering::Release);
        let descriptor_bytes = self
            .completed_owners
            .lock()
            .map_err(|_| poisoned())?
            .1
            .reserved();
        let binding_bytes = {
            let current = self.completed.lock().map_err(|_| poisoned())?;
            let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
            current
                .values()
                .chain(frozen.values())
                .try_fold(0usize, |bytes, source| {
                    let serialized = serde_json::to_vec(&source.view)
                        .map_err(ModelError::codec)?
                        .len();
                    Ok::<_, ModelError>(
                        bytes.saturating_add(serialized.saturating_mul(8).saturating_add(512)),
                    )
                })?
        };
        let _handoff_charge = self.budget.reserve(
            "compiler-final-handoff",
            descriptor_bytes
                .saturating_mul(3)
                .saturating_add(binding_bytes),
        )?;
        let inventory = self.native.freeze_content().await?;
        let expected_owners = self
            .completed_owners
            .lock()
            .map_err(|_| poisoned())?
            .0
            .clone();
        let mut actual_owners = BTreeMap::new();
        for owner in inventory.contributions {
            if actual_owners
                .insert(owner.spec.identity()?, owner)
                .is_some()
            {
                return Err(ModelError::Conflict(
                    "duplicate frozen native completed owner",
                ));
            }
        }
        if actual_owners != expected_owners {
            return Err(ModelError::Conflict(
                "frozen native completed owner handoff",
            ));
        }
        let mut expected_bindings = Vec::new();
        for source in self.completed.lock().map_err(|_| poisoned())?.values() {
            expected_bindings.push(lctx_model::domain::completed::CompletedBinding {
                boundary: None,
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        for ((boundary, _), source) in self.frozen_shared.lock().map_err(|_| poisoned())?.iter() {
            expected_bindings.push(lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        let binding_key = |binding: &lctx_model::domain::completed::CompletedBinding| {
            (binding.boundary.clone(), binding.view.relation.clone())
        };
        expected_bindings.sort_by_key(binding_key);
        let mut actual_bindings = inventory.bindings;
        actual_bindings.sort_by_key(binding_key);
        if actual_bindings != expected_bindings {
            return Err(ModelError::Conflict(
                "frozen native completed binding handoff",
            ));
        }
        if let Some((captures, frontier, profile, settings)) = request {
            self.require_compilation(captures, frontier, profile, settings)?;
        }
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
    fn coverage_rows<R: Record>(
        &self,
        source: &CompletedRelation,
    ) -> Result<(Vec<R>, StateCharge), ModelError> {
        let mut rows = Vec::new();
        let mut charge = StateCharge::new(&self.budget, "facts-coverage-input");
        for batch in source.read::<R>(self.model.clone(), self.budget.clone())? {
            for row in batch?.rows() {
                charge.grow(row.row_bytes().saturating_add(size_of::<R>()))?;
                rows.push(row.clone());
            }
        }
        Ok((rows, charge))
    }
    async fn coverage_rows_async<R: Record>(
        &self,
        source: &CompletedRelation,
    ) -> Result<(Vec<R>, StateCharge), ModelError> {
        let mut stream = self
            .native
            .scan_batches(
                source.view(),
                &Relation::of::<R>(),
                None,
                None,
                &self.budget,
                self.options.batch_rows,
            )
            .await?;
        let mut rows = Vec::new();
        let mut charge = StateCharge::new(&self.budget, "facts-coverage-input");
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            self.cancellation.check()?;
            let batch = Batch::<R>::read(&self.model, &batch, &self.budget)?;
            for row in batch.rows() {
                charge.grow(row.row_bytes() + size_of::<R>())?;
                rows.push(row.clone());
            }
        }
        Ok((rows, charge))
    }
    fn facts_descriptor_charge(
        &self,
        rows: &[lctx_model::domain::completed::CompletedContribution],
    ) -> Result<StateCharge, ModelError> {
        use lctx_model::domain::HeapSize;
        let mut charge = StateCharge::new(&self.budget, "facts-coverage-descriptors");
        for row in rows {
            charge.grow(
                size_of::<lctx_model::domain::completed::CompletedContribution>()
                    + row.spec.producer.len()
                    + row
                        .spec
                        .captured_binding
                        .as_ref()
                        .map_or(0, |binding| binding.heap_bytes())
                    + row
                        .spec
                        .inputs
                        .iter()
                        .map(|input| {
                            size_of::<lctx_model::domain::analysis::sources::SourceSnapshot>()
                                + input.heap_bytes()
                        })
                        .sum::<usize>()
                    + row
                        .spec
                        .outputs
                        .iter()
                        .map(|name| name.len() + size_of::<String>() + 32)
                        .sum::<usize>()
                    + row
                        .outputs
                        .keys()
                        .map(|name| {
                            name.len()
                                + size_of::<String>()
                                + size_of::<lctx_model::domain::completed::OutputContent>()
                                + 32
                        })
                        .sum::<usize>(),
            )?;
        }
        Ok(charge)
    }
    fn availability_from_rows(
        &self,
        profile: Profile,
        scope_rows: Vec<lctx_model::domain::source::CoverageScope>,
        facts: crate::facts::RecordedFacts<'_>,
    ) -> Result<AdmittedFacts, ModelError> {
        let rows = facts.observed;
        let recorded = crate::facts::recorded_coverage(&self.model, profile, &facts, &self.budget)?;
        let scopes = scope_rows.into_iter().map(|row| (row.id(), row)).collect();
        let availability = lctx_model::domain::admission::ScopedAvailability::from_completed(
            profile,
            &recorded.expected,
            rows,
            &scopes,
            &self.budget,
        )?;
        Ok(AdmittedFacts {
            contract: recorded.contract,
            availability: Arc::new(availability),
            reporting: recorded.reporting,
            _reporting_charge: recorded.charge,
        })
    }
    fn facts_availability_sources(
        &self,
        profile: Profile,
    ) -> Result<(FactsAvailabilityKey, [Arc<CompletedRelation>; 9]), ModelError> {
        use lctx_model::domain::{
            attribution::{AnalysisContext, Provider, ProviderCoverage, ProviderRun, RunFamily},
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
        };
        self.cancellation.check()?;
        let completed = self.completed.lock().map_err(|_| poisoned())?;
        let bind = |name| {
            completed
                .get(name)
                .cloned()
                .ok_or_else(|| ModelError::Invalid(format!("input {name} is not completed")))
        };
        let sources = [
            bind(InputRevision::NAME)?,
            bind(SourceArtifact::NAME)?,
            bind(ArtifactUse::NAME)?,
            bind(CoverageScope::NAME)?,
            bind(ProviderCoverage::NAME)?,
            bind(Provider::NAME)?,
            bind(ProviderRun::NAME)?,
            bind(RunFamily::NAME)?,
            bind(AnalysisContext::NAME)?,
        ];
        let key = FactsAvailabilityKey {
            profile,
            views: sources.each_ref().map(|source| source.view_identity()),
        };
        Ok((key, sources))
    }
    fn cached_facts_availability(
        &self,
        key: &FactsAvailabilityKey,
    ) -> Result<Option<Arc<AdmittedFacts>>, ModelError> {
        Ok(self
            .facts_availability
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .filter(|(known, _)| known == key)
            .map(|(_, value)| value.clone()))
    }
    fn remember_facts_availability(
        &self,
        key: FactsAvailabilityKey,
        value: AdmittedFacts,
    ) -> Result<Arc<AdmittedFacts>, ModelError> {
        let mut admitted = self.facts_availability.lock().map_err(|_| poisoned())?;
        if let Some((known, value)) = &*admitted
            && *known == key
        {
            return Ok(value.clone());
        }
        let value = Arc::new(value);
        *admitted = Some((key, value.clone()));
        Ok(value)
    }
    pub fn facts_availability(
        &self,
        profile: Profile,
    ) -> Result<Arc<lctx_model::domain::admission::ScopedAvailability>, ModelError> {
        use lctx_model::domain::{
            attribution::{AnalysisContext, Provider, ProviderCoverage, ProviderRun, RunFamily},
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
        };
        let (key, sources) = self.facts_availability_sources(profile)?;
        if let Some(value) = self.cached_facts_availability(&key)? {
            return Ok(value.availability.clone());
        }
        let bound_sources = sources
            .iter()
            .map(|source| (source.name().to_owned(), source.snapshot().clone()))
            .collect();
        let authority = sources
            .iter()
            .flat_map(|source| source.view().contributions.iter().copied())
            .collect();
        let [
            input_source,
            artifact_source,
            use_source,
            scope_source,
            coverage_source,
            provider_source,
            run_source,
            family_source,
            context_source,
        ] = sources;
        let (inputs, _inputs) = self.coverage_rows::<InputRevision>(&input_source)?;
        let (artifacts, _artifacts) = self.coverage_rows::<SourceArtifact>(&artifact_source)?;
        let (uses, _uses) = self.coverage_rows::<ArtifactUse>(&use_source)?;
        let (scopes, _scopes) = self.coverage_rows::<CoverageScope>(&scope_source)?;
        let (rows, _rows) = self.coverage_rows::<ProviderCoverage>(&coverage_source)?;
        let (providers, _providers) = self.coverage_rows::<Provider>(&provider_source)?;
        let (runs, _runs) = self.coverage_rows::<ProviderRun>(&run_source)?;
        let (families, _families) = self.coverage_rows::<RunFamily>(&family_source)?;
        let (contexts, _contexts) = self.coverage_rows::<AnalysisContext>(&context_source)?;
        let native = self.native.clone();
        let calls = self.native_calls.clone();
        let contributions = self.bridge.call(async move {
            calls
                .call(async move { native.contributions().await })
                .await
        })?;
        let _descriptors = self.facts_descriptor_charge(&contributions)?;
        let value = self.availability_from_rows(
            profile,
            scopes,
            crate::facts::RecordedFacts {
                inputs: &inputs,
                artifacts: &artifacts,
                uses: &uses,
                observed: &rows,
                recorded: &providers,
                runs: &runs,
                families: &families,
                contexts: &contexts,
                contributions: &contributions,
                authority: &authority,
                bound_sources: &bound_sources,
            },
        )?;
        Ok(self
            .remember_facts_availability(key, value)?
            .availability
            .clone())
    }
    pub async fn facts_availability_async(
        &self,
        profile: Profile,
    ) -> Result<Arc<lctx_model::domain::admission::ScopedAvailability>, ModelError> {
        Ok(self
            .admitted_facts_async(profile)
            .await?
            .availability
            .clone())
    }
    pub(crate) async fn admitted_facts_async(
        &self,
        profile: Profile,
    ) -> Result<Arc<AdmittedFacts>, ModelError> {
        use lctx_model::domain::{
            attribution::{AnalysisContext, Provider, ProviderCoverage, ProviderRun, RunFamily},
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
        };
        let (key, sources) = self.facts_availability_sources(profile)?;
        if let Some(value) = self.cached_facts_availability(&key)? {
            return Ok(value);
        }
        let bound_sources = sources
            .iter()
            .map(|source| (source.name().to_owned(), source.snapshot().clone()))
            .collect();
        let authority = sources
            .iter()
            .flat_map(|source| source.view().contributions.iter().copied())
            .collect();
        let [
            input_source,
            artifact_source,
            use_source,
            scope_source,
            coverage_source,
            provider_source,
            run_source,
            family_source,
            context_source,
        ] = sources;
        let (inputs, _inputs) = self
            .coverage_rows_async::<InputRevision>(&input_source)
            .await?;
        let (artifacts, _artifacts) = self
            .coverage_rows_async::<SourceArtifact>(&artifact_source)
            .await?;
        let (uses, _uses) = self.coverage_rows_async::<ArtifactUse>(&use_source).await?;
        let (scopes, _scopes) = self
            .coverage_rows_async::<CoverageScope>(&scope_source)
            .await?;
        let (rows, _rows) = self
            .coverage_rows_async::<ProviderCoverage>(&coverage_source)
            .await?;
        let (providers, _providers) = self
            .coverage_rows_async::<Provider>(&provider_source)
            .await?;
        let (runs, _runs) = self.coverage_rows_async::<ProviderRun>(&run_source).await?;
        let (families, _families) = self
            .coverage_rows_async::<RunFamily>(&family_source)
            .await?;
        let (contexts, _contexts) = self
            .coverage_rows_async::<AnalysisContext>(&context_source)
            .await?;
        let native = self.native.clone();
        let contributions = self
            .native_calls
            .call(async move { native.contributions().await })
            .await?;
        let _descriptors = self.facts_descriptor_charge(&contributions)?;
        let value = self.availability_from_rows(
            profile,
            scopes,
            crate::facts::RecordedFacts {
                inputs: &inputs,
                artifacts: &artifacts,
                uses: &uses,
                observed: &rows,
                recorded: &providers,
                runs: &runs,
                families: &families,
                contexts: &contexts,
                contributions: &contributions,
                authority: &authority,
                bound_sources: &bound_sources,
            },
        )?;
        self.remember_facts_availability(key, value)
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
        self.bind_admission_epochs(&mut inputs)?;
        self.validate_scope(profile, true, Some(&inputs)).await?;
        Ok(CheckedInputs {
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
            for declaration in self.model.invariants().iter().flat_map(|invariant| &invariant.inputs) {
                if declaration.prefix().is_none() || inputs.relations.contains_key(&(declaration.name(), declaration.prefix())) { continue; }
                if let Some(source) = inputs.relations.get(&(declaration.name(), None)).map(|binding| binding.source.clone()) {
                    let role = inputs.relations.len();
                    inputs.relations.insert((declaration.name(), declaration.prefix()), ResolvedInput { role, requested: declaration.prefix(), resolved: None, source });
                }
            }
        } else {
            for ((boundary, name), source) in frozen.iter() {
                let role = inputs.relations.len();
                inputs.relations.insert((*name, Some(*boundary)), ResolvedInput { role, requested: Some(*boundary), resolved: Some(*boundary), source: source.clone() });
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
        let relations = match selected {
            Some(inputs) => inputs.relations().cloned().collect(),
            None => self.completed_relations()?,
        };
        let names = relations
            .iter()
            .map(|r| r.name())
            .collect::<std::collections::BTreeSet<_>>();
        // A checked dependency closure retains its selected publication epochs.
        // Reconstructing this set by name would silently substitute the latest stream.
        let mut inputs = match selected {
            Some(inputs) => inputs.clone(),
            None => self.inputs("artifact-admission", profile, names.iter().copied())?,
        };
        if selected.is_none() { self.bind_admission_epochs(&mut inputs)?; }
        // Provider registration, native selection and memoization resolve this one frozen
        // inventory even for whole-artifact admission, never a later live workspace prefix.
        let selected = Some(&inputs);
        let session = inputs.session(self).await?;
        self.validate_references(&session, &relations, &inputs)
            .await?;
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
                // Only synthesize profile premises within this workspace's supplied model.
                .filter(|relation| self.model.relation(relation.name()).is_some())
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
                && self
                    .checked_premises
                    .lock()
                    .map_err(|_| poisoned())?
                    .0
                    .contains(&premise)
            {
                continue;
            }
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
                            &self.model,
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
                            &self.model,
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
                            &self.model,
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
                            &self.model,
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
                    &self.model,
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
                    &self.model,
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
                let mut stream = inputs.query_template(&session, input, &sql)
                    .await?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish().map_err(|error| match error {
                ModelError::Invalid(message) => ModelError::Invalid(format!(
                    "{} [{}]: {message}",
                    invariant.name,
                    profile.name()
                )),
                other => other,
            })?;
            self.remember_premise(premise)?;
        }
        self.identity()
    }
    fn validation_premise(
        &self,
        invariant: &lctx_model::domain::Invariant,
        selected: Option<&CompletedInputs>,
        unrequested: &std::collections::BTreeSet<&'static str>,
        profile: Profile,
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
                selected.validation_source(input).ok().map(|binding| {
                    sink.part(b"requested-selector", binding.requested.map(|p| p.name()).unwrap_or("default").as_bytes());
                    sink.part(b"resolved-selector", binding.resolved.map(|p| p.name()).unwrap_or("default").as_bytes());
                    binding.source.clone()
                })
            } else {
                self.input_relation(input).ok()
            };
            let Some(relation) = relation else {
                return Ok(None);
            };
            relation.view.identity.encode(&mut sink);
            relation.snapshot.identity().encode(&mut sink);
        }
        Ok(Some(sink.finish()))
    }
    fn remember_premise(&self, premise: Option<ContentHash>) -> Result<(), ModelError> {
        if let Some(premise) = premise {
            let mut checked = self.checked_premises.lock().map_err(|_| poisoned())?;
            if !checked.0.contains(&premise) {
                checked.1.grow(128)?;
                checked.0.insert(premise);
            }
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
            session
                .register_table(
                    &table,
                    self.native.table_provider(
                        &source.view,
                        source.relation.clone(),
                        self.budget.clone(),
                        self.options.batch_rows,
                    )?,
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
    async fn validate_references(
        &self,
        session: &SessionContext,
        relations: &[Arc<CompletedRelation>],
        inputs: &CompletedInputs,
    ) -> Result<(), ModelError> {
        let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
        for source in relations {
            for field in source
                .relation
                .fields()
                .iter()
                .filter(|field| !field.list())
            {
                let Some((_, target)) = field.target() else {
                    continue;
                };
                let targets = distinct_reference_views(inputs
                    .relations
                    .iter()
                    .filter(|((name, _), _)| *name == target)
                    .collect(), |(_, source)| &source.view)?;
                let target_source = targets.first().map(|(_, source)| *source);
                let mut key = lctx_model::domain::KeySink::new("compiler-reference-premise/v1");
                key.part(b"model", &self.model.digest().0);
                key.part(b"source", &source.view_identity().0);
                key.part(b"field", field.name().as_bytes());
                for (_, source) in &targets {
                    key.part(b"target", &source.view_identity().0);
                }
                if targets.is_empty() {
                    key.part(b"missing-target", target.as_bytes());
                }
                let premise = key.finish();
                if self
                    .checked_premises
                    .lock()
                    .map_err(|_| poisoned())?
                    .0
                    .contains(&premise)
                {
                    continue;
                }
                let source_key = inputs
                    .relations
                    .iter()
                    .find(|(_, bound)| bound.view == source.view)
                    .map(|(key, _)| key)
                    .ok_or(ModelError::Schema("selected reference source binding"))?;
                let source_name = quote(&CompletedInputs::table(source_key.0, source_key.1));
                let field_name = quote(field.name());
                let sql = if let Some(target_source) = target_source {
                    let subtype = if let Some(tag) = field.subtype() {
                        let tag_field = target_source
                            .relation
                            .sum()
                            .ok_or(ModelError::Schema("nominal subtype target"))?
                            .tag;
                        format!(
                            " OR t.{} IS NULL OR t.{}<>{tag}",
                            quote(tag_field),
                            quote(tag_field)
                        )
                    } else {
                        String::new()
                    };
                    let target_tables = targets
                        .iter()
                        .map(|((name, prefix), _)| {
                            format!(
                                "SELECT * FROM {}",
                                quote(&CompletedInputs::table(name, *prefix))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" UNION ALL ");
                    format!(
                        "SELECT s.{field_name} FROM {source_name} s LEFT JOIN ({target_tables}) t ON s.{field_name}=t.id WHERE s.{field_name} IS NOT NULL AND (t.id IS NULL{subtype}) LIMIT 1"
                    )
                } else {
                    format!(
                        "SELECT {field_name} FROM {source_name} WHERE {field_name} IS NOT NULL LIMIT 1"
                    )
                };
                let mut stream = crate::sql::query(session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    if batch.num_rows() > 0 {
                        return Err(ModelError::Invalid(format!(
                            "missing or wrong-subtype nominal reference {}.{} -> {target}",
                            source.name(),
                            field.name()
                        )));
                    }
                }
                self.remember_premise(Some(premise))?;
            }
        }
        Ok(())
    }
    pub(crate) fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        self.native_calls.call_boxed(future.boxed())
    }
    pub fn native(&self) -> &Arc<lctx_surrealdb::compiler::NativeCompilerStore> {
        &self.native
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
    /// Drain abandoned provider threads before discarding attempt-owned native state.
    pub async fn drain_report(&self) -> lctx_model::domain::completion::Completion {
        use lctx_model::domain::completion::{Completion, LocalState};
        self.cancellation.cancel();
        let mut completion = Completion::default();
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
        completion.step("provider bridge drain", self.bridge.drain().await);
        completion.step("native calls drain", self.native_calls.drain().await);
        completion.step(
            "native store drain",
            lctx_model::domain::completion::complete(Ok(()), self.native.drain_report().await),
        );
        completion
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
            relations.insert((name, None), ResolvedInput {
                role,
                requested: None,
                resolved: None,
                source: self.relation(name)?,
            });
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
    /// Bind each declared semantic boundary to immutable shared streams. Only small
    /// descriptors are retained; later contributions cannot widen an earlier producer's inputs.
    pub async fn freeze_inputs_async(
        &self,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        let _completion = self.completion_gate.lock().await;
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
            let native = self.native.clone();
            let binding = lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            };
            self.native_calls
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
            let native = self.native.clone();
            let binding = lctx_model::domain::completed::CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            };
            let calls = self.native_calls.clone();
            self.bridge
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
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let mut relations = BTreeMap::new();
        for (role, (declared, input)) in declaration.inputs.iter().zip(&selected.inputs).enumerate() {
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
                .insert((declared.name(), declared.prefix()), ResolvedInput {
                    role,
                    requested: declared.prefix(),
                    resolved: input.prefix(),
                    source,
                })
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
            failed: AtomicBool::new(false),
            product_capture: Mutex::default(),
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
    pub fn schema(&self) -> &Arc<arrow_schema::Schema> { self.relation.schema() }
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
    pub fn batches(&self) -> Result<NativeBatches, ModelError> { self.batches_scoped(None) }
    fn batches_scoped(&self, scope: Option<lctx_surrealdb::compiler::ProducingScope>) -> Result<NativeBatches, ModelError> {
        let (driver, reader) = self.prepare_batches(scope);
        self.bridge.launch(driver)?;
        Ok(reader)
    }
    pub async fn batches_async(&self) -> Result<NativeBatches, ModelError> {
        let (driver, reader) = self.prepare_batches(None);
        self.bridge.launch_async(driver).await?;
        Ok(reader)
    }
    fn prepare_batches(&self, scope: Option<lctx_surrealdb::compiler::ProducingScope>) -> (BoxFuture<'static, ()>, NativeBatches) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let native = self.native.clone();
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
            let result = match scope { Some(scope) => scope.run(read).await, None => read.await };
            if let Err(error) = result {
                tokio::select! {
                    ()=cancellation.cancelled()=>{},
                    _=sender.send(Err(error))=>{},
                }
            }
        });
        let reader = NativeBatches {
            receiver,
            cancellation: self.cancellation.clone(),
            charge: None,
            done: false,
        };
        (driver, reader)
    }
    pub async fn read_async<R: Record>(&self, model: Arc<ValidatedModel>, budget: ResourceBudget) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        Ok(TypedBatches { reader: self.batches_async().await?, model, budget, marker: Default::default(), _files: self._files.clone() })
    }
    pub fn read<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<TypedBatches<R>, ModelError> {
        self.read_scoped(model, budget, None)
    }
    fn read_scoped<R: Record>(&self, model: Arc<ValidatedModel>, budget: ResourceBudget, scope: Option<lctx_surrealdb::compiler::ProducingScope>) -> Result<TypedBatches<R>, ModelError> {
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
    receiver: tokio::sync::mpsc::Receiver<ChargedBatch>,
    cancellation: Cancellation,
    charge: Option<Box<dyn Reservation>>,
    done: bool,
}
impl NativeBatches {
    /// Async compiler callers await the same handoff that wakes provider threads.
    pub async fn next_async(&mut self) -> Option<Result<RecordBatch, ModelError>> {
        if self.done { return None; }
        self.charge = None;
        let item = tokio::select! {
            biased;
            () = self.cancellation.cancelled() => Some(Err(ModelError::Invalid("compilation cancelled".into()))),
            item = self.receiver.recv() => item,
        };
        match item {
            Some(Ok((batch, charge))) => { self.charge = Some(charge); Some(Ok(batch)) },
            Some(Err(error)) => { self.receiver.close(); self.done = true; Some(Err(error)) },
            None => { self.done = true; None },
        }
    }
}
impl Iterator for NativeBatches {
    type Item = Result<RecordBatch, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        // Genuine synchronous provider callbacks run on their existing blocking owner. Tokio
        // compiler code uses next_async; no sleeping poll loop or blocking_recv is required.
        futures::executor::block_on(self.next_async())
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
        self.reader.next_async().await.map(|batch| batch.and_then(|batch| Batch::read(&self.model, &batch, &self.budget)))
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
                    .is_none_or(|other| source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source))
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
    producing_scope: Option<lctx_surrealdb::compiler::ProducingScope>,
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
    aliases: BTreeMap<String, Arc<dyn datafusion::catalog::TableProvider>>,
    templates: Mutex<(BTreeMap<ContentHash, Arc<PreparedLogicalTemplate>>, StateCharge)>,
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
// Aliases preserve semantic roles, but identical exact views need only one physical branch.
// Keep descriptor comparison as well as identity comparison: a collision is not authority.
fn distinct_reference_views<T>(
    mut targets: Vec<T>,
    view: impl Fn(&T) -> &lctx_model::domain::completed::CompletedView,
) -> Result<Vec<T>, ModelError> {
    targets.sort_unstable_by_key(|target| view(target).identity);
    for adjacent in targets.windows(2) {
        if view(&adjacent[0]).identity == view(&adjacent[1]).identity && view(&adjacent[0]) != view(&adjacent[1]) {
            return Err(ModelError::Conflict("reference target view descriptor collision"));
        }
    }
    targets.dedup_by_key(|target| view(target).identity);
    Ok(targets)
}
impl PreparedInputSession {
    fn prepare_template(&self, plan: &datafusion::logical_expr::LogicalPlan, budget: &ResourceBudget, sql_bytes: usize)
        -> Result<(bool, PreparedLogicalTemplate), ModelError> {
        use datafusion::common::tree_node::{TreeNode, TreeNodeRecursion};
        let mut charge = StateCharge::new(budget, "completed-input-logical-preparation");
        charge.grow(sql_bytes.saturating_mul(32).saturating_add(4096))?;
        let mut compatible = true;
        let mut ports = BTreeMap::new();
        plan.apply_with_subqueries(|node| {
            // Reserve traversal/expression and eventual cloned-plan allowance before growth.
            charge.grow(2048).map_err(|error| datafusion::error::DataFusionError::External(Box::new(error)))?;
            if let datafusion::logical_expr::LogicalPlan::TableScan(scan) = node {
                if let Some(canonical) = self.aliases.get(scan.table_name.table()) {
                    let captured = datafusion::datasource::source_as_provider(&scan.source)?;
                    if !Arc::ptr_eq(canonical, &captured) {
                        return Err(datafusion::error::DataFusionError::External(Box::new(
                            ModelError::Conflict("prepared completed input provider replaced"))));
                    }
                    if !ports.contains_key(&scan.table_name) {
                        let reference_bytes = scan.table_name.table().len()
                            .saturating_add(scan.table_name.schema().map_or(0, str::len))
                            .saturating_add(scan.table_name.catalog().map_or(0, str::len));
                        // Map and final vector coexist during collection. Reserve both.
                        charge.grow(size_of::<PreparedTemplatePort>().saturating_mul(2)
                            .saturating_add(reference_bytes.saturating_mul(4)).saturating_add(256))
                            .map_err(|error| datafusion::error::DataFusionError::External(Box::new(error)))?;
                        ports.insert(scan.table_name.clone(), captured);
                    }
                } else {
                    compatible = false;
                }
            }
            for expression in node.expressions() {
                expression.apply(|expression| {
                    if matches!(expression, datafusion::logical_expr::Expr::Literal(..)) { compatible = false; }
                    Ok(TreeNodeRecursion::Continue)
                })?;
            }
            Ok(TreeNodeRecursion::Continue)
        }).map_err(crate::sql::model_error)?;
        let ports = ports.into_iter().map(|(reference, provider)| PreparedTemplatePort { reference, provider }).collect();
        Ok((compatible, PreparedLogicalTemplate { plan: plan.clone(), ports, _charge: charge }))
    }
}
impl PreparedLogicalTemplate {
    async fn verify_ports(&self, session: &SessionContext) -> Result<(), ModelError> {
        for port in &self.ports {
            let current = session.table_provider(port.reference.clone()).await.map_err(crate::sql::model_error)?;
            if !Arc::ptr_eq(&port.provider, &current) {
                return Err(ModelError::Conflict("prepared completed input provider replaced"));
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
    fn deref(&self) -> &Self::Target { &self.source }
}
impl CompletedInputs {
    fn ordered_bindings(&self) -> Vec<((&'static str, Option<lctx_model::domain::stages::PublicationBoundary>), &ResolvedInput)> {
        let mut bindings = self.relations.iter().map(|(key, value)| (*key, value)).collect::<Vec<_>>();
        bindings.sort_by_key(|(_, binding)| binding.role);
        bindings
    }
    fn validation_source(&self, input: &lctx_model::domain::ValidationInput) -> Result<&ResolvedInput, ModelError> {
        if let Some(source) = self.relations.get(&(input.name(), input.prefix())) { return Ok(source); }
        if input.prefix().is_some() { return Err(ModelError::Conflict("checked admission requires a missing input epoch")); }
        let mut choices = self.relations.iter().filter(|((name, _), _)| *name == input.name()).map(|(_, source)| source);
        let source = choices.next().ok_or(ModelError::Conflict("checked admission input is absent"))?;
        if choices.any(|other| other.resolved != source.resolved || !Arc::ptr_eq(&source.source, &other.source)) {
            return Err(ModelError::Conflict("checked admission input epoch is ambiguous"));
        }
        Ok(source)
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
                    || consumer
                        .relations
                        .get(key)
                        .is_none_or(|other| source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source))
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
        if choices.any(|(_, other)| source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source)) {
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
                if choices.any(|(_, other)| source.resolved != other.resolved || !Arc::ptr_eq(&source.source, &other.source)) {
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
        if sources.any(|other| other.resolved != source.resolved || other.view.identity != source.view.identity) {
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
        self.ordered_bindings().into_iter().map(|(_, source)| source.snapshot())
    }
    pub fn relations(&self) -> impl Iterator<Item = &Arc<CompletedRelation>> {
        self.relations.values().map(|binding| &binding.source)
    }
    fn provider(&self, workspace: &Workspace, source: &CompletedRelation) -> Result<Arc<dyn datafusion::catalog::TableProvider>, ModelError> {
        let provider = workspace.native.table_provider(&source.view, source.relation.clone(), workspace.budget.clone(), workspace.options.batch_rows)?;
        match &self.producing_scope {
            Some(scope) => lctx_surrealdb::compiler_provider::bind_producing(&provider, scope),
            None => Ok(provider),
        }
    }
    pub async fn session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        // The immutable inventory owns provider registration, so general compatible checks
        // reuse it as well. Selecting ports or changing producing scope creates a fresh cell.
        if !Arc::ptr_eq(&self.attempt, &workspace.files) || self.model != workspace.model.digest()
            || !self.budget.shares_pool(workspace.budget())
            || self.relations.values().any(|binding| !Arc::ptr_eq(&binding.source._files, &workspace.files)) {
            return Err(ModelError::Conflict("completed input session foreign attempt"));
        }
        self.prepared_session.get_or_try_init(|| self.prepare_session(workspace)).await.map(|prepared| prepared.context.clone())
    }
    /// Candidate providers are mutable selected ports. Borrow the prepared base providers
    /// into a private catalog so their arrays and charges end with the semantic predicate.
    pub(crate) async fn predicate_session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        let base = self.session(workspace).await?;
        let prepared = self.prepared_session.get().expect("prepared immutable input session");
        let context = SessionContext::new_with_config_rt(base.copied_config(), base.runtime_env());
        for (alias, provider) in &prepared.aliases {
            context.register_table(alias, provider.clone()).map_err(ModelError::codec)?;
        }
        Ok(context)
    }
    async fn prepare_session(&self, workspace: &Workspace) -> Result<PreparedInputSession, ModelError> {
        if !Arc::ptr_eq(&self.attempt, &workspace.files) || self.model != workspace.model.digest()
            || !self.budget.shares_pool(workspace.budget())
            || self.relations.values().any(|binding| !Arc::ptr_eq(&binding.source._files, &workspace.files)) {
            return Err(ModelError::Conflict("completed input session foreign attempt"));
        }
        let context = SessionContext::new_with_config_rt(
            workspace.context.copied_config(),
            workspace.context.runtime_env(),
        );
        let mut aliases = BTreeMap::new();
        for ((name, prefix), source) in self.ordered_bindings() {
            let table = Self::table(name, prefix);
            let provider = self.provider(workspace, source)?;
            aliases.insert(table.clone(), provider.clone());
            context
                .register_table(
                    &table,
                    provider,
                )
                .map_err(ModelError::codec)?;
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
            if sources.all(|other| other.resolved == source.resolved && other.view.identity == source.view.identity) {
                let provider = self.provider(workspace, source)?;
                aliases.insert(name.to_owned(), provider.clone());
                context
                    .register_table(
                        name,
                        provider,
                    )
                    .map_err(ModelError::codec)?;
            }
        }
        Ok(PreparedInputSession { context, aliases, templates: Mutex::new((BTreeMap::new(), StateCharge::new(&self.budget, "completed-input-logical-templates"))) })
    }
    /// Read-only logical preparation is bound to these exact captured ports and operational
    /// read scope. Physical plans, streams and predicate state remain fresh per request.
    pub(crate) async fn query_template(&self, session: &SessionContext, input: &lctx_model::domain::ValidationInput, sql: &str)
        -> Result<datafusion::dataframe::DataFrame, ModelError> {
        use lctx_model::domain::Key;
        let Some(prepared) = self.prepared_session.get().filter(|prepared| prepared.context.session_id() == session.session_id()) else {
            return crate::sql::query(session, sql).await.map_err(crate::sql::model_error);
        };
        let mut key = lctx_model::domain::KeySink::new("completed-input-logical-template/v1");
        self.model.encode(&mut key);
        self.program.encode(&mut key);
        self.profile.name().to_owned().encode(&mut key);
        self.name.to_owned().encode(&mut key);
        input.encode_contract(&mut key);
        sql.to_owned().encode(&mut key);
        let key = key.finish();
        let cached = { prepared.templates.lock().map_err(|_| poisoned())?.0.get(&key).cloned() };
        if let Some(template) = cached {
            // Only ports captured by this plan participate. An unrelated catalog mutation
            // neither invalidates this template nor adds work to every cache hit.
            template.verify_ports(session).await?;
            return Ok(datafusion::dataframe::DataFrame::new(session.state(), template.plan.clone()));
        }
        let frame = crate::sql::query(session, sql).await.map_err(crate::sql::model_error)?;
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
                templates.1.grow(size_of::<ContentHash>() + size_of::<Arc<PreparedLogicalTemplate>>() + 128)?;
                templates.0.insert(key, Arc::new(template));
            }
        }
        Ok(frame)
    }
}

struct PendingRelation {
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
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
    producer: ContentHash,
    batch: Batch<R>,
    scope: Option<lctx_surrealdb::compiler::ProducingScope>,
) -> BatchWrite {
    let relation = Relation::of::<R>();
    async move {
        let write = native.write_batch(&producer, &relation, batch.arrow());
        match scope { Some(scope) => scope.run(write).await, None => write.await }
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
    producing_scope: Option<lctx_surrealdb::compiler::ProducingScope>,
    native_calls: Arc<crate::native_calls::NativeCalls>,
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
    fn write_batch(&mut self, batch: Batch<R>) -> Result<(), ModelError> {
        let write = owned_batch_write(self.native.clone(), self.producer, batch, self.producing_scope.clone());
        self.bridge
            .call_boxed(submit_batch_write(self.native_calls.clone(), write, None))
    }
    fn prepare_close(
        mut self: Box<Self>,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<PreparedClose, ModelError> {
        let pending = PendingRelation {
            relation: Relation::of::<R>(),
            contribution: self.contribution,
        };
        let write = if self.pending.is_empty() {
            None
        } else {
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(&model, std::mem::take(&mut self.pending), charge)?;
            Some(owned_batch_write(self.native.clone(), self.producer, batch, self.producing_scope.clone()))
        };
        Ok(PreparedClose {
            pending,
            write,
            calls: self.native_calls.clone(),
            keep_alive: self,
        })
    }
    fn flush(&mut self, model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.pending.is_empty() {
            // Transfer the existing row reservation into encoding; these are the same rows,
            // rather than an additional resident copy of the pending batch.
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(model, std::mem::take(&mut self.pending), charge)?;
            self.write_batch(batch)?;
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
    producing_scope: Mutex<Option<lctx_surrealdb::compiler::ProducingScope>>,
    failed: AtomicBool,
    product_capture: Mutex<Option<lctx_model::domain::compilation_product::ProductRequest>>,
}
impl ProducerOutput {
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    fn producing_scope(&self) -> Result<lctx_surrealdb::compiler::ProducingScope, ModelError> {
        let mut scope = self.producing_scope.lock().map_err(|_| poisoned())?;
        if let Some(scope) = scope.as_ref() { return Ok(scope.clone()); }
        let created = self.workspace.native.producing_scope(self.contribution_spec().identity()?, self.workspace.budget())?;
        *scope = Some(created.clone());
        Ok(created)
    }
    /// Bind explicit read descendants to this producer, retaining semantic input identity.
    pub(crate) fn bound_inputs(&self) -> Result<CompletedInputs, ModelError> {
        let mut inputs = self.inputs.clone();
        inputs.producing_scope = Some(self.producing_scope()?);
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
        lctx_model::domain::completed::ContributionSpec {
            captured_binding: self.captured_binding.clone(),
            producer: self.name.into(),
            profile: self.profile,
            model: self.workspace.model.digest(),
            implementation: self.implementation,
            configuration: self.configuration,
            inputs: self.inputs.snapshots().collect(),
            outputs: self.allowed.iter().map(|name| (*name).to_owned()).collect(),
        }
    }
    fn registration_request(&self) -> BoxFuture<'static, Result<ContentHash, ModelError>> {
        match self.contribution.lock() {
            Ok(contribution) => if let Some(id) = *contribution { return futures::future::ready(Ok(id)).boxed(); },
            Err(_) => return futures::future::ready(Err(poisoned())).boxed(),
        }
        let registration = self.registration.clone();
        let contribution = self.contribution.clone();
        let native = self.workspace.native.clone();
        let calls = self.workspace.native_calls.clone();
        let descriptor = self.contribution_spec();
        let scope = self.producing_scope();
        async move {
            let scope = scope?;
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
        Box::pin(async move {
            let result = async {
                self.check_output::<R>()?;
                let producer = self.registration_request().await?;
                self.insert_writer::<R>(
                    producer,
                    lctx_model::domain::stages::is_epoch_shared(R::NAME),
                )
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
    fn declare_kind<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.guarded(|| {
            self.check_output::<R>()?;
            let producer = self.workspace.bridge.call(self.registration_request())?;
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
                producing_scope: Some(self.producing_scope()?),
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
        let result = writer.write_batch(batch);
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
    pub async fn push<R: Record>(&self, row: R) -> Result<(), ModelError> {
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
                let write = owned_batch_write(self.workspace.native.clone(), producer, batch, Some(self.producing_scope()?));
                submit_batch_write(self.workspace.native_calls.clone(), write, None).await?;
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
            self.mark_finished(outcome)?;
            self.complete().await
        })
    }
    pub fn complete(self) -> BoxFuture<'static, Result<(), ModelError>> {
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
            let product_request = self.product_capture.lock().map_err(|_|poisoned())?.take();
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
                            relation,
                            contribution: true,
                        });
                    }
                }
            }
            let id = registration.await?;
            producing_scope.close_and_wait().await?;
            let _completion = self.workspace.completion_gate.lock().await;
            self.workspace.writable()?;
            let previous = self
                .workspace
                .completed
                .lock()
                .map_err(|_| poisoned())?
                .iter()
                .map(|(name, source)| ((*name).to_owned(), source.view.clone()))
                .collect();
            for source in &pending {
                if let Ok(previous) = self.workspace.relation(source.relation.name())
                    && !source.contribution
                    && !previous.contribution
                {
                    return Err(ModelError::Invalid(format!(
                        "completed output {} already has an owner",
                        source.relation.name()
                    )));
                }
            }
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
            let native = self.workspace.native.clone();
            let views = self
                .workspace
                .native_calls
                .call(async move {
                    native
                        .complete_contribution_scoped(&producing_scope, id, outcome, &outputs, &previous)
                        .await
                })
                .await?;
            let native = self.workspace.native.clone();
            let descriptor = self
                .workspace
                .native_calls
                .call(async move { native.completed_contribution(id).await })
                .await?;
            if descriptor.spec != spec || descriptor.outcome != expected_outcome {
                return Err(ModelError::Conflict("completed native producer handoff"));
            }
            if let Some(request) = product_request {
                self.workspace.retain_product(request,id,outcome).await?;
            }
            let descriptor_bytes = serde_json::to_vec(&descriptor)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(4)
                .saturating_add(256);
            let mut completed = Vec::new();
            for pending in pending {
                let name = pending.relation.name();
                let view = views
                    .get(name)
                    .ok_or(ModelError::Schema("completed native output absent"))?
                    .clone();
                completed.push(Arc::new(CompletedRelation {
                    snapshot:
                        lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                            &pending.relation,
                            self.workspace.model.digest(),
                            &view,
                        )?,
                    relation: pending.relation,
                    producer: self.name.into(),
                    implementation: self.implementation,
                    configuration: self.configuration,
                    contract: self.workspace.model.digest(),
                    rows: view.rows,
                    view,
                    native: self.workspace.native.clone(),
                    bridge: self.workspace.bridge.clone(),
                    budget: self.workspace.budget.clone(),
                    batch_rows: self.workspace.options.batch_rows,
                    cancellation: self.workspace.cancellation.clone(),
                    inputs: input_snapshots.clone(),
                    profile: self.profile,
                    contribution: pending.contribution,
                    _files: self.workspace.files.clone(),
                }));
            }
            for source in &completed {
                let native = self.workspace.native.clone();
                let binding = lctx_model::domain::completed::CompletedBinding {
                    boundary: None,
                    source: source.snapshot(),
                    view: source.view.clone(),
                    configuration: None,
                };
                self.workspace
                    .native_calls
                    .call(async move { native.bind(binding).await })
                    .await?;
            }
            self.workspace.cancellation.check()?;
            let mut owners = self
                .workspace
                .completed_owners
                .lock()
                .map_err(|_| poisoned())?;
            owners.1.grow(descriptor_bytes)?;
            if owners.0.insert(id, descriptor).is_some() {
                return Err(ModelError::Conflict(
                    "duplicate completed native producer handoff",
                ));
            }
            drop(owners);
            let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
            for source in completed {
                visible.insert(source.name(), source);
            }
            Ok(())
        })
    }
}

#[path = "workspace_products.rs"]
mod products;
pub(crate) use products::{ProductCandidate,decode_product_rows};
#[cfg(test)]
pub(crate) use products::validate_product_rows;

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
        let joined = async move {
            task.await
                .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
        }
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
            self.workspace.model.clone(), self.workspace.budget.clone(), Some(self.producing_scope()?),
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
    #[tokio::test]
    async fn native_transfer_windows_do_not_amplify_small_query_partitions() {
        use datafusion::{common::stats::Precision, physical_plan::ExecutionPlanProperties, prelude::{col, lit}};
        use lctx_model::domain::{completed::CompletedView, input::Release, serving::Name};
        let native = lctx_surrealdb::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_surrealdb::surrealdb::Surreal::init()),
            Name::new("planning").unwrap(), Name::new("planning").unwrap());
        let workspace = Workspace::new(Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()]).unwrap()), WorkspaceOptions {
            memory_bytes: 128 << 20, batch_rows: 7, ..Default::default()
        }, native.clone()).unwrap();
        let config = workspace.context.copied_config();
        assert_eq!(workspace.options.batch_rows, 7);
        assert_eq!(config.batch_size(), SessionConfig::default().batch_size());
        assert_eq!(config.target_partitions(), SessionConfig::default().target_partitions());
        let compute_rows = config.batch_size();
        let large_rows = compute_rows.saturating_mul(128);
        let provider = |rows: usize| {
            let view = CompletedView::new(Release::NAME.into(),
                [ContentHash::of(b"partition-planning")].into(), rows as u64).unwrap();
            lctx_surrealdb::compiler_provider::table_provider(native.clone(), view,
                Relation::of::<Release>(), workspace.budget.clone(), workspace.options.batch_rows).unwrap()
        };
        let tiny = provider(256);
        let large = provider(large_rows);
        let mut charge = StateCharge::new(workspace.budget(), "planning-selected-keys");
        charge.grow(7 * 16).unwrap();
        let selected = lctx_surrealdb::compiler_provider::select_table(&large,
            Arc::new((1..=7).map(|value| [value; 16]).collect()), Arc::new(charge)).unwrap().unwrap();
        assert_eq!(tiny.statistics().unwrap().num_rows, Precision::Exact(256));
        assert_eq!(selected.statistics().unwrap().num_rows, Precision::Inexact(7));
        let empty = lctx_surrealdb::compiler_provider::select_table(&large,
            Arc::new(vec![]), Arc::new(StateCharge::new(workspace.budget(), "planning-empty-keys"))).unwrap().unwrap();
        assert_eq!(empty.statistics().unwrap().num_rows, Precision::Inexact(0));
        for (name, provider) in [("tiny", tiny), ("selected", selected), ("empty", empty), ("large", large)] {
            workspace.context.register_table(name, provider).unwrap();
        }
        fn partitioned(plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>) -> bool {
            plan.output_partitioning().partition_count() > 1
                || plan.children().into_iter().any(partitioned)
        }
        for (name, substantial) in [("tiny", false), ("selected", false), ("empty", false), ("large", true)] {
            // Unsupported native LIKE leaves a real compute filter below a non-native sort.
            // The plan is built against the actual native provider without database I/O.
            let plan = workspace.context.table(name).await.unwrap()
                .filter(col("version").like(lit("%x%"))).unwrap()
                .sort(vec![col("version").sort(true, false)]).unwrap()
                .create_physical_plan().await.unwrap();
            assert_eq!(partitioned(&plan), substantial && config.target_partitions() > 1,
                "partition benefit must follow cardinality, not the seven-row transfer window: {name}");
        }
        fn native_source(plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>) -> bool {
            plan.name() == "NativeCompilerExec" || plan.children().into_iter().any(native_source)
        }
        let frame = crate::sql::query(&workspace.context, "SELECT count(*) FROM empty").await.unwrap();
        assert!(native_source(&frame.create_physical_plan().await.unwrap()),
            "inexact empty selection cannot replace native authority with aggregate statistics");
        assert!(frame.collect().await.is_err(),
            "an empty selected scan must reach this deliberately unconnected authority, rather than return a count");
    }
    #[tokio::test]
    async fn logical_templates_share_only_fixed_ports_program_and_input_order() {
        use datafusion::datasource::MemTable;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let context = SessionContext::new();
        let prepared = PreparedInputSession { context: context.clone(), aliases: BTreeMap::new(), templates: Mutex::new((BTreeMap::new(), StateCharge::new(&budget, "template-control"))) };
        let cell = tokio::sync::OnceCell::new();
        assert!(cell.set(prepared).is_ok());
        let access = CompletedInputs { attempt: Arc::new(WorkspaceFiles { directory: tempfile::tempdir().unwrap() }), model: ContentHash::of(b"model"), program: ContentHash::of(b"program"), budget: budget.clone(), producing_scope: None, prepared_session: Arc::new(cell), name: "template-control", profile: Profile::Catalog, relations: BTreeMap::new() };
        let input = lctx_model::domain::ValidationInput::of::<Package>(&["id"]);
        access.query_template(&context, &input, "SELECT current_date() AS id").await.unwrap();
        access.query_template(&context, &input, "SELECT current_date() AS id").await.unwrap();
        assert_eq!(access.prepared_session.get().unwrap().templates.lock().unwrap().0.len(), 1);
        let order = lctx_model::domain::ValidationInput::of::<Package>(&["name"]);
        access.query_template(&context, &order, "SELECT current_date() AS id").await.unwrap();
        assert_eq!(access.prepared_session.get().unwrap().templates.lock().unwrap().0.len(), 2);
        access.query_template(&context, &input, "SELECT 7 AS id").await.unwrap();
        access.query_template(&context, &input, "SELECT 8 AS id").await.unwrap();
        assert_eq!(access.prepared_session.get().unwrap().templates.lock().unwrap().0.len(), 2, "per-root literal SQL must not retain plan history");
        let batch = Package::encode(&[Package { name: "temporary".into() }]).unwrap();
        context.register_table("temporary_selected_port", Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap())).unwrap();
        access.query_template(&context, &input, "SELECT * FROM temporary_selected_port").await.unwrap();
        assert_eq!(access.prepared_session.get().unwrap().templates.lock().unwrap().0.len(), 2, "selected providers require typed rebinding");
        assert!(budget.reserved() > 0);
        drop(access);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn reference_target_branches_share_exact_views_but_retain_distinct_epochs() {
        use lctx_model::domain::{completed::CompletedView, stages::PublicationBoundary};
        let first = CompletedView::new(Package::NAME.into(),
            [ContentHash::of(b"first-completed-owner")].into(), 1).unwrap();
        let later = CompletedView::new(Package::NAME.into(),
            [ContentHash::of(b"later-completed-owner")].into(), 1).unwrap();
        let roles = [
            ((PublicationBoundary::Facts, "facts_port"), &first),
            ((PublicationBoundary::Dispatch, "dispatch_port"), &first),
        ];
        let branches = distinct_reference_views(roles.to_vec(), |entry| entry.1).unwrap();
        assert_eq!(branches.len(), 1,
            "two requested roles resolving to one actual view emit one target scan");
        assert_eq!(roles.len(), 2, "physical preparation preserves both semantic roles");
        let branches = distinct_reference_views(roles.into_iter().chain([
            ((PublicationBoundary::CatalogCore, "catalog_port"), &later),
        ]).collect(), |entry| entry.1).unwrap();
        assert_eq!(branches.len(), 2);
        assert!(branches.iter().any(|(role, _)| *role == (PublicationBoundary::CatalogCore, "catalog_port")),
            "a distinct frozen view remains an independent target branch");
        let mut collision = first.clone();
        collision.rows += 1;
        assert!(matches!(distinct_reference_views(vec![("first", &first), ("collision", &collision)], |entry| entry.1),
            Err(ModelError::Conflict("reference target view descriptor collision"))));
        assert!(distinct_reference_views(Vec::<(&str, &CompletedView)>::new(), |entry| entry.1).unwrap().is_empty(),
            "missing target authority stays missing for the non-null reference refusal");
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
        context.register_table("unrelated_port", unrelated.clone()).unwrap();
        let prepared = PreparedInputSession { context: context.clone(),
            aliases: [("fixed_port".into(), used.clone()), ("unrelated_port".into(), unrelated)].into(),
            templates: Mutex::new((BTreeMap::new(), StateCharge::new(&budget, "template-port-control"))) };
        let cell = tokio::sync::OnceCell::new();
        assert!(cell.set(prepared).is_ok());
        let access = CompletedInputs { attempt: Arc::new(WorkspaceFiles { directory: tempfile::tempdir().unwrap() }),
            model: ContentHash::of(b"model"), program: ContentHash::of(b"program"), budget: budget.clone(),
            producing_scope: None, prepared_session: Arc::new(cell), name: "template-port-control",
            profile: Profile::Catalog, relations: BTreeMap::new() };
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
        context.register_table("unrelated_port", provider("replaced-unrelated")).unwrap();
        access.query_template(&context, &input, sql).await.unwrap();
        access.query_template(&context, &input, "SELECT name FROM fixed_port").await.unwrap();
        assert_eq!(prepared.templates.lock().unwrap().0.len(), 2,
            "unrelated mutation affects neither hits nor misses for referenced fixed ports");
        let retained_budget = budget.reserved();
        let literal = crate::sql::query(&context, "SELECT 7 AS id").await.unwrap();
        let (compatible, transient) = prepared.prepare_template(literal.logical_plan(), &budget, 14).unwrap();
        assert!(!compatible);
        assert!(budget.reserved() > retained_budget, "even noncacheable preparation is charged while live");
        drop(transient);
        drop(literal);
        assert_eq!(budget.reserved(), retained_budget);

        let replacement = provider("replaced-used");
        context.register_table("fixed_port", replacement.clone()).unwrap();
        let hit_error = access.query_template(&context, &input, sql).await.err().unwrap();
        assert!(matches!(hit_error, ModelError::Conflict("prepared completed input provider replaced")));
        let miss_error = access.query_template(&context, &input, "SELECT id FROM fixed_port").await.err().unwrap();
        assert!(matches!(miss_error, ModelError::Conflict("prepared completed input provider replaced")));
        assert_eq!(prepared.templates.lock().unwrap().0.len(), 2,
            "a substituted provider never becomes a cached miss");
        assert_eq!(budget.reserved(), retained_budget, "failed preparation releases its local reservation");

        // Simulate catalog replacement during planning, followed by restoration before
        // inspection. Checking the current alias alone would miss this stale capture.
        let stale = crate::sql::query(&context, sql).await.unwrap();
        context.register_table("fixed_port", used.clone()).unwrap();
        assert!(matches!(prepared.prepare_template(stale.logical_plan(), &budget, sql.len()),
            Err(ModelError::Conflict("prepared completed input provider replaced"))));
        assert_eq!(budget.reserved(), retained_budget);
        access.query_template(&context, &input, sql).await.unwrap();
        let captured = crate::sql::query(&context, sql).await.unwrap();
        let (_, template) = prepared.prepare_template(captured.logical_plan(), &budget, sql.len()).unwrap();
        assert!(template._charge.reserved() > 0, "preparation owns its allowance before catalog verification awaits");
        let refused_budget = ResourceBudget::fixed(64).unwrap();
        assert!(matches!(prepared.prepare_template(captured.logical_plan(), &refused_budget, sql.len()),
            Err(ModelError::Resource { .. })));
        assert_eq!(refused_budget.reserved(), 0);
        context.register_table("fixed_port", replacement).unwrap();
        assert!(matches!(template.verify_ports(&context).await,
            Err(ModelError::Conflict("prepared completed input provider replaced"))),
            "replacement after capture is checked before retaining a planned miss");
        drop(template);
        drop(captured);
        drop(stale);
        assert_eq!(budget.reserved(), retained_budget);
        context.register_table("fixed_port", used).unwrap();
        let concurrent_sql = "SELECT name AS copied_name FROM fixed_port";
        let (left, right) = tokio::join!(
            access.query_template(&context, &input, concurrent_sql),
            access.query_template(&context, &input, concurrent_sql));
        drop((left.unwrap(), right.unwrap()));
        {
            let templates = prepared.templates.lock().unwrap();
            assert_eq!(templates.0.len(), 3);
            let retained = templates.0.values().map(|template| template._charge.reserved()).sum::<usize>();
            assert_eq!(budget.reserved(), retained + templates.1.reserved(),
                "concurrent duplicate misses retain only the winning template reservation");
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
        let batch = Package::encode(&[Package { name: "handoff".into() }]).unwrap();
        sender.send(Ok((batch, budget.reserve("handoff-control", 4096).unwrap()))).await.unwrap();
        let mut reader = NativeBatches { receiver, cancellation: cancellation.clone(), charge: None, done: false };
        assert!(reader.next_async().await.unwrap().is_ok());
        assert_eq!(budget.reserved(), 4096);
        let cancel = async { tokio::task::yield_now().await; cancellation.cancel(); };
        let (next, ()) = tokio::join!(reader.next_async(), cancel);
        assert!(next.unwrap().is_err());
        assert_eq!(budget.reserved(), 0);
        assert!(reader.next_async().await.is_none());
    }
    #[tokio::test]
    async fn completed_inventory_preserves_roles_and_actual_epochs() {
        use lctx_model::domain::stages::PublicationBoundary;
        let workspace = Workspace::new(model(), WorkspaceOptions::default(), crate::test_native::store()).unwrap();
        let output = workspace.output("inventory-source", Profile::Catalog, ContentHash::of(b"inventory-source"),
            workspace.inputs("inventory-source", Profile::Catalog, []).unwrap(), [Package::NAME]);
        output.declare_async::<Package>().await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let source = workspace.completed::<Package>().unwrap();
        let facts = PublicationBoundary::Facts;
        let normalized = PublicationBoundary::Dispatch;
        let access = CompletedInputs { attempt: workspace.files.clone(), model: workspace.model.digest(), program: ContentHash::of(b"inventory-consumer"), budget: workspace.budget.clone(), producing_scope: None, prepared_session: Arc::new(tokio::sync::OnceCell::new()), name: "inventory-consumer", profile: Profile::Catalog, relations: [
            ((Package::NAME, Some(facts)), ResolvedInput { role: 1, requested: Some(facts), resolved: Some(facts), source: source.clone() }),
            ((Package::NAME, Some(normalized)), ResolvedInput { role: 0, requested: Some(normalized), resolved: Some(normalized), source }),
        ].into() };
        let roles = access.ordered_bindings().iter().map(|(_, binding)| binding.resolved).collect::<Vec<_>>();
        assert_eq!(roles, vec![Some(normalized), Some(facts)]);
        assert!(access.relation::<Package>().is_err());
        assert!(access.table_for(&lctx_model::domain::ValidationInput::of::<Package>(&["id"])).is_err());
        let session = access.session(&workspace).await.unwrap();
        assert!(!session.table_exist(Package::NAME).unwrap());
        assert!(session.table_exist(CompletedInputs::table(Package::NAME, Some(facts))).unwrap());
        let left = access.predicate_session(&workspace).await.unwrap();
        let right = access.predicate_session(&workspace).await.unwrap();
        left.register_batch("cached_normalization_control", Package::encode(&[Package { name: "left".into() }]).unwrap()).unwrap();
        right.register_batch("cached_normalization_control", Package::encode(&[Package { name: "right".into() }]).unwrap()).unwrap();
        let (left_rows, right_rows) = tokio::join!(
            left.table("cached_normalization_control"), right.table("cached_normalization_control"));
        let (left_rows, right_rows) = tokio::join!(left_rows.unwrap().collect(), right_rows.unwrap().collect());
        assert_eq!(Package::decode(&left_rows.unwrap()[0]).unwrap()[0].name, "left");
        assert_eq!(Package::decode(&right_rows.unwrap()[0]).unwrap()[0].name, "right");
        drop(left);
        drop(right);
        assert!(!session.table_exist("cached_normalization_control").unwrap(), "candidate arrays never enter the retained input catalog");
        let output = workspace.output("inventory-consumer", Profile::Catalog, ContentHash::of(b"consumer"), access,
            [Package::NAME]);
        let request = output.product_request().unwrap();
        assert_eq!(request.dependencies[0].role, "input/0");
        assert_eq!(request.dependencies[0].prefix.as_deref(), Some(normalized.name()));
        assert_eq!(request.dependencies[1].prefix.as_deref(), Some(facts.name()));
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
            producing_scope: None,
            native_calls: calls.clone(),
            native,
            bridge: bridge.clone(),
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
        let task_budget = ResourceBudget::fixed(1 << 20).unwrap();
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
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            crate::test_native::store(),
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
                relation: Relation::of::<Package>(),
                contribution: true,
            },
            write: Some(write),
            calls: calls.clone(),
            keep_alive: Box::new(CloseOwnerWitness(dropped.clone())),
        };
        let caller = tokio::spawn(close_writer(Box::new(move || Ok(prepared))));
        entered.notified().await;
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
    }
    #[tokio::test]
    async fn cancelled_final_close_keeps_batch_and_writer_until_native_terminality() {
        cancelled_final_close_retains_batch_and_writer(false).await;
    }
    #[tokio::test]
    async fn cancelled_final_close_retains_late_failure_without_releasing_batch_early() {
        cancelled_final_close_retains_batch_and_writer(true).await;
    }
    async fn packages(memory: usize, batch: usize, reverse: bool) -> Arc<CompletedRelation> {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions {
                memory_bytes: memory,
                partitions: 1,
                batch_rows: batch,
            },
            crate::test_native::store(),
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
        let contributions = workspace.native().contributions().await.unwrap();
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
        assert!(workspace.native().contributions().await.unwrap().is_empty());
        assert!(refused.finish(ProviderOutcome::Complete).await.is_err());
        let duplicate = workspace.output(
            "duplicate-async",
            Profile::Catalog,
            ContentHash::of(b"duplicate-async"),
            workspace
                .inputs("duplicate-async", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        duplicate.declare_async::<Package>().await.unwrap();
        assert!(
            matches!(duplicate.declare_async::<Package>().await,Err(ModelError::Invalid(message)) if message.contains("declared twice"))
        );
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        workspace.drain().await.unwrap();
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
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let contributions = workspace.native().contributions().await.unwrap();
        assert_eq!(contributions.len(), 1);
        assert!(contributions[0].spec.outputs.is_empty());
        assert!(contributions[0].outputs.is_empty());
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
        let budget = ResourceBudget::fixed(4096).unwrap();
        let constrained = Workspace::with_budget(
            model(),
            WorkspaceOptions {
                memory_bytes: 4096,
                ..Default::default()
            },
            budget,
            crate::test_native::store(),
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
