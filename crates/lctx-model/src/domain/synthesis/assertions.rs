//! One programmatic assertion emitter. Display text is never evidence.
use super::documentary::{self, DocumentaryConclusion};
use crate::domain::{
    analysis::{
        self,
        policy::{AssertionKind, BriefSection, EvidenceStatus, SupportRole},
        support::{DerivedEvidence, SourceFacts},
        synthesis as owner,
    },
    assertion::AssertionQualification,
    catalog::CatalogMemberInvocation,
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainSum};
/// Exact retained A0 premises for deterministic control wording. These are not new findings.
#[macro_export]
macro_rules! synthesis_control_text_inputs {
    ($m:ident) => {
        $m! {
         control_paths:$crate::domain::structural::controls::ControlPath,
         control_traversals:$crate::domain::structural::controls::ControlTraversal,
         control_steps:$crate::domain::structural::controls::ControlStep,
         argument_flows:$crate::domain::structural::controls::ArgumentFlow,
         literal_arguments:$crate::domain::structural::controls::LiteralArgument,
         conditional_raises:$crate::domain::structural::controls::ConditionalRaise,
         unfollowed_paths:$crate::domain::structural::controls::UnfollowedPath,
         unfollowed_arguments:$crate::domain::structural::controls::UnfollowedArgument,
         call_bindings:$crate::domain::normalized::bindings::CallBinding,
         binding_sources:$crate::domain::calls::BindingSource,
         entries:$crate::domain::conditions::entry::EntryValueWitness,
         leaves:$crate::domain::flow::FlowTestLeafObservation,
         regions:$crate::domain::flow::FlowRegionObservation,
         parameter_entities:$crate::domain::normalized::entities::ParameterEntity,
        }
    };
}
macro_rules! control_data{($($f:ident:$t:ty,)*)=>{pub struct ControlData{$(pub $f:Rows<$t>,)*}impl ControlData{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}}};}
crate::synthesis_control_text_inputs!(control_data);
pub const TEMPLATE_VERSION: i64 = 2;
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "synthesis_assertion_templates")]
pub enum AssertionTemplate {
    #[model(code = 0)]
    AuthoredOutcome {
        conclusion: Id<DocumentaryConclusion>,
    },
    #[model(code = 1)]
    StructuralObservation {
        conclusion: Id<structural::Conclusion>,
    },
    #[model(code = 2)]
    AnalyticObservation {
        conclusion: Id<analytics::Conclusion>,
    },
    #[model(code = 3)]
    Summary {
        facet: Id<super::summary::SummaryFacet>,
    },
    #[model(code = 4)]
    AuthoredCode {
        conclusion: Id<super::patterns::AuthoredCodeConclusion>,
    },
    #[model(code = 5)]
    AuthoredComponent {
        conclusion: Id<DocumentaryConclusion>,
    },
    #[model(code = 6)]
    TerminalSummary {
        witness: Id<execution::summary_terminal::SummaryTerminalWitness>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "programmatic_assertion_sources")]
pub enum AssertionSource {
    #[model(code = 0)]
    Documentary {
        conclusion: Id<DocumentaryConclusion>,
    },
    #[model(code = 1)]
    Structural {
        conclusion: Id<structural::Conclusion>,
    },
    #[model(code = 2)]
    Analytic {
        conclusion: Id<analytics::Conclusion>,
    },
    #[model(code = 3)]
    Summary {
        facet: Id<super::summary::SummaryFacet>,
    },
    #[model(code = 4)]
    AuthoredCode {
        conclusion: Id<super::patterns::AuthoredCodeConclusion>,
    },
    #[model(code = 5)]
    TerminalSummary {
        witness: Id<execution::summary_terminal::SummaryTerminalWitness>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="programmatic_assertions",invariant_refs=invariants_refs,semantic_source=include_bytes!("assertions.rs"))]
pub struct ProgrammaticAssertion {
    #[model(key)]
    pub invocation: Id<owner::Invocation>,
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
    #[model(key)]
    pub template: Id<AssertionTemplate>,
    #[model(key)]
    pub version: i64,
    kind: AssertionKind,
    section: BriefSection,
    status: EvidenceStatus,
    qualification: Id<AssertionQualification>,
    text: Utf8Text,
}
impl ProgrammaticAssertion {
    pub fn kind(&self) -> AssertionKind {
        self.kind
    }
    pub fn section(&self) -> BriefSection {
        self.section
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
    pub fn text(&self) -> &str {
        self.text.as_str()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="programmatic_assertion_supports",rule="programmatic_assertion_support",conclusion=assertion)]
pub struct ProgrammaticAssertionSupport {
    #[model(key)]
    pub assertion: Id<ProgrammaticAssertion>,
    #[model(key)]
    pub role: SupportRole,
    #[model(key, premise)]
    pub source: Id<AssertionSource>,
}
macro_rules! output_rows{($apply:ident)=>{$apply!{assertions:ProgrammaticAssertion,templates:AssertionTemplate,sources:AssertionSource,supports:ProgrammaticAssertionSupport,}};}
macro_rules! output{($($field:ident:$ty:ty,)*)=>{pub struct Output{$(pub $field:Rows<$ty>,)*}impl Output{pub fn new(b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$field.decode(b)?;return Ok(true);})*Ok(false)}pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}pub fn matches(&self,expected:&Self)->Result<(),ModelError>{$(if !self.$field.same(&expected.$field){return Err(invalid(concat!("programmatic assertion closure differs: ",stringify!($field))));})*Ok(())}}};}
output_rows!(output);
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.required(id, || {
        invalid(format!("programmatic assertion input absent: {}", R::NAME))
    })
}
/// All constructors use the same status/policy operation. Requested status is not an input.
struct AssertionContent<'a> {
    kind: AssertionKind,
    source: &'a AssertionSource,
    facts: SourceFacts,
    text: String,
}

