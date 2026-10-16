use lctx_model::domain::{
    analytics::{self, communities},
    normalized::entities::EntityRef,
    resources::ResourceBudget,
    *,
};
fn id(n: u8) -> Id<EntityRef> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
#[test]
fn canonical_definitions_pin_each_method_and_every_optional_flag() {
    let s = analysis::settings::AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec!["api.entry".into()],
        depth: 4,
        vertices: 64,
        arcs: 128,
        witnesses: 2,
        brief_budget: 4,
        communities: false,
        pagerank: false,
        fca: false,
        rca: false,
        knn: false,
        type_layer: false,
        mention_layer: false,
        knn_layer: false,
    };
    let rows =
        analytics::build::METHODS.map(|m| analytics::build::definition(&s, m).unwrap().1.id());
    let unique = rows.into_iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 5);
    assert!(
        analytics::build::METHODS
            .iter()
            .all(|m| !analytics::build::selected(&s, *m))
    );
    let mut selected = s.clone();
    selected.communities = true;
    selected.type_layer = true;
    for method in analytics::build::METHODS {
        assert_ne!(
            analytics::build::definition(&selected, method)
                .unwrap()
                .1
                .id(),
            analytics::build::definition(&s, method).unwrap().1.id()
        );
    }
}
#[test]
fn layers_retain_checked_parallel_counts_hub_scaling_and_reservation() {
    let budget = ResourceBudget::fixed(1 << 22).unwrap();
    let vertices = [id(1), id(2), id(3), id(4)];
    let rows = [(id(1), id(2), 3), (id(2), id(1), 2), (id(2), id(3), 1)];
    let a = communities::normalize(&vertices, &rows, &budget).unwrap();
    assert!(budget.reserved() > 0);
    // Strengths are 5, 6, 1; the retained 95th-percentile threshold is 6,
    // so neither edge is downscaled before normalization by total count 6.
    assert_eq!(
        a.pairs(),
        &[
            communities::Pair {
                left: id(1),
                right: id(2),
                weight: FiniteF64::new(5.0 / 6.0).unwrap()
            },
            communities::Pair {
                left: id(2),
                right: id(3),
                weight: FiniteF64::new(1.0 / 6.0).unwrap()
            },
        ]
    );
    // Two canonical edges remain after reversed observations are combined.
    let retained = vertices.len() * 256 + 2 * 512;
    assert_eq!(budget.reserved(), retained);
    let b = communities::normalize(
        &vertices,
        &rows.into_iter().rev().collect::<Vec<_>>(),
        &budget,
    )
    .unwrap();
    assert_eq!(a.pairs(), b.pairs());
    let unordered =
        communities::normalize(&[id(4), id(2), id(1), id(3), id(2)], &rows, &budget).unwrap();
    assert_eq!(a.pairs(), unordered.pairs());
    let before_refusal = budget.reserved();
    assert!(communities::normalize(&vertices, &[(id(1), id(1), 1)], &budget).is_err());
    assert!(communities::normalize(&vertices, &[(id(1), id(5), 1)], &budget).is_err());
    assert!(
        communities::normalize(
            &vertices,
            &[(id(1), id(2), u64::MAX), (id(2), id(1), 1)],
            &budget
        )
        .is_err()
    );
    assert_eq!(budget.reserved(), before_refusal);
    // The temporary membership keys must be charged in addition to the
    // retained normalization workspace, and refusal releases both owners.
    let insufficient = ResourceBudget::fixed(retained).unwrap();
    assert!(communities::normalize(&vertices, &rows, &insufficient).is_err());
    assert_eq!(insufficient.reserved(), 0);
    drop(a);
    drop(b);
    drop(unordered);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn seeded_leiden_profiles_compare_nominal_membership_under_permutation() {
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let vertices = (1..=6).map(id).collect::<Vec<_>>();
    let edges = [(1, 2), (2, 3), (1, 3), (4, 5), (5, 6), (4, 6)];
    let pairs = edges.map(|(a, b)| communities::Pair {
        left: id(a),
        right: id(b),
        weight: FiniteF64::new(1.0 / 6.0).unwrap(),
    });
    let result = communities::partition(&vertices, &pairs, 100_000_000, &budget).unwrap();
    assert_eq!(result.runs().len(), 40);
    assert_eq!(result.profiles().len(), 4);
    assert!(result.chosen());
    let reversed = communities::partition(
        &vertices.iter().rev().copied().collect::<Vec<_>>(),
        &pairs.iter().rev().copied().collect::<Vec<_>>(),
        100_000_000,
        &budget,
    )
    .unwrap();
    assert_eq!(result.runs(), reversed.runs());
    assert_eq!(result.profiles(), reversed.profiles());
    let r = result
        .runs()
        .iter()
        .find(|r| r.resolution.get() == 1.0 && r.seed == 0)
        .unwrap();
    assert_eq!(r.labels[0], r.labels[1]);
    assert_eq!(r.labels[1], r.labels[2]);
    assert_eq!(r.labels[3], r.labels[4]);
    assert_eq!(r.labels[4], r.labels[5]);
    assert_ne!(r.labels[0], r.labels[3]);
    assert!(communities::partition(&vertices, &pairs, 1, &budget).is_err());
    assert!(
        communities::partition(
            &vertices,
            &pairs,
            100_000_000,
            &ResourceBudget::fixed(1).unwrap()
        )
        .is_err()
    );
    drop(result);
    drop(reversed);
    assert_eq!(budget.reserved(), 0);
}
#[test]
fn empty_and_degenerate_community_domains_do_not_invent_completed_members() {
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let empty = communities::partition(&[], &[], 100, &budget).unwrap();
    assert!(empty.runs().is_empty());
    assert!(!empty.chosen());
    let pair = communities::Pair {
        left: id(1),
        right: id(2),
        weight: FiniteF64::new(1.0).unwrap(),
    };
    let p = communities::partition(&[id(1), id(2)], &[pair], 100_000_000, &budget).unwrap();
    assert!(!p.chosen());
    assert!(p.profiles().iter().all(|p| p.degenerate));
    drop(p);
    drop(empty);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn retained_dual_bitset_context_agrees_with_finite_set_oracle_and_keeps_partiality() {
    use analytics::concepts::Context;
    use std::collections::BTreeSet;
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let objects = [0u8, 1, 2];
    let attributes = [0u8, 1, 2];
    let contexts = [
        vec![],
        objects
            .into_iter()
            .flat_map(|o| attributes.into_iter().map(move |a| (o, a)))
            .collect(),
        vec![(0, 0), (1, 1), (2, 2)],
        vec![(0, 0), (0, 1), (1, 0), (2, 2)],
        vec![(0, 0), (0, 0), (1, 0), (1, 1)],
    ];
    for incidence in contexts {
        let context = Context::new(&objects, &attributes, &incidence, &budget).unwrap();
        let lattice = context.analyse(0, 256).unwrap();
        assert!(!lattice.budget_reached());
        // Independent finite set definition: A' contains exactly all objects possessing A;
        // A'' intersects those objects' attributes, including the vacuous full intersection.
        let mut expected = BTreeSet::new();
        for mask in 0..8 {
            let intent = attributes
                .into_iter()
                .filter(|a| mask & (1 << a) != 0)
                .collect::<Vec<_>>();
            let extent = objects
                .into_iter()
                .filter(|o| intent.iter().all(|a| incidence.contains(&(*o, *a))))
                .collect::<Vec<_>>();
            let closure = attributes
                .into_iter()
                .filter(|a| extent.iter().all(|o| incidence.contains(&(*o, *a))))
                .collect::<Vec<_>>();
            if intent == closure {
                expected.insert((extent, intent));
            }
        }
        let actual = lattice
            .concepts()
            .iter()
            .map(|c| (c.extent.clone(), c.intent.clone()))
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
        for implication in lattice.implications() {
            for object in objects {
                if implication
                    .premise
                    .iter()
                    .all(|a| incidence.contains(&(object, *a)))
                {
                    assert!(
                        implication
                            .conclusion
                            .iter()
                            .all(|a| incidence.contains(&(object, *a)))
                    );
                }
            }
        }
    }
    let context = Context::new(&objects, &attributes, &[(0, 0), (1, 1), (2, 2)], &budget).unwrap();
    let partial = context.analyse(0, 1).unwrap();
    assert!(partial.budget_reached());
    assert_eq!(partial.examined(), 1);
    drop(partial);
    drop(context);
    assert_eq!(budget.reserved(), 0);
}
