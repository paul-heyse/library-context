//! Canonical, unreviewed documents. Parts retain every source byte used by the renderer.
use super::{assertions, documentary, seeds};
use crate::domain::{
    analysis::{
        policy::{AssertionKind, BriefSection, EvidenceStatus},
        synthesis as owner,
    },
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainCode};
pub const RENDERING_VERSION: i64 = 3;
pub const PART_BYTES: usize = 8192;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ReviewStatus {
    Unreviewed = 0,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OmissionReason {
    NoDocumentedOutcome = 0,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="synthesis_briefs",invariant_refs=invariants_refs)]
pub struct Brief {
    #[model(key)]
    pub seed: Id<seeds::SelectedSeed>,
    #[model(key)]
    pub version: i64,
    #[model(key)]
    pub assertions: ContentHash,
    #[model(key)]
    pub originals: ContentHash,
    pub title: Utf8Text,
    pub rendered: ContentHash,
    pub bytes: i64,
    pub review: ReviewStatus,
    pub documentation_only: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "synthesis_brief_assertions")]
pub struct BriefAssertion {
    #[model(key)]
    pub brief: Id<Brief>,
    #[model(key)]
    pub ordinal: i64,
    pub assertion: Id<assertions::ProgrammaticAssertion>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "synthesis_brief_sources")]
