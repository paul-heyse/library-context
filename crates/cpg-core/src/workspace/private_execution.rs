//! Compiler-private execution. Logical descriptors never confer installed native authority.
use super::*;
use lctx_compiler_store::private_validation::{CheckedPrivateScope, PrivateBinding};
use lctx_model::domain::completed::{CompletedContribution, CompletedView, OutputContent};

impl Workspace {
    pub(super) fn private_snapshot_bytes<'a>(
        owners: &CompletedOwners,
        sources: impl Iterator<Item = &'a Arc<CompletedRelation>>,
    ) -> usize {
        owners
            .charge
            .accounted_bytes()
            .saturating_mul(3)
            .saturating_add(
                sources
                    .map(|source| {
                        size_of::<PrivateBinding>()
                            .saturating_add(source.snapshot.heap_bytes())
                            .saturating_add(lctx_native::installed_view::view_heap(&source.view))
                            .saturating_add(1024)
                    })
                    .sum::<usize>()
                    .saturating_mul(3),
            )
    }
    pub(crate) fn private_directory(&self) -> Arc<tempfile::TempDir> {
        self.files._directory.clone()
    }
    pub(crate) async fn materialize(
        self: &Arc<Self>,
        native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
        profile: Profile,
    ) -> Result<Arc<lctx_compiler_store::admission::AdmissionRuntime>, ModelError> {
        if !self.is_private() {
            return Err(ModelError::Conflict(
                "materialization requires a private operation",
            ));
        }
        self.cancellation.check()?;
        if !self.content_frozen.load(Ordering::Acquire) {
            return Err(ModelError::Conflict(
                "materialization requires sealed compilation",
            ));
        }
        if let Some(budget) = native.recovery_budget()?
            && !budget.shares_pool(&self.budget)
        {
            return Err(ModelError::Conflict("materialization native budget"));
        }
        let (producers, bindings, dependency_views, _transfer) = {
            let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
            let exact = self.completed_by_producer.lock().map_err(|_| poisoned())?;
            let current = self.completed.lock().map_err(|_| poisoned())?;
            let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
            let transfer = self.budget.reserve(
                "private materialization descriptor transfer",
                Self::private_snapshot_bytes(
                    &owners,
                    exact
                        .values()
                        .chain(current.values())
                        .chain(frozen.values()),
                ),
            )?;
            let producers = owners
                .by_spec
                .values()
                .map(|descriptor| {
                    let outputs = descriptor
                        .outputs
                        .keys()
                        .map(|name| {
                            let source = exact
                                .iter()
                                .find(|((producer, relation), _)| {
                                    *producer == descriptor.spec.producer
                                        && *relation == name.as_str()
                                })
                                .map(|(_, source)| source)
                                .ok_or(ModelError::Conflict(
                                    "materialization exact producer source",
                                ))?;
                            Ok((name.clone(), source.private_source()?.clone()))
                        })
                        .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
                    Ok(super::native_materialization::PrivateProducer {
                        descriptor: descriptor.clone(),
                        outputs,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let mut bindings = current
                .values()
                .map(|source| lctx_model::domain::completed::CompletedBinding {
                    boundary: None,
                    source: source.snapshot(),
                    view: source.view.clone(),
                    configuration: None,
                })
                .collect::<Vec<_>>();
            bindings.extend(frozen.iter().map(|((boundary, _), source)| {
                lctx_model::domain::completed::CompletedBinding {
                    boundary: Some(boundary.name().into()),
                    source: source.snapshot(),
                    view: source.view.clone(),
                    configuration: None,
                }
            }));
            let dependency_views = exact
                .values()
                .chain(current.values())
                .chain(frozen.values())
                .map(|source| source.view.clone())
                .collect();
            (producers, bindings, dependency_views, transfer)
        };
        let contributions = producers
            .iter()
            .map(|producer| producer.descriptor.clone())
            .collect();
        super::native_materialization::materialize(
            native.clone(),
            producers,
            bindings.clone(),
            dependency_views,
            &self.model,
            &self.budget,
            &self.cancellation,
        )
        .await?;
        let calls = Arc::new(crate::native_calls::NativeCalls::new(
            native.clone(),
            self.cancellation.clone(),
            &self.budget,
        ));
        let execution = Arc::new(MaterializedExecution {
            private: self.execution.clone(),
            native: native.clone(),
            calls: calls.clone(),
            budget: self.budget.clone(),
        });
        let runtime = lctx_compiler_store::admission::AdmissionRuntime::borrow_runtime(
            self.model.clone(),
            self.options,
            self.budget.clone(),
            native,
            self.cancellation.clone(),
            calls,
            self.admission_memo.clone(),
            self.context.copied_config(),
            self.context.runtime_env(),
            execution,
        )?;
        runtime.install_completed(profile, contributions, bindings)?;
        runtime.attach_private_validation(self.private_validation.clone())?;
        Ok(runtime)
    }
    pub(super) fn private_binding(
        source: &CompletedRelation,
        role: usize,
        requested: Option<lctx_model::domain::stages::PublicationBoundary>,
        resolved: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<PrivateBinding, ModelError> {
        PrivateBinding::completed(
            role,
            requested,
            resolved,
            source.snapshot(),
            source.view.contributions.clone(),
            source.private_source()?.clone(),
        )
    }
    pub(super) async fn private_checked(
        &self,
        inputs: &CompletedInputs,
    ) -> Result<Arc<CheckedPrivateScope>, ModelError> {
        if !Arc::ptr_eq(&self.files, &inputs.attempt)
            || inputs.model != self.model.digest()
            || !self.budget.shares_pool(&inputs.budget)
        {
            return Err(ModelError::Conflict("private selected source owner"));
        }
        let (selected, universe, owners, _transfer) = {
            let _gate = self.completion_gate.lock().await;
            self.cancellation.check()?;
            let owners = self.completed_owners.lock().map_err(|_| poisoned())?;
            let exact = self.completed_by_producer.lock().map_err(|_| poisoned())?;
            let current = self.completed.lock().map_err(|_| poisoned())?;
            let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
            let transfer = self.budget.reserve(
                "private semantic descriptor transfer",
                Self::private_snapshot_bytes(
                    &owners,
                    exact
                        .values()
                        .chain(current.values())
                        .chain(frozen.values())
                        .chain(inputs.relations.values().map(|binding| &binding.source)),
                ),
            )?;
            let selected = inputs
                .ordered_bindings()
                .into_iter()
                .map(|(_, binding)| {
                    Self::private_binding(
                        &binding.source,
                        binding.role,
                        binding.requested,
                        binding.resolved,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut universe = Vec::new();
            let mut seen = BTreeSet::new();
            for source in current.values().chain(exact.values()) {
                if seen.insert((source.snapshot().identity(), None)) {
                    universe.push(Self::private_binding(source, universe.len(), None, None)?);
                }
            }
            for ((boundary, _), source) in frozen.iter() {
                if seen.insert((source.snapshot().identity(), Some(*boundary))) {
                    universe.push(Self::private_binding(
                        source,
                        universe.len(),
                        Some(*boundary),
                        Some(*boundary),
                    )?);
                }
            }
            (
                selected,
                universe,
                owners.by_spec.values().cloned().collect(),
                transfer,
            )
        };
        self.private_validation
            .validate(inputs.profile, selected, universe, owners)
            .await
    }
    pub(super) fn private_relation(
        &self,
        template: &CompletedRelation,
        source: Arc<private_source::PrivateRelation>,
        contributors: BTreeSet<ContentHash>,
    ) -> Result<Arc<CompletedRelation>, ModelError> {
        let view = CompletedView::new(template.name().into(), contributors, source.rows())?;
        let charge = self.budget.reserve(
            "private completed descriptor",
            size_of::<CompletedRelation>() + 1024 + view.contributions.len() * 96,
        )?;
        let mut completed = template.clone();
        completed.rows = source.rows();
        completed.snapshot = analysis::sources::SourceSnapshot::of_completed_view(
            &template.relation,
            self.model.digest(),
            &view,
        )?;
        completed.view = view;
        completed.private = Some(source);
        completed.native = None;
        completed.bridge = None;
        completed._selection_charge = Some(Arc::from(charge));
        Ok(Arc::new(completed))
    }
    pub(super) fn compose_private(
        &self,
        sources: &[Arc<CompletedRelation>],
    ) -> Result<Arc<CompletedRelation>, ModelError> {
        let template = sources
            .first()
            .ok_or(ModelError::Conflict("empty private contributor set"))?;
        let mut source = template.private_source()?.clone();
        let mut contributors = template.view.contributions.clone();
        for next in &sources[1..] {
            if next.profile != template.profile
                || next.relation.type_id() != template.relation.type_id()
            {
                return Err(ModelError::Conflict("private contributor relation/profile"));
            }
            source = private_source::PrivateRelation::merge(&source, next.private_source()?)?;
            contributors.extend(next.view.contributions.iter().copied());
        }
        self.private_relation(template, source, contributors)
    }
    pub(super) async fn bind_private_selection(
        &self,
        selection: &lctx_model::domain::stages::InputSelection,
    ) -> Result<(), ModelError> {
        self.bind_private_selections(std::slice::from_ref(selection))
            .await
    }
    pub(super) async fn bind_private_selections(
        &self,
        selections: &[lctx_model::domain::stages::InputSelection],
    ) -> Result<(), ModelError> {
        let _preparation = self.budget.reserve(
            "private frozen binding set",
            selections.len().saturating_mul(512),
        )?;
        let mut prepared = Vec::with_capacity(selections.len());
        for selection in selections {
            let Some(boundary) = selection.resolved else {
                continue;
            };
            let sources = {
                let by_producer = self.completed_by_producer.lock().map_err(|_| poisoned())?;
                selection
                    .producers
                    .iter()
                    .map(|producer| {
                        by_producer
                            .get(&(*producer, selection.relation))
                            .cloned()
                            .ok_or(ModelError::Conflict("private exact contributor absent"))
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            prepared.push((
                (boundary, selection.relation),
                self.compose_private(&sources)?,
            ));
        }
        let _gate = self.completion_gate.lock().await;
        self.cancellation.check()?;
        self.writable()?;
        let mut frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        for (key, source) in &prepared {
            if frozen
                .get(key)
                .is_some_and(|existing| existing.view != source.view)
            {
                return Err(ModelError::Conflict(
                    "private exact boundary changed contributors",
                ));
            }
        }
        for (key, source) in prepared {
            frozen.entry(key).or_insert(source);
        }
        Ok(())
    }
}

struct MaterializedExecution {
    private: Arc<WorkspaceExecutionOwner>,
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    calls: Arc<crate::native_calls::NativeCalls>,
    budget: ResourceBudget,
}
impl lctx_compiler_store::admission::ExecutionOwner for MaterializedExecution {
    fn drain_owned_report(&self) -> BoxFuture<'_, lctx_model::domain::completion::Completion> {
        async move {
            let mut completion = self.private.drain_owned_report().await;
            completion.merge(self.calls.drain_finality_report().await);
            // Keep the real native drain independently charged and erased rather
            // than embedding its full recovery continuation in every outer owner.
            match lctx_native::continuation::admit(
                &self.budget,
                "materialized native finalization",
                self.native.drain_finality_report(),
            ) {
                Ok(native) => completion.merge(native.await),
                Err(error) => {
                    completion.local = lctx_model::domain::completion::LocalState::Outstanding;
                    completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                    completion.step("materialized native finalization admission", Err(error));
                }
            }
            completion
        }
        .boxed()
    }
    fn complete_signal<'a>(
        &'a self,
        completion: &'a lctx_model::domain::completion::Completion,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        self.private.complete_signal(completion)
    }
    fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        self.private.append_operation_failures(completion);
        self.calls.append_operation_failures(completion);
        self.native.append_operation_failures(completion);
    }
}

impl ProducerOutput {
    pub(crate) fn install_private_product(
        &self,
        outputs: BTreeMap<&'static str, Arc<private_source::PrivateRelation>>,
    ) -> Result<(), ModelError> {
        self.check()?;
        if !self.workspace.is_private()
            || outputs.keys().copied().collect::<BTreeSet<_>>() != self.allowed
        {
            return Err(ModelError::Conflict("private retained output inventory"));
        }
        if outputs
            .values()
            .any(|source| source.model() != self.workspace.model.digest())
        {
            return Err(ModelError::Conflict("private retained output model"));
        }
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        if !writers.is_empty() {
            return Err(ModelError::Conflict(
                "private retained output already started",
            ));
        }
        for (name, source) in outputs {
            writers.insert(
                name,
                Box::new(PrivateReplay {
                    source,
                    contribution: lctx_model::domain::stages::is_epoch_shared(name),
                }),
            );
        }
        Ok(())
    }
    pub(super) fn complete_private(self) -> BoxFuture<'static, Result<(), ModelError>> {
        async move {
            self.workspace.cancellation.check()?; self.workspace.writable()?;
            let outcome = self.outcome.lock().map_err(|_| poisoned())?.ok_or(ModelError::Conflict("private producer outcome absent"))?;
            if self.failed.load(Ordering::Acquire) || outcome == ProviderOutcome::Failed { return Err(ModelError::Conflict("private producer failed")); }
            let spec = self.contribution_spec();
            spec.validate_profile(self.profile)?;
            let product_request = self.product_capture.lock().map_err(|_| poisoned())?.take();
            let writers = self.writers.into_inner().map_err(|_| poisoned())?;
            if self.expected.iter().any(|name| !writers.contains_key(name)) { return Err(ModelError::Conflict("private declared output missing")); }
            let mut pending = BTreeMap::new();
            for writer in writers.into_values() {
                let mut completed = writer.close(self.workspace.model.clone(), self.workspace.budget.clone()).await?;
                if completed.private.is_none() { completed.private = Some(private_source::PrivateRelation::empty(self.workspace.model.digest(), completed.relation.clone(), self.workspace.budget.clone(), self.workspace.files._directory.clone())?); }
                pending.insert(completed.relation.name(), completed);
            }
            for name in &self.allowed {
                if !pending.contains_key(name) {
                    let relation = self.workspace.model.relation(name).ok_or(ModelError::Schema("private output declaration"))?.clone();
                    let source = private_source::PrivateRelation::empty(self.workspace.model.digest(), relation.clone(), self.workspace.budget.clone(), self.workspace.files._directory.clone())?;
                    pending.insert(*name, PendingRelation { relation, contribution: true, private: Some(source) });
                }
            }
            let descriptor = CompletedContribution { spec, assurance: lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface, outcome: outcome as i16, outputs: pending.iter().map(|(name, output)| { let source = output.private.as_ref().expect("completed private output"); ((*name).into(), OutputContent { rows: source.rows(), content: source.content() }) }).collect() };
            let owner_key = descriptor.spec.identity()?;
            let completed_key = descriptor.identity()?;
            let input_snapshots: Arc<[_]> = self.inputs.snapshots().collect::<Vec<_>>().into();
            let mut charge = StateCharge::new(&self.workspace.budget, "private coherent producer handoff");
            charge.grow(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?.len().saturating_mul(4).saturating_add(pending.len() * 1024 + 1024))?;
            let mut singletons = BTreeMap::new();
            for (name, output) in &pending {
                let source = output.private.as_ref().expect("completed private source").clone();
                let view = CompletedView::new((*name).into(), [completed_key].into(), source.rows())?;
                let metadata = self.workspace.budget.reserve("private singleton descriptor", size_of::<CompletedRelation>() + 1024 + input_snapshots.len() * 256)?;
                singletons.insert(*name, Arc::new(CompletedRelation {
                    relation: output.relation.clone(), private: Some(source), producer: self.name.into(), implementation: self.implementation, configuration: self.configuration, contract: self.workspace.model.digest(), rows: view.rows,
                    snapshot: analysis::sources::SourceSnapshot::of_completed_view(&output.relation, self.workspace.model.digest(), &view)?, view,
                    native: None, bridge: None, budget: self.workspace.budget.clone(), batch_rows: self.workspace.options.batch_rows, cancellation: self.workspace.cancellation.clone(), inputs: input_snapshots.clone(), profile: self.profile, contribution: output.contribution,
                    _files: self.workspace.files.clone(), _execution: self.workspace.execution.clone(), _selection_charge: Some(Arc::from(metadata)),
                }));
            }
            let mut predecessors = {
                let _gate = self.workspace.completion_gate.lock().await;
                let visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
                singletons.keys().map(|name| (*name, visible.get(name).cloned())).collect::<BTreeMap<_, _>>()
            };
            let mut prepared = BTreeMap::new();
            loop {
                // Pure index composition is outside visibility exclusion. Only affected stale
                // predecessors are rebased; no completed producer is rerun.
                for (name, previous) in &predecessors {
                    #[cfg(test)] self.workspace.composition_control.attempt(self.name, name);
                    let singleton = singletons.get(name).expect("planned singleton");
                    let composed = match previous { Some(previous) => {
                        if !singleton.contribution && !previous.contribution { return Err(ModelError::Conflict("private relation already owned")); }
                        self.workspace.compose_private(&[previous.clone(), singleton.clone()])?
                    }, None => singleton.clone() };
                    prepared.insert(*name, (previous.clone(), composed));
                }
                #[cfg(test)] self.workspace.composition_control.prepared(self.name).await;
                let gate = self.workspace.completion_gate.lock().await;
                self.workspace.cancellation.check()?; self.workspace.writable()?;
                let mut owners = self.workspace.completed_owners.lock().map_err(|_| poisoned())?;
                let mut exact = self.workspace.completed_by_producer.lock().map_err(|_| poisoned())?;
                let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
                if owners.by_spec.contains_key(&owner_key) || owners.spec_by_completed.contains_key(&completed_key) || singletons.keys().any(|name| exact.contains_key(&(self.name, *name))) { return Err(ModelError::Conflict("private producer already completed")); }
                predecessors.clear();
                for (name, (previous, _)) in &prepared {
                    if visible.get(name).map(|s| s.view.identity) != previous.as_ref().map(|s| s.view.identity) { predecessors.insert(*name, visible.get(name).cloned()); }
                }
                if !predecessors.is_empty() { drop((visible, exact, owners, gate)); continue; }
                owners.charge.prepare_absorb(&charge)?;
                owners.charge.absorb(charge)?;
                owners.spec_by_completed.insert(completed_key, owner_key);
                owners.by_spec.insert(owner_key, descriptor.clone());
                for (name, source) in &singletons { exact.insert((self.name, *name), source.clone()); }
                for (name, (_, source)) in prepared { visible.insert(name, source); }
                drop((visible, exact, owners, gate));
                break;
            }
            self.workspace.refresh_membership_demand()?;
            if let Some(request) = product_request {
                let workspace = self.workspace.clone();
                let outputs = singletons.into_iter().map(|(name, source)| Ok((name.to_owned(), source.private_source()?.clone()))).collect::<Result<BTreeMap<_, _>, ModelError>>()?;
                self.workspace.retain_child(async move { workspace.retain_private_product(request, descriptor, outputs).await })?;
            }
            Ok(())
        }.boxed()
    }
}

struct PrivateReplay {
    source: Arc<private_source::PrivateRelation>,
    contribution: bool,
}
impl ErasedWriter for PrivateReplay {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(
        self: Box<Self>,
        _model: Arc<ValidatedModel>,
        _budget: ResourceBudget,
    ) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
        futures::future::ready(Ok(PendingRelation {
            relation: self.source.relation().clone(),
            private: Some(self.source),
            contribution: self.contribution,
        }))
        .boxed()
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::input::Package;
    #[tokio::test]
    async fn captured_private_plan_and_stream_retain_execution_and_refuse_after_cancellation() {
        use futures::TryStreamExt;
        let model = Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap());
        let workspace = Workspace::private(model, Default::default()).unwrap();
        let output = workspace.output(
            "private-plan",
            Profile::Catalog,
            ContentHash::of(b"private-plan"),
            workspace
                .inputs("private-plan", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        output.declare::<Package>().unwrap();
        output
            .push(Package {
                name: "retained".into(),
            })
            .await
            .unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let source = workspace.completed::<Package>().unwrap();
        let access = workspace
            .inputs("private-plan-consumer", Profile::Catalog, [Package::NAME])
            .unwrap();
        let session = access.session(&workspace).await.unwrap();
        let plan = session
            .sql("SELECT * FROM packages")
            .await
            .unwrap()
            .create_physical_plan()
            .await
            .unwrap();
        let stream = plan.execute(0, session.task_ctx()).unwrap();
        let execution = Arc::downgrade(&workspace.execution);
        let mut finality = workspace.drain_owned_report().await;
        assert!(source.table_provider().is_err());
        assert!(
            session
                .sql("SELECT * FROM packages WHERE false")
                .await
                .is_err()
        );
        assert!(
            stream.try_collect::<Vec<_>>().await.is_err(),
            "a captured stream polls the actual cancellation owner"
        );
        let after_cancel = plan.execute(0, session.task_ctx()).unwrap();
        assert!(
            after_cancel.try_collect::<Vec<_>>().await.is_err(),
            "a captured plan cannot restart reading after drain"
        );
        let signal = workspace.complete_signal(&finality).await;
        finality.step("captured private plan signal", signal);
        lctx_model::domain::completion::complete(Ok(()), finality).unwrap();
        drop((source, access, workspace));
        assert!(
            execution.upgrade().is_some(),
            "captured provider/plan retains actual execution ownership"
        );
        drop((plan, session));
        assert!(execution.upgrade().is_none());
    }
    fn private_workspace() -> Arc<Workspace> {
        Workspace::private(
            Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[tokio::test]
    async fn consumed_preparation_releases_successful_value_before_workspace_drain() {
        struct Value(Arc<AtomicBool>);
        impl Drop for Value {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let workspace = private_workspace();
        let dropped = Arc::new(AtomicBool::new(false));
        let value = Value(dropped.clone());
        let result = workspace
            .spawn_preparation(async move { Ok(value) })
            .unwrap();
        let value = result.await.unwrap();
        assert!(!dropped.load(Ordering::Acquire));
        drop(value);
        assert!(
            dropped.load(Ordering::Acquire),
            "the terminal observer must not retain an already consumed successful value"
        );
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn accepted_preparation_cancelled_before_entry_never_runs_body() {
        let workspace = private_workspace();
        let entered = Arc::new(AtomicBool::new(false));
        let observed = entered.clone();
        let result = workspace
            .spawn_preparation(async move {
                observed.store(true, Ordering::Release);
                Ok(1u32)
            })
            .unwrap();
        workspace.cancellation.cancel();
        assert!(result.await.is_err());
        assert!(!entered.load(Ordering::Acquire));
        let completion = workspace.drain_owned_report().await;
        workspace.complete_signal(&completion).await.unwrap();
    }
    #[tokio::test]
    async fn abandoned_preparation_waiters_preserve_body_error_and_panic() {
        for panic in [false, true] {
            let workspace = private_workspace();
            let result = workspace
                .spawn_preparation(async move {
                    assert!(!panic, "owned preparation panic control");
                    Err::<u32, _>(ModelError::Schema("abandoned preparation failure control"))
                })
                .unwrap();
            drop(result);
            tokio::task::yield_now().await;
            let completion = workspace.drain_owned_report().await;
            assert!(
                !completion.failures.is_empty(),
                "an abandoned delivery cannot erase accepted work outcome"
            );
            workspace.complete_signal(&completion).await.unwrap();
        }
    }
    #[tokio::test]
    async fn sealed_preparation_launch_refuses_before_registration() {
        let workspace = private_workspace();
        drop(workspace.execution.stages.seal().unwrap());
        let entered = Arc::new(AtomicBool::new(false));
        let observed = entered.clone();
        assert!(
            workspace
                .spawn_preparation(async move {
                    observed.store(true, Ordering::Release);
                    Ok(1u32)
                })
                .is_err()
        );
        assert!(workspace.execution.preparations.lock().unwrap().is_empty());
        assert!(!entered.load(Ordering::Acquire));
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn validation_snapshot_and_completion_wait_for_the_same_atomic_boundary() {
        let workspace = private_workspace();
        let initial = workspace.output(
            "snapshot-initial",
            Profile::Catalog,
            ContentHash::of(b"initial"),
            workspace
                .inputs("snapshot-initial", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        initial
            .contribute(
                Batch::new(
                    workspace.model(),
                    vec![Package { name: "one".into() }],
                    workspace.budget(),
                )
                .unwrap(),
            )
            .unwrap();
        initial.finish(ProviderOutcome::Complete).await.unwrap();
        let inputs = workspace
            .inputs("snapshot-consumer", Profile::Catalog, [Package::NAME])
            .unwrap();
        let peer = workspace.output(
            "snapshot-peer",
            Profile::Catalog,
            ContentHash::of(b"peer"),
            workspace
                .inputs("snapshot-peer", Profile::Catalog, [])
                .unwrap(),
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
        let gate = workspace.completion_gate.lock().await;
        let validator = workspace.clone();
        let (validation_started, validation_entered) = tokio::sync::oneshot::channel();
        let validating = tokio::spawn(async move {
            let _ = validation_started.send(());
            validator.checked_inputs(&inputs).await
        });
        let (completion_started, completion_entered) = tokio::sync::oneshot::channel();
        let completing = tokio::spawn(async move {
            let _ = completion_started.send(());
            peer.finish(ProviderOutcome::Complete).await
        });
        validation_entered.await.unwrap();
        completion_entered.await.unwrap();
        assert!(!validating.is_finished());
        assert!(!completing.is_finished());
        assert!(workspace.completed_owners.try_lock().is_ok());
        assert!(workspace.completed_by_producer.try_lock().is_ok());
        assert!(
            workspace.completed.try_lock().is_ok(),
            "a snapshot waiting for visibility never holds a later registry lock"
        );
        drop(gate);
        let (checked, completed) = tokio::join!(validating, completing);
        let checked = checked.unwrap().unwrap();
        completed.unwrap().unwrap();
        assert_eq!(checked.inputs().relation::<Package>().unwrap().rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn producer_private_consumer_and_query_require_no_native_attempt() {
        let model = Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap());
        let options = WorkspaceOptions {
            memory_bytes: 32 << 20,
            ..Default::default()
        };
        let workspace = Workspace::private(model.clone(), options).unwrap();
        assert!(workspace.native().is_err());
        let producer = workspace.output(
            "private-packages",
            Profile::Catalog,
            ContentHash::of(b"private-packages"),
            workspace
                .inputs("private-packages", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        producer.declare::<Package>().unwrap();
        producer.push(Package { name: "one".into() }).await.unwrap();
        producer.finish(ProviderOutcome::Complete).await.unwrap();
        let selected = workspace
            .inputs("private-consumer", Profile::Catalog, [Package::NAME])
            .unwrap();
        let original = selected.relation::<Package>().unwrap().clone();
        let mut rows = original
            .read::<Package>(model.clone(), workspace.budget.clone())
            .unwrap();
        assert_eq!(
            rows.next().unwrap().unwrap().rows(),
            &[Package { name: "one".into() }]
        );
        drop(rows);
        let session = selected.session(&workspace).await.unwrap();
        let data = crate::sql::query(&session, "SELECT name FROM packages")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(data.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
        let more = workspace.output(
            "private-more",
            Profile::Catalog,
            ContentHash::of(b"private-more"),
            workspace
                .inputs("private-more", Profile::Catalog, [])
                .unwrap(),
            [Package::NAME],
        );
        more.contribute(
            Batch::new(
                &model,
                vec![Package { name: "two".into() }],
                workspace.budget(),
            )
            .unwrap(),
        )
        .unwrap();
        more.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(original.rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        assert!(workspace.native().is_err());
        workspace.drain().await.unwrap();
    }
}
