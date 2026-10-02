//! Pure canonical serving schemas and wire validation; semantic requests run in their model owner.
use lctx_model::domain::{embedding::Spec, serving};
use pyo3::{exceptions::PyValueError, prelude::*};
fn error(e: serving::WireError) -> PyErr {
    PyValueError::new_err(e.to_string())
}
#[pyfunction]
fn canonical_embedding_spec(text: &str) -> PyResult<String> {
    Spec::parse(text)
        .map(|s| s.canonical_json())
        .map_err(PyValueError::new_err)
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
        .map_err(error)
}
#[pyfunction]
#[pyo3(signature=(encoded,expanded=false))]
fn admit_envelope(py: Python<'_>, encoded: &str, expanded: bool) -> PyResult<()> {
    py.detach(|| serving::admit_envelope(encoded, expanded))
        .map_err(error)
}
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(canonical_embedding_spec, m)?)?;
    m.add_function(wrap_pyfunction!(wire_schema, m)?)?;
    m.add_function(wrap_pyfunction!(wire_decode, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tool, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tools, m)?)?;
    m.add_function(wrap_pyfunction!(wire_resources, m)?)?;
    m.add_function(wrap_pyfunction!(wire_tool_result, m)?)?;
    m.add_function(wrap_pyfunction!(admit_envelope, m)?)?;
    Ok(())
}
