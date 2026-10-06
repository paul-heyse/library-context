//! Actual admitted Catalog compilation, native publication, ten tools and original bytes.
#[path="../../cpg-core/tests/fixtures/catalog_runtime.rs"] mod runtime;
use cpg_core::{artifact,compilation::{self,PreparedCompilation},workspace::{Workspace,WorkspaceOptions}};
use lctx_model::domain::{*,admission::Frontier,stages::Profile,serving::*};
use lctx_serving::NativeService;
use lctx_surrealdb::{NativeReader,RuntimeConfig,reader,RecordSelection};
use std::{sync::Arc,io::Write,os::unix::fs::OpenOptionsExt};
const LIBRARY:&str="synthesis-sources";
/// The source bytes remain the captured fixture. Explicit distribution ownership is the
/// library admission premise; a labelled, unowned tree supplies no such authority.
fn library_fixture(budget:&lctx_model::domain::resources::ResourceBudget)->Arc<cpg_extract::bundle::CapturedInputs>{
 use cpg_extract::{acquisition::{AcquiredInput,Acquisition,LibraryInventory,InventoryDistribution,InventoryFile,derive_blocks},capture::CapturedInput,bundle::CapturedInputs};
 let tree=runtime::capture("synthesis_sources",Profile::Catalog,budget);let original=tree.inputs()[0].captured();
 let derived=original.derivations().iter().map(|d|d.path()).collect::<std::collections::BTreeSet<_>>();
 let paths=original.artifacts().iter().filter(|a|!derived.contains(a.path.as_str())).map(|a|a.path.clone()).collect::<Vec<_>>();
 let documents=paths.iter().filter(|p|p.ends_with(".md")||p.ends_with(".mdx")).cloned().collect::<Vec<_>>();
 let captured=CapturedInput::capture_derived(original.root(),&paths,budget,&documents,derive_blocks).unwrap();
 let files=paths.into_iter().filter_map(|path|admission::ArtifactClass::of(&path).map(|class|InventoryFile{path,owners:vec![LIBRARY.into()],role:match class{admission::ArtifactClass::PythonSource=>input::SourceRole::Release,admission::ArtifactClass::Document=>input::SourceRole::Document},record_sha256:None})).collect();
 let inventory=LibraryInventory{name:LIBRARY.into(),requirement:format!("{LIBRARY}==0.0.0"),lock_digest:ContentHash::of(b"native-serving-first-party-fixture"),installer:None,python_version:"3.14.7".into(),platform:"linux".into(),site_packages:captured.root().to_owned(),distributions:vec![InventoryDistribution{name:LIBRARY.into(),version:"0.0.0".into(),first_party:true,artifact_sha256:vec![],record_digest:captured.revision().manifest}],files,configuration:ContentHash::of(b"native-serving-first-party-fixture/v1")};
 Arc::new(CapturedInputs::new(vec![AcquiredInput::new(captured,Acquisition::Installed(inventory))],tree.config().clone()))
}
async fn call(service:&NativeService,tool:&str,request:serde_json::Value)->serde_json::Value{
 let encoded=service.execute(tool,&serde_json::to_string(&request).unwrap()).await.unwrap_or_else(|e|panic!("{tool}: {e}"));
 serde_json::from_str(&encoded).unwrap()
}
#[tokio::test]
async fn compiled_catalog_serves_ten_tools_with_attributed_originals_and_foreign_cursor_refusal(){
 let file=std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable SurrealDB fixture required");
 let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();let scratch=tempfile::tempdir().unwrap();
 let workspace=Workspace::new(Arc::new(model().unwrap()),WorkspaceOptions{memory_bytes:1<<30,partitions:1,batch_rows:128}).unwrap();
 let captured=library_fixture(workspace.budget());let settings=ContentHash::of(b"native-serving-journey");
 let prepared=PreparedCompilation::new(Frontier::Catalog,runtime::settings("api"),captured.config().catalog(),None,workspace.budget()).unwrap();
 compilation::compile(&workspace,captured.clone(),Profile::Catalog,settings,Frontier::Catalog,Some(&prepared),None,None).await.unwrap();drop(prepared);
 let admitted=artifact::admit(&workspace,&captured,Frontier::Catalog,Profile::Catalog,settings).await.unwrap();let export=scratch.path().join("export");admitted.export(&export).unwrap();drop(admitted);
 let verified=artifact::verify_export(&export,&workspace).await.unwrap();
 let config=RuntimeConfig{endpoint:fixture["grpc_endpoint"].as_str().unwrap().into(),username:fixture["admin_user"].as_str().unwrap().into(),password:fixture["admin_password"].as_str().unwrap().into(),viewer_username:"serving_fixture".into(),viewer_password:format!("serving-fixture-{}",std::process::id()),namespace:Name::new("gn_serving_journeys").unwrap(),cache_database:Name::new("cache").unwrap(),selection:scratch.path().join("selection.json")};
 let handle=lctx_publisher::publish(&verified,&config,&lctx_serving::native_definitions()).await.unwrap();assert!(!config.selection.exists());
 let reader=NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await.unwrap();
 let service=NativeService::new(reader.clone(),ResourceLimits::default()).unwrap();
 let library=LIBRARY;
 let find=call(&service,"find_operations",serde_json::json!({"library":library,"page":{"size":1}})).await;
 assert_eq!(find["snapshot"],serde_json::to_value(&handle).unwrap());assert!(find["supported"]["items"].as_array().is_some_and(|v|!v.is_empty()));
 let token=find["supported"]["continuation"].as_str().expect("multiple captured public members").to_owned();
 let next=call(&service,"find_operations",serde_json::json!({"library":library,"page":{"size":1,"cursor":token}})).await;
 assert_ne!(next["supported"]["items"][0]["member"],find["supported"]["items"][0]["member"]);
 let mut foreign=handle.clone();foreign.realization=ContentHash::of(b"foreign executable");let foreign_client=reader::connect(&config.endpoint,&config.viewer_credentials(),config.namespace.as_str(),handle.database.database.as_str()).await.unwrap();let foreign_reader=NativeReader::new(foreign_client,foreign);let foreign_service=NativeService::new(foreign_reader,ResourceLimits::default()).unwrap();
 assert!(foreign_service.execute("find_operations",&serde_json::json!({"library":library,"page":{"size":1,"cursor":token}}).to_string()).await.is_err());
 let browse=call(&service,"browse_library",serde_json::json!({"library":library,"view":"members"})).await;assert!(!browse["entries"]["items"].as_array().unwrap().is_empty());
 let operation=call(&service,"get_operation",serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["api","connect"]},"sections":["briefs","contextual_typing","scenarios","deployment","relationships","behavior"],"page":{"expanded":true}})).await;
 assert_eq!(operation["operation"]["resolution"],"unique");let core=&operation["operation"]["packet"]["core"];
 assert!(core["signatures"].as_array().is_some_and(|v|!v.is_empty()));
 let member:Id<catalog::CatalogMember>=serde_json::from_value(core["member"].clone()).unwrap();
 let comparison=call(&service,"compare_operations",serde_json::json!({"library":library,"operations":[{"kind":"member","member":member},{"kind":"public_path","path":["api","alias"]}]})).await;assert_eq!(comparison["operations"].as_array().unwrap().len(),2);
 let search=call(&service,"search_operations",serde_json::json!({"library":library,"query":"connect"})).await;assert_eq!(search["channels"]["vector"]["status"],"disabled");assert!(!search["results"]["items"].as_array().unwrap().is_empty());
 let degraded:serde_json::Value=serde_json::from_str(&service.execute_unavailable("search_operations",&serde_json::json!({"library":library,"query":"connect"}).to_string()).await.unwrap()).unwrap();assert_eq!(degraded["channels"]["vector"]["status"],"degraded");
 let evidence_hits=call(&service,"search_evidence",serde_json::json!({"library":library,"query":"connect","families":[]})).await;assert!(!evidence_hits["results"]["items"].as_array().unwrap().is_empty());
 let artifacts:Vec<source::SourceArtifact>=reader.records(RecordSelection::Scope{field:"path".into(),values:vec![serde_json::json!("api.py")]}).await.unwrap();let original=artifacts.first().expect("actual captured source");
 let evidence=call(&service,"get_evidence",serde_json::json!({"source":{"kind":"artifact","artifact":original.id()},"page":{"expanded":true}})).await;
 let body:Vec<u8>=serde_json::from_value(evidence["evidence"]["body"]["bytes"].clone()).unwrap();let expected=std::fs::read(runtime::root("synthesis_sources").join("api.py")).unwrap();assert_eq!(body,expected);
 let capabilities=call(&service,"search_capabilities",serde_json::json!({"library":library,"query":"connect"})).await;
 let brief=capabilities["results"]["items"].as_array().unwrap().first().expect("authored fixture brief")["capability"].clone();
 let capability=call(&service,"get_capability",serde_json::json!({"capability":brief,"page":{"expanded":true}})).await;assert!(!capability["capability"]["assertions"].as_array().unwrap().is_empty());assert!(capability["capability"]["rendered"].as_str().unwrap().contains("Connect"));
 let signature=core["signatures"].as_array().unwrap().iter().find(|s|s["parameters"].as_array().is_some_and(|v|v.iter().any(|p|p["name"]=="host"))).expect("connect host formal");let parameter=signature["parameters"].as_array().unwrap().iter().find(|p|p["name"]=="host").unwrap();let formal=parameter["formals"].as_array().unwrap().first().expect("source formal").clone();
 let inspection=call(&service,"inspect_value_paths",serde_json::json!({"member":member,"analysis":signature["analysis"],"inputs":[{"formal":formal,"value":{"kind":"string","value":"localhost"}}],"assumptions":{"builtin_namespace":"unknown"}})).await;assert!(inspection["paths"]["items"].is_array());
 assert!(service.execute_for("find_operations",&serde_json::json!({"library":library}).to_string(),None,false,0).await.is_err());
 let retained=std::env::var("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG").ok();
 if let Some(path)=retained{let viewer=lctx_surrealdb::config::ViewerConfig{endpoint:config.endpoint.clone(),username:config.viewer_username.clone(),password:config.viewer_password.clone(),snapshot:handle.clone()};let mut output=std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path).unwrap();output.write_all(&serde_json::to_vec(&viewer).unwrap()).unwrap();}else{let admin=reader::connect(&config.endpoint,&config.root_credentials(),config.namespace.as_str(),handle.database.database.as_str()).await.unwrap();admin.query(format!("REMOVE DATABASE `{}`",handle.database.database.as_str())).await.unwrap().check().unwrap();}
}
