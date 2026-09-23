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
use deltalake::kernel::{Action, Add};
use deltalake::logstore::get_actions;
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

/// The actions of `table`'s commit `version`. The JSON commits are kept (log cleanup off, verified
/// at open), so the entry is always there to read.
async fn commit_actions(table: &DeltaTable, version: u64) -> Result<Vec<Action>, CoreError> {
    let bytes = table
        .log_store()
        .read_commit_entry(version)
        .await?
        .ok_or_else(|| CoreError::VersionMismatch {
            table: table.table_url().to_string(),
            requested: version,
            loaded: None,
        })?;
    Ok(get_actions(version, &bytes)?)
}

/// The snapshot a commit's `commitInfo` records (`lctx.snapshot_id`, which `delta::append` writes
/// on every data commit), if any.
fn recorded_snapshot(actions: &[Action]) -> Option<String> {
    actions.iter().find_map(|a| match a {
        Action::CommitInfo(ci) => ci
            .info
            .get("lctx.snapshot_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        _ => None,
    })
}

/// The data files commit `version` of `table` added, once the commit is shown to be
/// `snapshot_id`'s. A snapshot's rows of a table are exactly one commit's (one commit per table
/// per attempt, §6.1), so a pinned read opens these files and no other (H1 P3). The commit's own
/// `lctx.snapshot_id` must name `snapshot_id`: a commit of another snapshot, or of none, is
/// refused rather than read as an empty or foreign snapshot (H1 review F2). `snapshots` remains
/// the authority for which version; the commit metadata only confirms it.
pub async fn commit_adds(
    table: &DeltaTable,
    version: u64,
    snapshot_id: Id,
) -> Result<Vec<Add>, CoreError> {
    let actions = commit_actions(table, version).await?;
    let recorded = recorded_snapshot(&actions);
    if recorded.as_deref() != Some(snapshot_id.hex().as_str()) {
        return Err(CoreError::ForeignCommit {
            table: table.table_url().to_string(),
            version,
            snapshot: snapshot_id.hex(),
            recorded,
        });
    }
    Ok(actions
        .into_iter()
        .filter_map(|a| match a {
            Action::Add(add) => Some(add),
            _ => None,
        })
        .collect())
}

/// A provider over `table` (loaded at `version`) that reads only the files that commit added,
/// after [`commit_adds`] confirms the commit is `snapshot_id`'s. A selected file the snapshot does
/// not hold active fails the scan, never a silent shortfall.
pub async fn commit_provider(
    table: &DeltaTable,
    version: u64,
    snapshot_id: Id,
) -> Result<std::sync::Arc<dyn datafusion::catalog::TableProvider>, CoreError> {
    let adds = commit_adds(table, version, snapshot_id).await?;
    Ok(table.table_provider().with_adds(adds).await?)
}

/// Register `name` at `version`, filtered to `snapshot_id`, as a view under its own name. Only the
/// files of the commit at `version` are read (H1 P3); the `snapshot_id` filter stays as the row
/// predicate.
pub async fn register(
    ctx: &SessionContext,
    root: &Path,
    name: &str,
    version: u64,
    snapshot_id: Id,
) -> Result<(), CoreError> {
    let table = load_at(root, name, version).await?;
    table.update_datafusion_session(&ctx.state())?;
    // A global table (ADR-0017 amendment) is read at its version over all its files: it has no
    // snapshot column, and its version may be another attempt's commit.
    if cpg_schema::embedding::is_global(name) {
        // It may stand registered as empty (a session opened before the attempt embedded).
        ctx.deregister_table(name)?;
        ctx.register_table(name, table.table_provider().await?)?;
        return Ok(());
    }
    let view = ctx
        .read_table(commit_provider(&table, version, snapshot_id).await?)?
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
    register_empty_globals(&ctx, versions)?;
    Ok(ctx)
}

/// A snapshot that used no global table (no embedder) reads it as empty, so every rule and
/// reader that names it still plans (ADR-0017 amendment).
pub fn register_empty_globals(ctx: &SessionContext, versions: &Versions) -> Result<(), CoreError> {
    macro_rules! empty {
        ($($t:ty),+) => {$(
            let name = <$t as Table>::NAME;
            if !versions.contains_key(name) && !ctx.table_exist(name)? {
                let schema = <$t as Table>::schema();
                let table = datafusion::datasource::MemTable::try_new(schema, vec![vec![]])?;
                ctx.register_table(name, std::sync::Arc::new(table))?;
            }
        )+};
    }
    cpg_schema::for_each_global_table!(empty);
    Ok(())
}

/// Each table's version holding `snapshot_id`'s own commit (its `lctx.snapshot_id`), found by
/// walking the table's kept JSON commits from the latest down. This is how an attempt that
/// published nothing (validation rejected it) is inspected, never how a reader reads: the
/// tables it did not write are left out, so a query naming them fails rather than reading an
/// empty snapshot (H1 review F2).
pub async fn attempt_versions(root: &Path, snapshot_id: Id) -> Result<Versions, CoreError> {
    let mut names: Vec<String> = fs_err::read_dir(root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("_delta_log").is_dir())
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    let wanted = snapshot_id.hex();
    let mut versions = Versions::new();
    for name in names {
        let table = DeltaTableBuilder::from_url(table_url(root, &name)?)?
            .load()
            .await?;
        let Some(latest) = table.version() else {
            continue;
        };
        // A global table the attempt may not have committed to (every key cached) is inspected
        // at its latest version (ADR-0019 review O5).
        if cpg_schema::embedding::is_global(&name) {
            versions.insert(name, latest);
            continue;
        }
        for v in (0..=latest).rev() {
            if recorded_snapshot(&commit_actions(&table, v).await?).as_deref() == Some(&wanted) {
                versions.insert(name, v);
                break;
            }
        }
    }
    if versions.is_empty() {
        return Err(CoreError::NoAttempt(wanted));
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
