//! Native indexed candidate collection; Rust only fuses its bounded contextual witnesses.
use lctx_model::domain::{
    Id, ModelError, Record,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    embedding::EmbeddingSpec,
    embedding::{projection::ProjectionDefinition, value::FullValue},
    retrieval::{ContentPart, Family, Origin, OriginalAnchor, SearchWindow, Unit, WindowBinding},
    serving::{SnapshotHandle, ranking::*},
};
use lctx_surrealdb::NativeReader;
use lctx_model::domain::{HeapSize,charged::StateCharge,resources::ResourceBudget};
mod state;
pub use state::SearchRows;
use state::{native_heap,object_heap,id_heap,admit_object};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Object, RecordId, SerdeWrapper, SurrealValue, Value, Variables};
use std::collections::{BTreeMap, BTreeSet};
use lctx_surrealdb::ordered_rows::{OrderedRows, SortedRows};
fn binding(value: impl Serialize) -> Result<Value, ModelError> {
    lctx_surrealdb::loader::json_value(serde_json::to_value(value).map_err(ModelError::codec)?)
}

/// A native query witness over an explicit compiler-owned view scope. This contains no
/// publication identity or reader grant; published retrieval stamps its validated handle.
#[derive(Debug, Clone)]
pub struct ScopedCandidate {
    pub occurrence: Occurrence,
    pub channel: Channel,
    pub channel_identity: lctx_model::domain::serving::identity::ChannelIdentity,
    pub score: Option<f64>,
}
impl ScopedCandidate {
    fn published(self, snapshot: &SnapshotHandle) -> CandidateScore {
        CandidateScore {
            snapshot: snapshot.clone(),
            occurrence: self.occurrence,
            channel: self.channel,
            channel_identity: self.channel_identity,
            score: self.score,
        }
    }
    fn from_published(row: &CandidateScore) -> Self {
        Self {
            occurrence: row.occurrence,
            channel: row.channel,
            channel_identity: row.channel_identity,
            score: row.score,
        }
    }
}

impl SearchRows<ScopedCandidate> {
    fn published(self,snapshot:&SnapshotHandle)->Result<SearchRows<CandidateScore>,ModelError>{
        let heap=snapshot.database.namespace.as_str().len().checked_add(snapshot.database.database.as_str().len()).ok_or(ModelError::Schema("search publication-name allocation overflow"))?;
        self.map_with_heap(heap,|row|Ok(row.published(snapshot)))
    }
}

