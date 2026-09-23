//! One snapshot's tables as a DataFusion session (DESIGN §6.2): each table loaded at a pinned
//! version, the version asserted, filtered to the snapshot, and registered under its own name.
//! Derivation, validation and readers all query through such a session.

use std::collections::BTreeMap;
use std::path::Path;

use cpg_schema::id::Id;
use cpg_schema::table::Table;
use cpg_schema::tables::Snapshots;
use datafusion::common::ScalarValue;
use datafusion::execution::SessionStateBuilder;
use datafusion::prelude::{SessionContext, col, lit};
use deltalake::delta_datafusion::create_session;
use deltalake::{DeltaTable, DeltaTableBuilder};

use crate::delta::table_url;
use crate::{CoreError, sql};

/// Table name → the Delta version a snapshot's rows are visible at.
pub type Versions = BTreeMap<String, u64>;

/// Partitions per plan (H1 P2): the 32-thread default doubled derivation's working set for no
/// wall time (deriving `nodes`: 433 → 152 MiB accounted, +0.1 s), and validation runs
/// `validate::CONCURRENT_RULES` plans at once. Outputs are canonically sorted, so no result depends
/// on it.
pub const TARGET_PARTITIONS: usize = 8;

/// A session over Delta's planner defaults, with [`TARGET_PARTITIONS`], the `lctx_id` UDF (§3.4.1)
/// and no tables.
pub fn empty_session() -> SessionContext {
    let state = create_session().into_inner().state();
    let config = state
        .config()
        .clone()
        .with_target_partitions(TARGET_PARTITIONS);
    let ctx = SessionContext::new_with_state(
        SessionStateBuilder::new_from_existing(state)
            .with_config(config)
            .build(),
    );
    ctx.register_udf(crate::udf::lctx_id());
    ctx
}

/// Load a table at exactly `version`. A provider built on an already-loaded handle ignores the
/// requested version (`delta.open.2`), so every pinned read loads afresh and asserts it.
pub async fn load_at(root: &Path, name: &str, version: u64) -> Result<DeltaTable, CoreError> {
    let table = DeltaTableBuilder::from_url(table_url(root, name)?)?
        .with_version(version)
        .load()
        .await?;
    if table.version() != Some(version) {
        return Err(CoreError::VersionMismatch {
            table: name.to_owned(),
            requested: version,
            loaded: table.version(),
        });
    }
    Ok(table)
}

/// Register `name` at `version`, filtered to `snapshot_id`, as a view under its own name.
pub async fn register(
    ctx: &SessionContext,
    root: &Path,
    name: &str,
    version: u64,
    snapshot_id: Id,
) -> Result<(), CoreError> {
    let table = load_at(root, name, version).await?;
    table.update_datafusion_session(&ctx.state())?;
    let view = ctx
        .read_table(table.table_provider().await?)?
        .filter(col("snapshot_id").eq(lit(ScalarValue::Binary(Some(snapshot_id.0.to_vec())))))?
        .into_view();
    ctx.register_table(name, view)?;
    Ok(())
}

/// A session over `versions`, every table filtered to `snapshot_id`.
pub async fn session(
    root: &Path,
    snapshot_id: Id,
    versions: &Versions,
) -> Result<SessionContext, CoreError> {
    let ctx = empty_session();
    for (name, version) in versions {
        register(&ctx, root, name, *version, snapshot_id).await?;
    }
    Ok(ctx)
}

/// Every table of the store at its latest version. What an attempt that failed validation wrote is
/// there, though no `snapshots` row publishes it: for inspecting that attempt, never for a reader.
pub async fn latest(root: &Path) -> Result<Versions, CoreError> {
    let mut versions = Versions::new();
    let mut names: Vec<String> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("_delta_log").is_dir())
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    for name in names {
        let table = DeltaTableBuilder::from_url(table_url(root, &name)?)?
            .load()
            .await?;
        if let Some(v) = table.version() {
            versions.insert(name, v);
        }
    }
    Ok(versions)
}

/// The published row set of `snapshot_id`: `None` unless its `snapshots` append committed. The
/// latest `snapshots` version is the authority (it is append-only).
pub async fn resolve(root: &Path, snapshot_id: Id) -> Result<Option<Versions>, CoreError> {
    if !root.join(Snapshots::NAME).join("_delta_log").exists() {
        return Ok(None);
    }
    let table = DeltaTableBuilder::from_url(table_url(root, Snapshots::NAME)?)?
        .load()
        .await?;
    let ctx = empty_session();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(Snapshots::NAME, table.table_provider().await?)?;
    let statement = format!(
        "SELECT table_name, table_version FROM {} WHERE snapshot_id = X'{}' ORDER BY table_name",
        Snapshots::NAME,
        snapshot_id.hex()
    );
    let mut versions = Versions::new();
    for batch in sql::query(&ctx, &statement).await?.collect().await? {
        let names = arrow_cast::cast(batch.column(0), &arrow_schema::DataType::Utf8)?;
        let names = names
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .ok_or(CoreError::ColumnType("snapshots.table_name"))?;
        let found = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow_array::Int64Array>()
            .ok_or(CoreError::ColumnType("snapshots.table_version"))?;
        for (name, version) in names.iter().zip(found.iter()) {
            if let (Some(name), Some(version)) = (name, version) {
                versions.insert(name.to_owned(), u64::try_from(version).unwrap_or(u64::MAX));
            }
        }
    }
    Ok((!versions.is_empty()).then_some(versions))
}

/// A reader's session over a published snapshot; `None` if it was never published.
pub async fn published(
    root: &Path,
    snapshot_id: Id,
) -> Result<Option<(Versions, SessionContext)>, CoreError> {
    match resolve(root, snapshot_id).await? {
        Some(versions) => {
            let ctx = session(root, snapshot_id, &versions).await?;
            Ok(Some((versions, ctx)))
        }
        None => Ok(None),
    }
}
