//! Explicit, resumable migration under the host's drained, closed maintenance capability.
//! Ordinary runtime attachment never installs or consumes this progress overlay.
use crate::{AuthenticationScope, Loader, RuntimeConfig, reader};
use lctx_model::domain::{ContentHash, ModelError};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, Value, Variables}};
mod contract;
pub use contract::{UpgradeExecutionContract, overlay_schema_identity, transition_protocol_identity, preflight_contract_identity, verifier_identity};
pub(crate) mod progress;
pub use progress::UpgradeAdvance;
use progress::{Pass, Progress};
pub(crate) fn native(value:impl surrealdb::types::SurrealValue)->Value{value.into_value()}

pub const SOURCE_SCHEMA: &str = "0870158e2d5568650845dd61f61884384c44315071f0b0dc402d02994a4798da";
pub const SOURCE_VERSION:u32=4;
pub const TRANSITION_KIND:&str="history_page_receipts_v5";
const JOURNAL_SCHEMA: &str = "DEFINE TABLE native_upgrade SCHEMAFULL; DEFINE FIELD source ON native_upgrade TYPE string; DEFINE FIELD target ON native_upgrade TYPE string; DEFINE FIELD generation ON native_upgrade TYPE string; DEFINE FIELD phase ON native_upgrade TYPE string;";

