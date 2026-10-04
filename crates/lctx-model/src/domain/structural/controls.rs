//! Pass B preserves parameter identity paths and qualified implementation restrictions.
use super::{
    build::{invalid, need},
    handoffs, *,
};
use crate::Domain;
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, structural as owner},
    assertion::{AssertionQualification, Support},
    attribution::Modality,
    calls::*,
    conditions::{BooleanOperation, Diagram, entry::EntryValueWitness},
    flow::*,
    local_semantics::*,
    normalized::{Rows, bindings::*, entities::*, signature_applicability::BindingAuthority},
    source::*,
    syntax::*,
    value::*,
    *,
};
#[macro_export]
macro_rules! structural_control_inputs {
    ($apply:ident) => {
        $apply! {
         entries:$crate::domain::conditions::entry::EntryValueWitness,
         entry_sources:$crate::domain::conditions::entry::EntryAccessSource,
         contributions:$crate::domain::local_semantics::LocalContribution,
         assessments:$crate::domain::local_semantics::LocalAssessment,
         details:$crate::domain::syntax::SyntaxDetail,
         observations:$crate::domain::syntax::SyntaxDetailObservation,
         supports:$crate::domain::syntax::SyntaxDetailSupport,
        }
    };
}
macro_rules! inputs {($($f:ident:$ty:ty,)*)=>{
 pub struct Data {$(pub $f:Rows<$ty>,)*}
 impl Data {pub fn new(b:&resources::ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}}
};}
crate::structural_control_inputs!(inputs);
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_argument_flows")]
pub struct ArgumentFlow {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub binding: Id<CallBinding>,
    #[model(key)]
    pub contribution: Id<LocalContribution>,
    #[model(key)]
    pub alias: Option<Id<handoffs::ValueSource>>,
    pub caller: Id<EntityRef>,
    pub callee: Id<EntityRef>,
    pub source: Id<ParameterEntity>,
    pub formal: Id<ParameterEntity>,
    pub phase: CallPhase,
    pub modality: Modality,
    pub qualification: Id<AssertionQualification>,
    pub conditional: bool,
    pub may_catch: bool,
    pub value_tested: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_literal_arguments")]
pub struct LiteralArgument {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub binding: Id<CallBinding>,
    #[model(key)]
    pub observation: Id<SyntaxDetailObservation>,
    #[model(key)]
    pub support: Id<SyntaxDetailSupport>,
    pub caller: Id<EntityRef>,
    pub callee: Id<EntityRef>,
    pub literal: Id<Literal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_unfollowed_arguments")]
pub struct UnfollowedArgument {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
    #[model(key)]
    pub argument: Id<Occurrence>,
    pub binding: Option<Id<CallBinding>>,
    #[model(key)]
    pub value: Id<FlowValueObservation>,
    pub caller: Id<EntityRef>,
    pub formal: Id<ParameterEntity>,
    pub assessment: Option<Id<LocalAssessment>>,
    pub reason: obligation::ObligationKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_control_traversals")]
pub struct ControlTraversal {
    #[model(key)]
    pub frame: Id<StructuralFrame>,
    #[model(key)]
    pub seed: Id<EntityRef>,
    #[model(key)]
    pub formal: Id<ParameterEntity>,
    pub stop: Option<TraversalStop>,
    pub vertices: i64,
    pub arcs: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_control_paths")]
pub struct ControlPath {
    #[model(key)]
    pub traversal: Id<ControlTraversal>,
    #[model(key)]
    pub target: Id<EntityRef>,
    #[model(key)]
    pub formal: Id<ParameterEntity>,
    #[model(key)]
    pub may_suppress: bool,
    pub length: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_control_steps")]
pub struct ControlStep {
    #[model(key)]
    pub path: Id<ControlPath>,
    #[model(key)]
    pub ordinal: i64,
    pub flow: Id<ArgumentFlow>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_conditional_raises")]
pub struct ConditionalRaise {
    #[model(key)]
    pub path: Id<ControlPath>,
    #[model(key)]
    pub entry: Id<EntryValueWitness>,
    #[model(key)]
    pub leaf: Id<FlowTestLeafObservation>,
    #[model(key)]
    pub leaf_support: Id<FlowTestLeafSupport>,
    #[model(key)]
    pub region: Id<FlowRegionObservation>,
    #[model(key)]
    pub support: Id<FlowRegionSupport>,
    pub positive: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "structural_unfollowed_paths")]
pub struct UnfollowedPath {
    #[model(key)]
    pub path: Id<ControlPath>,
    #[model(key)]
    pub argument: Id<UnfollowedArgument>,
}
fn same(a: &Occurrence, b: &Occurrence) -> bool {
    a.source == b.source
        && a.structural_path == b.structural_path
        && a.syntax_kind == b.syntax_kind
        && a.start == b.start
        && a.end == b.end
}
fn inside(a: &Occurrence, b: &Occurrence) -> bool {
    a.source == b.source
        && a.structural_path.starts_with(&b.structural_path)
        && a.start >= b.start
        && a.end <= b.end
}
fn origin<'a>(
    h: &'a handoffs::Data,
    site: &Occurrence,
) -> Result<Option<&'a normalized::entities::OccurrenceOwnership>, ModelError> {
    let mut owners = h.entry.owners.iter().filter(|o| {
        h.entry
            .occurrences
            .get(o.occurrence)
            .is_some_and(|r| same(r, site))
    });
    let first = owners.next();
    if owners.any(|o| first.is_some_and(|f| f.entity != o.entity)) {
        return Ok(None);
    }
    Ok(first)
}
fn suppression(
    h: &handoffs::Data,
    d: &Data,
    site: &Occurrence,
    formal: Id<ParameterEntity>,
    owner: Id<EntityRef>,
    context: Id<attribution::AnalysisContext>,
) -> Result<(bool, bool), ModelError> {
    let mut catches = false;
    let mut tested = false;
    for row in h
        .entry
        .occurrences
        .iter()
        .filter(|o| inside(site, o) && !same(site, o))
    {
        if !h
            .entry
            .owners
            .iter()
            .any(|o| o.occurrence == row.id() && o.entity == owner)
        {
            continue;
        }
        catches |= matches!(row.syntax_kind, SyntaxKind::StmtTry | SyntaxKind::StmtWith);
        if matches!(
            row.syntax_kind,
            SyntaxKind::StmtIf | SyntaxKind::StmtWhile | SyntaxKind::ExprIf
        ) {
            for witness in d.entries.iter().filter(|w| {
                w.owner == owner
                    && w.formal == formal
                    && w.context == context
                    && matches!(
                        d.entry_sources.get(w.access_source),
                        Some(crate::domain::conditions::entry::EntryAccessSource::Use { .. })
                    )
            }) {
                let access = need(&h.entry.occurrences, witness.access)?;
                for leaf in h.entry.leaves.iter() {
                    let test = need(&h.entry.occurrences, leaf.test)?;
                    if inside(access, test)
                        && inside(test, row)
                        && h.entry
                            .qualifications
                            .get(leaf.qualification)
                            .is_some_and(|q| q.context == context)
                    {
                        tested = true;
                    }
                }
            }
        }
    }
    Ok((catches, tested))
}
fn target<'a>(
    base: &'a build::Data,
    alternative: &normalized::events::NormalizedCallAlternative,
) -> Result<&'a CallTarget, ModelError> {
    need(
        &base.projection.targets,
        need(&base.events.alternative_sources, alternative.source)?.target(),
    )
}
struct FlowInputs<'a> {
    d: &'a Data,
    h: &'a handoffs::Data,
    base: &'a build::Data,
}

