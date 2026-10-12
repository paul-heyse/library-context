//! Actual format-6 product owners. These controls require the installed native fixture;
//! none manufactures semantic admission or substitutes a descriptor for a held owner.
use futures::FutureExt;
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_model::domain::{ContentHash, ModelError, Record, Relation, admission::Frontier,
    completed::{CompletedBinding, ContributionSpec, OutputProfile, attempt_binding}, input::Package, resources::ResourceBudget,
    stages::{Profile, ProviderOutcome}, completion::{Completion, complete}};
use lctx_native::{RuntimeConfig, control};
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, RecordId, Value}};

struct Product {
    source: Arc<NativeCompilerStore>, client: Arc<Surreal<Client>>, budget: ResourceBudget,
    request: ContentHash, contribution: RecordId, product: RecordId,
}
fn config() -> RuntimeConfig {
    RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
        .expect("installed format-6 validation fixture"))).unwrap()
}
impl Product {
    async fn new(cfg: &RuntimeConfig) -> Self {
        let source=NativeCompilerStore::begin(cfg,Frontier::Facts).await.unwrap();
        let client=lctx_compiler_store::compiler::check_installation(cfg).await.unwrap();
        let budget=ResourceBudget::fixed(256<<20).unwrap();
        let request=control::fresh_identity("product-retention-control").unwrap();
        let relation=Relation::of::<Package>();
        let spec=ContributionSpec{captured_binding:None,producer:format!("product-{}",request.hex()),
            profile:OutputProfile::Specific(Profile::Catalog),model:lctx_model::domain::model().unwrap().digest(),
            implementation:ContentHash::of(b"product-retention-control/v1"),configuration:None,
            inputs:vec![],outputs:[Package::NAME.into()].into()};
        let id=source.begin_contribution(spec).await.unwrap();
        let row=Package{name:format!("retention-{}",request.hex())};
        source.write_batch(&id,&relation,&Package::encode(&[row]).unwrap(),&budget).await.unwrap();
        source.complete_contribution(id,ProviderOutcome::Complete,&[relation],&BTreeMap::new(),&budget).await.unwrap();
        source.retain_product_identity(request,id).await.unwrap();
        Self{product:RecordId::new("native_product",format!("{}_{}",request.hex(),source.attempt().hex())),
            contribution:RecordId::new("compiler_contribution",id.hex()),source,client,budget,request}
    }
    async fn row(&self, id: &RecordId) -> Option<Object> {
        let mut response=self.client.query("SELECT * FROM $record").bind(("record",id.clone())).await.unwrap().check().unwrap();
        let rows:Vec<Object>=response.take(0).unwrap();assert!(rows.len()<=1);rows.into_iter().next()
    }
    async fn holds(&self) -> Vec<Object> {
        let mut response=self.client.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$product ORDER BY id")
            .bind(("product",self.product.clone())).await.unwrap().check().unwrap();response.take(0).unwrap()
    }
    async fn release(&self) -> ContentHash {
        control::release_retained_product(self.client.clone(),self.request,self.source.attempt(),&self.budget).await.unwrap().unwrap()
    }
    async fn bind(&self) -> (CompletedBinding, RecordId) {
        let view=self.source.views().await.unwrap().into_iter().find(|view|view.relation==Package::NAME).unwrap();
        let binding=CompletedBinding{boundary:None,configuration:None,
            source:lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(
                &Relation::of::<Package>(),lctx_model::domain::model().unwrap().digest(),&view).unwrap(),view};
        self.source.bind(binding.clone()).await.unwrap();
        let slot=RecordId::new("compiler_binding",attempt_binding(self.source.attempt(),binding.key()).hex());
        (binding,slot)
    }
    async fn finish(self, result: Result<(),Box<dyn std::any::Any+Send>>) {
        let mut terminal=Completion::default();
        terminal.step("product control source close",self.source.abandon().await);
        terminal.step("product control explicit warm release",control::release_retained_product(self.client.clone(),self.request,self.source.attempt(),&self.budget).await.map(|_|()));
        terminal.step("product control observer close",self.client.invalidate().await.map_err(ModelError::codec));
        let cleanup=complete(Ok(()),terminal);
        if let Err(primary)=result {if let Err(error)=cleanup{eprintln!("product control cleanup: {error:#?}");}std::panic::resume_unwind(primary);}
        cleanup.unwrap();
    }
}

