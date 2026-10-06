//! Actual native compilation admits graph streams without a database or a publication transaction.
#[path = "fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{artifact, compilation::{self, PreparedCompilation}, workspace::{Workspace, WorkspaceOptions}};
use lctx_model::domain::{admission::Frontier, graph::GraphFamily, stages::Profile, ContentHash};
use std::sync::Arc;

async fn compiled(profile:Profile,frontier:Frontier,memory:usize,batch_rows:usize)->artifact::AdmittedArtifact {
    let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions {memory_bytes:memory,partitions:1,batch_rows}).unwrap();
    let captured=runtime::capture("catalog_core",profile,workspace.budget());
    let prepared=matches!(frontier,Frontier::Analysis|Frontier::Catalog).then(||PreparedCompilation::new(frontier,runtime::settings("api"),captured.config().catalog(),None,workspace.budget()).unwrap());
    compilation::compile(&workspace,captured.clone(),profile,ContentHash::of(b"artifact-fixture"),frontier,prepared.as_ref(),None,None).await.unwrap();
    artifact::admit(&workspace,&captured,frontier,profile,ContentHash::of(b"artifact-fixture")).await.unwrap()
}
#[tokio::test]
async fn both_profiles_admit_every_frontier_and_export_exact_originals() {
    for profile in Profile::ALL {
        for frontier in [Frontier::Facts,Frontier::Normalized,Frontier::Analysis,Frontier::Catalog] {
            let admitted=compiled(profile,frontier,1<<30,4096).await;
            let manifest=admitted.manifest();
            manifest.validate().unwrap();
            assert_eq!(manifest.frontier,frontier);
            assert!(manifest.families.iter().any(|f|f.family==GraphFamily::Entities&&f.rows>0));
            assert!(manifest.families.iter().any(|f|f.family==GraphFamily::Assertions&&f.rows>0));
            assert!(!manifest.required_outcomes.is_empty());
            let root=tempfile::tempdir().unwrap();let output=root.path().join("graph");
            admitted.export(&output).unwrap();
            assert!(output.join("manifest.json").is_file());
            assert!(output.join("entities.arrow").is_file());
            for original in &manifest.originals {
                let bytes=std::fs::read(output.join(format!("original-{}.bin",original.source.0.hex()))).unwrap();
                assert_eq!(bytes.len() as u64,original.byte_len);
                assert_eq!(ContentHash::of(&bytes),original.content);
            }
            let marker=std::fs::read(output.join("manifest.json")).unwrap();
            assert!(admitted.export(&output).is_err());
            assert_eq!(std::fs::read(output.join("manifest.json")).unwrap(),marker);
        }
    }
}
#[tokio::test]
async fn graph_content_is_independent_of_transfer_batching() {
    let ordinary=compiled(Profile::Catalog,Frontier::Normalized,1<<30,4096).await;
    let small=compiled(Profile::Catalog,Frontier::Normalized,128<<20,7).await;
    assert_eq!(ordinary.manifest().content(),small.manifest().content());
}
#[tokio::test]
async fn transported_graph_refuses_missing_and_tampered_originals() {
    let admitted=compiled(Profile::Catalog,Frontier::Facts,1<<30,4096).await;
    let resources=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let root=tempfile::tempdir().unwrap();let output=root.path().join("graph");admitted.export(&output).unwrap();
    admitted.verify_export(&output,&resources).await.unwrap();
    let original=&admitted.manifest().originals[0];let file=output.join(format!("original-{}.bin",original.source.0.hex()));
    let bytes=std::fs::read(&file).unwrap();std::fs::write(&file,b"tampered").unwrap();
    assert!(admitted.verify_export(&output,&resources).await.is_err());
    std::fs::write(&file,&bytes).unwrap();std::fs::remove_file(&file).unwrap();
    assert!(admitted.verify_export(&output,&resources).await.is_err());
    std::fs::write(&file,&bytes).unwrap();std::fs::write(output.join("assertions.arrow"),b"invalid stream").unwrap();
    assert!(admitted.verify_export(&output,&resources).await.is_err());
}
#[tokio::test]
async fn incomplete_or_cancelled_compilation_cannot_be_admitted() {
    let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let captured=runtime::capture("catalog_core",Profile::Catalog,workspace.budget());
    assert!(artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Catalog,ContentHash::of(b"fixture")).await.is_err());
    workspace.cancellation().cancel();
    assert!(artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Catalog,ContentHash::of(b"fixture")).await.is_err());
}

