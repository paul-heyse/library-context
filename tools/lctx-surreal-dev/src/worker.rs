//! Host-issued disposable campaign binding. Parent ownership remains authoritative.
use anyhow::{Context, Result, bail};
use lctx_native::{AuthenticationScope, RuntimeConfig};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use surrealdb::{
    Surreal,
    engine::any::{self, Any},
    opt::{Config, auth::Root},
    types::{Number, Object, Value},
};

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetBinding {
    pub format: u32,
    pub purpose: String,
    pub target_kind: String,
    pub installation_id: String,
    pub server_generation: String,
    pub runtime_config: PathBuf,
    pub namespace: String,
    pub database: String,
    pub allowed_operations: Vec<String>,
    pub project_dir: PathBuf,
    pub report_path: PathBuf,
}

impl TargetBinding {
    pub fn read(path: &Path, operation: &str) -> Result<(Self, RuntimeConfig)> {
        let binding: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        binding.validate(operation)?;
        if binding
            .runtime_config
            .file_name()
            .and_then(|name| name.to_str())
            != Some("validation-installer.json")
            || std::fs::symlink_metadata(&binding.runtime_config)?
                .file_type()
                .is_symlink()
        {
            bail!("campaign requires its physical validation installer configuration");
        }
        let runtime = RuntimeConfig::read(&binding.runtime_config)?;
        if runtime.authentication != AuthenticationScope::Root
            || runtime.service_generation.hex() != binding.server_generation
            || runtime.namespace.as_str() != binding.namespace
            || runtime.database.as_str() != "validation"
        {
            bail!("campaign binding does not match its installer configuration");
        }
        let descriptor = binding
            .runtime_config
            .parent()
            .context("installer parent")?
            .join("installation.json");
        if std::fs::symlink_metadata(&descriptor)?
            .file_type()
            .is_symlink()
        {
            bail!("installation descriptor cannot be a symlink");
        }
        let installation: serde_json::Value = serde_json::from_slice(&std::fs::read(descriptor)?)?;
        if installation.get("kind").and_then(serde_json::Value::as_str) != Some("campaign")
            || installation
                .get("purpose")
                .and_then(serde_json::Value::as_str)
                != Some("surrealdb-lifecycle")
            || installation
                .get("installation_id")
                .and_then(serde_json::Value::as_str)
                != Some(&binding.installation_id)
            || installation
                .get("phase")
                .and_then(serde_json::Value::as_str)
                != Some("ready")
            || installation
                .get("grpc_endpoint")
                .and_then(serde_json::Value::as_str)
                != Some(&runtime.endpoint)
            || installation
                .get("namespace")
                .and_then(serde_json::Value::as_str)
                != Some(&binding.namespace)
            || installation.get("service_generation")
                != Some(&serde_json::to_value(runtime.service_generation)?)
        {
            bail!("binding does not match the lifecycle campaign owner descriptor");
        }
        Ok((binding, runtime))
    }

