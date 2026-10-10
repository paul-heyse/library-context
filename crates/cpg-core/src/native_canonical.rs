//! Bounded native canonical graph reads and compact admission keys.
use crate::workspace::Workspace;
use d::resources::Reservation;
use futures::stream::BoxStream;
use lctx_model::domain::{self as d, ModelError, Record, charged::StateCharge, graph::*};
use lctx_surrealdb::{
    compiler::CompilerRows,
    surrealdb::types::{Number, Object, RecordId, Value},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) fn object(value: Value) -> Result<Object, ModelError> {
    if let Value::Object(value) = value {
        Ok(value)
    } else {
        Err(ModelError::Schema("native canonical object"))
    }
}
fn hash(value: &str) -> Result<d::ContentHash, ModelError> {
    let bytes = hex::decode(value).map_err(ModelError::codec)?;
    Ok(d::ContentHash(bytes.try_into().map_err(|_| {
        ModelError::Schema("native graph hash width")
    })?))
}
fn integer(value: Option<&Value>) -> Result<Option<i64>, ModelError> {
    match value {
        Some(Value::Number(Number::Int(value))) => Ok(Some(*value)),
        None | Some(Value::None | Value::Null) => Ok(None),
        _ => Err(ModelError::Schema("native graph integer")),
    }
}
fn payload(row: &Object) -> Result<&[u8], ModelError> {
    match row.get("canonical") {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        _ => Err(ModelError::Schema("native canonical bytes")),
    }
}
fn verify(
    row: &Object,
    id: RecordId,
    anchor: RecordId,
    view: &lctx_surrealdb::codec::RecordView,
    content: d::ContentHash,
    kind: i64,
    subtype: Option<i16>,
) -> Result<(), ModelError> {
    if row.get("id") != Some(&Value::RecordId(id))
        || row.get("anchor") != Some(&Value::RecordId(anchor))
        || row.get("semantic_type") != Some(&Value::String(view.semantic_type.clone()))
        || row.get("semantic_key") != Some(&Value::String(view.semantic_key.clone()))
        || row.get("content") != Some(&Value::String(content.hex()))
        || integer(row.get("kind"))? != Some(kind)
        || integer(row.get("subtype"))? != subtype.map(i64::from)
    {
        return Err(ModelError::Conflict("native canonical graph metadata"));
    }
    Ok(())
}
fn entity(row: &Object) -> Result<Entity, ModelError> {
    let entity: Entity = serde_json::from_slice(payload(row)?).map_err(ModelError::codec)?;
    let views = lctx_surrealdb::codec::entity_views(std::slice::from_ref(&entity))?;
    verify(
        row,
        lctx_surrealdb::loader::entity_payload_id(&entity)?,
        RecordId::new("entity_anchor", entity.id().0.hex()),
        &views[0],
        entity.content(),
        i64::from(entity.kind() as u16),
        entity.subtype(),
    )?;
    entity.validate()?;
    Ok(entity)
}
fn assertion(row: &Object) -> Result<Assertion, ModelError> {
    let assertion: Assertion = serde_json::from_slice(payload(row)?).map_err(ModelError::codec)?;
    let views = lctx_surrealdb::codec::assertion_views(std::slice::from_ref(&assertion))?;
    verify(
        row,
        lctx_surrealdb::loader::assertion_payload_id(&assertion)?,
        RecordId::new("assertion_anchor", assertion.id().0.hex()),
        &views[0],
        assertion.content(),
        i64::from(assertion.kind as u16),
        None,
    )?;
    assertion.validate()?;
    Ok(assertion)
}
fn text<'a>(row: &'a Object, field: &str) -> Result<&'a str, ModelError> {
    match row.get(field) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(ModelError::Schema("native graph header string")),
    }
}
fn typed_key<R: Record>(key: &str) -> Result<d::Id<R>, ModelError> {
    let bytes: [u8; 16] = hex::decode(key)
        .map_err(ModelError::codec)?
        .try_into()
        .map_err(|_| ModelError::Schema("native typed graph key width"))?;
    if hex::encode(bytes) != key {
        return Err(ModelError::Conflict("native typed graph key encoding"));
    }
    serde_json::from_value(serde_json::to_value(bytes).map_err(ModelError::codec)?)
        .map_err(ModelError::codec)
}
macro_rules! entity_headers {($($variant:ident:$ty:ty,)*)=>{fn entity_header_key(name:&str,key:&str)->Result<(d::ContentHash,i64),ModelError>{match name{$(<$ty>::NAME=>Ok((EntityId::of(typed_key::<$ty>(key)?).0,i64::from(<$ty as GraphEntityRecord>::GRAPH_KIND as u16))),)*_=>Err(ModelError::Schema("native entity header declaration"))}}};}
lctx_model::graph_entity_records!(entity_headers);
macro_rules! assertion_headers {($($variant:ident:$ty:ty,)*)=>{fn assertion_header_key(name:&str,key:&str)->Result<(d::ContentHash,Option<i64>),ModelError>{match name{$(<$ty>::NAME=>Ok((AssertionId::of(typed_key::<$ty>(key)?).0,Some(i64::from(<$ty as GraphAssertionRecord>::GRAPH_KIND as u16)))),)*"__graph_assertion"=>Ok((hash(key)?,None)),_=>Err(ModelError::Schema("native assertion header declaration"))}}};}
lctx_model::graph_assertion_records!(assertion_headers);
fn header_identity(row: &Object, entities: bool) -> Result<d::ContentHash, ModelError> {
    let name = text(row, "semantic_type")?;
    let key = text(row, "semantic_key")?;
    let content_text = text(row, "content")?;
    let content = hash(content_text)?;
    let (nominal, kind) = if entities {
        let (key, kind) = entity_header_key(name, key)?;
        (key, Some(kind))
    } else {
        assertion_header_key(name, key)?
    };
    let table = if entities { "entity" } else { "assertion" };
    let actual_kind = integer(row.get("kind"))?;
    if content.hex() != content_text
        || (name == "__graph_assertion" && nominal.hex() != key)
        || row.get("id")
            != Some(&Value::RecordId(lctx_surrealdb::loader::payload_id(
                table, name, &nominal.0, content,
            )?))
        || row.get("anchor")
            != Some(&Value::RecordId(RecordId::new(
                format!("{table}_anchor"),
                nominal.hex(),
            )))
        || actual_kind.is_none()
        || kind.is_some_and(|kind| actual_kind != Some(kind))
        || (!entities && integer(row.get("subtype"))?.is_some())
    {
        return Err(ModelError::Conflict("native graph header identity"));
    }
    Ok(nominal)
}
fn stream<R: Send + 'static>(
    rows: CompilerRows,
    workspace: Arc<Workspace>,
    decode: fn(&Object) -> Result<R, ModelError>,
) -> BoxStream<'static, Result<R, ModelError>> {
    Box::pin(futures::stream::try_unfold(
        (rows, workspace, None::<Box<dyn Reservation>>),
        move |(mut rows, workspace, charge)| async move {
            drop(charge);
            workspace.cancellation().check()?;
            let Some(row) = rows.next().await? else {
                return Ok(None);
            };
            let row = object(row)?;
            let charge = workspace.budget().reserve(
                "native-canonical-decode",
                payload(&row)?.len().saturating_mul(4).saturating_add(512),
            )?;
            let record = decode(&row)?;
            Ok(Some((record, (rows, workspace, Some(charge)))))
        },
    ))
}
pub async fn entities(
    workspace: Arc<Workspace>,
) -> Result<BoxStream<'static, Result<Entity, ModelError>>, ModelError> {
    Ok(stream(
        workspace
            .native()
            .scan_canonical(true, workspace.budget())
            .await?,
        workspace,
        entity,
    ))
}
pub async fn assertions(
    workspace: Arc<Workspace>,
) -> Result<BoxStream<'static, Result<Assertion, ModelError>>, ModelError> {
    Ok(stream(
        workspace
            .native()
            .scan_canonical(false, workspace.budget())
            .await?,
        workspace,
        assertion,
    ))
}
macro_rules! kinds {($unused:ident;$($variant:ident:$kind:ident=>$ty:ty,)*)=>{
    fn native_kind(value:i64)->Result<EntityKind,ModelError>{[$(EntityKind::$kind,)*].into_iter().find(|kind|i64::from(*kind as u16)==value).ok_or(ModelError::Schema("native graph entity kind"))}
};}
lctx_model::graph_entity_declarations!(kinds, unused);
struct Header {
    kind: EntityKind,
    subtype: Option<i16>,
    length: Option<u64>,
}
pub(crate) struct Lookup {
    entities: BTreeMap<EntityId, Header>,
    assertions: BTreeSet<AssertionId>,
    _charge: StateCharge,
}
impl Lookup {
    pub(crate) async fn load(workspace: &Arc<Workspace>) -> Result<Self, ModelError> {
        let mut lookup = Self {
            entities: BTreeMap::new(),
            assertions: BTreeSet::new(),
            _charge: StateCharge::new(workspace.budget(), "native-final-graph-keys"),
        };
        for entities in [true, false] {
            let mut rows = workspace
                .native()
                .scan_graph_headers(entities, workspace.budget())
                .await?;
            while let Some(row) = rows.next().await? {
                workspace.cancellation().check()?;
                let row = object(row)?;
                let key = header_identity(&row, entities)?;
                lookup._charge.grow(192)?;
                if entities {
                    let kind = native_kind(
                        integer(row.get("kind"))?.ok_or(ModelError::Schema("native graph kind"))?,
                    )?;
                    let subtype = integer(row.get("subtype"))?
                        .map(i16::try_from)
                        .transpose()
                        .map_err(ModelError::codec)?;
                    let length = if kind == EntityKind::Source {
                        integer(row.get("source_length"))?
                            .map(u64::try_from)
                            .transpose()
                            .map_err(ModelError::codec)?
                    } else {
                        None
                    };
                    if lookup
                        .entities
                        .insert(
                            EntityId(key),
                            Header {
                                kind,
                                subtype,
                                length,
                            },
                        )
                        .is_some()
                    {
                        return Err(ModelError::Conflict("duplicate native graph header"));
                    }
                } else if !lookup.assertions.insert(AssertionId(key)) {
                    return Err(ModelError::Conflict("duplicate native assertion header"));
                }
            }
        }
        Ok(lookup)
    }
}
impl GraphLookup for Lookup {
    fn entity_kind(&self, id: EntityId) -> Result<Option<EntityKind>, ModelError> {
        Ok(self.entities.get(&id).map(|row| row.kind))
    }
    fn entity_subtype(&self, id: EntityId) -> Result<Option<i16>, ModelError> {
        Ok(self.entities.get(&id).and_then(|row| row.subtype))
    }
    fn assertion_exists(&self, id: AssertionId) -> Result<bool, ModelError> {
        Ok(self.assertions.contains(&id))
    }
    fn source_length(&self, id: EntityId) -> Result<Option<u64>, ModelError> {
        Ok(self.entities.get(&id).and_then(|row| row.length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn envelope(
        id: RecordId,
        anchor: RecordId,
        view: lctx_surrealdb::codec::RecordView,
        content: d::ContentHash,
        canonical: Vec<u8>,
        kind: i64,
        subtype: Option<i16>,
    ) -> Object {
        let mut row = Object::new();
        row.insert("id", id);
        row.insert("anchor", anchor);
        row.insert("semantic_type", view.semantic_type);
        row.insert("semantic_key", view.semantic_key);
        row.insert("content", content.hex());
        row.insert(
            "canonical",
            lctx_surrealdb::surrealdb::types::Bytes::from(canonical),
        );
        row.insert("kind", kind);
        row.insert("subtype", subtype.map(Value::from_t).unwrap_or(Value::Null));
        row
    }
    #[test]
    fn canonical_payload_addresses_preserve_nominal_headers_and_refuse_tampering() {
        let value = Entity::from(d::input::Package {
            name: "canonical-control".into(),
        });
        let view = lctx_surrealdb::codec::entity_views(std::slice::from_ref(&value))
            .unwrap()
            .remove(0);
        let row = envelope(
            lctx_surrealdb::loader::entity_payload_id(&value).unwrap(),
            RecordId::new("entity_anchor", value.id().0.hex()),
            view,
            value.content(),
            serde_json::to_vec(&value).unwrap(),
            i64::from(value.kind() as u16),
            value.subtype(),
        );
        assert_ne!(
            row.get("id"),
            Some(&Value::RecordId(RecordId::new(
                "entity",
                value.id().0.hex()
            )))
        );
        assert_eq!(entity(&row).unwrap(), value);
        assert_eq!(header_identity(&row, true).unwrap(), value.id().0);
        for (field, replacement) in [
            (
                "id",
                Value::RecordId(RecordId::new(
                    "entity",
                    d::ContentHash::of(b"foreign payload").hex(),
                )),
            ),
            (
                "anchor",
                Value::RecordId(RecordId::new(
                    "entity_anchor",
                    d::ContentHash::of(b"foreign anchor").hex(),
                )),
            ),
            ("semantic_type", Value::String("releases".into())),
            ("semantic_key", Value::String(hex::encode([7u8; 16]))),
            (
                "content",
                Value::String(d::ContentHash::of(b"foreign content").hex()),
            ),
            ("kind", Value::from_t(999i64)),
        ] {
            let mut tampered = row.clone();
            tampered.insert(field, replacement);
            assert!(entity(&tampered).is_err(), "canonical {field}");
            assert!(header_identity(&tampered, true).is_err(), "header {field}");
        }
        let package = d::input::Package {
            name: "canonical-control".into(),
        };
        let release = d::input::Release {
            package: package.id(),
            version: "1".into(),
        };
        let input = d::input::InputRevision {
            manifest: d::ContentHash::of(b"input"),
        };
        let typed = Assertion::from_record(d::input::InputDistribution {
            input: input.id(),
            release: release.id(),
            role: d::input::DistributionRole::FirstParty,
        })
        .unwrap();
        let detached = Assertion {
            source: None,
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                field: None,
                position: None,
                target: Target::Entity(value.id()),
            }],
            qualification: Qualification::Ref(value.id()),
            run: None,
            evidence: vec![],
            value: AssertionValue::None,
            derivation: None,
        };
        for value in [typed, detached] {
            let view = lctx_surrealdb::codec::assertion_views(std::slice::from_ref(&value))
                .unwrap()
                .remove(0);
            let row = envelope(
                lctx_surrealdb::loader::assertion_payload_id(&value).unwrap(),
                RecordId::new("assertion_anchor", value.id().0.hex()),
                view,
                value.content(),
                serde_json::to_vec(&value).unwrap(),
                i64::from(value.kind as u16),
                None,
            );
            assert_eq!(assertion(&row).unwrap(), value);
            assert_eq!(header_identity(&row, false).unwrap(), value.id().0);
            let mut tampered = row.clone();
            tampered.insert("id", RecordId::new("assertion", value.id().0.hex()));
            assert!(assertion(&tampered).is_err());
            assert!(header_identity(&tampered, false).is_err());
        }
    }
}
