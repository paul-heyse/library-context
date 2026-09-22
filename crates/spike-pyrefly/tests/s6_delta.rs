//! S6: Delta built-ins at the pinned delta-rs (58f07cd6). Each test prints its observation so
//! the spike record can quote it; assertions encode what the design relies on.

use std::sync::Arc;

use arrow_array::{Array, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::prelude::SessionContext;
use deltalake::delta_datafusion::create_session;
use deltalake::kernel::StructType;
use deltalake::kernel::engine::arrow_conversion::TryIntoKernel as _;
use deltalake::kernel::transaction::CommitProperties;
use deltalake::{DeltaTable, DeltaTableBuilder, TableProperty};

fn schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("snapshot_id", DataType::FixedSizeBinary(16), false),
        Field::new("origin", DataType::Int16, false),
        Field::new("start_byte", DataType::Int64, false),
        Field::new("end_byte", DataType::Int64, false),
    ]))
}

fn batch(sid: u8, origin: i16, start: i64, end: i64) -> RecordBatch {
    RecordBatch::try_new(
        schema(),
        vec![
            Arc::new(FixedSizeBinaryArray::try_from_iter([[sid; 16]].into_iter()).unwrap()),
            Arc::new(Int16Array::from(vec![origin])),
            Arc::new(Int64Array::from(vec![start])),
            Arc::new(Int64Array::from(vec![end])),
        ],
    )
    .unwrap()
}

async fn create(dir: &std::path::Path) -> DeltaTable {
    let url = url::Url::from_directory_path(dir).unwrap();
    let kernel: StructType = schema().as_ref().try_into_kernel().unwrap();
    let created = DeltaTable::try_from_url(url)
        .await
        .unwrap()
        .create()
        .with_columns(kernel.fields().cloned())
        .with_configuration_property(TableProperty::AppendOnly, Some("true"))
        .await
        .expect("create");
    // `delta.constraints.*` keys are rejected by CreateBuilder unless unknown keys are allowed;
    // add_constraint is the built-in route (it also enables the CheckConstraints feature).
    created
        .add_constraint()
        .with_constraint("origin_code", "origin BETWEEN 0 AND 4")
        .with_constraint("span_order", "start_byte >= 0 AND end_byte >= start_byte")
        .await
        .expect("add constraints")
}

#[tokio::test]
async fn check_constraints_and_append_only_on_write_builder() {
    let dir = tempfile::tempdir().unwrap();
    let t = create(dir.path()).await;
    let cfg = t.snapshot().unwrap().metadata().configuration().clone();
    println!("S6 config after create: {cfg:?}");
    let proto = t.snapshot().unwrap().protocol().clone();
    println!("S6 protocol: {proto:?}");

    let t = t
        .write([batch(1, 1, 0, 10)])
        .with_commit_properties(CommitProperties::default().with_metadata([(
            "lctx.snapshot_id".to_owned(),
            serde_json::json!("01".repeat(16)),
        )]))
        .await
        .expect("valid append");
    assert_eq!(t.version(), Some(2));

    let bad_code = t.clone().write([batch(1, 9, 0, 10)]).await;
    println!(
        "S6 bad origin via WriteBuilder: {:?}",
        bad_code.as_ref().err().map(|e| e.to_string())
    );
    assert!(bad_code.is_err(), "CHECK origin_code must reject origin=9");

    let bad_span = t.clone().write([batch(1, 1, 10, 5)]).await;
    println!(
        "S6 bad span via WriteBuilder: {:?}",
        bad_span.as_ref().err().map(|e| e.to_string())
    );
    assert!(
        bad_span.is_err(),
        "CHECK span_order must reject end < start"
    );

    let deleted = t.clone().delete().with_predicate("origin = 1").await;
    println!(
        "S6 delete on appendOnly: {:?}",
        deleted.as_ref().err().map(|e| e.to_string())
    );
    assert!(
        deleted.is_err(),
        "appendOnly must reject a data-changing remove"
    );

    let history: Vec<_> = t.history(Some(5)).await.unwrap().collect();
    let meta = history
        .iter()
        .find_map(|c| c.info.get("lctx.snapshot_id").cloned());
    println!("S6 commitInfo lctx.snapshot_id: {meta:?}");
    assert!(meta.is_some(), "commitInfo keeps app metadata");
}