#[tokio::test(flavor="multi_thread")]
async fn terminal_binding_slot_collection_preserves_canonical_completion_and_live_peer() {
    let product=Product::new(&config()).await;
    let peer=Product::new(&config()).await;
    let pin=control::ReaderPin::acquire(product.client.clone(),&[]).await.unwrap();
    let result=std::panic::AssertUnwindSafe(async {
        let (binding,slot)=product.bind().await;
        let (_,peer_slot)=peer.bind().await;
        let peer_before=peer.row(&peer_slot).await.unwrap();
        let mut current=binding.clone();current.configuration=Some(ContentHash::of(b"current binding configuration"));
        product.source.bind(current.clone()).await.unwrap();
        let mut response=product.client.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND object=$slot")
            .bind(("attempt",RecordId::new("native_attempt",product.source.attempt().hex())))
            .bind(("slot",slot.clone())).await.unwrap().check().unwrap();
        let holds:Vec<Object>=response.take(0).unwrap();assert_eq!(holds.len(),1,"replacement keeps one owned mutable slot");
        product.source.freeze_content(&product.budget).await.unwrap();
        let physical=product.source.complete_physical_content(&product.budget).await.unwrap();
        pin.protect(physical.completion().record().unwrap()).await.unwrap();
        let canonical=lctx_native::binding_descriptor::record(&current).unwrap();
        let canonical_before=product.row(&canonical).await.unwrap();
        assert_eq!(lctx_native::binding_descriptor::decode(&Value::Object(canonical_before.clone())).unwrap(),current);
        product.source.abandon().await.unwrap();
        assert!(product.row(&slot).await.is_some(),"close releases incidence only after durable nomination; collection is separate");
        let mut response=product.client.query("SELECT job,incarnation,state FROM native_retirement_item WITH INDEX object_retirements WHERE object=$slot ORDER BY id")
            .bind(("slot",slot.clone())).await.unwrap().check().unwrap();
        let items:Vec<Object>=response.take(0).unwrap();assert_eq!(items.len(),1);
        assert_eq!(items[0].get("state"),Some(&Value::String("pending".into())));
        let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(slot.clone())).unwrap()).hex());
        assert_eq!(items[0].get("incarnation"),product.row(&guard).await.unwrap().get("incarnation"),"cleanup nominates the original exact slot incarnation");
        let Some(Value::RecordId(job))=items[0].get("job") else{panic!("slot retirement owner")};
        let surrealdb::types::RecordIdKey::String(job)=&job.key else{panic!("slot retirement key")};
        let job=ContentHash(hex::decode(job).unwrap().try_into().unwrap());
        control::resume_retirement(&product.client,job,128).await.unwrap();
        assert!(product.row(&slot).await.is_none(),"the collector removes the redundant attempt slot");
        assert_eq!(peer.row(&peer_slot).await,Some(peer_before),"a live peer owns its separate mutable slot");
        assert_eq!(product.row(&canonical).await,Some(canonical_before),"completed reads and backup retain canonical full binding meaning");
        assert!(product.row(&RecordId::new("compiler_view",current.view.identity.hex())).await.is_some());
        let portable=lctx_native::binding_descriptor::portable_row(&current).unwrap();
        lctx_native::completed_rows::validate_state_row("compiler_binding",&portable).unwrap();
    }).catch_unwind().await;
    let released=pin.release().await;
    peer.finish(Ok(())).await;product.finish(result).await;released.unwrap();
}

