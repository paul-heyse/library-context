//! The effectful async boundary. The semantic extension stays pure and Arrow/schema-owned.
use cpg_schema::{id::Digest, serving_projection::FailureKind};
use lctx_postgres::{
    Error,
    repository::{PinnedGeneration, Where},
    serving::{RoleConfig, ServingStore},
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::sync::{Arc, Once};

pyo3::create_exception!(lctx_storage, StorageError, PyRuntimeError);
fn error(e: Error) -> PyErr {
    if let Error::Request(message) = e {
        return PyValueError::new_err(message);
    }
    let kind = match &e {
        Error::Projection(e) => match e.kind {
            FailureKind::Unavailable => "unavailable",
            FailureKind::Incomplete => "incomplete",
            FailureKind::Corrupt => "corrupt",
            FailureKind::Incompatible => "incompatible",
            FailureKind::ResourceRefused => "resource_refused",
        },
        Error::Config(_) | Error::Schema => "incompatible",
        Error::Integrity(_) => "corrupt",
        _ => "unavailable",
    };
    let err = StorageError::new_err(e.to_string());
    Python::attach(|py| {
        let _ = err.value(py).setattr("kind", kind);
    });
    err
}
#[pyclass(module = "lctx_storage", frozen)]
struct Repository {
    store: ServingStore,
}
#[pymethods]
impl Repository {
    #[pyo3(signature=(library,generation=None,profile=None))]
    fn pin<'py>(
        &self,
        py: Python<'py>,
        library: String,
        generation: Option<String>,
        profile: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let generation = generation
            .map(|s| {
                Digest::from_hex(&s)
                    .ok_or_else(|| PyValueError::new_err("invalid generation digest"))
            })
            .transpose()?;
        let profile = profile
            .map(|s| {
                Digest::from_hex(&s).ok_or_else(|| PyValueError::new_err("invalid profile digest"))
            })
            .transpose()?;
        let store = self.store.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let pinned = Arc::new(
                store
                    .pin(&library, generation, profile)
                    .await
                    .map_err(error)?,
            );
            Ok(PinnedRepository { store, pinned })
        })
    }
    fn check<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            serde_json::to_string(&store.check().await.map_err(error)?)
                .map_err(|_| PyValueError::new_err("health encoding"))
        })
    }
    fn close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            store.close().await;
            Ok(())
        })
    }
}
#[pyfunction]
fn open_repository(py: Python<'_>, config_path: std::path::PathBuf) -> PyResult<Bound<'_, PyAny>> {
    // Config I/O occurs only on explicit open, never module import, and outside the event loop.
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let config = tokio::task::spawn_blocking(move || RoleConfig::load(&config_path))
            .await
            .map_err(|_| PyRuntimeError::new_err("configuration task failed"))?
            .map_err(error)?;
        Ok(Repository {
            store: config.open_serving().await.map_err(error)?,
        })
    })
}
#[pymodule]
fn _storage(m: &Bound<'_, PyModule>) -> PyResult<()> {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder
            .worker_threads(2)
            .max_blocking_threads(4)
            .enable_all();
        pyo3_async_runtimes::tokio::init(builder);
    });
    m.add("StorageError", m.py().get_type::<StorageError>())?;
    m.add_class::<Repository>()?;
    m.add_class::<PinnedRepository>()?;
    m.add_function(wrap_pyfunction!(open_repository, m)?)?;
    Ok(())
}

