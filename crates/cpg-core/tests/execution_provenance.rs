//! Execution composition belongs to native contributions; supplier identities survive cold state.
use cpg_core::workspace::{Workspace, WorkspaceOptions};
use lctx_model::domain::{
    ContentHash, Record, Relation, ValidatedModel,
    analysis::sources::SourceSnapshot,
    attribution::Provider,
    completed::{CompletedBinding, CompletedContribution},
    graph::Entity,
    input::Package,
    stages::{Profile, ProviderOutcome},
};
use std::{collections::BTreeMap, sync::Arc};

#[path = "fixtures/native.rs"]
mod native_fixture;

fn supplier() -> Provider {
    cpg_extract::ruff_context::provider()
}

fn package() -> Package {
    Package { name: "provenance-package".into() }
}

fn workspace() -> Arc<Workspace> {
    let model = Arc::new(
        ValidatedModel::declared(vec![Relation::of::<Provider>(), Relation::of::<Package>()])
            .unwrap(),
    );
    Workspace::new(model, WorkspaceOptions::default(), native_fixture::store()).unwrap()
}

async fn produce(code: ContentHash, supplier: &Provider) -> Arc<Workspace> {
    let workspace = workspace();
    let output = workspace.output(
        "provenance-output",
        Profile::Catalog,
        code,
        workspace.inputs("provenance-output", Profile::Catalog, []).unwrap(),
        [Provider::NAME, Package::NAME],
    );
    output.declare_async::<Provider>().await.unwrap();
    output.declare_async::<Package>().await.unwrap();
    output.push(supplier.clone()).await.unwrap();
    output.push(package()).await.unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    workspace
}

fn rows<R: Record>(workspace: &Workspace) -> Vec<R> {
    workspace.completed::<R>().unwrap().batches().unwrap()
        .flat_map(|batch| R::decode(&batch.unwrap()).unwrap())
        .collect()
}

async fn contribution(workspace: &Workspace) -> CompletedContribution {
    let mut inventory = workspace.native().contributions().await.unwrap();
    assert_eq!(inventory.len(), 1);
    let contribution = inventory.pop().unwrap();
    assert_eq!(contribution.spec.outputs, [Provider::NAME.into(), Package::NAME.into()].into_iter().collect());
    assert_eq!(contribution.outputs[Provider::NAME].rows, 1);
    assert_eq!(contribution.outputs[Package::NAME].rows, 1);
    contribution
}

#[tokio::test]
async fn ordinary_output_composes_stage_code_without_rewriting_supplier_identity() {
    let supplier = supplier();
    let original_code = supplier.build_digest;
    let changed_code = ContentHash::of(b"changed-stage-implementation/v2");
    let original = produce(original_code, &supplier).await;
    let changed = produce(changed_code, &supplier).await;
    let first = contribution(&original).await;
    let second = contribution(&changed).await;

    assert_ne!(first.spec.implementation, original_code,
        "native execution includes the compiler composition beyond raw supplied code");
    assert_ne!(second.spec.implementation, changed_code);
    assert_ne!(first.spec.implementation, second.spec.implementation,
        "changing stage code changes the persisted execution identity");
    assert_ne!(first.identity().unwrap(), second.identity().unwrap());
    assert_eq!(first.outputs, second.outputs,
        "identical supplier observations retain identical row content");
    for workspace in [&original, &changed] {
        assert_eq!(rows::<Provider>(workspace), vec![supplier.clone()]);
        assert_eq!(rows::<Package>(workspace), vec![package()]);
    }
    assert_eq!(supplier.build_digest, original_code);
    original.drain().await.unwrap();
    changed.drain().await.unwrap();
}

#[tokio::test]
async fn cold_native_restore_retains_foreign_execution_and_exact_supplier_inventory() {
    let supplier = supplier();
    let current = produce(supplier.build_digest, &supplier).await;
    let current_descriptor = contribution(&current).await;
    let current_implementation = current_descriptor.spec.implementation;
    let mut captured = current_descriptor.spec;
    captured.implementation = ContentHash::of(b"captured-foreign-compiler-composition/v1");
    captured.configuration = Some(ContentHash::of(b"captured-foreign-settings"));
    assert_ne!(captured.implementation, current_implementation);
    let mut captured_supplier = supplier.clone();
    captured_supplier.build_digest = ContentHash::of(b"captured-foreign-supplier-implementation/v1");
    assert_ne!(captured_supplier.build_digest, supplier.build_digest);
    let expected_implementation = captured.implementation;
    let source = workspace();
    let relations = [Relation::of::<Provider>(), Relation::of::<Package>()];
    let pending = source.native().begin_contribution(captured.clone()).await.unwrap();
    source.native().write_batch(&pending, &relations[0],
        &Provider::encode(std::slice::from_ref(&captured_supplier)).unwrap()).await.unwrap();
    source.native().write_batch(&pending, &relations[1],
        &Package::encode(&[package()]).unwrap()).await.unwrap();
    let views = source.native().complete_contribution(
        pending, ProviderOutcome::Complete, &relations, &BTreeMap::new(),
    ).await.unwrap();
    for relation in &relations {
        let view = views[relation.name()].clone();
        source.native().bind(CompletedBinding {
            boundary: None,
            source: SourceSnapshot::of_completed_view(relation, source.model().digest(), &view).unwrap(),
            view,
            configuration: None,
        }).await.unwrap();
    }
    source.restore(Profile::Catalog).await.unwrap();
    let expected = contribution(&source).await;
    assert_eq!(expected.spec, captured);
    let bindings = source.native().bindings().await.unwrap();
    let export = tempfile::NamedTempFile::new().unwrap();
    let state = source.native().export_state(export.path()).await.unwrap();

    let restored = workspace();
    // Completed-state transport carries contribution/membership state. Its canonical graph
    // payload is loaded through the independent graph loader before cold state admission.
    lctx_surrealdb::Loader::new(restored.native().shared_client()).entities(&[
        Entity::from(captured_supplier.clone()), Entity::from(package()),
    ]).await.unwrap();
    restored.native().import_state(export.path(), &state).await.unwrap();
    restored.restore(Profile::Catalog).await.unwrap();

    let imported = contribution(&restored).await;
    assert_eq!(imported, expected);
    assert_eq!(imported.spec.implementation, expected_implementation,
        "cold restoration retains captured execution rather than current-build composition");
    assert_eq!(imported.identity().unwrap(), expected.identity().unwrap());
    assert_eq!(restored.native().completed_state().await.unwrap(), state);
    assert_eq!(restored.native().bindings().await.unwrap(), bindings);
    assert_eq!(rows::<Provider>(&restored), vec![captured_supplier]);
    assert_eq!(rows::<Package>(&restored), vec![package()]);
    for relation in &relations {
        assert_eq!(restored.relation(relation.name()).unwrap().view(), &views[relation.name()]);
    }
    current.drain().await.unwrap();
    source.drain().await.unwrap();
    restored.drain().await.unwrap();
}
