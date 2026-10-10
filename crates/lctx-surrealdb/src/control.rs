//! Durable native ownership. Every reference transition writes the same small object guard
//! as retirement; snapshot reads and predicate scans alone do not provide this protection.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError, completed::RetirementEligibility};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Object, RecordId, Value, Variables},
};

// Failed transactions include NotExecuted frames before the causal error. Inspect every
// statement so contention remains retryable and permanent refusals retain their actual cause.
fn checked_transaction(
    mut response: surrealdb::IndexedResults,
) -> Result<surrealdb::IndexedResults, ModelError> {
    transaction_errors(response.take_errors().into_iter().collect())?;
    Ok(response)
}
fn transaction_errors(mut errors: Vec<(usize, surrealdb::Error)>) -> Result<(), ModelError> {
    errors.sort_by_key(|(index, _)| *index);
    let decisive = errors
        .iter()
        .position(|(_, error)| {
            !matches!(
                error.query_details(),
                Some(surrealdb::types::QueryError::NotExecuted)
            ) && !retryable_conflict(error)
        })
        .or_else(|| {
            errors
                .iter()
                .position(|(_, error)| retryable_conflict(error))
        })
        .or_else(|| (!errors.is_empty()).then_some(0));
    let Some(index) = decisive else {
        return Ok(());
    };
    let primary = ModelError::Cause(Box::new(errors.remove(index).1));
    let mut completion = lctx_model::domain::completion::Completion::default();
    for (_, error) in errors {
        completion.step(
            "native transaction secondary statement",
            Err(ModelError::Cause(Box::new(error))),
        );
    }
    lctx_model::domain::completion::complete(Err(primary), completion)
}
fn retryable_transaction(error: &ModelError) -> bool {
    fn native(error: &ModelError) -> Option<&surrealdb::Error> {
        match error {
            ModelError::Cause(cause) => cause.downcast_ref(),
            _ => None,
        }
    }
    match error {
        ModelError::Completion(outcome) => {
            outcome
                .primary
                .as_deref()
                .and_then(native)
                .is_some_and(retryable_conflict)
                && outcome.completion.failures.iter().all(|failure| {
                    native(&failure.error).is_some_and(|error| {
                        matches!(
                            error.query_details(),
                            Some(surrealdb::types::QueryError::NotExecuted)
                        ) || retryable_conflict(error)
                    })
                })
        }
        _ => native(error).is_some_and(retryable_conflict),
    }
}
fn retryable_conflict(error: &surrealdb::Error) -> bool {
    error.query_details() == Some(&surrealdb::types::QueryError::TransactionConflict)
        || (error.is_query()
            && error.query_details().is_none()
            && error.message()
                == "There was a problem with the key-value store: Transaction conflict: Resource busy. This transaction can be retried")
}

pub fn fresh_identity(kind: &str) -> Result<ContentHash, ModelError> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let mut sink = KeySink::new("native-durable-operation/v1");
    kind.to_string().encode(&mut sink);
    sink.part(
        b"clock",
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(ModelError::codec)?
            .as_nanos()
            .to_le_bytes(),
    );
    sink.part(b"process", &std::process::id().to_le_bytes());
    sink.part(
        b"sequence",
        &SEQUENCE.fetch_add(1, Ordering::Relaxed).to_le_bytes(),
    );
    Ok(sink.finish())
}
pub fn schema() -> &'static str {
    "DEFINE TABLE native_installation SCHEMAFULL; DEFINE FIELD generation ON native_installation TYPE string; DEFINE FIELD schema ON native_installation TYPE string; DEFINE FIELD schema_version ON native_installation TYPE int; DEFINE FIELD admission_open ON native_installation TYPE bool; DEFINE FIELD backup_revision ON native_installation TYPE option<int>; DEFINE FIELD admission_revision ON native_installation TYPE option<int>;
    DEFINE TABLE native_attempt SCHEMAFULL; DEFINE FIELD generation ON native_attempt TYPE string; DEFINE FIELD state ON native_attempt TYPE string; DEFINE FIELD revision ON native_attempt TYPE int; DEFINE FIELD admitted ON native_attempt TYPE bool; DEFINE FIELD admitted_view ON native_attempt TYPE option<string>; DEFINE FIELD epoch ON native_attempt TYPE int;
    DEFINE TABLE native_effect SCHEMAFULL; DEFINE FIELD attempt ON native_effect TYPE option<record<native_attempt>>; DEFINE FIELD request ON native_effect TYPE string; DEFINE FIELD committed ON native_effect TYPE bool; DEFINE FIELD resolved ON native_effect TYPE bool; DEFINE FIELD revision ON native_effect TYPE int; DEFINE FIELD epoch ON native_effect TYPE int;
    DEFINE TABLE native_guard SCHEMAFULL; DEFINE FIELD revision ON native_guard TYPE int; DEFINE FIELD retired ON native_guard TYPE bool; DEFINE FIELD retired_through ON native_guard TYPE option<int>;
    DEFINE TABLE native_hold SCHEMAFULL; DEFINE FIELD owner ON native_hold TYPE record; DEFINE FIELD object ON native_hold TYPE record; DEFINE INDEX object_holds ON native_hold FIELDS object,owner UNIQUE; DEFINE INDEX owner_holds ON native_hold FIELDS owner,object;
    DEFINE TABLE native_pin SCHEMAFULL; DEFINE FIELD views ON native_pin TYPE array<record<compiler_view>>; DEFINE FIELD released ON native_pin TYPE bool; DEFINE FIELD epoch ON native_pin TYPE int;
    DEFINE TABLE native_product SCHEMAFULL; DEFINE FIELD request ON native_product TYPE string; DEFINE FIELD contribution ON native_product TYPE record<compiler_contribution>; DEFINE INDEX request_product ON native_product FIELDS request; DEFINE INDEX contribution_products ON native_product FIELDS contribution;
    DEFINE TABLE native_retirement SCHEMAFULL; DEFINE FIELD examined ON native_retirement TYPE int; DEFINE FIELD retired ON native_retirement TYPE int; DEFINE FIELD revision ON native_retirement TYPE int; DEFINE TABLE native_retirement_item SCHEMAFULL; DEFINE FIELD job ON native_retirement_item TYPE record<native_retirement>; DEFINE FIELD object ON native_retirement_item TYPE record; DEFINE FIELD state ON native_retirement_item TYPE string; DEFINE INDEX retirement_queue ON native_retirement_item FIELDS job,state; DEFINE TABLE native_backup_hold SCHEMAFULL; DEFINE FIELD active ON native_backup_hold TYPE bool;"
}
pub async fn check_installation(
    client: &Surreal<Client>,
    generation: ContentHash,
) -> Result<(), ModelError> {
    let mut response = client
        .query("SELECT generation,schema,schema_version FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let actual: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = actual.as_slice() else {
        return Err(ModelError::Conflict("installed native service marker"));
    };
    if row.get("generation") != Some(&Value::String(generation.hex()))
        || row.get("schema")
            != Some(&Value::String(
                crate::compiler::base_schema_identity().hex(),
            ))
        || row.get("schema_version") != Some(&Value::Number(surrealdb::types::Number::Int(3)))
    {
        return Err(ModelError::Conflict(
            "installed native service generation/schema",
        ));
    }
    Ok(())
}
pub async fn begin_attempt(
    client: &Surreal<Client>,
    attempt: ContentHash,
    generation: ContentHash,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert("generation", generation.hex());
    loop {
        match client.query("BEGIN; LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF !$installation.admission_open OR $installation.generation!=$generation { THROW 'native installation admission closed'; }; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $attempt SET generation=$generation,state='open',revision=0,admitted=false,epoch=($installation.admission_revision ?? 0)+1 RETURN NONE; COMMIT;").bind(bindings.clone()).await {
        Ok(response)=>match checked_transaction(response){Ok(_)=>return Ok(()),Err(error) if retryable_transaction(&error)=>{tokio::task::yield_now().await;continue;},Err(error)=>return Err(error)},Err(error)=>return Err(crate::loader::write_failure(error)),
    }
    }
}
/// Execute one already identified effect. Intent remains durable when acknowledgment is lost.
/// A query transport failure is reconciled by the same operation ID, never blindly replayed.
pub async fn effect(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(
        client,
        attempt
            .map(EffectOwner::OpenAttempt)
            .unwrap_or(EffectOwner::Installation),
        sql,
        bindings,
    )
    .await
}
// Cleanup can remove only this irreversible terminal owner's existing roots.
// Reuse its original epoch instead of allocating a global installation epoch per page.
#[derive(Clone, Copy)]
struct TerminalAttempt {
    identity: ContentHash,
    epoch: i64,
}
enum EffectOwner {
    Installation,
    OpenAttempt(ContentHash),
    Pin(RecordId),
    TerminalAttempt(TerminalAttempt),
}

