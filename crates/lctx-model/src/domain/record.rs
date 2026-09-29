use std::{any::TypeId, collections::HashMap, sync::Arc};
use arrow_array::RecordBatch;
use arrow_schema::{DataType, Field as ArrowField, Schema, SchemaRef};
use super::{ContentHash, Id, Key, ModelError, ValidatedModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar { Text, Bool, Int16, Int32, Int64, Id, Digest, Binary }
impl Scalar {
    fn arrow(self) -> DataType {
        match self {
            Self::Text => DataType::Utf8, Self::Bool => DataType::Boolean,
            Self::Int16 => DataType::Int16, Self::Int32 => DataType::Int32, Self::Int64 => DataType::Int64,
            Self::Id => DataType::FixedSizeBinary(16), Self::Digest => DataType::FixedSizeBinary(32), Self::Binary => DataType::Binary,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Field {
    name: &'static str,
    scalar: Scalar,
    nullable: bool,
    list: bool,
    key: bool,
    provenance: bool,
    subtype: Option<i16>,
    codes: &'static [(i16, &'static str)],
    target: Option<(TypeId, &'static str)>,
}
impl Field {
    pub fn of<T: FieldValue>(name: &'static str, key: bool, provenance: bool) -> Self {
        Self { name, scalar: T::SCALAR, nullable: T::NULLABLE, list: T::LIST, key, provenance, subtype: T::subtype(), target: T::target(), codes: T::codes() }
    }
    pub fn name(&self) -> &'static str { self.name }
    pub fn scalar(&self) -> Scalar { self.scalar }
    pub fn nullable(&self) -> bool { self.nullable }
    pub fn list(&self) -> bool { self.list }
    pub fn is_key(&self) -> bool { self.key }
    pub fn is_provenance(&self) -> bool { self.provenance }
    pub fn subtype(&self) -> Option<i16> { self.subtype }
    pub fn codes(&self) -> &'static [(i16, &'static str)] { self.codes }
    pub fn target(&self) -> Option<(TypeId, &'static str)> { self.target }
    pub fn arrow(&self) -> ArrowField {
        let ty = if self.list { DataType::List(Arc::new(ArrowField::new("item", self.scalar.arrow(), false))) } else { self.scalar.arrow() };
        ArrowField::new(self.name, ty, self.nullable)
    }
}
pub trait FlatValue: FieldValue {}
pub trait FieldValue {
    fn subtype() -> Option<i16> { None }
    const SCALAR: Scalar;
    const NULLABLE: bool = false;
    const LIST: bool = false;
    fn codes() -> &'static [(i16, &'static str)] { &[] }
    fn target() -> Option<(TypeId, &'static str)> { None }
}
macro_rules! scalar {
    ($t:ty, $s:ident) => { impl FlatValue for $t {} impl FieldValue for $t { const SCALAR: Scalar = Scalar::$s; } };
}
scalar!(String, Text); scalar!(bool, Bool); scalar!(i16, Int16); scalar!(i32, Int32); scalar!(i64, Int64);
scalar!(ContentHash, Digest);
scalar!(super::EvidenceBytes, Binary);
impl<T: Record> FlatValue for Id<T> {}
impl<T: Record> FieldValue for Id<T> {
    const SCALAR: Scalar = Scalar::Id;
    fn target() -> Option<(TypeId, &'static str)> { Some((TypeId::of::<T>(), T::NAME)) }
}
impl<T: FlatValue> FieldValue for Option<T> {
    const SCALAR: Scalar = T::SCALAR;
    const NULLABLE: bool = true;
    fn subtype() -> Option<i16> { T::subtype() }
    fn codes() -> &'static [(i16, &'static str)] { T::codes() }
    const LIST: bool = T::LIST;
    fn target() -> Option<(TypeId, &'static str)> { T::target() }
}
// Only primitive collections are admitted. Reference collections must be relationship records.
macro_rules! primitive_list {
    ($($t:ty),*) => { $(impl FlatValue for Vec<$t> {} impl FieldValue for Vec<$t> {
        const SCALAR: Scalar = <$t as FieldValue>::SCALAR;
        const LIST: bool = true;
    })* };
}
primitive_list!(String, bool, i16, i32, i64);

#[derive(Debug, Clone)]
pub struct ArmField { pub name: &'static str, pub required: bool }
#[derive(Debug, Clone)]
pub struct Arm { pub code: i16, pub fields: Vec<ArmField> }
#[derive(Debug, Clone)]
pub struct Sum { pub tag: &'static str, pub arms: Vec<Arm> }

pub trait SumRecord: Record { fn tag(&self) -> i16; }

pub trait Record: Sized + Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static {
    type Key: Key + Clone + Eq + std::hash::Hash;
    const NAME: &'static str;
    const CONTRACT: &'static str;
    const OWNER: &'static str;
    const SEMANTIC_SOURCE: &'static [u8];
    fn key(&self) -> Self::Key;
    fn fields() -> Vec<Field>;
    /// Hash every semantic field, including non-key payload, independently of Arrow framing.
    fn content_digest(&self) -> ContentHash;
    fn sum() -> Option<Sum> { None }
    fn validate(&self) -> Result<(), ModelError>;
    fn encode(rows: &[Self]) -> Result<RecordBatch, ModelError>;
    fn decode(batch: &RecordBatch) -> Result<Vec<Self>, ModelError>;
    fn id(&self) -> Id<Self> { Id::of(&self.key()) }
    fn schema() -> SchemaRef {
        Arc::new(Schema::new(std::iter::once(ArrowField::new("id", DataType::FixedSizeBinary(16), false))
            .chain(Self::fields().iter().map(Field::arrow)).collect::<Vec<_>>()))
    }
}

/// A model-checked, identity-checked typed boundary. Equal keys with different payload refuse.
#[derive(Debug)]
pub struct Batch<R: Record> { rows: Vec<R>, arrow: RecordBatch }
impl<R: Record> Batch<R> {
    pub fn new(model: &ValidatedModel, rows: Vec<R>) -> Result<Self, ModelError> {
        model.require::<R>()?;
        let mut unique = HashMap::new();
        let mut canonical = Vec::new();
        for row in rows {
            row.validate()?;
            let key = row.key();
            if let Some(old) = unique.get(&key) {
                if old != &row { return Err(ModelError::Conflict(R::NAME)); }
            } else {
                unique.insert(key, row.clone());
                canonical.push(row);
            }
        }
        canonical.sort_by_key(Record::id);
        let arrow = R::encode(&canonical)?;
        Ok(Self { rows: canonical, arrow })
    }
    pub fn read(model: &ValidatedModel, arrow: &RecordBatch) -> Result<Self, ModelError> {
        Self::new(model, R::decode(arrow)?)
    }
    pub fn rows(&self) -> &[R] { &self.rows }
    pub fn arrow(&self) -> &RecordBatch { &self.arrow }
}
