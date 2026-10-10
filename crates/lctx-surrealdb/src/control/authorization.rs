//! Issuance is fenced independently of intent arrival. A retry carries this exact request.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuanceEra {
    pub generation: ContentHash,
    pub era: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRequest {
    pub issuance: IssuanceEra,
    pub operation: ContentHash,
    pub request: ContentHash,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectDisposition { Committed, FencedUncommitted, Pending, CompactedTerminal }
#[derive(Debug)]
pub struct NativeRequestError {
    pub request: NativeRequest,
    pub source: ModelError,
}
#[derive(Debug)]
pub struct NativeAttemptError { pub attempt:ContentHash,pub issuance:IssuanceEra,pub source:ModelError }
impl std::fmt::Display for NativeAttemptError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {write!(f,"native attempt {} era {} generation {}: {}",self.attempt.hex(),self.issuance.era,self.issuance.generation.hex(),self.source)}
}
impl std::error::Error for NativeAttemptError {fn source(&self)->Option<&(dyn std::error::Error+'static)>{Some(&self.source)}}
#[derive(Debug)]
pub struct NativeLifecycleError {pub kind:&'static str,pub identity:ContentHash,pub issuance:IssuanceEra,pub source:ModelError}
impl std::fmt::Display for NativeLifecycleError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {write!(f,"native {} {} era {} generation {}: {}",self.kind,self.identity.hex(),self.issuance.era,self.issuance.generation.hex(),self.source)}
}
impl std::error::Error for NativeLifecycleError {fn source(&self)->Option<&(dyn std::error::Error+'static)>{Some(&self.source)}}
pub(super) fn lifecycle_error(kind:&'static str,identity:ContentHash,issuance:IssuanceEra,source:ModelError)->ModelError {ModelError::Cause(Box::new(NativeLifecycleError{kind,identity,issuance,source}))}
impl std::fmt::Display for NativeRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native request {} era {} generation {} digest {}: {}", self.request.operation.hex(),
            self.request.issuance.era, self.request.issuance.generation.hex(), self.request.request.hex(), self.source)
    }
}
impl std::error::Error for NativeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(&self.source) }
}
impl NativeRequest {
    pub async fn issue(client: &Surreal<Client>, request: ContentHash) -> Result<Self, ModelError> {
        let issuance = IssuanceEra::capture(client).await?;
        // Never allocate the operation before capturing the authorization era.
        Ok(Self { issuance, operation: fresh_identity("effect")?, request })
    }
    pub(super) fn bind(&self, bindings: &mut Variables) {
        bindings.insert("__effect", RecordId::new("native_effect", self.operation.hex()));
        bindings.insert("__request", self.request.hex());
        bindings.insert("__generation", self.issuance.generation.hex());
        bindings.insert("__era", self.issuance.era);
    }
    pub(super) fn error(self, source: ModelError) -> ModelError {
        ModelError::Cause(Box::new(NativeRequestError { request: self, source }))
    }
}
impl IssuanceEra {
    pub async fn capture(client: &Surreal<Client>) -> Result<Self, ModelError> {
        let mut response = client.query("SELECT generation,era,closed_through FROM native_installation:current")
            .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else { return Err(ModelError::Conflict("native issuance installation")); };
        let generation = hash_field(row, "generation")?;
        let era = int_field(row, "era")?;
        if era <= int_field(row, "closed_through")? || era <= 0 {
            return Err(ModelError::Conflict("native issuance era closed"));
        }
        Ok(Self { generation, era })
    }
}
pub(super) fn int_field(row: &Object, field: &str) -> Result<i64, ModelError> {
    match row.get(field) {
        Some(Value::Number(surrealdb::types::Number::Int(value))) => Ok(*value),
        _ => Err(ModelError::Schema("native control integer field")),
    }
}
pub(super) fn hash_field(row: &Object, field: &str) -> Result<ContentHash, ModelError> {
    match row.get(field) {
        Some(Value::String(value)) => Ok(ContentHash(hex::decode(value).map_err(ModelError::codec)?
            .try_into().map_err(|_| ModelError::Schema("native control hash width"))?)),
        _ => Err(ModelError::Schema("native control hash field")),
    }
}
// Every mutation conflicts with era closure, even if its payload writes disjoint records.
pub(super) const ERA_GUARD: &str = "LET $__installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $__installation=NONE OR $__installation.generation!=$__generation OR $__installation.era!=$__era OR $__era<=($__installation.closed_through ?? 0) { THROW 'native issuance era fenced'; }; UPDATE native_installation:current SET control_revision=(control_revision ?? 0)+1 RETURN NONE;";

