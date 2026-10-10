//! Mechanical native lowering of model-owned serving scope intent.
//! Indexed graph candidates are streamed in finite windows, then intersected with exact
//! immutable selection. No server-side array represents a complete publication.
use crate::{NativeReader, batches::CanonicalBatches, prepared::PreparedQuery};
use lctx_model::domain::{ModelError, Record, input::Package, charged::StateCharge, completion::{Completion,complete}, resources::{Reservation,ResourceBudget}, serving_scope::{ServingEdge,ServingScopeProgram}};
use std::{collections::BTreeSet,sync::Arc};
use surrealdb::types::{Object,RecordId,SurrealValue,Value,Variables};
const WINDOW:usize=128;

struct ScopeRows {rows:Vec<Object>,charges:Vec<StateCharge>}
impl ScopeRows {
    fn new()->Self{Self{rows:Vec::new(),charges:Vec::new()}}
    fn append(&mut self,mut other:Self){self.rows.append(&mut other.rows);self.charges.append(&mut other.charges);}
    fn retain(&mut self,mut keep:impl FnMut(&Object)->bool){self.rows.retain(|row|keep(row));}
}
impl std::ops::Deref for ScopeRows {type Target=[Object];fn deref(&self)->&Self::Target{&self.rows}}
impl<'a> IntoIterator for &'a ScopeRows {type Item=&'a Object;type IntoIter=std::slice::Iter<'a,Object>;fn into_iter(self)->Self::IntoIter{self.rows.iter()}}
struct ScopeRowsIter{rows:std::vec::IntoIter<Object>,_charges:Vec<StateCharge>}
impl Iterator for ScopeRowsIter{type Item=Object;fn next(&mut self)->Option<Object>{self.rows.next()}}
impl IntoIterator for ScopeRows{type Item=Object;type IntoIter=ScopeRowsIter;fn into_iter(self)->Self::IntoIter{ScopeRowsIter{rows:self.rows.into_iter(),_charges:self.charges}}}

pub struct PreparedServingScope { program:Arc<ServingScopeProgram>,_charge:Box<dyn Reservation> }
fn edge_table(edge:ServingEdge)->&'static str {match edge {ServingEdge::Reference=>"reference",ServingEdge::Participant=>"participant"}}
fn record(row:&Object,field:&str)->Result<RecordId,ModelError>{RecordId::from_value(row.get(field).cloned().ok_or(ModelError::Schema("serving scope pointer"))?).map_err(ModelError::codec)}
fn relation(row:&Object)->Result<&str,ModelError>{match row.get("semantic_type"){Some(Value::String(value))=>Ok(value),_=>Err(ModelError::Schema("serving scope relation"))}}
fn body(row:&Object,field:&str)->Result<Value,ModelError>{row.get(field).cloned().ok_or(ModelError::Schema("serving companion field"))}

