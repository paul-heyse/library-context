//! Native rows refine source correspondence within the existing normalization owner.
use super::{
    entities::ResolutionStatus,
    links::*,
    relation_normalization::{RelationData, RelationOutput},
};
use crate::domain::{
    assertion::*, attribution::*, resources::ResourceBudget, ruff::*, source::*, *,
};
use std::collections::BTreeMap;
fn context(
    data: &RelationData,
    q: Id<AssertionQualification>,
) -> Result<Id<AnalysisContext>, ModelError> {
    data.facts
        .qualifications
        .get(q)
        .map(|q| q.context)
        .ok_or_else(|| ModelError::Invalid("native lexical qualification missing".into()))
}
fn supported(
    data: &RelationData,
    q: Id<AssertionQualification>,
    subject: Id<Occurrence>,
    attribution: Option<SupportAttribution>,
) -> bool {
    let Some(SupportAttribution {
        run,
        surface,
        evidence,
        origin,
        mode,
        fidelity,
    }) = attribution
    else {
        return false;
    };
    let Some(q) = data.facts.qualifications.get(q) else {
        return false;
    };
    let Some(run) = data.facts.runs.get(run) else {
        return false;
    };
    let Some(surface) = data.native_surfaces.get(surface) else {
        return false;
    };
    let Some(provider) = data.native_providers.get(run.provider) else {
        return false;
    };
    let Some(occurrence) = data.facts.occurrences.get(subject) else {
        return false;
    };
    let Some(artifact) = data.artifacts.get(occurrence.source) else {
        return false;
    };
    let scope_matches = match data.scopes.get(q.scope) {
        Some(CoverageScope::Artifact { artifact }) => *artifact == occurrence.source,
        Some(CoverageScope::Module { module }) => data
            .facts
            .modules
            .get(*module)
            .is_some_and(|m| m.source == occurrence.source),
        Some(CoverageScope::Input { input }) => *input == artifact.input,
        _ => false,
    };
    scope_matches
        && provider.tool == "ruff"
        && run.provider == surface.provider
        && surface.family == FactFamily::Lexical
        && run.context == q.context
        && run.input == artifact.input
        && q.assumptions == assumptions::AssumptionSet::empty_id()
        && q.modality == Modality::Definite
        && q.approximation == Approximation::Exact
        && q.condition == conditions::Diagram::always().id()
        && origin == Origin::AnalyzerAssertion
        && mode == ExtractionMode::NativeTraversal
        && fidelity == Fidelity::NativeStructural
        && data.native_evidence.get(evidence)
            == Some(&Evidence::Occurrence {
                occurrence: subject,
            })
}
pub(super) fn characterize(
    data: &RelationData,
    out: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let count = data.references.len()
        + data.ruff_bindings.len()
        + data.ruff_context_supports.len()
        + data.ruff_binding_supports.len()
        + data.declaration_syntax.len()
        + data.ruff_definition_supports.len()
        + out.reference_entity_candidates.len();
    let _indexes = budget.reserve(
        "native-lexical-correspondence-indexes",
        count.saturating_mul(1024),
    )?;
    let mut references = BTreeMap::<_, Vec<_>>::new();
    let mut bindings = BTreeMap::<_, Vec<_>>::new();
    let mut cs = BTreeMap::<_, Vec<_>>::new();
    let mut bs = BTreeMap::<_, Vec<_>>::new();
    let mut ds = BTreeMap::<_, Vec<_>>::new();
    let mut declarations = BTreeMap::<_, Vec<_>>::new();
    let mut candidates = BTreeMap::<_, Vec<_>>::new();
    for row in data.references.iter() {
        references
            .entry((row.read, context(data, row.qualification)?))
            .or_default()
            .push(row);
    }
    for row in data.ruff_bindings.iter() {
        bindings
            .entry((row.event, context(data, row.qualification)?))
            .or_default()
            .push(row);
    }
    for row in data.ruff_context_supports.iter() {
        cs.entry(row.assertion).or_default().push(row);
    }
    for row in data.ruff_binding_supports.iter() {
        bs.entry(row.assertion).or_default().push(row);
    }
    for row in data.ruff_definition_supports.iter() {
        ds.entry(row.assertion).or_default().push(row);
    }
    for row in data.declaration_syntax.iter() {
        declarations
            .entry((row.declaration, context(data, row.qualification)?))
            .or_default()
            .push(row);
    }
    for row in out.reference_entity_candidates.iter() {
        let Some(a) = out.reference_entity_assessments.get(row.assessment) else {
            continue;
        };
        if let Some(ReferenceEntityTarget::Binding { event, .. }) =
            out.reference_targets.get(row.target)
        {
            candidates
                .entry((a.reference, *event))
                .or_default()
                .push((row, a));
        }
    }
    for native in data
        .ruff_contexts
        .iter()
        .filter(|r| r.phase == ContextPhase::FinalReference)
    {
        let Some(event) = native.final_binding else {
            continue;
        };
        let ctx = context(data, native.qualification)?;
        for reference in references.get(&(native.subject, ctx)).into_iter().flatten() {
            for binding in bindings.get(&(event, ctx)).into_iter().flatten() {
                let site = data
                    .facts
                    .bindings
                    .get(event)
                    .map(|b| b.site)
                    .ok_or_else(|| ModelError::Invalid("native lexical event missing".into()))?;
                let context_support = cs
                    .get(&native.id())
                    .into_iter()
                    .flatten()
                    .find(|s| {
                        supported(data, native.qualification, native.subject, s.attribution())
                    })
                    .map(|s| s.id());
                let support = bs
                    .get(&binding.id())
                    .into_iter()
                    .flatten()
                    .find(|s| supported(data, binding.qualification, site, s.attribution()))
                    .map(|s| s.id());
                let hits = candidates.get(&(reference.id(), event));
                if let Some(hits) = hits {
                    for (candidate, assessment) in hits {
                        let admitted = context_support.is_some() && support.is_some();
                        out.reference_binding_characterizations.insert(
                            ReferenceBindingCharacterization {
                                reference: reference.id(),
                                native_context: native.id(),
                                binding: binding.id(),
                                context_support,
                                support,
                                candidate: Some(candidate.id()),
                                status: if admitted {
                                    assessment.status
                                } else {
                                    ResolutionStatus::Unresolved
                                },
                                reason: if admitted {
                                    assessment.reason
                                } else {
                                    LinkReason::UnsupportedNativeOrigin
                                },
                            },
                        )?;
                    }
                } else {
                    out.reference_binding_characterizations.insert(
                        ReferenceBindingCharacterization {
                            reference: reference.id(),
                            native_context: native.id(),
                            binding: binding.id(),
                            context_support,
                            support,
                            candidate: None,
                            status: ResolutionStatus::Unresolved,
                            reason: LinkReason::MissingCorrespondence,
                        },
                    )?;
                }
            }
        }
    }
    for native in data.ruff_definitions.iter() {
        for declaration in declarations
            .get(&(native.declaration, context(data, native.qualification)?))
            .into_iter()
            .flatten()
        {
            let support = ds
                .get(&native.id())
                .into_iter()
                .flatten()
                .find(|s| {
                    supported(
                        data,
                        native.qualification,
                        native.declaration,
                        s.attribution(),
                    )
                })
                .map(|s| s.id());
            let kind_matches = matches!(
                (declaration.kind, native.kind),
                (
                    crate::domain::syntax::DeclarationKind::Class,
                    RuffDefinitionKind::Class | RuffDefinitionKind::NestedClass
                ) | (
                    crate::domain::syntax::DeclarationKind::Function
                        | crate::domain::syntax::DeclarationKind::AsyncFunction,
                    RuffDefinitionKind::Function
                        | RuffDefinitionKind::NestedFunction
                        | RuffDefinitionKind::Method
                )
            );
            let admitted = support.is_some()
                && native.parent_location != NativeRelationLocation::Unlocated
                && native.parent == declaration.parent
                && kind_matches;
            out.declaration_native_characterizations
                .insert(DeclarationNativeCharacterization {
                    declaration: declaration.id(),
                    native_definition: native.id(),
                    support,
                    status: if admitted {
                        ResolutionStatus::Resolved
                    } else {
                        ResolutionStatus::Unresolved
                    },
                    reason: if admitted {
                        LinkReason::ExplicitIdentity
                    } else if support.is_none() {
                        LinkReason::UnsupportedNativeOrigin
                    } else {
                        LinkReason::MissingCorrespondence
                    },
                })?;
        }
    }
    Ok(())
}
