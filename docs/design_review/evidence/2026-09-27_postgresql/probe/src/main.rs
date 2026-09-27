use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ImageExt, runners::AsyncRunner}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = std::env::var("LCTX_POSTGRES_TEST_TAG")?;
    let db = Postgres::default().with_fsync_enabled().with_tag(image).start().await?;
    let dsn = format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", db.get_host_port_ipv4(5432).await?);
    let pool = PgPoolOptions::new().max_connections(2).connect(&dsn).await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num").fetch_one(&pool).await?;
    assert_eq!(version.parse::<u32>()? / 10000, 18);
    sqlx::raw_sql("CREATE TABLE values_probe (key bytea PRIMARY KEY, value bytea NOT NULL); CREATE ROLE app LOGIN PASSWORD 'probe-only'; GRANT SELECT, INSERT ON values_probe TO app;").execute(&pool).await?;
    let app = PgPoolOptions::new().max_connections(2).connect(&dsn.replace("postgres:postgres@", "app:probe-only@")).await?;
    let (a, b) = tokio::join!(
        sqlx::query("INSERT INTO values_probe VALUES ($1, $2) ON CONFLICT DO NOTHING").bind([1u8].as_slice()).bind([0u8,0,0,128].as_slice()).execute(&app),
        sqlx::query("INSERT INTO values_probe VALUES ($1, $2) ON CONFLICT DO NOTHING").bind([1u8].as_slice()).bind([0u8,0,0,0].as_slice()).execute(&app)
    );
    assert_eq!(a?.rows_affected() + b?.rows_affected(), 1);
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT value FROM values_probe WHERE key = $1").bind([1u8].as_slice()).fetch_one(&app).await?;
    assert!(bytes == [0,0,0,128] || bytes == [0,0,0,0]);
    assert!(sqlx::query("DELETE FROM values_probe").execute(&app).await.is_err());
    println!("passed: PostgreSQL {version}; SQLx pool; concurrent insert winner; bytea round trip; runtime DELETE denied");
    app.close().await;
    pool.close().await;
    Ok(())
}
