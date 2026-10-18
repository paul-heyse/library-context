//! Executed semantic checks over compiler-owned immutable providers. No native attempt,
//! installed view or artifact admission capability is created by this module.
use crate::admission::{AdmissionOptions, Cancellation};
use crate::consumed_rows::{ClosureTable, identifier};
use datafusion::{
    catalog::TableProvider,
    execution::runtime_env::RuntimeEnv,
    prelude::{SessionConfig, SessionContext},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    self as d, ContentHash, HeapSize, Key, KeySink, ModelError, Record, Relation, ValidatedModel,
    ValidationInput,
    analysis::{owner::AnalysisSelectedBinding, sources::SourceSnapshot},
    charged::StateCharge,
    checked_admission::{CheckedValidationPremise, FixedValidationDriver, UniformOwnerPremise},
    completed::CompletedContribution,
    resources::ResourceBudget,
    stages::{Profile, PublicationBoundary},
    uniform_outcome::{
        AnalysisUniformInputs, UniformKind, UniformNotRequested, ValidatedUniformOwner,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, Weak},
};

/// Actual provider ownership accompanies logical metadata. Neither field alone is a read grant.
#[derive(Clone)]
pub struct PrivateBinding {
    pub(crate) role: usize,
    pub(crate) requested: Option<PublicationBoundary>,
    pub(crate) resolved: Option<PublicationBoundary>,
    pub(crate) relation: Relation,
    pub(crate) source: SourceSnapshot,
    pub(crate) provider: Arc<dyn TableProvider>,
    pub(crate) contributions: BTreeSet<ContentHash>,
    immutable: Arc<crate::private_source::PrivateRelation>,
}
impl PrivateBinding {
    pub(crate) fn immutable_source(&self) -> &Arc<crate::private_source::PrivateRelation> {
        &self.immutable
    }
    /// Only successful checked ingress can produce the immutable source accepted here.
    /// A schema, digest, arbitrary provider or a deserialized descriptor cannot qualify it.
    pub fn completed(
        role: usize,
        requested: Option<PublicationBoundary>,
        resolved: Option<PublicationBoundary>,
        source: SourceSnapshot,
        contributions: BTreeSet<ContentHash>,
        immutable: Arc<crate::private_source::PrivateRelation>,
    ) -> Result<Self, ModelError> {
        let relation = immutable.relation().clone();
        let view = d::completed::CompletedView::new(
            relation.name().into(),
            contributions.clone(),
            immutable.rows(),
        )?;
        if source.model() != immutable.model()
            || source.relation() != relation.name()
            || source.rows() < 0
            || source.rows() as u64 != immutable.rows()
            || source.view() != view.identity
        {
            return Err(ModelError::Conflict(
                "private completed source identity/membership",
            ));
        }
        let provider = immutable.provider();
        Ok(Self {
            role,
            requested,
            resolved,
            relation,
            source,
            provider,
            contributions,
            immutable,
        })
    }
}
/// Ordered semantic roles, independent of incidental inventory positions. These metadata
/// are lookup context only; successful execution remains an opaque operation-owned value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivateCheckInput {
    pub(crate) name: &'static str,
    pub(crate) order: Vec<&'static str>,
    pub(crate) requested: Option<PublicationBoundary>,
    pub(crate) selected: Option<(Option<PublicationBoundary>, SourceSnapshot)>,
    pub(crate) not_requested: Option<ContentHash>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivateCheckPremise {
    pub(crate) model: ContentHash,
    pub(crate) profile: Profile,
    pub(crate) name: &'static str,
    pub(crate) revision: u32,
    pub(crate) definition: ContentHash,
    pub(crate) factory: ContentHash,
    pub(crate) inputs: Vec<PrivateCheckInput>,
    pub(crate) uniform_owners: Vec<UniformOwnerPremise>,
}
/// Constructed only by the terminal production branch. Scoped/empty results never acquire
/// a whole-input proof. Actual immutable sources and uniform owners survive materialization.
pub(crate) struct PrivateExecutedCheck {
    premise: PrivateCheckPremise,
    execution: d::artifact_admission::Execution,
    bindings: Vec<PrivateBinding>,
    _uniform: Vec<ValidatedUniformOwner>,
    whole: Option<CheckedValidationPremise>,
    cancellation: Cancellation,
    _charge: StateCharge,
}
impl PrivateExecutedCheck {
    pub(crate) fn premise(&self) -> &PrivateCheckPremise {
        &self.premise
    }
    pub(crate) fn execution(&self) -> d::artifact_admission::Execution {
        self.execution
    }
    pub(crate) fn bindings(&self) -> &[PrivateBinding] {
        &self.bindings
    }
    pub(crate) fn whole(&self) -> Option<&CheckedValidationPremise> {
        self.whole.as_ref()
    }
    pub(crate) fn check_current(
        &self,
        model: ContentHash,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
    ) -> Result<(), ModelError> {
        self.cancellation.check()?;
        cancellation.check()?;
        if self.premise.model != model
            || !self.cancellation.shares_owner(cancellation)
            || !self
                ._charge
                .budget()
                .is_some_and(|own| own.shares_pool(budget))
        {
            return Err(ModelError::Conflict(
                "private executed check operation owner",
            ));
        }
        Ok(())
    }
    fn same_actual(&self, other: &Self) -> bool {
        self.premise == other.premise
            && self.execution == other.execution
            && self.bindings.len() == other.bindings.len()
            && self
                .bindings
                .iter()
                .zip(&other.bindings)
                .all(|(left, right)| actual_binding(left) == actual_binding(right))
    }
}
struct PreparedPrivateCheck {
    premise: PrivateCheckPremise,
    bindings: Vec<PrivateBinding>,
    uniform: Vec<ValidatedUniformOwner>,
    cancellation: Cancellation,
    charge: StateCharge,
}
impl PreparedPrivateCheck {
    fn finish(
        self,
        execution: d::artifact_admission::Execution,
        whole: Option<CheckedValidationPremise>,
    ) -> Result<Arc<PrivateExecutedCheck>, ModelError> {
        use d::artifact_admission::Execution;
        self.cancellation.check()?;
        if execution != Execution::EmptyDomain
            && self
                .premise
                .inputs
                .iter()
                .any(|role| role.selected.is_none() && role.not_requested.is_none())
        {
            return Err(ModelError::Conflict(
                "nonempty private execution lacks an exact input role",
            ));
        }
        match (&whole, execution) {
            (Some(whole), Execution::WholeInput) => {
                let data = whole.data();
                if data.model != self.premise.model
                    || data.name != self.premise.name
                    || data.revision != self.premise.revision
                    || data.definition != self.premise.definition
                    || data.factory != self.premise.factory
                    || data.inputs.len() != self.premise.inputs.len()
                    || data.uniform_owners != self.premise.uniform_owners
                    || data
                        .inputs
                        .iter()
                        .zip(&self.premise.inputs)
                        .any(|(actual, role)| {
                            role.selected.as_ref().is_none_or(|(resolved, source)| {
                                actual.source != *source
                                    || actual.requested != role.requested
                                    || actual.resolved != *resolved
                            })
                        })
                {
                    return Err(ModelError::Conflict(
                        "private whole executed ledger differs from exact context",
                    ));
                }
            }
            (None, Execution::Scoped | Execution::EmptyDomain) => {}
            _ => {
                return Err(ModelError::Conflict(
                    "private execution kind differs from earned whole proof",
                ));
            }
        }
        Ok(Arc::new(PrivateExecutedCheck {
            premise: self.premise,
            execution,
            bindings: self.bindings,
            _uniform: self.uniform,
            whole,
            cancellation: self.cancellation,
            _charge: self.charge,
        }))
    }
}
fn actual_binding(
    binding: &PrivateBinding,
) -> (
    ContentHash,
    Option<PublicationBoundary>,
    Option<PublicationBoundary>,
    ContentHash,
    u64,
) {
    (
        binding.source.identity(),
        binding.requested,
        binding.resolved,
        binding.immutable.content(),
        binding.immutable.rows(),
    )
}

