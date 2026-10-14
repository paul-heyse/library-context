//! Publication inputs bind immutable completed streams, including contributed vocabulary.
use cpg_core::{
    embedding_service::FakeEmbedder,
    workspace::{CompletedInputs, Workspace, WorkspaceOptions},
};
use futures::FutureExt;
use lctx_model::domain::embedding::Embedder;
use lctx_model::domain::embedding::{
    EmbeddingSpec,
    projection::{ProjectedValue, ProjectionDefinition},
    value::{AdmittedValue, FullValue, encode_vector, value_digest},
};
use lctx_model::domain::{
    input::InputRevision,
    source::{Module, SourceArtifact},
    stages::{Effect, Profile, ProviderOutcome, PublicationBoundary, RelationUse, Stage},
    value::{AccessPath, Place, PlaceRoot},
    *,
};
use std::{future::Future, panic::AssertUnwindSafe, sync::Arc};

// These controls own their source attempts. Drain the workspace before abandoning
// the attempt, and settle its signal with the combined actual finality even when
// an assertion panics. Operation refusals remain independently observed diagnostics;
// actual execution/session finality must succeed without filtering its failures.
async fn with_workspace<F, Fut>(options: WorkspaceOptions, expected_poison: bool, check: F)
where
    F: FnOnce(Arc<Workspace>) -> Fut,
    Fut: Future<Output = ()>,
{
    let native = crate::native_fixture::store();
    let mut workspace = None;
    let result = AssertUnwindSafe(async {
        let current = Workspace::new(Arc::new(model().unwrap()), options, native.clone()).unwrap();
        workspace = Some(current.clone());
        check(current).await;
    })
    .catch_unwind()
    .await;
    let mut completion = match &workspace {
        Some(workspace) => workspace.drain_owned_report().await,
        None => completion::Completion::default(),
    };
    if completion.local == completion::LocalState::Terminal
        && completion.remote == completion::RemoteState::Confirmed
        && completion
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup())
    {
        let mut source = native.abandon_finality_report().await;
        if source.local == completion::LocalState::Outstanding {
            completion.local = completion::LocalState::Outstanding;
        }
        if source.remote == completion::RemoteState::Unknown {
            completion.remote = completion::RemoteState::Unknown;
        }
        completion.storage.append(&mut source.storage);
        completion.committed.append(&mut source.committed);
        completion.failures.append(&mut source.failures);
    } else {
        completion
            .storage
            .push(completion::StorageState::Orphan(format!(
                "native_attempt:{}",
                native.attempt().hex()
            )));
    }
    if let Some(workspace) = &workspace {
        let signal = workspace.complete_signal(&completion).await;
        completion.step("workspace-input control workspace signal", signal);
    }
    let diagnostics = native.drain_report().await;
    if let Err(panic) = result {
        if let Err(error) = completion::complete(Ok(()), completion) {
            eprintln!("workspace-input control finalization after assertion panic: {error:#?}");
        }
        if !diagnostics.failures.is_empty() {
            eprintln!(
                "workspace-input control operation diagnostics after assertion panic: {diagnostics:#?}"
            );
        }
        std::panic::resume_unwind(panic);
    }
    completion::complete(Ok(()), completion).unwrap();
    assert_eq!(
        !diagnostics.failures.is_empty(),
        expected_poison,
        "failed selected assembly must poison final admission: {diagnostics:?}"
    );
    if expected_poison {
        assert_eq!(
            diagnostics.local,
            completion::LocalState::Terminal,
            "{diagnostics:?}"
        );
        assert_eq!(
            diagnostics.remote,
            completion::RemoteState::Confirmed,
            "{diagnostics:?}"
        );
        assert!(diagnostics.storage.is_empty(), "{diagnostics:?}");
        assert!(
            !diagnostics.failures.is_empty(),
            "the refusal must survive finalization"
        );
        assert!(
            diagnostics
                .failures
                .iter()
                .all(|failure| expected_pointer_conflict(&failure.error)),
            "unexpected operation diagnostics: {diagnostics:?}"
        );
        eprintln!(
            "expected selected-assembly refusal retained after actual finalization: {diagnostics:#?}"
        );
    } else {
        completion::complete(Ok(()), diagnostics).unwrap();
    }
}

