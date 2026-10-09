//! Portable execution admission selects each actual owner before rich decoding.
use crate::{
    consumed_rows::{
        ClosureTable, PreparedEdges, PreparedRoot, PreparedRootBatch, PreparedRootKind, identifier,
    },
    scoped_admission::column,
    workspace::Cancellation,
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    charged::{ChargedSet, StateCharge},
    execution::fidelity::ExecutionScope,
    resources::ResourceBudget,
    scope_program::ScopeInterner,
    *,
};

pub(crate) async fn validate_execution(
    invariant: &Invariant,
    scope: &ExecutionScope,
    tables: Vec<ClosureTable>,
    model: &ValidatedModel,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let declared = scope.program(invariant.inputs.clone(), model, budget)?;
    let root = declared.root;
    let root_namespace = declared.root_namespace;
    let mut interner = ScopeInterner::new(budget)?;
    let program = interner.intern(declared.program, model)?;
    let prepared = crate::scope_compilation::lower_compiled(
        crate::scope_compilation::compile(program.program(), model, budget, None)?,
        &tables,
        &scope_program::ScopeParameters(vec![]),
        budget,
    )?
    .prepare(session, budget)
    .await?;
    let _roots = budget.reserve("execution-fidelity-root", ROOT_ROWS * 128)?;
    let sql = format!(
        "SELECT id FROM {} ORDER BY id",
        identifier(&tables[root].alias)
    );
    let mut roots = crate::sql::query(session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    let mut window = Vec::with_capacity(ROOT_ROWS);
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            window.push(
                column(&batch, "id", row)?.ok_or(ModelError::Schema("execution owner root ID"))?,
            );
            if window.len() == ROOT_ROWS {
                validate_window(
                    invariant,
                    root,
                    root_namespace,
                    &prepared,
                    &window,
                    budget,
                    cancellation,
                )
                .await?;
                window.clear();
            }
        }
    }
    if !window.is_empty() {
        validate_window(
            invariant,
            root,
            root_namespace,
            &prepared,
            &window,
            budget,
            cancellation,
        )
        .await?;
    }
    Ok(())
}

