//! Actual admitted Catalog compilation, native publication, ten tools and original bytes.
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact,
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{admission::Frontier, serving::*, stages::Profile, *};
use lctx_serving::NativeService;
use lctx_surrealdb::{NativeReader, RecordSelection, RuntimeConfig, reader};
use std::{io::Write, os::unix::fs::OpenOptionsExt, sync::Arc};
const LIBRARY: &str = "synthesis-sources";
/// The source bytes remain the captured fixture. Explicit distribution ownership is the
/// library admission premise; a labelled, unowned tree supplies no such authority.
fn library_fixture(
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    library_fixture_at("synthesis_sources", LIBRARY, budget)
}
fn library_fixture_at(case: &str, library: &str, budget: &lctx_model::domain::resources::ResourceBudget) -> Arc<cpg_extract::bundle::CapturedInputs> {
    use cpg_extract::{
        acquisition::{
            AcquiredInput, Acquisition, InventoryDistribution, InventoryFile, LibraryInventory,
            derive_blocks,
        },
        bundle::CapturedInputs,
        capture::CapturedInput,
    };
    let tree = runtime::capture(case, Profile::Catalog, budget);
    let original = tree.inputs()[0].captured();
    let derived = original
        .derivations()
        .iter()
        .map(|d| d.path())
        .collect::<std::collections::BTreeSet<_>>();
    let paths = original
        .artifacts()
        .iter()
        .filter(|a| !derived.contains(a.path.as_str()))
        .map(|a| a.path.clone())
        .collect::<Vec<_>>();
    let documents = paths
        .iter()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    let captured =
        CapturedInput::capture_derived(original.root(), &paths, budget, &documents, derive_blocks)
            .unwrap();
    let files = paths
        .into_iter()
        .filter_map(|path| {
            admission::ArtifactClass::of(&path).map(|class| InventoryFile {
                path,
                owners: vec![library.into()],
                role: match class {
                    admission::ArtifactClass::PythonSource => input::SourceRole::Release,
                    admission::ArtifactClass::Document => input::SourceRole::Document,
                },
                record_sha256: None,
            })
        })
        .collect();
    let inventory = LibraryInventory {
        name: library.into(),
        requirement: format!("{library}==0.0.0"),
        lock_digest: ContentHash::of(b"native-serving-first-party-fixture"),
        installer: None,
        python_version: "3.14.7".into(),
        platform: "linux".into(),
        site_packages: captured.root().to_owned(),
        distributions: vec![InventoryDistribution {
            name: library.into(),
            version: "0.0.0".into(),
            first_party: true,
            artifact_sha256: vec![],
            record_digest: captured.revision().manifest,
        }],
        files,
        configuration: ContentHash::of(b"native-serving-first-party-fixture/v1"),
    };
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::new(
            captured,
            Acquisition::Installed(inventory),
        )],
        tree.config().clone(),
    ))
}
async fn call(
    service: &NativeService,
    tool: &str,
    request: serde_json::Value,
) -> serde_json::Value {
    let encoded = service
        .execute(tool, &serde_json::to_string(&request).unwrap())
        .await
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    serde_json::from_str(&encoded).unwrap()
}
#[tokio::test]
async fn compiled_catalog_serves_ten_tools_with_attributed_originals_and_foreign_cursor_refusal() {
    let file = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("owned disposable SurrealDB fixture required");
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            partitions: 1,
            batch_rows: 128,
        },
    )
    .unwrap();
    let captured = library_fixture(workspace.budget());
    let settings = ContentHash::of(b"native-serving-journey");
    let prepared = PreparedCompilation::new(
        Frontier::Catalog,
        runtime::settings("api"),
        captured.config().catalog(),
        None,
        workspace.budget(),
    )
    .unwrap();
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        settings,
        Frontier::Catalog,
        Some(&prepared),
        None,
        None,
    )
    .await
    .unwrap();
    drop(prepared);
    let admitted = artifact::admit(
        &workspace,
        &captured,
        Frontier::Catalog,
        Profile::Catalog,
        settings,
    )
    .await
    .unwrap();
    let export = scratch.path().join("export");
    admitted.export(&export).unwrap();
    drop(admitted);
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();
    let config = RuntimeConfig {
        endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
        username: fixture["admin_user"].as_str().unwrap().into(),
        password: fixture["admin_password"].as_str().unwrap().into(),
        viewer_username: "serving_fixture".into(),
        viewer_password: format!("serving-fixture-{}", std::process::id()),
        namespace: Name::new("gn_serving_journeys").unwrap(),
        cache_database: Name::new("cache").unwrap(),
        selection: scratch.path().join("selection.json"),
    };
    let handle = lctx_publisher::publish(&verified, &config, &lctx_serving::native_definitions())
        .await
        .unwrap();
    assert!(!config.selection.exists());
    let reader = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap();
    let service = NativeService::new(reader.clone(), ResourceLimits::default()).unwrap();
    let missing = service.execute("browse_library", r#"{"library":"absent-library"}"#).await.unwrap_err();
    assert_eq!(missing.public_failure(), PublicFailure::new(FailureKind::UnknownLibrary));
    let invalid = service.execute("browse_library", r#"{"library":"fixture","unknown":true}"#).await.unwrap_err();
    assert_eq!(invalid.public_failure(), PublicFailure::new(FailureKind::Incompatible));
    let refused = service.execute_for("browse_library", r#"{"library":"fixture"}"#, None, false, 0).await.unwrap_err();
    assert_eq!(refused.public_failure(), PublicFailure::new(FailureKind::ResourceRefused));
    let library = LIBRARY;
    let find = call(
        &service,
        "find_operations",
        serde_json::json!({"library":library,"page":{"size":1}}),
    )
    .await;
    assert_eq!(find["snapshot"], serde_json::to_value(&handle).unwrap());
    assert!(
        find["supported"]["items"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    );
    let token = find["supported"]["continuation"]
        .as_str()
        .expect("multiple captured public members")
        .to_owned();
    let next = call(
        &service,
        "find_operations",
        serde_json::json!({"library":library,"page":{"size":1,"cursor":token}}),
    )
    .await;
    assert_ne!(
        next["supported"]["items"][0]["member"],
        find["supported"]["items"][0]["member"]
    );
    let mut foreign = handle.clone();
    foreign.realization = ContentHash::of(b"foreign executable");
    let foreign_client = reader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        config.namespace.as_str(),
        handle.database.database.as_str(),
    )
    .await
    .unwrap();
    let foreign_reader = NativeReader::new(foreign_client, foreign);
    let foreign_service = NativeService::new(foreign_reader, ResourceLimits::default()).unwrap();
    assert!(
        foreign_service
            .execute(
                "find_operations",
                &serde_json::json!({"library":library,"page":{"size":1,"cursor":token}})
                    .to_string()
            )
            .await
            .is_err()
    );
    let browse = call(
        &service,
        "browse_library",
        serde_json::json!({"library":library,"view":"members"}),
    )
    .await;
    assert!(!browse["entries"]["items"].as_array().unwrap().is_empty());
    let operation=call(&service,"get_operation",serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["api","connect"]},"sections":["briefs","contextual_typing","scenarios","deployment","relationships","behavior","access_routes","incoming_references","conflicts"],"page":{"expanded":true}})).await;
    assert_eq!(operation["operation"]["resolution"], "unique");
    assert!(
        !operation["operation"]["packet"]["access_routes"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let core = &operation["operation"]["packet"]["core"];
    assert!(core["signatures"].as_array().is_some_and(|v| !v.is_empty()));
    let member: Id<catalog::CatalogMember> =
        serde_json::from_value(core["member"].clone()).unwrap();
    let comparison=call(&service,"compare_operations",serde_json::json!({"library":library,"operations":[{"kind":"member","member":member},{"kind":"public_path","path":["api","alias"]}]})).await;
    assert_eq!(comparison["operations"].as_array().unwrap().len(), 2);
    let search = call(
        &service,
        "search_operations",
        serde_json::json!({"library":library,"query":"connect"}),
    )
    .await;
    assert_eq!(search["channels"]["vector"]["status"], "disabled");
    assert!(!search["results"]["items"].as_array().unwrap().is_empty());
    let degraded: serde_json::Value = serde_json::from_str(
        &service
            .execute_unavailable(
                "search_operations",
                &serde_json::json!({"library":library,"query":"connect"}).to_string(),
            )
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(degraded["channels"]["vector"]["status"], "degraded");
    let evidence_hits = call(
        &service,
        "search_evidence",
        serde_json::json!({"library":library,"query":"carefully","families":[]}),
    )
    .await;
    assert!(
        !evidence_hits["results"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let artifacts: Vec<source::SourceArtifact> = reader
        .records(RecordSelection::Scope {
            field: "path".into(),
            values: vec![serde_json::json!("api.py")],
        })
        .await
        .unwrap();
    let original = artifacts.first().expect("actual captured source");
    let evidence=call(&service,"get_evidence",serde_json::json!({"source":{"kind":"artifact","artifact":original.id()},"page":{"expanded":true}})).await;
    let body: Vec<u8> =
        serde_json::from_value(evidence["evidence"]["body"]["bytes"].clone()).unwrap();
    let expected = std::fs::read(runtime::root("synthesis_sources").join("api.py")).unwrap();
    assert_eq!(body, expected);
    // A rare authored term has a positive BM25 score even when brief fragmentation varies.
    let capabilities=call(&service,"search_capabilities",serde_json::json!({"library":library,"query":"carefully","page":{"size":1,"expanded":true}})).await;
    let brief = capabilities["results"]["items"]
        .as_array()
        .unwrap()
        .first()
        .expect("authored fixture brief")["capability"]
        .clone();
    let capability = call(
        &service,
        "get_capability",
        serde_json::json!({"capability":brief,"page":{"expanded":true}}),
    )
    .await;
    assert!(
        !capability["capability"]["assertions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        capability["capability"]["rendered"]
            .as_str()
            .unwrap()
            .contains("Connect")
    );
    let signature = core["signatures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| {
            s["parameters"].as_array().is_some_and(|v| {
                v.iter().any(|p| {
                    p["name"] == "host"
                        && p["formals"]
                            .as_array()
                            .is_some_and(|formals| !formals.is_empty())
                })
            })
        })
        .expect("connect host formal");
    let callable_comparison=call(&service,"get_operation",serde_json::json!({"library":library,"operation":{"kind":"member","member":member},"sections":["callable_comparison"],"comparison":{"analysis":signature["analysis"],"left":signature["variant"],"right":signature["variant"]}})).await;
    assert_eq!(
        callable_comparison["operation"]["packet"]["callable_comparison"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let parameter = signature["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "host")
        .unwrap();
    let formal = parameter["formals"]
        .as_array()
        .unwrap()
        .first()
        .expect("source formal")
        .clone();
    let inspection=call(&service,"inspect_value_paths",serde_json::json!({"member":member,"analysis":signature["analysis"],"inputs":[{"formal":formal,"value":{"kind":"string","value":"localhost"}}],"assumptions":{"builtin_namespace":"unknown"}})).await;
    assert!(inspection["paths"]["items"].is_array());
    assert!(
        service
            .execute_for(
                "find_operations",
                &serde_json::json!({"library":library}).to_string(),
                None,
                false,
                0
            )
            .await
            .is_err()
    );
    let retained = std::env::var("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG").ok();
    if let Some(path) = retained {
        let selection = std::path::Path::new(&path).with_extension("selected.json");
        let mut selected = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&selection).unwrap();
        selected.write_all(&serde_json::to_vec(&handle).unwrap()).unwrap();
        selected.sync_all().unwrap();
        let viewer = lctx_surrealdb::config::ViewerConfig {
            endpoint: config.endpoint.clone(),
            username: config.viewer_username.clone(),
            password: config.viewer_password.clone(),
            selection,
        };
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
        output
            .write_all(&serde_json::to_vec(&viewer).unwrap())
            .unwrap();
    } else {
        let admin = reader::connect(
            &config.endpoint,
            &config.root_credentials(),
            config.namespace.as_str(),
            handle.database.database.as_str(),
        )
        .await
        .unwrap();
        admin
            .query(format!(
                "REMOVE DATABASE `{}`",
                handle.database.database.as_str()
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
    }
}

#[tokio::test]
async fn remediation_browse_scopes_share_members_counts_and_vocabulary() {
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(
        std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned fixture required")
    ).unwrap()).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions {
        memory_bytes: 1 << 30, partitions: 1, batch_rows: 128,
    }).unwrap();
    let library = "remediation-scopes";
    let captured = library_fixture_at("remediation_scopes", library, workspace.budget());
    let mut analytics = runtime::settings("alpha");
    analytics.module_prefixes = vec!["alpha".into(), "beta".into()];
    analytics.public_roots = analytics.module_prefixes.clone();
    let settings = ContentHash::of(b"remediation-scoped-native-serving");
    let prepared = PreparedCompilation::new(Frontier::Catalog, analytics,
        captured.config().catalog(), None, workspace.budget()).unwrap();
    compilation::compile(&workspace, captured.clone(), Profile::Catalog, settings,
        Frontier::Catalog, Some(&prepared), None, None).await.unwrap();
    drop(prepared);
    let admitted = artifact::admit(&workspace, &captured, Frontier::Catalog, Profile::Catalog, settings).await.unwrap();
    let export = scratch.path().join("export");
    admitted.export(&export).unwrap();
    drop(admitted);
    let verified = artifact::verify_export(&export, &workspace).await.unwrap();
    let config = RuntimeConfig {
        endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
        username: fixture["admin_user"].as_str().unwrap().into(),
        password: fixture["admin_password"].as_str().unwrap().into(),
        viewer_username: "scope_viewer".into(), viewer_password: "owned-scope-viewer".into(),
        namespace: Name::new("gn_remediation_scopes").unwrap(),
        cache_database: Name::new("cache").unwrap(), selection: scratch.path().join("selection.json"),
    };
    let handle = lctx_publisher::publish(&verified, &config, &lctx_serving::native_definitions()).await.unwrap();
    let reader = NativeReader::connect(&config.endpoint, &config.viewer_credentials(), handle).await.unwrap();
    let sources = reader.records::<source::SourceArtifact>(RecordSelection::Scope { field: "input".into(), values: vec![serde_json::json!(captured.inputs()[0].captured().revision().id().bytes())] }).await.unwrap();
    let modules = reader.records::<source::Module>(RecordSelection::Scope { field: "source".into(), values: sources.iter().map(|source| serde_json::json!(source.id().bytes())).collect() }).await.unwrap();
    let module = |name: &str| modules.iter().find(|module| module.qualified_name == name).unwrap().id();
    let service = NativeService::new(reader.clone(), ResourceLimits::default()).unwrap();
    let unknown_scope = service.execute("browse_library", &serde_json::json!({"library":library,"scope":{"kind":"module","module":vec![0u8;16]}}).to_string()).await.unwrap_err();
    assert_eq!(unknown_scope.public_failure(), PublicFailure::new(FailureKind::Incompatible));
    let selection = serde_json::json!({"requirements":[{"predicate":{"DeclaresParameter":{"name":"red"}},"quantifier":0}],"mode":1,"joint":1});
    for (module_name, class_name, own, foreign) in [("alpha", "Alpha", "red", "blue"), ("beta", "Beta", "blue", "red")] {
        let scope = serde_json::json!({"kind":"module","module":module(module_name)});
        let members = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"page":{"size":100}})).await;
        let unique = members["entries"]["items"].as_array().unwrap().iter().map(|entry| entry["candidate"]["member"].to_string()).collect::<std::collections::BTreeSet<_>>();
        assert!(!unique.is_empty());
        assert_eq!(members["extent"]["total"], unique.len() as u64);
        let groups = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"view":"modules"})).await;
        let groups = groups["entries"]["items"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["members"], unique.len() as u64);
        let vocabulary = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"view":"vocabulary"})).await;
        let names = vocabulary["entries"]["items"].as_array().unwrap().iter().flat_map(|entry| entry["values"].as_array().unwrap()).filter_map(serde_json::Value::as_str).collect::<std::collections::BTreeSet<_>>();
        assert!(names.contains(own)); assert!(!names.contains(foreign));
        let classes = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"view":"classes"})).await;
        let class_entries = classes["entries"]["items"].as_array().unwrap();
        assert_eq!(class_entries.len(), class_entries.iter().map(|entry| entry["member"].to_string()).collect::<std::collections::BTreeSet<_>>().len());
        let class = classes["entries"]["items"].as_array().unwrap().iter().find(|entry| entry["name"].as_str().unwrap().ends_with(class_name)).unwrap();
        let class_scope = serde_json::json!({"kind":"class","member":class["member"]});
        let children = call(&service, "browse_library", serde_json::json!({"library":library,"scope":class_scope,"page":{"size":100}})).await;
        let children_unique = children["entries"]["items"].as_array().unwrap().iter().map(|entry| entry["candidate"]["member"].to_string()).collect::<std::collections::BTreeSet<_>>();
        assert!(!children_unique.is_empty());
        assert_eq!(class["members"], children_unique.len() as u64);
        let class_vocabulary = call(&service, "browse_library", serde_json::json!({"library":library,"scope":class_scope,"view":"vocabulary"})).await;
        let encoded = class_vocabulary.to_string(); assert!(encoded.contains(own)); assert!(!encoded.contains(foreign));
        let selected = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"selection":selection})).await;
        if own == "blue" { assert!(selected["entries"]["items"].as_array().unwrap().is_empty()); }
        else { assert!(!selected["entries"]["items"].as_array().unwrap().is_empty()); }
        let selected_vocabulary = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"selection":selection,"view":"vocabulary"})).await;
        assert_eq!(selected_vocabulary["extent"]["total"], selected["extent"]["total"]);
        if own == "blue" { assert!(selected_vocabulary["entries"]["items"].as_array().unwrap().is_empty()); }
        else {
            let names = selected_vocabulary["entries"]["items"].as_array().unwrap().iter().flat_map(|entry| entry["values"].as_array().unwrap()).filter_map(serde_json::Value::as_str).collect::<std::collections::BTreeSet<_>>();
            assert!(names.contains("red")); assert!(!names.contains("blue"));
        }
    }
    let empty = call(&service, "browse_library", serde_json::json!({"library":library,"scope":{"kind":"module","module":module("empty")},"view":"vocabulary"})).await;
    assert_eq!(empty["extent"]["total"], 0); assert!(empty["entries"]["items"].as_array().unwrap().is_empty());
    // Independent document blocks call the same API. Each contains three distinct undefined
    // arguments; diagnostic correspondence must retain its exact owning association.
    let mut request = serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["alpha","alpha"]},"sections":["scenarios"],"page":{"size":1}});
    let first = call(&service,"get_operation",request.clone()).await;
    let first_page = &first["operation"]["packet"]["scenarios"];
    request["page"]["cursor"] = first_page["continuation"].clone();
    assert!(!request["page"]["cursor"].is_null());
    let second = call(&service,"get_operation",request.clone()).await;
    let parent = &second["operation"]["packet"]["scenarios"]["items"][0];
    let scenario = parent["scenario"].clone();
    assert_ne!(scenario, first_page["items"][0]["scenario"]);
    let child = &parent["diagnostic_correlations"];
    assert!(child["omitted"].as_u64().unwrap() >= 2, "{parent}");
    let mut seen = std::collections::BTreeSet::from([child["items"][0].to_string()]);
    let nested_token = child["continuation"].as_str().expect("real nested continuation");
    let nested: Cursor = serde_json::from_slice(&hex::decode(nested_token).unwrap()).unwrap();
    let CursorPosition::ScenarioDiagnostic { association, key } = nested.after.clone() else { panic!("nested scenario diagnostic position"); };
    // The independent oracle is the published canonical target/link witness inventory.
    // It does not invoke pagination or derive expected IDs from a returned page.
    let associations = reader.records::<catalog::evidence::ScenarioAssociation>(RecordSelection::Scope { field: "scenario".into(), values: vec![scenario.clone()] }).await.unwrap();
    assert!(associations.iter().any(|row| row.id() == association));
    let targets = reader.records::<catalog::evidence::DiagnosticUseTarget>(RecordSelection::Scope { field: "association".into(), values: vec![serde_json::json!(association)] }).await.unwrap();
    let links = reader.records::<catalog::evidence::DiagnosticUseLink>(RecordSelection::Keys(targets.iter().map(|target| *target.link.bytes()).collect())).await.unwrap();
    assert_eq!(links.len(), targets.iter().map(|target| target.link).collect::<std::collections::BTreeSet<_>>().len());
    let expected = links.iter().map(|link| serde_json::to_value(link.assessment).unwrap().to_string()).collect::<std::collections::BTreeSet<_>>();
    assert!(expected.len() >= 3);
    let mut cursor = child["continuation"].clone();
    while !cursor.is_null() {
        request["page"]["cursor"] = cursor;
        let next = call(&service,"get_operation",request.clone()).await;
        let parents = next["operation"]["packet"]["scenarios"]["items"].as_array().unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0]["scenario"], scenario);
        let children = &parents[0]["diagnostic_correlations"];
        assert!(seen.insert(children["items"][0].to_string()), "diagnostic repeated");
        cursor = children["continuation"].clone();
    }
    assert_eq!(seen, expected, "all and only the selected parent's diagnostic witnesses");
    let mut absent_parent = nested.clone();
    absent_parent.after = CursorPosition::ScenarioDiagnostic { association: serde_json::from_value(serde_json::json!(vec![0u8;16])).unwrap(), key };
    let mut absent_key = nested.clone();
    absent_key.after = CursorPosition::ScenarioDiagnostic { association, key: ContentHash::of(b"absent diagnostic cursor position") };
    let mut wrong_pin = nested.clone();
    wrong_pin.binding.snapshot.realization = ContentHash::of(b"foreign nested cursor pin");
    let mut wrong_member = nested.clone();
    wrong_member.binding.member = Some(serde_json::from_value(serde_json::json!(vec![0u8;16])).unwrap());
    for (case, adversary) in [("absent parent", absent_parent), ("absent position", absent_key), ("foreign pin", wrong_pin), ("foreign member", wrong_member)] {
        request["page"]["cursor"] = serde_json::json!(adversary.encode().unwrap().as_str());
        let refused = service.execute("get_operation", &request.to_string()).await.unwrap_err();
        assert_eq!(refused.public_failure(), PublicFailure::new(FailureKind::Incompatible), "{case}");
    }
    request["page"]["cursor"] = serde_json::json!(nested_token);
    request["operation"] = serde_json::json!({"kind":"public_path","path":["beta","beta"]});
    let refused = service.execute("get_operation", &request.to_string()).await.unwrap_err();
    assert_eq!(refused.public_failure(), PublicFailure::new(FailureKind::Incompatible), "changed operation request");
}
