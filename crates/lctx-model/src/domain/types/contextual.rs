//! Actual/Expected observations characterize a selected original expression. Retained
//! structure differences are not assignability decisions or runtime violations.
use crate::domain::{
    normalized::{
        contract_comparison::{ChargedResult, Difference, compare_terms},
        incoming_references::member_entities,
    },
    resources::ResourceBudget,
    selection::classification::ClassificationData,
    serving::ProofReference,
    *,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextualTypeDisplay {
    pub presentation: Id<super::TypePresentation>,
    pub term: Id<super::TypeTerm>,
    pub qualification: Id<assertion::AssertionQualification>,
    /// Native display is presentation only; it does not define type correspondence.
    pub display: String,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextualType {
    pub subject: Id<source::Occurrence>,
    pub artifact: Id<source::SourceArtifact>,
    pub start: i64,
    pub end: i64,
    pub context: Id<attribution::AnalysisContext>,
    pub actual: Vec<Id<super::TypeObservation>>,
    pub expected: Vec<Id<super::TypeObservation>>,
    pub actual_terms: Vec<Id<super::TypeTerm>>,
    pub expected_terms: Vec<Id<super::TypeTerm>>,
    pub presentations: Vec<ContextualTypeDisplay>,
    pub retained_structure: Difference,
    /// The trace does not expose a trustworthy error-recovery state here.
    pub error_recovery_known: bool,
    pub proof: Vec<ProofReference>,
}
pub fn explain(
    d: &ClassificationData,
    member: Id<catalog::CatalogMember>,
    budget: &ResourceBudget,
) -> Result<ChargedResult<Vec<ContextualType>>, ModelError> {
    let selected = d
        .source
        .catalog
        .members
        .get(member)
        .ok_or_else(|| ModelError::Invalid("contextual member absent".into()))?;
    if d.facts.type_observations.len() > 100_000 {
        return Err(ModelError::Limit {
            owner: "contextual-types",
            limit: "input observations",
            observed: d.facts.type_observations.len(),
            bound: 100_000,
        });
    }
    let mut charge = budget.reserve("contextual-type-result", 1024)?;
    let _scratch = budget.reserve(
        "contextual-type-closure-work",
        4096usize
            .saturating_mul(192)
            .saturating_add(d.facts.type_observations.len().saturating_mul(256)),
    )?;
    let entities = member_entities(d, member);
    let mut groups = std::collections::BTreeMap::<_, (Vec<_>, Vec<_>)>::new();
    for observation in d.facts.type_observations.iter().filter(|o| {
        matches!(
            o.role,
            super::TypeRole::Expected
                | super::TypeRole::Argument
                | super::TypeRole::AttributeBase
                | super::TypeRole::AssignmentValue
                | super::TypeRole::ReturnExpression
        )
    }) {
        let Some(q) = d.source.core.qualifications.get(observation.qualification) else {
            return Err(ModelError::Invalid(
                "contextual qualification absent".into(),
            ));
        };
        let Some(occurrence) = d.source.core.occurrences.get(observation.subject) else {
            return Err(ModelError::Invalid("contextual occurrence absent".into()));
        };
        if !d
            .source
            .core
            .artifacts
            .get(occurrence.source)
            .is_some_and(|a| a.input == selected.input)
            || !d
                .source
                .core
                .ownership
                .iter()
                .any(|o| o.occurrence == observation.subject && entities.contains(&o.entity))
        {
            continue;
        }
        if !d.facts.type_supports.iter().any(|s| {
            s.assertion == observation.id()
                && d.facts
                    .runs
                    .get(s.run)
                    .is_some_and(|r| r.context == q.context && r.input == selected.input)
        }) {
            continue;
        }
        let group = groups.entry((occurrence.id(), q.context)).or_default();
        if observation.role == super::TypeRole::Expected {
            group.1.push(observation.id())
        } else {
            group.0.push(observation.id())
        }
    }
    let mut rows = Vec::new();
    for ((subject, context), (actual, expected)) in groups {
        let occurrence = d
            .source
            .core
            .occurrences
            .get(subject)
            .expect("grouped occurrence");
        let terms = |ids: &[Id<super::TypeObservation>]| {
            let mut terms = ids
                .iter()
                .filter_map(|id| d.facts.type_observations.get(*id))
                .map(|o| o.term)
                .collect::<Vec<_>>();
            terms.sort();
            terms.dedup();
            terms
        };
        let actual_terms = terms(&actual);
        let expected_terms = terms(&expected);
        let retained_structure = compare_terms(d, &actual_terms, &expected_terms);
        let mut presentations = Vec::new();
        for presentation in d
            .facts
            .type_presentations
            .iter()
            .filter(|p| actual_terms.contains(&p.term) || expected_terms.contains(&p.term))
        {
            if !d
                .source
                .core
                .qualifications
                .get(presentation.qualification)
                .is_some_and(|q| q.context == context)
            {
                continue;
            }
            if !d.facts.type_presentation_supports.iter().any(|s| {
                s.assertion == presentation.id()
                    && d.facts
                        .runs
                        .get(s.run)
                        .is_some_and(|r| r.context == context && r.input == selected.input)
            }) {
                continue;
            }
            if presentations.len() >= 16 {
                return Err(ModelError::Limit {
                    owner: "contextual-types",
                    limit: "presentations per expression",
                    observed: presentations.len() + 1,
                    bound: 16,
                });
            }
            charge.try_resize(
                charge
                    .size()
                    .saturating_add(presentation.heap_bytes())
                    .saturating_add(512),
            )?;
            presentations.push(ContextualTypeDisplay {
                presentation: presentation.id(),
                term: presentation.term,
                qualification: presentation.qualification,
                display: presentation.display.clone(),
                detail: presentation.detail.clone(),
            });
        }
        charge.try_resize(
            charge
                .size()
                .saturating_add((actual.len() + expected.len() + 1).saturating_mul(4096)),
        )?;
        let mut proof = vec![ProofReference::from_canonical(derivation::RowRef::of(
            subject,
        ))];
        for id in actual.iter().chain(&expected) {
            proof.push(ProofReference::from_canonical(derivation::RowRef::of(*id)));
            for support in d.facts.type_supports.iter().filter(|s| s.assertion == *id) {
                proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                    support.id(),
                )));
            }
        }
        for p in &presentations {
            proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                p.presentation,
            )));
        }
        rows.push(ContextualType {
            subject,
            artifact: occurrence.source,
            start: occurrence.start,
            end: occurrence.end,
            context,
            actual,
            expected,
            actual_terms,
            expected_terms,
            presentations,
            retained_structure,
            error_recovery_known: false,
            proof,
        });
    }
    rows.sort_by_key(|r| (r.artifact, r.start, r.end, r.context, r.subject));
    Ok(ChargedResult {
        value: rows,
        _charge: charge,
    })
}

pub fn definition() -> ContentHash {
    ContentHash::of(include_bytes!("contextual.rs"))
}
