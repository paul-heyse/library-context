//! A pinned computational representation, rebuilt once per generation. Its wire format is not
//! a semantic schema or a reusable cache key. Canonical rows are checked before publication.
use super::normalization::{ProjectionInput, ProjectionKey, invalid};
use super::*;
use crate::domain::{
    normalized::Rows,
    resources::{Reservation, ResourceBudget},
};
pub use petgraph::Direction;
use petgraph::{
    Directed, Graph,
    graph::NodeIndex,
    visit::{Dfs, EdgeFiltered, EdgeRef, Reversed},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CHUNK_BYTES: usize = 1024 * 1024;
pub const FORMAT_VERSION: i32 = 1;
pub const PETGRAPH_VERSION: &str = "0.8.3";
pub const CODEC: &str = "postcard-1.1.3";
pub(super) type ProgramGraph = Graph<EntityRef, ArcId, Directed, u32>;
#[derive(Serialize, Deserialize)]
struct Wire<G> {
    format: i32,
    petgraph: [u16; 3],
    projection_version: i32,
    key: ProjectionKey,
    graph: G,
}
/// Immutable graph and runtime indexes. Public operations return domain IDs, never petgraph indices.
pub struct MaterializedGraph {
    key: ProjectionKey,
    graph: ProgramGraph,
    index: BTreeMap<Id<EntityRef>, NodeIndex<u32>>,
    outgoing: Vec<Vec<ArcId>>,
    incoming: Vec<Vec<ArcId>>,
    _reservation: Box<dyn Reservation>,
}
pub fn check_capacity(vertices: usize, arcs: usize) -> Result<(), ModelError> {
    // petgraph 0.8.3's deserializer refuses equality with its index sentinel as well.
    if vertices >= u32::MAX as usize || arcs >= u32::MAX as usize {
        return Err(invalid("projection exceeds petgraph u32 capacity"));
    }
    Ok(())
}
fn allocation(vertices: usize, arcs: usize) -> Result<usize, ModelError> {
    check_capacity(vertices, arcs)?;
    // Includes graph vectors, map node overhead and both geometrically grown adjacency vectors.
    vertices
        .checked_mul(256)
        .and_then(|v| arcs.checked_mul(256).and_then(|e| v.checked_add(e)))
        .and_then(|v| v.checked_add(4096))
        .ok_or_else(|| invalid("projection allocation overflow"))
}
impl MaterializedGraph {
    /// Borrow the stored graph under a fresh invariant brand. Convert algorithm outputs to
    /// canonical entities/arc IDs inside this callback; local tokens cannot escape or mix.
    ///
    /// ```compile_fail
    /// use lctx_model::domain::projection::snapshot::MaterializedGraph;
    /// use petgraph::visit::IntoNodeIdentifiers;
    /// fn cross_graph(first: &MaterializedGraph, second: &MaterializedGraph) {
    ///     first.with_native_graph(|a| second.with_native_graph(|b| {
    ///         let node = a.node_identifiers().next().unwrap();
    ///         b.entity(node);
    ///     }));
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// use lctx_model::domain::projection::snapshot::MaterializedGraph;
    /// use petgraph::visit::IntoNodeIdentifiers;
    /// fn escape(graph: &MaterializedGraph) {
    ///     let _node = graph.with_native_graph(|view| view.node_identifiers().next());
    /// }
    /// ```
    ///
    /// ```
    /// use lctx_model::domain::{Id, normalized::entities::EntityRef, projection::snapshot::MaterializedGraph};
    /// use petgraph::visit::IntoNodeIdentifiers;
    /// fn canonical(graph: &MaterializedGraph) -> Vec<Id<EntityRef>> {
    ///     graph.with_native_graph(|view| view.node_identifiers().map(|node| view.entity_id(node)).collect())
    /// }
    /// ```
    pub fn with_native_graph<'graph, R>(
        &'graph self,
        visit: impl for<'id> FnOnce(super::native::NativeGraphView<'graph, 'id>) -> R,
    ) -> R {
        visit(super::native::NativeGraphView::new(&self.graph))
    }
    pub fn build(input: &ProjectionInput, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let reservation = budget.reserve(
            "materialized-graph",
            allocation(input.vertex_count(), input.arc_count())?,
        )?;
        let mut graph = ProgramGraph::with_capacity(input.vertex_count(), input.arc_count());
        let mut index = BTreeMap::new();
        for row in input.vertices() {
            let node = graph.try_add_node(row.clone()).map_err(ModelError::codec)?;
            index.insert(row.id(), node);
        }
        for arc in input.arcs() {
            let source = *index
                .get(&arc.source)
                .ok_or_else(|| invalid("missing graph source endpoint"))?;
            let target = *index
                .get(&arc.target)
                .ok_or_else(|| invalid("missing graph target endpoint"))?;
            graph
                .try_add_edge(source, target, arc.id)
                .map_err(ModelError::codec)?;
        }
        Self::indexed(input.key(), graph, reservation)
    }
    fn indexed(
        key: ProjectionKey,
        graph: ProgramGraph,
        reservation: Box<dyn Reservation>,
    ) -> Result<Self, ModelError> {
        check_capacity(graph.node_count(), graph.edge_count())?;
        let mut index = BTreeMap::new();
        let mut previous = None;
        for node in graph.node_indices() {
            let entity = &graph[node];
            let id = entity.id();
            if !ProjectionSpec::builtin(key.name).accepts(entity)
                || previous.is_some_and(|p| p >= id)
            {
                return Err(invalid("noncanonical graph entity universe"));
            }
            previous = Some(id);
            index.insert(id, node);
        }
        let mut outgoing = vec![Vec::new(); graph.node_count()];
        let mut incoming = vec![Vec::new(); graph.node_count()];
        let mut previous = None;
        for edge in graph.edge_references() {
            let id = *edge.weight();
            if !ProjectionSpec::builtin(key.name)
                .roles()
                .contains(&id.role())
                || previous.is_some_and(|p| p >= id)
            {
                return Err(invalid("noncanonical graph arc identity or role"));
            }
            previous = Some(id);
            outgoing[edge.source().index()].push(id);
            incoming[edge.target().index()].push(id);
        }
        // Edges were serialized/inserted in canonical ArcId order, so both lists are already sorted.
        Ok(Self {
            key,
            graph,
            index,
            outgoing,
            incoming,
            _reservation: reservation,
        })
    }
    pub fn key(&self) -> ProjectionKey {
        self.key
    }
    pub fn vertex_count(&self) -> usize {
        self.graph.node_count()
    }
    pub fn arc_count(&self) -> usize {
        self.graph.edge_count()
    }
    pub fn entities(&self) -> impl Iterator<Item = &EntityRef> {
        self.graph.node_weights()
    }
    pub fn arcs(&self) -> impl Iterator<Item = Arc> + '_ {
        self.graph.edge_references().map(|e| Arc {
            id: *e.weight(),
            source: self.graph[e.source()].id(),
            target: self.graph[e.target()].id(),
        })
    }
    pub fn outgoing(&self, entity: Id<EntityRef>) -> Option<&[ArcId]> {
        self.index
            .get(&entity)
            .map(|n| self.outgoing[n.index()].as_slice())
    }
    pub fn incoming(&self, entity: Id<EntityRef>) -> Option<&[ArcId]> {
        self.index
            .get(&entity)
            .map(|n| self.incoming[n.index()].as_slice())
    }
    /// Traverse the complete universe before applying the output selector. Borrowed graph views
    /// preserve intermediate nodes and cannot change the named arc meaning.
    pub fn reachable(
        &self,
        start: Id<EntityRef>,
        direction: Direction,
        role: Option<EndpointRole>,
        mut select: impl FnMut(&EntityRef) -> bool,
        budget: &ResourceBudget,
    ) -> Result<SelectedEntities, ModelError> {
        let start = *self
            .index
            .get(&start)
            .ok_or_else(|| invalid("graph start outside universe"))?;
        let held = budget.reserve(
            "graph-traversal",
            self.vertex_count()
                .saturating_mul(128)
                .saturating_add(self.arc_count().saturating_mul(16))
                .saturating_add(4096),
        )?;
        let filtered = EdgeFiltered::from_fn(&self.graph, |e| {
            role.is_none_or(|role| e.weight().role() == role)
        });
        let mut values = Vec::new();
        match direction {
            Direction::Outgoing => {
                let mut dfs = Dfs::new(&filtered, start);
                while let Some(node) = dfs.next(&filtered) {
                    if select(&self.graph[node]) {
                        values.push(self.graph[node].id());
                    }
                }
            }
            Direction::Incoming => {
                let reversed = Reversed(&filtered);
                let mut dfs = Dfs::new(reversed, start);
                while let Some(node) = dfs.next(reversed) {
                    if select(&self.graph[node]) {
                        values.push(self.graph[node].id());
                    }
                }
            }
        }
        values.sort();
        Ok(SelectedEntities {
            values,
            _reservation: held,
        })
    }
    pub fn encode(&self, budget: &ResourceBudget) -> Result<EncodedGraph, ModelError> {
        let wire = Wire {
            format: FORMAT_VERSION,
            petgraph: [0, 8, 3],
            projection_version: ProjectionSpec::VERSION,
            key: self.key,
            graph: &self.graph,
        };
        let size = postcard::experimental::serialized_size(&wire).map_err(ModelError::codec)?;
        let reservation = budget.reserve("graph-encoding", size)?;
        let mut bytes = vec![0; size];
        let used = postcard::to_slice(&wire, &mut bytes)
            .map_err(ModelError::codec)?
            .len();
        if used != size {
            return Err(invalid("graph encoding size differs"));
        }
        Ok(EncodedGraph {
            bytes,
            _reservation: reservation,
        })
    }
    /// Decode a generation-owned object. No SQL edge reconstruction or graph insertion occurs.
    pub fn decode(
        bytes: &[u8],
        key: ProjectionKey,
        vertices: usize,
        arcs: usize,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let graph_bytes = allocation(vertices, arcs)?;
        // The pinned petgraph visitor grows only after decoding each fixed-size weight and does
        // not reserve untrusted sequence length hints. 16x encoded size bounds its temporary
        // geometric vectors even for malformed counts; the wrapper contains no variable strings.
        let decode_bytes = bytes
            .len()
            .checked_mul(16)
            .and_then(|n| n.checked_add(graph_bytes))
            .ok_or_else(|| invalid("graph decoding size overflow"))?;
        let reservation = budget.reserve("graph-hydration", decode_bytes)?;
        let (wire, tail): (Wire<ProgramGraph>, _) =
            postcard::take_from_bytes(bytes).map_err(ModelError::codec)?;
        if !tail.is_empty()
            || wire.format != FORMAT_VERSION
            || wire.petgraph != [0, 8, 3]
            || wire.projection_version != ProjectionSpec::VERSION
            || wire.key != key
            || wire.graph.node_count() != vertices
            || wire.graph.edge_count() != arcs
        {
            return Err(invalid("graph snapshot wrapper or shape mismatch"));
        }
        let mut result = Self::indexed(key, wire.graph, reservation)?;
        // Keep the conservative decode reservation through indexing. The immutable object retains
        // only its graph/index allowance after all temporary decoder state has been released.
        result._reservation.try_resize(graph_bytes)?;
        Ok(result)
    }
    pub fn matches(&self, input: &ProjectionInput) -> Result<(), ModelError> {
        if self.key != input.key()
            || !self.entities().eq(input.vertices())
            || !self.arcs().eq(input.arcs().copied())
        {
            return Err(invalid("graph snapshot differs from canonical projection"));
        }
        Ok(())
    }
}
pub struct SelectedEntities {
    values: Vec<Id<EntityRef>>,
    _reservation: Box<dyn Reservation>,
}
impl SelectedEntities {
    pub fn as_slice(&self) -> &[Id<EntityRef>] {
        &self.values
    }
}
pub struct EncodedGraph {
    bytes: Vec<u8>,
    _reservation: Box<dyn Reservation>,
}
impl EncodedGraph {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
pub fn header(
    assessment: Id<ProjectionSourceAssessment>,
    bytes: usize,
) -> Result<ProjectionSnapshot, ModelError> {
    Ok(ProjectionSnapshot {
        assessment,
        format_version: FORMAT_VERSION,
        petgraph_version: PETGRAPH_VERSION.into(),
        codec: CODEC.into(),
        bytes: i64::try_from(bytes).map_err(ModelError::codec)?,
        chunks: i64::try_from(bytes.div_ceil(CHUNK_BYTES)).map_err(ModelError::codec)?,
    })
}
pub fn validate_header(row: &ProjectionSnapshot) -> Result<(), ModelError> {
    if row.format_version != FORMAT_VERSION
        || row.petgraph_version != PETGRAPH_VERSION
        || row.codec != CODEC
        || row.bytes <= 0
        || row.chunks != (row.bytes as u64).div_ceil(CHUNK_BYTES as u64) as i64
    {
        return Err(invalid("unsupported graph snapshot header"));
    }
    Ok(())
}
pub fn validate_chunk(row: &ProjectionSnapshotChunk) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.payload.0.is_empty() || row.payload.0.len() > CHUNK_BYTES {
        return Err(invalid("invalid graph snapshot chunk"));
    }
    Ok(())
}
pub fn hydrate(
    header: &ProjectionSnapshot,
    assessment: &ProjectionSourceAssessment,
    chunks: &Rows<ProjectionSnapshotChunk>,
    budget: &ResourceBudget,
) -> Result<MaterializedGraph, ModelError> {
    header.validate()?;
    if header.assessment != assessment.id() || assessment.version != ProjectionSpec::VERSION {
        return Err(invalid("graph header source mismatch"));
    }
    let len = usize::try_from(header.bytes).map_err(ModelError::codec)?;
    let _held = budget.reserve(
        "graph-chunk-assembly",
        len.saturating_add((header.chunks as usize).saturating_mul(64)),
    )?;
    let mut ordered = BTreeMap::new();
    for row in chunks.iter().filter(|c| c.snapshot == header.id()) {
        row.validate()?;
        if ordered.insert(row.ordinal, &row.payload.0).is_some() {
            return Err(invalid("duplicate graph chunk"));
        }
    }
    if ordered.len() != header.chunks as usize {
        return Err(invalid("graph chunk set incomplete"));
    }
    let mut bytes = Vec::with_capacity(len);
    for (ordinal, (actual, payload)) in ordered.iter().enumerate() {
        let expected = (len - bytes.len()).min(CHUNK_BYTES);
        if *actual != ordinal as i64 || payload.len() != expected {
            return Err(invalid("noncanonical graph chunk ordering or size"));
        }
        bytes.extend_from_slice(payload);
    }
    MaterializedGraph::decode(
        &bytes,
        ProjectionKey {
            input: assessment.input,
            context: assessment.context,
            name: assessment.projection,
        },
        usize::try_from(assessment.vertices).map_err(ModelError::codec)?,
        usize::try_from(assessment.arcs).map_err(ModelError::codec)?,
        budget,
    )
}
