//! Canonical completed-row codecs shared by native physical completion and compiler replay.
//! These checks confer no completion, admission or held native authority.
use lctx_model::domain::{self as d,completed::*,*};
use lctx_model::domain::resources::ResourceBudget;
use arrow_array::{Array,FixedSizeBinaryArray};
use surrealdb::types::{Object,Value,RecordId,RecordIdKey,SerdeWrapper,SurrealValue};
use std::collections::BTreeMap;
use crate::recovery::{RecoveryTableKind,classify_recovery_table};
fn value_object(value:&Value)->Option<&Object>{if let Value::Object(row)=value{Some(row)}else{None}}
pub fn validate_state_row(table: &str, row: &Value) -> Result<Option<OriginalBacking>, ModelError> {
    let object = value_object(row).ok_or(ModelError::Schema("completed state object"))?;
    let Some(Value::RecordId(id)) = object.get("id") else {
        return Err(ModelError::Schema("completed state key"));
    };
    if id.table.as_str() != table {
        return Err(ModelError::Schema("completed state table/key"));
    }
    use d::recovery_closure::RecoveryFamily::*;
    let RecoveryTableKind::Content(family) = classify_recovery_table(table)
        .ok_or(ModelError::Schema("completed state unknown family"))?
        .kind
    else {
        return Err(ModelError::Schema("completed state publication root"));
    };
    match family {
        TypedBacking => return validate_compiler_backing(object, id),
        Contributions => {
            if object.get("completed") != Some(&Value::Bool(true)) {
                return Err(ModelError::Conflict(
                    "pending contribution cannot be sealed",
                ));
            }
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("completed contribution descriptor"));
            };
            let descriptor: CompletedContribution =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            if object.get("logical") != Some(&Value::String(descriptor.identity()?.hex()))
                || object
                    .get("attempt")
                    .and_then(|value| {
                        if let Value::RecordId(attempt) = value {
                            if let RecordIdKey::String(key) = &attempt.key {
                                hex::decode(key)
                                    .ok()
                                    .and_then(|bytes| bytes.try_into().ok())
                                    .map(ContentHash)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .is_none_or(|attempt| {
                        // Producing owners allocate before completion is known; detached
                        // imports already know the full descriptor and preserve distinct
                        // completions of an otherwise identical specification.
                        let producing = attempt_contribution(attempt, descriptor.spec.identity().expect("validated specification"));
                        let detached = attempt_contribution(attempt, descriptor.identity().expect("validated completion"));
                        id.key != RecordIdKey::String(producing.hex())
                            && id.key != RecordIdKey::String(detached.hex())
                    })
            {
                return Err(ModelError::Conflict("completed contribution identity"));
            }
            let Some(Value::Bytes(spec)) = object.get("spec") else {
                return Err(ModelError::Schema("completed contribution specification"));
            };
            if serde_json::from_slice::<ContributionSpec>(spec).map_err(ModelError::codec)?
                != descriptor.spec
            {
                return Err(ModelError::Conflict("completed contribution specification"));
            }
            check_projection(object, contribution_projection(&descriptor.spec))?;
            if object.get("outcome") != Some(&Value::from_t(descriptor.outcome)) {
                return Err(ModelError::Conflict(
                    "completed contribution outcome projection",
                ));
            }
        }
        Memberships => {
            let Some(Value::RecordId(contribution)) = object.get("contribution") else {
                return Err(ModelError::Schema("membership contribution"));
            };
            let surrealdb::types::RecordIdKey::String(physical) = &contribution.key else {
                return Err(ModelError::Schema("membership physical owner"));
            };
            let physical = ContentHash(
                hex::decode(physical)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("membership owner width"))?,
            );
            let Some(Value::String(relation)) = object.get("relation") else {
                return Err(ModelError::Schema("membership relation"));
            };
            let Some(Value::String(key)) = object.get("semantic_key") else {
                return Err(ModelError::Schema("membership nominal key"));
            };
            if contribution.table.as_str() != "compiler_contribution"
                || *id != membership_id(physical, relation, &decode_nominal_key(key)?)
            {
                return Err(ModelError::Conflict(
                    "compiler membership physical identity",
                ));
            }
        }
        Views => {
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("completed view descriptor"));
            };
            let descriptor: CompletedView =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            descriptor.validate()?;
            if id.key != surrealdb::types::RecordIdKey::String(descriptor.identity.hex()) {
                return Err(ModelError::Conflict("completed view key"));
            }
            check_projection(object, view_projection(&descriptor)?)?;
        }
        Bindings => {
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("completed binding descriptor"));
            };
            let descriptor: CompletedBinding =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            descriptor.validate()?;
            let attempt = object
                .get("attempt")
                .and_then(|value| {
                    if let Value::RecordId(attempt) = value {
                        if let RecordIdKey::String(key) = &attempt.key {
                            hex::decode(key)
                                .ok()
                                .and_then(|bytes| bytes.try_into().ok())
                                .map(ContentHash)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .ok_or(ModelError::Schema("native binding attempt"))?;
            if id.key
                != surrealdb::types::RecordIdKey::String(
                    attempt_binding(attempt, descriptor.key()).hex(),
                )
            {
                return Err(ModelError::Conflict("completed binding key"));
            }
            check_projection(object, binding_projection(&descriptor))?;
        }
        Aliases => {}
        GraphPayloads | GraphRoles | ExternalEndpoints | OriginalHeaders | OriginalChunks
        | SearchOccurrences | SearchDocuments | SearchVectors => {
            return Err(ModelError::Schema(
                "non-completed recovery family in completed state",
            ));
        }
    }
    Ok(None)
}

pub struct OriginalBacking {
    pub artifact: Id<d::source::SourceArtifact>,
    pub start: u64,
    pub len: usize,
    pub digest: ContentHash,
}

pub fn validate_compiler_backing(
    row: &Object,
    id: &RecordId,
) -> Result<Option<OriginalBacking>, ModelError> {
    let Some(Value::Object(body)) = row.get("body") else {
        return Err(ModelError::Schema("compiler backing body"));
    };
    let Some(Value::String(name)) = body.get("__type") else {
        return Err(ModelError::Schema("compiler backing type"));
    };
    let (key, original) = if name == d::artifact::ArtifactChunk::NAME {
        require_backing_fields(
            body,
            &[
                "__type", "artifact", "ordinal", "original", "start", "len", "digest",
            ],
        )?;
        let artifact: Id<d::source::SourceArtifact> = decode_backing_id(body, "artifact")?;
        let ordinal = backing_integer(body, "ordinal")?;
        let start = u64::try_from(backing_integer(body, "start")?).map_err(ModelError::codec)?;
        let len = usize::try_from(backing_integer(body, "len")?).map_err(ModelError::codec)?;
        let Some(Value::String(digest)) = body.get("digest") else {
            return Err(ModelError::Schema("compiler original digest"));
        };
        let digest_bytes: [u8; 32] = hex::decode(digest)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("compiler original digest width"))?;
        let digest_hash = ContentHash(digest_bytes);
        if u64::try_from(ordinal)
            .ok()
            .and_then(|ordinal| ordinal.checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64))
            != Some(start)
            || len == 0
            || len > d::artifact::ARTIFACT_CHUNK_BYTES
            || start
                .checked_add(len as u64)
                .is_none_or(|end| end > i64::MAX as u64)
            || digest_hash.hex() != *digest
            || body.get("original")
                != Some(&Value::RecordId(RecordId::new(
                    "original",
                    d::graph::EntityId::of(artifact).0.hex(),
                )))
        {
            return Err(ModelError::Conflict("compiler original metadata"));
        }
        let key = Id::<d::artifact::ArtifactChunk>::of(&d::artifact::ArtifactChunkKey {
            artifact,
            ordinal,
        });
        (
            *key.bytes(),
            Some(OriginalBacking {
                artifact,
                start,
                len,
                digest: digest_hash,
            }),
        )
    } else {
        let adapter = crate::adapter::select(name)?;
        if adapter.table != crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("undeclared compiler backing type"));
        }
        let relation = &adapter.relation;
        let fields = std::iter::once("__type")
            .chain(relation.fields().iter().map(|field| field.name()))
            .collect::<Vec<_>>();
        require_backing_fields(body, &fields)?;
        let Some(Value::String(key)) = row.get("semantic_key") else {
            return Err(ModelError::Schema("compiler backing semantic key"));
        };
        let key: [u8; 16] = hex::decode(key)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("compiler backing semantic key width"))?;
        // This checks physical framing only. The bounded type slice below independently
        // recomputes ordinary-record nominal keys through the model callback.
        (key, None)
    };
    if row.get("semantic_type") != Some(&Value::String(name.clone()))
        || row.get("semantic_key") != Some(&Value::String(hex::encode(key)))
        || *id
            != crate::loader::payload_id(
                "compiler_record",
                name,
                &key,
                ContentHash::of(
                    serde_json::to_vec(&Value::Object(body.clone()))
                        .map_err(ModelError::codec)?
                        .as_slice(),
                ),
            )?
    {
        return Err(ModelError::Conflict("compiler backing typed identity"));
    }
    let Some(Value::Bytes(canonical)) = row.get("canonical") else {
        return Err(ModelError::Schema("compiler backing canonical bytes"));
    };
    let canonical_body: Value = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
    if canonical_body != Value::Object(body.clone())
        || canonical.as_ref()
            != serde_json::to_vec(&Value::Object(body.clone()))
                .map_err(ModelError::codec)?
                .as_slice()
    {
        return Err(ModelError::Conflict("compiler backing canonical body"));
    }
    if row.get("content") != Some(&Value::String(ContentHash::of(canonical).hex())) {
        return Err(ModelError::Conflict("compiler backing content"));
    }
    let adapter = crate::adapter::select(name)?;
    if adapter.table != crate::schema::ScopeTable::CompilerRecord {
        return Err(ModelError::Schema("undeclared compiler backing type"));
    }
    let expected = crate::adapter::physical_row(
        id.clone(),
        ContentHash::of(canonical),
        canonical.to_vec(),
        None,
        crate::codec::RecordView {
            semantic_type: name.clone(),
            semantic_key: hex::encode(key),
            scopes: adapter.scopes(&canonical_body)?,
            body: canonical_body,
        },
    )?;
    if serde_json::to_vec(&Value::Object(row.clone())).map_err(ModelError::codec)?
        != serde_json::to_vec(&expected).map_err(ModelError::codec)?
    {
        return Err(ModelError::Conflict("compiler backing complete envelope"));
    }
    Ok(original)
}

