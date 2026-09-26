//! The production generation projection. Schemas come from cpg-schema, and fields are decoded
//! by name after whole-file IPC/schema validation; Python does not construct positional tuples.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;

use arrow_array::{Array, FixedSizeBinaryArray, Int64Array, RecordBatch, StringArray};
use arrow_ipc::reader::FileReader;
use cpg_schema::bundle;
use cpg_schema::id::{Digest, Id};
use pyo3::exceptions::PyValueError;
use pyo3::PyResult;

use super::{FlowInput, LeafInput, LinkInput, MAX_SUMMARY_ROWS, MAX_SURFACE_ROWS};

const NAMES: &[&str] = &[
    "conditions",
    "condition_nodes",
    "analysis_conditions",
    "analysis_condition_nodes",
    "operations",
    "public_paths",
    "operation_parameters",
    "summary_flows",
    "summary_flow_steps",
    "summary_boundaries",
    "flow_test_leaves",
    "flow_test_value_links",
];
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct Inputs {
    pub conditions: Vec<(String, Option<String>, Option<String>)>,
    pub nodes: Vec<(String, String, String, String)>,
    pub operations: Vec<String>,
    pub public_paths: Vec<(String, String)>,
    pub parameters: Vec<(String, String, String)>,
    pub flows: Vec<FlowInput>,
    pub steps: Vec<(String, i64, String, String, String)>,
    pub boundaries: Vec<(String, String, String, String, String, String)>,
    pub leaves: Vec<LeafInput>,
    pub links: Vec<LinkInput>,
}

fn invalid(message: impl Into<String>) -> pyo3::PyErr {
    PyValueError::new_err(message.into())
}

struct NamedBatch {
    name: String,
    batch: RecordBatch,
}

impl NamedBatch {
    fn array<T: Array + 'static>(&self, field: &str) -> PyResult<&T> {
        self.batch
            .column_by_name(field)
            .and_then(|column| column.as_any().downcast_ref::<T>())
            .ok_or_else(|| invalid(format!("{}: invalid {field} column", self.name)))
    }

    fn id(&self, field: &str, row: usize) -> PyResult<String> {
        let array = self.array::<FixedSizeBinaryArray>(field)?;
        if array.is_null(row) {
            return Err(invalid(format!("{}: null {field}", self.name)));
        }
        let bytes: [u8; 16] = array.value(row).try_into().map_err(|_| invalid("invalid id"))?;
        Ok(Id(bytes).hex())
    }

    fn optional_id(&self, field: &str, row: usize) -> PyResult<Option<String>> {
        let array = self.array::<FixedSizeBinaryArray>(field)?;
        if array.is_null(row) {
            Ok(None)
        } else {
            self.id(field, row).map(Some)
        }
    }

    fn digest(&self, field: &str, row: usize) -> PyResult<String> {
        let array = self.array::<FixedSizeBinaryArray>(field)?;
        if array.is_null(row) {
            return Err(invalid(format!("{}: null {field}", self.name)));
        }
        let bytes: [u8; 32] = array.value(row).try_into().map_err(|_| invalid("invalid digest"))?;
        Ok(Digest(bytes).hex())
    }

    fn text(&self, field: &str, row: usize) -> PyResult<String> {
        let array = self.array::<StringArray>(field)?;
        if array.is_null(row) {
            return Err(invalid(format!("{}: null {field}", self.name)));
        }
        Ok(array.value(row).to_owned())
    }

    fn optional_text(&self, field: &str, row: usize) -> PyResult<Option<String>> {
        let array = self.array::<StringArray>(field)?;
        Ok((!array.is_null(row)).then(|| array.value(row).to_owned()))
    }

    fn integer(&self, field: &str, row: usize) -> PyResult<i64> {
        let array = self.array::<Int64Array>(field)?;
        if array.is_null(row) {
            return Err(invalid(format!("{}: null {field}", self.name)));
        }
        Ok(array.value(row))
    }

    fn len(&self) -> usize {
        self.batch.num_rows()
    }
}

fn batch<'a>(batches: &'a HashMap<String, NamedBatch>, name: &str) -> &'a NamedBatch {
    &batches[name]
}

