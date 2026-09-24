//! The table contract: one declaration per table generates its row type, Arrow schema, total
//! sort key, immutable CHECK constraints and batch builder (DESIGN §B2, §4.3, DM-52).

use std::sync::Arc;

use arrow_array::{Array, RecordBatch, Scalar};
use arrow_ord::cmp::not_distinct;
use arrow_ord::sort::{SortColumn, SortOptions, lexsort_to_indices};
use arrow_schema::{ArrowError, Schema, SchemaRef};
use arrow_select::take::take_record_batch;

use crate::codebook::{Codebook, FactFamily};
use crate::id::{Digest, IdHasher};

/// Schema metadata key naming the table.
pub const TABLE_KEY: &str = "lctx.table";
/// Schema metadata key naming the table's fact family.
pub const FAMILY_KEY: &str = "lctx.family";

pub trait Table {
    /// The Delta table name.
    const NAME: &'static str;
    const FAMILY: FactFamily;
    type Row;
    fn schema() -> SchemaRef;
    /// The declared **total** key: the canonical sort order, unique within a snapshot.
    fn key() -> &'static [&'static str];
    /// Invariants that never change, created as Delta CHECK constraints and verified when a table
    /// is opened (DESIGN §4.3, §8). Codebook membership is never one of them.
    fn checks() -> &'static [(&'static str, &'static str)];
    /// Build a batch; `RecordBatch::try_new` checks types, nullability and metadata.
    fn to_batch(rows: &[Self::Row]) -> Result<RecordBatch, ArrowError>;

    /// Build and canonically sort a batch.
    fn to_sorted_batch(rows: &[Self::Row]) -> Result<RecordBatch, ArrowError> {
        canonical_sort(&Self::to_batch(rows)?, Self::key())
    }
}

/// Sort by the declared total key. Arrow's sort is unstable, so the key must be total.
///
/// A leading key column that is constant across the batch (a write batch's `snapshot_id`) orders
/// nothing, so it is left out of the comparison: the same order, about 3x faster on `edges` (H1
/// P4). Constancy is checked, never assumed; a batch holding several snapshots sorts by all of it.
pub fn canonical_sort(batch: &RecordBatch, key: &[&str]) -> Result<RecordBatch, ArrowError> {
    if batch.num_rows() < 2 {
        return Ok(batch.clone());
    }
    let mut key = key;
    while key.len() > 1 && constant(batch.column(batch.schema().index_of(key[0])?).as_ref())? {
        key = &key[1..];
    }
    let columns = key
        .iter()
        .map(|name| {
            Ok(SortColumn {
                values: batch.column(batch.schema().index_of(name)?).clone(),
                options: Some(SortOptions {
                    descending: false,
                    nulls_first: true,
                }),
            })
        })
        .collect::<Result<Vec<_>, ArrowError>>()?;
    let indices = lexsort_to_indices(&columns, None)?;
    take_record_batch(batch, &indices)
}

/// Whether every value of `column` equals its first (nulls equal to nulls).
fn constant(column: &dyn Array) -> Result<bool, ArrowError> {
    let first = Scalar::new(column.slice(0, 1));
    Ok(not_distinct(&column, &first)?.true_count() == column.len())
}

/// The store's canonical schema form (DESIGN §6.3; the serving bundle has its own, ADR-0019): table
/// metadata, then per field its name, type,
/// nullability and sorted metadata. Snapshot-tested, and the input to [`schema_digest`].
pub fn canonical_schema(schema: &Schema) -> String {
    let mut out = String::new();
    let mut meta: Vec<_> = schema.metadata().iter().collect();
    meta.sort();
    for (k, v) in meta {
        out.push_str(&format!("@{k}={v}\n"));
    }
    for f in schema.fields() {
        let mut fm: Vec<_> = f.metadata().iter().collect();
        fm.sort();
        let fm: Vec<String> = fm.iter().map(|(k, v)| format!("{k}={v}")).collect();
        out.push_str(&format!(
            "{}: {}{}{}\n",
            f.name(),
            f.data_type(),
            if f.is_nullable() {
                " NULL"
            } else {
                " NOT NULL"
            },
            if fm.is_empty() {
                String::new()
            } else {
                format!(" [{}]", fm.join(", "))
            }
        ));
    }
    out
}

