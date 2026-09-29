//! Disposable PostgreSQL 18 databases for tests (feature `testing`), bootstrapped as production is:
//! a non-superuser service owner owns database `lctx`, and each runtime role logs in separately.
use std::path::Path;
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner}};
use crate::OwnerPool;

/// The pinned test image (`specs/postgres-vector-image.txt`).
pub const TEST_IMAGE: &str = include_str!("../../../specs/postgres-vector-image.txt");
const PASSWORD: &str = "postgres";

/// One container with the service owner, the runtime roles and database `lctx`. The container
/// stops when this value is dropped.
pub struct DisposableDatabase {
    _container: ContainerAsync<Postgres>,
    port: u16,
    /// The container superuser on `lctx`, for controls that inspect or tamper with the catalog.
    pub superuser: PgPool,
    /// The verified service owner (`lctx_migrator`).
    pub owner: OwnerPool,
    /// `lctx_importer`, the generation writer.
    pub writer: PgPool,
    /// `lctx_serving`, the generation reader.
    pub reader: PgPool,
    /// `lctx_app`, the retained services' application role.
    pub app: PgPool,
}
impl DisposableDatabase {
    /// Start the pinned image and bootstrap it. Panics when Docker or the image is unavailable:
    /// callers are tests, and a missing prerequisite is reported by the test runner.
    pub async fn start() -> Self {
        let (image, tag) = TEST_IMAGE.trim().split_once(':').expect("image:tag");
        let container = Postgres::default().with_name(image).with_tag(tag).start().await
            .expect("Docker and the pinned PostgreSQL 18 image are required");
        let port = container.get_host_port_ipv4(5432).await.expect("mapped port");
        let url = |role: &str, database: &str| format!("postgres://{role}:{PASSWORD}@127.0.0.1:{port}/{database}");
        let bootstrap = PgPool::connect(&url("postgres", "postgres")).await.expect("superuser");
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE ROLE lctx_migrator LOGIN PASSWORD '{PASSWORD}'; CREATE ROLE lctx_app LOGIN PASSWORD '{PASSWORD}';
            CREATE ROLE lctx_importer LOGIN PASSWORD '{PASSWORD}'; CREATE ROLE lctx_serving LOGIN PASSWORD '{PASSWORD}'")))
            .execute(&bootstrap).await.expect("roles");
        // CREATE DATABASE refuses the implicit transaction of a multi-statement query.
        sqlx::raw_sql("CREATE DATABASE lctx OWNER lctx_migrator").execute(&bootstrap).await.expect("database");
        bootstrap.close().await;
        let superuser = PgPool::connect(&url("postgres", "lctx")).await.expect("superuser on lctx");
        sqlx::raw_sql("REVOKE ALL ON DATABASE lctx FROM PUBLIC; GRANT CONNECT ON DATABASE lctx TO lctx_app, lctx_importer, lctx_serving;
            GRANT TEMP ON DATABASE lctx TO lctx_importer; ALTER ROLE lctx_serving SET default_transaction_read_only = on")
            .execute(&superuser).await.expect("runtime grants");
        let connect = |role: &'static str| PgPool::connect_lazy(&url(role, "lctx")).expect("pool");
        let owner = OwnerPool::verify(PgPool::connect(&url("lctx_migrator", "lctx")).await.expect("owner")).await.expect("verified owner");
        Self { _container: container, port, superuser, owner, writer: connect("lctx_importer"), reader: connect("lctx_serving"), app: connect("lctx_app") }
    }
    /// A connection URL for a role on `lctx`.
    pub fn url(&self, role: &str) -> String { format!("postgres://{role}:{PASSWORD}@127.0.0.1:{}/lctx", self.port) }
    /// Write mode-0600 protected configurations into `dir`: `postgres.json` (application),
    /// `postgres-admin.json` (service owner) and one role file each for the writer and reader.
    pub fn write_configs(&self, dir: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let write = |name: &str, value: serde_json::Value| -> std::io::Result<()> {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
            options.open(dir.join(name))?.write_all(value.to_string().as_bytes())
        };
        write("postgres.json", serde_json::json!({ "application_url": self.url("lctx_app"), "max_connections": 4,
            "acquire_timeout_seconds": 10, "statement_timeout_seconds": 60, "lock_timeout_seconds": 10, "max_receipt_bytes": 1_048_576 }))?;
        write("postgres-admin.json", serde_json::json!({ "migration_url": self.url("lctx_migrator") }))?;
        for (name, role, connections) in [("postgres-importer.json", "importer", 2), ("postgres-serving.json", "serving", 3)] {
            write(name, serde_json::json!({ "format": 1, "role": role, "url": self.url(&format!("lctx_{role}")), "max_connections": connections,
                "provider_connections": 0, "acquire_timeout_seconds": 10, "statement_timeout_seconds": 60, "lock_timeout_seconds": 10 }))?;
        }
        Ok(())
    }
}
