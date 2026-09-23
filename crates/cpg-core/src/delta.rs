//! Fact-family tables in Delta (DESIGN §4.3, §6.1–§6.3).
//!
//! Create: `appendOnly` plus the table's immutable CHECKs via `add_constraint` (`CreateBuilder`
//! rejects `delta.constraints.*`). Open: verify both against the declaration without writing.
//! Write: `DeltaTable::write` only, the path that enforces CHECKs and invariants. Read: pinned
//! version, `snapshot_id` filter, total order, and the cast back to the declared schema (§3.3).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

use arrow_array::{ArrayRef, RecordBatch};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_schema::DataType;
use cpg_schema::id::Id;
use cpg_schema::table::Table;
use datafusion::common::DFSchema;
use datafusion::optimizer::simplify_expressions::{ExprSimplifier, SimplifyContext};
use datafusion::prelude::SessionContext;
use deltalake::delta_datafusion::create_session;
use deltalake::delta_datafusion::expr::{fmt_expr_to_sql, parse_predicate_expression};
use deltalake::kernel::StructType;
use deltalake::kernel::engine::arrow_conversion::TryIntoKernel as _;
use deltalake::kernel::transaction::CommitProperties;
use deltalake::{DeltaTable, DeltaTableBuilder, TableProperty};
use url::Url;

use crate::{CoreError, sql};

const STRICT: CastOptions<'static> = CastOptions {
    safe: false,
    format_options: arrow_cast::display::FormatOptions::new(),
};

/// Key prefix delta-rs stores CHECK constraints under.
const CONSTRAINT_PREFIX: &str = "delta.constraints.";

/// Retention for pinned reads (DESIGN §6.1, ADR-0014). delta-rs's post-commit hook checkpoints
/// every 100 versions and then deletes the log below the newest checkpoint older than 30 days,
/// after which an old version no longer loads. Every table turns that cleanup off; checkpoints
/// stay on. The values are compared as exact strings at open, because delta-rs silently falls
/// back to its default on a value it cannot parse.
pub fn retention() -> [(TableProperty, &'static str); 2] {
    [
        (TableProperty::EnableExpiredLogCleanup, "false"),
        (TableProperty::LogRetentionDuration, "interval 36500 days"),
    ]
}

/// The table's location. Touches nothing: a reader of a missing table gets "not a table".
pub fn table_url(root: &Path, name: &str) -> Result<Url, CoreError> {
    let dir = std::path::absolute(root.join(name))?;
    Url::from_directory_path(&dir).map_err(|()| CoreError::Url(dir.display().to_string()))
}

/// Create the table: its declared schema, `delta.appendOnly`, the retention properties, then its
/// immutable CHECKs.
pub async fn create<T: Table>(root: &Path) -> Result<DeltaTable, CoreError> {
    fs_err::create_dir_all(root.join(T::NAME))?;
    let kernel: StructType = T::schema().as_ref().try_into_kernel()?;
    let mut builder = DeltaTable::try_from_url(table_url(root, T::NAME)?)
        .await?
        .create()
        .with_columns(kernel.fields().cloned())
        .with_configuration_property(TableProperty::AppendOnly, Some("true"));
    for (property, value) in retention() {
        builder = builder.with_configuration_property(property, Some(value));
    }
    let table = builder.await?;
    if T::checks().is_empty() {
        return Ok(table);
    }
    let mut builder = table.add_constraint();
    for (name, expr) in T::checks() {
        builder = builder.with_constraint(*name, *expr);
    }
    Ok(builder.await?)
}

/// The CHECKs as delta-rs stores them. `add_constraint` parses each expression, coerces its
/// literals to the column types, simplifies it with DataFusion's `ExprSimplifier` (which
/// canonicalizes `end_byte >= start_byte` to `start_byte <= end_byte`) and renders it back to
/// SQL. The same parse, simplifier and renderer run here; literal coercion is a no-op because
/// every declared CHECK compares `Int64` columns with integer literals.
fn expected_constraints<T: Table>() -> Result<BTreeMap<String, String>, CoreError> {
    let schema = Arc::new(DFSchema::try_from(T::schema().as_ref().clone())?);
    let state = create_session().state();
    let context = SimplifyContext::builder()
        .with_schema(schema.clone())
        .with_config_options(state.config().options().clone())
        .build();
    let simplifier = ExprSimplifier::new(context);
    T::checks()
        .iter()
        .map(|(name, expr)| {
            let parsed = parse_predicate_expression(&schema, *expr, &state)?;
            let simplified = simplifier.simplify(parsed)?;
            Ok(((*name).to_owned(), fmt_expr_to_sql(&simplified)?))
        })
        .collect()
}

/// Refuse a table whose schema, `appendOnly` or CHECK set differs from the declaration, including
/// one left without constraints by a crash between create and `add_constraint`. Never writes.
pub fn verify<T: Table>(table: &DeltaTable) -> Result<(), CoreError> {
    // A changed contract is a migration (DESIGN §6.3): refuse the old table by name, rather than
    // let the write fail on a cast.
    let declared: StructType = T::schema().as_ref().try_into_kernel()?;
    if table.snapshot()?.schema().as_ref() != &declared {
        return Err(CoreError::SchemaDrift(T::NAME));
    }
    let config = table.snapshot()?.metadata().configuration().clone();
    if config.get("delta.appendOnly").map(String::as_str) != Some("true") {
        return Err(CoreError::NotAppendOnly(T::NAME));
    }
    for (property, value) in retention() {
        let key = property.as_ref();
        if config.get(key).map(String::as_str) != Some(value) {
            return Err(CoreError::Retention {
                table: T::NAME,
                property: key.to_owned(),
                found: config.get(key).cloned(),
            });
        }
    }
    let stored: BTreeMap<String, String> = config
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix(CONSTRAINT_PREFIX)
                .map(|n| (n.to_owned(), v.clone()))
        })
        .collect();
    let expected = expected_constraints::<T>()?;
    if stored != expected {
        return Err(CoreError::ConstraintMismatch {
            table: T::NAME,
            detail: format!("stored {stored:?}, declared {expected:?}"),
        });
    }
    Ok(())
}