pub fn require_backing_fields(body: &Object, fields: &[&str]) -> Result<(), ModelError> {
    if body.len() != fields.len() || fields.iter().any(|field| !body.contains_key(*field)) {
        return Err(ModelError::Schema("compiler backing closed body"));
    }
    Ok(())
}

pub fn backing_integer(body: &Object, field: &str) -> Result<i64, ModelError> {
    match body.get(field) {
        Some(Value::Number(surrealdb::types::Number::Int(value))) => Ok(*value),
        _ => Err(ModelError::Schema("compiler backing declared integer")),
    }
}

pub fn decode_backing_id<T: Record>(body: &Object, field: &str) -> Result<Id<T>, ModelError> {
    let Some(Value::Array(values)) = body.get(field) else {
        return Err(ModelError::Schema("compiler backing nominal bytes"));
    };
    if values.len()!=16 || values.iter().any(|value|!matches!(value,Value::Number(surrealdb::types::Number::Int(value)) if u8::try_from(*value).is_ok())) {return Err(ModelError::Schema("compiler backing nominal bytes"));}
    Ok(
        SerdeWrapper::<Id<T>>::from_value(Value::Array(values.clone()))
            .map_err(ModelError::codec)?
            .0,
    )
}

pub fn contribution_projection(spec: &ContributionSpec) -> Object {
    let mut fields = Object::new();
    fields.insert("producer", spec.producer.clone());
    fields.insert("profile", spec.profile.name().to_string());
    fields.insert("model", spec.model.hex());
    fields.insert("implementation", spec.implementation.hex());
    fields.insert(
        "configuration",
        spec.configuration
            .map(|hash| Value::String(hash.hex()))
            .unwrap_or(Value::Null),
    );
    let views = spec
        .inputs
        .iter()
        .map(|source| source.view())
        .collect::<std::collections::BTreeSet<_>>();
    fields.insert(
        "inputs",
        views
            .iter()
            .map(|view| RecordId::new("compiler_view", view.hex()))
            .collect::<Vec<_>>(),
    );
    fields.insert("outputs", spec.outputs.iter().cloned().collect::<Vec<_>>());
    fields
}

