//! Allocation admission for the declared finite row-value receive subset.
//!
//! Unsupported Arrow allocation branches refuse before generated decoding;
//! regex compilation requires a separately qualified owner hook. Ordinary SDK
//! routes keep their complete value/compression support.

use std::mem::{align_of, size_of};
use std::sync::Arc;

use surrealdb_protocol::proto::{rpc::v1 as rpc, v1 as proto};
use surrealdb_rpc::{IngressAdmission, IngressReservation, IngressStage};
use tonic::codec::{BufferSettings, Codec, DecodeBuf, Decoder, EncodeBuf, Encoder, ReceiveAdmission, ReceiveAllocation, ReceiveReservation};
use tonic::Status;
use crate::types::prost::{Message, bytes::Buf, encoding::{decode_key, decode_varint, skip_field, DecodeContext, WireType}};
use crate::types::Value;

#[derive(Clone, Copy)]
enum Shape { Response, Begin, End, Batch, Stats, Values, Value, Array, Set, Object, Entry, Record, Key, File, Range, Bound, KeyRange, KeyBound, Decimal, Uuid, Duration, Error, Details, Retry, Geometry, Point, Line, Polygon, MultiPoint, MultiLine, MultiPolygon, Collection, CapResponse, Capabilities, SemVer, Limits, Live, SessionResponse, UseResponse, TokensResponse, AuthenticateResponse, Tokens, Empty }

enum Field { Message(Shape), Repeated(Shape), RepeatedString, Boxed(Shape), String, Regex, Bytes, Scalar, Unsupported }

fn field(shape: Shape, tag: u32) -> Field {
    use Field::*;
    use Shape as S;
    match (shape, tag) {
        (S::CapResponse, 1) => Message(S::Capabilities),
        (S::SessionResponse, 1) => Message(S::Uuid),
        (S::UseResponse, 1 | 2) | (S::Tokens, 1 | 2) => String,
        (S::TokensResponse, 1) | (S::AuthenticateResponse, 2) => Message(S::Tokens),
        (S::Tokens, 3) | (S::AuthenticateResponse, 1) => Message(S::Duration),
        (S::Capabilities, 1) | (S::SemVer, 4) => String,
        (S::Capabilities, 2 | 3) => Message(S::SemVer),
        (S::Capabilities, 4 | 5 | 8) => RepeatedString,
        (S::Capabilities, 6) => Message(S::Limits), (S::Capabilities, 7) => Message(S::Live),
        (S::Limits, 3) | (S::Live, 4) => Message(S::Duration),
        (S::Response, 1) => Message(S::Begin), (S::Response, 2) => Message(S::Batch), (S::Response, 3) => Message(S::End), (S::Response, 4) => Message(S::Error),
        (S::Begin, 1) => Message(S::Uuid), (S::End, 2) | (S::Stats, 5) => Message(S::Duration),
        (S::Batch, 5) => Message(S::Stats), (S::Batch, 6) => Message(S::Error), (S::Batch, 8) => Unsupported, (S::Batch, 7) => Message(S::Values),
        (S::Error, 2) | (S::Details, 1) => String,
        (S::Error, 4) => Message(S::Details), (S::Error, 5) => Boxed(S::Error), (S::Error, 6) => Message(S::Retry),
        (S::Retry, 1) => Message(S::Duration), (S::Details, 2) => Message(S::Value),
        (S::Geometry, 1) => Message(S::Point), (S::Geometry, 2) => Message(S::Line), (S::Geometry, 3) => Message(S::Polygon),
        (S::Geometry, 4) => Message(S::MultiPoint), (S::Geometry, 5) => Message(S::MultiLine), (S::Geometry, 6) => Message(S::MultiPolygon), (S::Geometry, 7) => Message(S::Collection),
        (S::Line | S::MultiPoint, 1) => Repeated(S::Point), (S::MultiLine, 1) => Repeated(S::Line),
        (S::MultiPolygon, 1) => Repeated(S::Polygon), (S::Collection, 1) => Repeated(S::Geometry),
        (S::Polygon, 1) => Message(S::Line), (S::Polygon, 2) => Repeated(S::Line),
        (S::Values | S::Array | S::Set, 1) => Repeated(S::Value),
        (S::Object, 1) => Repeated(S::Entry),
        (S::Entry, 1) | (S::Record, 1) | (S::File, 1 | 2) | (S::Decimal, 1) => String,
        (S::Entry, 2) => Message(S::Value), (S::Record, 2) => Message(S::Key),
        (S::Uuid, 1) => Bytes,
        (S::Range, 1 | 2) => Boxed(S::Bound), (S::KeyRange, 1 | 2) => Boxed(S::KeyBound),
        (S::Bound, 1 | 2) => Boxed(S::Value), (S::KeyBound, 1 | 2) => Boxed(S::Key),
        (S::Bound | S::KeyBound, 3) => Message(S::Empty),
        (S::Value, 1 | 2) => Message(S::Empty), (S::Value, 6) => Message(S::Decimal),
        (S::Value, 7 | 13 | 15) => String, (S::Value, 8) => Bytes,
        (S::Value, 9 | 10) => Message(S::Duration), (S::Value, 11) => Message(S::Uuid),
        (S::Value, 12) => Message(S::Geometry), (S::Value, 18) => Regex, (S::Value, 14) => Message(S::Record), (S::Value, 16) => Message(S::File),
        (S::Value, 17) => Boxed(S::Range), (S::Value, 19) => Message(S::Object),
        (S::Value, 20) => Message(S::Array), (S::Value, 21) => Message(S::Set),
        (S::Key, 2) => String, (S::Key, 3) => Message(S::Uuid), (S::Key, 4) => Message(S::Array),
        (S::Key, 5) => Boxed(S::KeyRange), (S::Key, 6) => Message(S::Object), (S::Key, 8) => Message(S::Decimal),
        _ => Scalar,
    }
}

