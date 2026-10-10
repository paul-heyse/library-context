//! Actual admitted Catalog compilation, native publication, ten tools and original bytes.
#[path = "../../cpg-core/tests/fixtures/native.rs"]
mod native_fixture;
#[path = "../../cpg-core/tests/fixtures/catalog_runtime.rs"]
mod runtime;
use cpg_core::{
    artifact,
    compilation::{self, PreparedCompilation},
    embedding_service::{EmbedFuture, Embedder, FakeEmbedder},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{admission::Frontier, serving::*, stages::Profile, *};
use lctx_serving::NativeService;
use lctx_surrealdb::phase::{Phase, Terminal};
use lctx_surrealdb::{NativeReader, RecordSelection, RuntimeConfig};
use std::{io::Write, os::unix::fs::OpenOptionsExt, sync::Arc};
use tracing::{Instrument, instrument::WithSubscriber};

async fn journey(name: &'static str, run: impl std::future::Future<Output = ()>) {
    let dispatch = cpg_extract::logging::dispatch();
    let span = tracing::dispatcher::with_default(
        &dispatch,
        || tracing::info_span!(target: "lctx_phase", "native_journey", journey = name),
    );
    async {
        let phase = Phase::begin("native_journey");
        run.await;
        phase.finish(Terminal::Passed);
    }
    .instrument(span)
    .with_subscriber(dispatch)
    .await;
}
const LIBRARY: &str = "synthesis-sources";
/// Nominal E1 output needs known above-floor geometry, not the random-text fake's cosine.
/// This fixture owns a distinct encoder identity. A shared dominant component plus small
/// deterministic text components gives distinct full values and prefix projections with
/// above-floor cosine. It makes no claim about a live model's semantic similarity.
struct ContractEmbedder {
    inner: FakeEmbedder,
    specification: embedding::Spec,
}
impl ContractEmbedder {
    fn new() -> Self {
        let inner = FakeEmbedder::new();
        let mut specification = inner.spec().clone();
        specification.model = "lctx-native-journey-contract-aligned-vectors".into();
        specification.revision = "dominant-e0-random-quarter-v1".into();
        // Tokenization is still the exact delegated bytes/4 tokenizer; no pooling occurs.
        specification.server = "in-process-aligned-vector-fixture".into();
        Self {
            inner,
            specification,
        }
    }
}
impl Embedder for ContractEmbedder {
    fn spec(&self) -> &embedding::Spec {
        &self.specification
    }
    fn endpoint(&self) -> &str {
        "fixture://native-journey-aligned-vectors"
    }
    fn document_tokenizer(&self) -> Option<Arc<dyn retrieval::partition::Tokenizer>> {
        self.inner.document_tokenizer()
    }
    fn count_tokens<'a>(&'a self, text: &'a str) -> EmbedFuture<'a, usize> {
        self.inner.count_tokens(text)
    }
    fn embed<'a>(&'a self, texts: &'a [String]) -> EmbedFuture<'a, Vec<Vec<f32>>> {
        Box::pin(async move {
            Ok(texts
                .iter()
                .map(|text| {
                    let mut vector = self.inner.vector(text);
                    vector.iter_mut().for_each(|v| *v *= 0.25);
                    vector[0] = 1.0;
                    let norm = vector
                        .iter()
                        .map(|v| f64::from(*v).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    vector
                        .iter_mut()
                        .for_each(|v| *v = (f64::from(*v) / norm) as f32);
                    vector
                })
                .collect())
        })
    }
}
/// The source bytes remain the captured fixture. Explicit distribution ownership is the
/// library admission premise; a labelled, unowned tree supplies no such authority.
fn library_fixture(
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    library_fixture_at("synthesis_sources", LIBRARY, budget)
}
fn library_fixture_at(
    case: &str,
    library: &str,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    library_fixture_tree(
        runtime::capture(case, Profile::Catalog, budget),
        library,
        budget,
    )
}
fn library_fixture_tree(
    tree: Arc<cpg_extract::bundle::CapturedInputs>,
    library: &str,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    use cpg_extract::{
        acquisition::{
            AcquiredInput, Acquisition, InventoryDistribution, InventoryFile, LibraryInventory,
            derive_blocks,
        },
        bundle::CapturedInputs,
        capture::CapturedInput,
    };
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
#[tracing::instrument(target = "lctx_phase", name = "tool", skip_all, fields(tool))]
async fn call(
    service: &NativeService,
    tool: &str,
    request: serde_json::Value,
) -> serde_json::Value {
    let phase = Phase::begin("tool_execute");
    let encoded = service
        .execute(tool, &serde_json::to_string(&request).unwrap())
        .await
        .unwrap_or_else(|e| panic!("{tool} request {request}: {e}"));
    let result = serde_json::from_str(&encoded).unwrap();
    phase.finish(Terminal::Passed);
    result
}
#[tokio::test(flavor = "multi_thread")]
async fn compiled_catalog_serves_ten_tools_with_attributed_originals_and_foreign_cursor_refusal() {
    catalog_journey().await;
}
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires exclusive just service maintenance --native-clients"]
async fn published_service_refuses_executable_epoch_drift_before_library_lookup() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "exclusive maintenance owner required"
    );
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
    ))
    .unwrap();
    let installer = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
            .expect("validation maintenance installer"),
    ))
    .unwrap();
    let viewer = lctx_surrealdb::config::ViewerConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_NATIVE_SERVING_CONFIG").expect(
            "retained admitted synthetic serving fixture; run with just fixture --serving NAME",
        ),
    ))
    .unwrap();
    let handle = viewer.selected().unwrap();
    handle.validate_identity().unwrap();
    assert_eq!(
        installer.authentication,
        lctx_surrealdb::AuthenticationScope::Root
    );
    assert_eq!(config.namespace.as_str(), "library_context");
    assert_eq!(config.database.as_str(), "validation");
    assert_eq!(installer.namespace, config.namespace);
    assert_eq!(installer.database, config.database);
    assert_eq!(installer.endpoint, config.endpoint);
    assert_eq!(viewer.endpoint, config.endpoint);
    assert_eq!(installer.service_generation, config.service_generation);
    assert_eq!(handle.service_generation, config.service_generation);
    assert_eq!(handle.database.namespace, config.namespace);
    assert_eq!(handle.database.database, config.database);
    // Reuse exact admitted content; the ordinary producer still owns full Catalog qualification.
    lctx_publisher::inspection::audit(&config, &handle, &lctx_serving::native_definitions())
        .await
        .unwrap();
    let reader = NativeReader::connect(&viewer.endpoint, &viewer.credentials(), handle.clone())
        .await
        .unwrap();
    let result = async {
        let admin = lctx_surrealdb::reader::connect(&installer.endpoint, &installer.writer_credentials(), installer.namespace.as_str(), installer.database.as_str()).await?;
        let checked = async {
            async fn actual_function(
                client: &lctx_surrealdb::surrealdb::Surreal<lctx_surrealdb::surrealdb::engine::remote::grpc::Client>,
                function: &str,
            ) -> Result<String, ModelError> {
                use lctx_surrealdb::surrealdb::types::Value;
                let mut response = client.query("INFO FOR DB").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let value: Value = response.take(0).map_err(ModelError::codec)?;
                let Value::Object(info) = value else {
                    return Err(ModelError::Schema("native function inventory object"));
                };
                let Some(Value::Object(functions)) = info.get("functions") else { return Err(ModelError::Schema("native function inventory")); };
                let prefix = format!("DEFINE FUNCTION {function}(");
                let definitions = functions.values().filter_map(|value| match value {
                    Value::String(definition) if definition.starts_with(&prefix) => Some(definition),
                    _ => None,
                }).collect::<Vec<_>>();
                match definitions.as_slice() {
                    [definition] => Ok((*definition).clone()),
                    _ => Err(ModelError::Schema("exact retained operation function")),
                }
            }
            let function = handle.operation_definition_function();
            let saved = actual_function(&admin, &function).await?;
            let service = NativeService::new(reader.clone(), ResourceLimits::default())?;
            let refusal = async {
                admin.query(format!("DEFINE FUNCTION OVERWRITE {function}() {{ RETURN 'incompatible'; }} PERMISSIONS FULL;")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                Ok::<_, ModelError>(service.execute("find_operations", r#"{"library":"unqueried"}"#).await)
            }.await;
            // Restore even when mutation/dispatch fails; preserve the actual preflighted body.
            let restoration = async {
                admin.query(saved.replacen("DEFINE FUNCTION ", "DEFINE FUNCTION OVERWRITE ", 1)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                if actual_function(&admin, &function).await? != saved {
                    return Err(ModelError::Conflict("exact operation function restoration"));
                }
                Ok(())
            }.await;
            service.close().await;
            drop(service);
            let mut completion = completion::Completion::default();
            completion.step("operation function restoration", restoration);
            completion.step("retained publication audit", lctx_publisher::inspection::audit(&config, &handle, &lctx_serving::native_definitions()).await);
            completion::complete(refusal, completion)
        }.await;
        let mut completion = completion::Completion::default();
        completion.step("drift admin invalidation", admin.invalidate().await.map_err(ModelError::codec));
        completion::complete(checked, completion)
    }.await;
    let mut completion = completion::Completion::default();
    completion.step("retained drift reader close", reader.close().await);
    completion.step(
        "retained drift reader invalidation",
        reader
            .client()
            .invalidate()
            .await
            .map_err(ModelError::codec),
    );
    let refusal = completion::complete(result, completion).unwrap();
    assert_eq!(
        refusal.unwrap_err().public_failure(),
        PublicFailure::new(FailureKind::Incompatible)
    );
}
async fn catalog_journey() {
    journey("catalog_ten_tools", async {
    let setup_phase = Phase::begin("journey_setup");
    let scratch=tempfile::tempdir().unwrap();
    let mut config=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"))).unwrap();
    let installed_selection = config.selection.clone();
    let installed_selection_before = installed_selection.try_exists().unwrap()
        .then(|| std::fs::read(&installed_selection).unwrap());
    config.selection=scratch.path().join("selection.json");
    let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Catalog)
        .await
        .unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions {
            batch_rows: 128,
            ..Default::default()
        },
        native,
    )
    .unwrap();
    let captured = library_fixture(workspace.budget());
    let settings = ContentHash::of(b"native-serving-journey");
    let fake = ContractEmbedder::new();
    let mut analytics = runtime::settings("api");
    analytics.knn = true;
    let prepared = PreparedCompilation::new(
        Frontier::Catalog,
        analytics,
        captured.config().catalog(),
        Some(&fake),
        workspace.budget(),
    )
    .unwrap();
    setup_phase.finish(Terminal::Passed);
    compilation::compile(
        &workspace,
        captured.clone(),
        Profile::Catalog,
        settings,
        Frontier::Catalog,
        Some(&prepared),
        Some(&fake),
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
    let handle =
        lctx_publisher::seal_completed(&admitted, &config, &lctx_serving::native_definitions())
            .await
            .unwrap();
    assert!(!config.selection.exists());
    let reader = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await
    .unwrap();
    let encoder = Id::<embedding::EmbeddingSpec>::of(&embedding::EmbeddingSpecKey {
        service_hash: fake.spec().hash(),
    });
    let mut full = reader
        .records::<embedding::value::FullValue>(RecordSelection::Scope {
            field: "encoder".into(),
            values: vec![serde_json::to_value(encoder).unwrap()],
        })
        .await
        .unwrap();
    full.sort_by_key(Record::id);
    assert!(
        !full.is_empty(),
        "actual compiler must publish canonical full winners"
    );
    assert!(
        full.iter()
            .all(|v| v.dimensions == 4096 && v.bytes.0.len() == 4096 * 4)
    );
    let policy = embedding::projection::ProjectionDefinition::initial(fake.spec());
    let mut projected = reader
        .records::<embedding::projection::ProjectedValue>(RecordSelection::Scope {
            field: "definition".into(),
            values: vec![serde_json::to_value(policy.id()).unwrap()],
        })
        .await
        .unwrap();
    projected.sort_by_key(Record::id);
    assert_eq!(
        projected.len(),
        full.len(),
        "all canonical winners have the declared shared projection"
    );
    for p in &projected {
        p.verify(full.iter().find(|f| f.id() == p.value).unwrap(), &policy)
            .unwrap();
    }
    let mut analytic_uses = reader
        .records::<embedding::analytic::AnalysisEmbeddingUse>(RecordSelection::Scope {
            field: "specification".into(),
            values: vec![serde_json::to_value(encoder).unwrap()],
        })
        .await
        .unwrap();
    analytic_uses.sort_by_key(Record::id);
    assert!(!analytic_uses.is_empty());
    assert!(analytic_uses.iter().all(|u| {
        u.availability == embedding::analytic::VectorAvailability::Available
            && projected
                .iter()
                .any(|p| Some(p.id()) == u.projection && Some(p.value) == u.value)
    }));
    let mut results = reader
        .records::<analytics::TechniqueResult>(RecordSelection::Scope {
            field: "method".into(),
            values: vec![serde_json::to_value(analysis::AnalysisMethod::Neighbours).unwrap()],
        })
        .await
        .unwrap();
    results.sort_by_key(Record::id);
    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|r| r.selected && r.status == analysis::AnalysisStatus::Completed)
    );
    let frames = results
        .iter()
        .map(|r| serde_json::to_value(r.frame).unwrap())
        .collect::<Vec<_>>();
    let mut selections = reader
        .records::<analytics::VectorSelection>(RecordSelection::Scope {
            field: "frame".into(),
            values: frames.clone(),
        })
        .await
        .unwrap();
    selections.sort_by_key(Record::id);
    assert!(selections.iter().any(|s| s.available_windows > 0));
    let mut cohort_entities = std::collections::BTreeMap::new();
    for selection in &selections {
        if let Some(entity) = selection.entity.filter(|_| selection.available_windows > 0) {
            cohort_entities
                .entry(selection.frame)
                .or_insert_with(std::collections::BTreeSet::new)
                .insert(entity);
        }
    }
    assert!(
        cohort_entities.values().any(|entities| entities.len() >= 2),
        "chosen E1 frame must contain two distinct available public entities"
    );
    let result_ids = results
        .iter()
        .map(|r| serde_json::to_value(r.id()).unwrap())
        .collect::<Vec<_>>();
    let mut neighbours = reader
        .records::<analytics::Neighbour>(RecordSelection::Scope {
            field: "result".into(),
            values: result_ids.clone(),
        })
        .await
        .unwrap();
    neighbours.sort_by_key(Record::id);
    assert!(
        !neighbours.is_empty(),
        "chosen E1 policy must produce actual neighbour results"
    );
    let analytic_windows = reader
        .records::<embedding::text::TextWindow>(RecordSelection::Keys(
            analytic_uses.iter().map(|u| *u.window.bytes()).collect(),
        ))
        .await
        .unwrap();
    let analytic_assessments = reader
        .records::<embedding::text::TextAssessment>(RecordSelection::Keys(
            analytic_windows
                .iter()
                .map(|w| *w.assessment.bytes())
                .collect(),
        ))
        .await
        .unwrap();
    for neighbour in &neighbours {
        let result = results.iter().find(|r| r.id() == neighbour.result).unwrap();
        let entities = &cohort_entities[&result.frame];
        assert_ne!(neighbour.query, neighbour.target);
        assert!(entities.contains(&neighbour.query) && entities.contains(&neighbour.target));
        assert!(neighbour.score.get() >= analytics::policy::RETAINED.neighbour_floor);
        assert!(neighbour.score.get() <= 1.000001);
        let mut endpoint_digests = Vec::new();
        for (entity, use_id) in [
            (neighbour.query, neighbour.query_use),
            (neighbour.target, neighbour.target_use),
        ] {
            let use_ = analytic_uses.iter().find(|u| u.id() == use_id).unwrap();
            endpoint_digests.push(
                projected
                    .iter()
                    .find(|p| Some(p.id()) == use_.projection)
                    .unwrap()
                    .digest,
            );
            let window = analytic_windows
                .iter()
                .find(|w| w.id() == use_.window)
                .unwrap();
            let assessment = analytic_assessments
                .iter()
                .find(|a| a.id() == window.assessment)
                .unwrap();
            assert_eq!(assessment.entity, Some(entity));
            assert!(selections.iter().any(|s| {
                s.frame == result.frame
                    && s.entity == Some(entity)
                    && s.subject == assessment.subject
                    && s.invocation == use_.invocation
            }));
        }
        assert_ne!(endpoint_digests[0], endpoint_digests[1]);
    }
    let tools_phase = Phase::begin("tool_groups");
    let service = NativeService::new(reader.clone(), ResourceLimits::default()).unwrap();
    let missing = service
        .execute("browse_library", r#"{"library":"absent-library"}"#)
        .await
        .unwrap_err();
    assert_eq!(
        missing.public_failure(),
        PublicFailure::new(FailureKind::UnknownLibrary)
    );
    let invalid = service
        .execute("browse_library", r#"{"library":"fixture","unknown":true}"#)
        .await
        .unwrap_err();
    assert_eq!(
        invalid.public_failure(),
        PublicFailure::new(FailureKind::Incompatible)
    );
    let refused = service
        .execute_for("browse_library", r#"{"library":"fixture"}"#, None, false, 0)
        .await
        .unwrap_err();
    assert_eq!(
        refused.public_failure(),
        PublicFailure::new(FailureKind::ResourceRefused)
    );
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
    let mut foreign:Cursor=serde_json::from_slice(&hex::decode(&token).unwrap()).unwrap();
    foreign.binding.snapshot.realization=ContentHash::of(b"foreign executable");
    let refused=service.execute("find_operations",&serde_json::json!({"library":library,"page":{"size":1,"cursor":foreign.encode().unwrap().as_str()}}).to_string()).await.unwrap_err();
    assert_eq!(refused.public_failure(),PublicFailure::new(FailureKind::Incompatible));
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
    // A keeps its full serving capability epoch, actual public cursor and attributed bytes.
    // B uses a distinct supported search-only epoch: it is consumed through NativeReader,
    // not a second full NativeService. Publishing B must leave A's exact named functions intact.
    async fn executable_observation(
        reader: &NativeReader,
    ) -> Result<([String; 2], String), ModelError> {
        use lctx_surrealdb::surrealdb::types::Value;
        let mut response = reader.client().query("INFO FOR DB").await
            .map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let value: Value = response.take(0).map_err(ModelError::codec)?;
        let Value::Object(info) = value else {
            return Err(ModelError::Schema("coexisting native function inventory object"));
        };
        let Some(Value::Object(functions)) = info.get("functions") else {
            return Err(ModelError::Schema("coexisting native function inventory"));
        };
        let actual = |name: &str| -> Result<String, ModelError> {
            let prefix = format!("DEFINE FUNCTION {name}(");
            let mut matches = functions.values().filter_map(|value| match value {
                Value::String(definition) if definition.starts_with(&prefix) => Some(definition),
                _ => None,
            });
            let definition = matches.next().ok_or(ModelError::Schema(
                "exact coexisting native function",
            ))?;
            if matches.next().is_some() {
                return Err(ModelError::Conflict("duplicate coexisting native function"));
            }
            Ok(definition.clone())
        };
        let definitions = [
            actual(&reader.handle().operation_definition_function())?,
            actual(&reader.handle().library_roots_function())?,
        ];
        let operation: String = reader.query(
            format!("RETURN {}();", reader.handle().operation_definition_function()),
            Default::default(),
        ).await?;
        Ok((definitions, operation))
    }
    let a_executables_before = executable_observation(&reader).await.unwrap();
    assert_eq!(a_executables_before.1, lctx_serving::operation_definition().hex());
    let coexistence_phase = Phase::begin("distinct_capability_epoch_publication_coexists");
    let b_native = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Facts)
        .await
        .unwrap();
    let b_workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions::default(),
        b_native,
    ).unwrap();
    let b_identity = lctx_surrealdb::control::fresh_identity("native-journey-facts-coexistence").unwrap();
    let b_library = format!("native-coexistence-facts-{}", b_identity.hex());
    let b_captured = library_fixture_at("catalog_core", &b_library, b_workspace.budget());
    let b_settings = ContentHash::of(b"native-serving-distinct-facts-coexistence/v1");
    compilation::compile(
        &b_workspace, b_captured.clone(), Profile::Catalog, b_settings,
        Frontier::Facts, None, None, None,
    ).await.unwrap();
    let b_admitted = artifact::admit(
        &b_workspace, &b_captured, Frontier::Facts, Profile::Catalog, b_settings,
    ).await.unwrap();
    let b_handle = lctx_publisher::seal_completed(
        &b_admitted, &config, &lctx_surrealdb::materialization::native_definitions(),
    ).await.unwrap();
    let mut b_reader = None;
    let observations = async {
        lctx_publisher::inspection::audit(&config, &b_handle, &lctx_surrealdb::materialization::native_definitions()).await?;
        b_reader = Some(NativeReader::connect(
            &config.endpoint, &config.writer_credentials(), b_handle.clone(),
        ).await?);
        let b_sources = b_reader.as_ref().unwrap().records::<source::SourceArtifact>(
            RecordSelection::Scope {
                field: "input".into(),
                values: vec![serde_json::to_value(b_captured.inputs()[0].captured().revision().id()).map_err(ModelError::codec)?],
            },
        ).await?;
        let continued: serde_json::Value = serde_json::from_str(&service.execute(
            "find_operations",
            &serde_json::json!({"library":library,"page":{"size":1,"cursor":token}}).to_string(),
        ).await.map_err(|error| ModelError::Cause(Box::new(error)))?).map_err(ModelError::codec)?;
        let packet: serde_json::Value = serde_json::from_str(&service.execute(
            "get_operation",
            &serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["api","connect"]},"sections":["briefs","contextual_typing","scenarios","deployment","relationships","behavior","access_routes","incoming_references","conflicts"],"page":{"expanded":true}}).to_string(),
        ).await.map_err(|error| ModelError::Cause(Box::new(error)))?).map_err(ModelError::codec)?;
        let attributed: serde_json::Value = serde_json::from_str(&service.execute(
            "get_evidence",
            &serde_json::json!({"source":{"kind":"artifact","artifact":original.id()},"page":{"expanded":true}}).to_string(),
        ).await.map_err(|error| ModelError::Cause(Box::new(error)))?).map_err(ModelError::codec)?;
        let a_executables_after = executable_observation(&reader).await?;
        Ok::<_, ModelError>((b_sources, continued, packet, attributed, reader.handle().clone(), a_executables_after))
    }.await;
    let mut b_completion = completion::Completion::default();
    b_completion.step("coexisting Facts compiler drainage", b_workspace.drain().await);
    if let Some(b_reader) = b_reader.take() {
        b_completion.step("coexisting Facts reader close", b_reader.close().await);
        b_completion.step("coexisting Facts reader invalidation", b_reader.client().invalidate().await.map_err(ModelError::codec));
    }
    if b_completion.failures.is_empty() {
        b_completion.cleanup(b_handle.publication.hex(),
            lctx_publisher::backup::retire(&config, &b_handle, true).await.map(|_| ()));
    } else {
        b_completion.storage.push(completion::StorageState::Orphan(b_handle.publication.hex()));
    }
    let observations = completion::complete(observations, b_completion);
    coexistence_phase.finish_result(&observations);
    let (b_sources, continued, packet, attributed, a_handle_after, a_executables_after) = observations.unwrap();
    assert!(!b_sources.is_empty(), "B must contain real captured provider facts");
    assert_ne!(b_handle.semantic, handle.semantic);
    assert_ne!(b_handle.publication, handle.publication);
    assert_eq!(b_handle.database, handle.database);
    assert_eq!(b_handle.service_generation, handle.service_generation);
    assert_ne!(b_handle.definition_epoch, handle.definition_epoch,
        "full serving A and search-only B have distinct supported capability epochs");
    assert_eq!(a_handle_after, handle);
    assert_eq!(a_executables_after, a_executables_before,
        "A's exact named operation/library definitions and operation result survive B publication");
    assert_eq!(continued, next, "A's saved continuation remains exact after B publication");
    assert_eq!(packet, operation, "A's attributed operation packet remains exact");
    assert_eq!(attributed, evidence, "A's original evidence bytes and attribution remain exact");
    assert!(!config.selection.exists(), "neither publication selects itself");
    assert_eq!(installed_selection.try_exists().unwrap()
        .then(|| std::fs::read(&installed_selection).unwrap()), installed_selection_before);
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
    let callable_comparison=call(&service,"get_operation",serde_json::json!({"library":library,"operation":{"kind":"member","member":member},"sections":["callable_comparison"],"comparison":{"analysis":signature["analysis"],"left":signature["variant"],"right":signature["variant"]},"page":{"expanded":true}})).await;
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
    // Canonical full values and policies survive actual backup/re-admission. Restore has no
    // embedder or mutable-cache parameter and reconstructs search arrays from these exact bytes.
    tools_phase.finish(Terminal::Passed);
    let backup = scratch.path().join("canonical-values.surql");
    lctx_publisher::backup::backup(&config, &handle, &backup)
        .await
        .unwrap();
    let restored =
        lctx_publisher::backup::restore_publication(&config, &backup, handle.publication, &lctx_serving::native_definitions())
            .await
            .unwrap();
    assert_eq!(restored.semantic, handle.semantic);
    let restored_reader = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        restored.clone(),
    )
    .await
    .unwrap();
    let mut restored_full = restored_reader
        .records::<embedding::value::FullValue>(RecordSelection::Scope {
            field: "encoder".into(),
            values: vec![serde_json::to_value(encoder).unwrap()],
        })
        .await
        .unwrap();
    restored_full.sort_by_key(Record::id);
    assert_eq!(restored_full, full);
    let mut restored_projected = restored_reader
        .records::<embedding::projection::ProjectedValue>(RecordSelection::Scope {
            field: "definition".into(),
            values: vec![serde_json::to_value(policy.id()).unwrap()],
        })
        .await
        .unwrap();
    restored_projected.sort_by_key(Record::id);
    assert_eq!(restored_projected, projected);
    let mut restored_uses = restored_reader
        .records::<embedding::analytic::AnalysisEmbeddingUse>(RecordSelection::Scope {
            field: "specification".into(),
            values: vec![serde_json::to_value(encoder).unwrap()],
        })
        .await
        .unwrap();
    restored_uses.sort_by_key(Record::id);
    assert_eq!(restored_uses, analytic_uses);
    let mut restored_results = restored_reader
        .records::<analytics::TechniqueResult>(RecordSelection::Scope {
            field: "method".into(),
            values: vec![serde_json::to_value(analysis::AnalysisMethod::Neighbours).unwrap()],
        })
        .await
        .unwrap();
    restored_results.sort_by_key(Record::id);
    assert_eq!(restored_results, results);
    let mut restored_selections = restored_reader
        .records::<analytics::VectorSelection>(RecordSelection::Scope {
            field: "frame".into(),
            values: frames,
        })
        .await
        .unwrap();
    restored_selections.sort_by_key(Record::id);
    assert_eq!(restored_selections, selections);
    let mut restored_neighbours = restored_reader
        .records::<analytics::Neighbour>(RecordSelection::Scope {
            field: "result".into(),
            values: result_ids,
        })
        .await
        .unwrap();
    restored_neighbours.sort_by_key(Record::id);
    assert_eq!(restored_neighbours, neighbours);
    lctx_publisher::inspection::audit(&config, &restored, &lctx_serving::native_definitions())
        .await
        .unwrap();
    restored_reader.close().await.unwrap();
    restored_reader.client().invalidate().await.unwrap();
    drop(restored_reader);
    if restored != handle {
        lctx_publisher::backup::retire(&config, &restored, true).await.unwrap();
    }
    service.close().await;
    drop(service);
    reader.close().await.unwrap();
    let retained = std::env::var("LCTX_RETAIN_NATIVE_FIXTURE_CONFIG").ok();
    if let Some(path) = retained {
        let selection = std::path::Path::new(&path).with_extension("selected.json");
        let mut selected = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&selection)
            .unwrap();
        selected
            .write_all(&serde_json::to_vec(&handle).unwrap())
            .unwrap();
        selected.sync_all().unwrap();
        let viewer = lctx_surrealdb::config::ViewerConfig {
            endpoint: config.endpoint.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
            selection,
            serving_limits: None,
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
        lctx_publisher::backup::retire(&config,&handle,true).await.unwrap();
    }
    }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn remediation_browse_scopes_share_members_counts_and_vocabulary() {
    journey("browse_scopes", async {
    let setup_phase = Phase::begin("journey_setup");
    let scratch=tempfile::tempdir().unwrap();
    let mut config=RuntimeConfig::read(std::path::Path::new(&std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"))).unwrap();
    config.selection=scratch.path().join("selection.json");
    let native = lctx_surrealdb::compiler::NativeCompilerStore::begin(&config, Frontier::Catalog)
        .await
        .unwrap();
    let workspace = Workspace::new(
        Arc::new(model().unwrap()),
        WorkspaceOptions {
            batch_rows: 128,
            ..Default::default()
        },
        native,
    )
    .unwrap();
    let library = "remediation-scopes";
    // An independently captured beta document supplies a real association owned by another
    // member. No fabricated/missing key can stand in for this ownership adversary.
    let source_root = scratch.path().join("sources");
    std::fs::create_dir(&source_root).unwrap();
    let paths = ["alpha.py", "beta.py", "empty.py", "guide.md"].map(String::from);
    for path in &paths {
        std::fs::copy(
            runtime::root("remediation_scopes").join(path),
            source_root.join(path),
        )
        .unwrap();
    }
    let mut guide = std::fs::OpenOptions::new()
        .append(true)
        .open(source_root.join("guide.md"))
        .unwrap();
    guide.write_all(b"\n# Independent beta call\n\n```python\nfrom beta import beta\nbeta(foreign_one, foreign_two, foreign_three, unexpected=True)\n```\n").unwrap();
    drop(guide);
    let source_capture = cpg_extract::capture::CapturedInput::capture_derived(
        &source_root,
        &paths,
        workspace.budget(),
        &["guide.md".into()],
        cpg_extract::acquisition::derive_blocks,
    )
    .unwrap();
    let tree = Arc::new(cpg_extract::bundle::CapturedInputs::new(
        vec![cpg_extract::acquisition::AcquiredInput::tree(
            source_capture,
            "remediation_scopes",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(
            Profile::Catalog,
            workspace.budget(),
        )
        .unwrap(),
    ));
    let captured = library_fixture_tree(tree, library, workspace.budget());
    let mut analytics = runtime::settings("alpha");
    analytics.module_prefixes = vec!["alpha".into(), "beta".into()];
    analytics.public_roots = analytics.module_prefixes.clone();
    let settings = ContentHash::of(b"remediation-scoped-native-serving");
    let prepared = PreparedCompilation::new(
        Frontier::Catalog,
        analytics,
        captured.config().catalog(),
        None,
        workspace.budget(),
    )
    .unwrap();
    setup_phase.finish(Terminal::Passed);
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
    let handle =
        lctx_publisher::seal_completed(&admitted, &config, &lctx_serving::native_definitions())
            .await
            .unwrap();
    let reader = NativeReader::connect(&config.endpoint, &config.writer_credentials(), handle)
        .await
        .unwrap();
    let sources = reader
        .records::<source::SourceArtifact>(RecordSelection::Scope {
            field: "input".into(),
            values: vec![serde_json::json!(
                captured.inputs()[0].captured().revision().id().bytes()
            )],
        })
        .await
        .unwrap();
    let modules = reader
        .records::<source::Module>(RecordSelection::Scope {
            field: "source".into(),
            values: sources
                .iter()
                .map(|source| serde_json::json!(source.id().bytes()))
                .collect(),
        })
        .await
        .unwrap();
    let module = |name: &str| {
        modules
            .iter()
            .find(|module| module.qualified_name == name)
            .unwrap()
            .id()
    };
    let tools_phase = Phase::begin("tool_groups");
    let service = NativeService::new(reader.clone(), ResourceLimits::default()).unwrap();
    let unknown_scope = service
        .execute(
            "browse_library",
            &serde_json::json!({"library":library,"scope":{"kind":"module","module":vec![0u8;16]}})
                .to_string(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        unknown_scope.public_failure(),
        PublicFailure::new(FailureKind::Incompatible)
    );
    let selection = serde_json::json!({"requirements":[{"predicate":{"DeclaresParameter":{"name":"red"}},"quantifier":0}],"mode":1,"joint":1});
    for (module_name, class_name, own, foreign) in [
        ("alpha", "Alpha", "red", "blue"),
        ("beta", "Beta", "blue", "red"),
    ] {
        let scope = serde_json::json!({"kind":"module","module":module(module_name)});
        let members = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":scope,"page":{"size":100}}),
        )
        .await;
        let unique = members["entries"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["candidate"]["member"].to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!unique.is_empty());
        assert_eq!(members["extent"]["total"], unique.len() as u64);
        let groups = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":scope,"view":"modules"}),
        )
        .await;
        let groups = groups["entries"]["items"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["members"], unique.len() as u64);
        let vocabulary = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":scope,"view":"vocabulary"}),
        )
        .await;
        let names = vocabulary["entries"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|entry| entry["values"].as_array().unwrap())
            .filter_map(serde_json::Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(names.contains(own));
        assert!(!names.contains(foreign));
        let classes = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":scope,"view":"classes"}),
        )
        .await;
        let class_entries = classes["entries"]["items"].as_array().unwrap();
        assert_eq!(
            class_entries.len(),
            class_entries
                .iter()
                .map(|entry| entry["member"].to_string())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        let class = classes["entries"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"].as_str().unwrap().ends_with(class_name))
            .unwrap();
        let class_scope = serde_json::json!({"kind":"class","member":class["member"]});
        let children = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":class_scope,"page":{"size":100}}),
        )
        .await;
        let children_unique = children["entries"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["candidate"]["member"].to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!children_unique.is_empty());
        assert_eq!(class["members"], children_unique.len() as u64);
        let class_vocabulary = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":class_scope,"view":"vocabulary"}),
        )
        .await;
        let names = class_vocabulary["entries"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|entry| entry["values"].as_array().unwrap())
            .filter_map(serde_json::Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(names.contains(own));
        assert!(!names.contains(foreign));
        let selected = call(
            &service,
            "browse_library",
            serde_json::json!({"library":library,"scope":scope,"selection":selection}),
        )
        .await;
        if own == "blue" {
            assert!(selected["entries"]["items"].as_array().unwrap().is_empty());
        } else {
            assert!(!selected["entries"]["items"].as_array().unwrap().is_empty());
        }
        let selected_vocabulary = call(&service, "browse_library", serde_json::json!({"library":library,"scope":scope,"selection":selection,"view":"vocabulary"})).await;
        assert_eq!(
            selected_vocabulary["extent"]["total"],
            selected["extent"]["total"]
        );
        if own == "blue" {
            assert!(
                selected_vocabulary["entries"]["items"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        } else {
            let names = selected_vocabulary["entries"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|entry| entry["values"].as_array().unwrap())
                .filter_map(serde_json::Value::as_str)
                .collect::<std::collections::BTreeSet<_>>();
            assert!(names.contains("red"));
            assert!(!names.contains("blue"));
        }
    }
    let empty = call(&service, "browse_library", serde_json::json!({"library":library,"scope":{"kind":"module","module":module("empty")},"view":"vocabulary"})).await;
    assert_eq!(empty["extent"]["total"], 0);
    assert!(empty["entries"]["items"].as_array().unwrap().is_empty());
    // Independent document blocks call the same API. Each contains three distinct undefined
    // arguments; diagnostic correspondence must retain its exact owning association.
    // Mandatory signatures, coverage and delivery maps exceed the normal envelope here.
    // This control verifies parent-bound continuation through the public expanded route.
    let mut request = serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["alpha","alpha"]},"sections":["scenarios"],"page":{"size":1,"expanded":true}});
    let first = call(&service, "get_operation", request.clone()).await;
    let first_page = &first["operation"]["packet"]["scenarios"];
    request["page"]["cursor"] = first_page["continuation"].clone();
    assert!(!request["page"]["cursor"].is_null());
    let second = call(&service, "get_operation", request.clone()).await;
    let parent = &second["operation"]["packet"]["scenarios"]["items"][0];
    let scenario = parent["scenario"].clone();
    assert_ne!(scenario, first_page["items"][0]["scenario"]);
    let child = &parent["diagnostic_correlations"];
    assert!(child["omitted"].as_u64().unwrap() >= 2, "{parent}");
    let mut seen = std::collections::BTreeSet::from([child["items"][0].to_string()]);
    let nested_token = child["continuation"]
        .as_str()
        .expect("real nested continuation");
    let nested: Cursor = serde_json::from_slice(&hex::decode(nested_token).unwrap()).unwrap();
    let CursorPosition::ScenarioDiagnostic { association, key } = nested.after.clone() else {
        panic!("nested scenario diagnostic position");
    };
    // The independent oracle is the published canonical target/link witness inventory.
    // It does not invoke pagination or derive expected IDs from a returned page.
    let associations = reader
        .records::<catalog::evidence::ScenarioAssociation>(RecordSelection::Scope {
            field: "scenario".into(),
            values: vec![scenario.clone()],
        })
        .await
        .unwrap();
    assert!(associations.iter().any(|row| row.id() == association));
    let targets = reader
        .records::<catalog::evidence::DiagnosticUseTarget>(RecordSelection::Scope {
            field: "association".into(),
            values: vec![serde_json::json!(association)],
        })
        .await
        .unwrap();
    let links = reader
        .records::<catalog::evidence::DiagnosticUseLink>(RecordSelection::Keys(
            targets.iter().map(|target| *target.link.bytes()).collect(),
        ))
        .await
        .unwrap();
    assert_eq!(
        links.len(),
        targets
            .iter()
            .map(|target| target.link)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    let expected = links
        .iter()
        .map(|link| serde_json::to_value(link.assessment).unwrap().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(expected.len() >= 3);
    let mut cursor = child["continuation"].clone();
    while !cursor.is_null() {
        request["page"]["cursor"] = cursor;
        let next = call(&service, "get_operation", request.clone()).await;
        let parents = next["operation"]["packet"]["scenarios"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0]["scenario"], scenario);
        let children = &parents[0]["diagnostic_correlations"];
        assert!(
            seen.insert(children["items"][0].to_string()),
            "diagnostic repeated"
        );
        cursor = children["continuation"].clone();
    }
    assert_eq!(
        seen, expected,
        "all and only the selected parent's diagnostic witnesses"
    );
    let beta_operation = call(&service, "get_operation", serde_json::json!({"library":library,"operation":{"kind":"public_path","path":["beta","beta"]},"sections":["scenarios"],"page":{"size":1,"expanded":true}})).await;
    let beta_scenario = &beta_operation["operation"]["packet"]["scenarios"]["items"][0]["scenario"];
    assert!(!beta_scenario.is_null());
    let beta_associations = reader
        .records::<catalog::evidence::ScenarioAssociation>(RecordSelection::Scope {
            field: "scenario".into(),
            values: vec![beta_scenario.clone()],
        })
        .await
        .unwrap();
    let own = associations
        .iter()
        .find(|row| row.id() == association)
        .unwrap();
    let foreign = beta_associations
        .iter()
        .find(|row| row.member != own.member)
        .expect("actual beta association owns a different member");
    let mut existing_foreign_parent = nested.clone();
    existing_foreign_parent.after = CursorPosition::ScenarioDiagnostic {
        association: foreign.id(),
        key,
    };
    let mut existing_foreign_member = nested.clone();
    existing_foreign_member.binding.member = Some(foreign.member);
    for (case, adversary) in [
        ("existing foreign association", existing_foreign_parent),
        ("existing foreign member", existing_foreign_member),
    ] {
        request["page"]["cursor"] = serde_json::json!(adversary.encode().unwrap().as_str());
        let refused = service
            .execute("get_operation", &request.to_string())
            .await
            .unwrap_err();
        assert_eq!(
            refused.public_failure(),
            PublicFailure::new(FailureKind::Incompatible),
            "{case}"
        );
    }
    let mut absent_parent = nested.clone();
    absent_parent.after = CursorPosition::ScenarioDiagnostic {
        association: serde_json::from_value(serde_json::json!(vec![0u8; 16])).unwrap(),
        key,
    };
    let mut absent_key = nested.clone();
    absent_key.after = CursorPosition::ScenarioDiagnostic {
        association,
        key: ContentHash::of(b"absent diagnostic cursor position"),
    };
    let mut wrong_pin = nested.clone();
    wrong_pin.binding.snapshot.realization = ContentHash::of(b"foreign nested cursor pin");
    let mut wrong_member = nested.clone();
    wrong_member.binding.member =
        Some(serde_json::from_value(serde_json::json!(vec![0u8; 16])).unwrap());
    for (case, adversary) in [
        ("absent parent", absent_parent),
        ("absent position", absent_key),
        ("foreign pin", wrong_pin),
        ("foreign member", wrong_member),
    ] {
        request["page"]["cursor"] = serde_json::json!(adversary.encode().unwrap().as_str());
        let refused = service
            .execute("get_operation", &request.to_string())
            .await
            .unwrap_err();
        assert_eq!(
            refused.public_failure(),
            PublicFailure::new(FailureKind::Incompatible),
            "{case}"
        );
    }
    request["page"]["cursor"] = serde_json::json!(nested_token);
    request["operation"] = serde_json::json!({"kind":"public_path","path":["beta","beta"]});
    let refused = service
        .execute("get_operation", &request.to_string())
        .await
        .unwrap_err();
    assert_eq!(
        refused.public_failure(),
        PublicFailure::new(FailureKind::Incompatible),
        "changed operation request"
    );
    tools_phase.finish(Terminal::Passed);
    service.close().await;
    drop(service);
    reader.close().await.unwrap();
    }).await;
}