const ROOT_ROWS: usize = 128;
struct ColumnarInputs {
    rows: Vec<Vec<arrow_array::RecordBatch>>,
    _charge: StateCharge,
}
/// Physical owner input and the private namespace used to isolate its selected root.
struct RootBinding {
    input: usize,
    namespace: usize,
}
async fn hydrate(
    batch: &PreparedRootBatch,
    inputs: &[ValidationInput],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<ColumnarInputs, ModelError> {
    let mut charge = StateCharge::new(budget, "execution-admission-columnar-union");
    charge.grow(
        inputs
            .len()
            .saturating_mul(size_of::<Vec<arrow_array::RecordBatch>>())
            .saturating_add(4096),
    )?;
    let mut rows = (0..inputs.len()).map(|_| Vec::new()).collect::<Vec<_>>();
    crate::scoped_batch::hydrate_union(
        batch,
        inputs,
        budget,
        cancellation,
        &mut |index, _, batch| {
            let target = &mut rows[index];
            let additional = if target.len() == target.capacity() {
                target.capacity().max(4)
            } else {
                0
            };
            charge.grow(batch.get_array_memory_size().saturating_add(
                additional.saturating_mul(size_of::<arrow_array::RecordBatch>()),
            ))?;
            target.reserve_exact(additional);
            target.push(batch.clone());
            Ok(())
        },
    )
    .await?;
    Ok(ColumnarInputs {
        rows,
        _charge: charge,
    })
}
fn visit_partition(
    invariant: &Invariant,
    root: &RootBinding,
    batch: &PreparedRootBatch,
    partition: usize,
    union: &ColumnarInputs,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let mut check = (invariant.create)(budget);
    for (index, input) in invariant.inputs.iter().enumerate() {
        let table = if index == root.input {
            root.namespace
        } else {
            index
        };
        let mut charge = StateCharge::new(budget, "execution-admission-partition-keys");
        let mut keys = ChargedSet::default();
        for key in batch.keys(partition, table)? {
            keys.insert(&mut charge, key)?;
        }
        for rows in &union.rows[index] {
            cancellation.check()?;
            // Arrow's stable filter preserves the union's declared total order and schema.
            // Reserve mask, filtered arrays and checker handoff while retained union stays live.
            let _filter = budget.reserve(
                "execution-admission-columnar-partition",
                rows.get_array_memory_size()
                    .saturating_add(rows.num_rows().saturating_mul(32))
                    .saturating_add(4096),
            )?;
            let mut mask = Vec::with_capacity(rows.num_rows());
            for row in 0..rows.num_rows() {
                mask.push(column(rows, "id", row)?.is_some_and(|key| keys.contains(&key)));
            }
            let mask = arrow_array::BooleanArray::from(mask);
            let selected = datafusion::arrow::compute::filter_record_batch(rows, &mask)
                .map_err(ModelError::codec)?;
            if selected.num_rows() != 0 {
                check.visit_input(input, &selected)?;
            }
        }
    }
    check.finish()
}
async fn validate_window(
    invariant: &Invariant,
    root: usize,
    root_namespace: usize,
    prepared: &PreparedEdges,
    ids: &[[u8; 16]],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(), ModelError> {
    let roots = ids
        .iter()
        .map(|key| PreparedRoot {
            table: root_namespace,
            key: *key,
            kind: PreparedRootKind::Virtual,
        })
        .collect::<Vec<_>>();
    let batch = prepared
        .batch_with_cancellation(&roots, budget, cancellation)
        .await?;
    let union = hydrate(&batch, &invariant.inputs, budget, cancellation).await?;
    for partition in 0..ids.len() {
        cancellation.check()?;
        visit_partition(
            invariant,
            &RootBinding {
                input: root,
                namespace: root_namespace,
            },
            &batch,
            partition,
            &union,
            budget,
            cancellation,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::consumed_rows::NominalClosure;
    use datafusion::datasource::MemTable;
    use std::sync::{Arc, Mutex};
    struct Check {
        rows: Vec<Id<input::Release>>,
        expected: Arc<Mutex<Vec<Vec<Id<input::Release>>>>>,
    }
    impl InvariantCheck for Check {
        fn visit(
            &mut self,
            name: &str,
            batch: &arrow_array::RecordBatch,
        ) -> Result<(), ModelError> {
            if name == input::Release::NAME {
                self.rows
                    .extend(input::Release::decode(batch)?.iter().map(Record::id));
            }
            Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            self.expected.lock().unwrap().push(self.rows);
            Ok(())
        }
    }
    #[tokio::test]
    async fn execution_columnar_union_preserves_private_root_partitions_and_order() {
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let session = SessionContext::new();
        let package = input::Package {
            name: "shared".into(),
        };
        let releases = [
            input::Release {
                package: package.id(),
                version: "1".into(),
            },
            input::Release {
                package: package.id(),
                version: "2".into(),
            },
        ];
        let package_batch = input::Package::encode(std::slice::from_ref(&package)).unwrap();
        let release_batch = input::Release::encode(&releases).unwrap();
        session
            .register_table(
                "union_packages",
                Arc::new(
                    MemTable::try_new(package_batch.schema(), vec![vec![package_batch]]).unwrap(),
                ),
            )
            .unwrap();
        session
            .register_table(
                "union_releases",
                Arc::new(
                    MemTable::try_new(release_batch.schema(), vec![vec![release_batch]]).unwrap(),
                ),
            )
            .unwrap();
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<input::Release>(),
                alias: "union_releases".into(),
            },
            ClosureTable {
                relation: Relation::of::<input::Package>(),
                alias: "union_packages".into(),
            },
            ClosureTable {
                relation: Relation::of::<input::Release>(),
                alias: "union_releases".into(),
            },
        ];
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        plan.own_reverse(0, "package", 1).unwrap();
        plan.pairs(
            2,
            0,
            "SELECT id AS source_id,id AS target_id FROM union_releases".into(),
        )
        .unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let inputs = vec![
            ValidationInput::of::<input::Release>(&["id"]),
            ValidationInput::of::<input::Package>(&["id"]),
        ];
        let ids = releases.iter().map(Record::id).collect::<Vec<_>>();
        let roots = ids
            .iter()
            .map(|id| PreparedRoot {
                table: 2,
                key: *id.bytes(),
                kind: PreparedRootKind::Virtual,
            })
            .collect::<Vec<_>>();
        let batch = edges
            .batch_with_cancellation(&roots, &budget, &Cancellation::default())
            .await
            .unwrap();
        let union = hydrate(&batch, &inputs, &budget, &Cancellation::default())
            .await
            .unwrap();
        assert_eq!(
            union.rows[0]
                .iter()
                .map(|batch| batch.num_rows())
                .sum::<usize>(),
            2
        );
        assert_eq!(
            union.rows[1]
                .iter()
                .map(|batch| batch.num_rows())
                .sum::<usize>(),
            1
        );
        let expected = Arc::new(Mutex::new(Vec::new()));
        let capture = expected.clone();
        let invariant = Invariant {
            purpose: InvariantPurpose::Admission,
            revision: 1,
            name: "execution_columnar_control",
            inputs,
            create: Arc::new(move |_| {
                Box::new(Check {
                    rows: Vec::new(),
                    expected: capture.clone(),
                })
            }),
        };
        for partition in 0..ids.len() {
            visit_partition(
                &invariant,
                &RootBinding {
                    input: 0,
                    namespace: 2,
                },
                &batch,
                partition,
                &union,
                &budget,
                &Cancellation::default(),
            )
            .unwrap();
        }
        assert_eq!(*expected.lock().unwrap(), vec![vec![ids[0]], vec![ids[1]]]);
        drop(union);
        drop(batch);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
}
