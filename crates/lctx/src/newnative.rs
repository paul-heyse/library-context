//! Explicit native operator actions and fixed-snapshot viewer journeys.
use anyhow::Context as _;
use lctx_model::domain::serving::{ResourceLimits, SnapshotHandle};
use lctx_native::surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{SerdeWrapper, SurrealValue},
};
use lctx_native::{NativeReader, RuntimeConfig};
use std::{path::Path, sync::Arc, time::Duration};

pub fn default_config() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG") {
        return path.into();
    }
    if let Some(path) = std::env::var_os("LCTX_SURREAL_SERVICE_CONFIG") {
        return std::path::PathBuf::from(path).with_file_name("main-runtime.json");
    }
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/state")
        });
    root.join("library-context/surrealdb/main-runtime.json")
}

pub fn config(path: &Path) -> anyhow::Result<RuntimeConfig> {
    RuntimeConfig::read(path)
        .with_context(|| format!("native runtime configuration {}", path.display()))
}

/// Observe the installed generation before acquisition. This never provisions storage.
pub async fn ready(config: &RuntimeConfig) -> anyhow::Result<Arc<Surreal<Client>>> {
    tokio::time::timeout(deadline(), async {
        Ok(lctx_compiler_store::compiler::check_installation(config).await?)
    })
    .await
    .context("native runtime readiness deadline exceeded")?
}

fn deadline() -> Duration {
    Duration::from_millis(ResourceLimits::default().request_deadline_ms)
}
pub(crate) fn operation_error(error: anyhow::Error) -> lctx_model::domain::ModelError {
    match error.downcast::<lctx_model::domain::ModelError>() {
        Ok(error) => error,
        Err(error) => lctx_model::domain::ModelError::Cause(error.into_boxed_dyn_error()),
    }
}

pub async fn publish(
    path: &Path,
    receipt_path: &Path,
    config: &RuntimeConfig,
    memory_bytes: usize,
) -> anyhow::Result<SnapshotHandle> {
    let expected: lctx_model::domain::completed_transport::ArtifactExportReceipt =
        serde_json::from_slice(&read_expected_receipt(receipt_path)?)?;
    expected.validate()?;
    let manifest =
        lctx_model::domain::graph::Manifest::decode(&std::fs::read(path.join("manifest.json"))?)?;
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(memory_bytes)?;
    let model = Arc::new(lctx_model::domain::model()?);
    let factory = lctx_compiler_store::compiler::recovery_factory(config, budget.clone())?;
    let native = lctx_compiler_store::compiler::NativeCompilerStore::begin(
        config,
        manifest.frontier,
        &factory,
    )
    .await?;
    let workspace = match cpg_core::workspace::Workspace::with_budget(
        model,
        cpg_core::workspace::WorkspaceOptions {
            memory_bytes,
            ..Default::default()
        },
        budget.clone(),
        native.clone(),
    ) {
        Ok(workspace) => workspace,
        Err(error) => {
            native.fail();
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step("import setup abandon", native.abandon().await);
            return lctx_model::domain::completion::complete::<SnapshotHandle>(
                Err(error),
                completion,
            )
            .map_err(Into::into);
        }
    };
    let result = async {
        let export = cpg_core::artifact::verify_export(path, &workspace, &expected).await?;
        lctx_publisher::publish(&export, config, &lctx_serving::native_definitions()).await
    }
    .await;
    let committed = result.as_ref().ok().map(|handle| format!("{handle:?}"));
    let completion = finalize_compilation(&workspace, &native, &result, committed).await;
    lctx_model::domain::completion::complete(result, completion).map_err(Into::into)
}

