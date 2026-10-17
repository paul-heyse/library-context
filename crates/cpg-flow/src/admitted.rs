//! Attempt-owned raw adapter allocation. Private operational unwinding crosses native visitors;
//! the public runner catches only that payload and Salsa cancellation.
use crate::{Condition, FlowOperationalError, native::Atom};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use std::{cell::RefCell, fmt, ops::Deref, sync::Arc};

pub(crate) struct Aborted(pub FlowOperationalError);
pub(crate) struct ConsumerStopped;
pub(crate) fn abort(error: FlowOperationalError) -> ! {
    std::panic::resume_unwind(Box::new(Aborted(error)))
}
pub(crate) fn admit(charge: &mut StateCharge, bytes: usize) {
    charge
        .grow(bytes)
        .unwrap_or_else(|error| abort(FlowOperationalError::Model(error)));
}
pub(crate) fn slots<T>(values: &mut Vec<T>, charge: &mut StateCharge, additional: usize) {
    let wanted = values.len().checked_add(additional).unwrap_or_else(|| {
        abort(FlowOperationalError::Model(ModelError::Invalid(
            "flow capacity overflow".into(),
        )))
    });
    if wanted > values.capacity() {
        let capacity = wanted.max(values.capacity().saturating_mul(2)).max(4);
        admit(
            charge,
            (capacity - values.capacity()).saturating_mul(size_of::<T>()),
        );
        values.reserve_exact(capacity - values.len());
    }
}
pub(crate) fn hash_slot<K, V>(
    values: &mut std::collections::HashMap<K, V>,
    charge: &mut StateCharge,
) where
    K: Eq + std::hash::Hash,
{
    if values.len() == values.capacity() {
        let capacity = values.capacity().saturating_mul(2).max(8);
        admit(
            charge,
            capacity
                .saturating_mul(size_of::<(K, V)>() + 1)
                .saturating_mul(2),
        );
        values.reserve(capacity - values.len());
    }
}
pub(crate) struct Resources<'a> {
    pub budget: &'a ResourceBudget,
    pub check: &'a dyn Fn() -> Result<(), ModelError>,
    scratch: RefCell<StateCharge>,
}
impl<'a> Resources<'a> {
    pub fn new(budget: &'a ResourceBudget, check: &'a dyn Fn() -> Result<(), ModelError>) -> Self {
        Self {
            budget,
            check,
            scratch: RefCell::new(StateCharge::new(budget, "flow_scratch")),
        }
    }
    pub fn checkpoint(&self) {
        (self.check)().unwrap_or_else(|error| abort(FlowOperationalError::Model(error)));
    }
    pub fn grow(&self, bytes: usize) {
        self.checkpoint();
        admit(&mut self.scratch.borrow_mut(), bytes);
    }
    pub fn push<T>(&self, values: &mut Vec<T>, value: T) {
        self.checkpoint();
        slots(values, &mut self.scratch.borrow_mut(), 1);
        values.push(value);
    }
    pub fn scope(&self) -> ScratchScope<'_> {
        ScratchScope {
            resources: self,
            start: self.scratch.borrow().reserved(),
        }
    }
    pub fn string(&self, text: &str) -> String {
        self.grow(text.len());
        let mut value = String::with_capacity(text.len());
        value.push_str(text);
        value
    }
    pub fn format(&self, value: fmt::Arguments<'_>) -> String {
        struct Writer<'a>(&'a Resources<'a>, String);
        impl fmt::Write for Writer<'_> {
            fn write_str(&mut self, text: &str) -> fmt::Result {
                let wanted = self.1.len().saturating_add(text.len());
                if wanted > self.1.capacity() {
                    self.0.grow(wanted - self.1.capacity());
                    self.1.reserve_exact(wanted - self.1.len());
                }
                self.1.push_str(text);
                Ok(())
            }
        }
        let mut writer = Writer(self, String::new());
        fmt::write(&mut writer, value).expect("infallible admitted formatter");
        writer.1
    }
    pub fn compact(&self, text: &str, separator: &str) -> String {
        self.grow(text.len());
        let mut value = String::with_capacity(text.len());
        for part in text.split_whitespace() {
            if !value.is_empty() {
                value.push_str(separator);
            }
            value.push_str(part);
        }
        value
    }
}
/// The native helper returns caller-owned Symbol/Member preparation (including member-builder
/// vectors), unlike the opaque database's retained index. Admit from borrowed source geometry
/// before invoking it; name/literal backing and member slots are bounded by that expression.
pub(crate) fn place_expr(
    expr: &ruff_python_ast_ty::Expr,
    resources: &Resources<'_>,
) -> Option<ty_python_core::place::PlaceExpr> {
    use ruff_text_size_ty::Ranged;
    resources.grow((usize::from(expr.range().len()).saturating_add(1)).saturating_mul(128));
    ty_python_core::place::PlaceExpr::try_from_expr(expr)
}
pub(crate) struct ScratchScope<'a> {
    resources: &'a Resources<'a>,
    start: usize,
}
impl Drop for ScratchScope<'_> {
    fn drop(&mut self) {
        let mut charge = self.resources.scratch.borrow_mut();
        let release = charge.reserved() - self.start;
        charge.release(release);
    }
}

