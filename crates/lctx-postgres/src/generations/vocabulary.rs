//! Lifecycle-owned private deltas and immutable vocabulary prefix publication (ADR-0105).
use super::{Error, GenerationId, GenerationStore, ddl, execute, qualified, quoted, visit_named};
use lctx_model::domain::{
    Record, Relation,
    resources::ResourceBudget,
    stages::{
        GroupCompletion, PublicationGroup, RelationReceipt, Stage, StageCompletion,
        VocabularyEpoch, is_vocabulary,
    },
};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};

impl GenerationStore {
    pub(super) async fn prepare_publications(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        groups: &[PublicationGroup],
        stages: &[Stage],
    ) -> Result<(), Error> {
        for group in groups {
            sqlx::query("INSERT INTO lctx_model_store.publication_groups(generation_id,epoch) VALUES($1,$2)").bind(g.0.to_vec()).bind(i16::from(group.epoch.code())).execute(&mut *tx).await?;
            for stage in stages.iter().filter(|s| group.stages.contains(&s.name)) {
                for output in &stage.outputs {
                    let relation = self
                        .model
                        .relations()
                        .iter()
                        .find(|r| r.name() == output.name())
                        .ok_or(Error::Contract)?;
                    // Canonical outputs are lifecycle-owned; the importer writes only this private delta.
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "REVOKE INSERT ON {} FROM lctx_importer",
                        qualified(g, relation.name())
                    )))
                    .execute(&mut *tx)
                    .await?;
                    execute(
                        tx,
                        ddl::delta_create(&g.schema(), relation, stage.name, true),
                    )
                    .await?;
                    sqlx::query("INSERT INTO lctx_model_store.publication_outputs(generation_id,epoch,stage_name,relation_name) VALUES($1,$2,$3,$4)").bind(g.0.to_vec()).bind(i16::from(group.epoch.code())).bind(stage.name).bind(relation.name()).execute(&mut *tx).await?;
                }
            }
        }
        Ok(())
    }
    pub(super) async fn compute_stage_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        completion: &StageCompletion,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<&'static str, RelationReceipt>, Error> {
        self.lock_installation(tx).await?;
        super::lock(tx, g, false).await?;
        self.registered(tx, g).await?.expect("staging")?;
        super::check_schedule(tx, g, completion.schedule()).await?;
        if completion.outcome() == lctx_model::domain::stages::ProviderOutcome::Failed {
            return Err(Error::State);
        }
        let planned: Vec<String> = sqlx::query_scalar("SELECT relation_name FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2 AND NOT sealed ORDER BY relation_name COLLATE \"C\"").bind(g.0.to_vec()).bind(completion.stage()).fetch_all(&mut *tx).await?;
        if planned.iter().map(String::as_str).collect::<BTreeSet<_>>() != *completion.outputs() {
            return Err(Error::Contract);
        }
        let mut receipts = BTreeMap::new();
        for name in completion.outputs() {
            let delta = ddl::delta_name(completion.stage(), name);
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, &delta)
            )))
            .execute(&mut *tx)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "REVOKE INSERT ON {} FROM lctx_importer",
                qualified(g, &delta)
            )))
            .execute(&mut *tx)
            .await?;
            let relation = self
                .model
                .relations()
                .iter()
                .find(|r| r.name() == *name)
                .ok_or(Error::Contract)?;
            let receipt = receipt(tx, g, relation, &delta, budget).await?;
            sqlx::query("UPDATE lctx_model_store.publication_outputs SET sealed=true,row_count=$4,content_digest=$5 WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(g.0.to_vec()).bind(completion.stage()).bind(name).bind(i64::try_from(receipt.rows).map_err(|_| Error::Contract)?).bind(receipt.content.0.to_vec()).execute(&mut *tx).await?;
            receipts.insert(*name, receipt);
        }
        Ok(receipts)
    }
    pub(super) async fn close_vocabulary_step(
        &self,
        tx: &mut PgConnection,
        g: GenerationId,
        group: &GroupCompletion,
        budget: &ResourceBudget,
    ) -> Result<(BTreeMap<&'static str, BTreeMap<&'static str, RelationReceipt>>, BTreeMap<&'static str, RelationReceipt>), Error> {
        self.lock_installation(tx).await?;
        super::lock(tx, g, false).await?;
        self.registered(tx, g).await?.expect("staging")?;
        let epoch = i16::from(group.epoch().code());
        let status: Option<bool> = sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2").bind(g.0.to_vec()).bind(epoch).fetch_optional(&mut *tx).await?;
        if status != Some(false) {
            return Err(Error::State);
        }
        let previous: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch<$2 AND NOT closed").bind(g.0.to_vec()).bind(epoch).fetch_one(&mut *tx).await?;
        if previous != 0 {
            return Err(Error::State);
        }
        let planned: Vec<(String,String,bool,i64,Vec<u8>)> = sqlx::query_as("SELECT stage_name,relation_name,sealed,row_count,content_digest FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND epoch=$2 ORDER BY relation_name COLLATE \"C\",stage_name COLLATE \"C\"").bind(g.0.to_vec()).bind(epoch).fetch_all(&mut *tx).await?;
        let actual: BTreeSet<_> = group
            .stages()
            .iter()
            .flat_map(|s| {
                s.completion()
                    .outputs()
                    .iter()
                    .map(move |r| (s.completion().stage(), *r))
            })
            .collect();
        if actual
            != planned
                .iter()
                .map(|(s, r, ..)| (s.as_str(), r.as_str()))
                .collect()
        {
            return Err(Error::Contract);
        }
        let mut available: BTreeSet<String> = sqlx::query_scalar(
            "SELECT relation_name FROM lctx_model_store.stage_receipts WHERE generation_id=$1",
        )
        .bind(g.0.to_vec())
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .collect();
        available.extend(actual.iter().map(|(_, r)| r.to_string()));
        // Deterministic canonical relation locks precede deterministic private delta locks.
        for name in &available {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, name)
            )))
            .execute(&mut *tx)
            .await?;
        }
        for (stage, name, sealed, count, digest) in &planned {
            if !sealed {
                return Err(Error::State);
            }
            let delta = ddl::delta_name(stage, name);
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE {} IN ACCESS EXCLUSIVE MODE",
                qualified(g, &delta)
            )))
            .execute(&mut *tx)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "REVOKE INSERT ON {} FROM lctx_importer",
                qualified(g, &delta)
            )))
            .execute(&mut *tx)
            .await?;
            let relation = self
                .model
                .relations()
                .iter()
                .find(|r| r.name() == name)
                .ok_or(Error::Contract)?;
            let frozen = receipt(tx, g, relation, &delta, budget).await?;
            let sealed = group
                .stages()
                .iter()
                .find(|s| s.completion().stage() == stage)
                .and_then(|s| s.deltas().get(name.as_str()))
                .ok_or(Error::Contract)?;
            if frozen != *sealed
                || i64::try_from(frozen.rows).ok() != Some(*count)
                || frozen.content.0 != digest.as_slice()
            {
                return Err(Error::Contract);
            }
            let columns = relation
                .schema()
                .fields()
                .iter()
                .map(|f| quoted(f.name()))
                .collect::<Vec<_>>()
                .join(",");
            let base = qualified(g, name);
            let delta = qualified(g, &delta);
            let payload = relation
                .schema()
                .fields()
                .iter()
                .map(|f| {
                    format!(
                        "b.{} IS DISTINCT FROM d.{}",
                        quoted(f.name()),
                        quoted(f.name())
                    )
                })
                .collect::<Vec<_>>()
                .join(" OR ");
            let conflict: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {base} b JOIN {delta} d USING(id) WHERE {payload}) OR EXISTS(SELECT 1 FROM (SELECT DISTINCT {columns} FROM {delta}) d GROUP BY id HAVING count(*)>1)"))).fetch_one(&mut *tx).await?;
            if conflict {
                return Err(Error::Model(lctx_model::domain::ModelError::Conflict(
                    relation.name(),
                )));
            }
            if is_vocabulary(name) {
                if name == <lctx_model::domain::value::LiteralSetMember as Record>::NAME {
                    let set = qualified(g, <lctx_model::domain::value::LiteralSet as Record>::NAME);
                    let mutates: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {delta} d JOIN {set} s ON s.id=d.\"set\" WHERE s.introduced_epoch < {epoch} AND NOT EXISTS(SELECT 1 FROM {base} b WHERE b.id=d.id))"))).fetch_one(&mut *tx).await?;
                    if mutates {
                        return Err(Error::Contract);
                    }
                }
                sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {base} ({columns},introduced_epoch) SELECT DISTINCT {columns},{epoch}::smallint FROM {delta} ON CONFLICT(generation_id,id) DO NOTHING"))).execute(&mut *tx).await?;
            } else {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "INSERT INTO {base} ({columns}) SELECT {columns} FROM {delta}"
                )))
                .execute(&mut *tx)
                .await?;
            }
        }
        // The same generated permanent FK declarations used at final seal validate the loaded
        // group here. Roll back only these transient constraints, then check literal visibility.
        sqlx::query("SAVEPOINT vocabulary_references")
            .execute(&mut *tx)
            .await?;
        let frontier = self.registered(tx, g).await?.frontier;
        execute(tx, self.lowering(g, frontier)?.phase("validated")).await?;
        sqlx::query("ROLLBACK TO SAVEPOINT vocabulary_references")
            .execute(&mut *tx)
            .await?;
        // Generated nominal references over the candidate prefix, including already completed
        // ordinary outputs. Merely existing in a future base row never establishes visibility.
        for relation in self
            .model
            .relations()
            .iter()
            .filter(|r| available.contains(r.name()))
        {
            for field in relation.fields() {
                if let Some((_, target)) = field.target() {
                    if !available.contains(target) {
                        return Err(Error::Contract);
                    }
                    let source = physical(relation.name(), group.epoch());
                    let target_relation = self
                        .model
                        .relations()
                        .iter()
                        .find(|r| r.name() == target)
                        .ok_or(Error::Contract)?;
                    let target_source = physical(target, group.epoch());
                    let subtype = field.subtype().map_or(String::new(), |code| {
                        format!(
                            " AND b.{}={code}",
                            quoted(target_relation.sum().expect("validated subtype").tag)
                        )
                    });
                    let invalid: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {} a WHERE a.{} IS NOT NULL AND NOT EXISTS(SELECT 1 FROM {} b WHERE b.id=a.{}{subtype}))",qualified(g,&source),quoted(field.name()),qualified(g,&target_source),quoted(field.name())))).fetch_one(&mut *tx).await?;
                    if invalid {
                        return Err(Error::Contract);
                    }
                }
            }
        }
        for invariant in self
            .model
            .invariants()
            .iter()
            .filter(|i| i.inputs.iter().all(|r| available.contains(r.name())))
        {
            let mut check = (invariant.create)(budget);
            for input in &invariant.inputs {
                let relation = self
                    .model
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .ok_or(Error::Contract)?;
                visit_named(
                    tx,
                    g,
                    relation,
                    &physical(relation.name(), group.epoch()),
                    input.order(),
                    budget,
                    |batch| {
                        check.visit(input.name(), &batch)?;
                        Ok(())
                    },
                )
                .await?;
            }
            check.finish()?;
        }
        let mut outputs = BTreeMap::new();
        for stage in group.stages() {
            let completion = stage.completion();
            super::check_schedule(tx, g, completion.schedule()).await?;
            let mut receipts = BTreeMap::new();
            for name in completion.outputs() {
                let relation = self
                    .model
                    .relations()
                    .iter()
                    .find(|r| r.name() == *name)
                    .ok_or(Error::Contract)?;
                let frozen =
                    receipt(tx, g, relation, &physical(name, group.epoch()), budget).await?;
                sqlx::query(
                    "INSERT INTO lctx_model_store.stage_receipts VALUES($1,$2,$3,$4,$5,$6)",
                )
                .bind(g.0.to_vec())
                .bind(completion.stage())
                .bind(name)
                .bind(completion.schedule().0.to_vec())
                .bind(i64::try_from(frozen.rows).map_err(|_| Error::Contract)?)
                .bind(frozen.content.0.to_vec())
                .execute(&mut *tx)
                .await?;
                execute(tx, ddl::completed_output(&g.schema(), name).to_vec()).await?;
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP TABLE {}",
                    qualified(g, &ddl::delta_name(completion.stage(), name))
                )))
                .execute(&mut *tx)
                .await?;
                receipts.insert(*name, frozen);
            }
            sqlx::query("INSERT INTO lctx_model_store.stage_outcomes VALUES($1,$2,$3)")
                .bind(g.0.to_vec())
                .bind(completion.stage())
                .bind(completion.outcome().code())
                .execute(&mut *tx)
                .await?;
            outputs.insert(completion.stage(), receipts);
        }
        let mut vocabulary = BTreeMap::new();
        for relation in self
            .model
            .relations()
            .iter()
            .filter(|r| available.contains(r.name()) && is_vocabulary(r.name()))
        {
            let frozen = receipt(
                tx,
                g,
                relation,
                &physical(relation.name(), group.epoch()),
                budget,
            )
            .await?;
            vocabulary.insert(relation.name(),frozen);
            sqlx::query("INSERT INTO lctx_model_store.epoch_receipts VALUES($1,$2,$3,$4,$5)")
                .bind(g.0.to_vec())
                .bind(epoch)
                .bind(relation.name())
                .bind(i64::try_from(frozen.rows).map_err(|_| Error::Contract)?)
                .bind(frozen.content.0.to_vec())
                .execute(&mut *tx)
                .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT SELECT ON {} TO lctx_importer",
                qualified(g, &group.epoch().view(relation.name()))
            )))
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE lctx_model_store.publication_groups SET closed=true WHERE generation_id=$1 AND epoch=$2").bind(g.0.to_vec()).bind(epoch).execute(&mut *tx).await?;
        Ok((outputs,vocabulary))
    }
}
pub(super) fn physical(name: &str, epoch: VocabularyEpoch) -> String {
    if is_vocabulary(name) {
        epoch.view(name)
    } else {
        name.to_owned()
    }
}
pub(super) async fn receipt(
    tx: &mut PgConnection,
    g: GenerationId,
    relation: &Relation,
    physical: &str,
    budget: &ResourceBudget,
) -> Result<RelationReceipt, Error> {
    let mut content = relation.content();
    visit_named(tx, g, relation, physical, &["id"], budget, |batch| {
        relation.hash_rows(&batch, &mut content)?;
        Ok(())
    })
    .await?;
    let (rows, content) = content.finish();
    Ok(RelationReceipt { rows, content })
}
