//! The effectful async boundary. The semantic extension stays pure and Arrow/schema-owned.
use cpg_schema::{id::Digest, serving_projection::FailureKind};
use lctx_postgres::{
    Error,
    repository::PinnedGeneration,
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
    m.add_class::<PreparedSelectionHandle>()?;
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
    #[pyo3(signature=(snapshot,operation,expanded=false,evidence_limit=20,evidence_cursor=None))]
    fn get_operation<'py>(
        &self,
        py: Python<'py>,
        snapshot: String,
        operation: String,
        expanded: bool,
        evidence_limit: u32,
        evidence_cursor: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .get_operation_with_evidence(
                        &pinned,
                        &snapshot,
                        &operation,
                        &lctx_postgres::EvidenceOptions {
                            expanded,
                            limit: evidence_limit,
                            cursor: evidence_cursor,
                        },
                    )
                    .await
                    .map_err(error)?,
            )
        })
    }
    fn get_evidence<'py>(&self, py: Python<'py>, request: String) -> PyResult<Bound<'py, PyAny>> {
        let request: cpg_schema::wire::GetEvidenceRequest =
            serde_json::from_str(&request).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let result = match request.evidence {
                cpg_schema::wire::EvidenceTarget::Original(reference) => {
                    store
                        .get_evidence(
                            &pinned,
                            request.snapshot_id.hex().as_str(),
                            reference,
                            request.cursor.as_ref().map(|c| c.as_str()),
                            request.expanded,
                        )
                        .await
                }
                cpg_schema::wire::EvidenceTarget::Retrieval(
                    cpg_schema::wire::RetrievalTarget::RetrievalUnit(unit),
                ) => {
                    store
                        .get_retrieval_unit(
                            &pinned,
                            request.snapshot_id.hex().as_str(),
                            unit,
                            request.cursor.as_ref().map(|c| c.as_str()),
                            request.expanded,
                        )
                        .await
                }
            }
            .map_err(error)?;
            encode(result)
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
    #[pyo3(signature=(selection_json="{}".to_owned(),query="".to_owned()))]
    fn prepare_selection<'py>(
        &self,
        py: Python<'py>,
        selection_json: String,
        query: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let selection: cpg_schema::wire::Selection = serde_json::from_str(&selection_json)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let prepared = store
                .prepare_selection(&pinned, &selection, &query)
                .await
                .map_err(error)?;
            Ok(PreparedSelectionHandle {
                store,
                pinned,
                prepared,
            })
        })
    }
    #[pyo3(signature=(selection_json,limit=20,cursor=None))]
    fn find_operations<'py>(
        &self,
        py: Python<'py>,
        selection_json: String,
        limit: u32,
        cursor: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let selection: cpg_schema::wire::Selection = serde_json::from_str(&selection_json)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(
                store
                    .find_operations(&pinned, &selection, limit, cursor.as_deref())
                    .await
                    .map_err(error)?,
            )
        })
    }
    #[pyo3(signature=(query))]
    fn search_scope<'py>(&self, py: Python<'py>, query: String) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            encode(store.search_scope(&pinned, &query).await.map_err(error)?)
        })
    }
    #[pyo3(signature=(query,spec,limit=10))]
    fn vector_ranks<'py>(
        &self,
        py: Python<'py>,
        query: Vec<f32>,
        spec: String,
        limit: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let result = store
                .vector_ranks(&pinned, &query, &spec, limit)
                .await
                .map_err(error)?;
            let metadata = encode(result.metadata)?;
            Python::attach(|py| Ok((metadata, PyBytes::new(py, &result.ipc).unbind())))
        })
    }
    fn brief_rank_schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = lctx_postgres::retrieval::rank_schema_ipc().map_err(error)?;
        Ok(PyBytes::new(py, &bytes))
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

/// Immutable request-local handle. Holds no query lease and cannot be constructed from Python.
#[pyclass(module = "lctx_storage", frozen)]
struct PreparedSelectionHandle {
    store: ServingStore,
    pinned: Arc<PinnedGeneration>,
    prepared: lctx_postgres::selection::PreparedSelection,
}
#[pymethods]
impl PreparedSelectionHandle {
    fn scope(&self, py: Python<'_>) -> PyResult<String> {
        py.detach(|| encode(self.prepared.scope()))
    }
    fn rank_schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(
            py,
            &lctx_postgres::selection::encode_winners(&[]).map_err(error)?,
        ))
    }
    #[pyo3(signature=(limit=20,cursor=None))]
    fn page(&self, py: Python<'_>, limit: u32, cursor: Option<String>) -> PyResult<String> {
        py.detach(|| {
            encode(
                self.prepared
                    .page(&self.pinned, limit, cursor.as_deref())
                    .map_err(error)?,
            )
        })
    }
    fn vectors<'py>(
        &self,
        py: Python<'py>,
        query: Vec<f32>,
        spec: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        let prepared = self.prepared.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let bytes = store
                .selection_vectors(&pinned, &prepared, &query, &spec)
                .await
                .map_err(error)?;
            Python::attach(|py| Ok(PyBytes::new(py, &bytes).unbind()))
        })
    }
    #[pyo3(signature=(ranked_json,channel_state,limit=20,cursor=None))]
    fn finish<'py>(
        &self,
        py: Python<'py>,
        ranked_json: String,
        channel_state: String,
        limit: u32,
        cursor: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if ranked_json.len() > 32 * 1024 * 1024 {
            return Err(PyValueError::new_err(
                "resource_refused: ranked input budget",
            ));
        }
        let store = self.store.clone();
        let pinned = self.pinned.clone();
        let prepared = self.prepared.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let ranked = store
                .run_cpu(move || {
                    serde_json::from_str(&ranked_json).map_err(|_| {
                        lctx_postgres::Error::Request("invalid ranked selection".into())
                    })
                })
                .await
                .map_err(error)?;
            encode(
                store
                    .finish_selection_search(
                        &pinned,
                        &prepared,
                        ranked,
                        &channel_state,
                        limit,
                        cursor.as_deref(),
                    )
                    .await
                    .map_err(error)?,
            )
        })
    }
}
