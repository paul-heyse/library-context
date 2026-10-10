//! Admitted operation products attach memberships; selected kernel products retain typed values.
use super::*;
use lctx_model::domain::{Key, KeySink, compilation_product::*};
use lctx_surrealdb::surrealdb::types::Value;

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
        cache: Option<Arc<lctx_surrealdb::NativeProductCache>>,
    ) -> Result<(), ModelError> {
        self.writable()?;
        *self.product_cache.lock().map_err(|_| poisoned())? = cache;
        Ok(())
    }
    pub(crate) fn product_cache(
        &self,
    ) -> Result<Option<Arc<lctx_surrealdb::NativeProductCache>>, ModelError> {
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
            })
        }
        .boxed()
    }
}
impl ProducerOutput {
    pub(crate) fn product_request(&self) -> Result<ProductRequest, ModelError> {
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
        let request = ProductRequest {
            kind: ProductKind::PureRows,
            operation: self.name.into(),
            model: self.workspace.model.digest(),
            implementation: self.implementation,
            policy: ContentHash::of(&lctx_model::domain::SEMANTIC_POLICY_REVISION.to_le_bytes()),
            result_contract: contract.finish(),
            configuration: self.configuration,
            profile: self.profile.name().into(),
            parameters: vec![],
            dependencies,
            outputs: self.allowed.iter().map(|s| (*s).into()).collect(),
        };
        request.validate()?;
        Ok(request)
    }
    /// Miss arms terminal retention; admitted native hits attach current memberships.
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
    fn reuse_product_inner<'a, F>(
        &'a self,
        prepare_owner: bool,
        check: F,
    ) -> BoxFuture<'a, Result<Option<ProviderOutcome>, ModelError>>
    where
        F: FnOnce(Arc<ProductCandidate>) -> BoxFuture<'a, Result<(), ModelError>> + Send + 'a,
    {
        async move {
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
            let native = self.workspace.native.clone();
            let identity = request.identity()?;
            let spec = self.contribution_spec();
            let retained = if self
                .workspace
                .admitted_product_reuse
                .load(Ordering::Acquire)
            {
                self.workspace
                    .native_call(
                        async move { native.attach_retained_product(identity, &spec).await },
                    )
                    .await?
            } else {
                None
            };
            if let Some((id, _, descriptor)) = retained {
                // Persisted admission does not carry ephemeral Rust verification owners.
                // Reconstruct those only for consumers that require them, without ingress.
                if prepare_owner {
                    let mut charge = self
                        .workspace
                        .budget
                        .reserve("admitted-product-application", 0)?;
                    let mut sections = Vec::with_capacity(inventory.len());
                    for relation in &inventory {
                        let mut stream = self
                            .workspace
                            .native
                            .scan_contribution_batches(
                                id,
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
                    .await?;
                }
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
            let batch = lctx_surrealdb::codec::decode_bodies(relation, window.to_vec(), budget)?;
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
    pub(super) async fn retain_product(
        &self,
        request: ProductRequest,
        id: ContentHash,
        outcome: ProviderOutcome,
    ) -> Result<(), ModelError> {
        match self.retain_product_owned(request, id, outcome).await {
            Err(ModelError::Resource { .. } | ModelError::Limit { .. }) => {
                tracing::debug!("optional product capture refused; completed producer retained");
                Ok(())
            }
            result => result,
        }
    }
    async fn retain_product_owned(
        &self,
        request: ProductRequest,
        id: ContentHash,
        outcome: ProviderOutcome,
    ) -> Result<(), ModelError> {
        let identity = request.identity()?;
        let native = self.native.clone();
        self.native_call(async move { native.retain_product_identity(identity, id).await })
            .await?;
        let _ = outcome;
        Ok(())
    }
}
fn canonical_bodies(relation: &Relation, batch: &RecordBatch) -> Result<Vec<Value>, ModelError> {
    use arrow_array::FixedSizeBinaryArray;
    let ids = batch
        .column_by_name("id")
        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or(ModelError::Schema("product nominal keys"))?;
    let mut bodies = lctx_surrealdb::codec::batch_bodies(relation, batch)?;
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
    use lctx_model::domain::{admission::Frontier, input::Package};
    fn config() -> lctx_surrealdb::RuntimeConfig {
        lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(
            &std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap()
    }
    async fn workspace(config: &lctx_surrealdb::RuntimeConfig) -> Arc<Workspace> {
        let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(config, Frontier::Facts)
            .await
            .unwrap();
        Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
            native,
        )
        .unwrap()
    }
    fn output(workspace: &Arc<Workspace>, operation: &'static str) -> ProducerOutput {
        static IMPLEMENTATION: std::sync::OnceLock<ContentHash> = std::sync::OnceLock::new();
        let implementation = *IMPLEMENTATION
            .get_or_init(|| lctx_surrealdb::control::fresh_identity("product-control").unwrap());
        workspace.output(
            operation,
            Profile::Catalog,
            implementation,
            workspace.inputs(operation, Profile::Catalog, []).unwrap(),
            [Package::NAME],
        )
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
    async fn native_admission_gates_attachment_and_current_ownership() {
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
        unadmitted.native().abandon().await.unwrap();
        // This native ownership control exercises the admission gate. Full semantic admission
        // is established independently by the real-provider compilation matrices.
        first.freeze_for_admission(None).await.unwrap();
        first.native().mark_attempt_admitted().await.unwrap();
        let old = first
            .inputs("old", Profile::Catalog, [Package::NAME])
            .unwrap();
        let second = workspace(&config).await;
        let hit = output(&second, "pure-package");
        assert!(matches!(
            hit.reuse_product().await.unwrap(),
            Some(ProviderOutcome::Complete)
        ));
        hit.finish(ProviderOutcome::Complete).await.unwrap();
        let new = second
            .inputs("new", Profile::Catalog, [Package::NAME])
            .unwrap();
        assert_eq!(
            first.relation(Package::NAME).unwrap().view_identity(),
            second.relation(Package::NAME).unwrap().view_identity()
        );
        assert!(old.require_subset(&second, &new).is_err());
        let native = second.native().clone();
        let mut rows = native
            .scan_batches(
                second.relation(Package::NAME).unwrap().view(),
                &Relation::of::<Package>(),
                None,
                None,
                second.budget(),
                32,
            )
            .await
            .unwrap();
        let batch = rows.try_next().await.unwrap().unwrap();
        assert_eq!(
            Package::decode(&batch).unwrap(),
            vec![Package {
                name: "expected".into()
            }]
        );
        assert!(rows.try_next().await.unwrap().is_none());
        second.freeze_for_admission(None).await.unwrap();
        first.native().abandon().await.unwrap();
        second.native().abandon().await.unwrap();
    }
}
