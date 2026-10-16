use std::collections::BTreeMap;
use std::ops::Bound;
use std::str::FromStr;

use anyhow::anyhow;
use surrealdb_protocol::proto::v1 as proto;
use surrealdb_protocol::proto::v1::value::Value as ValueInner;
use surrealdb_protocol::proto::v1::value_bound::Bound as ValueBoundInner;

use crate::{
	Array, Bytes, Datetime, Duration, File, Number, Object, Range, Regex, Set, Table, Uuid, Value,
};

impl TryFrom<Value> for proto::Value {
	type Error = anyhow::Error;

	/// Fallible because the wire's `Duration` counts seconds in an `i64` while
	/// `std::time::Duration` counts them in a `u64`, so the largest durations
	/// this crate can hold have no representation on the wire. Refusing one
	/// beats wrapping it into a negative duration.
	fn try_from(value: Value) -> Result<Self, Self::Error> {
		Ok(match value {
			Value::None => proto::Value::none(),
			Value::Null => proto::Value::null(),
			Value::Bool(b) => proto::Value::bool(b),
			Value::Number(Number::Int(i)) => proto::Value::int64(i),
			Value::Number(Number::Float(f)) => proto::Value::float64(f),
			Value::Number(Number::Decimal(d)) => proto::Value::decimal(d.into()),
			Value::String(s) => proto::Value::string(s),
			Value::Bytes(b) => proto::Value::bytes(b.0),
			Value::Duration(d) => proto::Value::duration(d.0.try_into()?),
			Value::Datetime(dt) => proto::Value::datetime(proto::Datetime::from_chrono(dt.0)),
			Value::Uuid(u) => proto::Value::uuid(u.0.into()),
			Value::Geometry(g) => proto::Value::geometry(g.into()),
			Value::Table(t) => proto::Value::table(t.into_string()),
			Value::RecordId(r) => proto::Value::record_id(r.try_into()?),
			Value::File(f) => proto::Value::file(proto::File::new(f.bucket, f.key)),
			Value::Range(r) => proto::Value {
				value: Some(ValueInner::Range(Box::new((*r).try_into()?))),
			},
			Value::Regex(r) => proto::Value::regex(r.0.as_str().to_string()),
			Value::Object(o) => proto::Value::object(o.try_into()?),
			Value::Array(a) => proto::Value::array(a.try_into()?),
			Value::Set(s) => proto::Value::set(s.try_into()?),
		})
	}
}

impl TryFrom<proto::Value> for Value {
	type Error = anyhow::Error;

	fn try_from(value: proto::Value) -> Result<Self, Self::Error> {
		Self::try_from_proto_with_regex(value, &mut |pattern| Ok(Regex::from_str(pattern)?))
	}
}

