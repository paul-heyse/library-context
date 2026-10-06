//! Coarse publication passes build shared native documents/vectors and contextual adjacency.
use lctx_model::domain::{
    embedding::{EmbeddingSpec, analytic::VectorAvailability, value},
    graph::{EntityId, Target},
    retrieval::{
        Family, Fragment, OriginalAnchor, Subject, Unit, UnitSubject,
        consumption::RetrievalEmbeddingUse,
    },
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    *,
};
use lctx_surrealdb::surrealdb::types::{Bytes, Object, RecordId, Value, Variables};
use lctx_surrealdb::{Loader, NativeReader, RecordSelection, reader::target_id};
use std::collections::{BTreeMap, BTreeSet};
fn ids<R: Record>(
    values: impl Iterator<Item = Id<R>>,
) -> Result<Vec<serde_json::Value>, ModelError> {
    values
        .map(|id| serde_json::to_value(id).map_err(ModelError::codec))
        .collect()
}
fn table(family: Family) -> &'static str {
    match family {
        Family::ApiOptions => "search_api_options",
        Family::DocumentationDeployment => "search_documentation_deployment",
        Family::Scenario => "search_scenario",
        Family::Source => "search_source",
    }
}
pub async fn materialize_search(loader: &Loader) -> Result<(), ModelError> {
    let snapshot = SnapshotHandle {
        semantic: ContentHash::of(b"private-loading"),
        realization: ContentHash::of(b"private-loading"),
        database: DatabaseIdentity {
            namespace: Name::new("private").map_err(ModelError::codec)?,
            database: Name::new("private").map_err(ModelError::codec)?,
        },
    };
    // The private loader's already-selected session is deliberately shared through Arc, not SDK clone.
    let reader = NativeReader::new(loader.shared_client(), snapshot);
    let mut after = String::new();
    let mut expected: BTreeMap<String, BTreeSet<RecordId>> = BTreeMap::new();
    loop {
        let mut bindings = Variables::new();
        bindings.insert("after", after.clone());
        bindings.insert("unit_type", Unit::NAME.to_string());
        let keys:Vec<String>=reader.query("SELECT VALUE semantic_key FROM entity WHERE semantic_type=$unit_type AND semantic_key>$after ORDER BY semantic_key LIMIT 128",bindings).await?;
        if keys.is_empty() {
            break;
        }
        after = keys.last().ok_or(ModelError::Schema("unit page"))?.clone();
        let keys = keys
            .into_iter()
            .map(|s| {
                hex::decode(s)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("unit key"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let units = reader.records::<Unit>(RecordSelection::Keys(keys)).await?;
        let fragments = reader
            .records::<Fragment>(RecordSelection::Scope {
                field: "corpus".into(),
                values: ids(units.iter().map(|r| r.corpus))?,
            })
            .await?;
        let links = reader
            .records::<UnitSubject>(RecordSelection::Scope {
                field: "unit".into(),
                values: ids(units.iter().map(Record::id))?,
            })
            .await?;
        let subjects: BTreeMap<_, _> = reader
            .records::<Subject>(RecordSelection::Keys(
                links.iter().map(|r| *r.subject.bytes()).collect(),
            ))
            .await?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let anchors = reader
            .records::<OriginalAnchor>(RecordSelection::Scope {
                field: "unit".into(),
                values: ids(units.iter().map(Record::id))?,
            })
            .await?;
        let uses = reader
            .records::<RetrievalEmbeddingUse>(RecordSelection::Scope {
                field: "fragment".into(),
                values: ids(fragments.iter().map(Record::id))?,
            })
            .await?;
        let specs: BTreeMap<_, _> = reader
            .records::<EmbeddingSpec>(RecordSelection::Keys(
                uses.iter().map(|r| *r.specification.bytes()).collect(),
            ))
            .await?
            .into_iter()
            .map(|r| (r.id(), r))
            .collect();
        let mut documents: BTreeMap<&str, Vec<Value>> = BTreeMap::new();
        let mut vectors = Vec::new();
        let mut lexical = Vec::new();
        let mut vector_occurrences = Vec::new();
        for unit in &units {
            let mut members = links
                .iter()
                .filter(|link| link.unit == unit.id())
                .filter_map(|link| match subjects.get(&link.subject) {
                    Some(Subject::Member { member }) => Some(Some(*member)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            members.sort();
            members.dedup();
            if members.is_empty() {
                members.push(None)
            }
            let mut unit_anchors = anchors
                .iter()
                .filter(|r| r.unit == unit.id())
                .map(|r| Some(r.id()))
                .collect::<Vec<_>>();
            unit_anchors.sort();
            if unit_anchors.is_empty() {
                unit_anchors.push(None)
            }
            for fragment in fragments.iter().filter(|f| f.corpus == unit.corpus) {
                let document = RecordId::new(table(unit.family), fragment.digest.hex());
                let mut doc = Object::new();
                doc.insert("id", document.clone());
                doc.insert("text", fragment.text.as_str().to_string());
                doc.insert("digest", crate_json(fragment.digest)?);
                documents
                    .entry(table(unit.family))
                    .or_default()
                    .push(Value::Object(doc));
                for member in &members {
                    for anchor in &unit_anchors {
                        let mut identity = KeySink::new("native-search-occurrence/v1");
                        unit.id().encode(&mut identity);
                        fragment.id().encode(&mut identity);
                        member.encode(&mut identity);
                        anchor.encode(&mut identity);
                        unit.context.encode(&mut identity);
                        unit.family.encode(&mut identity);
                        let key = identity.finish().hex();
                        let out = member
                            .map(|m| target_id(Target::Entity(EntityId::of(m))))
                            .unwrap_or_else(|| target_id(Target::Entity(EntityId::of(unit.id()))));
                        let lexical_occurrence = occurrence(
                            "lex_occurs",
                            &key,
                            document.clone(),
                            out.clone(),
                            unit,
                            fragment,
                            *member,
                            *anchor,
                        )?;
                        lexical.push(lexical_occurrence);
                        for consumed in uses.iter().filter(|u| {
                            u.fragment == fragment.id()
                                && u.availability == VectorAvailability::Available
                        }) {
                            let spec = specs
                                .get(&consumed.specification)
                                .ok_or(ModelError::Schema("retrieval vector specification"))?
                                .configuration()?;
                            let bytes = consumed
                                .bytes
                                .as_ref()
                                .ok_or(ModelError::Schema("retrieval winning bytes"))?;
                            let vector = value::decode_vector(bytes.0.as_slice(), spec.dimensions)
                                .map_err(ModelError::Invalid)?;
                            lctx_model::domain::embedding::check_vector(&vector, spec.dimensions)
                                .map_err(ModelError::Invalid)?;
                            if value::value_digest(&vector)
                                != consumed
                                    .value_digest
                                    .ok_or(ModelError::Schema("retrieval value digest"))?
                            {
                                return Err(ModelError::Conflict("retrieval winning vector"));
                            }
                            let vector_id = RecordId::new(
                                "vector",
                                format!("{}_{}", spec.hash().hex(), consumed.input.hex()),
                            );
                            let mut row = Object::new();
                            row.insert("id", vector_id.clone());
                            row.insert("specification", crate_json(spec.hash())?);
                            row.insert("input", crate_json(consumed.input)?);
                            row.insert("digest", crate_json(value::value_digest(&vector))?);
                            row.insert("bytes", Bytes::from(bytes.0.as_slice().to_vec()));
                            row.insert("embedding", vector);
                            vectors.push(Value::Object(row));
                            let vector_key =
                                format!("{}_{}_{}", key, spec.hash().hex(), consumed.input.hex());
                            vector_occurrences.push(occurrence(
                                "vec_occurs",
                                &vector_key,
                                vector_id,
                                out.clone(),
                                unit,
                                fragment,
                                *member,
                                *anchor,
                            )?);
                        }
                    }
                }
            }
        }
        for (table, rows) in documents {
            insert(loader, table, rows, false, &mut expected).await?;
        }
        insert(loader, "vector", vectors, false, &mut expected).await?;
        insert(loader, "lex_occurs", lexical, true, &mut expected).await?;
        insert(
            loader,
            "vec_occurs",
            vector_occurrences,
            true,
            &mut expected,
        )
        .await?;
    }
    for table in [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
        "vector",
        "lex_occurs",
        "vec_occurs",
    ] {
        let count = lctx_surrealdb::reconciliation::table_count(loader, table).await?;
        if count != expected.get(table).map_or(0, |keys| keys.len()) as u64 {
            return Err(ModelError::Conflict("native search inventory"));
        }
        let required = if table.starts_with("search_") {
            vec!["exact_text", "lexical"]
        } else if table == "vector" {
            vec!["exact_value", "neighbor"]
        } else {
            vec!["occurrence", "document_occurrences", "target_occurrences"]
        };
        let mut response = loader
            .client()
            .query(format!("INFO FOR TABLE {table}"))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let info: Value = response.take(0).map_err(ModelError::codec)?;
        let Value::Object(info) = info else {
            return Err(ModelError::Schema("native search index information"));
        };
        let Some(Value::Object(indexes)) = info.get("indexes") else {
            return Err(ModelError::Schema("native search indexes"));
        };
        if required.iter().any(|name| !indexes.contains_key(*name)) {
            return Err(ModelError::Conflict("native search index readiness"));
        }
    }
    Ok(())
}
fn crate_json<T: serde::Serialize>(value: T) -> Result<Value, ModelError> {
    lctx_surrealdb::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}
#[allow(
    clippy::too_many_arguments,
    reason = "The lowering keeps physical endpoints and distinct semantic occurrence witnesses explicit"
)]
fn occurrence(
    table: &str,
    key: &str,
    input: RecordId,
    out: RecordId,
    unit: &Unit,
    fragment: &Fragment,
    member: Option<Id<catalog::CatalogMember>>,
    anchor: Option<Id<OriginalAnchor>>,
) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", RecordId::new(table, key));
    row.insert("in", input);
    row.insert("out", out.clone());
    row.insert("family", unit.family as i16);
    row.insert("unit", crate_json(unit.id())?);
    row.insert("fragment", crate_json(fragment.id())?);
    row.insert("context", crate_json(unit.context)?);
    row.insert("member", crate_json(member)?);
    row.insert("anchor", crate_json(anchor)?);
    row.insert("input", crate_json(unit.input)?);
    row.insert("eligible", true);
    row.insert(
        "occurrence_key",
        format!(
            "{}|{:02}|{}|{}|{}|{}",
            member
                .map(|id| format!("0{}", id.hex()))
                .unwrap_or_else(|| format!("1{}", unit.id().hex())),
            unit.family as i16,
            unit.id().hex(),
            fragment.id().hex(),
            unit.context.hex(),
            anchor
                .map(|id| format!("1{}", id.hex()))
                .unwrap_or_else(|| "0".into())
        ),
    );
    Ok(Value::Object(row))
}
#[allow(
    clippy::mutable_key_type,
    reason = "Owned search rows use immutable string RecordIds, never SDK regex keys"
)]
async fn insert(
    loader: &Loader,
    table: &str,
    rows: Vec<Value>,
    relation: bool,
    inventory: &mut BTreeMap<String, BTreeSet<RecordId>>,
) -> Result<(), ModelError> {
    use lctx_surrealdb::surrealdb::types::SurrealValue;
    for chunk in rows.chunks(128) {
        let mut expected = BTreeMap::new();
        for row in chunk {
            let Value::Object(object) = row else {
                return Err(ModelError::Schema("native search row"));
            };
            let id = RecordId::from_value(
                object
                    .get("id")
                    .ok_or(ModelError::Schema("native search identity"))?
                    .clone(),
            )
            .map_err(ModelError::codec)?;
            if expected
                .insert(id.clone(), row.clone())
                .is_some_and(|old| old != *row)
            {
                return Err(ModelError::Conflict("native search shared row"));
            }
            inventory.entry(table.into()).or_default().insert(id);
        }
        let mut bind = Variables::new();
        bind.insert("rows", expected.values().cloned().collect::<Vec<_>>());
        bind.insert("keys", expected.keys().cloned().collect::<Vec<_>>());
        let mut response=loader.client().query(format!("INSERT {}IGNORE INTO {table} $rows RETURN NONE; SELECT * FROM {table} WHERE id IN $keys ORDER BY id",if relation{"RELATION "}else{""})).bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let actual: Vec<Value> = response.take(1).map_err(ModelError::codec)?;
        let actual = actual
            .into_iter()
            .map(|row| match row {
                Value::Object(mut object) => {
                    object.retain(|key, _| !key.starts_with("scope_"));
                    Value::Object(object)
                }
                other => other,
            })
            .collect::<Vec<_>>();
        if actual != expected.into_values().collect::<Vec<_>>() {
            return Err(ModelError::Conflict(
                "native search content/eligibility readback",
            ));
        }
    }
    Ok(())
}
