//! Independent native artifact admission. Compiler providers and captured filesystem inputs remain above.
mod reference_preparation;
mod uniform;
pub use uniform::AdmittedUniformOwners;
pub(crate) mod candidate;
mod durable;
mod normalization_declarations;
pub(crate) mod scoped;
use crate::compiler::NativeCompilerStore;
use arrow_array::RecordBatch;
use datafusion::{
    catalog::TableProvider,
    execution::{
        disk_manager::{DiskManagerBuilder, DiskManagerMode},
        memory_pool::{MemoryPool, MemoryReservation},
        runtime_env::RuntimeEnvBuilder,
    },
    prelude::{SessionConfig, SessionContext},
};
use futures::{FutureExt, TryStreamExt, future::BoxFuture};
use lctx_model::domain::completed::{CompletedBinding, CompletedContribution, CompletedView};
use lctx_model::domain::{
    Batch, ContentHash, ModelError, Record, Relation, ValidatedModel, ValidationInput,
    charged::StateCharge,
    resources::{Reservation, ResourceBudget},
    stages::{Profile, PublicationBoundary},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Debug, Clone, Copy)]
pub struct AdmissionOptions {
    pub memory_bytes: usize,
    pub partitions: usize,
    pub batch_rows: usize,
}
impl Default for AdmissionOptions {
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
    native_retry: lctx_native::control::RetryCancellation,
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<CancellationState>);
impl Cancellation {
    pub fn shares_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.native_retry.cancel();
        self.0.notify.notify_waiters();
    }
    pub(crate) fn native_retry(&self) -> lctx_native::control::RetryCancellation {
        self.0.native_retry.clone()
    }
    pub fn check(&self) -> Result<(), ModelError> {
        if self.0.cancelled.load(Ordering::Acquire) {
            Err(ModelError::Invalid("compilation cancelled".into()))
        } else {
            Ok(())
        }
    }
    pub async fn cancelled(&self) {
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

/// Only a caller-owned read-only planner is borrowed by the shared compiler validation kernel.
/// This creates no artifact capability and exposes none of the private admission runtime's catalog.
pub trait ValidationPlanner: Send + Sync {
    fn query<'a>(
        &'a self,
        session: &'a SessionContext,
        input: &'a lctx_model::domain::ValidationInput,
        sql: &'a str,
    ) -> BoxFuture<'a, Result<datafusion::dataframe::DataFrame, ModelError>>;
}
pub trait InputTables: Send + Sync {
    fn table_for(&self, input: &lctx_model::domain::ValidationInput) -> Result<String, ModelError>;
    fn contains_relation(&self, name: &str) -> bool;
}
/// Exact metadata for an upper compiler selection. Requested roles are independent of resolved epochs.
#[derive(Clone)]
pub struct SelectedBinding {
    pub role: usize,
    pub requested: Option<PublicationBoundary>,
    pub resolved: Option<PublicationBoundary>,
    pub binding: CompletedBinding,
}
#[derive(Clone)]
struct ResolvedInput {
    role: usize,
    requested: Option<PublicationBoundary>,
    resolved: Option<PublicationBoundary>,
    source: Arc<ExactRelation>,
}
impl std::ops::Deref for ResolvedInput {
    type Target = Arc<ExactRelation>;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
/// Read metadata never acquires producer write authority or a synchronous bridge.
pub struct ExactRelation {
    relation: Relation,
    view: CompletedView,
    snapshot: lctx_model::domain::analysis::sources::SourceSnapshot,
}
impl ExactRelation {
    pub fn name(&self) -> &'static str {
        self.relation.name()
    }
    pub fn view(&self) -> &CompletedView {
        &self.view
    }
    pub fn view_identity(&self) -> ContentHash {
        self.view.identity
    }
    pub fn snapshot(&self) -> lctx_model::domain::analysis::sources::SourceSnapshot {
        self.snapshot.clone()
    }
}
#[derive(PartialEq, Eq)]
struct FactsAvailabilityKey {
    profile: Profile,
    views: [ContentHash; 10],
}
pub struct AdmittedFacts {
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

/// Fresh executed production evidence. Only validate_scope constructs this after
/// reference closure and every applicable check reaches its required terminal state.
pub(crate) struct CheckedSemanticScope {
    invariants: Vec<lctx_model::domain::artifact_admission::ValidationExecution>,
    model: ContentHash,
    profile: Profile,
    bindings: ContentHash,
    _charge: StateCharge,
    // Actual private checks and their charged native backing survive the assembly borrower.
    scoped: Vec<Arc<scoped::ExecutedCheck>>,
}
type SemanticEvidence = (
    Vec<lctx_model::domain::artifact_admission::ValidationExecution>,
    StateCharge,
    Vec<Arc<scoped::ExecutedCheck>>,
);
impl CheckedSemanticScope {
    pub(crate) fn into_evidence(
        self,
        model: ContentHash,
        profile: Profile,
        bindings: ContentHash,
    ) -> Result<SemanticEvidence, ModelError> {
        if self.model != model || self.profile != profile || self.bindings != bindings {
            return Err(ModelError::Conflict(
                "executed semantic scope differs from artifact",
            ));
        }
        Ok((self.invariants, self._charge, self.scoped))
    }
}
pub(crate) struct CheckedFrontier {
    checks: Vec<lctx_model::domain::artifact_admission::ValidationExecution>,
    model: ContentHash,
    profile: Profile,
    frontier: lctx_model::domain::admission::Frontier,
    _charge: StateCharge,
}
impl CheckedFrontier {
    pub(crate) fn into_evidence(
        self,
        model: ContentHash,
        profile: Profile,
        frontier: lctx_model::domain::admission::Frontier,
    ) -> Result<
        (
            Vec<lctx_model::domain::artifact_admission::ValidationExecution>,
            StateCharge,
        ),
        ModelError,
    > {
        if self.model != model || self.profile != profile || self.frontier != frontier {
            return Err(ModelError::Conflict(
                "executed frontier differs from artifact",
            ));
        }
        Ok((self.checks, self._charge))
    }
}
struct ScopeValidation {
    identity: ContentHash,
    evidence: Option<CheckedSemanticScope>,
}

type UniformOwnersKey = (Profile, ContentHash);
type CachedUniformOwners = (UniformOwnersKey, Arc<AdmittedUniformOwners>);
/// Exact completed metadata and the reservation covering its transfer lifetime.
pub struct CompletedInventory {
    pub contributions: Vec<CompletedContribution>,
    pub bindings: Vec<CompletedBinding>,
    pub transfer: Box<dyn Reservation>,
}
/// Share compact premises across compiler validation calls without sharing mutable provider state.
pub struct AdmissionMemo {
    scoped: scoped::ScopedChecks,
    checked: Mutex<(BTreeSet<ContentHash>, StateCharge)>,
    facts: Mutex<Option<(FactsAvailabilityKey, Arc<AdmittedFacts>)>>,
    uniform: Mutex<Option<CachedUniformOwners>>,
}
impl AdmissionMemo {
    pub fn new(budget: &ResourceBudget) -> Arc<Self> {
        Arc::new(Self {
            scoped: scoped::ScopedChecks::new(budget),
            checked: Mutex::new((
                BTreeSet::new(),
                StateCharge::new(budget, "compiler-validity-premises"),
            )),
            facts: Mutex::default(),
            uniform: Mutex::default(),
        })
    }
    /// Drainage ends operation-local reuse. Actual artifact borrowers retain their
    /// individual charged checks independently until their final use.
    pub fn close_scoped(&self) -> Result<(), ModelError> {
        self.scoped.close()
    }
}
/// Shared execution finality retained independently from a producer's read catalog.
pub trait ExecutionOwner: Send + Sync {
    /// Actual joins and cleanup only; acknowledged operation outcomes remain separate.
    fn drain_owned_report(&self) -> BoxFuture<'_, lctx_model::domain::completion::Completion>;
    /// Append retained operation diagnostics only after the finality signal is settled.
    fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    );
    fn complete_signal<'a>(
        &'a self,
        completion: &'a lctx_model::domain::completion::Completion,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
}
/// The restored/publication owner has only exact completed bindings, a private spillable session,
/// typed model validators, resource/cancellation ownership and the native attempt.
pub struct AdmissionRuntime {
    _directory: Option<tempfile::TempDir>,
    borrowed_execution: Option<Arc<dyn ExecutionOwner>>,
    context: SessionContext,
    options: AdmissionOptions,
    budget: ResourceBudget,
    model: Arc<ValidatedModel>,
    native: Arc<NativeCompilerStore>,
    cancellation: Cancellation,
    signal: Option<lctx_native::signal::SignalRegistration>,
    native_calls: Arc<crate::native_calls::NativeCalls>,
    completed: Mutex<BTreeMap<&'static str, Arc<ExactRelation>>>,
    frozen_shared: Mutex<BTreeMap<(PublicationBoundary, &'static str), Arc<ExactRelation>>>,
    owners: Mutex<(BTreeMap<ContentHash, CompletedContribution>, StateCharge)>,
    bindings_charge: Mutex<StateCharge>,
    // A live selected assurance snapshot retains exact producer dependency bindings without
    // pretending that those bindings constitute a complete restored publication inventory.
    selected_dependencies: Mutex<Vec<CompletedBinding>>,
    selected_only: bool,
    selection_bound: AtomicBool,
    memo: Arc<AdmissionMemo>,
    private_validation: Mutex<Option<Arc<crate::private_validation::PrivateValidation>>>,
}
impl AdmissionRuntime {
    pub fn with_budget(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        native: Arc<NativeCompilerStore>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::with_ownership(
            model,
            options,
            budget,
            native,
            Cancellation::default(),
            None,
            None,
        )
    }
    pub fn with_ownership(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        calls: Option<Arc<crate::native_calls::NativeCalls>>,
        memo: Option<Arc<AdmissionMemo>>,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes != budget.limit()
            || options.memory_bytes == 0
            || options.partitions == 0
            || options.batch_rows == 0
        {
            return Err(ModelError::Invalid(
                "admission runtime sizes or supplied pool".into(),
            ));
        }
        let token = cancellation.clone();
        let signal = Some(lctx_native::signal::SignalRegistration::new(
            "independent artifact admission",
            move || token.cancel(),
        )?);
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let pool = Arc::new(BudgetMemoryPool {
            budget: budget.clone(),
            allocated: Mutex::new(budget.reserve("admission-datafusion", 0)?),
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
        let native_calls = calls.unwrap_or_else(|| {
            Arc::new(crate::native_calls::NativeCalls::new(
                native.clone(),
                cancellation.clone(),
                &budget,
            ))
        });
        let memo = memo.unwrap_or_else(|| AdmissionMemo::new(&budget));
        let owners = Mutex::new((
            BTreeMap::new(),
            StateCharge::new(&budget, "compiler-completed-owners"),
        ));
        Ok(Arc::new(Self {
            _directory: Some(directory),
            borrowed_execution: None,
            bindings_charge: Mutex::new(StateCharge::new(&budget, "compiler-completed-bindings")),
            context: SessionContext::new_with_config_rt(config, runtime),
            options,
            budget,
            model,
            native,
            cancellation,
            signal,
            native_calls,
            completed: Mutex::default(),
            frozen_shared: Mutex::default(),
            owners,
            selected_dependencies: Mutex::default(),
            selected_only: false,
            selection_bound: AtomicBool::new(false),
            memo,
            private_validation: Mutex::new(None),
        }))
    }
    /// Borrow runtime services and their checked execution owner. No caller catalog is copied.
    #[expect(
        clippy::too_many_arguments,
        reason = "Composition wires independent semantic, execution, cancellation and resource owners explicitly"
    )]
    pub fn borrow_runtime(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        calls: Arc<crate::native_calls::NativeCalls>,
        memo: Arc<AdmissionMemo>,
        config: SessionConfig,
        runtime: Arc<datafusion::execution::runtime_env::RuntimeEnv>,
        execution: Arc<dyn ExecutionOwner>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::borrow_runtime_for(
            model,
            options,
            budget,
            native,
            cancellation,
            calls,
            memo,
            config,
            runtime,
            execution,
            false,
        )
    }
    /// Construct a private selected read/assurance owner. It can never acquire whole
    /// inventory installation, restoration, freezing or publication authority.
    #[expect(
        clippy::too_many_arguments,
        reason = "Selected assurance borrows the same independent runtime services as full admission"
    )]
    pub fn borrow_selected_runtime(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        calls: Arc<crate::native_calls::NativeCalls>,
        memo: Arc<AdmissionMemo>,
        config: SessionConfig,
        runtime: Arc<datafusion::execution::runtime_env::RuntimeEnv>,
        execution: Arc<dyn ExecutionOwner>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::borrow_runtime_for(
            model,
            options,
            budget,
            native,
            cancellation,
            calls,
            memo,
            config,
            runtime,
            execution,
            true,
        )
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Immutable inventory scope is fixed while composing independent runtime services"
    )]
    fn borrow_runtime_for(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        calls: Arc<crate::native_calls::NativeCalls>,
        memo: Arc<AdmissionMemo>,
        config: SessionConfig,
        runtime: Arc<datafusion::execution::runtime_env::RuntimeEnv>,
        execution: Arc<dyn ExecutionOwner>,
        selected_only: bool,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes != budget.limit()
            || options.memory_bytes == 0
            || options.partitions == 0
            || options.batch_rows == 0
        {
            return Err(ModelError::Invalid(
                "admission runtime sizes or supplied pool".into(),
            ));
        }
        let owners = Mutex::new((
            BTreeMap::new(),
            StateCharge::new(&budget, "compiler-completed-owners"),
        ));
        let bindings_charge = Mutex::new(StateCharge::new(&budget, "compiler-completed-bindings"));
        Ok(Arc::new(Self {
            _directory: None,
            borrowed_execution: Some(execution),
            context: SessionContext::new_with_config_rt(config, runtime),
            options,
            budget,
            model,
            native,
            cancellation,
            signal: None,
            native_calls: calls,
            completed: Mutex::default(),
            frozen_shared: Mutex::default(),
            owners,
            bindings_charge,
            selected_dependencies: Mutex::default(),
            selected_only,
            selection_bound: AtomicBool::new(false),
            memo,
            private_validation: Mutex::new(None),
        }))
    }
    /// Transfer exact admitted metadata without exposing the private read catalog.
    pub fn completed_inventory(&self) -> Result<CompletedInventory, ModelError> {
        let owners = self.owners.lock().map_err(|_| poisoned())?;
        let bytes = owners.1.reserved().saturating_mul(3).saturating_add(
            self.bindings_charge
                .lock()
                .map_err(|_| poisoned())?
                .reserved()
                .saturating_mul(3),
        );
        let transfer = self
            .budget
            .reserve("compiler-completed-inventory-transfer", bytes)?;
        Ok(CompletedInventory {
            contributions: owners.0.values().cloned().collect(),
            bindings: self.bindings()?,
            transfer,
        })
    }
    pub(crate) fn require_complete_inventory(&self) -> Result<(), ModelError> {
        if self.selected_only {
            return Err(ModelError::Conflict(
                "selected assurance is not complete inventory authority",
            ));
        }
        Ok(())
    }
    /// Bind an immutable live consumer snapshot. Unlike restoration, this inventory is
    /// intentionally only the selected roots and their authentic producer dependencies.
    pub fn bind_selection_snapshot(
        &self,
        profile: Profile,
        contributions: Vec<CompletedContribution>,
        roots: Vec<CompletedBinding>,
        dependencies: Vec<CompletedBinding>,
    ) -> Result<(), ModelError> {
        if !self.selected_only {
            return Err(ModelError::Conflict(
                "selected snapshot requires selected inventory authority",
            ));
        }
        // Claim initialization before validating or installing any metadata. Even an
        // empty or failed snapshot cannot be rebound, and concurrent binders cannot race.
        if self.selection_bound.swap(true, Ordering::AcqRel) {
            return Err(ModelError::Conflict(
                "selected assurance snapshot already bound",
            ));
        }
        if !self.completed.lock().map_err(|_| poisoned())?.is_empty()
            || !self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .is_empty()
            || !self.owners.lock().map_err(|_| poisoned())?.0.is_empty()
        {
            return Err(ModelError::Conflict(
                "selected assurance requires empty runtime",
            ));
        }
        let mut charge = StateCharge::new(self.budget(), "selected assurance owners");
        let mut owners = BTreeMap::new();
        for owner in contributions {
            owner.spec.validate_profile(profile)?;
            if owner.spec.model != self.model.digest() {
                return Err(ModelError::Conflict("selected assurance owner model"));
            }
            charge.grow(
                serde_json::to_vec(&owner)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(4)
                    .saturating_add(256),
            )?;
            if owners.insert(owner.identity()?, owner).is_some() {
                return Err(ModelError::Conflict("duplicate selected assurance owner"));
            }
        }
        let mut binding_charge = StateCharge::new(self.budget(), "selected assurance bindings");
        for binding in roots.iter().chain(&dependencies) {
            binding_charge.grow(
                serde_json::to_vec(binding)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(8)
                    .saturating_add(1024),
            )?;
            self.exact(binding.clone())?;
            for id in &binding.view.contributions {
                let owner = owners
                    .get(id)
                    .ok_or(ModelError::Conflict("selected assurance view owner absent"))?;
                if !owner.outputs.contains_key(&binding.view.relation) {
                    return Err(ModelError::Conflict(
                        "selected assurance view output absent",
                    ));
                }
            }
        }
        for owner in owners.values() {
            for input in &owner.spec.inputs {
                if !roots
                    .iter()
                    .chain(&dependencies)
                    .any(|binding| binding.source == *input)
                {
                    return Err(ModelError::Conflict(
                        "selected assurance producer input binding absent",
                    ));
                }
            }
        }
        let mut completed = BTreeMap::new();
        let mut frozen = BTreeMap::new();
        for binding in roots {
            let boundary = binding
                .boundary
                .as_ref()
                .map(|name| {
                    PublicationBoundary::ALL
                        .into_iter()
                        .find(|boundary| boundary.name() == name)
                        .ok_or(ModelError::Schema("selected assurance boundary"))
                })
                .transpose()?;
            let source = self.exact(binding)?;
            if let Some(boundary) = boundary {
                if frozen.insert((boundary, source.name()), source).is_some() {
                    return Err(ModelError::Conflict("duplicate selected assurance epoch"));
                }
            } else if completed.insert(source.name(), source).is_some() {
                return Err(ModelError::Conflict(
                    "duplicate selected assurance current relation",
                ));
            }
        }
        *self.owners.lock().map_err(|_| poisoned())? = (owners, charge);
        *self.bindings_charge.lock().map_err(|_| poisoned())? = binding_charge;
        *self.completed.lock().map_err(|_| poisoned())? = completed;
        *self.frozen_shared.lock().map_err(|_| poisoned())? = frozen;
        *self.selected_dependencies.lock().map_err(|_| poisoned())? = dependencies;
        Ok(())
    }
    pub fn install_completed(
        &self,
        profile: Profile,
        contributions: Vec<CompletedContribution>,
        bindings: Vec<CompletedBinding>,
    ) -> Result<(), ModelError> {
        self.require_complete_inventory()?;
        if !self.completed.lock().map_err(|_| poisoned())?.is_empty()
            || !self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .is_empty()
        {
            return Err(ModelError::Conflict(
                "admission restoration requires empty runtime",
            ));
        }
        let mut names = BTreeSet::new();
        let mut owners = BTreeMap::new();
        let mut charge = StateCharge::new(&self.budget, "compiler-completed-owners");
        for contribution in contributions {
            charge.grow(
                serde_json::to_vec(&contribution)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(4)
                    .saturating_add(256),
            )?;
            if owners
                .insert(contribution.identity()?, contribution.clone())
                .is_some()
            {
                return Err(ModelError::Conflict(
                    "duplicate restored contribution owner",
                ));
            }
            contribution.identity()?;
            if contribution.spec.model != self.model.digest() {
                return Err(ModelError::Conflict("restored contribution model"));
            }
            contribution.spec.validate_profile(profile)?;
            for input in &contribution.spec.inputs {
                if input.model() != self.model.digest()
                    || input.rows() < 0
                    || self.model.relation(input.relation()).is_none()
                {
                    return Err(ModelError::Conflict(
                        "restored contribution dependency model",
                    ));
                }
            }
            for output in &contribution.spec.outputs {
                if self.model.relation(output).is_none() {
                    return Err(ModelError::Schema("restored contribution output relation"));
                }
                names.insert(output.clone());
            }
        }
        let current = bindings
            .iter()
            .filter(|b| b.boundary.is_none())
            .map(|b| b.view.relation.clone())
            .collect::<BTreeSet<_>>();
        if current != names {
            return Err(ModelError::Conflict(
                "restored current output binding inventory",
            ));
        }
        let mut binding_charge = StateCharge::new(&self.budget, "compiler-completed-bindings");
        for binding in &bindings {
            binding_charge.grow(
                serde_json::to_vec(binding)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(8)
                    .saturating_add(1024),
            )?;
        }
        let mut completed = BTreeMap::new();
        let mut frozen = BTreeMap::new();
        for binding in bindings {
            let boundary = binding
                .boundary
                .as_ref()
                .map(|name| {
                    PublicationBoundary::ALL
                        .into_iter()
                        .find(|b| b.name() == name)
                        .ok_or(ModelError::Schema("restored compiler boundary"))
                })
                .transpose()?;
            let source = self.exact(binding)?;
            let name = source.name();
            if let Some(boundary) = boundary {
                if frozen.insert((boundary, name), source).is_some() {
                    return Err(ModelError::Conflict("duplicate frozen restored binding"));
                }
            } else if completed.insert(name, source).is_some() {
                return Err(ModelError::Conflict("duplicate current restored binding"));
            }
        }
        *self.bindings_charge.lock().map_err(|_| poisoned())? = binding_charge;
        *self.owners.lock().map_err(|_| poisoned())? = (owners, charge);
        *self.completed.lock().map_err(|_| poisoned())? = completed;
        *self.frozen_shared.lock().map_err(|_| poisoned())? = frozen;
        Ok(())
    }
    fn exact(&self, binding: CompletedBinding) -> Result<Arc<ExactRelation>, ModelError> {
        binding.validate()?;
        let relation = self
            .model
            .relation(binding.source.relation())
            .ok_or(ModelError::Schema("restored compiler relation"))?
            .clone();
        if binding.configuration.is_some()
            || binding.source.model() != self.model.digest()
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
        Ok(Arc::new(ExactRelation {
            relation,
            view: binding.view,
            snapshot: binding.source,
        }))
    }
    pub async fn restore(&self, profile: Profile) -> Result<(), ModelError> {
        self.require_complete_inventory()?;
        self.install_completed(
            profile,
            self.native.contributions(&self.budget).await?,
            self.native.bindings().await?,
        )
    }
    pub(crate) async fn freeze_for_admission(&self) -> Result<(), ModelError> {
        self.require_complete_inventory()?;
        self.cancellation.check()?;
        let descriptor_bytes = self.owners.lock().map_err(|_| poisoned())?.1.reserved();
        let bindings = self.bindings()?;
        let binding_bytes = bindings.iter().try_fold(0usize, |bytes, binding| {
            Ok::<_, ModelError>(
                bytes.saturating_add(
                    serde_json::to_vec(&binding.view)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(8)
                        .saturating_add(512),
                ),
            )
        })?;
        let _charge = self.budget.reserve(
            "compiler-final-handoff",
            descriptor_bytes
                .saturating_mul(3)
                .saturating_add(binding_bytes),
        )?;
        let inventory = self.native.freeze_content(&self.budget).await?;
        let mut owners = BTreeMap::new();
        for owner in inventory.contributions {
            if owners.insert(owner.identity()?, owner).is_some() {
                return Err(ModelError::Conflict(
                    "duplicate frozen native completed owner",
                ));
            }
        }
        if owners != self.owners.lock().map_err(|_| poisoned())?.0 {
            return Err(ModelError::Conflict(
                "frozen native completed owner handoff",
            ));
        }
        let key = |b: &CompletedBinding| (b.boundary.clone(), b.view.relation.clone());
        let mut actual = inventory.bindings;
        let mut expected = bindings;
        actual.sort_by_key(key);
        expected.sort_by_key(key);
        if actual != expected {
            return Err(ModelError::Conflict(
                "frozen native completed binding handoff",
            ));
        }
        self.native.complete_physical_content(&self.budget).await?;
        Ok(())
    }
    fn bindings(&self) -> Result<Vec<CompletedBinding>, ModelError> {
        let mut result = Vec::new();
        for source in self.completed.lock().map_err(|_| poisoned())?.values() {
            result.push(CompletedBinding {
                boundary: None,
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        for ((boundary, _), source) in self.frozen_shared.lock().map_err(|_| poisoned())?.iter() {
            result.push(CompletedBinding {
                boundary: Some(boundary.name().into()),
                source: source.snapshot(),
                view: source.view.clone(),
                configuration: None,
            });
        }
        for binding in self
            .selected_dependencies
            .lock()
            .map_err(|_| poisoned())?
            .iter()
        {
            if !result.contains(binding) {
                result.push(binding.clone());
            }
        }
        Ok(result)
    }
    pub(crate) fn transport_session(&self) -> SessionContext {
        SessionContext::new_with_config_rt(self.context.copied_config(), self.context.runtime_env())
    }
    pub fn native(&self) -> &Arc<NativeCompilerStore> {
        &self.native
    }
    pub fn model(&self) -> &Arc<ValidatedModel> {
        &self.model
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub fn options(&self) -> AdmissionOptions {
        self.options
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    pub(crate) fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        self.native_calls.call(future)
    }
    /// Drain actual execution ownership without ending a surrounding publication's signal lifetime.
    /// Historical acknowledged operation failures are appended by the aggregate wrapper.
    pub fn drain_owned_report(&self) -> BoxFuture<'_, lctx_model::domain::completion::Completion> {
        // Dispatch before constructing the owned-native continuation. A borrowed
        // execution already owns its drain; composing the inactive native branch
        // into its future needlessly multiplies native finalization stack state.
        if let Some(execution) = &self.borrowed_execution {
            return execution.drain_owned_report();
        }
        async move {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.merge(self.native_calls.drain_finality_report().await);
            completion.step("private scoped check release", self.memo.close_scoped());
            let native = self.native.drain_finality_report().await;
            completion.merge(native);
            completion
        }
        .boxed()
    }
    /// The enclosing lifetime supplies finality after publication reconciliation and cleanup.
    pub async fn complete_signal(
        &self,
        completion: &lctx_model::domain::completion::Completion,
    ) -> Result<(), ModelError> {
        if let Some(execution) = &self.borrowed_execution {
            execution.complete_signal(completion).await
        } else if let Some(signal) = &self.signal {
            signal.complete(completion).await
        } else {
            Ok(())
        }
    }
    pub async fn drain_report(&self) -> lctx_model::domain::completion::Completion {
        let mut completion = self.drain_owned_report().await;
        let settled = self.complete_signal(&completion).await;
        completion.step("independent artifact admission signal finality", settled);
        self.append_operation_failures(&mut completion);
        completion
    }
    pub fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        if let Some(execution) = &self.borrowed_execution {
            execution.append_operation_failures(completion);
        } else {
            self.native_calls.append_operation_failures(completion);
            self.native.append_operation_failures(completion);
        }
    }
    pub fn completed<R: Record>(&self) -> Result<Arc<ExactRelation>, ModelError> {
        self.relation(R::NAME)
    }
    pub fn relation(&self, name: &str) -> Result<Arc<ExactRelation>, ModelError> {
        self.completed
            .lock()
            .map_err(|_| poisoned())?
            .get(name)
            .cloned()
            .ok_or_else(|| ModelError::Invalid(format!("input {name} is not completed")))
    }
    pub fn completed_relations(&self) -> Result<Vec<Arc<ExactRelation>>, ModelError> {
        Ok(self
            .completed
            .lock()
            .map_err(|_| poisoned())?
            .values()
            .cloned()
            .collect())
    }
    fn input_relation(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<Arc<ExactRelation>, ModelError> {
        if let Some(boundary) = input.prefix() {
            return self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .get(&(boundary, input.name()))
                .cloned()
                .ok_or(ModelError::Conflict("missing frozen validation input"));
        }
        self.relation(input.name())
    }
    fn inputs(
        &self,
        names: impl IntoIterator<Item = &'static str>,
    ) -> Result<ValidationInputs, ModelError> {
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
        Ok(ValidationInputs {
            relations,
            companions: BTreeMap::new(),
        })
    }
    pub async fn validate_compiler_selection(
        &self,
        profile: Profile,
        admission: bool,
        bindings: Vec<SelectedBinding>,
        session: &SessionContext,
        planner: &dyn ValidationPlanner,
    ) -> Result<ContentHash, ModelError> {
        let inputs = self.selected(bindings)?;
        self.validate_scope(
            profile,
            admission,
            Some(&inputs),
            Some((session, planner)),
            false,
        )
        .await
        .map(|result| result.identity)
    }
    fn selected(&self, bindings: Vec<SelectedBinding>) -> Result<ValidationInputs, ModelError> {
        let mut roles = BTreeSet::new();
        let mut relations = BTreeMap::new();
        let mut exact = BTreeMap::<ContentHash, Arc<ExactRelation>>::new();
        for selected in bindings {
            if !roles.insert(selected.role) {
                return Err(ModelError::Conflict("selected admission duplicate role"));
            }
            let source = self.exact(selected.binding)?;
            let source = if let Some(previous) = exact.get(&source.view.identity) {
                if previous.view != source.view || previous.snapshot != source.snapshot {
                    return Err(ModelError::Conflict("selected admission view collision"));
                }
                previous.clone()
            } else {
                exact.insert(source.view.identity, source.clone());
                source
            };
            let key = (source.name(), selected.requested);
            if relations
                .insert(
                    key,
                    ResolvedInput {
                        role: selected.role,
                        requested: selected.requested,
                        resolved: selected.resolved,
                        source,
                    },
                )
                .is_some()
            {
                return Err(ModelError::Conflict(
                    "selected admission duplicate selector",
                ));
            }
        }
        Ok(ValidationInputs {
            relations,
            companions: BTreeMap::new(),
        })
    }
    pub async fn admit_semantics(&self, profile: Profile) -> Result<(), ModelError> {
        self.execute_semantics(profile).await.map(|_| ())
    }
    pub(crate) fn execute_semantics<'a>(
        &'a self,
        profile: Profile,
    ) -> BoxFuture<'a, Result<CheckedSemanticScope, ModelError>> {
        Box::pin(async move {
            self.require_complete_inventory()?;
            let relations = self.completed_relations()?;
            let mut inputs = self.inputs(relations.iter().map(|r| r.name()))?;
            self.bind_admission_epochs(&mut inputs)?;
            self.validate_scope(profile, true, Some(&inputs), None, true)
                .await?
                .evidence
                .ok_or(ModelError::Conflict(
                    "missing fresh executed semantic evidence",
                ))
        })
    }
    async fn coverage_rows_async<R: Record>(
        &self,
        source: &ExactRelation,
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
                    + row.assurance.heap_bytes()
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
        compact: &[lctx_model::domain::uniform_outcome::UniformNotRequested],
        compact_authority: &BTreeSet<ContentHash>,
    ) -> Result<AdmittedFacts, ModelError> {
        let rows = facts.observed;
        let recorded = crate::facts::recorded_coverage_with_uniform(
            &self.model,
            profile,
            &facts,
            compact,
            compact_authority,
            &self.budget,
        )?;
        let scopes = scope_rows.into_iter().map(|row| (row.id(), row)).collect();
        let uniform = recorded
            .uniform
            .iter()
            .map(|row| (&row.descriptor, &row.domain, &row.owner))
            .collect::<Vec<_>>();
        let availability =
            lctx_model::domain::admission::ScopedAvailability::from_completed_with_uniform(
                profile,
                &recorded.expected,
                rows,
                &scopes,
                &uniform,
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
    ) -> Result<(FactsAvailabilityKey, [Arc<ExactRelation>; 10]), ModelError> {
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
            bind(lctx_model::domain::uniform_outcome::UniformNotRequested::NAME)?,
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
            .memo
            .facts
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
        let mut admitted = self.memo.facts.lock().map_err(|_| poisoned())?;
        if let Some((known, value)) = &*admitted
            && *known == key
        {
            return Ok(value.clone());
        }
        let value = Arc::new(value);
        *admitted = Some((key, value.clone()));
        Ok(value)
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
            uniform_source,
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
        let (compact, _compact) = self
            .coverage_rows_async::<lctx_model::domain::uniform_outcome::UniformNotRequested>(
                &uniform_source,
            )
            .await?;
        let compact_authority = uniform_source.view().contributions.clone();
        let inventory = self.completed_inventory()?;
        let contributions = &inventory.contributions;
        let _descriptors = self.facts_descriptor_charge(contributions)?;
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
                contributions,
                authority: &authority,
                bound_sources: &bound_sources,
            },
            &compact,
            &compact_authority,
        )?;
        self.remember_facts_availability(key, value)
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = lctx_model::domain::KeySink::new("compiler-workspace-view-set/v1");
        for relation in self.completed_relations()? {
            sink.part(relation.name().as_bytes(), &relation.view_identity().0);
        }
        Ok(sink.finish())
    }
    fn bind_admission_epochs(&self, inputs: &mut ValidationInputs) -> Result<(), ModelError> {
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
    fn validate_scope<'a>(
        &'a self,
        profile: Profile,
        admission: bool,
        selected: Option<&'a ValidationInputs>,
        external: Option<(&'a SessionContext, &'a dyn ValidationPlanner)>,
        earn: bool,
    ) -> BoxFuture<'a, Result<ScopeValidation, ModelError>> {
        Box::pin(async move {
            use lctx_model::domain::artifact_admission::Execution;
            if earn && !admission {
                return Err(ModelError::Conflict(
                    "diagnostic replay cannot earn artifact admission",
                ));
            }
            let mut scoped_attempt = self.memo.scoped.attempt();
            let mut scoped_borrowers = Vec::new();
            let mut evidence = Vec::new();
            let mut evidence_charge =
                StateCharge::new(self.budget(), "executed applicable validation evidence");
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
                None => self.inputs(names.iter().copied())?,
            };
            if selected.is_none() {
                self.bind_admission_epochs(&mut inputs)?;
            }
            // Provider registration, native selection and memoization resolve this one frozen
            // inventory even for whole-artifact admission, never a later live workspace prefix.
            let selected = Some(&inputs);
            let owned_session = if external.is_none() {
                Some(inputs.session(self).await?)
            } else {
                None
            };
            let session = external.map(|(session, _)| session).unwrap_or_else(|| {
                &owned_session
                    .as_ref()
                    .expect("private admission session")
                    .context
            });
            let planner = external
                .map(|(_, planner)| planner)
                .unwrap_or(&inputs as &dyn ValidationPlanner);
            // External registrations are mutable. Only actual exact native providers
            // earn a private reference snapshot; finite or differently selected tables
            // keep real external SQL and cannot consult native premise memoization.
            let reference_snapshot = match external {
                Some((external, _)) => inputs.reference_session(self, external).await?,
                None => None,
            };
            let reference_owner = owned_session.as_ref().or(reference_snapshot.as_ref());
            let reference_session = reference_owner.map_or(session, |owned| &owned.context);
            let reference_phase = lctx_native::phase::Phase::begin("semantic_reference_validation");
            let references = self
                .validate_references(reference_session, &relations, &inputs, reference_owner)
                .await;
            reference_phase.finish_result(&references);
            references?;
            if earn {
                self.validate_normalization_declarations(profile, &relations)
                    .await?;
            }
            // Catalog explicitly leaves native flow unrequested. Empty premises come from that
            // provider declaration, never from arbitrary missing requested relations.
            let mut unrequested = std::collections::BTreeSet::new();
            if profile == Profile::Catalog {
                let declaration = lctx_model::domain::producer_contract::facts::ty_flow();
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
                        if !std::ptr::eq(session, reference_session) {
                            reference_session
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
                        }
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
                                ValidationInputs::table(name, Some(boundary)),
                            )
                        })
                    })
                    .collect(),
                None => self.validation_views(session).await?,
            };
            let referrers = inputs
                .relations
                .iter()
                .map(|((name, _), source)| {
                    let relation = self
                        .model
                        .relation(name)
                        .ok_or(ModelError::Schema("validation referrer relation"))?;
                    let input = ValidationInput::of_relation(relation, &["id"]);
                    // Detached requested roles alias canonical input; they are premises,
                    // never additional physically selected publication roots.
                    Ok(if let Some(epoch) = source.resolved {
                        input.at_epoch(epoch)
                    } else {
                        input
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let checks = if admission {
                self.model.admission_candidates_for_views(&referrers)?
            } else {
                self.model
                    .invariants_for_views_with_premises(&referrers, &unrequested)?
            };
            let uniform = self.uniform_owners_async(profile).await?;
            let scoped_inventory = if admission && reference_owner.is_some() {
                Some(self.scoped_inventory()?)
            } else {
                None
            };
            for invariant in checks {
                tracing::info!(target:"lctx_phase", invariant=invariant.name, "semantic check selected");
                let premise = if inputs.companions.is_empty() {
                    self.validation_premise(
                        &invariant,
                        selected,
                        &unrequested,
                        profile,
                        uniform.identity(),
                    )?
                } else {
                    None
                };
                let mut check = (invariant.create)(self.budget());
                for owner in uniform.owners() {
                    check.admit_uniform_owner(owner)?;
                }
                let is_scoped = check.execution_scope().is_some()
                    || check.normalization_scope().is_some()
                    || check.projection_scope().is_some()
                    || check.retrieval_scope().is_some()
                    || check.aspect_scope().is_some()
                    || check.inventory_scope().is_some()
                    || check.support_scope().is_some();
                // An external planner is not native execution authority. Scoped kernels use
                // the exact provider snapshot already qualified for reference validation.
                let session = if is_scoped && reference_owner.is_some() {
                    reference_session
                } else {
                    session
                };
                let private_whole = !is_scoped
                    && reference_owner.is_some()
                    && self.private_whole_eligible(
                        &invariant,
                        &inputs,
                        &unrequested,
                        uniform.owners().is_empty(),
                    );
                let private_transfer = self
                    .private_validation
                    .lock()
                    .map_err(|_| poisoned())?
                    .is_some();
                let mut scoped = if (is_scoped || private_whole || private_transfer)
                    && let Some(inventory) = &scoped_inventory
                {
                    self.scoped_premise(
                        &invariant,
                        &inputs,
                        &unrequested,
                        profile,
                        is_scoped && check.scoped_uniform_independent(),
                        uniform.owners(),
                        inventory,
                    )
                    .await?
                } else {
                    None
                };
                if let Some(prepared) = &scoped
                    && let Some(executed) = self.reuse_scoped(prepared).await?
                {
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            executed.execution(),
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                        scoped_borrowers.push(executed);
                    }
                    continue;
                }
                if let (Some(prepared), Some(inventory)) = (&mut scoped, &scoped_inventory)
                    && let Some(private) = self.reconcile_private_check(prepared, inventory).await?
                {
                    let transferred = scoped
                        .take()
                        .expect("prepared private transfer")
                        .finish_transferred(private, &self.cancellation)?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            transferred.execution(),
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                        scoped_borrowers.push(transferred.clone());
                    }
                    tracing::info!(target:"lctx_phase", invariant=invariant.name, "reused private semantic check after actual native correspondence");
                    scoped_attempt.retain(transferred)?;
                    self.remember_premise(premise)?;
                    continue;
                }
                if private_whole && let Some(prepared) = scoped.take() {
                    let executed = self
                        .execute_private_whole(prepared, &invariant, &inputs, reference_session)
                        .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::WholeInput,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                        if let Some(executed) = &executed {
                            scoped_borrowers.push(executed.clone());
                        }
                    }
                    if let Some(executed) = executed {
                        scoped_attempt.retain(executed)?;
                    }
                    self.remember_premise(premise)?;
                    continue;
                }
                let execution_scope = check.execution_scope();
                if let Some(scope) = &execution_scope {
                    let alias = if unrequested.contains(scope.root.name()) {
                        scope.root.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(&scope.root)?
                    } else {
                        Self::validation_table(&scope.root, &frozen_tables)?.to_owned()
                    };
                    let sql = format!(
                        "SELECT id FROM {} LIMIT 1",
                        crate::consumed_rows::identifier(&alias)
                    );
                    let mut roots = crate::sql::query(session, &sql)
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
                        if earn {
                            evidence.push(self.execution_evidence(
                                &invariant,
                                Execution::EmptyDomain,
                                &inputs,
                                &unrequested,
                                &mut evidence_charge,
                            )?);
                        }
                        if let Some(prepared) = scoped {
                            self.native.check()?;
                            let executed =
                                prepared.finish(Execution::EmptyDomain, &self.cancellation)?;
                            scoped_attempt.retain(executed.clone())?;
                            if earn {
                                scoped_borrowers.push(executed);
                            }
                        }
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
                    if declared_roots.iter().all(|root| {
                        names.contains(root.name()) || unrequested.contains(root.name())
                    }) {
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
                                session,
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
                            while let Some(batch) =
                                rows.try_next().await.map_err(ModelError::codec)?
                            {
                                self.cancellation.check()?;
                                nonempty |= batch.num_rows() != 0;
                            }
                            if nonempty {
                                break;
                            }
                        }
                        if !nonempty {
                            if earn {
                                evidence.push(self.execution_evidence(
                                    &invariant,
                                    Execution::EmptyDomain,
                                    &inputs,
                                    &unrequested,
                                    &mut evidence_charge,
                                )?);
                            }
                            if let Some(prepared) = scoped {
                                self.native.check()?;
                                let executed =
                                    prepared.finish(Execution::EmptyDomain, &self.cancellation)?;
                                scoped_attempt.retain(executed.clone())?;
                                if earn {
                                    scoped_borrowers.push(executed);
                                }
                            }
                            self.remember_premise(premise)?;
                            continue;
                        }
                    }
                }
                if let Some(input) = invariant.inputs.iter().find(|input| {
                    !names.contains(input.name())
                        && !inputs.companions.contains_key(input.name())
                        && !unrequested.contains(input.name())
                }) {
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
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
                                session,
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
                                session,
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
                                session,
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
                                session,
                                self.budget(),
                                &self.cancellation,
                                &self.model,
                            )
                            .await?
                        }
                    }
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
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
                        session,
                        self.budget(),
                        &self.cancellation,
                    )
                    .await?;
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::Scoped,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
                    if let Some(prepared) = scoped {
                        self.native.check()?;
                        let executed = prepared.finish(Execution::Scoped, &self.cancellation)?;
                        scoped_attempt.retain(executed.clone())?;
                        if earn {
                            scoped_borrowers.push(executed);
                        }
                    }
                    self.remember_premise(premise)?;
                    continue;
                }
                if external.is_none()
                    && !matches!(
                        self.validate_durable_whole(
                            &invariant,
                            &inputs,
                            &unrequested,
                            profile,
                            uniform.owners(),
                            session,
                            planner,
                            None,
                        )
                        .await?,
                        durable::WholeExecution::Unsupported
                    )
                {
                    if earn {
                        evidence.push(self.execution_evidence(
                            &invariant,
                            Execution::WholeInput,
                            &inputs,
                            &unrequested,
                            &mut evidence_charge,
                        )?);
                    }
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
                    let mut stream = planner
                        .query(session, input, &sql)
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
                if earn {
                    evidence.push(self.execution_evidence(
                        &invariant,
                        Execution::WholeInput,
                        &inputs,
                        &unrequested,
                        &mut evidence_charge,
                    )?);
                }
                self.remember_premise(premise)?;
            }
            self.cancellation.check()?;
            let identity = self.identity()?;
            let evidence = if earn {
                Some(CheckedSemanticScope {
                    invariants: evidence,
                    model: self.model.digest(),
                    profile,
                    bindings: lctx_model::domain::completed::binding_inventory_identity(
                        &self.bindings()?,
                    )?,
                    _charge: evidence_charge,
                    scoped: scoped_borrowers,
                })
            } else {
                None
            };
            self.native.check()?;
            self.cancellation.check()?;
            scoped_attempt.commit()?;
            Ok(ScopeValidation { identity, evidence })
        })
    }
    fn execution_evidence(
        &self,
        invariant: &lctx_model::domain::Invariant,
        execution: lctx_model::domain::artifact_admission::Execution,
        inputs: &ValidationInputs,
        unrequested: &BTreeSet<&'static str>,
        charge: &mut StateCharge,
    ) -> Result<lctx_model::domain::artifact_admission::ValidationExecution, ModelError> {
        use lctx_model::domain::artifact_admission::{
            Execution, SelectedInput, ValidationExecution,
        };
        charge.grow(
            invariant
                .name
                .len()
                .saturating_add(invariant.inputs.len().saturating_mul(4096))
                .saturating_add(1024),
        )?;
        let mut selected = Vec::with_capacity(invariant.inputs.len());
        for input in &invariant.inputs {
            if unrequested.contains(input.name()) {
                selected.push(SelectedInput::NotRequested {
                    declaration: lctx_model::domain::producer_contract::facts::ty_flow().identity(),
                });
            } else {
                match inputs.validation_source(input) {
                    Ok(binding) => {
                        let source = binding.source.snapshot();
                        charge.grow(
                            lctx_model::domain::HeapSize::heap_bytes(&source)
                                .saturating_mul(4)
                                .saturating_add(512),
                        )?;
                        selected.push(SelectedInput::Completed {
                            source,
                            requested: binding.requested,
                            resolved: binding.resolved,
                        });
                    }
                    Err(_) if execution == Execution::EmptyDomain => {
                        selected.push(SelectedInput::EmptyDomain)
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(ValidationExecution {
            name: invariant.name.into(),
            revision: invariant.revision,
            definition: invariant.digest(),
            factory: self
                .model
                .factory_contract(invariant.identity())
                .map(|factory| factory.identity())
                .transpose()?,
            execution,
            inputs: selected,
        })
    }
    fn validation_premise(
        &self,
        invariant: &lctx_model::domain::Invariant,
        selected: Option<&ValidationInputs>,
        unrequested: &std::collections::BTreeSet<&'static str>,
        profile: Profile,
        uniform: ContentHash,
    ) -> Result<Option<ContentHash>, ModelError> {
        use lctx_model::domain::Key;
        let Some(factory) = self.model.factory_contract(invariant.identity()) else {
            return Ok(None);
        };
        let mut sink = lctx_model::domain::KeySink::new("compiler-checked-premises/v3");
        self.model.digest().encode(&mut sink);
        invariant.digest().encode(&mut sink);
        factory.identity()?.encode(&mut sink);
        profile.name().to_string().encode(&mut sink);
        uniform.encode(&mut sink);
        for input in &invariant.inputs {
            if unrequested.contains(input.name()) {
                sink.part(b"unrequested", input.name().as_bytes());
                continue;
            }
            let relation = if let Some(selected) = selected {
                selected.validation_source(input).ok().map(|binding| {
                    sink.part(
                        b"requested-selector",
                        binding
                            .requested
                            .map(|p| p.name())
                            .unwrap_or("default")
                            .as_bytes(),
                    );
                    sink.part(
                        b"resolved-selector",
                        binding
                            .resolved
                            .map(|p| p.name())
                            .unwrap_or("default")
                            .as_bytes(),
                    );
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
            let mut checked = self.memo.checked.lock().map_err(|_| poisoned())?;
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
        self.execute_frontier(frontier, profile).await.map(|_| ())
    }
    pub(crate) async fn execute_frontier(
        &self,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
    ) -> Result<CheckedFrontier, ModelError> {
        self.require_complete_inventory()?;
        use lctx_model::domain::artifact_admission::{
            Execution, SelectedInput, ValidationExecution, frontier_checks,
        };
        let ids = frontier_checks(frontier)?;
        let mut evidence = Vec::with_capacity(ids.len());
        let mut charge = StateCharge::new(self.budget(), "executed frontier evidence");
        let relations = self.completed_relations()?;
        let mut access = self.inputs(relations.iter().map(|r| r.name()))?;
        self.bind_admission_epochs(&mut access)?;
        let session = access.session(self).await?;
        let uniform = self.uniform_owners_async(profile).await?;
        for id in ids {
            let invariant = self.model.publication_check(id)?;
            let mut check = (invariant.create)(self.budget());
            for owner in uniform.owners() {
                check.admit_uniform_owner(owner)?;
            }
            charge.grow(
                invariant
                    .inputs
                    .len()
                    .saturating_mul(4096)
                    .saturating_add(invariant.name.len())
                    .saturating_add(1024),
            )?;
            let mut selected = Vec::with_capacity(invariant.inputs.len());
            let mut sources = Vec::with_capacity(invariant.inputs.len());
            for input in &invariant.inputs {
                let binding = access.validation_source(input)?;
                let source = binding.source.snapshot();
                charge.grow(
                    lctx_model::domain::HeapSize::heap_bytes(&source)
                        .saturating_mul(8)
                        .saturating_add(1024),
                )?;
                selected.push(SelectedInput::Completed {
                    source: source.clone(),
                    requested: binding.requested,
                    resolved: binding.resolved,
                });
                sources.push(source);
                let order = input
                    .order()
                    .iter()
                    .map(|field| crate::consumed_rows::identifier(field))
                    .collect::<Vec<_>>()
                    .join(",");
                let mut stream = crate::sql::query(
                    &session,
                    &format!(
                        "SELECT * FROM {}{}",
                        crate::consumed_rows::identifier(&access.validation_table(input)?),
                        if order.is_empty() {
                            String::new()
                        } else {
                            format!(" ORDER BY {order}")
                        }
                    ),
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
            check.finish(&sources, profile)?;
            evidence.push(ValidationExecution {
                name: invariant.name.into(),
                revision: invariant.revision,
                definition: invariant.digest(),
                factory: self
                    .model
                    .factory_contract(invariant.identity())
                    .map(|factory| factory.identity())
                    .transpose()?,
                execution: Execution::WholeInput,
                inputs: selected,
            });
        }
        Ok(CheckedFrontier {
            checks: evidence,
            model: self.model.digest(),
            profile,
            frontier,
            _charge: charge,
        })
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
                    self.native
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
    async fn validate_references(
        &self,
        session: &SessionContext,
        _relations: &[Arc<ExactRelation>],
        inputs: &ValidationInputs,
        owned: Option<&ValidationSession>,
    ) -> Result<(), ModelError> {
        self.cancellation.check()?;
        if let Some(owned) = owned {
            // This call owns the reference phase. No field-level memo can
            // substitute for actual observations of every original source.
            owned.current(self, session).await?;
        }
        let mut pending = PendingReferencePremises::new(self.budget());
        let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
        let tables = inputs.reference_targets()?;
        let mut prepared =
            reference_preparation::PreparedReferenceTargets::new(self, session, owned);
        // Agreement is one relation-group operation, not one query per pair.
        // These private inputs retain full typed rows and never supply owner authority.
        let _groups = self.budget().reserve(
            "reference agreement groups",
            tables.len().saturating_mul(256).saturating_add(512),
        )?;
        let mut groups = BTreeMap::<&str, Vec<(&ContentHash, &(Relation, String))>>::new();
        for (identity, target) in &tables {
            groups
                .entry(target.0.name())
                .or_default()
                .push((identity, target));
        }
        for targets in groups.values() {
            let relation = &targets[0].1.0;
            if targets.len() < 2
                || relation
                    .schema()
                    .fields()
                    .iter()
                    .all(|field| field.name() == "id")
            {
                continue;
            }
            let mut local_tables = Vec::with_capacity(targets.len());
            for (identity, (other, table)) in targets {
                self.cancellation.check()?;
                if other.schema().as_ref() != relation.schema().as_ref() {
                    return Err(ModelError::Schema("reference agreement schema differs"));
                }
                if let Some(owned) = owned
                    && owned.earned_empty(self, session, identity).await?
                {
                    continue;
                }
                local_tables.push(prepared.input(**identity, other, table).await?);
            }
            // Original owners bracket the private preparation, including earned-empty inputs.
            if let Some(owned) = owned {
                for (identity, _) in targets {
                    owned.check_empty(self, session, identity).await?;
                }
            }
            let columns_bytes = relation
                .schema()
                .fields()
                .iter()
                .map(|field| field.name().len().saturating_mul(2).saturating_add(4))
                .sum::<usize>();
            let query_bytes = local_tables
                .iter()
                .map(|table| {
                    table
                        .len()
                        .saturating_mul(2)
                        .saturating_add(columns_bytes)
                        .saturating_add(64)
                })
                .sum::<usize>()
                .saturating_add(columns_bytes)
                .saturating_add(256);
            let _query = self.budget().reserve(
                "reference agreement query",
                query_bytes.saturating_mul(4).saturating_add(1024),
            )?;
            let Some(sql) = nominal_payload_conflict_sql(relation.schema(), &local_tables) else {
                continue;
            };
            let mut stream = crate::sql::query(prepared.session(), &sql)
                .await
                .map_err(crate::sql::model_error)?
                .execute_stream()
                .await
                .map_err(crate::sql::model_error)?;
            while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                if batch.num_rows() > 0 {
                    return Err(ModelError::Conflict(
                        "selected nominal reference payloads conflict",
                    ));
                }
            }
        }
        for (identity, (relation, source_table)) in &tables {
            // Actual current observations bracket this private phase. Every field
            // still checks its original local owners and schema obligations.
            let empty = if relation
                .fields()
                .iter()
                .any(|field| !field.list() && field.target().is_some())
            {
                match owned {
                    Some(owned) => owned.earned_empty(self, session, identity).await?,
                    None => false,
                }
            } else {
                false
            };
            for field in relation.fields().iter().filter(|field| !field.list()) {
                self.cancellation.check()?;
                let Some((_, target)) = field.target() else {
                    continue;
                };
                let targets = tables
                    .iter()
                    .filter(|(_, (relation, _))| relation.name() == target)
                    .collect::<Vec<_>>();
                let mut key = lctx_model::domain::KeySink::new("compiler-reference-premise/v1");
                key.part(b"model", &self.model.digest().0);
                key.part(b"source", &identity.0);
                key.part(b"field", field.name().as_bytes());
                for (identity, _) in &targets {
                    key.part(b"target", &identity.0);
                }
                if targets.is_empty() {
                    key.part(b"missing-target", target.as_bytes());
                }
                let premise = key.finish();
                // Resolve the same subtype schema obligation before proving a source
                // empty or consulting memoized execution. Empty rows cannot repair
                // an invalid declared reference target.
                let subtype = if let Some((_, (target_relation, _))) = targets.first() {
                    if let Some(tag) = field.subtype() {
                        let tag_field = target_relation
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
                    }
                } else {
                    String::new()
                };
                if let Some(owner) = owned {
                    owner.check_empty(self, session, identity).await?;
                    for (target_identity, _) in &targets {
                        owner.check_empty(self, session, target_identity).await?;
                    }
                }
                if empty {
                    pending.insert(premise)?;
                    continue;
                }
                if owned.is_some()
                    && (pending.contains(&premise)
                        || self
                            .memo
                            .checked
                            .lock()
                            .map_err(|_| poisoned())?
                            .0
                            .contains(&premise))
                {
                    // Local original-owner checks above remain mandatory even
                    // for old memo entries or facts staged earlier in this call.
                    continue;
                }
                let source_local = prepared.input(*identity, relation, source_table).await?;
                let source_name = quote(&source_local);
                let field_name = quote(field.name());
                let sql = if !targets.is_empty() {
                    let target_table = prepared.family(&targets).await?;
                    let target_tables = format!("SELECT * FROM {}", quote(&target_table));
                    format!(
                        "SELECT s.{field_name} FROM {source_name} s LEFT JOIN ({target_tables}) t ON s.{field_name}=t.id WHERE s.{field_name} IS NOT NULL AND (t.id IS NULL{subtype}) LIMIT 1"
                    )
                } else {
                    format!(
                        "SELECT {field_name} FROM {source_name} WHERE {field_name} IS NOT NULL LIMIT 1"
                    )
                };
                let mut stream = crate::sql::query(prepared.session(), &sql)
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
                            relation.name(),
                            field.name()
                        )));
                    }
                }
                if owned.is_some() {
                    pending.insert(premise)?;
                }
            }
        }
        #[cfg(test)]
        if let Ok(barrier) = REFERENCE_PHASE_BARRIER.try_with(Arc::clone) {
            barrier
                .pending
                .store(pending.values.len(), Ordering::Release);
            barrier.reached.wait().await;
            barrier.resume.wait().await;
        }
        if let Some(owned) = owned {
            // All SQL has reached EOF. A changed epoch/selector/hold or source
            // lifetime refuses the whole call before any new memo is visible.
            owned.current(self, session).await?;
        }
        self.cancellation.check()?;
        // Local file providers and their preparation charges end before later phases.
        drop(prepared);
        pending.commit(&self.memo)?;
        Ok(())
    }
}

/// Proved reference premises remain private until the complete call's terminal
/// observations succeed. Admission never exposes a charged prefix on refusal.
struct PendingReferencePremises {
    values: BTreeSet<ContentHash>,
    charge: StateCharge,
}
impl PendingReferencePremises {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            values: BTreeSet::new(),
            charge: StateCharge::new(budget, "pending reference premises"),
        }
    }
    fn contains(&self, value: &ContentHash) -> bool {
        self.values.contains(value)
    }
    fn insert(&mut self, value: ContentHash) -> Result<(), ModelError> {
        if !self.values.contains(&value) {
            self.charge.grow(128)?;
            self.values.insert(value);
        }
        Ok(())
    }
    fn commit(self, memo: &AdmissionMemo) -> Result<(), ModelError> {
        let mut checked = memo.checked.lock().map_err(|_| poisoned())?;
        let missing = self
            .values
            .iter()
            .filter(|value| !checked.0.contains(*value))
            .count();
        // Reserve the entire merge before its first insertion: resource refusal
        // and every preceding phase failure leave the shared memo unchanged.
        checked.1.grow(missing.saturating_mul(128))?;
        checked.0.extend(self.values);
        Ok(())
    }
}

#[cfg(test)]
struct ReferencePhaseBarrier {
    reached: tokio::sync::Barrier,
    resume: tokio::sync::Barrier,
    pending: std::sync::atomic::AtomicUsize,
}
#[cfg(test)]
tokio::task_local! {
    static REFERENCE_PHASE_BARRIER: Arc<ReferencePhaseBarrier>;
}

struct ValidationSession {
    context: SessionContext,
    native: Arc<NativeCompilerStore>,
    native_sources: BTreeMap<ContentHash, Vec<crate::compiler_provider::ReferenceSource>>,
    empty_companions: BTreeSet<ContentHash>,
    _charge: StateCharge,
}
impl std::ops::Deref for ValidationSession {
    type Target = SessionContext;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
// Provider Arcs are retained by native_sources throughout each current() call;
// their data addresses identify actual owners, never logical selector equality.
fn first_reference_provider(
    seen: &mut BTreeSet<usize>,
    provider: &Arc<dyn TableProvider>,
    charge: &mut StateCharge,
) -> Result<bool, ModelError> {
    let identity = Arc::as_ptr(provider) as *const () as usize;
    if seen.contains(&identity) {
        return Ok(false);
    }
    charge.grow(std::mem::size_of::<usize>().saturating_add(64))?;
    seen.insert(identity);
    Ok(true)
}
impl ValidationSession {
    fn require_owner(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
    ) -> Result<(), ModelError> {
        runtime.cancellation.check()?;
        if !Arc::ptr_eq(&self.native, &runtime.native) || !std::ptr::eq(&self.context, context) {
            return Err(ModelError::Conflict(
                "reference emptiness belongs to another session or native owner",
            ));
        }
        Ok(())
    }
    async fn check_empty(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
        identity: &ContentHash,
    ) -> Result<(), ModelError> {
        self.require_owner(runtime, context)?;
        if let Some(sources) = self.native_sources.get(identity) {
            for source in sources {
                source.check().await?;
            }
        }
        self.require_owner(runtime, context)?;
        Ok(())
    }
    async fn check_all(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
    ) -> Result<(), ModelError> {
        self.require_owner(runtime, context)?;
        for identity in self.native_sources.keys() {
            self.check_empty(runtime, context, identity).await?;
        }
        self.require_owner(runtime, context)
    }
    async fn current(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
    ) -> Result<(), ModelError> {
        self.require_owner(runtime, context)?;
        // Only the very same provider Arc shares this remote observation.
        // Equal selectors can retain distinct external producing owners, which
        // must each qualify independently. Every per-role local check remains.
        let mut charge =
            StateCharge::new(runtime.budget(), "reference current provider identities");
        let mut seen = BTreeSet::new();
        for sources in self.native_sources.values() {
            for source in sources {
                if first_reference_provider(&mut seen, &source.provider(), &mut charge)? {
                    source.current().await?;
                    self.require_owner(runtime, context)?;
                }
            }
        }
        self.check_all(runtime, context).await
    }
    async fn earned_empty(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
        identity: &ContentHash,
    ) -> Result<bool, ModelError> {
        self.require_owner(runtime, context)?;
        if let Some(sources) = self.native_sources.get(identity) {
            // Entry/terminal observations belong to validate_references. Within
            // that call, even nonempty memo inputs retain their original owners.
            for source in sources {
                source.check().await?;
            }
            runtime.cancellation.check()?;
            return Ok(sources.iter().all(|source| source.is_empty()));
        }
        Ok(self.empty_companions.contains(identity))
    }
}

#[derive(Clone)]
struct ValidationInputs {
    relations: BTreeMap<(&'static str, Option<PublicationBoundary>), ResolvedInput>,
    companions: BTreeMap<&'static str, CompanionTable>,
}
#[derive(Clone)]
struct CompanionTable {
    relation: Relation,
    batches: Vec<RecordBatch>,
    identity: ContentHash,
}
impl CompanionTable {
    fn table(&self) -> String {
        format!("_captured_reference_{}", self.relation.name())
    }
}
type ReferenceTargets = BTreeMap<ContentHash, (Relation, String)>;
impl ValidationInputs {
    fn reference_targets(&self) -> Result<ReferenceTargets, ModelError> {
        let mut tables = BTreeMap::<ContentHash, (Relation, String)>::new();
        let mut seen_views = BTreeMap::new();
        for ((name, prefix), source) in &self.relations {
            if seen_views
                .insert(source.view.identity, &source.view)
                .is_some_and(|prior| prior != &source.view)
            {
                return Err(ModelError::Conflict(
                    "reference target view descriptor collision",
                ));
            }
            tables.entry(source.view.identity).or_insert_with(|| {
                (
                    source.relation.clone(),
                    ValidationInputs::table(name, *prefix),
                )
            });
        }
        for companion in self.companions.values() {
            tables.insert(
                companion.identity,
                (companion.relation.clone(), companion.table()),
            );
        }
        Ok(tables)
    }

    fn relations(&self) -> impl Iterator<Item = &Arc<ExactRelation>> {
        self.relations.values().map(|r| &r.source)
    }
    fn private_session(&self, runtime: &AdmissionRuntime) -> ValidationSession {
        ValidationSession {
            context: SessionContext::new_with_config_rt(
                runtime.context.copied_config(),
                runtime.context.runtime_env(),
            ),
            native: runtime.native.clone(),
            native_sources: BTreeMap::new(),
            empty_companions: BTreeSet::new(),
            _charge: StateCharge::new(runtime.budget(), "private reference source handles"),
        }
    }
    async fn reference_session(
        &self,
        runtime: &AdmissionRuntime,
        external: &SessionContext,
    ) -> Result<Option<ValidationSession>, ModelError> {
        // Captured companions use the private inputs.session route. An external
        // companion registration cannot assert ownership of those captured rows.
        if !self.companions.is_empty() {
            return Ok(None);
        }
        let mut snapshot = self.private_session(runtime);
        for ((name, prefix), source) in &self.relations {
            runtime.cancellation.check()?;
            let alias = Self::table(name, *prefix);
            let provider = external
                .table_provider(alias.as_str())
                .await
                .map_err(ModelError::codec)?;
            let Some(actual) = crate::compiler_provider::reference_source(
                &provider,
                &runtime.native,
                &source.view,
                &source.relation,
                runtime.budget(),
            ) else {
                return Ok(None);
            };
            snapshot._charge.grow(
                alias
                    .len()
                    .saturating_add(
                        std::mem::size_of::<crate::compiler_provider::ReferenceSource>(),
                    )
                    .saturating_add(128),
            )?;
            snapshot
                .context
                .register_table(alias, actual.provider())
                .map_err(ModelError::codec)?;
            snapshot
                .native_sources
                .entry(source.view.identity)
                .or_default()
                .push(actual);
        }
        // Qualify every captured source and target, including nonempty memo inputs,
        // only after all aliases match. Closed/failed owner errors propagate instead
        // of silently converting an actual native source to finite external SQL.
        snapshot.current(runtime, &snapshot.context).await?;
        runtime.cancellation.check()?;
        Ok(Some(snapshot))
    }
    async fn session(&self, runtime: &AdmissionRuntime) -> Result<ValidationSession, ModelError> {
        let mut session = self.private_session(runtime);
        let mut bindings = self.relations.iter().collect::<Vec<_>>();
        bindings.sort_by_key(|(_, b)| b.role);
        for companion in self.companions.values() {
            session
                .register_table(
                    companion.table(),
                    Arc::new(
                        datafusion::datasource::MemTable::try_new(
                            companion.relation.schema().clone(),
                            vec![companion.batches.clone()],
                        )
                        .map_err(ModelError::codec)?,
                    ),
                )
                .map_err(ModelError::codec)?;
        }
        for companion in self.companions.values() {
            if companion.batches.iter().all(|batch| batch.num_rows() == 0) {
                session
                    ._charge
                    .grow(std::mem::size_of::<ContentHash>().saturating_add(64))?;
                session.empty_companions.insert(companion.identity);
            }
        }
        // Exact repeated views in this invocation share their already acquired
        // provider, while each semantic role still registers/qualifies separately.
        // Runtime fixes the actual native client, pool and admission context.
        let mut provider_charge =
            StateCharge::new(runtime.budget(), "private reference exact-view providers");
        let mut providers =
            BTreeMap::<ContentHash, (&ExactRelation, Arc<dyn TableProvider>)>::new();
        for ((name, prefix), source) in bindings {
            let provider = if let Some((previous, provider)) = providers.get(&source.view.identity)
            {
                if previous.view != source.view
                    || previous.relation.type_id() != source.relation.type_id()
                    || previous.relation.name() != source.relation.name()
                    || previous.relation.schema() != source.relation.schema()
                {
                    return Err(ModelError::Conflict(
                        "private reference view or relation descriptor collision",
                    ));
                }
                provider.clone()
            } else {
                provider_charge.grow(
                    std::mem::size_of::<(ContentHash, &ExactRelation, Arc<dyn TableProvider>)>()
                        .saturating_add(128),
                )?;
                let provider = runtime
                    .native
                    .table_provider(
                        &source.view,
                        source.relation.clone(),
                        runtime.budget.clone(),
                        runtime.options.batch_rows,
                    )
                    .await?;
                providers.insert(
                    source.view.identity,
                    (source.source.as_ref(), provider.clone()),
                );
                provider
            };
            let actual = crate::compiler_provider::reference_source(
                &provider,
                &runtime.native,
                &source.view,
                &source.relation,
                runtime.budget(),
            )
            .ok_or(ModelError::Conflict(
                "private reference provider differs from its input",
            ))?;
            session._charge.grow(
                std::mem::size_of::<crate::compiler_provider::ReferenceSource>()
                    .saturating_add(128),
            )?;
            session
                .register_table(Self::table(name, *prefix), provider)
                .map_err(ModelError::codec)?;
            session
                .native_sources
                .entry(source.view.identity)
                .or_default()
                .push(actual);
        }
        let names = self
            .relations
            .keys()
            .map(|(name, _)| *name)
            .collect::<BTreeSet<_>>();
        for name in names {
            if let Some((name, prefix)) = self.plain_binding(name) {
                // The qualified registration already owns this exact native capability.
                // Plain naming must neither reacquire it nor replace any original source.
                let provider = session
                    .context
                    .table_provider(Self::table(name, prefix))
                    .await
                    .map_err(ModelError::codec)?;
                session
                    .register_table(name, provider)
                    .map_err(ModelError::codec)?;
            }
        }
        Ok(session)
    }
    fn plain_binding(
        &self,
        name: &'static str,
    ) -> Option<(&'static str, Option<PublicationBoundary>)> {
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
    fn validation_source(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<&ResolvedInput, ModelError> {
        if let Some(source) = self.relations.get(&(input.name(), input.prefix())) {
            return Ok(source);
        }
        if input.prefix().is_some() {
            return Err(ModelError::Conflict(
                "checked admission requires a missing input epoch",
            ));
        }
        let mut choices = self
            .relations
            .iter()
            .filter(|((name, _), _)| *name == input.name())
            .map(|(_, source)| source);
        let source = choices
            .next()
            .ok_or(ModelError::Conflict("checked admission input is absent"))?;
        if choices.any(|other| {
            other.resolved != source.resolved || !Arc::ptr_eq(&source.source, &other.source)
        }) {
            return Err(ModelError::Conflict(
                "checked admission input epoch is ambiguous",
            ));
        }
        Ok(source)
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
        if let Some(companion) = self.companions.get(input.name()) {
            return Ok(companion.table());
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
}

impl ValidationPlanner for ValidationInputs {
    fn query<'a>(
        &'a self,
        session: &'a SessionContext,
        _input: &'a lctx_model::domain::ValidationInput,
        sql: &'a str,
    ) -> BoxFuture<'a, Result<datafusion::dataframe::DataFrame, ModelError>> {
        async move {
            crate::sql::query(session, sql)
                .await
                .map_err(crate::sql::model_error)
        }
        .boxed()
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

pub(crate) fn nominal_payload_conflict_sql(
    schema: &arrow_schema::Schema,
    tables: &[String],
) -> Option<String> {
    if tables.len() < 2 || schema.fields().iter().all(|field| field.name() == "id") {
        return None;
    }
    let quote = crate::consumed_rows::identifier;
    let columns = schema
        .fields()
        .iter()
        .map(|field| quote(field.name()))
        .collect::<Vec<_>>()
        .join(", ");
    let union = tables
        .iter()
        .map(|table| format!("SELECT {columns} FROM {}", quote(table)))
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    // Whole-row DISTINCT retains NULL as a value. Counting variants afterwards avoids
    // COUNT(DISTINCT payload)'s NULL omission and compares every complete typed column.
    Some(format!(
        "SELECT id FROM (SELECT DISTINCT {columns} FROM ({union}) AS selected_payloads) \
         AS payload_variants GROUP BY id HAVING COUNT(*) > 1 LIMIT 1"
    ))
}

fn poisoned() -> ModelError {
    ModelError::Invalid("admission runtime lock poisoned".into())
}

/// Resource access is independent of compilation and grants no admitted artifact construction.
pub trait AdmissionResources: Sized {
    fn admission_resources(self: &Arc<Self>) -> Result<Arc<AdmissionRuntime>, ModelError>;
}
impl AdmissionResources for AdmissionRuntime {
    fn admission_resources(self: &Arc<Self>) -> Result<Arc<AdmissionRuntime>, ModelError> {
        Ok(self.clone())
    }
}
/// Shared selected-domain preparation borrows the caller's existing submitted-call owner.
pub trait NativeRuntime: Send + Sync {
    fn budget(&self) -> &ResourceBudget;
    fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>>;
}
impl NativeRuntime for AdmissionRuntime {
    fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    fn native_call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        self.native_calls.call(future)
    }
}
impl<T: NativeRuntime> NativeRuntime for Arc<T> {
    fn budget(&self) -> &ResourceBudget {
        self.as_ref().budget()
    }
    fn native_call<R: Send + 'static>(
        &self,
        future: impl Future<Output = Result<R, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<R, ModelError>> {
        self.as_ref().native_call(future)
    }
}

#[cfg(test)]
mod reference_empty_controls {
    use super::*;
    use lctx_model::domain::{
        analysis::sources::SourceSnapshot,
        input::{Package, Release},
    };
    #[test]
    fn reference_premise_merge_is_charged_atomic_and_private_until_commit() {
        let scratch = ResourceBudget::fixed(4096).unwrap();
        let tiny = ResourceBudget::fixed(1).unwrap();
        let refused = AdmissionMemo::new(&tiny);
        let values = [ContentHash::of(b"first"), ContentHash::of(b"second")];
        let mut pending = PendingReferencePremises::new(&scratch);
        for value in values {
            pending.insert(value).unwrap();
        }
        pending.insert(values[0]).unwrap();
        assert!(refused.checked.lock().unwrap().0.is_empty());
        assert_eq!(
            pending.charge.reserved(),
            256,
            "duplicate premise keeps one admitted owner"
        );
        assert!(matches!(
            pending.commit(&refused),
            Err(ModelError::Resource { .. })
        ));
        assert!(
            refused.checked.lock().unwrap().0.is_empty(),
            "resource refusal publishes no prefix"
        );
        assert_eq!(tiny.reserved(), 0);
        assert_eq!(scratch.reserved(), 0);

        let backing = ResourceBudget::fixed(4096).unwrap();
        let memo = AdmissionMemo::new(&backing);
        let mut abandoned = PendingReferencePremises::new(&scratch);
        abandoned.insert(values[0]).unwrap();
        drop(abandoned);
        assert!(
            memo.checked.lock().unwrap().0.is_empty(),
            "abandoned phase leaves no reusable proof"
        );
        let mut pending = PendingReferencePremises::new(&scratch);
        for value in values {
            pending.insert(value).unwrap();
        }
        pending.commit(&memo).unwrap();
        assert_eq!(
            memo.checked.lock().unwrap().0,
            values.into_iter().collect::<BTreeSet<_>>()
        );
        assert_eq!(memo.checked.lock().unwrap().1.reserved(), 256);
        let retained = backing.reserved();
        let mut repeated = PendingReferencePremises::new(&scratch);
        repeated.insert(values[0]).unwrap();
        repeated.commit(&memo).unwrap();
        assert_eq!(
            backing.reserved(),
            retained,
            "existing proof retains its one charge"
        );
        assert_eq!(scratch.reserved(), 0);
    }
    // Independent oracle retained only in controls: the previous field-wise semantics.
    fn pair_payload_conflict_sql(schema: &arrow_schema::Schema, left: &str, right: &str) -> String {
        let quote = crate::consumed_rows::identifier;
        let differences = schema
            .fields()
            .iter()
            .filter(|field| field.name() != "id")
            .map(|field| {
                format!(
                    "(l.{} IS DISTINCT FROM r.{})",
                    quote(field.name()),
                    quote(field.name())
                )
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        format!(
            "SELECT l.id FROM {} l JOIN {} r ON l.id=r.id WHERE {differences} LIMIT 1",
            quote(left),
            quote(right)
        )
    }
    async fn has_payload_conflict(session: &SessionContext, sql: &str) -> bool {
        crate::sql::query(session, sql)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap()
            .iter()
            .any(|batch| batch.num_rows() != 0)
    }
    #[tokio::test]
    async fn grouped_payload_agreement_matches_typed_pair_oracle_for_nested_nulls_and_zero() {
        let session = SessionContext::new();
        let fixture = |score: &str, nested: &str, ordered: &str, optional: &str| {
            format!(
                "SELECT 1 AS id, CAST({score} AS DOUBLE) AS score, {nested} AS nested, \
             {ordered} AS ordered, CAST({optional} AS BIGINT) AS optional"
            )
        };
        let nested =
            "named_struct('name', 'value', 'items', make_array(1, CAST(NULL AS BIGINT), 2))";
        let ordered = "make_array(1, CAST(NULL AS BIGINT), 2)";
        let base = fixture("0.0", nested, ordered, "NULL");
        let batch = crate::sql::query(&session, &base)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap()
            .remove(0);
        let schema = batch.schema();
        session.register_batch("first", batch.clone()).unwrap();
        session.register_batch("duplicate", batch.clone()).unwrap();
        session
            .register_batch("empty", RecordBatch::new_empty(schema.clone()))
            .unwrap();
        // An ID absent from all other inputs is not a disagreement.
        let absent = crate::sql::query(&session, &base.replace("1 AS id", "2 AS id"))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap()
            .remove(0);
        session.register_batch("absent", absent).unwrap();
        let tables = ["first", "duplicate", "empty", "absent", "late"].map(String::from);
        for (query, expected) in [
            (base.clone(), false),
            (fixture("-0.0", nested, ordered, "NULL"), false),
            (fixture("0.0", nested, ordered, "7"), true),
            (
                fixture(
                    "0.0",
                    nested,
                    "make_array(2, CAST(NULL AS BIGINT), 1)",
                    "NULL",
                ),
                true,
            ),
            (
                fixture(
                    "0.0",
                    "named_struct('name', 'changed', 'items', make_array(1, CAST(NULL AS BIGINT), 2))",
                    ordered,
                    "NULL",
                ),
                true,
            ),
            (
                fixture(
                    "0.0",
                    "named_struct('name', 'value', 'items', make_array(1, 3, 2))",
                    ordered,
                    "NULL",
                ),
                true,
            ),
        ] {
            session.deregister_table("late").unwrap();
            let batch = crate::sql::query(&session, &query)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap()
                .remove(0);
            session.register_batch("late", batch).unwrap();
            let mut pairwise = false;
            for (position, left) in tables.iter().enumerate() {
                for right in tables.iter().skip(position + 1) {
                    pairwise |= has_payload_conflict(
                        &session,
                        &pair_payload_conflict_sql(&schema, left, right),
                    )
                    .await;
                }
            }
            let grouped = nominal_payload_conflict_sql(&schema, &tables).unwrap();
            assert!(
                !grouped.contains(" JOIN "),
                "one union/group operation replaces pair joins"
            );
            assert_eq!(
                has_payload_conflict(&session, &grouped).await,
                pairwise,
                "{query}"
            );
            assert_eq!(pairwise, expected, "explicit typed fixture: {query}");
        }
        // A NULL list/struct is distinct from a present container; two NULL containers
        // compare equal. Exercise validity, rather than only nullable child values.
        for field in ["nested", "ordered"] {
            let original = crate::sql::query(&session, &base)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap()
                .remove(0);
            let index = schema.index_of(field).unwrap();
            let nullable_schema = Arc::new(arrow_schema::Schema::new(
                schema
                    .fields()
                    .iter()
                    .enumerate()
                    .map(|(position, field)| {
                        field
                            .as_ref()
                            .clone()
                            .with_nullable(position == index || field.is_nullable())
                    })
                    .collect::<Vec<_>>(),
            ));
            let mut columns = original.columns().to_vec();
            columns[index] = arrow_array::new_null_array(schema.field(index).data_type(), 1);
            let null_container = RecordBatch::try_new(nullable_schema, columns).unwrap();
            session.deregister_table("first").unwrap();
            session.deregister_table("duplicate").unwrap();
            session.register_batch("first", original.clone()).unwrap();
            session.register_batch("duplicate", original).unwrap();
            session.deregister_table("late").unwrap();
            session
                .register_batch("late", null_container.clone())
                .unwrap();
            let grouped = nominal_payload_conflict_sql(&schema, &tables).unwrap();
            let pair = pair_payload_conflict_sql(&schema, "first", "late");
            assert!(
                has_payload_conflict(&session, &pair).await,
                "NULL {field} differs from present"
            );
            assert!(
                has_payload_conflict(&session, &grouped).await,
                "group retains NULL {field} variant"
            );
            for table in ["first", "duplicate"] {
                session.deregister_table(table).unwrap();
                session
                    .register_batch(table, null_container.clone())
                    .unwrap();
            }
            assert!(
                !has_payload_conflict(&session, &pair).await,
                "NULL {field} equals NULL"
            );
            assert!(
                !has_payload_conflict(&session, &grouped).await,
                "group agrees on NULL {field}"
            );
        }
        assert!(nominal_payload_conflict_sql(&schema, &["empty".into()]).is_none());
    }
    #[tokio::test]
    async fn grouped_payload_agreement_preserves_nested_float_total_order_with_null_children() {
        use arrow_array::builder::{Float64Builder, ListBuilder};
        use arrow_array::{ArrayRef, Float64Array, Int64Array, StructArray};
        use arrow_schema::{DataType, Field, Schema};
        let batch = |list_zero: f64, struct_zero: f64, deep_zero: f64| {
            let list = |zero| {
                let mut list = ListBuilder::new(Float64Builder::new());
                list.values().append_value(zero);
                list.values().append_null();
                list.append(true);
                Arc::new(list.finish()) as ArrayRef
            };
            let ordered = list(list_zero);
            let nested = Arc::new(StructArray::from(vec![
                (
                    Arc::new(Field::new("value", DataType::Float64, false)),
                    Arc::new(Float64Array::from(vec![struct_zero])) as ArrayRef,
                ),
                (
                    Arc::new(Field::new("missing", DataType::Float64, true)),
                    Arc::new(Float64Array::from(vec![None::<f64>])) as ArrayRef,
                ),
            ])) as ArrayRef;
            let deep_items = list(deep_zero);
            let deep = Arc::new(StructArray::from(vec![(
                Arc::new(Field::new("items", deep_items.data_type().clone(), false)),
                deep_items,
            )])) as ArrayRef;
            let columns = vec![
                Arc::new(Int64Array::from(vec![1])) as ArrayRef,
                ordered,
                nested,
                deep,
            ];
            let schema = Arc::new(Schema::new(
                ["id", "ordered", "nested", "deep"]
                    .into_iter()
                    .zip(&columns)
                    .map(|(name, column)| Field::new(name, column.data_type().clone(), false))
                    .collect::<Vec<_>>(),
            ));
            RecordBatch::try_new(schema, columns).unwrap()
        };
        let session = SessionContext::new();
        let first = batch(0.0, 0.0, 0.0);
        let schema = first.schema();
        session.register_batch("first", first.clone()).unwrap();
        session.register_batch("duplicate", first).unwrap();
        let tables = ["first", "duplicate", "late"].map(String::from);
        // Construct arrays directly so negative-zero bits cannot be erased by SQL
        // constant folding. Pinned nested IS DISTINCT FROM uses totalOrder, unlike
        // its top-level float path, and grouping must preserve that distinction.
        for (late, expected) in [
            (batch(0.0, 0.0, 0.0), false),
            (batch(-0.0, 0.0, 0.0), true),
            (batch(0.0, -0.0, 0.0), true),
            (batch(0.0, 0.0, -0.0), true),
        ] {
            session.deregister_table("late").unwrap();
            session.register_batch("late", late).unwrap();
            let pair = pair_payload_conflict_sql(&schema, "first", "late");
            let grouped = nominal_payload_conflict_sql(&schema, &tables).unwrap();
            let original = has_payload_conflict(&session, &pair).await;
            assert_eq!(original, expected, "nested NULL/float comparator contract");
            assert_eq!(
                has_payload_conflict(&session, &grouped).await,
                original,
                "grouping retains exact nested comparator semantics"
            );
        }
    }
    #[tokio::test]
    async fn nominal_payload_comparisons_group_binary_boolean_and_nullable_predicates() {
        use arrow_array::{Array, BooleanArray, FixedSizeBinaryArray};
        use arrow_schema::{DataType, Field, Schema};
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::FixedSizeBinary(16), false),
            Field::new("payload", DataType::FixedSizeBinary(16), false),
            Field::new("active", DataType::Boolean, false),
            Field::new("nullable", DataType::FixedSizeBinary(16), true),
        ]));
        let batch = |payload: [[u8; 16]; 2], active: [bool; 2], nullable: [Option<[u8; 16]>; 2]| {
            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(
                        FixedSizeBinaryArray::try_from_iter([[1u8; 16], [2u8; 16]].into_iter())
                            .unwrap(),
                    ),
                    Arc::new(FixedSizeBinaryArray::try_from_iter(payload.into_iter()).unwrap()),
                    Arc::new(BooleanArray::from(active.to_vec())),
                    Arc::new(
                        FixedSizeBinaryArray::try_from_sparse_iter_with_size(
                            nullable.into_iter(),
                            16,
                        )
                        .unwrap(),
                    ),
                ],
            )
            .unwrap()
        };
        let session = SessionContext::new();
        let left = "left\"input";
        let right = "right-input";
        session
            .register_batch(left, batch([[7; 16]; 2], [true; 2], [Some([9; 16]), None]))
            .unwrap();
        let sql = nominal_payload_conflict_sql(&schema, &[left.into(), right.into()]).unwrap();
        for (payload, active, nullable, expected) in [
            ([[7; 16]; 2], [true; 2], [Some([9; 16]), None], None),
            (
                [[8; 16], [7; 16]],
                [true; 2],
                [Some([9; 16]), None],
                Some([1u8; 16]),
            ),
            (
                [[7; 16]; 2],
                [false, true],
                [Some([9; 16]), None],
                Some([1; 16]),
            ),
            ([[7; 16]; 2], [true; 2], [None, None], Some([1; 16])),
            (
                [[7; 16]; 2],
                [true; 2],
                [Some([9; 16]), Some([9; 16])],
                Some([2; 16]),
            ),
        ] {
            session.deregister_table(right).unwrap();
            session
                .register_batch(right, batch(payload, active, nullable))
                .unwrap();
            let results = crate::sql::query(&session, &sql)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            let found = results
                .iter()
                .flat_map(|batch| {
                    let ids = batch
                        .column(0)
                        .as_any()
                        .downcast_ref::<FixedSizeBinaryArray>()
                        .unwrap();
                    (0..ids.len())
                        .map(|row| ids.value(row).to_vec())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                found,
                expected
                    .into_iter()
                    .map(|id| id.to_vec())
                    .collect::<Vec<_>>(),
                "nominal payload differences retain binary, boolean and NULL semantics"
            );
        }
        assert!(
            nominal_payload_conflict_sql(
                &Schema::new(vec![Field::new("id", DataType::FixedSizeBinary(16), false)]),
                &[left.into(), right.into()]
            )
            .is_none(),
            "an identity-only relation has no payload to compare"
        );
    }
    fn runtime(
        model: Arc<ValidatedModel>,
        native: Option<Arc<NativeCompilerStore>>,
    ) -> Arc<AdmissionRuntime> {
        let native = native.unwrap_or_else(|| {
            NativeCompilerStore::from_existing(
                Arc::new(lctx_native::surrealdb::Surreal::init()),
                lctx_model::domain::serving::Name::new("reference_empty_control").unwrap(),
                lctx_model::domain::serving::Name::new("reference_empty_control").unwrap(),
            )
        });
        AdmissionRuntime::with_budget(
            model,
            AdmissionOptions::default(),
            ResourceBudget::fixed(AdmissionOptions::default().memory_bytes).unwrap(),
            native,
        )
        .unwrap()
    }
    #[derive(Debug)]
    struct CountingReferenceTable {
        inner: datafusion::datasource::MemTable,
        scans: Arc<std::sync::atomic::AtomicUsize>,
    }
    #[async_trait::async_trait]
    impl datafusion::catalog::TableProvider for CountingReferenceTable {
        fn schema(&self) -> arrow_schema::SchemaRef {
            datafusion::catalog::TableProvider::schema(&self.inner)
        }
        fn table_type(&self) -> datafusion::logical_expr::TableType {
            datafusion::logical_expr::TableType::Base
        }
        async fn scan(
            &self,
            state: &dyn datafusion::catalog::Session,
            projection: Option<&Vec<usize>>,
            filters: &[datafusion::logical_expr::Expr],
            limit: Option<usize>,
        ) -> datafusion::error::Result<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
            self.scans.fetch_add(1, Ordering::SeqCst);
            datafusion::catalog::TableProvider::scan(&self.inner, state, projection, filters, limit)
                .await
        }
    }
    #[derive(Debug)]
    struct LateFailureReferenceTable(RecordBatch);
    #[derive(Debug)]
    struct LateFailureReferencePartition(RecordBatch);
    impl datafusion::physical_plan::streaming::PartitionStream for LateFailureReferencePartition {
        fn schema(&self) -> &arrow_schema::SchemaRef {
            self.0.schema_ref()
        }
        fn execute(
            &self,
            _: Arc<datafusion::execution::TaskContext>,
        ) -> datafusion::physical_plan::SendableRecordBatchStream {
            Box::pin(
                datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(
                    self.0.schema(),
                    futures::stream::iter([
                        Ok(self.0.clone()),
                        Err(datafusion::error::DataFusionError::Execution(
                            "late reference source EOF failure".into(),
                        )),
                    ]),
                ),
            )
        }
    }
    #[async_trait::async_trait]
    impl datafusion::catalog::TableProvider for LateFailureReferenceTable {
        fn schema(&self) -> arrow_schema::SchemaRef {
            self.0.schema()
        }
        fn table_type(&self) -> datafusion::logical_expr::TableType {
            datafusion::logical_expr::TableType::Base
        }
        async fn scan(
            &self,
            _: &dyn datafusion::catalog::Session,
            projection: Option<&Vec<usize>>,
            _: &[datafusion::logical_expr::Expr],
            _: Option<usize>,
        ) -> datafusion::error::Result<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
            Ok(Arc::new(
                datafusion::physical_plan::streaming::StreamingTableExec::try_new(
                    self.0.schema(),
                    vec![Arc::new(LateFailureReferencePartition(self.0.clone()))],
                    projection,
                    vec![],
                    false,
                    None,
                )?,
            ))
        }
    }
    #[tokio::test]
    async fn prepared_reference_input_requires_eof_and_releases_failed_preparation() {
        let relation = Relation::of::<Package>();
        let runtime = runtime(
            Arc::new(ValidatedModel::declared(vec![relation.clone()]).unwrap()),
            None,
        );
        let row = Package {
            name: "valid-prefix".into(),
        };
        let batch = Package::encode(&[row]).unwrap();
        let session = SessionContext::new();
        session
            .register_table(
                "late_failure",
                Arc::new(LateFailureReferenceTable(batch.clone())),
            )
            .unwrap();
        let baseline = runtime.budget().reserved();
        let mut prepared =
            reference_preparation::PreparedReferenceTargets::new(&runtime, &session, None);
        assert_eq!(
            runtime.budget().reserved(),
            baseline,
            "an unused reference preparation is allocation-free"
        );
        let identity = ContentHash::of(b"late-reference-input");
        let error = prepared
            .input(identity, &relation, "late_failure")
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("late reference source EOF failure")
        );
        assert!(runtime.memo.checked.lock().unwrap().0.is_empty());
        // A failed stream cannot register its prefix as a prepared exact input.
        session.deregister_table("late_failure").unwrap();
        session.register_batch("late_failure", batch).unwrap();
        let local = prepared
            .input(identity, &relation, "late_failure")
            .await
            .unwrap();
        let rows = crate::sql::query(
            prepared.session(),
            &format!("SELECT * FROM {}", crate::consumed_rows::identifier(&local)),
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
        assert_eq!(rows.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
        drop(rows);
        drop(prepared);
        assert_eq!(
            runtime.budget().reserved(),
            baseline,
            "failed and successful private IPC owners release together"
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[derive(
        Debug, Clone, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "reference_pair_sources")]
    struct ReferencePair {
        #[model(key)]
        key: i64,
        first: lctx_model::domain::Id<Package>,
        second: lctx_model::domain::Id<Package>,
    }
    #[derive(
        Debug, Clone, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "reference_full_payloads")]
    struct ReferencePayload {
        #[model(key)]
        key: i64,
        payload: lctx_model::domain::Id<Package>,
        active: bool,
        nullable: Option<lctx_model::domain::Id<Package>>,
        members: Vec<i64>,
    }
    #[tokio::test]
    async fn prepared_reference_conflicts_preserve_complete_typed_and_null_payloads() {
        let relation = Relation::of::<ReferencePayload>();
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), relation.clone()]).unwrap(),
        );
        let runtime = runtime(model, None);
        let first = Package {
            name: "first".into(),
        }
        .id();
        let second = Package {
            name: "second".into(),
        }
        .id();
        let row = ReferencePayload {
            key: 1,
            payload: first,
            active: true,
            nullable: Some(first),
            members: vec![1, 2],
        };
        let mut variants = vec![row.clone()];
        let mut changed = row.clone();
        changed.payload = second;
        variants.push(changed);
        let mut changed = row.clone();
        changed.active = false;
        variants.push(changed);
        let mut changed = row.clone();
        changed.nullable = None;
        variants.push(changed);
        let mut changed = row.clone();
        changed.members.reverse();
        variants.push(changed);
        let session = SessionContext::new();
        session
            .register_batch(
                "left_payload",
                ReferencePayload::encode(std::slice::from_ref(&row)).unwrap(),
            )
            .unwrap();
        let mut prepared =
            reference_preparation::PreparedReferenceTargets::new(&runtime, &session, None);
        let left = prepared
            .input(ContentHash::of(b"left-payload"), &relation, "left_payload")
            .await
            .unwrap();
        for (index, variant) in variants.into_iter().enumerate() {
            session.deregister_table("right_payload").unwrap();
            session
                .register_batch(
                    "right_payload",
                    ReferencePayload::encode(&[variant]).unwrap(),
                )
                .unwrap();
            let right = prepared
                .input(
                    ContentHash::of(format!("right-payload-{index}").as_bytes()),
                    &relation,
                    "right_payload",
                )
                .await
                .unwrap();
            let sql =
                nominal_payload_conflict_sql(relation.schema(), &[left.clone(), right]).unwrap();
            let batches = crate::sql::query(prepared.session(), &sql)
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            let rows = batches.iter().map(RecordBatch::num_rows).sum::<usize>();
            assert_eq!(
                rows,
                usize::from(index != 0),
                "typed payload variant {index} preserves complete comparison including list order"
            );
        }
        drop(prepared);
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[tokio::test]
    async fn grouped_reference_admission_reads_three_owners_once_and_refuses_late_conflict() {
        let relation = Relation::of::<ReferencePayload>();
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), relation.clone()]).unwrap(),
        );
        let runtime = runtime(model, None);
        let package = Package {
            name: "target".into(),
        };
        let row = ReferencePayload {
            key: 1,
            payload: package.id(),
            active: true,
            nullable: None,
            members: vec![1, 2],
        };
        let mut inputs = selected(
            &runtime,
            vec![
                CompletedView::new(
                    ReferencePayload::NAME.into(),
                    [ContentHash::of(b"first")].into(),
                    1,
                )
                .unwrap(),
            ],
        );
        for (boundary, seed) in [
            (PublicationBoundary::Facts, b"second".as_slice()),
            (PublicationBoundary::Dispatch, b"third".as_slice()),
        ] {
            let mut later = selected(
                &runtime,
                vec![
                    CompletedView::new(
                        ReferencePayload::NAME.into(),
                        [ContentHash::of(seed)].into(),
                        1,
                    )
                    .unwrap(),
                ],
            );
            inputs.relations.insert(
                (ReferencePayload::NAME, Some(boundary)),
                later.relations.pop_first().unwrap().1,
            );
        }
        inputs
            .companions
            .insert(Package::NAME, companion(&[package]));
        for conflict in [false, true] {
            let session = SessionContext::new_with_config_rt(
                runtime.context.copied_config(),
                runtime.context.runtime_env(),
            );
            let mut counters = Vec::new();
            for ((name, prefix), selected) in &inputs.relations {
                let mut selected_row = row.clone();
                if conflict && *prefix == Some(PublicationBoundary::Dispatch) {
                    selected_row.members.reverse();
                }
                let scans = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                session
                    .register_table(
                        ValidationInputs::table(name, *prefix),
                        Arc::new(CountingReferenceTable {
                            inner: datafusion::datasource::MemTable::try_new(
                                selected.relation.schema().clone(),
                                vec![vec![ReferencePayload::encode(&[selected_row]).unwrap()]],
                            )
                            .unwrap(),
                            scans: scans.clone(),
                        }),
                    )
                    .unwrap();
                counters.push(scans);
            }
            for table in inputs.companions.values() {
                session
                    .register_table(
                        table.table(),
                        Arc::new(
                            datafusion::datasource::MemTable::try_new(
                                table.relation.schema().clone(),
                                vec![table.batches.clone()],
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap();
            }
            let result = runtime
                .validate_references(&session, &[], &inputs, None)
                .await;
            if conflict {
                assert!(matches!(
                    result,
                    Err(ModelError::Conflict(
                        "selected nominal reference payloads conflict"
                    ))
                ));
            } else {
                result.unwrap();
            }
            assert!(
                counters
                    .iter()
                    .all(|counter| counter.load(Ordering::SeqCst) == 1),
                "agreement and downstream references share each original acquisition"
            );
            assert!(
                runtime.memo.checked.lock().unwrap().0.is_empty(),
                "receiving preparation publishes no portable proof"
            );
        }
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[tokio::test]
    async fn prepared_references_read_originals_once_across_fields_and_target_sets() {
        let model = Arc::new(
            ValidatedModel::declared(vec![
                Relation::of::<Package>(),
                Relation::of::<ReferencePair>(),
            ])
            .unwrap(),
        );
        let runtime = runtime(model, None);
        let package = Package {
            name: "same-package".into(),
        };
        let pair = ReferencePair {
            key: 1,
            first: package.id(),
            second: package.id(),
        };
        let first = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(b"first-package-copy")].into(),
            1,
        )
        .unwrap();
        let second = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(b"second-package-copy")].into(),
            1,
        )
        .unwrap();
        let source = CompletedView::new(
            ReferencePair::NAME.into(),
            [ContentHash::of(b"reference-pair-copy")].into(),
            1,
        )
        .unwrap();
        let mut inputs = selected(&runtime, vec![first.clone(), source]);
        let mut later = selected(&runtime, vec![second.clone()]);
        inputs.relations.insert(
            (Package::NAME, Some(PublicationBoundary::Facts)),
            later.relations.pop_first().unwrap().1,
        );
        let session = SessionContext::new_with_config_rt(
            runtime.context.copied_config(),
            runtime.context.runtime_env(),
        );
        let mut counters = Vec::new();
        for ((name, prefix), selected) in &inputs.relations {
            let batch = if *name == Package::NAME {
                Package::encode(std::slice::from_ref(&package)).unwrap()
            } else {
                ReferencePair::encode(std::slice::from_ref(&pair)).unwrap()
            };
            let scans = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            session
                .register_table(
                    ValidationInputs::table(name, *prefix),
                    Arc::new(CountingReferenceTable {
                        inner: datafusion::datasource::MemTable::try_new(
                            selected.relation.schema().clone(),
                            vec![vec![batch]],
                        )
                        .unwrap(),
                        scans: scans.clone(),
                    }),
                )
                .unwrap();
            counters.push(scans);
        }
        runtime
            .validate_references(&session, &[], &inputs, None)
            .await
            .unwrap();
        assert!(
            counters
                .iter()
                .all(|count| count.load(Ordering::SeqCst) == 1),
            "full-payload comparisons and both reference fields share one original scan"
        );
        assert!(
            runtime.memo.checked.lock().unwrap().0.is_empty(),
            "preparation supplies no portable success"
        );
        let baseline = runtime.budget().reserved();
        let tables = inputs.reference_targets().unwrap();
        let all = tables
            .iter()
            .filter(|(_, (relation, _))| relation.name() == Package::NAME)
            .collect::<Vec<_>>();
        let mut prepared =
            reference_preparation::PreparedReferenceTargets::new(&runtime, &session, None);
        let one = prepared.family(&all[..1]).await.unwrap();
        let both = prepared.family(&all).await.unwrap();
        assert_ne!(
            one, both,
            "different ordered target sets do not share preparation"
        );
        let retained = runtime.budget().reserved();
        assert_eq!(both, prepared.family(&all).await.unwrap());
        assert_eq!(
            runtime.budget().reserved(),
            retained,
            "a reused family retains no extra metadata or lookup grant"
        );
        assert!(
            counters
                .iter()
                .all(|count| count.load(Ordering::SeqCst) <= 2)
        );
        drop(prepared);
        assert_eq!(
            runtime.budget().reserved(),
            baseline,
            "private IPC grants end with the invocation"
        );
        runtime.cancellation.cancel();
        assert!(
            runtime
                .validate_references(&session, &[], &inputs, None)
                .await
                .is_err()
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[tokio::test]
    async fn reference_target_preparation_preserves_roles_epochs_and_descriptor_refusal() {
        let model = Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap());
        let runtime = runtime(model, None);
        let first = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(b"first-owner")].into(),
            1,
        )
        .unwrap();
        let later = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(b"later-owner")].into(),
            1,
        )
        .unwrap();
        let mut inputs = selected(&runtime, vec![first]);
        let source = inputs.relations.values().next().unwrap().clone();
        inputs.relations.insert(
            (Package::NAME, Some(PublicationBoundary::Facts)),
            source.clone(),
        );
        assert_eq!(
            inputs.relations.len(),
            2,
            "both semantic roles remain selected"
        );
        assert_eq!(
            inputs.reference_targets().unwrap().len(),
            1,
            "one exact view supplies one target table"
        );
        assert_eq!(
            inputs.plain_binding(Package::NAME),
            Some((Package::NAME, None))
        );
        inputs
            .relations
            .get_mut(&(Package::NAME, Some(PublicationBoundary::Facts)))
            .unwrap()
            .resolved = Some(PublicationBoundary::Facts);
        assert!(
            inputs.plain_binding(Package::NAME).is_none(),
            "distinct resolved epochs remain ambiguous"
        );
        inputs
            .relations
            .get_mut(&(Package::NAME, Some(PublicationBoundary::Facts)))
            .unwrap()
            .resolved = None;
        let mut distinct = selected(&runtime, vec![later]);
        inputs.relations.insert(
            (Package::NAME, Some(PublicationBoundary::CatalogCore)),
            distinct.relations.pop_first().unwrap().1,
        );
        assert_eq!(
            inputs.reference_targets().unwrap().len(),
            2,
            "different epochs retain independent target branches"
        );
        assert!(
            inputs.plain_binding(Package::NAME).is_none(),
            "equal resolved epochs cannot alias distinct views"
        );
        let mut collision = source;
        let mut view = collision.view.clone();
        view.rows += 1;
        collision.source = Arc::new(ExactRelation {
            relation: collision.relation.clone(),
            view,
            snapshot: collision.snapshot.clone(),
        });
        inputs.relations.insert(
            (Package::NAME, Some(PublicationBoundary::Dispatch)),
            collision,
        );
        assert!(matches!(
            inputs.reference_targets(),
            Err(ModelError::Conflict(
                "reference target view descriptor collision"
            ))
        ));
        inputs.relations.clear();
        assert!(
            inputs.reference_targets().unwrap().is_empty(),
            "absent authority stays absent"
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    fn companion<R: Record>(rows: &[R]) -> CompanionTable {
        let mut identity = lctx_model::domain::KeySink::new("reference-empty-control-companion");
        identity.part(b"relation", R::NAME.as_bytes());
        for row in rows {
            identity.part(b"content", &row.content_digest().0);
        }
        CompanionTable {
            relation: Relation::of::<R>(),
            batches: vec![R::encode(rows).unwrap()],
            identity: identity.finish(),
        }
    }
    fn selected(runtime: &AdmissionRuntime, views: Vec<CompletedView>) -> ValidationInputs {
        let bindings = views
            .into_iter()
            .enumerate()
            .map(|(role, view)| {
                let relation = runtime.model.relation(&view.relation).unwrap();
                SelectedBinding {
                    role,
                    requested: None,
                    resolved: None,
                    binding: CompletedBinding {
                        source: SourceSnapshot::of_completed_view(
                            relation,
                            runtime.model.digest(),
                            &view,
                        )
                        .unwrap(),
                        view,
                        boundary: None,
                        configuration: None,
                    },
                }
            })
            .collect();
        runtime.selected(bindings).unwrap()
    }
    #[tokio::test]
    async fn forged_external_zero_row_selection_cannot_hide_actual_dangling_rows() {
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let runtime = runtime(model, None);
        let views = [Package::NAME, Release::NAME].map(|name| {
            CompletedView::new(
                name.into(),
                [ContentHash::of(b"unearned-reference-control-contributor")].into(),
                0,
            )
            .unwrap()
        });
        let inputs = selected(&runtime, views.to_vec());
        let package = Package {
            name: "missing-package".into(),
        };
        let dangling = Release {
            package: package.id(),
            version: "1.0".into(),
        };
        let session = SessionContext::new();
        session
            .register_batch(
                ValidationInputs::table(Package::NAME, None),
                Package::encode(&[]).unwrap(),
            )
            .unwrap();
        session
            .register_batch(
                ValidationInputs::table(Release::NAME, None),
                Release::encode(&[dangling]).unwrap(),
            )
            .unwrap();
        let bindings = inputs
            .relations
            .values()
            .map(|source| SelectedBinding {
                role: source.role,
                requested: source.requested,
                resolved: source.resolved,
                binding: CompletedBinding {
                    source: source.source.snapshot(),
                    view: source.view.clone(),
                    boundary: None,
                    configuration: None,
                },
            })
            .collect();
        let error = runtime
            .validate_compiler_selection(Profile::Catalog, false, bindings, &session, &inputs)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("missing or wrong-subtype nominal reference"),
            "actual dangling row must be read: {error}"
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[tokio::test]
    async fn captured_empty_reference_premises_bind_all_targets_and_refuse_cancelled_or_foreign_sessions()
     {
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let runtime = runtime(model.clone(), None);
        let mut inputs = ValidationInputs {
            relations: BTreeMap::new(),
            companions: [
                (Package::NAME, companion::<Package>(&[])),
                (Release::NAME, companion::<Release>(&[])),
            ]
            .into(),
        };
        let session = inputs.session(&runtime).await.unwrap();
        runtime
            .validate_references(&session, &[], &inputs, Some(&session))
            .await
            .unwrap();
        let initial = runtime.memo.checked.lock().unwrap().0.len();
        assert!(initial > 0);
        inputs.companions.insert(
            Package::NAME,
            companion(&[Package {
                name: "present-package".into(),
            }]),
        );
        let changed = inputs.session(&runtime).await.unwrap();
        runtime
            .validate_references(&changed, &[], &inputs, Some(&changed))
            .await
            .unwrap();
        assert_eq!(
            runtime.memo.checked.lock().unwrap().0.len(),
            initial + 1,
            "empty target identities remain in the premise"
        );
        inputs.companions.remove(Package::NAME);
        let missing = inputs.session(&runtime).await.unwrap();
        runtime
            .validate_references(&missing, &[], &inputs, Some(&missing))
            .await
            .unwrap();
        assert_eq!(
            runtime.memo.checked.lock().unwrap().0.len(),
            initial + 2,
            "missing-target identity remains distinct"
        );
        let foreign = SessionContext::new();
        assert!(
            runtime
                .validate_references(&foreign, &[], &inputs, Some(&missing))
                .await
                .is_err()
        );
        let other = self::runtime(model, None);
        assert!(
            other
                .validate_references(&missing, &[], &inputs, Some(&missing))
                .await
                .is_err()
        );
        runtime.cancellation.cancel();
        assert!(
            runtime
                .validate_references(&missing, &[], &inputs, Some(&missing))
                .await
                .is_err()
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
        lctx_model::domain::completion::complete(Ok(()), other.drain_report().await).unwrap();
    }
    #[derive(
        Debug, Clone, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "reference_subtype_source")]
    struct SubtypeSource {
        #[model(key)]
        key: i64,
        value: lctx_model::domain::ArmId<lctx_model::domain::value::Literal, 0>,
    }
    #[derive(
        Debug, Clone, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "literal_values")]
    struct NonSumTarget {
        #[model(key)]
        key: i64,
    }
    #[derive(
        Debug, Clone, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "packages")]
    struct DifferentPackageType {
        #[model(key)]
        name: String,
    }
    #[tokio::test]
    async fn empty_reference_source_still_rejects_nonsum_subtype_target() {
        let model = Arc::new(
            ValidatedModel::declared(vec![
                Relation::of::<lctx_model::domain::value::Literal>(),
                Relation::of::<SubtypeSource>(),
            ])
            .unwrap(),
        );
        let runtime = runtime(model, None);
        let inputs = ValidationInputs {
            relations: BTreeMap::new(),
            companions: [
                (SubtypeSource::NAME, companion::<SubtypeSource>(&[])),
                (NonSumTarget::NAME, companion::<NonSumTarget>(&[])),
            ]
            .into(),
        };
        let session = inputs.session(&runtime).await.unwrap();
        assert!(matches!(
            runtime
                .validate_references(&session, &[], &inputs, Some(&session))
                .await,
            Err(ModelError::Schema("nominal subtype target"))
        ));
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
    #[tokio::test]
    async fn owned_native_empty_reference_proof_refuses_failed_source_even_after_memo_success() {
        use lctx_model::domain::{
            completed::{ContributionSpec, OutputProfile},
            stages::ProviderOutcome,
        };
        let config = lctx_native::RuntimeConfig::read(&std::path::PathBuf::from(
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("explicit owned validation fixture"),
        ))
        .unwrap();
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let native = NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &crate::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let runtime = runtime(model.clone(), Some(native.clone()));
        let outcome = std::panic::AssertUnwindSafe(async {
            let relations = [Relation::of::<Package>(), Relation::of::<Release>()];
            let pending = native
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: "empty-reference-control".into(),
                    profile: OutputProfile::Specific(Profile::Catalog),
                    model: model.digest(),
                    implementation: ContentHash::of(b"empty-reference-control/v1"),
                    configuration: None,
                    inputs: vec![],
                    outputs: relations
                        .iter()
                        .map(|relation| relation.name().into())
                        .collect(),
                })
                .await
                .unwrap();
            native
                .write_batch(
                    &pending,
                    &relations[0],
                    &Package::encode(&[]).unwrap(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            native
                .write_batch(
                    &pending,
                    &relations[1],
                    &Release::encode(&[]).unwrap(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            let views = native
                .complete_contribution(
                    pending,
                    ProviderOutcome::Complete,
                    &relations,
                    &BTreeMap::new(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            let mut inputs = selected(&runtime, views.into_values().collect());
            let mut package_role=inputs.relations.get(&(Package::NAME,None)).unwrap().clone();
            package_role.role=inputs.relations.len();
            package_role.requested=Some(PublicationBoundary::Facts);
            inputs.relations.insert((Package::NAME,Some(PublicationBoundary::Facts)),package_role);
            let session = inputs.session(&runtime).await.unwrap();
            assert_eq!(session.native_sources.len(), 2);
            let base=session.table_provider(ValidationInputs::table(Package::NAME,None)).await.unwrap();
            let alias=session.table_provider(ValidationInputs::table(Package::NAME,Some(PublicationBoundary::Facts))).await.unwrap();
            assert!(Arc::ptr_eq(&base,&alias),"same exact private view shares its actual provider across requested roles");
            assert_eq!(session.native_sources.get(&inputs.relations[&(Package::NAME,None)].view.identity).unwrap().len(),2,"both role-specific reference handles remain");
            for name in [Package::NAME, Release::NAME] {
                let (name, prefix) = inputs.plain_binding(name).unwrap();
                let qualified = session.table_provider(ValidationInputs::table(name, prefix)).await.unwrap();
                let plain = session.table_provider(name).await.unwrap();
                assert!(Arc::ptr_eq(&plain, &qualified), "plain admission alias reuses its exact captured provider");
            }
            runtime
                .validate_references(&session, &[], &inputs, Some(&session))
                .await
                .unwrap();
            assert!(!runtime.memo.checked.lock().unwrap().0.is_empty());
            // Production compiler selection supplies external sessions of actual
            // native providers. Preserve their producing owner in a private snapshot.
            let producing = native
                .producing_scope(
                    ContentHash::of(b"empty-reference-producing"),
                    runtime.budget(),
                )
                .unwrap();
            let external_native = SessionContext::new();
            for (name, prefix) in inputs.relations.keys() {
                let provider = session
                    .table_provider(ValidationInputs::table(name, *prefix))
                    .await
                    .unwrap();
                let bound =
                    crate::compiler_provider::bind_producing(&provider, &producing).unwrap();
                external_native
                    .register_table(ValidationInputs::table(name, *prefix), bound)
                    .unwrap();
            }
            let snapshot = inputs
                .reference_session(&runtime, &external_native)
                .await
                .unwrap()
                .expect("actual native aliases earn the snapshot");
            runtime
                .validate_references(&snapshot, &[], &inputs, Some(&snapshot))
                .await
                .unwrap();
            // An external alias may select the same view through a genuinely
            // independent producing owner. It must not be collapsed by view ID.
            let alias_scope=native.producing_scope(ContentHash::of(b"independent-reference-role-owner"),runtime.budget()).unwrap();
            let alias_name=ValidationInputs::table(Package::NAME,Some(PublicationBoundary::Facts));
            let original_alias=external_native.deregister_table(alias_name.as_str()).unwrap().unwrap();
            external_native.register_table(alias_name.clone(),crate::compiler_provider::bind_producing(&base,&alias_scope).unwrap()).unwrap();
            let independent_snapshot=inputs.reference_session(&runtime,&external_native).await.unwrap().unwrap();
            let package_sources=&independent_snapshot.native_sources[&inputs.relations[&(Package::NAME,None)].view.identity];
            assert_eq!(package_sources.len(),2);
            assert!(!Arc::ptr_eq(&package_sources[0].provider(),&package_sources[1].provider()),"external equal-view providers retain distinct allocation/producer authority");
            alias_scope.close_and_wait().await.unwrap();
            assert!(independent_snapshot.current(&runtime,&independent_snapshot.context).await.is_err(),"the independent role's closed owner cannot hide behind a live equal-view provider");
            drop(independent_snapshot);
            external_native.deregister_table(alias_name.as_str()).unwrap().unwrap();
            external_native.register_table(alias_name,original_alias).unwrap();
            let package_source = inputs
                .relations
                .values()
                .find(|source| source.relation.name() == Package::NAME)
                .unwrap();
            let package_provider = external_native
                .table_provider(ValidationInputs::table(Package::NAME, None))
                .await
                .unwrap();
            assert!(
                crate::compiler_provider::reference_source(
                    &package_provider,
                    &native,
                    &package_source.view,
                    &package_source.relation,
                    runtime.budget()
                )
                .is_some()
            );
            let wrong_store = NativeCompilerStore::from_existing(
                Arc::new(lctx_native::surrealdb::Surreal::init()),
                lctx_model::domain::serving::Name::new("different_reference_owner").unwrap(),
                lctx_model::domain::serving::Name::new("different_reference_owner").unwrap(),
            );
            let wrong_store_eligible = crate::compiler_provider::reference_source(
                &package_provider,
                &wrong_store,
                &package_source.view,
                &package_source.relation,
                runtime.budget(),
            )
            .is_some();
            lctx_model::domain::completion::complete(Ok(()), wrong_store.drain_report().await)
                .unwrap();
            assert!(!wrong_store_eligible);
            let different_view = CompletedView::new(
                package_source.view.relation.clone(),
                package_source.view.contributions.clone(),
                1,
            )
            .unwrap();
            assert!(
                crate::compiler_provider::reference_source(
                    &package_provider,
                    &native,
                    &different_view,
                    &package_source.relation,
                    runtime.budget()
                )
                .is_none()
            );
            assert!(
                crate::compiler_provider::reference_source(
                    &package_provider,
                    &native,
                    &package_source.view,
                    &Relation::of::<Release>(),
                    runtime.budget()
                )
                .is_none()
            );
            assert!(
                crate::compiler_provider::reference_source(
                    &package_provider,
                    &native,
                    &package_source.view,
                    &Relation::of::<DifferentPackageType>(),
                    runtime.budget()
                )
                .is_none(),
                "Arrow shape cannot substitute for typed relation identity"
            );
            assert!(
                crate::compiler_provider::reference_source(
                    &package_provider,
                    &native,
                    &package_source.view,
                    &package_source.relation,
                    &ResourceBudget::fixed(runtime.budget().limit()).unwrap()
                )
                .is_none()
            );
            let filtered = crate::compiler_provider::select_table(
                &package_provider,
                Arc::new(vec![[1; 16]]),
                Arc::new(StateCharge::new(
                    runtime.budget(),
                    "reference filtered control",
                )),
            )
            .unwrap()
            .unwrap();
            assert!(
                crate::compiler_provider::reference_source(
                    &filtered,
                    &native,
                    &package_source.view,
                    &package_source.relation,
                    runtime.budget()
                )
                .is_none()
            );
            // The same exact declared views and memo must not vouch for arbitrary
            // external tables that contain a nonempty dangling source.
            let external = SessionContext::new();
            let absent = Package {
                name: "absent-from-owned-empty-view".into(),
            };
            let dangling = Release {
                package: absent.id(),
                version: "1.0".into(),
            };
            external
                .register_batch(
                    ValidationInputs::table(Package::NAME, None),
                    Package::encode(&[]).unwrap(),
                )
                .unwrap();
            external
                .register_batch(
                    ValidationInputs::table(Release::NAME, None),
                    Release::encode(&[dangling]).unwrap(),
                )
                .unwrap();
            let replacement = external
                .table_provider(ValidationInputs::table(Release::NAME, None))
                .await
                .unwrap();
            external_native
                .deregister_table(ValidationInputs::table(Release::NAME, None))
                .unwrap()
                .unwrap();
            external_native
                .register_table(ValidationInputs::table(Release::NAME, None), replacement)
                .unwrap();
            // The already captured original native Arcs remain the authority.
            runtime
                .validate_references(&snapshot, &[], &inputs, Some(&snapshot))
                .await
                .unwrap();
            assert!(
                inputs
                    .reference_session(&runtime, &external_native)
                    .await
                    .unwrap()
                    .is_none(),
                "a later capture sees the finite replacement"
            );
            let bindings = inputs
                .relations
                .values()
                .map(|source| SelectedBinding {
                    role: source.role,
                    requested: source.requested,
                    resolved: source.resolved,
                    binding: CompletedBinding {
                        source: source.source.snapshot(),
                        view: source.view.clone(),
                        boundary: None,
                        configuration: None,
                    },
                })
                .collect();
            let external_error = runtime
                .validate_compiler_selection(Profile::Catalog, false, bindings, &external, &inputs)
                .await
                .unwrap_err();
            assert!(
                external_error
                    .to_string()
                    .contains("missing or wrong-subtype nominal reference"),
                "owned empty memo must not bypass external actual rows: {external_error}"
            );
            // Pause after a new complete-empty missing-target premise has been
            // staged, then close the actual producing owner. Terminal refusal
            // must preserve all old memo entries and expose none from this call.
            let mut missing_target = inputs.clone();
            missing_target.relations.retain(|(name, _), _| *name != Package::NAME);
            let before_refusal = runtime.memo.checked.lock().unwrap().0.clone();
            let barrier = Arc::new(ReferencePhaseBarrier {
                reached: tokio::sync::Barrier::new(2),
                resume: tokio::sync::Barrier::new(2),
                pending: std::sync::atomic::AtomicUsize::new(0),
            });
            let validation = REFERENCE_PHASE_BARRIER.scope(barrier.clone(),
                runtime.validate_references(&snapshot, &[], &missing_target, Some(&snapshot)));
            tokio::pin!(validation);
            tokio::select! {
                result = &mut validation => panic!("reference phase did not reach terminal control: {result:?}"),
                _ = barrier.reached.wait() => {},
            }
            // No prefix is visible even before terminal validation fails.
            let unchanged = runtime.memo.checked.lock().unwrap().0 == before_refusal;
            let staged = barrier.pending.load(Ordering::Acquire);
            let closed = producing.close_and_wait().await;
            let (_, refused) = tokio::join!(barrier.resume.wait(), validation);
            assert!(staged > 0, "actual reference phase staged a new proof before the late close");
            assert!(unchanged, "staged reference proof remains private before terminal checks");
            closed.unwrap();
            assert!(refused.is_err(), "late producing close refuses the complete private phase");
            assert_eq!(runtime.memo.checked.lock().unwrap().0, before_refusal,
                "terminal refusal leaves the existing memo exactly unchanged");
            assert!(
                runtime
                    .validate_references(&snapshot, &[], &inputs, Some(&snapshot))
                    .await
                    .is_err(),
                "empty/memo proof cannot outlive its original producing scope"
            );
            // Restore actual native registrations so closed-owner failure cannot
            // be confused with the finite-provider fallback above.
            for ((name, prefix), source) in &inputs.relations {
                let provider = session
                    .table_provider(ValidationInputs::table(name, *prefix))
                    .await
                    .unwrap();
                external_native
                    .deregister_table(ValidationInputs::table(name, *prefix))
                    .unwrap()
                    .unwrap();
                external_native
                    .register_table(
                        ValidationInputs::table(name, *prefix),
                        crate::compiler_provider::bind_producing(&provider, &producing).unwrap(),
                    )
                    .unwrap();
                assert_eq!(source.view.rows, 0);
            }
            assert!(
                inputs
                    .reference_session(&runtime, &external_native)
                    .await
                    .is_err(),
                "closed actual owner must propagate instead of falling back"
            );
            // Nonempty reference SQL can earn memo reuse too; it still belongs
            // to the original producing owner rather than just logical identities.
            let pending = native
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: "nonempty-reference-control".into(),
                    profile: OutputProfile::Specific(Profile::Catalog),
                    model: model.digest(),
                    implementation: ContentHash::of(b"nonempty-reference-control/v1"),
                    configuration: None,
                    inputs: vec![],
                    outputs: relations
                        .iter()
                        .map(|relation| relation.name().into())
                        .collect(),
                })
                .await
                .unwrap();
            let package = Package {
                name: "actual-reference-target".into(),
            };
            let release = Release {
                package: package.id(),
                version: "1.0".into(),
            };
            native
                .write_batch(
                    &pending,
                    &relations[0],
                    &Package::encode(&[package]).unwrap(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            native
                .write_batch(
                    &pending,
                    &relations[1],
                    &Release::encode(&[release]).unwrap(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            let views = native
                .complete_contribution(
                    pending,
                    ProviderOutcome::Complete,
                    &relations,
                    &BTreeMap::new(),
                    runtime.budget(),
                )
                .await
                .unwrap();
            let nonempty = selected(&runtime, views.into_values().collect());
            let base = nonempty.session(&runtime).await.unwrap();
            let scope = native
                .producing_scope(
                    ContentHash::of(b"nonempty-reference-producing"),
                    runtime.budget(),
                )
                .unwrap();
            let external_nonempty = SessionContext::new();
            for (name, prefix) in nonempty.relations.keys() {
                let alias = ValidationInputs::table(name, *prefix);
                let provider = base.table_provider(alias.as_str()).await.unwrap();
                external_nonempty
                    .register_table(
                        alias,
                        crate::compiler_provider::bind_producing(&provider, &scope).unwrap(),
                    )
                    .unwrap();
            }
            let nonempty_snapshot = nonempty
                .reference_session(&runtime, &external_nonempty)
                .await
                .unwrap()
                .unwrap();
            let before = runtime.memo.checked.lock().unwrap().0.len();
            runtime
                .validate_references(&nonempty_snapshot, &[], &nonempty, Some(&nonempty_snapshot))
                .await
                .unwrap();
            let checked = runtime.memo.checked.lock().unwrap().0.len();
            assert!(
                checked > before,
                "actual nonempty reference SQL earns its own full premise"
            );
            runtime
                .validate_references(&nonempty_snapshot, &[], &nonempty, Some(&nonempty_snapshot))
                .await
                .unwrap();
            assert_eq!(runtime.memo.checked.lock().unwrap().0.len(), checked);
            scope.close_and_wait().await.unwrap();
            assert!(
                runtime
                    .validate_references(
                        &nonempty_snapshot,
                        &[],
                        &nonempty,
                        Some(&nonempty_snapshot)
                    )
                    .await
                    .is_err(),
                "nonempty memo cannot outlive producing authority"
            );
            native.fail();
            assert!(
                runtime
                    .validate_references(&session, &[], &inputs, Some(&session))
                    .await
                    .is_err(),
                "cached empty premise cannot outlive native source authority"
            );
        })
        .catch_unwind()
        .await;
        let mut terminal = runtime.drain_owned_report().await;
        terminal.merge(native.abandon_finality_report().await);
        let settled = runtime.complete_signal(&terminal).await;
        terminal.step("empty reference runtime signal finality", settled);
        runtime.append_operation_failures(&mut terminal);
        let cleanup = lctx_model::domain::completion::complete(Ok(()), terminal);
        if let Err(panic) = outcome {
            if let Err(error) = cleanup {
                eprintln!("empty reference cleanup also failed: {error}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
}
