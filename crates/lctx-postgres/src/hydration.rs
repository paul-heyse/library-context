//! Complete selected records. Query projection and evidence assembly have one Rust owner.
use crate::{
    Error,
    repository::{Hydration, Object, PinnedGeneration, check_response},
    serving::{QueryLease, ServingStore},
};
use cpg_schema::{id::Id, serving_projection::corrupt};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn packet<T: serde::de::DeserializeOwned + serde::Serialize>(
    value: Value,
) -> Result<Value, Error> {
    let typed: T =
        serde_json::from_value(value).map_err(|_| corrupt("hydrated packet contract"))?;
    let value = serde_json::to_value(typed).map_err(|_| corrupt("packet encoding"))?;
    check_response(&value)?;
    Ok(value)
}
pub(crate) fn id(text: &str) -> Result<Vec<u8>, Error> {
    Ok(Id::from_hex(text)
        .ok_or_else(|| Error::Request("invalid entity identity".into()))?
        .0
        .to_vec())
}
fn ids(rows: &[Object], key: &str) -> Result<Vec<Vec<u8>>, Error> {
    rows.iter()
        .filter_map(|r| r[key].as_str())
        .map(id)
        .collect()
}
fn keyed(rows: Vec<Object>, key: &str) -> BTreeMap<String, Object> {
    rows.into_iter()
        .map(|r| (r[key].as_str().expect("validated ID").to_owned(), r))
        .collect()
}
fn grouped(rows: Vec<Object>, key: &str) -> BTreeMap<String, Vec<Object>> {
    let mut out: BTreeMap<String, Vec<Object>> = BTreeMap::new();
    for row in rows {
        out.entry(row[key].as_str().expect("validated ID").to_owned())
            .or_default()
            .push(row);
    }
    out
}
fn group<'a>(rows: &'a BTreeMap<String, Vec<Object>>, id: &str) -> &'a [Object] {
    rows.get(id).map_or(&[], Vec::as_slice)
}
fn pick(row: &Object, fields: &[&str]) -> Object {
    fields
        .iter()
        .map(|f| ((*f).to_owned(), row[*f].clone()))
        .collect()
}

pub(crate) async fn singleton_class(
    conn: &mut sqlx::PgConnection,
    g: &PinnedGeneration,
    member: &Object,
) -> Result<Option<String>, Error> {
    if member["operation_node_id"].is_string() || !g.manifest.capabilities.behavioral_claims {
        return Ok(None);
    }
    let rows = sqlx::query_file!(
        "queries/packet_singleton.sql",
        g.id.0.as_slice(),
        member["access_path"].as_str()
    )
    .fetch_all(&mut *conn)
    .await?;
    if rows.len() > 1 {
        return Err(Error::Request("ambiguous singleton class".into()));
    }
    rows.into_iter()
        .next()
        .map(|r| {
            r.class_node_id
                .try_into()
                .map(|bytes| cpg_schema::Id(bytes).hex())
                .map_err(|_| corrupt("singleton class ID").into())
        })
        .transpose()
}
pub(crate) struct Invocation {
    bindings: Vec<Object>,
    constructors: Vec<Object>,
    pub signatures: Vec<Object>,
    subjects: Vec<Vec<u8>>,
}
pub(crate) async fn catalog_invocation(
    conn: &mut sqlx::PgConnection,
    generation: &PinnedGeneration,
    budget: &mut Hydration,
    member: &Object,
    singleton_class: Option<&str>,
) -> Result<Invocation, Error> {
    let members = vec![id(member["member_id"]
        .as_str()
        .ok_or_else(|| corrupt("catalog member identity"))?)?];
    let bindings = budget
        .fetch(
            conn,
            generation,
            "catalog_bindings",
            Some("member_id"),
            &members,
        )
        .await?;
    let mut declarations = ids(&bindings, "declaration_node_id")?;
    if let Some(node) = member["operation_node_id"].as_str() {
        declarations.push(id(node)?);
    }
    if let Some(node) = singleton_class {
        declarations.push(id(node)?);
    }
    declarations.sort();
    declarations.dedup();
    let constructors = budget
        .fetch(
            conn,
            generation,
            "catalog_constructors",
            Some("class_node_id"),
            &declarations,
        )
        .await?;
    let mut signatures = budget
        .fetch_contract(
            conn,
            generation,
            "catalog_signatures",
            Some("callable_node_id"),
            &declarations,
        )
        .await?;
    signatures.extend(
        budget
            .fetch_contract(
                conn,
                generation,
                "catalog_signatures",
                Some("signature_id"),
                &ids(&constructors, "signature_id")?,
            )
            .await?,
    );
    signatures.sort_by(|a, b| a["signature_id"].as_str().cmp(&b["signature_id"].as_str()));
    signatures.dedup_by(|a, b| a["signature_id"] == b["signature_id"]);
    let parameters = budget
        .fetch_contract(
            conn,
            generation,
            "catalog_parameters",
            Some("signature_id"),
            &ids(&signatures, "signature_id")?,
        )
        .await?;
    let mut subjects = declarations.clone();
    subjects.extend(ids(&signatures, "declaration_node_id")?);
    subjects.extend(ids(&signatures, "constructor_class_id")?);
    subjects.extend(ids(&parameters, "formal_node_id")?);
    subjects.sort();
    subjects.dedup();
    // Original bodies are now selected and expanded through typed evidence references.
    let params = grouped(parameters, "signature_id");
    for signature in &mut signatures {
        let mut parameters = group(
            &params,
            signature["signature_id"]
                .as_str()
                .ok_or_else(|| corrupt("signature identity"))?,
        )
        .to_vec();
        parameters.sort_by_key(|p| p["ordinal"].as_i64());
        signature.insert("parameters".into(), json!(parameters));
    }
    Ok(Invocation {
        bindings,
        constructors,
        signatures,
        subjects,
    })
}

