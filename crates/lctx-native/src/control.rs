//! Durable native ownership. Every reference transition writes the same small object guard
//! as retirement; snapshot reads and predicate scans alone do not provide this protection.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError, completed::RetirementEligibility};
use std::sync::{Arc, atomic::Ordering};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Object, RecordId, Value, Variables},
};

#[cfg(test)]
fn test_value(value: impl surrealdb::types::SurrealValue) -> Value {
    value.into_value()
}

// Failed transactions include NotExecuted frames before the causal error. Inspect every
// statement so contention remains retryable and permanent refusals retain their actual cause.
pub(crate) fn checked_transaction(
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
    let mut sink = KeySink::new("native-durable-operation/v6");
    kind.to_string().encode(&mut sink);
    let operation = surrealdb::types::Uuid::new_v4().into_inner();
    sink.part(b"operation-uuid", operation.as_bytes());
    Ok(sink.finish())
}
// A native primary-key range avoids rescanning consumed prefixes. At the pinned
// engine, a literal range uses RecordIdScan, which eagerly materializes its entire
// suffix before the outer LIMIT. The identity cast selects DynamicScan instead,
// carrying LIMIT into the same ordered native range lookup and storage scanner.
fn keyset_source(table: &str, after: &Value) -> Result<String, ModelError> {
    keyset_range_source(table, after, std::ops::Bound::Unbounded)
}
fn keyset_range_source(
    table: &str,
    after: &Value,
    end: std::ops::Bound<surrealdb::types::RecordIdKey>,
) -> Result<String, ModelError> {
    use std::ops::Bound;
    use surrealdb::types::{RecordIdKey, RecordIdKeyRange, ToSql};
    let start = match after {
        Value::None => Bound::<RecordIdKey>::Unbounded,
        Value::RecordId(id) if id.table.as_str() == table => Bound::Excluded(id.key.clone()),
        _ => return Err(ModelError::Conflict("native keyset cursor table")),
    };
    let range = RecordId::new(table, RecordIdKeyRange::from((start, end)));
    Ok(format!("type::record({})", range.to_sql()))
}

pub const SCHEMA_VERSION: i64 = 6;
mod coordination;
pub use coordination::{ADMISSION_STRIPES, ExpectedObject};
pub(crate) use coordination::{
    initial_stripes as initial_admission_stripes, preflight as preflight_control, set_admission,
};
mod authorization;
mod product_recovery;
mod product_release;
mod retirement;
pub use product_recovery::{HeldTerminalProduct, hold_terminal_product};
pub use product_release::release_retained_product;
mod cleanup;
mod history;
mod immutable_window;
pub mod staging;
#[doc(hidden)]
pub use immutable_window::{ImmutableWindows, install_window};
mod receipts;
mod retry;
use cleanup::CleanupHolds;
pub use cleanup::{recover_cleanup, resume_cleanup};
pub use receipts::{EffectReceipt, EffectReceipts, capture_effect_receipts};
pub(crate) use retry::PremiseRetry;
pub use retry::{
    RetryCancellation, RetryCancellationContext, capture_retry_cancellation,
    current_retry_cancellation, with_retry_cancellation, with_retry_context,
};

