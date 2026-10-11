//! An acknowledgement keeps the original request identities and actual dispositions.
//! These are installation-local observations, never portable admission authority.
use super::*;
use lctx_model::domain::resources::{Reservation, ResourceBudget};
use std::{future::Future, sync::Mutex};

#[derive(Clone, Copy, Debug)]
pub struct EffectReceipt {
    pub request: NativeRequest,
    pub disposition: EffectDisposition,
}
pub struct EffectReceipts {
    entries: Vec<EffectReceipt>,
    _charge: Box<dyn Reservation>,
}
impl EffectReceipts {
    pub fn entries(&self) -> &[EffectReceipt] { &self.entries }
}
struct Book {
    entries: Vec<EffectReceipt>,
    charge: Box<dyn Reservation>,
}
tokio::task_local! { static RECEIPTS: Arc<Mutex<Book>>; }
pub(super) fn active() -> bool { RECEIPTS.try_with(|_| ()).is_ok() }

pub async fn capture_effect_receipts<T>(budget: &ResourceBudget,
    future: impl Future<Output = Result<T, ModelError>>)
    -> Result<(Result<T, ModelError>, EffectReceipts), ModelError>
{
    let book = Arc::new(Mutex::new(Book { entries: Vec::new(),
        charge: budget.reserve("native ingress receipts", 0)? }));
    let result = RECEIPTS.scope(book.clone(), future).await;
    let book = Arc::try_unwrap(book).map_err(|_| ModelError::Conflict("native receipt ownership"))?
        .into_inner().map_err(|_| ModelError::Conflict("native receipt collection"))?;
    Ok((result, EffectReceipts { entries: book.entries, _charge: book.charge }))
}

pub(super) fn accepted(request: NativeRequest) -> Result<(), ModelError> {
    RECEIPTS.try_with(|book| {
        let mut book = book.lock().map_err(|_| ModelError::Conflict("native receipt collection"))?;
        let length = book.entries.len().checked_add(1).ok_or(ModelError::Schema("native receipt count"))?;
        // reserve_exact admits the requested storage before allocation. Account actual
        // allocator capacity afterwards, retaining the prior reservation on refusal.
        let capacity = length.max(book.entries.capacity());
        book.charge.try_resize(capacity.saturating_mul(size_of::<EffectReceipt>()))?;
        if length > book.entries.capacity() { book.entries.reserve_exact(1); }
        let capacity = book.entries.capacity();
        book.charge.try_resize(capacity.saturating_mul(size_of::<EffectReceipt>()))?;
        book.entries.push(EffectReceipt { request, disposition: EffectDisposition::Pending });
        Ok(())
    }).unwrap_or(Ok(()))
}
pub(super) fn observed(request: NativeRequest, disposition: EffectDisposition) -> Result<(), ModelError> {
    RECEIPTS.try_with(|book| {
        let mut book = book.lock().map_err(|_| ModelError::Conflict("native receipt collection"))?;
        let receipt = book.entries.iter_mut().rev().find(|entry| entry.request == request)
            .ok_or(ModelError::Conflict("native receipt request correspondence"))?;
        receipt.disposition = disposition;
        Ok(())
    }).unwrap_or(Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn receipts_retain_original_request_disposition_and_budget() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let request = NativeRequest { issuance: IssuanceEra { generation: ContentHash::of(b"g"), era: 7 },
            operation: ContentHash::of(b"operation"), request: ContentHash::of(b"bytes") };
        let (result, receipts) = capture_effect_receipts(&budget, async {
            accepted(request)?;
            observed(request, EffectDisposition::FencedUncommitted)?;
            Err::<(), _>(ModelError::Conflict("original failure"))
        }).await.unwrap();
        assert!(matches!(result, Err(ModelError::Conflict("original failure"))));
        assert_eq!(receipts.entries()[0].request, request);
        assert_eq!(receipts.entries()[0].disposition, EffectDisposition::FencedUncommitted);
        assert!(budget.reserved() > 0);
        drop(receipts); assert_eq!(budget.reserved(), 0);
    }
}
