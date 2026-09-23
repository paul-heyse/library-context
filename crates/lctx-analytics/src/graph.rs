//! The projection → graph adapter (DESIGN §5; ADR-0011): Arrow batches in, an immutable petgraph
//! `Graph` out, with the maps that take every graph-local index back to its canonical id.
//!
//! - The dense index is the vertex rows' sorted `node_id`s: `NodeIndex(i)` is vertex row `i`, and
//!   domain → dense is a `binary_search`.
//! - Arcs are added in canonical order, so `EdgeIndex(k)` is arc row `k` and the edge weight is
//!   `k`. The arc's columns stay here, in Arrow order; nothing is copied into the graph.
//! - Parallel arcs are kept, nothing is ever removed, and traversals never rely on petgraph's
//!   walker order: they sort a vertex's arcs by row, which is (target id, call site, edge id).

use arrow_array::{
    Array, BooleanArray, FixedSizeBinaryArray, Int16Array, RecordBatch, StringArray,
};
use cpg_schema::codebook::{ArcKind, Codebook, InvocationPhase, Modality, NodeKind, SourceRole};
use cpg_schema::id::Id;
use fixedbitset::FixedBitSet;
use petgraph::graph::{EdgeIndex, Graph, NodeIndex};
use petgraph::{Directed, Direction};

use crate::AnalyticsError;

/// The invocation projection's container: arc weights are arc row indices (§5).
pub type InvocationGraph = Graph<(), u32, Directed, u32>;

/// One arc's columns, by row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arc {
    pub src: u32,
    pub dst: u32,
    pub call_site: Id,
    pub edge_id: Id,
    /// A call's phase; a definition arc has none.
    pub phase: Option<InvocationPhase>,
    pub modality: Modality,
    pub has_unresolved_remainder: bool,
    pub arc_kind: ArcKind,
}

impl Arc {
    /// A definite call: the only arc that can make a direct delegation (DESIGN §3.6).
    pub fn is_definite_call(&self) -> bool {
        self.arc_kind == ArcKind::Call && self.modality == Modality::Definite
    }
}

/// A call site with no target, or an unresolved remainder, and the vertex whose body holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedSite {
    pub caller: u32,
    pub call_site: Id,
}

/// A projection as a graph: vertices, arcs and unresolved sites, with every index mapped back.
#[derive(Debug)]
pub struct Projection {
    pub ids: Vec<Id>,
    pub kinds: Vec<NodeKind>,
    /// The module a vertex belongs to, when it is in an analyzed release.
    pub modules: Vec<Option<String>>,
    pub roles: Vec<Option<SourceRole>>,
    pub arcs: Vec<Arc>,
    pub unresolved: Vec<UnresolvedSite>,
    pub graph: InvocationGraph,
}

fn column<'a, T: 'static>(batch: &'a RecordBatch, name: &str) -> Result<&'a T, AnalyticsError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<T>())
        .ok_or_else(|| AnalyticsError::Column(name.to_owned()))
}

fn id_at(array: &FixedSizeBinaryArray, row: usize, name: &str) -> Result<Id, AnalyticsError> {
    <[u8; 16]>::try_from(array.value(row))
        .map(Id)
        .map_err(|_| AnalyticsError::Column(name.to_owned()))
}

fn code<C: Codebook>(array: &Int16Array, row: usize, name: &str) -> Result<C, AnalyticsError> {
    C::from_code(array.value(row)).ok_or_else(|| AnalyticsError::Column(name.to_owned()))
}

