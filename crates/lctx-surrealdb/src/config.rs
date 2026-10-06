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
    /// Explicit operator selection; ordinary compilation and publication do not call this.
    pub fn select(&self, handle: &SnapshotHandle) -> Result<(), ModelError> {
        let parent = self
            .selection
            .parent()
            .ok_or(ModelError::Schema("snapshot selection path"))?;
        std::fs::create_dir_all(parent).map_err(ModelError::codec)?;
        let staged = self
            .selection
            .with_extension(format!("{}.new", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
            .map_err(ModelError::codec)?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec_pretty(handle).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)?;
        file.sync_all().map_err(ModelError::codec)?;
        let viewer = ViewerConfig {
            endpoint: self.endpoint.clone(),
            username: self.viewer_username.clone(),
            password: self.viewer_password.clone(),
            snapshot: handle.clone(),
        };
        let serving_path = self.selection.with_extension("serving.json");
        use std::os::unix::fs::OpenOptionsExt;
        let serving_staged = serving_path.with_extension(format!("{}.new", std::process::id()));
        let mut serving = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&serving_staged)
            .map_err(ModelError::codec)?;
        serving
            .write_all(&serde_json::to_vec(&viewer).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)?;
        serving.sync_all().map_err(ModelError::codec)?;
        std::fs::rename(&serving_staged, &serving_path).map_err(ModelError::codec)?;
        std::fs::rename(&staged, &self.selection).map_err(ModelError::codec)?;
        std::fs::File::open(parent)
            .map_err(ModelError::codec)?
            .sync_all()
            .map_err(ModelError::codec)
    }
}

/// Minimal serving credentials: no administrator or cache writer secret crosses this boundary.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewerConfig {
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub snapshot: SnapshotHandle,
}
impl ViewerConfig {
    pub fn read(path: &Path) -> Result<Self, ModelError> {
        serde_json::from_slice(&std::fs::read(path).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)
    }
    pub fn credentials(&self) -> Credentials {
        Credentials::Database {
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}
