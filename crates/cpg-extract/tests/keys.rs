//! Review F3 oracle: Pysa-side rows are keyed by file, and class references by class id
//! (DESIGN §4.2.3). A `.py` and its `.pyi` share a module name; nested classes share a name.

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

    // (file, function key) is unique, and the `.py`/`.pyi` pair both define `F:0`.
    let funcs = out.table("pysa_functions").unwrap();
    let mut keys = BTreeSet::new();
    let mut dual_f0 = BTreeSet::new();
    for r in 0..funcs.num_rows() {
        let file = path[&cell(funcs, "module_node_id", r)].clone();
        let key = cell(funcs, "function_key", r);
        assert!(keys.insert((file.clone(), key.clone())), "{file} {key}");
        if cell(funcs, "module_name", r) == "keys.dual" && key == "F:0" {
            dual_f0.insert(file);
        }
    }
    assert_eq!(
        dual_f0,
        BTreeSet::from(["keys/dual.py".to_owned(), "keys/dual.pyi".to_owned()])
    );

    // The two nested `Config` classes stay distinct wherever a class is referenced.
    let defining: BTreeSet<String> = (0..funcs.num_rows())
        .filter(|&r| cell(funcs, "name", r) == "m")
        .map(|r| cell(funcs, "defining_class", r))
        .collect();
    assert_eq!(defining.len(), 2, "{defining:?}");
    let params = out.table("parameter_semantics").unwrap();
    let annotated: BTreeSet<String> = (0..params.num_rows())
        .filter(|&r| ["a", "b"].contains(&cell(params, "name", r).as_str()))
        .map(|r| cell(params, "annotation_classes", r))
        .collect();
    assert_eq!(annotated.len(), 2, "{annotated:?}");
    let calls = out.table("pysa_calls").unwrap();
    let receivers: BTreeSet<String> = (0..calls.num_rows())
        .filter(|&r| cell(calls, "target_name", r) == "m")
        .map(|r| cell(calls, "receiver_class", r))
        .collect();
    assert_eq!(receivers, defining, "receivers are the defining classes");

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
