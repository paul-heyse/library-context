//! A deterministic apply/recovery collision on an explicitly owned disposable target.
//! Successful assertions deliberately retain unknown native finality for campaign discard.
use super::*;
use futures::FutureExt;
use lctx_model::domain::{
    completion::{Completion, LocalState, RemoteState},
    resources::ResourceBudget,
};
use std::{panic::AssertUnwindSafe, sync::Arc};

fn require_owned_campaign(config: &crate::RuntimeConfig) {
    assert_eq!(config.authentication, crate::AuthenticationScope::Database);
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Reuse the installation adapter, and prove the exact fixture binding before
    // opening a session. Ignored selection alone cannot authorize retained effects.
    let proof = std::process::Command::new(repository.join(".venv/bin/python"))
        .env("PYTHONPATH", repository.join("scripts"))
        .args(["-c", r#"
import json
import os
from pathlib import Path
import sys
import surrealdb_service as service
installation = service.Installation.load(ready=False)
record = installation.record
assert record.get("kind") == "campaign" and record.get("purpose") == "surrealdb-lifecycle"
assert installation.id == os.environ["LCTX_SURREAL_INSTALLATION_ID"]
assert Path(os.environ["LCTX_SURREAL_SERVICE_CONFIG"]).resolve() == installation.directory / "installation.json"
endpoint, namespace, generation, database, selection = sys.argv[1:]
assert installation.grpc_endpoint == endpoint and record["namespace"] == namespace
assert bytes(record["service_generation"]).hex() == generation == os.environ["LCTX_SURREAL_GENERATION"]
assert database in record["databases"]
attachment = os.environ["LCTX_FIXTURE_ATTACHMENT"]
directory = installation.directory / "attachments" / attachment
assert Path(os.environ["LCTX_NATIVE_DRAIN_DIRECTORY"]).resolve() == directory / "native-drain"
assert Path(os.environ["LCTX_COMPILER_RUNTIME_CONFIG"]).resolve() == directory / "compiler-runtime.json"
assert Path(selection).resolve() == directory / "scratch/selected.json"
binding = json.loads((directory / "record.json").read_text())
assert binding["id"] == attachment and not binding["released"]
assert binding["native_drain"] == {"attachment": attachment, "directory": str(directory / "native-drain"), "generation": generation}
"#])
        .arg(&config.endpoint).arg(config.namespace.as_str())
        .arg(config.service_generation.hex()).arg(config.database.as_str())
        .arg(&config.selection).output().unwrap();
    assert!(
        proof.status.success(),
        "owned collision campaign refused: {}",
        String::from_utf8_lossy(&proof.stderr)
    );
}

struct Source {
    client: Arc<Surreal<Client>>,
    scope: Arc<RecoveryScope>,
    attempt: ContentHash,
    _lease: FactoryLease,
}
impl Drop for Source {
    fn drop(&mut self) {
        // Stop future production, including on assertion panic. This is not an
        // acknowledgement or a release of any exact request/source obligation.
        self.scope.cancellation().cancel();
    }
}
async fn source(
    config: &crate::RuntimeConfig,
    factory: &Arc<RecoveryFactoryOwner>,
) -> Result<Source, ModelError> {
    let (client, lease) = crate::reader::connect_owned(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
        factory,
    )
    .await?;
    let scope = lease.recovery_scope().clone();
    let attempt = with_recovery_scope(scope.clone(), async {
        let allocation = prepare_attempt_allocation(
            fresh_identity("collision-source")?,
            IssuanceEra::capture(&client).await?,
        )?;
        let attempt = allocation.attempt;
        lease.register_attempt(attempt, allocation.request)?;
        allocation.issue(&client).await?;
        lease.source_ready()?;
        Ok::<_, ModelError>(attempt)
    })
    .await?;
    Ok(Source {
        client,
        scope,
        attempt,
        _lease: lease,
    })
}

async fn inspect(
    reader: &crate::NativeReader<()>,
    request: NativeRequest,
    reference: &RecordId,
) -> Result<Object, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert(
        "effect",
        RecordId::new("native_effect", request.operation.hex()),
    );
    bindings.insert("reference", reference.clone());
    reader.query_native(
        "RETURN {effect:(SELECT * FROM ONLY $effect),reference:(SELECT * FROM ONLY $reference)}",
        bindings,
    ).await
}

async fn collision(
    config: &crate::RuntimeConfig,
    factory: &Arc<RecoveryFactoryOwner>,
    observer: &crate::NativeReader<()>,
    recovery: &RetryCancellation,
    retain: bool,
) -> Result<(), ModelError> {
    let winner = source(config, factory).await?;
    let loser = source(config, factory).await?;
    assert_ne!(winner.attempt, loser.attempt);
    assert!(!Arc::ptr_eq(&winner.client, &loser.client));
    assert!(!Arc::ptr_eq(&winner.scope, &loser.scope));
    let payload = PreparedNativePayload::new("RETURN true", Variables::new())?;
    let request = NativeRequest::issue(&winner.client, payload.digest).await?;
    // These are precisely the public execute_recorded pre-observations. Pause
    // both continuations here so no second replay lookup can conceal the race.
    for client in [&winner.client, &loser.client] {
        let (disposition, row) = observe_request(client, request).await?;
        assert_eq!(disposition, EffectDisposition::Pending);
        assert!(
            row.is_none(),
            "both original pre-observations must see actual absence"
        );
    }
    let winner_plan = owner_effect_plan(
        request,
        &EffectOwner::OpenAttempt(winner.attempt),
        payload.sql,
        payload.bindings.clone(),
        retain,
    )?;
    let loser_plan = owner_effect_plan(
        request,
        &EffectOwner::OpenAttempt(loser.attempt),
        payload.sql,
        payload.bindings,
        retain,
    )?;
    let policy = if retain {
        RecoveryReferencePolicy::RetainCaller
    } else {
        RecoveryReferencePolicy::ReleaseRecovery
    };
    assert_eq!(
        winner_plan.reference, loser_plan.reference,
        "request-only reference identity is shared; ownership must still be checked"
    );
    let reference = winner_plan.reference.clone();
    let winner_pending = winner.scope.prepare(
        request,
        winner_plan.recovery_owner.clone(),
        reference.clone(),
        policy,
    )?;
    let loser_pending = loser.scope.prepare(
        request,
        loser_plan.recovery_owner.clone(),
        reference.clone(),
        policy,
    )?;
    let original_winner = winner_pending.inspect(Clone::clone)?;
    let original_loser = loser_pending.inspect(Clone::clone)?;
    assert_eq!(
        original_winner.owner,
        RecoveryOwner::OpenAttempt(winner.attempt)
    );
    assert_eq!(
        original_loser.owner,
        RecoveryOwner::OpenAttempt(loser.attempt)
    );
    assert_ne!(original_winner.host_owner, original_loser.host_owner);
    assert_eq!(
        original_loser.action,
        RecoveryAction::ReconcileOriginalRequest
    );
    assert_eq!(
        original_loser.observed_disposition,
        EffectDisposition::Pending
    );

    // A commits the actual production atomic packet. Its registered owner does
    // not observe the acknowledgement, so its real witness remains retained.
    super::super::checked_transaction(
        winner
            .client
            .query(&winner_plan.query)
            .bind(winner_plan.bindings)
            .await
            .map_err(ModelError::codec)?,
    )?;
    let before = inspect(observer, request, &reference).await?;
    let effect = before
        .get("effect")
        .and_then(Value::as_object)
        .expect("actual winner outcome");
    assert_eq!(effect.get("committed"), Some(&Value::Bool(true)));
    assert_eq!(effect.get("resolved"), Some(&Value::Bool(true)));
    assert_eq!(int_field(effect, "revision")?, 1);
    assert_eq!(hash_field(effect, "request")?, request.request);
    assert_eq!(
        effect.get("owner"),
        Some(&Value::RecordId(RecordId::new(
            "native_attempt",
            winner.attempt.hex()
        )))
    );
    assert_eq!(effect.get("attempt"), effect.get("owner"));
    let witness = before
        .get("reference")
        .and_then(Value::as_object)
        .expect("actual retained winner reference");
    assert_eq!(witness.get("id"), Some(&Value::RecordId(reference.clone())));
    assert_eq!(
        witness.get("kind"),
        Some(&Value::String(policy.kind().into()))
    );

    // Resume B after its already-absent observation at the exact execution
    // continuation. Calling execute_request again would incorrectly test replay
    // prevalidation instead of the apply-versus-recovery collision.
    let error = with_recovery_scope(loser.scope.clone(), async {
        execute_owner_plan(&loser.client, request, loser_plan)?.await
    })
    .await
    .expect_err("the losing source must never acquire the winner's success");
    assert!(
        error
            .primary()
            .expect("original apply refusal remains primary")
            .to_string()
            .contains("native operation identity collision"),
        "{error:?}"
    );
    assert!(
        !error.permits_storage_cleanup(),
        "collision retains actual unsettled ownership: {error:?}"
    );
    assert!(
        !error.has_committed_effect(),
        "winner commit cannot become the losing owner's commit"
    );
    let ModelError::Completion(failed) = &error else {
        panic!("expected protected collision finality, observed {error:?}");
    };
    assert_eq!(failed.completion.local, LocalState::Outstanding);
    assert_eq!(failed.completion.remote, RemoteState::Unknown);
    assert_eq!(loser.scope.pending_count()?, 1);
    assert_eq!(
        loser_pending.inspect(Clone::clone)?,
        original_loser,
        "failed reconciliation cannot rewrite the original losing owner or terminal knowledge"
    );
    assert_eq!(winner_pending.inspect(Clone::clone)?, original_winner);
    assert_eq!(
        inspect(observer, request, &reference).await?,
        before,
        "failed apply/recovery cannot change the winner's full row or reference"
    );

    // Independent recovery admission still cannot adopt another owner's result.
    resume_recovery_scope_admitted(&loser.scope, recovery)
        .await
        .expect_err("admitted recovery must preserve the owner mismatch");
    assert_eq!(loser.scope.pending_count()?, 1);
    assert_eq!(loser_pending.inspect(Clone::clone)?, original_loser);
    assert_eq!(winner.scope.pending_count()?, 1);
    assert_eq!(winner_pending.inspect(Clone::clone)?, original_winner);
    assert_eq!(inspect(observer, request, &reference).await?, before);
    winner.scope.join_finalization_reads().await?;
    loser.scope.join_finalization_reads().await?;
    eprintln!(
        "collision body passed policy={} operation={} winner={} loser={}; exact pending recipients {} and {} remain for authenticated whole-campaign discard",
        policy.kind(),
        request.operation.hex(),
        winner.attempt.hex(),
        loser.attempt.hex(),
        original_winner.host_owner.receipt.display(),
        original_loser.host_owner.receipt.display()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "wholly owned disposable native campaign only; deliberately retains owner collision obligations and Unknown finality for authenticated whole-target discard"]
async fn native_cross_owner_apply_recovery_collision_preserves_winner_and_original_pending_owner() {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned collision campaign"),
    ))
    .unwrap();
    require_owned_campaign(&config);
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let factory =
        RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget.clone()).unwrap();
    let observer_factory =
        RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget.clone()).unwrap();
    let recovery = RetryCancellation::default();
    let stop = recovery.clone();
    let signals = config
        .signal_owner(
            "native owner collision requires campaign discard",
            move || stop.cancel(),
        )
        .unwrap();
    let mut observer = None;
    let mut observer_lease = None;
    let result = AssertUnwindSafe(async {
        let (_, lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &observer_factory,
        )
        .await?;
        let scope = lease.recovery_scope().clone();
        observer_lease = Some(lease);
        observer = Some(scope.finalization_reader(scope.cancellation()).await?);
        for retain in [true, false] {
            collision(
                &config,
                &factory,
                observer.as_ref().unwrap(),
                &recovery,
                retain,
            )
            .await?;
        }
        Ok::<_, ModelError>(())
    })
    .catch_unwind()
    .await;
    let mut finality = Completion {
        local: LocalState::Outstanding,
        remote: RemoteState::Unknown,
        ..Default::default()
    };
    if let Some(reader) = observer.as_ref() {
        finality.step("collision observer reader drainage", reader.close().await);
    }
    drop(observer);
    drop(observer_lease);
    finality.step(
        "collision observer owner drainage",
        observer_factory.drain(&recovery).await,
    );
    // The actual source factories/pending recipients survive both body errors and
    // panics. Never mark the collision Committed, drain these sources, or release
    // A's witness to make fixture cleanup look conclusive. The campaign disposer
    // alone settles their obligations after authenticated execution/daemon drain.
    let recorded = signals.complete(&finality).await;
    eprintln!("collision native finality remains outstanding/unknown: {finality:?}");
    match result {
        Ok(result) => {
            result.unwrap();
            recorded.unwrap();
            assert!(
                finality.failures.is_empty(),
                "observer finality failed: {finality:?}"
            );
        }
        Err(panic) => {
            if let Err(error) = recorded {
                eprintln!("collision finality receipt also failed: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
    }
}

#[test]
fn recovery_owner_context_matrix_preserves_kind_identity_attempt_slot_and_terminal_epoch() {
    // Finite validation only: these rows establish no native execution success.
    // Expected contexts are authored independently of EffectOwner::replay_context.
    let identity = ContentHash::of(b"owner-context-matrix");
    let request = NativeRequest {
        issuance: IssuanceEra {
            generation: identity,
            era: 1,
        },
        operation: ContentHash::of(b"owner-context-request"),
        request: identity,
    };
    let attempt = RecordId::new("native_attempt", identity.hex());
    let retirement = RecordId::new("native_retirement", identity.hex());
    let cases = [
        (
            RecoveryOwner::Installation,
            "installation",
            coordination::operation_owner(request),
            false,
            1,
        ),
        (
            RecoveryOwner::OpenAttempt(identity),
            "attempt",
            attempt.clone(),
            true,
            1,
        ),
        (
            RecoveryOwner::FrozenAttempt(identity),
            "frozen_attempt",
            attempt.clone(),
            true,
            1,
        ),
        (
            RecoveryOwner::TerminalAttempt { identity, epoch: 9 },
            "terminal_attempt",
            attempt.clone(),
            true,
            9,
        ),
        (
            RecoveryOwner::Pin(RecordId::new("native_pin", identity.hex())),
            "pin",
            RecordId::new("native_pin", identity.hex()),
            false,
            1,
        ),
        (
            RecoveryOwner::Retirement(retirement.clone()),
            "retirement",
            retirement.clone(),
            false,
            1,
        ),
        (
            RecoveryOwner::RetirementSetup(retirement.clone()),
            "retirement_setup",
            retirement,
            false,
            1,
        ),
        (
            RecoveryOwner::Cleanup(RecordId::new("native_cleanup", identity.hex())),
            "cleanup",
            RecordId::new("native_cleanup", identity.hex()),
            false,
            1,
        ),
    ];
    for (index, (original, kind, owner, has_attempt, epoch)) in cases.iter().enumerate() {
        let mut row = Object::new();
        row.insert("owner_kind", *kind);
        row.insert("owner", owner.clone());
        row.insert(
            "attempt",
            if *has_attempt {
                Value::RecordId(attempt.clone())
            } else {
                Value::None
            },
        );
        row.insert(
            "attempt_slot",
            if *has_attempt {
                surrealdb::types::SurrealValue::into_value(coordination::attempt_slot(
                    request.operation,
                ))
            } else {
                Value::None
            },
        );
        row.insert("epoch", *epoch as i64);
        row.insert("committed", true);
        row.insert("resolved", true);
        for (other_index, (other, _, _, _, _)) in cases.iter().enumerate() {
            assert_eq!(
                validate_replay_owner(request, &other.effect_owner(), &row).is_ok(),
                index == other_index,
                "{original:?} must not become {other:?}"
            );
        }
        let changed_fields = [
            ("owner", Value::RecordId(RecordId::new("foreign", "owner"))),
            ("owner_kind", Value::String("foreign".into())),
            (
                "attempt",
                Value::RecordId(RecordId::new("native_attempt", "foreign")),
            ),
            (
                "attempt_slot",
                Value::Number(surrealdb::types::Number::Int(-1)),
            ),
            ("epoch", Value::Number(surrealdb::types::Number::Int(0))),
            ("committed", Value::Bool(false)),
            ("resolved", Value::Bool(false)),
        ];
        for (field, value) in changed_fields {
            let mut changed = row.clone();
            changed.insert(field, value);
            assert!(
                validate_replay_owner(request, &original.effect_owner(), &changed).is_err(),
                "{original:?} must refuse changed {field}"
            );
        }
        if matches!(original, RecoveryOwner::TerminalAttempt { .. }) {
            row.insert("epoch", 8i64);
            assert!(
                validate_replay_owner(request, &original.effect_owner(), &row).is_err(),
                "positive but stale terminal epoch must refuse"
            );
        }
    }
}