#[tokio::test]
async fn artifact_identity_includes_exact_consumed_embedding_values() {
    use cpg_core::embedding_service::{Embedder,FakeEmbedder};
    let embedder=FakeEmbedder::new();
    let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let captured=runtime::capture("normalized_relations",Profile::Catalog,workspace.budget());
    let mut settings=runtime::settings("analytic_text");settings.knn=true;
    let prepared=PreparedCompilation::new(Frontier::Analysis,settings,captured.config().catalog(),Some(&embedder),workspace.budget()).unwrap();
    compilation::compile(&workspace,captured.clone(),Profile::Catalog,ContentHash::of(b"vector-artifact-fixture"),Frontier::Analysis,Some(&prepared),Some(&embedder),None).await.unwrap();
    let admitted=artifact::admit(&workspace,&captured,Frontier::Analysis,Profile::Catalog,ContentHash::of(b"vector-artifact-fixture")).await.unwrap();
    assert!(!admitted.manifest().embeddings.is_empty(),"selected exact kNN must retain actual consumption");
    assert!(admitted.manifest().embeddings.iter().all(|value|value.dimension==embedder.spec().dimensions && value.specification==embedder.spec().hash()));
    let directory=tempfile::tempdir().unwrap();let destination=directory.path().join("vector-graph");
    admitted.export(&destination).unwrap();
    admitted.verify_export(&destination,&workspace).await.unwrap();
}

#[tokio::test]
async fn completed_compilation_binds_frontier_captures_and_configuration() {
    use lctx_model::domain::{stages::ProviderOutcome,input::Package};
    let workspace=Workspace::new(Arc::new(lctx_model::domain::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let captured=runtime::capture("catalog_core",Profile::Catalog,workspace.budget());
    let configuration=ContentHash::of(b"completed-facts-fixture");
    compilation::compile(&workspace,captured.clone(),Profile::Catalog,configuration,Frontier::Facts,None,None,None).await.unwrap();
    assert!(artifact::admit(&workspace,&captured,Frontier::Normalized,Profile::Catalog,configuration).await.is_err(),"completion cannot upgrade the frontier");
    assert!(artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Behavioral,configuration).await.is_err());
    assert!(artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Catalog,ContentHash::of(b"different configuration")).await.is_err());
    let other=runtime::capture("normalized_relations",Profile::Catalog,workspace.budget());
    assert!(artifact::admit(&workspace,&other,Frontier::Facts,Profile::Catalog,configuration).await.is_err(),"another capture cannot be relabelled admitted");
    let late=workspace.output("late",Profile::Catalog,configuration,workspace.inputs("late",Profile::Catalog,[]).unwrap());
    assert!(late.declare::<Package>().is_err());
    assert!(late.finish(ProviderOutcome::Complete).await.is_err());
    artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Catalog,configuration).await.unwrap();
}

