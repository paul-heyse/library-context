//! Pure capture/request controls and the real acquisition provider over owned deterministic files.
use cpg_extract::{acquisition::{AcquiredInput, Acquisition, Acquire, InventoryDistribution, InventoryFile, LibraryInventory}, bundle::{CapturedInputs, Declared, ProviderSink}, capture::CapturedInput, native_context::NativeContextConfig};
use lctx_model::domain::{*, input::*, resources::ResourceBudget, stages::*, producer_contract::{request::{CapturedRequest, Candidate, Eligibility}}, completed::{ContributionSpec, CompletedContribution, OutputContent}, compilation_product::{DependencyToken, DependencyKind}};
use std::{collections::{BTreeMap,BTreeSet},sync::{Arc,Mutex}};

fn acquired(bytes:&[u8],label:&str,budget:&ResourceBudget)->AcquiredInput {
    let dir=tempfile::tempdir().unwrap();std::fs::write(dir.path().join("api.py"),bytes).unwrap();
    AcquiredInput::tree(CapturedInput::capture(dir.path(),&["api.py".into()],budget).unwrap(),label)
}
fn captured(inputs:Vec<AcquiredInput>,budget:&ResourceBudget)->CapturedInputs {
    CapturedInputs::new(inputs,NativeContextConfig::committed(Profile::Catalog,budget).unwrap())
}
fn request(inputs:&CapturedInputs)->CapturedRequest {
    let mut stage=Acquire::new(ContentHash::of(b"settings")).declaration(Profile::Catalog);
    stage.captured_binding.as_mut().unwrap().sources=cpg_extract::contracts::captured_sources(inputs).unwrap();
    CapturedRequest::new(ContributionSpec {captured_binding:stage.captured_binding,producer:stage.name.into(),profile:Profile::Catalog,
        model:ContentHash::of(b"model"),implementation:stage.code,configuration:Some(stage.configuration),inputs:vec![],
        outputs:stage.outputs.iter().chain(&stage.contributes).map(|r|r.name().into()).collect()},ContentHash::of(b"output contract"),vec![DependencyToken {kind:DependencyKind::ExactView,role:"imports".into(),relation:"source_artifacts".into(),prefix:None,identity:ContentHash::of(b"exact dependency")}]).unwrap()
}
fn installed(budget:&ResourceBudget,record:&[u8],role:SourceRole)->AcquiredInput {
    // The selected source bytes stay identical; attribution and RECORD evidence vary.
    let dir=tempfile::tempdir().unwrap();std::fs::write(dir.path().join("api.py"),b"VALUE = 1\n").unwrap();
    let frozen=CapturedInput::capture(dir.path(),&["api.py".into()],budget).unwrap();
    AcquiredInput::new(frozen,Acquisition::Installed(LibraryInventory {name:"demo".into(),requirement:"demo==1".into(),lock_digest:ContentHash::of(b"lock"),installer:Some("uv pinned".into()),python_version:"3.14.7".into(),platform:"linux".into(),site_packages:dir.path().into(),configuration:ContentHash::of(b"configuration"),
        distributions:vec![InventoryDistribution {name:"demo".into(),version:"1".into(),first_party:true,artifact_sha256:vec!["a".repeat(64)],record_digest:ContentHash::of(record)}],
        files:vec![InventoryFile {path:"api.py".into(),owners:vec!["demo".into()],role,record_sha256:Some("verified-record-sha256".into())}]}))
}
#[test]
fn complete_request_distinguishes_acquisition_metadata_order_and_negative_premises() {
    let budget=ResourceBudget::fixed(256<<20).unwrap();
    let a=captured(vec![acquired(b"VALUE = 1\n","first",&budget)],&budget);
    let repeat=captured(vec![acquired(b"VALUE = 1\n","first",&budget)],&budget);
    let origin=captured(vec![acquired(b"VALUE = 1\n","second",&budget)],&budget);
    let original=request(&a);
    assert_eq!(original,request(&repeat)); // physical relocation is irrelevant
    assert_eq!(a.inputs()[0].captured().revision(),origin.inputs()[0].captured().revision());
    assert_ne!(original.identity().unwrap(),request(&origin).identity().unwrap());
    let changed=captured(vec![acquired(b"VALUE = 2\n","first",&budget)],&budget);
    assert_ne!(original.identity().unwrap(),request(&changed).identity().unwrap());
    let absent=captured(vec![acquired(b"VALUE = 1\n","first",&budget).with_selected_absent(["optional.py".into()]).unwrap()],&budget);
    assert_ne!(original.identity().unwrap(),request(&absent).identity().unwrap());
    assert!(acquired(b"VALUE = 1\n","first",&budget).with_selected_absent(["api.py".into()]).is_err());
    let ordered=captured(vec![acquired(b"VALUE = 1\n","first",&budget),acquired(b"VALUE = 1\n","second",&budget)],&budget);
    let reversed=captured(vec![acquired(b"VALUE = 1\n","second",&budget),acquired(b"VALUE = 1\n","first",&budget)],&budget);
    assert_eq!(request(&ordered).contribution().captured_binding.as_ref().unwrap().sources.len(),2);
    assert_ne!(request(&ordered).identity().unwrap(),request(&reversed).identity().unwrap());
    let baseline=captured(vec![installed(&budget,b"RECORD one",SourceRole::Release)],&budget);
    let record=captured(vec![installed(&budget,b"RECORD two",SourceRole::Release)],&budget);
    let role=captured(vec![installed(&budget,b"RECORD one",SourceRole::Dependency)],&budget);
    assert_eq!(baseline.inputs()[0].captured().revision(),record.inputs()[0].captured().revision());
    assert_eq!(baseline.inputs()[0].captured().revision(),role.inputs()[0].captured().revision());
    assert_ne!(request(&baseline).identity().unwrap(),request(&record).identity().unwrap());
    assert_ne!(request(&baseline).identity().unwrap(),request(&role).identity().unwrap());
    let mut mutations=vec![];
    let mut spec=original.contribution().clone();spec.implementation=ContentHash::of(b"new implementation");mutations.push((spec,original.product().dependencies.clone()));
    let mut spec=original.contribution().clone();spec.configuration=Some(ContentHash::of(b"new settings"));spec.captured_binding.as_mut().unwrap().producer_settings=spec.configuration.unwrap();mutations.push((spec,original.product().dependencies.clone()));
    let mut dependencies=original.product().dependencies.clone();dependencies[0].identity=ContentHash::of(b"changed upstream");mutations.push((original.contribution().clone(),dependencies));
    for (spec,dependencies) in mutations {
        let rebuilt=CapturedRequest::new(spec,original.product().result_contract,dependencies).unwrap();
        assert_ne!(original.identity().unwrap(),rebuilt.identity().unwrap());
    }
}
#[derive(Default)]
struct Sink {declared:Mutex<BTreeSet<String>>,rows:Mutex<BTreeMap<String,Vec<ContentHash>>>,outcomes:Mutex<Vec<ProviderOutcome>>}
impl ProviderSink for Sink {
    fn read<R:Record>(&self)->Result<cpg_extract::bundle::ProviderBatchStream<R>,ModelError>{Err(ModelError::Invalid("acquisition has no upstream reads".into()))}
    fn declare<R:Record>(&self)->Result<(),ModelError>{self.declared.lock().unwrap().insert(R::NAME.into());self.rows.lock().unwrap().entry(R::NAME.into()).or_default();Ok(())}
    fn write<R:Record>(&self,batch:Batch<R>)->Result<(),ModelError>{self.rows.lock().unwrap().entry(R::NAME.into()).or_default().extend(batch.rows().iter().map(Record::content_digest));Ok(())}
    fn contribute<R:Record>(&self,batch:Batch<R>)->Result<(),ModelError>{self.write(batch)}
    fn finish(&self,outcome:ProviderOutcome)->Result<(),ModelError>{self.outcomes.lock().unwrap().push(outcome);Ok(())}
}
#[tokio::test]
async fn real_acquisition_emits_the_bound_inventory_and_explicit_empty_outputs() {
    let budget=ResourceBudget::fixed(256<<20).unwrap();let inputs=Arc::new(captured(vec![installed(&budget,b"RECORD",SourceRole::Release)],&budget));
    let expected=request(&inputs);let descriptor=inputs.inputs()[0].descriptor(None).unwrap();let sink=Arc::new(Sink::default());
    cpg_extract::bundle::run_provider(Box::new(Acquire::new(ContentHash::of(b"settings"))),Profile::Catalog,sink.clone(),Arc::new(model().unwrap()),inputs,budget,Default::default()).await.unwrap();
    assert_eq!(*sink.outcomes.lock().unwrap(),vec![ProviderOutcome::Complete]);
    assert_eq!(*sink.declared.lock().unwrap(),expected.contribution().outputs.clone());
    let rows=sink.rows.lock().unwrap();
    macro_rules! same {($($field:ident : $ty:ty),*)=>{$(assert_eq!(rows[<$ty>::NAME],descriptor.$field.iter().map(Record::content_digest).collect::<Vec<_>>());)*};}
    same!(artifacts:source::SourceArtifact,packages:Package,releases:Release,distributions:InputDistribution,verifications:DistributionVerification,fingerprints:EnvironmentFingerprint,ownership:ArtifactOwnership,unowned:UnownedArtifact,derivations:DerivedArtifact,uses:ArtifactUse);
    assert!(rows[DerivedArtifact::NAME].is_empty());
    assert!(rows[CorpusLibrary::NAME].is_empty());
    let completed=CompletedContribution {spec:expected.contribution().clone(),outcome:0,outputs:rows.iter().map(|(name,values)|{
        let mut digest=KeySink::new("focused-output");for value in values {digest.part(b"row",&value.0);}(name.clone(),OutputContent{rows:values.len() as u64,content:digest.finish()})
    }).collect()};
    assert!(matches!(expected.eligibility(Candidate::Completed {request:expected.identity().unwrap(),descriptor:&completed,admitted:true}),Eligibility::Hit));
    let mut missing=completed.clone();missing.outputs.remove(DerivedArtifact::NAME);
    assert!(matches!(expected.eligibility(Candidate::Completed {request:expected.identity().unwrap(),descriptor:&missing,admitted:true}),Eligibility::Corrupt(_)));
}
#[test]
fn terminal_eligibility_is_total_and_never_promotes_partial_or_unrequested_results() {
    let budget=ResourceBudget::fixed(64<<20).unwrap();let input=captured(vec![acquired(b"","empty",&budget)],&budget);let request=request(&input);
    let complete=CompletedContribution {spec:request.contribution().clone(),outcome:0,outputs:request.contribution().outputs.iter().map(|name|(name.clone(),OutputContent{rows:0,content:ContentHash::of(b"empty")})).collect()};
    let key=request.identity().unwrap();
    assert!(matches!(request.eligibility(Candidate::Completed {request:key,descriptor:&complete,admitted:true}),Eligibility::Hit));
    assert!(matches!(request.eligibility(Candidate::Completed {request:key,descriptor:&complete,admitted:false}),Eligibility::Miss(_)));
    for state in [Candidate::Unfinished,Candidate::Abandoned,Candidate::Poisoned,Candidate::ResourcePartial] {assert!(matches!(request.eligibility(state),Eligibility::Miss(_)));}
    for outcome in 1..=4 {let mut value=complete.clone();value.outcome=outcome;assert!(matches!(request.eligibility(Candidate::Completed {request:key,descriptor:&value,admitted:true}),Eligibility::Miss(_)));}
    let mut bad=complete.clone();bad.spec.implementation=ContentHash::of(b"other impl");
    assert!(matches!(request.eligibility(Candidate::Completed {request:key,descriptor:&bad,admitted:true}),Eligibility::Corrupt(_)));
    assert!(matches!(request.eligibility(Candidate::Completed {request:ContentHash::of(b"other key"),descriptor:&bad,admitted:true}),Eligibility::Miss(_)));
    let mut bad=complete;bad.outcome=9;
    assert!(matches!(request.eligibility(Candidate::Completed {request:key,descriptor:&bad,admitted:true}),Eligibility::Corrupt(_)));
}

