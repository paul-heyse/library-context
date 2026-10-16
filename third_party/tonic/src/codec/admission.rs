//! Optional allocation admission for receive-side transport buffers.
//!
//! This seam covers tonic-owned copying and buffer growth. HTTP/2, TLS,
//! compression-library scratch and application decoding have separate owners.

use std::fmt::Debug;
use std::sync::Arc;

use bytes::{BufMut, BytesMut};

use crate::Status;

/// Receive-side allocation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiveAllocation {
    /// A copied HTTP-body frame, before `Buf::copy_to_bytes`.
    BodyCopy,
    /// Encoded gRPC storage, including bootstrap allocation.
    EncodedBuffer,
    /// Decoder context, dictionary/window and fixed output scratch.
    CompressionWorkspace,
    /// Expanded uncompressed frame storage.
    DecompressedBuffer,
}

/// Opaque ownership of an application reservation.
pub trait ReceiveReservation: Debug + Send + Sync + 'static {}

/// Budget-neutral admission callback for tonic-owned allocations.
pub trait ReceiveAdmission: Debug + Send + Sync + 'static {
    /// Optional application ceiling on the complete uncompressed message. The
    /// connection's independent raw-wire ceiling still applies to compressed
    /// input, whose framing can be slightly larger than the original message.
    fn max_uncompressed_message_size(&self) -> Option<usize> { None }

    /// Authorize bytes before the corresponding allocation is performed.
    fn reserve(&self, boundary: ReceiveAllocation, bytes: usize)
        -> Result<Arc<dyn ReceiveReservation>, Status>;
}

/// Exactly sized buffer replacement keeps old and new reservations live across
/// the copy. It does not rely on BytesMut's geometric reserve-growth policy.
pub(crate) fn ensure_capacity(
    buf: &mut BytesMut,
    reservation: &mut Option<Arc<dyn ReceiveReservation>>,
    admission: &dyn ReceiveAdmission,
    required: usize,
) -> Result<(), Status> {
    ensure_capacity_for(buf, reservation, admission, ReceiveAllocation::EncodedBuffer, required)
}

pub(crate) fn ensure_capacity_for(
    buf: &mut BytesMut,
    reservation: &mut Option<Arc<dyn ReceiveReservation>>,
    admission: &dyn ReceiveAdmission,
    boundary: ReceiveAllocation,
    required: usize,
) -> Result<(), Status> {
    if buf.capacity() >= required { return Ok(()); }
    let next_reservation = admission.reserve(boundary, required)?;
    let mut next = BytesMut::with_capacity(required);
    next.put_slice(buf);
    // Destroy the old buffer before releasing its charge. A decoder sharing
    // bytes from that allocation separately retains the same reservation.
    *buf = next;
    *reservation = Some(next_reservation);
    Ok(())
}
