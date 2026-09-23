//! Known answers for the fact-id payload encoding and the canonical sort (H1 C8). cargo-mutants
//! found every `hash_into → ()` mutant and the two-row sort mutants alive: nothing asserted what
//! each column type feeds the hasher, or that a two-row batch is sorted.

use std::sync::Arc;

use arrow_array::{Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use cpg_schema::codebook::StaticBranch;
use cpg_schema::hash::HashField;
use cpg_schema::id::{Digest, Id, IdHasher};
use cpg_schema::table::canonical_sort;

/// The payload of `value` alone, under one fixed kind.
fn via_trait(value: &impl HashField) -> Id {
    let mut h = IdHasher::new("h1-c8");
    value.hash_into(&mut h);
    h.finish_id()
}

fn via(f: impl FnOnce(&mut IdHasher)) -> Id {
    let mut h = IdHasher::new("h1-c8");
    f(&mut h);
    h.finish_id()
}

/// Each column type feeds exactly its `IdHasher` method, and something (never nothing).
#[test]
fn each_column_type_feeds_its_own_encoding() {
    let empty = via(|_| {});
    let id = Id([3; 16]);
    let digest = Digest([5; 32]);
    let cases: Vec<(&str, Id, Id)> = vec![
        (
            "Id",
            via_trait(&id),
            via(|h| {
                h.id(id);
            }),
        ),
        (
            "Option<Id> some",
            via_trait(&Some(id)),
            via(|h| {
                h.opt_id(Some(id));
            }),
        ),
        (
            "Option<Id> none",
            via_trait(&None::<Id>),
            via(|h| {
                h.opt_id(None);
            }),
        ),
        (
            "Digest",
            via_trait(&digest),
            via(|h| {
                h.digest_field(digest);
            }),
        ),
        (
            "Option<Digest>",
            via_trait(&Some(digest)),
            via(|h| {
                h.opt_digest(Some(digest));
            }),
        ),
        (
            "String",
            via_trait(&"x".to_owned()),
            via(|h| {
                h.str("x");
            }),
        ),
        (
            "Option<String>",
            via_trait(&Some("x".to_owned())),
            via(|h| {
                h.opt_str(Some("x"));
            }),
        ),
        (
            "i64",
            via_trait(&7_i64),
            via(|h| {
                h.i64(7);
            }),
        ),
        (
            "Option<i64>",
            via_trait(&Some(7_i64)),
            via(|h| {
                h.opt_i64(Some(7));
            }),
        ),
        (
            "bool",
            via_trait(&true),
            via(|h| {
                h.bool(true);
            }),
        ),
        (
            "Option<bool>",
            via_trait(&Some(false)),
            via(|h| {
                h.opt_bool(Some(false));
            }),
        ),
        (
            "codebook",
            via_trait(&StaticBranch::Platform),
            via(|h| {
                h.i64(2);
            }),
        ),
        (
            "Option<codebook>",
            via_trait(&Some(StaticBranch::Platform)),
            via(|h| {
                h.opt_i64(Some(2));
            }),
        ),
        (
            "Vec<String>",
            via_trait(&vec!["a".to_owned(), "b".to_owned()]),
            via(|h| {
                h.strs(["a", "b"]);
            }),
        ),
    ];
    for (name, got, want) in cases {
        assert_eq!(got, want, "{name}");
        assert_ne!(got, empty, "{name} fed nothing");
    }
    // A present and an absent optional never collide.
    assert_ne!(via_trait(&None::<i64>), via_trait(&Some(0_i64)));
}

fn batch(keys: Vec<Option<i64>>, values: &[&str]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("k", DataType::Int64, true),
        Field::new("v", DataType::Utf8, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(keys)),
            Arc::new(StringArray::from(values.to_vec())),
        ],
    )
    .unwrap()
}

fn order(b: &RecordBatch) -> Vec<String> {
    let v = b.column(1).as_any().downcast_ref::<StringArray>().unwrap();
    (0..v.len()).map(|i| v.value(i).to_owned()).collect()
}

/// Two rows are sorted too (not only three or more), and nulls come first.
#[test]
fn the_canonical_sort_orders_small_batches_nulls_first() {
    let two = canonical_sort(&batch(vec![Some(2), Some(1)], &["b", "a"]), &["k"]).unwrap();
    assert_eq!(order(&two), ["a", "b"]);
    let nulls = canonical_sort(
        &batch(vec![Some(1), None, Some(0)], &["one", "null", "zero"]),
        &["k"],
    )
    .unwrap();
    assert_eq!(order(&nulls), ["null", "zero", "one"]);
    let single = canonical_sort(&batch(vec![Some(9)], &["only"]), &["k"]).unwrap();
    assert_eq!(order(&single), ["only"]);
}
