//! Captured entries feed the ordinary finite Summary transfer worklist through completed replay.
use super::{
    capture_bridge::CapturedEntryBinding,
    enriched_records::{BodyExecution, ExecutionOutcome, SourceExecutionInvocation},
    summary_production::SummaryData,
};
use crate::domain::{
    analysis::{
        self,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::*,
    conditions::Diagram,
    normalized::entities::EntityRef,
    resources::ResourceBudget,
    transfer::{TransferBranch, TransferKind, summary::TransferKey},
    value::{AccessPath, Place, PlaceRoot},
    *,
};
#[derive(Debug, Clone, PartialEq, Eq, crate::Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "summary_capture_witnesses",
    rule = "completed_own_frame_capture_summary"
)]
pub struct SummaryCaptureWitness {
    #[model(key)]
    pub invocation: Id<analysis::summary::AnalysisInvocation>,
    #[model(key)]
    pub transfer: Id<TransferKey>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key, premise)]
    pub binding: Id<CapturedEntryBinding>,
    #[model(key, premise)]
    pub call: Id<SourceExecutionInvocation>,
    #[model(key, premise)]
    pub body: Id<BodyExecution>,
    pub status: analysis::policy::EvidenceStatus,
}
impl analysis::support::sealed::DerivedEvidence for SummaryCaptureWitness {}
impl DerivedEvidence for SummaryCaptureWitness {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, crate::Domain)]
#[model(name="summary_capture_contributions",rule="summary_capture_contribution",conclusion=alternative)]
pub struct SummaryCaptureContribution {
    #[model(key)]
    pub alternative: Id<crate::domain::transfer::summary::TransferAlternative>,
    #[model(key, premise)]
    pub witness: Id<SummaryCaptureWitness>,
}
pub(super) struct CaptureSeed {
    pub witness: SummaryCaptureWitness,
    pub branch: TransferBranch<TransferKey>,
    pub roots: [PlaceRoot; 2],
    pub places: [Place; 2],
}
fn need<R: Record>(rows: &normalized::Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id).ok_or_else(|| {
        ModelError::Invalid(format!("capture Summary predecessor missing {}", R::NAME))
    })
}
pub(super) fn seeds(
    data: &SummaryData,
    invocation: &analysis::summary::AnalysisInvocation,
    budget: &ResourceBudget,
) -> Result<(Vec<CaptureSeed>, Box<dyn resources::Reservation>), ModelError> {
    let charge = budget.reserve(
        "summary-capture-seeds",
        data.captured_entries
            .len()
            .saturating_mul(size_of::<CaptureSeed>() + 1024),
    )?;
    let mut seeds = Vec::new();
    for capture in data.captured_entries.iter() {
        let parent = need(&data.enriched_invocations, capture.invocation)?;
        if (parent.input, parent.context) != (invocation.input, invocation.context) {
            continue;
        }
        if !capture.under_caller_entry {
            return Err(ModelError::Invalid(
                "capture loses caller entry contract".into(),
            ));
        }
        let header = need(&data.headers, capture.header)?;
        if header.owner != capture.caller || header.callee != capture.callee {
            return Err(ModelError::Invalid(
                "capture Summary changes exact frame".into(),
            ));
        }
        let q = need(&data.entry.qualifications, capture.qualification)?;
        if q.context != invocation.context
            || q.condition != Diagram::always().id()
            || q.assumptions != assumptions::AssumptionSet::empty_id()
            || q.modality != attribution::Modality::Definite
            || q.approximation != Approximation::Exact
        {
            continue;
        }
        let calls = data
            .fresh_calls
            .iter()
            .filter(|call| call.invocation == capture.invocation && call.header == capture.header)
            .collect::<Vec<_>>();
        if calls.len() != 1 {
            continue;
        }
        let call = calls[0];
        if call.qualification != capture.qualification
            || need(&data.exception_values, call.outcome)? != &ExecutionOutcome::Normal
        {
            continue;
        }
        let bodies = data
            .exception_bodies
            .iter()
            .filter(|body| body.invocation == capture.invocation && body.owner == capture.caller)
            .collect::<Vec<_>>();
        if bodies.len() != 1 {
            continue;
        }
        let body = bodies[0];
        if body.qualification != capture.qualification {
            continue;
        }
        let ExecutionOutcome::Return { site } = need(&data.exception_values, body.outcome)? else {
            continue;
        };
        let returned = data
            .bindings
            .placements
            .iter()
            .filter(|p| p.parent == Some(*site) && p.field == lexical::SyntaxField::Value)
            .collect::<Vec<_>>();
        if returned.len() != 1
            || returned[0].occurrence != need(&data.bindings.event_events, header.event)?.site
        {
            continue;
        }
        let EntityRef::Callable { callable } = need(&data.entry.refs, capture.caller)? else {
            continue;
        };
        let normalized::entities::CallableEntity::Source {
            declaration: caller,
            ..
        } = need(&data.entry.callables, *callable)?
        else {
            continue;
        };
        let input = match need(&data.captured_values, capture.value_source)? {
            super::capture_bridge::CapturedValueSource::Entry { declaration, .. } => {
                PlaceRoot::Entry {
                    declaration: *declaration,
                }
            }
            super::capture_bridge::CapturedValueSource::Literal { value, .. } => {
                PlaceRoot::Occurrence { occurrence: *value }
            }
        };
        let roots = [input, PlaceRoot::Return { callable: *caller }];
        let places = roots.clone().map(|root| Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        });
        let key = TransferKey {
            owner: capture.caller,
            input: places[0].id(),
            output: places[1].id(),
            context: invocation.context,
            scope: q.scope,
            modality: q.modality,
            approximation: q.approximation,
            kind: TransferKind::Identity,
            call_site: None,
            provenance: transfer::ProvenanceClass::DerivedSummary,
        };
        let status = analysis::support::inferred_status(
            analysis::Interpretation::Structural,
            [body.status, call.status],
        );
        analysis::policy::behavioral_support(status, false).map_err(|_| {
            ModelError::Invalid("capture Summary lacks admitted body support".into())
        })?;
        let witness = SummaryCaptureWitness {
            invocation: invocation.id(),
            transfer: key.id(),
            qualification: q.id(),
            binding: capture.id(),
            call: call.id(),
            body: body.id(),
            status,
        };
        seeds.push(CaptureSeed {
            witness,
            branch: TransferBranch::new(key, q.clone(), Diagram::always(), budget)?,
            roots,
            places,
        });
    }
    Ok((seeds, charge))
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<SummaryCaptureWitness>(),
        Relation::of::<SummaryCaptureContribution>(),
    ]
}
