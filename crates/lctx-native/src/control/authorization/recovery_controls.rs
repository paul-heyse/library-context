//! Native terminal-reference controls, including ordinary logical ownership scopes.
//! Ignored acknowledgement controls below require a task-owned RocksDB target;
//! they model unobserved acknowledgements rather than inject network faults.
use super::*;
use futures::FutureExt;
use lctx_model::domain::{
    completion::{Completion, RemoteState, complete},
    resources::{DEFAULT_MEMORY_BYTES, ResourceBudget},
};
use std::panic::AssertUnwindSafe;

async fn inspect(
    reader: &crate::NativeReader<()>,
    request: NativeRequest,
    allocation: NativeRequest,
    attempt: ContentHash,
) -> Result<Object, ModelError> {
    let reference = request_outcome_reference(request, RecoveryReferencePolicy::ReleaseRecovery)?;
    let retained = vec![
        reference.clone(),
        allocation_outcome_reference(allocation)?,
        source_attempt_reference(attempt),
    ];
    let mut bindings = Variables::new();
    bindings.insert(
        "effect",
        RecordId::new("native_effect", request.operation.hex()),
    );
    bindings.insert("reference", reference);
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert("retained", retained);
    reader.query_native("RETURN {effect:(SELECT * FROM ONLY $effect),reference:(SELECT VALUE id FROM ONLY $reference),attempt:(SELECT * FROM ONLY $attempt),retained:(SELECT VALUE id FROM $retained),holds:(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt LIMIT 1)}",bindings).await
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "task-owned RocksDB native recovery campaign only; models an unobserved acknowledgement"]
async fn native_committed_packets_with_unobserved_ack_require_explicit_recovery_policy() {
    committed_packet_recovery(false).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "task-owned RocksDB native recovery campaign only; retains a known terminal reference"]
async fn native_known_commit_stopped_release_retains_exact_reference_and_recipient() {
    committed_packet_recovery(true).await;
}

