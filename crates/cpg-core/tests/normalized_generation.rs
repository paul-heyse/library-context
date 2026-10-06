//! Cumulative normalization is repeatable and retains explicit scope outcomes.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
#[tokio::test]
async fn normalized_is_self_contained_and_repeatable() {
    let mut previous = None;
    for profile in [Profile::Catalog, Profile::Behavioral, Profile::Behavioral] {
        let fixture = catalog_runtime::compile(
            "normalized_projections",
            profile,
            Frontier::Normalized,
            catalog_runtime::settings("graph"),
            None,
        )
        .await;
        if profile == Profile::Behavioral {
            let content = fixture.workspace.content().unwrap();
            if let Some(previous) = previous {
                assert_eq!(previous, content);
            }
            previous = Some(content);
        }
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture,"SELECT (SELECT count(*) FROM projection_snapshots) AS fixture_column_0,(SELECT count(*) FROM call_binding_attempts) AS fixture_column_1,(SELECT count(*) FROM source_artifacts) AS fixture_column_2").await;
        assert_eq!(counts.0, 4);
        assert!(counts.1 > 0);
        assert_eq!(counts.2, 2);
        let availability:Vec<i16>=catalog_runtime::query(&fixture,"SELECT c.availability FROM normalization_coverage c JOIN normalization_computations n ON c.computation=n.id WHERE n.capability=10").await;
        assert_eq!(
            availability.len(),
            2,
            "one flow outcome per Python artifact"
        );
        assert!(
            availability
                .iter()
                .all(|a| (*a == 3) == (profile == Profile::Catalog))
        );
    }
}
#[tokio::test]
async fn empty_captured_scope_completes_explicit_no_scope_and_empty_snapshots() {
    use cpg_core::{
        compilation,
        workspace::{Workspace, WorkspaceOptions},
    };
    use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
    use std::sync::Arc;
    let workspace =
        Workspace::new(Arc::new(model().unwrap()), WorkspaceOptions::default()).unwrap();
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(
            CapturedInput::capture(
                &catalog_runtime::root("normalized_projections"),
                &[],
                workspace.budget(),
            )
            .unwrap(),
            "empty-normalized",
        )],
        cpg_extract::native_context::NativeContextConfig::committed(
            Profile::Catalog,
            workspace.budget(),
        )
        .unwrap(),
    ));
    compilation::compile(
        &workspace,
        captured,
        Profile::Catalog,
        ContentHash::of(b"empty-normalized"),
        Frontier::Normalized,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    workspace.validate().await.unwrap();
    let completed = workspace.completed_relations().unwrap();
    let inputs = workspace
        .inputs(
            "inspect",
            Profile::Catalog,
            completed.iter().map(|r| r.name()),
        )
        .unwrap();
    let fixture = catalog_runtime::Fixture {
        session: inputs.session(&workspace).await.unwrap(),
        workspace,
    };
    let outcomes:Vec<(i16,i16)>=catalog_runtime::query(&fixture,"SELECT capability,availability FROM normalization_computations WHERE capability IN (0,8,10,16) ORDER BY capability").await;
    assert_eq!(outcomes, vec![(0, 4), (8, 4), (10, 3), (16, 3)]);
    let count: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM projection_source_assessments WHERE vertices=0 AND arcs=0",
    )
    .await;
    assert_eq!(count, 4);
}
