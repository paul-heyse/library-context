//! Four-family deterministic renderer and shared replay, over completed canonical C0/C1 evidence.
use super::*;
use crate::domain::{
    catalog::evidence::build::{EvidenceData, EvidenceOutput},
    normalized::Rows,
    resources::ResourceBudget,
    stages::*,
};
pub fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.required(id, || {
        invalid(format!("retrieval premise absent: {}", R::NAME))
    })
}
pub struct Data {
    pub source: EvidenceData,
    pub evidence: EvidenceOutput,
    pub facts: Facts,
    pub synthesis: SynthesisFacts,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            source: EvidenceData::new(b),
            evidence: EvidenceOutput::new(b),
            facts: Facts::new(b),
            synthesis: SynthesisFacts::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        let source = self.source.visit(n, b)?;
        let evidence = self.evidence.visit(n, b)?;
        let facts = self.facts.visit(n, b)?;
        let synthesis = self.synthesis.visit(n, b)?;
        Ok(source || evidence || facts || synthesis)
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut r = EvidenceData::inputs();
        r.extend(EvidenceOutput::inputs());
        r.extend(Facts::inputs());
        r.extend(SynthesisFacts::inputs());
        r.sort_by_key(|r| r.name());
        r.dedup_by_key(|r| r.name());
        r
    }
    pub fn selected(&self) -> Result<&Definition, ModelError> {
        if self.facts.definitions.len() != 1 {
            return Err(invalid("retrieval needs one completed authored definition"));
        }
        let r = self.facts.definitions.iter().next().unwrap();
        r.validate()?;
        Ok(r)
    }
}
macro_rules! facts {($($f:ident:$ty:ty,)*)=>{pub struct Facts {$(pub $f:Rows<$ty>,)*}impl Facts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}fn uses()->Vec<RelationUse> {vec![$(RelationUse::stored::<$ty>()),*]}}};}
crate::retrieval_inputs!(facts);
macro_rules! synthesis {($($f:ident:$ty:ty,)*)=>{pub struct SynthesisFacts {$(pub $f:Rows<$ty>,)*}impl SynthesisFacts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}}};}
crate::retrieval_synthesis_inputs!(synthesis);
macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{pub struct Output {$(pub $f:Rows<$ty>,)*}impl Output {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}pub fn matches(&self,o:&Self)->Result<(),ModelError> {$(if !self.$f.same(&o.$f) {return Err(invalid(format!("canonical retrieval closure differs: {}",<$ty>::NAME)));})*Ok(())}}};}
crate::retrieval_outputs!(outputs);
pub fn fragments(
    definition: &Definition,
    corpus: &CorpusText,
    out: &mut Rows<Fragment>,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    definition.validate()?;
    corpus.validate()?;
    let text = corpus.text.as_str();
    let mut start = 0;
    let mut ordinal = 0;
    while start < text.len() {
        let mut end = (start + definition.fragment_bytes as usize).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            return Err(invalid("retrieval window cannot fit Unicode scalar"));
        }
        let _copy = b.reserve(
            "retrieval-fragment-copy",
            (end - start) * 2 + size_of::<Fragment>(),
        )?;
        let part = &text[start..end];
        out.insert(Fragment {
            definition: definition.id(),
            fragment_bytes: definition.fragment_bytes,
            corpus: corpus.id(),
            ordinal,
            start: start as i64,
            end: end as i64,
            digest: ContentHash::of(part.as_bytes()),
            text: Utf8Text::from(part),
        })?;
        start = end;
        ordinal += 1;
    }
    Ok(())
}
struct Render {
    text: String,
    anchors: Vec<AnchorSource>,
    subjects: Vec<Subject>,
}
fn member_path(d: &Data, m: &catalog::CatalogMember) -> Result<String, ModelError> {
    let module = need(&d.source.core.modules, m.access)?;
    Ok(format!("{}.{}", module.qualified_name, m.path.join(".")))
}
fn api(
    d: &Data,
    m: &catalog::CatalogMember,
    context: Id<AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Render, ModelError> {
    let mut text = member_path(d, m)?;
    text.push('\n');
    for exposure in d
        .source
        .catalog
        .exposures
        .iter()
        .filter(|r| r.member == m.id())
    {
        let exposure = need(&d.source.core.exposures, exposure.exposure)?;
        if exposure.context == context {
            text.push_str(&format!(
                "Exposure status={:?} reason={:?}\n",
                exposure.status, exposure.reason
            ));
        }
    }

    let mut callables = d
        .source
        .catalog
        .callables
        .iter()
        .filter(|r| {
            r.member == m.id()
                && d.source
                    .core
                    .assessments
                    .get(r.assessment)
                    .is_some_and(|a| a.context == context)
        })
        .collect::<Vec<_>>();
    callables.sort_by_key(|r| r.id());
    for c in callables {
        let a = need(&d.source.core.assessments, c.assessment)?;
        text.push_str(&format!("Callable {:?}: identity={:?} signatures={:?} descriptor={:?}/{:?} body={:?} admitted={}\n",c.basis,a.identity,a.signatures,a.descriptor,a.descriptor_kind,a.body,a.body_admitted));
        let mut variants = d
            .source
            .catalog
            .invocations
            .iter()
            .filter(|r| r.callable == c.id())
            .collect::<Vec<_>>();
        variants.sort_by_key(|r| r.variant);
        for i in variants {
            let v = need(&d.source.core.variants, i.variant)?;
            let signature = need(&d.facts.signatures, v.signature)?;
            text.push_str(&format!(
                "Signature role={:?} form={:?} adjustment={:?}\n",
                v.role, signature.form, v.adjustment
            ));
            let mut slots = d
                .source
                .core
                .slots
                .iter()
                .filter(|r| r.variant == v.id())
                .collect::<Vec<_>>();
            slots.sort_by_key(|r| (r.ordinal, r.id()));
            for slot in slots {
                let parameter = need(&d.facts.signature_parameters, slot.parameter)?;
                let shape = need(&d.facts.shapes, parameter.shape)?;
                text.push_str(&format!(
                    "Parameter {:?} {:?} required={} default-slot={:?}\n",
                    shape.name, shape.kind, shape.required, slot.default
                ));
            }
        }
        for aspect in d
            .source
            .catalog
            .aspects
            .iter()
            .filter(|r| r.callable == c.id())
        {
            let a = need(&d.source.core.aspects, aspect.aspect)?;
            text.push_str(&format!(
                "Aspect {:?} admission={:?}\n",
                a.kind, a.admission
            ));
        }
    }
    let mut options = d
        .source
        .catalog
        .options
        .iter()
        .filter(|r| r.member == m.id())
        .collect::<Vec<_>>();
    options.sort_by_key(|r| r.id());
    for o in options {
        let subject = need(&d.source.catalog.subjects, o.subject)?;
        let default = need(&d.source.catalog.defaults, o.default)?;
        let label = match subject {
            catalog::CatalogOptionSubject::Parameter { slot } => {
                let slot = need(&d.source.core.slots, *slot)?;
                let p = need(&d.facts.signature_parameters, slot.parameter)?;
                let shape = need(&d.facts.shapes, p.shape)?;
                format!(
                    "effective parameter {} ({})",
                    shape.name.as_deref().unwrap_or("unnamed"),
                    shape.kind.label()
                )
            }
            catalog::CatalogOptionSubject::Field { field } => format!(
                "configuration field {}",
                need(&d.source.core.fields, *field)?.name
            ),
            catalog::CatalogOptionSubject::SourceParameter { .. } => {
                "original source parameter".into()
            }
        };
        let value = match default {
            catalog::CatalogDefault::Absent {} => "Absent".into(),
            catalog::CatalogDefault::Unknown {} => "Unknown".into(),
            catalog::CatalogDefault::Unavailable {} => "Unavailable".into(),
            catalog::CatalogDefault::Literal { literal } => {
                format!(
                    "Literal {}",
                    value::presentation::render(
                        need(&d.facts.literals, *literal)?,
                        value::presentation::Mode::Human,
                        None,
                        b
                    )?
                    .map_err(|_| invalid("human literal presentation unavailable"))?
                    .text
                )
            }
            catalog::CatalogDefault::Expression { .. } => "Unevaluated expression".into(),
            catalog::CatalogDefault::Factory { .. } => "Factory (not evaluated)".into(),
        };
        text.push_str(&format!("Option {label}: default={value}\n"));
    }
    for class in d
        .source
        .catalog
        .classes
        .iter()
        .filter(|r| r.member == m.id())
    {
        for c in d
            .source
            .catalog
            .constructors
            .iter()
            .filter(|r| r.class == class.id())
        {
            text.push_str(&format!(
                "Constructor {:?} {:?} applicability={:?} disposition={:?}\n",
                c.origin, c.kind, c.applicability, c.disposition
            ));
        }
    }
    let module = need(&d.source.core.modules, m.access)?;
    let original = c1::OriginalSource::Artifact {
        artifact: module.source,
    };
    need(&d.evidence.original_sources, original.id())?;
    Ok(Render {
        text,
        anchors: vec![AnchorSource::Original {
            source: original.id(),
        }],
        subjects: vec![Subject::Member { member: m.id() }],
    })
}
struct RenderedIdentity {
    family: Family,
    origin: Origin,
    title: String,
}

fn add(
    d: &Data,
    out: &mut Output,
    root: &c1::EvidenceRoot,
    rendered_identity: RenderedIdentity,
    render: Render,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let RenderedIdentity {
        family,
        origin,
        title,
    } = rendered_identity;
    for a in &render.anchors {
        let (artifact, _, _) = super::source::coordinates(d, a)?;
        if !matches!(origin, Origin::Brief { .. })
            && need(&d.source.core.artifacts, artifact)?.input != root.input
        {
            return Err(invalid("retrieval anchor crosses captured input"));
        }
    }
    let corpus = out.corpus.insert(CorpusText {
        family,
        rendering_version: d.selected()?.rendering_version,
        digest: ContentHash::of(render.text.as_bytes()),
        text: Utf8Text::from(render.text),
    })?;
    let origin = out.origins.insert(origin)?;
    let unit = out.units.insert(Unit {
        input: root.input,
        context: root.context,
        family,
        origin,
        corpus,
        title: Utf8Text::from(title),
    })?;
    out.roots.insert(UnitRoot {
        unit,
        root: root.id(),
    })?;
    for subject in render.subjects {
        let subject = out.subjects.insert(subject)?;
        out.unit_subjects.insert(UnitSubject { unit, subject })?;
    }
    for (ordinal, anchor) in render.anchors.into_iter().enumerate() {
        let original = out.anchor_sources.insert(anchor)?;
        out.anchors.insert(OriginalAnchor {
            unit,
            ordinal: ordinal as i64,
            original,
        })?;
    }
    // Each distinct corpus text is fragmented once; membership remains on all original units.
    if !out.fragments.iter().any(|f| f.corpus == corpus) {
        fragments(
            d.selected()?,
            need(&out.corpus, corpus)?,
            &mut out.fragments,
            b,
        )?;
    }
    Ok(())
}
pub fn build(d: &Data, b: &ResourceBudget) -> Result<Output, ModelError> {
    d.selected()?;
    let mut out = Output::new(b);
    // Bound render buffers before allocating. Input rows and retained output rows have separate charges.
    let mut payload = 0usize;
    let mut count = 0usize;
    macro_rules! measure {
        ($rows:expr) => {
            for row in $rows.iter() {
                payload = payload
                    .checked_add(row.heap_bytes())
                    .ok_or_else(|| invalid("retrieval render heap overflow"))?;
                count = count
                    .checked_add(1)
                    .ok_or_else(|| invalid("retrieval render count overflow"))?;
            }
        };
    }
    macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.core.$f);)*};}
    crate::catalog_inputs!(core);
    macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.catalog.$f);)*};}
    crate::catalog_outputs!(catalog);
    macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(measure!(d.source.facts.$f);)*};}
    crate::catalog_evidence_inputs!(facts);
    macro_rules! evidence {($($f:ident:$ty:ty,)*)=>{$(measure!(d.evidence.$f);)*};}
    crate::catalog_evidence_outputs!(evidence);
    macro_rules! extra {($($f:ident:$ty:ty,)*)=>{$(measure!(d.facts.$f);)*};}
    crate::retrieval_inputs!(extra);
    macro_rules! synthesis {($($f:ident:$ty:ty,)*)=>{$(measure!(d.synthesis.$f);)*};}
    crate::retrieval_synthesis_inputs!(synthesis);
    let bound = payload
        .checked_mul(16)
        .and_then(|n| count.checked_mul(4096).and_then(|c| n.checked_add(c)))
        .ok_or_else(|| invalid("retrieval render allocation overflow"))?;
    let _render = b.reserve("retrieval-render-buffers", bound)?;
    for root in d.evidence.roots.iter() {
        match need(&d.evidence.subjects, root.subject)? {
            c1::RootSubject::Member { member } => {
                let m = need(&d.source.catalog.members, *member)?;
                if m.input != root.input {
                    return Err(invalid("retrieval member root crosses input"));
                }
                let title = member_path(d, m)?;
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::ApiOptions,
                        origin: Origin::Api { member: *member },
                        title: title.clone(),
                    },
                    api(d, m, root.context, b)?,
                    b,
                )?;
                let module = need(&d.source.core.modules, m.access)?;
                let source = c1::OriginalSource::Artifact {
                    artifact: module.source,
                };
                let anchor = AnchorSource::Original {
                    source: source.id(),
                };
                let text = super::source::read(d, &anchor, b)?;
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::Source,
                        origin: Origin::Original {
                            source: source.id(),
                        },
                        title: need(&d.source.core.artifacts, module.source)?.path.clone(),
                    },
                    Render {
                        text: text.value.clone(),
                        anchors: vec![anchor],
                        subjects: vec![Subject::Member { member: *member }],
                    },
                    b,
                )?;
            }
            c1::RootSubject::Scenario { scenario } => {
                let r = need(&d.evidence.scenarios, *scenario)?;
                let mut text = format!(
                    "Scenario intent={:?} extraction={:?} parse={:?} binding={:?} environment={:?} execution={:?}\n",
                    r.intent, r.extraction, r.parse, r.binding, r.environment, r.execution
                );
                let mut anchors = vec![];
                let mut spans = d
                    .evidence
                    .spans
                    .iter()
                    .filter(|r| r.scenario == *scenario)
                    .collect::<Vec<_>>();
                spans.sort_by_key(|r| (r.ordinal, r.id()));
                for span in spans {
                    let anchor = AnchorSource::Original {
                        source: span.source,
                    };
                    let original = super::source::read(d, &anchor, b)?;
                    text.push_str(&format!("Span {:?}\n{}\n", span.role, original.value));
                    anchors.push(anchor);
                }
                for dependency in d
                    .evidence
                    .dependencies
                    .iter()
                    .filter(|r| r.scenario == *scenario)
                {
                    text.push_str(&format!(
                        "Setup {:?} status={:?}\n",
                        need(&d.evidence.setup, dependency.dependency)?,
                        dependency.status
                    ));
                }
                let _diagnostic_notes = b.reserve(
                    "retrieval-diagnostic-relevance",
                    d.evidence.diagnostic_use_targets.len().saturating_mul(256),
                )?;
                let mut diagnostics = std::collections::BTreeSet::new();
                let mut notes = 0usize;
                for target in d.evidence.diagnostic_use_targets.iter() {
                    let association = need(&d.evidence.associations, target.association)?;
                    if association.scenario != *scenario {
                        continue;
                    }
                    let link = need(&d.evidence.diagnostic_use_links, target.link)?;
                    let assessment = need(&d.evidence.diagnostic_use_assessments, link.assessment)?;
                    if assessment.status != c1::DiagnosticUseStatus::UniqueUse
                        || assessment.remainder
                    {
                        continue;
                    }
                    if !diagnostics.insert((assessment.characterization, association.member)) {
                        continue;
                    }
                    if notes == 128 {
                        text.push_str("Additional diagnostic relevance notes omitted by finite rendering bound.\n");
                        break;
                    }
                    notes += 1;
                    let characterization = need(
                        &d.evidence.source_characterizations,
                        assessment.characterization,
                    )?;
                    let native = need(
                        &d.source.facts.characterization_native,
                        characterization.native,
                    )?;
                    let (channel,settings)=match native {
                        crate::domain::analysis::native::NativeAssertionPremise::RuffDiagnosticObservation{assertion,..}=>{let row=need(&d.source.facts.ruff_diagnostics,*assertion)?;(row.channel,row.settings)},
                        crate::domain::analysis::native::NativeAssertionPremise::PyreflyDiagnosticObservation{assertion,support}=>{let row=need(&d.source.facts.pyrefly_diagnostics,*assertion)?;let support=need(&d.source.facts.pyrefly_diagnostic_supports,*support)?;(row.channel,need(&d.source.facts.runs,support.run)?.configuration)},
                        _=>return Err(invalid("diagnostic relevance has a non-diagnostic native premise")),
                    };
                    text.push_str(&format!("Diagnostic source relevance: exact use; target={:?}; channel={:?}; settings={:?}; diagnostic-only, no execution claim.\n",association.basis,channel,settings));
                }
                let mut subjects = vec![];
                for a in d.evidence.associations.iter().filter(|r| {
                    r.scenario == *scenario
                        && d.source
                            .core
                            .qualifications
                            .get(r.qualification)
                            .is_some_and(|q| q.context == root.context)
                }) {
                    text.push_str(&format!(
                        "Association basis={:?} phase={:?} intent={:?}\n",
                        a.basis, a.phase, a.intent
                    ));
                    subjects.push(Subject::Member { member: a.member });
                }
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::Scenario,
                        origin: Origin::Scenario {
                            scenario: *scenario,
                        },
                        title: "Usage scenario".into(),
                    },
                    Render {
                        text,
                        anchors,
                        subjects,
                    },
                    b,
                )?;
            }
            c1::RootSubject::Document { observation } => {
                let document = need(&d.source.facts.documents, *observation)?;
                let artifact = need(&d.source.core.artifacts, document.source)?;
                let mut any = false;
                for passage in d.source.facts.passages.iter().filter(|p| {
                    d.source
                        .core
                        .qualifications
                        .get(p.qualification)
                        .is_some_and(|q| q.context == root.context)
                }) {
                    let node = need(&d.source.facts.nodes, passage.passage.id())?;
                    let anchor = AnchorSource::Span { span: node.span() };
                    if super::source::coordinates(d, &anchor)?.0 != document.source {
                        continue;
                    }
                    any = true;
                    let text = super::source::read(d, &anchor, b)?;
                    let mut subjects = vec![];
                    for a in d.evidence.document_associations.iter() {
                        let candidate = need(&d.source.facts.mention_candidates, a.candidate)?;
                        let assessment =
                            need(&d.source.facts.mention_assessments, candidate.assessment)?;
                        let mention = need(&d.source.facts.mentions, assessment.observation)?;
                        if mention.passage == passage.passage
                            && d.source
                                .core
                                .qualifications
                                .get(mention.qualification)
                                .is_some_and(|q| q.context == root.context)
                        {
                            subjects.push(Subject::Member { member: a.member });
                        }
                    }
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::DocumentationDeployment,
                            origin: Origin::Passage {
                                observation: passage.id(),
                            },
                            title: passage
                                .heading
                                .clone()
                                .unwrap_or_else(|| artifact.path.clone()),
                        },
                        Render {
                            text: text.value.clone(),
                            anchors: vec![anchor],
                            subjects,
                        },
                        b,
                    )?;
                }
                if !any {
                    let anchor = AnchorSource::Artifact {
                        artifact: document.source,
                    };
                    let text = super::source::read(d, &anchor, b)?;
                    add(
                        d,
                        &mut out,
                        root,
                        RenderedIdentity {
                            family: Family::DocumentationDeployment,
                            origin: Origin::Document {
                                observation: *observation,
                            },
                            title: document
                                .title
                                .clone()
                                .unwrap_or_else(|| artifact.path.clone()),
                        },
                        Render {
                            text: text.value.clone(),
                            anchors: vec![anchor],
                            subjects: vec![],
                        },
                        b,
                    )?;
                }
            }
            c1::RootSubject::Deployment { deployment } => {
                let row = need(&d.evidence.deployments, *deployment)?;
                let observation = need(&d.source.facts.deployment, row.observation)?;
                let anchor = AnchorSource::Span {
                    span: observation.span,
                };
                let text = super::source::read(d, &anchor, b)?;
                let subjects = d
                    .evidence
                    .release_deployments
                    .iter()
                    .filter(|r| r.deployment == *deployment)
                    .map(|r| Subject::Release { release: r.release })
                    .collect();
                add(
                    d,
                    &mut out,
                    root,
                    RenderedIdentity {
                        family: Family::DocumentationDeployment,
                        origin: Origin::Deployment {
                            deployment: *deployment,
                        },
                        title: observation.field.clone(),
                    },
                    Render {
                        text: format!(
                            "Declared {} {:?} interpretation={:?}\n{}",
                            observation.field,
                            observation.name,
                            observation.interpretation,
                            text.value
                        ),
                        anchors: vec![anchor],
                        subjects,
                    },
                    b,
                )?;
            }
            c1::RootSubject::Option { .. } | c1::RootSubject::Release { .. } => {}
        }
    }
    briefs(d, &mut out, b)?;
    Ok(out)
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend(Output::inputs());
    vec![Invariant {
        revision: 1,
        name: "retrieval_canonical_rendering",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                out: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    out: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.data.visit(n, b)? && !self.out.visit(n, b)? {
            return Err(invalid("undeclared retrieval rendering input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.out.matches(&build(&self.data, &self.budget)?)
    }
}
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let (parameters, _) = catalog::build::definition();
    let mut version = KeySink::new("retrieval-final-owner/v2");
    parameters.id().encode(&mut version);
    for bytes in [
        include_bytes!("build.rs").as_slice(),
        include_bytes!("source.rs").as_slice(),
        include_bytes!("inventory.rs").as_slice(),
        include_bytes!("consumption.rs").as_slice(),
    ] {
        ContentHash::of(bytes).encode(&mut version);
    }
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::Retrieval,
        interpretation: analysis::Interpretation::Heuristic,
        parameters: parameters.id(),
        semantic_version: version.finish(),
    };
    (parameters, definition)
}

/// Optional briefs enrich ApiOptions without replacing any mandatory family or original source.
fn briefs(d: &Data, out: &mut Output, b: &ResourceBudget) -> Result<(), ModelError> {
    for brief in d.synthesis.briefs.iter() {
        let seed = need(&d.synthesis.seeds, brief.seed)?;
        let plan = need(&d.synthesis.seed_plans, seed.plan)?;
        let invocation = need(&d.synthesis.synthesis_invocations, plan.invocation)?;
        if invocation.definition != crate::domain::synthesis::build::definition().1.id() {
            return Err(invalid("retrieval brief has foreign S0 owner"));
        }
        let member = need(&d.synthesis.member_frames, seed.member)?;
        let core = need(&d.source.facts.core_invocations, member.invocation)?;
        if core.input != invocation.input || core.context != invocation.context {
            return Err(invalid("retrieval brief crosses its public member frame"));
        }
        let subject = c1::RootSubject::Member {
            member: member.member,
        };
        let root = d
            .evidence
            .roots
            .iter()
            .find(|r| {
                r.input == invocation.input
                    && r.context == invocation.context
                    && r.subject == subject.id()
            })
            .ok_or_else(|| invalid("retrieval brief has no exact C1 member root"))?;
        let mut rows = d
            .synthesis
            .brief_documents
            .iter()
            .filter(|r| r.brief == brief.id())
            .collect::<Vec<_>>();
        rows.sort_by_key(|r| r.ordinal);
        let _copy = b.reserve(
            "retrieval-brief-copy",
            usize::try_from(brief.bytes)
                .map_err(ModelError::codec)?
                .checked_mul(2)
                .and_then(|n| n.checked_add(rows.len() * size_of::<AnchorSource>()))
                .ok_or_else(|| invalid("retrieval brief allocation overflow"))?,
        )?;
        let mut text = String::new();
        for (ordinal, row) in rows.into_iter().enumerate() {
            if row.ordinal != ordinal as i64 {
                return Err(invalid("retrieval brief part membership changed"));
            }
            row.validate()?;
            text.push_str(row.text.as_str());
        }
        if text.len() as i64 != brief.bytes || ContentHash::of(text.as_bytes()) != brief.rendered {
            return Err(invalid("retrieval brief differs from canonical S0 bytes"));
        }
        let mut anchors = vec![];
        for source in d
            .synthesis
            .brief_sources
            .iter()
            .filter(|r| r.brief == brief.id())
        {
            let proof = need(&d.synthesis.documentary, source.documentary)?;
            need(&d.synthesis.prose_slices, proof.prose)?;
            anchors.push(AnchorSource::Prose { slice: proof.prose });
        }
        if anchors.is_empty() {
            return Err(invalid("retrieval brief lacks authored original anchors"));
        }
        anchors.sort_by_key(Record::id);
        anchors.dedup();
        add(
            d,
            out,
            root,
            RenderedIdentity {
                family: Family::ApiOptions,
                origin: Origin::Brief { brief: brief.id() },
                title: brief.title.as_str().into(),
            },
            Render {
                text,
                anchors,
                subjects: vec![Subject::Member {
                    member: member.member,
                }],
            },
            b,
        )?;
    }
    Ok(())
}

/// Mandatory helper closure, reused within the single final E0 producer stage.
pub fn mandatory_inputs(
    profile: Profile,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Vec<RelationUse>, ModelError> {
    let parent = c1::build::stage(profile, model, order)?;
    let mut inputs = parent.inputs;
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_store()));
    inputs.extend(Facts::uses());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    Ok(inputs)
}

