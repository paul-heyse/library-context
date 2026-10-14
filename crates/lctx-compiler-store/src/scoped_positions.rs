//! Operation-local positions over exact ordered, charged Arrow input ports.
//! The preparation borrows its root namespace, backing and owner; callers cannot apply it to
//! another union or reorder its backing. Repeated relation/alias ports stay distinct.
use crate::{admission::Cancellation, consumed_rows::PreparedRootBatch, scoped_admission::column};
use arrow_array::{RecordBatch, UInt64Array};
use lctx_model::domain::{
    ModelError, ValidationInput,
    charged::{ChargedSet, StateCharge},
    resources::ResourceBudget,
};
use std::collections::BTreeMap;

pub(crate) struct ColumnarInputs<'a> {
    roots: &'a PreparedRootBatch,
    inputs: &'a [ValidationInput],
    rows: Vec<Vec<RecordBatch>>,
    charge: StateCharge,
}
impl<'a> ColumnarInputs<'a> {
    pub(crate) fn new(
        roots: &'a PreparedRootBatch,
        inputs: &'a [ValidationInput],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let ports = inputs.len();
        let mut charge = StateCharge::new(budget, "scoped-columnar-union");
        charge.grow(
            ports
                .saturating_mul(size_of::<Vec<RecordBatch>>())
                .saturating_add(4096),
        )?;
        Ok(Self {
            roots,
            inputs,
            rows: (0..ports).map(|_| Vec::new()).collect(),
            charge,
        })
    }
    pub(crate) fn push(&mut self, port: usize, rows: &RecordBatch) -> Result<(), ModelError> {
        let target = self
            .rows
            .get_mut(port)
            .ok_or(ModelError::Schema("scoped input port"))?;
        let additional = if target.len() == target.capacity() {
            target.capacity().max(4)
        } else {
            0
        };
        self.charge.grow(
            rows.get_array_memory_size()
                .saturating_add(additional.saturating_mul(size_of::<RecordBatch>())),
        )?;
        target.reserve_exact(additional);
        target.push(rows.clone());
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn rows(&self, port: usize) -> &[RecordBatch] {
        &self.rows[port]
    }
}

type Position = (usize, usize);
type PortIndex = BTreeMap<[u8; 16], Vec<Position>>;
pub(crate) struct PreparedPositions<'a> {
    roots: &'a PreparedRootBatch,
    inputs: &'a [ValidationInput],
    union: &'a ColumnarInputs<'a>,
    budget: &'a ResourceBudget,
    ports: Vec<PortIndex>,
    _charge: StateCharge,
}
impl<'a> PreparedPositions<'a> {
    pub(crate) fn require_owner(&self, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict("scoped positions resource owner"));
        }
        Ok(())
    }
    pub(crate) fn new(
        roots: &'a PreparedRootBatch,
        inputs: &'a [ValidationInput],
        union: &'a ColumnarInputs<'a>,
        budget: &'a ResourceBudget,
        cancellation: &Cancellation,
    ) -> Result<Self, ModelError> {
        cancellation.check()?;
        if union
            .charge
            .budget()
            .is_none_or(|owner| !owner.shares_pool(budget))
        {
            return Err(ModelError::Conflict("scoped positions resource owner"));
        }
        if !std::ptr::eq(roots, union.roots)
            || !std::ptr::eq(inputs, union.inputs)
            || inputs.len() != union.rows.len()
        {
            return Err(ModelError::Conflict("scoped positions input ports"));
        }
        let (ports, charge) = prepare_positions(roots, inputs, &union.rows, budget, cancellation)?;
        Ok(Self {
            roots,
            inputs,
            union,
            budget,
            ports,
            _charge: charge,
        })
    }

    /// Union the requested root memberships as positions, retaining every physical duplicate.
    /// Sorting positions restores source batch/row order independently of nominal ID order.
    pub(crate) fn gather(
        &self,
        partitions: &[usize],
        port: usize,
        namespace: usize,
        cancellation: &Cancellation,
    ) -> Result<GatheredRows<'_>, ModelError> {
        gather_positions(
            self.roots,
            self.inputs,
            &self.union.rows,
            &self.ports,
            self.budget,
            partitions,
            port,
            namespace,
            cancellation,
            &mut |_, _| true,
        )
    }
}

