//! Public paths derive from normalized identity plus declaration-parent/binding evidence.
use super::{
    build::{CatalogOutput, CatalogPreparation},
    *,
};
use crate::domain::{
    charged::{ChargedMap, ChargedSet, StateCharge},
    lexical::{BindingEventKind, LexicalScopeKind},
    resources::ResourceBudget,
    symbols::{AncestryRelation, Linearization},
    syntax::DeclarationObservation,
};
const MAX_DEPTH: usize = 32;
/// A class's own entry or one MRO ancestor: its rank, class and ancestry member.
type ChainKey = (i64, Id<ClassEntity>, Option<Id<AncestryEntityMember>>);
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &normalized::Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid("catalog path premise absent"))
}
fn class_entity(data: &CatalogPreparation<'_>, id: Id<EntityRef>) -> Option<Id<ClassEntity>> {
    match data.refs.get(id) {
        Some(EntityRef::Class { class }) => Some(*class),
        _ => None,
    }
}
fn boundary(
    out: &mut CatalogOutput,
    parent: Id<CatalogCandidate>,
    reason: PublicPathBoundaryReason,
    binding: Option<Id<lexical::BindingObservation>>,
) -> Result<(), ModelError> {
    out.boundaries.insert(CatalogPathBoundary {
        parent,
        reason,
        binding,
    })?;
    Ok(())
}
// Fixed path-specific indexes borrow the admitted rows and names. Invalid references remain
// lazy: a path that never expands must not start inspecting binding or declaration premises.
struct PathPreparation<'a> {
    entities: std::collections::BTreeMap<(Id<Occurrence>, bool), Vec<Id<EntityRef>>>,
    bindings: std::collections::BTreeMap<Id<Occurrence>, Vec<&'a lexical::BindingObservation>>,
    rebindings:
        std::collections::BTreeMap<(Id<Occurrence>, &'a str), Vec<&'a lexical::BindingObservation>>,
    missing_ancestry_owner: bool,
    missing_event: bool,
    missing_scope: bool,
    _charge: StateCharge,
}
impl<'a> PathPreparation<'a> {
    fn new(data: &'a CatalogPreparation<'_>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "catalog-path-correspondence");
        let mut entities = std::collections::BTreeMap::new();
        for row in data.source_classes.iter() {
            if let ClassEntity::Source { declaration } = row {
                let entity = EntityRef::Class { class: row.id() }.id();
                build::append_index(&mut entities, &mut charge, (*declaration, true), entity)?;
            }
        }
        for row in data.source_callables.iter() {
            if let CallableEntity::Source { declaration, .. } = row {
                let entity = EntityRef::Callable { callable: row.id() }.id();
                build::append_index(&mut entities, &mut charge, (*declaration, false), entity)?;
            }
        }
        let mut bindings = std::collections::BTreeMap::new();
        let mut rebindings = std::collections::BTreeMap::new();
        let mut missing_event = false;
        let mut missing_scope = false;
        for binding in data.bindings.iter() {
            let Some(event) = data.binding_events.get(binding.event) else {
                missing_event = true;
                continue;
            };
            build::append_index(&mut bindings, &mut charge, event.site, binding)?;
            let Some(scope) = data.lexical_scopes.get(binding.scope) else {
                missing_scope = true;
                continue;
            };
            if scope.kind == LexicalScopeKind::Class {
                build::append_index(
                    &mut rebindings,
                    &mut charge,
                    (scope.owner, event.name.as_str()),
                    binding,
                )?;
            }
        }
        let missing_ancestry_owner = data
            .ancestry
            .iter()
            .any(|a| data.resolutions.get(a.class).is_none());
        Ok(Self {
            entities,
            bindings,
            rebindings,
            missing_ancestry_owner,
            missing_event,
            missing_scope,
            _charge: charge,
        })
    }
    fn declaration_entity(
        &self,
        data: &CatalogPreparation<'_>,
        declaration: &DeclarationObservation,
    ) -> Option<Id<EntityRef>> {
        // Source rows were visited in their canonical order. Do not pair class/function kinds.
        let entity = *self
            .entities
            .get(&(
                declaration.declaration,
                declaration.kind == syntax::DeclarationKind::Class,
            ))?
            .first()?;
        data.refs.get(entity).map(Record::id)
    }
    fn bindings(
        &self,
        site: Id<Occurrence>,
    ) -> Result<impl Iterator<Item = &'a lexical::BindingObservation> + '_, ModelError> {
        if self.missing_event {
            return Err(invalid("catalog path premise absent"));
        }
        Ok(self.bindings.get(&site).into_iter().flatten().copied())
    }
    fn rebindings<'s>(
        &'s self,
        owner: Id<Occurrence>,
        name: &'s str,
    ) -> Result<impl Iterator<Item = &'a lexical::BindingObservation> + 's, ModelError> {
        if self.missing_event || self.missing_scope {
            return Err(invalid("catalog path premise absent"));
        }
        // Borrowed str keys compare by content; the query does not allocate another name.
        Ok(self
            .rebindings
            .get(&(owner, name))
            .into_iter()
            .flatten()
            .copied())
    }
}
struct PendingTrail {
    trail: Vec<Id<ClassEntity>>,
    charge: StateCharge,
}
fn enqueue(
    pending: &mut std::collections::BTreeMap<Id<CatalogClass>, PendingTrail>,
    budget: &ResourceBudget,
    id: Id<CatalogClass>,
    trail: &[Id<ClassEntity>],
    next: Id<ClassEntity>,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "catalog-path-pending");
    let len = trail
        .len()
        .checked_add(1)
        .ok_or_else(|| invalid("catalog path trail overflow"))?;
    charge.grow(
        size_of::<Id<CatalogClass>>()
            + size_of::<PendingTrail>()
            + 32
            + len.saturating_mul(size_of::<Id<ClassEntity>>()),
    )?;
    let mut retained = Vec::with_capacity(len);
    retained.extend_from_slice(trail);
    retained.push(next);
    pending.insert(
        id,
        PendingTrail {
            trail: retained,
            charge,
        },
    );
    Ok(())
}
pub(super) fn expand(
    data: &CatalogPreparation<'_>,
    out: &mut CatalogOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let prepared = PathPreparation::new(data, budget)?;
    let mut done_charge = StateCharge::new(budget, "catalog-path-completed");
    // Consume canonical minimum keys instead of repeatedly scanning all completed work.
    let mut pending = std::collections::BTreeMap::new();
    let mut done = ChargedSet::default();
    for class in out.classes.iter() {
        enqueue(&mut pending, budget, class.id(), &[], class.class)?;
    }
    while let Some((id, mut retained)) = pending.pop_first() {
        // The B-tree entry is gone. The moved trail retains its separate allocation charge
        // until this expansion ends, including early depth boundaries and failures.
        retained.charge.release(size_of::<Id<CatalogClass>>() + 32);
        if !done.insert(&mut done_charge, id)? {
            continue;
        }
        let trail = &retained.trail;
        let mut charge = StateCharge::new(budget, "catalog-path-class-scratch");
        charge.admit(need(&out.classes, id)?)?;
        let class = need(&out.classes, id)?.clone();
        charge.admit(need(&out.candidates, class.candidate)?)?;
        let root = need(&out.candidates, class.candidate)?.clone();
        charge.admit(need(&out.exposures, root.exposure)?)?;
        let exposure_link = need(&out.exposures, root.exposure)?.clone();
        let exposure = need(&data.exposures, exposure_link.exposure)?;
        charge.admit(need(&out.members, class.member)?)?;
        let parent_member = need(&out.members, class.member)?.clone();
        if parent_member.path.len() >= MAX_DEPTH {
            boundary(out, class.candidate, PublicPathBoundaryReason::Depth, None)?;
            continue;
        }
        if prepared.missing_ancestry_owner {
            return Err(invalid("catalog path premise absent"));
        }
        let mut chain: ChargedMap<ChainKey, bool> = Default::default();
        chain.insert(&mut charge, (-1, class.class, None), true)?;
        for ancestry in data
            .resolutions_for_entity((
                exposure.context,
                Some(EntityRef::Class { class: class.class }.id()),
            ))
            .flat_map(|owner| data.ancestry_for(owner.id()))
        {
            let owner = need(&data.resolutions, ancestry.class)?;
            if owner.context != exposure.context
                || owner.entity.and_then(|e| class_entity(data, e)) != Some(class.class)
            {
                continue;
            }
            let raw = need(&data.ancestry_observations, ancestry.observation)?;
            if raw.relation != AncestryRelation::Mro {
                continue;
            }
            let complete = ancestry.status == ResolutionStatus::Resolved
                && raw.linearization == Some(Linearization::Complete);
            if !complete {
                boundary(
                    out,
                    class.candidate,
                    PublicPathBoundaryReason::OpenAncestry,
                    None,
                )?;
            }
            for ancestor in data.ancestors_for(ancestry.id()) {
                let resolution = need(&data.resolutions, ancestor.resolution)?;
                let ordinal = need(&data.sequence_members, ancestor.member)?.ordinal;
                if let Some(base) = resolution.entity.and_then(|e| class_entity(data, e)) {
                    chain.insert(
                        &mut charge,
                        (ordinal, base, Some(ancestor.id())),
                        complete && resolution.status == ResolutionStatus::Resolved,
                    )?;
                } else {
                    boundary(
                        out,
                        class.candidate,
                        PublicPathBoundaryReason::OpenAncestry,
                        None,
                    )?;
                }
            }
        }
        let mut first_gap = i64::MAX;
        for ancestry in data
            .resolutions_for_entity((
                exposure.context,
                Some(EntityRef::Class { class: class.class }.id()),
            ))
            .flat_map(|owner| data.ancestry_for(owner.id()))
        {
            let owner = need(&data.resolutions, ancestry.class)?;
            if owner.context != exposure.context
                || owner.entity.and_then(|e| class_entity(data, e)) != Some(class.class)
            {
                continue;
            }
            if need(&data.ancestry_observations, ancestry.observation)?.relation
                != AncestryRelation::Mro
            {
                continue;
            }
            for ancestor in data.ancestors_for(ancestry.id()) {
                let resolution = need(&data.resolutions, ancestor.resolution)?;
                let base = resolution.entity.and_then(|e| class_entity(data, e));
                if resolution.status != ResolutionStatus::Resolved
                    || !base.is_some_and(|c| {
                        matches!(data.source_classes.get(c), Some(ClassEntity::Source { .. }))
                    })
                {
                    first_gap =
                        first_gap.min(need(&data.sequence_members, ancestor.member)?.ordinal);
                    boundary(
                        out,
                        class.candidate,
                        PublicPathBoundaryReason::OpenAncestry,
                        None,
                    )?;
                }
            }
        }
        let mut nearest: ChargedMap<&str, i64> = Default::default();
        for (rank, base, _) in chain.keys() {
            let Some(ClassEntity::Source {
                declaration: anchor,
            }) = data.source_classes.get(*base)
            else {
                continue;
            };
            for declaration in data.children_for(Some(*anchor)) {
                for binding in prepared.bindings(declaration.declaration)? {
                    let event = need(&data.binding_events, binding.event)?;
                    if event.site != declaration.declaration
                        || need(&data.qualifications, binding.qualification)?.context
                            != exposure.context
                    {
                        continue;
                    }
                    let previous = nearest
                        .get(event.name.as_str())
                        .copied()
                        .unwrap_or(i64::MAX);
                    nearest.insert(&mut charge, event.name.as_str(), previous.min(*rank))?;
                }
            }
        }
        for ((rank, base, ancestor), complete) in chain.iter() {
            let Some(ClassEntity::Source {
                declaration: anchor,
            }) = data.source_classes.get(*base)
            else {
                continue;
            };
            for declaration in data.children_for(Some(*anchor)) {
                if need(&data.qualifications, declaration.qualification)?.context
                    != exposure.context
                {
                    continue;
                }
                let Some(entity) = prepared.declaration_entity(data, declaration) else {
                    boundary(
                        out,
                        class.candidate,
                        PublicPathBoundaryReason::MissingCorrespondence,
                        None,
                    )?;
                    continue;
                };
                for binding in prepared.bindings(declaration.declaration)? {
                    let event = need(&data.binding_events, binding.event)?;
                    if event.site != declaration.declaration
                        || need(&data.qualifications, binding.qualification)?.context
                            != exposure.context
                    {
                        continue;
                    }
                    if event.name.starts_with('_')
                        && !matches!(event.name.as_str(), "__init__" | "__call__")
                    {
                        continue;
                    }
                    let q = need(&data.qualifications, binding.qualification)?;
                    let definite = q.modality == attribution::Modality::Definite
                        && q.approximation == assertion::Approximation::Exact
                        && q.condition == conditions::Diagram::always().id()
                        && binding.static_branch.is_none();
                    let mut disposition = if exposure.status == ResolutionStatus::Resolved
                        && root.alias.is_none()
                        && *complete
                        && *rank < first_gap
                        && definite
                        && nearest.get(event.name.as_str()) == Some(rank)
                    {
                        PublicPathDisposition::Effective
                    } else if nearest.get(event.name.as_str()).is_some_and(|r| r < rank) {
                        PublicPathDisposition::Shadowed
                    } else {
                        PublicPathDisposition::Candidate
                    };
                    for ((other_rank, other_base, _), _) in
                        chain.iter().filter(|((r, _, _), _)| r <= rank)
                    {
                        let Some(ClassEntity::Source {
                            declaration: other_anchor,
                        }) = data.source_classes.get(*other_base)
                        else {
                            continue;
                        };
                        for rebind in prepared.rebindings(*other_anchor, &event.name)? {
                            let scope = need(&data.lexical_scopes, rebind.scope)?;
                            let rebound_event = need(&data.binding_events, rebind.event)?;
                            if scope.owner != *other_anchor
                                || scope.kind != LexicalScopeKind::Class
                                || rebound_event.name != event.name
                                || need(&data.qualifications, rebind.qualification)?.context
                                    != exposure.context
                            {
                                continue;
                            }
                            if !matches!(
                                rebind.kind,
                                BindingEventKind::FunctionDef
                                    | BindingEventKind::ClassDef
                                    | BindingEventKind::AnnotationOnly
                            ) {
                                boundary(
                                    out,
                                    class.candidate,
                                    PublicPathBoundaryReason::Rebound,
                                    Some(rebind.id()),
                                )?;
                                disposition = PublicPathDisposition::Candidate;
                            } else if *other_rank == *rank
                                && rebind.ordinal > binding.ordinal
                                && !declaration.overload
                            {
                                disposition = PublicPathDisposition::Shadowed;
                            }
                        }
                    }
                    let mut segments = parent_member.path.clone();
                    segments.push(event.name.clone());
                    let member = out.members.insert(CatalogMember {
                        input: parent_member.input,
                        access: parent_member.access,
                        name: segments.join("."),
                        path: segments,
                    })?;
                    let path = out.paths.insert(CatalogPath {
                        parent: class.candidate,
                        declaration: declaration.id(),
                        binding: binding.id(),
                        entity,
                        ancestry: *ancestor,
                        disposition,
                    })?;
                    let nested_exposure = out.exposures.insert(CatalogExposure {
                        member,
                        exposure: exposure.id(),
                    })?;
                    let candidate = out.candidates.insert(CatalogCandidate {
                        exposure: nested_exposure,
                        candidate: root.candidate,
                        entity: None,
                        path: Some(path),
                        alias: root.alias,
                    })?;
                    match need(&data.refs, entity)? {
                        EntityRef::Callable { callable } => build::callable(
                            data,
                            out,
                            member,
                            candidate,
                            *callable,
                            exposure.context,
                        )?,
                        EntityRef::Class { class: nested } => {
                            let nested_class = out.classes.insert(CatalogClass {
                                member,
                                candidate,
                                class: *nested,
                            })?;
                            build::constructors(data, out, nested_class, exposure.context)?;
                            build::fields(data, out, member, *nested, exposure.context)?;
                            if trail.contains(nested) {
                                boundary(
                                    out,
                                    candidate,
                                    PublicPathBoundaryReason::RecursiveClass,
                                    None,
                                )?;
                            } else {
                                if !done.contains(&nested_class) {
                                    enqueue(&mut pending, budget, nested_class, trail, *nested)?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
