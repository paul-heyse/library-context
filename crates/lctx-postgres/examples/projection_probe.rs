//! Bounded 1x/10x transport probe on real pilot vectors. Repeated rows are a COPY workload,
//! not a claim that a repeated semantic generation satisfies its uniqueness constraints.
use arrow_ipc::reader::FileReader;
use arrow_select::concat::concat_batches;
use cpg_schema::serving_projection::{batch_bytes, receipt};
use lctx_postgres::{
    projection::{copy_bytes, decode_rows},
    serving::TEST_IMAGE,
};
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ImageExt, runners::AsyncRunner},
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("supply the pilot operation_vectors.arrow path")?;
    let reader = FileReader::try_new(std::fs::File::open(path)?, None)?;
    let schema = reader.schema();
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    let original = concat_batches(&schema, &batches)?;
    let (name, tag) = TEST_IMAGE.trim().split_once(':').unwrap();
    let db = Postgres::default()
        .with_name(name)
        .with_tag(tag)
        .start()
        .await?;
    let port = db.get_host_port_ipv4(5432).await?;
    let mut conn = sqlx::PgConnection::connect(&format!(
        "postgres://postgres:postgres@127.0.0.1:{port}/postgres"
    ))
    .await?;
    use sqlx::Connection;
    sqlx::raw_sql("CREATE SCHEMA lctx_ext; CREATE EXTENSION vector WITH SCHEMA lctx_ext VERSION '0.8.6'; SET search_path=pg_catalog,lctx_ext; CREATE TEMP TABLE stage(node_id bytea,embedding_view text,chunk bigint,input_hash bytea,vector real[]);").execute(&mut conn).await?;
    for scale in [1, 10] {
        let repeated = vec![&original; scale];
        let batch = concat_batches(&schema, repeated)?;
        let expected = receipt("operation_vectors", 1024, std::slice::from_ref(&batch))?;
        let start = std::time::Instant::now();
        for offset in (0..batch.num_rows()).step_by(1024) {
            let slice = batch.slice(offset, (batch.num_rows() - offset).min(1024));
            let bytes = copy_bytes("operation_vectors", 1024, &slice)?;
            let mut copy = conn.copy_in_raw("COPY stage FROM STDIN BINARY").await?;
            copy.send(bytes.freeze()).await?;
            copy.finish().await?;
        }
        let rows=sqlx::query("SELECT node_id,embedding_view,chunk,input_hash,vector::lctx_ext.vector(1024) vector FROM stage").fetch_all(&mut conn).await?;
        let returned = decode_rows("operation_vectors", 1024, &rows)?;
        assert_eq!(expected, receipt("operation_vectors", 1024, &[returned])?);
        let rss = std::fs::read_to_string("/proc/self/status")?
            .lines()
            .find(|l| l.starts_with("VmHWM:"))
            .unwrap_or("unknown")
            .to_owned();
        println!(
            "{}",
            serde_json::json!({"scale":scale,"rows":batch.num_rows(),"logical_bytes":batch_bytes(&batch)?,"seconds":start.elapsed().as_secs_f64(),"peak_rss":rss,"receipt_equal":true})
        );
        sqlx::query("TRUNCATE stage").execute(&mut conn).await?;
    }
    Ok(())
}
