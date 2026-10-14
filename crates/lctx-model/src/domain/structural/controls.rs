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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "structural_control_steps")]
pub struct ControlStep {
    #[model(key)]
    pub path: Id<ControlPath>,
    #[model(key)]
    pub ordinal: i64,
    pub flow: Id<ArgumentFlow>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
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
    prepared: &'a handoffs::Conditions<'a>,
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
    let FlowInputs {
        d,
        h,
        base,
        prepared,
    } = flow_inputs;
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
    let local_q = need(&h.local.qualifications, proof.qualification)?;
    let alias = alias
        .map(|id| need(&out.handoff_values, id).cloned())
        .transpose()?;
    let q = super::qualifications::intersect_prepared(
        base,
        q,
        local_q,
        alias.as_ref(),
        site.source,
        out,
        prepared,
        budget,
    )?;
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
    let prepared = handoffs::Conditions::new(&base.handoffs, budget)?;
    produce_prepared(d, base, frame, invocation, settings, out, &prepared, budget)
}
#[expect(
    clippy::too_many_arguments,
    reason = "Independent structural input owners and borrowed conditions retain separate contracts"
)]
pub(super) fn produce_prepared(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    invocation: &owner::Invocation,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    prepared: &handoffs::Conditions<'_>,
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
                FlowInputs {
                    d,
                    h,
                    base,
                    prepared,
                },
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
            && let Some((_, alias)) = handoffs::named_definition_prepared(
                h,
                argument,
                invocation.context,
                &prepared.entry,
                budget,
            )?
            && let handoffs::ValueSource::Named { definition, .. } = &alias
        {
            for proof in d.contributions.iter() {
                let value = need(&h.entry.values, proof.value)?;
                if value.kind == FlowSinkKind::Definition
                    && value.transfer == transfer::TransferKind::Identity
                    && proof.definition == Some(*definition)
                    && alias_covers(h, &alias, proof.qualification, prepared, budget)?
                {
                    out.handoff_values.insert(alias.clone())?;
                    flow(
                        FlowInputs {
                            d,
                            h,
                            base,
                            prepared,
                        },
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
    traverse(
        d,
        base,
        frame,
        invocation.context,
        settings,
        out,
        prepared,
        budget,
    )
}
#[derive(Clone, Copy)]
struct State {
    target: Id<EntityRef>,
    formal: Id<ParameterEntity>,
    suppressed: bool,
    path: Option<usize>,
}
#[derive(Clone, Copy)]
struct PathNode {
    parent: Option<usize>,
    flow: Id<ArgumentFlow>,
    length: usize,
}
impl HeapSize for PathNode {}
type ControlPort = (Id<EntityRef>, Id<ParameterEntity>);
struct ControlEdges {
    outgoing: std::collections::BTreeMap<ControlPort, Vec<Id<ArgumentFlow>>>,
    unfollowed: std::collections::BTreeMap<ControlPort, Vec<Id<UnfollowedArgument>>>,
    _charge: charged::StateCharge,
}
impl ControlEdges {
    fn new(out: &Output, budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut result = Self {
            outgoing: Default::default(),
            unfollowed: Default::default(),
            _charge: charged::StateCharge::new(budget, "structural-control-correspondence"),
        };
        for edge in out.argument_flows.iter() {
            build::append_index(
                &mut result.outgoing,
                &mut result._charge,
                (edge.caller, edge.source),
                edge.id(),
            )?;
        }
        for row in out.unfollowed_arguments.iter() {
            build::append_index(
                &mut result.unfollowed,
                &mut result._charge,
                (row.caller, row.formal),
                row.id(),
            )?;
        }
        Ok(result)
    }
}
fn enqueue(
    pending: &mut std::collections::VecDeque<State>,
    charge: &mut charged::StateCharge,
    state: State,
) -> Result<(), ModelError> {
    if pending.len() == pending.capacity() {
        let additional = pending.capacity().max(4);
        charge.grow(
            additional
                .checked_mul(size_of::<State>())
                .ok_or_else(|| invalid("control queue allocation overflow"))?,
        )?;
        pending.reserve_exact(additional);
    }
    pending.push_back(state);
    Ok(())
}
#[expect(
    clippy::too_many_arguments,
    reason = "The pure per-root traversal borrows immutable frame correspondence and publishes existing rows"
)]
fn traverse_root(
    frame: Id<StructuralFrame>,
    seed: Id<EntityRef>,
    formal: Id<ParameterEntity>,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    edges: &ControlEdges,
    budget: &resources::ResourceBudget,
    mut on_path: impl FnMut(&ControlPath, &mut Output) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let traversal_id = ControlTraversal {
        frame,
        seed,
        formal,
        stop: None,
        vertices: 0,
        arcs: 0,
    }
    .id();
    // Root scratch is dropped before the next root. FIFO backing and arena nodes remain
    // charged even after dequeue; published rows own independent reservations.
    let mut scratch = charged::StateCharge::new(budget, "structural-control-root");
    let mut visited = charged::ChargedSet::default();
    let mut arena: charged::ChargedVec<PathNode> = Default::default();
    let mut pending = std::collections::VecDeque::new();
    enqueue(
        &mut pending,
        &mut scratch,
        State {
            target: seed,
            formal,
            suppressed: false,
            path: None,
        },
    )?;
    let mut arcs = 0;
    let mut stop = None;
    while let Some(state) = pending.pop_front() {
        let key = (state.target, state.formal, state.suppressed);
        if visited.contains(&key) {
            continue;
        }
        if visited.len() >= settings.vertices as usize {
            stop = Some(TraversalStop::Vertices);
            break;
        }
        visited.insert(&mut scratch, key)?;
        let length = state.path.map_or(0, |node| arena[node].length);
        let path = ControlPath {
            traversal: traversal_id,
            target: state.target,
            formal: state.formal,
            may_suppress: state.suppressed,
            length: length as i64,
        };
        out.control_paths.insert(path.clone())?;
        {
            let mut witness_charge =
                charged::StateCharge::new(budget, "structural-control-witness");
            let mut witness = charged::ChargedVec::default();
            let mut cursor = state.path;
            while let Some(node) = cursor {
                witness.push(&mut witness_charge, arena[node].flow)?;
                cursor = arena[node].parent;
            }
            for (ordinal, flow) in witness.iter().rev().enumerate() {
                out.control_steps.insert(ControlStep {
                    path: path.id(),
                    ordinal: ordinal as i64,
                    flow: *flow,
                })?;
            }
        }
        for argument in edges
            .unfollowed
            .get(&(state.target, state.formal))
            .into_iter()
            .flatten()
        {
            out.unfollowed_paths.insert(UnfollowedPath {
                path: path.id(),
                argument: *argument,
            })?;
        }
        if !state.suppressed {
            on_path(&path, out)?;
        }
        for edge in edges
            .outgoing
            .get(&(state.target, state.formal))
            .into_iter()
            .flatten()
        {
            if length >= settings.depth as usize {
                stop = Some(TraversalStop::Depth);
                continue;
            }
            if arcs >= settings.arcs {
                stop = Some(TraversalStop::Arcs);
                break;
            }
            arcs += 1;
            let edge = need(&out.argument_flows, *edge)?;
            let next = State {
                target: edge.callee,
                formal: edge.formal,
                suppressed: state.suppressed || edge.may_catch || edge.value_tested,
                path: Some(arena.len()),
            };
            arena.push(
                &mut scratch,
                PathNode {
                    parent: state.path,
                    flow: edge.id(),
                    length: length + 1,
                },
            )?;
            enqueue(&mut pending, &mut scratch, next)?;
        }
        if stop == Some(TraversalStop::Arcs) {
            break;
        }
    }
    out.control_traversals.insert(ControlTraversal {
        frame,
        seed,
        formal,
        stop,
        vertices: visited.len() as i64,
        arcs,
    })?;
    Ok(())
}
#[expect(
    clippy::too_many_arguments,
    reason = "Independent structural input owners and borrowed conditions retain separate contracts"
)]
fn traverse(
    d: &Data,
    base: &build::Data,
    frame: &StructuralFrame,
    context: Id<attribution::AnalysisContext>,
    settings: &AnalyticsConfiguration,
    out: &mut Output,
    prepared: &handoffs::Conditions<'_>,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = charged::StateCharge::new(budget, "structural-control-roots");
    let mut roots = charged::ChargedSet::default();
    let edges = ControlEdges::new(out, budget)?;
    let mut parameter_charge =
        charged::StateCharge::new(budget, "structural-control-root-parameters");
    let mut root_parameters = None;
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
        if root_parameters.is_none() {
            // Preserve the original lazy premise validation: no source callable means no
            // parameter walk. The complete walk is shared once there is a consumer.
            let mut declaration_charge =
                charged::StateCharge::new(budget, "structural-control-root-declarations");
            let mut declarations = std::collections::BTreeMap::new();
            for row in base.handoffs.entry.symbol_declarations.iter() {
                build::append_index(
                    &mut declarations,
                    &mut declaration_charge,
                    row.symbol,
                    row.declaration,
                )?;
            }
            let mut parameters = std::collections::BTreeMap::new();
            for link in base.handoffs.entry.links.iter() {
                let parameter = need(&base.handoffs.entry.parameters, link.parameter)?;
                let signature = need(&base.handoffs.entry.signatures, parameter.signature)?;
                for declaration in declarations.get(&signature.symbol).into_iter().flatten() {
                    build::append_index(
                        &mut parameters,
                        &mut parameter_charge,
                        *declaration,
                        link.entity,
                    )?;
                }
            }
            root_parameters = Some(parameters);
        }
        for formal in root_parameters
            .as_ref()
            .expect("prepared for source callable")
            .get(declaration)
            .into_iter()
            .flatten()
        {
            roots.insert(&mut charge, (candidate.entity, *formal))?;
        }
    }
    drop(root_parameters);
    drop(parameter_charge);
    for (seed, formal) in roots.iter() {
        traverse_root(
            frame.id(),
            *seed,
            *formal,
            settings,
            out,
            &edges,
            budget,
            |path, out| raises(d, &base.handoffs, path, context, out, prepared, budget),
        )?;
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
    prepared: &handoffs::Conditions<'_>,
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
                    let condition = prepared
                        .entry
                        .diagram(need(&h.entry.conditions, q.condition)?)?;
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
    prepared: &handoffs::Conditions<'_>,
    budget: &resources::ResourceBudget,
) -> Result<bool, ModelError> {
    let handoffs::ValueSource::Named { observation, .. } = alias else {
        return Ok(false);
    };
    let q = need(
        &h.entry.qualifications,
        need(&h.entry.use_observations, *observation)?.qualification,
    )?;
    let local = need(&h.local.qualifications, local)?;
    if (q.context, q.scope) != (local.context, local.scope) {
        return Ok(false);
    }
    let read = handoffs::named_condition_prepared(h, alias, &prepared.entry, budget)?;
    let local = prepared
        .local
        .diagram(need(&h.local.conditions, local.condition)?)?;
    let covered = read
        .admitted_binary(&local, BooleanOperation::Conjunction, budget)
        .map_err(|e| match e {
            conditions::DiagramAdmissionError::Resource(error) => error,
            conditions::DiagramAdmissionError::Boundary(boundary) => {
                invalid(format!("alias condition refused: {boundary:?}"))
            }
        })?;
    Ok(covered.id() == read.id())
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

#[cfg(test)]
mod traversal_controls {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn settings() -> AnalyticsConfiguration {
        AnalyticsConfiguration {
            module_prefixes: vec!["api".into()],
            public_roots: vec!["api".into()],
            configured_seeds: vec![],
            depth: 20,
            vertices: 100,
            arcs: 100,
            witnesses: 2,
            brief_budget: 3,
            communities: false,
            pagerank: false,
            fca: false,
            rca: false,
            knn: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        }
    }
    fn edge(
        out: &mut Output,
        n: u8,
        caller: u8,
        source: u8,
        callee: u8,
        formal: u8,
        suppressed: bool,
    ) -> Id<ArgumentFlow> {
        out.argument_flows
            .insert(ArgumentFlow {
                frame: id(40),
                binding: id(n),
                contribution: id(n),
                alias: None,
                caller: id(caller),
                callee: id(callee),
                source: id(source),
                formal: id(formal),
                phase: CallPhase::Call,
                modality: Modality::Definite,
                qualification: id(50),
                conditional: false,
                may_catch: suppressed,
                value_tested: false,
            })
            .unwrap()
    }
    fn graph(
        b: &resources::ResourceBudget,
    ) -> (Output, [Id<ArgumentFlow>; 8], Id<UnfollowedArgument>) {
        let mut out = Output::new(b);
        let flows = [
            edge(&mut out, 10, 0, 1, 1, 1, false),
            edge(&mut out, 11, 0, 1, 1, 1, false),
            edge(&mut out, 12, 0, 1, 2, 2, true),
            edge(&mut out, 13, 1, 1, 3, 3, false),
            edge(&mut out, 14, 2, 2, 3, 3, false),
            edge(&mut out, 15, 3, 3, 0, 1, false),
            edge(&mut out, 16, 1, 1, 3, 4, false),
            edge(&mut out, 17, 3, 4, 3, 4, false),
        ];
        let unfollowed = out
            .unfollowed_arguments
            .insert(UnfollowedArgument {
                frame: id(40),
                attempt: id(30),
                argument: id(31),
                binding: None,
                value: id(32),
                caller: id(3),
                formal: id(3),
                assessment: None,
                reason: obligation::ObligationKind::AmbiguousBinding,
            })
            .unwrap();
        (out, flows, unfollowed)
    }
    fn steps(out: &Output, path: &ControlPath) -> Vec<Id<ArgumentFlow>> {
        let mut steps = out
            .control_steps
            .iter()
            .filter(|s| s.path == path.id())
            .collect::<Vec<_>>();
        steps.sort_by_key(|s| s.ordinal);
        steps.into_iter().map(|s| s.flow).collect()
    }
    #[test]
    fn fifo_arena_preserves_parallel_diamond_cycle_formals_and_suppression() {
        let b = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let (mut out, flow, unfollowed) = graph(&b);
        let edges = ControlEdges::new(&out, &b).unwrap();
        let mut raise_paths = Vec::new();
        traverse_root(
            id(40),
            id(0),
            id(1),
            &settings(),
            &mut out,
            &edges,
            &b,
            |path, _| {
                raise_paths.push((path.target, path.formal));
                Ok(())
            },
        )
        .unwrap();
        let traversal = out.control_traversals.iter().next().unwrap();
        assert_eq!(
            (traversal.stop, traversal.vertices, traversal.arcs),
            (None, 9, 15)
        );
        let path = |target, formal, suppressed| {
            out.control_paths
                .iter()
                .find(|p| {
                    (p.target, p.formal, p.may_suppress) == (id(target), id(formal), suppressed)
                })
                .unwrap()
        };
        let first = flow[0].min(flow[1]);
        assert_eq!(steps(&out, path(1, 1, false)), vec![first]);
        assert_eq!(steps(&out, path(3, 3, false)), vec![first, flow[3]]);
        assert_eq!(steps(&out, path(3, 3, true)), vec![flow[2], flow[4]]);
        assert_eq!(steps(&out, path(3, 4, false)), vec![first, flow[6]]);
        assert_eq!(steps(&out, path(0, 1, false)), vec![]);
        assert!(path(0, 1, true).length > 0);
        assert_eq!(raise_paths.len(), 4);
        for suppressed in [false, true] {
            assert!(
                out.unfollowed_paths
                    .iter()
                    .any(|p| p.path == path(3, 3, suppressed).id() && p.argument == unfollowed)
            );
        }
        drop(edges);
        drop(out);
        assert_eq!(b.reserved(), 0);
    }
    #[test]
    fn fifo_arena_keeps_limit_stop_order_and_drops_failed_root_scratch() {
        for (depth, vertices, arcs, expected, count) in [
            (20, 100, 2, TraversalStop::Arcs, 1),
            (1, 100, 100, TraversalStop::Depth, 3),
            (20, 2, 100, TraversalStop::Vertices, 2),
        ] {
            let b = resources::ResourceBudget::fixed(1 << 20).unwrap();
            let (mut out, _, _) = graph(&b);
            let edges = ControlEdges::new(&out, &b).unwrap();
            let mut settings = settings();
            settings.depth = depth;
            settings.vertices = vertices;
            settings.arcs = arcs;
            traverse_root(
                id(40),
                id(0),
                id(1),
                &settings,
                &mut out,
                &edges,
                &b,
                |_, _| Ok(()),
            )
            .unwrap();
            let traversal = out.control_traversals.iter().next().unwrap();
            assert_eq!(traversal.stop, Some(expected));
            assert_eq!(traversal.vertices, count);
            assert_eq!(out.control_paths.len() as i64, count);
            if expected == TraversalStop::Arcs {
                assert_eq!(traversal.arcs, 2);
            }
            drop(edges);
            drop(out);
            assert_eq!(b.reserved(), 0);
        }
        let b = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let (mut out, _, _) = graph(&b);
        let edges = ControlEdges::new(&out, &b).unwrap();
        let before = b.reserved();
        let result = traverse_root(
            id(40),
            id(0),
            id(1),
            &settings(),
            &mut out,
            &edges,
            &b,
            |_, _| Err(invalid("deliberate path-publication refusal")),
        );
        assert!(result.is_err());
        assert!(out.control_traversals.is_empty());
        assert_eq!(out.control_paths.len(), 1);
        // Published path rows remain charged; dropping the actual output releases them.
        assert!(b.reserved() > before);
        drop(out);
        drop(edges);
        assert_eq!(b.reserved(), 0);
    }
    #[derive(Debug)]
    struct RootPool {
        all: resources::ResourceBudget,
        scratch: resources::ResourceBudget,
        witness: resources::ResourceBudget,
    }
    impl resources::ResourcePool for RootPool {
        fn reserve(
            &self,
            owner: &'static str,
            bytes: usize,
        ) -> Result<Box<dyn resources::Reservation>, ModelError> {
            if owner == "structural-control-root" {
                self.scratch.reserve(owner, bytes)
            } else if owner == "structural-control-witness" {
                self.witness.reserve(owner, bytes)
            } else {
                self.all.reserve(owner, bytes)
            }
        }
        fn reserved(&self) -> usize {
            self.all.reserved() + self.scratch.reserved() + self.witness.reserved()
        }
        fn limit(&self) -> usize {
            self.all.limit() + self.scratch.limit() + self.witness.limit()
        }
    }
    #[test]
    fn fifo_arena_releases_each_root_and_refused_allocation() {
        let scratch = resources::ResourceBudget::fixed(16 << 10).unwrap();
        let b = resources::ResourceBudget::from_pool(std::sync::Arc::new(RootPool {
            all: resources::ResourceBudget::fixed(1 << 20).unwrap(),
            scratch: scratch.clone(),
            witness: resources::ResourceBudget::fixed(16 << 10).unwrap(),
        }))
        .unwrap();
        let (mut out, _, _) = graph(&b);
        let edges = ControlEdges::new(&out, &b).unwrap();
        for seed in [0, 1] {
            traverse_root(
                id(40),
                id(seed),
                id(1),
                &settings(),
                &mut out,
                &edges,
                &b,
                |_, _| Ok(()),
            )
            .unwrap();
            assert_eq!(
                scratch.reserved(),
                0,
                "root scratch must not accumulate across roots"
            );
        }
        assert_eq!(out.control_traversals.len(), 2);
        assert!(
            b.reserved() > 0,
            "frame correspondence and flat outputs remain live"
        );
        drop(out);
        drop(edges);
        assert_eq!(b.reserved(), 0);
        let tiny = resources::ResourceBudget::fixed(1).unwrap();
        let mut output = Output::new(&tiny);
        let edges = ControlEdges::new(&output, &tiny).unwrap();
        assert!(
            traverse_root(
                id(40),
                id(0),
                id(1),
                &settings(),
                &mut output,
                &edges,
                &tiny,
                |_, _| Ok(())
            )
            .is_err()
        );
        assert_eq!(tiny.reserved(), 0);
        let scratch = resources::ResourceBudget::fixed(16 << 10).unwrap();
        let witness = resources::ResourceBudget::fixed(1).unwrap();
        let b = resources::ResourceBudget::from_pool(std::sync::Arc::new(RootPool {
            all: resources::ResourceBudget::fixed(1 << 20).unwrap(),
            scratch: scratch.clone(),
            witness: witness.clone(),
        }))
        .unwrap();
        let (mut out, _, _) = graph(&b);
        let edges = ControlEdges::new(&out, &b).unwrap();
        assert!(
            traverse_root(
                id(40),
                id(0),
                id(1),
                &settings(),
                &mut out,
                &edges,
                &b,
                |_, _| Ok(())
            )
            .is_err()
        );
        assert_eq!(scratch.reserved(), 0);
        assert_eq!(witness.reserved(), 0);
        assert!(
            !out.control_paths.is_empty(),
            "admitted flat output remains owned on later refusal"
        );
        drop(out);
        drop(edges);
        assert_eq!(b.reserved(), 0);
    }
}
