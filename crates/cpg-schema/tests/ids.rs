//! Id derivation (DESIGN §3.4.1, ADR-0007 oracles).

use cpg_schema::codebook::{SummaryFlowKind, SummaryFlowStepKind};
use cpg_schema::id::{Id, IdHasher, content_digest, kind, recipe};
use proptest::prelude::*;

#[test]
fn known_answer_vectors_pin_the_encoding() {
    // A change here is `lctx-id/v2` and a migration (DM-51), never a silent edit.
    let id = IdHasher::new(kind::MODULE)
        .str("pkg/mod.py")
        .i64(7)
        .finish_id();
    let digest = content_digest(b"print('hi')\n");
    insta::assert_snapshot!(format!("{}\n{}", id.hex(), digest.hex()));
}

#[test]
fn optional_absent_and_empty_differ() {
    let absent = IdHasher::new("t").opt_str(None).finish_id();
    let empty = IdHasher::new("t").opt_str(Some("")).finish_id();
    assert_ne!(absent, empty);
}

#[test]
fn summary_identity_retains_the_ordered_evidence_path() {
    fn summary(output_path: &str, steps: &[recipe::SummaryFlowProofStep]) -> Id {
        recipe::summary_flow(&recipe::SummaryFlowIdentity {
            function: Id([1; 16]),
            parameter: Id([2; 16]),
            source_origin: Id([8; 16]),
            input_path: "Parameter[x]",
            output_path,
            transfer_kind: SummaryFlowKind::Value,
            condition: Id([3; 16]),
            return_site: Id([4; 16]),
            return_region: Id([5; 16]),
            steps,
        })
    }
    let steps = [
        recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::RawIdentity,
            evidence_id: Id([6; 16]),
            condition_id: Id([3; 16]),
        },
        recipe::SummaryFlowProofStep {
            kind: SummaryFlowStepKind::RawIdentity,
            evidence_id: Id([7; 16]),
            condition_id: Id([3; 16]),
        },
    ];
    let id = summary("ReturnValue", &steps);
    assert_eq!(id, summary("ReturnValue", &steps));
    assert_ne!(id, summary("ReturnValue", &[steps[1], steps[0]]));
    assert_ne!(id, summary("Field[y]", &steps));
}

#[test]
fn value_flow_origin_distinguishes_siblings_on_one_raw_fact() {
    let fact = Id([1; 16]);
    let use_id = Id([2; 16]);
    let key = recipe::ValueFlowOriginKey {
        fact, use_id, source_key: "Parameter[value]", identity: true,
        through_call: false, local_through_call: false,
        upstream_identity: true, upstream_through_call: false,
    };
    let direct = recipe::value_flow_origin(&key);
    assert_eq!(direct, recipe::value_flow_origin(&key));
    assert_ne!(direct, recipe::value_flow_origin(&recipe::ValueFlowOriginKey {
        source_key: "Parameter[value]:sibling", ..key
    }));
    assert_ne!(direct, recipe::value_flow_origin(&recipe::ValueFlowOriginKey {
        identity: false, through_call: true, local_through_call: true, ..key
    }));
}

proptest! {
    #[test]
    fn same_inputs_same_id(a in ".*", b in any::<i64>()) {
        let x = IdHasher::new("k").str(&a).i64(b).finish_id();
        let y = IdHasher::new("k").str(&a).i64(b).finish_id();
        prop_assert_eq!(x, y);
    }

    #[test]
    fn length_prefix_separates_field_boundaries(a in "[a-z]{0,8}", b in "[a-z]{0,8}", c in "[a-z]{1,8}") {
        // ("a"+"b", "c") vs ("a", "b"+"c"): same concatenation, different fields.
        let x = IdHasher::new("k").str(&format!("{a}{b}")).str(&c).finish_id();
        let y = IdHasher::new("k").str(&a).str(&format!("{b}{c}")).finish_id();
        prop_assert!(b.is_empty() || x != y);
    }

    #[test]
    fn kind_tag_is_part_of_identity(a in ".*") {
        prop_assert_ne!(IdHasher::new("module").str(&a).finish_id(), IdHasher::new("syntax").str(&a).finish_id());
    }
}
