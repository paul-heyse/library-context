//! Diagnostic: a failed selection must not publish a new serving selection.
use lctx_model::domain::{ContentHash, serving::{DatabaseIdentity, Name, SnapshotHandle}};
use lctx_surrealdb::{RuntimeConfig, config::ViewerConfig};
fn main() {
    let dir = tempfile::tempdir().unwrap();
    let selection = dir.path().join("selected.json");
    // A valid directory collision makes the final selection rename fail deterministically.
    std::fs::create_dir(&selection).unwrap();
    let config = RuntimeConfig { endpoint:"grpc://127.0.0.1:1".into(), username:"admin".into(), password:"unused".into(), viewer_username:"viewer".into(), viewer_password:"unused".into(), namespace:Name::new("audit").unwrap(), cache_database:Name::new("cache").unwrap(), selection:selection.clone() };
    let handle = SnapshotHandle { semantic:ContentHash::of(b"audit"), realization:ContentHash::of(b"realization"), database:DatabaseIdentity { namespace:Name::new("audit").unwrap(), database:Name::new("snapshot_audit").unwrap() }};
    let refused = config.select(&handle).is_err();
    let serving = ViewerConfig::read(&selection.with_extension("serving.json")).unwrap();
    let new_pin_published = serving.snapshot == handle;
    println!("{}",serde_json::json!({"probe":"selection_failure","selection_refused":refused,"new_serving_pin_published":new_pin_published,"defect_reproduced":refused && new_pin_published}));
    assert!(refused && new_pin_published);
}