pub(crate) async fn catalog_record(
    conn: &mut sqlx::PgConnection,
    generation: &PinnedGeneration,
    budget: &mut Hydration,
    member: &Object,
    singleton_class: Option<&str>,
) -> Result<Value, Error> {
    let Invocation {
        bindings,
        constructors,
        signatures,
        mut subjects,
    } = catalog_invocation(conn, generation, budget, member, singleton_class).await?;
    let surfaces = budget
        .fetch(
            conn,
            generation,
            "catalog_surfaces",
            Some("declaration_node_id"),
            &subjects,
        )
        .await?;
    let configurations = budget
        .fetch(
            conn,
            generation,
            "catalog_configurations",
            Some("class_node_id"),
            &subjects,
        )
        .await?;
    let field_links = budget
        .fetch(
            conn,
            generation,
            "catalog_field_links",
            Some("class_node_id"),
            &subjects,
        )
        .await?;
    subjects.extend(ids(&configurations, "field_id")?);
    let observations = budget
        .fetch(
            conn,
            generation,
            "catalog_type_observations",
            Some("subject_node_id"),
            &subjects,
        )
        .await?;
    let mut term_ids = ids(&observations, "term_id")?;
    term_ids.extend(ids(&configurations, "term_id")?);
    let mut visited = BTreeSet::new();
    let mut types = Vec::new();
    let mut type_args = Vec::new();
    while !term_ids.is_empty() {
        term_ids.retain(|id| visited.insert(id.clone()));
        if term_ids.is_empty() {
            break;
        }
        types.extend(
            budget
                .fetch(
                    conn,
                    generation,
                    "catalog_types",
                    Some("term_id"),
                    &term_ids,
                )
                .await?,
        );
        let args = budget
            .fetch(
                conn,
                generation,
                "catalog_type_args",
                Some("parent_term_id"),
                &term_ids,
            )
            .await?;
        term_ids = ids(&args, "child_term_id")?;
        type_args.extend(args);
    }
    Ok(
        json!({"member":member,"bindings":bindings,"constructors":constructors,"signatures":signatures,
        "type_observations":observations,"types":types,"type_arguments":type_args,"surfaces":surfaces,"configurations":configurations,"field_links":field_links,
        "effective_surface":"unresolved","basis":"source and attributed provider observations"}),
    )
}