/// Finish a CLI-owned compiler at its last use. A publication owns a separate
/// session, so a committed handle does not release the original compiler session.
/// Borrowed seals and exports must leave this decision to their actual owner.
pub(crate) async fn finalize_compilation<T>(
    workspace: &cpg_core::workspace::Workspace,
    native: &lctx_compiler_store::compiler::NativeCompilerStore,
    result: &Result<T, lctx_model::domain::ModelError>,
    committed: Option<String>,
) -> lctx_model::domain::completion::Completion {
    use lctx_model::domain::{
        ModelError,
        completion::{LocalState, RemoteState, StorageState},
    };
    let mut completion = workspace.drain_owned_report().await;
    if let Some(identity) = committed {
        completion.committed("published unselected manifest", identity);
    }
    if result.is_err() || !completion.failures.is_empty() {
        native.fail();
    }
    if result
        .as_ref()
        .err()
        .is_none_or(ModelError::permits_storage_cleanup)
        && completion.local == LocalState::Terminal
        && completion.remote == RemoteState::Confirmed
        && completion
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup())
    {
        let finality = native.abandon_finality_report().await;
        completion.merge(finality);
    } else {
        completion.storage.push(StorageState::Orphan(format!(
            "native_attempt:{}",
            native.attempt().hex()
        )));
        if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            completion.remote = RemoteState::Unknown;
        }
    }
    let settled = workspace.complete_signal(&completion).await;
    completion.step("CLI compiler workspace signal finality", settled);
    // Sticky acknowledged operation failures remain part of the caller outcome;
    // they are not failed joins or failed native source/session finalization.
    workspace.append_operation_failures(&mut completion);
    completion
}

fn handle(config: &RuntimeConfig, path: Option<&Path>) -> anyhow::Result<SnapshotHandle> {
    match path {
        Some(path) => serde_json::from_slice(&std::fs::read(path)?)
            .with_context(|| format!("snapshot handle {}", path.display())),
        None => Ok(config.selected()?),
    }
}

pub struct PinnedReader {
    reader: NativeReader,
    finalization: lctx_native::reader::ReadFinalizationGuard,
    signal: lctx_native::signal::SignalRegistration,
}
impl std::ops::Deref for PinnedReader {
    type Target = NativeReader;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
pub async fn pin(config: &RuntimeConfig, path: Option<&Path>) -> anyhow::Result<PinnedReader> {
    let limits = config.serving_limits.clone().unwrap_or_default();
    limits.validate()?;
    let budget =
        lctx_model::domain::resources::ResourceBudget::fixed(limits.shared_bytes.try_into()?)?;
    let reader_factory =
        lctx_native::control::RecoveryFactoryOwner::new(config.receipt_binding()?, budget.clone())?;
    let handle = handle(config, path)?;
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancellation = lctx_native::control::RetryCancellation::default();
    let recovery_admission = cancellation.clone();
    let (flag, retry) = (stopped.clone(), cancellation.clone());
    let signal = lctx_native::signal::SignalRegistration::new("CLI selected reader", move || {
        flag.store(true, std::sync::atomic::Ordering::Release);
        retry.cancel();
    })?;
    let result = async {
        if cancellation.stopped() {
            return Err(anyhow::anyhow!("native process admission cancelled"));
        }
        Ok(tokio::time::timeout(
            deadline(),
            lctx_native::control::with_retry_cancellation(
                cancellation,
                NativeReader::connect(
                    &config.endpoint,
                    &config.writer_credentials(),
                    handle,
                    &budget,
                    &reader_factory,
                ),
            ),
        )
        .await
        .context("snapshot readiness deadline exceeded")??)
    }
    .await;
    match result {
        Ok(reader) => {
            let reader = reader.with_read_cancellation(stopped);
            let finalization = reader.protect_terminal_close();
            Ok(PinnedReader {
                reader,
                finalization,
                signal,
            })
        }
        Err(error) => {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "CLI reader constructor factory drainage",
                reader_factory.drain(&recovery_admission).await,
            );
            // A failed/expired connection cannot prove that pin admission never
            // committed. Retain its started owner for native reconciliation.
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
            completion.step(
                "CLI reader signal owner",
                signal.complete(&completion).await,
            );
            lctx_model::domain::completion::complete(Err(operation_error(error)), completion)
                .map_err(Into::into)
        }
    }
}

pub async fn select(config: &RuntimeConfig, path: &Path) -> anyhow::Result<SnapshotHandle> {
    let guard = config.lock_selection().await?;
    let reader = pin(config, Some(path)).await?;
    let result = config
        .select_locked(reader.handle(), &guard)
        .map(|()| reader.handle().clone())
        .map_err(Into::into);
    finish_reader(reader, result).await
}

pub async fn show(
    config: &RuntimeConfig,
    path: Option<&Path>,
) -> anyhow::Result<lctx_publisher::inspection::SnapshotDetails> {
    let reader = pin(config, path).await?;
    let result = lctx_publisher::inspection::show(&reader)
        .await
        .map_err(Into::into);
    finish_reader(reader, result).await
}

