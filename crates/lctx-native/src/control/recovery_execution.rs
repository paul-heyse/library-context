//! Explicit exact-request maintenance after its authenticated host owner is gone.
//! This route has no production payload, SQL or authority-refresh input.
use super::*;
use crate::signal::PendingRequestHostOwner;
use lctx_model::domain::resources::ResourceBudget;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};

#[derive(Debug, Serialize)]
pub struct RecoverySettlement {
    pub request: NativeRequest,
    pub disposition: EffectDisposition,
    pub reference_policy: RecoveryReferencePolicy,
}
fn require_host_gone(host: &PendingRequestHostOwner) -> Result<(), ModelError> {
    let boot =
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(ModelError::codec)?;
    if boot.trim() != host.boot_id {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if std::fs::metadata("/proc/self/ns/pid")
            .map_err(ModelError::codec)?
            .ino()
            != host.pid_namespace
        {
            return Err(ModelError::Conflict(
                "native recovery host PID namespace unobserved",
            ));
        }
    }
    let stat = match std::fs::read_to_string(format!("/proc/{}/stat", host.pid)) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ModelError::codec(error)),
    };
    let ticks = stat
        .rsplit_once(')')
        .and_then(|(_, fields)| fields.split_whitespace().nth(19))
        .ok_or(ModelError::Conflict(
            "native recovery host process identity",
        ))?
        .parse::<u64>()
        .map_err(ModelError::codec)?;
    if ticks == host.start_ticks {
        return Err(ModelError::Conflict(
            "native recovery original host owner remains live",
        ));
    }
    Ok(())
}
fn check_host_receipt(
    path: &Path,
    host: &PendingRequestHostOwner,
    target: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<File, ModelError> {
    if host.receipt.canonicalize().map_err(ModelError::codec)? != host.receipt {
        return Err(ModelError::Conflict(
            "native recovery host receipt redirected",
        ));
    }
    let file = File::open(&host.receipt).map_err(ModelError::codec)?;
    // Every explicit resumer locks the original stable receipt inode. A live
    // original writer is independently refused by its complete process identity.
    file.lock().map_err(ModelError::codec)?;
    require_host_gone(host)?;
    let length: usize = file
        .metadata()
        .map_err(ModelError::codec)?
        .len()
        .try_into()
        .map_err(ModelError::codec)?;
    let _charge = budget.reserve(
        "native recovery host receipt",
        length
            .checked_mul(4)
            .ok_or(ModelError::Schema("native recovery host receipt size"))?,
    )?;
    let bytes = std::fs::read(&host.receipt).map_err(ModelError::codec)?;
    if bytes.len() != length {
        return Err(ModelError::Conflict("native recovery host receipt changed"));
    }
    let row: serde_json::Value = serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
    if row["kind"] != "native-process-owner"
        || row["generation"] != target.generation.hex()
        || row["process"]["pid"] != host.pid
        || row["process"]["start_ticks"] != host.start_ticks
        || row["host_boot_id"] != host.boot_id
        || row["host_pid_namespace"] != host.pid_namespace
        || row["pending_requests"].as_str().map(Path::new) != path.parent()
    {
        return Err(ModelError::Conflict(
            "native recovery authenticated host receipt correspondence",
        ));
    }
    Ok(file)
}
/// Explicit caller authorization selects this exact installed target and request
/// file. Parsing alone grants neither maintenance authority nor deletion rights.
pub async fn recover_pending_request_file(
    config: &crate::RuntimeConfig,
    path: &Path,
    budget: &ResourceBudget,
) -> Result<RecoverySettlement, ModelError> {
    let target = RecoveryTarget {
        generation: config.service_generation,
        namespace: config.namespace.as_str().into(),
        database: config.database.as_str().into(),
        endpoint: Some(config.endpoint.clone()),
    };
    let first = read_recovery_envelope(path, &target, budget)?;
    let _host_lock = check_host_receipt(
        path,
        &first.envelope().host_owner,
        &first.envelope().target,
        budget,
    )?;
    // A competing successful resumer may have settled it before the lock. Never
    // recreate its metadata or infer outcome from the absent file.
    let mut original = read_recovery_envelope(path, &target, budget)?;
    let owner = crate::maintenance::MaintenanceOwner::connect(config, budget).await?;
    let client = owner.client().clone();
    let result = with_recovery_scope(owner.scope(), async {
        owner.require_closed_admission().await?;
        if owner.scope().cancellation().stopped() {
            return Err(ModelError::Conflict(
                "native request recovery issuance stopped",
            ));
        }
        let disposition = match original.envelope().action {
            RecoveryAction::ReconcileOriginalRequest | RecoveryAction::ReconcileOperationRoot => {
                let disposition = authorization::reconcile_pending_envelope(
                    &client,
                    original.envelope(),
                    owner.scope().cancellation(),
                )
                .await?;
                original.record_terminal(disposition)?;
                disposition
            }
            RecoveryAction::ReleaseTerminalReference | RecoveryAction::ReleaseTerminalOperation => original.envelope().observed_disposition,
        };
        authorization::settle_reference(&client, original.envelope(), owner.scope().cancellation())
            .await?;
        let settlement = RecoverySettlement {
            request: original.envelope().request,
            disposition,
            reference_policy: original.envelope().reference_policy,
        };
        original.settle(disposition)?;
        Ok(settlement)
    })
    .await;
    owner.finish(result).await
}

#[derive(Debug, Serialize)]
pub struct SourceRecoverySettlement {
    pub kind: &'static str,
    pub identity: ContentHash,
    pub allocation: NativeRequest,
    pub disposition: EffectDisposition,
    pub pins_settled: usize,
}
enum SidecarKind {
    Request,
    Source,
    Pin,
}
fn sidecar_kind(path: &Path, budget: &ResourceBudget) -> Result<SidecarKind, ModelError> {
    #[derive(Deserialize)]
    struct Header<'a> {
        #[serde(borrow)]
        kind: Option<&'a str>,
    }
    let size: usize = std::fs::metadata(path)
        .map_err(ModelError::codec)?
        .len()
        .try_into()
        .map_err(ModelError::codec)?;
    let _charge = budget.reserve("native recovery sidecar dispatch", size)?;
    let bytes = std::fs::read(path).map_err(ModelError::codec)?;
    if bytes.len() != size {
        return Err(ModelError::Conflict("native recovery sidecar changed"));
    }
    match serde_json::from_slice::<Header<'_>>(&bytes)
        .map_err(ModelError::codec)?
        .kind
    {
        None => Ok(SidecarKind::Request),
        Some("source_finalization") => Ok(SidecarKind::Source),
        Some("pin_finalization") => Ok(SidecarKind::Pin),
        _ => Err(ModelError::Conflict("native recovery sidecar kind")),
    }
}
// The original stable receipt lock covers its entire finite outstanding set.
// Each request is parsed/charged separately; no accumulated replay history exists.
async fn settle_original_requests_locked(
    client: &Surreal<Client>,
    marker_path: &Path,
    host: &PendingRequestHostOwner,
    target: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let scope = capture_recovery_scope().ok_or(ModelError::Conflict(
        "native original recovery owning scope",
    ))?;
    for entry in std::fs::read_dir(
        marker_path
            .parent()
            .ok_or(ModelError::Schema("native source marker parent"))?,
    )
    .map_err(ModelError::codec)?
    {
        let path = entry.map_err(ModelError::codec)?.path();
        if path == marker_path {
            continue;
        }
        match sidecar_kind(&path, budget)? {
            SidecarKind::Source => {
                let marker = read_source_finalization_marker(&path, target, budget)?;
                if &marker.marker().host_owner != host {
                    return Err(ModelError::Conflict("native source marker host differs"));
                }
                continue;
            }
            SidecarKind::Pin => {
                let marker = read_pin_finalization_marker(&path, target, budget)?;
                if &marker.marker().host_owner != host {
                    return Err(ModelError::Conflict("native pin marker host differs"));
                }
                continue;
            }
            SidecarKind::Request => {}
        }
        let mut request = read_recovery_envelope(&path, target, budget)?;
        if &request.envelope().host_owner != host {
            return Err(ModelError::Conflict("native source request host differs"));
        }
        let disposition = match request.envelope().action {
            RecoveryAction::ReconcileOriginalRequest | RecoveryAction::ReconcileOperationRoot => {
                let disposition = authorization::reconcile_pending_envelope(
                    client,
                    request.envelope(),
                    scope.cancellation(),
                )
                .await?;
                request.record_terminal(disposition)?;
                disposition
            }
            RecoveryAction::ReleaseTerminalReference | RecoveryAction::ReleaseTerminalOperation => request.envelope().observed_disposition,
        };
        authorization::settle_reference(client, request.envelope(), scope.cancellation()).await?;
        request.settle(disposition)?;
    }
    Ok(())
}
async fn settle_pin_marker_locked(
    scope: &Arc<RecoveryScope>,
    path: &Path,
    host: &PendingRequestHostOwner,
    target: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<SourceRecoverySettlement, ModelError> {
    let mut pin = read_pin_finalization_marker(path, target, budget)?;
    if &pin.marker().host_owner != host {
        return Err(ModelError::Conflict("native pin finalization host differs"));
    }
    let disposition = authorization::finalize_factory_pin_admitted(
        scope,
        pin.marker(),
        scope.cancellation(),
        authorization::PinFinalizationAuthority::HostGoneMaintenance,
    )
    .await?;
    pin.record_terminal(disposition)?;
    authorization::release_factory_pin_references_admitted(
        scope,
        pin.marker(),
        scope.cancellation(),
    )
    .await?;
    resume_recovery_scope(scope).await?;
    let settlement = SourceRecoverySettlement {
        kind: "reader_pin",
        identity: pin.marker().pin,
        allocation: pin.marker().allocation,
        disposition,
        pins_settled: 1,
    };
    pin.settle()?;
    Ok(settlement)
}
/// Host-loss repair for one exact compiler source. The allocation and native
/// object witnesses remain held until its durable terminal marker is written.
pub async fn recover_source_finalization_file(
    config: &crate::RuntimeConfig,
    path: &Path,
    budget: &ResourceBudget,
) -> Result<SourceRecoverySettlement, ModelError> {
    let target = RecoveryTarget {
        generation: config.service_generation,
        namespace: config.namespace.as_str().into(),
        database: config.database.as_str().into(),
        endpoint: Some(config.endpoint.clone()),
    };
    let first = read_source_finalization_marker(path, &target, budget)?;
    let _host_lock = check_host_receipt(
        path,
        &first.marker().host_owner,
        &first.marker().target,
        budget,
    )?;
    let mut source = read_source_finalization_marker(path, &target, budget)?;
    let owner = crate::maintenance::MaintenanceOwner::connect(config, budget).await?;
    let scope = owner.scope();
    let result = with_recovery_scope(scope.clone(), async {
        owner.require_closed_admission().await?;
        settle_original_requests_locked(
            owner.client(),
            path,
            &source.marker().host_owner,
            &target,
            budget,
        )
        .await?;
        let mut pins_settled = 0;
        for entry in std::fs::read_dir(
            path.parent()
                .ok_or(ModelError::Schema("native source marker parent"))?,
        )
        .map_err(ModelError::codec)?
        {
            let pin_path = entry.map_err(ModelError::codec)?.path();
            if matches!(sidecar_kind(&pin_path, budget)?, SidecarKind::Pin) {
                settle_pin_marker_locked(
                    &scope,
                    &pin_path,
                    &source.marker().host_owner,
                    &target,
                    budget,
                )
                .await?;
                pins_settled += 1;
            }
        }
        let disposition = authorization::finalize_factory_source_admitted(
            &scope,
            source.marker(),
            scope.cancellation(),
            true,
        )
        .await?;
        source.record_terminal(disposition)?;
        authorization::release_factory_source_references_admitted(
            &scope,
            source.marker(),
            scope.cancellation(),
        )
        .await?;
        resume_recovery_scope(&scope).await?;
        let settlement = SourceRecoverySettlement {
            kind: "compiler_attempt",
            identity: source.marker().attempt,
            allocation: source.marker().allocation,
            disposition,
            pins_settled,
        };
        source.settle()?;
        Ok(settlement)
    })
    .await;
    owner.finish(result).await
}
/// Repair one exact pin after the original host has gone; other pins remain
/// independently named obligations unless their source owner is being finalized.
pub async fn recover_pin_finalization_file(
    config: &crate::RuntimeConfig,
    path: &Path,
    budget: &ResourceBudget,
) -> Result<SourceRecoverySettlement, ModelError> {
    let target = RecoveryTarget {
        generation: config.service_generation,
        namespace: config.namespace.as_str().into(),
        database: config.database.as_str().into(),
        endpoint: Some(config.endpoint.clone()),
    };
    let first = read_pin_finalization_marker(path, &target, budget)?;
    let _host_lock = check_host_receipt(
        path,
        &first.marker().host_owner,
        &first.marker().target,
        budget,
    )?;
    let pin = read_pin_finalization_marker(path, &target, budget)?;
    let owner = crate::maintenance::MaintenanceOwner::connect(config, budget).await?;
    let scope = owner.scope();
    let result = with_recovery_scope(scope.clone(), async {
        owner.require_closed_admission().await?;
        settle_original_requests_locked(
            owner.client(),
            path,
            &pin.marker().host_owner,
            &target,
            budget,
        )
        .await?;
        settle_pin_marker_locked(&scope, path, &pin.marker().host_owner, &target, budget).await
    })
    .await;
    owner.finish(result).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidecar_dispatch_uses_the_actual_marker_wire_kinds() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("sidecar.json");
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let target = RecoveryTarget {
            generation: ContentHash::of(b"generation"),
            namespace: "ns".into(),
            database: "db".into(),
            endpoint: None,
        };
        let host = PendingRequestHostOwner {
            receipt: root.path().join("owner.json"),
            pid: 1,
            start_ticks: 1,
            boot_id: "boot".into(),
            pid_namespace: 1,
        };
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: target.generation,
                era: 1,
            },
            operation: ContentHash::of(b"operation"),
            request: ContentHash::of(b"request"),
        };
        let source = SourceFinalizationMarker {
            schema: 1,
            kind: SourceMarkerKind::SourceFinalization,
            host_owner: host.clone(),
            target: target.clone(),
            attempt: ContentHash::of(b"attempt"),
            allocation: request,
            original_epoch: 1,
            phase: SourceFinalizationPhase::AllocationPending,
            allocation_disposition: EffectDisposition::Pending,
        };
        std::fs::write(&path, serde_json::to_vec(&source).unwrap()).unwrap();
        assert!(matches!(
            sidecar_kind(&path, &budget).unwrap(),
            SidecarKind::Source
        ));
        let pin = PinFinalizationMarker {
            schema: 1,
            kind: PinMarkerKind::PinFinalization,
            host_owner: host,
            target,
            pin: ContentHash::of(b"pin"),
            allocation: request,
            original_epoch: 1,
            phase: SourceFinalizationPhase::AllocationPending,
            allocation_disposition: EffectDisposition::Pending,
        };
        std::fs::write(&path, serde_json::to_vec(&pin).unwrap()).unwrap();
        assert!(matches!(
            sidecar_kind(&path, &budget).unwrap(),
            SidecarKind::Pin
        ));
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(matches!(
            sidecar_kind(&path, &budget).unwrap(),
            SidecarKind::Request
        ));
        std::fs::write(&path, br#"{"kind":"unknown_owner"}"#).unwrap();
        assert!(sidecar_kind(&path, &budget).is_err());
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn exact_live_host_refuses_and_reused_identity_does_not_block_recovery() {
        use std::os::unix::fs::MetadataExt;
        let ticks = std::fs::read_to_string("/proc/self/stat")
            .unwrap()
            .rsplit_once(')')
            .unwrap()
            .1
            .split_whitespace()
            .nth(19)
            .unwrap()
            .parse()
            .unwrap();
        let mut host = PendingRequestHostOwner {
            receipt: "/unused/receipt.json".into(),
            pid: std::process::id(),
            start_ticks: ticks,
            boot_id: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim()
                .into(),
            pid_namespace: std::fs::metadata("/proc/self/ns/pid").unwrap().ino(),
        };
        assert!(require_host_gone(&host).is_err());
        host.start_ticks += 1;
        require_host_gone(&host).unwrap();
        host.pid_namespace += 1;
        assert!(require_host_gone(&host).is_err());
        host.boot_id = "confirmed different boot".into();
        require_host_gone(&host).unwrap();
    }
}
