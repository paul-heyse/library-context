//! ClassOf is an expression-relative receiver operation, never an instance identity or runtime class.
use super::{
    Rows, callables::*, entities::*, policy_revision, signature_applicability::ScopeCatalog,
};
use crate::domain::charged::{ChargedMap, StateCharge};
use crate::domain::{assertion::*, attribution::*, calls::*, source::*, syntax::*, *};
use crate::{Domain, DomainCode, DomainSum};
trait Source<R: Record> {
    fn rows(&self) -> &Rows<R>;
}
macro_rules! input { ($($field:ident: $ty:ty,)*) => {
    pub struct ReceiverData { $(pub $field: Rows<$ty>,)* }
    $(impl Source<$ty> for ReceiverData { fn rows(&self)->&Rows<$ty> { &self.$field } })*
    impl ReceiverData {
        pub fn new(budget:&resources::ResourceBudget)->Self {Self { $($field: Rows::new(budget),)* }}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})* Ok(false)}
        pub fn validation_inputs()->Vec<ValidationInput> {super::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*])}
        pub fn stage_inputs()->Vec<stages::RelationUse> {vec![$(stages::RelationUse::completed::<$ty>()),*]}
    }
}; }
crate::normalized_receiver_inputs!(input);
macro_rules! output { ($($field:ident: $ty:ty,)*) => {
    pub struct ReceiverOutput { $(pub $field: Rows<$ty>,)* }
    impl ReceiverOutput {
        pub fn new(budget:&resources::ResourceBudget)->Self {Self { $($field: Rows::new(budget),)* }}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})* Ok(false)}
        pub fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$field.same(&other.$field) {return Err(ModelError::Invalid(format!("receiver closure differs: {}",<$ty>::NAME)));})* Ok(())}
        pub fn validation_inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
    }
}; }
crate::normalized_receiver_outputs!(output);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ReceiverReason {
    MissingCallable = 0,
    DescriptorUnknown = 1,
    MissingSyntax = 2,
    AmbiguousSyntax = 3,
    MissingPlacement = 4,
    AmbiguousPlacement = 5,
    QualificationDisagreement = 6,
    MissingSupport = 7,
    SupportDisagreement = 8,
    IncompleteCoverage = 9,
    SyntaxDisagreement = 10,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name="receiver_assessments", invariant_refs=invariants_refs)]
pub enum ReceiverAssessment {
    #[model(code = 0)]
    ClassOf {
        target: Id<CallTarget>,
        policy: ContentHash,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
        syntax: Id<CallSyntax>,
        placement: Id<SyntaxPlacement>,
        effective: Id<EffectiveCallableAssessment>,
        actual: Id<Occurrence>,
        members: ContentHash,
    },
    #[model(code = 1)]
    Unknown {
        target: Id<CallTarget>,
        policy: ContentHash,
        reason: ReceiverReason,
        members: ContentHash,
    },
}
impl ReceiverAssessment {
    pub fn target(&self) -> Id<CallTarget> {
        match self {
            Self::ClassOf { target, .. } | Self::Unknown { target, .. } => *target,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "receiver_premises")]
pub enum ReceiverPremise {
    #[model(code = 0)]
    Target { support: Id<CallTargetSupport> },
    #[model(code = 1)]
    Syntax { support: Id<CallSyntaxSupport> },
    #[model(code = 2)]
    Placement { support: Id<SyntaxPlacementSupport> },
    #[model(code = 3)]
    Coverage { coverage: Id<ProviderCoverage> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "receiver_evidence")]
pub struct ReceiverEvidence {
    #[model(key)]
    pub assessment: Id<ReceiverAssessment>,
    #[model(key)]
    pub premise: Id<ReceiverPremise>,
}
/// Replay of receiver and callable premises creates this internal operation capability.
/// It assumes checked predecessor correspondence; final binding admission independently replays
/// N1/N2, and store read eligibility validates those predecessor invariants.
/// ```
/// use lctx_model::domain::normalized::receiver::ApplicableReceiver;
/// fn inspect(proof: &ApplicableReceiver) { let _ = proof.assessment(); }
/// ```
/// ```compile_fail
/// use lctx_model::domain::normalized::receiver::ApplicableReceiver;
/// fn forge(mut proof: ApplicableReceiver) { proof.actual = todo!(); }
/// ```
/// ```compile_fail
/// use lctx_model::domain::normalized::receiver::VerifiedReceivers;
/// fn extract_proof(checked: &VerifiedReceivers) { let _ = checked.get(todo!()); }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ApplicableReceiver {
    assessment: Id<ReceiverAssessment>,
    target: Id<CallTarget>,
    syntax: Id<CallSyntax>,
    effective: Id<EffectiveCallableAssessment>,
    actual: Id<Occurrence>,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
}
impl HeapSize for ApplicableReceiver {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl ApplicableReceiver {
    pub fn assessment(&self) -> Id<ReceiverAssessment> {
        self.assessment
    }
    pub(crate) fn actual(&self) -> Id<Occurrence> {
        self.actual
    }
    pub(crate) fn matches_operation(
        &self,
        target: &CallTarget,
        call: &CallSyntax,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
    ) -> bool {
        self.target == target.id()
            && self.syntax == call.id()
            && self.input == input
            && self.context == context
    }
    pub(crate) fn matches(
        &self,
        target: &CallTarget,
        call: &CallSyntax,
        variant: &SignatureVariant,
        input: Id<input::InputRevision>,
        context: Id<AnalysisContext>,
    ) -> bool {
        self.target == target.id()
            && self.syntax == call.id()
            && Some(self.effective) == variant.assessment
            && self.input == input
            && self.context == context
    }
}
pub struct VerifiedReceivers {
    proofs: ChargedMap<Id<CallTarget>, ApplicableReceiver>,
    _charge: StateCharge,
}
impl VerifiedReceivers {
    pub fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if !self
            ._charge
            .budget()
            .expect("owner budget")
            .shares_pool(other._charge.budget().expect("owner budget"))
        {
            return Err(ModelError::Conflict("receiver authority budget"));
        }
        for (target, proof) in other.proofs.iter() {
            if self
                .proofs
                .get(target)
                .is_some_and(|existing| existing.assessment != proof.assessment)
            {
                return Err(ModelError::Conflict("receiver authority domain"));
            }
            self.proofs.insert(&mut self._charge, *target, *proof)?;
        }
        Ok(())
    }

