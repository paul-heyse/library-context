//! Checked metric values. Source literals and embedding bytes have separate contracts.
use super::{FieldValue, FlatValue, HeapSize, Key, KeySink, ModelError, Scalar};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A finite IEEE-754 metric, with one representation for zero.
///
/// Equality and hashing use canonical bits. This is a metric payload, not an analytic identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::Invalid("metric must be finite".into()));
        }
        Ok(Self(if value == 0.0 { 0 } else { value.to_bits() }))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }

    pub fn bits(self) -> u64 {
        self.0
    }
}

impl TryFrom<f64> for FiniteF64 {
    type Error = ModelError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.get())
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(f64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl HeapSize for FiniteF64 {}
impl FlatValue for FiniteF64 {}
impl FieldValue for FiniteF64 {
    const SCALAR: Scalar = Scalar::FiniteF64;
}
impl Key for FiniteF64 {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"finite-f64", &self.0.to_le_bytes());
    }
}
