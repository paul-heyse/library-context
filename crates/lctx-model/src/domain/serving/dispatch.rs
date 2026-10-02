//! One finite route inventory supplies Rust decoding, schemas and Python registration.
use super::identity::{RequestIdentity, WireIdentity};
use super::*;
use crate::domain::{KeySink, selection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
macro_rules! routes {($($variant:ident:$name:literal=>$request:ident,$response:ident),*$(,)?)=>{
    #[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(rename_all="snake_case")]
    pub enum Tool {$($variant),*}
    impl Tool {
        pub const ALL:[Self;10]=[$(Self::$variant),*];
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
            let mut canonical=self.clone(); *canonical.page_mut()=PageRequest::default();
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
        /// Count canonical wire bytes without allocating a response buffer.
        pub fn json_len(&self)->Result<usize,WireError>{
            let mut writer=CountBytes(0);
            match self{$(Self::$variant(response)=>serde_json::to_writer(&mut writer,response)?),*}
            Ok(writer.0)
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
    SearchOperations:"search_operations"=>SearchOperationsRequest,SearchOperationsResponse,
    FindOperations:"find_operations"=>FindOperationsRequest,FindOperationsResponse,
    GetOperation:"get_operation"=>GetOperationRequest,GetOperationResponse,
    BrowseLibrary:"browse_library"=>BrowseLibraryRequest,BrowseLibraryResponse,
    GetEvidence:"get_evidence"=>GetEvidenceRequest,GetEvidenceResponse,
    SearchEvidence:"search_evidence"=>SearchEvidenceRequest,SearchEvidenceResponse,
    CompareOperations:"compare_operations"=>CompareOperationsRequest,CompareOperationsResponse,
    SearchCapabilities:"search_capabilities"=>SearchCapabilitiesRequest,SearchCapabilitiesResponse,
    GetCapability:"get_capability"=>GetCapabilityRequest,GetCapabilityResponse,
    InspectValuePaths:"inspect_value_paths"=>InspectValuePathsRequest,InspectValuePathsResponse,
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
    let limits = ResourceLimits::default();
    let response = decode_response(name, raw, expanded, &limits)?;
    let structured: Value = serde_json::from_str(&response.to_json()?)?;
    let envelope = serde_json::json!({"content":[{"type":"text","text":format!("{}: generation-bound result",response.tool().name())}],"structuredContent":structured,"isError":false});
    let encoded = serde_json::to_string(&envelope)?;
    admit_envelope(&encoded, expanded)?;
    Ok(encoded)
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
}
pub fn resources() -> Vec<ResourceDeclaration> {
    vec![ResourceDeclaration {
        uri_template: CAPABILITY_RESOURCE_TEMPLATE,
        name: "capability",
        mime_type: "text/markdown",
    }]
}
pub fn wire_identity() -> WireIdentity {
    let mut sink = KeySink::new("serving-wire/v1");
    for tool in Tool::ALL {
        sink.part(b"tool", tool.name().as_bytes());
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
        b"canonical-authored-body/attributed-assertion-evidence/v2",
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
            if r.sections.len() > 6 {
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
