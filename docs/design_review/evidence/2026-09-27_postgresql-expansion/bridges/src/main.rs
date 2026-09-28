use arrow_array::{
    Array, ArrayRef, FixedSizeBinaryArray, Float32Array, RecordBatch, StringArray,
    builder::{Float32Builder, ListBuilder},
};
use arrow_schema::{DataType, Field, Schema};
use bytes::BytesMut;
use pgpq::ArrowToPostgresBinaryEncoder;
use sqlx::{Connection, PgConnection, Row};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut list = ListBuilder::new(Float32Builder::new());
    list.values().append_value(1.25);
    list.values().append_null();
    list.append(true);
    list.append(true);
    list.append(false);
    let list = list.finish();
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false).with_metadata(
            std::collections::HashMap::from([("lctx.probe".into(), "identity".into())]),
        ),
        Field::new("x", DataType::Float32, false),
        Field::new("label", DataType::Utf8, true),
        Field::new("items", list.data_type().clone(), true),
    ]));
    let ids = [[0_u8; 16], [1_u8; 16], [255_u8; 16]];
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(FixedSizeBinaryArray::try_from_iter(ids.into_iter())?) as ArrayRef,
            Arc::new(Float32Array::from(vec![-0.0_f32, 0.25, 1.0])),
            Arc::new(StringArray::from(vec![Some(""), None, Some("null")])),
            Arc::new(list),
        ],
    )?;
    let mut encoder = ArrowToPostgresBinaryEncoder::try_new(&schema)?;
    let mut buf = BytesMut::new();
    encoder.write_header(&mut buf)?;
    encoder.write_batch(&batch, &mut buf)?;
    encoder.write_footer(&mut buf)?;
    let mut conn = PgConnection::connect(&std::env::var("BRIDGE_DATABASE_URL")?).await?;
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&mut conn)
        .await?;
    assert_eq!(version / 10000, 18);
    sqlx::query("CREATE TABLE bridge_probe (id bytea NOT NULL CHECK(octet_length(id)=16), x real NOT NULL, label text, items real[])").execute(&mut conn).await?;
    let mut copy = conn
        .copy_in_raw("COPY bridge_probe (id,x,label,items) FROM STDIN BINARY")
        .await?;
    copy.send(buf.freeze()).await?;
    assert_eq!(copy.finish().await?, 3);
    let rows = sqlx::query("SELECT * FROM bridge_probe ORDER BY id")
        .fetch_all(&mut conn)
        .await?;
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.try_get::<Vec<u8>, _>("id")?, ids[i]);
    }
    assert_eq!(
        rows[0].try_get::<f32, _>("x")?.to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        rows[0].try_get::<Option<String>, _>("label")?,
        Some(String::new())
    );
    assert_eq!(rows[1].try_get::<Option<String>, _>("label")?, None);
    assert_eq!(
        rows[2].try_get::<Option<String>, _>("label")?,
        Some("null".into())
    );
    assert_eq!(
        rows[0].try_get::<Option<Vec<Option<f32>>>, _>("items")?,
        Some(vec![Some(1.25), None])
    );
    assert_eq!(
        rows[1].try_get::<Option<Vec<Option<f32>>>, _>("items")?,
        Some(vec![])
    );
    assert_eq!(
        rows[2].try_get::<Option<Vec<Option<f32>>>, _>("items")?,
        None
    );
    conn.copy_in_raw("COPY bridge_probe (id,x,label,items) FROM STDIN BINARY")
        .await?
        .abort("probe abort")
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM bridge_probe")
            .fetch_one(&mut conn)
            .await?,
        3
    );
    #[cfg(feature = "datafusion-table-providers-postgres")]
    {
        use datafusion_table_providers_postgres::{
            PostgresTableFactory, pool::PostgresConnectionPool,
        };
        use secrecy::SecretString;
        use std::collections::HashMap;
        let options = HashMap::from([
            ("host".into(), SecretString::from("127.0.0.1")),
            (
                "port".into(),
                SecretString::from(std::env::var("BRIDGE_PG_PORT")?),
            ),
            ("user".into(), SecretString::from("postgres")),
            ("pass".into(), SecretString::from("bridge_probe")),
            ("db".into(), SecretString::from("postgres")),
            ("sslmode".into(), SecretString::from("disable")),
        ]);
        let pool = Arc::new(PostgresConnectionPool::new(options).await?);
        let provider = PostgresTableFactory::new(pool)
            .table_provider("bridge_probe".into())
            .await
            .map_err(|e| format!("provider: {e}"))?;
        let ctx = datafusion::prelude::SessionContext::new_with_state(
            datafusion_federation::default_session_state(),
        );
        ctx.register_table("probe", provider)?;
        let df = ctx
            .sql("SELECT id,x,label,items FROM probe WHERE label IS NULL LIMIT 1")
            .await?;
        let batches = df.collect().await?;
        assert_eq!(batches.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
        assert_eq!(batches[0].schema().field(0).data_type(), &DataType::Binary);
        assert!(
            batches[0]
                .schema()
                .field(0)
                .metadata()
                .get("lctx.probe")
                .is_none()
        );
        println!(
            "provider projected metadata: {:?}",
            batches[0].schema().field(0).metadata()
        );
        let empty = ctx
            .sql("SELECT id FROM probe WHERE false")
            .await?
            .collect()
            .await?;
        assert_eq!(empty.iter().map(RecordBatch::num_rows).sum::<usize>(), 0);
        let joined = ctx
            .sql("SELECT count(*) AS n FROM probe a JOIN probe b ON a.id=b.id")
            .await?
            .collect()
            .await?;
        assert_eq!(
            joined[0]
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::Int64Array>()
                .unwrap()
                .value(0),
            3
        );
        println!(
            "passed: migrated PostgreSQL provider + federation on DF55.1/Arrow59.3: null filter/limit, empty result, same-source join; observed BYTEA->Binary and no original lctx.probe metadata (normalization adapter required)"
        );
    }
    println!(
        "passed: PG{version} pgpq0.12 + SQLx0.9 direct binary COPY: 16-byte IDs, Float32 signed zero, empty/null/literal-null text, nonempty/empty/null arrays with nullable elements, explicit COPY abort and connection reuse"
    );
    Ok(())
}
