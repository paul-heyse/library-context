//! Mechanical query fields alongside the unchanged canonical graph payload.
use arrow_array::{
    Array, ArrayRef, BinaryArray, BooleanArray, FixedSizeBinaryArray, Float64Array, Int16Array,
    Int32Array, Int64Array, ListArray, StringArray,
};
use lctx_model::domain::{
    ModelError, Record, SemanticReference,
    graph::{Assertion, Entity},
};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordView {
    pub semantic_type: String,
    pub semantic_key: String,
    pub body: surrealdb::types::Value,
    pub(crate) scopes: surrealdb::types::Object,
}
macro_rules! entity_view {
    ($($variant:ident:$ty:ty,)*)=>{
        pub fn entity_references(entity:&Entity)->Vec<SemanticReference>{match entity{$(Entity::$variant(row)=>row.references(),)*}}
        pub fn entity_views(entities:&[Entity])->Result<Vec<RecordView>,ModelError>{
            let mut groups=std::collections::BTreeMap::<&str,Vec<(usize,&Entity)>>::new();
            for (position,entity) in entities.iter().enumerate(){
                let name=match entity{$(Entity::$variant(_)=><$ty>::NAME,)*};
                groups.entry(name).or_default().push((position,entity));
            }
            let mut output:Vec<Option<RecordView>>=vec![None;entities.len()];
            for (name,group) in groups {
                let rows=group.iter().map(|(_,row)|*row).collect::<Vec<_>>();
                for ((position,_),view) in group.into_iter().zip(crate::adapter::select(name)?.entities(&rows)?) {output[position]=Some(view);}
            }
            complete_views(output)
        }
    }
}
lctx_model::graph_entity_records!(entity_view);
pub use lctx_model::domain::graph::record::{assertion_record, entity_record};
macro_rules! assertion_view {
    ($($variant:ident:$ty:ty,)*) => {
        pub fn assertion_views(assertions: &[Assertion]) -> Result<Vec<RecordView>, ModelError> {
            let mut groups = std::collections::BTreeMap::<&str, Vec<(usize, &Assertion)>>::new();
            let mut output: Vec<Option<RecordView>> = vec![None; assertions.len()];
            for (position, assertion) in assertions.iter().enumerate() {
                if let Some(source) = &assertion.source {
                    groups
                        .entry(source.domain())
                        .or_default()
                        .push((position, assertion));
                } else {
                    let mut body = surrealdb::types::Object::new();
                    body.insert("__type", "__graph_assertion");
                    output[position] = Some(RecordView {
                        semantic_type: "__graph_assertion".into(),
                        semantic_key: assertion.id().0.hex(),
                        body: surrealdb::types::Value::Object(body),
                        scopes: {
                            let mut scopes = surrealdb::types::Object::new();
                            scopes.insert("scope_keys", Vec::<String>::new());
                            scopes
                        },
                    });
                }
            }
            for (name, group) in groups {
                let rows = group.iter().map(|(_, row)| *row).collect::<Vec<_>>();
                for ((position, _), view) in group
                    .into_iter()
                    .zip(crate::adapter::select(name)?.assertions(&rows)?)
                {
                    output[position] = Some(view);
                }
            }
            complete_views(output)
        }
    };
}
lctx_model::graph_assertion_records!(assertion_view);
fn complete_views(output: Vec<Option<RecordView>>) -> Result<Vec<RecordView>, ModelError> {
    output
        .into_iter()
        .map(|row| row.ok_or(ModelError::Schema("native body inventory")))
        .collect()
}
#[cfg(test)]
pub(crate) fn views<R: Record>(rows: &[R]) -> Result<Vec<RecordView>, ModelError> {
    views_with_adapter(rows, crate::adapter::select(R::NAME)?)
}
pub(crate) fn views_with_adapter<R: Record>(
    rows: &[R],
    adapter: &crate::adapter::Adapter,
) -> Result<Vec<RecordView>, ModelError> {
    for row in rows {
        row.validate()?;
    }
    let batch = R::encode(rows)?;
    let bodies = batch_bodies(&adapter.relation, &batch)?;
    if bodies.len() != rows.len() {
        return Err(ModelError::Schema("native body count"));
    }
    rows.iter()
        .zip(bodies)
        .map(|(row, body)| {
            Ok(RecordView {
                semantic_type: R::NAME.into(),
                semantic_key: hex::encode(row.id().bytes()),
                scopes: adapter.scopes(&body)?,
                body,
            })
        })
        .collect()
}