fn shape_size(shape: Shape) -> usize {
    match shape {
        Shape::Value => size_of::<proto::Value>(), Shape::Entry => size_of::<proto::KeyValue>(),
        Shape::Range => size_of::<proto::Range>(), Shape::Bound => size_of::<proto::ValueBound>(),
        Shape::Key => size_of::<proto::RecordIdKey>(), Shape::KeyRange => size_of::<proto::RecordIdKeyRange>(),
        Shape::KeyBound => size_of::<proto::RecordIdKeyBound>(),
        Shape::Point => size_of::<proto::Point>(), Shape::Line => size_of::<proto::Line>(),
        Shape::Polygon => size_of::<proto::Polygon>(), Shape::Geometry => size_of::<proto::Geometry>(), _ => 0,
    }
}

/// Costs are derived from field structure, not from an encoded-byte multiplier.
/// Vec's pinned RawVec growth keeps old+new allocation below 3*n elements,
/// with a four-element minimum for these non-byte element types. Strings use
/// the corresponding byte-vector minimum of eight. Summing every occurrence
/// also covers duplicate-field replacement and message merging.
#[derive(Default, Debug, Clone, Copy)]
pub(super) struct Cost { pub protobuf: usize, pub native: usize }
impl Cost {
    fn add(&mut self, protobuf: usize, native: usize) -> Result<(), Status> {
        self.protobuf = self.protobuf.checked_add(protobuf).ok_or_else(|| Status::resource_exhausted("Protobuf admission size overflow"))?;
        self.native = self.native.checked_add(native).ok_or_else(|| Status::resource_exhausted("Value admission size overflow"))?;
        Ok(())
    }
}

fn multiply(a: usize, b: usize) -> Result<usize, Status> {
    a.checked_mul(b).ok_or_else(|| Status::resource_exhausted("Ingress allocation size overflow"))
}

