use super::Record;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A nominal semantic identifier. It cannot be converted into another target's identifier.
///
/// ```compile_fail
/// use lctx_model::domain::{Id, source::{SourceArtifact, Module}};
/// fn wrong(source: Id<SourceArtifact>) -> Id<Module> { source }
/// ```
///
/// ```
/// use lctx_model::domain::{Id, source::SourceArtifact};
/// fn same(source: Id<SourceArtifact>) -> Id<SourceArtifact> { source }
/// ```
pub struct Id<T> {
    bytes: [u8; 16],
    target: PhantomData<fn() -> T>,
}
impl<T> Id<T> {
    pub fn bytes(&self) -> &[u8; 16] {
        &self.bytes
    }
    pub fn hex(&self) -> String {
        self.bytes.iter().map(|v| format!("{v:02x}")).collect()
    }
}
impl<T: Record> Id<T> {
    pub fn of(key: &T::Key) -> Self {
        let mut sink = KeySink::new(T::NAME);
        key.encode(&mut sink);
        Self::from_sink(sink)
    }
    pub(crate) fn of_record(record: &T) -> Self {
        let mut sink = KeySink::new(T::NAME);
        record.write_key(&mut sink);
        Self::from_sink(sink)
    }
    fn from_sink(sink: KeySink) -> Self {
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&sink.finish().0[..16]);
        Self {
            bytes,
            target: PhantomData,
        }
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.bytes == rhs.bytes
    }
}
impl<T> Eq for Id<T> {}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.bytes.cmp(&rhs.bytes)
    }
}
impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.bytes.hash(h);
    }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}
impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.bytes.serialize(s)
    }
}
impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(d)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 16-byte nominal ID"))?;
        Ok(Self {
            bytes,
            target: PhantomData,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(pub [u8; 32]);
impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}
impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(d)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 32-byte digest"))?;
        Ok(Self(bytes))
    }
}

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }
    pub fn hex(&self) -> String {
        self.0.iter().map(|v| format!("{v:02x}")).collect()
    }
}

/// Incremental original-byte digest; identical to ContentHash::of for every fragmentation.
#[derive(Default)]
pub struct ContentHasher(blake3::Hasher);
impl ContentHasher {
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    pub fn finish(self) -> ContentHash {
        ContentHash(*self.0.finalize().as_bytes())
    }
}

/// Structural encoding: each scalar is tagged and length delimited; containers carry arity.
/// There is no caller-supplied stringification or provider-local index identity.
pub struct KeySink(blake3::Hasher);
impl KeySink {
    pub fn new(namespace: &str) -> Self {
        let mut sink = Self(blake3::Hasher::new());
        sink.part(b"domain", b"lctx-semantic/v3");
        sink.part(b"type", namespace.as_bytes());
        sink
    }
    pub fn part(&mut self, tag: &[u8], bytes: &[u8]) {
        self.0.update(&(tag.len() as u64).to_le_bytes());
        self.0.update(tag);
        self.0.update(&(bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
    }
    pub fn finish(self) -> ContentHash {
        ContentHash(*self.0.finalize().as_bytes())
    }
}
pub trait Key {
    fn encode(&self, sink: &mut KeySink);
}
impl Key for String {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"text", self.as_bytes());
    }
}
impl Key for bool {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"bool", &[u8::from(*self)]);
    }
}
macro_rules! numeric_key {
    ($($t:ty),*) => { $(impl Key for $t {
        fn encode(&self, sink: &mut KeySink) { sink.part(stringify!($t).as_bytes(), &self.to_le_bytes()); }
    })* };
}
numeric_key!(i16, i32, i64);
impl<T: Record> Key for Id<T> {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(T::NAME.as_bytes(), &self.bytes);
    }
}
impl Key for ContentHash {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"digest", &self.0);
    }
}
impl<T: Key> Key for Option<T> {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"option", &[u8::from(self.is_some())]);
        if let Some(value) = self {
            value.encode(sink);
        }
    }
}
impl<T: Key> Key for Vec<T> {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"list", &(self.len() as u64).to_le_bytes());
        for value in self {
            value.encode(sink);
        }
    }
}