impl Value {
	/// Decode the existing protobuf value model with an independently owned
	/// regex compiler. The callback is used at every recursive regex value,
	/// including arrays, objects, sets and record/range bounds. Other validation
	/// and conversion semantics are identical to `TryFrom<proto::Value>`.
	///
	/// A bounded caller must authorize compilation before allocating and keep
	/// its resulting allocation reservation as long as the regex remains live.
	pub fn try_from_proto_with_regex(
		value: proto::Value,
		regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>,
	) -> anyhow::Result<Self> {
		// An unset oneof means an unknown variant from a newer peer, or a
		// malformed message -- never `NONE`. See the note on `value.proto`'s
		// `Value` message.
		let inner = value
			.value
			.ok_or_else(|| anyhow!("unrecognised Value variant: this build cannot represent it"))?;
		match inner {
			ValueInner::None(_) => Ok(Value::None),
			ValueInner::Null(_) => Ok(Value::Null),
			ValueInner::Bool(b) => Ok(Value::Bool(b)),
			ValueInner::Int64(i) => Ok(Value::Number(Number::Int(i))),
			ValueInner::Float64(f) => Ok(Value::Number(Number::Float(f))),
			ValueInner::Decimal(d) => Ok(Value::Number(Number::Decimal(d.try_into()?))),
			ValueInner::String(s) => Ok(Value::String(s)),
			ValueInner::Bytes(b) => Ok(Value::Bytes(Bytes(b))),
			ValueInner::Duration(d) => Ok(Value::Duration(Duration(d.try_into()?))),
			ValueInner::Datetime(dt) => {
				let chrono_dt =
					dt.to_chrono().ok_or_else(|| anyhow!("Datetime out of representable range"))?;
				Ok(Value::Datetime(Datetime(chrono_dt)))
			}
			ValueInner::Uuid(u) => Ok(Value::Uuid(Uuid(u.to_uuid()?))),
			ValueInner::Geometry(g) => Ok(Value::Geometry(g.try_into()?)),
			ValueInner::Table(t) => Ok(Value::Table(Table::new(t))),
			ValueInner::RecordId(r) => Ok(Value::RecordId(crate::RecordId::from_proto_with_regex(r, regex)?)),
			// A raw, unparsed record id awaiting server-side parsing. This
			// build has no "unparsed record id" value to decode it into.
			ValueInner::StringRecordId(_) => {
				Err(anyhow!("Unsupported Value type: an unparsed string record id"))
			}
			ValueInner::File(f) => Ok(Value::File(File {
				bucket: f.bucket,
				key: f.key,
			})),
			ValueInner::Range(r) => Ok(Value::Range(Box::new(Range::from_proto_with_regex(*r, regex)?))),
			ValueInner::Regex(s) => {
				let compiled = regex(&s)?;
				// Match the existing FromStr normalization without allocating a
				// second pattern. A reused compiler result must describe this
				// exact normalized pattern, not merely some cached regex.
				let mut chars = s.chars().peekable();
				let normalized = std::iter::from_fn(|| {
					let next = chars.next()?;
					if next == '\\' && chars.peek() == Some(&'/') { chars.next() } else { Some(next) }
				});
				if !compiled.regex().as_str().chars().eq(normalized) {
					return Err(anyhow!("Regex decoder returned a different normalized pattern"));
				}
				Ok(Value::Regex(compiled))
			}
			ValueInner::Object(o) => Ok(Value::Object(Object::from_proto_with_regex(o, regex)?)),
			ValueInner::Array(a) => Ok(Value::Array(Array::from_proto_with_regex(a, regex)?)),
			ValueInner::Set(s) => Ok(Value::Set(Set::from_proto_with_regex(s, regex)?)),
		}
	}
}

impl TryFrom<Object> for proto::Object {
	type Error = anyhow::Error;

	fn try_from(value: Object) -> Result<Self, Self::Error> {
		let mut items = BTreeMap::new();
		for (key, value) in value.0 {
			items.insert(key, proto::Value::try_from(value)?);
		}
		Ok(items.into())
	}
}

impl TryFrom<proto::Object> for Object {
	type Error = anyhow::Error;

	fn try_from(value: proto::Object) -> Result<Self, Self::Error> {
		Self::from_proto_with_regex(value, &mut |pattern| Ok(Regex::from_str(pattern)?))
	}
}
impl Object {
	pub(crate) fn from_proto_with_regex(value: proto::Object, regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>) -> anyhow::Result<Self> {
		let mut map = BTreeMap::new();
		for entry in value.items {
			let value = entry
				.value
				.ok_or_else(|| anyhow!("Missing value in Object entry for key {:?}", entry.key))?;
			map.insert(entry.key, Value::try_from_proto_with_regex(value, regex)?);
		}
		Ok(Object(map))
	}
}

impl TryFrom<Array> for proto::Array {
	type Error = anyhow::Error;

	fn try_from(value: Array) -> Result<Self, Self::Error> {
		Ok(proto::Array::new(try_values(value.0)?))
	}
}

impl TryFrom<proto::Array> for Array {
	type Error = anyhow::Error;

