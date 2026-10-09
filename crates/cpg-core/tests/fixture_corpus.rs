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
            mod $case {
                use super::*;
                #[tokio::test]
                async fn catalog() {
                    run_profile(stringify!($case), Profile::Catalog).await;
                }
                #[tokio::test]
                async fn behavioral() {
                    run_profile(stringify!($case), Profile::Behavioral).await;
                }
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
    remediation_scopes,
    scoped_source_execution,
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
    ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
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

// Every profile owns its own budget, capture and native workspace. Separate tests retain
// the same two provider jobs while reporting each independent result and deadline separately.
async fn run_profile(case: &str, profile: Profile) {
    let resources = budget();
    let result = async {
        let workspace = Workspace::with_budget(
            Arc::new(lctx_model::domain::model()?),
            WorkspaceOptions {
                memory_bytes: resources.limit(),
                ..Default::default()
            },
            resources.clone(),
            crate::native_fixture::store(),
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
        assert_registered_contract(case, &workspace)?;
        workspace.identity()
    }
    .await;
    match result {
        Ok(content) => println!("passed {case} {} {}", profile.name(), content.hex()),
        Err(error) => panic!("{case} {}: {error}", profile.name()),
    }
}

// These cases exercise scoped declarations and closure capture shapes. The Facts registry
// checks their exact original sources and top-level declarations; behavioral producer controls
// separately own SourceCall/Enriched and supported-vs-refused capture semantics.
fn assert_registered_contract(
    case: &str,
    workspace: &Workspace,
) -> Result<(), lctx_model::domain::ModelError> {
    use lctx_model::domain::{
        ModelError, Record,
        source::{Occurrence, SourceArtifact},
        syntax::DeclarationObservation,
    };
    fn rows<R: Record>(workspace: &Workspace) -> Result<Vec<R>, ModelError> {
        let mut rows = Vec::new();
        for batch in workspace.completed::<R>()?.batches()? {
            rows.extend(R::decode(&batch?)?);
        }
        Ok(rows)
    }
    let expected: &[(&str, &[&str])] = match case {
        "remediation_scopes" => &[
            ("alpha.py", &["Alpha", "alpha"]),
            ("beta.py", &["Beta", "beta"]),
            ("empty.py", &[]),
        ],
        "scoped_source_execution" => &[
            (
                "cases.py",
                &[
                    "literal_argument",
                    "held_argument",
                    "documented_argument",
                    "unavailable_default",
                    "enriched_argument",
                ],
            ),
            ("other.py", &["independent_argument"]),
        ],
        "stable_capture_shapes" => &[(
            "cases.py",
            &[
                "captured_entry",
                "captured_literal",
                "mutation",
                "call_before_assignment",
                "escaped",
                "delayed",
                "nonlocal_write",
                "global_read",
                "loop_capture",
                "nested_scope",
            ],
        )],
        _ => return Ok(()),
    };
    let artifacts = rows::<SourceArtifact>(workspace)?;
    let occurrences = rows::<Occurrence>(workspace)?
        .into_iter()
        .map(|row| (row.id(), row))
        .collect::<std::collections::BTreeMap<_, _>>();
    let declarations = rows::<DeclarationObservation>(workspace)?;
    for (path, names) in expected {
        let sources = artifacts
            .iter()
            .filter(|source| source.path == *path)
            .collect::<Vec<_>>();
        assert_eq!(sources.len(), 1, "{case}: exact original source {path}");
        let source = sources[0];
        let bytes = std::fs::read(root().join(case).join(path)).map_err(ModelError::codec)?;
        assert_eq!(
            source.content,
            ContentHash::of(&bytes),
            "{case}: immutable bytes for {path}"
        );
        assert_eq!(
            source.byte_len,
            i64::try_from(bytes.len()).unwrap(),
            "{case}: original length for {path}"
        );
        let mut actual = BTreeSet::new();
        for declaration in declarations
            .iter()
            .filter(|declaration| declaration.parent.is_none())
        {
            let name = occurrences
                .get(&declaration.name)
                .expect("declaration name retains its canonical occurrence");
            if name.source == source.id() {
                let spelling = std::str::from_utf8(&bytes[name.start as usize..name.end as usize])
                    .map_err(ModelError::codec)?;
                actual.insert(spelling.to_owned());
            }
        }
        assert_eq!(
            actual,
            names.iter().map(|name| (*name).to_owned()).collect(),
            "{case}: exact top-level declarations for {path}"
        );
    }
    if case == "remediation_scopes" {
        let documents = artifacts
            .iter()
            .filter(|source| source.path == "guide.md")
            .collect::<Vec<_>>();
        assert_eq!(
            documents.len(),
            1,
            "independent repeated alpha examples retain their original document"
        );
        let guide = std::fs::read(root().join(case).join("guide.md")).map_err(ModelError::codec)?;
        assert_eq!(documents[0].content, ContentHash::of(&guide));
        let blocks = rows::<lctx_model::domain::documents::DocumentNode>(workspace)?;
        assert_eq!(
            blocks
                .iter()
                .filter(|node| matches!(
                    node,
                    lctx_model::domain::documents::DocumentNode::CodeBlock { .. }
                ))
                .count(),
            2,
            "both repeated alpha examples retain distinct document blocks"
        );
    }
    Ok(())
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
                    crate::native_fixture::store(),
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
                contents.push(workspace.identity().unwrap());
                drop(workspace);
                assert_eq!(resources.reserved(), 0);
            }
            assert_eq!(contents[0], contents[1], "{case} {}", profile.name());
        }
    }
}

#[path = "fixtures/native.rs"]
mod native_fixture;