    pub fn validate(&self, operation: &str) -> Result<()> {
        if self.format != 1
            || self.purpose != "surrealdb-lifecycle"
            || self.target_kind != "campaign"
        {
            bail!("worker requires a lifecycle-owned campaign binding");
        }
        if !matches!(operation, "sync" | "test")
            || !self
                .allowed_operations
                .iter()
                .any(|allowed| allowed == operation)
        {
            bail!("campaign binding does not allow this operation");
        }
        let hex = |text: &str| {
            text.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let parts = self.installation_id.split('-').collect::<Vec<_>>();
        if parts.iter().map(|part| part.len()).collect::<Vec<_>>() != [8, 4, 4, 4, 12]
            || !parts.iter().all(|part| hex(part))
            || self.server_generation.len() != 64
            || !hex(&self.server_generation)
        {
            bail!("invalid campaign identity");
        }
        if self.namespace != "library_context"
            || !self
                .database
                .strip_prefix("lifecycle_")
                .is_some_and(|suffix| suffix.len() == 32 && hex(suffix))
        {
            bail!("worker database must be a fresh lifecycle-owned database");
        }
        if !self.runtime_config.is_absolute()
            || !self.project_dir.is_absolute()
            || !self.report_path.is_absolute()
        {
            bail!("campaign paths must be host-issued absolute paths");
        }
        Ok(())
    }
}

async fn connected(runtime: &RuntimeConfig, binding: &TargetBinding) -> Result<Surreal<Any>> {
    crate::check_embedded_authority()?;
    // Default capabilities deny scripts/guest/network and enable no experimental feature.
    // The server's independently qualified capability ceiling still governs remote execution.
    let db = any::connect((runtime.endpoint.clone(), Config::new()))
        .await
        .context("owned Any gRPC connection")?;
    let result: Result<()> = async {
        db.signin(Root {
            username: runtime.username.clone(),
            password: runtime.password.clone(),
        })
        .await
        .map_err(|_| anyhow::anyhow!("owned campaign authentication failed"))?;
        db.use_ns(runtime.namespace.as_str())
            .use_db("validation")
            .await?;
        let version = db.version().await?.to_string();
        if !version.starts_with("3.3.0") {
            bail!("campaign requires pinned server 3.3.0");
        }
        let mut response = db
            .query("SELECT * FROM native_installation:current")
            .await?
            .check()?;
        let rows: Vec<Object> = response.take(0)?;
        if rows.len() != 1
            || rows[0].get("generation") != Some(&Value::String(binding.server_generation.clone()))
            || rows[0].get("schema")
                != Some(&Value::String(
                    lctx_native::installation::base_schema_identity().hex(),
                ))
            || rows[0].get("schema_version")
                != Some(&Value::Number(Number::Int(
                    lctx_native::control::SCHEMA_VERSION,
                )))
        {
            bail!("campaign generation marker mismatch");
        }
        db.use_ns(&binding.namespace)
            .use_db(&binding.database)
            .await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        let invalidation = db.invalidate().await;
        return Err(error.context(if invalidation.is_ok() {
            "setup refused; session invalidated"
        } else {
            "setup refused; session invalidation unconfirmed; parent must drain whole target"
        }));
    }
    Ok(db)
}

pub async fn sync(
    binding: &TargetBinding,
    runtime: &RuntimeConfig,
    modules: &[String],
) -> Result<()> {
    let selected = crate::selected_modules(modules)?;
    let db = connected(runtime, binding).await?;
    let result: Result<()> = async {
        for module in selected {
            let (_, files) = crate::embedded_modules()
                .iter()
                .find(|(name, _)| *name == module)
                .context("embedded responsibility inventory")?;
            surrealkit::Sync::embedded(files)
                .module(module)?
                .prune(true)
                .fail_fast(true)
                .run(&db)
                .await?;
        }
        crate::catalog::check_required(&db, modules).await?;
        Ok(())
    }
    .await;
    let closed = db.invalidate().await;
    match (result, closed) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => {
            Err(error.context("schema sync failed; session invalidated; target still parent-owned"))
        }
        (_, Err(_)) => {
            bail!("schema sync session invalidation unconfirmed; parent must drain whole target")
        }
    }
}

pub async fn test(binding: &TargetBinding, runtime: &RuntimeConfig) -> Result<()> {
    let projection = crate::read_projection(&binding.project_dir)?;
    check_test_inputs(&binding.project_dir.join("tests/suites"))?;
    // Authenticate/verify exact generation on a directly owned session first. Tester private
    // sessions have no injectable per-session cancellation owner; whole-target drainage settles them.
    let db = connected(runtime, binding).await?;
    let config_path = binding.project_dir.join("tests/config.toml");
    let mut token_input = None;
    let prepared: Result<()> = async {
        if config_path.exists() {
            if std::fs::symlink_metadata(&config_path)?
                .file_type()
                .is_symlink()
            {
                bail!("tester config cannot be a symlink");
            }
            let original = std::fs::read_to_string(&config_path)?;
            if original.contains(crate::sample::TOKEN_MARKER) {
                // Publish recovery before issuing or copying any credential. The parent
                // can restore this exact owned input after authenticated child drainage.
                token_input = Some(TokenInput::prepare(binding, original)?);
                let token = db
                    .signin(Root {
                        username: runtime.username.clone(),
                        password: runtime.password.clone(),
                    })
                    .await
                    .map_err(|_| anyhow::anyhow!("sample token issuance failed"))?;
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&config_path)?;
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
                file.write_all(
                    token_input
                        .as_ref()
                        .context("prepared token input")?
                        .original
                        .replace(
                            crate::sample::TOKEN_MARKER,
                            token.access.as_insecure_token(),
                        )
                        .as_bytes(),
                )?;
                file.sync_all()?;
            }
        }
        Ok(())
    }
    .await;
    let invalidated = db.invalidate().await;
    let result: Result<()> = async {
        prepared?;
        invalidated.context("tester preflight session invalidation")?;
        let opts = surrealkit::tester::TestOpts {
            suite: None,
            case: None,
            tags: vec![],
            fail_fast: false,
            parallel: std::thread::available_parallelism()?.get(),
            json_out: Some(binding.report_path.clone()),
            no_setup: false,
            no_sync: false,
            no_seed: true,
            base_url: None,
            timeout_ms: None,
            keep_db: true,
        };
        let overrides = surrealkit::DbOverrides {
            host: Some(runtime.endpoint.clone()),
            ns: Some(binding.namespace.clone()),
            db: Some(binding.database.clone()),
            user: Some(runtime.username.clone()),
            pass: Some(runtime.password.clone()),
            auth_level: Some("root".into()),
            folder: Some(
                binding
                    .project_dir
                    .to_str()
                    .context("UTF-8 owned project path")?
                    .into(),
            ),
        };
        let result = surrealkit::tester::run_test(
            None,
            opts,
            surrealkit::TemplateVars::default(),
            &overrides,
        )
        .await;
        if !binding.report_path.exists() {
            return match result {
                Err(error) => Err(error.context(
                    "beta tester failed without a JSON report; whole target remains parent-owned",
                )),
                Ok(()) => Err(anyhow::anyhow!(
                    "beta tester returned success without its required JSON report"
                )),
            };
        }
        {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&binding.report_path)?)?;
            let suites = report
                .get("suites")
                .and_then(serde_json::Value::as_array)
                .context("beta report suite inventory")?;
            let db = connected(runtime, binding).await?;
            let checked: Result<()> = async {
                for suite in suites {
                    let namespace = suite
                        .get("namespace")
                        .and_then(serde_json::Value::as_str)
                        .context("suite namespace")?;
                    let database = suite
                        .get("database")
                        .and_then(serde_json::Value::as_str)
                        .context("suite database")?;
                    if !namespace.starts_with(&format!("{}_sk_test_", binding.namespace))
                        || !database.starts_with(&format!("{}_sk_test_", binding.database))
                    {
                        bail!("beta report suite escaped issued target scope");
                    }
                    db.use_ns(namespace).use_db(database).await?;
                    crate::catalog::check_required(&db, &projection.modules).await?;
                }
                Ok(())
            }
            .await;
            let closed = db.invalidate().await;
            checked?;
            closed.context(
                "tester comparison session invalidation; whole target remains parent-owned",
            )?;
        }
        result
    }
    .await;
    if let Some(input) = token_input.as_mut() {
        input
            .restore()
            .context("sample token input cleanup; parent still owns project")?;
    }
    result
}

