//! Native indexed candidate collection; Rust only fuses its bounded contextual witnesses.
use lctx_model::domain::{
    Id, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    retrieval::{Family, SearchWindow, ContentPart, WindowBinding, OriginalAnchor, Unit},
    embedding::{projection::ProjectionDefinition,value::FullValue},
    embedding::EmbeddingSpec, Record,
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
    window: Id<SearchWindow>,
    part: Id<ContentPart>,
    binding: lctx_model::domain::serving::Nullable<Id<WindowBinding>>,
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
    if !(1..=1024).contains(&cap) {return Err(ModelError::Invalid("native candidate target cap".into()));}
    let mut vars = Variables::new();
    vars.insert("query", query.to_owned());
    vars.insert("family", family as i16);
    vars.insert("inputs", binding(inputs)?);
    vars.insert("pairs", binding(pairs)?);
    vars.insert("cap", i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode", member_mode);
    vars.insert("units", binding(units)?);
    let table = family_table(family);
    let target = if member_mode { "out" } else { "unit" };
    let mut rows=Vec::new();
    for tier in [128,256,512,1024] {
        let sql=format!(r#"RETURN {{
 LET $input_keys=$inputs.map(|$v|<string>$v);
 LET $documents=SELECT id,search::score(1) AS score FROM {table} WHERE text @1,OR@ $query AND array::len((SELECT VALUE id FROM lex_occurs WHERE in=$parent.id AND eligible=true AND family=$family AND ($units=NULL OR unit IN $units) AND scope_input IN $input_keys AND ($member_mode=false OR (member!=NULL AND binding!=NULL)) AND ($pairs=NULL OR [member,context] IN $pairs) LIMIT 1))>0 ORDER BY score DESC,id ASC LIMIT {tier};
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $documents WHERE id=$parent.in)[0] ?? 0.0 AS score FROM lex_occurs WHERE eligible=true AND family=$family AND ($units=NULL OR unit IN $units) AND (in IN $documents.id OR exact_name=$query OR exact_path=$query OR exact_option=$query) AND scope_input IN $input_keys AND ($member_mode=false OR (member!=NULL AND binding!=NULL)) AND ($pairs=NULL OR [member,context] IN $pairs);
 LET $best=SELECT {target} AS target,context,in,math::max(score) AS score FROM $occurrences GROUP BY target,context,in;
 RETURN SELECT VALUE (SELECT score,unit,window,part,binding,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND context=$parent.context AND in=$parent.in AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best ORDER BY score DESC,target ASC,context ASC,in ASC LIMIT 1024;
}};"#);
        rows=reader.query::<Vec<NativeHit>>(sql,vars.clone()).await?;
        let targets=rows.iter().map(|row|(if member_mode {row.member.0.map(|id|*id.bytes()).unwrap_or(*row.unit.bytes())}else{*row.unit.bytes()},*row.context.bytes())).collect::<std::collections::BTreeSet<_>>();
        if targets.len()>=cap {break;}
    }
    let binding = ChannelBinding::lexical(policy, query)?;

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
            window: row.window,
            part: row.part,
            binding: row.binding.0,
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
    recipe_digest: lctx_model::domain::ContentHash,
    projection: Id<ProjectionDefinition>,
    family: Family,
    inputs: &[[u8; 16]],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: Option<&[Id<Unit>]>,
    cap: usize,
    policy: &RankingPolicy,
) -> Result<Vec<CandidateScore>, ModelError> {
    if !(1..=1024).contains(&cap) {return Err(ModelError::Invalid("native candidate target cap".into()));}
    if vector.len() != 1024 || vector.iter().any(|v| !v.is_finite()) {
        return Err(ModelError::Invalid("native query vector shape".into()));
    }
    let mut vars = Variables::new();
    vars.insert("vector", vector.to_vec());
    vars.insert("encoder_hash", specification.hex());
    vars.insert("policy_key", projection.hex());
    vars.insert("family", family as i16);
    let input_keys=inputs.iter().map(|v|binding(v).map(|v|lctx_surrealdb::reconciliation::scope_string(&v))).collect::<Result<Vec<_>,_>>()?;
    vars.insert("input_keys", binding(input_keys)?);
    vars.insert("inputs", binding(inputs)?);
    vars.insert("pairs", binding(pairs)?);
    vars.insert("cap", i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode", member_mode);
    vars.insert("units", binding(units)?);
    let target = if member_mode { "out" } else { "unit" };
    let mut rows=Vec::new();
    for tier in [128,256,512,1024] {
        let selection=vector_selection_sql(tier)?;
        let sql=format!(r#"RETURN {{
 LET $vectors={selection};
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $vectors WHERE id=$parent.in)[0] AS score FROM vec_occurs WHERE eligible=true AND ($units=NULL OR unit IN $units) AND family=$family AND in IN $vectors.id AND scope_input IN $input_keys AND ($member_mode=false OR (member!=NULL AND binding!=NULL)) AND ($pairs=NULL OR [member,context] IN $pairs);
 LET $best=SELECT {target} AS target,context,in,math::max(score) AS score FROM $occurrences GROUP BY target,context,in;
 RETURN SELECT VALUE (SELECT score,unit,window,part,binding,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND context=$parent.context AND in=$parent.in AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best ORDER BY score DESC,target ASC,context ASC,in ASC LIMIT 1024;
}};"#);
        rows=reader.query::<Vec<NativeHit>>(sql,vars.clone()).await?;
        let targets=rows.iter().map(|row|(if member_mode {row.member.0.map(|id|*id.bytes()).unwrap_or(*row.unit.bytes())}else{*row.unit.bytes()},*row.context.bytes())).collect::<std::collections::BTreeSet<_>>();
        if targets.len()>=cap {break;}
    }
    let binding=ChannelBinding::vector(policy,specification,vector_digest,recipe_digest,projection)?;
    rows.into_iter()
        .map(|row| candidate(reader.handle(), row, family, binding, member_mode))
        .collect()
}

