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
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use cpg_schema::{
    id::{Digest, Id},
    serving_projection::{self as contract, Manifest, corrupt, refused},
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};
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
    pub(crate) vector_population: Option<u64>,
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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Where {
    #[serde(default)]
    pub facets: Vec<FacetTerm>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub path_prefix: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacetTerm {
    pub facet: String,
    pub value: String,
}
impl Where {
    pub fn validate(&self) -> Result<(), Error> {
        use cpg_schema::codebook::{Codebook, OperationFacet};
        if self.facets.len() > 10
            || self.path_prefix.as_ref().is_some_and(|s| s.len() > 2000)
            || self
                .kind
                .as_ref()
                .is_some_and(|s| !matches!(s.as_str(), "function" | "method" | "class"))
            || self.facets.iter().any(|t| {
                t.value.is_empty()
                    || t.value.len() > 2000
                    || !OperationFacet::all().iter().any(|f| f.text() == t.facet)
            })
        {
            return Err(Error::Request("invalid operation filter".into()));
        }
        Ok(())
    }
    fn hash(&self) -> Result<String, Error> {
        Ok(hex(Sha256::digest(
            serde_json::to_vec(self).map_err(|_| corrupt("filter encoding"))?,
        )))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    g: String,
    h: String,
    o: u64,
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
        let vector_population = if policy.route == crate::profiles::Route::Mixed {
            Some(crate::admission::universe(&mut lease.connection, id).await?)
        } else {
            None
        };
        let pinned = PinnedGeneration {
            id,
            manifest,
            profile,
            policy,
            artifacts,
            vector_population,
        };
        crate::admission::check(&mut lease.connection, &pinned).await?;
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
        filter: &Where,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        filter.validate()?;
        if !(1..=50).contains(&limit) {
            return Err(Error::Request("limit must be 1 through 50".into()));
        }
        let hash = filter.hash()?;
        let offset = if let Some(cursor) = cursor {
            if cursor.len() > 2048 {
                return Err(Error::Request("invalid cursor".into()));
            }
            let value: Cursor = serde_json::from_slice(
                &URL_SAFE
                    .decode(cursor)
                    .map_err(|_| Error::Request("invalid cursor".into()))?,
            )
            .map_err(|_| Error::Request("invalid cursor".into()))?;
            if value.g != generation.id.hex()
                || value.h != hash
                || value.o > contract::MAX_RELATION_ROWS as u64
            {
                return Err(Error::Request(
                    "cursor belongs to another generation or request".into(),
                ));
            }
            value.o
        } else {
            0
        };
        let mut lease = QueryLease::acquire(&self.pool).await?;
        check_terms(&mut lease.connection, generation, filter).await?;
        let rows = classify(&mut lease.connection, generation, filter).await?;
        let mut matches = Vec::new();
        let mut unknown = Vec::new();
        let mut total = 0;
        let mut unknown_total = 0;
        for row in rows {
            let matched: bool = row.try_get("matched")?;
            let opened: bool = row.try_get("opened")?;
            if matched {
                if total >= offset && matches.len() < (limit as usize) {
                    matches.push(row.try_get::<Vec<u8>, _>("node_id")?);
                }
                total += 1;
            } else if opened {
                unknown_total += 1;
                if unknown.len() < 50 {
                    unknown.push(row.try_get::<Vec<u8>, _>("node_id")?);
                }
            }
        }
        let next = offset + matches.len() as u64;
        let more = next < total;
        let next_cursor = if more {
            Some(
                URL_SAFE.encode(
                    serde_json::to_vec(&Cursor {
                        g: generation.id.hex(),
                        h: hash,
                        o: next,
                    })
                    .map_err(|_| corrupt("cursor encoding"))?,
                ),
            )
        } else {
            None
        };
        let matches = operation_refs(&mut lease.connection, generation, &matches).await?;
        let unknown = operation_refs(&mut lease.connection, generation, &unknown).await?;
        let result = json!({"snapshot_id":generation.manifest.snapshot_id,"generation":generation.id.hex(),"matches":matches,"total":total,"complete":unknown_total==0,"unknown":unknown,"unknown_total":unknown_total,"unknown_truncated":unknown_total>50,"truncated":more,"next_cursor":next_cursor});
        check_response(&result)?;
        lease.complete();
        Ok(result)
    }
    /// Exact eligibility and name promotion, without retaining a lease during embedding/fusion.
    pub async fn search_scope(
        &self,
        generation: &PinnedGeneration,
        filter: Option<&Where>,
        query: &str,
        operations: bool,
    ) -> Result<Value, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut eligible = Vec::new();
        let promoted: Vec<Vec<u8>>;
        if operations {
            let filter = filter.cloned().unwrap_or_default();
            filter.validate()?;
            check_terms(&mut lease.connection, generation, &filter).await?;
            for row in classify(&mut lease.connection, generation, &filter).await? {
                if row.try_get::<bool, _>("matched")? {
                    eligible.push(hex(row.try_get::<Vec<u8>, _>("node_id")?));
                }
            }
            promoted=sqlx::query_scalar("SELECT node_id FROM lctx_serving.public_paths WHERE generation_digest=$1 AND access_path=$2").bind(generation.id.0.as_slice()).bind(query.trim()).fetch_all(&mut *lease.connection).await?;
        } else {
            eligible=sqlx::query_scalar::<_,Vec<u8>>("SELECT brief_id FROM lctx_serving.briefs WHERE generation_digest=$1 ORDER BY brief_id").bind(generation.id.0.as_slice()).fetch_all(&mut *lease.connection).await?.into_iter().map(hex).collect();
            promoted=sqlx::query_scalar("SELECT brief_id FROM lctx_serving.symbol_map WHERE generation_digest=$1 AND symbol=$2 ORDER BY brief_id").bind(generation.id.0.as_slice()).bind(query.trim()).fetch_all(&mut *lease.connection).await?;
        }
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
fn operation_ref(row: &sqlx::postgres::PgRow) -> Result<Value, Error> {
    Ok(
        json!({"operation_id":hex(row.try_get::<Vec<u8>,_>("node_id")?),"access_path":row.try_get::<String,_>("access_path")?,"kind":row.try_get::<String,_>("kind")?,"docstring_summary":row.try_get::<Option<String>,_>("docstring_summary")?,"behavior_status":row.try_get::<String,_>("behavior_status")?}),
    )
}
async fn operation_refs(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    ids: &[Vec<u8>],
) -> Result<Vec<Value>, Error> {
    let mut stream=sqlx::query("SELECT node_id,access_path,kind,docstring_summary,behavior_status FROM lctx_serving.operations WHERE generation_digest=$1 AND node_id=ANY($2) ORDER BY access_path COLLATE \"C\",node_id").bind(generation.id.0.as_slice()).bind(ids).fetch(conn);
    let mut out = Vec::new();
    let mut bytes = 0usize;
    while let Some(row) = stream.try_next().await? {
        let value = operation_ref(&row)?;
        bytes += serde_json::to_vec(&value)
            .map_err(|_| corrupt("operation reference serialization"))?
            .len();
        if bytes > RESPONSE_BYTES {
            return Err(refused("operation page response byte budget").into());
        }
        out.push(value);
    }
    Ok(out)
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
async fn check_terms(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    filter: &Where,
) -> Result<(), Error> {
    for term in &filter.facets {
        let invalid:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT FROM lctx_serving.operation_facets WHERE generation_digest=$1 AND facet=$2 AND value=$3) AND NOT EXISTS(SELECT FROM lctx_serving.operations o WHERE generation_digest=$1 AND NOT EXISTS(SELECT FROM lctx_serving.operation_facet_status s WHERE s.generation_digest=$1 AND s.node_id=o.node_id AND s.facet=$2 AND s.verdict='established'))")
            .bind(generation.id.0.as_slice()).bind(&term.facet).bind(&term.value).fetch_one(&mut *conn).await?;
        if invalid {
            return Err(Error::Request(format!(
                "no operation has {} = {:?} in this generation",
                term.facet, term.value
            )));
        }
    }
    Ok(())
}
async fn classify(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    filter: &Where,
) -> Result<Vec<sqlx::postgres::PgRow>, Error> {
    let facets: Vec<_> = filter.facets.iter().map(|t| t.facet.clone()).collect();
    let values: Vec<_> = filter.facets.iter().map(|t| t.value.clone()).collect();
    let rows=sqlx::query("WITH terms AS (SELECT * FROM unnest($4::text[],$5::text[]) AS t(facet,value)) SELECT o.node_id, NOT EXISTS(SELECT FROM terms t WHERE NOT EXISTS(SELECT FROM lctx_serving.operation_facets f WHERE f.generation_digest=$1 AND f.node_id=o.node_id AND f.facet=t.facet AND f.value=t.value AND f.verdict IN ('established','conditional'))) matched, NOT EXISTS(SELECT FROM terms t WHERE NOT EXISTS(SELECT FROM lctx_serving.operation_facets f WHERE f.generation_digest=$1 AND f.node_id=o.node_id AND f.facet=t.facet AND f.value=t.value) AND EXISTS(SELECT FROM lctx_serving.operation_facet_status s WHERE s.generation_digest=$1 AND s.node_id=o.node_id AND s.facet=t.facet AND s.verdict='established')) opened FROM lctx_serving.operations o WHERE o.generation_digest=$1 AND ($2::text IS NULL OR EXISTS(SELECT FROM lctx_serving.operation_facets k WHERE k.generation_digest=$1 AND k.node_id=o.node_id AND k.facet='kind' AND k.value=$2)) AND ($3::text IS NULL OR EXISTS(SELECT FROM lctx_serving.public_paths p WHERE p.generation_digest=$1 AND p.node_id=o.node_id AND left(p.access_path,length($3))=$3)) ORDER BY o.access_path COLLATE \"C\",o.node_id LIMIT 200001")
        .bind(generation.id.0.as_slice()).bind(&filter.kind).bind(&filter.path_prefix).bind(facets).bind(values).fetch_all(conn).await?;
    if rows.len() > contract::MAX_RELATION_ROWS {
        return Err(refused("operation universe row budget").into());
    }
    Ok(rows)
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
    Prefixes(&'a [String]),
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
        let file = cpg_schema::bundle::files(generation.manifest.dimensions)
            .into_iter()
            .find(|f| f.name == name)
            .ok_or_else(|| corrupt("undeclared query relation"))?;
        if key.is_some_and(|k| file.schema.field_with_name(k).is_err()) {
            return Err(corrupt("undeclared query key").into());
        }
        // Both identifiers originate in the executable inventory; data always uses parameters.
        let predicate = key.map_or(String::new(), |k| {
            if matches!(ids, Keys::Prefixes(_)) {
                format!("AND EXISTS(SELECT FROM unnest($2::text[]) p WHERE starts_with(\"{k}\",p))")
            } else {
                format!("AND \"{k}\"=ANY($2)")
            }
        });
        let sql = format!(
            "SELECT * FROM lctx_serving.\"{name}\" WHERE generation_digest=$1 {predicate} ORDER BY row_ordinal LIMIT 200001"
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(generation.id.0.as_slice());
        if key.is_some() {
            query = match ids {
                Keys::Binary(ids) => query.bind(ids),
                Keys::Text(ids) | Keys::Prefixes(ids) => query.bind(ids),
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
