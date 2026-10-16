//! Pure receive-admission controls; these do not establish native executor drainage.

use std::sync::{Arc, Mutex};

use futures::StreamExt;
use prost::bytes::{Buf, Bytes, BytesMut};
use surrealdb::method::{AdmittedBatchFailure, AdmittedBatchItem, BatchItem, QueryStreamOptions};
use surrealdb_rpc::{IngressAdmission, IngressReservation, IngressStage};
use tonic::codec::{
    BufferSettings, Codec, CompressionEncoding, DecodeBuf, Decoder, ReceiveAdmission,
    ReceiveAllocation, ReceiveReservation,
};
use tonic::{Code, Status};

#[derive(Debug, Default)]
struct State {
    live: usize,
    maximum: usize,
    requests: Vec<(ReceiveAllocation, usize)>,
    ingress_requests: Vec<(IngressStage, usize)>,
}

#[derive(Debug)]
struct Budget {
    state: Arc<Mutex<State>>,
    ceiling: usize,
    reject: Option<ReceiveAllocation>,
}

#[derive(Debug)]
struct Lease {
    state: Arc<Mutex<State>>,
    bytes: usize,
}
impl ReceiveReservation for Lease {}
impl IngressReservation for Lease {}
impl Drop for Lease {
    fn drop(&mut self) {
        self.state.lock().expect("state").live -= self.bytes;
    }
}

impl ReceiveAdmission for Budget {
    fn reserve(
        &self,
        boundary: ReceiveAllocation,
        bytes: usize,
    ) -> Result<Arc<dyn ReceiveReservation>, Status> {
        let mut state = self.state.lock().expect("state");
        state.requests.push((boundary, bytes));
        if self.reject == Some(boundary)
            || state
                .live
                .checked_add(bytes)
                .is_none_or(|sum| sum > self.ceiling)
        {
            return Err(Status::resource_exhausted("test ingress budget"));
        }
        state.live += bytes;
        state.maximum = state.maximum.max(state.live);
        Ok(Arc::new(Lease {
            state: Arc::clone(&self.state),
            bytes,
        }))
    }
}

impl IngressAdmission for Budget {
    fn reserve(
        &self,
        stage: IngressStage,
        bytes: usize,
    ) -> Result<Arc<dyn IngressReservation>, surrealdb::Error> {
        let mut state = self.state.lock().expect("state");
        state.ingress_requests.push((stage, bytes));
        if state
            .live
            .checked_add(bytes)
            .is_none_or(|sum| sum > self.ceiling)
        {
            return Err(surrealdb::Error::internal("test ingress budget".to_owned()));
        }
        state.live += bytes;
        state.maximum = state.maximum.max(state.live);
        Ok(Arc::new(Lease {
            state: Arc::clone(&self.state),
            bytes,
        }))
    }
}

// Exercise the production codec source without a test-only public SDK API or
// a second standalone dependency resolution for the vendored SDK unit suite.
pub mod types {
    pub use surrealdb::types::*;
}
pub use surrealdb::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;
#[path = "../../../third_party/surrealdb/src/engine/remote/grpc/ingress.rs"]
mod ingress;

#[derive(Debug)]
struct Decoded {
    bytes: Bytes,
    _leases: Vec<Arc<dyn ReceiveReservation>>,
}

struct AdmittedDecoder(Arc<Budget>);
impl Decoder for AdmittedDecoder {
    type Item = Decoded;
    type Error = Status;
    fn decode(&mut self, _: &mut DecodeBuf<'_>) -> Result<Option<Decoded>, Status> {
        panic!("admitted path must retain reservations")
    }
    fn buffer_settings(&self) -> BufferSettings {
        BufferSettings::new(8, 32)
    }
    fn receive_admission(&self) -> Option<Arc<dyn ReceiveAdmission>> {
        Some(self.0.clone())
    }
    fn decode_with_reservations(
        &mut self,
        src: &mut DecodeBuf<'_>,
        leases: &[Arc<dyn ReceiveReservation>],
    ) -> Result<Option<Decoded>, Status> {
        Ok(Some(Decoded {
            bytes: src.copy_to_bytes(src.remaining()),
            _leases: leases.to_vec(),
        }))
    }
}

fn budget(ceiling: usize, reject: Option<ReceiveAllocation>) -> Arc<Budget> {
    Arc::new(Budget {
        state: Arc::new(Mutex::new(State::default())),
        ceiling,
        reject,
    })
}