pub fn schema_digest(schema: &Schema) -> Digest {
    IdHasher::new("schema")
        .str(&canonical_schema(schema))
        .finish_digest()
}

/// A table's full contract as text: name, family, key, checks and canonical schema.
pub fn contract<T: Table>() -> String {
    let mut out = format!(
        "table {}\nfamily {}\nkey {}\n",
        T::NAME,
        T::FAMILY.text(),
        T::key().join(", ")
    );
    for (name, expr) in T::checks() {
        out.push_str(&format!("check {name}: {expr}\n"));
    }
    out.push_str(&canonical_schema(&T::schema()));
    out
}

pub(crate) fn table_schema(
    name: &str,
    family: FactFamily,
    fields: Vec<arrow_schema::Field>,
) -> SchemaRef {
    Arc::new(
        Schema::new(fields).with_metadata(
            [
                (TABLE_KEY.to_owned(), name.to_owned()),
                (FAMILY_KEY.to_owned(), family.text().to_owned()),
            ]
            .into(),
        ),
    )
}

/// Declare a table once; the row struct, schema, key, checks and builder are generated.
macro_rules! table {
    (
        $(#[$meta:meta])*
        $table:ident, $row:ident = $name:literal,
        family = $family:ident,
        key = [$($key:ident),+ $(,)?],
        checks = [$(($cname:literal, $cexpr:literal)),* $(,)?],
        { $($(#[$fmeta:meta])* $field:ident : $ty:ty),+ $(,)? }
    ) => {
        #[doc = concat!("A row of `", $name, "`.")]
        #[derive(Debug, Clone, PartialEq)]
        pub struct $row { $($(#[$fmeta])* pub $field: $ty),+ }

        impl $row {
            /// Feed every field, in declaration order, to `h` (canonical payload bytes).
            pub fn hash_fields(&self, h: &mut $crate::id::IdHasher) {
                $(<$ty as $crate::hash::HashField>::hash_into(&self.$field, h);)+
            }
        }

        $(#[$meta])*
        pub struct $table;

        impl $crate::table::Table for $table {
            const NAME: &'static str = $name;
            const FAMILY: $crate::codebook::FactFamily = $crate::codebook::FactFamily::$family;
            type Row = $row;
            fn schema() -> arrow_schema::SchemaRef {
                $crate::table::table_schema(
                    $name,
                    $crate::codebook::FactFamily::$family,
                    vec![$(<$ty as $crate::column::ArrowColumn>::field(stringify!($field))),+],
                )
            }
            fn key() -> &'static [&'static str] {
                &[$(stringify!($key)),+]
            }
            fn checks() -> &'static [(&'static str, &'static str)] {
                &[$(($cname, $cexpr)),*]
            }
            fn to_batch(rows: &[$row]) -> Result<arrow_array::RecordBatch, arrow_schema::ArrowError> {
                let columns: Vec<arrow_array::ArrayRef> = vec![
                    $(<$ty as $crate::column::ArrowColumn>::array(rows.iter().map(|r| &r.$field))),+
                ];
                arrow_array::RecordBatch::try_new(<Self as $crate::table::Table>::schema(), columns)
            }
        }

        // A stored row is also a query result row: `sql::fetch` reads a table's rows as itself.
        impl $crate::query::QueryRow for $row {
            fn schema() -> arrow_schema::SchemaRef {
                <$table as $crate::table::Table>::schema()
            }
            fn read_batch(
                batch: &arrow_array::RecordBatch,
            ) -> Result<Vec<Self>, arrow_schema::ArrowError> {
                $crate::__read_rows!($row, batch, $($field: $ty),+)
            }
        }
    };
}
pub(crate) use table;
