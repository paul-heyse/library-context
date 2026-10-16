//! Compact exact outstanding requests retained by the actual source-finalization owner.
//! No payload, SQL, credentials or executor is part of this recovery capability.
use super::*;
mod factory;
use crate::signal::{PendingRequestHostOwner, PendingRequestRecipient};
pub use factory::{
    FactoryHandback, FactoryLease, PinFinalizationMarker, PinMarkerKind, ReadPinFinalizationMarker,
    ReadSourceFinalizationMarker, RecoveryFactoryOwner, SourceFinalizationMarker,
    SourceFinalizationPhase, SourceMarkerKind, read_pin_finalization_marker,
    read_source_finalization_marker,
};
use lctx_model::domain::resources::{Reservation, ResourceBudget};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    io::{Read, Write},
    path::Path,
    sync::{Mutex, Weak},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryTarget {
    pub generation: ContentHash,
    pub namespace: String,
    pub database: String,
    pub endpoint: Option<String>,
}
impl RecoveryTarget {
    fn validate(&self) -> Result<(), ModelError> {
        if self.namespace.is_empty()
            || self.database.is_empty()
            || self.namespace.contains('\0')
            || self.database.contains('\0')
            || self.endpoint.as_ref().is_some_and(|endpoint| {
                endpoint.is_empty() || endpoint.contains(['@', '?', '#', '\0'])
            })
        {
            return Err(ModelError::Conflict("native recovery target"));
        }
        Ok(())
    }
    fn heap_bytes(&self) -> usize {
        self.namespace.capacity()
            + self.database.capacity()
            + self.endpoint.as_ref().map_or(0, String::capacity)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryOwner {
    Installation,
    OpenAttempt(ContentHash),
    FrozenAttempt(ContentHash),
    Pin(RecordId),
    TerminalAttempt { identity: ContentHash, epoch: i64 },
    Retirement(RecordId),
    RetirementSetup(RecordId),
    Cleanup(RecordId),
}
impl RecoveryOwner {
    pub(super) fn effect_owner(&self) -> EffectOwner {
        match self {
            Self::Installation => EffectOwner::Installation,
            Self::OpenAttempt(id) => EffectOwner::OpenAttempt(*id),
            Self::FrozenAttempt(id) => EffectOwner::FrozenAttempt(*id),
            Self::Pin(id) => EffectOwner::Pin(id.clone()),
            Self::TerminalAttempt { identity, epoch } => EffectOwner::TerminalAttempt(TerminalAttempt { identity: *identity, epoch: *epoch }),
            Self::Retirement(id) => EffectOwner::Retirement(id.clone()),
            Self::RetirementSetup(id) => EffectOwner::RetirementSetup(id.clone()),
            Self::Cleanup(id) => EffectOwner::Cleanup(id.clone()),
        }
    }
    pub(super) fn from_effect(owner: &EffectOwner) -> Self {
        match owner {
            EffectOwner::Installation => Self::Installation,
            EffectOwner::OpenAttempt(id) => Self::OpenAttempt(*id),
            EffectOwner::FrozenAttempt(id) => Self::FrozenAttempt(*id),
            EffectOwner::Pin(id) => Self::Pin(id.clone()),
            EffectOwner::TerminalAttempt(owner) => Self::TerminalAttempt {
                identity: owner.identity,
                epoch: owner.epoch,
            },
            EffectOwner::Retirement(id) => Self::Retirement(id.clone()),
            EffectOwner::RetirementSetup(id) => Self::RetirementSetup(id.clone()),
            EffectOwner::Cleanup(id) => Self::Cleanup(id.clone()),
        }
    }
    fn heap_bytes(&self) -> Result<usize, ModelError> {
        match self {
            Self::Pin(id) => record_bytes(id, "native_pin"),
            Self::Retirement(id) | Self::RetirementSetup(id) => {
                record_bytes(id, "native_retirement")
            }
            Self::Cleanup(id) => record_bytes(id, "native_cleanup"),
            Self::TerminalAttempt { epoch, .. } if *epoch <= 0 => {
                Err(ModelError::Conflict("native recovery terminal epoch"))
            }
            _ => Ok(0),
        }
    }
}
fn record_bytes(id: &RecordId, table: &str) -> Result<usize, ModelError> {
    match &id.key {
        surrealdb::types::RecordIdKey::String(key) if id.table.as_str() == table => {
            Ok(id.table.as_str().len() + key.as_str().len())
        }
        _ => Err(ModelError::Conflict("native recovery record identity")),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    ReconcileOriginalRequest,
    ReleaseTerminalReference,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryReferencePolicy {
    RetainCaller,
    ReleaseRecovery,
}
impl RecoveryReferencePolicy {
    pub fn kind(self) -> &'static str {
        match self {
            Self::RetainCaller => "caller",
            Self::ReleaseRecovery => "recovery",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingRecoveryEnvelope {
    pub schema: u8,
    pub host_owner: PendingRequestHostOwner,
    pub request: NativeRequest,
    pub target: RecoveryTarget,
    pub owner: RecoveryOwner,
    pub outcome_reference: RecordId,
    pub reference_policy: RecoveryReferencePolicy,
    pub action: RecoveryAction,
    pub observed_disposition: EffectDisposition,
}
impl PendingRecoveryEnvelope {
    /// Deserialization is observation, never native maintenance/admission authority.
    pub fn validate(&self, expected: &RecoveryTarget) -> Result<(), ModelError> {
        self.target.validate()?;
        if !self.host_owner.receipt.is_absolute()
            || self.host_owner.pid == 0
            || self.host_owner.boot_id.is_empty()
            || self.host_owner.pid_namespace == 0
        {
            return Err(ModelError::Conflict(
                "native pending recovery host identity",
            ));
        }
        if self.schema != 1
            || self.target.generation != expected.generation
            || self.target.namespace != expected.namespace
            || self.target.database != expected.database
            || self
                .target
                .endpoint
                .as_ref()
                .is_some_and(|endpoint| expected.endpoint.as_ref() != Some(endpoint))
            || self.request.issuance.generation != expected.generation
            || self.request.issuance.era <= 0
        {
            return Err(ModelError::Conflict(
                "native pending recovery correspondence",
            ));
        }
        self.owner.heap_bytes()?;
        record_bytes(&self.outcome_reference, "native_outcome_ref")?;
        if self.outcome_reference
            != super::authorization::request_outcome_reference(self.request, self.reference_policy)?
        {
            return Err(ModelError::Conflict(
                "native pending recovery exact reference correspondence",
            ));
        }
        match (self.action, self.observed_disposition) {
            (RecoveryAction::ReconcileOriginalRequest, EffectDisposition::Pending)
            | (
                RecoveryAction::ReleaseTerminalReference,
                EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal,
            ) => Ok(()),
            _ => Err(ModelError::Conflict(
                "native pending recovery action/disposition",
            )),
        }
    }
}
/// Admission covers file bytes and the decoded compact envelope through consumption.
pub struct ReadRecoveryEnvelope {
    envelope: PendingRecoveryEnvelope,
    recipient: PendingRequestRecipient,
    budget: ResourceBudget,
    settled: bool,
    _charge: Box<dyn Reservation>,
}
impl ReadRecoveryEnvelope {
    pub fn envelope(&self) -> &PendingRecoveryEnvelope {
        &self.envelope
    }
    /// Caller holds authenticated maintenance and proves the original host dead
    /// under its exclusive receipt lock. Record known terminal BEFORE ref release.
    pub fn record_terminal(&mut self, disposition: EffectDisposition) -> Result<(), ModelError> {
        if self.settled
            || !matches!(
                disposition,
                EffectDisposition::Committed
                    | EffectDisposition::FencedUncommitted
                    | EffectDisposition::CompactedTerminal
            )
        {
            return Err(ModelError::Conflict(
                "offline recovery terminal disposition absent",
            ));
        }
        if self.envelope.action == RecoveryAction::ReleaseTerminalReference {
            return if self.envelope.observed_disposition == disposition {
                Ok(())
            } else {
                Err(ModelError::Conflict(
                    "offline recovery terminal disposition changed",
                ))
            };
        }
        self.envelope.action = RecoveryAction::ReleaseTerminalReference;
        self.envelope.observed_disposition = disposition;
        if let Err(error) = write_envelope(&self.recipient, &self.budget, &self.envelope, true) {
            self.envelope.action = RecoveryAction::ReconcileOriginalRequest;
            self.envelope.observed_disposition = EffectDisposition::Pending;
            return Err(error);
        }
        Ok(())
    }
    /// Caller has checked exact reference release/retained ownership separately.
    /// This never modifies or upgrades the original native-owner receipt.
    pub fn settle(&mut self, disposition: EffectDisposition) -> Result<(), ModelError> {
        if self.settled {
            return Ok(());
        }
        if self.envelope.action != RecoveryAction::ReleaseTerminalReference
            || self.envelope.observed_disposition != disposition
            || !matches!(
                disposition,
                EffectDisposition::Committed
                    | EffectDisposition::FencedUncommitted
                    | EffectDisposition::CompactedTerminal
            )
        {
            return Err(ModelError::Conflict(
                "offline recovery settlement not established",
            ));
        }
        self.recipient
            .remove(&self.envelope.request.operation.hex())?;
        self.settled = true;
        Ok(())
    }
}
/// Read one exact sidecar. The caller separately holds authenticated maintenance
/// authority for its actual target; a parsed envelope grants no such authority.
pub fn read_recovery_envelope(
    path: &Path,
    expected: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<ReadRecoveryEnvelope, ModelError> {
    if !path.is_absolute() || path.canonicalize().map_err(ModelError::codec)? != path {
        return Err(ModelError::Conflict("native recovery envelope redirected"));
    }
    let mut file = std::fs::File::open(path).map_err(ModelError::codec)?;
    let length: usize = file
        .metadata()
        .map_err(ModelError::codec)?
        .len()
        .try_into()
        .map_err(ModelError::codec)?;
    let bytes = length
        .checked_mul(3)
        .and_then(|bytes| bytes.checked_add(size_of::<ReadRecoveryEnvelope>()))
        .ok_or(ModelError::Conflict("native recovery envelope size"))?;
    let charge = budget.reserve("native recovery envelope read", bytes)?;
    let mut encoded = vec![0; length];
    file.read_exact(&mut encoded).map_err(ModelError::codec)?;
    let mut extra = [0];
    if file.read(&mut extra).map_err(ModelError::codec)? != 0 {
        return Err(ModelError::Conflict(
            "native recovery envelope changed while reading",
        ));
    }
    let envelope: PendingRecoveryEnvelope =
        serde_json::from_slice(&encoded).map_err(ModelError::codec)?;
    envelope.validate(expected)?;
    let receipt = &envelope.host_owner.receipt;
    let parent = receipt
        .parent()
        .ok_or(ModelError::Conflict("native pending owner receipt parent"))?;
    let stem = receipt
        .file_stem()
        .ok_or(ModelError::Conflict("native pending owner receipt name"))?;
    if path.parent() != Some(parent.with_extension("requests").join(stem).as_path()) {
        return Err(ModelError::Conflict(
            "native recovery host receipt/path correspondence",
        ));
    }
    if path.file_name().and_then(|name| name.to_str())
        != Some(format!("{}.json", envelope.request.operation.hex()).as_str())
    {
        return Err(ModelError::Conflict(
            "native recovery envelope operation filename",
        ));
    }
    let recipient = PendingRequestRecipient::for_recovery(
        envelope.host_owner.clone(),
        envelope.target.generation,
    )?;
    Ok(ReadRecoveryEnvelope {
        envelope,
        recipient,
        budget: budget.clone(),
        settled: false,
        _charge: charge,
    })
}
struct PendingState {
    envelope: PendingRecoveryEnvelope,
    settled: bool,
    durable: bool,
}
struct Entry {
    request: NativeRequest,
    state: Mutex<PendingState>,
    // Each outstanding entry retains real resources even if its caller drops.
    _client: Arc<Surreal<Client>>,
    _budget: ResourceBudget,
    _cancellation: RetryCancellation,
    _charge: Box<dyn Reservation>,
}
struct PendingBook {
    entries: Vec<Arc<Entry>>,
    charge: Option<Box<dyn Reservation>>,
}
pub struct RecoveryScope {
    client: Arc<Surreal<Client>>,
    budget: ResourceBudget,
    cancellation: RetryCancellation,
    recipient: PendingRequestRecipient,
    target: RecoveryTarget,
    factory: Mutex<Weak<RecoveryFactoryOwner>>,
    lease: Mutex<Weak<factory::Lease>>,
    // Private mechanical reads have no factory lease, so this owner has no
    // strong cycle. Cancelled finalizers keep their actual query-drain registry.
    finalization_reader: tokio::sync::Mutex<Option<Arc<dyn crate::reader::RecoveryReadOwner>>>,
    pending: Mutex<PendingBook>,
    _charge: Box<dyn Reservation>,
}
// Every fallible metadata admission completes before an SDK session clone can queue work.
struct PreparedScope {
    budget: ResourceBudget,
    cancellation: RetryCancellation,
    recipient: PendingRequestRecipient,
    target: RecoveryTarget,
    charge: Box<dyn Reservation>,
    entries_charge: Box<dyn Reservation>,
}
impl PreparedScope {
    fn attach(self, client: Arc<Surreal<Client>>) -> Arc<RecoveryScope> {
        Arc::new(RecoveryScope {
            client,
            budget: self.budget,
            cancellation: self.cancellation,
            recipient: self.recipient,
            target: self.target,
            factory: Mutex::new(Weak::new()),
            lease: Mutex::new(Weak::new()),
            finalization_reader: tokio::sync::Mutex::new(None),
            pending: Mutex::new(PendingBook {
                entries: vec![],
                charge: Some(self.entries_charge),
            }),
            _charge: self.charge,
        })
    }
}
impl RecoveryScope {
    pub fn new(
        client: Arc<Surreal<Client>>,
        budget: ResourceBudget,
        cancellation: RetryCancellation,
        recipient: PendingRequestRecipient,
        target: RecoveryTarget,
    ) -> Result<Arc<Self>, ModelError> {
        Ok(Self::prepare_scope(budget, cancellation, recipient, target)?.attach(client))
    }
    fn prepare_scope(
        budget: ResourceBudget,
        cancellation: RetryCancellation,
        recipient: PendingRequestRecipient,
        target: RecoveryTarget,
    ) -> Result<PreparedScope, ModelError> {
        target.validate()?;
        if target.generation != recipient.generation() {
            return Err(ModelError::Conflict("native recovery recipient generation"));
        }
        let charge = budget.reserve(
            "native recovery scope",
            size_of::<Self>()
                + target.heap_bytes()
                + recipient.heap_bytes()
                + 2 * size_of::<usize>()
                + crate::NativeReader::<()>::private_owner_bytes(),
        )?;
        let entries_charge = budget.reserve("native pending recovery index", 0)?;
        Ok(PreparedScope {
            budget,
            cancellation,
            recipient,
            target,
            charge,
            entries_charge,
        })
    }
    pub fn client(&self) -> &Arc<Surreal<Client>> {
        &self.client
    }
    pub fn target(&self) -> &RecoveryTarget {
        &self.target
    }
    pub(crate) async fn join_finalization_reads(&self) -> Result<(), ModelError> {
        let mut reader = self.finalization_reader.lock().await;
        if let Some(owner) = reader.as_ref()
            && let Err(error) = owner.close().await
        {
            if error.permits_storage_cleanup() && owner.terminal()? {
                reader.take();
            }
            return Err(error);
        }
        reader.take();
        Ok(())
    }
    pub(crate) async fn finalization_reader(
        &self,
        admission: &RetryCancellation,
    ) -> Result<crate::NativeReader<()>, ModelError> {
        let mut reader = self.finalization_reader.lock().await;
        if let Some(owner) = reader.as_ref() {
            owner.close().await?;
        }
        reader.take();
        let next =
            crate::reader::recovery_read_owner(self.client.clone(), &self.budget, admission.flag());
        *reader = Some(next.clone());
        Ok(next.reader())
    }
    pub(crate) fn factory(&self) -> Result<Arc<RecoveryFactoryOwner>, ModelError> {
        self.factory
            .lock()
            .map_err(|_| ModelError::Conflict("native scope factory owner"))?
            .upgrade()
            .ok_or(ModelError::Conflict(
                "native scope requires retained factory owner",
            ))
    }
    pub(crate) fn factory_lease(&self) -> Result<FactoryLease, ModelError> {
        self.lease
            .lock()
            .map_err(|_| ModelError::Conflict("native scope factory lease"))?
            .upgrade()
            .map(FactoryLease)
            .ok_or(ModelError::Conflict(
                "native scope requires live factory lease",
            ))
    }
    pub(crate) fn fork_session(
        &self,
        budget: &ResourceBudget,
    ) -> Result<
        (
            Arc<Self>,
            Arc<crate::signal::SignalRegistration>,
            FactoryLease,
        ),
        ModelError,
    > {
        let cancellation = RetryCancellation::default();
        let stopped = cancellation.clone();
        let signals = Arc::new(crate::signal::SignalRegistration::with_receipt(
            "independent native reader",
            move || stopped.cancel(),
            self.recipient.binding()?,
        )?);
        self.fork_session_with_signals(budget, cancellation, signals)
    }
    /// Own an independent effect session beneath this source's existing factory.
    /// The returned lease retains the exact client, resource pool, target and
    /// recovery recipient before SDK attachment. Consumers enter its recovery
    /// scope for effects, drain their readers/borrowers, then complete the lease.
    /// Dropping it transfers unresolved cleanup to the same factory; it never
    /// closes or transfers authority over the borrowed source session.
    pub fn fork_owned_session(&self, budget: &ResourceBudget) -> Result<FactoryLease, ModelError> {
        if self.cancellation.stopped() {
            return Err(ModelError::Conflict(
                "native session source admission stopped",
            ));
        }
        let (_, _, lease) = self.fork_session(budget)?;
        Ok(lease)
    }
    pub(crate) fn fork_session_with_signals(
        &self,
        budget: &ResourceBudget,
        cancellation: RetryCancellation,
        signals: Arc<crate::signal::SignalRegistration>,
    ) -> Result<
        (
            Arc<Self>,
            Arc<crate::signal::SignalRegistration>,
            FactoryLease,
        ),
        ModelError,
    > {
        let factory = self.factory()?;
        factory.fork_session(self, budget, cancellation, signals)
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub fn cancellation(&self) -> &RetryCancellation {
        &self.cancellation
    }
    pub fn pending_count(&self) -> Result<usize, ModelError> {
        Ok(self
            .pending
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery scope"))?
            .entries
            .len())
    }
    pub fn first_pending(self: &Arc<Self>) -> Result<Option<PendingRecovery>, ModelError> {
        Ok(self
            .pending
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery scope"))?
            .entries
            .first()
            .map(|entry| PendingRecovery {
                scope: Arc::downgrade(self),
                entry: entry.clone(),
            }))
    }
    /// Publish before first remote issue, while the real parent still owns this scope.
    pub fn prepare(
        self: &Arc<Self>,
        request: NativeRequest,
        owner: RecoveryOwner,
        outcome_reference: RecordId,
        reference_policy: RecoveryReferencePolicy,
    ) -> Result<PendingRecovery, ModelError> {
        if self.cancellation.stopped() {
            return Err(ModelError::Conflict("native recovery admission stopped"));
        }
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery scope"))?;
        if let Some(entry) = pending
            .entries
            .iter()
            .find(|entry| entry.request.operation == request.operation)
        {
            let mut state = entry
                .state
                .lock()
                .map_err(|_| ModelError::Conflict("native recovery entry"))?;
            if state.envelope.request != request
                || state.envelope.owner != owner
                || state.envelope.outcome_reference != outcome_reference
                || state.envelope.reference_policy != reference_policy
            {
                return Err(ModelError::Conflict("native pending recovery collision"));
            }
            if !state.durable {
                write_envelope(&self.recipient, &self.budget, &state.envelope, true)?;
                state.durable = true;
            }
            return Ok(PendingRecovery {
                scope: Arc::downgrade(self),
                entry: entry.clone(),
            });
        }
        // Admit exact Vec capacity before growth; only outstanding entries remain.
        let length = pending
            .entries
            .len()
            .checked_add(1)
            .ok_or(ModelError::Conflict("native pending request count"))?;
        let capacity = length.max(pending.entries.capacity());
        let index_bytes = capacity
            .checked_mul(size_of::<Arc<Entry>>())
            .ok_or(ModelError::Conflict("native pending request index"))?;
        pending
            .charge
            .as_mut()
            .expect("live recovery index")
            .try_resize(index_bytes)?;
        if length > pending.entries.capacity() {
            pending.entries.reserve_exact(1);
        }
        let actual_bytes = pending
            .entries
            .capacity()
            .checked_mul(size_of::<Arc<Entry>>())
            .ok_or(ModelError::Conflict("native pending request index"))?;
        pending
            .charge
            .as_mut()
            .expect("live recovery index")
            .try_resize(actual_bytes)?;
        // Entry identity and immutable target are admitted before cloning/allocation.
        let bytes = size_of::<Entry>()
            + 2 * size_of::<usize>()
            + self.target.heap_bytes()
            + self.recipient.host_owner().heap_bytes()
            + owner.heap_bytes()?
            + record_bytes(&outcome_reference, "native_outcome_ref")?;
        let charge = self.budget.reserve("native pending recovery", bytes)?;
        let envelope = PendingRecoveryEnvelope {
            schema: 1,
            host_owner: self.recipient.host_owner().clone(),
            request,
            target: self.target.clone(),
            owner,
            outcome_reference,
            reference_policy,
            action: RecoveryAction::ReconcileOriginalRequest,
            observed_disposition: EffectDisposition::Pending,
        };
        envelope.validate(&self.target)?;
        let entry = Arc::new(Entry {
            request,
            state: Mutex::new(PendingState {
                envelope,
                settled: false,
                durable: false,
            }),
            _client: self.client.clone(),
            _budget: self.budget.clone(),
            _cancellation: self.cancellation.clone(),
            _charge: charge,
        });
        pending.entries.push(entry.clone());
        {
            let mut state = entry
                .state
                .lock()
                .map_err(|_| ModelError::Conflict("native recovery entry"))?;
            write_envelope(&self.recipient, &self.budget, &state.envelope, false)?;
            state.durable = true;
        }
        Ok(PendingRecovery {
            scope: Arc::downgrade(self),
            entry,
        })
    }
}
impl Drop for RecoveryScope {
    fn drop(&mut self) {
        // Unexpected source-owner loss is not resource release. Every outstanding
        // entry keeps its actual session and charges; exact durable repair remains.
        // Ordinary finalization settles the map and never takes this path.
        let pending = self
            .pending
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !pending.entries.is_empty() {
            let entries = std::mem::take(&mut pending.entries);
            let charge = pending.charge.take();
            std::mem::forget((entries, charge));
        }
    }
}
#[derive(Clone)]
pub struct PendingRecovery {
    scope: Weak<RecoveryScope>,
    entry: Arc<Entry>,
}
impl PendingRecovery {
    pub fn inspect<T>(
        &self,
        reader: impl FnOnce(&PendingRecoveryEnvelope) -> T,
    ) -> Result<T, ModelError> {
        let state = self
            .entry
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery entry"))?;
        Ok(reader(&state.envelope))
    }
    /// Persist known commit before deleting or transferring its final reference.
    pub fn committed(&self) -> Result<(), ModelError> {
        self.observed(EffectDisposition::Committed)
    }
    /// Both committed and fenced-uncommitted identities must survive last-ref release.
    pub fn observed(&self, disposition: EffectDisposition) -> Result<(), ModelError> {
        if !matches!(
            disposition,
            EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal
        ) {
            return Err(ModelError::Conflict(
                "native recovery terminal disposition absent",
            ));
        }
        let scope = self
            .scope
            .upgrade()
            .ok_or(ModelError::Conflict("native recovery recipient lost"))?;
        let mut state = self
            .entry
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery entry"))?;
        if state.settled {
            return Err(ModelError::Conflict("native recovery already settled"));
        }
        if state.envelope.action == RecoveryAction::ReleaseTerminalReference {
            return if state.envelope.observed_disposition == disposition {
                Ok(())
            } else {
                Err(ModelError::Conflict(
                    "native recovery terminal disposition changed",
                ))
            };
        }
        state.envelope.action = RecoveryAction::ReleaseTerminalReference;
        state.envelope.observed_disposition = disposition;
        if let Err(error) = write_envelope(&scope.recipient, &scope.budget, &state.envelope, true) {
            state.envelope.action = RecoveryAction::ReconcileOriginalRequest;
            state.envelope.observed_disposition = EffectDisposition::Pending;
            return Err(error);
        }
        state.durable = true;
        Ok(())
    }
    /// Caller has checked terminal reconciliation AND exact reference release,
    /// or independently transferred a deliberately retained reference's ownership.
    /// Original command outcomes remain immutable; this settles only an obligation.
    pub fn settle(&self, disposition: EffectDisposition) -> Result<(), ModelError> {
        let scope = self
            .scope
            .upgrade()
            .ok_or(ModelError::Conflict("native recovery recipient lost"))?;
        let mut pending = scope
            .pending
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery scope"))?;
        let mut state = self
            .entry
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery entry"))?;
        if state.settled {
            return Ok(());
        }
        if state.envelope.action != RecoveryAction::ReleaseTerminalReference
            || state.envelope.observed_disposition != disposition
            || !matches!(
                disposition,
                EffectDisposition::Committed
                    | EffectDisposition::FencedUncommitted
                    | EffectDisposition::CompactedTerminal
            )
        {
            return Err(ModelError::Conflict(
                "native recovery settlement not established",
            ));
        }
        scope
            .recipient
            .remove(&state.envelope.request.operation.hex())?;
        let index = pending
            .entries
            .iter()
            .position(|entry| Arc::ptr_eq(entry, &self.entry))
            .ok_or(ModelError::Conflict("native recovery entry correspondence"))?;
        pending.entries.swap_remove(index);
        state.settled = true;
        Ok(())
    }
}
struct Count(usize);
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("native pending envelope overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn write_envelope(
    recipient: &PendingRequestRecipient,
    budget: &ResourceBudget,
    envelope: &PendingRecoveryEnvelope,
    replace: bool,
) -> Result<(), ModelError> {
    write_sidecar(
        recipient,
        budget,
        &envelope.request.operation.hex(),
        envelope,
        replace,
    )
}
fn write_sidecar(
    recipient: &PendingRequestRecipient,
    budget: &ResourceBudget,
    name: &str,
    envelope: &impl Serialize,
    replace: bool,
) -> Result<(), ModelError> {
    let mut length = Count(0);
    serde_json::to_writer(&mut length, envelope).map_err(ModelError::codec)?;
    let _bytes = budget.reserve("native pending envelope encoding", length.0)?;
    let mut bytes = Vec::with_capacity(length.0);
    serde_json::to_writer(&mut bytes, envelope).map_err(ModelError::codec)?;
    recipient.write(name, &bytes, replace)
}
tokio::task_local! { static RECOVERY: Arc<RecoveryScope>; }
pub fn capture_recovery_scope() -> Option<Arc<RecoveryScope>> {
    RECOVERY.try_with(Clone::clone).ok()
}
pub async fn with_recovery_scope<T>(
    scope: Arc<RecoveryScope>,
    future: impl Future<Output = T>,
) -> T {
    with_retry_cancellation(scope.cancellation.clone(), RECOVERY.scope(scope, future)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::SignalRegistration;
    use lctx_model::domain::completion::Completion;
    use std::fs;

    fn target() -> RecoveryTarget {
        RecoveryTarget {
            generation: ContentHash::of(b"owned-generation"),
            namespace: "owned".into(),
            database: "validation".into(),
            endpoint: Some("grpc://127.0.0.1:9000".into()),
        }
    }
    fn request(target: &RecoveryTarget) -> NativeRequest {
        NativeRequest {
            issuance: IssuanceEra {
                generation: target.generation,
                era: 7,
            },
            operation: ContentHash::of(b"original-operation"),
            request: ContentHash::of(b"original-payload-digest"),
        }
    }
    fn reference() -> RecordId {
        super::super::authorization::request_outcome_reference(
            request(&target()),
            RecoveryReferencePolicy::ReleaseRecovery,
        )
        .unwrap()
    }
    fn owner(
        directory: &Path,
        target: &RecoveryTarget,
        cancellation: RetryCancellation,
    ) -> SignalRegistration {
        let receipts = directory.join("native-drain");
        fs::create_dir(&receipts).unwrap();
        SignalRegistration::new_bound(
            "owned recovery control",
            move || cancellation.cancel(),
            receipts,
            "owned-recovery-control".into(),
            target.generation,
        )
        .unwrap()
    }
    fn envelope_path(scope: &RecoveryScope, request: NativeRequest) -> std::path::PathBuf {
        scope
            .recipient
            .directory()
            .join(format!("{}.json", request.operation.hex()))
    }
    #[tokio::test]
    async fn durable_pending_preserves_exact_identity_and_charged_actual_session_until_settled() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let client = Arc::new(Surreal::init());
        let scope = RecoveryScope::new(
            client.clone(),
            budget.clone(),
            cancellation,
            signals.pending_recipient().unwrap(),
            target.clone(),
        )
        .unwrap();
        let baseline = budget.reserved();
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::OpenAttempt(ContentHash::of(b"attempt")),
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        assert_eq!(scope.pending_count().unwrap(), 1);
        assert!(budget.reserved() > baseline);
        assert_eq!(
            Arc::strong_count(&client),
            3,
            "scope and entry retain the actual same session"
        );
        let path = envelope_path(&scope, request);
        let read = read_recovery_envelope(&path, &target, &budget).unwrap();
        assert_eq!(read.envelope().request, request);
        assert_eq!(read.envelope().outcome_reference, reference());
        assert_eq!(
            read.envelope().reference_policy,
            RecoveryReferencePolicy::ReleaseRecovery
        );
        assert_eq!(
            read.envelope().action,
            RecoveryAction::ReconcileOriginalRequest
        );
        let raw: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw.as_object().unwrap().len(), 9);
        assert!(!raw.as_object().unwrap().contains_key("sql"));
        assert!(!raw.as_object().unwrap().contains_key("bindings"));
        assert!(!raw.as_object().unwrap().contains_key("credentials"));
        drop(read);
        drop(pending);
        assert_eq!(
            scope.pending_count().unwrap(),
            1,
            "caller drop does not hand back to nobody"
        );
        assert!(signals.complete(&Completion::default()).await.is_err());
        let pending = scope.first_pending().unwrap().unwrap();
        pending
            .observed(EffectDisposition::FencedUncommitted)
            .unwrap();
        pending
            .settle(EffectDisposition::FencedUncommitted)
            .unwrap();
        pending
            .settle(EffectDisposition::FencedUncommitted)
            .unwrap();
        assert_eq!(scope.pending_count().unwrap(), 0);
        assert!(!path.exists());
        drop(pending);
        assert!(budget.reserved() >= baseline);
        drop(scope);
        assert_eq!(budget.reserved(), 0);
        assert_eq!(Arc::strong_count(&client), 1);
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn committed_release_action_is_durable_before_reference_settlement() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget.clone(),
            cancellation,
            signals.pending_recipient().unwrap(),
            target.clone(),
        )
        .unwrap();
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::Installation,
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        assert!(pending.settle(EffectDisposition::Committed).is_err());
        assert!(pending.settle(EffectDisposition::Pending).is_err());
        pending.committed().unwrap();
        pending.committed().unwrap();
        let recorded =
            read_recovery_envelope(&envelope_path(&scope, request), &target, &budget).unwrap();
        assert_eq!(
            recorded.envelope().action,
            RecoveryAction::ReleaseTerminalReference
        );
        assert_eq!(
            recorded.envelope().observed_disposition,
            EffectDisposition::Committed
        );
        assert!(
            pending
                .settle(EffectDisposition::FencedUncommitted)
                .is_err()
        );
        drop(recorded);
        pending.settle(EffectDisposition::Committed).unwrap();
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn failed_commit_envelope_publication_retains_reconcile_and_allows_exact_retry() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget,
            cancellation,
            signals.pending_recipient().unwrap(),
            target,
        )
        .unwrap();
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::Installation,
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        let path = envelope_path(&scope, request);
        let saved = path.with_extension("saved");
        fs::rename(&path, &saved).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(pending.committed().is_err());
        assert_eq!(
            pending.inspect(|envelope| envelope.action).unwrap(),
            RecoveryAction::ReconcileOriginalRequest
        );
        assert_eq!(scope.pending_count().unwrap(), 1);
        assert!(pending.settle(EffectDisposition::Committed).is_err());
        fs::remove_dir(&path).unwrap();
        fs::rename(saved, &path).unwrap();
        pending.committed().unwrap();
        pending.settle(EffectDisposition::Committed).unwrap();
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn stopped_owner_refuses_new_preparation_without_discarding_existing_request() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget,
            cancellation.clone(),
            signals.pending_recipient().unwrap(),
            target,
        )
        .unwrap();
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::Installation,
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        let path = envelope_path(&scope, request);
        cancellation.cancel();
        assert!(
            scope
                .prepare(
                    NativeRequest {
                        operation: ContentHash::of(b"another"),
                        ..request
                    },
                    RecoveryOwner::Installation,
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        assert!(path.exists());
        assert_eq!(scope.pending_count().unwrap(), 1);
        pending
            .observed(EffectDisposition::FencedUncommitted)
            .unwrap();
        pending
            .settle(EffectDisposition::FencedUncommitted)
            .unwrap();
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn metadata_budget_and_persistence_refuse_before_any_remote_issue() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let recipient = signals.pending_recipient().unwrap();
        let budget = ResourceBudget::fixed(
            size_of::<RecoveryScope>()
                + target.heap_bytes()
                + recipient.heap_bytes()
                + 2 * size_of::<usize>()
                + crate::NativeReader::<()>::private_owner_bytes()
                + 8,
        )
        .unwrap();
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget.clone(),
            cancellation,
            recipient.clone(),
            target,
        )
        .unwrap();
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        assert_eq!(scope.pending_count().unwrap(), 0);
        assert!(!recipient.directory().exists());
        drop(scope);
        assert_eq!(budget.reserved(), 0);
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn failed_initial_persistence_retains_actual_recipient_and_requires_durable_retry() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let recipient = signals.pending_recipient().unwrap();
        fs::create_dir(recipient.directory().parent().unwrap()).unwrap();
        fs::write(recipient.directory(), b"owned persistence refusal").unwrap();
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            ResourceBudget::fixed(1 << 20).unwrap(),
            cancellation,
            recipient.clone(),
            target,
        )
        .unwrap();
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        assert_eq!(
            scope.pending_count().unwrap(),
            1,
            "failed persistence retains its actual cleanup recipient"
        );
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        fs::remove_file(recipient.directory()).unwrap();
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::Installation,
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        assert!(
            envelope_path(&scope, request).exists(),
            "retry cannot issue before durable persistence succeeds"
        );
        pending
            .observed(EffectDisposition::FencedUncommitted)
            .unwrap();
        pending
            .settle(EffectDisposition::FencedUncommitted)
            .unwrap();
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn collisions_and_foreign_target_envelopes_never_gain_recovery_authority() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let request = request(&target);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget.clone(),
            cancellation,
            signals.pending_recipient().unwrap(),
            target.clone(),
        )
        .unwrap();
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    RecordId::new("native_outcome_ref", "foreign-reference"),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        assert_eq!(
            scope.pending_count().unwrap(),
            0,
            "a wrong native reference cannot establish a pending recipient"
        );
        let caller = super::super::authorization::request_outcome_reference(
            request,
            RecoveryReferencePolicy::RetainCaller,
        )
        .unwrap();
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    caller,
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err(),
            "reference kind is part of exact identity"
        );
        let pending = scope
            .prepare(
                request,
                RecoveryOwner::Installation,
                reference(),
                RecoveryReferencePolicy::ReleaseRecovery,
            )
            .unwrap();
        let mut invalid = pending.inspect(Clone::clone).unwrap();
        invalid.outcome_reference = RecordId::new("native_outcome_ref", "foreign-reference");
        assert!(invalid.validate(&target).is_err());
        assert!(
            scope
                .prepare(
                    NativeRequest {
                        request: ContentHash::of(b"changed"),
                        ..request
                    },
                    RecoveryOwner::Installation,
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        assert!(
            scope
                .prepare(
                    request,
                    RecoveryOwner::OpenAttempt(ContentHash::of(b"other")),
                    reference(),
                    RecoveryReferencePolicy::ReleaseRecovery
                )
                .is_err()
        );
        let path = envelope_path(&scope, request);
        assert!(
            read_recovery_envelope(
                &path,
                &RecoveryTarget {
                    database: "foreign".into(),
                    ..target.clone()
                },
                &budget
            )
            .is_err()
        );
        assert!(
            read_recovery_envelope(
                &path,
                &RecoveryTarget {
                    generation: ContentHash::of(b"foreign"),
                    ..target.clone()
                },
                &budget
            )
            .is_err()
        );
        assert!(
            read_recovery_envelope(
                &path,
                &RecoveryTarget {
                    endpoint: Some("grpc://other:9000".into()),
                    ..target
                },
                &budget
            )
            .is_err()
        );
        pending
            .observed(EffectDisposition::FencedUncommitted)
            .unwrap();
        pending
            .settle(EffectDisposition::FencedUncommitted)
            .unwrap();
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[tokio::test]
    async fn task_context_retains_actual_scope_without_a_strong_entry_cycle() {
        let directory = tempfile::tempdir().unwrap();
        let target = target();
        let cancellation = RetryCancellation::default();
        let signals = owner(directory.path(), &target, cancellation.clone());
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            ResourceBudget::fixed(1 << 20).unwrap(),
            cancellation,
            signals.pending_recipient().unwrap(),
            target,
        )
        .unwrap();
        assert!(capture_recovery_scope().is_none());
        with_recovery_scope(scope.clone(), async {
            let captured = capture_recovery_scope().unwrap();
            assert!(Arc::ptr_eq(&scope, &captured));
        })
        .await;
        assert_eq!(Arc::strong_count(&scope), 1);
        signals.complete(&Completion::default()).await.unwrap();
    }
    #[test]
    #[ignore = "invoked only by the exact pending-envelope host-loss control"]
    fn pending_envelope_host_loss_child() {
        let directory =
            std::path::PathBuf::from(std::env::var_os("LCTX_PENDING_ENVELOPE_CONTROL").unwrap());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let target = target();
                let cancellation = RetryCancellation::default();
                let signals = owner(&directory, &target, cancellation.clone());
                let scope = RecoveryScope::new(
                    Arc::new(Surreal::init()),
                    ResourceBudget::fixed(1 << 20).unwrap(),
                    cancellation,
                    signals.pending_recipient().unwrap(),
                    target.clone(),
                )
                .unwrap();
                scope
                    .prepare(
                        request(&target),
                        RecoveryOwner::Installation,
                        reference(),
                        RecoveryReferencePolicy::ReleaseRecovery,
                    )
                    .unwrap()
                    .committed()
                    .unwrap();
                // Deliberately skip destructors: persisted request survives process loss.
                std::process::exit(0);
            });
    }
    #[test]
    fn exact_release_only_envelope_survives_actual_host_process_loss() {
        let directory = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "control::recovery::tests::pending_envelope_host_loss_child",
                "--ignored",
            ])
            .env("LCTX_PENDING_ENVELOPE_CONTROL", directory.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let records = fs::read_dir(directory.path().join("native-drain"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            records.len(),
            1,
            "pending files remain outside native-drain's receipt scan"
        );
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(records[0].path()).unwrap()).unwrap();
        assert_eq!(
            original["state"], "started",
            "host loss never upgrades the original receipt"
        );
        assert!(original["completion"].is_null());
        let target = target();
        let path = std::path::PathBuf::from(original["pending_requests"].as_str().unwrap())
            .join(format!("{}.json", request(&target).operation.hex()));
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let recovered = read_recovery_envelope(&path, &target, &budget).unwrap();
        assert_eq!(recovered.envelope().request, request(&target));
        assert_eq!(
            recovered.envelope().observed_disposition,
            EffectDisposition::Committed
        );
        assert_eq!(
            recovered.envelope().action,
            RecoveryAction::ReleaseTerminalReference
        );
        assert_eq!(recovered.envelope().outcome_reference, reference());
        drop(recovered);
        assert_eq!(budget.reserved(), 0);
        // This control has no native queries; its entirely owned sidecar has no remote borrower.
        fs::remove_file(path).unwrap();
        fs::remove_dir_all(directory.path().join("native-drain.requests")).unwrap();
    }
}
