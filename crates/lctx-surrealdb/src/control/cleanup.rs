//! Durable exact terminal cleanup, with a distinct current-era maintenance successor.
use super::*;

pub(super) struct CleanupHolds { identity: ContentHash }
impl CleanupHolds {
    pub(super) async fn begin(client:&Surreal<Client>,authorization:TerminalAttempt,owners:Vec<RecordId>,unadmitted:bool)->Result<Self,ModelError>{
        let mut response=client.query("SELECT generation,era FROM $attempt").bind(("attempt",RecordId::new("native_attempt",authorization.identity.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [origin]=rows.as_slice() else{return Err(ModelError::Conflict("native cleanup original attempt missing"));};
        let issuance=IssuanceEra{generation:authorization::hash_field(origin,"generation")?,era:authorization::int_field(origin,"era")?};
        let identity=fresh_identity("cleanup-obligation")?;
        let mut bindings=Variables::new();
        bindings.insert("cleanup",RecordId::new("native_cleanup",identity.hex()));
        bindings.insert("owners",owners);bindings.insert("unadmitted",unadmitted);
        effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"IF $unadmitted AND $__owner.admitted { THROW 'native cleanup admitted branch'; }; CREATE $cleanup SET attempt=$__attempt,generation=$__generation,era=$__era,epoch=$__epoch,owners=$owners,unadmitted=$unadmitted,state='open' RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("cleanup obligation",identity,issuance,error))?;
        Ok(Self{identity})
    }
    pub(super) async fn step(&self,client:&Surreal<Client>)->Result<bool,ModelError>{
        let cleanup=RecordId::new("native_cleanup",self.identity.hex());
        let mut pending=client.query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND owner=$cleanup LIMIT 1").bind(("cleanup",cleanup.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let pending:Vec<RecordId>=pending.take(0).map_err(ModelError::codec)?;
        if !pending.is_empty(){return Err(ModelError::Conflict("native cleanup previous page unresolved"));}
        let mut response=client.query("SELECT * FROM $cleanup").bind(("cleanup",cleanup.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native cleanup obligation missing"));};
        if row.get("state")==Some(&Value::String("done".into())) {return Ok(true);}
        let Some(Value::Array(owners))=row.get("owners") else{return Err(ModelError::Schema("native cleanup exact owners"));};
        let owner=owners.first().cloned().unwrap_or(Value::None);
        let mut bindings=Variables::new();bindings.insert("cleanup",cleanup.clone());bindings.insert("owner",owner);
        effect_for_owner(client,EffectOwner::Cleanup(cleanup),"LET $origin_id=$__owner.attempt; LET $origin=SELECT * FROM ONLY $origin_id FOR UPDATE; IF $origin=NONE OR $origin.generation!=$__owner.generation OR $origin.epoch<=0 OR $origin.epoch!=$__owner.epoch OR $origin.state NOT IN ['closing','closed','abandoned','frozen','maintenance_fenced'] OR ($__owner.unadmitted AND $origin.admitted) { THROW 'native cleanup origin changed'; }; IF array::len($__owner.owners)=0 { UPDATE $cleanup SET state='done' RETURN NONE; } ELSE { IF $__owner.owners[0]!=$owner { THROW 'native cleanup owner raced'; }; LET $holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner LIMIT 128; DELETE $holds RETURN NONE; IF array::len($holds)=0 { IF $__owner.unadmitted AND record::table($owner)='native_product' { LET $product=SELECT * FROM ONLY $owner FOR UPDATE; IF $product!=NONE { LET $contribution_id=$product.contribution; LET $contribution=SELECT * FROM ONLY $contribution_id FOR UPDATE; IF $contribution.attempt!=$__owner.attempt { THROW 'native cleanup product origin changed'; }; DELETE $owner RETURN NONE; }; }; UPDATE $cleanup SET owners=array::slice(owners,1) RETURN NONE; }; };",bindings).await?;
        let mut response=client.query("SELECT VALUE state FROM $cleanup").bind(("cleanup",RecordId::new("native_cleanup",self.identity.hex()))).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let states:Vec<String>=response.take(0).map_err(ModelError::codec)?;
        Ok(states==["done"])
    }
}
pub async fn resume_cleanup(client:&Surreal<Client>,identity:ContentHash)->Result<(),ModelError>{
    let cleanup=CleanupHolds{identity};while !cleanup.step(client).await?{}Ok(())
}
/// This is a new operation over only the predecessor's persisted remaining owners.
pub async fn recover_cleanup(client:&Surreal<Client>,identity:ContentHash)->Result<ContentHash,ModelError>{
    let issuance=IssuanceEra::capture(client).await?;
    let old=RecordId::new("native_cleanup",identity.hex());
    let mut response=client.query("SELECT successor FROM $old").bind(("old",old.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    if let Some(Value::RecordId(successor))=rows.first().and_then(|row|row.get("successor")) {
        let mut response=client.query("SELECT era,generation FROM $successor").bind(("successor",successor.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("cleanup successor missing"));};
        if authorization::int_field(row,"era")?!=issuance.era || authorization::hash_field(row,"generation")?!=issuance.generation {return Err(ModelError::Conflict("recover named cleanup successor first"));}
        let surrealdb::types::RecordIdKey::String(key)=&successor.key else{return Err(ModelError::Schema("cleanup successor identity"));};
        return Ok(ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("cleanup successor width"))?));
    }
    let successor=fresh_identity("cleanup-maintenance-successor")?;
    let next=RecordId::new("native_cleanup",successor.hex());
    let mut bindings=Variables::new();bindings.insert("old",old);bindings.insert("next",next);
    bindings.insert("current_era",issuance.era);
    effect(client,None,"IF $__era!=$current_era OR $__installation.admission_open { THROW 'cleanup recovery requires original current-era drained maintenance'; }; LET $before=SELECT * FROM ONLY $old FOR UPDATE; IF $before=NONE OR $before.state!='open' OR $before.generation!=$__generation OR $before.epoch<=0 OR $before.successor!=NONE OR $before.era>($__installation.closed_through ?? 0) { THROW 'cleanup predecessor not fenced'; }; IF array::len(SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false AND id!=$__effect LIMIT 1)>0 { THROW 'cleanup recovery has unresolved effects'; }; CREATE $next SET attempt=$before.attempt,generation=$__generation,era=$current_era,epoch=$before.epoch,owners=$before.owners,unadmitted=$before.unadmitted,state='open' RETURN NONE; UPDATE $old SET successor=$next,state='superseded' RETURN NONE",bindings).await?;
    Ok(successor)
}
