//! Source4 → format5 is an additive optional receipt declaration, not a replay of v3.
//! Frozen format4 installers own the separate original migration and its retained evidence.
use super::*;
use authorization::{hash_field,int_field};
use crate::upgrade::{progress::{Pass,State},UpgradeExecutionContract};

const PHASES:&[(&str,&str)]=&[("effect","native_effect"),("cleanup","native_cleanup"),("retirement_item","native_retirement_item"),("retirement","native_retirement"),("pin","native_pin"),("backup","native_backup_hold"),("attempt","native_attempt"),("checkpoint","native_history_checkpoint")];
fn absent(row:&Object,field:&str)->bool{row.get(field).is_none_or(|value|*value==Value::None)}
/// Validate only the actual migration consumer's source state. No historical page outcome
/// can be reconstructed from cumulative counters; an absent optional receipt stays absent.
fn classify_checkpoint(row:&Object,generation:ContentHash)->Result<(),ModelError>{
    let Some(Value::RecordId(id))=row.get("id")else{return Err(ModelError::Schema("history migration checkpoint identity"));};
    if id.table.as_str()!="native_history_checkpoint" || hash_field(row,"generation")?!=generation{return Err(ModelError::Conflict("history migration checkpoint scope"));}
    let era=int_field(row,"era")?;let horizon=int_field(row,"closed_through")?;
    if era<=0 || horizon<0 || horizon>=era || int_field(row,"epoch")?<=0 || int_field(row,"revision")?<0 || int_field(row,"removed_effects")?<0 || int_field(row,"removed_records")?<int_field(row,"removed_effects")?{return Err(ModelError::Conflict("history migration original issuance/counters"));}
    if !matches!(row.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"open"|"done")){return Err(ModelError::Schema("history migration checkpoint state"));}
    let Some(Value::String(phase))=row.get("phase")else{return Err(ModelError::Schema("history migration checkpoint phase"));};
    let (_,table)=PHASES.iter().find(|(name,_)|*name==phase).ok_or(ModelError::Schema("history migration checkpoint phase"))?;
    if !absent(row,"after") && !matches!(row.get("after"),Some(Value::RecordId(after)) if after.table.as_str()==*table){return Err(ModelError::Conflict("history migration original phase/cursor"));}
    if !absent(row,"last_page"){return Err(ModelError::Conflict("history migration cannot invent prior page outcome"));}
    Ok(())
}
pub(crate) struct Page {pub next:State,pub body:String,pub bindings:Variables,pub observations:Vec<Object>}
pub(crate) async fn prepare_page(client:&Surreal<Client>,contract:&UpgradeExecutionContract,state:&State)->Result<Page,ModelError>{
    if !matches!(state.pass,Pass::Preflight|Pass::Verify){return Err(ModelError::Conflict("history migration not a read-validation pass"));}
    let after=state.after.clone().map(Value::RecordId).unwrap_or(Value::None);
    let mut response=client.query(format!("SELECT * FROM {} LIMIT {}",keyset_source("native_history_checkpoint",&after)?,crate::loader::NATIVE_WINDOW_ROWS)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
    let rows=crate::loader::NativeWindows::new(rows).next().transpose()?.unwrap_or_default();
    let rows=rows.into_iter().map(|row|match row{Value::Object(row)=>Ok(row),_=>Err(ModelError::Schema("history migration checkpoint row"))}).collect::<Result<Vec<_>,_>>()?;
    let mut next=state.clone();let mut body=String::new();
    for row in &rows{classify_checkpoint(row,contract.generation)?;}
    if let Some(row)=rows.last(){let Some(Value::RecordId(id))=row.get("id")else{unreachable!()};next.after=Some(id.clone());}
    else{match state.pass{Pass::Preflight=>next.next_pass(Pass::Declarations),Pass::Verify=>{next.next_pass(Pass::Ready);body.push_str("UPDATE $journal SET phase='verified' RETURN NONE;");},_=>unreachable!()}}
    Ok(Page{next,body,bindings:Variables::new(),observations:rows})
}
#[cfg(test)]
mod tests{
    use super::*;
    fn checkpoint()->Object{
        let mut row=Object::new();row.insert("id",RecordId::new("native_history_checkpoint","history"));row.insert("generation",ContentHash::of(b"generation").hex());row.insert("era",3i64);row.insert("epoch",9i64);row.insert("closed_through",2i64);row.insert("revision",7i64);row.insert("removed_effects",4i64);row.insert("removed_records",6i64);row.insert("phase","retirement");row.insert("state","open");row.insert("after",RecordId::new("native_retirement","consumed"));row
    }
    #[test]
    fn optional_receipt_absence_preserves_existing_issuance_horizon_and_cursor(){
        let mut row=checkpoint();let original=row.clone();assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_ok());assert_eq!(row,original);
        row.insert("last_page",Value::None);assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_ok());
        row.insert("last_page",Object::new());assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_err());
    }
    #[test]
    fn malformed_source4_counter_horizon_and_cursor_are_not_repaired(){
        let mut row=checkpoint();row.insert("removed_records",1i64);assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_err());
        row=checkpoint();row.insert("closed_through",3i64);assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_err());
        row=checkpoint();row.insert("after",RecordId::new("native_pin","foreign"));assert!(classify_checkpoint(&row,ContentHash::of(b"generation")).is_err());
        row=checkpoint();assert!(classify_checkpoint(&row,ContentHash::of(b"foreign")).is_err());
    }
}
