//! Resumable publication of a verified portable projection. No Delta/compiler dependency.
use crate::{
    Error,
    profiles::{Policy, hex},
    projection,
    serving::{IMPORT_BUFFER_BYTES, ImportStore, QueryLease, STAGING_BYTES},
};
use arrow_array::RecordBatch;
use arrow_ipc::reader::FileReader;
use cpg_schema::{
    bundle,
    id::Digest,
    serving_projection::{self as contract, Manifest, corrupt, refused},
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use sqlx::{
    Connection, PgConnection,
    postgres::{PgAdvisoryLock, PgAdvisoryLockKey},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn io_error(_: std::io::Error) -> Error {
    contract::ProjectionError {
        kind: contract::FailureKind::Unavailable,
        message: "projection artifact I/O failed".into(),
    }
    .into()
}
pub(crate) fn digest(value: &str) -> Result<Digest, Error> {
    Digest::from_hex(value).ok_or_else(|| corrupt("invalid full digest").into())
}
pub(crate) fn lock(id: &Digest) -> PgAdvisoryLock {
    PgAdvisoryLock::with_key(PgAdvisoryLockKey::BigInt(i64::from_be_bytes(
        id.0[..8].try_into().expect("digest prefix"),
    )))
}

#[derive(Clone)]
pub struct Source {
    manifest: Manifest,
    generation: Digest,
    root: PathBuf,
    files: BTreeMap<String, String>,
    transport: Digest,
}
impl Source {
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn generation(&self) -> Digest {
        self.generation
    }

    /// Full shared validation occurs once, before obtaining a database lease.
    pub fn open(root: &Path) -> Result<Self, Error> {
        let raw = read_bounded(&root.join("MANIFEST.json"), 1024 * 1024)?;
        let outer: Value =
            serde_json::from_slice(&raw).map_err(|_| corrupt("invalid source manifest"))?;
        let manifest: Manifest = serde_json::from_value(outer["projection"].clone())
            .map_err(|_| corrupt("invalid projection manifest"))?;
        manifest.validate()?;
        manifest.validate_envelope(&outer)?;
        let generation = digest(&manifest.generation()?)?;
        if outer["projection_generation"].as_str() != Some(generation.hex().as_str()) {
            return Err(corrupt("projection source identity").into());
        }
        let mut files = BTreeMap::new();
        for file in bundle::files(manifest.dimensions) {
            let entry = &outer["files"][file.name];
            if entry["file"].as_str() != Some(format!("{}.arrow", file.name).as_str()) {
                return Err(corrupt("source file name").into());
            }
            let hash = entry["sha256"]
                .as_str()
                .ok_or_else(|| corrupt("source file digest"))?;
            digest(hash)?;
            files.insert(file.name.to_owned(), hash.to_owned());
        }
        let transport = Digest(
            Sha256::digest(serde_json::to_vec(&files).map_err(|_| corrupt("transport encoding"))?)
                .into(),
        );
        let source = Self {
            manifest,
            generation,
            root: root.to_owned(),
            files,
            transport,
        };
        let mut tables = BTreeMap::new();
        let mut bytes = 0;
        for name in source.files.keys() {
            let batches = source.read(name)?;
            for b in &batches {
                bytes += contract::batch_bytes(b)?;
            }
            if bytes > 512 * 1024 * 1024 {
                return Err(refused("source validation byte budget").into());
            }
            tables.insert(name.clone(), batches);
        }
        source.manifest.validate_relations(&tables)?;
        Ok(source)
    }
    fn read(&self, name: &str) -> Result<Vec<RecordBatch>, Error> {
        let expected = self
            .files
            .get(name)
            .ok_or_else(|| corrupt("undeclared input relation"))?;
        let raw = read_bounded(
            &self.root.join(format!("{name}.arrow")),
            contract::MAX_RELATION_BYTES,
        )?;
        if hex(Sha256::digest(&raw)) != *expected {
            return Err(corrupt("frozen source changed").into());
        }
        self.manifest
            .verify_artifact(&format!("{name}.arrow"), &raw)?;
        let reader = FileReader::try_new(std::io::Cursor::new(raw), None)
            .map_err(|_| corrupt("source IPC"))?;
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| corrupt("source IPC rows"))?;
        if contract::receipt(name, self.manifest.dimensions, &batches)?
            != self.manifest.relations[name]
        {
            return Err(corrupt("source relation receipt").into());
        }
        Ok(batches)
    }
    fn artifacts(&self, root: &Path) -> Result<Vec<(String, PathBuf)>, Error> {
        fs_err::create_dir_all(root).map_err(io_error)?;
        let root = fs_err::canonicalize(root).map_err(io_error)?;
        let mut locations = Vec::new();
        for (name, receipt) in &self.manifest.artifacts {
            let dir = root.join(&receipt.sha256);
            fs_err::create_dir_all(&dir).map_err(io_error)?;
            fs_err::File::open(&root)
                .map_err(io_error)?
                .sync_all()
                .map_err(io_error)?;
            let path = dir.join(name);
            if !path.exists() {
                let bytes = read_bounded(&self.root.join(name), contract::MAX_RELATION_BYTES)?;
                self.manifest.verify_artifact(name, &bytes)?;
                let mut out = tempfile::NamedTempFile::new_in(&dir).map_err(io_error)?;
                out.write_all(&bytes).map_err(io_error)?;
                out.as_file().sync_all().map_err(io_error)?;
                if let Err(e) = out.persist_noclobber(&path)
                    && e.error.kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(io_error(e.error));
                }
                fs_err::File::open(&dir)
                    .map_err(io_error)?
                    .sync_all()
                    .map_err(io_error)?;
            }
            self.manifest
                .verify_artifact(name, &read_bounded(&path, contract::MAX_RELATION_BYTES)?)?;
            locations.push((name.clone(), path));
        }
        Ok(locations)
    }
}
pub(crate) fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, Error> {
    let meta = fs_err::symlink_metadata(path).map_err(io_error)?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(refused("artifact file type/byte budget").into());
    }
    let mut bytes = Vec::new();
    fs_err::File::open(path)
        .map_err(io_error)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > max {
        return Err(refused("artifact read byte budget").into());
    }
    Ok(bytes)
}