/// Intent permanently records the authorization epoch. Retry never borrows a newer
/// installation/attempt/pin epoch to make an old operation eligible after retirement.
async fn effect_for_owner(
    client: &Surreal<Client>,
    owner: EffectOwner,
    sql: &str,
    mut bindings: Variables,
) -> Result<(), ModelError> {
    if matches!(&owner, EffectOwner::TerminalAttempt(terminal) if terminal.epoch <= 0) {
        return Err(ModelError::Invalid("native terminal cleanup epoch".into()));
    }
    let attempt = match &owner {
        EffectOwner::OpenAttempt(identity) => Some(*identity),
        EffectOwner::TerminalAttempt(terminal) => Some(terminal.identity),
        _ => None,
    };
    let pin = match &owner {
        EffectOwner::Pin(pin) => Some(pin.clone()),
        _ => None,
    };
    if let EffectOwner::TerminalAttempt(terminal) = &owner {
        bindings.insert("__expected_epoch", terminal.epoch);
    }
    let operation = fresh_identity("effect")?;
    let request =
        ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).map_err(ModelError::codec)?);
    bindings.insert("__effect", RecordId::new("native_effect", operation.hex()));
    bindings.insert("__request", request.hex());
    bindings.insert(
        "__attempt",
        attempt
            .map(|id| Value::RecordId(RecordId::new("native_attempt", id.hex())))
            .unwrap_or(Value::None),
    );
    bindings.insert(
        "__pin",
        pin.clone().map(Value::RecordId).unwrap_or(Value::None),
    );
    let authorization = match &owner {
        EffectOwner::OpenAttempt(_) => {
            "LET $__authorized=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__authorized.state!='open' { THROW 'native attempt fenced'; }; LET $__epoch=$__authorized.epoch;"
        }
        EffectOwner::Pin(_) => {
            "LET $__authorized=SELECT * FROM ONLY $__pin FOR UPDATE; IF $__authorized=NONE OR $__authorized.released { THROW 'native reader pin fenced'; }; LET $__epoch=$__authorized.epoch;"
        }
        EffectOwner::TerminalAttempt(_) => {
            "LET $__authorized=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__authorized=NONE OR $__authorized.epoch!=$__expected_epoch OR $__authorized.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] { THROW 'native terminal cleanup fenced'; }; LET $__epoch=$__authorized.epoch;"
        }
        EffectOwner::Installation => {
            "LET $__installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $__installation=NONE { THROW 'native installation missing'; }; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; LET $__epoch=($__installation.admission_revision ?? 0)+1;"
        }
    };
    let intent = format!(
        "BEGIN; {authorization} IF $__epoch=NONE OR $__epoch<=0 {{ THROW 'native authorization epoch missing'; }}; CREATE $__effect SET attempt=$__attempt,request=$__request,committed=false,resolved=false,revision=0,epoch=$__epoch RETURN NONE; COMMIT;"
    );
    loop {
        match client.query(intent.clone()).bind(bindings.clone()).await {
            Ok(response) => match checked_transaction(response) {
                Ok(_) => break,
                Err(error) if retryable_transaction(&error) => {
                    tokio::task::yield_now().await;
                    continue;
                }
                Err(error) => return Err(error),
            },
            Err(error) => {
                return match reconcile_effect(client, operation).await {
                    Ok(_) => Err(ModelError::Cause(Box::new(error))),
                    Err(_) => Err(crate::loader::write_failure(error)),
                };
            }
        }
    }
    let fence = match &owner {
        EffectOwner::OpenAttempt(_) => {
            "LET $__owner=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__owner.state!='open' OR $__owner.epoch!=$__epoch { THROW 'native attempt fenced'; };"
        }
        EffectOwner::Pin(_) => {
            "LET $__pin_owner=SELECT * FROM ONLY $__pin FOR UPDATE; IF $__pin_owner=NONE OR $__pin_owner.released OR $__pin_owner.epoch!=$__epoch { THROW 'native reader pin fenced'; };"
        }
        EffectOwner::TerminalAttempt(_) => {
            "LET $__owner=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__owner=NONE OR $__operation.attempt!=$__attempt OR $__epoch!=$__expected_epoch OR $__owner.epoch!=$__epoch OR $__owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] { THROW 'native terminal cleanup fenced'; };"
        }
        EffectOwner::Installation => "",
    };
    let query = format!(
        "BEGIN; LET $__operation=SELECT * FROM ONLY $__effect FOR UPDATE; IF $__operation=NONE OR $__operation.resolved {{ THROW 'native operation fenced'; }}; LET $__epoch=$__operation.epoch; IF $__epoch=NONE OR $__epoch<=0 {{ THROW 'native effect epoch missing'; }}; {fence} {sql}; UPDATE $__effect SET committed=true,resolved=true,revision+=1 RETURN NONE; COMMIT;"
    );
    loop {
        match client.query(query.clone()).bind(bindings.clone()).await {
            Ok(response) => match checked_transaction(response) {
                Ok(_) => return Ok(()),
                Err(error) if retryable_transaction(&error) => {
                    tokio::task::yield_now().await;
                    continue;
                }
                Err(error) => {
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "native effect reconciliation",
                        reconcile_effect(client, operation).await.map(|_| ()),
                    );
                    return lctx_model::domain::completion::complete(Err(error), completion);
                }
            },
            Err(error) => {
                return match reconcile_effect(client, operation).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(ModelError::Cause(Box::new(error))),
                    Err(_) => Err(crate::loader::write_failure(error)),
                };
            }
        }
    }
}
/// Reconciliation writes the effect guard. A prior remote transaction also writes it, so
/// either its commit is observed or the fence prevents that transaction from ever committing.
pub async fn reconcile_effect(
    client: &Surreal<Client>,
    operation: ContentHash,
) -> Result<bool, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("effect", RecordId::new("native_effect", operation.hex()));
    loop {
        match client.query("BEGIN; LET $state=SELECT * FROM ONLY $effect FOR UPDATE; IF $state=NONE { CREATE $effect SET attempt=NONE,request='fenced-before-intent-ack',committed=false,resolved=true,revision=1,epoch=0 RETURN NONE; } ELSE { IF !$state.resolved { UPDATE $effect SET resolved=true,revision+=1 RETURN NONE; }; }; COMMIT;").bind(bindings.clone()).await {
            Ok(response)=>match checked_transaction(response){Ok(_)=>break,Err(error) if retryable_transaction(&error)=>{tokio::task::yield_now().await;continue;},Err(error)=>return Err(error)},
            Err(error)=>return Err(crate::loader::write_failure(error)),
        }
    }
    let mut response = client
        .query("SELECT VALUE committed FROM $effect WHERE resolved=true")
        .bind(bindings)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let values: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    match values.as_slice() {
        [committed] => Ok(*committed),
        _ => Err(ModelError::Conflict("native effect reconciliation fence")),
    }
}
pub async fn check_pending_effects(client: &Surreal<Client>) -> Result<(), ModelError> {
    let mut response = client
        .query("SELECT VALUE id FROM native_effect WHERE resolved=false LIMIT 1")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let pending: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    if pending.is_empty() {
        Ok(())
    } else {
        Err(ModelError::infrastructure(
            lctx_model::domain::Infrastructure::Unconfirmed,
            "native effects need explicit maintenance reconciliation",
        ))
    }
}
pub async fn drain_effects(client: &Surreal<Client>) -> Result<(), ModelError> {
    loop {
        let mut response = client
            .query("SELECT VALUE id FROM native_effect WHERE resolved=false LIMIT 128")
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let pending: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        if pending.is_empty() {
            return Ok(());
        }
        for id in pending {
            let surrealdb::types::RecordIdKey::String(key) = id.key else {
                return Err(ModelError::Schema("native effect identity"));
            };
            let operation = ContentHash(
                hex::decode(key)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("native effect width"))?,
            );
            reconcile_effect(client, operation).await?;
        }
    }
}
/// Full native values are compared under the per-address guard, including concurrent first
/// insertion. Content hashes are addresses, not substitutes for collision checking.
pub async fn ensure_rows(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    rows: Vec<Value>,
) -> Result<(), ModelError> {
    ensure_rows_owned(client, attempt, None, rows).await
}
pub async fn ensure_rows_owned(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    contribution: Option<ContentHash>,
    rows: Vec<Value>,
) -> Result<(), ModelError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut guards = Vec::with_capacity(rows.len());
    for row in &rows {
        crate::loader::validate_native_row(row)?;
        let Value::Object(row) = row else {
            return Err(ModelError::Schema("native immutable object"));
        };
        let id = row
            .get("id")
            .ok_or(ModelError::Schema("native immutable identity"))?;
        guards.push(guard_id(id)?);
    }
    let mut bindings = Variables::new();
    bindings.insert("rows", rows);
    bindings.insert("guards", guards);
    bindings.insert(
        "contribution",
        contribution
            .map(|id| Value::RecordId(RecordId::new("compiler_contribution", id.hex())))
            .unwrap_or(Value::None),
    );
    let fence = if attempt.is_some() {
        "IF $__owner.admitted AND array::len($rows.filter(|$row|record::table($row.id) IN ['entity','assertion','compiler_record','compiler_alias','original','original_chunk']))>0 { THROW 'native canonical content already admitted'; }; IF $contribution != NONE { LET $producer=SELECT * FROM ONLY $contribution FOR UPDATE; IF $producer.completed OR $producer.attempt!=$__attempt { THROW 'native producing contribution fenced'; }; };"
    } else {
        ""
    };
    let sql = format!(
        "{fence} FOR $i IN 0..array::len($rows) {{ LET $row=$rows[$i]; LET $guard=$guards[$i]; LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; IF $__epoch<=($prior.retired_through ?? 0) {{ THROW 'native content epoch retired'; }}; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,retired_through=(retired_through ?? 0) RETURN NONE; LET $existing=SELECT * FROM ONLY $row.id; IF $existing != NONE AND $existing != $row {{ THROW 'native immutable address collision'; }}; IF $existing=NONE {{ IF record::table($row.id) IN ['participant','reference','lex_occurs','vec_occurs'] {{ INSERT RELATION $row RETURN NONE; }} ELSE {{ CREATE $row.id CONTENT $row RETURN NONE; }}; }}; }}"
    );
    effect(client, attempt, &sql, bindings).await
}
fn guard_id(id: &Value) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "native_guard",
        ContentHash::of(&serde_json::to_vec(id).map_err(ModelError::codec)?).hex(),
    ))
}

