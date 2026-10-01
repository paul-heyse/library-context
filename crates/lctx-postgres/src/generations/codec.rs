//! SQLx rows to the exact declared Arrow schema. Type dispatch lives only at this effect boundary.
use super::Error;
use arrow_array::{
    ArrayRef, BooleanArray, FixedSizeBinaryArray, Float64Array, Int16Array, Int32Array, Int64Array,
    RecordBatch, StringArray,
};
use lctx_model::domain::Relation;
use sqlx::{Row, postgres::PgRow};
use std::sync::Arc;

pub(super) fn decode(relation: &Relation, rows: &[PgRow]) -> Result<RecordBatch, Error> {
    let mut columns: Vec<ArrayRef> = Vec::new();
    for field in relation.schema().fields() {
        let name = field.name().as_str();
        macro_rules! scalar {
            ($t:ty, $a:ty) => {
                Arc::new(<$a>::from(
                    rows.iter()
                        .map(|r| r.try_get::<Option<$t>, _>(name))
                        .collect::<Result<Vec<_>, _>>()?,
                )) as ArrayRef
            };
        }
        use arrow_schema::DataType;
        let array = match field.data_type() {
            DataType::Utf8 => scalar!(String, StringArray),
            DataType::Boolean => scalar!(bool, BooleanArray),
            DataType::Int16 => scalar!(i16, Int16Array),
            DataType::Int32 => scalar!(i32, Int32Array),
            DataType::Int64 => scalar!(i64, Int64Array),
            DataType::Float64 => scalar!(f64, Float64Array),
            DataType::Binary => {
                let values = rows
                    .iter()
                    .map(|r| r.try_get::<Option<Vec<u8>>, _>(name))
                    .collect::<Result<Vec<_>, _>>()?;
                Arc::new(arrow_array::BinaryArray::from_iter(
                    values.iter().map(|v| v.as_deref()),
                ))
            }
            DataType::FixedSizeBinary(size) => {
                let values = rows
                    .iter()
                    .map(|r| r.try_get::<Option<Vec<u8>>, _>(name))
                    .collect::<Result<Vec<_>, _>>()?;
                Arc::new(
                    FixedSizeBinaryArray::try_from_sparse_iter_with_size(
                        values.iter().map(|v| v.as_deref()),
                        *size,
                    )
                    .map_err(|e| Error::Codec(e.to_string()))?,
                )
            }
            DataType::List(item) => {
                macro_rules! list {
                    ($t:ty, $builder:ty) => {{
                        let mut builder = arrow_array::builder::ListBuilder::new(<$builder>::new())
                            .with_field(item.clone());
                        for row in rows {
                            let value = row.try_get::<Option<Vec<$t>>, _>(name)?;
                            if let Some(value) = value {
                                for v in value {
                                    builder.values().append_value(v);
                                }
                                builder.append(true);
                            } else {
                                builder.append(false);
                            }
                        }
                        Arc::new(builder.finish()) as ArrayRef
                    }};
                }
                match item.data_type() {
                    DataType::Utf8 => list!(String, arrow_array::builder::StringBuilder),
                    DataType::Boolean => list!(bool, arrow_array::builder::BooleanBuilder),
                    DataType::Int16 => list!(i16, arrow_array::builder::Int16Builder),
                    DataType::Int32 => list!(i32, arrow_array::builder::Int32Builder),
                    DataType::Int64 => list!(i64, arrow_array::builder::Int64Builder),
                    _ => return Err(Error::Codec("unsupported list element".into())),
                }
            }
            _ => return Err(Error::Codec("unsupported physical type".into())),
        };
        columns.push(array);
    }
    RecordBatch::try_new(relation.schema().clone(), columns)
        .map_err(|e| Error::Codec(e.to_string()))
}
