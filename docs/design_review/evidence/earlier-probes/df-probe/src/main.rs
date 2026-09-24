//! Scratch probe: DataFusion 55.1 capabilities against a copy of the pilot store.
//! Modes: info | validate <delta|cached> <target_partitions> <concurrency> | derive <pool_mib|0> <hash:1|0>
//!        | sort | explain

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use arrow_array::{Array, RecordBatch, UInt32Array};
use arrow_ord::sort::SortOptions;
use arrow_row::{RowConverter, SortField};
use cpg_core::snapshot::{Versions, register, resolve};
use cpg_core::sql;
use cpg_schema::derived::Derived;
use cpg_schema::id::Id;
use cpg_schema::rules::rules;
use cpg_schema::table::{Table, canonical_sort};
use datafusion::common::tree_node::TreeNodeRecursion;
use datafusion::datasource::MemTable;
use datafusion::error::Result;
use datafusion::execution::disk_manager::{DiskManagerBuilder, DiskManagerMode};
use datafusion::execution::memory_pool::{
    FairSpillPool, MemoryConsumer, MemoryLimit, MemoryPool, MemoryReservation, UnboundedMemoryPool,
};
use datafusion::execution::runtime_env::{RuntimeEnv, RuntimeEnvBuilder};
use datafusion::logical_expr::LogicalPlan;
use datafusion::physical_plan::{ExecutionPlan, collect, displayable};
use datafusion::prelude::SessionContext;
use deltalake::delta_datafusion::DeltaSessionContext;
use futures::StreamExt;

mod fast;

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

fn rss(field: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|k| k / 1024)
        .unwrap_or(0)
}

/// A pool wrapper that records the peak reserved bytes of its inner pool.
#[derive(Debug)]
struct PeakPool {
    inner: Arc<dyn MemoryPool>,
    peak: AtomicUsize,
}
impl fmt::Display for PeakPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PeakPool({})", self.inner.name())
    }
}
impl PeakPool {
    fn bump(&self) {
        self.peak
            .fetch_max(self.inner.reserved(), Ordering::Relaxed);
    }
}
impl MemoryPool for PeakPool {
    fn name(&self) -> &str {
        "peak"
    }
    fn register(&self, c: &MemoryConsumer) {
        self.inner.register(c)
    }
    fn unregister(&self, c: &MemoryConsumer) {
        self.inner.unregister(c)
    }
    fn grow(&self, r: &MemoryReservation, n: usize) {
        self.inner.grow(r, n);
        self.bump();
    }
    fn shrink(&self, r: &MemoryReservation, n: usize) {
        self.inner.shrink(r, n)
    }
    fn try_grow(&self, r: &MemoryReservation, n: usize) -> Result<()> {
        self.inner.try_grow(r, n)?;
        self.bump();
        Ok(())
    }
    fn reserved(&self) -> usize {
        self.inner.reserved()
    }
    fn memory_limit(&self) -> MemoryLimit {
        self.inner.memory_limit()
    }
}

fn runtime(pool: Arc<dyn MemoryPool>, spill: &Path) -> Arc<RuntimeEnv> {
    RuntimeEnvBuilder::new()
        .with_memory_pool(pool)
        .with_disk_manager_builder(
            DiskManagerBuilder::default()
                .with_mode(DiskManagerMode::Directories(vec![spill.to_path_buf()])),
        )
        .build_arc()
        .expect("runtime")
}

fn base_ctx(tp: usize, rt: Arc<RuntimeEnv>, prefer_hash: bool) -> SessionContext {
    let ctx = DeltaSessionContext::with_runtime_env(rt).into_inner();
    {
        let state = ctx.state_ref();
        let mut s = state.write();
        let o = s.config_mut().options_mut();
        o.execution.target_partitions = tp;
        o.optimizer.prefer_hash_join = prefer_hash;
    }
    ctx.register_udf(cpg_core::udf::lctx_id());
    ctx
}

