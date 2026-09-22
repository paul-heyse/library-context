//! Id derivation (DESIGN §3.4.1, ADR-0007 oracles).

use cpg_schema::id::{IdHasher, content_digest, kind};
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
