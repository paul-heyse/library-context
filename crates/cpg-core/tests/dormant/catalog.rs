// DORMANT P5: Catalog serving manifest/projection expectations over a supplied generation.
// Public contracts, scan-order, classification and typed publication controls moved to C0/C1/C2.
#[path = "catalog/pr4.rs"]
mod pr4;
use std::path::Path;

pub fn catalog_profile_publishes_original_contracts_without_native_analysis(generation_dir: &Path) {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation_dir.join("MANIFEST.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["capabilities"]["catalog"], true);
    assert_eq!(manifest["capabilities"]["native_value_paths"], false);
    assert!(
        manifest["projection"]["artifacts"]
            .get("conditions.arrow")
            .is_none()
    );
    assert!(manifest["condition_kernel_format"].is_null());
    assert_eq!(cpg_core::bundle::verify(generation_dir).unwrap(), manifest);
    pr4::check(generation_dir);
}
