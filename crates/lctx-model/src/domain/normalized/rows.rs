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
        transient.grow(crate::domain::record::decode_allowance::<R>(batch)?)?;
        let rows = R::decode(batch)?;
        // Physical decoder scratch has ended. Keep the returned vector and payload charged
        // through the map handoff, including spare capacity; map insertions reserve first.
        let retained = crate::domain::record::rows_bytes(&rows, rows.capacity())?;
        if retained > transient.reserved() {
            transient.grow(retained - transient.reserved())?;
        } else {
            transient.release(transient.reserved() - retained);
        }
        for row in rows {
            self.insert(row)?;
        }
        Ok(())
    }
    pub fn same(&self, other: &Self) -> bool {
        self.rows.iter().eq(other.rows.iter())
    }
    /// Bounded diagnostics for a failed exact algorithm replay; never changes membership.
    pub(crate) fn difference(&self, expected: &Self) -> String {
        let missing = expected
            .rows
            .iter()
            .filter(|(id, _)| !self.rows.contains_key(id));
        let extra = self
            .rows
            .iter()
            .filter(|(id, _)| !expected.rows.contains_key(id));
        let changed = self
            .rows
            .iter()
            .filter(|(id, row)| expected.rows.get(id).is_some_and(|other| other != *row));
        let describe = |rows: Vec<(&Id<R>, &R)>| {
            rows.into_iter()
                .map(|(id, row)| {
                    format!(
                        "{id:?}: {}",
                        format!("{row:?}").chars().take(256).collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
        };
        format!(
            "actual={} expected={} missing={} {:?}; extra={} {:?}; changed={} {:?}",
            self.len(),
            expected.len(),
            missing.clone().count(),
            describe(missing.take(8).collect()),
            extra.clone().count(),
            describe(extra.take(8).collect()),
            changed.clone().count(),
            describe(changed.take(8).collect())
        )
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
    #[model(name = "required_rows_control")]
    struct Row {
        #[model(key)]
        key: String,
        value: String,
    }
    #[test]
    fn decoding_a_tiny_slice_does_not_charge_unrelated_retained_arrow_bytes() {
        let rows = vec![
            Row {
                key: "small".into(),
                value: "v".into(),
            },
            Row {
                key: "large".into(),
                value: "x".repeat(1 << 20),
            },
        ];
        let batch = Row::encode(&rows).unwrap().slice(0, 1);
        assert!(batch.get_array_memory_size() > 1 << 20);
        let budget = ResourceBudget::fixed(1 << 15).unwrap();
        let mut output = Rows::<Row>::new(&budget);
        output.decode(&batch).unwrap();
        assert_eq!(output.get(rows[0].id()), Some(&rows[0]));
        assert!(budget.reserved() < 1 << 15);
    }
    #[test]
    fn decoder_scratch_is_released_before_large_rows_enter_retained_state() {
        let row = Row {
            key: "large".into(),
            value: "x".repeat(1 << 20),
        };
        let batch = Row::encode(std::slice::from_ref(&row)).unwrap();
        let budget = ResourceBudget::fixed(7 << 19).unwrap();
        // The reader still owns Arrow while Rows owns the decoded vector and map handoff.
        let arrow = budget
            .reserve("reader-arrow-control", batch.get_array_memory_size())
            .unwrap();
        let mut output = Rows::new(&budget);
        output.decode(&batch).unwrap();
        assert_eq!(output.get(row.id()), Some(&row));
        drop(output);
        assert_eq!(budget.reserved(), arrow.size());
        drop(arrow);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn required_lookup_preserves_error_domains_conflicts_and_retained_admission() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let row = Row {
            key: "key".into(),
            value: "original".into(),
        };
        let mut rows = Rows::new(&budget);
        assert_eq!(
            rows.required(row.id(), || {
                crate::domain::obligation::ObligationKind::MissingEvidence
            }),
            Err(crate::domain::obligation::ObligationKind::MissingEvidence)
        );
        assert_eq!(
            rows.required(row.id(), || "different domain"),
            Err("different domain")
        );
        rows.insert(row.clone()).unwrap();
        let reserved = budget.reserved();
        assert!(reserved > 0);
        assert_eq!(rows.required(row.id(), || "missing").unwrap(), &row);
        rows.insert(row.clone()).unwrap();
        assert_eq!(budget.reserved(), reserved);
        assert!(matches!(
            rows.insert(Row {
                value: "conflict".into(),
                ..row.clone()
            }),
            Err(ModelError::Conflict("required_rows_control"))
        ));
        assert_eq!(rows.get(row.id()), Some(&row));
        drop(rows);
        assert_eq!(budget.reserved(), 0);
        let tiny = ResourceBudget::fixed(1).unwrap();
        let mut rows = Rows::new(&tiny);
        assert!(matches!(rows.insert(row), Err(ModelError::Resource { .. })));
        assert!(rows.is_empty());
        assert_eq!(tiny.reserved(), 0);
    }
}
