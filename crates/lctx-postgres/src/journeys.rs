//! Browse and comparison compose the canonical catalog and existing classifier.
use crate::{
    Error,
    repository::{Hydration, PinnedGeneration},
    serving::{QueryLease, ServingStore},
};
use cpg_schema::{Codebook, Id, serving_projection::corrupt, wire::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

impl ServingStore {
    pub async fn browse_library(
        &self,
        g: &PinnedGeneration,
        request: &BrowseLibraryRequest,
    ) -> Result<Value, Error> {
        if request.library.as_str() != g.manifest.context.library {
            return Err(Error::Request("library is not the pinned one".into()));
        }
        let target = json!({"scope":request.scope,"view":request.view,"expanded":request.expanded,"limit":request.limit});
        let offset =
            crate::journey_cursor::position(g, &target, request.cursor.as_ref().map(Text::as_str))?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut budget = Hydration::new();
        let members = budget
            .fetch(&mut lease.connection, g, "catalog_members", None, &[])
            .await?;
        let domains = sqlx::query_file!("queries/browse_ownership.sql", g.id.0.as_slice())
            .fetch_all(&mut *lease.connection)
            .await?;
        if domains.len() > 200_000 {
            return Err(cpg_schema::serving_projection::refused("ownership population").into());
        }
        let ownership = domains
            .into_iter()
            .map(|r| {
                Ok((
                    Id(r.member_id
                        .try_into()
                        .map_err(|_| corrupt("ownership ID"))?)
                    .hex(),
                    (r.module, r.class_owner),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;
        let mut scoped = Vec::new();
        for row in members {
            let member: CatalogMember =
                serde_json::from_value(json!(row)).map_err(|_| corrupt("member contract"))?;
            let (module, class) = ownership
                .get(&member.member_id.hex())
                .cloned()
                .ok_or_else(|| corrupt("member ownership domain missing"))?;
            let admitted = match &request.scope {
                BrowseScope::Library => true,
                BrowseScope::Module { name } => module.as_deref() == Some(name.as_str()),
                BrowseScope::Class { name } => {
                    class.as_deref() == Some(name.as_str())
                        || (member.kind == "class" && member.access_path == name.as_str())
                }
            };
            if admitted {
                scoped.push((member, module, class));
            }
        }
        let member_count = scoped.len() as u64;
        let mut state = if scoped.is_empty() && !matches!(request.scope, BrowseScope::Library) {
            SectionState::Unavailable
        } else {
            SectionState::Available
        };
        let mut reason = if state == SectionState::Unavailable {
            Some("scope is not recorded in this generation".to_owned())
        } else {
            None
        };
        let unknown = scoped.iter().filter(|(_, m, _)| m.is_none()).count() as u64;
        let mut items = Vec::new();
        let mut vocabulary = None;
        match &request.view {
            BrowseView::Vocabulary => {
                vocabulary = Some(QueryVocabulary {
                    requirement_schema: schema_for::<Requirement>(false),
                    facets: cpg_schema::codebook::OperationFacet::all()
                        .iter()
                        .map(|f| f.text().into())
                        .collect(),
                    retrieval_families: cpg_schema::retrieval::VIEWS
                        .iter()
                        .map(|v| v.family)
                        .collect(),
                });
            }
            BrowseView::Outline => {
                let mut groups: BTreeMap<String, u64> = BTreeMap::new();
                for (member, module, class_owner) in scoped {
                    let group = match &request.scope {
                        BrowseScope::Library => module.clone(),
                        BrowseScope::Module { .. } => {
                            if member.kind == "class" {
                                Some(member.access_path.clone())
                            } else {
                                class_owner.clone()
                            }
                        }
                        BrowseScope::Class { .. } => None,
                    };
                    if let Some(name) = group {
                        *groups.entry(name).or_default() += 1;
                    } else {
                        items.push(BrowseEntry::Member {
                            member,
                            module,
                            class_owner,
                        });
                    }
                }
                let mut grouped = groups
                    .into_iter()
                    .map(|(name, members)| match request.scope {
                        BrowseScope::Library => BrowseEntry::Module { name, members },
                        _ => BrowseEntry::Class { name, members },
                    })
                    .collect::<Vec<_>>();
                items.sort_by_key(|v| match v {
                    BrowseEntry::Member { member, .. } => member.access_path.clone(),
                    _ => String::new(),
                });
                grouped.extend(items);
                items = grouped;
            }
            BrowseView::FacetValues { facet } => {
                let facet_kind = cpg_schema::codebook::OperationFacet::all()
                    .iter()
                    .find(|f| f.text() == facet.as_str())
                    .expect("validated facet");
                if state == SectionState::Available
                    && facet_kind.requires_behavior()
                    && !g.manifest.capabilities.behavioral_claims
                {
                    state = SectionState::NotRequested;
                    reason = Some("behavioral facet producer not requested".into());
                }
                let mut by_node: BTreeMap<Id, BTreeSet<Id>> = BTreeMap::new();
                for (member, _, _) in scoped {
                    if let Some(node) = member.operation_node_id {
                        by_node.entry(node).or_default().insert(member.member_id);
                    }
                }
                let nodes = by_node.keys().map(|id| id.0.to_vec()).collect::<Vec<_>>();
                let rows = budget
                    .fetch(
                        &mut lease.connection,
                        g,
                        "operation_facets",
                        Some("node_id"),
                        &nodes,
                    )
                    .await?;
                type FacetCounts = (BTreeSet<Id>, BTreeMap<String, BTreeSet<Id>>);
                let mut counts: BTreeMap<String, FacetCounts> = BTreeMap::new();
                for row in rows {
                    if row["facet"].as_str() != Some(facet.as_str()) {
                        continue;
                    }
                    let node = Id::from_hex(
                        row["node_id"]
                            .as_str()
                            .ok_or_else(|| corrupt("facet node"))?,
                    )
                    .ok_or_else(|| corrupt("facet ID"))?;
                    let members = by_node
                        .get(&node)
                        .ok_or_else(|| corrupt("facet outside scope"))?;
                    let entry = counts
                        .entry(
                            row["value"]
                                .as_str()
                                .ok_or_else(|| corrupt("facet value"))?
                                .into(),
                        )
                        .or_default();
                    entry.0.extend(members);
                    entry
                        .1
                        .entry(
                            row["verdict"]
                                .as_str()
                                .ok_or_else(|| corrupt("facet verdict"))?
                                .into(),
                        )
                        .or_default()
                        .extend(members);
                }
                items = counts
                    .into_iter()
                    .map(|(value, (members, verdicts))| BrowseEntry::FacetValue {
                        value,
                        members: members.len() as u64,
                        verdicts: verdicts
                            .into_iter()
                            .map(|(v, m)| (v, m.len() as u64))
                            .collect(),
                    })
                    .collect();
            }
        }
        lease.complete();
        let total = items.len() as u64;
        if offset > total {
            return Err(Error::Request("browse cursor exceeds population".into()));
        }
        let items = items
            .into_iter()
            .skip(offset as usize)
            .take(request.limit.get() as usize)
            .collect::<Vec<_>>();
        let next = offset + items.len() as u64;
        let response = BrowseLibraryResponse {
            snapshot_id: SnapshotId::parse(&g.manifest.snapshot_id)?,
            generation: GenerationDigest::parse(&g.generation())?,
            scope: request.scope.clone(),
            view: request.view.clone(),
            state,
            reason,
            members: member_count,
            unknown_ownership: unknown,
            capabilities: g.manifest.capabilities.clone(),
            items,
            vocabulary,
            total,
            next_cursor: if next < total {
                Some(crate::journey_cursor::encode(g, &target, next)?)
            } else {
                None
            },
        };
        let value = json!(response);
        crate::evidence::check_operation_packet(&value, request.expanded, 1024)?;
        Ok(value)
    }
    pub async fn compare_operations(
        &self,
        g: &PinnedGeneration,
        request: &CompareOperationsRequest,
    ) -> Result<Value, Error> {
        if request.library.as_str() != g.manifest.context.library {
            return Err(Error::Request("library is not the pinned one".into()));
        }
        let prepared = self.prepare_selection(g, &request.selection, "").await?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut budget = Hydration::new();
        let mut candidates = Vec::new();
        for requested in &request.candidates.0 {
            let members = crate::packet::resolve_members(
                &mut lease.connection,
                g,
                requested.as_str(),
                &mut budget,
            )
            .await?;
            candidates.push(if members.len() > 1 {
                ComparedOperation::Ambiguous {
                    requested: requested.to_string(),
                    choices: members
                        .into_iter()
                        .map(|row| {
                            serde_json::from_value(json!(row))
                                .map_err(|_| corrupt("comparison member"))
                        })
                        .collect::<Result<_, _>>()?,
                }
            } else if let Some(member) = members.first() {
                let member_id = PublicMemberId::parse(
                    member["member_id"]
                        .as_str()
                        .ok_or_else(|| corrupt("comparison member ID"))?,
                )?;
                let selected = prepared
                    .candidates
                    .iter()
                    .find(|c| c.member_id == member_id)
                    .ok_or_else(|| corrupt("classified comparison member missing"))?
                    .clone();
                let singleton =
                    crate::hydration::singleton_class(&mut lease.connection, g, member).await?;
                let contract = crate::hydration::catalog_invocation(
                    &mut lease.connection,
                    g,
                    &mut budget,
                    member,
                    singleton.as_deref(),
                )
                .await?;
                let signatures = serde_json::from_value(json!(contract.signatures))
                    .map_err(|_| corrupt("comparison signature contract"))?;
                ComparedOperation::Resolved {
                    requested: requested.to_string(),
                    selection: Box::new(selected),
                    signatures,
                    packet: GetOperationRequest {
                        snapshot_id: SnapshotId::parse(&g.manifest.snapshot_id)?,
                        operation: Text::new(member_id.hex())?,
                        expanded: false,
                        view: OperationView::Packet,
                    },
                }
            } else {
                ComparedOperation::NotFound {
                    requested: requested.to_string(),
                }
            });
        }
        lease.complete();
        let response = CompareOperationsResponse {
            snapshot_id: SnapshotId::parse(&g.manifest.snapshot_id)?,
            generation: GenerationDigest::parse(&g.generation())?,
            candidates,
        };
        let value = json!(response);
        crate::evidence::check_operation_packet(&value, request.expanded, 1024)?;
        Ok(value)
    }
}
