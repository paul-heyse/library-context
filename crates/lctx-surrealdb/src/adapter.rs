//! Declaration-selected synchronous physical adapters. Selection is once per type slice,
//! never an inventory walk inside a row write or an async dispatch frame.
use crate::{
    codec::{self, RecordView},
    schema::ScopeTable,
};
use arrow_array::RecordBatch;
use lctx_model::domain::{
    self as d, Field, ModelError, Record, Relation,
    graph::{Assertion, Entity},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use surrealdb::types::{Object, Value};

type GraphDecoder = fn(&RecordBatch) -> Result<Vec<GraphRow>, ModelError>;
type EntityCodec = fn(&Adapter, &[&Entity]) -> Result<Vec<RecordView>, ModelError>;
type AssertionCodec = fn(&Adapter, &[&Assertion]) -> Result<Vec<RecordView>, ModelError>;

#[derive(Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "Typed bounded graph windows avoid per-row boxing"
)]
pub(crate) enum GraphRow {
    Entity(Entity),
    Assertion(Assertion),
}
pub(crate) struct Adapter {
    pub(crate) relation: Relation,
    pub(crate) table: ScopeTable,
    pub(crate) scope_fields: BTreeSet<&'static str>,
    graph: Option<GraphDecoder>,
    entity: Option<EntityCodec>,
    assertion: Option<AssertionCodec>,
}
impl Adapter {
    fn new(relation: Relation, table: ScopeTable) -> Self {
        let scope_fields = relation
            .fields()
            .iter()
            .filter(|field| {
                !field.list()
                    && (field.target().is_some()
                        || crate::schema::SCOPE_FIELDS.contains(&field.name()))
            })
            .map(Field::name)
            .collect();
        Self {
            relation,
            table,
            scope_fields,
            graph: None,
            entity: None,
            assertion: None,
        }
    }
    pub(crate) fn graph(&self, batch: &RecordBatch) -> Result<Vec<Option<GraphRow>>, ModelError> {
        match self.graph {
            Some(decode) => Ok(decode(batch)?.into_iter().map(Some).collect()),
            None => Ok(vec![None; batch.num_rows()]),
        }
    }
    pub(crate) fn entities(&self, rows: &[&Entity]) -> Result<Vec<RecordView>, ModelError> {
        self.entity
            .ok_or(ModelError::Schema("native entity adapter"))?(self, rows)
    }
    pub(crate) fn assertions(&self, rows: &[&Assertion]) -> Result<Vec<RecordView>, ModelError> {
        self.assertion
            .ok_or(ModelError::Schema("native assertion adapter"))?(self, rows)
    }
}
pub(crate) fn select(name: &str) -> Result<&'static Adapter, ModelError> {
    static ADAPTERS: OnceLock<BTreeMap<&'static str, Adapter>> = OnceLock::new();
    ADAPTERS.get_or_init(|| {
        let mut adapters=BTreeMap::new();
        macro_rules! entities {($($variant:ident:$ty:ty,)*)=>{$({
            let mut adapter=Adapter::new(Relation::of::<$ty>(),ScopeTable::Entity);
            adapter.graph=Some(|batch| <$ty>::decode(batch)?.into_iter().map(|row| {let entity=Entity::from(row);entity.validate()?;Ok(GraphRow::Entity(entity))}).collect());
            adapter.entity=Some(|adapter, rows| {let typed=rows.iter().map(|row|match row {Entity::$variant(row)=>Ok(row.clone()),_=>Err(ModelError::Schema("native entity group"))}).collect::<Result<Vec<$ty>,_>>()?;codec::views_with_adapter(&typed,adapter)});
            assert!(adapters.insert(<$ty>::NAME,adapter).is_none());
        })*};}
        lctx_model::graph_entity_records!(entities);
        macro_rules! assertions {($($variant:ident:$ty:ty,)*)=>{$({
            let mut adapter=Adapter::new(Relation::of::<$ty>(),ScopeTable::Assertion);
            adapter.graph=Some(|batch| <$ty>::decode(batch)?.into_iter().map(|row|Assertion::from_record(row).map(GraphRow::Assertion)).collect());
            adapter.assertion=Some(|adapter, rows| {let typed=rows.iter().map(|row|codec::assertion_record::<$ty>(row)).collect::<Result<Vec<_>,_>>()?;codec::views_with_adapter(&typed,adapter)});
            assert!(adapters.insert(<$ty>::NAME,adapter).is_none());
        })*};}
        lctx_model::graph_assertion_records!(assertions);
        for relation in crate::schema::compiler_relations() {
            assert!(adapters.insert(relation.name(),Adapter::new(relation.clone(),ScopeTable::CompilerRecord)).is_none());
        }
        adapters
    }).get(name).ok_or(ModelError::Schema("undeclared native physical relation"))
}