#[derive(Debug, Serialize)]
pub struct ImportStatus {
    pub generation: String,
    pub state: String,
    pub completed_batches: i64,
    pub profile: String,
}
fn load_order(dimensions: i32) -> Result<Vec<&'static str>, Error> {
    let mut remaining: BTreeSet<_> = bundle::files(dimensions).iter().map(|f| f.name).collect();
    let mut order = Vec::new();
    let links = contract::foreign_keys();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .copied()
            .find(|n| {
                links
                    .iter()
                    .filter(|(child, _, _, _)| child == n)
                    .all(|(_, _, parent, _)| !remaining.contains(parent))
            })
            .ok_or_else(|| corrupt("cyclic projection load order"))?;
        remaining.remove(next);
        order.push(next);
    }
    Ok(order)
}
impl ImportStore {
    /// Append verified content-addressed locations; never change generation content or selection.
    pub async fn relocate_artifacts(
        &self,
        id: Digest,
        root: PathBuf,
    ) -> Result<ImportStatus, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut guard = lock(&id).acquire(&mut *lease.connection).await?;
        let raw:String=sqlx::query_scalar("SELECT canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1 AND state='ready'").bind(id.0.as_slice()).fetch_one(guard.as_mut()).await?;
        let (_, artifacts, _) = recovery_envelope(&raw, &id)?;
        let owned = artifacts.clone();
        let locations = tokio::task::spawn_blocking(move || {
            let root = fs_err::canonicalize(root).map_err(io_error)?;
            let mut locations = Vec::new();
            for (name, r) in &owned {
                let path = root.join(&r.sha256).join(name);
                verify_artifact_receipt(r, &read_bounded(&path, contract::MAX_RELATION_BYTES)?)?;
                locations.push((name.clone(), path));
            }
            Ok::<_, Error>(locations)
        })
        .await
        .map_err(|_| Error::Integrity("relocation worker failed"))??;
        let mut tx = guard.as_mut().begin().await?;
        for (name, path) in locations {
            let r = &artifacts[&name];
            sqlx::query("SELECT lctx_serving.register_artifact($1,$2,$3,$4,$5,$6)")
                .bind(id.0.as_slice())
                .bind(name)
                .bind(digest(&r.sha256)?.0.as_slice())
                .bind(r.bytes as i64)
                .bind(r.format as i32)
                .bind(path.to_str().ok_or_else(|| corrupt("artifact path"))?)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        let status = status_on(guard.as_mut(), &id).await?;
        guard.release_now().await?;
        lease.complete();
        Ok(status)
    }

