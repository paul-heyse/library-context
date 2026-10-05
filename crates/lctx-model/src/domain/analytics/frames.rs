//! Exact activation and shared stored replay over independently retained structural frames.
use super::{
    build::{Data, METHODS, invalid, need},
    *,
};
use crate::domain::{
    analysis::{self, analytic as owner},
    normalized::Rows,
    resources::ResourceBudget,
    *,
};
pub struct Context {
    pub invocations: Rows<owner::Invocation>,
    pub sources: Rows<owner::InvocationSource>,
    pub inputs: Rows<owner::AnalysisInput>,
    pub outcomes: Rows<owner::AnalysisOutcome>,
}
impl Context {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            invocations: Rows::new(b),
            sources: Rows::new(b),
            inputs: Rows::new(b),
            outcomes: Rows::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        macro_rules! rows{($($f:ident:$t:ty,)*)=>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*};}
        rows! {invocations:owner::Invocation,sources:owner::InvocationSource,inputs:owner::AnalysisInput,outcomes:owner::AnalysisOutcome,}
        Ok(false)
    }
    pub fn validation_inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<owner::Invocation>(&["id"]),
            ValidationInput::of::<owner::InvocationSource>(&["id"]),
            ValidationInput::of::<owner::AnalysisInput>(&["id"]),
            ValidationInput::of::<owner::AnalysisOutcome>(&["id"]),
        ]
    }
}
pub fn parents(
    d: &Data,
    sf: &structural::StructuralFrame,
    method: analysis::AnalysisMethod,
) -> Result<Vec<owner::InvocationSource>, ModelError> {
    let mut rows = vec![
        owner::InvocationSource::Structural {
            invocation: sf.invocation,
        },
        owner::InvocationSource::Structural {
            invocation: sf.usage_invocation,
        },
    ];
    if method == analysis::AnalysisMethod::RelationalConcepts {
        rows.push(owner::InvocationSource::Structural {
            invocation: sf.handoff_invocation,
        });
    }
    let parent = need(&d.structural_invocations, sf.invocation)?;
    if build::selected(d.configuration()?, method)
        && matches!(
            method,
            analysis::AnalysisMethod::Neighbours | analysis::AnalysisMethod::Communities
        )
        && (method == analysis::AnalysisMethod::Neighbours || d.configuration()?.knn_layer)
    {
        let mut inv = d.embedding_invocations.iter().filter(|i| {
            i.input == parent.input && i.context == parent.context && i.subject.is_none()
        });
        let row = inv
            .next()
            .ok_or_else(|| invalid("analytic E1 invocation missing"))?;
        if inv.next().is_some() {
            return Err(invalid("analytic E1 invocation ambiguous"));
        }
        rows.push(owner::InvocationSource::AnalyticEmbedding {
            invocation: row.id(),
        });
    }
    Ok(rows)
}
pub fn verify(
    d: &Data,
    c: &Context,
    actual: &Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    verify_with_policy(d, c, actual, b, policy::RETAINED.attributes)
}
/// Replay an explicitly selected attribute policy against its corresponding definition identity.
pub fn verify_with_policy(
    d: &Data,
    c: &Context,
    actual: &Output,
    b: &ResourceBudget,
    attributes: policy::AttributePolicy,
) -> Result<(), ModelError> {
    if d.structural.frames.is_empty() && c.invocations.is_empty() && actual.frames.is_empty() {
        return actual.matches(&Output::new(b));
    }
    let s = d.configuration()?;
    let mut expected = Output::new(b);
    let mut sources = Rows::new(b);
    let mut inputs = Rows::new(b);
    let mut outcomes = Rows::new(b);
    let mut count = 0;
    for sf in d.structural.frames.iter() {
        let parent = need(&d.structural_invocations, sf.invocation)?;
        let frame = AnalyticFrame {
            structural: sf.id(),
            configuration: s.id(),
        };
        for method in METHODS {
            let def = build::definition_with_attribute_policy(s, method, attributes)?.1;
            let mut invs = c.invocations.iter().filter(|i| {
                i.input == parent.input
                    && i.context == parent.context
                    && i.definition == def.id()
                    && i.subject.is_none()
            });
            let invocation = invs
                .next()
                .ok_or_else(|| invalid("analytic invocation domain incomplete"))?;
            if invs.next().is_some() {
                return Err(invalid("analytic duplicate invocation"));
            }
            for source in parents(d, sf, method)? {
                let id = sources.insert(source)?;
                inputs.insert(owner::AnalysisInput {
                    invocation: invocation.id(),
                    parent: id,
                })?;
            }
            count += 1;
        }
        let assessment = need(&d.graphs.assessments, sf.invocation_graph)?;
        let mut headers = d
            .graphs
            .snapshots
            .iter()
            .filter(|h| h.assessment == assessment.id());
        let header = headers
            .next()
            .ok_or_else(|| invalid("analytic graph snapshot missing"))?;
        if headers.next().is_some() {
            return Err(invalid("analytic graph snapshot ambiguous"));
        }
        let graph = projection::snapshot::hydrate(header, assessment, &d.graphs.chunks, b)?;
        expected.extend(build::produce_with_policy(
            d,
            &frame,
            &c.invocations,
            &graph,
            b,
            attributes,
        )?)?;
    }
    for r in expected.results.iter() {
        outcomes.insert(outcome(r))?;
    }
    if count != c.invocations.len()
        || !sources.same(&c.sources)
        || !inputs.same(&c.inputs)
        || !outcomes.same(&c.outcomes)
    {
        return Err(invalid(
            "analytic exact invocation/parent/outcome membership differs",
        ));
    }
    actual.matches(&expected)
}
pub fn outcome(r: &TechniqueResult) -> owner::AnalysisOutcome {
    owner::AnalysisOutcome {
        invocation: r.invocation,
        status: r.status,
        reason: match r.stop {
            Stop::NotRequested => Some(obligation::ObligationKind::NotRequested),
            Stop::IterationLimit | Stop::WorkLimit | Stop::EnumerationLimit => {
                Some(obligation::ObligationKind::BudgetReached)
            }
            Stop::VectorsUnavailable => {
                Some(obligation::ObligationKind::EmbeddingServiceUnavailable)
            }
            _ if r.input_partial => Some(obligation::ObligationKind::IncompleteDomain),
            _ => None,
        },
    }
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::validation_inputs();
    inputs.extend(Context::validation_inputs());
    inputs.extend(Output::validation_inputs());
    inputs.sort_by_key(|i| (i.name(), i.prefix()));
    inputs.dedup_by_key(|i| (i.name(), i.prefix()));
    vec![Invariant {
        revision: 1,
        name: "analytic_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                context: Context::new(b),
                output: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    context: Context,
    output: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(n, b)? || self.context.visit(n, b)? || self.output.visit(n, b)? {
            Ok(())
        } else {
            Err(invalid("undeclared analytic input"))
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        verify(&self.data, &self.context, &self.output, &self.budget)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["analytic_replay"]
}
