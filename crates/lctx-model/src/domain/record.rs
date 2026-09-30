use super::resources::{Reservation, ResourceBudget};
use super::{ContentHash, EvidenceBytes, Id, Key, ModelError, ValidatedModel};
use arrow_array::RecordBatch;
use arrow_schema::{DataType, Field as ArrowField, Schema, SchemaRef};
use std::{any::TypeId, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    Text,
    Bool,
    Int16,
    Int32,
    Int64,
    Id,
    Digest,
    Binary,
}
impl Scalar {
    fn arrow(self) -> DataType {
        match self {
            Self::Text => DataType::Utf8,
            Self::Bool => DataType::Boolean,
            Self::Int16 => DataType::Int16,
            Self::Int32 => DataType::Int32,
            Self::Int64 => DataType::Int64,
            Self::Id => DataType::FixedSizeBinary(16),
            Self::Digest => DataType::FixedSizeBinary(32),
            Self::Binary => DataType::Binary,
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
        Self {
            name,
            scalar: T::SCALAR,
            nullable: T::NULLABLE,
            list: T::LIST,
            key,
            provenance,
            subtype: T::subtype(),
            target: T::target(),
            codes: T::codes(),
        }
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn scalar(&self) -> Scalar {
        self.scalar
    }
    pub fn nullable(&self) -> bool {
        self.nullable
    }
    pub fn list(&self) -> bool {
        self.list
    }
    pub fn is_key(&self) -> bool {
        self.key
    }
    pub fn is_provenance(&self) -> bool {
        self.provenance
    }
    pub fn subtype(&self) -> Option<i16> {
        self.subtype
    }
    pub fn codes(&self) -> &'static [(i16, &'static str)] {
        self.codes
    }
    pub fn target(&self) -> Option<(TypeId, &'static str)> {
        self.target
    }
    pub fn arrow(&self) -> ArrowField {
        let ty = if self.list {
            DataType::List(Arc::new(ArrowField::new(
                "item",
                self.scalar.arrow(),
                false,
            )))
        } else {
            self.scalar.arrow()
        };
        ArrowField::new(self.name, ty, self.nullable)
    }
}
/// Heap bytes a value owns beyond its inline size. This is allocation admission, not an
/// allocator measurement: capacities are charged, shared buffers are not discovered.
pub trait HeapSize {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl HeapSize for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl HeapSize for super::Utf8Text {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl HeapSize for EvidenceBytes {
    fn heap_bytes(&self) -> usize {
        self.0.capacity()
    }
}
impl<T: HeapSize> HeapSize for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, HeapSize::heap_bytes)
    }
}
impl<T: HeapSize> HeapSize for Vec<T> {
    fn heap_bytes(&self) -> usize {
        self.iter().fold(
            self.capacity().saturating_mul(size_of::<T>()),
            |bytes, item| bytes.saturating_add(item.heap_bytes()),
        )
    }
}
impl<T> HeapSize for Id<T> {}
impl<T: SumRecord, const CODE: i16> HeapSize for super::ArmId<T, CODE> {}
impl<T: HeapSize> HeapSize for std::collections::BTreeSet<T> {
    fn heap_bytes(&self) -> usize {
        self.iter().fold(
            self.len().saturating_mul(size_of::<T>() + 16),
            |bytes, item| bytes.saturating_add(item.heap_bytes()),
        )
    }
}
impl<K: HeapSize, V: HeapSize> HeapSize for std::collections::BTreeMap<K, V> {
    fn heap_bytes(&self) -> usize {
        self.iter().fold(
            self.len().saturating_mul(size_of::<(K, V)>() + 16),
            |bytes, (k, v)| {
                bytes
                    .saturating_add(k.heap_bytes())
                    .saturating_add(v.heap_bytes())
            },
        )
    }
}
macro_rules! inline_only { ($($t:ty),*) => { $(impl HeapSize for $t {})* }; }
inline_only!(
    ContentHash,
    bool,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64,
    usize,
    char
);
impl<const N: usize> HeapSize for [u8; N] {}
/// A borrow owns no heap; the referent is accounted by its owner.
impl<T: ?Sized> HeapSize for &T {}
macro_rules! tuple_heap {
    ($($name:ident),+) => {
        impl<$($name: HeapSize),+> HeapSize for ($($name,)+) {
            #[allow(non_snake_case, reason = "Shared generated contracts and fixtures require this scoped exception")]
            fn heap_bytes(&self) -> usize { let ($($name,)+) = self; 0usize $(.saturating_add($name.heap_bytes()))+ }
        }
    };
}
tuple_heap!(A);
tuple_heap!(A, B);
tuple_heap!(A, B, C);
tuple_heap!(A, B, C, D);
tuple_heap!(A, B, C, D, E);