// Pinned alloc BTreeMap has B=6, eleven key/value slots, twelve child
// pointers, a parent pointer and two u16 fields. Summed fields plus one
// alignment allowance per field bounds either compiler layout. The root has
// at least one key and each non-root node at least five after split; ceil(n/5)
// therefore bounds nodes for n insertions, including the eventual split nodes.
fn map_node_bound() -> usize {
    let align = align_of::<Value>().max(align_of::<String>()).max(align_of::<usize>());
    11 * (size_of::<String>() + size_of::<Value>()) + 13 * size_of::<usize>() + 4 + 7 * align
}

fn scan(mut bytes: &[u8], shape: Shape, depth: usize, cost: &mut Cost) -> Result<(), Status> {
    if depth > 100 { return Err(Status::resource_exhausted("Ingress protobuf recursion exceeds the pinned decoder limit")); }
    let mut repeated = 0u64;
    let mut map_entries = 0usize;
    let mut entry_key = "";
    let mut entry_value = false;
    if matches!(shape, Shape::Details) {
        // proto_details constructs one two-field object, with static key copies.
        cost.add(0, 2 * map_node_bound() + 16)?;
    }
    if matches!(shape, Shape::Error) {
        // Pinned typed error details consume their maps; they have no flatten
        // clones or arbitrary Value formatting. The deepest detail/field route
        // is three layers. Variant/field/type names are <32 bytes and each
        // format template <128 bytes; six simultaneously retained 512-byte
        // formatting buffers cover failed scalar conversion and its wrappers.
        cost.add(0, 6 * 512 + 2 * size_of::<crate::types::Kind>() + crate::Error::resource_allocation_size())?;
    }
    while bytes.has_remaining() {
        let (tag, wire) = decode_key(&mut bytes).map_err(|error| Status::data_loss(error.to_string()))?;
        if matches!(shape, Shape::Entry) && tag == 2 { entry_value = true; }
        match field(shape, tag) {
            Field::Unsupported => return Err(Status::unimplemented("Ingress admission does not cover Arrow decoding")),
            Field::Scalar => skip_field(wire, tag, &mut bytes, DecodeContext::default()).map_err(|error| Status::data_loss(error.to_string()))?,
            field => {
                if wire != WireType::LengthDelimited { return Err(Status::data_loss("Invalid ingress field wire type")); }
                let len = usize::try_from(decode_varint(&mut bytes).map_err(|error| Status::data_loss(error.to_string()))?).map_err(|_| Status::resource_exhausted("Ingress field length overflow"))?;
                if len > bytes.len() { return Err(Status::data_loss("Truncated ingress field")); }
                let (body, rest) = bytes.split_at(len); bytes = rest;
                match field {
                    Field::String => {
                        cost.add(multiply(len, 2)?.checked_add(8).ok_or_else(|| Status::resource_exhausted("String size overflow"))?, 0)?;
                        if matches!(shape, Shape::Entry) && tag == 1 {
                            entry_key = std::str::from_utf8(body).map_err(|_| Status::data_loss("Invalid object key UTF-8"))?;
                        }
                    },
                    Field::Regex => {
                        // One compilation lease joins the growing source Vec
                        // (old+new <=3*n slots), and at most two statement items
                        // clone that inventory. A sixth slot conservatively
                        // retains the source Vec minimum-capacity allowance;
                        // the callback owns its opaque lease allocation cost.
                        cost.add(multiply(len, 2)?.checked_add(8).ok_or_else(|| Status::resource_exhausted("Regex string size overflow"))?, 6 * size_of::<Arc<dyn IngressReservation>>())?;
                    },
                    Field::Bytes => {}, // Shares the admitted transport buffer.
                    Field::Message(child) => scan(body, child, depth + 1, cost)?,
                    Field::Boxed(child) => {
                        let proto_box = if matches!(child, Shape::Error) { size_of::<proto::SurrealError>() } else { shape_size(child) };
                        cost.add(proto_box, size_of::<crate::types::Range>().max(size_of::<crate::types::RecordIdKeyRange>()).max(size_of::<Value>()).max(size_of::<crate::Error>()))?;
                        scan(body, child, depth + 1, cost)?;
                    }
                    Field::Repeated(child) => {
                        if repeated & (1 << tag) == 0 {
                            let native_min = if matches!(shape, Shape::Object) { 0 } else { multiply(4, size_of::<Value>())? };
                            cost.add(multiply(4, shape_size(child))?, native_min)?;
                            repeated |= 1 << tag;
                        }
                        if matches!(shape, Shape::Object | Shape::Set) {
                            map_entries = map_entries.checked_add(1).ok_or_else(|| Status::resource_exhausted("Map entry count overflow"))?;
                        }
                        // Set's pinned collect first builds a Vec and uses a
                        // stable sort: retained capacity <2*n plus <=n/2 scratch
                        // fits this same 3*n bound before bulk tree building.
                        let native = if matches!(shape, Shape::Object) { 0 } else { multiply(3, size_of::<Value>())? };
                        cost.add(multiply(3, shape_size(child))?, native)?;
                        scan(body, child, depth + 1, cost)?;
                    }
                    Field::RepeatedString => {
                        if repeated & (1 << tag) == 0 {
                            cost.add(4 * size_of::<std::string::String>(), 0)?;
                            repeated |= 1 << tag;
                        }
                        cost.add(3 * size_of::<std::string::String>(), 0)?;
                        cost.add(multiply(len, 2)?.checked_add(8).ok_or_else(|| Status::resource_exhausted("Capability string size overflow"))?, 0)?;
                    }
                    Field::Scalar | Field::Unsupported => unreachable!(),
                }
            }
        }
    }
    if matches!(shape, Shape::Entry) && !entry_value {
        // The existing converter includes the Debug-escaped key in its missing-
        // value error. Count that exact formatting without allocating; admit
        // both the anyhow message and its SDK serialization-error copy. Each
        // String's pinned old+new geometric growth stays below three output
        // lengths, plus the byte-vector minimum and fixed wrapper storage.
        struct Count(usize);
        impl std::fmt::Write for Count {
            fn write_str(&mut self, value: &str) -> std::fmt::Result {
                self.0 = self.0.checked_add(value.len()).ok_or(std::fmt::Error)?;
                Ok(())
            }
        }
        let mut count = Count(0);
        std::fmt::write(&mut count, format_args!("Missing value in Object entry for key {entry_key:?}"))
            .map_err(|_| Status::resource_exhausted("Object error diagnostic size overflow"))?;
        cost.add(0, multiply(count.0, 6)?.checked_add(128).ok_or_else(|| Status::resource_exhausted("Object error diagnostic size overflow"))?)?;
    }
    if map_entries != 0 {
        // BTreeSet bulk construction can hold a partially initialized right
        // spine; one extra node per binary height is a conservative bound on
        // that smaller B=6 spine, without charging one whole node per value.
        let spine = if matches!(shape, Shape::Set) { (usize::BITS - map_entries.leading_zeros()) as usize } else { 0 };
        cost.add(0, multiply(map_entries.div_ceil(5).checked_add(spine).ok_or_else(|| Status::resource_exhausted("Map node count overflow"))?, map_node_bound())?)?;
    }
    Ok(())
}