async fn committed_packet_recovery(persist_terminal: bool) {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("task-owned RocksDB recovery target"),
    ))
    .unwrap();
    let budget = ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap();
    let factory =
        RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget.clone()).unwrap();
    let (client, lease) = crate::reader::connect_owned(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
        &factory,
    )
    .await
    .unwrap();
    let scope = lease.recovery_scope().clone();
    let mut lease = Some(lease);
    // An independent, concretely owned session can inspect after source invalidation.
    let observer_factory =
        RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget.clone()).unwrap();
    let (_, observer_lease) = crate::reader::connect_owned(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
        &observer_factory,
    )
    .await
    .unwrap();
    let observer_scope = observer_lease.recovery_scope().clone();
    let observer = observer_scope
        .finalization_reader(observer_scope.cancellation())
        .await
        .unwrap();
    // Explicit resumption has its own process-signalled admission; the original
    // token remains stopped throughout and is never reset or replaced.
    let recovery = RetryCancellation::default();
    let stop = recovery.clone();
    let recovery_signal = crate::signal::SignalRegistration::with_receipt(
        "owned native recovery control successor",
        move || stop.cancel(),
        config.receipt_binding().unwrap(),
    )
    .unwrap();
    let result = AssertUnwindSafe(async {
        let (request, allocation_request, attempt, pending) =
            with_recovery_scope(scope.clone(), async {
                let era = IssuanceEra::capture(&client).await?;
                let allocation =
                    prepare_attempt_allocation(fresh_identity("ack-recovery-source")?, era)?;
                let (attempt, allocation_request) = (allocation.attempt, allocation.request);
                lease
                    .as_ref()
                    .unwrap()
                    .register_attempt(attempt, allocation_request)?;
                allocation.issue(&client).await?;
                lease.as_ref().unwrap().source_ready()?;
                let payload = PreparedNativePayload::new("RETURN true", Variables::new())?;
                let request = NativeRequest::issue(&client, payload.digest).await?;
                let plan = owner_effect_plan(
                    request,
                    &EffectOwner::OpenAttempt(attempt),
                    payload.sql,
                    payload.bindings,
                    false,
                )?;
                let pending = scope.prepare(
                    request,
                    plan.recovery_owner.clone(),
                    plan.reference.clone(),
                    RecoveryReferencePolicy::ReleaseRecovery,
                )?;
                // The candidate atomically commits payload/outcome. Its response
                // is deliberately not observed by PendingRecovery.
                super::super::checked_transaction(
                    client
                        .query(&plan.query)
                        .bind(plan.bindings)
                        .await
                        .map_err(ModelError::codec)?,
                )?;
                // The production SQL/bindings leave scope here; recovery receives only
                // the compact original identity, owner and exact reference policy.
                Ok::<_, ModelError>((request, allocation_request, attempt, pending))
            })
            .await?;
        let before = inspect(&observer, request, allocation_request, attempt).await?;
        let effect = before
            .get("effect")
            .and_then(Value::as_object)
            .expect("actual committed outcome");
        assert_eq!(effect.get("committed"), Some(&Value::Bool(true)));
        assert_eq!(effect.get("resolved"), Some(&Value::Bool(true)));
        assert_eq!(hash_field(effect, "request")?, request.request);
        assert_eq!(int_field(effect, "revision")?, 1);
        assert_eq!(
            before.get("reference"),
            Some(&Value::RecordId(request_outcome_reference(
                request,
                RecoveryReferencePolicy::ReleaseRecovery
            )?))
        );
        assert_eq!(
            pending.inspect(|envelope| envelope.observed_disposition)?,
            EffectDisposition::Pending
        );
        if persist_terminal {
            pending.committed()?;
            assert_eq!(
                pending.inspect(|envelope| envelope.action)?,
                RecoveryAction::ReleaseTerminalReference
            );
        }
        let retained = pending.inspect(Clone::clone)?;
        scope.cancellation().cancel();
        assert!(resume_recovery_scope(&scope).await.is_err());
        assert_eq!(
            scope.pending_count()?,
            1,
            "stopped production policy must not clear the pending owner"
        );
        assert_eq!(
            pending.inspect(|envelope| envelope.observed_disposition)?,
            if persist_terminal {
                EffectDisposition::Committed
            } else {
                EffectDisposition::Pending
            }
        );
        assert_eq!(
            pending.inspect(Clone::clone)?,
            retained,
            "failed release preserves exact original recipient and reference policy"
        );
        let stopped = inspect(&observer, request, allocation_request, attempt).await?;
        assert_eq!(
            stopped.get("reference"),
            before.get("reference"),
            "native reference remains until admitted exact settlement"
        );
        resume_recovery_scope_admitted(&scope, &recovery).await?;
        assert!(
            scope.cancellation().stopped(),
            "resumption never resets the original token"
        );
        assert!(!recovery.stopped());
        assert_eq!(scope.pending_count()?, 0);
        assert_eq!(
            pending.inspect(|envelope| envelope.observed_disposition)?,
            EffectDisposition::Committed
        );
        let after = inspect(&observer, request, allocation_request, attempt).await?;
        let effect = after
            .get("effect")
            .and_then(Value::as_object)
            .expect("same original native outcome");
        assert_eq!(hash_field(effect, "request")?, request.request);
        assert_eq!(
            int_field(effect, "revision")?,
            1,
            "recovery settles metadata without executing the original payload"
        );
        assert_eq!(effect.get("committed"), Some(&Value::Bool(true)));
        assert_eq!(after.get("reference"), Some(&Value::None));
        drop(pending);
        scope.join_finalization_reads().await?;
        drop(lease.take());
        factory.drain(&recovery).await?;
        assert_eq!(factory.pending_count()?, 0);
        let closed = inspect(&observer, request, allocation_request, attempt).await?;
        let attempt = closed
            .get("attempt")
            .and_then(Value::as_object)
            .expect("terminal original attempt");
        assert_eq!(
            attempt.get("state"),
            Some(&Value::String("abandoned".into()))
        );
        assert!(
            closed
                .get("retained")
                .and_then(Value::as_array)
                .expect("exact reference inventory")
                .is_empty(),
            "request, allocation-caller and attempt-lifecycle references must all be released"
        );
        assert!(
            closed
                .get("holds")
                .and_then(Value::as_array)
                .expect("exact source ownership inventory")
                .is_empty(),
            "terminal source must not retain a native ownership hold"
        );
        assert_eq!(scope.pending_count()?, 0);
        Ok::<(), ModelError>(())
    })
    .catch_unwind()
    .await;
    // Independent cleanup still runs on a revealing assertion/statement failure.
    drop(lease.take());
    let mut completion = Completion::default();
    completion.step(
        "owned recovery source finalization",
        factory.drain(&recovery).await,
    );
    let closed = observer.close().await;
    let can_close = closed.is_ok();
    completion.step("owned recovery observer query drainage", closed);
    if can_close {
        completion.step(
            "owned recovery observer session finality",
            observer_lease.complete().await,
        );
    }
    if !completion.failures.is_empty() {
        completion.remote = RemoteState::Unknown;
    }
    completion.step(
        "owned recovery successor process receipt",
        recovery_signal.complete(&completion).await,
    );
    match result {
        Ok(result) => complete(result, completion).unwrap(),
        Err(panic) => {
            if !completion.failures.is_empty() {
                eprintln!("native recovery control cleanup: {completion:?}");
            }
            std::panic::resume_unwind(panic);
        }
    }
}

