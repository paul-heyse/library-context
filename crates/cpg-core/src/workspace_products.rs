//! Admitted operation products attach memberships; selected kernel products retain typed values.
use super::*;
use lctx_model::domain::{Key, KeySink, compilation_product::*};
use lctx_native::surrealdb::types::Value;

pub(crate) struct ProductCandidate {
    sections: Vec<(Relation, Vec<RecordBatch>)>,
    _charge: Box<dyn Reservation>,
}
impl ProductCandidate {
    pub(crate) fn sections(&self) -> impl Iterator<Item = (&Relation, &[RecordBatch])> {
        self.sections
            .iter()
            .map(|(relation, batches)| (relation, batches.as_slice()))
    }
    pub(crate) fn batches(&self, name: &str) -> Option<&[RecordBatch]> {
        self.sections
            .iter()
            .find(|(relation, _)| relation.name() == name)
            .map(|(_, batches)| batches.as_slice())
    }
}

impl Workspace {
    pub(crate) fn product_reuse_enabled(&self) -> bool {
        self.admitted_product_reuse.load(Ordering::Acquire)
    }
    /// Select an independent recomputation control before producing any completed output.
    /// It retains current admission and may still retain its completed content for later use.
    pub fn set_admitted_product_reuse(&self, enabled: bool) -> Result<(), ModelError> {
        self.writable()?;
        if !self.completed.lock().map_err(|_| poisoned())?.is_empty() {
            return Err(ModelError::Conflict("reuse policy after completed output"));
        }
        self.admitted_product_reuse
            .store(enabled, Ordering::Release);
        Ok(())
    }
    /// Installation is an explicit store/fixture operation; ordinary attempts only connect.
    pub fn set_product_cache(
        &self,
        cache: Option<Arc<lctx_compiler_store::NativeProductCache>>,
    ) -> Result<(), ModelError> {
        self.writable()?;
        *self.product_cache.lock().map_err(|_| poisoned())? = cache;
        Ok(())
    }
    pub(crate) fn product_cache(
        &self,
    ) -> Result<Option<Arc<lctx_compiler_store::NativeProductCache>>, ModelError> {
        Ok(self.product_cache.lock().map_err(|_| poisoned())?.clone())
    }
}
struct ReplayWriter {
    relation: Relation,
    contribution: bool,
}
impl ErasedWriter for ReplayWriter {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(
        self: Box<Self>,
        _model: Arc<ValidatedModel>,
        _budget: ResourceBudget,
    ) -> BoxFuture<'static, Result<PendingRelation, ModelError>> {
        async move {
            Ok(PendingRelation {
                relation: self.relation,
                contribution: self.contribution,
                private: None,
            })
        }
        .boxed()
    }
}
impl ProducerOutput {
    pub(crate) fn product_request(&self) -> Result<ProductRequest, ModelError> {
        Ok(self.captured_request()?.product().clone())
    }
    pub(crate) fn captured_request(
        &self,
    ) -> Result<&lctx_model::domain::producer_contract::request::CapturedRequest, ModelError> {
        self.captured_request
            .get_or_init(|| {
                let bytes = self
                    .captured_binding
                    .as_ref()
                    .map_or(0, |binding| binding.heap_bytes())
                    .saturating_add(
                        self.allowed
                            .iter()
                            .map(|name| name.len() + 128)
                            .sum::<usize>(),
                    )
                    .saturating_add(self.inputs.ordered_bindings().len().saturating_mul(4096))
                    .saturating_mul(8)
                    .saturating_add(65536);
                let charge = self
                    .workspace
                    .budget()
                    .reserve("captured-provider-request", bytes)
                    .map_err(Arc::new)?;
                self.build_captured_request()
                    .map(|request| (request, charge))
                    .map_err(Arc::new)
            })
            .as_ref()
            .map(|(request, _)| request)
            .map_err(|error| ModelError::SharedCause(error.clone()))
    }
    fn build_captured_request(
        &self,
    ) -> Result<lctx_model::domain::producer_contract::request::CapturedRequest, ModelError> {
        let mut contract = KeySink::new("compiler-product-row-contract/v1");
        for name in &self.allowed {
            let relation = self
                .workspace
                .model
                .relation(name)
                .ok_or(ModelError::Schema("product output relation"))?;
            relation.name().to_owned().encode(&mut contract);
            // The model contract includes typed schemas, invariants and their implementation.
            self.workspace.model.digest().encode(&mut contract);
        }
        let dependencies = self
            .inputs
            .ordered_bindings()
            .into_iter()
            .map(|((name, _), source)| DependencyToken {
                kind: DependencyKind::ExactView,
                role: format!("input/{}", source.role),
                relation: name.into(),
                prefix: source.resolved.map(|p| p.name().into()),
                identity: source.snapshot.identity(),
            })
            .collect();
        lctx_model::domain::producer_contract::request::CapturedRequest::new(
            self.contribution_spec(),
            contract.finish(),
            dependencies,
        )
    }
    /// Miss arms terminal retention; admitted native hits hold exact contributors by reference.
    pub(crate) fn reuse_product(
        &self,
    ) -> BoxFuture<'_, Result<Option<ProviderOutcome>, ModelError>> {
        self.reuse_product_inner(false, |_| async { Ok(()) }.boxed())
    }
    pub(crate) fn reuse_product_checked_async<'a, F>(
        &'a self,
        check: F,
    ) -> BoxFuture<'a, Result<Option<ProviderOutcome>, ModelError>>
    where
        F: FnOnce(Arc<ProductCandidate>) -> BoxFuture<'a, Result<(), ModelError>> + Send + 'a,
    {
        self.reuse_product_inner(true, check)
    }
    async fn reuse_private_product<'a, F>(
        &'a self,
        prepare_owner: bool,
        check: F,
    ) -> Result<Option<ProviderOutcome>, ModelError>
    where
        F: FnOnce(Arc<ProductCandidate>) -> BoxFuture<'a, Result<(), ModelError>> + Send + 'a,
    {
        self.check()?;
        let Some(cache) = self.workspace.product_cache()? else {
            // No configured retained-product consumer: decline before a native request.
            return Ok(None);
        };
        let request = self.product_request()?;
        if !self.writers.lock().map_err(|_| poisoned())?.is_empty() {
            return Err(ModelError::Conflict(
                "private product lookup after output started",
            ));
        }
        let cached = if self.workspace.product_reuse_enabled() {
            let request = self.captured_request()?;
            let request_charge = self.workspace.budget().reserve(
                "private compiled cache request transfer",
                request
                    .product()
                    .retained_bytes()?
                    .saturating_mul(2)
                    .saturating_add(65536),
            )?;
            let request = request.clone();
            let workspace = self.workspace.clone();
            let allowed = self.allowed.clone();
            let profile = self.profile;
            self.workspace
                .cache_call(async move {
                    let _request_charge = request_charge;
                    let Some(retained) = cache
                        .lookup_compiled(
                            &request,
                            workspace.model.clone(),
                            profile,
                            workspace.budget(),
                        )
                        .await?
                    else {
                        return Ok(None);
                    };
                    // The accepted operation owns the native source through checked copying
                    // and actual close. Only settled private backing crosses result delivery,
                    // including when its waiter disappears after a successful lookup.
                    let copied = async {
                        let descriptor = retained.descriptor();
                        if descriptor
                            .outputs
                            .keys()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>()
                            != allowed
                        {
                            return Err(ModelError::Conflict("private retained output inventory"));
                        }
                        let mut outputs = BTreeMap::new();
                        for name in &allowed {
                            let relation = workspace
                                .model
                                .relation(name)
                                .ok_or(ModelError::Schema("private cached relation"))?
                                .clone();
                            let mut writer = super::private_source::PrivateWriter::new_with_spill(
                                workspace.model.digest(),
                                relation.clone(),
                                workspace.budget().clone(),
                                workspace.private_directory(),
                                workspace.private_spill.clone(),
                            )?;
                            let mut stream = retained
                                .batches(
                                    &relation,
                                    workspace.budget(),
                                    workspace.options.batch_rows,
                                )
                                .await?;
                            while let Some(batch) =
                                stream.try_next().await.map_err(ModelError::codec)?
                            {
                                workspace.cancellation.check()?;
                                writer.push(relation.checked_batch(
                                    &workspace.model,
                                    batch,
                                    workspace.budget(),
                                )?)?;
                            }
                            let source = writer.finish()?;
                            let expected = descriptor
                                .outputs
                                .get(*name)
                                .ok_or(ModelError::Conflict("private cached output absent"))?;
                            if source.rows() != expected.rows
                                || source.content() != expected.content
                            {
                                return Err(ModelError::Conflict("private cached typed content"));
                            }
                            outputs.insert(*name, source);
                        }
                        Ok(outputs)
                    }
                    .await;
                    let mut terminal = lctx_model::domain::completion::Completion::default();
                    terminal.step(
                        "private retained product source close",
                        retained.close().await,
                    );
                    lctx_model::domain::completion::complete(copied, terminal).map(Some)
                })
                .await?
        } else {
            None
        };
        let Some(outputs) = cached else {
            *self.product_capture.lock().map_err(|_| poisoned())? = Some(request);
            return Ok(None);
        };
        if prepare_owner {
            let mut charge = self
                .workspace
                .budget
                .reserve("private product application metadata", 0)?;
            let mut sections = Vec::new();
            for source in outputs.values() {
                let mut batches = Vec::new();
                for batch in source.read_batches()? {
                    charge
                        .try_resize(charge.size().saturating_add(size_of::<RecordBatch>() + 256))?;
                    batches.push(batch?);
                }
                sections.push((source.relation().clone(), batches));
            }
            check(Arc::new(ProductCandidate {
                sections,
                _charge: charge,
            }))
            .await?;
        }
        self.workspace.cancellation.check()?;
        self.install_private_product(outputs)?;
        Ok(Some(ProviderOutcome::Complete))
    }
    fn reuse_product_inner<'a, F>(
        &'a self,
        prepare_owner: bool,
        check: F,
    ) -> BoxFuture<'a, Result<Option<ProviderOutcome>, ModelError>>
    where
        F: FnOnce(Arc<ProductCandidate>) -> BoxFuture<'a, Result<(), ModelError>> + Send + 'a,
    {
        async move {
            if self.workspace.is_private() {
                return self.reuse_private_product(prepare_owner, check).await;
            }
            self.check()?;
            let request = self.product_request()?;
            let inventory = self
                .allowed
                .iter()
                .map(|name| {
                    self.workspace
                        .model
                        .relation(name)
                        .cloned()
                        .ok_or(ModelError::Schema("retained output relation"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if !self.writers.lock().map_err(|_| poisoned())?.is_empty()
                || self.contribution.lock().map_err(|_| poisoned())?.is_some()
            {
                return Err(ModelError::Conflict("product lookup after output started"));
            }
            let native = self.workspace.native()?.clone();
            let budget = self.workspace.budget().clone();
            let captured = self.captured_request()?.clone();
            let model = self.workspace.model.clone();
            let profile = self.profile();
            let retained = if self
                .workspace
                .admitted_product_reuse
                .load(Ordering::Acquire)
            {
                self.workspace
                    .native_call(async move {
                        native
                            .hold_captured_product(&captured, model, profile, &budget)
                            .await
                    })
                    .await?
            } else {
                None
            };
            if let Some(candidate) = retained {
                // Persisted admission does not carry ephemeral Rust verification owners.
                // Reconstruct those only for consumers that require them, without ingress.
                if prepare_owner {
                    let preflight = async {
                        let mut charge = self
                            .workspace
                            .budget
                            .reserve("admitted-product-application", 0)?;
                        let mut sections = Vec::with_capacity(inventory.len());
                        for relation in &inventory {
                            let mut stream = self
                                .workspace
                                .native()?
                                .scan_held_candidate_batches(
                                    &candidate,
                                    relation,
                                    self.workspace.budget(),
                                    self.workspace.options.batch_rows,
                                )
                                .await?;
                            let mut batches = Vec::new();
                            while let Some(batch) =
                                stream.try_next().await.map_err(crate::sql::model_error)?
                            {
                                self.workspace.cancellation.check()?;
                                charge.try_resize(
                                    charge
                                        .size()
                                        .saturating_add(lctx_model::domain::logical_batch_bytes(
                                            &batch,
                                        )?)
                                        .saturating_add(4096),
                                )?;
                                batches.push(batch);
                            }
                            sections.push((relation.clone(), batches));
                        }
                        check(Arc::new(ProductCandidate {
                            sections,
                            _charge: charge,
                        }))
                        .await
                    }
                    .await;
                    if let Err(error) = preflight {
                        if !error.permits_storage_cleanup() {
                            return Err(error);
                        }
                        let mut terminal = lctx_model::domain::completion::Completion::default();
                        terminal.step(
                            "rejected retained product candidate close",
                            candidate.close().await,
                        );
                        return lctx_model::domain::completion::complete(Err(error), terminal);
                    }
                }
                // The candidate remains physically held through complete receiver
                // preflight; only now install receiver views and output ownership.
                let native = self.workspace.native()?.clone();
                let budget = self.workspace.budget().clone();
                let (id, _, descriptor) = self
                    .workspace
                    .native_call(
                        async move { native.attach_held_product(candidate, &budget).await },
                    )
                    .await?;
                let mut writers = self.writers.lock().map_err(|_| poisoned())?;
                if !writers.is_empty()
                    || self.contribution.lock().map_err(|_| poisoned())?.is_some()
                {
                    return Err(ModelError::Conflict(
                        "retained product output already started",
                    ));
                }
                for relation in inventory {
                    writers.insert(
                        relation.name(),
                        Box::new(ReplayWriter {
                            contribution: lctx_model::domain::stages::is_epoch_shared(
                                relation.name(),
                            ),
                            relation,
                        }),
                    );
                }
                *self.contribution.lock().map_err(|_| poisoned())? = Some(id);
                let outcome = match descriptor.outcome {
                    code if code == ProviderOutcome::Complete as i16 => ProviderOutcome::Complete,
                    code if code == ProviderOutcome::NotRequested as i16 => {
                        ProviderOutcome::NotRequested
                    }
                    _ => return Err(ModelError::Conflict("retained product outcome")),
                };
                tracing::debug!(operation = self.name, "admitted compiled product attached");
                return Ok(Some(outcome));
            }
            *self.product_capture.lock().map_err(|_| poisoned())? = Some(request.clone());
            Ok(None)
        }
        .boxed()
    }
}
/// One canonical typed-body/global-order check shared by whole-stage and selected products.
#[cfg(test)]
pub(crate) fn validate_product_rows(
    product: &PortableProduct,
    model: &ValidatedModel,
    budget: &ResourceBudget,
    batch_rows: usize,
) -> Result<(), ModelError> {
    decode_product_rows(product, model, budget, batch_rows).map(drop)
}
/// Decode one charged typed candidate. Canonical checks, semantic predicates and ingress all
/// use these batches; no encoded clone or second JSON/Arrow decoder exists on the hit path.
pub(crate) fn decode_product_rows(
    product: &PortableProduct,
    model: &ValidatedModel,
    budget: &ResourceBudget,
    batch_rows: usize,
) -> Result<ProductCandidate, ModelError> {
    product.validate()?;
    if batch_rows == 0 {
        return Err(ModelError::Schema("cached product batch rows"));
    }
    let mut charge = budget.reserve("cached-typed-product", 0)?;
    let mut sections = Vec::new();
    for section in &product.sections {
        let relation = model
            .relation(&section.name)
            .ok_or(ModelError::Schema("cached product relation"))?;
        let _scratch = budget.reserve(
            "cached-canonical-body-validation",
            section.bytes.len().saturating_mul(16).saturating_add(1024),
        )?;
        let rows: Vec<Value> = serde_json::from_slice(&section.bytes).map_err(ModelError::codec)?;
        if rows.len() as u64 != section.rows {
            return Err(ModelError::Conflict("cached product row count"));
        }
        let mut content = relation.content();
        let mut batches = Vec::new();
        for window in rows.chunks(batch_rows) {
            let batch = lctx_native::codec::decode_bodies(relation, window.to_vec(), budget)?;
            relation.hash_rows(&batch, &mut content)?;
            let bodies = canonical_bodies(relation, &batch)?;
            if bodies != window {
                return Err(ModelError::Conflict("cached product canonical typed body"));
            }
            charge.try_resize(
                charge
                    .size()
                    .saturating_add(lctx_model::domain::logical_batch_bytes(&batch)?)
                    .saturating_add(4096),
            )?;
            batches.push(batch);
        }
        charge.try_resize(charge.size().saturating_add(1024))?;
        sections.push((relation.clone(), batches));
    }
    Ok(ProductCandidate {
        sections,
        _charge: charge,
    })
}

impl Workspace {
    /// Queue a named reusable product without creating private-stage native effects.
    /// Its actual persistence uses the final target's completed content before native freeze.
    pub(super) async fn retain_private_product(
        &self,
        request: ProductRequest,
        descriptor: lctx_model::domain::completed::CompletedContribution,
        outputs: BTreeMap<String, Arc<super::private_source::PrivateRelation>>,
    ) -> Result<(), ModelError> {
        if self.product_cache()?.is_none()
            || descriptor.outcome != 0
            || descriptor.spec.producer == lctx_model::domain::producer_contract::facts::ASSEMBLE
        {
            return Ok(());
        }
        if outputs.keys().ne(descriptor.outputs.keys()) {
            return Err(ModelError::Conflict("private retained output inventory"));
        }
        for (name, source) in &outputs {
            let expected = &descriptor.outputs[name];
            if source.model() != self.model.digest()
                || source.rows() != expected.rows
                || source.content() != expected.content
            {
                return Err(ModelError::Conflict("private retained output content"));
            }
        }
        let mut charge = StateCharge::new(&self.budget, "pending native product retention");
        charge.grow(
            request
                .retained_bytes()?
                .saturating_add(lctx_native::installed_view::contribution_heap(&descriptor))
                .saturating_add(512),
        )?;
        let mut pending = self.private_products.lock().map_err(|_| poisoned())?;
        pending.try_reserve(1).map_err(ModelError::codec)?;
        pending.push((request, descriptor, charge));
        Ok(())
    }
    pub(crate) async fn retain_materialized_products(
        &self,
        native: &Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
    ) -> Result<(), ModelError> {
        let pending = std::mem::take(&mut *self.private_products.lock().map_err(|_| poisoned())?);
        for (request, descriptor, _charge) in pending {
            self.cancellation.check()?;
            let physical = lctx_model::domain::completed::attempt_contribution(
                native.attempt(),
                descriptor.spec.identity()?,
            );
            let actual = native.completed_contribution(physical).await?;
            if actual != descriptor {
                return Err(ModelError::Conflict(
                    "materialized retained product descriptor",
                ));
            }
            // The first request is a real accepted native effect. Propagate refusal and
            // Unknown unchanged; native finalization remains owned by the explicit target.
            native
                .retain_product_identity(request.identity()?, physical, &self.budget)
                .await?;
        }
        Ok(())
    }
    pub(super) async fn retain_product(
        &self,
        request: ProductRequest,
        production: Arc<lctx_compiler_store::compiler::CompletedProduction>,
    ) -> Result<(), ModelError> {
        use lctx_compiler_store::compiler::RetentionPreparation;
        let identity = request.identity()?;
        // No dispatcher/native admission before an explicit effect-free outcome.
        let prepared = tokio::select! {
            biased;
            () = self.cancellation.cancelled() => { self.cancellation.check()?; unreachable!("cancelled token checked"); }
            prepared = self.native()?.prepare_product_retention(identity, production, &self.budget) => prepared?,
        };
        match prepared {
            RetentionPreparation::Declined => {
                tracing::debug!("optional product capture declined before native admission");
                Ok(())
            }
            RetentionPreparation::Ready(prepared) => {
                self.cancellation.check()?;
                let native = self.native()?.clone();
                let budget = self.budget.clone();
                self.native_call(
                    async move { native.retain_prepared_product(prepared, &budget).await },
                )
                .await
            }
        }
    }
}
fn canonical_bodies(relation: &Relation, batch: &RecordBatch) -> Result<Vec<Value>, ModelError> {
    use arrow_array::FixedSizeBinaryArray;
    let ids = batch
        .column_by_name("id")
        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or(ModelError::Schema("product nominal keys"))?;
    let mut bodies = lctx_native::codec::batch_bodies(relation, batch)?;
    for (index, body) in bodies.iter_mut().enumerate() {
        let Value::Object(fields) = body else {
            return Err(ModelError::Schema("product native body"));
        };
        fields.insert("id", hex::encode(ids.value(index)));
    }
    Ok(bodies)
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{
        admission::Frontier,
        input::{Package, Release},
    };
    fn config() -> lctx_native::RuntimeConfig {
        lctx_native::RuntimeConfig::read(std::path::Path::new(
            &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap()
    }
    async fn workspace(config: &lctx_native::RuntimeConfig) -> Arc<Workspace> {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(32 << 20).unwrap();
        let native = lctx_compiler_store::compiler::NativeCompilerStore::begin(
            config,
            Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        Workspace::with_budget(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
            budget,
            native,
        )
        .unwrap()
    }
    fn output(workspace: &Arc<Workspace>, operation: &'static str) -> ProducerOutput {
        static IMPLEMENTATION: std::sync::OnceLock<ContentHash> = std::sync::OnceLock::new();
        let implementation = *IMPLEMENTATION
            .get_or_init(|| lctx_native::control::fresh_identity("product-control").unwrap());
        workspace.output(
            operation,
            Profile::Catalog,
            implementation,
            workspace.inputs(operation, Profile::Catalog, []).unwrap(),
            [Package::NAME],
        )
    }
    async fn earned(
        workspace: &Arc<Workspace>,
        operation: &'static str,
    ) -> (
        ProductRequest,
        Arc<lctx_compiler_store::compiler::CompletedProduction>,
    ) {
        let produced = output(workspace, operation);
        let request = produced.product_request().unwrap();
        produced.declare_async::<Package>().await.unwrap();
        let id = produced.registration_request().await.unwrap();
        let scope = produced.producing_scope().unwrap();
        produced.finish(ProviderOutcome::Complete).await.unwrap();
        let native = workspace.native().unwrap().clone();
        let budget = workspace.budget.clone();
        let result = workspace
            .native_call(async move {
                native
                    .complete_singleton_scoped(
                        &scope,
                        id,
                        ProviderOutcome::Complete,
                        &[Relation::of::<Package>()],
                        &budget,
                    )
                    .await
            })
            .await
            .unwrap();
        (request, result)
    }
    #[tokio::test]
    async fn singleton_completion_progresses_while_current_visibility_gate_is_held() {
        let cfg = config();
        let workspace = workspace(&cfg).await;
        let produced = output(&workspace, "singleton-before-visibility");
        produced.declare_async::<Package>().await.unwrap();
        let id = produced.registration_request().await.unwrap();
        let gate = workspace.completion_gate.lock().await;
        let finished = tokio::spawn(produced.finish(ProviderOutcome::Complete));
        let observer = lctx_compiler_store::compiler::check_installation(&cfg)
            .await
            .unwrap();
        // Observe a real completed descriptor before releasing visibility. The deadline
        // bounds a deadlock control; it makes no speed or throughput claim.
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let mut response = observer
                    .query("SELECT completed FROM $record")
                    .bind((
                        "record",
                        lctx_native::surrealdb::types::RecordId::new(
                            "compiler_contribution",
                            id.hex(),
                        ),
                    ))
                    .await
                    .unwrap()
                    .check()
                    .unwrap();
                let rows: Vec<Value> = response.take(0).unwrap();
                if rows
                    .first()
                    .and_then(Value::as_object)
                    .and_then(|row| row.get("completed"))
                    == Some(&Value::Bool(true))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(
            workspace.relation(Package::NAME).is_err(),
            "singleton metadata grants no current binding"
        );
        assert!(!finished.is_finished());
        drop(gate);
        finished.await.unwrap().unwrap();
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(), 0);
        observer.invalidate().await.unwrap();
        workspace.drain().await.unwrap();
        workspace.native().unwrap().abandon().await.unwrap();
    }
    #[tokio::test]
    async fn private_capture_wait_does_not_hold_disjoint_completion_or_change_old_view() {
        use lctx_compiler_store::compiler::RetentionPreparation;
        let cfg = config();
        let workspace = workspace(&cfg).await;
        let (_, seed) = earned(&workspace, "private-wait-seed").await;
        let old = workspace.relation(Package::NAME).unwrap();
        let produced = output(&workspace, "private-wait-producer");
        let request = produced.product_request().unwrap();
        let identity = request.identity().unwrap();
        // A wholly local prepared owner holds this exact request gate. No native
        // product or pin has been created by that preparation.
        let RetentionPreparation::Ready(blocker) = workspace
            .native()
            .expect("actual native fixture owner")
            .prepare_product_retention(identity, seed.clone(), &workspace.budget)
            .await
            .unwrap()
        else {
            panic!("local preparation should fit")
        };
        *produced.product_capture.lock().unwrap() = Some(request);
        // This client's SDK router runs on the current async test runtime;
        // synchronous provider callbacks belong on their blocking execution lane.
        produced.declare_kind_async::<Package>(true).await.unwrap();
        produced
            .push(Package {
                name: "private-wait-row".into(),
            })
            .await
            .unwrap();
        let id = produced.registration_request().await.unwrap();
        let finished = tokio::spawn(produced.finish(ProviderOutcome::Complete));
        let observer = lctx_compiler_store::compiler::check_installation(&cfg)
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let mut response = observer
                    .query("SELECT completed FROM $record")
                    .bind((
                        "record",
                        lctx_native::surrealdb::types::RecordId::new(
                            "compiler_contribution",
                            id.hex(),
                        ),
                    ))
                    .await
                    .unwrap()
                    .check()
                    .unwrap();
                let rows: Vec<Value> = response.take(0).unwrap();
                if rows
                    .first()
                    .and_then(Value::as_object)
                    .and_then(|row| row.get("completed"))
                    == Some(&Value::Bool(true))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let disjoint = workspace.output(
            "private-wait-disjoint",
            Profile::Catalog,
            ContentHash::of(b"private-wait-disjoint"),
            workspace
                .inputs("private-wait-disjoint", Profile::Catalog, [])
                .unwrap(),
            [Release::NAME],
        );
        disjoint.declare_async::<Release>().await.unwrap();
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            disjoint.finish(ProviderOutcome::Complete),
        )
        .await
        .unwrap()
        .unwrap();
        let release = workspace.relation(Release::NAME).unwrap();
        assert_eq!(
            release.rows(),
            0,
            "a real unrelated empty relation is composed and bound"
        );
        assert_eq!(release.producer(), "private-wait-disjoint");
        assert_eq!(release.view().relation, Release::NAME);
        assert_eq!(release.view().contributions.len(), 1);
        assert_ne!(release.view_identity(), old.view_identity());
        // The exact producer is now ready while its optional child is still held
        // behind a distinct request gate. Previously this awaited retention.
        tokio::time::timeout(std::time::Duration::from_secs(30), finished)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_ne!(
            workspace.relation(Package::NAME).unwrap().view_identity(),
            old.view_identity()
        );
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(), 1);
        assert_eq!(workspace.execution.retention.lock().unwrap().len(), 1);
        let final_join = workspace.execution.join_retention();
        tokio::pin!(final_join);
        assert!(
            futures::poll!(&mut final_join).is_pending(),
            "required readiness does not claim optional terminality"
        );
        drop(blocker);
        final_join.await.unwrap();
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(), 1);
        assert_eq!(old.rows(), 0);
        observer.invalidate().await.unwrap();
        drop(seed);
        workspace.drain().await.unwrap();
        workspace.native().unwrap().abandon().await.unwrap();
    }
    #[tokio::test]
    async fn optional_capture_declines_before_dispatch_and_preserves_empty_required_output() {
        use lctx_compiler_store::compiler::RetentionPreparation;
        let workspace = workspace(&config()).await;
        let (request, production) = earned(&workspace, "capture-local-refusal").await;
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(), 0);
        let identity = request.identity().unwrap();
        let before = workspace.budget.reserved();
        let fill = workspace
            .budget
            .reserve(
                "control local capture refusal",
                workspace.budget.limit() - before,
            )
            .unwrap();
        assert!(matches!(
            workspace
                .native()
                .expect("actual native fixture owner")
                .prepare_product_retention(identity, production.clone(), &workspace.budget)
                .await
                .unwrap(),
            RetentionPreparation::Declined
        ));
        // The production route sees the same explicit optional result without dispatching
        // a mandatory operation, poisoning current views, or cancelling its caller.
        workspace
            .retain_product(request.clone(), production.clone())
            .await
            .unwrap();
        workspace.cancellation.check().unwrap();
        workspace.writable().unwrap();
        drop(fill);
        assert_eq!(workspace.budget.reserved(), before);
        let (retained, charge) = workspace
            .native()
            .expect("actual native fixture owner")
            .retained_product_views(&workspace.budget)
            .await
            .unwrap();
        assert!(
            retained.is_empty(),
            "decline is not successful empty capture"
        );
        drop((retained, charge));
        // Empty required output is nevertheless eligible for a real successful capture.
        workspace
            .retain_product(request, production.clone())
            .await
            .unwrap();
        let (retained, charge) = workspace
            .native()
            .expect("actual native fixture owner")
            .retained_product_views(&workspace.budget)
            .await
            .unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].rows, 0);
        drop((retained, charge, production));
        workspace.freeze_for_admission(None).await.unwrap();
        workspace.drain().await.unwrap();
        workspace.native().unwrap().abandon().await.unwrap();
    }
    #[tokio::test]
    async fn prepared_capture_drop_releases_local_backing_and_post_cutoff_refusal_is_fatal() {
        use lctx_compiler_store::compiler::RetentionPreparation;
        let workspace = workspace(&config()).await;
        let (request, production) = earned(&workspace, "capture-cutoff").await;
        let identity = request.identity().unwrap();
        let prepared = workspace
            .native()
            .expect("actual native fixture owner")
            .prepare_product_retention(identity, production.clone(), &workspace.budget)
            .await
            .unwrap();
        let retained = workspace.budget.reserved();
        let mut waiting = Box::pin(workspace.native().unwrap().prepare_product_retention(
            identity,
            production.clone(),
            &workspace.budget,
        ));
        assert!(matches!(
            futures::poll!(waiting.as_mut()),
            std::task::Poll::Pending
        ));
        drop(waiting); // Cancellation before a local gate is acquired has no native owner.
        drop(prepared);
        assert!(workspace.budget.reserved() < retained);
        let RetentionPreparation::Ready(prepared) = workspace
            .native()
            .expect("actual native fixture owner")
            .prepare_product_retention(identity, production.clone(), &workspace.budget)
            .await
            .unwrap()
        else {
            panic!("local admission should fit")
        };
        let fill = workspace
            .budget
            .reserve(
                "control strict capture cutoff",
                workspace.budget.limit() - workspace.budget.reserved(),
            )
            .unwrap();
        let native = workspace.native().unwrap().clone();
        let budget = workspace.budget.clone();
        let failure = workspace
            .native_call(async move { native.retain_prepared_product(prepared, &budget).await })
            .await;
        assert!(
            failure.is_err(),
            "mandatory native refusal cannot become decline"
        );
        assert!(workspace.cancellation.check().is_err());
        drop((fill, production));
        assert!(workspace.freeze_for_admission(None).await.is_err());
        let finality = workspace.drain_owned_report().await;
        workspace.complete_signal(&finality).await.unwrap();
        lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.native().unwrap().abandon_finality_report().await,
        )
        .unwrap();
    }
    #[test]
    fn optional_capture_preserves_typed_refusal_and_unknown_native_disposition() {
        use datafusion::error::DataFusionError;
        let refusal = ModelError::Resource {
            owner: "native-projected-arrow",
            requested: 1024,
            used: 2048,
            limit: 2048,
        };
        assert!(matches!(
            crate::sql::model_error(DataFusionError::External(Box::new(refusal))),
            ModelError::Resource {
                owner: "native-projected-arrow",
                ..
            }
        ));
        let unknown = lctx_model::domain::completion::complete::<()>(
            Ok(()),
            lctx_model::domain::completion::Completion {
                remote: lctx_model::domain::completion::RemoteState::Unknown,
                ..Default::default()
            },
        )
        .unwrap_err();
        let preserved = crate::sql::model_error(DataFusionError::External(Box::new(unknown)));
        assert!(!preserved.permits_storage_cleanup());
        assert!(matches!(preserved, ModelError::Completion(_)));
        let shared = Arc::new(DataFusionError::External(Box::new(preserved)));
        let wrapped = crate::sql::model_error(DataFusionError::Shared(shared.clone()));
        assert!(!crate::sql::product_fallback_allowed(&wrapped));
    }
    #[test]
    fn public_product_bodies_reject_extra_fields_and_cross_window_duplicate_ids() {
        let model = lctx_model::domain::model().unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let row = Package {
            name: "canonical".into(),
        };
        let relation = Relation::of::<Package>();
        let body = canonical_bodies(&relation, &Package::encode(&[row]).unwrap())
            .unwrap()
            .pop()
            .unwrap();
        let request = ProductRequest {
            kind: ProductKind::PureRows,
            operation: "canonical-control".into(),
            model: model.digest(),
            implementation: ContentHash::of(b"i"),
            policy: ContentHash::of(b"p"),
            result_contract: ContentHash::of(b"r"),
            configuration: None,
            profile: "catalog".into(),
            parameters: vec![],
            dependencies: vec![],
            outputs: [Package::NAME.into()].into(),
        };
        let mut product = PortableProduct {
            request,
            outcome: ProductOutcome::Complete,
            sections: vec![ProductSection {
                name: Package::NAME.into(),
                rows: 1,
                bytes: serde_json::to_vec(&vec![body.clone()]).unwrap(),
            }],
        };
        validate_product_rows(&product, &model, &budget, 1).unwrap();
        let mut extra = body.clone();
        let Value::Object(fields) = &mut extra else {
            panic!("object")
        };
        fields.insert("unexpected", true);
        product.sections[0].bytes = serde_json::to_vec(&vec![extra]).unwrap();
        assert!(validate_product_rows(&product, &model, &budget, 1).is_err());
        product.sections[0].rows = 2;
        product.sections[0].bytes = serde_json::to_vec(&vec![body.clone(), body]).unwrap();
        assert!(validate_product_rows(&product, &model, &budget, 1).is_err());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn private_cache_miss_queues_local_retention_without_allocating_native_target() {
        let config = config();
        let options = WorkspaceOptions::default();
        let budget = ResourceBudget::fixed(options.memory_bytes).unwrap();
        let factory = lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
            .expect("actual cache recovery owner");
        let workspace = Workspace::private_with_budget(
            Arc::new(lctx_model::domain::model().unwrap()),
            options,
            budget,
        )
        .unwrap();
        let cache = lctx_compiler_store::NativeProductCache::connect(&config, &factory)
            .await
            .unwrap()
            .map(Arc::new)
            .expect("real configured retained-product cache");
        workspace.set_product_cache(Some(cache)).unwrap();
        let produced = output(&workspace, "private-cache-miss-no-native-target");
        assert!(
            produced.reuse_product().await.unwrap().is_none(),
            "fresh exact request has no retained native product"
        );
        assert!(produced.product_capture.lock().unwrap().is_some());
        assert!(
            workspace.native().is_err(),
            "cache discovery has its own genuine owner, never a receiver compilation attempt"
        );
        produced.declare_async::<Package>().await.unwrap();
        produced
            .push(Package {
                name: "private-cache-miss".into(),
            })
            .await
            .unwrap();
        produced.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.execution.join_retention().await.unwrap();
        assert_eq!(
            workspace.private_products.lock().unwrap().len(),
            1,
            "the useful consumer queues local retention metadata before native materialization"
        );
        let completed = workspace.completed::<Package>().unwrap();
        let rows = completed
            .private_source()
            .unwrap()
            .read_batches()
            .unwrap()
            .flat_map(|batch| Package::decode(&batch.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![Package {
                name: "private-cache-miss".into()
            }]
        );
        assert!(
            workspace.native().is_err(),
            "a completed local producer cannot trigger persistence for optional capture"
        );
        drop(completed);
        workspace.drain().await.unwrap();
        assert!(
            workspace.product_cache().unwrap().is_none(),
            "actual cache session close belongs to private execution drainage"
        );
    }
    #[tokio::test]
    async fn completion_and_marker_only_admission_cannot_grant_attachment() {
        let config = config();
        let first = workspace(&config).await;
        let produced = output(&first, "pure-package");
        assert!(produced.reuse_product().await.unwrap().is_none());
        produced.declare_async::<Package>().await.unwrap();
        produced
            .push(Package {
                name: "expected".into(),
            })
            .await
            .unwrap();
        produced.finish(ProviderOutcome::Complete).await.unwrap();
        let unadmitted = workspace(&config).await;
        assert!(
            output(&unadmitted, "pure-package")
                .reuse_product()
                .await
                .unwrap()
                .is_none(),
            "completion alone grants no admitted attachment"
        );
        unadmitted.native().unwrap().abandon().await.unwrap();
        // A Package-only fixture cannot earn full semantic admission. Positive attachment and
        // fresh-owner checks live in compilation::cache_controls after artifact::admit executes
        // the actual captured-provider and native premises.
        first.freeze_for_admission(None).await.unwrap();
        assert!(matches!(
            first.native().unwrap().mark_attempt_admitted().await,
            Err(ModelError::Conflict(
                "native admission requires exact physical completion and executed full premises"
            ))
        ));
        let second = workspace(&config).await;
        let hit = output(&second, "pure-package");
        assert!(hit.reuse_product().await.unwrap().is_none());
        assert!(second.relation(Package::NAME).is_err());
        drop(hit);
        first.native().unwrap().abandon().await.unwrap();
        second.native().unwrap().abandon().await.unwrap();
    }
}

#[cfg(test)]
mod retention_join_controls {
    use super::*;
    #[tokio::test]
    async fn unconfigured_private_cache_declines_before_request_or_retention_effects() {
        let workspace = Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            Default::default(),
        )
        .unwrap();
        let produced = workspace.output(
            "private-no-cache",
            Profile::Catalog,
            ContentHash::of(b"private-no-cache"),
            workspace
                .inputs("private-no-cache", Profile::Catalog, [])
                .unwrap(),
            [lctx_model::domain::input::Package::NAME],
        );
        assert!(produced.reuse_product().await.unwrap().is_none());
        assert!(
            produced.captured_request.get().is_none(),
            "no configured consumer means no rich request preparation"
        );
        assert!(produced.product_capture.lock().unwrap().is_none());
        produced
            .declare_async::<lctx_model::domain::input::Package>()
            .await
            .unwrap();
        produced.finish(ProviderOutcome::Complete).await.unwrap();
        assert!(workspace.private_products.lock().unwrap().is_empty());
        assert!(workspace.native().is_err());
        assert_eq!(
            workspace
                .completed::<lctx_model::domain::input::Package>()
                .unwrap()
                .rows(),
            0
        );
        workspace.drain().await.unwrap();
    }
    #[tokio::test]
    async fn accepted_retention_failure_is_observed_by_the_enclosing_join() {
        let workspace = Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            Default::default(),
        )
        .unwrap();
        let (release, held) = tokio::sync::oneshot::channel();
        workspace
            .retain_child(async move {
                held.await.map_err(ModelError::codec)?;
                Err(ModelError::Conflict("accepted retention failure control"))
            })
            .unwrap();
        let joined = workspace.execution.join_retention();
        tokio::pin!(joined);
        assert!(futures::poll!(&mut joined).is_pending());
        release.send(()).unwrap();
        let error = joined.await.unwrap_err();
        assert!(format!("{error:?}").contains("accepted retention failure control"));
        assert!(
            workspace.execution.join_retention().await.is_err(),
            "retrying observation cannot turn accepted failure into a cache miss"
        );
    }
}