pub(super) fn response_cost(bytes: &[u8]) -> Result<Cost, Status> {
    // Fixed response/item/lease-vector wrappers have explicit storage bounds.
    let mut cost = Cost { protobuf: size_of::<rpc::QueryResponse>() + 4 * size_of::<Arc<dyn IngressReservation>>(), native: 4 * size_of::<surrealdb_rpc::QueryStreamItem>() + 8 * size_of::<Arc<dyn IngressReservation>>() };
    scan(bytes, Shape::Response, 0, &mut cost)?;
    Ok(cost)
}

#[derive(Debug)]
struct ReceiveLease { _lease: Arc<dyn IngressReservation> }
impl ReceiveReservation for ReceiveLease {}
#[derive(Debug)]
struct TransportLease { _lease: Arc<dyn ReceiveReservation> }
impl IngressReservation for TransportLease {}
#[derive(Debug)]
struct TransportAdmission(Arc<dyn IngressAdmission>, Option<usize>);
impl ReceiveAdmission for TransportAdmission {
    fn max_uncompressed_message_size(&self) -> Option<usize> { self.1 }
    fn reserve(&self, boundary: ReceiveAllocation, bytes: usize) -> Result<Arc<dyn ReceiveReservation>, Status> {
        match boundary {
            ReceiveAllocation::BodyCopy | ReceiveAllocation::EncodedBuffer => self.0.reserve(IngressStage::EncodedFrame, bytes).map(|lease| Arc::new(ReceiveLease { _lease: lease }) as Arc<dyn ReceiveReservation>).map_err(|error| Status::resource_exhausted(error.to_string())),
            ReceiveAllocation::CompressionWorkspace => self.0.reserve(IngressStage::CompressionWorkspace, bytes).map(|lease| Arc::new(ReceiveLease { _lease: lease }) as Arc<dyn ReceiveReservation>).map_err(|error| Status::resource_exhausted(error.to_string())),
            ReceiveAllocation::DecompressedBuffer => self.0.reserve(IngressStage::DecompressedFrame, bytes).map(|lease| Arc::new(ReceiveLease { _lease: lease }) as Arc<dyn ReceiveReservation>).map_err(|error| Status::resource_exhausted(error.to_string())),
            _ => Err(Status::unimplemented("Unknown receive allocation boundary")),
        }
    }
}

