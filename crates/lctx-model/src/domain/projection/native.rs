//! Read-only petgraph traits over the stored graph. Tokens belong to one callback invocation.
use super::{ArcId, snapshot::ProgramGraph};
use crate::domain::{Id, Record, normalized::entities::EntityRef};
use petgraph::{
    Directed, Direction,
    graph::{self, EdgeIndex, NodeIndex},
    visit::*,
};
use std::{fmt, marker::PhantomData};

// Both argument and result make the brand invariant, independently of the graph borrow.
type Brand<'id> = PhantomData<fn(&'id ()) -> &'id ()>;

macro_rules! token {
    ($name:ident, $index:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<'id> {
            index: $index<u32>,
            brand: Brand<'id>,
        }
        impl $name<'_> {
            fn new(index: $index<u32>) -> Self {
                Self {
                    index,
                    brand: PhantomData,
                }
            }
        }
        impl fmt::Debug for $name<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(..)"))
            }
        }
    };
}
token!(GraphNode, NodeIndex);
token!(GraphEdge, EdgeIndex);

/// Borrowed topology. Only MaterializedGraph can create a view; constructors and indexes are private.
/// Kernel owners reserve their algorithm workspace before visiting; this view retains the
/// materialization's borrow and therefore its graph/index reservation for every algorithm.
#[derive(Clone, Copy)]
pub struct NativeGraphView<'graph, 'id> {
    graph: &'graph ProgramGraph,
    brand: Brand<'id>,
}
impl<'graph, 'id> NativeGraphView<'graph, 'id> {
    pub(super) fn new(graph: &'graph ProgramGraph) -> Self {
        Self {
            graph,
            brand: PhantomData,
        }
    }
    pub fn entity(self, node: GraphNode<'id>) -> &'graph EntityRef {
        &self.graph[node.index]
    }
    pub fn entity_id(self, node: GraphNode<'id>) -> Id<EntityRef> {
        self.entity(node).id()
    }
    pub fn arc_id(self, edge: GraphEdge<'id>) -> ArcId {
        self.graph[edge.index]
    }
}
impl<'id> GraphBase for NativeGraphView<'_, 'id> {
    type NodeId = GraphNode<'id>;
    type EdgeId = GraphEdge<'id>;
}
impl GraphRef for NativeGraphView<'_, '_> {}
impl Data for NativeGraphView<'_, '_> {
    type NodeWeight = EntityRef;
    type EdgeWeight = ArcId;
}
impl GraphProp for NativeGraphView<'_, '_> {
    type EdgeType = Directed;
}
impl NodeCount for NativeGraphView<'_, '_> {
    fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}
impl EdgeCount for NativeGraphView<'_, '_> {
    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

pub type NativeNodes<'id> =
    std::iter::Map<graph::NodeIndices<u32>, fn(NodeIndex<u32>) -> GraphNode<'id>>;
pub type NativeNeighbors<'graph, 'id> =
    std::iter::Map<graph::Neighbors<'graph, ArcId, u32>, fn(NodeIndex<u32>) -> GraphNode<'id>>;

impl<'id> IntoNodeIdentifiers for NativeGraphView<'_, 'id> {
    type NodeIdentifiers = NativeNodes<'id>;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.graph.node_indices().map(GraphNode::new)
    }
}
impl<'graph, 'id> IntoNeighbors for NativeGraphView<'graph, 'id> {
    type Neighbors = NativeNeighbors<'graph, 'id>;
    fn neighbors(self, node: GraphNode<'id>) -> Self::Neighbors {
        self.graph.neighbors(node.index).map(GraphNode::new)
    }
}
impl<'graph, 'id> IntoNeighborsDirected for NativeGraphView<'graph, 'id> {
    type NeighborsDirected = NativeNeighbors<'graph, 'id>;
    fn neighbors_directed(
        self,
        node: GraphNode<'id>,
        direction: Direction,
    ) -> Self::NeighborsDirected {
        self.graph
            .neighbors_directed(node.index, direction)
            .map(GraphNode::new)
    }
}

#[derive(Clone, Copy)]
pub struct NativeEdgeRef<'graph, 'id> {
    edge: graph::EdgeReference<'graph, ArcId, u32>,
    brand: Brand<'id>,
}
impl<'graph> NativeEdgeRef<'graph, '_> {
    fn new(edge: graph::EdgeReference<'graph, ArcId, u32>) -> Self {
        Self {
            edge,
            brand: PhantomData,
        }
    }
}
impl<'id> EdgeRef for NativeEdgeRef<'_, 'id> {
    type NodeId = GraphNode<'id>;
    type EdgeId = GraphEdge<'id>;
    type Weight = ArcId;
    fn source(&self) -> Self::NodeId {
        GraphNode::new(self.edge.source())
    }
    fn target(&self) -> Self::NodeId {
        GraphNode::new(self.edge.target())
    }
    fn weight(&self) -> &ArcId {
        self.edge.weight()
    }
    fn id(&self) -> Self::EdgeId {
        GraphEdge::new(self.edge.id())
    }
}
pub type NativeEdgeReferences<'graph, 'id> = std::iter::Map<
    graph::EdgeReferences<'graph, ArcId, u32>,
    fn(graph::EdgeReference<'graph, ArcId, u32>) -> NativeEdgeRef<'graph, 'id>,
>;
pub type NativeEdges<'graph, 'id> = std::iter::Map<
    graph::Edges<'graph, ArcId, Directed, u32>,
    fn(graph::EdgeReference<'graph, ArcId, u32>) -> NativeEdgeRef<'graph, 'id>,
>;
impl<'graph, 'id> IntoEdgeReferences for NativeGraphView<'graph, 'id> {
    type EdgeRef = NativeEdgeRef<'graph, 'id>;
    type EdgeReferences = NativeEdgeReferences<'graph, 'id>;
    fn edge_references(self) -> Self::EdgeReferences {
        self.graph.edge_references().map(NativeEdgeRef::new)
    }
}
impl<'graph, 'id> IntoEdges for NativeGraphView<'graph, 'id> {
    type Edges = NativeEdges<'graph, 'id>;
    fn edges(self, node: GraphNode<'id>) -> Self::Edges {
        self.graph.edges(node.index).map(NativeEdgeRef::new)
    }
}
impl<'graph, 'id> IntoEdgesDirected for NativeGraphView<'graph, 'id> {
    type EdgesDirected = NativeEdges<'graph, 'id>;
    fn edges_directed(self, node: GraphNode<'id>, direction: Direction) -> Self::EdgesDirected {
        self.graph
            .edges_directed(node.index, direction)
            .map(NativeEdgeRef::new)
    }
}

pub struct NativeVisitMap<'id> {
    map: <ProgramGraph as Visitable>::Map,
    brand: Brand<'id>,
}
impl<'id> VisitMap<GraphNode<'id>> for NativeVisitMap<'id> {
    fn visit(&mut self, node: GraphNode<'id>) -> bool {
        self.map.visit(node.index)
    }
    fn is_visited(&self, node: &GraphNode<'id>) -> bool {
        self.map.is_visited(&node.index)
    }
    fn unvisit(&mut self, node: GraphNode<'id>) -> bool {
        self.map.unvisit(node.index)
    }
}
impl<'id> Visitable for NativeGraphView<'_, 'id> {
    type Map = NativeVisitMap<'id>;
    fn visit_map(&self) -> Self::Map {
        NativeVisitMap {
            map: self.graph.visit_map(),
            brand: PhantomData,
        }
    }
    fn reset_map(&self, map: &mut Self::Map) {
        self.graph.reset_map(&mut map.map);
    }
}
