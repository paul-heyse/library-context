//! Structural FCA attributes and optional layers use exact native/normalized inputs.
use super::{
    build::{Data, invalid, need},
    *,
};
use crate::domain::{normalized::entities::*, resources::ResourceBudget};
use std::collections::{BTreeMap, BTreeSet};
fn declaration(
    d: &Data,
    entity: Id<EntityRef>,
) -> Result<Option<Id<source::Occurrence>>, ModelError> {
    let EntityRef::Callable { callable } = need(&d.native.refs, entity)? else {
        return Ok(None);
    };
    Ok(match need(&d.native.callables, *callable)? {
        CallableEntity::Source { declaration, .. } => Some(*declaration),
        _ => None,
    })
}
fn context(
    d: &Data,
    q: Id<assertion::AssertionQualification>,
    ctx: Id<attribution::AnalysisContext>,
) -> Result<bool, ModelError> {
    Ok(need(&d.native.qualifications, q)?.context == ctx)
}
fn receiver(
    d: &Data,
    entity: Id<EntityRef>,
    p: &normalized::parameter_correspondence::SourceParameterCorrespondence,
    ctx: Id<attribution::AnalysisContext>,
) -> bool {
    let Some(EntityRef::Callable { callable }) = d.native.refs.get(entity) else {
        return false;
    };
    normalized::parameter_correspondence::is_bound_receiver(&d.native, p, *callable, ctx)
}
/// Unknown/truncated/recursive aliases are excluded explicitly. The traversal follows modeled
/// structure, never type rendering, with bounded native lists and per-branch cycle detection.
fn types(
    d: &Data,
    term: Id<types::TypeTerm>,
    ctx: Id<attribution::AnalysisContext>,
    classes: &mut BTreeSet<Id<EntityRef>>,
    path: &mut BTreeSet<Id<types::TypeTerm>>,
    work: &mut usize,
) -> Result<bool, ModelError> {
    *work += 1;
    if *work > 32768 || path.len() >= 256 {
        return Ok(false);
    }
    if !path.insert(term) {
        return Ok(false);
    }
    let mut children = vec![];
    let mut lists = vec![];
    let mut symbol = None;
    let mut parameters = None;
    let known = match need(&d.native.terms, term)? {
        types::TypeTerm::ClassInstance { class, arguments }
        | types::TypeTerm::SelfType { class, arguments }
        | types::TypeTerm::TypedDict {
            class, arguments, ..
        } => {
            symbol = Some(*class);
            lists.push(*arguments);
            true
        }
        types::TypeTerm::ClassObject { class } | types::TypeTerm::EnumLiteral { class, .. } => {
            symbol = Some(*class);
            true
        }
        types::TypeTerm::Union { members }
        | types::TypeTerm::Intersection { members }
        | types::TypeTerm::Overload {
            signatures: members,
            ..
        }
        | types::TypeTerm::Tuple { elements: members } => {
            lists.push(*members);
            true
        }
        types::TypeTerm::TypeOf { target }
        | types::TypeTerm::Annotated { target }
        | types::TypeTerm::Unpack { target }
        | types::TypeTerm::TypeGuard { target, .. }
        | types::TypeTerm::TypeForm { target } => {
            children.push(*target);
            true
        }
        types::TypeTerm::TypeAlias {
            target, untyped, ..
        } => {
            children.push(*target);
            !*untyped
        }
        types::TypeTerm::BoundMethod { receiver, function } => {
            children.extend([*receiver, *function]);
            true
        }
        types::TypeTerm::Generic { parameters, body } => {
            lists.push(*parameters);
            children.push(*body);
            true
        }
        types::TypeTerm::Callable {
            parameters: p,
            returns,
            param_spec,
            ..
        } => {
            parameters = Some(*p);
            children.push(*returns);
            children.extend(*param_spec);
            true
        }
        types::TypeTerm::ParamList {
            parameters: p,
            param_spec,
        } => {
            parameters = Some(*p);
            children.extend(*param_spec);
            true
        }
        types::TypeTerm::Literal { .. }
        | types::TypeTerm::None
        | types::TypeTerm::Never { .. }
        | types::TypeTerm::LiteralString
        | types::TypeTerm::SpecialForm { .. }
        | types::TypeTerm::Module { .. }
        | types::TypeTerm::TypeVar { .. }
        | types::TypeTerm::ParamSpec { .. }
        | types::TypeTerm::TypeVarTuple { .. }
        | types::TypeTerm::VariableForm { .. }
        | types::TypeTerm::Any {
            flavor: types::AnyFlavor::Explicit,
        } => true,
        _ => false,
    };
    if let Some(symbol) = symbol {
        for r in d.native.symbol_resolutions.iter().filter(|r| {
            r.symbol == symbol && r.context == ctx && r.status == ResolutionStatus::Resolved
        }) {
            if let Some(entity) = r.entity
                && matches!(d.native.refs.get(entity), Some(EntityRef::Class { .. }))
            {
                classes.insert(entity);
            }
        }
    }
    for sequence in lists {
        need(&d.type_sequences, sequence)?;
        for m in d.type_members.iter().filter(|m| m.sequence == sequence) {
            children.push(m.child);
        }
    }
    if let Some(list) = parameters {
        for p in d.callable_type_slots.iter().filter(|p| p.list == list) {
            children.push(p.term);
        }
    }
    let mut complete = known;
    for child in children {
        complete &= types(d, child, ctx, classes, path, work)?;
    }
    path.remove(&term);
    Ok(complete)
}
pub(super) fn classes(
    d: &Data,
    term: Id<types::TypeTerm>,
    ctx: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Option<BTreeSet<Id<EntityRef>>>, ModelError> {
    let _r = b.reserve(
        "analytic-type-structure",
        d.native
            .terms
            .len()
            .checked_mul(512)
            .and_then(|n| n.checked_add(32768 * 128))
            .ok_or_else(|| invalid("analytic type structure overflow"))?,
    )?;
    let mut out = BTreeSet::new();
    if types(d, term, ctx, &mut out, &mut BTreeSet::new(), &mut 0)? {
        Ok(Some(out))
    } else {
        Ok(None)
    }
}
pub fn type_layer(
    d: &Data,
    f: &AnalyticFrame,
    scope: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let sf = need(&d.structural.frames, f.structural)?;
    let parent = need(&d.structural_invocations, sf.invocation)?;
    let mut members =
        BTreeMap::<Id<EntityRef>, BTreeMap<Id<EntityRef>, Id<types::TypeObservation>>>::new();
    let mut charge = charged::StateCharge::new(b, "analytic-type-layer");
    for entity in scope {
        let Some(declaration) = declaration(d, *entity)? else {
            continue;
        };
        for p in d
            .parameter_syntax
            .iter()
            .filter(|p| p.function == declaration)
        {
            let Some(correspondence) = normalized::parameter_correspondence::source_parameter(
                &d.native,
                p,
                parent.context,
                b,
            )?
            else {
                continue;
            };
            if receiver(d, *entity, &correspondence, parent.context) {
                continue;
            }
            for observation in d.native.type_observations.iter().filter(|o| {
                o.subject == correspondence.formal
                    && o.role == types::TypeRole::Parameter
                    && o.declared
            }) {
                if !context(d, observation.qualification, parent.context)? {
                    continue;
                }
                let Some(classes) = classes(d, observation.term, parent.context, b)? else {
                    continue;
                };
                for class in classes {
                    let EntityRef::Class { class: class_id } = need(&d.native.refs, class)? else {
                        continue;
                    };
                    let ClassEntity::Source { declaration } =
                        need(&d.native.entity_classes, *class_id)?
                    else {
                        continue;
                    };
                    let source = need(&d.native.occurrences, *declaration)?.source;
                    let input = need(&d.native.artifacts, source)?.input;
                    if input != parent.input
                        || !d.uses.iter().any(|r| {
                            r.input == parent.input
                                && r.artifact == source
                                && r.role == input::SourceRole::Release
                        })
                    {
                        continue;
                    }
                    charge.grow(256)?;
                    members
                        .entry(class)
                        .or_default()
                        .entry(*entity)
                        .or_insert(observation.id());
                }
            }
        }
    }
    for (class, rows) in members {
        let rows = rows.into_iter().collect::<Vec<_>>();
        if rows
            .len()
            .checked_mul(rows.len())
            .is_none_or(|n| n > build::MAX_WORK as usize)
        {
            return Err(invalid("analytic type layer pair bound"));
        }
        for (i, (left, l)) in rows.iter().enumerate() {
            for (right, r) in &rows[i + 1..] {
                build::add_pair(
                    out,
                    f.id(),
                    Layer::Type,
                    *left,
                    *right,
                    PairSource::Type {
                        left: *l,
                        right: *r,
                        class,
                    },
                )?;
            }
        }
    }
    Ok(())
}
pub fn mention_layer(
    d: &Data,
    f: &AnalyticFrame,
    scope: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let sf = need(&d.structural.frames, f.structural)?;
    let parent = need(&d.structural_invocations, sf.invocation)?;
    let mut charge = charged::StateCharge::new(b, "analytic-mention-layer");
    let mut passages = BTreeMap::<
        Id<documents::DocumentNode>,
        BTreeMap<Id<EntityRef>, Id<documents::DocumentMentionObservation>>,
    >::new();
    for a in d
        .native
        .relation_mention_entity_assessments
        .iter()
        .filter(|a| a.status == ResolutionStatus::Resolved)
    {
        let mention = need(&d.native.mentions, a.observation)?;
        if mention.class != documents::MentionClass::Exact {
            continue;
        }
        let q = need(&d.native.qualifications, mention.qualification)?;
        let artifact = match need(&d.native.scopes, q.scope)? {
            source::CoverageScope::Artifact { artifact } => Some(*artifact),
            source::CoverageScope::Module { module } => {
                Some(need(&d.native.modules, *module)?.source)
            }
            _ => None,
        };
        let Some(artifact) = artifact else {
            continue;
        };
        let document_input = need(&d.native.artifacts, artifact)?.input;
        let admitted_input = if document_input == parent.input {
            q.context == parent.context
        } else {
            d.corpus_libraries.iter().any(|link| {
                link.corpus == document_input && link.library == parent.input
            })
        };
        // A corpus link admits documentary correspondence, not native context equality.
        // The candidate resolution below must still belong to this exact analytic context.
        if !admitted_input || !d.uses.iter().any(|u| {
            u.input == document_input && u.artifact == artifact
                && u.role == input::SourceRole::Document
        }) {
            continue;
        }
        for c in d
            .native
            .relation_mention_entity_candidates
            .iter()
            .filter(|c| c.assessment == a.id())
        {
            for ec in d
                .native
                .entity_exposure_candidates
                .iter()
                .filter(|ec| ec.exposure == c.exposure)
            {
                let resolution = need(&d.native.symbol_resolutions, ec.resolution)?;
                if resolution.context != parent.context
                    || resolution.status != ResolutionStatus::Resolved
                {
                    continue;
                }
                let Some(entity) = resolution.entity else {
                    continue;
                };
                if !scope.contains(&entity) {
                    continue;
                }
                charge.grow(256)?;
                passages
                    .entry(mention.passage.id())
                    .or_default()
                    .entry(entity)
                    .or_insert(mention.id());
            }
        }
    }
    for rows in passages.into_values() {
        let rows = rows.into_iter().collect::<Vec<_>>();
        if rows
            .len()
            .checked_mul(rows.len())
            .is_none_or(|n| n > build::MAX_WORK as usize)
        {
            return Err(invalid("analytic mention layer pair bound"));
        }
        for (i, (left, l)) in rows.iter().enumerate() {
            for (right, r) in &rows[i + 1..] {
                build::add_pair(
                    out,
                    f.id(),
                    Layer::Mention,
                    *left,
                    *right,
                    PairSource::Mention {
                        left: *l,
                        right: *r,
                    },
                )?;
            }
        }
    }
    Ok(())
}
pub(super) struct Fact {
    pub entity: Id<EntityRef>,
    pub attribute: Attribute,
    pub source: IncidenceSource,
}
impl HeapSize for Fact {
    fn heap_bytes(&self) -> usize {
        self.attribute
            .heap_bytes()
            .saturating_add(self.source.heap_bytes())
    }
}
fn facts(
    d: &Data,
    objects: &BTreeSet<Id<EntityRef>>,
    ctx: Id<attribution::AnalysisContext>,
    rca: bool,
    out: &Output,
    b: &ResourceBudget,
    policy: policy::AttributePolicy,
) -> Result<
    (
        charged::ChargedVec<Fact>,
        charged::ChargedVec<super::native_attributes::Selection>,
        charged::StateCharge,
    ),
    ModelError,
> {
    let mut facts = charged::ChargedVec::default();
    let mut selections = charged::ChargedVec::default();
    let mut charge = charged::StateCharge::new(b, "analytic-concept-incidence");
    for entity in objects {
        let Some(declaration) = declaration(d, *entity)? else {
            continue;
        };
        super::native_attributes::variants(d, *entity, ctx, b, &mut facts, &mut charge, policy)?;
        super::native_attributes::dependence(d, *entity, ctx, &mut facts, &mut charge)?;
        for p in d
            .parameter_syntax
            .iter()
            .filter(|p| p.function == declaration && policy.signature_roles.source())
        {
            let Some(correspondence) =
                normalized::parameter_correspondence::source_parameter(&d.native, p, ctx, b)?
            else {
                continue;
            };
            if policy.receiver == policy::ReceiverPolicy::ExcludeSourceBoundPreserveNative
                && receiver(d, *entity, &correspondence, ctx)
            {
                continue;
            }
            let shapes = d
                .native
                .parameters
                .iter()
                .filter(|slot| correspondence.parameters.contains(&slot.id()))
                .filter_map(|p| d.native.shapes.get(p.shape));
            for shape in shapes {
                if let Some(name) = &shape.name {
                    facts.push(
                        &mut charge,
                        Fact {
                            entity: *entity,
                            attribute: Attribute::Parameter {
                                name: name.clone(),
                                kind: shape.kind,
                            },
                            source: IncidenceSource::Parameter {
                                observation: p.id(),
                            },
                        },
                    )?;
                }
            }
            for o in d.native.type_observations.iter().filter(|o| {
                o.subject == correspondence.formal
                    && o.role == types::TypeRole::Parameter
                    && o.declared
            }) {
                if context(d, o.qualification, ctx)? && classes(d, o.term, ctx, b)?.is_some() {
                    facts.push(
                        &mut charge,
                        Fact {
                            entity: *entity,
                            attribute: Attribute::ParameterType { term: o.term },
                            source: IncidenceSource::Type {
                                observation: o.id(),
                            },
                        },
                    )?;
                    super::native_attributes::metadata(
                        d,
                        *entity,
                        o,
                        TypePortRole::Parameter,
                        ctx,
                        b,
                        &mut facts,
                        &mut selections,
                        &mut charge,
                    )?;
                }
            }
        }
        for o in d.native.type_observations.iter() {
            if !context(d, o.qualification, ctx)? {
                continue;
            }
            let attribute = if policy.signature_roles.source()
                && o.subject == declaration
                && o.role == types::TypeRole::Return
                && o.declared
                && classes(d, o.term, ctx, b)?.is_some()
            {
                Some(Attribute::Returns { term: o.term })
            } else if o.role == types::TypeRole::Raised
                && d.native
                    .owners
                    .iter()
                    .any(|owner| owner.occurrence == o.subject && owner.entity == *entity)
            {
                let values = classes(d, o.term, ctx, b)?;
                values.and_then(|c| {
                    if c.len() == 1 {
                        c.into_iter()
                            .next()
                            .map(|class| Attribute::Raises { class })
                    } else {
                        None
                    }
                })
            } else {
                None
            };
            if let Some(attribute) = attribute {
                let returns = matches!(&attribute, Attribute::Returns { .. });
                facts.push(
                    &mut charge,
                    Fact {
                        entity: *entity,
                        attribute,
                        source: IncidenceSource::Type {
                            observation: o.id(),
                        },
                    },
                )?;
                if returns {
                    super::native_attributes::metadata(
                        d,
                        *entity,
                        o,
                        TypePortRole::Return,
                        ctx,
                        b,
                        &mut facts,
                        &mut selections,
                        &mut charge,
                    )?;
                }
            }
        }
        for dec in d
            .native
            .decorators
            .iter()
            .filter(|dec| dec.declaration == declaration)
        {
            if !context(d, dec.qualification, ctx)? {
                continue;
            }
            super::native_attributes::decorator(
                d,
                *entity,
                dec,
                ctx,
                &mut facts,
                &mut selections,
                &mut charge,
            )?;
        }
    }
    if rca {
        for arc in out.arcs.iter().filter(|a| {
            objects.contains(&a.source)
                && a.source != a.target
                && d.structural.scope.iter().any(|m| {
                    m.frame == need(&out.frames, a.frame).unwrap().structural
                        && m.entity == a.target
                })
        }) {
            let alternative = need(&d.native.event_alternatives, arc.alternative)?;
            facts.push(
                &mut charge,
                Fact {
                    entity: arc.source,
                    attribute: Attribute::Calls {
                        target: arc.target,
                        modality: need(
                            &d.native.qualifications,
                            need(
                                &d.native.targets,
                                need(&d.native.event_alternative_sources, alternative.source)?
                                    .target(),
                            )?
                            .qualification,
                        )?
                        .modality,
                        phase: need(
                            &d.native.targets,
                            need(&d.native.event_alternative_sources, alternative.source)?.target(),
                        )?
                        .phase,
                    },
                    source: IncidenceSource::Call { arc: arc.id() },
                },
            )?;
        }
        for h in d
            .structural
            .handoffs
            .iter()
            .filter(|h| out.frames.iter().any(|f| f.structural == h.frame))
        {
            let producer = need(&d.native.event_alternatives, h.producer)?;
            let consumer = need(&d.native.event_alternatives, h.consumer)?;
            let (Some(source), Some(target)) = (producer.entity, consumer.entity) else {
                return Err(invalid("retained handoff endpoints absent"));
            };
            if source == target {
                continue;
            }
            let p = need(
                &d.native.targets,
                need(&d.native.event_alternative_sources, producer.source)?.target(),
            )?;
            let c = need(
                &d.native.targets,
                need(&d.native.event_alternative_sources, consumer.source)?.target(),
            )?;
            for (entity, target, takes) in [(source, target, false), (target, source, true)] {
                if objects.contains(&entity) {
                    facts.push(
                        &mut charge,
                        Fact {
                            entity,
                            attribute: Attribute::Handoff {
                                takes,
                                target,
                                producer_modality: need(&d.native.qualifications, p.qualification)?
                                    .modality,
                                consumer_modality: need(&d.native.qualifications, c.qualification)?
                                    .modality,
                                producer_phase: p.phase,
                                consumer_phase: c.phase,
                            },
                            source: IncidenceSource::Handoff { occurrence: h.id() },
                        },
                    )?;
                }
            }
        }
    }
    Ok((facts, selections, charge))
}
pub fn concepts(
    d: &Data,
    sf: &structural::StructuralFrame,
    public: &BTreeSet<Id<EntityRef>>,
    result: &mut TechniqueResult,
    out: &mut Output,
    b: &ResourceBudget,
    policy: policy::AttributePolicy,
) -> Result<(), ModelError> {
    let parent = need(&d.structural_invocations, sf.invocation)?;
    let mut charge = charged::StateCharge::new(b, "analytic-concept-scopes");
    let mut groups =
        BTreeMap::<(Id<source::Module>, Vec<String>), Vec<&structural::PublicCandidate>>::new();
    for c in d
        .structural
        .public
        .iter()
        .filter(|c| c.frame == sf.id() && public.contains(&c.entity))
    {
        let member = need(&d.members, c.member)?;
        let mut namespace = member.path.clone();
        namespace.pop();
        charge.grow(512 + namespace.heap_bytes())?;
        groups
            .entry((member.access, namespace))
            .or_default()
            .push(c);
    }
    if groups.is_empty() {
        result.stop = Stop::EmptyDomain;
    }
    for ((access, namespace), candidates) in groups {
        let objects = candidates.iter().map(|c| c.entity).collect::<BTreeSet<_>>();
        let (facts, selections, _charge) = facts(
            d,
            &objects,
            parent.context,
            result.method == analysis::AnalysisMethod::RelationalConcepts,
            out,
            b,
            policy,
        )?;
        let _context = b.reserve(
            "analytic-concept-conversion",
            facts
                .len()
                .checked_mul(768)
                .ok_or_else(|| invalid("concept conversion overflow"))?,
        )?;
        let mut attributes = BTreeSet::new();
        let mut incidences = BTreeSet::new();
        for fact in facts.iter() {
            attributes.insert(fact.attribute.id());
            incidences.insert((fact.entity, fact.attribute.id()));
        }
        let object_list = objects.iter().copied().collect::<Vec<_>>();
        let attribute_list = attributes.into_iter().collect::<Vec<_>>();
        let incidence_list = incidences.into_iter().collect::<Vec<_>>();
        let context =
            super::concepts::Context::new(&object_list, &attribute_list, &incidence_list, b)?;
        let lattice = context.analyse(
            super::policy::RETAINED.concept_support,
            super::policy::RETAINED.concept_bound,
        )?;
        let scope = ConceptScope {
            result: result.id(),
            access,
            namespace,
            examined: lattice.examined() as i64,
            partial: lattice.budget_reached(),
        };
        for selection in selections.iter() {
            selection.emit(scope.id(), out)?;
        }
        for candidate in candidates {
            out.objects.insert(ConceptObject {
                scope: scope.id(),
                entity: candidate.entity,
                candidate: candidate.id(),
            })?;
        }
        for fact in facts.iter() {
            out.attributes.insert(fact.attribute.clone())?;
            out.incidence_sources.insert(fact.source.clone())?;
            out.incidences.insert(Incidence {
                scope: scope.id(),
                entity: fact.entity,
                attribute: fact.attribute.id(),
                source: fact.source.id(),
            })?;
        }
        for c in lattice.concepts() {
            let row = Concept {
                scope: scope.id(),
                extent: build::digest("analytic-extent", &c.extent),
                intent: build::digest("analytic-intent", &c.intent),
            };
            for entity in &c.extent {
                out.extents.insert(ConceptExtent {
                    concept: row.id(),
                    entity: *entity,
                })?;
            }
            for attribute in &c.intent {
                out.intents.insert(ConceptIntent {
                    concept: row.id(),
                    attribute: *attribute,
                })?;
            }
            out.concepts.insert(row)?;
        }
        for i in lattice.implications() {
            let row = Implication {
                scope: scope.id(),
                premise: build::digest("analytic-premise", &i.premise),
                conclusion: build::digest("analytic-conclusion", &i.conclusion),
                support: i.support as i64,
            };
            for attribute in &i.premise {
                out.implication_members.insert(ImplicationMember {
                    implication: row.id(),
                    premise: true,
                    attribute: *attribute,
                })?;
            }
            for attribute in &i.conclusion {
                out.implication_members.insert(ImplicationMember {
                    implication: row.id(),
                    premise: false,
                    attribute: *attribute,
                })?;
            }
            out.implications.insert(row)?;
        }
        result.examined += scope.examined;
        if scope.partial {
            result.status = analysis::AnalysisStatus::Partial;
            result.stop = Stop::EnumerationLimit;
        }
        out.scopes.insert(scope)?;
    }
    Ok(())
}
