//! The production generation projection. Schemas come from cpg-schema, and fields are decoded
//! by name after whole-file IPC/schema validation; Python does not construct positional tuples.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;

use arrow_array::{Array, FixedSizeBinaryArray, Int64Array, RecordBatch, StringArray};
use arrow_ipc::reader::FileReader;
use cpg_schema::bundle;
use cpg_schema::id::{Digest, Id};
use cpg_schema::parameter_identity::SourceParameterIdentitiesRow;
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
    "callable_parameters",
    "summary_flows",
    "source_parameter_identities", "source_context_value_identities", "source_modeled_identities", "model_frame_exits", "model_frame_exit_arguments", "model_frame_exit_steps", "source_body_completions", "source_body_steps", "source_body_release_inputs", "source_call_normals", "source_call_header_steps",
    "model_context_protocols", "source_context_sites", "source_context_arguments", "return_completion_certificates",
    "summary_flow_steps",
    "summary_boundaries",
    "flow_test_leaves",
    "flow_test_value_links",
];
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct Inputs {
    pub source_body_completions:Vec<cpg_schema::source_body::SourceBodyCompletionsRow>,
    pub source_body_steps:Vec<cpg_schema::source_body::SourceBodyStepsRow>,
    pub source_body_release_inputs:Vec<cpg_schema::source_body::SourceBodyReleaseInputsRow>,
    pub source_call_normals:Vec<cpg_schema::source_call::SourceCallNormalsRow>,
    pub source_call_header_steps:Vec<cpg_schema::source_call::SourceCallHeaderStepsRow>,

    pub model_frame_exits:Vec<cpg_schema::frame_exit::ModelFrameExitsRow>,
    pub model_frame_exit_arguments:Vec<cpg_schema::frame_exit::ModelFrameExitArgumentsRow>,
    pub model_frame_exit_steps:Vec<cpg_schema::frame_exit::ModelFrameExitStepsRow>,

    pub return_certificates: Vec<cpg_schema::completion_proof::ReturnCompletionCertificatesRow>,
    pub model_context_protocols: Vec<cpg_schema::context_protocol::ModelContextProtocolsRow>,
    pub source_context_sites: Vec<cpg_schema::context_protocol::SourceContextSitesRow>,
    pub source_context_arguments: Vec<cpg_schema::context_protocol::SourceContextArgumentsRow>,

    pub context_value_identities: Vec<cpg_schema::context_value::SourceContextValueIdentitiesRow>,
    pub identities: Vec<SourceParameterIdentitiesRow>,
    pub modeled_identities:Vec<cpg_schema::modeled_identity::SourceModeledIdentitiesRow>,
    pub return_sites: HashMap<Id, Id>,
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

    fn boolean(&self, field:&str, row:usize)->PyResult<bool> {
        let array=self.array::<arrow_array::BooleanArray>(field)?;
        if array.is_null(row) {return Err(invalid(format!("{}: null {field}",self.name)));}
        Ok(array.value(row))
    }

    fn code<T:cpg_schema::codebook::Codebook>(&self,field:&str,row:usize)->PyResult<T> {
        let value=self.text(field,row)?;
        T::all().iter().copied().find(|k|k.text()==value).ok_or_else(||invalid(format!("{}: invalid {field}",self.name)))
    }

    fn optional_code<T:cpg_schema::codebook::Codebook>(&self,field:&str,row:usize)->PyResult<Option<T>> {
        self.optional_text(field,row)?.map(|value|T::all().iter().copied().find(|k|k.text()==value)
            .ok_or_else(||invalid(format!("{}: invalid {field}",self.name)))).transpose()
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
        let limit = if matches!(*name, "operations" | "public_paths" | "callable_parameters") {
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
    let table = batch(&batches, "callable_parameters");
    let parameters = (0..table.len()).map(|row| Ok((table.id("function_node_id", row)?, table.id("formal_node_id", row)?, table.text("name", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_flows");
    let return_sites = (0..table.len()).map(|row| Ok((super::id(&table.id("summary_id", row)?)?,
        super::id(&table.id("return_site_fact_id", row)?)?))).collect::<PyResult<_>>()?;
    let flows = (0..table.len()).map(|row| Ok((table.id("summary_id", row)?, table.id("function_node_id", row)?, table.id("parameter_node_id", row)?, table.id("condition_id", row)?, table.text("verdict", row)?, table.optional_text("boundary_reason", row)?, table.integer("path_depth", row)?, table.id("source_flow_fact_id", row)?, table.id("source_origin_id", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_flow_steps");
    let steps = (0..table.len()).map(|row| Ok((table.id("summary_id", row)?, table.integer("ordinal", row)?, table.text("kind", row)?, table.id("evidence_id", row)?, table.id("condition_id", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "summary_boundaries");
    let boundaries = (0..table.len()).map(|row| Ok((table.id("function_node_id", row)?, table.id("parameter_node_id", row)?, table.id("source_flow_fact_id", row)?, table.id("source_origin_id", row)?, table.id("condition_id", row)?, table.text("reason", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "flow_test_leaves");
    let leaves = (0..table.len()).map(|row| Ok((table.id("fact_id", row)?, table.id("module_node_id", row)?, table.id("condition_id", row)?, table.id("atom_id", row)?, table.text("atom", row)?, table.optional_text("path", row)?, table.integer("leaf_start_byte", row)?, table.integer("leaf_end_byte", row)?))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "flow_test_value_links");
    let links = (0..table.len()).map(|row| Ok((table.id("link_id", row)?, table.id("operation_node_id", row)?, table.id("formal_node_id", row)?, table.id("module_node_id", row)?, table.id("leaf_fact_id", row)?, table.id("atom_id", row)?, table.id("condition_id", row)?, table.text("place", row)?, table.text("origin", row)?, table.digest("effect_model_digest", row)?, (table.optional_text("path", row)?, table.integer("operand_start_byte", row)?, table.integer("operand_end_byte", row)?)))).collect::<PyResult<_>>()?;
    let table = batch(&batches, "source_context_value_identities");
    let context_value_identities = (0..table.len()).map(|row| {
        let key = |name| super::id(&table.id(name, row)?);
        Ok(cpg_schema::context_value::SourceContextValueIdentitiesRow { snapshot_id: Id::ZERO,
            identity_id: key("identity_id")?,
            function_node_id: key("function_node_id")?,
            parameter_node_id: key("parameter_node_id")?,
            parameter_name: table.text("parameter_name", row)?,
            source_flow_fact_id: key("source_flow_fact_id")?,
            source_origin_id: key("source_origin_id")?,
            condition_id: key("condition_id")?,
            return_site_fact_id: key("return_site_fact_id")?,
            return_region_fact_id: key("return_region_fact_id")?,
            return_condition_id: key("return_condition_id")?,
            return_start_byte: table.integer("return_start_byte", row)?,
            context_site_id: key("context_site_id")?,
            argument_fact_id: key("argument_fact_id")?,
            argument_expression_fact_id: key("argument_expression_fact_id")?,
            argument_reference_fact_id: key("argument_reference_fact_id")?,
            argument_resolution_fact_id: key("argument_resolution_fact_id")?,
            parameter_binding_fact_id: key("parameter_binding_fact_id")?,
            parameter_fact_id: key("parameter_fact_id")?,
            target_binding_fact_id: key("target_binding_fact_id")?,
            expression_fact_id: key("expression_fact_id")?,
            reference_fact_id: key("reference_fact_id")?,
            resolution_fact_id: key("resolution_fact_id")?,
            scope_fact_id: key("scope_fact_id")?,
            module_node_id: key("module_node_id")?,
            start_byte: table.integer("start_byte", row)?,
            end_byte: table.integer("end_byte", row)?,
        })
    }).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_body_completions");
    let source_body_completions=(0..table.len()).map(|row|Ok(cpg_schema::source_body::SourceBodyCompletionsRow {snapshot_id:Id::ZERO,
        body_id:super::id(&table.id("body_id",row)?)?,
        function_node_id:super::id(&table.id("function_node_id",row)?)?,
        declaration_fact_id:super::id(&table.id("declaration_fact_id",row)?)?,
        syntax_fact_id:super::id(&table.id("syntax_fact_id",row)?)?,
        kind:table.code::<cpg_schema::codebook::CompletionKind>("kind",row)?,
        terminal_fact_id:table.optional_id("terminal_fact_id",row)?.map(|v|super::id(&v)).transpose()?,
        exception:table.optional_code::<cpg_schema::codebook::ExactRuntimeException>("exception",row)?,
        reason:table.optional_code::<cpg_schema::codebook::BoundaryReason>("reason",row)?,
        release_reason:table.optional_code::<cpg_schema::codebook::BoundaryReason>("release_reason",row)?,
        function_retainer_required:table.boolean("function_retainer_required",row)?,
        runtime_statement_count:table.integer("runtime_statement_count",row)?,
        step_count:table.integer("step_count",row)?,
        steps_digest:super::digest(&table.digest("steps_digest",row)?)?,
        release_count:table.integer("release_count",row)?,
        releases_digest:super::digest(&table.digest("releases_digest",row)?)?,
        work:table.integer("work",row)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_body_steps");
    let source_body_steps=(0..table.len()).map(|row|Ok(cpg_schema::source_body::SourceBodyStepsRow {snapshot_id:Id::ZERO,
        body_id:super::id(&table.id("body_id",row)?)?,
        ordinal:table.integer("ordinal",row)?,
        kind:table.code::<cpg_schema::codebook::SummaryFlowStepKind>("kind",row)?,
        evidence_id:super::id(&table.id("evidence_id",row)?)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_body_release_inputs");
    let source_body_release_inputs=(0..table.len()).map(|row|Ok(cpg_schema::source_body::SourceBodyReleaseInputsRow {snapshot_id:Id::ZERO,
        body_id:super::id(&table.id("body_id",row)?)?,
        ordinal:table.integer("ordinal",row)?,
        syntax_fact_id:super::id(&table.id("syntax_fact_id",row)?)?,
        safety:table.code::<cpg_schema::codebook::ReleaseSafety>("safety",row)?,
        proof_offset:table.integer("proof_offset",row)?,
        proof_count:table.integer("proof_count",row)?,
        proof_digest:super::digest(&table.digest("proof_digest",row)?)?,
        evaluation_evidence_id:super::id(&table.id("evaluation_evidence_id",row)?)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_call_normals");
    let source_call_normals=(0..table.len()).map(|row|Ok(cpg_schema::source_call::SourceCallNormalsRow {snapshot_id:Id::ZERO,
        certificate_id:super::id(&table.id("certificate_id",row)?)?,
        function_node_id:super::id(&table.id("function_node_id",row)?)?,
        call_node_id:super::id(&table.id("call_node_id",row)?)?,
        call_fact_id:super::id(&table.id("call_fact_id",row)?)?,
        syntax_fact_id:super::id(&table.id("syntax_fact_id",row)?)?,
        callee_node_id:super::id(&table.id("callee_node_id",row)?)?,
        pysa_fact_id:super::id(&table.id("pysa_fact_id",row)?)?,
        signature_fact_id:super::id(&table.id("signature_fact_id",row)?)?,
        body_id:super::id(&table.id("body_id",row)?)?,
        header_fact_id:super::id(&table.id("header_fact_id",row)?)?,
        statement_fact_id:super::id(&table.id("statement_fact_id",row)?)?,
        binding_fact_id:super::id(&table.id("binding_fact_id",row)?)?,
        reference_fact_id:super::id(&table.id("reference_fact_id",row)?)?,
        resolution_fact_id:super::id(&table.id("resolution_fact_id",row)?)?,
        header_count:table.integer("header_count",row)?,
        header_digest:super::digest(&table.digest("header_digest",row)?)?,
        body_count:table.integer("body_count",row)?,
        body_kind:table.code::<cpg_schema::codebook::CompletionKind>("body_kind",row)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_call_header_steps");
    let source_call_header_steps=(0..table.len()).map(|row|Ok(cpg_schema::source_call::SourceCallHeaderStepsRow {snapshot_id:Id::ZERO,
        certificate_id:super::id(&table.id("certificate_id",row)?)?,
        ordinal:table.integer("ordinal",row)?,
        kind:table.code::<cpg_schema::codebook::SummaryFlowStepKind>("kind",row)?,
        evidence_id:super::id(&table.id("evidence_id",row)?)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"model_frame_exits");
    let model_frame_exits=(0..table.len()).map(|row|Ok(cpg_schema::frame_exit::ModelFrameExitsRow {snapshot_id:Id::ZERO,
        frame_exit_id:super::id(&table.id("frame_exit_id",row)?)?,
        function_node_id:super::id(&table.id("function_node_id",row)?)?,
        call_node_id:super::id(&table.id("call_node_id",row)?)?,
        call_fact_id:super::id(&table.id("call_fact_id",row)?)?,
        syntax_fact_id:super::id(&table.id("syntax_fact_id",row)?)?,
        target_node_id:super::id(&table.id("target_node_id",row)?)?,
        pysa_fact_id:super::id(&table.id("pysa_fact_id",row)?)?,
        model_id:super::id(&table.id("model_id",row)?)?,
        return_parameter:table.text("return_parameter",row)?,
        return_argument_fact_id:super::id(&table.id("return_argument_fact_id",row)?)?,
        argument_count:table.integer("argument_count",row)?,
        signature_count:table.integer("signature_count",row)?,
        arguments_digest:super::digest(&table.digest("arguments_digest",row)?)?,
        invocation_count:table.integer("invocation_count",row)?,
        invocation_digest:super::digest(&table.digest("invocation_digest",row)?)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"model_frame_exit_arguments");
    let model_frame_exit_arguments=(0..table.len()).map(|row|Ok(cpg_schema::frame_exit::ModelFrameExitArgumentsRow {snapshot_id:Id::ZERO,
        frame_exit_id:super::id(&table.id("frame_exit_id",row)?)?,
        ordinal:table.integer("ordinal",row)?,
        argument_fact_id:super::id(&table.id("argument_fact_id",row)?)?,
        expression_fact_id:super::id(&table.id("expression_fact_id",row)?)?,
        safety:table.code::<cpg_schema::codebook::ReleaseSafety>("safety",row)?,
        parameter_name:table.text("parameter_name",row)?,
        expression_offset:table.integer("expression_offset",row)?,
        expression_count:table.integer("expression_count",row)?,
        expression_digest:super::digest(&table.digest("expression_digest",row)?)?,
        parameters_digest:super::digest(&table.digest("parameters_digest",row)?)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"model_frame_exit_steps");
    let model_frame_exit_steps=(0..table.len()).map(|row|Ok(cpg_schema::frame_exit::ModelFrameExitStepsRow {snapshot_id:Id::ZERO,
        frame_exit_id:super::id(&table.id("frame_exit_id",row)?)?,
        ordinal:table.integer("ordinal",row)?,
        operand_fact_id:super::id(&table.id("operand_fact_id",row)?)?,
        evidence_id:super::id(&table.id("evidence_id",row)?)?,
        status:table.code::<cpg_schema::codebook::ModeledArgumentEvaluationStatus>("status",row)?,
        kind:table.code::<cpg_schema::codebook::SummaryFlowStepKind>("kind",row)?,
    })).collect::<PyResult<_>>()?;
    let table=batch(&batches,"source_modeled_identities");
    let modeled_identities=(0..table.len()).map(|row| {
        let key=|name|super::id(&table.id(name,row)?);
        Ok(cpg_schema::modeled_identity::SourceModeledIdentitiesRow {snapshot_id:Id::ZERO,
            identity_id:key("identity_id")?,
            function_node_id:key("function_node_id")?,
            parameter_node_id:key("parameter_node_id")?,
            source_flow_fact_id:key("source_flow_fact_id")?,
            source_origin_id:key("source_origin_id")?,
            condition_id:key("condition_id")?,
            return_site_fact_id:key("return_site_fact_id")?,
            call_fact_id:key("call_fact_id")?,
            call_expression_fact_id:key("call_expression_fact_id")?,
            source_argument_fact_id:key("source_argument_fact_id")?,
            pysa_fact_id:key("pysa_fact_id")?,
            model_id:key("model_id")?,
            rule_id:key("rule_id")?,
            callee_resolution_fact_id:key("callee_resolution_fact_id")?,
            expression_fact_id:key("expression_fact_id")?,
            reference_fact_id:key("reference_fact_id")?,
            resolution_fact_id:key("resolution_fact_id")?,
            binding_fact_id:key("binding_fact_id")?,
            parameter_fact_id:key("parameter_fact_id")?,
            scope_fact_id:key("scope_fact_id")?,
            module_node_id:key("module_node_id")?,
            start_byte:table.integer("start_byte",row)?,
            end_byte:table.integer("end_byte",row)?,
            model_proof_count:table.integer("model_proof_count",row)?,
            model_proof_digest:super::digest(&table.digest("model_proof_digest",row)?)?,
        })
    }).collect::<PyResult<_>>()?;
    let table = batch(&batches, "source_parameter_identities");
    let identities = (0..table.len()).map(|row| {
        let key = |name| super::id(&table.id(name, row)?);
        Ok(SourceParameterIdentitiesRow { snapshot_id: Id::ZERO,
            identity_id: key("identity_id")?, function_node_id: key("function_node_id")?,
            parameter_node_id: key("parameter_node_id")?, source_flow_fact_id: key("source_flow_fact_id")?,
            source_origin_id: key("source_origin_id")?, condition_id: key("condition_id")?,
            return_site_fact_id: key("return_site_fact_id")?, expression_fact_id: key("expression_fact_id")?,
            reference_fact_id: key("reference_fact_id")?, resolution_fact_id: key("resolution_fact_id")?,
            binding_fact_id: key("binding_fact_id")?, parameter_fact_id: key("parameter_fact_id")?,
            module_node_id: key("module_node_id")?, start_byte: table.integer("start_byte", row)?,
            end_byte: table.integer("end_byte", row)?,
        })
    }).collect::<PyResult<_>>()?;
    let table = batch(&batches, "model_context_protocols");
    let model_context_protocols = (0..table.len()).map(|row| {
        Ok(cpg_schema::context_protocol::ModelContextProtocolsRow { snapshot_id: Id::ZERO,
            model_id: super::id(&table.id("model_id", row)?)?,
            revision: table.integer("revision", row)?,
            class_node_id: super::id(&table.id("class_node_id", row)?)?,
            class_fact_id: super::id(&table.id("class_fact_id", row)?)?,
            class_module_fact_id: super::id(&table.id("class_module_fact_id", row)?)?,
            allocation_node_id: super::id(&table.id("allocation_node_id", row)?)?,
            allocation_fact_id: super::id(&table.id("allocation_fact_id", row)?)?,
            allocation_module_fact_id: super::id(&table.id("allocation_module_fact_id", row)?)?,
            initialization_node_id: super::id(&table.id("initialization_node_id", row)?)?,
            initialization_fact_id: super::id(&table.id("initialization_fact_id", row)?)?,
            initialization_module_fact_id: super::id(&table.id("initialization_module_fact_id", row)?)?,
            entry: table.code::<cpg_schema::codebook::ContextEntryKind>("entry", row)?,
            entry_formal: table.optional_text("entry_formal", row)?,
            exit: table.code::<cpg_schema::codebook::ContextExitKind>("exit", row)?,
            exception_formal: table.optional_text("exception_formal", row)?,
            origin: table.code::<cpg_schema::codebook::Origin>("origin", row)?,
        })
    }).collect::<PyResult<_>>()?;
    let table = batch(&batches, "source_context_sites");
    let source_context_sites = (0..table.len()).map(|row| {
        Ok(cpg_schema::context_protocol::SourceContextSitesRow { snapshot_id: Id::ZERO,
            site_id: super::id(&table.id("site_id", row)?)?,
            function_node_id: super::id(&table.id("function_node_id", row)?)?,
            with_node_id: super::id(&table.id("with_node_id", row)?)?,
            with_fact_id: super::id(&table.id("with_fact_id", row)?)?,
            item_node_id: super::id(&table.id("item_node_id", row)?)?,
            item_fact_id: super::id(&table.id("item_fact_id", row)?)?,
            item_ordinal: table.integer("item_ordinal", row)?,
            call_node_id: super::id(&table.id("call_node_id", row)?)?,
            call_fact_id: super::id(&table.id("call_fact_id", row)?)?,
            expression_fact_id: super::id(&table.id("expression_fact_id", row)?)?,
            protocol_id: super::id(&table.id("protocol_id", row)?)?,
            model_id: super::id(&table.id("model_id", row)?)?,
            class_node_id: super::id(&table.id("class_node_id", row)?)?,
            reference_fact_id: super::id(&table.id("reference_fact_id", row)?)?,
            resolution_fact_id: super::id(&table.id("resolution_fact_id", row)?)?,
            import_binding_fact_id: super::id(&table.id("import_binding_fact_id", row)?)?,
            import_region_fact_id: super::id(&table.id("import_region_fact_id", row)?)?,
            import_condition_id: super::id(&table.id("import_condition_id", row)?)?,
            export_fact_id: super::id(&table.id("export_fact_id", row)?)?,
            allocation_call_fact_id: super::id(&table.id("allocation_call_fact_id", row)?)?,
            initialization_call_fact_id: super::id(&table.id("initialization_call_fact_id", row)?)?,
            constructor_valid: table.boolean("constructor_valid", row)?,
            entry_argument_fact_id: table.optional_id("entry_argument_fact_id", row)?.as_deref().map(super::id).transpose()?,
        })
    }).collect::<PyResult<_>>()?;
    let table = batch(&batches, "source_context_arguments");
    let source_context_arguments = (0..table.len()).map(|row| {
        Ok(cpg_schema::context_protocol::SourceContextArgumentsRow { snapshot_id: Id::ZERO,
            site_id: super::id(&table.id("site_id", row)?)?,
            ordinal: table.integer("ordinal", row)?,
            argument_fact_id: super::id(&table.id("argument_fact_id", row)?)?,
            expression_fact_id: super::id(&table.id("expression_fact_id", row)?)?,
            parameter_fact_id: table.optional_id("parameter_fact_id", row)?.as_deref().map(super::id).transpose()?,
            exception_class_node_id: table.optional_id("exception_class_node_id", row)?.as_deref().map(super::id).transpose()?,
            exception_class_fact_id: table.optional_id("exception_class_fact_id", row)?.as_deref().map(super::id).transpose()?,
            exception_module_fact_id: table.optional_id("exception_module_fact_id", row)?.as_deref().map(super::id).transpose()?,
            reference_fact_id: table.optional_id("reference_fact_id", row)?.as_deref().map(super::id).transpose()?,
            resolution_fact_id: table.optional_id("resolution_fact_id", row)?.as_deref().map(super::id).transpose()?,
        })
    }).collect::<PyResult<_>>()?;
    let table=batch(&batches,"return_completion_certificates");
    let return_certificates=(0..table.len()).map(|row| {
        let key=|name|super::id(&table.id(name,row)?);
        Ok(cpg_schema::completion_proof::ReturnCompletionCertificatesRow {snapshot_id:Id::ZERO,
            certificate_id:key("certificate_id")?,function_node_id:key("function_node_id")?,return_site_fact_id:key("return_site_fact_id")?,
            entry_condition_id:key("entry_condition_id")?,exit_condition_id:key("exit_condition_id")?,
            entry_count:table.integer("entry_count",row)?,exit_count:table.integer("exit_count",row)?,
            entry_digest:super::digest(&table.digest("entry_digest",row)?)?,exit_digest:super::digest(&table.digest("exit_digest",row)?)?,
        })
    }).collect::<PyResult<_>>()?;
    Ok(Inputs {
        source_body_completions,
        source_body_steps,
        source_body_release_inputs,
        source_call_normals,
        source_call_header_steps,
        model_frame_exits,model_frame_exit_arguments,model_frame_exit_steps,
        modeled_identities,        context_value_identities,
        return_certificates,
        model_context_protocols,
        source_context_sites,
        source_context_arguments,

        identities,
        return_sites,
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