#[tokio::test]
async fn datafusion_insert_into_bypasses_check_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let t = create(dir.path()).await;
    let t = t.write([batch(1, 1, 0, 10)]).await.unwrap();
    let ctx: SessionContext = create_session().into_inner();
    t.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("t", t.table_provider().await.unwrap())
        .unwrap();
    let r = async {
        ctx.sql("INSERT INTO t VALUES (X'09090909090909090909090909090909', 9, 10, 5)")
            .await?
            .collect()
            .await
    }
    .await;
    println!(
        "S6 INSERT INTO (invalid row): {:?}",
        r.as_ref().err().map(|e| e.to_string())
    );
    let reloaded = DeltaTableBuilder::from_url(url::Url::from_directory_path(dir.path()).unwrap())
        .unwrap()
        .load()
        .await
        .unwrap();
    let ctx2 = SessionContext::new();
    reloaded.update_datafusion_session(&ctx2.state()).unwrap();
    ctx2.register_table("t", reloaded.table_provider().await.unwrap())
        .unwrap();
    let n = ctx2
        .sql("SELECT count(*) FROM t WHERE origin = 9")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let bad_rows = n[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap()
        .value(0);
    println!(
        "S6 rows violating CHECK after INSERT INTO: {bad_rows} (version {:?})",
        reloaded.version()
    );
}

#[tokio::test]
async fn pinned_read_casts_ids_back_and_derived_write_from_plan() {
    let dir = tempfile::tempdir().unwrap();
    let url = url::Url::from_directory_path(dir.path()).unwrap();
    let t = create(dir.path()).await;
    let t = t.write([batch(1, 1, 0, 10)]).await.unwrap();
    let _ = t.write([batch(2, 2, 3, 4)]).await.unwrap();

    // Pin version 2 (create=0, constraints=1): only the first append is visible.
    let pinned = DeltaTableBuilder::from_url(url.clone())
        .unwrap()
        .with_version(2)
        .load()
        .await
        .unwrap();
    assert_eq!(pinned.version(), Some(2));
    let delta_ctx = create_session();
    let ctx: SessionContext = delta_ctx.into_inner();
    pinned.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("raw", pinned.table_provider().await.unwrap())
        .unwrap();
    let out = ctx
        .sql("SELECT snapshot_id FROM raw ORDER BY start_byte")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let col = out[0].column(0).clone();
    println!(
        "S6 read-back id type: {:?}, rows {}",
        col.data_type(),
        col.len()
    );
    let strict = CastOptions {
        safe: false,
        ..Default::default()
    };
    let direct = cast_with_options(&col, &DataType::FixedSizeBinary(16), &strict);
    println!(
        "S6 direct cast to FSB(16): {:?}",
        direct.as_ref().err().map(|e| e.to_string())
    );
    let binary = cast_with_options(&col, &DataType::Binary, &strict).unwrap();
    let fsb = cast_with_options(&binary, &DataType::FixedSizeBinary(16), &strict).unwrap();
    assert_eq!(fsb.data_type(), &DataType::FixedSizeBinary(16));
    assert_eq!(fsb.len(), 1);

    // Derived table written from a plan over the pinned read (the §4.3 derive path).
    let derived_dir = tempfile::tempdir().unwrap();
    let derived = create(derived_dir.path()).await;
    let plan = ctx
        .sql("SELECT snapshot_id, origin, start_byte, end_byte FROM raw WHERE origin = 1")
        .await
        .unwrap()
        .logical_plan()
        .clone();
    let derived = derived
        .write(vec![])
        .with_input_plan(plan)
        .with_session_state(Arc::new(ctx.state()))
        .await;
    println!(
        "S6 derived write from plan: {:?}",
        derived
            .as_ref()
            .map(|t| t.version())
            .map_err(|e| e.to_string())
    );
    let derived = derived.expect("derived write");
    assert_eq!(derived.version(), Some(2));

    // A derived plan that violates a CHECK must also be rejected on this path.
    let bad_plan = ctx
        .sql("SELECT snapshot_id, CAST(9 AS SMALLINT) AS origin, start_byte, end_byte FROM raw")
        .await
        .unwrap()
        .logical_plan()
        .clone();
    let bad = derived
        .write(vec![])
        .with_input_plan(bad_plan)
        .with_session_state(Arc::new(ctx.state()))
        .await;
    println!(
        "S6 derived write violating CHECK: {:?}",
        bad.as_ref().err().map(|e| e.to_string())
    );
    assert!(bad.is_err(), "CHECK enforced on with_input_plan writes");
}
