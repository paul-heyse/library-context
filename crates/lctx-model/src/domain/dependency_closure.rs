//! Exact immutable semantic view requirements and their static compiler inputs.
use super::{stages::*, *};
use std::collections::{BTreeMap, BTreeSet};

pub struct DependencyClosure {
    /// Ordered validator inputs retain their independent epoch and stream contracts.
    pub requirements: Vec<ValidationInput>,
    /// One static compiler input per exact relation and completed semantic view.
    /// Inferred ordinary facts remain validation premises covered by the frozen facts checkpoint.
    pub grants: Vec<RelationUse>,
}
#[derive(Clone, Copy)]
pub enum LowerLayerPolicy {
    OmitInferredOrdinaryFacts,
    IncludeInferredOrdinaryFacts,
}
impl DependencyClosure {
    /// Construct sufficient grants without discarding exact validation roots on the way in.
    pub fn grants(
        model: &ValidatedModel,
        roots: Vec<ValidationInput>,
        direct: Vec<RelationUse>,
        owned: &[RelationUse],
        vocabulary: PublicationBoundary,
        lower: LowerLayerPolicy,
        order: &PublicationOrder,
    ) -> Result<Vec<RelationUse>, ModelError> {
        let mut roots = roots;
        roots.extend(Self::roots_from_uses(model, &direct)?);
        Ok(Self::build(model, roots, direct, owned, vocabulary, lower, order)?.grants)
    }
    /// Read declarations are roots too. Their epoch is retained, rather than inferred by name.
    pub fn roots_from_uses(
        model: &ValidatedModel,
        uses: &[RelationUse],
    ) -> Result<Vec<ValidationInput>, ModelError> {
        uses.iter()
            .map(|use_| {
                let relation = model.relation(use_.name()).ok_or_else(|| {
                    ModelError::Invalid(format!("closure relation absent: {}", use_.name()))
                })?;
                let input = ValidationInput::of_relation(relation, &["id"]);
                Ok(if let Some(epoch) = use_.prefix() {
                    input.at_epoch(epoch)
                } else {
                    input
                })
            })
            .collect()
    }

