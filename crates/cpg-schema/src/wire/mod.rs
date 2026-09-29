//! Rust-owned JSON contracts (ADR-0073). Shape validation never proves evidence membership.
//! Nominal identities cannot be accidentally interchanged.
//! ```compile_fail
//! use cpg_schema::{Id, wire::{PublicMemberId, SignatureId}};
//! let member=PublicMemberId::from_storage(Id::ZERO);
//! let signature:SignatureId=member;
//! ```
//! ```
//! use cpg_schema::{Id, wire::PublicMemberId};
//! let member=PublicMemberId::from_storage(Id::ZERO);
//! assert_eq!(member.storage(),Id::ZERO);
//! ```
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::{borrow::Cow, collections::BTreeMap, fmt};

mod dispatch;
mod evidence;
mod responses;
mod vocabulary;
pub use dispatch::{decode, schema};
pub use evidence::*;
pub use responses::*;
pub use vocabulary::*;
pub const FORMAT: u32 = 3;
pub const RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError(pub String);
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for WireError {}
impl From<serde_json::Error> for WireError {
    fn from(e: serde_json::Error) -> Self {
        Self(e.to_string())
    }
}
pub fn schema_for<T: JsonSchema>(output: bool) -> serde_json::Value {
    let settings = schemars::generate::SchemaSettings::draft2020_12();
    let settings = if output {
        settings.for_serialize()
    } else {
        settings.for_deserialize()
    };
    serde_json::to_value(settings.into_generator().into_root_schema_for::<T>())
        .expect("schema JSON")
}
fn normalize<T: DeserializeOwned + Serialize>(raw: &str) -> Result<String, WireError> {
    let normalized = serde_json::to_string(&serde_json::from_str::<T>(raw)?)?;
    if normalized.len() > RESPONSE_BYTES {
        return Err(WireError(
            "resource_refused: normalized wire byte budget".into(),
        ));
    }
    Ok(normalized)
}

