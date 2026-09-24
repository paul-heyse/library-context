//! Formal concept analysis (DESIGN §9.6; ADR-0011): our own NextClosure (Ganter) over
//! `fixedbitset`, which enumerates in lectic order every set closed under the implications found
//! so far. Each such set is either an intent (a concept) or a pseudo-intent, whose implication
//! `P → P''∖P` joins the Duquenne–Guigues basis. One pass yields both.
//!
//! **Support.** A set's support is its extent's size. Since a larger set never has a larger
//! extent, the lectically next *frequent* closed set is the first canonical candidate that is
//! frequent, so pruning infrequent candidates enumerates exactly the frequent concepts and the
//! frequent part of the basis (every pseudo-intent inside a frequent set is itself frequent).
//!
//! **Budget.** Enumeration stops after `budget` closed sets, and says so; FCbO replaces this only
//! if the budget binds on a real scope (ADR-0011).

use std::collections::{BTreeMap, BTreeSet};

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, StringArray};
use cpg_schema::codebook::{Codebook, CoverageStatus, FindingKind, MemberRole, StopReason};
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, recipe::FindingKey,
};
use cpg_schema::id::{Digest, Id, content_digest};
use fixedbitset::FixedBitSet;
use serde::Serialize;

use crate::AnalyticsError;

/// A formal context: objects × attributes, both as dense indices.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// The pre-registered parameters (slice 2.5; deviation log D32), frozen with the others.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    /// A concept's or implication's least support, in public APIs.
    pub min_support: usize,
    /// The most closed sets enumerated per scope.
    pub budget: usize,
}

impl Params {
    pub fn preregistered() -> Self {
        Params {
            min_support: 2,
            budget: 20_000,
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("parameters serialize")
    }

    pub fn digest(&self) -> Digest {
        content_digest(self.json().as_bytes())
    }
}

/// One structural scope: the public APIs of one exported class or module namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The class or module declaration the scope is.
    pub node: Id,
    /// Its access path.
    pub label: String,
    /// Its public APIs with their access paths, sorted by id.
    pub objects: Vec<(Id, String)>,
}

#[derive(Debug, Serialize)]
struct Diagnostics {
    objects: usize,
    attributes: usize,
    concepts: usize,
    implications: usize,
    examined: usize,
    budget_reached: bool,
}

/// One scope's results.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub objects: usize,
    pub attributes: usize,
    pub examined: usize,
    pub completion: CoverageStatus,
    pub stop_reason: Option<StopReason>,
    pub diagnostics: String,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
}

/// Each function's attributes from `cpg_schema::concepts::attributes_sql`'s rows.
pub fn attributes_of(
    batches: &[RecordBatch],
) -> Result<BTreeMap<Id, BTreeSet<String>>, AnalyticsError> {
    let mut out: BTreeMap<Id, BTreeSet<String>> = BTreeMap::new();
    for b in batches {
        let node = b
            .column_by_name("function_node_id")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
            .ok_or_else(|| AnalyticsError::Column("function_node_id".to_owned()))?;
        let attribute = b
            .column_by_name("attribute")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| AnalyticsError::Column("attribute".to_owned()))?;
        for i in 0..b.num_rows() {
            out.entry(Id(<[u8; 16]>::try_from(node.value(i)).expect("16 bytes")))
                .or_default()
                .insert(attribute.value(i).to_owned());
        }
    }
    Ok(out)
}