/// Bind an object to a live owner and protect it against concurrent retirement.
pub async fn hold(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    owner: RecordId,
    objects: Vec<RecordId>,
) -> Result<(), ModelError> {
    hold_many(
        client,
        attempt,
        objects
            .into_iter()
            .map(|object| (owner.clone(), object))
            .collect(),
    )
    .await
}
pub async fn hold_many(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    references: Vec<(RecordId, RecordId)>,
) -> Result<(), ModelError> {
    if references.is_empty() {
        return Ok(());
    }
    let mut rows = Vec::with_capacity(references.len());
    let mut guards = Vec::new();
    for (owner, object) in references {
        let mut sink = KeySink::new("native-hold/v1");
        sink.part(
            b"owner",
            &serde_json::to_vec(&owner).map_err(ModelError::codec)?,
        );
        sink.part(
            b"object",
            &serde_json::to_vec(&object).map_err(ModelError::codec)?,
        );
        let mut row = Object::new();
        row.insert("id", RecordId::new("native_hold", sink.finish().hex()));
        row.insert("owner", owner.clone());
        row.insert("object", object.clone());
        guards.push(guard_id(&Value::RecordId(object))?);
        guards.push(guard_id(&Value::RecordId(owner))?);
        rows.push(Value::Object(row));
    }
    guards.sort();
    guards.dedup();
    let pins = rows
        .iter()
        .filter_map(|row| {
            if let Value::Object(row) = row {
                match row.get("owner") {
                    Some(Value::RecordId(owner)) if owner.table.as_str() == "native_pin" => {
                        Some(owner.clone())
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<std::collections::BTreeSet<_>>();
    if pins.len() > 1 || (attempt.is_some() && !pins.is_empty()) {
        return Err(ModelError::Conflict(
            "native hold mixed authorization owners",
        ));
    }
    let mut bindings = Variables::new();
    bindings.insert("holds", rows);
    bindings.insert("guards", guards);
    let owner = if let Some(attempt) = attempt {
        EffectOwner::OpenAttempt(attempt)
    } else {
        pins.into_iter()
            .next()
            .map(EffectOwner::Pin)
            .unwrap_or(EffectOwner::Installation)
    };
    effect_for_owner(client,owner,"FOR $guard IN $guards { LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; IF $__epoch<=($prior.retired_through ?? 0) { THROW 'native attachment epoch retired'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,retired_through=(retired_through ?? 0) RETURN NONE; }; FOR $row IN $holds { IF (SELECT VALUE id FROM ONLY $row.object)=NONE { THROW 'native attachment target missing'; }; UPSERT $row.id CONTENT $row RETURN NONE; }",bindings).await
}

pub async fn pin_views(
    client: &Surreal<Client>,
    views: &[ContentHash],
) -> Result<ContentHash, ModelError> {
    let pin = fresh_identity("reader-pin")?;
    let owner = RecordId::new("native_pin", pin.hex());
    let objects = views
        .iter()
        .map(|view| RecordId::new("compiler_view", view.hex()))
        .collect::<Vec<_>>();
    let mut bindings = Variables::new();
    bindings.insert("pin", owner.clone());
    bindings.insert("views", objects.clone());
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF !$installation.admission_open { THROW 'native reader admission closed'; }; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $pin SET views=$views,released=false,epoch=$__epoch RETURN NONE",bindings).await?;
    if let Err(error) = hold(client, None, owner, objects).await {
        let _ = release_pin(client, pin).await;
        return Err(error);
    }
    Ok(pin)
}
pub async fn release_pin(client: &Surreal<Client>, pin: ContentHash) -> Result<(), ModelError> {
    release_pin_checked(client, pin, false).await
}
pub(crate) async fn release_pin_checked(
    client: &Surreal<Client>,
    pin: ContentHash,
    maintenance: bool,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    let id = RecordId::new("native_pin", pin.hex());
    bindings.insert("guard", guard_id(&Value::RecordId(id.clone()))?);
    bindings.insert("pin", id);
    bindings.insert("maintenance", maintenance);
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $maintenance { IF $installation.admission_open { THROW 'maintenance admission must remain closed'; }; IF (SELECT VALUE id FROM ONLY $pin)=NONE { THROW 'maintenance pin identity missing'; }; }; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; UPSERT $guard SET revision=(revision ?? 0)+1,retired=true,retired_through=math::max([retired_through ?? 0,$installation.admission_revision ?? 0]) RETURN NONE; UPDATE $pin SET released=true RETURN NONE; LET $pin_holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$pin; DELETE $pin_holds RETURN NONE",bindings).await
}

pub async fn close_attempt(
    client: &Surreal<Client>,
    attempt: ContentHash,
    state: &str,
) -> Result<(), ModelError> {
    if !matches!(state, "closed" | "abandoned" | "frozen") {
        return Err(ModelError::Invalid("native attempt terminal state".into()));
    }
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert("state", state.to_string());
    // Fence ordinary effects first. The terminal attempt and remaining ownership rows
    // are durable progress: interruption never requires reopening or replaying a write.
    effect(client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE { THROW 'native cleanup attempt missing'; }; IF $owner.state='open' { UPDATE $attempt SET state=$state,revision+=1 RETURN NONE; } ELSE IF $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] { THROW 'native cleanup attempt state'; };",bindings.clone()).await?;
    let mut response = client.query("SELECT VALUE admitted FROM $attempt; SELECT VALUE epoch FROM $attempt; SELECT VALUE state FROM $attempt").bind(bindings.clone()).await.map_err(crate::reader::sdk_error)?.check().map_err(crate::reader::sdk_error)?;
    let admitted: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    let epochs: Vec<i64> = response.take(1).map_err(ModelError::codec)?;
    let states: Vec<String> = response.take(2).map_err(ModelError::codec)?;
    let ([epoch], [terminal]) = (epochs.as_slice(), states.as_slice()) else {
        return Err(ModelError::Conflict("native cleanup attempt identity"));
    };
    if *epoch <= 0
        || !matches!(
            terminal.as_str(),
            "closed" | "abandoned" | "frozen" | "maintenance_fenced"
        )
    {
        return Err(ModelError::Conflict("native cleanup attempt fence"));
    }
    bindings.insert("attempt_epoch", *epoch);
    let authorization = TerminalAttempt {
        identity: attempt,
        epoch: *epoch,
    };
    match admitted.as_slice() {
        [true] => {
            return release_cleanup_holds(
                client,
                authorization,
                vec![RecordId::new("native_attempt", attempt.hex())],
                false,
            )
            .await;
        }
        [false] => {}
        _ => return Err(ModelError::Conflict("native cleanup attempt inventory")),
    }
    bindings.insert("after", Value::None);
    loop {
        let contributions = cleanup_ids(client,"SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt AND ($after=NONE OR id>$after) ORDER BY id LIMIT 128",bindings.clone()).await?;
        let Some(last) = contributions.last() else {
            break;
        };
        bindings.insert("after", last.clone());
        let mut product_bindings = bindings.clone();
        product_bindings.insert("contributions", contributions);
        loop {
            let products = cleanup_ids(client,"SELECT VALUE id FROM native_product WITH INDEX contribution_products WHERE contribution IN $contributions LIMIT 128",product_bindings.clone()).await?;
            if products.is_empty() {
                break;
            }
            // Keep the product itself until its outgoing ownership has drained.
            release_cleanup_holds(client, authorization, products.clone(), true).await?;
            let mut deletion = product_bindings.clone();
            deletion.insert("products", products);
            effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] OR $owner.admitted { THROW 'native cleanup product owner'; }; LET $owned=SELECT VALUE id FROM $products WHERE contribution IN $contributions; DELETE $owned RETURN NONE",deletion).await?;
        }
    }
    // These roots keep immutable contributor IDs discoverable across interrupted
    // product cleanup, even when concurrent retirement is making progress.
    release_cleanup_holds(
        client,
        authorization,
        vec![RecordId::new("native_attempt", attempt.hex())],
        true,
    )
    .await
}

async fn cleanup_ids(
    client: &Surreal<Client>,
    sql: &str,
    bindings: Variables,
) -> Result<Vec<RecordId>, ModelError> {
    let mut response = client
        .query(sql)
        .bind(bindings)
        .await
        .map_err(crate::reader::sdk_error)?
        .check()
        .map_err(crate::reader::sdk_error)?;
    response.take(0).map_err(ModelError::codec)
}

// One visible unresolved operation protects the entire cleanup scope. Each bounded
// step advances its existing receipt revision; only the final empty proof commits it.
struct CleanupHolds {
    operation: ContentHash,
    bindings: Variables,
}
impl CleanupHolds {
    async fn begin(
        client: &Surreal<Client>,
        authorization: TerminalAttempt,
        owners: Vec<RecordId>,
        unadmitted: bool,
    ) -> Result<Self, ModelError> {
        if authorization.epoch <= 0 {
            return Err(ModelError::Invalid("native terminal cleanup epoch".into()));
        }
        let operation = fresh_identity("cleanup-holds")?;
        let mut bindings = Variables::new();
        bindings.insert(
            "attempt",
            RecordId::new("native_attempt", authorization.identity.hex()),
        );
        bindings.insert("attempt_epoch", authorization.epoch);
        bindings.insert("owners", owners);
        bindings.insert("unadmitted", unadmitted);
        let request = ContentHash::of(
            &serde_json::to_vec(&("native terminal hold cleanup/v1", &bindings))
                .map_err(ModelError::codec)?,
        );
        bindings.insert("effect", RecordId::new("native_effect", operation.hex()));
        bindings.insert("request", request.hex());
        loop {
            match client.query("BEGIN; LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] OR ($unadmitted AND $owner.admitted) { THROW 'native cleanup hold owner'; }; CREATE $effect SET attempt=$attempt,request=$request,committed=false,resolved=false,revision=0,epoch=$attempt_epoch RETURN NONE; COMMIT;").bind(bindings.clone()).await {
                Ok(response) => match checked_transaction(response) {
                    Ok(_) => return Ok(Self { operation, bindings }),
                    Err(error) if retryable_transaction(&error) => { tokio::task::yield_now().await; }
                    Err(error) => return Err(error),
                },
                Err(error) => {
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step("native cleanup intent reconciliation", reconcile_effect(client, operation).await.map(|_| ()));
                    return lctx_model::domain::completion::complete(Err(crate::loader::write_failure(error)), completion);
                }
            }
        }
    }
    fn statements() -> [&'static str; 12] {
        [
            "BEGIN",
            "LET $operation=SELECT * FROM ONLY $effect FOR UPDATE",
            "IF $operation=NONE OR $operation.resolved OR $operation.request!=$request OR $operation.attempt!=$attempt OR $operation.epoch!=$attempt_epoch { THROW 'native cleanup operation fenced'; }",
            "LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE",
            "IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] OR ($unadmitted AND $owner.admitted) { THROW 'native cleanup hold owner'; }",
            "LET $holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner IN $owners LIMIT 128",
            "LET $owned=SELECT VALUE id FROM $holds WHERE owner IN $owners",
            "DELETE $owned RETURN NONE",
            "LET $done=(array::len($holds)=0)",
            "UPDATE $effect SET committed=$done,resolved=$done,revision+=1 RETURN NONE",
            "SELECT VALUE resolved FROM $effect",
            "COMMIT",
        ]
    }
    async fn reconciled_failure(
        &self,
        client: &Surreal<Client>,
        error: ModelError,
    ) -> Result<bool, ModelError> {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "native cleanup step reconciliation",
            reconcile_effect(client, self.operation).await.map(|_| ()),
        );
        lctx_model::domain::completion::complete(Err(error), completion)
    }
    async fn step(&self, client: &Surreal<Client>) -> Result<bool, ModelError> {
        let statements = Self::statements();
        let sql = statements.join(";") + ";";
        loop {
            match client.query(sql.clone()).bind(self.bindings.clone()).await {
                Ok(response) => match checked_transaction(response) {
                    Ok(mut response) => {
                        let result = if response.num_statements() != statements.len() {
                            Err(ModelError::Schema("native cleanup statement inventory"))
                        } else {
                            response
                                .take::<Vec<bool>>(statements.len() - 2)
                                .map_err(ModelError::codec)
                                .and_then(|values| match values.as_slice() {
                                    [done] => Ok(*done),
                                    _ => Err(ModelError::Schema(
                                        "native cleanup completion inventory",
                                    )),
                                })
                        };
                        return match result {
                            Ok(done) => Ok(done),
                            Err(error) => self.reconciled_failure(client, error).await,
                        };
                    }
                    Err(error) if retryable_transaction(&error) => {
                        tokio::task::yield_now().await;
                    }
                    Err(error) => {
                        return self.reconciled_failure(client, error).await;
                    }
                },
                Err(error) => {
                    return match reconcile_effect(client, self.operation).await {
                        Ok(true) => Ok(true),
                        Ok(false) => Err(ModelError::Cause(Box::new(error))),
                        Err(reconciliation) => {
                            let mut completion =
                                lctx_model::domain::completion::Completion::default();
                            completion
                                .step("native cleanup step reconciliation", Err(reconciliation));
                            lctx_model::domain::completion::complete(
                                Err(crate::loader::write_failure(error)),
                                completion,
                            )
                        }
                    };
                }
            }
        }
    }
}
async fn release_cleanup_holds(
    client: &Surreal<Client>,
    authorization: TerminalAttempt,
    owners: Vec<RecordId>,
    unadmitted: bool,
) -> Result<(), ModelError> {
    let cleanup = CleanupHolds::begin(client, authorization, owners, unadmitted).await?;
    while !cleanup.step(client).await? {}
    Ok(())
}
pub async fn acquire_backup_hold(client: &Surreal<Client>) -> Result<ContentHash, ModelError> {
    let id = fresh_identity("database-backup-hold")?;
    let mut bindings = Variables::new();
    bindings.insert("hold", RecordId::new("native_backup_hold", id.hex()));
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF !$installation.admission_open { THROW 'native backup admission closed'; }; UPDATE native_installation:current SET backup_revision=(backup_revision ?? 0)+1,admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $hold SET active=true RETURN NONE",bindings).await?;
    Ok(id)
}
pub async fn release_backup_hold(
    client: &Surreal<Client>,
    id: ContentHash,
) -> Result<(), ModelError> {
    release_backup_hold_checked(client, id, false).await
}
pub(crate) async fn release_backup_hold_checked(
    client: &Surreal<Client>,
    id: ContentHash,
    maintenance: bool,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("hold", RecordId::new("native_backup_hold", id.hex()));
    bindings.insert("maintenance", maintenance);
    effect(client,None,"IF $maintenance { LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installation.admission_open { THROW 'maintenance admission must remain closed'; }; IF (SELECT VALUE id FROM ONLY $hold)=NONE { THROW 'maintenance backup identity missing'; }; }; UPDATE $hold SET active=false RETURN NONE",bindings).await
}

pub async fn retirement_eligibility(
    client: &Surreal<Client>,
    object: RecordId,
) -> Result<RetirementEligibility, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("object", object);
    let mut response=client.query("SELECT VALUE id FROM native_hold WHERE object=$object LIMIT 1; SELECT VALUE id FROM native_backup_hold WHERE active=true LIMIT 1").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let holds: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    let backups: Vec<RecordId> = response.take(1).map_err(ModelError::codec)?;
    let mut reasons = Vec::new();
    if !backups.is_empty() {
        reasons.push("active native backup hold".into());
    }
    if !holds.is_empty() {
        reasons.push("reachable through native owner hold".into());
    }
    Ok(RetirementEligibility {
        eligible: reasons.is_empty(),
        reasons,
    })
}
/// Bounded leaf retirement. All references and immutable writes take this exact guard,
/// so unrelated active effects need not be globally drained. A late competing effect
/// conflicts or observes the retired guard; unknown effects remain a maintenance concern.
/// Higher-level reachability removal must first remove its own outgoing holds.
pub async fn retire(client: &Surreal<Client>, object: RecordId) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("guard", guard_id(&Value::RecordId(object.clone()))?);
    bindings.insert("object", object);
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; UPDATE native_installation:current SET backup_revision=(backup_revision ?? 0)+1,admission_revision=(admission_revision ?? 0)+1 RETURN NONE; IF array::len(SELECT VALUE id FROM native_backup_hold WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=true,retired_through=math::max([retired_through ?? 0,$installation.admission_revision ?? 0]) RETURN NONE; IF array::len(SELECT VALUE id FROM native_hold WHERE object=$object LIMIT 1)>0 { THROW 'native retirement reachable'; }; DELETE $object RETURN NONE; LET $owner_holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object; DELETE $owner_holds RETURN NONE",bindings).await
}

/// Retire a dependency closure in bounded passes. Every removed parent releases its outgoing
/// references; children shared by another owner remain reachable and are reported as retained.
/// The caller supplies exact roots (normally publications), never a database-wide deletion.
pub async fn retire_reachable(
    client: &Surreal<Client>,
    mut roots: Vec<RecordId>,
    limit: usize,
) -> Result<lctx_model::domain::completed::RetirementProgress, ModelError> {
    if limit == 0 {
        return Err(ModelError::Invalid("bounded retirement limit".into()));
    }
    roots.sort();
    roots.dedup();
    let identity = ContentHash::of(
        &serde_json::to_vec(&("native-retirement/v2", &roots)).map_err(ModelError::codec)?,
    );
    let job = RecordId::new("native_retirement", identity.hex());
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    effect(client,None,"LET $job_state=SELECT * FROM ONLY $job FOR UPDATE; IF $job_state=NONE { CREATE $job SET examined=0,retired=0,revision=0 RETURN NONE; };",bindings).await?;
    for window in roots.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        enqueue_retirement(client, &job, window.to_vec()).await?;
    }
    resume_retirement(client, identity, limit).await
}
async fn enqueue_retirement(
    client: &Surreal<Client>,
    job: &RecordId,
    objects: Vec<RecordId>,
) -> Result<(), ModelError> {
    if objects.is_empty() {
        return Ok(());
    }
    let mut rows = Vec::new();
    for object in objects {
        let key = ContentHash::of(&serde_json::to_vec(&(job, &object)).map_err(ModelError::codec)?);
        let mut row = Object::new();
        row.insert("id", RecordId::new("native_retirement_item", key.hex()));
        row.insert("job", job.clone());
        row.insert("object", object);
        row.insert("state", "pending");
        rows.push(Value::Object(row));
    }
    let mut bindings = Variables::new();
    bindings.insert("rows", rows);
    effect(client,None,"FOR $row IN $rows { LET $row_id=$row.id; LET $existing=SELECT * FROM ONLY $row_id FOR UPDATE; IF $existing=NONE { CREATE $row.id CONTENT $row RETURN NONE; } ELSE { IF $existing.state='done' { UPDATE $row.id SET state='pending' RETURN NONE; }; }; };",bindings).await
}
async fn reconsider_retirement(client: &Surreal<Client>, job: &RecordId) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    effect(client,None,"UPDATE native_retirement_item SET state='pending' WHERE job=$job AND state='retained' RETURN NONE",bindings).await
}
// Candidate classification is only a preparation hint. The final effect rechecks each
// exact item/guard/incoming hold; changed guards stay pending for a fresh nomination.
async fn retirement_candidates(
    client: &Surreal<Client>,
    job: &RecordId,
    limit: usize,
) -> Result<Vec<Object>, ModelError> {
    let response = client.query("SELECT id,object FROM native_retirement_item WITH INDEX retirement_queue WHERE job=$job AND state='pending' ORDER BY id LIMIT $limit")
        .bind(("job", job.clone())).bind(("limit", limit)).await.map_err(ModelError::codec).and_then(|response| response.check().map_err(ModelError::codec));
    let mut response = response?;
    let mut candidates: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    if candidates.is_empty() {
        return Ok(candidates);
    }
    let mut sql = String::new();
    let mut bindings = Variables::new();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        let Some(Value::RecordId(object)) = candidate.get("object") else {
            return Err(ModelError::Schema("native retirement object"));
        };
        if !matches!(candidate.get("id"), Some(Value::RecordId(_))) {
            return Err(ModelError::Schema("native retirement item"));
        }
        let guard = guard_id(&Value::RecordId(object.clone()))?;
        bindings.insert(format!("object_{index}"), object.clone());
        bindings.insert(format!("guard_{index}"), guard.clone());
        candidate.insert("guard", guard);
        sql.push_str(&format!("SELECT VALUE revision FROM $guard_{index}; SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object_{index} LIMIT 1;"));
    }
    let response = client
        .query(sql)
        .bind(bindings)
        .await
        .map_err(ModelError::codec)
        .and_then(|response| response.check().map_err(ModelError::codec));
    let mut response = response?;
    if response.num_statements() != candidates.len() * 2 {
        return Err(ModelError::Schema(
            "native retirement classification terminals",
        ));
    }
    for (index, candidate) in candidates.iter_mut().enumerate() {
        let revisions: Vec<i64> = response.take(index * 2).map_err(ModelError::codec)?;
        let held: Vec<RecordId> = response.take(index * 2 + 1).map_err(ModelError::codec)?;
        let revision = match revisions.as_slice() {
            [] => 0,
            [revision] => *revision,
            _ => return Err(ModelError::Schema("native retirement guard revision")),
        };
        candidate.insert("revision", revision);
        candidate.insert("prepared", held.is_empty());
    }
    Ok(candidates)
}
async fn prepare_retirement_children(
    client: &Surreal<Client>,
    job: &RecordId,
    candidates: &[Object],
) -> Result<(), ModelError> {
    let owners = candidates
        .iter()
        .filter(|candidate| candidate.get("prepared") == Some(&Value::Bool(true)))
        .map(|candidate| {
            candidate
                .get("object")
                .cloned()
                .ok_or(ModelError::Schema("native retirement object"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if owners.is_empty() {
        return Ok(());
    }
    // Exact owner equalities retain the compound-index prefix access. A grouped IN
    // with ordering can select an unfiltered index scan in the pinned planner.
    // One streamed request still checks every owner's statement and the outer EOF.
    let statements = owners.len();
    let mut sql = String::new();
    let mut bindings = Variables::new();
    for (index, owner) in owners.into_iter().enumerate() {
        bindings.insert(format!("owner_{index}"), owner);
        sql.push_str(&format!("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$owner_{index};"));
    }
    let mut children = crate::reader::NativeRows::new(client.query(sql)
        .bind(bindings).stream_items().map_err(ModelError::codec)?, statements)?;
    let result = async {
        let mut window = Vec::new();
        while let Some(child) = children.next().await? {
            let Value::RecordId(child) = child else {
                return Err(ModelError::Schema("native retirement child"));
            };
            window.push(child);
            if window.len() == crate::loader::NATIVE_WINDOW_ROWS {
                enqueue_retirement(client, job, std::mem::take(&mut window)).await?;
            }
        }
        enqueue_retirement(client, job, window).await
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "native retirement child stream drainage",
        children.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)
}
async fn finish_retirement_window(
    client: &Surreal<Client>,
    job: &RecordId,
    candidates: Vec<Object>,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    bindings.insert(
        "candidates",
        candidates
            .into_iter()
            .map(Value::Object)
            .collect::<Vec<_>>(),
    );
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; UPDATE native_installation:current SET backup_revision=(backup_revision ?? 0)+1,admission_revision=(admission_revision ?? 0)+1 RETURN NONE; IF array::len(SELECT VALUE id FROM native_backup_hold WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; FOR $candidate IN $candidates { LET $item=$candidate.id; LET $object=$candidate.object; LET $guard=$candidate.guard; LET $queued=SELECT * FROM ONLY $item FOR UPDATE; IF $queued.job=$job AND $queued.object=$object AND $queued.state='pending' { LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF ($before.revision ?? 0)=$candidate.revision { LET $held=array::len(SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1)>0; IF $held OR $candidate.prepared { UPSERT $guard SET revision=(revision ?? 0)+1,retired=(retired ?? false) RETURN NONE; IF $held { UPDATE $item SET state='retained' RETURN NONE; UPDATE $job SET examined+=1,revision+=1 RETURN NONE; } ELSE { UPDATE $guard SET retired=true,retired_through=math::max([retired_through ?? 0,$installation.admission_revision ?? 0]) RETURN NONE; DELETE $object RETURN NONE; LET $owner_holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object; DELETE $owner_holds RETURN NONE; UPDATE $item SET state='done' RETURN NONE; UPDATE $job SET examined+=1,retired+=1,revision+=1 RETURN NONE; }; }; }; }; };",bindings).await
}
pub async fn resume_retirement(
    client: &Surreal<Client>,
    identity: ContentHash,
    limit: usize,
) -> Result<lctx_model::domain::completed::RetirementProgress, ModelError> {
    if limit == 0 {
        return Err(ModelError::Invalid("bounded retirement limit".into()));
    }
    let job = RecordId::new("native_retirement", identity.hex());
    // An explicit later pass reconsiders retained objects after reader/publication holds end.
    reconsider_retirement(client, &job).await?;
    let mut attempted = 0;
    while attempted < limit {
        let candidates = retirement_candidates(
            client,
            &job,
            (limit - attempted).min(crate::loader::NATIVE_WINDOW_ROWS),
        )
        .await?;
        if candidates.is_empty() {
            break;
        }
        attempted += candidates.len();
        // Children become durable work before any eligible parent's holds disappear.
        // A retained parent keeps its entire subtree reachable without walking it now.
        let result = prepare_retirement_children(client, &job, &candidates).await;
        result?;
        let result = finish_retirement_window(client, &job, candidates).await;
        result?;
    }
    let mut response=client.query("SELECT examined,retired FROM $job; SELECT VALUE object FROM native_retirement_item WHERE job=$job AND state='pending' ORDER BY id LIMIT 128; SELECT VALUE object FROM native_retirement_item WHERE job=$job AND state='retained' ORDER BY id LIMIT 128").bind(("job",job)).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict("native retirement progress"));
    };
    let pending: Vec<RecordId> = response.take(1).map_err(ModelError::codec)?;
    let retained: Vec<RecordId> = response.take(2).map_err(ModelError::codec)?;
    let count = |field| match row.get(field) {
        Some(Value::Number(surrealdb::types::Number::Int(count))) => {
            u64::try_from(*count).map_err(ModelError::codec)
        }
        _ => Err(ModelError::Schema("native retirement count")),
    };
    let identities = |values: Vec<RecordId>| {
        values
            .into_iter()
            .map(|value| serde_json::to_string(&value).map_err(ModelError::codec))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(lctx_model::domain::completed::RetirementProgress {
        identity,
        retired: count("retired")?,
        examined: count("examined")?,
        remaining: identities(pending)?,
        retained: identities(retained)?,
    })
}

/// Arc ownership ensures a pin is shared by cloned readers, rather than reacquired per clone.
pub struct ReaderPin {
    pub identity: ContentHash,
    client: Arc<Surreal<Client>>,
    runtime: tokio::runtime::Handle,
    released: std::sync::atomic::AtomicBool,
    uncertain: std::sync::atomic::AtomicBool,
}
impl ReaderPin {
    pub async fn acquire(
        client: Arc<Surreal<Client>>,
        views: &[ContentHash],
    ) -> Result<Arc<Self>, ModelError> {
        let identity = pin_views(&client, views).await?;
        Ok(Arc::new(Self {
            identity,
            client,
            runtime: tokio::runtime::Handle::current(),
            released: std::sync::atomic::AtomicBool::new(false),
            uncertain: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    pub async fn protect(&self, object: RecordId) -> Result<(), ModelError> {
        hold(
            &self.client,
            None,
            RecordId::new("native_pin", self.identity.hex()),
            vec![object],
        )
        .await
    }
    pub fn retain_unknown(&self) {
        self.uncertain.store(true, Ordering::Release);
    }
    pub async fn release(&self) -> Result<(), ModelError> {
        if self.uncertain.load(Ordering::Acquire) {
            return Err(ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                "native reader transport remains uncertain",
            ));
        }
        if !self.released.load(Ordering::Acquire) {
            release_pin(&self.client, self.identity).await?;
            self.released.store(true, Ordering::Release);
        }
        Ok(())
    }
}
impl Drop for ReaderPin {
    fn drop(&mut self) {
        if self.released.load(Ordering::Acquire) || self.uncertain.load(Ordering::Acquire) {
            return;
        }
        let client = self.client.clone();
        let pin = self.identity;
        self.runtime.spawn(async move {
            if let Err(error) = release_pin(&client, pin).await {
                tracing::warn!(%error,"native reader pin release remains unresolved");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn retirement_windows_skip_retained_subtrees_recheck_late_holds_and_resume_exact_limits()
    {
        let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_test_writer().try_init();
        macro_rules! observed {
            ($name:literal, $work:expr) => {{
                let phase = crate::phase::Phase::begin($name);
                let result = $work.await;
                phase.finish_result(&result);
                result
            }};
        }
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let mut client = crate::compiler::check_installation(&config).await.unwrap();
        let nonce = fresh_identity("retirement-window-control").unwrap();
        let parent = RecordId::new("native_guard", format!("parent_{}", nonce.hex()));
        let racer = RecordId::new("native_guard", format!("race_{}", nonce.hex()));
        let children = (0..257)
            .map(|n| RecordId::new("native_guard", format!("child_{}_{n}", nonce.hex())))
            .collect::<Vec<_>>();
        let pins = [
            ReaderPin::acquire(client.clone(), &[]).await.unwrap(),
            ReaderPin::acquire(client.clone(), &[]).await.unwrap(),
        ];
        let result = async {
            for window in [parent.clone(), racer.clone()].into_iter().chain(children.iter().cloned()).collect::<Vec<_>>().chunks(128) {
                observed!("retirement_fixture_ensure", ensure_rows(&client, None, window.iter().map(|id| {
                    let mut row = Object::new(); row.insert("id", id.clone()); row.insert("revision", 0i64); row.insert("retired", false); Value::Object(row)
                }).collect()))?;
            }
            for window in children.chunks(128) { observed!("retirement_fixture_hold", hold(&client, None, parent.clone(), window.to_vec()))?; }
            pins[0].protect(parent.clone()).await?;
            pins[0].protect(racer.clone()).await?;
            let retained = observed!("retirement_held_parent_pass", retire_reachable(&client, vec![parent.clone()], 128))?;
            let job = RecordId::new("native_retirement", retained.identity.hex());
            let mut vars = Variables::new(); vars.insert("job", job.clone()); vars.insert("parent", parent.clone());
            let retained_queue = cleanup_ids(&client, "SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue WHERE job=$job", vars.clone()).await?;
            let race = observed!("retirement_race_initial_pass", retire_reachable(&client, vec![racer.clone()], 1))?;
            let race_job = RecordId::new("native_retirement", race.identity.hex());
            pins[0].release().await?;
            reconsider_retirement(&client, &race_job).await?;
            let nominated = retirement_candidates(&client, &race_job, 128).await?;
            observed!("retirement_race_prepare_children", prepare_retirement_children(&client, &race_job, &nominated))?;
            // An actual owner added after the child snapshot changes the exact guard.
            pins[1].protect(racer.clone()).await?;
            observed!("retirement_race_final_window", finish_retirement_window(&client, &race_job, nominated))?;
            vars.insert("job", race_job); vars.insert("parent", racer.clone());
            let pending_parent = cleanup_ids(&client, "SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue WHERE job=$job AND state='pending' AND object=$parent", vars.clone()).await?;
            let reader = crate::NativeReader::private(client.clone());
            let parent_after_race: Vec<RecordId> = reader.query_native("SELECT VALUE id FROM $parent", vars.clone()).await?;
            pins[1].release().await?;
            let race_finished = resume_retirement(&client, race.identity, 1).await?;
            vars.insert("job", job.clone()); vars.insert("parent", parent.clone());
            let parent_pass = observed!("retirement_parent_resume", resume_retirement(&client, retained.identity, 1))?;
            client.invalidate().await.map_err(ModelError::codec)?;
            // Durable queued children remain exact across an independent session.
            client = crate::reader::connect(&config.endpoint, &config.writer_credentials(), config.namespace.as_str(), config.database.as_str()).await?;
            let partial = observed!("retirement_leaf_partial_resume", resume_retirement(&client, retained.identity, 129))?;
            let pending_children = cleanup_ids(&client, "SELECT VALUE object FROM native_retirement_item WITH INDEX retirement_queue WHERE job=$job AND state='pending'", vars.clone()).await?;
            let finished = observed!("retirement_leaf_final_resume", resume_retirement(&client, retained.identity, 128))?;
            vars.insert("children", children.clone());
            let remaining = cleanup_ids(&client, "SELECT VALUE id FROM $children", vars).await?;
            Ok::<_, ModelError>((retained, retained_queue, pending_parent, parent_after_race, parent_pass, partial, pending_children, finished, remaining, race_finished))
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        for pin in pins {
            completion.step("retirement window control pin release", pin.release().await);
        }
        let cleanup_phase = crate::phase::Phase::begin("retirement_fixture_cleanup");
        completion.step(
            "retirement window control owned closure cleanup",
            retire_reachable(&client, vec![parent.clone(), racer.clone()], 4096)
                .await
                .map(|_| ()),
        );
        cleanup_phase.finish(if completion.failures.is_empty() { crate::phase::Terminal::Passed } else { crate::phase::Terminal::Failed });
        completion.step(
            "retirement window control session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let (
            retained,
            retained_queue,
            pending_parent,
            parent_after_race,
            parent_pass,
            partial,
            pending_children,
            finished,
            remaining,
            race_finished,
        ) = lctx_model::domain::completion::complete(result, completion).unwrap();
        assert_eq!(retained.examined, 1);
        assert_eq!(retained.retired, 0);
        assert_eq!(
            retained_queue,
            [parent.clone()],
            "held high-fanout parent must not enqueue its subtree"
        );
        assert_eq!(
            pending_parent,
            [racer.clone()],
            "changed guard keeps prepared parent pending"
        );
        assert_eq!(
            parent_after_race,
            [racer],
            "late actual hold prevents deletion"
        );
        assert_eq!(race_finished.retired, 1);
        assert_eq!(parent_pass.retired, 1);
        assert_eq!(
            partial.retired, 130,
            "129 attempted leaves span one full window plus one item"
        );
        assert_eq!(pending_children.len(), 128);
        assert_eq!(finished.retired, 258);
        assert!(finished.remaining.is_empty());
        assert!(finished.retained.is_empty());
        assert!(remaining.is_empty());
    }
    // Actual SDK transactions exercise partial durable cleanup, reconciliation fencing
    // of a later uncommitted page, and a fresh scope resuming the remaining ownership.
    #[tokio::test(flavor = "multi_thread")]
    async fn terminal_cleanup_scope_fences_partial_pages_and_resumes_with_fresh_intent() {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let client = crate::compiler::check_installation(&config).await.unwrap();
        let attempt = fresh_identity("partial-cleanup-control").unwrap();
        let unrelated = fresh_identity("partial-cleanup-unrelated").unwrap();
        begin_attempt(&client, attempt, config.service_generation)
            .await
            .unwrap();
        begin_attempt(&client, unrelated, config.service_generation)
            .await
            .unwrap();
        let owner = RecordId::new("native_attempt", attempt.hex());
        let other = RecordId::new("native_attempt", unrelated.hex());
        let reader = crate::NativeReader::private(client.clone());
        let mut operations = Vec::new();
        let result = async {
            let targets = (0..257)
                .map(|ordinal| {
                    RecordId::new(
                        "native_guard",
                        format!("partial_{}_{}", attempt.hex(), ordinal),
                    )
                })
                .collect::<Vec<_>>();
            for window in targets.chunks(128) {
                let rows = window
                    .iter()
                    .map(|id| {
                        let mut row = Object::new();
                        row.insert("id", id.clone());
                        row.insert("revision", 0i64);
                        row.insert("retired", false);
                        Value::Object(row)
                    })
                    .collect();
                ensure_rows(&client, Some(attempt), rows).await?;
                hold(&client, Some(attempt), owner.clone(), window.to_vec()).await?;
            }
            hold(
                &client,
                Some(unrelated),
                other.clone(),
                vec![targets[0].clone()],
            )
            .await?;
            let mut vars = Variables::new();
            vars.insert("attempt", owner.clone());
            effect(
                &client,
                None,
                "UPDATE $attempt SET state='frozen',revision+=1 RETURN NONE",
                vars.clone(),
            )
            .await?;
            let epochs: Vec<i64> = reader
                .query("SELECT VALUE epoch FROM $attempt", vars.clone())
                .await?;
            let [epoch] = epochs.as_slice() else {
                return Err(ModelError::Schema("partial cleanup epoch"));
            };
            let authorization = TerminalAttempt {
                identity: attempt,
                epoch: *epoch,
            };
            let first =
                CleanupHolds::begin(&client, authorization, vec![owner.clone()], true).await?;
            operations.push(first.operation);
            let partial_done = first.step(&client).await?;
            let partial: Vec<Object> = reader
                .query_native("SELECT * FROM $effect", first.bindings.clone())
                .await?;
            let remaining = cleanup_ids(
                &client,
                "SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt",
                vars.clone(),
            )
            .await?;
            let pending = client
                .as_ref()
                .clone()
                .begin()
                .await
                .map_err(ModelError::codec)?;
            let statements = CleanupHolds::statements();
            let staged = pending
                .query(statements[1..statements.len() - 1].join(";") + ";")
                .bind(first.bindings.clone())
                .await
                .map_err(ModelError::codec)
                .and_then(checked_transaction);
            if let Err(error) = staged {
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.step(
                    "partial cleanup transaction cancel",
                    pending
                        .cancel()
                        .await
                        .map(|_| ())
                        .map_err(ModelError::codec),
                );
                return lctx_model::domain::completion::complete(Err(error), completion);
            }
            let reconciled = match reconcile_effect(&client, first.operation).await {
                Ok(value) => value,
                Err(error) => {
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "partial cleanup transaction cancel",
                        pending
                            .cancel()
                            .await
                            .map(|_| ())
                            .map_err(ModelError::codec),
                    );
                    return lctx_model::domain::completion::complete(Err(error), completion);
                }
            };
            let late_commit_refused = pending.commit().await.is_err();
            let fenced_step_refused = first.step(&client).await.is_err();
            let fenced: Vec<Object> = reader
                .query_native("SELECT * FROM $effect", first.bindings.clone())
                .await?;
            let remaining_after_fence = cleanup_ids(
                &client,
                "SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt",
                vars.clone(),
            )
            .await?;
            let fresh =
                CleanupHolds::begin(&client, authorization, vec![owner.clone()], true).await?;
            operations.push(fresh.operation);
            while !fresh.step(&client).await? {}
            let repeated = reconcile_effect(&client, fresh.operation).await?;
            let finished: Vec<Object> = reader
                .query_native("SELECT * FROM $effect", fresh.bindings.clone())
                .await?;
            let final_holds = cleanup_ids(
                &client,
                "SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt",
                vars.clone(),
            )
            .await?;
            vars.insert("other", other.clone());
            vars.insert("targets", targets);
            let retained: Vec<RecordId> = reader
                .query_native("SELECT VALUE id FROM $targets", vars.clone())
                .await?;
            let unrelated_holds = cleanup_ids(
                &client,
                "SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$other",
                vars,
            )
            .await?;
            let ordinary_refused = effect(&client, Some(attempt), "RETURN NONE", Variables::new())
                .await
                .is_err();
            Ok::<_, ModelError>((
                partial_done,
                partial,
                remaining,
                reconciled,
                late_commit_refused,
                fenced_step_refused,
                fenced,
                remaining_after_fence,
                repeated,
                finished,
                final_holds,
                retained,
                unrelated_holds,
                ordinary_refused,
                *epoch,
                first.bindings,
                fresh.bindings,
            ))
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        for operation in operations {
            completion.step(
                "partial cleanup final effect fence",
                reconcile_effect(&client, operation).await.map(|_| ()),
            );
        }
        completion.step(
            "partial cleanup terminal owner",
            close_attempt(&client, attempt, "abandoned").await,
        );
        completion.step(
            "partial cleanup unrelated owner",
            close_attempt(&client, unrelated, "abandoned").await,
        );
        completion.step("partial cleanup reader close", reader.close().await);
        completion.step(
            "partial cleanup session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let (
            partial_done,
            partial,
            remaining,
            reconciled,
            late_commit_refused,
            fenced_step_refused,
            fenced,
            remaining_after_fence,
            repeated,
            finished,
            final_holds,
            retained,
            unrelated_holds,
            ordinary_refused,
            epoch,
            first_bindings,
            fresh_bindings,
        ) = lctx_model::domain::completion::complete(result, completion).unwrap();
        assert!(!partial_done && !reconciled);
        assert!(late_commit_refused && fenced_step_refused && ordinary_refused);
        assert_eq!(remaining.len(), 129);
        assert_eq!(
            remaining_after_fence
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            remaining
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
        );
        assert!(repeated && final_holds.is_empty());
        assert_eq!(retained.len(), 257);
        assert_eq!(unrelated_holds.len(), 1);
        let check = |rows: Vec<Object>, bindings: &Variables, revision, committed, resolved| {
            let [row] = rows.as_slice() else {
                panic!("exact cleanup receipt");
            };
            let mut request = bindings.clone();
            request.remove("effect");
            request.remove("request");
            let expected = ContentHash::of(
                &serde_json::to_vec(&("native terminal hold cleanup/v1", &request)).unwrap(),
            )
            .hex();
            assert_eq!(row.get("id"), bindings.get("effect"));
            assert_eq!(row.get("attempt"), Some(&Value::RecordId(owner.clone())));
            assert_eq!(row.get("request"), Some(&Value::String(expected)));
            assert_eq!(
                row.get("epoch"),
                Some(&Value::Number(surrealdb::types::Number::Int(epoch)))
            );
            assert_eq!(
                row.get("revision"),
                Some(&Value::Number(surrealdb::types::Number::Int(revision)))
            );
            assert_eq!(row.get("committed"), Some(&Value::Bool(committed)));
            assert_eq!(row.get("resolved"), Some(&Value::Bool(resolved)));
        };
        check(partial, &first_bindings, 1i64, false, false);
        check(fenced, &first_bindings, 2i64, false, true);
        check(finished, &fresh_bindings, 3i64, true, true);
        assert_ne!(first_bindings.get("effect"), fresh_bindings.get("effect"));
        assert_eq!(first_bindings.get("request"), fresh_bindings.get("request"));
    }
    #[test]
    fn transaction_classification_preserves_primary_and_secondary_without_replaying_mixed_errors() {
        let aborted = || {
            surrealdb::Error::query(
                "not executed".into(),
                surrealdb::types::QueryError::NotExecuted,
            )
        };
        let conflict = || {
            surrealdb::Error::query(
                "conflict".into(),
                surrealdb::types::QueryError::TransactionConflict,
            )
        };
        let error =
            transaction_errors(vec![(0, aborted()), (1, conflict()), (2, aborted())]).unwrap_err();
        assert!(retryable_transaction(&error));
        let ModelError::Completion(outcome) = error else {
            panic!("secondary errors retained");
        };
        assert_eq!(outcome.completion.failures.len(), 2);
        for permanent in [
            surrealdb::Error::validation("bad row".into(), None),
            surrealdb::Error::internal("unclassified failure".into()),
        ] {
            let error = transaction_errors(vec![(0, aborted()), (1, conflict()), (2, permanent)])
                .unwrap_err();
            assert!(!retryable_transaction(&error));
            let ModelError::Completion(outcome) = error else {
                panic!("secondary errors retained");
            };
            assert_eq!(outcome.completion.failures.len(), 2);
            let Some(ModelError::Cause(primary)) = outcome.primary.as_deref() else {
                panic!("typed primary retained");
            };
            assert!(!retryable_conflict(
                primary.downcast_ref::<surrealdb::Error>().unwrap()
            ));
        }
        assert!(!retryable_transaction(
            &transaction_errors(vec![(0, aborted())]).unwrap_err()
        ));
    }
}
