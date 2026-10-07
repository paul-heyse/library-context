//! Trusted local logical transport. Imported metadata never becomes a serving realization.
use crate::{PrivatePublication, abandon, begin, seal};
use lctx_model::domain::{
    Infrastructure, ModelError,
    graph::{Assertion, Entity, Manifest, Target, semantic_contract},
    serving::SnapshotHandle,
};
use lctx_surrealdb::surrealdb::{
    Surreal,
    engine::remote::http::{Client, Http},
    opt::auth::Root,
    types::{Bytes, RecordId, SurrealValue, ToSql, Variables},
};
use lctx_surrealdb::{NativeReader, RuntimeConfig};
use std::{
    io::{Read, Seek, Write},
    path::Path,
};

async fn http(config: &RuntimeConfig, database: &str) -> Result<Surreal<Client>, ModelError> {
    // The managed server exposes HTTP and gRPC on the same configured authority.
    let authority = config
        .endpoint
        .strip_prefix("grpc://")
        .ok_or(ModelError::Schema("managed gRPC endpoint"))?;
    let client = Surreal::new::<Http>(authority)
        .await
        .map_err(ModelError::codec)?;
    client
        .signin(Root {
            username: config.username.clone(),
            password: config.password.clone(),
        })
        .await
        .map_err(ModelError::codec)?;
    client
        .use_ns(config.namespace.as_str())
        .use_db(database)
        .await
        .map_err(ModelError::codec)?;
    Ok(client)
}

/// Write a logical dump after the gRPC file export consumes successful terminal completion.
/// Database users/access credentials and historical versions are excluded.
/// A parent-directory sync failure after publication leaves the dump in place and reports
/// uncertain durability; callers must inspect the destination before retrying.
pub async fn backup(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    output: &Path,
) -> Result<(), ModelError> {
    if handle.database.namespace != config.namespace {
        return Err(ModelError::Conflict("backup namespace"));
    }
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await?;
    viewer
        .client()
        .invalidate()
        .await
        .map_err(ModelError::codec)?;
    drop(viewer);
    if output.exists() {
        return Err(ModelError::Conflict("backup destination already exists"));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let staged = tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
    let client = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        handle.database.database.as_str(),
    )
    .await?;
    // Transport only canonical families, originals and their native role arcs. Derived search
    // is rebuilt on restore; its optional array fields have a 3.3 export/import DDL mismatch.
    let tables = [
        "entity",
        "assertion",
        "participant",
        "reference",
        "external",
        "original",
        "original_chunk",
        "publication",
        "compiler_contribution",
        "compiler_membership",
        "compiler_view",
        "compiler_record",
        "compiler_binding",
        "compiler_alias",
    ]
    .map(str::to_owned)
    .to_vec();
    // SDK 3.3's gRPC file route awaits the export copy and requires a terminal trailer
    // with the received byte count. The server sends it only after engine success.
    // Its optional BLAKE3 trailer digest is not verified by the SDK.
    let result = client
        .export(staged.path())
        .with_config()
        .users(false)
        .accesses(false)
        .versions(false)
        .params(false)
        .functions(false)
        .analyzers(false)
        .apis(false)
        .buckets(false)
        .modules(false)
        .configs(false)
        .tables(tables)
        .await
        .map_err(ModelError::codec);
    let drained = client.invalidate().await.map_err(ModelError::codec);
    drop(client);
    complete_backup(staged, output, result, drained, |parent| {
        std::fs::File::open(parent)?.sync_all()
    })
}

fn complete_backup(
    staged: tempfile::NamedTempFile,
    output: &Path,
    result: Result<(), ModelError>,
    drained: Result<(), ModelError>,
    sync_parent: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), ModelError> {
    // Cleanup errors must not replace a failed export. Until persist_noclobber succeeds,
    // the NamedTempFile owns and removes every provisional dump on any failure.
    result?;
    drained?;
    staged.as_file().sync_all().map_err(ModelError::codec)?;
    staged
        .persist_noclobber(output)
        .map_err(|error| ModelError::codec(error.error))?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // Destination publication has committed. Never remove it or claim rollback when
    // the directory cannot be opened/synchronized: the verified dump may already exist.
    sync_parent(parent).map_err(|error| {
        ModelError::infrastructure(
            Infrastructure::Unconfirmed,
            format!(
                "backup destination {} was published but its durability is uncertain: parent directory synchronization failed: {error}",
                output.display()
            ),
        )
    })
}

