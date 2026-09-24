//! The serving tokenizer's known answers, shared with Python (`specs/serving/tokens.json`; the
//! holistic assessment's A1(c)).

use std::path::Path;

#[test]
fn the_serving_tokenizer_gives_the_shared_known_answers() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/serving/tokens.json");
    let answers: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let strings = |v: &serde_json::Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_owned())
            .collect()
    };
    for case in answers["text"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        assert_eq!(
            cpg_schema::bundle::tokens(text),
            strings(&case["tokens"]),
            "{text:?}"
        );
    }
    for case in answers["names"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut out = Vec::new();
        cpg_schema::bundle::name_tokens(name, &mut out);
        assert_eq!(out, strings(&case["tokens"]), "{name:?}");
    }
}
