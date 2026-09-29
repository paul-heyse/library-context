//! One compile attempt (DESIGN §6.1, ADR-0009): write every raw table, derive (Stage C/D),
//! validate (§8), and only then publish with one `snapshots` append. Any error aborts the attempt;
//! a retry is a new attempt with a new `snapshot_id`. Rows of an unpublished attempt stay invisible,
//! because readers resolve versions through `snapshots` and filter by `snapshot_id`.

use std::path::Path;

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use cpg_schema::derived::{Derived, derivations};
use cpg_schema::id::{Digest, Id, IdHasher, kind};
use cpg_schema::metrics::{Stage, Stages};
use cpg_schema::query::QueryRow;
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
    /// Actual pure-stage execution decisions, never inferred from the command.
    pub catalog_stages: Vec<crate::rebuild::Step>,
}

/// Bumped by hand whenever the derive, cast, sort or analysis code changes output for the same
/// inputs; the derived-table and analysis snapshots are what show such a change. 2: Stage E
/// (ADR-0019). 3: definition arcs, the witness-cap kind fix (slice 1.4 review F1, F3). 4: the
/// documents' spec hash and `embedding_specs` (slice 1.7). 5: Pass B and parameter docs (slice
/// 2.1). 6: Pass C (slice 2.2). 7: the slice 2.1 review: handler, conditional and tested calls,
/// unfollowed arguments, supported predicates, receivers by kind, header-anchored descriptions.
/// 8: communities and their invocations' diagnostics (slice 2.3). 9: centrality (slice 2.4).
/// 10: the slice 2.2 review: release handoff endpoints, one-target producers, the narrowed
/// receiver exclusion, doc blocks by document, self-contained usage patterns. 11: the ADR-0011
/// review: γ = 1 fixed, the public co-assignment score, named layer policies. 12: FCA (slice 2.5).
/// 13: seed selection within the brief budget (slice 2.6). 14: E0 and kNN (slice 3.1). 15: the
/// increment-2 review: direct usage and selection by it, one preferred path per callable, FCA
/// scopes keyed by node, attributes from term structure (no `Unknown`, one raised class). 16: the
/// selection invocation records the whole technique set (the ADR-0020 review's F3).
/// 17: direct entry-formal/test-use links (Stage 3.0). 18: resolved builtin `type(x) is C`
/// atoms, their guarded entry-value links and exact-class origins. 19: path-stable later-use
/// links under an exact type guard. 20: committed typed model catalog identity and validation.
/// 21: pinned-source model target bindings and their publication contract. 22: compiled authored
/// transfer rows, gated on those target bindings. 23: external Pysa signatures and model formal
/// path validation. 24: attributed explicit exits and finally-body actions. 25: attributed
/// except clauses and direct handler actions. 26: authored typed effects and their pinned binding.
/// 27: typed callback, resource and exception model assertions. 28: cited handler type status.
/// 29: unresolved try/with frames withhold definite raise escape (ADR-0027).
/// 30: cited model applications at source call sites. 31: typed model formal paths.
/// 32: exact per-signature model formal to source-argument bindings.
/// 33: candidate-local modeled callback sites and binding boundaries.
/// 34: candidate-local modeled resource sites. 35: candidate-local modeled transfer sites.
/// 36: candidate-local modeled effect sites. 37: exact flow-call source links (ADR-0028).
/// 38: pinned exception class identities and candidate-local modeled exception sites.
/// 41: preserve unaggregated provider-value contributions for exact call-path composition.
/// 42: decide clause order within a modeled exception's candidate try frame.
/// 43: name the sink callable separately from a captured source parameter's owner.
/// 44: preserve the upstream transfer before each raw value fact's local call path.
/// 45: direct, pre-finally return-None witnesses for handler bodies.
/// 46: candidate modeled raises compose through a direct first handler to a return-None path.
/// 47: direct one-call return candidates join raw parameter flows to pinned model transfers.
/// 48: raw predecessor candidates retain reaching-definition identity and separate conditions.
/// 49: persist the BDD closures of recomposed flow-analysis conditions.
/// 50: a direct modeled return requires the call result to be the entire sink expression.
/// 51: bounded BDD compatibility of cited predecessor-path conditions.
/// 52: exact model-call steps apply to whole definition values as well as whole returns.
/// 53: cite a condition-checked identity return from a whole-assignment modeled value.
/// 54: finite direct identity-return summary seeds with structural condition decisions.
/// 55: explicit incomplete parameter-to-return summary boundaries.
/// 56: pinned model targets carry an explicit total-normal-return assertion.
/// 57: model applications retain actual target count and selected target completion assertion.
/// 58: finite summary paths gain canonical ids and ordered, typed proof steps.
/// 59: each modeled exact-value candidate records ordered argument evaluation evidence.
/// 60: exact unshadowed builtin names gain local normal-evaluation evidence.
/// 61: first finite modeled identity-return paths carry complete ordered call proofs.
/// 62: unique assignment-to-return predecessors gain finite model-call summaries.
/// 63: attributed source call SCCs have canonical callee-first component rows.
/// 64: acyclic exact local wrappers compose unconditional callee value summaries.
/// 65: bounded return-frame statuses gate nested finite normal-return summary seeds.
/// 66: one cited literal-finally-pass frame admits a pending normal return.
/// 67: an admitted summary cannot erase a sibling source contribution's open boundary.
/// 68: pinned Pydantic TypeAdapter validation candidate model and fixture coverage.
/// 69: finite returns through a sole pass finalizer cite the pass in ordered summary steps.
/// 70: nested pass-only finalizers cite every source action in inner-to-outer proof order.
/// 71: direct and modeled finite flows in recursive SCCs remain explicit unknown paths.
/// 72: direct paths screen earlier source calls, admitting a recursive base before its self-call.
/// 73: incompatible earlier call regions no longer block a finite direct return path.
/// 74: raw value-flow contribution keys retain local and upstream transfer provenance;
/// multi-release validation accepts one snapshot with multiple releases.
/// 76: normalized local-call arguments, multi-control Boolean specialization and bounded
/// closed-expression completion with separately represented exact Boolean values.
/// 77: shared source-admitted expression evaluation and ordered operand proofs.
/// 78: condition-keyed ordered return-entry completion proofs.
/// 79: typed handler/re-raise completion and native context-MRO completeness.
/// 80: exact completion exceptions and origin/channel coverage.
/// 81: default-availability admission and semantic/witness-separated value fixed point.
/// 82: definition-time header completion, typed site/origin coverage and shared callee proof admission.
/// 83: independently authored model phases and catalog format 2.
/// 84: source binding and call-specific fresh default availability/value/stability evidence.
/// 85: typed static/runtime/unresolved validation schema contracts and catalog format 3.
/// 86: condition-safe transfer alternatives and typed behavior transfer/scope in FORMAT 9.
/// 87: occurrence-specific source parameter identity independent of provider reach approximation.
/// 88: typed current FCA/RCA attributes, incidence evidence and presentation-only labels.
/// 89: independently bound synchronous class protocols and mandatory return obligations (catalog format 4).
/// 90: separate context entry-value identity and mandatory base value witnesses.
/// 91: named-handler binding/implicit-deletion boundary retained through serving.
/// 92: invocation prefixes and source call execution independent of callee completion.
/// 93: independent lexical input/model identity for direct modeled returns.
/// 94: explicit authored action triggers and candidate-backed action assessments.
/// 95: pinned default availability and independently committed omitted-formal obligations.
/// 96: normal-exit action implications with mandatory undischarged outcome obligations.
/// 97–100: frame/body/fresh-call completion and independent source invocation.
/// 101: nondominated depth/proof-cost progress through shared bounded scheduling helpers.
/// 102: agree on repeated pinned context observations without counting them as overloads.
/// 103: builtin binding-preserving descriptors no longer write the decorator boundary.
/// 104: summaries neither start in nor compose through a decorated function (ADR-0064).
/// 105: call-transfer return claims graded after summaries with claim-keyed discharges (ADR-0064).
/// 112: canonical semantic producer identity and validated coarse-stage reuse (ADR-0081).
pub const COMPILER_OUTPUT_VERSION: u32 = 112;