#[tokio::test]
async fn initial_transport_allocation_refuses_before_decode() {
    let owner = budget(0, None);
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner.clone()),
        tonic::body::Body::empty(),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    assert_eq!(
        stream
            .next()
            .await
            .expect("error")
            .expect_err("refused")
            .code(),
        Code::ResourceExhausted
    );
    assert!(
        stream.next().await.is_none(),
        "refusal is one terminal error, not reissuance"
    );
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

fn framed(payload: &[u8]) -> Bytes {
    let mut wire = BytesMut::new();
    wire.extend_from_slice(&[0]);
    wire.extend_from_slice(&u32::try_from(payload.len()).expect("length").to_be_bytes());
    wire.extend_from_slice(payload);
    wire.freeze()
}

#[tokio::test]
async fn body_copy_refuses_before_decode_and_growth() {
    let owner = budget(usize::MAX, Some(ReceiveAllocation::BodyCopy));
    let body = http_body_util::Full::new(framed(&[7; 1024]));
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner.clone()),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    assert_eq!(
        stream
            .next()
            .await
            .expect("error")
            .expect_err("refused")
            .code(),
        Code::ResourceExhausted
    );
    assert!(stream.next().await.is_none());
    let state = owner.state.lock().expect("state");
    assert_eq!(
        state.requests,
        vec![
            (ReceiveAllocation::EncodedBuffer, 8),
            (ReceiveAllocation::BodyCopy, 1029)
        ]
    );
}

#[tokio::test]
async fn a_wide_value_retains_exact_buffer_admission_after_stream_drop() {
    let owner = budget(8192, None);
    let body = http_body_util::Full::new(framed(&[7; 1024]));
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner.clone()),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let output = stream.next().await.expect("item").expect("admitted");
    assert_eq!(output.bytes.as_ref(), &[7; 1024]);
    drop(stream);
    assert_eq!(owner.state.lock().expect("state").live, 1029);
    drop(output);
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

#[test]
fn delivery_wrapper_transfers_its_live_reservations() {
    let state = Arc::new(Mutex::new(State {
        live: 64,
        ..State::default()
    }));
    let lease: Arc<dyn IngressReservation> = Arc::new(Lease {
        state: Arc::clone(&state),
        bytes: 64,
    });
    let item = AdmittedBatchItem::new(BatchItem::EmptyValue { index: 9 }, vec![lease]);
    assert!(matches!(item.item(), BatchItem::EmptyValue { index: 9 }));
    let (value, leases) = item.into_parts();
    assert_eq!(state.lock().expect("state").live, 64);
    drop(value);
    drop(leases);
    assert_eq!(state.lock().expect("state").live, 0);
    assert!(QueryStreamOptions::new().admission.is_none());
}

#[test]
fn terminal_failure_transfers_leases_without_replacing_original_causes() {
    let state = Arc::new(Mutex::new(State {
        live: 128,
        ..State::default()
    }));
    let lease: Arc<dyn IngressReservation> = Arc::new(Lease {
        state: state.clone(),
        bytes: 128,
    });
    let error = Error::internal("primary".to_owned())
        .with_cause(Error::internal("original".to_owned()))
        .append_cause(Error::internal("late".to_owned()));
    let failure = AdmittedBatchFailure::new(error, vec![lease]);
    assert_eq!(
        failure.error().cause().expect("original cause").message(),
        "original"
    );
    let (error, leases) = failure.into_parts();
    assert_eq!(
        error
            .cause()
            .expect("original")
            .cause()
            .expect("late")
            .message(),
        "late"
    );
    assert_eq!(state.lock().expect("state").live, 128);
    drop(error);
    drop(leases);
    assert_eq!(state.lock().expect("state").live, 0);
}

fn compressed_frame(hex: &str) -> Bytes {
    let payload = hex::decode(hex).expect("fixed independent compression fixture");
    let mut frame = framed(&payload).to_vec();
    frame[0] = 1;
    Bytes::from(frame)
}