fn emit(
    invocation: Id<owner::Invocation>,
    member: Id<CatalogMemberInvocation>,
    template: &AssertionTemplate,
    assertion_content: AssertionContent<'_>,
    b: &ResourceBudget,
) -> Result<(ProgrammaticAssertion, ProgrammaticAssertionSupport), ModelError> {
    let AssertionContent {
        kind,
        source,
        facts,
        text,
    } = assertion_content;
    let _admission = b.reserve(
        "programmatic-assertion-emitter",
        text.len() + size_of::<ProgrammaticAssertion>(),
    )?;
    let status = analysis::policy::derive_status(&[(SupportRole::Support, facts.status)]);
    let section = analysis::policy::assertion_policy(kind, status)?;
    let row = ProgrammaticAssertion {
        invocation,
        member,
        template: template.id(),
        version: TEMPLATE_VERSION,
        kind,
        section,
        status,
        qualification: facts.qualification,
        text: text.into(),
    };
    let support = ProgrammaticAssertionSupport {
        assertion: row.id(),
        role: SupportRole::Support,
        source: source.id(),
    };
    Ok((row, support))
}
/// A documentary Outcome is an excerpt from exact earlier authored bytes, retaining candidate linkage.
pub fn authored_outcome(
    d: &documentary::Data,
    rows: &documentary::Output,
    invocation: &owner::Invocation,
    conclusion: &DocumentaryConclusion,
    b: &ResourceBudget,
) -> Result<
    (
        AssertionTemplate,
        AssertionSource,
        ProgrammaticAssertion,
        ProgrammaticAssertionSupport,
    ),
    ModelError,