#[tokio::test(flavor="multi_thread")]
async fn unadmitted_close_preserves_warm_roots_until_explicit_incarnation_nomination() {
    let product=Product::new(&config()).await;
    let result=std::panic::AssertUnwindSafe(async {
        let original=product.row(&product.product).await.unwrap();let holds=product.holds().await;
        assert!(!holds.is_empty());
        product.source.abandon().await.unwrap();
        assert_eq!(product.row(&product.product).await,Some(original));
        assert_eq!(product.holds().await,holds,"attempt cleanup cannot drain product-owned output holds");
        let retained=control::retire_reachable(&product.client,vec![product.product.clone()],1).await.unwrap();
        assert_eq!(retained.retired,0);assert_eq!(retained.retained.len(),1,"unreleased product is a live warm owner even without an attempt hold");
        let release=product.release().await;let job=RecordId::new("native_retirement",release.hex());
        assert_eq!(product.release().await,release,"repeated release returns the durable original nomination");
        let released=product.row(&product.product).await.unwrap();
        assert_eq!(released.get("released"),Some(&Value::Bool(true)));assert_eq!(released.get("release_job"),Some(&Value::RecordId(job.clone())));
        assert_eq!(product.holds().await,holds,"release nominates; the collector owns hold removal");
        let guard=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(product.product.clone())).unwrap()).hex());
        let guard=product.row(&guard).await.unwrap();let nomination=product.row(&job).await.unwrap();
        assert_eq!(nomination.get("root_count"),nomination.get("roots_registered"));
        let mut response=product.client.query("SELECT object,incarnation,state FROM native_retirement_item WITH INDEX retirement_queue_v4 WHERE job=$job")
            .bind(("job",job)).await.unwrap().check().unwrap();let items:Vec<Object>=response.take(0).unwrap();
        assert_eq!(items.len(),1);assert_eq!(items[0].get("object"),Some(&Value::RecordId(product.product.clone())));
        assert_eq!(items[0].get("incarnation"),guard.get("incarnation"));assert_eq!(items[0].get("state"),Some(&Value::String("pending".into())));
    }).catch_unwind().await;product.finish(result).await;
}

#[tokio::test(flavor="multi_thread")]
async fn release_refuses_active_or_inconsistent_source_without_changing_warm_ownership() {
    let product=Product::new(&config()).await;
    let other=Product::new(&config()).await;
    let result=std::panic::AssertUnwindSafe(async {
        let original=product.row(&product.product).await.unwrap();let holds=product.holds().await;
        assert!(control::release_retained_product(product.client.clone(),product.request,product.source.attempt(),&product.budget).await.is_err());
        assert_eq!(product.row(&product.product).await,Some(original.clone()));assert_eq!(product.holds().await,holds);
        product.source.abandon().await.unwrap();
        // An exact product address cannot nominate another producer's contribution.
        product.client.query("UPDATE $product SET contribution=$other RETURN NONE").bind(("product",product.product.clone())).bind(("other",other.contribution.clone())).await.unwrap().check().unwrap();
        let wrong=control::release_retained_product(product.client.clone(),product.request,product.source.attempt(),&product.budget).await;
        product.client.query("UPDATE $product SET contribution=$original RETURN NONE").bind(("product",product.product.clone())).bind(("original",product.contribution.clone())).await.unwrap().check().unwrap();
        assert!(wrong.is_err());assert_eq!(product.row(&product.product).await,Some(original.clone()));assert_eq!(product.holds().await,holds);
        product.client.query("UPDATE $product SET request=$different RETURN NONE").bind(("product",product.product.clone())).bind(("different",other.request.hex())).await.unwrap().check().unwrap();
        let stale=control::release_retained_product(product.client.clone(),product.request,product.source.attempt(),&product.budget).await;
        product.client.query("UPDATE $product SET request=$original RETURN NONE").bind(("product",product.product.clone())).bind(("original",product.request.hex())).await.unwrap().check().unwrap();
        assert!(stale.is_err());assert_eq!(product.row(&product.product).await,Some(original));assert_eq!(product.holds().await,holds);
        let guard_id=RecordId::new("native_guard",ContentHash::of(&serde_json::to_vec(&Value::RecordId(product.product.clone())).unwrap()).hex());
        let guard=product.row(&guard_id).await.unwrap();
        product.client.query("UPDATE $guard SET phase='retiring' RETURN NONE").bind(("guard",guard_id.clone())).await.unwrap().check().unwrap();
        let retiring=control::release_retained_product(product.client.clone(),product.request,product.source.attempt(),&product.budget).await;
        product.client.query("UPDATE $guard CONTENT $original RETURN NONE").bind(("guard",guard_id)).bind(("original",guard)).await.unwrap().check().unwrap();
        assert!(retiring.is_err(),"a nonactive exact product cannot earn a new release nomination");assert_eq!(product.holds().await,holds);
    }).catch_unwind().await;
    other.finish(Ok(())).await;product.finish(result).await;
}