/// Observe the delta inside the actual production transaction, so unrelated
/// processes changing the same installation stripe cannot mask or fail the case.
async fn settle_with_stripe_delta(
    client: &Surreal<Client>,
    envelope: &PendingRecoveryEnvelope,
    expected_delta: i64,
) -> Result<(), ModelError> {
    envelope.validate(&envelope.target)?;
    let current = IssuanceEra::capture(client).await?;
    let (sql, mut bindings) = terminal_reference_settlement_plan(envelope, current)?;
    let body = sql.strip_prefix("BEGIN; ").and_then(|sql| sql.strip_suffix(" COMMIT;"))
        .ok_or(ModelError::Schema("native settlement control transaction"))?;
    bindings.insert("control_expected_delta", expected_delta);
    let observed = format!(
        "BEGIN; LET $control_before=SELECT VALUE revision FROM ONLY $__stripe; {body} IF (SELECT VALUE revision FROM ONLY $__stripe) - $control_before != $control_expected_delta {{ THROW 'native terminal settlement stripe delta'; }}; COMMIT;"
    );
    coordination::preflight(&observed, &bindings)?;
    super::super::retry::transaction_named(client, &observed, bindings, true, "terminal_settlement_stripe_control").await
}

#[tokio::test(flavor = "multi_thread")]
async fn native_terminal_release_preserves_stripe_checks_without_an_ordinary_revision_write() {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("installed logical validation target"),
    )).unwrap();
    let budget = ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap();
    let factory = RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget).unwrap();
    let (client, lease) = crate::reader::connect_owned(
        &config.endpoint, &config.writer_credentials(), config.namespace.as_str(),
        config.database.as_str(), &factory,
    ).await.unwrap();
    let scope = lease.recovery_scope().clone();
    // The successor is an actually enrolled parent recovery operation, not a
    // replacement/reset of the production token which will remain stopped.
    let recovery = RetryCancellation::default();
    let stopped = recovery.clone();
    let recovery_signal = crate::signal::SignalRegistration::with_receipt(
        "ordinary terminal settlement parent recovery", move || stopped.cancel(),
        config.receipt_binding().unwrap(),
    ).unwrap();
    let mut retained = Vec::new();
    let result = AssertUnwindSafe(with_recovery_scope(scope.clone(), async {
        for (policy, expected_delta) in [
            (RecoveryReferencePolicy::ReleaseRecovery, 0),
            (RecoveryReferencePolicy::RetainCaller, 1),
        ] {
            let payload = PreparedNativePayload::new("RETURN true", Variables::new())?;
            let request = NativeRequest::issue(&client, payload.digest).await?;
            let plan = owner_effect_plan(request, &EffectOwner::Installation, payload.sql,
                payload.bindings, policy == RecoveryReferencePolicy::RetainCaller)?;
            let pending = scope.prepare(request, plan.recovery_owner.clone(), plan.reference.clone(), policy)?;
            if policy == RecoveryReferencePolicy::RetainCaller { retained.push(request); }
            super::super::retry::transaction_named(&client, &plan.query, plan.bindings, false, "terminal_settlement_control_apply").await?;
            pending.committed()?;
            let envelope = pending.inspect(Clone::clone)?;
            if policy == RecoveryReferencePolicy::ReleaseRecovery {
                let mut wrong_owner = envelope.clone();
                wrong_owner.owner = RecoveryOwner::OpenAttempt(fresh_identity("foreign-settlement-owner")?);
                assert!(settle_reference(&client, &wrong_owner, scope.cancellation()).await.is_err());
                let mut wrong_reference = envelope.clone();
                wrong_reference.outcome_reference = RecordId::new("native_outcome_ref", fresh_identity("foreign-settlement-reference")?.hex());
                assert!(settle_reference(&client, &wrong_reference, scope.cancellation()).await.is_err());
                let mut foreign_generation = envelope.clone();
                let foreign = fresh_identity("foreign-settlement-generation")?;
                foreign_generation.request.issuance.generation = foreign;
                foreign_generation.target.generation = foreign;
                assert!(settle_reference(&client, &foreign_generation, scope.cancellation()).await.is_err());
                let mut response = client.query("SELECT VALUE id FROM ONLY $reference")
                    .bind(("reference", envelope.outcome_reference.clone())).await.map_err(ModelError::codec)?
                    .check().map_err(ModelError::codec)?;
                let reference: Value = response.take(0).map_err(ModelError::codec)?;
                assert_eq!(reference, Value::RecordId(envelope.outcome_reference.clone()),
                    "refusals must preserve the exact surviving reference");
                assert_eq!(pending.inspect(Clone::clone)?, envelope,
                    "refusals must preserve durable terminal knowledge and the actual recipient");
            }
            settle_with_stripe_delta(&client, &envelope, expected_delta).await?;
            pending.settle(EffectDisposition::Committed)?;
            // A lost settlement acknowledgement can repeat only this exact metadata
            // action; it must not revive the original payload or a recovery reference.
            settle_with_stripe_delta(&client, &envelope, expected_delta).await?;
            let mut response = client.query("SELECT * FROM ONLY $effect; SELECT VALUE id FROM ONLY $reference")
                .bind(("effect", RecordId::new("native_effect", request.operation.hex())))
                .bind(("reference", envelope.outcome_reference.clone())).await.map_err(ModelError::codec)?
                .check().map_err(ModelError::codec)?;
            let Value::Object(outcome) = response.take::<Value>(0).map_err(ModelError::codec)? else {
                return Err(ModelError::Schema("native terminal settlement outcome object"));
            };
            assert_eq!(int_field(&outcome, "revision")?, 1, "settlement leaves the original outcome unchanged");
            assert_eq!(hash_field(&outcome, "request")?, request.request);
            let reference: Value = response.take(1).map_err(ModelError::codec)?;
            assert_eq!(reference, if policy == RecoveryReferencePolicy::RetainCaller {
                Value::RecordId(envelope.outcome_reference)
            } else { Value::None });
        }
        assert_eq!(scope.pending_count()?, 0);
        // Caller-retained ownership is a separate explicit consumer. Release it
        // while production is still open; parent recovery below cannot admit a
        // new history effect through the original stopped production scope.
        while let Some(request) = retained.last().copied() {
            super::super::history::release_outcome(&client, request,
                &request.operation.hex(), "caller").await?;
            let _ = retained.pop();
        }
        // This response is observed and durably recorded. Stop only subsequent
        // settlement issuance; this is not a simulated transport acknowledgement loss.
        let payload = PreparedNativePayload::new("RETURN true", Variables::new())?;
        let request = NativeRequest::issue(&client, payload.digest).await?;
        let plan = owner_effect_plan(request, &EffectOwner::Installation, payload.sql,
            payload.bindings, false)?;
        let pending = scope.prepare(request, plan.recovery_owner.clone(), plan.reference.clone(),
            RecoveryReferencePolicy::ReleaseRecovery)?;
        super::super::retry::transaction_named(&client, &plan.query, plan.bindings, false,
            "terminal_settlement_stopped_control_apply").await?;
        pending.committed()?;
        let envelope = pending.inspect(Clone::clone)?;
        let receipt = &envelope.host_owner.receipt;
        let path = receipt.parent().ok_or(ModelError::Schema("pending control receipt parent"))?
            .with_extension("requests")
            .join(receipt.file_stem().ok_or(ModelError::Schema("pending control receipt name"))?)
            .join(format!("{}.json", request.operation.hex()));
        let persisted = read_recovery_envelope(&path, scope.target(), scope.budget())?;
        assert_eq!(persisted.envelope(), &envelope);
        drop(persisted);
        scope.cancellation().cancel();
        assert!(matches!(resume_recovery_scope(&scope).await,
            Err(ModelError::Conflict("native recovery issuance stopped; owner retained"))));
        assert_eq!(scope.pending_count()?, 1);
        assert_eq!(pending.inspect(Clone::clone)?, envelope);
        let persisted = read_recovery_envelope(&path, scope.target(), scope.budget())?;
        assert_eq!(persisted.envelope(), &envelope,
            "stopped release preserves the original durable recipient and terminal knowledge");
        drop(persisted);
        let mut response = client.query("SELECT VALUE id FROM ONLY $reference")
            .bind(("reference", envelope.outcome_reference.clone())).await.map_err(ModelError::codec)?
            .check().map_err(ModelError::codec)?;
        assert_eq!(response.take::<Value>(0).map_err(ModelError::codec)?,
            Value::RecordId(envelope.outcome_reference.clone()));
        resume_recovery_scope_admitted(&scope, &recovery).await?;
        assert_eq!(scope.pending_count()?, 0);
        assert!(!path.exists(), "checked settlement consumes the original durable sidecar");
        assert!(scope.cancellation().stopped());
        assert!(!recovery.stopped());
        assert!(matches!(scope.prepare(request, envelope.owner.clone(),
            envelope.outcome_reference.clone(), envelope.reference_policy),
            Err(ModelError::Conflict("native recovery admission stopped"))),
            "parent recovery must not reopen production admission");
        let mut response = client.query("SELECT * FROM ONLY $effect; SELECT VALUE id FROM ONLY $reference")
            .bind(("effect", RecordId::new("native_effect", request.operation.hex())))
            .bind(("reference", envelope.outcome_reference)).await.map_err(ModelError::codec)?
            .check().map_err(ModelError::codec)?;
        let Value::Object(outcome) = response.take::<Value>(0).map_err(ModelError::codec)? else {
            return Err(ModelError::Schema("native recovered terminal outcome object"));
        };
        assert_eq!(int_field(&outcome, "revision")?, 1);
        assert_eq!(hash_field(&outcome, "request")?, request.request);
        assert_eq!(response.take::<Value>(1).map_err(ModelError::codec)?, Value::None);
        Ok::<(), ModelError>(())
    })).catch_unwind().await;
    let mut completion = Completion::default();
    completion.step("terminal settlement pending recovery", resume_recovery_scope_admitted(&scope, &recovery).await);
    for request in retained {
        completion.step("terminal settlement caller reference release", with_recovery_scope(scope.clone(),
            super::super::history::release_outcome(&client, request, &request.operation.hex(), "caller")).await);
    }
    drop(lease);
    completion.step("terminal settlement owned session finality", factory.drain(&recovery).await);
    completion.step("terminal settlement parent recovery receipt", recovery_signal.complete(&completion).await);
    match result {
        Ok(result) => complete(result, completion).unwrap(),
        Err(panic) => {
            if !completion.failures.is_empty() { eprintln!("terminal settlement control cleanup: {completion:?}"); }
            std::panic::resume_unwind(panic);
        }
    }
}
