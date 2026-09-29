//! Declared Arrow types (DESIGN §3.3; cutover plan WP1.0): every batch a session holds has its
//! table's declared schema, whichever store it was read from, so a query plans the same way over
//! the legacy read-back and over in-memory compute.

use arrow_array::{ArrayRef, RecordBatch};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_schema::{DataType, SchemaRef};
use cpg_schema::table::Table;

use crate::CoreError;

const STRICT: CastOptions<'static> = CastOptions {
    safe: false,
    format_options: arrow_cast::display::FormatOptions::new(),
};

/// Cast a batch to its table's declared schema: `BinaryView → Binary → FixedSizeBinary` in two
/// steps (no direct cast exists), `Utf8View → Utf8`, and list children back to their declared
/// field. Casts are strict: a wrong width is an error.
pub fn to_declared<T: Table>(batch: &RecordBatch) -> Result<RecordBatch, CoreError> {
    to_schema(batch, &T::schema())
}

/// [`to_declared`] for any declared schema: columns by name, the same strict casts.
pub fn to_schema(batch: &RecordBatch, schema: &SchemaRef) -> Result<RecordBatch, CoreError> {
    let schema = schema.clone();
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let col = batch.column(batch.schema().index_of(field.name())?).clone();
        let cast = match field.data_type() {
            DataType::FixedSizeBinary(_) if col.data_type() != field.data_type() => {
                let binary = cast_with_options(&col, &DataType::Binary, &STRICT)?;
                cast_with_options(&binary, field.data_type(), &STRICT)?
            }
            other => cast_with_options(&col, other, &STRICT)?,
        };
        columns.push(cast);
    }
    Ok(RecordBatch::try_new(schema, columns)?)
}

/// A legacy table's declared schema, by name. An unknown name is an error, never another table's.
pub fn declared_schema(name: &str) -> Result<SchemaRef, CoreError> {
    macro_rules! find {
        ($($t:ty),+) => {$(
            if name == <$t as Table>::NAME {
                return Ok(<$t as Table>::schema());
            }
        )+};
    }
    cpg_schema::for_each_table!(find);
    cpg_schema::for_each_derived_table!(find);
    cpg_schema::for_each_analysis_table!(find);
    find!(cpg_schema::tables::Snapshots);
    Err(CoreError::UnknownTable(name.to_owned()))
}

/// A schema as a session registers it: schema-level metadata removed (a UNION of two tables with
/// different `lctx.table` metadata does not plan), field metadata kept.
pub fn session_schema(schema: &SchemaRef) -> SchemaRef {
    std::sync::Arc::new(arrow_schema::Schema::new(schema.fields().clone()))
}

/// A batch rebound to [`session_schema`].
pub fn session_batch(batch: &RecordBatch) -> Result<RecordBatch, CoreError> {
    Ok(RecordBatch::try_new(
        session_schema(&batch.schema()),
        batch.columns().to_vec(),
    )?)
}
