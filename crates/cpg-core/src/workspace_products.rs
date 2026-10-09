//! Optional pure products replay through the same current contribution and native typed ingress.
use super::*;
use lctx_model::domain::{Key,KeySink,compilation_product::*};
use lctx_surrealdb::surrealdb::types::Value;

pub(crate) struct ProductCandidate {
    sections: Vec<(Relation, Vec<RecordBatch>)>,
    _charge: Box<dyn Reservation>,
}
impl ProductCandidate {
    pub(crate) fn sections(&self) -> impl Iterator<Item = (&Relation, &[RecordBatch])> {
        self.sections.iter().map(|(relation, batches)| (relation, batches.as_slice()))
    }
    pub(crate) fn batches(&self, name: &str) -> Option<&[RecordBatch]> {
        self.sections.iter().find(|(relation, _)| relation.name() == name).map(|(_, batches)| batches.as_slice())
    }
}

impl Workspace {
    /// Installation is an explicit store/fixture operation; ordinary attempts only connect.
    pub fn set_product_cache(&self,cache:Option<Arc<lctx_surrealdb::NativeProductCache>>)->Result<(),ModelError> {
        self.writable()?;
        *self.product_cache.lock().map_err(|_|poisoned())?=cache;Ok(())
    }
    pub(crate) fn product_cache(&self)->Result<Option<Arc<lctx_surrealdb::NativeProductCache>>,ModelError> {
        Ok(self.product_cache.lock().map_err(|_|poisoned())?.clone())
    }
}
struct ReplayWriter {relation:Relation,contribution:bool}
type PreparedReplay = ProductCandidate;
impl ErasedWriter for ReplayWriter {
    fn as_any_mut(&mut self)->&mut dyn Any {self}
    fn close(self:Box<Self>,_model:Arc<ValidatedModel>,_budget:ResourceBudget)->BoxFuture<'static,Result<PendingRelation,ModelError>> {
        async move {Ok(PendingRelation{relation:self.relation,contribution:self.contribution})}.boxed()
    }
}
impl ProducerOutput {
    pub(crate) fn product_request(&self)->Result<ProductRequest,ModelError> {
        let mut contract=KeySink::new("compiler-product-row-contract/v1");
        for name in &self.allowed {
            let relation=self.workspace.model.relation(name).ok_or(ModelError::Schema("product output relation"))?;
            relation.name().to_owned().encode(&mut contract);
            // The model contract includes typed schemas, invariants and their implementation.
            self.workspace.model.digest().encode(&mut contract);
        }
        let dependencies=self.inputs.ordered_bindings().into_iter().map(|((name,_),source)|DependencyToken {
            kind:DependencyKind::ExactView,role:format!("input/{}",source.role),relation:name.into(),
            prefix:source.resolved.map(|p|p.name().into()),identity:source.snapshot.identity(),
        }).collect();
        let request=ProductRequest{kind:ProductKind::PureRows,operation:self.name.into(),model:self.workspace.model.digest(),implementation:self.implementation,
            policy:ContentHash::of(&lctx_model::domain::SEMANTIC_POLICY_REVISION.to_le_bytes()),result_contract:contract.finish(),configuration:self.configuration,
            profile:self.profile.name().into(),parameters:vec![],dependencies,outputs:self.allowed.iter().map(|s|(*s).into()).collect()};
        request.validate()?;Ok(request)
    }
    /// Miss arms terminal capture; hit writes data through fresh ordinary contributions.
    pub(crate) fn reuse_product(&self)->BoxFuture<'_,Result<Option<ProviderOutcome>,ModelError>> {
        self.reuse_product_checked_async(|_|async {Ok(())}.boxed())
    }
    pub(crate) fn reuse_product_checked_async<'a,F>(&'a self,check:F)->BoxFuture<'a,Result<Option<ProviderOutcome>,ModelError>> where F:FnOnce(Arc<ProductCandidate>)->BoxFuture<'a,Result<(),ModelError>>+Send+'a {
        async move {
            self.check()?;
            let Some(cache)=self.workspace.product_cache()? else{return Ok(None)};
            let request=self.product_request()?;
            let cache_read=cache.clone();let lookup=request.clone();let budget=self.workspace.budget.clone();
            let leased=self.workspace.native_call(async move{cache_read.lookup(&lookup,&budget).await}).await?;
            if let Some(leased)=leased {
                let product=leased.product();
                let prepared = match decode_product_rows(product, &self.workspace.model, self.workspace.budget(), self.workspace.options.batch_rows) {
                    Ok(candidate) => {
                        let candidate = Arc::new(candidate);
                        check(candidate.clone()).await.and_then(|()| Arc::try_unwrap(candidate)
                            .map_err(|_| ModelError::Conflict("cached predicate retained replay candidate")))
                    },
                    Err(error) => Err(error),
                };
                if let Err(error)=prepared {
                    if !crate::sql::product_fallback_allowed(&error) {
                        return Err(error);
                    }
                    tracing::warn!(%error,operation=self.name,"discarding semantically invalid derived product");
                    drop(leased);
                    if !matches!(error.primary(),Some(ModelError::Resource{..}|ModelError::Limit{..})) {
                        let invalidate=request.clone();let cache=cache.clone();
                        self.workspace.native_call(async move{cache.invalidate(&invalidate).await}).await?;
                    }
                } else {
                    let outcome=match product.outcome {ProductOutcome::Complete=>ProviderOutcome::Complete,ProductOutcome::NotRequested=>ProviderOutcome::NotRequested};
                    // All cache-only allocations/admission happen before mutation. Once owned
                    // typed batches exist, release cache leases/bytes before native ingress.
                    drop(leased);
                    self.replay_product_rows(prepared.expect("checked prepared replay")).await?;
                    tracing::debug!(operation=self.name,"compiled product reused with fresh ingress");
                    return Ok(Some(outcome));
                }
            }
            *self.product_capture.lock().map_err(|_|poisoned())?=Some(request);Ok(None)
        }.boxed()
    }
    async fn replay_product_rows(&self,prepared:PreparedReplay)->Result<(),ModelError> {
        let ProductCandidate{sections,_charge}=prepared;
        // Prepare the complete writer inventory and its allocations before native
        // registration. No optional-product fallback is permitted after that first effect.
        let scope = self.producing_scope()?;
        {
            let mut writers = self.writers.lock().map_err(|_| poisoned())?;
            if sections.iter().any(|(relation, _)| writers.contains_key(relation.name())) {
                return Err(ModelError::Conflict("cached output already declared"));
            }
            for (relation, _) in &sections {
                writers.insert(relation.name(), Box::new(ReplayWriter {
                    contribution: lctx_model::domain::stages::is_epoch_shared(relation.name()), relation: relation.clone(),
                }));
            }
        }
        let registration = self.registration_request();
        let producer = registration.await?;
        for (relation,batches) in sections {
            self.workspace.cancellation.check()?;
            for batch in batches {
                let native=self.workspace.native.clone();let relation=relation.clone();let scope=scope.clone();
                self.workspace.native_call(async move{scope.run(native.write_batch(&producer,&relation,&batch)).await}).await?;
            }
        }Ok(())
    }
}
/// One canonical typed-body/global-order check shared by whole-stage and selected products.
#[cfg(test)]
pub(crate) fn validate_product_rows(product:&PortableProduct,model:&ValidatedModel,budget:&ResourceBudget,batch_rows:usize)->Result<(),ModelError> {
    decode_product_rows(product, model, budget, batch_rows).map(drop)
}
/// Decode one charged typed candidate. Canonical checks, semantic predicates and ingress all
/// use these batches; no encoded clone or second JSON/Arrow decoder exists on the hit path.
pub(crate) fn decode_product_rows(product:&PortableProduct,model:&ValidatedModel,budget:&ResourceBudget,batch_rows:usize)->Result<ProductCandidate,ModelError> {
    product.validate()?;
    if batch_rows == 0 { return Err(ModelError::Schema("cached product batch rows")); }
    let mut charge = budget.reserve("cached-typed-product", 0)?;
    let mut sections = Vec::new();
    for section in &product.sections {
        let relation=model.relation(&section.name).ok_or(ModelError::Schema("cached product relation"))?;
        let _scratch=budget.reserve("cached-canonical-body-validation",section.bytes.len().saturating_mul(16).saturating_add(1024))?;
        let rows:Vec<Value>=serde_json::from_slice(&section.bytes).map_err(ModelError::codec)?;
        if rows.len() as u64!=section.rows {return Err(ModelError::Conflict("cached product row count"));}
        let mut content=relation.content();
        let mut batches = Vec::new();
        for window in rows.chunks(batch_rows) {
            let batch=lctx_surrealdb::codec::decode_bodies(relation,window.to_vec(),budget)?;
            relation.hash_rows(&batch,&mut content)?;
            let bodies=canonical_bodies(relation,&batch)?;
            if bodies!=window {return Err(ModelError::Conflict("cached product canonical typed body"));}
            charge.try_resize(charge.size().saturating_add(lctx_model::domain::logical_batch_bytes(&batch)?).saturating_add(4096))?;
            batches.push(batch);
        }
        charge.try_resize(charge.size().saturating_add(1024))?;
        sections.push((relation.clone(), batches));
    }
    Ok(ProductCandidate { sections, _charge: charge })
}