impl ServingStore {
    pub async fn get_capability(
        &self,
        generation: &PinnedGeneration,
        snapshot: &str,
        capability: &str,
    ) -> Result<Value, Error> {
        generation.check_snapshot(snapshot)?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut budget = Hydration::new();
        let selected = vec![id(capability)?];
        let briefs = budget
            .fetch(
                &mut lease.connection,
                generation,
                "briefs",
                Some("brief_id"),
                &selected,
            )
            .await?;
        let brief = briefs
            .first()
            .ok_or_else(|| Error::Request("no capability in this generation".into()))?;
        let members = budget
            .fetch(
                &mut lease.connection,
                generation,
                "brief_members",
                Some("brief_id"),
                &selected,
            )
            .await?;
        let assertions = budget
            .fetch(
                &mut lease.connection,
                generation,
                "assertions",
                Some("brief_id"),
                &selected,
            )
            .await?;
        let supports = budget
            .fetch(
                &mut lease.connection,
                generation,
                "supports",
                Some("assertion_id"),
                &ids(&assertions, "assertion_id")?,
            )
            .await?;
        let finding_ids = ids(&supports, "finding_id")?;
        let findings = keyed(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "support_findings",
                    Some("finding_id"),
                    &finding_ids,
                )
                .await?,
            "finding_id",
        );
        let witnesses = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "support_witnesses",
                    Some("finding_id"),
                    &finding_ids,
                )
                .await?,
            "finding_id",
        );
        let finding_members = budget
            .fetch(
                &mut lease.connection,
                generation,
                "support_members",
                Some("finding_id"),
                &finding_ids,
            )
            .await?;
        let attributes = keyed(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "support_attributes",
                    Some("attribute_id"),
                    &ids(&finding_members, "attribute_id")?,
                )
                .await?,
            "attribute_id",
        );
        let finding_members = grouped(finding_members, "finding_id");
        let incidences = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "support_attribute_incidences",
                    Some("finding_id"),
                    &finding_ids,
                )
                .await?,
            "finding_id",
        );
        let evidence = keyed(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "evidence",
                    Some("evidence_id"),
                    &ids(&supports, "evidence_id")?,
                )
                .await?,
            "evidence_id",
        );
        let supports = grouped(supports, "assertion_id");
        let mut output = Vec::new();
        let mut cited = Vec::new();
        let mut sections = BTreeSet::new();
        budget.charge_response((brief, &members, &evidence))?;
        for a in &assertions {
            budget.charge_response(a)?;
            let mut record = pick(
                a,
                &[
                    "assertion_id",
                    "kind",
                    "section",
                    "text",
                    "applicable_case",
                    "conditions",
                    "limitations",
                ],
            );
            record.insert("status".into(), a["status"].clone());
            sections.insert(a["section"].as_str().expect("section"));
            let mut linked = Vec::new();
            for s in group(&supports, a["assertion_id"].as_str().expect("assertion ID")) {
                if let Some(e) = s["evidence_id"].as_str()
                    && !cited.contains(&e.to_owned())
                {
                    cited.push(e.to_owned());
                }
                budget.charge_response(s)?;
                let mut support = pick(s, &["role", "finding_id", "finding_kind", "evidence_id"]);
                let finding = if let Some(f) = s["finding_id"].as_str() {
                    let row = findings
                        .get(f)
                        .ok_or_else(|| corrupt("missing cited finding"))?;
                    budget.charge_response((
                        row,
                        group(&witnesses, f),
                        group(&finding_members, f),
                        group(&incidences, f),
                    ))?;
                    let mut result = pick(
                        row,
                        &[
                            "finding_id",
                            "evidence_status",
                            "subject_node_id",
                            "related_node_id",
                            "invocation_id",
                            "model_id",
                            "method",
                            "parameters",
                            "completion",
                            "stop_reason",
                            "witnesses_omitted",
                        ],
                    );
                    result.insert("kind".into(), row["finding_kind"].clone());
                    let w = group(&witnesses, f);
                    let m = group(&finding_members, f);
                    let i = group(&incidences, f);
                    let attribute_ids: BTreeSet<_> = m
                        .iter()
                        .filter_map(|r| r["attribute_id"].as_str())
                        .collect();
                    let attributes = attribute_ids
                        .into_iter()
                        .map(|id| {
                            attributes
                                .get(id)
                                .ok_or_else(|| corrupt("missing support attribute"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    budget.charge_response(&attributes)?;
                    let resolution = if !w.is_empty() {
                        "source_span"
                    } else if !i.is_empty() || m.iter().any(|r| !r["cited_fact_id"].is_null()) {
                        "fact_only"
                    } else {
                        "unavailable"
                    };
                    result.extend(json!({"witnesses":w,"members":m,"attributes":attributes,"attribute_incidences":i,"source_resolution":resolution}).as_object().expect("object").clone());
                    Value::Object(result)
                } else {
                    Value::Null
                };
                support.insert("finding".into(), finding);
                linked.push(support);
            }
            record.insert("supports".into(), json!(linked));
            output.push(record);
        }
        let evidence = cited
            .iter()
            .map(|id| {
                evidence
                    .get(id)
                    .cloned()
                    .ok_or_else(|| corrupt("missing cited evidence"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let absent: Vec<_> = generation
            .manifest
            .context
            .summary
            .slot_sections
            .iter()
            .filter(|s| !sections.contains(s.as_str()))
            .collect();
        let mut record = pick(
            brief,
            &[
                "title",
                "access_path",
                "documentation_only",
                "review_state",
                "outcome",
                "outcome_status",
            ],
        );
        let public_paths: Vec<_> = members
            .iter()
            .filter(|r| r["own"] == true)
            .map(|r| r["access_path"].clone())
            .collect();
        record.extend(json!({"library":generation.manifest.context.library,"snapshot_id":generation.manifest.snapshot_id,"generation":generation.id.hex(),"capability_id":capability,"public_paths":public_paths,"assertions":output,"evidence":evidence,"sections_absent":absent}).as_object().expect("object").clone());
        let result = Value::Object(record);
        check_response(&result)?;
        lease.complete();
        packet::<cpg_schema::wire::Capability>(result)
    }
    pub async fn hit_records(
        &self,
        generation: &PinnedGeneration,
        entities: &[String],
        operations: bool,
    ) -> Result<Value, Error> {
        if entities.len() > 50 {
            return Err(Error::Request("at most 50 selected hit identities".into()));
        }
        let ids = entities
            .iter()
            .map(|s| id(s))
            .collect::<Result<Vec<_>, _>>()?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut budget = Hydration::new();
        let (table, key) = if operations {
            ("operations", "node_id")
        } else {
            ("briefs", "brief_id")
        };
        let rows = keyed(
            budget
                .fetch(&mut lease.connection, generation, table, Some(key), &ids)
                .await?,
            key,
        );
        let result = Value::Array(
            entities
                .iter()
                .map(|id| {
                    rows.get(id)
                        .cloned()
                        .map(Value::Object)
                        .ok_or_else(|| corrupt("missing selected hit"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        check_response(&result)?;
        lease.complete();
        Ok(result)
    }
}
pub(crate) fn fate(row: &Object, discharges: &[Object]) -> Value {
    let mut result = pick(
        row,
        &[
            "kind",
            "transfer",
            "callee",
            "value",
            "depth",
            "conditional",
            "verdict",
            "boundary_reason",
            "condition",
            "callee_text",
            "phase",
            "premise_key",
            "occurrences",
            "path",
            "line",
            "site_text",
        ],
    );
    for (target, source) in [
        ("condition_scope_id", "condition_scope_node_id"),
        ("parameter", "parameter_name"),
        ("target", "target_name"),
    ] {
        result.insert(target.into(), row[source].clone());
    }
    result.insert(
        "discharges".into(),
        json!(
            discharges
                .iter()
                .map(|r| pick(
                    r,
                    &[
                        "origin_id",
                        "proof_kind",
                        "decision",
                        "summary_id",
                        "reason"
                    ]
                ))
                .collect::<Vec<_>>()
        ),
    );
    Value::Object(result)
}
pub(crate) fn fields(global: &str, reads: &[Object], claims: &[Object]) -> Vec<Value> {
    let mut grouped: BTreeMap<&str, Vec<Object>> = BTreeMap::new();
    for row in reads {
        grouped
            .entry(row["field"].as_str().expect("field"))
            .or_default()
            .push(pick(
                row,
                &["reader", "phase", "path", "line", "spelled", "condition"],
            ));
    }
    let prefix = format!("Global[{global}].");
    let claims: BTreeMap<_, _> = claims
        .iter()
        .filter_map(|r| {
            r["place_key"]
                .as_str()
                .and_then(|s| s.strip_prefix(&prefix))
                .map(|s| (s, r))
        })
        .collect();
    let names: BTreeSet<_> = grouped.keys().chain(claims.keys()).copied().collect();
    names.into_iter().map(|name|{let never=if !grouped.contains_key(name){claims.get(name).map(|r|if r["holds"]==true{"refuted_under_model: no read of the field anywhere in the release, and no name-driven access reaches it (external readers are outside the model)".into()}else{format!("unknown ({}): {}",r["boundary_reason"].as_str().filter(|s|!s.is_empty()).unwrap_or("not refuted"),r["reason"].as_str().unwrap_or("None"))})}else{None};json!({"name":name,"reads":grouped.get(name).cloned().unwrap_or_default(),"never_read":never})}).collect()
}
