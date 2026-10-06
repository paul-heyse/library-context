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
pub use lctx_model::domain::graph::record::{assertion_record, entity_record};
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
            && let Some(descriptor) = R::fields().iter().find(|f| f.name() == field.name())
        {
            if descriptor.scalar() != Scalar::Binary {
                body.insert(field.name().clone(), arrow_value(array, 0)?);
            } else if descriptor.textual() {
                let bytes = array
                    .as_any()
                    .downcast_ref::<BinaryArray>()
                    .ok_or(ModelError::Schema("native exact text representation"))?;
                let value = if bytes.is_null(0) {
                    Value::Null
                } else {
                    Value::String(
                        std::str::from_utf8(bytes.value(0))
                            .map_err(ModelError::codec)?
                            .into(),
                    )
                };
                body.insert(field.name().clone(), value);
            }
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

#[cfg(test)]
mod remediation_text_projection {
    use super::*;
    use lctx_model::domain::{
        ContentHash,
        retrieval::{CorpusText, Family, RENDER_VERSION},
    };
    #[test]
    fn binary_backed_text_is_projected_as_exact_unicode_text() {
        let text = "discover 雪\nexact evidence";
        let row = CorpusText {
            family: Family::Source,
            rendering_version: RENDER_VERSION,
            digest: ContentHash::of(text.as_bytes()),
            text: text.into(),
        };
        let projected = view(&row).unwrap();
        assert_eq!(projected.body["text"], text);
    }
    #[test]
    fn enum_text_is_exact_and_opaque_binary_has_no_query_text() {
        use lctx_model::domain::{EvidenceBytes, value::Literal};
        let text = Literal::String {
            value: "雪\nexact".into(),
        };
        assert_eq!(view(&text).unwrap().body["string_value"], "雪\nexact");
        let bytes = Literal::Bytes {
            value: EvidenceBytes(b"not text".to_vec()),
        };
        assert!(view(&bytes).unwrap().body.get("bytes_value").is_none());
    }
}
