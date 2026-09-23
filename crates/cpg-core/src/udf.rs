//! `lctx_id(kind, field, …)`: the one scalar UDF Stage D computes ids with (DESIGN §3.4.1,
//! ADR-0014). It is `IdHasher` exactly: the kind through `IdHasher::new`, then every other argument
//! in the `opt_*` encoding (a presence byte, then the length-prefixed value), so an id computed in
//! SQL equals the same recipe computed in Rust. Types are checked at plan time and never cast:
//! UInt64 (`row_number()`), Int32 and floats are refused rather than silently re-encoded.

use std::sync::Arc;

use arrow_array::builder::FixedSizeBinaryBuilder;
use arrow_array::cast::AsArray;
use arrow_array::types::{Int16Type, Int64Type};
use arrow_array::{Array, ArrayRef};
use arrow_schema::DataType;
use cpg_schema::id::IdHasher;
use datafusion::common::{Result, exec_err, plan_err};
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, Volatility,
};

/// The SQL name.
pub const NAME: &str = "lctx_id";

/// Bumped by hand whenever the encoding changes; part of `compiler_digest`.
pub const VERSION: u32 = 1;

#[derive(Debug, PartialEq, Eq, Hash)]
struct LctxId {
    signature: Signature,
}

/// The UDF, ready to register.
pub fn lctx_id() -> ScalarUDF {
    ScalarUDF::from(LctxId {
        signature: Signature::variadic_any(Volatility::Immutable),
    })
}

fn text(t: &DataType) -> bool {
    matches!(t, DataType::Utf8 | DataType::Utf8View | DataType::LargeUtf8)
}

fn accepted(t: &DataType) -> bool {
    text(t)
        || matches!(
            t,
            DataType::Int16
                | DataType::Int64
                | DataType::Binary
                | DataType::BinaryView
                | DataType::LargeBinary
                | DataType::FixedSizeBinary(_)
                | DataType::Boolean
        )
}

fn text_at(a: &ArrayRef, row: usize) -> Option<&str> {
    if a.is_null(row) {
        return None;
    }
    Some(match a.data_type() {
        DataType::Utf8 => a.as_string::<i32>().value(row),
        DataType::LargeUtf8 => a.as_string::<i64>().value(row),
        _ => a.as_string_view().value(row),
    })
}

/// Feed `a[row]` to `h` in the `opt_*` encoding of its type.
fn feed(h: &mut IdHasher, a: &ArrayRef, row: usize) -> Result<()> {
    let null = a.is_null(row);
    match a.data_type() {
        t if text(t) => {
            h.opt_str(text_at(a, row));
        }
        DataType::Int16 => {
            h.opt_i64((!null).then(|| i64::from(a.as_primitive::<Int16Type>().value(row))));
        }
        DataType::Int64 => {
            h.opt_i64((!null).then(|| a.as_primitive::<Int64Type>().value(row)));
        }
        DataType::Binary => {
            h.opt_bytes((!null).then(|| a.as_binary::<i32>().value(row)));
        }
        DataType::LargeBinary => {
            h.opt_bytes((!null).then(|| a.as_binary::<i64>().value(row)));
        }
        DataType::BinaryView => {
            h.opt_bytes((!null).then(|| a.as_binary_view().value(row)));
        }
        DataType::FixedSizeBinary(_) => {
            h.opt_bytes((!null).then(|| a.as_fixed_size_binary().value(row)));
        }
        DataType::Boolean => {
            h.opt_bool((!null).then(|| a.as_boolean().value(row)));
        }
        other => return exec_err!("{NAME}: unsupported argument type {other}"),
    }
    Ok(())
}

