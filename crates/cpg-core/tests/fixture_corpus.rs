//! Whole fixture inventory over production providers. Registration is explicit: an added case
//! fails this suite until its expectations are assigned. Data is never executed.
use cpg_core::workspace::{Workspace, WorkspaceOptions};
use cpg_extract::{
    acquisition::{AcquiredInput, derive_blocks},
    bundle::CapturedInputs,
    capture::CapturedInput,
};
use lctx_model::domain::{ContentHash, resources::ResourceBudget, stages::Profile};
use std::{collections::BTreeSet, path::Path, sync::Arc};
macro_rules! fixture_cases {
    ($($case:ident),* $(,)?) => {
        const CASES: &[&str] = &[$(stringify!($case)),*];
        $(
            #[tokio::test]
            async fn $case() {
                both_profiles_use_the_real_facts_frontier(stringify!($case)).await;
            }
        )*
    };
}
fixture_cases! {
    behavioral_frontiers,
    capture_observations,
    class_metadata,
    contextual_expected_arguments,
    guarded_origin_inventory,
    module_resolution,
    native_callable_deprecation,
    native_callable_variants,
    native_candidate_inventory,
    native_class_traits,
    native_diagnostics,
    native_exports,
    native_generics,
    native_lexical,
    native_overload_origins,
    native_usage,
    protocol_observations,
    python_reference_oracle,
    ruff_context,
    stable_capture_shapes,
    terminal_question,
    symbolic_fields,
    retired_read_expectations,
    retired_attribute_expectations,
    retired_shadow_boundaries,
    field_dynamic_all,
    field_read_screen,
    analytic_optional,
    entry_value_witnesses,
    transfer_composition,
    dictionary_keys,
    direct_usage,
    execution_channels,
    local_semantics,
    string_values,
    _invalid,
    action_shapes,
    analysis_shapes,
    behavior_shapes,
    catalog,
    catalog_evidence,
    catalog_core,
    catalog_context,
    context_protocol_shapes,
    dep_env,
    derive_cases,
    docs_shapes,
    dunder_all,
    entry_bridge,
    expression_completion_shapes,
    exact_exception_shapes,
    flow_call_paths,
    flow_shapes,
    graph_shapes,
    handler_shapes,
    import_cycle,
    invocation_shapes,
    lexical_shapes,
    model_handler_shapes,
    model_shapes,
    native_signature,
    native_model_context,
    normalized_relations,
    normalized_projections,
    phase4_context_constructors,
    phase4_summaries,
    phase4_models,
    normalized_entities,
    normalized_bindings,
    effective_callables,
    postgres_report_corpus,
    public_shapes,
    pysa_keys,
    pysa_tito_shapes,
    pysa_variants,
    rebuild_matrix,
    return_completion_shapes,
    semantic_declarations,
    semantic_deployment,
    semantic_documents,
    semantic_guards,
    semantic_lexical,
    semantic_owner,
    semantic_shapes,
    semantic_stability,
    semantic_symbols,
    semantic_syntax,
    source_body_shapes,
    structural_usage,
    summary_caps,
    syntax_shapes,
    synthesis_refutation,
    synthesis_sources,
    transfer_alternatives,
    type_guard,
    type_shapes,
    typed_semantics,
    unicode_bom,
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(4 << 30).unwrap()
}
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python")
}
fn capture(case: &str, profile: Profile, resources: &ResourceBudget) -> Arc<CapturedInputs> {
    let root = root().join(case);
    let mut paths = vec![];
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                paths.push(path.strip_prefix(&root).unwrap().display().to_string());
            }
        }
    }
    paths.sort();
    let documents = paths
        .iter()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    let frozen =
        CapturedInput::capture_derived(&root, &paths, resources, &documents, derive_blocks)
            .unwrap();
    Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(frozen, case)],
        cpg_extract::native_context::NativeContextConfig::committed(profile, resources).unwrap(),
    ))
}
#[test]
fn every_fixture_is_registered() {
    let listed = std::fs::read_dir(root())
        .unwrap()
        .map(Result::unwrap)
        .filter(|p| p.path().is_dir())
        .map(|p| p.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(listed, CASES.iter().map(|c| c.to_string()).collect());
}

async fn both_profiles_use_the_real_facts_frontier(case: &str) {
    let mut failures = vec![];
    for profile in Profile::ALL {
        let resources = budget();
        let result = async {
            let workspace = Workspace::with_budget(
                Arc::new(lctx_model::domain::model()?),
                WorkspaceOptions {
                    memory_bytes: resources.limit(),
                    ..Default::default()
                },
                resources.clone(),
            )?;
            cpg_core::facts::compile_facts(
                &workspace,
                &capture(case, profile, &resources),
                profile,
                cpg_core::facts::providers(ContentHash::of(b"fixture-corpus")),
                Default::default(),
            )
            .await?;
            workspace.validate().await?;
            workspace.facts_availability(profile)?;
            workspace.content()
        }
        .await;
        match result {
            Ok(content) => {
                println!("passed {case} {} {}", profile.name(), content.hex());
            }
            Err(error) => failures.push(format!("{case} {}: {error}", profile.name())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn representative_fixtures_preserve_semantics_across_workspace_batching_and_partitions() {
    for case in ["flow_call_paths", "semantic_documents", "type_shapes"] {
        for profile in Profile::ALL {
            let mut contents = Vec::new();
            for (partitions, batch_rows) in [(1, 1), (4, 4096)] {
                let resources = budget();
                let workspace = Workspace::with_budget(
                    Arc::new(lctx_model::domain::model().unwrap()),
                    WorkspaceOptions {
                        memory_bytes: resources.limit(),
                        partitions,
                        batch_rows,
                    },
                    resources.clone(),
                )
                .unwrap();
                cpg_core::facts::compile_facts(
                    &workspace,
                    &capture(case, profile, &resources),
                    profile,
                    cpg_core::facts::providers(ContentHash::of(b"fixture-corpus")),
                    Default::default(),
                )
                .await
                .unwrap();
                workspace.validate().await.unwrap();
                contents.push(workspace.content().unwrap());
                drop(workspace);
                assert_eq!(resources.reserved(), 0);
            }
            assert_eq!(contents[0], contents[1], "{case} {}", profile.name());
        }
    }
}
