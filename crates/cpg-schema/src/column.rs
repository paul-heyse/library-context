//! Rust value types → Arrow columns (DESIGN §3.3 physical profiles).
//!
//! A table's row struct is its single declaration; each field type maps to one Arrow type,
//! nullability and metadata here, and builds its own array. No table writes builder code.

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_array::{
    ArrayRef, BooleanArray, FixedSizeBinaryArray, Int16Array, Int64Array, StringArray,
};
use arrow_schema::{DataType, Field};

use crate::codebook::Codebook;
use crate::id::{Digest, Id};

/// Field metadata key naming a column's codebook.
pub const CODEBOOK_KEY: &str = "lctx.codebook";

pub trait ArrowColumn: Sized {
    fn data_type() -> DataType;
    fn nullable() -> bool;
    fn metadata() -> HashMap<String, String> {
        HashMap::new()
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef
    where
        Self: 'a;

    fn field(name: &str) -> Field {
        Field::new(name, Self::data_type(), Self::nullable()).with_metadata(Self::metadata())
    }
}

fn fixed<'a, const N: usize>(
    values: impl ExactSizeIterator<Item = Option<&'a [u8; N]>>,
) -> ArrayRef {
    Arc::new(
        FixedSizeBinaryArray::try_from_sparse_iter_with_size(values, N as i32)
            .expect("every value is N bytes"),
    )
}

impl ArrowColumn for Id {
    fn data_type() -> DataType {
        DataType::FixedSizeBinary(16)
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        fixed(values.map(|v| Some(&v.0)))
    }
}

impl ArrowColumn for Option<Id> {
    fn data_type() -> DataType {
        DataType::FixedSizeBinary(16)
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        fixed(values.map(|v| v.as_ref().map(|i| &i.0)))
    }
}

impl ArrowColumn for Digest {
    fn data_type() -> DataType {
        DataType::FixedSizeBinary(32)
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        fixed(values.map(|v| Some(&v.0)))
    }
}

impl ArrowColumn for Option<Digest> {
    fn data_type() -> DataType {
        DataType::FixedSizeBinary(32)
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        fixed(values.map(|v| v.as_ref().map(|d| &d.0)))
    }
}

impl ArrowColumn for String {
    fn data_type() -> DataType {
        DataType::Utf8
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(StringArray::from_iter_values(values))
    }
}

impl ArrowColumn for Option<String> {
    fn data_type() -> DataType {
        DataType::Utf8
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(values.map(|v| v.as_deref()).collect::<StringArray>())
    }
}

impl ArrowColumn for i64 {
    fn data_type() -> DataType {
        DataType::Int64
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(Int64Array::from_iter_values(values.copied()))
    }
}

impl ArrowColumn for Option<i64> {
    fn data_type() -> DataType {
        DataType::Int64
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(values.copied().collect::<Int64Array>())
    }
}

impl ArrowColumn for bool {
    fn data_type() -> DataType {
        DataType::Boolean
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(values.map(|v| Some(*v)).collect::<BooleanArray>())
    }
}

impl ArrowColumn for Option<bool> {
    fn data_type() -> DataType {
        DataType::Boolean
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(values.copied().collect::<BooleanArray>())
    }
}

impl<C: Codebook> ArrowColumn for C {
    fn data_type() -> DataType {
        DataType::Int16
    }
    fn nullable() -> bool {
        false
    }
    fn metadata() -> HashMap<String, String> {
        HashMap::from([(CODEBOOK_KEY.to_owned(), C::NAME.to_owned())])
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(Int16Array::from_iter_values(values.map(|v| v.code())))
    }
}

impl<C: Codebook> ArrowColumn for Option<C> {
    fn data_type() -> DataType {
        DataType::Int16
    }
    fn nullable() -> bool {
        true
    }
    fn metadata() -> HashMap<String, String> {
        HashMap::from([(CODEBOOK_KEY.to_owned(), C::NAME.to_owned())])
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(
            values
                .map(|v| v.map(Codebook::code))
                .collect::<Int16Array>(),
        )
    }
}

/// The list element field: named `item` in computation; Delta renames it `element` on read-back,
/// which only the Delta boundary handles (DESIGN §6.3).
fn list_item() -> Arc<Field> {
    Arc::new(Field::new("item", DataType::Utf8, false))
}

impl ArrowColumn for Vec<String> {
    fn data_type() -> DataType {
        DataType::List(list_item())
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        let mut b = ListBuilder::new(StringBuilder::new()).with_field(list_item());
        for list in values {
            for s in list {
                b.values().append_value(s);
            }
            b.append(true);
        }
        Arc::new(b.finish())
    }
}
