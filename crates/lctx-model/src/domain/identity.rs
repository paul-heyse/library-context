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
        self.bytes.as_slice().serialize(s)
    }
}
impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = deserialize_fixed::<D, 16>(d)?;
        Ok(Self {
            bytes,
            target: PhantomData,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 32]);
impl Serialize for ContentHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.as_slice().serialize(s)
    }
}
impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = deserialize_fixed::<D, 32>(d)?;
        Ok(Self(bytes))
    }
}

/// Fixed-width IDs use Serde sequences, which both Arrow FixedSizeBinary and binary codecs
/// support. The visitor admits exactly N elements without allocating from an untrusted hint.
fn deserialize_fixed<'de, D: Deserializer<'de>, const N: usize>(d: D) -> Result<[u8; N], D::Error> {
    struct Fixed<const N: usize>;
    impl<'de, const N: usize> serde::de::Visitor<'de> for Fixed<N> {
        type Value = [u8; N];
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "exactly {N} bytes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence.size_hint().is_some_and(|n| n != N) {
                return Err(serde::de::Error::custom("incorrect nominal byte width"));
            }
            let mut bytes = [0; N];
            for byte in &mut bytes {
                *byte = sequence
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("truncated nominal bytes"))?;
            }
            if sequence.next_element::<u8>()?.is_some() {
                return Err(serde::de::Error::custom("excess nominal bytes"));
            }
            Ok(bytes)
        }
    }
    d.deserialize_seq(Fixed::<N>)
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
        framed_part(tag, bytes, |fragment| {
            self.0.update(fragment);
        });
    }
    pub fn finish(self) -> ContentHash {
        ContentHash(*self.0.finalize().as_bytes())
    }
}
// Both direct hashing and prepared replay use this sole scalar framing recipe.
fn framed_part(tag: &[u8], bytes: &[u8], mut emit: impl FnMut(&[u8])) {
    emit(&(tag.len() as u64).to_le_bytes());
    emit(tag);
    emit(&(bytes.len() as u64).to_le_bytes());
    emit(bytes);
}

/// Opaque, already framed sequence of content hashes. No list or aggregate-hash frame is added.
/// The invoking owner reserves capacity before construction and retains that reservation.
pub(crate) struct PreparedContentHashes {
    bytes: Vec<u8>,
    limit: usize,
}
impl PreparedContentHashes {
    pub(crate) fn encoded_size(count: usize) -> Option<usize> {
        count.checked_mul(8 + b"digest".len() + 8 + 32)
    }
    pub(crate) fn try_new(count: usize) -> Result<Self, super::ModelError> {
        let limit = Self::encoded_size(count).ok_or_else(|| {
            super::ModelError::Invalid("content hash sequence size overflow".into())
        })?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(limit).map_err(|_| {
            super::ModelError::Invalid("content hash sequence allocation failed".into())
        })?;
        Ok(Self { bytes, limit })
    }
    pub(crate) fn capacity(&self) -> usize {
        self.bytes.capacity()
    }
    pub(crate) fn push(&mut self, hash: ContentHash) -> Result<(), super::ModelError> {
        if self
            .bytes
            .len()
            .checked_add(Self::encoded_size(1).expect("one frame"))
            .is_none_or(|end| end > self.limit)
        {
            return Err(super::ModelError::Invalid(
                "content hash sequence exceeds reserved size".into(),
            ));
        }
        framed_part(b"digest", &hash.0, |fragment| {
            self.bytes.extend_from_slice(fragment)
        });
        Ok(())
    }
}
impl Key for PreparedContentHashes {
    fn encode(&self, sink: &mut KeySink) {
        sink.0.update(&self.bytes);
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
    fn semantic_reference(&self,field:&'static str)->Option<super::SemanticReference>{Some(super::SemanticReference{field,target:T::NAME,key:*self.0.bytes(),subtype:Some(CODE)})}
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

#[cfg(test)]
mod prepared_hash_tests {
    use super::*;
    #[test]
    fn prepared_hashes_preserve_original_framing_and_order() {
        // Independent, explicit byte framing fixes the v3 contract, including empty sequences.
        for hashes in [
            vec![],
            vec![ContentHash([0; 32])],
            vec![
                ContentHash([1; 32]),
                ContentHash([2; 32]),
                ContentHash([1; 32]),
            ],
        ] {
            let mut expected = Vec::new();
            for (tag, value) in [
                (b"domain".as_slice(), b"lctx-semantic/v3".as_slice()),
                (b"type".as_slice(), b"inventory-control".as_slice()),
                (b"text".as_slice(), b"class/context".as_slice()),
            ] {
                expected.extend_from_slice(&(tag.len() as u64).to_le_bytes());
                expected.extend_from_slice(tag);
                expected.extend_from_slice(&(value.len() as u64).to_le_bytes());
                expected.extend_from_slice(value);
            }
            for hash in &hashes {
                expected.extend_from_slice(&6_u64.to_le_bytes());
                expected.extend_from_slice(b"digest");
                expected.extend_from_slice(&32_u64.to_le_bytes());
                expected.extend_from_slice(&hash.0);
            }
            expected.extend_from_slice(&4_u64.to_le_bytes());
            expected.extend_from_slice(b"text");
            expected.extend_from_slice(&6_u64.to_le_bytes());
            expected.extend_from_slice(b"symbol");
            let mut direct = KeySink::new("inventory-control");
            "class/context".to_owned().encode(&mut direct);
            let mut prepared = PreparedContentHashes::try_new(hashes.len()).unwrap();
            for hash in &hashes {
                hash.encode(&mut direct);
                prepared.push(*hash).unwrap();
            }
            "symbol".to_owned().encode(&mut direct);
            let mut replay = KeySink::new("inventory-control");
            "class/context".to_owned().encode(&mut replay);
            prepared.encode(&mut replay);
            "symbol".to_owned().encode(&mut replay);
            let expected = ContentHash::of(&expected);
            assert_eq!(direct.finish(), expected);
            assert_eq!(replay.finish(), expected);
            assert!(prepared.push(ContentHash([3; 32])).is_err());
        }
        assert!(PreparedContentHashes::encoded_size(usize::MAX).is_none());
    }
}