    /// Export validated ready manifests/artifact closure under the backup's exported snapshot.
    pub async fn recovery_inventory(&self, snapshot: &str) -> Result<serde_json::Value, Error> {
        if snapshot.is_empty()
            || snapshot.len() > 64
            || !snapshot.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        {
            return Err(Error::Request("invalid exported snapshot".into()));
        }
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut tx = lease
            .connection
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "SET TRANSACTION SNAPSHOT '{snapshot}'; SET LOCAL transaction_timeout='300s'"
        )))
        .execute(&mut *tx)
        .await?;
        let rows:Vec<(Vec<u8>,String)>=sqlx::query_as("SELECT generation_digest,canonical_manifest FROM lctx_serving.generations WHERE state='ready' ORDER BY generation_digest LIMIT 1001").fetch_all(&mut *tx).await?;
        if rows.len() > 1000 {
            return Err(refused("recovery generation budget").into());
        }
        let mut generations = Vec::new();
        let mut bytes = 0;
        for (id, raw) in rows {
            bytes += raw.len();
            if bytes > 16 * 1024 * 1024 {
                return Err(refused("recovery manifest budget").into());
            }
            let id = Digest(id.try_into().map_err(|_| corrupt("recovery identity"))?);
            let (manifest, receipts, current) = recovery_envelope(&raw, &id)?;
            let paths = verify_artifact_locations(&mut tx, &id, &receipts).await?;
            let artifacts=paths.into_iter().map(|(name,path)| {let r=&receipts[&name];serde_json::json!({"name":name,"location":path,"sha256":r.sha256,"bytes":r.bytes,"format":r.format})}).collect::<Vec<_>>();
            generations.push(serde_json::json!({"generation":id.hex(),"manifest":manifest,"canonical_manifest":raw,"runtime_admission":if current {"current"} else {"legacy_runtime_required"},"artifacts":artifacts}));
        }
        tx.commit().await?;
        lease.complete();
        Ok(serde_json::json!({"format":2,"generations":generations}))
    }
    pub async fn import(
        &self,
        source: Source,
        artifact_root: PathBuf,
    ) -> Result<ImportStatus, Error> {
        // A retry always reconciles durable receipts on a fresh lease before sending COPY.
        let mut tries = 0;
        loop {
            match self.import_once(&source, &artifact_root).await {
                Err(e) if e.retryable() && tries < 2 => {
                    tries += 1;
                }
                result => return result,
            }
        }
    }
    async fn import_once(
        &self,
        source: &Source,
        artifact_root: &Path,
    ) -> Result<ImportStatus, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut guard = lock(&source.generation)
            .acquire(&mut *lease.connection)
            .await?;
        let attempt: i64 = sqlx::query_scalar("SELECT lctx_serving.prepare_generation($1,$2)")
            .bind(
                serde_json::to_string(&source.manifest)
                    .map_err(|_| corrupt("manifest encoding"))?,
            )
            .bind(source.transport.0.as_slice())
            .fetch_one(guard.as_mut())
            .await?;
        let result = self.load(source, artifact_root, guard.as_mut()).await;
        let terminal = matches!(&result,Err(Error::Projection(e)) if matches!(e.kind,contract::FailureKind::Corrupt|contract::FailureKind::Incompatible));
        let outcome = if result.is_ok() {
            "completed"
        } else if terminal {
            "failed"
        } else {
            "interrupted"
        };
        if terminal {
            let _ = sqlx::query("SELECT lctx_serving.mark_failed($1)")
                .bind(source.generation.0.as_slice())
                .execute(guard.as_mut())
                .await;
        }
        let finished = sqlx::query("SELECT lctx_serving.finish_attempt($1,$2,$3)")
            .bind(attempt)
            .bind(outcome)
            .bind(if result.is_err() {
                Some(if terminal {
                    "invalid_projection"
                } else {
                    "import_interrupted"
                })
            } else {
                None::<&str>
            })
            .execute(guard.as_mut())
            .await;
        if result.is_ok() {
            finished?;
        }
        guard.release_now().await?;
        if result.is_ok() {
            lease.complete();
        }
        result
    }
    async fn load(
        &self,
        source: &Source,
        artifact_root: &Path,
        conn: &mut PgConnection,
    ) -> Result<ImportStatus, Error> {
        let state: String = sqlx::query_scalar(
            "SELECT state FROM lctx_serving.generations WHERE generation_digest=$1",
        )
        .bind(source.generation.0.as_slice())
        .fetch_one(&mut *conn)
        .await?;
        let owned = source.clone();
        let root = artifact_root.to_owned();
        let locations = tokio::task::spawn_blocking(move || owned.artifacts(&root))
            .await
            .map_err(|_| Error::Integrity("artifact worker failed"))??;
        for (name, path) in &locations {
            let a = &source.manifest.artifacts[name];
            sqlx::query("SELECT lctx_serving.register_artifact($1,$2,$3,$4,$5,$6)")
                .bind(source.generation.0.as_slice())
                .bind(name)
                .bind(digest(&a.sha256)?.0.as_slice())
                .bind(a.bytes as i64)
                .bind(a.format as i32)
                .bind(
                    path.to_str()
                        .ok_or_else(|| corrupt("artifact path encoding"))?,
                )
                .execute(&mut *conn)
                .await?;
        }
        if state == "loading" {
            for name in load_order(source.manifest.dimensions)? {
                let owned = source.clone();
                let batches = tokio::task::spawn_blocking(move || owned.read(name))
                    .await
                    .map_err(|_| Error::Integrity("input worker failed"))??;
                let retained = batches
                    .iter()
                    .try_fold(0usize, |n, b| contract::batch_bytes(b).map(|size| n + size))?;
                let mut ordinal = 0i64;
                let mut batch_ordinal = 0i64;
                for batch in batches {
                    let mut offset = 0;
                    while offset < batch.num_rows() {
                        let mut length = 1000.min(batch.num_rows() - offset);
                        let (slice, encoded) = loop {
                            let slice = batch.slice(offset, length);
                            match projection::copy_bytes(name, source.manifest.dimensions, &slice) {
                                Ok(bytes) => break (slice, bytes),
                                Err(e)
                                    if e.kind == contract::FailureKind::ResourceRefused
                                        && length > 1 =>
                                {
                                    length = length.div_ceil(2);
                                }
                                Err(e) => return Err(e.into()),
                            }
                        };
                        if retained + encoded.len() > IMPORT_BUFFER_BYTES {
                            return Err(refused("aggregate COPY buffer budget").into());
                        }
                        let hash = Sha256::digest(&encoded);
                        let prior:Option<(i64,i64,Vec<u8>)>=sqlx::query_as("SELECT first_row,row_count,content_digest FROM lctx_serving.import_batches WHERE generation_digest=$1 AND relation_name=$2 AND batch_ordinal=$3")
                            .bind(source.generation.0.as_slice()).bind(name).bind(batch_ordinal).fetch_optional(&mut *conn).await?;
                        if let Some(prior) = prior {
                            if prior != (ordinal, length as i64, hash.to_vec()) {
                                return Err(corrupt("committed batch receipt conflict").into());
                            }
                        } else {
                            load_batch(
                                conn,
                                &source.generation,
                                name,
                                &slice,
                                &encoded,
                                ordinal,
                                batch_ordinal,
                                &hash,
                            )
                            .await?;
                        }
                        offset += length;
                        ordinal += length as i64;
                        batch_ordinal += 1;
                    }
                }
            }
            sqlx::query("SELECT lctx_serving.freeze_generation($1)")
                .bind(source.generation.0.as_slice())
                .execute(&mut *conn)
                .await?;
        }
        if state != "ready" {
            let mut maintenance = conn.begin().await?;
            sqlx::raw_sql("SET LOCAL statement_timeout='300s'; SET LOCAL transaction_timeout='300s'; SET LOCAL maintenance_work_mem='256MB'; SET LOCAL max_parallel_maintenance_workers=2").execute(&mut *maintenance).await?;
            sqlx::query("SELECT lctx_serving.analyze_generation($1)")
                .bind(source.generation.0.as_slice())
                .execute(&mut *maintenance)
                .await?;
            maintenance.commit().await?;
            validate_stored(conn, &source.generation, &source.manifest).await?;
            check_indexes(conn, &source.generation).await?;
            verify_locations(conn, &source.generation, &source.manifest).await?;
            // No materialization or filesystem work occurs inside this final transaction.
            let mut tx = conn.begin().await?;
            sqlx::query("SELECT lctx_serving.mark_ready($1,$2,$3)")
                .bind(source.generation.0.as_slice())
                .bind(Policy::exact().canonical()?)
                .bind(
                    serde_json::to_value(&source.manifest.relations)
                        .map_err(|_| corrupt("receipt encoding"))?,
                )
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        status_on(conn, &source.generation).await
    }
    pub async fn status(&self, generation: Digest) -> Result<ImportStatus, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let result = status_on(&mut lease.connection, &generation).await?;
        lease.complete();
        Ok(result)
    }
    pub async fn select(
        &self,
        library: &str,
        generation: Digest,
        profile: Digest,
    ) -> Result<(), Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let raw:String=sqlx::query_scalar("SELECT canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1 AND state='ready'").bind(generation.0.as_slice()).fetch_one(&mut *lease.connection).await?;
        let manifest: Manifest =
            serde_json::from_str(&raw).map_err(|_| corrupt("selection manifest"))?;
        manifest.validate()?;
        verify_locations(&mut lease.connection, &generation, &manifest).await?;
        sqlx::query("SELECT lctx_serving.select_generation($1,$2,$3)")
            .bind(library)
            .bind(generation.0.as_slice())
            .bind(profile.0.as_slice())
            .execute(&mut *lease.connection)
            .await?;
        lease.complete();
        Ok(())
    }
    pub async fn cleanup(&self, generation: Digest) -> Result<(), Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        sqlx::query("SELECT lctx_serving.cleanup_generation($1)")
            .bind(generation.0.as_slice())
            .execute(&mut *lease.connection)
            .await?;
        lease.complete();
        Ok(())
    }
    pub async fn reconcile(&self, generation: Digest) -> Result<ImportStatus, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut guard = lock(&generation).acquire(&mut *lease.connection).await?;
        let text: String = sqlx::query_scalar(
            "SELECT canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1",
        )
        .bind(generation.0.as_slice())
        .fetch_one(guard.as_mut())
        .await?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|_| corrupt("stored manifest"))?;
        manifest.validate()?;
        let state: String = sqlx::query_scalar(
            "SELECT state FROM lctx_serving.generations WHERE generation_digest=$1",
        )
        .bind(generation.0.as_slice())
        .fetch_one(guard.as_mut())
        .await?;
        if state != "validating" && state != "ready" {
            return Err(contract::ProjectionError {
                kind: contract::FailureKind::Incomplete,
                message: "resume the frozen import input before reconciliation".into(),
            }
            .into());
        }
        validate_stored(guard.as_mut(), &generation, &manifest).await?;
        verify_locations(guard.as_mut(), &generation, &manifest).await?;
        check_indexes(guard.as_mut(), &generation).await?;
        if state == "validating" {
            sqlx::query("SELECT lctx_serving.mark_ready($1,$2,$3)")
                .bind(generation.0.as_slice())
                .bind(Policy::exact().canonical()?)
                .bind(
                    serde_json::to_value(&manifest.relations)
                        .map_err(|_| corrupt("receipt encoding"))?,
                )
                .execute(guard.as_mut())
                .await?;
        }
        let result = status_on(guard.as_mut(), &generation).await?;
        guard.release_now().await?;
        lease.complete();
        Ok(result)
    }
}
async fn status_on(conn: &mut PgConnection, id: &Digest) -> Result<ImportStatus, Error> {
    let (state,batches):(String,i64)=sqlx::query_as("SELECT state,(SELECT count(*) FROM lctx_serving.import_batches b WHERE b.generation_digest=g.generation_digest) FROM lctx_serving.generations g WHERE generation_digest=$1")
        .bind(id.0.as_slice()).fetch_one(conn).await?;
    Ok(ImportStatus {
        generation: id.hex(),
        state,
        completed_batches: batches,
        profile: Policy::exact().digest()?,
    })
}
#[expect(
    clippy::too_many_arguments,
    reason = "explicit COPY batch and its durable receipt"
)]
async fn load_batch(
    conn: &mut PgConnection,
    id: &Digest,
    name: &str,
    batch: &RecordBatch,
    encoded: &[u8],
    first: i64,
    number: i64,
    hash: &[u8],
) -> Result<(), Error> {
    if encoded.len() > STAGING_BYTES {
        return Err(refused("COPY byte budget").into());
    }
    // Identifiers come only from the validated cpg-schema inventory; values stay bound.
    let schema = batch.schema();
    let columns = schema
        .fields()
        .iter()
        .map(|f| {
            format!(
                "\"{}\" {}{}",
                f.name(),
                projection::staging_type(f.data_type()).expect("validated schema"),
                if f.is_nullable() { "" } else { " NOT NULL" }
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let names = schema
        .fields()
        .iter()
        .map(|f| format!("\"{}\"", f.name()))
        .collect::<Vec<_>>()
        .join(",");
    let selected = schema
        .fields()
        .iter()
        .map(|f| {
            if matches!(f.data_type(), arrow_schema::DataType::FixedSizeList(..)) {
                format!("\"{}\"::lctx_ext.vector(1024)", f.name())
            } else {
                format!("\"{}\"", f.name())
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut tx = conn.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TEMP TABLE lctx_stage(_ordinal bigint GENERATED ALWAYS AS IDENTITY,{columns}) ON COMMIT DROP"))).execute(&mut *tx).await?;
    let mut copy = tx
        .copy_in_raw(&format!(
            "COPY pg_temp.lctx_stage({names}) FROM STDIN WITH(FORMAT BINARY)"
        ))
        .await?;
    copy.send(encoded).await?;
    let copied = copy.finish().await?;
    if copied != batch.num_rows() as u64 {
        return Err(corrupt("COPY count mismatch").into());
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO lctx_serving.\"{name}\"(generation_digest,row_ordinal,{names}) SELECT $1,$2+_ordinal-1,{selected} FROM pg_temp.lctx_stage ORDER BY _ordinal")))
        .bind(id.0.as_slice()).bind(first).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO lctx_serving.import_batches VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id.0.as_slice())
        .bind(name)
        .bind(number)
        .bind(first)
        .bind(batch.num_rows() as i64)
        .bind(hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
pub(crate) async fn validate_stored(
    conn: &mut PgConnection,
    id: &Digest,
    manifest: &Manifest,
) -> Result<(), Error> {
    let mut tables = BTreeMap::new();
    let mut total = 0;
    for file in bundle::files(manifest.dimensions) {
        let mut batches = Vec::new();
        let mut offset = 0i64;
        loop {
            let rows=sqlx::query(sqlx::AssertSqlSafe(format!("SELECT * FROM lctx_serving.\"{}\" WHERE generation_digest=$1 ORDER BY row_ordinal LIMIT 1000 OFFSET $2",file.name))).bind(id.0.as_slice()).bind(offset).fetch_all(&mut *conn).await?;
            if rows.is_empty() {
                break;
            }
            offset += rows.len() as i64;
            if offset > contract::MAX_RELATION_ROWS as i64 {
                return Err(refused("stored relation row budget").into());
            }
            let batch = projection::decode_rows(file.name, manifest.dimensions, &rows)?;
            total += contract::batch_bytes(&batch)?;
            if total > 512 * 1024 * 1024 {
                return Err(refused("stored validation byte budget").into());
            }
            batches.push(batch);
        }
        tables.insert(file.name.to_owned(), batches);
    }
    manifest.validate_relations(&tables)?;
    Ok(())
}

async fn check_indexes(conn: &mut PgConnection, id: &Digest) -> Result<(), Error> {
    let definitions = [
        (
            "catalog_configuration_class",
            "catalog_configurations",
            "generation_digest, class_node_id",
        ),
        (
            "catalog_field_link_class",
            "catalog_field_links",
            "generation_digest, class_node_id",
        ),
        (
            "operation_order",
            "operations",
            "generation_digest, access_path COLLATE \"C\", node_id",
        ),
        (
            "facet_lookup",
            "operation_facets",
            "generation_digest, facet, value, verdict, node_id",
        ),
        (
            "behavior_owner",
            "behaviors",
            "generation_digest, operation_node_id, row_ordinal",
        ),
        (
            "discharge_owner",
            "behavior_discharges",
            "generation_digest, behavior_id, row_ordinal",
        ),
        (
            "assertion_owner",
            "assertions",
            "generation_digest, brief_id, ordinal",
        ),
        (
            "support_owner",
            "supports",
            "generation_digest, assertion_id, ordinal",
        ),
        (
            "witness_owner",
            "support_witnesses",
            "generation_digest, finding_id, row_ordinal",
        ),
        (
            "member_owner",
            "support_members",
            "generation_digest, finding_id, row_ordinal",
        ),
        (
            "incidence_owner",
            "support_attribute_incidences",
            "generation_digest, finding_id, row_ordinal",
        ),
    ];
    for (name, table, columns) in definitions {
        let definition:Option<String>=sqlx::query_scalar("SELECT pg_get_indexdef(i.indexrelid) FROM pg_catalog.pg_index i JOIN pg_catalog.pg_class c ON c.oid=i.indexrelid JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_serving' AND c.relname=$1 AND i.indisready AND i.indisvalid").bind(name).fetch_optional(&mut *conn).await?;
        let expected =
            format!("CREATE INDEX {name} ON lctx_serving.{table} USING btree ({columns})");
        if definition.as_deref() != Some(&expected) {
            return Err(corrupt("required projection index definition").into());
        }
    }
    sqlx::query("SELECT lctx_serving.check_partitions($1)")
        .bind(id.0.as_slice())
        .execute(conn)
        .await?;
    Ok(())
}
pub(crate) async fn verify_locations(
    conn: &mut PgConnection,
    id: &Digest,
    manifest: &Manifest,
) -> Result<Vec<(String, PathBuf)>, Error> {
    verify_artifact_locations(conn, id, &manifest.artifacts).await
}

/// A transport envelope is not admission to the current relational/native reader.
fn recovery_envelope(
    raw: &str,
    id: &Digest,
) -> Result<(Value, BTreeMap<String, contract::ArtifactReceipt>, bool), Error> {
    if hex(Sha256::digest(raw.as_bytes())) != id.hex() {
        return Err(corrupt("recovery manifest identity").into());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| corrupt("recovery manifest"))?;
    let current = match (value["format"].as_u64(), value["bundle_format"].as_u64()) {
        (Some(f), Some(b))
            if f == u64::from(contract::FORMAT) && b == u64::from(contract::BUNDLE_FORMAT) =>
        {
            let manifest: Manifest =
                serde_json::from_str(raw).map_err(|_| corrupt("recovery current manifest"))?;
            manifest.validate()?;
            if manifest.generation()? != id.hex() {
                return Err(corrupt("noncanonical current manifest").into());
            }
            true
        }
        (Some(2), Some(12)) | (Some(3), Some(13)) | (Some(4), Some(14)) => false,
        _ => return Err(corrupt("unsupported retained manifest format").into()),
    };
    let artifacts: BTreeMap<String, contract::ArtifactReceipt> =
        serde_json::from_value(value["artifacts"].clone())
            .map_err(|_| corrupt("recovery artifacts"))?;
    if artifacts.is_empty() || artifacts.len() > 256 {
        return Err(refused("recovery artifact inventory budget").into());
    }
    for (name, receipt) in &artifacts {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || digest(&receipt.sha256).is_err()
            || receipt.format != 1
        {
            return Err(corrupt("recovery artifact identity").into());
        }
        if receipt.bytes > contract::MAX_RELATION_BYTES as u64 {
            return Err(refused("recovery artifact bytes").into());
        }
    }
    Ok((value, artifacts, current))
}

fn verify_artifact_receipt(receipt: &contract::ArtifactReceipt, bytes: &[u8]) -> Result<(), Error> {
    if receipt.bytes != bytes.len() as u64 || receipt.sha256 != hex(Sha256::digest(bytes)) {
        return Err(corrupt("artifact content identity").into());
    }
    Ok(())
}

async fn verify_artifact_locations(
    conn: &mut PgConnection,
    id: &Digest,
    artifacts: &BTreeMap<String, contract::ArtifactReceipt>,
) -> Result<Vec<(String, PathBuf)>, Error> {
    let registered: Vec<(String,Vec<u8>,i64,i32)> = sqlx::query_as("SELECT name,content_digest,bytes,format FROM lctx_serving.artifacts WHERE generation_digest=$1 ORDER BY name")
        .bind(id.0.as_slice()).fetch_all(&mut *conn).await?;
    if registered.len() != artifacts.len()
        || registered.iter().any(|(name, sha, bytes, format)| {
            artifacts.get(name).is_none_or(|r| {
                r.sha256 != hex(sha) || r.bytes != *bytes as u64 || r.format != *format as u32
            })
        })
    {
        return Err(corrupt("registered artifact closure differs from manifest").into());
    }
    let rows:Vec<(String,String)>=sqlx::query_as("SELECT name,location FROM lctx_serving.artifact_locations WHERE generation_digest=$1 ORDER BY name,location").bind(id.0.as_slice()).fetch_all(conn).await?;
    let artifacts = artifacts.clone();
    tokio::task::spawn_blocking(move || {
        let mut locations = Vec::new();
        for (name, receipt) in &artifacts {
            let mut damaged = false;
            let found = rows
                .iter()
                .filter(|(n, _)| n == name)
                .find_map(|(_, location)| {
                    let path = PathBuf::from(location);
                    match read_bounded(&path, contract::MAX_RELATION_BYTES) {
                        Ok(bytes) if verify_artifact_receipt(receipt, &bytes).is_ok() => Some(path),
                        Err(Error::Projection(e))
                            if e.kind == contract::FailureKind::Unavailable =>
                        {
                            None
                        }
                        _ => {
                            damaged = true;
                            None
                        }
                    }
                })
                .ok_or_else(|| {
                    if damaged {
                        corrupt("required artifact corrupt")
                    } else {
                        contract::ProjectionError {
                            kind: contract::FailureKind::Unavailable,
                            message: "required artifact missing".into(),
                        }
                    }
                })?;
            locations.push((name.clone(), found));
        }
        Ok(locations)
    })
    .await
    .map_err(|_| Error::Integrity("artifact verification worker failed"))?
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn retained_envelope_preserves_bytes_without_admitting_old_runtime() {
        let bytes = b"retained artifact";
        let mut value = serde_json::json!({"format":2,"bundle_format":12,"legacy_field":"kept",
            "artifacts":{"operations.arrow":{"sha256":hex(Sha256::digest(bytes)),"bytes":bytes.len(),"format":1}}});
        let raw = serde_json::to_string(&value).unwrap();
        let id = Digest(Sha256::digest(raw.as_bytes()).into());
        let (retained, artifacts, current) = recovery_envelope(&raw, &id).unwrap();
        assert_eq!(retained, value);
        assert!(!current);
        assert!(verify_artifact_receipt(&artifacts["operations.arrow"], bytes).is_ok());
        assert!(verify_artifact_receipt(&artifacts["operations.arrow"], b"corrupt").is_err());
        assert!(recovery_envelope(&(raw + " "), &id).is_err());
        value["format"] = serde_json::json!(4);
        let raw = value.to_string();
        assert!(recovery_envelope(&raw, &Digest(Sha256::digest(raw.as_bytes()).into())).is_err());
    }
}
