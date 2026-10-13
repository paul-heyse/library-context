//! Independently specified correspondence scenarios; these data controls confer no admission.
use lctx_model::domain::{
    ContentHash,
    completed::{
        CompletedContribution, CompletedStateIdentity, CompletedView, ContributionSpec,
        OutputContent, OutputProfile,
    },
    completed_identity::{CompletedFamilyHasher, CompletedFamilyRoots},
    completed_transport::*,
    producer_assurance::ProducerAssurance,
    recovery_closure::COMPLETED_FAMILIES,
    stages::Profile,
};
use std::collections::{BTreeMap, BTreeSet};

fn inventory(kind: TransportInventoryKind, rows: &[&[u8]]) -> TransportInventory {
    let mut fold = TransportInventoryHasher::new(kind);
    for row in rows {
        fold.row(row).unwrap();
    }
    fold.finish()
}
fn fixture() -> CompletedTransportEnvelope {
    let descriptor = CompletedContribution {
        spec: ContributionSpec {
            captured_binding: None,
            producer: "transport-control".into(),
            profile: OutputProfile::Specific(Profile::Catalog),
            model: ContentHash::of(b"model"),
            implementation: ContentHash::of(b"implementation"),
            configuration: None,
            inputs: vec![],
            outputs: BTreeSet::from(["packages".into(), "releases".into()]),
        },
        assurance: ProducerAssurance::CurrentProviderInterface,
        outcome: 0,
        outputs: BTreeMap::from([
            (
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty-packages"),
                },
            ),
            (
                "releases".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty-releases"),
                },
            ),
        ]),
    };
    let logical = descriptor.identity().unwrap();
    let mut roots = ["packages", "releases"]
        .map(|relation| {
            let view = CompletedView::new(relation.into(), [logical].into(), 0).unwrap();
            TransportRoot {
                slots: vec![TransportSlot {
                    view: view.identity,
                    logical,
                    copy: 0,
                }],
                view,
                dependencies: vec![],
            }
        })
        .to_vec();
    roots.sort_by_key(|root| root.view.identity);
    roots[1].slots[0].copy = 1;
    let copies = (0..2)
        .map(|label| TransportCopy {
            label,
            descriptor: descriptor.clone(),
            membership: inventory(TransportInventoryKind::Memberships, &[]),
            aliases: inventory(TransportInventoryKind::Aliases, &[]),
        })
        .collect();
    let families = CompletedFamilyRoots::new(
        COMPLETED_FAMILIES.map(|family| CompletedFamilyHasher::new(family).unwrap().finish()),
    )
    .unwrap();
    CompletedTransportEnvelope {
        version: TRANSPORT_CODEC,
        semantic: CompletedStateIdentity::from_roots(&families).unwrap(),
        roots,
        copies,
        families,
    }
}

#[test]
fn equal_v4_swapped_local_alias_incidence_changes_eligibility() {
    let mut a = fixture();
    a.copies[0].aliases = inventory(TransportInventoryKind::Aliases, &[b"source-a -> target-a"]);
    a.copies[1].aliases = inventory(TransportInventoryKind::Aliases, &[b"source-b -> target-b"]);
    let mut b = a.clone();
    b.copies[0].aliases = a.copies[1].aliases.clone();
    b.copies[1].aliases = a.copies[0].aliases.clone();
    assert_eq!(a.semantic, b.semantic);
    assert_ne!(
        a.eligibility_identity().unwrap(),
        b.eligibility_identity().unwrap()
    );
    assert_ne!(a.receipt().unwrap().envelope, b.receipt().unwrap().envelope);
}

#[test]
fn equivalent_fresh_copies_and_shared_copies_have_equal_eligibility() {
    let independent = fixture();
    let mut shared = independent.clone();
    shared.copies.truncate(1);
    shared.roots[1].slots[0].copy = 0;
    assert_eq!(
        shared.eligibility_identity().unwrap(),
        independent.eligibility_identity().unwrap()
    );
    assert_ne!(
        shared.receipt().unwrap().envelope,
        independent.receipt().unwrap().envelope
    );
    assert_eq!(shared.semantic, independent.semantic);
}

#[test]
fn missing_foreign_unreachable_and_noncanonical_correspondence_refuse() {
    let base = fixture();
    let mut missing = base.clone();
    missing.roots[0].slots.clear();
    assert!(missing.validate().is_err());
    let mut foreign = base.clone();
    foreign.roots[0].slots[0].logical = ContentHash::of(b"foreign");
    assert!(foreign.validate().is_err());
    let mut duplicate = base.clone();
    let repeated = duplicate.roots[0].slots[0].clone();
    duplicate.roots[0].slots.push(repeated);
    assert!(duplicate.validate().is_err());
    let mut unused = base.clone();
    let mut copy = unused.copies[0].clone();
    copy.label = 2;
    unused.copies.push(copy);
    assert!(unused.validate().is_err());
    let mut order = base.clone();
    order.roots.reverse();
    assert!(order.validate().is_err());
    let mut labels = base.clone();
    labels.roots[0].slots[0].copy = 1;
    labels.roots[1].slots[0].copy = 0;
    assert!(labels.validate().is_err());
    let mut old = base.clone();
    old.version = 0;
    assert!(old.validate().is_err());
    let mut receipt = base.receipt().unwrap();
    receipt.transport_codec = 0;
    assert!(receipt.validate().is_err());
    assert_eq!(base.receipt().unwrap().inventory_counts.aliases, 0);
    assert_eq!(base.receipt().unwrap().inventory_counts.slots, 2);
}