fn transported<T:serde::de::DeserializeOwned>(path:&std::path::Path)->Vec<T> {
    use datafusion::arrow::{array::BinaryArray,ipc::reader::FileReader};
    FileReader::try_new(std::fs::File::open(path).unwrap(),None).unwrap().flat_map(|batch|{
        let batch=batch.unwrap();let payload=batch.column_by_name("payload").unwrap().as_any().downcast_ref::<BinaryArray>().unwrap();
        (0..batch.num_rows()).map(|row|serde_json::from_slice(payload.value(row)).unwrap()).collect::<Vec<T>>()
    }).collect()
}
fn completed_rows<R:lctx_model::domain::Record>(workspace:&Workspace)->Vec<R> {
    workspace.completed::<R>().unwrap().batches().unwrap().flat_map(|batch|R::decode(&batch.unwrap()).unwrap()).collect()
}
#[tokio::test]
async fn selected_analytics_export_membership_provenance_and_projection_losses() {
    use lctx_model::domain::{self as d,Record,graph::{Assertion,AssertionValue,AnalysisValue,Entity,SemanticKey}};
    let workspace=Workspace::new(Arc::new(d::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let captured=runtime::capture("analytic_optional",Profile::Catalog,workspace.budget());
    let mut settings=runtime::settings("api");
    settings.communities=true;settings.pagerank=true;settings.fca=true;settings.rca=true;
    let configuration=ContentHash::of(b"selected-analytics-artifact");
    let prepared=PreparedCompilation::new(Frontier::Analysis,settings,captured.config().catalog(),None,workspace.budget()).unwrap();
    compilation::compile(&workspace,captured.clone(),Profile::Catalog,configuration,Frontier::Analysis,Some(&prepared),None,None).await.unwrap();
    let admitted=artifact::admit(&workspace,&captured,Frontier::Analysis,Profile::Catalog,configuration).await.unwrap();
    let directory=tempfile::tempdir().unwrap();let destination=directory.path().join("selected-graph");
    admitted.export(&destination).unwrap();admitted.verify_export(&destination,&workspace).await.unwrap();
    let assertions:Vec<Assertion>=transported(&destination.join("assertions.arrow"));
    let entities:Vec<Entity>=transported(&destination.join("entities.arrow"));
    let results=assertions.iter().filter_map(|value|match &value.value {AssertionValue::Analysis(AnalysisValue::AnalyticTechnique(row))=>Some(row),_=>None}).collect::<Vec<_>>();
    for method in [d::analysis::AnalysisMethod::Communities,d::analysis::AnalysisMethod::PageRank,d::analysis::AnalysisMethod::Concepts,d::analysis::AnalysisMethod::RelationalConcepts] {
        assert!(results.iter().any(|row|row.method==method && row.selected && row.status!=d::analysis::AnalysisStatus::NotRequested),"selected {method:?} has no attributed graph outcome");
    }
    // These are semantic members and evidence, not optional diagnostic traces. Check exact
    // typed values and source keys against the completed owners, independently of graph mapping.
    // Explicit matches keep the acceptance inventory independent from emission macros.
    // Native run partitions are selected semantic memberships; heuristic presentation may be empty.
    let partitions=completed_rows::<d::analytics::PartitionMember>(&workspace);
    assert!(!partitions.is_empty(),"selected community analysis has no native partition memberships");
    for row in partitions {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticPartitionMember(v)) if v==&row)));}
    for row in completed_rows::<d::analytics::CommunityMember>(&workspace) {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticCommunityMember(v)) if v==&row)));}
    for row in completed_rows::<d::analytics::ConceptExtent>(&workspace) {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticConceptExtent(v)) if v==&row)));}
    for row in completed_rows::<d::analytics::ConceptIntent>(&workspace) {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Membership(d::graph::MembershipValue::AnalyticConceptIntent(v)) if v==&row)));}
    for row in completed_rows::<d::analytics::Incidence>(&workspace) {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Provenance(d::graph::ProvenanceValue::AnalyticIncidence(v)) if v==&row)));}
    for row in completed_rows::<d::analytics::IncidenceSource>(&workspace) {assert!(assertions.iter().any(|a|a.source==Some(SemanticKey::of(row.id())) && matches!(&a.value,AssertionValue::Provenance(d::graph::ProvenanceValue::AnalyticIncidenceSource(v)) if v==&row)));}
    assert!(!completed_rows::<d::analytics::ConceptExtent>(&workspace).is_empty());
    assert!(!completed_rows::<d::analytics::ConceptIntent>(&workspace).is_empty());
    assert!(!completed_rows::<d::analytics::Incidence>(&workspace).is_empty());
    for row in completed_rows::<d::analytics::Attribute>(&workspace) {assert!(entities.iter().any(|entity|matches!(entity,Entity::AnalyticAttribute(v) if v==&row)));}
    let projection=admitted.manifest().projections.iter().find(|p|p.name=="CallableInvocation").unwrap();
    assert_ne!(projection.source_membership,ContentHash::of(b""));
    assert_eq!(projection.definition,d::analysis::ProjectionDefinition::builtin(d::projection::ProjectionName::CallableInvocation).content_digest());
    assert!(projection.declared_losses.iter().any(|loss|loss.contains("conditions, provider qualifications and source evidence")));
    assert!(projection.declared_losses.iter().any(|loss|loss.contains(d::normalized::entities::EntityCategory::Type.label()) && loss.contains(d::normalized::entities::EntityCategory::Place.label())));
    let mut manifest=admitted.manifest().clone();
    manifest.projections.iter_mut().find(|p|p.name=="CallableInvocation").unwrap().declared_losses.clear();
    std::fs::write(destination.join("manifest.json"),serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(admitted.verify_export(&destination,&workspace).await.is_err(),"transport cannot erase projection losses");
}

