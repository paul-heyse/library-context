//! Pure finite source-to-return summary composition over explicit, typed inputs.
//! DataFusion acquisition and Delta publication belong to `cpg-core`.
use std::collections::{HashMap, HashSet};
use cpg_schema::behavior::{ModeledArgumentEvaluationsRow, SummaryComponentsRow, SummaryFlowStepsRow, SummaryFlowsRow};
use cpg_schema::codebook::{BoundaryReason, ModeledArgumentEvaluationStatus, SummaryFlowKind, SummaryFlowStepKind, Verdict};
use cpg_schema::condition_kernel::Diagram;
use cpg_schema::id::{Id, recipe};
use cpg_schema::models::{InputPath, OutputPath};

cpg_schema::query_row! {
    pub struct SummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_start_byte: i64,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    pub struct ModeledSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        call_fact_id: Id,
        source_argument_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        callee_resolution_fact_id: Id,
        argument_count: i64,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    pub struct ModeledAssignmentSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        predecessor_flow_fact_id: Id,
        reaching_fact_id: Id,
        predecessor_condition_id: Id,
        reaching_condition_id: Id,
        successor_condition_id: Id,
        source_argument_fact_id: Id,
        call_fact_id: Id,
        pysa_fact_id: Id,
        model_id: Id,
        rule_id: Id,
        callee_resolution_fact_id: Id,
        argument_count: i64,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

cpg_schema::query_row! {
    pub struct LocalCallSummaryFlowSeed {
        snapshot_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        condition_id: Id,
        call_site_node_id: Id,
        call_fact_id: Id,
        pysa_fact_id: Id,
        callee_node_id: Id,
        callee_parameter_node_id: Id,
        callee_resolution_fact_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        approximated: bool,
    }
}

struct CallEvidence {
    flow_fact_id: Id,
    parameter_node_id: Id,
    source_argument_fact_id: Id,
    call_fact_id: Id,
    pysa_fact_id: Id,
    model_id: Id,
    rule_id: Id,
    callee_resolution_fact_id: Id,
    argument_count: i64,
    condition_id: Id,
}

type ArgumentIndex = HashMap<(Id, Id, Id, Id, Id), Vec<ModeledArgumentEvaluationsRow>>;

struct FinitePath {
    snapshot_id: Id,
    function_node_id: Id,
    parameter_node_id: Id,
    parameter_name: String,
    source_flow_fact_id: Id,
    condition_id: Id,
    condition_is_true: bool,
    return_site_fact_id: Id,
    return_region_fact_id: Id,
    approximated: bool,
    path_depth: i64,
    proof: Vec<recipe::SummaryFlowProofStep>,
}

cpg_schema::query_row! {
    pub struct ReturnPassStep {
        return_site_fact_id: Id,
        pass_fact_id: Id,
        condition_id: Id,
        ordinal: i64,
    }
}

cpg_schema::query_row! {
    pub struct PrecedingCallRegion {
        function_node_id: Id,
        call_fact_id: Id,
        call_start_byte: i64,
        condition_id: Option<Id>,
        approximated: Option<bool>,
    }
}

type PrecedingCallIndex = HashMap<Id, Vec<PrecedingCallRegion>>;
type ReturnPassIndex = HashMap<Id, Vec<ReturnPassStep>>;

pub struct FiniteSummaryInputs {
    pub diagrams: HashMap<Id, Diagram>,
    pub boundaries: HashMap<Id, BoundaryReason>,
    pub pass_steps: HashMap<Id, Vec<ReturnPassStep>>,
    pub preceding_calls: HashMap<Id, Vec<PrecedingCallRegion>>,
    pub components: Vec<SummaryComponentsRow>,
    pub direct_seeds: Vec<SummaryFlowSeed>,
    pub modeled_seeds: Vec<ModeledSummaryFlowSeed>,
    pub evaluations: Vec<ModeledArgumentEvaluationsRow>,
    pub assignment_seeds: Vec<ModeledAssignmentSummaryFlowSeed>,
    pub local_seeds: Vec<LocalCallSummaryFlowSeed>,
}

fn has_unproved_preceding_call(
    seed: &SummaryFlowSeed,
    calls: &PrecedingCallIndex,
    diagrams: &HashMap<Id, Diagram>,
) -> bool {
    let Some(return_condition) = diagrams.get(&seed.condition_id) else {
        return calls.get(&seed.function_node_id).is_some_and(|rows| {
            rows.iter()
                .any(|call| call.call_start_byte < seed.return_start_byte)
        });
    };
    calls.get(&seed.function_node_id).is_some_and(|rows| {
        rows.iter()
            .take_while(|call| call.call_start_byte < seed.return_start_byte)
            .any(|call| {
                if call.approximated != Some(false) {
                    return true;
                }
                let Some(call_condition) = call.condition_id.and_then(|id| diagrams.get(&id))
                else {
                    return true;
                };
                !matches!(return_condition.and(call_condition), Ok(both) if both.is_false())
            })
    })
}

/// The first finite summary case: a direct, synchronous body return of a local parameter.
/// A bounded/missing condition is a named unknown, never an admitted positive flow.
fn direct_flows(
    seeds: Vec<SummaryFlowSeed>,
    diagrams: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
    pass_steps: &ReturnPassIndex,
    preceding_calls: &PrecedingCallIndex,
) -> (Vec<SummaryFlowsRow>, Vec<SummaryFlowStepsRow>) {
    let mut flows = Vec::new();
    let mut steps = Vec::new();
    for seed in seeds {
        if has_unproved_preceding_call(&seed, preceding_calls, diagrams) {
            continue;
        }
        let (verdict, boundary_reason) = match diagrams.get(&seed.condition_id) {
            Some(diagram) if diagram.is_false() => continue,
            Some(diagram) if diagram.is_true() => (Verdict::Established, None),
            Some(_) => (Verdict::Conditional, None),
            None => (
                Verdict::Unknown,
                Some(
                    boundaries
                        .get(&seed.condition_id)
                        .copied()
                        .unwrap_or(BoundaryReason::MissingEvidence),
                ),
            ),
        };
        let input_path = InputPath::Parameter {
            name: seed.parameter_name,
        }
        .render();
        let output_path = OutputPath::ReturnValue.render();
        let mut proof = vec![recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::RawIdentity,
            evidence_id: seed.source_flow_fact_id,
            condition_id: seed.condition_id,
        }];
        let finalizers = pass_steps
            .get(&seed.return_site_fact_id)
            .map_or(&[][..], Vec::as_slice);
        for finalizer in finalizers {
            proof.push(recipe::SummaryFlowProofStep {
                kind: SummaryFlowStepKind::FinalizerPass,
                evidence_id: finalizer.pass_fact_id,
                condition_id: finalizer.condition_id,
            });
        }
        let summary_id = recipe::summary_flow(&recipe::SummaryFlowIdentity {
            function: seed.function_node_id,
            parameter: seed.parameter_node_id,
            input_path: &input_path,
            output_path: &output_path,
            transfer_kind: SummaryFlowKind::Value,
            condition: seed.condition_id,
            return_site: seed.return_site_fact_id,
            return_region: seed.return_region_fact_id,
            steps: &proof,
        });
        flows.push(SummaryFlowsRow {
            snapshot_id: seed.snapshot_id,
            summary_id,
            function_node_id: seed.function_node_id,
            parameter_node_id: seed.parameter_node_id,
            input_path,
            output_path,
            kind: SummaryFlowKind::Value,
            condition_id: seed.condition_id,
            verdict,
            boundary_reason,
            source_flow_fact_id: seed.source_flow_fact_id,
            return_site_fact_id: seed.return_site_fact_id,
            return_region_fact_id: seed.return_region_fact_id,
            approximated: seed.approximated,
            path_depth: 0,
        });
        steps.extend(
            proof
                .into_iter()
                .enumerate()
                .map(|(ordinal, step)| SummaryFlowStepsRow {
                    snapshot_id: seed.snapshot_id,
                    summary_id,
                    ordinal: ordinal as i64,
                    kind: step.kind,
                    evidence_id: step.evidence_id,
                    condition_id: step.condition_id,
                }),
        );
    }
    (flows, steps)
}

