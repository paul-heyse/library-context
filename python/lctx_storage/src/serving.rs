//! Python numerical callbacks execute within the original generation's granted CPU lifetime.
use lctx_model::domain::serving::{self as wire, Request, ResourceLimits, ranking::DocumentScore};
use lctx_postgres::{
    generations::{
        Error, FailureClass, GenerationId, PreparedReservation, RequestExecution, ServingService,
    },
    roles::RoleConfig,
};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PySequence, PyString, PyTuple},
};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};
pyo3::create_exception!(lctx_storage, StorageError, PyRuntimeError);
fn error(error: Error) -> PyErr {
    let kind = match FailureClass::of(&error) {
        FailureClass::Resource | FailureClass::Limit | FailureClass::Contention => {
            "resource_refused"
        }
        FailureClass::Contract | FailureClass::Frontier => "incompatible",
        FailureClass::Invalid | FailureClass::Codec => "corrupt",
        _ => "unavailable",
    };
    // No SQL driver, bound value or configuration text crosses the public boundary.
    refused(kind, "canonical serving request refused")
}
fn refused(kind: &'static str, message: &'static str) -> PyErr {
    let result = StorageError::new_err(format!("{kind}: {message}"));
    Python::attach(|py| {
        let _ = result.value(py).setattr("kind", kind);
    });
    result
}
fn invalid(message: impl std::fmt::Display) -> PyErr {
    let mut message = message.to_string();
    if message.len() > 256 {
        let mut end = 256;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    PyValueError::new_err(message)
}
fn encoding(_: serde_json::Error) -> Error {
    Error::Codec("serving bridge encoding".into())
}
struct CountWrite(usize);
impl std::io::Write for CountWrite {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("bridge byte count overflow"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn serialized_len<T: serde::Serialize>(value: &T) -> Result<usize, Error> {
    let mut count = CountWrite(0);
    serde_json::to_writer(&mut count, value).map_err(encoding)?;
    Ok(count.0)
}
struct NumericalState {
    _reservation: PreparedReservation,
    documents: usize,
}
struct State {
    service: ServingService,
    numerical: Mutex<Option<NumericalState>>,
}
#[pyclass(module = "lctx_storage", frozen)]
pub struct Service {
    state: Arc<State>,
}
#[pyclass(module = "lctx_storage", frozen)]
pub struct RequestGrant {
    state: Arc<State>,
    execution: Mutex<Option<RequestExecution>>,
}
impl RequestGrant {
    fn execution(&self, state: &Arc<State>) -> PyResult<RequestExecution> {
        if !Arc::ptr_eq(&self.state, state) {
            return Err(invalid("request grant belongs to another service"));
        }
        self.execution
            .lock()
            .map_err(|_| PyRuntimeError::new_err("request grant state"))?
            .as_ref()
            .cloned()
            .ok_or_else(|| invalid("request grant released"))
    }
}
#[pymethods]
impl RequestGrant {
    fn remaining_seconds(&self) -> PyResult<f64> {
        let execution = self.execution(&self.state)?;
        execution
            .remaining()
            .map(|remaining| remaining.as_secs_f64())
            .map_err(error)
    }
    fn release(&self) -> PyResult<()> {
        self.execution
            .lock()
            .map_err(|_| PyRuntimeError::new_err("request grant state"))?
            .take();
        Ok(())
    }
}
fn query(request: &Request) -> Option<&str> {
    match request {
        Request::SearchOperations(r) => Some(r.query.as_str()),
        Request::SearchEvidence(r) => Some(r.query.as_str()),
        Request::SearchCapabilities(r) => Some(r.query.as_str()),
        _ => None,
    }
}
fn json_lower_add(bytes: &mut usize, additional: usize, maximum: usize) -> PyResult<()> {
    *bytes = bytes
        .checked_add(additional)
        .filter(|n| *n <= maximum)
        .ok_or_else(|| refused("resource_refused", "request wire bytes"))?;
    Ok(())
}
// Inspect only builtin JSON values before a trusted synchronous codec materializes them.
// Active ancestors detect cycles; shared acyclic containers retain their repeated JSON cost.
fn preflight_json<'py>(
    execution: &RequestExecution,
    budget: &lctx_model::domain::resources::ResourceBudget,
    value: Bound<'py, PyAny>,
    maximum: usize,
) -> PyResult<()> {
    let mut scratch = budget
        .reserve("python-json-traversal", 4096)
        .map_err(|e| error(e.into()))?;
    let mut pending = vec![(value, false)];
    let mut active = BTreeSet::new();
    let mut bytes = 1usize;
    let mut work = 0usize;
    while let Some((value, leave)) = pending.pop() {
        work += 1;
        if work.is_multiple_of(256) {
            execution.remaining().map_err(error)?;
        }
        let identity = value.as_ptr() as usize;
        if leave {
            active.remove(&identity);
            continue;
        }
        if value.is_none() {
            json_lower_add(&mut bytes, 3, maximum)?;
            continue;
        }
        if value.is_exact_instance_of::<PyBool>() {
            json_lower_add(&mut bytes, 3, maximum)?;
            continue;
        }
        if value.is_exact_instance_of::<PyString>() {
            let length = value
                .cast::<PyString>()
                .map_err(|_| refused("corrupt", "JSON string"))?
                .len()
                .map_err(|_| refused("corrupt", "JSON string"))?;
            json_lower_add(&mut bytes, length.saturating_add(1), maximum)?;
            continue;
        }
        if value.is_exact_instance_of::<PyInt>() {
            let bits = value
                .call_method0("bit_length")
                .and_then(|n| n.extract::<usize>())
                .map_err(|_| refused("corrupt", "JSON integer"))?;
            // 301/1000 is below log10(2), giving a lower bound without decimal allocation.
            let digits = bits
                .saturating_sub(1)
                .checked_mul(301)
                .map(|n| n / 1000 + 1)
                .ok_or_else(|| refused("resource_refused", "request wire bytes"))?;
            json_lower_add(&mut bytes, digits - 1, maximum)?;
            continue;
        }
        if value.is_exact_instance_of::<PyFloat>() {
            if !value
                .extract::<f64>()
                .map_err(|_| refused("corrupt", "JSON float"))?
                .is_finite()
            {
                return Err(refused("corrupt", "nonfinite JSON scalar"));
            }
            continue;
        }
        let dictionary = value.is_exact_instance_of::<PyDict>();
        let list = value.is_exact_instance_of::<PyList>();
        let tuple = value.is_exact_instance_of::<PyTuple>();
        if !dictionary && !list && !tuple {
            return Err(refused("corrupt", "JSON arguments require builtin values"));
        }
        if active.contains(&identity) {
            return Err(refused("corrupt", "cyclic JSON arguments"));
        }
        let length = if dictionary {
            value
                .cast::<PyDict>()
                .map_err(|_| refused("corrupt", "JSON object"))?
                .len()
        } else if list {
            value
                .cast::<PyList>()
                .map_err(|_| refused("corrupt", "JSON array"))?
                .len()
        } else {
            value
                .cast::<PyTuple>()
                .map_err(|_| refused("corrupt", "JSON array"))?
                .len()
        };
        // Quotes/colon for each object key and one minimum byte per child are counted now,
        // before allocating traversal slots. Each later scalar replaces its one-byte minimum.
        let minimum = length
            .checked_mul(if dictionary { 4 } else { 1 })
            .and_then(|n| n.checked_add(length.saturating_sub(1)))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| refused("resource_refused", "request wire bytes"))?;
        json_lower_add(&mut bytes, minimum, maximum)?;
        let slots = pending
            .len()
            .checked_add(length)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| refused("resource_refused", "JSON traversal bytes"))?;
        let allowance = slots
            .checked_mul(2 * std::mem::size_of::<(Bound<'py, PyAny>, bool)>())
            .and_then(|n| n.checked_add((active.len() + 1).saturating_mul(128)))
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(|| refused("resource_refused", "JSON traversal bytes"))?;
        scratch
            .try_resize(scratch.size().max(allowance))
            .map_err(|e| error(e.into()))?;
        pending
            .try_reserve_exact(length + 1)
            .map_err(|_| refused("resource_refused", "JSON traversal allocation"))?;
        active.insert(identity);
        pending.push((value.clone(), true));
        if dictionary {
            for (key, child) in value
                .cast::<PyDict>()
                .map_err(|_| refused("corrupt", "JSON object"))?
                .iter()
            {
                if !key.is_exact_instance_of::<PyString>() {
                    return Err(refused(
                        "corrupt",
                        "JSON object keys require builtin strings",
                    ));
                }
                json_lower_add(
                    &mut bytes,
                    key.cast::<PyString>()
                        .map_err(|_| refused("corrupt", "JSON object key"))?
                        .len()
                        .map_err(|_| refused("corrupt", "JSON object key"))?,
                    maximum,
                )?;
                pending.push((child, false));
            }
        } else if list {
            for child in value
                .cast::<PyList>()
                .map_err(|_| refused("corrupt", "JSON array"))?
                .iter()
            {
                pending.push((child, false));
            }
        } else {
            for child in value
                .cast::<PyTuple>()
                .map_err(|_| refused("corrupt", "JSON array"))?
                .iter()
            {
                pending.push((child, false));
            }
        }
    }
    Ok(())
}
async fn decode(
    execution: &RequestExecution,
    tool: Py<PyString>,
    raw: Py<PyString>,
) -> PyResult<Request> {
    let retained = execution.clone();
    execution
        .cpu(move |budget| {
            Python::attach(|py| {
                let name = tool.bind(py);
                let raw = raw.bind(py);
                if !name.is_exact_instance_of::<PyString>()
                    || !raw.is_exact_instance_of::<PyString>()
                {
                    return Ok(Err(invalid("exact builtin strings required")));
                }
                let maximum_name = wire::Tool::ALL
                    .iter()
                    .map(|tool| tool.name().len())
                    .max()
                    .unwrap_or(0);
                let name_chars = name.len().map_err(|_| Error::Contract)?;
                if name_chars > maximum_name {
                    return Ok(Err(invalid("unknown serving tool")));
                }
                let _name = budget.reserve("python-tool-name-utf8", 8192)?;
                let name = match name.to_str() {
                    Ok(value) => value,
                    Err(_) => return Ok(Err(invalid("invalid serving tool encoding"))),
                };
                if wire::Tool::from_name(name).is_err() {
                    return Ok(Err(invalid("unknown serving tool")));
                }
                let raw_chars = raw.len().map_err(|_| Error::Contract)?;
                let maximum_bytes = ResourceLimits::default().expanded_response_bytes as usize;
                if raw_chars > maximum_bytes {
                    return Ok(Err(refused("resource_refused", "request wire bytes")));
                }
                // CPython may materialize its UTF-8 cache when borrowed. Admit that conversion before
                // borrowing, then enforce the exact UTF-8 wire bound before parsing or copying.
                let _unicode = budget.reserve(
                    "python-request-utf8",
                    raw_chars
                        .checked_mul(4)
                        .and_then(|n| n.checked_add(8192))
                        .ok_or(Error::ResourceRefused("request utf8 bytes"))?,
                )?;
                let raw = match raw.to_str() {
                    Ok(value) => value,
                    Err(_) => return Ok(Err(invalid("invalid request encoding"))),
                };
                if raw.len() > maximum_bytes {
                    return Ok(Err(refused("resource_refused", "request wire bytes")));
                }
                let bytes = raw
                    .len()
                    .checked_mul(6)
                    .and_then(|n| n.checked_add(8192))
                    .ok_or(Error::ResourceRefused("request decode bytes"))?;
                let _charge = budget.reserve("python-request-decode", bytes)?;
                let request =
                    wire::decode_request(name, raw, &ResourceLimits::default()).map_err(|error| {
                        match error {
                            wire::WireError::ResourceRefused(_) => {
                                refused("resource_refused", "request wire limit")
                            }
                            other => invalid(other),
                        }
                    });
                if request.is_ok() {
                    retained.retain("python-decoded-request-handoff", bytes)?;
                }
                Ok(request)
            })
        })
        .await
        .map_err(error)?
}
async fn vector(
    execution: &RequestExecution,
    value: Option<Py<PyAny>>,
) -> PyResult<Option<Vec<f32>>> {
    let retained = execution.clone();
    execution
        .cpu(move |budget| {
            Python::attach(|py| {
                let Some(value) = value else {
                    return Ok(Ok(None));
                };
                let value = match value.bind(py).cast::<PySequence>() {
                    Ok(value) => value,
                    Err(_) => return Ok(Err(refused("corrupt", "query vector sequence contract"))),
                };
                if value.len().map_err(|_| Error::Contract)? != 1024 {
                    return Ok(Err(refused("corrupt", "query vector dimension contract")));
                }
                let _charge = budget.reserve("python-query-vector-conversion", 1024 * 32 + 8192)?;
                let mut vector = Vec::with_capacity(1024);
                for index in 0..1024 {
                    let component = match value.get_item(index) {
                        Ok(value) => value,
                        Err(_) => {
                            return Ok(Err(refused("corrupt", "query vector component contract")));
                        }
                    };
                    if component.is_instance_of::<PyBool>() {
                        return Ok(Err(refused("corrupt", "query vector component contract")));
                    }
                    let component = match component.extract::<f32>() {
                        Ok(value) if value.is_finite() => value,
                        _ => return Ok(Err(refused("corrupt", "query vector component contract"))),
                    };
                    vector.push(component);
                }
                if lctx_model::domain::embedding::check_vector(&vector, 1024).is_err() {
                    return Ok(Err(refused("corrupt", "query vector model contract")));
                }
                retained.retain("python-query-vector", 1024 * 16 + 512)?;
                Ok(Ok(Some(vector)))
            })
        })
        .await
        .map_err(error)?
}

#[pymethods]
impl Service {
    fn admit<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let state = self.state.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let execution = state.service.runtime().execution().await.map_err(error)?;
            Ok(RequestGrant {
                state,
                execution: Mutex::new(Some(execution)),
            })
        })
    }
    fn encode_request<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        arguments: Py<PyDict>,
        encoder: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            execution
                .cpu(move |budget| {
                    Python::attach(|py| {
                        let maximum = ResourceLimits::default().expanded_response_bytes as usize;
                        if let Err(error) = preflight_json(
                            &retained,
                            budget,
                            arguments.bind(py).as_any().clone(),
                            maximum,
                        ) {
                            return Ok(Err(error));
                        }
                        let _codec = budget.reserve("python-request-codec", maximum * 64 + 8192)?;
                        let encoded = match encoder.bind(py).call1((arguments.bind(py),)) {
                            Ok(value) => value,
                            Err(_) => {
                                return Ok(Err(refused("corrupt", "request encoder refused")));
                            }
                        };
                        if !encoded.is_exact_instance_of::<PyString>() {
                            return Ok(Err(refused(
                                "corrupt",
                                "request encoder requires builtin string",
                            )));
                        }
                        let encoded = encoded.cast::<PyString>().map_err(|_| Error::Contract)?;
                        if encoded.len().map_err(|_| Error::Contract)? > maximum {
                            return Ok(Err(refused("resource_refused", "request wire bytes")));
                        }
                        let encoded = match encoded.to_str() {
                            Ok(value) => value,
                            Err(_) => return Ok(Err(refused("corrupt", "request wire encoding"))),
                        };
                        if encoded.len() > maximum {
                            return Ok(Err(refused("resource_refused", "request wire bytes")));
                        }
                        retained
                            .retain("python-request-codec-handoff", encoded.len() * 4 + 8192)?;
                        Ok(Ok(encoded.to_owned()))
                    })
                })
                .await
                .map_err(error)?
        })
    }
    fn encode_envelope<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        encoder: Py<PyAny>,
        expanded: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            execution
                .cpu(move |budget| {
                    Python::attach(|py| {
                        let maximum = ResourceLimits::default().response_bytes(expanded) as usize;
                        // Both pinned transport encodings and their JSON validation are admitted before
                        // the trusted synchronous serializer runs; this is not a Python RSS reservation.
                        let _codec =
                            budget.reserve("python-envelope-codec", maximum * 64 + 8192)?;
                        let encoded = match encoder.bind(py).call0() {
                            Ok(value) => value,
                            Err(_) => {
                                return Ok(Err(refused("corrupt", "envelope encoder refused")));
                            }
                        };
                        if !encoded.is_exact_instance_of::<PyTuple>() {
                            return Ok(Err(refused(
                                "corrupt",
                                "envelope encoder requires builtin tuple",
                            )));
                        }
                        let encoded = encoded.cast::<PyTuple>().map_err(|_| Error::Contract)?;
                        if encoded.len() != 2 {
                            return Ok(Err(refused(
                                "corrupt",
                                "envelope encoder requires two encodings",
                            )));
                        }
                        let left = encoded.get_item(0).map_err(|_| Error::Contract)?;
                        let right = encoded.get_item(1).map_err(|_| Error::Contract)?;
                        if !left.is_exact_instance_of::<PyBytes>()
                            || !right.is_exact_instance_of::<PyBytes>()
                        {
                            return Ok(Err(refused(
                                "corrupt",
                                "envelope encoder requires builtin bytes",
                            )));
                        }
                        let left = left.cast::<PyBytes>().map_err(|_| Error::Contract)?;
                        let right = right.cast::<PyBytes>().map_err(|_| Error::Contract)?;
                        for bytes in [left.as_bytes(), right.as_bytes()] {
                            if bytes.len() > maximum {
                                return Ok(Err(refused(
                                    "resource_refused",
                                    "final MCP envelope bytes",
                                )));
                            }
                            let raw = match std::str::from_utf8(bytes) {
                                Ok(value) => value,
                                Err(_) => {
                                    return Ok(Err(refused(
                                        "corrupt",
                                        "final MCP envelope encoding",
                                    )));
                                }
                            };
                            if let Err(error) = wire::admit_envelope(raw, expanded) {
                                return Ok(Err(match error {
                                    wire::WireError::ResourceRefused(_) => {
                                        refused("resource_refused", "final MCP envelope bytes")
                                    }
                                    _ => refused("corrupt", "final MCP envelope contract"),
                                }));
                            }
                        }
                        retained.retain(
                            "python-envelope-codec-handoff",
                            (left.as_bytes().len() + right.as_bytes().len()) * 3 + 8192,
                        )?;
                        Ok(Ok((left.clone().unbind(), right.clone().unbind())))
                    })
                })
                .await
                .map_err(error)?
        })
    }
    fn request_info<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        tool: Py<PyString>,
        raw: Py<PyString>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let request = decode(&execution, tool, raw).await?;
            let retained = execution.clone();
            execution.cpu(move|_|{let encoded=serde_json::to_string(&serde_json::json!({"query":query(&request),"expanded":request.page().expanded})).map_err(encoding)?;retained.retain("python-request-info",encoded.len()*3+512)?;Ok(encoded)}).await.map_err(error)
        })
    }
    fn embedding_spec<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let state = self.state.clone();
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            execution
                .cpu(move |budget| {
                    let _charge = budget.reserve("python-embedding-spec-encode", 8192)?;
                    let encoded = state
                        .service
                        .retrieval()
                        .embedding_spec()
                        .map(|s| s.canonical_json());
                    if let Some(value) = &encoded {
                        retained.retain("python-embedding-spec", value.len() * 3 + 512)?;
                    }
                    Ok(encoded)
                })
                .await
                .map_err(error)
        })
    }
    fn initialize_numerical<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        callback: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let state = self.state.clone();
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let reservation = state
                .service
                .retrieval()
                .reserve_numerical()
                .await
                .map_err(error)?;
            execution
                .cpu(move |budget| {
                    let mut numerical = state.numerical.lock().map_err(|_| Error::State)?;
                    if numerical.is_some() {
                        return Ok(Err(invalid("numerical corpus already initialized")));
                    }
                    let corpus = state.service.retrieval().numerical_corpus(&retained)?;
                    let bytes = corpus
                        .documents
                        .iter()
                        .try_fold(8192usize, |n, d| {
                            n.checked_add(
                                d.text.len() * 8
                                    + d.tokens.iter().map(|t| t.len() * 8 + 192).sum::<usize>()
                                    + 1024,
                            )
                        })
                        .ok_or(Error::ResourceRefused("numerical corpus bytes"))?;
                    let _charge = budget.reserve("python-numerical-corpus-json", bytes)?;
                    let encoded = serde_json::to_string(&corpus).map_err(encoding)?;
                    let result = Python::attach(|py| {
                        callback
                            .bind(py)
                            .call1((encoded,))
                            .map(|_| ())
                            .map_err(|_| {
                                refused("unavailable", "numerical initialization callback refused")
                            })
                    });
                    if result.is_ok() {
                        *numerical = Some(NumericalState {
                            _reservation: reservation,
                            documents: corpus.documents.len(),
                        });
                    }
                    Ok(result)
                })
                .await
                .map_err(error)?
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The Python call boundary keeps explicit tool, vector and numerical parameters"
    )]
    #[pyo3(signature=(grant,tool,raw,query_vector=None,degradation=None,numerical_callback=None))]
    fn dispatch<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        tool: Py<PyString>,
        raw: Py<PyString>,
        query_vector: Option<Py<PyAny>>,
        degradation: Option<String>,
        numerical_callback: Option<Py<PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let state = self.state.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let request = decode(&execution, tool, raw).await?;
            let query_vector = vector(&execution, query_vector).await?;
            let response = if let Some(query) = query(&request).map(str::to_owned) {
                let callback = numerical_callback
                    .ok_or_else(|| invalid("search requires numerical callback"))?;
                let ranking = state
                    .service
                    .retrieval()
                    .request(&execution, &request, query_vector.clone(), degradation)
                    .await
                    .map_err(error)?;
                let retained = execution.clone();
                let scoring = state.clone();
                let (ranking, scores) = execution
                    .cpu(move |budget| {
                        let documents = scoring
                            .numerical
                            .lock()
                            .map_err(|_| Error::State)?
                            .as_ref()
                            .map(|n| n.documents)
                            .ok_or(Error::State)?;
                        let _charge = budget.reserve(
                            "python-numerical-query-buffers",
                            documents
                                .checked_mul(1024)
                                .and_then(|n| n.checked_add(8192))
                                .ok_or(Error::ResourceRefused("numerical query bytes"))?,
                        )?;
                        let tokens = scoring
                            .service
                            .retrieval()
                            .numerical_query(&retained, &ranking, &query)?;
                        let encoded = serde_json::to_string(&tokens).map_err(encoding)?;
                        let scores = Python::attach(|py| -> PyResult<Vec<DocumentScore>> {
                            let result = callback.bind(py).call1((encoded,)).map_err(|_| {
                                refused("unavailable", "numerical scoring callback refused")
                            })?;
                            let string = result.cast::<pyo3::types::PyString>().map_err(|_| {
                                refused("corrupt", "numerical score callback contract")
                            })?;
                            let raw = string.to_str().map_err(|_| {
                                refused("corrupt", "numerical score callback encoding")
                            })?;
                            let _decoded = budget
                                .reserve(
                                    "python-score-decode",
                                    raw.len()
                                        .checked_mul(6)
                                        .and_then(|n| n.checked_add(8192))
                                        .ok_or_else(|| {
                                            refused("resource_refused", "score decode bytes")
                                        })?,
                                )
                                .map_err(|_| refused("resource_refused", "score decode bytes"))?;
                            let scores: Vec<DocumentScore> =
                                serde_json::from_str(raw).map_err(|_| {
                                    refused("corrupt", "numerical score callback contract")
                                })?;
                            retained
                                .retain("python-score-handoff", scores.len() * 256 + 8192)
                                .map_err(error)?;
                            Ok(scores)
                        });
                        match scores {
                            Ok(scores) => Ok(Ok((ranking, scores))),
                            Err(error) => Ok(Err(error)),
                        }
                    })
                    .await
                    .map_err(error)??;
                let (ranking, ranked) = state
                    .service
                    .retrieval()
                    .rank(&execution, ranking, scores, query_vector)
                    .await
                    .map_err(error)?;
                state
                    .service
                    .retrieval()
                    .finish(&execution, request, ranking, ranked)
                    .await
                    .map_err(error)?
            } else {
                if query_vector.is_some() || degradation.is_some() || numerical_callback.is_some() {
                    return Err(invalid("numerical arguments require a search request"));
                }
                state
                    .service
                    .dispatch(&execution, request)
                    .await
                    .map_err(error)?
            };
            let retained = execution.clone();
            execution
                .cpu(move |budget| {
                    let _charge = budget.reserve(
                        "python-response-encode",
                        ResourceLimits::default().expanded_response_bytes as usize * 6 + 8192,
                    )?;
                    let encoded = response.to_json().map_err(|_| Error::Contract)?;
                    retained.retain("python-response-handoff", encoded.len() * 6 + 8192)?;
                    Ok(encoded)
                })
                .await
                .map_err(error)
        })
    }
    fn admit_envelope<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        encoded: Py<PyBytes>,
        expanded: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            execution
                .cpu(move |budget| {
                    Python::attach(|py| {
                        let bytes = encoded.bind(py).as_bytes();
                        // Byte refusal precedes decoding or allocating the actual transport envelope.
                        let bound = ResourceLimits::default().response_bytes(expanded) as usize;
                        if bytes.len() > bound {
                            return Ok(Err(invalid("resource_refused: final MCP envelope bytes")));
                        }
                        let _charge =
                            budget.reserve("python-final-envelope", bytes.len() * 6 + 8192)?;
                        let result = std::str::from_utf8(bytes)
                            .map_err(invalid)
                            .and_then(|raw| wire::admit_envelope(raw, expanded).map_err(invalid));
                        if result.is_ok() {
                            retained
                                .retain("python-final-envelope-handoff", bytes.len() * 3 + 512)?;
                        }
                        Ok(result)
                    })
                })
                .await
                .map_err(error)?
        })
    }
    fn tool_result<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        tool: Py<PyString>,
        response: Py<PyString>,
        expanded: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let retained = execution.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            execution
                .cpu(move |budget| {
                    Python::attach(|py| {
                        let name = tool.bind(py);
                        let response = response.bind(py);
                        if !name.is_exact_instance_of::<PyString>()
                            || !response.is_exact_instance_of::<PyString>()
                        {
                            return Ok(Err(invalid("exact builtin strings required")));
                        }
                        let maximum_name = wire::Tool::ALL
                            .iter()
                            .map(|tool| tool.name().len())
                            .max()
                            .unwrap_or(0);
                        if name.len().map_err(|_| Error::Contract)? > maximum_name {
                            return Ok(Err(invalid("unknown serving tool")));
                        }
                        let _name = budget.reserve("python-result-tool-name", 8192)?;
                        let name = match name.to_str() {
                            Ok(value) => value,
                            Err(_) => return Ok(Err(invalid("invalid serving tool encoding"))),
                        };
                        if wire::Tool::from_name(name).is_err() {
                            return Ok(Err(invalid("unknown serving tool")));
                        }
                        let bound = ResourceLimits::default().response_bytes(expanded) as usize;
                        let characters = response.len().map_err(|_| Error::Contract)?;
                        if characters > bound {
                            return Ok(Err(refused(
                                "resource_refused",
                                "structured response bytes",
                            )));
                        }
                        let _unicode = budget.reserve(
                            "python-result-utf8",
                            characters
                                .checked_mul(4)
                                .and_then(|n| n.checked_add(8192))
                                .ok_or(Error::ResourceRefused("result utf8 bytes"))?,
                        )?;
                        let response = match response.to_str() {
                            Ok(value) => value,
                            Err(_) => {
                                return Ok(Err(refused("corrupt", "structured response encoding")));
                            }
                        };
                        if response.len() > bound {
                            return Ok(Err(refused(
                                "resource_refused",
                                "structured response bytes",
                            )));
                        }
                        let _charge = budget.reserve(
                            "python-tool-result-codec",
                            response
                                .len()
                                .checked_mul(8)
                                .and_then(|n| n.checked_add(8192))
                                .ok_or(Error::ResourceRefused("tool result bytes"))?,
                        )?;
                        let result =
                            wire::tool_result(name, response, expanded).map_err(
                                |error| match error {
                                    wire::WireError::ResourceRefused(_) => {
                                        refused("resource_refused", "final MCP result bytes")
                                    }
                                    _ => refused("corrupt", "structured response contract"),
                                },
                            );
                        if let Ok(encoded) = &result {
                            retained
                                .retain("python-tool-result-handoff", encoded.len() * 6 + 8192)?;
                        }
                        Ok(result)
                    })
                })
                .await
                .map_err(error)?
        })
    }
    fn capability_resource<'py>(
        &self,
        py: Python<'py>,
        grant: &RequestGrant,
        capability: Py<PyString>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let execution = grant.execution(&self.state)?;
        let state = self.state.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let request = execution
                .cpu(move |budget| {
                    Python::attach(|py| {
                        let capability = capability.bind(py);
                        if !capability.is_exact_instance_of::<PyString>() {
                            return Ok(Err(invalid("exact builtin strings required")));
                        }
                        if capability.len().map_err(|_| Error::Contract)? != 32 {
                            return Ok(Err(invalid("invalid canonical capability id")));
                        }
                        let _unicode =
                            budget.reserve("python-capability-id-utf8", 32 * 4 + 8192)?;
                        let capability = match capability.to_str() {
                            Ok(value) => value,
                            Err(_) => return Ok(Err(invalid("invalid canonical capability id"))),
                        };
                        if capability.len() != 32
                            || !capability
                                .bytes()
                                .all(|b| matches!(b,b'0'..=b'9'|b'a'..=b'f'))
                        {
                            return Ok(Err(invalid("invalid canonical capability id")));
                        }
                        let mut bytes = [0u8; 16];
                        for (i, byte) in bytes.iter_mut().enumerate() {
                            *byte = u8::from_str_radix(&capability[i * 2..i * 2 + 2], 16)
                                .map_err(|_| Error::Contract)?;
                        }
                        let capability =
                            serde_json::from_value(serde_json::json!(bytes)).map_err(encoding)?;
                        Ok(Ok(Request::GetCapability(wire::GetCapabilityRequest {
                            capability,
                            page: wire::PageRequest {
                                expanded: true,
                                ..Default::default()
                            },
                        })))
                    })
                })
                .await
                .map_err(error)??;
            let response = state
                .service
                .dispatch(&execution, request)
                .await
                .map_err(error)?;
            let retained = execution.clone();
            execution
                .cpu(move |budget| {
                    let wire::Response::GetCapability(response) = response else {
                        return Err(Error::Contract);
                    };
                    let bytes = serialized_len(&response)?
                        .checked_mul(32)
                        .and_then(|n| n.checked_add(8192))
                        .ok_or(Error::ResourceRefused("capability resource bytes"))?;
                    let _charge = budget.reserve("python-capability-resource-codec", bytes)?;
                    let text = response
                        .resource_text()
                        .map_err(|_| Error::Contract)?;
                    if text.len() > ResourceLimits::default().expanded_response_bytes as usize {
                        return Err(Error::ResourceRefused("capability resource bytes"));
                    }
                    retained.retain("python-capability-resource", text.len() * 6 + 8192)?;
                    Ok(text)
                })
                .await
                .map_err(error)
        })
    }
    fn shutdown<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let state = self.state.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            state.service.shutdown().await.map_err(error)?;
            state
                .numerical
                .lock()
                .map_err(|_| PyRuntimeError::new_err("numerical state"))?
                .take();
            Ok(())
        })
    }
}
#[pyfunction]
#[pyo3(signature=(config_path,generation=None,vectors=false))]
fn open_service(
    py: Python<'_>,
    config_path: PathBuf,
    generation: Option<String>,
    vectors: bool,
) -> PyResult<Bound<'_, PyAny>> {
    let generation = generation
        .map(|s| GenerationId::from_hex(&s).ok_or_else(|| invalid("invalid generation id")))
        .transpose()?;
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let config = tokio::task::spawn_blocking(move || RoleConfig::load(&config_path))
            .await
            .map_err(|_| PyRuntimeError::new_err("configuration task failed"))?
            .map_err(|_| refused("incompatible", "serving configuration refused"))?;
        let service = ServingService::open(&config, generation, vectors)
            .await
            .map_err(error)?;
        Ok(Service {
            state: Arc::new(State {
                service,
                numerical: Mutex::new(None),
            }),
        })
    })
}
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("StorageError", m.py().get_type::<StorageError>())?;
    m.add_class::<Service>()?;
    m.add_class::<RequestGrant>()?;
    m.add_function(wrap_pyfunction!(open_service, m)?)?;
    Ok(())
}