pub(super) struct Response {
    pub frame: rpc::QueryResponse,
    pub leases: Vec<Arc<dyn IngressReservation>>,
}

pub(super) struct QueryCodec(pub Arc<dyn IngressAdmission>, pub Option<usize>);
pub(super) struct QueryDecoder(Arc<dyn IngressAdmission>, Option<usize>);
pub(super) struct QueryEncoder;
impl Codec for QueryCodec {
    type Encode = rpc::QueryRequest; type Decode = Response;
    type Encoder = QueryEncoder; type Decoder = QueryDecoder;
    fn encoder(&mut self) -> QueryEncoder { QueryEncoder }
    fn decoder(&mut self) -> QueryDecoder { QueryDecoder(self.0.clone(), self.1) }
}
impl Encoder for QueryEncoder {
    type Item = rpc::QueryRequest; type Error = Status;
    fn encode(&mut self, item: rpc::QueryRequest, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        item.encode(dst).map_err(|error| Status::internal(error.to_string()))
    }
}
impl Decoder for QueryDecoder {
    type Item = Response; type Error = Status;
    fn decode(&mut self, _: &mut DecodeBuf<'_>) -> Result<Option<Response>, Status> {
        Err(Status::internal("Query decoder requires receive reservations"))
    }
    fn receive_admission(&self) -> Option<Arc<dyn ReceiveAdmission>> { Some(Arc::new(TransportAdmission(self.0.clone(), self.1))) }
    fn buffer_settings(&self) -> BufferSettings { BufferSettings::default() }
    fn decode_with_reservations(&mut self, src: &mut DecodeBuf<'_>, transport: &[Arc<dyn ReceiveReservation>]) -> Result<Option<Response>, Status> {
        let cost = response_cost(src.chunk())?;
        let protobuf = self.0.reserve(IngressStage::ProtobufDecode, cost.protobuf).map_err(|error| Status::resource_exhausted(error.to_string()))?;
        let native = self.0.reserve(IngressStage::ValueConversion, cost.native).map_err(|error| Status::resource_exhausted(error.to_string()))?;
        let frame = rpc::QueryResponse::decode(src).map_err(|error| Status::data_loss(error.to_string()))?;
        let mut leases: Vec<Arc<dyn IngressReservation>> = vec![protobuf, native];
        leases.extend(transport.iter().cloned().map(|lease| Arc::new(TransportLease { _lease: lease }) as Arc<dyn IngressReservation>));
        Ok(Some(Response { frame, leases }))
    }
}

