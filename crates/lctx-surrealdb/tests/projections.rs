//! Real native ownership selection with source isolates, a self-loop and coverage gaps.
use lctx_model::domain::{
    attribution::{AnalysisContext, CoverageStatus, FactFamily, ProviderCoverage},
    graph::{Assertion, Entity, Manifest, ProjectionDefinition},
    input::{InputRevision, ManifestEntry},
    normalized::entities::{CallableEntity, CallableKind, EntityRef, OccurrenceOwnership},
    projection::{
        ArcId, ProjectionName,
        normalization::{ProjectionData, ProjectionKey, describe},
    },
    resources::ResourceBudget,
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    source::{CoverageScope, Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind},
    *,
};
use lctx_surrealdb::{Credentials, Loader, NativeReader, reader};
use surrealdb::types::{Bytes, Variables};

#[tokio::test]
async fn named_projection_preserves_native_universe_arcs_and_gap_metadata() {
    let path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("owned disposable SurrealDB configuration required");
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "gn_controls";
    let db = format!("projection_{}", std::process::id());
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        ns,
        &db,
    )
    .await
    .unwrap();
    client
        .query(format!(
            "DEFINE NAMESPACE IF NOT EXISTS {ns}; DEFINE DATABASE OVERWRITE {db} STRICT;"
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let loader = Loader::new(client.clone());
    loader.install("").await.unwrap();
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"projection-fixture"),
        environment_digest: ContentHash::of(b"projection-fixture"),
        lock_digest: None,
    };
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "graph.py".into(),
        content: ContentHash::of(&[b' '; 64]),
        byte_len: 64,
    }])
    .unwrap();
    let artifact = SourceArtifact::from_bytes(input.id(), "graph.py".into(), &[b' '; 64]).unwrap();
    let scope = CoverageScope::Input { input: input.id() };
    let occurrences: Vec<_> = (0..3)
        .map(|i| Occurrence {
            source: artifact.id(),
            start: i,
            end: i + 1,
            syntax_kind: SyntaxKind::ExprCall,
            role: OccurrenceRole::Call,
            structural_path: vec![i as i32],
        })
        .collect();
    let refs: Vec<_> = occurrences
        .iter()
        .map(|o| EntityRef::Occurrence { occurrence: o.id() })
        .collect();
    let callable = CallableEntity::Source {
        declaration: occurrences[1].id(),
        kind: CallableKind::Function,
    };
    let callable_ref = EntityRef::Callable {
        callable: callable.id(),
    };
    let owners: Vec<_> = (0..2)
        .map(|i| OccurrenceOwnership {
            occurrence: occurrences[i].id(),
            owner: occurrences[0].id(),
            entity: refs[0].id(),
        })
        .collect();
    let coverage = ProviderCoverage {
        scope: scope.id(),
        provider: None,
        context: context.id(),
        family: FactFamily::Artifacts,
        run: None,
        status: CoverageStatus::NotRequested,
        reason: None,
        diagnostic: None,
    };
    let key = ProjectionKey {
        input: input.id(),
        context: context.id(),
        name: ProjectionName::DefinitionContainment,
    };
    let mut data = ProjectionData::new(&budget);
    data.artifacts.insert(artifact.clone()).unwrap();
    data.scopes.insert(scope.clone()).unwrap();
    data.coverage.insert(coverage.clone()).unwrap();
    for o in &occurrences {
        data.occurrences.insert(o.clone()).unwrap();
    }
    for r in &refs {
        data.refs.insert(r.clone()).unwrap();
    }
    data.refs.insert(callable_ref.clone()).unwrap();
    data.callables.insert(callable.clone()).unwrap();
    for owner in &owners {
        data.owners.insert(owner.clone()).unwrap();
    }
    let expected = describe(&data, key, &budget).unwrap();
    assert_eq!(expected.vertex_count(), 4);
    assert_eq!(expected.arc_count(), 3);
    assert_eq!(expected.gaps().count(), 4);
    let mut entities = vec![
        Entity::from(input.clone()),
        Entity::from(context.clone()),
        Entity::from(artifact),
        Entity::from(scope),
        Entity::from(callable),
        Entity::from(callable_ref),
    ];
    entities.extend(occurrences.iter().cloned().map(Entity::from));
    entities.extend(refs.iter().cloned().map(Entity::from));
    // Same context, distinct input: the exported universe must not absorb its isolate or coverage.
    let foreign_input = InputRevision::from_entries(vec![ManifestEntry {
        path: "other.py".into(),
        content: ContentHash::of(b"x"),
        byte_len: 1,
    }])
    .unwrap();
    let foreign_artifact =
        SourceArtifact::from_bytes(foreign_input.id(), "other.py".into(), b"x").unwrap();
    let foreign_occurrence = Occurrence {
        source: foreign_artifact.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprCall,
        role: OccurrenceRole::Call,
        structural_path: vec![0],
    };
    entities.extend([
        Entity::from(foreign_input),
        Entity::from(foreign_artifact),
        Entity::from(foreign_occurrence.clone()),
        Entity::from(EntityRef::Occurrence {
            occurrence: foreign_occurrence.id(),
        }),
    ]);
    let mut assertions = owners
        .iter()
        .cloned()
        .map(Assertion::from_record)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assertions.push(Assertion::from_record(coverage).unwrap());
    assertions.push(Assertion::from_record(expected.assessment()).unwrap());
    loader.entities(&entities).await.unwrap();
    loader.entity_references(&entities).await.unwrap();
    loader.assertions(&assertions).await.unwrap();
    loader.assertion_references(&assertions).await.unwrap();
    let definition = ProjectionDefinition {
        name: "DefinitionContainment".into(),
        definition: analysis::ProjectionDefinition::builtin(key.name).content_digest(),
        source_membership: ContentHash::of(b"fixture-selected-projection-inputs"),
        declared_losses: vec!["fixture declared loss".into()],
    };
    let manifest = Manifest {
        format_version: 6,
        frontier: admission::Frontier::Normalized,
        profile: stages::Profile::Catalog,
        captures: vec![graph::EntityId::of(input.id())],
        semantic_contract: ContentHash::of(b"fixture-contract"),
        producers: vec![],
        settings: ContentHash::of(b"fixture-settings"),
        families: vec![],
        required_outcomes: vec![],
        outcomes: vec![],
        originals: vec![],
        projections: vec![definition.clone()],
        embeddings: vec![],
    };
    let handle = SnapshotHandle {
        semantic: manifest.content(),
        realization: lctx_surrealdb::schema::realization_identity(""),
        database: DatabaseIdentity {
            namespace: Name::new(ns).unwrap(),
            database: Name::new(&db).unwrap(),
        },
    };
    let mut bindings = Variables::new();
    bindings.insert("handle", hex::encode(serde_json::to_vec(&handle).unwrap()));
    bindings.insert(
        "manifest",
        Bytes::from(serde_json::to_vec(&manifest).unwrap()),
    );
    client
        .query("CREATE publication:current CONTENT {handle:$handle,manifest:$manifest}")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let reader = NativeReader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        handle.clone(),
    )
    .await
    .unwrap();
    let projection = lctx_surrealdb::projections::materialize(&reader, key, &budget)
        .await
        .unwrap();
    projection.graph.matches(&expected).unwrap();
    assert_eq!(
        projection.source.coverage().collect::<Vec<_>>(),
        expected.coverage().collect::<Vec<_>>()
    );
    assert_eq!(
        projection.source.gaps().collect::<Vec<_>>(),
        expected.gaps().collect::<Vec<_>>()
    );
    assert!(projection.graph.outgoing(refs[2].id()).unwrap().is_empty());
    let self_loop = projection
        .graph
        .arcs()
        .find(|a| a.id == ArcId::Containment(owners[0].id()))
        .unwrap();
    assert_eq!(self_loop.source, self_loop.target);
    let mut exported = vec![];
    projection.write_json(&mut exported).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&exported).unwrap();
    assert_eq!(json["snapshot"], serde_json::to_value(handle).unwrap());
    assert_eq!(
        json["definition"],
        serde_json::to_value(definition).unwrap()
    );
    assert_eq!(json["vertices"].as_array().unwrap().len(), 4);
    assert_eq!(json["arcs"].as_array().unwrap().len(), 3);
    assert_eq!(json["gaps"].as_array().unwrap().len(), 4);
    let missing = ProjectionKey {
        input: input.id(),
        context: key.context,
        name: ProjectionName::PublicExposure,
    };
    assert!(
        lctx_surrealdb::projections::materialize(&reader, missing, &budget)
            .await
            .is_err()
    );
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}
