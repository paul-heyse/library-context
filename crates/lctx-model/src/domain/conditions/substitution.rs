//! Capture-safe simultaneous substitution through bounded library apply (DESIGN §15.6–§15.7).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use biodivine_lib_bdd::{Bdd, BddPointer, BddVariable, BddVariableSet, op_function};

use super::EvaluationAtom;
use super::kernel::{Diagram, KernelBoundary, MAX_ATOMS, MAX_NODES, MAX_PAIR_WORK, context};
use crate::domain::Id;
type AtomId = Id<EvaluationAtom>;

const MAX_RETAINED_NODES: usize = 1_000_000;

struct Budget {
    work: usize,
    /// Charges every constructed diagram, released temporaries included, so peak retained nodes
    /// are bounded without relying on allocator behavior or evaluation order.
    nodes: usize,
}

impl Budget {
    fn work(&mut self, amount: usize) -> Result<(), KernelBoundary> {
        self.work = self
            .work
            .checked_sub(amount)
            .ok_or(KernelBoundary::WorkPreflight)?;
        Ok(())
    }

    fn retain(&mut self, amount: usize) -> Result<(), KernelBoundary> {
        self.work(amount)?;
        self.nodes = self
            .nodes
            .checked_sub(amount)
            .ok_or(KernelBoundary::NodeLimit)?;
        Ok(())
    }

    fn apply(
        &mut self,
        a: &Bdd,
        b: &Bdd,
        op: fn(Option<bool>, Option<bool>) -> Option<bool>,
    ) -> Result<Bdd, KernelBoundary> {
        self.work(
            a.size()
                .checked_mul(b.size())
                .ok_or(KernelBoundary::WorkPreflight)?,
        )?;
        let out = Bdd::binary_op_with_limit(MAX_NODES.min(self.nodes), a, b, op)
            .ok_or(KernelBoundary::NodeLimit)?;
        self.retain(out.size())?;
        Ok(out)
    }
}

pub(super) fn compose(
    source: &Diagram,
    replacements: &[(AtomId, &Diagram)],
) -> Result<Diagram, KernelBoundary> {
    compose_with_limits(source, replacements, MAX_PAIR_WORK, MAX_RETAINED_NODES)
}

pub(super) fn compose_with_limits(
    source: &Diagram,
    replacements: &[(AtomId, &Diagram)],
    work: usize,
    nodes: usize,
) -> Result<Diagram, KernelBoundary> {
    // Refuse malformed bindings before any work; iterate in canonical order.
    let mut bindings = BTreeMap::new();
    for &(atom, replacement) in replacements {
        if source.support.binary_search(&atom).is_err()
            || bindings.insert(atom, replacement).is_some()
        {
            return Err(KernelBoundary::TransferUnsupported);
        }
    }
    let support: Vec<AtomId> = source
        .support
        .iter()
        .filter(|atom| !bindings.contains_key(atom))
        .chain(bindings.values().flat_map(|r| r.support.iter()))
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if support.len() > MAX_ATOMS {
        return Err(KernelBoundary::AtomLimit);
    }
    let ctx = context(&support);
    let mut budget = Budget { work, nodes };
    let mut predicates = BTreeMap::new();
    for (atom, replacement) in bindings {
        budget.work(
            replacement
                .bdd
                .size()
                .checked_mul(support.len().max(1))
                .ok_or(KernelBoundary::WorkPreflight)?,
        )?;
        budget.retain(replacement.bdd.size())?;
        let bdd = ctx
            .transfer_from(&replacement.bdd, &replacement.ctx)
            .ok_or(KernelBoundary::TransferUnsupported)?;
        predicates.insert(atom, bdd);
    }

    struct Composer<'a> {
        source: &'a Diagram,
        support: &'a [AtomId],
        ctx: &'a BddVariableSet,
        predicates: BTreeMap<AtomId, Bdd>,
        memo: HashMap<BddPointer, Bdd>,
        budget: Budget,
    }
    impl Composer<'_> {
        fn visit(&mut self, pointer: BddPointer) -> Result<Bdd, KernelBoundary> {
            self.budget.work(1)?;
            if let Some(value) = self.memo.get(&pointer) {
                self.budget.retain(value.size())?;
                return Ok(value.clone());
            }
            if pointer.is_terminal() {
                let out = if pointer.is_one() {
                    self.ctx.mk_true()
                } else {
                    self.ctx.mk_false()
                };
                self.budget.retain(out.size())?;
                return Ok(out);
            }
            let atom = self.source.support[self.source.bdd.var_of(pointer).to_index()];
            let predicate = if let Some(value) = self.predicates.get(&atom) {
                self.budget.retain(value.size())?;
                value.clone()
            } else {
                let variable = self
                    .support
                    .binary_search(&atom)
                    .map_err(|_| KernelBoundary::TransferUnsupported)?;
                self.budget.retain(3)?;
                self.ctx.mk_literal(BddVariable::from_index(variable), true)
            };
            let low = self.source.bdd.low_link_of(pointer);
            let high = self.source.bdd.high_link_of(pointer);
            let out = if predicate.is_true() {
                self.visit(high)?
            } else if predicate.is_false() {
                self.visit(low)?
            } else {
                let low = self.visit(low)?;
                let high = self.visit(high)?;
                let yes = self.budget.apply(&predicate, &high, op_function::and)?;
                self.budget.work(predicate.size())?;
                self.budget.retain(predicate.size())?;
                let no = self
                    .budget
                    .apply(&predicate.not(), &low, op_function::and)?;
                self.budget.apply(&yes, &no, op_function::or)?
            };
            self.budget.retain(out.size())?;
            self.memo.insert(pointer, out.clone());
            Ok(out)
        }
    }
    let mut composer = Composer {
        source,
        support: &support,
        ctx: &ctx,
        predicates,
        memo: HashMap::new(),
        budget,
    };
    let bdd = composer.visit(source.bdd.root_pointer())?;
    // Effective-support normalization also allocates; charge it to the same budget.
    composer.budget.work(
        bdd.size()
            .checked_mul(support.len().max(1))
            .ok_or(KernelBoundary::WorkPreflight)?,
    )?;
    composer.budget.retain(bdd.size())?;
    drop(composer);
    Diagram::effective(support, ctx, bdd)
}
