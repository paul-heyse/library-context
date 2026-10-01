//! Bounded delegation over borrowed invocation and definition graphs. Containment arcs are
//! never calls. Earlier normalized rows supply the exact event, phase and qualification.
use crate::domain::{
    attribution::Modality,
    calls::CallPhase,
    input::{ArtifactUse, SourceRole},
    normalized::{Rows, entities::*, events::*},
    projection::{normalization::ProjectionData, snapshot::MaterializedGraph, *},
    resources::{Reservation, ResourceBudget},
    source::Occurrence,
    *,
};
use petgraph::visit::{EdgeRef, IntoEdges, IntoNodeIdentifiers};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("delegation premise absent"))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub depth: u32,
    pub vertices: u32,
    pub arcs: u32,
    pub witnesses: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    External,
    Synthetic,
    Dependency,
    Subsystem,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Depth,
    Vertices,
    Arcs,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Direct,
    BoundedPath,
    Boundary(Boundary),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub arc: ArcId,
    pub source: Id<EntityRef>,
    pub target: Id<EntityRef>,
    pub evidence: StepEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepEvidence {
    Call {
        event: Id<NormalizedCallEvent>,
        site: Id<Occurrence>,
        phase: CallPhase,
        qualification: Id<assertion::AssertionQualification>,
        modality: Modality,
        derived_dispatch: bool,
    },
    Declaration {
        owner: Id<OccurrenceOwnership>,
        declaration: Id<Occurrence>,
        callable: Id<CallableEntity>,
    },
}
impl Step {
    fn definite_call(self) -> bool {
        matches!(self.arc, ArcId::Invocation(_))
            && matches!(
                self.evidence,
                StepEvidence::Call {
                    modality: Modality::Definite,
                    derived_dispatch: false,
                    ..
                }
            )
    }
    pub fn derived_dispatch(self) -> bool {
        matches!(
            self.evidence,
            StepEvidence::Call {
                derived_dispatch: true,
                ..
            }
        )
    }
    fn site(self) -> Id<Occurrence> {
        match self.evidence {
            StepEvidence::Call { site, .. } => site,
            StepEvidence::Declaration { declaration, .. } => declaration,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reached {
    pub target: Id<EntityRef>,
    pub kind: Kind,
    pub depth: u32,
    pub paths: Vec<Vec<Step>>,
    pub witnesses_omitted: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub event: Id<NormalizedCallEvent>,
    pub assessment: Id<EventAssessment>,
    pub site: Id<Occurrence>,
    pub depth: u32,
    pub caller_path: Vec<Step>,
}
/// Retained results own their full allocation allowance after the borrowed graphs are released.
pub struct Traversal {
    seed: Id<EntityRef>,
    reached: Vec<Reached>,
    unresolved: Vec<Unresolved>,
    stop: Option<Stop>,
    vertices: u32,
    arcs: u32,
    _reservation: Box<dyn Reservation>,
}
impl Traversal {
    pub fn seed(&self) -> Id<EntityRef> {
        self.seed
    }
    pub fn reached(&self) -> &[Reached] {
        &self.reached
    }
    pub fn unresolved(&self) -> &[Unresolved] {
        &self.unresolved
    }
    pub fn stop(&self) -> Option<Stop> {
        self.stop
    }
    pub fn partial(&self) -> bool {
        matches!(self.stop, Some(Stop::Vertices | Stop::Arcs))
    }
    pub fn vertices_examined(&self) -> u32 {
        self.vertices
    }
    pub fn arcs_examined(&self) -> u32 {
        self.arcs
    }
}

pub struct Inputs<'a> {
    pub invocation: &'a MaterializedGraph,
    pub definition: &'a MaterializedGraph,
    /// Acknowledged normalized/source inputs, validated by their earlier owners.
    pub data: &'a ProjectionData,
    pub uses: &'a Rows<ArtifactUse>,
}
impl Inputs<'_> {
    fn step(
        &self,
        arc: Arc,
        context: Id<attribution::AnalysisContext>,
    ) -> Result<Step, ModelError> {
        if let ArcId::SourceDefinition(id) = arc.id {
            let owner = need(&self.data.owners, id)?;
            let EntityRef::Callable { callable } = need(&self.data.refs, arc.target)? else {
                return Err(invalid("definition target is not a callable"));
            };
            if owner.entity != arc.source
                || !matches!(need(&self.data.callables,*callable)?,CallableEntity::Source {declaration,..} if *declaration==owner.occurrence)
            {
                return Err(invalid("definition graph and canonical declaration differ"));
            }
            return Ok(Step {
                arc: arc.id,
                source: arc.source,
                target: arc.target,
                evidence: StepEvidence::Declaration {
                    owner: id,
                    declaration: owner.occurrence,
                    callable: *callable,
                },
            });
        }
        let alternative = match arc.id {
            ArcId::Invocation(id) | ArcId::Definition(id) => need(&self.data.alternatives, id)?,
            _ => return Err(invalid("delegation admitted a non-call/definition arc")),
        };
        let event = need(&self.data.events, alternative.event)?;
        let source = need(&self.data.alternative_sources, alternative.source)?;
        let raw = need(&self.data.targets, source.target())?;
        let qualification = need(&self.data.qualifications, raw.qualification)?;
        if event.context != context
            || qualification.context != context
            || need(&self.data.owners, event.owner)?.entity != arc.source
            || alternative.entity != Some(arc.target)
            || raw.site != event.site
            || raw.origin != event.origin
            || (matches!(arc.id, ArcId::Definition(_))
                && !matches!(raw.phase, CallPhase::Definition | CallPhase::Decorator))
            || (matches!(arc.id, ArcId::Invocation(_)) && raw.phase == CallPhase::Definition)
        {
            return Err(invalid("delegation graph and canonical event differ"));
        }
        Ok(Step {
            arc: arc.id,
            source: arc.source,
            target: arc.target,
            evidence: StepEvidence::Call {
                event: event.id(),
                site: event.site,
                phase: raw.phase,
                qualification: raw.qualification,
                modality: qualification.modality,
                derived_dispatch: matches!(source, CallAlternativeSource::DerivedDispatch { .. }),
            },
        })
    }
    fn boundary(
        &self,
        entity: Id<EntityRef>,
        subsystem: &BTreeSet<Id<EntityRef>>,
    ) -> Result<Option<Boundary>, ModelError> {
        let declaration = match need(&self.data.refs, entity)? {
            EntityRef::Callable { callable } => match need(&self.data.callables, *callable)? {
                CallableEntity::External { .. } => return Ok(Some(Boundary::External)),
                CallableEntity::Synthetic { .. } => return Ok(Some(Boundary::Synthetic)),
                CallableEntity::Source { declaration, .. } => Some(*declaration),
            },
            EntityRef::Class { class } => match need(&self.data.classes, *class)? {
                ClassEntity::External { .. } => return Ok(Some(Boundary::External)),
                ClassEntity::Synthetic { .. } => return Ok(Some(Boundary::Synthetic)),
                ClassEntity::Source { declaration } => Some(*declaration),
            },
            EntityRef::Module { .. } => None,
            _ => return Err(invalid("delegation endpoint has no executable owner")),
        };
        let artifact = match declaration {
            Some(id) => need(&self.data.occurrences, id)?.source,
            None => {
                let EntityRef::Module { module } = need(&self.data.refs, entity)? else {
                    unreachable!()
                };
                need(&self.data.modules, *module)?.source
            }
        };
        let artifact_input = need(&self.data.artifacts, artifact)?.input;
        let role = self.uses.iter().filter(|u| {
            (u.input == self.invocation.key().input || u.input == artifact_input)
                && u.artifact == artifact
        });
        let mut release = false;
        let mut observed = false;
        for usage in role {
            observed = true;
            if usage.role == SourceRole::Dependency {
                return Ok(Some(Boundary::Dependency));
            }
            release |= usage.role == SourceRole::Release;
        }
        if artifact_input != self.invocation.key().input {
            return Ok(Some(Boundary::External));
        }
        if !observed {
            return Err(invalid("delegation source role is absent"));
        }
        Ok((!release || !subsystem.contains(&entity)).then_some(Boundary::Subsystem))
    }
    pub fn traverse(
        &self,
        seed: Id<EntityRef>,
        subsystem: &[Id<EntityRef>],
        bounds: Bounds,
        budget: &ResourceBudget,
    ) -> Result<Traversal, ModelError> {
        let key = self.invocation.key();
        let other = self.definition.key();
        if key.name != ProjectionName::CallableInvocation
            || other.name != ProjectionName::DefinitionContainment
            || (key.input, key.context) != (other.input, other.context)
        {
            return Err(invalid(
                "delegation needs matching invocation/definition projections",
            ));
        }
        if bounds.depth == 0 || bounds.witnesses == 0 || bounds.vertices == 0 {
            return Err(invalid(
                "delegation depth, vertices and witnesses must be positive",
            ));
        }
        let vertices = self
            .invocation
            .vertex_count()
            .checked_add(self.definition.vertex_count())
            .ok_or_else(|| invalid("delegation vertex count overflow"))?;
        let arcs = self
            .invocation
            .arc_count()
            .checked_add(self.definition.arc_count())
            .ok_or_else(|| invalid("delegation arc count overflow"))?;
        // At most one shortest path per retained final arc, each bounded by the vertex universe.
        // Includes temporary maps/adjacency, BFS parents, doubled geometric vector capacities,
        // retained paths, and unresolved caller paths. No dense all-pairs topology is built.
        let paths = arcs
            .checked_add(self.data.event_assessments.len())
            .and_then(|n| n.checked_mul(vertices.min(bounds.depth as usize)))
            .and_then(|n| n.checked_mul(2 * size_of::<Step>()))
            .ok_or_else(|| invalid("delegation path allowance overflow"))?;
        let bytes = vertices
            .checked_mul(1024)
            .and_then(|n| {
                arcs.checked_mul(10 * size_of::<Step>() + 512)
                    .and_then(|a| n.checked_add(a))
            })
            .and_then(|n| {
                self.data
                    .event_assessments
                    .len()
                    .checked_mul(512)
                    .and_then(|a| n.checked_add(a))
            })
            .and_then(|n| {
                subsystem
                    .len()
                    .checked_mul(128)
                    .and_then(|s| n.checked_add(s))
            })
            .and_then(|n| n.checked_add(paths))
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(|| invalid("delegation allocation overflow"))?;
        let reservation = budget.reserve("delegation-traversal", bytes)?;
        let mut selected = BTreeSet::new();
        for id in subsystem {
            if !selected.insert(*id) || self.invocation.outgoing(*id).is_none() {
                return Err(invalid("duplicate or foreign subsystem entity"));
            }
        }
        if self.invocation.outgoing(seed).is_none() || self.boundary(seed, &selected)?.is_some() {
            return Err(invalid("delegation seed is outside the release subsystem"));
        }
        let mut unresolved = BTreeMap::<Id<EntityRef>, Vec<&EventAssessment>>::new();
        for assessment in self.data.event_assessments.iter() {
            let event = need(&self.data.events, assessment.event)?;
            if event.context == key.context
                && (assessment.unresolved || !assessment.complete || assessment.disagreement)
            {
                let owner = need(&self.data.owners, event.owner)?.entity;
                if self.invocation.outgoing(owner).is_some() {
                    unresolved.entry(owner).or_default().push(assessment);
                }
            }
        }
        self.invocation.with_native_graph(|calls| {
            self.definition.with_native_graph(|definitions| {
                let call_nodes = calls
                    .node_identifiers()
                    .map(|n| (calls.entity_id(n), n))
                    .collect::<BTreeMap<_, _>>();
                let definition_nodes = definitions
                    .node_identifiers()
                    .map(|n| (definitions.entity_id(n), n))
                    .collect::<BTreeMap<_, _>>();
                let adjacency = |id| -> Result<Vec<Step>, ModelError> {
                    let mut rows = Vec::new();
                    if let Some(node) = call_nodes.get(&id) {
                        for edge in calls.edges(*node) {
                            rows.push(self.step(
                                Arc {
                                    id: *edge.weight(),
                                    source: id,
                                    target: calls.entity_id(edge.target()),
                                },
                                key.context,
                            )?);
                        }
                    }
                    if let Some(node) = definition_nodes.get(&id) {
                        for edge in definitions.edges(*node).filter(|e| {
                            matches!(
                                e.weight(),
                                ArcId::Definition(_) | ArcId::SourceDefinition(_)
                            )
                        }) {
                            rows.push(self.step(
                                Arc {
                                    id: *edge.weight(),
                                    source: id,
                                    target: definitions.entity_id(edge.target()),
                                },
                                key.context,
                            )?);
                        }
                    }
                    rows.sort_unstable_by_key(|s| (s.target, s.site(), s.arc));
                    Ok(rows)
                };
                let mut depths = BTreeMap::from([(seed, 0u32)]);
                let mut parents = BTreeMap::<Id<EntityRef>, Step>::new();
                let mut queue = VecDeque::from([seed]);
                let mut into = BTreeMap::<Id<EntityRef>, Vec<Step>>::new();
                let mut boundaries = BTreeMap::new();
                let mut expanded = Vec::new();
                let mut arc_count = 0u32;
                let mut stop = None;
                let mut depth_limited = false;
                'search: while let Some(node) = queue.pop_front() {
                    let edges = adjacency(node)?;
                    if depths[&node] >= bounds.depth {
                        depth_limited |= !edges.is_empty() || unresolved.contains_key(&node);
                        continue;
                    }
                    for step in edges {
                        if arc_count == bounds.arcs {
                            stop = Some(Stop::Arcs);
                            break 'search;
                        }
                        arc_count += 1;
                        if let Some(boundary) = self.boundary(step.target, &selected)? {
                            boundaries.insert(step.target, boundary);
                        } else if !depths.contains_key(&step.target) {
                            if depths.len() >= bounds.vertices as usize {
                                stop = Some(Stop::Vertices);
                                break 'search;
                            }
                            depths.insert(step.target, depths[&node] + 1);
                            parents.insert(step.target, step);
                            queue.push_back(step.target);
                        }
                        into.entry(step.target).or_default().push(step);
                    }
                    expanded.push(node);
                }
                let path_to = |mut node| {
                    let mut path = Vec::new();
                    while let Some(step) = parents.get(&node) {
                        path.push(*step);
                        node = step.source;
                    }
                    path.reverse();
                    path
                };
                let mut reached = Vec::new();
                for (target, rows) in into {
                    if target == seed {
                        continue;
                    }
                    let depth = rows
                        .iter()
                        .map(|s| depths[&s.source] + 1)
                        .min()
                        .ok_or_else(|| invalid("empty reached target"))?;
                    let mut finals = rows
                        .into_iter()
                        .filter(|s| depths[&s.source] + 1 == depth)
                        .collect::<Vec<_>>();
                    finals.sort_unstable_by_key(|s| s.arc);
                    finals.dedup_by_key(|s| s.arc);
                    let direct = !boundaries.contains_key(&target)
                        && depth == 1
                        && finals.iter().any(|s| s.definite_call());
                    let first = if direct {
                        *finals.iter().find(|s| s.definite_call()).unwrap()
                    } else if boundaries.contains_key(&target) {
                        finals[0]
                    } else {
                        parents[&target]
                    };
                    let omitted = finals.len() > bounds.witnesses as usize;
                    let mut chosen = vec![first];
                    chosen.extend(
                        finals
                            .into_iter()
                            .filter(|s| s.arc != first.arc)
                            .take(bounds.witnesses as usize - 1),
                    );
                    let paths = chosen
                        .into_iter()
                        .map(|step| {
                            let mut path = path_to(step.source);
                            path.push(step);
                            path
                        })
                        .collect();
                    let kind = boundaries
                        .get(&target)
                        .map(|b| Kind::Boundary(*b))
                        .unwrap_or(if direct {
                            Kind::Direct
                        } else {
                            Kind::BoundedPath
                        });
                    reached.push(Reached {
                        target,
                        kind,
                        depth,
                        paths,
                        witnesses_omitted: omitted,
                    });
                }
                let mut open = Vec::new();
                for node in expanded {
                    for assessment in unresolved.get(&node).into_iter().flatten() {
                        let event = need(&self.data.events, assessment.event)?;
                        open.push(Unresolved {
                            event: event.id(),
                            assessment: assessment.id(),
                            site: event.site,
                            depth: depths[&node] + 1,
                            caller_path: path_to(node),
                        });
                    }
                }
                open.sort_unstable_by_key(|s| (s.event, s.assessment));
                Ok(Traversal {
                    seed,
                    reached,
                    unresolved: open,
                    stop: stop.or(depth_limited.then_some(Stop::Depth)),
                    vertices: u32::try_from(depths.len())
                        .map_err(|_| invalid("delegation vertex count overflow"))?,
                    arcs: arc_count,
                    _reservation: reservation,
                })
            })
        })
    }
}
