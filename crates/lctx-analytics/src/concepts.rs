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

use arrow_array::RecordBatch;
use cpg_schema::concept_attributes::{AttributeKey, ConceptAttributesRow, ConceptIncidencesRow};
use cpg_schema::query::QueryRow;
use cpg_schema::codebook::{ConceptAttributeKind, InvocationPhase, Modality};
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

/// Meaning, object incidence and source occurrences are independent from presentation.
#[derive(Debug, Default)]
pub struct Attributes {
    pub catalog: BTreeMap<Id, ConceptAttributesRow>,
    pub by_object: BTreeMap<Id, BTreeSet<Id>>,
    pub incidences: BTreeMap<Id, ConceptIncidencesRow>,
}

impl Attributes {
    pub fn add(&mut self, attribute: ConceptAttributesRow, mut incidence: ConceptIncidencesRow) {
        let id=attribute.attribute_id;
        incidence.attribute_id=id;
        incidence.incidence_id=incidence.identity();
        self.by_object.entry(incidence.object_node_id).or_default().insert(id);
        self.incidences.insert(incidence.incidence_id,incidence);
        // Different producer displays cannot alter membership or make input order observable.
        self.catalog.entry(id).and_modify(|old| {
            if attribute.display < old.display {old.display=attribute.display.clone();}
        }).or_insert(attribute);
    }
}

pub const RCA_POLICY: &str = "rca: one existential-scaling step over typed call targets and official-usage handoffs; target, endpoint modality and invocation phase define attributes; consumer formal is occurrence evidence; every site/edge pair remains an attributed incidence; labels are presentation";

#[derive(Debug, Clone)]
pub struct CallAttributeObservation {
    pub caller: Id, pub target: Id, pub site: Id, pub edge: Id,
    pub modality: Modality, pub phase: InvocationPhase,
}

pub fn relational(snapshot:Id, attributes:&mut Attributes, objects:&[Id],
    calls:&[CallAttributeObservation], handoffs:&[crate::pass_c::Handoff], names:&BTreeMap<Id,String>) {
    let wanted:BTreeSet<_>=objects.iter().copied().collect();
    for call in calls {
        if !wanted.contains(&call.caller) || call.caller==call.target {continue;}
        let Some(display)=names.get(&call.target) else {continue};
        let row=AttributeKey::Calls {target:call.target,modality:call.modality,phase:call.phase}.row(snapshot,display.clone());
        attributes.add(row,ConceptIncidencesRow {snapshot_id:snapshot,incidence_id:Id::ZERO,
            object_node_id:call.caller,attribute_id:Id::ZERO,source_fact_id:None,
            site_node_id:Some(call.site),edge_id:Some(call.edge),other_site_node_id:None,other_edge_id:None,consumer_formal_id:None});
    }
    for h in handoffs {
        for (object,target,takes) in [(h.producer,h.consumer,false),(h.consumer,h.producer,true)] {
            if !wanted.contains(&object) || object==target {continue;}
            let Some(display)=names.get(&target) else {continue};
            let row=AttributeKey::Handoff {takes,target,consumer_modality:h.consumer_modality,
                consumer_phase:h.consumer_phase,producer_modality:h.producer_modality,producer_phase:h.producer_phase}.row(snapshot,display.clone());
            attributes.add(row,ConceptIncidencesRow {snapshot_id:snapshot,incidence_id:Id::ZERO,
                object_node_id:object,attribute_id:Id::ZERO,source_fact_id:None,
                site_node_id:Some(h.producer_site),edge_id:Some(h.producer_edge),
                other_site_node_id:Some(h.consumer_site),other_edge_id:Some(h.consumer_edge),consumer_formal_id:Some(h.formal)});
        }
    }
}

pub fn attributes_of(snapshot:Id,batches:&[RecordBatch])->Result<Attributes,AnalyticsError> {
    let mut out=Attributes::default();
    for batch in batches {
        for row in cpg_schema::concepts::AttributeObservation::read_batch(batch)
            .map_err(|e|AnalyticsError::Column(e.to_string()))? {
            let invalid=||AnalyticsError::Column("invalid typed attribute observation".to_owned());
            let key=match row.kind {
                ConceptAttributeKind::Parameter=>AttributeKey::Parameter {name:row.symbol.ok_or_else(invalid)?,kind:row.parameter_kind.ok_or_else(invalid)?},
                ConceptAttributeKind::ParameterType=>AttributeKey::ParameterType(row.type_term_id.ok_or_else(invalid)?),
                ConceptAttributeKind::Returns=>AttributeKey::Returns(row.type_term_id.ok_or_else(invalid)?),
                ConceptAttributeKind::Raises=>AttributeKey::Raises {module:row.class_module.ok_or_else(invalid)?,key:row.class_key.ok_or_else(invalid)?},
                ConceptAttributeKind::Decorator=>AttributeKey::Decorator(row.symbol.ok_or_else(invalid)?),
                _=>return Err(invalid()),
            };
            out.add(key.row(snapshot,row.display),ConceptIncidencesRow {snapshot_id:snapshot,incidence_id:Id::ZERO,
                object_node_id:row.function_node_id,attribute_id:Id::ZERO,source_fact_id:Some(row.source_fact_id),
                site_node_id:None,edge_id:None,other_site_node_id:None,other_edge_id:None,consumer_formal_id:None});
        }
    }
    Ok(out)
}

