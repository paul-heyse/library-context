//! Delta persistence of the fact-family tables, the one read-only SQL helper, and the compile
//! attempt: write, derive, validate, publish (DESIGN §4.3, §6, §8).

pub mod analyze;
pub mod attempt;
pub mod behavior;
pub mod bundle;
pub mod delta;
pub mod derive;
pub mod diff;
pub mod embed;
pub mod entry_links;
pub mod flow_model;
pub mod primitive_theory;
pub mod snapshot;
pub mod sql;
pub mod synth;
pub mod udf;
pub mod usage;
pub mod validate;

use validate::Violation;

fn summary(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|v| format!("{} ({} rows)", v.rule, v.rows))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("delta: {0}")]
    Delta(#[from] deltalake::DeltaTableError),
    #[error("datafusion: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),
    #[error("arrow: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a table url: {0}")]
    Url(String),
    #[error("a validation rule's task failed: {0}")]
    Rule(String),
    #[error("parquet: {0}")]
    Parquet(String),
    #[error(
        "{table} version {version} is not snapshot {snapshot}'s commit (it records {recorded:?}); a pinned read opens only its own snapshot's commit"
    )]
    ForeignCommit {
        table: String,
        version: u64,
        snapshot: String,
        recorded: Option<String>,
    },
    #[error("no table holds a commit of attempt {0}")]
    NoAttempt(String),
    #[error(
        "{0}: the stored schema differs from the declared contract (a schema migration: use a new store or migrate the table)"
    )]
    SchemaDrift(&'static str),
    #[error("{0}: delta.appendOnly is not set")]
    NotAppendOnly(&'static str),
    #[error("{table}: {property} is {found:?}, not the retention pinned reads need (DESIGN §6.1)")]
    Retention {
        table: &'static str,
        property: String,
        found: Option<String>,
    },
    #[error("{table}: CHECK constraints differ from the declaration: {detail}")]
    ConstraintMismatch { table: &'static str, detail: String },
    #[error("{table}: requested version {requested}, loaded {loaded:?}")]
    VersionMismatch {
        table: String,
        requested: u64,
        loaded: Option<u64>,
    },
    #[error("{0}: the table has no version after a write")]
    NoVersion(&'static str),
    #[error("{0}: unexpected column type")]
    ColumnType(&'static str),
    #[error("the attempt has no batch for table {0}")]
    MissingTable(&'static str),
    #[error("not a raw table: {0}")]
    UnknownTable(String),
    #[error("{0}: the batch schema differs from the declared contract")]
    SchemaMismatch(&'static str),
    #[error("{0}: a row carries another snapshot_id")]
    ForeignSnapshot(&'static str),
    #[error("analysis: {0}")]
    Analysis(String),
    #[error("embedding: {0}")]
    Embed(String),
    #[error("bundle: {0}")]
    Bundle(String),
    #[error("validation failed, nothing published: {}", summary(.0))]
    Invalid(Vec<Violation>),
    #[error("snapshot {0} is already published")]
    AlreadyPublished(String),
    #[error("the snapshots append failed and re-reading shows it unpublished: {0}")]
    Unpublished(Box<CoreError>),
}