/// Import a trusted current-format local dump privately, then copy canonical graph, exact completed state and
/// original bytes into a fresh realization. No imported marker, permission or function is served.
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    native_definitions: &str,
) -> Result<SnapshotHandle, ModelError> {
    let staging = begin(config).await?;
    let imported = import(config, input, &staging).await.map_err(|error| restore_phase("staging admission",error));
    let result = match imported {
        Err(error) => Err(error),
        Ok(manifest) => {
            let fresh = match begin(config).await {
                Ok(fresh) => fresh,
                Err(error) => {
                    abandon(&staging).await?;
                    return Err(error);
                }
            };
            match copy(config, &staging, &fresh, &manifest, native_definitions).await.map_err(|error| restore_phase("canonical copy",error)) {
                Err(error) => {
                    cleanup_result(Err(error), abandon(&fresh).await)
                }
                Ok(()) => match seal(&fresh, &manifest, config, native_definitions).await {
                    Ok(handle) => Ok(handle),
                    Err(error) => {
                        let error=restore_phase("final sealing",error);
                        cleanup_result(Err(error), abandon(&fresh).await)
                    }
                },
            }
        }
    };
    cleanup_result(result, abandon(&staging).await)
}

fn cleanup_result<T>(result:Result<T,ModelError>,cleanup:Result<(),ModelError>)->Result<T,ModelError>{
    match (result,cleanup){
        (result,Ok(()))=>result,
        (Ok(_),Err(cleanup))=>Err(cleanup),
        (Err(primary),Err(cleanup))=>Err(ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed,format!("restore failed: {primary}; {cleanup}"))),
    }
}

fn restore_phase(phase:&'static str,error:ModelError)->ModelError {
    match error {
        ModelError::Codec(detail)=>ModelError::Codec(format!("restore {phase}: {detail}")),
        ModelError::Invalid(detail)=>ModelError::Invalid(format!("restore {phase}: {detail}")),
        ModelError::Frontier(detail)=>ModelError::Frontier(format!("restore {phase}: {detail}")),
        ModelError::Infrastructure {class,detail}=>ModelError::Infrastructure {class,detail:format!("restore {phase}: {detail}")},
        other=>other,
    }
}

async fn import(
    config: &RuntimeConfig,
    input: &Path,
    staging: &PrivatePublication,
) -> Result<Manifest, ModelError> {
    let client = http(config, staging.database.as_str()).await?;
    let imported = import_units(&client, input).await;
    let drained = client.invalidate().await.map_err(ModelError::codec);
    drop(client);
    imported?;
    drained?;
    let reader = NativeReader::private(staging.loader.shared_client());
    let bytes: Vec<Bytes> = reader
        .query(
            "SELECT VALUE manifest FROM publication:current",
            Variables::new(),
        )
        .await?;
    let manifest: Manifest = serde_json::from_slice(
        bytes
            .first()
            .filter(|_| bytes.len() == 1)
            .ok_or(ModelError::Schema("logical dump publication manifest"))?,
    )
    .map_err(ModelError::codec)?;
    manifest.validate()?;
    if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
        return Err(ModelError::Conflict("restore semantic contract"));
    }
    staging.loader.reconcile(&manifest).await.map_err(|error|restore_phase("staging graph reconciliation",error))?;
    let native=lctx_surrealdb::compiler::NativeCompilerStore::from_existing(staging.loader.shared_client(),config.namespace.clone(),staging.database.clone());
    if native.completed_state().await.map_err(|error|restore_phase("staging completed state",error))? != manifest.completed_state {return Err(ModelError::Conflict("restore completed state"));}
    Ok(manifest)
}

const IMPORT_PREFIX: &str = "OPTION IMPORT;\n";

/// The pinned parser owns lexical boundaries, including semicolons inside literals and blocks.
/// Only the unfinished statement and (when present) one complete transaction are retained.
struct ImportUnits<R> {
    input: R,
    parser: surrealdb_syn::parser::StatementStream,
    buffer: bytes::BytesMut,
    eof: bool,
    initial_import: bool,
    transaction: Vec<surrealdb_sql::TopLevelExpr>,
    transaction_bytes: usize,
    max_bytes: usize,
}