/// Only complete reference checking plus all selected semantic kernels creates this owner.
/// Retained providers keep every earned premise's immutable backing alive.
pub struct CheckedPrivateScope {
    identity: ContentHash,
    uniform: Vec<ValidatedUniformOwner>,
    _executed: Vec<Arc<PrivateExecutedCheck>>,
    _bindings: Vec<PrivateBinding>,
    _typed: Vec<Arc<CheckedPrivateSource>>,
    _charge: StateCharge,
}
impl CheckedPrivateScope {
    pub fn identity(&self) -> ContentHash {
        self.identity
    }
    pub fn uniform_owners(&self) -> &[ValidatedUniformOwner] {
        &self.uniform
    }
    pub(crate) fn shares_budget(&self, budget: &ResourceBudget) -> bool {
        self._charge
            .budget()
            .is_some_and(|own| own.shares_pool(budget))
    }
    pub fn admitted_uniform_owners(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<Arc<crate::admission::AdmittedUniformOwners>, ModelError> {
        crate::admission::AdmittedUniformOwners::from_private(self.clone(), budget)
    }
}
struct CheckedPrivateSource {
    _binding: PrivateBinding,
    _charge: StateCharge,
}
pub struct PrivateValidation {
    model: Arc<ValidatedModel>,
    options: AdmissionOptions,
    budget: ResourceBudget,
    cancellation: Cancellation,
    runtime: Arc<RuntimeEnv>,
    completed: Mutex<BTreeMap<ContentHash, Weak<CheckedPrivateScope>>>,
    typed: Mutex<BTreeMap<ContentHash, Weak<CheckedPrivateSource>>>,
    registry_charge: Mutex<StateCharge>,
    // Strong operation-local retention has one concrete future consumer: materialized
    // admission. Sources contain no Workspace/PrivateValidation links, so there is no cycle.
    executed: Mutex<(Vec<Arc<PrivateExecutedCheck>>, StateCharge)>,
}
struct Registered<'a> {
    binding: &'a PrivateBinding,
    table: String,
}
impl PrivateValidation {
    pub fn new(
        model: Arc<ValidatedModel>,
        options: AdmissionOptions,
        budget: ResourceBudget,
        cancellation: Cancellation,
        runtime: Arc<RuntimeEnv>,
    ) -> Result<Self, ModelError> {
        if options.memory_bytes != budget.limit()
            || options.batch_rows == 0
            || options.partitions == 0
        {
            return Err(ModelError::Conflict("private validation resource settings"));
        }
        let mut registry_charge = StateCharge::new(&budget, "private semantic lookup registry");
        registry_charge.grow(size_of::<Self>())?;
        let executed_charge = StateCharge::new(&budget, "retained private executed check slots");
        Ok(Self {
            model,
            options,
            budget,
            cancellation,
            runtime,
            completed: Mutex::new(BTreeMap::new()),
            typed: Mutex::new(BTreeMap::new()),
            registry_charge: Mutex::new(registry_charge),
            executed: Mutex::new((Vec::new(), executed_charge)),
        })
    }
    pub(crate) fn check_operation(
        &self,
        model: ContentHash,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
    ) -> Result<(), ModelError> {
        self.cancellation.check()?;
        cancellation.check()?;
        if self.model.digest() != model
            || !self.budget.shares_pool(budget)
            || !self.cancellation.shares_owner(cancellation)
        {
            return Err(ModelError::Conflict(
                "private semantic transfer operation owner",
            ));
        }
        Ok(())
    }
    /// Exact metadata nominates a property; native consumers must still establish full
    /// typed-row correspondence under their own actual held installed capabilities.
    pub(crate) fn find_executed(
        &self,
        premise: &PrivateCheckPremise,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
    ) -> Result<Option<Arc<PrivateExecutedCheck>>, ModelError> {
        self.check_operation(premise.model, budget, cancellation)?;
        let entries = self.executed.lock().map_err(|_| poisoned())?;
        let mut matches = entries.0.iter().filter(|check| check.premise == *premise);
        let Some(first) = matches.next() else {
            return Ok(None);
        };
        // Logical SourceSnapshot headers omit payload. Never arbitrarily choose one
        // successful property if equal metadata describes different actual private content.
        if matches.any(|other| !first.same_actual(other)) {
            return Ok(None);
        }
        first.check_current(premise.model, budget, cancellation)?;
        Ok(Some(first.clone()))
    }
    fn prepare_executed(
        &self,
        invariant: &d::Invariant,
        profile: Profile,
        selected: &[Registered<'_>],
        universe: &[Registered<'_>],
        unrequested: &BTreeSet<&str>,
        uniform: &[ValidatedUniformOwner],
    ) -> Result<Option<PreparedPrivateCheck>, ModelError> {
        let Some(factory) = self.model.factory_contract(invariant.identity()) else {
            return Ok(None);
        };
        let mut charge = StateCharge::new(&self.budget, "private executed invariant property");
        charge.grow(
            size_of::<PrivateExecutedCheck>()
                + 1024
                + invariant.inputs.len().saturating_mul(512)
                + uniform.len().saturating_mul(
                    size_of::<UniformOwnerPremise>() + size_of::<ValidatedUniformOwner>(),
                ),
        )?;
        let mut inputs = Vec::with_capacity(invariant.inputs.len());
        let mut actual = BTreeMap::new();
        for input in &invariant.inputs {
            let (selected, not_requested) = match resolve(input, selected) {
                Ok(source) => {
                    charge.grow(
                        source.binding.source.heap_bytes().saturating_mul(2)
                            + input.order().len().saturating_mul(32),
                    )?;
                    let key = actual_binding(source.binding);
                    if !actual.contains_key(&key) {
                        charge.grow(128)?;
                    }
                    actual.insert(key, source.binding);
                    (
                        Some((source.binding.resolved, source.binding.source.clone())),
                        None,
                    )
                }
                Err(_) if unrequested.contains(input.name()) => (
                    None,
                    Some(d::producer_contract::facts::ty_flow().identity()),
                ),
                Err(_) => (None, None),
            };
            inputs.push(PrivateCheckInput {
                name: input.name(),
                order: input.order().to_vec(),
                requested: input.prefix(),
                selected,
                not_requested,
            });
        }
        // Every check was supplied these actual validated uniform owners. Retain their
        // compact sources and qualification inputs as well as the declared semantic roles.
        if !uniform.is_empty() {
            for source in selected
                .iter()
                .filter(|source| source.binding.relation.name() == UniformNotRequested::NAME)
                .chain(universe)
            {
                let key = actual_binding(source.binding);
                if !actual.contains_key(&key) {
                    charge.grow(128)?;
                }
                actual.insert(key, source.binding);
            }
        }
        let binding_bytes = actual
            .values()
            .map(|binding| {
                size_of::<PrivateBinding>()
                    + binding.source.heap_bytes()
                    + binding.contributions.len().saturating_mul(64)
                    + 256
            })
            .sum::<usize>();
        charge.grow(binding_bytes + actual.len().saturating_mul(128))?;
        let bindings = actual.into_values().cloned().collect();
        let mut uniform_owners = uniform
            .iter()
            .map(|owner| UniformOwnerPremise {
                contribution: owner.contribution(),
                descriptor: *owner.descriptor().id().bytes(),
                content: owner.descriptor().content_digest(),
                profile: owner.profile(),
            })
            .collect::<Vec<_>>();
        uniform_owners.sort_by_key(|owner| (owner.contribution, owner.descriptor));
        if uniform_owners.windows(2).any(|pair| {
            (pair[0].contribution, pair[0].descriptor) == (pair[1].contribution, pair[1].descriptor)
        }) {
            return Err(ModelError::Conflict(
                "private executed duplicate uniform context",
            ));
        }
        Ok(Some(PreparedPrivateCheck {
            premise: PrivateCheckPremise {
                model: self.model.digest(),
                profile,
                name: invariant.name,
                revision: invariant.revision,
                definition: invariant.digest(),
                factory: factory.identity()?,
                inputs,
                uniform_owners,
            },
            bindings,
            uniform: uniform.to_vec(),
            cancellation: self.cancellation.clone(),
            charge,
        }))
    }
    fn commit_executed(&self, pending: &[Arc<PrivateExecutedCheck>]) -> Result<(), ModelError> {
        self.cancellation.check()?;
        let mut entries = self.executed.lock().map_err(|_| poisoned())?;
        let additional = pending
            .iter()
            .filter(|check| !entries.0.iter().any(|known| known.same_actual(check)))
            .count();
        let bytes = additional.saturating_mul(size_of::<Arc<PrivateExecutedCheck>>());
        entries.1.grow(bytes)?;
        if let Err(error) = entries.0.try_reserve_exact(additional) {
            entries.1.release(bytes);
            return Err(ModelError::codec(error));
        }
        // All fallible reservations precede publication. A failed validation prefix cannot
        // expose its newly completed checks, even when earlier individual kernels passed.
        for check in pending {
            if !entries.0.iter().any(|known| known.same_actual(check)) {
                entries.0.push(check.clone());
            }
        }
        Ok(())
    }
    pub async fn validate(
        &self,
        profile: Profile,
        selected: Vec<PrivateBinding>,
        universe: Vec<PrivateBinding>,
        owners: Vec<CompletedContribution>,
    ) -> Result<Arc<CheckedPrivateScope>, ModelError> {
        self.cancellation.check()?;
        // Only genuine compact contributors require the wider qualification universe.
        // Unrelated completed results must neither inhibit reuse nor retain their payloads.
        let compact = selected
            .iter()
            .filter(|binding| binding.relation.name() == UniformNotRequested::NAME)
            .flat_map(|binding| binding.contributions.iter().copied())
            .collect::<BTreeSet<_>>();
        let owners = owners
            .into_iter()
            .filter(|owner| {
                owner.outputs.contains_key(UniformNotRequested::NAME)
                    && owner
                        .identity()
                        .is_ok_and(|identity| compact.contains(&identity))
            })
            .collect::<Vec<_>>();
        let required = owners
            .iter()
            .filter(|owner| {
                owner.spec.producer != d::producer_contract::facts::ASSEMBLE
                    && owner.outputs[UniformNotRequested::NAME].rows != 0
            })
            .flat_map(|owner| owner.spec.inputs.iter().map(SourceSnapshot::identity))
            .collect::<BTreeSet<_>>();
        let universe = universe
            .into_iter()
            .filter(|binding| {
                required.contains(&binding.source.identity())
                    || (binding.relation.name() == UniformNotRequested::NAME
                        && binding.contributions.len() == 1
                        && binding
                            .contributions
                            .iter()
                            .all(|identity| compact.contains(identity)))
            })
            .collect::<Vec<_>>();
        let mut charge = StateCharge::new(&self.budget, "private semantic scope ownership");
        charge.grow(
            selected
                .iter()
                .chain(&universe)
                .map(|b| {
                    size_of::<PrivateBinding>()
                        + b.source.heap_bytes()
                        + b.contributions.len() * 64
                        + 256
                })
                .sum::<usize>()
                + owners
                    .iter()
                    .map(|o| serde_json::to_vec(o).map(|v| v.len() * 4 + 512))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ModelError::codec)?
                    .into_iter()
                    .sum::<usize>()
                + 1024,
        )?;
        let key = self.identity(profile, &selected, &universe, &owners, true)?;
        let identity = self.identity(profile, &selected, &universe, &owners, false)?;
        if let Some(checked) = self
            .completed
            .lock()
            .map_err(|_| poisoned())?
            .get(&key)
            .and_then(Weak::upgrade)
        {
            return Ok(checked);
        }
        let session = SessionContext::new_with_config_rt(
            SessionConfig::new()
                .with_target_partitions(self.options.partitions)
                .with_batch_size(self.options.batch_rows),
            self.runtime.clone(),
        );
        let selected = self.register(&session, &selected, "selected")?;
        let universe = self.register(&session, &universe, "universe")?;
        // Provider descriptors are not evidence of actual typed content or EOF.
        // One checked pass per exact provider/source within this acquisition establishes that ledger.
        let mut examined = BTreeSet::new();
        let mut prepared_typed = Vec::new();
        for registered in &selected {
            let key = binding_key(registered.binding);
            if !examined.insert(key) {
                continue;
            }
            if let Some(property) = self
                .typed
                .lock()
                .map_err(|_| poisoned())?
                .get(&key)
                .and_then(Weak::upgrade)
            {
                prepared_typed.push((key, property));
                continue;
            }
            self.check_rows(&session, registered).await?;
            let mut retained = StateCharge::new(&self.budget, "private executed typed property");
            retained.grow(
                size_of::<CheckedPrivateSource>()
                    + registered.binding.source.heap_bytes()
                    + registered.binding.contributions.len() * 64
                    + 128,
            )?;
            prepared_typed.push((
                key,
                Arc::new(CheckedPrivateSource {
                    _binding: registered.binding.clone(),
                    _charge: retained,
                }),
            ));
        }
        self.references(&session, &selected).await?;
        let uniform = self
            .uniform(&session, profile, &selected, &universe, &owners)
            .await?;
        let unrequested = self.unrequested(profile);
        let referrers = selected
            .iter()
            .map(|r| {
                let input = ValidationInput::of_relation(&r.binding.relation, &["id"]);
                r.binding
                    .requested
                    .map_or(input.clone(), |epoch| input.at_epoch(epoch))
            })
            .collect::<Vec<_>>();
        let checks = self.model.admission_candidates_for_views(&referrers)?;
        let mut executed = Vec::new();
        for invariant in checks {
            self.cancellation.check()?;
            let mut check = (invariant.create)(&self.budget);
            for owner in &uniform {
                check.admit_uniform_owner(owner)?;
            }
            let execution = check.execution_scope();
            let normalization = check.normalization_scope();
            let projection = check.projection_scope();
            let retrieval = check.retrieval_scope();
            let aspects = check.aspect_scope();
            let inventory = check.inventory_scope();
            let support = check.support_scope();
            let roots = execution
                .as_ref()
                .map(|s| vec![s.root.clone()])
                .or_else(|| normalization.map(|s| s.roots()))
                .or_else(|| projection.map(|s| s.roots()));
            let scoped = execution.is_some()
                || normalization.is_some()
                || projection.is_some()
                || retrieval.is_some()
                || aspects.is_some()
                || inventory.is_some()
                || support.is_some();
            // A model-declared no-op compact contract removes those dependencies only
            // for actual scoped kernels. Whole fixed-driver inputs remain unchanged.
            let effective_uniform = if scoped && check.scoped_uniform_independent() {
                &[][..]
            } else {
                uniform.as_slice()
            };
            let prepared = self.prepare_executed(
                &invariant,
                profile,
                &selected,
                &universe,
                &unrequested,
                effective_uniform,
            )?;
            if let Some(roots) = roots
                && roots
                    .iter()
                    .all(|root| self.empty_root(root, &selected, &unrequested))
            {
                if let Some(prepared) = prepared {
                    charge.grow(size_of::<Arc<PrivateExecutedCheck>>())?;
                    executed.push(
                        prepared.finish(d::artifact_admission::Execution::EmptyDomain, None)?,
                    );
                }
                continue;
            }
            let mut tables = Vec::new();
            for input in &invariant.inputs {
                let relation = self
                    .model
                    .relation(input.name())
                    .ok_or(ModelError::Schema(input.name()))?
                    .clone();
                let alias = match resolve(input, &selected) {
                    Ok(r) => r.table.clone(),
                    Err(_) if unrequested.contains(input.name()) => {
                        let alias = format!("private_unrequested_{}", input.name());
                        if !session
                            .table_exist(alias.as_str())
                            .map_err(crate::sql::model_error)?
                        {
                            session
                                .register_batch(
                                    alias.as_str(),
                                    arrow_array::RecordBatch::new_empty(relation.schema().clone()),
                                )
                                .map_err(crate::sql::model_error)?;
                        }
                        alias
                    }
                    Err(error) => return Err(error),
                };
                tables.push(ClosureTable { relation, alias });
            }
            if scoped {
                drop(check);
                if let Some(scope) = retrieval {
                    crate::scoped_retrieval::validate_retrieval(
                        &invariant,
                        scope,
                        tables,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                } else if let Some(scope) = projection {
                    crate::normalize::validate_projections(
                        scope,
                        &invariant,
                        tables,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                } else if let Some(scope) = normalization {
                    use d::normalized::admission::Scope;
                    match scope {
                        Scope::Callables => {
                            crate::normalize::validate_callables(
                                &invariant,
                                tables,
                                &session,
                                &self.budget,
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
                                &self.budget,
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
                                &self.budget,
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
                                &self.budget,
                                &self.cancellation,
                                &self.model,
                            )
                            .await?
                        }
                    }
                } else if let Some(scope) = execution {
                    crate::scoped_execution::validate_execution(
                        &invariant,
                        &scope,
                        tables,
                        &self.model,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                } else if let Some(scope) = aspects {
                    crate::scoped_aspects::validate_aspects(
                        &invariant,
                        &scope,
                        tables,
                        &self.model,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                } else if let Some(scope) = inventory {
                    crate::scoped_inventory::validate_inventory(
                        &invariant,
                        &scope,
                        tables,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                } else if let Some(scope) = support {
                    crate::scoped_admission::validate_support(
                        &invariant,
                        &scope,
                        tables,
                        &self.model,
                        &session,
                        &self.budget,
                        &self.cancellation,
                    )
                    .await?;
                }
                if let Some(prepared) = prepared {
                    charge.grow(size_of::<Arc<PrivateExecutedCheck>>())?;
                    executed.push(prepared.finish(d::artifact_admission::Execution::Scoped, None)?);
                }
                continue;
            }
            let declared = self
                .model
                .invariants()
                .iter()
                .find(|declared| declared.name == invariant.name);
            let exact_factory = self.model.factory_contract(invariant.identity()).is_some()
                && declared.is_some_and(|declared| {
                    declared
                        .inputs
                        .iter()
                        .map(|i| (i.name(), i.prefix(), i.order()))
                        .eq(invariant
                            .inputs
                            .iter()
                            .map(|i| (i.name(), i.prefix(), i.order())))
                })
                && invariant
                    .inputs
                    .iter()
                    .all(|i| !unrequested.contains(i.name()) || resolve(i, &selected).is_ok());
            let mut fixed = if exact_factory {
                Some(FixedValidationDriver::invariant(
                    &self.model,
                    invariant.identity(),
                    invariant
                        .inputs
                        .iter()
                        .map(|input| {
                            let r = resolve(input, &selected)?;
                            Ok((r.binding.source.clone(), input.prefix(), r.binding.resolved))
                        })
                        .collect::<Result<Vec<_>, ModelError>>()?,
                    &uniform,
                    &self.budget,
                )?)
            } else {
                None
            };
            for (index, (input, table)) in invariant.inputs.iter().zip(&tables).enumerate() {
                let order = input
                    .order()
                    .iter()
                    .map(|n| identifier(n))
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT * FROM {}{}",
                    identifier(&table.alias),
                    if order.is_empty() {
                        String::new()
                    } else {
                        format!(" ORDER BY {order}")
                    }
                );
                let mut rows = crate::sql::query(&session, &sql)
                    .await
                    .map_err(crate::sql::model_error)?
                    .execute_stream()
                    .await
                    .map_err(crate::sql::model_error)?;
                let mut count = 0u64;
                while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
                    self.cancellation.check()?;
                    count = count
                        .checked_add(batch.num_rows() as u64)
                        .ok_or(ModelError::Conflict("private validator row overflow"))?;
                    if let Some(driver) = &mut fixed {
                        driver.visit(index, &batch)?;
                    } else {
                        check.visit_input(input, &batch)?;
                    }
                }
                let expected = resolve(input, &selected)
                    .map(|r| r.binding.source.rows() as u64)
                    .unwrap_or(0);
                if count != expected {
                    return Err(ModelError::Conflict("private validator exact input EOF"));
                }
                if let Some(driver) = &mut fixed {
                    driver.end_input(index)?;
                }
            }
            if let Some(driver) = fixed {
                let whole = driver.finish()?;
                let prepared = prepared.ok_or(ModelError::Conflict(
                    "fixed private execution lacks its registered context",
                ))?;
                charge.grow(size_of::<Arc<PrivateExecutedCheck>>())?;
                executed.push(
                    prepared.finish(d::artifact_admission::Execution::WholeInput, Some(whole))?,
                );
            } else {
                // A successful rewritten callback is not a fixed whole-input factory proof.
                check.finish()?;
            }
        }
        self.cancellation.check()?;
        let bindings = selected
            .into_iter()
            .chain(universe)
            .map(|r| r.binding.clone())
            .collect();
        let scope = Arc::new(CheckedPrivateScope {
            identity,
            uniform,
            _executed: executed,
            _bindings: bindings,
            _typed: prepared_typed
                .iter()
                .map(|(_, property)| property.clone())
                .collect(),
            _charge: charge,
        });
        let mut completed = self.completed.lock().map_err(|_| poisoned())?;
        if let Some(existing) = completed.get(&key).and_then(Weak::upgrade) {
            return Ok(existing);
        }
        let mut typed = self.typed.lock().map_err(|_| poisoned())?;
        let new_entries = usize::from(!completed.contains_key(&key))
            + prepared_typed
                .iter()
                .filter(|(key, _)| !typed.contains_key(key))
                .count();
        self.registry_charge
            .lock()
            .map_err(|_| poisoned())?
            .grow(new_entries.saturating_mul(128))?;
        self.commit_executed(&scope._executed)?;
        for (key, property) in prepared_typed {
            typed.insert(key, Arc::downgrade(&property));
        }
        completed.insert(key, Arc::downgrade(&scope));
        Ok(scope)
    }
    /// Facts availability is reconstructed from actual completed provider records and exact
    /// captured contributor declarations, independently of optional analysis demand.
    pub async fn facts_availability(
        &self,
        profile: Profile,
        bindings: Vec<PrivateBinding>,
        exact_sources: Vec<PrivateBinding>,
        owners: Vec<CompletedContribution>,
    ) -> Result<Arc<d::admission::ScopedAvailability>, ModelError> {
        use d::{
            attribution::{AnalysisContext, Provider, ProviderCoverage, ProviderRun, RunFamily},
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
        };
        self.cancellation.check()?;
        let compact = bindings
            .iter()
            .filter(|binding| binding.relation.name() == UniformNotRequested::NAME)
            .flat_map(|binding| binding.contributions.iter().copied())
            .collect::<BTreeSet<_>>();
        let exact_sources = exact_sources
            .into_iter()
            .filter(|binding| {
                binding.relation.name() == UniformNotRequested::NAME
                    && binding.contributions.len() == 1
                    && binding
                        .contributions
                        .iter()
                        .all(|identity| compact.contains(identity))
            })
            .collect::<Vec<_>>();
        let mut transfer = StateCharge::new(&self.budget, "private facts descriptor transfer");
        transfer.grow(
            bindings
                .iter()
                .chain(&exact_sources)
                .map(|b| {
                    size_of::<PrivateBinding>()
                        + b.source.heap_bytes()
                        + b.contributions.len() * 64
                        + 256
                })
                .sum::<usize>()
                + owners
                    .iter()
                    .map(|o| serde_json::to_vec(o).map(|v| v.len() * 4 + 512))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ModelError::codec)?
                    .into_iter()
                    .sum::<usize>(),
        )?;
        let session = SessionContext::new_with_config_rt(
            SessionConfig::new()
                .with_target_partitions(self.options.partitions)
                .with_batch_size(self.options.batch_rows),
            self.runtime.clone(),
        );
        let registered = self.register(&session, &bindings, "facts")?;
        let exact = self.register(&session, &exact_sources, "facts_exact_compact")?;
        self.check_uniform_sources(&registered, &exact, &owners)?;
        let (inputs, _inputs) = self.rows::<InputRevision>(&session, &registered).await?;
        let (artifacts, _artifacts) = self.rows::<SourceArtifact>(&session, &registered).await?;
        let (uses, _uses) = self.rows::<ArtifactUse>(&session, &registered).await?;
        let (scopes, _scopes) = self.rows::<CoverageScope>(&session, &registered).await?;
        let (observed, _observed) = self.rows::<ProviderCoverage>(&session, &registered).await?;
        let (recorded, _recorded) = self.rows::<Provider>(&session, &registered).await?;
        let (runs, _runs) = self.rows::<ProviderRun>(&session, &registered).await?;
        let (families, _families) = self.rows::<RunFamily>(&session, &registered).await?;
        let (contexts, _contexts) = self.rows::<AnalysisContext>(&session, &registered).await?;
        let (compact, _compact) = self
            .rows::<UniformNotRequested>(&session, &registered)
            .await?;
        let names = [
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
        ];
        let mut bound_sources = BTreeMap::new();
        let mut authority = BTreeSet::new();
        for name in names {
            let relation = self.model.relation(name).ok_or(ModelError::Schema(name))?;
            let selected = resolve(
                &ValidationInput::of_relation(relation, &["id"]),
                &registered,
            )?;
            bound_sources.insert(name.to_owned(), selected.binding.source.clone());
            authority.extend(selected.binding.contributions.iter().copied());
        }
        let compact_authority = &resolve(
            &ValidationInput::of::<UniformNotRequested>(&["id"]),
            &registered,
        )?
        .binding
        .contributions;
        let recorded = crate::facts::recorded_coverage_with_uniform(
            &self.model,
            profile,
            &crate::facts::RecordedFacts {
                inputs: &inputs,
                artifacts: &artifacts,
                uses: &uses,
                observed: &observed,
                recorded: &recorded,
                runs: &runs,
                families: &families,
                contexts: &contexts,
                contributions: &owners,
                authority: &authority,
                bound_sources: &bound_sources,
            },
            &compact,
            compact_authority,
            &self.budget,
        )?;
        let uniform = recorded
            .uniform
            .iter()
            .map(|row| (&row.descriptor, &row.domain, &row.owner))
            .collect::<Vec<_>>();
        let scopes = scopes.into_iter().map(|row| (row.id(), row)).collect();
        let availability = d::admission::ScopedAvailability::from_completed_with_uniform(
            profile,
            &recorded.expected,
            &observed,
            &scopes,
            &uniform,
            &self.budget,
        )?;
        self.cancellation.check()?;
        Ok(Arc::new(availability))
    }
    async fn rows<R: Record>(
        &self,
        session: &SessionContext,
        registered: &[Registered<'_>],
    ) -> Result<(Vec<R>, StateCharge), ModelError> {
        let source = resolve(&ValidationInput::of::<R>(&["id"]), registered)?;
        let sql = format!("SELECT * FROM {} ORDER BY id", identifier(&source.table));
        let mut stream = crate::sql::query(session, &sql)
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        let mut rows = Vec::new();
        let mut charge = StateCharge::new(&self.budget, "private selected facts metadata");
        let mut content = source.binding.relation.content();
        while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
            self.cancellation.check()?;
            let _decode = self.budget.reserve(
                "private facts typed decode",
                d::decode_allowance::<R>(&batch)?,
            )?;
            let batch = source.binding.relation.ordered_canonical(&batch)?;
            source.binding.relation.hash_rows(&batch, &mut content)?;
            for row in R::decode(&batch)? {
                charge.grow(size_of::<R>() + row.heap_bytes())?;
                if rows.len() == rows.capacity() {
                    let additional = rows.capacity().max(4);
                    charge.grow(additional * size_of::<R>())?;
                    rows.reserve_exact(additional);
                }
                rows.push(row);
            }
        }
        let (actual_rows, actual_content) = content.finish();
        if actual_rows != source.binding.source.rows() as u64
            || actual_rows != source.binding.immutable.rows()
            || actual_content != source.binding.immutable.content()
        {
            return Err(ModelError::Conflict(
                "private facts actual metadata EOF/content",
            ));
        }
        Ok((rows, charge))
    }
    fn identity(
        &self,
        profile: Profile,
        selected: &[PrivateBinding],
        universe: &[PrivateBinding],
        owners: &[CompletedContribution],
        include_provider: bool,
    ) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("private-executed-semantic-scope/v2");
        self.model.digest().encode(&mut sink);
        profile.name().to_string().encode(&mut sink);
        for (tag, bindings) in [
            (b"selected".as_slice(), selected),
            (b"universe".as_slice(), universe),
        ] {
            sink.part(b"group", tag);
            for binding in bindings {
                binding.source.identity().encode(&mut sink);
                binding.immutable.content().encode(&mut sink);
                sink.part(b"actual-rows", &binding.immutable.rows().to_le_bytes());
                sink.part(b"role", &(binding.role as u64).to_le_bytes());
                sink.part(
                    b"requested",
                    binding
                        .requested
                        .map_or("", PublicationBoundary::name)
                        .as_bytes(),
                );
                sink.part(
                    b"resolved",
                    binding
                        .resolved
                        .map_or("", PublicationBoundary::name)
                        .as_bytes(),
                );
                if include_provider {
                    sink.part(
                        b"actual-provider",
                        &(Arc::as_ptr(&binding.provider) as *const () as usize).to_le_bytes(),
                    );
                }
                for contribution in &binding.contributions {
                    contribution.encode(&mut sink);
                }
            }
        }
        for owner in owners {
            owner.identity()?.encode(&mut sink);
        }
        Ok(sink.finish())
    }
    fn register<'a>(
        &self,
        session: &SessionContext,
        bindings: &'a [PrivateBinding],
        prefix: &str,
    ) -> Result<Vec<Registered<'a>>, ModelError> {
        bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| {
                let declared = self
                    .model
                    .relation(binding.relation.name())
                    .ok_or(ModelError::Schema(binding.relation.name()))?;
                if !binding.immutable.shares_pool(&self.budget)
                    || !Arc::ptr_eq(&binding.provider, &binding.immutable.provider())
                    || declared.type_id() != binding.relation.type_id()
                    || declared.schema().as_ref() != binding.relation.schema().as_ref()
                    || binding.source.model() != self.model.digest()
                    || binding.source.relation() != binding.relation.name()
                    || binding.source.rows() < 0
                    || binding.provider.schema().as_ref() != binding.relation.schema().as_ref()
                {
                    return Err(ModelError::Conflict(
                        "private selected provider/model/schema binding",
                    ));
                }
                let table = format!("private_{prefix}_{index}");
                session
                    .register_table(table.as_str(), binding.provider.clone())
                    .map_err(crate::sql::model_error)?;
                Ok(Registered { binding, table })
            })
            .collect()
    }
    async fn check_rows(
        &self,
        session: &SessionContext,
        source: &Registered<'_>,
    ) -> Result<(), ModelError> {
        let sql = format!("SELECT * FROM {} ORDER BY id", identifier(&source.table));
        let mut rows = crate::sql::query(session, &sql)
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        let mut content = source.binding.relation.content();
        let mut received_rows = 0u64;
        while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
            self.cancellation.check()?;
            let received = batch.num_rows();
            received_rows = received_rows
                .checked_add(received as u64)
                .ok_or(ModelError::Conflict("private source row overflow"))?;
            if received_rows > source.binding.source.rows() as u64 {
                return Err(ModelError::Conflict("private source exceeds exact view"));
            }
            let checked =
                source
                    .binding
                    .relation
                    .checked_batch(&self.model, batch, &self.budget)?;
            if checked.fingerprints().len() != received {
                return Err(ModelError::Conflict(
                    "private source duplicate nominal rows",
                ));
            }
            for fingerprint in checked.fingerprints() {
                content.push_fingerprint(*fingerprint)?;
            }
        }
        let (actual_rows, actual_content) = content.finish();
        if actual_rows != source.binding.source.rows() as u64
            || actual_rows != source.binding.immutable.rows()
            || actual_content != source.binding.immutable.content()
        {
            return Err(ModelError::Conflict(
                "private source exact typed EOF/content",
            ));
        }
        Ok(())
    }
    fn unrequested(&self, profile: Profile) -> BTreeSet<&'static str> {
        if profile != Profile::Catalog {
            return BTreeSet::new();
        }
        d::producer_contract::facts::ty_flow()
            .outputs
            .into_iter()
            .filter(|r| !d::stages::is_vocabulary(r.name()))
            .map(|r| r.name())
            .filter(|name| self.model.relation(name).is_some())
            .collect()
    }
    fn empty_root(
        &self,
        input: &ValidationInput,
        selected: &[Registered<'_>],
        unrequested: &BTreeSet<&str>,
    ) -> bool {
        resolve(input, selected)
            .map(|r| r.binding.source.rows() == 0)
            .unwrap_or_else(|_| unrequested.contains(input.name()))
    }
    async fn references(
        &self,
        session: &SessionContext,
        selected: &[Registered<'_>],
    ) -> Result<(), ModelError> {
        let mut groups = BTreeMap::<&str, Vec<&Registered<'_>>>::new();
        for source in selected {
            groups
                .entry(source.binding.relation.name())
                .or_default()
                .push(source);
        }
        for sources in groups.values() {
            let relation = &sources[0].binding.relation;
            let tables = sources.iter().map(|r| r.table.clone()).collect::<Vec<_>>();
            if let Some(sql) =
                crate::admission::nominal_payload_conflict_sql(relation.schema(), &tables)
            {
                self.refuse_rows(
                    session,
                    &sql,
                    "selected nominal reference payloads conflict",
                )
                .await?;
            }
        }
        for source in selected {
            for field in source
                .binding
                .relation
                .fields()
                .iter()
                .filter(|f| !f.list())
            {
                let Some((_, target)) = field.target() else {
                    continue;
                };
                let field_name = identifier(field.name());
                let sql = if let Some(targets) = groups.get(target) {
                    let subtype = if let Some(tag) = field.subtype() {
                        let tag_field = targets[0]
                            .binding
                            .relation
                            .sum()
                            .ok_or(ModelError::Schema("nominal subtype target"))?
                            .tag;
                        format!(
                            " OR t.{} IS NULL OR t.{} <> {tag}",
                            identifier(tag_field),
                            identifier(tag_field)
                        )
                    } else {
                        String::new()
                    };
                    let union = targets
                        .iter()
                        .map(|r| format!("SELECT * FROM {}", identifier(&r.table)))
                        .collect::<Vec<_>>()
                        .join(" UNION ALL ");
                    format!(
                        "SELECT s.{field_name} FROM {} s LEFT JOIN ({union}) t ON s.{field_name} = t.id WHERE s.{field_name} IS NOT NULL AND (t.id IS NULL{subtype}) LIMIT 1",
                        identifier(&source.table)
                    )
                } else {
                    format!(
                        "SELECT {field_name} FROM {} WHERE {field_name} IS NOT NULL LIMIT 1",
                        identifier(&source.table)
                    )
                };
                self.refuse_rows(
                    session,
                    &sql,
                    "private missing or wrong-subtype nominal reference",
                )
                .await?;
            }
        }
        Ok(())
    }
    async fn refuse_rows(
        &self,
        session: &SessionContext,
        sql: &str,
        reason: &'static str,
    ) -> Result<(), ModelError> {
        let mut rows = crate::sql::query(session, sql)
            .await
            .map_err(crate::sql::model_error)?
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
            self.cancellation.check()?;
            if batch.num_rows() != 0 {
                return Err(ModelError::Conflict(reason));
            }
        }
        Ok(())
    }
    /// Match actual sealed singleton outputs before treating a selected compact row as a
    /// contributor's result. Logical membership and a plausible count cannot establish this.
    fn check_uniform_sources(
        &self,
        selected: &[Registered<'_>],
        universe: &[Registered<'_>],
        owners: &[CompletedContribution],
    ) -> Result<(), ModelError> {
        let mut qualified =
            BTreeMap::<ContentHash, Arc<crate::private_source::PrivateRelation>>::new();
        let mut charge =
            StateCharge::new(&self.budget, "private compact exact contributor sources");
        for source in selected
            .iter()
            .filter(|source| source.binding.relation.name() == UniformNotRequested::NAME)
        {
            let mut union: Option<Arc<crate::private_source::PrivateRelation>> = None;
            for identity in &source.binding.contributions {
                self.cancellation.check()?;
                let exact = if let Some(exact) = qualified.get(identity) {
                    exact.clone()
                } else {
                    let mut matching = owners
                        .iter()
                        .filter(|owner| owner.identity().is_ok_and(|actual| actual == *identity));
                    let owner = matching.next().ok_or(ModelError::Conflict(
                        "private compact completed contributor absent",
                    ))?;
                    if matching.next().is_some() {
                        return Err(ModelError::Conflict(
                            "private compact completed contributor ambiguous",
                        ));
                    }
                    let output = owner.outputs.get(UniformNotRequested::NAME).ok_or(
                        ModelError::Conflict("private compact contributor output absent"),
                    )?;
                    let mut candidates = universe.iter().chain(selected).filter(|candidate| {
                        candidate.binding.relation.name() == UniformNotRequested::NAME
                            && candidate.binding.contributions.len() == 1
                            && candidate.binding.contributions.contains(identity)
                    });
                    let first = candidates.next().ok_or(ModelError::Conflict(
                        "private compact exact singleton source absent",
                    ))?;
                    for candidate in std::iter::once(first).chain(candidates) {
                        if candidate.binding.immutable.rows() != output.rows
                            || candidate.binding.immutable.content() != output.content
                        {
                            return Err(ModelError::Conflict(
                                "private compact actual singleton differs from completed output",
                            ));
                        }
                    }
                    charge.grow(128 + size_of::<Arc<crate::private_source::PrivateRelation>>())?;
                    qualified.insert(*identity, first.binding.immutable.clone());
                    first.binding.immutable.clone()
                };
                union = Some(match union {
                    None => exact,
                    Some(previous) => {
                        crate::private_source::PrivateRelation::merge(&previous, &exact)?
                    }
                });
            }
            let union = union.ok_or(ModelError::Conflict(
                "private compact selected contributor set empty",
            ))?;
            if union.rows() != source.binding.immutable.rows()
                || union.content() != source.binding.immutable.content()
            {
                return Err(ModelError::Conflict(
                    "private compact selected content differs from actual contributor union",
                ));
            }
            self.cancellation.check()?;
        }
        Ok(())
    }
    async fn uniform(
        &self,
        session: &SessionContext,
        profile: Profile,
        selected: &[Registered<'_>],
        universe: &[Registered<'_>],
        owners: &[CompletedContribution],
    ) -> Result<Vec<ValidatedUniformOwner>, ModelError> {
        self.check_uniform_sources(selected, universe, owners)?;
        let mut result = Vec::<ValidatedUniformOwner>::new();
        let mut prepared =
            BTreeMap::<ContentHash, (AnalysisUniformInputs, Vec<AnalysisSelectedBinding>)>::new();
        let mut preparation_charge =
            StateCharge::new(&self.budget, "private compact owner preparation");
        let required = AnalysisUniformInputs::inputs();
        for source in selected
            .iter()
            .filter(|r| r.binding.relation.name() == UniformNotRequested::NAME)
        {
            let sql = format!("SELECT * FROM {} ORDER BY id", identifier(&source.table));
            let mut rows = crate::sql::query(session, &sql)
                .await
                .map_err(crate::sql::model_error)?
                .execute_stream()
                .await
                .map_err(crate::sql::model_error)?;
            while let Some(batch) = rows.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                let _decode = self.budget.reserve(
                    "private compact descriptor decode",
                    d::decode_allowance::<UniformNotRequested>(&batch)?,
                )?;
                for row in UniformNotRequested::decode(&batch)? {
                    let matching = owners
                        .iter()
                        .filter(|owner| {
                            owner.spec.producer == row.owner
                                && owner
                                    .identity()
                                    .is_ok_and(|id| source.binding.contributions.contains(&id))
                        })
                        .collect::<Vec<_>>();
                    if matching.len() != 1 {
                        return Err(ModelError::Conflict(
                            "private compact exact contributor absent or ambiguous",
                        ));
                    }
                    let owner = matching[0];
                    let proof = match row.kind {
                        UniformKind::NativeFamily => {
                            ValidatedUniformOwner::validate_semantic_binding(
                                owner,
                                &row,
                                profile,
                                &self.budget,
                            )?
                        }
                        UniformKind::AnalysisOwner => {
                            let identity = owner.identity()?;
                            if let std::collections::btree_map::Entry::Vacant(entry) =
                                prepared.entry(identity)
                            {
                                let mut inputs = AnalysisUniformInputs::new(profile, &self.budget)?;
                                let mut bindings = Vec::new();
                                let mut loaded = BTreeSet::new();
                                for snapshot in &owner.spec.inputs {
                                    let found = universe
                                        .iter()
                                        .chain(selected)
                                        .filter(|r| r.binding.source == *snapshot)
                                        .collect::<Vec<_>>();
                                    if found.is_empty() {
                                        return Err(ModelError::Conflict(
                                            "private compact qualification input absent",
                                        ));
                                    }
                                    if found.iter().any(|candidate| {
                                        candidate.binding.immutable.rows()
                                            != found[0].binding.immutable.rows()
                                            || candidate.binding.immutable.content()
                                                != found[0].binding.immutable.content()
                                    }) {
                                        return Err(ModelError::Conflict(
                                            "private compact qualification input actual content ambiguous",
                                        ));
                                    }
                                    for r in &found {
                                        if !bindings.iter().any(|b: &AnalysisSelectedBinding| {
                                            b.source == *snapshot
                                                && b.boundary == r.binding.resolved
                                        }) {
                                            preparation_charge.grow(
                                                size_of::<AnalysisSelectedBinding>()
                                                    + snapshot.heap_bytes()
                                                    + 128,
                                            )?;
                                            bindings.push(AnalysisSelectedBinding {
                                                source: snapshot.clone(),
                                                boundary: r.binding.resolved,
                                            });
                                        }
                                    }
                                    if snapshot.relation() == UniformNotRequested::NAME
                                        || !required.iter().any(|r| r.name() == snapshot.relation())
                                        || !loaded.insert(snapshot.identity())
                                    {
                                        continue;
                                    }
                                    let r = found[0];
                                    let typed = selected.iter().any(|selected| {
                                        selected.binding.source == *snapshot
                                            && Arc::ptr_eq(
                                                &selected.binding.provider,
                                                &r.binding.provider,
                                            )
                                    });
                                    let sql = format!(
                                        "SELECT * FROM {} ORDER BY id",
                                        identifier(&r.table)
                                    );
                                    let mut metadata = crate::sql::query(session, &sql)
                                        .await
                                        .map_err(crate::sql::model_error)?
                                        .execute_stream()
                                        .await
                                        .map_err(crate::sql::model_error)?;
                                    let mut count = 0u64;
                                    while let Some(batch) = metadata
                                        .try_next()
                                        .await
                                        .map_err(crate::sql::model_error)?
                                    {
                                        self.cancellation.check()?;
                                        count = count.checked_add(batch.num_rows() as u64).ok_or(
                                            ModelError::Conflict(
                                                "private compact metadata overflow",
                                            ),
                                        )?;
                                        if count > snapshot.rows() as u64 {
                                            return Err(ModelError::Conflict(
                                                "private compact metadata exceeds exact view",
                                            ));
                                        }
                                        if typed {
                                            inputs.visit(snapshot.relation(), &batch)?;
                                        } else {
                                            let received = batch.num_rows();
                                            let checked = r.binding.relation.checked_batch(
                                                &self.model,
                                                batch,
                                                &self.budget,
                                            )?;
                                            if checked.fingerprints().len() != received {
                                                return Err(ModelError::Conflict(
                                                    "private compact duplicate metadata",
                                                ));
                                            }
                                            inputs.visit(snapshot.relation(), checked.arrow())?;
                                        }
                                    }
                                    if count != snapshot.rows() as u64 {
                                        return Err(ModelError::Conflict(
                                            "private compact exact metadata EOF",
                                        ));
                                    }
                                }
                                preparation_charge.grow(128)?;
                                entry.insert((inputs, bindings));
                            }
                            let (inputs, bindings) = &prepared[&identity];
                            ValidatedUniformOwner::validate_analysis_sources(
                                owner,
                                &row,
                                profile,
                                &self.model,
                                bindings,
                                inputs,
                                &self.budget,
                            )?
                        }
                    };
                    if result.iter().any(|p| {
                        p.descriptor().id() == row.id()
                            && (p.descriptor() != &row || p.contribution() != proof.contribution())
                    }) {
                        return Err(ModelError::Conflict("private compact duplicate owner"));
                    }
                    if !result.iter().any(|p| p.descriptor().id() == row.id()) {
                        result.push(proof);
                    }
                }
            }
        }
        Ok(result)
    }
}
fn resolve<'a, 'b>(
    input: &ValidationInput,
    selected: &'a [Registered<'b>],
) -> Result<&'a Registered<'b>, ModelError> {
    let mut choices = selected.iter().filter(|r| {
        r.binding.relation.name() == input.name() && r.binding.requested == input.prefix()
    });
    if let Some(first) = choices.next() {
        if choices.any(|r| {
            r.binding.source != first.binding.source
                || r.binding.resolved != first.binding.resolved
                || !Arc::ptr_eq(&r.binding.provider, &first.binding.provider)
        }) {
            return Err(ModelError::Conflict(
                "private selected input role ambiguous",
            ));
        }
        return Ok(first);
    }
    if input.prefix().is_some() {
        return Err(ModelError::Conflict("private selected input epoch absent"));
    }
    let mut choices = selected
        .iter()
        .filter(|r| r.binding.relation.name() == input.name());
    let first = choices
        .next()
        .ok_or(ModelError::Conflict("private selected input absent"))?;
    if choices.any(|r| {
        r.binding.source != first.binding.source
            || r.binding.resolved != first.binding.resolved
            || !Arc::ptr_eq(&r.binding.provider, &first.binding.provider)
    }) {
        return Err(ModelError::Conflict(
            "private selected input epoch ambiguous",
        ));
    }
    Ok(first)
}
fn binding_key(binding: &PrivateBinding) -> ContentHash {
    let mut sink = KeySink::new("private typed property source/v2");
    binding.source.identity().encode(&mut sink);
    binding.immutable.content().encode(&mut sink);
    sink.part(b"actual-rows", &binding.immutable.rows().to_le_bytes());
    sink.part(
        b"provider",
        &(Arc::as_ptr(&binding.provider) as *const () as usize).to_le_bytes(),
    );
    sink.finish()
}
fn poisoned() -> ModelError {
    ModelError::Conflict("private semantic ownership lock poisoned")
}

#[cfg(test)]
mod controls {
    use super::*;
    use d::input::{Package, Release};
    fn runtime_for(model: ValidatedModel) -> PrivateValidation {
        let model = Arc::new(model);
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        PrivateValidation::new(
            model,
            AdmissionOptions {
                memory_bytes: budget.limit(),
                ..Default::default()
            },
            budget,
            Cancellation::default(),
            SessionContext::new().runtime_env(),
        )
        .unwrap()
    }
    fn runtime() -> PrivateValidation {
        runtime_for(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        )
    }
    fn immutable<R: Record>(
        runtime: &PrivateValidation,
        rows: &[R],
        directory: Arc<tempfile::TempDir>,
    ) -> Arc<crate::private_source::PrivateRelation> {
        let mut writer = crate::private_source::PrivateWriter::new(
            runtime.model.digest(),
            Relation::of::<R>(),
            runtime.budget.clone(),
            directory,
        )
        .unwrap();
        writer
            .push(
                d::Batch::<R>::new(&runtime.model, rows.to_vec(), &runtime.budget)
                    .unwrap()
                    .into_checked()
                    .unwrap(),
            )
            .unwrap();
        writer.finish().unwrap()
    }
    fn completed(
        runtime: &PrivateValidation,
        immutable: Arc<crate::private_source::PrivateRelation>,
        role: usize,
        contributions: BTreeSet<ContentHash>,
    ) -> PrivateBinding {
        let view = d::completed::CompletedView::new(
            immutable.relation().name().into(),
            contributions.clone(),
            immutable.rows(),
        )
        .unwrap();
        let source =
            SourceSnapshot::of_completed_view(immutable.relation(), runtime.model.digest(), &view)
                .unwrap();
        PrivateBinding::completed(role, None, None, source, contributions, immutable).unwrap()
    }
    fn binding<R: Record>(runtime: &PrivateValidation, rows: &[R], role: usize) -> PrivateBinding {
        completed(
            runtime,
            immutable(runtime, rows, Arc::new(tempfile::tempdir().unwrap())),
            role,
            BTreeSet::from([ContentHash::of(R::NAME.as_bytes())]),
        )
    }
    #[tokio::test]
    async fn equal_logical_metadata_never_erases_different_actual_content() {
        let runtime = runtime();
        let left = binding(
            &runtime,
            &[Package {
                name: "left-source".into(),
            }],
            0,
        );
        let right = binding(
            &runtime,
            &[Package {
                name: "right-source".into(),
            }],
            0,
        );
        assert_eq!(left.source, right.source);
        assert_eq!(left.contributions, right.contributions);
        assert_ne!(left.immutable.content(), right.immutable.content());
        assert_ne!(binding_key(&left), binding_key(&right));
        let first = runtime
            .validate(Profile::Catalog, vec![left.clone()], vec![], vec![])
            .await
            .unwrap();
        let second = runtime
            .validate(Profile::Catalog, vec![right.clone()], vec![], vec![])
            .await
            .unwrap();
        assert_ne!(
            first.identity(),
            second.identity(),
            "public properties identify actual typed content"
        );
        let session =
            SessionContext::new_with_config_rt(SessionConfig::new(), runtime.runtime.clone());
        session
            .register_table("actual_left", left.provider.clone())
            .unwrap();
        let mut swapped = left;
        swapped.immutable = right.immutable;
        assert!(
            runtime
                .check_rows(
                    &session,
                    &Registered {
                        binding: &swapped,
                        table: "actual_left".into()
                    }
                )
                .await
                .is_err(),
            "equal EOF counts cannot replace the actual content comparison"
        );
    }
    fn uniform_owner(
        runtime: &PrivateValidation,
        name: &str,
        source: &crate::private_source::PrivateRelation,
    ) -> CompletedContribution {
        CompletedContribution {
            spec: d::completed::ContributionSpec {
                captured_binding: None,
                producer: name.into(),
                profile: d::completed::OutputProfile::Specific(Profile::Catalog),
                model: runtime.model.digest(),
                implementation: ContentHash::of(b"ownership-only-control"),
                configuration: None,
                inputs: vec![],
                outputs: BTreeSet::from([UniformNotRequested::NAME.into()]),
            },
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: BTreeMap::from([(
                UniformNotRequested::NAME.into(),
                d::completed::OutputContent {
                    rows: source.rows(),
                    content: source.content(),
                },
            )]),
        }
    }
    #[test]
    fn compact_ownership_refuses_wrong_singleton_and_equal_count_wrong_union() {
        // Exercise only the ownership layer; these fixtures do not claim domain qualification.
        let runtime = runtime_for(d::model().unwrap());
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let input = d::input::InputRevision::from_entries(vec![]).unwrap();
        let context = d::attribution::AnalysisContext {
            python_version: "3.14.8".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"compact-content-control"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let first_row = UniformNotRequested {
            kind: UniformKind::AnalysisOwner,
            owner: "first-compact".into(),
            family: None,
            input: input.id(),
            context: context.id(),
            declaration: ContentHash::of(b"declaration"),
            sources: ContentHash::of(b"sources"),
            definitions: ContentHash::of(b"definitions"),
            domain: ContentHash::of(b"domain"),
            members: 0,
        };
        let mut second_row = first_row.clone();
        second_row.owner = "second-compact".into();
        let a = immutable(
            &runtime,
            std::slice::from_ref(&first_row),
            directory.clone(),
        );
        let b = immutable(
            &runtime,
            std::slice::from_ref(&second_row),
            directory.clone(),
        );
        let owner_a = uniform_owner(&runtime, &first_row.owner, &a);
        let owner_b = uniform_owner(&runtime, &second_row.owner, &b);
        let a_binding = completed(
            &runtime,
            a.clone(),
            0,
            BTreeSet::from([owner_a.identity().unwrap()]),
        );
        let b_binding = completed(
            &runtime,
            b.clone(),
            1,
            BTreeSet::from([owner_b.identity().unwrap()]),
        );
        let union = crate::private_source::PrivateRelation::merge(&a, &b).unwrap();
        let members = BTreeSet::from([owner_a.identity().unwrap(), owner_b.identity().unwrap()]);
        let selected = completed(&runtime, union, 0, members.clone());
        let exact = [
            Registered {
                binding: &a_binding,
                table: "unused_a".into(),
            },
            Registered {
                binding: &b_binding,
                table: "unused_b".into(),
            },
        ];
        runtime
            .check_uniform_sources(
                &[Registered {
                    binding: &selected,
                    table: "unused_union".into(),
                }],
                &exact,
                &[owner_a.clone(), owner_b.clone()],
            )
            .unwrap();
        let mut other_row = second_row;
        other_row.owner = "unowned-compact".into();
        let wrong_union = completed(
            &runtime,
            immutable(&runtime, &[first_row, other_row], directory),
            0,
            members,
        );
        assert_eq!(selected.immutable.rows(), wrong_union.immutable.rows());
        assert!(
            runtime
                .check_uniform_sources(
                    &[Registered {
                        binding: &wrong_union,
                        table: "unused_wrong_union".into()
                    }],
                    &exact,
                    &[owner_a.clone(), owner_b]
                )
                .is_err()
        );
        let mut wrong_owner = owner_a;
        wrong_owner
            .outputs
            .get_mut(UniformNotRequested::NAME)
            .unwrap()
            .content = ContentHash::of(b"invented-singleton-content");
        let false_binding = completed(
            &runtime,
            a,
            0,
            BTreeSet::from([wrong_owner.identity().unwrap()]),
        );
        assert!(
            runtime
                .check_uniform_sources(
                    &[Registered {
                        binding: &false_binding,
                        table: "unused_wrong_singleton".into()
                    }],
                    &[],
                    &[wrong_owner]
                )
                .is_err()
        );
    }
    #[derive(
        Clone, Debug, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "private_ledger_rows", invariant_refs = ledger_refs)]
    struct LedgerRow {
        #[model(key)]
        key: String,
        payload: String,
    }
    fn ledger_refs() -> Vec<&'static str> {
        vec!["aa_private_success", "zz_private_refusal"]
    }
    #[derive(
        Clone, Debug, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "private_ledger_dependencies")]
    struct LedgerDependency {
        #[model(key)]
        key: String,
    }
    #[derive(
        Clone, Debug, PartialEq, Eq, lctx_model::Domain, serde::Serialize, serde::Deserialize,
    )]
    #[model(name = "private_empty_ledger_rows", invariant_refs = empty_ledger_refs)]
    struct EmptyLedgerRow {
        #[model(key)]
        key: String,
    }
    fn empty_ledger_refs() -> Vec<&'static str> {
        vec!["private_empty_applicability"]
    }
    struct LedgerCheck {
        reject: bool,
        successes: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl d::InvariantCheck for LedgerCheck {
        fn visit(
            &mut self,
            relation: &str,
            batch: &arrow_array::RecordBatch,
        ) -> Result<(), ModelError> {
            if relation == LedgerRow::NAME
                && self.reject
                && LedgerRow::decode(batch)?
                    .iter()
                    .any(|row| row.payload == "forbidden")
            {
                return Err(ModelError::Conflict("later semantic predicate refused"));
            }
            Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            self.successes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }
    fn ledger_runtime() -> (PrivateValidation, Arc<std::sync::atomic::AtomicUsize>) {
        let successes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let definitions = [("aa_private_success", false), ("zz_private_refusal", true)]
            .into_iter()
            .map(|(name, reject)| {
                let successes = successes.clone();
                d::Invariant {
                    purpose: d::InvariantPurpose::Admission,
                    revision: 1,
                    name,
                    inputs: vec![
                        ValidationInput::of::<LedgerRow>(&["id"]),
                        ValidationInput::of::<LedgerDependency>(&["id"]),
                    ],
                    create: Arc::new(move |_| {
                        Box::new(LedgerCheck {
                            reject,
                            successes: successes.clone(),
                        })
                    }),
                }
            })
            .collect::<Vec<_>>();
        let mut model = ValidatedModel::validate(
            vec![
                Relation::of::<LedgerRow>(),
                Relation::of::<LedgerDependency>(),
            ],
            d::ValidationDefinitions {
                invariants: definitions.clone(),
                publication_checks: vec![],
            },
        )
        .unwrap();
        for invariant in definitions {
            model
                .register_factory(
                    invariant.identity(),
                    ContentHash::of(b"executed ledger control"),
                    vec![],
                )
                .unwrap();
        }
        (runtime_for(model), successes)
    }
    fn ledger_bindings(runtime: &PrivateValidation, name: &str) -> Vec<PrivateBinding> {
        let row = LedgerRow {
            key: "stable-key".into(),
            payload: name.into(),
        };
        let dependency = LedgerDependency {
            key: "dependency".into(),
        };
        vec![
            binding(runtime, &[row], 0),
            binding(runtime, &[dependency], 1),
        ]
    }
    #[tokio::test]
    async fn retained_executed_ledger_matches_complete_context_and_operation_after_scope_drop() {
        use d::artifact_admission::Execution;
        let (runtime, successes) = ledger_runtime();
        let selected = ledger_bindings(&runtime, "allowed");
        let source = Arc::downgrade(selected[0].immutable_source());
        let scope = runtime
            .validate(Profile::Catalog, selected, vec![], vec![])
            .await
            .unwrap();
        assert_eq!(successes.load(std::sync::atomic::Ordering::SeqCst), 2);
        let premise = scope._executed[0].premise().clone();
        assert_eq!(scope._executed[0].execution(), Execution::WholeInput);
        assert!(scope._executed[0].whole().is_some());
        drop(scope);
        assert!(
            source.upgrade().is_some(),
            "materialization's operation ledger keeps actual source ownership"
        );
        let retained = runtime
            .find_executed(&premise, &runtime.budget, &runtime.cancellation)
            .unwrap()
            .unwrap();
        let mut changed = Vec::new();
        let mut next = premise.clone();
        next.inputs.swap(0, 1);
        changed.push(next);
        let mut next = premise.clone();
        next.inputs[0].order = vec!["payload"];
        changed.push(next);
        let mut next = premise.clone();
        next.inputs[0].requested = Some(PublicationBoundary::Local);
        changed.push(next);
        let mut next = premise.clone();
        next.inputs[0].selected.as_mut().unwrap().0 = Some(PublicationBoundary::Local);
        changed.push(next);
        let mut next = premise.clone();
        next.inputs[0].selected = None;
        next.inputs[0].not_requested = Some(ContentHash::of(b"different demand"));
        changed.push(next);
        let mut next = premise.clone();
        next.definition = ContentHash::of(b"different definition");
        changed.push(next);
        let mut next = premise.clone();
        next.factory = ContentHash::of(b"different factory");
        changed.push(next);
        let mut next = premise.clone();
        next.profile = Profile::Behavioral;
        changed.push(next);
        let mut next = premise.clone();
        next.uniform_owners.push(UniformOwnerPremise {
            contribution: ContentHash::of(b"lookup-only context"),
            descriptor: [7; 16],
            content: ContentHash::of(b"lookup-only content"),
            profile: Profile::Catalog,
        });
        changed.push(next);
        for next in changed {
            assert!(
                runtime
                    .find_executed(&next, &runtime.budget, &runtime.cancellation)
                    .unwrap()
                    .is_none()
            );
        }
        let foreign = ResourceBudget::fixed(runtime.budget.limit()).unwrap();
        assert!(
            runtime
                .find_executed(&premise, &foreign, &runtime.cancellation)
                .is_err()
        );
        assert!(
            runtime
                .find_executed(&premise, &runtime.budget, &Cancellation::default())
                .is_err()
        );
        assert!(
            retained
                .check_current(runtime.model.digest(), &foreign, &runtime.cancellation)
                .is_err()
        );
        // SourceSnapshot headers can coincide across actual different content. The ledger
        // retains each genuine success but refuses an arbitrary metadata-only nomination.
        let other = runtime
            .validate(
                Profile::Catalog,
                ledger_bindings(&runtime, "other-allowed"),
                vec![],
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(other._executed[0].premise(), &premise);
        assert!(
            runtime
                .find_executed(&premise, &runtime.budget, &runtime.cancellation)
                .unwrap()
                .is_none()
        );
        runtime.cancellation.cancel();
        assert!(
            runtime
                .find_executed(&premise, &runtime.budget, &runtime.cancellation)
                .is_err()
        );
        assert!(
            retained
                .check_current(
                    runtime.model.digest(),
                    &runtime.budget,
                    &runtime.cancellation
                )
                .is_err()
        );
    }
    #[tokio::test]
    async fn failed_validation_prefix_publishes_no_executed_kernel_properties() {
        let (runtime, successes) = ledger_runtime();
        assert!(
            runtime
                .validate(
                    Profile::Catalog,
                    ledger_bindings(&runtime, "forbidden"),
                    vec![],
                    vec![]
                )
                .await
                .is_err()
        );
        assert_eq!(
            successes.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the first genuine predicate succeeded before a later predicate refused"
        );
        assert!(
            runtime.executed.lock().unwrap().0.is_empty(),
            "a failed aggregate never publishes its successful prefix"
        );
        assert!(runtime.completed.lock().unwrap().is_empty());
    }
    struct EmptyLedgerCheck;
    impl d::InvariantCheck for EmptyLedgerCheck {
        fn execution_scope(&self) -> Option<d::execution::fidelity::ExecutionScope> {
            Some(d::execution::fidelity::ExecutionScope {
                root: ValidationInput::of::<EmptyLedgerRow>(&["id"]),
                memberships: vec![],
                joins: vec![],
            })
        }
        fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<(), ModelError> {
            Err(ModelError::Conflict(
                "empty applicability must not execute a whole callback",
            ))
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            Err(ModelError::Conflict(
                "empty applicability has no whole callback proof",
            ))
        }
    }
    #[tokio::test]
    async fn complete_empty_root_retains_separate_applicability_without_whole_proof() {
        let invariant = d::Invariant {
            purpose: d::InvariantPurpose::Admission,
            revision: 1,
            name: "private_empty_applicability",
            inputs: vec![
                ValidationInput::of::<EmptyLedgerRow>(&["id"]),
                ValidationInput::of::<LedgerDependency>(&["id"]),
            ],
            create: Arc::new(|_| Box::new(EmptyLedgerCheck)),
        };
        let mut model = ValidatedModel::validate(
            vec![
                Relation::of::<EmptyLedgerRow>(),
                Relation::of::<LedgerDependency>(),
            ],
            d::ValidationDefinitions {
                invariants: vec![invariant.clone()],
                publication_checks: vec![],
            },
        )
        .unwrap();
        model
            .register_factory(
                invariant.identity(),
                ContentHash::of(b"empty applicability control"),
                vec![],
            )
            .unwrap();
        let runtime = runtime_for(model);
        let scope = runtime
            .validate(
                Profile::Catalog,
                vec![binding::<EmptyLedgerRow>(&runtime, &[], 0)],
                vec![],
                vec![],
            )
            .await
            .unwrap();
        let executed = &scope._executed[0];
        assert_eq!(
            executed.execution(),
            d::artifact_admission::Execution::EmptyDomain
        );
        assert!(executed.whole().is_none());
        assert!(executed.premise().inputs[1].selected.is_none());
        let premise = executed.premise().clone();
        drop(scope);
        assert!(
            runtime
                .find_executed(&premise, &runtime.budget, &runtime.cancellation)
                .unwrap()
                .is_some()
        );
    }
    #[derive(Debug)]
    struct LateFailure(arrow_array::RecordBatch);
    #[derive(Debug)]
    struct LatePartition(arrow_array::RecordBatch);
    impl datafusion::physical_plan::streaming::PartitionStream for LatePartition {
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
                            "late private source EOF".into(),
                        )),
                    ]),
                ),
            )
        }
    }
    #[async_trait::async_trait]
    impl TableProvider for LateFailure {
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
                    vec![Arc::new(LatePartition(self.0.clone()))],
                    projection,
                    vec![],
                    false,
                    None,
                )?,
            ))
        }
    }
    #[tokio::test]
    async fn private_semantics_checks_actual_reference_content_and_retains_exact_success_only() {
        let runtime = runtime();
        let package = Package {
            name: "private-provider".into(),
        };
        let release = Release {
            package: package.id(),
            version: "1.0".into(),
        };
        let selected = vec![
            binding(&runtime, &[package], 0),
            binding(&runtime, &[release], 1),
        ];
        let first = runtime
            .validate(Profile::Catalog, selected.clone(), selected.clone(), vec![])
            .await
            .unwrap();
        let second = runtime
            .validate(Profile::Catalog, selected.clone(), selected.clone(), vec![])
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(first.uniform_owners().is_empty());
        assert!(first.admitted_uniform_owners(&runtime.budget).is_ok());
        assert!(
            first
                .admitted_uniform_owners(&ResourceBudget::fixed(runtime.budget.limit()).unwrap())
                .is_err()
        );
        let behavioral = runtime
            .validate(
                Profile::Behavioral,
                selected.clone(),
                selected.clone(),
                vec![],
            )
            .await
            .unwrap();
        assert_ne!(
            first.identity(),
            behavioral.identity(),
            "profile is part of the executed premise"
        );
        assert!(!Arc::ptr_eq(&first, &behavioral));
        let successful = runtime.completed.lock().unwrap().len();
        let mut wrong_model = selected.clone();
        wrong_model[0].source = SourceSnapshot::of_relation(
            &wrong_model[0].relation,
            "private-fixture",
            ContentHash::of(b"foreign-model"),
            ContentHash::of(b"private-fixture-program"),
            wrong_model[0].source.view(),
            1,
        )
        .unwrap();
        assert!(
            runtime
                .validate(Profile::Catalog, wrong_model.clone(), wrong_model, vec![])
                .await
                .is_err()
        );
        let mut wrong_role = selected.clone();
        wrong_role[0].requested = Some(PublicationBoundary::Facts);
        wrong_role[0].resolved = Some(PublicationBoundary::Facts);
        assert!(
            runtime
                .validate(Profile::Catalog, wrong_role.clone(), wrong_role, vec![])
                .await
                .is_err(),
            "ordinary input cannot borrow a fabricated vocabulary role"
        );
        let mut late = selected.clone();
        late[0].provider = Arc::new(LateFailure(
            Package::encode(&[Package {
                name: "private-provider".into(),
            }])
            .unwrap(),
        ));
        assert!(
            runtime
                .validate(Profile::Catalog, late.clone(), late, vec![])
                .await
                .is_err(),
            "an unqualified replacement provider cannot borrow sealed source authority"
        );
        let mut missing = selected.clone();
        missing[0] = binding::<Package>(&runtime, &[], 0);
        assert!(
            runtime
                .validate(Profile::Catalog, missing.clone(), missing, vec![])
                .await
                .is_err()
        );
        let mut wrong_count = selected.clone();
        wrong_count[1].source = SourceSnapshot::of_relation(
            &wrong_count[1].relation,
            "private-fixture",
            runtime.model.digest(),
            ContentHash::of(b"private-fixture-program"),
            ContentHash::of(b"wrong-count"),
            2,
        )
        .unwrap();
        assert!(
            runtime
                .validate(Profile::Catalog, wrong_count.clone(), wrong_count, vec![])
                .await
                .is_err()
        );
        assert_eq!(
            runtime.completed.lock().unwrap().len(),
            successful,
            "failed acquisitions publish no reusable prefix"
        );
        runtime.cancellation.cancel();
        assert!(
            runtime
                .validate(Profile::Catalog, selected.clone(), selected, vec![])
                .await
                .is_err(),
            "memo success never hides cancellation"
        );
    }
}
