//! Mechanical query fields alongside the unchanged canonical graph payload.
use arrow_array::{
    Array, ArrayRef, BinaryArray, BooleanArray, FixedSizeBinaryArray, Float64Array, Int16Array,
    Int32Array, Int64Array, ListArray, StringArray,
};
use lctx_model::domain::{
    ModelError, Record, Scalar, SemanticReference,
    graph::{Assertion, Entity},
};
use serde::Serialize;
use serde_json::{Map, Value};

pub struct RecordView {
    pub semantic_type: String,
    pub semantic_key: String,
    pub body: Value,
}
macro_rules! entity_view {
    ($($variant:ident:$ty:ty,)*)=>{
        pub fn entity_references(entity:&Entity)->Vec<SemanticReference>{match entity{$(Entity::$variant(row)=>row.references(),)*}}
        pub fn entity_view(entity:&Entity)->Result<RecordView,ModelError>{match entity{$(Entity::$variant(row)=>view(row),)*}}
    }
}
lctx_model::graph_entity_records!(entity_view);
pub use lctx_model::domain::graph::record::{entity_record, assertion_record};
macro_rules! assertion_view {
    ($($variant:ident:$ty:ty,)*)=>{
        pub fn assertion_view(assertion:&Assertion)->Result<RecordView,ModelError>{
            let Some(source)=&assertion.source else {return Ok(RecordView{semantic_type:"__graph_assertion".into(),semantic_key:assertion.id().0.hex(),body:serde_json::json!({"__type":"__graph_assertion"})})};
            match source.domain(){$(<$ty>::NAME=>view(&assertion_record::<$ty>(assertion)?),)*_=>Err(ModelError::Schema("undeclared native assertion record"))}
        }
    }
}
lctx_model::graph_assertion_records!(assertion_view);
fn view<R: Record + Serialize>(row: &R) -> Result<RecordView, ModelError> {
    row.validate()?;
    let batch = R::encode(std::slice::from_ref(row))?;
    let mut body = Map::new();
    body.insert("__type".into(), Value::String(R::NAME.into()));
    for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
        if field.name() != "id"
            && R::fields()
                .iter()
                .find(|f| f.name() == field.name())
                .is_some_and(|f| f.scalar() != Scalar::Binary)
        {
            body.insert(field.name().clone(), arrow_value(array, 0)?);
        }
    }
    Ok(RecordView {
        semantic_type: R::NAME.into(),
        semantic_key: hex::encode(row.id().bytes()),
        body: Value::Object(body),
    })
}
fn arrow_value(array: &ArrayRef, row: usize) -> Result<Value, ModelError> {
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    macro_rules! scalar {
        ($ty:ty) => {
            if let Some(array) = array.as_any().downcast_ref::<$ty>() {
                return serde_json::to_value(array.value(row)).map_err(ModelError::codec);
            }
        };
    }
    scalar!(BooleanArray);
    scalar!(Int16Array);
    scalar!(Int32Array);
    scalar!(Int64Array);
    scalar!(Float64Array);
    scalar!(StringArray);
    scalar!(FixedSizeBinaryArray);
    scalar!(BinaryArray);
    if let Some(array) = array.as_any().downcast_ref::<ListArray>() {
        let list = array.value(row);
        return (0..list.len())
            .map(|i| arrow_value(&list, i))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array);
    }
    Err(ModelError::Schema("native graph field representation"))
}