#[test]
fn membership_runtime_ownership_distribution_configuration_and_derivations_are_request_inputs() {
    let budget=ResourceBudget::fixed(256<<20).unwrap();
    let base=captured(vec![installed(&budget,b"RECORD",SourceRole::Release)],&budget);let key=request(&base).identity().unwrap();
    let Acquisition::Installed(inventory)=base.inputs()[0].acquisition() else {unreachable!()};
    let recapture=|inventory:LibraryInventory| {
        let dir=tempfile::tempdir().unwrap();std::fs::write(dir.path().join("api.py"),b"VALUE = 1\n").unwrap();
        captured(vec![AcquiredInput::new(CapturedInput::capture(dir.path(),&["api.py".into()],&budget).unwrap(),Acquisition::Installed(inventory))],&budget)
    };
    let mut mutations=vec![];
    let mut changed=inventory.clone();changed.python_version="3.14.8".into();mutations.push(changed);
    let mut changed=inventory.clone();changed.platform="win32".into();mutations.push(changed);
    let mut changed=inventory.clone();changed.configuration=ContentHash::of(b"changed selection configuration");mutations.push(changed);
    let mut changed=inventory.clone();changed.files[0].owners.clear();mutations.push(changed);
    let mut changed=inventory.clone();changed.distributions[0].first_party=false;mutations.push(changed);
    let mut changed=inventory.clone();changed.distributions[0].version="2".into();mutations.push(changed);
    let mut changed=inventory.clone();changed.distributions[0].artifact_sha256=vec!["b".repeat(64)];mutations.push(changed);
    let mut changed=inventory.clone();changed.files[0].record_sha256=Some("different verified digest".into());mutations.push(changed);
    for changed in mutations {let changed=recapture(changed);assert_eq!(base.inputs()[0].captured().revision(),changed.inputs()[0].captured().revision());assert_ne!(key,request(&changed).identity().unwrap());}
    let dir=tempfile::tempdir().unwrap();std::fs::write(dir.path().join("api.py"),b"VALUE = 1\n").unwrap();std::fs::write(dir.path().join("helper.py"),b"HELPER = 1\n").unwrap();
    let added=captured(vec![AcquiredInput::tree(CapturedInput::capture(dir.path(),&["api.py".into(),"helper.py".into()],&budget).unwrap(),"tree")],&budget);
    let deleted=captured(vec![AcquiredInput::tree(CapturedInput::capture(dir.path(),&["api.py".into()],&budget).unwrap(),"tree")],&budget);
    assert_ne!(request(&added).identity().unwrap(),request(&deleted).identity().unwrap());
    std::fs::write(dir.path().join("README.md"),b"```python\nx=1\n```\n").unwrap();
    let derive=|ordinal|AcquiredInput::tree(CapturedInput::capture_derived(dir.path(),&["README.md".into()],&budget,&["README.md".into()],|document,_|Ok(vec![cpg_extract::capture::Derived{path:"_lctx/block.py".into(),document:document.into(),ordinal,fence:(0,5),bytes:b"x=1\n".to_vec()}])).unwrap(),"docs");
    let first=captured(vec![derive(0)],&budget);let next=captured(vec![derive(1)],&budget);
    assert_eq!(first.inputs()[0].captured().revision(),next.inputs()[0].captured().revision());
    assert_ne!(request(&first).identity().unwrap(),request(&next).identity().unwrap());
    let settings=cpg_extract::ruff_context::ContextSettings {custom_builtins:vec!["custom_name".into()],..Default::default()};
    let changed=CapturedInputs::new(vec![installed(&budget,b"RECORD",SourceRole::Release)],NativeContextConfig::committed(Profile::Catalog,&budget).unwrap().with_ruff(settings).unwrap());
    assert_ne!(key,request(&changed).identity().unwrap());
}

