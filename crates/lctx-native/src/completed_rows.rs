//! Canonical completed-row codecs shared by native physical completion and compiler replay.
//! These checks confer no completion, admission or held native authority.
use crate::recovery::{RecoveryTableKind, classify_logical_recovery_table};
use arrow_array::{Array, FixedSizeBinaryArray};
use lctx_model::domain::resources::ResourceBudget;
use lctx_model::domain::{self as d, completed::*, *};
use std::collections::BTreeMap;
use surrealdb::types::{Object, RecordId, RecordIdKey, SerdeWrapper, SurrealValue, Value};
fn value_object(value: &Value) -> Option<&Object> {
    if let Value::Object(row) = value {
        Some(row)
    } else {
        None
    }
}
/// Decoded endpoints and their allocation authority. Retain the reservation through consumption.
pub struct CanonicalBackingEndpoints {
    pub endpoints: Vec<Value>,
    pub original: Option<OriginalBacking>,
    pub reservation: Box<dyn d::resources::Reservation>,
}
/// Validate complete canonical backing bytes and reconstruct their exact endpoint rows.
/// Keep the returned reservation alive while consuming the decoded endpoints. This
/// lowers content only; callers independently establish native ownership and guards.
pub fn canonical_backing_endpoints(
    row: &Value,
    budget: &ResourceBudget,
) -> Result<CanonicalBackingEndpoints, ModelError> {
    use d::graph::{Assertion, Entity};
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("installed backing object"))?;
    let Some(Value::RecordId(node)) = object.get("id") else {
        return Err(ModelError::Schema("installed backing id"));
    };
    // Nongraph framing borrows retained canonical bytes and parses one current
    // body; its typed Arrow builder separately admits its own arenas. Graph
    // reconstruction still owns complete typed entity/role representations.
    let expansion = if node.table.as_str() == "compiler_record" {
        2
    } else {
        32
    };
    let decode = budget.reserve(
        "installed backing canonical decode",
        crate::loader::native_bytes(row)
            .saturating_mul(expansion)
            .saturating_add(65536),
    )?;
    let canonical = match object.get("canonical") {
        Some(Value::Bytes(bytes)) => bytes,
        _ => return Err(ModelError::Schema("installed canonical backing")),
    };
    let (expected, endpoints) = match node.table.as_str() {
        "compiler_record" => {
            let original = validate_state_row("compiler_record", row)?;
            validate_backing_batch(std::slice::from_ref(row), budget)?;
            return Ok(CanonicalBackingEndpoints {
                endpoints: vec![],
                original,
                reservation: decode,
            });
        }
        "entity" => {
            let entity: Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
            entity.validate()?;
            let view = crate::codec::entity_views(std::slice::from_ref(&entity))?
                .pop()
                .ok_or(ModelError::Schema("installed entity view"))?;
            (
                crate::adapter::physical_row(
                    crate::loader::entity_payload_id(&entity)?,
                    entity.content(),
                    serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                    Some((entity.kind() as i64, entity.subtype())),
                    view,
                )?,
                crate::loader::entity_endpoint_values(&entity)?,
            )
        }
        "assertion" => {
            let assertion: Assertion =
                serde_json::from_slice(canonical).map_err(ModelError::codec)?;
            assertion.validate()?;
            let view = crate::codec::assertion_views(std::slice::from_ref(&assertion))?
                .pop()
                .ok_or(ModelError::Schema("installed assertion view"))?;
            (
                crate::adapter::physical_row(
                    crate::loader::assertion_payload_id(&assertion)?,
                    assertion.content(),
                    serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                    Some((assertion.kind as i64, None)),
                    view,
                )?,
                crate::loader::assertion_endpoint_values(&assertion)?,
            )
        }
        _ => return Err(ModelError::Conflict("installed undeclared backing family")),
    };
    if expected != *row {
        return Err(ModelError::Conflict(
            "installed canonical graph backing projection",
        ));
    }
    Ok(CanonicalBackingEndpoints {
        endpoints,
        original: None,
        reservation: decode,
    })
}

