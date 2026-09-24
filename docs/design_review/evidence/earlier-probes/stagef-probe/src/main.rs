use std::sync::Arc;
use arrow_array::{BinaryArray, RecordBatch, StringArray, Array};
use arrow_schema::{DataType, Field, Schema};
use datafusion::common::ScalarValue;
use datafusion::execution::context::SQLOptions;
use datafusion::prelude::*;

fn ids(n: &[u8]) -> Vec<Vec<u8>> { n.iter().map(|b| vec![*b; 16]).collect() }

fn list(values: &[Vec<u8>]) -> ScalarValue {
    let arr: BinaryArray = values.iter().map(|v| Some(v.as_slice())).collect();
    ScalarValue::List(ScalarValue::new_list_nullable(
        &arr.iter().map(|v| ScalarValue::Binary(v.map(|b| b.to_vec()))).collect::<Vec<_>>(),
        &DataType::Binary,
    ))
}

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    let ctx = SessionContext::new();
    let schema = Arc::new(Schema::new(vec![
        Field::new("node_id", DataType::Binary, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    let node: BinaryArray = ids(&[1, 2, 3, 4]).iter().map(|v| Some(v.as_slice())).collect();
    let name = StringArray::from(vec!["a", "b", "c", "d"]);
    let batch = RecordBatch::try_new(schema, vec![Arc::new(node), Arc::new(name)])?;
    ctx.register_batch("declarations", batch)?;
    let opts = SQLOptions::new().with_allow_ddl(false).with_allow_dml(false).with_allow_statements(false);
    let q = "SELECT name FROM declarations WHERE array_has($ids, node_id) ORDER BY name";

    let many: Vec<u8> = (1..=200u8).collect();
    for (label, wanted) in [("two", ids(&[2, 4])), ("empty", vec![]), ("many", ids(&many))] {
        let df = ctx.sql_with_options(q, opts).await?
            .with_param_values(vec![("ids", list(&wanted))])?;
        let plan = df.clone().into_optimized_plan()?; let phys = df.clone().create_physical_plan().await?; let pd = datafusion::physical_plan::displayable(phys.as_ref()).indent(false).to_string(); println!("[{label}] physical: {}", pd.lines().find(|l| l.contains("Filter")).unwrap_or("-").chars().take(160).collect::<String>());
        let rows = df.collect().await?;
        let names: Vec<String> = rows.iter().flat_map(|b| {
            let a = b.column(0).as_any().downcast_ref::<StringArray>().unwrap();
            (0..a.len()).map(|i| a.value(i).to_owned()).collect::<Vec<_>>()
        }).collect();
        let text = plan.display_indent().to_string(); println!("[{label}] rows={}; plan: {}", names.len(), text.chars().take(300).collect::<String>());
    }

    // An empty IN list in the default (Generic) dialect.
    match ctx.sql_with_options("SELECT name FROM declarations WHERE node_id IN ()", opts).await {
        Ok(_) => println!("[in-empty] parsed"),
        Err(e) => println!("[in-empty] error: {e}"),
    }

    // A quote inside a name interpolated with format!.
    let evil = "x'y";
    match ctx.sql_with_options(&format!("SELECT name FROM declarations WHERE name = '{evil}'"), opts).await {
        Ok(_) => println!("[quote] parsed"),
        Err(e) => println!("[quote] error: {e}"),
    }
    // The same through a placeholder.
    let n = ctx.sql_with_options("SELECT name FROM declarations WHERE name = $name", opts).await?
        .with_param_values(vec![("name", ScalarValue::from(evil))])?.count().await?;
    println!("[quote-param] rows={n}");

    // The id set as a DataFrame and a semi join: no catalog registration.
    let wanted_schema = Arc::new(Schema::new(vec![Field::new("wanted_id", DataType::Binary, false)]));
    let w: BinaryArray = ids(&[1, 3]).iter().map(|v| Some(v.as_slice())).collect();
    let wanted = ctx.read_batch(RecordBatch::try_new(wanted_schema, vec![Arc::new(w)])?)?;
    let semi = ctx.table("declarations").await?
        .join(wanted, JoinType::LeftSemi, &["node_id"], &["wanted_id"], None)?
        .sort(vec![col("name").sort(true, true)])?;
    let n = semi.clone().count().await?;
    println!("[semi] rows={n}; tables after: {:?}", ctx.catalog("datafusion").unwrap().schema("public").unwrap().table_names());
    Ok(())
}