fn expected_pointer_conflict(error: &ModelError) -> bool {
    match error {
        ModelError::Completion(outcome) => {
            outcome.completion.local == completion::LocalState::Terminal
                && outcome.completion.remote == completion::RemoteState::Confirmed
                && outcome.completion.storage.is_empty()
                && (outcome.primary.is_some() || !outcome.completion.failures.is_empty())
                && outcome
                    .primary
                    .as_deref()
                    .is_none_or(expected_pointer_conflict)
                && outcome
                    .completion
                    .failures
                    .iter()
                    .all(|failure| expected_pointer_conflict(&failure.error))
        }
        ModelError::SharedCause(error) => expected_pointer_conflict(error),
        _ => matches!(
            error.primary(),
            Some(ModelError::Conflict(
                "conflicting nominal candidate pointer"
            ))
        ),
    }
}

fn consumer(prefix: Option<PublicationBoundary>) -> Stage {
    let input = RelationUse::completed::<Place>();
    Stage {
        captured_binding: None,
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
        [<Place>::NAME],
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
    with_workspace(WorkspaceOptions::default(), false, |workspace| async move {
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
    })
    .await;
}

#[tokio::test]
async fn missing_relation_at_a_frozen_boundary_cannot_fall_back_to_latest() {
    with_workspace(WorkspaceOptions::default(), false, |workspace| async move {
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
    })
    .await;
}

#[tokio::test]
async fn distinct_completed_views_remain_selectable_together() {
    with_workspace(WorkspaceOptions::default(), false, |workspace| async move {
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
        assert_ne!(facts.source().view(), model.source().view());
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
    })
    .await;
}

// Use the production record codec and nominal keys. Referenced encoder/policy anchors are
// outside this selected completed-view control, as source anchors are in the Place controls.
fn value_pair(workspace: &Workspace, input: &str, payload: &str) -> (FullValue, ProjectedValue) {
    let embedder = FakeEmbedder::new();
    let encoder = EmbeddingSpec::new(embedder.spec()).unwrap();
    let policy = ProjectionDefinition::initial(embedder.spec());
    let admitted = AdmittedValue::new(
        embedder.spec(),
        input,
        1,
        &embedder.vector(payload),
        workspace.budget(),
    )
    .unwrap();
    let full = FullValue::new(&encoder, &admitted).unwrap();
    let projected = ProjectedValue::new(&full, &policy).unwrap();
    (full, projected)
}

fn value_consumer(boundary: PublicationBoundary) -> Stage {
    let mut declaration = consumer(None);
    declaration.name = "value_consumer";
    declaration.inputs = vec![
        RelationUse::completed::<FullValue>().at_epoch(boundary),
        RelationUse::completed::<ProjectedValue>().at_epoch(boundary),
    ];
    declaration
}
async fn publish_values(
    workspace: &Arc<Workspace>,
    producer: &'static str,
    rows: &[(&str, &str)],
) -> Result<(), ModelError> {
    let inputs = workspace.inputs(producer, Profile::Catalog, [])?;
    let output = workspace.output(
        producer,
        Profile::Catalog,
        ContentHash::of(producer.as_bytes()),
        inputs,
        [<FullValue>::NAME, <ProjectedValue>::NAME],
    );
    output.declare::<FullValue>()?;
    output.declare::<ProjectedValue>()?;
    for (input, payload) in rows {
        let (full, projected) = value_pair(workspace, input, payload);
        output.push(full).await?;
        output.push(projected).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}

#[tokio::test]
async fn canonical_values_merge_exactly_and_preserve_analytic_and_retrieval_views() {
    with_workspace(
        WorkspaceOptions {
            batch_rows: 1,
            ..Default::default()
        },
        true,
        |workspace| async move {
            publish_values(
                &workspace,
                "e1",
                &[("shared", "original"), ("analytic", "first")],
            )
            .await
            .unwrap();
            let analytic = value_consumer(PublicationBoundary::AnalyticEmbedding);
            assert!(workspace.stage_inputs(&analytic, Profile::Catalog).is_err());
            workspace
                .freeze_inputs(PublicationBoundary::AnalyticEmbedding)
                .unwrap();
            let before = workspace.stage_inputs(&analytic, Profile::Catalog).unwrap();
            let original = before.relation::<FullValue>().unwrap().clone();
            publish_values(
                &workspace,
                "e0",
                &[("shared", "original"), ("retrieval", "second")],
            )
            .await
            .unwrap();
            workspace
                .freeze_inputs(PublicationBoundary::Retrieval)
                .unwrap();
            let frozen = workspace.stage_inputs(&analytic, Profile::Catalog).unwrap();
            let latest = workspace
                .stage_inputs(
                    &value_consumer(PublicationBoundary::Retrieval),
                    Profile::Catalog,
                )
                .unwrap();
            assert!(Arc::ptr_eq(
                &original,
                frozen.relation::<FullValue>().unwrap()
            ));
            assert_eq!(frozen.relation::<FullValue>().unwrap().rows(), 2);
            assert_eq!(frozen.relation::<ProjectedValue>().unwrap().rows(), 2);
            assert_eq!(latest.relation::<FullValue>().unwrap().rows(), 3);
            assert_eq!(latest.relation::<ProjectedValue>().unwrap().rows(), 3);
            let content = latest.relation::<FullValue>().unwrap().view_identity();
            let error = publish_values(&workspace, "conflict", &[("shared", "changed")])
                .await
                .unwrap_err();
            assert!(
                matches!(
                    error.primary(),
                    Some(ModelError::Conflict(
                        "conflicting nominal candidate pointer"
                    ))
                ),
                "selected canonical assembly must refuse unequal payloads: {error}"
            );
            assert_eq!(
                workspace.relation(FullValue::NAME).unwrap().view_identity(),
                content
            );
            assert_eq!(
                frozen.relation::<FullValue>().unwrap().view_identity(),
                original.view_identity()
            );
        },
    )
    .await;
}

#[tokio::test]
async fn projected_value_conflicts_preserve_the_completed_view() {
    with_workspace(
        WorkspaceOptions {
            batch_rows: 1,
            ..Default::default()
        },
        true,
        |workspace| async move {
            publish_values(&workspace, "e1", &[("shared", "original")])
                .await
                .unwrap();
            workspace
                .freeze_inputs(PublicationBoundary::Retrieval)
                .unwrap();
            let frozen = workspace
                .stage_inputs(
                    &value_consumer(PublicationBoundary::Retrieval),
                    Profile::Catalog,
                )
                .unwrap();
            let projected_content = frozen.relation::<ProjectedValue>().unwrap().view_identity();
            let full_content = frozen.relation::<FullValue>().unwrap().view_identity();
            let (_, mut competing) = value_pair(&workspace, "shared", "original");
            // Keep the exact nominal value/policy key while presenting different valid unit bytes.
            let values = competing
                .values()
                .unwrap()
                .into_iter()
                .map(|value| -value)
                .collect::<Vec<_>>();
            competing.bytes = EvidenceBytes(encode_vector(&values));
            competing.digest = value_digest(&values);
            competing.validate().unwrap();
            let output = workspace.output(
                "projection_conflict",
                Profile::Catalog,
                ContentHash::of(b"projection_conflict"),
                workspace
                    .inputs("projection_conflict", Profile::Catalog, [])
                    .unwrap(),
                [FullValue::NAME, ProjectedValue::NAME],
            );
            output.declare::<FullValue>().unwrap();
            output.declare::<ProjectedValue>().unwrap();
            // The full-value aggregate changes validly before the projected-value union
            // refuses. No member of this output set may replace a current binding early.
            let (fresh, _) = value_pair(&workspace, "new full before projected refusal", "fresh");
            output.push(fresh).await.unwrap();
            let result = match output.push(competing).await {
                Ok(()) => output.finish(ProviderOutcome::Complete).await,
                Err(error) => Err(error),
            };
            let error = result.unwrap_err();
            assert!(
                matches!(
                    error.primary(),
                    Some(ModelError::Conflict(
                        "conflicting nominal candidate pointer"
                    ))
                ),
                "selected canonical assembly must refuse unequal payloads: {error}"
            );
            assert_eq!(
                workspace
                    .relation(ProjectedValue::NAME)
                    .unwrap()
                    .view_identity(),
                projected_content
            );
            assert_eq!(
                frozen.relation::<ProjectedValue>().unwrap().view_identity(),
                projected_content
            );
            assert_eq!(
                workspace.relation(FullValue::NAME).unwrap().view_identity(),
                full_content
            );
            assert_eq!(
                frozen.relation::<FullValue>().unwrap().view_identity(),
                full_content
            );
        },
    )
    .await;
}

#[path = "fixtures/native.rs"]
mod native_fixture;