#[tokio::test(flavor="multi_thread")]
async fn released_product_collection_preserves_an_independent_reader_and_shared_root() {
    let product=Product::new(&config()).await;
    let pin=control::ReaderPin::acquire(product.client.clone(),&[]).await.unwrap();
    let result=std::panic::AssertUnwindSafe(async {
        pin.protect(product.contribution.clone()).await.unwrap();
        let contributor=product.row(&product.contribution).await.unwrap();
        let mut response=product.client.query("SELECT semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$contribution ORDER BY semantic_key")
            .bind(("contribution",product.contribution.clone())).await.unwrap().check().unwrap();let before:Vec<Object>=response.take(0).unwrap();assert_eq!(before.len(),1);
        product.source.abandon().await.unwrap();let job=product.release().await;
        let progress=control::resume_retirement(&product.client,job,128).await.unwrap();
        assert!(product.row(&product.product).await.is_none(),"collector can end the released warm owner");
        assert_eq!(product.row(&product.contribution).await,Some(contributor),"reader incidence preserves the shared exact physical contributor");
        assert!(!progress.retained.is_empty());
        let mut response=product.client.query("SELECT semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$contribution ORDER BY semantic_key")
            .bind(("contribution",product.contribution.clone())).await.unwrap().check().unwrap();let after:Vec<Object>=response.take(0).unwrap();assert_eq!(after,before);
        for row in &after {let Some(Value::RecordId(node))=row.get("node") else{panic!("native backing identity")};assert!(product.row(node).await.is_some(),"shared canonical backing remains readable");}
    }).catch_unwind().await;
    pin.release().await.unwrap();product.finish(result).await;
}

#[tokio::test(flavor="multi_thread")]
async fn frozen_warm_only_release_requires_real_physical_completion_and_preserves_shared_owners() {
    let product=Product::new(&config()).await;
    let unearned=Product::new(&config()).await;
    let result=std::panic::AssertUnwindSafe(async {
        // A terminal spelling without earned completion cannot supply release authority.
        let source=RecordId::new("native_attempt",unearned.source.attempt().hex());
        let original=unearned.row(&source).await.unwrap();let unearned_holds=unearned.holds().await;
        unearned.client.query("UPDATE $source SET state='frozen' RETURN NONE").bind(("source",source.clone())).await.unwrap().check().unwrap();
        let refused=control::release_retained_product(unearned.client.clone(),unearned.request,unearned.source.attempt(),&unearned.budget).await;
        unearned.client.query("UPDATE $source CONTENT $original RETURN NONE").bind(("source",source)).bind(("original",original)).await.unwrap().check().unwrap();
        assert!(refused.is_err());assert_eq!(unearned.holds().await,unearned_holds);
        assert_eq!(unearned.row(&unearned.product).await.unwrap().get("released"),Some(&Value::Bool(false)));

        let product_holds=product.holds().await;let mut original_singletons=Vec::new();
        for edge in &product_holds {
            if let Some(Value::RecordId(record))=edge.get("object") {
                if record.table.as_str()=="compiler_view_completion" {
                    original_singletons.push((record.clone(),product.row(record).await.unwrap()));
                }
            }
        }
        assert!(!original_singletons.is_empty(),"production retains its original sealed singleton witness");
        product.source.freeze_content(&product.budget).await.unwrap();
        let physical=product.source.complete_physical_content(&product.budget).await.unwrap();
        for (record,original) in &original_singletons {
            assert_eq!(product.row(record).await.as_ref(),Some(original),"freeze reuses the original immutable seal despite later retention incidence");
        }
        let completed=physical.completion().record().unwrap();
        let completion_before=product.row(&completed).await.unwrap();
        let contributor_before=product.row(&product.contribution).await.unwrap();
        let holds_before=product.holds().await;
        let mut response=product.client.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$completed ORDER BY id")
            .bind(("completed",completed.clone())).await.unwrap().check().unwrap();let completed_holds:Vec<Object>=response.take(0).unwrap();
        assert!(!completed_holds.is_empty());
        let source=RecordId::new("native_attempt",product.source.attempt().hex());
        let source_before=product.row(&source).await.unwrap();
        assert_eq!(source_before.get("state"),Some(&Value::String("frozen".into())));
        assert_eq!(source_before.get("admitted"),Some(&Value::Bool(false)),"physical-only warm completion earns no artifact admission");
        let nomination=product.release().await;
        assert_eq!(product.release().await,nomination);
        assert_eq!(product.row(&completed).await,Some(completion_before));
        assert_eq!(product.row(&product.contribution).await,Some(contributor_before));
        assert_eq!(product.row(&source).await,Some(source_before));
        assert_eq!(product.holds().await,holds_before,"nomination does not drain product output owners");
        let mut response=product.client.query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$completed ORDER BY id")
            .bind(("completed",completed)).await.unwrap().check().unwrap();let after:Vec<Object>=response.take(0).unwrap();
        assert_eq!(after,completed_holds,"independently held physical completion keeps every selected singleton and binding incidence");
    }).catch_unwind().await;
    unearned.finish(Ok(())).await;product.finish(result).await;
}