fn prepare_positions(
    roots: &PreparedRootBatch,
    inputs: &[ValidationInput],
    rows: &[Vec<RecordBatch>],
    budget: &ResourceBudget,
    cancellation: &Cancellation,
) -> Result<(Vec<PortIndex>, StateCharge), ModelError> {
    cancellation.check()?;
    roots.require_owner(budget)?;
    if inputs.len() != rows.len() {
        return Err(ModelError::Conflict("scoped positions input ports"));
    }
    let mut charge = StateCharge::new(budget, "scoped-position-index");
    charge.grow(inputs.len().saturating_mul(size_of::<PortIndex>()))?;
    let mut ports = Vec::with_capacity(inputs.len());
    for (port, input) in inputs.iter().enumerate() {
        let relation = roots.relation(port)?;
        if relation.type_id() != input.type_id() || relation.name() != input.name() {
            return Err(ModelError::Conflict("scoped positions nominal port"));
        }
        let mut index = PortIndex::new();
        for (batch, rows) in rows[port].iter().enumerate() {
            cancellation.check()?;
            if rows.schema().as_ref() != relation.schema().as_ref() {
                return Err(ModelError::Schema("scoped positions input schema"));
            }
            for row in 0..rows.num_rows() {
                let Some(id) = column(rows, "id", row)? else {
                    continue;
                };
                // Reserve B-tree node allowance plus vector capacity before either grows.
                if !index.contains_key(&id) {
                    charge.grow(size_of::<[u8; 16]>() + size_of::<Vec<Position>>() + 32)?;
                }
                let positions = index.entry(id).or_default();
                let additional = if positions.len() == positions.capacity() {
                    positions.capacity().max(4)
                } else {
                    0
                };
                charge.grow(additional.saturating_mul(size_of::<Position>()))?;
                positions.reserve_exact(additional);
                positions.push((batch, row));
            }
        }
        ports.push(index);
    }
    Ok((ports, charge))
}

fn gather_positions<'a>(
    roots: &'a PreparedRootBatch,
    inputs: &'a [ValidationInput],
    backing: &'a [Vec<RecordBatch>],
    ports: &[PortIndex],
    budget: &ResourceBudget,
    partitions: &[usize],
    port: usize,
    namespace: usize,
    cancellation: &Cancellation,
    include: &mut dyn FnMut(usize, [u8; 16]) -> bool,
) -> Result<GatheredRows<'a>, ModelError> {
    cancellation.check()?;
    let input = inputs
        .get(port)
        .ok_or(ModelError::Schema("scoped gather input port"))?;
    let source_relation = roots.relation(port)?;
    let namespace_relation = roots.relation(namespace)?;
    if namespace_relation.type_id() != input.type_id()
        || namespace_relation.name() != input.name()
        || namespace_relation.schema() != source_relation.schema()
    {
        return Err(ModelError::Conflict("scoped gather root namespace"));
    }
    let mut selection_charge = StateCharge::new(budget, "scoped-selected-positions");
    let mut positions = ChargedSet::default();
    for partition in partitions {
        cancellation.check()?;
        for key in roots.keys(*partition, namespace)? {
            cancellation.check()?;
            if !include(*partition, key) {
                continue;
            }
            if let Some(matches) = ports[port].get(&key) {
                for chunk in matches.chunks(lctx_model::domain::resources::TRANSFER_ROWS) {
                    cancellation.check()?;
                    for position in chunk {
                        positions.insert(&mut selection_charge, *position)?;
                    }
                }
            }
        }
    }
    let mut charge = StateCharge::new(budget, "scoped-gathered-rows");
    // Include both retained batch handles and the handles handed to a scoped provider.
    charge.grow(
        backing[port]
            .len()
            .saturating_mul(size_of::<RecordBatch>())
            .saturating_mul(2),
    )?;
    let mut rows = Vec::with_capacity(backing[port].len());
    let mut ordered = positions.iter().copied().peekable();
    while let Some((batch, _)) = ordered.peek().copied() {
        cancellation.check()?;
        let source = &backing[port][batch];
        let count = positions.range((batch, 0)..=(batch, usize::MAX)).count();
        let _indices = budget.reserve(
            "scoped-gather-indices",
            count.saturating_mul(size_of::<u64>()).saturating_add(128),
        )?;
        let mut indices = Vec::with_capacity(count);
        while let Some((next, row)) = ordered.peek().copied() {
            if next != batch {
                break;
            }
            ordered.next();
            indices.push(row as u64);
        }
        let indices = UInt64Array::from(indices);
        // Source, index and selected arrays overlap. The full source allowance bounds a
        // subset gather, including kernels that retain source backing for view arrays.
        charge.grow(
            source
                .get_array_memory_size()
                .saturating_add(count.saturating_mul(32))
                .saturating_add(4096),
        )?;
        let selected = datafusion::arrow::compute::take_record_batch(source, &indices)
            .map_err(ModelError::codec)?;
        rows.push(selected);
    }
    Ok(GatheredRows {
        rows,
        _charge: charge,
        _roots: roots,
        _inputs: inputs,
        _backing: backing,
    })
}

