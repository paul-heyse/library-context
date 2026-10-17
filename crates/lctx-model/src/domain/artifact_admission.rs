//! Durable admission evidence is data, never a constructor for an admitted artifact.
//! Compiler storage earns its private owner by executing the applicable production gates.
use super::{self as d, ContentHash, KeySink, ModelError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SelectedInput {
    Completed {
        source: d::analysis::sources::SourceSnapshot,
        requested: Option<d::stages::PublicationBoundary>,
        resolved: Option<d::stages::PublicationBoundary>,
    },
    NotRequested {
        declaration: ContentHash,
    },
    /// Missing non-root premises are irrelevant only after the declared applicability
    /// roots have actually reached complete-empty EOF. This is never a missing input.
    EmptyDomain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Execution {
    WholeInput,
    Scoped,
    EmptyDomain,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationExecution {
    pub name: String,
    pub revision: u32,
    pub definition: ContentHash,
    pub factory: Option<ContentHash>,
    pub execution: Execution,
    pub inputs: Vec<SelectedInput>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAdmissionEvidence {
    pub version: u32,
    pub model: ContentHash,
    pub bindings: ContentHash,
    pub manifest: d::graph::Manifest,
    pub invariants: Vec<ValidationExecution>,
    pub frontier: Vec<ValidationExecution>,
    /// Fixed production pipeline version covering facts and producer coverage,
    /// references, applicable invariants/frontier, graph derivations and manifest outcomes.
    pub production_contract: ContentHash,
}
pub fn production_contract() -> ContentHash {
    ContentHash::of(b"artifact-admission/v1;facts-producer-coverage;exact-references;exact-root-epoch-invariant-instances;finalized-analysis-owner-recipes;profile-frontier;canonical-entity-assertion-derivations;manifest-outcomes;physical-completion")
}
pub fn frontier_checks(
    frontier: d::admission::Frontier,
) -> Result<&'static [&'static str], ModelError> {
    use d::admission::Frontier;
    match frontier {
        Frontier::Facts | Frontier::Normalized => Ok(&[]),
        Frontier::Analysis => Ok(&["complete_analysis_frontier"]),
        Frontier::Catalog => Ok(&["complete_analysis_frontier", "complete_catalog_frontier"]),
        Frontier::Conformance => Err(ModelError::Frontier(
            "diagnostic conformance is not a complete artifact".into(),
        )),
    }
}
impl ArtifactAdmissionEvidence {
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("complete-artifact-admission-evidence/v1");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    /// Checks present executable eligibility and exact selected metadata. Deserialization
    /// does not establish that the original visits occurred; trusted persistence and its
    /// protected exact physical completion supply that provenance.
    pub fn validate_current(
        &self,
        model: &d::ValidatedModel,
        bindings: &[d::completed::CompletedBinding],
        roots: &d::completed_identity::CompletedFamilyRoots,
        budget: &d::resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        self.manifest.validate()?;
        if self.version != 1
            || self.model != model.digest()
            || self.production_contract != production_contract()
            || self.bindings != d::completed::binding_inventory_identity(bindings)?
            || self.manifest.completed_state
                != d::completed::CompletedStateIdentity::from_roots(roots)?
        {
            return Err(ModelError::Conflict(
                "artifact admission complete current contract/physical binding",
            ));
        }
        let mut referrers = Vec::new();
        let mut roles = BTreeSet::new();
        for binding in bindings {
            let relation = model
                .relation(&binding.view.relation)
                .ok_or(ModelError::Schema("admission referrer relation"))?;
            let prefix = binding
                .boundary
                .as_deref()
                .map(|boundary| {
                    d::stages::PublicationBoundary::from_name(boundary).ok_or(ModelError::Conflict(
                        "admission referrer publication boundary",
                    ))
                })
                .transpose()?;
            if roles.insert((relation.name(), prefix)) {
                let input = d::ValidationInput::of_relation(relation, &["id"]);
                referrers.push(if let Some(epoch) = prefix {
                    input.at_epoch(epoch)
                } else {
                    input
                });
            }
        }
        let checks = model.admission_candidates_for_views(&referrers)?;
        if checks.len() != self.invariants.len() {
            return Err(ModelError::Conflict(
                "artifact admission applicable invariant ledger",
            ));
        }
        for (check, actual) in checks.iter().zip(&self.invariants) {
            let factory = model
                .factory_contract(check.identity())
                .map(|factory| factory.identity())
                .transpose()?;
            if check.name != actual.name
                || check.revision != actual.revision
                || check.digest() != actual.definition
                || factory != actual.factory
            {
                return Err(ModelError::Conflict(
                    "artifact admission current invariant definition",
                ));
            }
            validate_inputs(
                model,
                &check.inputs,
                actual,
                bindings,
                self.manifest.profile,
            )?;
            if actual.execution == Execution::EmptyDomain {
                let kernel = (check.create)(budget);
                let roots = kernel
                    .execution_scope()
                    .map(|scope| vec![scope.root])
                    .or_else(|| kernel.normalization_scope().map(|scope| scope.roots()))
                    .or_else(|| kernel.projection_scope().map(|scope| scope.roots()))
                    .ok_or(ModelError::Conflict(
                        "non-scoped check cannot claim empty applicability",
                    ))?;
                for root in roots {
                    let position = check
                        .inputs
                        .iter()
                        .position(|input| {
                            input.name() == root.name() && input.prefix() == root.prefix()
                        })
                        .ok_or(ModelError::Conflict(
                            "empty applicability root absent from declared roles",
                        ))?;
                    match &actual.inputs[position] {
                        SelectedInput::Completed { source, .. } if source.rows() == 0 => {}
                        SelectedInput::NotRequested { .. } => {}
                        _ => {
                            return Err(ModelError::Conflict(
                                "empty applicability needs complete-empty declared roots",
                            ));
                        }
                    }
                }
            }
        }
        let ids = frontier_checks(self.manifest.frontier)?;
        if ids.len() != self.frontier.len() {
            return Err(ModelError::Conflict(
                "artifact admission exact frontier ledger",
            ));
        }
        for (id, actual) in ids.iter().zip(&self.frontier) {
            let check = model.publication_check(id)?;
            if actual.name != *id
                || actual.revision != check.revision
                || actual.definition != check.digest()
                || actual.factory
                    != model
                        .factory_contract(check.identity())
                        .map(|factory| factory.identity())
                        .transpose()?
                || actual.execution != Execution::WholeInput
            {
                return Err(ModelError::Conflict(
                    "artifact admission frontier definition",
                ));
            }
            validate_inputs(
                model,
                &check.inputs,
                actual,
                bindings,
                self.manifest.profile,
            )?;
        }
        Ok(())
    }
}
fn validate_inputs(
    model: &d::ValidatedModel,
    declared: &[d::ValidationInput],
    actual: &ValidationExecution,
    bindings: &[d::completed::CompletedBinding],
    profile: d::stages::Profile,
) -> Result<(), ModelError> {
    if declared.len() != actual.inputs.len() {
        return Err(ModelError::Conflict(
            "artifact admission ordered input roles",
        ));
    }
    for (input, evidence) in declared.iter().zip(&actual.inputs) {
        match evidence {
            SelectedInput::Completed {
                source,
                requested,
                resolved,
            } => {
                if *requested != input.prefix()
                    || input
                        .prefix()
                        .is_some_and(|prefix| resolved.is_some_and(|actual| actual != prefix))
                    || source.model() != model.digest()
                    || source.relation() != input.name()
                    || !bindings.iter().any(|binding| {
                        binding.source == *source
                            && binding.boundary.as_deref()
                                == resolved.map(|boundary| boundary.name())
                    })
                {
                    return Err(ModelError::Conflict(
                        "artifact admission exact resolved input binding",
                    ));
                }
            }
            SelectedInput::NotRequested { declaration } => {
                let flow = d::producer_contract::facts::ty_flow();
                if profile != d::stages::Profile::Catalog
                    || *declaration != flow.identity()
                    || !flow
                        .outputs
                        .iter()
                        .any(|relation| relation.name() == input.name())
                {
                    return Err(ModelError::Conflict(
                        "artifact admission undeclared unrequested input",
                    ));
                }
            }
            SelectedInput::EmptyDomain if actual.execution == Execution::EmptyDomain => {}
            SelectedInput::EmptyDomain => {
                return Err(ModelError::Conflict(
                    "artifact admission missing requested input",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod exact_epoch_controls {
    use super::*;
    use d::{Record, Relation, stages::PublicationBoundary as P};

    #[test]
    fn artifact_ledger_retains_each_root_epoch_and_refuses_other_epoch_substitution() {
        let model = d::model().unwrap();
        let budget = d::resources::ResourceBudget::fixed(16 << 20).unwrap();
        let mut bindings = Vec::new();
        for epoch in [P::Facts, P::Local] {
            for relation in [
                Relation::of::<d::conditions::Condition>(),
                Relation::of::<d::conditions::ConditionNode>(),
            ] {
                let view = d::completed::CompletedView::new(
                    relation.name().into(),
                    [ContentHash::of(epoch.name().as_bytes())].into(),
                    1,
                )
                .unwrap();
                bindings.push(d::completed::CompletedBinding {
                    boundary: Some(epoch.name().into()),
                    source: d::analysis::sources::SourceSnapshot::of_completed_view(
                        &relation,
                        model.digest(),
                        &view,
                    )
                    .unwrap(),
                    view,
                    configuration: None,
                });
            }
        }
        let referrers = [P::Facts, P::Local, P::Facts].map(|epoch| {
            d::ValidationInput::of::<d::conditions::Condition>(&["id"]).at_epoch(epoch)
        });
        let checks = model.admission_candidates_for_views(&referrers).unwrap();
        assert_eq!(
            checks.len(),
            2,
            "same check and exact root role deduplicate"
        );
        assert_eq!(checks[0].name, checks[1].name);
        assert_ne!(checks[0].digest(), checks[1].digest());
        assert!(std::sync::Arc::ptr_eq(&checks[0].create, &checks[1].create));
        for (check, epoch) in checks.iter().zip([P::Facts, P::Local]) {
            assert!(
                check
                    .inputs
                    .iter()
                    .all(|input| input.prefix() == Some(epoch))
            );
            let mut kernel = (check.create)(&budget);
            let node = if epoch == P::Facts {
                d::conditions::ConditionNode::False
            } else {
                d::conditions::ConditionNode::True
            };
            for input in &check.inputs {
                let batch = if input.name() == d::conditions::ConditionNode::NAME {
                    d::conditions::ConditionNode::encode(std::slice::from_ref(&node)).unwrap()
                } else {
                    d::conditions::Condition::encode(&[d::conditions::Condition {
                        root: node.id(),
                    }])
                    .unwrap()
                };
                kernel.visit_input(input, &batch).unwrap();
            }
            kernel.finish().unwrap();
        }
        let roots = d::completed_identity::CompletedFamilyRoots::new(
            d::recovery_closure::COMPLETED_FAMILIES.map(|family| {
                d::completed_identity::CompletedFamilyHasher::new(family)
                    .unwrap()
                    .finish()
            }),
        )
        .unwrap();
        let mut evidence = ArtifactAdmissionEvidence {
            version: 1,
            model: model.digest(),
            bindings: d::completed::binding_inventory_identity(&bindings).unwrap(),
            manifest: d::graph::Manifest {
                admission_contract: ContentHash::of(b"epoch-ledger-admission-control"),
                completed_state: d::completed::CompletedStateIdentity::from_roots(&roots).unwrap(),
                format_version: d::graph::ARTIFACT_FORMAT_VERSION,
                frontier: d::admission::Frontier::Facts,
                profile: d::stages::Profile::Behavioral,
                captures: vec![],
                semantic_contract: model.digest(),
                producers: vec![],
                settings: ContentHash::of(b"epoch-ledger-control"),
                families: vec![],
                required_outcomes: vec![],
                outcomes: vec![],
                originals: vec![],
                projections: vec![],
                embeddings: vec![],
            },
            invariants: checks
                .iter()
                .map(|check| ValidationExecution {
                    name: check.name.into(),
                    revision: check.revision,
                    definition: check.digest(),
                    factory: model
                        .factory_contract(check.identity())
                        .map(|factory| factory.identity().unwrap()),
                    execution: Execution::WholeInput,
                    inputs: check
                        .inputs
                        .iter()
                        .map(|input| {
                            let binding = bindings
                                .iter()
                                .find(|binding| {
                                    binding.view.relation == input.name()
                                        && binding.boundary.as_deref()
                                            == input.prefix().map(P::name)
                                })
                                .unwrap();
                            SelectedInput::Completed {
                                source: binding.source.clone(),
                                requested: input.prefix(),
                                resolved: input.prefix(),
                            }
                        })
                        .collect(),
                })
                .collect(),
            frontier: vec![],
            production_contract: production_contract(),
        };
        evidence
            .validate_current(&model, &bindings, &roots, &budget)
            .unwrap();
        let pristine = evidence.clone();
        let mut missing_check = evidence.clone();
        missing_check.invariants.pop();
        assert!(
            missing_check
                .validate_current(&model, &bindings, &roots, &budget)
                .is_err()
        );
        let replacement = bindings
            .iter()
            .find(|binding| {
                binding.view.relation == d::conditions::ConditionNode::NAME
                    && binding.boundary.as_deref() == Some(P::Local.name())
            })
            .unwrap();
        evidence.invariants[0].inputs[0] = SelectedInput::Completed {
            source: replacement.source.clone(),
            requested: Some(P::Facts),
            resolved: Some(P::Local),
        };
        assert!(
            evidence
                .validate_current(&model, &bindings, &roots, &budget)
                .is_err(),
            "another authentic epoch cannot satisfy the Facts companion"
        );
        evidence = pristine.clone();
        bindings.retain(|binding| {
            !(binding.view.relation == d::conditions::ConditionNode::NAME
                && binding.boundary.as_deref() == Some(P::Facts.name()))
        });
        evidence.bindings = d::completed::binding_inventory_identity(&bindings).unwrap();
        assert!(
            evidence
                .validate_current(&model, &bindings, &roots, &budget)
                .is_err(),
            "missing one companion cannot fall back to another epoch"
        );

        let canonical_bindings = bindings
            .iter()
            .filter(|binding| binding.boundary.as_deref() == Some(P::Local.name()))
            .cloned()
            .map(|mut binding| {
                binding.boundary = None;
                binding
            })
            .collect::<Vec<_>>();
        let canonical = model
            .admission_candidates_for_views(&[d::ValidationInput::of::<d::conditions::Condition>(
                &["id"],
            )])
            .unwrap();
        assert_eq!(
            canonical.len(),
            1,
            "canonical imported roots do not invent publication epochs"
        );
        let check = &canonical[0];
        let mut detached = pristine;
        detached.bindings = d::completed::binding_inventory_identity(&canonical_bindings).unwrap();
        detached.invariants = vec![ValidationExecution {
            name: check.name.into(),
            revision: check.revision,
            definition: check.digest(),
            factory: model
                .factory_contract(check.identity())
                .map(|factory| factory.identity().unwrap()),
            execution: Execution::WholeInput,
            inputs: check
                .inputs
                .iter()
                .map(|input| SelectedInput::Completed {
                    source: canonical_bindings
                        .iter()
                        .find(|binding| binding.view.relation == input.name())
                        .unwrap()
                        .source
                        .clone(),
                    requested: None,
                    resolved: None,
                })
                .collect(),
        }];
        detached
            .validate_current(&model, &canonical_bindings, &roots, &budget)
            .unwrap();
        let declaration =
            d::ValidationInput::of::<d::conditions::ConditionNode>(&["id"]).at_epoch(P::Facts);
        let source = canonical_bindings
            .iter()
            .find(|binding| binding.view.relation == declaration.name())
            .unwrap();
        let alias = ValidationExecution {
            inputs: vec![SelectedInput::Completed {
                source: source.source.clone(),
                requested: Some(P::Facts),
                resolved: None,
            }],
            ..detached.invariants[0].clone()
        };
        validate_inputs(
            &model,
            &[declaration],
            &alias,
            &canonical_bindings,
            d::stages::Profile::Behavioral,
        )
        .unwrap();
    }
}