fn parse_hex(s: &str) -> Id {
    let mut out = [0u8; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    Id(out)
}

async fn latest_snapshot(root: &Path) -> std::result::Result<(Id, Versions), BoxErr> {
    let versions = cpg_core::snapshot::latest(root).await?;
    let ctx = base_ctx(4, RuntimeEnvBuilder::new().build_arc()?, true);
    let t = cpg_core::snapshot::load_at(root, "snapshots", versions["snapshots"]).await?;
    t.update_datafusion_session(&ctx.state())?;
    ctx.register_table("snapshots", t.table_provider().await?)?;
    let b = sql::query(
        &ctx,
        "SELECT encode(snapshot_id, 'hex') AS h, max(table_version) AS v FROM snapshots \
         GROUP BY 1 ORDER BY 2 DESC LIMIT 1",
    )
    .await?
    .collect()
    .await?;
    let col = arrow_cast_utf8(b[0].column(0));
    let id = parse_hex(&col);
    let v = resolve(root, id).await?.expect("published");
    Ok((id, v))
}

fn arrow_cast_utf8(a: &arrow_array::ArrayRef) -> String {
    let c = datafusion::arrow::compute::cast(a, &arrow_schema::DataType::Utf8).unwrap();
    c.as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap()
        .value(0)
        .to_owned()
}

async fn delta_ctx(
    root: &Path,
    id: Id,
    v: &Versions,
    tp: usize,
    rt: Arc<RuntimeEnv>,
    hash: bool,
) -> std::result::Result<SessionContext, BoxErr> {
    let ctx = base_ctx(tp, rt, hash);
    for (name, version) in v {
        register(&ctx, root, name, *version, id).await?;
    }
    Ok(ctx)
}

/// Same session, but every table materialized once into a MemTable.
async fn cached_ctx(
    root: &Path,
    id: Id,
    v: &Versions,
    tp: usize,
    rt: Arc<RuntimeEnv>,
) -> std::result::Result<(SessionContext, usize), BoxErr> {
    let src = delta_ctx(root, id, v, tp, rt.clone(), true).await?;
    let hot = std::env::var("HOT").ok();
    let ctx = if hot.is_some() { delta_ctx(root, id, v, tp, rt, true).await? } else { base_ctx(tp, rt, true) };
    let mut bytes = 0;
    for name in v.keys() {
        if let Some(h) = &hot { if !h.split(',').any(|x| x == name) { continue; } ctx.deregister_table(name.as_str())?; }
        let df = src.table(name.as_str()).await?;
        let schema = Arc::new(df.schema().as_arrow().clone());
        let parts = df.collect_partitioned().await?;
        bytes += parts
            .iter()
            .flatten()
            .map(RecordBatch::get_array_memory_size)
            .sum::<usize>();
        ctx.register_table(name.as_str(), Arc::new(MemTable::try_new(schema, parts)?))?;
    }
    Ok((ctx, bytes))
}

#[derive(Default, Clone, Copy)]
struct Cost {
    sql: f64,
    phys: f64,
    exec: f64,
    rows: usize,
}

async fn run_rule(ctx: SessionContext, name: String, text: String) -> Result<(String, Cost)> {
    let t0 = Instant::now();
    let df = sql::query(&ctx, &text).await?;
    let t1 = Instant::now();
    let plan = df.create_physical_plan().await?;
    let t2 = Instant::now();
    let out = collect(plan, ctx.task_ctx()).await?;
    let t3 = Instant::now();
    Ok((
        name,
        Cost {
            sql: (t1 - t0).as_secs_f64(),
            phys: (t2 - t1).as_secs_f64(),
            exec: (t3 - t2).as_secs_f64(),
            rows: out.iter().map(RecordBatch::num_rows).sum(),
        },
    ))
}

async fn validate(ctx: &SessionContext, conc: usize) -> Result<(f64, BTreeMap<String, Cost>)> {
    let started = Instant::now();
    let mut out = BTreeMap::new();
    if conc <= 1 {
        for r in rules() {
            let (n, c) = run_rule(ctx.clone(), r.name, r.sql).await?;
            out.insert(n, c);
        }
    } else {
        let mut s = futures::stream::iter(rules().into_iter().map(|r| {
            let ctx = ctx.clone();
            tokio::spawn(run_rule(ctx, r.name, r.sql))
        }))
        .buffer_unordered(conc);
        while let Some(j) = s.next().await {
            let (n, c) = j.expect("join")?;
            out.insert(n, c);
        }
    }
    Ok((started.elapsed().as_secs_f64(), out))
}

fn tables_of(plan: &LogicalPlan) -> Vec<String> {
    let mut t = Vec::new();
    plan.apply_with_subqueries(|p| {
        match p {
            LogicalPlan::TableScan(s) => t.push(format!("scan:{}", s.table_name.table())),
            LogicalPlan::SubqueryAlias(a) => t.push(a.alias.table().to_owned()),
            _ => {}
        }
        Ok(TreeNodeRecursion::Continue)
    })
    .unwrap();
    t.sort();
    t.dedup();
    t
}

fn sum_metrics(
    plan: &Arc<dyn ExecutionPlan>,
    acc: &mut BTreeMap<String, (usize, usize, usize, usize)>,
) {
    if let Some(m) = plan.metrics() {
        let m = m.aggregate_by_name();
        let e = acc.entry(plan.name().to_owned()).or_default();
        e.0 += m.elapsed_compute().unwrap_or(0);
        e.1 += m.output_rows().unwrap_or(0);
        e.2 += m.spill_count().unwrap_or(0);
        e.3 += m.spilled_bytes().unwrap_or(0);
    }
    for c in plan.children() {
        sum_metrics(c, acc);
    }
}

async fn derive_probe<T: Derived>(
    ctx: &SessionContext,
    id: Id,
    pool: &PeakPool,
) -> std::result::Result<RecordBatch, BoxErr> {
    pool.peak.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let df = sql::query(ctx, &T::sql()).await?;
    let plan = df.create_physical_plan().await?;
    let res = collect(plan.clone(), ctx.task_ctx()).await;
    let el = t0.elapsed().as_secs_f64();
    match res {
        Err(e) => {
            println!(
                "derive {:<8} FAILED after {el:.2}s pool-peak {} MiB: {}",
                T::NAME,
                pool.peak.load(Ordering::Relaxed) >> 20,
                e.to_string().lines().next().unwrap_or("")
            );
            Err(Box::new(e))
        }
        Ok(batches) => {
            let mut acc = BTreeMap::new();
            sum_metrics(&plan, &mut acc);
            let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
            let bytes: usize = batches.iter().map(RecordBatch::get_array_memory_size).sum();
            println!(
                "derive {:<8} ok {el:.2}s rows {rows} result {} MiB pool-peak {} MiB rss {} MiB hwm {} MiB",
                T::NAME,
                bytes >> 20,
                pool.peak.load(Ordering::Relaxed) >> 20,
                rss("VmRSS:"),
                rss("VmHWM:")
            );
            for (op, (ns, rows, sc, sb)) in acc {
                if ns > 5_000_000 || sc > 0 {
                    println!(
                        "    {op:<28} compute {:>7.3}s rows {rows:>9} spills {sc} ({} MiB)",
                        ns as f64 / 1e9,
                        sb >> 20
                    );
                }
            }
            // the production path, for the batch the sort probe uses
            let t = Instant::now();
            let b = cpg_core::derive::derive::<T>(ctx, id).await?;
            println!(
                "    cpg_core::derive (query+cast+concat+sort) {:.2}s",
                t.elapsed().as_secs_f64()
            );
            Ok(b)
        }
    }
}

fn row_sort(batch: &RecordBatch, key: &[&str]) -> RecordBatch {
    let cols: Vec<_> = key
        .iter()
        .map(|k| batch.column(batch.schema().index_of(k).unwrap()).clone())
        .collect();
    let fields = cols
        .iter()
        .map(|c| {
            SortField::new_with_options(
                c.data_type().clone(),
                SortOptions {
                    descending: false,
                    nulls_first: true,
                },
            )
        })
        .collect();
    let conv = RowConverter::new(fields).unwrap();
    let rows = conv.convert_columns(&cols).unwrap();
    let mut idx: Vec<u32> = (0..batch.num_rows() as u32).collect();
    idx.sort_unstable_by(|&a, &b| rows.row(a as usize).cmp(&rows.row(b as usize)));
    arrow_select::take::take_record_batch(batch, &UInt32Array::from(idx)).unwrap()
}

#[tokio::main]
async fn main() -> std::result::Result<(), BoxErr> {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(std::env::var("STORE").expect("STORE"));
    let spill = PathBuf::from(std::env::var("SPILL").expect("SPILL"));
    let (id, versions) = latest_snapshot(&root).await?;
    println!(
        "snapshot {} with {} tables; rules {}",
        id.hex(),
        versions.len(),
        rules().len()
    );
    let mode = args.get(1).map(String::as_str).unwrap_or("info");
    match mode {
        "info" => {
            let rt = RuntimeEnvBuilder::new().build_arc()?;
            let ctx = delta_ctx(&root, id, &versions, 32, rt, true).await?;
            // which tables each rule and derivation reads (unoptimized logical plan, views not inlined)
            let mut per_table: BTreeMap<String, usize> = BTreeMap::new();
            let known = |t: &String| versions.contains_key(t.as_str()) || t.starts_with("scan:");
            for r in rules() {
                let p = ctx.state().create_logical_plan(&r.sql).await?;
                if r.name == "unique:type_terms" { println!("{}", p.display_indent()); }
                for t in tables_of(&p).into_iter().filter(known) {
                    *per_table.entry(t).or_default() += 1;
                }
            }
            let mut v: Vec<_> = per_table.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            println!(
                "rules reading each table (top 15): {:?}",
                &v[..15.min(v.len())]
            );
            println!("tables read by any rule: {}", v.len());
            let order: Vec<(&str, String)> = cpg_schema::derived::derivations();
            let derived: Vec<&str> = order.iter().map(|(n, _)| *n).collect();
            let mut bad = 0;
            for (i, (name, text)) in order.iter().enumerate() {
                let p = ctx.state().create_logical_plan(text).await?;
                let reads: Vec<String> = tables_of(&p).into_iter().filter(|t| versions.contains_key(t.as_str())).collect();
                let later: Vec<_> = reads
                    .iter()
                    .filter(|t| derived[i..].contains(&t.as_str()))
                    .collect();
                if !later.is_empty() {
                    bad += 1;
                    println!("derivation {name} reads itself or a later derivation: {later:?}");
                }
                if *name == "edges" || *name == "nodes" {
                    println!("derivation {name} reads {} tables", reads.len());
                }
            }
            println!("derivation-order violations: {bad}");
            // EXPLAIN through the read-only helper
            for q in [
                "EXPLAIN SELECT count(*) FROM edges",
                "EXPLAIN ANALYZE SELECT node_id FROM type_terms GROUP BY node_id HAVING count(DISTINCT kind) > 1",
            ] {
                match sql::query(&ctx, q).await {
                    Ok(df) => {
                        let b = df.collect().await?;
                        let s =
                            datafusion::arrow::util::pretty::pretty_format_batches(&b)?.to_string();
                        println!("{q}: allowed by read_only(); {} lines", s.lines().count());
                        if q.contains("ANALYZE") {
                            for l in s.lines().filter(|l| l.contains("metrics")).take(6) {
                                println!("  {}", &l[..l.len().min(260)]);
                            }
                        }
                    }
                    Err(e) => println!("{q}: refused: {e}"),
                }
            }
        }
        "validate" => {
            let kind = args.get(2).map(String::as_str).unwrap_or("delta");
            let tp: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(32);
            let conc: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
            let pool_mib: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
            let inner: Arc<dyn MemoryPool> = if pool_mib == 0 {
                Arc::new(UnboundedMemoryPool::default())
            } else {
                Arc::new(FairSpillPool::new(pool_mib << 20))
            };
            let pool = Arc::new(PeakPool {
                inner,
                peak: AtomicUsize::new(0),
            });
            let rt = runtime(pool.clone(), &spill);
            let hwm0 = rss("VmHWM:");
            let t = Instant::now();
            let (ctx, bytes) = if kind == "cached" {
                cached_ctx(&root, id, &versions, tp, rt).await?
            } else {
                (delta_ctx(&root, id, &versions, tp, rt, true).await?, 0)
            };
            let setup = t.elapsed().as_secs_f64();
            let hwm1 = rss("VmHWM:");
            let (wall, costs) = validate(&ctx, conc).await?;
            let s = costs.values().fold(Cost::default(), |a, c| Cost {
                sql: a.sql + c.sql,
                phys: a.phys + c.phys,
                exec: a.exec + c.exec,
                rows: a.rows + c.rows,
            });
            let failing = costs.values().filter(|c| c.rows > 0).count();
            let mut slow: Vec<_> = costs
                .iter()
                .map(|(n, c)| (c.sql + c.phys + c.exec, n))
                .collect();
            slow.sort_by(|a, b| b.0.total_cmp(&a.0));
            println!(
                "validate {kind} tp={tp} conc={conc} pool={pool_mib}MiB: setup {setup:.2}s (cached {} MiB) \
                 rules {} wall {wall:.2}s | sum sql {:.2}s phys {:.2}s exec {:.2}s | failing {failing} rows {} \
                 | pool-peak {} MiB | hwm before {hwm0} after-setup {hwm1} end {} MiB rss {} MiB",
                bytes >> 20,
                costs.len(),
                s.sql,
                s.phys,
                s.exec,
                s.rows,
                pool.peak.load(Ordering::Relaxed) >> 20,
                rss("VmHWM:"),
                rss("VmRSS:")
            );
            println!("  slowest: {:?}", &slow[..3]);
        }
        "derive" => {
            let pool_mib: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            let hash = args.get(3).map(|s| s == "1").unwrap_or(true);
            let tp: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(32);
            let inner: Arc<dyn MemoryPool> = if pool_mib == 0 {
                Arc::new(UnboundedMemoryPool::default())
            } else {
                Arc::new(FairSpillPool::new(pool_mib << 20))
            };
            let pool = Arc::new(PeakPool {
                inner,
                peak: AtomicUsize::new(0),
            });
            let rt = runtime(pool.clone(), &spill);
            let ctx = delta_ctx(&root, id, &versions, tp, rt, hash).await?;
            println!("derive pool={pool_mib}MiB prefer_hash_join={hash} tp={tp}");
            let _ = derive_probe::<cpg_schema::graph::Nodes>(&ctx, id, &pool).await;
            let _ = derive_probe::<cpg_schema::graph::Edges>(&ctx, id, &pool).await;
        }
        "sort" => {
            let rt = RuntimeEnvBuilder::new().build_arc()?;
            let ctx = delta_ctx(&root, id, &versions, 32, rt, true).await?;
            let batch = cpg_core::derive::derive::<cpg_schema::graph::Edges>(&ctx, id).await?;
            // shuffle deterministically by reversing, so the sort has work to do
            let n = batch.num_rows() as u32;
            let rev = UInt32Array::from((0..n).rev().collect::<Vec<_>>());
            let batch = arrow_select::take::take_record_batch(&batch, &rev)?;
            let key = cpg_schema::graph::Edges::key();
            for _ in 0..2 {
                let t = Instant::now();
                let a = canonical_sort(&batch, key)?;
                let ta = t.elapsed().as_secs_f64();
                let t = Instant::now();
                let b = row_sort(&batch, key);
                let tb = t.elapsed().as_secs_f64();
                let t = Instant::now();
                let c = canonical_sort(&batch, &key[1..])?;
                let tc = t.elapsed().as_secs_f64();
                println!(
                    "sort edges rows {n} key {key:?}: lexsort_to_indices {ta:.3}s | RowConverter {tb:.3}s (equal {}) | lexsort without the constant snapshot_id {tc:.3}s (equal {})",
                    a == b,
                    a == c
                );
            }
            // DataFusion's sort in the plan, instead of Arrow after collect
            let q = format!(
                "SELECT * FROM ({}) d ORDER BY edge_id ASC NULLS FIRST",
                cpg_schema::graph::Edges::sql()
            );
            let t = Instant::now();
            let b = sql::query(&ctx, &q).await?.collect().await?;
            let rows: usize = b.iter().map(RecordBatch::num_rows).sum();
            println!(
                "edges query with ORDER BY in DataFusion: {:.2}s rows {rows} batches {}",
                t.elapsed().as_secs_f64(),
                b.len()
            );
            let t = Instant::now();
            let _ = sql::query(&ctx, &cpg_schema::graph::Edges::sql())
                .await?
                .collect()
                .await?;
            println!(
                "edges query without ORDER BY: {:.2}s",
                t.elapsed().as_secs_f64()
            );
        }
        "plan" => {
            let rt = RuntimeEnvBuilder::new().build_arc()?;
            let ctx = delta_ctx(&root, id, &versions, 32, rt, true).await?;
            let name = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "unique:type_terms".into());
            let r = rules().into_iter().find(|r| r.name == name).expect("rule");
            let plan = sql::query(&ctx, &r.sql)
                .await?
                .create_physical_plan()
                .await?;
            let _ = collect(plan.clone(), ctx.task_ctx()).await?;
            println!(
                "{}",
                datafusion::physical_plan::display::DisplayableExecutionPlan::with_metrics(
                    plan.as_ref()
                )
                .indent(true)
            );
            let _ = displayable(plan.as_ref());
        }
        "udf" => {
            let rt = RuntimeEnvBuilder::new().build_arc()?;
            let src = delta_ctx(&root, id, &versions, 1, rt.clone(), true).await?;
            let ctx = base_ctx(1, rt, true);
            ctx.register_udf(fast::lctx_fast());
            let df = src.table("edges").await?;
            let schema = Arc::new(df.schema().as_arrow().clone());
            let parts = df.collect_partitioned().await?;
            ctx.register_table("edges", Arc::new(MemTable::try_new(schema, parts)?))?;
            let args = "'edge', edge_kind, src_node_id, dst_node_id, ordinal, CAST(NULL AS BYTEA)";
            for _ in 0..3 {
                for f in ["lctx_id", "lctx_fast"] {
                    let q = format!("SELECT count(*) FROM (SELECT {f}({args}) AS x FROM edges) t WHERE x <> X'00000000000000000000000000000000'");
                    let t = Instant::now();
                    let _ = sql::query(&ctx, &q).await?.collect().await?;
                    println!("{f}: {:.3}s over 1449162 rows, one partition", t.elapsed().as_secs_f64());
                }
            }
            let q = format!("SELECT count(*) AS differ FROM edges WHERE lctx_id({args}) <> lctx_fast({args})");
            let b = sql::query(&ctx, &q).await?.collect().await?;
            println!("{}", datafusion::arrow::util::pretty::pretty_format_batches(&b)?);
            for (f, q) in [("lctx_id", "SELECT lctx_id('k', 'x') IS NULL"), ("lctx_fast", "SELECT lctx_fast('k', 'x') IS NULL")] {
                let df = sql::query(&ctx, q).await?;
                println!("{f} simplified plan: {}", df.into_optimized_plan()?.display_indent().to_string().replace('\n', " / "));
            }
        }
        "breakdown" => {
            let rt = RuntimeEnvBuilder::new().build_arc()?;
            let ctx = delta_ctx(&root, id, &versions, 32, rt, true).await?;
            use cpg_schema::graph::Edges;
            let t = Instant::now();
            let batches = sql::query(&ctx, &Edges::sql()).await?.collect().await?;
            let tq = t.elapsed().as_secs_f64();
            println!("result types: {:?}", batches[0].schema().fields().iter().map(|f| format!("{}:{}", f.name(), f.data_type())).collect::<Vec<_>>());
            let t = Instant::now();
            let mut declared = Vec::new();
            for b in &batches {
                let mut fields = vec![Arc::new(arrow_schema::Field::new("snapshot_id", arrow_schema::DataType::FixedSizeBinary(16), false))];
                fields.extend(b.schema().fields().iter().cloned());
                let sid = datafusion::common::ScalarValue::FixedSizeBinary(16, Some(id.0.to_vec())).to_array_of_size(b.num_rows())?;
                let mut cols = vec![sid];
                cols.extend(b.columns().iter().cloned());
                let w = RecordBatch::try_new(Arc::new(arrow_schema::Schema::new(fields)), cols)?;
                declared.push(cpg_core::delta::to_declared::<Edges>(&w)?);
            }
            let tc = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let one = arrow_select::concat::concat_batches(&Edges::schema(), &declared)?;
            let tcat = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let a = canonical_sort(&one, Edges::key())?;
            let ts = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let b = row_sort(&one, Edges::key());
            let tr = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let c = canonical_sort(&one, &Edges::key()[1..])?;
            let t1 = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let idx = arrow_ord::sort::sort_to_indices(one.column(one.schema().index_of("edge_id")?), Some(SortOptions { descending: false, nulls_first: true }), None)?;
            let d = arrow_select::take::take_record_batch(&one, &idx)?;
            let t2 = t.elapsed().as_secs_f64();
            println!("sort of the real derive order ({} rows): lexsort 2 cols {ts:.3}s | RowConverter {tr:.3}s eq {} | lexsort edge_id only {t1:.3}s eq {} | sort_to_indices edge_id {t2:.3}s eq {}", one.num_rows(), a == b, a == c, a == d);
            println!("edges derive breakdown: query {tq:.3}s | add snapshot_id + to_declared casts {tc:.3}s ({} batches) | concat {tcat:.3}s | canonical_sort {ts:.3}s", batches.len());
        }
        _ => println!("unknown mode"),
    }
    Ok(())
}
