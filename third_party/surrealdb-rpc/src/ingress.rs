//! Per-query ingress policy and ownership, independent of an application's budget type.

use std::fmt::Debug;
use std::num::NonZeroU32;
use std::sync::Arc;

use surrealdb_types::Error;

use crate::QueryStreamItem;

/// An allocation boundary covered by a caller's ingress admission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngressStage {
	/// Connection and capability-handshake storage.
	Connection,
	/// Encoded transport-frame storage.
	EncodedFrame,
	/// Decompressed transport-frame storage.
	DecompressedFrame,
	/// Compression context, dictionary/window and fixed decode scratch.
	CompressionWorkspace,
	/// Protobuf container and field decoding.
	ProtobufDecode,
	/// Conversion into native values, including container allocations.
	ValueConversion,
}

/// An admission lease. Dropping the final shared lease releases its charge.
///
/// Implementations own their actual reservation; the transport does not know the
/// application's budget representation. A lease must remain live while the
/// allocation it covers is reachable, including through queued/delivered items.
pub trait IngressReservation: Debug + Send + Sync + 'static {}

/// Authorizes an allocation *before* the named allocation boundary.
pub trait IngressAdmission: Debug + Send + Sync + 'static {
	/// Refusal must leave the corresponding allocation unperformed.
	fn reserve(&self, stage: IngressStage, bytes: usize)
		-> Result<Arc<dyn IngressReservation>, Error>;

	/// Compile or reuse a regex under this owner's actual allocation contract.
	/// The returned lease must cover compilation workspace and retained compiled
	/// state. No generic encoded-size multiplier can establish that guarantee.
	/// Owners without a qualified compiler refuse before regex compilation.
	fn compile_regex(&self, _pattern: &str)
		-> Result<(surrealdb_types::Regex, Arc<dyn IngressReservation>), Error> {
		Err(Error::configuration("This ingress owner does not admit regex compilation".to_owned(), None))
	}
}

/// Options for finite streamed query ingress.
///
/// A byte ceiling is on each complete uncompressed encoded `QueryResponse`,
/// including control, error and terminal frames. It is not a decoded-memory
/// bound. Admission is a separate contract and cannot silently degrade to byte
/// framing on an engine that does not implement its allocation boundaries.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct QueryStreamOptions {
	/// Optional complete encoded-response ceiling; zero is not a valid ceiling.
	pub max_batch_bytes: Option<NonZeroU32>,
	/// The owner authorizing all supported ingress allocation boundaries.
	pub admission: Option<Arc<dyn IngressAdmission>>,
}

impl QueryStreamOptions {
	/// Default transport behavior, without an ingress-admission claim.
	pub fn new() -> Self { Self::default() }
	/// Request complete-response byte framing, requiring peer capability support.
	pub fn max_batch_bytes(mut self, bytes: NonZeroU32) -> Self {
		self.max_batch_bytes = Some(bytes);
		self
	}
	/// Attach the independently owned admission policy.
	pub fn admission(mut self, admission: Arc<dyn IngressAdmission>) -> Self {
		self.admission = Some(admission);
		self
	}
}

/// A query item and the reservations covering its reachable allocations.
#[derive(Debug)]
pub struct AdmittedQueryStreamItem {
	item: QueryStreamItem,
	reservations: Vec<Arc<dyn IngressReservation>>,
}

impl AdmittedQueryStreamItem {
	/// Construct an item after its allocations were admitted.
	pub fn new(item: QueryStreamItem, reservations: Vec<Arc<dyn IngressReservation>>) -> Self {
		Self { item, reservations }
	}
	/// Borrow the item without releasing its reservations.
	pub fn item(&self) -> &QueryStreamItem { &self.item }
	/// Transfer both item and leases to a new owner. Keep the leases until the
	/// transferred item is destroyed or fully covered by replacement admission.
	pub fn into_parts(self) -> (QueryStreamItem, Vec<Arc<dyn IngressReservation>>) {
		(self.item, self.reservations)
	}
}

/// A terminal query failure and reservations covering its retained details.
/// The producer sends this after its receive stream reports termination. A
/// local decode/admission failure may terminate that stream without physical
/// body EOF; callers must preserve unconfirmed remote finality in that case.
#[derive(Debug)]
pub struct AdmittedQueryFailure {
	error: Box<Error>,
	reservations: Vec<Arc<dyn IngressReservation>>,
}
impl AdmittedQueryFailure {
	/// Transfer a failed operation and its actual retained allocation leases.
	/// The supplied leases must already cover `boxed_error_allocation_size()`.
	pub fn new(error: Error, reservations: Vec<Arc<dyn IngressReservation>>) -> Self { Self { error: Box::new(error), reservations } }
	/// Exact cold error backing admitted before constructing this wrapper.
	pub const fn boxed_error_allocation_size() -> usize { std::mem::size_of::<Error>() }
	/// Inspect the original failure without discarding ownership.
	pub fn error(&self) -> &Error { &self.error }
	/// Transfer failure and leases together to the caller's result owner.
	pub fn into_parts(self) -> (Error, Vec<Arc<dyn IngressReservation>>) { (*self.error, self.reservations) }
}
impl std::fmt::Display for AdmittedQueryFailure {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Display::fmt(&self.error, formatter)
	}
}
impl std::error::Error for AdmittedQueryFailure {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(self.error.as_ref()) }
}

/// Buffered query results with one finite owner for their retained allocations.
/// Extracted bare values require independent admission before this owner drops.
#[derive(Debug)]
pub struct AdmittedQueryResults {
    /// Original per-statement wire/semantic results, unchanged.
    pub results: Vec<crate::QueryResult>,
    /// Shared ownership of the actual admitted query allocation resources.
    pub reservations: Option<Arc<dyn IngressReservation>>,
    /// Owner available to admit the SDK's final indexing allocation.
    pub admission: Option<Arc<dyn IngressAdmission>>,
}
impl AdmittedQueryResults {
    /// Ordinary engines make no ingress allocation claim.
    pub fn unadmitted(results: Vec<crate::QueryResult>) -> Self {
        Self { results, reservations: None, admission: None }
    }
}

/// Ordered delivery from an admitted finite query. Failure is a stream-level
/// terminal; it is never fabricated as a statement's Finished item.
#[derive(Debug)]
pub enum AdmittedQueryStreamEvent {
	/// An actual statement item.
	Item(AdmittedQueryStreamItem),
	/// A failure after producer completion or receive-stream termination; this
	/// does not itself prove physical body EOF or remote executor completion.
	Failure(AdmittedQueryFailure),
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[derive(Debug)]
	struct Lease(Arc<AtomicUsize>);
	impl IngressReservation for Lease {}
	impl Drop for Lease {
		fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
	}

	#[test]
	fn leases_survive_delivery_and_explicit_transfer() {
		let drops = Arc::new(AtomicUsize::new(0));
		let lease: Arc<dyn IngressReservation> = Arc::new(Lease(Arc::clone(&drops)));
		let first = AdmittedQueryStreamItem::new(QueryStreamItem::EmptyValue { index: 1 }, vec![Arc::clone(&lease)]);
		let second = AdmittedQueryStreamItem::new(QueryStreamItem::Rows { index: 2, values: vec![] }, vec![Arc::clone(&lease)]);
		drop(lease);
		assert!(matches!(first.item(), QueryStreamItem::EmptyValue { index: 1 }));
		let (item, leases) = first.into_parts();
		drop(second);
		assert_eq!(drops.load(Ordering::SeqCst), 0);
		drop(item);
		drop(leases);
		assert_eq!(drops.load(Ordering::SeqCst), 1);
	}
}
