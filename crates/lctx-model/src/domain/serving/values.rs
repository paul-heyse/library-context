//! Bounded transport values, separate from source-byte and canonical record codecs.
use super::WireError;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text<const MIN: usize, const MAX: usize>(String);
impl<const MIN: usize, const MAX: usize> Text<MIN, MAX> {
    pub fn new(value: impl Into<String>) -> Result<Self, WireError> {
        let value = value.into();
        if value.len() < MIN || value.len() > MAX || value.contains('\0') {
            return Err(WireError::Invalid("text byte bound or NUL".into()));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<const MIN: usize, const MAX: usize> Serialize for Text<MIN, MAX> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de, const MIN: usize, const MAX: usize> Deserialize<'de> for Text<MIN, MAX> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
impl<const MIN: usize, const MAX: usize> JsonSchema for Text<MIN, MAX> {
    fn schema_name() -> Cow<'static, str> {
        format!("Text{MIN}To{MAX}Bytes").into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","minLength":usize::from(MIN>0),"maxLength":MAX,"x-minUtf8Bytes":MIN,"x-maxUtf8Bytes":MAX,"pattern":r"^[^\u0000]*$"})
    }
}
/// Absent is distinct from JSON null; an optional supplied field must hold its declared value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Optional<T>(pub Option<T>);
impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T> Optional<T> {
    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }
    pub fn supplied(value: T) -> Self {
        Self(Some(value))
    }
}
impl<T: Serialize> Serialize for Optional<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0
            .as_ref()
            .ok_or_else(|| serde::ser::Error::custom("absent field must be omitted"))?
            .serialize(s)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Optional<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(|v| Self(Some(v)))
    }
}
impl<T: JsonSchema> JsonSchema for Optional<T> {
    fn schema_name() -> Cow<'static, str> {
        T::schema_name()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        T::json_schema(g)
    }
}
pub type Name = Text<1, 512>;
pub type QueryText = Text<1, 8192>;
pub type CursorToken = Text<1, 8192>;
/// A required field whose present value can be JSON null; absence remains a decoding error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);
impl<T: JsonSchema> JsonSchema for Nullable<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Nullable{}", T::schema_name()).into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        Option::<T>::json_schema(g)
    }
}
impl<'de, T: serde::de::DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // deserialize_any distinguishes a genuinely supplied null from Serde's missing-field
        // sentinel, which Option's own deserializer would silently turn into None.
        let supplied = serde_json::Value::deserialize(d)?;
        serde_json::from_value::<Option<T>>(supplied)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}
