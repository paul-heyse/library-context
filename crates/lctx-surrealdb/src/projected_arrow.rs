//! One declared-field conversion from native values into projected Arrow builders.
use arrow_array::{RecordBatch, builder::{ArrayBuilder, BinaryBuilder, BooleanBuilder,
    FixedSizeBinaryBuilder, Float64Builder, Int16Builder, Int32Builder, Int64Builder,
    ListBuilder, StringBuilder, make_builder}};
use arrow_schema::{DataType, Field, SchemaRef};
use lctx_model::domain::{ModelError, Relation, resources::{ResourceBudget, Reservation}};
use surrealdb::types::{Number, Value};

pub(crate) struct ProjectedBuilder {
    relation: Relation,
    schema: SchemaRef,
    builders: Vec<Box<dyn ArrayBuilder>>,
    rows: usize,
    reservation: Box<dyn Reservation>,
}
impl ProjectedBuilder {
    pub(crate) fn new(relation: Relation, schema: SchemaRef, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let reservation = budget.reserve("native-projected-arrow", 0)?;
        let builders = schema.fields().iter().map(|field| make_builder(field.data_type(), 0)).collect();
        Ok(Self { relation, schema, builders, rows: 0, reservation })
    }
    pub(crate) fn rows(&self) -> usize { self.rows }
    pub(crate) fn bytes(&self) -> usize { self.reservation.size() }
    pub(crate) fn push(&mut self, value: Value) -> Result<(), ModelError> {
        let Value::Object(row) = value else { return Err(ModelError::Schema("native projected object")); };
        let bytes = row.values().try_fold(0usize, |bytes, value| bytes.checked_add(value_bytes(value)?).ok_or(ModelError::Schema("native projected allocation overflow")))?;
        // Reserve both native transfer and Arrow variable data before builders grow.
        self.reservation.try_resize(self.reservation.size().checked_add(bytes.saturating_mul(2).saturating_add(self.schema.fields().len()*64)).ok_or(ModelError::Schema("native projected allocation overflow"))?)?;
        for (field, builder) in self.schema.fields().iter().zip(&mut self.builders) {
            let value = row.get(field.name()).or_else(|| (field.name()=="id").then(||row.get("semantic_key")).flatten()).unwrap_or(&Value::None);
            let textual = self.relation.fields().iter().find(|descriptor| descriptor.name()==field.name()).is_some_and(|field|field.textual());
            append(builder.as_mut(), field, value, textual)?;
        }
        self.rows += 1;
        Ok(())
    }
    pub(crate) fn finish(&mut self) -> Result<RecordBatch, ModelError> {
        let arrays = self.builders.iter_mut().map(|builder| builder.finish()).collect();
        let batch = RecordBatch::try_new_with_options(self.schema.clone(), arrays,
            &arrow_array::RecordBatchOptions::new().with_row_count(Some(self.rows))).map_err(ModelError::codec)?;
        self.rows = 0;
        // Kept until the next push/finish: this stream owns the yielded transfer batch.
        Ok(batch)
    }
    pub(crate) fn release(&mut self) -> Result<(), ModelError> { self.reservation.try_resize(0) }
}
fn value_bytes(value: &Value) -> Result<usize, ModelError> {
    Ok(match value {
        Value::String(value) => value.len(), Value::Bytes(value) => value.len(),
        Value::Array(values) => values.iter().try_fold(32usize, |bytes, value| bytes.checked_add(value_bytes(value)?).ok_or(ModelError::Schema("native list allocation overflow")))?,
        Value::Object(_) => return Err(ModelError::Schema("unexpected nested native object")),
        _ => 16,
    })
}
fn integer(value: &Value) -> Result<i64, ModelError> {
    match value { Value::Number(Number::Int(value)) => Ok(*value), _=> Err(ModelError::Schema("native declared integer")) }
}
fn bytes(value: &Value, textual: bool, fixed: bool) -> Result<Vec<u8>, ModelError> {
    match value {
        Value::Bytes(value) => Ok(value.to_vec()),
        Value::String(value) if textual => Ok(value.as_bytes().to_vec()),
        Value::String(value) if fixed => hex::decode(value).map_err(ModelError::codec),
        Value::Array(values) if fixed => values.iter().map(|value|u8::try_from(integer(value)?).map_err(ModelError::codec)).collect(),
        _ => Err(ModelError::Schema("native declared bytes")),
    }
}
fn append(builder: &mut dyn ArrayBuilder, field: &Field, value: &Value, textual: bool) -> Result<(), ModelError> {
    let null = matches!(value, Value::None|Value::Null);
    if null && !field.is_nullable() { return Err(ModelError::Schema("null native required field")); }
    macro_rules! scalar {($ty:ty,$convert:expr)=>{{
        let builder=builder.as_any_mut().downcast_mut::<$ty>().ok_or(ModelError::Schema("native Arrow builder"))?;
        if null {builder.append_null();} else {builder.append_value($convert);}
    }};}
    match field.data_type() {
        DataType::Boolean => scalar!(BooleanBuilder, match value { Value::Bool(value)=>*value, _=>return Err(ModelError::Schema("native declared bool")) }),
        DataType::Int16 => scalar!(Int16Builder, i16::try_from(integer(value)?).map_err(ModelError::codec)?),
        DataType::Int32 => scalar!(Int32Builder, i32::try_from(integer(value)?).map_err(ModelError::codec)?),
        DataType::Int64 => scalar!(Int64Builder, integer(value)?),
        DataType::Float64 => scalar!(Float64Builder, match value {
            Value::Number(Number::Float(value)) if value.is_finite()=>*value,
            Value::Number(Number::Int(value)) if (*value as f64) as i128==i128::from(*value)=>*value as f64,
            _=>return Err(ModelError::Schema("native declared finite float")),
        }),
        DataType::Utf8 => scalar!(StringBuilder, match value { Value::String(value)=>value.as_str(), _=>return Err(ModelError::Schema("native declared text")) }),
        DataType::Binary => scalar!(BinaryBuilder, bytes(value,textual,false)?),
        DataType::FixedSizeBinary(width) => {
            let builder=builder.as_any_mut().downcast_mut::<FixedSizeBinaryBuilder>().ok_or(ModelError::Schema("native fixed builder"))?;
            if null {builder.append_null();} else {
                let bytes=bytes(value,false,true)?;
                if bytes.len()!=usize::try_from(*width).map_err(ModelError::codec)? {return Err(ModelError::Schema("native nominal byte width"));}
                builder.append_value(bytes).map_err(ModelError::codec)?;
            }
        }
        DataType::List(child) => {
            let builder=builder.as_any_mut().downcast_mut::<ListBuilder<Box<dyn ArrayBuilder>>>().ok_or(ModelError::Schema("native list builder"))?;
            if null {builder.append(false);} else {
                let Value::Array(values)=value else{return Err(ModelError::Schema("native declared list"));};
                for value in values.iter(){append(builder.values().as_mut(),child,value,textual)?;}
                builder.append(true);
            }
        }
        _=>return Err(ModelError::Schema("undeclared native Arrow field type")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Array, ListArray};
    #[test]
    fn opaque_bytes_nominal_width_and_full_vectors_preserve_declared_values() {
        let mut opaque=BinaryBuilder::new();
        append(&mut opaque,&Field::new("body",DataType::Binary,false),&Value::Bytes(vec![0,255,128].into()),false).unwrap();
        let array=opaque.finish(); assert_eq!(array.value(0),[0,255,128]);
        let mut nominal=FixedSizeBinaryBuilder::new(16);
        let values=(0..16).map(|n|Value::Number(Number::Int(n))).collect::<Vec<_>>();
        append(&mut nominal,&Field::new("id",DataType::FixedSizeBinary(16),false),&Value::Array(values.into()),false).unwrap();
        let array=nominal.finish(); assert_eq!(array.value(0),(0u8..16).collect::<Vec<_>>());
        let field=Field::new("values",DataType::List(Arc::new(Field::new("item",DataType::Float64,false))),false);
        let mut vector=make_builder(field.data_type(),0);
        let values=(0..4096).map(|i|Value::Number(Number::Float(f64::from(i)/4096.0))).collect::<Vec<_>>();
        append(vector.as_mut(),&field,&Value::Array(values.into()),false).unwrap();
        let array=vector.finish();let array=array.as_any().downcast_ref::<ListArray>().unwrap();
        assert_eq!(array.value(0).len(),4096);
    }
    #[test]
    fn malformed_native_values_fail_instead_of_lossy_coercion() {
        let mut nominal=FixedSizeBinaryBuilder::new(16);
        assert!(append(&mut nominal,&Field::new("id",DataType::FixedSizeBinary(16),false),&Value::Bytes(vec![1;15].into()),false).is_err());
        let mut number=Float64Builder::new();
        let field=Field::new("score",DataType::Float64,false);
        assert!(append(&mut number,&field,&Value::Number(Number::Float(f64::NAN)),false).is_err());
        assert!(append(&mut number,&field,&Value::Number(Number::Int(9_007_199_254_740_993)),false).is_err());
        assert!(append(&mut number,&field,&Value::Null,false).is_err());
        let mut opaque=BinaryBuilder::new();
        assert!(append(&mut opaque,&Field::new("body",DataType::Binary,false),&Value::String("unowned conversion".into()),false).is_err());
    }
    #[test]
    fn null_list_and_zero_column_projection_preserve_row_count() {
        let field=Field::new("values",DataType::List(Arc::new(Field::new("item",DataType::Int64,true))),true);
        let mut list=make_builder(field.data_type(),0);
        append(list.as_mut(),&field,&Value::Null,false).unwrap();
        append(list.as_mut(),&field,&Value::Array(vec![Value::Null,Value::Number(Number::Int(17))].into()),false).unwrap();
        let array=list.finish();let array=array.as_any().downcast_ref::<ListArray>().unwrap();assert!(array.is_null(0));assert_eq!(array.value(1).len(),2);
        let relation=Relation::of::<lctx_model::domain::input::Package>();
        let budget=ResourceBudget::fixed(1<<20).unwrap();
        let mut builder=ProjectedBuilder::new(relation,Arc::new(arrow_schema::Schema::empty()),&budget).unwrap();
        builder.push(Value::Object(Default::default())).unwrap();
        builder.push(Value::Object(Default::default())).unwrap();
        assert_eq!(builder.finish().unwrap().num_rows(),2);
    }
    use std::sync::Arc;
}
