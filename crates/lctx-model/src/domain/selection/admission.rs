//! Pure interpretation of verified C2 metadata. These rows grant no store capability.
use super::{
    build::{Output, invalid, need},
    classification::ClassificationData,
    *,
};
use crate::domain::{
    normalized::{Rows, coverage::EvidenceAvailability},
    resources::ResourceBudget,
    *,
};
use analysis::selection as owner;
/// Separate from the classifier inventory: canonical invocation, availability and capture evidence.
pub struct AdmissionData {
    pub invocations: Rows<owner::Invocation>,
    pub outcomes: Rows<owner::AnalysisOutcome>,
    pub coverage: Rows<owner::AnalysisCoverage>,
    pub links: Rows<SelectionInvocation>,
    pub sources: Rows<owner::SourceReceipt>,
}
impl AdmissionData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            invocations: Rows::new(b),
            outcomes: Rows::new(b),
            coverage: Rows::new(b),
            links: Rows::new(b),
            sources: Rows::new(b),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<owner::Invocation>(&["id"]),
            ValidationInput::of::<owner::AnalysisOutcome>(&["id"]),
            ValidationInput::of::<owner::AnalysisCoverage>(&["id"]),
            ValidationInput::of::<SelectionInvocation>(&["id"]),
            ValidationInput::of::<owner::SourceReceipt>(&["id"]),
        ]
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        macro_rules! row {
            ($t:ty,$f:ident) => {
                if n == <$t>::NAME {
                    self.$f.decode(b)?;
                    return Ok(true);
                }
            };
        }
        row!(owner::Invocation, invocations);
        row!(owner::AnalysisOutcome, outcomes);
        row!(owner::AnalysisCoverage, coverage);
        row!(SelectionInvocation, links);
        row!(owner::SourceReceipt, sources);
        Ok(false)
    }
    /// Requires canonical validator/receipt admission by the effect owner before use.
    /// Partial lower availability preserves unknown evidence; a refused/unrequested C2 owner does not serve.
    pub fn validate(
        &self,
        d: &ClassificationData,
        o: &Output,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self.links.same(&frames::links(
            o,
            &self.invocations,
            &d.source.catalog.members,
            b,
        )?) {
            return Err(invalid(
                "selection admission domain/invocation links differ",
            ));
        }
        let definition = build::definition().1.id();
        for i in self.invocations.iter() {
            if i.definition != definition || i.subject.is_some() {
                return Err(invalid("selection admission has foreign fixed definition"));
            }
            let mut outcomes = self.outcomes.iter().filter(|r| r.invocation == i.id());
            let outcome = outcomes
                .next()
                .ok_or_else(|| invalid("selection admission outcome absent"))?;
            if outcomes.next().is_some()
                || !matches!(
                    outcome.status,
                    analysis::AnalysisStatus::Completed | analysis::AnalysisStatus::Partial
                )
            {
                return Err(invalid(
                    "mandatory selection computation unavailable or unrequested",
                ));
            }
            let mut count = 0;
            for r in self.coverage.iter().filter(|r| r.invocation == i.id()) {
                if r.context != i.context
                    || r.capability != analysis::AnalysisCapability::CatalogSelection
                    || r.availability == EvidenceAvailability::NotRequested
                {
                    return Err(invalid(
                        "selection scoped availability crosses invocation or request",
                    ));
                }
                count += 1;
            }
            if count == 0 {
                return Err(invalid("selection scoped availability absent"));
            }
        }
        for r in self.outcomes.iter() {
            need(&self.invocations, r.invocation)?;
        }
        for r in self.coverage.iter() {
            need(&self.invocations, r.invocation)?;
        }
        for r in self.sources.iter() {
            need(&self.invocations, r.invocation)?;
        }
        Ok(())
    }
}


