#![allow(
    dead_code,
    reason = "Shared event fixtures support several focused suites"
)]
//! Explicit normalized-input premises for independent event expectations. This fixture supplies
//! correspondence and ownership; their producers are exercised by the N1 suites.
use lctx_model::domain::{
    calls::*,
    normalized::{entities::*, event_normalization::*, events::*},
    resources::ResourceBudget,
    *,
};
use std::collections::{BTreeMap, BTreeSet};
pub fn entity(symbol: &ProviderSymbol) -> EntityRef {
    if symbol.kind == SymbolKind::Class {
        EntityRef::Class {
            class: ClassEntity::External {
                symbol: symbol.id(),
            }
            .id(),
        }
    } else {
        EntityRef::Callable {
            callable: CallableEntity::External {
                symbol: symbol.id(),
            }
            .id(),
        }
    }
}
type Report<'a> = (
    &'a NormalizedSite,
    &'a BTreeMap<Id<CallTarget>, Vec<CallTargetSupport>>,
);
pub fn data(reports: &[Report<'_>], symbols: &[ProviderSymbol]) -> (EventData, ResourceBudget) {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = EventData::new(&budget);
    for symbol in symbols {
        data.symbols.insert(symbol.clone()).unwrap();
        let entity = entity(symbol);
        data.refs.insert(entity.clone()).unwrap();
        data.symbol_resolutions
            .insert(SymbolEntityResolution {
                symbol: symbol.id(),
                context: symbol.context,
                policy: normalized::policy_revision(),
                status: ResolutionStatus::Resolved,
                entity: Some(entity.id()),
                reason: EntityReason::ProviderExternal,
            })
            .unwrap();
    }
    for (report, supports) in reports {
        macro_rules! fill { ($($field:ident),+) => { $(for row in &report.$field { data.$field.insert(row.clone()).unwrap(); })+ }; }
        fill!(
            qualifications,
            channels,
            destinations,
            receivers,
            targets,
            resolutions,
            members
        );
        for support in supports.values().flatten() {
            data.target_supports.insert(support.clone()).unwrap();
        }
        for resolution in &report.resolutions {
            let support = supports.values().flatten().next().unwrap();
            data.resolution_supports
                .insert(CallResolutionSupport {
                    assertion: resolution.id(),
                    run: support.run,
                    surface: support.surface,
                    evidence: support.evidence,
                    origin: support.origin,
                    mode: support.mode,
                    fidelity: support.fidelity,
                })
                .unwrap();
            let owner = EntityRef::Occurrence {
                occurrence: resolution.site,
            };
            data.refs.insert(owner.clone()).unwrap();
            data.owners
                .insert(OccurrenceOwnership {
                    occurrence: resolution.site,
                    owner: resolution.site,
                    entity: owner.id(),
                })
                .unwrap();
        }
    }
    (data, budget)
}
pub struct Evaluated {
    pub data: EventData,
    pub output: EventOutput,
    pub budget: ResourceBudget,
}
pub fn evaluate(data: EventData, budget: ResourceBudget) -> Evaluated {
    let output = normalize(&data, &budget).unwrap();
    Evaluated {
        data,
        output,
        budget,
    }
}
impl Evaluated {
    pub fn assessment(&self) -> &EventAssessment {
        assert_eq!(self.output.assessments.len(), 1);
        self.output.assessments.iter().next().unwrap()
    }
    pub fn admitted(&self, policy: CallPolicy) -> Vec<Id<CallTarget>> {
        let mut targets: Vec<_> = self
            .output
            .admissions
            .iter()
            .filter(|a| {
                self.output
                    .policy_assessments
                    .get(a.assessment)
                    .unwrap()
                    .policy
                    == policy
            })
            .map(|a| {
                self.output
                    .alternative_sources
                    .get(self.output.alternatives.get(a.alternative).unwrap().source)
                    .unwrap()
                    .target()
            })
            .collect();
        targets.sort();
        targets
    }
    pub fn phase_targets(&self) -> BTreeMap<CallPhase, BTreeSet<Id<EntityRef>>> {
        let mut result: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for row in self.output.phase_targets.iter() {
            result.entry(row.phase).or_default().insert(row.entity);
        }
        result
    }
}