fn flow(
    flow_inputs: FlowInputs<'_>,
    frame: &StructuralFrame,
    binding: &CallBinding,
    proof: &LocalContribution,
    alias: Option<Id<handoffs::ValueSource>>,
    out: &mut Output,
    budget: &resources::ResourceBudget,
) -> Result<bool, ModelError> {
    let FlowInputs { d, h, base } = flow_inputs;
    let attempt = need(&h.attempts, binding.attempt)?;
    let alt = need(&base.events.alternatives, attempt.alternative)?;
    let Some(callee) = alt.entity else {
        return Ok(false);
    };
    let event = need(&base.events.events, alt.event)?;
    let call = target(base, alt)?;
    let q = need(&base.projection.qualifications, call.qualification)?;
    let entry = need(&d.entries, proof.entry)?;
    let slot = need(&h.slots, binding.slot)?;
    let mut links = h
        .entry
        .links
        .iter()
        .filter(|l| l.parameter == slot.parameter);
    let Some(link) = links.next() else {
        return Ok(false);
    };
    if links.next().is_some() {
        return Ok(false);
    }
    let site = need(&h.entry.occurrences, event.site)?;
    let Some(caller) = origin(h, site)? else {
        return Ok(false);
    };
    if caller.entity != entry.owner || q.context != entry.context {
        return Ok(false);
    }
    let local_q = need(&h.entry.qualifications, proof.qualification)?;
    let alias = alias.map(|id| need(&out.handoff_values, id).cloned()).transpose()?;
    let q = super::qualifications::intersect(base, q, local_q, alias.as_ref(), site.source, out, budget)?;
    let (catches, tested) = suppression(h, d, site, entry.formal, entry.owner, q.context)?;
    let modality = if matches!(
        base.events.alternative_sources.get(alt.source),
        Some(normalized::events::CallAlternativeSource::DerivedDispatch { .. })
    ) {
        q.modality.weakest(Modality::Candidate)
    } else {
        q.modality
    };
    out.argument_flows.insert(ArgumentFlow {
        frame: frame.id(),
        binding: binding.id(),
        contribution: proof.id(),
        alias: alias.as_ref().map(Record::id),
        caller: entry.owner,
        callee,
        source: entry.formal,
        formal: link.entity,
        phase: call.phase,
        modality,
        qualification: q.id(),
        conditional: q.condition != Diagram::always().id(),
        may_catch: catches,
        value_tested: tested,
    })?;
    Ok(true)
}
/// Only exact Local identity contributions and the canonical effective binder permit following.
pub fn produce(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    invocation: &owner::Invocation,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let h = &base.handoffs;
    for binding in h.bindings.iter() {
        let attempt = need(&h.attempts, binding.attempt)?;
        let alt = need(&base.events.alternatives, attempt.alternative)?;
        let event = need(&base.events.events, alt.event)?;
        if event.context != invocation.context
            || !alt
                .entity
                .is_some_and(|e| out.scope.iter().any(|s| s.entity == e))
        {
            continue;
        }
        let BindingSource::Actual { occurrence } = need(&h.sources, binding.source)? else {
            continue;
        };
        let argument = need(&h.entry.occurrences, *occurrence)?;
        let site = need(&h.entry.occurrences, event.site)?;
        let Some(caller) = origin(h, site)? else {
            continue;
        };
        if !out.scope.iter().any(|s| s.entity == caller.entity) {
            continue;
        }
        let bound = attempt.outcome == BindingOutcome::Bound
            && attempt.authority == BindingAuthority::EffectiveInvocation
            && binding.projection == BindingProjection::Whole.id();
        for proof in d.contributions.iter() {
            let value = need(&h.entry.values, proof.value)?;
            if value.kind != FlowSinkKind::Argument
                || value.transfer != transfer::TransferKind::Identity
                || !same(need(&h.entry.occurrences, value.sink)?, argument)
                || !bound
            {
                continue;
            }
            flow(
                FlowInputs { d, h, base },
                frame,
                binding,
                proof,
                None,
                out,
                budget,
            )?;
        }
        if bound
            && argument.syntax_kind == SyntaxKind::ExprName
            && let Some((_, alias)) =
                handoffs::named_definition(h, argument, invocation.context, budget)?
            && let handoffs::ValueSource::Named { definition, .. } = &alias
        {
            for proof in d.contributions.iter() {
                let value = need(&h.entry.values, proof.value)?;
                if value.kind == FlowSinkKind::Definition
                    && value.transfer == transfer::TransferKind::Identity
                    && proof.definition == Some(*definition)
                    && alias_covers(h, &alias, proof.qualification, budget)?
                {
                    out.handoff_values.insert(alias.clone())?;
                    flow(
                        FlowInputs { d, h, base },
                        frame,
                        binding,
                        proof,
                        Some(alias.id()),
                        out,
                        budget,
                    )?;
                }
            }
        }
        if bound {
            for row in d.observations.iter().filter(|r| {
                h.entry
                    .occurrences
                    .get(r.occurrence)
                    .is_some_and(|o| same(o, argument))
            }) {
                let Some(SyntaxDetail::Literal { literal }) = d.details.get(row.detail) else {
                    continue;
                };
                for support in d.supports.iter().filter(|s| s.assertion == row.id()) {
                    let Some(a) = support.attribution() else {
                        continue;
                    };
                    if !h.entry.runs.get(a.run).is_some_and(|r| {
                        r.context == invocation.context && r.input == invocation.input
                    }) {
                        continue;
                    }
                    let premise =
                        analysis::native::NativeAssertionPremise::SyntaxDetailObservation {
                            assertion: row.id(),
                            support: support.id(),
                        };
                    if !h.native.iter().any(|n| {
                        n.premise == premise.id()
                            && n.qualification == row.qualification
                            && n.family == attribution::FactFamily::Syntax
                            && n.fidelity == attribution::Fidelity::NativeStructural
                    }) {
                        continue;
                    }
                    out.literal_arguments.insert(LiteralArgument {
                        frame: frame.id(),
                        binding: binding.id(),
                        observation: row.id(),
                        support: support.id(),
                        caller: caller.entity,
                        callee: alt.entity.unwrap(),
                        literal: *literal,
                    })?;
                }
            }
        }
        for value in h.entry.values.iter().filter(|v| {
            v.kind == FlowSinkKind::Argument
                && h.entry
                    .qualifications
                    .get(v.qualification)
                    .is_some_and(|q| q.context == invocation.context)
                && h.entry
                    .occurrences
                    .get(v.sink)
                    .is_some_and(|o| same(o, argument))
        }) {
            let use_ = need(&h.entry.uses, value.use_)?;
            let place = need(&h.entry.places, use_.place)?;
            let Some(PlaceRoot::Formal { declaration }) = h.entry.roots.get(place.root) else {
                continue;
            };
            let formal = ParameterEntity::Source {
                declaration: *declaration,
            }
            .id();
            if value.transfer == transfer::TransferKind::Identity
                && out.argument_flows.iter().any(|f| {
                    f.binding == binding.id()
                        && f.source == formal
                        && d.contributions
                            .get(f.contribution)
                            .is_some_and(|c| c.value == value.id())
                })
            {
                continue;
            }
            let assessment = d.assessments.iter().find(|a| a.value == value.id());
            let reason = if !bound {
                obligation::ObligationKind::AmbiguousBinding
            } else {
                assessment
                    .and_then(|a| a.reason)
                    .unwrap_or(obligation::ObligationKind::ConditionTransferUnsupported)
            };
            out.unfollowed_arguments.insert(UnfollowedArgument {
                frame: frame.id(),
                attempt: attempt.id(),
                argument: argument.id(),
                binding: Some(binding.id()),
                value: value.id(),
                caller: caller.entity,
                formal,
                assessment: assessment.map(Record::id),
                reason,
            })?;
        }
    }
    unmapped(d, base, frame, invocation, out)?;
    traverse(d, base, frame, invocation.context, settings, out, budget)
}
struct State {
    target: Id<EntityRef>,
    formal: Id<ParameterEntity>,
    suppressed: bool,
    path: Vec<Id<ArgumentFlow>>,
}
impl HeapSize for State {
    fn heap_bytes(&self) -> usize {
        self.path.heap_bytes()
    }
}
fn traverse(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    context: Id<attribution::AnalysisContext>,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut roots = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(budget, "structural-control-worklist");
    for candidate in out.public.iter().filter(|c| c.in_subsystem) {
        let EntityRef::Callable { callable } = need(&base.handoffs.entry.refs, candidate.entity)?
        else {
            continue;
        };
        let CallableEntity::Source { declaration, .. } =
            need(&base.handoffs.entry.callables, *callable)?
        else {
            continue;
        };
        for link in base.handoffs.entry.links.iter() {
            let parameter = need(&base.handoffs.entry.parameters, link.parameter)?;
            let signature = need(&base.handoffs.entry.signatures, parameter.signature)?;
            if base
                .handoffs
                .entry
                .symbol_declarations
                .iter()
                .any(|row| row.symbol == signature.symbol && row.declaration == *declaration)
            {
                roots.insert(&mut charge, (candidate.entity, link.entity))?;
            }
        }
    }
    for (seed, formal) in roots.iter() {
        let traversal_id = ControlTraversal {
            frame: frame.id(),
            seed: *seed,
            formal: *formal,
            stop: None,
            vertices: 0,
            arcs: 0,
        }
        .id();
        let mut visited = charged::ChargedSet::default();
        let mut pending = charged::ChargedVec::default();
        pending.push(
            &mut charge,
            State {
                target: *seed,
                formal: *formal,
                suppressed: false,
                path: vec![],
            },
        )?;
        let mut cursor = 0;
        let mut arcs = 0;
        let mut stop = None;
        while cursor < pending.len() {
            let state = &pending[cursor];
            let key = (state.target, state.formal, state.suppressed);
            if visited.contains(&key) {
                cursor += 1;
                continue;
            }
            if visited.len() >= settings.vertices as usize {
                stop = Some(TraversalStop::Vertices);
                break;
            }
            visited.insert(&mut charge, key)?;
            let _path = budget.reserve("control-path-copy", state.path.heap_bytes())?;
            let path = ControlPath {
                traversal: traversal_id,
                target: state.target,
                formal: state.formal,
                may_suppress: state.suppressed,
                length: state.path.len() as i64,
            };
            out.control_paths.insert(path.clone())?;
            for (ordinal, flow) in state.path.iter().enumerate() {
                out.control_steps.insert(ControlStep {
                    path: path.id(),
                    ordinal: ordinal as i64,
                    flow: *flow,
                })?;
            }
            for row in out
                .unfollowed_arguments
                .iter()
                .filter(|r| r.caller == state.target && r.formal == state.formal)
            {
                out.unfollowed_paths.insert(UnfollowedPath {
                    path: path.id(),
                    argument: row.id(),
                })?;
            }
            if !state.suppressed {
                raises(d, &base.handoffs, &path, context, out, budget)?;
            }
            let current = (
                state.target,
                state.formal,
                state.suppressed,
                state.path.len(),
            );
            let _temporary = budget.reserve(
                "control-pending-paths",
                state
                    .path
                    .heap_bytes()
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(size_of::<Id<ArgumentFlow>>()))
                    .ok_or_else(|| invalid("control path allowance overflow"))?,
            )?;
            let path = state.path.clone();
            for edge in out
                .argument_flows
                .iter()
                .filter(|e| e.caller == current.0 && e.source == current.1)
            {
                if current.3 >= settings.depth as usize {
                    stop = Some(TraversalStop::Depth);
                    continue;
                }
                if arcs >= settings.arcs {
                    stop = Some(TraversalStop::Arcs);
                    break;
                }
                arcs += 1;
                let mut steps = path.clone();
                steps.push(edge.id());
                pending.push(
                    &mut charge,
                    State {
                        target: edge.callee,
                        formal: edge.formal,
                        suppressed: current.2 || edge.may_catch || edge.value_tested,
                        path: steps,
                    },
                )?;
            }
            cursor += 1;
            if stop == Some(TraversalStop::Arcs) {
                break;
            }
        }
        out.control_traversals.insert(ControlTraversal {
            frame: frame.id(),
            seed: *seed,
            formal: *formal,
            stop,
            vertices: visited.len() as i64,
            arcs,
        })?;
    }
    Ok(())
}
/// A source-level branch observation needs an exact entry read in the native test. It does
/// not require (or establish) predicate stability for substitution into another invocation.
fn raises(
    d: &Data,
    h: &handoffs::Data,
    path: &ControlPath,
    context: Id<attribution::AnalysisContext>,
    out: &mut Output,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    for entry in d.entries.iter().filter(|e| {
        (e.owner, e.formal, e.context) == (path.target, path.formal, context)
            && matches!(
                d.entry_sources.get(e.access_source),
                Some(crate::domain::conditions::entry::EntryAccessSource::Use { .. })
            )
    }) {
        let read = need(&h.entry.occurrences, entry.access)?;
        for leaf in h.entry.leaves.iter() {
            let test = need(&h.entry.occurrences, leaf.test)?;
            if !inside(read, test) {
                continue;
            }
            let lq = need(&h.entry.qualifications, leaf.qualification)?;
            if lq.context != entry.context {
                continue;
            }
            for leaf_support in h.entry.leaf_supports.iter().filter(|s| {
                s.assertion == leaf.id() && s.attribution().is_some_and(|a| a.run == entry.run)
            }) {
                let lp = analysis::native::NativeAssertionPremise::Leaf {
                    assertion: leaf.id(),
                    support: leaf_support.id(),
                };
                if !h.native.iter().any(|n| {
                    n.premise == lp.id()
                        && n.qualification == lq.id()
                        && n.family == attribution::FactFamily::Flow
                        && n.fidelity == attribution::Fidelity::NativeStructural
                }) {
                    continue;
                }
                for region in h.entry.regions.iter() {
                    let statement = need(&h.entry.occurrences, region.statement)?;
                    if statement.syntax_kind != SyntaxKind::StmtRaise
                        || !h
                            .entry
                            .owners
                            .iter()
                            .any(|o| o.occurrence == statement.id() && o.entity == entry.owner)
                    {
                        continue;
                    }
                    let if_ = h
                        .entry
                        .occurrences
                        .iter()
                        .filter(|o| {
                            o.syntax_kind == SyntaxKind::StmtIf
                                && inside(test, o)
                                && inside(statement, o)
                        })
                        .min_by_key(|o| o.end - o.start);
                    let Some(_if_) = if_ else { continue };
                    if h.entry.occurrences.iter().any(|o| {
                        inside(statement, o)
                            && matches!(o.syntax_kind, SyntaxKind::StmtTry | SyntaxKind::StmtWith)
                            && h.entry.owners.iter().any(|owned| {
                                owned.occurrence == o.id() && owned.entity == entry.owner
                            })
                    }) {
                        continue;
                    }
                    let q = need(&h.entry.qualifications, region.qualification)?;
                    if (q.context, q.scope) != (entry.context, lq.scope) {
                        continue;
                    }
                    let _nodes = budget.reserve(
                        "control-raise-condition",
                        h.entry
                            .condition_nodes
                            .len()
                            .checked_mul(2048)
                            .ok_or_else(|| invalid("condition allocation overflow"))?,
                    )?;
                    let nodes = h.entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
                    let condition =
                        Diagram::from_records(need(&h.entry.conditions, q.condition)?, &nodes)?;
                    if condition.is_false() {
                        continue;
                    }
                    let atom = Diagram::from_atom(leaf.atom);
                    let mut polarity = None;
                    for positive in [true, false] {
                        let arm = if positive {
                            atom.clone()
                        } else {
                            atom.not()
                                .map_err(|e| invalid(format!("guard complement refused: {e:?}")))?
                        };
                        let compared = condition
                            .admitted_binary(&arm, BooleanOperation::Conjunction, budget)
                            .map_err(|e| invalid(format!("guard implication refused: {e:?}")))?;
                        if compared.into_parts().0.id() == condition.id() {
                            polarity = Some(positive);
                        }
                    }
                    let Some(positive) = polarity else { continue };
                    for support in h.entry.region_supports.iter().filter(|s| {
                        s.assertion == region.id()
                            && s.attribution().is_some_and(|a| a.run == entry.run)
                    }) {
                        let premise = analysis::native::NativeAssertionPremise::Region {
                            assertion: region.id(),
                            support: support.id(),
                        };
                        if h.native.iter().any(|n| {
                            n.premise == premise.id()
                                && n.qualification == q.id()
                                && n.family == attribution::FactFamily::Flow
                                && n.fidelity == attribution::Fidelity::NativeStructural
                        }) {
                            out.conditional_raises.insert(ConditionalRaise {
                                path: path.id(),
                                entry: entry.id(),
                                leaf: leaf.id(),
                                leaf_support: leaf_support.id(),
                                region: region.id(),
                                support: support.id(),
                                positive,
                            })?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn alias_covers(
    h: &handoffs::Data,
    alias: &handoffs::ValueSource,
    local: Id<AssertionQualification>,
    budget: &resources::ResourceBudget,
) -> Result<bool, ModelError> {
    let handoffs::ValueSource::Named { observation, .. } = alias else {
        return Ok(false);
    };
    let q = need(
        &h.entry.qualifications,
        need(&h.entry.use_observations, *observation)?.qualification,
    )?;
    let local = need(&h.entry.qualifications, local)?;
    if (q.context, q.scope) != (local.context, local.scope) {
        return Ok(false);
    }
    let _nodes = budget.reserve(
        "alias-condition-replay",
        h.entry
            .condition_nodes
            .len()
            .checked_mul(2048)
            .ok_or_else(|| invalid("alias condition overflow"))?,
    )?;
    let nodes = h.entry.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let read = handoffs::named_condition(h, alias, budget)?;
    let local = Diagram::from_records(need(&h.entry.conditions, local.condition)?, &nodes)?;
    let covered = read
        .admitted_binary(&local, BooleanOperation::Conjunction, budget)
        .map_err(|e| invalid(format!("alias condition refused: {e:?}")))?;
    Ok(covered.into_parts().0.id() == read.id())
}

/// Retain a source parameter read even when no canonical formal mapping exists.
fn unmapped(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    invocation: &owner::Invocation,
    out: &mut Output,
) -> Result<(), ModelError> {
    let h = &base.handoffs;
    for attempt in h.attempts.iter() {
        let alt = need(&base.events.alternatives, attempt.alternative)?;
        let event = need(&base.events.events, alt.event)?;
        if event.context != invocation.context
            || !alt
                .entity
                .is_some_and(|e| out.scope.iter().any(|s| s.entity == e))
        {
            continue;
        }
        let Some(syntax) = attempt.syntax else {
            continue;
        };
        let site = need(&h.entry.occurrences, event.site)?;
        let Some(caller) = origin(h, site)? else {
            continue;
        };
        if !out.scope.iter().any(|s| s.entity == caller.entity) {
            continue;
        }
        for argument in h.arguments.iter().filter(|a| a.call == syntax) {
            let actual = need(&h.entry.occurrences, argument.value)?;
            if h.bindings.iter().any(|b|b.attempt==attempt.id()&&matches!(h.sources.get(b.source),Some(BindingSource::Actual{occurrence}) if h.entry.occurrences.get(*occurrence).is_some_and(|o|same(o,actual)))){continue}
            for value in h.entry.values.iter().filter(|v| {
                v.kind == FlowSinkKind::Argument
                    && h.entry
                        .qualifications
                        .get(v.qualification)
                        .is_some_and(|q| q.context == invocation.context)
                    && h.entry
                        .occurrences
                        .get(v.sink)
                        .is_some_and(|o| same(o, actual) || inside(o, actual))
            }) {
                let use_ = need(&h.entry.uses, value.use_)?;
                let place = need(&h.entry.places, use_.place)?;
                let Some(PlaceRoot::Formal { declaration }) = h.entry.roots.get(place.root) else {
                    continue;
                };
                let formal = ParameterEntity::Source {
                    declaration: *declaration,
                }
                .id();
                let assessment = d.assessments.iter().find(|a| a.value == value.id());
                out.unfollowed_arguments.insert(UnfollowedArgument {
                    frame: frame.id(),
                    attempt: attempt.id(),
                    argument: actual.id(),
                    binding: None,
                    value: value.id(),
                    caller: caller.entity,
                    formal,
                    assessment: assessment.map(Record::id),
                    reason: if matches!(
                        argument.kind,
                        ArgumentKind::Starred | ArgumentKind::DoubleStarred
                    ) {
                        obligation::ObligationKind::UnsupportedUnpacking
                    } else {
                        obligation::ObligationKind::AmbiguousBinding
                    },
                })?;
            }
        }
    }
    Ok(())
}
