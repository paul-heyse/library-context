//! Capture-safe simultaneous substitution using bounded library apply, not unbounded substitute.
use super::*;

const MAX_RETAINED_NODES: usize = 1_000_000;

struct Budget {
    work: usize,
    /// Conservatively charges every constructed diagram, including released temporaries.
    /// This bounds peak retained nodes without relying on allocator behavior or evaluation order.
    nodes: usize,
}

impl Budget {
    fn work(&mut self, amount: usize) -> Result<(), KernelBoundary> {
        self.work = self.work.checked_sub(amount).ok_or(KernelBoundary::WorkPreflight)?;
        Ok(())
    }

    fn retain(&mut self, amount: usize) -> Result<(), KernelBoundary> {
        self.work(amount)?;
        self.nodes = self.nodes.checked_sub(amount).ok_or(KernelBoundary::NodeLimit)?;
        Ok(())
    }

    fn apply(&mut self, a: &Bdd, b: &Bdd,
        op: fn(Option<bool>, Option<bool>) -> Option<bool>) -> Result<Bdd, KernelBoundary> {
        self.work(a.size().checked_mul(b.size()).ok_or(KernelBoundary::WorkPreflight)?)?;
        // Bound allocation before constructing the next intermediate, including terminals.
        let out = Bdd::binary_op_with_limit(MAX_NODES.min(self.nodes), a, b, op)
            .ok_or(KernelBoundary::NodeLimit)?;
        self.retain(out.size())?;
        Ok(out)
    }
}

pub(super) fn compose(source: &Diagram, replacements: &[(&str, &Diagram)])
    -> Result<Diagram, KernelBoundary> {
    compose_with_limits(source, replacements, MAX_PAIR_WORK, MAX_RETAINED_NODES)
}

fn compose_with_limits(source: &Diagram, replacements: &[(&str, &Diagram)],
    work: usize, nodes: usize) -> Result<Diagram, KernelBoundary> {
    // Reject malformed bindings before doing any work. Iteration order is canonical even if
    // callers supply mappings in a different order.
    let mut bindings = BTreeMap::new();
    for &(atom, replacement) in replacements {
        if source.support.binary_search_by(|a| a.as_str().cmp(atom)).is_err()
            || bindings.insert(atom, replacement).is_some() {
            return Err(KernelBoundary::TransferUnsupported);
        }
    }
    let support: Vec<String> = source.support.iter()
        .filter(|atom| !bindings.contains_key(atom.as_str()))
        .chain(bindings.values().flat_map(|replacement| replacement.support.iter()))
        .cloned().collect::<BTreeSet<_>>().into_iter().collect();
    if support.len() > MAX_ATOMS { return Err(KernelBoundary::AtomLimit); }
    let names: Vec<String> = support.iter().map(|atom| atom_name(atom)).collect();
    if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
        return Err(KernelBoundary::AtomNameCollision);
    }
    let ctx = BddVariableSet::new(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut budget = Budget { work, nodes };
    let mut predicates = BTreeMap::new();
    for (atom, replacement) in bindings {
        budget.work(replacement.bdd.size().checked_mul(support.len().max(1))
            .ok_or(KernelBoundary::WorkPreflight)?)?;
        budget.retain(replacement.bdd.size())?;
        let bdd = ctx.transfer_from(&replacement.bdd, &replacement.ctx)
            .ok_or(KernelBoundary::TransferUnsupported)?;
        predicates.insert(atom, bdd);
    }

    struct Composer<'a> {
        source: &'a Diagram,
        support: &'a [String],
        ctx: &'a BddVariableSet,
        predicates: BTreeMap<&'a str, Bdd>,
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
                let out = if pointer.is_one() { self.ctx.mk_true() } else { self.ctx.mk_false() };
                self.budget.retain(out.size())?;
                return Ok(out);
            }
            let atom = &self.source.support[self.source.bdd.var_of(pointer).to_index()];
            let predicate = if let Some(value) = self.predicates.get(atom.as_str()) {
                self.budget.retain(value.size())?;
                value.clone()
            } else {
                let variable = self.support.binary_search(atom)
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
                let no = self.budget.apply(&predicate.not(), &low, op_function::and)?;
                self.budget.apply(&yes, &no, op_function::or)?
            };
            self.budget.retain(out.size())?;
            self.memo.insert(pointer, out.clone());
            Ok(out)
        }
    }
    let mut composer = Composer { source, support: &support, ctx: &ctx, predicates,
        memo: HashMap::new(), budget };
    let bdd = composer.visit(source.bdd.root_pointer())?;
    // Effective-support normalization also allocates; include it in cumulative preparation.
    composer.budget.work(bdd.size().checked_mul(support.len().max(1))
        .ok_or(KernelBoundary::WorkPreflight)?)?;
    composer.budget.retain(bdd.size())?;
    drop(composer);
    Diagram::effective(support, ctx, bdd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(name: &str) -> (String, Diagram) {
        let atom = Atom::Truthy { place: name.to_owned() };
        (atom.encode(), Diagram::from_atom(&atom).unwrap())
    }

    #[test]
    fn simultaneous_substitution_preserves_swaps_and_does_not_capture_replacements() {
        let (a, da) = atom("a");
        let (b, db) = atom("b");
        let original = da.and(&db.not().unwrap()).unwrap();
        let swapped = original.substitute_atoms(&[(&a, &db), (&b, &da)]).unwrap();
        let reversed = original.substitute_atoms(&[(&b, &da), (&a, &db)]).unwrap();
        assert_eq!(swapped.id(), reversed.id());
        for va in [false, true] {
            for vb in [false, true] {
                assert_eq!(swapped.restrict_atoms(&[(&a, va), (&b, vb)]).unwrap().is_true(), vb && !va);
            }
        }
        let self_replaced = da.substitute_atoms(&[(&a, &da.not().unwrap())]).unwrap();
        assert_eq!(self_replaced.id(), da.not().unwrap().id());
        assert_eq!(original.substitute_atoms(&[]).unwrap().id(), original.id());
    }

    #[test]
    fn substitutions_share_target_atoms_and_reduce_effective_support() {
        let (a, da) = atom("a");
        let (b, db) = atom("b");
        let (_, dc) = atom("c");
        let source = da.and(&db.not().unwrap()).unwrap();
        let result = source.substitute_atoms(&[(&a, &dc), (&b, &dc)]).unwrap();
        assert!(result.is_false());
        assert!(result.support().is_empty());
        let result = source.substitute_atoms(&[(&a, &Diagram::always())]).unwrap();
        assert_eq!(result.id(), db.not().unwrap().id());
    }

    #[test]
    fn substitution_refuses_malformed_bindings_and_cumulative_limits() {
        let (a, da) = atom("a");
        let (b, db) = atom("b");
        assert_eq!(da.substitute_atoms(&[(&a, &db), (&a, &db)]).err(), Some(KernelBoundary::TransferUnsupported));
        assert_eq!(da.substitute_atoms(&[(&b, &db)]).err(), Some(KernelBoundary::TransferUnsupported));
        assert_eq!(compose_with_limits(&da, &[(&a, &db)], 0, MAX_RETAINED_NODES).err(), Some(KernelBoundary::WorkPreflight));
        assert_eq!(compose_with_limits(&da, &[(&a, &db)], MAX_PAIR_WORK, 0).err(), Some(KernelBoundary::NodeLimit));
        // Each primitive operation fits, but their cumulative construction does not.
        let conjunction = da.and(&db).unwrap();
        assert_eq!(compose_with_limits(&conjunction, &[], 5, MAX_RETAINED_NODES).err(), Some(KernelBoundary::WorkPreflight));
    }
}