pub async fn issue_attempt(client: &Surreal<Client>, generation: ContentHash) -> Result<ContentHash, ModelError> {
    let era = IssuanceEra::capture(client).await?;
    if era.generation != generation { return Err(ModelError::Conflict("native attempt generation")); }
    let attempt = fresh_identity("compiler-attempt")?;
    begin_attempt_in(client, attempt, era).await?;
    Ok(attempt)
}
pub async fn begin_attempt_in(client: &Surreal<Client>, attempt: ContentHash, era: IssuanceEra) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert("__generation", era.generation.hex()); bindings.insert("__era", era.era);
    let query = format!("BEGIN; {ERA_GUARD} LET $existing=SELECT * FROM ONLY $attempt FOR UPDATE; IF $existing!=NONE {{ IF $existing.generation!=$__generation OR $existing.era!=$__era OR $existing.epoch<=0 {{ THROW 'native attempt identity collision'; }}; }} ELSE {{ IF !$__installation.admission_open {{ THROW 'native installation admission closed'; }}; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $attempt SET generation=$__generation,era=$__era,state='open',revision=0,admitted=false,epoch=($__installation.admission_revision ?? 0)+1 RETURN NONE; }}; COMMIT;");
    super::run_transaction(client, &query, bindings).await.map_err(|source|ModelError::Cause(Box::new(NativeAttemptError{attempt,issuance:era,source})))
}

