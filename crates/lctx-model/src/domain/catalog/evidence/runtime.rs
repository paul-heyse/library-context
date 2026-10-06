//! Exact earlier receiver locations and fresh constructor binding are distinct from heap state.
use super::{
    build::{EvidenceData, EvidenceOutput, invalid, need, qualification},
    *,
};
use crate::Domain;
use crate::domain::{
    local_fields::{FieldLocation, FieldLocationCandidate},
    normalized::Rows,
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "catalog_field_location_links")]
pub struct FieldLocationLink {
    #[model(key)]
    pub assessment: Id<FieldAccessAssessment>,
    #[model(key)]
    pub location: Id<FieldLocation>,
    #[model(key)]
    pub candidate: Id<FieldLocationCandidate>,
    pub phase: calls::CallPhase,
    pub applicability: Knowledge,
    pub state: obligation::ObligationKind,
}
/// Earlier normalized constructor target/binding evidence is a source candidate, not execution.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "catalog_constructor_candidate_links")]
pub struct ConstructorCandidateLink {
    #[model(key)]
    pub association: Id<ScenarioAssociation>,
    #[model(key)]
    pub constructor: Id<CatalogConstructor>,
    #[model(key)]
    pub attempt: Id<normalized::bindings::CallBindingAttempt>,
    pub target: Id<calls::CallTarget>,
    pub receiver: Id<calls::Receiver>,
    pub variant: Option<Id<normalized::callables::SignatureVariant>>,
    pub phase: calls::CallPhase,
    pub applicability: Knowledge,
    pub state: obligation::ObligationKind,
}
#[macro_export]
macro_rules! catalog_runtime_inputs {($m:ident)=>{$m!{
 local_invocations:$crate::domain::analysis::local::Invocation,local_outcomes:$crate::domain::analysis::local::AnalysisOutcome,
 source_invocations:$crate::domain::analysis::source_call::Invocation,source_outcomes:$crate::domain::analysis::source_call::AnalysisOutcome,
 locations:$crate::domain::local_fields::FieldLocation,location_candidates:$crate::domain::local_fields::FieldLocationCandidate,

}};}
macro_rules! data {($($f:ident:$ty:ty,)*)=>{pub struct RuntimeData{$(pub $f:Rows<$ty>,)*}impl RuntimeData{pub fn new(b:&resources::ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}pub fn stage_inputs()->Vec<stages::RelationUse>{vec![$(stages::RelationUse::completed::<$ty>(),)*]}}};}
crate::catalog_runtime_inputs!(data);
impl RuntimeData {
    pub fn lower(&self) -> super::frames::LowerFrames<'_> {
        super::frames::LowerFrames {
            local: &self.local_invocations,
            local_outcomes: &self.local_outcomes,
            source: &self.source_invocations,
            source_outcomes: &self.source_outcomes,
        }
    }
}
pub(super) fn derive(d: &EvidenceData, out: &mut EvidenceOutput) -> Result<(), ModelError> {
    for candidate in d.runtime.location_candidates.iter() {
        let location = need(&d.runtime.locations, candidate.location)?;
        let invocation = need(&d.runtime.local_invocations, location.invocation)?;
        if invocation.definition != local_semantics::definition().1.id()
            || invocation.subject.is_some()
        {
            return Err(invalid("C1 receiver location has a foreign Local owner"));
        }
        let placement = need(&d.core.placements, location.placement)?;
        let Some(attribute) = placement.parent else {
            return Err(invalid("C1 receiver location has no attribute parent"));
        };
        let q = need(&d.local_qualifications, location.qualification)?;
        if q.context != invocation.context {
            return Err(invalid("C1 receiver location has a foreign qualification"));
        }
        for access in out.accesses.iter() {
            if access.occurrence != attribute
                || access.field != candidate.field
                || !matches!(
                    access.kind,
                    FieldAccessKind::Read | FieldAccessKind::Augment
                )
            {
                continue;
            }
            let owner = need(&d.core.ownership, access.owner)?;
            let option = need(&d.catalog.options, access.option)?;
            let member = need(&d.catalog.members, option.member)?;
            let aq = qualification(d, access.qualification)?;
            let occurrence = need(&d.core.occurrences, access.occurrence)?;
            if owner.entity != location.owner
                || member.input != invocation.input
                || aq.context != invocation.context
                || need(&d.core.artifacts, occurrence.source)?.input != invocation.input
            {
                continue;
            }
            out.field_locations.insert(FieldLocationLink {
                assessment: access.id(),
                location: location.id(),
                candidate: candidate.id(),
                phase: location.phase,
                applicability: Knowledge::Unknown,
                state: obligation::ObligationKind::HeapFieldStateUnavailable,
            })?;
        }
    }
    for association in out.associations.iter() {
        let alternative = need(&d.facts.alternatives, association.alternative)?;
        let event = need(&d.facts.events, alternative.event)?;
        let target = need(
            &d.facts.raw_targets,
            need(&d.facts.alternative_sources, alternative.source)?.target(),
        )?;
        if !matches!(target.phase, calls::CallPhase::New | calls::CallPhase::Init)
            || target.site != event.site
            || qualification(d, target.qualification)?.context != event.context
        {
            continue;
        }
        for constructor in d.catalog.constructors.iter() {
            let class = need(&d.catalog.classes, constructor.class)?;
            let callable = need(&d.catalog.callables, constructor.callable)?;
            if class.member != association.member
                || callable.member != association.member
                || match constructor.kind {
                    ConstructorKind::New => target.phase != calls::CallPhase::New,
                    ConstructorKind::Init => target.phase != calls::CallPhase::Init,
                }
            {
                continue;
            }
            let assessment = need(&d.core.assessments, callable.assessment)?;
            if assessment.context != event.context
                || alternative.entity
                    != Some(
                        normalized::entities::EntityRef::Callable {
                            callable: assessment.callable,
                        }
                        .id(),
                    )
            {
                continue;
            }
            // Constructor class correspondence comes from the target's explicit receiver class, never name.
            let Some(receiver_class) = target.receiver_class else {
                continue;
            };
            let class_ref = normalized::entities::EntityRef::Class { class: class.class }.id();
            if !d.core.resolutions.iter().any(|r| {
                r.symbol == receiver_class
                    && r.context == event.context
                    && r.status == ResolutionStatus::Resolved
                    && r.entity == Some(class_ref)
            }) {
                continue;
            }
            let member = need(&d.catalog.members, association.member)?;
            let occurrence = need(&d.core.occurrences, event.site)?;
            if member.input != need(&d.core.artifacts, occurrence.source)?.input {
                continue;
            }
            for attempt in d
                .facts
                .attempts
                .iter()
                .filter(|a| a.alternative == alternative.id())
            {
                if attempt.event != event.id() {
                    return Err(invalid(
                        "C1 constructor attempt changes exact target or receiver",
                    ));
                }
                if attempt.receiver != target.receiver {
                    continue;
                }
                if attempt.effective.is_some_and(|e| e != assessment.id()) {
                    continue;
                }
                if let Some(variant) = attempt.variant {
                    let variant = need(&d.core.variants, variant)?;
                    if variant.context != event.context
                        || variant.callable != Some(assessment.callable)
                        || variant.assessment != Some(assessment.id())
                    {
                        continue;
                    }
                }
                out.constructor_candidates
                    .insert(ConstructorCandidateLink {
                        association: association.id(),
                        constructor: constructor.id(),
                        attempt: attempt.id(),
                        target: target.id(),
                        receiver: target.receiver,
                        variant: attempt.variant,
                        phase: target.phase,
                        applicability: Knowledge::Unknown,
                        state: obligation::ObligationKind::HeapFieldStateUnavailable,
                    })?;
            }
        }
    }
    Ok(())
}
