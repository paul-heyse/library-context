//! Synthesis ranking and independent conclusion root inventories. Root parameters are execution
//! values; these exact joins and negative/empty universes belong to the model.
use super::{scope_program::*, *};
use std::any::TypeId;
fn col(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn eq(a: usize, af: &'static str, b: usize, bf: &'static str) -> ScopePredicate {
    ScopePredicate::Equal(col(a, af), col(b, bf))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inventory {
    Public,
    Usage,
    Rank,
    Community,
    CommunityMember,
    StructuralConclusion,
    AnalyticConclusion,
    SummaryConclusion,
}
impl Inventory {
    pub const ALL: [Self; 8] = [
        Self::Public,
        Self::Usage,
        Self::Rank,
        Self::Community,
        Self::CommunityMember,
        Self::StructuralConclusion,
        Self::AnalyticConclusion,
        Self::SummaryConclusion,
    ];
    pub fn type_id(self) -> TypeId {
        match self {
            Self::Public => TypeId::of::<structural::PublicCandidate>(),
            Self::Usage => TypeId::of::<structural::UsageScore>(),
            Self::Rank => TypeId::of::<analytics::RankScore>(),
            Self::Community => TypeId::of::<analytics::Community>(),
            Self::CommunityMember => TypeId::of::<analytics::CommunityMember>(),
            Self::StructuralConclusion => TypeId::of::<structural::Conclusion>(),
            Self::AnalyticConclusion => TypeId::of::<analytics::Conclusion>(),
            Self::SummaryConclusion => {
                TypeId::of::<execution::summary_consequences::ClaimConclusion>()
            }
        }
    }
}
/// Parameters: Structural frame, Analytic frame, Summary invocation.
pub fn program(inputs: Vec<ValidationInput>, kind: Inventory) -> Result<ScopeProgram, ModelError> {
    let find = |ty| {
        inputs
            .iter()
            .position(|input| input.type_id() == ty)
            .ok_or(ModelError::Schema("synthesis inventory input"))
    };
    let root = find(kind.type_id())?;
    let mut rows = vec![root];
    let mut predicates = Vec::new();
    match kind {
        Inventory::Public | Inventory::StructuralConclusion => {
            predicates.push(ScopePredicate::Parameter(col(0, "frame"), 0))
        }
        Inventory::AnalyticConclusion => {
            predicates.push(ScopePredicate::Parameter(col(0, "frame"), 1))
        }
        Inventory::SummaryConclusion => {
            predicates.push(ScopePredicate::Parameter(col(0, "invocation"), 2))
        }
        Inventory::Usage => {
            rows.push(find(TypeId::of::<structural::PublicCandidate>())?);
            predicates.extend([
                eq(1, "entity", 0, "target"),
                eq(1, "frame", 0, "frame"),
                ScopePredicate::Parameter(col(0, "frame"), 0),
            ]);
        }
        Inventory::Rank => {
            rows.extend([
                find(TypeId::of::<analytics::TechniqueResult>())?,
                find(TypeId::of::<structural::PublicCandidate>())?,
            ]);
            predicates.extend([
                eq(0, "result", 1, "id"),
                eq(2, "entity", 0, "target"),
                ScopePredicate::Parameter(col(1, "frame"), 1),
                ScopePredicate::Parameter(col(2, "frame"), 0),
            ]);
        }
        Inventory::Community => {
            rows.push(find(TypeId::of::<analytics::TechniqueResult>())?);
            predicates.extend([
                eq(0, "result", 1, "id"),
                ScopePredicate::Parameter(col(1, "frame"), 1),
            ]);
        }
        Inventory::CommunityMember => {
            rows.extend([
                find(TypeId::of::<analytics::Community>())?,
                find(TypeId::of::<analytics::TechniqueResult>())?,
                find(TypeId::of::<structural::PublicCandidate>())?,
            ]);
            predicates.extend([
                eq(0, "community", 1, "id"),
                eq(1, "result", 2, "id"),
                eq(3, "entity", 0, "entity"),
                ScopePredicate::Parameter(col(2, "frame"), 1),
                ScopePredicate::Parameter(col(3, "frame"), 0),
            ]);
        }
    }
    let ports = (0..inputs.len())
        .map(|input| ScopePort {
            input,
            virtual_owner: false,
        })
        .collect();
    Ok(ScopeProgram {
        inputs,
        ports,
        rules: vec![ScopeRule::Pairs {
            source: root,
            target: root,
            rows,
            predicates,
            source_key: col(0, "id"),
            target_key: col(0, "id"),
        }],
    })
}
#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn synthesis_inventory_domains_validate_against_exact_model() {
        let model = super::super::model().unwrap();
        let inputs = synthesis::production::Data::inputs(stages::Profile::Behavioral);
        for kind in Inventory::ALL {
            program(inputs.clone(), kind)
                .unwrap()
                .validate(&model)
                .unwrap();
        }
    }
}
