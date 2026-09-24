//! Developer smoke bridge to the Rust condition kernel; generation queries are not exposed yet.
use cpg_schema::condition::Condition;
use cpg_schema::condition_kernel::Diagram;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Bumped only with a migration of the persisted diagram-node format.
const KERNEL_FORMAT: u32 = 1;
const MAX_SMOKE_INPUT_BYTES: usize = 64 * 1024;

#[pyfunction]
fn kernel_format() -> u32 {
    KERNEL_FORMAT
}

fn diagram(text: &str) -> PyResult<Option<Diagram>> {
    // Condition::parse normalizes the full DNF, so reject large raw inputs first.
    if text.len() > MAX_SMOKE_INPUT_BYTES {
        return Ok(None);
    }
    let parsed = Condition::parse(text).map_err(PyValueError::new_err)?;
    Ok(Diagram::from_condition(&parsed).ok())
}

/// Developer smoke probe only: no generation, proof links or typed verdict.
#[pyfunction]
fn probe_compatible(left: &str, right: &str) -> PyResult<Option<bool>> {
    let (Some(left), Some(right)) = (diagram(left)?, diagram(right)?) else {
        return Ok(None);
    };
    Ok(left.compatible(&right).ok())
}

#[pyfunction]
fn probe_implies(left: &str, right: &str) -> PyResult<Option<bool>> {
    let (Some(left), Some(right)) = (diagram(left)?, diagram(right)?) else {
        return Ok(None);
    };
    Ok(left.implies(&right).ok())
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(kernel_format, m)?)?;
    m.add_function(wrap_pyfunction!(probe_compatible, m)?)?;
    m.add_function(wrap_pyfunction!(probe_implies, m)?)?;
    Ok(())
}
