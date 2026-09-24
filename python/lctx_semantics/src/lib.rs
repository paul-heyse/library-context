//! Generation-pinned condition kernel and developer smoke probes.
use std::collections::HashMap;

use cpg_schema::condition::Condition;
use cpg_schema::condition_kernel::{
    ConditionRoot, Diagram, DiagramNode, KERNEL_FORMAT, KernelBoundary, MAX_CATALOG_CONDITIONS,
    MAX_CATALOG_NODES, hydrate_catalog,
};
use cpg_schema::id::Id;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

const MAX_SMOKE_INPUT_BYTES: usize = 64 * 1024;

#[pyfunction]
fn kernel_format() -> u32 {
    KERNEL_FORMAT
}

#[pyfunction]
fn catalog_limits() -> (usize, usize) {
    (MAX_CATALOG_CONDITIONS, MAX_CATALOG_NODES)
}

fn id(value: &str) -> PyResult<Id> {
    Id::from_hex(value)
        .ok_or_else(|| PyValueError::new_err(format!("invalid condition id {value}")))
}

/// Hydrated once from rows of one validated, immutable generation.
#[pyclass]
struct ConditionGraph {
    diagrams: HashMap<Id, Diagram>,
    boundaries: HashMap<Id, KernelBoundary>,
    node_count: usize,
}

#[pymethods]
impl ConditionGraph {
    #[new]
    fn new(
        kernel_format: u32,
        conditions: Vec<(String, Option<String>, Option<String>)>,
        nodes: Vec<(String, String, String, String)>,
    ) -> PyResult<Self> {
        if kernel_format != KERNEL_FORMAT {
            return Err(PyValueError::new_err(format!(
                "condition kernel format {kernel_format}, not {KERNEL_FORMAT}"
            )));
        }
        let conditions = conditions
            .into_iter()
            .map(|(condition, root, boundary)| {
                Ok(ConditionRoot {
                    condition_id: id(&condition)?,
                    root_id: root.as_deref().map(id).transpose()?,
                    boundary_reason: boundary,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let nodes = nodes
            .into_iter()
            .map(|(node, atom, low, high)| {
                Ok(DiagramNode {
                    node_id: id(&node)?,
                    atom,
                    low: id(&low)?,
                    high: id(&high)?,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let diagrams = hydrate_catalog(&conditions, &nodes).map_err(PyValueError::new_err)?;
        let boundaries = conditions
            .iter()
            .filter_map(|row| {
                Some((
                    row.condition_id,
                    KernelBoundary::from_code(row.boundary_reason.as_deref()?)?,
                ))
            })
            .collect();
        Ok(Self {
            diagrams,
            boundaries,
            node_count: nodes.len(),
        })
    }

    #[getter]
    fn condition_count(&self) -> usize {
        self.diagrams.len() + self.boundaries.len()
    }

    #[getter]
    fn node_count(&self) -> usize {
        self.node_count
    }

    /// `None` carries a named boundary; a missing id is an invalid query.
    fn compatible(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.question(left, right, Diagram::compatible)
    }

    fn implies(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.question(left, right, Diagram::implies)
    }
}

impl ConditionGraph {
    fn question(
        &self,
        left: &str,
        right: &str,
        operation: fn(&Diagram, &Diagram) -> Result<bool, KernelBoundary>,
    ) -> PyResult<(Option<bool>, Option<String>)> {
        let left = id(left)?;
        let right = id(right)?;
        for condition in [left, right] {
            if let Some(reason) = self.boundaries.get(&condition) {
                return Ok((None, Some(reason.code().to_owned())));
            }
        }
        let a = self.diagrams.get(&left).ok_or_else(|| {
            PyValueError::new_err(format!(
                "condition {} is absent from generation",
                left.hex()
            ))
        })?;
        let b = self.diagrams.get(&right).ok_or_else(|| {
            PyValueError::new_err(format!(
                "condition {} is absent from generation",
                right.hex()
            ))
        })?;
        match operation(a, b) {
            Ok(answer) => Ok((Some(answer), None)),
            Err(reason) => Ok((None, Some(reason.code().to_owned()))),
        }
    }
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
    m.add_class::<ConditionGraph>()?;
    m.add_function(wrap_pyfunction!(kernel_format, m)?)?;
    m.add_function(wrap_pyfunction!(catalog_limits, m)?)?;
    m.add_function(wrap_pyfunction!(probe_compatible, m)?)?;
    m.add_function(wrap_pyfunction!(probe_implies, m)?)?;
    Ok(())
}
