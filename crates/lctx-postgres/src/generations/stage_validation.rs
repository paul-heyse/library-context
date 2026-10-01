//! Consumer eligibility over immutable completed inputs; semantic checks remain model-owned.
use super::{Error, GenerationId, GenerationStore, check_schedule, lock, qualified, visit_named};
use lctx_model::domain::{
    ContentHash, KeySink,
    admission::FrontierAdmission,
    resources::ResourceBudget,
    stages::{CompletedRelation, Stage},
};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};

impl GenerationStore {
    /// Freeze-time vocabulary admission for ordinary outputs. This runs under the completion
    /// transaction's output locks, before a receipt or read grant can escape. Native outputs
    /// preceding the Facts close retain their existing checkpoint/group validation boundary.
    pub(super) async fn check_completed_output_references(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        completion: &lctx_model::domain::stages::StageCompletion,
    ) -> Result<(), Error> {
        let order = super::vocabulary::publication_order(tx, g).await?;
        let explicit = completion.prefix_ordinal();
        let Some(prefix) = explicit.or_else(|| {
            order
                .resolve(lctx_model::domain::stages::PublicationBoundary::Facts)
                .ok()
        }) else {
            return Ok(());
        };
        order.validate(prefix)?;
        let closed: bool = sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2")
            .bind(g.0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_| Error::Contract)?)
            .fetch_one(&mut *tx).await?;
        if !closed {
            return if explicit.is_none() {
                Ok(())
            } else {
                Err(Error::Contract)
            };
        }
        let sources: BTreeMap<_, _> = completion
            .sources()
            .iter()
            .map(|s| (s.relation(), s))
            .collect();
        for source in sources.values() {
            if source.schedule() != completion.schedule()
                || source.identity().attempt() != completion.identity().attempt()
            {
                return Err(Error::Contract);
            }
            if let Some(bound) = source.prefix_ordinal() {
                order.validate(bound)?;
            }
        }
        for relation in self
            .model
            .relations()
            .iter()
            .filter(|r| completion.outputs().contains(r.name()))
        {
            for field in relation.fields() {
                let Some((_, target)) = field.target() else {
                    continue;
                };
                // Ordinary nominal targets retain the existing checkpoint/final validation
                // contract, including handoffs and inactive nullable sum arms. This additional
                // check enforces only the immutable vocabulary visibility bound.
                if !lctx_model::domain::stages::is_vocabulary(target) {
                    continue;
                }
                let bound = sources
                    .get(target)
                    .and_then(|s| s.prefix_ordinal())
                    .unwrap_or(prefix)
                    .earlier(prefix)?;
                let target_source = super::vocabulary::physical(target, bound);
                let subtype = if let Some(code) = field.subtype() {
                    let tag = self
                        .model
                        .relations()
                        .iter()
                        .find(|r| r.name() == target)
                        .and_then(|r| r.sum())
                        .ok_or(Error::Contract)?
                        .tag;
                    format!(" AND b.\"{tag}\"={code}")
                } else {
                    String::new()
                };
                let invalid: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT EXISTS(SELECT 1 FROM {} a WHERE a.\"{}\" IS NOT NULL AND NOT EXISTS(SELECT 1 FROM {} b WHERE b.id=a.\"{}\"{subtype}))",
                    qualified(g, relation.name()), field.name(), qualified(g, &target_source), field.name()
                ))).fetch_one(&mut *tx).await?;
                if invalid {
                    return Err(Error::Contract);
                }
            }
        }
        Ok(())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Input validation explicitly carries transaction, generation, stage, frozen sources and budget"
    )]
    pub(super) async fn check_stage_inputs(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        schedule: ContentHash,
        stage: &Stage,
        sources: &[CompletedRelation],
        checkpoint: Option<&FrontierAdmission>,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<&'static str, ContentHash>, Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        self.registered(tx, g).await?.expect("staging")?;
        check_schedule(tx, g, schedule).await?;
        let validated = checkpoint
            .map(|c| c.frontier().descriptor().relations(&self.model))
            .transpose()?
            .unwrap_or_default();
        let declared: BTreeSet<_> = sources.iter().map(|s| s.relation()).collect();
        let mut digest = KeySink::new("stage-read-inputs");
        digest.part(b"model", &self.model.digest().0);
        digest.part(b"schedule", &schedule.0);
        digest.part(b"consumer", &stage.digest().0);
        if let Some(c) = checkpoint {
            digest.part(b"checkpoint-content", &c.content().0);
            digest.part(b"checkpoint-coverage", &c.coverage().0);
        }
        let order = super::vocabulary::publication_order(tx, g).await?;
        let facts = order
            .resolve(lctx_model::domain::stages::PublicationBoundary::Facts)
            .ok();
        for source in sources {
            if let Some(prefix) = source.prefix_ordinal() {
                order.validate(prefix)?;
            }
        }
        let ordered: BTreeMap<_, _> = sources.iter().map(|s| (s.relation(), s)).collect();
        let input_physical = |name: &str| {
            ordered.get(name).map_or_else(
                || {
                    facts.map_or_else(
                        || name.to_owned(),
                        |prefix| super::vocabulary::physical(name, prefix),
                    )
                },
                |s| s.physical_relation(),
            )
        };
        let checkpoint_covers = |name: &str| {
            validated.contains(name)
                && ordered.get(name).is_none_or(|s| {
                    s.prefix()
                        .is_none_or(|e| e == lctx_model::domain::stages::PublicationBoundary::Facts)
                })
        };
        for (name, source) in &ordered {
            let actual: Option<(i64, Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT row_count,content_digest,schedule_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3")
                .bind(g.0.to_vec()).bind(source.producer()).bind(name).fetch_optional(&mut *tx).await?;
            let receipt = source.receipt();
            if actual
                .as_ref()
                .is_none_or(|(rows, content, stored_schedule)| {
                    u64::try_from(*rows).ok() != Some(receipt.rows)
                        || content != &receipt.content.0
                        || stored_schedule != &schedule.0
                })
            {
                return Err(Error::Contract);
            }
            let relation = self
                .model
                .relations()
                .iter()
                .find(|r| r.name() == *name)
                .ok_or(Error::Contract)?;
            let frozen =
                super::vocabulary::receipt(tx, g, relation, &source.physical_relation(), budget)
                    .await?;
            if frozen != receipt {
                return Err(Error::Contract);
            }
            digest.part(name.as_bytes(), &receipt.content.0);
            if let Some(prefix) = source.prefix_ordinal() {
                digest.part(b"prefix", &prefix.ordinal().to_le_bytes());
            }
            digest.part(b"rows", &receipt.rows.to_le_bytes());
        }
        let input_digest = digest.finish();
        // Every unvalidated reference target must be explicitly declared and frozen. Checking
        // all declared sources together admits same-stage cycles without trusting an empty join.
        for relation in self.model.relations().iter().filter(|r| {
            declared.contains(r.name())
                && (!validated.contains(r.name())
                    || ordered.get(r.name()).is_some_and(|s| {
                        s.prefix().is_some_and(|e| {
                            e != lctx_model::domain::stages::PublicationBoundary::Facts
                        })
                    }))
        }) {
            for field in relation.fields() {
                if let Some((_, target)) = field.target() {
                    if !declared.contains(target) && !validated.contains(target) {
                        return Err(Error::Contract);
                    }
                    let subtype = if let Some(code) = field.subtype() {
                        let tag = self
                            .model
                            .relations()
                            .iter()
                            .find(|r| r.name() == target)
                            .and_then(|r| r.sum())
                            .ok_or(Error::Contract)?
                            .tag;
                        format!(" AND b.\"{tag}\"={code}")
                    } else {
                        String::new()
                    };
                    let sql = format!(
                        "SELECT EXISTS(SELECT 1 FROM {} a WHERE a.\"{}\" IS NOT NULL AND NOT EXISTS(SELECT 1 FROM {} b WHERE b.id=a.\"{}\"{}))",
                        qualified(g, &ordered[relation.name()].physical_relation()),
                        field.name(),
                        qualified(g, &{
                            if lctx_model::domain::stages::is_vocabulary(target) {
                                let target_prefix = ordered
                                    .get(target)
                                    .and_then(|s| s.prefix_ordinal())
                                    .or(facts)
                                    .ok_or(Error::Contract)?;
                                let source_prefix = ordered[relation.name()]
                                    .prefix_ordinal()
                                    .or(facts)
                                    .ok_or(Error::Contract)?;
                                super::vocabulary::physical(
                                    target,
                                    target_prefix.earlier(source_prefix)?,
                                )
                            } else {
                                input_physical(target)
                            }
                        }),
                        field.name(),
                        subtype
                    );
                    let invalid: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
                        .fetch_one(&mut *tx)
                        .await?;
                    if invalid {
                        return Err(Error::Contract);
                    }
                }
            }
        }
        let mut checks = BTreeMap::from([("nominal_references", input_digest)]);
        for invariant in stage.read_invariants(&self.model)? {
            if invariant
                .inputs
                .iter()
                .any(|i| !declared.contains(i.name()) && !validated.contains(i.name()))
            {
                return Err(Error::Contract);
            }
            if !invariant
                .inputs
                .iter()
                .all(|i| i.prefix().is_none() && checkpoint_covers(i.name()))
            {
                let mut check = (invariant.create)(budget);
                for input in &invariant.inputs {
                    let relation = self
                        .model
                        .relations()
                        .iter()
                        .find(|r| r.name() == input.name())
                        .ok_or(Error::Contract)?;
                    let inherited = ordered
                        .get(input.name())
                        .and_then(|source| source.prefix_ordinal())
                        .or_else(|| validated.contains(input.name()).then_some(facts).flatten());
                    if input.prefix().is_some() && inherited.is_none() {
                        return Err(Error::Contract);
                    }
                    let physical = super::validation_views::physical(
                        tx,
                        g,
                        input,
                        relation,
                        &input_physical(input.name()),
                        super::validation_views::Scope {
                            upper: inherited,
                            candidate: None,
                        },
                        budget,
                    )
                    .await?;
                    visit_named(tx, g, relation, &physical, input.order(), budget, |batch| {
                        check.visit_input(input, &batch)?;
                        Ok(())
                    })
                    .await?;
                }
                check.finish()?;
            }
            checks.insert(invariant.name, input_digest);
        }
        for (name, digest) in &checks {
            sqlx::query("INSERT INTO lctx_model_store.stage_read_checks VALUES($1,$2,$3,$4) ON CONFLICT(generation_id,consumer,check_name) DO NOTHING")
                .bind(g.0.to_vec()).bind(stage.name).bind(name).bind(digest.0.to_vec()).execute(&mut *tx).await?;
            let stored: Vec<u8> = sqlx::query_scalar("SELECT input_digest FROM lctx_model_store.stage_read_checks WHERE generation_id=$1 AND consumer=$2 AND check_name=$3")
                .bind(g.0.to_vec()).bind(stage.name).bind(name).fetch_one(&mut *tx).await?;
            if stored != digest.0 {
                return Err(Error::Contract);
            }
        }
        Ok(checks)
    }
}
