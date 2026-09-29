//! Reuse qualification compares every canonical column, including evidence and provenance.
use cpg_core::{
    catalog::CompileInputs,
    rebuild::{Outcome, Stage},
};
use cpg_schema::{
    Id, Table,
    catalog::CompileProfile,
    table::{canonical_sort, rebind_snapshot},
};
use std::path::Path;

async fn neutral<T: Table>(
    ctx: &datafusion::prelude::SessionContext,
    snapshot: Id,
) -> arrow_array::RecordBatch {
    let batches = ctx.table(T::NAME).await.unwrap().collect().await.unwrap();
    let batches = batches
        .iter()
        .map(cpg_core::arrow_types::to_declared::<T>)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let b = arrow_select::concat::concat_batches(&T::schema(), &batches).unwrap();
    rebind_snapshot::<T>(&canonical_sort(&b, T::key()).unwrap(), snapshot, Id::ZERO).unwrap()
}
async fn equal(root: &Path, a: Id, b: Id) {
    let (_, left) = cpg_core::snapshot::published(root, a)
        .await
        .unwrap()
        .unwrap();
    let (_, right) = cpg_core::snapshot::published(root, b)
        .await
        .unwrap()
        .unwrap();
    macro_rules! check {($($t:ty),+)=>{$(assert_eq!(neutral::<$t>(&left,a).await,neutral::<$t>(&right,b).await,"{}",<$t as Table>::NAME);)+};}
    cpg_schema::for_each_table!(check);
    cpg_schema::for_each_derived_table!(check);
    cpg_schema::for_each_analysis_table!(check);
}

async fn changed_inputs_match_clean(case: &str) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    let package = root.join("rebuildpkg");
    std::fs::create_dir_all(&package).unwrap();
    let original = include_str!("../../../fixtures/python/rebuild_matrix/rebuildpkg/__init__.py");
    let source = package.join("__init__.py");
    std::fs::write(&source, original).unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let extract = |snapshot_id| {
        let mut extracted = cpg_extract::extract(&cpg_extract::ExtractInput {
            profile: CompileProfile::Catalog,
            release: cpg_extract::Release::from_tree(root.clone(), "rebuild-change").unwrap(),
            venv_root: temp.path().join("venv"),
            site_packages: vec![site.clone()],
            python_version: (3, 14, 7),
            python_platform: "linux".into(),
            snapshot_id,
            corpus: None,
            keep_pysa_json: false,
            test_hooks: Default::default(),
        })
        .unwrap();
        // Controlled attributed-provider inputs, not a claim to have executed another provider.
        if snapshot_id == Id([162; 16]) {
            use cpg_schema::{
                query::QueryRow,
                tables::{Coverage, CoverageRow, Producers, ProducersRow},
            };
            for (name, batch) in &mut extracted.tables {
                if case == "provider_attribution" && *name == Producers::NAME {
                    let mut rows = ProducersRow::read_batch(batch).unwrap();
                    for row in &mut rows {
                        row.revision.push_str("-qualification");
                    }
                    *batch = Producers::to_sorted_batch(&rows).unwrap();
                }
                if case == "coverage_change" && *name == Coverage::NAME {
                    let mut rows = CoverageRow::read_batch(batch).unwrap();
                    for row in &mut rows {
                        if row.status
                            == cpg_schema::codebook::CoverageStatus::CompleteUnderStatedModel
                        {
                            row.status = cpg_schema::codebook::CoverageStatus::Partial;
                            row.reason =
                                Some(cpg_schema::codebook::BoundaryReason::MissingEvidence);
                            row.detail = Some("bounded qualification input".into());
                        }
                    }
                    *batch = Coverage::to_sorted_batch(&rows).unwrap();
                }
            }
        }
        extracted
    };
    let store = temp.path().join("store");
    let inputs = CompileInputs {
        public_roots: vec!["rebuildpkg".into()],
        profile: CompileProfile::Catalog,
        embedder: None,
        embedding_cache: None,
    };
    let before = Id([161; 16]);
    cpg_core::attempt::compile_catalog(&store, before, extract(before).tables, &inputs, None)
        .await
        .unwrap();
    match case {
        "membership_addition" => std::fs::write(package.join("added.py"), "def newly_present(value):\n    return value\n").unwrap(),
        "membership_deletion" => std::fs::write(&source, original.replace("alias = existing", "")).unwrap(),
        "missing_evidence" => std::fs::write(&source, original.replace("    \"\"\"Return a value.\n\n    Args:\n        value: An attributed parameter description.\n    \"\"\"\n", "")).unwrap(),
        "source_coordinates" => std::fs::write(&source, format!("# coordinate shift\n\n{original}")).unwrap(),
        "provider_attribution" | "coverage_change" => (),
        _ => unreachable!(),
    }
    let changed = Id([162; 16]);
    cpg_core::attempt::compile_catalog(&store, changed, extract(changed).tables, &inputs, None)
        .await
        .unwrap();
    // Seed the previous complete input (including its negative lookups) immediately before reuse.
    let (_, previous) =
        cpg_core::rebuild::catalog(&store, before, Id([163; 16]), &inputs, None, true)
            .await
            .unwrap();
    let (rebuilt, receipt) =
        cpg_core::rebuild::catalog(&store, changed, Id([164; 16]), &inputs, None, false)
            .await
            .unwrap();
    for stage in [Stage::ContractNormalization, Stage::ContextualAssociation] {
        let now = receipt.steps.iter().find(|s| s.stage == stage).unwrap();
        let prior = previous.steps.iter().find(|s| s.stage == stage).unwrap();
        assert_ne!(now.key, prior.key, "{case}: {stage:?}");
        assert_eq!(now.outcome, Outcome::Recomputed, "{case}: {stage:?}");
    }
    equal(&store, changed, rebuilt.snapshot_id).await;
}

