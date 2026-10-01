//! Whole fixture inventory over production providers. Registration is explicit: an added case
//! fails this suite until its expectations are assigned. Data is never executed.
use cpg_extract::{
    acquisition::{AcquiredInput, derive_blocks},
    bundle::CapturedInputs,
    capture::CapturedInput,
};
use lctx_model::domain::{ContentHash, resources::ResourceBudget, stages::Profile};
use std::{collections::BTreeSet, path::Path, sync::Arc};
const CASES: &[&str] = &[
    "dictionary_keys",
    "direct_usage",
    "string_values",
    "_invalid",
    "action_shapes",
    "analysis_shapes",
    "behavior_shapes",
    "catalog",
    "catalog_evidence",
    "catalog_core",
    "catalog_context",
    "context_protocol_shapes",
    "dep_env",
    "derive_cases",
    "docs_shapes",
    "dunder_all",
    "entry_bridge",
    "expression_completion_shapes",
    "flow_call_paths",
    "flow_shapes",
    "graph_shapes",
    "handler_shapes",
    "import_cycle",
    "invocation_shapes",
    "lexical_shapes",
    "model_handler_shapes",
    "model_shapes",
    "native_signature",
    "native_model_context",
    "normalized_relations",
    "normalized_projections",
    "normalized_entities",
    "normalized_bindings",
    "effective_callables",
    "postgres_report_corpus",
    "public_shapes",
    "pysa_keys",
    "pysa_tito_shapes",
    "pysa_variants",
    "rebuild_matrix",
    "return_completion_shapes",
    "semantic_declarations",
    "semantic_deployment",
    "semantic_documents",
    "semantic_guards",
    "semantic_lexical",
    "semantic_owner",
    "semantic_shapes",
    "semantic_stability",
    "semantic_symbols",
    "semantic_syntax",
    "source_body_shapes",
    "summary_caps",
    "syntax_shapes",
    "transfer_alternatives",
    "type_guard",
    "type_shapes",
    "typed_semantics",
    "unicode_bom",
];
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
        CapturedInput::capture_derived(&root, &paths, &budget(), &documents, derive_blocks)
            .unwrap();
    Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(frozen, case)], cpg_extract::native_context::NativeContextConfig::committed(profile, resources).unwrap()))
}
#[tokio::test]
async fn every_fixture_is_registered_and_both_profiles_use_the_real_facts_frontier() {
    let listed = std::fs::read_dir(root())
        .unwrap()
        .map(Result::unwrap)
        .filter(|p| p.path().is_dir())
        .map(|p| p.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(listed, CASES.iter().map(|c| c.to_string()).collect());
    let mut failures = vec![];
    for case in CASES {
        for profile in Profile::ALL {
            let resources = budget();
            let result = cpg_core::facts::memory(
                capture(case,profile,&resources),
                resources.clone(),
                profile,
                ContentHash::of(b"fixture-corpus"),
            )
            .await;
            match result {
                Ok(admission) => {
                    assert_eq!(admission.profile(), profile);
                    println!(
                        "passed {case} {} {}",
                        profile.name(),
                        admission.content().hex()
                    );
                }
                Err(error) => failures.push(format!("{case} {}: {error}", profile.name())),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn representative_fixtures_have_equal_memory_and_postgresql_content() {
    use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
    let db = DisposableDatabase::start().await;
    let store = GenerationStore::install(
        db.owner.clone(),
        Arc::new(lctx_model::domain::model().unwrap()),
    )
    .await
    .unwrap();
    for case in ["flow_call_paths", "semantic_documents", "type_shapes"] {
        for profile in Profile::ALL {
            let resources = budget();
            let configuration = ContentHash::of(b"fixture-corpus");
            let memory = cpg_core::facts::memory(capture(case,profile,&resources), resources.clone(), profile, configuration)
                .await
                .unwrap();
            let published = cpg_core::facts::publish(
                &store,
                db.writer.clone(),
                capture(case,profile,&resources),
                resources.clone(),
                profile,
                configuration,
            )
            .await
            .unwrap();
            assert_eq!(
                published.content,
                memory.content(),
                "{case} {}",
                profile.name()
            );
            assert_eq!(published.availability, *memory.availability());
            store.retire(published.generation).await.unwrap();
        }
    }
}
