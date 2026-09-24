//! The facet names both languages share (`specs/serving/facets.json`; increment 3's deep review,
//! F4): the `operation_facet` codebook in code order. Python's `FacetName` is held to the same
//! file. `LCTX_WRITE_KNOWN_ANSWERS=1` rewrites it after a codebook append.

use std::path::Path;

use cpg_schema::Codebook;
use cpg_schema::codebook::OperationFacet;

#[test]
fn the_facet_names_are_the_shared_known_answers() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/serving/facets.json");
    let names: Vec<&str> = OperationFacet::all().iter().map(|f| f.text()).collect();
    let want = serde_json::to_string_pretty(&serde_json::json!({
        "_doc": "The operation_facet codebook's names in code order (DESIGN §11.3). Rust writes \
                 this file (LCTX_WRITE_KNOWN_ANSWERS=1); Python's FacetName must equal it.",
        "facets": names,
    }))
    .unwrap()
        + "\n";
    if std::env::var_os("LCTX_WRITE_KNOWN_ANSWERS").is_some() {
        std::fs::write(&path, &want).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), want);
}
