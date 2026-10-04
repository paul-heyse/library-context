//! Shared connection mechanics; role, timeout and read policy remain with their owners.
use crate::Error;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};
use std::str::FromStr;

pub(crate) fn parse(url: &str, invalid: &'static str) -> Result<PgConnectOptions, Error> {
    let options = PgConnectOptions::from_str(url).map_err(|_| Error::Config(invalid))?;
    let host = options.get_host();
    let local =
        host.starts_with('/') || matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]");
    if !local && !matches!(options.get_ssl_mode(), PgSslMode::VerifyFull) {
        return Err(Error::Config(
            "remote PostgreSQL requires sslmode=verify-full",
        ));
    }
    Ok(options)
}
pub(crate) fn session(
    options: PgConnectOptions,
    application: &str,
    statement: u64,
    lock: u64,
    search_path: &str,
) -> PgConnectOptions {
    options
        .disable_statement_logging()
        .application_name(application)
        .options([
            ("statement_timeout", format!("{statement}s")),
            ("lock_timeout", format!("{lock}s")),
            ("idle_in_transaction_session_timeout", "30s".to_owned()),
            ("search_path", search_path.to_owned()),
        ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_tls_and_local_role_options_preserve_policy() {
        for host in ["localhost", "127.0.0.1", "[::1]"] {
            assert!(parse(&format!("postgres://lctx_app@{host}/lctx"), "invalid").is_ok());
        }
        for mode in ["disable", "allow", "prefer", "require", "verify-ca"] {
            assert!(matches!(
                parse(
                    &format!("postgres://lctx_app@remote.invalid/lctx?sslmode={mode}"),
                    "invalid"
                ),
                Err(Error::Config(
                    "remote PostgreSQL requires sslmode=verify-full"
                ))
            ));
        }
        let options = parse(
            "postgres://lctx_serving@remote.invalid/lctx?sslmode=verify-full",
            "invalid",
        )
        .unwrap();
        let options = session(options, "lctx-serving", 30, 3, "pg_catalog,lctx_ext");
        assert_eq!(options.get_username(), "lctx_serving");
        assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
    }
}
