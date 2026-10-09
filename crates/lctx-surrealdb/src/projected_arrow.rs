//! One declared-field conversion from native values into projected Arrow builders.
use arrow_array::{
    RecordBatch,
    builder::{
        ArrayBuilder, BinaryBuilder, BooleanBuilder, FixedSizeBinaryBuilder, Float64Builder,
        Int16Builder, Int32Builder, Int64Builder, ListBuilder, StringBuilder, make_builder,
    },
};
use arrow_schema::{DataType, Field, SchemaRef};
use lctx_model::domain::{
    ModelError, Relation,
    resources::{Reservation, ResourceBudget},
};
use std::{
    borrow::Cow,
    mem::{size_of, size_of_val},
};
use surrealdb::types::{Number, Value};

pub(crate) struct ProjectedBuilder {
    relation: Relation,
    schema: SchemaRef,
    builders: Vec<Box<dyn ArrayBuilder>>,
    rows: usize,
    reservation: Box<dyn Reservation>,
}
impl ProjectedBuilder {
    pub(crate) fn new(
        relation: Relation,
        schema: SchemaRef,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        // Admit empty builder objects, offset slots and make_builder's initial variable
        // data buffers before constructing them, including nested list children.
        let metadata = schema.fields().iter().try_fold(
            schema
                .fields()
                .len()
                .checked_mul(size_of::<Box<dyn ArrayBuilder>>())
                .ok_or_else(allocation_overflow)?,
            |bytes, field| {
                bytes
                    .checked_add(empty_builder_allowance(field.data_type())?)
                    .ok_or_else(allocation_overflow)
            },
        )?;
        let reservation = budget.reserve("native-projected-arrow", metadata)?;
        let builders = schema
            .fields()
            .iter()
            .map(|field| make_builder(field.data_type(), 0))
            .collect();
        let mut builder = Self {
            relation,
            schema,
            builders,
            rows: 0,
            reservation,
        };
        builder.release()?; // Reconcile actual constructor capacities while ownership is held.
        Ok(builder)
    }
    pub(crate) fn rows(&self) -> usize {
        self.rows
    }
    pub(crate) fn bytes(&self) -> usize {
        self.reservation.size()
    }
    pub(crate) fn push(&mut self, value: Value) -> Result<(), ModelError> {
        let Value::Object(row) = value else {
            return Err(ModelError::Schema("native projected object"));
        };
        let bytes = row.values().try_fold(0usize, |bytes, value| {
            bytes
                .checked_add(value_bytes(value)?)
                .ok_or(ModelError::Schema("native projected allocation overflow"))
        })?;
        let arrays = if self.rows == 0 {
            self.array_metadata()?
        } else {
            0
        };
        // Reserve native transfer, Arrow variable growth and the forthcoming array objects
        // before copying. Finished transfer ownership is measured separately below.
        let additional = bytes
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(self.schema.fields().len().checked_mul(64)?))
            .and_then(|bytes| bytes.checked_add(arrays))
            .ok_or_else(allocation_overflow)?;
        self.reservation.try_resize(
            self.reservation
                .size()
                .checked_add(additional)
                .ok_or_else(allocation_overflow)?,
        )?;
        for (field, builder) in self.schema.fields().iter().zip(&mut self.builders) {
            let value = row
                .get(field.name())
                .or_else(|| {
                    (field.name() == "id")
                        .then(|| row.get("semantic_key"))
                        .flatten()
                })
                .unwrap_or(&Value::None);
            let textual = self
                .relation
                .fields()
                .iter()
                .find(|descriptor| descriptor.name() == field.name())
                .is_some_and(|field| field.textual());
            append(builder.as_mut(), field, value, textual)?;
        }
        self.rows += 1;
        Ok(())
    }
    pub(crate) fn finish(&mut self) -> Result<RecordBatch, ModelError> {
        // Finish transfers current offsets into Arrow and allocates replacement slots in
        // every byte/list builder. Admit those recursive allocations before any finish runs.
        let mut additional = self
            .schema
            .fields()
            .iter()
            .try_fold(0usize, |bytes, field| {
                bytes
                    .checked_add(reset_offset_allowance(field.data_type())?)
                    .ok_or_else(allocation_overflow)
            })?;
        // Empty transfers have no first push to admit their forthcoming array objects.
        if self.rows == 0 {
            additional = additional
                .checked_add(self.array_metadata()?)
                .ok_or_else(allocation_overflow)?;
        }
        self.reservation.try_resize(
            self.reservation
                .size()
                .checked_add(additional)
                .ok_or_else(allocation_overflow)?,
        )?;
        let arrays: Vec<_> = self
            .builders
            .iter_mut()
            .map(|builder| builder.finish())
            .collect();
        let columns = arrays
            .capacity()
            .checked_mul(size_of::<arrow_array::ArrayRef>())
            .ok_or_else(allocation_overflow)?;
        let batch = RecordBatch::try_new_with_options(
            self.schema.clone(),
            arrays,
            &arrow_array::RecordBatchOptions::new().with_row_count(Some(self.rows)),
        )
        .map_err(ModelError::codec)?;
        self.rows = 0;
        // Native values and builder growth scratch have ended. This stream still owns the
        // yielded Arrow buffers and empty reusable builders until its next poll.
        let retained = batch.columns().iter().try_fold(
            self.retained_builders()?
                .checked_add(columns)
                .ok_or_else(allocation_overflow)?,
            |bytes, array| {
                bytes
                    .checked_add(array.get_array_memory_size())
                    .ok_or_else(allocation_overflow)
            },
        )?;
        self.reservation.try_resize(retained)?;
        Ok(batch)
    }
    fn array_metadata(&self) -> Result<usize, ModelError> {
        self.schema.fields().iter().try_fold(
            self.schema
                .fields()
                .len()
                .checked_mul(size_of::<arrow_array::ArrayRef>())
                .ok_or_else(allocation_overflow)?,
            |bytes, field| {
                bytes
                    .checked_add(array_metadata_bytes(field.data_type())?)
                    .ok_or_else(allocation_overflow)
            },
        )
    }
    fn retained_builders(&self) -> Result<usize, ModelError> {
        self.builders.iter().zip(self.schema.fields()).try_fold(
            self.builders
                .capacity()
                .checked_mul(size_of::<Box<dyn ArrayBuilder>>())
                .ok_or_else(allocation_overflow)?,
            |bytes, (builder, field)| {
                bytes
                    .checked_add(empty_builder_bytes(builder.as_ref(), field.data_type())?)
                    .ok_or_else(allocation_overflow)
            },
        )
    }
    pub(crate) fn release(&mut self) -> Result<(), ModelError> {
        let retained = self.retained_builders()?;
        self.reservation.try_resize(retained)
    }
}
fn allocation_overflow() -> ModelError {
    ModelError::Schema("native projected allocation overflow")
}
// Arrow 59.3 byte/list finish resets its offsets Vec with mem::take then push(0).
// The replacement Vec grows to four i32 slots; its old buffer remains in yielded Arrow.
fn reset_offset_allowance(data_type: &DataType) -> Result<usize, ModelError> {
    Ok(match data_type {
        DataType::Binary | DataType::Utf8 => 4 * size_of::<i32>(),
        DataType::List(child) => (4 * size_of::<i32>())
            .checked_add(reset_offset_allowance(child.data_type())?)
            .ok_or_else(allocation_overflow)?,
        _ => 0,
    })
}
fn array_metadata_bytes(data_type: &DataType) -> Result<usize, ModelError> {
    Ok(match data_type {
        DataType::Boolean => size_of::<arrow_array::BooleanArray>(),
        DataType::Int16 => size_of::<arrow_array::Int16Array>(),
        DataType::Int32 => size_of::<arrow_array::Int32Array>(),
        DataType::Int64 => size_of::<arrow_array::Int64Array>(),
        DataType::Float64 => size_of::<arrow_array::Float64Array>(),
        DataType::Utf8 => size_of::<arrow_array::StringArray>(),
        DataType::Binary => size_of::<arrow_array::BinaryArray>(),
        DataType::FixedSizeBinary(_) => size_of::<arrow_array::FixedSizeBinaryArray>(),
        DataType::List(child) => size_of::<arrow_array::ListArray>()
            .checked_add(array_metadata_bytes(child.data_type())?)
            .ok_or_else(allocation_overflow)?,
        _ => return Err(ModelError::Schema("undeclared native Arrow field type")),
    })
}
// The supported builders transfer data/null buffers on finish. Byte and list builders
// additionally recreate a retained offset slot; include its actual allocated capacity.
fn empty_builder_allowance(data_type: &DataType) -> Result<usize, ModelError> {
    Ok(match data_type {
        DataType::Boolean => size_of::<BooleanBuilder>(),
        DataType::Int16 => size_of::<Int16Builder>(),
        DataType::Int32 => size_of::<Int32Builder>(),
        DataType::Int64 => size_of::<Int64Builder>(),
        DataType::Float64 => size_of::<Float64Builder>(),
        // Arrow 59.3 make_builder(dt, 0) still allocates 1024 data bytes for these types.
        DataType::Utf8 => size_of::<StringBuilder>() + 4 * size_of::<i32>() + 1024,
        DataType::Binary => size_of::<BinaryBuilder>() + 4 * size_of::<i32>() + 1024,
        DataType::FixedSizeBinary(_) => size_of::<FixedSizeBinaryBuilder>(),
        DataType::List(child) => (size_of::<ListBuilder<Box<dyn ArrayBuilder>>>()
            + 4 * size_of::<i32>())
        .checked_add(empty_builder_allowance(child.data_type())?)
        .ok_or_else(allocation_overflow)?,
        _ => return Err(ModelError::Schema("undeclared native Arrow field type")),
    })
}
fn empty_builder_bytes(
    builder: &dyn ArrayBuilder,
    data_type: &DataType,
) -> Result<usize, ModelError> {
    let mut bytes = size_of_val(builder);
    macro_rules! primitive {
        ($ty:ty,$width:ty) => {{
            let builder = builder
                .as_any()
                .downcast_ref::<$ty>()
                .ok_or(ModelError::Schema("native Arrow builder"))?;
            bytes = bytes
                .checked_add(
                    builder
                        .capacity()
                        .checked_mul(size_of::<$width>())
                        .ok_or_else(allocation_overflow)?,
                )
                .and_then(|bytes| bytes.checked_add(builder.validity_capacity()))
                .ok_or_else(allocation_overflow)?;
        }};
    }
    macro_rules! variable {
        ($ty:ty) => {{
            let builder = builder
                .as_any()
                .downcast_ref::<$ty>()
                .ok_or(ModelError::Schema("native Arrow builder"))?;
            bytes = bytes
                .checked_add(builder.values_capacity())
                .and_then(|bytes| {
                    bytes.checked_add(builder.offsets_capacity().checked_mul(size_of::<i32>())?)
                })
                .and_then(|bytes| bytes.checked_add(builder.validity_capacity()))
                .ok_or_else(allocation_overflow)?;
        }};
    }
    match data_type {
        DataType::Boolean => {
            let builder = builder
                .as_any()
                .downcast_ref::<BooleanBuilder>()
                .ok_or(ModelError::Schema("native Arrow builder"))?;
            bytes = bytes
                .checked_add(builder.capacity().div_ceil(8))
                .ok_or_else(allocation_overflow)?;
        }
        DataType::Int16 => primitive!(Int16Builder, i16),
        DataType::Int32 => primitive!(Int32Builder, i32),
        DataType::Int64 => primitive!(Int64Builder, i64),
        DataType::Float64 => primitive!(Float64Builder, f64),
        DataType::Utf8 => variable!(StringBuilder),
        DataType::Binary => variable!(BinaryBuilder),
        // Fixed-size binary transfers its whole value vector and null buffer on finish.
        DataType::FixedSizeBinary(_) => {}
        DataType::List(child) => {
            let builder = builder
                .as_any()
                .downcast_ref::<ListBuilder<Box<dyn ArrayBuilder>>>()
                .ok_or(ModelError::Schema("native list builder"))?;
            let child_bytes =
                empty_builder_bytes(builder.values_ref().as_ref(), child.data_type())?;
            bytes = bytes
                .checked_add(
                    builder
                        .offsets_capacity()
                        .checked_mul(size_of::<i32>())
                        .ok_or_else(allocation_overflow)?,
                )
                .and_then(|bytes| bytes.checked_add(builder.validity_capacity()))
                .and_then(|bytes| bytes.checked_add(child_bytes))
                .ok_or_else(allocation_overflow)?;
        }
        _ => return Err(ModelError::Schema("undeclared native Arrow field type")),
    }
    Ok(bytes)
}
fn value_bytes(value: &Value) -> Result<usize, ModelError> {
    Ok(match value {
        Value::String(value) => value.len(),
        Value::Bytes(value) => value.len(),
        Value::Array(values) => values.iter().try_fold(32usize, |bytes, value| {
            bytes
                .checked_add(value_bytes(value)?)
                .ok_or(ModelError::Schema("native list allocation overflow"))
        })?,
        Value::Object(_) => return Err(ModelError::Schema("unexpected nested native object")),
        _ => 16,
    })
}
fn integer(value: &Value) -> Result<i64, ModelError> {
    match value {
        Value::Number(Number::Int(value)) => Ok(*value),
        _ => Err(ModelError::Schema("native declared integer")),
    }
}
fn bytes(value: &Value, textual: bool, fixed: bool) -> Result<Cow<'_, [u8]>, ModelError> {
    match value {
        Value::Bytes(value) => Ok(Cow::Borrowed(value.as_ref())),
        Value::String(value) if textual => Ok(Cow::Borrowed(value.as_bytes())),
        Value::String(value) if fixed => hex::decode(value)
            .map(Cow::Owned)
            .map_err(ModelError::codec),
        Value::Array(values) if fixed => values
            .iter()
            .map(|value| u8::try_from(integer(value)?).map_err(ModelError::codec))
            .collect::<Result<Vec<_>, _>>()
            .map(Cow::Owned),
        _ => Err(ModelError::Schema("native declared bytes")),
    }
}
fn append(
    builder: &mut dyn ArrayBuilder,
    field: &Field,
    value: &Value,
    textual: bool,
) -> Result<(), ModelError> {
    let null = matches!(value, Value::None | Value::Null);
    if null && !field.is_nullable() {
        return Err(ModelError::Schema("null native required field"));
    }
    macro_rules! scalar {
        ($ty:ty,$convert:expr) => {{
            let builder = builder
                .as_any_mut()
                .downcast_mut::<$ty>()
                .ok_or(ModelError::Schema("native Arrow builder"))?;
            if null {
                builder.append_null();
            } else {
                builder.append_value($convert);
            }
        }};
    }
    match field.data_type() {
        DataType::Boolean => scalar!(
            BooleanBuilder,
            match value {
                Value::Bool(value) => *value,
                _ => return Err(ModelError::Schema("native declared bool")),
            }
        ),
        DataType::Int16 => scalar!(
            Int16Builder,
            i16::try_from(integer(value)?).map_err(ModelError::codec)?
        ),
        DataType::Int32 => scalar!(
            Int32Builder,
            i32::try_from(integer(value)?).map_err(ModelError::codec)?
        ),
        DataType::Int64 => scalar!(Int64Builder, integer(value)?),
        DataType::Float64 => scalar!(
            Float64Builder,
            match value {
                Value::Number(Number::Float(value)) if value.is_finite() => *value,
                Value::Number(Number::Int(value))
                    if (*value as f64) as i128 == i128::from(*value) =>
                    *value as f64,
                _ => return Err(ModelError::Schema("native declared finite float")),
            }
        ),
        DataType::Utf8 => scalar!(
            StringBuilder,
            match value {
                Value::String(value) => value.as_str(),
                _ => return Err(ModelError::Schema("native declared text")),
            }
        ),
        DataType::Binary => scalar!(BinaryBuilder, bytes(value, textual, false)?),
        DataType::FixedSizeBinary(width) => {
            let builder = builder
                .as_any_mut()
                .downcast_mut::<FixedSizeBinaryBuilder>()
                .ok_or(ModelError::Schema("native fixed builder"))?;
            if null {
                builder.append_null();
            } else {
                let bytes = bytes(value, false, true)?;
                if bytes.len() != usize::try_from(*width).map_err(ModelError::codec)? {
                    return Err(ModelError::Schema("native nominal byte width"));
                }
                builder.append_value(bytes).map_err(ModelError::codec)?;
            }
        }
        DataType::List(child) => {
            let builder = builder
                .as_any_mut()
                .downcast_mut::<ListBuilder<Box<dyn ArrayBuilder>>>()
                .ok_or(ModelError::Schema("native list builder"))?;
            if null {
                builder.append(false);
            } else {
                let Value::Array(values) = value else {
                    return Err(ModelError::Schema("native declared list"));
                };
                for value in values.iter() {
                    append(builder.values().as_mut(), child, value, textual)?;
                }
                builder.append(true);
            }
        }
        _ => return Err(ModelError::Schema("undeclared native Arrow field type")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Array, ListArray};
    #[test]
    fn opaque_bytes_nominal_width_and_full_vectors_preserve_declared_values() {
        let mut opaque = BinaryBuilder::new();
        append(
            &mut opaque,
            &Field::new("body", DataType::Binary, false),
            &Value::Bytes(vec![0, 255, 128].into()),
            false,
        )
        .unwrap();
        let array = opaque.finish();
        assert_eq!(array.value(0), [0, 255, 128]);
        let mut nominal = FixedSizeBinaryBuilder::new(16);
        let values = (0..16)
            .map(|n| Value::Number(Number::Int(n)))
            .collect::<Vec<_>>();
        append(
            &mut nominal,
            &Field::new("id", DataType::FixedSizeBinary(16), false),
            &Value::Array(values.into()),
            false,
        )
        .unwrap();
        let array = nominal.finish();
        assert_eq!(array.value(0), (0u8..16).collect::<Vec<_>>());
        let field = Field::new(
            "values",
            DataType::List(Arc::new(Field::new("item", DataType::Float64, false))),
            false,
        );
        let mut vector = make_builder(field.data_type(), 0);
        let values = (0..4096)
            .map(|i| Value::Number(Number::Float(f64::from(i) / 4096.0)))
            .collect::<Vec<_>>();
        append(vector.as_mut(), &field, &Value::Array(values.into()), false).unwrap();
        let array = vector.finish();
        let array = array.as_any().downcast_ref::<ListArray>().unwrap();
        assert_eq!(array.value(0).len(), 4096);
    }
    #[test]
    fn initial_variable_capacity_is_admitted_before_flat_and_nested_construction() {
        let relation = Relation::of::<lctx_model::domain::artifact::ArtifactChunk>();
        let nested = |data_type| DataType::List(Arc::new(Field::new("item", data_type, true)));
        fn assert_initial_capacities(builder: &dyn ArrayBuilder) {
            if let Some(binary) = builder.as_any().downcast_ref::<BinaryBuilder>() {
                assert_eq!(binary.values_capacity(), 1024);
                assert_eq!(binary.offsets_capacity(), 1);
            } else if let Some(text) = builder.as_any().downcast_ref::<StringBuilder>() {
                assert_eq!(text.values_capacity(), 1024);
                assert_eq!(text.offsets_capacity(), 1);
            } else {
                let list = builder
                    .as_any()
                    .downcast_ref::<ListBuilder<Box<dyn ArrayBuilder>>>()
                    .unwrap();
                assert_eq!(list.offsets_capacity(), 1);
                assert_initial_capacities(list.values_ref().as_ref());
            }
        }
        // Exercise the actual Arrow constructor shapes independently of row conversion.
        for data_type in [
            DataType::Binary,
            DataType::Utf8,
            nested(DataType::Binary),
            nested(nested(DataType::Utf8)),
        ] {
            let schema = Arc::new(arrow_schema::Schema::new(vec![Field::new(
                "control",
                data_type.clone(),
                true,
            )]));
            let refused = ResourceBudget::fixed(1023).unwrap();
            assert!(
                matches!(
                    ProjectedBuilder::new(relation.clone(), schema.clone(), &refused),
                    Err(ModelError::Resource { .. })
                ),
                "one initial 1024-byte child buffer cannot fit"
            );
            assert_eq!(refused.reserved(), 0);
            let budget = ResourceBudget::fixed(4096).unwrap();
            let mut builder = ProjectedBuilder::new(relation.clone(), schema, &budget).unwrap();
            assert_initial_capacities(builder.builders[0].as_ref());
            assert_eq!(budget.reserved(), builder.retained_builders().unwrap());
            assert!(
                budget.reserved() >= 1024,
                "zero rows still own the initial variable buffer"
            );
            assert!(
                budget.reserved() < 2048,
                "constructor reconciles bounded metadata and child capacity"
            );
            let initial = budget.reserved();
            let without_replacement_offsets = initial + builder.array_metadata().unwrap();
            let batch = builder.finish().unwrap();
            assert!(
                budget.reserved()
                    >= batch.get_array_memory_size() + builder.retained_builders().unwrap()
            );
            drop(batch);
            builder.release().unwrap();
            assert_eq!(budget.reserved(), builder.retained_builders().unwrap());
            drop(builder);
            assert_eq!(budget.reserved(), 0);
            let refused = ResourceBudget::fixed(without_replacement_offsets).unwrap();
            let schema = Arc::new(arrow_schema::Schema::new(vec![Field::new(
                "control",
                data_type.clone(),
                true,
            )]));
            let mut builder = ProjectedBuilder::new(relation.clone(), schema, &refused).unwrap();
            assert!(
                matches!(builder.finish(), Err(ModelError::Resource { .. })),
                "replacement offsets are admitted before array/builder allocation"
            );
            assert_initial_capacities(builder.builders[0].as_ref());
            assert_eq!(refused.reserved(), initial);
            drop(builder);
            assert_eq!(refused.reserved(), 0);
        }
    }
    #[test]
    fn projected_transfer_releases_scratch_but_keeps_arrow_and_builder_charge() {
        use lctx_model::domain::artifact::ArtifactChunk;
        let opaque = Value::Bytes(vec![255; 1 << 20].into());
        assert!(matches!(
            bytes(&opaque, false, false).unwrap(),
            Cow::Borrowed(_)
        ));
        let text = Value::String("exact λ".into());
        assert!(matches!(
            bytes(&text, true, false).unwrap(),
            Cow::Borrowed(_)
        ));
        let relation = Relation::of::<ArtifactChunk>();
        let index = relation.schema().index_of("body").unwrap();
        let schema = Arc::new(relation.schema().project(&[index]).unwrap());
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let mut builder = ProjectedBuilder::new(relation, schema, &budget).unwrap();
        let mut object = surrealdb::types::Object::new();
        object.insert("body", opaque);
        builder.push(Value::Object(object)).unwrap();
        assert!(
            builder.bytes() > 2 << 20,
            "native transfer and Arrow growth are admitted before copying"
        );
        let batch = builder.finish().unwrap();
        assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::BinaryArray>()
                .unwrap()
                .value(0)
                .len(),
            1 << 20
        );
        assert!(builder.bytes() >= batch.get_array_memory_size());
        assert!(
            builder.bytes() < (1 << 20) + 4096,
            "finished transfer does not retain native scratch allowance"
        );
        drop(batch);
        builder.release().unwrap();
        assert!(
            budget.reserved() > 0 && budget.reserved() < 4096,
            "empty reusable builder retains its metadata charge"
        );
        let mut object = surrealdb::types::Object::new();
        object.insert("body", Value::Bytes(vec![7; 32].into()));
        builder.push(Value::Object(object)).unwrap();
        let second = builder.finish().unwrap();
        assert_eq!(second.num_rows(), 1);
        drop(second);
        drop(builder);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn malformed_native_values_fail_instead_of_lossy_coercion() {
        let mut nominal = FixedSizeBinaryBuilder::new(16);
        assert!(
            append(
                &mut nominal,
                &Field::new("id", DataType::FixedSizeBinary(16), false),
                &Value::Bytes(vec![1; 15].into()),
                false
            )
            .is_err()
        );
        let mut number = Float64Builder::new();
        let field = Field::new("score", DataType::Float64, false);
        assert!(
            append(
                &mut number,
                &field,
                &Value::Number(Number::Float(f64::NAN)),
                false
            )
            .is_err()
        );
        assert!(
            append(
                &mut number,
                &field,
                &Value::Number(Number::Int(9_007_199_254_740_993)),
                false
            )
            .is_err()
        );
        assert!(append(&mut number, &field, &Value::Null, false).is_err());
        let mut opaque = BinaryBuilder::new();
        assert!(
            append(
                &mut opaque,
                &Field::new("body", DataType::Binary, false),
                &Value::String("unowned conversion".into()),
                false
            )
            .is_err()
        );
    }
    #[test]
    fn null_list_and_zero_column_projection_preserve_row_count() {
        let field = Field::new(
            "values",
            DataType::List(Arc::new(Field::new("item", DataType::Int64, true))),
            true,
        );
        let mut list = make_builder(field.data_type(), 0);
        append(list.as_mut(), &field, &Value::Null, false).unwrap();
        append(
            list.as_mut(),
            &field,
            &Value::Array(vec![Value::Null, Value::Number(Number::Int(17))].into()),
            false,
        )
        .unwrap();
        let array = list.finish();
        let array = array.as_any().downcast_ref::<ListArray>().unwrap();
        assert!(array.is_null(0));
        assert_eq!(array.value(1).len(), 2);
        let relation = Relation::of::<lctx_model::domain::input::Package>();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut builder =
            ProjectedBuilder::new(relation, Arc::new(arrow_schema::Schema::empty()), &budget)
                .unwrap();
        builder.push(Value::Object(Default::default())).unwrap();
        builder.push(Value::Object(Default::default())).unwrap();
        assert_eq!(builder.finish().unwrap().num_rows(), 2);
    }
    use std::sync::Arc;
}