pub async fn query(
    config: &RuntimeConfig,
    path: Option<&Path>,
    relation: &str,
    limit: usize,
) -> anyhow::Result<serde_json::Value> {
    if limit == 0 {
        return Err(crate::Refused("query limit must be positive".into()).into());
    }
    let model = lctx_model::domain::model()?;
    if model.relation(relation).is_none() {
        return Err(crate::Refused("query requires a model-declared relation".into()).into());
    }
    let reader = pin(config, path).await?;
    let result = tokio::time::timeout(deadline(), async {
        let mut stream = reader.relation_bodies(relation, limit)?;
        let mut rows = Vec::new();
        while let Some(value) = stream.next().await? {
            rows.push(SerdeWrapper::<serde_json::Value>::from_value(value)?.0);
        }
        Ok::<_, anyhow::Error>(serde_json::Value::Array(rows))
    })
    .await
    .context("snapshot query deadline exceeded");
    finish_reader(reader, result.and_then(|value| value)).await
}

async fn finish_reader<T>(
    mut reader: PinnedReader,
    result: anyhow::Result<T>,
) -> anyhow::Result<T> {
    let result = result.map_err(operation_error);
    let mut completion = lctx_model::domain::completion::Completion::default();
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        reader.reader.retain_unknown();
        completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
    }
    completion.step(
        "published reader pin release",
        reader.finalization.close(&reader.reader).await,
    );
    completion.step(
        "CLI reader signal owner",
        reader.signal.complete(&completion).await,
    );
    lctx_model::domain::completion::complete(result, completion).map_err(Into::into)
}

pub async fn export(
    config: &RuntimeConfig,
    handle: Option<&Path>,
    key: lctx_model::domain::projection::normalization::ProjectionKey,
    output: &Path,
    memory_bytes: usize,
) -> anyhow::Result<()> {
    if output.exists() {
        return Err(crate::Refused("projection destination already exists".into()).into());
    }
    let reader = pin(config, handle).await?;
    let result = async {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(memory_bytes)?;
        let projection = tokio::time::timeout(
            deadline(),
            lctx_native::projections::materialize(&reader, key, &budget),
        )
        .await
        .context("projection materialization deadline exceeded")??;
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut writer = std::io::BufWriter::new(staged.as_file_mut());
            projection.write_json(&mut writer)?;
            use std::io::Write;
            writer.flush()?;
        }
        staged.as_file().sync_all()?;
        staged
            .persist_noclobber(output)
            .map_err(|error| error.error)?;
        Ok(())
    }
    .await;
    finish_reader(reader, result).await
}

pub async fn tool(
    config: &RuntimeConfig,
    path: Option<&Path>,
    tool: &str,
    raw: &str,
) -> anyhow::Result<String> {
    let limits = config.serving_limits.clone().unwrap_or_default();
    lctx_model::domain::serving::decode_request(tool, raw, &limits)?;
    let reader = pin(config, path).await?;
    let result = match lctx_serving::NativeService::new(reader.reader.clone(), limits) {
        Ok(service) => {
            let result = service
                .execute(tool, raw)
                .await
                .map_err(anyhow::Error::from);
            service.close().await;
            drop(service);
            result
        }
        Err(error) => Err(error.into()),
    };
    finish_reader(reader, result).await
}

