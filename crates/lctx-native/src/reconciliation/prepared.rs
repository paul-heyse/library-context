//! Retained expectations earned while the actual admitted publication lowers its roles.
use super::*;
use crate::ordered_rows::{PreparedRows, SortedRows};
use lctx_model::domain::{
    ContentHash,
    graph::FamilyContent,
    resources::{Reservation, ResourceBudget},
};
use std::sync::Arc;

struct Context {
    owner: Arc<()>,
    client: Arc<crate::surrealdb::Surreal<crate::surrealdb::engine::remote::grpc::Client>>,
    source_client: Arc<crate::surrealdb::Surreal<crate::surrealdb::engine::remote::grpc::Client>>,
    staging: Arc<crate::control::staging::DerivedStaging>,
    pin: Arc<crate::control::ReaderPin>,
    budget: ResourceBudget,
    realization: ContentHash,
    scope: ContentHash,
    _charge: Box<dyn Reservation>,
}
impl Context {
    fn new(loader: &Loader) -> Result<Self, ModelError> {
        let reader = loader.reader();
        reader.require_installed_scope()?;
        let budget = loader.read_budget()?;
        Ok(Self {
            owner: loader.search_owner().clone(),
            client: loader.shared_client(),
            source_client: reader.shared_client(),
            staging: loader
                .staging()
                .ok_or(ModelError::Conflict("prepared roles staging"))?
                .clone(),
            pin: reader
                .scope_pin()
                .ok_or(ModelError::Conflict("prepared roles source pin"))?,
            realization: loader.derivation()?,
            scope: crate::lexical_stats::scope_identity(&reader)?,
            _charge: budget.reserve("retained publication role context", size_of::<Self>())?,
            budget,
        })
    }
    fn check(&self, loader: &Loader) -> Result<(), ModelError> {
        loader.check_read_admission()?;
        let reader = loader.reader();
        reader.ensure_scope_live()?;
        self.pin.ensure_live()?;
        if !Arc::ptr_eq(&self.owner, loader.search_owner())
            || !Arc::ptr_eq(&self.client, &loader.shared_client())
            || !Arc::ptr_eq(&self.source_client, &reader.shared_client())
            || !loader
                .staging()
                .is_some_and(|owner| Arc::ptr_eq(owner, &self.staging))
            || !reader
                .scope_pin()
                .is_some_and(|pin| Arc::ptr_eq(&pin, &self.pin))
            || !self.budget.shares_pool(&loader.read_budget()?)
            || self.realization != loader.derivation()?
            || self.scope != crate::lexical_stats::scope_identity(&reader)?
        {
            return Err(ModelError::Conflict(
                "prepared roles protected context changed",
            ));
        }
        Ok(())
    }
}
/// Mutable construction is not authority. Only successful complete lowering can finish it.
pub struct ReferencePreparation {
    context: Context,
    canonical: SortedRows,
    roles: SortedRows,
    entities: FamilyHasher,
    assertions: FamilyHasher,
    entity_pending: crate::loader::ReferenceWrites,
    assertion_pending: crate::loader::ReferenceWrites,
}
impl ReferencePreparation {
    pub fn new(loader: &Loader) -> Result<Self, ModelError> {
        let context = Context::new(loader)?;
        Ok(Self {
            canonical: SortedRows::with_budget_and_row_bytes(
                &context.budget,
                lctx_model::domain::resources::MAX_ROW_BYTES,
            )?,
            roles: SortedRows::with_budget(&context.budget)?,
            entities: FamilyHasher::new(GraphFamily::Entities),
            assertions: FamilyHasher::new(GraphFamily::Assertions),
            context,
            entity_pending: Default::default(),
            assertion_pending: Default::default(),
        })
    }
    pub async fn entities(&mut self, loader: &Loader, rows: &[Entity]) -> Result<(), ModelError> {
        self.context.check(loader)?;
        for (row, view) in rows.iter().zip(codec::entity_views(rows)?) {
            row.validate().map_err(crate::reader::canonical_error)?;
            if !self.entities.push(row.id().0, row.content())? {
                return Err(ModelError::Conflict("prepared canonical entity duplicate"));
            }
            self.canonical.push(crate::adapter::physical_row_prepared(
                crate::loader::entity_payload_id(row)?,
                row.content(),
                serde_json::to_vec(row).map_err(ModelError::codec)?,
                Some(crate::adapter::GraphEnvelope::entity(row)),
                view,
            )?)?;
        }
        loader
            .entity_references_inner(rows, Some(&mut self.roles), &mut self.entity_pending)
            .await?;
        self.context.check(loader)
    }
    pub async fn assertions(
        &mut self,
        loader: &Loader,
        rows: &[Assertion],
    ) -> Result<(), ModelError> {
        self.context.check(loader)?;
        for (row, view) in rows.iter().zip(codec::assertion_views(rows)?) {
            row.validate().map_err(crate::reader::canonical_error)?;
            if !self.assertions.push(row.id().0, row.content())? {
                return Err(ModelError::Conflict(
                    "prepared canonical assertion duplicate",
                ));
            }
            self.canonical.push(crate::adapter::physical_row_prepared(
                crate::loader::assertion_payload_id(row)?,
                row.content(),
                serde_json::to_vec(row).map_err(ModelError::codec)?,
                Some(crate::adapter::GraphEnvelope::assertion(row)),
                view,
            )?)?;
        }
        loader
            .assertion_references_inner(rows, Some(&mut self.roles), &mut self.assertion_pending)
            .await?;
        self.context.check(loader)
    }
    pub async fn finish(
        mut self,
        loader: &Loader,
        manifest: &Manifest,
    ) -> Result<PreparedRoles, ModelError> {
        self.context.check(loader)?;
        manifest.validate()?;
        loader
            .flush_references("reference", &mut self.entity_pending)
            .await?;
        loader
            .flush_references("participant", &mut self.assertion_pending)
            .await?;
        self.context.check(loader)?;
        let families = [self.entities.finish(), self.assertions.finish()];
        if families
            .iter()
            .any(|family| !manifest.families.contains(family))
        {
            return Err(ModelError::Conflict(
                "prepared canonical family reconciliation",
            ));
        }
        Ok(PreparedRoles {
            context: self.context,
            families,
            canonical: self.canonical.finish()?.into_prepared(),
            roles: self.roles.finish()?.into_prepared(),
        })
    }
}
/// Private same-operation preparation; incoming transports cannot construct or serialize it.
pub struct PreparedRoles {
    context: Context,
    families: [FamilyContent; 2],
    canonical: PreparedRows,
    pub(super) roles: PreparedRows,
}
impl PreparedRoles {
    pub(super) async fn reconcile_canonical(
        &self,
        loader: &Loader,
        manifest: &Manifest,
    ) -> Result<(), ModelError> {
        self.context.check(loader)?;
        if self
            .families
            .iter()
            .any(|family| !manifest.families.contains(family))
        {
            return Err(ModelError::Conflict("prepared roles manifest changed"));
        }
        let reader = loader.reader();
        let budget = self.context.budget.clone();
        let mut actual =
            crate::reader::NativeRows::owned(budget.clone(), move |sender| async move {
                let mut actual = SortedRows::with_budget_and_row_bytes(
                    &budget,
                    lctx_model::domain::resources::MAX_ROW_BYTES,
                )?;
                for table in ["entity", "assertion"] {
                    let mut rows = reader.selected_payload_rows(
                        table,
                        "true",
                        Variables::new(),
                        vec![],
                        "anchor,id",
                        None,
                    )?;
                    while let Some(row) = rows.next().await? {
                        actual.push(row)?;
                    }
                }
                let mut rows = actual.finish()?;
                while let Some(row) = rows.next_row()? {
                    if !sender.send(row).await? {
                        break;
                    }
                }
                Ok(())
            })?;
        self.canonical.cursor()?.reconcile(&mut actual).await?;
        self.context.check(loader)
    }
}
