//! The vLLM client without a GPU (DESIGN §11.1; ADR-0010): the committed spec is canonical, the
//! request bodies for the shared conformance inputs are exactly the committed ones (the Python
//! client is held to the same file, E2), every §11.1 rejection fires, and a stub HTTP server
//! stands in for vLLM to exercise the real `reqwest` path.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

use cpg_core::embed::Embedder;
use lctx_embed::{QWEN_SPEC, VllmEmbedder, parse_embeddings, qwen_spec, request_body};

fn specs() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/embedding")
}

#[test]
fn the_committed_spec_is_its_canonical_form() {
    let spec = qwen_spec();
    assert_eq!(format!("{}\n", spec.canonical_json()), QWEN_SPEC);
    assert_eq!(spec.dimensions, 4096);
    assert_eq!(spec.max_document_tokens, 2048);
}

/// The request bodies both clients must build byte for byte (E2): regenerate with
/// `LCTX_WRITE_BODIES=1 cargo test -p lctx-embed` after a deliberate spec change.
#[test]
fn request_bodies_match_the_committed_conformance_file() {
    let spec = qwen_spec();
    let inputs: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(specs().join("conformance_inputs.json")).unwrap(),
    )
    .unwrap();
    let mut bodies = serde_json::Map::new();
    for i in &inputs {
        let text = i["text"].as_str().unwrap();
        let request = match i["kind"].as_str().unwrap() {
            "query" => spec.query_text(text),
            _ => spec.document_text(text),
        };
        let body = String::from_utf8(request_body(&spec, &[request])).unwrap();
        bodies.insert(i["id"].as_str().unwrap().to_owned(), body.into());
    }
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::Value::Object(bodies)).unwrap()
    );
    let path = specs().join("request_bodies.json");
    if std::env::var_os("LCTX_WRITE_BODIES").is_some() {
        std::fs::write(&path, &rendered).unwrap();
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), rendered);
}

fn unit(dims: usize) -> Vec<f64> {
    vec![1.0 / (dims as f64).sqrt(); dims]
}

fn response(model: &str, data: &[(usize, Vec<f64>)]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "object": "list",
        "model": model,
        "data": data.iter().map(|(i, v)| serde_json::json!({
            "object": "embedding", "index": i, "embedding": v
        })).collect::<Vec<_>>(),
    }))
    .unwrap()
}

#[test]
fn every_rejection_fires() {
    let spec = qwen_spec();
    let m = spec.model.clone();
    let ok = response(&m, &[(1, unit(4096)), (0, unit(4096))]);
    assert_eq!(parse_embeddings(&spec, 2, &ok).unwrap().len(), 2);
    assert!(
        parse_embeddings(&spec, 3, &ok)
            .unwrap_err()
            .contains("2 vectors for 3")
    );
    let other = response("other/model", &[(0, unit(4096))]);
    assert!(
        parse_embeddings(&spec, 1, &other)
            .unwrap_err()
            .contains("model")
    );
    let twice = response(&m, &[(0, unit(4096)), (0, unit(4096))]);
    assert!(
        parse_embeddings(&spec, 2, &twice)
            .unwrap_err()
            .contains("twice")
    );
    let range = response(&m, &[(5, unit(4096))]);
    assert!(
        parse_embeddings(&spec, 1, &range)
            .unwrap_err()
            .contains("out of range")
    );
    let short = response(&m, &[(0, unit(8))]);
    assert!(
        parse_embeddings(&spec, 1, &short)
            .unwrap_err()
            .contains("dimensions")
    );
    let long = response(&m, &[(0, vec![0.5; 4096])]);
    assert!(
        parse_embeddings(&spec, 1, &long)
            .unwrap_err()
            .contains("norm")
    );
}

/// Each request a stub served: its path and body.
type Seen = Vec<(String, Vec<u8>)>;