#[derive(Clone, Copy)]
pub(super) struct TerminalAttempt { pub identity: ContentHash, pub epoch: i64 }
pub(super) enum EffectOwner {
    Installation,
    OpenAttempt(ContentHash),
    FrozenAttempt(ContentHash),
    Pin(RecordId),
    TerminalAttempt(TerminalAttempt),
    Retirement(RecordId),
    RetirementSetup(RecordId),
    Cleanup(RecordId),
}
impl EffectOwner {
    fn record(&self) -> Option<RecordId> {
        match self {
            Self::OpenAttempt(id) | Self::FrozenAttempt(id) => Some(RecordId::new("native_attempt", id.hex())),
            Self::TerminalAttempt(owner) => Some(RecordId::new("native_attempt", owner.identity.hex())),
            Self::Pin(id) | Self::Retirement(id) | Self::RetirementSetup(id) | Self::Cleanup(id) => Some(id.clone()),
            Self::Installation => None,
        }
    }
    fn kind(&self) -> &'static str {
        match self { Self::Installation=>"installation", Self::OpenAttempt(_)=>"attempt",
            Self::FrozenAttempt(_)=>"frozen_attempt", Self::Pin(_)=>"pin", Self::TerminalAttempt(_)=>"terminal_attempt",
            Self::Retirement(_)=>"retirement", Self::RetirementSetup(_)=>"retirement_setup", Self::Cleanup(_)=>"cleanup" }
    }
    fn predicate(&self) -> &'static str {
        match self { Self::OpenAttempt(_)=>"$__owner.state='open'", Self::FrozenAttempt(_)=>"$__owner.state='frozen'",
            Self::Pin(_)=>"!$__owner.released", Self::TerminalAttempt(_)=>"$__owner.state IN ['closing','closed','abandoned','frozen','maintenance_fenced'] AND $__owner.epoch=$__expected_epoch",
            Self::Retirement(_)|Self::Cleanup(_)=>"$__owner.state='open' AND $__owner.successor=NONE",
            Self::RetirementSetup(_)=>"$__owner.state IN ['registering','recovering','open'] AND $__owner.successor=NONE", Self::Installation=>"true" }
    }
}
pub async fn effect(client: &Surreal<Client>, attempt: Option<ContentHash>, sql: &str, bindings: Variables) -> Result<(), ModelError> {
    effect_for_owner(client, attempt.map(EffectOwner::OpenAttempt).unwrap_or(EffectOwner::Installation), sql, bindings).await
}
pub async fn effect_frozen_attempt(client: &Surreal<Client>, attempt: ContentHash, sql: &str, bindings: Variables) -> Result<(), ModelError> {
    effect_for_owner(client, EffectOwner::FrozenAttempt(attempt), sql, bindings).await
}
pub(super) async fn effect_for_owner(client: &Surreal<Client>, owner: EffectOwner, sql: &str, bindings: Variables) -> Result<(), ModelError> {
    let digest = ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).map_err(ModelError::codec)?);
    let issuance=if let Some(id)=owner.record(){
        let mut response=client.query("SELECT generation,era FROM $owner").bind(("owner",id)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
        let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native effect original owner missing"));};
        IssuanceEra{generation:hash_field(row,"generation")?,era:int_field(row,"era")?}
    } else {IssuanceEra::capture(client).await?};
    let request=NativeRequest{issuance,operation:fresh_identity("effect")?,request:digest};
    execute_owner(client, request, owner, sql, bindings, false).await
}
/// Reissue only the original operation, bytes and authority. This cannot refresh a closed era.
pub async fn execute_request(client: &Surreal<Client>, request: NativeRequest, attempt: Option<ContentHash>, sql: &str, bindings: Variables) -> Result<(), ModelError> {
    execute_owner(client, request, attempt.map(EffectOwner::OpenAttempt).unwrap_or(EffectOwner::Installation), sql, bindings, true).await
}
async fn execute_owner(client: &Surreal<Client>, request: NativeRequest, owner: EffectOwner, sql: &str, mut bindings: Variables, retain: bool) -> Result<(), ModelError> {
    if request.request != ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).map_err(ModelError::codec)?) {
        return Err(request.error(ModelError::Conflict("native retry payload changed")));
    }
    request.bind(&mut bindings);
    let reference_owner=request.operation.hex();
    let reference_kind=if retain {"caller"} else {"recovery"};
    let reference=RecordId::new("native_outcome_ref",ContentHash::of(&serde_json::to_vec(&(request.operation,&reference_owner,reference_kind)).map_err(ModelError::codec)?).hex());
    bindings.insert("__outcome_ref",reference.clone());bindings.insert("__reference_owner",reference_owner);bindings.insert("__reference_kind",reference_kind);
    let owner_id = owner.record();
    let attempt = match &owner { EffectOwner::OpenAttempt(id)|EffectOwner::FrozenAttempt(id)=>Some(*id),
        EffectOwner::TerminalAttempt(owner)=>Some(owner.identity), _=>None };
    bindings.insert("__attempt", attempt.map(|id|Value::RecordId(RecordId::new("native_attempt",id.hex()))).unwrap_or(Value::None));
    bindings.insert("__owner_id", owner_id.clone().map(Value::RecordId).unwrap_or(Value::None));
    bindings.insert("__owner_kind", owner.kind());
    if let EffectOwner::TerminalAttempt(owner) = &owner { bindings.insert("__expected_epoch", owner.epoch); }
    let predicate = owner.predicate();
    let authorization = if owner_id.is_some() {
        format!("LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.generation!=$__generation OR $__owner.era!=$__era OR !({predicate}) {{ THROW 'native owner fenced'; }}; LET $__epoch=$__owner.epoch;")
    } else {
        "UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; LET $__epoch=($__installation.admission_revision ?? 0)+1;".into()
    };
    let intent = format!("BEGIN; {ERA_GUARD} LET $__prior=SELECT * FROM ONLY $__effect FOR UPDATE; IF $__prior=NONE {{ {authorization} IF $__epoch=NONE OR $__epoch<=0 {{ THROW 'native authorization epoch missing'; }}; CREATE $__effect SET attempt=$__attempt,owner=$__owner_id,owner_kind=$__owner_kind,generation=$__generation,era=$__era,request=$__request,committed=false,resolved=false,revision=0,epoch=$__epoch RETURN NONE; CREATE $__outcome_ref SET effect=$__effect,owner=$__reference_owner,kind=$__reference_kind RETURN NONE; }} ELSE {{ IF $__prior.request!=$__request OR $__prior.era!=$__era OR $__prior.generation!=$__generation OR $__prior.owner!=$__owner_id OR $__prior.owner_kind!=$__owner_kind {{ THROW 'native operation identity collision'; }}; }}; COMMIT;");
    if let Err(error) = super::run_transaction(client, &intent, bindings.clone()).await {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("native intent reconciliation", reconcile_request(client, request).await.map(|_| ()));
        return lctx_model::domain::completion::complete(Err(request.error(error)), completion);
    }
    let fence = if owner_id.is_some() {
        format!("LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.epoch!=$__epoch OR $__owner.era!=$__era OR $__owner.generation!=$__generation OR !({predicate}) {{ THROW 'native owner fenced'; }};")
    } else { String::new() };
    let query = format!("BEGIN; {ERA_GUARD} LET $__operation=SELECT * FROM ONLY $__effect FOR UPDATE; IF $__operation=NONE OR $__operation.request!=$__request OR $__operation.era!=$__era OR $__operation.generation!=$__generation {{ THROW 'native operation fenced'; }}; IF $__operation.resolved {{ IF !$__operation.committed {{ THROW 'native operation fenced uncommitted'; }}; }} ELSE {{ IF $__operation.epoch<=0 {{ THROW 'native operation epoch missing'; }}; LET $__epoch=$__operation.epoch; {fence} {sql}; UPDATE $__effect SET committed=true,resolved=true,revision+=1 RETURN NONE; }}; COMMIT;");
    let result=match super::run_transaction(client, &query, bindings).await {
        Ok(())=>Ok(()),
        Err(error)=>match reconcile_request(client,request).await {
            Ok(EffectDisposition::Committed)=>Ok(()),
            Ok(_)=>Err(request.error(error)),
            Err(reconciliation)=>{
                let mut completion=lctx_model::domain::completion::Completion::default();
                completion.step("native effect reconciliation",Err(reconciliation));
                lctx_model::domain::completion::complete(Err(request.error(error)),completion)
            }
        }
    };
    result?;
    if !retain {
        let mut release=Variables::new();release.insert("reference",reference);release.insert("generation",request.issuance.generation.hex());
        if let Err(error)=super::run_transaction(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.generation!=$generation { THROW 'native outcome release generation'; }; UPDATE native_installation:current SET control_revision=(control_revision ?? 0)+1 RETURN NONE; DELETE $reference RETURN NONE; COMMIT;",release).await {
            let mut completion=lctx_model::domain::completion::Completion::default();completion.committed("native effect committed",request.operation.hex());
            return lctx_model::domain::completion::complete(Err(request.error(error)),completion);
        }
    }
    Ok(())
}
pub async fn request_disposition(client: &Surreal<Client>, request: NativeRequest) -> Result<EffectDisposition, ModelError> {
    let mut bindings=Variables::new(); request.bind(&mut bindings);
    let mut response=client.query("SELECT * FROM $__effect; SELECT generation,closed_through FROM native_installation:current")
        .bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let installations:Vec<Object>=response.take(1).map_err(ModelError::codec)?;
    if let [row]=rows.as_slice() {
        if hash_field(row,"request")?!=request.request || int_field(row,"era")?!=request.issuance.era
            || hash_field(row,"generation")?!=request.issuance.generation { return Err(ModelError::Conflict("native receipt identity")); }
        return Ok(if row.get("committed")==Some(&Value::Bool(true)) {EffectDisposition::Committed}
            else if row.get("resolved")==Some(&Value::Bool(true)) {EffectDisposition::FencedUncommitted} else {EffectDisposition::Pending});
    }
    if !rows.is_empty() {return Err(ModelError::Schema("native receipt inventory"));}
    let [installation]=installations.as_slice() else {return Err(ModelError::Schema("native receipt installation"));};
    if hash_field(installation,"generation")?!=request.issuance.generation {return Err(ModelError::Conflict("native receipt generation"));}
    Ok(if request.issuance.era<=int_field(installation,"closed_through")? {EffectDisposition::CompactedTerminal} else {EffectDisposition::Pending})
}
pub async fn reconcile_request(client:&Surreal<Client>, request:NativeRequest)->Result<EffectDisposition,ModelError>{
    let mut bindings=Variables::new();request.bind(&mut bindings);
    // Reconciliation is an exact fence, permitted under current installation coordination
    // after an era cut. It never executes the predecessor's payload under new authority.
    super::run_transaction(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.generation!=$__generation { THROW 'native reconciliation generation'; }; UPDATE native_installation:current SET control_revision=(control_revision ?? 0)+1 RETURN NONE; LET $state=SELECT * FROM ONLY $__effect FOR UPDATE; IF $state=NONE { IF $__era>($installation.closed_through ?? 0) { IF $__era!=$installation.era { THROW 'native reconciliation era'; }; CREATE $__effect SET attempt=NONE,owner=NONE,owner_kind='fence',generation=$__generation,era=$__era,request=$__request,committed=false,resolved=true,revision=1,epoch=0 RETURN NONE; }; } ELSE { IF $state.request!=$__request OR $state.era!=$__era OR $state.generation!=$__generation { THROW 'native reconciliation identity'; }; IF !$state.resolved { UPDATE $__effect SET resolved=true,revision+=1 RETURN NONE; }; }; COMMIT;",bindings).await.map_err(|error|request.error(error))?;
    request_disposition(client,request).await
}
/// Legacy ID-only observation cannot invent missing authorization. Callers that may retry
/// use reconcile_request and retain NativeRequest, including its pre-intent era.
pub async fn reconcile_effect(client:&Surreal<Client>,operation:ContentHash)->Result<bool,ModelError>{
    let mut bindings=Variables::new();bindings.insert("effect",RecordId::new("native_effect",operation.hex()));
    let mut response=client.query("SELECT * FROM $effect").bind(bindings.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else{return Err(ModelError::Conflict("native reconciliation requires original request"));};
    // Migrated legacy receipts retain exact outcomes and are never collected automatically.
    if int_field(row,"era")?==0 {
        super::run_transaction(client,"BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; UPDATE native_installation:current SET control_revision=(control_revision ?? 0)+1 RETURN NONE; LET $state=SELECT * FROM ONLY $effect FOR UPDATE; IF !$state.resolved { UPDATE $effect SET resolved=true,revision+=1 RETURN NONE; }; COMMIT;",bindings).await?;
        return Ok(row.get("committed")==Some(&Value::Bool(true)));
    }
    let request=NativeRequest{issuance:IssuanceEra{generation:hash_field(row,"generation")?,era:int_field(row,"era")?},operation,request:hash_field(row,"request")?};
    Ok(reconcile_request(client,request).await?==EffectDisposition::Committed)
}
