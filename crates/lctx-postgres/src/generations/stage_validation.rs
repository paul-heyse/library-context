//! Consumer eligibility over immutable completed inputs; semantic checks remain model-owned.
use super::{
    Error, GenerationId, GenerationStore, check_schedule, lock, qualified, visit_physical,
};
use lctx_model::domain::{
    ContentHash, KeySink,
    admission::FrontierAdmission,
    resources::ResourceBudget,
    stages::{CompletedRelation, Stage},
};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};

impl GenerationStore {
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
        let ordered: BTreeMap<_, _> = sources.iter().map(|s| (s.relation(), s)).collect();
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
            digest.part(name.as_bytes(), &receipt.content.0);
            digest.part(b"rows", &receipt.rows.to_le_bytes());
        }
        let input_digest = digest.finish();
        // Every unvalidated reference target must be explicitly declared and frozen. Checking
        // all declared sources together admits same-stage cycles without trusting an empty join.
        for relation in self
            .model
            .relations()
            .iter()
            .filter(|r| declared.contains(r.name()) && !validated.contains(r.name()))
        {
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
                        qualified(g, relation.name()),
                        field.name(),
                        qualified(g, target),
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
                .all(|i| validated.contains(i.name()))
            {
                let mut check = (invariant.create)(budget);
                for input in &invariant.inputs {
                    let relation = self
                        .model
                        .relations()
                        .iter()
                        .find(|r| r.name() == input.name())
                        .ok_or(Error::Contract)?;
                    visit_physical(tx, g, relation, input.order(), budget, |batch| {
                        check.visit(input.name(), &batch)?;
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
