//! Actual completed selection with service-free deterministic contract values.
//! These effects validate wiring; they do not qualify a live embedding service.
use cpg_core::{
    analysis_prepare, embedding_realization,
    embedding_service::{Embedder, FakeEmbedder},
    workspace::{Workspace, WorkspaceOptions},
};
use lctx_model::domain::{
    embedding::{
        EmbeddingSpec,
        configuration::{Configuration, ServiceConfiguration},
    },
    stages::*,
    *,
};
use std::sync::Arc;
#[tokio::test]
async fn completed_selection_is_immutable_and_required_for_embedding_effects() {
    let model = Arc::new(ValidatedModel::declared(embedding::configuration_relations()).unwrap());
    let provider = FakeEmbedder::new();
    for selected in [true, false] {
        let workspace = Workspace::new(
            model.clone(),
            WorkspaceOptions {
                memory_bytes: 1 << 24,
                ..Default::default()
            },
        )
        .unwrap();
        let budget = workspace.budget().clone();
        let configuration = selected
            .then(|| Configuration::new(provider.spec(), provider.endpoint(), &budget).unwrap());
        let empty = workspace
            .inputs("embedding_configuration", Profile::Catalog, [])
            .unwrap();
        let output = workspace.output(
            "embedding_configuration",
            Profile::Catalog,
            ContentHash::of(b"configuration"),
            empty.clone(),
        );
        analysis_prepare::embedding_configuration(
            empty.clone(),
            output,
            &model,
            &workspace,
            configuration.as_ref(),
        )
        .await
        .unwrap();
        assert!(
            embedding_realization::Session::open(&empty, &workspace, &model, &provider, None)
                .await
                .is_err(),
            "unselected input set cannot acquire later selection"
        );
        let inputs = workspace
            .inputs(
                "embedding_consumer",
                Profile::Catalog,
                [EmbeddingSpec::NAME, ServiceConfiguration::NAME,embedding::DocumentRecipe::NAME,embedding::projection::ProjectionDefinition::NAME],
            )
            .unwrap();
        let result =
            embedding_realization::Session::open(&inputs, &workspace, &model, &provider, None)
                .await;
        if selected {
            let mut session = result.unwrap();
            let spec = session.configuration().specification().clone();
            let value = session.realize("source declaration").await.unwrap();
            let replay = value.decode(&spec, &budget).unwrap();
            assert_eq!(replay.values().len(), spec.dimensions as usize);
            drop(replay);
            drop(session);
        } else {
            assert!(
                result.is_err(),
                "unselected specification is explicit unavailability"
            );
        }
        let original = workspace.completed::<EmbeddingSpec>().unwrap();
        let duplicate = workspace.output(
            "second_configuration",
            Profile::Catalog,
            ContentHash::of(b"second"),
            empty,
        );
        duplicate.declare::<EmbeddingSpec>().unwrap();
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        let unchanged = workspace.completed::<EmbeddingSpec>().unwrap();
        assert_eq!(unchanged.content(), original.content());
        assert_eq!(unchanged.rows(), original.rows());
        drop(original);
        drop(unchanged);
        drop(inputs);
        drop(configuration);
        drop(workspace);
        assert_eq!(budget.reserved(), 0);
    }
}
