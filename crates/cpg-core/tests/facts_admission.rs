//! The production frontier admits every exact coverage scope and never invents a ty run in catalog.
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use lctx_model::domain::{
    admission::*, attribution::*, resources::ResourceBudget, stages::Profile, *,
};
use std::sync::Arc;
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn captured(source: &[u8], document: bool) -> Arc<CapturedInputs> {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("api.py"), source).unwrap();
    let mut paths = vec!["api.py".into()];
    if document {
        std::fs::write(root.path().join("README.md"), b"# API\n\nRead `api.f`.\n").unwrap();
        paths.push("README.md".into());
    }
    Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(
        CapturedInput::capture(root.path(), &paths, &budget()).unwrap(),
        "facts-admission",
    )]))
}
#[tokio::test]
async fn complete_frontier_is_admitted_with_profile_owned_availability() {
    for profile in Profile::ALL {
        let admission = cpg_core::facts::memory(
            captured(b"def f(x):\n    return x\n", true),
            budget(),
            profile,
            ContentHash::of(b"fixture"),
        )
        .await
        .unwrap();
        assert_eq!(admission.profile(), profile);
        assert_eq!(
            admission.availability()[&FactFamily::Artifacts],
            Availability::Complete
        );
        assert_eq!(
            admission.availability()[&FactFamily::Docs],
            Availability::Complete
        );
        assert_eq!(
            admission.availability()[&FactFamily::Flow],
            if profile == Profile::Catalog {
                Availability::NotRequested
            } else {
                Availability::Complete
            }
        );
    }
    let (_, memory, _) = cpg_core::facts::inspect(
        captured(b"def f(x): return x\n", false),
        budget(),
        Profile::Catalog,
    )
    .await
    .unwrap();
    let model = model().unwrap();
    assert!(
        !memory
            .read::<Provider>(&model, &budget())
            .unwrap()
            .rows()
            .iter()
            .any(|p| p.tool == "ty")
    );
    assert!(
        !memory
            .read::<ProviderCoverage>(&model, &budget())
            .unwrap()
            .rows()
            .iter()
            .filter(|c| c.family == FactFamily::Flow)
            .any(|c| c.provider.is_some() || c.run.is_some())
    );
}
#[tokio::test]
async fn syntax_failure_and_no_document_scope_have_distinct_availability() {
    let admission = cpg_core::facts::memory(
        captured(b"def f(:\n", false),
        budget(),
        Profile::Catalog,
        ContentHash::of(b"fixture"),
    )
    .await
    .unwrap();
    assert_eq!(
        admission.availability()[&FactFamily::Syntax],
        Availability::Partial
    );
    assert_eq!(
        admission.availability()[&FactFamily::Docs],
        Availability::NoScope
    );
}
