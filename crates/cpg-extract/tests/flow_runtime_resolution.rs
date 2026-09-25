//! The extraction boundary must pass our lexical import resolution to the independent ty flow
//! provider. The provider's direct tests supply spans; this test checks the real join.

mod common;

use std::collections::HashMap;

use common::{cell, fixture, run};

#[test]
fn flow_runtime_decisions_follow_lexical_imports() {
    let (_dir, out) = run("flow_shapes");
    let source =
        std::fs::read_to_string(fixture("flow_shapes").join("release/flowpkg/shapes.py")).unwrap();
    let conditions = out.table("conditions").unwrap();
    let encodings: HashMap<String, String> = (0..conditions.num_rows())
        .map(|i| {
            (
                cell(conditions, "condition_id", i),
                cell(conditions, "encoding", i),
            )
        })
        .collect();
    let regions = out.table("flow_regions").unwrap();
    let condition_at = |function: &str, statement: &str| {
        let start = source.find(function).unwrap();
        let byte = start + source[start..].find(statement).unwrap();
        let row = (0..regions.num_rows())
            .find(|&i| cell(regions, "start_byte", i) == byte.to_string())
            .unwrap_or_else(|| panic!("no region at {statement}"));
        encodings[&cell(regions, "condition_id", row)].clone()
    };
    assert!(
        condition_at("def choose", "return \"parameter\"").starts_with("truthy(TYPE_CHECKING)#")
    );
    assert!(
        condition_at("def config_check", "return \"ordinary attribute\"")
            .starts_with("truthy(config.TYPE_CHECKING)#")
    );
    assert_eq!(
        condition_at("def checking_alias", "return \"checker only\""),
        "false"
    );
    assert_eq!(
        condition_at("def checking_module_alias", "return \"checker only\""),
        "false"
    );
    assert_eq!(condition_at("def version_prefix", "above = True"), "true");
    assert_eq!(
        condition_at("def version_prefix", "at_most = True"),
        "false"
    );
    let coverage = out.table("coverage").unwrap();
    assert!((0..coverage.num_rows()).any(|i| {
        let detail = cell(coverage, "detail", i);
        detail.contains("skipped_reaching_runtime_view=")
            && detail.contains("skipped_value_branches_runtime_view=")
    }));
}

#[test]
fn test_type_rows_name_the_selected_operand_and_trace() {
    let (_dir, out) = run("flow_shapes");
    let source =
        std::fs::read_to_string(fixture("flow_shapes").join("release/flowpkg/shapes.py")).unwrap();
    let rows = out.table("flow_test_types").unwrap();
    let test = source.find("if isinstance(x, C):").unwrap();
    let x = test + "if isinstance(".len();
    assert!((0..rows.num_rows()).any(|i| {
        cell(rows, "place", i) == "x"
            && cell(rows, "operand_start_byte", i) == x.to_string()
            && cell(rows, "operand_end_byte", i) == (x + 1).to_string()
            && cell(rows, "role", i) == "tested_place"
    }));
    // The sibling class expression is a use inside the test span, but it is not the tested place.
    assert!(!(0..rows.num_rows()).any(|i| {
        cell(rows, "operand_start_byte", i) == (x + 3).to_string() && cell(rows, "place", i) == "C"
    }));
}

#[test]
fn exact_type_guard_requires_resolved_builtins_and_names_the_inner_use() {
    let (_dir, out) = run("type_guard");
    let source = std::fs::read_to_string(fixture("type_guard").join("guardpkg/cases.py")).unwrap();
    let leaves = out.table("flow_test_leaves").unwrap();
    let typed: Vec<_> = (0..leaves.num_rows())
        .filter(|&i| cell(leaves, "atom", i).starts_with("type_is(x,str)#"))
        .collect();
    assert_eq!(typed.len(), 2, "both unshadowed builtin guards are exact");
    let exact_start = source.find("if type(x) is str:").unwrap();
    let operand = exact_start + "if type(".len();
    let rows = out.table("flow_test_types").unwrap();
    assert!((0..rows.num_rows()).any(|i| {
        cell(rows, "operand_start_byte", i) == operand.to_string()
            && cell(rows, "operand_end_byte", i) == (operand + 1).to_string()
            && cell(rows, "place", i) == "x"
    }));
    let opaque = (0..leaves.num_rows())
        .filter(|&i| cell(leaves, "atom", i).starts_with("opaque("))
        .count();
    assert!(opaque >= 2, "shadowed names keep the test opaque");
}
