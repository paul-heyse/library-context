//! Complete selected records. Query projection and evidence assembly have one Rust owner.
use crate::{
    Error,
    repository::{Hydration, Keys, Object, PinnedGeneration, check_response, resolve_on},
    serving::{QueryLease, ServingStore},
};
use cpg_schema::{id::Id, serving_projection::corrupt};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn id(text: &str) -> Result<Vec<u8>, Error> {
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

async fn catalog_record(conn: &mut sqlx::PgConnection, generation: &PinnedGeneration,
    budget: &mut Hydration, member: &Object) -> Result<Value, Error> {
    let members = vec![id(member["member_id"].as_str().ok_or_else(|| corrupt("catalog member identity"))?)?];
    let bindings = budget.fetch(conn, generation, "catalog_bindings", Some("member_id"), &members).await?;
    let mut declarations = ids(&bindings, "declaration_node_id")?;
    if let Some(node) = member["operation_node_id"].as_str() { declarations.push(id(node)?); }
    declarations.sort(); declarations.dedup();
    let constructors = budget.fetch(conn, generation, "catalog_constructors", Some("class_node_id"), &declarations).await?;
    let mut signatures = budget.fetch(conn, generation, "catalog_signatures", Some("callable_node_id"), &declarations).await?;
    signatures.extend(budget.fetch(conn, generation, "catalog_signatures", Some("signature_id"), &ids(&constructors, "signature_id")?).await?);
    signatures.sort_by(|a,b| a["signature_id"].as_str().cmp(&b["signature_id"].as_str()));
    signatures.dedup_by(|a,b| a["signature_id"] == b["signature_id"]);
    let parameters = budget.fetch(conn, generation, "catalog_parameters", Some("signature_id"), &ids(&signatures,"signature_id")?).await?;
    let mut subjects = declarations.clone();
    subjects.extend(ids(&signatures, "declaration_node_id")?);
    subjects.extend(ids(&parameters, "formal_node_id")?);
    subjects.sort(); subjects.dedup();
    let observations = budget.fetch(conn, generation, "catalog_type_observations", Some("subject_node_id"), &subjects).await?;
    let mut term_ids = ids(&observations,"term_id")?;
    let mut visited = BTreeSet::new();
    let mut types = Vec::new(); let mut type_args = Vec::new();
    while !term_ids.is_empty() {
        term_ids.retain(|id| visited.insert(id.clone()));
        if term_ids.is_empty() { break; }
        types.extend(budget.fetch(conn, generation, "catalog_types", Some("term_id"), &term_ids).await?);
        let args = budget.fetch(conn, generation, "catalog_type_args", Some("parent_term_id"), &term_ids).await?;
        term_ids = ids(&args,"child_term_id")?;
        type_args.extend(args);
    }
    let evidence = budget.fetch(conn, generation, "catalog_evidence", Some("subject_node_id"), &subjects).await?;
    let params = grouped(parameters, "signature_id");
    for signature in &mut signatures {
        let mut parameters = group(&params, signature["signature_id"].as_str().ok_or_else(|| corrupt("signature identity"))?).to_vec();
        parameters.sort_by_key(|p| p["ordinal"].as_i64());
        signature.insert("parameters".into(), json!(parameters));
    }
    Ok(json!({"member":member,"bindings":bindings,"constructors":constructors,"signatures":signatures,"evidence":evidence,
        "type_observations":observations,"types":types,"type_arguments":type_args,
        "effective_surface":"unresolved","basis":"source and attributed provider observations"}))
}

impl ServingStore {
    pub async fn get_operation(
        &self,
        generation: &PinnedGeneration,
        snapshot: &str,
        operation: &str,
    ) -> Result<Value, Error> {
        generation.check_snapshot(snapshot)?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut catalog_budget = Hydration::new();
        let spelling = operation.trim();
        let member_id = spelling.strip_prefix("member:").map(id).transpose()?;
        let legacy = Id::from_hex(spelling).map(|id| id.0.to_vec());
        let member_ids: Vec<(Vec<u8>,)> = sqlx::query_as(
            "SELECT member_id FROM lctx_serving.catalog_members WHERE generation_digest=$1 AND (access_path=$2 OR member_id=$3 OR operation_node_id=$4) ORDER BY access_path COLLATE \"C\",member_id LIMIT 101")
            .bind(generation.id.0.as_slice()).bind(spelling).bind(member_id).bind(legacy)
            .fetch_all(&mut *lease.connection).await?;
        if member_ids.len() > 100 { return Err(cpg_schema::serving_projection::refused("public member choice budget").into()); }
        let member_ids: Vec<_> = member_ids.into_iter().map(|r| r.0).collect();
        let members = catalog_budget.fetch(&mut lease.connection, generation, "catalog_members", Some("member_id"), &member_ids).await?;
        if members.len() > 1 {
            let result = json!({"snapshot_id":snapshot,"generation":generation.generation(),
                "resolution":"ambiguous","requested":operation,"choices":members});
            lease.complete();
            return Ok(result);
        }
        let member = members.first();
        let catalog = if let Some(member) = member {
            Some(catalog_record(&mut lease.connection, generation, &mut catalog_budget, member).await?)
        } else { None };
        if let Some(member) = member && member["operation_node_id"].is_null() {
            let mut result = json!({"snapshot_id":snapshot,"generation":generation.generation(),
                "operation_id":null,"member_id":member["member_id"],"access_path":member["access_path"],
                "resolution":member["resolution"],"kind":member["kind"],"is_method":null,
                "own_paths":[member["access_path"]],"inherited_paths":[],"qualified_name":member["access_path"],
                "module":member["owner_path"],"docstring_summary":null,"behavior_status":"not_analyzed",
                "boundary_reason":"unresolved_target","status_reason":"public binding unresolved",
                "capability_id":null,"facets":{},"incomplete_facets":{},"parameters":[],
                "delegates":[],"handoffs":[],"reads":[],"constructor":null,"singleton_of":null,"fields":[]});
            result["catalog"] = catalog.unwrap_or(Value::Null);
            result["capabilities"] = json!(generation.manifest.capabilities);
            check_response(&result)?;
            lease.complete();
            return Ok(result);
        }
        let selected_operation = member.and_then(|m| m["operation_node_id"].as_str()).unwrap_or(operation);
        let resolved = resolve_on(&mut lease.connection, generation, selected_operation).await?;
        let mut budget = Hydration::new();
        let selected = vec![id(&resolved.operation_id)?];
        let initial = budget
            .fetch(
                &mut lease.connection,
                generation,
                "operations",
                Some("node_id"),
                &selected,
            )
            .await?;
        let mut requested = selected.clone();
        let mut constructor = None;
        if initial.first().is_some_and(|o| o["kind"] == "class") && let Some(contract) = &catalog {
            let signature_ids: BTreeSet<_> = contract["constructors"].as_array().into_iter().flatten()
                .filter_map(|c| c["signature_id"].as_str()).collect();
            let callable_ids: BTreeSet<_> = contract["signatures"].as_array().into_iter().flatten()
                .filter(|s| signature_ids.contains(s["signature_id"].as_str().unwrap_or("")))
                .filter_map(|s| s["callable_node_id"].as_str()).collect();
            if callable_ids.len() == 1 {
                let callable = *callable_ids.first().expect("one constructor");
                let candidate = id(callable)?;
                if !budget.fetch(&mut lease.connection, generation, "operations", Some("node_id"), std::slice::from_ref(&candidate)).await?.is_empty() {
                    constructor = Some(callable.to_owned()); requested.push(candidate);
                }
            }
        }
        let operations = keyed(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "operations",
                    Some("node_id"),
                    &requested,
                )
                .await?,
            "node_id",
        );
        let paths = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "public_paths",
                    Some("node_id"),
                    &requested,
                )
                .await?,
            "node_id",
        );
        let facets = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "operation_facets",
                    Some("node_id"),
                    &requested,
                )
                .await?,
            "node_id",
        );
        let statuses = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "operation_facet_status",
                    Some("node_id"),
                    &requested,
                )
                .await?,
            "node_id",
        );
        let behavior_rows = budget
            .fetch(
                &mut lease.connection,
                generation,
                "behaviors",
                Some("operation_node_id"),
                &requested,
            )
            .await?;
        let discharges = grouped(
            budget
                .fetch(
                    &mut lease.connection,
                    generation,
                    "behavior_discharges",
                    Some("behavior_id"),
                    &ids(&behavior_rows, "behavior_id")?,
                )
                .await?,
            "behavior_id",
        );
        let behaviors = grouped(behavior_rows, "operation_node_id");
        let singletons = budget
            .fetch(
                &mut lease.connection,
                generation,
                "singletons",
                Some("class_node_id"),
                &requested,
            )
            .await?;
        let globals: Vec<_> = singletons
            .iter()
            .map(|r| r["global"].as_str().expect("global").to_owned())
            .collect();
        let reads = grouped(
            budget
                .fetch_keys(
                    &mut lease.connection,
                    generation,
                    "ambient_reads",
                    Some("global"),
                    Keys::Text(&globals),
                )
                .await?,
            "global",
        );
        let prefixes: Vec<_> = globals.iter().map(|g| format!("Global[{g}].")).collect();
        budget.charge_response(&reads)?;
        let claims = budget
            .fetch_keys(
                &mut lease.connection,
                generation,
                "place_claims",
                Some("place_key"),
                Keys::Prefixes(&prefixes),
            )
            .await?;
        let mut records = BTreeMap::new();
        for (node, row) in &operations {
            budget.charge_response((
                row,
                group(&paths, node),
                group(&facets, node),
                group(&statuses, node),
            ))?;
            let mut record = pick(
                row,
                &[
                    "access_path",
                    "kind",
                    "is_method",
                    "qualified_name",
                    "module",
                    "docstring_summary",
                    "behavior_status",
                    "boundary_reason",
                    "status_reason",
                ],
            );
            record.extend(json!({"snapshot_id":generation.manifest.snapshot_id,"generation":generation.id.hex(),"operation_id":node,"capability_id":row["brief_id"]}).as_object().expect("object").clone());
            let mut own = Vec::new();
            let mut inherited = Vec::new();
            for p in group(&paths, node) {
                if p["own"] == true {
                    own.push(p["access_path"].clone());
                } else {
                    inherited.push(p["access_path"].clone());
                }
            }
            own.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            inherited.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            let mut facet_map: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            let mut parameters: Vec<(String, Vec<Value>)> = catalog.as_ref()
                .and_then(|c| c["signatures"].as_array()).into_iter().flatten()
                .flat_map(|s| s["parameters"].as_array().into_iter().flatten())
                .filter_map(|p| p["name"].as_str()).collect::<BTreeSet<_>>()
                .into_iter().map(|name| (name.to_owned(), Vec::new())).collect();
            for f in group(&facets, node) {
                let name = f["facet"].as_str().expect("facet");
                facet_map
                    .entry(name.into())
                    .or_default()
                    .push(json!({"value":f["value"],"verdict":f["verdict"]}));
            }
            let incomplete: BTreeMap<_, _> = group(&statuses, node)
                .iter()
                .filter(|s| s["verdict"] != "established")
                .map(|s| {
                    (
                        s["facet"].as_str().expect("facet").to_owned(),
                        match s["reason"].as_str() {
                            Some(r) if !r.is_empty() => {
                                format!("{}: {r}", s["verdict"].as_str().expect("verdict"))
                            }
                            _ => s["verdict"].as_str().expect("verdict").to_owned(),
                        },
                    )
                })
                .collect();
            let mut unbound_fates = Vec::new();
            let mut delegates = Vec::new();
            let mut supplies = Vec::new();
            let mut handoffs = Vec::new();
            let mut settings = Vec::new();
            for b in group(&behaviors, node) {
                budget.charge_response((
                    b,
                    group(&discharges, b["behavior_id"].as_str().expect("behavior ID")),
                ))?;
                let fate = fate(
                    b,
                    group(&discharges, b["behavior_id"].as_str().expect("behavior ID")),
                );
                let kind = b["kind"].as_str().expect("behavior kind");
                if matches!(
                    kind,
                    "forwards"
                        | "raises_when"
                        | "unfollowed"
                        | "derives"
                        | "stores"
                        | "returns"
                        | "is_read"
                        | "tests"
                ) && let Some(name) = b["parameter_name"].as_str().filter(|s| !s.is_empty())
                {
                    if let Some((_, fates)) = parameters.iter_mut().find(|(n, _)| n == name) {
                        fates.push(fate.clone());
                    } else {
                        unbound_fates.push(fate.clone());
                    }
                }
                match kind {
                    "delegates" => delegates.push(fate),
                    "supplies_literal" => supplies.push(fate),
                    "hands_off_to" | "takes_from" => handoffs.push(fate),
                    "reads_setting" => settings.push(fate),
                    _ => {}
                }
            }
            delegates.extend(supplies);
            let parameters:Vec<_>=parameters.into_iter().map(|(name,fates)|json!({"name":name,"note":if fates.is_empty(){Some("no fate found: reads the flow IR could not attribute (a dynamic or unpacked use) are not shown; never read this as unused")}else{None},"fates":fates})).collect();
            let singleton = singletons
                .iter()
                .find(|s| s["class_node_id"].as_str() == Some(node));
            let fields = if let Some(singleton) = singleton {
                fields(
                    singleton["global"].as_str().expect("global"),
                    group(&reads, singleton["global"].as_str().expect("global")),
                    &claims,
                )
            } else {
                Vec::new()
            };
            record.extend(json!({"own_paths":own,"inherited_paths":inherited,"facets":facet_map,"incomplete_facets":incomplete,"parameters":parameters,"unbound_parameter_fates":unbound_fates,"delegates":delegates,"handoffs":handoffs,"reads":settings,"constructor":null,"singleton_of":singleton.map(|s|s["global"].clone()),"fields":fields}).as_object().expect("object").clone());
            records.insert(node.clone(), Value::Object(record));
        }
        let ctor = if let Some(ctor) = constructor {
            if ctor == resolved.operation_id {
                return Err(corrupt("cyclic constructor").into());
            }
            Some(
                records
                    .remove(&ctor)
                    .ok_or_else(|| corrupt("missing constructor"))?,
            )
        } else {
            None
        };
        let mut result = records
            .remove(&resolved.operation_id)
            .ok_or_else(|| corrupt("missing resolved operation"))?;
        result["constructor"] = ctor.unwrap_or(Value::Null);
        result["catalog"] = catalog.unwrap_or(Value::Null);
        result["capabilities"] = json!(generation.manifest.capabilities);
        if let Some(member) = member {
            result["member_id"] = member["member_id"].clone();
            result["access_path"] = member["access_path"].clone();
            result["resolution"] = member["resolution"].clone();
        }
        check_response(&result)?;
        lease.complete();
        Ok(result)
    }
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
        Ok(result)
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
fn fate(row: &Object, discharges: &[Object]) -> Value {
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
fn fields(global: &str, reads: &[Object], claims: &[Object]) -> Vec<Value> {
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
