//! Whole-input N11 reuse within the exact current native validity scope.
use super::*;
use lctx_model::domain::{self as d,checked_admission::FixedValidationDriver};
use lctx_native::control::validation_premise::{PendingValidation,SelectedInput};
impl AdmissionRuntime {
    // Called only after every specialized scope branch has selected its genuine
    // grouped kernel. Unsupported premises still execute that ordinary kernel.
    pub(super) fn validate_durable_whole<'a>(&'a self,invariant:&'a d::Invariant,inputs:&'a ValidationInputs,
        unrequested:&'a BTreeSet<&'static str>,profile:Profile,uniform:&'a [d::uniform_outcome::ValidatedUniformOwner],
        session:&'a SessionContext,planner:&'a dyn ValidationPlanner,
    )->BoxFuture<'a,Result<bool,ModelError>>{Box::pin(async move {
        if self.model.factory_contract(invariant.identity()).is_none() || invariant.inputs.is_empty() || !uniform.is_empty()
            || !inputs.companions.is_empty() || invariant.inputs.iter().any(|input|unrequested.contains(input.name())
                || input.name()==d::artifact::ArtifactChunk::NAME || inputs.validation_source(input).is_err()){
            return Ok(false);
        }
        let mut sources=Vec::with_capacity(invariant.inputs.len());
        for input in &invariant.inputs {
            let selected=inputs.validation_source(input)?;
            sources.push(SelectedInput{source:selected.source.snapshot(),requested:selected.requested,resolved:selected.resolved});
        }
        // Protect only actual declared roles. Compact owners require an exhaustive
        // parent premise not yet carried by this whole-input slice; they use the
        // ordinary shared validator instead of broad unrelated selection keys.
        let mut caps=BTreeMap::new();
        for input in &invariant.inputs {
            let selected=inputs.validation_source(input)?;
            if caps.contains_key(&selected.view.identity){continue;}
            let native=self.native.clone();let view=selected.view.clone();let budget=self.budget.clone();
            let cap=self.native_call(async move{native.installed_view(&view,&budget).await}).await?;
            caps.insert(view_identity(&cap),cap);
        }
        if uniform.iter().any(|owner|!caps.values().any(|cap|cap.completion().mappings.iter().any(|mapping|mapping.logical==owner.contribution()))){return Ok(false);}
        let mut driver=FixedValidationDriver::invariant(&self.model,invariant.identity(),sources.iter().map(|input|(input.source.clone(),input.requested,input.resolved)).collect(),uniform,self.budget())?;
        let mut pending=PendingValidation::prepare(self.model.clone(),invariant,profile,sources,caps.into_values().collect(),uniform,self.budget()).await?;
        let result:Result<(),ModelError>=async {
            if pending.lookup().await?{
                tracing::debug!(invariant=invariant.name,"reused exact durable native validation premise");
                return Ok(());
            }
            for (index,input) in invariant.inputs.iter().enumerate(){
                let order=input.order().iter().map(|name|crate::consumed_rows::identifier(name)).collect::<Vec<_>>().join(",");
                let table=inputs.validation_table(input)?;
                let sql=format!("SELECT * FROM {}{}",crate::consumed_rows::identifier(&table),if order.is_empty(){String::new()}else{format!(" ORDER BY {order}")});
                let mut stream=planner.query(session,input,&sql).await?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{
                    self.cancellation.check()?;
                    pending.visit(index,&batch).await?;
                    driver.visit(index,&batch)?;
                }
                pending.end_input(index)?;driver.end_input(index)?;
            }
            let proof=driver.finish()?;
            pending.persist(&proof).await?;
            tracing::debug!(invariant=invariant.name,"persisted executed native validation premise");
            Ok(())
        }.await;
        if result.as_ref().err().is_some_and(|error|!error.permits_storage_cleanup()){
            // PendingValidation owns the exact reader and physical caps; its drop
            // preserves them on unknown execution instead of attempting cleanup.
            return result.map(|_|true);
        }
        let mut terminal=d::completion::Completion::default();terminal.step("durable validation native scope close",pending.close().await);
        d::completion::complete(result,terminal).map(|_|true)
    })}
}
fn view_identity(cap:&lctx_native::installed_view::HeldInstalledView)->ContentHash{cap.completion().view.identity}