/// Immutable owning counterpart for compiler windows that retain their actual root batch
/// and hydrated columns. Private positions cannot be rebound to another backing or namespace.
pub struct IndexedRootColumns {
    roots: PreparedRootBatch,
    inputs: Vec<ValidationInput>,
    rows: Vec<Vec<RecordBatch>>,
    ports: Vec<PortIndex>,
    budget: ResourceBudget,
    _backing_charge: StateCharge,
    _preparation_charge: StateCharge,
}
impl IndexedRootColumns {
    pub fn new(
        roots: PreparedRootBatch,
        inputs: &[ValidationInput],
        rows: Vec<Vec<RecordBatch>>,
        backing_charge: StateCharge,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
    ) -> Result<Self, ModelError> {
        cancellation.check()?;
        if backing_charge
            .budget()
            .is_none_or(|owner| !owner.shares_pool(budget))
        {
            return Err(ModelError::Conflict("scoped positions resource owner"));
        }
        let backing_bytes = rows.iter().fold(
            rows.capacity()
                .saturating_mul(size_of::<Vec<RecordBatch>>()),
            |bytes, batches| {
                batches.iter().fold(
                    bytes.saturating_add(
                        batches.capacity().saturating_mul(size_of::<RecordBatch>()),
                    ),
                    |bytes, batch| bytes.saturating_add(batch.get_array_memory_size()),
                )
            },
        );
        if backing_charge.reserved() < backing_bytes {
            return Err(ModelError::Conflict("scoped positions backing reservation"));
        }
        let (ports, mut preparation_charge) =
            prepare_positions(&roots, inputs, &rows, budget, cancellation)?;
        preparation_charge.grow(
            inputs
                .len()
                .saturating_mul(size_of::<ValidationInput>())
                .saturating_add(
                    inputs
                        .iter()
                        .map(|input| input.order().len().saturating_mul(size_of::<&str>()))
                        .sum::<usize>(),
                ),
        )?;
        let inputs = inputs.to_vec();
        Ok(Self {
            roots,
            inputs,
            rows,
            ports,
            budget: budget.clone(),
            _backing_charge: backing_charge,
            _preparation_charge: preparation_charge,
        })
    }
    pub fn roots(&self) -> &PreparedRootBatch {
        &self.roots
    }
    pub fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    /// Filter only already selected nominal keys; duplicate physical rows and source order
    /// remain intact. The result borrows this exact immutable root/backing owner.
    pub fn gather(
        &self,
        partitions: &[usize],
        port: usize,
        namespace: usize,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
        include: &mut dyn FnMut(usize, [u8; 16]) -> bool,
    ) -> Result<GatheredRows<'_>, ModelError> {
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict("scoped positions resource owner"));
        }
        gather_positions(
            &self.roots,
            &self.inputs,
            &self.rows,
            &self.ports,
            budget,
            partitions,
            port,
            namespace,
            cancellation,
            include,
        )
    }
}

