//! Rust value types → Arrow columns (DESIGN §3.3 physical profiles).
//!
//! A table's row struct is its single declaration; each field type maps to one Arrow type,
//! nullability and metadata here, and builds its own array. No table writes builder code.
//!
//! The same mapping reads a value back ([`ArrowColumn::read`]), for typed query results
//! (`query_row!`). A null in a column whose type is not an `Option` is an error, never a default
//! (the holistic assessment's A8).

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::builder::{Float32Builder, Float64Builder, ListBuilder, StringBuilder};
use arrow_array::cast::AsArray;
use arrow_array::types::{Float32Type, Float64Type, Int16Type, Int64Type};
use arrow_array::{
    Array, ArrayRef, BooleanArray, FixedSizeBinaryArray, Float64Array, Int16Array, Int64Array,
    StringArray,
};
use arrow_schema::{ArrowError, DataType, Field};

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
    /// Row `i` of `array`, which has [`Self::data_type`] (a caller casts first). A null is an
    /// error unless `Self` is an `Option`.
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError>;

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

/// Row `i`, or an error for a null in a column that admits none.
fn present(array: &dyn Array, i: usize) -> Result<(), ArrowError> {
    if array.is_null(i) {
        Err(ArrowError::InvalidArgumentError(format!(
            "a null at row {i} of a column that admits none"
        )))
    } else {
        Ok(())
    }
}

/// Row `i` of an optional column: `None` for a null, else the non-null reading.
fn optional<T: ArrowColumn>(array: &dyn Array, i: usize) -> Result<Option<T>, ArrowError> {
    if array.is_null(i) {
        Ok(None)
    } else {
        T::read(array, i).map(Some)
    }
}

fn bytes<const N: usize>(array: &dyn Array, i: usize) -> Result<[u8; N], ArrowError> {
    present(array, i)?;
    let a = array.as_fixed_size_binary_opt().ok_or_else(|| {
        ArrowError::CastError(format!(
            "expected FixedSizeBinary({N}), got {}",
            array.data_type()
        ))
    })?;
    <[u8; N]>::try_from(a.value(i))
        .map_err(|_| ArrowError::InvalidArgumentError(format!("expected {N} bytes at row {i}")))
}

fn typed<'a, T: 'static>(
    array: &'a dyn Array,
    cast: impl FnOnce(&'a dyn Array) -> Option<&'a T>,
    expected: &str,
) -> Result<&'a T, ArrowError> {
    cast(array).ok_or_else(|| {
        ArrowError::CastError(format!("expected {expected}, got {}", array.data_type()))
    })
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        bytes::<16>(array, i).map(Id)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        bytes::<32>(array, i).map(Digest)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        Ok(typed(array, |a| a.as_string_opt::<i32>(), "Utf8")?
            .value(i)
            .to_owned())
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        Ok(typed(array, |a| a.as_primitive_opt::<Int64Type>(), "Int64")?.value(i))
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        Ok(typed(array, |a| a.as_boolean_opt(), "Boolean")?.value(i))
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    /// A code outside the codebook is an error, as the generated `codebook:` rules make it.
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        let code = typed(array, |a| a.as_primitive_opt::<Int16Type>(), "Int16")?.value(i);
        C::from_code(code).ok_or_else(|| {
            ArrowError::InvalidArgumentError(format!("{code} is not in codebook {}", C::NAME))
        })
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
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
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        let list = typed(array, |a| a.as_list_opt::<i32>(), "List<Utf8>")?.value(i);
        let values = typed(list.as_ref(), |a| a.as_string_opt::<i32>(), "Utf8 items")?;
        (0..values.len()).map(|j| String::read(values, j)).collect()
    }
}

/// A score or weight (DESIGN §3.3): `Float64`, finite. Finiteness is checked where the value is
/// made (the analytics refuse a non-finite result) and by a generated `finite:` rule.
impl ArrowColumn for f64 {
    fn data_type() -> DataType {
        DataType::Float64
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(Float64Array::from_iter_values(values.copied()))
    }
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        Ok(typed(array, |a| a.as_primitive_opt::<Float64Type>(), "Float64")?.value(i))
    }
}

impl ArrowColumn for Option<f64> {
    fn data_type() -> DataType {
        DataType::Float64
    }
    fn nullable() -> bool {
        true
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        Arc::new(values.copied().collect::<Float64Array>())
    }
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        optional(array, i)
    }
}

fn float_item() -> Arc<Field> {
    Arc::new(Field::new("item", DataType::Float64, false))
}

/// A sequence of scores in order (a quality history).
impl ArrowColumn for Vec<f64> {
    fn data_type() -> DataType {
        DataType::List(float_item())
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        let mut b = ListBuilder::new(Float64Builder::new()).with_field(float_item());
        for list in values {
            for v in list {
                b.values().append_value(*v);
            }
            b.append(true);
        }
        Arc::new(b.finish())
    }
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        let list = typed(array, |a| a.as_list_opt::<i32>(), "List<Float64>")?.value(i);
        let values = typed(
            list.as_ref(),
            |a| a.as_primitive_opt::<Float64Type>(),
            "Float64 items",
        )?;
        (0..values.len()).map(|j| f64::read(values, j)).collect()
    }
}

fn float32_item() -> Arc<Field> {
    Arc::new(Field::new("item", DataType::Float32, false))
}

/// An embedding vector (DESIGN §3.3): `List<Float32>` in `embedding_cache` (the child is renamed
/// `element` by Delta); its length, finiteness and norm are checked where it is made and read.
impl ArrowColumn for Vec<f32> {
    fn data_type() -> DataType {
        DataType::List(float32_item())
    }
    fn nullable() -> bool {
        false
    }
    fn array<'a>(values: impl ExactSizeIterator<Item = &'a Self>) -> ArrayRef {
        let mut b = ListBuilder::new(Float32Builder::new()).with_field(float32_item());
        for list in values {
            b.values().append_slice(list);
            b.append(true);
        }
        Arc::new(b.finish())
    }
    fn read(array: &dyn Array, i: usize) -> Result<Self, ArrowError> {
        present(array, i)?;
        let list = typed(array, |a| a.as_list_opt::<i32>(), "List<Float32>")?.value(i);
        let values = typed(
            list.as_ref(),
            |a| a.as_primitive_opt::<Float32Type>(),
            "Float32 items",
        )?;
        if values.null_count() > 0 {
            return Err(ArrowError::InvalidArgumentError(format!(
                "a null element in the vector at row {i}"
            )));
        }
        Ok(values.values().to_vec())
    }
}
