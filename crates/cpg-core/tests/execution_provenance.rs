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
    Package {
        name: "provenance-package".into(),
    }
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
        workspace
            .inputs("provenance-output", Profile::Catalog, [])
            .unwrap(),
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
    workspace
        .completed::<R>()
        .unwrap()
        .batches()
        .unwrap()
        .flat_map(|batch| R::decode(&batch.unwrap()).unwrap())
        .collect()
}

async fn contribution(workspace: &Workspace) -> CompletedContribution {
    let mut inventory = workspace
        .native()
        .expect("actual native fixture owner")
        .contributions(workspace.budget())
        .await
        .unwrap();
    assert_eq!(inventory.len(), 1);
    let contribution = inventory.pop().unwrap();
    assert_eq!(
        contribution.spec.outputs,
        [Provider::NAME.into(), Package::NAME.into()]
            .into_iter()
            .collect()
    );
    assert_eq!(contribution.outputs[Provider::NAME].rows, 1);
    assert_eq!(contribution.outputs[Package::NAME].rows, 1);
    contribution
}

fn assert_owner_hold_plan(
    plan: &lctx_native::surrealdb::types::Value,
    parent: &lctx_native::surrealdb::types::RecordId,
) {
    use lctx_native::surrealdb::types::{ToSql, Value};
    fn visit(node: &Value, expected: &str, scans: &mut usize) {
        let object = node.as_object().expect("native inventory plan object");
        let Some(Value::String(operator)) = object.get("operator") else {
            panic!("native inventory plan operator: {node:?}");
        };
        match operator.as_str() {
            "IndexScan" => {
                *scans += 1;
                let attributes = object.get("attributes").unwrap().as_object().unwrap();
                assert_eq!(
                    attributes.get("index"),
                    Some(&Value::String("owner_holds".into())),
                    "{node:?}"
                );
                assert_eq!(
                    attributes.get("access"),
                    Some(&Value::String(expected.into())),
                    "exact owner prefix: {node:?}"
                );
                assert_eq!(
                    attributes.get("direction"),
                    Some(&Value::String("Forward".into())),
                    "{node:?}"
                );
                assert!(
                    object.get("children").is_none(),
                    "one native index leaf: {node:?}"
                );
            }
            "SelectProject" | "Sort" | "SortByKey" | "Filter" => {
                let Some(Value::Array(children)) = object.get("children") else {
                    panic!("native inventory plan children: {node:?}");
                };
                assert_eq!(children.len(), 1, "one owner inventory branch: {node:?}");
                for child in children.iter() {
                    visit(child, expected, scans);
                }
            }
            _ => panic!("unexpected native inventory operator: {node:?}"),
        }
    }
    let expected = Value::Array(vec![Value::RecordId(parent.clone())].into()).to_sql();
    let mut scans = 0;
    visit(plan, &expected, &mut scans);
    assert_eq!(scans, 1, "exactly one owner prefix scan: {plan:?}");
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

    assert_ne!(
        first.spec.implementation, original_code,
        "native execution includes the compiler composition beyond raw supplied code"
    );
    assert_ne!(second.spec.implementation, changed_code);
    assert_ne!(
        first.spec.implementation, second.spec.implementation,
        "changing stage code changes the persisted execution identity"
    );
    assert_ne!(first.identity().unwrap(), second.identity().unwrap());
    assert_eq!(
        first.outputs, second.outputs,
        "identical supplier observations retain identical row content"
    );
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
    captured_supplier.build_digest =
        ContentHash::of(b"captured-foreign-supplier-implementation/v1");
    assert_ne!(captured_supplier.build_digest, supplier.build_digest);
    let expected_implementation = captured.implementation;
    let source = workspace();
    let relations = [Relation::of::<Provider>(), Relation::of::<Package>()];
    let pending = source
        .native()
        .expect("actual native fixture owner")
        .begin_contribution(captured.clone())
        .await
        .unwrap();
    source
        .native()
        .expect("actual native fixture owner")
        .write_batch(
            &pending,
            &relations[0],
            &Provider::encode(std::slice::from_ref(&captured_supplier)).unwrap(),
            source.budget(),
        )
        .await
        .unwrap();
    source
        .native()
        .expect("actual native fixture owner")
        .write_batch(
            &pending,
            &relations[1],
            &Package::encode(&[package()]).unwrap(),
            source.budget(),
        )
        .await
        .unwrap();
    let views = source
        .native()
        .expect("actual native fixture owner")
        .complete_contribution(
            pending,
            ProviderOutcome::Complete,
            &relations,
            &BTreeMap::new(),
            source.budget(),
        )
        .await
        .unwrap();
    for relation in &relations {
        let view = views[relation.name()].clone();
        source
            .native()
            .expect("actual native fixture owner")
            .bind(CompletedBinding {
                boundary: None,
                source: SourceSnapshot::of_completed_view(relation, source.model().digest(), &view)
                    .unwrap(),
                view,
                configuration: None,
            })
            .await
            .unwrap();
    }
    source.restore(Profile::Catalog).await.unwrap();
    let expected = contribution(&source).await;
    assert_eq!(expected.spec, captured);
    let mut bindings = source
        .native()
        .expect("actual native fixture owner")
        .bindings()
        .await
        .unwrap();
    let export = tempfile::NamedTempFile::new().unwrap();
    source
        .native()
        .expect("actual native fixture owner")
        .freeze_content(source.budget())
        .await
        .unwrap();
    {
        // Earn the real compound parent, then EXPLAIN both unchanged production
        // inventory selectors under this source's actual client, pool and scope.
        let physical = source
            .native()
            .expect("actual native fixture owner")
            .complete_physical_content(source.budget())
            .await
            .unwrap();
        let parent = physical.completion().record().unwrap();
        let scope = source
            .native()
            .expect("actual native fixture owner")
            .recovery_scope()
            .unwrap();
        let reader =
            lctx_native::NativeReader::private(scope.client().clone()).with_budget(source.budget());
        let result = lctx_native::control::with_recovery_scope(scope, async {
            for sql in [
                lctx_native::completed_state::COMPOUND_BINDING_POINTERS_SQL,
                lctx_native::completed_state::COMPOUND_PARENT_HOLDS_SQL,
            ] {
                let mut bindings = lctx_native::surrealdb::types::Variables::new();
                bindings.insert("compound_parent", parent.clone());
                let plan: lctx_native::surrealdb::types::Value = reader
                    .query_native(&format!("{sql} EXPLAIN"), bindings)
                    .await?;
                assert_owner_hold_plan(&plan, &parent);
            }
            Ok::<_, lctx_model::domain::ModelError>(())
        })
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        // This private reader borrows the source session; close drains only its
        // query operations. The workspace retains the actual session owner.
        completion.step(
            "compound inventory explain reader drainage",
            reader.close().await,
        );
        lctx_model::domain::completion::complete(result, completion).unwrap();
    }
    let state = source
        .native()
        .expect("actual native fixture owner")
        .export_state(export.path(), source.budget())
        .await
        .unwrap();

    let restored = workspace();
    // Completed-state transport carries contribution/membership state. Its canonical graph
    // payload is loaded through the graph loader before cold state admission, using the
    // restored attempt's actual owned session and resource pool.
    let recovery = restored
        .native()
        .expect("actual native fixture owner")
        .recovery_scope()
        .unwrap();
    let client = recovery.client().clone();
    lctx_native::control::with_recovery_scope(recovery, async {
        lctx_native::Loader::for_attempt_views(
            client,
            restored
                .native()
                .expect("actual native fixture owner")
                .attempt(),
            vec![],
        )
        .with_budget(restored.budget())
        .entities(&[
            Entity::from(captured_supplier.clone()),
            Entity::from(package()),
        ])
        .await
    })
    .await
    .unwrap();
    restored
        .native()
        .expect("actual native fixture owner")
        .import_state(export.path(), &state, restored.budget())
        .await
        .unwrap();
    restored.restore(Profile::Catalog).await.unwrap();

    let imported = contribution(&restored).await;
    assert_eq!(imported, expected);
    assert_eq!(
        imported.spec.implementation, expected_implementation,
        "cold restoration retains captured execution rather than current-build composition"
    );
    assert_eq!(imported.identity().unwrap(), expected.identity().unwrap());
    restored
        .native()
        .expect("actual native fixture owner")
        .freeze_content(restored.budget())
        .await
        .unwrap();
    assert_eq!(
        restored
            .native()
            .expect("actual native fixture owner")
            .completed_state(restored.budget())
            .await
            .unwrap(),
        state.semantic
    );
    let mut restored_bindings = restored
        .native()
        .expect("actual native fixture owner")
        .bindings()
        .await
        .unwrap();
    // Native iteration orders physical IDs derived from each attempt. Exact logical
    // inventory comparison uses binding keys while retaining every full descriptor.
    bindings.sort_by_key(CompletedBinding::key);
    restored_bindings.sort_by_key(CompletedBinding::key);
    assert_eq!(restored_bindings, bindings);
    assert_eq!(rows::<Provider>(&restored), vec![captured_supplier]);
    assert_eq!(rows::<Package>(&restored), vec![package()]);
    for relation in &relations {
        assert_eq!(
            restored.relation(relation.name()).unwrap().view(),
            &views[relation.name()]
        );
    }
    for attempt in [current, source, restored] {
        let budget = attempt.budget().clone();
        let native = attempt
            .native()
            .expect("actual native fixture owner")
            .clone();
        let connection = native
            .recovery_scope()
            .unwrap()
            .client()
            .connection_completion()
            .unwrap();
        attempt.drain().await.unwrap();
        native.abandon().await.unwrap();
        drop(attempt);
        drop(native);
        connection.wait().await.unwrap();
        drop(connection);
        assert_eq!(budget.reserved(), 0);
    }
}