/// A checked Unicode-scalar count, shared by decoding and schema generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Text<const MIN: usize, const MAX: usize>(String);
impl<const MIN: usize, const MAX: usize> Text<MIN, MAX> {
    pub fn new(s: String) -> Result<Self, WireError> {
        if (MIN..=MAX).contains(&s.chars().count()) {
            Ok(Self(s))
        } else {
            Err(WireError(format!(
                "text requires {MIN}..={MAX} Unicode scalars"
            )))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<const MIN: usize, const MAX: usize> fmt::Display for Text<MIN, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl<const MIN: usize, const MAX: usize> AsRef<str> for Text<MIN, MAX> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl<'de, const MIN: usize, const MAX: usize> Deserialize<'de> for Text<MIN, MAX> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl<const MIN: usize, const MAX: usize> JsonSchema for Text<MIN, MAX> {
    fn schema_name() -> Cow<'static, str> {
        format!("Text{MIN}To{MAX}").into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string", "minLength":MIN, "maxLength":MAX})
    }
}

macro_rules! nominal_id {
    ($name:ident, $inner:ty, $width:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);
        impl $name {
            pub fn from_storage(id: $inner) -> Self { Self(id) }
            pub fn storage(self) -> $inner { self.0 }
            pub fn hex(self) -> String { self.0.hex() }
            pub fn parse(s: &str) -> Result<Self, WireError> {
                <$inner>::from_hex(s).map(Self).ok_or_else(|| WireError(concat!("invalid ",stringify!($name)).into()))
            }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&self.0.hex()) }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                Self::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
            }
        }
        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static,str> { stringify!($name).into() }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({"type":"string", "minLength":$width,"maxLength":$width,"pattern":concat!("^[0-9a-fA-F]{",stringify!($width),"}$")})
            }
        }
    };
}
nominal_id!(OperationId, crate::Id, 32);
nominal_id!(PublicMemberId, crate::Id, 32);
nominal_id!(BindingId, crate::Id, 32);
nominal_id!(SignatureId, crate::Id, 32);
nominal_id!(EvidenceId, crate::Id, 32);
nominal_id!(SpanId, crate::Id, 32);
nominal_id!(ScenarioId, crate::Id, 32);
nominal_id!(DeploymentId, crate::Id, 32);
nominal_id!(TypeTermId, crate::Id, 32);
nominal_id!(SnapshotId, crate::Id, 32);
nominal_id!(CapabilityId, crate::Id, 32);
nominal_id!(GenerationDigest, crate::Digest, 64);
nominal_id!(RetrievalUnitId, crate::Id, 32);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RankSource {
    Hybrid,
    Lexical,
    Vector,
    ExactSymbol,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct FacetName(String);
impl<'de> Deserialize<'de> for FacetName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use crate::Codebook;
        let s = String::deserialize(d)?;
        if crate::codebook::OperationFacet::all()
            .iter()
            .any(|v| v.text() == s)
        {
            Ok(Self(s))
        } else {
            Err(serde::de::Error::custom("unknown facet"))
        }
    }
}
impl FacetName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for FacetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl JsonSchema for FacetName {
    fn schema_name() -> Cow<'static, str> {
        "FacetName".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        use crate::Codebook;
        json_schema!({"type":"string", "enum":crate::codebook::OperationFacet::all().iter().map(|v|v.text()).collect::<Vec<_>>()})
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FacetTerm {
    pub facet: FacetName,
    pub value: Text<1, 500>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExactPrimitive {
    None(()),
    Bool(bool),
    Int(i64),
    Str(Text<0, 500>),
}
impl ExactPrimitive {
    pub fn native(&self) -> (&'static str, String) {
        match self {
            Self::None(()) => ("none", String::new()),
            Self::Bool(v) => ("bool", v.to_string()),
            Self::Int(v) => ("int", v.to_string()),
            Self::Str(v) => ("str", v.as_str().into()),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
pub struct Limit<const MAX: u32, const DEFAULT: u32>(u32);
impl<const MAX: u32, const DEFAULT: u32> Limit<MAX, DEFAULT> {
    pub fn get(self) -> u32 {
        self.0
    }
}
impl<const MAX: u32, const DEFAULT: u32> Default for Limit<MAX, DEFAULT> {
    fn default() -> Self {
        Self(DEFAULT)
    }
}
impl<'de, const MAX: u32, const DEFAULT: u32> Deserialize<'de> for Limit<MAX, DEFAULT> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let n = u32::deserialize(d)?;
        if (1..=MAX).contains(&n) {
            Ok(Self(n))
        } else {
            Err(serde::de::Error::custom("limit outside range"))
        }
    }
}
impl<const MAX: u32, const DEFAULT: u32> JsonSchema for Limit<MAX, DEFAULT> {
    fn schema_name() -> Cow<'static, str> {
        format!("Limit{MAX}Default{DEFAULT}").into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"integer","minimum":1,"maximum":MAX})
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchCapabilitiesRequest {
    pub library: Text<1, 500>,
    pub query: Text<1, 4000>,
    #[serde(default)]
    pub limit: Limit<10, 5>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetCapabilityRequest {
    pub snapshot_id: SnapshotId,
    pub capability_id: CapabilityId,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetOperationRequest {
    pub snapshot_id: SnapshotId,
    pub operation: Text<1, 500>,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub evidence_limit: Limit<50, 20>,
    #[serde(default)]
    pub evidence_cursor: Option<Text<0, 2048>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectValuePathsRequest {
    pub snapshot_id: SnapshotId,
    pub operation: Text<1, 500>,
    pub formal: Text<1, 500>,
    pub exact_input: ExactPrimitive,
    #[serde(default)]
    pub standard_builtins: bool,
    #[serde(default)]
    pub limit: Limit<50, 20>,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindOperationsRequest {
    pub library: Text<1, 500>,
    #[serde(default)]
    pub selection: Option<Selection>,
    #[serde(default)]
    pub limit: Limit<100, 20>,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchOperationsRequest {
    pub library: Text<1, 500>,
    pub query: Text<1, 4000>,
    #[serde(default)]
    pub selection: Option<Selection>,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
    #[serde(default)]
    pub limit: Limit<100, 20>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum GetOperationResponse {
    Operation(Box<Operation>),
    Ambiguous(AmbiguousOperation),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum GetCapabilityResponse {
    Capability(Box<Capability>),
    Unavailable(CapabilityUnavailable),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum SearchCapabilitiesResponse {
    Results(Box<SearchResult>),
    Unavailable(CapabilityUnavailable),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum InspectValuePathsResponse {
    Paths(ValuePathPage),
    Unavailable(CapabilityUnavailable),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum FindOperationsResponse {
    Selected(SelectionResults),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum SearchOperationsResponse {
    Selected(SelectionResults),
}

pub fn tool_contract(name: &str) -> Result<(&'static str, &'static str), WireError> {
    match name {
        "get_evidence" => Ok(("GetEvidenceRequest", "GetEvidenceResponse")),
        "get_operation" => Ok(("GetOperationRequest", "GetOperationResponse")),
        "get_capability" => Ok(("GetCapabilityRequest", "GetCapabilityResponse")),
        "search_capabilities" => Ok(("SearchCapabilitiesRequest", "SearchCapabilitiesResponse")),
        "inspect_value_paths" => Ok(("InspectValuePathsRequest", "InspectValuePathsResponse")),
        "find_operations" => Ok(("FindOperationsRequest", "FindOperationsResponse")),
        "search_operations" => Ok(("SearchOperationsRequest", "SearchOperationsResponse")),
        _ => Err(WireError("unknown tool contract".into())),
    }
}

/// MCP requires root object schemas, including internally tagged response unions.
pub fn tool_schemas(name: &str) -> Result<(serde_json::Value, serde_json::Value), WireError> {
    let (request, response) = tool_contract(name)?;
    let mut output = schema(response, true)?;
    output["type"] = serde_json::json!("object");
    Ok((schema(request, false)?, output))
}

/// Select the explicit public union after the effect adapter has produced its typed payload.
pub fn tool_result(name: &str, raw: &str, expanded: bool) -> Result<String, WireError> {
    if raw.len() > RESPONSE_BYTES {
        return Err(WireError("resource_refused: wire byte budget".into()));
    }
    let mut value: serde_json::Value = serde_json::from_str(raw)?;
    let kind = if value["status"] == "unavailable" {
        "unavailable"
    } else {
        match name {
            "find_operations" | "search_operations" if value.get("supported").is_some() => {
                "selected"
            }
            "get_operation" if value["resolution"] == "ambiguous" => "ambiguous",
            "get_operation" => "operation",
            "get_evidence" if value.get("unit").is_some() => "retrieval_unit",
            "get_evidence" => "evidence",
            "get_capability" => "capability",
            "inspect_value_paths" => "paths",
            _ => "results",
        }
    };
    value
        .as_object_mut()
        .ok_or_else(|| WireError("expected response object".into()))?
        .insert("result_kind".into(), kind.into());
    let encoded = decode(tool_contract(name)?.1, &serde_json::to_string(&value)?)?;
    let limit = if matches!(name, "get_operation" | "get_evidence") {
        if expanded { 256 * 1024 } else { 32 * 1024 }
    } else {
        RESPONSE_BYTES
    };
    if encoded.len() > limit {
        return Err(WireError(
            if name == "get_operation" && !expanded {
                "resource_refused: operation packet exceeds byte budget; request expanded=true"
            } else {
                "resource_refused: final packet byte budget; no signature was truncated"
            }
            .into(),
        ));
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn validator(name: &str, output: bool) -> jsonschema::Validator {
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .offline()
            .should_validate_formats(false)
            .build(&schema(name, output).unwrap())
            .unwrap()
    }
    #[test]
    fn wire_requests_share_unicode_unknown_and_exact_contracts() {
        let v = validator("SearchOperationsRequest", false);
        for (query, expected) in [
            ("界".repeat(4000), true),
            ("界".repeat(4001), false),
            (String::new(), false),
        ] {
            let raw = serde_json::json!({"library":"demo","query":query});
            assert_eq!(v.is_valid(&raw), expected);
            assert_eq!(
                decode("SearchOperationsRequest", &raw.to_string()).is_ok(),
                expected
            );
        }
        for raw in [
            r#"{"kind":"bool","value":1}"#,
            r#"{"kind":"int","value":true}"#,
            r#"{"kind":"none","value":0}"#,
            r#"{"kind":"int","value":"3"}"#,
            r#"{"kind":"str","value":"x","extra":1}"#,
        ] {
            assert!(decode("ExactPrimitive", raw).is_err(), "{raw}");
            assert!(
                !validator("ExactPrimitive", false)
                    .is_valid(&serde_json::from_str::<serde_json::Value>(raw).unwrap())
            );
        }
        assert!(decode("ExactPrimitive", r#"{"kind":"int","value":1.0}"#).is_err());
        // Draft2020-12 integer is mathematical; token strictness remains a decoder check.
        assert!(
            validator("ExactPrimitive", false)
                .is_valid(&serde_json::json!({"kind":"int","value":1.0}))
        );
        assert!(decode("Selection", r#"{"unknown":true}"#).is_err());
        assert!(!validator("Selection", false).is_valid(&serde_json::json!({"unknown":true})));
    }
    #[test]
    fn wire_nominal_ids_are_hex_and_never_change_hashes() {
        let id = crate::Id([0xab; 16]);
        let nominal = PublicMemberId::parse(&id.hex().to_uppercase()).unwrap();
        assert_eq!(nominal.storage(), id);
        assert_eq!(
            serde_json::to_string(&nominal).unwrap(),
            format!("\"{}\"", id.hex())
        );
        let schema = schema_for::<PublicMemberId>(false);
        let v = jsonschema::options().offline().build(&schema).unwrap();
        assert!(v.is_valid(&serde_json::json!(id.hex())));
        assert!(!v.is_valid(&serde_json::json!("g".repeat(32))));
    }
    #[test]
    fn wire_all_tool_schemas_compile_offline_and_defaults_are_owned() {
        for name in [
            "get_operation",
            "get_capability",
            "search_capabilities",
            "inspect_value_paths",
            "find_operations",
            "search_operations",
        ] {
            let (input, output) = tool_contract(name).unwrap();
            validator(input, false);
            validator(output, true);
        }
        let decoded: serde_json::Value = serde_json::from_str(
            &decode(
                "FindOperationsRequest",
                r#"{"library":"demo","selection":{}}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(decoded["limit"], 20);
        assert_eq!(decoded["selection"]["requirements"], serde_json::json!([]));
        let bad = serde_json::json!({"library":"demo","query":"x","surprise":true});
        assert!(decode("SearchCapabilitiesRequest", &bad.to_string()).is_err());
        assert!(!validator("SearchCapabilitiesRequest", false).is_valid(&bad));
    }
}

/// Cursor scope excludes page size, but includes the resolved path and every semantic input.
pub fn value_query(request: &InspectValuePathsRequest, generation: GenerationDigest) -> String {
    use sha2::Digest;
    let raw =
        serde_json::json!({"format":FORMAT,"snapshot":request.snapshot_id,"generation":generation,
        "operation":request.operation,"formal":request.formal,"exact":request.exact_input,
        "standard_builtins":request.standard_builtins})
        .to_string();
    sha2::Sha256::digest(raw.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValueCursor {
    format: u32,
    generation: GenerationDigest,
    query: String,
    offset: usize,
}
pub fn value_offset(
    cursor: Option<&str>,
    generation: GenerationDigest,
    query: &str,
) -> Result<usize, WireError> {
    use base64::Engine;
    let Some(raw) = cursor else { return Ok(0) };
    if raw.len() > 2048 {
        return Err(WireError("invalid value-path cursor".into()));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE
        .decode(raw)
        .map_err(|_| WireError("invalid value-path cursor".into()))?;
    let c: ValueCursor = serde_json::from_slice(&bytes)?;
    if c.format != FORMAT || c.generation != generation || c.query != query {
        return Err(WireError(
            "cursor belongs to another version, generation or query".into(),
        ));
    }
    Ok(c.offset)
}
pub fn value_cursor(
    generation: GenerationDigest,
    query: String,
    offset: usize,
) -> Result<String, WireError> {
    use base64::Engine;
    Ok(
        base64::engine::general_purpose::URL_SAFE.encode(serde_json::to_vec(&ValueCursor {
            format: FORMAT,
            generation,
            query,
            offset,
        })?),
    )
}

/// Required nullable field: omission is malformed, explicit null is legitimate uncertainty.
pub fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn nullable_presence_and_cursor_scope_are_enforced() {
        let raw = serde_json::json!({"evidence_id":"11".repeat(16),"kind":"source","path":null,"start_byte":null,"end_byte":null,"text":null});
        assert!(decode("Evidence", &raw.to_string()).is_ok());
        assert!(
            jsonschema::options()
                .offline()
                .build(&schema("Evidence", false).unwrap())
                .unwrap()
                .is_valid(&raw)
        );
        for key in ["path", "start_byte", "end_byte", "text"] {
            let mut missing = raw.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(decode("Evidence", &missing.to_string()).is_err(), "{key}");
            assert!(
                !jsonschema::options()
                    .offline()
                    .build(&schema("Evidence", false).unwrap())
                    .unwrap()
                    .is_valid(&missing)
            );
        }
    }
    #[test]
    fn broader_owner_schemas_compile_offline_and_keep_spec_bytes() {
        for name in [
            "EmbeddingSpec",
            "ProjectionManifest",
            "RelationReceipt",
            "ArtifactReceipt",
            "Capabilities",
            "CoverageSummary",
        ] {
            for output in [false, true] {
                let schema = schema(name, output).unwrap();
                assert!(
                    jsonschema::options().offline().build(&schema).is_ok(),
                    "{name}"
                );
            }
        }
        let raw = include_str!("../../../../specs/embedding/qwen3-embedding-8b.json");
        let decoded = decode("EmbeddingSpec", raw).unwrap();
        assert_eq!(decoded + "\n", raw);
        assert!(
            jsonschema::options()
                .offline()
                .build(&schema("EmbeddingSpec", false).unwrap())
                .unwrap()
                .is_valid(&serde_json::from_str(raw).unwrap())
        );
        let mut bad: serde_json::Value = serde_json::from_str(raw).unwrap();
        bad["unknown"] = true.into();
        assert!(decode("EmbeddingSpec", &bad.to_string()).is_err());
        assert!(
            !jsonschema::options()
                .offline()
                .build(&schema("EmbeddingSpec", false).unwrap())
                .unwrap()
                .is_valid(&bad)
        );
    }
}

/// Schema adapter for required-but-nullable fields. Unlike `required` on Option, it keeps null.
pub struct Nullable<T>(std::marker::PhantomData<T>);
impl<T: JsonSchema> JsonSchema for Nullable<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Nullable{}", T::schema_name()).into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<Option<T>>()
    }
}

mod requirements;
pub use requirements::*;
