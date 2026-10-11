//! Installed native declarations and maintenance authority, independent of compiler orchestration.
use crate::{Loader, RuntimeConfig, reader};
use lctx_model::domain::{self as d, ContentHash, ModelError};
use std::sync::Arc;
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, RecordId, Value, Variables}};

pub fn compiler_schema() -> String {
    "DEFINE TABLE compiler_contribution SCHEMAFULL; DEFINE FIELD spec ON compiler_contribution TYPE bytes; DEFINE FIELD descriptor ON compiler_contribution TYPE option<bytes>; DEFINE FIELD completed ON compiler_contribution TYPE bool; DEFINE FIELD logical ON compiler_contribution TYPE option<string>; DEFINE INDEX logical_contribution ON compiler_contribution FIELDS logical; DEFINE FIELD attempt ON compiler_contribution TYPE record<native_attempt>; DEFINE INDEX attempt_contributions ON compiler_contribution FIELDS attempt; DEFINE TABLE compiler_membership SCHEMAFULL; DEFINE FIELD contribution ON compiler_membership TYPE record<compiler_contribution>; DEFINE FIELD relation ON compiler_membership TYPE string; DEFINE FIELD semantic_key ON compiler_membership TYPE string; DEFINE FIELD node ON compiler_membership TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_membership TYPE string; DEFINE INDEX contribution_rows ON compiler_membership FIELDS contribution,relation,semantic_key UNIQUE; DEFINE INDEX member_keys ON compiler_membership FIELDS relation,semantic_key,contribution; DEFINE TABLE compiler_view SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view TYPE bytes; DEFINE TABLE compiler_binding SCHEMAFULL; DEFINE FIELD id ON compiler_binding TYPE string; DEFINE FIELD descriptor ON compiler_binding TYPE bytes; DEFINE FIELD attempt ON compiler_binding TYPE record<native_attempt>; DEFINE TABLE compiler_alias SCHEMAFULL; DEFINE FIELD source ON compiler_alias TYPE record<entity>; DEFINE FIELD target ON compiler_alias TYPE record<entity>; DEFINE INDEX alias_source ON compiler_alias FIELDS source,target UNIQUE; DEFINE INDEX alias_target ON compiler_alias FIELDS target,source;".to_string()+"DEFINE FIELD producer ON compiler_contribution TYPE string; DEFINE FIELD profile ON compiler_contribution TYPE string; DEFINE FIELD model ON compiler_contribution TYPE string; DEFINE FIELD implementation ON compiler_contribution TYPE string; DEFINE FIELD configuration ON compiler_contribution TYPE option<string|null>; DEFINE FIELD inputs ON compiler_contribution TYPE array<record<compiler_view>>; DEFINE FIELD outputs ON compiler_contribution TYPE array<string>; DEFINE FIELD outcome ON compiler_contribution TYPE option<int>; DEFINE INDEX contribution_inputs ON compiler_contribution FIELDS inputs; DEFINE FIELD relation ON compiler_view TYPE string; DEFINE FIELD contributions ON compiler_view TYPE array<string>; DEFINE FIELD rows ON compiler_view TYPE int; DEFINE FIELD relation ON compiler_binding TYPE string; DEFINE FIELD boundary ON compiler_binding TYPE option<string|null>; DEFINE FIELD view ON compiler_binding TYPE record<compiler_view>; DEFINE INDEX binding_view ON compiler_binding FIELDS view; DEFINE INDEX binding_attempt ON compiler_binding FIELDS attempt,id;"+&crate::schema::compiler_record_schema()
}