fn check_source(row: &Object, config: &RuntimeConfig, expected: ContentHash) -> Result<(), ModelError> {
    if expected.hex() != SOURCE_SCHEMA || row.get("generation") != Some(&Value::String(config.service_generation.hex())) || row.get("admission_open") != Some(&Value::Bool(false)) || row.get("schema") != Some(&Value::String(expected.hex())) || row.get("schema_version") != Some(&native(i64::from(SOURCE_VERSION))) {
        return Err(ModelError::Conflict("exact source4 native upgrade/generation/closed admission"));
    }
    source_marker_fields(row)
}
fn source_marker_fields(row:&Object)->Result<(),ModelError>{
    let integer=|name:&str|match row.get(name){Some(Value::Number(surrealdb::types::Number::Int(value)))=>Ok(*value),_=>Err(ModelError::Schema("source4 installation integer"))};
    let era=integer("era")?;let closed=integer("closed_through")?;let revision=integer("control_revision")?;
    if era<=0 || closed<0 || closed>=era || revision<0 || revision==i64::MAX{return Err(ModelError::Conflict("source4 installation era/watermarks"));}
    for field in ["admission_revision","backup_revision"]{
        if row.get(field).is_some_and(|value|*value!=Value::None && !matches!(value,Value::Number(surrealdb::types::Number::Int(value)) if *value>=0)){return Err(ModelError::Schema("source4 installation optional watermark"));}
    }
    if row.get("history_inventory").is_some_and(|value|*value!=Value::None && !matches!(value,Value::String(value) if !value.is_empty())){return Err(ModelError::Schema("source4 installation history consumer inventory"));}
    Ok(())
}
async fn marker(client: &Surreal<Client>) -> Result<Object, ModelError> {
    let mut response=client.query("SELECT * FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else { return Err(ModelError::Conflict("native upgrade installation marker")); };
    Ok(row.clone())
}
/// The host maintenance owner supplies borrower-drain proof; native state alone cannot grant it.
pub async fn maintenance_client(config: &RuntimeConfig) -> Result<std::sync::Arc<Surreal<Client>>, ModelError> {
    if config.authentication != AuthenticationScope::Root { return Err(ModelError::Conflict("native lifecycle maintenance requires installer authority")); }
    let client=crate::compiler::check_installation(config).await?;
    let checked=async {
        if marker(&client).await?.get("admission_open")!=Some(&Value::Bool(false)) { return Err(ModelError::Conflict("native lifecycle maintenance requires closed admission")); }
        crate::control::check_pending_effects(&client).await
    }.await;
    if let Err(error)=checked{
        let mut completion=lctx_model::domain::completion::Completion::default();
        completion.step("failed maintenance session invalidation",client.invalidate().await.map_err(ModelError::codec));
        return lctx_model::domain::completion::complete(Err(error),completion);
    }
    Ok(client)
}
async fn check_journal(client:&Surreal<Client>,contract:&UpgradeExecutionContract,published:bool)->Result<(),ModelError>{
    let mut response=client.query("SELECT * FROM $journal").bind(progress::base_bindings(contract)?).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let [journal]=rows.as_slice() else{return Err(ModelError::Conflict("native upgrade journal missing"));};
    if journal.get("source")!=Some(&native(contract.source.hex())) || journal.get("target")!=Some(&native(contract.target.hex())) || journal.get("generation")!=Some(&native(contract.generation.hex())) || !matches!(journal.get("phase"),Some(Value::String(phase)) if if published{phase=="published"}else{matches!(phase.as_str(),"intent"|"declarations"|"verified")}) {return Err(ModelError::Conflict("native upgrade journal identity/phase"));}
    Ok(())
}
/// Run or advance an explicitly identified execution. `Some(0)` establishes only identity;
/// `Some(n)` stops after at most n durable checkpoints and before publication if ready.
/// The host must independently finish executable definitions and its durable scope receipt.
pub async fn upgrade_with_contract(config:&RuntimeConfig,contract:&UpgradeExecutionContract,native_definitions:&str,stop_after_pages:Option<u64>)->Result<UpgradeAdvance,ModelError>{
    contract.validate(config)?;
    if config.authentication!=AuthenticationScope::Root{return Err(ModelError::Conflict("native upgrade requires installer authority"));}
    let client=reader::connect(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str()).await?;
    let result=upgrade_in(&client,config,contract,native_definitions,stop_after_pages).await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("native upgrade session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion)
}
async fn upgrade_in(client:&std::sync::Arc<Surreal<Client>>,config:&RuntimeConfig,contract:&UpgradeExecutionContract,native_definitions:&str,stop_after_pages:Option<u64>)->Result<UpgradeAdvance,ModelError>{
    let installed=marker(client).await?;
    let loader=Loader::new(client.clone());
    let mut catalog=loader.installation_catalog().await?;
    let published=installed.get("schema_version")==Some(&native(crate::control::SCHEMA_VERSION)) && installed.get("schema")==Some(&native(contract.target.hex()));
    if published {
        if installed.get("generation")!=Some(&native(contract.generation.hex())) || installed.get("admission_open")!=Some(&Value::Bool(false)){return Err(ModelError::Conflict("native upgrade published installation identity"));}
        check_journal(&client,contract,true).await?;
        crate::control::check_installation(&client,config.service_generation).await?;
        // Every format5 publication is owned by this protocol. Historical format4 scopes
        // are handled by their frozen installer; current publication cannot invent progress.
        let has_overlay=catalog.has_table("native_upgrade_progress_v1");
        if has_overlay{catalog.check(contract::OVERLAY_SCHEMA)?;}
        if !has_overlay{return Err(ModelError::Conflict("native format5 publication progress overlay missing"));}
        let revision=match progress::read(&client,contract).await?{Some(p) if p.state.pass==Pass::Sealed=>p.revision as u64,_=>return Err(ModelError::Conflict("native format5 publication sealed progress missing"))};
        return Ok(UpgradeAdvance{revision,phase:"published".into(),sealed:true,published:true});
    }
    check_source(&installed,config,contract.source)?;
    crate::control::check_pending_effects(client).await?;
    catalog.apply(JOURNAL_SCHEMA,"native upgrade original operation").await?;
    catalog.apply(contract::OVERLAY_SCHEMA,"native upgrade progress overlay").await?;
    let bindings=progress::base_bindings(contract)?;
    client.query("BEGIN; LET $installed=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installed.generation!=$generation OR $installed.schema!=$source OR $installed.schema_version!=$source_version OR $installed.admission_open { THROW 'native upgrade operation authority'; }; LET $old=SELECT * FROM ONLY $journal FOR UPDATE; IF $old=NONE { CREATE $journal SET source=$source,target=$target,generation=$generation,phase='intent' RETURN NONE; } ELSE { IF $old.source!=$source OR $old.target!=$target OR $old.generation!=$generation { THROW 'native upgrade operation collision'; }; }; COMMIT;")
        .bind(bindings).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    check_journal(&client,contract,false).await?;
    let mut current=progress::acquire(&client,contract).await?;
    let mut committed=0u64;
    loop {
        if stop_after_pages.is_some_and(|limit|committed>=limit){return Ok(current.result());}
        match current.state.pass {
            Pass::Declarations=>{
                catalog.apply(crate::control::schema(),"native control format upgrade").await?;
                catalog.install(native_definitions).await?;
                catalog.apply(&crate::compiler::compiler_schema(),"compiler format upgrade").await?;
                catalog.apply(crate::compiler::VIEW_SCHEMA,"completed view format upgrade").await?;
                let mut next=current.state.clone();next.next_pass(Pass::Verify);
                current=progress::commit(&client,&current,next,"UPDATE $journal SET phase='declarations' RETURN NONE;",Variables::new(),vec![]).await?;
            }
            Pass::Ready=>return if stop_after_pages.is_some(){Ok(current.result())}else{publish(&client,current).await},
            Pass::Sealed=>return Err(ModelError::Conflict("native upgrade sealed progress without publication")),
            _=>{
                let page=crate::control::migration::prepare_page(&client,contract,&current.state).await?;
                current=progress::commit(&client,&current,page.next,&page.body,page.bindings,page.observations).await?;
            }
        }
        committed+=1;
        tracing::info!(migration=%contract.migration.hex(),execution=%contract.execution.hex(),revision=current.revision,pass=?current.state.pass,"native upgrade checkpoint committed");
    }
}
async fn publish(client:&Surreal<Client>,current:Progress)->Result<UpgradeAdvance,ModelError>{
    let contract=&current.contract;let mut bindings=progress::base_bindings(contract)?;
    let mut next=current.state.clone();next.next_pass(Pass::Sealed);
    bindings.insert("revision",current.revision);bindings.insert("expected_state",serde_json::to_string(&current.state).map_err(ModelError::codec)?);bindings.insert("next",serde_json::to_string(&next).map_err(ModelError::codec)?);bindings.insert("version",crate::control::SCHEMA_VERSION);
    let result=client.query("BEGIN; LET $installed=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installed.generation!=$generation OR $installed.schema!=$source OR $installed.schema_version!=$source_version OR $installed.admission_open { THROW 'native upgrade publication authority'; }; LET $old=SELECT * FROM ONLY $progress FOR UPDATE; IF $old.contract!=$contract OR $old.execution!=$execution OR $old.revision!=$revision OR $old.state!=$expected_state OR $old.sealed { THROW 'native upgrade publication progress'; }; LET $original=SELECT * FROM ONLY $journal FOR UPDATE; IF $original.source!=$source OR $original.target!=$target OR $original.generation!=$generation OR $original.phase!='verified' { THROW 'native upgrade publication original journal'; }; UPDATE native_installation:current SET schema=$target,schema_version=$version,control_revision=control_revision+1 RETURN NONE; UPDATE $journal SET phase='published' RETURN NONE; UPDATE $progress SET state=$next,revision+=1,sealed=true RETURN NONE; COMMIT;").bind(bindings).await.map_err(crate::loader::write_failure).and_then(crate::control::checked_transaction).map(|_|());
    if result.is_ok(){return Ok(Progress{revision:current.revision+1,state:next,contract:contract.clone()}.result());}
    let (actual,result)=progress::read_after(client,contract,result).await?;
    let actual=actual.ok_or(ModelError::Conflict("native upgrade publication progress absent"))?;
    if actual.revision==current.revision+1 && actual.state==next && actual.contract==*contract {
        check_journal(client,contract,true).await?;
        crate::control::check_installation(client,contract.generation).await?;
        return Ok(actual.result());
    }
    result?;Err(ModelError::Conflict("native upgrade publication acknowledgement"))
}

/// Maintenance-only production-executor controls. These intentionally exercise only the
/// progress slot; they do not claim network failure or qualification of every data transform.
#[derive(Debug,Clone,serde::Serialize)]
pub struct UpgradeQualification {pub rollback:bool,pub acknowledgement_reconciliation:bool,pub stale_revision_refusal:bool,pub revision:u64}
pub async fn qualify_upgrade_pages(config:&RuntimeConfig,contract:&UpgradeExecutionContract)->Result<UpgradeQualification,ModelError>{
    if config.database.as_str()!="validation" || config.authentication!=AuthenticationScope::Root{return Err(ModelError::Conflict("upgrade qualification requires root validation scope"));}
    let initial=upgrade_with_contract(config,contract,"",Some(0)).await?;
    if initial.published{return Err(ModelError::Conflict("upgrade qualification requires unpublished scope"));}
    let client=reader::connect(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str()).await?;
    let result=qualify_pages_in(&client,contract).await;
    let mut completion=lctx_model::domain::completion::Completion::default();
    completion.step("native upgrade qualification session invalidation",client.invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result,completion)
}
async fn qualify_pages_in(client:&Surreal<Client>,contract:&UpgradeExecutionContract)->Result<UpgradeQualification,ModelError>{
    let current=progress::read(client,contract).await?.ok_or(ModelError::Conflict("upgrade qualification progress missing"))?;
    if current.contract!=*contract{return Err(ModelError::Conflict("upgrade qualification active execution"));}
    let installation=marker(client).await?;
    let observed=vec![installation.clone()];
    // Exercise nonempty correspondence inside the production transaction, including
    // refusal before its body. Neither negative control mutates source/runtime rows.
    let mut changed=installation.clone();changed.insert("qualification_changed_body",true);
    let mut missing=installation;missing.insert("id",surrealdb::types::RecordId::new("native_installation","qualification_unavailable"));
    for observation in [changed,missing]{
        let refused=progress::commit(client,&current,current.state.clone(),"UPDATE $progress SET state='qualification-correspondence-sentinel' RETURN NONE;",Variables::new(),vec![observation]).await;
        match refused{Err(ref error) if error.to_string().contains("native upgrade page inputs changed")=>{},Err(error)=>return Err(error),Ok(_)=>return Err(ModelError::Conflict("upgrade qualification changed/missing observation accepted"))}
        let actual=progress::read(client,contract).await?.ok_or(ModelError::Conflict("upgrade qualification correspondence lost progress"))?;
        if actual.revision!=current.revision || actual.state!=current.state || actual.contract!=current.contract{return Err(ModelError::Conflict("upgrade qualification correspondence changed state"));}
    }
    let rolled_back=progress::commit(&client,&current,current.state.clone(),"UPDATE $progress SET state='qualification-rollback-sentinel' RETURN NONE; THROW 'deliberate upgrade qualification rollback';",Variables::new(),observed.clone()).await;
    match rolled_back {Err(ref error) if error.to_string().contains("deliberate upgrade qualification rollback")=>{},Err(error)=>return Err(error),Ok(_)=>return Err(ModelError::Conflict("upgrade qualification rollback unexpectedly committed"))}
    let actual=progress::read(&client,contract).await?.ok_or(ModelError::Conflict("upgrade qualification rollback lost progress"))?;
    if actual.revision!=current.revision || actual.state!=current.state || actual.contract!=current.contract{return Err(ModelError::Conflict("upgrade qualification rollback changed state"));}
    let committed=progress::commit_with_ack(&client,&current,current.state.clone(),"",Variables::new(),observed.clone(),true).await?;
    if committed.revision!=current.revision+1 || committed.state!=current.state{return Err(ModelError::Conflict("upgrade qualification discarded acknowledgement"));}
    let replay=progress::commit(&client,&current,current.state.clone(),"",Variables::new(),observed.clone()).await?;
    if replay.revision!=committed.revision{return Err(ModelError::Conflict("upgrade qualification stale replay advanced"));}
    let mut wrong=current.state.clone();wrong.next_pass(if current.state.pass==Pass::Ready{Pass::Preflight}else{Pass::Ready});
    match progress::commit(&client,&current,wrong,"",Variables::new(),observed).await {Err(ref error) if error.to_string().contains("native upgrade page revision")=>{},Err(error)=>return Err(error),Ok(_)=>return Err(ModelError::Conflict("upgrade qualification stale revision accepted"))}
    let actual=progress::read(&client,contract).await?.ok_or(ModelError::Conflict("upgrade qualification final progress missing"))?;
    if actual.revision!=committed.revision || actual.state!=committed.state || actual.contract!=committed.contract{return Err(ModelError::Conflict("upgrade qualification stale revision changed state"));}
    Ok(UpgradeQualification{rollback:true,acknowledgement_reconciliation:true,stale_revision_refusal:true,revision:actual.revision as u64})
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn source4_era_watermarks_and_inventory_are_validated_without_rewriting(){
        let mut row=Object::new();row.insert("era",8i64);row.insert("closed_through",7i64);row.insert("control_revision",42i64);row.insert("admission_revision",100i64);row.insert("backup_revision",6i64);row.insert("history_inventory",ContentHash::of(b"qualified-consumers").hex());
        let original=row.clone();assert!(source_marker_fields(&row).is_ok());assert_eq!(row,original);
        row.insert("closed_through",8i64);assert!(source_marker_fields(&row).is_err());row.insert("closed_through",7i64);row.insert("control_revision",Value::None);assert!(source_marker_fields(&row).is_err());
    }
}
