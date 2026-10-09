//! Attempt-owned dependency graph for reusable computations and current provenance.
//! It is acceleration metadata, never a completed-input or semantic admission capability.
use lctx_model::domain::{
    ContentHash, Key, KeySink, ModelError, charged::StateCharge,
    compilation_product::{DependencyKind, DependencyToken, ProductRequest},
    resources::ResourceBudget,
};
use petgraph::{Direction, graph::{DiGraph, NodeIndex}, visit::EdgeRef};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) enum Eligibility {
    Rows,
    Fresh(&'static str),
}
impl Eligibility {
    pub(super) fn reason(self) -> Option<&'static str> {
        match self { Self::Rows => None, Self::Fresh(reason) => Some(reason) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeKind { Computation, Provenance }
#[derive(Debug)]
enum Node {
    Dependency,
    Product { operation: String, request: ContentHash, binding: ContentHash },
}

/// A result whose complete premise is an explicitly declared pure value may stop value
/// propagation. An exact completed-view or provenance edge always observes the new binding.
pub(super) struct DependencyGraph {
    graph: DiGraph<Node, EdgeKind>,
    dependencies: BTreeMap<(String, Option<String>, ContentHash), NodeIndex>,
    products: BTreeMap<String, NodeIndex>,
    charge: StateCharge,
}
impl DependencyGraph {
    pub(super) fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "compiler-product-dependency-graph");
        charge.grow(size_of::<Self>())?;
        Ok(Self { graph: DiGraph::new(), dependencies: BTreeMap::new(),
            products: BTreeMap::new(), charge })
    }
    fn dependency(&mut self, token: &DependencyToken) -> Result<NodeIndex, ModelError> {
        let key = (token.relation.clone(), token.prefix.clone(), token.identity);
        if let Some(index) = self.dependencies.get(&key) { return Ok(*index); }
        self.charge.grow(size_of::<Node>() * 2 + token.relation.len() * 2
            + token.prefix.as_ref().map_or(0, |prefix| prefix.len() * 2) + 192)?;
        let index = self.graph.add_node(Node::Dependency);
        self.dependencies.insert(key, index);
        Ok(index)
    }
    #[cfg(test)]
    fn bind(&mut self, request: &ProductRequest) -> Result<(), ModelError> {
        self.bind_selected(request, &request.dependencies)
    }
    /// Only an implicit declaration may acquire its selected epoch. This affects attempt
    /// reachability, while the unchanged request continues to own the cache lookup identity.
    pub(super) fn bind_selected(&mut self, request: &ProductRequest, selected: &[DependencyToken]) -> Result<(), ModelError> {
        if request.dependencies.len() != selected.len() || request.dependencies.iter().zip(selected).any(|(declared, actual)| {
            declared.kind != actual.kind || declared.role != actual.role
                || declared.relation != actual.relation || declared.identity != actual.identity
                || declared.prefix.as_ref().is_some_and(|prefix| Some(prefix) != actual.prefix.as_ref())
        }) {
            return Err(ModelError::Conflict("compiler product selected graph dependency"));
        }
        let identity = request.identity()?;
        let mut binding = KeySink::new("compiler-product-selected-graph/v1");
        identity.encode(&mut binding);
        for token in selected {
            binding.part(b"prefix-present", &[u8::from(token.prefix.is_some())]);
            if let Some(prefix) = &token.prefix { prefix.encode(&mut binding); }
        }
        let binding = binding.finish();
        if let Some(index) = self.products.get(&request.operation) {
            if !matches!(&self.graph[*index], Node::Product { request, binding: selected, .. } if *request == identity && *selected == binding) {
                return Err(ModelError::Conflict("compiler product bound twice with different premises"));
            }
            return Ok(());
        }
        self.charge.grow(size_of::<Node>() * 2 + request.operation.len() * 2 + 192)?;
        let product = self.graph.add_node(Node::Product { operation: request.operation.clone(), request: identity, binding });
        self.products.insert(request.operation.clone(), product);
        for token in selected {
            let dependency = self.dependency(token)?;
            self.charge.grow(size_of::<EdgeKind>() + 64)?;
            self.graph.add_edge(dependency, product, if token.kind == DependencyKind::Provenance {
                EdgeKind::Provenance
            } else { EdgeKind::Computation });
        }
        Ok(())
    }
    /// Bind the newly completed attempt view, rather than retaining the cache entry's old
    /// contribution. Later consumers connect through their exact current source identity.
    pub(super) fn complete(&mut self, operation: &str, outputs: impl IntoIterator<Item = DependencyToken>) -> Result<(), ModelError> {
        let product = *self.products.get(operation).ok_or(ModelError::Conflict("compiler product not bound"))?;
        for token in outputs {
            let dependency = self.dependency(&token)?;
            self.charge.grow(size_of::<EdgeKind>() + 64)?;
            self.graph.add_edge(product, dependency, EdgeKind::Provenance);
        }
        Ok(())
    }
    /// Reverse dependency reachability. A value cutoff is permitted only for pure-value
    /// dependency tokens; exact views are not silently converted to value-only premises.
    pub(super) fn affected(&self, tokens: &[DependencyToken]) -> BTreeSet<String> {
        let mut pending = tokens.iter().filter_map(|token| self.dependencies.get(
            &(token.relation.clone(), token.prefix.clone(), token.identity))).copied().collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        let mut operations = BTreeSet::new();
        while let Some(index) = pending.pop() {
            if !visited.insert(index.index()) { continue; }
            if let Node::Product { operation, .. } = &self.graph[index] { operations.insert(operation.clone()); }
            pending.extend(self.graph.edges_directed(index, Direction::Outgoing).map(|edge| edge.target()));
        }
        operations
    }
    pub(super) fn trace(&self, operation: &str, hit: bool, reason: Option<&str>) {
        let inputs = self.products.get(operation).map_or(0, |index|
            self.graph.edges_directed(*index, Direction::Incoming).count());
        tracing::debug!(operation, hit, inputs, reason, "compiler product disposition");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::compilation_product::ProductKind;
    fn token(relation: &str, prefix: Option<&str>, identity: &[u8]) -> DependencyToken {
        DependencyToken { kind: DependencyKind::ExactView, role: relation.into(), relation: relation.into(),
            prefix: prefix.map(str::to_owned), identity: ContentHash::of(identity) }
    }
    fn request(operation: &str, dependencies: Vec<DependencyToken>) -> ProductRequest {
        ProductRequest { kind: ProductKind::PureRows, operation: operation.into(), model: ContentHash::of(b"m"),
            implementation: ContentHash::of(b"i"), policy: ContentHash::of(b"p"), result_contract: ContentHash::of(b"r"),
            configuration: None, profile: "catalog".into(), parameters: vec![], dependencies, outputs: BTreeSet::new() }
    }
    #[test]
    fn reverse_dependencies_preserve_epochs_and_independent_branches() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let mut graph = DependencyGraph::new(&budget).unwrap();
        let a = token("source", Some("facts"), b"a");
        let later = token("source", Some("model"), b"a");
        let normalized = token("normalized", None, b"n");
        graph.bind(&request("normalize", vec![a.clone()])).unwrap();
        graph.complete("normalize", [normalized.clone()]).unwrap();
        graph.bind(&request("catalog", vec![normalized])).unwrap();
        graph.bind(&request("unrelated", vec![later.clone()])).unwrap();
        assert_eq!(graph.affected(&[a]), BTreeSet::from(["normalize".into(), "catalog".into()]));
        assert_eq!(graph.affected(&[later]), BTreeSet::from(["unrelated".into()]));
    }
    #[test]
    fn changed_binding_cannot_reuse_an_attempt_product_node() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let mut graph = DependencyGraph::new(&budget).unwrap();
        let a = request("normalize", vec![token("source", None, b"a")]);
        graph.bind(&a).unwrap();graph.bind(&a).unwrap();
        assert!(graph.bind(&request("normalize", vec![token("source", None, b"b")])).is_err());
    }
    #[test]
    fn implicit_selection_connects_exact_completed_epoch_without_collapsing_explicit_epochs() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let mut graph = DependencyGraph::new(&budget).unwrap();
        let source = token("source", None, b"source");
        let facts = token("shared", Some("facts"), b"same-view");
        let model = token("shared", Some("model"), b"same-view");
        graph.bind(&request("producer", vec![source.clone()])).unwrap();
        graph.complete("producer", [facts.clone()]).unwrap();
        let implicit = request("implicit", vec![token("shared", None, b"same-view")]);
        let cache_identity = implicit.identity().unwrap();
        graph.bind_selected(&implicit, std::slice::from_ref(&facts)).unwrap();
        graph.bind(&request("explicit-model", vec![model.clone()])).unwrap();
        assert_eq!(implicit.identity().unwrap(), cache_identity);
        assert_eq!(implicit.dependencies[0].prefix, None);
        assert_eq!(graph.affected(&[source]), BTreeSet::from(["producer".into(), "implicit".into()]));
        assert_eq!(graph.affected(std::slice::from_ref(&facts)), BTreeSet::from(["implicit".into()]));
        assert_eq!(graph.affected(std::slice::from_ref(&model)), BTreeSet::from(["explicit-model".into()]));
        assert!(graph.bind_selected(&implicit, std::slice::from_ref(&model)).is_err());
        assert!(graph.bind_selected(&request("explicit-facts", vec![facts]), &[model]).is_err());
        assert!(graph.bind_selected(&request("wrong-view", vec![token("shared", None, b"expected")]),
            &[token("shared", Some("facts"), b"different")]).is_err());
    }
}
