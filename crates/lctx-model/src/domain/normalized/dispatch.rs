//! Conservative captured override members under the original native dispatch premise.
//! Complete captured ancestry is never a closed runtime subclass universe.
use super::{
    entities::*,
    event_normalization::{EventData, EventOutput},
    events::*,
    policy_revision,
};
use crate::domain::{assertion::*, attribution::*, calls::*, charged::StateCharge, symbols::*, *};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DispatchReason {
    OpenRuntimeSubclasses = 0,
    MissingReceiverClass = 1,
    MissingCorrespondence = 2,
    MissingTraits = 3,
    ConflictingTraits = 4,
    IncompleteAncestry = 5,
    ConflictingAncestry = 6,
    MissingSupport = 7,
    SupportDisagreement = 8,
    IncompleteCoverage = 9,
    QualificationDisagreement = 10,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "normalized_dispatch_assessments")]
pub struct DispatchAssessment {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub target: Id<CallTarget>,
    #[model(key)]
    pub policy: ContentHash,
    pub named: Id<ProviderSymbol>,
    pub receiver: Option<Id<ClassEntity>>,
    pub receiver_correspondence: Option<Id<SymbolEntityResolution>>,
    pub members: ContentHash,
    pub open: bool,
    pub reason: DispatchReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "normalized_dispatch_members")]