const TOKEN_RESTORE_JOURNAL: &str = "sample-token-restore.json";

struct TokenInput {
    config: PathBuf,
    journal: PathBuf,
    original: String,
    active: bool,
}

impl TokenInput {
    fn prepare(binding: &TargetBinding, original: String) -> Result<Self> {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let config = binding.project_dir.join("tests/config.toml");
        let journal = binding.project_dir.join(TOKEN_RESTORE_JOURNAL);
        for path in [&config, &journal] {
            if path.ancestors().any(|parent| parent.is_symlink()) {
                bail!("token recovery paths must be physical");
            }
        }
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600))?;
        let value = serde_json::json!({"schema":1, "purpose":"surrealdb-lifecycle-token-restore",
            "installation_id":binding.installation_id, "server_generation":binding.server_generation,
            "project_dir":binding.project_dir, "config_path":config, "original":original});
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&journal)?;
        file.write_all(&serde_json::to_vec(&value)?)?;
        file.sync_all()?;
        std::fs::File::open(&binding.project_dir)?.sync_all()?;
        Ok(Self {
            config,
            journal,
            original,
            active: true,
        })
    }

    fn restore(&mut self) -> Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if !self.active {
            return Ok(());
        }
        if self.config.ancestors().any(|path| path.is_symlink())
            || self.journal.ancestors().any(|path| path.is_symlink())
        {
            bail!("token recovery paths changed");
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.config)?;
        file.write_all(self.original.as_bytes())?;
        file.sync_all()?;
        // Retain the non-secret recovery input until parent verification/recovery.
        // SIGTERM does not run Drop; the parent's exact ledger remains authoritative.
        self.active = false;
        Ok(())
    }
}