/// Explicit maintenance installation, never called by ordinary compilation or readiness.
pub async fn install(config: &RuntimeConfig, keep_closed: bool) -> anyhow::Result<()> {
    use lctx_native::phase::Phase;
    let operation = Phase::begin("native_installation");
    // Explicit maintenance installation spans many checked operations and
    // background index-readiness waits. A serving request deadline must not
    // drop issued schema work; host interruption retains the owned target's
    // separate maintenance/recovery obligation.
    let result = async {
        anyhow::ensure!(
            config.authentication == lctx_native::config::AuthenticationScope::Root,
            "installation requires explicit maintenance credentials"
        );
        let phase = Phase::begin("native_installation_shared_schema");
        let result = lctx_compiler_store::compiler::install_shared(
            config,
            &lctx_serving::native_definitions(),
        )
        .await;
        phase.finish_result(&result);
        result?;
        let phase = Phase::begin("native_installation_check");
        let result = lctx_compiler_store::compiler::check_installation(config).await;
        phase.finish_result(&result);
        let client = result?;
        let session = lctx_native::reader::SessionFinalizationGuard::new(client.clone());
        let phase = Phase::begin("native_installation_embedding_schema");
        let embedding = lctx_native::NativeEmbeddingCache::install(client)
            .await
            .map(|_| ());
        phase.finish_result(&embedding);
        if embedding
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            return embedding.map_err(Into::into);
        }
        let mut terminal = lctx_model::domain::completion::Completion::default();
        let phase = Phase::begin("native_installation_embedding_session_close");
        let result = session.close().await;
        phase.finish_result(&result);
        terminal.step("embedding schema session close", result);
        lctx_model::domain::completion::complete(embedding, terminal)?;
        let product_budget = lctx_model::domain::resources::ResourceBudget::fixed(
            lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
        )?;
        let product_factory =
            lctx_compiler_store::compiler::recovery_factory(config, product_budget)?;
        let phase = Phase::begin("native_installation_product_schema");
        let result =
            lctx_compiler_store::NativeProductCache::install(config, &product_factory).await;
        phase.finish_result(&result);
        if let Some(cache) = result? {
            let phase = Phase::begin("native_installation_product_session_close");
            let result = std::sync::Arc::new(cache).close().await;
            phase.finish_result(&result);
            result?;
        }
        let phase = Phase::begin("native_installation_publication_schema");
        let result =
            lctx_publisher::install_definitions(config, &lctx_serving::native_definitions()).await;
        phase.finish_result(&result);
        result?;
        if !keep_closed {
            let phase = Phase::begin("native_installation_open_admission");
            let result = lctx_compiler_store::compiler::open_admission(config).await;
            phase.finish_result(&result);
            result?;
        }
        Ok(())
    }
    .await;
    operation.finish_result(&result);
    result
}

/// Explicit lifecycle operations never update the selected snapshot pointer.
pub async fn backup(
    config: &RuntimeConfig,
    path: Option<&Path>,
    output: &Path,
) -> anyhow::Result<lctx_model::domain::completed_transport::BackupReceipt> {
    let selected = handle(config, path)?;
    Ok(lctx_publisher::backup::backup(
        config,
        &selected,
        output,
        &lctx_serving::native_definitions(),
    )
    .await?)
}
pub async fn restore(
    config: &RuntimeConfig,
    input: &Path,
    publication: Option<lctx_model::domain::ContentHash>,
    receipt_path: &Path,
) -> anyhow::Result<SnapshotHandle> {
    let expected: lctx_model::domain::completed_transport::BackupReceipt =
        serde_json::from_slice(&read_expected_receipt(receipt_path)?)?;
    expected.validate()?;
    let definitions = lctx_serving::native_definitions();
    Ok(match publication {
        Some(publication) => {
            lctx_publisher::backup::restore_publication(
                config,
                input,
                publication,
                &definitions,
                &expected,
            )
            .await?
        }
        None => lctx_publisher::backup::restore(config, input, &definitions, &expected).await?,
    })
}
pub async fn retire(
    config: &RuntimeConfig,
    path: &Path,
    readers_stopped: bool,
    clear_selection: bool,
    job: Option<lctx_model::domain::ContentHash>,
) -> anyhow::Result<lctx_model::domain::completed::RetirementProgress> {
    let snapshot = handle(config, Some(path))?;
    Ok(lctx_publisher::backup::retire_named(
        config,
        &snapshot,
        readers_stopped,
        clear_selection,
        job,
    )
    .await?)
}

pub async fn list(config: &RuntimeConfig) -> anyhow::Result<Vec<SnapshotHandle>> {
    Ok(lctx_publisher::inspection::list(config).await?)
}
pub async fn audit(config: &RuntimeConfig, path: &Path) -> anyhow::Result<SnapshotHandle> {
    let snapshot = handle(config, Some(path))?;
    lctx_publisher::inspection::audit(config, &snapshot, &lctx_serving::native_definitions())
        .await?;
    Ok(snapshot)
}

/// A caller-selected expectation is independent of the incoming artifact and sidecars.
fn read_expected_receipt(path: &Path) -> anyhow::Result<Vec<u8>> {
    use std::io::Read;
    let limit = lctx_model::domain::resources::MAX_ROW_BYTES;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((limit as u64) + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= limit, "expected receipt exceeds row bound");
    Ok(bytes)
}
