//! Mandatory invocation closure and independently paged optional enrichment.
use crate::hydration::{catalog_record, fate, fields, id, packet};
use crate::{
    Error,
    repository::{Hydration, Keys, PinnedGeneration},
    serving::{QueryLease, ServingStore},
};
use cpg_schema::{
    Id,
    serving_projection::{corrupt, refused},
    wire::*,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

impl ServingStore {
    pub async fn get_operation(
        &self,
        g: &PinnedGeneration,
        snapshot: &str,
        operation: &str,
    ) -> Result<Value, Error> {
        self.operation_packet(
            g,
            &GetOperationRequest {
                snapshot_id: SnapshotId::parse(snapshot)?,
                operation: Text::new(operation.into())?,
                expanded: false,
                view: OperationView::Packet,
            },
        )
        .await
    }
    pub async fn operation_packet(
        &self,
        g: &PinnedGeneration,
        request: &GetOperationRequest,
    ) -> Result<Value, Error> {
        g.check_snapshot(&request.snapshot_id.hex())?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut budget = Hydration::new();
        let members = resolve_members(
            &mut lease.connection,
            g,
            request.operation.as_str(),
            &mut budget,
        )
        .await?;
        if members.len() > 1 {
            let value = packet::<AmbiguousOperation>(
                json!({"snapshot_id":request.snapshot_id,"generation":g.generation(),"resolution":"ambiguous","requested":request.operation,"choices":members}),
            )?;
            lease.complete();
            return Ok(value);
        }
        let member = members
            .first()
            .ok_or_else(|| Error::Request("public member not found in generation".into()))?;
        // A singleton's declared class contributes invocation contracts, never nested behavior.
        let singleton_hex =
            crate::hydration::singleton_class(&mut lease.connection, g, member).await?;
        let node = singleton_hex
            .as_deref()
            .or_else(|| member["operation_node_id"].as_str());
        let nodes = node.map(id).transpose()?.into_iter().collect::<Vec<_>>();
        let resolution = if singleton_hex.is_some() {
            json!("singleton_class")
        } else {
            member["resolution"].clone()
        };
        if let OperationView::Section { section, cursor } = &request.view {
            let value = section_page(
                &mut lease.connection,
                g,
                member,
                &nodes,
                *section,
                cursor.as_ref().map(Text::as_str),
                request.expanded,
            )
            .await?;
            lease.complete();
            return Ok(value);
        }
        let catalog = catalog_record(
            &mut lease.connection,
            g,
            &mut budget,
            member,
            singleton_hex.as_deref(),
        )
        .await?;
        let operations = budget
            .fetch(
                &mut lease.connection,
                g,
                "operations",
                Some("node_id"),
                &nodes,
            )
            .await?;
        let paths = budget
            .fetch(
                &mut lease.connection,
                g,
                "public_paths",
                Some("node_id"),
                &nodes,
            )
            .await?;
        let statuses = budget
            .fetch(
                &mut lease.connection,
                g,
                "operation_facet_status",
                Some("node_id"),
                &nodes,
            )
            .await?;
        let incomplete: BTreeMap<_, _> = statuses
            .iter()
            .filter(|s| s["verdict"] != "established")
            .map(|s| {
                (
                    s["facet"].as_str().unwrap_or_default().to_owned(),
                    format!(
                        "{}: {}",
                        s["verdict"].as_str().unwrap_or_default(),
                        s["reason"].as_str().unwrap_or_default()
                    ),
                )
            })
            .collect();
        let mut own = paths
            .iter()
            .filter(|p| p["own"] == true)
            .map(|p| p["access_path"].clone())
            .collect::<Vec<_>>();
        if own.is_empty() {
            own.push(member["access_path"].clone());
        }
        let mut inherited = paths
            .iter()
            .filter(|p| p["own"] != true)
            .map(|p| p["access_path"].clone())
            .collect::<Vec<_>>();
        own.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        own.dedup();
        inherited.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        inherited.dedup();
        let op = operations.first();
        let sections = [
            OperationSection::Behavior,
            OperationSection::Relationships,
            OperationSection::Evidence,
            OperationSection::Fields,
            OperationSection::Facets,
        ]
        .into_iter()
        .map(|section| SectionDirectoryEntry {
            section,
            state: if section != OperationSection::Evidence
                && !g.manifest.capabilities.behavioral_claims
            {
                SectionState::NotRequested
            } else if section != OperationSection::Evidence && nodes.is_empty() {
                SectionState::Unavailable
            } else {
                SectionState::Available
            },
            total: None,
            reason: if section != OperationSection::Evidence
                && !g.manifest.capabilities.behavioral_claims
            {
                Some("behavioral analysis not requested".into())
            } else {
                None
            },
        })
        .collect::<Vec<_>>();
        let mut value = packet::<OperationPacket>(json!({
            "snapshot_id":request.snapshot_id,"generation":g.generation(),"library":g.manifest.context.library,"requirement":g.manifest.context.requirement,
            "member_id":member["member_id"],"operation_id":node,"access_path":member["access_path"],"resolution":resolution,"own_paths":own,"inherited_paths":inherited,
            "capabilities":g.manifest.capabilities,"catalog":catalog,"behavior_status":op.map(|o| o["behavior_status"].clone()).unwrap_or(json!("not_analyzed")),
            "boundary_reason":op.map(|o| o["boundary_reason"].clone()),"status_reason":op.map(|o| o["status_reason"].clone()),
            "capability_id":op.map(|o| o["brief_id"].clone()),"incomplete_facets":incomplete,"sections":sections,"demonstrations":[],"relationships":[],"evidence":RetrievalTarget::RetrievalUnit(cpg_schema::retrieval::unit_id(cpg_schema::retrieval::Family::ApiOptions,Id::from_hex(member["member_id"].as_str().expect("member ID")).expect("member ID")))
        }))?;
        // The core alone must fit. Optional records are admitted only after this check.
        crate::evidence::check_operation_packet(&value, request.expanded, 1024)?;
        let demos = sqlx::query_file!(
            "queries/packet_demonstrations.sql",
            g.id.0.as_slice(),
            id(member["member_id"].as_str().expect("member ID"))?
        )
        .fetch_all(&mut *lease.connection)
        .await?;
        value["demonstrations"] = json!(
            demos
                .into_iter()
                .map(
                    |r| cpg_schema::evidence::EvidenceRef::Scenario(ScenarioId::from_storage(Id(
                        r.evidence_id.try_into().expect("validated ID")
                    )))
                )
                .collect::<Vec<_>>()
        );
        if crate::evidence::check_operation_packet(&value, request.expanded, 1024).is_err() {
            value["demonstrations"] = json!([]);
            value["sections"][2]["state"] = json!("omitted_budget");
        }
        if g.manifest.capabilities.behavioral_claims && !nodes.is_empty() {
            match section_page(
                &mut lease.connection,
                g,
                member,
                &nodes,
                OperationSection::Relationships,
                None,
                request.expanded,
            )
            .await
            {
                Ok(links) => {
                    for record in links["items"].as_array().into_iter().flatten().take(5) {
                        value["relationships"]
                            .as_array_mut()
                            .expect("relationships")
                            .push(record["record"].clone());
                        if crate::evidence::check_operation_packet(&value, request.expanded, 1024)
                            .is_err()
                        {
                            value["relationships"]
                                .as_array_mut()
                                .expect("relationships")
                                .pop();
                            value["sections"][1]["state"] = json!("omitted_budget");
                            break;
                        }
                    }
                    value["sections"][1]["total"] = links["total"].clone();
                }
                Err(Error::Projection(e))
                    if e.kind == cpg_schema::serving_projection::FailureKind::ResourceRefused =>
                {
                    value["sections"][1]["state"] = json!("omitted_budget");
                }
                Err(e) => return Err(e),
            }
        }
        lease.complete();
        Ok(value)
    }
}

pub(crate) async fn resolve_members(
    conn: &mut sqlx::PgConnection,
    g: &PinnedGeneration,
    spelling: &str,
    budget: &mut Hydration,
) -> Result<Vec<serde_json::Map<String, Value>>, Error> {
    let entity = Id::from_hex(spelling.trim()).map(|id| id.0.to_vec());
    let ids = sqlx::query_file!(
        "queries/packet_resolve.sql",
        g.id.0.as_slice(),
        spelling.trim(),
        entity
    )
    .fetch_all(&mut *conn)
    .await?;
    if ids.len() > 100 {
        return Err(refused("public member choice budget").into());
    }
    budget
        .fetch(
            conn,
            g,
            "catalog_members",
            Some("member_id"),
            &ids.into_iter().map(|r| r.member_id).collect::<Vec<_>>(),
        )
        .await
}

async fn section_page(
    conn: &mut sqlx::PgConnection,
    g: &PinnedGeneration,
    member: &serde_json::Map<String, Value>,
    nodes: &[Vec<u8>],
    section: OperationSection,
    raw: Option<&str>,
    expanded: bool,
) -> Result<Value, Error> {
    let member_id = PublicMemberId::parse(
        member["member_id"]
            .as_str()
            .ok_or_else(|| corrupt("member ID"))?,
    )?;
    let target = json!({"member":member_id,"section":section,"expanded":expanded});
    let offset = crate::journey_cursor::position(g, &target, raw)?;
    let mut page = OperationSectionPage {
        snapshot_id: SnapshotId::parse(&g.manifest.snapshot_id)?,
        generation: GenerationDigest::parse(&g.generation())?,
        member_id,
        section,
        state: SectionState::Available,
        reason: None,
        items: vec![],
        total: 0,
        next_cursor: None,
    };
    if section != OperationSection::Evidence && !g.manifest.capabilities.behavioral_claims {
        page.state = SectionState::NotRequested;
        page.reason = Some("behavioral analysis not requested".into());
        return Ok(json!(page));
    }
    if section != OperationSection::Evidence && nodes.is_empty() {
        page.state = SectionState::Unavailable;
        page.reason = Some("public binding has no resolved behavioral operation".into());
        return Ok(json!(page));
    }
    let mut budget = Hydration::new();
    match section {
        OperationSection::Evidence => {
            let associations =
                crate::evidence::association_page(conn, g, member_id.storage(), offset, 20).await?;
            page.total = associations.total;
            page.items = associations
                .items
                .into_iter()
                .map(|record| OperationSectionRecord::Association { record })
                .collect();
        }
        OperationSection::Behavior | OperationSection::Relationships => {
            let relationships = section == OperationSection::Relationships;
            let rows = sqlx::query_file!(
                "queries/packet_behaviors.sql",
                g.id.0.as_slice(),
                nodes,
                relationships,
                offset as i64
            )
            .fetch_all(&mut *conn)
            .await?;
            page.total = sqlx::query_file_scalar!(
                "queries/packet_behavior_count.sql",
                g.id.0.as_slice(),
                nodes,
                relationships
            )
            .fetch_one(&mut *conn)
            .await? as u64;
            let ids = rows.into_iter().map(|r| r.behavior_id).collect::<Vec<_>>();
            let mut behaviors = budget
                .fetch(conn, g, "behaviors", Some("behavior_id"), &ids)
                .await?;
            behaviors.sort_by(|a, b| a["behavior_id"].as_str().cmp(&b["behavior_id"].as_str()));
            let discharges = budget
                .fetch(conn, g, "behavior_discharges", Some("behavior_id"), &ids)
                .await?;
            for row in behaviors {
                let ds = discharges
                    .iter()
                    .filter(|d| d["behavior_id"] == row["behavior_id"])
                    .cloned()
                    .collect::<Vec<_>>();
                let record = serde_json::from_value(fate(&row, &ds))
                    .map_err(|_| corrupt("fate contract"))?;
                page.items.push(OperationSectionRecord::Fate { record });
            }
        }
        OperationSection::Facets => {
            page.total = sqlx::query_file_scalar!(
                "queries/packet_facet_count.sql",
                g.id.0.as_slice(),
                nodes
            )
            .fetch_one(&mut *conn)
            .await? as u64;
            let rows = sqlx::query_file!(
                "queries/packet_facets.sql",
                g.id.0.as_slice(),
                nodes,
                offset as i64
            )
            .fetch_all(&mut *conn)
            .await?;
            page.items = rows
                .into_iter()
                .map(|r| OperationSectionRecord::Facet {
                    name: r.facet,
                    value: r.value,
                    verdict: r.verdict,
                })
                .collect();
        }
        OperationSection::Fields => {
            let global = member["access_path"]
                .as_str()
                .ok_or_else(|| corrupt("member path"))?;
            let prefix = format!("Global[{global}].");
            let all: Vec<String> = sqlx::query_file_scalar!(
                "queries/packet_field_keys.sql",
                g.id.0.as_slice(),
                global,
                &prefix
            )
            .fetch_all(&mut *conn)
            .await?;
            if all.len() > 200_000 {
                return Err(refused("field key population").into());
            }
            page.total = all.len() as u64;
            let names = all
                .into_iter()
                .skip(offset as usize)
                .take(20)
                .collect::<Vec<_>>();
            let places = names
                .iter()
                .map(|name| format!("{prefix}{name}"))
                .collect::<Vec<_>>();
            let ordinals: Vec<i64> = sqlx::query_file_scalar!(
                "queries/packet_field_ordinals.sql",
                g.id.0.as_slice(),
                global,
                &names
            )
            .fetch_all(&mut *conn)
            .await?;
            if ordinals.len() > 200_000 {
                return Err(refused("field read population").into());
            }
            let reads = budget
                .fetch_keys(
                    conn,
                    g,
                    "ambient_reads",
                    Some("row_ordinal"),
                    Keys::Ordinals(&ordinals),
                )
                .await?;
            let claims = budget
                .fetch_keys(
                    conn,
                    g,
                    "place_claims",
                    Some("place_key"),
                    Keys::Text(&places),
                )
                .await?;
            for record in fields(global, &reads, &claims) {
                page.items.push(OperationSectionRecord::Field {
                    record: serde_json::from_value(record)
                        .map_err(|_| corrupt("field contract"))?,
                });
            }
        }
    }
    if offset > page.total {
        return Err(Error::Request("section cursor exceeds population".into()));
    }
    if page.total == 0 {
        page.state = if section == OperationSection::Evidence {
            SectionState::EmptyUnderCoverage
        } else {
            SectionState::Unavailable
        };
        page.reason = Some(
            if section == OperationSection::Evidence {
                "no recorded associations in this generation"
            } else {
                "no records; scoped behavioral coverage is not established"
            }
            .into(),
        );
    }
    loop {
        let end = offset + page.items.len() as u64;
        page.next_cursor = if end < page.total {
            Some(crate::journey_cursor::encode(g, &target, end)?)
        } else {
            None
        };
        let value = json!(page);
        if crate::evidence::check_operation_packet(&value, expanded, 1024).is_ok() {
            return Ok(value);
        }
        page.items.pop();
        if page.items.is_empty() {
            return Err(refused("indivisible section record exceeds response budget; request expanded=true or get_evidence for originals").into());
        }
    }
}