    pub fn stage_grants(
        model: &ValidatedModel,
        roots: Vec<ValidationInput>,
        owned: &[RelationUse],
        vocabulary: PublicationBoundary,
        lower: LowerLayerPolicy,
        order: &PublicationOrder,
    ) -> Result<Vec<RelationUse>, ModelError> {
        let direct = roots
            .iter()
            .map(|input| {
                let relation = model.relation(input.name()).ok_or_else(|| {
                    ModelError::Invalid(format!("closure relation absent: {}", input.name()))
                })?;
                let use_ = RelationUse::of_relation(relation).completed_input();
                Ok(if let Some(epoch) = input.prefix() {
                    use_.at_epoch(epoch)
                } else {
                    use_
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Self::grants(model, roots, direct, owned, vocabulary, lower, order)
    }
    pub fn build(
        model: &ValidatedModel,
        roots: Vec<ValidationInput>,
        direct: Vec<RelationUse>,
        owned: &[RelationUse],
        vocabulary: PublicationBoundary,
        lower: LowerLayerPolicy,
        order: &PublicationOrder,
    ) -> Result<Self, ModelError> {
        let relation = |name| {
            model
                .relation(name)
                .ok_or_else(|| ModelError::Invalid(format!("closure relation absent: {name}")))
        };
        let own: BTreeSet<_> = owned
            .iter()
            .filter(|r| !is_epoch_shared(r.name()))
            .map(|r| r.name())
            .collect();
        let facts: BTreeSet<_> = facts_relations().iter().map(Relation::name).collect();
        // Canonical value views are selected by exact typed roots, independently of the
        // vocabulary checkpoint. Keep all declared views; inference cannot choose by recency.
        let mut declared_views = BTreeMap::<_, BTreeSet<_>>::new();
        for (name, epoch) in roots.iter().map(|r| (r.name(), r.prefix()))
            .chain(direct.iter().map(|r| (r.name(), r.prefix())))
        {
            if is_epoch_shared(name) && !is_vocabulary(name) && let Some(epoch) = epoch {
                declared_views.entry(name).or_default().insert(epoch);
            }
        }
        let selected_view = |name, views: Option<&BTreeSet<PublicationBoundary>>| {
            match views {
                Some(views) if views.len() == 1 => Ok(*views.first().expect("one view")),
                Some(_) => Err(ModelError::Invalid(format!(
                    "ambiguous canonical dependency view: {name}"
                ))),
                None => Err(ModelError::Invalid(format!(
                    "canonical dependency needs an explicit view: {name}"
                ))),
            }
        };
        let resolve = |mut input: ValidationInput| -> Result<ValidationInput, ModelError> {
            if is_epoch_shared(input.name()) {
                let epoch = match input.prefix() {
                    Some(epoch) => epoch,
                    None if is_vocabulary(input.name()) => vocabulary,
                    None => selected_view(input.name(), declared_views.get(input.name()))?,
                };
                input = input.at_epoch(epoch);
                order.resolve(input.prefix().expect("resolved epoch"))?;
            } else if input.prefix().is_some() {
                return Err(ModelError::Invalid(
                    "ordinary closure input has shared epoch".into(),
                ));
            }
            if own.contains(input.name()) {
                return Err(ModelError::Invalid(format!(
                    "predecessor requires unfinished own output: {}",
                    input.name()
                )));
            }
            Ok(input)
        };
        let direct_names: BTreeSet<_> = roots
            .iter()
            .map(ValidationInput::name)
            .chain(direct.iter().map(|r| r.name()))
            .collect();
        let mut pending = roots
            .into_iter()
            .map(resolve)
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = BTreeSet::new();
        let mut requirements = Vec::new();
        while let Some(input) = pending.pop() {
            let key = (input.name(), input.prefix(), input.order().to_vec());
            if !seen.insert(key) {
                continue;
            }
            let row = relation(input.name())?;
            // An ordinary relation's attached invariant names the authoritative companion
            // views for its foreign keys. Resolve locally before considering caller roots.
            let mut owner_views = BTreeMap::<_, BTreeSet<_>>::new();
            let mut premises = Vec::new();
            for id in row.invariant_refs() {
                let invariant = model.invariant(id)?;
                for required in &invariant.inputs {
                    if is_epoch_shared(required.name()) && !is_vocabulary(required.name())
                        && let Some(epoch) = required.prefix()
                    {
                        owner_views.entry(required.name()).or_default().insert(epoch);
                    }
                    premises.push(required.clone());
                }
            }
            for target in row
                .fields()
                .iter()
                .filter_map(|f| f.target().map(|(_, name)| name))
            {
                if own.contains(target) {
                    return Err(ModelError::Invalid(format!(
                        "predecessor requires unfinished own output: {target}"
                    )));
                }
                if matches!(lower, LowerLayerPolicy::OmitInferredOrdinaryFacts)
                    && facts.contains(target)
                    && !is_vocabulary(target)
                {
                    continue;
                }
                let mut target_input = ValidationInput::of_relation(relation(target)?, &["id"]);
                if is_epoch_shared(target) && !is_vocabulary(target) {
                    let epoch = if is_epoch_shared(input.name()) && !is_vocabulary(input.name()) {
                        input.prefix().expect("resolved canonical parent view")
                    } else if !is_epoch_shared(input.name()) && owner_views.contains_key(target) {
                        selected_view(target, owner_views.get(target))?
                    } else {
                        selected_view(target, declared_views.get(target))?
                    };
                    target_input = target_input.at_epoch(epoch);
                }
                pending.push(resolve(target_input)?);
            }
            for required in premises {
                pending.push(resolve(required)?);
            }
            requirements.push(input);
        }
        let mut grants = BTreeMap::<_, RelationUse>::new();
        for mut grant in direct {
            relation(grant.name())?;
            if own.contains(grant.name()) {
                return Err(ModelError::Invalid(format!(
                    "predecessor requires unfinished own output: {}",
                    grant.name()
                )));
            }
            if is_epoch_shared(grant.name()) {
                let epoch = match grant.prefix() {
                    Some(epoch) => epoch,
                    None => resolve(ValidationInput::of_relation(relation(grant.name())?, &["id"]))?
                        .prefix().expect("resolved shared grant"),
                };
                grant = grant.at_epoch(epoch);
            }
            merge(&mut grants, grant, order)?;
        }
        for input in &requirements {
            if matches!(lower, LowerLayerPolicy::OmitInferredOrdinaryFacts)
                && facts.contains(input.name())
                && !is_vocabulary(input.name())
                && !direct_names.contains(input.name())
            {
                // This is a validation premise, not an actual consumed source. The admitted
                // facts checkpoint must cover it; store input admission refuses otherwise.
                continue;
            }
            let mut grant = RelationUse::of_relation(relation(input.name())?).completed_input();
            if let Some(epoch) = input.prefix() {
                grant = grant.at_epoch(epoch);
            }
            merge(&mut grants, grant, order)?;
        }
        Ok(Self {
            requirements,
            grants: grants.into_values().collect(),
        })
    }
}
fn merge(
    grants: &mut BTreeMap<(&'static str, Option<PublicationBoundary>), RelationUse>,
    grant: RelationUse,
    order: &PublicationOrder,
) -> Result<(), ModelError> {
    if let Some(existing) = grants.get_mut(&(grant.name(), grant.prefix())) {
        if existing.transport() != grant.transport()
            || (existing.requirement().is_some()
                && grant.requirement().is_some()
                && existing.requirement() != grant.requirement())
            || (!existing.validators().is_empty()
                && !grant.validators().is_empty()
                && existing.validators() != grant.validators())
        {
            return Err(ModelError::Invalid(
                "incompatible closure grant policies".into(),
            ));
        }
        if existing.requirement().is_none()
            && let Some(r) = grant.requirement()
        {
            *existing = existing.availability(r.group, r.policy);
        }
        if existing.validators().is_empty() {
            *existing = existing.validated_by(grant.validators());
        }
    } else {
        if let Some(epoch) = grant.prefix() {
            order.resolve(epoch)?;
        }
        grants.insert((grant.name(), grant.prefix()), grant);
    }
    Ok(())
}
