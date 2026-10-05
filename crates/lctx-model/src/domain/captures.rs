//! Native capture origin characterizes dependence, not a concrete call-time value.
use super::{assertion::AssertionQualification, attribution::FactFamily, calls::ProviderSymbol, *};
use crate::{Assertion, Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CaptureOrigin {
    OuterFunction = 0,
    Global = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CaptureTiming {
    Unknown = 0,
    EagerSnapshot = 1,
    LazySnapshot = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "capture_observations", validate = validate_capture)]
#[assertion(support = CaptureSupport, name = "capture_supports", family = FactFamily::Signatures, subjects(function, declaring))]
pub struct CaptureObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub function: Id<ProviderSymbol>,
    #[model(key)]
    pub name: String,
    #[model(key)]
    pub origin: CaptureOrigin,
    #[model(key)]
    pub declaring: Option<Id<ProviderSymbol>>,
    /// Pyrefly's native capture map does not certify runtime mutation or snapshot time.
    #[model(key)]
    pub mutable: Option<bool>,
    #[model(key)]
    pub timing: CaptureTiming,
}
fn validate_capture(row: &CaptureObservation) -> Result<(), ModelError> {
    if row.name.is_empty()
        || (row.origin == CaptureOrigin::OuterFunction) != row.declaring.is_some()
        || row.declaring == Some(row.function)
    {
        return Err(ModelError::Invalid("invalid native capture origin".into()));
    }
    Ok(())
}
