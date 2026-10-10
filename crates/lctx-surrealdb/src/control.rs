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
fn checked_transaction(mut response: surrealdb::IndexedResults) -> Result<(), ModelError> {
    transaction_errors(response.take_errors().into_iter().collect())
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
    effect_for_owner(client, attempt, None, sql, bindings).await
}
/// Intent permanently records the authorization epoch. Retry never borrows a newer
/// installation/attempt/pin epoch to make an old operation eligible after retirement.
async fn effect_for_owner(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    pin: Option<RecordId>,
    sql: &str,
    mut bindings: Variables,
) -> Result<(), ModelError> {
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
    let authorization = if attempt.is_some() {
        "LET $__authorized=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__authorized.state!='open' { THROW 'native attempt fenced'; }; LET $__epoch=$__authorized.epoch;"
    } else if pin.is_some() {
        "LET $__authorized=SELECT * FROM ONLY $__pin FOR UPDATE; IF $__authorized=NONE OR $__authorized.released { THROW 'native reader pin fenced'; }; LET $__epoch=$__authorized.epoch;"
    } else {
        "LET $__installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $__installation=NONE { THROW 'native installation missing'; }; UPDATE native_installation:current SET admission_revision=(admission_revision ?? 0)+1 RETURN NONE; LET $__epoch=($__installation.admission_revision ?? 0)+1;"
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
    let fence = if attempt.is_some() {
        "LET $__owner=SELECT * FROM ONLY $__attempt FOR UPDATE; IF $__owner.state!='open' OR $__owner.epoch!=$__epoch { THROW 'native attempt fenced'; };"
    } else if pin.is_some() {
        "LET $__pin_owner=SELECT * FROM ONLY $__pin FOR UPDATE; IF $__pin_owner=NONE OR $__pin_owner.released OR $__pin_owner.epoch!=$__epoch { THROW 'native reader pin fenced'; };"
    } else {
        ""
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
    effect_for_owner(client,attempt,pins.into_iter().next(),"FOR $guard IN $guards { LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; IF $__epoch<=($prior.retired_through ?? 0) { THROW 'native attachment epoch retired'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,retired_through=(retired_through ?? 0) RETURN NONE; }; FOR $row IN $holds { IF (SELECT VALUE id FROM ONLY $row.object)=NONE { THROW 'native attachment target missing'; }; UPSERT $row.id CONTENT $row RETURN NONE; }",bindings).await
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
    match admitted.as_slice() {
        [true] => {
            return release_cleanup_holds(
                client,
                &bindings,
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
            release_cleanup_holds(client, &bindings, products.clone(), true).await?;
            let mut deletion = product_bindings.clone();
            deletion.insert("products", products);
            effect(client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] OR $owner.admitted { THROW 'native cleanup product owner'; }; LET $owned=SELECT VALUE id FROM $products WHERE contribution IN $contributions; DELETE $owned RETURN NONE",deletion).await?;
        }
    }
    // These roots keep immutable contributor IDs discoverable across interrupted
    // product cleanup, even when concurrent retirement is making progress.
    release_cleanup_holds(
        client,
        &bindings,
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

async fn release_cleanup_holds(
    client: &Surreal<Client>,
    attempt: &Variables,
    owners: Vec<RecordId>,
    unadmitted: bool,
) -> Result<(), ModelError> {
    let mut bindings = attempt.clone();
    bindings.insert("owners", owners);
    bindings.insert("unadmitted", unadmitted);
    loop {
        let holds = cleanup_ids(client,"SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner IN $owners LIMIT 128",bindings.clone()).await?;
        if holds.is_empty() {
            return Ok(());
        }
        bindings.insert("holds", holds);
        effect(client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closed','abandoned','frozen','maintenance_fenced'] OR ($unadmitted AND $owner.admitted) { THROW 'native cleanup hold owner'; }; LET $owned=SELECT VALUE id FROM $holds WHERE owner IN $owners; DELETE $owned RETURN NONE",bindings.clone()).await?;
    }
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
    let mut bindings = Variables::new();
    bindings.insert("job", job.clone());
    effect(client,None,"UPDATE native_retirement_item SET state='pending' WHERE job=$job AND state='retained' RETURN NONE",bindings).await?;
    for _ in 0..limit {
        let mut response=client.query("SELECT id,object FROM native_retirement_item WHERE job=$job AND state='pending' ORDER BY id LIMIT 1").bind(("job",job.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let queue: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let Some(item) = queue.first() else {
            break;
        };
        let (Some(Value::RecordId(item)), Some(Value::RecordId(object))) =
            (item.get("id"), item.get("object"))
        else {
            return Err(ModelError::Schema("native retirement item"));
        };
        let guard = guard_id(&Value::RecordId(object.clone()))?;
        let mut response = client
            .query("SELECT VALUE revision FROM $guard")
            .bind(("guard", guard.clone()))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let revision: Vec<i64> = response.take(0).map_err(ModelError::codec)?;
        let revision = revision.first().copied().unwrap_or(0);
        // Persist every child before releasing parent references. The owner guard revision
        // proves the bounded child scan was complete, even if references changed meanwhile.
        let mut children=crate::reader::NativeRows::new(client.query("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$owner ORDER BY object").bind(("owner",object.clone())).stream_items().map_err(ModelError::codec)?,1)?;
        let mut window = Vec::new();
        while let Some(child) = children.next().await? {
            let Value::RecordId(child) = child else {
                return Err(ModelError::Schema("native retirement child"));
            };
            window.push(child);
            if window.len() == crate::loader::NATIVE_WINDOW_ROWS {
                enqueue_retirement(client, &job, std::mem::take(&mut window)).await?;
            }
        }
        enqueue_retirement(client, &job, window).await?;
        let mut bindings = Variables::new();
        bindings.insert("job", job.clone());
        bindings.insert("item", item.clone());
        bindings.insert("object", object.clone());
        bindings.insert("guard", guard);
        bindings.insert("revision", revision);
        effect(client,None,"LET $queued=SELECT * FROM ONLY $item FOR UPDATE; IF $queued.state='pending' { LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; UPDATE native_installation:current SET backup_revision=(backup_revision ?? 0)+1,admission_revision=(admission_revision ?? 0)+1 RETURN NONE; IF array::len(SELECT VALUE id FROM native_backup_hold WHERE active=true LIMIT 1)>0 { THROW 'native retirement backup hold'; }; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF ($before.revision ?? 0)=$revision { UPSERT $guard SET revision=(revision ?? 0)+1,retired=(retired ?? false) RETURN NONE; IF array::len(SELECT VALUE id FROM native_hold WHERE object=$object LIMIT 1)>0 { UPDATE $item SET state='retained' RETURN NONE; UPDATE $job SET examined+=1,revision+=1 RETURN NONE; } ELSE { UPDATE $guard SET retired=true,retired_through=math::max([retired_through ?? 0,$installation.admission_revision ?? 0]) RETURN NONE; DELETE $object RETURN NONE; LET $owner_holds=SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$object; DELETE $owner_holds RETURN NONE; UPDATE $item SET state='done' RETURN NONE; UPDATE $job SET examined+=1,retired+=1,revision+=1 RETURN NONE; }; }; };",bindings).await?;
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
