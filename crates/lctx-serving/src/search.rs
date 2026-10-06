//! Native indexed candidate collection; Rust only fuses its bounded contextual witnesses.
use lctx_model::domain::{
    Id, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    retrieval::{Family, Fragment, OriginalAnchor, Unit},
    serving::{SnapshotHandle, ranking::*},
};
use lctx_surrealdb::NativeReader;
use serde::{Deserialize, Serialize};
use surrealdb::types::{Value, Variables};
fn binding(value: impl Serialize) -> Result<Value, ModelError> {
    lctx_surrealdb::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeHit {
    score: f64,
    unit: Id<Unit>,
    fragment: Id<Fragment>,
    context: Id<AnalysisContext>,
    member: lctx_model::domain::serving::Nullable<Id<CatalogMember>>,
    anchor: lctx_model::domain::serving::Nullable<Id<OriginalAnchor>>,
}
fn family_table(family: Family) -> &'static str {
    match family {
        Family::ApiOptions => "search_api_options",
        Family::DocumentationDeployment => "search_documentation_deployment",
        Family::Scenario => "search_scenario",
        Family::Source => "search_source",
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Native lexical selection keeps exact eligibility, target mode, candidate cap and ranking policy explicit"
)]
pub async fn lexical(
    reader: &NativeReader,
    query: &str,
    family: Family,
    inputs: &[[u8; 16]],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: Option<&[Id<Unit>]>,
    cap: usize,
    policy: &RankingPolicy,
) -> Result<Vec<CandidateScore>, ModelError> {
    let mut vars = Variables::new();
    vars.insert("query", query.to_owned());
    vars.insert("inputs", binding(inputs)?);
    vars.insert("pairs", binding(pairs)?);
    vars.insert("cap", i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode", member_mode);
    vars.insert("units", binding(units)?);
    let table = family_table(family);
    let target = if member_mode { "out" } else { "unit" };
    let sql = format!(
        r#"RETURN {{
 LET $input_keys=$inputs.map(|$v|<string>$v);
 LET $documents=SELECT id,search::score(1) AS score FROM {table} WHERE text @1,OR@ $query AND search::score(1)>0 AND array::len((SELECT VALUE id FROM lex_occurs WHERE in=$parent.id AND eligible=true AND ($units=NULL OR unit IN $units) AND scope_input IN $input_keys AND ($member_mode=false OR member!=NULL) AND ($pairs=NULL OR [member,context] IN $pairs) LIMIT 1))>0 ORDER BY score DESC,id ASC LIMIT 100;
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $documents WHERE id=$parent.in)[0] AS score FROM lex_occurs WHERE eligible=true AND ($units=NULL OR unit IN $units) AND in IN $documents.id AND scope_input IN $input_keys AND ($member_mode=false OR member!=NULL) AND ($pairs=NULL OR [member,context] IN $pairs);
 LET $best=SELECT {target} AS target,math::max(score) AS score FROM $occurrences GROUP BY target;
 RETURN SELECT VALUE (SELECT score,unit,fragment,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best WHERE score>0 ORDER BY score DESC,target ASC LIMIT $cap;
}};"#
    );
    let binding = ChannelBinding::lexical(policy, query)?;
    let rows: Vec<NativeHit> = reader.query(sql, vars).await?;
    rows.into_iter()
        .map(|row| candidate(reader.handle(), row, family, binding, member_mode))
        .collect()
}
fn candidate(
    snapshot: &SnapshotHandle,
    row: NativeHit,
    family: Family,
    binding: ChannelBinding,
    member_mode: bool,
) -> Result<CandidateScore, ModelError> {
    if !row.score.is_finite() {
        return Err(ModelError::Invalid(
            "native search returned a nonfinite score".into(),
        ));
    }
    Ok(CandidateScore {
        snapshot: snapshot.clone(),
        occurrence: Occurrence {
            target: if member_mode {
                Target::Member {
                    member: row
                        .member
                        .0
                        .ok_or(ModelError::Schema("native member search occurrence"))?,
                }
            } else {
                Target::Unit { unit: row.unit }
            },
            unit: row.unit,
            fragment: row.fragment,
            context: row.context,
            anchor: row.anchor.0,
            family,
        },
        channel: binding.channel(),
        channel_identity: binding.identity(),
        score: Some(if row.score == 0.0 { 0.0 } else { row.score }),
    })
}

/// Filter-aware HNSW candidate collection. The same occurrence predicate admits both
/// the vector and its winning contextual witness; an excluded nearest vector cannot crowd it out.
#[allow(
    clippy::too_many_arguments,
    reason = "Native vector selection keeps exact vector identity, eligibility, target mode, cap and ranking policy explicit"
)]
pub async fn vector(
    reader: &NativeReader,
    vector: &[f32],
    specification: lctx_model::domain::ContentHash,
    vector_digest: lctx_model::domain::ContentHash,
    family: Family,
    inputs: &[[u8; 16]],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: Option<&[Id<Unit>]>,
    cap: usize,
    policy: &RankingPolicy,
) -> Result<Vec<CandidateScore>, ModelError> {
    if vector.len() != 1024 || vector.iter().any(|v| !v.is_finite()) {
        return Err(ModelError::Invalid("native query vector shape".into()));
    }
    let mut vars = Variables::new();
    vars.insert("vector", vector.to_vec());
    vars.insert("specification", binding(specification)?);
    vars.insert("family", family as i16);
    vars.insert("inputs", binding(inputs)?);
    vars.insert("pairs", binding(pairs)?);
    vars.insert("cap", i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode", member_mode);
    vars.insert("units", binding(units)?);
    let target = if member_mode { "out" } else { "unit" };
    let sql = format!(
        r#"RETURN {{
 LET $input_keys=$inputs.map(|$v|<string>$v);
 LET $vectors=SELECT id,1.0-vector::distance::knn() AS score FROM vector WHERE scope_specification=<string>$specification AND array::len((SELECT VALUE id FROM vec_occurs WHERE in=$parent.id AND eligible=true AND ($units=NULL OR unit IN $units) AND family=$family AND scope_input IN $input_keys AND ($member_mode=false OR member!=NULL) AND ($pairs=NULL OR [member,context] IN $pairs) LIMIT 1))>0 AND embedding <|100,200|> $vector;
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $vectors WHERE id=$parent.in)[0] AS score FROM vec_occurs WHERE eligible=true AND ($units=NULL OR unit IN $units) AND family=$family AND in IN $vectors.id AND scope_input IN $input_keys AND ($member_mode=false OR member!=NULL) AND ($pairs=NULL OR [member,context] IN $pairs);
 LET $best=SELECT {target} AS target,math::max(score) AS score FROM $occurrences GROUP BY target;
 RETURN SELECT VALUE (SELECT score,unit,fragment,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best ORDER BY score DESC,target ASC LIMIT $cap;
}};"#
    );
    let binding = ChannelBinding::vector(policy, specification, vector_digest)?;
    let rows: Vec<NativeHit> = reader.query(sql, vars).await?;
    rows.into_iter()
        .map(|row| candidate(reader.handle(), row, family, binding, member_mode))
        .collect()
}