impl ScalarUDFImpl for LctxId {
    fn name(&self) -> &str {
        NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, args: &[DataType]) -> Result<DataType> {
        match args.first() {
            Some(t) if text(t) => {}
            _ => return plan_err!("{NAME}: the first argument is the kind, a text literal"),
        }
        if let Some((i, t)) = args.iter().enumerate().skip(1).find(|(_, t)| !accepted(t)) {
            return plan_err!(
                "{NAME}: argument {i} has type {t}; accepted: Utf8, Int16, Int64, Binary, \
                 FixedSizeBinary, Boolean (cast explicitly, e.g. a row_number() to BIGINT)"
            );
        }
        Ok(DataType::FixedSizeBinary(16))
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let rows = args.number_rows;
        let arrays: Vec<ArrayRef> = args
            .args
            .iter()
            .map(|a| a.to_array(rows))
            .collect::<Result<_>>()?;
        let mut out = FixedSizeBinaryBuilder::with_capacity(rows, 16);
        for row in 0..rows {
            let Some(kind) = text_at(&arrays[0], row) else {
                return exec_err!("{NAME}: the kind is null");
            };
            let mut h = IdHasher::new(kind);
            for a in &arrays[1..] {
                feed(&mut h, a, row)?;
            }
            out.append_value(h.finish_id().0)?;
        }
        Ok(ColumnarValue::Array(Arc::new(out.finish())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::FixedSizeBinaryArray;
    use cpg_schema::id::Id;

    async fn eval(sql: &str) -> Result<Vec<Id>> {
        let ctx = crate::snapshot::empty_session();
        let batches = crate::sql::query(&ctx, sql)
            .await
            .map_err(|e| datafusion::error::DataFusionError::External(Box::new(e)))?
            .collect()
            .await?;
        let mut out = Vec::new();
        for b in batches {
            let a = b
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .expect("FixedSizeBinary(16)");
            for i in 0..a.len() {
                out.push(Id(a.value(i).try_into().expect("16 bytes")));
            }
        }
        Ok(out)
    }

    #[tokio::test]
    async fn sql_ids_equal_the_rust_recipe() {
        // Known answers shared with `IdHasher`: every field in the opt_* encoding.
        let expected = IdHasher::new("edge")
            .opt_i64(Some(7))
            .opt_str(Some("call_target"))
            .opt_str(None)
            .opt_bytes(Some(&[1, 2]))
            .opt_bool(Some(true))
            .opt_i64(Some(3))
            .finish_id();
        let got = eval(
            "SELECT lctx_id('edge', CAST(7 AS BIGINT), 'call_target', CAST(NULL AS VARCHAR), \
             X'0102', true, CAST(3 AS SMALLINT))",
        )
        .await
        .unwrap();
        assert_eq!(got, [expected]);
        // A view-typed column hashes like its plain type (Delta reads Utf8 back as Utf8View).
        let view = eval("SELECT lctx_id('k', arrow_cast('x', 'Utf8View'))")
            .await
            .unwrap();
        let plain = eval("SELECT lctx_id('k', 'x')").await.unwrap();
        assert_eq!(view, plain);
        assert_eq!(plain, [IdHasher::new("k").opt_str(Some("x")).finish_id()]);
        // An id column (16 bytes) hashes like `opt_id`.
        let id = Id([5; 16]);
        let from_id = IdHasher::new("k").opt_id(Some(id)).finish_id();
        let lit = format!("SELECT lctx_id('k', X'{}')", id.hex());
        assert_eq!(eval(&lit).await.unwrap(), [from_id]);
    }

    #[tokio::test]
    async fn every_rust_recipe_equals_its_sql_form() {
        // Review F3: the recipes the extractor computes in Rust, recomputed by the UDF with the
        // field order the `id:*` rules use.
        use cpg_schema::id::recipe;
        let call = Id([3; 16]);
        let module = recipe::external_module("fastmcp-slim", "4.0.5", "fastmcp.server");
        let cases = [
            (
                recipe::argument(call, 2),
                format!(
                    "SELECT lctx_id('argument', X'{}', CAST(2 AS BIGINT))",
                    call.hex()
                ),
            ),
            (
                module,
                "SELECT lctx_id('external_module', 'fastmcp-slim', '4.0.5', 'fastmcp.server')"
                    .to_owned(),
            ),
            (
                recipe::external_symbol(module, 1, "7"),
                format!(
                    "SELECT lctx_id('external_symbol', X'{}', CAST(1 AS SMALLINT), '7')",
                    module.hex()
                ),
            ),
            (
                recipe::field(call, "retries"),
                format!("SELECT lctx_id('field', X'{}', 'retries')", call.hex()),
            ),
        ];
        for (rust, sql) in cases {
            assert_eq!(eval(&sql).await.unwrap(), [rust], "{sql}");
        }
        // Pinned values: a recipe change is a migration (DM-51).
        insta::assert_snapshot!(
            [
                recipe::argument(call, 2),
                module,
                recipe::external_symbol(module, 1, "7"),
                recipe::field(call, "retries")
            ]
            .iter()
            .map(Id::hex)
            .collect::<Vec<_>>()
            .join("\n")
        );
    }

    #[tokio::test]
    async fn unsupported_types_are_refused_at_plan_time() {
        for sql in [
            "SELECT lctx_id('k', CAST(1 AS INT))",
            "SELECT lctx_id('k', row_number() OVER ())",
            "SELECT lctx_id('k', 1.5)",
            "SELECT lctx_id(CAST(1 AS BIGINT))",
        ] {
            let err = eval(sql).await.unwrap_err().to_string();
            assert!(err.contains("lctx_id"), "{sql}: {err}");
        }
    }
}