#[tokio::test]
async fn admitted_gzip_preserves_optional_headers_members_and_checksum() {
    for fixture in [
        "1f8b081e0000000000000300666f6f6261720062617a00b83863671f05a360148c540000b71e338400040000",
        "1f8b08000000000002ff63671f0523190000ed6f4424000200001f8b08000000000002ff63671f0523190000ed6f442400020000",
    ] {
        let owner = budget(1 << 20, None);
        let body = http_body_util::Full::new(compressed_frame(fixture));
        let mut stream = tonic::Streaming::new_response(
            AdmittedDecoder(owner.clone()),
            body,
            tonic::codegen::http::StatusCode::OK,
            Some(CompressionEncoding::Gzip),
            Some(2048),
        );
        let decoded = stream.next().await.expect("item").expect("gzip decoded");
        assert_eq!(decoded.bytes.as_ref(), &[7; 1024]);
        assert!(
            owner
                .state
                .lock()
                .expect("state")
                .requests
                .iter()
                .any(|(stage, _)| *stage == ReceiveAllocation::CompressionWorkspace)
        );
        drop(stream);
        assert!(owner.state.lock().expect("state").live >= 1024);
        drop(decoded);
        assert_eq!(owner.state.lock().expect("state").live, 0);
    }
}

#[tokio::test]
async fn admitted_zstd_uses_a_reserved_static_workspace_for_all_members() {
    for fixture in [
        "28b52ffd6000034d00001007070100fb2b8005",
        "28b52ffd6000014d00001007070100fb2ac00228b52ffd6000014d00001007070100fb2ac002",
        "502a4d180300000061626328b52ffd6000034d00001007070100fb2b8005",
    ] {
        let owner = budget(1 << 20, None);
        let body = http_body_util::Full::new(compressed_frame(fixture));
        let mut stream = tonic::Streaming::new_response(
            AdmittedDecoder(owner.clone()),
            body,
            tonic::codegen::http::StatusCode::OK,
            Some(CompressionEncoding::Zstd),
            Some(2048),
        );
        let decoded = stream.next().await.expect("item").expect("zstd decoded");
        assert_eq!(decoded.bytes.as_ref(), &[7; 1024]);
        drop(stream);
        assert!(owner.state.lock().expect("state").live >= 1024);
        drop(decoded);
        assert_eq!(owner.state.lock().expect("state").live, 0);
    }
}

#[tokio::test]
async fn compression_workspace_refuses_and_corrupt_gzip_remains_failed() {
    let fixture =
        "1f8b081e0000000000000300666f6f6261720062617a00b83863671f05a360148c540000b71e338400040000";
    let owner = budget(1 << 20, Some(ReceiveAllocation::CompressionWorkspace));
    let body = http_body_util::Full::new(compressed_frame(fixture));
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner),
        body,
        tonic::codegen::http::StatusCode::OK,
        Some(CompressionEncoding::Gzip),
        Some(2048),
    );
    assert_eq!(
        stream
            .next()
            .await
            .expect("error")
            .expect_err("workspace refused")
            .code(),
        Code::ResourceExhausted
    );
    assert!(stream.next().await.is_none());
    let mut corrupted = compressed_frame(fixture).to_vec();
    let len = corrupted.len();
    corrupted[len - 8] ^= 1;
    let body = http_body_util::Full::new(Bytes::from(corrupted));
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(budget(1 << 20, None)),
        body,
        tonic::codegen::http::StatusCode::OK,
        Some(CompressionEncoding::Gzip),
        Some(2048),
    );
    assert_eq!(
        stream
            .next()
            .await
            .expect("error")
            .expect_err("bad checksum")
            .code(),
        Code::DataLoss
    );
    assert!(stream.next().await.is_none());
}

fn response(
    values: Vec<surrealdb_protocol::proto::v1::Value>,
) -> surrealdb_protocol::proto::rpc::v1::QueryResponse {
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    rpc::QueryResponse {
        frame: Some(rpc::query_response::Frame::Batch(rpc::QueryBatchFrame {
            query_index: 0,
            batch_index: 0,
            kind: rpc::QueryResponseKind::BatchedFinal as i32,
            statement_kind: rpc::QueryStatementKind::Other as i32,
            stats: None,
            error: None,
            payload: Some(rpc::query_batch_frame::Payload::Values(rpc::ValueBatch {
                values,
            })),
        })),
    }
}