#[test]
fn corpus_origin_names_declared_base_and_exact_capture_without_ambient_git() {
    let budget=ResourceBudget::fixed(64<<20).unwrap();
    let source=tempfile::tempdir().unwrap();std::fs::write(source.path().join("api.py"),b"VALUE = 1\n").unwrap();
    let capture=||AcquiredInput::new(CapturedInput::capture(source.path(),&["api.py".into()],&budget).unwrap(),Acquisition::Corpus{repository:"https://example.invalid/source".into(),commit:"0123456789abcdef0123456789abcdef01234567".into(),uses:BTreeMap::new(),library:0});
    let first=capture();let origin=first.origin();
    assert!(matches!(&origin,InputOrigin::Corpus{revision,..} if revision==&format!("base:0123456789abcdef0123456789abcdef01234567;captured:{}",first.captured().revision().manifest.hex())));
    std::fs::write(source.path().join("unrelated.txt"),b"unselected dirtiness").unwrap();
    assert_eq!(origin,capture().origin());
    std::fs::write(source.path().join("api.py"),b"VALUE = 2\n").unwrap();
    let changed=capture();assert_ne!(origin,changed.origin());
    assert!(matches!(changed.acquisition(),Acquisition::Corpus{commit,..} if commit=="0123456789abcdef0123456789abcdef01234567"));
}
