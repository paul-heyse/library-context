//! One finite route inventory supplies Rust decoding, schemas and Python registration.
use super::identity::{RequestIdentity, WireIdentity};
use super::*;
use crate::domain::{KeySink, selection};
use crate::domain::resources::{Reservation, ResourceBudget};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
macro_rules! routes {($($variant:ident:$name:literal=>$request:ident,$response:ident,$description:literal),*$(,)?)=>{
    #[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(rename_all="snake_case")]
    pub enum Tool {$($variant),*}
    impl Tool {
        pub const ALL:[Self;10]=[$(Self::$variant),*];
        pub fn description(self)->&'static str {match self{$(Self::$variant=>$description),*}}
        pub fn name(self)->&'static str {match self{$(Self::$variant=>$name),*}}
        pub fn from_name(name:&str)->Result<Self,WireError>{match name{$($name=>Ok(Self::$variant),)*_=>Err(WireError::UnknownTool(name.into()))}}
        pub fn request_schema(self)->Value{match self{$(Self::$variant=>schema_for::<$request>(false)),*}}
        pub fn response_schema(self)->Value{match self{$(Self::$variant=>schema_for::<$response>(true)),*}}
    }
    #[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize)]
    pub enum Request {$($variant($request)),*}
    impl Request {
        pub fn tool(&self)->Tool{match self{$(Self::$variant(_)=>Tool::$variant),*}}
        pub fn page(&self)->&PageRequest{match self{$(Self::$variant(request)=>&request.page),*}}
        fn page_mut(&mut self)->&mut PageRequest{match self{$(Self::$variant(request)=>&mut request.page),*}}
        pub fn to_json(&self)->Result<String,WireError>{match self{$(Self::$variant(request)=>Ok(serde_json::to_string(request)?)),*}}
        pub fn canonical_identity(&self)->Result<RequestIdentity,WireError>{
            let mut canonical=self.clone(); let demand=canonical.page().evidence_demand.clone(); *canonical.page_mut()=PageRequest::default(); canonical.page_mut().evidence_demand=demand;
            if let Some(demand)=&mut canonical.page_mut().evidence_demand.0 { demand.facets.sort(); demand.facets.dedup(); }
            match &mut canonical {
                Self::GetOperation(r)=>{r.sections.sort_by_key(|s|*s as u8);r.sections.dedup();},
                Self::SearchEvidence(r)=>{r.families.sort_by_key(|f|*f as i16);r.families.dedup();},_=>{}
            }
            let mut sink=KeySink::new("serving-request/v1");
            sink.part(b"tool",canonical.tool().name().as_bytes());
            // The decoded finite DTO's declared serialization fixes field ordering and tags.
            sink.part(b"request",&postcard::to_allocvec(&canonical).map_err(|e|WireError::Invalid(e.to_string()))?);
            Ok(RequestIdentity(sink.finish()))
        }
    }
    #[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
    #[allow(clippy::large_enum_variant, reason = "Inline finite packets retain the value-size accounting used by request reservations")]
    pub enum Response {$($variant($response)),*}
    impl Response {
        pub fn tool(&self)->Tool{match self{$(Self::$variant(_)=>Tool::$variant),*}}
        pub fn to_json(&self)->Result<String,WireError>{match self{$(Self::$variant(response)=>Ok(serde_json::to_string(response)?)),*}}
        /// Exact cost of the actual text/structured MCP result, without an output buffer.
        pub fn mcp_result_len(&self)->Result<usize,WireError>{
            let summary=format!("{}: snapshot-bound result",self.tool().name()); let mut writer=CountBytes(0);
            match self{$(Self::$variant(response)=>serde_json::to_writer(&mut writer,&McpResult{content:[McpText{kind:"text",text:&summary}],structured:response,error:false})?),*};Ok(writer.0)
        }
        pub fn encode_mcp_result(&self,budget:&ResourceBudget,limit:usize)->Result<EncodedJson,WireError>{
            let summary=format!("{}: snapshot-bound result",self.tool().name());
            match self{$(Self::$variant(response)=>encode_json(&McpResult{content:[McpText{kind:"text",text:&summary}],structured:response,error:false},budget,limit,"final MCP envelope bytes")),*}
        }
        pub fn delivery_mut(&mut self)->&mut Optional<PacketEvidenceMap>{match self{$(Self::$variant(response)=>&mut response.delivery),*}}
        /// Encode the final raw response once, retaining its allocation charge until transport.
        pub fn encode_json(&self,budget:&ResourceBudget,limit:usize)->Result<EncodedJson,WireError>{
            match self{$(Self::$variant(response)=>encode_json(response,budget,limit,"complete structured response bytes")),*}
        }
    }
    pub fn decode_request(name:&str,raw:&str,limits:&ResourceLimits)->Result<Request,WireError>{
        limits.validate().map_err(|e|WireError::Invalid(e.to_string()))?;
        if raw.len() as u64>limits.expanded_response_bytes{return Err(WireError::ResourceRefused("request wire bytes".into()));}
        let tool=Tool::from_name(name)?;
        let request=match tool{$(Tool::$variant=>Request::$variant(serde_json::from_str::<$request>(raw)?)),*};
        check_closed(&serde_json::from_str::<Value>(raw)?,&serde_json::from_str::<Value>(&request.to_json()?)?)?;
        validate_request(&request,limits)?;Ok(request)
    }
    pub fn decode_response(name:&str,raw:&str,expanded:bool,limits:&ResourceLimits)->Result<Response,WireError>{
        let bound=limits.response_bytes(expanded);
        if raw.len() as u64>bound{return Err(WireError::ResourceRefused("structured response bytes".into()));}
        let response=match Tool::from_name(name)?{$(Tool::$variant=>Response::$variant(serde_json::from_str::<$response>(raw)?)),*};
        check_closed(&serde_json::from_str::<Value>(raw)?,&serde_json::from_str::<Value>(&response.to_json()?)?)?;
        Ok(response)
    }
    /// Current native JSON normalization, retaining coarse bridge names without legacy encodings.
    pub fn decode(name:&str,raw:&str)->Result<String,WireError>{match name{
        $($name|stringify!($request)=>decode_request($name,raw,&ResourceLimits::default())?.to_json(),
        stringify!($response)=>decode_response($name,raw,true,&ResourceLimits::default())?.to_json(),)*
        _=>Err(WireError::UnknownTool(name.into()))}}
    pub fn schema(name:&str,output:bool)->Result<Value,WireError>{match name{
        $($name=>Ok(if output{schema_for::<$response>(true)}else{schema_for::<$request>(false)}),
        stringify!($request)=>Ok(schema_for::<$request>(output)),stringify!($response)=>Ok(schema_for::<$response>(output)),)*
        _=>Err(WireError::UnknownTool(name.into()))}}
};}
#[derive(Serialize)]
struct McpText<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    text: &'a str,
}
#[derive(Serialize)]
struct McpResult<'a, T: Serialize> {
    content: [McpText<'a>; 1],
    #[serde(rename = "structuredContent")]
    structured: &'a T,
    #[serde(rename = "isError")]
    error: bool,
}
/// Final wire bytes and their live reservation. Transports borrow these bytes while copying;
/// Rust callers that take ownership of the String explicitly release the reservation.
#[derive(Debug)]
pub struct EncodedJson {
    bytes: String,
    _charge: Box<dyn Reservation>,
}
impl EncodedJson {
    pub fn as_str(&self) -> &str {
        &self.bytes
    }
    pub fn into_string(self) -> String {
        self.bytes
    }
}
const ENCODING_GROWTH_BYTES: usize = 8 * 1024;
struct BoundedJson {
    bytes: Vec<u8>,
    charge: Box<dyn Reservation>,
    limit: usize,
    refusal: &'static str,
    failure: Option<WireError>,
}
impl BoundedJson {
    fn append(&mut self, bytes: &[u8]) -> Result<(), WireError> {
        let end = self.bytes.len().checked_add(bytes.len())
            .filter(|end| *end <= self.limit)
            .ok_or_else(|| WireError::ResourceRefused(self.refusal.into()))?;
        let mut remaining = bytes;
        while self.bytes.len() < end {
            if self.bytes.len() == self.bytes.capacity() {
                // Geometric small growth, then bounded increments. Never reserve the maximum
                // eagerly; charge both final storage and the existing bridge-copy allowance.
                let old = self.bytes.capacity();
                let increment = old.clamp(64, ENCODING_GROWTH_BYTES);
                let capacity = old.saturating_add(increment).min(self.limit);
                let charged = capacity.checked_mul(2)
                    .ok_or_else(|| WireError::ResourceRefused(self.refusal.into()))?;
                self.charge.try_resize(charged).map_err(|_| {
                    WireError::Failure(PublicFailure::new(FailureKind::ResourceRefused))
                })?;
                self.bytes.try_reserve_exact(capacity - self.bytes.len())
                    .map_err(|_| WireError::ResourceRefused("response allocation".into()))?;
            }
            let count = remaining.len().min(self.bytes.capacity() - self.bytes.len());
            self.bytes.extend_from_slice(&remaining[..count]);
            remaining = &remaining[count..];
        }
        Ok(())
    }
}
impl std::io::Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Err(error) = self.append(bytes) {
            self.failure = Some(error);
            return Err(std::io::Error::other("bounded response encoding refused"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encode_json<T: Serialize>(value: &T, budget: &ResourceBudget, limit: usize, refusal: &'static str) -> Result<EncodedJson, WireError> {
    let charge = budget.reserve("native-complete-response", 0)
        .map_err(|_| WireError::Failure(PublicFailure::new(FailureKind::ResourceRefused)))?;
    let mut writer = BoundedJson { bytes: Vec::new(), charge, limit, refusal, failure: None };
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        return Err(writer.failure.take().unwrap_or_else(|| error.into()));
    }
    let bytes = String::from_utf8(writer.bytes)
        .map_err(|error| WireError::Invalid(error.to_string()))?;
    Ok(EncodedJson { bytes, _charge: writer.charge })
}
#[cfg(test)]
mod encoding_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bounded_encoding_preserves_exact_utf8_escaping_null_and_numeric_bytes() {
        let value = serde_json::json!({
            "text": "é🦀\n\t\"\\",
            "null": null,
            "integer": u64::MAX,
            "number": 1.25,
            "large": "α\\\n".repeat(9000),
        });
        let expected = serde_json::to_string(&value).unwrap();
        for limit in [expected.len(), expected.len() + 1] {
            let budget = ResourceBudget::fixed((limit + ENCODING_GROWTH_BYTES) * 2).unwrap();
            let encoded = encode_json(&value, &budget, limit, "exact cap").unwrap();
            assert_eq!(encoded.as_str(), expected);
            assert!(budget.reserved() >= encoded.as_str().len() * 2);
            assert!(budget.reserved() <= limit * 2);
            drop(encoded);
            assert_eq!(budget.reserved(), 0);
        }
        let budget = ResourceBudget::fixed(expected.len() * 2).unwrap();
        assert!(matches!(
            encode_json(&value, &budget, expected.len() - 1, "exact cap"),
            Err(WireError::ResourceRefused(message)) if message == "exact cap"
        ));
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn reservation_refusal_precedes_buffer_growth_and_releases_on_drop() {
        let budget = ResourceBudget::fixed(128).unwrap();
        let mut writer = BoundedJson {
            bytes: Vec::new(),
            charge: budget.reserve("test-response", 0).unwrap(),
            limit: 65536,
            refusal: "exact cap",
            failure: None,
        };
        writer.write_all(&[b'x'; 64]).unwrap();
        assert_eq!(writer.bytes.capacity(), 64);
        assert_eq!(budget.reserved(), 128);
        assert!(writer.write_all(b"x").is_err());
        assert_eq!(writer.bytes.len(), 64);
        assert_eq!(writer.bytes.capacity(), 64);
        assert!(matches!(writer.failure,
            Some(WireError::Failure(PublicFailure { kind: FailureKind::ResourceRefused, .. }))));
        drop(writer);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn small_output_does_not_reserve_configured_maximum_and_serializes_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Once(AtomicUsize);
        impl Serialize for Once {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                assert_eq!(self.0.fetch_add(1, Ordering::Relaxed), 0);
                serializer.serialize_str("small")
            }
        }
        let budget = ResourceBudget::fixed(128).unwrap();
        let value = Once(AtomicUsize::new(0));
        let encoded = encode_json(&value, &budget, 256 * 1024, "exact cap").unwrap();
        assert_eq!(encoded.as_str(), "\"small\"");
        assert_eq!(budget.peak(), Some(128));
        drop(encoded);
        assert_eq!(budget.reserved(), 0);
    }
}
struct CountBytes(usize);
impl std::io::Write for CountBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("response byte count overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
routes! {
    SearchOperations:"search_operations"=>SearchOperationsRequest,SearchOperationsResponse,"Search pinned public APIs using lexical and optional vector navigation, with finite selection evidence and unresolved results disclosed.",
    FindOperations:"find_operations"=>FindOperationsRequest,FindOperationsResponse,"Find public APIs by declared predicates. Discovery preserves supported, unresolved and conflicting candidates; Strict retains supported candidates.",
    GetOperation:"get_operation"=>GetOperationRequest,GetOperationResponse,"Resolve a canonical member or exact public path and inspect its signature, options and explicitly requested evidence sections. Ambiguity remains explicit.",
    BrowseLibrary:"browse_library"=>BrowseLibraryRequest,BrowseLibraryResponse,"Browse declared library, module or class members and finite vocabulary. Unknown ownership remains visible.",
    GetEvidence:"get_evidence"=>GetEvidenceRequest,GetEvidenceResponse,"Read original attributed bytes and their canonical derivation. Continuations remain bound to the original snapshot and source.",
    SearchEvidence:"search_evidence"=>SearchEvidenceRequest,SearchEvidenceResponse,"Search original evidence in the selected finite families; ranking does not establish API support or behavioral feasibility.",
    CompareOperations:"compare_operations"=>CompareOperationsRequest,CompareOperationsResponse,"Compare declared API requirements across selected operations while retaining each operation and context outcome.",
    SearchCapabilities:"search_capabilities"=>SearchCapabilitiesRequest,SearchCapabilitiesResponse,"Search authored capability briefs assembled from canonical assertions; optional vector navigation is disclosed.",
    GetCapability:"get_capability"=>GetCapabilityRequest,GetCapabilityResponse,"Read a canonical authored capability brief with its assertions and original evidence; no generative model runs.",
    InspectValuePaths:"inspect_value_paths"=>InspectValuePathsRequest,InspectValuePathsResponse,"Inspect finite native paths for exact public formal inputs without executing Python. Defaults and missing contexts retain explicit refusal reasons and original conditions.",
}
pub fn schema_for<T: JsonSchema>(output: bool) -> Value {
    let settings = schemars::generate::SchemaSettings::draft2020_12();
    let settings = if output {
        settings.for_serialize()
    } else {
        settings.for_deserialize()
    };
    serde_json::to_value(settings.into_generator().into_root_schema_for::<T>())
        .expect("schema JSON")
}
/// The complete structured result is accompanied by concise discovery text, never copied into it.
pub fn tool_result(name: &str, raw: &str, expanded: bool) -> Result<String, WireError> {
    Ok(encode_tool_result(name, raw, expanded)?.into_string())
}
/// Keep the bounded final MCP encoding charged until the bridge has copied its bytes.
pub fn encode_tool_result(name: &str, raw: &str, expanded: bool) -> Result<EncodedJson, WireError> {
    let limits = ResourceLimits::default();
    let response = decode_response(name, raw, expanded, &limits)?;
    let budget = ResourceBudget::fixed(limits.request_bytes as usize)
        .map_err(|_| WireError::Failure(PublicFailure::new(FailureKind::ResourceRefused)))?;
    response.encode_mcp_result(&budget, limits.response_bytes(expanded) as usize)
}
/// Apply the byte contract to the transport's actual serialized envelope, including text,
/// structured data and protocol metadata. Admission precedes any JSON allocation.
pub fn admit_envelope(encoded: &str, expanded: bool) -> Result<(), WireError> {
    if encoded.len() as u64 > ResourceLimits::default().response_bytes(expanded) {
        return Err(WireError::ResourceRefused(
            "final MCP envelope bytes".into(),
        ));
    }
    serde_json::from_str::<Value>(encoded)?;
    Ok(())
}
#[derive(Debug, Clone, Serialize)]
pub struct ToolDeclaration {
    pub name: &'static str,
    pub description: &'static str,
    pub request_schema: Value,
    pub response_schema: Value,
    pub read_only: bool,
    pub idempotent: bool,
}
pub fn tools() -> Vec<ToolDeclaration> {
    Tool::ALL
        .into_iter()
        .map(|tool| ToolDeclaration {
            name: tool.name(),
            description: tool.description(),
            request_schema: tool.request_schema(),
            response_schema: tool.response_schema(),
            read_only: true,
            idempotent: true,
        })
        .collect()
}
/// The authored capability body uses the same admitted closure as `get_capability`.
pub const CAPABILITY_RESOURCE_TEMPLATE: &str = "lctx://capability/{capability}";
#[derive(Debug, Clone, Serialize)]
pub struct ResourceDeclaration {
    pub uri_template: &'static str,
    pub name: &'static str,
    pub mime_type: &'static str,
    pub description: &'static str,
}
pub fn resources() -> Vec<ResourceDeclaration> {
    vec![ResourceDeclaration {
        uri_template: CAPABILITY_RESOURCE_TEMPLATE,
        name: "capability",
        mime_type: "text/markdown",
        description: "Canonical authored capability body with snapshot and attributed assertion evidence. The URI is relative to this process and its pinned snapshot.",
    }]
}
pub fn wire_identity() -> WireIdentity {
    let mut sink = KeySink::new("serving-wire/v1");
    sink.part(
        b"failure-schema",
        &serde_json::to_vec(&schema_for::<PublicFailure>(true)).expect("failure schema"),
    );
    for kind in FailureKind::ALL {
        sink.part(b"failure-kind", kind.name().as_bytes());
        sink.part(b"failure-message", kind.message().as_bytes());
    }
    sink.part(b"failure-envelope",b"admitted-original-grant/tool-isError-meta-lctx_failure/resource-error-data/internal-error/v1");
    for tool in Tool::ALL {
        sink.part(b"tool", tool.name().as_bytes());
        sink.part(b"description", tool.description().as_bytes());
        sink.part(
            b"decode-schema",
            &serde_json::to_vec(&tool.request_schema()).expect("schema bytes"),
        );
        sink.part(
            b"encode-schema",
            &serde_json::to_vec(&tool.response_schema()).expect("schema bytes"),
        );
    }
    sink.part(
        b"resource-presentation",
        b"canonical-authored-body/snapshot/process-relative/attributed-assertion-evidence/v3",
    );
    sink.part(
        b"resource-inventory",
        &serde_json::to_vec(&resources()).expect("resource bytes"),
    );
    WireIdentity(sink.finish())
}
/// Native canonical selection enums intentionally have existing Serde encodings. This boundary
/// additionally refuses unknown nested object fields that their recovery codecs would ignore.
fn check_closed(input: &Value, decoded: &Value) -> Result<(), WireError> {
    match (input, decoded) {
        (Value::Object(input), Value::Object(decoded)) => {
            for (key, value) in input {
                let expected = decoded
                    .get(key)
                    .ok_or_else(|| WireError::Invalid(format!("unknown field {key}")))?;
                check_closed(value, expected)?;
            }
        }
        (Value::Array(input), Value::Array(decoded)) => {
            for (a, b) in input.iter().zip(decoded) {
                check_closed(a, b)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn validate_selection(
    selection: &selection::Selection,
    limits: &ResourceLimits,
) -> Result<(), WireError> {
    if selection.requirements.len() > limits.maximum_requirements as usize {
        return Err(WireError::ResourceRefused("requirements".into()));
    }
    for requirement in &selection.requirements {
        requirement
            .predicate
            .validate()
            .map_err(|e| WireError::Invalid(e.to_string()))?;
    }
    Ok(())
}
fn validate_selector(selector: &OperationSelector) -> Result<(), WireError> {
    if let OperationSelector::PublicPath { path } = selector
        && (path.is_empty() || path.len() > 128)
    {
        return Err(WireError::Invalid("public path extent".into()));
    }
    Ok(())
}
fn validate_request(request: &Request, limits: &ResourceLimits) -> Result<(), WireError> {
    if request.page().size == 0 || request.page().size > limits.maximum_page_rows {
        return Err(WireError::ResourceRefused("page rows".into()));
    }
    if let Some(demand) = &request.page().evidence_demand.0 {
        if demand.facets.len() > 9 || demand.maximum_followups > limits.maximum_page_rows {
            return Err(WireError::ResourceRefused("evidence demand extent".into()));
        }
        let facets: std::collections::BTreeSet<_> = demand.facets.iter().collect();
        if facets.len() != demand.facets.len() {
            return Err(WireError::Invalid("duplicate evidence facet".into()));
        }
    }
    match request {
        Request::SearchOperations(r) => validate_selection(&r.selection.0, limits)?,
        Request::FindOperations(r) => validate_selection(&r.selection.0, limits)?,
        Request::BrowseLibrary(r) => validate_selection(&r.selection.0, limits)?,
        Request::CompareOperations(r) => {
            if r.operations.is_empty()
                || r.operations.len() > limits.maximum_comparison_candidates as usize
            {
                return Err(WireError::ResourceRefused("comparison candidates".into()));
            }
            validate_selection(&r.selection.0, limits)?;
            for operation in &r.operations {
                validate_selector(operation)?;
            }
        }
        Request::GetOperation(r) => {
            validate_selector(&r.operation)?;
            if r.reference_parameter.0.is_some()
                && !r.sections.contains(&OperationSection::IncomingReferences)
            {
                return Err(WireError::Invalid(
                    "formal reference target requires incoming reference section".into(),
                ));
            }
            if r.sections.contains(&OperationSection::CallableComparison)
                != r.comparison.0.is_some()
            {
                return Err(WireError::Invalid(
                    "comparison section requires explicit variants and context".into(),
                ));
            }
            if r.sections.len() > 10 {
                return Err(WireError::Invalid("operation sections".into()));
            }
        }
        Request::InspectValuePaths(r) => {
            if r.inputs.len() > limits.maximum_requirements as usize {
                return Err(WireError::ResourceRefused("exact input bindings".into()));
            }
            let mut formals = std::collections::BTreeSet::new();
            for input in &r.inputs {
                input
                    .value
                    .validate()
                    .map_err(|e| WireError::Invalid(e.to_string()))?;
                if !formals.insert(input.formal) {
                    return Err(WireError::Invalid("duplicate exact input formal".into()));
                }
            }
        }
        _ => {}
    }
    Ok(())
}
