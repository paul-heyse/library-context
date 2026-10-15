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
    ContentHash::of(b"artifact-admission/v1;facts-producer-coverage;exact-references;applicable-invariants;profile-frontier;canonical-entity-assertion-derivations;manifest-outcomes;physical-completion")
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
        let names = bindings
            .iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view.relation.as_str())
            .collect::<BTreeSet<_>>();
        let checks = model.admission_candidates_for_scope(&names)?;
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