pub fn validate_state_row(table: &str, row: &Value) -> Result<Option<OriginalBacking>, ModelError> {
    let object = value_object(row).ok_or(ModelError::Schema("completed state object"))?;
    let Some(Value::RecordId(id)) = object.get("id") else {
        return Err(ModelError::Schema("completed state key"));
    };
    if id.table.as_str() != table {
        return Err(ModelError::Schema("completed state table/key"));
    }
    use d::recovery_closure::RecoveryFamily::*;
    let RecoveryTableKind::Content(family) = classify_logical_recovery_table(table)
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
                        let producing = attempt_contribution(
                            attempt,
                            descriptor.spec.identity().expect("validated specification"),
                        );
                        let detached = attempt_contribution(
                            attempt,
                            descriptor.identity().expect("validated completion"),
                        );
                        if let Some(Value::String(copy)) = object.get("transport_copy") {
                            let Ok(bytes) = hex::decode(copy) else {
                                return true;
                            };
                            let Ok(bytes) = <[u8; 32]>::try_from(bytes) else {
                                return true;
                            };
                            return id.key
                                != RecordIdKey::String(
                                    attempt_contribution(attempt, ContentHash(bytes)).hex(),
                                );
                        }
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
    crate::loader::validate_native_object(row)?;
    let body_value = row
        .get("body")
        .ok_or(ModelError::Schema("compiler backing body"))?;
    let Value::Object(body) = body_value else {
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
                != Some(&Value::RecordId(crate::originals::layout_id(
                    d::graph::EntityId::of(artifact),
                )?))
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
    let Some(Value::Bytes(canonical)) = row.get("canonical") else {
        return Err(ModelError::Schema("compiler backing canonical bytes"));
    };
    // Preserve the complete generic native Value decoding grammar. In particular,
    // serde encodes nonfinite floats as null, which must not become a valid forged
    // canonical body (including inside sets, ranges, record keys or geometry).
    // This single row-local decode replaces the former multiple full-body copies.
    let canonical_body: Value = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
    if canonical_body != body_value {
        return Err(ModelError::Conflict("compiler backing canonical body"));
    }
    // Compare the exact canonical encoding against the borrowed native body.
    // The one row-local parser above preserves generic native grammar. Streaming
    // the byte comparison avoids a second canonical buffer and full-row cloning.
    if !canonical_matches(body_value, canonical)? {
        return Err(ModelError::Conflict("compiler backing canonical body"));
    }
    let content = ContentHash::of(canonical);
    if row.get("semantic_type") != Some(&Value::String(name.clone()))
        || row.get("semantic_key") != Some(&Value::String(hex::encode(key)))
        || *id != crate::loader::payload_id("compiler_record", name, &key, content)?
    {
        return Err(ModelError::Conflict("compiler backing typed identity"));
    }
    if row.get("content") != Some(&Value::String(content.hex())) {
        return Err(ModelError::Conflict("compiler backing content"));
    }
    let adapter = crate::adapter::select(name)?;
    if adapter.table != crate::schema::ScopeTable::CompilerRecord {
        return Err(ModelError::Schema("undeclared compiler backing type"));
    }
    let mut expected = adapter.scopes(body_value)?;
    expected.insert("id", id.clone());
    expected.insert("semantic_type", name.clone());
    expected.insert("semantic_key", hex::encode(key));
    expected.insert("content", content.hex());
    // Body/canonical have already been compared exactly. Every other field is
    // independently re-derived, and the count rejects extra physical columns.
    if row.len() != expected.len() + 2
        || expected
            .iter()
            .any(|(field, value)| row.get(field) != Some(value))
    {
        return Err(ModelError::Conflict("compiler backing complete envelope"));
    }
    Ok(original)
}

fn canonical_matches(value: &Value, canonical: &[u8]) -> Result<bool, ModelError> {
    struct CanonicalComparison<'a> {
        expected: &'a [u8],
        position: usize,
        equal: bool,
    }
    impl std::io::Write for CanonicalComparison<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let end = self
                .position
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("compiler canonical comparison overflow"))?;
            self.equal &= self.expected.get(self.position..end) == Some(bytes);
            self.position = end;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut compare = CanonicalComparison {
        expected: canonical,
        position: 0,
        equal: true,
    };
    serde_json::to_writer(&mut compare, value).map_err(ModelError::codec)?;
    Ok(compare.equal && compare.position == canonical.len())
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

