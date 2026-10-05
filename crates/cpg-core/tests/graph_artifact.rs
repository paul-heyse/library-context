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
