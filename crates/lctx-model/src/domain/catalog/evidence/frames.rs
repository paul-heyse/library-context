//! Total invocation domain is derived from actual native runs, including inputs with no roots.
use super::{build::invalid, *};
use crate::domain::{
    attribution::ProviderRun,
    charged::{ChargedSet, StateCharge},
    normalized::Rows,
    resources::ResourceBudget,
};
use analysis::{catalog_core as core, catalog_evidence as evidence};
pub fn parents(
    runs: &Rows<ProviderRun>,
    parents: &Rows<core::Invocation>,
    budget: &ResourceBudget,
) -> Result<Rows<core::Invocation>, ModelError> {
    let mut charge = StateCharge::new(budget, "catalog-evidence-invocation-frames");
    let mut frames = ChargedSet::default();
    for run in runs.iter() {
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    let definition = catalog::build::definition().1.id();
    let mut output = Rows::new(budget);
    for (input, context) in frames.iter() {
        let mut candidates = parents.iter().filter(|r| {
            r.input == *input
                && r.context == *context
                && r.definition == definition
                && r.subject.is_none()
        });
        let parent = candidates.next().ok_or_else(|| {
            invalid("C1 requires the fixed completed C0 invocation for every native frame")
        })?;
        if candidates.next().is_some() {
            return Err(invalid("C1 native frame has duplicate C0 invocations"));
        }
        output.insert(parent.clone())?;
    }
    Ok(output)
}
pub struct LowerFrames<'a> {
    pub compact: &'a uniform_outcome::AnalysisPredecessors,
    pub local: &'a Rows<analysis::local::Invocation>,
    pub local_outcomes: &'a Rows<analysis::local::AnalysisOutcome>,
    pub source: &'a Rows<analysis::source_call::Invocation>,
    pub source_outcomes: &'a Rows<analysis::source_call::AnalysisOutcome>,
}
pub fn sources(
    parent: &core::Invocation,
    lower: &LowerFrames<'_>,
) -> Result<[evidence::InvocationSource; 3], ModelError> {
    macro_rules! fixed {
        ($rows:expr,$outcomes:expr,$definition:expr) => {{
            let mut rows = $rows.iter().filter(|r| {
                r.input == parent.input
                    && r.context == parent.context
                    && r.definition == $definition
                    && r.subject.is_none()
            });
            let row = rows
                .next()
                .ok_or_else(|| invalid("C1 named lower native frame absent"))?;
            if rows.next().is_some() {
                return Err(invalid("C1 named lower native frame ambiguous"));
            }
            let mut outcomes = $outcomes.iter().filter(|r| r.invocation == row.id());
            let outcome = outcomes
                .next()
                .ok_or_else(|| invalid("C1 named lower frame outcome absent"))?;
            if outcomes.next().is_some() {
                return Err(invalid("C1 named lower frame outcome ambiguous"));
            }
            outcome.validate()?;
            row
        }};
    }
    let local = if let Some(descriptor) = lower.compact.get(
        analysis::owner::AnalysisOwner::Local,
        parent.input,
        parent.context,
    )? {
        if lower
            .local
            .iter()
            .any(|row| (row.input, row.context) == (parent.input, parent.context))
        {
            return Err(invalid("C1 compact and ordinary Local overlap"));
        }
        evidence::InvocationSource::UniformAnalysis { descriptor }
    } else {
        evidence::InvocationSource::Local {
            invocation: fixed!(
                lower.local,
                lower.local_outcomes,
                crate::domain::local_semantics::definition().1.id()
            )
            .id(),
        }
    };
    let source = if let Some(descriptor) = lower.compact.get(
        analysis::owner::AnalysisOwner::SourceCalls,
        parent.input,
        parent.context,
    )? {
        if lower
            .source
            .iter()
            .any(|row| (row.input, row.context) == (parent.input, parent.context))
        {
            return Err(invalid("C1 compact and ordinary SourceCall overlap"));
        }
        evidence::InvocationSource::UniformAnalysis { descriptor }
    } else {
        evidence::InvocationSource::SourceCallAnalysis {
            invocation: fixed!(
                lower.source,
                lower.source_outcomes,
                crate::domain::execution::configuration::source_calls()
                    .1
                    .id()
            )
            .id(),
        }
    };
    Ok([
        evidence::InvocationSource::CatalogCore {
            invocation: parent.id(),
        },
        local,
        source,
    ])
}
pub fn verify(
    runs: &Rows<ProviderRun>,
    core: &Rows<core::Invocation>,
    invocations: &Rows<evidence::Invocation>,
    sources: &Rows<evidence::InvocationSource>,
    inputs: &Rows<evidence::AnalysisInput>,
    lower: &LowerFrames<'_>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let parents = parents(runs, core, budget)?;
    let definition = super::build::definition().1.id();
    let mut expected_sources = Rows::new(budget);
    let mut expected_inputs = Rows::new(budget);
    let mut count = 0;
    for parent in parents.iter() {
        let mut matches = invocations.iter().filter(|r| {
            r.input == parent.input
                && r.context == parent.context
                && r.definition == definition
                && r.subject.is_none()
        });
        let invocation = matches
            .next()
            .ok_or_else(|| invalid("catalog evidence invocation domain is incomplete"))?;
        if matches.next().is_some() {
            return Err(invalid("catalog evidence frame has duplicate invocations"));
        }
        count += 1;
        for source in self::sources(parent, lower)? {
            let source = expected_sources.insert(source)?;
            expected_inputs.insert(evidence::AnalysisInput {
                invocation: invocation.id(),
                parent: source,
            })?;
        }
    }
    if count != invocations.len()
        || !expected_sources.same(sources)
        || !expected_inputs.same(inputs)
    {
        return Err(invalid(
            "catalog evidence invocation frames or exact C0 parent membership differ",
        ));
    }
    Ok(())
}
pub(crate) fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 2,
        name: "catalog_evidence_invocation_domain",
        inputs: uniform_outcome::AnalysisUniformConsumer::inputs(
            analysis::owner::AnalysisOwner::SourceCalls,
            uniform_outcome::AnalysisUniformConsumer::inputs(
                analysis::owner::AnalysisOwner::Local,
                vec![
                    ValidationInput::of::<ProviderRun>(&["id"]),
                    ValidationInput::of::<core::Invocation>(&["id"]),
                    ValidationInput::of::<evidence::Invocation>(&["id"]),
                    ValidationInput::of::<evidence::InvocationSource>(&["id"]),
                    ValidationInput::of::<evidence::AnalysisInput>(&["id"]),
                    ValidationInput::of::<analysis::local::Invocation>(&["id"]),
                    ValidationInput::of::<analysis::local::AnalysisOutcome>(&["id"]),
                    ValidationInput::of::<analysis::source_call::Invocation>(&["id"]),
                    ValidationInput::of::<analysis::source_call::AnalysisOutcome>(&["id"]),
                ],
            ),
        ),
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                local_uniform: uniform_outcome::AnalysisUniformConsumer::new(
                    analysis::owner::AnalysisOwner::Local,
                    b,
                ),
                source_uniform: uniform_outcome::AnalysisUniformConsumer::new(
                    analysis::owner::AnalysisOwner::SourceCalls,
                    b,
                ),
                runs: Rows::new(b),
                core: Rows::new(b),
                invocations: Rows::new(b),
                sources: Rows::new(b),
                inputs: Rows::new(b),
                lower: super::runtime::RuntimeData::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    local_uniform: uniform_outcome::AnalysisUniformConsumer,
    source_uniform: uniform_outcome::AnalysisUniformConsumer,
    runs: Rows<ProviderRun>,
    core: Rows<core::Invocation>,
    invocations: Rows<evidence::Invocation>,
    sources: Rows<evidence::InvocationSource>,
    inputs: Rows<evidence::AnalysisInput>,
    lower: super::runtime::RuntimeData,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn admit_uniform_owner(
        &mut self,
        owner: &uniform_outcome::ValidatedUniformOwner,
    ) -> Result<(), ModelError> {
        self.local_uniform.admit(owner)?;
        self.source_uniform.admit(owner)
    }
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        self.local_uniform.visit(name, batch)?;
        self.source_uniform.visit(name, batch)?;
        if self.lower.visit(name, batch)? {
            return Ok(());
        }
        if name == ProviderRun::NAME {
            self.runs.decode(batch)?;
        } else if name == core::Invocation::NAME {
            self.core.decode(batch)?;
        } else if name == evidence::Invocation::NAME {
            self.invocations.decode(batch)?;
        } else if name == evidence::InvocationSource::NAME {
            self.sources.decode(batch)?;
        } else if name == evidence::AnalysisInput::NAME {
            self.inputs.decode(batch)?;
        } else if !uniform_outcome::AnalysisUniformInputs::inputs()
            .iter()
            .any(|input| input.name() == name)
        {
            return Err(invalid("undeclared evidence frame input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        for parent in parents(&self.runs, &self.core, &self.budget)?.iter() {
            for helper in [&self.local_uniform, &self.source_uniform] {
                if let Some(domain) = helper.frame(parent.input, parent.context)? {
                    self.lower.compact.include(domain)?;
                }
            }
        }
        verify(
            &self.runs,
            &self.core,
            &self.invocations,
            &self.sources,
            &self.inputs,
            &self.lower.lower(),
            &self.budget,
        )
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["catalog_evidence_invocation_domain"]
}

#[cfg(test)]
#[path = "frame_controls.rs"]
mod controls;
