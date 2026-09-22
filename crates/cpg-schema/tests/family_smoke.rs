//! Proves the pinned dependency family (DESIGN §7, ADR-0002) links and works
//! end to end: Arrow 59.3 batches → Delta table on local disk → DataFusion 55.1
//! query through the Delta table provider (not a raw Parquet scan).

use std::sync::Arc;

use arrow_array::{Array, FixedSizeBinaryArray, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use datafusion::prelude::SessionContext;
use deltalake::DeltaTable;

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
async fn delta_write_then_datafusion_query_through_provider() {
    let dir = tempfile::tempdir().expect("tempdir");
    let uri = url::Url::from_directory_path(dir.path()).expect("absolute path");

    let table = DeltaTable::try_from_url(uri)
        .await
        .expect("open location")
        .write([batch(&[1, 2, 3])])
        .await
        .expect("first commit");
    let table = table.write([batch(&[4])]).await.expect("second commit");
    assert_eq!(table.version(), Some(1));

    let ctx = SessionContext::new();
    ctx.register_table("t", table.table_provider().await.expect("provider"))
        .expect("register");
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
/// The Delta-boundary conversion to `Binary` is increment-1 work.
#[test]
fn fixed_size_binary_ids_are_available() {
    let ids = FixedSizeBinaryArray::try_from_iter([[0u8; 16], [1u8; 16]].into_iter())
        .expect("16-byte ids");
    assert_eq!(ids.value_length(), 16);
    assert_eq!(ids.len(), 2);
}
