//! Generation-pinned condition kernel and developer smoke probes.
use std::collections::{BTreeSet, HashMap, HashSet};

use cpg_schema::codebook::{BoundaryReason, Codebook, SummaryFlowStepKind, TestValueLinkOrigin};
use cpg_schema::condition::{Atom, Condition, Value};
use cpg_schema::condition_kernel::{
    ConditionRoot, Diagram, DiagramNode, KERNEL_FORMAT, KernelBoundary, MAX_CATALOG_CONDITIONS,
    MAX_CATALOG_NODES, MAX_CATALOG_RETAINED_NODES, hydrate_catalog,
};
use cpg_schema::id::{Digest, Id, IdHasher};
use cpg_schema::primitive_theory::{
    BuiltinNamespace, ExactInput, ExactInputOutcome, TestLeaf, TheoryBoundary, TheoryWork,
    ValueLink, assess_exact_input_with_work,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod ipc_input;

const MAX_SMOKE_INPUT_BYTES: usize = 64 * 1024;
const MAX_SUMMARY_ROWS: usize = 100_000;
const MAX_SURFACE_ROWS: usize = 200_000;
type FlowInput = (String, String, String, String, String, Option<String>, i64, String, String);
type ProofStep = (String, String, String);
type OpenPathBoundary = (String, String, String, String);
type BoundaryIndex = HashMap<(Id, Id), Vec<(Id, Id, Id, String)>>;
type LeafInput = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
    i64,
);
type LinkInput = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    (Option<String>, i64, i64),
);
type WorkAnswer = (usize, usize, usize, usize);
type RefutationAnswer = (
    String,
    Vec<(String, Option<String>, i64, i64)>,
    Option<String>,
    WorkAnswer,
);
type InspectedPath = (
    String,
    String,
    String,
    Vec<ProofStep>,
    String,
    Vec<(String, Option<String>, i64, i64)>,
    Option<String>,
    WorkAnswer,
    String,
    String,
);
type ValuePathPage = (Vec<InspectedPath>, Vec<OpenPathBoundary>, usize, bool, usize);

fn work_answer(work: TheoryWork) -> WorkAnswer {
    (
        work.links_examined,
        work.assignments_applied,
        work.bdd_preflight_pairs,
        work.peak_bdd_nodes,
    )
}

#[pyfunction]
fn kernel_format() -> u32 {
    KERNEL_FORMAT
}

#[pyfunction]
fn catalog_limits() -> (usize, usize, usize) {
    (
        MAX_CATALOG_CONDITIONS,
        MAX_CATALOG_NODES,
        MAX_CATALOG_RETAINED_NODES,
    )
}

fn id(value: &str) -> PyResult<Id> {
    Id::from_hex(value)
        .ok_or_else(|| PyValueError::new_err(format!("invalid condition id {value}")))
}

fn digest(value: &str) -> PyResult<Digest> {
    Digest::from_hex(value).ok_or_else(|| PyValueError::new_err("invalid effect-model digest"))
}

/// Hydrated once from rows of one validated, immutable generation.
#[pyclass]
struct ConditionGraph {
    diagrams: HashMap<Id, Diagram>,
    boundaries: HashMap<Id, KernelBoundary>,
    node_count: usize,
    retained_node_count: usize,
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
        let retained_node_count = diagrams
            .values()
            .map(|diagram| diagram.node_count().saturating_sub(2))
            .sum();
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
            retained_node_count,
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

    #[getter]
    fn retained_node_count(&self) -> usize {
        self.retained_node_count
    }

    /// `None` carries a named boundary; a missing id is an invalid query.
    fn compatible(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.question(left, right, Diagram::compatible)
    }

    fn implies(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.question(left, right, Diagram::implies)
    }
}

struct NativeSummary {
    function_node_id: Id,
    condition_id: Id,
    source_flow_fact_id: Id,
    source_origin_id: Id,
    verdict: String,
    boundary: Option<String>,
    path_depth: i64,
    steps: Vec<(String, Id, Id)>,
}

