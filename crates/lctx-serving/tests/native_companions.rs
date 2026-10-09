//! Canonical input companions under the actual current native scope layout.
use lctx_model::domain::{
    attribution::AnalysisContext,
    catalog::CatalogMember,
    graph::{Assertion, Entity, EntityId, Target},
    input::{CorpusLibrary, DistributionRole, InputDistribution, InputRevision, Package, Release},
    resources::ResourceBudget,
    retrieval::{CorpusText, Family, Origin, RENDER_VERSION, Unit},
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    source::{Module, SourceArtifact},
    *,
};
use lctx_surrealdb::{Credentials, Loader, NativeReader, reader};

fn decode<R: Record>(batches: &lctx_surrealdb::batches::CanonicalBatches) -> Vec<R> {
    batches
        .batches
        .iter()
        .filter(|(name, _)| *name == R::NAME)
        .flat_map(|(_, batch)| R::decode(batch).unwrap())
        .collect()
}

#[tokio::test]
async fn canonical_input_companions_retain_corpus_distribution_without_retired_scalars() {
    let cfg: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable native fixture"),
        )
        .unwrap(),
    )
    .unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let namespace = "gn_companion_controls";
    let database = format!("companions_{}", std::process::id());
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        namespace,
        &database,
    )
    .await
    .unwrap();
    client.query(format!("DEFINE NAMESPACE IF NOT EXISTS {namespace}; DEFINE DATABASE OVERWRITE {database} STRICT;")).await.unwrap().check().unwrap();
    let loader = Loader::new(client.clone());
    loader.install("").await.unwrap();
    let library = InputRevision {
        manifest: ContentHash::of(b"selected library input"),
    };
    let corpus = InputRevision {
        manifest: ContentHash::of(b"selected corpus input"),
    };
    let foreign = InputRevision {
        manifest: ContentHash::of(b"unrelated corpus input"),
    };
    let package = Package {
        name: "companion-control".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let source =
        SourceArtifact::from_bytes(corpus.id(), "guide.py".into(), b"selected corpus source")
            .unwrap();
    let direct = SourceArtifact::from_bytes(
        library.id(),
        "library.py".into(),
        b"selected library source",
    )
    .unwrap();
    let module = Module {
        source: direct.id(),
        qualified_name: "library".into(),
    };
    let member = CatalogMember {
        input: library.id(),
        access: module.id(),
        path: vec!["operation".into()],
        name: "operation".into(),
    };
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"environment"),
        lock_digest: None,
    };
    let origin = Origin::Source {
        artifact: source.id(),
    };
    let text = CorpusText {
        family: Family::Source,
        rendering_version: RENDER_VERSION,
        digest: ContentHash::of(b"selected corpus source"),
        text: "selected corpus source".into(),
    };
    let unit = Unit {
        input: corpus.id(),
        context: context.id(),
        family: Family::Source,
        origin: origin.id(),
        corpus: text.id(),
        title: "guide.py".into(),
    };
    let entities = vec![
        Entity::from(library.clone()),
        Entity::from(corpus.clone()),
        Entity::from(foreign.clone()),
        Entity::from(package),
        Entity::from(release.clone()),
        Entity::from(source.clone()),
        Entity::from(direct.clone()),
        Entity::from(module),
        Entity::from(context),
        Entity::from(origin),
        Entity::from(text),
        Entity::from(member.clone()),
        Entity::from(unit.clone()),
    ];
    loader.entities(&entities).await.unwrap();
    loader.entity_references(&entities).await.unwrap();
    let companion = CorpusLibrary {
        corpus: corpus.id(),
        library: library.id(),
    };
    let foreign_companion = CorpusLibrary {
        corpus: foreign.id(),
        library: foreign.id(),
    };
    let library_distribution = InputDistribution {
        input: library.id(),
        release: release.id(),
        role: DistributionRole::FirstParty,
    };
    let corpus_distribution = InputDistribution {
        input: corpus.id(),
        release: release.id(),
        role: DistributionRole::Dependency,
    };
    let foreign_distribution = InputDistribution {
        input: foreign.id(),
        release: release.id(),
        role: DistributionRole::Dependency,
    };
    let assertions = vec![
        Assertion::from_record(companion.clone()).unwrap(),
        Assertion::from_record(foreign_companion).unwrap(),
        Assertion::from_record(library_distribution.clone()).unwrap(),
        Assertion::from_record(corpus_distribution.clone()).unwrap(),
        Assertion::from_record(foreign_distribution).unwrap(),
    ];
    loader.assertions(&assertions).await.unwrap();
    loader.assertion_references(&assertions).await.unwrap();
    let handle = SnapshotHandle {
        semantic: ContentHash::of(b"private companion fixture"),
        realization: lctx_surrealdb::schema::realization_identity(""),
        database: DatabaseIdentity {
            namespace: Name::new(namespace).unwrap(),
            database: Name::new(&database).unwrap(),
        },
    };
    let native = NativeReader::new(client.clone(), handle);
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    // Current SCHEMAFULL rows contain canonical body.input and scope_keys, never scope_input.
    let retired:Vec<bool>=native.query("SELECT VALUE scope_input IS NONE FROM entity WHERE semantic_type IN ['source_artifacts','catalog_members','retrieval_units']",surrealdb::types::Variables::new()).await.unwrap();
    assert_eq!(retired.len(), 4);
    assert!(retired.iter().all(|absent| *absent));
    for include_corpus in [false, true] {
        let mut inputs = vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<CatalogMember>(&["id"]),
            ValidationInput::of::<Unit>(&["id"]),
            ValidationInput::of::<InputDistribution>(&["id"]),
            ValidationInput::of::<InputRevision>(&["id"]),
            ValidationInput::of::<Release>(&["id"]),
            ValidationInput::of::<Package>(&["id"]),
        ];
        if include_corpus {
            inputs.push(ValidationInput::of::<CorpusLibrary>(&["id"]));
        }
        for (root, from_corpus) in [
            (EntityId::of(source.id()), true),
            (EntityId::of(unit.id()), true),
            (EntityId::of(direct.id()), false),
            (EntityId::of(member.id()), false),
        ] {
            let batches = lctx_serving::scope::hydrate_with(
                &native,
                vec![reader::target_id(Target::Entity(root))],
                &inputs,
                &[],
                &budget,
            )
            .await
            .unwrap();
            let mut observed = decode::<InputDistribution>(&batches);
            observed.sort_by_key(Record::id);
            let mut expected = vec![library_distribution.clone()];
            if from_corpus {
                expected.push(corpus_distribution.clone());
            }
            expected.sort_by_key(Record::id);
            assert_eq!(
                observed, expected,
                "distribution companions follow canonical input and exact corpus membership"
            );
            assert_eq!(
                decode::<CorpusLibrary>(&batches),
                if include_corpus && from_corpus {
                    vec![companion.clone()]
                } else {
                    vec![]
                }
            );
        }
    }
    client
        .query(format!("REMOVE DATABASE {database}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}

#[tokio::test]
async fn shared_serving_template_binds_each_frontier_and_hydration_to_its_reader_database() {
    use lctx_model::domain::serving_scope::ServingScopeProgram;
    use lctx_surrealdb::scope::PreparedServingScope;
    let cfg: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable native fixture"),
        )
        .unwrap(),
    )
    .unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let namespace = "gn_shared_serving_scope_controls";
    let database_a = format!("reader_a_{}", std::process::id());
    let database_b = format!("reader_b_{}", std::process::id());
    let client_a = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        namespace,
        &database_a,
    )
    .await
    .unwrap();
    let client_b = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        namespace,
        &database_b,
    )
    .await
    .unwrap();
    for (client, database) in [(&client_a, &database_a), (&client_b, &database_b)] {
        client.query(format!("DEFINE NAMESPACE IF NOT EXISTS {namespace}; DEFINE DATABASE OVERWRITE {database} STRICT;")).await.unwrap().check().unwrap();
        Loader::new(client.clone()).install("").await.unwrap();
    }
    let package_a = Package {
        name: "shared-template-reader-a".into(),
    };
    let package_b = Package {
        name: "shared-template-reader-b".into(),
    };
    let release_a = Release {
        package: package_a.id(),
        version: "1".into(),
    };
    let release_b = Release {
        package: package_b.id(),
        version: "1".into(),
    };
    for (client, entities) in [
        (
            &client_a,
            vec![
                Entity::from(package_a.clone()),
                Entity::from(release_a.clone()),
            ],
        ),
        (
            &client_b,
            vec![
                Entity::from(package_b.clone()),
                Entity::from(release_b.clone()),
            ],
        ),
    ] {
        let loader = Loader::new(client.clone());
        loader.entities(&entities).await.unwrap();
        loader.entity_references(&entities).await.unwrap();
    }
    let handle = |database: &str, semantic: &[u8]| SnapshotHandle {
        semantic: ContentHash::of(semantic),
        realization: lctx_surrealdb::schema::realization_identity(""),
        database: DatabaseIdentity {
            namespace: Name::new(namespace).unwrap(),
            database: Name::new(database).unwrap(),
        },
    };
    let native_a = NativeReader::new(
        client_a.clone(),
        handle(&database_a, b"private scope reader a"),
    );
    let native_b = NativeReader::new(
        client_b.clone(),
        handle(&database_b, b"private scope reader b"),
    );
    let budget = ResourceBudget::fixed(2 << 20).unwrap();
    let inputs = [
        ValidationInput::of::<Release>(&["id"]),
        ValidationInput::of::<Package>(&["id"]),
    ];
    let program = ServingScopeProgram::new(&inputs, &[], &[], &budget).unwrap();
    let prepared = PreparedServingScope::new(program, &budget).unwrap();
    let root_charge = budget
        .reserve("shared-reader-scope-roots", 2 * 384)
        .unwrap();
    let root_a = reader::target_id(Target::Entity(EntityId::of(release_a.id())));
    let root_b = reader::target_id(Target::Entity(EntityId::of(release_b.id())));
    let baseline = budget.reserved();
    // The same prepared metadata is reused across readers and again after another reader ran.
    // Both frontier execution and canonical hydration are submitted through the supplied reader.
    let observed: Result<Vec<_>, ModelError> = async {
        let mut observations = Vec::new();
        for (native, root) in [
            (&native_a, &root_a),
            (&native_b, &root_a),
            (&native_b, &root_b),
            (&native_a, &root_a),
        ] {
            let request_charge = budget.reserve("shared-reader-scope-request", 1024)?;
            let next: Vec<surrealdb::types::RecordId> = native
                .query_prepared(prepared.frontier_query(vec![root.clone()])?)
                .await?;
            let mut nodes = vec![root.clone()];
            nodes.extend(next.iter().cloned());
            let batches = prepared.hydrate(native, nodes, &budget).await?;
            observations.push((
                next,
                decode::<Package>(&batches),
                decode::<Release>(&batches),
            ));
            drop(batches);
            drop(request_charge);
        }
        Ok(observations)
    }
    .await;
    let after_requests = budget.reserved();
    drop(prepared);
    drop(root_charge);
    let released = budget.reserved();
    for (client, database) in [(&client_a, &database_a), (&client_b, &database_b)] {
        client
            .query(format!("REMOVE DATABASE {database}"))
            .await
            .unwrap()
            .check()
            .unwrap();
    }
    assert_ne!(native_a.handle().database, native_b.handle().database);
    assert_ne!(native_a.handle().semantic, native_b.handle().semantic);
    let observed = observed.unwrap();
    let package_id =
        |package: &Package| reader::target_id(Target::Entity(EntityId::of(package.id())));
    assert_eq!(
        observed
            .iter()
            .map(|(frontier, _, _)| frontier.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![package_id(&package_a)],
            vec![],
            vec![package_id(&package_b)],
            vec![package_id(&package_a)]
        ],
        "frontier discovery follows the supplied reader database"
    );
    let (mut packages, mut releases) = (Vec::new(), Vec::new());
    for (_, selected_packages, selected_releases) in observed {
        packages.push(selected_packages);
        releases.push(selected_releases);
    }
    assert_eq!(
        packages,
        vec![
            vec![package_a.clone()],
            vec![],
            vec![package_b],
            vec![package_a]
        ]
    );
    assert_eq!(
        releases,
        vec![
            vec![release_a.clone()],
            vec![],
            vec![release_b],
            vec![release_a]
        ]
    );
    assert_eq!(after_requests, baseline);
    assert_eq!(released, 0);
}
