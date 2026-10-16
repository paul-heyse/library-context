//! Retained allocation bound for native catalog values from the pinned SDK and
//! model/serde producers. This is owned backing storage, not allocator RSS.
use lctx_model::domain::ModelError;
use std::{
    mem::{align_of, size_of, size_of_val},
    ops::Bound,
};
use surrealdb::types::{Geometry, RecordIdKey, Value};

fn add(a: usize, b: usize) -> Result<usize, ModelError> {
    a.checked_add(b)
        .ok_or(ModelError::Schema("native retained allocation overflow"))
}
fn mul(a: usize, b: usize) -> Result<usize, ModelError> {
    a.checked_mul(b)
        .ok_or(ModelError::Schema("native retained allocation overflow"))
}
fn sum(mut items: impl Iterator<Item = Result<usize, ModelError>>) -> Result<usize, ModelError> {
    items.try_fold(0, |total, item| add(total, item?))
}
fn slots<T>(values: &Vec<T>) -> Result<usize, ModelError> {
    mul(values.capacity(), size_of::<T>())
}

// Pinned alloc B=6: a leaf contains 11 keys/values, parent pointer, u16
// parent-index and length; an internal node adds 12 child pointers. Every
// nonroot node has >=5 entries. One possible partial bulk-build spine is
// bounded by binary height (larger than the actual B=6 tree's height).
fn tree<K, V>(len: usize) -> Result<usize, ModelError> {
    let align = align_of::<K>()
        .max(align_of::<V>())
        .max(align_of::<usize>());
    let node = add(
        add(
            mul(11, add(size_of::<K>(), size_of::<V>())?)?,
            13 * size_of::<usize>() + 4,
        )?,
        7 * align,
    )?;
    let spine = (usize::BITS - len.leading_zeros()) as usize;
    mul(add(len.div_ceil(5).max(1), spine)?, node)
}
fn array(values: &surrealdb::types::Array) -> Result<usize, ModelError> {
    add(slots(values)?, sum(values.iter().map(heap))?)
}
fn object(values: &surrealdb::types::Object) -> Result<usize, ModelError> {
    add(
        tree::<String, Value>(values.len())?,
        sum(values
            .iter()
            .map(|(key, value)| add(key.capacity(), heap(value)?)))?,
    )
}
fn key(value: &RecordIdKey) -> Result<usize, ModelError> {
    match value {
        RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => Ok(0),
        RecordIdKey::String(value) => Ok(value.capacity()),
        RecordIdKey::Array(values) => array(values),
        RecordIdKey::Object(values) => object(values),
        RecordIdKey::Range(value) => add(
            size_of_val(value.as_ref()),
            add(key_bound(&value.start)?, key_bound(&value.end)?)?,
        ),
    }
}
fn key_bound(value: &Bound<RecordIdKey>) -> Result<usize, ModelError> {
    match value {
        Bound::Included(value) | Bound::Excluded(value) => key(value),
        Bound::Unbounded => Ok(0),
    }
}
fn bound(value: &Bound<Value>) -> Result<usize, ModelError> {
    match value {
        Bound::Included(value) | Bound::Excluded(value) => heap(value),
        Bound::Unbounded => Ok(0),
    }
}
macro_rules! polygon_storage {
    ($polygon:expr) => {{
        let value = $polygon;
        // The public API hides interiors Vec capacity. Pinned SDK Result
        // collect/model serde capacity is <2*n, minimum four. Arbitrarily
        // externally overallocated polygons are outside this producer profile.
        let interiors = value.interiors();
        let bytes = if interiors.is_empty() {
            0
        } else {
            mul(2, size_of_val(interiors))?.max(mul(4, size_of_val(&interiors[0]))?)
        };
        add(
            slots(&value.exterior().0)?,
            add(bytes, sum(interiors.iter().map(|line| slots(&line.0)))?)?,
        )
    }};
}

fn geometry(value: &Geometry) -> Result<usize, ModelError> {
    match value {
        Geometry::Point(_) => Ok(0),
        Geometry::Line(value) => slots(&value.0),
        Geometry::Polygon(value) => polygon_storage!(value),
        Geometry::MultiPoint(value) => slots(&value.0),
        Geometry::MultiLine(value) => add(
            slots(&value.0)?,
            sum(value.0.iter().map(|line| slots(&line.0)))?,
        ),
        Geometry::MultiPolygon(value) => add(
            slots(&value.0)?,
            sum(value.0.iter().map(|polygon| polygon_storage!(polygon)))?,
        ),
        Geometry::Collection(values) => add(slots(values)?, sum(values.iter().map(geometry))?),
    }
}
fn heap(value: &Value) -> Result<usize, ModelError> {
    match value {
        Value::None
        | Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Duration(_)
        | Value::Datetime(_)
        | Value::Uuid(_) => Ok(0),
        Value::String(value) => Ok(value.capacity()),
        // materialize_native_bytes creates exact-length owned backing. Pinned
        // bytes 1.12.1's possible lazy shared owner is {buf,cap,ref_cnt}, three
        // pointer-sized fields. The original ingress lease covers old backing
        // until materialization finishes under this replacement admission.
        Value::Bytes(value) => {
            if value.is_empty() {
                Ok(0)
            } else {
                add(value.len(), 3 * size_of::<usize>())
            }
        }
        Value::Array(values) => array(values),
        Value::Object(values) => object(values),
        Value::Set(values) => add(
            tree::<Value, ()>(values.len())?,
            sum(values.iter().map(heap))?,
        ),
        Value::Table(value) => Ok(value.capacity()),
        Value::RecordId(value) => add(value.table.capacity(), key(&value.key)?),
        Value::File(value) => add(value.bucket.capacity(), value.key.capacity()),
        Value::Range(value) => add(
            size_of_val(value.as_ref()),
            add(bound(&value.start)?, bound(&value.end)?)?,
        ),
        Value::Geometry(value) => geometry(value),
        Value::Regex(_) => Err(ModelError::Schema(
            "native catalog retained regex requires a qualified compiler owner",
        )),
    }
}
/// Bound the native retained representation, including unused container slots.
/// Call before transferring a decoded row from its SDK lease to a native owner.
/// These catalog producers do not emit compiled Regex or arbitrarily externally
/// overallocated geometry. Materialize shared Bytes under this new reservation
/// while the original source allocation owner remains live.
pub(crate) fn native_retained_bytes(value: &Value) -> Result<usize, ModelError> {
    add(size_of::<Value>(), heap(value)?)
}

