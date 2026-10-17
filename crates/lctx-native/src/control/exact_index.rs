//! Candidate positions for the scalar identity branches whose equality is exact.
//!
//! Compound record keys deliberately retain an ordered scan: native Value numeric
//! equality is not a lawful generic hash/ordering key. Every candidate is retained,
//! without assuming that native equality authorizes representative membership.
use super::*;
use std::collections::HashMap;
use surrealdb::types::{RecordIdKey, Uuid};

#[derive(Default)]
struct Table {
    strings: HashMap<String, Vec<usize>>,
    numbers: HashMap<i64, Vec<usize>>,
    uuids: HashMap<Uuid, Vec<usize>>,
}
impl Table {
    fn insert(&mut self, key: &RecordIdKey, position: usize) {
        match key {
            RecordIdKey::String(key) => match self.strings.get_mut(key.as_str()) {
                Some(positions) => positions.push(position),
                None => {
                    self.strings.insert(key.clone(), vec![position]);
                }
            },
            RecordIdKey::Number(key) => self.numbers.entry(*key).or_default().push(position),
            RecordIdKey::Uuid(key) => self.uuids.entry(*key).or_default().push(position),
            _ => unreachable!("scalar branch checked before table lookup"),
        }
    }
}
#[derive(Default)]
pub(super) struct ExactRecordIndex {
    tables: HashMap<String, Table>,
    compound: Vec<usize>,
    unkeyed: Vec<usize>,
}
impl ExactRecordIndex {
    pub(super) fn insert(&mut self, id: Option<&RecordId>, position: usize) {
        let Some(id) = id else {
            self.unkeyed.push(position);
            return;
        };
        if !matches!(
            id.key,
            RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_)
        ) {
            self.compound.push(position);
            return;
        }
        match self.tables.get_mut(id.table.as_str()) {
            Some(table) => table.insert(&id.key, position),
            None => {
                let mut table = Table::default();
                table.insert(&id.key, position);
                self.tables.insert(id.table.as_str().to_owned(), table);
            }
        }
    }
    pub(super) fn candidates(&self, id: Option<&RecordId>) -> &[usize] {
        let Some(id) = id else {
            return &self.unkeyed;
        };
        if !matches!(
            id.key,
            RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_)
        ) {
            return &self.compound;
        }
        let Some(table) = self.tables.get(id.table.as_str()) else {
            return &[];
        };
        match &id.key {
            RecordIdKey::String(key) => table.strings.get(key),
            RecordIdKey::Number(key) => table.numbers.get(key),
            RecordIdKey::Uuid(key) => table.uuids.get(key),
            _ => unreachable!("scalar branch checked above"),
        }
        .map_or(&[], Vec::as_slice)
    }
}
/// Conservative backing allowance for the candidate index and simultaneous output
/// positions/containers. Tables, subtype maps, one-element Vec allocations and hash
/// capacity are covered per entry, even when each identity belongs to its own table.
pub(super) fn entry_workspace(value: &Value) -> usize {
    crate::loader::native_bytes(value).saturating_add(1024)
}
pub(super) fn record(value: &Value) -> Option<&RecordId> {
    if let Value::RecordId(id) = value {
        Some(id)
    } else {
        None
    }
}
pub(super) fn field<'a>(value: &'a Value, name: &str) -> Option<&'a RecordId> {
    value
        .as_object()
        .and_then(|row| row.get(name))
        .and_then(record)
}
pub(super) fn stable_unique(values: Vec<Value>, name: Option<&str>) -> Vec<Value> {
    let mut index = ExactRecordIndex::default();
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        let id = name.map_or_else(|| record(&value), |name| field(&value, name));
        if !index
            .candidates(id)
            .iter()
            .any(|&position| output[position] == value)
        {
            index.insert(id, output.len());
            output.push(value);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::{Geometry, Number, RecordIdKey};

    fn legacy_unique(values: Vec<Value>) -> Vec<Value> {
        let mut output = Vec::new();
        for value in values {
            if !output.contains(&value) {
                output.push(value);
            }
        }
        output
    }
    fn compound(value: Value) -> RecordId {
        RecordId::new("compound", RecordIdKey::Array(vec![value].into()))
    }
    #[test]
    fn exact_coalescing_matches_ordered_scan_for_nested_numeric_geometry_and_scalar_tags() {
        let uuid = Uuid::new_v4();
        let geometries = [
            Geometry::Collection(Vec::new()),
            Geometry::Collection(vec![Geometry::Collection(Vec::new())]),
        ];
        let values = vec![
            Value::RecordId(RecordId::new("scalar", "1")),
            Value::RecordId(RecordId::new("scalar", 1i64)),
            Value::RecordId(RecordId::new("scalar", uuid)),
            Value::RecordId(RecordId::new("other", "1")),
            Value::RecordId(compound(Value::Number(Number::Int(1)))),
            Value::RecordId(compound(Value::Number(Number::Float(1.0)))),
            Value::Array(vec![Value::Number(Number::Int(1))].into()),
            Value::Array(vec![Value::Number(Number::Float(1.0))].into()),
            Value::Number(Number::Float(-0.0)),
            Value::Number(Number::Float(0.0)),
            Value::Number(Number::Float(f64::from_bits(0x7ff8_0000_0000_0001))),
            Value::Number(Number::Float(f64::from_bits(0x7ff8_0000_0000_0002))),
            Value::Geometry(geometries[0].clone()),
            Value::Geometry(geometries[1].clone()),
        ];
        let mut repeated = values.clone();
        repeated.extend(values.into_iter().rev());
        assert_eq!(
            stable_unique(repeated.clone(), None),
            legacy_unique(repeated)
        );
        let id = RecordId::new("entity", "same");
        let row = |payload: Value| {
            let mut row = Object::new();
            row.insert("id", id.clone());
            row.insert("payload", payload);
            Value::Object(row)
        };
        let rows = vec![
            row(Value::Number(Number::Int(1))),
            row(Value::Number(Number::Float(1.0))),
            row(Value::Geometry(geometries[0].clone())),
            row(Value::Geometry(geometries[1].clone())),
        ];
        assert_eq!(stable_unique(rows.clone(), Some("id")), legacy_unique(rows));
    }
    #[test]
    fn immutable_compound_identity_keeps_first_equal_payload_and_refuses_unequal_payload() {
        let id_int = compound(Value::Number(Number::Int(1)));
        let id_float = compound(Value::Number(Number::Float(1.0)));
        assert_eq!(
            id_int, id_float,
            "revealing pinned cross-representation equality"
        );
        let row = |id: RecordId, payload: i64| {
            let mut row = Object::new();
            row.insert("id", id);
            row.insert("payload", payload);
            Value::Object(row)
        };
        let first = row(RecordId::new("entity", "before"), 1);
        let selected = row(id_int.clone(), 2);
        let last = row(RecordId::new("entity", "after"), 3);
        let equal = row(id_float.clone(), 2);
        assert_eq!(
            super::super::coalesce_immutable_rows(vec![
                first.clone(),
                selected.clone(),
                last.clone(),
                equal
            ])
            .unwrap(),
            vec![first, selected, last]
        );
        assert!(matches!(
            super::super::coalesce_immutable_rows(vec![row(id_int, 2), row(id_float, 4)]),
            Err(ModelError::Conflict("native immutable address collision"))
        ));
    }
    #[test]
    fn preparation_index_retains_all_candidates_and_exact_ordered_first_matches() {
        let candidates = [
            compound(Value::Number(Number::Int(1))),
            compound(Value::Number(Number::Float(1.0))),
            compound(Value::Number(Number::Float(-0.0))),
        ];
        let mut index = ExactRecordIndex::default();
        for (position, id) in candidates.iter().enumerate() {
            index.insert(Some(id), position);
        }
        for query in [
            &candidates[0],
            &candidates[1],
            &compound(Value::Number(Number::Float(0.0))),
        ] {
            assert_eq!(
                index.candidates(Some(query)),
                &[0, 1, 2],
                "compound candidates are never collapsed"
            );
            assert_eq!(
                index
                    .candidates(Some(query))
                    .iter()
                    .copied()
                    .find(|&position| candidates[position] == *query),
                candidates.iter().position(|candidate| candidate == query)
            );
        }
    }
}