/// One completed candidate call path, shared by direct returns and the first assignment hop.
/// This is a may-path under the source/model abstraction, not universal normal execution.
fn modeled_call_proof(
    seed: &CallEvidence,
    by_candidate: &ArgumentIndex,
) -> Option<Vec<recipe::SummaryFlowProofStep>> {
    let arguments = by_candidate.get(&(
        seed.flow_fact_id,
        seed.parameter_node_id,
        seed.pysa_fact_id,
        seed.model_id,
        seed.rule_id,
    ))?;
    if arguments.len() != usize::try_from(seed.argument_count).ok()? {
        return None;
    }
    let mut ordered = arguments.clone();
    ordered.sort_by_key(|argument| argument.ordinal);
    if ordered.iter().enumerate().any(|(ordinal, argument)| {
        argument.ordinal != ordinal as i64
            || argument.status == ModeledArgumentEvaluationStatus::Unknown
            || argument.evidence_id.is_none()
            || argument.condition_id != seed.condition_id
    }) || ordered
        .iter()
        .filter(|argument| {
            argument.status == ModeledArgumentEvaluationStatus::SourceOperand
                && argument.argument_fact_id == seed.source_argument_fact_id
        })
        .count()
        != 1
    {
        return None;
    }
    let mut proof = Vec::with_capacity(ordered.len() + 4);
    proof.push(recipe::SummaryFlowProofStep {
        kind: SummaryFlowStepKind::CalleeResolution,
        evidence_id: seed.callee_resolution_fact_id,
        condition_id: seed.condition_id,
    });
    for argument in &ordered {
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ArgumentEvaluation,
            evidence_id: argument.evidence_id?,
            condition_id: argument.condition_id,
        });
    }
    for (kind, evidence_id) in [
        (SummaryFlowStepKind::CallSite, seed.call_fact_id),
        (SummaryFlowStepKind::CallTarget, seed.pysa_fact_id),
        (SummaryFlowStepKind::ModelRule, seed.rule_id),
    ] {
        proof.push(recipe::SummaryFlowProofStep {
            kind,
            evidence_id,
            condition_id: seed.condition_id,
        });
    }
    Some(proof)
}