/// Whole-batch field lowering. Opaque bytes retain native Bytes, while declared textual
/// binary fields remain exact UTF-8. No per-row semantic re-encoding is involved.
pub fn batch_bodies(
    relation: &lctx_model::domain::Relation,
    batch: &arrow_array::RecordBatch,
) -> Result<Vec<surrealdb::types::Value>, ModelError> {
    use surrealdb::types::{Object, Value as NativeValue};
    let mut rows = (0..batch.num_rows())
        .map(|_| {
            let mut body = Object::new();
            body.insert("__type", relation.name().to_string());
            body
        })
        .collect::<Vec<_>>();
    for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
        if field.name() == "id" {
            continue;
        }
        let descriptor = relation
            .fields()
            .iter()
            .find(|f| f.name() == field.name())
            .ok_or(ModelError::Schema("undeclared native body field"))?;
        for (row, body) in rows.iter_mut().enumerate() {
            body.insert(
                field.name().clone(),
                native_arrow_value(array, row, descriptor.textual())?,
            );
        }
    }
    Ok(rows.into_iter().map(NativeValue::Object).collect())
}
/// Reconstruct canonical typed ingress batches from portable native fields. Each body carries
/// its nominal `id`; this is data and confers no completed-owner authority.
pub fn decode_bodies(relation: &lctx_model::domain::Relation, bodies: Vec<surrealdb::types::Value>, budget: &lctx_model::domain::resources::ResourceBudget) -> Result<arrow_array::RecordBatch, ModelError> {
    let mut builder = crate::projected_arrow::ProjectedBuilder::new(relation.clone(), relation.schema().clone(), budget)?;
    for body in bodies { builder.push(body)?; }
    relation.canonical(&builder.finish()?)
}
fn native_arrow_value(
    array: &ArrayRef,
    row: usize,
    textual: bool,
) -> Result<surrealdb::types::Value, ModelError> {
    use surrealdb::types::{Bytes, Value as V};
    if array.is_null(row) {
        return Ok(V::Null);
    }
    macro_rules! scalar {
        ($ty:ty) => {
            if let Some(a) = array.as_any().downcast_ref::<$ty>() {
                return Ok(V::from_t(a.value(row)));
            }
        };
    }
    scalar!(BooleanArray);
    scalar!(Int16Array);
    scalar!(Int32Array);
    scalar!(Int64Array);
    scalar!(StringArray);
    if let Some(a) = array.as_any().downcast_ref::<Float64Array>() {
        let v = a.value(row);
        if !v.is_finite() {
            return Err(ModelError::Schema("native finite float"));
        }
        return Ok(V::from_t(v));
    }
    if let Some(a) = array.as_any().downcast_ref::<FixedSizeBinaryArray>() {
        return Ok(V::from_t(
            a.value(row)
                .iter()
                .map(|byte| i64::from(*byte))
                .collect::<Vec<_>>(),
        ));
    }
    if let Some(a) = array.as_any().downcast_ref::<BinaryArray>() {
        return if textual {
            Ok(V::from_t(
                std::str::from_utf8(a.value(row))
                    .map_err(ModelError::codec)?
                    .to_string(),
            ))
        } else {
            Ok(V::Bytes(Bytes::from(a.value(row).to_vec())))
        };
    }
    if let Some(a) = array.as_any().downcast_ref::<ListArray>() {
        let values = a.value(row);
        return Ok(V::from_t(
            (0..values.len())
                .map(|i| native_arrow_value(&values, i, textual))
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    Err(ModelError::Schema("native batch field representation"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{
        EvidenceBytes, artifact::ArtifactChunk, input::Package, value::Literal,
    };
    use surrealdb::types::{Bytes, Value};
    #[test]
    fn grouped_native_bodies_preserve_order_text_bytes_and_inactive_sum_fields() {
        let rows = vec![
            Entity::from(Literal::String {
                value: "exact 雪\n\0text".into(),
            }),
            Entity::from(Package {
                name: "native-codec".into(),
            }),
            Entity::from(Literal::Bytes {
                value: EvidenceBytes(vec![0xff, 0, 0x80]),
            }),
            Entity::from(Literal::None),
        ];
        let bodies = entity_views(&rows).unwrap();
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.semantic_type.as_str())
                .collect::<Vec<_>>(),
            [Literal::NAME, Package::NAME, Literal::NAME, Literal::NAME]
        );
        for (row, body) in rows.iter().zip(&bodies) {
            assert_eq!(
                body.semantic_key,
                match row {
                    Entity::Literal(row) => hex::encode(row.id().bytes()),
                    Entity::Package(row) => hex::encode(row.id().bytes()),
                    _ => unreachable!(),
                }
            );
        }
        let text = bodies[0].body.as_object().unwrap();
        let opaque = bodies[2].body.as_object().unwrap();
        assert!(
            text.values()
                .any(|value| value == Value::from_t("exact 雪\n\0text"))
        );
        assert!(
            opaque
                .values()
                .any(|value| value == Value::Bytes(Bytes::from(vec![0xff, 0, 0x80])))
        );
        assert!(
            bodies[3]
                .body
                .as_object()
                .unwrap()
                .values()
                .filter(|value| matches!(value, Value::Null))
                .count()
                >= 3
        );
    }
    #[test]
    fn source_less_assertions_keep_minimal_body_and_full_graph_key() {
        use lctx_model::domain::{
            ContentHash,
            graph::{
                AssertionKind, AssertionValue, EntityId, InlineQualification, Participant,
                ParticipantRole, Qualification, Target,
            },
        };
        // Codec input: endpoint/semantic admission is a separate owner.
        let assertion = Assertion {
            source: None,
            kind: AssertionKind::ParameterBinding,
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                field: Some("subject".into()),
                position: None,
                target: Target::Entity(EntityId(ContentHash::of(b"subject"))),
            }],
            qualification: Qualification::Inline(InlineQualification {
                context: EntityId(ContentHash::of(b"context")),
                scope: EntityId(ContentHash::of(b"scope")),
                condition: EntityId(ContentHash::of(b"condition")),
                modality: lctx_model::domain::attribution::Modality::Definite,
                approximation: lctx_model::domain::assertion::Approximation::Exact,
                assumptions: vec![],
            }),
            run: None,
            evidence: vec![],
            value: AssertionValue::None,
            derivation: None,
        };
        assertion.validate().unwrap();
        let view = assertion_views(std::slice::from_ref(&assertion))
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(view.semantic_type, "__graph_assertion");
        assert_eq!(view.semantic_key, assertion.id().0.hex());
        assert_eq!(view.body.as_object().unwrap().len(), 1);
        assert_eq!(
            view.body.as_object().unwrap().get("__type"),
            Some(&Value::from_t("__graph_assertion"))
        );
        assert_eq!(
            view.scopes.get("scope_keys"),
            Some(&Value::from_t(Vec::<String>::new()))
        );
        assert!(!view.scopes.contains_key("scope_context"));
    }
    #[test]
    fn compiler_record_batch_keeps_opaque_binary_and_typed_id() {
        let artifact = serde_json::from_value(serde_json::to_value([3u8; 16]).unwrap()).unwrap();
        let rows = vec![
            ArtifactChunk {
                artifact,
                ordinal: 0,
                body: EvidenceBytes(vec![0xff, 0, 0x80]),
            },
            ArtifactChunk {
                artifact,
                ordinal: 1,
                body: EvidenceBytes(vec![7]),
            },
        ];
        let bodies = views(&rows).unwrap();
        assert_eq!(
            bodies[0].body.as_object().unwrap().get("body"),
            Some(&Value::Bytes(Bytes::from(vec![0xff, 0, 0x80])))
        );
        assert_eq!(
            bodies[1].body.as_object().unwrap().get("ordinal"),
            Some(&Value::from_t(1i64))
        );
        assert_eq!(
            bodies[0].body.as_object().unwrap().get("artifact"),
            Some(&Value::from_t(vec![3i64; 16]))
        );
    }
    #[test]
    fn nested_nulls_and_invalid_text_are_not_silently_coerced() {
        use arrow_array::types::Int64Type;
        let list: ArrayRef =
            std::sync::Arc::new(ListArray::from_iter_primitive::<Int64Type, _, _>([
                Some(vec![Some(9), None, Some(-2)]),
                None,
            ]));
        assert_eq!(
            native_arrow_value(&list, 0, false).unwrap(),
            Value::from_t(vec![Value::from_t(9i64), Value::Null, Value::from_t(-2i64)])
        );
        assert_eq!(native_arrow_value(&list, 1, false).unwrap(), Value::Null);
        let invalid: ArrayRef = std::sync::Arc::new(BinaryArray::from(vec![Some(&[0xff][..])]));
        assert!(native_arrow_value(&invalid, 0, true).is_err());
        assert_eq!(
            native_arrow_value(&invalid, 0, false).unwrap(),
            Value::Bytes(Bytes::from(vec![0xff]))
        );
    }
}