#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FullLink { full_key:String, family:Family, witness:NativeHit }
/// Fetch the exact nominated frontier in coarse indexed batches, then hydrate each immutable
/// full winner once. Payload lifetime is one 64-row batch; only scalar scores survive it.
pub async fn rescore_union(reader:&NativeReader,query:&crate::QueryVector,nominated:&[CandidateScore],policy:&RankingPolicy)->Result<Vec<CandidateScore>,ModelError>{
    use std::collections::{BTreeMap,BTreeSet};
    if query.vector.len()!=4096 || query.vector.iter().any(|v|!v.is_finite()) {return Err(ModelError::Invalid("full query vector shape".into()));}
    let channel=ChannelBinding::vector(policy,query.spec,lctx_model::domain::embedding::value::value_digest(&query.vector),query.recipe.identity(),query.projection)?;
    let occurrences=nominated.iter().map(|row|row.occurrence).collect::<BTreeSet<_>>();
    let member_mode=occurrences.first().is_some_and(|o|matches!(o.target,Target::Member{..}));
    if occurrences.iter().any(|o|matches!(o.target,Target::Member{..})!=member_mode){return Err(ModelError::Invalid("mixed rescore target modes".into()));}
    let occurrences=occurrences.into_iter().collect::<Vec<_>>();
    let mut keys=BTreeMap::new();
    for chunk in occurrences.chunks(128) {
        let mut vars=Variables::new();
        let window_keys=chunk.iter().map(|o|binding(o.window).map(|v|lctx_surrealdb::reconciliation::scope_string(&v))).collect::<Result<BTreeSet<_>,_>>()?;
        vars.insert("window_keys",binding(window_keys)?);
        vars.insert("units",binding(chunk.iter().map(|o|o.unit).collect::<BTreeSet<_>>())?);
        vars.insert("tuples",binding(chunk.iter().map(|o|serde_json::json!([o.unit,o.window,o.part,o.binding,o.context,o.anchor,o.family as i16])).collect::<Vec<_>>())?);
        vars.insert("encoder",query.spec.hex());vars.insert("policy",query.projection.hex());
        let rows:Vec<FullLink>=reader.query("SELECT in.full_key AS full_key,family,{score:0.0,unit:unit,window:window,part:part,binding:binding,context:context,member:member,anchor:anchor} AS witness FROM vec_occurs WHERE eligible=true AND scope_window IN $window_keys AND unit IN $units AND [unit,window,part,binding,context,anchor,family] IN $tuples AND in.encoder_hash=$encoder AND in.policy_key=$policy AND in.family=family AND in.library_input=scope_input ORDER BY occurrence_key,in.full_key",vars).await?;
        for row in rows {
            let occurrence=candidate(reader.handle(),row.witness,row.family,channel,member_mode)?.occurrence;
            if !chunk.contains(&occurrence){return Err(ModelError::Conflict("rescore frontier changed exact lineage"));}
            if keys.insert(occurrence,row.full_key.clone()).is_some_and(|old|old!=row.full_key){return Err(ModelError::Conflict("window has competing full winners"));}
        }
    }
    let full_keys=keys.values().cloned().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    let mut scores=BTreeMap::new();let mut encoders=BTreeMap::new();
    let query_norm=query.vector.iter().map(|v|f64::from(*v).powi(2)).sum::<f64>().sqrt();
    for chunk in full_keys.chunks(64) {
        let ids=chunk.iter().map(|key|hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("full winner key"))).collect::<Result<Vec<[u8;16]>,_>>()?;
        let values=reader.records::<FullValue>(lctx_surrealdb::RecordSelection::Keys(ids)).await?;
        if values.len()!=chunk.len() || values.iter().map(|full|full.id().hex()).collect::<BTreeSet<_>>() != chunk.iter().cloned().collect(){return Err(ModelError::Schema("nominated full winner"));}
        let missing=values.iter().map(|v|v.encoder).filter(|id|!encoders.contains_key(id)).collect::<BTreeSet<_>>();
        for encoder in reader.records::<EmbeddingSpec>(lctx_surrealdb::RecordSelection::Keys(missing.iter().map(|id|*id.bytes()).collect())).await? {
            encoders.insert(encoder.id(),encoder);
        }
        for full in values {
            let encoder=encoders.get(&full.encoder).ok_or(ModelError::Schema("nominated encoder"))?;
            full.verify_encoder(encoder)?;
            if encoder.service_hash!=query.spec || full.dimensions!=4096 {return Err(ModelError::Conflict("nominated encoder differs from query"));}
            let values=lctx_model::domain::embedding::value::decode_vector(&full.bytes.0,4096).map_err(ModelError::Invalid)?;
            let dot=values.iter().zip(&query.vector).map(|(a,b)|f64::from(*a)*f64::from(*b)).sum::<f64>();
            let norm=values.iter().map(|v|f64::from(*v).powi(2)).sum::<f64>().sqrt()*query_norm;
            let score=dot/norm;
            if !score.is_finite(){return Err(ModelError::Invalid("nonfinite full winner rescore".into()));}
            scores.insert(full.id().hex(),score);
        }
    }
    Ok(keys.into_iter().map(|(occurrence,key)|CandidateScore{snapshot:reader.handle().clone(),occurrence,channel:Channel::Vector,channel_identity:channel.identity(),score:Some(scores[&key])}).collect())
}

/// The exact indexed selection used inside production aggregation, exposed for native EXPLAIN.
pub fn vector_selection_sql(tier:usize)->Result<String,ModelError>{
    if ![128,256,512,1024].contains(&tier) {return Err(ModelError::Invalid("unsupported candidate tier".into()));}
    Ok(format!(r"SELECT id,1.0-vector::distance::knn() AS score FROM vector WHERE encoder_hash=$encoder_hash AND policy_key=$policy_key AND family=$family AND library_input IN $input_keys AND array::len((SELECT VALUE id FROM vec_occurs WHERE in=$parent.id AND eligible=true AND ($units=NULL OR unit IN $units) AND family=$family AND scope_input IN $input_keys AND ($member_mode=false OR (member!=NULL AND binding!=NULL)) AND ($pairs=NULL OR [member,context] IN $pairs) LIMIT 1))>0 AND embedding <|{tier},{tier}|> $vector"))
}
