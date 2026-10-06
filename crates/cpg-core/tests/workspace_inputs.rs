//! Publication inputs bind immutable completed streams, including contributed vocabulary.
use cpg_core::workspace::{CompletedInputs, Workspace, WorkspaceOptions};
use lctx_model::domain::{
    input::InputRevision,
    source::{Module, SourceArtifact},
    stages::{Effect, Profile, ProviderOutcome, PublicationBoundary, RelationUse, Stage},
    value::{AccessPath, Place, PlaceRoot},
    *,
};
use std::sync::Arc;

fn consumer(prefix: Option<PublicationBoundary>) -> Stage {
    let input = RelationUse::completed::<Place>();
    Stage {
        name: "place_consumer",
        inputs: vec![prefix.map_or(input, |boundary| input.at_epoch(boundary))],
        outputs: vec![],
        contributes: vec![],
        coverage: vec![],
        profiles: Profile::ALL.to_vec(),
        effect: Effect::Pure,
        code: ContentHash::of(b"place_consumer"),
        configuration: ContentHash::of(b"place_consumer_configuration"),
    }
}

fn place(name: &str) -> Place {
    let input = InputRevision {
        manifest: ContentHash::of(b"frozen_input_fixture"),
    };
    let source =
        SourceArtifact::from_bytes(input.id(), "fixture.py".into(), b"value = 1\n").unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "fixture".into(),
    };
    Place {
        root: PlaceRoot::Global {
            module: module.id(),
            name: name.into(),
        }
        .id(),
        path: AccessPath::empty().id(),
    }
}

async fn contribute(workspace: &Arc<Workspace>, producer: &'static str, value: Place) {
    // This control exercises completed stream selection, rather than claiming admission of
    // an entire source graph. Referenced anchors are outside the selected transport contract.
    let inputs = workspace.inputs(producer, Profile::Catalog, []).unwrap();
    let output = workspace.output(
        producer,
        Profile::Catalog,
        ContentHash::of(producer.as_bytes()),
        inputs,
    );
    output.declare::<Place>().unwrap();
    output.push(value).await.unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
}

fn values(inputs: &CompletedInputs) -> Vec<Place> {
    inputs
        .relation::<Place>()
        .unwrap()
        .batches()
        .unwrap()
        .flat_map(|batch| Place::decode(&batch.unwrap()).unwrap())
        .collect()
}

#[tokio::test]
async fn later_vocabulary_contributions_do_not_widen_declared_facts_inputs() {
    let workspace =
        Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions::default()).unwrap();
    let first = place("native");
    contribute(&workspace, "native_places", first.clone()).await;
    let declaration = consumer(Some(PublicationBoundary::Facts));
    assert!(
        workspace
            .stage_inputs(&declaration, Profile::Catalog)
            .is_err(),
        "latest stream cannot replace an unfrozen boundary"
    );
    workspace.freeze_inputs(PublicationBoundary::Facts).unwrap();
    let before = workspace
        .stage_inputs(&declaration, Profile::Catalog)
        .unwrap();
    let original = before.relation::<Place>().unwrap().clone();

    let later = place("local");
    contribute(&workspace, "local_places", later.clone()).await;
    let frozen = workspace
        .stage_inputs(&declaration, Profile::Catalog)
        .unwrap();
    let latest = workspace
        .stage_inputs(&consumer(None), Profile::Catalog)
        .unwrap();
    assert_eq!(values(&before), vec![first.clone()]);
    assert_eq!(values(&frozen), vec![first.clone()]);
    assert!(Arc::ptr_eq(&original, frozen.relation::<Place>().unwrap()));
    assert_eq!(
        frozen.read::<Place>().unwrap().snapshot(),
        before.read::<Place>().unwrap().snapshot()
    );
    let mut expected = vec![first, later];
    expected.sort_by_key(|row| row.id());
    assert_eq!(values(&latest), expected);
    assert_eq!(
        latest.relation::<Place>().unwrap().producer(),
        "local_places"
    );
    let session = frozen.session(&workspace).await.unwrap();
    let batches = session
        .sql("SELECT * FROM places")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(
        batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
        1
    );
}

#[tokio::test]
async fn missing_relation_at_a_frozen_boundary_cannot_fall_back_to_latest() {
    let workspace =
        Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions::default()).unwrap();
    workspace.freeze_inputs(PublicationBoundary::Facts).unwrap();
    contribute(&workspace, "late_places", place("late")).await;
    assert_eq!(
        workspace
            .stage_inputs(&consumer(None), Profile::Catalog)
            .unwrap()
            .relation::<Place>()
            .unwrap()
            .rows(),
        1
    );
    assert!(
        workspace
            .stage_inputs(
                &consumer(Some(PublicationBoundary::Facts)),
                Profile::Catalog
            )
            .is_err()
    );
}

#[tokio::test]
async fn distinct_completed_views_remain_selectable_together() {
    let workspace =
        Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions::default()).unwrap();
    contribute(&workspace, "native_places", place("native")).await;
    workspace.freeze_inputs(PublicationBoundary::Facts).unwrap();
    contribute(&workspace, "model_places", place("model")).await;
    workspace.freeze_inputs(PublicationBoundary::Model).unwrap();
    let mut declaration = consumer(Some(PublicationBoundary::Facts));
    declaration
        .inputs
        .push(RelationUse::completed::<Place>().at_epoch(PublicationBoundary::Model));
    let inputs = workspace
        .stage_inputs(&declaration, Profile::Catalog)
        .unwrap();
    assert!(
        inputs.read::<Place>().is_err(),
        "an unqualified read must not blend different views"
    );
    let facts = inputs
        .read_at::<Place>(Some(PublicationBoundary::Facts))
        .unwrap();
    let model = inputs
        .read_at::<Place>(Some(PublicationBoundary::Model))
        .unwrap();
    assert_eq!(facts.source().rows(), 1);
    assert_eq!(model.source().rows(), 2);
    assert_ne!(facts.source().content(), model.source().content());
    let session = inputs.session(&workspace).await.unwrap();
    for (boundary, expected) in [
        (PublicationBoundary::Facts, 1),
        (PublicationBoundary::Model, 2),
    ] {
        let table = inputs.table_at::<Place>(Some(boundary)).unwrap();
        let batches = session
            .sql(&format!("SELECT * FROM {table}"))
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(
            batches.iter().map(|batch| batch.num_rows()).sum::<usize>(),
            expected
        );
    }
    assert!(session.sql("SELECT * FROM places").await.is_err());
}
