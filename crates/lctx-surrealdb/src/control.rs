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
// Use a native primary-key range, rather than a filter that repeatedly scans the
// already-consumed prefix or sorts a different composite index on every page.
fn keyset_source(table:&str,after:&Value)->Result<String,ModelError>{
    use std::ops::Bound;
    use surrealdb::types::{RecordIdKey,RecordIdKeyRange,ToSql};
    let start=match after {
        Value::None=>Bound::<RecordIdKey>::Unbounded,
        Value::RecordId(id) if id.table.as_str()==table=>Bound::Excluded(id.key.clone()),
        _=>return Err(ModelError::Conflict("native keyset cursor table")),
    };
    Ok(RecordId::new(table,RecordIdKeyRange::from((start,Bound::Unbounded))).to_sql())
}

pub const SCHEMA_VERSION: i64 = 4;
mod authorization;
mod retirement;
mod history;
mod cleanup;
use cleanup::CleanupHolds;
pub use cleanup::{recover_cleanup, resume_cleanup};
mod migration;
pub use authorization::{IssuanceEra, NativeRequest, NativeRequestError, NativeAttemptError, NativeLifecycleError, EffectDisposition};
pub use retirement::{retire, retire_reachable, resume_retirement, recover_retirement, register_retirement_roots};
pub use history::{cut_era, compact_history, HistoryCompaction, retain_outcome, retain_native_outcome, release_outcome, release_outcome_reference, qualify_history_inventory, resume_history_compaction};
pub(crate) use migration::migrate_legacy_state;
pub fn schema() -> &'static str {
    include_str!("control/schema.surql")
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
        || row.get("schema_version") != Some(&Value::Number(surrealdb::types::Number::Int(SCHEMA_VERSION)))
    {
        return Err(ModelError::Conflict(
            "installed native service generation/schema",
        ));
    }
    Ok(())
}
pub use authorization::{begin_attempt_in, issue_attempt, effect, effect_frozen_attempt, execute_request, reconcile_effect, reconcile_request, request_disposition};
use authorization::{EffectOwner, TerminalAttempt, effect_for_owner};
async fn run_transaction(client: &Surreal<Client>, sql: &str, bindings: Variables) -> Result<(), ModelError> {
    loop {
        match client.query(sql.to_owned()).bind(bindings.clone()).await {
            Ok(response) => match checked_transaction(response) {
                Ok(_) => return Ok(()),
                Err(error) if retryable_transaction(&error) => tokio::task::yield_now().await,
                Err(error) => return Err(error),
            },
            Err(error) => return Err(crate::loader::write_failure(error)),
        }
    }
}
pub async fn check_pending_effects(client: &Surreal<Client>) -> Result<(), ModelError> {
    let mut response = client
        .query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false LIMIT 1")
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
            .query("SELECT VALUE id FROM native_effect WITH INDEX live_effects WHERE resolved=false LIMIT 128")
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
/// Apply a bounded mutable native-state write under the same object lifecycle guards
/// used by immutable ingress. The guard and payload effect commit atomically.
pub async fn guarded_effect(client:&Surreal<Client>,attempt:Option<ContentHash>,objects:Vec<RecordId>,sql:&str,mut bindings:Variables)->Result<(),ModelError>{
    let guards=objects.iter().map(|id|guard_id(&Value::RecordId(id.clone()))).collect::<Result<Vec<_>,_>>()?;
    bindings.insert("__write_guards",guards);
    let guarded=format!("FOR $__guard IN $__write_guards {{ LET $__prior=SELECT * FROM ONLY $__guard FOR UPDATE; IF $__prior.phase='retiring' {{ THROW 'native object retiring; resume its original lifecycle'; }}; IF $__epoch<=($__prior.retired_through ?? 0) {{ THROW 'native state epoch retired'; }}; UPSERT $__guard SET revision=(revision ?? 0)+1,retired=false,phase='active',incarnation=($__prior.incarnation ?? 1)+<int>($__prior.phase='retired'),retiring_item=NONE,retired_through=(retired_through ?? 0) RETURN NONE; }}; {sql}");
    effect(client,attempt,&guarded,bindings).await
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
        "{fence} FOR $i IN 0..array::len($rows) {{ LET $row=$rows[$i]; LET $guard=$guards[$i]; LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; IF $prior.phase='retiring' {{ THROW 'native object retiring; resume its original lifecycle'; }}; IF $__epoch<=($prior.retired_through ?? 0) {{ THROW 'native content epoch retired'; }}; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,phase='active',incarnation=($prior.incarnation ?? 1)+<int>($prior.phase='retired'),retiring_item=NONE,retired_through=(retired_through ?? 0) RETURN NONE; LET $existing=SELECT * FROM ONLY $row.id; IF $existing != NONE AND $existing != $row {{ THROW 'native immutable address collision'; }}; IF $existing=NONE {{ IF record::table($row.id) IN ['participant','reference','lex_occurs','vec_occurs'] {{ INSERT RELATION $row RETURN NONE; }} ELSE {{ CREATE $row.id CONTENT $row RETURN NONE; }}; }}; }}"
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
    effect_for_owner(client,owner,"FOR $guard IN $guards { LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; IF $prior.phase='retiring' { THROW 'native object retiring; resume its original lifecycle'; }; IF $__epoch<=($prior.retired_through ?? 0) { THROW 'native attachment epoch retired'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,phase='active',incarnation=($prior.incarnation ?? 1)+<int>($prior.phase='retired'),retiring_item=NONE,retired_through=(retired_through ?? 0) RETURN NONE; }; FOR $row IN $holds { IF (SELECT VALUE id FROM ONLY $row.object)=NONE { THROW 'native attachment target missing'; }; UPSERT $row.id CONTENT $row RETURN NONE; }",bindings).await
}

pub async fn pin_views(
    client: &Surreal<Client>,
    views: &[ContentHash],
) -> Result<ContentHash, ModelError> {
    let issuance = IssuanceEra::capture(client).await?;
    let pin = fresh_identity("reader-pin")?;
    let owner = RecordId::new("native_pin", pin.hex());
    let objects = views
        .iter()
        .map(|view| RecordId::new("compiler_view", view.hex()))
        .collect::<Vec<_>>();
    let mut bindings = Variables::new();
    bindings.insert("pin", owner.clone());
    bindings.insert("issuance", issuance.era);
    bindings.insert("views", objects.clone());
    bindings.insert("guard",guard_id(&Value::RecordId(owner.clone()))?);
    let result=async {
        effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF !$installation.admission_open OR $__era!=$issuance { THROW 'native reader admission closed'; }; LET $before=SELECT * FROM ONLY $guard FOR UPDATE; IF $before.phase='retiring' OR $__epoch<=($before.retired_through ?? 0) { THROW 'native pin creation fenced'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=false,retired_through=(retired_through ?? 0),phase='active',incarnation=(incarnation ?? 1),retiring_item=NONE RETURN NONE; CREATE $pin SET views=$views,released=false,epoch=$__epoch,generation=$__generation,era=$__era RETURN NONE",bindings).await?;
        hold(client,None,owner,objects).await?;
        Ok(pin)
    }.await;
    match result {
        Ok(pin)=>Ok(pin),
        Err(error)=>{
            let mut completion=lctx_model::domain::completion::Completion::default();
            completion.step("native pin acquisition cleanup",release_pin(client,pin).await);
            lctx_model::domain::completion::complete(Err(authorization::lifecycle_error("reader pin",pin,issuance,error)),completion)
        }
    }
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
    bindings.insert("pin", id.clone());
    bindings.insert("maintenance", maintenance);
    effect(client,None,"IF $maintenance AND $__installation.admission_open { THROW 'maintenance admission must remain closed'; }; LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior=NONE OR $prior.generation!=$__generation OR $prior.epoch<=0 { THROW 'native pin release identity'; }; IF !$maintenance AND $prior.era!=$__era { THROW 'native pin release era'; }; UPSERT $guard SET revision=(revision ?? 0)+1,retired=true,phase='retired',incarnation=(incarnation ?? 1),retiring_item=NONE,retired_through=math::max([retired_through ?? 0,$prior.epoch]) RETURN NONE; UPDATE $pin SET released=true RETURN NONE",bindings).await?;
    // The durable released flag fences protect() before bounded ownership drainage.
    loop {
        let mut response=client.query("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$pin LIMIT 128").bind(("pin",id.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let holds:Vec<RecordId>=response.take(0).map_err(ModelError::codec)?;
        if holds.is_empty(){break;}
        let mut bindings=Variables::new();bindings.insert("pin",id.clone());bindings.insert("holds",holds);
        effect(client,None,"LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior=NONE OR !$prior.released { THROW 'native pin release not fenced'; }; LET $owned=SELECT VALUE id FROM $holds WHERE owner=$pin; DELETE $owned RETURN NONE",bindings).await?;
    }
    Ok(())
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
    // are durable progress. The indexed closing state blocks era cuts until exact
    // cleanup is complete; interruption cannot strand scope before its obligation exists.
    effect(client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE { THROW 'native cleanup attempt missing'; }; IF $owner.state='open' { UPDATE $attempt SET state='closing',revision+=1 RETURN NONE; } ELSE IF $owner.state NOT IN ['closing','closed','abandoned','frozen','maintenance_fenced'] { THROW 'native cleanup attempt state'; };",bindings.clone()).await?;
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
            "closing" | "closed" | "abandoned" | "frozen" | "maintenance_fenced"
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
            release_cleanup_holds(
                client,
                authorization,
                vec![RecordId::new("native_attempt", attempt.hex())],
                false,
            )
            .await?;
            return finish_attempt_close(client,authorization,state).await;
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
            effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE OR $owner.epoch!=$attempt_epoch OR $owner.state NOT IN ['closing','closed','abandoned','frozen','maintenance_fenced'] OR $owner.admitted { THROW 'native cleanup product owner'; }; LET $owned=SELECT VALUE id FROM $products WHERE contribution IN $contributions; DELETE $owned RETURN NONE",deletion).await?;
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
    .await?;
    finish_attempt_close(client,authorization,state).await
}

async fn finish_attempt_close(client:&Surreal<Client>,authorization:TerminalAttempt,state:&str)->Result<(),ModelError>{
    let mut bindings=Variables::new();bindings.insert("state",state.to_owned());
    effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"IF array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$__attempt LIMIT 1)>0 { THROW 'native attempt cleanup incomplete'; }; IF $__owner.state='closing' { UPDATE $__attempt SET state=$state,revision+=1 RETURN NONE; };",bindings).await
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
    authorization: TerminalAttempt,
    owners: Vec<RecordId>,
    unadmitted: bool,
) -> Result<(), ModelError> {
    let cleanup = CleanupHolds::begin(client, authorization, owners, unadmitted).await?;
    while !cleanup.step(client).await? {}
    Ok(())
}
pub async fn acquire_backup_hold(client: &Surreal<Client>) -> Result<ContentHash, ModelError> {
    let issuance = IssuanceEra::capture(client).await?;
    let id = fresh_identity("database-backup-hold")?;
    let mut bindings = Variables::new();
    bindings.insert("hold", RecordId::new("native_backup_hold", id.hex()));
    bindings.insert("issuance",issuance.era);
    effect(client,None,"LET $installation=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF !$installation.admission_open OR $__era!=$issuance { THROW 'native backup admission closed'; }; UPDATE native_installation:current SET backup_revision=(backup_revision ?? 0)+1,admission_revision=(admission_revision ?? 0)+1 RETURN NONE; CREATE $hold SET active=true,generation=$__generation,era=$__era,epoch=$__epoch RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("backup hold",id,issuance,error))?;
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
    let mut response=client.query("SELECT VALUE id FROM native_hold WITH INDEX object_holds WHERE object=$object LIMIT 1; SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
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
    #[cfg(test)]
    pub(crate) fn test_detached(released: bool) -> Arc<Self> {
        Arc::new(Self { identity: ContentHash([0; 32]), client: Arc::new(Surreal::init()), runtime: tokio::runtime::Handle::current(), released: std::sync::atomic::AtomicBool::new(released), uncertain: std::sync::atomic::AtomicBool::new(false) })
    }
    #[cfg(test)]
    pub(crate) fn test_uncertain(&self) -> bool { self.uncertain.load(Ordering::Acquire) }
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