pub(super) struct CapabilitiesResponse {
    pub response: rpc::GetCapabilitiesResponse,
    pub leases: Vec<Arc<dyn IngressReservation>>,
}
pub(super) struct CapabilitiesCodec(pub Arc<dyn IngressAdmission>);
pub(super) struct CapabilitiesDecoder(Arc<dyn IngressAdmission>);
pub(super) struct CapabilitiesEncoder;
impl Codec for CapabilitiesCodec {
    type Encode = rpc::GetCapabilitiesRequest; type Decode = CapabilitiesResponse;
    type Encoder = CapabilitiesEncoder; type Decoder = CapabilitiesDecoder;
    fn encoder(&mut self) -> CapabilitiesEncoder { CapabilitiesEncoder }
    fn decoder(&mut self) -> CapabilitiesDecoder { CapabilitiesDecoder(self.0.clone()) }
}
impl Encoder for CapabilitiesEncoder {
    type Item = rpc::GetCapabilitiesRequest; type Error = Status;
    fn encode(&mut self, item: rpc::GetCapabilitiesRequest, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        item.encode(dst).map_err(|error| Status::internal(error.to_string()))
    }
}
impl Decoder for CapabilitiesDecoder {
    type Item = CapabilitiesResponse; type Error = Status;
    fn decode(&mut self, _: &mut DecodeBuf<'_>) -> Result<Option<CapabilitiesResponse>, Status> {
        Err(Status::internal("Capabilities decoder requires receive reservations"))
    }
    fn receive_admission(&self) -> Option<Arc<dyn ReceiveAdmission>> { Some(Arc::new(TransportAdmission(self.0.clone(), None))) }
    fn decode_with_reservations(&mut self, src: &mut DecodeBuf<'_>, transport: &[Arc<dyn ReceiveReservation>]) -> Result<Option<CapabilitiesResponse>, Status> {
        let mut cost = Cost { protobuf: size_of::<rpc::GetCapabilitiesResponse>() + 4 * size_of::<Arc<dyn IngressReservation>>(), native: 0 };
        scan(src.chunk(), Shape::CapResponse, 0, &mut cost)?;
        let protobuf = self.0.reserve(IngressStage::Connection, cost.protobuf).map_err(|error| Status::resource_exhausted(error.to_string()))?;
        let response = rpc::GetCapabilitiesResponse::decode(src).map_err(|error| Status::data_loss(error.to_string()))?;
        let mut leases: Vec<Arc<dyn IngressReservation>> = vec![protobuf];
        leases.extend(transport.iter().cloned().map(|lease| Arc::new(TransportLease { _lease: lease }) as Arc<dyn IngressReservation>));
        Ok(Some(CapabilitiesResponse { response, leases }))
    }
}

