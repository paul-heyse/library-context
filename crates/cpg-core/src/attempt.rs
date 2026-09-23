//! One compile attempt (DESIGN §6.1, ADR-0009): write every raw table, derive (Stage C/D),
//! validate (§8), and only then publish with one `snapshots` append. Any error aborts the attempt;
//! a retry is a new attempt with a new `snapshot_id`. Rows of an unpublished attempt stay invisible,
//! because readers resolve versions through `snapshots` and filter by `snapshot_id`.

use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use cpg_schema::derived::{Derived, derivations};
use cpg_schema::id::{Digest, Id, IdHasher, kind};
use cpg_schema::metrics::{Stage, Stages};
use cpg_schema::rules::rules;
use cpg_schema::table::{Table, schema_digest};
use cpg_schema::tables::{Producers, Runs, Snapshots, SnapshotsRow, contracts};

use crate::CoreError;
use crate::analyze::{Analysis, AnalysisRows, CompilerRun, compiler_rows};
use crate::delta::{append, open_or_create};
use crate::derive::derive;
use crate::snapshot::{Versions, register, resolve, session};
use crate::validate::validate_costed;

/// A published attempt.
#[derive(Debug, Clone)]
pub struct Published {
    pub snapshot_id: Id,
    pub content_digest: Digest,
    pub versions: Versions,
    /// Rows the attempt wrote, per table (raw, then derived).
    pub rows: Vec<(&'static str, i64)>,
    /// Wall time and peak RSS per stage: each raw write, each derivation (with its write),
    /// validation, publication (DESIGN §4.3). Not content.
    pub stages: Vec<Stage>,
}

/// Bumped by hand whenever the derive, cast, sort or analysis code changes output for the same
/// inputs; the derived-table and analysis snapshots are what show such a change. 2: Stage E
/// (ADR-0019).
pub const COMPILER_OUTPUT_VERSION: u32 = 2;

/// The locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs, its kernel), read from
/// `Cargo.lock` at build time (`build.rs`).
pub const ENGINES: &str = env!("LCTX_ENGINES");

/// The compiler's identity from its parts (review F4).
pub fn compiler_digest_of(
    engines: &str,
    output_version: u32,
    udf_version: u32,
    derivations: &[(&str, String)],
    contracts: &[(&str, String)],
    rules: &[(String, String)],
) -> Digest {
    let mut h = IdHasher::new(kind::COMPILER);
    h.str(engines)
        .i64(i64::from(output_version))
        .i64(i64::from(udf_version));
    for (name, text) in derivations.iter().chain(contracts) {
        h.str(name).str(text);
    }
    for (name, sql) in rules {
        h.str(name).str(sql);
    }
    h.finish_digest()
}

/// The identity of the code that derives, analyzes, validates and publishes: the locked engines
/// and analysis libraries, the output version, every derivation query and declared projection,
/// every table contract and every validation rule. Stored on each `snapshots` row and folded into
/// `content_digest`.
pub fn compiler_digest() -> Digest {
    let rules: Vec<(String, String)> = rules().into_iter().map(|r| (r.name, r.sql)).collect();
    let mut queries = derivations();
    for spec in cpg_schema::projection::projections() {
        queries.push((spec.name, spec.digest().hex()));
    }
    compiler_digest_of(
        &format!("{ENGINES}; {}", lctx_analytics::LIBRARIES),
        COMPILER_OUTPUT_VERSION,
        crate::udf::VERSION,
        &queries,
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
    stages: &mut Stages,
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
            stages.mark(format!("write {name}"));
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
    stages: &mut Stages,
) -> Result<(), CoreError> {
    let batch = derive::<T>(ctx, snapshot_id).await?;
    let version = write::<T>(root, &batch, snapshot_id).await?;
    register(ctx, root, T::NAME, version, snapshot_id).await?;
    versions.insert(T::NAME.to_owned(), version);
    rows.push((T::NAME, batch.num_rows() as i64));
    stages.mark(format!("derive {}", T::NAME));
    Ok(())
}

/// A stored table's schema digest. An unknown name is an error, never another table's digest
/// (ADR-0019).
fn schema_digest_of(name: &str) -> Result<Digest, CoreError> {
    macro_rules! find {
        ($($t:ty),+) => {$(
            if name == <$t as Table>::NAME {
                return Ok(schema_digest(&<$t as Table>::schema()));
            }
        )+};
    }
    cpg_schema::for_each_table!(find);
    cpg_schema::for_each_derived_table!(find);
    cpg_schema::for_each_analysis_table!(find);
    Err(CoreError::UnknownTable(name.to_owned()))
}

/// Add the `lctx-compiler` run and producer to the raw `runs` and `producers` batches: the run is
/// over the release and context of the extractor run that declares `exports` (ADR-0019).
fn with_compiler_run(
    raw: &mut [(&str, RecordBatch)],
    snapshot_id: Id,
    analysis: &Analysis,
) -> Result<CompilerRun, CoreError> {
    let runs = raw
        .iter()
        .find(|(n, _)| *n == Runs::NAME)
        .map(|(_, b)| b.clone())
        .ok_or(CoreError::MissingTable(Runs::NAME))?;
    let families = runs
        .column(runs.schema().index_of("families")?)
        .as_any()
        .downcast_ref::<arrow_array::ListArray>()
        .ok_or(CoreError::ColumnType("families"))?
        .clone();
    let declares_exports: Vec<usize> = (0..runs.num_rows())
        .filter(|&i| {
            let list = families.value(i);
            let names = list
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .map(|a| (0..a.len()).any(|j| a.value(j) == "exports"));
            names.unwrap_or(false)
        })
        .collect();
    let [row] = declares_exports[..] else {
        return Err(CoreError::Analysis(format!(
            "{} runs declare exports; the compiler run needs exactly one library run",
            declares_exports.len()
        )));
    };
    let release = ids(&runs, "release_id")?[row];
    let context = ids(&runs, "context_id")?[row];
    let (compiler, run, producer) =
        compiler_rows(snapshot_id, release, context, analysis.config.digest());
    for (name, batch) in raw.iter_mut() {
        let extra = match *name {
            n if n == Runs::NAME => Runs::to_batch(std::slice::from_ref(&run))?,
            n if n == Producers::NAME => Producers::to_batch(std::slice::from_ref(&producer))?,
            _ => continue,
        };
        let joined = arrow_select::concat::concat_batches(&batch.schema(), [&*batch, &extra])?;
        let key = if *name == Runs::NAME {
            Runs::key()
        } else {
            Producers::key()
        };
        *batch = cpg_schema::table::canonical_sort(&joined, key)?;
    }
    Ok(compiler)
}

/// Write one analysis table's rows and register it in the session.
async fn write_analysis<T: Table>(
    ctx: &datafusion::prelude::SessionContext,
    root: &Path,
    snapshot_id: Id,
    rows_of: &[T::Row],
    w: &mut Written,
) -> Result<(), CoreError> {
    let batch = T::to_sorted_batch(rows_of)?;
    let version = write::<T>(root, &batch, snapshot_id).await?;
    register(ctx, root, T::NAME, version, snapshot_id).await?;
    w.versions.insert(T::NAME.to_owned(), version);
    w.rows.push((T::NAME, batch.num_rows() as i64));
    w.stages.mark(format!("write {}", T::NAME));
    Ok(())
}

/// Run one attempt over the extractor's raw batches. On success the snapshot is published; a
/// validation failure publishes nothing (`CoreError::Invalid`).
pub async fn compile(
    root: &Path,
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
) -> Result<Published, CoreError> {
    compile_analyzed(root, snapshot_id, raw, None).await
}

/// [`compile`] with Stage E and F over the given analytics config (ADR-0019). Without one, the
/// analysis tables are written empty and no compiler run is recorded.
pub async fn compile_analyzed(
    root: &Path,
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
    analysis: Option<&Analysis>,
) -> Result<Published, CoreError> {
    let mut raw = raw.to_vec();
    let compiler = analysis
        .map(|a| with_compiler_run(&mut raw, snapshot_id, a))
        .transpose()?;
    let mut written = Written::new();
    let runs = write_all(root, snapshot_id, &raw, &mut written).await?;
    finish(root, snapshot_id, runs, written, analysis.zip(compiler)).await
}

/// [`compile_analyzed`], releasing the raw batches once they are written: derivation and
/// validation read the Delta tables, and only the run ids are still needed (C6 review F3).
pub async fn compile_owned(
    root: &Path,
    snapshot_id: Id,
    mut raw: Vec<(&'static str, RecordBatch)>,
    analysis: Option<&Analysis>,
) -> Result<Published, CoreError> {
    let compiler = analysis
        .map(|a| with_compiler_run(&mut raw, snapshot_id, a))
        .transpose()?;
    let mut written = Written::new();
    let runs = write_all(root, snapshot_id, &raw, &mut written).await?;
    drop(raw);
    written.stages.mark("release raw batches");
    finish(root, snapshot_id, runs, written, analysis.zip(compiler)).await
}

/// What the raw writes leave for the rest of the attempt.
struct Written {
    stages: Stages,
    versions: Versions,
    rows: Vec<(&'static str, i64)>,
}

impl Written {
    fn new() -> Self {
        Self {
            stages: Stages::new(),
            versions: Versions::new(),
            rows: Vec::new(),
        }
    }
}

/// Write every raw table, and return the attempt's run ids (its `content_digest` input).
async fn write_all(
    root: &Path,
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
    w: &mut Written,
) -> Result<Vec<Id>, CoreError> {
    write_raw(
        root,
        snapshot_id,
        raw,
        &mut w.versions,
        &mut w.rows,
        &mut w.stages,
    )
    .await?;
    Ok(raw
        .iter()
        .find(|(n, _)| *n == Runs::NAME)
        .map(|(_, b)| ids(b, "run_id"))
        .transpose()?
        .unwrap_or_default())
}

/// Derive, analyze, validate and publish over the written raw tables.
async fn finish(
    root: &Path,
    snapshot_id: Id,
    runs: Vec<Id>,
    mut written: Written,
    analysis: Option<(&Analysis, CompilerRun)>,
) -> Result<Published, CoreError> {
    let ctx = session(root, snapshot_id, &written.versions).await?;
    written.stages.mark("open session");
    {
        let Written {
            stages,
            versions,
            rows,
        } = &mut written;
        macro_rules! derive_all {
            ($($t:ty),+) => {$(
                write_derived::<$t>(&ctx, root, snapshot_id, versions, rows, stages).await?;
            )+};
        }
        cpg_schema::for_each_derived_table!(derive_all);
    }

    // Stage E (ADR-0019): the analyses read the session, and their rows are written like any
    // other table's, one commit each.
    let found = match analysis {
        Some((a, compiler)) => crate::analyze::run(&ctx, snapshot_id, a, compiler).await?,
        None => AnalysisRows::default(),
    };
    written.stages.mark("analyze (Pass A)");
    use cpg_schema::findings::{AnalysisInvocations, FindingMembers, Findings, Witnesses};
    write_analysis::<AnalysisInvocations>(
        &ctx,
        root,
        snapshot_id,
        &found.invocations,
        &mut written,
    )
    .await?;
    write_analysis::<Findings>(&ctx, root, snapshot_id, &found.findings, &mut written).await?;
    write_analysis::<FindingMembers>(&ctx, root, snapshot_id, &found.members, &mut written).await?;
    write_analysis::<Witnesses>(&ctx, root, snapshot_id, &found.witnesses, &mut written).await?;

    // Stage F (DESIGN §10): assertions and briefs from the findings, written the same way.
    let made = match analysis {
        Some((_, compiler)) => crate::synth::run(&ctx, snapshot_id, compiler, &found).await?,
        None => crate::synth::SynthRows {
            policy: crate::synth::policy_rows(snapshot_id),
            ..Default::default()
        },
    };
    written.stages.mark("synthesize");
    use cpg_schema::findings::{
        AssertionPolicy, AssertionSupport, Assertions, BriefAssertions, BriefDocuments,
        BriefMembers, Briefs, Evidence,
    };
    write_analysis::<Evidence>(&ctx, root, snapshot_id, &made.evidence, &mut written).await?;
    write_analysis::<Assertions>(&ctx, root, snapshot_id, &made.assertions, &mut written).await?;
    write_analysis::<AssertionSupport>(&ctx, root, snapshot_id, &made.supports, &mut written)
        .await?;
    write_analysis::<Briefs>(&ctx, root, snapshot_id, &made.briefs, &mut written).await?;
    write_analysis::<BriefAssertions>(
        &ctx,
        root,
        snapshot_id,
        &made.brief_assertions,
        &mut written,
    )
    .await?;
    write_analysis::<BriefMembers>(&ctx, root, snapshot_id, &made.brief_members, &mut written)
        .await?;
    write_analysis::<BriefDocuments>(&ctx, root, snapshot_id, &made.brief_documents, &mut written)
        .await?;
    write_analysis::<AssertionPolicy>(&ctx, root, snapshot_id, &made.policy, &mut written).await?;
    let Written {
        mut stages,
        versions,
        rows,
    } = written;

    let (violations, costs) = validate_costed(&ctx).await?;
    stages.mark("validate");
    // What dominates validation (C6, §4.3; H1 P5, from each rule's own plan metrics): the
    // slowest rules by operator compute, and the largest hash-join build.
    let mut slowest = costs.clone();
    slowest.sort_by(|a, b| b.compute_seconds.total_cmp(&a.compute_seconds));
    for c in slowest.iter().take(3) {
        stages.push(
            format!("validate:   slowest {}", c.rule),
            std::time::Duration::from_secs_f64(c.compute_seconds),
        );
    }
    if let Some(c) = costs.iter().max_by_key(|c| c.build_bytes) {
        stages.push(
            format!(
                "validate:   largest hash build ({} MiB) {}",
                c.build_bytes >> 20,
                c.rule
            ),
            std::time::Duration::from_secs_f64(c.compute_seconds),
        );
    }
    if !violations.is_empty() {
        return Err(CoreError::Invalid(violations));
    }

    let digest = content_digest(&runs);
    let snapshot_rows: Vec<SnapshotsRow> = rows
        .iter()
        .map(|(name, count)| {
            Ok(SnapshotsRow {
                snapshot_id,
                content_digest: digest,
                table_name: (*name).to_owned(),
                table_version: versions[*name] as i64,
                schema_digest: schema_digest_of(name)?,
                compiler_digest: compiler_digest(),
                row_count: *count,
            })
        })
        .collect::<Result<_, CoreError>>()?;
    publish(root, snapshot_id, &snapshot_rows).await?;
    stages.mark("publish");
    Ok(Published {
        snapshot_id,
        content_digest: digest,
        versions,
        rows,
        stages: stages.stages,
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
        let base = compiler_digest_of("engines", 1, 1, &d, &c, &r);
        assert_ne!(base, compiler_digest_of("engines'", 1, 1, &d, &c, &r));
        assert_ne!(base, compiler_digest_of("engines", 2, 1, &d, &c, &r));
        assert_ne!(base, compiler_digest_of("engines", 1, 2, &d, &c, &r));
        let d2 = vec![("t", "SELECT 1 ".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, 1, &d2, &c, &r));
        let r2 = vec![("key:t".to_owned(), "SELECT 3".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, 1, &d, &c, &r2));
        assert!(ENGINES.contains("datafusion 55.1.0"), "{ENGINES}");
        assert!(ENGINES.contains("deltalake-core 1.0.0 git+"), "{ENGINES}");
    }
}