macro_rules! change_test {
    ($name:ident) => {
        #[tokio::test]
        async fn $name() {
            changed_inputs_match_clean(stringify!($name)).await;
        }
    };
}
change_test!(membership_addition);
change_test!(membership_deletion);
change_test!(missing_evidence);
change_test!(source_coordinates);
change_test!(provider_attribution);
change_test!(coverage_change);

#[tokio::test]
async fn behavioral_rebuild_preserves_provenance_and_invalidates_selected_policy() {
    let temp = tempfile::tempdir().unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/rebuild_matrix")
        .canonicalize()
        .unwrap();
    let source = Id([171; 16]);
    let extracted = cpg_extract::extract(&cpg_extract::ExtractInput {
        profile: CompileProfile::Behavioral,
        release: cpg_extract::Release::from_tree(root, "behavioral-rebuild").unwrap(),
        venv_root: temp.path().join("venv"),
        site_packages: vec![site],
        python_version: (3, 14, 7),
        python_platform: "linux".into(),
        snapshot_id: source,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap();
    let config = "version = 1\n[subsystem]\nmodule_prefixes = [\"rebuildpkg\"]\npublic_roots = [\"rebuildpkg\"]\n[seeds]\nprimary = [\"rebuildpkg.existing\"]\ndistractors = []\n[pass_a]\nmax_depth = 2\nmax_vertices = 128\nmax_edges = 512\nmax_witnesses = 3\n[briefs]\nbudget = 2\n";
    let mut analysis = cpg_core::analyze::Analysis {
        config: lctx_analytics::config::AnalyticsConfig::parse(config).unwrap(),
        techniques: Default::default(),
        embedder: None,
        embedding_cache: None,
    };
    let store = temp.path().join("store");
    let inputs = CompileInputs::from_analysis(Some(&analysis));
    cpg_core::attempt::compile_catalog(&store, source, extracted.tables, &inputs, Some(&analysis))
        .await
        .unwrap();
    let (reused, receipt) = cpg_core::rebuild::catalog(
        &store,
        source,
        Id([172; 16]),
        &inputs,
        Some(&analysis),
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::BehavioralEnrichment)
            .unwrap()
            .outcome,
        Outcome::Reused
    );
    let (clean, _) = cpg_core::rebuild::catalog(
        &store,
        source,
        Id([173; 16]),
        &inputs,
        Some(&analysis),
        true,
    )
    .await
    .unwrap();
    equal(&store, source, reused.snapshot_id).await;
    equal(&store, clean.snapshot_id, reused.snapshot_id).await;
    analysis.config = lctx_analytics::config::AnalyticsConfig::parse(
        &config.replace("max_depth = 2", "max_depth = 1"),
    )
    .unwrap();
    let (changed, receipt) = cpg_core::rebuild::catalog(
        &store,
        source,
        Id([174; 16]),
        &inputs,
        Some(&analysis),
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::BehavioralEnrichment)
            .unwrap()
            .outcome,
        Outcome::Recomputed
    );
    let (oracle, _) = cpg_core::rebuild::catalog(
        &store,
        source,
        Id([175; 16]),
        &inputs,
        Some(&analysis),
        true,
    )
    .await
    .unwrap();
    equal(&store, changed.snapshot_id, oracle.snapshot_id).await;
}

