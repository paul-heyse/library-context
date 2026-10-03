//! Scoped canonical packet reads check the attempted relation before querying or iterating.
use super::{Error, GenerationLease};
use lctx_model::domain::{
    serving::mappings::{PacketBinding, PacketOutput},
    *,
};
pub(super) struct PacketLease<'a> {
    pub(super) lease: &'a mut GenerationLease,
    binding: &'static PacketBinding,
}
impl<'a> PacketLease<'a> {
    pub(super) fn new<P: PacketOutput>(lease: &'a mut GenerationLease) -> Self {
        Self {
            lease,
            binding: P::binding(),
        }
    }
    pub(super) fn check_relation(&self, name: &str) -> Result<(), Error> {
        if !self.binding.permits_relation(name) {
            return Err(Error::Contract);
        }
        Ok(())
    }
    fn attempt<R: Record>(&self) -> Result<(), Error> {
        if !self.binding.permits::<R>() {
            return Err(Error::Contract);
        }
        Ok(())
    }
    pub(super) async fn read_ids<R: Record>(&mut self, ids: &[Id<R>]) -> Result<Batch<R>, Error> {
        self.attempt::<R>()?;
        self.lease.read_ids(ids).await
    }
    pub(super) async fn read_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
    ) -> Result<Batch<R>, Error> {
        self.attempt::<R>()?;
        self.lease.read_for(field, ids).await
    }
    pub(super) async fn visit_for<R: Record, T: Record>(
        &mut self,
        field: &'static str,
        ids: &[Id<T>],
        order: &[&str],
        consume: impl FnMut(Batch<R>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.attempt::<R>()?;
        self.lease.visit_for(field, ids, order, consume).await
    }
}

#[cfg(all(test, feature = "testing"))]
mod tests {
    use super::*;
    use crate::{
        generations::{GenerationReader, GenerationStore},
        testing::{DisposableDatabase, Harness},
    };
    use std::sync::Arc;
    #[tokio::test]
    async fn empty_attempted_undeclared_read_refuses_before_canonical_query() {
        let db = DisposableDatabase::start().await;
        db.migrate().await;
        let model = Arc::new(model().unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
        let mut attempt = Harness::begin_empty_conformance(
            &store,
            db.writer.clone(),
            stages::Profile::Catalog,
            budget.clone(),
            vec![],
        )
        .await
        .unwrap();
        attempt.seal().await.unwrap();
        attempt.validate(&budget).await.unwrap();
        attempt.publish().await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        db.write_configs(dir.path()).unwrap();
        let role =
            crate::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
        let reader = GenerationReader::connect(model, &role).await.unwrap();
        let guard = reader.guard(attempt.generation(), budget).await.unwrap();
        {
            let mut locked = guard.state.lease.lock().await;
            let lease = locked.as_mut().unwrap();
            let mut scoped = PacketLease::new::<serving::OperationCore>(lease);
            assert!(
                matches!(
                    scoped.read_ids::<retrieval::Fragment>(&[]).await,
                    Err(Error::Contract)
                ),
                "an empty ID set still constitutes an undeclared attempted read"
            );
            assert!(
                scoped
                    .read_ids::<input::Package>(&[])
                    .await
                    .unwrap()
                    .rows()
                    .is_empty()
            );
        }
        guard.check().await.unwrap();
        guard.release().await.unwrap();
        reader.close().await;
    }
}
