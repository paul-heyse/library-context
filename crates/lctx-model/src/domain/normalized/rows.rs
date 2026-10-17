//! Canonical typed algorithm output with charged retained state.
use crate::domain::{
    charged::{ChargedMap, StateCharge},
    batching::TransferLimits,
    resources::ResourceBudget,
    *,
};

/// Canonical validated rows and their transferred pool reservation. Only model
/// canonical owners can construct this carrier; no mutable raw-row escape is exposed.
pub struct CanonicalBatch<R: Record> {
    rows: Vec<R>,
    charge: StateCharge,
}
impl<R: Record> CanonicalBatch<R> {
    pub fn into_batch(self, model: &ValidatedModel, budget: &ResourceBudget, limits: TransferLimits) -> Result<Batch<R>, ModelError> {
        if !self.charge.budget().is_some_and(|source| source.shares_pool(budget)) {
            return Err(ModelError::Conflict("canonical batch foreign receiving pool"));
        }
        if self.rows.len() > limits.rows || self.rows.iter().any(|row| row.row_bytes() > limits.max_row)
            || (self.rows.len() > 1 && self.rows.iter().fold(0usize, |sum, row| sum.saturating_add(row.row_bytes())) > limits.bytes) {
            return Err(ModelError::Schema("canonical output transfer bounds"));
        }
        Batch::from_canonical(model, self.rows, Box::new(self.charge))
    }
}
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
    pub fn shares_backing(&self, other: Self) -> bool {
        std::ptr::eq(self.rows, other.rows)
    }
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
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &'a R> + ExactSizeIterator + use<'a, R> {
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
    /// Move the next canonical bounded prefix, including its heap reservations. The
    /// destination vector is admitted before allocation; removing the source map entry
    /// releases only its obsolete node, never the rich payload being transferred.
    pub fn append_owned(&mut self, other: &mut Self) -> Result<(), ModelError> {
        while let Some(id) = other.rows.first_key_value().map(|(id, _)| *id) {
            if let Some(previous) = self.rows.get(&id) {
                if previous != other.rows.get(&id).expect("observed canonical row") { return Err(ModelError::Conflict(R::NAME)); }
                drop(other.rows.remove(&mut other.charge, &id)?);
            } else {
                let row = other.rows.remove(&mut other.charge, &id)?.expect("observed canonical row");
                self.rows.insert_owned(&mut self.charge, id, row)?;
            }
        }
        Ok(())
    }
    pub fn take_batch(&mut self, max_rows: usize, max_bytes: usize) -> Result<Option<CanonicalBatch<R>>, ModelError> {
        take_canonical_batch(&mut self.rows, &mut self.charge, max_rows, max_bytes)
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
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &R> + ExactSizeIterator {
        self.rows.values()
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Charged vocabulary maps do not carry Rows' insertion validation contract. Check
/// only the prefix that will actually move before granting the canonical carrier.
pub(crate) fn take_checked_batch<R: Record>(rows: &mut ChargedMap<Id<R>, R>, charge: &mut StateCharge, max_rows: usize, max_bytes: usize) -> Result<Option<CanonicalBatch<R>>, ModelError> {
    let mut bytes = 0usize;
    for (ordinal, (key, row)) in rows.iter().take(max_rows).enumerate() {
        let next = row.row_bytes();
        if ordinal != 0 && bytes.saturating_add(next) > max_bytes { break; }
        row.validate()?;
        if row.id() != *key { return Err(ModelError::Conflict("canonical vocabulary nominal key")); }
        bytes = bytes.saturating_add(next);
    }
    take_canonical_batch(rows, charge, max_rows, max_bytes)
}
/// Consume one canonical charged map prefix, shared by normalized and vocabulary output.
pub(crate) fn take_canonical_batch<R: Record>(rows: &mut ChargedMap<Id<R>, R>, charge: &mut StateCharge, max_rows: usize, max_bytes: usize) -> Result<Option<CanonicalBatch<R>>, ModelError> {
        if rows.is_empty() { return Ok(None); }
        if max_rows == 0 || max_bytes == 0 { return Err(ModelError::Schema("canonical output batch bounds")); }
        let mut count = 0usize;
        let mut bytes = 0usize;
        for row in rows.values().take(max_rows) {
            let next = row.row_bytes();
            if count != 0 && bytes.saturating_add(next) > max_bytes { break; }
            count += 1;
            bytes = bytes.saturating_add(next);
        }
        let budget = charge.budget().expect("rows budget");
        let mut receiving = StateCharge::new(budget, "canonical output transfer");
        receiving.grow(count.checked_mul(size_of::<R>()).ok_or(ModelError::Schema("canonical output size"))?)?;
        let mut owned = Vec::with_capacity(count);
        for _ in 0..count {
            let id = *rows.first_key_value().expect("canonical prefix").0;
            let row = rows.remove(charge, &id)?.expect("canonical prefix");
            owned.push(row.into_inner(&mut receiving)?);
        }
        Ok(Some(CanonicalBatch { rows: owned, charge: receiving }))

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
    fn consuming_batches_move_heap_backing_in_order_and_keep_it_charged() {
        let budget = ResourceBudget::fixed(3 << 20).unwrap();
        let mut rows = Rows::new(&budget);
        let mut pointers = std::collections::BTreeMap::new();
        for key in ["gamma", "alpha", "beta"] {
            let row = Row { key: key.into(), value: "x".repeat(512 << 10) };
            pointers.insert(row.id(), row.value.as_ptr() as usize);
            rows.insert(row).unwrap();
        }
        let first = rows.take_batch(2, usize::MAX).unwrap().unwrap();
        assert_eq!(first.rows.len(), 2);
        assert_eq!(rows.len(), 1);
        assert!(first.rows.windows(2).all(|pair| pair[0].id() < pair[1].id()));
        for row in &first.rows { assert_eq!(row.value.as_ptr() as usize, pointers[&row.id()]); }
        assert!(budget.reserved() >= 3 * (512 << 10));
        drop(rows);
        assert!(budget.reserved() >= 2 * (512 << 10));
        drop(first);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn consuming_batch_refuses_before_mutation_and_rejects_a_foreign_pool() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut rows = Rows::new(&budget);
        let row = Row { key: "key".into(), value: "payload".into() };
        rows.insert(row.clone()).unwrap();
        let held = budget.reserve("canonical transfer refusal control", budget.limit() - budget.reserved()).unwrap();
        assert!(matches!(rows.take_batch(1, usize::MAX), Err(ModelError::Resource { .. })));
        assert_eq!(rows.get(row.id()), Some(&row));
        drop(held);
        let batch = rows.take_batch(1, usize::MAX).unwrap().unwrap();
        let foreign = ResourceBudget::fixed(1 << 20).unwrap();
        assert!(matches!(batch.into_batch(&crate::domain::model().unwrap(), &foreign, TransferLimits::default()), Err(ModelError::Conflict("canonical batch foreign receiving pool"))));
        drop(rows);
        assert_eq!(budget.reserved(), 0);
        assert_eq!(foreign.reserved(), 0);
    }
    #[test]
    fn consuming_append_refuses_conflicts_and_deduplicates_equal_rows() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut first = Rows::new(&budget);
        let mut second = Rows::new(&budget);
        let row = Row { key: "same".into(), value: "first".into() };
        first.insert(row.clone()).unwrap(); second.insert(row.clone()).unwrap();
        first.append_owned(&mut second).unwrap();
        assert_eq!(first.len(), 1); assert!(second.is_empty());
        second.insert(Row { value: "conflicting".into(), ..row }).unwrap();
        assert!(matches!(first.append_owned(&mut second), Err(ModelError::Conflict("required_rows_control"))));
        assert_eq!(second.len(), 1);
        drop((first, second)); assert_eq!(budget.reserved(), 0);
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

/// Resolve the exact typed input port once, then visit its ordered nominal demand.
/// Missing physical rows stay with the consumer; no rich union scan or key copy is needed.
pub(crate) fn visit_selected_ids<R: Record, I: Iterator<Item = [u8; 16]>>(
    inputs: &[ValidationInput],
    epoch: Option<stages::PublicationBoundary>,
    keys: &mut dyn FnMut(usize) -> Result<I, ModelError>,
    visit: &mut impl FnMut(Id<R>) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let prefix = epoch
        .or_else(|| stages::is_vocabulary(R::NAME).then_some(stages::PublicationBoundary::Facts));
    let mut bindings = inputs.iter().enumerate().filter(|(_, input)| {
        input.type_id() == std::any::TypeId::of::<R>()
            && input.name() == R::NAME
            && input.prefix() == prefix
    });
    let Some((table, _)) = bindings.next() else {
        return Ok(());
    };
    if bindings.next().is_some() {
        return Err(ModelError::Conflict(
            "selected copy immutable namespace ambiguity",
        ));
    }
    visit_ordered_port_ids(table, keys, visit)
}
/// Decode one already selected physical input port's ordered nominal demand.
/// The semantic adapter owns exact-port versus explicitly merged-port selection.
pub(crate) fn visit_ordered_port_ids<R: Record, I: Iterator<Item = [u8; 16]>>(
    table: usize,
    keys: &mut dyn FnMut(usize) -> Result<I, ModelError>,
    visit: &mut impl FnMut(Id<R>) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let mut previous = None;
    for key in keys(table)? {
        if previous.is_some_and(|prior| prior >= key) {
            return Err(ModelError::Conflict(
                "selected copy keys are not strictly ordered",
            ));
        }
        previous = Some(key);
        let id = serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new(key.into_iter()))
        .map_err(ModelError::codec)?;
        visit(id)?;
    }
    Ok(())
}

/// Copy only actual hydrated rows, preserving the nominal JOIN realization of absent keys.
pub(crate) fn copy_selected_rows<R: Record, I: Iterator<Item = [u8; 16]>>(
    source: &Rows<R>,
    target: &mut Rows<R>,
    inputs: &[ValidationInput],
    epoch: Option<stages::PublicationBoundary>,
    keys: &mut dyn FnMut(usize) -> Result<I, ModelError>,
) -> Result<(), ModelError> {
    visit_selected_ids::<R, I>(inputs, epoch, keys, &mut |id| {
        if let Some(row) = source.get(id) {
            target.insert_borrowed(row)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod selected_key_controls {
    use super::*;
    use crate::domain::input::{Package, Release};
    #[test]
    fn ordered_selected_keys_preserve_overlap_missing_rows_and_canonical_output() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let rows = [
            Package {
                name: "gamma".into(),
            },
            Package {
                name: "alpha".into(),
            },
            Package {
                name: "beta".into(),
            },
        ];
        let mut source = Rows::new(&budget);
        for row in rows.iter().rev() {
            source.insert_borrowed(row).unwrap();
        }
        let inputs = [
            ValidationInput::of::<Release>(&["id"]),
            ValidationInput::of::<Package>(&["id"]),
        ];
        let keys = [*rows[0].id().bytes(), *rows[2].id().bytes(), [255; 16]]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let overlap = [*rows[1].id().bytes(), *rows[2].id().bytes()]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut first = Rows::new(&budget);
        copy_selected_rows(&source, &mut first, &inputs, None, &mut |port| {
            assert_eq!(port, 1);
            Ok(keys.iter().copied())
        })
        .unwrap();
        let mut second = Rows::new(&budget);
        copy_selected_rows(&source, &mut second, &inputs, None, &mut |port| {
            assert_eq!(port, 1);
            Ok(overlap.iter().copied())
        })
        .unwrap();
        assert_eq!(
            first
                .iter()
                .map(|row| row.name.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["gamma", "beta"].into()
        );
        assert_eq!(
            second
                .iter()
                .map(|row| row.name.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["alpha", "beta"].into()
        );
        assert!(
            first
                .iter()
                .map(Record::id)
                .collect::<Vec<_>>()
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert_eq!(first.get(rows[2].id()), second.get(rows[2].id()));
        drop((source, first, second));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn selected_key_adapter_refuses_ambiguous_ports_and_noncanonical_demand() {
        let input = ValidationInput::of::<Package>(&["id"]);
        let mut touched = false;
        assert!(
            visit_selected_ids::<Package, _>(
                &[input.clone(), input.clone()],
                None,
                &mut |_| {
                    touched = true;
                    Ok(std::iter::empty())
                },
                &mut |_| Ok(())
            )
            .is_err()
        );
        assert!(!touched, "ambiguous namespace refuses before demand");
        for keys in [[[2; 16], [1; 16]], [[1; 16], [1; 16]]] {
            assert!(
                visit_selected_ids::<Package, _>(
                    std::slice::from_ref(&input),
                    None,
                    &mut |_| Ok(keys.into_iter()),
                    &mut |_| Ok(())
                )
                .is_err()
            );
        }
    }
}
