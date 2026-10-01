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
    admission::{AdmissionCheck, Frontier, FrontierAdmission, Preflight},
    stages::{ExecutionReceipt, ProviderOutcome, RelationReceipt, StageCompletion},
};
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};

/// A facts attempt's admission inputs: its preflight and its execution receipt.
pub(super) struct Admitting<'a> {
    pub preflight: &'a Preflight,
    pub receipt: &'a ExecutionReceipt,
}

impl GenerationStore {
    /// Freeze outputs on the attempt's lifecycle connection. This connection already owns the
    /// exclusive attempt lock; acquiring a shared attempt lock from another session would deadlock.
    #[allow(
        clippy::too_many_arguments,
        reason = "The atomic completion effect explicitly carries transaction, generation, stage and content authority"
    )]
    pub(super) async fn complete_stage_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        schedule: ContentHash,
        stage: &str,
        outputs: &BTreeSet<&'static str>,
        outcome: ProviderOutcome,
        completion: Option<&StageCompletion>,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<&'static str, RelationReceipt>, Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        registered.expect("staging")?;
        check_schedule(tx, g, schedule).await?;
        if outcome == ProviderOutcome::Failed {
            return Err(Error::State);
        }
        let grouped: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2)").bind(g.0.to_vec()).bind(stage).fetch_one(&mut *tx).await?;
        if grouped {
            return Err(Error::Contract);
        }

        let planned: Vec<String> = sqlx::query_scalar("SELECT relation_name FROM lctx_model_store.planned_outputs WHERE generation_id=$1 AND stage_name=$2 ORDER BY relation_name COLLATE \"C\"")
            .bind(g.0.to_vec()).bind(stage).fetch_all(&mut *tx).await?;
        if planned.iter().map(String::as_str).collect::<BTreeSet<_>>() != *outputs {
            return Err(Error::Contract);
        }
        for name in outputs {
            if !self.scope(registered.frontier)?.relations.contains(name) {
                return Err(Error::Contract);
            }
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, name)
            )))
            .execute(&mut *tx)
            .await?;
        }
        if let Some(completion) = completion {
            self.check_completed_output_references(tx, g, completion)
                .await?;
            self.check_publication_outputs(tx, g, outputs, completion.sources(), None, budget)
                .await?;
        }
        // The unscheduled, testing-only Harness has no declared source or StageCompletion
        // authority. It exercises schema/algebra conformance through the full ordinary model
        // validators. Scheduled conformance and product attempts always take the branch above;
        // their source-sensitive publication checks cannot be bypassed by fixture construction.
        let mut receipts = BTreeMap::new();
        for name in outputs {
            execute(tx, ddl::completed_output(&g.schema(), name).to_vec()).await?;
            let relation = self
                .model
                .relations()
                .iter()
                .find(|r| r.name() == *name)
                .ok_or(Error::Contract)?;
            let mut content = relation.content();
            visit_physical(tx, g, relation, &["id"], budget, |batch| {
                relation.hash_rows(&batch, &mut content)?;
                Ok(())
            })
            .await?;
            let (rows, content) = content.finish();
            sqlx::query("INSERT INTO lctx_model_store.stage_receipts VALUES($1,$2,$3,$4,$5,$6)")
                .bind(g.0.to_vec())
                .bind(stage)
                .bind(name)
                .bind(schedule.0.to_vec())
                .bind(
                    i64::try_from(rows)
                        .map_err(|_| Error::Codec("stage row count overflow".into()))?,
                )
                .bind(content.0.to_vec())
                .execute(&mut *tx)
                .await?;
            receipts.insert(*name, RelationReceipt { rows, content });
        }
        sqlx::query("INSERT INTO lctx_model_store.stage_outcomes VALUES($1,$2,$3)")
            .bind(g.0.to_vec())
            .bind(stage)
            .bind(outcome.code())
            .execute(&mut *tx)
            .await?;
        Ok(receipts)
    }
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
        let unclosed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND NOT closed)").bind(g.0.to_vec()).fetch_one(&mut *tx).await?;
        if unclosed {
            return Err(Error::State);
        }
        self.revoke_writer(tx, g, registered.frontier).await?;
        let completed = sqlx::query("SELECT stage_name,relation_name,schedule_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1")
            .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
        let mut actual = BTreeSet::new();
        for row in completed {
            if row.try_get::<Vec<u8>, _>("schedule_digest")? != schedule.0 {
                return Err(Error::Contract);
            }
            actual.insert((
                row.try_get::<String, _>("stage_name")?,
                row.try_get::<String, _>("relation_name")?,
            ));
        }
        if actual
            .iter()
            .map(|(s, r)| (s.as_str(), r.as_str()))
            .collect::<BTreeSet<_>>()
            != *written
        {
            return Err(Error::State);
        }
        if let Some(outcomes) = outcomes {
            let stored: Vec<(String, i16)> = sqlx::query_as("SELECT stage_name,outcome FROM lctx_model_store.stage_outcomes WHERE generation_id=$1")
                .bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
            if stored.len() != outcomes.len()
                || stored.iter().any(|(s, o)| {
                    outcomes
                        .get(s.as_str())
                        .is_none_or(|outcome| outcome.code() != *o)
                })
            {
                return Err(Error::Contract);
            }
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
    ) -> Result<(ContentHash, Option<FrontierAdmission>), Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        let registered = self.registered(tx, g).await?;
        registered.expect("sealed")?;
        let frontier = registered.frontier;
        if frontier.descriptor().requires_admission() != admitting.is_some()
            || admitting
                .as_ref()
                .is_some_and(|a| a.preflight.contract().frontier() != frontier)
        {
            return Err(Error::Frontier(
                "frontier validation requires exactly its declared admission".into(),
            ));
        }
        execute(tx, self.lowering(g, frontier)?.phase("validated")).await?;
        let (digest, admission) = self
            .validate_scope(tx, g, frontier, budget, admitting, true)
            .await?;
        sqlx::query("UPDATE lctx_model_store.generations SET content_digest=$2 WHERE id=$1")
            .bind(g.0.to_vec())
            .bind(digest.0.to_vec())
            .execute(&mut *tx)
            .await?;
        transition(tx, g, "validated").await?;
        Ok((digest, admission))
    }
    /// One validator path for private checkpoints and final publication. Checkpoint validation
    /// does not create final-generation receipts or change lifecycle state.
    async fn validate_scope(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        frontier: Frontier,
        budget: &ResourceBudget,
        admitting: Option<Admitting<'_>>,
        persist: bool,
    ) -> Result<(ContentHash, Option<FrontierAdmission>), Error> {
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
            let frozen: Option<(i64, Vec<u8>)> = if lctx_model::domain::stages::is_vocabulary(
                relation.name(),
            ) {
                sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND relation_name=$2 ORDER BY epoch DESC LIMIT 1").bind(g.0.to_vec()).bind(relation.name()).fetch_optional(&mut *tx).await?
            } else {
                sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND relation_name=$2").bind(g.0.to_vec()).bind(relation.name()).fetch_optional(&mut *tx).await?
            };
            if frozen.is_some_and(|(count, bytes)| {
                u64::try_from(count).ok() != Some(row_count) || bytes != digest.0
            }) {
                return Err(Error::Contract);
            }
            content.part(relation.name().as_bytes(), &digest.0);
            if persist {
                sqlx::query("INSERT INTO lctx_model_store.receipts(generation_id,relation_name,row_count,content_digest) VALUES($1,$2,$3,$4)")
                .bind(g.0.to_vec()).bind(relation.name()).bind(i64::try_from(row_count).map_err(|_| Error::Codec("row count overflow".into()))?)
                .bind(digest.0.to_vec()).execute(&mut *tx).await?;
            }
        }
        if persist {
            let old: Vec<(i16,String,i64,Vec<u8>)> = sqlx::query_as("SELECT epoch,relation_name,row_count,content_digest FROM lctx_model_store.epoch_receipts WHERE generation_id=$1").bind(g.0.to_vec()).fetch_all(&mut *tx).await?;
            let order = super::vocabulary::publication_order(tx, g).await?;
            for (epoch, name, count, bytes) in old {
                let epoch = order.decode(u16::try_from(epoch).map_err(|_| Error::Contract)?)?;
                let relation = self
                    .model
                    .relations()
                    .iter()
                    .find(|r| r.name() == name)
                    .ok_or(Error::Contract)?;
                let actual =
                    super::vocabulary::receipt(tx, g, relation, &epoch.view(&name), budget).await?;
                if i64::try_from(actual.rows).ok() != Some(count)
                    || actual.content.0 != bytes.as_slice()
                {
                    return Err(Error::Contract);
                }
            }
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
                let view=super::validation_views::physical(tx,g,input,relation,relation.name(),super::validation_views::Scope {upper:None,candidate:None},budget).await?;
                super::visit_named(tx, g, relation, &view, input.order(), budget, |batch| {
                    check.visit_input(input, &batch)?;
                    Ok(())
                })
                .await?;
            }
            check.finish()?;
            if persist {
                sqlx::query("INSERT INTO lctx_model_store.validation_receipts(generation_id,validator_name,content_digest,model_digest,physical_digest) VALUES($1,$2,$3,$4,$5)")
                .bind(g.0.to_vec()).bind(invariant.name).bind(digest.0.to_vec())
                .bind(self.model.digest().0.to_vec()).bind(physical.0.to_vec()).execute(&mut *tx).await?;
            }
        }
        let admission = match admitting {
            Some(Admitting { preflight, receipt }) => {
                let mut check = AdmissionCheck::new(preflight.clone(), budget);
                for input in check.inputs() {
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
        Ok((digest, admission))
    }
    pub(super) async fn checkpoint_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        budget: &ResourceBudget,
        preflight: &Preflight,
        receipt: &ExecutionReceipt,
    ) -> Result<FrontierAdmission, Error> {
        self.lock_installation(tx).await?;
        lock(tx, g, false).await?;
        self.registered(tx, g).await?.expect("staging")?;
        check_schedule(tx, g, receipt.schedule()).await?;
        let frontier = preflight.contract().frontier();
        for name in &self.scope(frontier)?.relations {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, name)
            )))
            .execute(&mut *tx)
            .await?;
        }
        for name in &self.scope(frontier)?.relations {
            execute(tx, ddl::completed_output(&g.schema(), name).to_vec()).await?;
        }
        // Validate nominal references using the same generated constraints as final seal. Keep
        // the checked DDL transient so final cumulative validation installs each constraint once.
        sqlx::query("SAVEPOINT checkpoint_references")
            .execute(&mut *tx)
            .await?;
        execute(tx, self.lowering(g, frontier)?.phase("validated")).await?;
        sqlx::query("ROLLBACK TO SAVEPOINT checkpoint_references")
            .execute(&mut *tx)
            .await?;
        let (_, admission) = self
            .validate_scope(
                tx,
                g,
                frontier,
                budget,
                Some(Admitting { preflight, receipt }),
                false,
            )
            .await?;
        let admission = admission.ok_or(Error::Contract)?;
        sqlx::query("INSERT INTO lctx_model_store.checkpoints VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(g.0.to_vec())
            .bind(frontier.name())
            .bind(admission.contract().0.to_vec())
            .bind(admission.model().0.to_vec())
            .bind(admission.schedule().0.to_vec())
            .bind(admission.coverage().0.to_vec())
            .bind(admission.content().0.to_vec())
            .execute(&mut *tx)
            .await?;
        Ok(admission)
    }
    /// Publication binds the receipts, the validator set, the planned outputs and, for a facts
    /// generation, its admission, then grants the reader and changes state, atomically.
    pub(super) async fn publish_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        admission: Option<&FrontierAdmission>,
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
        match (frontier.descriptor().requires_admission(), admission) {
            (true, Some(admission)) => {
                if admission.frontier() != frontier
                    || admission.model() != self.model.digest()
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
            (true, None) => {
                return Err(Error::Frontier(
                    "the frontier publishes only with its declared admission".into(),
                ));
            }
            (false, Some(_)) => return Err(Error::Contract),
            (false, None) => {}
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