pub fn view_projection(view: &CompletedView) -> Result<Object, ModelError> {
    let mut fields = Object::new();
    fields.insert("relation", view.relation.clone());
    fields.insert(
        "contributions",
        view.contributions
            .iter()
            .map(ContentHash::hex)
            .collect::<Vec<_>>(),
    );
    fields.insert("rows", i64::try_from(view.rows).map_err(ModelError::codec)?);
    Ok(fields)
}

pub fn binding_projection(binding: &CompletedBinding) -> Object {
    let mut fields = Object::new();
    fields.insert("relation", binding.view.relation.clone());
    fields.insert(
        "boundary",
        binding
            .boundary
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    fields.insert(
        "view",
        RecordId::new("compiler_view", binding.view.identity.hex()),
    );
    fields
}

pub fn check_projection(actual: &Object, expected: Object) -> Result<(), ModelError> {
    for (field, value) in expected {
        if actual.get(&field) != Some(&value) {
            return Err(ModelError::Conflict("completed metadata projection"));
        }
    }
    Ok(())
}

pub fn validate_backing_batch(rows: &[Value], budget: &ResourceBudget) -> Result<(), ModelError> {
    let mut groups = BTreeMap::<&str, Vec<&Object>>::new();
    for row in rows {
        let object = value_object(row).ok_or(ModelError::Schema("compiler backing row"))?;
        let Some(Value::String(name)) = object.get("semantic_type") else {
            return Err(ModelError::Schema("compiler backing semantic type"));
        };
        if name != d::artifact::ArtifactChunk::NAME {
            groups.entry(name).or_default().push(object);
        }
    }
    for (name, mut rows) in groups {
        let adapter = crate::adapter::select(name)?;
        if adapter.table != crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("undeclared compiler backing type"));
        }
        let relation = &adapter.relation;
        rows.sort_by_key(|row| match row.get("semantic_key") {
            Some(Value::String(key)) => key.as_str(),
            _ => "",
        });
        let mut builder = crate::projected_arrow::ProjectedBuilder::new(
            relation.clone(),
            relation.schema().clone(),
            &budget,
        )?;
        for row in &rows {
            let Some(Value::Object(body)) = row.get("body") else {
                return Err(ModelError::Schema("compiler backing body"));
            };
            let mut fields = body.clone();
            fields.insert(
                "id",
                row.get("semantic_key")
                    .ok_or(ModelError::Schema("compiler backing semantic key"))?
                    .clone(),
            );
            builder.push(Value::Object(fields))?;
        }
        let batch = relation
            .canonical(&builder.finish()?)
            .map_err(|_| ModelError::Conflict("compiler backing typed identity"))?;
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("compiler backing nominal column"))?;
        let bodies = crate::codec::batch_bodies(relation, &batch)?;
        if batch.num_rows() != rows.len() {
            return Err(ModelError::Conflict("compiler backing type slice"));
        }
        for (index, (row, body)) in rows.iter().zip(bodies).enumerate() {
            let key = hex::encode(ids.value(index));
            if row.get("semantic_key") != Some(&Value::String(key.clone())) {
                return Err(ModelError::Conflict("compiler backing typed identity"));
            }
            let canonical = serde_json::to_vec(&body).map_err(ModelError::codec)?;
            let expected = crate::adapter::physical_row(
                crate::loader::payload_id(
                    "compiler_record",
                    name,
                    ids.value(index),
                    ContentHash::of(&canonical),
                )?,
                ContentHash::of(&canonical),
                canonical,
                None,
                crate::codec::RecordView {
                    semantic_type: name.into(),
                    semantic_key: key,
                    scopes: adapter.scopes(&body)?,
                    body,
                },
            )?;
            if serde_json::to_vec(&Value::Object((*row).clone())).map_err(ModelError::codec)?
                != serde_json::to_vec(&expected).map_err(ModelError::codec)?
            {
                return Err(ModelError::Conflict("compiler backing declared body"));
            }
        }
    }
    Ok(())
}

