//! Pure canonical serving schemas and wire validation; semantic requests run in their model owner.
mod session;
use lctx_model::domain::{embedding::Spec, serving};
use pyo3::{exceptions::PyValueError, prelude::*};
pyo3::create_exception!(lctx_semantics, NativeFailure, pyo3::exceptions::PyRuntimeError);
pub(crate) fn public_error(failure: serving::PublicFailure) -> PyErr {
    let error = NativeFailure::new_err(failure.kind.message());
    Python::attach(|py| {
        // Payload is entirely model-owned and has a fixed message.
        let payload = serde_json::to_string(&failure).expect("fixed public failure encoding");
        if let Err(attribute) = error.value(py).setattr("lctx_failure_json", payload) { return attribute; }
        error
    })
}
fn error(e: serving::WireError) -> PyErr { public_error(e.public_failure()) }
fn response_error(e: serving::WireError) -> PyErr {
    let kind = if matches!(e, serving::WireError::ResourceRefused(_)) { serving::FailureKind::ResourceRefused } else { serving::FailureKind::Corrupt };
    public_error(serving::PublicFailure::new(kind))
}
pub(crate) fn model_error(e: lctx_model::domain::ModelError) -> PyErr {
    use lctx_model::domain::{ModelError, Infrastructure};
    let kind = match e {
        ModelError::Serving(kind) => kind,
        ModelError::Resource { .. } | ModelError::Limit { .. } => serving::FailureKind::ResourceRefused,
        ModelError::Infrastructure { class: Infrastructure::Contract, .. } => serving::FailureKind::Incompatible,
        ModelError::Schema(_) | ModelError::Identity(_) | ModelError::Conflict(_) | ModelError::Invalid(_) | ModelError::Frontier(_) => serving::FailureKind::Corrupt,
        _ => serving::FailureKind::Unavailable,
    };
    public_error(serving::PublicFailure::new(kind))
}
pub(crate) fn unavailable() -> PyErr { public_error(serving::PublicFailure::new(serving::FailureKind::Unavailable)) }
#[pyfunction]
fn canonical_embedding_spec(text: &str) -> PyResult<String> {
    Spec::parse(text)
        .map(|s| s.canonical_json())
        .map_err(|_| PyValueError::new_err("invalid canonical embedding specification"))
}
#[pyfunction]
fn wire_schema(name: &str, output: bool) -> PyResult<String> {
    serde_json::to_string(&serving::schema(name, output).map_err(error)?)
        .map_err(|_| PyValueError::new_err("schema encoding"))
}
#[pyfunction]
fn wire_decode(py: Python<'_>, name: &str, raw: &str) -> PyResult<String> {
    py.detach(|| serving::decode(name, raw)).map_err(error)
}
#[pyfunction]
fn wire_tool(name: &str) -> PyResult<String> {
    let tool = serving::Tool::from_name(name).map_err(error)?;
    Ok(serde_json::json!({"wire_identity":serving::wire_identity(),"parameters":tool.request_schema(),"output_schema":tool.response_schema(),"byte_limits":{"default":serving::ResourceLimits::default().response_bytes(false),"expanded":serving::ResourceLimits::default().response_bytes(true)}}).to_string())
}
#[pyfunction]
fn wire_tools() -> PyResult<String> {
    serde_json::to_string(&serving::tools())
        .map_err(|_| PyValueError::new_err("tool inventory encoding"))
}
#[pyfunction]
fn wire_resources() -> PyResult<String> {
    serde_json::to_string(&serving::resources())
        .map_err(|_| PyValueError::new_err("resource inventory encoding"))
}
#[pyfunction]
#[pyo3(signature=(name,raw,expanded=false))]
fn wire_tool_result(py: Python<'_>, name: &str, raw: &str, expanded: bool) -> PyResult<String> {
    py.detach(|| serving::tool_result(name, raw, expanded))
        .map_err(response_error)
}
#[pyfunction]
#[pyo3(signature=(encoded,expanded=false))]
fn admit_envelope(py: Python<'_>, encoded: &str, expanded: bool) -> PyResult<()> {
    py.detach(|| serving::admit_envelope(encoded, expanded))
        .map_err(error)
}
#[pyfunction]
fn wire_failure(kind: &str) -> PyResult<String> {
    let kind = serving::FailureKind::from_name(kind)
        .ok_or_else(|| PyValueError::new_err("unrecognized public failure kind"))?;
    serde_json::to_string(&serving::PublicFailure::new(kind))
        .map_err(|_| PyValueError::new_err("failure encoding"))
}
#[pyfunction]
fn wire_capability_resource(py: Python<'_>, raw: &str) -> PyResult<String> {
    py.detach(|| {
        serving::decode_response(
            "get_capability",
            raw,
            true,
            &serving::ResourceLimits::default(),
        )
        .map_err(response_error)?;
        let response: serving::GetCapabilityResponse =
            serde_json::from_str(raw).map_err(|_| public_error(serving::PublicFailure::new(serving::FailureKind::Corrupt)))?;
        let text = response.resource_text().map_err(error)?;
        if text.len() as u64 > serving::ResourceLimits::default().response_bytes(true) {
            return Err(public_error(serving::PublicFailure::new(serving::FailureKind::ResourceRefused)));
        }
        Ok(text)
    })
}
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("NativeFailure", m.py().get_type::<NativeFailure>())?;
    m.add_class::<session::NativeSession>()?;
    m.add_function(wrap_pyfunction!(wire_capability_resource, m)?)?;
    m.add_function(wrap_pyfunction!(canonical_embedding_spec, m)?)?;
    m.add_function(wrap_pyfunction!(wire_schema, m)?)?;
    m.add_function(wrap_pyfunction!(wire_decode, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tool, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tools, m)?)?;
    m.add_function(wrap_pyfunction!(wire_failure, m)?)?;
    m.add_function(wrap_pyfunction!(wire_resources, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tool_result, m)?)?;
    m.add_function(wrap_pyfunction!(admit_envelope, m)?)?;
    Ok(())
}