pub struct BriefSource {
    #[model(key)]
    pub brief: Id<Brief>,
    #[model(key)]
    pub documentary: Id<documentary::DocumentaryConclusion>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_brief_summary_facets")]
pub struct BriefSummary {
    #[model(key)]
    pub brief: Id<Brief>,
    #[model(key)]
    pub facet: Id<super::summary::SummaryFacet>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_brief_code_boundaries")]
pub struct BriefCodeBoundary {
    #[model(key)]
    pub brief: Id<Brief>,
    #[model(key)]
    pub boundary: Id<super::patterns::CodeBoundary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="synthesis_brief_documents",validate=validate_part)]
pub struct BriefDocument {
    #[model(key)]
    pub brief: Id<Brief>,
    #[model(key)]
    pub ordinal: i64,
    pub text: Utf8Text,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_brief_omissions")]
pub struct BriefOmission {
    #[model(key)]
    pub seed: Id<seeds::SelectedSeed>,
    pub reason: OmissionReason,
}
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("brief input absent: {}", R::NAME)))
}
fn validate_part(row: &BriefDocument) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.text.is_empty() || row.text.len() > PART_BYTES {
        return Err(invalid("canonical brief part exceeds its UTF-8 byte bound"));
    }
    Ok(())
}
macro_rules! outputs{($m:ident)=>{$m!{briefs:Brief,assertions:BriefAssertion,sources:BriefSource,summary:BriefSummary,code_boundaries:BriefCodeBoundary,documents:BriefDocument,omissions:BriefOmission,}};}
macro_rules! output{($($f:ident:$ty:ty,)*)=>{pub struct Output{$(pub $f:Rows<$ty>,)*}impl Output{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}pub fn matches(&self,o:&Self)->Result<(),ModelError>{$(if !self.$f.same(&o.$f){return Err(invalid(concat!("canonical brief closure differs: ",stringify!($f))));})*Ok(())}}};}
outputs!(output);
fn section(section: BriefSection) -> &'static str {
    match section {
        BriefSection::Outcome => "Outcome",
        BriefSection::PublicAccess => "Public access",
        BriefSection::ApplicableCase => "Applicable case",
        BriefSection::Controls => "Controls",
        BriefSection::UsagePattern => "Usage pattern",
        BriefSection::Limits => "Limits",
        BriefSection::Evidence => "Evidence",
        BriefSection::Related => "Related",
    }
}
/// UTF-8 parts partition the complete rendered document; no assertion, warning or limit is truncated.
pub fn parts(
    text: &str,
    mut visit: impl FnMut(i64, &str) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let mut start = 0;
    let mut ordinal = 0;
    while start < text.len() {
        let mut end = (start + PART_BYTES).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            return Err(invalid("brief UTF-8 partition cannot progress"));
        }
        visit(ordinal, &text[start..end])?;
        ordinal += 1;
        start = end;
    }
    Ok(())
}
pub fn build(
    d: &documentary::Data,
    docs: &documentary::Output,
    assertions: &assertions::Output,
    seeds: &seeds::Output,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    build_with_summary(
        d,
        docs,
        assertions,
        seeds,
        &super::summary::Data::new(b),
        &Rows::new(b),
        &Rows::new(b),
        &Rows::new(b),
        &super::patterns::Output::new(b),
        b,
    )
}
/// Summary facet qualifications come from the completed Analytic vocabulary; documentary
/// qualifications remain the immutable Facts view used by authored-source checks.
#[allow(
    clippy::too_many_arguments,
    reason = "Public brief construction keeps independent documentary, assertion, pattern, summary and seed owners explicit."
)]
pub fn build_with_summary(
    d: &documentary::Data,
    docs: &documentary::Output,
    assertions: &assertions::Output,
    seeds: &seeds::Output,
    summary: &super::summary::Data,
    summary_qualifications: &Rows<assertion::AssertionQualification>,
    facets: &Rows<super::summary::SummaryFacet>,
    frames: &Rows<super::frames::Frame>,
    patterns: &super::patterns::Output,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    for seed in seeds.selected.iter() {
        let plan = need(&seeds.plans, seed.plan)?;
        let mut charge = charged::StateCharge::new(b, "canonical-brief-members");
        let mut rows = charged::ChargedSet::default();
        for assertion in assertions
            .assertions
            .iter()
            .filter(|r| r.member == seed.member && r.invocation == plan.invocation)
        {
            rows.insert(
                &mut charge,
                (assertion.section(), assertion.kind(), assertion.id()),
            )?;
        }
        if !rows.iter().any(|(_, _, id)| {
            let r = assertions.assertions.get(*id).unwrap();
            r.kind() == AssertionKind::Outcome && r.status() == EvidenceStatus::Documented
        }) {
            out.omissions.insert(BriefOmission {
                seed: seed.id(),
                reason: OmissionReason::NoDocumentedOutcome,
            })?;
            continue;
        }
        let mut originals = Rows::new(b);
        let mut allowance = 256usize;
        for (_, _, id) in rows.iter() {
            let row = need(&assertions.assertions, *id)?;
            allowance = allowance
                .checked_add(row.text().len() + 64)
                .ok_or_else(|| invalid("brief allowance overflow"))?;
            for support in assertions.supports.iter().filter(|s| s.assertion == *id) {
                let assertions::AssertionSource::Documentary { conclusion } =
                    need(&assertions.sources, support.source)?
                else {
                    continue;
                };
                let proof = need(&docs.conclusions, *conclusion)?;
                let source = need(&docs.sources, proof.source)?;
                if source.member() != seed.member {
                    return Err(invalid("brief source crosses public slot"));
                }
                originals.insert(proof.clone())?;
            }
        }
        for proof in originals.iter() {
            let prose = need(&docs.slices, proof.prose)?;
            let length = usize::try_from(prose.end - prose.start).map_err(ModelError::codec)?;
            allowance = allowance
                .checked_add(length + 128)
                .ok_or_else(|| invalid("brief original allowance overflow"))?;
        }
        let mut selected_facets = Rows::new(b);
        for facet in facets.iter().filter(|f| {
            frames
                .get(f.frame)
                .is_some_and(|frame| frame.invocation == plan.invocation)
                && (f.member == Some(seed.member) || f.member.is_none())
        }) {
            selected_facets.insert(facet.clone())?;
            allowance = allowance
                .checked_add(1024)
                .ok_or_else(|| invalid("brief facet allowance overflow"))?;
        }
        let mut code_boundaries = Rows::new(b);
        for boundary in patterns.boundaries.iter().filter(|r| {
            r.member == seed.member
                && frames
                    .get(r.frame)
                    .is_some_and(|f| f.invocation == plan.invocation)
        }) {
            code_boundaries.insert(boundary.clone())?;
            allowance = allowance
                .checked_add(256)
                .ok_or_else(|| invalid("brief code boundary allowance overflow"))?;
        }
        let (title, _label) = seeds::path(d, seed.member, b)?;
        allowance = allowance
            .checked_add(title.len())
            .ok_or_else(|| invalid("brief title allowance overflow"))?;
        let _render = b.reserve(
            "canonical-brief-rendering",
            allowance
                .checked_mul(2)
                .ok_or_else(|| invalid("brief rendering allowance overflow"))?,
        )?;
        let mut text = String::with_capacity(allowance);
        text.push_str(&title);
        text.push_str("\nReview: Unreviewed\n\n");
        let mut prior = None;
        let mut assertion_hash = KeySink::new("canonical-brief-assertions");
        for (_, _, id) in rows.iter() {
            let row = need(&assertions.assertions, *id)?;
            id.encode(&mut assertion_hash);
            if prior != Some(row.section()) {
                text.push_str(section(row.section()));
                text.push_str(":\n");
                prior = Some(row.section());
            }
            text.push_str(row.text());
            text.push('\n');
        }
        if !selected_facets.is_empty() {
            text.push_str("\nBehavioral question coverage:\n");
            for facet in selected_facets.iter() {
                facet.id().encode(&mut assertion_hash);
                text.push_str(&super::summary::text(
                    summary,
                    facet,
                    facet
                        .qualification
                        .map(|q| need(summary_qualifications, q))
                        .transpose()?,
                    b,
                )?);
                text.push('\n');
            }
        }
        if !code_boundaries.is_empty() {
            text.push_str("\nAuthored usage limits:\n");
            for boundary in code_boundaries.iter() {
                boundary.id().encode(&mut assertion_hash);
                text.push_str(&format!(
                    "Whole-handoff source subset unavailable: {:?}; exact handoff {}.\n",
                    boundary.reason,
                    boundary.handoff.hex()
                ));
            }
        }
        text.push_str("\nEvidence — full authored prose, with original source anchors:\n");
        let mut original_hash = KeySink::new("canonical-brief-originals");
        for proof in originals.iter() {
            proof.id().encode(&mut original_hash);
            let prose = documentary::read_slice(d, docs, need(&docs.slices, proof.prose)?, b)?;
            ContentHash::of(prose.value.as_bytes()).encode(&mut original_hash);
            text.push_str("Authored source ");
            text.push_str(&proof.id().hex());
            text.push_str(":\n");
            text.push_str(&prose.value);
            if !prose.value.ends_with('\n') {
                text.push('\n');
            }
        }
        let brief = out.briefs.insert(Brief {
            seed: seed.id(),
            version: RENDERING_VERSION,
            assertions: assertion_hash.finish(),
            originals: original_hash.finish(),
            title: title.into(),
            rendered: ContentHash::of(text.as_bytes()),
            bytes: text.len() as i64,
            review: ReviewStatus::Unreviewed,
            documentation_only: !selected_facets.iter().any(|f| f.source.is_some()),
        })?;
        for (ordinal, (_, _, id)) in rows.iter().enumerate() {
            out.assertions.insert(BriefAssertion {
                brief,
                ordinal: ordinal as i64,
                assertion: *id,
            })?;
        }
        for boundary in code_boundaries.iter() {
            out.code_boundaries.insert(BriefCodeBoundary {
                brief,
                boundary: boundary.id(),
            })?;
        }
        for facet in selected_facets.iter() {
            out.summary.insert(BriefSummary {
                brief,
                facet: facet.id(),
            })?;
        }
        for proof in originals.iter() {
            out.sources.insert(BriefSource {
                brief,
                documentary: proof.id(),
            })?;
        }
        parts(&text, |ordinal, text| {
            out.documents.insert(BriefDocument {
                brief,
                ordinal,
                text: text.into(),
            })?;
            Ok(())
        })?;
    }
    Ok(out)
}
pub fn relations() -> Vec<Relation> {
    macro_rules! rows{($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    outputs!(rows)
}

pub fn invariants() -> Vec<Invariant> {
    let mut inputs = documentary::replay_inputs(
        documentary::Data::validation_inputs(),
        stages::PublicationBoundary::Facts,
    );
    inputs.extend(documentary::Output::validation_inputs());
    inputs.extend(assertions::Output::validation_inputs());
    inputs.extend(seeds::Output::validation_inputs());
    inputs.extend(Output::validation_inputs());
    inputs.extend(documentary::replay_inputs(
        super::observations::Data::inputs(),
        stages::PublicationBoundary::Analytic,
    ));
    inputs.extend(assertions::ControlData::inputs());
    inputs.extend(super::summary::Data::inputs());
    inputs.extend(documentary::replay_inputs(
        super::terminal::Data::inputs(),
        stages::PublicationBoundary::Analytic,
    ));
    inputs.extend(documentary::replay_inputs(
        super::patterns::Data::inputs(stages::Profile::Behavioral),
        stages::PublicationBoundary::Facts,
    ));
    inputs.extend([
        ValidationInput::of::<super::frames::Frame>(&["id"]),
        ValidationInput::of::<structural::PublicCandidate>(&["id"]),
    ]);
    inputs.push(ValidationInput::of::<owner::Invocation>(&["id"]));
    inputs.sort_by_key(|r| (r.name(), r.prefix()));
    inputs.dedup_by_key(|r| (r.name(), r.prefix()));
    vec![Invariant {
        revision: 3,
        name: "canonical_brief_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                budget: b.clone(),
                data: documentary::Data::new(b),
                observations: super::observations::Data::new(b),
                controls: assertions::ControlData::new(b),
                summary: super::summary::Data::new(b),
                terminal: super::terminal::Data::new(b),
                patterns: super::patterns::Data::new(b),
                frames: Rows::new(b),
                public: Rows::new(b),
                docs: documentary::Output::new(b),
                assertions: assertions::Output::new(b),
                seeds: seeds::Output::new(b),
                invocations: Rows::new(b),
                output: Output::new(b),
            })
        }),
    }]
}
struct Check {
    budget: ResourceBudget,
    data: documentary::Data,
    observations: super::observations::Data,
    controls: assertions::ControlData,
    summary: super::summary::Data,
    terminal: super::terminal::Data,
    patterns: super::patterns::Data,
    frames: Rows<super::frames::Frame>,
    public: Rows<structural::PublicCandidate>,
    docs: documentary::Output,
    assertions: assertions::Output,
    seeds: seeds::Output,
    invocations: Rows<owner::Invocation>,
    output: Output,
}
impl InvariantCheck for Check {
    fn visit(&mut self, _name: &str, _batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid(
            "S0 replay requires an explicit completed-input selector".into(),
        ))
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        documentary::replay_selector(input)?;
        let n = input.name();
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
        let observations = documentary::replay_visit(
            input,
            Some(stages::PublicationBoundary::Analytic),
            |name| self.observations.visit(name, b),
        )?;
        let controls =
            documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| {
                self.controls.visit(name, b)
            })?;
        let summary = documentary::replay_visit(input, None, |name| self.summary.visit(name, b))?;
        let terminal = documentary::replay_visit(
            input,
            Some(stages::PublicationBoundary::Analytic),
            |name| self.terminal.visit(name, b),
        )?;
        let patterns =
            documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| {
                self.patterns.visit(name, b)
            })?;
        let d =
            documentary::replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| {
                self.data.visit(name, b)
            })?;
        let doc = documentary::replay_visit(input, None, |name| self.docs.visit(name, b))?;
        let a = documentary::replay_visit(input, None, |name| self.assertions.visit(name, b))?;
        let s = documentary::replay_visit(input, None, |name| self.seeds.visit(name, b))?;
        let o = documentary::replay_visit(input, None, |name| self.output.visit(name, b))?;
        if !d
            && !doc
            && !a
            && !s
            && !o
            && !observations
            && !summary
            && !terminal
            && !patterns
            && !controls
        {
            return Err(invalid("undeclared brief replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.docs
            .matches(&documentary::build(&self.data, &self.budget)?)?;
        self.assertions.matches(&assertions::build_all(
            &self.data,
            &self.docs,
            &self.observations,
            &self.controls,
            &self.summary,
            &self.terminal,
            &self.patterns,
            &self.public,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?)?;
        let (facets, _) = super::summary::build(
            &self.summary,
            &self.data,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?;
        let patterns = super::patterns::build(
            &self.patterns,
            &self.data,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?;
        self.output.matches(&build_with_summary(
            &self.data,
            &self.docs,
            &self.assertions,
            &self.seeds,
            &self.summary,
            &self.observations.qualifications,
            &facets,
            &self.frames,
            &patterns,
            &self.budget,
        )?)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["canonical_brief_replay"]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs(
        raw: &str,
        interpreted: &str,
    ) -> (
        ResourceBudget,
        documentary::Data,
        documentary::Output,
        assertions::Output,
        seeds::Output,
        Rows<owner::Invocation>,
    ) {
        let (b, d, _) = documentary::tests::fixture(raw, interpreted);
        let docs = documentary::build(&d, &b).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(seeds::tests::invocation(&d)).unwrap();
        let assertions = assertions::build_documentary(&d, &docs, &invocations, &b).unwrap();
        let seed = seeds::tests::select(
            &d,
            &seeds::tests::settings(vec!["pkg.api.run".into()], 1),
            &b,
        );
        (b, d, docs, assertions, seed, invocations)
    }
    fn replay(
        d: &documentary::Data,
        docs: &documentary::Output,
        a: &assertions::Output,
        s: &seeds::Output,
        invocations: &Rows<owner::Invocation>,
        output: &Output,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut check = (invariants().remove(0).create)(b);
        macro_rules! data{($($f:ident:$ty:ty,)*)=>{$(check.visit_input(&documentary::replay_input::<$ty>(stages::PublicationBoundary::Facts),&<$ty as Record>::encode(&d.$f.iter().cloned().collect::<Vec<_>>())?)?;)*};}
        crate::synthesis_documentary_inputs!(data);
        macro_rules! row {
            ($ty:ty,$field:expr) => {
                check.visit_input(
                    &ValidationInput::of::<$ty>(&["id"]),
                    &<$ty as Record>::encode(&$field.iter().cloned().collect::<Vec<_>>())?,
                )?;
            };
        }
        row!(documentary::DocumentaryConclusion, docs.conclusions);
        row!(documentary::DocumentaryBoundary, docs.boundaries);
        row!(documentary::DocumentarySource, docs.sources);
        row!(documentary::ProseSource, docs.prose_sources);
        row!(documentary::ProseSlice, docs.slices);
        row!(assertion::AssertionQualification, docs.qualifications);
        row!(assertions::ProgrammaticAssertion, a.assertions);
        row!(assertions::AssertionTemplate, a.templates);
        row!(assertions::AssertionSource, a.sources);
        row!(assertions::ProgrammaticAssertionSupport, a.supports);
        row!(seeds::SeedPlan, s.plans);
        row!(seeds::ConfiguredSeedDecision, s.decisions);
        row!(seeds::ConfiguredSeedCandidate, s.candidates);
        row!(seeds::SelectedSeedSource, s.sources);
        row!(seeds::SelectedSeed, s.selected);
        row!(owner::Invocation, invocations);
        macro_rules! emit{($($f:ident:$ty:ty,)*)=>{$(row!($ty,output.$f);)*};}
        outputs!(emit);
        check.finish()
    }
    #[test]
    fn summary_facets_read_retained_qualifications_without_enlarging_documentary_facts() {
        // This renderer control assumes the Summary owner admitted the finite facet.
        let (b, d, docs, assertions, seeds, _) =
            inputs("\"\"\"Run carefully.\"\"\"", "Run carefully.");
        let seed = seeds.selected.iter().next().unwrap();
        let plan = seeds.plans.get(seed.plan).unwrap();
        let (_, earlier) = super::super::frames::tests::fixture();
        let parent = super::super::frames::parents(&earlier, &b)
            .unwrap()
            .remove(0);
        let frame = super::super::frames::frame(&parent, plan.invocation);
        let mut frames = Rows::new(&b);
        frames.insert(frame.clone()).unwrap();
        let mut retained = d.qualifications.iter().next().unwrap().clone();
        retained.modality = attribution::Modality::Candidate;
        retained.approximation = assertion::Approximation::Over;
        assert!(d.qualifications.get(retained.id()).is_none());
        let mut qualifications = Rows::new(&b);
        qualifications.insert(retained.clone()).unwrap();
        let mut facets = Rows::new(&b);
        facets
            .insert(super::super::summary::SummaryFacet {
                frame: frame.id(),
                conclusion: serde_json::from_value(serde_json::json!(vec![240u8; 16])).unwrap(),
                member: Some(seed.member),
                claim: None,
                qualification: Some(retained.id()),
                coverage: attribution::CoverageStatus::Partial,
                verdict: obligation::Verdict::Unknown,
                reason: None,
                source: None,
            })
            .unwrap();
        let summary = super::super::summary::Data::new(&b);
        let patterns = super::super::patterns::Output::new(&b);
        let output = build_with_summary(
            &d,
            &docs,
            &assertions,
            &seeds,
            &summary,
            &qualifications,
            &facets,
            &frames,
            &patterns,
            &b,
        )
        .unwrap();
        assert!(output.documents.iter().any(|part| {
            part.text
                .as_str()
                .contains("Behavioral analysis for this captured frame: Unknown; coverage Partial")
        }));
        assert!(
            build_with_summary(
                &d,
                &docs,
                &assertions,
                &seeds,
                &summary,
                &Rows::new(&b),
                &facets,
                &frames,
                &patterns,
                &b,
            )
            .is_err(),
            "missing retained qualification must be refused"
        );
        assert!(d.qualifications.get(retained.id()).is_none());
    }
    #[test]
    fn canonical_documents_are_unreviewed_and_retain_full_warnings_after_the_outcome() {
        let prose =
            "Run carefully. Then continue.\n\nWarning:\n    Authentication remains required.\n";
        let raw = format!("\"\"\"{prose}\"\"\"");
        let (b, d, docs, a, s, invocations) = inputs(&raw, prose);
        let out = build(&d, &docs, &a, &s, &b).unwrap();
        let brief = out.briefs.iter().next().unwrap();
        assert_eq!(brief.review, ReviewStatus::Unreviewed);
        assert!(brief.documentation_only);
        let body = out
            .documents
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>();
        assert!(body.contains(
            "Outcome:\nAuthored prose for public candidate `pkg.api.run`: Run carefully."
        ));
        assert!(body.contains("Authentication remains required."));
        assert_eq!(ContentHash::of(body.as_bytes()), brief.rendered);
        assert_eq!(body.len() as i64, brief.bytes);
        replay(&d, &docs, &a, &s, &invocations, &out, &b).unwrap();
    }
    #[test]
    fn no_admissible_outcome_is_an_explicit_omission_and_empty_budget_produces_no_briefs() {
        let (b, d, docs, a, s, invocations) = inputs("f\"Interpolated {x}.\"", "Interpolated {x}.");
        let out = build(&d, &docs, &a, &s, &b).unwrap();
        assert!(out.briefs.is_empty());
        assert_eq!(
            out.omissions.iter().next().unwrap().reason,
            OmissionReason::NoDocumentedOutcome
        );
        replay(&d, &docs, &a, &s, &invocations, &out, &b).unwrap();
        let (b, d, docs, a, _, _) = inputs("\"Run.\"", "Run.");
        let s = seeds::tests::select(&d, &seeds::tests::settings(vec![], 0), &b);
        let out = build(&d, &docs, &a, &s, &b).unwrap();
        assert!(out.briefs.is_empty());
        assert!(out.omissions.is_empty());
    }
    #[test]
    fn long_unicode_warning_is_partitioned_without_truncation() {
        let prose = format!("Run.\n\nWarning:\n{}\n", "é🙂慎重".repeat(1400));
        let raw = format!("\"\"\"{prose}\"\"\"");
        let (b, d, docs, a, s, invocations) = inputs(&raw, &prose);
        let out = build(&d, &docs, &a, &s, &b).unwrap();
        assert!(out.documents.len() > 1);
        let mut rows = out.documents.iter().collect::<Vec<_>>();
        rows.sort_by_key(|r| r.ordinal);
        let body = rows.iter().map(|r| r.text.as_str()).collect::<String>();
        assert!(body.contains(&prose));
        assert!(rows.iter().all(|r| r.text.len() <= PART_BYTES));
        let brief = out.briefs.iter().next().unwrap();
        assert_eq!(ContentHash::of(body.as_bytes()), brief.rendered);
        replay(&d, &docs, &a, &s, &invocations, &out, &b).unwrap();
    }
    #[test]
    fn shared_replay_rejects_relocated_sources_forged_document_text_and_coupled_erasure() {
        let (b, d, docs, a, s, invocations) = inputs("\"Run.\"", "Run.");
        for case in 0..3 {
            let mut out = build(&d, &docs, &a, &s, &b).unwrap();
            if case == 0 {
                let mut row = out.documents.iter().next().unwrap().clone();
                row.text = "This fake text is unsupported.".into();
                out.documents = Rows::new(&b);
                out.documents.insert(row).unwrap();
            } else if case == 1 {
                let mut row = out.briefs.iter().next().unwrap().clone();
                row.rendered = ContentHash::of(b"forged");
                out.briefs = Rows::new(&b);
                out.briefs.insert(row).unwrap();
            } else {
                out = Output::new(&b);
            }
            assert!(replay(&d, &docs, &a, &s, &invocations, &out, &b).is_err());
        }
    }
}