#[tokio::test]
async fn catalog_rebuild_clean_reused_and_corrupt_disposable_cache() {
    let temp = tempfile::tempdir().unwrap();
    let site = temp.path().join("venv/site-packages");
    std::fs::create_dir_all(&site).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/catalog")
        .canonicalize()
        .unwrap();
    let source = Id([151; 16]);
    let extracted = cpg_extract::extract(&cpg_extract::ExtractInput {
        profile: CompileProfile::Catalog,
        release: cpg_extract::Release::from_tree(root, "catalog-rebuild").unwrap(),
        venv_root: temp.path().join("venv"),
        site_packages: vec![site],
        python_version: (3, 14, 7),
        python_platform: "linux".into(),
        snapshot_id: source,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap();
    let store = temp.path().join("store");
    let inputs = CompileInputs {
        public_roots: vec!["catalogpkg".into()],
        profile: CompileProfile::Catalog,
        embedder: None,
        embedding_cache: None,
    };
    cpg_core::attempt::compile_catalog(&store, source, extracted.tables, &inputs, None)
        .await
        .unwrap();
    let (rebuilt, receipt) =
        cpg_core::rebuild::catalog(&store, source, Id([152; 16]), &inputs, None, false)
            .await
            .unwrap();
    for stage in [Stage::ContractNormalization, Stage::ContextualAssociation] {
        assert_eq!(
            receipt
                .steps
                .iter()
                .find(|s| s.stage == stage)
                .unwrap()
                .outcome,
            Outcome::Reused
        );
    }
    equal(&store, source, rebuilt.snapshot_id).await;
    let (clean, receipt) =
        cpg_core::rebuild::catalog(&store, source, Id([153; 16]), &inputs, None, true)
            .await
            .unwrap();
    assert!(
        receipt
            .steps
            .iter()
            .filter(|s| matches!(
                s.stage,
                Stage::ContractNormalization | Stage::ContextualAssociation
            ))
            .all(|s| s.outcome == Outcome::Recomputed)
    );
    equal(&store, clean.snapshot_id, rebuilt.snapshot_id).await;
    std::fs::write(
        store.join("rebuild-cache/association/catalog_associations.arrow"),
        b"corrupt disposable artifact",
    )
    .unwrap();
    let (repaired, receipt) =
        cpg_core::rebuild::catalog(&store, source, Id([154; 16]), &inputs, None, false)
            .await
            .unwrap();
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::ContractNormalization)
            .unwrap()
            .outcome,
        Outcome::Reused
    );
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::ContextualAssociation)
            .unwrap()
            .outcome,
        Outcome::Recomputed
    );
    equal(&store, clean.snapshot_id, repaired.snapshot_id).await;
    let cache = store.join("rebuild-cache/normalization");
    let path = cache.join("catalog_parameters.arrow");
    let b = arrow_ipc::reader::FileReader::try_new(std::fs::File::open(&path).unwrap(), None)
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    use cpg_schema::query::QueryRow;
    let mut parameters = cpg_schema::catalog::CatalogParametersRow::read_batch(&b).unwrap();
    parameters[0].name = Some("incorrect_cached_name".into());
    let b = cpg_schema::catalog::CatalogParameters::to_sorted_batch(&parameters).unwrap();
    let mut bytes = Vec::new();
    let mut writer = arrow_ipc::writer::FileWriter::try_new(&mut bytes, &b.schema()).unwrap();
    writer.write(&b).unwrap();
    writer.finish().unwrap();
    drop(writer);
    std::fs::write(&path, &bytes).unwrap();
    use sha2::{Digest as _, Sha256};
    let manifest = cache.join("CURRENT.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    value["files"]["catalog_parameters"] = format!("{:x}", Sha256::digest(&bytes)).into();
    std::fs::write(manifest, serde_json::to_vec(&value).unwrap()).unwrap();
    let (repaired, receipt) =
        cpg_core::rebuild::catalog(&store, source, Id([158; 16]), &inputs, None, false)
            .await
            .unwrap();
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::ContractNormalization)
            .unwrap()
            .outcome,
        Outcome::Recomputed
    );
    equal(&store, clean.snapshot_id, repaired.snapshot_id).await;
    // A scope change must not inherit membership from a previously complete domain.
    let narrow = CompileInputs {
        public_roots: vec!["catalogpkg.child".into()],
        ..inputs
    };
    let (changed, receipt) =
        cpg_core::rebuild::catalog(&store, source, Id([155; 16]), &narrow, None, false)
            .await
            .unwrap();
    assert_eq!(
        receipt
            .steps
            .iter()
            .find(|s| s.stage == Stage::ContractNormalization)
            .unwrap()
            .outcome,
        Outcome::Recomputed
    );
    let (oracle, _) =
        cpg_core::rebuild::catalog(&store, source, Id([156; 16]), &narrow, None, true)
            .await
            .unwrap();
    equal(&store, changed.snapshot_id, oracle.snapshot_id).await;
    // An invalid profile promotion cannot fabricate missing captured flow facts.
    let bad = CompileInputs {
        profile: CompileProfile::Behavioral,
        ..narrow
    };
    assert!(
        cpg_core::rebuild::catalog(&store, source, Id([157; 16]), &bad, None, false)
            .await
            .unwrap_err()
            .to_string()
            .contains("captured flow facts")
    );
    assert!(
        cpg_core::snapshot::published(&store, Id([157; 16]))
            .await
            .unwrap()
            .is_none()
    );
    let out = temp.path().join("generations");
    let (_, receipt) = cpg_core::rebuild::retrieval(&store, source, &out, None, None)
        .await
        .unwrap();
    assert_eq!(receipt.source, receipt.snapshot);
    assert_eq!(receipt.steps.len(), 1);
    assert_eq!(receipt.steps[0].stage, Stage::Retrieval);
}