fn push_finite_path(
    flows: &mut Vec<SummaryFlowsRow>,
    steps: &mut Vec<SummaryFlowStepsRow>,
    pass_steps: &ReturnPassIndex,
    path: FinitePath,
) {
    let input_path = InputPath::Parameter {
        name: path.parameter_name,
    }
    .render();
    let output_path = OutputPath::ReturnValue.render();
    let mut proof = path.proof;
    let finalizers = pass_steps
        .get(&path.return_site_fact_id)
        .map_or(&[][..], Vec::as_slice);
    if !finalizers.is_empty() {
        let index = proof
            .iter()
            .position(|step| step.kind == SummaryFlowStepKind::ReturnExit)
            .unwrap_or(proof.len());
        for (offset, finalizer) in finalizers.iter().enumerate() {
            proof.insert(
                index + offset,
                recipe::SummaryFlowProofStep {
                    kind: SummaryFlowStepKind::FinalizerPass,
                    evidence_id: finalizer.pass_fact_id,
                    condition_id: finalizer.condition_id,
                },
            );
        }
    }
    let summary_id = recipe::summary_flow(&recipe::SummaryFlowIdentity {
        function: path.function_node_id,
        parameter: path.parameter_node_id,
        input_path: &input_path,
        output_path: &output_path,
        transfer_kind: SummaryFlowKind::Value,
        condition: path.condition_id,
        return_site: path.return_site_fact_id,
        return_region: path.return_region_fact_id,
        steps: &proof,
    });
    flows.push(SummaryFlowsRow {
        snapshot_id: path.snapshot_id,
        summary_id,
        function_node_id: path.function_node_id,
        parameter_node_id: path.parameter_node_id,
        input_path,
        output_path,
        kind: SummaryFlowKind::Value,
        condition_id: path.condition_id,
        verdict: if path.condition_is_true {
            Verdict::Established
        } else {
            Verdict::Conditional
        },
        boundary_reason: None,
        source_flow_fact_id: path.source_flow_fact_id,
        return_site_fact_id: path.return_site_fact_id,
        return_region_fact_id: path.return_region_fact_id,
        approximated: path.approximated,
        path_depth: path.path_depth,
    });
    steps.extend(
        proof
            .into_iter()
            .enumerate()
            .map(|(ordinal, step)| SummaryFlowStepsRow {
                snapshot_id: path.snapshot_id,
                summary_id,
                ordinal: ordinal as i64,
                kind: step.kind,
                evidence_id: step.evidence_id,
                condition_id: step.condition_id,
            }),
    );
}