impl<R: Read> ImportUnits<R> {
    fn new(input: R, max_bytes: usize) -> Self {
        Self {
            input,
            parser: surrealdb_syn::parser::StatementStream::new(),
            buffer: bytes::BytesMut::new(),
            eof: false,
            initial_import: false,
            transaction: Vec::new(),
            transaction_bytes: 0,
            max_bytes,
        }
    }

    fn limit(&self, limit: &'static str, observed: usize) -> ModelError {
        ModelError::Limit { owner: "restore-import", limit, observed, bound: self.max_bytes }
    }

    fn statement(&mut self) -> Result<Option<(surrealdb_sql::TopLevelExpr, usize)>, ModelError> {
        loop {
            let before = self.buffer.len();
            let statement = if self.eof {
                self.parser.parse_complete(&mut self.buffer)
            } else {
                self.parser.parse_partial(&mut self.buffer)
            }.map_err(|error| ModelError::codec(format!("restore dump parse: {error}")))?;
            if let Some(statement) = statement {
                return Ok(Some((statement, before - self.buffer.len())));
            }
            if self.eof { return Ok(None); }
            let remaining = self.max_bytes.saturating_sub(self.buffer.len());
            if remaining == 0 { return Err(self.limit("statement bytes", self.buffer.len() + 1)); }
            // StatementStream reparses incomplete input. Grow geometrically up to the shared
            // transfer window instead of reparsing a large statement after every tiny read.
            let amount = self.buffer.len().max(65536)
                .min(lctx_model::domain::resources::TRANSFER_BYTES).min(remaining);
            let mut window = [0u8; 65536];
            let mut added = 0;
            while added < amount {
                let count = self.input.read(&mut window[..(amount - added).min(65536)])
                    .map_err(ModelError::codec)?;
                if count == 0 { self.eof = true; break; }
                self.buffer.extend_from_slice(&window[..count]);
                added += count;
            }
        }
    }

    fn next_request(&mut self) -> Result<Option<String>, ModelError> {
        use surrealdb_sql::{Ast, TopLevelExpr};
        loop {
            let Some((statement, bytes)) = self.statement()? else {
                if !self.initial_import { return Err(ModelError::Schema("dump initial OPTION IMPORT")); }
                if !self.transaction.is_empty() { return Err(ModelError::Schema("dump incomplete transaction")); }
                return Ok(None);
            };
            if !self.initial_import {
                if !matches!(&statement, TopLevelExpr::Option(option)
                    if option.name.as_str() == "IMPORT" && option.what) {
                    return Err(ModelError::Schema("dump initial OPTION IMPORT"));
                }
                self.initial_import = true;
                continue;
            }
            // Request-local modes cannot be silently lost or changed across requests.
            if matches!(&statement, TopLevelExpr::Option(_)) {
                return Err(ModelError::Schema("dump duplicate or noninitial import option"));
            }
            match &statement {
                TopLevelExpr::Begin if !self.transaction.is_empty() =>
                    return Err(ModelError::Schema("dump nested transaction")),
                TopLevelExpr::Commit | TopLevelExpr::Cancel if self.transaction.is_empty() =>
                    return Err(ModelError::Schema("dump transaction end without BEGIN")),
                _ => {}
            }
            if !self.transaction.is_empty() || matches!(&statement, TopLevelExpr::Begin) {
                self.transaction_bytes = self.transaction_bytes.saturating_add(bytes);
                if self.transaction_bytes > self.max_bytes {
                    return Err(self.limit("transaction bytes", self.transaction_bytes));
                }
                if self.transaction.len() >= lctx_model::domain::resources::TRANSFER_ROWS {
                    return Err(ModelError::Limit { owner: "restore-import", limit: "transaction statements",
                        observed: self.transaction.len() + 1, bound: lctx_model::domain::resources::TRANSFER_ROWS });
                }
                let terminal = matches!(&statement, TopLevelExpr::Commit | TopLevelExpr::Cancel);
                self.transaction.push(statement);
                if !terminal { continue; }
            } else {
                self.transaction.push(statement);
            }
            let mut units = Ast { expressions: std::mem::take(&mut self.transaction) }.into_execution_units();
            self.transaction_bytes = 0;
            if units.len() != 1 { return Err(ModelError::Schema("dump ordered execution unit")); }
            let unit = units.pop().ok_or(ModelError::Schema("dump empty execution unit"))?;
            let request = format!("{IMPORT_PREFIX}{}\n", unit.to_sql());
            if request.len() > self.max_bytes { return Err(self.limit("request bytes", request.len())); }
            return Ok(Some(request));
        }
    }
}