/// One immutable, generation-pinned index. Python only transports validated Arrow columns;
/// operation/formal resolution and proof admission happen here.
#[pyclass]
struct SemanticExecutor {
    graph: ConditionGraph,
    effect_model_digest: Digest,
    paths: HashMap<String, Id>,
    formals: HashMap<(Id, String), Id>,
    summaries: HashMap<Id, NativeSummary>,
    by_formal: HashMap<(Id, Id), Vec<Id>>,
    boundaries: BoundaryIndex,
    links_by_formal: HashMap<(Id, Id), Vec<ValueLink>>,
    leaves_by_formal: HashMap<(Id, Id), Vec<TestLeaf>>,
    link_spans: HashMap<Id, (Option<String>, i64, i64)>,
}

#[pymethods]
impl SemanticExecutor {
    /// Production loader: validate the IPC file schemas against cpg-schema and decode columns
    /// by name. The tuple constructor remains for focused synthetic proof tests.
    #[staticmethod]
    fn from_ipc(
        kernel_format: u32,
        snapshot_id: String,
        effect_model_digest: String,
        files: Vec<(String, Vec<u8>)>,
    ) -> PyResult<Self> {
        let input = ipc_input::decode(files)?;
        Self::new(
            kernel_format,
            snapshot_id,
            effect_model_digest,
            input.conditions,
            input.nodes,
            input.operations,
            input.public_paths,
            input.parameters,
            input.flows,
            input.steps,
            input.boundaries,
            input.leaves,
            input.links,
        )
    }

