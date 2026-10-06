//! The earlier native frame universe requires a C2 invocation even when it produced no domains.
use super::{
    build::{Data, Output, invalid, need},
    *,
};
use crate::domain::{normalized::Rows, resources::ResourceBudget};
use analysis::{catalog_evidence as c1, selection as owner};
/// Fixed frame metadata prepared once; it carries no rich declaration or evidence rows.
macro_rules! frame_rows {($apply:ident)=>{$apply!{
    runs:attribution::ProviderRun,core:analysis::catalog_core::Invocation,
    evidence:c1::Invocation,evidence_sources:c1::InvocationSource,evidence_inputs:c1::AnalysisInput,
    local:analysis::local::Invocation,local_outcomes:analysis::local::AnalysisOutcome,
    source:analysis::source_call::Invocation,source_outcomes:analysis::source_call::AnalysisOutcome,
}};}
macro_rules! metadata {($($field:ident:$ty:ty,)*)=>{
    pub struct Frames {$(pub $field:Rows<$ty>,)*}
    impl Frames {
        pub fn new(b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*}}
        pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"])),*]}
        pub fn visit(&mut self,n:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{
            $(if n==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})* Ok(false)
        }
    }
};}
frame_rows!(metadata);
impl Frames {
    pub fn parents(&self, b: &ResourceBudget) -> Result<Rows<c1::Invocation>, ModelError> {
        parents_from(
            &self.runs,
            &self.core,
            &self.evidence,
            &self.evidence_sources,
            &self.evidence_inputs,
            &catalog::evidence::frames::LowerFrames {
                local: &self.local,
                local_outcomes: &self.local_outcomes,
                source: &self.source,
                source_outcomes: &self.source_outcomes,
            },
            b,
        )
    }
}
pub fn parents(d: &Data, b: &ResourceBudget) -> Result<Rows<c1::Invocation>, ModelError> {
    parents_from(
        &d.source.facts.runs,
        &d.source.facts.core_invocations,
        &d.facts.evidence_invocations,
        &d.facts.evidence_sources,
        &d.facts.evidence_inputs,
        &d.source.runtime.lower(),
        b,
    )
}
fn parents_from(
    runs: &Rows<attribution::ProviderRun>,
    core: &Rows<analysis::catalog_core::Invocation>,
    evidence: &Rows<c1::Invocation>,
    sources: &Rows<c1::InvocationSource>,
    inputs: &Rows<c1::AnalysisInput>,
    lower: &catalog::evidence::frames::LowerFrames<'_>,
    b: &ResourceBudget,
) -> Result<Rows<c1::Invocation>, ModelError> {
    catalog::evidence::frames::verify(runs, core, evidence, sources, inputs, lower, b)?;
    let earlier = catalog::evidence::frames::parents(runs, core, b)?;
    let mut out = Rows::new(b);
    for parent in earlier.iter() {
        let mut matches = evidence.iter().filter(|i| {
            i.input == parent.input
                && i.context == parent.context
                && i.definition == catalog::evidence::build::definition().1.id()
                && i.subject.is_none()
        });
        let i = matches
            .next()
            .ok_or_else(|| invalid("C2 fixed completed C1 parent absent"))?;
        if matches.next().is_some() {
            return Err(invalid("duplicate C2 C1 parent"));
        }
        out.insert(i.clone())?;
    }
    Ok(out)
}
pub fn links(
    out: &Output,
    invocations: &Rows<owner::Invocation>,
    members: &Rows<catalog::CatalogMember>,
    b: &ResourceBudget,
) -> Result<Rows<SelectionInvocation>, ModelError> {
    let mut rows = Rows::new(b);
    for domain in out.domains.iter() {
        let member = need(members, domain.member)?;
        let mut matches = invocations.iter().filter(|r| {
            r.input == member.input
                && r.context == domain.analysis
                && r.definition == build::definition().1.id()
                && r.subject.is_none()
        });
        let i = matches
            .next()
            .ok_or_else(|| invalid("selection domain has no fixed invocation"))?;
        if matches.next().is_some() {
            return Err(invalid("selection domain has duplicate invocation"));
        }
        rows.insert(SelectionInvocation {
            domain: domain.id(),
            invocation: i.id(),
        })?;
    }
    Ok(rows)
}
pub fn verify(
    d: &Data,
    out: &Output,
    invocations: &Rows<owner::Invocation>,
    sources: &Rows<owner::InvocationSource>,
    inputs: &Rows<owner::AnalysisInput>,
    actual: &Rows<SelectionInvocation>,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let parents = parents(d, b)?;
    let mut expected_sources = Rows::new(b);
    let mut expected_inputs = Rows::new(b);
    let mut count = 0;
    for parent in parents.iter() {
        let mut matches = invocations.iter().filter(|r| {
            r.input == parent.input
                && r.context == parent.context
                && r.definition == build::definition().1.id()
                && r.subject.is_none()
        });
        let invocation = matches
            .next()
            .ok_or_else(|| invalid("selection invocation domain incomplete"))?;
        if matches.next().is_some() {
            return Err(invalid("selection frame has duplicate invocations"));
        }
        count += 1;
        let source = expected_sources.insert(owner::InvocationSource::CatalogEvidence {
            invocation: parent.id(),
        })?;
        expected_inputs.insert(owner::AnalysisInput {
            invocation: invocation.id(),
            parent: source,
        })?;
    }
    if count != invocations.len()
        || !expected_sources.same(sources)
        || !expected_inputs.same(inputs)
        || !actual.same(&links(out, invocations, &d.source.catalog.members, b)?)
    {
        return Err(invalid(
            "selection exact parent/invocation/domain membership differs",
        ));
    }
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Frames::inputs();
    inputs.extend([
        ValidationInput::of::<catalog::CatalogMember>(&["id"]),
        ValidationInput::of::<SelectionDomain>(&["id"]),
    ]);
    inputs.extend([
        ValidationInput::of::<owner::Invocation>(&["id"]),
        ValidationInput::of::<owner::InvocationSource>(&["id"]),
        ValidationInput::of::<owner::AnalysisInput>(&["id"]),
        ValidationInput::of::<SelectionInvocation>(&["id"]),
    ]);
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 2,
        name: "catalog_selection_invocation_domain",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                frames: Frames::new(b),
                members: Default::default(),
                charge: crate::domain::charged::StateCharge::new(b, "selection-frame-ownership"),
                domains: Rows::new(b),
                invocations: Rows::new(b),
                sources: Rows::new(b),
                inputs: Rows::new(b),
                links: Rows::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    frames: Frames,
    members:
        crate::domain::charged::ChargedMap<Id<catalog::CatalogMember>, Id<input::InputRevision>>,
    charge: crate::domain::charged::StateCharge,
    domains: Rows<SelectionDomain>,
    invocations: Rows<owner::Invocation>,
    sources: Rows<owner::InvocationSource>,
    inputs: Rows<owner::AnalysisInput>,
    links: Rows<SelectionInvocation>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.frames.visit(n, b)? {
            return Ok(());
        }
        if n == catalog::CatalogMember::NAME {
            use arrow_array::Array;
            let ids = b
                .column_by_name("id")
                .and_then(|a| {
                    a.as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|a| a.value_length() == 16 && a.null_count() == 0)
                .ok_or(ModelError::Schema("selection member identity projection"))?;
            let inputs = b
                .column_by_name("input")
                .and_then(|a| {
                    a.as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|a| a.value_length() == 16 && a.null_count() == 0)
                .ok_or(ModelError::Schema("selection member input projection"))?;
            fn id<T>(bytes: &[u8]) -> Result<Id<T>, ModelError> {
                serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                    _,
                    serde::de::value::Error,
                >::new(bytes.iter().copied()))
                .map_err(ModelError::codec)
            }
            for row in 0..b.num_rows() {
                let member = id(ids.value(row))?;
                let input = id(inputs.value(row))?;
                if self.members.get(&member).is_some_and(|old| *old != input) {
                    return Err(invalid("selection member input ownership differs"));
                }
                self.members.insert(&mut self.charge, member, input)?;
            }
        } else if n == SelectionDomain::NAME {
            self.domains.decode(b)?;
        } else if n == owner::Invocation::NAME {
            self.invocations.decode(b)?;
        } else if n == owner::InvocationSource::NAME {
            self.sources.decode(b)?;
        } else if n == owner::AnalysisInput::NAME {
            self.inputs.decode(b)?;
        } else if n == SelectionInvocation::NAME {
            self.links.decode(b)?;
        } else {
            return Err(invalid("undeclared selection frame input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let parents = self.frames.parents(&self.budget)?;
        let mut expected_sources = Rows::new(&self.budget);
        let mut expected_inputs = Rows::new(&self.budget);
        let mut count = 0;
        for parent in parents.iter() {
            let mut matches = self.invocations.iter().filter(|r| {
                r.input == parent.input
                    && r.context == parent.context
                    && r.definition == build::definition().1.id()
                    && r.subject.is_none()
            });
            let invocation = matches
                .next()
                .ok_or_else(|| invalid("selection invocation domain incomplete"))?;
            if matches.next().is_some() {
                return Err(invalid("selection frame has duplicate invocations"));
            }
            count += 1;
            let source = expected_sources.insert(owner::InvocationSource::CatalogEvidence {
                invocation: parent.id(),
            })?;
            expected_inputs.insert(owner::AnalysisInput {
                invocation: invocation.id(),
                parent: source,
            })?;
        }
        let mut links = Rows::new(&self.budget);
        for domain in self.domains.iter() {
            let input = self
                .members
                .get(&domain.member)
                .ok_or_else(|| invalid("selection domain member absent"))?;
            let mut matches = self.invocations.iter().filter(|r| {
                r.input == *input
                    && r.context == domain.analysis
                    && r.definition == build::definition().1.id()
                    && r.subject.is_none()
            });
            let invocation = matches
                .next()
                .ok_or_else(|| invalid("selection domain has no fixed invocation"))?;
            if matches.next().is_some() {
                return Err(invalid("selection domain has duplicate invocation"));
            }
            links.insert(SelectionInvocation {
                domain: domain.id(),
                invocation: invocation.id(),
            })?;
        }
        if count != self.invocations.len()
            || !expected_sources.same(&self.sources)
            || !expected_inputs.same(&self.inputs)
            || !links.same(&self.links)
        {
            return Err(invalid(
                "selection exact parent/invocation/domain membership differs",
            ));
        }
        Ok(())
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["catalog_selection_invocation_domain"]
}

#[cfg(test)]
mod compact_controls {
    use super::*;
    fn id<R>(byte: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    fn feed<R: Record>(check: &mut dyn InvariantCheck, rows: &[R]) {
        check.visit(R::NAME, &R::encode(rows).unwrap()).unwrap();
    }
    fn fixture(check: &mut dyn InvariantCheck, redirect: bool) {
        let run = attribution::ProviderRun {
            provider: id(1),
            context: id(2),
            input: id(3),
            configuration: ContentHash::of(b"config"),
            requested_families: ContentHash::of(b"families"),
        };
        let core = analysis::catalog_core::Invocation::new(
            run.input,
            run.context,
            catalog::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let local = analysis::local::Invocation::new(
            run.input,
            run.context,
            local_semantics::definition().1.id(),
            None,
            [],
        )
        .0;
        let source = analysis::source_call::Invocation::new(
            run.input,
            run.context,
            execution::configuration::source_calls().1.id(),
            None,
            [],
        )
        .0;
        let lower = [
            c1::InvocationSource::CatalogCore {
                invocation: core.id(),
            },
            c1::InvocationSource::Local {
                invocation: local.id(),
            },
            c1::InvocationSource::SourceCallAnalysis {
                invocation: source.id(),
            },
        ];
        let (evidence, evidence_inputs) = c1::Invocation::new(
            run.input,
            run.context,
            catalog::evidence::build::definition().1.id(),
            None,
            lower.iter().map(Record::id),
        );
        let evidence_source = owner::InvocationSource::CatalogEvidence {
            invocation: evidence.id(),
        };
        let (invocation, inputs) = owner::Invocation::new(
            run.input,
            run.context,
            build::definition().1.id(),
            None,
            [evidence_source.id()],
        );
        let member = catalog::CatalogMember {
            input: run.input,
            access: id(4),
            path: vec!["x".repeat(1 << 20)],
            name: "x".repeat(1 << 20),
        };
        let domain = SelectionDomain {
            member: member.id(),
            analysis: run.context,
            kind: DomainKind::SourceArtifacts,
            corpus_complete: true,
            analyzer_complete: true,
            contexts: ContentHash::of(b"contexts"),
            closure: ContentHash::of(b"closure"),
        };
        feed(check, &[run]);
        feed(check, &[core]);
        feed(check, &[local.clone()]);
        feed(check, &[source.clone()]);
        feed(
            check,
            &[analysis::local::AnalysisOutcome {
                invocation: local.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            }],
        );
        feed(
            check,
            &[analysis::source_call::AnalysisOutcome {
                invocation: source.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            }],
        );
        feed(check, &[evidence]);
        feed(check, &lower);
        feed(check, &evidence_inputs);
        feed(check, &[invocation.clone()]);
        feed(check, &[evidence_source]);
        feed(check, &inputs);
        feed(check, &[member]);
        feed(check, &[domain.clone()]);
        feed(
            check,
            &[SelectionInvocation {
                domain: domain.id(),
                invocation: if redirect { id(5) } else { invocation.id() },
            }],
        );
    }
    #[test]
    fn compact_frame_admission_preserves_exact_links_and_releases_labels() {
        let budget = ResourceBudget::fixed(64 << 10).unwrap();
        let invariant = invariants().pop().unwrap();
        let mut check = (invariant.create)(&budget);
        fixture(check.as_mut(), false);
        check.finish().unwrap();
        assert_eq!(budget.reserved(), 0);
        let mut check = (invariant.create)(&budget);
        fixture(check.as_mut(), true);
        assert!(check.finish().is_err());
        assert_eq!(budget.reserved(), 0);
    }
}
