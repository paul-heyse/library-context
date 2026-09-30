//! Review F3 oracle for the legacy call rows: class references are keyed by class id, and a
//! target names the file Pyrefly resolved (DESIGN §4.2.3). Definitions are `typed_symbols`'.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{cell, run};

#[test]
fn pysa_keys_are_unique_per_file_and_class_references_carry_the_class_id() {
    let (_dir, out) = run("pysa_keys");
    let files = out.table("source_files").unwrap();
    let path: BTreeMap<String, String> = (0..files.num_rows())
        .map(|r| (cell(files, "module_node_id", r), cell(files, "path", r)))
        .collect();

    // Definitions per file and nested classes are the typed symbols' (`typed_symbols`); the call
    // graph's receivers and targets stay here until the call producer (A10).
    let calls = out.table("pysa_calls").unwrap();
    let receivers: BTreeSet<String> = (0..calls.num_rows())
        .filter(|&r| cell(calls, "target_name", r) == "m")
        .map(|r| cell(calls, "receiver_class", r))
        .collect();
    assert_eq!(receivers.len(), 2, "the two nested Config classes stay distinct receivers: {receivers:?}");

    // A target names the file Pyrefly resolved: the stub from outside, the source from within.
    let targets: BTreeMap<(String, String), String> = (0..calls.num_rows())
        .filter(|&r| ["f", "g"].contains(&cell(calls, "target_name", r).as_str()))
        .map(|r| {
            (
                (
                    path[&cell(calls, "module_node_id", r)].clone(),
                    cell(calls, "target_name", r),
                ),
                cell(calls, "target_module", r),
            )
        })
        .collect();
    assert_eq!(
        targets[&("keys/use_dual.py".to_owned(), "g".to_owned())],
        "@keys/dual.pyi"
    );
    assert_eq!(
        targets[&("keys/dual.py".to_owned(), "f".to_owned())],
        "@keys/dual.py"
    );
}