/// The eligibility policy is applied before each channel's candidate cap.
#[derive(Clone, Copy)]
pub enum UnitScope {
    All,
    BriefOrigins,
}
// Candidate rows and unique point-source IDs spill to immutable ordered runs. Only the
// existing 1024-document and 1024-target ranking frontier remains in memory.
const POINT_BATCH: usize = 128;
const RANKED_FRONTIER: usize = 1024;
fn field(object: &Object, name: &str) -> Result<Value, ModelError> {
    object.get(name).cloned().ok_or(ModelError::Schema("search occurrence field"))
}
fn object(value: Value) -> Result<Object, ModelError> {
    match value { Value::Object(row) => Ok(row), _ => Err(ModelError::Schema("search native row")) }
}
fn row_id<T>(row:&Object,name:&str)->Result<Id<T>,ModelError>{
    let Some(Value::Array(values))=row.get(name)else{return Err(ModelError::Schema("search nominal identity"));};
    if values.len()!=16{return Err(ModelError::Schema("search nominal identity width"));}
    let mut bytes=[0u8;16];for (byte,value) in bytes.iter_mut().zip(values.iter()){*byte=u8::from_value(value.clone()).map_err(ModelError::codec)?;}
    Id::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new(bytes.into_iter())).map_err(ModelError::codec)
}
fn optional_id<T>(row:&Object,name:&str)->Result<lctx_model::domain::serving::Nullable<Id<T>>,ModelError>{
    match row.get(name){Some(Value::Null|Value::None)=>Ok(lctx_model::domain::serving::Nullable(None)),Some(_)=>row_id(row,name).map(|id|lctx_model::domain::serving::Nullable(Some(id))),None=>Err(ModelError::Schema("search nullable identity"))}
}
fn hit(row:&Object,score:f64)->Result<NativeHit,ModelError>{Ok(NativeHit{score,unit:row_id(row,"unit")?,window:row_id(row,"window")?,part:row_id(row,"part")?,binding:optional_id(row,"binding")?,context:row_id(row,"context")?,member:optional_id(row,"member")?,anchor:optional_id(row,"anchor")?})}
fn eligibility(inputs: &[[u8;16]], pairs: Option<&[([u8;16],[u8;16])]>, family: Family, member_mode: bool,budget:&ResourceBudget) -> Result<(Variables,StateCharge), ModelError> {
    let mut charge=StateCharge::new(budget,"native-search-query-bindings");
    charge.grow(inputs.len()*(2*128+3*size_of::<String>()+size_of::<Value>()+16*(size_of::<serde_json::Value>()+size_of::<Value>()))+pairs.map_or(0,|pairs|pairs.len())*(2*16*(size_of::<Value>()+size_of::<serde_json::Value>())+3*size_of::<Vec<Value>>())+4*(32+size_of::<Value>()+32))?;
    let mut vars = Variables::new();
    let input_keys = inputs.iter().map(|v| binding(v).map(|v| lctx_surrealdb::prepared::scope_string(&v))).collect::<Result<Vec<_>,_>>()?;
    vars.insert("input_keys", binding(input_keys)?);
    vars.insert("family", family as i16);
    vars.insert("pairs", binding(pairs)?);
    vars.insert("member_mode", member_mode);
    Ok((vars,charge))
}
const ELIGIBLE: &str = "eligible=true AND family=$family AND scope_input IN $input_keys AND ($member_mode=false OR (member!=NULL AND binding!=NULL)) AND ($pairs=NULL OR [member,context] IN $pairs)";
async fn selected_frontier<Context>(reader: &NativeReader<Context>, table: &str, predicate: &str, vars: Variables, units: UnitScope,budget:&ResourceBudget)->Result<(OrderedRows, OrderedRows), ModelError> {
    occurrence_frontier(reader, lctx_surrealdb::derived_search::selected_occurrences(reader,table,predicate,vars,budget)?, units, budget).await
}
async fn occurrence_frontier<Context>(reader: &NativeReader<Context>, mut stream: lctx_surrealdb::reader::NativeRows, units: UnitScope, budget: &ResourceBudget) -> Result<(OrderedRows, OrderedRows), ModelError> {
    let result=async {
        let mut occurrences=SortedRows::with_budget(budget)?;
        let mut documents=SortedRows::with_budget(budget)?;
        loop {
            let mut batch=SearchRows::new(budget);
            while batch.len()<POINT_BATCH {let Some(row)=stream.next().await? else{break;};let row=object(row)?;let heap=object_heap(&row)?;batch.push(row,heap)?;}
            if batch.is_empty(){break;}
            let mut scratch=StateCharge::new(budget,"native-search-occurrence-window");
            // Each ID/set/tuple is bounded by the already populated physical window.
            scratch.grow(batch.len()*(size_of::<[u8;16]>()+32)*4)?;
            let mut briefs=BTreeSet::new();
            if matches!(units,UnitScope::BriefOrigins){
                let ids=batch.iter().map(|row|hit(row,0.0).map(|hit|*hit.unit.bytes())).collect::<Result<BTreeSet<_>,_>>()?;
                let units=state::records::<Unit,_>(reader,ids.into_iter().collect(),budget).await?;
                let origins=units.iter().map(|unit|*unit.origin.bytes()).collect::<BTreeSet<_>>();
                let origins=state::records::<Origin,_>(reader,origins.into_iter().collect(),budget).await?;
                let origins=origins.iter().filter(|origin|matches!(origin,Origin::Brief{..})).map(Record::id).collect::<BTreeSet<_>>();
                briefs.extend(units.iter().filter(|unit|origins.contains(&unit.origin)).map(Record::id));
            }
            for row in batch.rows {
                let mut copy=StateCharge::new(budget,"native-search-occurrence-projection");admit_object(&mut copy,&row)?;
                if matches!(units,UnitScope::BriefOrigins)&&!briefs.contains(&hit(&row,0.0)?.unit){continue;}
                let id=RecordId::from_value(field(&row,"in")?).map_err(ModelError::codec)?;
                let mut source=Object::new();source.insert("id",id);documents.push(Value::Object(source))?;occurrences.push(Value::Object(row))?;
            }
        }
        Ok((occurrences.finish()?,documents.finish()?))
    }.await;
    let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("search occurrence stream drainage",stream.drain_transport().await);lctx_model::domain::completion::complete(result,terminal)
}
fn document_order(a: &(RecordId, f64), b: &(RecordId, f64)) -> std::cmp::Ordering {
    b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0))
}
async fn score_documents<Context>(reader:&NativeReader<Context>,mut documents:OrderedRows,vars:&Variables,sql:&str,budget:&ResourceBudget)->Result<SearchRows<(RecordId,f64)>,ModelError>{
    let mut best=SearchRows::new(budget);
    loop {
        let mut scratch=StateCharge::new(budget,"native-search-document-window");
        scratch.grow(POINT_BATCH*size_of::<RecordId>())?;let mut ids=Vec::with_capacity(POINT_BATCH);
        while ids.len()<POINT_BATCH {let Some(row)=documents.next_row()? else{break;};scratch.grow(native_heap(&row)?+size_of::<Value>())?;let id=RecordId::from_value(field(&object(row)?,"id")?).map_err(ModelError::codec)?;scratch.grow(id_heap(&id)?)?;ids.push(id);}
        if ids.is_empty(){break;}
        for (key,value) in vars.iter(){scratch.grow(key.len()+size_of::<Value>()+native_heap(value)?+32)?;}
        let mut vars=vars.clone();vars.insert("documents",ids);
        let mut stream=reader.stream_prepared(lctx_surrealdb::prepared::PreparedQuery::new(vars,vec![],vec![sql.to_owned()])?)?;
        let result=async {while let Some(row)=stream.next().await?{
            let row=object(row)?;let mut copy=StateCharge::new(budget,"native-search-document-projection");admit_object(&mut copy,&row)?;
            let id=RecordId::from_value(field(&row,"id")?).map_err(ModelError::codec)?;let score=f64::from_value(field(&row,"score")?).map_err(ModelError::codec)?;
            if !score.is_finite(){return Err(ModelError::Invalid("native search returned a nonfinite score".into()));}
            let heap=id_heap(&id)?;best.push((id,if score==0.0{0.0}else{score}),heap)?;
            best.rows.sort_unstable_by(document_order);
            if best.rows.len()>RANKED_FRONTIER{let (id,_)=best.rows.pop().expect("pop frontier");best.charge.release(id_heap(&id)?);}
        }Ok(())}.await;
        let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("search document score drainage",stream.drain_transport().await);lctx_model::domain::completion::complete(result,terminal)?;
    }Ok(best)
}
struct Witness { hit: NativeHit, exact: bool, key: String }
impl Witness {
    fn target(&self, member_mode: bool) -> ([u8;16],[u8;16]) {
        (if member_mode { self.hit.member.0.map(|id| *id.bytes()).unwrap_or(*self.hit.unit.bytes()) } else { *self.hit.unit.bytes() }, *self.hit.context.bytes())
    }
    fn rank(&self, other: &Self) -> std::cmp::Ordering {
        other.exact.cmp(&self.exact).then_with(|| other.hit.score.total_cmp(&self.hit.score))
    }
}
fn insert_witness(best:&mut BTreeMap<([u8;16],[u8;16]),Witness>,witness:Witness,member_mode:bool,charge:&mut StateCharge)->Result<(),ModelError>{
    let target=witness.target(member_mode);
    if let Some(previous)=best.get(&target){
        if witness.rank(previous).then_with(||witness.key.cmp(&previous.key)).is_lt(){charge.grow(witness.key.capacity())?;let old=best.insert(target,witness).expect("existing witness");charge.release(old.key.capacity());}
        return Ok(());
    }
    if best.len()==RANKED_FRONTIER{
        let worst=best.iter().max_by(|(ak,a),(bk,b)|a.rank(b).then_with(||ak.cmp(bk))).map(|(key,_)|*key).expect("nonempty frontier");
        if !witness.rank(&best[&worst]).then_with(||target.cmp(&worst)).is_lt(){return Ok(());}
        charge.grow(witness.key.capacity())?;let old=best.remove(&worst).expect("frontier witness");charge.release(old.key.capacity());best.insert(target,witness);return Ok(());
    }
    charge.grow(size_of::<([u8;16],[u8;16])>()+size_of::<Witness>()+32+witness.key.capacity())?;
    best.insert(target,witness);Ok(())
}
async fn rank_occurrences(mut rows:OrderedRows,documents:&[(RecordId,f64)],query:Option<&str>,member_mode:bool,cap:usize,budget:&ResourceBudget)->Result<SearchRows<NativeHit>,ModelError>{
    let mut winners=SearchRows::new(budget);
    for tier in [128,256,512,1024]{
        rows.rewind()?;let mut charge=StateCharge::new(budget,"native-search-context-witnesses");let mut best=BTreeMap::new();let mut consumed=0usize;
        while let Some(row)=rows.next_row()?{
            consumed+=1;if consumed.is_multiple_of(POINT_BATCH){tokio::task::yield_now().await;}
            let row=object(row)?;let mut scratch=StateCharge::new(budget,"native-search-ranking-row");admit_object(&mut scratch,&row)?;
            let source=RecordId::from_value(field(&row,"in")?).map_err(ModelError::codec)?;
            let exact=match query{Some(query)=>["exact_name","exact_path","exact_option"].into_iter().any(|name|row.get(name).is_some_and(|value|matches!(value,Value::String(value) if value==query))),None=>false};
            let Some(score)=documents.iter().take(tier).find(|(id,_)|id==&source).map(|(_,score)|*score).or_else(||exact.then_some(0.0))else{continue;};
            let witness=Witness{hit:hit(&row,score)?,exact,key:String::from_value(field(&row,"occurrence_key")?).map_err(ModelError::codec)?};insert_witness(&mut best,witness,member_mode,&mut charge)?;
        }
        charge.grow(best.len()*size_of::<(([u8;16],[u8;16]),Witness)>())?;let mut best=best.into_iter().collect::<Vec<_>>();best.sort_unstable_by(|(ak,a),(bk,b)|a.rank(b).then_with(||ak.cmp(bk)));
        winners=SearchRows::new(budget);for (_,row) in best{winners.push(row.hit,0)?;}if winners.len()>=cap{break;}
    }Ok(winners)
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
    units: UnitScope,
    cap: usize,
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<CandidateScore>, ModelError> {
    lexical_scoped(
        reader,
        query,
        family,
        inputs,
        pairs,
        member_mode,
        units,
        cap,
        policy,
        budget,
    )
    .await
    .and_then(|rows| rows.published(reader.handle()))
}
/// Query the exact explicit view scope without manufacturing a publication grant.
#[allow(
    clippy::too_many_arguments,
    reason = "The native kernel keeps eligibility and policy explicit"
)]
pub async fn lexical_scoped<Context>(
    reader: &NativeReader<Context>,
    query: &str,
    family: Family,
    inputs: &[[u8; 16]],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: UnitScope,
    cap: usize,
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<ScopedCandidate>, ModelError> {
    if !(1..=1024).contains(&cap) {
        return Err(ModelError::Invalid("native candidate target cap".into()));
    }
    policy.validate()?;
    let scope = lctx_surrealdb::lexical_stats::scope_identity(reader)?;
    let (vars,mut query_charge) = eligibility(inputs, pairs, family, member_mode,budget)?;
    for (key,value) in vars.iter(){query_charge.grow(key.len()+size_of::<Value>()+32+native_heap(value)?)?;}
    let mut vars = vars;
    query_charge.grow(query.len()+3*(64+32+size_of::<Value>()))?;
    vars.insert("lexical_scope", scope.hex());
    vars.insert("corpus_id", lctx_surrealdb::lexical_stats::corpus_id(scope, family as i16));
    vars.insert("query", query.to_owned());
    let family_table = match family {
        Family::ApiOptions => "search_api_options",
        Family::DocumentationDeployment => "search_documentation_deployment",
        Family::Scenario => "search_scenario",
        Family::Source => "search_source",
    };
    let nominations = lctx_surrealdb::derived_search::lexical_occurrences(reader, family_table, ELIGIBLE, vars.clone(), budget)?;
    let (occurrences, documents) = occurrence_frontier(reader, nominations, units, budget).await?;
    // Native indexed table nomination precedes exact membership. Frozen publication
    // statistics alone determine scores, including zero-IDF lexical matches. Exact
    // equality nominations remain independently eligible at score zero.
    let sql = r#"RETURN {
 LET $corpus=(SELECT * FROM ONLY $corpus_id);
 IF $corpus=NONE { THROW 'missing frozen lexical corpus'; };
 LET $query_terms=array::distinct(search::analyze('lctx_discovery',$query));
 LET $df=object::from_entries((SELECT term,df FROM lexical_term WHERE scope=$lexical_scope AND family=$family AND term IN $query_terms).map(|$v|[$v.term,$v.df]));
 LET $score_document=|$document| {
  LET $member=(SELECT * FROM lexical_member WHERE scope=$lexical_scope AND family=$family AND source=$document LIMIT 1)[0];
  IF $member=NONE { THROW 'missing frozen lexical document'; };
  LET $terms=$member.in.terms.filter(|$t|$t.term IN $query_terms);
  IF array::len($terms)=0 { RETURN NONE; };
  RETURN math::sum($terms.map(|$t| math::max([0.0,math::ln(($corpus.documents-($df[$t.term] ?? 0)+0.5)/(($df[$t.term] ?? 0)+0.5))]) * $t.tf * 2.5 / ($t.tf+1.5*(0.25+0.75*$t.dl/((<float>$corpus.total_length/$corpus.documents))))));
 };
 LET $ranked=SELECT id,$score_document(id) AS score FROM $documents;
 RETURN SELECT id,score FROM $ranked WHERE score!=NONE;
};"#;
    let scores = score_documents(reader, documents, &vars, sql,budget).await?;
    let rows = rank_occurrences(occurrences, &scores, Some(query), member_mode, cap,budget).await?;
    let binding = ChannelBinding::lexical(policy, query)?;

    rows.map(|row| candidate(row, family, binding, member_mode))
}
fn candidate(
    row: NativeHit,
    family: Family,
    binding: ChannelBinding,
    member_mode: bool,
) -> Result<ScopedCandidate, ModelError> {
    if !row.score.is_finite() {
        return Err(ModelError::Invalid(
            "native search returned a nonfinite score".into(),
        ));
    }
    Ok(ScopedCandidate {
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

/// Exact eligible-vector collection. Eligibility precedes similarity ordering and limits;
/// approximate candidate behavior is not part of this policy.
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
    units: UnitScope,
    cap: usize,
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<CandidateScore>, ModelError> {
    vector_scoped(
        reader,
        vector,
        specification,
        vector_digest,
        recipe_digest,
        projection,
        family,
        inputs,
        pairs,
        member_mode,
        units,
        cap,
        policy,
        budget,
    )
    .await
    .and_then(|rows| rows.published(reader.handle()))
}
/// Exact eligible-vector query over a compiler-owned view scope.
#[allow(
    clippy::too_many_arguments,
    reason = "The native kernel keeps vector and eligibility identities explicit"
)]
pub async fn vector_scoped<Context>(
    reader: &NativeReader<Context>,
    vector: &[f32],
    specification: lctx_model::domain::ContentHash,
    vector_digest: lctx_model::domain::ContentHash,
    recipe_digest: lctx_model::domain::ContentHash,
    projection: Id<ProjectionDefinition>,
    family: Family,
    inputs: &[[u8; 16]],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: UnitScope,
    cap: usize,
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<ScopedCandidate>, ModelError> {
    if !(1..=1024).contains(&cap) {
        return Err(ModelError::Invalid("native candidate target cap".into()));
    }
    if vector.len() != 1024 || vector.iter().any(|v| !v.is_finite()) {
        return Err(ModelError::Invalid("native query vector shape".into()));
    }
    let (vars,mut query_charge) = eligibility(inputs, pairs, family, member_mode,budget)?;
    for (key,value) in vars.iter(){query_charge.grow(key.len()+size_of::<Value>()+32+native_heap(value)?)?;}
    let (occurrences, documents) = selected_frontier(reader, "vec_occurs", ELIGIBLE, vars.clone(), units,budget).await?;
    let mut vars = vars;
    query_charge.grow(vector.len()*(size_of::<f32>()+size_of::<Value>())+2*(64+32+size_of::<Value>()))?;
    vars.insert("vector", vector.to_vec());
    vars.insert("encoder_hash", specification.hex());
    vars.insert("policy_key", projection.hex());
    let scores = score_documents(reader, documents, &vars, &vector_selection_sql(1024)?,budget).await?;
    let rows = rank_occurrences(occurrences, &scores, None, member_mode, cap,budget).await?;
    let binding = ChannelBinding::vector(
        policy,
        specification,
        vector_digest,
        recipe_digest,
        projection,
    )?;
    rows.map(|row| candidate(row, family, binding, member_mode))
}

/// Fetch the exact nominated frontier in coarse indexed batches, then hydrate each immutable
/// full winner once. Payload lifetime is one 64-row batch; only scalar scores survive it.
pub async fn rescore_union(
    reader: &NativeReader,
    query: &crate::QueryVector,
    nominated: &[CandidateScore],
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<CandidateScore>, ModelError> {
    if nominated.iter().any(|row| &row.snapshot != reader.handle()) {
        return Err(ModelError::Conflict("rescore publication identity"));
    }
    let mut rows=SearchRows::new(budget);
    for row in nominated {rows.push(ScopedCandidate::from_published(row),0)?;}
    rescore_union_scoped(reader, query, &rows, policy,budget)
        .await.and_then(|rows|rows.published(reader.handle()))
}
/// Re-score only the nominated exact scoped frontier; no publication identity is inferred.
pub async fn rescore_union_scoped<Context>(
    reader: &NativeReader<Context>,
    query: &crate::QueryVector,
    nominated: &[ScopedCandidate],
    policy: &RankingPolicy,
    budget:&ResourceBudget,
) -> Result<SearchRows<ScopedCandidate>, ModelError> {
    use std::collections::{BTreeMap, BTreeSet};
    if query.vector.len() != 4096 || query.vector.iter().any(|v| !v.is_finite()) {
        return Err(ModelError::Invalid("full query vector shape".into()));
    }
    let channel = ChannelBinding::vector(
        policy,
        query.spec,
        lctx_model::domain::embedding::value::value_digest(&query.vector),
        query.recipe.identity(),
        query.projection,
    )?;
    let mut retained=StateCharge::new(budget,"native-search-rescore-retained");
    retained.grow(nominated.len()*(size_of::<Occurrence>()+32))?;
    let occurrences=nominated.iter().map(|row|row.occurrence).collect::<BTreeSet<_>>();
    let member_mode=occurrences.first().is_some_and(|o|matches!(o.target,Target::Member{..}));
    if occurrences.iter().any(|o|matches!(o.target,Target::Member{..})!=member_mode){return Err(ModelError::Invalid("mixed rescore target modes".into()));}
    retained.grow(occurrences.len()*size_of::<Occurrence>())?;let occurrences=occurrences.into_iter().collect::<Vec<_>>();
    let mut keys=BTreeMap::new();
    for chunk in occurrences.chunks(POINT_BATCH){
        let mut scratch=StateCharge::new(budget,"native-search-rescore-window");
        // Six optional/fixed 16-byte IDs and the family scalar form each seven-field tuple.
        scratch.grow(chunk.len()*((7+6*16)*(size_of::<Value>()+size_of::<serde_json::Value>())+2*(size_of::<String>()+32+32)))?;
        let mut vars=reader.view_bindings();
        let window_keys=chunk.iter().map(|o|binding(o.window).map(|v|lctx_surrealdb::prepared::scope_string(&v))).collect::<Result<BTreeSet<_>,_>>()?;
        vars.insert("window_keys",binding(window_keys)?);
        vars.insert("units",binding(chunk.iter().map(|o|o.unit).collect::<BTreeSet<_>>())?);
        vars.insert("tuples",binding(chunk.iter().map(|o|serde_json::json!([o.unit,o.window,o.part,o.binding,o.context,o.anchor,o.family as i16])).collect::<Vec<_>>())?);
        vars.insert("encoder",query.spec.hex());vars.insert("policy",query.projection.hex());
        let predicate="eligible=true AND scope_window IN $window_keys AND unit IN $units AND [unit,window,part,binding,context,anchor,family] IN $tuples AND in.encoder_hash=$encoder AND in.policy_key=$policy AND in.family=family AND in.library_input=scope_input";
        let mut rows=lctx_surrealdb::derived_search::selected_occurrences(reader,"vec_occurs",predicate,vars,budget)?;
        let result=async{
            loop{
                let mut batch=SearchRows::new(budget);
                while batch.len()<POINT_BATCH{let Some(row)=rows.next().await?else{break;};let row=object(row)?;let heap=object_heap(&row)?;batch.push(row,heap)?;}
                if batch.is_empty(){break;}
                let mut copy=StateCharge::new(budget,"native-search-rescore-source-window");
                for row in &batch{admit_object(&mut copy,row)?;}
                copy.grow(batch.len()*(size_of::<RecordId>()+size_of::<Value>()+32))?;
                let sources=batch.iter().map(|row|RecordId::from_value(field(row,"in")?).map_err(ModelError::codec)).collect::<Result<BTreeSet<_>,_>>()?;
                let mut vars=Variables::new();vars.insert("documents",sources.into_iter().collect::<Vec<_>>());
                let full=state::values(reader,lctx_surrealdb::prepared::PreparedQuery::new(vars,vec![],vec!["SELECT id,full_key FROM $documents".into()])?,budget).await?;
                for row in &batch{
                    let source=RecordId::from_value(field(row,"in")?).map_err(ModelError::codec)?;
                    let source_row=full.iter().find(|value|value.get("id")==Some(&Value::RecordId(source.clone()))).ok_or(ModelError::Schema("nominated vector source"))?;
                    let Some(Value::String(full_key))=source_row.get("full_key")else{return Err(ModelError::Schema("nominated full key"));};
                    let family=SerdeWrapper::<Family>::from_value(field(row,"family")?).map_err(ModelError::codec)?.0;
                    let occurrence=candidate(hit(row,0.0)?,family,channel,member_mode)?.occurrence;
                    if !chunk.contains(&occurrence){return Err(ModelError::Conflict("rescore frontier changed exact lineage"));}
                    if let Some(old)=keys.get(&occurrence){if old!=full_key{return Err(ModelError::Conflict("window has competing full winners"));}}else{
                        retained.grow(size_of::<Occurrence>()+size_of::<String>()+32+full_key.len())?;keys.insert(occurrence,full_key.clone());
                    }
                }
            }Ok(())
        }.await;
        let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("rescore selected occurrence drainage",rows.drain_transport().await);lctx_model::domain::completion::complete(result,terminal)?;
    }
    for key in keys.values(){retained.grow(size_of::<String>()+32+key.len())?;}
    let full_keys=keys.values().cloned().collect::<BTreeSet<_>>();
    retained.grow(full_keys.len()*size_of::<String>())?;let full_keys=full_keys.into_iter().collect::<Vec<_>>();
    let mut scores=BTreeMap::new();let mut encoders:BTreeMap<Id<EmbeddingSpec>,EmbeddingSpec>=BTreeMap::new();
    let query_norm=query.vector.iter().map(|v|f64::from(*v).powi(2)).sum::<f64>().sqrt();
    for chunk in full_keys.chunks(64){
        let mut scratch=StateCharge::new(budget,"native-search-full-winner-window");scratch.grow(chunk.len()*size_of::<[u8;16]>())?;
        let ids=chunk.iter().map(|key|hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("full winner key"))).collect::<Result<Vec<[u8;16]>,_>>()?;
        let values=state::records::<FullValue,_>(reader,ids,budget).await?;
        if values.len()!=chunk.len()||values.iter().any(|full|!chunk.iter().any(|key|key==&full.id().hex())){return Err(ModelError::Schema("nominated full winner"));}
        scratch.grow(values.len()*(size_of::<Id<EmbeddingSpec>>()+32+size_of::<[u8;16]>()))?;
        let missing=values.iter().map(|v|v.encoder).filter(|id|!encoders.contains_key(id)).collect::<BTreeSet<_>>();
        let loaded=state::records::<EmbeddingSpec,_>(reader,missing.iter().map(|id|*id.bytes()).collect(),budget).await?;
        for encoder in &loaded{retained.grow(size_of::<Id<EmbeddingSpec>>()+size_of::<EmbeddingSpec>()+32+encoder.heap_bytes())?;encoders.insert(encoder.id(),encoder.clone());}
        for full in &values{
            let encoder=encoders.get(&full.encoder).ok_or(ModelError::Schema("nominated encoder"))?;full.verify_encoder(encoder)?;
            if encoder.service_hash!=query.spec||full.dimensions!=4096{return Err(ModelError::Conflict("nominated encoder differs from query"));}
            let _decoded=budget.reserve("native-search-full-vector",4096*size_of::<f32>())?;
            let decoded=lctx_model::domain::embedding::value::decode_vector(&full.bytes.0,4096).map_err(ModelError::Invalid)?;
            let dot=decoded.iter().zip(&query.vector).map(|(a,b)|f64::from(*a)*f64::from(*b)).sum::<f64>();let norm=decoded.iter().map(|v|f64::from(*v).powi(2)).sum::<f64>().sqrt()*query_norm;
            let score=dot/norm;if !score.is_finite(){return Err(ModelError::Invalid("nonfinite full winner rescore".into()));}
            retained.grow(size_of::<String>()+size_of::<f64>()+32+32)?;scores.insert(full.id().hex(),score);
        }
    }
    let mut output=SearchRows::new(budget);for (occurrence,key) in keys{output.push(ScopedCandidate{occurrence,channel:Channel::Vector,channel_identity:channel.identity(),score:Some(scores[&key])},0)?;}Ok(output)
}

/// The exact indexed selection used inside production aggregation, exposed for native EXPLAIN.
pub fn vector_selection_sql(tier: usize) -> Result<String, ModelError> {
    if ![128, 256, 512, 1024].contains(&tier) {
        return Err(ModelError::Invalid("unsupported candidate tier".into()));
    }
    Ok(format!(
        "SELECT id,vector::similarity::cosine(embedding,$vector) AS score FROM $documents WHERE encoder_hash=$encoder_hash AND policy_key=$policy_key AND family=$family AND library_input IN $input_keys ORDER BY score DESC,id ASC LIMIT {tier}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn occurrence(unit: u16, key: &str, document: &RecordId, exact: &str) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new("lex_occurs", key));
        row.insert("in", document.clone());
        let mut identity = [0;16]; identity[..2].copy_from_slice(&unit.to_be_bytes());
        row.insert("unit", binding(identity).unwrap());
        for name in ["window", "part", "context"] { row.insert(name, binding([1u8;16]).unwrap()); }
        for name in ["binding", "member", "anchor"] { row.insert(name, Value::Null); }
        row.insert("occurrence_key", key.to_owned());
        row.insert("exact_name", exact.to_owned());
        row.insert("exact_path", String::new()); row.insert("exact_option", String::new());
        Value::Object(row)
    }
    #[tokio::test]
    async fn selected_ranking_preserves_exact_promotion_and_primary_witness() {
        let matched = RecordId::new("search_source", "matched");
        let exact = RecordId::new("search_source", "not-text-matched");
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut rows = SortedRows::with_budget(&budget).unwrap();
        rows.push(occurrence(1,"later",&matched, "")).unwrap();
        rows.push(occurrence(1,"earlier",&matched, "")).unwrap();
        rows.push(occurrence(2,"exact",&exact, "requested")).unwrap();
        let rows = rank_occurrences(rows.finish().unwrap(), &[(matched,3.0)], Some("requested"), false, 1,&ResourceBudget::fixed(32<<20).unwrap()).await.unwrap();
        assert_eq!(rows.len(),2, "target cap ends tier expansion; it does not discard contextual winners");
        assert_eq!(&rows[0].unit.bytes()[..2], &2u16.to_be_bytes());
        assert_eq!(rows[0].score,0.0, "exact promotion has no fabricated lexical score");
        assert_eq!(&rows[1].unit.bytes()[..2], &1u16.to_be_bytes());
    }
    #[test]
    fn bounded_witness_frontier_can_reenter_after_eviction() {
        let document = RecordId::new("vector", "document");
        let mut best = BTreeMap::new();
        let budget=ResourceBudget::fixed(32<<20).unwrap();let mut charge=StateCharge::new(&budget,"test-witnesses");
        for unit in 0..=1024u16 {
            let row = object(occurrence(unit,"witness",&document, "")).unwrap();
            insert_witness(&mut best, Witness {hit: hit(&row,f64::from(unit)).unwrap(),exact:false,key:"witness".into()},false,&mut charge).unwrap();
        }
        assert_eq!(best.len(),1024);
        assert!(best.keys().all(|(unit,_)| unit[..2] != 0u16.to_be_bytes()));
        let row = object(occurrence(0,"better",&document, "")).unwrap();
        insert_witness(&mut best, Witness {hit:hit(&row,2000.0).unwrap(),exact:false,key:"better".into()},false,&mut charge).unwrap();
        assert_eq!(best.len(),1024);
        assert!(best.keys().any(|(unit,_)| unit[..2] == 0u16.to_be_bytes()));
        assert!(best.values().all(|row| row.hit.score != 1.0));
    }
    #[test]
    fn primary_witness_ties_use_occurrence_key() {
        let document = RecordId::new("vector", "document");
        let mut best = BTreeMap::new();
        let budget=ResourceBudget::fixed(32<<20).unwrap();let mut charge=StateCharge::new(&budget,"test-witnesses");
        for key in ["z", "a", "m"] {
            let row = object(occurrence(1,key,&document, "")).unwrap();
            insert_witness(&mut best, Witness {hit: hit(&row,1.0).unwrap(),exact:false,key:key.into()},false,&mut charge).unwrap();
        }
        assert_eq!(best.len(),1);
        assert_eq!(best.values().next().unwrap().key,"a");
    }
    fn populated_rows(budget:&ResourceBudget,count:u16)->OrderedRows{
        let document=RecordId::new("vector","document");let mut rows=SortedRows::with_budget(budget).unwrap();
        for unit in 0..count{rows.push(occurrence(unit,&format!("row{unit:04}"),&document,"")).unwrap();}rows.finish().unwrap()
    }
    #[tokio::test]
    async fn populated_ranking_refuses_growth_and_releases_request_state(){
        let owner=ResourceBudget::fixed(32<<20).unwrap();let rows=populated_rows(&owner,16);
        let document=RecordId::new("vector","document");let row=object(occurrence(0,"row0000",&document,"")).unwrap();
        let row_bytes=size_of::<Object>()+object_heap(&row).unwrap();
        let tiny=ResourceBudget::fixed(row_bytes+2*(size_of::<Witness>()+size_of::<([u8;16],[u8;16])>()+32+7)+64).unwrap();
        let error=rank_occurrences(rows,&[(document,1.0)],None,false,128,&tiny).await.unwrap_err();
        assert!(matches!(error,ModelError::Resource{..}));assert!(tiny.peak().unwrap()>row_bytes);assert_eq!(tiny.reserved(),0);assert_eq!(owner.reserved(),0);
    }
    #[test]
    fn cancelled_populated_ranking_releases_sorter_and_witnesses(){
        use std::{future::Future,task::{Context,Poll,Waker}};
        let budget=ResourceBudget::fixed(32<<20).unwrap();let rows=populated_rows(&budget,256);
        let documents=[(RecordId::new("vector","document"),1.0)];
        let mut future=Box::pin(rank_occurrences(rows,&documents,None,false,128,&budget));
        let mut context=Context::from_waker(Waker::noop());assert!(matches!(future.as_mut().poll(&mut context),Poll::Pending));
        assert!(budget.reserved()>0);drop(future);assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn malformed_ranking_row_releases_preceding_populated_state(){
        let budget=ResourceBudget::fixed(32<<20).unwrap();let mut sorter=SortedRows::with_budget(&budget).unwrap();
        let document=RecordId::new("vector","document");sorter.push(occurrence(1,"a",&document,"")).unwrap();
        let mut broken=Object::new();broken.insert("id",RecordId::new("lex_occurs","z"));sorter.push(Value::Object(broken)).unwrap();
        assert!(rank_occurrences(sorter.finish().unwrap(),&[(document,1.0)],None,false,128,&budget).await.is_err());assert_eq!(budget.reserved(),0);
    }
    #[tokio::test]
    async fn returned_ranked_frontier_keeps_request_charge_until_drop(){
        let budget=ResourceBudget::fixed(32<<20).unwrap();let rows=populated_rows(&budget,3);
        let result=rank_occurrences(rows,&[(RecordId::new("vector","document"),1.0)],None,false,1,&budget).await.unwrap();
        assert_eq!(result.len(),3);assert_eq!(budget.reserved(),result.charge.reserved());assert!(budget.reserved()>=result.len()*size_of::<NativeHit>());
        drop(result);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn published_long_names_refuse_before_clone_and_remain_charged_until_drop(){
        use lctx_model::domain::{ContentHash,serving::{DatabaseIdentity,Name}};
        let hash=ContentHash::of(b"long-name-publication");
        let snapshot=SnapshotHandle{publication:hash,semantic:hash,realization:hash,view:hash,service_generation:hash,definition_epoch:hash,database:DatabaseIdentity{namespace:Name::new("n".repeat(512)).unwrap(),database:Name::new("d".repeat(512)).unwrap()}};
        let populate=|budget:&ResourceBudget|{
            let mut rows=SearchRows::new(budget);
            for unit in 0..16 {
                let row=object(occurrence(unit,"witness",&RecordId::new("vector","document"),"")).unwrap();
                rows.push(candidate(hit(&row,1.0).unwrap(),Family::Source,ChannelBinding::lexical(&RankingPolicy::default(),"query").unwrap(),false).unwrap(),0).unwrap();
            }rows
        };
        let tiny=ResourceBudget::fixed(16*(size_of::<ScopedCandidate>()+size_of::<CandidateScore>()+256)).unwrap();
        assert!(matches!(populate(&tiny).published(&snapshot),Err(ModelError::Resource{..})));assert_eq!(tiny.reserved(),0);
        let budget=ResourceBudget::fixed(64<<10).unwrap();let published=populate(&budget).published(&snapshot).unwrap();
        assert_eq!(published.len(),16);assert!(budget.reserved()>=16*(size_of::<CandidateScore>()+1024));
        assert_eq!(budget.reserved(),published.charge.reserved());drop(published);assert_eq!(budget.reserved(),0);
    }

}