/// The locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs, its kernel), read from
/// `Cargo.lock` at build time (`build.rs`).
pub const ENGINES: &str = env!("LCTX_ENGINES");

/// The compiler's identity from its parts (review F4).
pub fn compiler_digest_of(
    engines: &str,
    output_version: u32,
    udf_version: u32,
    template_version: i64,
    derivations: &[(&str, String)],
    contracts: &[(&str, String)],
    rules: &[(String, String)],
) -> Digest {
    let mut h = IdHasher::new(kind::COMPILER);
    h.str(engines)
        .i64(i64::from(output_version))
        .i64(i64::from(udf_version))
        .i64(template_version);
    for (name, text) in derivations.iter().chain(contracts) {
        h.str(name).str(text);
    }
    for (name, sql) in rules {
        h.str(name).str(sql);
    }
    h.finish_digest()
}

/// The identity of the code that derives, analyzes, validates and publishes: the locked engines
/// and analysis libraries, the output version, the synthesis template version (which stands for
/// Stage F's queries and templates: the analysis ledger fails any output change without a bump;
/// increment-1 deep review F1), every derivation query, declared projection, Pass B and community
/// relation, the pre-registered community parameters (slice 2.3),
/// every table contract and every validation rule. Stored on each `snapshots` row and folded into
/// `content_digest`.
/// The digest of the compiler's sources (`build.rs`): every `.rs` file of `cpg-core`,
/// `lctx-analytics` and `cpg-schema`.
pub const SOURCE_DIGEST: &str = env!("LCTX_SOURCE_DIGEST");

/// Identity of canonical semantic production; serving-only code is still full compiler provenance.
pub fn semantic_digest() -> Digest {
    compiler_digest_for(env!("LCTX_SEMANTIC_SOURCE_DIGEST"))
}

/// Producer identity (DESIGN §15.3; review F11): every canonical producer's code. Recorded as
/// the compiler producer's `build_digest`, so a canonical row names the code that produced it.
pub fn producer_digest() -> Digest {
    compiler_digest_for(env!("LCTX_PRODUCER_SOURCE_DIGEST"))
}

/// The files producer identity leaves out: serving-only realization code.
pub const PRODUCER_EXCLUDED: &str = env!("LCTX_PRODUCER_EXCLUDED");

pub fn compiler_digest() -> Digest {
    compiler_digest_for(SOURCE_DIGEST)
}

