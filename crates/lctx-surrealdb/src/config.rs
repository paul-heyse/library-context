//! Operator runtime configuration and explicit selection of a complete published handle.
use crate::Credentials;
use lctx_model::domain::{
    ContentHash, ModelError,
    serving::{Name, SnapshotHandle, ResourceLimits},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
/// Installation is the only caller allowed to carry a root principal. Ordinary clients
/// authenticate inside their installed database and cannot create sibling databases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationScope {
    Root,
    Database,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse: Option<ReuseConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serving_limits: Option<ResourceLimits>,
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub viewer_username: String,
    pub viewer_password: String,
    pub namespace: Name,
    pub database: Name,
    pub service_generation: ContentHash,
    pub authentication: AuthenticationScope,
    pub cache_database: Name,
    pub selection: PathBuf,
}
/// Optional portable products share the installed database but retain their own table/lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReuseConfig {
    pub database: Name,
    pub capacity_bytes: u64,
    pub lease_directory: PathBuf,
}
impl ReuseConfig {
    pub fn validate(&self, runtime: &RuntimeConfig) -> Result<(), ModelError> {
        if self.database != runtime.database || self.capacity_bytes == 0 || self.capacity_bytes > i64::MAX as u64 || !self.lease_directory.is_absolute() {
            return Err(ModelError::Invalid("reuse requires the installed database, positive capacity and absolute coordination directory".into()));
        }
        Ok(())
    }
}
impl RuntimeConfig {
    pub fn read(path: &Path) -> Result<Self, ModelError> {
        let cfg: Self = serde_json::from_slice(&std::fs::read(path).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)?;
        if !cfg.endpoint.starts_with("grpc://127.0.0.1:")
            && !cfg.endpoint.starts_with("grpc://localhost:")
        {
            return Err(ModelError::Invalid(
                "native endpoint must be the managed local gRPC server".into(),
            ));
        }
        if cfg.username.is_empty()
            || cfg.password.is_empty()
            || cfg.viewer_username.is_empty()
            || cfg.viewer_password.is_empty()
            || cfg.viewer_username == cfg.username
        {
            return Err(ModelError::Invalid(
                "native database credentials required".into(),
            ));
        }
        if !matches!(cfg.database.as_str(), "main" | "validation")
            || cfg.cache_database != cfg.database
        {
            return Err(ModelError::Invalid("runtime requires stable main or validation storage".into()));
        }
        if let Some(reuse) = &cfg.reuse { reuse.validate(&cfg)?; }
        if let Some(limits) = &cfg.serving_limits { limits.validate()?; }
        Ok(cfg)
    }
    pub fn writer_credentials(&self) -> Credentials {
        match self.authentication {
            AuthenticationScope::Root => Credentials::Root {
                username: self.username.clone(), password: self.password.clone(),
            },
            AuthenticationScope::Database => Credentials::Database {
                username: self.username.clone(), password: self.password.clone(),
            },
        }
    }
    pub fn viewer_credentials(&self) -> Credentials {
        Credentials::Database {
            username: self.viewer_username.clone(),
            password: self.viewer_password.clone(),
        }
    }
    pub fn selected(&self) -> Result<SnapshotHandle, ModelError> {
        serde_json::from_slice(&std::fs::read(&self.selection).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)
    }
    fn selection_path(&self) -> Result<PathBuf, ModelError> {
        let parent = self
            .selection
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(ModelError::codec)?;
        let name = self
            .selection
            .file_name()
            .ok_or(ModelError::Schema("snapshot selection path"))?;
        Ok(std::fs::canonicalize(parent)
            .map_err(ModelError::codec)?
            .join(name))
    }
    fn lock_file(&self) -> Result<(std::fs::File, PathBuf), ModelError> {
        use std::os::unix::fs::OpenOptionsExt;
        let selection = self.selection_path()?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(selection.with_extension("selection.lock"))
            .map_err(ModelError::codec)?;
        Ok((file, selection))
    }
    /// Serialize lifecycle operations across processes without parking an async executor thread.
    pub async fn lock_selection(&self) -> Result<SelectionGuard, ModelError> {
        let (file, selection) = self.lock_file()?;
        let deadline = tokio::time::Instant::now()
            + std::time::Duration::from_millis(
                lctx_model::domain::serving::ResourceLimits::default().request_deadline_ms,
            );
        loop {
            match file.try_lock() {
                Ok(()) => {
                    return Ok(SelectionGuard {
                        _file: file,
                        selection,
                    });
                }
                Err(std::fs::TryLockError::Error(error)) => return Err(ModelError::codec(error)),
                Err(std::fs::TryLockError::WouldBlock) => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err(ModelError::infrastructure(
                            lctx_model::domain::Infrastructure::Contention,
                            "selection lifecycle lock deadline",
                        ));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
    }
    /// Synchronous explicit filesystem selection. Async operator entry points hold lock_selection
    /// across candidate validation and select_locked; retirement uses the same authority lock.
    pub fn select(&self, handle: &SnapshotHandle) -> Result<(), ModelError> {
        let (file, selection) = self.lock_file()?;
        file.lock().map_err(ModelError::codec)?;
        self.select_locked(
            handle,
            &SelectionGuard {
                _file: file,
                selection,
            },
        )
    }
    pub fn select_locked(
        &self,
        handle: &SnapshotHandle,
        guard: &SelectionGuard,
    ) -> Result<(), ModelError> {
        self.commit_selection(handle, guard, |parent| {
            std::fs::File::open(parent)?.sync_all()
        })
    }
    fn commit_selection(
        &self,
        handle: &SnapshotHandle,
        guard: &SelectionGuard,
        sync_parent: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<(), ModelError> {
        use std::io::Write;
        if guard.selection != self.selection_path()? {
            return Err(ModelError::Conflict("selection lifecycle guard"));
        }
        let selection = &guard.selection;
        let parent = selection
            .parent()
            .ok_or(ModelError::Schema("snapshot selection parent"))?;
        let viewer = ViewerConfig {
            serving_limits: self.serving_limits.clone(),
            endpoint: self.endpoint.clone(),
            // The trusted Rust reader writes durable pins. A native VIEWER principal cannot
            // do that; this private configuration never becomes a public snapshot capability.
            username: self.username.clone(),
            password: self.password.clone(),
            selection: selection.clone(),
        };
        let serving_path = self.selection.with_extension("serving.json");
        if serving_path.try_exists().map_err(ModelError::codec)? {
            if ViewerConfig::read(&serving_path)? != viewer {
                return Err(ModelError::Conflict(
                    "static serving configuration changed; reinstall it explicitly",
                ));
            }
        } else {
            let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
            staged
                .write_all(&serde_json::to_vec(&viewer).map_err(ModelError::codec)?)
                .map_err(ModelError::codec)?;
            staged.as_file().sync_all().map_err(ModelError::codec)?;
            staged
                .persist_noclobber(&serving_path)
                .map_err(|error| ModelError::codec(error.error))?;
            std::fs::File::open(parent)
                .map_err(ModelError::codec)?
                .sync_all()
                .map_err(ModelError::codec)?;
        }
        let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
        staged
            .write_all(&serde_json::to_vec_pretty(handle).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)?;
        staged.as_file().sync_all().map_err(ModelError::codec)?;
        staged
            .persist(selection)
            .map_err(|error| ModelError::codec(error.error))?;
        sync_parent(parent).map_err(|error| {
            ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                format!("selection was committed; directory durability is uncertain: {error}"),
            )
        })
    }
}

/// Minimal serving credentials: no administrator or cache writer secret crosses this boundary.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ViewerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serving_limits: Option<ResourceLimits>,
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub selection: PathBuf,
}
impl ViewerConfig {
    pub fn read(path: &Path) -> Result<Self, ModelError> {
        let cfg: Self = serde_json::from_slice(&std::fs::read(path).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        if let Some(limits) = &cfg.serving_limits { limits.validate()?; }
        Ok(cfg)
    }
    /// New launches read the one atomic handle. Existing sessions retain their immutable pin.
    pub fn selected(&self) -> Result<SnapshotHandle, ModelError> {
        serde_json::from_slice(&std::fs::read(&self.selection).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)
    }
    pub fn credentials(&self) -> Credentials {
        Credentials::Database {
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}

/// Held from candidate validation through commit/removal. Closing the file releases the lock.
pub struct SelectionGuard {
    _file: std::fs::File,
    selection: PathBuf,
}

#[cfg(test)]
mod selection_controls {
    use super::*;
    use lctx_model::domain::{ContentHash, Infrastructure, serving::DatabaseIdentity};
    fn config(root: &Path) -> RuntimeConfig {
        RuntimeConfig {
            reuse: None,
            serving_limits: None,
            endpoint: "grpc://127.0.0.1:9999".into(),
            username: "installer".into(),
            password: "private".into(),
            viewer_username: "viewer".into(),
            viewer_password: "viewer-private".into(),
            namespace: Name::new("fixture").unwrap(),
            database: Name::new("validation").unwrap(),
            service_generation: ContentHash::of(b"fixture-installation"),
            authentication: AuthenticationScope::Database,
            cache_database: Name::new("validation").unwrap(),
            selection: root.join("selected.json"),
        }
    }
    fn handle(label: &str) -> SnapshotHandle {
        SnapshotHandle {
            semantic: ContentHash::of(label.as_bytes()),
            realization: ContentHash::of(b"realization"),
            publication: ContentHash::of(label.as_bytes()),
            view: ContentHash::of(label.as_bytes()),
            service_generation: ContentHash::of(b"fixture-installation"),
            definition_epoch: ContentHash::of(b"definitions"),
            database: DatabaseIdentity {
                namespace: Name::new("fixture").unwrap(),
                database: Name::new(label).unwrap(),
            },
        }
    }
    #[tokio::test]
    async fn atomic_selection_is_one_authority_and_lifecycle_is_serialized() {
        let scratch = tempfile::tempdir().unwrap();
        let cfg = config(scratch.path());
        let old = handle("old");
        let new = handle("new");
        cfg.select(&old).unwrap();
        let viewer = ViewerConfig::read(&cfg.selection.with_extension("serving.json")).unwrap();
        assert_eq!(viewer.username,cfg.username,"trusted readers require the pin-writing principal");
        let pinned = viewer.selected().unwrap();
        let held = cfg.lock_selection().await.unwrap();
        let competitor = config(scratch.path());
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(30),
                competitor.lock_selection()
            )
            .await
            .is_err()
        );
        cfg.select_locked(&new, &held).unwrap();
        drop(held);
        let second = competitor.lock_selection().await.unwrap();
        assert_eq!(competitor.selected().unwrap(), new);
        assert_eq!(viewer.selected().unwrap(), new);
        assert_eq!(pinned, old);
        let alien = tempfile::tempdir().unwrap();
        assert!(config(alien.path()).select_locked(&old, &second).is_err());
    }
    #[tokio::test]
    async fn precommit_failure_keeps_selection_and_postcommit_uncertainty_keeps_new_handle() {
        let scratch = tempfile::tempdir().unwrap();
        let cfg = config(scratch.path());
        let old = handle("old");
        let new = handle("new");
        cfg.select(&old).unwrap();
        let mut changed = config(scratch.path());
        changed.endpoint = "grpc://127.0.0.1:9998".into();
        assert!(changed.select(&new).is_err());
        assert_eq!(cfg.selected().unwrap(), old);
        let guard = cfg.lock_selection().await.unwrap();
        let error = cfg
            .commit_selection(&new, &guard, |_| {
                Err(std::io::Error::other("injected directory sync failure"))
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ModelError::Infrastructure {
                class: Infrastructure::Unconfirmed,
                ..
            }
        ));
        assert_eq!(cfg.selected().unwrap(), new);
        assert_eq!(
            ViewerConfig::read(&cfg.selection.with_extension("serving.json"))
                .unwrap()
                .selected()
                .unwrap(),
            new
        );
    }
}