/// Reconstruct all admitted finite paths and their ordered proof steps from published inputs.
/// The same producer is called at write time and by the shared publication validator.
pub fn finite_flows(inputs: FiniteSummaryInputs) -> (Vec<SummaryFlowsRow>, Vec<SummaryFlowStepsRow>) {
    let FiniteSummaryInputs {
        diagrams, boundaries, pass_steps, preceding_calls, components,
        direct_seeds, modeled_seeds, evaluations, assignment_seeds, local_seeds,
    } = inputs;
    let recursive_functions: HashSet<Id> = components
        .iter()
        .filter(|row| row.recursive)
        .map(|row| row.function_node_id)
        .collect();
    let (mut flows, mut steps) =
        direct_flows(direct_seeds, &diagrams, &boundaries, &pass_steps, &preceding_calls);
    let seeds = modeled_seeds;
    let mut by_candidate: ArgumentIndex = HashMap::new();
    for evaluation in evaluations {
        by_candidate
            .entry((
                evaluation.candidate_flow_fact_id,
                evaluation.parameter_node_id,
                evaluation.pysa_fact_id,
                evaluation.model_id,
                evaluation.rule_id,
            ))
            .or_default()
            .push(evaluation);
    }
    for seed in seeds {
        if recursive_functions.contains(&seed.function_node_id) {
            continue;
        }
        let Some(condition) = diagrams.get(&seed.condition_id) else {
            continue;
        };
        let Some(return_condition) = diagrams.get(&seed.return_condition_id) else {
            continue;
        };
        if condition.is_false() || condition.implies(return_condition) != Ok(true) {
            continue;
        }
        let Some(mut proof) = modeled_call_proof(
            &CallEvidence {
                flow_fact_id: seed.source_flow_fact_id,
                parameter_node_id: seed.parameter_node_id,
                source_argument_fact_id: seed.source_argument_fact_id,
                call_fact_id: seed.call_fact_id,
                pysa_fact_id: seed.pysa_fact_id,
                model_id: seed.model_id,
                rule_id: seed.rule_id,
                callee_resolution_fact_id: seed.callee_resolution_fact_id,
                argument_count: seed.argument_count,
                condition_id: seed.condition_id,
            },
            &by_candidate,
        ) else {
            continue;
        };
        proof.push(recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::ReturnExit,
            evidence_id: seed.return_site_fact_id,
            condition_id: seed.return_condition_id,
        });
        push_finite_path(
            &mut flows,
            &mut steps,
            &pass_steps,
            FinitePath {
                snapshot_id: seed.snapshot_id,
                function_node_id: seed.function_node_id,
                parameter_node_id: seed.parameter_node_id,
                parameter_name: seed.parameter_name,
                source_flow_fact_id: seed.source_flow_fact_id,
                condition_id: seed.condition_id,
                condition_is_true: condition.is_true(),
                return_site_fact_id: seed.return_site_fact_id,
                return_region_fact_id: seed.return_region_fact_id,
                approximated: seed.approximated,
                path_depth: 1,
                proof,
            },
        );
    }
    for seed in assignment_seeds {
        if recursive_functions.contains(&seed.function_node_id) {
            continue;
        }
        let Some(condition) = diagrams.get(&seed.predecessor_condition_id) else {
            continue;
        };
        if condition.is_false()
            || [
                seed.reaching_condition_id,
                seed.successor_condition_id,
                seed.return_condition_id,
            ]
            .iter()
            .any(|id| {
                diagrams
                    .get(id)
                    .is_none_or(|other| condition.implies(other) != Ok(true))
            })
        {
            continue;
        }
        let Some(mut proof) = modeled_call_proof(
            &CallEvidence {
                flow_fact_id: seed.predecessor_flow_fact_id,
                parameter_node_id: seed.parameter_node_id,
                source_argument_fact_id: seed.source_argument_fact_id,
                call_fact_id: seed.call_fact_id,
                pysa_fact_id: seed.pysa_fact_id,
                model_id: seed.model_id,
                rule_id: seed.rule_id,
                callee_resolution_fact_id: seed.callee_resolution_fact_id,
                argument_count: seed.argument_count,
                condition_id: seed.predecessor_condition_id,
            },
            &by_candidate,
        ) else {
            continue;
        };
        for (kind, evidence_id, condition_id) in [
            (
                SummaryFlowStepKind::DefinitionReaching,
                seed.reaching_fact_id,
                seed.reaching_condition_id,
            ),
            (
                SummaryFlowStepKind::ReturnSource,
                seed.source_flow_fact_id,
                seed.successor_condition_id,
            ),
            (
                SummaryFlowStepKind::ReturnExit,
                seed.return_site_fact_id,
                seed.return_condition_id,
            ),
        ] {
            proof.push(recipe::SummaryFlowProofStep {
                kind,
                evidence_id,
                condition_id,
            });
        }
        push_finite_path(
            &mut flows,
            &mut steps,
            &pass_steps,
            FinitePath {
                snapshot_id: seed.snapshot_id,
                function_node_id: seed.function_node_id,
                parameter_node_id: seed.parameter_node_id,
                parameter_name: seed.parameter_name,
                source_flow_fact_id: seed.source_flow_fact_id,
                condition_id: seed.predecessor_condition_id,
                condition_is_true: condition.is_true(),
                return_site_fact_id: seed.return_site_fact_id,
                return_region_fact_id: seed.return_region_fact_id,
                approximated: seed.approximated,
                path_depth: 2,
                proof,
            },
        );
    }
    // The first interprocedural case needs no cross-scope atom substitution: the callee's
    // admitted path is unconditional. Acyclic components run after their callees, while
    // recursive components retain their existing unknown boundary until the bounded worklist.
    let component_by_function: HashMap<Id, (i64, bool)> = components
        .into_iter()
        .map(|row| (row.function_node_id, (row.component_order, row.recursive)))
        .collect();
    let mut local_seeds = local_seeds;
    local_seeds.sort_by_key(|seed| {
        (
            component_by_function
                .get(&seed.function_node_id)
                .map_or(i64::MAX, |c| c.0),
            seed.function_node_id,
            seed.call_site_node_id,
            seed.source_flow_fact_id,
            seed.pysa_fact_id,
        )
    });
    let mut by_formal: HashMap<(Id, Id), Vec<SummaryFlowsRow>> = HashMap::new();
    for flow in &flows {
        by_formal
            .entry((flow.function_node_id, flow.parameter_node_id))
            .or_default()
            .push(flow.clone());
    }
    const MAX_LOCAL_PATH_DEPTH: i64 = 8;
    for seed in local_seeds {
        let Some((_, false)) = component_by_function.get(&seed.function_node_id) else {
            continue;
        };
        let Some(condition) = diagrams.get(&seed.condition_id) else {
            continue;
        };
        let Some(exit_condition) = diagrams.get(&seed.return_condition_id) else {
            continue;
        };
        if condition.is_false() || condition.implies(exit_condition) != Ok(true) {
            continue;
        }
        let Some(callee_paths) = by_formal
            .get(&(seed.callee_node_id, seed.callee_parameter_node_id))
            .cloned()
        else {
            continue;
        };
        for callee in callee_paths {
            if callee.verdict != Verdict::Established
                || callee.boundary_reason.is_some()
                || callee.kind != SummaryFlowKind::Value
                || callee.output_path != OutputPath::ReturnValue.render()
                || callee.path_depth >= MAX_LOCAL_PATH_DEPTH
                || !diagrams
                    .get(&callee.condition_id)
                    .is_some_and(Diagram::is_true)
            {
                continue;
            }
            let proof = [
                (
                    SummaryFlowStepKind::CalleeResolution,
                    seed.callee_resolution_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::ArgumentEvaluation,
                    seed.source_flow_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CallSite,
                    seed.call_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CallTarget,
                    seed.pysa_fact_id,
                    seed.condition_id,
                ),
                (
                    SummaryFlowStepKind::CalleeSummary,
                    callee.summary_id,
                    callee.condition_id,
                ),
                (
                    SummaryFlowStepKind::ReturnExit,
                    seed.return_site_fact_id,
                    seed.return_condition_id,
                ),
            ]
            .into_iter()
            .map(
                |(kind, evidence_id, condition_id)| recipe::SummaryFlowProofStep {
                    kind,
                    evidence_id,
                    condition_id,
                },
            )
            .collect();
            push_finite_path(
                &mut flows,
                &mut steps,
                &pass_steps,
                FinitePath {
                    snapshot_id: seed.snapshot_id,
                    function_node_id: seed.function_node_id,
                    parameter_node_id: seed.parameter_node_id,
                    parameter_name: seed.parameter_name.clone(),
                    source_flow_fact_id: seed.source_flow_fact_id,
                    condition_id: seed.condition_id,
                    condition_is_true: condition.is_true(),
                    return_site_fact_id: seed.return_site_fact_id,
                    return_region_fact_id: seed.return_region_fact_id,
                    approximated: seed.approximated || callee.approximated,
                    path_depth: callee.path_depth + 1,
                    proof,
                },
            );
            if let Some(flow) = flows.last() {
                by_formal
                    .entry((flow.function_node_id, flow.parameter_node_id))
                    .or_default()
                    .push(flow.clone());
            }
        }
    }
    (flows, steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> Id {
        Id([byte; 16])
    }

    fn direct_seed() -> SummaryFlowSeed {
        SummaryFlowSeed {
            snapshot_id: id(1),
            function_node_id: id(2),
            parameter_node_id: id(3),
            parameter_name: "value".to_owned(),
            source_flow_fact_id: id(4),
            condition_id: Diagram::always().id(),
            return_site_fact_id: id(5),
            return_region_fact_id: id(6),
            return_start_byte: 30,
            approximated: false,
        }
    }

    fn inputs() -> FiniteSummaryInputs {
        let always = Diagram::always();
        FiniteSummaryInputs {
            diagrams: HashMap::from([(always.id(), always)]),
            boundaries: HashMap::new(),
            pass_steps: HashMap::new(),
            preceding_calls: HashMap::new(),
            components: Vec::new(),
            direct_seeds: vec![direct_seed()],
            modeled_seeds: Vec::new(),
            evaluations: Vec::new(),
            assignment_seeds: Vec::new(),
            local_seeds: Vec::new(),
        }
    }

    #[test]
    fn direct_admission_and_independent_preceding_call_control_use_pure_inputs() {
        let (flows, steps) = finite_flows(inputs());
        assert_eq!(flows.len(), 1);
        assert_eq!(flows[0].verdict, Verdict::Established);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].kind, SummaryFlowStepKind::RawIdentity);

        let mut withheld = inputs();
        withheld.preceding_calls.insert(
            id(2),
            vec![PrecedingCallRegion {
                function_node_id: id(2),
                call_fact_id: id(7),
                call_start_byte: 10,
                condition_id: None,
                approximated: None,
            }],
        );
        let (flows, steps) = finite_flows(withheld);
        assert!(flows.is_empty());
        assert!(steps.is_empty());
    }
}
