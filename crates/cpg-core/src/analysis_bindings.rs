//! Compact application authority produced by the binding owner and bound to admitted streams.
use crate::workspace::{CheckedInputs, CompletedInputs, Workspace};
use lctx_model::domain::{normalized::binding_normalization::{BindingData, BindingOutput, VerifiedBindings}, *};

pub struct PreparedBindings {
    checked: CheckedInputs,
    application: VerifiedBindings,
}
impl PreparedBindings {
    pub(crate) fn new(application: VerifiedBindings, normalized: &CheckedInputs) -> Result<Self, ModelError> {
        let mut inputs = BindingData::validation_inputs();
        inputs.extend(BindingOutput::validation_inputs());
        Ok(Self { checked: normalized.select(&inputs)?, application })
    }
    /// A borrowing consumer must declare exactly the immutable authority streams it uses.
    pub fn application<'a>(&'a self, access: &CompletedInputs, runtime: &Workspace)
        -> Result<&'a VerifiedBindings, ModelError>
    {
        self.checked.require_subset(runtime, access)?;
        Ok(&self.application)
    }
}