	fn try_from(value: proto::Array) -> Result<Self, Self::Error> {
		Self::from_proto_with_regex(value, &mut |pattern| Ok(Regex::from_str(pattern)?))
	}
}
impl Array {
	pub(crate) fn from_proto_with_regex(value: proto::Array, regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>) -> anyhow::Result<Self> {
		let values =
			value.values.into_iter().map(|value| Value::try_from_proto_with_regex(value, regex)).collect::<anyhow::Result<Vec<_>>>()?;
		Ok(Array(values))
	}
}

impl TryFrom<Set> for proto::Set {
	type Error = anyhow::Error;

	fn try_from(value: Set) -> Result<Self, Self::Error> {
		Ok(proto::Set {
			values: try_values(value.0)?,
		})
	}
}

/// Converts a sequence of values, stopping at the first one the wire cannot
/// carry.
fn try_values(values: impl IntoIterator<Item = Value>) -> anyhow::Result<Vec<proto::Value>> {
	values.into_iter().map(proto::Value::try_from).collect()
}

impl TryFrom<proto::Set> for Set {
	type Error = anyhow::Error;

	fn try_from(value: proto::Set) -> Result<Self, Self::Error> {
		Self::from_proto_with_regex(value, &mut |pattern| Ok(Regex::from_str(pattern)?))
	}
}
impl Set {
	pub(crate) fn from_proto_with_regex(value: proto::Set, regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>) -> anyhow::Result<Self> {
		let values =
			value.values.into_iter().map(|value| Value::try_from_proto_with_regex(value, regex)).collect::<anyhow::Result<_>>()?;
		Ok(Set(values))
	}
}

impl TryFrom<Range> for proto::Range {
	type Error = anyhow::Error;

	fn try_from(value: Range) -> Result<Self, Self::Error> {
		Ok(proto::Range {
			start: Some(Box::new(bound_to_proto(value.start)?)),
			end: Some(Box::new(bound_to_proto(value.end)?)),
		})
	}
}

impl TryFrom<proto::Range> for Range {
	type Error = anyhow::Error;

	fn try_from(value: proto::Range) -> Result<Self, Self::Error> {
		Self::from_proto_with_regex(value, &mut |pattern| Ok(Regex::from_str(pattern)?))
	}
}
impl Range {
	pub(crate) fn from_proto_with_regex(value: proto::Range, regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>) -> anyhow::Result<Self> {
		let start = value.start.ok_or_else(|| anyhow!("Missing start in Range"))?;
		let end = value.end.ok_or_else(|| anyhow!("Missing end in Range"))?;
		Ok(Range {
			start: bound_from_proto(*start, regex)?,
			end: bound_from_proto(*end, regex)?,
		})
	}
}

fn bound_to_proto(bound: Bound<Value>) -> anyhow::Result<proto::ValueBound> {
	let bound = match bound {
		Bound::Included(v) => ValueBoundInner::Inclusive(Box::new(v.try_into()?)),
		Bound::Excluded(v) => ValueBoundInner::Exclusive(Box::new(v.try_into()?)),
		Bound::Unbounded => ValueBoundInner::Unbounded(proto::NullValue::default()),
	};
	Ok(proto::ValueBound {
		bound: Some(bound),
	})
}

fn bound_from_proto(bound: proto::ValueBound, regex: &mut dyn FnMut(&str) -> anyhow::Result<Regex>) -> anyhow::Result<Bound<Value>> {
	let bound = bound.bound.ok_or_else(|| anyhow!("Missing bound in ValueBound"))?;
	match bound {
		ValueBoundInner::Inclusive(v) => Ok(Bound::Included(Value::try_from_proto_with_regex(*v, regex)?)),
		ValueBoundInner::Exclusive(v) => Ok(Bound::Excluded(Value::try_from_proto_with_regex(*v, regex)?)),
		ValueBoundInner::Unbounded(_) => Ok(Bound::Unbounded),
	}
}
