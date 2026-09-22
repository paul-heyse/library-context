//! ADR-0009 probe (remaining parts), at the pinned delta-rs 58f07cd6:
//! P1 Binary column statistics and snapshot_id pruning; P2 an injected validation failure
//! publishes nothing; P3 an ambiguous `snapshots` append is classified by re-reading;
//! P4 a serving bundle rebuilt from Delta at recorded versions is byte-identical.

use std::sync::Arc;

use arrow_array::{
    Array, FixedSizeBinaryArray, Int64Array, RecordBatch, StringArray, cast::AsArray,
};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::prelude::SessionContext;
use deltalake::delta_datafusion::create_session;
use deltalake::kernel::StructType;
use deltalake::kernel::engine::arrow_conversion::TryIntoKernel as _;
use deltalake::{DeltaTable, DeltaTableBuilder, TableProperty};

const STRICT: CastOptions = CastOptions {
    safe: false,
    format_options: arrow_cast::display::FormatOptions::new(),
};

fn facts_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("snapshot_id", DataType::FixedSizeBinary(16), false),
        Field::new("fact_key", DataType::Int64, false),
        Field::new("label", DataType::Utf8, false),
    ]))
}

fn snapshots_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("snapshot_id", DataType::FixedSizeBinary(16), false),
        Field::new("table_name", DataType::Utf8, false),
        Field::new("delta_version", DataType::Int64, false),
        Field::new("row_count", DataType::Int64, false),
    ]))
}

fn sid(n: u8) -> [u8; 16] {
    [n; 16]
}