async fn import_units(client: &Surreal<Client>, input: &Path) -> Result<(), ModelError> {
    let mut units = ImportUnits::new(std::fs::File::open(input).map_err(ModelError::codec)?,
        lctx_model::domain::resources::MAX_ROW_BYTES);
    let mut request = tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
    while let Some(sql) = units.next_request()? {
        request.as_file_mut().set_len(0).map_err(ModelError::codec)?;
        request.rewind().map_err(ModelError::codec)?;
        request.write_all(sql.as_bytes()).map_err(ModelError::codec)?;
        request.flush().map_err(ModelError::codec)?;
        drop(sql);
        // The HTTP SDK consumes terminal completion and every statement error before the next
        // unit starts. Do not use /sql, which has different import execution semantics.
        client.import(request.path()).await
            .map_err(|error| ModelError::codec(format!("restore HTTP dump import: {error}")))?;
    }
    Ok(())
}

async fn copy(
    config: &RuntimeConfig,
    staging: &PrivatePublication,
    fresh: &PrivatePublication,
    manifest: &Manifest,
    native_definitions: &str,
) -> Result<(), ModelError> {
    fresh.loader.install(native_definitions).await?;
    let native=lctx_surrealdb::compiler::NativeCompilerStore::from_existing(fresh.loader.shared_client(),config.namespace.clone(),fresh.database.clone());
    native.install_state_schema().await?;
    let runtime = cpg_core::workspace::Workspace::new(
        std::sync::Arc::new(lctx_model::domain::model()?),
        cpg_core::workspace::WorkspaceOptions::default(),
        native.clone(),
    )?;
    let result=async {
    let state=tempfile::NamedTempFile::new().map_err(ModelError::codec)?;
    let source_state=lctx_surrealdb::compiler::NativeCompilerStore::from_existing(staging.loader.shared_client(),config.namespace.clone(),staging.database.clone());
    if source_state.export_state(state.path()).await.map_err(|error|restore_phase("completed-state export",error))? != manifest.completed_state {return Err(ModelError::Conflict("restore source state"));}

    // Two passes: materialize every endpoint before constructing native role/reference arcs.
    for references in [false, true] {
        let mut after = RecordId::new("entity", "");
        loop {
            let (rows, last) = page::<Entity>(staging, "entity", after).await?;
            if rows.is_empty() {
                break;
            }
            if references {
                fresh.loader.entity_references(&rows).await?
            } else {
                fresh.loader.entities(&rows).await?
            }
            after = last;
        }
        let mut after = RecordId::new("assertion", "");
        loop {
            let (rows, last) = page::<Assertion>(staging, "assertion", after).await?;
            if rows.is_empty() {
                break;
            }
            if references {
                fresh.loader.assertion_references(&rows).await?
            } else {
                fresh.loader.assertions(&rows).await?
            }
            after = last;
        }
    }
    let source = NativeReader::private(staging.loader.shared_client());
    for original in &manifest.originals {
        let mut file = tempfile::tempfile().map_err(ModelError::codec)?;
        let mut start = 0;
        while start < original.byte_len {
            let mut ranges=Vec::new();
            for _ in 0..4 {
                if start>=original.byte_len {break;}
                let length=(original.byte_len-start).min(65536) as usize;
                ranges.push((original.source,start,length));start+=length as u64;
            }
            for bytes in source.original_bytes_batch(&ranges).await? {
                file.write_all(&bytes).map_err(ModelError::codec)?;
            }
        }
        file.rewind().map_err(ModelError::codec)?;
        let mut bindings = Variables::new();
        bindings.insert(
            "source",
            lctx_surrealdb::reader::target_id(Target::Entity(original.source)),
        );
        let headers: Vec<Bytes> = source
            .query(
                "SELECT VALUE canonical FROM entity WHERE id=$source",
                bindings,
            )
            .await?;
        let entity: Entity = serde_json::from_slice(
            headers
                .first()
                .filter(|_| headers.len() == 1)
                .ok_or(ModelError::Schema("restored original source"))?,
        )
        .map_err(ModelError::codec)?;
        let Entity::Source(header) = entity else {
            return Err(ModelError::Schema("restored original source"));
        };
        if header.content!=original.content || u64::try_from(header.byte_len).map_err(ModelError::codec)?!=original.byte_len {
            return Err(ModelError::Conflict("restored original source metadata"));
        }
        file.rewind().map_err(ModelError::codec)?;
        fresh
            .loader
            .original_stream(
                original.source.0,
                original.content,
                original.byte_len,
                &mut file,
            )
            .await?;
    }
    native.import_state(state.path(),&manifest.completed_state).await.map_err(|error|restore_phase("completed-state import",error))?;
    runtime.restore(manifest.profile).await?;
    cpg_core::artifact::verify_restored(&runtime,manifest).await.map_err(|error|restore_phase("restored artifact admission",error))?;
    crate::materialize_search(&fresh.loader).await.map_err(|error|restore_phase("search reconstruction",error))?;
    fresh.loader.reconcile(manifest).await.map_err(|error|restore_phase("final graph reconciliation",error))
    }.await;
    runtime.drain().await?;
    if result.is_err(){native.fail();}else{native.end_writes().await?;}
    result
}

