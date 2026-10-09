//! Shared rich hydration and borrowed typed partitions over an exact prepared root batch.
use crate::{
    consumed_rows::{PreparedRootBatch, stream_batches},
    workspace::Cancellation,
};
use lctx_model::domain::{
    charged::{ChargedVec, StateCharge},
    normalized::{Rows, callable_aspects::RowsView},
    resources::ResourceBudget,
    *,
};
use serde::Deserialize;
use std::any::TypeId;

/// Hydrate each declared namespace once. The semantic owner chooses declarations (including
/// first-epoch versus multiple-prefix cardinality) and decodes into its existing charged Rows.
/// Prepared bindings are immutable; this helper does not acquire completed-input authority.
///
/// Cancellation before planning submits nothing. Cancellation or callback refusal during a
/// stream drops the physical stream: existing CompilerRows drainage retains native admission,
/// scratch and reservations until submitted transport is terminal. The enclosing native owner
/// still performs its existing quiescence/completion; this helper never closes that owner.
pub(crate) async fn hydrate_union(
    batch: &PreparedRootBatch,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
    visit: &mut (
             dyn FnMut(usize, &ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>
                 + Send
         ),
) -> Result<(), ModelError> {
    hydrate_union_inner(batch, inputs, budget, cancellation, false, visit).await
}
pub(crate) async fn hydrate_union_first(
    batch: &PreparedRootBatch,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
    visit: &mut (
             dyn FnMut(usize, &ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>
                 + Send
         ),
) -> Result<(), ModelError> {
    hydrate_union_inner(batch, inputs, budget, cancellation, true, visit).await
}
async fn hydrate_union_inner(
    batch: &PreparedRootBatch,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
    first_only: bool,
    visit: &mut (
             dyn FnMut(usize, &ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>
                 + Send
         ),
) -> Result<(), ModelError> {
    // Refuse all nominal mismatches before starting any physical demand.
    for (table, input) in inputs.iter().enumerate() {
        let relation = batch.relation(table)?;
        if relation.type_id() != input.type_id() || relation.name() != input.name() {
            return Err(ModelError::Conflict("batched hydration nominal binding"));
        }
    }
    for (table, input) in inputs.iter().enumerate() {
        if first_only
            && inputs[..table]
                .iter()
                .any(|prior| prior.type_id() == input.type_id())
        {
            continue;
        }
        cancellation.check()?;
        let selected = batch.union.select(table)?;
        let mut consume = |rows: &arrow_array::RecordBatch| {
            cancellation.check()?;
            let _transfer =
                budget.reserve("batched-union-transfer", rows.get_array_memory_size())?;
            visit(table, input, rows)
        };
        stream_batches(input, batch.union.session(), &selected, None, &mut consume).await?;
    }
    Ok(())
}

/// Charged IDs borrow the one hydrated union, so selected kernels never copy rich rows.
/// Absent nominal references remain in PreparedRootBatch's memberships and root outcomes;
/// this actual-row view has the same realization as the old nominal-key/source table join.
pub(crate) struct SelectedRows<'a, R: Record> {
    rows: &'a Rows<R>,
    ids: ChargedVec<Id<R>>,
    _charge: StateCharge,
}
impl<'a, R: Record> SelectedRows<'a, R> {
    pub(crate) fn new(
        batch: &PreparedRootBatch,
        partition: usize,
        table: usize,
        rows: &'a Rows<R>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if batch.relation(table)?.type_id() != TypeId::of::<R>() {
            return Err(ModelError::Conflict(
                "batched row selection nominal binding",
            ));
        }
        let mut charge = StateCharge::new(budget, "batched-typed-row-selection");
        let mut ids = ChargedVec::default();
        for key in batch.keys(partition, table)? {
            // Nominal decoding uses the fixed-width Serde visitor without a JSON allocation.
            let id = Id::<R>::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new(key.into_iter()))
            .map_err(ModelError::codec)?;
            if rows.get(id).is_some() {
                ids.push(&mut charge, id)?;
            }
        }
        Ok(Self {
            rows,
            ids,
            _charge: charge,
        })
    }
    pub(crate) fn view(&self) -> Result<RowsView<'_, R>, ModelError> {
        RowsView::selected(self.rows, &self.ids)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::consumed_rows::{ClosureTable, NominalClosure, PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use std::sync::Arc;

    #[tokio::test]
    async fn union_decodes_once_and_partitions_borrow_identical_rich_rows() {
        let session = SessionContext::new();
        let package = input::Package {
            name: "shared".into(),
        };
        let encoded = input::Package::encode(std::slice::from_ref(&package)).unwrap();
        session
            .register_table(
                "packages",
                Arc::new(MemTable::try_new(encoded.schema(), vec![vec![encoded]]).unwrap()),
            )
            .unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let prepared = NominalClosure::new(vec![ClosureTable {
            relation: Relation::of::<input::Package>(),
            alias: "packages".into(),
        }])
        .unwrap()
        .prepare(&session, &budget)
        .await
        .unwrap();
        let root = PreparedRoot {
            table: 0,
            key: *package.id().bytes(),
            kind: PreparedRootKind::Physical,
        };
        let batch = prepared
            .batch(
                &[
                    root,
                    root,
                    PreparedRoot {
                        table: 0,
                        key: [254; 16],
                        kind: PreparedRootKind::Virtual,
                    },
                ],
                &budget,
            )
            .await
            .unwrap();
        let mut rows = Rows::<input::Package>::new(&budget);
        let inputs = [ValidationInput::of::<input::Package>(&["id"])];
        let mut decoded = 0;
        hydrate_union(
            &batch,
            &inputs,
            &budget,
            &Cancellation::default(),
            &mut |_, _, batch| {
                decoded += batch.num_rows();
                rows.decode(batch)
            },
        )
        .await
        .unwrap();
        assert_eq!(decoded, 1);
        let a = SelectedRows::new(&batch, 0, 0, &rows, &budget).unwrap();
        let b = SelectedRows::new(&batch, 1, 0, &rows, &budget).unwrap();
        assert!(std::ptr::eq(
            a.view().unwrap().get(package.id()).unwrap(),
            b.view().unwrap().get(package.id()).unwrap()
        ));
        let absent = SelectedRows::new(&batch, 2, 0, &rows, &budget).unwrap();
        assert!(absent.view().unwrap().is_empty());
        assert_eq!(
            batch.keys(2, 0).unwrap().count(),
            1,
            "absent nominal demand remains explicit"
        );
        assert!(
            SelectedRows::<input::Release>::new(&batch, 0, 0, &Rows::new(&budget), &budget)
                .is_err()
        );
        let cancellation = Cancellation::default();
        cancellation.cancel();
        let mut visits = 0;
        assert!(
            hydrate_union(&batch, &inputs, &budget, &cancellation, &mut |_, _, _| {
                visits += 1;
                Ok(())
            })
            .await
            .is_err()
        );
        assert_eq!(visits, 0);
    }
}

#[cfg(test)]
mod native_controls {
    use super::*;
    use crate::consumed_rows::{ClosureTable, NominalClosure, PreparedRoot, PreparedRootKind};
    use datafusion::prelude::SessionContext;
    use lctx_model::domain::{
        completed::ContributionSpec,
        completion::{LocalState, RemoteState},
        input::Package,
        stages::{Profile, ProviderOutcome},
    };
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };
    async fn native_packages() -> (
        Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
        ResourceBudget,
        SessionContext,
        crate::consumed_rows::PreparedEdges,
        Vec<Package>,
    ) {
        let store = crate::test_native::store();
        let relation = Relation::of::<Package>();
        let mut rows = vec![
            Package {
                name: "first".into(),
            },
            Package {
                name: "second".into(),
            },
        ];
        rows.sort_by_key(Record::id);
        let contribution = store
            .begin_contribution(ContributionSpec {
                captured_binding: None,
                producer: "batch-hydration-control".into(),
                profile: Profile::Catalog,
                model: ContentHash::of(b"batch-model"),
                implementation: ContentHash::of(b"batch-implementation"),
                configuration: None,
                inputs: vec![],
                outputs: BTreeSet::from([Package::NAME.into()]),
            })
            .await
            .unwrap();
        store
            .write_batch(&contribution, &relation, &Package::encode(&rows).unwrap())
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                contribution,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(),
            )
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let session = SessionContext::new();
        session
            .register_table(
                "packages",
                store
                    .table_provider(&views[Package::NAME], relation.clone(), budget.clone(), 1)
                    .unwrap(),
            )
            .unwrap();
        let edges = NominalClosure::new(vec![ClosureTable {
            relation,
            alias: "packages".into(),
        }])
        .unwrap()
        .prepare(&session, &budget)
        .await
        .unwrap();
        (store, budget, session, edges, rows)
    }
    fn roots(rows: &[Package]) -> Vec<PreparedRoot> {
        rows.iter()
            .map(|row| PreparedRoot {
                table: 0,
                key: *row.id().bytes(),
                kind: PreparedRootKind::Physical,
            })
            .collect()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_hydration_cancellation_preserves_other_consumer_and_terminal_charges() {
        let (store, budget, session, edges, rows) = native_packages().await;
        let selected = edges.batch(&roots(&rows), &budget).await.unwrap();
        let baseline = budget.reserved();
        let inputs = [ValidationInput::of::<Package>(&["id"])];
        let cancelled = Cancellation::default();
        let mut visits = 0;
        let result = hydrate_union(&selected, &inputs, &budget, &cancelled, &mut |_, _, _| {
            visits += 1;
            cancelled.cancel();
            cancelled.check()
        })
        .await;
        assert!(result.is_err());
        assert_eq!(
            visits, 1,
            "actual native submission yielded a batch before refusal"
        );
        let mut healthy = Rows::<Package>::new(&budget);
        hydrate_union(
            &selected,
            &inputs,
            &budget,
            &Cancellation::default(),
            &mut |_, _, batch| healthy.decode(batch),
        )
        .await
        .unwrap();
        assert_eq!(healthy.iter().cloned().collect::<Vec<_>>(), rows);
        drop(healthy);
        let completion = store.drain_report().await;
        assert_eq!(completion.local, LocalState::Terminal);
        assert_eq!(completion.remote, RemoteState::Confirmed);
        assert_eq!(
            budget.reserved(),
            baseline,
            "provisional read resources released only after terminal drainage"
        );
        drop(selected);
        drop(edges);
        drop(session);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn native_hydration_late_corrupt_row_refuses_after_provisional_good_batch() {
        let (store, budget, session, edges, rows) = native_packages().await;
        let selected = edges.batch(&roots(&rows), &budget).await.unwrap();
        let path = std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").unwrap();
        let config = lctx_surrealdb::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        let admin = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.root_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let mut vars = lctx_surrealdb::surrealdb::types::Variables::new();
        vars.insert("key", rows[1].id().hex());
        admin.query("UPDATE entity SET body.name='INVALID' WHERE semantic_type='packages' AND semantic_key=$key RETURN NONE").bind(vars).await.unwrap().check().unwrap();
        let inputs = [ValidationInput::of::<Package>(&["id"])];
        let mut delivered = Rows::<Package>::new(&budget);
        let mut visits = 0;
        let result = hydrate_union(
            &selected,
            &inputs,
            &budget,
            &Cancellation::default(),
            &mut |_, _, batch| {
                visits += 1;
                delivered.decode(batch)
            },
        )
        .await;
        assert!(
            result.is_err(),
            "a late invalid body cannot become an empty successful suffix"
        );
        assert_eq!(
            delivered.len(),
            1,
            "the earlier actual native batch was delivered before late refusal"
        );
        assert!(visits >= 1);
        let completion = store.drain_report().await;
        assert_eq!(completion.local, LocalState::Terminal);
        assert_eq!(completion.remote, RemoteState::Confirmed);
        drop(delivered);
        drop(selected);
        drop(edges);
        drop(session);
        drop(admin);
        assert_eq!(budget.reserved(), 0);
    }
}