    #[new]
    #[allow(
        clippy::too_many_arguments,
        reason = "one checked generation crosses this constructor"
    )]
    fn new(
        kernel_format: u32,
        snapshot_id: String,
        effect_model_digest: String,
        conditions: Vec<(String, Option<String>, Option<String>)>,
        nodes: Vec<(String, String, String, String)>,
        operations: Vec<String>,
        public_paths: Vec<(String, String)>,
        parameters: Vec<(String, String, String)>,
        flows: Vec<FlowInput>,
        steps: Vec<(String, i64, String, String, String)>,
        boundaries: Vec<(String, String, String, String, String, String)>,
        leaves: Vec<LeafInput>,
        links: Vec<LinkInput>,
    ) -> PyResult<Self> {
        if operations.len() > MAX_SURFACE_ROWS
            || public_paths.len() > MAX_SURFACE_ROWS
            || parameters.len() > MAX_SURFACE_ROWS
            || flows.len() > MAX_SUMMARY_ROWS
            || steps.len() > MAX_SUMMARY_ROWS
            || boundaries.len() > MAX_SUMMARY_ROWS
            || leaves.len() > MAX_SUMMARY_ROWS
            || links.len() > MAX_SUMMARY_ROWS
        {
            return Err(PyValueError::new_err("semantic index exceeds load limits"));
        }
        let graph = ConditionGraph::new(kernel_format, conditions, nodes)?;
        let snapshot_id = id(&snapshot_id)?;
        let effect_model_digest = digest(&effect_model_digest)?;
        let operations = operations
            .into_iter()
            .map(|value| id(&value))
            .collect::<PyResult<HashSet<_>>>()?;
        let mut paths = HashMap::new();
        for (path, node) in public_paths {
            let node = id(&node)?;
            if operations.contains(&node) && paths.insert(path.clone(), node).is_some() {
                return Err(PyValueError::new_err(format!(
                    "duplicate public path {path}"
                )));
            }
        }
        let mut formals = HashMap::new();
        for (operation, formal, name) in parameters {
            let operation = id(&operation)?;
            let formal = id(&formal)?;
            if !operations.contains(&operation) {
                return Err(PyValueError::new_err("parameter names a non-operation"));
            }
            if formals.insert((operation, name.clone()), formal).is_some() {
                return Err(PyValueError::new_err(format!("duplicate formal {name}")));
            }
        }
        let formal_ids: HashSet<(Id, Id)> = formals
            .iter()
            .map(|(&(operation, _), &formal)| (operation, formal))
            .collect();
        let mut summaries = HashMap::new();
        let mut by_formal: HashMap<(Id, Id), Vec<Id>> = HashMap::new();
        for (summary, function, formal, condition, verdict, boundary, path_depth,
             source_flow_fact, source_origin) in flows {
            let summary = id(&summary)?;
            let function = id(&function)?;
            let formal = id(&formal)?;
            let condition = id(&condition)?;
            let source_flow_fact = id(&source_flow_fact)?;
            let source_origin = id(&source_origin)?;
            if !graph.has_condition(condition) {
                return Err(PyValueError::new_err("summary references absent condition"));
            }
            if !(0..=8).contains(&path_depth)
                || !matches!(verdict.as_str(), "established" | "conditional" | "unknown")
                || (verdict == "unknown") != boundary.is_some()
            {
                return Err(PyValueError::new_err("invalid summary verdict/boundary"));
            }
            if verdict != "unknown"
                && !graph
                    .diagrams
                    .get(&condition)
                    .is_some_and(|root| !root.is_false())
            {
                return Err(PyValueError::new_err(
                    "positive summary has no finite condition",
                ));
            }
            if summaries
                .insert(
                    summary,
                    NativeSummary {
                        function_node_id: function,
                        condition_id: condition,
                        source_flow_fact_id: source_flow_fact,
                        source_origin_id: source_origin,
                        verdict,
                        boundary,
                        path_depth,
                        steps: Vec::new(),
                    },
                )
                .is_some()
            {
                return Err(PyValueError::new_err("duplicate summary id"));
            }
            by_formal
                .entry((function, formal))
                .or_default()
                .push(summary);
        }
        let mut ordinals: HashMap<Id, Vec<(i64, String, Id, Id)>> = HashMap::new();
        for (summary, ordinal, kind, evidence, condition) in steps {
            let summary = id(&summary)?;
            let evidence = id(&evidence)?;
            let condition = id(&condition)?;
            if ordinal < 0
                || !summaries.contains_key(&summary)
                || !graph.has_condition(condition)
                || !SummaryFlowStepKind::all()
                    .iter()
                    .any(|candidate| candidate.text() == kind)
            {
                return Err(PyValueError::new_err("invalid summary proof step"));
            }
            if summaries[&summary].verdict != "unknown" && !graph.diagrams.contains_key(&condition)
            {
                return Err(PyValueError::new_err(
                    "positive proof step has no finite condition",
                ));
            }
            ordinals
                .entry(summary)
                .or_default()
                .push((ordinal, kind, evidence, condition));
        }
        for (summary_id, summary) in &mut summaries {
            let Some(mut proof) = ordinals.remove(summary_id) else {
                return Err(PyValueError::new_err("summary has no proof steps"));
            };
            proof.sort_by_key(|step| step.0);
            if proof.iter().enumerate().any(|(i, step)| step.0 != i as i64) {
                return Err(PyValueError::new_err("summary proof ordinals have a gap"));
            }
            if proof.len() > 64 {
                return Err(PyValueError::new_err("summary proof exceeds depth limit"));
            }
            summary.steps = proof
                .into_iter()
                .map(|(_, kind, evidence, condition)| (kind, evidence, condition))
                .collect();
        }
        for ids in by_formal.values_mut() {
            ids.sort();
        }
        let mut boundary_index: BoundaryIndex = HashMap::new();
        for (function, formal, source, origin, condition, reason) in boundaries {
            let condition = id(&condition)?;
            if !graph.has_condition(condition) {
                return Err(PyValueError::new_err(
                    "boundary references absent condition",
                ));
            }
            if !BoundaryReason::all()
                .iter()
                .any(|candidate| candidate.text() == reason)
            {
                return Err(PyValueError::new_err("invalid summary boundary reason"));
            }
            boundary_index
                .entry((id(&function)?, id(&formal)?))
                .or_default()
                .push((id(&source)?, id(&origin)?, condition, reason));
        }
        for reasons in boundary_index.values_mut() {
            reasons.sort();
            reasons.dedup();
        }
        let mut leaves_by_id = HashMap::new();
        for (fact, module, condition, atom_id, atom, _, start, end) in leaves {
            let fact = id(&fact)?;
            let condition = id(&condition)?;
            let atom_id = id(&atom_id)?;
            if start < 0
                || end < start
                || !graph.has_condition(condition)
                || IdHasher::new("bdd-atom").str(&atom).finish_id() != atom_id
                || Atom::parse_encoded(&atom).is_err()
            {
                return Err(PyValueError::new_err("invalid test leaf"));
            }
            let leaf = TestLeaf {
                snapshot_id,
                fact_id: fact,
                module_node_id: id(&module)?,
                atom_id,
                condition_id: condition,
                atom,
            };
            if leaves_by_id.insert(fact, leaf).is_some() {
                return Err(PyValueError::new_err("duplicate test leaf"));
            }
        }
        let mut links_by_formal: HashMap<(Id, Id), Vec<ValueLink>> = HashMap::new();
        let mut link_spans = HashMap::new();
        for (
            link,
            operation,
            formal,
            module,
            leaf,
            atom,
            condition,
            place,
            origin,
            effect,
            (path, start, end),
        ) in links
        {
            let link = id(&link)?;
            let operation = id(&operation)?;
            let formal = id(&formal)?;
            let module = id(&module)?;
            let leaf = id(&leaf)?;
            let atom = id(&atom)?;
            let condition = id(&condition)?;
            let origin = TestValueLinkOrigin::all()
                .iter()
                .copied()
                .find(|candidate| candidate.text() == origin)
                .ok_or_else(|| PyValueError::new_err("invalid value-link origin"))?;
            let source = leaves_by_id
                .get(&leaf)
                .ok_or_else(|| PyValueError::new_err("value link has no test leaf"))?;
            if start < 0
                || end <= start
                || digest(&effect)? != effect_model_digest
                || source.module_node_id != module
                || source.atom_id != atom
                || source.condition_id != condition
                || Atom::parse_encoded(&source.atom)
                    .ok()
                    .and_then(|parsed| parsed.place().map(str::to_owned))
                    .as_deref()
                    != Some(place.as_str())
                || !formal_ids.contains(&(operation, formal))
            {
                return Err(PyValueError::new_err("invalid value link or effect digest"));
            }
            if link_spans.insert(link, (path, start, end)).is_some() {
                return Err(PyValueError::new_err("duplicate value link"));
            }
            links_by_formal
                .entry((operation, formal))
                .or_default()
                .push(ValueLink {
                    snapshot_id,
                    link_id: link,
                    operation_node_id: operation,
                    formal_node_id: formal,
                    module_node_id: module,
                    leaf_fact_id: leaf,
                    atom_id: atom,
                    condition_id: condition,
                    place,
                    origin,
                    effect_model_digest,
                });
        }
        let mut leaves_by_formal = HashMap::new();
        for (key, links) in &mut links_by_formal {
            links.sort_by_key(|link| link.link_id);
            let ids: BTreeSet<Id> = links.iter().map(|link| link.leaf_fact_id).collect();
            leaves_by_formal.insert(
                *key,
                ids.iter().map(|id| leaves_by_id[id].clone()).collect(),
            );
        }
        for summary in summaries.values() {
            for (index, (kind, evidence, _)) in summary.steps.iter().enumerate() {
                if kind == "caller_condition_link" {
                    let next_is_callee_link = summary.steps.get(index + 1)
                        .is_some_and(|next| next.0 == "callee_condition_link");
                    let supported = next_is_callee_link
                        && links_by_formal.values().flatten().any(|link| {
                            link.link_id == *evidence
                                && link.operation_node_id == summary.function_node_id
                                && link.origin == TestValueLinkOrigin::DirectParameterReachNoEffect
                                && leaves_by_id.get(&link.leaf_fact_id).is_some_and(|leaf| {
                                    graph.diagrams.get(&summary.condition_id).is_some_and(|root| {
                                        if !root.support().contains(&leaf.atom) { return false; }
                                        let Ok(atom) = Atom::parse_encoded(&leaf.atom) else {
                                            return false;
                                        };
                                        let Ok(predicate) = Diagram::from_atom(&atom) else {
                                            return false;
                                        };
                                        root.implies(&predicate) == Ok(true)
                                            || predicate.not().is_ok_and(|negated|
                                                root.implies(&negated) == Ok(true))
                                    })
                                })
                        });
                    if !supported {
                        return Err(PyValueError::new_err(
                            "caller condition link does not fix its direct formal",
                        ));
                    }
                }
                if kind != "callee_summary" { continue; }
                let Some(callee) = summaries.get(evidence) else {
                    return Err(PyValueError::new_err("missing cited callee summary"));
                };
                if callee.path_depth >= summary.path_depth {
                    return Err(PyValueError::new_err("cyclic or unordered callee proof"));
                }
                if callee.verdict == "established" { continue; }
                let Some((link_kind, link_id, condition)) = index.checked_sub(1)
                    .and_then(|prior| summary.steps.get(prior)) else {
                    return Err(PyValueError::new_err("conditional callee lacks a test link"));
                };
                let supported = callee.verdict == "conditional"
                    && link_kind == "callee_condition_link"
                    && *condition == callee.condition_id
                    && links_by_formal.values().flatten().any(|link| {
                        link.link_id == *link_id
                            && link.operation_node_id == callee.function_node_id
                            && link.origin == TestValueLinkOrigin::DirectParameterReachNoEffect
                            && leaves_by_id.get(&link.leaf_fact_id).is_some_and(|leaf| {
                                graph.diagrams.get(&callee.condition_id).is_some_and(|root|
                                    root.support().contains(&leaf.atom))
                            })
                    });
                if !supported {
                    return Err(PyValueError::new_err("conditional callee lacks a cited exact test link"));
                }
            }
        }
        Ok(Self {
            graph,
            effect_model_digest,
            paths,
            formals,
            summaries,
            by_formal,
            boundaries: boundary_index,
            links_by_formal,
            leaves_by_formal,
            link_spans,
        })
    }

    #[getter]
    fn condition_count(&self) -> usize {
        self.graph.condition_count()
    }

    #[getter]
    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    #[getter]
    fn retained_node_count(&self) -> usize {
        self.graph.retained_node_count()
    }

    fn compatible(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.graph.compatible(left, right)
    }

    fn implies(&self, left: &str, right: &str) -> PyResult<(Option<bool>, Option<String>)> {
        self.graph.implies(left, right)
    }

    /// An exact primitive input may refute one cited summary path. A satisfiable remainder or
    /// absent value link is unknown, never proof that a Python execution reaches the return.
    fn assess_value_path(
        &self,
        operation_path: &str,
        formal_name: &str,
        summary_id: &str,
        kind: &str,
        value: &str,
        standard_builtins: bool,
    ) -> PyResult<RefutationAnswer> {
        let operation = *self
            .paths
            .get(operation_path)
            .ok_or_else(|| PyValueError::new_err("unknown public operation"))?;
        let formal = *self
            .formals
            .get(&(operation, formal_name.to_owned()))
            .ok_or_else(|| PyValueError::new_err("unknown operation formal"))?;
        let summary_id = id(summary_id)?;
        if !self
            .by_formal
            .get(&(operation, formal))
            .is_some_and(|ids| ids.contains(&summary_id))
        {
            return Err(PyValueError::new_err(
                "summary does not belong to operation formal",
            ));
        }
        let summary = &self.summaries[&summary_id];
        if summary.verdict == "unknown" {
            return Ok((
                "unknown".to_owned(),
                Vec::new(),
                summary.boundary.clone(),
                work_answer(TheoryWork::default()),
            ));
        }
        let Some(diagram) = self.graph.diagrams.get(&summary.condition_id) else {
            return Ok((
                "unknown".to_owned(),
                Vec::new(),
                Some("condition_boundary".to_owned()),
                work_answer(TheoryWork::default()),
            ));
        };
        let input = match kind {
            "none" if value.is_empty() => Value::None,
            "bool" if value == "true" || value == "false" => Value::Bool(value == "true"),
            "int" => Value::Int(
                value
                    .parse()
                    .map_err(|_| PyValueError::new_err("invalid int"))?,
            ),
            "str" => Value::Str(value.to_owned()),
            _ => return Err(PyValueError::new_err("invalid exact primitive input")),
        };
        let key = (operation, formal);
        let links = self
            .links_by_formal
            .get(&key)
            .map_or(&[][..], Vec::as_slice);
        let leaves = self
            .leaves_by_formal
            .get(&key)
            .map_or(&[][..], Vec::as_slice);
        let query = ExactInput {
            operation_node_id: operation,
            formal_node_id: formal,
            value: &input,
            builtin_namespace: if standard_builtins {
                BuiltinNamespace::StandardAssumed
            } else {
                BuiltinNamespace::Unknown
            },
            effect_model_digest: self.effect_model_digest,
        };
        let link_evidence = |ids: &[Id]| {
            ids.iter()
                .map(|link| {
                    let (path, start, end) = &self.link_spans[link];
                    (link.hex(), path.clone(), *start, *end)
                })
                .collect()
        };
        let assessment = assess_exact_input_with_work(diagram, query, links, leaves);
        let work = work_answer(assessment.work);
        match assessment.outcome {
            Ok(ExactInputOutcome::Refuted(proof)) => Ok((
                "refuted_under_model".to_owned(),
                link_evidence(&proof.value_link_ids),
                None,
                work,
            )),
            Ok(ExactInputOutcome::CompatibleUnderModel { value_link_ids }) => Ok((
                "compatible_under_model".to_owned(),
                link_evidence(&value_link_ids),
                None,
                work,
            )),
            Ok(ExactInputOutcome::Unknown) => Ok(("unknown".to_owned(), Vec::new(), None, work)),
            Err(reason) => Ok((
                "unknown".to_owned(),
                Vec::new(),
                Some(match reason {
                    TheoryBoundary::Kernel(boundary) => boundary.code().to_owned(),
                    TheoryBoundary::AssignmentBudget => "budget_reached".to_owned(),
                    TheoryBoundary::ConflictingProof => "conflicting_proof".to_owned(),
                }),
                work,
            )),
        }
    }

    /// Page through one formal's finite summary paths and open boundaries. Each path's exact
    /// input result is local to that path; the page never claims complete operation behavior.
    #[allow(
        clippy::too_many_arguments,
        reason = "the typed Python request is unpacked at the native boundary"
    )]
    fn inspect_value_paths(
        &self,
        operation_path: &str,
        formal_name: &str,
        kind: &str,
        value: &str,
        standard_builtins: bool,
        offset: usize,
        limit: usize,
    ) -> PyResult<ValuePathPage> {
        if limit == 0 || limit > 50 {
            return Err(PyValueError::new_err("limit must be between 1 and 50"));
        }
        let operation = *self
            .paths
            .get(operation_path)
            .ok_or_else(|| PyValueError::new_err("unknown public operation"))?;
        let formal = *self
            .formals
            .get(&(operation, formal_name.to_owned()))
            .ok_or_else(|| PyValueError::new_err("unknown operation formal"))?;
        let ids = self
            .by_formal
            .get(&(operation, formal))
            .map_or(&[][..], Vec::as_slice);
        let open = self
            .boundaries
            .get(&(operation, formal))
            .map_or(&[][..], Vec::as_slice);
        let total = ids.len() + open.len();
        if offset > total {
            return Err(PyValueError::new_err("cursor offset exceeds result size"));
        }
        let end = offset.saturating_add(limit).min(total);
        let mut paths = Vec::new();
        let mut boundaries = Vec::new();
        for index in offset..end {
            if let Some(summary_id) = ids.get(index) {
                let summary = &self.summaries[summary_id];
                let (result, links, boundary, theory_work) = self.assess_value_path(
                    operation_path,
                    formal_name,
                    &summary_id.hex(),
                    kind,
                    value,
                    standard_builtins,
                )?;
                paths.push((
                    summary_id.hex(),
                    summary.verdict.clone(),
                    summary.condition_id.hex(),
                    summary
                        .steps
                        .iter()
                        .map(|(step_kind, evidence, condition)| {
                            (step_kind.clone(), evidence.hex(), condition.hex())
                        })
                        .collect(),
                    result,
                    links,
                    boundary,
                    theory_work,
                    summary.source_flow_fact_id.hex(),
                    summary.source_origin_id.hex(),
                ));
            } else {
                let (source, origin, condition, reason) = &open[index - ids.len()];
                boundaries.push((source.hex(), origin.hex(), condition.hex(), reason.clone()));
            }
        }
        Ok((paths, boundaries, total, end < total, end - offset))
    }
}

impl ConditionGraph {
    fn has_condition(&self, condition: Id) -> bool {
        self.diagrams.contains_key(&condition) || self.boundaries.contains_key(&condition)
    }

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
    m.add_class::<SemanticExecutor>()?;
    m.add_function(wrap_pyfunction!(kernel_format, m)?)?;
    m.add_function(wrap_pyfunction!(catalog_limits, m)?)?;
    m.add_function(wrap_pyfunction!(probe_compatible, m)?)?;
    m.add_function(wrap_pyfunction!(probe_implies, m)?)?;
    Ok(())
}