async fn page<T: serde::de::DeserializeOwned>(
    staging: &PrivatePublication,
    table: &str,
    after: RecordId,
) -> Result<(Vec<T>, RecordId), ModelError> {
    let mut vars = Variables::new();
    vars.insert("after", after.clone());
    let mut response = staging
        .loader
        .client()
        .query(format!(
            "SELECT id,canonical FROM {table} WHERE id>$after ORDER BY id LIMIT 128"
        ))
        .bind(vars)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    #[derive(lctx_surrealdb::surrealdb::types::SurrealValue)]
    #[surreal(crate = "lctx_surrealdb::surrealdb::types")]
    struct Row {
        id: RecordId,
        canonical: Bytes,
    }
    let rows: Vec<Row> = response.take(0).map_err(ModelError::codec)?;
    let last = rows.last().map(|r| r.id.clone()).unwrap_or(after);
    let values = rows
        .into_iter()
        .map(|r| serde_json::from_slice(&r.canonical).map_err(ModelError::codec))
        .collect::<Result<_, _>>()?;
    Ok((values, last))
}

/// Retire an explicitly named, unselected published database after the operator has stopped
/// every known reader. There is no automatic lease/history or selected-handle replacement.
pub async fn retire(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    readers_stopped: bool,
) -> Result<(), ModelError> {
    if !readers_stopped {
        return Err(ModelError::Invalid(
            "retirement requires stopped readers".into(),
        ));
    }
    if handle.database.namespace != config.namespace {
        return Err(ModelError::Conflict("retirement namespace"));
    }
    let _guard = config.lock_selection().await?;
    let database = handle.database.database.as_str();
    let suffix = database
        .strip_prefix("snapshot_")
        .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or(ModelError::Schema("published private database name"))?;
    if config.selection.try_exists().map_err(ModelError::codec)?
        && config.selected()?.database == handle.database
    {
        return Err(ModelError::Conflict("cannot retire selected snapshot"));
    }
    let viewer = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await?;
    viewer
        .client()
        .invalidate()
        .await
        .map_err(ModelError::codec)?;
    drop(viewer);
    let client = lctx_surrealdb::reader::connect(
        &config.endpoint,
        &config.root_credentials(),
        config.namespace.as_str(),
        database,
    )
    .await?;
    client
        .query(format!("REMOVE DATABASE `snapshot_{suffix}`"))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client.invalidate().await.map_err(ModelError::codec)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/fixtures/grpc_export.rs"]
mod grpc_export_fixture;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_import_units_preserve_language_and_transaction_boundaries() {
        // Chunked input and a literal crossing the parser's first read window exercise partial
        // parsing. The library must own semicolons inside strings and function bodies.
        struct Chunks<'a>(&'a [u8]);
        impl Read for Chunks<'_> {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let count = out.len().min(7).min(self.0.len());
                out[..count].copy_from_slice(&self.0[..count]);
                self.0 = &self.0[count..];
                Ok(count)
            }
        }
        let dump = format!("-- canonical dump\nOPTION IMPORT;\n\
            DEFINE TABLE example SCHEMALESS;\n\
            DEFINE FUNCTION fn::sentinel() {{ LET $x = '{}é;still literal'; RETURN $x; }};\n\
            BEGIN TRANSACTION;\n\
            INSERT INTO example [{{ id: example:a, text: 'value;with;semicolons' }}];\n\
            COMMIT TRANSACTION;\n\
            BEGIN; INSERT INTO example [{{ id: example:b }}]; CANCEL;\n\
            DEFINE TABLE final SCHEMALESS; -- terminal comment", "x".repeat(65536));
        let mut units = ImportUnits::new(Chunks(dump.as_bytes()), 128 * 1024);
        let mut requests = Vec::new();
        while let Some(request) = units.next_request().unwrap() { requests.push(request); }
        assert_eq!(requests.len(), 5);
        assert!(requests.iter().all(|sql| sql.starts_with(IMPORT_PREFIX)));
        assert!(requests[0].contains("DEFINE TABLE example"));
        assert!(requests[1].contains("é;still literal"));
        assert!(requests[1].contains("RETURN $x"));
        assert!(requests[2].contains("BEGIN;") && requests[2].contains("COMMIT;"));
        assert!(requests[2].contains("value;with;semicolons"));
        assert!(requests[3].contains("BEGIN;") && requests[3].contains("CANCEL;"));
        assert!(requests[4].contains("DEFINE TABLE final"));
    }

    #[test]
    fn restore_import_units_refuse_bad_tails_modes_and_oversized_transactions() {
        for dump in [
            "", "DEFINE TABLE example SCHEMALESS;", "OPTION IMPORT = FALSE;",
            "OPTION IMPORT; OPTION IMPORT;", "OPTION IMPORT; OPTION IMPORT = FALSE;",
            "OPTION IMPORT; BEGIN; INSERT INTO example [{id: example:a}];",
            "OPTION IMPORT; BEGIN; BEGIN; COMMIT;",
            "OPTION IMPORT; COMMIT;", "OPTION IMPORT; CANCEL;",
            "OPTION IMPORT; INSERT INTO example [{id: example:a, text: 'truncated",
            "OPTION IMPORT; DEFINE TABLE ok SCHEMALESS; INSERT INTO example [{ broken: }];",
        ] {
            let mut units = ImportUnits::new(dump.as_bytes(), 4096);
            let result = (|| { while units.next_request()?.is_some() {} Ok::<_, ModelError>(()) })();
            assert!(result.is_err(), "accepted bad dump: {dump}");
        }
        let dump = format!("OPTION IMPORT; BEGIN; {} COMMIT;", "RETURN 'padding';".repeat(40));
        let mut units = ImportUnits::new(dump.as_bytes(), 256);
        assert!(matches!(units.next_request(), Err(ModelError::Limit { .. })));
        let dump = format!("OPTION IMPORT; RETURN '{}';", "x".repeat(300));
        let mut units = ImportUnits::new(dump.as_bytes(), 256);
        assert!(matches!(units.next_request(), Err(ModelError::Limit { .. })));
        let primary = ModelError::Codec("terminal import failure".into());
        let failure: Result<(), _> = cleanup_result(Err(primary), Ok(()));
        assert!(matches!(failure, Err(ModelError::Codec(detail)) if detail == "terminal import failure"));
        let failure: Result<(), _> = cleanup_result(
            Err(ModelError::Codec("terminal import failure".into())),
            Err(ModelError::Codec("cleanup failure".into())),
        );
        assert!(matches!(failure, Err(ModelError::Infrastructure { detail, .. })
            if detail.contains("terminal import failure") && detail.contains("cleanup failure")));
    }

    #[tokio::test]
    async fn backup_sdk_file_export_faults_never_publish_provisional_bytes() {
        use super::grpc_export_fixture::{Fault, Fixture, PARTIAL};
        use lctx_surrealdb::Credentials;
        for fault in [
            Fault::LateEngineError,
            Fault::LateTaskError,
            Fault::MissingTrailer,
            Fault::ByteCountMismatch,
            Fault::TransportClose,
        ] {
            let fixture = Fixture::start(fault).await;
            let scratch = tempfile::tempdir().unwrap();
            let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
            let provisional = staged.path().to_owned();
            let output = scratch.path().join("snapshot.surql");
            let client = lctx_surrealdb::reader::connect(
                &fixture.endpoint,
                &Credentials::Root {
                    username: "fixture".into(),
                    password: "fixture".into(),
                },
                "injected_export",
                "fixture",
            )
            .await
            .unwrap();
            let exporting = client.clone();
            let path = provisional.clone();
            let mut export =
                tokio::spawn(
                    async move { exporting.export(&path).await.map_err(ModelError::codec) },
                );
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if tokio::fs::read(&provisional).await.unwrap() == PARTIAL {
                        break;
                    }
                    assert!(
                        !export.is_finished(),
                        "{fault:?} must be injected after actual partial file bytes"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            assert!(!export.is_finished());
            if matches!(fault, Fault::TransportClose) {
                fixture.disconnect();
            } else {
                fixture.release_terminal();
            }
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), &mut export)
                .await
                .unwrap()
                .unwrap();
            assert!(result.is_err(), "real SDK must refuse {fault:?}");
            assert_eq!(tokio::fs::read(&provisional).await.unwrap(), PARTIAL);
            let drained = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                client.invalidate().await
            })
            .await
            .unwrap()
            .map_err(ModelError::codec);
            drop(client);
            assert!(
                complete_backup(staged, &output, result, drained, |_| panic!(
                    "failed protocol export must not publish"
                ))
                .is_err()
            );
            assert!(!output.exists(), "{fault:?} left a completed destination");
            assert!(
                !provisional.exists(),
                "{fault:?} retained provisional bytes"
            );
            fixture.close().await;
        }
    }

    #[tokio::test]
    async fn backup_sdk_destination_write_failure_discards_provisional_path() {
        use super::grpc_export_fixture::{Fault, Fixture};
        use lctx_surrealdb::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        // The SDK follows this owned symlink and encounters actual ENOSPC while writing.
        // /dev/full is a device, not operator data; no production injection hook is involved.
        std::fs::remove_file(&provisional).unwrap();
        std::os::unix::fs::symlink("/dev/full", &provisional).unwrap();
        let client = lctx_surrealdb::reader::connect(
            &fixture.endpoint,
            &Credentials::Root {
                username: "fixture".into(),
                password: "fixture".into(),
            },
            "injected_export",
            "fixture",
        )
        .await
        .unwrap();
        fixture.release_terminal();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            client.export(&provisional).await
        })
        .await
        .unwrap()
        .map_err(ModelError::codec);
        assert!(result.is_err(), "actual SDK destination write must fail");
        let drained = client.invalidate().await.map_err(ModelError::codec);
        drop(client);
        assert!(
            complete_backup(staged, &output, result, drained, |_| panic!(
                "failed destination write must not publish"
            ))
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
        fixture.close().await;
    }

    #[tokio::test]
    async fn backup_sdk_terminal_success_publishes_exact_completed_bytes() {
        use super::grpc_export_fixture::{Fault, Fixture, PARTIAL};
        use lctx_surrealdb::Credentials;
        let fixture = Fixture::start(Fault::Success).await;
        let scratch = tempfile::tempdir().unwrap();
        let staged = tempfile::NamedTempFile::new_in(scratch.path()).unwrap();
        let output = scratch.path().join("snapshot.surql");
        let client = lctx_surrealdb::reader::connect(
            &fixture.endpoint,
            &Credentials::Root {
                username: "fixture".into(),
                password: "fixture".into(),
            },
            "injected_export",
            "fixture",
        )
        .await
        .unwrap();
        fixture.release_terminal();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            client.export(staged.path()).await
        })
        .await
        .unwrap()
        .map_err(ModelError::codec);
        let drained = client.invalidate().await.map_err(ModelError::codec);
        drop(client);
        complete_backup(staged, &output, result, drained, |parent| {
            std::fs::File::open(parent)?.sync_all()
        })
        .unwrap();
        assert_eq!(std::fs::read(output).unwrap(), PARTIAL);
        fixture.close().await;
    }

    fn partial_dump(parent: &Path) -> tempfile::NamedTempFile {
        let mut staged = tempfile::NamedTempFile::new_in(parent).unwrap();
        staged.write_all(b"-- partial streamed dump\n").unwrap();
        staged
    }

    #[test]
    fn backup_export_failure_discards_partial_dump_and_preserves_primary_error() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        let error = complete_backup(
            staged,
            &output,
            Err(ModelError::Codec("late export failure".into())),
            Err(ModelError::Codec("session cleanup failure".into())),
            |_| panic!("failed export must not publish or sync directory"),
        )
        .unwrap_err();
        assert!(matches!(error, ModelError::Codec(message) if message == "late export failure"));
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_session_drain_failure_discards_completed_provisional_dump() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        assert!(
            complete_backup(
                staged,
                &output,
                Ok(()),
                Err(ModelError::Codec("session cleanup failure".into())),
                |_| panic!("failed session drain must not publish or sync directory"),
            )
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_publication_never_clobbers_a_destination_created_during_export() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        std::fs::write(&output, b"preexisting backup").unwrap();
        assert!(
            complete_backup(staged, &output, Ok(()), Ok(()), |_| {
                panic!("failed publication must not sync directory")
            })
            .is_err()
        );
        assert_eq!(std::fs::read(output).unwrap(), b"preexisting backup");
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_destination_publication_failure_discards_provisional_dump() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("missing-parent/snapshot.surql");
        assert!(
            complete_backup(staged, &output, Ok(()), Ok(()), |_| {
                panic!("failed publication must not sync directory")
            })
            .is_err()
        );
        assert!(!output.exists());
        assert!(!provisional.exists());
    }

    #[test]
    fn backup_postpublication_sync_failure_retains_dump_and_reports_uncertain_durability() {
        let scratch = tempfile::tempdir().unwrap();
        let staged = partial_dump(scratch.path());
        let provisional = staged.path().to_owned();
        let output = scratch.path().join("snapshot.surql");
        let error = complete_backup(staged, &output, Ok(()), Ok(()), |_| {
            Err(std::io::Error::other("injected directory sync failure"))
        })
        .unwrap_err();
        assert!(matches!(error, ModelError::Infrastructure {
            class: Infrastructure::Unconfirmed,
            detail,
        } if detail.contains("was published") && detail.contains("durability is uncertain")));
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"-- partial streamed dump\n"
        );
        assert!(!provisional.exists());
        assert!(
            complete_backup(
                partial_dump(scratch.path()),
                &output,
                Ok(()),
                Ok(()),
                |_| { panic!("retry must not overwrite the committed destination") }
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"-- partial streamed dump\n"
        );
    }

    #[tokio::test]
    async fn backup_grpc_file_export_on_owned_persistent_fixture() {
        use lctx_model::domain::{
            ContentHash,
            serving::{DatabaseIdentity, Name},
        };

        let fixture_path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
            .expect("owned disposable native server required");
        let fixture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let config = RuntimeConfig {
            endpoint: fixture["grpc_endpoint"].as_str().unwrap().into(),
            username: fixture["admin_user"].as_str().unwrap().into(),
            password: fixture["admin_password"].as_str().unwrap().into(),
            viewer_username: "backup_viewer".into(),
            viewer_password: "owned-backup-fixture-password".into(),
            namespace: Name::new(format!("backup_control_{}", std::process::id())).unwrap(),
            cache_database: Name::new("cache").unwrap(),
            selection: scratch.path().join("selected.json"),
        };
        let handle = SnapshotHandle {
            semantic: ContentHash::of(b"backup-control-semantic"),
            realization: ContentHash::of(b"backup-control-realization"),
            database: DatabaseIdentity {
                namespace: config.namespace.clone(),
                database: Name::new("backup_fixture").unwrap(),
            },
        };
        let client = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.root_credentials(),
            config.namespace.as_str(),
            handle.database.database.as_str(),
        )
        .await
        .unwrap();
        let mut vars = Variables::new();
        vars.insert("handle", hex::encode(serde_json::to_vec(&handle).unwrap()));
        client
            .query("DEFINE USER backup_viewer ON DATABASE PASSWORD 'owned-backup-fixture-password' ROLES VIEWER;
                DEFINE TABLE publication SCHEMALESS PERMISSIONS FOR select FULL;
                CREATE publication:current SET handle = $handle;
                CREATE entity:example SET canonical = 'canonical-backup-sentinel';
                CREATE discovery_document:example SET text = 'derived-search-sentinel';")
            .bind(vars)
            .await
            .unwrap()
            .check()
            .unwrap();
        let output = scratch.path().join("snapshot.surql");
        backup(&config, &handle, &output).await.unwrap();
        let dump = std::fs::read_to_string(&output).unwrap();
        assert!(dump.contains("canonical-backup-sentinel"));
        assert!(dump.contains("publication"));
        assert!(!dump.contains("derived-search-sentinel"));
        assert!(!dump.contains("backup_viewer"));
        assert!(!dump.contains("owned-backup-fixture-password"));
        assert!(!config.selection.exists());
        assert!(backup(&config, &handle, &output).await.is_err());
        assert_eq!(std::fs::read_to_string(output).unwrap(), dump);
        client
            .query("REMOVE DATABASE backup_fixture")
            .await
            .unwrap()
            .check()
            .unwrap();
        client.invalidate().await.unwrap();
    }
}
