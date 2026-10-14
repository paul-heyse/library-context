//! Actual captured bytes with explicit first-party distribution ownership.
use lctx_model::domain::*;
use std::sync::Arc;
pub fn try_library_fixture_tree(
    tree: Arc<cpg_extract::bundle::CapturedInputs>,
    library: &str,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<Arc<cpg_extract::bundle::CapturedInputs>, ModelError> {
    use cpg_extract::{
        acquisition::{
            AcquiredInput, Acquisition, InventoryDistribution, InventoryFile, LibraryInventory,
            derive_blocks,
        },
        bundle::CapturedInputs,
        capture::CapturedInput,
    };
    let original = tree
        .inputs()
        .first()
        .ok_or(ModelError::Conflict(
            "library fixture requires an actual captured input",
        ))?
        .captured();
    let derived = original
        .derivations()
        .iter()
        .map(|d| d.path())
        .collect::<std::collections::BTreeSet<_>>();
    let paths = original
        .artifacts()
        .iter()
        .filter(|a| !derived.contains(a.path.as_str()))
        .map(|a| a.path.clone())
        .collect::<Vec<_>>();
    let documents = paths
        .iter()
        .filter(|p| p.ends_with(".md") || p.ends_with(".mdx"))
        .cloned()
        .collect::<Vec<_>>();
    let captured =
        CapturedInput::capture_derived(original.root(), &paths, budget, &documents, derive_blocks)
            .map_err(|error| ModelError::Cause(Box::new(error)))?;
    let files = paths
        .into_iter()
        .filter_map(|path| {
            admission::ArtifactClass::of(&path).map(|class| InventoryFile {
                path,
                owners: vec![library.into()],
                role: match class {
                    admission::ArtifactClass::PythonSource => input::SourceRole::Release,
                    admission::ArtifactClass::Document => input::SourceRole::Document,
                },
                record_sha256: None,
            })
        })
        .collect();
    let inventory = LibraryInventory {
        name: library.into(),
        requirement: format!("{library}==0.0.0"),
        lock_digest: ContentHash::of(b"native-serving-first-party-fixture"),
        installer: None,
        python_version: "3.14.7".into(),
        platform: "linux".into(),
        site_packages: captured.root().to_owned(),
        distributions: vec![InventoryDistribution {
            name: library.into(),
            version: "0.0.0".into(),
            first_party: true,
            artifact_sha256: vec![],
            record_digest: captured.revision().manifest,
        }],
        files,
        configuration: ContentHash::of(b"native-serving-first-party-fixture/v1"),
    };
    Ok(Arc::new(CapturedInputs::new(
        vec![AcquiredInput::new(
            captured,
            Acquisition::Installed(inventory),
        )],
        tree.config().clone(),
    )))
}

#[allow(
    dead_code,
    reason = "Shared fixture module; native_journey uses this infallible wrapper"
)]
pub fn library_fixture_tree(
    tree: Arc<cpg_extract::bundle::CapturedInputs>,
    library: &str,
    budget: &resources::ResourceBudget,
) -> Arc<cpg_extract::bundle::CapturedInputs> {
    try_library_fixture_tree(tree, library, budget).unwrap()
}