impl Workspace {
    pub(super) async fn retain_product(&self,request:ProductRequest,id:ContentHash,outcome:ProviderOutcome)->Result<(),ModelError> {
        match self.retain_product_owned(request,id,outcome).await {
            Err(ModelError::Resource{..}|ModelError::Limit{..})=>{tracing::debug!("optional product capture refused; completed producer retained");Ok(())},
            result=>result,
        }
    }
    async fn retain_product_owned(&self,request:ProductRequest,id:ContentHash,outcome:ProviderOutcome)->Result<(),ModelError> {
        let Some(cache)=self.product_cache()? else{return Ok(())};
        let outcome=match outcome {ProviderOutcome::Complete=>ProductOutcome::Complete,ProviderOutcome::NotRequested=>ProductOutcome::NotRequested,_=>return Ok(())};
        let mut retained=self.budget.reserve("compiled-product-capture",0)?;
        let mut sections=Vec::new();
        for name in &request.outputs {
            let relation=self.model.relation(name).ok_or(ModelError::Schema("product capture relation"))?.clone();
            let mut stream=self.native.scan_contribution_batches(id,&relation,self.budget(),self.options.batch_rows).await?;
            let mut values=Vec::new();
            while let Some(batch)=stream.try_next().await.map_err(crate::sql::model_error)? {
                self.cancellation.check()?;
                let bytes=lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(8).saturating_add(1024);
                if let Err(error)=retained.try_resize(retained.size().saturating_add(bytes)) {
                    if matches!(error,ModelError::Resource{..}) {tracing::debug!(%error,"optional product retention unavailable");return Ok(())}return Err(error);
                }
                values.extend(canonical_bodies(&relation,&batch)?);
            }
            let bytes=serde_json::to_vec(&values).map_err(ModelError::codec)?;
            sections.push(ProductSection{name:name.clone(),rows:values.len() as u64,bytes});
        }
        let product=PortableProduct{request,outcome,sections};
        let budget=self.budget.clone();
        self.native_call(async move{let _retained=retained;cache.insert(&product,&budget).await.map(|_|())}).await
    }
}
fn canonical_bodies(relation:&Relation,batch:&RecordBatch)->Result<Vec<Value>,ModelError> {
    use arrow_array::FixedSizeBinaryArray;
    let ids=batch.column_by_name("id").and_then(|c|c.as_any().downcast_ref::<FixedSizeBinaryArray>()).ok_or(ModelError::Schema("product nominal keys"))?;
    let mut bodies=lctx_surrealdb::codec::batch_bodies(relation,batch)?;
    for (index,body) in bodies.iter_mut().enumerate() {
        let Value::Object(fields)=body else{return Err(ModelError::Schema("product native body"))};
        fields.insert("id",hex::encode(ids.value(index)));
    }Ok(bodies)
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{admission::Frontier,input::Package,serving::Name};
    static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
    async fn setup()->(lctx_surrealdb::RuntimeConfig,tempfile::TempDir,Arc<lctx_surrealdb::NativeProductCache>) {
        let mut config=lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"))).unwrap();
        let scratch=tempfile::tempdir().unwrap();
        config.reuse=Some(lctx_surrealdb::ReuseConfig{database:Name::new(format!("core_products_{}_{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed))).unwrap(),capacity_bytes:16<<20,lease_directory:scratch.path().join("leases")});
        let cache=Arc::new(lctx_surrealdb::NativeProductCache::install(&config).await.unwrap().unwrap());
        (config,scratch,cache)
    }
    async fn workspace(config:&lctx_surrealdb::RuntimeConfig,cache:Arc<lctx_surrealdb::NativeProductCache>)->Arc<Workspace> {
        let native=lctx_surrealdb::compiler::NativeCompilerStore::begin(config,Frontier::Facts).await.unwrap();
        let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions{memory_bytes:32<<20,..Default::default()},native).unwrap();
        workspace.set_product_cache(Some(cache)).unwrap();workspace
    }
    fn output(workspace:&Arc<Workspace>,operation:&'static str)->ProducerOutput {
        workspace.output(operation,Profile::Catalog,ContentHash::of(b"pure-test/v1"),workspace.inputs(operation,Profile::Catalog,[]).unwrap(),[Package::NAME])
    }
    #[test]
    fn optional_capture_preserves_typed_refusal_and_unknown_native_disposition() {
        use datafusion::error::DataFusionError;
        let refusal=ModelError::Resource{owner:"native-projected-arrow",requested:1024,used:2048,limit:2048};
        assert!(matches!(crate::sql::model_error(DataFusionError::External(Box::new(refusal))),ModelError::Resource{owner:"native-projected-arrow",..}));
        let unknown=lctx_model::domain::completion::complete::<()>(Ok(()),lctx_model::domain::completion::Completion {
            remote:lctx_model::domain::completion::RemoteState::Unknown,..Default::default()
        }).unwrap_err();
        let preserved=crate::sql::model_error(DataFusionError::External(Box::new(unknown)));
        assert!(!preserved.permits_storage_cleanup());assert!(matches!(preserved,ModelError::Completion(_)));
        let shared=Arc::new(DataFusionError::External(Box::new(preserved)));
        let wrapped=crate::sql::model_error(DataFusionError::Shared(shared.clone()));
        assert!(!crate::sql::product_fallback_allowed(&wrapped));

    }
    #[test]
    fn public_product_bodies_reject_extra_fields_and_cross_window_duplicate_ids() {
        let model=lctx_model::domain::model().unwrap();let budget=ResourceBudget::fixed(1<<20).unwrap();
        let row=Package{name:"canonical".into()};let relation=Relation::of::<Package>();
        let body=canonical_bodies(&relation,&Package::encode(&[row]).unwrap()).unwrap().pop().unwrap();
        let request=ProductRequest{kind:ProductKind::PureRows,operation:"canonical-control".into(),model:model.digest(),implementation:ContentHash::of(b"i"),policy:ContentHash::of(b"p"),result_contract:ContentHash::of(b"r"),configuration:None,profile:"catalog".into(),parameters:vec![],dependencies:vec![],outputs:[Package::NAME.into()].into()};
        let mut product=PortableProduct{request,outcome:ProductOutcome::Complete,sections:vec![ProductSection{name:Package::NAME.into(),rows:1,bytes:serde_json::to_vec(&vec![body.clone()]).unwrap()}]};
        validate_product_rows(&product,&model,&budget,1).unwrap();
        let mut extra=body.clone();let Value::Object(fields)=&mut extra else{panic!("object")};fields.insert("unexpected",true);
        product.sections[0].bytes=serde_json::to_vec(&vec![extra]).unwrap();
        assert!(validate_product_rows(&product,&model,&budget,1).is_err());
        product.sections[0].rows=2;product.sections[0].bytes=serde_json::to_vec(&vec![body.clone(),body]).unwrap();
        assert!(validate_product_rows(&product,&model,&budget,1).is_err());assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn product_reopen_replays_fresh_contributions_not_old_owners() {
        let (config,_scratch,cache)=setup().await;
        let first=workspace(&config,cache).await;
        let produced=output(&first,"pure-package");assert!(produced.reuse_product().await.unwrap().is_none());
        produced.declare_async::<Package>().await.unwrap();produced.push(Package{name:"expected".into()}).await.unwrap();produced.finish(ProviderOutcome::Complete).await.unwrap();
        let old=first.inputs("old",Profile::Catalog,[Package::NAME]).unwrap();
        let reloaded=Arc::new(lctx_surrealdb::NativeProductCache::connect(&config).await.unwrap().unwrap());
        let second=workspace(&config,reloaded).await;let hit=output(&second,"pure-package");
        assert!(matches!(hit.reuse_product().await.unwrap(),Some(ProviderOutcome::Complete)));hit.finish(ProviderOutcome::Complete).await.unwrap();
        let new=second.inputs("new",Profile::Catalog,[Package::NAME]).unwrap();
        assert_eq!(first.relation(Package::NAME).unwrap().view_identity(),second.relation(Package::NAME).unwrap().view_identity());
        assert!(old.require_subset(&second,&new).is_err());
        let native=second.native().clone();let mut rows=native.scan_batches(second.relation(Package::NAME).unwrap().view(),&Relation::of::<Package>(),None,None,second.budget(),32).await.unwrap();
        let batch=rows.try_next().await.unwrap().unwrap();assert_eq!(Package::decode(&batch).unwrap(),vec![Package{name:"expected".into()}]);assert!(rows.try_next().await.unwrap().is_none());
        first.native().abandon().await.unwrap();second.native().abandon().await.unwrap();
    }
    #[tokio::test]
    async fn optional_capture_pressure_preserves_completed_native_output() {
        let (config,_scratch,cache)=setup().await;
        let workspace=workspace(&config,cache.clone()).await;
        workspace.set_product_cache(None).unwrap();
        let produced=output(&workspace,"pressure-package");
        let request=produced.product_request().unwrap();
        produced.declare_async::<Package>().await.unwrap();
        produced.push(Package{name:"completed".into()}).await.unwrap();
        produced.finish(ProviderOutcome::Complete).await.unwrap();
        let contributions=workspace.native().contributions().await.unwrap();
        let id=contributions[0].spec.identity().unwrap();
        workspace.set_product_cache(Some(cache.clone())).unwrap();
        let pressure=workspace.budget().reserve("required-current-work",
            workspace.budget().limit()-workspace.budget().reserved()-1024).unwrap();
        workspace.retain_product(request.clone(),id,ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(),1);
        drop(pressure);
        assert!(cache.lookup(&request,workspace.budget()).await.unwrap().is_none());
        workspace.native().abandon().await.unwrap();
    }
    #[tokio::test]
    async fn product_semantic_corruption_falls_back_before_fresh_ingress() {
        let (config,_scratch,cache)=setup().await;let workspace=workspace(&config,cache.clone()).await;
        let produced=output(&workspace,"corrupt-package");
        let request=produced.product_request().unwrap();
        let corrupt=PortableProduct{request,outcome:ProductOutcome::Complete,sections:vec![ProductSection{name:Package::NAME.into(),rows:1,bytes:b"[]".to_vec()}]};
        assert!(cache.insert(&corrupt,workspace.budget()).await.unwrap());
        assert!(produced.reuse_product().await.unwrap().is_none());
        produced.declare_async::<Package>().await.unwrap();produced.push(Package{name:"fresh".into()}).await.unwrap();produced.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.relation(Package::NAME).unwrap().rows(),1);
        workspace.native().abandon().await.unwrap();
    }
}
