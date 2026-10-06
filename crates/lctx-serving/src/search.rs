//! Native indexed candidate collection; Rust only fuses its bounded contextual witnesses.
use lctx_model::domain::{ModelError, Id, attribution::AnalysisContext, catalog::CatalogMember,
    retrieval::{Family, Unit, Fragment, OriginalAnchor}, serving::{SnapshotHandle,ranking::*}};
use lctx_surrealdb::NativeReader;
use serde::{Serialize,Deserialize};
use surrealdb::types::{Variables,SerdeWrapper};

#[derive(Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeHit {
    score:f64,
    unit:Id<Unit>,
    fragment:Id<Fragment>,
    context:Id<AnalysisContext>,
    member:Option<Id<CatalogMember>>,
    anchor:Option<Id<OriginalAnchor>>,
}
fn family_table(family:Family)-> &'static str {
    match family {Family::ApiOptions=>"search_api_options",Family::DocumentationDeployment=>"search_documentation_deployment",Family::Scenario=>"search_scenario",Family::Source=>"search_source"}
}
pub async fn lexical(reader:&NativeReader, query:&str, family:Family, inputs:&[[u8;16]],
    members:Option<&[Id<CatalogMember>]>, member_mode:bool, cap:usize, policy:&RankingPolicy)->Result<Vec<CandidateScore>,ModelError> {
    let mut vars=Variables::new();
    vars.insert("query",query.to_owned());
    vars.insert("inputs",SerdeWrapper(inputs.to_vec()));
    vars.insert("members",SerdeWrapper(members.map(|m|m.to_vec())));
    vars.insert("cap",i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode",member_mode);
    let table=family_table(family);
    let target=if member_mode {"out"} else {"unit"};
    let sql=format!(r#"RETURN {{
 LET $documents=SELECT id,search::score(1) AS score FROM {table} WHERE text @1,OR@ $query AND search::score(1)>0 AND array::len(->lex_occurs[WHERE eligible=true AND input IN $inputs AND ($member_mode=false OR member!=NULL) AND ($members=NULL OR member IN $members)])>0 ORDER BY score DESC,id ASC LIMIT 100;
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $documents WHERE id=$parent.in)[0] AS score FROM lex_occurs WHERE eligible=true AND in IN $documents.id AND input IN $inputs AND ($members=NULL OR member IN $members);
 LET $best=SELECT {target} AS target,math::max(score) AS score FROM $occurrences GROUP BY target;
 RETURN SELECT VALUE (SELECT score,unit,fragment,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best WHERE score>0 ORDER BY score DESC,target ASC LIMIT $cap;
}};"#);
    let binding=ChannelBinding::lexical(policy,query)?;
    let rows:Vec<NativeHit>=reader.query(sql,vars).await?;
    rows.into_iter().map(|row|candidate(reader.handle(),row,family,binding,member_mode)).collect()
}
fn candidate(snapshot:&SnapshotHandle,row:NativeHit,family:Family,binding:ChannelBinding,member_mode:bool)->Result<CandidateScore,ModelError> {
    if !row.score.is_finite(){return Err(ModelError::Invalid("native search returned a nonfinite score".into()))}
    Ok(CandidateScore{snapshot:snapshot.clone(),occurrence:Occurrence{
        target:if member_mode {Target::Member{member:row.member.ok_or(ModelError::Schema("native member search occurrence"))?}} else {Target::Unit{unit:row.unit}},
        unit:row.unit,fragment:row.fragment,context:row.context,anchor:row.anchor,family},
        channel:binding.channel(),channel_identity:binding.identity(),score:Some(if row.score==0.0{0.0}else{row.score})})
}

/// Filter-aware HNSW candidate collection. The same occurrence predicate admits both
/// the vector and its winning contextual witness; an excluded nearest vector cannot crowd it out.
pub async fn vector(reader:&NativeReader, vector:&[f32], specification:lctx_model::domain::ContentHash,
    input_digest:lctx_model::domain::ContentHash,family:Family,inputs:&[[u8;16]],
    members:Option<&[Id<CatalogMember>]>,member_mode:bool,cap:usize,policy:&RankingPolicy)
    ->Result<Vec<CandidateScore>,ModelError> {
    if vector.len()!=1024 || vector.iter().any(|v| !v.is_finite()) {
        return Err(ModelError::Invalid("native query vector shape".into()));
    }
    let mut vars=Variables::new();
    vars.insert("vector",vector.to_vec());vars.insert("specification",SerdeWrapper(specification));
    vars.insert("family",family as i16);vars.insert("inputs",SerdeWrapper(inputs.to_vec()));
    vars.insert("members",SerdeWrapper(members.map(|m|m.to_vec())));
    vars.insert("cap",i64::try_from(cap).map_err(ModelError::codec)?);
    vars.insert("member_mode",member_mode);
    let target=if member_mode {"out"} else {"unit"};
    let sql=format!(r#"RETURN {{
 LET $vectors=SELECT id,1.0-vector::distance::knn() AS score FROM vector WHERE specification=$specification AND array::len(->vec_occurs[WHERE eligible=true AND family=$family AND input IN $inputs AND ($member_mode=false OR member!=NULL) AND ($members=NULL OR member IN $members)])>0 AND embedding <|100,200|> $vector;
 LET $occurrences=SELECT *, (SELECT VALUE score FROM $vectors WHERE id=$parent.in)[0] AS score FROM vec_occurs WHERE eligible=true AND family=$family AND in IN $vectors.id AND input IN $inputs AND ($member_mode=false OR member!=NULL) AND ($members=NULL OR member IN $members);
 LET $best=SELECT {target} AS target,math::max(score) AS score FROM $occurrences GROUP BY target;
 RETURN SELECT VALUE (SELECT score,unit,fragment,context,member,anchor FROM $occurrences WHERE {target}=$parent.target AND score=$parent.score ORDER BY occurrence_key LIMIT 1)[0] FROM $best ORDER BY score DESC,target ASC LIMIT $cap;
}};"#);
    let binding=ChannelBinding::vector(policy,specification,input_digest)?;
    let rows:Vec<NativeHit>=reader.query(sql,vars).await?;
    rows.into_iter().map(|row|candidate(reader.handle(),row,family,binding,member_mode)).collect()
}
