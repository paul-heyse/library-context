//! Canonical input companions under the actual current native scope layout.
use lctx_model::domain::{
    attribution::AnalysisContext,
    catalog::CatalogMember,
    graph::{Assertion, Entity, EntityId, Target},
    input::{CorpusLibrary, DistributionRole, InputDistribution, InputRevision, Package, Release},
    resources::ResourceBudget,
    retrieval::{CorpusText, Family, Origin, RENDER_VERSION, Unit},
    source::{Module, SourceArtifact},
    *,
};
use lctx_surrealdb::reader;
#[path="fixtures/scoped.rs"]
mod scoped;

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
    let config=scoped::config();
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
    let native=scoped::reader(&config,&entities,&assertions).await.unwrap();
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    // Current SCHEMAFULL rows contain canonical body.input and scope_keys, never scope_input.
    let retired:Vec<bool>=native.query(format!("SELECT VALUE scope_input IS NONE FROM entity WHERE semantic_type IN ['source_artifacts','catalog_members','retrieval_units'] AND ({})",native.selected_node_predicate("id")),surrealdb::types::Variables::new()).await.unwrap();
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
    native.close().await.unwrap();
}

#[tokio::test]
async fn shared_serving_template_binds_each_frontier_and_hydration_to_its_exact_view() {
    use lctx_model::domain::serving_scope::ServingScopeProgram;
    use lctx_surrealdb::scope::PreparedServingScope;
    let config=scoped::config();
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
    let native_a=scoped::reader(&config,&[Entity::from(package_a.clone()),Entity::from(release_a.clone())],&[]).await.unwrap();
    let native_b=scoped::reader(&config,&[Entity::from(package_b.clone()),Entity::from(release_b.clone())],&[]).await.unwrap();
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
    assert!(native_b.query_prepared::<Vec<surrealdb::types::RecordId>>(prepared.frontier_query(vec![root_a.clone()]).unwrap()).await.is_err(),"foreign roots are refused before native traversal");
    let observed: Result<Vec<_>, ModelError> = async {
        let mut observations = Vec::new();
        for (native, root) in [
            (&native_a, &root_a),
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
    assert_eq!(native_a.store.database(),native_b.store.database());
    assert_ne!(native_a.store.attempt(),native_b.store.attempt());
    native_a.close().await.unwrap();
    native_b.close().await.unwrap();
    let observed = observed.unwrap();
    let package_id = |package:&Package| lctx_surrealdb::loader::entity_payload_id(&Entity::from(package.clone())).unwrap();
    assert_eq!(
        observed
            .iter()
            .map(|(frontier, _, _)| frontier.clone())
            .collect::<Vec<_>>(),
        vec![
            vec![package_id(&package_a)],
            vec![package_id(&package_b)],
            vec![package_id(&package_a)]
        ],
        "frontier discovery follows the supplied exact reader view"
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
            vec![package_b],
            vec![package_a]
        ]
    );
    assert_eq!(
        releases,
        vec![
            vec![release_a.clone()],
            vec![release_b],
            vec![release_a]
        ]
    );
    assert_eq!(after_requests, baseline);
    assert_eq!(released, 0);
}
