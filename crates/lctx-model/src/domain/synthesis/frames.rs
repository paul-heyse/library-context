//! The independently known native frame universe fixes every S0 parent and output invocation.
use crate::Domain;
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, synthesis as owner},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="synthesis_frames",invariant_refs=invariants_refs)]
pub struct Frame {
    #[model(key)]
    pub invocation: Id<owner::Invocation>,
    pub configuration: Id<AnalyticsConfiguration>,
    pub core: Id<analysis::catalog_core::Invocation>,
    pub evidence: Id<analysis::catalog_evidence::Invocation>,
    pub selection: Id<analysis::selection::Invocation>,
    pub structural: Id<structural::StructuralFrame>,
    pub analytic: Id<analytics::AnalyticFrame>,
    pub summary: Id<analysis::summary::Invocation>,
}
#[macro_export]
macro_rules! synthesis_frame_inputs {($apply:ident)=>{$apply!{
 local:$crate::domain::analysis::local::Invocation,local_outcomes:$crate::domain::analysis::local::AnalysisOutcome,source:$crate::domain::analysis::source_call::Invocation,source_outcomes:$crate::domain::analysis::source_call::AnalysisOutcome,
 runs:$crate::domain::attribution::ProviderRun,definitions:$crate::domain::analysis::AnalysisDefinition,parameters:$crate::domain::analysis::MethodParameters,settings:$crate::domain::analysis::settings::AnalyticsConfiguration,
 core:$crate::domain::analysis::catalog_core::Invocation,core_outcomes:$crate::domain::analysis::catalog_core::AnalysisOutcome,
 evidence:$crate::domain::analysis::catalog_evidence::Invocation,evidence_outcomes:$crate::domain::analysis::catalog_evidence::AnalysisOutcome,evidence_sources:$crate::domain::analysis::catalog_evidence::InvocationSource,evidence_inputs:$crate::domain::analysis::catalog_evidence::AnalysisInput,
 selection:$crate::domain::analysis::selection::Invocation,selection_outcomes:$crate::domain::analysis::selection::AnalysisOutcome,selection_sources:$crate::domain::analysis::selection::InvocationSource,selection_inputs:$crate::domain::analysis::selection::AnalysisInput,
 structural:$crate::domain::structural::StructuralFrame,structural_invocations:$crate::domain::analysis::structural::Invocation,structural_outcomes:$crate::domain::analysis::structural::AnalysisOutcome,
 analytic_outcomes:$crate::domain::analysis::analytic::AnalysisOutcome,
 summary:$crate::domain::analysis::summary::Invocation,summary_outcomes:$crate::domain::analysis::summary::AnalysisOutcome,
}};}
/// The three analytic parent relations shared by frame verification and automatic selection.
pub struct AnalyticParents {
    pub frames: Rows<analytics::AnalyticFrame>,
    pub invocations: Rows<analysis::analytic::Invocation>,
    pub results: Rows<analytics::TechniqueResult>,
}
impl AnalyticParents {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            frames: Rows::new(b),
            invocations: Rows::new(b),
            results: Rows::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        if n == analytics::AnalyticFrame::NAME {
            self.frames.decode(b)?;
            return Ok(true);
        }
        if n == analysis::analytic::Invocation::NAME {
            self.invocations.decode(b)?;
            return Ok(true);
        }
        if n == analytics::TechniqueResult::NAME {
            self.results.decode(b)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<analytics::AnalyticFrame>(&["id"]),
            ValidationInput::of::<analysis::analytic::Invocation>(&["id"]),
            ValidationInput::of::<analytics::TechniqueResult>(&["id"]),
        ]
    }
}
macro_rules! data{($($f:ident:$ty:ty,)*)=>{pub struct Data{pub analytic_parents:AnalyticParents,$(pub $f:Rows<$ty>,)*}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{analytic_parents:AnalyticParents::new(b),$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{if self.analytic_parents.visit(n,b)? {return Ok(true);}$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{let mut inputs=vec![$(ValidationInput::of::<$ty>(&["id"]),)*];inputs.extend(AnalyticParents::inputs());inputs}}};}
crate::synthesis_frame_inputs!(data);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("synthesis completed parent absent"))
}
impl Data {
    pub fn lower(&self) -> catalog::evidence::frames::LowerFrames<'_> {
        catalog::evidence::frames::LowerFrames {
            local: &self.local,
            local_outcomes: &self.local_outcomes,
            source: &self.source,
            source_outcomes: &self.source_outcomes,
        }
    }
    pub fn configuration(&self) -> Result<&AnalyticsConfiguration, ModelError> {
        let mut rows = self.settings.iter();
        let row = rows
            .next()
            .ok_or_else(|| invalid("synthesis selected settings absent"))?;
        if rows.next().is_some() {
            return Err(invalid("synthesis selected settings ambiguous"));
        }
        row.validate()?;
        Ok(row)
    }
    fn summary_definition(&self) -> Result<Id<analysis::AnalysisDefinition>, ModelError> {
        let mut rows = self
            .definitions
            .iter()
            .filter(|r| r.method == analysis::AnalysisMethod::Summaries);
        let row = rows
            .next()
            .ok_or_else(|| invalid("selected authored Summaries definition absent"))?;
        if rows.next().is_some() {
            return Err(invalid("selected authored Summaries definition ambiguous"));
        }
        row.validate()?;
        need(&self.parameters, row.parameters)?.validate()?;
        Ok(row.id())
    }
    fn authored(
        &self,
        pair: (analysis::MethodParameters, analysis::AnalysisDefinition),
    ) -> Result<Id<analysis::AnalysisDefinition>, ModelError> {
        if self.definitions.get(pair.1.id()) != Some(&pair.1)
            || self.parameters.get(pair.0.id()) != Some(&pair.0)
        {
            return Err(invalid(
                "canonical synthesis parent definition is not authored",
            ));
        }
        Ok(pair.1.id())
    }
}
/// No optional parent's NotRequested result becomes a blanket status on another conclusion.
pub struct Parents {
    pub input: Id<input::InputRevision>,
    pub context: Id<attribution::AnalysisContext>,
    pub configuration: Id<AnalyticsConfiguration>,
    pub core: Id<analysis::catalog_core::Invocation>,
    pub evidence: Id<analysis::catalog_evidence::Invocation>,
    pub selection: Id<analysis::selection::Invocation>,
    pub structural: Id<structural::StructuralFrame>,
    pub analytic: Id<analytics::AnalyticFrame>,
    pub summary: Id<analysis::summary::Invocation>,
    pub sources: Vec<owner::InvocationSource>,
    _reservation: Box<dyn resources::Reservation>,
}
macro_rules! fixed {
    ($rows:expr,$outcomes:expr,$definition:expr,$input:expr,$context:expr) => {{
        let mut rows = $rows.iter().filter(|r| {
            r.input == $input
                && r.context == $context
                && r.definition == $definition
                && r.subject.is_none()
        });
        let row = rows
            .next()
            .ok_or_else(|| invalid("synthesis fixed native-frame parent absent"))?;
        if rows.next().is_some() {
            return Err(invalid("synthesis fixed native-frame parent ambiguous"));
        }
        let mut outcomes = $outcomes.iter().filter(|r| r.invocation == row.id());
        let outcome = outcomes
            .next()
            .ok_or_else(|| invalid("synthesis named completed parent has no outcome"))?;
        if outcomes.next().is_some() {
            return Err(invalid("synthesis named parent has duplicate outcomes"));
        }
        outcome.validate()?;
        row
    }};
}
pub fn parents(d: &Data, b: &ResourceBudget) -> Result<Vec<Parents>, ModelError> {
    let settings = d.configuration()?;
    catalog::evidence::frames::verify(
        &d.runs,
        &d.core,
        &d.evidence,
        &d.evidence_sources,
        &d.evidence_inputs,
        &d.lower(),
        b,
    )?;
    let earlier = catalog::evidence::frames::parents(&d.runs, &d.core, b)?;
    let c0 = d.authored(catalog::build::definition())?;
    let c1 = d.authored(catalog::evidence::build::definition())?;
    let c2 = d.authored(selection::build::definition())?;
    let summary = d.summary_definition()?;
    d.authored(super::build::definition())?;
    let mut out = Vec::new();
    for parent in earlier.iter() {
        let reservation = b.reserve(
            "synthesis-fixed-parents",
            2 * size_of::<Parents>() + 13 * size_of::<owner::InvocationSource>(),
        )?;
        out.reserve_exact(1);
        let input = parent.input;
        let context = parent.context;
        let core = fixed!(d.core, d.core_outcomes, c0, input, context);
        let evidence = fixed!(d.evidence, d.evidence_outcomes, c1, input, context);
        let selection = fixed!(d.selection, d.selection_outcomes, c2, input, context);
        let source = analysis::selection::InvocationSource::CatalogEvidence {
            invocation: evidence.id(),
        };
        if d.selection_sources.get(source.id()) != Some(&source)
            || !d
                .selection_inputs
                .iter()
                .any(|i| i.invocation == selection.id() && i.parent == source.id())
        {
            return Err(invalid("synthesis C2 parent changes its exact C1 input"));
        }
        let def = d.authored(structural::build::definition(
            settings,
            analysis::AnalysisMethod::Delegation,
        )?)?;
        let delegation = fixed!(
            d.structural_invocations,
            d.structural_outcomes,
            def,
            input,
            context
        );
        let mut frames = d
            .structural
            .iter()
            .filter(|r| r.invocation == delegation.id() && r.configuration == settings.id());
        let sf = frames
            .next()
            .ok_or_else(|| invalid("synthesis structural frame absent"))?;
        if frames.next().is_some() {
            return Err(invalid("synthesis structural frame ambiguous"));
        }
        let mut sources = Vec::with_capacity(13);
        sources.extend([
            owner::InvocationSource::CatalogCore {
                invocation: core.id(),
            },
            owner::InvocationSource::CatalogEvidence {
                invocation: evidence.id(),
            },
            owner::InvocationSource::Selection {
                invocation: selection.id(),
            },
        ]);
        for (method, actual) in [
            (analysis::AnalysisMethod::Delegation, sf.invocation),
            (analysis::AnalysisMethod::DirectUsage, sf.usage_invocation),
            (analysis::AnalysisMethod::Handoffs, sf.handoff_invocation),
            (analysis::AnalysisMethod::Controls, sf.control_invocation),
        ] {
            let def = d.authored(structural::build::definition(settings, method)?)?;
            let inv = fixed!(
                d.structural_invocations,
                d.structural_outcomes,
                def,
                input,
                context
            );
            if inv.id() != actual {
                return Err(invalid("synthesis structural parent changes method/frame"));
            }
            sources.push(owner::InvocationSource::Structural {
                invocation: inv.id(),
            });
        }
        let af = analytics::AnalyticFrame {
            structural: sf.id(),
            configuration: settings.id(),
        };
        if d.analytic_parents.frames.get(af.id()) != Some(&af) {
            return Err(invalid("synthesis exact analytic frame absent"));
        }
        for method in analytics::build::METHODS {
            let def = d.authored(analytics::build::definition(settings, method)?)?;
            let inv = fixed!(
                d.analytic_parents.invocations,
                d.analytic_outcomes,
                def,
                input,
                context
            );
            let mut results = d
                .analytic_parents
                .results
                .iter()
                .filter(|r| r.frame == af.id() && r.method == method);
            let result = results
                .next()
                .ok_or_else(|| invalid("synthesis analytic method result absent"))?;
            if results.next().is_some()
                || result.invocation != inv.id()
                || result.selected != analytics::build::selected(settings, method)
                || d.analytic_outcomes
                    .get(analytics::frames::outcome(result).id())
                    != Some(&analytics::frames::outcome(result))
            {
                return Err(invalid(
                    "synthesis analytic method activation/outcome differs",
                ));
            }
            sources.push(owner::InvocationSource::Analytic {
                invocation: inv.id(),
            });
        }
        let summary = fixed!(d.summary, d.summary_outcomes, summary, input, context);
        sources.push(owner::InvocationSource::Summary {
            invocation: summary.id(),
        });
        out.push(Parents {
            input,
            context,
            configuration: settings.id(),
            core: core.id(),
            evidence: evidence.id(),
            selection: selection.id(),
            structural: sf.id(),
            analytic: af.id(),
            summary: summary.id(),
            sources,
            _reservation: reservation,
        });
    }
    Ok(out)
}
pub fn frame(p: &Parents, invocation: Id<owner::Invocation>) -> Frame {
    Frame {
        invocation,
        configuration: p.configuration,
        core: p.core,
        evidence: p.evidence,
        selection: p.selection,
        structural: p.structural,
        analytic: p.analytic,
        summary: p.summary,
    }
}
pub fn verify(
    d: &Data,
    frames: &Rows<Frame>,
    invocations: &Rows<owner::Invocation>,
    sources: &Rows<owner::InvocationSource>,
    inputs: &Rows<owner::AnalysisInput>,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut expected_frames = Rows::new(b);
    let mut expected_sources = Rows::new(b);
    let mut expected_inputs = Rows::new(b);
    let parents = parents(d, b)?;
    for parent in &parents {
        let mut rows = invocations.iter().filter(|i| {
            i.input == parent.input
                && i.context == parent.context
                && i.definition == super::build::definition().1.id()
                && i.subject.is_none()
        });
        let inv = rows
            .next()
            .ok_or_else(|| invalid("synthesis invocation native universe incomplete"))?;
        if rows.next().is_some() {
            return Err(invalid("synthesis invocation native frame ambiguous"));
        }
        for source in &parent.sources {
            let id = expected_sources.insert(source.clone())?;
            expected_inputs.insert(owner::AnalysisInput {
                invocation: inv.id(),
                parent: id,
            })?;
        }
        expected_frames.insert(frame(parent, inv.id()))?;
    }
    if invocations.len() != parents.len()
        || !expected_frames.same(frames)
        || !expected_sources.same(sources)
        || !expected_inputs.same(inputs)
    {
        return Err(invalid(
            "synthesis exact native-frame/parent membership differs",
        ));
    }
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend([
        ValidationInput::of::<Frame>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<owner::InvocationSource>(&["id"]),
        ValidationInput::of::<owner::AnalysisInput>(&["id"]),
    ]);
    let invariants = vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "synthesis_fixed_native_frames",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                frames: Rows::new(b),
                invocations: Rows::new(b),
                sources: Rows::new(b),
                inputs: Rows::new(b),
                budget: b.clone(),
            })
        }),
    }];
    invariants
}
struct Check {
    data: Data,
    frames: Rows<Frame>,
    invocations: Rows<owner::Invocation>,
    sources: Rows<owner::InvocationSource>,
    inputs: Rows<owner::AnalysisInput>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(n, b)? {
            return Ok(());
        }
        macro_rules! row {
            ($ty:ty,$field:ident) => {
                if n == <$ty>::NAME {
                    self.$field.decode(b)?;
                    return Ok(());
                }
            };
        }
        row!(Frame, frames);
        row!(owner::Invocation, invocations);
        row!(owner::InvocationSource, sources);
        row!(owner::AnalysisInput, inputs);
        Err(invalid("undeclared synthesis frame input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        verify(
            &self.data,
            &self.frames,
            &self.invocations,
            &self.sources,
            &self.inputs,
            &self.budget,
        )
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    let mut refs = vec!["synthesis_fixed_native_frames"];
    refs.extend(super::observations::invariants_refs());
    refs
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    pub(crate) fn fixture() -> (ResourceBudget, Data) {
        let b = ResourceBudget::fixed(64 << 20).unwrap();
        let mut d = Data::new(&b);
        let input = id(1);
        let context = id(2);
        let settings = super::super::seeds::tests::settings(vec![], 0);
        d.settings.insert(settings.clone()).unwrap();
        d.runs
            .insert(attribution::ProviderRun {
                input,
                context,
                provider: id(3),
                configuration: ContentHash::of(b"earlier-native-configuration"),
                requested_families: ContentHash::of(b"earlier-native-request"),
            })
            .unwrap();
        fn authored(
            d: &mut Data,
            pair: (analysis::MethodParameters, analysis::AnalysisDefinition),
        ) -> Id<analysis::AnalysisDefinition> {
            d.parameters.insert(pair.0).unwrap();
            d.definitions.insert(pair.1.clone()).unwrap();
            pair.1.id()
        }
        let c0 = authored(&mut d, catalog::build::definition());
        let c1 = authored(&mut d, catalog::evidence::build::definition());
        let c2 = authored(&mut d, selection::build::definition());
        authored(&mut d, super::super::build::definition());
        let core = analysis::catalog_core::Invocation::new(input, context, c0, None, []).0;
        d.core.insert(core.clone()).unwrap();
        d.core_outcomes
            .insert(analysis::catalog_core::AnalysisOutcome {
                invocation: core.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .unwrap();
        let ld = authored(&mut d, local_semantics::definition());
        let local = analysis::local::Invocation::new(input, context, ld, None, []).0;
        d.local_outcomes
            .insert(analysis::local::AnalysisOutcome {
                invocation: local.id(),
                status: analysis::AnalysisStatus::NotRequested,
                reason: Some(obligation::ObligationKind::NotRequested),
            })
            .unwrap();
        d.local.insert(local).unwrap();
        let sd = authored(&mut d, execution::configuration::source_calls());
        let source = analysis::source_call::Invocation::new(input, context, sd, None, []).0;
        d.source_outcomes
            .insert(analysis::source_call::AnalysisOutcome {
                invocation: source.id(),
                status: analysis::AnalysisStatus::NotRequested,
                reason: Some(obligation::ObligationKind::NotRequested),
            })
            .unwrap();
        d.source.insert(source).unwrap();
        let sources = catalog::evidence::frames::sources(&core, &d.lower()).unwrap();
        let evidence = analysis::catalog_evidence::Invocation::new(
            input,
            context,
            c1,
            None,
            sources.iter().map(Record::id),
        )
        .0;
        for source in sources {
            d.evidence_sources.insert(source.clone()).unwrap();
            d.evidence_inputs
                .insert(analysis::catalog_evidence::AnalysisInput {
                    invocation: evidence.id(),
                    parent: source.id(),
                })
                .unwrap();
        }
        d.evidence_outcomes
            .insert(analysis::catalog_evidence::AnalysisOutcome {
                invocation: evidence.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .unwrap();
        d.evidence.insert(evidence.clone()).unwrap();
        let source = analysis::selection::InvocationSource::CatalogEvidence {
            invocation: evidence.id(),
        };
        let selection =
            analysis::selection::Invocation::new(input, context, c2, None, [source.id()]).0;
        d.selection_sources.insert(source.clone()).unwrap();
        d.selection_inputs
            .insert(analysis::selection::AnalysisInput {
                invocation: selection.id(),
                parent: source.id(),
            })
            .unwrap();
        d.selection_outcomes
            .insert(analysis::selection::AnalysisOutcome {
                invocation: selection.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .unwrap();
        d.selection.insert(selection).unwrap();
        let mut structural_ids = Vec::new();
        for method in [
            analysis::AnalysisMethod::Delegation,
            analysis::AnalysisMethod::DirectUsage,
            analysis::AnalysisMethod::Handoffs,
            analysis::AnalysisMethod::Controls,
        ] {
            let def = authored(
                &mut d,
                structural::build::definition(&settings, method).unwrap(),
            );
            let inv = analysis::structural::Invocation::new(input, context, def, None, []).0;
            structural_ids.push(inv.id());
            d.structural_outcomes
                .insert(analysis::structural::AnalysisOutcome {
                    invocation: inv.id(),
                    status: if method == analysis::AnalysisMethod::Controls {
                        analysis::AnalysisStatus::NotRequested
                    } else {
                        analysis::AnalysisStatus::Completed
                    },
                    reason: (method == analysis::AnalysisMethod::Controls)
                        .then_some(obligation::ObligationKind::NotRequested),
                })
                .unwrap();
            d.structural_invocations.insert(inv).unwrap();
        }
        let sf = structural::StructuralFrame {
            invocation: structural_ids[0],
            usage_invocation: structural_ids[1],
            handoff_invocation: structural_ids[2],
            control_invocation: structural_ids[3],
            controls_requested: false,
            configuration: settings.id(),
            invocation_graph: id(4),
            definition_graph: id(5),
        };
        d.structural.insert(sf.clone()).unwrap();
        let af = analytics::AnalyticFrame {
            structural: sf.id(),
            configuration: settings.id(),
        };
        d.analytic_parents.frames.insert(af.clone()).unwrap();
        for method in analytics::build::METHODS {
            let def = authored(
                &mut d,
                analytics::build::definition(&settings, method).unwrap(),
            );
            let inv = analysis::analytic::Invocation::new(input, context, def, None, []).0;
            let r = analytics::TechniqueResult {
                frame: af.id(),
                method,
                invocation: inv.id(),
                selected: false,
                status: analysis::AnalysisStatus::NotRequested,
                stop: analytics::Stop::NotRequested,
                iterations: 0,
                examined: 0,
                residual: None,
                input_partial: false,
            };
            d.analytic_outcomes
                .insert(analytics::frames::outcome(&r))
                .unwrap();
            d.analytic_parents.results.insert(r).unwrap();
            d.analytic_parents.invocations.insert(inv).unwrap();
        }
        // This pure parent-inventory test assumes the earlier Summary owner already validated its
        // canonical authored definition. It is not an execution or store qualification fixture.
        let parameters = super::super::build::definition().0;
        let def = analysis::AnalysisDefinition {
            method: analysis::AnalysisMethod::Summaries,
            parameters: parameters.id(),
            semantic_version: ContentHash::of(b"test-earlier-validated-summary"),
            interpretation: analysis::Interpretation::ExactUnderContext,
        };
        let def = authored(&mut d, (parameters, def));
        let inv = analysis::summary::Invocation::new(input, context, def, None, []).0;
        d.summary_outcomes
            .insert(analysis::summary::AnalysisOutcome {
                invocation: inv.id(),
                status: analysis::AnalysisStatus::NotRequested,
                reason: Some(obligation::ObligationKind::NotRequested),
            })
            .unwrap();
        d.summary.insert(inv).unwrap();
        (b, d)
    }
    fn publication(
        d: &Data,
        b: &ResourceBudget,
    ) -> (
        Rows<Frame>,
        Rows<owner::Invocation>,
        Rows<owner::InvocationSource>,
        Rows<owner::AnalysisInput>,
    ) {
        let mut f = Rows::new(b);
        let mut i = Rows::new(b);
        let mut s = Rows::new(b);
        let mut a = Rows::new(b);
        for p in parents(d, b).unwrap() {
            let mut source_ids = Vec::new();
            for source in &p.sources {
                source_ids.push(s.insert(source.clone()).unwrap());
            }
            let (inv, inputs) = owner::Invocation::new(
                p.input,
                p.context,
                super::super::build::definition().1.id(),
                None,
                source_ids,
            );
            f.insert(frame(&p, inv.id())).unwrap();
            i.insert(inv).unwrap();
            for row in inputs {
                a.insert(row).unwrap();
            }
        }
        (f, i, s, a)
    }
    #[test]
    fn exact_parent_inventory_retains_all_not_requested_optional_methods() {
        let (b, d) = fixture();
        let p = parents(&d, &b).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].sources.len(), 13);
        assert_eq!(
            d.analytic_parents
                .results
                .iter()
                .filter(|r| !r.selected)
                .count(),
            5
        );
        let (f, i, s, a) = publication(&d, &b);
        verify(&d, &f, &i, &s, &a, &b).unwrap();
    }
    #[test]
    fn coupled_invocation_frame_erasure_cannot_shrink_native_universe() {
        let (b, mut d) = fixture();
        assert!(
            verify(
                &d,
                &Rows::new(&b),
                &Rows::new(&b),
                &Rows::new(&b),
                &Rows::new(&b),
                &b
            )
            .is_err()
        );
        d.core = Rows::new(&b);
        d.evidence = Rows::new(&b);
        d.selection = Rows::new(&b);
        d.structural = Rows::new(&b);
        d.analytic_parents.frames = Rows::new(&b);
        d.summary = Rows::new(&b);
        assert!(parents(&d, &b).is_err());
    }
    #[test]
    fn missing_optional_outcome_wrong_selected_flag_and_wrong_parent_links_refuse() {
        for case in 0..3 {
            let (b, mut d) = fixture();
            match case {
                0 => d.summary_outcomes = Rows::new(&b),
                1 => {
                    let mut r = d.analytic_parents.results.iter().next().unwrap().clone();
                    r.selected = true;
                    let old = d
                        .analytic_parents
                        .results
                        .iter()
                        .filter(|v| v.id() != r.id())
                        .cloned()
                        .collect::<Vec<_>>();
                    d.analytic_parents.results = Rows::new(&b);
                    for row in old {
                        d.analytic_parents.results.insert(row).unwrap();
                    }
                    d.analytic_parents.results.insert(r).unwrap();
                }
                _ => d.selection_inputs = Rows::new(&b),
            }
            assert!(parents(&d, &b).is_err());
        }
    }
}
