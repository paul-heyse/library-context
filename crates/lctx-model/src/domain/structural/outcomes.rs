//! Structural completion is derived from the same retained frames and bounds used by replay.
use super::{Output, build::invalid, frames::Context};
use crate::domain::{
    analysis::{AnalysisCapability, AnalysisMethod, AnalysisStatus, structural as owner},
    normalized::{Rows, coverage::EvidenceAvailability},
    source::CoverageScope,
    obligation::ObligationKind,
    resources::ResourceBudget,
    *,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn capability(method: AnalysisMethod) -> Result<AnalysisCapability, ModelError> {
    match method {
        AnalysisMethod::Delegation => Ok(AnalysisCapability::Delegation),
        AnalysisMethod::DirectUsage => Ok(AnalysisCapability::DirectUsage),
        AnalysisMethod::Handoffs => Ok(AnalysisCapability::Handoffs),
        AnalysisMethod::Controls => Ok(AnalysisCapability::Controls),
        _ => Err(invalid("non-structural method in Structural invocation")),
    }
}

pub fn derive(
    context: &Context,
    output: &Output,
    budget: &ResourceBudget,
) -> Result<Rows<owner::AnalysisOutcome>, ModelError> {
    let entries = output
        .traversals
        .len()
        .checked_add(output.control_traversals.len())
        .and_then(|n| n.checked_add(context.invocations.len()))
        .and_then(|n| n.checked_add(context.local.len()))
        .and_then(|n| n.checked_add(context.local_coverage.len()))
        .ok_or_else(|| invalid("Structural outcome index size overflow"))?;
    let _charge = budget.reserve(
        "structural outcome indexes",
        entries
            .checked_mul(128)
            .ok_or_else(|| invalid("Structural outcome allocation overflow"))?,
    )?;
    let delegation_stopped: BTreeSet<_> = output
        .traversals
        .iter()
        .filter(|r| r.stop.is_some())
        .map(|r| r.frame)
        .collect();
    let controls_stopped: BTreeSet<_> = output
        .control_traversals
        .iter()
        .filter(|r| r.stop.is_some())
        .map(|r| r.frame)
        .collect();
    // Controls and Transfers share the declared Python artifact domain, but not their
    // nonempty completeness rules. Only an acknowledged empty Local root can close Controls.
    let mut local_roots = BTreeMap::new();
    for local in context.local.iter().filter(|local| local.subject.is_none()) {
        local_roots.entry((local.input, local.context))
            .and_modify(|root| *root = None)
            .or_insert(Some(local.id()));
    }
    let mut empty_local = BTreeSet::new();
    for coverage in context.local_coverage.iter() {
        let Some(local) = context.local.get(coverage.invocation) else { continue; };
        if local.subject.is_none()
            && coverage.capability == AnalysisCapability::Transfers
            && coverage.context == local.context
            && coverage.scope == (CoverageScope::Input { input: local.input }).id()
            && coverage.availability == EvidenceAvailability::NoScope
            && coverage.reason.is_none()
        {
            empty_local.insert(local.id());
        }
    }
    let mut seen = BTreeSet::new();
    let mut outcomes = Rows::new(budget);
    for frame in output.frames.iter() {
        for (id, method) in [
            (frame.invocation, AnalysisMethod::Delegation),
            (frame.usage_invocation, AnalysisMethod::DirectUsage),
            (frame.handoff_invocation, AnalysisMethod::Handoffs),
            (frame.control_invocation, AnalysisMethod::Controls),
        ] {
            let invocation = context
                .invocations
                .get(id)
                .ok_or_else(|| invalid("Structural outcome invocation absent"))?;
            let definition = context
                .definitions
                .get(invocation.definition)
                .ok_or_else(|| invalid("Structural outcome definition absent"))?;
            if definition.method != method || !seen.insert(id) {
                return Err(invalid("Structural outcome invocation membership differs"));
            }
            let bounded = match method {
                AnalysisMethod::Delegation => delegation_stopped.contains(&frame.id()),
                AnalysisMethod::Controls => controls_stopped.contains(&frame.id()),
                _ => false,
            };
            let (status, reason) =
                if method == AnalysisMethod::Controls && !frame.controls_requested {
                    (
                        AnalysisStatus::NotRequested,
                        Some(ObligationKind::NotRequested),
                    )
                } else if bounded {
                    (AnalysisStatus::Partial, Some(ObligationKind::BudgetReached))
                } else if method == AnalysisMethod::Controls {
                    if local_roots.get(&(invocation.input, invocation.context))
                        .and_then(|root| *root).is_some_and(|root| empty_local.contains(&root)) {
                        (AnalysisStatus::Completed, None)
                    } else {
                        (
                            AnalysisStatus::Partial,
                            Some(ObligationKind::IncompleteDomain),
                        )
                    }
                } else {
                    (AnalysisStatus::Completed, None)
                };
            outcomes.insert(owner::AnalysisOutcome {
                invocation: id,
                status,
                reason,
            })?;
        }
    }
    if seen.len() != context.invocations.len() {
        return Err(invalid("Structural outcome invocation domain incomplete"));
    }
    Ok(outcomes)
}