/// A gather cannot outlive its exact indexed backing, and its charge covers provider clones.
pub struct GatheredRows<'a> {
    rows: Vec<RecordBatch>,
    _charge: StateCharge,
    _roots: &'a PreparedRootBatch,
    _inputs: &'a [ValidationInput],
    _backing: &'a [Vec<RecordBatch>],
}
impl GatheredRows<'_> {
    pub fn rows(&self) -> &[RecordBatch] {
        &self.rows
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::consumed_rows::{ClosureTable, NominalClosure, PreparedRoot, PreparedRootKind};
    use datafusion::{datasource::MemTable, prelude::SessionContext};
    use lctx_model::domain::{Record, Relation, input::Package};
    use std::sync::Arc;

    fn names(rows: &GatheredRows<'_>) -> Vec<String> {
        rows.rows()
            .iter()
            .flat_map(|batch| Package::decode(batch).unwrap())
            .map(|row| row.name)
            .collect()
    }

    #[tokio::test]
    async fn exact_positions_keep_overlap_duplicates_port_namespace_order_and_lifetimes() {
        let session = SessionContext::new();
        let packages = ["a", "b", "c"].map(|name| Package { name: name.into() });
        let encoded = Package::encode(&packages).unwrap();
        session
            .register_table(
                "position_packages",
                Arc::new(MemTable::try_new(encoded.schema(), vec![vec![encoded]]).unwrap()),
            )
            .unwrap();
        // The same relation and physical alias has three independent nominal ports.
        let mut plan = NominalClosure::new(
            (0..3)
                .map(|_| ClosureTable {
                    relation: Relation::of::<Package>(),
                    alias: "position_packages".into(),
                })
                .collect(),
        )
        .unwrap();
        plan.pairs(2, 0, "SELECT a.id AS source_id,b.id AS target_id FROM position_packages a CROSS JOIN position_packages b WHERE (a.name='a' AND b.name IN ('a','b')) OR (a.name='b' AND b.name IN ('b','c'))".into()).unwrap();
        plan.pairs(2, 1, "SELECT a.id AS source_id,b.id AS target_id FROM position_packages a CROSS JOIN position_packages b WHERE (a.name='a' AND b.name='c') OR (a.name='b' AND b.name='a')".into()).unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let cancellation = Cancellation::default();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let root = |key| PreparedRoot {
            table: 2,
            key,
            kind: PreparedRootKind::Virtual,
        };
        let roots = edges
            .batch(
                &[
                    root(*packages[0].id().bytes()),
                    root(*packages[1].id().bytes()),
                    root(*packages[0].id().bytes()),
                    root([255; 16]),
                ],
                &budget,
            )
            .await
            .unwrap();
        let inputs = vec![
            ValidationInput::of::<Package>(&["id"]),
            ValidationInput::of::<Package>(&["id"]),
        ];
        let mut union = ColumnarInputs::new(&roots, &inputs, &budget).unwrap();
        union
            .push(
                0,
                &Package::encode(&[packages[2].clone(), packages[1].clone()]).unwrap(),
            )
            .unwrap();
        union
            .push(
                0,
                &Package::encode(&[packages[0].clone(), packages[1].clone()]).unwrap(),
            )
            .unwrap();
        union
            .push(
                1,
                &Package::encode(&[
                    packages[1].clone(),
                    packages[2].clone(),
                    packages[0].clone(),
                ])
                .unwrap(),
            )
            .unwrap();
        let backing_usage = budget.reserved();
        let positions =
            PreparedPositions::new(&roots, &inputs, &union, &budget, &cancellation).unwrap();
        let indexed_usage = budget.reserved();
        assert!(indexed_usage > backing_usage);
        let other_inputs = inputs.clone();
        assert!(
            PreparedPositions::new(&roots, &other_inputs, &union, &budget, &cancellation).is_err(),
            "another input declaration cannot reuse the indexed union"
        );
        let other_roots = edges
            .batch(&[root(*packages[1].id().bytes())], &budget)
            .await
            .unwrap();
        assert!(
            PreparedPositions::new(&other_roots, &inputs, &union, &budget, &cancellation).is_err(),
            "another root/order binding cannot reuse the indexed union"
        );
        drop(other_roots);
        assert_eq!(
            names(&positions.gather(&[0], 0, 0, &cancellation).unwrap()),
            ["b", "a", "b"]
        );
        assert_eq!(
            names(&positions.gather(&[1], 0, 0, &cancellation).unwrap()),
            ["c", "b", "b"]
        );
        assert_eq!(
            names(&positions.gather(&[2], 0, 0, &cancellation).unwrap()),
            ["b", "a", "b"],
            "duplicate roots retain distinct partitions"
        );
        assert_eq!(
            names(&positions.gather(&[0, 1, 0], 0, 0, &cancellation).unwrap()),
            ["c", "b", "a", "b"],
            "same-grain union deduplicates positions, not duplicate IDs"
        );
        assert_eq!(
            names(&positions.gather(&[0], 1, 1, &cancellation).unwrap()),
            ["c"],
            "repeated physical port has its own membership"
        );
        assert_eq!(
            names(&positions.gather(&[0], 0, 2, &cancellation).unwrap()),
            ["a"],
            "private root namespace excludes reachable owners"
        );
        assert!(
            positions
                .gather(&[], 0, 0, &cancellation)
                .unwrap()
                .rows()
                .is_empty()
        );
        assert!(
            positions
                .gather(&[3], 0, 0, &cancellation)
                .unwrap()
                .rows()
                .is_empty()
        );
        assert_eq!(
            roots.keys(3, 2).unwrap().count(),
            1,
            "absent nominal root remains explicit"
        );
        assert!(positions.gather(&[4], 0, 0, &cancellation).is_err());
        assert!(positions.gather(&[0], 2, 0, &cancellation).is_err());
        let foreign = ResourceBudget::fixed(16 << 20).unwrap();
        assert!(positions.require_owner(&foreign).is_err());
        assert!(PreparedPositions::new(&roots, &inputs, &union, &foreign, &cancellation).is_err());
        assert_eq!(foreign.reserved(), 0);
        let retained = positions.gather(&[0], 0, 0, &cancellation).unwrap();
        assert!(budget.reserved() > indexed_usage);
        cancellation.cancel();
        assert!(positions.gather(&[0], 0, 0, &cancellation).is_err());
        assert_eq!(names(&retained), ["b", "a", "b"]);
        drop(retained);
        assert_eq!(budget.reserved(), indexed_usage);
        drop(positions);
        assert_eq!(budget.reserved(), backing_usage);
        let cancellation = Cancellation::default();
        let foreign_edges = plan.prepare(&session, &foreign).await.unwrap();
        let foreign_roots = foreign_edges
            .batch(&[root(*packages[0].id().bytes())], &foreign)
            .await
            .unwrap();
        let mut local_backing = StateCharge::new(&budget, "foreign-root-control");
        local_backing.grow(4096).unwrap();
        assert!(matches!(
            IndexedRootColumns::new(
                foreign_roots,
                &inputs[..1],
                vec![vec![]],
                local_backing,
                &budget,
                &cancellation,
            ),
            Err(ModelError::Conflict("selected root resource owner")),
        ));
        drop(foreign_edges);
        assert_eq!(foreign.reserved(), 0);
        assert_eq!(budget.reserved(), backing_usage);
        let uncharged_roots = edges
            .batch(&[root(*packages[0].id().bytes())], &budget)
            .await
            .unwrap();
        let uncharged_rows = vec![vec![Package::encode(&packages).unwrap()]];
        assert!(matches!(
            IndexedRootColumns::new(
                uncharged_roots,
                &inputs[..1],
                uncharged_rows,
                StateCharge::new(&budget, "uncharged-backing-control"),
                &budget,
                &cancellation,
            ),
            Err(ModelError::Conflict("scoped positions backing reservation")),
        ));
        assert_eq!(budget.reserved(), backing_usage);
        let ColumnarInputs { rows, charge, .. } = union;
        let owned =
            IndexedRootColumns::new(roots, &inputs, rows, charge, &budget, &cancellation).unwrap();
        assert_eq!(owned.inputs().len(), 2);
        assert_eq!(
            names(
                &owned
                    .gather(&[0, 1, 0], 0, 0, &budget, &cancellation, &mut |_, _| true)
                    .unwrap()
            ),
            ["c", "b", "a", "b"],
            "owning windows share the same duplicate-preserving source-order gather",
        );
        assert_eq!(
            names(
                &owned
                    .gather(
                        &[0],
                        0,
                        0,
                        &budget,
                        &cancellation,
                        &mut |partition, key| partition != 0 || key != *packages[1].id().bytes()
                    )
                    .unwrap()
            ),
            ["a"],
            "opaque exclusion applies to nominal membership before all matching positions",
        );
        assert_eq!(
            names(
                &owned
                    .gather(&[0], 1, 1, &budget, &cancellation, &mut |_, _| true)
                    .unwrap()
            ),
            ["c"],
            "filtering one input leaves repeated input namespaces intact",
        );
        assert!(
            owned
                .gather(&[3], 0, 0, &budget, &cancellation, &mut |_, _| true)
                .unwrap()
                .rows()
                .is_empty()
        );
        assert!(
            owned
                .gather(&[4], 0, 0, &budget, &cancellation, &mut |_, _| true)
                .is_err()
        );
        assert!(
            owned
                .gather(&[0], 0, 0, &foreign, &cancellation, &mut |_, _| true)
                .is_err()
        );
        assert_eq!(foreign.reserved(), 0);
        let indexed_usage = budget.reserved();
        let retained = owned
            .gather(&[0], 0, 0, &budget, &cancellation, &mut |_, _| true)
            .unwrap();
        assert!(budget.reserved() > indexed_usage);
        cancellation.cancel();
        assert!(
            owned
                .gather(&[0], 0, 0, &budget, &cancellation, &mut |_, _| true)
                .is_err()
        );
        assert_eq!(names(&retained), ["b", "a", "b"]);
        drop(retained);
        assert_eq!(budget.reserved(), indexed_usage);
        drop(owned);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn crossing_cycle_and_isolate_keep_full_topology_separate_from_output_roots() {
        let session = SessionContext::new();
        let packages = ["a", "b", "c", "isolate"].map(|name| Package { name: name.into() });
        let encoded = Package::encode(&packages).unwrap();
        session
            .register_table(
                "cycle_packages",
                Arc::new(MemTable::try_new(encoded.schema(), vec![vec![encoded]]).unwrap()),
            )
            .unwrap();
        let mut plan = NominalClosure::new(
            (0..2)
                .map(|_| ClosureTable {
                    relation: Relation::of::<Package>(),
                    alias: "cycle_packages".into(),
                })
                .collect(),
        )
        .unwrap();
        plan.pairs(
            1,
            0,
            "SELECT id AS source_id,id AS target_id FROM cycle_packages".into(),
        )
        .unwrap();
        plan.pairs(0, 0, "SELECT a.id AS source_id,b.id AS target_id FROM cycle_packages a CROSS JOIN cycle_packages b WHERE (a.name='a' AND b.name='b') OR (a.name='b' AND b.name='c') OR (a.name='c' AND b.name='a')".into()).unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let cancellation = Cancellation::default();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let roots = edges
            .batch(
                &[0, 3].map(|index| PreparedRoot {
                    table: 1,
                    key: *packages[index].id().bytes(),
                    kind: PreparedRootKind::Virtual,
                }),
                &budget,
            )
            .await
            .unwrap();
        let inputs = vec![ValidationInput::of::<Package>(&["id"])];
        let mut union = ColumnarInputs::new(&roots, &inputs, &budget).unwrap();
        union
            .push(
                0,
                &Package::encode(&[
                    packages[3].clone(),
                    packages[2].clone(),
                    packages[0].clone(),
                    packages[1].clone(),
                ])
                .unwrap(),
            )
            .unwrap();
        let positions =
            PreparedPositions::new(&roots, &inputs, &union, &budget, &cancellation).unwrap();
        assert_eq!(
            names(&positions.gather(&[0], 0, 0, &cancellation).unwrap()),
            ["c", "a", "b"],
            "cycle intermediates outside output partition remain available to the checker"
        );
        assert_eq!(
            names(&positions.gather(&[0], 0, 1, &cancellation).unwrap()),
            ["a"],
            "root output does not expand to the topology"
        );
        assert_eq!(
            names(&positions.gather(&[1], 0, 0, &cancellation).unwrap()),
            ["isolate"]
        );
        let incorrect_inputs = vec![ValidationInput::of::<lctx_model::domain::input::Release>(
            &["id"],
        )];
        assert!(
            PreparedPositions::new(&roots, &incorrect_inputs, &union, &budget, &cancellation)
                .is_err()
        );
        let baseline = budget.reserved();
        let occupied = budget
            .reserve("position-refusal-control", budget.limit() - baseline)
            .unwrap();
        assert!(matches!(
            PreparedPositions::new(&roots, &inputs, &union, &budget, &cancellation),
            Err(ModelError::Resource { .. })
        ));
        drop(occupied);
        assert_eq!(
            budget.reserved(),
            baseline,
            "failed preparation releases its index charge"
        );
        cancellation.cancel();
        assert!(PreparedPositions::new(&roots, &inputs, &union, &budget, &cancellation).is_err());
        drop(positions);
        drop(union);
        drop(roots);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
}
