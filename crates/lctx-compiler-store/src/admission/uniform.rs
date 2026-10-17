//! Exact selected producer binding for compact outcomes, shared by all compiler
//! validation consumers. These semantic owners do not grant physical read access.
use super::*;
use lctx_model::domain::uniform_outcome::{
    AnalysisUniformInputs, UniformKind, UniformNotRequested, ValidatedUniformOwner,
};
pub struct AdmittedUniformOwners {
    identity: ContentHash,
    owners: lctx_model::domain::charged::ChargedVec<ValidatedUniformOwner>,
    _charge: StateCharge,
}
impl AdmittedUniformOwners {
    pub fn identity(&self) -> ContentHash {
        self.identity
    }
    pub fn owners(&self) -> &[ValidatedUniformOwner] {
        &self.owners
    }
}
impl AdmissionRuntime {
    fn analysis_uniform_inputs<'a>(
        &'a self,
        owner: &'a CompletedContribution,
        bindings: &'a [CompletedBinding],
        profile: Profile,
    ) -> BoxFuture<'a, Result<AnalysisUniformInputs, ModelError>> {
        Box::pin(async move {
            let mut inputs = AnalysisUniformInputs::new(profile, self.budget())?;
            let required = AnalysisUniformInputs::inputs();
            let mut loaded = lctx_model::domain::charged::ChargedSet::default();
            let mut charge =
                StateCharge::new(self.budget(), "compact analysis selected metadata views");
            for source in &owner.spec.inputs {
                if source.relation() == UniformNotRequested::NAME
                    || !required
                        .iter()
                        .any(|input| input.name() == source.relation())
                {
                    continue;
                }
                if loaded.contains(&source.view()) {
                    continue;
                }
                let binding = bindings
                    .iter()
                    .find(|binding| binding.source == *source)
                    .ok_or(ModelError::Conflict(
                        "compact analysis actual selected metadata binding absent",
                    ))?;
                binding.validate()?;
                let relation = self
                    .model()
                    .relation(source.relation())
                    .ok_or(ModelError::Schema("compact analysis metadata relation"))?;
                let mut rows = self
                    .native
                    .scan_batches(
                        &binding.view,
                        relation,
                        None,
                        None,
                        self.budget(),
                        self.options.batch_rows,
                    )
                    .await?;
                let mut count = 0u64;
                while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    count =
                        count
                            .checked_add(batch.num_rows() as u64)
                            .ok_or(ModelError::Conflict(
                                "compact analysis metadata count overflow",
                            ))?;
                    if count > binding.view.rows {
                        return Err(ModelError::Conflict(
                            "compact analysis metadata exceeds exact view",
                        ));
                    }
                    inputs.visit(source.relation(), &batch)?;
                }
                if count != binding.view.rows {
                    return Err(ModelError::Conflict(
                        "compact analysis metadata missing physical EOF rows",
                    ));
                }
                loaded.insert(&mut charge, source.view())?;
            }
            Ok(inputs)
        })
    }
    pub fn uniform_owners_async(
        &self,
        profile: Profile,
    ) -> BoxFuture<'_, Result<Arc<AdmittedUniformOwners>, ModelError>> {
        Box::pin(async move {
            self.cancellation.check()?;
            let mut charge = StateCharge::new(self.budget(), "selected compact producer owners");
            let mut sources = Vec::new();
            for source in self
                .completed
                .lock()
                .map_err(|_| poisoned())?
                .values()
                .chain(self.frozen_shared.lock().map_err(|_| poisoned())?.values())
                .filter(|source| source.name() == UniformNotRequested::NAME)
            {
                if !sources
                    .iter()
                    .any(|known: &Arc<ExactRelation>| known.view() == source.view())
                {
                    charge.grow(size_of::<Arc<ExactRelation>>() + 128)?;
                    sources.push(source.clone());
                }
            }
            let CompletedInventory {
                contributions: completed,
                bindings,
                transfer: _transfer,
            } = self.completed_inventory()?;
            // Only actual compact contributors and their exact qualification inputs
            // affect this semantic owner. Unrelated consumer roots do not inhibit reuse.
            sources.sort_by_key(|source| source.view_identity());
            let mut digest = lctx_model::domain::KeySink::new("selected-uniform-owners/v3");
            for source in sources.iter().filter(|source| source.view().rows != 0) {
                digest.part(b"compact", &source.view_identity().0);
            }
            for owner in &completed {
                let id = owner.identity()?;
                if !sources.iter().any(|source| {
                    source.view().rows != 0 && source.view().contributions.contains(&id)
                }) {
                    continue;
                }
                digest.part(b"owner", &id.0);
                for input in &owner.spec.inputs {
                    let mut found = false;
                    for binding in bindings.iter().filter(|binding| binding.source == *input) {
                        digest.part(
                            b"qualification-binding",
                            &serde_json::to_vec(binding).map_err(ModelError::codec)?,
                        );
                        found = true;
                    }
                    if !found {
                        return Err(ModelError::Conflict(
                            "compact owner qualification input binding absent",
                        ));
                    }
                }
            }
            let identity = if sources.iter().all(|source| source.view().rows == 0) {
                ContentHash::of(b"empty-admitted-uniform-owners/v1")
            } else {
                digest.finish()
            };
            let key = Some((profile, identity));
            if let Some(key) = key
                && let Some((known, owners)) = &*self.memo.uniform.lock().map_err(|_| poisoned())?
                && *known == key
            {
                return Ok(owners.clone());
            }
            let mut owners = lctx_model::domain::charged::ChargedVec::default();
            for source in sources.iter().filter(|source| source.view().rows != 0) {
                let mut seen = lctx_model::domain::charged::ChargedSet::default();
                let mut seen_charge =
                    StateCharge::new(self.budget(), "selected compact row identities");
                let (rows, _rows) = self
                    .coverage_rows_async::<UniformNotRequested>(source)
                    .await?;
                for row in rows {
                    if seen.contains(&row.id()) {
                        return Err(ModelError::Conflict("duplicate selected compact outcome"));
                    }
                    seen.insert(&mut seen_charge, row.id())?;
                    let mut matching = completed.iter().filter(|owner| {
                        owner.spec.producer == row.owner
                            && owner.identity().is_ok_and(|identity| {
                                source.view().contributions.contains(&identity)
                            })
                    });
                    let owner = matching.next().ok_or(ModelError::Conflict(
                        "compact outcome has no exact selected producer",
                    ))?;
                    if matching.next().is_some() {
                        return Err(ModelError::Conflict(
                            "ambiguous selected compact outcome producer",
                        ));
                    }
                    let owner = match row.kind {
                        UniformKind::NativeFamily => {
                            ValidatedUniformOwner::validate_semantic_binding(
                                owner,
                                &row,
                                profile,
                                self.budget(),
                            )?
                        }
                        UniformKind::AnalysisOwner => {
                            let mut owner_binding_charge = StateCharge::new(
                                self.budget(),
                                "compact owner qualification binding selection",
                            );
                            let mut owner_bindings = Vec::new();
                            for binding in bindings
                                .iter()
                                .filter(|binding| owner.spec.inputs.contains(&binding.source))
                            {
                                owner_binding_charge.grow(
                                    serde_json::to_vec(binding)
                                        .map_err(ModelError::codec)?
                                        .len()
                                        .saturating_mul(8)
                                        .saturating_add(1024),
                                )?;
                                owner_bindings.push(binding.clone());
                            }
                            let inputs = self
                                .analysis_uniform_inputs(owner, &owner_bindings, profile)
                                .await?;
                            ValidatedUniformOwner::validate_analysis_binding(
                                owner,
                                &row,
                                profile,
                                self.model(),
                                &owner_bindings,
                                &inputs,
                                self.budget(),
                            )?
                        }
                    };
                    if let Some(prior) = owners
                        .iter()
                        .find(|prior: &&ValidatedUniformOwner| prior.descriptor().id() == row.id())
                    {
                        if prior.descriptor() != &row {
                            return Err(ModelError::Conflict("duplicate selected compact outcome"));
                        }
                    } else {
                        owners.push(&mut charge, owner)?;
                    }
                }
            }
            let owners = Arc::new(AdmittedUniformOwners {
                identity,
                owners,
                _charge: charge,
            });
            if let Some(key) = key {
                *self.memo.uniform.lock().map_err(|_| poisoned())? = Some((key, owners.clone()));
            }
            Ok(owners)
        })
    }
}
