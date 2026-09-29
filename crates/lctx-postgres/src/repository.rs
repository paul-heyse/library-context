//! Generation-pinned exact selection and set-based hydration. The only online query owner.
use crate::{
    Error,
    import::{digest, verify_locations},
    profiles::{Policy, hex},
    projection,
    serving::{QueryLease, ServingStore},
};
use arrow_array::{Array, BooleanArray, FixedSizeBinaryArray, Int64Array, StringArray};
use arrow_schema::DataType;
use cpg_schema::{
    id::{Digest, Id},
    serving_projection::{self as contract, Manifest, corrupt, refused},
};
use futures::TryStreamExt;
use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::{PgConnection, Row, ValueRef};
use std::{collections::BTreeSet, path::PathBuf};

pub const HYDRATION_BYTES: usize = 64 * 1024 * 1024;
pub const RESPONSE_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone)]
pub struct PinnedGeneration {
    pub(crate) id: Digest,
    pub(crate) manifest: Manifest,
    pub(crate) policy: Policy,
    pub(crate) profile: Digest,
    pub(crate) artifacts: Vec<(String, PathBuf)>,
}
impl PinnedGeneration {
    pub fn id(&self) -> Digest {
        self.id
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn artifacts(&self) -> &[(String, PathBuf)] {
        &self.artifacts
    }

    pub fn generation(&self) -> String {
        self.id.hex()
    }
    pub fn descriptor(&self) -> Value {
        json!({"generation":self.id.hex(),"profile":self.profile.hex(),"policy":self.policy,"manifest":self.manifest,"artifacts":self.artifacts})
    }
    pub fn check_snapshot(&self, snapshot: &str) -> Result<(), Error> {
        if snapshot != self.manifest.snapshot_id {
            return Err(Error::Request(
                "snapshot is not the pinned one; search again".into(),
            ));
        }
        Ok(())
    }
}
impl From<cpg_schema::wire::WireError> for Error {
    fn from(e: cpg_schema::wire::WireError) -> Self {
        Self::Request(e.to_string())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedOperation {
    pub operation_id: String,
    pub access_path: String,
}

impl ServingStore {
    pub async fn pin(
        &self,
        library: &str,
        generation: Option<Digest>,
        profile: Option<Digest>,
    ) -> Result<PinnedGeneration, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let (id, profile) = if let Some(id) = generation {
            (id, profile.unwrap_or(digest(&Policy::exact().digest()?)?))
        } else {
            let selected:Option<(Vec<u8>,Option<Vec<u8>>)>=sqlx::query_as("SELECT generation_digest,profile_digest FROM lctx_serving.selections WHERE library=$1").bind(library).fetch_optional(&mut *lease.connection).await?;
            let (id, p) = selected.ok_or_else(|| {
                Error::Request("no ready generation is selected for this library".into())
            })?;
            (
                Digest(
                    id.try_into()
                        .map_err(|_| corrupt("selected generation width"))?,
                ),
                profile.unwrap_or(Digest(
                    p.ok_or_else(|| corrupt("selected profile missing"))?
                        .try_into()
                        .map_err(|_| corrupt("selected profile width"))?,
                )),
            )
        };
        let row:Option<(String,String)>=sqlx::query_as("SELECT g.canonical_manifest,p.canonical_policy FROM lctx_serving.generations g JOIN lctx_serving.retrieval_profiles p USING(generation_digest) WHERE g.generation_digest=$1 AND p.profile_digest=$2 AND g.state='ready'")
            .bind(id.0.as_slice()).bind(profile.0.as_slice()).fetch_optional(&mut *lease.connection).await?;
        let (raw, policy) =
            row.ok_or_else(|| Error::Request("generation/profile is not ready".into()))?;
        let manifest: Manifest =
            serde_json::from_str(&raw).map_err(|_| corrupt("stored manifest encoding"))?;
        manifest.validate()?;
        if manifest.generation()? != id.hex() || manifest.context.library != library {
            return Err(corrupt("pinned manifest identity/library").into());
        }
        let policy: Policy =
            serde_json::from_str(&policy).map_err(|_| corrupt("stored profile encoding"))?;
        policy.validate()?;
        if policy.digest()? != profile.hex() {
            return Err(corrupt("profile identity").into());
        }
        let artifacts = verify_locations(&mut lease.connection, &id, &manifest).await?;
        let pinned = PinnedGeneration {
            id,
            manifest,
            profile,
            policy,
            artifacts,
        };
        lease.complete();
        Ok(pinned)
    }
    pub async fn resolve(
        &self,
        generation: &PinnedGeneration,
        operation: &str,
    ) -> Result<ResolvedOperation, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let row = resolve_on(&mut lease.connection, generation, operation).await?;
        lease.complete();
        Ok(row)
    }
    pub async fn find_operations(
        &self,
        generation: &PinnedGeneration,
        selection: &cpg_schema::wire::Selection,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        let prepared = self.prepare_selection(generation, selection, "").await?;
        let generation = generation.clone();
        let cursor = cursor.map(str::to_owned);
        self.run_cpu(move || prepared.page(&generation, limit, cursor.as_deref()))
            .await
    }
    /// Exact eligibility and name promotion, without retaining a lease during embedding/fusion.
    pub async fn search_scope(
        &self,
        generation: &PinnedGeneration,
        query: &str,
    ) -> Result<Value, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let eligible: Vec<String> = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT brief_id FROM lctx_serving.briefs WHERE generation_digest=$1 ORDER BY brief_id",
        )
        .bind(generation.id.0.as_slice())
        .fetch_all(&mut *lease.connection)
        .await?
        .into_iter()
        .map(hex)
        .collect();
        let promoted: Vec<Vec<u8>> = sqlx::query_scalar("SELECT brief_id FROM lctx_serving.symbol_map WHERE generation_digest=$1 AND symbol=$2 ORDER BY brief_id").bind(generation.id.0.as_slice()).bind(query.trim()).fetch_all(&mut *lease.connection).await?;
        if eligible.len() > contract::MAX_RELATION_ROWS {
            return Err(refused("search eligibility row budget").into());
        }
        let allowed: BTreeSet<_> = eligible.iter().collect();
        let promoted: Vec<_> = promoted
            .into_iter()
            .map(hex)
            .filter(|id| allowed.contains(id))
            .collect();
        let result = json!({"eligible":eligible,"promoted":promoted});
        check_response(&result)?;
        lease.complete();
        Ok(result)
    }
}
pub(crate) async fn resolve_on(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    operation: &str,
) -> Result<ResolvedOperation, Error> {
    let spelling = operation.trim();
    if spelling.is_empty() || spelling.len() > 2000 {
        return Err(Error::Request("invalid operation spelling".into()));
    }
    let raw = Id::from_hex(spelling).map(|id| id.0.to_vec());
    let rows:Vec<(Vec<u8>,String)>=sqlx::query_as("WITH candidates AS (SELECT node_id,0 priority FROM lctx_serving.public_paths WHERE generation_digest=$1 AND access_path=$2 UNION ALL SELECT class_node_id,1 FROM lctx_serving.singletons WHERE generation_digest=$1 AND global=$2 UNION ALL SELECT node_id,2 FROM lctx_serving.operations WHERE generation_digest=$1 AND node_id=$3) SELECT o.node_id,o.access_path FROM candidates c JOIN lctx_serving.operations o ON o.generation_digest=$1 AND o.node_id=c.node_id ORDER BY c.priority,o.node_id LIMIT 2")
        .bind(generation.id.0.as_slice()).bind(spelling).bind(raw).fetch_all(conn).await?;
    let mut unique = rows;
    unique.dedup_by(|a, b| a.0 == b.0);
    if unique.len() > 1 {
        return Err(Error::Request(
            "ambiguous operation; use get_operation for public member choices".into(),
        ));
    }
    let (id, path) = unique.pop().ok_or_else(|| {
        Error::Request("no public operation with this spelling or identity".into())
    })?;
    Ok(ResolvedOperation {
        operation_id: hex(id),
        access_path: path,
    })
}
pub(crate) type Object = Map<String, Value>;
/// One request-wide ledger includes all selected support relations, not just each query.
pub(crate) struct Hydration {
    bytes: usize,
    response_bytes: usize,
}
pub(crate) enum Keys<'a> {
    Binary(&'a [Vec<u8>]),
    Text(&'a [String]),
    Ordinals(&'a [i64]),
}
impl Hydration {
    pub(crate) fn new() -> Self {
        Self {
            bytes: 0,
            response_bytes: 1024,
        }
    }
    pub(crate) fn charge_response(&mut self, value: impl Serialize) -> Result<(), Error> {
        struct Count<'a>(&'a mut usize);
        impl std::io::Write for Count<'_> {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                *self.0 += b.len();
                if *self.0 > RESPONSE_BYTES {
                    return Err(std::io::Error::other("response budget"));
                }
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(Count(&mut self.response_bytes), &value)
            .map_err(|_| refused("expanded response byte budget").into())
    }
    pub(crate) async fn fetch(
        &mut self,
        conn: &mut PgConnection,
        generation: &PinnedGeneration,
        name: &'static str,
        key: Option<&'static str>,
        ids: &[Vec<u8>],
    ) -> Result<Vec<Object>, Error> {
        self.fetch_keys(conn, generation, name, key, Keys::Binary(ids))
            .await
    }
    pub(crate) async fn fetch_keys(
        &mut self,
        conn: &mut PgConnection,
        generation: &PinnedGeneration,
        name: &'static str,
        key: Option<&'static str>,
        ids: Keys<'_>,
    ) -> Result<Vec<Object>, Error> {
        self.fetch_projection(conn, generation, name, key, ids, false)
            .await
    }
    /// Invocation projection omits prose at SQL read time; these rows are never persisted.
    pub(crate) async fn fetch_contract(
        &mut self,
        conn: &mut PgConnection,
        generation: &PinnedGeneration,
        name: &'static str,
        key: Option<&'static str>,
        ids: &[Vec<u8>],
    ) -> Result<Vec<Object>, Error> {
        self.fetch_projection(conn, generation, name, key, Keys::Binary(ids), true)
            .await
    }
    async fn fetch_projection(
        &mut self,
        conn: &mut PgConnection,
        generation: &PinnedGeneration,
        name: &'static str,
        key: Option<&'static str>,
        ids: Keys<'_>,
        invocation: bool,
    ) -> Result<Vec<Object>, Error> {
        let file = cpg_schema::bundle::files(generation.manifest.dimensions)
            .into_iter()
            .find(|f| f.name == name)
            .ok_or_else(|| corrupt("undeclared query relation"))?;
        if key.is_some_and(|k| k != "row_ordinal" && file.schema.field_with_name(k).is_err()) {
            return Err(corrupt("undeclared query key").into());
        }
        // Both identifiers originate in the executable inventory; data always uses parameters.
        let predicate = key.map_or(String::new(), |k| format!("AND \"{k}\"=ANY($2)"));
        let columns = if invocation {
            let omitted = match name {
                "catalog_signatures" => "docstring",
                "catalog_parameters" => "documentation",
                _ => return Err(corrupt("undeclared invocation projection").into()),
            };
            file.schema
                .fields()
                .iter()
                .map(|f| {
                    if f.name() == omitted {
                        format!("NULL::text AS \"{omitted}\"")
                    } else {
                        format!("\"{}\"", f.name())
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            "*".into()
        };
        let sql = format!(
            "SELECT {columns} FROM lctx_serving.\"{name}\" WHERE generation_digest=$1 {predicate} ORDER BY row_ordinal LIMIT 200001"
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(generation.id.0.as_slice());
        if key.is_some() {
            query = match ids {
                Keys::Binary(ids) => query.bind(ids),
                Keys::Text(ids) => query.bind(ids),
                Keys::Ordinals(ids) => query.bind(ids),
            };
        }
        let mut stream = query.fetch(&mut *conn);
        let mut rows = Vec::new();
        let mut result = Vec::new();
        while let Some(row) = stream.try_next().await? {
            self.bytes += 256 + row.columns().len() * 192;
            for i in 0..row.columns().len() {
                let raw = row.try_get_raw(i)?;
                if !raw.is_null() {
                    self.bytes += raw
                        .as_bytes()
                        .map_err(|_| corrupt("row transport bytes"))?
                        .len()
                        * 3;
                }
            }
            if self.bytes > HYDRATION_BYTES {
                return Err(refused("hydration byte budget").into());
            }
            rows.push(row);
            if rows.len() == 200 {
                result.extend(objects(projection::decode_rows(
                    name,
                    generation.manifest.dimensions,
                    &rows,
                )?)?);
                rows.clear();
            }
            if result.len() + rows.len() > contract::MAX_RELATION_ROWS {
                return Err(refused("hydration row budget").into());
            }
        }
        if !rows.is_empty() {
            result.extend(objects(projection::decode_rows(
                name,
                generation.manifest.dimensions,
                &rows,
            )?)?);
        }
        Ok(result)
    }
}
fn objects(batch: arrow_array::RecordBatch) -> Result<Vec<Object>, Error> {
    let mut out = vec![Map::new(); batch.num_rows()];
    for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
        for (i, row) in out.iter_mut().enumerate() {
            let value = if array.is_null(i) {
                Value::Null
            } else {
                match field.data_type() {
                    DataType::Utf8 => json!(
                        array
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .ok_or_else(|| corrupt("text decode"))?
                            .value(i)
                    ),
                    DataType::Int64 => json!(
                        array
                            .as_any()
                            .downcast_ref::<Int64Array>()
                            .ok_or_else(|| corrupt("integer decode"))?
                            .value(i)
                    ),
                    DataType::Boolean => json!(
                        array
                            .as_any()
                            .downcast_ref::<BooleanArray>()
                            .ok_or_else(|| corrupt("boolean decode"))?
                            .value(i)
                    ),
                    DataType::FixedSizeBinary(_) => json!(hex(array
                        .as_any()
                        .downcast_ref::<FixedSizeBinaryArray>()
                        .ok_or_else(|| corrupt("identity decode"))?
                        .value(i))),
                    _ => return Err(corrupt("unsupported response column").into()),
                }
            };
            row.insert(field.name().clone(), value);
        }
    }
    Ok(out)
}
pub(crate) fn check_response(value: &Value) -> Result<(), Error> {
    // Bound serialization without allocating a second full response buffer.
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 += b.len();
            if self.0 > RESPONSE_BYTES {
                return Err(std::io::Error::other("response budget"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(0), value)
        .map_err(|_| refused("serialized response byte budget").into())
}