#[tokio::test(flavor="multi_thread")]
#[ignore="requires explicit drained validation maintenance and independently qualified history inventory"]
async fn history_preserves_a_released_products_durable_nomination() {
    assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some());
    let cfg=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("maintenance installer"))).unwrap();
    assert_eq!(cfg.authentication,lctx_native::AuthenticationScope::Root);assert_eq!(cfg.database.as_str(),"validation");
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let mut response=client.query("SELECT admission_open,history_inventory FROM native_installation:current").await.unwrap().check().unwrap();
    let rows:Vec<Object>=response.take(0).unwrap();let [installation]=rows.as_slice() else{panic!("maintenance installation")};
    let Some(Value::Bool(was_open))=installation.get("admission_open") else{panic!("maintenance admission")};let was_open=*was_open;
    let evidence=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("independently qualified consumer inventory");
    assert_eq!(installation.get("history_inventory"),Some(&Value::String(evidence)),"this control never manufactures history deletion authority");
    if !was_open {lctx_compiler_store::compiler::open_admission(&cfg).await.unwrap();}
    let product=Product::new(&cfg).await;
    let result=std::panic::AssertUnwindSafe(async {
        product.source.abandon().await.unwrap();let release=product.release().await;
        let job=RecordId::new("native_retirement",release.hex());
        let issuance=control::IssuanceEra::capture(&client).await.unwrap();
        lctx_compiler_store::compiler::close_admission(&cfg).await.unwrap();
        lctx_compiler_store::compiler::drain_installation(&cfg).await.unwrap();
        control::cut_era(&client).await.unwrap();
        // A finite exact native checkpoint exercises production history eligibility,
        // without scanning or deleting unrelated retained history.
        let mut bytes=release.0;for byte in bytes.iter_mut().rev(){if *byte>0{*byte-=1;break;}*byte=255;}
        let checkpoint=control::fresh_identity("product-history-control").unwrap();
        let mut bindings=surrealdb::types::Variables::new();
        bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",checkpoint.hex()));
        bindings.insert("closed",issuance.era);bindings.insert("after",RecordId::new("native_retirement",ContentHash(bytes).hex()));
        control::effect(&client,None,"IF $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through<$closed { THROW 'product history requires qualified drained horizon'; }; CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=$__epoch,closed_through=$closed,phase='retirement',after=$after,state='open',removed_effects=0,removed_records=0,revision=0 RETURN NONE",bindings).await.unwrap();
        let receipt=control::resume_history_compaction(&client,checkpoint,0,1).await.unwrap();
        assert_eq!(receipt.examined,[job.clone()]);assert_eq!(receipt.protected,[job.clone()]);assert!(receipt.removed.is_empty());
        assert!(product.row(&job).await.is_some());assert!(product.row(&product.product).await.is_some());
        assert_eq!(product.release().await,release,"release remains discoverable after the history page");
    }).catch_unwind().await;
    let restore=if was_open {lctx_compiler_store::compiler::open_admission(&cfg).await}else{lctx_compiler_store::compiler::close_admission(&cfg).await};
    if let Err(error)=&restore{eprintln!("product history admission restoration: {error:#?}");}
    client.invalidate().await.unwrap();product.finish(result).await;restore.unwrap();
}