/// Logical contribution digests reconstructed from actual canonical backing rows.
///
/// The caller independently authenticates membership, physical locators, canonical
/// envelopes and complete EOF. This owner only rebuilds the declared typed rows and
/// computes their model-owned logical content; physical payload hashes stay separate.
/// One bounded typed builder is reused across rows and pages of the same relation.
pub struct LogicalOutputDigests {
    outputs: BTreeMap<String, d::RelationContent>,
    active: Option<(Relation, crate::projected_arrow::ProjectedBuilder)>,
    budget: ResourceBudget,
    _charge: d::charged::StateCharge,
}
impl LogicalOutputDigests {
    pub fn new<'a>(
        relations: impl IntoIterator<Item = &'a str>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge =
            d::charged::StateCharge::new(budget, "logical contribution output digests");
        let mut outputs = BTreeMap::new();
        for name in relations {
            let relation = &crate::adapter::select(name)?.relation;
            charge.grow(
                name.len()
                    .saturating_add(size_of::<d::RelationContent>())
                    .saturating_add(256),
            )?;
            if outputs
                .insert(name.to_owned(), relation.content())
                .is_some()
            {
                return Err(ModelError::Conflict("duplicate logical output relation"));
            }
        }
        Ok(Self {
            outputs,
            active: None,
            budget: budget.clone(),
            _charge: charge,
        })
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((relation, builder)) = self.active.as_mut() {
            if builder.rows() == 0 {
                return Ok(());
            }
            let batch = builder.finish()?;
            let _decode = self.budget.reserve(
                "logical contribution typed digest decode",
                d::logical_batch_bytes(&batch)?
                    .saturating_mul(8)
                    .saturating_add(batch.num_rows().saturating_mul(256))
                    .saturating_add(4096),
            )?;
            let output = self
                .outputs
                .get_mut(relation.name())
                .ok_or(ModelError::Conflict("undeclared logical output relation"))?;
            relation.hash_rows(&batch, output)?;
            drop(batch);
            builder.release()?;
        }
        Ok(())
    }
    pub async fn push<Context>(
        &mut self,
        reader: &crate::NativeReader<Context>,
        row: &Value,
    ) -> Result<(), ModelError> {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("logical output backing object"))?;
        let Some(Value::String(name)) = object.get("semantic_type") else {
            return Err(ModelError::Schema("logical output backing relation"));
        };
        if !self.outputs.contains_key(name) {
            return Err(ModelError::Conflict("undeclared logical output membership"));
        }
        if self
            .active
            .as_ref()
            .is_none_or(|(relation, _)| relation.name() != name)
        {
            self.flush()?;
            self.active.take();
            let relation = crate::adapter::select(name)?.relation.clone();
            let builder = crate::projected_arrow::ProjectedBuilder::new(
                relation.clone(),
                relation.schema().clone(),
                &self.budget,
            )?;
            self.active = Some((relation, builder));
        }
        let Some(Value::Object(body)) = object.get("body") else {
            return Err(ModelError::Schema("logical output declared body"));
        };
        let _fields = self.budget.reserve(
            "logical output projected fields",
            crate::loader::native_bytes(row)
                .saturating_mul(2)
                .saturating_add(4096),
        )?;
        let mut fields = body.clone();
        fields.insert(
            "id",
            object
                .get("semantic_key")
                .ok_or(ModelError::Schema("logical output nominal key"))?
                .clone(),
        );
        let _original;
        if name == d::artifact::ArtifactChunk::NAME {
            let backing = validate_state_row("compiler_record", row)?
                .ok_or(ModelError::Schema("logical output original chunk backing"))?;
            _original = Some(self.budget.reserve(
                "logical output original chunk bytes",
                backing.len.saturating_add(256),
            )?);
            let mut bytes = Vec::with_capacity(backing.len);
            let source = crate::originals::layout_id(d::graph::EntityId::of(backing.artifact))?;
            // OriginalRanges intentionally bounds each physical union to 256 KiB;
            // one declared logical ArtifactChunk can be 1 MiB. Reassemble that
            // actual model unit without widening the shared range-read contract.
            while bytes.len() < backing.len {
                let length = (backing.len - bytes.len()).min(256 * 1024);
                let start = backing
                    .start
                    .checked_add(bytes.len() as u64)
                    .ok_or(ModelError::Schema("logical output original range overflow"))?;
                let mut ranges = crate::originals::read_ranges(
                    reader,
                    &[(source.clone(), start, length)],
                    &self.budget,
                )
                .await?;
                let value = ranges
                    .pop()
                    .ok_or(ModelError::Conflict("logical output original range absent"))?;
                if value.len() != length || !ranges.is_empty() {
                    return Err(ModelError::Conflict("logical output exact original range"));
                }
                bytes.extend_from_slice(value.as_ref());
            }
            if ContentHash::of(&bytes) != backing.digest {
                return Err(ModelError::Conflict("logical output original chunk digest"));
            }
            fields.insert("body", Value::Bytes(surrealdb::types::Bytes::from(bytes)));
        } else {
            _original = None;
        }
        let (_, builder) = self
            .active
            .as_mut()
            .ok_or(ModelError::Conflict("logical output active builder"))?;
        builder.push(Value::Object(fields))?;
        if builder.rows() >= d::resources::TRANSFER_ROWS
            || builder.bytes() >= d::resources::TRANSFER_BYTES
        {
            self.flush()?;
        }
        Ok(())
    }
    pub fn finish(mut self) -> Result<BTreeMap<String, OutputContent>, ModelError> {
        self.flush()?;
        Ok(self
            .outputs
            .into_iter()
            .map(|(name, content)| {
                let (rows, content) = content.finish();
                (name, OutputContent { rows, content })
            })
            .collect())
    }
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
            budget,
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
            let Some(Value::Bytes(canonical)) = row.get("canonical") else {
                return Err(ModelError::Schema("compiler backing canonical bytes"));
            };
            let actual_body = row
                .get("body")
                .ok_or(ModelError::Schema("compiler backing body"))?;
            // Both actual and independently reconstructed typed bodies must emit
            // these exact bytes. Numeric canonicalization (including negative
            // zero) stays an independent model check, without full-row clones.
            if !canonical_matches(actual_body, canonical)? || !canonical_matches(&body, canonical)?
            {
                return Err(ModelError::Conflict("compiler backing declared body"));
            }
            crate::loader::validate_native_object(row)?;
            let content = ContentHash::of(canonical);
            let mut expected = adapter.scopes(&body)?;
            expected.insert(
                "id",
                crate::loader::payload_id("compiler_record", name, ids.value(index), content)?,
            );
            expected.insert("semantic_type", name.to_owned());
            expected.insert("semantic_key", key);
            expected.insert("content", content.hex());
            if row.len() != expected.len() + 2
                || expected
                    .iter()
                    .any(|(field, value)| row.get(field) != Some(value))
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