/// Explicit maintenance installation only. Ordinary compiler attachment never defines schema.
pub async fn install_shared(
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native installation requires installer authority".into(),
        ));
    }
    let client =
        reader::authenticated(&config.endpoint, &config.writer_credentials(), None).await?;
    client
        .query(format!(
            "DEFINE NAMESPACE IF NOT EXISTS `{}`",
            config.namespace.as_str()
        ))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client
        .use_ns(config.namespace.as_str())
        .await
        .map_err(ModelError::codec)?;
    client
        .query(format!(
            "DEFINE DATABASE IF NOT EXISTS `{}` STRICT",
            config.database.as_str()
        ))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client
        .use_db(config.database.as_str())
        .await
        .map_err(ModelError::codec)?;
    // Installation retries may fill missing declarations but cannot stamp a new meaning
    // onto an older installation, or reset its accumulated authorization watermarks.
    let installed = match client
        .query("SELECT * FROM native_installation:current")
        .await
    {
        Ok(response) => match response.check() {
            Ok(mut response) => Some(response.take::<Vec<Object>>(0).map_err(ModelError::codec)?),
            Err(error)
                if matches!(
                    error.not_found_details(),
                    Some(surrealdb::types::NotFoundError::Table { .. })
                ) =>
            {
                None
            }
            Err(error) => return Err(ModelError::codec(error)),
        },
        Err(error) => return Err(ModelError::codec(error)),
    };
    let fresh = installed.as_ref().is_none_or(Vec::is_empty);
    if !fresh {
        crate::control::check_installation(&client, config.service_generation).await?;
        if installed
            .as_ref()
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("admission_open"))
            != Some(&Value::Bool(false))
        {
            return Err(ModelError::Conflict(
                "native installation requires closed admission",
            ));
        }
    }
    let loader = Loader::new(client.clone());
    let mut catalog = loader.installation_catalog().await?;
    catalog
        .apply(crate::control::schema(), "native durable control schema")
        .await?;
    catalog.install(native_definitions).await?;
    catalog
        .apply(&compiler_schema(), "compiler shared state schema")
        .await?;
    catalog
        .apply(VIEW_SCHEMA, "exact completed view mapping schema")
        .await?;
    let mut bindings = Variables::new();
    bindings.insert("generation", config.service_generation.hex());
    bindings.insert("schema", base_schema_identity().hex());
    if fresh {
        bindings.insert("version", crate::control::SCHEMA_VERSION);
        client.query("CREATE native_installation:current SET generation=$generation,schema=$schema,schema_version=$version,admission_open=false,era=1,closed_through=0,control_revision=0 RETURN NONE").bind(bindings).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    }
    Ok(())
}
pub async fn check_installation(
    config: &RuntimeConfig,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let checked = async {
        let version = client
            .version()
            .await
            .map_err(ModelError::codec)?
            .to_string();
        if !version.starts_with("3.3.0") {
            return Err(ModelError::Invalid(
                "shared native store requires pinned SurrealDB 3.3.0".into(),
            ));
        }
        crate::control::check_installation(&client, config.service_generation).await
    }
    .await;
    if let Err(error) = checked {
        let mut completion = d::completion::Completion::default();
        completion.step(
            "failed installation check session invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        return d::completion::complete(Err(error), completion);
    }
    Ok(client)
}
pub fn base_schema_identity() -> ContentHash {
    static IDENTITY: std::sync::OnceLock<ContentHash> = std::sync::OnceLock::new();
    *IDENTITY.get_or_init(|| {
        ContentHash::of(
            format!(
                "{}{}{}{}",
                crate::schema::canonical_schema(),
                compiler_schema(),
                crate::control::schema(),
                VIEW_SCHEMA
            )
            .as_bytes(),
        )
    })
}
pub async fn close_admission(config: &RuntimeConfig) -> Result<(), ModelError> {
    maintenance_admission(config, false).await
}
pub async fn open_admission(config: &RuntimeConfig) -> Result<(), ModelError> {
    maintenance_admission(config, true).await
}
async fn maintenance_admission(config: &RuntimeConfig, open: bool) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance requires installer authority".into(),
        ));
    }
    let client = check_installation(config).await?;
    if open {
        crate::control::check_pending_effects(&client).await?;
    }
    let mut bindings = Variables::new();
    bindings.insert("open", open);
    crate::control::effect(&client, None, "UPDATE native_installation:current SET admission_open=$open,admission_revision=(admission_revision ?? 0)+1 RETURN NONE", bindings).await?;
    Ok(())
}
pub async fn drain_installation(config: &RuntimeConfig) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance requires installer authority".into(),
        ));
    }
    let client = check_installation(config).await?;
    let mut response = client
        .query("SELECT VALUE admission_open FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let open: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    if open != vec![false] {
        return Err(ModelError::Conflict(
            "maintenance admission must close before drainage",
        ));
    }
    loop {
        let mut response = client
            .query("SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state='open' LIMIT 128")
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let attempts: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        if attempts.is_empty() {
            break;
        }
        crate::control::effect(
            &client,
            None,
            "UPDATE $attempts SET state='closing',revision+=1 RETURN NONE",
            {
                let mut bindings = Variables::new();
                bindings.insert("attempts", attempts);
                bindings
            },
        )
        .await?;
    }
    crate::control::drain_effects(&client).await?;
    loop {
        let mut response=client.query("SELECT VALUE id FROM native_attempt WITH INDEX live_attempts WHERE state='closing' LIMIT 128").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let attempts: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        if attempts.is_empty() {
            break;
        }
        for attempt in attempts {
            let surrealdb::types::RecordIdKey::String(key) = attempt.key else {
                return Err(ModelError::Schema("native closing attempt identity"));
            };
            let identity = ContentHash(
                hex::decode(key)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("native closing attempt width"))?,
            );
            crate::control::close_attempt(&client, identity, "abandoned").await?;
        }
    }
    let mut response=client.query("SELECT VALUE id FROM native_pin WITH INDEX live_pins WHERE released=false LIMIT 1; SELECT VALUE id FROM native_backup_hold WITH INDEX live_backups WHERE active=true LIMIT 1").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let pins: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    let backups: Vec<RecordId> = response.take(1).map_err(ModelError::codec)?;
    if !pins.is_empty() || !backups.is_empty() {
        return Err(ModelError::Conflict(
            "native readers or backup holds remain live",
        ));
    }
    Ok(())
}
/// Explicit recovery of named abandoned clients after the service owner proves actual
/// host drainage. Neither identity age nor an empty view list establishes that proof.
/// Successful per-identity fences are durable if a later identity fails; retry the same list.
pub async fn reconcile_maintenance(
    config: &RuntimeConfig,
    pins: &[ContentHash],
    backup_holds: &[ContentHash],
    readers_stopped: bool,
) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance reconciliation requires installer authority".into(),
        ));
    }
    if !readers_stopped {
        return Err(ModelError::Conflict(
            "maintenance reconciliation requires stopped readers",
        ));
    }
    let client = check_installation(config).await?;
    let mut response = client
        .query("SELECT VALUE admission_open FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let open: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    if open != vec![false] {
        return Err(ModelError::Conflict(
            "maintenance admission must close before reconciliation",
        ));
    }
    crate::control::drain_effects(&client).await?;
    let mut completion = lctx_model::domain::completion::Completion::default();
    let result = async {
        for pin in pins {
            crate::control::release_pin_checked(&client, *pin, true).await?;
            completion.committed("reconciled native reader pin", pin.hex());
        }
        for hold in backup_holds {
            crate::control::release_backup_hold_checked(&client, *hold, true).await?;
            completion.committed("reconciled native backup hold", hold.hex());
        }
        Ok(())
    }
    .await;
    lctx_model::domain::completion::complete(result, completion)
}
pub const VIEW_SCHEMA: &str = "DEFINE TABLE compiler_view_member SCHEMAFULL; DEFINE FIELD view ON compiler_view_member TYPE record<compiler_view>; DEFINE FIELD relation ON compiler_view_member TYPE string; DEFINE FIELD semantic_key ON compiler_view_member TYPE string; DEFINE FIELD node ON compiler_view_member TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_view_member TYPE string; DEFINE INDEX view_key ON compiler_view_member FIELDS view,relation,semantic_key UNIQUE; DEFINE INDEX view_nodes ON compiler_view_member FIELDS view,node;";