/// Unary bootstrap/lifecycle responses use the identical structural scanner and
/// transport admission as finite queries. No output string is decoded first and
/// charged later. Consumers retain or transfer these leases with their output.
pub(super) trait UnaryMessage: Message + Default + Send + 'static {
    fn allocation_cost(bytes: &[u8]) -> Result<Cost, Status>;
}
macro_rules! unary_messages {
    ($shape:ident => $($message:ty),+ $(,)?) => { $(
        impl UnaryMessage for $message {
            fn allocation_cost(bytes: &[u8]) -> Result<Cost, Status> {
                let mut cost = Cost { protobuf: size_of::<Self>() + 4 * size_of::<Arc<dyn IngressReservation>>(), native: 0 };
                scan(bytes, Shape::$shape, 0, &mut cost)?;
                Ok(cost)
            }
        }
    )+ };
}
unary_messages!(SessionResponse => rpc::AttachSessionResponse, rpc::BeginTransactionResponse);
unary_messages!(UseResponse => rpc::UseResponse);
unary_messages!(TokensResponse => rpc::SigninResponse, rpc::SignupResponse, rpc::RefreshTokensResponse);
unary_messages!(AuthenticateResponse => rpc::AuthenticateResponse);
unary_messages!(Empty => rpc::DetachSessionResponse, rpc::SetVariableResponse, rpc::UnsetVariableResponse,
    rpc::InvalidateResponse, rpc::RevokeTokensResponse, rpc::CommitTransactionResponse, rpc::CancelTransactionResponse,
    rpc::HealthResponse, rpc::KillResponse);

pub(super) struct UnaryResponse<T> {
    pub response: T,
    pub leases: Vec<Arc<dyn IngressReservation>>,
}
pub(super) struct UnaryCodec<Q, R> { owner: Option<Arc<dyn IngressAdmission>>, marker: std::marker::PhantomData<(Q, R)> }
impl<Q, R> UnaryCodec<Q, R> {
    pub fn new(owner: Arc<dyn IngressAdmission>) -> Self { Self { owner: Some(owner), marker: std::marker::PhantomData } }
    pub fn unadmitted() -> Self { Self { owner: None, marker: std::marker::PhantomData } }
}
pub(super) struct UnaryEncoder<Q>(std::marker::PhantomData<Q>);
pub(super) struct UnaryDecoder<R> { owner: Option<Arc<dyn IngressAdmission>>, marker: std::marker::PhantomData<R> }
impl<Q: Message + Send + 'static, R: UnaryMessage> Codec for UnaryCodec<Q, R> {
    type Encode = Q; type Decode = UnaryResponse<R>;
    type Encoder = UnaryEncoder<Q>; type Decoder = UnaryDecoder<R>;
    fn encoder(&mut self) -> Self::Encoder { UnaryEncoder(std::marker::PhantomData) }
    fn decoder(&mut self) -> Self::Decoder { UnaryDecoder { owner: self.owner.clone(), marker: std::marker::PhantomData } }
}
impl<Q: Message> Encoder for UnaryEncoder<Q> {
    type Item = Q; type Error = Status;
    fn encode(&mut self, item: Q, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        item.encode(dst).map_err(|error| Status::internal(error.to_string()))
    }
}
impl<R: UnaryMessage> Decoder for UnaryDecoder<R> {
    type Item = UnaryResponse<R>; type Error = Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Status> {
        if self.owner.is_some() { return Err(Status::internal("Unary decoder requires receive reservations")); }
        R::decode(src).map(|response| Some(UnaryResponse { response, leases: Vec::new() })).map_err(|error| Status::data_loss(error.to_string()))
    }
    fn receive_admission(&self) -> Option<Arc<dyn ReceiveAdmission>> { self.owner.as_ref().map(|owner| Arc::new(TransportAdmission(owner.clone(), None)) as Arc<dyn ReceiveAdmission>) }
    fn decode_with_reservations(&mut self, src: &mut DecodeBuf<'_>, transport: &[Arc<dyn ReceiveReservation>]) -> Result<Option<Self::Item>, Status> {
        let cost = R::allocation_cost(src.chunk())?;
        let owner = self.owner.as_ref().ok_or_else(|| Status::internal("Missing unary admission owner"))?;
        let protobuf = owner.reserve(IngressStage::Connection, cost.protobuf).map_err(|error| Status::resource_exhausted(error.to_string()))?;
        let response = R::decode(src).map_err(|error| Status::data_loss(error.to_string()))?;
        let mut leases = vec![protobuf];
        leases.extend(transport.iter().cloned().map(|lease| Arc::new(TransportLease { _lease: lease }) as Arc<dyn IngressReservation>));
        Ok(Some(UnaryResponse { response, leases }))
    }
}

