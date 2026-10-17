//! Synchronous raw-flow delivery. Only the immutable preamble changes owner; current event
//! payloads remain producer-owned and admitted until the callback returns.
use crate::{
    CaptureSnapshot, Def, FlowCondition, Reach, Region, SkipCounts, Span, Test, Use, UseCandidate,
    native,
};
use lctx_model::domain::{
    ContentHash, ModelError,
    charged::StateCharge,
    source::{OccurrenceRole, SyntaxKind},
};
use std::{
    borrow::Borrow,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

#[derive(Debug)]
pub enum FlowOperationalError {
    Model(ModelError),
    NativeCancelled(salsa::Cancelled),
}
#[derive(Debug)]
pub enum VisitError<E> {
    Operational(FlowOperationalError),
    Consumer(E),
}
#[derive(Debug)]
pub struct ModuleFailure {
    pub detail: String,
}
#[derive(Debug, Clone, Copy)]
pub struct NodeEntry {
    pub ordinal: usize,
    pub kind: Option<SyntaxKind>,
    pub role: OccurrenceRole,
}
#[derive(Debug, Clone)]
pub(crate) struct AtomKey {
    pub condition: FlowCondition,
    pub node: usize,
}
impl Borrow<native::Atom> for AtomKey {
    fn borrow(&self) -> &native::Atom {
        match &self.condition.nodes[self.node] {
            lctx_model::domain::conditions::CondNode::Leaf(atom) => atom,
            _ => unreachable!("operand key is a leaf"),
        }
    }
}
impl PartialEq for AtomKey {
    fn eq(&self, other: &Self) -> bool {
        Borrow::<native::Atom>::borrow(self) == Borrow::<native::Atom>::borrow(other)
    }
}
impl Eq for AtomKey {}
impl Hash for AtomKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Borrow::<native::Atom>::borrow(self).hash(state);
    }
}