impl Data {
    /// One actual S0 and C1 parent for every native frame, independently of observed retrieval rows.
    pub fn parents(
        &self,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<[analysis::retrieval::InvocationSource; 2], ModelError> {
        let (params, definition) = crate::domain::synthesis::build::definition();
        if self.synthesis.analysis_definitions.get(definition.id()) != Some(&definition)
            || self.synthesis.parameters.get(params.id()) != Some(&params)
        {
            return Err(invalid("retrieval canonical S0 definition is not authored"));
        }
        let (params, definition) = self::definition();
        if self.synthesis.analysis_definitions.get(definition.id()) != Some(&definition)
            || self.synthesis.parameters.get(params.id()) != Some(&params)
        {
            return Err(invalid("retrieval canonical E0 definition is not authored"));
        }
        let mut synthesis = self.synthesis.synthesis_invocations.iter().filter(|i| {
            i.input == input
                && i.context == context
                && i.definition == crate::domain::synthesis::build::definition().1.id()
                && i.subject.is_none()
        });
        let synthesis = synthesis
            .next()
            .ok_or_else(|| invalid("retrieval completed S0 native frame absent"))?;
        if self
            .synthesis
            .synthesis_invocations
            .iter()
            .filter(|i| {
                i.input == input
                    && i.context == context
                    && i.definition == synthesis.definition
                    && i.subject.is_none()
            })
            .count()
            != 1
        {
            return Err(invalid("retrieval S0 native frame ambiguous"));
        }
        let mut outcomes = self
            .synthesis
            .synthesis_outcomes
            .iter()
            .filter(|r| r.invocation == synthesis.id());
        let outcome = outcomes
            .next()
            .ok_or_else(|| invalid("retrieval S0 outcome absent"))?;
        if outcomes.next().is_some() {
            return Err(invalid("retrieval S0 outcome ambiguous"));
        }
        outcome.validate()?;
        let mut frames = self
            .synthesis
            .synthesis_frames
            .iter()
            .filter(|r| r.invocation == synthesis.id());
        let frame = frames
            .next()
            .ok_or_else(|| invalid("retrieval completed S0 frame closure absent"))?;
        if frames.next().is_some() {
            return Err(invalid("retrieval S0 frame ambiguous"));
        }
        let evidence = need(&self.facts.evidence_invocations, frame.evidence)?;
        if evidence.input != input
            || evidence.context != context
            || evidence.subject.is_some()
            || evidence.definition != c1::build::definition().1.id()
        {
            return Err(invalid("retrieval S0/C1 parent identity changed"));
        }
        let mut outcomes = self
            .synthesis
            .evidence_outcomes
            .iter()
            .filter(|r| r.invocation == evidence.id());
        let outcome = outcomes
            .next()
            .ok_or_else(|| invalid("retrieval C1 outcome absent"))?;
        if outcomes.next().is_some() {
            return Err(invalid("retrieval C1 outcome ambiguous"));
        }
        outcome.validate()?;
        Ok([
            analysis::retrieval::InvocationSource::Synthesis {
                invocation: synthesis.id(),
            },
            analysis::retrieval::InvocationSource::CatalogEvidence {
                invocation: evidence.id(),
            },
        ])
    }
}
/// Final single-writer E0 boundary. Completed E1 rows participate only in shared winner equality.
pub fn stage(
    profile: Profile,
    selected: &Definition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    selected.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    outputs.extend(
        analysis::retrieval::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    let own = outputs.iter().map(|r| r.name()).collect::<BTreeSet<_>>();
    let mut requested = super::consumption::ConsumptionData::inputs();
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Retrieval,
    ));
    requested.retain(|input| !own.contains(input.name()));
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        requested,
        &outputs,
        PublicationBoundary::Synthesis,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        name: "retrieval",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: if selected.embedding_requested {
            Effect::Embedding
        } else {
            Effect::Pure
        },
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: ContentHash::of(selected.id().bytes()),
    })
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["retrieval_canonical_rendering"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::synthesis::{assertions, briefs, documentary, seeds};
    #[test]
    fn optional_canonical_briefs_retain_full_warning_and_original_prose_without_replacing_mandatory_units()
     {
        let prose = "Run carefully.\n\nWarning:\nKeep authentication enabled.\n";
        let raw = format!("\"\"\"{prose}\"\"\"");
        let (b, doc, member) = documentary::tests::fixture(&raw, prose);
        let docs = documentary::build(&doc, &b).unwrap();
        let core = doc.core_invocations.iter().next().unwrap();
        let invocation = analysis::synthesis::Invocation::new(
            core.input,
            core.context,
            crate::domain::synthesis::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let mut invocations = Rows::new(&b);
        invocations.insert(invocation.clone()).unwrap();
        let settings = analysis::settings::AnalyticsConfiguration {
            module_prefixes: vec!["pkg.impl".into()],
            public_roots: vec!["pkg.api".into()],
            configured_seeds: vec!["pkg.api.run".into()],
            depth: 2,
            vertices: 128,
            arcs: 512,
            witnesses: 3,
            brief_budget: 1,
            communities: false,
            pagerank: false,
            fca: false,
            knn: false,
            rca: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        };
        let seeds = seeds::configured(
            &doc,
            &Rows::new(&b),
            &Rows::new(&b),
            &Rows::new(&b),
            &settings,
            &invocation,
            &b,
        )
        .unwrap();
        let assertions = assertions::build_documentary(&doc, &docs, &invocations, &b).unwrap();
        let briefs = briefs::build(&doc, &docs, &assertions, &seeds, &b).unwrap();
        assert_eq!(briefs.briefs.len(), 1);
        let mut d = Data::new(&b);
        d.facts
            .definitions
            .insert(Definition::builtin(false))
            .unwrap();
        for row in doc.members.iter() {
            d.source.catalog.members.insert(row.clone()).unwrap();
        }
        for row in doc.member_frames.iter() {
            d.synthesis.member_frames.insert(row.clone()).unwrap();
        }
        for row in doc.core_invocations.iter() {
            d.source.facts.core_invocations.insert(row.clone()).unwrap();
        }
        for row in doc.modules.iter() {
            d.source.core.modules.insert(row.clone()).unwrap();
        }
        for row in doc.artifacts.iter() {
            d.source.core.artifacts.insert(row.clone()).unwrap();
        }
        for row in doc.occurrences.iter() {
            d.source.core.occurrences.insert(row.clone()).unwrap();
        }
        for row in doc.chunks.iter() {
            d.facts.chunks.insert(row.clone()).unwrap();
        }
        for row in doc.canonical_evidence.iter() {
            d.source
                .facts
                .canonical_evidence
                .insert(row.clone())
                .unwrap();
        }
        let m = doc.member_frames.get(member).unwrap();
        let module = doc.modules.iter().next().unwrap();
        d.evidence
            .original_sources
            .insert(c1::OriginalSource::Artifact {
                artifact: module.source,
            })
            .unwrap();
        let subject = d
            .evidence
            .subjects
            .insert(c1::RootSubject::Member { member: m.member })
            .unwrap();
        d.evidence
            .roots
            .insert(c1::EvidenceRoot {
                input: core.input,
                context: core.context,
                subject,
            })
            .unwrap();
        d.synthesis.synthesis_invocations = invocations;
        d.synthesis.seed_plans = seeds.plans;
        d.synthesis.seeds = seeds.selected;
        d.synthesis.briefs = briefs.briefs;
        d.synthesis.brief_documents = briefs.documents;
        d.synthesis.brief_sources = briefs.sources;
        d.synthesis.documentary = docs.conclusions;
        d.synthesis.prose_slices = docs.slices;
        d.synthesis.prose_sources = docs.prose_sources;
        let out = build(&d, &b).unwrap();
        let unit = out
            .units
            .iter()
            .find(|r| matches!(out.origins.get(r.origin), Some(Origin::Brief { .. })))
            .unwrap();
        let text = need(&out.corpus, unit.corpus).unwrap().text.as_str();
        assert!(text.contains("Keep authentication enabled."));
        assert!(
            out.units
                .iter()
                .any(|r| matches!(out.origins.get(r.origin), Some(Origin::Api { .. })))
        );
        assert!(out.units.iter().any(|r| r.family == Family::Source));
        let anchor = out.anchors.iter().find(|r| r.unit == unit.id()).unwrap();
        let source = need(&out.anchor_sources, anchor.original).unwrap();
        assert!(matches!(source, AnchorSource::Prose { .. }));
        assert_eq!(
            super::super::source::read(&d, source, &b).unwrap().value,
            prose
        );
        let row = d.synthesis.brief_documents.iter().next().unwrap().clone();
        d.synthesis.brief_documents = Rows::new(&b);
        d.synthesis
            .brief_documents
            .insert(crate::domain::synthesis::briefs::BriefDocument {
                text: "Relocated or forged brief".into(),
                ..row
            })
            .unwrap();
        assert!(build(&d, &b).is_err());
    }
}
