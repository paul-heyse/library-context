pub struct Context {
    pub objects: usize,
    pub attributes: usize,
    /// Each object's attributes.
    rows: Vec<FixedBitSet>,
    /// Each attribute's objects.
    cols: Vec<FixedBitSet>,
}

impl Context {
    /// A context from its incidence pairs `(object, attribute)`.
    pub fn new(objects: usize, attributes: usize, incidence: &[(usize, usize)]) -> Self {
        let mut rows = vec![FixedBitSet::with_capacity(attributes); objects];
        let mut cols = vec![FixedBitSet::with_capacity(objects); attributes];
        for &(g, m) in incidence {
            rows[g].insert(m);
            cols[m].insert(g);
        }
        Context {
            objects,
            attributes,
            rows,
            cols,
        }
    }

    /// The objects having every attribute of `intent` (every object for the empty set).
    pub fn extent(&self, intent: &FixedBitSet) -> FixedBitSet {
        let mut out = FixedBitSet::with_capacity(self.objects);
        out.insert_range(..);
        for m in intent.ones() {
            out.intersect_with(&self.cols[m]);
        }
        out
    }

    /// The attributes every object of `extent` has (every attribute for the empty set).
    pub fn intent(&self, extent: &FixedBitSet) -> FixedBitSet {
        let mut out = FixedBitSet::with_capacity(self.attributes);
        out.insert_range(..);
        for g in extent.ones() {
            out.intersect_with(&self.rows[g]);
        }
        out
    }

    /// `B''`.
    pub fn closure(&self, set: &FixedBitSet) -> FixedBitSet {
        self.intent(&self.extent(set))
    }
}

/// An implication `premise → conclusion` holding in the context, with its support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Implication {
    pub premise: FixedBitSet,
    /// `premise'' ∖ premise`.
    pub conclusion: FixedBitSet,
    pub support: usize,
}

/// The frequent concepts, as `(extent, intent)` in lectic order of intents, and the frequent
/// Duquenne–Guigues basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lattice {
    pub concepts: Vec<(FixedBitSet, FixedBitSet)>,
    pub implications: Vec<Implication>,
    /// Closed sets enumerated (concepts and pseudo-intents).
    pub examined: usize,
    pub budget_reached: bool,
}

/// `X ↦ X ∪ ⋃{B : A → B, A ⊊ X}`, iterated to its fixed point.
fn implication_closure(set: &FixedBitSet, basis: &[(FixedBitSet, FixedBitSet)]) -> FixedBitSet {
    let mut x = set.clone();
    loop {
        let mut changed = false;
        for (premise, closed) in basis {
            if premise.is_subset(&x)
                && premise.count_ones(..) < x.count_ones(..)
                && !closed.is_subset(&x)
            {
                x.union_with(closed);
                changed = true;
            }
        }
        if !changed {
            return x;
        }
    }
}

/// Every frequent concept and the frequent implication basis of `context`, at `min_support`
/// objects, enumerating at most `budget` closed sets.
pub fn analyse(context: &Context, min_support: usize, budget: usize) -> Lattice {
    let m = context.attributes;
    let support = |set: &FixedBitSet| context.extent(set).count_ones(..);
    let mut out = Lattice {
        concepts: Vec::new(),
        implications: Vec::new(),
        examined: 0,
        budget_reached: false,
    };
    // Each pseudo-intent with its closure, for the implication closure.
    let mut basis: Vec<(FixedBitSet, FixedBitSet)> = Vec::new();
    let mut current = implication_closure(&FixedBitSet::with_capacity(m), &basis);
    if support(&current) < min_support {
        return out;
    }
    loop {
        if out.examined >= budget {
            out.budget_reached = true;
            return out;
        }
        out.examined += 1;
        let extent = context.extent(&current);
        let closed = context.intent(&extent);
        if closed == current {
            out.concepts.push((extent, current.clone()));
        } else {
            let mut conclusion = closed.clone();
            conclusion.difference_with(&current);
            out.implications.push(Implication {
                premise: current.clone(),
                conclusion,
                support: extent.count_ones(..),
            });
            basis.push((current.clone(), closed));
        }
        // The lectically next frequent closed set.
        let mut next = None;
        for i in (0..m).rev() {
            if current.contains(i) {
                continue;
            }
            let mut prefix = current.clone();
            prefix.remove_range(i..);
            prefix.insert(i);
            let candidate = implication_closure(&prefix, &basis);
            let canonical = (0..i).all(|j| candidate.contains(j) == current.contains(j));
            if canonical && support(&candidate) >= min_support {
                next = Some(candidate);
                break;
            }
        }
        match next {
            Some(n) => current = n,
            None => return out,
        }
    }
}