#[pyclass(module = "lctx_storage", frozen)]
struct PinnedRepository {
    store: ServingStore,
    pinned: Arc<PinnedGeneration>,
}
fn encode(value: impl serde::Serialize) -> PyResult<String> {
    serde_json::to_string(&value).map_err(|_| PyValueError::new_err("response encoding"))
}
fn filter(value: &str) -> PyResult<Where> {
    let filter: Where = serde_json::from_str(value)
        .map_err(|_| PyValueError::new_err("invalid operation filter"))?;
    filter.validate().map_err(|e| error(e.into()))?;
    Ok(filter)
}
#[pymethods]
impl PinnedRepository {
    #[pyo3(signature=(verify_artifacts=false))]
    fn diagnostics<'py>(
        &self,
        py: Python<'py>,
        verify_artifacts: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let id = self.pinned.id();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .diagnostics(id, verify_artifacts)
                    .await
                    .map_err(error)?,
            )
        })
    }
    fn descriptor(&self) -> PyResult<String> {
        encode(self.pinned.descriptor())
    }
    fn inputs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let inputs = tokio::task::spawn_blocking(move || {
                let mut out = Vec::new();
                let mut total = 0;
                for (name, path) in pinned.artifacts() {
                    use std::io::Read;
                    let expected = pinned.manifest().artifacts[name].bytes;
                    let metadata = std::fs::symlink_metadata(path).map_err(|_| {
                        error(Error::Projection(
                            cpg_schema::serving_projection::ProjectionError {
                                kind: cpg_schema::serving_projection::FailureKind::Unavailable,
                                message: "artifact unavailable".into(),
                            },
                        ))
                    })?;
                    if !metadata.is_file() || metadata.len() != expected {
                        return Err(error(Error::Projection(
                            cpg_schema::serving_projection::corrupt("artifact size/type mismatch"),
                        )));
                    }
                    let mut bytes = Vec::new();
                    std::fs::File::open(path)
                        .map_err(|_| {
                            error(Error::Projection(
                                cpg_schema::serving_projection::ProjectionError {
                                    kind: cpg_schema::serving_projection::FailureKind::Unavailable,
                                    message: "artifact unavailable".into(),
                                },
                            ))
                        })?
                        .take(expected + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|_| {
                            error(Error::Projection(
                                cpg_schema::serving_projection::ProjectionError {
                                    kind: cpg_schema::serving_projection::FailureKind::Unavailable,
                                    message: "artifact read failed".into(),
                                },
                            ))
                        })?;
                    total += bytes.len();
                    if total > 512 * 1024 * 1024 {
                        return Err(error(Error::Projection(
                            cpg_schema::serving_projection::refused("artifact memory budget"),
                        )));
                    }
                    pinned
                        .manifest()
                        .verify_artifact(name, &bytes)
                        .map_err(|e| error(Error::Projection(e)))?;
                    out.push((name.trim_end_matches(".arrow").to_owned(), bytes));
                }
                Ok::<_, PyErr>(out)
            })
            .await
            .map_err(|_| PyRuntimeError::new_err("artifact task failed"))??;
            Python::attach(|py| {
                let out = PyDict::new(py);
                for (name, bytes) in inputs {
                    out.set_item(name, PyBytes::new(py, &bytes))?;
                }
                Ok(out.unbind())
            })
        })
    }
    fn resolve<'py>(&self, py: Python<'py>, operation: String) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(store.resolve(&pinned, &operation).await.map_err(error)?)
        })
    }
    fn get_operation<'py>(
        &self,
        py: Python<'py>,
        snapshot: String,
        operation: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .get_operation(&pinned, &snapshot, &operation)
                    .await
                    .map_err(error)?,
            )
        })
    }
    fn get_capability<'py>(
        &self,
        py: Python<'py>,
        snapshot: String,
        capability: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .get_capability(&pinned, &snapshot, &capability)
                    .await
                    .map_err(error)?,
            )
        })
    }
    #[pyo3(signature=(where_json,limit=20,cursor=None))]
    fn find_operations<'py>(
        &self,
        py: Python<'py>,
        where_json: String,
        limit: u32,
        cursor: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let filter = filter(&where_json)?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .find_operations(&pinned, &filter, limit, cursor.as_deref())
                    .await
                    .map_err(error)?,
            )
        })
    }
    #[pyo3(signature=(query,operations,where_json=None))]
    fn search_scope<'py>(
        &self,
        py: Python<'py>,
        query: String,
        operations: bool,
        where_json: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let filter = where_json.as_deref().map(filter).transpose()?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .search_scope(&pinned, filter.as_ref(), &query, operations)
                    .await
                    .map_err(error)?,
            )
        })
    }
    #[pyo3(signature=(query,spec,operations,where_json=None,limit=10))]
    fn vector_ranks<'py>(
        &self,
        py: Python<'py>,
        query: Vec<f32>,
        spec: String,
        operations: bool,
        where_json: Option<String>,
        limit: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let filter = where_json.as_deref().map(filter).transpose()?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let result = store
                .vector_ranks(&pinned, &query, &spec, operations, filter.as_ref(), limit)
                .await
                .map_err(error)?;
            let metadata = encode(result.metadata)?;
            Python::attach(|py| Ok((metadata, PyBytes::new(py, &result.ipc).unbind())))
        })
    }
    fn hit_records<'py>(
        &self,
        py: Python<'py>,
        entities: Vec<String>,
        operations: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .hit_records(&pinned, &entities, operations)
                    .await
                    .map_err(error)?,
            )
        })
    }
}