#[derive(Debug)]
struct Backing {
    graph: Condition,
    budget: ResourceBudget,
    _charge: StateCharge,
}
/// Immutable raw condition backing. Clones retain the same admitted allocation.
#[derive(Debug, Clone)]
pub struct FlowCondition(Arc<Backing>);
impl Deref for FlowCondition {
    type Target = Condition;
    fn deref(&self) -> &Condition {
        &self.0.graph
    }
}
impl PartialEq for FlowCondition {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for FlowCondition {}
impl FlowCondition {
    fn build(budget: &ResourceBudget, bytes: usize, make: impl FnOnce() -> Condition) -> Self {
        let mut charge = StateCharge::new(budget, "flow_condition");
        let header = size_of::<Backing>() + 2 * size_of::<usize>();
        admit(&mut charge, bytes.saturating_add(header));
        let graph = make();
        let actual = graph
            .nodes
            .capacity()
            .saturating_mul(size_of::<lctx_model::domain::conditions::CondNode<Atom>>())
            .saturating_add(graph.leaves().map(Atom::heap_bytes).sum::<usize>())
            .saturating_add(header);
        assert!(
            actual <= charge.reserved(),
            "condition exceeded admitted construction"
        );
        let release = charge.reserved() - actual;
        charge.release(release);
        Self(Arc::new(Backing {
            graph,
            budget: budget.clone(),
            _charge: charge,
        }))
    }
    pub(crate) fn terminal(budget: &ResourceBudget, truth: Option<bool>) -> Self {
        Self::build(
            budget,
            4 * size_of::<lctx_model::domain::conditions::CondNode<Atom>>(),
            || match truth {
                Some(true) => Condition::always(),
                Some(false) => Condition::never(),
                None => Condition::refused(),
            },
        )
    }
    pub(crate) fn atom(budget: &ResourceBudget, atom: Atom) -> Self {
        Self::build(
            budget,
            4 * size_of::<lctx_model::domain::conditions::CondNode<Atom>>() + atom.heap_bytes(),
            || Condition::atom(atom),
        )
    }
    fn allowance(&self, other: Option<&Self>) -> usize {
        let nodes = self
            .nodes
            .len()
            .saturating_add(other.map_or(0, |c| c.nodes.len()))
            .saturating_add(1);
        nodes
            .saturating_mul(4)
            .max(4)
            .saturating_mul(size_of::<lctx_model::domain::conditions::CondNode<Atom>>())
            .saturating_add(self.leaves().map(Atom::heap_bytes).sum::<usize>())
            .saturating_add(other.map_or(0, |c| c.leaves().map(Atom::heap_bytes).sum::<usize>()))
    }
    pub fn not(&self) -> Self {
        Self::build(&self.0.budget, self.allowance(None), || self.0.graph.not())
    }
    pub fn and(&self, other: &Self) -> Self {
        Self::build(&self.0.budget, self.allowance(Some(other)), || {
            self.0.graph.and(other)
        })
    }
    pub fn or(&self, other: &Self) -> Self {
        Self::build(&self.0.budget, self.allowance(Some(other)), || {
            self.0.graph.or(other)
        })
    }
    pub fn with_approximation(&self) -> Self {
        Self::build(&self.0.budget, self.allowance(None), || {
            self.0.graph.clone().with_approximation()
        })
    }
}
