//! Execute model-owned publication checks against the completing stage's sealed inputs.
use super::{Error, GenerationId, GenerationStore};
use lctx_model::domain::{
    resources::ResourceBudget,
    stages::{CompletedRelation, PrefixOrdinal, Profile},
};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};

impl GenerationStore {
    #[allow(
        clippy::too_many_arguments,
        reason = "Publication carries explicit frozen sources, prefix and charged session"
    )]
    pub(super) async fn check_publication_outputs_prepared(
        &self,
        tx: &mut PgConnection,
        generation: GenerationId,
        outputs: &BTreeSet<&'static str>,
        sources: &[CompletedRelation],
        candidate_prefix: Option<PrefixOrdinal>,
        budget: &ResourceBudget,
        session: &mut super::validation_session::Session<'_>,
    ) -> Result<(), Error> {
        let _references = budget.reserve(
            "publication-definition-refs",
            self.model
                .relations()
                .iter()
                .filter(|relation| outputs.contains(relation.name()))
                .map(|relation| relation.publication_refs().len().saturating_mul(128))
                .sum(),
        )?;
        let ids: BTreeSet<_> = self
            .model
            .relations()
            .iter()
            .filter(|relation| outputs.contains(relation.name()))
            .flat_map(|relation| relation.publication_refs().iter().copied())
            .collect();
        let checks: Vec<_> = ids
            .into_iter()
            .map(|id| self.model.publication_check(id))
            .collect::<Result<_, _>>()?;
        if checks.is_empty() {
            return Ok(());
        }
        let _plan = budget.reserve(
            "publication-input-plan",
            checks
                .iter()
                .map(|i| 128 + i.inputs.len().saturating_mul(512))
                .sum(),
        )?;
        // The registry captured this profile when the execution schedule was registered.
        // Test harnesses also register an explicit profile; neither path trusts output metadata.
        let profile: String =
            sqlx::query_scalar("SELECT profile FROM lctx_model_store.generations WHERE id=$1")
                .bind(generation.0.to_vec())
                .fetch_one(&mut *tx)
                .await?;
        let profile = Profile::ALL
            .into_iter()
            .find(|candidate| candidate.name() == profile)
            .ok_or(Error::Contract)?;
        let order = super::vocabulary::publication_order(tx, generation).await?;
        let _sources = budget.reserve(
            "publication-source-refs",
            sources.len().checked_mul(128).ok_or(Error::Contract)?,
        )?;
        let mut inputs = BTreeMap::new();
        for source in sources {
            if source.model() != self.model.digest() {
                return Err(Error::Contract);
            }
            super::check_schedule(tx, generation, source.schedule()).await?;
            if let Some(prefix) = source.prefix_ordinal() {
                order.validate(prefix)?;
                let closed: bool = sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2")
                    .bind(generation.0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?)
                    .fetch_one(&mut *tx).await?;
                if !closed {
                    return Err(Error::Contract);
                }
            }
            if let Some(previous) = inputs.insert(source.relation(), source)
                && previous != source
            {
                return Err(Error::Contract);
            }
            let actual: Option<(i64, Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT row_count,content_digest,schedule_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3")
                .bind(generation.0.to_vec()).bind(source.producer()).bind(source.relation()).fetch_optional(&mut *tx).await?;
            let receipt = source.receipt();
            if actual.as_ref().is_none_or(|(rows, content, schedule)| {
                u64::try_from(*rows).ok() != Some(receipt.rows)
                    || content != &receipt.content.0
                    || schedule != &source.schedule().0
            }) {
                return Err(Error::Contract);
            }
        }
        let mut requests = Vec::with_capacity(checks.len());
        for invariant in checks {
            let mut frames = Vec::with_capacity(invariant.inputs.len());
            for input in &invariant.inputs {
                let relation = self
                    .model
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .ok_or(Error::Contract)?;
                let physical = if outputs.contains(input.name()) {
                    candidate_prefix.map_or_else(
                        || input.name().to_owned(),
                        |prefix| super::vocabulary::physical(input.name(), prefix),
                    )
                } else {
                    inputs
                        .get(input.name())
                        .ok_or(Error::Contract)?
                        .physical_relation()
                };
                let upper = if outputs.contains(input.name()) {
                    candidate_prefix
                } else {
                    inputs
                        .get(input.name())
                        .and_then(|source| source.prefix_ordinal())
                };
                if input.prefix().is_some() && upper.is_none() {
                    return Err(Error::Contract);
                }
                let physical = super::validation_views::physical(
                    tx,
                    generation,
                    input,
                    relation,
                    &physical,
                    super::validation_views::Scope {
                        upper,
                        candidate: candidate_prefix,
                    },
                    budget,
                )
                .await?;
                frames.push(physical);
            }
            requests.push((invariant, frames));
        }
        session.publications(tx, &requests, sources, profile).await
    }
}
