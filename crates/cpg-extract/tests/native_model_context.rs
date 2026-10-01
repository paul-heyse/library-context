//! Model requirements extend the one linked provider's captured dependency context.
#[path="typed_driver/mod.rs"] mod typed_driver;
use cpg_extract::{acquisition::*,bundle::CapturedInputs,capture::CapturedInput,native_context::{CatalogSelection,NativeContextConfig},pyrefly_stage::Pyrefly,typed_syntax::SyntaxLimits};
use lctx_model::domain::{attribution::*,calls::*,input::SourceRole,resources::ResourceBudget,stages::Profile,symbols::*,syntax::SubjectBoundary,*};
use std::sync::Arc;
inspector!(Observed, ProviderSymbol);
const SOURCE:&str=r#"
version = 7
[[models]]
phase = "call"
revision = 1
target = { scope="dependency", distribution="depctx", version="1.0", module="depctx", callable="Child.ping" }
coverage = { transfers="partial", effects="unspecified", callbacks="unspecified", resources="unspecified", exceptions="unspecified" }
[[models.rules]]
kind = "transfer"
from = { kind="parameter", name="value" }
to = { kind="return_value" }
transfer = "identity"
modality = "definite"
[[context_protocols]]
revision = 1
target = { scope="dependency", distribution="depctx", version="1.0", module="depctx", callable="Child" }
allocation = { scope="stdlib", python="3.14.7", module="builtins", callable="object.__new__" }
initialization = { scope="dependency", distribution="depctx", version="1.0", module="depctx", callable="Child.__init__" }
entry = { kind="none_value" }
exit = { kind="preserve" }
"#;
fn budget()->ResourceBudget { ResourceBudget::fixed(1<<30).unwrap() }
fn captured(source:&str,profile:Profile,budget:&ResourceBudget)->Arc<CapturedInputs> {
    let fixture=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/native_model_context");
    let paths:Vec<String>=["app.py","depctx.py","depbase.py","ancestors.py"].into_iter().map(Into::into).collect();
    let frozen=CapturedInput::capture(&fixture,&paths,budget).unwrap();
    let inventory=LibraryInventory{name:"app".into(),requirement:"app==1.0".into(),lock_digest:ContentHash::of(b"pinned test lock"),installer:Some("native fixture".into()),python_version:"3.14.7".into(),platform:"linux".into(),site_packages:fixture,
        distributions:[("app",true),("depctx",false)].into_iter().map(|(name,first_party)|InventoryDistribution{name:name.into(),version:"1.0".into(),first_party,artifact_sha256:vec![],record_digest:ContentHash::of(b"fixture record")}).collect(),
        files:paths.into_iter().map(|path|InventoryFile{role:if path=="app.py" {SourceRole::Release} else {SourceRole::Dependency},owners:vec![if path=="app.py" {"app".into()} else {"depctx".into()}],path,record_sha256:None}).collect(),configuration:ContentHash::of(b"fixture")};
    let selected=CatalogSelection::parse("model-context.toml",source,budget).unwrap();
    Arc::new(CapturedInputs::new(vec![AcquiredInput::new(frozen,Acquisition::Installed(inventory))],NativeContextConfig::new(profile,selected)))
}
async fn extract(source:&str,profile:Profile)->(typed_driver::Tables,Id<AnalysisContext>) {
    let resources=budget();let inputs=captured(source,profile,&resources);let context=cpg_extract::pyrefly_stage::analysis_context(&inputs.inputs()[0],None,inputs.config()).unwrap();let tables=typed_driver::Tables::default();
    typed_driver::run_profile_with_budget(inputs,Pyrefly::new(SyntaxLimits::default()),Observed(tables.clone()),profile,Default::default(),false,resources).await.unwrap();
    (tables,context.id())
}
#[tokio::test]
async fn behavioral_requirements_capture_unimported_protocols_exact_exceptions_and_ancestry() {
    let(tables,_)=extract(SOURCE,Profile::Behavioral).await;
    let symbols=typed_driver::rows::<ProviderSymbol>(&tables);let ancestry=typed_driver::rows::<ClassAncestryObservation>(&tables);let members=typed_driver::rows::<SymbolSequenceMember>(&tables);
    for name in ["Child","Base","Root","TypeError","OSError","BaseException"] {let symbol=symbols.iter().find(|s|s.name==name && s.kind==SymbolKind::Class).unwrap_or_else(||panic!("required class {name} absent"));assert!(ancestry.iter().any(|r|r.class==symbol.id() && r.relation==AncestryRelation::Mro && r.linearization==Some(Linearization::Complete)),"MRO for {name}");}
    let child=symbols.iter().find(|s|s.name=="Child" && s.kind==SymbolKind::Class).unwrap();let mro=ancestry.iter().find(|r|r.class==child.id() && r.relation==AncestryRelation::Mro).unwrap();let names:Vec<_>=members.iter().filter(|m|m.sequence==mro.ancestors).map(|m|symbols.iter().find(|s|s.id()==m.symbol).unwrap().name.as_str()).collect();assert!(names.contains(&"Base")&&names.contains(&"Root"));
    let boundaries=typed_driver::rows::<SubjectBoundary>(&tables);assert!(!boundaries.iter().any(|b|b.detail.as_ref().is_some_and(|d|d.starts_with("model-context "))),"model context boundaries: {:?}",boundaries.iter().filter_map(|b|b.detail.as_ref()).collect::<Vec<_>>());
    let contexts=typed_driver::rows::<AnalysisContext>(&tables);assert_eq!(contexts.len(),1,"all providers share configured identity");
}
#[tokio::test]
async fn missing_and_version_changed_requirements_remain_explicit_and_change_context_identity() {
    let(_,original)=extract(SOURCE,Profile::Behavioral).await;
    for (source,expected) in [(SOURCE.replace("Child.ping","Missing.ping"),"unavailable"),(SOURCE.replace("module=\"depctx\"","module=\"notcaptured_model_dep\""),"unavailable"),(SOURCE.replace("version=\"1.0\"","version=\"2.0\""),"captured distribution is 1.0")] {
        let(tables,changed)=extract(&source,Profile::Behavioral).await;assert_ne!(original,changed);let boundaries=typed_driver::rows::<SubjectBoundary>(&tables);assert!(boundaries.iter().any(|b|b.detail.as_ref().is_some_and(|d|d.starts_with("model-context ") && d.contains(expected))),"{:?}",boundaries.iter().filter_map(|b|b.detail.as_ref()).collect::<Vec<_>>());assert!(typed_driver::rows::<ProviderCoverage>(&tables).iter().any(|c|c.family==FactFamily::Signatures && c.status==CoverageStatus::Partial));
    }
}
#[tokio::test]
async fn catalog_profile_binds_selection_without_dependency_expansion_or_flow() {
    let(tables,context)=extract(SOURCE,Profile::Catalog).await;assert!(!typed_driver::rows::<ProviderSymbol>(&tables).iter().any(|s|s.name=="Child"));assert!(!typed_driver::rows::<Provider>(&tables).iter().any(|p|p.tool=="ty"));assert!(typed_driver::rows::<ProviderCoverage>(&tables).iter().filter(|c|c.family==FactFamily::Flow).all(|c|c.status==CoverageStatus::NotRequested));
    let resources=budget();let changed=captured(&SOURCE.replace("Child.ping","Missing.ping"),Profile::Catalog,&resources);assert!(changed.config().requirements("3.14.7",&resources).unwrap().is_none());assert_ne!(context,cpg_extract::pyrefly_stage::analysis_context(&changed.inputs()[0],None,changed.config()).unwrap().id());
    assert!(changed.config().check_profile(Profile::Behavioral).is_err());
}
#[test]
fn catalog_parse_admission_is_reserved_before_parsing_and_retained_through_selection() {
    let resources=budget();let selected=CatalogSelection::parse("test.toml",SOURCE,&resources).unwrap();assert!(resources.reserved()>SOURCE.len());let config=NativeContextConfig::new(Profile::Behavioral,selected.clone());drop(selected);assert!(resources.reserved()>0);drop(config);assert_eq!(resources.reserved(),0);assert!(CatalogSelection::parse("test.toml",SOURCE,&ResourceBudget::fixed(1).unwrap()).is_err());
}
#[tokio::test]
async fn driver_refuses_a_profile_mismatch_before_the_first_provider_can_write() {
    let resources=budget();let inputs=captured(SOURCE,Profile::Catalog,&resources);let tables=typed_driver::Tables::default();
    let error=typed_driver::run_profile_with_budget(inputs,Pyrefly::new(SyntaxLimits::default()),Observed(tables.clone()),Profile::Behavioral,Default::default(),false,resources).await.unwrap_err();
    assert!(error.to_string().contains("scheduled profile differs"));assert!(tables.lock().unwrap().is_empty());
}

#[tokio::test]
async fn driver_refuses_a_foreign_catalog_allocation_pool_before_the_first_write() {
    let selected_pool=budget();let attempt_pool=budget();let inputs=captured(SOURCE,Profile::Catalog,&selected_pool);let tables=typed_driver::Tables::default();
    assert!(inputs.config().check_budget(&selected_pool.clone()).is_ok());
    let error=typed_driver::run_profile_with_budget(inputs,Pyrefly::new(SyntaxLimits::default()),Observed(tables.clone()),Profile::Catalog,Default::default(),false,attempt_pool.clone()).await.unwrap_err();
    assert!(error.to_string().contains("different attempt resource pool"));assert!(tables.lock().unwrap().is_empty());assert_eq!(attempt_pool.reserved(),0);assert_eq!(selected_pool.reserved(),0);
}