#[derive(Debug)]
pub struct ModulePrelude {
    pub original_content: ContentHash,
    pub view_content: ContentHash,
    pub view_byte_len: usize,
    pub renamed: u32,
    pub syntax_errors: usize,
    pub(crate) nodes: BTreeMap<Span, Vec<NodeEntry>>,
    pub(crate) places: BTreeMap<Span, native::Place>,
    pub(crate) operands: std::collections::HashMap<AtomKey, Option<Span>>,
    pub(crate) _charge: StateCharge,
}
impl ModulePrelude {
    pub(crate) fn record_operand(
        &mut self,
        condition: FlowCondition,
        node: usize,
        operand: Option<Span>,
    ) {
        let key = AtomKey { condition, node };
        if !self.operands.contains_key(&key) {
            crate::admitted::hash_slot(&mut self.operands, &mut self._charge);
            self.operands.insert(key, operand);
        }
    }
    /// Ordered raw entries, including duplicates and unsupported native kinds.
    pub fn nodes(&self, span: Span) -> &[NodeEntry] {
        self.nodes.get(&span).map_or(&[], Vec::as_slice)
    }
    pub fn place(&self, span: Span) -> Option<&native::Place> {
        self.places.get(&span)
    }
    pub fn raw_nodes(&self) -> impl Iterator<Item = (Span, NodeEntry)> + '_ {
        self.nodes
            .iter()
            .flat_map(|(span, rows)| rows.iter().map(|row| (*span, *row)))
    }
    pub fn places(&self) -> impl Iterator<Item = (Span, &native::Place)> + '_ {
        self.places.iter().map(|(span, place)| (*span, place))
    }
    /// First equal atom in stable (test span, leaf span) order, including its None operand.
    pub fn operand(&self, atom: &native::Atom) -> Option<Span> {
        self.operands.get(atom).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admitted::{Resources, admit};
    use lctx_model::domain::{resources::ResourceBudget, source::SyntaxKind};
    fn prelude(budget: &ResourceBudget) -> ModulePrelude {
        ModulePrelude {
            original_content: ContentHash::of(b""),
            view_content: ContentHash::of(b""),
            view_byte_len: 0,
            renamed: 0,
            syntax_errors: 0,
            nodes: BTreeMap::new(),
            places: BTreeMap::new(),
            operands: Default::default(),
            _charge: StateCharge::new(budget, "test_preamble"),
        }
    }
    #[test]
    fn raw_span_buckets_keep_duplicate_statements_and_unsupported_shapes() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut prelude = prelude(&budget);
        let span = Span { start: 0, end: 1 };
        admit(&mut prelude._charge, 1024);
        prelude.nodes.insert(
            span,
            vec![
                NodeEntry {
                    ordinal: 0,
                    kind: Some(SyntaxKind::StmtAssign),
                    role: OccurrenceRole::Syntax,
                },
                NodeEntry {
                    ordinal: 1,
                    kind: Some(SyntaxKind::StmtAssign),
                    role: OccurrenceRole::Syntax,
                },
                NodeEntry {
                    ordinal: 2,
                    kind: None,
                    role: OccurrenceRole::Syntax,
                },
            ],
        );
        assert_eq!(prelude.nodes(span).len(), 3);
        assert_eq!(
            prelude
                .nodes(span)
                .iter()
                .filter(|n| n.kind.is_some_and(|k| (2..=26).contains(&(k as i16))))
                .count(),
            2
        );
        let mut supported: Vec<_> = prelude
            .nodes(span)
            .iter()
            .filter_map(|n| n.kind.map(|kind| (kind as i16, n.role as i16)))
            .collect();
        supported.sort_unstable();
        supported.dedup();
        assert_eq!(supported.len(), 1);
        drop(prelude);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn operand_lookup_preserves_first_equal_atom_even_when_none() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let check = || Ok(());
        let resources = Resources::new(&budget, &check);
        let mut prelude = prelude(&budget);
        {
            let _scratch = resources.scope();
            let condition =
                FlowCondition::atom(&budget, native::Atom::opaque_in(&resources, "same"));
            prelude.record_operand(condition.clone(), 0, None);
            prelude.record_operand(condition.clone(), 0, Some(Span { start: 1, end: 2 }));
            assert_eq!(prelude.operand(condition.leaves().next().unwrap()), None);
        }
        assert!(budget.reserved() > 0);
        drop(prelude);
        drop(resources);
        assert_eq!(budget.reserved(), 0);
    }
}
#[derive(Debug)]
pub struct UseGroup {
    pub use_: Use,
    pub capture: Option<CaptureSnapshot>,
    pub candidates: Vec<UseCandidate>,
    pub reaching: Vec<Reach>,
    pub(crate) _charge: StateCharge,
}
#[derive(Debug, Clone, Copy)]
pub struct ValueSourceRef<'a> {
    pub sink: crate::Sink,
    pub span: Span,
    pub use_ix: u32,
    pub identity: bool,
    pub through_call: bool,
    pub call_path: &'a [crate::CallFrame],
    pub condition: &'a FlowCondition,
}
#[derive(Debug, Clone, Copy)]
pub struct AttributeLoadRef<'a> {
    pub span: Span,
    pub name: &'a str,
}
#[derive(Debug, Clone, Copy)]
pub struct TestLeafRef<'a> {
    pub scope: crate::Scope,
    pub test_span: Span,
    pub condition: &'a FlowCondition,
    pub atom: &'a native::Atom,
    pub leaf_span: Span,
    pub operand_span: Option<Span>,
}
#[derive(Debug)]
pub enum ModuleEvent<'a> {
    Prelude(ModulePrelude),
    Definition { ordinal: u32, value: &'a Def },
    UnavailableUse(Span),
    UseGroup { ordinal: u32, value: &'a UseGroup },
    Value(ValueSourceRef<'a>),
    Region(&'a Region),
    Attribute(AttributeLoadRef<'a>),
    TraversalComplete,
    Test(&'a Test),
    TestLeaf(TestLeafRef<'a>),
    End(SkipCounts),
    ModuleFailure(&'a ModuleFailure),
}