pub fn admit_backing_row(
    pending: &mut Vec<Value>,
    bytes: &mut usize,
    row: Value,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let weight = crate::loader::native_bytes(&row);
    if weight > d::resources::MAX_ROW_BYTES {
        return Err(ModelError::Schema("compiler backing row bound"));
    }
    if !pending.is_empty()
        && (pending.len() >= d::resources::TRANSFER_ROWS
            || bytes.saturating_add(weight) > d::resources::TRANSFER_BYTES)
    {
        validate_backing_batch(pending, budget)?;
        pending.clear();
        *bytes = 0;
    }
    *bytes = bytes.saturating_add(weight);
    pending.push(row);
    Ok(())
}

pub fn membership_id(contribution: ContentHash, relation: &str, key: &[u8]) -> RecordId {
    let mut sink = KeySink::new("compiler-membership/v1");
    contribution.encode(&mut sink);
    relation.to_string().encode(&mut sink);
    sink.part(b"key", key);
    RecordId::new("compiler_membership", sink.finish().hex())
}

pub fn decode_nominal_key(key: &str) -> Result<[u8; 16], ModelError> {
    hex::decode(key)
        .map_err(ModelError::codec)?
        .try_into()
        .map_err(|_| ModelError::Schema("native nominal key width"))
}
