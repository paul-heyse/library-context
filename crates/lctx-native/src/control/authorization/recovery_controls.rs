//! Actual native packets model a committed acknowledgement unobserved by its owner.
//! This is not network fault injection; the fixture must be a task-owned RocksDB target.
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