    pub fn has_class_of(&self, target: Id<CallTarget>) -> bool {
        self.proofs.contains_key(&target)
    }
    pub(crate) fn get(&self, target: Id<CallTarget>) -> Option<&ApplicableReceiver> {
        self.proofs.get(&target)
    }
}
fn scopes(data: &ReceiverData) -> ScopeCatalog<'_> {
    ScopeCatalog {
        scopes: &data.scopes,
        artifacts: &data.artifacts,
        modules: &data.modules,
    }
}
fn exact(q: &AssertionQualification) -> bool {
    q.modality == Modality::Definite && q.approximation == Approximation::Exact
}
fn frame(data: &ReceiverData, q: &AssertionQualification, other: &AssertionQualification) -> bool {
    q.context == other.context
        && q.condition == other.condition
        && scopes(data).input(q.scope).is_some()
        && scopes(data).input(q.scope) == scopes(data).input(other.scope)
        && matches!(q.modality, Modality::Definite | Modality::Candidate)
        && q.approximation == Approximation::Exact
        && exact(other)
}
fn candidates(data: &ReceiverData, target: &CallTarget) -> bool {
    target.passing == Some(ReceiverPassing::Object)
        && target.class_method == Some(true)
        && target.static_method != Some(true)
        && matches!(
            data.receivers.get(target.receiver),
            Some(Receiver::Unknown { .. })
        )
}
fn known_descriptor<'a>(
    data: &'a ReceiverData,
    target: &CallTarget,
    q: &AssertionQualification,
) -> Option<&'a EffectiveCallableAssessment> {
    let symbol = data.destinations.get(target.destination)?.symbol()?;
    let mut resolved = data.symbol_resolutions.iter().filter(|r| {
        r.symbol == symbol
            && r.context == q.context
            && r.policy == policy_revision()
            && r.status == ResolutionStatus::Resolved
    });
    let resolution = resolved.next()?;
    if resolved.next().is_some() {
        return None;
    }
    let EntityRef::Callable { callable } = data.refs.get(resolution.entity?)? else {
        return None;
    };
    let mut effective = data.callable_assessments.iter().filter(|a| {
        a.callable == *callable && a.context == q.context && a.policy == policy_revision()
    });
    let a = effective.next()?;
    if effective.next().is_some()
        || a.identity != Knowledge::Known
        || a.descriptor != Knowledge::Known
        || a.descriptor_kind != Some(DescriptorKind::ClassMethod)
    {
        return None;
    }
    if !data.callable_variants.iter().any(|v| {
        v.assessment == Some(a.id())
            && v.context == q.context
            && v.adjustment == SignatureAdjustment::BindClassReceiver
    }) {
        return None;
    }
    Some(a)
}
fn support_frame(
    data: &ReceiverData,
    s: Option<SupportAttribution>,
    q: &AssertionQualification,
    family: FactFamily,
    subject: Id<Occurrence>,
    premises: &mut Vec<ReceiverPremise>,
) -> Result<Id<ProviderRun>, ReceiverReason> {
    let s = s.ok_or(ReceiverReason::SupportDisagreement)?;
    let run = data.runs.get(s.run).ok_or(ReceiverReason::MissingSupport)?;
    let surface = data
        .surfaces
        .get(s.surface)
        .ok_or(ReceiverReason::MissingSupport)?;
    if run.context != q.context
        || Some(run.input) != scopes(data).input(q.scope)
        || surface.provider != run.provider
        || surface.family != family
        || s.fidelity != Fidelity::NativeStructural
    {
        return Err(ReceiverReason::SupportDisagreement);
    }
    let occurrence = data
        .occurrences
        .get(subject)
        .ok_or(ReceiverReason::SyntaxDisagreement)?;
    match data.native_evidence.get(s.evidence) {
        Some(Evidence::Invocation { run: observed })
            if family == FactFamily::Calls && *observed == s.run => {}
        Some(Evidence::Occurrence { occurrence: o }) if *o == subject => {}
        Some(Evidence::SourceSpan { source, start, end })
            if *source == occurrence.source
                && *start <= occurrence.start
                && *end >= occurrence.end => {}
        _ => return Err(ReceiverReason::SupportDisagreement),
    }
    let coverage: Vec<_> = data
        .coverage
        .iter()
        .filter(|c| {
            c.scope == q.scope
                && c.context == q.context
                && c.provider == Some(run.provider)
                && c.run == Some(s.run)
                && c.family == family
        })
        .collect();
    if coverage.is_empty()
        || coverage
            .iter()
            .any(|c| c.status != CoverageStatus::CompleteUnderStatedModel)
    {
        return Err(ReceiverReason::IncompleteCoverage);
    }
    for c in coverage {
        premises.push(ReceiverPremise::Coverage { coverage: c.id() });
    }
    Ok(s.run)
}
// Native calls and canonical syntax have independent providers. Compose their exact
// receipts only within one input, analysis context and extraction configuration.
fn same_source_frame(data: &ReceiverData, left: Id<ProviderRun>, right: Id<ProviderRun>) -> bool {
    data.runs
        .get(left)
        .zip(data.runs.get(right))
        .is_some_and(|(left, right)| {
            left.input == right.input
                && left.context == right.context
                && left.configuration == right.configuration
        })
}
fn derive(
    data: &ReceiverData,
    target: &CallTarget,
    premises: &mut Vec<ReceiverPremise>,
) -> Result<ReceiverAssessment, ReceiverReason> {
    let q = data
        .qualifications
        .get(target.qualification)
        .ok_or(ReceiverReason::QualificationDisagreement)?;
    if !matches!(q.modality, Modality::Definite | Modality::Candidate)
        || q.approximation != Approximation::Exact
        || target.origin != CallOrigin::explicit()
        || target.implicit
    {
        return Err(ReceiverReason::QualificationDisagreement);
    }
    let effective = known_descriptor(data, target, q).ok_or(ReceiverReason::DescriptorUnknown)?;
    let mut syntax = data.syntax.iter().filter(|s| {
        s.site == target.site
            && data
                .qualifications
                .get(s.qualification)
                .is_some_and(|cq| cq.context == q.context)
    });
    let call = syntax.next().ok_or(ReceiverReason::MissingSyntax)?;
    if syntax.next().is_some() {
        return Err(ReceiverReason::AmbiguousSyntax);
    }
    let cq = data
        .qualifications
        .get(call.qualification)
        .ok_or(ReceiverReason::QualificationDisagreement)?;
    if !frame(data, q, cq) || call.in_annotation {
        return Err(ReceiverReason::QualificationDisagreement);
    }
    let site = data
        .occurrences
        .get(call.site)
        .ok_or(ReceiverReason::SyntaxDisagreement)?;
    let callee = data
        .occurrences
        .get(call.callee)
        .ok_or(ReceiverReason::SyntaxDisagreement)?;
    if callee.syntax_kind != SyntaxKind::ExprAttribute
        || callee.source != site.source
        || site.start > callee.start
        || site.end < callee.end
    {
        return Err(ReceiverReason::SyntaxDisagreement);
    }
    // Count the complete qualified Value-edge domain. A conflicting frame cannot be silently
    // filtered out and used as evidence of uniqueness.
    let mut placements = data
        .placements
        .iter()
        .filter(|p| p.parent == Some(call.callee) && p.field == lexical::SyntaxField::Value);
    let placement = placements.next().ok_or(ReceiverReason::MissingPlacement)?;
    if placements.next().is_some() {
        return Err(ReceiverReason::AmbiguousPlacement);
    }
    let pq = data
        .qualifications
        .get(placement.qualification)
        .ok_or(ReceiverReason::QualificationDisagreement)?;
    let actual = data
        .occurrences
        .get(placement.occurrence)
        .ok_or(ReceiverReason::SyntaxDisagreement)?;
    if !frame(data, q, pq)
        || placement.ordinal != 0
        || actual.source != callee.source
        || actual.start < callee.start
        || actual.end > callee.end
    {
        return Err(ReceiverReason::QualificationDisagreement);
    }
    let mut supports = 0;
    let mut run = None;
    for support in data
        .target_supports
        .iter()
        .filter(|s| s.assertion == target.id())
    {
        supports += 1;
        premises.push(ReceiverPremise::Target {
            support: support.id(),
        });
        if support.origin != Origin::AnalyzerAssertion
            || support.mode != ExtractionMode::NativeTraversal
        {
            return Err(ReceiverReason::SupportDisagreement);
        }
        let observed = support_frame(
            data,
            support.attribution(),
            q,
            FactFamily::Calls,
            target.site,
            premises,
        )?;
        if run.is_some_and(|r| r != observed) {
            return Err(ReceiverReason::SupportDisagreement);
        }
        run = Some(observed);
    }
    if supports == 0 {
        return Err(ReceiverReason::MissingSupport);
    }
    let expected = run.unwrap();
    supports = 0;
    let mut syntax_run = None;
    for support in data
        .syntax_supports
        .iter()
        .filter(|s| s.assertion == call.id())
    {
        supports += 1;
        premises.push(ReceiverPremise::Syntax {
            support: support.id(),
        });
        if support.origin != Origin::SourceObservation
            || support.mode != ExtractionMode::NativeTraversal
        {
            return Err(ReceiverReason::SupportDisagreement);
        }
        let observed = support_frame(
            data,
            support.attribution(),
            cq,
            FactFamily::Syntax,
            call.site,
            premises,
        )?;
        if !same_source_frame(data, expected, observed)
            || syntax_run.is_some_and(|run| run != observed)
        {
            return Err(ReceiverReason::SupportDisagreement);
        }
        syntax_run = Some(observed);
    }
    if supports == 0 {
        return Err(ReceiverReason::MissingSupport);
    }
    supports = 0;
    for support in data
        .placement_supports
        .iter()
        .filter(|s| s.assertion == placement.id())
    {
        supports += 1;
        premises.push(ReceiverPremise::Placement {
            support: support.id(),
        });
        if support.origin != Origin::SourceObservation
            || support.mode != ExtractionMode::NativeTraversal
            || support_frame(
                data,
                support.attribution(),
                pq,
                FactFamily::Syntax,
                placement.occurrence,
                premises,
            )? != syntax_run.unwrap()
        {
            return Err(ReceiverReason::SupportDisagreement);
        }
    }
    if supports == 0 {
        return Err(ReceiverReason::MissingSupport);
    }
    Ok(ReceiverAssessment::ClassOf {
        target: target.id(),
        policy: policy_revision(),
        input: scopes(data).input(q.scope).unwrap(),
        context: q.context,
        syntax: call.id(),
        placement: placement.id(),
        effective: effective.id(),
        actual: actual.id(),
        members: ContentHash::of(b"pending"),
    })
}
/// Owner-produced compact admissions for the complete selected receiver candidate domain.
fn collect_native_premises(
    data: &ReceiverData,
    target: &CallTarget,
    premises: &mut Vec<ReceiverPremise>,
) {
    for support in data
        .target_supports
        .iter()
        .filter(|s| s.assertion == target.id())
    {
        premises.push(ReceiverPremise::Target {
            support: support.id(),
        });
    }
    for call in data.syntax.iter().filter(|s| {
        s.site == target.site
            && data
                .qualifications
                .get(s.qualification)
                .zip(data.qualifications.get(target.qualification))
                .is_some_and(|(cq, tq)| cq.context == tq.context)
    }) {
        for support in data
            .syntax_supports
            .iter()
            .filter(|s| s.assertion == call.id())
        {
            premises.push(ReceiverPremise::Syntax {
                support: support.id(),
            });
        }
        for placement in data
            .placements
            .iter()
            .filter(|p| p.parent == Some(call.callee) && p.field == lexical::SyntaxField::Value)
        {
            for support in data
                .placement_supports
                .iter()
                .filter(|s| s.assertion == placement.id())
            {
                premises.push(ReceiverPremise::Placement {
                    support: support.id(),
                });
            }
        }
    }
}
pub fn normalize_produced(
    data: &ReceiverData,
    budget: &resources::ResourceBudget,
) -> Result<(ReceiverOutput, VerifiedReceivers), ModelError> {
    normalize_targets(data, None, budget)
}
pub fn normalize(
    data: &ReceiverData,
    budget: &resources::ResourceBudget,
) -> Result<ReceiverOutput, ModelError> {
    Ok(normalize_targets(data, None, budget)?.0)
}
/// Derive one complete target domain from its exact stored premises. Ancillary targets in a
/// dependency closure do not become additional publication roots.
pub fn normalize_target(
    data: &ReceiverData,
    target: Id<CallTarget>,
    budget: &resources::ResourceBudget,
) -> Result<ReceiverOutput, ModelError> {
    if data.targets.get(target).is_none() {
        return Err(ModelError::Invalid("receiver root target absent".into()));
    }
    Ok(normalize_target_produced(data, target, budget)?.0)
}
/// Capture the applicable receiver at the actual owning derivation, without a second prepare.
pub fn normalize_target_produced(
    data: &ReceiverData,
    target: Id<CallTarget>,
    budget: &resources::ResourceBudget,
) -> Result<(ReceiverOutput, VerifiedReceivers), ModelError> {
    if data.targets.get(target).is_none() {
        return Err(ModelError::Invalid("receiver root target absent".into()));
    }
    normalize_targets(data, Some(target), budget)
}
fn normalize_targets(
    data: &ReceiverData,
    selected: Option<Id<CallTarget>>,
    budget: &resources::ResourceBudget,
) -> Result<(ReceiverOutput, VerifiedReceivers), ModelError> {
    let mut output = ReceiverOutput::new(budget);
    let mut verified = VerifiedReceivers {
        proofs: Default::default(),
        _charge: StateCharge::new(budget, "produced-receivers"),
    };
    for target in data
        .targets
        .iter()
        .filter(|t| selected.is_none_or(|selected| t.id() == selected) && candidates(data, t))
    {
        let mut held = StateCharge::new(budget, "receiver-premises");
        held.grow(
            (data.target_supports.len()
                + data.syntax_supports.len()
                + data.placement_supports.len()
                + data.coverage.len())
            .saturating_mul(256),
        )?;
        let mut premises = Vec::new();
        collect_native_premises(data, target, &mut premises);
        let assessed = derive(data, target, &mut premises);
        premises.sort_by_key(Record::id);
        premises.dedup();
        let mut digest = KeySink::new("class-of-receiver-premises");
        for p in &premises {
            p.id().encode(&mut digest);
        }
        let members = digest.finish();
        let assessment = match assessed {
            Ok(ReceiverAssessment::ClassOf {
                target,
                policy,
                input,
                context,
                syntax,
                placement,
                effective,
                actual,
                ..
            }) => ReceiverAssessment::ClassOf {
                target,
                policy,
                input,
                context,
                syntax,
                placement,
                effective,
                actual,
                members,
            },
            Err(reason) => ReceiverAssessment::Unknown {
                target: target.id(),
                policy: policy_revision(),
                reason,
                members,
            },
            _ => unreachable!(),
        };
        if let ReceiverAssessment::ClassOf {
            target,
            syntax,
            effective,
            actual,
            input,
            context,
            ..
        } = &assessment
        {
            verified.proofs.insert(
                &mut verified._charge,
                *target,
                ApplicableReceiver {
                    assessment: assessment.id(),
                    target: *target,
                    syntax: *syntax,
                    effective: *effective,
                    actual: *actual,
                    input: *input,
                    context: *context,
                },
            )?;
        }
        let assessment = output.receiver_assessments.insert(assessment)?;
        for premise in premises {
            let premise = output.receiver_premises.insert(premise)?;
            output.receiver_evidence.insert(ReceiverEvidence {
                assessment,
                premise,
            })?;
        }
    }
    Ok((output, verified))
}
pub fn verify(
    data: &ReceiverData,
    stored: &ReceiverOutput,
    budget: &resources::ResourceBudget,
) -> Result<VerifiedReceivers, ModelError> {
    stored.matches(&normalize(data, budget)?)?;
    if stored
        .receiver_assessments
        .iter()
        .any(|r| matches!(r, ReceiverAssessment::ClassOf { .. }))
    {
        let mut input = super::callable_normalization::CallableData::new(budget);
        let mut output = super::callable_normalization::CallableOutput::new(budget);
        macro_rules! inputs {($($field:ident: $ty:ty,)*)=>{$(for row in <ReceiverData as Source<$ty>>::rows(data).iter(){input.$field.insert(row.clone())?;})*};}
        macro_rules! outputs {($($field:ident: $ty:ty,)*)=>{$(for row in <ReceiverData as Source<$ty>>::rows(data).iter(){output.$field.insert(row.clone())?;})*};}
        crate::normalized_callable_inputs!(inputs);
        crate::normalized_callable_outputs!(outputs);
        output.matches(&super::callable_normalization::normalize(&input, budget)?)?;
    }
    prepare(data, stored, budget)
}

