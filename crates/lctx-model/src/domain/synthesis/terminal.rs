//! The given-entry terminal question has its own S0 source, independently of completion proofs.
use super::{documentary, frames, summary};
use crate::domain::{
    analysis::{
        self,
        support::{DerivedEvidence, SourceFacts},
        synthesis as owner,
    },
    assertion::AssertionQualification,
    execution::{
        closed_targets::{ClosedTargetAssessment, TargetBasis},
        protocol_interpretation::{
            ConditionalTerminalFrontier, InvocationQuestion, NormalContinuationRestriction,
        },
        summary_consequences::SummaryClaim,
        summary_terminal::SummaryTerminalWitness,
    },
    normalized::Rows,
    resources::ResourceBudget,
    *,
};

#[macro_export]
macro_rules! synthesis_terminal_inputs {($m:ident)=>{$m!{
 witnesses:$crate::domain::execution::summary_terminal::SummaryTerminalWitness,
 frontiers:$crate::domain::execution::protocol_interpretation::ConditionalTerminalFrontier,
 restrictions:$crate::domain::execution::protocol_interpretation::NormalContinuationRestriction,
 targets:$crate::domain::execution::closed_targets::ClosedTargetAssessment,
 model_invocations:$crate::domain::analysis::model::AnalysisInvocation,
 summary_invocations:$crate::domain::analysis::summary::AnalysisInvocation,
 sets:$crate::domain::assumptions::AssumptionSet,
 members:$crate::domain::assumptions::AssumptionSetMember,
 assumptions:$crate::domain::assumptions::Assumption,
 native:$crate::domain::analysis::native::NativeAssertionPremise,
}};}
macro_rules! data {($($f:ident:$t:ty,)*)=>{pub struct Data{budget:ResourceBudget,scope:ownership::ScopeIndex,$(pub $f:Rows<$t>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{budget:b.clone(),scope:ownership::ScopeIndex::new(b,"terminal-question-scope"),$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{if self.scope.visit(n,b)?{return Ok(true);}$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{let mut rows=vec![$(ValidationInput::of::<$t>(&["id"]),)*];rows.extend(ownership::ScopeIndex::inputs());rows}}};}
crate::synthesis_terminal_inputs!(data);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("given-entry terminal premise absent"))
}
/// A checked scoped question. No field certifies body entry, effects, exceptions or cleanup.
pub struct Checked<'a> {
    pub(crate) input: Id<input::InputRevision>,
    pub(crate) scope: Id<source::CoverageScope>,
    pub(crate) context: Id<attribution::AnalysisContext>,
    pub(crate) witness: &'a SummaryTerminalWitness,
    pub(crate) frontier: &'a ConditionalTerminalFrontier,
    pub(crate) restriction: &'a NormalContinuationRestriction,
    pub(crate) target: &'a ClosedTargetAssessment,
    pub(crate) derivation: &'a analysis::summary::AnalysisDerivation,
    pub(crate) proposition: &'a analysis::summary::AnalysisProposition,
    pub(crate) premise: &'a analysis::summary::AnalysisDerivationPremise,
    pub(crate) source: &'a analysis::summary::SupportSource,
}
impl Checked<'_> {
    pub fn facts(&self) -> SourceFacts {
        self.witness.source_facts()
    }
    pub fn proof(&self) -> Vec<derivation::RowRef> {
        let mut refs = vec![
            derivation::RowRef::of(self.witness.id()),
            derivation::RowRef::of(self.frontier.id()),
            derivation::RowRef::of(self.restriction.id()),
            derivation::RowRef::of(self.target.id()),
            derivation::RowRef::of(self.witness.claim),
            derivation::RowRef::of(self.witness.invocation),
            derivation::RowRef::of(self.frontier.invocation),
            derivation::RowRef::of(self.derivation.id()),
            derivation::RowRef::of(self.proposition.id()),
            derivation::RowRef::of(self.premise.id()),
            derivation::RowRef::of(self.source.id()),
            derivation::RowRef::of(self.frontier.declared_return),
            derivation::RowRef::of(self.frontier.qualification),
            derivation::RowRef::of(self.scope),
        ];
        refs.extend(
            [
                Some(self.frontier.native),
                Some(self.restriction.statement_native),
                Some(self.restriction.following_native),
                self.target.target_native,
                self.target.final_native,
                self.target.member_native,
            ]
            .into_iter()
            .flatten()
            .map(derivation::RowRef::of),
        );
        refs.extend(
            [
                self.target.original_qualification,
                self.target.receiver_qualification,
            ]
            .into_iter()
            .flatten()
            .map(derivation::RowRef::of),
        );
        refs.extend(self.target.ancestry.into_iter().map(derivation::RowRef::of));
        refs.extend(
            self.target
                .universe_support
                .into_iter()
                .map(derivation::RowRef::of),
        );
        refs.sort();
        refs.dedup();
        refs
    }
}
/// Shared by production, replay and packet hydration; lower Model admission remains its owner.
pub fn checked<'a>(
    d: &'a Data,
    s: &'a summary::Data,
    witness: Id<SummaryTerminalWitness>,
    q: &AssertionQualification,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
) -> Result<Checked<'a>, ModelError> {
    let w = need(&d.witnesses, witness)?;
    let f = need(&d.frontiers, w.frontier)?;
    let edge = need(&d.restrictions, w.restriction)?;
    let target = need(&d.targets, f.target)?;
    let parent = need(&d.model_invocations, f.invocation)?;
    let invocation = need(&d.summary_invocations, w.invocation)?;
    if (parent.input, parent.context) != (input, context)
        || (invocation.input, invocation.context) != (input, context)
        || q.context != context
        || !d.scope.owns_scope(input, d.scope.scope(q.scope)?)?
        || q.modality != attribution::Modality::Definite
        || q.approximation != assertion::Approximation::Exact
        || q.assumptions == assumptions::AssumptionSet::empty_id()
    {
        return Err(invalid(
            "terminal question changes its exact input/context or conditional basis",
        ));
    }
    let claim = SummaryClaim::NoNormalContinuation {
        owner: f.owner,
        frontier: f.id(),
        restriction: edge.id(),
        qualification: q.id(),
        question: InvocationQuestion::GivenInvocationEntered,
    };
    if need(&s.claims, w.claim)? != &claim
        || (
            w.frontier,
            w.restriction,
            w.qualification,
            w.question,
            w.status,
        ) != (
            f.id(),
            edge.id(),
            q.id(),
            InvocationQuestion::GivenInvocationEntered,
            analysis::policy::EvidenceStatus::StructurallyObserved,
        )
        || (edge.frontier, edge.owner, edge.from, edge.qualification)
            != (f.id(), f.owner, f.statement, q.id())
        || f.qualification != q.id()
        || f.question != w.question
        || !f.effects_unknown
        || !f.exceptions_unknown
        || target.invocation != f.invocation
        || target.basis == TargetBasis::Open
        || target.reason.is_some()
    {
        return Err(invalid(
            "terminal question changes its admitted claim or lower frontier",
        ));
    }
    if target.basis == TargetBasis::TypingConditional
        && (target.receiver_conformance.is_none()
            || target.no_extra_overrides.is_none()
            || target.universe_support.is_none()
            || target.ancestry.is_none())
    {
        return Err(invalid(
            "terminal typing closure loses its exact receiver/MRO/universe basis",
        ));
    }
    if !matches!(need(&d.assumptions,f.conformance)?,assumptions::Assumption::TypeConformance{observation,..} if *observation==f.declared_return)
    {
        return Err(invalid(
            "terminal declared-return conformance changes its observation",
        ));
    }
    if !matches!(need(&d.native,f.native)?,analysis::native::NativeAssertionPremise::NativeTerminal{assertion,..} if *assertion==f.observation)
    {
        return Err(invalid("terminal native source changes its observation"));
    }
    for (native, placement) in [
        (edge.statement_native, edge.statement),
        (edge.following_native, edge.following),
    ] {
        if !matches!(need(&d.native,native)?,analysis::native::NativeAssertionPremise::SyntaxPlacement{assertion,..} if *assertion==placement)
        {
            return Err(invalid(
                "terminal edge changes its canonical placement source",
            ));
        }
    }
    let _basis_charge = d.budget.reserve(
        "terminal-question-basis",
        (d.sets.len() + d.members.len() + d.assumptions.len()).saturating_mul(256),
    )?;
    let sets = d.sets.iter().map(|r| (r.id(), r.clone())).collect();
    let members = d.members.iter().map(|r| (r.id(), r.clone())).collect();
    let definitions = d.assumptions.iter().map(|r| (r.id(), r.clone())).collect();
    let basis = assumptions::AssumptionResolver::resolve(
        &assumptions::AssumptionCatalog {
            sets: &sets,
            members: &members,
            definitions: &definitions,
        },
        q.assumptions,
    )?;
    for id in [
        Some(f.conformance),
        target.receiver_conformance,
        target.no_extra_overrides,
    ]
    .into_iter()
    .flatten()
    {
        if !basis.members.iter().any(|m| m.assumption == id) {
            return Err(invalid("terminal question loses a required lower premise"));
        }
    }
    for id in [
        Some(f.native),
        Some(edge.statement_native),
        Some(edge.following_native),
        target.target_native,
        target.final_native,
        target.member_native,
    ]
    .into_iter()
    .flatten()
    {
        need(&d.native, id)?;
    }
    let source = analysis::summary::SupportSource::TerminalFrontier { witness: w.id() };
    let source = need(&s.sources, source.id())?;
    let subject = analysis::summary::ObligationSubject::SummaryClaim {
        transfer: claim.id(),
    };
    need(&s.subjects, subject.id())?;
    let mut found = s.derivations.iter().filter(|r| {
        r.invocation == w.invocation
            && r.qualification == q.id()
            && r.operation == analysis::support::QualificationOperation::Conjunction
            && s.propositions.get(r.proposition).is_some_and(|p| {
                p.subject == subject.id()
                    && p.channel == analysis::AnalysisChannel::Role
                    && p.phase == calls::CallPhase::Call
                    && p.qualification == q.id()
            })
    });
    let derived = found
        .next()
        .ok_or_else(|| invalid("terminal Summary Role/Call derivation absent"))?;
    if found.next().is_some() {
        return Err(invalid("terminal Summary derivation ambiguous"));
    }
    if derived.status != w.status || derived.heuristic {
        return Err(invalid("terminal Summary derived evidence status differs"));
    }
    let mut premises = s.premises.iter().filter(|p| p.derivation == derived.id());
    let premise = premises
        .next()
        .ok_or_else(|| invalid("terminal Summary source premise absent"))?;
    if premise.source != source.id() || premises.next().is_some() {
        return Err(invalid("terminal Summary source13 premise differs"));
    }
    Ok(Checked {
        input,
        scope: q.scope,
        context,
        witness: w,
        frontier: f,
        restriction: edge,
        target,
        derivation: derived,
        proposition: need(&s.propositions, derived.proposition)?,
        premise,
        source,
    })
}
/// Exact public-member correspondence is the existing documentary identity operation.
pub(super) fn linked(
    d: &documentary::Data,
    entity: Id<normalized::entities::EntityRef>,
    inv: &owner::Invocation,
) -> Result<Vec<Id<catalog::CatalogMemberInvocation>>, ModelError> {
    let mut out = vec![];
    for member in d.member_frames.iter().filter(|m| {
        d.core_invocations
            .get(m.invocation)
            .is_some_and(|p| (p.input, p.context) == (inv.input, inv.context))
    }) {
        let exposed = |id| {
            d.exposures.get(id).is_some_and(|e| {
                e.member == member.member
                    && d.public_exposures
                        .get(e.exposure)
                        .is_some_and(|p| p.context == inv.context)
            })
        };
        let candidate = d.candidates.iter().any(|c| {
            exposed(c.exposure)
                && documentary::entity(d, c).is_ok_and(|actual| actual == Some(entity))
        });
        if candidate
            || d.aliases
                .iter()
                .any(|a| a.entity == entity && exposed(a.parent))
        {
            out.push(member.id());
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}
pub(super) fn selected<'a>(
    d: &'a Data,
    s: &'a summary::Data,
    q: &Rows<AssertionQualification>,
    frame: &frames::Frame,
    inv: &owner::Invocation,
) -> Result<Vec<Checked<'a>>, ModelError> {
    let mut out = vec![];
    for w in d.witnesses.iter().filter(|w| w.invocation == frame.summary) {
        out.push(checked(
            d,
            s,
            w.id(),
            need(q, w.qualification)?,
            inv.input,
            inv.context,
        )?);
    }
    Ok(out)
}