/// FCA of one scope: each frequent concept with a non-empty intent is an `applicable_case`
/// finding (its extent's APIs and its intent's attributes as members), and each frequent
/// implication of the basis an `implication` finding. The findings cite `invocation_id`.
pub fn run(
    scope: &Scope,
    attributes: &BTreeMap<Id, BTreeSet<String>>,
    params: &Params,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Outcome, AnalyticsError> {
    let names: Vec<&String> = scope
        .objects
        .iter()
        .flat_map(|(id, _)| attributes.get(id).into_iter().flatten())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let index: BTreeMap<&String, usize> = names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let mut incidence = Vec::new();
    for (g, (id, _)) in scope.objects.iter().enumerate() {
        for a in attributes.get(id).into_iter().flatten() {
            incidence.push((g, index[a]));
        }
    }
    let context = Context::new(scope.objects.len(), names.len(), &incidence);
    let lattice = analyse(&context, params.min_support, params.budget);
    let status = |kind: FindingKind| {
        FINDING_STATUS
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, s)| *s)
            .ok_or_else(|| AnalyticsError::Graph(format!("no status policy for {kind:?}")))
    };
    let mut findings = Vec::new();
    let mut members = Vec::new();
    let mut emit = |kind: FindingKind,
                    rows: Vec<(MemberRole, Option<Id>, String)>,
                    score: usize|
     -> Result<(), AnalyticsError> {
        let status = status(kind)?;
        let keys: Vec<MemberKey> = rows
            .iter()
            .enumerate()
            .map(|(ordinal, (role, node, label))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: *node,
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let finding_id = FindingKey {
            finding_kind: kind.code(),
            subject: scope.node,
            related: None,
            condition: None,
            evidence_status: status.code(),
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            paths: &[],
            members: &keys,
        }
        .id();
        for (ordinal, (role, node, label)) in rows.into_iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role,
                ordinal: ordinal as i64,
                node_id: node,
                cited_fact_id: None,
                label: Some(label),
                weight: None,
            });
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: kind,
            subject_node_id: scope.node,
            related_node_id: None,
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            score: Some(score as f64),
            condition_node_id: None,
        });
        Ok(())
    };
    let label = |role: MemberRole, set: &FixedBitSet| -> Vec<(MemberRole, Option<Id>, String)> {
        set.ones().map(|m| (role, None, names[m].clone())).collect()
    };
    let mut concepts = 0usize;
    for (extent, intent) in &lattice.concepts {
        if intent.count_ones(..) == 0 {
            continue;
        }
        let mut rows: Vec<(MemberRole, Option<Id>, String)> = extent
            .ones()
            .map(|g| {
                let (id, path) = &scope.objects[g];
                (MemberRole::ExtentMember, Some(*id), path.clone())
            })
            .collect();
        rows.extend(label(MemberRole::IntentAttribute, intent));
        emit(FindingKind::ApplicableCase, rows, extent.count_ones(..))?;
        concepts += 1;
    }
    for i in &lattice.implications {
        let mut rows = label(MemberRole::Premise, &i.premise);
        rows.extend(label(MemberRole::Conclusion, &i.conclusion));
        emit(FindingKind::Implication, rows, i.support)?;
    }
    let diagnostics = serde_json::to_string(&Diagnostics {
        objects: scope.objects.len(),
        attributes: names.len(),
        concepts,
        implications: lattice.implications.len(),
        examined: lattice.examined,
        budget_reached: lattice.budget_reached,
    })
    .expect("diagnostics serialize");
    Ok(Outcome {
        objects: scope.objects.len(),
        attributes: names.len(),
        examined: lattice.examined,
        completion: if lattice.budget_reached {
            CoverageStatus::Partial
        } else {
            CoverageStatus::CompleteUnderStatedModel
        },
        stop_reason: lattice.budget_reached.then_some(StopReason::ConceptBudget),
        diagnostics,
        findings,
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn set(n: usize, ones: &[usize]) -> FixedBitSet {
        let mut s = FixedBitSet::with_capacity(n);
        for &i in ones {
            s.insert(i);
        }
        s
    }

    fn ones(s: &FixedBitSet) -> Vec<usize> {
        s.ones().collect()
    }

    /// A hand-computed context (§12): four objects, four attributes.
    ///
    /// ```text
    ///      a b c d
    ///   1  x x . .
    ///   2  x . x .
    ///   3  x x x .
    ///   4  . . . x
    /// ```
    /// Concepts (extent, intent): ({1,2,3,4}, ∅), ({1,2,3}, {a}), ({1,3}, {a,b}), ({2,3}, {a,c}),
    /// ({3}, {a,b,c}), ({4}, {d}), (∅, {a,b,c,d}): seven. Basis: b → a, c → a, {a,d} → {b,c}.
    #[test]
    fn a_hand_computed_context_gives_its_concepts_and_basis() {
        let ctx = Context::new(
            4,
            4,
            &[
                (0, 0),
                (0, 1),
                (1, 0),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
                (3, 3),
            ],
        );
        let lattice = analyse(&ctx, 0, 1000);
        let concepts: BTreeSet<(Vec<usize>, Vec<usize>)> = lattice
            .concepts
            .iter()
            .map(|(e, i)| (ones(e), ones(i)))
            .collect();
        let expected: BTreeSet<(Vec<usize>, Vec<usize>)> = [
            (vec![0, 1, 2, 3], vec![]),
            (vec![0, 1, 2], vec![0]),
            (vec![0, 2], vec![0, 1]),
            (vec![1, 2], vec![0, 2]),
            (vec![2], vec![0, 1, 2]),
            (vec![3], vec![3]),
            (vec![], vec![0, 1, 2, 3]),
        ]
        .into_iter()
        .collect();
        assert_eq!(concepts, expected);
        let basis: BTreeSet<(Vec<usize>, Vec<usize>)> = lattice
            .implications
            .iter()
            .map(|i| (ones(&i.premise), ones(&i.conclusion)))
            .collect();
        let expected: BTreeSet<(Vec<usize>, Vec<usize>)> = [
            (vec![1], vec![0]),
            (vec![2], vec![0]),
            (vec![0, 3], vec![1, 2]),
        ]
        .into_iter()
        .collect();
        assert_eq!(basis, expected);
        assert!(!lattice.budget_reached);
    }

    /// A deterministic pseudo-random context.
    fn random(objects: usize, attributes: usize, seed: u64) -> Context {
        let mut state = seed;
        let mut incidence = Vec::new();
        for g in 0..objects {
            for m in 0..attributes {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                if (state >> 33).is_multiple_of(3) {
                    incidence.push((g, m));
                }
            }
        }
        Context::new(objects, attributes, &incidence)
    }

    /// ADR-0011's oracle: fcars enumerates the same concept set, at every support threshold.
    #[test]
    fn fcars_agrees_on_the_concepts() {
        use bitvec::prelude::*;
        for seed in 1..6u64 {
            let ctx = random(12, 9, seed);
            let relation: Vec<BitVec> = (0..ctx.objects)
                .map(|g| {
                    (0..ctx.attributes)
                        .map(|m| ctx.rows[g].contains(m))
                        .collect()
                })
                .collect();
            let oracle = fcars::FormalContext::new(
                (0..ctx.objects).collect::<Vec<_>>(),
                (0..ctx.attributes).collect::<Vec<_>>(),
                relation,
            );
            let theirs: Vec<(Vec<usize>, Vec<usize>)> = oracle
                .all_concepts_raw()
                .into_iter()
                .map(|c| {
                    (
                        c.extent.iter_ones().collect(),
                        c.intent.iter_ones().collect(),
                    )
                })
                .collect();
            for min_support in [0, 2, 4] {
                let ours: BTreeSet<(Vec<usize>, Vec<usize>)> = analyse(&ctx, min_support, 100_000)
                    .concepts
                    .iter()
                    .map(|(e, i)| (ones(e), ones(i)))
                    .collect();
                let expected: BTreeSet<(Vec<usize>, Vec<usize>)> = theirs
                    .iter()
                    .filter(|(e, _)| e.len() >= min_support)
                    .cloned()
                    .collect();
                assert_eq!(ours, expected, "seed {seed}, support {min_support}");
            }
        }
    }

    /// The basis is sound and complete: every implication holds, and closing any attribute set
    /// under the whole basis gives its Galois closure.
    #[test]
    fn the_basis_closes_every_set_to_its_closure() {
        for seed in 1..6u64 {
            let ctx = random(10, 7, seed);
            let lattice = analyse(&ctx, 0, 100_000);
            let full: Vec<(FixedBitSet, FixedBitSet)> = lattice
                .implications
                .iter()
                .map(|i| {
                    let mut closed = i.premise.clone();
                    closed.union_with(&i.conclusion);
                    (i.premise.clone(), closed)
                })
                .collect();
            for i in &lattice.implications {
                assert!(ctx.closure(&i.premise).is_superset(&i.conclusion));
            }
            for bits in 0u32..(1 << ctx.attributes) {
                let x = set(
                    ctx.attributes,
                    &(0..ctx.attributes)
                        .filter(|m| bits >> m & 1 == 1)
                        .collect::<Vec<_>>(),
                );
                // Close with every premise that is a subset, proper or not.
                let mut closed = x.clone();
                loop {
                    let before = closed.clone();
                    for (p, q) in &full {
                        if p.is_subset(&closed) {
                            closed.union_with(q);
                        }
                    }
                    if closed == before {
                        break;
                    }
                }
                assert_eq!(closed, ctx.closure(&x), "seed {seed}, set {bits:b}");
            }
        }
    }

    #[test]
    fn a_budget_stops_enumeration_and_says_so() {
        let ctx = random(12, 9, 3);
        let lattice = analyse(&ctx, 0, 5);
        assert!(lattice.budget_reached && lattice.examined == 5);
    }
}