impl Adapter {
    pub(crate) fn scopes(&self, body: &Value) -> Result<Object, ModelError> {
        scope_fields(
            body,
            self.relation.name(),
            self.table,
            Some(&self.scope_fields),
        )
    }
}
fn scope_fields(
    body: &Value,
    name: &str,
    table: ScopeTable,
    fields: Option<&BTreeSet<&'static str>>,
) -> Result<Object, ModelError> {
    let Value::Object(body) = body else {
        return Err(ModelError::Schema("canonical scope body"));
    };
    let mut object = Object::new();
    let mut keys = Vec::new();
    if let Some(fields) = fields {
        for field in fields {
            if let Some(value) = body
                .get(*field)
                .filter(|value| !matches!(value, Value::None | Value::Null))
            {
                keys.push(format!(
                    "{name}|{field}|{}",
                    crate::prepared::scope_string(value)
                ));
            }
        }
    }
    if table != ScopeTable::CompilerRecord
        && let Some(value) = body
            .get("context")
            .filter(|value| !matches!(value, Value::None))
    {
        object.insert("scope_context", crate::prepared::scope_string(value));
    }
    object.insert("scope_keys", keys);
    Ok(object)
}
pub(crate) fn add_scope_fields(
    object: &mut Object,
    body: &Value,
    name: &str,
    table: ScopeTable,
) -> Result<(), ModelError> {
    let scope = if name == "__graph_assertion" {
        if table != ScopeTable::Assertion {
            return Err(ModelError::Schema("native source-less assertion table"));
        }
        scope_fields(body, name, table, None)?
    } else {
        let adapter = select(name)?;
        if adapter.table != table {
            return Err(ModelError::Schema("native scope table/relation"));
        }
        adapter.scopes(body)?
    };
    for (name, value) in scope {
        object.insert(name, value);
    }
    Ok(())
}

/// All arithmetic is checked synchronously before any original page is persisted.
pub(crate) fn original_metadata(row: &d::artifact::ArtifactChunk) -> Result<Value, ModelError> {
    row.validate()?;
    let start = u64::try_from(row.ordinal)
        .map_err(ModelError::codec)?
        .checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64)
        .ok_or(ModelError::Schema("original chunk start"))?;
    start
        .checked_add(row.body.0.len() as u64)
        .filter(|end| *end <= i64::MAX as u64)
        .ok_or(ModelError::Schema("original chunk range"))?;
    let mut body = Object::new();
    body.insert("__type", d::artifact::ArtifactChunk::NAME);
    body.insert(
        "artifact",
        row.artifact
            .bytes()
            .iter()
            .map(|byte| i64::from(*byte))
            .collect::<Vec<_>>(),
    );
    body.insert("ordinal", row.ordinal);
    body.insert(
        "original",
        surrealdb::types::RecordId::new("original", d::graph::EntityId::of(row.artifact).0.hex()),
    );
    body.insert("start", start);
    body.insert("len", row.body.0.len() as u64);
    body.insert("digest", d::ContentHash::of(&row.body.0).hex());
    Ok(Value::Object(body))
}

/// Complete mechanical envelope, shared by every writer and cold reconstruction.
pub(crate) fn physical_row(
    id: surrealdb::types::RecordId,
    content: d::ContentHash,
    canonical: Vec<u8>,
    graph: Option<(i64, Option<i16>)>,
    view: RecordView,
) -> Result<Value, ModelError> {
    let mut row = view.scopes;
    row.insert("id", id.clone());
    row.insert("semantic_type", view.semantic_type);
    row.insert("semantic_key", view.semantic_key);
    row.insert("content", content.hex());
    row.insert("canonical", surrealdb::types::Bytes::from(canonical.clone()));
    row.insert("body", view.body);
    if let Some((kind, subtype)) = graph {
        let anchor=match id.table.as_str() {
            "entity" => {let entity:d::graph::Entity=serde_json::from_slice(&canonical).map_err(ModelError::codec)?;crate::reader::target_id(d::graph::Target::Entity(entity.id()))},
            "assertion" => {let assertion:d::graph::Assertion=serde_json::from_slice(&canonical).map_err(ModelError::codec)?;crate::reader::target_id(d::graph::Target::Assertion(assertion.id()))},
            _=>return Err(ModelError::Schema("graph payload family")),
        };
        row.insert("anchor",anchor);
        row.insert("kind", kind);
        row.insert("subtype", subtype.map(Value::from_t).unwrap_or(Value::Null));
    }
    let row = Value::Object(row);
    crate::loader::validate_native_row(&row)?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::{
        EvidenceBytes, Id,
        artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunk},
        source::SourceArtifact,
    };
    #[test]
    fn whole_original_window_checks_range_before_effects() {
        let artifact: Id<SourceArtifact> =
            serde_json::from_value(serde_json::to_value([3u8; 16]).unwrap()).unwrap();
        let last_ordinal = i64::MAX / (ARTIFACT_CHUNK_BYTES as i64);
        let rows = [
            ArtifactChunk {
                artifact,
                ordinal: 0,
                body: EvidenceBytes(vec![0xff]),
            },
            ArtifactChunk {
                artifact,
                ordinal: last_ordinal + 1,
                body: EvidenceBytes(vec![1]),
            },
        ];
        assert!(
            rows[1].validate().is_ok(),
            "model validation alone does not establish physical integer range"
        );
        assert!(
            rows.iter()
                .map(original_metadata)
                .collect::<Result<Vec<_>, _>>()
                .is_err()
        );
        let end_overflow = ArtifactChunk {
            artifact,
            ordinal: last_ordinal,
            body: EvidenceBytes(vec![0; ARTIFACT_CHUNK_BYTES]),
        };
        assert!(original_metadata(&end_overflow).is_err());
        let metadata = original_metadata(&rows[0]).unwrap();
        assert_eq!(
            metadata.as_object().unwrap().get("start"),
            Some(&Value::from_t(0i64))
        );
    }
}
