use lctx_model::domain::embedding::Embedder;
fn specs() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/embedding")
}

/// The fake embedder's spec and vectors are known answers the Python twin is held to (slice 1.8):
/// its spec as canonical JSON, and the leading components of the vectors of the conformance
/// inputs' request texts. `LCTX_WRITE_KNOWN_ANSWERS=1` rewrites them after a deliberate change.
#[test]
fn the_fake_embedder_is_a_committed_known_answer() {
    use cpg_core::embedding_service::FakeEmbedder;
    let fake = FakeEmbedder::new();
    let spec = format!("{}\n", fake.spec().canonical_json());
    let inputs: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(specs().join("conformance_inputs.json")).unwrap(),
    )
    .unwrap();
    let mut vectors = serde_json::Map::new();
    for input in &inputs {
        let text = input["text"].as_str().unwrap();
        let request = if input["kind"] == "query" {
            fake.spec().query_text(text)
        } else {
            fake.spec().document_text(text)
        };
        let v = fake.vector(&request);
        vectors.insert(
            input["id"].as_str().unwrap().to_owned(),
            serde_json::json!({ "request": request, "head": &v[..16] }),
        );
    }
    let answers = serde_json::to_string_pretty(&serde_json::Value::Object(vectors)).unwrap() + "\n";
    if std::env::var_os("LCTX_WRITE_KNOWN_ANSWERS").is_some() {
        std::fs::write(specs().join("lctx-fake-embedder.json"), &spec).unwrap();
        std::fs::write(specs().join("fake_vectors.json"), &answers).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(specs().join("lctx-fake-embedder.json")).unwrap(),
        spec
    );
    assert_eq!(
        std::fs::read_to_string(specs().join("fake_vectors.json")).unwrap(),
        answers
    );
}

