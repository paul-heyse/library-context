//! Independent native artifact admission. Compiler providers and captured filesystem inputs remain above.
mod uniform;
pub use uniform::AdmittedUniformOwners;
pub(crate) mod candidate;
mod durable;
pub(crate) mod scoped;
mod normalization_declarations;
use crate::compiler::NativeCompilerStore;
use arrow_array::RecordBatch;
use datafusion::{
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
    Batch, ContentHash, ModelError, Record, Relation, ValidatedModel,
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
    scoped: Vec<Arc<scoped::ExecutedScopedCheck>>,
}
type SemanticEvidence = (
    Vec<lctx_model::domain::artifact_admission::ValidationExecution>,
    StateCharge,
    Vec<Arc<scoped::ExecutedScopedCheck>>,
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
    fn drain_owned_report(&self) -> BoxFuture<'_, lctx_model::domain::completion::Completion>;
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
    memo: Arc<AdmissionMemo>,
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
            memo,
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
            memo,
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
    pub fn install_completed(
        &self,
        profile: Profile,
        contributions: Vec<CompletedContribution>,
        bindings: Vec<CompletedBinding>,
    ) -> Result<(), ModelError> {
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
        self.install_completed(
            profile,
            self.native.contributions(&self.budget).await?,
            self.native.bindings().await?,
        )
    }
    pub(crate) async fn freeze_for_admission(&self) -> Result<(), ModelError> {
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
    /// Drain execution ownership without ending a surrounding publication's signal lifetime.
    pub async fn drain_owned_report(&self) -> lctx_model::domain::completion::Completion {
        if let Some(execution) = &self.borrowed_execution {
            return execution.drain_owned_report().await;
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("native calls drain", self.native_calls.drain().await);
        completion.step("private scoped check release", self.memo.close_scoped());
        completion.step(
            "native store drain",
            lctx_model::domain::completion::complete(Ok(()), self.native.drain_report().await),
        );
        completion
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
        completion
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
        let native = self.native.clone();
        let budget = self.budget.clone();
        let contributions = self
            .native_calls
            .call(async move { native.contributions(&budget).await })
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
                            reference_session.register_batch(
                                relation.name(),
                                RecordBatch::new_empty(self.model.relation(relation.name()).ok_or(ModelError::Schema(relation.name()))?.schema().clone()),
                            ).map_err(ModelError::codec)?;
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
            let checks = if admission {
                self.model.admission_candidates_for_scope(&names)?
            } else {
                self.model
                    .invariants_for_scope_with_premises(&names, &unrequested)?
            };
            let uniform = self.uniform_owners_async(profile).await?;
            let scoped_inventory = if admission && reference_owner.is_some() {
                Some(self.scoped_inventory()?)
            } else { None };
            for invariant in checks {
                tracing::info!(target:"lctx_phase", invariant=invariant.name, "semantic check selected");
                let premise = if inputs.companions.is_empty() {
                    self.validation_premise(&invariant, selected, &unrequested, profile)?
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
                if !is_scoped && !earn
                    && let Some(premise) = premise
                    && self.memo.checked.lock().map_err(|_| poisoned())?.0.contains(&premise)
                {
                    continue;
                }
                let scoped = if is_scoped && let Some(inventory) = &scoped_inventory {
                    self.scoped_premise(&invariant, &inputs, &unrequested, profile,
                        check.scoped_uniform_independent(), uniform.owners().is_empty(), inventory).await?
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
                            let executed = prepared.finish(Execution::EmptyDomain, &self.cancellation)?;
                            scoped_attempt.retain(executed.clone())?;
                            if earn { scoped_borrowers.push(executed); }
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
                            let executed = prepared.finish(Execution::EmptyDomain, &self.cancellation)?;
                            scoped_attempt.retain(executed.clone())?;
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
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
                            if earn { scoped_borrowers.push(executed); }
                        }
                        self.remember_premise(premise)?;
                    continue;
                }
                if external.is_none()
                    && self
                        .validate_durable_whole(
                            &invariant,
                            &inputs,
                            &unrequested,
                            profile,
                            uniform.owners(),
                            session,
                            planner,
                        )
                        .await?
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
    ) -> Result<Option<ContentHash>, ModelError> {
        use lctx_model::domain::Key;
        let Some(factory) = self.model.factory_contract(invariant.identity()) else {
            return Ok(None);
        };
        let mut sink = lctx_model::domain::KeySink::new("compiler-checked-premises/v2");
        self.model.digest().encode(&mut sink);
        invariant.digest().encode(&mut sink);
        factory.identity()?.encode(&mut sink);
        profile.name().to_string().encode(&mut sink);
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
            owned.require_owner(self, session)?;
        }
        let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
        let tables = inputs.reference_targets()?;
        // A union of reference targets cannot hide conflicting nominal payloads.
        // Captured companions carry independently protected canonical rows; each
        // selected copy of their IDs must preserve every typed field as well.
        for (position, (left_identity, (left_relation, left_table))) in tables.iter().enumerate() {
            let matching = tables
                .iter()
                .skip(position + 1)
                .any(|(_, (right, _))| right.name() == left_relation.name());
            let left_empty = if matching {
                match owned {
                    Some(owned) => owned.earned_empty(self, session, left_identity).await?,
                    None => false,
                }
            } else {
                false
            };
            for (right_identity, (right_relation, right_table)) in tables.iter().skip(position + 1)
            {
                self.cancellation.check()?;
                if left_relation.name() != right_relation.name() {
                    continue;
                }
                let differences = left_relation
                    .schema()
                    .fields()
                    .iter()
                    .filter(|field| field.name() != "id")
                    .map(|field| {
                        format!(
                            "l.{} IS DISTINCT FROM r.{}",
                            quote(field.name()),
                            quote(field.name())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" OR ");
                if differences.is_empty() {
                    continue;
                }
                if let Some(owned) = owned {
                    if left_empty {
                        owned.check_empty(self, session, left_identity)?;
                        continue;
                    }
                    if owned.earned_empty(self, session, right_identity).await? {
                        continue;
                    }
                }
                let sql = format!(
                    "SELECT l.id FROM {} l JOIN {} r ON l.id=r.id WHERE {differences} LIMIT 1",
                    quote(left_table),
                    quote(right_table)
                );
                let mut stream = crate::sql::query(session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    if batch.num_rows() > 0 {
                        return Err(ModelError::Conflict(
                            "selected nominal reference payloads conflict",
                        ));
                    }
                }
            }
        }
        for (identity, (relation, source_table)) in &tables {
            // One current-owner observation covers this synchronous empty-source
            // field block. Every field still checks cancellation and schema/premise
            // obligations; an empty block never suspends after earning this fact.
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
                if empty {
                    owned
                        .expect("earned empty private session")
                        .check_empty(self, session, identity)?;
                    self.remember_premise(Some(premise))?;
                    continue;
                }
                if let Some(owner) = owned
                    && self
                        .memo
                        .checked
                        .lock()
                        .map_err(|_| poisoned())?
                        .0
                        .contains(&premise)
                {
                    // A preceding nonempty field may have suspended for SQL. Do
                    // not let its later memo hit outlive any producing input owner.
                    owner.earned_empty(self, session, identity).await?;
                    for (target_identity, _) in &targets {
                        owner.earned_empty(self, session, target_identity).await?;
                    }
                    owner.check_empty(self, session, identity)?;
                    continue;
                }
                let source_name = quote(source_table);
                let field_name = quote(field.name());
                let sql = if !targets.is_empty() {
                    let target_tables = targets
                        .iter()
                        .map(|(_, (_, table))| format!("SELECT * FROM {}", quote(table)))
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
                            relation.name(),
                            field.name()
                        )));
                    }
                }
                if owned.is_some() {
                    self.remember_premise(Some(premise))?;
                }
            }
        }
        Ok(())
    }
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
    fn check_empty(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
        identity: &ContentHash,
    ) -> Result<(), ModelError> {
        self.require_owner(runtime, context)?;
        if let Some(sources) = self.native_sources.get(identity) {
            for source in sources {
                source.check()?;
            }
        }
        Ok(())
    }
    async fn earned_empty(
        &self,
        runtime: &AdmissionRuntime,
        context: &SessionContext,
        identity: &ContentHash,
    ) -> Result<bool, ModelError> {
        self.require_owner(runtime, context)?;
        if let Some(sources) = self.native_sources.get(identity) {
            // Nonempty providers also check their original producing and installed
            // owners before native reference memoization becomes eligible.
            for source in sources {
                source.current().await?;
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
        for sources in snapshot.native_sources.values() {
            for source in sources {
                source.current().await?;
            }
        }
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
        for ((name, prefix), source) in bindings {
            let provider = runtime
                .native
                .table_provider(
                    &source.view,
                    source.relation.clone(),
                    runtime.budget.clone(),
                    runtime.options.batch_rows,
                )
                .await?;
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
            let mut sources = self
                .relations
                .iter()
                .filter(|((n, _), _)| *n == name)
                .map(|(_, s)| s);
            let source = sources.next().expect("declared input");
            if sources.all(|other| {
                other.resolved == source.resolved && other.view.identity == source.view.identity
            }) {
                session
                    .register_table(
                        name,
                        runtime
                            .native
                            .table_provider(
                                &source.view,
                                source.relation.clone(),
                                runtime.budget.clone(),
                                runtime.options.batch_rows,
                            )
                            .await?,
                    )
                    .map_err(ModelError::codec)?;
            }
        }
        Ok(session)
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
        let native =
            NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts)
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
            let inputs = selected(&runtime, views.into_values().collect());
            let session = inputs.session(&runtime).await.unwrap();
            assert_eq!(session.native_sources.len(), 2);
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
            producing.close_and_wait().await.unwrap();
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
        terminal.step(
            "empty reference original native finality",
            native.abandon().await,
        );
        let settled = runtime.complete_signal(&terminal).await;
        terminal.step("empty reference runtime signal finality", settled);
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