impl Projection {
    /// Build from the projection's three result sets (each a list of batches in its query's
    /// order): vertices `(node_id, node_kind, module_name, role)`, arcs `(src_node_id,
    /// dst_node_id, call_site_node_id, edge_id, phase, modality, has_unresolved_remainder,
    /// arc_kind)` and
    /// unresolved sites `(caller_node_id, call_site_node_id, …)`. Vertex ids must be strictly
    /// increasing; an arc whose ends are not vertices, or arcs out of canonical order, are refused.
    pub fn build(
        vertices: &[RecordBatch],
        arcs: &[RecordBatch],
        unresolved: &[RecordBatch],
    ) -> Result<Self, AnalyticsError> {
        let mut ids = Vec::new();
        let mut kinds = Vec::new();
        let mut modules = Vec::new();
        let mut roles = Vec::new();
        for b in vertices {
            let id = column::<FixedSizeBinaryArray>(b, "node_id")?;
            let kind = column::<Int16Array>(b, "node_kind")?;
            let module = column::<StringArray>(b, "module_name")?;
            let role = column::<Int16Array>(b, "role")?;
            for row in 0..b.num_rows() {
                ids.push(id_at(id, row, "node_id")?);
                kinds.push(code::<NodeKind>(kind, row, "node_kind")?);
                modules.push((!module.is_null(row)).then(|| module.value(row).to_owned()));
                roles.push(if role.is_null(row) {
                    None
                } else {
                    Some(code::<SourceRole>(role, row, "role")?)
                });
            }
        }
        if ids.windows(2).any(|w| w[0] >= w[1]) {
            return Err(AnalyticsError::Order("vertices".to_owned()));
        }
        let dense = |id: Id| -> Result<u32, AnalyticsError> {
            ids.binary_search(&id)
                .map(|i| i as u32)
                .map_err(|_| AnalyticsError::UnknownVertex(id.hex()))
        };

        let mut out_arcs = Vec::new();
        for b in arcs {
            let src = column::<FixedSizeBinaryArray>(b, "src_node_id")?;
            let dst = column::<FixedSizeBinaryArray>(b, "dst_node_id")?;
            let site = column::<FixedSizeBinaryArray>(b, "call_site_node_id")?;
            let edge = column::<FixedSizeBinaryArray>(b, "edge_id")?;
            let phase = column::<Int16Array>(b, "phase")?;
            let modality = column::<Int16Array>(b, "modality")?;
            let remainder = column::<BooleanArray>(b, "has_unresolved_remainder")?;
            let arc_kind = column::<Int16Array>(b, "arc_kind")?;
            for row in 0..b.num_rows() {
                out_arcs.push(Arc {
                    src: dense(id_at(src, row, "src_node_id")?)?,
                    dst: dense(id_at(dst, row, "dst_node_id")?)?,
                    call_site: id_at(site, row, "call_site_node_id")?,
                    edge_id: id_at(edge, row, "edge_id")?,
                    phase: if phase.is_null(row) {
                        None
                    } else {
                        Some(code(phase, row, "phase")?)
                    },
                    modality: code(modality, row, "modality")?,
                    has_unresolved_remainder: remainder.value(row),
                    arc_kind: code(arc_kind, row, "arc_kind")?,
                });
            }
        }
        // Canonical order is (src id, dst id, call site, edge id); dense indices order like ids.
        let key = |a: &Arc| (a.src, a.dst, a.call_site, a.edge_id);
        if out_arcs.windows(2).any(|w| key(&w[0]) >= key(&w[1])) {
            return Err(AnalyticsError::Order("arcs".to_owned()));
        }

        let mut out_unresolved = Vec::new();
        for b in unresolved {
            let caller = column::<FixedSizeBinaryArray>(b, "caller_node_id")?;
            let site = column::<FixedSizeBinaryArray>(b, "call_site_node_id")?;
            for row in 0..b.num_rows() {
                out_unresolved.push(UnresolvedSite {
                    caller: dense(id_at(caller, row, "caller_node_id")?)?,
                    call_site: id_at(site, row, "call_site_node_id")?,
                });
            }
        }

        let mut graph = InvocationGraph::with_capacity(ids.len(), out_arcs.len());
        for i in 0..ids.len() {
            let n = graph
                .try_add_node(())
                .map_err(|e| AnalyticsError::Graph(e.to_string()))?;
            debug_assert_eq!(n.index(), i);
        }
        for (k, a) in out_arcs.iter().enumerate() {
            let e = graph
                .try_add_edge(
                    NodeIndex::new(a.src as usize),
                    NodeIndex::new(a.dst as usize),
                    k as u32,
                )
                .map_err(|e| AnalyticsError::Graph(e.to_string()))?;
            debug_assert_eq!(e.index(), k);
        }
        Ok(Self {
            ids,
            kinds,
            modules,
            roles,
            arcs: out_arcs,
            unresolved: out_unresolved,
            graph,
        })
    }

    /// The dense index of a canonical id.
    pub fn dense(&self, id: Id) -> Option<u32> {
        self.ids.binary_search(&id).ok().map(|i| i as u32)
    }

    /// A vertex's outgoing arc rows in canonical order (target id, call site, edge id), never in
    /// petgraph's newest-first walker order (§5).
    pub fn out_arcs(&self, vertex: u32) -> Vec<u32> {
        let mut rows: Vec<u32> = self
            .graph
            .edges_directed(NodeIndex::new(vertex as usize), Direction::Outgoing)
            .map(|e| *e.weight())
            .collect();
        rows.sort_unstable();
        rows
    }

    /// The arc behind a graph edge.
    pub fn arc(&self, edge: EdgeIndex<u32>) -> &Arc {
        &self.arcs[self.graph[edge] as usize]
    }

    /// A mask of the vertices a predicate accepts, for `NodeFiltered` views and kernels.
    pub fn mask(&self, keep: impl Fn(usize) -> bool) -> FixedBitSet {
        let mut bits = FixedBitSet::with_capacity(self.ids.len());
        for i in 0..self.ids.len() {
            bits.set(i, keep(i));
        }
        bits
    }
}
