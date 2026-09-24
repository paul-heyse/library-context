//! Typed query results and named relations (the holistic assessment's A4, B2 and A8).
//!
//! A query's result row is declared once with [`query_row!`](crate::query_row): the struct, its
//! Arrow schema and its decoder come from the same [`ArrowColumn`](crate::column::ArrowColumn)
//! mapping `table!` uses, so a column's type, nullability and codebook are stated in one place. A
//! result is cast to the row's schema before it is read. A null where the row admits none, or a
//! code outside its codebook, is an error.
//!
//! A [`Relation`] is a named, read-only SQL text over the snapshot's tables, with the tables it
//! reads. Its values arrive as bound parameters (`$ids`, `$roots`), never spliced into the text.
//! `relations!` declares a module's relations and their inventory in one place.

use arrow_array::RecordBatch;
use arrow_schema::{ArrowError, SchemaRef};

/// A query result row: its schema, and how to read one batch of it.
pub trait QueryRow: Sized {
    fn schema() -> SchemaRef;
    /// Every row of `batch`, which has [`Self::schema`] (a caller casts first).
    fn read_batch(batch: &RecordBatch) -> Result<Vec<Self>, ArrowError>;
}

/// A named, read-only SQL relation: its text, and the tables it scans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub name: &'static str,
    pub sql: String,
    pub deps: &'static [&'static str],
}

/// Read one field of a result row, naming it in any error.
#[doc(hidden)]
pub fn read_field<T: crate::column::ArrowColumn>(
    row: &str,
    field: &str,
    column: &dyn arrow_array::Array,
    i: usize,
) -> Result<T, ArrowError> {
    T::read(column, i).map_err(|e| ArrowError::InvalidArgumentError(format!("{row}.{field}: {e}")))
}

/// Declare a query result row once: the struct, its schema and its decoder.
///
/// ```ignore
/// cpg_schema::query_row! {
///     /// A component of a passage.
///     pub struct ComponentRow {
///         document_node_id: Id,
///         parent_ordinal: Option<i64>,
///     }
/// }
/// ```
#[macro_export]
macro_rules! query_row {
    (
        $(#[$meta:meta])*
        $vis:vis struct $row:ident { $($(#[$fmeta:meta])* $field:ident : $ty:ty),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        $vis struct $row { $($(#[$fmeta])* pub $field: $ty),+ }

        impl $crate::query::QueryRow for $row {
            fn schema() -> $crate::arrow_schema::SchemaRef {
                ::std::sync::Arc::new($crate::arrow_schema::Schema::new(vec![
                    $(<$ty as $crate::column::ArrowColumn>::field(stringify!($field))),+
                ]))
            }
            fn read_batch(
                batch: &$crate::arrow_array::RecordBatch,
            ) -> Result<Vec<Self>, $crate::arrow_schema::ArrowError> {
                $crate::__read_rows!($row, batch, $($field: $ty),+)
            }
        }
    };
}

/// The decoder `query_row!` and `table!` share: every row of a batch cast to the row's schema.
#[doc(hidden)]
#[macro_export]
macro_rules! __read_rows {
    ($row:ident, $batch:expr, $($field:ident : $ty:ty),+) => {{
        let batch: &$crate::arrow_array::RecordBatch = $batch;
        let columns = batch.columns();
        (0..batch.num_rows())
            .map(|i| {
                // Struct fields are evaluated in the order written: the schema's order.
                let mut column = columns.iter();
                Ok($row {
                    $($field: $crate::query::read_field::<$ty>(
                        stringify!($row),
                        stringify!($field),
                        column.next().expect("a column per field").as_ref(),
                        i,
                    )?),+
                })
            })
            .collect()
    }};
}

/// Declare named relations and their inventory in one place. Each becomes a function returning
/// its [`Relation`]; `inventory` returns them all, in declaration order.
#[macro_export]
macro_rules! relations {
    (
        inventory $all:ident;
        $(
            $(#[$meta:meta])*
            $f:ident = $name:literal, deps = [$($dep:literal),* $(,)?], sql = $sql:expr;
        )+
    ) => {
        $(
            $(#[$meta])*
            pub fn $f() -> $crate::query::Relation {
                $crate::query::Relation { name: $name, sql: $sql, deps: &[$($dep),*] }
            }
        )+
        /// Every relation this module declares.
        pub fn $all() -> Vec<$crate::query::Relation> {
            vec![$($f()),+]
        }
    };
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{ArrayRef, FixedSizeBinaryArray, Int16Array, Int64Array, StringArray};

    use super::*;
    use crate::codebook::ComponentForm;
    use crate::id::Id;

    crate::query_row! {
        struct Probe {
            node: Id,
            parent: Option<i64>,
            name: String,
            form: ComponentForm,
        }
    }

    fn batch(parent: Vec<Option<i64>>, name: Vec<Option<&str>>, form: Vec<i16>) -> RecordBatch {
        let n = parent.len();
        let columns: Vec<ArrayRef> = vec![
            Arc::new(
                FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n([7u8; 16], n)).unwrap(),
            ),
            Arc::new(Int64Array::from(parent)),
            Arc::new(StringArray::from(name)),
            Arc::new(Int16Array::from(form)),
        ];
        // Nullable fields throughout, so the decoder, not the batch, meets each null.
        let schema = arrow_schema::Schema::new(
            Probe::schema()
                .fields()
                .iter()
                .map(|f| f.as_ref().clone().with_nullable(true))
                .collect::<Vec<_>>(),
        );
        RecordBatch::try_new(Arc::new(schema), columns).unwrap()
    }

    #[test]
    fn a_row_reads_its_declared_columns() {
        let rows = Probe::read_batch(&batch(
            vec![None, Some(0)],
            vec![Some("Warning"), Some("ParamField")],
            vec![0, 1],
        ))
        .unwrap();
        assert_eq!(
            rows[1],
            Probe {
                node: Id([7; 16]),
                parent: Some(0),
                name: "ParamField".to_owned(),
                form: ComponentForm::Text,
            }
        );
        assert_eq!(rows[0].parent, None);
    }

    /// The holistic assessment's A8: a null in a column the row declares non-null is an error that
    /// names the field, never an empty default; so is a code outside the codebook.
    #[test]
    fn a_null_or_an_unknown_code_is_an_error() {
        let null = Probe::read_batch(&batch(vec![None], vec![None], vec![0])).unwrap_err();
        assert!(null.to_string().contains("Probe.name"), "{null}");
        let code = Probe::read_batch(&batch(vec![None], vec![Some("x")], vec![9])).unwrap_err();
        assert!(code.to_string().contains("component_form"), "{code}");
    }

    crate::relations! {
        inventory all;
        /// One.
        one = "one", deps = ["doc_components"], sql = "SELECT 1".to_owned();
        two = "two", deps = [], sql = format!("SELECT {}", 2);
    }

    #[test]
    fn relations_are_listed_where_they_are_declared() {
        let names: Vec<_> = all().iter().map(|r| r.name).collect();
        assert_eq!(names, ["one", "two"]);
        assert_eq!(one().deps, ["doc_components"]);
        assert_eq!(two().sql, "SELECT 2");
    }
}
