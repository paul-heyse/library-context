//! Complete native signature enumeration domains, including omitted and constructor members.
use super::super::{
    catalog_scope_program::{BudgetedCatalogProgram, Builder},
    scope_program::*,
    *,
};
use assertion::AssertionQualification;
use calls::{
    ProviderSymbol, Signature, SignatureEnumerationMember, SignatureEnumerationObservation,
};
use std::any::TypeId;
pub fn build(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    budget: &resources::ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let index = |kind| {
        inputs
            .iter()
            .position(|i| i.type_id() == kind)
            .ok_or(ModelError::Schema("enumeration scope input"))
    };
    let symbol = index(TypeId::of::<ProviderSymbol>())?;
    let enumeration = index(TypeId::of::<SignatureEnumerationObservation>())?;
    let member = index(TypeId::of::<SignatureEnumerationMember>())?;
    let signature = index(TypeId::of::<Signature>())?;
    let qualification = index(TypeId::of::<AssertionQualification>())?;
    let real = inputs.len();
    let mut declarations = inputs.clone();
    declarations.push(inputs[symbol].clone());
    let mut b = Builder::new(declarations, real, input_relations, budget)?;
    let col = |row, field| ScopeColumn { row, field };
    b.pair(real, symbol, &[symbol], vec![], col(0, "id"), col(0, "id"));
    b.reverse(enumeration, "symbol", real);
    b.reverse(signature, "symbol", real);
    b.reverse(member, "enumeration", enumeration);
    b.follow(enumeration, "qualification", qualification, false);
    b.follow(signature, "qualification", qualification, false);
    b.finish()
}
