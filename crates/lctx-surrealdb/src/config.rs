//! Operator runtime configuration and explicit selection of a complete published handle.
use crate::Credentials;
use lctx_model::domain::{
    ModelError,
    serving::{Name, SnapshotHandle},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub viewer_username: String,
    pub viewer_password: String,
    pub namespace: Name,
    pub cache_database: Name,
    pub selection: PathBuf,
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
        Ok(cfg)
    }
    pub fn root_credentials(&self) -> Credentials {
        Credentials::Root {
            username: self.username.clone(),
            password: self.password.clone(),
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
        let parent = self.selection.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(ModelError::codec)?;
        let name = self.selection.file_name().ok_or(ModelError::Schema("snapshot selection path"))?;
        Ok(std::fs::canonicalize(parent).map_err(ModelError::codec)?.join(name))
    }
    fn lock_file(&self) -> Result<(std::fs::File, PathBuf), ModelError> {
        use std::os::unix::fs::OpenOptionsExt;
        let selection = self.selection_path()?;
        let file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
            .open(selection.with_extension("selection.lock")).map_err(ModelError::codec)?;
        Ok((file, selection))
    }
    /// Serialize lifecycle operations across processes without parking an async executor thread.
    pub async fn lock_selection(&self) -> Result<SelectionGuard, ModelError> {
        let (file, selection) = self.lock_file()?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(lctx_model::domain::serving::ResourceLimits::default().request_deadline_ms);
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(SelectionGuard { _file: file, selection }),
                Err(std::fs::TryLockError::Error(error)) => return Err(ModelError::codec(error)),
                Err(std::fs::TryLockError::WouldBlock) => {
                    if tokio::time::Instant::now() >= deadline { return Err(ModelError::infrastructure(lctx_model::domain::Infrastructure::Contention, "selection lifecycle lock deadline")); }
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
        self.select_locked(handle, &SelectionGuard { _file: file, selection })
    }
    pub fn select_locked(&self, handle: &SnapshotHandle, guard: &SelectionGuard) -> Result<(), ModelError> {
        self.commit_selection(handle, guard, |parent| std::fs::File::open(parent)?.sync_all())
    }
    fn commit_selection(&self, handle: &SnapshotHandle, guard: &SelectionGuard,
        sync_parent: impl FnOnce(&Path) -> std::io::Result<()>) -> Result<(), ModelError> {
        use std::io::Write;
        if guard.selection != self.selection_path()? { return Err(ModelError::Conflict("selection lifecycle guard")); }
        let selection = &guard.selection;
        let parent = selection.parent().ok_or(ModelError::Schema("snapshot selection parent"))?;
        let viewer = ViewerConfig { endpoint: self.endpoint.clone(), username: self.viewer_username.clone(), password: self.viewer_password.clone(), selection: selection.clone() };
        let serving_path = self.selection.with_extension("serving.json");
        if serving_path.try_exists().map_err(ModelError::codec)? {
            if ViewerConfig::read(&serving_path)? != viewer { return Err(ModelError::Conflict("static serving configuration changed; reinstall it explicitly")); }
        } else {
            let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
            staged.write_all(&serde_json::to_vec(&viewer).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
            staged.as_file().sync_all().map_err(ModelError::codec)?;
            staged.persist_noclobber(&serving_path).map_err(|error| ModelError::codec(error.error))?;
            std::fs::File::open(parent).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)?;
        }
        let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
        staged.write_all(&serde_json::to_vec_pretty(handle).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
        staged.as_file().sync_all().map_err(ModelError::codec)?;
        staged.persist(selection).map_err(|error| ModelError::codec(error.error))?;
        sync_parent(parent).map_err(|error| ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed,
            format!("selection was committed; directory durability is uncertain: {error}")))
    }

}

/// Minimal serving credentials: no administrator or cache writer secret crosses this boundary.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ViewerConfig {
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub selection: PathBuf,
}
impl ViewerConfig {
    pub fn read(path: &Path) -> Result<Self, ModelError> {
        serde_json::from_slice(&std::fs::read(path).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)
    }
    /// New launches read the one atomic handle. Existing sessions retain their immutable pin.
    pub fn selected(&self) -> Result<SnapshotHandle, ModelError> {
        serde_json::from_slice(&std::fs::read(&self.selection).map_err(ModelError::codec)?).map_err(ModelError::codec)
    }
    pub fn credentials(&self) -> Credentials {
        Credentials::Database {
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}

/// Held from candidate validation through commit/removal. Closing the file releases the lock.
pub struct SelectionGuard { _file: std::fs::File, selection: PathBuf }

#[cfg(test)]
mod selection_controls {
    use super::*;
    use lctx_model::domain::{ContentHash, Infrastructure, serving::DatabaseIdentity};
    fn config(root: &Path) -> RuntimeConfig {
        RuntimeConfig { endpoint: "grpc://127.0.0.1:9999".into(), username:"installer".into(),password:"private".into(), viewer_username:"viewer".into(),viewer_password:"viewer-private".into(), namespace:Name::new("fixture").unwrap(),cache_database:Name::new("cache").unwrap(),selection:root.join("selected.json") }
    }
    fn handle(label: &str) -> SnapshotHandle {
        SnapshotHandle { semantic:ContentHash::of(label.as_bytes()),realization:ContentHash::of(b"realization"),database:DatabaseIdentity {namespace:Name::new("fixture").unwrap(),database:Name::new(label).unwrap()} }
    }
    #[tokio::test]
    async fn atomic_selection_is_one_authority_and_lifecycle_is_serialized() {
        let scratch=tempfile::tempdir().unwrap(); let cfg=config(scratch.path());
        let old=handle("old"); let new=handle("new"); cfg.select(&old).unwrap();
        let viewer=ViewerConfig::read(&cfg.selection.with_extension("serving.json")).unwrap();
        let pinned=viewer.selected().unwrap();
        let held=cfg.lock_selection().await.unwrap();
        let competitor=config(scratch.path());
        assert!(tokio::time::timeout(std::time::Duration::from_millis(30),competitor.lock_selection()).await.is_err());
        cfg.select_locked(&new,&held).unwrap(); drop(held);
        let second=competitor.lock_selection().await.unwrap();
        assert_eq!(competitor.selected().unwrap(),new);
        assert_eq!(viewer.selected().unwrap(),new);
        assert_eq!(pinned,old);
        let alien=tempfile::tempdir().unwrap();
        assert!(config(alien.path()).select_locked(&old,&second).is_err());
    }
    #[tokio::test]
    async fn precommit_failure_keeps_selection_and_postcommit_uncertainty_keeps_new_handle() {
        let scratch=tempfile::tempdir().unwrap(); let cfg=config(scratch.path());
        let old=handle("old"); let new=handle("new"); cfg.select(&old).unwrap();
        let mut changed=config(scratch.path());changed.endpoint="grpc://127.0.0.1:9998".into();
        assert!(changed.select(&new).is_err());assert_eq!(cfg.selected().unwrap(),old);
        let guard=cfg.lock_selection().await.unwrap();
        let error=cfg.commit_selection(&new,&guard,|_|Err(std::io::Error::other("injected directory sync failure"))).unwrap_err();
        assert!(matches!(error,ModelError::Infrastructure {class:Infrastructure::Unconfirmed,..}));
        assert_eq!(cfg.selected().unwrap(),new);
        assert_eq!(ViewerConfig::read(&cfg.selection.with_extension("serving.json")).unwrap().selected().unwrap(),new);
    }
}
