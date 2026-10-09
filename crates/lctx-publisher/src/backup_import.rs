//! Grammar-owned whole import units, aggregated into bounded HTTP requests.
use lctx_model::domain::{
    ModelError,
    resources::{MAX_ROW_BYTES, TRANSFER_BYTES, TRANSFER_ROWS},
};
use lctx_surrealdb::loader::write_failure;
use lctx_surrealdb::surrealdb::types::ToSql;
use std::io::Read;

pub(crate) async fn import(
    client: &lctx_surrealdb::surrealdb::Surreal<
        lctx_surrealdb::surrealdb::engine::remote::http::Client,
    >,
    input: &std::path::Path,
) -> Result<(), ModelError> {
    send(
        client,
        Requests::new(std::fs::File::open(input).map_err(ModelError::codec)?),
    )
    .await
}

async fn send<R: Read>(
    client: &lctx_surrealdb::surrealdb::Surreal<
        lctx_surrealdb::surrealdb::engine::remote::http::Client,
    >,
    mut requests: Requests<R>,
) -> Result<(), ModelError> {
    use std::io::{Seek, Write};
    let mut request = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
    let result = async {
        while let Some(sql) = requests.next_request()? {
            request
                .as_file_mut()
                .set_len(0)
                .map_err(ModelError::codec)?;
            request.rewind().map_err(ModelError::codec)?;
            request
                .write_all(sql.as_bytes())
                .map_err(ModelError::codec)?;
            request.flush().map_err(ModelError::codec)?;
            drop(sql);
            // Import checks the HTTP terminal response and returned statement errors. A failure
            // stops here even if later independent units in this request already had private effects.
            client.import(request.path()).await.map_err(write_failure)?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.cleanup(
        request.path().display().to_string(),
        request
            .close()
            .map_err(|error| ModelError::Cause(Box::new(error))),
    );
    lctx_model::domain::completion::complete(result, completion)
}

const PREFIX: &str = "OPTION IMPORT;\n";

struct Unit {
    sql: String,
    statements: usize,
}

struct Units<R> {
    input: R,
    parser: surrealdb_syn::parser::StatementStream,
    buffer: bytes::BytesMut,
    eof: bool,
    initial_import: bool,
    max_bytes: usize,
}

impl<R: Read> Units<R> {
    fn new(input: R, max_bytes: usize) -> Self {
        Self {
            input,
            parser: surrealdb_syn::parser::StatementStream::new(),
            buffer: bytes::BytesMut::new(),
            eof: false,
            initial_import: false,
            max_bytes,
        }
    }
    fn limit(&self, limit: &'static str, observed: usize) -> ModelError {
        ModelError::Limit {
            owner: "restore-import",
            limit,
            observed,
            bound: self.max_bytes,
        }
    }
    fn statement(&mut self) -> Result<Option<(surrealdb_sql::TopLevelExpr, usize)>, ModelError> {
        loop {
            let before = self.buffer.len();
            let statement = if self.eof {
                self.parser.parse_complete(&mut self.buffer)
            } else {
                self.parser.parse_partial(&mut self.buffer)
            }
            .map_err(|error| ModelError::codec(format!("restore dump parse: {error}")))?;
            if let Some(statement) = statement {
                return Ok(Some((statement, before - self.buffer.len())));
            }
            if self.eof {
                return Ok(None);
            }
            let remaining = self.max_bytes.saturating_sub(self.buffer.len());
            if remaining == 0 {
                return Err(self.limit("statement bytes", self.buffer.len() + 1));
            }
            // The pinned parser reparses unfinished input: retain geometric read growth.
            let amount = self
                .buffer
                .len()
                .clamp(65536, TRANSFER_BYTES)
                .min(remaining);
            let mut window = [0u8; 65536];
            let mut added = 0;
            while added < amount {
                let count = self
                    .input
                    .read(&mut window[..(amount - added).min(65536)])
                    .map_err(ModelError::codec)?;
                if count == 0 {
                    self.eof = true;
                    break;
                }
                self.buffer.extend_from_slice(&window[..count]);
                added += count;
            }
        }
    }
    fn next_unit(&mut self) -> Result<Option<Unit>, ModelError> {
        use surrealdb_sql::TopLevelExpr;
        let mut sql = String::new();
        let mut statements = 0;
        let mut transaction = false;
        let mut source_bytes = 0usize;
        loop {
            let Some((statement, bytes)) = self.statement()? else {
                if !self.initial_import {
                    return Err(ModelError::Schema("dump initial OPTION IMPORT"));
                }
                if transaction {
                    return Err(ModelError::Schema("dump incomplete transaction"));
                }
                return Ok(None);
            };
            if !self.initial_import {
                if !matches!(&statement, TopLevelExpr::Option(option) if option.name.as_str() == "IMPORT" && option.what)
                {
                    return Err(ModelError::Schema("dump initial OPTION IMPORT"));
                }
                self.initial_import = true;
                continue;
            }
            if matches!(&statement, TopLevelExpr::Option(_)) {
                return Err(ModelError::Schema(
                    "dump duplicate or noninitial import option",
                ));
            }
            let terminal = matches!(&statement, TopLevelExpr::Commit | TopLevelExpr::Cancel);
            match &statement {
                TopLevelExpr::Begin if transaction => {
                    return Err(ModelError::Schema("dump nested transaction"));
                }
                TopLevelExpr::Begin => transaction = true,
                TopLevelExpr::Commit | TopLevelExpr::Cancel if !transaction => {
                    return Err(ModelError::Schema("dump transaction end without BEGIN"));
                }
                _ => {}
            }
            statements += 1;
            if statements > TRANSFER_ROWS {
                return Err(ModelError::Limit {
                    owner: "restore-import",
                    limit: "transaction statements",
                    observed: statements,
                    bound: TRANSFER_ROWS,
                });
            }
            source_bytes = source_bytes.saturating_add(bytes);
            if source_bytes > self.max_bytes {
                return Err(self.limit("transaction bytes", source_bytes));
            }
            // Serialize one AST at a time; do not retain a transaction AST beside its SQL.
            sql.push_str(&statement.to_sql());
            sql.push_str(";\n");
            let request_bytes = PREFIX.len().saturating_add(sql.len());
            if request_bytes > self.max_bytes {
                return Err(self.limit("request bytes", request_bytes));
            }
            if !transaction || terminal {
                return Ok(Some(Unit { sql, statements }));
            }
        }
    }
}

pub(crate) struct Requests<R> {
    units: Units<R>,
    lookahead: Option<Unit>,
    target_bytes: usize,
    max_statements: usize,
}
impl<R: Read> Requests<R> {
    pub(crate) fn new(input: R) -> Self {
        Self::bounded(input, TRANSFER_BYTES, TRANSFER_ROWS, MAX_ROW_BYTES)
    }
    fn bounded(input: R, target_bytes: usize, max_statements: usize, max_bytes: usize) -> Self {
        Self {
            units: Units::new(input, max_bytes),
            lookahead: None,
            target_bytes,
            max_statements,
        }
    }
    pub(crate) fn next_request(&mut self) -> Result<Option<String>, ModelError> {
        let mut request = String::from(PREFIX);
        let mut statements = 0;
        loop {
            let unit = match self.lookahead.take() {
                Some(unit) => Some(unit),
                None => self.units.next_unit()?,
            };
            let Some(unit) = unit else {
                break;
            };
            if unit.statements > self.max_statements {
                return Err(ModelError::Limit {
                    owner: "restore-import",
                    limit: "request statements",
                    observed: unit.statements,
                    bound: self.max_statements,
                });
            }
            if statements > 0
                && (request.len().saturating_add(unit.sql.len()) > self.target_bytes
                    || statements + unit.statements > self.max_statements)
            {
                self.lookahead = Some(unit);
                break;
            }
            statements += unit.statements;
            request.push_str(&unit.sql);
            // The unit's complete prefixed serialization already passed the hard bound.
            // An oversize whole unit travels alone, without reading the next unit.
            if request.len() >= self.target_bytes || statements == self.max_statements {
                break;
            }
        }
        Ok((statements > 0).then_some(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn collect(mut requests: Requests<&[u8]>) -> Result<Vec<String>, ModelError> {
        let mut result = Vec::new();
        while let Some(request) = requests.next_request()? {
            result.push(request);
        }
        Ok(result)
    }
    #[test]
    fn restore_import_units_preserve_language_and_transaction_boundaries() {
        struct Chunks<'a>(&'a [u8]);
        impl Read for Chunks<'_> {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let n = out.len().min(7).min(self.0.len());
                out[..n].copy_from_slice(&self.0[..n]);
                self.0 = &self.0[n..];
                Ok(n)
            }
        }
        let dump = format!(
            "-- dump\nOPTION IMPORT; DEFINE TABLE example SCHEMALESS; DEFINE FUNCTION fn::sentinel() {{ LET $x = '{}é;still literal'; RETURN $x; }}; BEGIN; INSERT INTO example [{{id: example:a, text: 'value;with;semicolons'}}]; COMMIT; BEGIN; INSERT INTO example [{{id: example:b}}]; CANCEL; DEFINE TABLE final SCHEMALESS; -- end",
            "x".repeat(65536)
        );
        let mut units = Units::new(Chunks(dump.as_bytes()), 128 * 1024);
        let mut found = Vec::new();
        while let Some(unit) = units.next_unit().unwrap() {
            found.push(unit);
        }
        assert_eq!(
            found.iter().map(|u| u.statements).collect::<Vec<_>>(),
            [1, 1, 3, 3, 1]
        );
        assert!(found[1].sql.contains("é;still literal"));
        assert!(found[2].sql.starts_with("BEGIN;\n") && found[2].sql.ends_with("COMMIT;\n"));
        assert!(found[3].sql.ends_with("CANCEL;\n"));
        let requests = collect(Requests::new(dump.as_bytes())).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].matches(PREFIX).count(), 1);
    }
    #[test]
    fn restore_import_units_refuse_bad_tails_modes_and_oversized_transactions() {
        for dump in [
            "",
            "RETURN 1;",
            "OPTION IMPORT = FALSE;",
            "OPTION IMPORT; OPTION IMPORT;",
            "OPTION IMPORT; BEGIN; RETURN 1;",
            "OPTION IMPORT; BEGIN; BEGIN; COMMIT;",
            "OPTION IMPORT; COMMIT;",
            "OPTION IMPORT; CANCEL;",
            "OPTION IMPORT; RETURN 'truncated",
            "OPTION IMPORT; RETURN 1; INSERT INTO example [{broken: }];",
        ] {
            assert!(
                collect(Requests::new(dump.as_bytes())).is_err(),
                "accepted {dump}"
            );
        }
        let maximum = format!(
            "OPTION IMPORT; BEGIN; {} COMMIT;",
            "RETURN 1;".repeat(TRANSFER_ROWS - 2)
        );
        let accepted = collect(Requests::new(maximum.as_bytes())).unwrap();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].matches("RETURN 1;").count(), TRANSFER_ROWS - 2);
        let dump = format!(
            "OPTION IMPORT; BEGIN; {} COMMIT;",
            "RETURN 1;".repeat(TRANSFER_ROWS - 1)
        );
        assert!(matches!(
            collect(Requests::new(dump.as_bytes())),
            Err(ModelError::Limit {
                limit: "transaction statements",
                ..
            })
        ));
        let dump = format!("OPTION IMPORT; RETURN '{}';", "x".repeat(300));
        assert!(collect(Requests::bounded(dump.as_bytes(), 128, TRANSFER_ROWS, 256)).is_err());
    }
    #[test]
    fn restore_requests_count_controls_and_preserve_order() {
        let dump = b"OPTION IMPORT; RETURN 1; BEGIN; RETURN 2; COMMIT; RETURN 3; RETURN 4;";
        let requests = collect(Requests::bounded(dump, 4096, 4, 8192)).unwrap();
        assert_eq!(
            requests,
            [
                "OPTION IMPORT;\nRETURN 1;\nBEGIN;\nRETURN 2;\nCOMMIT;\n",
                "OPTION IMPORT;\nRETURN 3;\nRETURN 4;\n"
            ]
        );
        let dump = format!("OPTION IMPORT; {}", "RETURN 1;".repeat(TRANSFER_ROWS + 1));
        let requests = collect(Requests::new(dump.as_bytes())).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].matches("RETURN 1;").count(), TRANSFER_ROWS);
        assert_eq!(requests[1].matches("RETURN 1;").count(), 1);
    }
    #[test]
    fn restore_requests_include_prefix_and_separators_in_exact_byte_edges() {
        let dump = b"OPTION IMPORT; RETURN 1; RETURN 2;";
        let one = "OPTION IMPORT;\nRETURN 1;\n".len();
        let two = one + "RETURN 2;\n".len();
        assert_eq!(
            collect(Requests::bounded(dump, two, TRANSFER_ROWS, two))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            collect(Requests::bounded(dump, two - 1, TRANSFER_ROWS, two))
                .unwrap()
                .len(),
            2
        );
        let dump = b"OPTION IMPORT; RETURN 1;";
        assert_eq!(
            collect(Requests::bounded(dump, one - 1, TRANSFER_ROWS, one)).unwrap(),
            ["OPTION IMPORT;\nRETURN 1;\n"]
        );
        assert!(collect(Requests::bounded(dump, one - 1, TRANSFER_ROWS, one - 1)).is_err());
        assert!(
            collect(Requests::new(b"OPTION IMPORT;".as_slice()))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn restore_requests_distinguish_returned_failure_from_uncertain_transport() {
        use lctx_surrealdb::surrealdb::Error;
        let known = write_failure(Error::query("checked statement refusal".into(), None));
        assert!(known.permits_storage_cleanup());
        assert!(
            matches!(known.primary(), Some(ModelError::Cause(cause)) if cause.downcast_ref::<Error>().is_some_and(Error::is_query))
        );
        let unknown = write_failure(Error::internal("SDK HTTP body read interrupted".into()));
        assert!(!unknown.permits_storage_cleanup());
        let ModelError::Completion(outcome) = unknown else {
            panic!("write certainty")
        };
        assert_eq!(
            outcome.completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert_eq!(
            outcome.completion.remote,
            lctx_model::domain::completion::RemoteState::Unknown
        );
    }

    #[tokio::test]
    async fn native_failed_import_keeps_later_effects_private_and_submits_no_following_request() {
        use lctx_surrealdb::surrealdb::types::Value;
        let config = lctx_surrealdb::RuntimeConfig::read(&std::path::PathBuf::from(
            std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native fixture"),
        ))
        .unwrap();
        for dump in [
            "OPTION IMPORT; DEFINE TABLE import_probe SCHEMALESS; THROW 'import-sentinel'; CREATE import_probe:within; CREATE import_probe:following;",
            "OPTION IMPORT; DEFINE TABLE import_probe SCHEMALESS; CREATE import_probe:within; THROW 'import-sentinel'; CREATE import_probe:following;",
            "OPTION IMPORT; DEFINE TABLE import_probe SCHEMALESS; CREATE import_probe:before; RETURN 1; THROW 'import-sentinel'; CREATE import_probe:within; RETURN 1; CREATE import_probe:following;",
        ] {
            let staging = crate::begin(&config).await.unwrap();
            let client = crate::backup::http(&config, staging.database.as_str())
                .await
                .unwrap();
            let result = send(
                &client,
                Requests::bounded(dump.as_bytes(), TRANSFER_BYTES, 3, MAX_ROW_BYTES),
            )
            .await;
            let failure = result.unwrap_err();
            assert!(failure.to_string().contains("import-sentinel"));
            assert!(
                failure.permits_storage_cleanup(),
                "checked refusal is distinct from lost acknowledgement"
            );
            let mut observed = client.query("RETURN record::exists(import_probe:within); RETURN record::exists(import_probe:following);").await.unwrap().check().unwrap();
            assert!(
                observed.take::<Option<bool>>(0).unwrap().unwrap(),
                "later independent unit ran inside failed request"
            );
            assert!(
                !observed.take::<Option<bool>>(1).unwrap().unwrap(),
                "following request must never execute"
            );
            crate::abandon(&staging).await.unwrap();
            let info = client
                .query("INFO FOR NS;")
                .await
                .unwrap()
                .check()
                .unwrap()
                .take::<Option<Value>>(0)
                .unwrap()
                .unwrap();
            let Value::Object(info) = info else {
                panic!("namespace info")
            };
            let Some(Value::Object(databases)) = info.get("databases") else {
                panic!("namespace databases")
            };
            assert!(
                !databases.contains_key(staging.database.as_str()),
                "failed private staging was removed"
            );
            client.invalidate().await.unwrap();
        }
    }
}
