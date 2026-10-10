//! Read-only domain session. Rust retains the private database principal needed for durable
//! reader pins; Python receives no credentials or arbitrary native-query operation.
use lctx_serving::NativeService;
use lctx_surrealdb::{NativeReader, config::ViewerConfig};
use pyo3::prelude::*;
use std::sync::{Arc, Condvar, Mutex};
struct State {
    service: Option<Arc<NativeService>>,
    active: usize,
    closing: bool,
}
struct Shared {
    state: Mutex<State>,
    drained: Condvar,
}
#[pyclass(module = "lctx_semantics", frozen)]
pub struct NativeSession {
    runtime: Arc<tokio::runtime::Runtime>,
    shared: Arc<Shared>,
    reader: NativeReader,
}
#[pymethods]
impl NativeSession {
    #[new]
    fn new(py: Python<'_>, serving_config: String) -> PyResult<Self> {
        py.detach(move || {
            let config = ViewerConfig::read(std::path::Path::new(&serving_config))
                .map_err(crate::model_error)?;
            let runtime = Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| crate::unavailable())?,
            );
            let reader = runtime
                .block_on(NativeReader::connect(
                    &config.endpoint,
                    &config.credentials(),
                    config.selected().map_err(crate::model_error)?,
                ))
                .map_err(crate::model_error)?;
            let service = Arc::new(
                NativeService::new(reader.clone(), config.serving_limits.clone().unwrap_or_default())
                    .map_err(crate::model_error)?,
            );
            Ok(Self {
                runtime,
                shared: Arc::new(Shared {
                    state: Mutex::new(State {
                        service: Some(service),
                        active: 0,
                        closing: false,
                    }),
                    drained: Condvar::new(),
                }),
                reader,
            })
        })
    }
    #[pyo3(signature=(tool,request_json,query_vector_json=None,remaining_deadline_ms=None))]
    fn execute(
        &self,
        py: Python<'_>,
        tool: String,
        request_json: String,
        query_vector_json: Option<String>,
        remaining_deadline_ms: Option<u64>,
    ) -> PyResult<Py<pyo3::types::PyString>> {
        let encoded = py.detach(|| {
            let service = {
                let mut state = self.shared.state.lock().map_err(|_| crate::unavailable())?;
                if state.closing {
                    return Err(crate::unavailable());
                }
                let service = state
                    .service
                    .as_ref()
                    .ok_or_else(crate::unavailable)?
                    .clone();
                state.active += 1;
                service
            };
            let _active = Active(self.shared.clone());
            let unavailable = query_vector_json.as_deref() == Some("unavailable");
            let vector = if unavailable {
                None
            } else {
                query_vector_json
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|_| {
                        crate::public_error(lctx_model::domain::serving::PublicFailure::new(
                            lctx_model::domain::serving::FailureKind::Incompatible,
                        ))
                    })?
            };
            let remaining =
                remaining_deadline_ms.unwrap_or(service.request_deadline_ms());
            self.runtime
                .block_on(service.execute_encoded_for(&tool, &request_json, vector, unavailable, remaining))
                .map_err(crate::error)
        })?;
        let result = pyo3::types::PyString::new(py, encoded.as_str()).unbind();
        drop(encoded);
        Ok(result)
    }
    fn handle_json(&self) -> PyResult<String> {
        serde_json::to_string(self.reader.handle()).map_err(|_| crate::unavailable())
    }
    /// Drain admitted calls before invalidating the one server session.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| {
            let mut state = self.shared.state.lock().map_err(|_| crate::unavailable())?;
            state.closing = true;
            while state.active > 0 {
                state = self
                    .shared
                    .drained
                    .wait(state)
                    .map_err(|_| crate::unavailable())?;
            }
            if let Some(service) = state.service.take() {
                drop(state);
                self.runtime
                    .block_on(async { service.close().await; self.reader.client().invalidate().await })
                    .map_err(|_| crate::unavailable())?;
            }
            Ok(())
        })
    }
}
struct Active(Arc<Shared>);
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.active = state.active.saturating_sub(1);
            self.0.drained.notify_all();
        }
    }
}
