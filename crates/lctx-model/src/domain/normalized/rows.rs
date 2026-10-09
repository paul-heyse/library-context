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
/// Immutable typed membership over charged rows. Selected IDs are borrowed from caller-owned,
/// charged storage; construction neither clones rich rows nor allocates another membership set.
pub struct RowsView<'a, R: Record> {
    rows: &'a Rows<R>,
    selected: Option<&'a [Id<R>]>,
}
impl<R: Record> Copy for RowsView<'_, R> {}
impl<R: Record> Clone for RowsView<'_, R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, R: Record> RowsView<'a, R> {
    /// Select an exact canonical set. Duplicate, unordered and absent IDs are refused, so
    /// cardinality and iteration never silently shrink or depend on physical hydration order.
    pub fn selected(rows: &'a Rows<R>, ids: &'a [Id<R>]) -> Result<Self, ModelError> {
        if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ModelError::Invalid(format!(
                "{} selected row IDs are not strictly ordered",
                R::NAME
            )));
        }
        if ids.iter().any(|id| rows.get(*id).is_none()) {
            return Err(ModelError::Invalid(format!(
                "{} selected row premise absent",
                R::NAME
            )));
        }
        Ok(Self {
            rows,
            selected: Some(ids),
        })
    }
    pub fn get(&self, id: Id<R>) -> Option<&'a R> {
        if self
            .selected
            .is_some_and(|ids| ids.binary_search(&id).is_err())
        {
            return None;
        }
        self.rows.get(id)
    }
    /// Keep missing-premise meaning with the consuming domain, including unselected rows.
    pub fn required<E>(&self, id: Id<R>, missing: impl FnOnce() -> E) -> Result<&'a R, E> {
        self.get(id).ok_or_else(missing)
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &'a R> + ExactSizeIterator {
        match self.selected {
            Some(ids) => RowsViewIter::Selected {
                rows: self.rows,
                ids: ids.iter(),
            },
            None => RowsViewIter::Whole(self.rows.rows.values()),
        }
    }
    pub fn same(&self, other: &Rows<R>) -> bool {
        self.iter().eq(other.iter())
    }
    /// Bounded replay diagnostics over exactly the visible row membership.
    pub(crate) fn difference(&self, expected: &Rows<R>) -> String {
        let missing = || expected.iter().filter(|row| self.get(row.id()).is_none());
        let extra = || self.iter().filter(|row| expected.get(row.id()).is_none());
        let changed = || {
            self.iter()
                .filter(|row| expected.get(row.id()).is_some_and(|other| other != *row))
        };
        let describe = |rows: Vec<&R>| {
            rows.into_iter()
                .map(|row| {
                    format!(
                        "{:?}: {}",
                        row.id(),
                        format!("{row:?}").chars().take(256).collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
        };
        format!(
            "actual={} expected={} missing={} {:?}; extra={} {:?}; changed={} {:?}",
            self.len(),
            expected.len(),
            missing().count(),
            describe(missing().take(8).collect()),
            extra().count(),
            describe(extra().take(8).collect()),
            changed().count(),
            describe(changed().take(8).collect())
        )
    }
    pub fn len(&self) -> usize {
        self.selected
            .map_or_else(|| self.rows.len(), <[Id<R>]>::len)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl<'a, R: Record> From<&'a Rows<R>> for RowsView<'a, R> {
    fn from(rows: &'a Rows<R>) -> Self {
        Self {
            rows,
            selected: None,
        }
    }
}
enum RowsViewIter<'a, R: Record> {
    Whole(std::collections::btree_map::Values<'a, Id<R>, R>),
    Selected {
        rows: &'a Rows<R>,
        ids: std::slice::Iter<'a, Id<R>>,
    },
}
impl<'a, R: Record> Iterator for RowsViewIter<'a, R> {
    type Item = &'a R;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Whole(rows) => rows.next(),
            Self::Selected { rows, ids } => ids
                .next()
                .map(|id| rows.get(*id).expect("validated row selection")),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<R: Record> DoubleEndedIterator for RowsViewIter<'_, R> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Whole(rows) => rows.next_back(),
            Self::Selected { rows, ids } => ids
                .next_back()
                .map(|id| rows.get(*id).expect("validated row selection")),
        }
    }
}
impl<R: Record> ExactSizeIterator for RowsViewIter<'_, R> {
    fn len(&self) -> usize {
        match self {
            Self::Whole(rows) => rows.len(),
            Self::Selected { ids, .. } => ids.len(),
        }
    }
}
impl<R: Record> Rows<R> {
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            rows: Default::default(),
            charge: StateCharge::new(budget, R::NAME),
        }
    }
    pub fn view(&self) -> RowsView<'_, R> {
        self.into()
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
    /// Copy an already decoded immutable premise, reserving clone scratch before allocation.
    /// Used when an existing finite kernel requires owned rows for its current partition.
    pub fn insert_borrowed(&mut self, row: &R) -> Result<Id<R>, ModelError> {
        let _scratch = self.charge.budget().expect("rows budget").reserve(
            "selected-row-clone",
            size_of::<R>().saturating_add(row.heap_bytes()),
        )?;
        self.insert(row.clone())
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
        self.view().difference(expected)
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
    fn selected_views_preserve_order_identity_cardinality_and_missing_domains() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut rows = Rows::new(&budget);
        for key in ["a", "b", "c"] {
            rows.insert(Row {
                key: key.into(),
                value: key.into(),
            })
            .unwrap();
        }
        let all = rows.iter().map(Record::id).collect::<Vec<_>>();
        // Selection storage belongs to the caller and stays charged across borrowed views.
        let _selection = budget
            .reserve("row-selection-control", 2 * size_of::<Id<Row>>())
            .unwrap();
        let ids = [all[0], all[2]];
        let reserved = budget.reserved();
        let view = RowsView::selected(&rows, &ids).unwrap();
        assert_eq!(budget.reserved(), reserved);
        assert_eq!(view.len(), 2);
        assert_eq!(view.iter().len(), 2);
        assert_eq!(view.iter().map(Record::id).collect::<Vec<_>>(), ids);
        assert_eq!(
            view.iter().rev().map(Record::id).collect::<Vec<_>>(),
            [all[2], all[0]]
        );
        assert!(std::ptr::eq(
            view.get(all[0]).unwrap(),
            rows.get(all[0]).unwrap()
        ));
        assert!(view.get(all[1]).is_none());
        assert_eq!(
            view.required(all[1], || "outside selected domain"),
            Err("outside selected domain")
        );
        assert_eq!(rows.view().iter().map(Record::id).collect::<Vec<_>>(), all);
        let empty = RowsView::selected(&rows, &[]).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.iter().len(), 0);
        assert!(empty.get(all[0]).is_none());
        assert!(!view.same(&rows));
        let difference = view.difference(&rows);
        assert!(difference.starts_with("actual=2 expected=3 missing=1"));
        assert!(difference.contains("extra=0 []"));
        assert!(difference.contains("changed=0 []"));
        assert_eq!(budget.reserved(), reserved);
    }
    #[test]
    fn selected_views_refuse_duplicate_unordered_and_absent_membership() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut rows = Rows::new(&budget);
        for key in ["a", "b"] {
            rows.insert(Row {
                key: key.into(),
                value: String::new(),
            })
            .unwrap();
        }
        let ids = rows.iter().map(Record::id).collect::<Vec<_>>();
        let absent = Row {
            key: "absent".into(),
            value: String::new(),
        }
        .id();
        assert!(RowsView::selected(&rows, &[ids[0], ids[0]]).is_err());
        assert!(RowsView::selected(&rows, &[ids[1], ids[0]]).is_err());
        assert!(RowsView::selected(&rows, &[absent]).is_err());
        assert_eq!(rows.len(), 2);
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
