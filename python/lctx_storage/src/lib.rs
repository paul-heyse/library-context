//! The effectful async boundary. The semantic extension stays pure and Arrow/schema-owned.
use lctx_postgres::{
    Error,
    serving::{RoleConfig, ServingStore},
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::sync::Once;

pyo3::create_exception!(lctx_storage, StorageError, PyRuntimeError);
fn error(e: Error) -> PyErr {
    let kind = match &e {
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
    m.add_function(wrap_pyfunction!(open_repository, m)?)?;
    Ok(())
}
