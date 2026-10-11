//! One current bounded checkpoint, never a receipt per row or page.
use super::*;
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Pass { Preflight, Declarations, Verify, Ready, Sealed }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {pub pass:Pass,pub after:Option<RecordId>}
impl Default for State{fn default()->Self{Self{pass:Pass::Preflight,after:None}}}
impl State{
    pub fn validate(&self)->Result<(),ModelError>{
        if self.after.as_ref().is_some_and(|id|id.table.as_str()!="native_history_checkpoint") || (self.after.is_some()&&!matches!(self.pass,Pass::Preflight|Pass::Verify)){return Err(ModelError::Conflict("native upgrade history cursor"));}Ok(())
    }
    pub fn next_pass(&mut self,pass:Pass){*self=Self{pass,..Self::default()};}
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeAdvance { pub revision: u64, pub phase: String, pub sealed: bool, pub published: bool }
#[derive(Debug, Clone)]
pub(crate) struct Progress { pub revision: i64, pub state: State, pub contract: UpgradeExecutionContract }
impl Progress {
    pub fn result(&self) -> UpgradeAdvance {
        UpgradeAdvance { revision: self.revision as u64, phase: format!("{:?}", self.state.pass).to_lowercase(), sealed: self.state.pass == Pass::Sealed, published: self.state.pass == Pass::Sealed }
    }
}
pub(crate) async fn read(client: &Surreal<Client>, contract: &UpgradeExecutionContract) -> Result<Option<Progress>, ModelError> {
    let mut response = client.query("SELECT * FROM $progress").bind(("progress", RecordId::new("native_upgrade_progress_v1", contract.migration.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let row = match rows.as_slice() { [] => return Ok(None), [row] => row, _ => return Err(ModelError::Conflict("native upgrade progress cardinality")) };
    let Some(Value::String(encoded)) = row.get("contract") else { return Err(ModelError::Schema("native upgrade progress contract")); };
    let previous: UpgradeExecutionContract = serde_json::from_str(encoded).map_err(ModelError::codec)?;
    if !contract.compatible(&previous) || row.get("execution") != Some(&Value::String(previous.execution.hex())) { return Err(ModelError::Conflict("native upgrade progress incompatible identity")); }
    let Some(Value::String(encoded)) = row.get("state") else { return Err(ModelError::Schema("native upgrade progress state")); };
    let state: State = serde_json::from_str(encoded).map_err(ModelError::codec)?;
    state.validate()?;
    let Some(Value::Number(surrealdb::types::Number::Int(revision))) = row.get("revision") else { return Err(ModelError::Schema("native upgrade progress revision")); };
    if *revision < 0 || row.get("sealed") != Some(&Value::Bool(state.pass == Pass::Sealed)) { return Err(ModelError::Conflict("native upgrade progress shape")); }
    Ok(Some(Progress { revision: *revision, state, contract: previous }))
}
/// The caller's host capability proves predecessor drainage. Native checks retain the exact
/// original operation and reject incompatible prefix reuse; no historical receipt is invented.
pub(crate) async fn acquire(client: &Surreal<Client>, contract: &UpgradeExecutionContract) -> Result<Progress, ModelError> {
    let mut bindings = base_bindings(contract)?;
    bindings.insert("initial", serde_json::to_string(&State::default()).map_err(ModelError::codec)?);
    let created=execute(client, "BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.generation!=$generation OR $installation.schema!=$source OR $installation.schema_version!=$source_version OR $installation.admission_open { THROW 'native upgrade progress authority'; }; LET $old=SELECT * FROM ONLY $progress FOR UPDATE; IF $old=NONE { CREATE $progress SET contract=$contract,execution=$execution,state=$initial,revision=0,sealed=false RETURN NONE; }; COMMIT;", bindings).await;
    let (actual,created)=read_after(client,contract,created).await?;
    let mut progress=match actual{Some(progress)=>progress,None=>{created?;return Err(ModelError::Conflict("native upgrade progress absent"));}};
    if progress.contract != *contract {
        let mut next = progress.state.clone();
        if progress.contract.verifier != contract.verifier && matches!(next.pass, Pass::Verify | Pass::Ready) { next.next_pass(Pass::Verify); }
        let mut bindings=base_bindings(contract)?;
        bindings.insert("previous", serde_json::to_string(&progress.contract).map_err(ModelError::codec)?);
        bindings.insert("revision", progress.revision);
        bindings.insert("next", serde_json::to_string(&next).map_err(ModelError::codec)?);
        let transferred=execute(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.generation!=$generation OR $installation.schema!=$source OR $installation.schema_version!=$source_version OR $installation.admission_open { THROW 'native upgrade successor authority'; }; LET $old=SELECT * FROM ONLY $progress FOR UPDATE; IF $old.contract!=$previous OR $old.revision!=$revision OR $old.sealed { THROW 'native upgrade successor changed'; }; UPDATE $progress SET contract=$contract,execution=$execution,state=$next,revision+=1 RETURN NONE; COMMIT;",bindings).await;
        let (actual,transferred)=read_after(client,contract,transferred).await?;
        let actual=actual.ok_or(ModelError::Conflict("native upgrade successor absent"))?;
        if actual.contract!=*contract || actual.revision!=progress.revision+1 || actual.state!=next{transferred?;return Err(ModelError::Conflict("native upgrade successor receipt mismatch"));}
        progress=actual;
    }
    Ok(progress)
}
pub(crate) fn base_bindings(contract: &UpgradeExecutionContract) -> Result<Variables, ModelError> {
    let mut bindings=Variables::new();
    bindings.insert("progress", RecordId::new("native_upgrade_progress_v1",contract.migration.hex()));
    bindings.insert("journal",RecordId::new("native_upgrade",contract.migration.hex()));
    bindings.insert("contract",serde_json::to_string(contract).map_err(ModelError::codec)?);
    bindings.insert("execution",contract.execution.hex()); bindings.insert("generation",contract.generation.hex());
    bindings.insert("source",contract.source.hex()); bindings.insert("source_version",contract.source_version); bindings.insert("target",contract.target.hex());
    Ok(bindings)
}
async fn execute(client:&Surreal<Client>,sql:&str,bindings:Variables)->Result<(),ModelError>{
    crate::control::checked_transaction(client.query(sql.to_owned()).bind(bindings).await.map_err(crate::loader::write_failure)?)?;
    Ok(())
}
pub(crate) async fn commit(client:&Surreal<Client>,current:&Progress,next:State,body:&str,bindings:Variables, observations:Vec<Object>)->Result<Progress,ModelError>{
    commit_with_ack(client,current,next,body,bindings,observations,false).await
}
pub(crate) async fn commit_with_ack(client:&Surreal<Client>,current:&Progress,next:State,body:&str,mut bindings:Variables, observations:Vec<Object>,discard_ack:bool)->Result<Progress,ModelError>{
    next.validate()?;
    for (key,value) in base_bindings(&current.contract)? { bindings.insert(key,value); }
    bindings.insert("revision",current.revision); bindings.insert("expected_state",serde_json::to_string(&current.state).map_err(ModelError::codec)?); bindings.insert("next",serde_json::to_string(&next).map_err(ModelError::codec)?);
    let (observed,observed_ids)=canonical_observations(observations)?;
    bindings.insert("observed",observed);bindings.insert("observed_ids",observed_ids);
    let sql=page_query(body);
    let result=execute(client,&sql,bindings).await;
    if result.is_ok() && !discard_ack{return Ok(Progress{revision:current.revision+1,state:next,contract:current.contract.clone()});}
    let result=if discard_ack && result.is_ok(){Err(ModelError::Conflict("qualification deliberately discarded page acknowledgement"))}else{result};
    // A failed acknowledgement is never blindly replayed. Read the one authoritative slot.
    let (actual,result)=read_after(client,&current.contract,result).await?;
    let actual=actual.ok_or(ModelError::Conflict("native upgrade page receipt absent"))?;
    if actual.revision==current.revision+1 && actual.state==next && actual.contract==current.contract { return Ok(actual); }
    if let Err(error)=result { return Err(error); }
    Err(ModelError::Conflict("native upgrade page receipt mismatch"))
}

/// Keep complete observed bodies and their native identities in exact correspondence.
/// A page remains one existing native window before duplicate normalization; conflicting
/// observations of one identity can never establish a valid input snapshot.
fn canonical_observations(observations:Vec<Object>)->Result<(Vec<Value>,Vec<RecordId>),ModelError>{
    let mut windows=crate::loader::NativeWindows::new(observations.into_iter().map(Value::Object).collect());
    let rows=windows.next().transpose()?.unwrap_or_default();
    if let Some(extra)=windows.next(){extra?;return Err(ModelError::Conflict("native upgrade observations exceed one bounded page"));}
    let mut unique=std::collections::BTreeMap::new();
    for value in rows{
        let Value::Object(row)=value else{unreachable!("observation windows contain objects")};
        let Some(Value::RecordId(id))=row.get("id")else{return Err(ModelError::Schema("native upgrade observed identity"));};
        match unique.entry(id.clone()){
            std::collections::btree_map::Entry::Vacant(entry)=>{entry.insert(row);},
            std::collections::btree_map::Entry::Occupied(entry)=>{if entry.get()!=&row{return Err(ModelError::Conflict("native upgrade conflicting observations"));}},
        }
    }
    let ids=unique.keys().cloned().collect();
    let rows=unique.into_values().map(Value::Object).collect();
    Ok((rows,ids))
}

pub(crate) fn page_query(body:&str)->String{
    format!("BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.generation!=$generation OR $installation.schema!=$source OR $installation.schema_version!=$source_version OR $installation.admission_open {{ THROW 'native upgrade page authority'; }}; LET $old=SELECT * FROM ONLY $progress FOR UPDATE; IF $old.contract!=$contract OR $old.execution!=$execution OR $old.revision!=$revision OR $old.state!=$expected_state OR $old.sealed {{ THROW 'native upgrade page revision'; }}; LET $journal_state=SELECT * FROM ONLY $journal FOR UPDATE; IF $journal_state.source!=$source OR $journal_state.target!=$target OR $journal_state.generation!=$generation OR $journal_state.phase NOT IN ['intent','declarations','verified'] {{ THROW 'native upgrade page original operation'; }}; LET $observed_expected=SELECT * FROM $observed ORDER BY id; LET $observed_actual=SELECT * FROM $observed_ids ORDER BY id; IF $observed_actual!=$observed_expected {{ THROW 'native upgrade page inputs changed'; }}; {body} UPDATE $progress SET state=$next,revision+=1 RETURN NONE; COMMIT;")
}

/// Keep a failed write's primary outcome if acknowledgement observation also fails.
pub(crate) async fn read_after(client:&Surreal<Client>,contract:&UpgradeExecutionContract,write:Result<(),ModelError>)->Result<(Option<Progress>,Result<(),ModelError>),ModelError>{
    match read(client,contract).await{
        Ok(actual)=>Ok((actual,write)),
        Err(observation)=>match write{
            Ok(())=>Err(observation),
            Err(primary)=>{let mut completion=lctx_model::domain::completion::Completion::default();completion.step("native upgrade acknowledgement observation",Err(observation));lctx_model::domain::completion::complete(Err(primary),completion)}
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn observed(key:&str,body:&str)->Object{
        let mut row=Object::new();row.insert("id",RecordId::new("native_history_checkpoint",key));row.insert("body",body);row
    }
    #[test]
    fn observations_preserve_native_ids_and_complete_bodies_with_duplicate_refusal(){
        let first=observed("first","source body");let second=observed("second","other body");
        let canonical=canonical_observations(vec![second.clone(),first.clone(),first.clone()]).unwrap();
        assert_eq!(canonical,canonical_observations(vec![first.clone(),second]).unwrap());
        assert_eq!(canonical.0.len(),2);
        for (row,id) in canonical.0.iter().zip(&canonical.1){let Value::Object(row)=row else{panic!("complete object expected")};assert_eq!(row.get("id"),Some(&Value::RecordId(id.clone())));assert!(row.contains_key("body"));}
        assert!(canonical_observations(vec![first,observed("first","changed body")]).is_err());
        let mut text_id=Object::new();text_id.insert("id","native_history_checkpoint:first");assert!(canonical_observations(vec![text_id]).is_err());
        assert_eq!(canonical_observations(vec![]).unwrap(),(vec![],vec![]));
    }
    #[test]
    fn observation_window_retains_existing_row_and_byte_bounds(){
        let rows=(0..=crate::loader::NATIVE_WINDOW_ROWS).map(|index|observed(&index.to_string(),"body")).collect();assert!(canonical_observations(rows).is_err());
        let body="x".repeat(lctx_model::domain::resources::TRANSFER_BYTES/2);
        assert!(canonical_observations(vec![observed("first",&body),observed("second",&body)]).is_err());
        // The existing window permits one large row within the separate max-row bound.
        assert!(canonical_observations(vec![observed("single",&body)]).is_ok());
    }
    #[test]
    fn bounded_history_cursor_roundtrips_and_resets_at_pass_boundary(){
        let mut state=State::default();state.after=Some(RecordId::new("native_history_checkpoint","consumed"));assert!(state.validate().is_ok());
        let restored:State=serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();assert_eq!(restored,state);
        state.next_pass(Pass::Verify);assert!(state.after.is_none());assert!(state.validate().is_ok());
        surrealdb_syn::parse(&page_query("UPDATE $progress SET state='qualification-rollback-sentinel' RETURN NONE; THROW 'deliberate upgrade qualification rollback';")).unwrap();
    }
    #[test]
    fn foreign_or_terminal_history_cursor_refuses(){
        let mut state=State::default();state.after=Some(RecordId::new("native_effect","foreign"));assert!(state.validate().is_err());
        state.after=Some(RecordId::new("native_history_checkpoint","cursor"));state.pass=Pass::Ready;assert!(state.validate().is_err());
    }
}
