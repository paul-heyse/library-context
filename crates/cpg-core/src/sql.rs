//! The one SQL entry point (DESIGN §4.3; review F7): DDL, DML and statements are disallowed, so no
//! query can write to a table and bypass `DeltaTable::write`'s CHECK and invariant validation.

use cpg_schema::Id;
use cpg_schema::query::{QueryRow, Relation};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::ScalarValue;
use datafusion::dataframe::DataFrame;
use datafusion::error::Result;
use datafusion::execution::context::SQLOptions;
use datafusion::prelude::SessionContext;

use crate::CoreError;

pub fn read_only() -> SQLOptions {
    SQLOptions::new()
        .with_allow_ddl(false)
        .with_allow_dml(false)
        .with_allow_statements(false)
}

/// Plan a read-only query. Any other code path that calls `ctx.sql` is a rule violation
/// (`rules/sql-through-helper.yml`).
pub async fn query(ctx: &SessionContext, sql: &str) -> Result<DataFrame> {
    ctx.sql_with_options(sql, read_only()).await
}

/// Run a read-only query and render its result as a table (the `lctx query` output).
pub async fn render(ctx: &SessionContext, sql: &str) -> Result<String> {
    let batches = query(ctx, sql).await?.collect().await?;
    Ok(datafusion::arrow::util::pretty::pretty_format_batches(&batches)?.to_string())
}

/// A relation's parameters, bound by name (`$ids`, `$roots`, …) through DataFusion's
/// `with_param_values`: a value never becomes SQL text, so no quoting or `X'…'` literal is built by
/// hand (the holistic assessment's A4).
#[derive(Debug, Default, Clone)]
pub struct Params(Vec<(String, ScalarValue)>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    /// `$name` as a list of ids, for `array_has($name, column)`. They are bound as `BinaryView`, the
    /// type a session's Delta scans give every id column (`array_has` coerces neither
    /// `FixedSizeBinary` nor `Binary` to it). An empty list is a list, and matches nothing.
    pub fn ids(mut self, name: &str, ids: impl IntoIterator<Item = Id>) -> Self {
        let values: Vec<ScalarValue> = ids
            .into_iter()
            .map(|id| ScalarValue::BinaryView(Some(id.0.to_vec())))
            .collect();
        self.0.push((
            name.to_owned(),
            ScalarValue::List(ScalarValue::new_list_nullable(
                &values,
                &DataType::BinaryView,
            )),
        ));
        self
    }

    /// `$name` as a list of texts, for `array_has($name, column)`.
    pub fn texts<S: AsRef<str>>(mut self, name: &str, texts: impl IntoIterator<Item = S>) -> Self {
        let values: Vec<ScalarValue> = texts
            .into_iter()
            .map(|t| ScalarValue::Utf8(Some(t.as_ref().to_owned())))
            .collect();
        self.0.push((
            name.to_owned(),
            ScalarValue::List(ScalarValue::new_list_nullable(&values, &DataType::Utf8)),
        ));
        self
    }

    /// `$name` as one text.
    pub fn text(mut self, name: &str, text: &str) -> Self {
        self.0
            .push((name.to_owned(), ScalarValue::Utf8(Some(text.to_owned()))));
        self
    }

    /// `$name` as one integer.
    pub fn int(mut self, name: &str, value: i64) -> Self {
        self.0
            .push((name.to_owned(), ScalarValue::Int64(Some(value))));
        self
    }
}

/// Run a relation with its parameters bound, and read every row as `R`: the result is cast to
/// `R`'s schema (by column name), so a null where `R` admits none, or a code outside its
/// codebook, is an error.
pub async fn fetch<R: QueryRow>(
    ctx: &SessionContext,
    relation: &Relation,
    params: Params,
) -> Result<Vec<R>, CoreError> {
    let mut frame = query(ctx, &relation.sql).await?;
    if !params.0.is_empty() {
        frame = frame.with_param_values(params.0)?;
    }
    let schema = R::schema();
    let mut rows = Vec::new();
    for batch in frame.collect().await? {
        let batch = crate::delta::to_schema(&batch, &schema)?;
        rows.extend(R::read_batch(&batch)?);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datafusion::arrow::array::{BinaryViewArray, RecordBatch, StringArray};
    use datafusion::arrow::datatypes::{Field, Schema};

    use super::*;

    cpg_schema::query_row! {
        struct Named {
            node_id: Id,
            name: String,
        }
    }

    fn session() -> SessionContext {
        let ctx = SessionContext::new();
        // Ids as a session's Delta scans give them (`BinaryView`), never the declared
        // `FixedSizeBinary(16)`.
        let schema = Arc::new(Schema::new(vec![
            Field::new("node_id", DataType::BinaryView, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(BinaryViewArray::from_iter_values([
                    [1u8; 16], [2; 16], [3; 16],
                ])),
                Arc::new(StringArray::from(vec![Some("a"), Some("o'brien"), None])),
            ],
        )
        .unwrap();
        ctx.register_batch("names", batch).unwrap();
        ctx
    }

    fn relation(sql: &str) -> Relation {
        Relation {
            name: "probe",
            sql: sql.to_owned(),
            deps: &["names"],
        }
    }

    /// The holistic assessment's A4 probe: a bound id list selects by `array_has`, an empty one
    /// selects nothing, and a text with a quote is a value, not SQL.
    #[tokio::test]
    async fn parameters_are_bound_never_spliced() {
        let ctx = session();
        let by_ids = relation(
            "SELECT node_id, name FROM names WHERE array_has($ids, node_id) ORDER BY node_id",
        );
        let rows: Vec<Named> = fetch(
            &ctx,
            &by_ids,
            Params::new().ids("ids", [Id([2; 16]), Id([1; 16])]),
        )
        .await
        .unwrap();
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["a", "o'brien"]);

        let none: Vec<Named> = fetch(&ctx, &by_ids, Params::new().ids("ids", []))
            .await
            .unwrap();
        assert!(none.is_empty());

        let by_name = relation("SELECT node_id, name FROM names WHERE name = $name");
        let quoted: Vec<Named> = fetch(&ctx, &by_name, Params::new().text("name", "o'brien"))
            .await
            .unwrap();
        assert_eq!(quoted.len(), 1);
        let by_texts = relation("SELECT node_id, name FROM names WHERE array_has($names, name)");
        let listed: Vec<Named> = fetch(
            &ctx,
            &by_texts,
            Params::new().texts("names", ["o'brien", "zz"]),
        )
        .await
        .unwrap();
        assert_eq!(listed.len(), 1);
    }

    /// A8: the third row's name is null, and `Named` admits none.
    #[tokio::test]
    async fn a_null_the_row_does_not_admit_is_an_error() {
        let ctx = session();
        let all = relation("SELECT node_id, name FROM names");
        let err = fetch::<Named>(&ctx, &all, Params::new()).await.unwrap_err();
        assert!(err.to_string().contains("name"), "{err}");
    }
}