pub use authorization::{
    EffectDisposition, IssuanceEra, NativeAttemptError, NativeLifecycleError, NativeRequest,
    NativeRequestError,
};
pub use history::{
    HistoryCompaction, compact_history, cut_era, qualify_history_inventory, release_outcome,
    release_outcome_reference, resume_history_compaction, retain_native_outcome, retain_outcome,
};
use retirement::retire_terminal_roots;
pub use retirement::{
    recover_retirement, register_retirement_roots, resume_retirement, retire, retire_reachable,
    retire_reachable_named,
};
pub fn schema() -> &'static str {
    include_str!("control/schema.surql")
}
pub async fn check_installation(
    client: &Surreal<Client>,
    generation: ContentHash,
) -> Result<(), ModelError> {
    let mut response = client
        .query("SELECT * FROM native_installation:current; SELECT * FROM native_admission_stripe LIMIT 65; SELECT * FROM native_backup_gate:current")
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
                crate::installation::base_schema_identity().hex(),
            ))
        || row.get("schema_version")
            != Some(&Value::Number(surrealdb::types::Number::Int(
                SCHEMA_VERSION,
            )))
    {
        return Err(ModelError::Conflict(
            "installed native service generation/schema",
        ));
    }
    let stripes: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    let gates: Vec<Object> = response.take(2).map_err(ModelError::codec)?;
    coordination::validate_control_records(row, &stripes, &gates)?;
    Ok(())
}
use authorization::{EffectOwner, TerminalAttempt, effect_for_owner};
pub use authorization::{
    begin_attempt_in, effect, execute_request, issue_attempt, reconcile_effect, reconcile_request,
    request_disposition,
};
mod frozen;
pub use authorization::effect_frozen_attempt;
pub(crate) use authorization::{OriginalCompletionOutcome, original_completion_effect};
pub use frozen::schema_target as frozen_schema_target;
pub use frozen::{advance_validity_epoch_native, freeze_content_native, resume_content_fence};
pub(crate) mod held;
pub(crate) mod live_view;
pub use held::completed_admission_plan;
pub use held::{ExpectedRecord, HeldRecord, hold_expected, inspect_expected};
pub(crate) use held::{hold_live_view_expected, hold_live_views_expected};
pub(crate) async fn prepare_maintenance_completion_expectations(
    client: &Surreal<Client>,
    bindings: &mut Variables,
) -> Result<(), ModelError> {
    coordination::prepare_expectations(client, bindings).await
}
pub(crate) fn maintenance_completion_expectation_guard() -> &'static str {
    coordination::EXPECTED_OBJECT_GUARD
}
pub(crate) use held::{
    hold_frozen_admission_expected, hold_frozen_completed_expected, hold_frozen_physical_expected,
    inspect_expected_read,
};
pub(crate) mod completed;
pub mod validation_premise;
async fn run_transaction(
    client: &Surreal<Client>,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    retry::transaction(client, sql, bindings, false).await
}
async fn run_recovery_transaction(
    client: &Surreal<Client>,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    retry::transaction(client, sql, bindings, true).await
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
pub async fn guarded_effect(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    objects: Vec<RecordId>,
    sql: &str,
    mut bindings: Variables,
) -> Result<(), ModelError> {
    let guarded = guarded_effect_plan(objects, sql, &mut bindings)?;
    effect(client, attempt, &guarded, bindings).await
}
/// Publish mutable attempt state and its ownership in one effect. Terminal cleanup
/// can then nominate every created object before releasing the attempt's holds.
pub async fn guarded_effect_with_holds(
    client: &Surreal<Client>,
    attempt: ContentHash,
    objects: Vec<RecordId>,
    references: Vec<(RecordId, RecordId)>,
    sql: &str,
    mut bindings: Variables,
) -> Result<(), ModelError> {
    let mut guarded = guarded_effect_plan(objects, sql, &mut bindings)?;
    if let Some((_, holds, hold_bindings)) = hold_plan(Some(attempt), references)? {
        merge_plan_bindings(&mut bindings, hold_bindings)?;
        guarded.push(';');
        guarded.push_str(&holds);
    }
    effect(client, Some(attempt), &guarded, bindings).await
}
fn guarded_effect_plan(
    objects: Vec<RecordId>,
    sql: &str,
    bindings: &mut Variables,
) -> Result<String, ModelError> {
    let guards = objects
        .iter()
        .map(|id| guard_id(&Value::RecordId(id.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    bindings.insert("__guard_records", guard_records(objects.iter())?);
    bindings.insert("__write_guards", guards);
    let incidence = if matches!(bindings.get("row"), Some(Value::Object(_))) {
        "fn::native_incidence_insert($row);"
    } else {
        ""
    };
    Ok(format!(
        "{incidence} FOR $__target IN $__guard_records {{ fn::native_touch($__target.record); }}; {sql}"
    ))
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
    let Some((sql, bindings)) = immutable_row_plan(attempt, contribution, rows)? else {
        return Ok(());
    };
    effect(client, attempt, &sql, bindings).await
}
fn immutable_row_plan(
    attempt: Option<ContentHash>,
    contribution: Option<ContentHash>,
    rows: Vec<Value>,
) -> Result<Option<(String, Variables)>, ModelError> {
    if rows.is_empty() {
        return Ok(None);
    }
    let mut guards = Vec::with_capacity(rows.len());
    let mut records = Vec::with_capacity(rows.len());
    let mut blob_guards = Vec::new();
    for row in &rows {
        crate::loader::validate_native_row(row)?;
        let Value::Object(row) = row else {
            return Err(ModelError::Schema("native immutable object"));
        };
        let id = row
            .get("id")
            .ok_or(ModelError::Schema("native immutable identity"))?;
        guards.push(guard_id(id)?);
        let Value::RecordId(id) = id else {
            return Err(ModelError::Schema("native immutable record identity"));
        };
        records.push(id.clone());
        if id.table.as_str() == "original_chunk" {
            let Some(Value::RecordId(blob)) = row.get("blob") else {
                return Err(ModelError::Schema("native original piece parent"));
            };
            if blob.table.as_str() != "original_blob" {
                return Err(ModelError::Schema("native original piece parent family"));
            }
            records.push(blob.clone());
            blob_guards.push(guard_id(&Value::RecordId(blob.clone()))?);
        }
    }
    let mut bindings = Variables::new();
    // Creation may join only the exact successor incarnation this request planned.
    // These full values remain in the frozen payload, separate from ordinary holds.
    let creations = rows
        .iter()
        .map(|row| {
            let mut creation = Object::new();
            creation.insert("kind", "immutable");
            creation.insert(
                "record",
                row.as_object()
                    .and_then(|row| row.get("id"))
                    .unwrap()
                    .clone(),
            );
            Value::Object(creation)
        })
        .collect::<Vec<_>>();
    bindings.insert("__immutable_creations", creations);
    bindings.insert("rows", rows);
    bindings.insert("guards", guards);
    bindings.insert("__write_guards", blob_guards);
    bindings.insert("__guard_records", guard_records(records.iter())?);
    bindings.insert(
        "contribution",
        contribution
            .map(|id| Value::RecordId(RecordId::new("compiler_contribution", id.hex())))
            .unwrap_or(Value::None),
    );
    let fence = if attempt.is_some() {
        "IF $__owner.admitted AND array::len($rows.filter(|$row|record::table($row.id) IN ['entity','assertion','compiler_record','compiler_alias','original','original_blob','original_chunk','native_pending_original','native_pending_original_ordinal','native_pending_original_piece']))>0 { THROW 'native canonical content already admitted'; }; IF $contribution != NONE { LET $producer=SELECT * FROM ONLY $contribution FOR UPDATE; IF $producer.completed OR $producer.attempt!=$__attempt { THROW 'native producing contribution fenced'; }; };"
    } else {
        ""
    };
    let sql = format!(
        "{fence} FOR $i IN 0..array::len($rows) {{ LET $row=$rows[$i]; IF record::table($row.id)='original_chunk' {{ fn::native_touch($row.blob); LET $blob_header=SELECT * FROM ONLY $row.blob; IF $blob_header=NONE OR type::is_bool($blob_header.completed)=false {{ THROW 'native original blob declaration missing'; }}; IF $blob_header.completed AND (SELECT * FROM ONLY $row.id)!=$row {{ THROW 'completed original blob refuses missing or changed piece'; }}; }}; fn::native_incidence_insert($row); LET $existing=SELECT * FROM ONLY $row.id; IF $existing != NONE AND $existing != $row {{ THROW 'native immutable address collision'; }}; IF $existing=NONE {{ IF record::table($row.id) IN ['participant','reference','lex_occurs','vec_occurs'] {{ INSERT RELATION $row RETURN NONE; }} ELSE {{ CREATE $row.id CONTENT $row RETURN NONE; }}; }}; }};"
    );
    Ok(Some((sql, bindings)))
}
/// Compose immutable rows and holds without replacing an earlier exact record map.
/// Shared by immutable windows and internal completed-state staging.
fn merge_plan_bindings(bindings: &mut Variables, additional: Variables) -> Result<(), ModelError> {
    for (key, value) in additional {
        if let Some(prior) = bindings.get(&key).cloned() {
            match (prior, value) {
                (Value::Array(mut left), Value::Array(right)) => {
                    left.extend(right);
                    bindings.insert(key, left);
                }
                (left, right) if left == right => {
                    bindings.insert(key, left);
                }
                _ => return Err(ModelError::Conflict("native composed plan binding")),
            }
        } else {
            bindings.insert(key, value);
        }
    }
    Ok(())
}
fn guard_records<'a>(
    records: impl Iterator<Item = &'a RecordId>,
) -> Result<Vec<Value>, ModelError> {
    records
        .map(|record| {
            let mut row = Object::new();
            row.insert("record", record.clone());
            row.insert("guard", guard_id(&Value::RecordId(record.clone()))?);
            Ok(Value::Object(row))
        })
        .collect()
}
pub(crate) fn guard_id(id: &Value) -> Result<RecordId, ModelError> {
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
    let Some((owner, sql, bindings)) = hold_plan(attempt, references)? else {
        return Ok(());
    };
    effect_for_owner(client, owner, &sql, bindings).await
}
fn hold_plan(
    attempt: Option<ContentHash>,
    references: Vec<(RecordId, RecordId)>,
) -> Result<Option<(EffectOwner, String, Variables)>, ModelError> {
    if references.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::with_capacity(references.len());
    let mut guards = Vec::new();
    let mut records = Vec::new();
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
        records.extend([object.clone(), owner.clone()]);
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
    bindings.insert("hold_guards", guards);
    bindings.insert("__guard_records", guard_records(records.iter())?);
    let owner = if let Some(attempt) = attempt {
        EffectOwner::OpenAttempt(attempt)
    } else {
        pins.into_iter()
            .next()
            .map(EffectOwner::Pin)
            .unwrap_or(EffectOwner::Installation)
    };
    Ok(Some((owner,"FOR $row IN $holds { IF record::table($row.owner)='compiler_contribution' { fn::native_contribution_incidence($row.owner,$row); }; IF record::table($row.owner)='compiler_view_realization' { fn::native_realization_incidence($row.owner,$row); }; IF (SELECT VALUE id FROM ONLY $row.owner)=NONE { THROW 'native attachment owner missing'; }; IF (SELECT VALUE id FROM ONLY $row.object)=NONE { THROW 'native attachment target missing'; }; fn::native_incidence($row); UPSERT $row.id CONTENT $row RETURN NONE; }".to_owned(),bindings)))
}

/// Parent-private staging composition preserves frozen attempt authority. It cannot
/// mint a public frozen owner or silently authorize mixed pin/attempt references.
fn hold_plan_for_owner(
    owner: EffectOwner,
    references: Vec<(RecordId, RecordId)>,
) -> Result<Option<(EffectOwner, String, Variables)>, ModelError> {
    let attempt = match &owner {
        EffectOwner::OpenAttempt(id) | EffectOwner::FrozenAttempt(id) => *id,
        _ => return Err(ModelError::Conflict("native staged hold owner family")),
    };
    Ok(hold_plan(Some(attempt), references)?.map(|(_, sql, bindings)| (owner, sql, bindings)))
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
    bindings.insert("guard", guard_id(&Value::RecordId(owner.clone()))?);
    let result=async {
        effect(client,None,"IF !$__admission.admission_open OR $__era!=$issuance { THROW 'native reader admission closed'; }; fn::native_touch($pin); LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior!=NONE { THROW 'native pin identity collision'; }; CREATE $guard SET revision=1,retired=false,phase='active',incarnation=1,tombstone_revision=0 RETURN NONE; CREATE $pin SET views=$views,released=false,epoch=$__epoch,generation=$__generation,era=$__era RETURN NONE",bindings).await?;
        hold(client,None,owner,objects).await?;
        Ok(pin)
    }.await;
    match result {
        Ok(pin) => Ok(pin),
        Err(error) => {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "native pin acquisition cleanup",
                release_pin(client, pin).await,
            );
            lctx_model::domain::completion::complete(
                Err(authorization::lifecycle_error(
                    "reader pin",
                    pin,
                    issuance,
                    error,
                )),
                completion,
            )
        }
    }
}
pub async fn release_pin(client: &Surreal<Client>, pin: ContentHash) -> Result<(), ModelError> {
    release_pin_checked(client, pin, false).await
}
pub async fn release_pin_checked(
    client: &Surreal<Client>,
    pin: ContentHash,
    maintenance: bool,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    let id = RecordId::new("native_pin", pin.hex());
    bindings.insert("guard", guard_id(&Value::RecordId(id.clone()))?);
    bindings.insert("pin", id.clone());
    bindings.insert("maintenance", maintenance);
    effect(client,None,"IF $maintenance AND $__installation.admission_open { THROW 'maintenance admission must remain closed'; }; LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior=NONE OR $prior.generation!=$__generation OR $prior.epoch<=0 { THROW 'native pin release identity'; }; IF !$maintenance AND $prior.era!=$__era { THROW 'native pin release era'; }; fn::native_touch($pin); UPSERT $guard SET revision=(revision ?? 0)+1,retired=true,phase='retired',incarnation=(incarnation ?? 1),retiring_item=NONE,tombstone_revision=(tombstone_revision ?? 0)+(IF !$prior.released { 1 } ELSE { 0 }) RETURN NONE; UPDATE $pin SET released=true RETURN NONE",bindings).await?;
    // The durable released flag fences protect() before bounded ownership drainage.
    loop {
        let mut response=client.query("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$pin LIMIT 128").bind(("pin",id.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let holds: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        if holds.is_empty() {
            break;
        }
        let mut bindings = Variables::new();
        bindings.insert("pin", id.clone());
        bindings.insert("holds", holds);
        effect(client,None,"LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior=NONE OR !$prior.released { THROW 'native pin release not fenced'; }; LET $owned=SELECT * FROM $holds WHERE owner=$pin; FOR $hold_row IN $owned { fn::native_incidence($hold_row); DELETE $hold_row.id RETURN NONE; }",bindings).await?;
    }
    Ok(())
}

pub async fn close_attempt(
    client: &Surreal<Client>,
    attempt: ContentHash,
    state: &str,
) -> Result<(), ModelError> {
    if !matches!(state, "closed" | "abandoned") {
        return Err(ModelError::Invalid("native attempt terminal state".into()));
    }
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert("state", state.to_string());
    // Fence ordinary effects first. The terminal attempt and remaining ownership rows
    // are durable progress. The indexed closing state blocks era cuts until exact
    // cleanup is complete; interruption cannot strand scope before its obligation exists.
    effect(client,None,"fn::native_touch($attempt); LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner=NONE { THROW 'native cleanup attempt missing'; }; IF $owner.state IN ['open','frozen'] { UPDATE $attempt SET state='closing',revision+=1 RETURN NONE; } ELSE IF $owner.state NOT IN ['closing','closed','abandoned','maintenance_fenced'] { THROW 'native cleanup attempt state'; };",bindings.clone()).await?;
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
            "closing" | "closed" | "abandoned" | "maintenance_fenced"
        )
    {
        return Err(ModelError::Conflict("native cleanup attempt fence"));
    }
    bindings.insert("attempt_epoch", *epoch);
    let authorization = TerminalAttempt {
        identity: attempt,
        epoch: *epoch,
    };
    nominate_pending_originals(client, authorization).await?;
    // The staging module is integrated by the coordinator with N05. Registration
    // precedes releasing any attempt hold, including the admitted terminal branch.
    staging::nominate_attempt_roots(client, authorization).await?;
    match admitted.as_slice() {
        [true] => {
            release_cleanup_holds(
                client,
                authorization,
                vec![RecordId::new("native_attempt", attempt.hex())],
                false,
            )
            .await?;
            return finish_attempt_close(client, authorization, state).await;
        }
        [false] => {}
        _ => return Err(ModelError::Conflict("native cleanup attempt inventory")),
    }
    // A terminal producer is not an instruction to end warm retention. Product
    // roots retain their exact completed outputs independently of artifact admission;
    // only explicit product release and the retirement owner can remove those roots.
    // The attempt's own outgoing roots have already been nominated above.
    release_cleanup_holds(
        client,
        authorization,
        vec![RecordId::new("native_attempt", attempt.hex())],
        true,
    )
    .await?;
    finish_attempt_close(client, authorization, state).await
}

async fn finish_attempt_close(
    client: &Surreal<Client>,
    authorization: TerminalAttempt,
    state: &str,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("state", state.to_owned());
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
    bindings.insert("issuance", issuance.era);
    effect(client,None,"IF !$__admission.admission_open OR $__era!=$issuance { THROW 'native backup admission closed'; }; LET $gate=SELECT * FROM ONLY native_backup_gate:current FOR UPDATE; IF $gate=NONE OR $gate.active_count<0 OR $gate.active_count>=9223372036854775807 OR $gate.revision>=9223372036854775807 { THROW 'native backup gate missing'; }; UPDATE native_backup_gate:current SET revision+=1,active_count+=1 RETURN NONE; fn::native_touch($hold); CREATE $hold SET active=true,generation=$__generation,era=$__era,epoch=$__epoch RETURN NONE",bindings).await.map_err(|error|authorization::lifecycle_error("backup hold",id,issuance,error))?;
    Ok(id)
}
pub async fn release_backup_hold(
    client: &Surreal<Client>,
    id: ContentHash,
) -> Result<(), ModelError> {
    release_backup_hold_checked(client, id, false).await
}
pub async fn release_backup_hold_checked(
    client: &Surreal<Client>,
    id: ContentHash,
    maintenance: bool,
) -> Result<(), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("hold", RecordId::new("native_backup_hold", id.hex()));
    bindings.insert("maintenance", maintenance);
    effect(client,None,"IF $maintenance { IF $__installation.admission_open { THROW 'maintenance admission must remain closed'; }; IF (SELECT VALUE id FROM ONLY $hold)=NONE { THROW 'maintenance backup identity missing'; }; }; LET $before=SELECT * FROM ONLY $hold FOR UPDATE; IF $before=NONE OR $before.generation!=$__generation { THROW 'native backup release identity'; }; fn::native_touch($hold); LET $gate=SELECT * FROM ONLY native_backup_gate:current FOR UPDATE; IF $gate=NONE OR $gate.active_count<0 OR $gate.revision>=9223372036854775807 OR ($before.active AND $gate.active_count=0) { THROW 'native backup gate corrupt'; }; UPDATE native_backup_gate:current SET revision+=1,active_count-=IF $before.active { 1 } ELSE { 0 } RETURN NONE; UPDATE $hold SET active=false RETURN NONE",bindings).await
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
    pub(crate) fn ensure_live(&self) -> Result<(), ModelError> {
        if self.released.load(Ordering::Acquire) || self.uncertain.load(Ordering::Acquire) {
            return Err(ModelError::Conflict(
                "native reader pin is released or uncertain",
            ));
        }
        Ok(())
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
        Arc::new(Self {
            identity: ContentHash([0; 32]),
            client: Arc::new(Surreal::init()),
            runtime: tokio::runtime::Handle::current(),
            released: std::sync::atomic::AtomicBool::new(released),
            uncertain: std::sync::atomic::AtomicBool::new(false),
        })
    }
    #[cfg(test)]
    pub(crate) fn test_uncertain(&self) -> bool {
        self.uncertain.load(Ordering::Acquire)
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
    fn immutable_witness_fragments_compose_inside_actual_payload_closure() {
        let attempt = ContentHash([17; 32]);
        let blob = crate::originals::Blob::new(ContentHash::of(b"abc"), 3).unwrap();
        let (rows, _) = immutable_row_plan(None, None, vec![blob.value(false)])
            .unwrap()
            .unwrap();
        let (_, holds, _) = hold_plan(
            Some(attempt),
            vec![(RecordId::new("native_attempt", attempt.hex()), blob.id())],
        )
        .unwrap()
        .unwrap();
        // Completed/live/recovery witnesses concatenate these actual fragments. A
        // missing statement delimiter previously reached the service as `}FOR`.
        surrealdb_syn::parse(&format!(
            "BEGIN; LET $__payload=|| {{ {rows}{holds}; }}; RETURN $__payload(); COMMIT;"
        ))
        .unwrap();
    }
    #[test]
    fn keyset_identity_cast_preserves_native_key_types_and_excluded_bounds() {
        use surrealdb::types::{RecordIdKey, ToSql};
        let mut composite = Object::new();
        composite.insert("part", "quoted:' key");
        let keys = vec![
            RecordIdKey::Number(7),
            RecordIdKey::String("7".into()),
            RecordIdKey::Uuid(surrealdb::types::Uuid::new_v4()),
            RecordIdKey::String("quoted:' key".into()),
            RecordIdKey::Array(
                vec![
                    Value::String("scope".into()),
                    Value::Number(surrealdb::types::Number::Int(7)),
                ]
                .into(),
            ),
            RecordIdKey::Object(composite),
        ];
        assert_eq!(
            keyset_source("native_guard", &Value::None).unwrap(),
            "type::record(native_guard:..)"
        );
        for key in keys {
            let source = keyset_source(
                "native_guard",
                &Value::RecordId(RecordId::new("native_guard", key.clone())),
            )
            .unwrap();
            assert_eq!(
                source,
                format!("type::record(native_guard:{}>..)", key.to_sql())
            );
            surrealdb_syn::parse(&format!("SELECT * FROM {source} LIMIT 128;")).unwrap();
            let bounded = keyset_range_source(
                "native_guard",
                &Value::RecordId(RecordId::new("native_guard", key.clone())),
                std::ops::Bound::Included(key.clone()),
            )
            .unwrap();
            assert_eq!(
                bounded,
                format!(
                    "type::record(native_guard:{}>..={})",
                    key.to_sql(),
                    key.to_sql()
                )
            );
            surrealdb_syn::parse(&format!("SELECT * FROM {bounded} LIMIT 128;")).unwrap();
        }
        surrealdb_syn::parse(&format!(
            "SELECT * FROM {} LIMIT 128;",
            keyset_source("native_guard", &Value::None).unwrap()
        ))
        .unwrap();
    }
    #[test]
    fn keyset_cursor_rejects_foreign_tables_and_non_native_values() {
        for after in [
            Value::RecordId(RecordId::new("native_hold", "cursor")),
            Value::String("native_guard:cursor".into()),
            Value::Null,
            Value::Number(surrealdb::types::Number::Int(1)),
        ] {
            assert!(keyset_source("native_guard", &after).is_err());
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires installed stable validation fixture; creates and removes only nonce-owned guards"]
    async fn native_keyset_dynamic_scan_pushes_limit_and_preserves_full_ordered_pages() {
        use surrealdb::types::RecordIdKey;
        let path =
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        assert_eq!(
            config.database.as_str(),
            "validation",
            "qualification cannot touch operator scope"
        );
        let prefix = format!(
            "sa_keyset_{}_",
            fresh_identity("native-keyset-limit-control").unwrap().hex()
        );
        let client = crate::installation::check_installation(&config)
            .await
            .unwrap();
        let ids = (0..260)
            .map(|index| RecordId::new("native_guard", format!("{prefix}{index:04}")))
            .collect::<Vec<_>>();
        let expected = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let mut row = Object::new();
                row.insert("id", id.clone());
                row.insert("revision", index as i64);
                row.insert("retired", false);
                row.insert("phase", "active");
                row.insert("incarnation", 1i64);
                row.insert("tombstone_revision", 0i64);
                row
            })
            .collect::<Vec<_>>();
        let result = async {
            for rows in expected.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let rows = rows.iter().cloned().map(Value::Object).collect::<Vec<_>>();
                checked_transaction(
                    client
                        .query("BEGIN; INSERT INTO native_guard $rows RETURN NONE; COMMIT;")
                        .bind(("rows", rows))
                        .await
                        .map_err(ModelError::codec)?,
                )?;
            }
            let end = std::ops::Bound::Included(ids.last().unwrap().key.clone());
            let mut after =
                Value::RecordId(RecordId::new("native_guard", RecordIdKey::String(prefix)));
            let mut offset = 0;
            for count in [128, 128, 4, 0] {
                let source = keyset_range_source("native_guard", &after, end.clone())?;
                let sql = format!(
                    "SELECT * FROM {source} LIMIT {}",
                    crate::loader::NATIVE_WINDOW_ROWS
                );
                let mut response = client
                    .query(format!("EXPLAIN {sql}; {sql};"))
                    .await
                    .map_err(ModelError::codec)?
                    .check()
                    .map_err(ModelError::codec)?;
                let plan: Value = response.take(0).map_err(ModelError::codec)?;
                let Value::String(plan) = plan else {
                    return Err(ModelError::Schema("native keyset explanation shape"));
                };
                if !plan.contains("DynamicScan")
                    || !plan.contains("limit: 128")
                    || plan.contains("RecordIdScan")
                    || plan.contains("Sort")
                {
                    return Err(ModelError::Invalid(format!(
                        "native keyset must push the page limit into DynamicScan: {plan}"
                    )));
                }
                let actual: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
                if actual != expected[offset..offset + count] {
                    return Err(ModelError::Conflict("native keyset full ordered page"));
                }
                offset += count;
                if let Some(last) = actual.last() {
                    after = last
                        .get("id")
                        .cloned()
                        .ok_or(ModelError::Schema("native keyset row identity"))?;
                }
            }
            Ok::<_, ModelError>(())
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        for rows in expected.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            completion.step("native keyset nonce-owned guard cleanup",async {
                let ids=rows.iter().map(|row|row.get("id").cloned().unwrap()).collect::<Vec<_>>();
                let rows=rows.iter().cloned().map(Value::Object).collect::<Vec<_>>();
                // One bounded lookup checks full bodies; missing rows are safe after
                // partial insertion, while changed rows retain cleanup uncertainty.
                checked_transaction(client.query("BEGIN; LET $actual=(SELECT * FROM $ids); FOR $row IN $actual { IF $row NOT IN $rows { THROW 'native keyset owned guard changed before cleanup'; }; }; DELETE $ids RETURN NONE; COMMIT;").bind(("ids",ids)).bind(("rows",rows)).await.map_err(ModelError::codec)?)?;
                Ok::<_,ModelError>(())
            }.await);
        }
        completion.step(
            "native keyset qualification session invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        lctx_model::domain::completion::complete(result, completion).unwrap();
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

/// Shared by private original staging and publication staging. Each caller validates
/// its own fixed envelope, then proves exact bytes/incarnation and either its still-live
/// attempt hold or an already durable retirement obligation before any hold is released.
async fn nominate_terminal_roots(
    client: &Surreal<Client>,
    authorization: TerminalAttempt,
    values: Vec<Value>,
) -> Result<(), ModelError> {
    let attempt = RecordId::new("native_attempt", authorization.identity.hex());
    let mut roots = Vec::new();
    let mut proofs = Vec::new();
    let mut windows = crate::loader::NativeWindows::new(values);
    let values = windows.next().transpose()?.unwrap_or_default();
    if windows.next().is_some() {
        return Err(ModelError::Conflict(
            "terminal staging page exceeds exact native window",
        ));
    }
    for value in values {
        let Value::Object(row) = &value else {
            return Err(ModelError::Schema("terminal staging envelope"));
        };
        let Some(Value::RecordId(root)) = row.get("id") else {
            return Err(ModelError::Schema("terminal staging identity"));
        };
        if !matches!(
            root.table.as_str(),
            "native_pending_original" | "native_publication_staging"
        ) || row.get("attempt") != Some(&Value::RecordId(attempt.clone()))
        {
            return Err(ModelError::Conflict("terminal staging original owner"));
        }
        let guard = guard_id(&Value::RecordId(root.clone()))?;
        let mut observed=client.query("SELECT * FROM $guard; SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND object=$root LIMIT 1")
            .bind(("guard",guard.clone())).bind(("attempt",attempt.clone())).bind(("root",root.clone())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let physical: Vec<Object> = observed.take(0).map_err(ModelError::codec)?;
        let held: Vec<RecordId> = observed.take(1).map_err(ModelError::codec)?;
        let [physical] = physical.as_slice() else {
            return Err(ModelError::Conflict(
                "terminal staging physical guard missing",
            ));
        };
        let incarnation = authorization::int_field(physical, "incarnation")?;
        let Some(Value::String(phase)) = physical.get("phase") else {
            return Err(ModelError::Schema("terminal staging physical phase"));
        };
        if incarnation <= 0
            || !matches!(phase.as_str(), "active" | "retiring")
            || (!held.is_empty() && phase != "active")
        {
            return Err(ModelError::Conflict("terminal staging incarnation/hold"));
        }
        let mut proof = Object::new();
        proof.insert("id", root.clone());
        proof.insert("row", value.clone());
        proof.insert("guard", guard);
        proof.insert("incarnation", incarnation);
        proof.insert("phase", phase.clone());
        proof.insert("held", !held.is_empty());
        proofs.push(Value::Object(proof));
        if !held.is_empty() {
            roots.push(root.clone());
        }
    }
    let mut bindings = Variables::new();
    bindings.insert("proofs", proofs);
    effect_for_owner(client,EffectOwner::TerminalAttempt(authorization),"FOR $proof IN $proofs { LET $id=$proof.id; LET $guard=$proof.guard; fn::native_touch($id); LET $physical=SELECT * FROM ONLY $guard FOR UPDATE; LET $row=SELECT * FROM ONLY $id FOR UPDATE; IF $row!=$proof.row OR $row.attempt!=$__attempt OR ($row.generation!=NONE AND $row.generation!=$__generation) OR ($row.epoch!=NONE AND $row.epoch!=$__expected_epoch) OR $physical=NONE OR $physical.incarnation!=$proof.incarnation OR $physical.phase!=$proof.phase OR $physical.revision>=9223372036854775807 { THROW 'terminal staging cleanup identity changed'; }; UPDATE $guard SET revision+=1 RETURN NONE; fn::native_incidence($row); IF $proof.held { IF $physical.phase!='active' OR array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$__attempt AND object=$id LIMIT 1)!=1 { THROW 'terminal staging cleanup attempt hold changed'; }; } ELSE { IF array::len(SELECT VALUE id FROM native_retirement_item WITH INDEX object_retirements WHERE object=$id AND incarnation=$proof.incarnation AND state IN ['pending','retained','retiring','done'] AND job.generation=$__generation AND job.state IN ['open','recovering','done'] LIMIT 1)<1 { THROW 'terminal staging cleanup nomination missing'; }; }; };",bindings).await?;
    if !roots.is_empty() {
        retire_terminal_roots(client, authorization, roots, 1).await?;
    }
    Ok(())
}

/// No original bytes are read here. Only bounded fixed staging envelopes are discovered.
async fn nominate_pending_originals(
    client: &Surreal<Client>,
    authorization: TerminalAttempt,
) -> Result<(), ModelError> {
    let attempt = RecordId::new("native_attempt", authorization.identity.hex());
    let mut bindings = Variables::new();
    bindings.insert("attempt", attempt);
    bindings.insert("after", Value::None);
    loop {
        let mut response=client.query("SELECT * FROM native_pending_original WITH INDEX pending_attempt WHERE attempt=$attempt AND ($after=NONE OR id>$after) ORDER BY id LIMIT 32").bind(bindings.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let values: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        if values.is_empty() {
            return Ok(());
        }
        let mut after = None;
        for value in &values {
            after = Some(crate::originals::pending_root_identity(
                authorization.identity,
                value,
            )?);
        }
        nominate_terminal_roots(client, authorization, values).await?;
        bindings.insert(
            "after",
            after.ok_or(ModelError::Schema("pending original terminal page"))?,
        );
    }
}
