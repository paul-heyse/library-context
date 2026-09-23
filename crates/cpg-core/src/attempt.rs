//! One compile attempt (DESIGN §6.1, ADR-0009): write every raw table, derive (Stage C/D),
//! validate (§8), and only then publish with one `snapshots` append. Any error aborts the attempt;
//! a retry is a new attempt with a new `snapshot_id`. Rows of an unpublished attempt stay invisible,
//! because readers resolve versions through `snapshots` and filter by `snapshot_id`.

use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use cpg_schema::derived::{Derived, derivations};
use cpg_schema::id::{Digest, Id, IdHasher, kind};
use cpg_schema::rules::rules;
use cpg_schema::table::{Table, schema_digest};
use cpg_schema::tables::{Runs, Snapshots, SnapshotsRow, contracts};

use crate::CoreError;
use crate::delta::{append, open_or_create};
use crate::derive::derive;
use crate::snapshot::{Versions, register, resolve, session};
use crate::validate::validate;

/// A published attempt.
#[derive(Debug, Clone)]
pub struct Published {
    pub snapshot_id: Id,
    pub content_digest: Digest,
    pub versions: Versions,
    /// Rows the attempt wrote, per table (raw, then derived).
    pub rows: Vec<(&'static str, i64)>,
}

/// Bumped by hand whenever the derive, cast or sort code changes output for the same inputs; the
/// derived-table snapshots are what show such a change.
pub const COMPILER_OUTPUT_VERSION: u32 = 1;

/// The locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs, its kernel), read from
/// `Cargo.lock` at build time (`build.rs`).
pub const ENGINES: &str = env!("LCTX_ENGINES");

/// The compiler's identity from its parts (review F4).
pub fn compiler_digest_of(
    engines: &str,
    output_version: u32,
    derivations: &[(&str, String)],
    contracts: &[(&str, String)],
    rules: &[(String, String)],
) -> Digest {
    let mut h = IdHasher::new(kind::COMPILER);
    h.str(engines).i64(i64::from(output_version));
    for (name, text) in derivations.iter().chain(contracts) {
        h.str(name).str(text);
    }
    for (name, sql) in rules {
        h.str(name).str(sql);
    }
    h.finish_digest()
}

/// The identity of the code that derives, validates and publishes: the locked engines, the output
/// version, every derivation query, every table contract and every validation rule. Stored on each
/// `snapshots` row and folded into `content_digest`.
pub fn compiler_digest() -> Digest {
    let rules: Vec<(String, String)> = rules().into_iter().map(|r| (r.name, r.sql)).collect();
    compiler_digest_of(
        ENGINES,
        COMPILER_OUTPUT_VERSION,
        &derivations(),
        &contracts(),
        &rules,
    )
}

fn ids(batch: &RecordBatch, column: &'static str) -> Result<Vec<Id>, CoreError> {
    let array = batch
        .column(batch.schema().index_of(column)?)
        .as_any()
        .downcast_ref::<FixedSizeBinaryArray>()
        .ok_or(CoreError::ColumnType(column))?;
    (0..array.len())
        .map(|i| {
            <[u8; 16]>::try_from(array.value(i))
                .map(Id)
                .map_err(|_| CoreError::ColumnType(column))
        })
        .collect()
}

/// §3.4.1 `content_digest` for what exists so far: the sorted run ids (each carrying its release,
/// and its context's lock and environment digests, ADR-0013) and the compiler digest. The
/// analytics-config and embedding inputs join it as their stages land.
pub fn content_digest(run_ids: &[Id]) -> Digest {
    let mut runs: Vec<String> = run_ids.iter().map(Id::hex).collect();
    runs.sort();
    runs.dedup();
    IdHasher::new(kind::SNAPSHOT_CONTENT)
        .strs(runs.iter().map(String::as_str))
        .digest_field(compiler_digest())
        .finish_digest()
}

async fn write<T: Table>(
    root: &Path,
    batch: &RecordBatch,
    snapshot_id: Id,
) -> Result<u64, CoreError> {
    if batch.schema() != T::schema() {
        return Err(CoreError::SchemaMismatch(T::NAME));
    }
    if ids(batch, "snapshot_id")?.iter().any(|s| *s != snapshot_id) {
        return Err(CoreError::ForeignSnapshot(T::NAME));
    }
    let table = append(open_or_create::<T>(root).await?, batch.clone(), snapshot_id).await?;
    table.version().ok_or(CoreError::NoVersion(T::NAME))
}

/// Write each raw table's batch. Every raw table must be present exactly once.
async fn write_raw(
    root: &Path,
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
    versions: &mut Versions,
    rows: &mut Vec<(&'static str, i64)>,
) -> Result<(), CoreError> {
    for (name, _) in raw {
        macro_rules! known {
            ($($t:ty),+) => { [$(<$t as Table>::NAME),+].contains(name) };
        }
        if !cpg_schema::for_each_table!(known) {
            return Err(CoreError::UnknownTable((*name).to_owned()));
        }
    }
    macro_rules! each {
        ($($t:ty),+) => {$({
            let name = <$t as Table>::NAME;
            let mut given = raw.iter().filter(|(n, _)| *n == name);
            let (Some((_, batch)), None) = (given.next(), given.next()) else {
                return Err(CoreError::MissingTable(name));
            };
            let version = write::<$t>(root, batch, snapshot_id).await?;
            versions.insert(name.to_owned(), version);
            rows.push((name, batch.num_rows() as i64));
        })+};
    }
    cpg_schema::for_each_table!(each);
    Ok(())
}

async fn write_derived<T: Derived>(
    ctx: &datafusion::prelude::SessionContext,
    root: &Path,
    snapshot_id: Id,
    versions: &mut Versions,
    rows: &mut Vec<(&'static str, i64)>,
) -> Result<(), CoreError> {
    let batch = derive::<T>(ctx, snapshot_id).await?;
    let version = write::<T>(root, &batch, snapshot_id).await?;
    register(ctx, root, T::NAME, version, snapshot_id).await?;
    versions.insert(T::NAME.to_owned(), version);
    rows.push((T::NAME, batch.num_rows() as i64));
    Ok(())
}

fn schema_digest_of(name: &str) -> Digest {
    macro_rules! find {
        ($($t:ty),+) => {$(
            if name == <$t as Table>::NAME {
                return schema_digest(&<$t as Table>::schema());
            }
        )+};
    }
    cpg_schema::for_each_table!(find);
    cpg_schema::for_each_derived_table!(find);
    schema_digest(&Snapshots::schema())
}

/// Run one attempt over the extractor's raw batches. On success the snapshot is published; a
/// validation failure publishes nothing (`CoreError::Invalid`).
pub async fn compile(
    root: &Path,
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
) -> Result<Published, CoreError> {
    let mut versions = Versions::new();
    let mut rows = Vec::new();
    write_raw(root, snapshot_id, raw, &mut versions, &mut rows).await?;

    let ctx = session(root, snapshot_id, &versions).await?;
    macro_rules! derive_all {
        ($($t:ty),+) => {$(
            write_derived::<$t>(&ctx, root, snapshot_id, &mut versions, &mut rows).await?;
        )+};
    }
    cpg_schema::for_each_derived_table!(derive_all);

    let violations = validate(&ctx).await?;
    if !violations.is_empty() {
        return Err(CoreError::Invalid(violations));
    }

    let runs = raw
        .iter()
        .find(|(n, _)| *n == Runs::NAME)
        .map(|(_, b)| ids(b, "run_id"))
        .transpose()?
        .unwrap_or_default();
    let digest = content_digest(&runs);
    let snapshot_rows: Vec<SnapshotsRow> = rows
        .iter()
        .map(|(name, count)| SnapshotsRow {
            snapshot_id,
            content_digest: digest,
            table_name: (*name).to_owned(),
            table_version: versions[*name] as i64,
            schema_digest: schema_digest_of(name),
            compiler_digest: compiler_digest(),
            row_count: *count,
        })
        .collect();
    publish(root, snapshot_id, &snapshot_rows).await?;
    Ok(Published {
        snapshot_id,
        content_digest: digest,
        versions,
        rows,
    })
}

/// The publication act: one `snapshots` append. An error on it is ambiguous (`delta.commit.1`),
/// so the attempt is classified by re-reading `snapshots` before anything else happens. A snapshot
/// is published at most once.
pub async fn publish(root: &Path, snapshot_id: Id, rows: &[SnapshotsRow]) -> Result<(), CoreError> {
    if resolve(root, snapshot_id).await?.is_some() {
        return Err(CoreError::AlreadyPublished(snapshot_id.hex()));
    }
    let batch = Snapshots::to_sorted_batch(rows)?;
    let attempt =
        async { append(open_or_create::<Snapshots>(root).await?, batch, snapshot_id).await };
    match attempt.await {
        Ok(_) => Ok(()),
        Err(error) => match resolve(root, snapshot_id).await? {
            Some(_) => Ok(()),
            None => Err(CoreError::Unpublished(Box::new(error))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiler_digest_follows_each_input() {
        let d = vec![("t", "SELECT 1".to_owned())];
        let c = vec![("t", "contract".to_owned())];
        let r = vec![("key:t".to_owned(), "SELECT 2".to_owned())];
        let base = compiler_digest_of("engines", 1, &d, &c, &r);
        assert_ne!(base, compiler_digest_of("engines'", 1, &d, &c, &r));
        assert_ne!(base, compiler_digest_of("engines", 2, &d, &c, &r));
        let d2 = vec![("t", "SELECT 1 ".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, &d2, &c, &r));
        let r2 = vec![("key:t".to_owned(), "SELECT 3".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, &d, &c, &r2));
        assert!(ENGINES.contains("datafusion 55.1.0"), "{ENGINES}");
        assert!(ENGINES.contains("deltalake-core 1.0.0 git+"), "{ENGINES}");
    }
}
