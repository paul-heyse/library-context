//! Bounded read-only operational observations, including incompatible schemas.
use crate::{
    Error, admission,
    import::verify_locations,
    profiles::{Policy, hex},
    repository::PinnedGeneration,
    serving::{QueryLease, RoleConfig, ServingStore, check_connection},
};
use cpg_schema::{
    Digest,
    serving_projection::{FailureKind, Manifest, corrupt},
};
use serde::Serialize;
use serde_json::Value;
use sqlx::{Connection, PgConnection};

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Diagnostics {
    pub format: u32,
    pub database: &'static str,
    pub role: Option<String>,
    pub server: Option<String>,
    pub extension: Option<String>,
    pub schema_current: bool,
    pub failure: Option<String>,
    pub pool: ConfiguredPool,
    pub generation: Option<GenerationDiagnostic>,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ConfiguredPool {
    pub configured_total: u32,
    pub provider_reserved: u32,
    pub probe_connections: u32,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LivePool {
    pub connections: u32,
    pub idle: usize,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LiveDiagnostics {
    pub format: u32,
    pub generation: GenerationDiagnostic,
    pub pool: LivePool,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum GenerationDiagnostic {
    NotVisible {
        generation: String,
        publication: &'static str,
        availability: &'static str,
    },
    Visible(Box<VisibleGeneration>),
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProfileDiagnostic {
    pub profile: String,
    pub route: crate::profiles::Route,
    pub available: bool,
    pub admission: Option<String>,
    pub failure: Option<String>,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SelectionDiagnostic {
    pub library: String,
    pub profile: String,
}
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VisibleGeneration {
    pub generation: String,
    pub publication: String,
    pub availability: String,
    pub artifact_failure: Option<String>,
    pub spec: Option<String>,
    pub dimensions: i32,
    pub relations: std::collections::BTreeMap<String, u64>,
    pub artifact_count: usize,
    /// PostgreSQL function-owned administrative payload; intentionally opaque to this Rust DTO.
    pub import: Value,
    pub selections: Vec<SelectionDiagnostic>,
    pub profiles: Vec<ProfileDiagnostic>,
    pub vector_partition_bytes: i64,
}
impl RoleConfig {
    pub async fn diagnose(&self, id: Option<Digest>, verify_artifacts: bool) -> Diagnostics {
        let mut out = Diagnostics {
            format: 1,
            database: "unavailable",
            role: None,
            server: None,
            extension: None,
            schema_current: false,
            failure: None,
            pool: ConfiguredPool {
                configured_total: self.max_connections,
                provider_reserved: self.provider_connections,
                probe_connections: 1,
            },
            generation: None,
        };
        let result=async {
            self.validate()?;
            let mut conn=tokio::time::timeout(std::time::Duration::from_secs(self.acquire_timeout_seconds),PgConnection::connect_with(&self.options()?)).await
                .map_err(|_|Error::Database{kind:"diagnostic connection timed out",code:"none".into()})??;
            sqlx::raw_sql("SET default_transaction_read_only=on").execute(&mut conn).await?;
            out.database="available";
            let (role,server,extension):(String,String,Option<String>)=sqlx::query_as("SELECT current_user::text,current_setting('server_version_num'),(SELECT extversion::text FROM pg_extension WHERE extname='vector')").fetch_one(&mut conn).await?;
            out.role=Some(role);out.server=Some(server);out.extension=extension;
            let check=check_connection(&mut conn).await;
            out.schema_current=check.is_ok();
            if let Err(e)=check {out.failure=Some(e.to_string());}
            if out.schema_current && let Some(id)=id {
                let mut tx=conn.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY").await?;
                out.generation=Some(generation_on(&mut tx,id,verify_artifacts).await?);
                tx.commit().await?;
            }
            conn.close().await?;
            Ok::<_,Error>(())
        }.await;
        if let Err(e) = result {
            out.failure = Some(e.to_string());
        }
        out
    }
}
impl ServingStore {
    pub async fn diagnostics(&self, id: Digest, verify_artifacts: bool) -> Result<Value, Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut tx = lease
            .connection
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .await?;
        let generation = generation_on(&mut tx, id, verify_artifacts).await?;
        tx.commit().await?;
        lease.complete();
        serde_json::to_value(LiveDiagnostics {
            format: 1,
            generation,
            pool: LivePool {
                connections: self.pool.size(),
                idle: self.pool.num_idle(),
            },
        })
        .map_err(|_| corrupt("diagnostic encoding").into())
    }
}
async fn generation_on(
    conn: &mut PgConnection,
    id: Digest,
    verify: bool,
) -> Result<GenerationDiagnostic, Error> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT state,canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1",
    )
    .bind(id.0.as_slice())
    .fetch_optional(&mut *conn)
    .await?;
    let Some((state, raw)) = row else {
        return Ok(GenerationDiagnostic::NotVisible {
            generation: id.hex(),
            publication: "not_visible",
            availability: "unchecked",
        });
    };
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|_| corrupt("diagnostic manifest"))?;
    manifest.validate()?;
    if manifest.generation()? != id.hex() {
        return Err(corrupt("diagnostic manifest identity").into());
    }
    let mut availability = "unchecked";
    let mut failure = None;
    if verify {
        match verify_locations(conn, &id, &manifest).await {
            Ok(_) => availability = "available",
            Err(e) => {
                availability = match &e {
                    Error::Projection(p) if p.kind == FailureKind::Unavailable => "missing",
                    Error::Projection(p) if p.kind == FailureKind::Corrupt => "corrupt",
                    _ => "unavailable",
                };
                failure = Some(e.to_string());
            }
        }
    }
    let selections:Vec<(String,Vec<u8>)>=sqlx::query_as("SELECT library,profile_digest FROM lctx_serving.selections WHERE generation_digest=$1 ORDER BY library LIMIT 51").bind(id.0.as_slice()).fetch_all(&mut *conn).await?;
    let profiles:Vec<(Vec<u8>,String)>=sqlx::query_as("SELECT profile_digest,canonical_policy FROM lctx_serving.retrieval_profiles WHERE generation_digest=$1 ORDER BY profile_digest LIMIT 51").bind(id.0.as_slice()).fetch_all(&mut *conn).await?;
    if selections.len() > 50 || profiles.len() > 50 {
        return Err(cpg_schema::serving_projection::refused("diagnostic profile budget").into());
    }
    let mut admissions = Vec::new();
    for (profile, raw) in profiles {
        let policy: Policy =
            serde_json::from_str(&raw).map_err(|_| corrupt("diagnostic policy"))?;
        let pinned = PinnedGeneration {
            id,
            manifest: manifest.clone(),
            profile: Digest(profile.try_into().map_err(|_| corrupt("profile digest"))?),
            policy,
            artifacts: Vec::new(),
            vector_population: None,
        };
        let check = async {
            pinned.policy.validate()?;
            if pinned.policy.digest()? != pinned.profile.hex() {
                return Err(corrupt("diagnostic policy digest").into());
            }
            admission::check(conn, &pinned).await
        }
        .await;
        admissions.push(ProfileDiagnostic {
            profile: pinned.profile.hex(),
            route: pinned.policy.route,
            available: check.is_ok(),
            admission: check.as_ref().ok().cloned().flatten(),
            failure: check.err().map(|e| e.to_string()),
        });
    }
    let imports: Value = sqlx::query_scalar("SELECT lctx_serving.import_diagnostics($1)")
        .bind(id.0.as_slice())
        .fetch_one(&mut *conn)
        .await?;
    let storage:i64=sqlx::query_scalar("SELECT coalesce(sum(pg_total_relation_size(c.oid)),0)::bigint FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='lctx_serving' AND c.relkind='r' AND (c.relname LIKE 'v_'||$1||'%' OR c.relname LIKE 'o_'||$1||'%')").bind(&id.hex()[..48]).fetch_one(&mut *conn).await?;
    Ok(GenerationDiagnostic::Visible(Box::new(VisibleGeneration {
        generation: id.hex(),
        publication: state,
        availability: availability.into(),
        artifact_failure: failure,
        spec: manifest.spec_hash,
        dimensions: manifest.dimensions,
        relations: manifest
            .relations
            .iter()
            .map(|(n, r)| (n.clone(), r.rows))
            .collect(),
        artifact_count: manifest.artifacts.len(),
        import: imports,
        selections: selections
            .into_iter()
            .map(|(library, p)| SelectionDiagnostic {
                library,
                profile: hex(p),
            })
            .collect(),
        profiles: admissions,
        vector_partition_bytes: storage,
    })))
}