impl Drop for TokenInput {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn check_test_inputs(root: &Path) -> Result<()> {
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() {
        bail!("test suites cannot be symlinked");
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            bail!("test suites cannot contain symlinks");
        }
        if kind.is_dir() {
            check_test_inputs(&entry.path())?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            let suite: toml::Value = toml::from_str(&std::fs::read_to_string(entry.path())?)?;
            if suite
                .get("cases")
                .and_then(toml::Value::as_array)
                .is_some_and(|cases| {
                    cases.iter().any(|case| {
                        case.get("kind").and_then(toml::Value::as_str) == Some("api_request")
                    })
                })
            {
                bail!(
                    "API request suites require a separately qualified HTTP target binding and are not supported by this worker"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> TargetBinding {
        TargetBinding {
            format: 1,
            purpose: "surrealdb-lifecycle".into(),
            target_kind: "campaign".into(),
            installation_id: "12345678-1234-1234-1234-123456789abc".into(),
            server_generation: "a".repeat(64),
            runtime_config: PathBuf::from("/owned/runtime.json"),
            namespace: "library_context".into(),
            database: format!("lifecycle_{}", "a".repeat(32)),
            allowed_operations: vec!["sync".into(), "test".into()],
            project_dir: PathBuf::from("/owned/project"),
            report_path: PathBuf::from("/owned/report.json"),
        }
    }
    #[test]
    fn refuses_shared_scope_identity_and_operation_before_connecting() {
        let mut target = binding();
        target.validate("sync").unwrap();
        target.target_kind = "stable".into();
        assert!(target.validate("sync").is_err());
        let mut target = binding();
        target.database = "validation".into();
        assert!(target.validate("sync").is_err());
        let mut target = binding();
        target.server_generation = "G".repeat(64);
        assert!(target.validate("test").is_err());
        let mut target = binding();
        target.allowed_operations.clear();
        assert!(target.validate("test").is_err());
        let mut target = binding();
        target.report_path = PathBuf::from("relative");
        assert!(target.validate("test").is_err());
    }
    #[test]
    fn requires_matching_actual_campaign_descriptor_before_connecting() {
        let root = tempfile::tempdir().unwrap();
        let mut target = binding();
        target.runtime_config = root.path().join("validation-installer.json");
        let generation = vec![0xaa_u8; 32];
        let runtime = serde_json::json!({
            "endpoint":"grpc://127.0.0.1:12345", "username":"root", "password":"private",
            "viewer_username":"viewer", "viewer_password":"private-viewer", "namespace":"library_context",
            "database":"validation", "cache_database":"validation", "service_generation":generation,
            "authentication":"root", "selection":root.path().join("selection.json")
        });
        std::fs::write(
            &target.runtime_config,
            serde_json::to_vec(&runtime).unwrap(),
        )
        .unwrap();
        let mut descriptor = serde_json::json!({
            "kind":"campaign", "purpose":"surrealdb-lifecycle", "installation_id":target.installation_id,
            "phase":"ready", "grpc_endpoint":"grpc://127.0.0.1:12345", "namespace":"library_context",
            "service_generation":generation
        });
        let descriptor_path = root.path().join("installation.json");
        let binding_path = root.path().join("binding.json");
        std::fs::write(&binding_path, serde_json::to_vec(&target).unwrap()).unwrap();
        std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        TargetBinding::read(&binding_path, "sync").unwrap();
        descriptor["installation_id"] = serde_json::json!("87654321-1234-1234-1234-123456789abc");
        std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        assert!(TargetBinding::read(&binding_path, "sync").is_err());
        descriptor["installation_id"] = serde_json::json!(target.installation_id);
        descriptor["kind"] = serde_json::json!("stable");
        std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        assert!(TargetBinding::read(&binding_path, "sync").is_err());
        descriptor["kind"] = serde_json::json!("campaign");
        descriptor["service_generation"] = serde_json::json!(vec![0xbb_u8; 32]);
        std::fs::write(&descriptor_path, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        assert!(TargetBinding::read(&binding_path, "sync").is_err());
    }
    #[test]
    fn unsupported_http_suite_refuses_without_a_connection() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("api.toml"),
            "[[cases]]\nname='request'\nkind='api_request'\npath='/'\n",
        )
        .unwrap();
        assert!(check_test_inputs(root.path()).is_err());
        std::fs::write(
            root.path().join("api.toml"),
            "[[cases]]\nname='sql'\nkind='sql_expect'\nsql='RETURN 1;'\n",
        )
        .unwrap();
        check_test_inputs(root.path()).unwrap();
    }
    #[test]
    fn token_recovery_precedes_mutation_and_restores_early_error_scope() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let mut target = binding();
        target.project_dir = root.path().join("project");
        std::fs::create_dir_all(target.project_dir.join("tests")).unwrap();
        let config = target.project_dir.join("tests/config.toml");
        let original = format!("token='{}'\n", crate::sample::TOKEN_MARKER);
        std::fs::write(&config, &original).unwrap();
        {
            let input = TokenInput::prepare(&target, original.clone()).unwrap();
            let journal: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&input.journal).unwrap()).unwrap();
            assert_eq!(journal["original"], original);
            assert_eq!(journal["installation_id"], target.installation_id);
            assert_eq!(
                std::fs::metadata(&input.journal)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
            // A partial credential write followed by any early return drops the guard.
            std::fs::write(&config, "partial-issued-credential").unwrap();
        }
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
        assert!(
            target.project_dir.join(TOKEN_RESTORE_JOURNAL).exists(),
            "parent recovery input survives until verified cleanup"
        );
    }
}