#[tokio::test]
async fn production_prescan_admits_nested_values_and_duplicate_message_merges() {
    use prost::Message;
    use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};
    let value = proto::Value::array(proto::Array {
        values: vec![
            proto::Value::string("wide".repeat(1024)),
            proto::Value::object(proto::Object {
                items: vec![proto::KeyValue {
                    key: "k".to_owned(),
                    value: Some(proto::Value::int64(42)),
                }],
            }),
        ],
    });
    let first = response(vec![value.clone()]);
    let mut payload = first.encode_to_vec();
    payload.extend(first.encode_to_vec());
    let owner = budget(1 << 20, None);
    let body = http_body_util::Full::new(framed(&payload));
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), None).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream
        .next()
        .await
        .expect("item")
        .expect("admitted decoded response");
    let Some(rpc::query_response::Frame::Batch(batch)) = decoded.frame.frame else {
        panic!("batch");
    };
    let Some(rpc::query_batch_frame::Payload::Values(values)) = batch.payload else {
        panic!("values");
    };
    assert_eq!(
        values.values.len(),
        2,
        "duplicate singular messages merge their repeated fields"
    );
    let native: Vec<types::Value> = values
        .values
        .into_iter()
        .map(|value| value.try_into().expect("native conversion"))
        .collect();
    drop(stream);
    assert!(owner.state.lock().expect("state").live > 0);
    assert!(
        owner
            .state
            .lock()
            .expect("state")
            .ingress_requests
            .iter()
            .any(|(stage, _)| *stage == IngressStage::ProtobufDecode)
    );
    drop(native);
    drop(decoded.leases);
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

#[tokio::test]
async fn a_regex_owner_refuses_before_regex_compilation() {
    use prost::Message;
    use surrealdb_protocol::proto::v1 as proto;
    let payload = response(vec![proto::Value::regex("a+".to_owned())]).encode_to_vec();
    let owner = budget(1 << 20, None);
    let body = http_body_util::Full::new(framed(&payload));
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), None).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let mut decoded = stream
        .next()
        .await
        .expect("item")
        .expect("encoded string admitted");
    let value = proto::Value::regex("a+".to_owned());
    let error = ingress::convert_value(value, owner.as_ref(), &mut decoded.leases)
        .expect_err("owner refuses regex compilation");
    assert!(error.is_configuration());
    assert!(
        owner
            .state
            .lock()
            .expect("state")
            .ingress_requests
            .iter()
            .any(|(stage, _)| *stage == IngressStage::ProtobufDecode)
    );
}

#[tokio::test]
async fn structured_error_details_and_geometry_keep_native_meaning() {
    use prost::Message;
    use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};
    let duration = proto::Value::duration(proto::Duration {
        seconds: 7,
        nanos: 0,
    });
    let error = proto::SurrealError {
        kind: proto::ErrorKind::Query as i32,
        message: "bounded timeout".to_owned(),
        code: 0,
        details: Some(proto::ErrorDetails {
            kind: "TimedOut".to_owned(),
            content: Some(proto::Value::object(proto::Object {
                items: vec![proto::KeyValue {
                    key: "duration".to_owned(),
                    value: Some(duration),
                }],
            })),
        }),
        cause: None,
        retry: None,
    };
    let frame = rpc::QueryResponse {
        frame: Some(rpc::query_response::Frame::Error(error)),
    };
    let owner = budget(1 << 20, None);
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), None).decoder(),
        http_body_util::Full::new(framed(&frame.encode_to_vec())),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let mut decoded = stream
        .next()
        .await
        .expect("item")
        .expect("typed error decoded");
    let Some(rpc::query_response::Frame::Error(error)) = decoded.frame.frame else {
        panic!("error");
    };
    let error = ingress::convert_error(error, owner.as_ref(), &mut decoded.leases)
        .expect("typed native error");
    assert!(
        matches!(error.query_details(), Some(types::QueryError::TimedOut { duration }) if duration.as_secs() == 7)
    );
    let polygon = proto::Value::geometry(proto::Geometry::polygon(proto::Polygon {
        exterior: Some(proto::Line {
            points: vec![
                proto::Point { x: 0.0, y: 0.0 },
                proto::Point { x: 1.0, y: 0.0 },
                proto::Point { x: 0.0, y: 1.0 },
            ],
        }),
        interiors: vec![],
    }));
    let payload = response(vec![polygon]).encode_to_vec();
    let mut geometry_stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), None).decoder(),
        http_body_util::Full::new(framed(&payload)),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let mut decoded_geometry = geometry_stream
        .next()
        .await
        .expect("item")
        .expect("geometry decoded");
    let Some(rpc::query_response::Frame::Batch(batch)) = decoded_geometry.frame.frame else {
        panic!("batch");
    };
    let Some(rpc::query_batch_frame::Payload::Values(mut values)) = batch.payload else {
        panic!("values");
    };
    let geometry = ingress::convert_value(
        values.values.remove(0),
        owner.as_ref(),
        &mut decoded_geometry.leases,
    )
    .expect("geometry conversion");
    assert!(
        matches!(geometry, types::Value::Geometry(types::Geometry::Polygon(ref polygon)) if polygon.exterior().0.len() == 4)
    );
}

