//! Evidence discovery ranks actual units; membership is independent of API selection.
use crate::{
    Error,
    repository::PinnedGeneration,
    serving::{QueryLease, ServingStore},
};
use cpg_schema::{
    Id,
    retrieval::*,
    serving_projection::{corrupt, refused},
    wire::*,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

impl ServingStore {
    pub async fn search_evidence(
        &self,
        g: &PinnedGeneration,
        request: &SearchEvidenceRequest,
        mut lexical: Vec<UnitWinner>,
        vector: Option<(&[f32], &str)>,
        channel_state: &str,
    ) -> Result<Value, Error> {
        if request.library.as_str() != g.manifest.context.library {
            return Err(Error::Request("library is not the pinned one".into()));
        }
        if request.families.is_empty()
            || request.families.len() > 4
            || lexical.len() > 200_000
            || channel_state.len() > 500
        {
            return Err(refused("evidence search input budget").into());
        }
        if lexical
            .iter()
            .any(|w| w.channel != Channel::Lexical || !w.score.is_finite())
        {
            return Err(corrupt("lexical unit contract").into());
        }
        if let Some((query, spec)) = vector {
            crate::retrieval::validate_query(query)?;
            if g.manifest.spec_hash.as_deref() != Some(spec) {
                return Err(Error::Request(
                    "query embedding spec differs from generation".into(),
                ));
            }
        }
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let families = request
            .families
            .iter()
            .map(|f| f.name().to_owned())
            .collect::<Vec<_>>();
        let intent = request.intent.as_ref().map(|i| {
            serde_json::to_value(i)
                .expect("intent")
                .as_str()
                .expect("intent string")
                .to_owned()
        });
        let (member, release) = match &request.subject {
            Some(Subject::Member(id)) => (Some(id.storage().0.to_vec()), None),
            Some(Subject::Release(id)) => (None, Some(id.0.to_vec())),
            None => (None, None),
        };
        let eligible: Vec<Vec<u8>> = sqlx::query_file_scalar!(
            "queries/evidence_eligible.sql",
            g.id.0.as_slice(),
            &families,
            intent,
            member,
            release
        )
        .fetch_all(&mut *lease.connection)
        .await?;
        if eligible.len() > 200_000 {
            return Err(refused("evidence unit population").into());
        }
        let allowed = eligible
            .iter()
            .map(|b| RetrievalUnitId::from_storage(Id(b.clone().try_into().expect("validated ID"))))
            .collect::<BTreeSet<_>>();
        lexical.retain(|w| allowed.contains(&w.unit_id));
        let units = lexical
            .iter()
            .map(|w| w.unit_id.storage().0.to_vec())
            .collect::<Vec<_>>();
        let fragments = lexical
            .iter()
            .map(|w| w.fragment_id.0.to_vec())
            .collect::<Vec<_>>();
        let family = lexical
            .iter()
            .map(|w| w.family.name().to_owned())
            .collect::<Vec<_>>();
        let valid: bool = sqlx::query_file_scalar!(
            "queries/evidence_winner_membership.sql",
            g.id.0.as_slice(),
            &units,
            &fragments,
            &family
        )
        .fetch_one(&mut *lease.connection)
        .await?;
        if !valid {
            return Err(corrupt("evidence winning fragment membership").into());
        }
        if let Some((query, _)) = vector {
            let rows = sqlx::query_file!(
                "queries/evidence_unit_ranks.sql",
                g.id.0.as_slice(),
                &eligible,
                query
            )
            .fetch_all(&mut *lease.connection)
            .await?;
            if rows.len() > 200_000 {
                return Err(refused("evidence vector rank population").into());
            }
            for row in rows {
                lexical.push(UnitWinner {
                    unit_id: RetrievalUnitId::from_storage(Id(row
                        .unit_id
                        .try_into()
                        .map_err(|_| corrupt("unit width"))?)),
                    fragment_id: Id(row
                        .fragment_id
                        .try_into()
                        .map_err(|_| corrupt("fragment width"))?),
                    family: serde_json::from_value(json!(row.family))
                        .map_err(|_| corrupt("family"))?,
                    channel: Channel::Vector,
                    score: row.score,
                    rank: 0,
                });
            }
        }
        // Release the SQL lease before ranking. No lease is ever held during embedding.
        lease.complete();
        drop(lease);
        let ranked = self
            .run_cpu(move || Ok(fuse_units(&unit_channel_winners(lexical)?)))
            .await?;
        let target = json!({"query":request.query,"families":request.families,"intent":request.intent,"subject":request.subject,"limit":request.limit,"expanded":request.expanded,"channel":channel_state,"fusion":FUSION_REVISION});
        let offset =
            crate::journey_cursor::position(g, &target, request.cursor.as_ref().map(Text::as_str))?;
        let total = ranked.len() as u64;
        if offset > total {
            return Err(Error::Request("evidence cursor exceeds population".into()));
        }
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut result = SearchEvidenceResponse {
            snapshot_id: SnapshotId::parse(&g.manifest.snapshot_id)?,
            generation: GenerationDigest::parse(&g.generation())?,
            items: vec![],
            total,
            next_cursor: None,
            retrieval: RetrievalMetadata {
                profile: g.profile.hex(),
                requested_route: RetrievalMetadataRequestedRoute::Exact,
                actual_route: if vector.is_some() {
                    RetrievalMetadataActualRoute::Exact
                } else {
                    RetrievalMetadataActualRoute::LexicalOnly
                },
                fallback: if vector.is_none() {
                    Some(channel_state.into())
                } else {
                    None
                },
                candidate_depth: total as i64,
                approximate: false,
                routing_reason: "independent_unit_family_rrf_v1".into(),
                admission: None,
            },
        };
        for row in ranked
            .into_iter()
            .skip(offset as usize)
            .take(request.limit.get() as usize)
        {
            let unit_id = row.unit_id.storage();
            let detail: Option<String> = sqlx::query_file_scalar!(
                "queries/evidence_search_unit.sql",
                g.id.0.as_slice(),
                unit_id.0.as_slice()
            )
            .fetch_one(&mut *lease.connection)
            .await?;
            let detail = detail.ok_or_else(|| refused("evidence header input budget"))?;
            let unit: Unit = serde_json::from_str(&detail).map_err(|_| corrupt("unit contract"))?;
            let scenario: Option<Option<String>> = if unit.family == Family::Scenario {
                sqlx::query_file_scalar!(
                    "queries/evidence_search_scenario.sql",
                    g.id.0.as_slice(),
                    unit.source_key.0.as_slice()
                )
                .fetch_optional(&mut *lease.connection)
                .await?
            } else {
                None
            };
            let scenario: Option<ScenarioSummary> = scenario
                .map(|raw| {
                    let raw = raw.ok_or_else(|| refused("scenario metadata input budget"))?;
                    serde_json::from_str(&raw).map_err(|_| corrupt("scenario contract"))
                })
                .transpose()?;
            let intent = scenario.as_ref().map(|s| s.intent.clone());
            result.items.push(EvidenceHit {
                unit: RetrievalUnitHeader::from(&unit),
                intent,
                scenario,
                winners: row.winners,
                score: row.score,
            });
            let next = offset + result.items.len() as u64;
            result.next_cursor = if next < total {
                Some(crate::journey_cursor::encode(g, &target, next)?)
            } else {
                None
            };
            if crate::evidence::check_operation_packet(&json!(result), request.expanded, 1024)
                .is_err()
            {
                result.items.pop();
                if result.items.is_empty() {
                    return Err(refused(
                        "indivisible evidence hit exceeds response budget; request expanded=true",
                    )
                    .into());
                }
                result.next_cursor = Some(crate::journey_cursor::encode(
                    g,
                    &target,
                    offset + result.items.len() as u64,
                )?);
                break;
            }
        }
        lease.complete();
        Ok(json!(result))
    }
}