/// Admit expression-relative receiver applicability from completed checked callable inputs.
/// This checks the local receiver predicates without rerunning callable normalization.
pub fn admit(
    data: &ReceiverData,
    stored: &ReceiverOutput,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    prepare(data, stored, budget).map(|_| ())
}
pub(super) fn prepare(
    data: &ReceiverData,
    stored: &ReceiverOutput,
    budget: &resources::ResourceBudget,
) -> Result<VerifiedReceivers, ModelError> {
    let mut result = VerifiedReceivers {
        proofs: Default::default(),
        _charge: StateCharge::new(budget, "verified-receivers"),
    };
    for row in stored.receiver_assessments.iter() {
        if !data
            .targets
            .get(row.target())
            .is_some_and(|target| candidates(data, target))
        {
            return Err(ModelError::Invalid(
                "receiver outcome exceeds independent candidate domain".into(),
            ));
        }
    }
    for target in data
        .targets
        .iter()
        .filter(|target| candidates(data, target))
    {
        let mut outcomes = stored
            .receiver_assessments
            .iter()
            .filter(|assessment| assessment.target() == target.id());
        let row = outcomes.next().ok_or_else(|| {
            ModelError::Invalid("receiver candidate lacks required outcome".into())
        })?;
        if outcomes.next().is_some() {
            return Err(ModelError::Invalid(
                "receiver candidate has ambiguous outcome".into(),
            ));
        }
        let mut charge = StateCharge::new(budget, "receiver-admission-source-members");
        charge.grow(
            (data.target_supports.len()
                + data.syntax_supports.len()
                + data.placement_supports.len()
                + data.coverage.len())
            .saturating_mul(256),
        )?;
        let mut premises = Vec::new();
        collect_native_premises(data, target, &mut premises);
        let derived = derive(data, target, &mut premises);
        premises.sort_by_key(Record::id);
        premises.dedup();
        let mut digest = KeySink::new("class-of-receiver-premises");
        for premise in &premises {
            premise.id().encode(&mut digest);
            let evidence = ReceiverEvidence {
                assessment: row.id(),
                premise: premise.id(),
            };
            if stored.receiver_premises.get(premise.id()) != Some(premise)
                || stored.receiver_evidence.get(evidence.id()) != Some(&evidence)
            {
                return Err(ModelError::Invalid(
                    "receiver source membership differs".into(),
                ));
            }
        }
        if stored
            .receiver_evidence
            .iter()
            .filter(|evidence| evidence.assessment == row.id())
            .count()
            != premises.len()
        {
            return Err(ModelError::Invalid(
                "receiver source member domain differs".into(),
            ));
        }
        let expected_members = digest.finish();
        match (row, derived) {
            (
                ReceiverAssessment::Unknown {
                    policy,
                    reason,
                    members,
                    ..
                },
                Err(expected_reason),
            ) if *policy == policy_revision()
                && *reason == expected_reason
                && *members == expected_members => {}
            (
                ReceiverAssessment::ClassOf {
                    target,
                    syntax,
                    effective,
                    actual,
                    input,
                    context,
                    policy,
                    placement,
                    members,
                },
                Ok(ReceiverAssessment::ClassOf {
                    target: et,
                    syntax: es,
                    effective: ee,
                    actual: ea,
                    input: ei,
                    context: ec,
                    policy: ep,
                    placement: eplace,
                    ..
                }),
            ) if (
                *target, *syntax, *effective, *actual, *input, *context, *policy, *placement,
                *members,
            ) == (et, es, ee, ea, ei, ec, ep, eplace, expected_members) =>
            {
                result.proofs.insert(
                    &mut result._charge,
                    *target,
                    ApplicableReceiver {
                        assessment: row.id(),
                        target: *target,
                        syntax: *syntax,
                        effective: *effective,
                        actual: *actual,
                        input: *input,
                        context: *context,
                    },
                )?;
            }
            _ => {
                return Err(ModelError::Invalid(
                    "receiver applicability/source members differ from actual premises".into(),
                ));
            }
        }
    }
    Ok(result)
}
pub fn relations() -> Vec<Relation> {
    macro_rules! rel {($($field:ident: $ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    crate::normalized_receiver_outputs!(rel)
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    // Read eligibility retains the checked predecessor invariant closure without copying all
    // predecessor tables into this pure receiver kernel.
    let mut inputs = super::callable_normalization::stage(profile).inputs;
    macro_rules! prior {($($field:ident: $ty:ty,)*)=>{$(inputs.push(stages::RelationUse::completed::<$ty>());)*};}
    crate::normalized_callable_outputs!(prior);
    inputs.extend(ReceiverData::stage_inputs());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    if profile == stages::Profile::Catalog {
        inputs.retain(|r| r.name() != flow::FlowTestLeafObservation::NAME);
    }
    stages::Stage {
        name: "normalize_receivers",
        inputs: super::facts_stage_inputs(inputs),
        outputs: relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"receivers/class-of/v1"),
    }
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = ReceiverData::validation_inputs();
    inputs.extend(ReceiverOutput::validation_inputs());
    vec![
        Invariant {
            purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "normalized_receiver_closure",
            inputs: inputs.clone(),
            create: std::sync::Arc::new(|budget| {
                Box::new(Check {
                    input: ReceiverData::new(budget),
                    output: ReceiverOutput::new(budget),
                    budget: budget.clone(),
                    admission: false,
                })
            }),
        },
        Invariant {
            purpose: crate::domain::InvariantPurpose::Admission,
            revision: 1,
            name: "normalized_receiver_admission",
            inputs,
            create: std::sync::Arc::new(|budget| {
                Box::new(Check {
                    input: ReceiverData::new(budget),
                    output: ReceiverOutput::new(budget),
                    budget: budget.clone(),
                    admission: true,
                })
            }),
        },
    ]
}
struct Check {
    input: ReceiverData,
    output: ReceiverOutput,
    budget: resources::ResourceBudget,
    admission: bool,
}
impl InvariantCheck for Check {
    fn normalization_scope(&self) -> Option<super::admission::Scope> {
        self.admission.then_some(super::admission::Scope::Receivers)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.input.visit(name, batch)? && !self.output.visit(name, batch)? {
            return Err(ModelError::Invalid(
                "undeclared receiver validation input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.admission {
            return prepare(&self.input, &self.output, &self.budget).map(|_| ());
        }
        verify(&self.input, &self.output, &self.budget).map(|_| ())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec![
        "normalized_receiver_closure",
        "normalized_receiver_admission",
    ]
}

#[cfg(test)]
mod tests {
    use super::super::signature_applicability::{Application, BindingAuthority, establish};
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn class_of_capability_binds_only_a_class_receiver_and_never_an_instance_identity() {
        // This pure binder control starts after receiver replay. Native replay controls exercise
        // creation of the private capability from complete stored premises independently.
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let input = input::InputRevision::from_entries(vec![]).unwrap();
        let source = SourceArtifact::from_bytes(input.id(), "a.py".into(), b"c.m(x)").unwrap();
        let occurrence = |start, end, syntax_kind| Occurrence {
            source: source.id(),
            start,
            end,
            syntax_kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![start as i32],
        };
        let site = occurrence(0, 6, SyntaxKind::ExprCall);
        let actual = occurrence(0, 1, SyntaxKind::ExprName);
        let callee = occurrence(0, 3, SyntaxKind::ExprAttribute);
        let value = occurrence(4, 5, SyntaxKind::ExprName);
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"fixture"),
            environment_digest: ContentHash::of(b"fixture"),
            lock_digest: None,
        };
        let scope = CoverageScope::Input { input: input.id() };
        let qualification = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let provider = Provider {
            tool: "pure binder fixture".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"fixture"),
        };
        let symbol = ProviderSymbol {
            provider: provider.id(),
            context: context.id(),
            module: ProviderModule::Bundled {
                provider: provider.id(),
                bundle: ModuleBundle::Typeshed,
                name: "fixture".into(),
            }
            .id(),
            native_key: "m".into(),
            name: "m".into(),
            kind: SymbolKind::Method,
        };
        let shapes = vec![
            ParameterShape {
                name: Some("cls".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            },
            ParameterShape {
                name: Some("x".into()),
                kind: ParameterKind::PositionalOrKeyword,
                required: true,
            },
        ];
        let (signature, parameters) = Signature::new(
            &qualification,
            crate::domain::calls::SignatureRole::Source,
            None,
            symbol.id(),
            0,
            SignatureForm::List,
            &shapes,
        )
        .unwrap();
        let (call, arguments) = CallSyntax::new(
            qualification.id(),
            site.id(),
            callee.id(),
            false,
            &[Actual {
                kind: ArgumentKind::Positional,
                keyword: None,
                occurrence: value.id(),
            }],
        )
        .unwrap();
        let shapes: BTreeMap<_, _> = shapes.into_iter().map(|s| (s.id(), s)).collect();
        let receiver = Receiver::Unknown {
            reason: obligation::ObligationKind::AmbiguousBinding,
        };
        let channel = CallChannel::Direct;
        let destination = CallDestination::Resolved {
            symbol: symbol.id(),
        };
        let callable = CallableEntity::Source {
            declaration: site.id(),
            kind: CallableKind::Function,
        };
        let entity = EntityRef::Callable {
            callable: callable.id(),
        };
        let resolution = SymbolEntityResolution {
            symbol: symbol.id(),
            context: context.id(),
            policy: policy_revision(),
            status: ResolutionStatus::Resolved,
            entity: Some(entity.id()),
            reason: EntityReason::DeclarationAgreement,
        };
        let mut scopes = Rows::new(&budget);
        scopes.insert(scope).unwrap();
        let artifacts = Rows::new(&budget);
        let modules = Rows::new(&budget);
        for mutation in 0..6 {
            let target_q = AssertionQualification {
                modality: if mutation == 5 {
                    Modality::Candidate
                } else {
                    Modality::Definite
                },
                ..qualification.clone()
            };
            let target = CallTarget {
                qualification: target_q.id(),
                site: site.id(),
                origin: CallOrigin::explicit(),
                destination: destination.id(),
                channel: channel.id(),
                phase: CallPhase::Call,
                receiver: receiver.id(),
                implicit: false,
                receiver_class: None,
                passing: Some(ReceiverPassing::Object),
                class_method: Some(true),
                static_method: Some(mutation == 3),
            };
            let effective = EffectiveCallableAssessment {
                callable: callable.id(),
                context: context.id(),
                decorators: ContentHash::of(b"fixture"),
                policy: policy_revision(),
                identity: Knowledge::Known,
                identity_reason: CallableReason::EvidenceAgreement,
                signatures: Knowledge::Known,
                signature_reason: CallableReason::EvidenceAgreement,
                descriptor: Knowledge::Known,
                descriptor_kind: Some(if mutation == 1 {
                    DescriptorKind::InstanceMethod
                } else {
                    DescriptorKind::ClassMethod
                }),
                descriptor_reason: CallableReason::EvidenceAgreement,
                body: Knowledge::Known,
                body_admitted: true,
                body_reason: CallableReason::EvidenceAgreement,
                asynchronous: Some(false),
                generator: Some(false),
            };
            let variant = SignatureVariant {
                role: crate::domain::calls::SignatureRole::Source,
                native: None,
                signature: signature.id(),
                context: context.id(),
                resolution: resolution.id(),
                callable: Some(callable.id()),
                assessment: Some(effective.id()),
                adjustment: if mutation == 2 {
                    SignatureAdjustment::BindInstanceReceiver
                } else {
                    SignatureAdjustment::BindClassReceiver
                },
            };
            let proof = ApplicableReceiver {
                assessment: ReceiverAssessment::ClassOf {
                    target: target.id(),
                    policy: policy_revision(),
                    input: input.id(),
                    context: context.id(),
                    syntax: call.id(),
                    placement: SyntaxPlacement {
                        qualification: qualification.id(),
                        occurrence: actual.id(),
                        parent: Some(callee.id()),
                        field: lexical::SyntaxField::Value,
                        ordinal: 0,
                    }
                    .id(),
                    effective: effective.id(),
                    actual: actual.id(),
                    members: ContentHash::of(b"unit control capability"),
                }
                .id(),
                target: target.id(),
                syntax: call.id(),
                effective: effective.id(),
                actual: actual.id(),
                input: input.id(),
                context: context.id(),
            };
            let application = establish(Application {
                target: &target,
                qualification: &target_q,
                destination: &destination,
                channel: &channel,
                receiver: &receiver,
                dispatch_proof: None,
                receiver_proof: if mutation == 4 { None } else { Some(&proof) },
                signature: &signature,
                signature_qualification: &qualification,
                call: &call,
                call_qualification: &qualification,
                target_resolution: &resolution,
                signature_resolution: &resolution,
                entity: &entity,
                callable: &callable,
                variant: &variant,
                effective: Some(&effective),
                scopes: ScopeCatalog {
                    scopes: &scopes,
                    artifacts: &artifacts,
                    modules: &modules,
                },
            })
            .unwrap();
            let bound = bind(BindingInput {
                application: &application,
                parameters: &parameters,
                shapes: &shapes,
                arguments: &arguments,
            });
            if mutation == 0 || mutation == 5 {
                let bound = bound.unwrap();
                assert_eq!(
                    application.authority(),
                    if mutation == 0 {
                        BindingAuthority::EffectiveInvocation
                    } else {
                        BindingAuthority::SourceInspection
                    }
                );
                assert_eq!(
                    bound.bindings()[0].source,
                    BindingSource::ClassOf {
                        actual: actual.id()
                    }
                );
                assert_eq!(bound.bindings()[0].kind, BindingKind::Receiver);
                assert_eq!(
                    bound.bindings()[1].source,
                    BindingSource::Actual {
                        occurrence: value.id()
                    }
                );
            } else {
                assert_ne!(
                    application.authority(),
                    BindingAuthority::EffectiveInvocation
                );
                assert!(
                    bound.is_err(),
                    "mutation {mutation} must not lower an unknown into an instance actual"
                );
            }
        }
    }
}