// A one-record production page keeps this maintenance control inside its exact
// owned scope. The qualified inventory and drained horizon remain prerequisites.
async fn exact_history_page(client:&Surreal<Client>,closed:i64,phase:&str,target:&RecordId)->control::HistoryCompaction {
    let surrealdb::types::RecordIdKey::String(key)=&target.key else{panic!("history target key")};
    let mut before: [u8;32]=hex::decode(key).unwrap().try_into().unwrap();
    for byte in before.iter_mut().rev(){if *byte>0{*byte-=1;break;}*byte=255;}
    let checkpoint=control::fresh_identity("binding-history-control").unwrap();
    let mut bindings=surrealdb::types::Variables::new();
    bindings.insert("checkpoint",RecordId::new("native_history_checkpoint",checkpoint.hex()));
    bindings.insert("closed",closed);bindings.insert("phase",phase.to_owned());
    bindings.insert("after",RecordId::new(target.table.as_str(),ContentHash(before).hex()));
    control::effect(client,None,"IF $__installation.admission_open OR $__installation.history_inventory=NONE OR $__installation.closed_through<$closed { THROW 'binding history requires qualified drained horizon'; }; CREATE $checkpoint SET generation=$__generation,era=$__era,epoch=$__epoch,closed_through=$closed,phase=$phase,after=$after,state='open',removed_effects=0,removed_records=0,revision=0 RETURN NONE",bindings).await.unwrap();
    let receipt=control::resume_history_compaction(client,checkpoint,0,1).await.unwrap();
    assert_eq!(receipt.examined,[target.clone()],"the page cannot consume another test's history");receipt
}