pub(super) fn decode(files: Vec<(String, Vec<u8>)>) -> PyResult<Inputs> {
    if files.len() != NAMES.len() {
        return Err(invalid("native generation projection has missing or extra files"));
    }
    let wanted: HashSet<&str> = NAMES.iter().copied().collect();
    let schemas: HashMap<_, _> = bundle::files(0)
        .into_iter()
        .filter(|file| wanted.contains(file.name))
        .map(|file| (file.name, file.schema))
        .collect();
    let mut batches = HashMap::new();
    for (name, bytes) in files {
        if !wanted.contains(name.as_str()) || bytes.len() > MAX_FILE_BYTES {
            return Err(invalid(format!("{name}: unexpected or oversized native IPC file")));
        }
        let expected = &schemas[name.as_str()];
        let mut reader = FileReader::try_new(Cursor::new(bytes), None)
            .map_err(|error| invalid(format!("{name}: {error}")))?;
        if reader.schema().as_ref() != expected.as_ref() {
            return Err(invalid(format!("{name}: native serving schema drift")));
        }
        let first = reader
            .next()
            .transpose()
            .map_err(|error| invalid(format!("{name}: {error}")))?
            .unwrap_or_else(|| RecordBatch::new_empty(expected.clone()));
        if reader.next().is_some() {
            return Err(invalid(format!("{name}: expected one IPC record batch")));
        }
        if batches
            .insert(name.clone(), NamedBatch { name, batch: first })
            .is_some()
        {
            return Err(invalid("duplicate native IPC file"));
        }
    }
    if batches.len() != NAMES.len() {
        return Err(invalid("native generation projection has missing files"));
    }
    for name in NAMES {
        let limit = if matches!(*name, "operations" | "public_paths" | "operation_parameters") {
            MAX_SURFACE_ROWS
        } else {
            MAX_SUMMARY_ROWS
        };
        if batch(&batches, name).len() > limit {
            return Err(invalid(format!("{name}: native row limit exceeded")));
        }
    }

    let mut conditions = BTreeMap::new();
    for name in ["conditions", "analysis_conditions"] {
        let table = batch(&batches, name);
        for row in 0..table.len() {
            let id = table.id("condition_id", row)?;
            let value = (id.clone(), table.optional_id("root_id", row)?, table.optional_text("boundary_reason", row)?);
            if conditions.insert(id.clone(), value.clone()).is_some_and(|prior| prior != value) {
                return Err(invalid(format!("conflicting condition {id} across catalogs")));
            }
        }
    }
    let mut nodes = BTreeMap::new();
    for name in ["condition_nodes", "analysis_condition_nodes"] {
        let table = batch(&batches, name);
        for row in 0..table.len() {
            let id = table.id("node_id", row)?;
            let value = (id.clone(), table.text("atom", row)?, table.id("low_id", row)?, table.id("high_id", row)?);
            if nodes.insert(id.clone(), value.clone()).is_some_and(|prior| prior != value) {
                return Err(invalid(format!("conflicting condition node {id} across catalogs")));
            }
        }
    }
    let table = batch(&batches, "operations");
    let operations = (0..table.len()).map(|row| table.id("node_id", row)).collect::<PyResult<_>>()?;
    let table = batch(&batches, "public_paths");
    let public_paths = (0..table.len()).map(|row| Ok((table.text("access_path", row)?, table.id("node_id", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "operation_parameters");
    let parameters = (0..table.len()).map(|row| Ok((table.id("operation_node_id", row)?, table.id("formal_node_id", row)?, table.text("name", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_flows");
    let flows = (0..table.len()).map(|row| Ok((table.id("summary_id", row)?, table.id("function_node_id", row)?, table.id("parameter_node_id", row)?, table.id("condition_id", row)?, table.text("verdict", row)?, table.optional_text("boundary_reason", row)?, table.integer("path_depth", row)?, table.id("source_flow_fact_id", row)?, table.id("source_origin_id", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_flow_steps");
    let steps = (0..table.len()).map(|row| Ok((table.id("summary_id", row)?, table.integer("ordinal", row)?, table.text("kind", row)?, table.id("evidence_id", row)?, table.id("condition_id", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_boundaries");
    let boundaries = (0..table.len()).map(|row| Ok((table.id("function_node_id", row)?, table.id("parameter_node_id", row)?, table.id("source_flow_fact_id", row)?, table.id("source_origin_id", row)?, table.id("condition_id", row)?, table.text("reason", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "flow_test_leaves");
    let leaves = (0..table.len()).map(|row| Ok((table.id("fact_id", row)?, table.id("module_node_id", row)?, table.id("condition_id", row)?, table.id("atom_id", row)?, table.text("atom", row)?, table.optional_text("path", row)?, table.integer("leaf_start_byte", row)?, table.integer("leaf_end_byte", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "flow_test_value_links");
    let links = (0..table.len()).map(|row| Ok((table.id("link_id", row)?, table.id("operation_node_id", row)?, table.id("formal_node_id", row)?, table.id("module_node_id", row)?, table.id("leaf_fact_id", row)?, table.id("atom_id", row)?, table.id("condition_id", row)?, table.text("place", row)?, table.text("origin", row)?, table.digest("effect_model_digest", row)?, (table.optional_text("path", row)?, table.integer("operand_start_byte", row)?, table.integer("operand_end_byte", row)?)))).collect::<PyResult<_>>()?;
    Ok(Inputs {
        conditions: conditions.into_values().collect(),
        nodes: nodes.into_values().collect(),
        operations,
        public_paths,
        parameters,
        flows,
        steps,
        boundaries,
        leaves,
        links,
    })
}