/// A stored codebook enum: each variant's append-only code, and back.
pub trait Codebook: Sized {
    fn code(&self) -> i16;
    fn from_code(code: i16) -> Option<Self>;
}
pub trait FlatValue: FieldValue {}
pub trait FieldValue: HeapSize {
    fn subtype() -> Option<i16> {
        None
    }
    const SCALAR: Scalar;
    const NULLABLE: bool = false;
    const LIST: bool = false;
    fn codes() -> &'static [(i16, &'static str)] {
        &[]
    }
    fn target() -> Option<(TypeId, &'static str)> {
        None
    }
}
macro_rules! scalar {
    ($t:ty, $s:ident) => {
        impl FlatValue for $t {}
        impl FieldValue for $t {
            const SCALAR: Scalar = Scalar::$s;
        }
    };
}
scalar!(String, Text);
scalar!(bool, Bool);
scalar!(i16, Int16);
scalar!(i32, Int32);
scalar!(i64, Int64);
scalar!(ContentHash, Digest);
scalar!(super::EvidenceBytes, Binary);
scalar!(super::Utf8Text, Binary);
impl<T: Record> FlatValue for Id<T> {}
impl<T: Record> FieldValue for Id<T> {
    const SCALAR: Scalar = Scalar::Id;
    fn target() -> Option<(TypeId, &'static str)> {
        Some((TypeId::of::<T>(), T::NAME))
    }
}
impl<T: FlatValue> FieldValue for Option<T> {
    const SCALAR: Scalar = T::SCALAR;
    const NULLABLE: bool = true;
    fn subtype() -> Option<i16> {
        T::subtype()
    }
    fn codes() -> &'static [(i16, &'static str)] {
        T::codes()
    }
    const LIST: bool = T::LIST;
    fn target() -> Option<(TypeId, &'static str)> {
        T::target()
    }
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
pub struct ArmField {
    pub name: &'static str,
    pub required: bool,
}
#[derive(Debug, Clone)]
pub struct Arm {
    pub code: i16,
    pub fields: Vec<ArmField>,
}
#[derive(Debug, Clone)]
pub struct Sum {
    pub tag: &'static str,
    pub arms: Vec<Arm>,
}

pub trait SumRecord: Record {
    fn tag(&self) -> i16;
}

pub trait Record:
    HeapSize + Sized + Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static
{
    fn derivation() -> Option<super::derivation::Derivation> {
        None
    }
    fn proof(&self) -> Option<super::derivation::Proof> {
        None
    }
    fn required_relations() -> Vec<(std::any::TypeId, &'static str)> {
        Vec::new()
    }
    /// The fact family whose coverage states this relation's completeness: an assertion's and its
    /// support's declared family; `None` for vocabulary and derived relations.
    fn family() -> Option<super::attribution::FactFamily> {
        None
    }
    /// Finite typed endpoint meanings declared by the relation that supplies their evidence.
    fn projection_roles() -> Vec<super::projection::EndpointRole> {
        Vec::new()
    }
    type Key: Key + Clone + Eq + std::hash::Hash;
    const NAME: &'static str;
    const CONTRACT: &'static str;
    const OWNER: &'static str;
    const SEMANTIC_SOURCE: &'static [u8];
    fn key(&self) -> Self::Key;
    /// Encode the declared key by reference; identity checks must not clone large key payloads.
    fn write_key(&self, sink: &mut super::KeySink);
    fn fields() -> Vec<Field>;
    fn invariants() -> Vec<super::Invariant> {
        Vec::new()
    }
    /// Hash every semantic field, including non-key payload, independently of Arrow framing.
    fn content_digest(&self) -> ContentHash;
    /// Admission size of one typed row: its inline size plus owned heap bytes.
    fn row_bytes(&self) -> usize {
        size_of::<Self>().saturating_add(self.heap_bytes())
    }
    fn sum() -> Option<Sum> {
        None
    }
    fn validate(&self) -> Result<(), ModelError>;
    fn encode(rows: &[Self]) -> Result<RecordBatch, ModelError>;
    fn decode(batch: &RecordBatch) -> Result<Vec<Self>, ModelError>;
    fn id(&self) -> Id<Self> {
        Id::of_record(self)
    }
    fn schema() -> SchemaRef {
        Arc::new(Schema::new(
            std::iter::once(ArrowField::new("id", DataType::FixedSizeBinary(16), false))
                .chain(Self::fields().iter().map(Field::arrow))
                .collect::<Vec<_>>(),
        ))
    }
}