#[tokio::test(flavor="multi_thread")]
#[ignore="requires explicit drained validation maintenance and independently qualified history inventory"]
async fn released_binding_slots_allow_exact_terminal_attempt_history_compaction() {
    assert!(std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some());
    let cfg=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("maintenance installer"))).unwrap();
    assert_eq!(cfg.authentication,lctx_native::AuthenticationScope::Root);assert_eq!(cfg.database.as_str(),"validation");
    let client=lctx_compiler_store::compiler::check_installation(&cfg).await.unwrap();
    let mut response=client.query("SELECT admission_open,history_inventory FROM native_installation:current").await.unwrap().check().unwrap();
    let rows:Vec<Object>=response.take(0).unwrap();let [installation]=rows.as_slice() else{panic!("maintenance installation")};
    let Some(Value::Bool(was_open))=installation.get("admission_open") else{panic!("maintenance admission")};let was_open=*was_open;
    let evidence=std::env::var("LCTX_HISTORY_INVENTORY_EVIDENCE").expect("independently qualified consumer inventory");
    assert_eq!(installation.get("history_inventory"),Some(&Value::String(evidence)));
    if !was_open {lctx_compiler_store::compiler::open_admission(&cfg).await.unwrap();}
    let product=Product::new(&cfg).await;
    let peer=Product::new(&cfg).await;
    let result=std::panic::AssertUnwindSafe(async {
        let (_,slot)=product.bind().await;let (_,peer_slot)=peer.bind().await;
        let peer_before=peer.row(&peer_slot).await.unwrap();
        let source=RecordId::new("native_attempt",product.source.attempt().hex());
        let mut response=client.query("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$source ORDER BY object")
            .bind(("source",source.clone())).await.unwrap().check().unwrap();
        let mut roots:Vec<RecordId>=response.take(0).unwrap();assert!(roots.contains(&slot));
        product.source.abandon().await.unwrap();peer.source.abandon().await.unwrap();
        product.release().await;roots.push(product.product.clone());
        // The collector consumes only this producer's captured roots. Warm peers
        // remain explicit owners and are not nominated by this operation.
        let mut progress=control::retire_reachable(&client,roots,4096).await.unwrap();
        // A parent visited later in a pass can release a previously retained
        // child. Revisit this exact bounded fixture graph, never another owner.
        for _ in 0..8 {
            if progress.remaining.is_empty()&&progress.retained.is_empty(){break;}
            progress=control::resume_retirement(&client,progress.identity,4096).await.unwrap();
        }
        assert!(progress.remaining.is_empty()&&progress.retained.is_empty());
        assert!(product.row(&slot).await.is_none());assert!(product.row(&product.contribution).await.is_none());
        assert_eq!(peer.row(&peer_slot).await,Some(peer_before));
        let issuance=control::IssuanceEra::capture(&client).await.unwrap();
        lctx_compiler_store::compiler::close_admission(&cfg).await.unwrap();
        lctx_compiler_store::compiler::drain_installation(&cfg).await.unwrap();control::cut_era(&client).await.unwrap();
        for (phase,sql) in [("effect","SELECT VALUE id FROM native_effect WITH INDEX attempt_effects WHERE attempt=$source ORDER BY id"),
            ("cleanup","SELECT VALUE id FROM native_cleanup WITH INDEX attempt_cleanup WHERE attempt=$source ORDER BY id")] {
            let mut response=client.query(sql).bind(("source",source.clone())).await.unwrap().check().unwrap();
            let ids:Vec<RecordId>=response.take(0).unwrap();
            for id in ids {let receipt=exact_history_page(&client,issuance.era,phase,&id).await;assert_eq!(receipt.removed,[id]);}
        }
        let receipt=exact_history_page(&client,issuance.era,"attempt",&source).await;
        assert_eq!(receipt.removed,[source]);assert!(product.row(&RecordId::new("native_attempt",product.source.attempt().hex())).await.is_none());
        assert!(peer.row(&RecordId::new("native_attempt",peer.source.attempt().hex())).await.is_some());
        assert!(peer.row(&peer.product).await.is_some(),"the peer's warm product remains owned");
    }).catch_unwind().await;
    let restore=if was_open {lctx_compiler_store::compiler::open_admission(&cfg).await}else{lctx_compiler_store::compiler::close_admission(&cfg).await};
    let mut terminal=Completion::default();
    for owner in [&product,&peer] {
        if let Some(source)=owner.row(&RecordId::new("native_attempt",owner.source.attempt().hex())).await {
            // Ended old-era producers must not issue a fresh terminal-attempt
            // operation after the cut. Their explicit warm release remains owned.
            if !matches!(source.get("state"),Some(Value::String(state))if matches!(state.as_str(),"closed"|"abandoned"|"maintenance_fenced")) {
                terminal.step("binding history active source close",owner.source.abandon().await);
            }
            terminal.step("binding history warm release",control::release_retained_product(owner.client.clone(),owner.request,owner.source.attempt(),&owner.budget).await.map(|_|()));
        }
    }
    terminal.step("binding history source observer close",product.client.invalidate().await.map_err(ModelError::codec));
    terminal.step("binding history peer observer close",peer.client.invalidate().await.map_err(ModelError::codec));
    terminal.step("binding history maintenance observer close",client.invalidate().await.map_err(ModelError::codec));
    terminal.step("binding history admission restoration",restore);
    let cleanup=complete(Ok(()),terminal);
    if let Err(primary)=result{if let Err(error)=cleanup{eprintln!("binding history cleanup: {error:#?}");}std::panic::resume_unwind(primary);}cleanup.unwrap();
}
