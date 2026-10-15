//! Private executed grouped checks within one live admission operation. There is no
//! serialized authority and no route through the whole-input replay proof driver.
use super::*;
use lctx_model::domain::{HeapSize, artifact_admission::Execution};
use std::sync::Weak;

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputPremise {
    // Position in this ordered vector is the invariant's semantic role. Inventory
    // positions are incidental and can change when unrelated relations complete.
    name: &'static str,
    order: Vec<&'static str>,
    requested: Option<PublicationBoundary>,
    selected: Option<(
        Option<PublicationBoundary>,
        CompletedView,
        lctx_model::domain::analysis::sources::SourceSnapshot,
    )>,
    not_requested: Option<ContentHash>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopedPremise {
    model: ContentHash,
    profile: Profile,
    name: &'static str,
    revision: u32,
    definition: ContentHash,
    factory: ContentHash,
    inputs: Vec<InputPremise>,
    // Full producer descriptors include model, profile, implementation, config and
    // exact dependency snapshots. Compact-consuming checks conservatively execute
    // afresh whenever compact owners exist.
    owners: Vec<CompletedContribution>,
    uniform_independent: bool,
}
struct Operation {
    // These links compare allocation identity only, never acquire native authority.
    // Erasure keeps downstream Send proofs out of the concrete ownership graphs.
    native: Weak<dyn Send + Sync>,
    calls: Weak<dyn Send + Sync>,
    execution: Option<Weak<dyn ExecutionOwner>>,
    cancellation: Cancellation,
    budget: ResourceBudget,
}
fn weak_identity(owner: Arc<dyn Send + Sync>) -> Weak<dyn Send + Sync> {
    Arc::downgrade(&owner)
}
impl Operation {
    fn of(runtime: &AdmissionRuntime) -> Self {
        Self {
            native: weak_identity(runtime.native.clone()),
            calls: weak_identity(runtime.native_calls.clone()),
            execution: runtime.borrowed_execution.as_ref().map(Arc::downgrade),
            cancellation: runtime.cancellation.clone(),
            budget: runtime.budget.clone(),
        }
    }
    fn matches(&self, runtime: &AdmissionRuntime) -> bool {
        self.native.ptr_eq(&weak_identity(runtime.native.clone()))
            && self
                .calls
                .ptr_eq(&weak_identity(runtime.native_calls.clone()))
            && self.budget.shares_pool(runtime.budget())
            && Arc::ptr_eq(&self.cancellation.0, &runtime.cancellation.0)
            && match (&self.execution, &runtime.borrowed_execution) {
                (None, None) => true,
                (Some(known), Some(actual)) => known.ptr_eq(&Arc::downgrade(actual)),
                _ => false,
            }
    }
}
/// Kept crate-private so only the successful specialized production branch can
/// construct it. Weak operation links avoid an execution-owner -> memo -> owner cycle.
pub(crate) struct ExecutedScopedCheck {
    premise: ScopedPremise,
    operation: Operation,
    caps: Vec<Arc<lctx_native::installed_view::HeldInstalledView>>,
    execution: Execution,
    _charge: StateCharge,
}
impl ExecutedScopedCheck {
    pub(super) fn execution(&self) -> Execution {
        self.execution
    }
}
pub(super) struct PreparedScopedCheck {
    premise: ScopedPremise,
    operation: Operation,
    caps: Vec<Arc<lctx_native::installed_view::HeldInstalledView>>,
    charge: StateCharge,
}
impl PreparedScopedCheck {
    // This call follows the actual scoped kernel/empty-root stream's terminal Ok.
    pub(super) fn finish(
        self,
        execution: Execution,
        cancellation: &Cancellation,
    ) -> Result<Arc<ExecutedScopedCheck>, ModelError> {
        cancellation.check()?;
        if execution == Execution::WholeInput {
            return Err(ModelError::Conflict(
                "whole-input result is not scoped evidence",
            ));
        }
        Ok(Arc::new(ExecutedScopedCheck {
            premise: self.premise,
            operation: self.operation,
            caps: self.caps,
            execution,
            _charge: self.charge,
        }))
    }
}
pub(super) struct ScopedChecks {
    entries: Mutex<(Vec<Arc<ExecutedScopedCheck>>, StateCharge)>,
    invalidated: AtomicBool,
    budget: ResourceBudget,
    #[cfg(test)]
    executions: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    reuses: std::sync::atomic::AtomicUsize,
}
impl ScopedChecks {
    pub(super) fn new(budget: &ResourceBudget) -> Self {
        Self {
            entries: Mutex::new((
                Vec::new(),
                StateCharge::new(budget, "private scoped check slots"),
            )),
            invalidated: AtomicBool::new(false),
            budget: budget.clone(),
            #[cfg(test)]
            executions: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            reuses: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub(super) fn close(&self) -> Result<(), ModelError> {
        self.invalidated.store(true, Ordering::Release);
        let mut entries = self.entries.lock().map_err(|_| poisoned())?;
        entries.0 = Vec::new();
        let bytes = entries.1.reserved();
        entries.1.release(bytes);
        Ok(())
    }
    pub(super) fn attempt(&self) -> ScopedAttempt<'_> {
        ScopedAttempt {
            cache: self,
            pending: Vec::new(),
            charge: StateCharge::new(&self.budget, "pending executed scoped check slots"),
            complete: false,
        }
    }
}
/// A failed, cancelled or dropped validation prefix publishes no successes and
/// invalidates earlier reuse for this operation. Existing borrowers keep their charge.
pub(super) struct ScopedAttempt<'a> {
    cache: &'a ScopedChecks,
    pending: Vec<Arc<ExecutedScopedCheck>>,
    charge: StateCharge,
    complete: bool,
}
impl ScopedAttempt<'_> {
    pub(super) fn retain(&mut self, check: Arc<ExecutedScopedCheck>) -> Result<(), ModelError> {
        self.charge.grow(size_of::<Arc<ExecutedScopedCheck>>())?;
        self.pending.reserve_exact(1);
        self.pending.push(check);
        #[cfg(test)]
        self.cache.executions.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    pub(super) fn commit(&mut self) -> Result<(), ModelError> {
        if !self.cache.invalidated.load(Ordering::Acquire) {
            let mut entries = self.cache.entries.lock().map_err(|_| poisoned())?;
            for check in &self.pending {
                if !entries.0.iter().any(|prior| {
                    prior.premise == check.premise
                        && prior.operation.native.ptr_eq(&check.operation.native)
                        && prior.operation.calls.ptr_eq(&check.operation.calls)
                }) {
                    entries.1.grow(size_of::<Arc<ExecutedScopedCheck>>())?;
                    entries.0.reserve_exact(1);
                    entries.0.push(check.clone());
                }
            }
        }
        self.complete = true;
        Ok(())
    }
}
impl Drop for ScopedAttempt<'_> {
    fn drop(&mut self) {
        if !self.complete {
            self.cache.invalidated.store(true, Ordering::Release);
            if let Ok(mut entries) = self.cache.entries.lock() {
                entries.0.clear();
            }
        }
    }
}
/// One immutable invocation inventory. Identity/correspondence is prepared once,
/// then each check looks up only its actual declared contributors.
pub(super) struct ScopedInventory {
    owners: BTreeMap<ContentHash, CompletedContribution>,
    _transfer: Box<dyn Reservation>,
    _charge: StateCharge,
}
impl AdmissionRuntime {
    pub(super) fn scoped_inventory(&self) -> Result<ScopedInventory, ModelError> {
        let inventory = self.completed_inventory()?;
        let mut owners = BTreeMap::new();
        let mut charge = StateCharge::new(self.budget(), "scoped invocation owner correspondence");
        for owner in inventory.contributions {
            charge.grow(128)?;
            if owners.insert(owner.identity()?, owner).is_some() {
                return Err(ModelError::Conflict("duplicate scoped invocation owner"));
            }
        }
        Ok(ScopedInventory {
            owners,
            _transfer: inventory.transfer,
            _charge: charge,
        })
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Exact check roles, applicability, uniform dependency and invocation inventory stay explicit"
    )]
    pub(super) async fn scoped_premise(
        &self,
        invariant: &lctx_model::domain::Invariant,
        inputs: &ValidationInputs,
        unrequested: &BTreeSet<&'static str>,
        profile: Profile,
        uniform_independent: bool,
        uniform_empty: bool,
        inventory: &ScopedInventory,
    ) -> Result<Option<PreparedScopedCheck>, ModelError> {
        // Captured companion rows have no native capability in this route. They
        // continue to execute independently, as do unregistered executable factories.
        let Some(factory) = self.model.factory_contract(invariant.identity()) else {
            return Ok(None);
        };
        if (!uniform_independent && !uniform_empty)
            || !inputs.companions.is_empty()
            || !self.memo.scoped.budget.shares_pool(self.budget())
            || self.memo.scoped.invalidated.load(Ordering::Acquire)
        {
            return Ok(None);
        }
        self.cancellation.check()?;
        self.native.check()?;
        let mut charge = StateCharge::new(self.budget(), "private executed scoped check");
        charge.grow(size_of::<ExecutedScopedCheck>().saturating_add(1024))?;
        let mut roles = Vec::new();
        let mut views = BTreeMap::new();
        for input in &invariant.inputs {
            charge.grow(512usize.saturating_add(input.order().len().saturating_mul(32)))?;
            let (selected, not_requested) = if unrequested.contains(input.name()) {
                (
                    None,
                    Some(lctx_model::domain::producer_contract::facts::ty_flow().identity()),
                )
            } else if let Ok(binding) = inputs.validation_source(input) {
                charge.grow(
                    serde_json::to_vec(&binding.view)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_add(binding.snapshot.heap_bytes())
                        .saturating_mul(4),
                )?;
                views.insert(binding.view.identity, binding.view.clone());
                (
                    Some((
                        binding.resolved,
                        binding.view.clone(),
                        binding.snapshot.clone(),
                    )),
                    None,
                )
            } else {
                // Absence is explicit. Only actual complete-empty applicability can
                // produce a success with this role; adding it changes the full premise.
                (None, None)
            };
            roles.push(InputPremise {
                name: input.name(),
                order: input.order().to_vec(),
                requested: input.prefix(),
                selected,
                not_requested,
            });
        }
        let mut owners = Vec::new();
        for id in views
            .values()
            .flat_map(|view| &view.contributions)
            .collect::<BTreeSet<_>>()
        {
            let Some(owner) = inventory.owners.get(id) else {
                return Ok(None);
            };
            charge.grow(
                serde_json::to_vec(owner)
                    .map_err(ModelError::codec)?
                    .len()
                    .saturating_mul(8)
                    .saturating_add(512),
            )?;
            owners.push(owner.clone());
        }
        let mut caps = Vec::new();
        for view in views.into_values() {
            charge.grow(size_of::<Arc<lctx_native::installed_view::HeldInstalledView>>())?;
            let native = self.native.clone();
            let budget = self.budget.clone();
            caps.push(
                self.native_call(async move { native.installed_view(&view, &budget).await })
                    .await?,
            );
        }
        self.cancellation.check()?;
        Ok(Some(PreparedScopedCheck {
            premise: ScopedPremise {
                model: self.model.digest(),
                profile,
                name: invariant.name,
                revision: invariant.revision,
                definition: invariant.digest(),
                factory: factory.identity()?,
                inputs: roles,
                owners,
                uniform_independent,
            },
            operation: Operation::of(self),
            caps,
            charge,
        }))
    }
    pub(super) async fn reuse_scoped(
        &self,
        prepared: &PreparedScopedCheck,
    ) -> Result<Option<Arc<ExecutedScopedCheck>>, ModelError> {
        self.cancellation.check()?;
        self.native.check()?;
        if self.memo.scoped.invalidated.load(Ordering::Acquire) {
            return Ok(None);
        }
        let known = self
            .memo
            .scoped
            .entries
            .lock()
            .map_err(|_| poisoned())?
            .0
            .iter()
            .find(|known| known.premise == prepared.premise && known.operation.matches(self))
            .cloned();
        let Some(known) = known else { return Ok(None) };
        // Fresh final capabilities may be independently frozen replacements of
        // the operation-local selectors. Full exact mappings must agree, and the
        // ORIGINAL actually retained source pin must separately remain current.
        if known.caps.len() != prepared.caps.len()
            || known
                .caps
                .iter()
                .zip(&prepared.caps)
                .any(|(old, actual)| old.selector() != actual.selector())
        {
            return Ok(None);
        }
        for old in &known.caps {
            let native = self.native.clone();
            let cap = old.clone();
            let budget = self.budget.clone();
            self.native_call(async move { native.revalidate_scoped_source(cap, &budget).await })
                .await?;
        }
        self.native.check()?;
        known.operation.cancellation.check()?;
        self.cancellation.check()?;
        if self.memo.scoped.invalidated.load(Ordering::Acquire) {
            return Ok(None);
        }
        #[cfg(test)]
        self.memo.scoped.reuses.fetch_add(1, Ordering::SeqCst);
        tracing::debug!(
            invariant = known.premise.name,
            "reused privately executed scoped check"
        );
        Ok(Some(known))
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{
        self as d, Invariant, InvariantCheck, InvariantPurpose, ValidationDefinitions,
        ValidationInput,
        completed::{ContributionSpec, OutputContent, OutputProfile},
        input::{InputAcquisition, InputOrigin, InputRevision},
        producer_assurance::ProducerAssurance,
        stages::ProviderOutcome,
    };
    use std::sync::atomic::AtomicUsize;

    fn premise() -> ScopedPremise {
        let relation = Relation::of::<InputRevision>();
        let model = ContentHash::of(b"scoped premise model");
        let owner = CompletedContribution {
            spec: ContributionSpec {
                captured_binding: None,
                producer: "scoped-key-control".into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model,
                implementation: ContentHash::of(b"producer executable"),
                configuration: Some(ContentHash::of(b"settings")),
                inputs: vec![],
                outputs: [InputRevision::NAME.into()].into(),
            },
            assurance: ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: [(
                InputRevision::NAME.into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty"),
                },
            )]
            .into(),
        };
        let view = CompletedView::new(
            InputRevision::NAME.into(),
            [owner.identity().unwrap()].into(),
            0,
        )
        .unwrap();
        let source =
            d::analysis::sources::SourceSnapshot::of_completed_view(&relation, model, &view)
                .unwrap();
        ScopedPremise {
            model,
            profile: Profile::Catalog,
            name: "input_manifest_membership",
            revision: 1,
            definition: ContentHash::of(b"ordered declared invariant"),
            factory: ContentHash::of(b"factory implementation"),
            inputs: vec![InputPremise {
                name: InputRevision::NAME,
                order: vec!["id"],
                requested: None,
                selected: Some((None, view, source)),
                not_requested: None,
            }],
            owners: vec![owner],
            uniform_independent: true,
        }
    }
    #[test]
    fn scoped_premises_bind_roles_sources_selectors_factory_requestedness_and_config() {
        let original = premise();
        for field in [
            "model",
            "profile",
            "name",
            "revision",
            "definition",
            "factory",
            "role",
            "order",
            "requested",
            "resolved",
            "view",
            "source",
            "removed",
            "added",
            "not_requested",
            "configuration",
            "owner",
            "uniform_dependence",
        ] {
            let mut changed = original.clone();
            let different = ContentHash::of(b"different exact premise");
            match field {
                "model" => changed.model = different,
                "profile" => changed.profile = Profile::Behavioral,
                "name" => changed.name = "different_invariant",
                "revision" => changed.revision += 1,
                "definition" => changed.definition = different,
                "factory" => changed.factory = different,
                "role" => changed.inputs[0].name = InputAcquisition::NAME,
                "order" => changed.inputs[0].order.push("manifest"),
                "requested" => changed.inputs[0].requested = Some(PublicationBoundary::Local),
                "resolved" => {
                    changed.inputs[0].selected.as_mut().unwrap().0 =
                        Some(PublicationBoundary::Local)
                }
                "view" => changed.inputs[0].selected.as_mut().unwrap().1.rows += 1,
                "source" => changed.inputs[0].selected = None,
                "removed" => changed.inputs.clear(),
                "added" => changed.inputs.push(changed.inputs[0].clone()),
                "not_requested" => changed.inputs[0].not_requested = Some(different),
                "configuration" => changed.owners[0].spec.configuration = Some(different),
                "owner" => changed.owners[0].spec.implementation = different,
                "uniform_dependence" => changed.uniform_independent = false,
                _ => unreachable!(),
            }
            assert_ne!(original, changed, "{field} cannot inherit scoped evidence");
        }
        let mut ordered = original.clone();
        let mut second = ordered.inputs[0].clone();
        second.name = InputAcquisition::NAME;
        ordered.inputs.push(second);
        let mut reversed = ordered.clone();
        reversed.inputs.reverse();
        assert_ne!(
            ordered, reversed,
            "semantic role order is part of the complete premise"
        );
    }
    #[test]
    fn failed_or_dropped_scoped_prefix_invalidates_operation_reuse() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let successful = ScopedChecks::new(&budget);
        successful.attempt().commit().unwrap();
        assert!(!successful.invalidated.load(Ordering::Acquire));
        drop(successful.attempt());
        assert!(successful.invalidated.load(Ordering::Acquire));
        let closed = ScopedChecks::new(&budget);
        closed.close().unwrap();
        assert!(closed.invalidated.load(Ordering::Acquire));
        assert_eq!(
            budget.reserved(),
            0,
            "empty operation slots release their reservations"
        );
    }
    #[test]
    fn normalized_uniform_independence_is_explicit_and_structural_is_conservative() {
        let model = d::model().unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        for name in [
            "normalized_callable_admission",
            "normalized_event_admission",
            "normalized_binding_admission",
        ] {
            let check = (model.invariant(name).unwrap().create)(&budget);
            assert!(check.normalization_scope().is_some());
            assert!(
                check.scoped_uniform_independent(),
                "{name} uses the no-op compact owner contract"
            );
        }
        for name in ["structural_frame_fidelity", "structural_source_fidelity"] {
            let check = (model.invariant(name).unwrap().create)(&budget);
            assert!(
                !check.scoped_uniform_independent(),
                "{name} must conservatively earn its own compact premises"
            );
        }
    }
    struct Check {
        root: ValidationInput,
        rows: usize,
        finished: Arc<AtomicUsize>,
        rejected: Arc<AtomicUsize>,
    }
    impl InvariantCheck for Check {
        fn scoped_uniform_independent(&self) -> bool {
            true
        }
        fn execution_scope(&self) -> Option<d::execution::fidelity::ExecutionScope> {
            Some(d::execution::fidelity::ExecutionScope {
                root: self.root.clone(),
                memberships: vec![],
                joins: vec![],
            })
        }
        fn visit(&mut self, name: &str, batch: &RecordBatch) -> Result<(), ModelError> {
            if name == InputRevision::NAME {
                // Independent expected validity: exactly the manifest of zero entries.
                // A different manifest is rejected regardless of any cache observation.
                let expected = InputRevision::from_entries(vec![])?;
                for row in InputRevision::decode(batch)? {
                    if row != expected {
                        self.rejected.fetch_add(1, Ordering::SeqCst);
                        return Err(ModelError::Conflict(
                            "scoped control expected empty input manifest",
                        ));
                    }
                    self.rows += 1;
                }
            } else {
                InputAcquisition::decode(batch)?;
            }
            Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            if self.root.name() == InputRevision::NAME && self.rows != 1 {
                return Err(ModelError::Conflict(
                    "scoped control expected exactly one input revision",
                ));
            }
            self.finished.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    fn model(
        finished: &Arc<AtomicUsize>,
        rejected: &Arc<AtomicUsize>,
        changed_factory: bool,
    ) -> Arc<ValidatedModel> {
        let checks = [
            (
                "input_manifest_membership",
                ValidationInput::of::<InputRevision>(&["id"]),
            ),
            (
                "input_acquisition_boundaries",
                ValidationInput::of::<InputAcquisition>(&["id"]),
            ),
        ]
        .into_iter()
        .map(|(name, root)| {
            let finished = finished.clone();
            let rejected = rejected.clone();
            Invariant {
                name,
                purpose: InvariantPurpose::Admission,
                revision: 1,
                inputs: vec![root.clone()],
                create: Arc::new(move |_| {
                    Box::new(Check {
                        root: root.clone(),
                        rows: 0,
                        finished: finished.clone(),
                        rejected: rejected.clone(),
                    })
                }),
            }
        })
        .collect::<Vec<_>>();
        let mut model = ValidatedModel::validate(
            vec![
                Relation::of::<InputRevision>(),
                Relation::of::<InputAcquisition>(),
                Relation::of::<InputOrigin>(),
            ],
            ValidationDefinitions {
                invariants: checks.clone(),
                publication_checks: vec![],
            },
        )
        .unwrap();
        for check in checks {
            let mut implementation = d::KeySink::new("private-scoped-control-factory/v1");
            implementation.part(b"source", include_bytes!("scoped.rs"));
            implementation.part(b"variant", &[u8::from(changed_factory)]);
            model
                .register_factory(check.identity(), implementation.finish(), vec![])
                .unwrap();
        }
        Arc::new(model)
    }
    async fn seed(
        native: &Arc<NativeCompilerStore>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
        bad: bool,
        all: bool,
    ) -> (CompletedContribution, Vec<CompletedBinding>) {
        let mut relations = vec![Relation::of::<InputRevision>()];
        if all {
            relations.extend([
                Relation::of::<InputAcquisition>(),
                Relation::of::<InputOrigin>(),
            ]);
        }
        let id = native
            .begin_contribution(ContributionSpec {
                captured_binding: None,
                producer: format!("private-scoped-control-{}-{}", bad, all),
                profile: OutputProfile::Specific(Profile::Catalog),
                model: model.digest(),
                implementation: ContentHash::of(b"private scoped control production/v1"),
                configuration: None,
                inputs: vec![],
                outputs: relations
                    .iter()
                    .map(|relation| relation.name().into())
                    .collect(),
            })
            .await
            .unwrap();
        let row = if bad {
            InputRevision {
                manifest: ContentHash::of(b"independently invalid input manifest"),
            }
        } else {
            InputRevision::from_entries(vec![]).unwrap()
        };
        native
            .write_batch(
                &id,
                &relations[0],
                &InputRevision::encode(&[row]).unwrap(),
                budget,
            )
            .await
            .unwrap();
        if all {
            native
                .write_batch(
                    &id,
                    &relations[1],
                    &InputAcquisition::encode(&[]).unwrap(),
                    budget,
                )
                .await
                .unwrap();
            native
                .write_batch(
                    &id,
                    &relations[2],
                    &InputOrigin::encode(&[]).unwrap(),
                    budget,
                )
                .await
                .unwrap();
        }
        let views = native
            .complete_contribution(
                id,
                ProviderOutcome::Complete,
                &relations,
                &BTreeMap::new(),
                budget,
            )
            .await
            .unwrap();
        let owner = native.completed_contribution(id).await.unwrap();
        let bindings = views
            .into_values()
            .map(|view| CompletedBinding {
                boundary: None,
                configuration: None,
                source: d::analysis::sources::SourceSnapshot::of_completed_view(
                    model.relation(&view.relation).unwrap(),
                    model.digest(),
                    &view,
                )
                .unwrap(),
                view,
            })
            .collect();
        (owner, bindings)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Controls independently vary native, pool, executable and operation ownership"
    )]
    fn runtime(
        model: Arc<ValidatedModel>,
        native: &Arc<NativeCompilerStore>,
        budget: &ResourceBudget,
        memo: &Arc<AdmissionMemo>,
        cancellation: &Cancellation,
        calls: &Arc<crate::native_calls::NativeCalls>,
        owners: Vec<CompletedContribution>,
        bindings: Vec<CompletedBinding>,
        retained: &mut Vec<Arc<AdmissionRuntime>>,
    ) -> Arc<AdmissionRuntime> {
        let runtime = AdmissionRuntime::with_ownership(
            model,
            AdmissionOptions {
                memory_bytes: budget.limit(),
                ..AdmissionOptions::default()
            },
            budget.clone(),
            native.clone(),
            cancellation.clone(),
            Some(calls.clone()),
            Some(memo.clone()),
        )
        .unwrap();
        // Retain the real signal owner before restoration or any assertion can panic.
        retained.push(runtime.clone());
        runtime
            .install_completed(Profile::Catalog, owners, bindings)
            .unwrap();
        runtime
    }
    async fn drain_controls(
        native: &Arc<NativeCompilerStore>,
        calls: &Arc<crate::native_calls::NativeCalls>,
        runtimes: &[Arc<AdmissionRuntime>],
    ) -> d::completion::Completion {
        let mut terminal = d::completion::Completion::default();
        // All assertion futures have ended. Release every memo's retained selectors
        // before any runtime attempts to close their shared native source audit.
        for runtime in runtimes {
            terminal.step(
                "scoped control retained check release",
                runtime.memo.close_scoped(),
            );
        }
        for runtime in runtimes {
            let drained = runtime.drain_owned_report().await;
            if drained.local == d::completion::LocalState::Outstanding {
                terminal.local = d::completion::LocalState::Outstanding;
            }
            if drained.remote == d::completion::RemoteState::Unknown {
                terminal.remote = d::completion::RemoteState::Unknown;
            }
            terminal.storage.extend(drained.storage);
            terminal.committed.extend(drained.committed);
            terminal.failures.extend(drained.failures);
        }
        // Also cover setup failure before the first runtime was constructed.
        if runtimes.is_empty() {
            terminal.merge(calls.drain_finality_report().await);
        }
        // Execution drainage alone does not finish the original native signal.
        // Its real owner acknowledges attempt close and session invalidation here.
        let abandoned = native.abandon_finality_report().await;
        if abandoned.local == d::completion::LocalState::Outstanding {
            terminal.local = d::completion::LocalState::Outstanding;
        }
        if abandoned.remote == d::completion::RemoteState::Unknown {
            terminal.remote = d::completion::RemoteState::Unknown;
        }
        terminal.storage.extend(abandoned.storage);
        terminal.committed.extend(abandoned.committed);
        terminal.failures.extend(abandoned.failures);
        terminal
    }
    async fn finish_control_signals(
        runtimes: &[Arc<AdmissionRuntime>],
        terminal: &mut d::completion::Completion,
    ) {
        let mut finality = d::completion::Completion::default();
        for runtime in runtimes {
            // Every independent registration observes the same combined native/
            // runtime drainage and abandonment report, never a fresh success value.
            finality.step(
                "scoped control admission signal finality",
                runtime.complete_signal(terminal).await,
            );
        }
        terminal.step(
            "scoped control all admission signals",
            d::completion::complete(Ok(()), finality),
        );
    }
    async fn early(runtime: &AdmissionRuntime) {
        let inputs = runtime.inputs([InputRevision::NAME]).unwrap();
        let session = inputs.session(runtime).await.unwrap();
        let bindings = inputs
            .relations
            .values()
            .map(|input| SelectedBinding {
                role: input.role,
                requested: input.requested,
                resolved: input.resolved,
                binding: CompletedBinding {
                    boundary: None,
                    source: input.snapshot.clone(),
                    view: input.view.clone(),
                    configuration: None,
                },
            })
            .collect();
        runtime
            .validate_compiler_selection(Profile::Catalog, true, bindings, &session, &inputs)
            .await
            .unwrap();
    }
    /// Requires the explicitly owned native campaign config. It never installs,
    /// repairs, restarts or falls back from the selected target.
    #[tokio::test(flavor = "multi_thread")]
    async fn scoped_native_check_executes_once_through_final_freeze_and_new_applicability() {
        let config = lctx_native::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("explicitly owned native control campaign"),
        ))
        .unwrap();
        let native = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let cancellation = Cancellation::default();
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &budget,
        ));
        let memo = AdmissionMemo::new(&budget);
        let finished = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicUsize::new(0));
        let mut runtimes = Vec::new();
        let result = std::panic::AssertUnwindSafe(async {
            let model = model(&finished, &rejected, false);
            let (owner, bindings) = seed(&native, &model, &budget, false, true).await;
            let runtime = runtime(
                model.clone(),
                &native,
                &budget,
                &memo,
                &cancellation,
                &calls,
                vec![owner],
                bindings,
                &mut runtimes,
            );
            // Completing a contribution registers immutable views; installing the
            // runtime inventory alone does not publish the native compiler's exact
            // selected roots. Final freeze follows persisted bindings to its owner
            // closure, so establish the same handoff ordinary compilation performs.
            // Early validation still requests only InputRevision; the empty
            // InputAcquisition gate first becomes applicable at final admission.
            for binding in runtime.bindings().unwrap() {
                native.bind(binding).await.unwrap();
            }
            early(&runtime).await;
            assert_eq!(
                finished.load(Ordering::SeqCst),
                1,
                "actual nonempty grouped validator executed"
            );
            runtime.freeze_for_admission().await.unwrap();
            let scope = runtime.execute_semantics(Profile::Catalog).await.unwrap();
            assert_eq!(
                finished.load(Ordering::SeqCst),
                1,
                "unchanged check must not execute a second time"
            );
            assert_eq!(rejected.load(Ordering::SeqCst), 0);
            let check = scope
                .invariants
                .iter()
                .find(|check| check.name == "input_manifest_membership")
                .unwrap();
            assert_eq!(check.execution, Execution::Scoped);
            let added = scope
                .invariants
                .iter()
                .find(|check| check.name == "input_acquisition_boundaries")
                .unwrap();
            assert_eq!(
                added.execution,
                Execution::EmptyDomain,
                "new applicability earns actual complete-empty root EOF"
            );
            assert_eq!(memo.scoped.executions.load(Ordering::SeqCst), 2);
            assert_eq!(memo.scoped.reuses.load(Ordering::SeqCst), 1);
            assert_eq!(
                scope.scoped.len(),
                2,
                "final evidence retains genuinely executed checks"
            );
            let binding_identity =
                d::completed::binding_inventory_identity(&runtime.bindings().unwrap()).unwrap();
            let (evidence, charge, borrowers) = scope
                .into_evidence(model.digest(), Profile::Catalog, binding_identity)
                .unwrap();
            let inventory = runtime.completed_inventory().unwrap();
            let changed_model = self::model(&finished, &rejected, true);
            assert_eq!(
                model.digest(),
                changed_model.digest(),
                "factory eligibility is separate from semantic model identity"
            );
            let changed = super::controls::runtime(
                changed_model,
                &native,
                &budget,
                &memo,
                &cancellation,
                &calls,
                inventory.contributions,
                inventory.bindings,
                &mut runtimes,
            );
            changed.execute_semantics(Profile::Catalog).await.unwrap();
            assert_eq!(
                finished.load(Ordering::SeqCst),
                2,
                "changed executable factory earns a fresh real grouped check"
            );
            let inventory = runtime.completed_inventory().unwrap();
            let foreign_pool = ResourceBudget::fixed(budget.limit()).unwrap();
            let wrong_pool = super::controls::runtime(
                model.clone(),
                &native,
                &foreign_pool,
                &memo,
                &cancellation,
                &calls,
                inventory.contributions,
                inventory.bindings,
                &mut runtimes,
            );
            // Frozen physical inventory retains the actual pool that admitted its
            // backing. Copying its logical descriptors into another runtime cannot
            // earn a foreign-pool native reader, even when both limits are equal.
            // Refuse at that ownership boundary before attempting native execution;
            // a receiving operation would need independently earned physical owners.
            let physical = native.complete_physical_content(&budget).await.unwrap();
            assert!(matches!(
                physical.inventory().require_pool(&foreign_pool),
                Err(ModelError::Conflict(
                    "completed inventory foreign resource pool"
                ))
            ));
            assert!(
                !Operation::of(&runtime).matches(&wrong_pool),
                "equal-sized foreign pool cannot consume scoped semantic authority"
            );
            assert_eq!(
                finished.load(Ordering::SeqCst),
                2,
                "foreign-pool refusal must not execute a grouped validator"
            );
            memo.close_scoped().unwrap();
            assert!(
                borrowers.iter().all(|check| check._charge.reserved() > 0),
                "actual final borrowers retain charged native evidence after memo release"
            );
            assert!(charge.reserved() > 0 && !evidence.is_empty());
            drop((evidence, charge, borrowers));
            // A fresh admission memo cannot borrow the prior execution evidence
            // even over the same protected rows. Physical cold restore remains a
            // separate control with independent receiving/native ownership.
            let inventory = runtime.completed_inventory().unwrap();
            let cold = super::controls::runtime(
                model.clone(),
                &native,
                &budget,
                &AdmissionMemo::new(&budget),
                &cancellation,
                &calls,
                inventory.contributions,
                inventory.bindings,
                &mut runtimes,
            );
            cold.execute_semantics(Profile::Catalog).await.unwrap();
            assert_eq!(
                finished.load(Ordering::SeqCst),
                3,
                "fresh admission independently executes"
            );
            cold.memo.close_scoped().unwrap();
        })
        .catch_unwind()
        .await;
        let mut terminal = drain_controls(&native, &calls, &runtimes).await;
        finish_control_signals(&runtimes, &mut terminal).await;
        calls.append_operation_failures(&mut terminal);
        native.append_operation_failures(&mut terminal);
        let cleanup = d::completion::complete(Ok(()), terminal);
        if let Err(panic) = result {
            if let Err(error) = cleanup {
                eprintln!("scoped native cleanup: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn scoped_native_changed_input_refuses_and_failed_cancelled_closed_owners_cannot_reuse() {
        let config = lctx_native::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("explicitly owned native control campaign"),
        ))
        .unwrap();
        let native = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let cancellation = Cancellation::default();
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &budget,
        ));
        let memo = AdmissionMemo::new(&budget);
        let finished = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicUsize::new(0));
        let mut runtimes = Vec::new();
        let result = std::panic::AssertUnwindSafe(async {
            let model = model(&finished, &rejected, false);
            let (owner, bindings) = seed(&native, &model, &budget, false, true).await;
            let runtime = runtime(
                model.clone(),
                &native,
                &budget,
                &memo,
                &cancellation,
                &calls,
                vec![owner.clone()],
                bindings.clone(),
                &mut runtimes,
            );
            early(&runtime).await;
            assert_eq!(finished.load(Ordering::SeqCst), 1);
            let retained = memo.scoped.entries.lock().unwrap().0[0].clone();
            let (changed_owner, changed_bindings) =
                seed(&native, &model, &budget, true, false).await;
            let mut changed = bindings
                .into_iter()
                .filter(|binding| binding.view.relation != InputRevision::NAME)
                .collect::<Vec<_>>();
            changed.extend(changed_bindings);
            let changed_runtime = super::controls::runtime(
                model,
                &native,
                &budget,
                &memo,
                &cancellation,
                &calls,
                vec![owner, changed_owner],
                changed,
                &mut runtimes,
            );
            let error = changed_runtime
                .execute_semantics(Profile::Catalog)
                .await
                .err()
                .expect("independently invalid manifest must refuse");
            assert!(
                error
                    .to_string()
                    .contains("scoped control expected empty input manifest"),
                "actual grouped validator refusal: {error}"
            );
            assert_eq!(
                rejected.load(Ordering::SeqCst),
                1,
                "changed exact source executes and rejects the independent invalid value"
            );
            assert!(memo.scoped.invalidated.load(Ordering::Acquire));
            assert!(
                memo.scoped.entries.lock().unwrap().0.is_empty(),
                "failed prefix publishes no reusable success"
            );
            assert!(
                retained._charge.reserved() > 0,
                "an actual outstanding borrower retains backing after invalidation"
            );
            cancellation.cancel();
            assert!(
                runtime.execute_semantics(Profile::Catalog).await.is_err(),
                "cancelled operation cannot reuse previous valid check"
            );
            drop(retained);
        })
        .catch_unwind()
        .await;
        let mut terminal = drain_controls(&native, &calls, &runtimes).await;
        let closed = std::panic::AssertUnwindSafe(async {
            if let Some(runtime) = runtimes.first() {
                assert!(
                    runtime.execute_semantics(Profile::Catalog).await.is_err(),
                    "closed operation cannot consume prior scoped authority"
                );
            }
        })
        .catch_unwind()
        .await;
        finish_control_signals(&runtimes, &mut terminal).await;
        calls.append_operation_failures(&mut terminal);
        native.append_operation_failures(&mut terminal);
        let cleanup = d::completion::complete(Ok(()), terminal);
        if let Err(panic) = result {
            if let Err(error) = cleanup {
                eprintln!("scoped changed-input cleanup: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        if let Err(panic) = closed {
            if let Err(error) = cleanup {
                eprintln!("scoped closed-owner cleanup: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleanup.unwrap();
    }
}
