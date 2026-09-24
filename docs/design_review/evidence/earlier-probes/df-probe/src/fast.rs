//! A probe copy of `lctx_id` with a scalar-kind fast path, per-column dispatch hoisted out of the
//! row loop, the kind prefix hashed once per batch (IdHasher is Clone), and one output buffer.
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{Int16Type, Int64Type};
use arrow_array::{Array, ArrayRef, FixedSizeBinaryArray};
use arrow_schema::{DataType, Field, FieldRef};
use cpg_schema::id::IdHasher;
use datafusion::common::{Result, ScalarValue, exec_err};
use datafusion::logical_expr::{
    ColumnarValue, ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature,
    Volatility,
};

#[derive(Debug, PartialEq, Eq, Hash)]
struct Fast {
    signature: Signature,
}

pub fn lctx_fast() -> ScalarUDF {
    ScalarUDF::from(Fast { signature: Signature::variadic_any(Volatility::Immutable) })
}

enum Col<'a> {
    Str(Box<dyn Fn(usize) -> Option<&'a str> + 'a>),
    I64(Box<dyn Fn(usize) -> Option<i64> + 'a>),
    Bytes(Box<dyn Fn(usize) -> Option<&'a [u8]> + 'a>),
    Bool(Box<dyn Fn(usize) -> Option<bool> + 'a>),
}

fn col(a: &ArrayRef) -> Result<Col<'_>> {
    Ok(match a.data_type() {
        DataType::Utf8 => { let x = a.as_string::<i32>(); Col::Str(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::LargeUtf8 => { let x = a.as_string::<i64>(); Col::Str(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::Utf8View => { let x = a.as_string_view(); Col::Str(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::Int16 => { let x = a.as_primitive::<Int16Type>(); Col::I64(Box::new(move |i| x.is_valid(i).then(|| i64::from(x.value(i))))) }
        DataType::Int64 => { let x = a.as_primitive::<Int64Type>(); Col::I64(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::Binary => { let x = a.as_binary::<i32>(); Col::Bytes(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::LargeBinary => { let x = a.as_binary::<i64>(); Col::Bytes(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::BinaryView => { let x = a.as_binary_view(); Col::Bytes(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::FixedSizeBinary(_) => { let x = a.as_fixed_size_binary(); Col::Bytes(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        DataType::Boolean => { let x = a.as_boolean(); Col::Bool(Box::new(move |i| x.is_valid(i).then(|| x.value(i)))) }
        other => return exec_err!("lctx_fast: unsupported {other}"),
    })
}

impl ScalarUDFImpl for Fast {
    fn name(&self) -> &str { "lctx_fast" }
    fn signature(&self) -> &Signature { &self.signature }
    fn return_type(&self, _args: &[DataType]) -> Result<DataType> { Ok(DataType::FixedSizeBinary(16)) }
    fn return_field_from_args(&self, _args: ReturnFieldArgs) -> Result<FieldRef> {
        // never null: a null field hashes as absent; a null kind is an execution error
        Ok(Arc::new(Field::new(self.name(), DataType::FixedSizeBinary(16), false)))
    }
    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let rows = args.number_rows;
        let ColumnarValue::Scalar(kind) = &args.args[0] else { return exec_err!("kind must be a literal") };
        let kind = match kind {
            ScalarValue::Utf8(Some(k)) | ScalarValue::Utf8View(Some(k)) | ScalarValue::LargeUtf8(Some(k)) => k.clone(),
            _ => return exec_err!("kind must be a non-null text literal"),
        };
        let base = IdHasher::new(&kind);
        let arrays: Vec<ArrayRef> = args.args[1..].iter().map(|a| a.to_array(rows)).collect::<Result<_>>()?;
        let cols: Vec<Col<'_>> = arrays.iter().map(col).collect::<Result<_>>()?;
        let mut out = Vec::with_capacity(rows * 16);
        for row in 0..rows {
            let mut h = base.clone();
            for c in &cols {
                match c {
                    Col::Str(f) => { h.opt_str(f(row)); }
                    Col::I64(f) => { h.opt_i64(f(row)); }
                    Col::Bytes(f) => { h.opt_bytes(f(row)); }
                    Col::Bool(f) => { h.opt_bool(f(row)); }
                }
            }
            out.extend_from_slice(&h.finish_id().0);
        }
        Ok(ColumnarValue::Array(Arc::new(FixedSizeBinaryArray::try_new(16, out.into(), None)?)))
    }
}
