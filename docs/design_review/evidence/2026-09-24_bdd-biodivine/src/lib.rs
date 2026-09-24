//! Stage 3.0 contract spike, intentionally separate from the product kernel.
use std::collections::{BTreeSet, HashMap};

use biodivine_lib_bdd::{Bdd, BddPointer, BddVariableSet, op_function};
use cpg_schema::condition::Condition;
use cpg_schema::id::{Id, IdHasher};

const MAX_ATOMS: usize = 64;
const MAX_NODES: usize = 50_000;
const MAX_PAIR_WORK: usize = 1_000_000;

fn atom_name(atom: &str) -> String {
    format!(
        "a_{}",
        IdHasher::new("bdd-atom").str(atom).finish_id().hex()
    )
}

fn apply(
    left: &Bdd,
    right: &Bdd,
    op: fn(Option<bool>, Option<bool>) -> Option<bool>,
) -> Option<Bdd> {
    if left.size().checked_mul(right.size())? > MAX_PAIR_WORK {
        return None;
    }
    Bdd::binary_op_with_limit(MAX_NODES, left, right, op)
}

/// A condition's support is intrinsic; context positions are library-local.
pub struct Diagram {
    support: Vec<String>,
    ctx: BddVariableSet,
    bdd: Bdd,
}

impl Diagram {
    pub fn from_condition(condition: &Condition) -> Option<Self> {
        let Condition::Dnf(terms) = condition else {
            return None;
        };
        let support: Vec<String> = terms
            .iter()
            .flatten()
            .map(|l| l.atom.encode())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if support.len() > MAX_ATOMS {
            return None;
        }
        let names: Vec<String> = support.iter().map(|a| atom_name(a)).collect();
        if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
            return None;
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ctx = BddVariableSet::new(&refs);
        let vars = ctx.variables();
        let mut bdd = ctx.mk_false();
        for term in terms {
            let mut conjunction = ctx.mk_true();
            for literal in term {
                let index = support.binary_search(&literal.atom.encode()).ok()?;
                let part = ctx.mk_literal(vars[index], literal.positive);
                conjunction = apply(&conjunction, &part, op_function::and)?;
            }
            bdd = apply(&bdd, &conjunction, op_function::or)?;
        }
        Some(Self { support, ctx, bdd })
    }

    pub fn node_count(&self) -> usize {
        self.bdd.size()
    }

    /// Structural Merkle id, independent of local variable indices and construction order.
    pub fn id(&self) -> Id {
        fn node(diagram: &Diagram, pointer: BddPointer, memo: &mut HashMap<BddPointer, Id>) -> Id {
            if pointer.is_zero() {
                return IdHasher::new("bdd-false").finish_id();
            }
            if pointer.is_one() {
                return IdHasher::new("bdd-true").finish_id();
            }
            if let Some(id) = memo.get(&pointer) {
                return *id;
            }
            let atom = &diagram.support[diagram.bdd.var_of(pointer).to_index()];
            let low = node(diagram, diagram.bdd.low_link_of(pointer), memo);
            let high = node(diagram, diagram.bdd.high_link_of(pointer), memo);
            let id = IdHasher::new("bdd-node")
                .str(atom)
                .id(low)
                .id(high)
                .finish_id();
            memo.insert(pointer, id);
            id
        }
        let root = node(self, self.bdd.root_pointer(), &mut HashMap::new());
        IdHasher::new("condition-bdd").id(root).finish_id()
    }

    fn in_union(&self, union: &[String]) -> Option<Bdd> {
        if union.len() > MAX_ATOMS || self.node_count().checked_mul(union.len())? > MAX_PAIR_WORK {
            return None;
        }
        let names: Vec<String> = union.iter().map(|a| atom_name(a)).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        BddVariableSet::new(&refs).transfer_from(&self.bdd, &self.ctx)
    }

    pub fn compatible(&self, other: &Self) -> Option<bool> {
        let union: Vec<String> = self
            .support
            .iter()
            .chain(other.support.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        Some(!apply(&left, &right, op_function::and)?.is_false())
    }

    pub fn implies(&self, other: &Self) -> Option<bool> {
        let union: Vec<String> = self
            .support
            .iter()
            .chain(other.support.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        if right.size() > MAX_NODES {
            return None;
        }
        Some(apply(&left, &right.not(), op_function::and)?.is_false())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> Diagram {
        Diagram::from_condition(&Condition::parse(s).unwrap()).unwrap()
    }

    #[test]
    fn stage2_known_answers_have_canonical_ids() {
        let pairs = [
            ("true", "true"),
            ("false", "false"),
            (
                "equals(mode,\"http\") & !is_none(x)",
                "!is_none(x) & equals(mode,\"http\")",
            ),
            ("is_none(x) & !is_none(x)", "false"),
            ("truthy(a) | truthy(a) & truthy(b)", "truthy(a)"),
            (
                "truthy(a) | !truthy(a) & truthy(b)",
                "truthy(a) | truthy(b)",
            ),
            (
                "truthy(a) & !truthy(y) | truthy(a) & truthy(y)",
                "truthy(a)",
            ),
            (
                "member_of(t,{\"sse\",\"http\",\"sse\"})",
                "member_of(t,{\"http\",\"sse\"})",
            ),
            (
                "opaque(\"f(x)  and g\") & truthy(y)",
                "opaque(\"f(x) and g\") & truthy(y)",
            ),
        ];
        for (a, b) in pairs {
            assert_eq!(d(a).id(), d(b).id(), "{a}");
        }
    }

    #[test]
    fn union_transfer_and_factoring_are_exact() {
        let f = d("truthy(a) & truthy(c) | truthy(b) & truthy(c)");
        let normal = d("truthy(a) | truthy(b)");
        let quotient = d("truthy(c)");
        assert!(f.implies(&normal) == Some(true));
        assert!(f.compatible(&d("!truthy(a) & !truthy(b)")) == Some(false));
        let factored = Condition::parse("truthy(a) & truthy(c) | truthy(b) & truthy(c)")
            .unwrap()
            .given(&Condition::parse("truthy(a) | truthy(b)").unwrap());
        assert_eq!(
            Diagram::from_condition(&factored).unwrap().id(),
            quotient.id()
        );
        assert_eq!(
            f.id(),
            d("truthy(c) & truthy(b) | truthy(c) & truthy(a)").id()
        );
    }

    #[test]
    fn structural_id_ignores_redundant_support_and_input_order() {
        let Condition::Dnf(mut z) = Condition::parse("truthy(z)").unwrap() else {
            unreachable!()
        };
        let Condition::Dnf(mut az) = Condition::parse("truthy(a) & truthy(z)").unwrap() else {
            unreachable!()
        };
        let first =
            Diagram::from_condition(&Condition::Dnf(vec![z.remove(0), az.remove(0)])).unwrap();
        let second = Diagram::from_condition(&Condition::parse("truthy(z)").unwrap()).unwrap();
        assert_eq!(first.id(), second.id());
        assert!(first.support.len() > second.support.len());
    }

    #[test]
    fn site_atoms_stay_independent_and_budget_refuses_work() {
        assert_eq!(
            d("truthy(p) & !truthy(q)").compatible(&d("true")),
            Some(true)
        );
        assert_eq!(
            d("truthy(p) & !truthy(p)").compatible(&d("true")),
            Some(false)
        );
        let ctx = BddVariableSet::new_anonymous(2);
        let vars = ctx.variables();
        let a = ctx.mk_literal(vars[0], true);
        let b = ctx.mk_literal(vars[1], true);
        assert!(Bdd::binary_op_with_limit(1, &a, &b, op_function::and).is_none());
    }
}
