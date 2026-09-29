//! Request-local classified selection. No connection or cache survives preparation.
use crate::{
    Error,
    repository::{Hydration, PinnedGeneration},
    serving::{QueryLease, ServingStore},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cpg_schema::{
    selection::{self, catalog::*},
    serving_projection::{corrupt, refused},
    wire::*,
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

fn allowed(mode: SelectionMode, outcome: SelectionOutcome) -> bool {
    outcome == SelectionOutcome::Supported
        || (mode == SelectionMode::Discovery
            && matches!(
                outcome,
                SelectionOutcome::Unresolved | SelectionOutcome::Conflicting
            ))
}

#[derive(Clone)]
pub struct PreparedSelection {
    generation: String,
    hash: String,
    selection: Selection,
    pub(crate) candidates: std::sync::Arc<Vec<CandidateSelection>>,
    pub(crate) promoted: BTreeSet<PublicMemberId>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    generation: String,
    request: String,
    group: SelectionOutcome,
    ordering: u32,
    offset: usize,
}
impl PreparedSelection {
    pub fn check(&self, generation: &PinnedGeneration) -> Result<(), Error> {
        if self.generation != generation.generation() {
            Err(Error::Request(
                "selection belongs to another generation".into(),
            ))
        } else {
            Ok(())
        }
    }
    pub fn eligible(&self) -> Vec<PublicMemberId> {
        self.candidates
            .iter()
            .filter(|c| allowed(self.selection.mode, c.outcome))
            .map(|c| c.member_id)
            .collect()
    }
    pub fn scope(&self) -> Value {
        serde_json::json!({"generation":self.generation,"eligible":self.eligible(),"promoted":self.promoted,"request":self.hash})
    }
    pub fn page(
        &self,
        generation: &PinnedGeneration,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        self.check(generation)?;
        if !(1..=100).contains(&limit) {
            return Err(Error::Request("limit must be 1 through 100".into()));
        }
        let parsed: Option<Cursor> = cursor
            .map(|raw| {
                if raw.len() > 2048 {
                    return Err(Error::Request("invalid selection cursor".into()));
                }
                let bytes = URL_SAFE_NO_PAD
                    .decode(raw)
                    .map_err(|_| Error::Request("invalid selection cursor".into()))?;
                serde_json::from_slice(&bytes)
                    .map_err(|_| Error::Request("invalid selection cursor".into()))
            })
            .transpose()?;
        if parsed.as_ref().is_some_and(|c| {
            c.generation != self.generation
                || c.request != self.hash
                || c.ordering != selection::POLICY_REVISION
                || c.offset > 200_000
                || c.group == SelectionOutcome::Contradicted
        }) {
            return Err(Error::Request(
                "cursor belongs to another generation, request or ordering".into(),
            ));
        }
        let group = |outcome: SelectionOutcome| -> Result<SelectionGroup, Error> {
            let available = self.selection.mode == SelectionMode::Discovery
                || outcome == SelectionOutcome::Supported;
            let all: Vec<_> = self
                .candidates
                .iter()
                .filter(|c| c.outcome == outcome && available)
                .collect();
            let total = all.len();
            let offset = parsed
                .as_ref()
                .filter(|c| c.group == outcome)
                .map_or(0, |c| c.offset);
            let items = all
                .into_iter()
                .skip(offset)
                .take(limit as usize)
                .cloned()
                .collect::<Vec<_>>();
            let end = offset + items.len();
            let next_cursor = if end < total {
                Some(
                    URL_SAFE_NO_PAD.encode(
                        serde_json::to_vec(&Cursor {
                            generation: self.generation.clone(),
                            request: self.hash.clone(),
                            group: outcome,
                            ordering: selection::POLICY_REVISION,
                            offset: end,
                        })
                        .map_err(|_| corrupt("cursor encoding"))?,
                    ),
                )
            } else {
                None
            };
            Ok(SelectionGroup {
                items,
                total: total as u64,
                page_complete: next_cursor.is_none(),
                next_cursor,
            })
        };
        let page = SelectionResults {
            snapshot_id: SnapshotId::parse(&generation.manifest().snapshot_id)?,
            generation: GenerationDigest::parse(&self.generation)?,
            supported: group(SelectionOutcome::Supported)?,
            unresolved: group(SelectionOutcome::Unresolved)?,
            conflicting: group(SelectionOutcome::Conflicting)?,
            contradicted_count: self
                .candidates
                .iter()
                .filter(|c| c.outcome == SelectionOutcome::Contradicted)
                .count() as u64,
            ranked_discovery: false,
            policy_revision: selection::POLICY_REVISION,
            retrieval: None,
        };
        let value = serde_json::to_value(page).map_err(|_| corrupt("selection response"))?;
        crate::repository::check_response(&value)?;
        Ok(value)
    }
}
async fn typed<T: serde::de::DeserializeOwned>(
    input_bytes: &mut usize,
    conn: &mut sqlx::PgConnection,
    generation: &PinnedGeneration,
    name: &'static str,
) -> Result<Vec<T>, Error> {
    // Hydration bounds each transient Arrow/JSON conversion. Those objects are consumed here,
    // so their map/column overhead must not be charged as retained across unrelated relations.
    let rows = Hydration::new()
        .fetch(conn, generation, name, None, &[])
        .await?;
    rows.into_iter()
        .map(|r| {
            let value = Value::Object(r);
            charge_input(
                input_bytes,
                serde_json::to_vec(&value)
                    .map_err(|_| corrupt("selection input encoding"))?
                    .len(),
                std::mem::size_of::<T>(),
            )?;
            serde_json::from_value(value)
                .map_err(|_| corrupt(format!("typed selection relation {name}")).into())
        })
        .collect()
}
fn charge_input(total: &mut usize, bytes: usize, row_size: usize) -> Result<(), Error> {
    // Include simultaneous encoded/decoded data and typed row/index overhead. Refuse before
    // accumulating a new typed record; this is independent of the transient hydration limit.
    *total = total.saturating_add(
        bytes
            .saturating_mul(3)
            .saturating_add(row_size)
            .saturating_add(128),
    );
    if *total > 64 * 1024 * 1024 {
        return Err(refused("selection retained input byte budget").into());
    }
    Ok(())
}
impl ServingStore {
    pub async fn prepare_selection(
        &self,
        generation: &PinnedGeneration,
        selection: &Selection,
        query: &str,
    ) -> Result<PreparedSelection, Error> {
        if selection.requirements.len() > 16 || query.chars().count() > 4000 {
            return Err(Error::Request("selection request budget".into()));
        }
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut input_bytes = 0usize;
        let mut input = Inputs::default();
        macro_rules! load {
            ($field:ident,$name:literal) => {
                input.$field =
                    typed(&mut input_bytes, &mut lease.connection, generation, $name).await?
            };
        }
        load!(members, "catalog_members");
        let domain_names: Vec<String> = selection
            .requirements
            .iter()
            .filter_map(Requirement::input_domain)
            .map(|domain| {
                serde_json::to_value(domain)
                    .expect("finite domain enum")
                    .as_str()
                    .expect("domain string")
                    .to_owned()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let generation_id = generation.id();
        let mut domain_rows = sqlx::query_file!(
            "queries/selection_domains.sql",
            generation_id.0.as_slice(),
            &domain_names
        )
        .fetch(&mut *lease.connection);
        while let Some(r) = domain_rows.try_next().await? {
            charge_input(
                &mut input_bytes,
                r.detail.len(),
                std::mem::size_of::<DomainInput>(),
            )?;
            if input.domains.len() >= 200_000 {
                return Err(refused("selection domain input budget").into());
            }
            input.domains.push(DomainInput {
                member_id: cpg_schema::Id(
                    r.member_id
                        .try_into()
                        .map_err(|_| corrupt("domain member identity"))?,
                ),
                domain_id: cpg_schema::Id(
                    r.domain_id
                        .try_into()
                        .map_err(|_| corrupt("domain identity"))?,
                ),
                detail: r.detail,
            });
        }
        drop(domain_rows);
        let dependencies: BTreeSet<_> = selection
            .requirements
            .iter()
            .flat_map(Requirement::dependencies)
            .copied()
            .collect();
        for name in &dependencies {
            match *name {
                "catalog_bindings" => load!(bindings, "catalog_bindings"),
                "catalog_surfaces" => load!(surfaces, "catalog_surfaces"),
                "catalog_signatures" => load!(signatures, "catalog_signatures"),
                "catalog_parameters" => load!(parameters, "catalog_parameters"),
                "catalog_type_observations" => {
                    load!(type_observations, "catalog_type_observations")
                }
                "catalog_types" => load!(types, "catalog_types"),
                "catalog_type_args" => load!(type_args, "catalog_type_args"),
                "catalog_configurations" => load!(configurations, "catalog_configurations"),
                "catalog_field_links" => load!(field_links, "catalog_field_links"),
                "operation_facets" => load!(facets, "operation_facets"),
                "operation_facet_status" => load!(facet_status, "operation_facet_status"),
                "catalog_associations" => load!(associations, "catalog_associations"),
                "catalog_scenarios" => load!(scenarios, "catalog_scenarios"),
                "catalog_deployments" => load!(deployments, "catalog_deployments"),
                "catalog_spans" => load!(spans, "catalog_spans"),
                "catalog_artifacts" => {}
                _ => return Err(corrupt("undeclared selection dependency").into()),
            }
        }
        // Original bodies are not decision inputs; project only the three typed alignment fields.
        if dependencies.contains("catalog_artifacts") {
            let generation_id = generation.id();
            let mut rows =
                sqlx::query_file!("queries/artifact_alignment.sql", generation_id.0.as_slice())
                    .fetch(&mut *lease.connection);
            while let Some(r) = rows.try_next().await? {
                if input.artifacts.len() >= 200_000 {
                    return Err(refused("selection artifact rows").into());
                }
                charge_input(&mut input_bytes, 64, std::mem::size_of::<ArtifactInput>())?;
                let id = r.artifact_id;
                let release = r.release_id;
                let alignment = r.alignment;
                input.artifacts.push(ArtifactInput {
                    artifact_id: cpg_schema::Id(
                        id.try_into().map_err(|_| corrupt("artifact identity"))?,
                    ),
                    release_id: cpg_schema::Id(
                        release
                            .try_into()
                            .map_err(|_| corrupt("release identity"))?,
                    ),
                    alignment: serde_json::from_value(alignment.into())
                        .map_err(|_| corrupt("alignment"))?,
                });
            }
        }
        lease.complete();
        drop(lease);
        let selection_copy = selection.clone();
        let candidates = self
            .run_cpu(move || Ok(PreparedCatalog::new(&input)?.classify(&selection_copy)?))
            .await?;
        let promoted = candidates
            .iter()
            .filter(|c| c.access_path == query.trim() && allowed(selection.mode, c.outcome))
            .map(|c| c.member_id)
            .collect();
        Ok(PreparedSelection {
            generation: generation.generation(),
            hash: cpg_schema::IdHasher::new("prepared-selection-v1")
                .str(&selection::selection_digest(selection)?)
                .str(query)
                .finish_digest()
                .hex(),
            selection: selection.clone(),
            candidates: std::sync::Arc::new(candidates),
            promoted,
        })
    }
}

#[derive(sqlx::FromRow)]
struct UnitRankRow {
    #[sqlx(rename = "member_id!")]
    member_id: Vec<u8>,
    family: String,
    unit_id: Vec<u8>,
    fragment_id: Vec<u8>,
    #[sqlx(rename = "score!")]
    score: f64,
    #[sqlx(rename = "rank!")]
    rank: i64,
}
impl ServingStore {
    pub async fn selection_vectors(
        &self,
        generation: &PinnedGeneration,
        prepared: &PreparedSelection,
        query: &[f32],
        spec: &str,
    ) -> Result<Vec<u8>, Error> {
        use cpg_schema::retrieval::*;
        prepared.check(generation)?;
        crate::retrieval::validate_query(query)?;
        if generation.manifest().spec_hash.as_deref() != Some(spec) {
            return Err(Error::Request(
                "query embedding spec differs from generation".into(),
            ));
        }
        if generation.policy.route != crate::profiles::Route::Exact {
            return Err(Error::Request(
                "addressable retrieval requires a separately admitted profile; exact is selected"
                    .into(),
            ));
        }
        let eligible = prepared
            .eligible()
            .into_iter()
            .map(|m| m.storage().0.to_vec())
            .collect::<Vec<_>>();
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let generation_id = generation.id();
        let rows: Vec<UnitRankRow> = sqlx::query_file_as!(
            UnitRankRow,
            "queries/unit_ranks.sql",
            generation_id.0.as_slice(),
            &eligible,
            query
        )
        .fetch_all(&mut *lease.connection)
        .await?;
        lease.complete();
        drop(lease);
        if rows.len() > 200_000 {
            return Err(refused("rank row budget").into());
        }
        let rows = rows
            .into_iter()
            .map(|r| {
                let id = |bytes: Vec<u8>| {
                    bytes
                        .try_into()
                        .map(cpg_schema::Id)
                        .map_err(|_| corrupt("unit rank identity"))
                };
                Ok(Winner {
                    member_id: PublicMemberId::from_storage(id(r.member_id)?),
                    family: serde_json::from_value(r.family.into())
                        .map_err(|_| corrupt("rank family"))?,
                    channel: Channel::Vector,
                    unit_id: RetrievalUnitId::from_storage(id(r.unit_id)?),
                    fragment_id: id(r.fragment_id)?,
                    rank: r.rank.try_into().map_err(|_| corrupt("rank ordinal"))?,
                    score: r.score,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        encode_winners(&rows)
    }
    pub async fn finish_selection_search(
        &self,
        generation: &PinnedGeneration,
        prepared: &PreparedSelection,
        ranked: Vec<cpg_schema::retrieval::RankedMember>,
        channel_state: &str,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        use std::collections::BTreeMap;
        prepared.check(generation)?;
        if ranked.len() > 200_000 || channel_state.len() > 500 {
            return Err(refused("ranked assembly budget").into());
        }
        let prepared_copy = prepared.clone();
        let (ranked, tuples) = self
            .run_cpu(move || {
                let prepared = &prepared_copy;
                let eligible: BTreeSet<_> = prepared.eligible().into_iter().collect();
                let mut tuples = BTreeSet::new();
                let mut members = BTreeSet::new();
                for r in &ranked {
                    if !r.score.is_finite()
                        || !eligible.contains(&r.member_id)
                        || !members.insert(r.member_id)
                        || r.winners.len() > 8
                        || r.promoted != prepared.promoted.contains(&r.member_id)
                    {
                        return Err(corrupt("ranked selection membership or score").into());
                    }
                    let mut channels = BTreeSet::new();
                    for w in &r.winners {
                        if w.member_id != r.member_id
                            || !w.score.is_finite()
                            || w.rank == 0
                            || !channels.insert((w.family, w.channel))
                        {
                            return Err(corrupt("winning channel contract").into());
                        }
                        tuples.insert((w.member_id, w.family, w.unit_id, w.fragment_id));
                    }
                }
                let all_winners = ranked
                    .iter()
                    .flat_map(|r| r.winners.iter().cloned())
                    .collect::<Vec<_>>();
                let expected = cpg_schema::retrieval::channel_winners(all_winners.clone())?;
                let expected: BTreeMap<_, _> = expected
                    .iter()
                    .map(|w| {
                        (
                            (w.member_id, w.family, w.channel),
                            (w.rank, w.unit_id, w.fragment_id),
                        )
                    })
                    .collect();
                if all_winners.iter().any(|w| {
                    expected.get(&(w.member_id, w.family, w.channel))
                        != Some(&(w.rank, w.unit_id, w.fragment_id))
                }) {
                    return Err(
                        corrupt("channel ranks are incomplete or incorrectly ordered").into(),
                    );
                }
                let oracle = cpg_schema::retrieval::fuse(&all_winners, &prepared.promoted);
                if oracle.len() != ranked.len()
                    || oracle.iter().zip(&ranked).any(|(a, b)| {
                        a.member_id != b.member_id || (a.score - b.score).abs() > 1e-12
                    })
                {
                    return Err(corrupt("family fusion policy or ordering differs").into());
                }
                Ok((ranked, tuples))
            })
            .await?;
        let members = tuples
            .iter()
            .map(|t| t.0.storage().0.to_vec())
            .collect::<Vec<_>>();
        let families = tuples
            .iter()
            .map(|t| t.1.name().to_owned())
            .collect::<Vec<_>>();
        let units = tuples
            .iter()
            .map(|t| t.2.storage().0.to_vec())
            .collect::<Vec<_>>();
        let fragments = tuples.iter().map(|t| t.3.0.to_vec()).collect::<Vec<_>>();
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let generation_id = generation.id();
        let row = sqlx::query_file!(
            "queries/winner_membership.sql",
            generation_id.0.as_slice(),
            &members,
            &families,
            &units,
            &fragments
        )
        .fetch_one(&mut *lease.connection)
        .await?;
        lease.complete();
        drop(lease);
        if !row.valid {
            return Err(corrupt("winning unit belongs to another member or family").into());
        }
        let prepared = prepared.clone();
        let generation = generation.clone();
        let channel_state = channel_state.to_owned();
        let cursor = cursor.map(str::to_owned);
        self.run_cpu(move || {
        let mut page=prepared.clone();let map:BTreeMap<_,_>=ranked.iter().enumerate().map(|(i,r)|(r.member_id,(i,r))).collect();
        let mut candidates=page.candidates.iter().filter(|c|map.contains_key(&c.member_id)).cloned().collect::<Vec<_>>();candidates.sort_by_key(|c|map[&c.member_id].0);
        for candidate in &mut candidates {candidate.ranking=Some(map[&candidate.member_id].1.clone());}
        page.candidates=std::sync::Arc::new(candidates);
        page.hash=cpg_schema::IdHasher::new("ranked-selection-cursor-v1").str(&prepared.hash).str(&channel_state).str(&generation.profile.hex()).finish_digest().hex();
        let mut result=page.page(&generation,limit,cursor.as_deref())?;result["contradicted_count"]=(prepared.candidates.iter().filter(|c|c.outcome==SelectionOutcome::Contradicted).count() as u64).into();result["ranked_discovery"]=true.into();result["retrieval"]=serde_json::json!({"profile":generation.profile.hex(),"requested_route":"exact","actual_route":if channel_state.starts_with("vector:"){"exact"}else{"lexical-only"},"routing_reason":"addressable_family_rrf_v1"});
        let typed:SelectionResults=serde_json::from_value(result).map_err(|_|corrupt("ranked selection response contract"))?;let value=serde_json::to_value(typed).map_err(|_|corrupt("ranked response encode"))?;crate::repository::check_response(&value)?;Ok(value)
        }).await
    }
}
pub fn encode_winners(rows: &[cpg_schema::retrieval::Winner]) -> Result<Vec<u8>, Error> {
    use arrow_array::{FixedSizeBinaryArray, Float64Array, RecordBatch, StringArray, UInt32Array};
    use std::sync::Arc;
    let schema = cpg_schema::retrieval::rank_schema();
    let mut bytes = Vec::new();
    {
        let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut bytes, &schema)
            .map_err(|_| corrupt("rank IPC"))?;
        for rows in rows.chunks(1024) {
            let ids = |values: Vec<Vec<u8>>| {
                FixedSizeBinaryArray::try_from_iter(values.iter().map(Vec::as_slice))
                    .map_err(|_| corrupt("rank identity"))
            };
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(ids(rows
                        .iter()
                        .map(|r| r.member_id.storage().0.to_vec())
                        .collect())?),
                    Arc::new(StringArray::from_iter_values(
                        rows.iter().map(|r| r.family.name()),
                    )),
                    Arc::new(StringArray::from(vec!["vector"; rows.len()])),
                    Arc::new(ids(rows
                        .iter()
                        .map(|r| r.unit_id.storage().0.to_vec())
                        .collect())?),
                    Arc::new(ids(rows
                        .iter()
                        .map(|r| r.fragment_id.0.to_vec())
                        .collect())?),
                    Arc::new(UInt32Array::from_iter_values(rows.iter().map(|r| r.rank))),
                    Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.score))),
                ],
            )
            .map_err(|_| corrupt("rank batch"))?;
            writer
                .write(&batch)
                .map_err(|_| corrupt("rank IPC write"))?;
        }
        writer.finish().map_err(|_| corrupt("rank IPC finish"))?;
    }
    if bytes.len() > crate::retrieval::RANK_BYTES {
        return Err(refused("rank IPC bytes").into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod input_budget_tests {
    use super::*;
    #[test]
    fn retained_selection_inputs_share_one_bound() {
        let mut bytes = 64 * 1024 * 1024 - 131;
        charge_input(&mut bytes, 1, 0).unwrap();
        assert!(
            charge_input(&mut bytes, 0, 0)
                .unwrap_err()
                .to_string()
                .contains("retained input byte budget")
        );
        let mut bytes = 0;
        assert!(charge_input(&mut bytes, usize::MAX, 0).is_err());
    }
}
