//! A concrete caller-owned recipient exists before asynchronous source construction.
use super::*;
use crate::{
    reader::SessionFinalizationGuard,
    signal::{NativeReceiptBinding, SignalRegistration},
};
use std::sync::atomic::{AtomicBool, Ordering};

struct Child {
    scope: Arc<RecoveryScope>,
    signals: Arc<SignalRegistration>,
    session: SessionFinalizationGuard,
    borrowed: AtomicBool,
    settling: tokio::sync::Mutex<()>,
    marker: Mutex<Option<OwnedMarker>>,
    pins: Mutex<PinObligations>,
    successor: Mutex<Option<Arc<RecoveryScope>>>,
    _charge: Box<dyn Reservation>,
}
struct ChildAdmission {
    charge: Box<dyn Reservation>,
    pin_index: Box<dyn Reservation>,
}
struct Children {
    entries: Vec<Arc<Child>>,
    charge: Option<Box<dyn Reservation>>,
}
/// Retain before await and drain in the caller's existing finalization path.
/// Neither configuration nor a completion/error report substitutes for this owner.
pub struct RecoveryFactoryOwner {
    binding: NativeReceiptBinding,
    budget: ResourceBudget,
    children: Mutex<Children>,
    _charge: Box<dyn Reservation>,
}
impl RecoveryFactoryOwner {
    pub fn new(
        binding: NativeReceiptBinding,
        budget: ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let charge = budget.reserve(
            "native recovery factory",
            size_of::<Self>()
                + 2 * size_of::<usize>()
                + binding.directory.capacity()
                + binding.attachment.capacity(),
        )?;
        let children_charge = budget.reserve("native recovery factory children", 0)?;
        Ok(Arc::new(Self {
            binding,
            budget,
            children: Mutex::new(Children {
                entries: vec![],
                charge: Some(children_charge),
            }),
            _charge: charge,
        }))
    }
    pub fn binding(&self) -> &NativeReceiptBinding {
        &self.binding
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub fn pending_count(&self) -> Result<usize, ModelError> {
        Ok(self
            .children
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery factory"))?
            .entries
            .len())
    }
    /// Attach the actual session before the first asynchronous/native issue.
    pub fn attach(
        self: &Arc<Self>,
        scope: Arc<RecoveryScope>,
        signals: Arc<SignalRegistration>,
    ) -> Result<FactoryLease, ModelError> {
        if !self.budget.shares_pool(scope.budget())
            || self.binding.generation != scope.target.generation
            || signals.pending_recipient()?.host_owner() != scope.recipient.host_owner()
        {
            return Err(ModelError::Conflict(
                "native factory child budget/target differs",
            ));
        }
        let mut children = self
            .children
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery factory"))?;
        if children
            .entries
            .iter()
            .any(|child| Arc::ptr_eq(&child.scope, &scope))
        {
            return Err(ModelError::Conflict("native factory duplicate child"));
        }
        let slots = scope.clone();
        let mut parent = slots
            .factory
            .lock()
            .map_err(|_| ModelError::Conflict("native scope factory owner"))?;
        let mut weak_lease = slots
            .lease
            .lock()
            .map_err(|_| ModelError::Conflict("native scope factory lease"))?;
        if parent.upgrade().is_some() || weak_lease.upgrade().is_some() {
            return Err(ModelError::Conflict(
                "native scope already has a factory owner",
            ));
        }
        let admitted = self.admit_child(&mut children)?;
        let lease = self.insert_child(&mut children, scope, signals, admitted);
        *parent = Arc::downgrade(self);
        *weak_lease = Arc::downgrade(&lease.0);
        Ok(lease)
    }
    fn admit_child(&self, children: &mut Children) -> Result<ChildAdmission, ModelError> {
        let capacity = children
            .entries
            .len()
            .checked_add(1)
            .ok_or(ModelError::Conflict("native factory child count"))?
            .max(children.entries.capacity());
        let bytes = capacity
            .checked_mul(size_of::<Arc<Child>>())
            .ok_or(ModelError::Conflict("native factory child index"))?;
        children
            .charge
            .as_mut()
            .expect("live factory index")
            .try_resize(bytes)?;
        if children.entries.len() == children.entries.capacity() {
            children.entries.reserve_exact(1);
        }
        let bytes = children
            .entries
            .capacity()
            .checked_mul(size_of::<Arc<Child>>())
            .ok_or(ModelError::Conflict("native factory child index"))?;
        children
            .charge
            .as_mut()
            .expect("live factory index")
            .try_resize(bytes)?;
        Ok(ChildAdmission {
            charge: self.budget.reserve(
                "native factory child",
                size_of::<Child>() + size_of::<Lease>() + 4 * size_of::<usize>(),
            )?,
            pin_index: self.budget.reserve("native factory pin index", 0)?,
        })
    }
    fn insert_child(
        self: &Arc<Self>,
        children: &mut Children,
        scope: Arc<RecoveryScope>,
        signals: Arc<SignalRegistration>,
        admitted: ChildAdmission,
    ) -> FactoryLease {
        let child = Arc::new(Child {
            session: SessionFinalizationGuard::new(scope.client.clone()),
            scope,
            signals,
            borrowed: AtomicBool::new(true),
            settling: tokio::sync::Mutex::new(()),
            marker: Mutex::new(None),
            pins: Mutex::new(PinObligations {
                entries: vec![],
                charge: admitted.pin_index,
            }),
            successor: Mutex::new(None),
            _charge: admitted.charge,
        });
        children.entries.push(child.clone());
        FactoryLease(Arc::new(Lease {
            factory: self.clone(),
            child,
        }))
    }
    pub(super) fn fork_session(
        self: &Arc<Self>,
        parent: &RecoveryScope,
        budget: &ResourceBudget,
        cancellation: RetryCancellation,
        signals: Arc<SignalRegistration>,
    ) -> Result<(Arc<RecoveryScope>, Arc<SignalRegistration>, FactoryLease), ModelError> {
        if self.binding.generation != parent.target.generation {
            return Err(ModelError::Conflict(
                "independent session factory target differs",
            ));
        }
        // The factory charges its child/index metadata on its retained pool;
        // this independently owned scope charges the exact caller-supplied pool.
        // Neither allocation replaces the original parent's live admission owner.
        let prepared = RecoveryScope::prepare_scope(
            budget.clone(),
            cancellation,
            signals.pending_recipient()?,
            parent.target.clone(),
        )?;
        let mut children = self
            .children
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery factory"))?;
        if !children.entries.iter().any(|child| std::ptr::eq(child.scope.as_ref(), parent)) {
            return Err(ModelError::Conflict("independent session source is not an actual factory child"));
        }
        let admitted = self.admit_child(&mut children)?;
        // SDK clone queues session attachment. Everything below is infallible
        // local construction, with actual resources installed before any await.
        let client = Arc::new(parent.client.as_ref().clone());
        let scope = prepared.attach(client);
        let lease = self.insert_child(&mut children, scope.clone(), signals.clone(), admitted);
        *scope.factory.lock().expect("new scope factory slot") = Arc::downgrade(self);
        *scope.lease.lock().expect("new scope lease slot") = Arc::downgrade(&lease.0);
        Ok((scope, signals, lease))
    }
    fn remove(&self, child: &Arc<Child>) -> Result<(), ModelError> {
        let mut children = self
            .children
            .lock()
            .map_err(|_| ModelError::Conflict("native recovery factory"))?;
        if let Some(index) = children
            .entries
            .iter()
            .position(|entry| Arc::ptr_eq(entry, child))
        {
            children.entries.swap_remove(index);
        }
        Ok(())
    }
    /// Resume abandoned construction under an explicit independent issuance policy.
    /// Live source/reader leases cannot be stolen or closed by this route.
    pub async fn drain(self: &Arc<Self>, admission: &RetryCancellation) -> Result<(), ModelError> {
        self.drain_inner(admission)
            .await
            .map_err(|error| self.handback(error))
    }
    async fn drain_inner(
        self: &Arc<Self>,
        admission: &RetryCancellation,
    ) -> Result<(), ModelError> {
        loop {
            let child = {
                let children = self
                    .children
                    .lock()
                    .map_err(|_| ModelError::Conflict("native recovery factory"))?;
                match children
                    .entries
                    .iter()
                    .find(|child| !child.borrowed.load(Ordering::Acquire))
                {
                    Some(child) => child.clone(),
                    None if children.entries.is_empty() => return Ok(()),
                    None => return Err(outstanding("native factory still has live source owners")),
                }
            };
            let _settling = child.settling.lock().await;
            if admission.stopped() {
                return Err(outstanding("native factory recovery admission stopped"));
            }
            super::super::authorization::resume_recovery_scope_admitted(&child.scope, admission)
                .await?;
            child.finalize_pin(admission, true)?.await?;
            child.finalize_source(admission, true)?.await?;
            if admission.stopped() {
                return Err(outstanding(
                    "native factory recovery admission stopped before close",
                ));
            }
            child.session.close().await?;
            child
                .signals
                .complete(&lctx_model::domain::completion::Completion::default())
                .await?;
            self.remove(&child)?;
        }
    }
}
fn outstanding(message: &'static str) -> ModelError {
    let completion = lctx_model::domain::completion::Completion {
        local: lctx_model::domain::completion::LocalState::Outstanding,
        ..Default::default()
    };
    lctx_model::domain::completion::complete::<()>(Err(ModelError::Conflict(message)), completion)
        .unwrap_err()
}
impl Drop for RecoveryFactoryOwner {
    fn drop(&mut self) {
        let children = self
            .children
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !children.entries.is_empty() {
            // Caller loss is not native cleanup. Retain actual sessions, receipts
            // and charges; process loss leaves the exact durable repair route.
            std::mem::forget((
                std::mem::take(&mut children.entries),
                children.charge.take(),
            ));
        }
    }
}
pub(super) struct Lease {
    factory: Arc<RecoveryFactoryOwner>,
    child: Arc<Child>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.child.borrowed.store(false, Ordering::Release);
    }
}
/// A successful constructor transfers this into its actual finalization owner.
#[derive(Clone)]
pub struct FactoryLease(pub(super) Arc<Lease>);
impl FactoryLease {
    pub fn recovery_scope(&self) -> &Arc<RecoveryScope> {
        &self.0.child.scope
    }
    pub fn signals(&self) -> Arc<SignalRegistration> {
        self.0.child.signals.clone()
    }
    pub fn factory(&self) -> &Arc<RecoveryFactoryOwner> {
        &self.0.factory
    }
    pub async fn close_session(&self) -> Result<(), ModelError> {
        let _settling = self.0.child.settling.lock().await;
        self.close_session_locked()
            .await
            .map_err(|error| self.0.factory.handback(error))
    }
    async fn close_session_locked(&self) -> Result<(), ModelError> {
        self.close_source_and_session()?.await
    }
    fn close_source_and_session(
        &self,
    ) -> Result<futures::future::BoxFuture<'_, Result<(), ModelError>>, ModelError> {
        let closing = async move {
            self.0.child.scope.join_finalization_reads().await?;
            if self.0.child.scope.pending_count()? != 0 {
                return Err(outstanding(
                    "native factory source retains pending requests",
                ));
            }
            self.0
                .child
                .finalize_pin(self.0.child.scope.cancellation(), false)?
                .await?;
            self.0
                .child
                .finalize_source(self.0.child.scope.cancellation(), false)?
                .await?;
            self.0.child.session.close().await
        };
        // Charge the body, its owned reservation and alignment/discriminant
        // padding before allocating the single erased logical continuation.
        crate::continuation::admit(
            &self.0.child.scope.budget,
            "native factory close continuation",
            closing,
        )
    }
    /// Source-specific query/borrower drainage has already been established.
    pub async fn complete(&self) -> Result<(), ModelError> {
        self.complete_with(&lctx_model::domain::completion::Completion::default())
            .await
    }
    pub async fn complete_with(
        &self,
        completion: &lctx_model::domain::completion::Completion,
    ) -> Result<(), ModelError> {
        self.complete_with_inner(completion)
            .await
            .map_err(|error| self.0.factory.handback(error))
    }
    async fn complete_with_inner(
        &self,
        completion: &lctx_model::domain::completion::Completion,
    ) -> Result<(), ModelError> {
        let _settling = self.0.child.settling.lock().await;
        self.close_session_locked().await?;
        self.0.child.signals.complete(completion).await?;
        self.0.child.borrowed.store(false, Ordering::Release);
        self.0.factory.remove(&self.0.child)
    }
}

