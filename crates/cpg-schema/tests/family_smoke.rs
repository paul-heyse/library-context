//! Proves the pinned dependency family (DESIGN §7, ADR-0090) links and works end to end:
//! Arrow 59.3 batches queried by DataFusion 55.1 SQL.

use std::sync::Arc;

use arrow_array::{Array, FixedSizeBinaryArray, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use datafusion::prelude::SessionContext;

fn batch(ids: &[i64]) -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![Field::new(
            "ordinal",
            DataType::Int64,
            false,
        )])),
        vec![Arc::new(Int64Array::from(ids.to_vec()))],
    )
    .expect("valid batch")
}

#[tokio::test]
async fn arrow_batches_query_through_datafusion() {
    let schema = batch(&[]).schema();
    let table = datafusion::datasource::MemTable::try_new(
        schema,
        vec![vec![batch(&[1, 2, 3]), batch(&[4])]],
    )
    .expect("a table of two batches");
    let ctx = SessionContext::new();
    ctx.register_table("t", Arc::new(table)).expect("register");
    let out = ctx
        .sql("SELECT count(*) AS n, sum(ordinal) AS s FROM t")
        .await
        .expect("plan")
        .collect()
        .await
        .expect("execute");

    let n = out[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("count is Int64");
    let s = out[0]
        .column(1)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("sum is Int64");
    assert_eq!((n.value(0), s.value(0)), (4, 10));
}

/// The computation profile uses `FixedSizeBinary(16)` ids (Initial_plan §2.1);
/// this only checks the type is constructible at the pinned Arrow version.
#[test]
fn fixed_size_binary_ids_are_available() {
    let ids = FixedSizeBinaryArray::try_from_iter([[0u8; 16], [1u8; 16]].into_iter())
        .expect("16-byte ids");
    assert_eq!(ids.value_length(), 16);
    assert_eq!(ids.len(), 2);
}