fn key_has_bytes(value: &RecordIdKey) -> bool {
    match value {
        RecordIdKey::Array(values) => values.iter().any(has_bytes),
        RecordIdKey::Object(values) => values.values().any(has_bytes),
        RecordIdKey::Range(value) => {
            [&value.start, &value.end]
                .into_iter()
                .any(|bound| match bound {
                    Bound::Included(key) | Bound::Excluded(key) => key_has_bytes(key),
                    Bound::Unbounded => false,
                })
        }
        _ => false,
    }
}
fn has_bytes(value: &Value) -> bool {
    match value {
        Value::Bytes(_) => true,
        Value::Array(values) => values.iter().any(has_bytes),
        Value::Object(values) => values.values().any(has_bytes),
        Value::Set(values) => values.iter().any(has_bytes),
        Value::RecordId(value) => key_has_bytes(&value.key),
        Value::Range(value) => [&value.start, &value.end]
            .into_iter()
            .any(|bound| match bound {
                Bound::Included(value) | Bound::Excluded(value) => has_bytes(value),
                Bound::Unbounded => false,
            }),
        _ => false,
    }
}
fn materialize_key(value: &mut RecordIdKey) {
    match value {
        RecordIdKey::Array(values) => values.iter_mut().for_each(materialize_native_bytes),
        RecordIdKey::Object(values) => values.values_mut().for_each(materialize_native_bytes),
        RecordIdKey::Range(value) => {
            for bound in [&mut value.start, &mut value.end] {
                if let Bound::Included(value) | Bound::Excluded(value) = bound {
                    materialize_key(value);
                }
            }
        }
        _ => {}
    }
}
/// Complete destination admission first, then call while the original source
/// leases remain live. A Set containing Bytes rebuilds via the existing BTreeSet
/// insertion API: destination nodes are covered by new admission, original nodes
/// by the original source leases, with both held across the move. Byte contents
/// and set ordering/identity are unchanged.
pub(crate) fn materialize_native_bytes(value: &mut Value) {
    match value {
        Value::Bytes(value) => *value = prost::bytes::Bytes::copy_from_slice(value.as_ref()).into(),
        Value::Array(values) => values.iter_mut().for_each(materialize_native_bytes),
        Value::Object(values) => values.values_mut().for_each(materialize_native_bytes),
        Value::Set(values) if values.iter().any(has_bytes) => {
            let mut next = std::collections::BTreeSet::new();
            for mut item in std::mem::take(values).into_inner() {
                materialize_native_bytes(&mut item);
                next.insert(item);
            }
            *values = next.into();
        }
        Value::RecordId(value) => materialize_key(&mut value.key),
        Value::Range(value) => {
            for bound in [&mut value.start, &mut value.end] {
                if let Bound::Included(value) | Bound::Excluded(value) = bound {
                    materialize_native_bytes(value);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn unused_vector_capacity_is_admitted_and_nested_set_bytes_keep_identity() {
        let mut values = Vec::with_capacity(1024);
        values.push(Value::from_t(1));
        let value = Value::Array(values.into());
        assert!(native_retained_bytes(&value).unwrap() >= 1024 * size_of::<Value>());
        let backing = prost::bytes::Bytes::from(vec![11; 8192]);
        let slice = backing.slice(1..4);
        let pointer = slice.as_ptr();
        let mut value = Value::Set(
            std::collections::BTreeSet::from([Value::Bytes(slice.into()), Value::from_t(3)]).into(),
        );
        let before = value.clone();
        let admitted = native_retained_bytes(&value).unwrap();
        materialize_native_bytes(&mut value);
        assert_eq!(value, before);
        assert!(native_retained_bytes(&value).unwrap() <= admitted);
        let Value::Set(values) = &value else {
            panic!("set");
        };
        for value in values {
            if let Value::Bytes(bytes) = value {
                assert_ne!(bytes.as_ptr(), pointer);
            }
        }
    }
}