pub async fn open_verified<T: Table>(root: &Path) -> Result<DeltaTable, CoreError> {
    let table = DeltaTableBuilder::from_url(table_url(root, T::NAME)?)?
        .load()
        .await?;
    verify::<T>(&table)?;
    Ok(table)
}

pub async fn open_or_create<T: Table>(root: &Path) -> Result<DeltaTable, CoreError> {
    if root.join(T::NAME).join("_delta_log").exists() {
        open_verified::<T>(root).await
    } else {
        create::<T>(root).await
    }
}

/// Append one attempt's rows through `DeltaTable::write` (CHECK and non-null enforced). The
/// `lctx.snapshot_id` commit metadata is audit only; `snapshots` stays the authority.
pub async fn append(
    table: DeltaTable,
    batch: RecordBatch,
    snapshot_id: Id,
) -> Result<DeltaTable, CoreError> {
    let props = CommitProperties::default().with_metadata([(
        "lctx.snapshot_id".to_owned(),
        serde_json::Value::String(snapshot_id.hex()),
    )]);
    // Delta keeps field metadata (codebook names) but not schema-level metadata; strip the latter
    // at the boundary. `to_declared` restores the full declared schema on read.
    let bare = Arc::new(
        batch
            .schema()
            .as_ref()
            .clone()
            .with_metadata(HashMap::new()),
    );
    let batch = RecordBatch::try_new(bare, batch.columns().to_vec())?;
    Ok(table
        .write([batch])
        .with_writer_properties(writer_properties()?)
        .with_commit_properties(props)
        .await?)
}

/// zstd level 3 for every data file (H1 P6): 13.7% smaller than the Snappy default on the pilot
/// store (`source_files` 36.5%), scans unchanged; dictionary encoding and page statistics stay at
/// their defaults. Content is unaffected, and files written before read as they are.
pub fn writer_properties() -> Result<WriterProperties, CoreError> {
    let level = ZstdLevel::try_new(3).map_err(|e| CoreError::Parquet(e.to_string()))?;
    Ok(WriterProperties::builder()
        .set_compression(Compression::ZSTD(level))
        .build())
}

/// Cast a batch read back from Delta to the declared schema: `BinaryView → Binary →
/// FixedSizeBinary` in two steps (no direct cast exists), `Utf8View → Utf8`, and list children
/// back to their declared field (§3.3). Casts are strict: a wrong width is an error.
pub fn to_declared<T: Table>(batch: &RecordBatch) -> Result<RecordBatch, CoreError> {
    let schema = T::schema();
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let col = batch.column(batch.schema().index_of(field.name())?).clone();
        let cast = match field.data_type() {
            DataType::FixedSizeBinary(_) => {
                let binary = cast_with_options(&col, &DataType::Binary, &STRICT)?;
                cast_with_options(&binary, field.data_type(), &STRICT)?
            }
            other => cast_with_options(&col, other, &STRICT)?,
        };
        columns.push(cast);
    }
    Ok(RecordBatch::try_new(schema, columns)?)
}

/// Read one snapshot's rows at a pinned version, in the declared total order.
pub async fn read_at<T: Table>(
    root: &Path,
    version: u64,
    snapshot_id: Id,
) -> Result<RecordBatch, CoreError> {
    let table = crate::snapshot::load_at(root, T::NAME, version).await?;
    let ctx: SessionContext = create_session().into_inner();
    table.update_datafusion_session(&ctx.state())?;
    ctx.register_table(
        "t",
        crate::snapshot::commit_provider(&table, version).await?,
    )?;
    let quote = |c: &str| format!("\"{c}\"");
    let schema = T::schema();
    let columns: Vec<String> = schema.fields().iter().map(|f| quote(f.name())).collect();
    // The canonical sort puts nulls first; SQL's default is NULLS LAST for ASC.
    let order: Vec<String> = T::key()
        .iter()
        .map(|k| format!("{} ASC NULLS FIRST", quote(k)))
        .collect();
    let statement = format!(
        "SELECT {} FROM t WHERE snapshot_id = X'{}' ORDER BY {}",
        columns.join(", "),
        snapshot_id.hex(),
        order.join(", ")
    );
    let batches = sql::query(&ctx, &statement).await?.collect().await?;
    let declared: Vec<RecordBatch> = batches
        .iter()
        .map(to_declared::<T>)
        .collect::<Result<_, _>>()?;
    Ok(arrow_select::concat::concat_batches(&schema, &declared)?)
}
