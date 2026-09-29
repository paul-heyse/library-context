//! Database bootstrap shared by tests, `just pg-dev` and the operator's provisioning (cutover plan
//! WP0.6/WP1.5): the four roles and one database's grants and extension, from one SQL source.

use sqlx::PgPool;

use crate::Error;

/// The roles every lctx database uses (cutover plan §4.1 D5): owner, application (cache), writer
/// and reader.
pub const ROLES: [&str; 4] = ["lctx_migrator", "lctx_app", "lctx_importer", "lctx_serving"];

/// One database's bootstrap, run by a superuser after [`ROLES`] exist.
pub const DATABASE_SQL: &str = include_str!("../sql/database.sql");

/// Create the four roles with one password (disposable clusters only), then bootstrap the
/// connected database. `admin` must be a superuser connection.
pub async fn disposable_cluster(admin: &PgPool, password: &str) -> Result<(), Error> {
    if !password.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(Error::Config("disposable passwords are alphanumeric"));
    }
    for role in ROLES {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "CREATE ROLE {role} LOGIN PASSWORD '{password}' \
             NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS"
        )))
        .execute(admin)
        .await?;
    }
    sqlx::raw_sql(DATABASE_SQL).execute(admin).await?;
    Ok(())
}
