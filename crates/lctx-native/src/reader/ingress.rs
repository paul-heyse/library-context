//! Native finite-query allocation admission. ResourceBudget covers owned buffers,
//! not process RSS or the shared H2/TLS runtime's independent allocation policy.
use lctx_model::domain::{
    ModelError,
    resources::{Reservation, ResourceBudget},
};
use std::{num::NonZeroU32, sync::Arc};
use surrealdb::method::{IngressAdmission, IngressReservation, IngressStage, QueryStreamOptions};

#[derive(Debug)]
struct NativeIngressBudget(ResourceBudget);
#[derive(Debug)]
struct NativeIngressReservation {
    _reservation: Box<dyn Reservation>,
}
impl IngressReservation for NativeIngressReservation {}
impl IngressAdmission for NativeIngressBudget {
    fn reserve(
        &self,
        stage: IngressStage,
        bytes: usize,
    ) -> surrealdb::Result<Arc<dyn IngressReservation>> {
        let owner = match stage {
            IngressStage::Connection => "native connection ingress",
            IngressStage::EncodedFrame => "native encoded ingress",
            IngressStage::DecompressedFrame => "native decompressed ingress",
            IngressStage::CompressionWorkspace => "native compression workspace",
            IngressStage::ProtobufDecode => "native protobuf ingress",
            IngressStage::ValueConversion => "native converted ingress",
            _ => "native ingress",
        };
        self.0
            .reserve(owner, bytes)
            .map(|reservation| {
                Arc::new(NativeIngressReservation {
                    _reservation: reservation,
                }) as Arc<dyn IngressReservation>
            })
            .map_err(|error| {
                surrealdb::Error::configuration(
                    format!("Native ingress admission refused: {error}"),
                    None,
                )
            })
    }
    // Model-owned catalog envelopes contain typed primitives, arrays, objects and
    // exact native IDs, never compiled regex programs. A future regex consumer
    // must supply a qualified compiler/reuse lease through the SDK policy hook.
}
pub(crate) fn admission(budget: &ResourceBudget) -> Arc<dyn IngressAdmission> {
    Arc::new(NativeIngressBudget(budget.clone()))
}
pub(crate) fn options(budget: &ResourceBudget) -> Result<QueryStreamOptions, ModelError> {
    // Wire size and semantic row width are independent. Never ask the peer for
    // a frame larger than this entire actual pool could admit; the negotiated
    // connection ceiling also applies. Structural expansion and coexistence use
    // their actual reservations, rather than a guessed logical-to-wire ratio.
    let bytes = NonZeroU32::new(
        budget
            .limit()
            .min(u32::MAX as usize)
            .try_into()
            .map_err(|_| ModelError::Schema("native ingress frame bound"))?,
    )
    .ok_or(ModelError::Schema("native ingress frame bound"))?;
    Ok(QueryStreamOptions::new()
        .max_batch_bytes(bytes)
        .admission(admission(budget)))
}
#[derive(Debug)]
pub(super) struct LeasedFailure {
    pub source: ModelError,
    pub _reservations: Vec<Arc<dyn IngressReservation>>,
}
impl std::fmt::Display for LeasedFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}
impl std::error::Error for LeasedFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_pool_refusal_and_shared_lease_lifetime() {
        let budget = ResourceBudget::fixed(128).unwrap();
        let owner = admission(&budget);
        let lease = owner.reserve(IngressStage::EncodedFrame, 96).unwrap();
        let borrower = lease.clone();
        assert!(owner.reserve(IngressStage::ProtobufDecode, 33).is_err());
        assert_eq!(budget.reserved(), 96);
        drop(lease);
        assert_eq!(budget.reserved(), 96);
        drop(borrower);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn retained_ingress_failure_keeps_model_certainty_and_actual_reservations() {
        let budget = ResourceBudget::fixed(128).unwrap();
        let leases = vec![
            admission(&budget)
                .reserve(IngressStage::ValueConversion, 96)
                .unwrap(),
        ];
        let failure = ModelError::Cause(Box::new(LeasedFailure {
            source: ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                "late terminal failure",
            ),
            _reservations: leases,
        }));
        assert!(!failure.permits_storage_cleanup());
        assert_eq!(budget.reserved(), 96);
        drop(failure);
        assert_eq!(budget.reserved(), 0);
    }
}
