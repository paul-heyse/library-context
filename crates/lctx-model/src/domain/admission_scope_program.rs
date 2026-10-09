//! Model-owned execution and support selection, including source ownership set operations.
use super::{
    analysis::enriched_execution::AnalysisInvocation,
    assertion::{AssertionQualification, SupportScope},
    attribution::{ProviderRun, RunFamily},
    execution::{
        context_execution::ContextExecution,
        fidelity::{ExecutionMembershipContext, ExecutionScope},
    },
    input::{CorpusLibrary, InputDistribution, InputRevision},
    resources::{Reservation, ResourceBudget},
    scope_program::*,
    source::{CoverageScope, SourceArtifact},
    *,
};
use std::{any::TypeId, sync::Arc};

pub struct ExecutionProgram {
    pub program: ScopeProgram,
    pub root: usize,
    pub root_namespace: usize,
    _construction: Box<dyn Reservation>,
}
pub struct SupportProgram {
    pub program: ScopeProgram,
    pub root: usize,
    pub ownership: SupportOwnership,
    _construction: Box<dyn Reservation>,
}
/// A bounded set of selected or full declared rows. Only exact membership filters and
/// projection/union compose ownership; no arbitrary expression or query text is accepted.
#[derive(Clone)]
pub enum OwnershipRows {
    Selected(usize),
    Full {
        input: usize,
        filters: Vec<OwnershipFilter>,
    },
}
#[derive(Clone)]
pub struct OwnershipFilter {
    pub field: &'static str,
    pub values: Arc<OwnershipSet>,
}
#[derive(Clone)]
pub enum OwnershipSet {
    Projection {
        rows: Arc<OwnershipRows>,
        field: &'static str,
        non_null: bool,
    },
    Union(Vec<Arc<OwnershipSet>>),
}
pub struct SupportOwnership {
    pub corpus_input: usize,
    pub corpus: Arc<OwnershipRows>,
    pub distributions_input: usize,
    pub distributions: Arc<OwnershipRows>,
}
fn column(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn equal(left: (usize, &'static str), right: (usize, &'static str)) -> ScopePredicate {
    ScopePredicate::Equal(column(left.0, left.1), column(right.0, right.1))
}
fn typed<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    input_binding(inputs, &ValidationInput::of::<R>(&["id"]))
}
fn reserve(
    inputs: &[ValidationInput],
    model: &ValidatedModel,
    extra: usize,
    budget: &ResourceBudget,
) -> Result<Box<dyn Reservation>, ModelError> {
    let fields = inputs.iter().try_fold(0usize, |count, input| {
        let relation = model
            .relation(input.name())
            .ok_or(ModelError::Schema("admission scope relation absent"))?;
        count
            .checked_add(relation.fields().len())
            .ok_or(ModelError::Schema("admission program size overflow"))
    })?;
    let bytes = fields
        .checked_mul(1024)
        .and_then(|bytes| bytes.checked_add(inputs.len().checked_mul(512)?))
        .and_then(|bytes| bytes.checked_add(extra.checked_mul(4096)?))
        .and_then(|bytes| bytes.checked_add(65536))
        .ok_or(ModelError::Schema("admission program size overflow"))?;
    budget.reserve("admission-scope-program-construction", bytes)
}
impl ExecutionScope {
    pub fn program(
        &self,
        inputs: Vec<ValidationInput>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<ExecutionProgram, ModelError> {
        let construction = reserve(
            &inputs,
            model,
            self.memberships.len() + self.joins.len(),
            budget,
        )?;
        let root = input_binding(&inputs, &self.root)?;
        let mut ports = inputs
            .iter()
            .enumerate()
            .map(|(input, _)| ScopePort {
                input,
                virtual_owner: false,
            })
            .collect::<Vec<_>>();
        let root_namespace = ports.len();
        ports.push(ScopePort {
            input: root,
            virtual_owner: true,
        });
        let mut rules = forward_rules(&inputs, model)?;
        rules.push(ScopeRule::Pairs {
            source: root_namespace,
            target: root,
            rows: vec![root],
            predicates: vec![],
            source_key: column(0, "id"),
            target_key: column(0, "id"),
        });
        for (member, field, owner) in &self.memberships {
            rules.push(ScopeRule::Reference {
                source: input_binding(&inputs, member)?,
                field,
                target: input_binding(&inputs, owner)?,
                direction: ScopeDirection::OwnedReverse,
                list: false,
            });
        }
        for join in &self.joins {
            let source = input_binding(&inputs, &join.source)?;
            let member = input_binding(&inputs, &join.member)?;
            let mut rows = vec![source, member];
            let mut predicates = vec![equal((0, join.source_key), (1, join.member_key))];
            match join.context {
                None => {}
                Some(ExecutionMembershipContext::EnrichedInvocation) => {
                    rows.extend([
                        typed::<AnalysisInvocation>(&inputs)?,
                        typed::<AssertionQualification>(&inputs)?,
                    ]);
                    predicates.extend([
                        equal((2, "id"), (0, "invocation")),
                        equal((3, "id"), (1, "qualification")),
                        equal((3, "context"), (2, "context")),
                    ]);
                }
                Some(ExecutionMembershipContext::EnrichedContextItem) => {
                    rows.extend([
                        typed::<ContextExecution>(&inputs)?,
                        typed::<AnalysisInvocation>(&inputs)?,
                        typed::<AssertionQualification>(&inputs)?,
                    ]);
                    predicates.extend([
                        equal((2, "id"), (0, "execution")),
                        equal((3, "id"), (2, "invocation")),
                        equal((4, "id"), (1, "qualification")),
                        equal((4, "context"), (3, "context")),
                    ]);
                }
            }
            rules.push(ScopeRule::Pairs {
                source: if source == root {
                    root_namespace
                } else {
                    source
                },
                target: member,
                rows,
                predicates,
                source_key: column(0, "id"),
                target_key: column(1, "id"),
            });
        }
        Ok(ExecutionProgram {
            program: ScopeProgram {
                inputs,
                ports,
                rules,
            },
            root,
            root_namespace,
            _construction: construction,
        })
    }
}
fn owned<M: Record, O: Record>(
    inputs: &[ValidationInput],
    rules: &mut Vec<ScopeRule>,
    field: &'static str,
) -> Result<(), ModelError> {
    for (member, _) in inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == TypeId::of::<M>())
    {
        if let Some(owner) = field_target(inputs, member, TypeId::of::<O>())? {
            rules.push(ScopeRule::Reference {
                source: member,
                field,
                target: owner,
                direction: ScopeDirection::OwnedReverse,
                list: false,
            });
        }
    }
    Ok(())
}
fn projection(input: usize, field: &'static str, non_null: bool) -> Arc<OwnershipSet> {
    Arc::new(OwnershipSet::Projection {
        rows: Arc::new(OwnershipRows::Selected(input)),
        field,
        non_null,
    })
}
fn union(parts: Vec<Arc<OwnershipSet>>) -> Arc<OwnershipSet> {
    Arc::new(OwnershipSet::Union(parts))
}
impl SupportScope {
    pub fn program(
        &self,
        inputs: Vec<ValidationInput>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<SupportProgram, ModelError> {
        let construction = reserve(&inputs, model, self.source_inputs.len() + 16, budget)?;
        let root = input_binding(&inputs, &self.assertion)?;
        let support = input_binding(&inputs, &self.support)?;
        let ports = inputs
            .iter()
            .enumerate()
            .map(|(input, _)| ScopePort {
                input,
                virtual_owner: false,
            })
            .collect::<Vec<_>>();
        let mut rules = forward_rules(&inputs, model)?;
        rules.push(ScopeRule::Reference {
            source: support,
            field: "assertion",
            target: root,
            direction: ScopeDirection::OwnedReverse,
            list: false,
        });
        owned::<flow::FlowCallStep, flow::FlowCallPath>(&inputs, &mut rules, "path")?;
        owned::<types::TypeSequenceMember, types::TypeSequence>(&inputs, &mut rules, "sequence")?;
        owned::<types::TypedDictField, types::TypedDictFieldList>(&inputs, &mut rules, "list")?;
        owned::<types::CallableParameter, types::CallableParameterList>(
            &inputs, &mut rules, "list",
        )?;
        let run = typed::<ProviderRun>(&inputs)?;
        let family = typed::<RunFamily>(&inputs)?;
        rules.push(ScopeRule::Pairs {
            source: run,
            target: family,
            rows: vec![family],
            predicates: vec![ScopePredicate::Code(
                column(0, "family"),
                self.family as i16,
            )],
            source_key: column(0, "run"),
            target_key: column(0, "id"),
        });
        let scopes = typed::<CoverageScope>(&inputs)?;
        let sources = typed::<SourceArtifact>(&inputs)?;
        let distributions_input = typed::<InputDistribution>(&inputs)?;
        let corpus_input = typed::<CorpusLibrary>(&inputs)?;
        let mut owner_inputs = vec![projection(run, "input", false)];
        for source in &self.source_inputs {
            let index = input_binding(&inputs, source)?;
            let relation = model
                .relation(inputs[index].name())
                .ok_or(ModelError::Schema("support source input relation absent"))?;
            for field in relation.fields().iter().filter(|field| {
                !field.list()
                    && field
                        .target()
                        .is_some_and(|(kind, _)| kind == TypeId::of::<InputRevision>())
            }) {
                owner_inputs.push(projection(index, field.name(), false));
            }
        }
        owner_inputs.push(projection(scopes, "input_input", true));
        let owners = union(owner_inputs);
        let releases = projection(scopes, "release_release", true);
        let artifact_inputs = projection(sources, "input", false);
        let released_distributions = Arc::new(OwnershipRows::Full {
            input: distributions_input,
            filters: vec![OwnershipFilter {
                field: "release",
                values: releases.clone(),
            }],
        });
        let needed_libraries = union(vec![
            artifact_inputs.clone(),
            Arc::new(OwnershipSet::Projection {
                rows: released_distributions,
                field: "input",
                non_null: false,
            }),
        ]);
        let corpus = Arc::new(OwnershipRows::Full {
            input: corpus_input,
            filters: vec![
                OwnershipFilter {
                    field: "corpus",
                    values: owners.clone(),
                },
                OwnershipFilter {
                    field: "library",
                    values: needed_libraries,
                },
            ],
        });
        let needed_inputs = union(vec![
            artifact_inputs,
            owners,
            Arc::new(OwnershipSet::Projection {
                rows: corpus.clone(),
                field: "library",
                non_null: false,
            }),
        ]);
        let distributions = Arc::new(OwnershipRows::Full {
            input: distributions_input,
            filters: vec![
                OwnershipFilter {
                    field: "release",
                    values: releases,
                },
                OwnershipFilter {
                    field: "input",
                    values: needed_inputs,
                },
            ],
        });
        let ownership = SupportOwnership {
            corpus_input,
            corpus,
            distributions_input,
            distributions,
        };
        Ok(SupportProgram {
            program: ScopeProgram {
                inputs,
                ports,
                rules,
            },
            root,
            ownership,
            _construction: construction,
        })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn execution_and_support_programs_fit_all_current_declared_contracts() {
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let model = crate::domain::model().unwrap();
        let mut executions = 0;
        let mut supports = 0;
        for invariant in model.invariants() {
            let check = (invariant.create)(&budget);
            if let Some(scope) = check.execution_scope() {
                let declared = scope
                    .program(invariant.inputs.clone(), &model, &budget)
                    .unwrap();
                declared.program.validate(&model).unwrap();
                assert!(declared.program.ports[declared.root_namespace].virtual_owner);
                assert_eq!(
                    declared.program.ports[declared.root_namespace].input,
                    declared.root
                );
                executions += 1;
            }
            if let Some(scope) = check.support_scope() {
                let declared = scope
                    .program(invariant.inputs.clone(), &model, &budget)
                    .unwrap();
                declared.program.validate(&model).unwrap();
                assert_eq!(
                    declared.program.inputs[declared.root].type_id(),
                    scope.assertion.type_id()
                );
                assert_eq!(
                    declared.program.inputs[declared.ownership.corpus_input].type_id(),
                    TypeId::of::<CorpusLibrary>()
                );
                assert_eq!(
                    declared.program.inputs[declared.ownership.distributions_input].type_id(),
                    TypeId::of::<InputDistribution>()
                );
                supports += 1;
            }
        }
        assert!(executions > 0 && supports > 0);
        assert_eq!(budget.reserved(), 0);
    }
}