> {
    if invocation.definition != super::build::definition().1.id() {
        return Err(invalid(
            "synthesis invocation has a noncanonical definition",
        ));
    }
    let source = need(&rows.sources, conclusion.source)?;
    let frame = need(&d.member_frames, source.member())?;
    let member = need(&d.members, frame.member)?;
    let core = need(&d.core_invocations, frame.invocation)?;
    let q = need(&rows.qualifications, conclusion.qualification())?;
    if (invocation.input, invocation.context) != (core.input, core.context)
        || member.input != invocation.input
        || q.context != invocation.context
        || q.scope
            != (source::CoverageScope::Input {
                input: invocation.input,
            })
            .id()
    {
        return Err(invalid(
            "authored assertion crosses exact release/context association",
        ));
    }
    if conclusion.status() != EvidenceStatus::Documented {
        return Err(invalid("authored outcome lacks documentary evidence floor"));
    }
    let slice = need(&rows.slices, conclusion.excerpt)?;
    let excerpt = documentary::read_slice(d, rows, slice, b)?;
    if ContentHash::of(excerpt.value.as_bytes()) != conclusion.excerpt_digest {
        return Err(invalid("authored assertion excerpt digest differs"));
    }
    let module = need(&d.modules, member.access)?;
    let allowance = module.qualified_name.len()
        + member.path.iter().map(String::len).sum::<usize>()
        + member.path.len()
        + excerpt.value.len()
        + 128;
    let _render = b.reserve("authored-assertion-render", allowance)?;
    let mut text = String::with_capacity(allowance);
    text.push_str("Authored prose for public candidate `");
    text.push_str(&module.qualified_name);
    for segment in &member.path {
        text.push('.');
        text.push_str(segment);
    }
    text.push_str("`: ");
    let mut words = excerpt.value.split_whitespace();
    if let Some(first) = words.next() {
        text.push_str(first);
        for word in words {
            text.push(' ');
            text.push_str(word);
        }
    }
    let (kind, text) = if let documentary::DocumentarySource::Component { role, .. } = source {
        (
            if *role == super::documentary_templates::ComponentRole::Warning {
                AssertionKind::DocumentedWarning
            } else {
                AssertionKind::Parameter
            },
            super::documentary_templates::text(d, rows, conclusion, b)?,
        )
    } else {
        (AssertionKind::Outcome, text)
    };
    let template = if matches!(source, documentary::DocumentarySource::Component { .. }) {
        AssertionTemplate::AuthoredComponent {
            conclusion: conclusion.id(),
        }
    } else {
        AssertionTemplate::AuthoredOutcome {
            conclusion: conclusion.id(),
        }
    };
    let source = AssertionSource::Documentary {
        conclusion: conclusion.id(),
    };
    let (assertion, support) = emit(
        invocation.id(),
        frame.id(),
        &template,
        AssertionContent {
            kind,
            source: &source,
            facts: conclusion.source_facts(),
            text,
        },
        b,
    )?;
    Ok((template, source, assertion, support))
}
pub fn build_documentary(
    d: &documentary::Data,
    rows: &documentary::Output,
    invocations: &Rows<owner::Invocation>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    for invocation in invocations.iter() {
        for conclusion in rows.conclusions.iter() {
            let source = need(&rows.sources, conclusion.source)?;
            let frame = need(&d.member_frames, source.member())?;
            let core = need(&d.core_invocations, frame.invocation)?;
            if (core.input, core.context) != (invocation.input, invocation.context)
                || conclusion.status() != EvidenceStatus::Documented
            {
                continue;
            }
            let (template, source, assertion, support) =
                authored_outcome(d, rows, invocation, conclusion, b)?;
            out.templates.insert(template)?;
            out.sources.insert(source)?;
            out.assertions.insert(assertion)?;
            out.supports.insert(support)?;
        }
    }
    Ok(out)
}
/// Exact public slots are associated by earlier A0 identity, preserving aliases independently.
#[allow(
    clippy::too_many_arguments,
    reason = "Public assertion construction keeps separate documentary, observation, seed and invocation owners explicit."
)]
pub fn build(
    d: &documentary::Data,
    docs: &documentary::Output,
    o: &super::observations::Data,
    controls: &ControlData,
    public: &Rows<structural::PublicCandidate>,
    frames: &Rows<super::frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = build_documentary(d, docs, invocations, b)?;
    for frame in frames.iter() {
        let inv = need(invocations, frame.invocation)?;
        for candidate in public.iter().filter(|p| p.frame == frame.structural) {
            for member in d.member_frames.iter().filter(|m| {
                m.member == candidate.member
                    && d.core_invocations
                        .get(m.invocation)
                        .is_some_and(|i| (i.input, i.context) == (inv.input, inv.context))
            }) {
                for conclusion in o
                    .structural_conclusions
                    .iter()
                    .filter(|r| r.frame == frame.structural && r.subject == candidate.entity)
                {
                    let q = need(&o.qualifications, conclusion.qualification())?;
                    if q.context != inv.context
                        || q.scope != (source::CoverageScope::Input { input: inv.input }).id()
                    {
                        return Err(invalid("structural assertion changes its static frame"));
                    }
                    let source = need(&o.structural_sources, conclusion.source)?;
                    let (kind, text) = structural_text(
                        d,
                        controls,
                        source,
                        conclusion.frame,
                        conclusion.subject,
                        q.context,
                        b,
                    )?;
                    let template = AssertionTemplate::StructuralObservation {
                        conclusion: conclusion.id(),
                    };
                    let source = AssertionSource::Structural {
                        conclusion: conclusion.id(),
                    };
                    let (assertion, support) = emit(
                        inv.id(),
                        member.id(),
                        &template,
                        AssertionContent {
                            kind,
                            source: &source,
                            facts: conclusion.source_facts(),
                            text,
                        },
                        b,
                    )?;
                    out.templates.insert(template)?;
                    out.sources.insert(source)?;
                    out.assertions.insert(assertion)?;
                    out.supports.insert(support)?;
                }
                for conclusion in o
                    .analytic_conclusions
                    .iter()
                    .filter(|r| r.frame == frame.analytic && r.subject == candidate.entity)
                {
                    let q = need(&o.qualifications, conclusion.qualification())?;
                    if q.context != inv.context
                        || q.scope != (source::CoverageScope::Input { input: inv.input }).id()
                    {
                        return Err(invalid("analytic assertion changes its static frame"));
                    }
                    let source = need(&o.analytic_sources, conclusion.source)?;
                    let (kind, text) = analytic_text(source);
                    let template = AssertionTemplate::AnalyticObservation {
                        conclusion: conclusion.id(),
                    };
                    let source = AssertionSource::Analytic {
                        conclusion: conclusion.id(),
                    };
                    let (assertion, support) = emit(
                        inv.id(),
                        member.id(),
                        &template,
                        AssertionContent {
                            kind,
                            source: &source,
                            facts: conclusion.source_facts(),
                            text: text.into(),
                        },
                        b,
                    )?;
                    out.templates.insert(template)?;
                    out.sources.insert(source)?;
                    out.assertions.insert(assertion)?;
                    out.supports.insert(support)?;
                }
            }
        }
    }
    Ok(out)
}
#[allow(
    clippy::too_many_arguments,
    reason = "Summary assertion lowering keeps the separately owned qualification and nominal proof inputs explicit."
)]
pub fn extend_summary(
    d: &super::summary::Data,
    observations: &super::observations::Data,
    facets: &Rows<super::summary::SummaryFacet>,
    frames: &Rows<super::frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    for facet in facets.iter() {
        let (Some(member), Some(source)) = (facet.member, facet.source) else {
            continue;
        };
        let frame = need(frames, facet.frame)?;
        let inv = need(invocations, frame.invocation)?;
        let earlier = need(&d.conclusions, facet.conclusion)?;
        let actual = super::summary::evidence(d, earlier)?
            .ok_or_else(|| invalid("Summary assertion lacks proof"))?;
        if actual.id() != source {
            return Err(invalid("Summary assertion changes source"));
        }
        let owner::SupportSource::Summary { derivation } = actual else {
            return Err(invalid("Summary assertion changes owner"));
        };
        let derived = need(&d.derivations, derivation)?;
        let facts = derived.source_facts();
        let template = AssertionTemplate::Summary { facet: facet.id() };
        let source = AssertionSource::Summary { facet: facet.id() };
        let qualification = need(&observations.qualifications, facts.qualification)?;
        let text = super::summary::text(d, facet, Some(qualification), b)?;
        let (assertion, support) = emit(
            inv.id(),
            member,
            &template,
            AssertionContent {
                kind: if facet.verdict == obligation::Verdict::RefutedUnderModel {
                    AssertionKind::BehavioralRefutation
                } else {
                    AssertionKind::ApplicableCase
                },
                source: &source,
                facts,
                text,
            },
            b,
        )?;
        out.templates.insert(template)?;
        out.sources.insert(source)?;
        out.assertions.insert(assertion)?;
        out.supports.insert(support)?;
    }
    Ok(())
}
#[allow(
    clippy::too_many_arguments,
    reason = "Public assertion construction keeps separate documentary, observation, summary and seed owners explicit."
)]
pub fn build_all(
    d: &documentary::Data,
    docs: &documentary::Output,
    o: &super::observations::Data,
    controls: &ControlData,
    summary: &super::summary::Data,
    terminal: &super::terminal::Data,
    patterns: &super::patterns::Data,
    public: &Rows<structural::PublicCandidate>,
    frames: &Rows<super::frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = build(d, docs, o, controls, public, frames, invocations, b)?;
    let (facets, _) = super::summary::build(summary, d, frames, invocations, b)?;
    extend_summary(summary, o, &facets, frames, invocations, &mut out, b)?;
    extend_terminal(terminal, summary, o, d, frames, invocations, &mut out, b)?;
    let code = super::patterns::build(patterns, d, frames, invocations, b)?;
    extend_patterns(patterns, d, &code, frames, invocations, &mut out, b)?;
    Ok(out)
}
#[allow(
    clippy::too_many_arguments,
    reason = "Terminal lowering retains separate nominal owners and qualified evidence."
)]
fn extend_terminal(
    d: &super::terminal::Data,
    summary: &super::summary::Data,
    observations: &super::observations::Data,
    docs: &documentary::Data,
    frames: &Rows<super::frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    for frame in frames.iter() {
        let inv = need(invocations, frame.invocation)?;
        for checked in
            super::terminal::selected(d, summary, &observations.qualifications, frame, inv)?
        {
            for member in super::terminal::linked(docs, checked.frontier.owner, inv)? {
                let template = AssertionTemplate::TerminalSummary {
                    witness: checked.witness.id(),
                };
                let source = AssertionSource::TerminalSummary {
                    witness: checked.witness.id(),
                };
                let (assertion,support)=emit(inv.id(),member,&template,AssertionContent{kind:AssertionKind::ApplicableCase,source:&source,facts:checked.facts(),text:"Given entry to this invocation and the retained typing assumptions, the direct following statement has no normal continuation. Invocation entry was not established; effects, exceptions and cleanup remain unknown.".into()},b)?;
                out.templates.insert(template)?;
                out.sources.insert(source)?;
                out.assertions.insert(assertion)?;
                out.supports.insert(support)?;
            }
        }
    }
    Ok(())
}
pub fn extend_patterns(
    d: &super::patterns::Data,
    docs: &documentary::Data,
    code: &super::patterns::Output,
    frames: &Rows<super::frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    for conclusion in code.conclusions.iter() {
        let source = need(&code.sources, conclusion.source)?;
        let frame = need(frames, source.frame)?;
        let inv = need(invocations, frame.invocation)?;
        let bytes = super::patterns::code(d, docs, code, conclusion, inv.context, b)?;
        let _render = b.reserve("authored-code-assertion", bytes.value.len() + 256)?;
        let template = AssertionTemplate::AuthoredCode {
            conclusion: conclusion.id(),
        };
        let support = AssertionSource::AuthoredCode {
            conclusion: conclusion.id(),
        };
        let text = format!(
            "Authored official code documents this producer-to-consumer handoff, including its required setup. Execution was not established.\n```python\n{}```",
            bytes.value
        );
        let (assertion, member) = emit(
            inv.id(),
            source.member,
            &template,
            AssertionContent {
                kind: AssertionKind::UsagePattern,
                source: &support,
                facts: conclusion.source_facts(),
                text,
            },
            b,
        )?;
        out.templates.insert(template)?;
        out.sources.insert(support)?;
        out.assertions.insert(assertion)?;
        out.supports.insert(member)?;
    }
    Ok(())
}
fn parameter_text(
    d: &documentary::Data,
    c: &ControlData,
    formal: Id<normalized::entities::ParameterEntity>,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<String, ModelError> {
    let mut name: Option<&str> = None;
    for link in d.parameter_links.iter().filter(|l| l.entity == formal) {
        let parameter = need(&d.signature_parameters, link.parameter)?;
        let signature = need(&d.parameter_signatures, parameter.signature)?;
        if need(&d.qualifications, signature.qualification)?.context != context {
            continue;
        }
        if let Some(n) = need(&d.parameter_shapes, parameter.shape)?.name.as_ref() {
            if name.is_some_and(|old| old != n.as_str()) {
                return Err(invalid("control parameter names disagree in exact context"));
            }
            name = Some(n.as_str());
        }
    }
    if let Some(name) = name {
        return Ok(name.to_owned());
    }
    match need(&c.parameter_entities, formal)? {
        normalized::entities::ParameterEntity::Source { declaration } => {
            occurrence_text(d, *declaration, b)
        }
        normalized::entities::ParameterEntity::NativeSlot { parameter, .. } => Ok(need(
            &d.parameter_shapes,
            need(&d.signature_parameters, *parameter)?.shape,
        )?
        .name
        .as_ref()
        .map_or_else(
            || format!("parameter {}", formal.hex()),
            |n| n.as_str().to_owned(),
        )),
    }
}
fn occurrence_text(
    d: &documentary::Data,
    occurrence: Id<source::Occurrence>,
    b: &ResourceBudget,
) -> Result<String, ModelError> {
    let occurrence = need(&d.occurrences, occurrence)?;
    Ok(documentary::read_range(d, occurrence.source, occurrence.start, occurrence.end, b)?.value)
}
fn structural_text(
    d: &documentary::Data,
    c: &ControlData,
    source: &structural::ConclusionSource,
    frame: Id<structural::StructuralFrame>,
    subject: Id<normalized::entities::EntityRef>,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<(AssertionKind, String), ModelError> {
    use structural::ConclusionSource::*;
    let _allowance = b.reserve("structural-control-render", 1024)?;
    let (kind, text) = match source {
        Forward { path } => {
            let path = need(&c.control_paths, *path)?;
            let traversal = need(&c.control_traversals, path.traversal)?;
            if traversal.frame != frame || traversal.seed != subject {
                return Err(invalid("control wording changes traversal frame/seed"));
            }
            let source = parameter_text(d, c, traversal.formal, context, b)?;
            let target = parameter_text(d, c, path.formal, context, b)?;
            let mut next = traversal.seed;
            let mut formal = traversal.formal;
            let mut conditional = false;
            let mut suppress = path.may_suppress;
            for ordinal in 0..path.length {
                let mut steps = c
                    .control_steps
                    .iter()
                    .filter(|s| s.path == path.id() && s.ordinal == ordinal);
                let step = steps
                    .next()
                    .ok_or_else(|| invalid("control wording path step absent"))?;
                if steps.next().is_some() {
                    return Err(invalid("control wording path step ambiguous"));
                }
                let flow = need(&c.argument_flows, step.flow)?;
                if flow.frame != frame
                    || flow.caller != next
                    || flow.source != formal
                    || need(&d.qualifications, flow.qualification)?.context != context
                {
                    return Err(invalid(
                        "control wording path changes exact parameter/context",
                    ));
                }
                next = flow.callee;
                formal = flow.formal;
                conditional |= flow.conditional;
                suppress |= flow.may_catch || flow.value_tested;
            }
            if next != path.target || formal != path.formal {
                return Err(invalid("control wording target differs from exact steps"));
            }
            (
                AssertionKind::Control,
                format!(
                    "Source parameter `{source}` is forwarded unchanged to `{target}` of callable {} along {} captured call step(s){}.{} Each step retains its original condition and call phase; this is a source observation.",
                    path.target.hex(),
                    path.length,
                    if conditional {
                        " on a conditional path"
                    } else {
                        ""
                    },
                    if suppress {
                        " The path may suppress or test the forwarded value."
                    } else {
                        ""
                    }
                ),
            )
        }
        Literal { argument } => {
            let argument = need(&c.literal_arguments, *argument)?;
            if argument.frame != frame || argument.caller != subject {
                return Err(invalid("literal wording changes frame/caller"));
            }
            let binding = need(&c.call_bindings, argument.binding)?;
            let slot = need(&d.option_slots, binding.slot)?;
            let parameter = need(&d.signature_parameters, slot.parameter)?;
            let signature = need(&d.parameter_signatures, parameter.signature)?;
            if need(&d.qualifications, signature.qualification)?.context != context {
                return Err(invalid("literal wording changes signature context"));
            }
            let name = need(&d.parameter_shapes, parameter.shape)?
                .name
                .as_ref()
                .map_or("unnamed slot", |n| n.as_str());
            let calls::BindingSource::Actual { occurrence } =
                need(&c.binding_sources, binding.source)?
            else {
                return Err(invalid("literal wording lacks exact actual argument"));
            };
            let expression = occurrence_text(d, *occurrence, b)?;
            let literal = need(&d.literals, argument.literal)?;
            let presentation =
                value::presentation::render(literal, value::presentation::Mode::Human, None, b)?
                    .map_err(|_| invalid("human literal presentation unavailable"))?;
            let literal = &presentation.text;
            (
                AssertionKind::TransformedControl,
                format!(
                    "The captured implementation supplies literal {literal}, from original argument `{expression}`, to declared parameter `{name}` of callable {}. This argument is an implementation value, not identity forwarding from a caller parameter.",
                    argument.callee.hex()
                ),
            )
        }
        Raise { observation } => {
            let observation = need(&c.conditional_raises, *observation)?;
            let path = need(&c.control_paths, observation.path)?;
            let traversal = need(&c.control_traversals, path.traversal)?;
            let entry = need(&c.entries, observation.entry)?;
            let leaf = need(&c.leaves, observation.leaf)?;
            let region = need(&c.regions, observation.region)?;
            if traversal.frame != frame
                || traversal.seed != subject
                || entry.context != context
                || entry.owner != path.target
                || entry.formal != path.formal
                || need(&d.qualifications, leaf.qualification)?.context != context
                || need(&d.qualifications, region.qualification)?.context != context
            {
                return Err(invalid("raise wording changes exact branch/entry frame"));
            }
            let source = parameter_text(d, c, traversal.formal, context, b)?;
            let parameter = parameter_text(d, c, entry.formal, context, b)?;
            let test = occurrence_text(d, leaf.test, b)?;
            let statement = need(&d.occurrences, region.statement)?;
            if statement.syntax_kind != source::SyntaxKind::StmtRaise {
                return Err(invalid("control restriction is not an original raise"));
            }
            let raised = occurrence_text(d, region.statement, b)?;
            (
                AssertionKind::Restriction,
                format!(
                    "Captured source forwards `{source}` to entry parameter `{parameter}` of callable {} and contains `{raised}` when test `{test}` is {}. The original branch condition and entry-read witness are retained; this does not assert a caller execution or normal completion.",
                    path.target.hex(),
                    if observation.positive {
                        "true"
                    } else {
                        "false"
                    }
                ),
            )
        }
        Unfollowed { observation } => {
            let observation = need(&c.unfollowed_paths, *observation)?;
            let path = need(&c.control_paths, observation.path)?;
            let traversal = need(&c.control_traversals, path.traversal)?;
            let argument = need(&c.unfollowed_arguments, observation.argument)?;
            if traversal.frame != frame || traversal.seed != subject || argument.frame != frame {
                return Err(invalid("unfollowed wording changes source frame"));
            }
            let source = parameter_text(d, c, traversal.formal, context, b)?;
            let target = parameter_text(d, c, argument.formal, context, b)?;
            let expression = occurrence_text(d, argument.argument, b)?;
            (
                AssertionKind::UnfollowedControl,
                format!(
                    "The source path for `{source}` stops at original argument `{expression}` for parameter `{target}` (boundary {:?}). The argument is not followed as parameter identity beyond this boundary.",
                    argument.reason
                ),
            )
        }
        _ => {
            let (kind, text) = basic_structural_text(source);
            (kind, text.to_owned())
        }
    };
    Ok((kind, text))
}
fn basic_structural_text(source: &structural::ConclusionSource) -> (AssertionKind, &'static str) {
    use structural::ConclusionSource::*;
    match source {
        Public { .. } => (
            AssertionKind::PublicAccess,
            "This public candidate preserves its exact access path and source resolution evidence.",
        ),
        Path { .. } => (
            AssertionKind::Coordinates,
            "The captured source graph contains a delegation path; each step retains its own condition and call phase.",
        ),
        Unresolved { .. } => (
            AssertionKind::AnalysisBoundary,
            "A source call remains unresolved; its candidate evidence and resolution boundary are retained.",
        ),
        Stop { .. } | ControlStop { .. } => (
            AssertionKind::AnalysisBoundary,
            "This bounded source traversal stopped with its explicit depth or enumeration boundary.",
        ),
        Usage { .. } => (
            AssertionKind::StaticUsageObservation,
            "Observed calls in captured sources retain their official usage sites, counts and uncertainty.",
        ),
        Handoff { .. } => (
            AssertionKind::Handoff,
            "Captured source contains a qualified producer-to-consumer handoff with original call and binding evidence.",
        ),
        Forward { .. } => (
            AssertionKind::Control,
            "A source parameter is forwarded along a qualified argument-flow path; each step retains its condition and phase.",
        ),
        Literal { .. } => (
            AssertionKind::TransformedControl,
            "The implementation supplies a source literal or expression argument with its original provenance.",
        ),
        Raise { .. } => (
            AssertionKind::Restriction,
            "The captured implementation contains a conditional raise observation with branch and entry-read evidence.",
        ),
        Unfollowed { .. } => (
            AssertionKind::UnfollowedControl,
            "The analysis does not follow this source argument beyond the recorded boundary.",
        ),
    }
}
fn analytic_text(source: &analytics::ConclusionSource) -> (AssertionKind, &'static str) {
    use analytics::ConclusionSource::*;
    match source {
        Rank { .. } | Community { .. } | Neighbour { .. } => (
            AssertionKind::Related,
            "Selected statistical analysis supplies a navigation association for this public candidate.",
        ),
        Concept { .. } => (
            AssertionKind::SharedSignature,
            "These members share attributes in the declared finite extracted context.",
        ),
        Implication { .. } => (
            AssertionKind::Implication,
            "The declared finite extracted attribute context satisfies this implication.",
        ),
        Document { .. } => (
            AssertionKind::DocLink,
            "A selected statistical neighbour links this public candidate to an original documentation passage.",
        ),
        Label { .. } => (
            AssertionKind::DocLink,
            "The selected community label retains its original passage and exact embedding-use evidence.",
        ),
    }
}
pub fn relations() -> Vec<Relation> {
    macro_rules! relations{($($field:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    output_rows!(relations)
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = documentary::Data::validation_inputs();
    inputs.extend(documentary::Output::validation_inputs());
    inputs.extend(Output::validation_inputs());
    inputs.extend(super::observations::Data::inputs());
    inputs.extend(ControlData::inputs());
    inputs.extend(super::summary::Data::inputs());
    inputs.extend(super::terminal::Data::inputs());
    inputs.extend(super::patterns::Data::inputs(stages::Profile::Behavioral));
    inputs.extend([
        ValidationInput::of::<super::frames::Frame>(&["id"]),
        ValidationInput::of::<structural::PublicCandidate>(&["id"]),
    ]);
    inputs.push(ValidationInput::of::<owner::Invocation>(&["id"]));
    inputs.sort_by_key(|i| i.name());
    inputs.dedup_by_key(|i| i.name());
    vec![Invariant {
        revision: 1,
        name: "programmatic_assertion_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: documentary::Data::new(b),
                observations: super::observations::Data::new(b),
                controls: ControlData::new(b),
                summary: super::summary::Data::new(b),
                terminal: super::terminal::Data::new(b),
                patterns: super::patterns::Data::new(b),
                frames: Rows::new(b),
                public: Rows::new(b),
                conclusions: documentary::Output::new(b),
                invocations: Rows::new(b),
                output: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: documentary::Data,
    observations: super::observations::Data,
    controls: ControlData,
    summary: super::summary::Data,
    terminal: super::terminal::Data,
    patterns: super::patterns::Data,
    frames: Rows<super::frames::Frame>,
    public: Rows<structural::PublicCandidate>,
    conclusions: documentary::Output,
    invocations: Rows<owner::Invocation>,
    output: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if n == owner::Invocation::NAME {
            self.invocations.decode(b)?;
            return Ok(());
        }
        if n == super::frames::Frame::NAME {
            self.frames.decode(b)?;
            return Ok(());
        }
        if n == structural::PublicCandidate::NAME {
            self.public.decode(b)?;
            return Ok(());
        }
        let observations = self.observations.visit(n, b)?;
        let controls = self.controls.visit(n, b)?;
        let summary = self.summary.visit(n, b)?;
        let terminal = self.terminal.visit(n, b)?;
        let patterns = self.patterns.visit(n, b)?;
        let a = self.data.visit(n, b)?;
        let c = self.conclusions.visit(n, b)?;
        let o = self.output.visit(n, b)?;
        if !a && !c && !o && !observations && !summary && !terminal && !patterns && !controls {
            return Err(invalid("undeclared assertion replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.conclusions
            .matches(&documentary::build(&self.data, &self.budget)?)?;
        self.output.matches(&build_all(
            &self.data,
            &self.conclusions,
            &self.observations,
            &self.controls,
            &self.summary,
            &self.terminal,
            &self.patterns,
            &self.public,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> { vec!["programmatic_assertion_replay"] }

#[cfg(test)]
mod tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn invocation(d: &documentary::Data) -> owner::Invocation {
        let core = d.core_invocations.iter().next().unwrap();
        owner::Invocation::new(
            core.input,
            core.context,
            super::super::build::definition().1.id(),
            None,
            [],
        )
        .0
    }
    fn replay(
        d: &documentary::Data,
        docs: &documentary::Output,
        invocations: &Rows<owner::Invocation>,
        out: &Output,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut check = (invariants().remove(0).create)(b);
        macro_rules! data{($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.$f.iter().cloned().collect::<Vec<_>>())?)?;)*};}
        crate::synthesis_documentary_inputs!(data);
        for relation in documentary::Output::validation_inputs() {
            let batch = match relation.name() {
                n if n == documentary::DocumentaryConclusion::NAME => {
                    documentary::DocumentaryConclusion::encode(
                        &docs.conclusions.iter().cloned().collect::<Vec<_>>(),
                    )?
                }
                n if n == documentary::DocumentaryBoundary::NAME => {
                    documentary::DocumentaryBoundary::encode(
                        &docs.boundaries.iter().cloned().collect::<Vec<_>>(),
                    )?
                }
                n if n == super::super::documentary_templates::ComponentBoundary::NAME => {
                    super::super::documentary_templates::ComponentBoundary::encode(
                        &docs
                            .component_boundaries
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>(),
                    )?
                }
                n if n == documentary::DocumentarySource::NAME => {
                    <documentary::DocumentarySource as Record>::encode(
                        &docs.sources.iter().cloned().collect::<Vec<_>>(),
                    )?
                }
                n if n == documentary::ProseSource::NAME => {
                    <documentary::ProseSource as Record>::encode(
                        &docs.prose_sources.iter().cloned().collect::<Vec<_>>(),
                    )?
                }
                n if n == documentary::ProseSlice::NAME => documentary::ProseSlice::encode(
                    &docs.slices.iter().cloned().collect::<Vec<_>>(),
                )?,
                _ => AssertionQualification::encode(
                    &docs.qualifications.iter().cloned().collect::<Vec<_>>(),
                )?,
            };
            check.visit(relation.name(), &batch)?;
        }
        check.visit(
            owner::Invocation::NAME,
            &owner::Invocation::encode(&invocations.iter().cloned().collect::<Vec<_>>())?,
        )?;
        macro_rules! output{($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$f.iter().cloned().collect::<Vec<_>>())?)?;)*};}
        output_rows!(output);
        check.finish()
    }
    #[test]
    fn canonical_extractive_assertion_keeps_candidate_qualification_and_original_warning() {
        let raw = "\"\"\"Run — carefully\n    across contexts. Then stop.\n\nWarning:\n    Authentication is required.\"\"\"";
        let (b, d, _) = documentary::tests::fixture(raw, &raw[3..raw.len() - 3]);
        let docs = documentary::build(&d, &b).unwrap();
        let inv = invocation(&d);
        let mut invocations = Rows::new(&b);
        invocations.insert(inv).unwrap();
        let out = build_documentary(&d, &docs, &invocations, &b).unwrap();
        assert_eq!(out.assertions.len(), 1);
        let row = out.assertions.iter().next().unwrap();
        assert_eq!(
            row.text(),
            "Authored prose for public candidate `pkg.api.run`: Run — carefully across contexts."
        );
        assert_eq!(
            (row.kind(), row.section(), row.status()),
            (
                AssertionKind::Outcome,
                BriefSection::Outcome,
                EvidenceStatus::Documented
            )
        );
        let proof = docs.conclusions.iter().next().unwrap();
        assert_eq!(row.qualification(), proof.qualification());
        let full =
            documentary::read_slice(&d, &docs, docs.slices.get(proof.prose).unwrap(), &b).unwrap();
        assert!(full.value.contains("Authentication is required"));
        replay(&d, &docs, &invocations, &out, &b).unwrap();
    }
    #[test]
    fn cross_input_authored_prose_is_rendered_through_release_association() {
        let (b, d, _, _) =
            documentary::tests::passage_fixture(true, "`pkg.api.run` starts a session.\n", (1, 12));
        let docs = documentary::build(&d, &b).unwrap();
        let mut invocations = Rows::new(&b);
        let inv = invocation(&d);
        invocations.insert(inv.clone()).unwrap();
        let out = build_documentary(&d, &docs, &invocations, &b).unwrap();
        assert!(
            out.assertions
                .iter()
                .any(|r| r.text().ends_with("`pkg.api.run` starts a session."))
        );
        for r in out.assertions.iter() {
            let q = docs.qualifications.get(r.qualification()).unwrap();
            assert_eq!(
                q.scope,
                (source::CoverageScope::Input { input: inv.input }).id()
            );
        }
        replay(&d, &docs, &invocations, &out, &b).unwrap();
    }
    #[test]
    fn canonical_definition_wrong_frame_and_unsupported_literal_refuse() {
        let (b, d, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let docs = documentary::build(&d, &b).unwrap();
        let proof = docs.conclusions.iter().next().unwrap();
        let mut inv = invocation(&d);
        inv.definition = id(80);
        assert!(authored_outcome(&d, &docs, &inv, proof, &b).is_err());
        let mut inv = invocation(&d);
        inv.context = id(81);
        assert!(authored_outcome(&d, &docs, &inv, proof, &b).is_err());
        let mut inv = invocation(&d);
        inv.input = id(82);
        assert!(authored_outcome(&d, &docs, &inv, proof, &b).is_err());
        let (b, d, _) = documentary::tests::fixture("f\"Interpolated {x}.\"", "Interpolated {x}.");
        let docs = documentary::build(&d, &b).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(invocation(&d)).unwrap();
        assert!(
            build_documentary(&d, &docs, &invocations, &b)
                .unwrap()
                .assertions
                .is_empty()
        );
        assert!(
            docs.boundaries
                .iter()
                .any(|r| r.reason
                    == documentary::DocumentaryBoundaryReason::UnsupportedLiteralMapping)
        );
    }
    #[test]
    fn shared_assertion_replay_refuses_forged_text_status_support_and_erasure() {
        let (b, d, _) = documentary::tests::fixture("\"Run.\"", "Run.");
        let docs = documentary::build(&d, &b).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(invocation(&d)).unwrap();
        for case in 0..4 {
            let mut out = build_documentary(&d, &docs, &invocations, &b).unwrap();
            match case {
                0 | 1 => {
                    let mut row = out.assertions.iter().next().unwrap().clone();
                    if case == 0 {
                        row.text = "Invented behavior".into();
                    } else {
                        row.status = EvidenceStatus::FixtureChecked;
                    }
                    out.assertions = Rows::new(&b);
                    out.assertions.insert(row).unwrap();
                }
                2 => {
                    let mut row = out.supports.iter().next().unwrap().clone();
                    row.source = id(84);
                    out.supports = Rows::new(&b);
                    out.supports.insert(row).unwrap();
                }
                _ => {
                    out = Output::new(&b);
                }
            }
            assert!(replay(&d, &docs, &invocations, &out, &b).is_err());
        }
    }
    #[test]
    fn static_usage_policy_preserves_stronger_usage_floor() {
        use EvidenceStatus as S;
        assert_eq!(
            analysis::policy::assertion_policy(
                AssertionKind::StaticUsageObservation,
                S::StructurallyObserved
            )
            .unwrap(),
            BriefSection::UsagePattern
        );
        for status in [
            S::Documented,
            S::FixtureChecked,
            S::StatisticallyDerived,
            S::Unresolved,
        ] {
            assert!(
                analysis::policy::assertion_policy(AssertionKind::StaticUsageObservation, status)
                    .is_err()
            );
        }
        assert!(
            analysis::policy::assertion_policy(
                AssertionKind::UsagePattern,
                S::StructurallyObserved
            )
            .is_err()
        );
        assert!(
            analysis::policy::assertion_policy(AssertionKind::UsagePattern, S::Documented).is_ok()
        );
        assert!(
            analysis::policy::assertion_policy(AssertionKind::UsagePattern, S::FixtureChecked)
                .is_ok()
        );
        assert!(
            analysis::policy::assertion_policy(AssertionKind::Outcome, S::StatisticallyDerived)
                .is_err()
        );
    }
    #[test]
    fn proof_backed_negative_assertion_retains_limits_and_structural_floor() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let template = AssertionTemplate::Summary { facet: id(61) };
        let source = AssertionSource::Summary { facet: id(61) };
        let facts = SourceFacts {
            qualification: id(62),
            status: EvidenceStatus::StructurallyObserved,
            heuristic: false,
        };
        let (negative, support) = emit(
            id(63),
            id(64),
            &template,
            AssertionContent {
                kind: AssertionKind::BehavioralRefutation,
                source: &source,
                facts,
                text: "RefutedUnderModel: exact admitted question is false.".into(),
            },
            &b,
        )
        .unwrap();
        assert_eq!(negative.kind(), AssertionKind::BehavioralRefutation);
        assert_eq!(negative.section(), BriefSection::Limits);
        assert_eq!(negative.status(), EvidenceStatus::StructurallyObserved);
        assert_eq!(negative.qualification(), id(62));
        assert_eq!(support.source, source.id());
        assert!(
            emit(
                id(63),
                id(64),
                &template,
                AssertionContent {
                    kind: AssertionKind::BehavioralRefutation,
                    source: &source,
                    facts: SourceFacts {
                        qualification: id(62),
                        status: EvidenceStatus::Documented,
                        heuristic: false
                    },
                    text: "RefutedUnderModel".into()
                },
                &b
            )
            .is_err()
        );
    }
    #[test]
    fn control_text_uses_exact_source_parameters_literals_and_conditioned_raises() {
        use crate::domain::{
            normalized::entities::ParameterEntity, source::*, structural::controls::*,
        };
        let original = "flag
True
not flag
raise ValueError('flag required')
";
        let (b, mut d, _) = documentary::tests::fixture(original, original);
        let artifact = d.artifacts.iter().next().unwrap().id();
        let q = d.qualifications.iter().next().unwrap().clone();
        let mut c = ControlData::new(&b);
        let mut occurrence = |start: i64, end: i64, kind| {
            d.occurrences
                .insert(Occurrence {
                    source: artifact,
                    start,
                    end,
                    syntax_kind: kind,
                    role: OccurrenceRole::Syntax,
                    structural_path: vec![start as i32],
                })
                .unwrap()
        };
        let parameter = occurrence(0, 4, SyntaxKind::Parameter);
        let argument = occurrence(5, 9, SyntaxKind::ExprBooleanLiteral);
        let test = occurrence(10, 18, SyntaxKind::ExprUnaryOp);
        let raised = occurrence(19, original.len() as i64 - 1, SyntaxKind::StmtRaise);
        let formal = c
            .parameter_entities
            .insert(ParameterEntity::Source {
                declaration: parameter,
            })
            .unwrap();
        let traversal = c
            .control_traversals
            .insert(ControlTraversal {
                frame: id(65),
                seed: id(66),
                formal,
                stop: None,
                vertices: 1,
                arcs: 0,
            })
            .unwrap();
        let path = c
            .control_paths
            .insert(ControlPath {
                traversal,
                target: id(66),
                formal,
                may_suppress: false,
                length: 0,
            })
            .unwrap();
        let (kind, text) = structural_text(
            &d,
            &c,
            &structural::ConclusionSource::Forward { path },
            id(65),
            id(66),
            q.context,
            &b,
        )
        .unwrap();
        assert_eq!(kind, AssertionKind::Control);
        assert!(text.contains("`flag`"));
        let slot = d
            .option_slots
            .insert(normalized::callables::SignatureSlot {
                parameter: id(68),
                variant: id(69),
                ordinal: 0,
                default: normalized::callables::DefaultSlot::Required,
            })
            .unwrap();
        let shape = d
            .parameter_shapes
            .insert(calls::ParameterShape {
                name: Some("flag".into()),
                kind: calls::ParameterKind::KeywordOnly,
                required: true,
            })
            .unwrap();
        let signature = d
            .parameter_signatures
            .insert(calls::Signature {
                role: crate::domain::calls::SignatureRole::Source,
                native: None,
                qualification: q.id(),
                scope: q.scope,
                symbol: id(70),
                variant: 0,
                form: calls::SignatureForm::List,
                parameters: ContentHash::of(b"flag"),
            })
            .unwrap();
        let p = d
            .signature_parameters
            .insert(calls::SignatureParameter {
                signature,
                ordinal: 0,
                shape,
            })
            .unwrap();
        let old = d.option_slots.get(slot).unwrap().clone();
        d.option_slots = Rows::new(&b);
        let slot = d
            .option_slots
            .insert(normalized::callables::SignatureSlot {
                parameter: p,
                ..old
            })
            .unwrap();
        let source = c
            .binding_sources
            .insert(calls::BindingSource::Actual {
                occurrence: argument,
            })
            .unwrap();
        let binding = c
            .call_bindings
            .insert(normalized::bindings::CallBinding {
                attempt: id(71),
                ordinal: 0,
                slot,
                source,
                kind: calls::BindingKind::Keyword,
                projection: calls::BindingProjection::Whole.id(),
            })
            .unwrap();
        let literal = d
            .literals
            .insert(value::Literal::Bool { value: true })
            .unwrap();
        let literal = c
            .literal_arguments
            .insert(LiteralArgument {
                frame: id(65),
                binding,
                observation: id(72),
                support: id(73),
                caller: id(66),
                callee: id(74),
                literal,
            })
            .unwrap();
        let (kind, text) = structural_text(
            &d,
            &c,
            &structural::ConclusionSource::Literal { argument: literal },
            id(65),
            id(66),
            q.context,
            &b,
        )
        .unwrap();
        assert_eq!(kind, AssertionKind::TransformedControl);
        assert!(text.contains("literal True"));
        assert!(text.contains("original argument `True`"));
        assert!(text.contains("parameter `flag`"));
        // Missing exact value premises cannot fall back to generic wording.
        c.literal_arguments = Rows::new(&b);
        assert!(
            structural_text(
                &d,
                &c,
                &structural::ConclusionSource::Literal { argument: literal },
                id(65),
                id(66),
                q.context,
                &b
            )
            .is_err()
        );
        let leaf = c
            .leaves
            .insert(flow::FlowTestLeafObservation {
                qualification: q.id(),
                test,
                atom: id(75),
                operand: Some(parameter),
            })
            .unwrap();
        let region = c
            .regions
            .insert(flow::FlowRegionObservation {
                qualification: q.id(),
                statement: raised,
                scope: id(76),
            })
            .unwrap();
        // EntryValueWitness has many mandatory typed premises; exercise the rendering branch with
        // the actual record layout rather than a test-only replacement validator.
        let entry = conditions::entry::EntryValueWitness {
            owner: id(66),
            formal,
            access: parameter,
            context: q.context,
            run: id(77),
            access_source: id(78),
            link: id(79),
            declaration_support: id(80),
            owner_support: id(81),
            use_observation: id(82),
            use_support: id(83),
            inventory: id(92),
            inventory_support: id(93),
            reaching: id(84),
            reaching_support: id(85),
            definition: id(86),
            definition_support: id(87),
            parameter_placement: None,
            parameter_placement_support: None,
            coverage: id(91),
        };
        let entry = c.entries.insert(entry).unwrap();
        let observation = c
            .conditional_raises
            .insert(ConditionalRaise {
                path,
                entry,
                leaf,
                leaf_support: id(88),
                region,
                support: id(89),
                positive: false,
            })
            .unwrap();
        let (kind, text) = structural_text(
            &d,
            &c,
            &structural::ConclusionSource::Raise { observation },
            id(65),
            id(66),
            q.context,
            &b,
        )
        .unwrap();
        assert_eq!(kind, AssertionKind::Restriction);
        assert!(text.contains("raise ValueError('flag required')"));
        assert!(text.contains("test `not flag` is false"));
        assert!(
            structural_text(
                &d,
                &c,
                &structural::ConclusionSource::Raise { observation },
                id(90),
                id(66),
                q.context,
                &b
            )
            .is_err()
        );
    }
}
