//! Pass C: observed producer-result arguments in official usage, never type compatibility.
use super::{
    build::{invalid, need},
    *,
};
use crate::domain::{
    analysis::{native::*, settings::AnalyticsConfiguration},
    assertion::{Approximation, AssertionQualification, Support},
    attribution::{FactFamily, Fidelity, Modality},
    calls::*,
    conditions::{BooleanOperation, Diagram, entry::EntryData},
    flow::*,
    input::{ArtifactUse, SourceRole},
    normalized::{Rows, bindings::*, callables::*, events::*},
    source::*,
    *,
};
use crate::{Domain, DomainSum};
#[macro_export]
macro_rules! structural_handoff_inputs {
    ($apply:ident) => {
        $apply! {
         attempts:$crate::domain::normalized::bindings::CallBindingAttempt,
         bindings:$crate::domain::normalized::bindings::CallBinding,
         sources:$crate::domain::calls::BindingSource,
         projections:$crate::domain::calls::BindingProjection,
         slots:$crate::domain::normalized::callables::SignatureSlot,
         syntax:$crate::domain::calls::CallSyntax,
         arguments:$crate::domain::calls::CallArgument,
         native:$crate::domain::analysis::native::NativeQualification,
        }
    };
}
macro_rules! inputs {($($f:ident:$ty:ty,)*)=>{
 pub struct Data {pub entry:EntryData,$(pub $f:Rows<$ty>,)*}
 impl Data {pub fn new(b:&resources::ResourceBudget)->Self{Self{entry:EntryData::new(b),$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{let e=self.entry.visit(n,b)?;$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(e)}pub fn inputs()->Vec<ValidationInput>{let mut v=EntryData::validation_inputs();v.extend([$(ValidationInput::of::<$ty>(&["id"]),)*]);v}}
};}
crate::structural_handoff_inputs!(inputs);
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "structural_handoff_values")]
pub enum ValueSource {
    #[model(code = 0)]
    Nested { expression: Id<Occurrence> },
    #[model(code = 1)]
    Named {
        observation: Id<FlowUseObservation>,
        support: Id<FlowUseSupport>,
        inventory: Id<flow_inventory::FlowUseInventoryObservation>,
        inventory_support: Id<flow_inventory::FlowUseInventorySupport>,
        reaching: Id<FlowReachingObservation>,
        reaching_support: Id<FlowReachingSupport>,
        definition: Id<FlowDefinitionObservation>,
        definition_support: Id<FlowDefinitionSupport>,
        region: Option<Id<FlowRegionObservation>>,
        region_support: Option<Id<FlowRegionSupport>>,
        coverage: Id<attribution::ProviderCoverage>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_handoff_assessments")]
pub struct Assessment {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub binding: Id<CallBinding>,
    pub argument: Id<Occurrence>,
    pub value: Option<Id<ValueSource>>,
    pub reason: Option<obligation::ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_handoff_occurrences")]
pub struct Handoff {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub consumer: Id<NormalizedCallAlternative>,
    #[model(key)]
    pub producer: Id<NormalizedCallAlternative>,
    #[model(key)]
    pub binding: Id<CallBinding>,
    #[model(key)]
    pub value: Id<ValueSource>,
    pub role: Id<ArtifactUse>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_handoff_groups")]
pub struct Group {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub seed: Id<normalized::entities::EntityRef>,
    #[model(key)]
    pub other: Id<normalized::entities::EntityRef>,
    #[model(key)]
    pub formal: Id<SignatureSlot>,
    #[model(key)]
    pub producer_phase: CallPhase,
    #[model(key)]
    pub consumer_phase: CallPhase,
    #[model(key)]
    pub producer_modality: Modality,
    #[model(key)]
    pub consumer_modality: Modality,
    pub occurrences: i64,
    pub retained: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_handoff_members")]
pub struct Member {
    #[model(key)]
    pub group: Id<Group>,
    #[model(key)]
    pub ordinal: i64,
    pub occurrence: Id<Handoff>,
}
fn same(a: &Occurrence, b: &Occurrence) -> bool {
    a.source == b.source
        && a.structural_path == b.structural_path
        && a.syntax_kind == b.syntax_kind
        && a.start == b.start
        && a.end == b.end
}
fn native(d: &Data, premise: NativeAssertionPremise, q: Id<AssertionQualification>) -> bool {
    d.native.iter().any(|n| {
        n.premise == premise.id()
            && n.qualification == q
            && n.family == FactFamily::Flow
            && n.fidelity == Fidelity::NativeStructural
    })
}
fn exact(
    d: &Data,
    q: Id<AssertionQualification>,
    context: Id<attribution::AnalysisContext>,
) -> Result<bool, ModelError> {
    let q = need(&d.entry.qualifications, q)?;
    Ok(q.context == context
        && q.modality == Modality::Definite
        && q.approximation == Approximation::Exact)
}
fn source_covered(
    d: &Data,
    scope: Id<CoverageScope>,
    source: &SourceArtifact,
) -> Result<bool, ModelError> {
    Ok(match need(&d.entry.scopes, scope)? {
        CoverageScope::Input { input } => *input == source.input,
        CoverageScope::Artifact { artifact } => *artifact == source.id(),
        CoverageScope::Module { module } => need(&d.entry.modules, *module)?.source == source.id(),
        _ => false,
    })
}
fn read_region(
    d: &Data,
    observation: &FlowUseObservation,
    support: &FlowUseSupport,
) -> Result<Option<(Id<FlowRegionObservation>, Id<FlowRegionSupport>)>, ModelError> {
    let use_ = need(&d.entry.uses, observation.use_)?;
    let lexical = need(&d.entry.lexical_scopes, observation.scope)?;
    let q = need(&d.entry.qualifications, observation.qualification)?;
    let mut owners = d.entry.owners.iter().filter(|o| {
        o.occurrence == use_.occurrence && o.owner == lexical.owner
            && d.entry.refs.get(o.entity).is_some()
    });
    let Some(owner) = owners.next() else { return Ok(None) };
    if owners.next().is_some() { return Ok(None) }
    let Ok((region, region_support)) = conditions::entry::region_for_access(
        &d.entry, owner.entity, use_.occurrence, q.context, support.run,
    ) else { return Ok(None) };
    let row = need(&d.entry.regions, region)?;
    let rq = need(&d.entry.qualifications, row.qualification)?;
    if rq.scope != q.scope || !native(d, NativeAssertionPremise::Region {
        assertion: region, support: region_support,
    }, rq.id()) {
        return Ok(None);
    }
    Ok(Some((region, region_support)))
}
/// The named origin's execution domain, retaining its exact containing native region when needed.
pub(crate) fn named_condition(
    d: &Data,
    value: &ValueSource,
    budget: &resources::ResourceBudget,
) -> Result<Diagram, ModelError> {
    let ValueSource::Named { observation, support, reaching, region, region_support, .. } = value else {
        return Err(invalid("nested value has no named read condition"));
    };
    let observation = need(&d.entry.use_observations, *observation)?;
    let support = need(&d.entry.use_supports, *support)?;
    let q = need(&d.entry.qualifications, observation.qualification)?;
    let _nodes = budget.reserve("named-read-condition", d.entry.condition_nodes.len().checked_mul(2048)
        .ok_or_else(|| invalid("named read condition overflow"))?)?;
    let nodes = d.entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let condition = Diagram::from_records(need(&d.entry.conditions, q.condition)?, &nodes)?;
    let domain = match (*region, *region_support) {
        (None, None) => condition,
        (Some(region), Some(region_support)) => {
            if read_region(d, observation, support)? != Some((region, region_support)) {
                return Err(invalid("named read region does not replay"));
            }
            let rq = need(&d.entry.qualifications, need(&d.entry.regions, region)?.qualification)?;
            let region_condition = Diagram::from_records(need(&d.entry.conditions, rq.condition)?, &nodes)?;
            let admitted = condition.admitted_binary(&region_condition, BooleanOperation::Conjunction, budget)
                .map_err(|e| invalid(format!("named read region boundary: {e:?}")))?;
            let domain = admitted.into_parts().0;
            if domain.is_false() { return Err(invalid("named read region has no execution domain")); }
            domain
        }
        _ => return Err(invalid("named read region and support must be paired")),
    };
    let rq = need(&d.entry.qualifications, need(&d.entry.reaching, *reaching)?.qualification)?;
    let reaching = Diagram::from_records(need(&d.entry.conditions, rq.condition)?, &nodes)?;
    let covered = domain.admitted_binary(&reaching, BooleanOperation::Conjunction, budget)
        .map_err(|e| invalid(format!("named read implication boundary: {e:?}")))?;
    if covered.into_parts().0.id() != domain.id() { return Err(invalid("named read condition does not imply reaching")); }
    Ok(domain)
}
/// One reaching definition under the read condition, in the same provider invocation and scope.
/// Rebinding selects its actual new definition; unions, loops, foreign supports and captured state refuse.
pub fn named_definition(
    d: &Data,
    value: &Occurrence,
    context: Id<attribution::AnalysisContext>,
    budget: &resources::ResourceBudget,
) -> Result<Option<(Id<Occurrence>, ValueSource)>, ModelError> {
    let mut uses = d.entry.uses.iter().filter(|u| {
        d.entry
            .occurrences
            .get(u.occurrence)
            .is_some_and(|r| same(r, value))
    });
    let Some(use_) = uses.next() else {
        return Ok(None);
    };
    if uses.next().is_some() {
        return Ok(None);
    }
    let mut observations = d.entry.use_observations.iter().filter(|o| {
        o.use_ == use_.id()
            && d.entry
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == context)
    });
    let Some(observation) = observations.next() else {
        return Ok(None);
    };
    if observations.next().is_some()
        || observation.annotation
        || !exact(d, observation.qualification, context)?
    {
        return Ok(None);
    }
    let mut reaches = d.entry.reaching.iter().filter(|r| {
        r.use_ == use_.id()
            && d.entry
                .qualifications
                .get(r.qualification)
                .is_some_and(|q| q.context == context)
    });
    let Some(reaching) = reaches.next() else {
        return Ok(None);
    };
    if reaches.next().is_some()
        || reaching.loop_carried
        || !exact(d, reaching.qualification, context)?
    {
        return Ok(None);
    }
    let ReachingDefinition::Bound { definition } = need(&d.entry.targets, reaching.target)? else {
        return Ok(None);
    };
    if need(&d.entry.definitions, *definition)?.place != use_.place {
        return Ok(None);
    };
    let mut definitions = d.entry.definition_observations.iter().filter(|o| {
        o.definition == *definition
            && d.entry
                .qualifications
                .get(o.qualification)
                .is_some_and(|q| q.context == context)
    });
    let Some(definition) = definitions.next() else {
        return Ok(None);
    };
    if definitions.next().is_some()
        || definition.scope != observation.scope
        || !exact(d, definition.qualification, context)?
    {
        return Ok(None);
    }
    let Some(origin) = definition.value else {
        return Ok(None);
    };
    let origin_row = need(&d.entry.occurrences, origin)?;
    if origin_row.source != value.source {
        return Ok(None);
    }
    let use_q = need(&d.entry.qualifications, observation.qualification)?;
    let reach_q = need(&d.entry.qualifications, reaching.qualification)?;
    if use_q.scope != reach_q.scope
        || use_q.scope != need(&d.entry.qualifications, definition.qualification)?.scope
    {
        return Ok(None);
    }
    let source = need(&d.entry.artifacts, value.source)?;
    if !source_covered(d, use_q.scope, source)? {
        return Ok(None);
    }
    let _nodes = budget.reserve(
        "handoff-condition",
        d.entry
            .condition_nodes
            .len()
            .checked_mul(2048)
            .ok_or_else(|| invalid("handoff condition overflow"))?,
    )?;
    let nodes = d.entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let condition = Diagram::from_records(need(&d.entry.conditions, use_q.condition)?, &nodes)?;
    let reach_condition =
        Diagram::from_records(need(&d.entry.conditions, reach_q.condition)?, &nodes)?;
    let admitted = condition
        .admitted_binary(&reach_condition, BooleanOperation::Conjunction, budget)
        .map_err(|e| invalid(format!("handoff condition boundary: {e:?}")))?;
    let directly_covered = admitted.into_parts().0.id() == condition.id();
    for support in d
        .entry
        .use_supports
        .iter()
        .filter(|s| s.assertion == observation.id())
    {
        let Some(a) = support.attribution() else {
            continue;
        };
        let run = need(&d.entry.runs, a.run)?;
        if (run.context, run.input) != (context, source.input) {
            continue;
        }
        let mut coverage = None;
        for candidate in d.entry.coverage.iter().filter(|c| {
            c.run == Some(a.run)
                && c.provider == Some(run.provider)
                && c.context == context
                && c.family == FactFamily::Flow
                && matches!(
                    c.status,
                    attribution::CoverageStatus::CompleteUnderStatedModel
                        | attribution::CoverageStatus::Partial
                )
        }) {
            if source_covered(d, candidate.scope, source)? {
                coverage = Some(candidate);
                break;
            }
        }
        let Some(coverage) = coverage else {
            continue;
        };
        if !native(
            d,
            NativeAssertionPremise::Use {
                assertion: observation.id(),
                support: support.id(),
            },
            observation.qualification,
        ) {
            continue;
        }
        let region = if directly_covered {
            None
        } else {
            let Some((region, region_support)) = read_region(d, observation, support)? else { continue };
            let rq = need(&d.entry.qualifications, need(&d.entry.regions, region)?.qualification)?;
            let region_condition = Diagram::from_records(need(&d.entry.conditions, rq.condition)?, &nodes)?;
            let domain = condition.admitted_binary(&region_condition, BooleanOperation::Conjunction, budget)
                .map_err(|e| invalid(format!("handoff read region boundary: {e:?}")))?.into_parts().0;
            if domain.is_false() { continue }
            let covered = domain.admitted_binary(&reach_condition, BooleanOperation::Conjunction, budget)
                .map_err(|e| invalid(format!("handoff region implication boundary: {e:?}")))?;
            if covered.into_parts().0.id() != domain.id() { continue }
            Some((region, region_support))
        };
        for rs in d.entry.reaching_supports.iter().filter(|s| {
            s.assertion == reaching.id() && s.attribution().is_some_and(|r| r.run == a.run)
        }) {
            let Some((inventory, inventory_support)) = flow_inventory::complete_native_singleton(
                &d.entry,
                observation,
                support,
                reaching,
                rs,
                budget,
            )?
            else {
                continue;
            };
            if !native(
                d,
                NativeAssertionPremise::FlowUseInventory {
                    assertion: inventory.id(),
                    support: inventory_support.id(),
                },
                inventory.qualification,
            ) {
                continue;
            }
            if !native(
                d,
                NativeAssertionPremise::Reaching {
                    assertion: reaching.id(),
                    support: rs.id(),
                },
                reaching.qualification,
            ) {
                continue;
            }
            for ds in d.entry.definition_supports.iter().filter(|s| {
                s.assertion == definition.id() && s.attribution().is_some_and(|r| r.run == a.run)
            }) {
                if native(
                    d,
                    NativeAssertionPremise::Definition {
                        assertion: definition.id(),
                        support: ds.id(),
                    },
                    definition.qualification,
                ) {
                    return Ok(Some((
                        origin,
                        ValueSource::Named {
                            observation: observation.id(),
                            support: support.id(),
                            inventory: inventory.id(),
                            inventory_support: inventory_support.id(),
                            reaching: reaching.id(),
                            reaching_support: rs.id(),
                            definition: definition.id(),
                            definition_support: ds.id(),
                            region: region.map(|r| r.0),
                            region_support: region.map(|r| r.1),
                            coverage: coverage.id(),
                        },
                    )));
                }
            }
        }
    }
    Ok(None)
}
fn rank(role: SourceRole) -> u8 {
    match role {
        SourceRole::Example => 0,
        SourceRole::DocBlock => 1,
        SourceRole::Test => 2,
        _ => 3,
    }
}
pub fn produce(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    invocation: &analysis::structural::Invocation,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    for binding in d.bindings.iter() {
        let attempt = need(&d.attempts, binding.attempt)?;
        if attempt.outcome != BindingOutcome::Bound {
            continue;
        }
        let consumer = need(&base.events.alternatives, attempt.alternative)?;
        let ce = need(&base.events.events, consumer.event)?;
        if ce.context != invocation.context {
            continue;
        }
        let Some(consumer_entity) = consumer.entity else {
            continue;
        };
        if !out.scope.iter().any(|s| s.entity == consumer_entity) {
            continue;
        }
        let BindingSource::Actual { occurrence } = need(&d.sources, binding.source)? else {
            continue;
        };
        let value = need(&d.entry.occurrences, *occurrence)?;
        let source = need(&base.projection.artifacts, value.source)?;
        if source.input != invocation.input {
            continue;
        }
        let mut roles = base.uses.iter().filter(|r| {
            r.input == invocation.input
                && r.artifact == source.id()
                && matches!(
                    r.role,
                    SourceRole::Example | SourceRole::DocBlock | SourceRole::Test
                )
        });
        let Some(role) = roles.next() else { continue };
        let mut role = role;
        for next in roles {
            if rank(next.role) < rank(role.role) {
                role = next;
            }
        }
        // An argument is selected by the canonical binding, not by text or parameter name.
        let Some(syntax) = attempt.syntax else {
            continue;
        };
        if !d.arguments.iter().any(|a| {
            a.call == syntax
                && d.entry
                    .occurrences
                    .get(a.value)
                    .is_some_and(|v| same(v, value))
        }) {
            continue;
        }
        if attempt.authority
            != normalized::signature_applicability::BindingAuthority::EffectiveInvocation
            || binding.projection != BindingProjection::Whole.id()
        {
            out.handoff_assessments.insert(Assessment {
                frame: frame.id(),
                binding: binding.id(),
                argument: value.id(),
                value: None,
                reason: Some(obligation::ObligationKind::AmbiguousBinding),
            })?;
            continue;
        }
        let origin = if value.syntax_kind == SyntaxKind::ExprCall {
            Some((
                value.id(),
                ValueSource::Nested {
                    expression: value.id(),
                },
            ))
        } else if value.syntax_kind == SyntaxKind::ExprName {
            named_definition(d, value, invocation.context, budget)?.filter(|(origin, _)| {
                d.entry
                    .occurrences
                    .get(*origin)
                    .is_some_and(|o| o.syntax_kind == SyntaxKind::ExprCall)
            })
        } else {
            None
        };
        let Some((origin, proof)) = origin else {
            out.handoff_assessments.insert(Assessment {
                frame: frame.id(),
                binding: binding.id(),
                argument: value.id(),
                value: None,
                reason: Some(obligation::ObligationKind::MissingEvidence),
            })?;
            continue;
        };
        out.handoff_values.insert(proof.clone())?;
        out.handoff_assessments.insert(Assessment {
            frame: frame.id(),
            binding: binding.id(),
            argument: value.id(),
            value: Some(proof.id()),
            reason: None,
        })?;
        let origin = need(&d.entry.occurrences, origin)?;
        for producer in base.events.alternatives.iter() {
            let pe = need(&base.events.events, producer.event)?;
            if pe.context != invocation.context
                || !same(need(&base.projection.occurrences, pe.site)?, origin)
                || !matches!(
                    target(base, producer)?.phase,
                    CallPhase::Call | CallPhase::New
                )
            {
                continue;
            }
            let Some(entity) = producer.entity else {
                continue;
            };
            if entity == consumer_entity || !out.scope.iter().any(|s| s.entity == entity) {
                continue;
            }
            let admitted = |alternative| {
                base.events.admissions.iter().any(|a| {
                    a.alternative == alternative
                        && base
                            .events
                            .policy_assessments
                            .get(a.assessment)
                            .is_some_and(|p| p.policy == CallPolicy::Usage)
                })
            };
            if !admitted(consumer.id()) || !admitted(producer.id()) {
                continue;
            }
            out.handoff_values.insert(proof.clone())?;
            out.handoffs.insert(Handoff {
                frame: frame.id(),
                consumer: consumer.id(),
                producer: producer.id(),
                binding: binding.id(),
                value: proof.id(),
                role: role.id(),
            })?;
        }
    }
    let mut keys = charged::ChargedMap::<
        (
            Id<normalized::entities::EntityRef>,
            Id<normalized::entities::EntityRef>,
            Id<SignatureSlot>,
            (i16, i16),
            (i16, i16),
        ),
        Vec<Id<Handoff>>,
    >::default();
    let mut charge = charged::StateCharge::new(budget, "handoff-groups");
    for row in out.handoffs.iter() {
        let c = need(&base.events.alternatives, row.consumer)?;
        let p = need(&base.events.alternatives, row.producer)?;
        let formal = need(&d.bindings, row.binding)?.slot;
        let _cq = need(
            &base.projection.qualifications,
            target(base, c)?.qualification,
        )?;
        let _pq = need(
            &base.projection.qualifications,
            target(base, p)?.qualification,
        )?;
        for (seed, other) in [
            (c.entity.unwrap(), p.entity.unwrap()),
            (p.entity.unwrap(), c.entity.unwrap()),
        ] {
            if out
                .public
                .iter()
                .any(|r| r.in_subsystem && r.entity == seed)
            {
                keys.update(
                    &mut charge,
                    (
                        seed,
                        other,
                        formal,
                        (target(base, p)?.phase.code(), target(base, c)?.phase.code()),
                        (modality(base, p)?.code(), modality(base, c)?.code()),
                    ),
                    |v| v.push(row.id()),
                )?;
            }
        }
    }
    for ((seed, other, formal, (pp, cp), (pm, cm)), members) in keys.iter() {
        let order = |id: &Id<Handoff>| {
            let h = out.handoffs.get(*id).unwrap();
            let c = base.events.alternatives.get(h.consumer).unwrap();
            let e = base.events.events.get(c.event).unwrap();
            let o = base.projection.occurrences.get(e.site).unwrap();
            let a = base.projection.artifacts.get(o.source).unwrap();
            (
                rank(base.uses.get(h.role).unwrap().role),
                &a.path,
                o.start,
                e.site,
                h.producer,
                *id,
            )
        };
        let _sort = budget.reserve(
            "handoff-sort",
            members
                .len()
                .checked_mul(size_of::<Id<Handoff>>())
                .ok_or_else(|| invalid("handoff sort overflow"))?,
        )?;
        let mut members = members.clone();
        members.sort_by_key(order);
        members.dedup();
        let total = members.len();
        members.truncate(settings.witnesses as usize);
        let group = Group {
            frame: frame.id(),
            seed: *seed,
            other: *other,
            formal: *formal,
            producer_phase: CallPhase::from_code(*pp).unwrap(),
            consumer_phase: CallPhase::from_code(*cp).unwrap(),
            producer_modality: Modality::from_code(*pm).unwrap(),
            consumer_modality: Modality::from_code(*cm).unwrap(),
            occurrences: total as i64,
            retained: members.len() as i64,
        };
        for (ordinal, id) in members.into_iter().enumerate() {
            out.handoff_members.insert(Member {
                group: group.id(),
                ordinal: ordinal as i64,
                occurrence: id,
            })?;
        }
        out.handoff_groups.insert(group)?;
    }
    Ok(())
}

fn target<'a>(
    base: &'a build::Data,
    row: &NormalizedCallAlternative,
) -> Result<&'a CallTarget, ModelError> {
    need(
        &base.projection.targets,
        need(&base.events.alternative_sources, row.source)?.target(),
    )
}

fn modality(base: &build::Data, row: &NormalizedCallAlternative) -> Result<Modality, ModelError> {
    let source = need(&base.events.alternative_sources, row.source)?;
    let q = need(
        &base.projection.qualifications,
        target(base, row)?.qualification,
    )?;
    Ok(
        if matches!(source, CallAlternativeSource::DerivedDispatch { .. }) {
            q.modality.weakest(Modality::Candidate)
        } else {
            q.modality
        },
    )
}
