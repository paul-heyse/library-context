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
