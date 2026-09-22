//! Delta persistence of the fact-family tables and the one read-only SQL helper (DESIGN §4.3, §6).

pub mod delta;
pub mod sql;

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
        table: &'static str,
        requested: u64,
        loaded: Option<u64>,
    },
}
