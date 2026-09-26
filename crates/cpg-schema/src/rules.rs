//! Cross-table validation (DESIGN §8): one DataFusion query per rule, generated from the
//! contracts. A rule passes when its query returns no rows. The rules are:
//! - `key`: every table's declared total key is unique in the snapshot;
//! - `ref`: every reference below names an existing row (null passes);
//! - `fact`: every raw row has its `facts` row, and every `facts` row has its raw row;
//! - `codebook`: every `Int16` codebook column holds a code of its codebook;
//! - `finite`: every `Float64` value, scalar or listed, is finite;
//! - `coverage`: every family a run declares has a row for every module of its release;
//! - `semantic`: hand-written rules no declaration generates (listed in [`semantic`]).
//!
//! The queries read tables registered under their own names and filtered to one snapshot.

use crate::codebook::{
    AssertionKind, BehaviorKind, BindingKind, Codebook, DeclarationKind, EdgeKind, EvidenceStatus, FactFamily,
    PremiseKind, SourceRole, SupportRole, Verdict, registry,
};
use crate::column::CODEBOOK_KEY;
use crate::derived::{Derived, FlowValueCallLinks};
use crate::table::Table;

/// One validation query and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    pub sql: String,
}

/// A column whose non-null values must name a row of one of `to`.
pub struct Reference {
    pub table: &'static str,
    pub column: &'static str,
    pub to: &'static [(&'static str, &'static str)],
}

const FACT: &[(&str, &str)] = &[("facts", "fact_id")];
const NODE: &[(&str, &str)] = &[("nodes", "node_id")];

const fn r(
    table: &'static str,
    column: &'static str,
    to: &'static [(&'static str, &'static str)],
) -> Reference {
    Reference { table, column, to }
}