/// Fixed Arrow bytes per row: the identity column plus each declared column's slot, offset and
/// validity. Variable payload is charged separately from `Record::heap_bytes`.
pub(crate) fn fixed_width<R: Record>() -> usize {
    R::fields().iter().fold(16, |bytes, field| {
        let slot = match field.scalar() {
            Scalar::Text | Scalar::Binary | Scalar::Int32 => 4,
            Scalar::Bool => 1,
            Scalar::Int16 => 2,
            Scalar::Int64 => 8,
            Scalar::Id => 16,
            Scalar::Digest => 32,
        };
        bytes + slot + if field.list() { 4 } else { 0 } + usize::from(field.nullable())
    })
}
/// Admission size of held typed rows, including unused vector capacity.
pub(crate) fn rows_bytes<R: Record>(rows: &[R], capacity: usize) -> Result<usize, ModelError> {
    let mut bytes = capacity.checked_mul(size_of::<R>());
    for row in rows {
        bytes = bytes.and_then(|held| held.checked_add(row.heap_bytes()));
    }
    bytes.ok_or_else(|| ModelError::Invalid(format!("{} batch size overflow", R::NAME)))
}

/// A model-checked, identity-checked typed boundary. Equal keys with different payload refuse.
/// The batch owns one reservation covering its typed rows and their Arrow encoding for its whole
/// lifetime; dropping the batch returns it to the attempt budget.
#[derive(Debug)]
pub struct Batch<R: Record> {
    rows: Vec<R>,
    arrow: RecordBatch,
    reservation: Box<dyn Reservation>,
}
impl<R: Record> Batch<R> {
    pub fn new(
        model: &ValidatedModel,
        rows: Vec<R>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let reservation = budget.reserve(R::NAME, rows_bytes(&rows, rows.capacity())?)?;
        Self::with_reservation(model, rows, reservation)
    }
    /// `reservation` already covers `rows`; it grows before encoding and settles on the encoded size.
    pub(crate) fn with_reservation(
        model: &ValidatedModel,
        mut rows: Vec<R>,
        mut reservation: Box<dyn Reservation>,
    ) -> Result<Self, ModelError> {
        model.require::<R>()?;
        for row in &rows {
            row.validate()?;
        }
        rows.sort_unstable_by_key(Record::id);
        let mut conflict = false;
        rows.dedup_by(|a, b| {
            if a.id() != b.id() {
                return false;
            }
            conflict |= a != b;
            true
        });
        if conflict {
            return Err(ModelError::Conflict(R::NAME));
        }
        let held = rows_bytes(&rows, rows.capacity())?;
        let overflow = || ModelError::Invalid(format!("{} encoding size overflow", R::NAME));
        let variable = rows
            .iter()
            .try_fold(0usize, |bytes, row| bytes.checked_add(row.heap_bytes()))
            .ok_or_else(overflow)?;
        // Growable Arrow buffers may double while encoding; admit that before allocating.
        let encoding = rows
            .len()
            .checked_mul(fixed_width::<R>())
            .and_then(|fixed| fixed.checked_add(variable.checked_mul(2)?))
            .ok_or_else(overflow)?;
        reservation.try_resize(
            held.checked_add(encoding)
                .ok_or_else(overflow)?
                .max(reservation.size()),
        )?;
        let arrow = R::encode(&rows)?;
        reservation.try_resize(
            held.checked_add(arrow.get_array_memory_size())
                .ok_or_else(overflow)?,
        )?;
        Ok(Self {
            rows,
            arrow,
            reservation,
        })
    }
    pub fn read(
        model: &ValidatedModel,
        arrow: &RecordBatch,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        model.require::<R>()?;
        let encoded = arrow.get_array_memory_size();
        // Decoded rows hold at most their inline size plus the encoded variable payload.
        let decoded = arrow
            .num_rows()
            .checked_mul(size_of::<R>())
            .and_then(|inline| inline.checked_add(encoded))
            .ok_or_else(|| ModelError::Invalid(format!("{} decoding size overflow", R::NAME)))?;
        let mut reservation = budget.reserve(R::NAME, encoded.saturating_add(decoded))?;
        let rows = R::decode(arrow)?;
        let held = rows_bytes(&rows, rows.capacity())?;
        if rows.windows(2).all(|pair| pair[0].id() < pair[1].id()) {
            // A canonical stored batch already has the correct physical representation.
            // Share its Arrow buffers instead of constructing another encoded copy.
            reservation.try_resize(held.saturating_add(encoded))?;
            Ok(Self {
                rows,
                arrow: arrow.clone(),
                reservation,
            })
        } else {
            reservation.try_resize(held)?;
            Self::with_reservation(model, rows, reservation)
        }
    }
    pub fn rows(&self) -> &[R] {
        &self.rows
    }
    pub fn arrow(&self) -> &RecordBatch {
        &self.arrow
    }
    /// Bytes this batch currently holds against its attempt budget.
    pub fn reserved(&self) -> usize {
        self.reservation.size()
    }
}