/// A reference to one arm of a sum, validated at construction and by a generated composite FK.
#[derive(Debug)]
pub struct ArmId<T: super::SumRecord, const CODE: i16>(Id<T>);
impl<T: super::SumRecord, const CODE: i16> ArmId<T, CODE> {
    pub fn of(value: &T) -> Result<Self, super::ModelError> {
        if value.tag() != CODE {
            return Err(super::ModelError::Invalid("wrong sum subtype".into()));
        }
        Ok(Self(value.id()))
    }
    pub fn id(self) -> Id<T> {
        self.0
    }
}
impl<T: super::SumRecord, const CODE: i16> Copy for ArmId<T, CODE> {}
impl<T: super::SumRecord, const CODE: i16> Clone for ArmId<T, CODE> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: super::SumRecord, const CODE: i16> PartialEq for ArmId<T, CODE> {
    fn eq(&self, rhs: &Self) -> bool {
        self.0 == rhs.0
    }
}
impl<T: super::SumRecord, const CODE: i16> Eq for ArmId<T, CODE> {}
impl<T: super::SumRecord, const CODE: i16> Hash for ArmId<T, CODE> {
    fn hash<H: Hasher>(&self, sink: &mut H) {
        self.0.hash(sink);
    }
}
impl<T: super::SumRecord, const CODE: i16> Serialize for ArmId<T, CODE> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}
impl<'de, T: super::SumRecord, const CODE: i16> Deserialize<'de> for ArmId<T, CODE> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self(Id::deserialize(d)?))
    }
}
impl<T: super::SumRecord, const CODE: i16> Key for ArmId<T, CODE> {
    fn encode(&self, sink: &mut KeySink) {
        self.0.encode(sink);
        CODE.encode(sink);
    }
}
impl<T: super::SumRecord, const CODE: i16> super::FlatValue for ArmId<T, CODE> {}
impl<T: super::SumRecord, const CODE: i16> super::FieldValue for ArmId<T, CODE> {
    const SCALAR: super::Scalar = super::Scalar::Id;
    fn target() -> Option<(std::any::TypeId, &'static str)> {
        Some((std::any::TypeId::of::<T>(), T::NAME))
    }
    fn subtype() -> Option<i16> {
        Some(CODE)
    }
}

/// Original evidence bytes. Binary is distinct from an integer list and never requires UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvidenceBytes(pub Vec<u8>);
impl Key for EvidenceBytes {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"binary", &self.0);
    }
}
impl Serialize for EvidenceBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}
impl<'de> Deserialize<'de> for EvidenceBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BytesVisitor;
        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = EvidenceBytes;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("binary evidence")
            }
            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                Ok(EvidenceBytes(bytes.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
                Ok(EvidenceBytes(bytes))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut bytes = Vec::new();
                while let Some(byte) = seq.next_element()? {
                    bytes.push(byte);
                }
                Ok(EvidenceBytes(bytes))
            }
        }
        deserializer.deserialize_byte_buf(BytesVisitor)
    }
}

/// Unicode text whose full UTF-8 bytes survive stores that cannot represent NUL in text columns.
/// The private value guarantees that binary decoding cannot admit invalid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Utf8Text(String);
impl Utf8Text {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
}
impl From<String> for Utf8Text {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for Utf8Text {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl std::ops::Deref for Utf8Text {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Utf8Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl PartialEq<&str> for Utf8Text {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
impl Key for Utf8Text {
    fn encode(&self, sink: &mut KeySink) {
        self.0.encode(sink);
    }
}
impl Serialize for Utf8Text {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0.as_bytes())
    }
}
impl<'de> Deserialize<'de> for Utf8Text {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = EvidenceBytes::deserialize(deserializer)?;
        String::from_utf8(bytes.0)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}