async fn values<C>(reader:&NativeReader<C>,query:PreparedQuery,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut rows=reader.stream_prepared(query)?;
    let mut result=Vec::new();let mut charge=StateCharge::new(budget,"serving-indexed-scope-window");
    let outcome=async {while let Some(value)=rows.next().await? {charge.grow(crate::loader::native_bytes(&value).saturating_mul(2).saturating_add(128))?;result.push(Object::from_value(value).map_err(ModelError::codec)?);}Ok(ScopeRows{rows:result,charges:vec![charge]})}.await;
    let mut terminal=Completion::default();terminal.step("serving scope window drainage",rows.drain_transport().await);complete(outcome,terminal)
}
async fn candidates<C>(reader:&NativeReader<C>,sql:String,bindings:Variables,preparation:Vec<String>,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    candidate_query(reader,PreparedQuery::new(bindings,preparation,vec![sql])?,budget).await
}
async fn candidate_query<C>(reader:&NativeReader<C>,query:PreparedQuery,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    // Only compact candidate metadata is transferred. Payloads remain point reads below.
    let mut rows=reader.stream_prepared(query)?;
    let mut accepted=Vec::new();let mut charge=StateCharge::new(budget,"serving-selected-candidates");
    let outcome=async {
        loop {let mut window=Vec::new();let mut window_charge=StateCharge::new(budget,"serving-candidate-handoff");while window.len()<WINDOW {let Some(value)=rows.next().await? else{break;};window_charge.grow(crate::loader::native_bytes(&value).saturating_mul(2).saturating_add(128))?;window.push(Object::from_value(value).map_err(ModelError::codec)?);}if window.is_empty(){break;}
            let ids=window.iter().map(|row|record(row,"id")).collect::<Result<Vec<_>,_>>()?;
            let members=reader.selected_candidate_ids(&ids,budget).await?;
            for row in window {if members.contains(&record(&row,"id")?) {let value=Value::Object(row);charge.grow(crate::loader::native_bytes(&value).saturating_mul(2).saturating_add(128))?;accepted.push(Object::from_value(value).map_err(ModelError::codec)?);}}
        }Ok(ScopeRows{rows:accepted,charges:vec![charge]})
    }.await;
    let mut terminal=Completion::default();terminal.step("serving candidate drainage",rows.drain_transport().await);complete(outcome,terminal)
}
async fn payloads<C>(reader:&NativeReader<C>,ids:&[RecordId],budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut result=ScopeRows::new();
    for window in ids.chunks(WINDOW){let mut bindings=Variables::new();bindings.insert("nodes",window.to_vec());result.append(values(reader,PreparedQuery::new(bindings,vec![],vec!["SELECT id,anchor,semantic_type,body.input AS input,body.library AS library,body.role AS role FROM $nodes".into()])?,budget).await?);}Ok(result)
}
async fn resolve<C>(reader:&NativeReader<C>,roots:&[RecordId],budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut ids=BTreeSet::new();let mut charge=StateCharge::new(budget,"serving-resolved-identities");
    for window in roots.chunks(WINDOW) {
        let mut vars=Variables::new();let mut statements=Vec::new();
        for (index,root) in window.iter().enumerate() {
            let name=format!("root{index}");vars.insert(name.clone(),root.clone());
            let sql=match root.table.as_str(){
                "entity_anchor"=>format!("SELECT id FROM entity WITH INDEX anchor_payload WHERE anchor=${name}"),
                "assertion_anchor"=>format!("SELECT id FROM assertion WITH INDEX anchor_payload WHERE anchor=${name}"),
                "entity"|"assertion"=>format!("SELECT id FROM ${name}"),
                _=>continue,
            };statements.push(sql);
        }
        if statements.is_empty(){continue;}
        for row in candidate_query(reader,PreparedQuery::new(vars,vec![],statements)?,budget).await? {let id=record(&row,"id")?;if !ids.contains(&id){charge.grow(768)?;ids.insert(id);}}
    }
    payloads(reader,&ids.into_iter().collect::<Vec<_>>(),budget).await
}
async fn related<'a,C>(reader:&NativeReader<C>,roots:impl IntoIterator<Item=&'a Object>,edge:ServingEdge,field:Option<&str>,target_relation:Option<&str>,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut ids=BTreeSet::new();let mut charge=StateCharge::new(budget,"serving-related-identities");
    let mut roots=roots.into_iter();
    loop {
        let mut vars=Variables::new();let mut statements=Vec::new();if let Some(field)=field {vars.insert("field",field.to_owned());}
        let predicate=if field.is_some(){" AND field=$field"}else{""};
        for index in 0..WINDOW {let Some(root)=roots.next()else{break;};let name=format!("anchor{index}");vars.insert(name.clone(),record(root,"anchor")?);
            statements.push(format!("SELECT in AS id,field FROM {} WITH INDEX incoming WHERE out=${name}{predicate}",edge_table(edge)));
        }
        if statements.is_empty(){break;}
        for row in candidate_query(reader,PreparedQuery::new(vars,vec![],statements)?,budget).await? {let id=record(&row,"id")?;if !ids.contains(&id){charge.grow(768)?;ids.insert(id);}}
    }
    let mut rows=payloads(reader,&ids.into_iter().collect::<Vec<_>>(),budget).await?;
    for row in &rows {relation(row)?;}
    rows.retain(|row|target_relation.is_none_or(|expected|relation(row).is_ok_and(|actual|expected==actual)));
    Ok(rows)
}
async fn outgoing_targets<C>(reader:&NativeReader<C>,roots:&ScopeRows,field:&str,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut result=ScopeRows::new();
    for roots in roots.chunks(WINDOW) {
        let mut vars=Variables::new();vars.insert("field",field.to_owned());let mut statements=Vec::new();
        for (index,root) in roots.iter().enumerate(){let name=format!("node{index}");vars.insert(name.clone(),record(root,"id")?);statements.push(format!("SELECT out AS id FROM participant WITH INDEX outgoing WHERE in=${name} AND field=$field"));}
        result.append(values(reader,PreparedQuery::new(vars,vec![],statements)?,budget).await?);
    }
    Ok(result)
}
async fn companion<C>(reader:&NativeReader<C>,kind:&str,field:&str,inputs:Vec<Value>,budget:&ResourceBudget)->Result<ScopeRows,ModelError>{
    let mut ids=BTreeSet::new();let mut charge=StateCharge::new(budget,"serving-companion-identities");
    for window in inputs.chunks(WINDOW) {let mut vars=Variables::new();vars.insert("kind",kind.to_owned());vars.insert("inputs",window.to_vec());
        let mut preparation=Vec::new();let predicate=crate::prepared::prepare_scope(&mut preparation,"scope","$kind",field,"$inputs");
        for row in candidates(reader,format!("SELECT id FROM assertion WITH INDEX by_scope WHERE semantic_type=$kind AND ({predicate})"),vars,preparation,budget).await? {let id=record(&row,"id")?;if !ids.contains(&id){charge.grow(768)?;ids.insert(id);}}
    }
    payloads(reader,&ids.into_iter().collect::<Vec<_>>(),budget).await
}
impl PreparedServingScope {
    pub fn new(program:Arc<ServingScopeProgram>,budget:&ResourceBudget)->Result<Arc<Self>,ModelError>{
        let bytes=program.inputs().iter().chain(program.incoming_inputs()).map(|input|input.name().len()+128).sum::<usize>()+program.owned_fields().iter().map(|field|field.len()+128).sum::<usize>();
        Ok(Arc::new(Self{program,_charge:budget.reserve("native-serving-scope-preparation",32768+bytes.saturating_mul(8))?}))
    }
    pub async fn frontier<C>(&self,reader:&NativeReader<C>,requested:Vec<RecordId>,budget:&ResourceBudget)->Result<Vec<RecordId>,ModelError>{
        let roots=resolve(reader,&requested,budget).await?;
        if requested.iter().any(|wanted|!roots.iter().any(|row|record(row,"id").is_ok_and(|id|&id==wanted)||record(row,"anchor").is_ok_and(|id|&id==wanted))) {return Err(ModelError::Conflict("scope root outside exact view"));}
        let mut next=BTreeSet::new();let mut charge=StateCharge::new(budget,"serving-scope-next-frontier");
        for edge in self.program.outgoing_edges() {
            for roots in roots.chunks(WINDOW) {
                let mut vars=Variables::new();let mut statements=Vec::new();
                for (index,root) in roots.iter().enumerate(){let name=format!("node{index}");vars.insert(name.clone(),record(root,"id")?);statements.push(format!("SELECT out AS id FROM {} WITH INDEX outgoing WHERE in=${name}",edge_table(*edge)));}
                let mut edges=reader.stream_prepared(PreparedQuery::new(vars,vec![],statements)?)?;
                let outcome=async {loop{let mut targets=Vec::new();while targets.len()<WINDOW {let Some(value)=edges.next().await? else{break;};targets.push(record(&Object::from_value(value).map_err(ModelError::codec)?,"id")?);}if targets.is_empty(){break;}
                    for row in resolve(reader,&targets,budget).await? {if self.program.inputs().iter().any(|input|relation(&row).is_ok_and(|actual|input.name()==actual)){let id=record(&row,"id")?;if !next.contains(&id){charge.grow(384)?;next.insert(id);}}}
                }Ok(())}.await;
                let mut terminal=Completion::default();terminal.step("serving outgoing traversal drainage",edges.drain_transport().await);complete(outcome,terminal)?;
                let mut vars=Variables::new();let mut statements=Vec::new();
                for (index,root) in roots.iter().enumerate(){let name=format!("anchor{index}");vars.insert(name.clone(),record(root,"anchor")?);statements.push(format!("SELECT in AS id,field FROM {} WITH INDEX incoming WHERE out=${name}",edge_table(*edge)));}
                let incoming=candidate_query(reader,PreparedQuery::new(vars,vec![],statements)?,budget).await?;
                for window in incoming.chunks(WINDOW) {let ids=window.iter().map(|row|record(row,"id")).collect::<Result<Vec<_>,_>>()?;
                    for row in payloads(reader,&ids,budget).await? {let id=record(&row,"id")?;let kind=relation(&row)?;let matches=window.iter().any(|edge_row|record(edge_row,"id").is_ok_and(|actual|actual==id)&&matches!(edge_row.get("field"),Some(Value::String(field)) if self.program.incoming(*edge,kind,field)));
                        if matches&&!next.contains(&id){charge.grow(384)?;next.insert(id);}
                    }
                }
            }
        }
        let capture=self.program.capture();let mut inputs=Vec::new();let mut sources=Vec::new();
        for root in &roots {let kind=relation(root)?;if capture.direct_input(kind){charge.grow(512)?;inputs.push(body(root,capture.source_input)?);}if capture.corpus_input(kind){charge.grow(512)?;sources.push(body(root,capture.source_input)?);}}
        let corpora=companion(reader,capture.corpus_relation,capture.corpus_input,sources,budget).await?;
        for corpus in &corpora {charge.grow(512)?;inputs.push(body(corpus,capture.corpus_library)?);if self.program.inputs().iter().any(|input|input.name()==capture.corpus_relation){let id=record(corpus,"id")?;if !next.contains(&id){charge.grow(384)?;next.insert(id);}}}
        if self.program.inputs().iter().any(|input|input.name()==capture.distribution_relation) {for row in companion(reader,capture.distribution_relation,capture.distribution_input,inputs,budget).await? {let id=record(&row,"id")?;if !next.contains(&id){charge.grow(384)?;next.insert(id);}}}
        Ok(next.into_iter().collect())
    }
    pub async fn hydrate<C>(&self,reader:&NativeReader<C>,requested:Vec<RecordId>,budget:&ResourceBudget)->Result<CanonicalBatches,ModelError>{
        let nodes=resolve(reader,&requested,budget).await?;
        if requested.iter().any(|wanted|!nodes.iter().any(|row|record(row,"id").is_ok_and(|id|&id==wanted)||record(row,"anchor").is_ok_and(|id|&id==wanted))) {return Err(ModelError::Conflict("scope hydration outside exact view"));}
        let types=self.program.inputs().iter().map(|input|input.name().to_owned()).collect::<Vec<_>>();
        let ids=nodes.iter().map(|row|record(row,"id")).collect::<Result<Vec<_>,_>>()?;
        reader.canonical_point_batches(&ids,&types,budget).await
    }
}
/// Resolve the library capture through the same indexed exact-view graph primitives.
pub async fn library_roots<C>(reader:&NativeReader<C>,name:Option<&str>,budget:&ResourceBudget)->Result<Vec<RecordId>,ModelError>{
    let mut vars=Variables::new();
    let query=if let Some(name)=name {
        let package=Package{name:name.to_owned()};
        vars.insert("key",hex::encode(package.id().bytes()));vars.insert("name",name.to_owned());
        "SELECT id FROM entity WITH INDEX semantic_key WHERE semantic_type='packages' AND semantic_key=$key AND body.name=$name"
    } else {"SELECT id FROM entity WITH INDEX semantic_key WHERE semantic_type='packages'"};
    let packages=candidates(reader,query.into(),vars,vec![],budget).await?;
    if packages.is_empty(){return Ok(Vec::new());}
    let ids=packages.iter().map(|row|record(row,"id")).collect::<Result<Vec<_>,_>>()?;
    let packages=payloads(reader,&ids,budget).await?;
    let releases=related(reader,&packages,ServingEdge::Reference,Some("package"),Some("releases"),budget).await?;
    let distributions=related(reader,&releases,ServingEdge::Participant,Some("release"),Some("input_distributions"),budget).await?;
    let mut distributions=distributions;distributions.retain(|row|body(row,"role").is_ok_and(|role|role==Value::Number(surrealdb::types::Number::Int(0))));
    // Canonical IDs are the graph endpoints; obtain inputs through their actual outgoing edges.
    let mut identity_charge=StateCharge::new(budget,"serving-library-identities");
    let mut input_ids=BTreeSet::new();
    for target in outgoing_targets(reader,&distributions,"input",budget).await? {let id=record(&target,"id")?;if !input_ids.contains(&id){identity_charge.grow(768)?;input_ids.insert(id);}}
    let inputs=resolve(reader,&input_ids.into_iter().collect::<Vec<_>>(),budget).await?;
    let corpora=related(reader,&inputs,ServingEdge::Participant,Some("library"),Some("corpus_libraries"),budget).await?;
    let mut corpus_ids=BTreeSet::new();
    for row in outgoing_targets(reader,&corpora,"corpus",budget).await? {let id=record(&row,"id")?;if !corpus_ids.contains(&id){identity_charge.grow(768)?;corpus_ids.insert(id);}}
    let corpus_inputs=resolve(reader,&corpus_ids.into_iter().collect::<Vec<_>>(),budget).await?;
    let runs=related(reader,inputs.iter().chain(&corpus_inputs),ServingEdge::Reference,Some("input"),Some("provider_runs"),budget).await?;
    let sources=related(reader,inputs.iter().chain(&corpus_inputs),ServingEdge::Reference,Some("input"),Some("source_artifacts"),budget).await?;
    let modules=related(reader,&sources,ServingEdge::Reference,Some("source"),Some("modules"),budget).await?;
    let mut scopes=ScopeRows::new();for field in ["input","release","artifact","module"] {scopes.append(related(reader,inputs.iter().chain(&corpus_inputs).chain(&releases).chain(&sources).chain(&modules),ServingEdge::Reference,Some(field),Some("coverage_scopes"),budget).await?);}
    let mut coverage=related(reader,&runs,ServingEdge::Participant,Some("run"),Some("provider_coverage"),budget).await?;
    coverage.append(related(reader,&scopes,ServingEdge::Participant,Some("scope"),Some("provider_coverage"),budget).await?);
    let mut output=BTreeSet::new();for row in distributions.iter().chain(&inputs).chain(&corpora).chain(&corpus_inputs).chain(&runs).chain(&coverage) {let id=record(row,"id")?;if !output.contains(&id){identity_charge.grow(768)?;output.insert(id);}}Ok(output.into_iter().collect())
}
