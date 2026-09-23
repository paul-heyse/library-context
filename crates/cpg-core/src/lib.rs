//! Delta persistence of the fact-family tables, the one read-only SQL helper, and the compile
//! attempt: write, derive, validate, publish (DESIGN §4.3, §6, §8).

pub mod attempt;
pub mod delta;
pub mod derive;
pub mod snapshot;
pub mod sql;
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
    #[error("{0}: delta.appendOnly is not set")]
    NotAppendOnly(&'static str),
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
    #[error("validation failed, nothing published: {}", summary(.0))]
    Invalid(Vec<Violation>),
    #[error("the snapshots append failed and re-reading shows it unpublished: {0}")]
    Unpublished(Box<CoreError>),
}