/// Serve `replies` in order on a local port, recording each request's path and body.
fn stub(replies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for reply in replies {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            let (head, body_start) = loop {
                let n = s.read(&mut chunk).unwrap();
                buf.extend_from_slice(&chunk[..n]);
                if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break (String::from_utf8_lossy(&buf[..at]).to_string(), at + 4);
                }
            };
            let len: usize = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap_or(0);
            while buf.len() < body_start + len {
                let n = s.read(&mut chunk).unwrap();
                buf.extend_from_slice(&chunk[..n]);
            }
            let path = head.split_whitespace().nth(1).unwrap().to_owned();
            seen.push((path, buf[body_start..body_start + len].to_vec()));
            write!(
                s,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                reply.len()
            )
            .unwrap();
            s.write_all(&reply).unwrap();
        }
        seen
    });
    (url, handle)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_round_trips_through_a_stub_service() {
    let spec = qwen_spec();
    let (url, server) = stub(vec![
        br#"{"count": 7, "max_model_len": 8192, "tokens": []}"#.to_vec(),
        response(&spec.model, &[(0, unit(4096))]),
    ]);
    let client = VllmEmbedder::new(&url, spec.clone());
    assert_eq!(client.count_tokens("hello there").await.unwrap(), 7);
    let texts = vec!["hello".to_owned()];
    let vectors = client.embed(&texts).await.unwrap();
    assert_eq!(vectors[0].len(), 4096);
    let seen = server.join().unwrap();
    assert_eq!(seen[0].0, "/tokenize");
    assert_eq!(seen[1].0, "/v1/embeddings");
    assert_eq!(
        seen[1].1,
        request_body(&spec, &texts),
        "the body sent is the body built"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_service_is_blocked_never_fake() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = VllmEmbedder::new(&url, qwen_spec());
    let err = client
        .embed(&["x".to_owned()])
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("blocked: no embedding service"), "{err}");
}

/// The fake embedder's spec and vectors are known answers the Python twin is held to (slice 1.8):
/// its spec as canonical JSON, and the leading components of the vectors of the conformance
/// inputs' request texts. `LCTX_WRITE_KNOWN_ANSWERS=1` rewrites them after a deliberate change.
#[test]
fn the_fake_embedder_is_a_committed_known_answer() {
    use cpg_core::embed::FakeEmbedder;
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

/// The live leg of E2 (`just embed-conformance`): with `LCTX_EMBED_URL` set, the Rust client
/// embeds the conformance inputs through the running service into `$LCTX_CONFORMANCE_OUT`, which
/// `scripts/embed_conformance.py` compares with the Python client's vectors. A no-op without it.
#[tokio::test]
async fn live_conformance_vectors() {
    let (Some(url), Some(out)) = (
        std::env::var_os("LCTX_EMBED_URL"),
        std::env::var_os("LCTX_CONFORMANCE_OUT"),
    ) else {
        return;
    };
    let embedder = VllmEmbedder::new(&url.to_string_lossy(), qwen_spec());
    let inputs: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(specs().join("conformance_inputs.json")).unwrap(),
    )
    .unwrap();
    let mut vectors = serde_json::Map::new();
    for input in &inputs {
        let text = input["text"].as_str().unwrap();
        let request = if input["kind"] == "query" {
            embedder.spec().query_text(text)
        } else {
            embedder.spec().document_text(text)
        };
        let v = embedder.embed(&[request]).await.unwrap();
        vectors.insert(
            input["id"].as_str().unwrap().to_owned(),
            serde_json::json!(v[0]),
        );
    }
    std::fs::write(out, serde_json::to_string(&vectors).unwrap()).unwrap();
}

/// The holistic assessment's A7: Rust's parse accepts or rejects each body of the shared corpus
/// (`specs/embedding/responses.json`) as Python's does.
#[test]
fn responses_are_judged_as_the_shared_corpus_says() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/embedding/responses.json");
    let corpus: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut spec = qwen_spec();
    spec.model = corpus["model"].as_str().unwrap().to_owned();
    spec.dimensions = corpus["dimensions"].as_u64().unwrap() as u32;
    for case in corpus["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let inputs = case["inputs"].as_u64().unwrap() as usize;
        let body = case["body"].as_str().unwrap().as_bytes();
        let accepted = parse_embeddings(&spec, inputs, body).is_ok();
        assert_eq!(accepted, case["accept"].as_bool().unwrap(), "{name}");
    }
}
