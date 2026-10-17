use crate::inspector;
use crate::typed_driver;
use lctx_model::domain::{calls::ProviderSymbol, captures::*, *};
use typed_driver::{files, rows};
inspector!(
    Captures,
    lctx_model::domain::captures::CaptureObservation,
    lctx_model::domain::captures::CaptureSupport,
    lctx_model::domain::calls::ProviderSymbol
);
async fn assert_native_declaring_identity_without_value_or_timing_claims(behavioral: bool) {
    let input = files("capture_observations");
    let tables = typed_driver::Tables::default();
    if behavioral {
        typed_driver::run_behavioral(&input, Captures(tables.clone()))
            .await
            .unwrap();
    } else {
        typed_driver::run(&input, Captures(tables.clone()))
            .await
            .unwrap();
    }
    let captures = rows::<CaptureObservation>(&tables);
    let symbols = rows::<ProviderSymbol>(&tables);
    let symbol = |id| symbols.iter().find(|s| s.id() == id).unwrap();
    for name in ["value", "local"] {
        assert!(captures.iter().any(|c| c.name == name
            && symbol(c.function).name == "inner"
            && c.origin == CaptureOrigin::OuterFunction
            && c.declaring.is_some_and(|d| symbol(d).name == "stable")));
    }
    assert!(captures.iter().any(|c| c.name == "GLOBAL"
        && symbol(c.function).name == "inner"
        && c.origin == CaptureOrigin::Global
        && c.declaring.is_none()));
    assert!(captures.iter().any(|c| c.name == "cell"
        && symbol(c.function).name == "reader"
        && c.declaring.is_some_and(|d| symbol(d).name == "late")));
    assert!(captures.iter().any(|c| c.name == "cell"
        && symbol(c.function).name == "writer"
        && c.declaring.is_some_and(|d| symbol(d).name == "mutable")));
    assert!(
        captures
            .iter()
            .all(|c| c.mutable.is_none() && c.timing == CaptureTiming::Unknown)
    );
    assert_eq!(rows::<CaptureSupport>(&tables).len(), captures.len());
}

#[tokio::test]
async fn catalog_captures_have_native_declaring_identity_without_value_or_timing_claims() {
    assert_native_declaring_identity_without_value_or_timing_claims(false).await;
}

#[tokio::test]
async fn behavioral_captures_have_native_declaring_identity_without_value_or_timing_claims() {
    assert_native_declaring_identity_without_value_or_timing_claims(true).await;
}
