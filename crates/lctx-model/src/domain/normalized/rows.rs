//! Canonical typed algorithm output with charged retained state.
use crate::domain::{
    charged::{ChargedMap, StateCharge},
    resources::ResourceBudget,
    *,
};

pub struct Rows<R: Record> {
    rows: ChargedMap<Id<R>, R>,
    charge: StateCharge,
}
impl<R: Record> Rows<R> {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: Default::default(),
            charge: StateCharge::new(budget, R::NAME),
        }
    }
    pub fn insert(&mut self, row: R) -> Result<Id<R>, ModelError> {
        row.validate()?;
        let id = row.id();
        if let Some(previous) = self.rows.get(&id) {
            if previous != &row {
                return Err(ModelError::Conflict(R::NAME));
            }
        } else {
            self.rows.insert(&mut self.charge, id, row)?;
        }
        Ok(id)
    }
    pub fn decode(&mut self, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        let mut transient = StateCharge::new(
            self.charge.budget().expect("rows budget"),
            "normalized-decode",
        );
        transient.grow(batch.get_array_memory_size().saturating_mul(4))?;
        for row in R::decode(batch)? {
            self.insert(row)?;
        }
        Ok(())
    }
    pub fn same(&self, other: &Self) -> bool {
        self.rows.iter().eq(other.rows.iter())
    }
    pub fn get(&self, id: Id<R>) -> Option<&R> {
        self.rows.get(&id)
    }
    /// Keep missing-premise meaning with the consuming domain.
    pub fn required<E>(&self, id: Id<R>, missing: impl FnOnce() -> E) -> Result<&R, E> {
        self.get(id).ok_or_else(missing)
    }
    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.rows.values()
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod required_controls {
    use super::*;
    use crate::Domain;
    #[derive(Debug, Clone, PartialEq, Eq, Domain)]
    #[model(name="required_rows_control")]
    struct Row { #[model(key)] key: String, value: String }
    #[test]
    fn required_lookup_preserves_error_domains_conflicts_and_retained_admission() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let row = Row { key: "key".into(), value: "original".into() };
        let mut rows = Rows::new(&budget);
        assert_eq!(rows.required(row.id(), || crate::domain::obligation::ObligationKind::MissingEvidence), Err(crate::domain::obligation::ObligationKind::MissingEvidence));
        assert_eq!(rows.required(row.id(), || "different domain"), Err("different domain"));
        rows.insert(row.clone()).unwrap();
        let reserved = budget.reserved();
        assert!(reserved > 0);
        assert_eq!(rows.required(row.id(), || "missing").unwrap(), &row);
        rows.insert(row.clone()).unwrap();
        assert_eq!(budget.reserved(), reserved);
        assert!(matches!(rows.insert(Row { value: "conflict".into(), ..row.clone() }), Err(ModelError::Conflict("required_rows_control"))));
        assert_eq!(rows.get(row.id()), Some(&row));
        drop(rows);
        assert_eq!(budget.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        let mut rows = Rows::new(&tiny);
        assert!(matches!(rows.insert(row), Err(ModelError::Resource { .. })));
        assert!(rows.is_empty()); assert_eq!(tiny.reserved(), 0);
    }
}
