//! `lctx_id(kind, field, …)`: the one scalar UDF Stage D computes ids with (DESIGN §3.4.1,
//! ADR-0014). It is `IdHasher` exactly: the kind through `IdHasher::new`, then every other argument
//! in the `opt_*` encoding (a presence byte, then the length-prefixed value), so an id computed in
//! SQL equals the same recipe computed in Rust. Types are checked at plan time and never cast:
//! UInt64 (`row_number()`), Int32 and floats are refused rather than silently re-encoded.

use std::sync::Arc;

use arrow_array::builder::FixedSizeBinaryBuilder;
use arrow_array::cast::AsArray;
use arrow_array::types::{Int16Type, Int64Type};
use arrow_array::{
    Array, ArrayRef, BinaryArray, BinaryViewArray, BooleanArray, FixedSizeBinaryArray, Int16Array,
    Int64Array, LargeBinaryArray, LargeStringArray, StringArray, StringViewArray,
};
use arrow_schema::{DataType, Field, FieldRef};
use cpg_schema::id::IdHasher;
use datafusion::common::{Result, ScalarValue, exec_err, plan_err};
use datafusion::logical_expr::{
    ColumnarValue, ReturnFieldArgs, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature,
    Volatility,
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

/// An argument column, its type resolved once per batch rather than per row (H1 P7).
enum Column {
    Utf8(StringArray),
    LargeUtf8(LargeStringArray),
    Utf8View(StringViewArray),
    Int16(Int16Array),
    Int64(Int64Array),
    Binary(BinaryArray),
    LargeBinary(LargeBinaryArray),
    BinaryView(BinaryViewArray),
    Fixed(FixedSizeBinaryArray),
    Boolean(BooleanArray),
}

impl Column {
    fn of(a: &ArrayRef) -> Result<Self> {
        Ok(match a.data_type() {
            DataType::Utf8 => Self::Utf8(a.as_string::<i32>().clone()),
            DataType::LargeUtf8 => Self::LargeUtf8(a.as_string::<i64>().clone()),
            DataType::Utf8View => Self::Utf8View(a.as_string_view().clone()),
            DataType::Int16 => Self::Int16(a.as_primitive::<Int16Type>().clone()),
            DataType::Int64 => Self::Int64(a.as_primitive::<Int64Type>().clone()),
            DataType::Binary => Self::Binary(a.as_binary::<i32>().clone()),
            DataType::LargeBinary => Self::LargeBinary(a.as_binary::<i64>().clone()),
            DataType::BinaryView => Self::BinaryView(a.as_binary_view().clone()),
            DataType::FixedSizeBinary(_) => Self::Fixed(a.as_fixed_size_binary().clone()),
            DataType::Boolean => Self::Boolean(a.as_boolean().clone()),
            other => return exec_err!("{NAME}: unsupported argument type {other}"),
        })
    }

    /// Feed row `row` to `h` in the `opt_*` encoding of its type.
    fn feed(&self, h: &mut IdHasher, row: usize) {
        match self {
            Self::Utf8(a) => h.opt_str((!a.is_null(row)).then(|| a.value(row))),
            Self::LargeUtf8(a) => h.opt_str((!a.is_null(row)).then(|| a.value(row))),
            Self::Utf8View(a) => h.opt_str((!a.is_null(row)).then(|| a.value(row))),
            Self::Int16(a) => h.opt_i64((!a.is_null(row)).then(|| i64::from(a.value(row)))),
            Self::Int64(a) => h.opt_i64((!a.is_null(row)).then(|| a.value(row))),
            Self::Binary(a) => h.opt_bytes((!a.is_null(row)).then(|| a.value(row))),
            Self::LargeBinary(a) => h.opt_bytes((!a.is_null(row)).then(|| a.value(row))),
            Self::BinaryView(a) => h.opt_bytes((!a.is_null(row)).then(|| a.value(row))),
            Self::Fixed(a) => h.opt_bytes((!a.is_null(row)).then(|| a.value(row))),
            Self::Boolean(a) => h.opt_bool((!a.is_null(row)).then(|| a.value(row))),
        };
    }
}

/// The argument types: the kind is text, every other argument an accepted type.
fn check(args: &[DataType]) -> Result<()> {
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
        check(args)?;
        Ok(DataType::FixedSizeBinary(16))
    }

    /// The kind must be a non-null text literal, checked here at plan time (§3.4.1 says so; H1 P7
    /// enforces it), and the id is never null (a null argument is encoded, not propagated), so
    /// the optimizer can fold `IS NULL` tests.
    fn return_field_from_args(&self, args: ReturnFieldArgs) -> Result<FieldRef> {
        let types: Vec<DataType> = args
            .arg_fields
            .iter()
            .map(|f| f.data_type().clone())
            .collect();
        check(&types)?;
        match args.scalar_arguments.first() {
            Some(Some(
                ScalarValue::Utf8(Some(_))
                | ScalarValue::Utf8View(Some(_))
                | ScalarValue::LargeUtf8(Some(_)),
            )) => {}
            _ => return plan_err!("{NAME}: the kind must be a non-null text literal"),
        }
        Ok(Arc::new(Field::new(
            NAME,
            DataType::FixedSizeBinary(16),
            false,
        )))
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let rows = args.number_rows;
        let kind = match args.args.first() {
            Some(ColumnarValue::Scalar(
                ScalarValue::Utf8(Some(k))
                | ScalarValue::Utf8View(Some(k))
                | ScalarValue::LargeUtf8(Some(k)),
            )) => k.clone(),
            _ => return exec_err!("{NAME}: the kind must be a non-null text literal"),
        };
        let columns: Vec<Column> = args.args[1..]
            .iter()
            .map(|a| Column::of(&a.to_array(rows)?))
            .collect::<Result<_>>()?;
        // The tag and kind are hashed once; each row continues from a copy.
        let seed = IdHasher::new(&kind);
        let mut out = FixedSizeBinaryBuilder::with_capacity(rows, 16);
        for row in 0..rows {
            let mut h = seed.clone();
            for c in &columns {
                c.feed(&mut h, row);
            }
            out.append_value(h.finish_id().0)?;
        }
        Ok(ColumnarValue::Array(Arc::new(out.finish())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// The kind is a literal, checked at planning (H1 P7): a column or a null kind is refused
    /// before any row runs. The id is never null.
    #[tokio::test]
    async fn the_kind_is_a_literal_and_the_id_never_null() {
        for sql in [
            "SELECT lctx_id(k, 'x') FROM (VALUES ('a'), ('b')) AS t(k)",
            "SELECT lctx_id(CAST(NULL AS VARCHAR), 'x')",
        ] {
            let err = eval(sql).await.unwrap_err().to_string();
            assert!(err.contains("literal"), "{sql}: {err}");
        }
        let ctx = crate::snapshot::empty_session();
        let df = crate::sql::query(&ctx, "SELECT lctx_id('k', CAST(NULL AS VARCHAR)) AS id")
            .await
            .unwrap();
        assert!(!df.schema().field(0).is_nullable());
    }
}
