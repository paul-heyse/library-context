//! Declared native request closure. The store owns content receipts and contextual publication
//! admission; this charged, pure inventory names actual current rows without a legacy ID bridge.
use super::{NativeContext, ObligationKind};
use crate::domain::{
    conditions::entry::{DerivedEntryValue, EntryData},
    local_theory::TheoryInventory,
    normalized::{Rows, callables::*, entities::EntityRef},
    resources::ResourceBudget,
    *,
};

pub struct NativeInventory {
    pub entry: EntryData,
    pub theory: TheoryInventory,
    pub effective: Rows<EffectiveCallableAssessment>,
    pub variants: Rows<SignatureVariant>,
    pub slots: Rows<SignatureSlot>,
    pub decorators: Rows<EffectiveDecoratorMember>,
}
impl NativeInventory {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            entry: EntryData::new(budget),
            theory: TheoryInventory::new(budget),
            effective: Rows::new(budget),
            variants: Rows::new(budget),
            slots: Rows::new(budget),
            decorators: Rows::new(budget),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut inputs = EntryData::validation_inputs();
        inputs.extend(TheoryInventory::validation_inputs());
        inputs.extend([
            ValidationInput::of::<EffectiveCallableAssessment>(&["id"]),
            ValidationInput::of::<SignatureVariant>(&["id"]),
            ValidationInput::of::<SignatureSlot>(&["id"]),
            ValidationInput::of::<EffectiveDecoratorMember>(&["id"]),
        ]);
        for input in &mut inputs {
            if stages::is_vocabulary(input.name()) {
                *input = input.clone().at_epoch(stages::PublicationBoundary::Facts);
            }
        }
        inputs.sort_by_key(|i| (i.name(), i.prefix()));
        inputs.dedup_by_key(|i| (i.name(), i.prefix()));
        inputs
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let mut handled = self.entry.visit(name, batch)?;
        handled |= self.theory.visit(name, batch)?;
        if name == EffectiveCallableAssessment::NAME {
            self.effective.decode(batch)?;
            handled = true;
        }
        if name == SignatureVariant::NAME {
            self.variants.decode(batch)?;
            handled = true;
        }
        if name == SignatureSlot::NAME {
            self.slots.decode(batch)?;
            handled = true;
        }
        if name == EffectiveDecoratorMember::NAME {
            self.decorators.decode(batch)?;
            handled = true;
        }
        if !handled {
            return Err(ModelError::Invalid(
                "undeclared native request inventory input".into(),
            ));
        }
        Ok(())
    }
    pub fn context(&self, entry: &DerivedEntryValue) -> Result<NativeContext, ObligationKind> {
        let Some(EntityRef::Callable { callable }) = self.entry.refs.get(entry.witness().owner)
        else {
            return Err(ObligationKind::EntryValueUnknown);
        };
        let mut assessments = self
            .effective
            .iter()
            .filter(|r| r.callable == *callable && r.context == entry.witness().context);
        let assessment = assessments.next().ok_or(ObligationKind::MissingEvidence)?;
        if assessments.next().is_some() {
            return Err(ObligationKind::EntryValueUnknown);
        }
        let mut slots = self
            .slots
            .iter()
            .filter(|r| r.parameter == entry.parameter());
        let slot = slots.next().ok_or(ObligationKind::MissingEvidence)?;
        if slots.next().is_some() {
            return Err(ObligationKind::EntryValueUnknown);
        }
        let variant = self
            .variants
            .get(slot.variant)
            .ok_or(ObligationKind::MissingEvidence)?;
        NativeContext::from_entry(
            entry,
            &self.entry,
            assessment,
            variant,
            slot,
            &self.decorators,
        )
    }
}