/// A bounded parsed source obligation. Parsing grants no maintenance authority.
pub struct ReadSourceFinalizationMarker {
    marker: SourceFinalizationMarker,
    recipient: PendingRequestRecipient,
    budget: ResourceBudget,
    settled: bool,
    _charge: Box<dyn Reservation>,
}
impl ReadSourceFinalizationMarker {
    pub fn marker(&self) -> &SourceFinalizationMarker {
        &self.marker
    }
    /// Caller authenticates the exact target and proves the original host dead
    /// under its receipt lock. Persist before releasing either lifecycle ref.
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
                "offline source terminal disposition absent",
            ));
        }
        if self.marker.phase == SourceFinalizationPhase::SourceTerminal {
            return if self.marker.allocation_disposition == disposition {
                Ok(())
            } else {
                Err(ModelError::Conflict(
                    "offline source terminal disposition changed",
                ))
            };
        }
        if self.marker.phase == SourceFinalizationPhase::SourceActive
            && disposition != EffectDisposition::Committed
        {
            return Err(ModelError::Conflict(
                "known committed allocation disposition changed",
            ));
        }
        let old_phase = self.marker.phase;
        let old_disposition = self.marker.allocation_disposition;
        self.marker.phase = SourceFinalizationPhase::SourceTerminal;
        self.marker.allocation_disposition = disposition;
        if let Err(error) = write_sidecar(
            &self.recipient,
            &self.budget,
            &self.marker.identity().hex(),
            &self.marker,
            true,
        ) {
            self.marker.phase = old_phase;
            self.marker.allocation_disposition = old_disposition;
            return Err(error);
        }
        Ok(())
    }
    /// Caller separately established source terminality, both exact reference
    /// releases and drainage of any finalization requests. Owner receipt stays unchanged.
    pub fn settle(&mut self) -> Result<(), ModelError> {
        if self.settled {
            return Ok(());
        }
        if self.marker.phase != SourceFinalizationPhase::SourceTerminal {
            return Err(ModelError::Conflict(
                "offline source settlement not established",
            ));
        }
        self.recipient.remove(&self.marker.identity().hex())?;
        self.settled = true;
        Ok(())
    }
}
pub fn read_source_finalization_marker(
    path: &Path,
    expected: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<ReadSourceFinalizationMarker, ModelError> {
    if !path.is_absolute() || path.canonicalize().map_err(ModelError::codec)? != path {
        return Err(ModelError::Conflict("native source marker redirected"));
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
        .and_then(|bytes| bytes.checked_add(size_of::<ReadSourceFinalizationMarker>()))
        .ok_or(ModelError::Conflict("native source marker size"))?;
    let charge = budget.reserve("native source marker read", bytes)?;
    let mut encoded = vec![0; length];
    file.read_exact(&mut encoded).map_err(ModelError::codec)?;
    if file.read(&mut [0]).map_err(ModelError::codec)? != 0 {
        return Err(ModelError::Conflict(
            "native source marker changed while reading",
        ));
    }
    let marker: SourceFinalizationMarker =
        serde_json::from_slice(&encoded).map_err(ModelError::codec)?;
    marker.validate(expected)?;
    let receipt = &marker.host_owner.receipt;
    let parent = receipt
        .parent()
        .ok_or(ModelError::Conflict("native source owner receipt parent"))?;
    let stem = receipt
        .file_stem()
        .ok_or(ModelError::Conflict("native source owner receipt name"))?;
    if path.parent() != Some(parent.with_extension("requests").join(stem).as_path())
        || path.file_name().and_then(|name| name.to_str())
            != Some(format!("{}.json", marker.identity().hex()).as_str())
    {
        return Err(ModelError::Conflict(
            "native source marker host/path correspondence",
        ));
    }
    let recipient =
        PendingRequestRecipient::for_recovery(marker.host_owner.clone(), marker.target.generation)?;
    Ok(ReadSourceFinalizationMarker {
        marker,
        recipient,
        budget: budget.clone(),
        settled: false,
        _charge: charge,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMarkerKind {
    SourceFinalization,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceFinalizationPhase {
    AllocationPending,
    SourceActive,
    SourceTerminal,
}
fn phase_order(phase: SourceFinalizationPhase) -> u8 {
    match phase {
        SourceFinalizationPhase::AllocationPending => 0,
        SourceFinalizationPhase::SourceActive => 1,
        SourceFinalizationPhase::SourceTerminal => 2,
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFinalizationMarker {
    pub schema: u8,
    pub kind: SourceMarkerKind,
    pub host_owner: crate::signal::PendingRequestHostOwner,
    pub target: RecoveryTarget,
    pub attempt: ContentHash,
    pub allocation: NativeRequest,
    pub original_epoch: i64,
    pub phase: SourceFinalizationPhase,
    pub allocation_disposition: EffectDisposition,
}
impl SourceFinalizationMarker {
    pub fn identity(&self) -> ContentHash {
        let mut sink = KeySink::new("native-source-finalization/v1");
        self.attempt.encode(&mut sink);
        self.allocation.operation.encode(&mut sink);
        self.allocation.issuance.generation.encode(&mut sink);
        sink.finish()
    }
    pub fn validate(&self, expected: &RecoveryTarget) -> Result<(), ModelError> {
        self.target.validate()?;
        if self.schema != 1
            || self.target.generation != expected.generation
            || self.target.namespace != expected.namespace
            || self.target.database != expected.database
            || self
                .target
                .endpoint
                .as_ref()
                .is_some_and(|endpoint| expected.endpoint.as_ref() != Some(endpoint))
            || self.allocation.issuance.generation != expected.generation
            || self.allocation.issuance.era <= 0
            || self.original_epoch != 1
            || !self.host_owner.receipt.is_absolute()
            || self.host_owner.pid == 0
            || self.host_owner.pid_namespace == 0
            || self.host_owner.boot_id.is_empty()
        {
            return Err(ModelError::Conflict(
                "native source finalization correspondence",
            ));
        }
        match (self.phase, self.allocation_disposition) {
            (SourceFinalizationPhase::AllocationPending, EffectDisposition::Pending)
            | (SourceFinalizationPhase::SourceActive, EffectDisposition::Committed)
            | (
                SourceFinalizationPhase::SourceTerminal,
                EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal,
            ) => Ok(()),
            _ => Err(ModelError::Conflict(
                "native source finalization phase/disposition",
            )),
        }
    }
}
#[derive(Clone)]
struct OwnedMarker {
    envelope: Arc<SourceFinalizationMarker>,
    _charge: Arc<Box<dyn Reservation>>,
}
fn marker_charge(
    budget: &ResourceBudget,
    scope: &RecoveryScope,
) -> Result<Box<dyn Reservation>, ModelError> {
    budget.reserve(
        "native source finalization marker",
        size_of::<SourceFinalizationMarker>()
            + scope.target.heap_bytes()
            + scope.recipient.host_owner().heap_bytes()
            + 6 * size_of::<usize>(),
    )
}
impl Child {
    fn marker(&self) -> Result<Option<OwnedMarker>, ModelError> {
        Ok(self
            .marker
            .lock()
            .map_err(|_| ModelError::Conflict("native source finalization marker"))?
            .clone())
    }
    fn write_marker(
        &self,
        marker: SourceFinalizationMarker,
        charge: Box<dyn Reservation>,
        replace: bool,
    ) -> Result<(), ModelError> {
        marker.validate(&self.scope.target)?;
        let mut registered = self
            .marker
            .lock()
            .map_err(|_| ModelError::Conflict("native source finalization marker"))?;
        match registered.as_ref() {
            Some(current)
                if replace
                    && current.envelope.identity() == marker.identity()
                    && phase_order(marker.phase) >= phase_order(current.envelope.phase) => {}
            None if !replace => {}
            _ => {
                return Err(ModelError::Conflict(
                    "native source marker changed registration or phase",
                ));
            }
        }
        if registered
            .as_ref()
            .is_some_and(|current| current.envelope.phase == SourceFinalizationPhase::SourceActive)
            && marker.allocation_disposition != EffectDisposition::Committed
        {
            return Err(ModelError::Conflict(
                "known source allocation disposition changed",
            ));
        }
        if registered.as_ref().is_some_and(|current| {
            current.envelope.phase == SourceFinalizationPhase::SourceTerminal
                && current.envelope.allocation_disposition != marker.allocation_disposition
        }) {
            return Err(ModelError::Conflict(
                "terminal source allocation disposition changed",
            ));
        }
        let marker = OwnedMarker {
            envelope: Arc::new(marker),
            _charge: Arc::new(charge),
        };
        let written = write_sidecar(
            &self.scope.recipient,
            &self.scope.budget,
            &marker.envelope.identity().hex(),
            &marker.envelope,
            replace,
        );
        // Initial write failure never permits allocation, but may have installed
        // a sidecar before fsync failed. Keep its actual live cleanup recipient.
        if written.is_ok() || !replace {
            *registered = Some(marker);
        }
        written
    }
    fn finalize_source<'a>(
        &'a self,
        admission: &'a RetryCancellation,
        abandon: bool,
    ) -> Result<futures::future::BoxFuture<'a, Result<(), ModelError>>, ModelError> {
        let body = async move {
            let Some(marker) = self.marker()? else {
                return Ok(());
            };
            if admission.stopped() {
                return Err(outstanding("native source finalization admission stopped"));
            }
            let old = self
                .successor
                .lock()
                .map_err(|_| ModelError::Conflict("native recovery successor"))?
                .clone();
            if let Some(old) = old {
                super::super::authorization::resume_recovery_scope_admitted(&old, admission)
                    .await?;
                if old.pending_count()? != 0 {
                    return Err(outstanding("native recovery successor still pending"));
                }
            }
            let successor = RecoveryScope::new(
                self.scope.client.clone(),
                self.scope.budget.clone(),
                admission.clone(),
                self.scope.recipient.clone(),
                self.scope.target.clone(),
            )?;
            *self
                .successor
                .lock()
                .map_err(|_| ModelError::Conflict("native recovery successor"))? =
                Some(successor.clone());
            if marker.envelope.phase != SourceFinalizationPhase::SourceTerminal {
                let disposition = super::super::authorization::finalize_factory_source_admitted(
                    &successor,
                    &marker.envelope,
                    admission,
                    abandon,
                )
                .await?;
                let charge = marker_charge(&self.scope.budget, &self.scope)?;
                let mut terminal = marker.envelope.as_ref().clone();
                terminal.phase = SourceFinalizationPhase::SourceTerminal;
                terminal.allocation_disposition = disposition;
                // Persist terminal source/allocation knowledge before either last ref can disappear.
                self.write_marker(terminal, charge, true)?;
            }
            let terminal = self.marker()?.ok_or(ModelError::Conflict(
                "native source finalization marker lost",
            ))?;
            super::super::authorization::release_factory_source_references_admitted(
                &successor,
                &terminal.envelope,
                admission,
            )
            .await?;
            super::super::authorization::resume_recovery_scope_admitted(&successor, admission)
                .await?;
            if successor.pending_count()? != 0 {
                return Err(outstanding(
                    "native source finalization effects still pending",
                ));
            }
            self.scope
                .recipient
                .remove(&terminal.envelope.identity().hex())?;
            self.marker
                .lock()
                .map_err(|_| ModelError::Conflict("native source finalization marker"))?
                .take();
            self.successor
                .lock()
                .map_err(|_| ModelError::Conflict("native recovery successor"))?
                .take();
            Ok(())
        };
        crate::continuation::admit(
            &self.scope.budget,
            "native finalize_source continuation",
            body,
        )
    }
}
impl FactoryLease {
    /// Allocation has a real retained caller outcome and attempt lifecycle reference.
    /// This marker is fsynced BEFORE issuing its exact prepared NativeRequest.
    pub fn register_attempt(
        &self,
        attempt: ContentHash,
        allocation: NativeRequest,
    ) -> Result<(), ModelError> {
        if self.0.child.marker()?.is_some() {
            return Err(ModelError::Conflict(
                "native source finalization already registered",
            ));
        }
        let scope = &self.0.child.scope;
        let charge = marker_charge(&scope.budget, scope)?;
        self.0.child.write_marker(
            SourceFinalizationMarker {
                schema: 1,
                kind: SourceMarkerKind::SourceFinalization,
                host_owner: scope.recipient.host_owner().clone(),
                target: scope.target.clone(),
                attempt,
                allocation,
                original_epoch: 1,
                phase: SourceFinalizationPhase::AllocationPending,
                allocation_disposition: EffectDisposition::Pending,
            },
            charge,
            false,
        )
    }
    /// Constructor has checked allocation success before handing out the source.
    pub fn source_ready(&self) -> Result<(), ModelError> {
        let marker = self.0.child.marker()?.ok_or(ModelError::Conflict(
            "native source allocation marker missing",
        ))?;
        if marker.envelope.phase == SourceFinalizationPhase::SourceActive {
            return Ok(());
        }
        if marker.envelope.phase != SourceFinalizationPhase::AllocationPending {
            return Err(ModelError::Conflict("native source allocation phase"));
        }
        let charge = marker_charge(&self.0.child.scope.budget, &self.0.child.scope)?;
        let mut active = marker.envelope.as_ref().clone();
        active.phase = SourceFinalizationPhase::SourceActive;
        active.allocation_disposition = EffectDisposition::Committed;
        self.0.child.write_marker(active, charge, true)
    }
}
/// An actual callable owner accompanies protected handback, rather than only a
/// descriptive Completion. Never store this in its own child/factory (cycle).
pub struct FactoryHandback {
    factory: Arc<RecoveryFactoryOwner>,
    primary: ModelError,
}
impl FactoryHandback {
    pub fn factory(&self) -> &Arc<RecoveryFactoryOwner> {
        &self.factory
    }
    pub async fn resume(&self, admission: &RetryCancellation) -> Result<(), ModelError> {
        self.factory.drain(admission).await
    }
}
impl std::fmt::Debug for FactoryHandback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FactoryHandback")
            .field("primary", &self.primary)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Display for FactoryHandback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "native source owner retained for recovery: {}",
            self.primary
        )
    }
}
impl std::error::Error for FactoryHandback {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.primary)
    }
}
impl RecoveryFactoryOwner {
    pub fn handback(self: &Arc<Self>, error: ModelError) -> ModelError {
        ModelError::Cause(Box::new(FactoryHandback {
            factory: self.clone(),
            primary: error,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> (tempfile::TempDir, Arc<RecoveryFactoryOwner>, FactoryLease) {
        let root = tempfile::tempdir().unwrap();
        let receipts = root.path().join("native-drain");
        std::fs::create_dir(&receipts).unwrap();
        let generation = ContentHash::of(b"factory pure control");
        let binding = NativeReceiptBinding {
            directory: receipts,
            attachment: "factory-control".into(),
            generation,
        };
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let factory = RecoveryFactoryOwner::new(binding.clone(), budget.clone()).unwrap();
        let cancellation = RetryCancellation::default();
        let stop = cancellation.clone();
        let signals = Arc::new(
            SignalRegistration::with_receipt(
                "factory pure control",
                move || stop.cancel(),
                binding,
            )
            .unwrap(),
        );
        let scope = RecoveryScope::new(
            Arc::new(Surreal::init()),
            budget,
            cancellation,
            signals.pending_recipient().unwrap(),
            RecoveryTarget {
                generation,
                namespace: "control".into(),
                database: "validation".into(),
                endpoint: None,
            },
        )
        .unwrap();
        let lease = factory.attach(scope, signals).unwrap();
        (root, factory, lease)
    }
    fn allocation(lease: &FactoryLease) -> NativeRequest {
        NativeRequest {
            issuance: IssuanceEra {
                generation: lease.recovery_scope().target().generation,
                era: 5,
            },
            operation: ContentHash::of(b"exact allocation"),
            request: ContentHash::of(b"exact allocation bytes"),
        }
    }
    // No SDK/native finality is inferred. Release only the detached test guard.
    fn release_detached(factory: &RecoveryFactoryOwner, lease: FactoryLease) {
        let child = lease.0.child.clone();
        drop(lease);
        release_detached_child(factory, child);
    }
    fn release_detached_child(factory: &RecoveryFactoryOwner, child: Arc<Child>) {
        factory.remove(&child).unwrap();
        let child = Arc::try_unwrap(child)
            .ok()
            .expect("pure unissued child has no borrowers");
        // All test factories use Surreal::init and never connect or issue.
        // Consume the guard rather than inventing a remote acknowledgement.
        child.session.transfer();
    }
    #[tokio::test]
    async fn source_marker_terminal_knowledge_precedes_exact_sidecar_removal() {
        let (_root, factory, lease) = owner();
        let request = allocation(&lease);
        lease
            .register_attempt(ContentHash::of(b"attempt"), request)
            .unwrap();
        let marker = lease.0.child.marker().unwrap().unwrap();
        let path = lease
            .recovery_scope()
            .recipient
            .directory()
            .join(format!("{}.json", marker.envelope.identity().hex()));
        let mut read = read_source_finalization_marker(
            &path,
            lease.recovery_scope().target(),
            factory.budget(),
        )
        .unwrap();
        assert_eq!(read.marker().allocation, request);
        assert!(read.settle().is_err());
        read.record_terminal(EffectDisposition::FencedUncommitted)
            .unwrap();
        let durable = read_source_finalization_marker(
            &path,
            lease.recovery_scope().target(),
            factory.budget(),
        )
        .unwrap();
        assert_eq!(
            durable.marker().phase,
            SourceFinalizationPhase::SourceTerminal
        );
        assert_eq!(
            durable.marker().allocation_disposition,
            EffectDisposition::FencedUncommitted
        );
        assert!(read.record_terminal(EffectDisposition::Committed).is_err());
        read.settle().unwrap();
        read.settle().unwrap();
        assert!(!path.exists());
        lease.0.child.marker.lock().unwrap().take();
        release_detached(&factory, lease);
    }
    #[tokio::test]
    async fn live_lease_and_stopped_recovery_never_steal_child_ownership() {
        let (_root, factory, lease) = owner();
        let stopped = RetryCancellation::default();
        stopped.cancel();
        assert!(factory.drain(&stopped).await.is_err());
        assert_eq!(factory.pending_count().unwrap(), 1);
        let child = lease.0.child.clone();
        drop(lease);
        assert!(factory.drain(&stopped).await.is_err());
        assert_eq!(factory.pending_count().unwrap(), 1);
        release_detached_child(&factory, child);
        assert!(factory.drain(&stopped).await.is_ok());
    }
    #[tokio::test]
    async fn handback_retains_callable_owner_without_strong_cycle() {
        let (_root, factory, lease) = owner();
        let weak = Arc::downgrade(&factory);
        let error = factory.handback(ModelError::Conflict("constructor stopped"));
        let ModelError::Cause(cause) = &error else {
            panic!("owning cause required");
        };
        let handback = cause.downcast_ref::<FactoryHandback>().unwrap();
        assert!(Arc::ptr_eq(handback.factory(), &factory));
        release_detached(&factory, lease);
        drop(factory);
        assert!(weak.upgrade().is_some());
        drop(error);
        assert!(weak.upgrade().is_none());
    }
    #[tokio::test]
    async fn pin_obligations_retain_actual_state_without_owning_public_borrowers() {
        let (_root, factory, lease) = owner();
        let scope = lease.recovery_scope();
        let pin = Arc::new(super::super::super::ReaderPin {
            identity: ContentHash::of(b"pin"),
            state: Arc::new(super::super::super::ReaderPinState {
                identity: ContentHash::of(b"pin"),
                client: scope.client().clone(),
                released: AtomicBool::new(false),
                uncertain: AtomicBool::new(false),
                borrowed: AtomicBool::new(true),
                recovery: Some(scope.clone()),
                _charge: None,
            }),
        });
        lease
            .register_pin(pin.identity, allocation(&lease))
            .unwrap();
        lease.pin_ready(pin.clone()).unwrap();
        assert_eq!(
            Arc::strong_count(&pin),
            1,
            "factory state is not a public borrower"
        );
        let state = pin.factory_state();
        let wrapper = Arc::downgrade(&pin);
        let pin = Arc::try_unwrap(pin)
            .ok()
            .expect("existing atomic borrower proof remains valid");
        assert!(
            state.has_borrower(),
            "Arc unwrapping keeps the actual capability live"
        );
        drop(pin);
        assert!(!state.has_borrower());
        assert!(wrapper.upgrade().is_none());
        assert!(!state.is_released());
        let another = ContentHash::of(b"second pin");
        lease
            .register_pin(
                another,
                NativeRequest {
                    operation: ContentHash::of(b"second pin allocation"),
                    ..allocation(&lease)
                },
            )
            .unwrap();
        let mut pins = lease.0.child.pins.lock().unwrap();
        assert_eq!(
            pins.entries.len(),
            2,
            "one actual scope supports independent pins"
        );
        for entry in &pins.entries {
            let path = scope
                .recipient
                .directory()
                .join(format!("{}.json", entry.marker.envelope.identity().hex()));
            let mut read =
                read_pin_finalization_marker(&path, scope.target(), factory.budget()).unwrap();
            assert!(read.settle().is_err());
            let disposition = if read.marker().phase == SourceFinalizationPhase::SourceActive {
                EffectDisposition::Committed
            } else {
                EffectDisposition::FencedUncommitted
            };
            read.record_terminal(disposition).unwrap();
            read.settle().unwrap();
        }
        pins.entries.clear();
        drop(pins);
        release_detached(&factory, lease);
    }
    #[tokio::test]
    async fn concurrent_source_registration_keeps_one_exact_durable_obligation() {
        let (_root, factory, lease) = owner();
        let (one, two) = std::thread::scope(|threads| {
            let one = threads
                .spawn(|| lease.register_attempt(ContentHash::of(b"one"), allocation(&lease)));
            let two = threads.spawn(|| {
                lease.register_attempt(
                    ContentHash::of(b"two"),
                    NativeRequest {
                        operation: ContentHash::of(b"two allocation"),
                        ..allocation(&lease)
                    },
                )
            });
            (one.join().unwrap(), two.join().unwrap())
        });
        assert_ne!(one.is_ok(), two.is_ok());
        let marker = lease.0.child.marker().unwrap().unwrap();
        assert_eq!(
            std::fs::read_dir(lease.recovery_scope().recipient.directory())
                .unwrap()
                .count(),
            1
        );
        let path = lease
            .recovery_scope()
            .recipient
            .directory()
            .join(format!("{}.json", marker.envelope.identity().hex()));
        let mut read = read_source_finalization_marker(
            &path,
            lease.recovery_scope().target(),
            factory.budget(),
        )
        .unwrap();
        assert_eq!(read.marker(), marker.envelope.as_ref());
        read.record_terminal(EffectDisposition::FencedUncommitted)
            .unwrap();
        read.settle().unwrap();
        lease.0.child.marker.lock().unwrap().take();
        release_detached(&factory, lease);
    }
    #[tokio::test]
    async fn compacted_unknown_remains_distinct_and_cannot_replace_known_commit() {
        let (_root, factory, lease) = owner();
        lease
            .register_attempt(ContentHash::of(b"known source"), allocation(&lease))
            .unwrap();
        lease.source_ready().unwrap();
        let marker = lease.0.child.marker().unwrap().unwrap();
        let path = lease
            .recovery_scope()
            .recipient
            .directory()
            .join(format!("{}.json", marker.envelope.identity().hex()));
        let mut read = read_source_finalization_marker(
            &path,
            lease.recovery_scope().target(),
            factory.budget(),
        )
        .unwrap();
        assert!(
            read.record_terminal(EffectDisposition::CompactedTerminal)
                .is_err()
        );
        assert!(
            read.record_terminal(EffectDisposition::FencedUncommitted)
                .is_err()
        );
        read.record_terminal(EffectDisposition::Committed).unwrap();
        read.settle().unwrap();
        lease.0.child.marker.lock().unwrap().take();
        lease
            .register_pin(ContentHash::of(b"never observed pin"), allocation(&lease))
            .unwrap();
        let pins = lease.0.child.pins.lock().unwrap();
        let marker = pins.entries[0].marker.clone();
        drop(pins);
        let path = lease
            .recovery_scope()
            .recipient
            .directory()
            .join(format!("{}.json", marker.envelope.identity().hex()));
        let mut read =
            read_pin_finalization_marker(&path, lease.recovery_scope().target(), factory.budget())
                .unwrap();
        read.record_terminal(EffectDisposition::CompactedTerminal)
            .unwrap();
        assert_eq!(
            read.marker().allocation_disposition,
            EffectDisposition::CompactedTerminal
        );
        assert!(read.record_terminal(EffectDisposition::Committed).is_err());
        read.settle().unwrap();
        lease.0.child.pins.lock().unwrap().entries.clear();
        release_detached(&factory, lease);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinMarkerKind {
    PinFinalization,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinFinalizationMarker {
    pub schema: u8,
    pub kind: PinMarkerKind,
    pub host_owner: crate::signal::PendingRequestHostOwner,
    pub target: RecoveryTarget,
    pub pin: ContentHash,
    pub allocation: NativeRequest,
    pub original_epoch: i64,
    pub phase: SourceFinalizationPhase,
    pub allocation_disposition: EffectDisposition,
}
impl PinFinalizationMarker {
    pub fn identity(&self) -> ContentHash {
        let mut sink = KeySink::new("native-pin-finalization/v1");
        self.pin.encode(&mut sink);
        self.allocation.operation.encode(&mut sink);
        self.target.generation.encode(&mut sink);
        sink.finish()
    }
    pub fn validate(&self, expected: &RecoveryTarget) -> Result<(), ModelError> {
        self.target.validate()?;
        if self.schema != 1
            || self.target.generation != expected.generation
            || self.target.namespace != expected.namespace
            || self.target.database != expected.database
            || self
                .target
                .endpoint
                .as_ref()
                .is_some_and(|endpoint| expected.endpoint.as_ref() != Some(endpoint))
            || self.allocation.issuance.generation != expected.generation
            || self.allocation.issuance.era <= 0
            || self.original_epoch != 1
            || !self.host_owner.receipt.is_absolute()
            || self.host_owner.pid == 0
            || self.host_owner.pid_namespace == 0
            || self.host_owner.boot_id.is_empty()
        {
            return Err(ModelError::Conflict(
                "native pin finalization correspondence",
            ));
        }
        match (self.phase, self.allocation_disposition) {
            (SourceFinalizationPhase::AllocationPending, EffectDisposition::Pending)
            | (SourceFinalizationPhase::SourceActive, EffectDisposition::Committed)
            | (
                SourceFinalizationPhase::SourceTerminal,
                EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal,
            ) => Ok(()),
            _ => Err(ModelError::Conflict(
                "native pin finalization phase/disposition",
            )),
        }
    }
}
#[derive(Clone)]
struct OwnedPinMarker {
    envelope: Arc<PinFinalizationMarker>,
    _charge: Arc<Box<dyn Reservation>>,
}
#[derive(Clone)]
struct PinOwner {
    state: Arc<super::super::ReaderPinState>,
}
struct PinObligation {
    marker: OwnedPinMarker,
    pin: Option<PinOwner>,
}
struct PinObligations {
    entries: Vec<PinObligation>,
    charge: Box<dyn Reservation>,
}
impl Child {
    fn write_pin_marker(
        &self,
        marker: PinFinalizationMarker,
        charge: Box<dyn Reservation>,
        replace: bool,
    ) -> Result<(), ModelError> {
        marker.validate(&self.scope.target)?;
        let mut pins = self
            .pins
            .lock()
            .map_err(|_| ModelError::Conflict("native factory pin owners"))?;
        let index = pins
            .entries
            .iter()
            .position(|entry| entry.marker.envelope.pin == marker.pin);
        if replace != index.is_some() {
            return Err(ModelError::Conflict("native pin marker registration phase"));
        }
        if let Some(index) = index {
            let current = &pins.entries[index].marker.envelope;
            if current.phase == SourceFinalizationPhase::SourceActive
                && marker.allocation_disposition != EffectDisposition::Committed
            {
                return Err(ModelError::Conflict(
                    "known pin allocation disposition changed",
                ));
            }
            if current.phase == SourceFinalizationPhase::SourceTerminal
                && current.allocation_disposition != marker.allocation_disposition
            {
                return Err(ModelError::Conflict(
                    "terminal pin allocation disposition changed",
                ));
            }
            if current.identity() != marker.identity()
                || phase_order(marker.phase) < phase_order(current.phase)
            {
                return Err(ModelError::Conflict(
                    "native pin marker changed registration or phase",
                ));
            }
        }
        if index.is_none() {
            let capacity = pins
                .entries
                .len()
                .checked_add(1)
                .ok_or(ModelError::Conflict("native pin obligation count"))?
                .max(pins.entries.capacity());
            pins.charge.try_resize(
                capacity
                    .checked_mul(size_of::<PinObligation>())
                    .ok_or(ModelError::Conflict("native pin obligation index"))?,
            )?;
            if pins.entries.len() == pins.entries.capacity() {
                pins.entries.reserve_exact(1);
            }
            let bytes = pins
                .entries
                .capacity()
                .checked_mul(size_of::<PinObligation>())
                .ok_or(ModelError::Conflict("native pin obligation index"))?;
            pins.charge.try_resize(bytes)?;
        }
        let marker = OwnedPinMarker {
            envelope: Arc::new(marker),
            _charge: Arc::new(charge),
        };
        let written = write_sidecar(
            &self.scope.recipient,
            &self.scope.budget,
            &marker.envelope.identity().hex(),
            &marker.envelope,
            replace,
        );
        if written.is_ok() || !replace {
            match index {
                Some(index) => pins.entries[index].marker = marker,
                None => pins.entries.push(PinObligation { marker, pin: None }),
            }
        }
        written
    }
    fn finalize_pin<'a>(
        &'a self,
        admission: &'a RetryCancellation,
        abandon: bool,
    ) -> Result<futures::future::BoxFuture<'a, Result<(), ModelError>>, ModelError> {
        let body = async move {
            loop {
                let (marker, pin) = {
                    let pins = self
                        .pins
                        .lock()
                        .map_err(|_| ModelError::Conflict("native factory pin owners"))?;
                    let Some(first) = pins.entries.first() else {
                        return Ok(());
                    };
                    (first.marker.clone(), first.pin.clone())
                };
                let authority = if abandon {
                    if let Some(pin) = &pin
                        && ((!pin.state.is_released() && pin.state.has_borrower())
                            || pin.state.factory_uncertain())
                    {
                        return Err(outstanding(
                            "native factory pin retains readers or uncertain transport",
                        ));
                    }
                    super::super::authorization::PinFinalizationAuthority::OwnedDrained
                } else {
                    if let Some(pin) = &pin {
                        if !pin.state.is_released() && pin.state.has_borrower() {
                            return Err(outstanding(
                                "native factory pin has active source borrowers",
                            ));
                        }
                        pin.state.release().await?;
                    }
                    super::super::authorization::PinFinalizationAuthority::VerifyReleased
                };
                if admission.stopped() {
                    return Err(outstanding("native pin finalization admission stopped"));
                }
                let old = self
                    .successor
                    .lock()
                    .map_err(|_| ModelError::Conflict("native recovery successor"))?
                    .clone();
                if let Some(old) = old {
                    super::super::authorization::resume_recovery_scope_admitted(&old, admission)
                        .await?;
                }
                let successor = RecoveryScope::new(
                    self.scope.client.clone(),
                    self.scope.budget.clone(),
                    admission.clone(),
                    self.scope.recipient.clone(),
                    self.scope.target.clone(),
                )?;
                *self
                    .successor
                    .lock()
                    .map_err(|_| ModelError::Conflict("native recovery successor"))? =
                    Some(successor.clone());
                if marker.envelope.phase != SourceFinalizationPhase::SourceTerminal {
                    let disposition = super::super::authorization::finalize_factory_pin_admitted(
                        &successor,
                        &marker.envelope,
                        admission,
                        authority,
                    )
                    .await?;
                    let charge = marker_charge(&self.scope.budget, &self.scope)?;
                    let mut terminal = marker.envelope.as_ref().clone();
                    terminal.phase = SourceFinalizationPhase::SourceTerminal;
                    terminal.allocation_disposition = disposition;
                    self.write_pin_marker(terminal, charge, true)?;
                    if let Some(pin) = &pin {
                        pin.state.factory_released();
                    }
                }
                let terminal = {
                    let pins = self
                        .pins
                        .lock()
                        .map_err(|_| ModelError::Conflict("native factory pin owners"))?;
                    pins.entries
                        .iter()
                        .find(|entry| entry.marker.envelope.pin == marker.envelope.pin)
                        .ok_or(ModelError::Conflict("native pin finalization marker lost"))?
                        .marker
                        .clone()
                };
                super::super::authorization::release_factory_pin_references_admitted(
                    &successor,
                    &terminal.envelope,
                    admission,
                )
                .await?;
                super::super::authorization::resume_recovery_scope_admitted(&successor, admission)
                    .await?;
                if successor.pending_count()? != 0 {
                    return Err(outstanding("native pin finalization effects pending"));
                }
                self.scope
                    .recipient
                    .remove(&terminal.envelope.identity().hex())?;
                let mut pins = self
                    .pins
                    .lock()
                    .map_err(|_| ModelError::Conflict("native factory pin owners"))?;
                let index = pins
                    .entries
                    .iter()
                    .position(|entry| entry.marker.envelope.pin == marker.envelope.pin)
                    .ok_or(ModelError::Conflict("native pin finalization marker lost"))?;
                pins.entries.swap_remove(index);
                self.successor
                    .lock()
                    .map_err(|_| ModelError::Conflict("native recovery successor"))?
                    .take();
            }
        };
        crate::continuation::admit(&self.scope.budget, "native finalize_pin continuation", body)
    }
}
impl FactoryLease {
    pub fn register_pin(
        &self,
        pin: ContentHash,
        allocation: NativeRequest,
    ) -> Result<(), ModelError> {
        let scope = &self.0.child.scope;
        let charge = marker_charge(&scope.budget, scope)?;
        self.0.child.write_pin_marker(
            PinFinalizationMarker {
                schema: 1,
                kind: PinMarkerKind::PinFinalization,
                host_owner: scope.recipient.host_owner().clone(),
                target: scope.target.clone(),
                pin,
                allocation,
                original_epoch: 1,
                phase: SourceFinalizationPhase::AllocationPending,
                allocation_disposition: EffectDisposition::Pending,
            },
            charge,
            false,
        )
    }
    pub fn pin_ready(&self, pin: Arc<super::super::ReaderPin>) -> Result<(), ModelError> {
        pin.require_client(self.0.child.scope.client())?;
        let marker = {
            let mut pins = self
                .0
                .child
                .pins
                .lock()
                .map_err(|_| ModelError::Conflict("native factory pin owners"))?;
            let entry = pins
                .entries
                .iter_mut()
                .find(|entry| entry.marker.envelope.pin == pin.identity)
                .ok_or(ModelError::Conflict("native pin allocation marker missing"))?;
            if entry.marker.envelope.phase != SourceFinalizationPhase::AllocationPending {
                return Err(ModelError::Conflict("native pin allocation correspondence"));
            }
            entry.pin = Some(PinOwner {
                state: pin.factory_state(),
            });
            entry.marker.clone()
        };
        let charge = marker_charge(&self.0.child.scope.budget, &self.0.child.scope)?;
        let mut active = marker.envelope.as_ref().clone();
        active.phase = SourceFinalizationPhase::SourceActive;
        active.allocation_disposition = EffectDisposition::Committed;
        self.0.child.write_pin_marker(active, charge, true)
    }
}

/// A bounded parsed pin obligation. Parsing grants no maintenance authority.
pub struct ReadPinFinalizationMarker {
    marker: PinFinalizationMarker,
    recipient: PendingRequestRecipient,
    budget: ResourceBudget,
    settled: bool,
    _charge: Box<dyn Reservation>,
}
impl ReadPinFinalizationMarker {
    pub fn marker(&self) -> &PinFinalizationMarker {
        &self.marker
    }
    /// Caller authenticates the exact target and proves the original host dead
    /// under its receipt lock. Persist before releasing either lifecycle ref.
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
                "offline pin terminal disposition absent",
            ));
        }
        if self.marker.phase == SourceFinalizationPhase::SourceTerminal {
            return if self.marker.allocation_disposition == disposition {
                Ok(())
            } else {
                Err(ModelError::Conflict(
                    "offline pin terminal disposition changed",
                ))
            };
        }
        if self.marker.phase == SourceFinalizationPhase::SourceActive
            && disposition != EffectDisposition::Committed
        {
            return Err(ModelError::Conflict(
                "known committed allocation disposition changed",
            ));
        }
        let old_phase = self.marker.phase;
        let old_disposition = self.marker.allocation_disposition;
        self.marker.phase = SourceFinalizationPhase::SourceTerminal;
        self.marker.allocation_disposition = disposition;
        if let Err(error) = write_sidecar(
            &self.recipient,
            &self.budget,
            &self.marker.identity().hex(),
            &self.marker,
            true,
        ) {
            self.marker.phase = old_phase;
            self.marker.allocation_disposition = old_disposition;
            return Err(error);
        }
        Ok(())
    }
    /// Caller separately established pin terminality, both exact reference
    /// releases and drainage of any finalization requests. Owner receipt stays unchanged.
    pub fn settle(&mut self) -> Result<(), ModelError> {
        if self.settled {
            return Ok(());
        }
        if self.marker.phase != SourceFinalizationPhase::SourceTerminal {
            return Err(ModelError::Conflict(
                "offline pin settlement not established",
            ));
        }
        self.recipient.remove(&self.marker.identity().hex())?;
        self.settled = true;
        Ok(())
    }
}
pub fn read_pin_finalization_marker(
    path: &Path,
    expected: &RecoveryTarget,
    budget: &ResourceBudget,
) -> Result<ReadPinFinalizationMarker, ModelError> {
    if !path.is_absolute() || path.canonicalize().map_err(ModelError::codec)? != path {
        return Err(ModelError::Conflict("native pin marker redirected"));
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
        .and_then(|bytes| bytes.checked_add(size_of::<ReadPinFinalizationMarker>()))
        .ok_or(ModelError::Conflict("native pin marker size"))?;
    let charge = budget.reserve("native pin marker read", bytes)?;
    let mut encoded = vec![0; length];
    file.read_exact(&mut encoded).map_err(ModelError::codec)?;
    if file.read(&mut [0]).map_err(ModelError::codec)? != 0 {
        return Err(ModelError::Conflict(
            "native pin marker changed while reading",
        ));
    }
    let marker: PinFinalizationMarker =
        serde_json::from_slice(&encoded).map_err(ModelError::codec)?;
    marker.validate(expected)?;
    let receipt = &marker.host_owner.receipt;
    let parent = receipt
        .parent()
        .ok_or(ModelError::Conflict("native pin owner receipt parent"))?;
    let stem = receipt
        .file_stem()
        .ok_or(ModelError::Conflict("native pin owner receipt name"))?;
    if path.parent() != Some(parent.with_extension("requests").join(stem).as_path())
        || path.file_name().and_then(|name| name.to_str())
            != Some(format!("{}.json", marker.identity().hex()).as_str())
    {
        return Err(ModelError::Conflict(
            "native pin marker host/path correspondence",
        ));
    }
    let recipient =
        PendingRequestRecipient::for_recovery(marker.host_owner.clone(), marker.target.generation)?;
    Ok(ReadPinFinalizationMarker {
        marker,
        recipient,
        budget: budget.clone(),
        settled: false,
        _charge: charge,
    })
}