/// FCA of one scope: each frequent concept with a non-empty intent is an `applicable_case`
/// finding (its extent's APIs and its intent's attributes as members), and each frequent
/// implication of the basis an `implication` finding. The findings cite `invocation_id`.
pub fn run(
    scope: &Scope,
    attributes: &Attributes,
    params: &Params,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Outcome, AnalyticsError> {
    let mut objects=scope.objects.clone();
    objects.sort_by_key(|(id,_)|*id);
    let names: Vec<Id> = objects
        .iter()
        .flat_map(|(id, _)| attributes.by_object.get(id).into_iter().flatten().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let index: BTreeMap<Id, usize> = names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let mut incidence = Vec::new();
    for (g, (id, _)) in objects.iter().enumerate() {
        for a in attributes.by_object.get(id).into_iter().flatten().copied() {
            incidence.push((g, index[&a]));
        }
    }
    let context = Context::new(objects.len(), names.len(), &incidence);
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
                    rows: Vec<(MemberRole, Option<Id>, Option<Id>, String)>,
                    score: usize|
     -> Result<(), AnalyticsError> {
        let status = status(kind)?;
        let keys: Vec<MemberKey> = rows
            .iter()
            .enumerate()
            .map(|(ordinal, (role, node, attribute, _label))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: *node,
                cited_fact: None,
                attribute: *attribute,
                label: None,
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
        for (ordinal, (role, node, attribute, label)) in rows.into_iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role,
                ordinal: ordinal as i64,
                node_id: node,
                cited_fact_id: None,
                attribute_id: attribute,
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
    let label = |role: MemberRole, set: &FixedBitSet| -> Vec<(MemberRole, Option<Id>, Option<Id>, String)> {
        set.ones().map(|m| (role, None, Some(names[m]), attributes.catalog[&names[m]].render().expect("validated attribute"))).collect()
    };
    let mut concepts = 0usize;
    for (extent, intent) in &lattice.concepts {
        if intent.count_ones(..) == 0 {
            continue;
        }
        let mut rows: Vec<(MemberRole, Option<Id>, Option<Id>, String)> = extent
            .ones()
            .map(|g| {
                let (id, path) = &objects[g];
                (MemberRole::ExtentMember, Some(*id), None, path.clone())
            })
            .collect();
        rows.extend(label(MemberRole::IntentAttribute, intent));
        emit(FindingKind::ApplicableCase, rows, extent.count_ones(..))?;
        concepts += 1;
    }
    for i in &lattice.implications {
        let mut rows:Vec<_>=context.extent(&i.premise).ones().map(|g| {
            let (id,path)=&objects[g];(MemberRole::ExtentMember,Some(*id),None,path.clone())
        }).collect();
        rows.extend(label(MemberRole::Premise, &i.premise));
        rows.extend(label(MemberRole::Conclusion, &i.conclusion));
        emit(FindingKind::Implication, rows, i.support)?;
    }
    let diagnostics = serde_json::to_string(&Diagnostics {
        objects: objects.len(),
        attributes: names.len(),
        concepts,
        implications: lattice.implications.len(),
        examined: lattice.examined,
        budget_reached: lattice.budget_reached,
    })
    .expect("diagnostics serialize");
    Ok(Outcome {
        objects: objects.len(),
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

    /// Close `x` under every implication whose premise it holds, proper or not.
    fn close_under(x: &FixedBitSet, basis: &[(FixedBitSet, FixedBitSet)]) -> FixedBitSet {
        let mut closed = x.clone();
        loop {
            let before = closed.clone();
            for (p, q) in basis {
                if p.is_subset(&closed) {
                    closed.union_with(q);
                }
            }
            if closed == before {
                return closed;
            }
        }
    }

    /// The increment-2 review's probe 1 (F6(c)): at the supports production uses (1–4), the
    /// pruned basis is sound, complete over every frequent attribute set, and non-redundant.
    #[test]
    fn the_frequent_basis_is_sound_complete_and_non_redundant() {
        for seed in 1..30u64 {
            let ctx = random(10, 7, seed);
            for min_support in 1..=4 {
                let lattice = analyse(&ctx, min_support, 100_000);
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
                    assert!(i.support >= min_support);
                    assert_eq!(i.support, ctx.extent(&i.premise).count_ones(..));
                }
                for bits in 0u32..(1 << ctx.attributes) {
                    let x = set(
                        ctx.attributes,
                        &(0..ctx.attributes)
                            .filter(|m| bits >> m & 1 == 1)
                            .collect::<Vec<_>>(),
                    );
                    if ctx.extent(&x).count_ones(..) >= min_support {
                        assert_eq!(
                            close_under(&x, &full),
                            ctx.closure(&x),
                            "seed {seed}, support {min_support}, set {bits:b}"
                        );
                    }
                }
                for (k, (premise, closed)) in full.iter().enumerate() {
                    let others: Vec<(FixedBitSet, FixedBitSet)> = full
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != k)
                        .map(|(_, r)| r.clone())
                        .collect();
                    assert_ne!(
                        &close_under(premise, &others),
                        closed,
                        "seed {seed}, support {min_support}: implication {k} is redundant"
                    );
                }
            }
        }
    }

    #[test]
    fn typed_relations_keep_modality_phase_parallel_evidence_and_stable_identity() {
        let id=|k:u8|Id([k;16]);
        let names=BTreeMap::from([(id(2),"same display".to_owned())]);
        let mut calls=vec![
            CallAttributeObservation {caller:id(1),target:id(2),site:id(10),edge:id(20),modality:Modality::Definite,phase:InvocationPhase::Call},
            CallAttributeObservation {caller:id(1),target:id(2),site:id(11),edge:id(21),modality:Modality::Definite,phase:InvocationPhase::Call},
            CallAttributeObservation {caller:id(1),target:id(2),site:id(12),edge:id(22),modality:Modality::Candidate,phase:InvocationPhase::Call},
            CallAttributeObservation {caller:id(1),target:id(2),site:id(13),edge:id(23),modality:Modality::Definite,phase:InvocationPhase::Init},
        ];
        let mut attrs=Attributes::default();
        relational(id(99),&mut attrs,&[id(1)],&calls,&[],&names);
        assert_eq!(attrs.by_object[&id(1)].len(),3);
        assert_eq!(attrs.incidences.len(),4);
        assert!(attrs.catalog.values().any(|a|a.render().unwrap().contains("candidate")));
        calls.reverse();
        let mut reversed=Attributes::default();
        relational(id(99),&mut reversed,&[id(1)],&calls,&[],&names);
        assert_eq!(attrs.catalog,reversed.catalog);
        assert_eq!(attrs.incidences,reversed.incidences);
        let scope=Scope {node:id(90),label:"scope".into(),objects:vec![(id(1),"one".into())]};
        let params=Params {min_support:1,budget:100};
        let before=run(&scope,&attrs,&params,id(99),id(98)).unwrap();
        for attr in attrs.catalog.values_mut() {attr.display="renamed".into();}
        let after=run(&scope,&attrs,&params,id(99),id(98)).unwrap();
        assert_eq!(before.findings,after.findings);
        assert_ne!(before.members,after.members);
        let a=AttributeKey::ParameterType(id(40)).row(id(99),"T".into());
        let b=AttributeKey::ParameterType(id(41)).row(id(99),"T".into());
        assert_ne!(a.attribute_id,b.attribute_id);
    }

    #[test]
    fn consumer_owned_formals_do_not_split_shared_handoff_attributes() {
        use crate::pass_c::{Handoff,Handoffs};
        use cpg_schema::codebook::SourceRole;
        let id=|k:u8|Id([k;16]);
        let first=Handoff {consumer:id(1),producer:id(3),formal:id(11),formal_name:"value".into(),
            path:"usage.py".into(),role:SourceRole::Example,consumer_start:50,consumer_site:id(31),producer_site:id(30),named:true,
            consumer_modality:Modality::Definite,consumer_phase:InvocationPhase::Call,
            producer_modality:Modality::Candidate,producer_phase:InvocationPhase::Call,
            consumer_edge:id(51),producer_edge:id(50)};
        let second=Handoff {consumer:id(2),formal:id(12),consumer_site:id(32),consumer_edge:id(52),..first.clone()};
        let mut attrs=Attributes::default();
        relational(id(99),&mut attrs,&[id(1),id(2)],&[],&[first.clone(),second],&BTreeMap::from([(id(3),"producer".into())]));
        assert_eq!(attrs.by_object[&id(1)],attrs.by_object[&id(2)]);
        assert_eq!(attrs.incidences.len(),2);
        let attribute=attrs.catalog.values().next().unwrap();
        assert!(attribute.render().unwrap().contains("candidate handoff pairing"));
        let definite=Handoff {producer_modality:Modality::Definite,..first.clone()};
        let passed=crate::pass_c::run(&Handoffs {rows:vec![first,definite]},id(1),3,id(99),id(98)).unwrap();
        assert_eq!(passed.findings.len(),2,"candidate and definite endpoint pairings stay separate");
        let ids: BTreeSet<_>=passed.members.iter().filter_map(|m|m.attribute_id).collect();
        assert_eq!(ids.len(),2);
    }

    #[test]
    fn a_budget_stops_enumeration_and_says_so() {
        let ctx = random(12, 9, 3);
        let lattice = analyse(&ctx, 0, 5);
        assert!(lattice.budget_reached && lattice.examined == 5);
    }
}
