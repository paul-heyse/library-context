//! Exact validation universes and their sufficient stage grants are different products.
use super::{stages::*, *};
use std::collections::{BTreeMap, BTreeSet};

pub struct DependencyClosure {
    /// Ordered validator inputs retain their independent epoch and stream contracts.
    pub requirements: Vec<ValidationInput>,
    /// One grant per read relation, at the widest required acknowledged epoch.
    /// Inferred ordinary facts remain validation premises covered by the frozen facts checkpoint.
    pub grants: Vec<RelationUse>,
}
#[derive(Clone, Copy)]
pub enum LowerLayerPolicy {
    OmitInferredOrdinaryFacts,
}
impl DependencyClosure {
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
                .relations()
                .iter()
                .find(|r| r.name() == name)
                .ok_or_else(|| ModelError::Invalid(format!("closure relation absent: {name}")))
        };
        let own: BTreeSet<_> = owned
            .iter()
            .filter(|r| !is_vocabulary(r.name()))
            .map(|r| r.name())
            .collect();
        let facts: BTreeSet<_> = facts_relations().iter().map(Relation::name).collect();
        let resolve = |mut input: ValidationInput| -> Result<ValidationInput, ModelError> {
            if is_vocabulary(input.name()) {
                let epoch = input.prefix().unwrap_or(vocabulary);
                input = input.at_epoch(epoch);
                order.resolve(input.prefix().expect("resolved epoch"))?;
            } else if input.prefix().is_some() {
                return Err(ModelError::Invalid(
                    "ordinary closure input has vocabulary epoch".into(),
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
                pending.push(resolve(ValidationInput::of_relation(
                    relation(target)?,
                    &["id"],
                ))?);
            }
            for invariant in row.invariants() {
                for required in &invariant.inputs {
                    pending.push(resolve(required.clone())?);
                }
            }
            requirements.push(input);
        }
        let mut grants = BTreeMap::<_, RelationUse>::new();
        for mut grant in direct {
            if is_vocabulary(grant.name()) {
                grant = grant.at_epoch(grant.prefix().unwrap_or(vocabulary));
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
            let mut grant = RelationUse::of_relation(relation(input.name())?).completed_store();
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
    grants: &mut BTreeMap<&'static str, RelationUse>,
    grant: RelationUse,
    order: &PublicationOrder,
) -> Result<(), ModelError> {
    if let Some(existing) = grants.get_mut(grant.name()) {
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
        match (existing.prefix(), grant.prefix()) {
            (Some(a), Some(b)) if order.resolve(b)?.ordinal() > order.resolve(a)?.ordinal() => {
                *existing = existing.at_epoch(b)
            }
            (None, None) | (Some(_), Some(_)) => {}
            _ => {
                return Err(ModelError::Invalid(
                    "incompatible closure epoch policies".into(),
                ));
            }
        }
    } else {
        if let Some(epoch) = grant.prefix() {
            order.resolve(epoch)?;
        }
        grants.insert(grant.name(), grant);
    }
    Ok(())
}