pub struct DispatchMember {
    #[model(key)]
    pub assessment: Id<DispatchAssessment>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    pub correspondence: Id<SymbolEntityResolution>,
    pub entity: Id<EntityRef>,
    pub defining_class: Id<ProviderSymbol>,
    pub named: bool,
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "normalized_dispatch_premises")]
pub enum DispatchPremise {
    #[model(code = 0)]
    TargetSupport { support: Id<CallTargetSupport> },
    #[model(code = 1)]
    Trait {
        observation: Id<FunctionTraitObservation>,
    },
    #[model(code = 2)]
    TraitSupport { support: Id<FunctionTraitSupport> },
    #[model(code = 3)]
    Ancestry {
        observation: Id<ClassAncestryObservation>,
    },
    #[model(code = 4)]
    AncestrySupport { support: Id<ClassAncestrySupport> },
    #[model(code = 5)]
    SequenceMember { member: Id<SymbolSequenceMember> },
    #[model(code = 6)]
    Coverage { coverage: Id<ProviderCoverage> },
    #[model(code = 7)]
    Correspondence {
        resolution: Id<SymbolEntityResolution>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_dispatch_evidence")]
pub struct DispatchEvidence {
    #[model(key)]
    pub assessment: Id<DispatchAssessment>,
    #[model(key)]
    pub member: Option<Id<DispatchMember>>,
    #[model(key)]
    pub premise: Id<DispatchPremise>,
}
/// A member-shape proof created only inside replay of the shared event closure.
/// ```compile_fail
/// use lctx_model::domain::normalized::dispatch::ApplicableDispatch;
/// fn forge(mut proof: ApplicableDispatch) { proof.symbol = todo!(); }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ApplicableDispatch {
    member: Id<DispatchMember>,
    target: Id<CallTarget>,
    symbol: Id<ProviderSymbol>,
    entity: Id<EntityRef>,
    context: Id<AnalysisContext>,
}
impl HeapSize for ApplicableDispatch {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl ApplicableDispatch {
    pub fn member(&self) -> Id<DispatchMember> {
        self.member
    }
    pub(crate) fn symbol(&self) -> Id<ProviderSymbol> {
        self.symbol
    }
    pub(crate) fn matches(
        &self,
        target: &CallTarget,
        resolution: &SymbolEntityResolution,
        entity: Id<EntityRef>,
        context: Id<AnalysisContext>,
    ) -> bool {
        self.target == target.id()
            && self.symbol == resolution.symbol
            && self.entity == entity
            && self.context == context
    }
    pub(crate) fn from_member(
        member: &DispatchMember,
        assessment: &DispatchAssessment,
        context: Id<AnalysisContext>,
    ) -> Self {
        Self {
            member: member.id(),
            target: assessment.target,
            symbol: member.symbol,
            entity: member.entity,
            context,
        }
    }
}
fn scope(data: &EventData, q: &AssertionQualification) -> Option<Id<input::InputRevision>> {
    super::signature_applicability::ScopeCatalog {
        scopes: &data.scopes,
        artifacts: &data.artifacts,
        modules: &data.modules,
    }
    .input(q.scope)
}
fn frame(data: &EventData, base: &AssertionQualification, q: &AssertionQualification) -> bool {
    q.context == base.context
        && q.condition == base.condition
        && q.modality == Modality::Definite
        && q.approximation == Approximation::Exact
        && scope(data, q).is_some()
        && scope(data, q) == scope(data, base)
}
fn resolution<'a>(
    data: &'a EventData,
    symbol: Id<ProviderSymbol>,
    q: &AssertionQualification,
    premises: &mut Vec<DispatchPremise>,
) -> Result<&'a SymbolEntityResolution, DispatchReason> {
    let mut rows = data
        .symbol_resolutions
        .iter()
        .filter(|r| r.symbol == symbol && r.context == q.context && r.policy == policy_revision());
    let row = rows.next().ok_or(DispatchReason::MissingCorrespondence)?;
    if rows.next().is_some() || row.status != ResolutionStatus::Resolved || row.entity.is_none() {
        return Err(DispatchReason::MissingCorrespondence);
    }
    premises.push(DispatchPremise::Correspondence {
        resolution: row.id(),
    });
    Ok(row)
}
fn support(
    data: &EventData,
    s: Option<SupportAttribution>,
    q: &AssertionQualification,
    family: FactFamily,
    provider: Id<Provider>,
    premises: &mut Vec<DispatchPremise>,
) -> Result<Id<ProviderRun>, DispatchReason> {
    let s = s.ok_or(DispatchReason::SupportDisagreement)?;
    let run = data.runs.get(s.run).ok_or(DispatchReason::MissingSupport)?;
    let surface = data
        .surfaces
        .get(s.surface)
        .ok_or(DispatchReason::MissingSupport)?;
    if run.context != q.context
        || Some(run.input) != scope(data, q)
        || run.provider != provider
        || surface.provider != provider
        || surface.family != family
        || !(s.fidelity == Fidelity::NativeStructural
            || family == FactFamily::Signatures && s.fidelity == Fidelity::ReportProjection)
    {
        return Err(DispatchReason::SupportDisagreement);
    }
    match data.native_evidence.get(s.evidence) {
        Some(Evidence::Invocation { run }) if *run == s.run => {}
        Some(Evidence::SourceSpan { source, .. })
            if data
                .artifacts
                .get(*source)
                .is_some_and(|a| a.input == run.input) => {}
        Some(Evidence::Occurrence { occurrence })
            if data
                .occurrences
                .get(*occurrence)
                .and_then(|o| data.artifacts.get(o.source))
                .is_some_and(|a| a.input == run.input) => {}
        _ => return Err(DispatchReason::SupportDisagreement),
    }
    let covered: Vec<_> = data
        .coverage
        .iter()
        .filter(|c| {
            c.context == q.context
                && c.scope == q.scope
                && c.run == Some(s.run)
                && c.provider == Some(provider)
                && c.family == family
        })
        .collect();
    if covered.is_empty()
        || covered
            .iter()
            .any(|c| c.status != CoverageStatus::CompleteUnderStatedModel)
    {
        return Err(DispatchReason::IncompleteCoverage);
    }
    for c in covered {
        premises.push(DispatchPremise::Coverage { coverage: c.id() });
    }
    Ok(s.run)
}
fn trait_row<'a>(
    data: &'a EventData,
    symbol: Id<ProviderSymbol>,
    q: &AssertionQualification,
    provider: Id<Provider>,
    run: Id<ProviderRun>,
    premises: &mut Vec<DispatchPremise>,
) -> Result<&'a FunctionTraitObservation, DispatchReason> {
    let mut rows = data.traits.iter().filter(|t| {
        t.symbol == symbol
            && data
                .qualifications
                .get(t.qualification)
                .is_some_and(|tq| tq.context == q.context)
    });
    let row = rows.next().ok_or(DispatchReason::MissingTraits)?;
    if rows.next().is_some() {
        return Err(DispatchReason::ConflictingTraits);
    }
    let tq = data
        .qualifications
        .get(row.qualification)
        .ok_or(DispatchReason::QualificationDisagreement)?;
    if !frame(data, q, tq) {
        return Err(DispatchReason::QualificationDisagreement);
    }
    let s = data
        .symbols
        .get(symbol)
        .ok_or(DispatchReason::MissingTraits)?;
    if s.provider != provider
        || s.context != q.context
        || !matches!(s.kind, SymbolKind::Method | SymbolKind::Function)
        || row.defining_class.is_none()
    {
        return Err(DispatchReason::MissingTraits);
    }
    premises.push(DispatchPremise::Trait {
        observation: row.id(),
    });
    let mut count = 0;
    for s in data
        .trait_supports
        .iter()
        .filter(|s| s.assertion == row.id())
    {
        count += 1;
        if s.origin != Origin::AnalyzerAssertion || s.mode != ExtractionMode::NativeTraversal {
            return Err(DispatchReason::SupportDisagreement);
        }
        premises.push(DispatchPremise::TraitSupport { support: s.id() });
        if support(
            data,
            s.attribution(),
            tq,
            FactFamily::Signatures,
            provider,
            premises,
        )? != run
        {
            return Err(DispatchReason::SupportDisagreement);
        }
    }
    if count == 0 {
        return Err(DispatchReason::MissingSupport);
    }
    Ok(row)
}
fn mro(
    data: &EventData,
    class: Id<ProviderSymbol>,
    q: &AssertionQualification,
    provider: Id<Provider>,
    run: Id<ProviderRun>,
    premises: &mut Vec<DispatchPremise>,
) -> Result<Vec<Id<ProviderSymbol>>, DispatchReason> {
    let s = data
        .symbols
        .get(class)
        .ok_or(DispatchReason::MissingReceiverClass)?;
    if s.provider != provider || s.context != q.context || s.kind != SymbolKind::Class {
        return Err(DispatchReason::QualificationDisagreement);
    }
    let mut rows = data.ancestry.iter().filter(|a| {
        a.class == class
            && a.relation == AncestryRelation::Mro
            && data
                .qualifications
                .get(a.qualification)
                .is_some_and(|aq| aq.context == q.context)
    });
    let row = rows.next().ok_or(DispatchReason::IncompleteAncestry)?;
    if rows.next().is_some() {
        return Err(DispatchReason::ConflictingAncestry);
    }
    let aq = data
        .qualifications
        .get(row.qualification)
        .ok_or(DispatchReason::QualificationDisagreement)?;
    if !frame(data, q, aq) {
        return Err(DispatchReason::QualificationDisagreement);
    }
    if row.linearization != Some(Linearization::Complete) {
        return Err(DispatchReason::IncompleteAncestry);
    }
    let sequence = data
        .sequences
        .get(row.ancestors)
        .ok_or(DispatchReason::IncompleteAncestry)?;
    let mut members: Vec<_> = data
        .sequence_members
        .iter()
        .filter(|m| m.sequence == row.ancestors)
        .collect();
    members.sort_by_key(|m| m.ordinal);
    let values: Vec<_> = members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if m.ordinal == i as i64 {
                Ok(m.symbol)
            } else {
                Err(DispatchReason::IncompleteAncestry)
            }
        })
        .collect::<Result<_, _>>()?;
    if SymbolSequence::new(&values)
        .map_err(|_| DispatchReason::IncompleteAncestry)?
        .0
        != *sequence
        || values.contains(&class)
        || values
            .iter()
            .enumerate()
            .any(|(i, s)| values[..i].contains(s))
        || values.iter().any(|s| {
            data.symbols.get(*s).is_none_or(|s| {
                s.provider != provider || s.context != q.context || s.kind != SymbolKind::Class
            })
        })
    {
        return Err(DispatchReason::ConflictingAncestry);
    }
    premises.push(DispatchPremise::Ancestry {
        observation: row.id(),
    });
    for m in members {
        premises.push(DispatchPremise::SequenceMember { member: m.id() });
    }
    let mut count = 0;
    for s in data
        .ancestry_supports
        .iter()
        .filter(|s| s.assertion == row.id())
    {
        count += 1;
        if s.origin != Origin::AnalyzerAssertion || s.mode != ExtractionMode::NativeTraversal {
            return Err(DispatchReason::SupportDisagreement);
        }
        premises.push(DispatchPremise::AncestrySupport { support: s.id() });
        if support(
            data,
            s.attribution(),
            aq,
            FactFamily::Signatures,
            provider,
            premises,
        )? != run
        {
            return Err(DispatchReason::SupportDisagreement);
        }
    }
    if count == 0 {
        return Err(DispatchReason::MissingSupport);
    }
    Ok(values)
}
fn digest(premises: &mut Vec<DispatchPremise>) -> ContentHash {
    premises.sort_by_key(Record::id);
    premises.dedup();
    let mut key = KeySink::new("normalized-dispatch-premises");
    for p in premises {
        p.id().encode(&mut key);
    }
    key.finish()
}
struct Candidate {
    symbol: Id<ProviderSymbol>,
    correspondence: Id<SymbolEntityResolution>,
    entity: Id<EntityRef>,
    defining_class: Id<ProviderSymbol>,
    named: bool,
    premises: Vec<DispatchPremise>,
}
/// Single event writer calls this before policy selection. An unsuccessful derivation retains
/// its original native alternative and an explicit open assessment, rather than a guessed edge.
pub(crate) struct AssessedDispatch {
    pub members: Vec<DispatchMember>,
    _charge: StateCharge,
}
pub(crate) fn assess(
    data: &EventData,
    event: Id<NormalizedCallEvent>,
    target: &CallTarget,
    output: &mut EventOutput,
    budget: &resources::ResourceBudget,
) -> Result<AssessedDispatch, ModelError> {
    let Some(CallDestination::Overrides { symbol: named }) =
        data.destinations.get(target.destination)
    else {
        return Ok(AssessedDispatch {
            members: vec![],
            _charge: StateCharge::new(budget, "empty-dispatch"),
        });
    };
    let mut charge = StateCharge::new(budget, "dispatch-native-premises");
    let native_count = data.traits.len()
        + data.trait_supports.len()
        + data.ancestry.len()
        + data.ancestry_supports.len()
        + data.sequence_members.len()
        + data.coverage.len()
        + data.target_supports.len();
    charge.grow(native_count.saturating_mul(256))?;
    let mut premises = Vec::new();
    let mut candidates = Vec::new();
    let mut receiver = None;
    let mut receiver_correspondence = None;
    let mut resource_error = None;
    let mut member_error = None;
    let result = (|| -> Result<(), DispatchReason> {
        let q = data
            .qualifications
            .get(target.qualification)
            .ok_or(DispatchReason::QualificationDisagreement)?;
        if !matches!(q.modality, Modality::Definite | Modality::Candidate)
            || q.approximation != Approximation::Exact
        {
            return Err(DispatchReason::QualificationDisagreement);
        }
        let provider = data
            .symbols
            .get(*named)
            .ok_or(DispatchReason::MissingTraits)?
            .provider;
        let mut run = None;
        let mut count = 0;
        for s in data
            .target_supports
            .iter()
            .filter(|s| s.assertion == target.id())
        {
            count += 1;
            if s.origin != Origin::AnalyzerAssertion || s.mode != ExtractionMode::NativeTraversal {
                return Err(DispatchReason::SupportDisagreement);
            }
            premises.push(DispatchPremise::TargetSupport { support: s.id() });
            let observed = support(
                data,
                s.attribution(),
                q,
                FactFamily::Calls,
                provider,
                &mut premises,
            )?;
            if run.is_some_and(|r| r != observed) {
                return Err(DispatchReason::SupportDisagreement);
            }
            run = Some(observed);
        }
        if count == 0 {
            return Err(DispatchReason::MissingSupport);
        }
        let run = run.unwrap();
        let class = target
            .receiver_class
            .ok_or(DispatchReason::MissingReceiverClass)?;
        let receiver_resolution = resolution(data, class, q, &mut premises)?;
        receiver_correspondence = Some(receiver_resolution.id());
        let Some(EntityRef::Class { class: resolved }) =
            receiver_resolution.entity.and_then(|r| data.refs.get(r))
        else {
            return Err(DispatchReason::MissingReceiverClass);
        };
        if data.classes.get(*resolved).is_none() {
            return Err(DispatchReason::MissingReceiverClass);
        }
        receiver = Some(*resolved);
        let ancestry = mro(data, class, q, provider, run, &mut premises)?;
        let named_trait = trait_row(data, *named, q, provider, run, &mut premises)?;
        let defining = named_trait.defining_class.unwrap();
        if class != defining && !ancestry.contains(&defining) {
            return Err(DispatchReason::IncompleteAncestry);
        }
        let mut frontier = vec![*named];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(symbol) = frontier.pop() {
            if !visited.insert(symbol) {
                continue;
            }
            if visited.len() > MAX_SYMBOL_NESTING {
                return Err(DispatchReason::ConflictingTraits);
            }
            // Reserve each retained candidate before cloning its premise set.
            if let Err(error) = charge.grow(native_count.saturating_mul(256)) {
                resource_error = Some(error);
                return Err(DispatchReason::IncompleteCoverage);
            }
            let candidate = (|| -> Result<Option<Candidate>, DispatchReason> {
                let mut member_premises = premises.clone();
                let trait_ = trait_row(data, symbol, q, provider, run, &mut member_premises)?;
                let defining = trait_.defining_class.unwrap();
                let members = mro(data, defining, q, provider, run, &mut member_premises)?;
                if defining != class && !members.contains(&class) && !ancestry.contains(&defining) {
                    return Ok(None);
                }
                let r = resolution(data, symbol, q, &mut member_premises)?;
                let Some(EntityRef::Callable { callable }) =
                    r.entity.and_then(|r| data.refs.get(r))
                else {
                    return Ok(None);
                };
                if data.callables.get(*callable).is_none() {
                    return Ok(None);
                }
                Ok(Some(Candidate {
                    symbol,
                    correspondence: r.id(),
                    entity: r.entity.unwrap(),
                    defining_class: defining,
                    named: symbol == *named,
                    premises: member_premises,
                }))
            })();
            match candidate {
                Ok(Some(c)) => {
                    premises.extend(c.premises.iter().cloned());
                    digest(&mut premises);
                    candidates.push(c);
                    frontier.extend(
                        data.traits
                            .iter()
                            .filter(|t| {
                                t.overrides == Some(symbol)
                                    && data.symbols.get(t.symbol).is_some_and(|s| {
                                        s.provider == provider && s.context == q.context
                                    })
                            })
                            .map(|t| t.symbol),
                    );
                }
                Ok(None) => {}
                Err(reason) => {
                    member_error = Some(reason);
                }
            }
        }
        Ok(())
    })();
    if let Some(error) = resource_error {
        return Err(error);
    }
    let reason = result
        .err()
        .or(member_error)
        .unwrap_or(DispatchReason::OpenRuntimeSubclasses);
    for c in &candidates {
        premises.extend(c.premises.iter().cloned());
    }
    let members = digest(&mut premises);
    let assessment = output.dispatch_assessments.insert(DispatchAssessment {
        event,
        target: target.id(),
        policy: policy_revision(),
        named: *named,
        receiver,
        receiver_correspondence,
        members,
        open: true,
        reason,
    })?;
    for p in &premises {
        let p = output.dispatch_premises.insert(p.clone())?;
        output.dispatch_evidence.insert(DispatchEvidence {
            assessment,
            member: None,
            premise: p,
        })?;
    }
    let mut result = Vec::new();
    for mut c in candidates {
        let members = digest(&mut c.premises);
        let row = DispatchMember {
            assessment,
            symbol: c.symbol,
            correspondence: c.correspondence,
            entity: c.entity,
            defining_class: c.defining_class,
            named: c.named,
            members,
        };
        let member = output.dispatch_members.insert(row.clone())?;
        for p in c.premises {
            let p = output.dispatch_premises.insert(p)?;
            output.dispatch_evidence.insert(DispatchEvidence {
                assessment,
                member: Some(member),
                premise: p,
            })?;
        }
        result.push(row);
    }
    result.sort_by_key(Record::id);
    Ok(AssessedDispatch {
        members: result,
        _charge: charge,
    })
}