#[cfg(test)]
mod logical_output_controls {
    use super::*;
    use std::sync::Arc;

    fn package_backing(package: &d::input::Package) -> Value {
        let entity = d::graph::Entity::from(package.clone());
        let view = crate::codec::entity_views(std::slice::from_ref(&entity))
            .unwrap()
            .pop()
            .unwrap();
        crate::adapter::physical_row_prepared(
            crate::loader::entity_payload_id(&entity).unwrap(),
            entity.content(),
            serde_json::to_vec(&entity).unwrap(),
            Some(crate::adapter::GraphEnvelope::entity(&entity)),
            view,
        )
        .unwrap()
    }
    #[tokio::test]
    async fn logical_output_content_matches_typed_rows_across_flush_and_empty_domains() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        // This control exercises effect-free typed reconstruction. The reader is
        // deliberately disconnected; graph rows require no native lookups here.
        let reader =
            crate::NativeReader::private(Arc::new(surrealdb::Surreal::init())).with_budget(&budget);
        let mut packages = [
            d::input::Package { name: "one".into() },
            d::input::Package { name: "two".into() },
            d::input::Package {
                name: "three".into(),
            },
        ];
        packages.sort_by_key(Record::id);
        let relation = Relation::of::<d::input::Package>();
        let empty = Relation::of::<d::input::Release>();
        let mut expected = relation.content();
        relation
            .hash_rows(
                &d::input::Package::encode(&packages).unwrap(),
                &mut expected,
            )
            .unwrap();
        let mut outputs =
            LogicalOutputDigests::new([relation.name(), empty.name()], &budget).unwrap();
        for package in &packages {
            let row = package_backing(package);
            let _physical = canonical_backing_endpoints(&row, &budget).unwrap();
            outputs.push(&reader, &row).await.unwrap();
            outputs.flush().unwrap();
        }
        let actual = outputs.finish().unwrap();
        let (rows, content) = expected.finish();
        assert_eq!(actual[relation.name()], OutputContent { rows, content });
        let (rows, content) = empty.content().finish();
        assert_eq!(actual[empty.name()], OutputContent { rows, content });
        assert_eq!(
            budget.reserved(),
            0,
            "typed reconstruction has no retained working state after EOF"
        );
    }
    #[tokio::test]
    async fn logical_output_reconstruction_refuses_duplicate_and_late_invalid_rows() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let reader =
            crate::NativeReader::private(Arc::new(surrealdb::Surreal::init())).with_budget(&budget);
        let package = d::input::Package { name: "one".into() };
        let row = package_backing(&package);
        let mut duplicate = LogicalOutputDigests::new([d::input::Package::NAME], &budget).unwrap();
        duplicate.push(&reader, &row).await.unwrap();
        duplicate.flush().unwrap();
        duplicate.push(&reader, &row).await.unwrap();
        assert!(
            duplicate.finish().is_err(),
            "a repeated nominal row cannot be hidden by page boundaries"
        );
        let mut invalid = LogicalOutputDigests::new([d::input::Package::NAME], &budget).unwrap();
        invalid.push(&reader, &row).await.unwrap();
        invalid.flush().unwrap();
        let Value::Object(mut forged) = row else {
            unreachable!()
        };
        let Some(Value::Object(body)) = forged.get_mut("body") else {
            unreachable!()
        };
        body.insert("name", "invalid--name");
        invalid.push(&reader, &Value::Object(forged)).await.unwrap();
        assert!(
            invalid.finish().is_err(),
            "an earlier valid prefix does not grant later row validity"
        );
        assert_eq!(budget.reserved(), 0);
    }
}