fn facts(snapshot: u8, keys: &[i64]) -> RecordBatch {
    RecordBatch::try_new(
        facts_schema(),
        vec![
            Arc::new(
                FixedSizeBinaryArray::try_from_iter(keys.iter().map(|_| sid(snapshot))).unwrap(),
            ),
            Arc::new(Int64Array::from(keys.to_vec())),
            Arc::new(StringArray::from(
                keys.iter()
                    .map(|k| format!("fact-{snapshot}-{k}"))
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .unwrap()
}

async fn create(dir: &std::path::Path, schema: &SchemaRef) -> DeltaTable {
    let kernel: StructType = schema.as_ref().try_into_kernel().unwrap();
    DeltaTable::try_from_url(url::Url::from_directory_path(dir).unwrap())
        .await
        .unwrap()
        .create()
        .with_columns(kernel.fields().cloned())
        .with_configuration_property(TableProperty::AppendOnly, Some("true"))
        .await
        .unwrap()
}

async fn load_at(dir: &std::path::Path, version: Option<u64>) -> DeltaTable {
    let b = DeltaTableBuilder::from_url(url::Url::from_directory_path(dir).unwrap()).unwrap();
    let t = match version {
        Some(v) => b.with_version(v).load().await.unwrap(),
        None => b.load().await.unwrap(),
    };
    if let Some(v) = version {
        assert_eq!(
            t.version(),
            Some(v),
            "pinned load must return the requested version"
        );
    }
    t
}

async fn query(t: &DeltaTable, sql: &str) -> Vec<RecordBatch> {
    let ctx: SessionContext = create_session().into_inner();
    t.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("t", t.table_provider().await.unwrap())
        .unwrap();
    ctx.sql(sql).await.unwrap().collect().await.unwrap()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Publish = one append to `snapshots` (one row per table), only after validation passes.
async fn publish(snapshots: DeltaTable, snapshot: u8, facts_version: u64, rows: i64) -> DeltaTable {
    let row = RecordBatch::try_new(
        snapshots_schema(),
        vec![
            Arc::new(FixedSizeBinaryArray::try_from_iter([sid(snapshot)].into_iter()).unwrap()),
            Arc::new(StringArray::from(vec!["facts"])),
            Arc::new(Int64Array::from(vec![facts_version as i64])),
            Arc::new(Int64Array::from(vec![rows])),
        ],
    )
    .unwrap();
    snapshots.write([row]).await.unwrap()
}

/// Classification after an ambiguous `snapshots` append: re-read the latest `snapshots`.
async fn is_published(snapshots_dir: &std::path::Path, snapshot: u8) -> bool {
    let t = load_at(snapshots_dir, None).await;
    let n = query(
        &t,
        &format!(
            "SELECT count(*) FROM t WHERE snapshot_id = X'{}'",
            hex(&sid(snapshot))
        ),
    )
    .await;
    n[0].column(0)
        .as_primitive::<arrow_array::types::Int64Type>()
        .value(0)
        > 0
}

/// Reader: resolve the row set, load the table at its recorded version, filter by snapshot_id.
async fn read_published(
    snapshots_dir: &std::path::Path,
    facts_dir: &std::path::Path,
    snapshot: u8,
) -> Vec<RecordBatch> {
    let snaps = load_at(snapshots_dir, None).await;
    let rs = query(
        &snaps,
        &format!(
            "SELECT delta_version FROM t WHERE snapshot_id = X'{}' AND table_name = 'facts'",
            hex(&sid(snapshot))
        ),
    )
    .await;
    let v = rs[0]
        .column(0)
        .as_primitive::<arrow_array::types::Int64Type>()
        .value(0);
    let t = load_at(facts_dir, Some(v as u64)).await;
    query(
        &t,
        &format!(
            "SELECT snapshot_id, fact_key, label FROM t WHERE snapshot_id = X'{}' \
             ORDER BY fact_key, label",
            hex(&sid(snapshot))
        ),
    )
    .await
}

#[tokio::test]
async fn p1_binary_statistics_and_snapshot_pruning() {
    let dir = tempfile::tempdir().unwrap();
    let mut t = create(dir.path(), &facts_schema()).await;
    for s in 1..=3u8 {
        t = t.write([facts(s, &[1, 2, 3])]).await.unwrap();
    }
    // Delta log statistics for the Binary snapshot_id column.
    let mut stats_cols = Vec::new();
    for entry in std::fs::read_dir(dir.path().join("_delta_log")).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|e| e == "json") {
            for line in std::fs::read_to_string(&p).unwrap().lines() {
                let v: serde_json::Value = serde_json::from_str(line).unwrap();
                if let Some(stats) = v["add"]["stats"].as_str() {
                    let s: serde_json::Value = serde_json::from_str(stats).unwrap();
                    stats_cols.push(s["minValues"].as_object().map(|m| {
                        let mut k: Vec<_> = m.keys().cloned().collect();
                        k.sort();
                        k
                    }));
                }
            }
        }
    }
    println!("P1 minValues columns per add: {stats_cols:?}");
    let binary_has_stats = stats_cols
        .iter()
        .flatten()
        .any(|k| k.iter().any(|c| c == "snapshot_id"));
    println!("P1 snapshot_id has Delta min/max stats: {binary_has_stats}");

    let ctx: SessionContext = create_session().into_inner();
    t.update_datafusion_session(&ctx.state()).unwrap();
    ctx.register_table("t", t.table_provider().await.unwrap())
        .unwrap();
    let plan = ctx
        .sql(&format!(
            "EXPLAIN ANALYZE SELECT count(*) FROM t WHERE snapshot_id = X'{}'",
            hex(&sid(2))
        ))
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    let text = arrow_cast::pretty::pretty_format_batches(&plan)
        .unwrap()
        .to_string();
    for line in text.lines().filter(|l| {
        l.contains("files")
            || l.contains("pruned")
            || l.contains("row_groups")
            || l.contains("DeltaScan")
    }) {
        println!("P1 plan: {}", line.trim());
    }
}

#[tokio::test]
async fn p2_injected_validation_failure_publishes_nothing() {
    let facts_dir = tempfile::tempdir().unwrap();
    let snaps_dir = tempfile::tempdir().unwrap();
    let t = create(facts_dir.path(), &facts_schema()).await;
    let snaps = create(snaps_dir.path(), &snapshots_schema()).await;

    // Attempt A writes rows (a duplicate key) and fails validation: no snapshots append.
    let t = t.write([facts(0xA, &[1, 1, 2])]).await.unwrap();
    let dup = query(
        &t,
        &format!(
            "SELECT fact_key FROM t WHERE snapshot_id = X'{}' GROUP BY fact_key HAVING count(*) > 1",
            hex(&sid(0xA))
        ),
    )
    .await;
    let violations: usize = dup.iter().map(|b| b.num_rows()).sum();
    assert_eq!(
        violations, 1,
        "the injected duplicate is caught by the validator"
    );
    // Attempt B writes and publishes.
    let t = t.write([facts(0xB, &[1, 2, 3])]).await.unwrap();
    let vb = t.version().unwrap();
    let _snaps = publish(snaps, 0xB, vb, 3).await;

    assert!(!is_published(snaps_dir.path(), 0xA).await);
    assert!(is_published(snaps_dir.path(), 0xB).await);
    let rows = read_published(snaps_dir.path(), facts_dir.path(), 0xB).await;
    let ids: Vec<Vec<u8>> = rows
        .iter()
        .flat_map(|b| {
            let c = cast_with_options(b.column(0), &DataType::Binary, &STRICT).unwrap();
            let c = c.as_binary::<i32>().clone();
            (0..c.len())
                .map(move |i| c.value(i).to_vec())
                .collect::<Vec<_>>()
        })
        .collect();
    println!(
        "P2 rows visible to B's reader: {} (A's rows physically present at v{vb})",
        ids.len()
    );
    assert_eq!(ids.len(), 3);
    assert!(
        ids.iter().all(|i| i == &sid(0xB)),
        "no row of the aborted attempt is visible"
    );
}

#[tokio::test]
async fn p3_ambiguous_snapshots_append_is_classified_by_rereading() {
    let facts_dir = tempfile::tempdir().unwrap();
    let snaps_dir = tempfile::tempdir().unwrap();
    let t = create(facts_dir.path(), &facts_schema()).await;
    let snaps = create(snaps_dir.path(), &snapshots_schema()).await;
    let t = t.write([facts(0xC, &[1])]).await.unwrap();
    // The append commits, but the caller sees an error (delta.commit.1): simulate by
    // discarding the result, as a caller that received Err would.
    let _lost = publish(snaps, 0xC, t.version().unwrap(), 1).await;
    let classified = is_published(snaps_dir.path(), 0xC).await;
    println!("P3 ambiguous append for C classified as published: {classified}");
    assert!(classified);
    // An attempt that never reached the append is classified unpublished.
    let t = t.write([facts(0xD, &[1])]).await.unwrap();
    let _ = t;
    assert!(!is_published(snaps_dir.path(), 0xD).await);
}

async fn build_bundle(
    snaps_dir: &std::path::Path,
    facts_dir: &std::path::Path,
    snapshot: u8,
) -> Vec<u8> {
    let rows = read_published(snaps_dir, facts_dir, snapshot).await;
    let merged = arrow_select::concat::concat_batches(&rows[0].schema(), &rows).unwrap();
    // Back to the declared schema: BinaryView -> Binary -> FixedSizeBinary(16); Utf8View -> Utf8.
    let schema = facts_schema();
    let cols = vec![
        cast_with_options(
            &cast_with_options(merged.column(0), &DataType::Binary, &STRICT).unwrap(),
            &DataType::FixedSizeBinary(16),
            &STRICT,
        )
        .unwrap(),
        cast_with_options(merged.column(1), &DataType::Int64, &STRICT).unwrap(),
        cast_with_options(merged.column(2), &DataType::Utf8, &STRICT).unwrap(),
    ];
    let batch = RecordBatch::try_new(schema.clone(), cols).unwrap();
    let mut buf = Vec::new();
    {
        let mut w = arrow_ipc::writer::FileWriter::try_new(&mut buf, &schema).unwrap();
        w.write(&batch).unwrap();
        w.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn p4_bundle_rebuilt_from_delta_is_byte_identical() {
    let facts_dir = tempfile::tempdir().unwrap();
    let snaps_dir = tempfile::tempdir().unwrap();
    let t = create(facts_dir.path(), &facts_schema()).await;
    let snaps = create(snaps_dir.path(), &snapshots_schema()).await;
    // Two batches so the table has several files and DataFusion may split them.
    let t = t.write([facts(0xE, &[5, 3, 9])]).await.unwrap();
    let t = t.write([facts(0xE, &[1, 7])]).await.unwrap();
    let snaps = publish(snaps, 0xE, t.version().unwrap(), 5).await;
    let first = build_bundle(snaps_dir.path(), facts_dir.path(), 0xE).await;
    // Later activity: another attempt's rows and another publication.
    let t = t.write([facts(0xF, &[2, 4])]).await.unwrap();
    let _snaps = publish(snaps, 0xF, t.version().unwrap(), 2).await;
    let second = build_bundle(snaps_dir.path(), facts_dir.path(), 0xE).await;
    println!(
        "P4 bundle bytes {} / {}; identical: {}; blake3 {}",
        first.len(),
        second.len(),
        first == second,
        blake3::hash(&first).to_hex()
    );
    assert_eq!(
        first, second,
        "rebuilding a published snapshot's bundle is byte-identical"
    );
}