#[derive(Debug)]
struct ErrorLeases { _leases: Vec<Arc<dyn IngressReservation>> }
impl crate::types::ErrorResource for ErrorLeases {}
pub(super) fn retain_error(error: crate::Error, leases: Vec<Arc<dyn IngressReservation>>) -> crate::Error {
    // Refusal before any grant owns no new attachment backing. Preserve the
    // original error (including any earlier opaque owner) without allocating.
    if leases.is_empty() { return error; }
    error.with_resource(Arc::new(ErrorLeases { _leases: leases }))
}

/// Conversion uses the existing type converters; the only substituted semantic
/// producer is the explicitly owned regex compiler. Its leases join this batch.
pub(super) fn convert_value(
    value: proto::Value,
    owner: &dyn IngressAdmission,
    leases: &mut Vec<Arc<dyn IngressReservation>>,
) -> crate::Result<Value> {
    let mut refused = None;
    let result = Value::try_from_proto_with_regex(value, &mut |pattern| {
        match owner.compile_regex(pattern) {
            Ok((regex, lease)) => { leases.push(lease); Ok(regex) },
            Err(error) => { refused = Some(error); Err(std::io::Error::other("Ingress owner refused regex compilation").into()) },
        }
    });
    if let Some(error) = refused { return Err(error); }
    result.map_err(deserialization_error)
}

pub(super) fn convert_error(
    error: proto::SurrealError,
    owner: &dyn IngressAdmission,
    leases: &mut Vec<Arc<dyn IngressReservation>>,
) -> crate::Result<crate::Error> {
    let kind = error.kind_or_internal();
    let cause = error.cause.map(|cause| convert_error(*cause, owner, leases)).transpose()?;
    let details = if let Some(details) = error.details {
        if details.kind.is_empty() { None } else {
            let mut object = crate::types::Object::new();
            object.insert("kind".to_owned(), Value::String(details.kind));
            if let Some(content) = details.content {
                let mut refused = None;
                let converted = Value::try_from_proto_with_regex(content, &mut |pattern| {
                    match owner.compile_regex(pattern) {
                        Ok((regex, lease)) => { leases.push(lease); Ok(regex) },
                        Err(error) => { refused = Some(error); Err(std::io::Error::other("Ingress owner refused regex compilation").into()) },
                    }
                });
                if let Some(error) = refused { return Err(error); }
                // Preserve ordinary proto_details semantics for an unsupported
                // semantic payload, but never swallow owner admission refusal.
                if let Ok(content) = converted { object.insert("details".to_owned(), content); }
            }
            Some(Value::Object(object))
        }
    } else { None };
    let mapped = crate::Error::from_wire(error.message, Some(kind_str(kind)), details);
    Ok(match cause { Some(cause) => mapped.with_cause(cause), None => mapped })
}

pub(super) fn deserialization_error(error: impl std::fmt::Display) -> crate::Error {
	crate::Error::serialization(error.to_string(), crate::types::SerializationError::Deserialization)
}

pub(super) fn kind_str(kind: proto::ErrorKind) -> &'static str {
	use proto::ErrorKind;
	match kind {
		ErrorKind::Validation => "Validation",
		ErrorKind::Configuration => "Configuration",
		ErrorKind::Query => "Query",
		ErrorKind::Serialization => "Serialization",
		ErrorKind::NotAllowed => "NotAllowed",
		ErrorKind::NotFound => "NotFound",
		ErrorKind::AlreadyExists => "AlreadyExists",
		ErrorKind::Connection => "Connection",
		ErrorKind::Thrown => "Thrown",
		ErrorKind::Context => "Context",
		ErrorKind::Internal | ErrorKind::Unspecified => "Internal",
	}
}