#[tokio::test]
async fn raw_original_bytes_and_half_open_span_survive_artifact_transport() {
    use cpg_extract::{acquisition::AcquiredInput,bundle::CapturedInputs,capture::CapturedInput};
    use lctx_model::domain::{self as d,Record,graph::{Entity,EntityId}};
    let workspace=Workspace::new(Arc::new(d::model().unwrap()),WorkspaceOptions {memory_bytes:1<<30,..Default::default()}).unwrap();
    let input=tempfile::tempdir().unwrap();let raw=b"a\x00\xff\xfez\r\n";
    std::fs::write(input.path().join("original.bin"),raw).unwrap();
    std::fs::write(input.path().join("api.py"),b"def f(x): return x\n").unwrap();
    let captured=Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(CapturedInput::capture(input.path(),&["api.py".into(),"original.bin".into()],workspace.budget()).unwrap(),"raw-artifact")],cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog,workspace.budget()).unwrap()));
    let configuration=ContentHash::of(b"raw-original-artifact");
    let source=captured.inputs()[0].captured().artifacts().iter().find(|row|row.path=="original.bin").unwrap().clone();
    let evidence=d::assertion::Evidence::SourceSpan {source:source.id(),start:1,end:4};
    let declaration=d::stages::Stage {name:"raw-source-span-fixture",inputs:vec![],outputs:vec![],contributes:vec![d::stages::RelationUse::of::<d::assertion::Evidence>()],coverage:vec![],profiles:vec![Profile::Catalog],effect:d::stages::Effect::Pure,code:ContentHash::of(b"raw-source-span-fixture/v1"),configuration};
    let output=workspace.producer(&declaration,Profile::Catalog,workspace.inputs(declaration.name,Profile::Catalog,[]).unwrap());
    let batch=d::Batch::new(workspace.model(),vec![evidence.clone()],workspace.budget()).unwrap();
    output.contribute(&batch).unwrap();
    output.finish(d::stages::ProviderOutcome::Complete).await.unwrap();
    compilation::compile(&workspace,captured.clone(),Profile::Catalog,configuration,Frontier::Facts,None,None,None).await.unwrap();
    let admitted=artifact::admit(&workspace,&captured,Frontier::Facts,Profile::Catalog,configuration).await.unwrap();
    let directory=tempfile::tempdir().unwrap();let destination=directory.path().join("raw-graph");
    admitted.export(&destination).unwrap();admitted.verify_export(&destination,&workspace).await.unwrap();
    assert!(completed_rows::<d::source::SourceArtifact>(&workspace).contains(&source));
    let source_id=EntityId::of(source.id());
    let originals=std::fs::read(destination.join(format!("original-{}.bin",source_id.0.hex()))).unwrap();
    assert_eq!(originals,raw);assert!(std::str::from_utf8(&originals).is_err());
    let entities:Vec<Entity>=transported(&destination.join("entities.arrow"));
    assert!(entities.iter().any(|entity|matches!(entity,Entity::Source(row) if row==&source)));
    let restored=entities.into_iter().find(|entity|entity==&Entity::from(evidence.clone())).expect("admitted artifact lost its source-span evidence");
    if let Entity::Evidence(d::assertion::Evidence::SourceSpan {source,start,end})=restored {
        assert_eq!(EntityId::of(source),source_id);
        assert_eq!(&originals[start as usize..end as usize],b"\x00\xff\xfe");
    } else {panic!("source evidence lost its typed span");}
}
