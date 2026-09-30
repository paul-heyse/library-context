//! The lifecycle steps an attempt runs on its lifecycle connection: sealing, stored-content
//! validation with facts admission, publication and failure (cutover plan P1.7).
//! Each runs inside its caller's transaction, under the installation lock and the exclusive
//! generation lock, in the order installation → selection → generation → attempt → relations.
use super::{
    Error, GenerationId, GenerationStore, check_schedule, ddl, execute, failure::Failure, lock,
    qualified, transition, visit_physical,
};
use lctx_model::domain::resources::ResourceBudget;
use lctx_model::domain::{
    Codebook, ContentHash, KeySink,
    admission::{AdmissionCheck, FactsAdmission, Frontier, Preflight},
    stages::{ExecutionReceipt, ProviderOutcome},
};
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};

/// A facts attempt's admission inputs: its preflight and its execution receipt.
pub(super) struct Admitting<'a> {
    pub preflight: &'a Preflight,
    pub receipt: &'a ExecutionReceipt,
}

impl GenerationStore {
    pub(super) async fn seal_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        schedule: ContentHash,
        written: &BTreeSet<(&str, &str)>,
        outcomes: Option<&BTreeMap<&'static str, ProviderOutcome>>,
    ) -> Result<(), Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        registered.expect("staging")?;
        check_schedule(tx, g, schedule).await?;
        self.revoke_writer(tx, g, registered.frontier).await?;
        for (stage, relation) in written {
            sqlx::query("INSERT INTO lctx_model_store.stage_receipts VALUES($1,$2,$3,$4)")
                .bind(g.0.to_vec())
                .bind(stage)
                .bind(relation)
                .bind(schedule.0.to_vec())
                .execute(&mut *tx)
                .await?;
        }
        for (stage, outcome) in outcomes.into_iter().flatten() {
            sqlx::query("INSERT INTO lctx_model_store.stage_outcomes VALUES($1,$2,$3)")
                .bind(g.0.to_vec())
                .bind(stage)
                .bind(outcome.code())
                .execute(&mut *tx)
                .await?;
        }
        transition(tx, g, "sealed").await
    }
    /// Table locks wait out in-flight writes, including raw writer SQL, before the writer is revoked.
    async fn revoke_writer(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        frontier: Frontier,
    ) -> Result<(), Error> {
        for relation in &self.scope(frontier)?.relations {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, relation)
            )))
            .execute(&mut *tx)
            .await?;
        }
        execute(tx, self.lowering(g, frontier)?.phase("sealed")).await
    }
    /// Receipt every held relation's stored content, run every invariant whose inputs the
    /// frontier holds and, for a facts generation, admission. Read buffers and validator state
    /// are charged to `budget`.
    pub(super) async fn validate_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        budget: &ResourceBudget,
        admitting: Option<Admitting<'_>>,
    ) -> Result<(ContentHash, Option<FactsAdmission>), Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        registered.expect("sealed")?;
        let frontier = registered.frontier;
        if (frontier == Frontier::Facts) != admitting.is_some() {
            return Err(Error::Frontier(
                "a facts generation, and only one, is validated with its admission".into(),
            ));
        }
        execute(tx, self.lowering(g, frontier)?.phase("validated")).await?;
        let (relations, invariants) = self.scoped(frontier)?;
        let physical = self.scope(frontier)?.physical;
        let mut content = KeySink::new("generation-content");
        for relation in &relations {
            let mut rows = relation.content();
            visit_physical(tx, g, relation, &["id"], budget, |batch| {
                relation.hash_rows(&batch, &mut rows)?;
                Ok(())
            })
            .await?;
            let (row_count, digest) = rows.finish();
            content.part(relation.name().as_bytes(), &digest.0);
            sqlx::query("INSERT INTO lctx_model_store.receipts(generation_id,relation_name,row_count,content_digest) VALUES($1,$2,$3,$4)")
                .bind(g.0.to_vec()).bind(relation.name()).bind(i64::try_from(row_count).map_err(|_| Error::Codec("row count overflow".into()))?)
                .bind(digest.0.to_vec()).execute(&mut *tx).await?;
        }
        let digest = content.finish();
        for invariant in &invariants {
            let mut check = (invariant.create)(budget);
            for input in &invariant.inputs {
                let relation = self
                    .model
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .expect("validated invariant member");
                visit_physical(tx, g, relation, input.order(), budget, |batch| {
                    check.visit(input.name(), &batch)?;
                    Ok(())
                })
                .await?;
            }
            check.finish()?;
            sqlx::query("INSERT INTO lctx_model_store.validation_receipts(generation_id,validator_name,content_digest,model_digest,physical_digest) VALUES($1,$2,$3,$4,$5)")
                .bind(g.0.to_vec()).bind(invariant.name).bind(digest.0.to_vec())
                .bind(self.model.digest().0.to_vec()).bind(physical.0.to_vec()).execute(&mut *tx).await?;
        }
        let admission = match admitting {
            Some(Admitting { preflight, receipt }) => {
                let mut check = AdmissionCheck::new(preflight.clone(), budget);
                for input in AdmissionCheck::inputs() {
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
                Some(check.finish(receipt, digest)?)
            }
            None => None,
        };
        sqlx::query("UPDATE lctx_model_store.generations SET content_digest=$2 WHERE id=$1")
            .bind(g.0.to_vec())
            .bind(digest.0.to_vec())
            .execute(&mut *tx)
            .await?;
        transition(tx, g, "validated").await?;
        Ok((digest, admission))
    }
    /// Publication binds the receipts, the validator set, the planned outputs and, for a facts
    /// generation, its admission, then grants the reader and changes state, atomically.
    pub(super) async fn publish_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        admission: Option<&FactsAdmission>,
    ) -> Result<(), Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        registered.expect("validated")?;
        let frontier = registered.frontier;
        let (relations, invariants) = self.scoped(frontier)?;
        let physical = self.scope(frontier)?.physical;
        let receipts = sqlx::query("SELECT relation_name,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 ORDER BY relation_name COLLATE \"C\"")
            .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
        if receipts.len() != relations.len() {
            return Err(Error::State);
        }
        let mut content = KeySink::new("generation-content");
        for (receipt, relation) in receipts.iter().zip(&relations) {
            if receipt.try_get::<String, _>("relation_name")? != relation.name() {
                return Err(Error::Contract);
            }
            content.part(
                relation.name().as_bytes(),
                &receipt.try_get::<Vec<u8>, _>("content_digest")?,
            );
        }
        let digest = content.finish();
        let (stored, schedule, profile): (Vec<u8>, Vec<u8>, String) = sqlx::query_as("SELECT content_digest, schedule_digest, profile FROM lctx_model_store.generations WHERE id=$1")
            .bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if stored != digest.0 {
            return Err(Error::Contract);
        }
        let validations = sqlx::query("SELECT validator_name,content_digest,model_digest,physical_digest FROM lctx_model_store.validation_receipts WHERE generation_id=$1 ORDER BY validator_name COLLATE \"C\"")
            .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
        if validations.len() != invariants.len() {
            return Err(Error::State);
        }
        for (receipt, invariant) in validations.iter().zip(&invariants) {
            if receipt.try_get::<String, _>("validator_name")? != invariant.name
                || receipt.try_get::<Vec<u8>, _>("content_digest")? != digest.0
                || receipt.try_get::<Vec<u8>, _>("model_digest")? != self.model.digest().0
                || receipt.try_get::<Vec<u8>, _>("physical_digest")? != physical.0
            {
                return Err(Error::Contract);
            }
        }
        let unplanned: i64 = sqlx::query_scalar("SELECT count(*) FROM (\
            (SELECT stage_name, relation_name FROM lctx_model_store.planned_outputs WHERE generation_id=$1 \
             EXCEPT SELECT stage_name, relation_name FROM lctx_model_store.stage_receipts WHERE generation_id=$1) UNION ALL \
            (SELECT stage_name, relation_name FROM lctx_model_store.stage_receipts WHERE generation_id=$1 \
             EXCEPT SELECT stage_name, relation_name FROM lctx_model_store.planned_outputs WHERE generation_id=$1)) differing")
            .bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if unplanned != 0 {
            return Err(Error::State);
        }
        match (frontier, admission) {
            (Frontier::Facts, Some(admission)) => {
                if admission.model() != self.model.digest()
                    || admission.content() != digest
                    || admission.profile().name() != profile
                    || schedule != admission.schedule().0
                {
                    return Err(Error::Contract);
                }
                sqlx::query("INSERT INTO lctx_model_store.admissions VALUES($1,$2,$3,$4,$5,$6,$7)")
                    .bind(g.0.to_vec())
                    .bind(admission.contract().0.to_vec())
                    .bind(admission.model().0.to_vec())
                    .bind(admission.schedule().0.to_vec())
                    .bind(admission.coverage().0.to_vec())
                    .bind(admission.content().0.to_vec())
                    .bind(profile)
                    .execute(&mut *tx)
                    .await?;
                for (family, availability) in admission.availability() {
                    sqlx::query("INSERT INTO lctx_model_store.admission_families VALUES($1,$2,$3)")
                        .bind(g.0.to_vec())
                        .bind(family.code())
                        .bind(availability.code())
                        .execute(&mut *tx)
                        .await?;
                }
            }
            (Frontier::Facts, None) => {
                return Err(Error::Frontier(
                    "a facts generation publishes only with its admission".into(),
                ));
            }
            (Frontier::Conformance, Some(_)) => return Err(Error::Contract),
            (Frontier::Conformance, None) => {}
        }
        execute(tx, self.lowering(g, frontier)?.phase("published")).await?;
        transition(tx, g, "published").await
    }
    /// Record a generation `failed` from its current unpublished state. A generation failed while
    /// staging loses its writer first.
    pub(super) async fn fail_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        failure: &Failure,
    ) -> Result<(), Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        if !matches!(
            registered.state.as_str(),
            "staging" | "sealed" | "validated"
        ) {
            return Err(Error::State);
        }
        if registered.state == "staging" {
            self.revoke_writer(tx, g, registered.frontier).await?;
        }
        sqlx::query("INSERT INTO lctx_model_store.failures VALUES($1,$2,$3,$4)")
            .bind(g.0.to_vec())
            .bind(&registered.state)
            .bind(failure.class.name())
            .bind(&failure.detail)
            .execute(&mut *tx)
            .await?;
        transition(tx, g, ddl::FAILED).await
    }
}