#[test]
fn recursive_regex_hook_preserves_ordinary_conversion_and_routes_every_value() {
    use std::str::FromStr;
    use surrealdb_protocol::proto::v1 as proto;
    let compiled = types::Regex::from_str("a+").expect("separately owned precompiled fixture");
    let regex = proto::Value::regex("a+".to_owned());
    let mut calls = 0;
    let mut compile = |pattern: &str| {
        assert_eq!(pattern, "a+");
        calls += 1;
        Ok(compiled.clone())
    };
    let value = proto::Value::array(proto::Array {
        values: vec![
            regex.clone(),
            proto::Value::object(proto::Object {
                items: vec![proto::KeyValue {
                    key: "k".to_owned(),
                    value: Some(regex.clone()),
                }],
            }),
            proto::Value::set(proto::Set {
                values: vec![regex.clone()],
            }),
        ],
    });
    let native = types::Value::try_from_proto_with_regex(value, &mut compile)
        .expect("qualified caller hook");
    assert_eq!(calls, 3);
    assert!(matches!(native, types::Value::Array(_)));
    assert!(matches!(
        types::Value::try_from(regex).expect("ordinary codec remains supported"),
        types::Value::Regex(_)
    ));
    assert!(
        types::Value::try_from_proto_with_regex(proto::Value::regex("b+".to_owned()), &mut |_| Ok(
            compiled.clone()
        ))
        .is_err(),
        "a reused compiler result must retain exact normalized pattern identity"
    );
}