#[cfg(test)]
mod controls {
    use super::*;
    use d::{Key,KeySink,Invariant,InvariantCheck,InvariantPurpose,ValidationDefinitions,ValidationInput,
        input::InputRevision,completed::{ContributionSpec,OutputProfile},producer_contract::request::CapturedRequest,
        admission::Frontier,stages::ProviderOutcome,completion::{Completion,complete}};
    use std::sync::atomic::{AtomicUsize,Ordering};
    static FINISHED:AtomicUsize=AtomicUsize::new(0);
    // Native ingress accepts the canonical record's TypeId and schema only.
    // Its existing relation-owned reference selects this fixture-local whole-input
    // check; registering an unreferenced factory alone would exercise no reuse.
    // This fixture checks an empty input revision, not full artifact admission.
    struct Check { rows:usize }
    impl InvariantCheck for Check {
        fn visit(&mut self,name:&str,batch:&RecordBatch)->Result<(),ModelError>{
            if name!=InputRevision::NAME{return Err(ModelError::Conflict("durable control exact input revision role"));}
            let expected=InputRevision::from_entries(vec![])?;
            for row in InputRevision::decode(batch)?{
                if row!=expected{return Err(ModelError::Conflict("durable control exact empty manifest"));}
                self.rows=self.rows.checked_add(1).ok_or(ModelError::Schema("durable control row count"))?;
            }Ok(())
        }
        fn finish(self:Box<Self>)->Result<(),ModelError>{
            if self.rows!=1{return Err(ModelError::Conflict("durable control exact input revision count"));}
            FINISHED.fetch_add(1,Ordering::SeqCst);Ok(())
        }
    }
    fn model(changed:bool)->Arc<ValidatedModel>{
        let invariant=Invariant{purpose:InvariantPurpose::Admission,revision:1,name:"input_manifest_membership",
            inputs:vec![ValidationInput::of::<InputRevision>(&["id"])],create:Arc::new(|_|Box::new(Check{rows:0}))};
        let identity=invariant.identity();
        let mut model=ValidatedModel::validate(vec![Relation::of::<InputRevision>()],ValidationDefinitions{invariants:vec![invariant],publication_checks:vec![]}).unwrap();
        // The counter observes execution only. It never changes the result, declared
        // input set or validator semantics. A separately declared implementation
        // must re-execute even though this semantic model digest remains unchanged.
        let mut implementation=KeySink::new("durable-candidate-control-executable/v1");
        implementation.part(b"source",include_bytes!("durable.rs"));implementation.part(b"revision",&[u8::from(changed)]);
        model.register_factory(identity,implementation.finish(),vec![]).unwrap();
        assert_eq!(model.admission_candidates_for_scope(&BTreeSet::from([InputRevision::NAME])).unwrap()
            .iter().map(Invariant::identity).collect::<Vec<_>>(),vec![identity],
            "the actual selected relation must require the counted factory");
        Arc::new(model)
    }
    #[derive(serde::Serialize,serde::Deserialize)]
    struct Seed {spec:ContributionSpec,attempt:ContentHash}
    fn request(spec:ContributionSpec,model:&ValidatedModel)->CapturedRequest{
        let mut contract=KeySink::new("compiler-product-row-contract/v1");
        for name in &spec.outputs{name.encode(&mut contract);model.digest().encode(&mut contract);}
        CapturedRequest::new(spec,contract.finish(),vec![]).unwrap()
    }
    fn config()->lctx_native::RuntimeConfig{
        lctx_native::RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("installed current native validation fixture"))).unwrap()
    }
    /// Invoked only by the enclosing real fixture test, in a fresh OS process.
    #[tokio::test(flavor="multi_thread")]
    async fn durable_candidate_process(){
        let Some(path)=std::env::var_os("LCTX_DURABLE_CANDIDATE_SEED")else{return;};
        let seed:Seed=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let changed=std::env::var("LCTX_DURABLE_CHANGED_FACTORY").unwrap()=="true";
        let expected:usize=std::env::var("LCTX_DURABLE_EXPECTED_EXECUTIONS").unwrap().parse().unwrap();
        let abandon_first=std::env::var("LCTX_DURABLE_ABANDON_RECEIVER_FIRST").unwrap()=="true";
        let result_path=std::env::var_os("LCTX_DURABLE_CHILD_RESULT").expect("owned result path");
        let nonce=std::env::var("LCTX_DURABLE_CHILD_NONCE").expect("owned child nonce");
        let model=model(changed);let request=request(seed.spec,&model);let cfg=config();
        let receiver=NativeCompilerStore::begin(&cfg,Frontier::Facts).await.unwrap();
        let client=crate::compiler::check_installation(&cfg).await.unwrap();
        let budget=ResourceBudget::fixed(256<<20).unwrap();
        let mut executed=None;
        let result=std::panic::AssertUnwindSafe(async {
            FINISHED.store(0,Ordering::SeqCst);
            let candidate=receiver.hold_captured_product(&request,model,Profile::Catalog,&budget).await.unwrap().expect("actual current terminal candidate admission");
            let finished=FINISHED.load(Ordering::SeqCst);
            executed=Some(finished);
            if abandon_first {
                receiver.abandon().await.unwrap();
                candidate.close().await.unwrap();
            }else{receiver.attach_held_product(candidate,&budget).await.unwrap();}
            assert_eq!(finished,expected,"fresh process must execute on miss and skip only exact protected durable success");
        }).catch_unwind().await;
        let mut terminal=Completion::default();
        terminal.step("durable child receiver close",receiver.abandon().await);
        terminal.step("durable child observer close",client.invalidate().await.map_err(ModelError::codec));
        let cleanup=complete(Ok(()),terminal);
        if let Err(panic)=result{if let Err(error)=cleanup{eprintln!("durable child cleanup: {error:#?}");}std::panic::resume_unwind(panic);}cleanup.unwrap();
        // A successful zero-test selection cannot manufacture this result. It is
        // written only after the actual candidate and all cleanup joins complete.
        use std::io::Write;
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(result_path).unwrap();
        file.write_all(&serde_json::to_vec(&serde_json::json!({"nonce":nonce,"process":std::process::id(),
            "request":request.identity().unwrap().hex(),"changed":changed,"abandon_first":abandon_first,"executed":executed.unwrap()})).unwrap()).unwrap();
        file.sync_all().unwrap();
    }
    #[tokio::test(flavor="multi_thread")]
    async fn durable_candidate_reuses_across_processes_and_reexecutes_changed_factory(){
        let cfg=config();let model=model(false);let budget=ResourceBudget::fixed(256<<20).unwrap();
        let nonce=lctx_native::control::fresh_identity("durable-candidate-native-control").unwrap();
        let spec=ContributionSpec{captured_binding:None,producer:format!("durable-candidate-{}",nonce.hex()),profile:OutputProfile::Specific(Profile::Catalog),
            model:model.digest(),implementation:ContentHash::of(b"actual current durable test producer/v1"),configuration:None,inputs:vec![],outputs:[InputRevision::NAME.into()].into()};
        let request=request(spec.clone(),&model);
        let source=NativeCompilerStore::begin(&cfg,Frontier::Facts).await.unwrap();
        let client=crate::compiler::check_installation(&cfg).await.unwrap();
        let directory=tempfile::tempdir().unwrap();let seed_path=directory.path().join("seed.json");
        let result=std::panic::AssertUnwindSafe(async {
            let id=source.begin_contribution(spec.clone()).await.unwrap();
            let relation=Relation::of::<InputRevision>();
            source.write_batch(&id,&relation,&InputRevision::encode(&[InputRevision::from_entries(vec![]).unwrap()]).unwrap(),&budget).await.unwrap();
            source.complete_contribution(id,ProviderOutcome::Complete,&[relation],&BTreeMap::new(),&budget).await.unwrap();
            source.retain_product_identity(request.identity().unwrap(),id).await.unwrap();
            source.abandon().await.unwrap();
            std::fs::write(&seed_path,serde_json::to_vec(&Seed{spec,attempt:source.attempt()}).unwrap()).unwrap();
            for (changed,count,abandon_first) in [(false,1usize,false),(false,0,false),(true,1,false),(true,0,false),(true,0,true)]{
                let child_nonce=lctx_native::control::fresh_identity("durable-candidate-child").unwrap().hex();
                let result_path=directory.path().join(format!("{child_nonce}.json"));
                let child=std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact","admission::durable::controls::durable_candidate_process","--nocapture"])
                    .env("LCTX_DURABLE_CANDIDATE_SEED",&seed_path)
                    .env("LCTX_DURABLE_CHANGED_FACTORY",changed.to_string())
                    .env("LCTX_DURABLE_EXPECTED_EXECUTIONS",count.to_string())
                    .env("LCTX_DURABLE_ABANDON_RECEIVER_FIRST",abandon_first.to_string())
                    .env("LCTX_DURABLE_CHILD_NONCE",&child_nonce).env("LCTX_DURABLE_CHILD_RESULT",&result_path)
                    .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
                let child_pid=child.id();let output=child.wait_with_output().unwrap();
                assert!(output.status.success(),"actual fresh candidate process failed: {}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
                let result:serde_json::Value=serde_json::from_slice(&std::fs::read(result_path).expect("selected child actually executed and joined cleanup")).unwrap();
                assert_eq!(result,serde_json::json!({"nonce":child_nonce,"process":child_pid,"request":request.identity().unwrap().hex(),
                    "changed":changed,"abandon_first":abandon_first,"executed":count}));
            }
        }).catch_unwind().await;
        let mut terminal=Completion::default();
        terminal.step("durable control source close",source.abandon().await);
        terminal.step("durable control warm product release",lctx_native::control::release_retained_product(client.clone(),request.identity().unwrap(),source.attempt(),&budget).await.map(|_|()));
        terminal.step("durable control observer close",client.invalidate().await.map_err(ModelError::codec));
        let cleanup=complete(Ok(()),terminal);
        if let Err(panic)=result{if let Err(error)=cleanup{eprintln!("durable control cleanup: {error:#?}");}std::panic::resume_unwind(panic);}cleanup.unwrap();
    }
}