fn compiler_digest_for(source: &str) -> Digest {
    let rules: Vec<(String, String)> = rules().into_iter().map(|r| (r.name, r.sql)).collect();
    let mut queries = derivations();
    for spec in cpg_schema::projection::projections() {
        queries.push((spec.name, spec.digest().hex()));
    }
    queries.push(("pass_b_relations", cpg_schema::flows::digest().hex()));
    // The compiler's own sources (the holistic assessment's A2(e); `build.rs`).
    queries.push(("compiler_sources", source.to_owned()));
    // The one public-path authority (the holistic assessment's A1): seeds resolve by it.
    for relation in cpg_schema::public::all()
        .into_iter()
        .chain(cpg_schema::public::candidate_relations())
        .chain(cpg_schema::public::member_relations())
    {
        queries.push((relation.name, relation.sql));
    }
    queries.push((
        "community_relations",
        cpg_schema::communities::digest().hex(),
    ));
    queries.push((
        "community_parameters",
        lctx_analytics::communities::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "usage_projection",
        lctx_analytics::ranking::projection_digest(cpg_schema::projection::invocation().digest())
            .hex(),
    ));
    queries.push(("concept_relations", cpg_schema::concepts::digest().hex()));
    queries.push((
        "behavior_models_catalog",
        cpg_schema::models::Catalog::committed_digest().hex(),
    ));
    queries.push((
        "neighbour_relations",
        cpg_schema::neighbours::digest().hex(),
    ));
    queries.push((
        "knn_parameters",
        lctx_analytics::neighbours::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "fca_parameters",
        lctx_analytics::concepts::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "pagerank_parameters",
        lctx_analytics::ranking::Params::preregistered()
            .digest()
            .hex(),
    ));
    compiler_digest_of(
        &format!("{ENGINES}; {}", lctx_analytics::LIBRARIES),
        COMPILER_OUTPUT_VERSION,
        crate::udf::VERSION,
        crate::synth::TEMPLATE_VERSION,
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
    content_digest_with(run_ids, None)
}

/// [`content_digest`] with the embedding inputs (§3.4.1; ADR-0017 amendment): the spec hash and a
/// digest of exact sorted key/value receipts the snapshot consumed, never mutable cache state.
pub fn content_digest_with(run_ids: &[Id], embedded: Option<(Digest, Digest)>) -> Digest {
    let mut runs: Vec<String> = run_ids.iter().map(Id::hex).collect();
    runs.sort();
    runs.dedup();
    let mut h = IdHasher::new(kind::SNAPSHOT_CONTENT);
    h.strs(runs.iter().map(String::as_str))
        .digest_field(compiler_digest());
    if let Some((spec, keys)) = embedded {
        h.digest_field(spec).digest_field(keys);
    }
    h.finish_digest()
}

pub(crate) async fn write<T: Table>(
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
pub(crate) fn schema_digest_of(name: &str) -> Result<Digest, CoreError> {
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
    config_digest: Digest,
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
    // The whole technique set joins the config digest (§9.8; the ADR-0020 review's F3).
    let (compiler, run, producer) = compiler_rows(snapshot_id, release, context, config_digest);
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
    write_analysis_batch::<T>(ctx, root, snapshot_id, batch, w).await
}

/// SQL-only derivations remain columnar. Strict conversion and canonical ordering are the
/// existing Delta/table contracts; this path removes decoding and re-encoding every row.
async fn write_analysis_query<T: Table>(
    ctx: &datafusion::prelude::SessionContext,
    root: &Path,
    snapshot_id: Id,
    relation: &cpg_schema::query::Relation,
    w: &mut Written,
) -> Result<(), CoreError> {
    let mut declared = Vec::new();
    for batch in crate::sql::query(ctx, &relation.sql)
        .await?
        .collect()
        .await?
    {
        declared.push(crate::arrow_types::to_declared::<T>(&batch)?);
    }
    let batch = arrow_select::concat::concat_batches(&T::schema(), &declared)?;
    let batch = cpg_schema::table::canonical_sort(&batch, T::key())?;
    write_analysis_batch::<T>(ctx, root, snapshot_id, batch, w).await
}

async fn write_analysis_batch<T: Table>(
    ctx: &datafusion::prelude::SessionContext,
    root: &Path,
    snapshot_id: Id,
    batch: RecordBatch,
    w: &mut Written,
) -> Result<(), CoreError> {
    let version = write::<T>(root, &batch, snapshot_id).await?;
    // Replace the empty planning relation with this attempt's committed Delta view.
    ctx.deregister_table(T::NAME)?;
    register(ctx, root, T::NAME, version, snapshot_id).await?;
    w.versions.insert(T::NAME.to_owned(), version);
    w.rows.retain(|(name, _)| *name != T::NAME);
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
    let models = bind_models(snapshot_id, raw, analysis.is_some())?;
    let mut raw = raw.to_vec();
    let compiler = analysis
        .map(|a| {
            with_compiler_run(
                &mut raw,
                snapshot_id,
                crate::catalog::CompileInputs::from_analysis(Some(a)).digest(Some(a)),
            )
        })
        .transpose()?;
    let mut written = Written::new();
    let runs = write_all(root, snapshot_id, &raw, &mut written).await?;
    finish(
        root,
        snapshot_id,
        runs,
        written,
        analysis.zip(compiler),
        &models,
        &crate::catalog::CompileInputs::from_analysis(analysis),
        false,
    )
    .await
}

/// [`compile_analyzed`], releasing the raw batches once they are written: derivation and
/// validation read the Delta tables, and only the run ids are still needed (C6 review F3).
pub async fn compile_owned(
    root: &Path,
    snapshot_id: Id,
    mut raw: Vec<(&'static str, RecordBatch)>,
    analysis: Option<&Analysis>,
) -> Result<Published, CoreError> {
    let models = bind_models(snapshot_id, &raw, analysis.is_some())?;
    let compiler = analysis
        .map(|a| {
            with_compiler_run(
                &mut raw,
                snapshot_id,
                crate::catalog::CompileInputs::from_analysis(Some(a)).digest(Some(a)),
            )
        })
        .transpose()?;
    let mut written = Written::new();
    let runs = write_all(root, snapshot_id, &raw, &mut written).await?;
    drop(raw);
    written.stages.mark("release raw batches");
    finish(
        root,
        snapshot_id,
        runs,
        written,
        analysis.zip(compiler),
        &models,
        &crate::catalog::CompileInputs::from_analysis(analysis),
        false,
    )
    .await
}

/// Product compilation: public scope/provenance are mandatory; analysis is explicit enrichment.
pub async fn compile_catalog(
    root: &Path,
    snapshot_id: Id,
    raw: Vec<(&'static str, RecordBatch)>,
    inputs: &crate::catalog::CompileInputs,
    analysis: Option<&Analysis>,
) -> Result<Published, CoreError> {
    compile_catalog_mode(root, snapshot_id, raw, inputs, analysis, false).await
}

pub(crate) async fn compile_catalog_mode(
    root: &Path,
    snapshot_id: Id,
    mut raw: Vec<(&'static str, RecordBatch)>,
    inputs: &crate::catalog::CompileInputs,
    analysis: Option<&Analysis>,
    clean: bool,
) -> Result<Published, CoreError> {
    cpg_schema::catalog::validate_roots(&inputs.public_roots).map_err(CoreError::Analysis)?;
    if inputs.profile.behavioral() != analysis.is_some()
        || analysis.is_some_and(|a| a.config.subsystem.public_roots != inputs.public_roots)
    {
        return Err(CoreError::Analysis(
            "compile profile, public scope and enrichment disagree".into(),
        ));
    }
    let models = bind_models(snapshot_id, &raw, analysis.is_some())?;
    let compiler = with_compiler_run(&mut raw, snapshot_id, inputs.digest(analysis))?;
    let mut written = Written::new();
    let runs = write_all(root, snapshot_id, &raw, &mut written).await?;
    drop(raw);
    finish(
        root,
        snapshot_id,
        runs,
        written,
        analysis.map(|a| (a, compiler)),
        &models,
        inputs,
        clean,
    )
    .await
}

#[derive(Default)]
struct BoundModels {
    contexts: Vec<cpg_schema::context_protocol::ModelContextProtocolsRow>,
    targets: Vec<cpg_schema::behavior::ModelTargetsRow>,
    rules: cpg_schema::models::CompiledRules,
}

fn bind_models(
    snapshot_id: Id,
    raw: &[(&str, RecordBatch)],
    analyzed: bool,
) -> Result<BoundModels, CoreError> {
    if !analyzed {
        return Ok(BoundModels::default());
    }
    fn rows<T: Table>(raw: &[(&str, RecordBatch)]) -> Result<Vec<T::Row>, CoreError>
    where
        T::Row: QueryRow,
    {
        let batch = raw
            .iter()
            .find(|(name, _)| *name == T::NAME)
            .ok_or_else(|| CoreError::Analysis(format!("missing raw table {}", T::NAME)))?
            .1
            .clone();
        Ok(T::Row::read_batch(&batch)?)
    }
    let catalog = cpg_schema::models::Catalog::committed().map_err(CoreError::Analysis)?;
    let definitions = rows::<cpg_schema::tables::ContextDefinitions>(raw)?;
    let contexts = rows::<cpg_schema::tables::Contexts>(raw)?;
    let modules = rows::<cpg_schema::tables::ContextModules>(raw)?;
    let targets = catalog
        .bind_targets(snapshot_id, &contexts, &modules, &definitions)
        .map_err(CoreError::Analysis)?;
    let parameters = rows::<cpg_schema::tables::ContextParameters>(raw)?;
    let rules = catalog
        .compile_rules(&targets, &definitions, &parameters)
        .map_err(CoreError::Analysis)?;
    let protocols = catalog
        .bind_context_protocols(snapshot_id, &contexts, &modules, &definitions, &parameters)
        .map_err(CoreError::Analysis)?;
    Ok(BoundModels {
        targets,
        rules,
        contexts: protocols,
    })
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
#[allow(
    clippy::too_many_arguments,
    reason = "one publication boundary with explicit inputs and clean-rebuild policy"
)]
async fn finish(
    root: &Path,
    snapshot_id: Id,
    runs: Vec<Id>,
    mut written: Written,
    analysis: Option<(&Analysis, CompilerRun)>,
    models: &BoundModels,
    inputs: &crate::catalog::CompileInputs,
    clean: bool,
) -> Result<Published, CoreError> {
    let cache = inputs.embedding_cache.clone();
    let budget = cache
        .as_ref()
        .map_or(268435456, crate::postgres::Store::receipt_budget);
    let mut embeddings = crate::embed::Session::new(snapshot_id, cache, budget);
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

    // The one public-path authority (the holistic assessment's A1; ADR-0019's amendment): from the
    // snapshot and the config's public roots, written before Stage E, which reads it.
    let public = crate::sql::fetch::<cpg_schema::findings::PublicPathsRow>(
        &ctx,
        &cpg_schema::public::public_paths(),
        crate::sql::Params::new().texts("roots", &inputs.public_roots),
    )
    .await?;
    // Every analysis table exists for validators and retained consumers. Unselected producers
    // leave typed empty tables; the compilation row records why, independently of row counts.
    macro_rules! initialize { ($($t:ty),+) => { $(
        ctx.register_batch(<$t as Table>::NAME, <$t as Table>::to_batch(&[])?)?;
    )+ }; }
    cpg_schema::for_each_analysis_table!(initialize);
    let (mut catalog, catalog_stages) = match crate::stage_cache::contracts(
        root,
        &ctx,
        snapshot_id,
        snapshot_id,
        &inputs.public_roots,
        &public,
        clean,
    )
    .await
    {
        Ok((catalog, steps)) => {
            for step in &steps {
                written.stages.mark(format!(
                    "catalog stage {:?}: {:?}",
                    step.stage, step.outcome
                ));
            }
            (catalog, steps)
        }
        Err(error) => {
            // Keep named invariant diagnostics for malformed inputs that strict typed decoding
            // cannot consume. The successful path retains one full publication validation.
            let cache = crate::validate::cached_session(&ctx).await?;
            let (violations, _) = crate::validate::relational_costed(&cache).await?;
            return Err(if violations.is_empty() {
                error
            } else {
                CoreError::Invalid(violations)
            });
        }
    };
    write_analysis::<cpg_schema::catalog::CatalogCompilation>(
        &ctx,
        root,
        snapshot_id,
        &[cpg_schema::catalog::CatalogCompilationRow {
            snapshot_id,
            profile: inputs.profile.name().into(),
            public_roots: inputs.public_roots.clone(),
            input_digest: inputs.digest(analysis.map(|(a, _)| a)),
        }],
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::findings::PublicPaths>(
        &ctx,
        root,
        snapshot_id,
        &public,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::context_protocol::ModelContextProtocols>(
        &ctx,
        root,
        snapshot_id,
        &models.contexts,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelTargets>(
        &ctx,
        root,
        snapshot_id,
        &models.targets,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelTransfers>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.transfers,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelEffects>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.effects,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelCallbacks>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.callbacks,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelResources>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.resources,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelExceptions>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.exceptions,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::behavior::ModelFormalPaths>(
        &ctx,
        root,
        snapshot_id,
        &models.rules.formals,
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModelApplications>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::model_applications(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModelArgumentBindings>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::model_argument_bindings(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModeledCallbackSites>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::modeled_callback_sites(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModeledResourceSites>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::modeled_resource_sites(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModeledTransferSites>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::modeled_transfer_sites(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModeledEffectSites>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::modeled_effect_sites(),
        &mut written,
    )
    .await?;
    write_analysis_query::<cpg_schema::behavior::ModeledExceptionSites>(
        &ctx,
        root,
        snapshot_id,
        &cpg_schema::behavior::modeled_exception_sites(),
        &mut written,
    )
    .await?;

    // Stage 2.6 (ADR-0022): what the flow IR says about the release, before the behavior scan
    // reads it.
    let flow_model = match analysis {
        Some(_) => {
            let rows = crate::flow_model::run(&ctx, snapshot_id).await?;
            written.stages.mark("behavior: the flow model");
            rows
        }
        None => crate::flow_model::FlowModelRows::default(),
    };
    let (entry_links, exact_origins) = match analysis {
        Some(_) => crate::entry_links::all(&ctx, snapshot_id).await?,
        None => (Vec::new(), Vec::new()),
    };
    // The behavior model's Stage 1 (ADR-0021, ADR-0022) runs inside this block after the finite
    // summaries, whose decisions discharge call-transfer claims (ADR-0064); before Stage E.
    let behavior = if analysis.is_some() {
        use cpg_schema::behavior::{
            AmbientReads, AnalysisConditionNodes, AnalysisConditions, ArgumentFlows, BehaviorSteps,
            Behaviors, Delegations, DynamicAccesses, ExitSites, ExpressionEvaluationSteps,
            ExpressionEvaluations, FieldAccesses, FlowReachBoundaries, FlowTestExactOrigins,
            FlowTestValueLinks, Guards, HandlerActions, HandlerClauses, HandlerReturnNoneSites,
            HandlerTypes, Handoffs, ModeledArgumentEvaluations, ModeledAssignmentReturnPaths,
            ModeledExactValueTransfers, ModeledExceptionHandlerCandidates,
            ModeledExceptionHandlerWalks, ModeledExceptionReturnNonePaths, NegativePremises,
            OperationFacetStatus, OperationFacets, Operations, ParameterReads, RaiseSites,
            ReturnExitStatuses, ReturnExitSteps, Singletons, StatementCompletionSteps,
            StatementCompletions, SummaryBoundaries, SummaryComponents, SummaryFlowSteps,
            SummaryFlows, ValueFlowContributions, ValueFlowPredecessorCandidates,
            ValueFlowPredecessorCompatibility, ValueFlows,
        };
        let w = &mut written;
        let m = &flow_model;
        let (analysis_conditions, analysis_condition_nodes) =
            crate::flow_model::condition_catalog_rows(snapshot_id, &m.condition_models);
        write_analysis::<AnalysisConditions>(&ctx, root, snapshot_id, &analysis_conditions, w)
            .await?;
        write_analysis::<AnalysisConditionNodes>(
            &ctx,
            root,
            snapshot_id,
            &analysis_condition_nodes,
            w,
        )
        .await?;
        write_analysis::<ValueFlows>(&ctx, root, snapshot_id, &m.value_flows, w).await?;
        write_analysis::<FlowReachBoundaries>(&ctx, root, snapshot_id, &m.reach_boundaries, w)
            .await?;
        write_analysis::<ValueFlowContributions>(
            &ctx,
            root,
            snapshot_id,
            &m.value_flow_contributions,
            w,
        )
        .await?;
        write_analysis_query::<ValueFlowPredecessorCandidates>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::value_flow_predecessor_candidates(),
            w,
        )
        .await?;
        let predecessor_compatibility = crate::summaries::predecessor_compatibility(&ctx).await?;
        write_analysis::<ValueFlowPredecessorCompatibility>(
            &ctx,
            root,
            snapshot_id,
            &predecessor_compatibility,
            w,
        )
        .await?;
        write_analysis_query::<ModeledExactValueTransfers>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_exact_value_transfers(),
            w,
        )
        .await?;
        write_analysis::<FlowTestValueLinks>(&ctx, root, snapshot_id, &entry_links, w).await?;
        write_analysis::<FlowTestExactOrigins>(&ctx, root, snapshot_id, &exact_origins, w).await?;
        write_analysis::<FieldAccesses>(&ctx, root, snapshot_id, &m.field_accesses, w).await?;
        write_analysis::<AmbientReads>(&ctx, root, snapshot_id, &m.ambient_reads, w).await?;
        write_analysis::<DynamicAccesses>(&ctx, root, snapshot_id, &m.dynamic_accesses, w).await?;
        write_analysis::<RaiseSites>(&ctx, root, snapshot_id, &m.raise_sites, w).await?;
        write_analysis::<Singletons>(&ctx, root, snapshot_id, &m.singletons, w).await?;
        write_analysis::<NegativePremises>(&ctx, root, snapshot_id, &m.premises, w).await?;
        // Summaries read argument flows before the behavior scan runs (ADR-0064).
        write_analysis_query::<ArgumentFlows>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::argument_flows(),
            w,
        )
        .await?;
        write_analysis_query::<ExitSites>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::exit_sites(),
            w,
        )
        .await?;
        write_analysis_query::<HandlerClauses>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::handler_clauses(),
            w,
        )
        .await?;
        write_analysis_query::<HandlerTypes>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::handler_types(),
            w,
        )
        .await?;
        let contexts = crate::summaries::source_contexts(&ctx).await?;
        write_analysis::<cpg_schema::context_protocol::SourceContextSites>(
            &ctx,
            root,
            snapshot_id,
            &contexts.sites,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::context_protocol::SourceContextArguments>(
            &ctx,
            root,
            snapshot_id,
            &contexts.arguments,
            w,
        )
        .await?;
        let prepared = crate::summaries::execution(&ctx).await?;
        let expressions = prepared.expressions;
        write_analysis::<cpg_schema::frame_exit::ModelFrameExits>(
            &ctx,
            root,
            snapshot_id,
            &expressions.frames,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::frame_exit::ModelFrameExitArguments>(
            &ctx,
            root,
            snapshot_id,
            &expressions.frame_arguments,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::frame_exit::ModelFrameExitSteps>(
            &ctx,
            root,
            snapshot_id,
            &expressions.frame_steps,
            w,
        )
        .await?;
        write_analysis::<ExpressionEvaluations>(
            &ctx,
            root,
            snapshot_id,
            &expressions.evaluations,
            w,
        )
        .await?;
        write_analysis::<ExpressionEvaluationSteps>(&ctx, root, snapshot_id, &expressions.steps, w)
            .await?;
        write_analysis_query::<ModeledArgumentEvaluations>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_argument_evaluations(),
            w,
        )
        .await?;
        write_analysis_query::<ModeledAssignmentReturnPaths>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_assignment_return_paths(),
            w,
        )
        .await?;
        let completions = prepared.completions;
        write_analysis::<cpg_schema::source_call::SourceCallBindings>(
            &ctx,
            root,
            snapshot_id,
            &completions.source_bindings,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::source_call::SourceCallNormals>(
            &ctx,
            root,
            snapshot_id,
            &completions.source_calls,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::source_call::SourceCallHeaderSteps>(
            &ctx,
            root,
            snapshot_id,
            &completions.source_call_headers,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::source_body::SourceBodyCompletions>(
            &ctx,
            root,
            snapshot_id,
            &completions.bodies,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::source_body::SourceBodySteps>(
            &ctx,
            root,
            snapshot_id,
            &completions.body_steps,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::source_body::SourceBodyReleaseInputs>(
            &ctx,
            root,
            snapshot_id,
            &completions.body_releases,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::call_execution::CallExecutions>(
            &ctx,
            root,
            snapshot_id,
            &completions.calls,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::call_execution::CallExecutionSteps>(
            &ctx,
            root,
            snapshot_id,
            &completions.call_steps,
            w,
        )
        .await?;
        let actions = crate::summaries::action_assessments(&ctx).await?;
        write_analysis::<cpg_schema::action::ModeledActionAssessments>(
            &ctx,
            root,
            snapshot_id,
            &actions.assessments,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::action::ModeledActionPostconditions>(
            &ctx,
            root,
            snapshot_id,
            &actions.postconditions,
            w,
        )
        .await?;

        write_analysis::<cpg_schema::completion_proof::ReturnCompletionCertificates>(
            &ctx,
            root,
            snapshot_id,
            &completions.certificates,
            w,
        )
        .await?;
        write_analysis::<StatementCompletions>(&ctx, root, snapshot_id, &completions.statements, w)
            .await?;
        write_analysis::<StatementCompletionSteps>(
            &ctx,
            root,
            snapshot_id,
            &completions.statement_steps,
            w,
        )
        .await?;
        write_analysis::<ReturnExitStatuses>(&ctx, root, snapshot_id, &completions.returns, w)
            .await?;
        write_analysis::<ReturnExitSteps>(&ctx, root, snapshot_id, &completions.return_steps, w)
            .await?;
        write_analysis::<cpg_schema::behavior::ReturnEntryStatuses>(
            &ctx,
            root,
            snapshot_id,
            &completions.entries,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::behavior::ReturnEntrySteps>(
            &ctx,
            root,
            snapshot_id,
            &completions.entry_steps,
            w,
        )
        .await?;
        write_analysis_query::<ModeledExceptionHandlerCandidates>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_exception_handler_candidates(),
            w,
        )
        .await?;
        write_analysis_query::<ModeledExceptionHandlerWalks>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_exception_handler_walks(),
            w,
        )
        .await?;
        write_analysis_query::<HandlerActions>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::handler_actions(),
            w,
        )
        .await?;
        write_analysis_query::<HandlerReturnNoneSites>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::handler_return_none_sites(),
            w,
        )
        .await?;
        write_analysis_query::<ModeledExceptionReturnNonePaths>(
            &ctx,
            root,
            snapshot_id,
            &cpg_schema::behavior::modeled_exception_return_none_paths(),
            w,
        )
        .await?;

        let summary_components = crate::summaries::call_components(&ctx).await?;
        write_analysis::<SummaryComponents>(&ctx, root, snapshot_id, &summary_components, w)
            .await?;
        let summaries = crate::summaries::finite_flows(&ctx).await?;
        write_analysis::<cpg_schema::modeled_identity::SourceModeledIdentities>(
            &ctx,
            root,
            snapshot_id,
            &summaries.modeled_identities,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::parameter_identity::SourceParameterIdentities>(
            &ctx,
            root,
            snapshot_id,
            &summaries.identities,
            w,
        )
        .await?;
        write_analysis::<cpg_schema::context_value::SourceContextValueIdentities>(
            &ctx,
            root,
            snapshot_id,
            &summaries.context_identities,
            w,
        )
        .await?;
        write_analysis::<SummaryFlows>(&ctx, root, snapshot_id, &summaries.flows, w).await?;
        write_analysis::<SummaryFlowSteps>(&ctx, root, snapshot_id, &summaries.steps, w).await?;
        write_analysis::<SummaryBoundaries>(&ctx, root, snapshot_id, &summaries.boundaries, w)
            .await?;
        write_analysis::<cpg_schema::behavior::SummaryOriginCoverage>(
            &ctx,
            root,
            snapshot_id,
            &summaries.coverage,
            w,
        )
        .await?;
        let decisions = lctx_analytics::summaries::discharge::Decisions::from_outcome(
            &summaries.flows,
            &summaries.boundaries,
        );
        let behavior = match analysis {
            Some((a, compiler)) => {
                crate::behavior::run(
                    &ctx,
                    &mut embeddings,
                    snapshot_id,
                    a,
                    compiler,
                    &catalog,
                    m,
                    &decisions,
                    &mut w.stages,
                )
                .await?
            }
            None => crate::behavior::BehaviorRows::default(),
        };
        write_analysis::<Guards>(&ctx, root, snapshot_id, &behavior.guards, w).await?;
        write_analysis::<ParameterReads>(&ctx, root, snapshot_id, &behavior.parameter_reads, w)
            .await?;
        write_analysis::<Handoffs>(&ctx, root, snapshot_id, &behavior.handoffs, w).await?;
        write_analysis::<Delegations>(&ctx, root, snapshot_id, &behavior.delegations, w).await?;
        write_analysis::<Operations>(&ctx, root, snapshot_id, &behavior.operations, w).await?;
        write_analysis::<OperationFacets>(&ctx, root, snapshot_id, &behavior.facets, w).await?;
        write_analysis::<OperationFacetStatus>(&ctx, root, snapshot_id, &behavior.facet_status, w)
            .await?;
        write_analysis::<Behaviors>(&ctx, root, snapshot_id, &behavior.behaviors, w).await?;
        write_analysis::<BehaviorSteps>(&ctx, root, snapshot_id, &behavior.steps, w).await?;
        write_analysis::<cpg_schema::behavior::BehaviorDischarges>(
            &ctx,
            root,
            snapshot_id,
            &behavior.discharges,
            w,
        )
        .await?;
        behavior
    } else {
        let mut behavior = crate::behavior::BehaviorRows::default();
        crate::catalog::populate(
            &ctx,
            &mut embeddings,
            snapshot_id,
            inputs.embedder.as_deref(),
            &catalog,
            &mut behavior,
            &mut written.stages,
        )
        .await?;
        write_analysis::<cpg_schema::behavior::Operations>(
            &ctx,
            root,
            snapshot_id,
            &behavior.operations,
            &mut written,
        )
        .await?;
        write_analysis::<cpg_schema::behavior::OperationFacets>(
            &ctx,
            root,
            snapshot_id,
            &behavior.facets,
            &mut written,
        )
        .await?;
        write_analysis::<cpg_schema::behavior::OperationFacetStatus>(
            &ctx,
            root,
            snapshot_id,
            &behavior.facet_status,
            &mut written,
        )
        .await?;
        behavior
    };
    macro_rules! contextual {
        ($table:ty, $field:ident) => {
            write_analysis::<$table>(
                &ctx,
                root,
                snapshot_id,
                &catalog.contextual.$field,
                &mut written,
            )
            .await?;
        };
    }
    contextual!(cpg_schema::evidence::CatalogArtifacts, artifacts);
    contextual!(cpg_schema::evidence::CatalogSpans, spans);
    contextual!(cpg_schema::evidence::CatalogScenarios, scenarios);
    contextual!(cpg_schema::evidence::CatalogDeployments, deployments);
    contextual!(cpg_schema::evidence::CatalogAssociations, associations);
    write_analysis::<cpg_schema::selection::catalog::CatalogSelectionDomains>(
        &ctx,
        root,
        snapshot_id,
        &catalog.domains,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogSurfaces>(
        &ctx,
        root,
        snapshot_id,
        &catalog.surfaces,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogConfigurations>(
        &ctx,
        root,
        snapshot_id,
        &catalog.configurations,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogFieldLinks>(
        &ctx,
        root,
        snapshot_id,
        &catalog.field_links,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogConstructors>(
        &ctx,
        root,
        snapshot_id,
        &catalog.constructors,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogBindings>(
        &ctx,
        root,
        snapshot_id,
        &catalog.bindings,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogSignatures>(
        &ctx,
        root,
        snapshot_id,
        &catalog.signatures,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogParameters>(
        &ctx,
        root,
        snapshot_id,
        &catalog.parameters,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogEvidence>(
        &ctx,
        root,
        snapshot_id,
        &catalog.evidence,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogTypes>(
        &ctx,
        root,
        snapshot_id,
        &catalog.types,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogTypeArgs>(
        &ctx,
        root,
        snapshot_id,
        &catalog.type_args,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::catalog::CatalogTypeObservations>(
        &ctx,
        root,
        snapshot_id,
        &catalog.type_observations,
        &mut written,
    )
    .await?;

    // Stage E (ADR-0019): the analyses read the session, and their rows are written like any
    // other table's, one commit each.
    // Each technique marks its own stage (the holistic assessment's D1).
    let found = match analysis {
        Some((a, compiler)) => {
            crate::analyze::run(
                &ctx,
                &mut embeddings,
                snapshot_id,
                a,
                compiler,
                &public,
                &mut written.stages,
            )
            .await?
        }
        None => AnalysisRows::default(),
    };
    let mut found = found;
    found.invocations.extend(behavior.invocations);
    use cpg_schema::findings::{AnalysisInvocations, FindingMembers, Findings, Witnesses};
    write_analysis::<AnalysisInvocations>(
        &ctx,
        root,
        snapshot_id,
        &found.invocations,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::concept_attributes::ConceptAttributes>(
        &ctx,
        root,
        snapshot_id,
        &found.concept_attributes,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::concept_attributes::ConceptIncidences>(
        &ctx,
        root,
        snapshot_id,
        &found.concept_incidences,
        &mut written,
    )
    .await?;
    write_analysis::<Findings>(&ctx, root, snapshot_id, &found.findings, &mut written).await?;
    write_analysis::<FindingMembers>(&ctx, root, snapshot_id, &found.members, &mut written).await?;
    write_analysis::<Witnesses>(&ctx, root, snapshot_id, &found.witnesses, &mut written).await?;

    // Stage F (DESIGN §10): assertions and briefs from the findings, written the same way.
    let mut made = match analysis {
        Some((_, compiler)) => {
            crate::synth::run(&ctx, snapshot_id, compiler, &found, &public).await?
        }
        None => crate::synth::SynthRows {
            policy: crate::synth::policy_rows(snapshot_id),
            ..Default::default()
        },
    };
    written.stages.mark("synthesize");
    if let Some(embedder) = analysis.and_then(|(a, _)| a.embedder.as_deref()) {
        embeddings
            .documents(embedder, &mut made.brief_documents)
            .await?;
        written.stages.mark("embed brief documents");
    }
    let receipt = embeddings.finish()?;
    for member in &mut catalog.members {
        if let Some(reason) = member
            .operation_node_id
            .and_then(|id| made.skipped.get(&id))
        {
            member.brief_status = "skipped".into();
            member.brief_reason = Some(reason.clone());
        } else if made
            .briefs
            .iter()
            .any(|b| Some(b.seed_node_id) == member.operation_node_id)
        {
            member.brief_status = "available".into();
        } else if analysis.is_some() {
            member.brief_status = "not_selected".into();
        }
    }
    write_analysis::<cpg_schema::catalog::CatalogMembers>(
        &ctx,
        root,
        snapshot_id,
        &catalog.members,
        &mut written,
    )
    .await?;
    let embedded = receipt
        .spec
        .as_ref()
        .zip(receipt.digest)
        .map(|(s, d)| (s.hash(), d));
    let specs = receipt
        .spec
        .iter()
        .map(|s| cpg_schema::findings::EmbeddingSpecsRow {
            snapshot_id,
            spec_hash: s.hash(),
            spec: s.canonical_json(),
        })
        .collect::<Vec<_>>();
    write_analysis::<cpg_schema::embedding::UsedEmbeddings>(
        &ctx,
        root,
        snapshot_id,
        &receipt.values,
        &mut written,
    )
    .await?;
    write_analysis::<cpg_schema::embedding::EmbeddingUses>(
        &ctx,
        root,
        snapshot_id,
        &receipt.uses,
        &mut written,
    )
    .await?;
    use cpg_schema::findings::{
        AssertionPolicy, AssertionSupport, Assertions, BriefAssertions, BriefDocuments,
        BriefMembers, Briefs, EmbeddingSpecs, Evidence,
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
    write_analysis::<EmbeddingSpecs>(&ctx, root, snapshot_id, &specs, &mut written).await?;
    write_analysis::<AssertionPolicy>(&ctx, root, snapshot_id, &made.policy, &mut written).await?;
    macro_rules! persist_unselected { ($($t:ty),+) => { $(
        if !written.versions.contains_key(<$t as Table>::NAME) {
            write_analysis::<$t>(&ctx, root, snapshot_id, &[], &mut written).await?;
        }
    )+ }; }
    cpg_schema::for_each_analysis_table!(persist_unselected);
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

    let digest = content_digest_with(&runs, embedded);
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
        catalog_stages,
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

    /// The holistic assessment's A2(e): every `.rs`/`.sql` file of the three source trees is in the
    /// source digest, so no module can change the compiler's output without moving it.
    #[test]
    fn every_compiler_source_is_hashed() {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else if path.extension().is_some_and(|e| e == "rs" || e == "sql") {
                    out.push(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let mut found = Vec::new();
        for tree in [
            "crates/lctx-model/src",
            "crates/cpg-core/src",
            "crates/lctx-analytics/src",
            "crates/cpg-schema/src",
        ] {
            walk(&root, &root.join(tree), &mut found);
        }
        found.push("crates/lctx-postgres/src/cache.rs".to_owned());
        found.sort();
        let hashed: Vec<&str> = env!("LCTX_SOURCE_FILES").split(';').collect();
        assert_eq!(found, hashed);
        assert!(hashed.contains(&"crates/cpg-core/src/synth.rs"));
        assert_eq!(SOURCE_DIGEST.len(), 64);
    }

    #[test]
    fn producer_identity_covers_every_canonical_producer() {
        let excluded: Vec<&str> = PRODUCER_EXCLUDED.split(';').filter(|f| !f.is_empty()).collect();
        for file in &excluded {
            assert!(
                file.contains("/bundle") || file.contains("retrieval") || file.contains("/wire/"),
                "{file} is excluded from producer identity but is not serving-only"
            );
        }
        for producer in ["crates/cpg-core/src/evidence.rs", "crates/cpg-core/src/catalog_domains.rs"] {
            assert!(!excluded.contains(&producer), "{producer} produces canonical relations");
        }
        assert_ne!(producer_digest(), compiler_digest());
    }

    #[test]
    fn the_compiler_digest_follows_each_input() {
        let d = vec![("t", "SELECT 1".to_owned())];
        let c = vec![("t", "contract".to_owned())];
        let r = vec![("key:t".to_owned(), "SELECT 2".to_owned())];
        let base = compiler_digest_of("engines", 1, 1, 1, &d, &c, &r);
        assert_ne!(base, compiler_digest_of("engines'", 1, 1, 1, &d, &c, &r));
        assert_ne!(base, compiler_digest_of("engines", 2, 1, 1, &d, &c, &r));
        assert_ne!(base, compiler_digest_of("engines", 1, 2, 1, &d, &c, &r));
        // Increment-1 deep review F1: a template revision is a compiler revision.
        assert_ne!(base, compiler_digest_of("engines", 1, 1, 2, &d, &c, &r));
        let d2 = vec![("t", "SELECT 1 ".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, 1, 1, &d2, &c, &r));
        let c2 = vec![("t", "contract'".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, 1, 1, &d, &c2, &r));
        let r2 = vec![("key:t".to_owned(), "SELECT 3".to_owned())];
        assert_ne!(base, compiler_digest_of("engines", 1, 1, 1, &d, &c, &r2));
        assert!(ENGINES.contains("datafusion 55.1.0"), "{ENGINES}");
        assert!(ENGINES.contains("deltalake-core 1.0.0 git+"), "{ENGINES}");
    }
}