#[tokio::test]
async fn bootstrap_capabilities_keep_their_separate_connection_owner() {
    use prost::Message;
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    let message = rpc::GetCapabilitiesResponse {
        capabilities: Some(rpc::ServerCapabilities {
            server_version: "3.3.0+candidate".to_owned(),
            capabilities: vec!["QUERY_BATCH_BYTES_V1".to_owned(), "SESSIONS".to_owned()],
            denied_methods: vec![
                "surrealdb.protocol.rpc.v1.SurrealDBService/ExportSurql".to_owned(),
            ],
            accepted_message_encodings: vec!["gzip".to_owned(), "zstd".to_owned()],
            ..Default::default()
        }),
    };
    let owner = budget(1 << 20, None);
    let body = http_body_util::Full::new(framed(&message.encode_to_vec()));
    let mut stream = tonic::Streaming::new_response(
        ingress::CapabilitiesCodec(owner.clone()).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream
        .next()
        .await
        .expect("capabilities")
        .expect("connection admitted");
    assert_eq!(
        decoded
            .response
            .capabilities
            .as_ref()
            .expect("capabilities")
            .capabilities
            .len(),
        2
    );
    assert!(!decoded.leases.is_empty());
    drop(stream);
    assert!(
        owner
            .state
            .lock()
            .expect("state")
            .ingress_requests
            .iter()
            .any(|(stage, _)| *stage == IngressStage::Connection)
    );
    assert!(owner.state.lock().expect("state").live > 0);
    drop(decoded);
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

#[tokio::test]
async fn receive_admission_preserves_delayed_physical_status() {
    use http_body_util::BodyExt;
    let owner = budget(1 << 20, None);
    let (release, wait) = tokio::sync::oneshot::channel();
    let body = http_body_util::Full::new(framed(&[7; 1024])).with_trailers(async move {
        wait.await.expect("release physical trailers");
        let mut trailers = tonic::codegen::http::HeaderMap::new();
        trailers.insert("grpc-status", "13".parse().expect("status"));
        trailers.insert(
            "grpc-message",
            "late%20transport%20failure".parse().expect("message"),
        );
        Some(Ok::<_, std::convert::Infallible>(trailers))
    });
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner.clone()),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream.next().await.expect("item").expect("decoded");
    assert!(
        futures::poll!(stream.next()).is_pending(),
        "decoded payload is not physical EOF"
    );
    release.send(()).expect("release");
    let failure = stream
        .next()
        .await
        .expect("late failure")
        .expect_err("status must remain failed");
    assert_eq!(failure.code(), Code::Internal);
    assert_eq!(failure.message(), "late transport failure");
    assert!(stream.next().await.is_none());
    drop(stream);
    assert!(owner.state.lock().expect("state").live > 0);
    drop(decoded);
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

#[tokio::test]
async fn queued_values_share_only_their_actual_encoded_allocation_lease() {
    let owner = budget(8192, None);
    let mut data = framed(&[7; 512]).to_vec();
    data.extend_from_slice(&framed(&[8; 512]));
    let body = http_body_util::Full::new(Bytes::from(data));
    let mut stream = tonic::Streaming::new_response(
        AdmittedDecoder(owner.clone()),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let first = stream.next().await.expect("first").expect("admitted");
    let second = stream.next().await.expect("second").expect("admitted");
    drop(stream);
    assert_eq!(owner.state.lock().expect("state").live, 1034);
    drop(first);
    assert_eq!(owner.state.lock().expect("state").live, 1034);
    drop(second);
    assert_eq!(owner.state.lock().expect("state").live, 0);
}

#[tokio::test]
async fn bootstrap_auth_and_selection_use_the_same_admitted_decoder() {
    use prost::Message;
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    let auth = rpc::SigninResponse {
        tokens: Some(rpc::Tokens {
            access: "token".repeat(1024),
            refresh: "refresh".repeat(512),
            ..Default::default()
        }),
    };
    let owner = budget(1 << 20, None);
    let mut stream = tonic::Streaming::new_response(
        ingress::UnaryCodec::<rpc::SigninRequest, rpc::SigninResponse>::new(owner.clone())
            .decoder(),
        http_body_util::Full::new(framed(&auth.encode_to_vec())),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream
        .next()
        .await
        .expect("auth response")
        .expect("admitted");
    assert_eq!(decoded.response.tokens.as_ref().unwrap().access.len(), 5120);
    assert!(!decoded.leases.is_empty());
    drop(stream);
    assert!(
        owner
            .state
            .lock()
            .unwrap()
            .ingress_requests
            .iter()
            .any(|(stage, _)| *stage == IngressStage::Connection)
    );
    assert!(owner.state.lock().unwrap().live > 0);
    drop(decoded);
    assert_eq!(owner.state.lock().unwrap().live, 0);
    let selection = rpc::UseResponse {
        namespace: "n".repeat(512),
        database: "d".repeat(1024),
    };
    let mut stream = tonic::Streaming::new_response(
        ingress::UnaryCodec::<rpc::UseRequest, rpc::UseResponse>::new(owner.clone()).decoder(),
        http_body_util::Full::new(framed(&selection.encode_to_vec())),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream
        .next()
        .await
        .expect("selection response")
        .expect("admitted");
    assert_eq!(decoded.response.database.len(), 1024);
    drop(decoded);
    drop(stream);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[test]
fn local_error_owners_do_not_change_wire_semantics_and_survive_chaining() {
    use surrealdb::types::SurrealValue;
    struct DiagnosticOwner;
    impl std::fmt::Debug for DiagnosticOwner {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            panic!("error diagnostics must not traverse local ownership resources")
        }
    }
    impl types::ErrorResource for DiagnosticOwner {}
    assert!(
        std::mem::size_of::<Error>() < 128,
        "ordinary Result errors stay compact"
    );
    assert!(
        std::mem::size_of::<AdmittedBatchFailure>() < 128,
        "cold error payload stays behind its admitted Box"
    );
    let owner = budget(1024, None);
    let leases =
        vec![IngressAdmission::reserve(owner.as_ref(), IngressStage::Connection, 96).unwrap()];
    let plain = Error::query("typed failure".into(), Some(types::QueryError::Cancelled));
    let expected = plain.clone().into_value();
    let retained = ingress::retain_error(plain.clone(), leases);
    let second = IngressAdmission::reserve(owner.as_ref(), IngressStage::Connection, 48).unwrap();
    let retained = ingress::retain_error(retained, vec![second]);
    let retained = retained.with_resource(Arc::new(DiagnosticOwner));
    assert_eq!(retained, plain);
    assert_eq!(retained.clone().into_value(), expected);
    assert_eq!(Error::from_value(expected).unwrap(), plain);
    let chain = Error::internal("primary".into()).append_cause(retained);
    let diagnostic = format!("{chain:?}");
    assert!(diagnostic.contains("primary"));
    assert!(diagnostic.contains("typed failure"));
    assert!(
        std::error::Error::source(&chain)
            .unwrap()
            .to_string()
            .contains("typed failure")
    );
    assert_eq!(owner.state.lock().unwrap().live, 144);
    drop(chain);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[tokio::test]
async fn uncompressed_query_ceiling_preserves_compression_wire_overhead() {
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    let owner = budget(1 << 20, None);
    // Independent gzip fixture for the two-byte protobuf End frame 1a00.
    // Its compressed representation is larger than the uncompressed ceiling.
    let body = http_body_util::Full::new(compressed_frame(
        "1f8b08000000000002039362000024e8f4f102000000",
    ));
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), Some(2)).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        Some(CompressionEncoding::Gzip),
        Some(64),
    );
    let decoded = stream
        .next()
        .await
        .expect("end frame")
        .expect("compression overhead is valid");
    assert!(matches!(
        decoded.frame.frame,
        Some(rpc::query_response::Frame::End(_))
    ));
    drop(decoded);
    drop(stream);
    assert_eq!(owner.state.lock().unwrap().live, 0);
    // Three uncompressed bytes exceed the same application ceiling, even
    // though the raw wire frame fits the independent connection ceiling.
    let body = http_body_util::Full::new(compressed_frame(
        "1f8b08000000000002039362640000f5ceebf703000000",
    ));
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), Some(2)).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        Some(CompressionEncoding::Gzip),
        Some(64),
    );
    assert_eq!(
        stream
            .next()
            .await
            .expect("refusal")
            .err()
            .expect("expanded frame refused")
            .code(),
        Code::ResourceExhausted
    );
    assert!(stream.next().await.is_none());
}