/// The declared references of every stored table (keys and fact links are generated).
pub const REFERENCES: &[Reference] = &[
    // Node-valued columns are generated from the registry (`graph::node_columns`) and checked
    // against `nodes` with their kinds; these are the fact, provenance and composite references.
    r("facts", "run_id", &[("runs", "run_id")]),
    r("runs", "release_id", &[("releases", "release_id")]),
    // A run's release has modules or documents, so `coverage:complete` cannot pass vacuously.
    r(
        "runs",
        "release_id",
        &[("source_files", "release_id"), ("documents", "release_id")],
    ),
    r("releases", "release_id", &[("runs", "release_id")]),
    r("distributions", "context_id", &[("contexts", "context_id")]),
    r("source_files", "release_id", &[("runs", "release_id")]),
    r("documents", "release_id", &[("runs", "release_id")]),
    r("runs", "context_id", &[("contexts", "context_id")]),
    r("runs", "producer_id", &[("producers", "producer_id")]),
    r("coverage", "run_id", &[("runs", "run_id")]),
    // Analysis results (ADR-0019): provenance in-row, citing runs, nodes, edges and facts.
    r("analysis_invocations", "run_id", &[("runs", "run_id")]),
    r("analysis_invocations", "subject_node_id", NODE),
    r(
        "findings",
        "invocation_id",
        &[("analysis_invocations", "invocation_id")],
    ),
    r("findings", "subject_node_id", NODE),
    r("findings", "related_node_id", NODE),
    r("findings", "condition_node_id", NODE),
    r(
        "finding_members",
        "finding_id",
        &[("findings", "finding_id")],
    ),
    r("finding_members", "node_id", NODE),
    r("finding_members", "cited_fact_id", FACT),
    r("witnesses", "finding_id", &[("findings", "finding_id")]),
    r("witnesses", "caller_node_id", NODE),
    r("witnesses", "call_site_node_id", NODE),
    r("witnesses", "callee_node_id", NODE),
    r("witnesses", "edge_id", &[("edges", "edge_id")]),
    r("evidence", "cited_fact_id", FACT),
    r("evidence", "node_id", NODE),
    r("evidence", "module_node_id", NODE),
    r(
        "flow_value_calls",
        "flow_value_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r("flow_value_calls", "use_id", &[("flow_uses", "use_id")]),
    r("flow_reach_boundaries", "use_id", &[("flow_uses", "use_id")]),
    r(
        "flow_value_call_links",
        "flow_value_call_fact_id",
        &[("flow_value_calls", "fact_id")],
    ),
    r(
        "flow_value_call_links",
        "flow_value_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "flow_value_call_links",
        "call_node_id",
        &[("call_syntax", "node_id")],
    ),
    r("flow_value_call_links", "call_fact_id", FACT),
    r(
        "flow_value_call_links",
        "argument_node_id",
        &[("arguments", "node_id")],
    ),
    r("flow_value_call_links", "argument_fact_id", FACT),
    r(
        "value_flow_contributions",
        "flow_value_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "value_flow_contributions",
        "use_id",
        &[("flow_uses", "use_id")],
    ),
    r(
        "value_flow_contributions",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "value_flow_contributions",
        "sink_function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "value_flow_contributions",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "value_flow_contributions",
        "class_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "flow_value_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "flow_value_call_fact_id",
        &[("flow_value_calls", "fact_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "call_site_node_id",
        &[("call_syntax", "node_id")],
    ),
    r("modeled_exact_value_transfers", "call_fact_id", FACT),
    r(
        "modeled_exact_value_transfers",
        "argument_node_id",
        &[("arguments", "node_id")],
    ),
    r("modeled_exact_value_transfers", "argument_fact_id", FACT),
    r(
        "modeled_exact_value_transfers",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "use_id",
        &[("flow_uses", "use_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r("modeled_exact_value_transfers", "pysa_fact_id", FACT),
    r(
        "modeled_exact_value_transfers",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "rule_id",
        &[("model_transfers", "rule_id")],
    ),
    r(
        "modeled_exact_value_transfers",
        "target_definition_fact_id",
        FACT,
    ),
    r(
        "modeled_argument_evaluations",
        "candidate_flow_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "modeled_argument_evaluations",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r("modeled_argument_evaluations", "pysa_fact_id", FACT),
    r(
        "modeled_argument_evaluations",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_argument_evaluations",
        "rule_id",
        &[("model_transfers", "rule_id")],
    ),
    r(
        "modeled_argument_evaluations",
        "call_site_node_id",
        &[("call_syntax", "node_id")],
    ),
    r(
        "modeled_argument_evaluations",
        "argument_node_id",
        &[("arguments", "node_id")],
    ),
    r("modeled_argument_evaluations", "argument_fact_id", FACT),
    r(
        "modeled_argument_evaluations",
        "evidence_id",
        &[
            ("flow_values", "fact_id"),
            ("syntax_nodes", "fact_id"),
            ("reference_resolutions", "fact_id"),
        ],
    ),
    r(
        "modeled_argument_evaluations",
        "condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "successor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "predecessor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "successor_use_id",
        &[("flow_uses", "use_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "predecessor_use_id",
        &[("flow_uses", "use_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "reaching_fact_id",
        FACT,
    ),
    r(
        "value_flow_predecessor_candidates",
        "reaching_condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "definition_id",
        &[("flow_definitions", "definition_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "definition_fact_id",
        FACT,
    ),
    r(
        "value_flow_predecessor_candidates",
        "successor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_candidates",
        "predecessor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_compatibility",
        "successor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "value_flow_predecessor_compatibility",
        "predecessor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "value_flow_predecessor_compatibility",
        "reaching_fact_id",
        FACT,
    ),
    r(
        "value_flow_predecessor_compatibility",
        "predecessor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_compatibility",
        "successor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "value_flow_predecessor_compatibility",
        "reaching_condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "successor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "predecessor_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r("modeled_assignment_return_paths", "reaching_fact_id", FACT),
    r(
        "modeled_assignment_return_paths",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "call_site_node_id",
        &[("call_syntax", "node_id")],
    ),
    r("modeled_assignment_return_paths", "call_fact_id", FACT),
    r("modeled_assignment_return_paths", "pysa_fact_id", FACT),
    r(
        "modeled_assignment_return_paths",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "rule_id",
        &[("model_transfers", "rule_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "target_definition_fact_id",
        FACT,
    ),
    r(
        "modeled_assignment_return_paths",
        "predecessor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "reaching_condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "modeled_assignment_return_paths",
        "successor_condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "summary_components",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "summary_flows",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "summary_flows",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "summary_flows",
        "condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "summary_flows",
        "source_flow_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r("summary_flows", "return_site_fact_id", FACT),
    r("summary_flows", "return_region_fact_id", FACT),
    r(
        "summary_flow_steps",
        "summary_id",
        &[("summary_flows", "summary_id")],
    ),
    r(
        "summary_flow_steps",
        "evidence_id",
        &[
            ("flow_values", "fact_id"),
            ("reference_resolutions", "fact_id"),
            ("syntax_nodes", "fact_id"),
            ("call_syntax", "fact_id"),
            ("call_targets", "pysa_fact_id"),
            ("model_transfers", "rule_id"),
            ("model_targets", "model_id"),
            ("bindings", "fact_id"),
            ("flow_regions", "fact_id"),
            ("flow_reaching", "fact_id"),
            ("summary_flows", "summary_id"),
        ],
    ),
    r(
        "summary_flow_steps",
        "condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "summary_boundaries",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "summary_boundaries",
        "parameter_node_id",
        &[("parameter_syntax", "node_id")],
    ),
    r(
        "summary_boundaries",
        "source_flow_fact_id",
        &[("flow_values", "fact_id")],
    ),
    r(
        "summary_boundaries",
        "condition_id",
        &[("analysis_conditions", "condition_id")],
    ),
    r(
        "model_exceptions",
        "class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_exceptions", "class_fact_id", FACT),
    r(
        "model_exceptions",
        "to_class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_exceptions", "to_class_fact_id", FACT),
    r(
        "modeled_exception_sites",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r(
        "modeled_exception_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("modeled_exception_sites", "call_fact_id", FACT),
    r("modeled_exception_sites", "pysa_fact_id", FACT),
    r(
        "modeled_exception_sites",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_exception_sites",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_exception_sites",
        "rule_id",
        &[("model_exceptions", "rule_id")],
    ),
    r("modeled_exception_sites", "target_definition_fact_id", FACT),
    r(
        "modeled_exception_sites",
        "class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("modeled_exception_sites", "class_fact_id", FACT),
    r(
        "modeled_exception_sites",
        "to_class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("modeled_exception_sites", "to_class_fact_id", FACT),
    r(
        "modeled_exception_handler_candidates",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r("modeled_exception_handler_candidates", "pysa_fact_id", FACT),
    r(
        "modeled_exception_handler_candidates",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_exception_handler_candidates",
        "rule_id",
        &[("model_exceptions", "rule_id")],
    ),
    r("modeled_exception_handler_candidates", "call_fact_id", FACT),
    r(
        "modeled_exception_handler_candidates",
        "raised_class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r(
        "modeled_exception_handler_candidates",
        "raised_class_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_handler_candidates",
        "try_node_id",
        &[("handler_clauses", "try_node_id")],
    ),
    r(
        "modeled_exception_handler_candidates",
        "handler_node_id",
        &[("handler_clauses", "handler_node_id")],
    ),
    r(
        "modeled_exception_handler_candidates",
        "handler_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_handler_candidates",
        "handler_class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r(
        "modeled_exception_handler_candidates",
        "handler_class_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_handler_candidates",
        "class_mro_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_handler_walks",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r("modeled_exception_handler_walks", "pysa_fact_id", FACT),
    r(
        "modeled_exception_handler_walks",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_exception_handler_walks",
        "rule_id",
        &[("model_exceptions", "rule_id")],
    ),
    r(
        "modeled_exception_handler_walks",
        "source_syntax_fact_id",
        FACT,
    ),
    r("assertions", "run_id", &[("runs", "run_id")]),
    r("assertions", "subject_node_id", NODE),
    r(
        "assertion_support",
        "assertion_id",
        &[("assertions", "assertion_id")],
    ),
    r(
        "assertion_support",
        "finding_id",
        &[("findings", "finding_id")],
    ),
    r(
        "assertion_support",
        "evidence_id",
        &[("evidence", "evidence_id")],
    ),
    r("briefs", "run_id", &[("runs", "run_id")]),
    r("briefs", "seed_node_id", NODE),
    r("brief_assertions", "brief_id", &[("briefs", "brief_id")]),
    r(
        "brief_assertions",
        "assertion_id",
        &[("assertions", "assertion_id")],
    ),
    r("brief_members", "brief_id", &[("briefs", "brief_id")]),
    r("brief_members", "export_node_id", NODE),
    r("brief_members", "declaration_node_id", NODE),
    r("brief_documents", "brief_id", &[("briefs", "brief_id")]),
    r(
        "brief_documents",
        "spec_hash",
        &[("embedding_specs", "spec_hash")],
    ),
    r(
        "model_targets",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_targets", "target_module_fact_id", FACT),
    r("model_targets", "target_definition_fact_id", FACT),
    r(
        "model_applications",
        "call_site_node_id",
        &[("call_syntax", "node_id")],
    ),
    r(
        "model_applications",
        "module_node_id",
        &[("source_files", "module_node_id")],
    ),
    r(
        "model_applications",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("model_applications", "call_fact_id", FACT),
    r("model_applications", "pysa_fact_id", FACT),
    r(
        "model_applications",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "model_applications",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r("model_applications", "target_module_fact_id", FACT),
    r("model_applications", "target_definition_fact_id", FACT),
    r(
        "model_formal_paths",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "model_formal_paths",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r("model_formal_paths", "target_definition_fact_id", FACT),
    r(
        "model_argument_bindings",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r("model_argument_bindings", "pysa_fact_id", FACT),
    r(
        "model_argument_bindings",
        "model_id",
        &[("model_formal_paths", "model_id")],
    ),
    r(
        "model_argument_bindings",
        "target_node_id",
        &[("model_applications", "target_node_id")],
    ),
    r(
        "model_argument_bindings",
        "rule_id",
        &[("model_formal_paths", "rule_id")],
    ),
    r(
        "model_argument_bindings",
        "argument_node_id",
        &[("arguments", "node_id")],
    ),
    r("model_argument_bindings", "argument_fact_id", FACT),
    r(
        "modeled_callback_sites",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r(
        "modeled_callback_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("modeled_callback_sites", "call_fact_id", FACT),
    r("modeled_callback_sites", "pysa_fact_id", FACT),
    r(
        "modeled_callback_sites",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_callback_sites",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_callback_sites",
        "rule_id",
        &[("model_callbacks", "rule_id")],
    ),
    r("modeled_callback_sites", "target_definition_fact_id", FACT),
    r(
        "modeled_callback_sites",
        "callback_path_id",
        &[("model_callbacks", "callback_path_id")],
    ),
    r(
        "modeled_callback_sites",
        "argument_node_id",
        &[("arguments", "node_id")],
    ),
    r("modeled_callback_sites", "argument_fact_id", FACT),
    r(
        "modeled_resource_sites",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r(
        "modeled_resource_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("modeled_resource_sites", "call_fact_id", FACT),
    r("modeled_resource_sites", "pysa_fact_id", FACT),
    r(
        "modeled_resource_sites",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_resource_sites",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_resource_sites",
        "rule_id",
        &[("model_resources", "rule_id")],
    ),
    r("modeled_resource_sites", "target_definition_fact_id", FACT),
    r(
        "modeled_resource_sites",
        "resource_path_id",
        &[("model_resources", "resource_path_id")],
    ),
    r(
        "modeled_resource_sites",
        "source_expression_node_id",
        &[("call_syntax", "node_id"), ("arguments", "node_id")],
    ),
    r("modeled_resource_sites", "source_expression_fact_id", FACT),
    r(
        "modeled_transfer_sites",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r(
        "modeled_transfer_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("modeled_transfer_sites", "call_fact_id", FACT),
    r("modeled_transfer_sites", "pysa_fact_id", FACT),
    r(
        "modeled_transfer_sites",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_transfer_sites",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_transfer_sites",
        "rule_id",
        &[("model_transfers", "rule_id")],
    ),
    r("modeled_transfer_sites", "target_definition_fact_id", FACT),
    r(
        "modeled_transfer_sites",
        "input_path_id",
        &[("model_transfers", "input_path_id")],
    ),
    r(
        "modeled_transfer_sites",
        "output_path_id",
        &[("model_transfers", "output_path_id")],
    ),
    r(
        "modeled_transfer_sites",
        "input_expression_node_id",
        &[("arguments", "node_id")],
    ),
    r("modeled_transfer_sites", "input_expression_fact_id", FACT),
    r(
        "modeled_transfer_sites",
        "output_expression_node_id",
        &[("call_syntax", "node_id")],
    ),
    r("modeled_transfer_sites", "output_expression_fact_id", FACT),
    r(
        "modeled_effect_sites",
        "call_site_node_id",
        &[("model_applications", "call_site_node_id")],
    ),
    r(
        "modeled_effect_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("modeled_effect_sites", "call_fact_id", FACT),
    r("modeled_effect_sites", "pysa_fact_id", FACT),
    r(
        "modeled_effect_sites",
        "target_node_id",
        &[("model_targets", "target_node_id")],
    ),
    r(
        "modeled_effect_sites",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_effect_sites",
        "rule_id",
        &[("model_effects", "rule_id")],
    ),
    r("modeled_effect_sites", "target_definition_fact_id", FACT),
    r(
        "modeled_effect_sites",
        "subject_path_id",
        &[("model_effects", "subject_path_id")],
    ),
    r(
        "modeled_effect_sites",
        "subject_expression_node_id",
        &[("arguments", "node_id")],
    ),
    r("modeled_effect_sites", "subject_expression_fact_id", FACT),
    r(
        "model_transfers",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_transfers", "target_definition_fact_id", FACT),
    r(
        "model_effects",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_effects", "target_definition_fact_id", FACT),
    r(
        "model_callbacks",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_callbacks", "target_definition_fact_id", FACT),
    r(
        "model_resources",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_resources", "target_definition_fact_id", FACT),
    r(
        "model_exceptions",
        "target_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("model_exceptions", "target_definition_fact_id", FACT),
    r(
        "handler_types",
        "handler_node_id",
        &[("handler_clauses", "handler_node_id")],
    ),
    r(
        "handler_types",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "handler_types",
        "type_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r("handler_types", "type_fact_id", FACT),
    r("handler_types", "reference_fact_id", FACT),
    r("handler_types", "resolution_fact_id", FACT),
    r(
        "handler_types",
        "class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r("handler_types", "class_fact_id", FACT),
    r("handler_types", "class_module_fact_id", FACT),
    r(
        "exit_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r("exit_sites", "site_node_id", &[("syntax_nodes", "node_id")]),
    r(
        "exit_sites",
        "module_node_id",
        &[("source_files", "module_node_id")],
    ),
    r(
        "exit_sites",
        "source_fact_id",
        &[("syntax_nodes", "fact_id")],
    ),
    r(
        "exit_sites",
        "region_fact_id",
        &[("flow_regions", "fact_id")],
    ),
    r(
        "exit_sites",
        "condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "handler_clauses",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "handler_clauses",
        "try_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r(
        "handler_clauses",
        "handler_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r(
        "handler_clauses",
        "module_node_id",
        &[("source_files", "module_node_id")],
    ),
    r("handler_clauses", "try_fact_id", FACT),
    r("handler_clauses", "handler_fact_id", FACT),
    r(
        "handler_clauses",
        "type_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r("handler_clauses", "type_fact_id", FACT),
    r("handler_clauses", "try_region_fact_id", FACT),
    r(
        "handler_clauses",
        "entry_condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "handler_actions",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "handler_actions",
        "handler_node_id",
        &[("handler_clauses", "handler_node_id")],
    ),
    r(
        "handler_actions",
        "action_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r(
        "handler_actions",
        "module_node_id",
        &[("source_files", "module_node_id")],
    ),
    r("handler_actions", "action_fact_id", FACT),
    r("handler_actions", "region_fact_id", FACT),
    r(
        "handler_actions",
        "condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "handler_return_none_sites",
        "function_node_id",
        &[("declarations", "node_id")],
    ),
    r(
        "handler_return_none_sites",
        "handler_node_id",
        &[("handler_clauses", "handler_node_id")],
    ),
    r("handler_return_none_sites", "handler_fact_id", FACT),
    r(
        "handler_return_none_sites",
        "return_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r("handler_return_none_sites", "return_fact_id", FACT),
    r(
        "handler_return_none_sites",
        "none_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r("handler_return_none_sites", "none_fact_id", FACT),
    r("handler_return_none_sites", "region_fact_id", FACT),
    r(
        "handler_return_none_sites",
        "condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "call_site_node_id",
        &[("call_syntax", "node_id")],
    ),
    r("modeled_exception_return_none_paths", "pysa_fact_id", FACT),
    r(
        "modeled_exception_return_none_paths",
        "model_id",
        &[("model_targets", "model_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "rule_id",
        &[("model_exceptions", "rule_id")],
    ),
    r("modeled_exception_return_none_paths", "call_fact_id", FACT),
    r(
        "modeled_exception_return_none_paths",
        "source_syntax_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "raised_class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "raised_class_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "try_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "handler_node_id",
        &[("handler_clauses", "handler_node_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "handler_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "class_mro_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "return_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r(
        "modeled_exception_return_none_paths",
        "return_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "none_node_id",
        &[("syntax_nodes", "node_id")],
    ),
    r("modeled_exception_return_none_paths", "none_fact_id", FACT),
    r(
        "modeled_exception_return_none_paths",
        "handler_region_fact_id",
        FACT,
    ),
    r(
        "modeled_exception_return_none_paths",
        "handler_condition_id",
        &[("conditions", "condition_id")],
    ),
    r(
        "context_parameters",
        "symbol_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r(
        "context_parameters",
        "module_node_id",
        &[("context_modules", "module_node_id")],
    ),
    r(
        "context_class_mro",
        "class_node_id",
        &[("context_definitions", "symbol_node_id")],
    ),
    r(
        "context_class_mro",
        "module_node_id",
        &[("context_modules", "module_node_id")],
    ),
    r("provider_node_map", "pysa_fact_id", FACT),
    r("provider_node_map", "declaration_fact_id", FACT),
    r("provider_class_map", "pysa_fact_id", FACT),
    r("provider_class_map", "declaration_fact_id", FACT),
    r("synthetic_callables", "pysa_fact_id", FACT),
    r("exports", "public_fact_id", FACT),
    r("exports", "declaration_fact_id", FACT),
    r("signatures", "declaration_fact_id", FACT),
    r(
        "parameters",
        "signature_node_id",
        &[("signatures", "signature_node_id")],
    ),
    r("parameters", "syntax_fact_id", FACT),
    r("parameters", "semantics_fact_id", FACT),
    r("resolutions", "call_fact_id", FACT),
    r(
        "call_targets",
        "call_site_node_id",
        &[("resolutions", "call_site_node_id")],
    ),
    r("call_targets", "pysa_fact_id", FACT),
    r("ancestry_targets", "ancestry_fact_id", FACT),
    r("override_targets", "function_fact_id", FACT),
    r("nodes", "existence_fact_id", FACT),
    r("graph_gaps", "gap_fact_id", FACT),
];

/// Name, key and schema of a stored table.
struct Shape {
    name: &'static str,
    key: &'static [&'static str],
    schema: arrow_schema::SchemaRef,
}

fn shapes() -> Vec<Shape> {
    macro_rules! all {
        ($($t:ty),+) => {
            vec![$(Shape {
                name: <$t as Table>::NAME,
                key: <$t as Table>::key(),
                schema: <$t as Table>::schema(),
            }),+]
        };
    }
    let mut out = crate::for_each_table!(all);
    out.extend(crate::for_each_derived_table!(all));
    out.extend(crate::for_each_analysis_table!(all));
    out.extend(crate::for_each_global_table!(all));
    out
}

fn quoted(values: impl IntoIterator<Item = impl std::fmt::Display>) -> String {
    values
        .into_iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rules that span tables in ways no declaration captures.
fn semantic() -> Vec<Rule> {
    let calls = FactFamily::Calls.code();
    // The flow family's parity with ours (ADR-0022 §The flow provider), over modules the
    // provider indexed completely.
    let flow_complete = format!(
        "SELECT scope_node_id AS module_node_id FROM coverage \
         WHERE fact_family = {flow} AND status = {complete}",
        flow = FactFamily::Flow.code(),
        complete = crate::codebook::CoverageStatus::CompleteUnderStatedModel.code(),
    );
    let name_place = "strpos(place, '.') = 0 AND strpos(place, '[') = 0";
    // Bindings ty never defines as such: `global`/`nonlocal` declarations, `del`, names the
    // runtime binds with no statement, and a star import (ours is the `*`; ty's are the names it
    // brings in).
    let not_defined = crate::flows::codes(&[
        crate::codebook::BindingKind::Global,
        crate::codebook::BindingKind::Nonlocal,
        crate::codebook::BindingKind::Del,
        crate::codebook::BindingKind::Implicit,
        crate::codebook::BindingKind::StarImport,
    ]);
    let flow_rules = [
        (
            // Every name ty reads, outside annotations, is one of our references.
            "semantic:flow-use-is-a-reference",
            format!(
                "SELECT u.use_id FROM flow_uses u \
                 LEFT ANTI JOIN references r ON r.module_node_id = u.module_node_id \
                   AND r.start_byte = u.start_byte AND r.end_byte = u.end_byte \
                 WHERE NOT u.annotation AND {name_place}"
            ),
        ),
        (
            // Every reference in a completely indexed module is a use ty reads (the direction
            // that protects negative answers; the Stage 2 review's F5).
            "semantic:reference-is-a-flow-use",
            format!(
                "SELECT r.node_id FROM references r \
                 JOIN ({flow_complete}) m ON m.module_node_id = r.module_node_id \
                 LEFT ANTI JOIN flow_uses u ON u.module_node_id = r.module_node_id \
                   AND u.start_byte = r.start_byte AND u.end_byte = r.end_byte"
            ),
        ),
        (
            // A ty-only package-submodule definition is not a lexical binding row.
            "semantic:ty-only-definition-not-lexical-binding",
            format!(
                "SELECT node_id FROM bindings WHERE kind = {}",
                BindingKind::ImportFromSubmodule.code()
            ),
        ),
        (
            // Every name ty defines is one of our bindings.
            "semantic:flow-definition-is-a-binding",
            format!(
                "SELECT d.definition_id FROM flow_definitions d \
                 LEFT ANTI JOIN bindings b ON b.module_node_id = d.module_node_id \
                   AND b.start_byte = d.start_byte AND b.end_byte = d.end_byte \
                 WHERE {name_place} AND d.kind <> {}",
                BindingKind::ImportFromSubmodule.code()
            ),
        ),
        (
            // Every binding a statement makes, in a completely indexed module, is a definition ty
            // records.
            "semantic:binding-is-a-flow-definition",
            format!(
                "SELECT b.node_id FROM bindings b \
                 JOIN ({flow_complete}) m ON m.module_node_id = b.module_node_id \
                 LEFT ANTI JOIN flow_definitions d ON d.module_node_id = b.module_node_id \
                   AND d.start_byte = b.start_byte AND d.end_byte = b.end_byte \
                 WHERE b.kind NOT IN ({not_defined})"
            ),
        ),
        (
            // ty's reaching definitions of a name lie within our candidate bindings for it (the
            // Stage 2.1 spike's exit test, kept).
            "semantic:flow-reaching-within-candidates",
            "SELECT fr.use_id FROM flow_reaching fr \
             JOIN flow_uses u ON u.use_id = fr.use_id \
             JOIN references r ON r.module_node_id = u.module_node_id \
               AND r.start_byte = u.start_byte AND r.end_byte = u.end_byte \
             JOIN flow_definitions d ON d.definition_id = fr.definition_id \
             JOIN bindings b ON b.module_node_id = d.module_node_id \
               AND b.start_byte = d.start_byte AND b.end_byte = d.end_byte \
             LEFT ANTI JOIN reference_resolutions rr \
               ON rr.reference_id = r.node_id AND rr.binding_id = b.node_id"
                .to_owned(),
        ),
        (
            // A call path is a dense outer-to-inner sequence on the same value/use. Every
            // subsequent call lies inside the preceding operand, and the final operand contains
            // the cited use. A through-call marker without its path cannot support L3 evidence.
            "semantic:flow-value-call-path",
            "WITH steps AS (SELECT c.*, v.module_node_id AS value_module, \
                    v.use_id AS value_use, v.through_call, v.sink_start_byte, v.sink_end_byte, \
                    u.start_byte AS use_start, u.end_byte AS use_end, \
                    row_number() OVER (PARTITION BY c.flow_value_fact_id ORDER BY c.step) - 1 AS expected_step, \
                    lag(c.operand_start_byte) OVER (PARTITION BY c.flow_value_fact_id ORDER BY c.step) AS parent_start, \
                    lag(c.operand_end_byte) OVER (PARTITION BY c.flow_value_fact_id ORDER BY c.step) AS parent_end, \
                    lead(c.step) OVER (PARTITION BY c.flow_value_fact_id ORDER BY c.step) AS next_step \
                 FROM flow_value_calls c \
                 JOIN flow_values v ON v.fact_id = c.flow_value_fact_id \
                 JOIN flow_uses u ON u.use_id = c.use_id) \
             SELECT fact_id FROM steps WHERE module_node_id <> value_module OR use_id <> value_use \
                OR NOT through_call OR step <> expected_step \
                OR (step = 0 AND (call_start_byte < sink_start_byte OR call_end_byte > sink_end_byte)) \
                OR (step > 0 AND (call_start_byte < parent_start OR call_end_byte > parent_end)) \
                OR (next_step IS NULL AND (use_start < operand_start_byte OR use_end > operand_end_byte)) \
             UNION ALL \
             SELECT v.fact_id FROM flow_values v \
             LEFT JOIN flow_value_calls c ON c.flow_value_fact_id = v.fact_id \
             GROUP BY v.fact_id, v.through_call \
             HAVING (v.through_call AND count(c.fact_id) = 0) \
                 OR (NOT v.through_call AND count(c.fact_id) > 0) \
                 OR count(DISTINCT c.step) <> count(c.fact_id)"
                .to_owned(),
        ),
        (
            // Recompute the source bridge with the same declared DataFusion relation on the
            // published raw facts. A forged resolved id, withheld unknown, or missing row is a
            // publication violation, not a later L3 heuristic.
            "semantic:flow-call-link-source-equality",
            format!(
                "WITH expected AS ({expected}) \
                 SELECT coalesce(e.flow_value_call_fact_id, l.flow_value_call_fact_id) AS step_fact_id \
                 FROM expected e FULL OUTER JOIN flow_value_call_links l \
                   ON e.flow_value_call_fact_id = l.flow_value_call_fact_id \
                 WHERE e.flow_value_call_fact_id IS NULL OR l.flow_value_call_fact_id IS NULL \
                    OR (e.flow_value_fact_id IS DISTINCT FROM l.flow_value_fact_id) \
                    OR (e.call_node_id IS DISTINCT FROM l.call_node_id) \
                    OR (e.call_fact_id IS DISTINCT FROM l.call_fact_id) \
                    OR (e.argument_node_id IS DISTINCT FROM l.argument_node_id) \
                    OR (e.argument_fact_id IS DISTINCT FROM l.argument_fact_id) \
                    OR (e.status IS DISTINCT FROM l.status)",
                expected = <FlowValueCallLinks as Derived>::sql(),
            ),
        ),
    ];
    flow_rules.into_iter().chain([
        (
            // A missing argument would make "all evaluated" vacuously true. The Ruff call
            // counts and the placed role rows must agree, with dense source ordinals.
            "semantic:call-argument-coverage",
            "WITH grouped AS (SELECT call_node_id, count(*) AS n, \
                    min(ordinal) AS first_ordinal, max(ordinal) AS last_ordinal \
                    FROM arguments GROUP BY call_node_id) \
             SELECT c.node_id FROM call_syntax c \
             LEFT JOIN grouped g ON g.call_node_id = c.node_id \
             WHERE COALESCE(g.n, 0) <> c.positional_count + c.keyword_count \
                OR (g.n > 0 AND (g.first_ordinal <> 0 OR g.last_ordinal <> g.n - 1))"
                .to_owned(),
        ),
        (
            "semantic:context-class-mro-coverage",
            format!(
                "SELECT d.symbol_node_id FROM context_definitions d \
                 LEFT ANTI JOIN context_class_mro m ON m.class_node_id = d.symbol_node_id \
                 WHERE d.kind = {}",
                crate::codebook::DefinitionKind::Class.code(),
            ),
        ),
        (
            "semantic:context-class-mro-shape",
            "WITH distinct_ancestors AS ( \
               SELECT DISTINCT class_node_id, ordinal, ancestor_module, ancestor_key, \
                               ancestor_name, cyclic FROM context_class_mro \
             ) \
             SELECT class_node_id, count(*) AS rows, MIN(ordinal) AS first_ordinal, \
                    MAX(ordinal) AS last_ordinal, \
                    SUM(CASE WHEN ordinal IS NULL THEN 1 ELSE 0 END) AS markers \
             FROM distinct_ancestors \
             GROUP BY class_node_id \
             HAVING SUM(CASE WHEN ordinal IS NULL THEN 1 ELSE 0 END) > 1 \
                OR (SUM(CASE WHEN ordinal IS NULL THEN 1 ELSE 0 END) = 1 AND count(*) > 1) \
                OR (SUM(CASE WHEN ordinal IS NULL THEN 1 ELSE 0 END) = 0 \
                   AND (MIN(ordinal) <> 0 OR MAX(ordinal) <> count(*) - 1))"
                .to_owned(),
        ),
        (
            "semantic:context-class-mro-identity",
            format!(
                "SELECT m.class_node_id FROM context_class_mro m \
                 JOIN context_definitions d ON d.symbol_node_id = m.class_node_id \
                 WHERE d.module_node_id <> m.module_node_id OR d.kind <> {}",
                crate::codebook::DefinitionKind::Class.code(),
            ),
        ),
        (
            "semantic:model-target-provenance",
            format!(
                "SELECT mt.model_id FROM model_targets mt \
                 LEFT JOIN context_definitions d ON d.fact_id = mt.target_definition_fact_id \
                   AND d.symbol_node_id = mt.target_node_id \
                 LEFT JOIN context_modules m ON m.fact_id = mt.target_module_fact_id \
                   AND m.module_node_id = d.module_node_id \
                 WHERE d.fact_id IS NULL OR m.fact_id IS NULL OR mt.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:modeled-callback-site-shape",
            format!(
                "SELECT rule_id FROM modeled_callback_sites WHERE \
                 (binding_status = {bound} AND (argument_node_id IS NULL \
                   OR argument_fact_id IS NULL OR binding_reason IS NOT NULL)) \
                 OR (binding_status = {unknown} AND (argument_node_id IS NOT NULL \
                   OR argument_fact_id IS NOT NULL OR binding_reason IS NULL)) \
                 OR (candidate_set_complete_under_model AND has_unresolved_remainder) \
                 OR origin <> {synthetic}",
                bound = crate::codebook::ModelArgumentStatus::Bound.code(),
                unknown = crate::codebook::ModelArgumentStatus::Unknown.code(),
                synthetic = crate::codebook::Origin::SyntheticModel.code(),
            ),
        ),
        (
            "semantic:modeled-resource-site-shape",
            format!(
                "SELECT rule_id FROM modeled_resource_sites WHERE \
                 (source_status IN ({call_result}, {bound_argument}) \
                   AND (source_expression_node_id IS NULL \
                     OR source_expression_fact_id IS NULL OR source_reason IS NOT NULL)) \
                 OR (source_status = {unknown} \
                   AND (source_expression_node_id IS NOT NULL \
                     OR source_expression_fact_id IS NOT NULL OR source_reason IS NULL)) \
                 OR (source_status = {call_result} \
                   AND (resource_path_kind <> {return_value} \
                     OR source_expression_node_id <> call_site_node_id \
                     OR source_expression_fact_id <> call_fact_id)) \
                 OR (source_status = {bound_argument} \
                   AND resource_path_kind <> {parameter}) \
                 OR (candidate_set_complete_under_model AND has_unresolved_remainder) \
                 OR origin <> {synthetic}",
                call_result = crate::codebook::ModelResourceSourceStatus::CallResult.code(),
                bound_argument = crate::codebook::ModelResourceSourceStatus::BoundArgument.code(),
                unknown = crate::codebook::ModelResourceSourceStatus::Unknown.code(),
                return_value = crate::codebook::ModelPathKind::ReturnValue.code(),
                parameter = crate::codebook::ModelPathKind::Parameter.code(),
                synthetic = crate::codebook::Origin::SyntheticModel.code(),
            ),
        ),
        (
            "semantic:modeled-transfer-site-shape",
            format!(
                "SELECT rule_id FROM modeled_transfer_sites WHERE \
                 (input_status = {bound_argument} AND \
                   (input_path_kind <> {parameter} OR input_expression_node_id IS NULL \
                     OR input_expression_fact_id IS NULL OR input_reason IS NOT NULL)) \
                 OR (input_status = {unknown} AND \
                   (input_expression_node_id IS NOT NULL \
                     OR input_expression_fact_id IS NOT NULL OR input_reason IS NULL)) \
                 OR (output_status = {call_result} AND \
                   (output_path_kind <> {return_value} OR output_expression_node_id IS NULL \
                     OR output_expression_fact_id IS NULL OR output_reason IS NOT NULL \
                     OR output_expression_node_id <> call_site_node_id \
                     OR output_expression_fact_id <> call_fact_id)) \
                 OR (output_status = {unknown} AND \
                   (output_expression_node_id IS NOT NULL \
                     OR output_expression_fact_id IS NOT NULL OR output_reason IS NULL)) \
                 OR input_status = {call_result} OR output_status = {bound_argument} \
                 OR (candidate_set_complete_under_model AND has_unresolved_remainder) \
                 OR origin <> {synthetic}",
                bound_argument = crate::codebook::ModelTransferEndpointStatus::BoundArgument.code(),
                call_result = crate::codebook::ModelTransferEndpointStatus::CallResult.code(),
                unknown = crate::codebook::ModelTransferEndpointStatus::Unknown.code(),
                parameter = crate::codebook::ModelPathKind::Parameter.code(),
                return_value = crate::codebook::ModelPathKind::ReturnValue.code(),
                synthetic = crate::codebook::Origin::SyntheticModel.code(),
            ),
        ),
        (
            "semantic:modeled-effect-site-shape",
            format!(
                "SELECT rule_id FROM modeled_effect_sites WHERE \
                 (subject_status = {unqualified} AND \
                   (subject_path_id IS NOT NULL OR subject_path_kind IS NOT NULL \
                     OR subject_expression_node_id IS NOT NULL \
                     OR subject_expression_fact_id IS NOT NULL OR subject_reason IS NOT NULL)) \
                 OR (subject_status = {bound_argument} AND \
                   (subject_path_id IS NULL OR subject_path_kind <> {parameter} \
                     OR subject_expression_node_id IS NULL \
                     OR subject_expression_fact_id IS NULL OR subject_reason IS NOT NULL)) \
                 OR (subject_status = {unknown} AND \
                   (subject_path_id IS NULL OR subject_path_kind IS NULL \
                     OR subject_expression_node_id IS NOT NULL \
                     OR subject_expression_fact_id IS NOT NULL OR subject_reason IS NULL)) \
                 OR (candidate_set_complete_under_model AND has_unresolved_remainder) \
                 OR origin <> {synthetic}",
                unqualified = crate::codebook::ModelEffectSubjectStatus::Unqualified.code(),
                bound_argument = crate::codebook::ModelEffectSubjectStatus::BoundArgument.code(),
                unknown = crate::codebook::ModelEffectSubjectStatus::Unknown.code(),
                parameter = crate::codebook::ModelPathKind::Parameter.code(),
                synthetic = crate::codebook::Origin::SyntheticModel.code(),
            ),
        ),
        (
            "semantic:model-argument-status-shape",
            format!(
                "SELECT rule_id FROM model_argument_bindings WHERE signature_count <= 0 \
                 OR (status = {bound} AND (argument_node_id IS NULL OR argument_fact_id IS NULL \
                   OR reason IS NOT NULL OR matched_signatures <> signature_count)) \
                 OR (status = {unknown} AND (argument_node_id IS NOT NULL \
                   OR argument_fact_id IS NOT NULL OR reason IS NULL))",
                bound = crate::codebook::ModelArgumentStatus::Bound.code(),
                unknown = crate::codebook::ModelArgumentStatus::Unknown.code(),
            ),
        ),
        (
            "semantic:model-formal-path-source",
            format!(
                "SELECT p.rule_id FROM model_formal_paths p \
                 LEFT JOIN model_targets m ON m.model_id = p.model_id \
                   AND m.target_node_id = p.target_node_id \
                   AND m.target_definition_fact_id = p.target_definition_fact_id \
                   AND m.revision = p.revision \
                 WHERE m.model_id IS NULL OR p.formal_name = '' OR p.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-application-boundary",
            format!(
                "SELECT a.call_site_node_id FROM model_applications a \
                 LEFT JOIN model_targets m ON m.model_id = a.model_id \
                   AND m.target_node_id = a.target_node_id \
                   AND m.target_definition_fact_id = a.target_definition_fact_id \
                   AND m.target_module_fact_id = a.target_module_fact_id \
                   AND m.revision = a.revision \
                 LEFT JOIN resolutions r ON r.call_site_node_id = a.call_site_node_id \
                   AND r.call_fact_id = a.call_fact_id \
                 WHERE m.model_id IS NULL OR r.call_site_node_id IS NULL \
                   OR a.candidate_set_complete_under_model <> r.candidate_set_complete_under_model \
                   OR a.has_unresolved_remainder <> r.has_unresolved_remainder \
                   OR (a.candidate_set_complete_under_model AND a.has_unresolved_remainder) \
                   OR a.model_origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-transfer-target",
            format!(
                "SELECT t.rule_id FROM model_transfers t \
                 LEFT JOIN model_targets m ON m.model_id = t.model_id \
                   AND m.target_node_id = t.target_node_id \
                   AND m.target_definition_fact_id = t.target_definition_fact_id \
                   AND m.revision = t.revision \
                 WHERE m.model_id IS NULL OR t.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-effect-target",
            format!(
                "SELECT e.rule_id FROM model_effects e \
                 LEFT JOIN model_targets m ON m.model_id = e.model_id \
                   AND m.target_node_id = e.target_node_id \
                   AND m.target_definition_fact_id = e.target_definition_fact_id \
                   AND m.revision = e.revision \
                 WHERE m.model_id IS NULL OR e.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-callback-target",
            format!(
                "SELECT c.rule_id FROM model_callbacks c \
                 LEFT JOIN model_targets m ON m.model_id = c.model_id \
                   AND m.target_node_id = c.target_node_id \
                   AND m.target_definition_fact_id = c.target_definition_fact_id \
                   AND m.revision = c.revision \
                 WHERE m.model_id IS NULL OR c.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-resource-target",
            format!(
                "SELECT r.rule_id FROM model_resources r \
                 LEFT JOIN model_targets m ON m.model_id = r.model_id \
                   AND m.target_node_id = r.target_node_id \
                   AND m.target_definition_fact_id = r.target_definition_fact_id \
                   AND m.revision = r.revision \
                 WHERE m.model_id IS NULL OR r.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-exception-target",
            format!(
                "SELECT e.rule_id FROM model_exceptions e \
                 LEFT JOIN model_targets m ON m.model_id = e.model_id \
                   AND m.target_node_id = e.target_node_id \
                   AND m.target_definition_fact_id = e.target_definition_fact_id \
                   AND m.revision = e.revision \
                 WHERE m.model_id IS NULL OR e.origin <> {}",
                crate::codebook::Origin::SyntheticModel.code()
            ),
        ),
        (
            "semantic:model-exception-shape",
            format!(
                "SELECT rule_id FROM model_exceptions WHERE \
                 (action = {} AND (to_class IS NULL OR to_class_node_id IS NULL \
                   OR to_class_fact_id IS NULL)) OR \
                 (action <> {} AND (to_class IS NOT NULL OR to_class_node_id IS NOT NULL \
                   OR to_class_fact_id IS NOT NULL))",
                crate::codebook::ModelExceptionAction::Convert.code(),
                crate::codebook::ModelExceptionAction::Convert.code()
            ),
        ),
        (
            "semantic:model-exception-class-identity",
            format!(
                "SELECT e.rule_id FROM model_exceptions e \
                 LEFT JOIN context_definitions d ON d.symbol_node_id = e.class_node_id \
                   AND d.fact_id = e.class_fact_id AND d.kind = {class_kind} \
                   AND concat(d.module_name, '.', d.qualified_name) = e.class \
                 LEFT JOIN context_definitions t ON t.symbol_node_id = e.to_class_node_id \
                   AND t.fact_id = e.to_class_fact_id AND t.kind = {class_kind} \
                   AND concat(t.module_name, '.', t.qualified_name) = e.to_class \
                 WHERE d.fact_id IS NULL OR (e.to_class IS NOT NULL AND t.fact_id IS NULL)",
                class_kind = crate::codebook::DefinitionKind::Class.code(),
            ),
        ),
        (
            "semantic:modeled-exception-site-shape",
            format!(
                "SELECT rule_id FROM modeled_exception_sites WHERE \
                 (candidate_set_complete_under_model AND has_unresolved_remainder) \
                 OR origin <> {}",
                crate::codebook::Origin::SyntheticModel.code(),
            ),
        ),
        (
            "semantic:handler-type-status-shape",
            format!(
                "SELECT handler_node_id FROM handler_types WHERE \
                 (status = {bare} AND (type_node_id IS NOT NULL OR class_node_id IS NOT NULL \
                   OR reason IS NOT NULL)) \
                 OR (status = {pinned} AND (type_node_id IS NULL OR type_fact_id IS NULL \
                   OR reference_fact_id IS NULL OR resolution_fact_id IS NULL \
                   OR class_node_id IS NULL OR class_fact_id IS NULL \
                   OR class_module_fact_id IS NULL OR class_name IS NULL OR reason IS NOT NULL)) \
                 OR (status = {unknown} AND (type_node_id IS NULL OR class_node_id IS NOT NULL \
                   OR class_fact_id IS NOT NULL OR class_name IS NOT NULL OR reason IS NULL))",
                bare = crate::codebook::HandlerTypeStatus::Bare.code(),
                pinned = crate::codebook::HandlerTypeStatus::PinnedBuiltin.code(),
                unknown = crate::codebook::HandlerTypeStatus::Unknown.code(),
            ),
        ),
        (
            // F2: every Pysa function with signatures is some signature row's callable.
            "semantic:pysa-signatures-placed",
            "SELECT m.module_node_id, m.function_key FROM provider_node_map m              JOIN pysa_functions f                ON f.module_node_id = m.module_node_id AND f.function_key = m.function_key              LEFT ANTI JOIN signatures s                ON s.module_node_id = m.module_node_id AND s.function_key = m.function_key              WHERE m.node_id IS NOT NULL AND f.signature_count > 0"
                .to_owned(),
        ),
        (
            // F3: the derived "no Pysa record" reason and the extractor's call boundary agree, per
            // call site, in both directions.
            "semantic:resolution-has-boundary",
            format!(
                "SELECT r.call_site_node_id FROM resolutions r LEFT ANTI JOIN boundaries b                    ON b.subject_node_id = r.call_site_node_id AND b.fact_family = {calls}                   AND b.reason = r.reason                  WHERE r.reason IS NOT NULL"
            ),
        ),
        (
            "semantic:boundary-has-resolution",
            format!(
                "SELECT b.subject_node_id FROM boundaries b LEFT ANTI JOIN resolutions r                    ON r.call_site_node_id = b.subject_node_id AND r.reason = b.reason                  WHERE b.fact_family = {calls} AND b.subject_node_id IS NOT NULL"
            ),
        ),
        (
            // O4: Stage C maps at most one Pysa key to a declaration.
            "semantic:stage-c-injective",
            "SELECT node_id, count(*) AS n FROM provider_node_map WHERE node_id IS NOT NULL              GROUP BY node_id HAVING count(*) > 1"
                .to_owned(),
        ),
        (
            // O2: a provider-local composite reference.
            "semantic:parameter-semantics-function",
            "SELECT q.module_node_id, q.function_key FROM parameter_semantics q              LEFT ANTI JOIN pysa_functions f                ON f.module_node_id = q.module_node_id AND f.function_key = q.function_key"
                .to_owned(),
        ),
        (
            // O2: `model_id` is `<producer_id hex>/<surface>` of the fact's own run (§3.5).
            "semantic:model-id-producer",
            "SELECT f.fact_id FROM facts f JOIN runs r ON r.run_id = f.run_id              WHERE split_part(f.model_id, '/', 1) <> encode(r.producer_id, 'hex')"
                .to_owned(),
        ),
        (
            // ADR-0019: an analysis invocation's model is its own run's producer, as for facts.
            "semantic:invocation-model-producer",
            "SELECT i.invocation_id FROM analysis_invocations i JOIN runs r ON r.run_id = i.run_id \
             WHERE split_part(i.model_id, '/', 1) <> encode(r.producer_id, 'hex')"
                .to_owned(),
        ),
        (
            // ADR-0019 review F6: a finding's status is the one its kind permits.
            "semantic:finding-status-policy",
            format!(
                "SELECT f.finding_id FROM findings f \
                 LEFT ANTI JOIN (VALUES {}) AS p(kind, status) \
                   ON p.kind = f.finding_kind AND p.status = f.evidence_status",
                crate::findings::FINDING_STATUS
                    .iter()
                    .map(|(k, s)| format!("({}, {})", k.code(), s.code()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
        (
            // The ADR-0011 review's deferred row (fired by the increment-2 review): a finding's
            // invocation ran the method its kind comes from.
            "semantic:finding-kind-by-method",
            format!(
                "SELECT f.finding_id FROM findings f \
                 JOIN analysis_invocations i ON i.invocation_id = f.invocation_id \
                 LEFT ANTI JOIN (VALUES {}) AS p(kind, method) \
                   ON p.kind = f.finding_kind AND p.method = i.method",
                crate::findings::FINDING_METHOD
                    .iter()
                    .map(|(k, m)| format!("({}, {})", k.code(), m.code()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
        (
            // The increment-2 review's F2: no concept or implication is about a type the analysis
            // could not determine (`Unknown`). A tripwire over labels: the attribute relation
            // drops such terms by their structure.
            "semantic:concept-attribute-known",
            format!(
                "SELECT m.finding_id, m.label FROM finding_members m \
                 WHERE m.role IN ({intent}, {premise}, {conclusion}) \
                   AND m.label ~ '\\bUnknown\\b'",
                intent = crate::codebook::MemberRole::IntentAttribute.code(),
                premise = crate::codebook::MemberRole::Premise.code(),
                conclusion = crate::codebook::MemberRole::Conclusion.code(),
            ),
        ),
        (
            // ADR-0019 review F6: every analysis invocation is the compiler's.
            "semantic:invocation-run-is-compiler",
            "SELECT i.invocation_id FROM analysis_invocations i \
             JOIN runs r ON r.run_id = i.run_id \
             JOIN producers p ON p.producer_id = r.producer_id \
             WHERE p.tool <> 'lctx-compiler'"
                .to_owned(),
        ),
        (
            // §11.1, slice 1.7: a document's cache key is whole, both parts or neither.
            "semantic:document-key-whole",
            "SELECT brief_id, chunk FROM brief_documents \
             WHERE (spec_hash IS NULL) <> (input_hash IS NULL)"
                .to_owned(),
        ),
        (
            // §6.4: every embedded document's vector is in the cache the snapshot records, so the
            // serving bundle can be built from the store alone.
            "semantic:document-vector-cached",
            "SELECT d.brief_id, d.chunk FROM brief_documents d \
             LEFT ANTI JOIN embedding_cache c \
               ON c.spec_hash = d.spec_hash AND c.input_hash = d.input_hash \
             WHERE d.input_hash IS NOT NULL"
                .to_owned(),
        ),
        (
            // §6.4: a snapshot's documents share one spec; a generation never mixes vector spaces.
            "semantic:one-embedding-spec",
            "SELECT count(*) AS specs FROM embedding_specs HAVING count(*) > 1".to_owned(),
        ),
        (
            // Increment-1 deep review O7 (fired by 2.1): a `documented` parameter cites its own
            // parameter's description, not any documentation of its operation, nor another
            // parameter's (slice 2.1 review F3: keyed by the parameter the evidence names). A
            // `<ParamField>` description (A3; R1 F1) cites the lead of a field with no
            // `<ParamField>` ancestor whose literal name is the parameter's, in a passage the
            // assertion's scope anchor ties to its brief's seed.
            "semantic:documented-parameter-cites-its-doc",
            format!(
                "WITH anchored AS ({anchored}) \
                 SELECT a.assertion_id FROM assertions a LEFT ANTI JOIN ( \
                   SELECT s.assertion_id FROM assertion_support s \
                   JOIN evidence e ON e.evidence_id = s.evidence_id \
                   JOIN parameter_syntax ps ON ps.node_id = e.node_id \
                   JOIN parameter_docs d ON d.function_node_id = ps.function_node_id \
                     AND d.name = ps.name AND d.module_node_id = e.module_node_id \
                     AND d.start_byte = e.start_byte AND d.end_byte = e.end_byte \
                   WHERE e.evidence_kind = {span} \
                   UNION \
                   SELECT s.assertion_id FROM assertion_support s \
                   JOIN evidence e ON e.evidence_id = s.evidence_id AND e.evidence_kind = {passage} \
                   JOIN doc_components c ON c.passage_node_id = e.node_id \
                     AND c.lead_start = e.start_byte AND c.lead_end = e.end_byte \
                     AND c.name = '{param_field}' \
                   JOIN doc_component_attributes at ON at.document_node_id = c.document_node_id \
                     AND at.component_ordinal = c.ordinal AND at.name = '{param_name}' \
                     AND at.value_kind = {literal} \
                   JOIN anchored n ON n.assertion_id = s.assertion_id \
                     AND n.passage_node_id = e.node_id \
                   JOIN assertion_support s2 ON s2.assertion_id = s.assertion_id \
                   JOIN evidence e2 ON e2.evidence_id = s2.evidence_id \
                   JOIN parameter_syntax ps ON ps.node_id = e2.node_id AND ps.name = at.value \
                   LEFT ANTI JOIN doc_components f ON f.document_node_id = c.document_node_id \
                     AND f.name = '{param_field}' AND f.depth < c.depth \
                     AND f.start_byte <= c.start_byte AND c.end_byte <= f.end_byte \
                 ) own ON own.assertion_id = a.assertion_id \
                 WHERE a.assertion_kind = {parameter} AND a.evidence_status = {documented}",
                anchored = anchored_sql(),
                span = crate::codebook::EvidenceKind::Span.code(),
                passage = crate::codebook::EvidenceKind::Passage.code(),
                literal = crate::codebook::AttributeValueKind::Literal.code(),
                param_field = crate::mdx::PARAM_FIELD,
                param_name = crate::mdx::PARAM_NAME,
                parameter = AssertionKind::Parameter.code(),
                documented = EvidenceStatus::Documented.code()
            ),
        ),
        (
            // Slice 3.4 and A3 (R1 F1): a documented warning quotes the inner bytes of a
            // `<Warning>` component of a passage its scope anchor ties to its brief's seed; inside
            // a `<ParamField>`, the nearest such field names a parameter of the seed (not its
            // receiver).
            "semantic:documented-warning-anchored",
            format!(
                "WITH anchored AS ({anchored}), \
                 receivers AS ({receivers}), \
                 warned AS ( \
                   SELECT a.assertion_id, b.seed_node_id, c.document_node_id, c.depth, \
                          c.start_byte, c.end_byte \
                   FROM assertions a \
                   JOIN assertion_support s ON s.assertion_id = a.assertion_id \
                     AND s.role = {support} \
                   JOIN evidence e ON e.evidence_id = s.evidence_id \
                     AND e.evidence_kind = {passage} \
                   JOIN doc_components c ON c.passage_node_id = e.node_id \
                     AND c.name = '{warning}' \
                     AND c.inner_start = e.start_byte AND c.inner_end = e.end_byte \
                   JOIN anchored n ON n.assertion_id = a.assertion_id \
                     AND n.passage_node_id = e.node_id \
                   JOIN brief_assertions ba ON ba.assertion_id = a.assertion_id \
                   JOIN briefs b ON b.brief_id = ba.brief_id \
                   WHERE a.assertion_kind = {warning_kind}), \
                 fields AS ( \
                   SELECT w.assertion_id, f.depth, v.value AS body FROM warned w \
                   JOIN doc_components f ON f.document_node_id = w.document_node_id \
                     AND f.name = '{param_field}' AND f.depth < w.depth \
                     AND f.start_byte <= w.start_byte AND w.end_byte <= f.end_byte \
                   LEFT JOIN doc_component_attributes v \
                     ON v.document_node_id = f.document_node_id \
                     AND v.component_ordinal = f.ordinal AND v.name = '{param_name}' \
                     AND v.value_kind = {literal}), \
                 nearest AS ( \
                   SELECT assertion_id, body FROM ( \
                     SELECT *, row_number() OVER (PARTITION BY assertion_id ORDER BY depth DESC) \
                       AS r FROM fields) WHERE r = 1), \
                 params AS ( \
                   SELECT p.signature_node_id, ps.name FROM parameters p \
                   JOIN parameter_syntax ps ON ps.fact_id = p.syntax_fact_id \
                   LEFT ANTI JOIN receivers r ON r.parameter_node_id = ps.node_id), \
                 ok AS ( \
                   SELECT w.assertion_id FROM warned w \
                   LEFT JOIN nearest n ON n.assertion_id = w.assertion_id \
                   LEFT JOIN params q ON q.signature_node_id = w.seed_node_id AND q.name = n.body \
                   WHERE n.assertion_id IS NULL OR q.name IS NOT NULL) \
                 SELECT a.assertion_id FROM assertions a \
                 LEFT ANTI JOIN ok ON ok.assertion_id = a.assertion_id \
                 WHERE a.assertion_kind = {warning_kind}",
                anchored = anchored_sql(),
                receivers = crate::flows::receivers_sql(),
                support = SupportRole::Support.code(),
                passage = crate::codebook::EvidenceKind::Passage.code(),
                literal = crate::codebook::AttributeValueKind::Literal.code(),
                warning = crate::mdx::WARNING,
                param_field = crate::mdx::PARAM_FIELD,
                param_name = crate::mdx::PARAM_NAME,
                warning_kind = AssertionKind::DocumentedWarning.code(),
            ),
        ),
        (
            // The holistic assessment's A3: a component's parent is in its document, earlier in
            // pre-order, one level up, and contains it; a top-level component is at depth 0.
            "semantic:doc-component-parent",
            "SELECT c.document_node_id, c.ordinal FROM doc_components c \
             LEFT ANTI JOIN doc_components p ON p.document_node_id = c.document_node_id \
               AND p.ordinal = c.parent_ordinal AND p.depth = c.depth - 1 \
               AND p.start_byte <= c.start_byte AND c.end_byte <= p.end_byte \
             WHERE c.parent_ordinal IS NOT NULL \
             UNION ALL \
             SELECT document_node_id, ordinal FROM doc_components \
             WHERE parent_ordinal IS NULL AND depth <> 0"
                .to_owned(),
        ),
        (
            // A3: a component lies inside its passage, which is in its document.
            "semantic:doc-component-in-passage",
            "SELECT c.document_node_id, c.ordinal FROM doc_components c \
             LEFT ANTI JOIN passages p ON p.node_id = c.passage_node_id \
               AND p.document_node_id = c.document_node_id \
               AND p.start_byte <= c.start_byte AND c.end_byte <= p.end_byte"
                .to_owned(),
        ),
        (
            // A3: an attribute belongs to a component, and its kind says which parts it has: no
            // name exactly for a spread, no value exactly for a bare name.
            "semantic:doc-attribute-component",
            format!(
                "SELECT a.document_node_id, a.component_ordinal, a.ordinal \
                 FROM doc_component_attributes a LEFT ANTI JOIN doc_components c \
                   ON c.document_node_id = a.document_node_id AND c.ordinal = a.component_ordinal \
                 UNION ALL \
                 SELECT document_node_id, component_ordinal, ordinal FROM doc_component_attributes \
                 WHERE (name IS NULL) <> (value_kind = {spread}) \
                    OR (value IS NULL) <> (value_kind = {bare})",
                spread = crate::codebook::AttributeValueKind::Spread.code(),
                bare = crate::codebook::AttributeValueKind::Bare.code(),
            ),
        ),
        (
            // A3 (the missing docs span rule): every docs span lies within its document's bytes.
            "semantic:docs-span-in-document",
            "SELECT 'passages' AS t, x.start_byte FROM passages x \
               JOIN documents d ON d.node_id = x.document_node_id WHERE x.end_byte > d.byte_len \
             UNION ALL SELECT 'code_blocks', x.start_byte FROM code_blocks x \
               JOIN documents d ON d.node_id = x.document_node_id WHERE x.end_byte > d.byte_len \
             UNION ALL SELECT 'doc_components', x.start_byte FROM doc_components x \
               JOIN documents d ON d.node_id = x.document_node_id WHERE x.end_byte > d.byte_len \
             UNION ALL SELECT 'doc_links', x.start_byte FROM doc_links x \
               JOIN passages p ON p.node_id = x.passage_node_id \
               JOIN documents d ON d.node_id = p.document_node_id WHERE x.end_byte > d.byte_len \
             UNION ALL SELECT 'mentions', x.start_byte FROM mentions x \
               JOIN passages p ON p.node_id = x.passage_node_id \
               JOIN documents d ON d.node_id = p.document_node_id WHERE x.end_byte > d.byte_len"
                .to_owned(),
        ),
        (
            // Slice 2.2 review F3: a usage pattern cites only a handoff it shows, one whose
            // consumer site lies inside one of its verbatim example statements.
            "semantic:usage-pattern-shows-its-handoff",
            format!(
                "SELECT a.assertion_id, s.finding_id FROM assertions a \
                 JOIN assertion_support s ON s.assertion_id = a.assertion_id \
                 JOIN findings f ON f.finding_id = s.finding_id AND f.finding_kind = {handoff} \
                 LEFT ANTI JOIN ( \
                   SELECT s2.assertion_id, s2.finding_id FROM assertion_support s2 \
                   JOIN finding_members m ON m.finding_id = s2.finding_id \
                     AND m.role = {consumer_site} \
                   JOIN call_syntax cs ON cs.node_id = m.node_id \
                   JOIN assertion_support se ON se.assertion_id = s2.assertion_id \
                   JOIN evidence e ON e.evidence_id = se.evidence_id \
                     AND e.evidence_kind = {example} AND e.module_node_id = cs.module_node_id \
                     AND e.start_byte <= cs.start_byte AND cs.end_byte <= e.end_byte) shown \
                   ON shown.assertion_id = a.assertion_id AND shown.finding_id = s.finding_id \
                 WHERE a.assertion_kind = {usage_pattern}",
                handoff = crate::codebook::FindingKind::Handoff.code(),
                consumer_site = crate::codebook::MemberRole::ConsumerSite.code(),
                example = crate::codebook::EvidenceKind::Example.code(),
                usage_pattern = AssertionKind::UsagePattern.code(),
            ),
        ),
        (
            // Slice 2.2 review F5: a published text names a doc block by its document, never by
            // the module the compiler materialized it as.
            "semantic:no-materialized-block-path",
            "SELECT assertion_id FROM assertions WHERE strpos(text, '_lctx_blocks/') > 0"
                .to_owned(),
        ),
        (
            // §11.1: one spec gives one vector length (its declared dimensions).
            "semantic:embedding-dimensions",
            "SELECT spec_hash FROM embedding_cache GROUP BY spec_hash \
             HAVING count(DISTINCT cardinality(vector)) > 1"
                .to_owned(),
        ),
        (
            // §10.2: an assertion's status is one its kind permits, in the published policy.
            "semantic:assertion-policy",
            "SELECT a.assertion_id FROM assertions a LEFT ANTI JOIN assertion_policy p \
               ON p.assertion_kind = a.assertion_kind AND p.evidence_status = a.evidence_status"
                .to_owned(),
        ),
        (
            // The published policy is the code's, both ways.
            "semantic:assertion-policy-published",
            {
                let values = crate::findings::ASSERTION_POLICY
                    .iter()
                    .flat_map(|(k, s, statuses)| {
                        statuses
                            .iter()
                            .map(move |st| format!("({}, {}, {})", k.code(), s.code(), st.code()))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "WITH v AS (SELECT * FROM (VALUES {values}) AS v(k, s, st)) \
                     SELECT p.assertion_kind FROM assertion_policy p LEFT ANTI JOIN v \
                       ON v.k = p.assertion_kind AND v.s = p.brief_section \
                      AND v.st = p.evidence_status \
                     UNION ALL \
                     SELECT v.k FROM v LEFT ANTI JOIN assertion_policy p \
                       ON v.k = p.assertion_kind AND v.s = p.brief_section \
                      AND v.st = p.evidence_status"
                )
            },
        ),
        (
            // §10.2, slice 1.5 review F1: an assertion's status is `findings::derive_status` of
            // its supports, recomputed here and compared for equality: a floor and a ceiling.
            "semantic:assertion-status-derived",
            {
                let evidence = crate::findings::EVIDENCE_STATUS
                    .iter()
                    .map(|(k, st)| format!("WHEN {} THEN {}", k.code(), st.code()))
                    .collect::<Vec<_>>()
                    .join(" ");
                let rank = crate::findings::STATUS_STRENGTH
                    .iter()
                    .enumerate()
                    .map(|(i, st)| format!("WHEN {} THEN {i}", st.code()))
                    .collect::<Vec<_>>()
                    .join(" ");
                let unrank = crate::findings::STATUS_STRENGTH
                    .iter()
                    .enumerate()
                    .map(|(i, st)| format!("WHEN {i} THEN {}", st.code()))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(
                    "WITH x AS ( \
                       SELECT s.assertion_id, s.role, f.evidence_status AS st \
                       FROM assertion_support s JOIN findings f ON f.finding_id = s.finding_id \
                       UNION ALL \
                       SELECT s.assertion_id, s.role, CASE e.evidence_kind {evidence} END AS st \
                       FROM assertion_support s JOIN evidence e ON e.evidence_id = s.evidence_id), \
                     d AS ( \
                       SELECT a.assertion_id, a.evidence_status, CASE \
                         WHEN count(CASE WHEN x.role = {support} THEN 1 END) = 0 THEN {unres} \
                         WHEN bool_or(x.role = {support} AND x.st = {unres}) THEN {unres} \
                         WHEN bool_or(x.st = {stat}) THEN {stat} \
                         ELSE CASE max(CASE WHEN x.role = {support} THEN CASE x.st {rank} END END) \
                              {unrank} END END AS derived \
                       FROM assertions a LEFT JOIN x ON x.assertion_id = a.assertion_id \
                       GROUP BY a.assertion_id, a.evidence_status) \
                     SELECT assertion_id FROM d WHERE derived IS NULL OR derived <> evidence_status",
                    support = SupportRole::Support.code(),
                    stat = EvidenceStatus::StatisticallyDerived.code(),
                    unres = EvidenceStatus::Unresolved.code(),
                )
            },
        ),
        (
            // §10.2: an unresolved slot has no text, and only an unresolved slot has none.
            "semantic:assertion-text-iff-resolved",
            format!(
                "SELECT assertion_id FROM assertions \
                 WHERE (text IS NULL) <> (evidence_status = {})",
                EvidenceStatus::Unresolved.code()
            ),
        ),
        (
            // A support row cites exactly one finding or one evidence row.
            "semantic:support-cites-one",
            "SELECT assertion_id FROM assertion_support \
             WHERE (finding_id IS NULL) = (evidence_id IS NULL)"
                .to_owned(),
        ),
        (
            // ADR-0019 review F8: resolved text is exactly its span's bytes.
            "semantic:evidence-text-bytes",
            "SELECT evidence_id FROM evidence \
             WHERE text IS NOT NULL AND start_byte IS NOT NULL AND end_byte IS NOT NULL \
               AND octet_length(text) <> end_byte - start_byte"
                .to_owned(),
        ),
        (
            // The holistic assessment's A1: a public path is its export's path, or that path and
            // the node's own name, and names the node's kind.
            "semantic:public-path-exported",
            "SELECT p.node_id, p.access_path FROM public_paths p \
             LEFT ANTI JOIN (SELECT q.node_id, q.access_path FROM public_paths q \
               JOIN declarations d ON d.node_id = q.node_id AND d.kind = q.kind \
               JOIN exports e ON e.export_node_id = q.export_node_id \
                 AND (q.access_path = e.access_path \
                      OR q.access_path = e.access_path || '.' || d.name)) ok \
               ON ok.node_id = p.node_id AND ok.access_path = p.access_path"
                .to_owned(),
        ),
        (
            // The behavioral-model plan, Stage 1 (ADR-0021): every public node is an operation,
            // and its behavior scan ran, or it says why not. Only a class is `not_analyzed`: its
            // controls are its `__init__`'s.
            "semantic:behavior-covers-public",
            format!(
                "SELECT p.node_id FROM (SELECT DISTINCT node_id FROM public_paths) p \
                 LEFT JOIN operations o ON o.node_id = p.node_id \
                 WHERE o.node_id IS NULL \
                    OR (o.kind = {class}) <> (o.behavior_status = {not_analyzed})",
                class = DeclarationKind::Class.code(),
                not_analyzed = Verdict::NotAnalyzed.code(),
            ),
        ),
        (
            // Stage 1: what a behavior or a facet describes is an operation.
            "semantic:behavior-of-an-operation",
            "SELECT b.operation_node_id AS node_id FROM behaviors b \
             LEFT ANTI JOIN operations o ON o.node_id = b.operation_node_id \
             UNION ALL \
             SELECT f.node_id FROM operation_facets f \
             LEFT ANTI JOIN operations o ON o.node_id = f.node_id"
                .to_owned(),
        ),
        (
            // Increment 3's deep review, F1: a behavior whose path crosses a non-definite arc
            // (override dispatch, a potential call) is never established or conditional; the
            // delegation over the same arc is unknown, and so is the behavior.
            "semantic:established-needs-definite-path",
            format!(
                "SELECT DISTINCT b.behavior_id FROM behaviors b \
                 JOIN behavior_steps s ON s.behavior_id = b.behavior_id \
                 WHERE b.verdict IN ({established}, {conditional}) AND s.modality <> {definite}",
                established = Verdict::Established.code(),
                conditional = Verdict::Conditional.code(),
                definite = crate::codebook::Modality::Definite.code(),
            ),
        ),
        (
            // Increment 3's deep review, F3: every operation says, for every facet, whether its
            // rows are complete; absence is never read as "complete".
            "semantic:facet-status-covers-operations",
            format!(
                "WITH facets AS (SELECT * FROM (VALUES {facets}) AS f(facet)) \
                 SELECT o.node_id FROM operations o CROSS JOIN facets f \
                 LEFT ANTI JOIN operation_facet_status s \
                   ON s.node_id = o.node_id AND s.facet = f.facet",
                facets = <crate::codebook::OperationFacet as Codebook>::all()
                    .iter()
                    .map(|f| format!("({})", f.code()))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ),
        (
            // ADR-0022 §Verdicts: a refutation holds only where its place's premise holds
            // (`negative_premises`); any other `refuted_under_model` row is rejected.
            "semantic:refuted-needs-complete-region",
            format!(
                "SELECT b.behavior_id FROM behaviors b \
                 LEFT JOIN negative_premises p ON p.place_key = b.premise_key \
                 WHERE b.verdict = {refuted} AND (p.place_key IS NULL OR NOT p.holds)",
                refuted = Verdict::RefutedUnderModel.code(),
            ),
        ),
        (
            // ADR-0022 §Places (the Stage 2 end review's R4): a feasible flow is never stated under
            // `false`; one that is has two values read as one atom.
            "semantic:condition-not-false",
            "SELECT 1 AS violation FROM behaviors WHERE condition = 'false' \
             UNION ALL SELECT 1 AS violation FROM value_flows WHERE condition = 'false'"
                .to_owned(),
        ),
        (
            // ADR-0022 §Verdicts (the Stage 2 end review's R7): a field's or a setting's premise
            // holds only if no attribute load of that name exists, on any receiver.
            "semantic:premise-no-attribute-load",
            format!(
                "SELECT p.place_key FROM negative_premises p \
                 JOIN flow_attribute_loads a \
                   ON a.name = regexp_replace(p.place_key, '^.*\\.([^.\\]]*)\\]?$', '\\1') \
                 WHERE p.holds AND p.kind IN ({field}, {global})",
                field = PremiseKind::Field.code(),
                global = PremiseKind::Global.code(),
            ),
        ),
        (
            // ADR-0022 §Verdicts (the Stage 2 end review's R1): "never read" is not refuted on a
            // method a release subclass defines again; the override may read it.
            "semantic:refuted-not-overridden",
            format!(
                "SELECT b.behavior_id FROM behaviors b \
                 JOIN declarations m ON m.node_id = b.operation_node_id \
                 JOIN edges e ON e.edge_kind = {mro} AND e.dst_node_id = m.parent_node_id \
                   AND e.src_node_id <> m.parent_node_id \
                 JOIN declarations o ON o.parent_node_id = e.src_node_id AND o.name = m.name \
                 WHERE b.kind = {is_read} AND b.verdict = {refuted}",
                mro = EdgeKind::MroEntry.code(),
                is_read = BehaviorKind::IsRead.code(),
                refuted = Verdict::RefutedUnderModel.code(),
            ),
        ),
        (
            // ADR-0022 §Composed layers (the Stage 2 end review's R2): an operation whose
            // declaration lies in a region the runtime view never reaches is not established.
            "semantic:unreachable-not-established",
            format!(
                "SELECT o.node_id FROM operations o JOIN declarations d ON d.node_id = o.node_id \
                 JOIN flow_regions r ON r.module_node_id = d.module_node_id \
                   AND r.start_byte <= d.name_start_byte AND d.name_end_byte <= r.end_byte \
                 JOIN conditions c ON c.condition_id = r.condition_id \
                 WHERE o.behavior_status = {established} AND c.encoding = 'false'",
                established = Verdict::Established.code(),
            ),
        ),
        (
            // R2 F3: one path names one node. Seed resolution, the gold matcher and promotion
            // all read a path as naming exactly one node (a module `pkg/C.py`'s `f` and a class
            // `pkg.C`'s member `f` would both spell `pkg.C.f`).
            "semantic:public-path-one-node",
            "SELECT access_path FROM public_paths GROUP BY access_path \
             HAVING count(DISTINCT node_id) > 1"
                .to_owned(),
        ),
        (
            // A1: one preferred path per node, the name every consumer shows it by.
            "semantic:public-path-preferred",
            "SELECT node_id FROM public_paths GROUP BY node_id \
             HAVING sum(CASE WHEN preferred THEN 1 ELSE 0 END) <> 1"
                .to_owned(),
        ),
        (
            // A1: `own` exactly when the path's export declares the node (it is the node, or the
            // class that defines it).
            "semantic:public-path-own",
            "SELECT p.node_id, p.access_path FROM public_paths p \
             JOIN declarations d ON d.node_id = p.node_id \
             JOIN (SELECT DISTINCT export_node_id, declaration_node_id FROM exports) e \
               ON e.export_node_id = p.export_node_id \
             WHERE p.own <> COALESCE(e.declaration_node_id = p.node_id \
                                     OR e.declaration_node_id = d.parent_node_id, false)"
                .to_owned(),
        ),
        (
            // §10.4 (slice 1.5 review O3; the holistic assessment's A1): a brief's members are
            // exactly its seed's public paths, each with the path's export and `own` flag, and
            // `semantic:public-path-exported` holds each of those to an export.
            "semantic:brief-member-public",
            "SELECT m.brief_id, m.access_path FROM brief_members m \
             JOIN briefs b ON b.brief_id = m.brief_id \
             LEFT ANTI JOIN public_paths p ON p.node_id = b.seed_node_id \
               AND p.node_id = m.declaration_node_id AND p.access_path = m.access_path \
               AND p.export_node_id = m.export_node_id AND p.own = m.own \
             UNION ALL \
             SELECT b.brief_id, p.access_path FROM briefs b \
             JOIN public_paths p ON p.node_id = b.seed_node_id \
             LEFT ANTI JOIN brief_members m ON m.brief_id = b.brief_id \
               AND m.access_path = p.access_path"
                .to_owned(),
        ),
        (
            // §1.5: no brief reaches publication by bypassing the analytics. A brief is labelled
            // documentation-only exactly when it cites no analysis-backed finding
            // (`findings::ANALYSIS_BACKED`; slice 1.5 review F4), both ways.
            "semantic:brief-cites-analysis",
            format!(
                "SELECT b.brief_id FROM briefs b LEFT JOIN ( \
                   SELECT DISTINCT ba.brief_id FROM brief_assertions ba \
                   JOIN assertion_support s ON s.assertion_id = ba.assertion_id \
                   JOIN findings f ON f.finding_id = s.finding_id \
                   WHERE f.finding_kind IN ({})) x ON x.brief_id = b.brief_id \
                 WHERE b.documentation_only = (x.brief_id IS NOT NULL)",
                crate::findings::ANALYSIS_BACKED
                    .iter()
                    .map(|k| k.code().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
        (
            // §1.5 (slice 1.5 review F4): a documentation-only brief has a documented Outcome;
            // with neither analysis nor documentation it says nothing.
            "semantic:documentation-only-has-outcome",
            format!(
                "SELECT b.brief_id FROM briefs b LEFT ANTI JOIN ( \
                   SELECT ba.brief_id FROM brief_assertions ba \
                   JOIN assertions a ON a.assertion_id = ba.assertion_id \
                   WHERE a.assertion_kind = {} AND a.evidence_status = {}) o \
                   ON o.brief_id = b.brief_id \
                 WHERE b.documentation_only",
                AssertionKind::Outcome.code(),
                EvidenceStatus::Documented.code()
            ),
        ),
        (
            // Slice 1.4 review F3: a direct delegation's first witness is one definite call.
            "semantic:direct-delegation-is-definite",
            format!(
                "SELECT f.finding_id FROM findings f \
                 JOIN witnesses w ON w.finding_id = f.finding_id AND w.path = 0 \
                 WHERE f.finding_kind = {direct} \
                   AND (w.modality <> {definite} OR w.arc_kind <> {call} OR w.step > 0 \
                        OR f.depth <> 1)",
                direct = crate::codebook::FindingKind::DirectDelegation.code(),
                definite = crate::codebook::Modality::Definite.code(),
                call = crate::codebook::ArcKind::Call.code()
            ),
        ),
        (
            // Slice 1.4 review O6: each witness step is the edge it cites: the edge ends at the
            // callee and starts at the step's site (a call) or its caller (a definition).
            "semantic:witness-edge",
            "SELECT w.finding_id, w.path, w.step FROM witnesses w \
             JOIN edges e ON e.edge_id = w.edge_id \
             WHERE e.dst_node_id <> w.callee_node_id \
                OR (e.src_node_id <> w.call_site_node_id AND e.src_node_id <> w.caller_node_id)"
                .to_owned(),
        ),
        (
            // ADR-0019: a witness path is a chain from the finding's subject: each step starts
            // where the previous one ended, and the first at the subject.
            "semantic:witness-chain",
            "SELECT w.finding_id, w.path, w.step FROM witnesses w \
             JOIN findings f ON f.finding_id = w.finding_id \
             LEFT JOIN witnesses p ON p.finding_id = w.finding_id AND p.path = w.path \
               AND p.step = w.step - 1 \
             WHERE (w.step = 0 AND w.caller_node_id <> f.subject_node_id) \
                OR (w.step > 0 AND (p.callee_node_id IS NULL \
                    OR p.callee_node_id <> w.caller_node_id))"
                .to_owned(),
        ),
        (
            // ADR-0015: a module's text is present exactly when its bytes are UTF-8, and it is
            // those bytes (their length; the digest is BLAKE3, which SQL does not compute).
            "semantic:source-text",
            "SELECT fact_id FROM source_files \
             WHERE (text IS NULL) = utf8 OR octet_length(text) <> byte_len"
                .to_owned(),
        ),
        (
            // ADR-0015: the release's modules are those of a run that declares `exports` (the
            // library, or a source tree); a corpus run's modules are its examples, tests and
            // doc blocks. Only extractor runs declare families: the `lctx-compiler` run is over
            // the library release too and declares none (ADR-0019).
            "semantic:source-role-by-run",
            format!(
                "SELECT f.fact_id FROM source_files f JOIN runs r ON r.release_id = f.release_id \
                 WHERE cardinality(r.families) > 0 \
                   AND array_has(r.families, 'exports') <> (f.role = {release})",
                release = SourceRole::Release.code()
            ),
        ),
    ])
    .map(|(name, sql)| Rule {
        name: name.to_owned(),
        sql,
    })
    .collect()
}

/// Rules that cannot fail on today's SQL, named so no count or label reads them as falsifiable
/// (C3 review O2; C6 review F1; DESIGN §8). Each lineage guard re-reads its edge kind's own
/// unfiltered source, so it guards an edit of that edge's SQL rather than a data condition; the
/// gaps partition reads a table that is empty by construction since C3. No injected violation
/// can exercise them, and the rule meta-test accepts them for that reason alone.
pub const EDIT_GUARDS: &[&str] = &[
    "lineage:declares",
    "lineage:has_parameter",
    "lineage:encloses_call",
    "lineage:has_argument",
    "lineage:ast_child",
    "lineage:owns_scope",
    "lineage:lexical_parent",
    "lineage:binds",
    "lineage:reads_binding",
    "lineage:captures",
    "lineage:declared_in",
    "lineage:has_type",
    "lineage:type_arg",
    "lineage:has_field",
    "lineage:field_type",
    "lineage:contains_passage",
    "lineage:contains_block",
    "partition:pysa_calls-gaps",
    // Both sides are built from `ASSERTION_POLICY` (slice 1.5 review O2): it guards an edit that
    // publishes the policy from anywhere else.
    "semantic:assertion-policy-published",
];

/// Every rule, in a fixed order: keys, references, fact links, codebooks, coverage, semantic.
pub fn rules() -> Vec<Rule> {
    let shapes = shapes();
    let mut out = Vec::new();
    for s in &shapes {
        let key = s.key.join(", ");
        out.push(Rule {
            name: format!("key:{}", s.name),
            sql: format!(
                "SELECT {key}, count(*) AS n FROM {} GROUP BY {key} HAVING count(*) > 1",
                s.name
            ),
        });
    }
    for r in REFERENCES {
        let targets =
            r.to.iter()
                .map(|(t, c)| format!("SELECT {c} AS k FROM {t}"))
                .collect::<Vec<_>>()
                .join(" UNION ALL ");
        out.push(Rule {
            name: format!(
                "ref:{}.{}->{}",
                r.table,
                r.column,
                r.to.iter().map(|(t, _)| *t).collect::<Vec<_>>().join("|")
            ),
            sql: format!(
                "SELECT f.{c} AS value FROM {t} f LEFT ANTI JOIN ({targets}) r ON f.{c} = r.k \
                 WHERE f.{c} IS NOT NULL",
                t = r.table,
                c = r.column
            ),
        });
    }
    let fact_tables: Vec<&str> = shapes
        .iter()
        .filter(|s| s.name != "facts" && s.schema.index_of("fact_id").is_ok())
        .map(|s| s.name)
        .collect();
    for t in &fact_tables {
        out.push(Rule {
            name: format!("fact:{t}"),
            sql: format!(
                "SELECT t.fact_id FROM {t} t LEFT ANTI JOIN facts f \
                 ON f.fact_id = t.fact_id AND f.table_name = '{t}'"
            ),
        });
        out.push(Rule {
            name: format!("fact-payload:{t}"),
            sql: format!(
                "SELECT f.fact_id FROM facts f LEFT ANTI JOIN {t} t ON t.fact_id = f.fact_id \
                 WHERE f.table_name = '{t}'"
            ),
        });
    }
    out.push(Rule {
        name: "fact:table-name".to_owned(),
        sql: format!(
            "SELECT table_name FROM facts WHERE table_name NOT IN ({})",
            quoted(&fact_tables)
        ),
    });
    let books = registry();
    for s in &shapes {
        for f in s.schema.fields() {
            let Some(book) = f.metadata().get(CODEBOOK_KEY) else {
                continue;
            };
            let codes = books
                .iter()
                .find(|b| b.name == book)
                .map(|b| {
                    b.values
                        .iter()
                        .map(|(code, _)| code.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
                .join(", ");
            out.push(Rule {
                name: format!("codebook:{}.{}", s.name, f.name()),
                sql: format!(
                    "SELECT {c} FROM {t} WHERE {c} IS NOT NULL AND {c} NOT IN ({codes})",
                    t = s.name,
                    c = f.name()
                ),
            });
        }
    }
    // Scores and weights are finite (DESIGN §3.3, §8): one rule per `Float64` column, scalar or
    // list. SQL has no `isfinite`; NaN is `isnan`, and an infinity is the only value whose
    // absolute value equals the largest double's successor.
    for s in &shapes {
        for f in s.schema.fields() {
            let values = match f.data_type() {
                arrow_schema::DataType::Float64 | arrow_schema::DataType::Float32 => {
                    format!("SELECT {c} AS v FROM {t}", c = f.name(), t = s.name)
                }
                arrow_schema::DataType::List(item)
                    if matches!(
                        item.data_type(),
                        arrow_schema::DataType::Float64 | arrow_schema::DataType::Float32
                    ) =>
                {
                    format!("SELECT unnest({c}) AS v FROM {t}", c = f.name(), t = s.name)
                }
                _ => continue,
            };
            out.push(Rule {
                name: format!("finite:{}.{}", s.name, f.name()),
                sql: format!(
                    "SELECT v FROM ({values}) x \
                     WHERE v IS NOT NULL AND (isnan(v) OR abs(v) = CAST('Infinity' AS DOUBLE))"
                ),
            });
        }
    }
    let families = FactFamily::all()
        .iter()
        .map(|f| format!("('{}', {})", f.text(), f.code()))
        .collect::<Vec<_>>()
        .join(", ");
    out.push(Rule {
        name: "coverage:declared-family".to_owned(),
        sql: format!(
            "WITH declared AS (SELECT unnest(families) AS family FROM runs) \
             SELECT family FROM declared WHERE family NOT IN ({})",
            quoted(
                FactFamily::all()
                    .iter()
                    .filter(|f| f.is_coverage_unit())
                    .map(|f| f.text())
            )
        ),
    });
    out.push(Rule {
        name: "coverage:complete".to_owned(),
        sql: format!(
            "WITH declared AS (SELECT run_id, release_id, unnest(families) AS family FROM runs), \
             codes AS (SELECT * FROM (VALUES {families}) AS c(family, code)), \
             expected AS ( \
               SELECT d.run_id, s.module_node_id, c.code \
               FROM declared d JOIN codes c ON c.family = d.family \
               JOIN source_files s ON s.release_id = d.release_id \
               WHERE d.family <> 'docs' \
               UNION ALL \
               SELECT d.run_id, x.node_id AS module_node_id, c.code \
               FROM declared d JOIN codes c ON c.family = d.family \
               JOIN documents x ON x.release_id = d.release_id \
               WHERE d.family = 'docs') \
             SELECT e.run_id, e.module_node_id, e.code FROM expected e \
             LEFT ANTI JOIN coverage v \
               ON v.run_id = e.run_id AND v.scope_node_id = e.module_node_id \
              AND v.fact_family = e.code"
        ),
    });
    // A run that declares a family has something the family covers: a document for `docs`, a
    // module for a code family (C5 review F5), so coverage cannot be complete over nothing.
    let code = [
        FactFamily::Exports,
        FactFamily::Signatures,
        FactFamily::Calls,
        FactFamily::Syntax,
        FactFamily::Lexical,
        FactFamily::Types,
    ];
    out.push(Rule {
        name: "coverage:family-has-scope".to_owned(),
        sql: format!(
            "WITH declared AS (SELECT run_id, release_id, unnest(families) AS family FROM runs), \
             docs_runs AS (SELECT DISTINCT run_id, release_id FROM declared WHERE family = 'docs'), \
             code_runs AS (SELECT DISTINCT run_id, release_id FROM declared \
                           WHERE family IN ({})) \
             SELECT r.run_id, 'docs' AS family FROM docs_runs r \
             LEFT ANTI JOIN documents x ON x.release_id = r.release_id \
             UNION ALL \
             SELECT r.run_id, 'code' AS family FROM code_runs r \
             LEFT ANTI JOIN source_files s ON s.release_id = r.release_id",
            quoted(code.iter().map(|f| f.text()))
        ),
    });
    out.extend(semantic());
    out.extend(crate::graph::rules());
    out
}

/// Each assertion's scope anchors (R1 F1): a `scope`-cited Fact evidence row that is an exact
/// mention, in its passage, of the assertion's brief's seed (by its declaration, or an export
/// whose target it is), as `(assertion_id, passage_node_id)`.
fn anchored_sql() -> String {
    format!(
        "SELECT DISTINCT s.assertion_id, m.passage_node_id FROM assertion_support s \
         JOIN evidence e ON e.evidence_id = s.evidence_id AND e.evidence_kind = {fact} \
         JOIN mentions m ON m.fact_id = e.cited_fact_id AND m.class = {exact} \
           AND m.passage_node_id = e.node_id \
         JOIN mention_targets t ON t.mention_fact_id = m.fact_id \
         JOIN brief_assertions ba ON ba.assertion_id = s.assertion_id \
         JOIN briefs b ON b.brief_id = ba.brief_id \
         LEFT JOIN (SELECT DISTINCT export_node_id, target_node_id FROM exports) x \
           ON x.export_node_id = t.target_node_id \
         WHERE s.role = {scope} \
           AND (t.target_node_id = b.seed_node_id OR x.target_node_id = b.seed_node_id)",
        fact = crate::codebook::EvidenceKind::Fact.code(),
        exact = crate::codebook::MentionClass::Exact.code(),
        scope = SupportRole::Scope.code(),
    )
}
