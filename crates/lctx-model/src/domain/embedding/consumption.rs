//! One pure value-admission operation shared by nominal analytic and retrieval consumers.
use super::{Spec,value::{self,DecodedValue}};
use crate::domain::{*,resources::ResourceBudget};

/// Borrowed canonical receipt fields. They are evidence to check, never an admission token.
pub struct ValueReceipt<'a> {
    pub input:ContentHash,
    pub codec:i16,
    pub digest:ContentHash,
    pub admitted_tokens:u32,
    pub bytes:&'a [u8],
}
/// A generation-scoped index retains exact bytes, including signed zero. Neither consumer may
/// replace an already observed winner for the same selected specification and request.
pub struct Winners {
    values:charged::ChargedMap<(ContentHash,ContentHash),EvidenceBytes>,
    charge:charged::StateCharge,
    budget:ResourceBudget,
}
impl Winners {
    pub fn new(budget:&ResourceBudget)->Self {Self {values:Default::default(),charge:charged::StateCharge::new(budget,"embedding-consumption-winners"),budget:budget.clone()}}
    pub fn replay(&mut self,spec:&Spec,document:&str,receipt:ValueReceipt<'_>)->Result<DecodedValue,ModelError> {
        let bound=document.len().checked_mul(spec.document_template.matches("{text}").count()).and_then(|n|n.checked_add(spec.document_template.len())).and_then(|n|n.checked_mul(2)).ok_or_else(||ModelError::Invalid("embedding replay request overflow".into()))?;
        let _request=self.budget.reserve("embedding-replay-request",bound)?;
        let request=spec.document_text(document);
        if receipt.codec!=value::VALUE_CODEC || receipt.input!=value::input_hash(&request) {return Err(ModelError::Invalid("consumed embedding codec or exact request differs".into()));}
        let decoded=value::decode(spec,receipt.bytes,receipt.digest,receipt.admitted_tokens,&self.budget)?;
        let key=(spec.hash(),receipt.input);
        if let Some(previous)=self.values.get(&key) {
            if previous.0!=receipt.bytes {return Err(ModelError::Invalid("embedding consumers disagree on exact winning bytes".into()));}
        }else {
            // Reserve transient copying before the charged map assumes retained ownership.
            let _copy=self.budget.reserve("embedding-winner-copy",receipt.bytes.len())?;
            self.values.insert(&mut self.charge,key,EvidenceBytes(receipt.bytes.to_vec()))?;
        }
        Ok(decoded)
    }
}