#[path = "../../../third_party/surrealdb/src/engine/remote/grpc/buffered.rs"]
mod buffered;

fn buffered_end(index: usize, error: Option<Error>) -> surrealdb_rpc::AdmittedQueryStreamEvent {
    surrealdb_rpc::AdmittedQueryStreamEvent::Item(AdmittedBatchItem::new(
        BatchItem::Finished {
            index,
            time: Default::default(),
            query_type: surrealdb::method::BatchQueryType::Other,
            error,
        },
        vec![],
    ))
}

#[test]
fn production_buffered_owner_retains_batches_and_preserves_single_empty_shape() {
    use surrealdb_rpc::AdmittedQueryStreamEvent;
    let owner = budget(1 << 20, None);
    let mut buffer = buffered::BufferedResults::new(owner.clone()).unwrap();
    for value in [11, 12] {
        let lease =
            IngressAdmission::reserve(owner.as_ref(), IngressStage::ValueConversion, 256).unwrap();
        buffer.ingest(AdmittedQueryStreamEvent::Item(AdmittedBatchItem::new(
            BatchItem::Rows {
                index: 0,
                values: vec![types::Value::from_t(value)],
            },
            vec![lease],
        )));
    }
    buffer.ingest(buffered_end(0, None));
    buffer.ingest(AdmittedQueryStreamEvent::Item(AdmittedBatchItem::new(
        BatchItem::EmptyValue { index: 2 },
        vec![],
    )));
    buffer.ingest(buffered_end(2, None));
    let results = buffer.finish().unwrap();
    assert_eq!(
        results.results.len(),
        2,
        "skipped indexes do not invent a statement result"
    );
    assert_eq!(
        results.results[0].result.as_ref().unwrap(),
        &types::Value::from_t(vec![11, 12])
    );
    assert_eq!(
        results.results[1].result.as_ref().unwrap(),
        &types::Value::None
    );
    assert!(results.reservations.is_some());
    assert!(owner.state.lock().unwrap().live > 512);
    drop(results);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[test]
fn buffered_typed_statement_errors_keep_their_owner_after_response_drop() {
    let owner = budget(1 << 20, None);
    let mut buffer = buffered::BufferedResults::new(owner.clone()).unwrap();
    buffer.ingest(buffered_end(
        0,
        Some(Error::query(
            "conflict".into(),
            Some(types::QueryError::TransactionConflict),
        )),
    ));
    let mut results = buffer.finish().unwrap();
    let failure = results.results.remove(0).result.unwrap_err();
    drop(results);
    assert!(failure.is_query());
    assert!(owner.state.lock().unwrap().live > 0);
    drop(failure);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[test]
fn buffered_local_refusal_still_collects_a_late_typed_failure() {
    use surrealdb_rpc::AdmittedQueryStreamEvent;
    let owner = budget(4096, None);
    let mut buffer = buffered::BufferedResults::new(owner.clone()).unwrap();
    // Isolate accumulator growth: this supplied fixture batch is not a claim
    // about transport admission, which the decoder controls test separately.
    buffer.ingest(AdmittedQueryStreamEvent::Item(AdmittedBatchItem::new(
        BatchItem::Rows {
            index: 0,
            values: (0..128).map(types::Value::from_t).collect(),
        },
        vec![],
    )));
    buffer.ingest(buffered_end(0, None));
    buffer.ingest(AdmittedQueryStreamEvent::Failure(
        AdmittedBatchFailure::new(
            Error::query(
                "late physical failure".into(),
                Some(types::QueryError::Cancelled),
            ),
            vec![],
        ),
    ));
    let failure = buffer.finish().unwrap_err();
    assert!(failure.message().contains("test ingress budget"));
    assert!(
        failure.cause().is_some_and(Error::is_query),
        "late failure must not disappear after local refusal"
    );
    assert!(owner.state.lock().unwrap().live <= 4096);
    drop(failure);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[tokio::test]
async fn ordinary_unconfigured_unary_decode_keeps_its_original_semantics() {
    use prost::Message;
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    let message = rpc::UseResponse {
        namespace: "n".into(),
        database: "d".into(),
    };
    let mut stream = tonic::Streaming::new_response(
        ingress::UnaryCodec::<rpc::UseRequest, rpc::UseResponse>::unadmitted().decoder(),
        http_body_util::Full::new(framed(&message.encode_to_vec())),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let decoded = stream.next().await.unwrap().unwrap();
    assert_eq!(decoded.response.namespace, "n");
    assert!(
        decoded.leases.is_empty(),
        "ordinary SDK has no new allocation claim"
    );
}

#[tokio::test]
async fn malformed_wide_object_keys_admit_the_actual_escaped_diagnostic() {
    use prost::Message;
    use surrealdb_protocol::proto::v1 as proto;
    let key = "\u{7f}".repeat(1024);
    let value = proto::Value::object(proto::Object {
        items: vec![proto::KeyValue {
            key: key.clone(),
            value: None,
        }],
    });
    let payload = response(vec![value.clone()]).encode_to_vec();
    let cost = ingress::response_cost(&payload).unwrap();
    let diagnostic = format!("Missing value in Object entry for key {key:?}");
    assert!(
        cost.native >= diagnostic.len() * 6,
        "Debug escaping must be counted before conversion"
    );
    let owner = budget(1 << 20, None);
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(owner.clone(), None).decoder(),
        http_body_util::Full::new(framed(&payload)),
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let mut decoded = stream.next().await.unwrap().unwrap();
    let error = ingress::convert_value(value, owner.as_ref(), &mut decoded.leases).unwrap_err();
    assert_eq!(error.message(), diagnostic);
    assert!(owner.state.lock().unwrap().live >= cost.native);
    drop(error);
    drop(decoded);
    drop(stream);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}

#[tokio::test]
async fn local_structural_refusal_can_terminate_before_withheld_physical_eof() {
    use http_body_util::BodyExt;
    use prost::Message;
    use std::sync::atomic::{AtomicBool, Ordering};
    use surrealdb_protocol::proto::rpc::v1 as rpc;
    #[derive(Debug)]
    struct RefuseProtobuf(Arc<Budget>);
    impl IngressAdmission for RefuseProtobuf {
        fn reserve(
            &self,
            stage: IngressStage,
            bytes: usize,
        ) -> Result<Arc<dyn IngressReservation>> {
            if matches!(stage, IngressStage::ProtobufDecode) {
                return Err(Error::configuration(
                    "structural allocation refused".into(),
                    None,
                ));
            }
            IngressAdmission::reserve(self.0.as_ref(), stage, bytes)
        }
    }
    let owner = budget(1 << 20, None);
    let polled = Arc::new(AtomicBool::new(false));
    let observed = polled.clone();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let message = rpc::QueryResponse {
        frame: Some(rpc::query_response::Frame::End(Default::default())),
    };
    let body =
        http_body_util::Full::new(framed(&message.encode_to_vec())).with_trailers(async move {
            observed.store(true, Ordering::Release);
            let _ = wait.await;
            Some(Ok::<_, std::convert::Infallible>(
                tonic::codegen::http::HeaderMap::new(),
            ))
        });
    let mut stream = tonic::Streaming::new_response(
        ingress::QueryCodec(Arc::new(RefuseProtobuf(owner.clone())), None).decoder(),
        body,
        tonic::codegen::http::StatusCode::OK,
        None,
        None,
    );
    let failure = stream
        .next()
        .await
        .expect("local failure")
        .err()
        .expect("predecode refusal");
    assert_eq!(failure.code(), Code::ResourceExhausted);
    assert!(
        stream.next().await.is_none(),
        "tonic receive termination is not physical body EOF"
    );
    assert!(
        !polled.load(Ordering::Acquire),
        "physical trailers remain withheld"
    );
    drop(release);
    drop(stream);
    assert_eq!(owner.state.lock().unwrap().live, 0);
}
