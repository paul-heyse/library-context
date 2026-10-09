//! Canonical input companions under the actual current native scope layout.
use lctx_model::domain::{
    *, graph::{Assertion,Entity,EntityId,Target},
    input::{CorpusLibrary,DistributionRole,InputDistribution,InputRevision,Package,Release},
    source::{SourceArtifact,Module}, catalog::CatalogMember,
    retrieval::{Unit,Origin,Family,CorpusText,RENDER_VERSION}, attribution::AnalysisContext,
    resources::ResourceBudget, serving::{SnapshotHandle,DatabaseIdentity,Name},
};
use lctx_surrealdb::{Credentials,Loader,NativeReader,reader};

fn decode<R:Record>(batches:&lctx_surrealdb::batches::CanonicalBatches)->Vec<R>{
    batches.batches.iter().filter(|(name,_)|*name==R::NAME).flat_map(|(_,batch)|R::decode(batch).unwrap()).collect()
}

#[tokio::test]
async fn canonical_input_companions_retain_corpus_distribution_without_retired_scalars(){
    let cfg:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable native fixture")).unwrap()).unwrap();
    let credentials=Credentials::Root{username:cfg["admin_user"].as_str().unwrap().into(),password:cfg["admin_password"].as_str().unwrap().into()};
    let namespace="gn_companion_controls";let database=format!("companions_{}",std::process::id());
    let client=reader::connect(cfg["grpc_endpoint"].as_str().unwrap(),&credentials,namespace,&database).await.unwrap();
    client.query(format!("DEFINE NAMESPACE IF NOT EXISTS {namespace}; DEFINE DATABASE OVERWRITE {database} STRICT;")).await.unwrap().check().unwrap();
    let loader=Loader::new(client.clone());loader.install("").await.unwrap();
    let library=InputRevision{manifest:ContentHash::of(b"selected library input")};
    let corpus=InputRevision{manifest:ContentHash::of(b"selected corpus input")};
    let foreign=InputRevision{manifest:ContentHash::of(b"unrelated corpus input")};
    let package=Package{name:"companion-control".into()};let release=Release{package:package.id(),version:"1".into()};
    let source=SourceArtifact::from_bytes(corpus.id(),"guide.py".into(),b"selected corpus source").unwrap();
    let direct=SourceArtifact::from_bytes(library.id(),"library.py".into(),b"selected library source").unwrap();
    let module=Module{source:direct.id(),qualified_name:"library".into()};
    let member=CatalogMember{input:library.id(),access:module.id(),path:vec!["operation".into()],name:"operation".into()};
    let context=AnalysisContext{python_version:"3.14".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"config"),environment_digest:ContentHash::of(b"environment"),lock_digest:None};
    let origin=Origin::Source{artifact:source.id()};
    let text=CorpusText{family:Family::Source,rendering_version:RENDER_VERSION,digest:ContentHash::of(b"selected corpus source"),text:"selected corpus source".into()};
    let unit=Unit{input:corpus.id(),context:context.id(),family:Family::Source,origin:origin.id(),corpus:text.id(),title:"guide.py".into()};
    let entities=vec![Entity::from(library.clone()),Entity::from(corpus.clone()),Entity::from(foreign.clone()),Entity::from(package),Entity::from(release.clone()),Entity::from(source.clone()),Entity::from(direct.clone()),Entity::from(module),Entity::from(context),Entity::from(origin),Entity::from(text),Entity::from(member.clone()),Entity::from(unit.clone())];
    loader.entities(&entities).await.unwrap();loader.entity_references(&entities).await.unwrap();
    let companion=CorpusLibrary{corpus:corpus.id(),library:library.id()};
    let foreign_companion=CorpusLibrary{corpus:foreign.id(),library:foreign.id()};
    let library_distribution=InputDistribution{input:library.id(),release:release.id(),role:DistributionRole::FirstParty};
    let corpus_distribution=InputDistribution{input:corpus.id(),release:release.id(),role:DistributionRole::Dependency};
    let foreign_distribution=InputDistribution{input:foreign.id(),release:release.id(),role:DistributionRole::Dependency};
    let assertions=vec![Assertion::from_record(companion.clone()).unwrap(),Assertion::from_record(foreign_companion).unwrap(),Assertion::from_record(library_distribution.clone()).unwrap(),Assertion::from_record(corpus_distribution.clone()).unwrap(),Assertion::from_record(foreign_distribution).unwrap()];
    loader.assertions(&assertions).await.unwrap();loader.assertion_references(&assertions).await.unwrap();
    let handle=SnapshotHandle{semantic:ContentHash::of(b"private companion fixture"),realization:lctx_surrealdb::schema::realization_identity(""),database:DatabaseIdentity{namespace:Name::new(namespace).unwrap(),database:Name::new(&database).unwrap()}};
    let native=NativeReader::new(client.clone(),handle);let budget=ResourceBudget::fixed(32<<20).unwrap();
    // Current SCHEMAFULL rows contain canonical body.input and scope_keys, never scope_input.
    let retired:Vec<surrealdb::types::Value>=native.query("SELECT VALUE scope_input FROM entity WHERE semantic_type IN ['source_artifacts','catalog_members','retrieval_units']",surrealdb::types::Variables::new()).await.unwrap();
    assert!(retired.iter().all(|value|matches!(value,surrealdb::types::Value::None)));
    for include_corpus in [false,true] {
        let mut inputs=vec![ValidationInput::of::<SourceArtifact>(&["id"]),ValidationInput::of::<CatalogMember>(&["id"]),ValidationInput::of::<Unit>(&["id"]),ValidationInput::of::<InputDistribution>(&["id"]),ValidationInput::of::<InputRevision>(&["id"]),ValidationInput::of::<Release>(&["id"]),ValidationInput::of::<Package>(&["id"])];
        if include_corpus{inputs.push(ValidationInput::of::<CorpusLibrary>(&["id"]));}
        for (root,from_corpus) in [(EntityId::of(source.id()),true),(EntityId::of(unit.id()),true),(EntityId::of(direct.id()),false),(EntityId::of(member.id()),false)] {
            let batches=lctx_serving::scope::hydrate_with(&native,vec![reader::target_id(Target::Entity(root))],&inputs,&[],&budget).await.unwrap();
            let mut observed=decode::<InputDistribution>(&batches);observed.sort_by_key(Record::id);
            let mut expected=vec![library_distribution.clone()];if from_corpus{expected.push(corpus_distribution.clone());}expected.sort_by_key(Record::id);
            assert_eq!(observed,expected,"distribution companions follow canonical input and exact corpus membership");
            assert_eq!(decode::<CorpusLibrary>(&batches),if include_corpus&&from_corpus{vec![companion.clone()]}else{vec![]});
        }
    }
    client.query(format!("REMOVE DATABASE {database}")).await.unwrap().check().unwrap();
}
